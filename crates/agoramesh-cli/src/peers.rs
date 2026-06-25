//! Peer configuration persistence for manually added mesh peers.

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use url::Url;

/// A manually configured peer endpoint.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Peer {
    /// Optional local display name for the peer.
    pub name: Option<String>,
    /// HTTP endpoint address in `http://host:port` form.
    pub address: String,
    /// Timestamp when the peer was added, formatted as RFC3339.
    pub added_at: String,
}

/// Collection persisted in `peers.json`.
#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Peers {
    peers: Vec<Peer>,
}

impl Peers {
    /// Loads peers from a JSON file, returning an empty list when it is absent.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, Error> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|source| Error::Parse {
                path: path.to_path_buf(),
                source,
            }),
            Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(Error::Read {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// Saves peers to a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error when the parent directory cannot be created, the peers
    /// cannot be serialized, or the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| Error::CreateParentDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let json = serde_json::to_vec_pretty(self).map_err(Error::Serialize)?;
        fs::write(path, json).map_err(|source| Error::Write {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Adds a peer address to the collection.
    ///
    /// # Errors
    ///
    /// Returns an error when the address is not a basic HTTP peer URL.
    pub fn add(&mut self, address: &str) -> Result<(), Error> {
        validate_peer_address(address)?;
        self.peers.push(Peer {
            name: None,
            address: address.to_owned(),
            added_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        });
        Ok(())
    }

    /// Returns the configured peers in insertion order.
    #[must_use]
    pub fn list(&self) -> &[Peer] {
        &self.peers
    }
}

fn validate_peer_address(address: &str) -> Result<(), Error> {
    let url = Url::parse(address).map_err(|source| Error::InvalidAddress {
        address: address.to_owned(),
        source,
    })?;
    if url.scheme() != "http" {
        return Err(Error::PeerAddressMustUseHttp);
    }
    if url.host_str().is_none() || url.port().is_none() {
        return Err(Error::PeerAddressMustIncludeHostAndPort);
    }
    Ok(())
}

/// Errors raised while reading or writing peer configuration.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The peer address could not be parsed as a URL.
    #[error("invalid peer address {address}: {source}")]
    InvalidAddress {
        /// Address that failed URL parsing.
        address: String,
        /// Underlying URL parser error.
        source: url::ParseError,
    },

    /// The peer address does not use plain HTTP.
    #[error("peer address must start with http://")]
    PeerAddressMustUseHttp,

    /// The peer address does not include both host and port.
    #[error("peer address must include host and port")]
    PeerAddressMustIncludeHostAndPort,

    /// The peer file could not be read.
    #[error("failed to read peers file {path}: {source}")]
    Read {
        /// Peer file path.
        path: PathBuf,
        /// Underlying filesystem error.
        source: io::Error,
    },

    /// The peer file could not be parsed.
    #[error("failed to parse peers file {path}: {source}")]
    Parse {
        /// Peer file path.
        path: PathBuf,
        /// Underlying JSON parser error.
        source: serde_json::Error,
    },

    /// The peers collection could not be serialized.
    #[error("failed to serialize peers: {0}")]
    Serialize(serde_json::Error),

    /// The peer file parent directory could not be created.
    #[error("failed to create peers directory {path}: {source}")]
    CreateParentDir {
        /// Directory path.
        path: PathBuf,
        /// Underlying filesystem error.
        source: io::Error,
    },

    /// The peer file could not be written.
    #[error("failed to write peers file {path}: {source}")]
    Write {
        /// Peer file path.
        path: PathBuf,
        /// Underlying filesystem error.
        source: io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_returns_empty_peers_when_file_is_missing() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let peers = Peers::load(&temp_dir.path().join("peers.json")).expect("load peers");

        assert!(peers.list().is_empty());
    }

    #[test]
    fn add_and_save_roundtrips_manual_peer_address() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let path = temp_dir.path().join("peers.json");
        let mut peers = Peers::default();

        peers.add("http://127.0.0.1:8080").expect("add valid peer");
        peers.save(&path).expect("save peers");
        let loaded = Peers::load(&path).expect("load saved peers");

        assert_eq!(loaded.list().len(), 1);
        assert_eq!(
            loaded.list().first().map(|peer| peer.address.as_str()),
            Some("http://127.0.0.1:8080")
        );
    }
}
