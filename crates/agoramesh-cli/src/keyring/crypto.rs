//! Cryptographic operations for CLI keyring files.

use agoramesh_core::Keypair;
use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use zeroize::Zeroizing;

use super::KeyringError;
use super::schema::{
    EncryptedKeyFile, KdfConfig, PlaintextKeyFile, decode_base64, decode_nonce, decode_seed,
    validate_encrypted_structure,
};
use super::{KEY_LEN, NONCE_LEN, SALT_LEN};

const ARGON2_MEMORY_COST_KIB: u32 = 19_456;
const ARGON2_TIME_COST: u32 = 2;
const ARGON2_PARALLELISM: u32 = 1;
const AAD: &[u8] = b"agoramesh-key-v1";

/// Generates encrypted keyring bytes for a new keypair.
pub(super) fn generate(passphrase: &str) -> Result<Vec<u8>, KeyringError> {
    serialize_encrypted(&Keypair::generate(), passphrase)
}

/// Loads an encrypted keyring from bytes with the provided passphrase.
pub(super) fn load(encrypted_bytes: &[u8], passphrase: &str) -> Result<Keypair, KeyringError> {
    let file = super::schema::parse_encrypted_key_file(encrypted_bytes)?;

    validate_encrypted_structure(&file)?;
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

/// Loads a development-only plaintext keypair from bytes.
pub(super) fn load_plaintext(bytes: &[u8]) -> Result<Keypair, KeyringError> {
    let file: PlaintextKeyFile = serde_json::from_slice(bytes)
        .map_err(|error| KeyringError::InvalidFormat(error.to_string()))?;

    if file.version != super::KEYRING_VERSION || file.format != "dev-plaintext" {
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
pub(super) fn public_key_hex(keypair: &Keypair) -> String {
    hex::encode(keypair.identity().verifying_key().as_bytes())
}

/// Serializes a keypair into an encrypted key file.
pub(super) fn serialize_encrypted(
    keypair: &Keypair,
    passphrase: &str,
) -> Result<Vec<u8>, KeyringError> {
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
        version: super::KEYRING_VERSION,
        public_key_hex: public_key_hex(keypair),
        kdf,
        salt: URL_SAFE_NO_PAD.encode(salt),
        nonce: URL_SAFE_NO_PAD.encode(nonce),
        ciphertext: URL_SAFE_NO_PAD.encode(ciphertext),
    };

    serde_json::to_vec_pretty(&file).map_err(|error| KeyringError::InvalidFormat(error.to_string()))
}

/// Serializes a keypair into a development-only plaintext key file.
pub(super) fn serialize_plaintext(keypair: &Keypair) -> Result<Vec<u8>, KeyringError> {
    let file = PlaintextKeyFile {
        version: super::KEYRING_VERSION,
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
