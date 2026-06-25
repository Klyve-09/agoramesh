//! Encrypted key file encoding for CLI-owned mesh identities.

use std::path::{Path, PathBuf};

use agoramesh_core::Keypair;
use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

const KEYRING_VERSION: u32 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const KEY_LEN: usize = 32;
const ARGON2_MEMORY_COST_KIB: u32 = 19_456;
const ARGON2_TIME_COST: u32 = 2;
const ARGON2_PARALLELISM: u32 = 1;
const AAD: &[u8] = b"agoramesh-key-v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct EncryptedKeyFile {
    version: u32,
    public_key_hex: String,
    kdf: KdfConfig,
    salt: String,
    nonce: String,
    ciphertext: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct KdfConfig {
    algorithm: String,
    memory_cost_kib: u32,
    time_cost: u32,
    parallelism: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct PlaintextKeyFile {
    version: u32,
    format: String,
    public_key_hex: String,
    secret_seed: String,
}

/// Errors produced while encoding or loading CLI key files.
#[derive(Debug, thiserror::Error)]
pub enum KeyringError {
    /// The key file JSON is malformed or uses an unknown schema.
    #[error("invalid key file: {0}")]
    InvalidFormat(String),

    /// The passphrase could not derive a decryption key.
    #[error("failed to derive key: {0}")]
    Kdf(String),

    /// The passphrase or ciphertext failed authenticated decryption.
    #[error("failed to decrypt key")]
    Decrypt,

    /// The decrypted seed cannot be imported as a mesh keypair.
    #[error("invalid key material: {0}")]
    KeyMaterial(String),

    /// Reading or writing the key file failed.
    #[error("key file I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Path-bound keyring used by CLI commands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Keyring {
    path: PathBuf,
}

impl Keyring {
    /// Creates a keyring at the given filesystem path.
    #[must_use]
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }

    /// Generates and saves an encrypted keypair.
    pub fn generate(&self, passphrase: &str) -> Result<(), KeyringError> {
        std::fs::write(&self.path, generate(passphrase)?)?;
        Ok(())
    }

    /// Loads an encrypted keypair.
    pub fn load(&self, passphrase: &str) -> Result<Keypair, KeyringError> {
        let bytes = std::fs::read(&self.path)?;
        load(&bytes, passphrase)
    }

    /// Generates and saves a development-only plaintext keypair.
    pub fn dev_plaintext_save(&self) -> Result<(), KeyringError> {
        let bytes = serialize_plaintext(&Keypair::generate())?;
        std::fs::write(&self.path, bytes)?;
        Ok(())
    }

    /// Loads a development-only plaintext keypair.
    pub fn dev_plaintext_load(&self) -> Result<Keypair, KeyringError> {
        let bytes = std::fs::read(&self.path)?;
        load_plaintext(&bytes)
    }
}

/// Generates encrypted keyring bytes for a new keypair.
pub fn generate(passphrase: &str) -> Result<Vec<u8>, KeyringError> {
    serialize_encrypted(&Keypair::generate(), passphrase)
}

/// Loads an encrypted keyring from bytes with the provided passphrase.
pub fn load(encrypted_bytes: &[u8], passphrase: &str) -> Result<Keypair, KeyringError> {
    let file: EncryptedKeyFile = serde_json::from_slice(encrypted_bytes)
        .map_err(|error| KeyringError::InvalidFormat(error.to_string()))?;

    validate_encrypted_header(&file)?;
    let salt = decode_base64(&file.salt, "salt")?;
    let nonce = decode_nonce(&file.nonce)?;
    let ciphertext = decode_base64(&file.ciphertext, "ciphertext")?;
    let key = derive_key(passphrase, &salt, &file.kdf)?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_slice()));
    let plaintext = cipher
        .decrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: ciphertext.as_ref(),
                aad: AAD,
            },
        )
        .map_err(|_error| KeyringError::Decrypt)?;
    let seed = decode_seed(&plaintext)?;
    let keypair =
        Keypair::from_bytes(&seed).map_err(|error| KeyringError::KeyMaterial(error.to_string()))?;

    if public_key_hex(&keypair) != file.public_key_hex {
        return Err(KeyringError::InvalidFormat(
            "public key metadata does not match encrypted seed".to_owned(),
        ));
    }

    Ok(keypair)
}

fn load_plaintext(bytes: &[u8]) -> Result<Keypair, KeyringError> {
    let file: PlaintextKeyFile = serde_json::from_slice(bytes)
        .map_err(|error| KeyringError::InvalidFormat(error.to_string()))?;

    if file.version != KEYRING_VERSION || file.format != "dev-plaintext" {
        return Err(KeyringError::InvalidFormat(
            "not a development plaintext key".to_owned(),
        ));
    }

    let keypair = Keypair::from_base64(&file.secret_seed)
        .map_err(|error| KeyringError::KeyMaterial(error.to_string()))?;
    if public_key_hex(&keypair) != file.public_key_hex {
        return Err(KeyringError::InvalidFormat(
            "public key metadata does not match plaintext seed".to_owned(),
        ));
    }

    Ok(keypair)
}

