#![allow(missing_docs, reason = "CLI command surface is described by clap help")]
#![allow(clippy::print_stdout, reason = "CLI output")]

use agoramesh_core::SystemClock;
use agoramesh_net::direct_sync::{self, SyncStats};
use serde::Serialize;

use crate::commands::helpers;
use crate::config::Config;
use crate::peers::Peers;

#[derive(Serialize)]
struct SyncOutput {
    #[serde(rename = "objects_pulled")]
    pulled: usize,
    #[serde(rename = "objects_pushed")]
    pushed: usize,
    #[serde(rename = "objects_rejected")]
    rejected: usize,
}

pub async fn run(config: &Config, category_id: &str, json: bool) -> Result<(), Error> {
    let peers = Peers::load(&config.peers_path())?;
    let mut store = helpers::open_store(config)?;
    let clock = SystemClock;
    let mut totals = SyncStats::default();

    for peer in peers.list() {
        let stats =
            direct_sync::sync_with_peer(&peer.address, &mut store, &clock, category_id).await?;
        totals.objects_pulled = totals.objects_pulled.saturating_add(stats.objects_pulled);
        totals.objects_pushed = totals.objects_pushed.saturating_add(stats.objects_pushed);
        totals.objects_rejected = totals
            .objects_rejected
            .saturating_add(stats.objects_rejected);
        if !json {
            println!(
                "{} pulled={} pushed={} rejected={}",
                peer.address, stats.objects_pulled, stats.objects_pushed, stats.objects_rejected
            );
        }
    }

    if json {
        let output = SyncOutput {
            pulled: totals.objects_pulled,
            pushed: totals.objects_pushed,
            rejected: totals.objects_rejected,
        };
        println!("{}", serde_json::to_string(&output)?);
    } else if peers.list().is_empty() {
        println!("No peers configured.");
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Helpers(#[from] helpers::Error),
    #[error(transparent)]
    Peers(#[from] crate::peers::Error),
    #[error(transparent)]
    DirectSync(#[from] direct_sync::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
