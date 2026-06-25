//! Filesystem configuration for the Agoramesh CLI.

use agoramesh_store::Connection;
use std::fs;
use std::path::PathBuf;

const STORE_FILE: &str = "store.db";
const KEY_FILE: &str = "identity.key";
const PEERS_FILE: &str = "peers.json";

/// Runtime filesystem paths used by the CLI.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Config {
    /// Root directory for CLI-managed state.
    pub data_dir: PathBuf,
}

impl Config {
    /// Opens the CLI configuration and initializes the store database.
    ///
    /// # Errors
    ///
    /// Returns an error when no platform data directory is available, the data
    /// directory cannot be created, or the store database cannot be opened.
    pub fn open(data_dir: Option<PathBuf>) -> Result<Self, Error> {
        let config = Self {
            data_dir: match data_dir {
                Some(path) => path,
                None => default_data_dir()?,
            },
        };
        config.ensure_dirs()?;
        let _connection = Connection::open(&config.store_path()).map_err(Error::Store)?;
        Ok(config)
    }

    /// Ensures the CLI data directory exists.
    ///
    /// # Errors
    ///
    /// Returns an error when the directory cannot be created.
    pub fn ensure_dirs(&self) -> Result<(), Error> {
        fs::create_dir_all(&self.data_dir).map_err(|source| Error::CreateDataDir {
            path: self.data_dir.clone(),
            source,
        })
    }

    /// Returns the `SQLite` store path under the data directory.
    #[must_use]
    pub fn store_path(&self) -> PathBuf {
        self.data_dir.join(STORE_FILE)
    }

    /// Returns the peer identity key path under the data directory.
    #[must_use]
    pub fn key_path(&self) -> PathBuf {
        self.data_dir.join(KEY_FILE)
    }

    /// Returns the peer configuration path under the data directory.
    #[must_use]
    pub fn peers_path(&self) -> PathBuf {
        self.data_dir.join(PEERS_FILE)
    }
}

fn default_data_dir() -> Result<PathBuf, Error> {
    let base = dirs::data_dir().ok_or(Error::MissingPlatformDataDir)?;
    Ok(base.join("agoramesh"))
}

/// Errors raised while loading CLI configuration.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The host platform did not provide a data directory.
    #[error("no platform data directory is available")]
    MissingPlatformDataDir,

    /// The CLI data directory could not be created.
    #[error("failed to create data directory {path}: {source}")]
    CreateDataDir {
        /// Directory that could not be created.
        path: PathBuf,
        /// Underlying filesystem error.
        source: std::io::Error,
    },

    /// The persistent store could not be opened.
    #[error(transparent)]
    Store(agoramesh_store::db::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn paths_are_derived_from_data_dir() {
        let config = Config {
            data_dir: Path::new("/tmp/agoramesh-test").to_path_buf(),
        };

        assert_eq!(
            config.store_path(),
            Path::new("/tmp/agoramesh-test/store.db")
        );
        assert_eq!(
            config.key_path(),
            Path::new("/tmp/agoramesh-test/identity.key")
        );
        assert_eq!(
            config.peers_path(),
            Path::new("/tmp/agoramesh-test/peers.json")
        );
    }

    #[test]
    fn open_creates_data_dir_and_store() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let data_dir = temp_dir.path().join("state");

        let config = Config::open(Some(data_dir.clone())).expect("open config");

        assert_eq!(config.data_dir, data_dir);
        assert!(config.store_path().is_file());
    }
}
