//! Data gateway between the TUI and the underlying `AgoraMesh` crates.

mod content;
mod file_io;
mod key_mgmt;
mod local_state;
mod peers;
use local_state::LocalState;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use agoramesh_cli::config::Config;
use agoramesh_store::db::{Connection, SqliteStore};

use crate::error::Error;
use crate::models::{AcknowledgedFirstSeen, KeyStatus, PeerStatus, Subscriptions};

/// Gateway that exposes TUI-friendly operations over a data directory.
#[derive(Debug)]
pub struct Backend {
    config: Config,
    plaintext: bool,
    passphrase: Mutex<Option<String>>,
}

impl Backend {
    /// Opens the backend for the given data directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the data directory or store cannot be initialized.
    pub fn open(data_dir: Option<PathBuf>, plaintext: bool) -> Result<Self, Error> {
        let config = Config::open(data_dir)?;
        Ok(Self {
            config,
            plaintext,
            passphrase: Mutex::new(None),
        })
    }

    /// Opens the `SQLite` store for this backend.
    ///
    /// Exposed publicly so integration tests and TUI event loops can read and
    /// write messages through the verified store.
    pub fn store(&self) -> Result<SqliteStore, Error> {
        let connection = Connection::open(&self.config.store_path())?;
        Ok(SqliteStore::new(connection))
    }

    /// Returns the filesystem path used by this backend.
    #[must_use]
    pub fn data_dir(&self) -> &Path {
        &self.config.data_dir
    }

    /// Loads locally persisted subscriptions.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or parsed.
    pub fn load_subscriptions(&self) -> Result<Subscriptions, Error> {
        LocalState::new(&self.config).load_subscriptions()
    }

    /// Saves locally persisted subscriptions.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be written.
    pub fn save_subscriptions(&self, subscriptions: &Subscriptions) -> Result<(), Error> {
        LocalState::new(&self.config).save_subscriptions(subscriptions)
    }

    /// Loads acknowledged first-seen values.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or parsed.
    pub fn load_acknowledged(&self) -> Result<AcknowledgedFirstSeen, Error> {
        LocalState::new(&self.config).load_acknowledged()
    }

    /// Saves acknowledged first-seen values.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be written.
    pub fn save_acknowledged(&self, acknowledged: &AcknowledgedFirstSeen) -> Result<(), Error> {
        LocalState::new(&self.config).save_acknowledged(acknowledged)
    }

    /// Returns the current key status, generating a dev plaintext key only when
    /// requested and development plaintext mode is enabled.
    ///
    /// # Errors
    ///
    /// Returns an error when the key file cannot be read or generated.
    pub fn key_status(&self, generate_if_missing: bool) -> Result<KeyStatus, Error> {
        key_mgmt::key_status(self, generate_if_missing)
    }

    /// Generates a new development plaintext key for the configured data dir.
    ///
    /// # Errors
    ///
    /// Returns an error when the key file cannot be written.
    pub fn generate_dev_key(&self) -> Result<KeyStatus, Error> {
        key_mgmt::generate_dev_key(self)
    }

    /// Generates and unlocks a Phase 1 encrypted keyring.
    pub fn generate_encrypted_key(&self, passphrase: &str) -> Result<KeyStatus, Error> {
        key_mgmt::generate_encrypted_key(self, passphrase)
    }

    /// Unlocks an existing encrypted keyring for this TUI session.
    pub fn unlock_key(&self, passphrase: &str) -> Result<KeyStatus, Error> {
        key_mgmt::unlock_key(self, passphrase)
    }

    /// Writes an atomic backup copy of the current key file.
    pub fn backup_key(&self) -> Result<PathBuf, Error> {
        key_mgmt::backup_key(self)
    }

    /// Restores the current key file from the default backup copy.
    pub fn restore_key_from_backup(&self) -> Result<(), Error> {
        key_mgmt::restore_key_from_backup(self)
    }

    /// Loads peer statuses from the persisted peers file.
    ///
    /// # Errors
    ///
    /// Returns an error when the peers file cannot be read or parsed.
    pub fn peer_statuses(&self) -> Result<Vec<PeerStatus>, Error> {
        peers::peer_statuses(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend_fixture(plaintext: bool) -> (Backend, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let backend =
            Backend::open(Some(temp_dir.path().to_path_buf()), plaintext).expect("open backend");
        (backend, temp_dir)
    }

    #[test]
    fn backend_generates_dev_plaintext_key_only_when_flagged() {
        let (backend, _temp_dir) = backend_fixture(true);
        let status = backend.key_status(true).expect("key status");
        assert!(
            matches!(status, KeyStatus::Present { .. }),
            "plaintext backend should generate a dev key on demand"
        );
    }
}
