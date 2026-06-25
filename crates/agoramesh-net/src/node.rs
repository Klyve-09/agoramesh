//! Mesh node lifecycle and peer coordination.

use std::net::SocketAddr;

use agoramesh_core::Identity;

/// Configuration for a mesh node.
#[derive(Clone, Debug)]
pub struct NodeConfig {
    /// Local address to bind the QUIC listener to.
    pub listen_addr: SocketAddr,
    /// Static public identity of this node.
    pub identity: Identity,
}

/// A running mesh node.
#[derive(Debug)]
pub struct Node {
    config: NodeConfig,
}

impl Node {
    /// Creates a new node without starting network I/O.
    #[must_use]
    pub const fn new(config: NodeConfig) -> Self {
        Self { config }
    }

    /// Returns a reference to the node configuration.
    #[must_use]
    pub const fn config(&self) -> &NodeConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agoramesh_core::Keypair;

    #[test]
    fn node_holds_config() {
        let identity = Keypair::generate().identity();
        let listen_addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let config = NodeConfig {
            listen_addr,
            identity: identity.clone(),
        };
        let node = Node::new(config);
        assert_eq!(node.config().listen_addr, listen_addr);
        assert_eq!(node.config().identity, identity);
    }
}
