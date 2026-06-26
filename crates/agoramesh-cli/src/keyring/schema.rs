//! Structural validation for CLI keyring files without decrypting them.

use serde::{Deserialize, Serialize};

use super::{CIPHERTEXT_LEN, KEY_LEN, KEYRING_VERSION, KeyringError, NONCE_LEN, SALT_LEN};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// Metadata proven to come from a structurally valid encrypted key file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptedKeyMetadata {
    /// Lower-hex Ed25519 verifying key stored beside the encrypted seed.
    pub public_key_hex: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct EncryptedKeyFile {
    pub(super) version: u32,
    pub(super) public_key_hex: String,
    pub(super) kdf: KdfConfig,
    pub(super) salt: String,
    pub(super) nonce: String,
    pub(super) ciphertext: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct KdfConfig {
    pub(super) algorithm: String,
    pub(super) memory_cost_kib: u32,
    pub(super) time_cost: u32,
    pub(super) parallelism: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct PlaintextKeyFile {
    pub(super) version: u32,
    pub(super) format: String,
    pub(super) public_key_hex: String,
    pub(super) secret_seed: String,
}

/// Validates that a file is a complete encrypted key file without decrypting it.
pub fn validate_encrypted_key_file_structure(
    path: &std::path::Path,
) -> Result<EncryptedKeyMetadata, KeyringError> {
    let bytes = std::fs::read(path)?;
    validate_encrypted_key_bytes_structure(&bytes)
}

/// Validates that bytes are a complete encrypted key file without decrypting them.
pub fn validate_encrypted_key_bytes_structure(
    bytes: &[u8],
) -> Result<EncryptedKeyMetadata, KeyringError> {
    let file = parse_encrypted_key_file(bytes)?;
    validate_encrypted_structure(&file)
}

/// Validates development plaintext key bytes by importing the contained seed.
pub fn validate_dev_plaintext_key_bytes_structure(bytes: &[u8]) -> Result<(), KeyringError> {
    let file: PlaintextKeyFile = serde_json::from_slice(bytes)
        .map_err(|error| KeyringError::InvalidFormat(error.to_string()))?;

    if file.version != KEYRING_VERSION || file.format != "dev-plaintext" {
        return Err(KeyringError::InvalidFormat(
            "not a development plaintext key".to_owned(),
        ));
    }

    let keypair = agoramesh_core::Keypair::from_base64(&file.secret_seed)
        .map_err(|error| KeyringError::KeyMaterial(error.to_string()))?;
    if public_key_hex(&keypair) != file.public_key_hex {
        return Err(KeyringError::InvalidFormat(
            "public key metadata does not match plaintext seed".to_owned(),
        ));
    }

    Ok(())
}

pub(super) fn parse_encrypted_key_file(bytes: &[u8]) -> Result<EncryptedKeyFile, KeyringError> {
    serde_json::from_slice(bytes).map_err(|error| KeyringError::InvalidFormat(error.to_string()))
}

pub(super) fn validate_encrypted_structure(
    file: &EncryptedKeyFile,
) -> Result<EncryptedKeyMetadata, KeyringError> {
    validate_encrypted_header(file)?;
    validate_public_key_hex(&file.public_key_hex)?;
    validate_salt(&file.salt)?;
    decode_nonce(&file.nonce)?;
    let ciphertext = decode_base64(&file.ciphertext, "ciphertext")?;
    if ciphertext.len() != CIPHERTEXT_LEN {
        return Err(KeyringError::InvalidFormat(format!(
            "invalid ciphertext length: expected {CIPHERTEXT_LEN}, got {}",
            ciphertext.len()
        )));
    }
    validate_kdf_params(&file.kdf)?;
    Ok(EncryptedKeyMetadata {
        public_key_hex: file.public_key_hex.clone(),
    })
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

fn validate_public_key_hex(public_key_hex: &str) -> Result<(), KeyringError> {
    let bytes = hex::decode(public_key_hex)
        .map_err(|error| KeyringError::InvalidFormat(format!("invalid public_key_hex: {error}")))?;
    if bytes.len() != KEY_LEN {
        return Err(KeyringError::InvalidFormat(format!(
            "invalid public_key_hex length: expected {KEY_LEN}, got {}",
            bytes.len()
        )));
    }
    Ok(())
}

fn validate_salt(encoded: &str) -> Result<(), KeyringError> {
    let salt = decode_base64(encoded, "salt")?;
    if salt.len() != SALT_LEN {
        return Err(KeyringError::InvalidFormat(format!(
            "invalid salt length: expected {SALT_LEN}, got {}",
            salt.len()
        )));
    }
    Ok(())
}

pub(super) fn validate_kdf_params(kdf: &KdfConfig) -> Result<(), KeyringError> {
    argon2::Params::new(
        kdf.memory_cost_kib,
        kdf.time_cost,
        kdf.parallelism,
        Some(KEY_LEN),
    )
    .map_err(|error| KeyringError::Kdf(error.to_string()))?;
    Ok(())
}

pub(super) fn decode_base64(encoded: &str, field: &str) -> Result<Vec<u8>, KeyringError> {
    URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|error| KeyringError::InvalidFormat(format!("invalid {field}: {error}")))
}

pub(super) fn decode_nonce(encoded: &str) -> Result<[u8; NONCE_LEN], KeyringError> {
    decode_base64(encoded, "nonce")?
        .try_into()
        .map_err(|bytes: Vec<u8>| {
            KeyringError::InvalidFormat(format!(
                "invalid nonce length: expected {NONCE_LEN}, got {}",
                bytes.len()
            ))
        })
}

pub(super) fn decode_seed(bytes: &[u8]) -> Result<[u8; KEY_LEN], KeyringError> {
    bytes.try_into().map_err(|_error| {
        KeyringError::InvalidFormat(format!(
            "invalid decrypted seed length: expected {KEY_LEN}, got {}",
            bytes.len()
        ))
    })
}

fn public_key_hex(keypair: &agoramesh_core::Keypair) -> String {
    hex::encode(keypair.identity().verifying_key().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_dev_plaintext_key_bytes_structure_rejects_malformed_json() {
        let result = validate_dev_plaintext_key_bytes_structure(b"not json");

        assert!(matches!(result, Err(KeyringError::InvalidFormat(_))));
    }
}
