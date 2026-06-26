//! Encrypted key file encoding for CLI-owned mesh identities.

pub mod crypto;
pub mod files;
pub mod schema;

use std::path::{Path, PathBuf};

use agoramesh_core::Keypair;

pub use files::{read_encrypted_public_key_for_display, validate_encrypted_key_file_structure};
pub use schema::{
    EncryptedKeyMetadata, validate_dev_plaintext_key_bytes_structure,
    validate_encrypted_key_bytes_structure,
};

pub(super) const KEYRING_VERSION: u32 = 1;
pub(super) const SALT_LEN: usize = 16;
pub(super) const NONCE_LEN: usize = 24;
pub(super) const KEY_LEN: usize = 32;
pub(super) const CIPHERTEXT_LEN: usize = KEY_LEN + 16;

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
        std::fs::write(&self.path, crypto::generate(passphrase)?)?;
        Ok(())
    }

    /// Loads an encrypted keypair.
    pub fn load(&self, passphrase: &str) -> Result<Keypair, KeyringError> {
        files::load_encrypted_key_with_passphrase(&self.path, passphrase)
    }

    /// Generates and saves a development-only plaintext keypair.
    pub fn dev_plaintext_save(&self) -> Result<(), KeyringError> {
        let bytes = crypto::serialize_plaintext(&Keypair::generate())?;
        std::fs::write(&self.path, bytes)?;
        Ok(())
    }

    /// Loads a development-only plaintext keypair.
    pub fn dev_plaintext_load(&self) -> Result<Keypair, KeyringError> {
        let bytes = std::fs::read(&self.path)?;
        crypto::load_plaintext(&bytes)
    }
}

/// Generates encrypted keyring bytes for a new keypair.
pub fn generate(passphrase: &str) -> Result<Vec<u8>, KeyringError> {
    crypto::generate(passphrase)
}

/// Loads an encrypted keyring from bytes with the provided passphrase.
pub fn load(encrypted_bytes: &[u8], passphrase: &str) -> Result<Keypair, KeyringError> {
    crypto::load(encrypted_bytes, passphrase)
}

/// Loads an encrypted key file with authenticated decryption.
pub fn load_encrypted_key_with_passphrase(
    path: &Path,
    passphrase: &str,
) -> Result<Keypair, KeyringError> {
    files::load_encrypted_key_with_passphrase(path, passphrase)
}

/// Returns the lower-hex public Ed25519 verifying key for metadata output.
#[must_use]
pub fn public_key_hex(keypair: &Keypair) -> String {
    crypto::public_key_hex(keypair)
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
