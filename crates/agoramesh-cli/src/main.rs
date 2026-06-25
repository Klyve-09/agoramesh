#![allow(
    missing_docs,
    reason = "placeholder binary until CLI commands are wired"
)]

use agoramesh_core::Keypair;
use agoramesh_net::{Node, NodeConfig};
use agoramesh_store::Connection;
use std::net::SocketAddr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let keypair = Keypair::generate();
    let identity = keypair.identity();
    let config = NodeConfig {
        listen_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
        identity,
    };
    let _node = Node::new(config);
    let _store = Connection::open_in_memory()?;
    Ok(())
}
