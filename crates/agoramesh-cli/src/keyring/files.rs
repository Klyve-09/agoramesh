//! Filesystem helpers for CLI keyring operations.

use std::path::Path;

use agoramesh_core::Keypair;

use super::KeyringError;
use super::schema::EncryptedKeyMetadata;

/// Reads only the encrypted-key public-key metadata for display.
///
/// This helper does not prove the encrypted key file is safe to restore. Use
/// [`validate_encrypted_key_file_structure`] for restore validation.
pub fn read_encrypted_public_key_for_display(path: &Path) -> Result<Option<String>, KeyringError> {
    let bytes = std::fs::read(path)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| KeyringError::InvalidFormat(error.to_string()))?;
    Ok(value
        .get("public_key_hex")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned))
}

/// Loads an encrypted key file with authenticated decryption.
pub(super) fn load_encrypted_key_with_passphrase(
    path: &Path,
    passphrase: &str,
) -> Result<Keypair, KeyringError> {
    let bytes = std::fs::read(path)?;
    super::crypto::load(&bytes, passphrase)
}

/// Validates that a file is a complete encrypted key file without decrypting it.
pub fn validate_encrypted_key_file_structure(
    path: &Path,
) -> Result<EncryptedKeyMetadata, KeyringError> {
    super::schema::validate_encrypted_key_file_structure(path)
}

/// Validates that bytes are a complete encrypted key file without decrypting them.
pub fn validate_encrypted_key_bytes_structure(
    bytes: &[u8],
) -> Result<EncryptedKeyMetadata, KeyringError> {
    super::schema::validate_encrypted_key_bytes_structure(bytes)
}

/// Validates development plaintext key bytes by importing the contained seed.
pub fn validate_dev_plaintext_key_bytes_structure(bytes: &[u8]) -> Result<(), KeyringError> {
    super::schema::validate_dev_plaintext_key_bytes_structure(bytes)
}
