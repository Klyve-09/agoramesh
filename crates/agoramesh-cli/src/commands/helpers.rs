#![allow(missing_docs, reason = "CLI helper surface is crate-internal")]

use std::path::{Path, PathBuf};

use agoramesh_store::{Connection, SqliteStore};

use crate::commands::key;
use crate::config::Config;
use agoramesh_core::objects::acceptance::{self, AcceptanceContext};
use agoramesh_core::{Keypair, Message, SystemClock};

pub fn load_keypair(key_path: &Path, plaintext: bool) -> Result<Keypair, Error> {
    key::load_keypair(key_path, plaintext).map_err(Error::Key)
}

pub fn open_store(config: &Config) -> Result<SqliteStore, Error> {
    let connection = Connection::open(&config.store_path())?;
    Ok(SqliteStore::new(connection))
}

pub fn resolve_key_path(config: &Config, key_path: Option<&Path>) -> PathBuf {
    key_path.map_or_else(|| config.key_path(), Path::to_path_buf)
}

pub fn ensure_phase1_acceptable(
    message: &Message,
    clock: &SystemClock,
) -> Result<(), acceptance::Error> {
    acceptance::validate_phase1_for_acceptance(message, &AcceptanceContext::phase1(clock))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Key(#[from] key::Error),

    #[error(transparent)]
    Store(#[from] agoramesh_store::db::Error),
}
