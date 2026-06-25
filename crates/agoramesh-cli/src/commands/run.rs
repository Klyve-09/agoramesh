#![allow(missing_docs, reason = "CLI command surface is described by clap help")]
#![allow(clippy::print_stdout, reason = "CLI output")]

use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use agoramesh_core::SystemClock;
use agoramesh_net::direct_sync;

use crate::commands::helpers;
use crate::config::Config;

pub async fn run(
    config: &Config,
    listen: SocketAddr,
    allow_public_bind: bool,
) -> Result<(), Error> {
    if !is_loopback(listen.ip()) && !allow_public_bind {
        return Err(Error::PublicBindNotAllowed(listen));
    }
    if !is_loopback(listen.ip()) {
        let mut stderr = std::io::stderr();
        let _ = writeln!(
            stderr,
            "warning: binding to non-loopback address {listen}; public bind is experimental and not official infrastructure"
        );
    }

    let store = helpers::open_store(config)?;
    let clock = Arc::new(SystemClock);
    let (bound_addr, server) = direct_sync::serve(store, clock, listen).await?;
    println!("listening on http://{bound_addr}");
    server.await?;
    Ok(())
}

fn is_loopback(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(addr) => addr == Ipv4Addr::LOCALHOST || addr.is_loopback(),
        IpAddr::V6(addr) => addr.is_loopback(),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Helpers(#[from] helpers::Error),
    #[error(transparent)]
    DirectSync(#[from] direct_sync::Error),
    #[error("public bind is not allowed for {0}; pass --allow-public-bind to opt in")]
    PublicBindNotAllowed(SocketAddr),
}
