//! Local JSON state persistence for subscriptions and first-seen acknowledgements.

use std::path::{Path, PathBuf};

use agoramesh_cli::config::Config;

use crate::error::Error;
use crate::models::{AcknowledgedFirstSeen, Subscriptions};

const SUBSCRIPTIONS_FILE: &str = "subscriptions.json";
const FIRST_SEEN_FILE: &str = "seen.json";

#[derive(Debug)]
pub(super) struct LocalState {
    data_dir: PathBuf,
}

impl LocalState {
    pub(super) fn new(config: &Config) -> Self {
        Self {
            data_dir: config.data_dir.clone(),
        }
    }

    fn subscriptions_path(&self) -> PathBuf {
        self.data_dir.join(SUBSCRIPTIONS_FILE)
    }

    fn first_seen_path(&self) -> PathBuf {
        self.data_dir.join(FIRST_SEEN_FILE)
    }

    /// Loads locally persisted subscriptions.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or parsed.
    pub(super) fn load_subscriptions(&self) -> Result<Subscriptions, Error> {
        load_json(&self.subscriptions_path(), Subscriptions::default())
    }

    /// Saves locally persisted subscriptions.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be written.
    pub(super) fn save_subscriptions(&self, subscriptions: &Subscriptions) -> Result<(), Error> {
        save_json(&self.subscriptions_path(), subscriptions)
    }

    /// Loads acknowledged first-seen values.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or parsed.
    pub(super) fn load_acknowledged(&self) -> Result<AcknowledgedFirstSeen, Error> {
        load_json(&self.first_seen_path(), AcknowledgedFirstSeen::default())
    }

    /// Saves acknowledged first-seen values.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be written.
    pub(super) fn save_acknowledged(
        &self,
        acknowledged: &AcknowledgedFirstSeen,
    ) -> Result<(), Error> {
        save_json(&self.first_seen_path(), acknowledged)
    }
}

fn load_json<T: Default + serde::de::DeserializeOwned>(
    path: &Path,
    default: T,
) -> Result<T, Error> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(Error::StateJson),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(default),
        Err(source) => Err(Error::StateIo(source)),
    }
}

fn save_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), Error> {
    let bytes = serde_json::to_vec_pretty(value).map_err(Error::StateJson)?;
    crate::backend::file_io::write_atomic(path, &bytes)
}
