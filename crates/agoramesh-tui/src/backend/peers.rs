//! Peer status loading for the TUI backend.

use agoramesh_cli::peers::Peers;

use crate::backend::Backend;
use crate::error::Error;
use crate::models::PeerStatus;

/// Loads peer statuses from the persisted peers file.
///
/// # Errors
///
/// Returns an error when the peers file cannot be read or parsed.
pub(super) fn peer_statuses(backend: &Backend) -> Result<Vec<PeerStatus>, Error> {
    let peers = Peers::load(&backend.config.peers_path())?;
    Ok(peers
        .list()
        .iter()
        .map(|peer| PeerStatus {
            name: peer.name.clone(),
            address: peer.address.clone(),
            last_sync_ok: None,
        })
        .collect())
}