/// Returns the lower-hex public Ed25519 verifying key for metadata output.
#[must_use]
pub fn public_key_hex(keypair: &Keypair) -> String {
    hex::encode(keypair.identity().verifying_key().as_bytes())
}

fn serialize_encrypted(keypair: &Keypair, passphrase: &str) -> Result<Vec<u8>, KeyringError> {
    let mut salt = [0_u8; SALT_LEN];
    let mut nonce = [0_u8; NONCE_LEN];
    let mut rng = rand::rngs::OsRng;
    rng.fill_bytes(&mut salt);
    rng.fill_bytes(&mut nonce);

    let kdf = default_kdf_config();
    let key = derive_key(passphrase, &salt, &kdf)?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_slice()));
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: keypair.to_bytes().as_ref(),
                aad: AAD,
            },
        )
        .map_err(|_error| KeyringError::InvalidFormat("encryption failed".to_owned()))?;

    let file = EncryptedKeyFile {
        version: KEYRING_VERSION,
        public_key_hex: public_key_hex(keypair),
        kdf,
        salt: URL_SAFE_NO_PAD.encode(salt),
        nonce: URL_SAFE_NO_PAD.encode(nonce),
        ciphertext: URL_SAFE_NO_PAD.encode(ciphertext),
    };

    serde_json::to_vec_pretty(&file).map_err(|error| KeyringError::InvalidFormat(error.to_string()))
}

fn serialize_plaintext(keypair: &Keypair) -> Result<Vec<u8>, KeyringError> {
    let file = PlaintextKeyFile {
        version: KEYRING_VERSION,
        format: "dev-plaintext".to_owned(),
        public_key_hex: public_key_hex(keypair),
        secret_seed: keypair.to_base64(),
    };
    serde_json::to_vec_pretty(&file).map_err(|error| KeyringError::InvalidFormat(error.to_string()))
}

fn default_kdf_config() -> KdfConfig {
    KdfConfig {
        algorithm: "argon2id".to_owned(),
        memory_cost_kib: ARGON2_MEMORY_COST_KIB,
        time_cost: ARGON2_TIME_COST,
        parallelism: ARGON2_PARALLELISM,
    }
}

fn validate_encrypted_header(file: &EncryptedKeyFile) -> Result<(), KeyringError> {
    if file.version != KEYRING_VERSION {
        return Err(KeyringError::InvalidFormat(format!(
            "unsupported version {}",
            file.version
        )));
    }
    if file.kdf.algorithm != "argon2id" {
        return Err(KeyringError::InvalidFormat(format!(
            "unsupported kdf {}",
            file.kdf.algorithm
        )));
    }
    Ok(())
}

fn derive_key(
    passphrase: &str,
    salt: &[u8],
    kdf: &KdfConfig,
) -> Result<Zeroizing<[u8; KEY_LEN]>, KeyringError> {
    let params = Params::new(
        kdf.memory_cost_kib,
        kdf.time_cost,
        kdf.parallelism,
        Some(KEY_LEN),
    )
    .map_err(|error| KeyringError::Kdf(error.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, key.as_mut())
        .map_err(|error| KeyringError::Kdf(error.to_string()))?;
    Ok(key)
}

fn decode_base64(encoded: &str, field: &str) -> Result<Vec<u8>, KeyringError> {
    URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|error| KeyringError::InvalidFormat(format!("invalid {field}: {error}")))
}

fn decode_nonce(encoded: &str) -> Result<[u8; NONCE_LEN], KeyringError> {
    decode_base64(encoded, "nonce")?
        .try_into()
        .map_err(|bytes: Vec<u8>| {
            KeyringError::InvalidFormat(format!(
                "invalid nonce length: expected {NONCE_LEN}, got {}",
                bytes.len()
            ))
        })
}

fn decode_seed(bytes: &[u8]) -> Result<[u8; KEY_LEN], KeyringError> {
    bytes.try_into().map_err(|_error| {
        KeyringError::InvalidFormat(format!(
            "invalid decrypted seed length: expected {KEY_LEN}, got {}",
            bytes.len()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyring_load_fails_closed_for_wrong_passphrase() {
        let encrypted = generate("right passphrase").expect("generate encrypted key");

        let result = load(&encrypted, "wrong passphrase");

        assert!(matches!(result, Err(KeyringError::Decrypt)));
    }

    #[test]
    fn keyring_roundtrips_encrypted_keypair() {
        let encrypted = generate("right passphrase").expect("generate encrypted key");

        let keypair = load(&encrypted, "right passphrase").expect("load encrypted key");

        assert_eq!(public_key_hex(&keypair).len(), 64);
    }
}
