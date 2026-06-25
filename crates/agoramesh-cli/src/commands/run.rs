#![allow(missing_docs, reason = "CLI command surface is described by clap help")]
#![allow(clippy::print_stdout, reason = "CLI output")]

use std::net::SocketAddr;
use std::sync::Arc;

use agoramesh_core::SystemClock;
use agoramesh_net::direct_sync;

use crate::commands::helpers;
use crate::config::Config;

pub async fn run(config: &Config, listen: SocketAddr) -> Result<(), Error> {
    let store = helpers::open_store(config)?;
    let clock = Arc::new(SystemClock);
    let (bound_addr, server) = direct_sync::serve(store, clock, listen).await?;
    println!("listening on http://{bound_addr}");
    server.await?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Helpers(#[from] helpers::Error),
    #[error(transparent)]
    DirectSync(#[from] direct_sync::Error),
}
