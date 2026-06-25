//! Peer command handlers.

#![allow(
    clippy::print_stdout,
    reason = "CLI commands intentionally write human-readable output to the terminal"
)]

use crate::config::Config;
use crate::peers::Peers;
use std::io::{self, Write};

/// Adds a peer address to the peer configuration file.
///
/// # Errors
///
/// Returns an error when the peer file cannot be loaded, updated, or saved.
pub fn add(config: &Config, address: &str) -> Result<(), Error> {
    let peers_path = config.peers_path();
    let mut peers = Peers::load(&peers_path).map_err(Error::Peers)?;
    peers.add(address).map_err(Error::Peers)?;
    peers.save(&peers_path).map_err(Error::Peers)
}

/// Lists peer addresses in either human-readable or JSON format.
///
/// # Errors
///
/// Returns an error when the peer file cannot be loaded, initialized, or written
/// to stdout.
pub fn list(config: &Config, json: bool) -> Result<(), Error> {
    let peers_path = config.peers_path();
    let peers = Peers::load(&peers_path).map_err(Error::Peers)?;
    peers.save(&peers_path).map_err(Error::Peers)?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if json {
        serde_json::to_writer(&mut handle, peers.list()).map_err(Error::Json)?;
        writeln!(handle).map_err(Error::WriteStdout)
    } else if peers.list().is_empty() {
        writeln!(handle, "No peers configured.").map_err(Error::WriteStdout)
    } else {
        for peer in peers.list() {
            writeln!(handle, "{}", peer.address).map_err(Error::WriteStdout)?;
        }
        Ok(())
    }
}

/// Errors raised by peer commands.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Peer configuration failed.
    #[error(transparent)]
    Peers(crate::peers::Error),

    /// Peer JSON output failed.
    #[error("failed to write peer JSON: {0}")]
    Json(serde_json::Error),

    /// Standard output could not be written.
    #[error("failed to write stdout: {0}")]
    WriteStdout(io::Error),
}
