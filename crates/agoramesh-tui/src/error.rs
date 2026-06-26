//! Errors surfaced by the `AgoraMesh` TUI.

use agoramesh_cli::config;
use agoramesh_cli::keyring;
use agoramesh_store::store::Error as StoreError;

/// Top-level error type returned by TUI backend operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to open the data directory or store.
    #[error(transparent)]
    Config(#[from] config::Error),

    /// Peer file read/write failed.
    #[error(transparent)]
    Peers(#[from] agoramesh_cli::peers::Error),

    /// Key file generation or load failed.
    #[error("key error: {0}")]
    Key(keyring::KeyringError),

    /// Reading or writing a local TUI state file failed.
    #[error("state file I/O failed: {0}")]
    StateIo(#[from] std::io::Error),

    /// JSON serialization of a local TUI state file failed.
    #[error("state file JSON failed: {0}")]
    StateJson(#[from] serde_json::Error),

    /// A store database open or migration failed.
    #[error("store database error: {0}")]
    StoreDb(#[from] agoramesh_store::db::Error),

    /// A store read or write failed.
    #[error(transparent)]
    Store(#[from] StoreError),

    /// A core message operation failed.
    #[error("message error: {0}")]
    Message(String),

    /// A direct sync operation failed.
    #[error("sync failed: {0}")]
    Sync(String),
}

impl From<keyring::KeyringError> for Error {
    fn from(source: keyring::KeyringError) -> Self {
        Self::Key(source)
    }
}

impl From<agoramesh_core::message::Error> for Error {
    fn from(source: agoramesh_core::message::Error) -> Self {
        Self::Message(source.to_string())
    }
}

impl From<agoramesh_core::objects::validation::Error> for Error {
    fn from(source: agoramesh_core::objects::validation::Error) -> Self {
        Self::Message(source.to_string())
    }
}

impl From<chrono::ParseError> for Error {
    fn from(source: chrono::ParseError) -> Self {
        Self::Message(source.to_string())
    }
}
