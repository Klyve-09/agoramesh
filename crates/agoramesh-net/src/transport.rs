//! QUIC transport abstraction for inter-peer connections.

use std::net::SocketAddr;

/// A bound QUIC endpoint ready to accept or initiate connections.
#[derive(Debug)]
pub struct Endpoint {
    local_addr: SocketAddr,
}

/// An active connection to a remote peer.
#[derive(Debug)]
pub struct Connection {
    remote_addr: SocketAddr,
}

impl Endpoint {
    /// Creates a placeholder endpoint without opening a real socket.
    #[must_use]
    pub const fn bind(local_addr: SocketAddr) -> Self {
        Self { local_addr }
    }

    /// Returns the local socket address.
    #[must_use]
    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl Connection {
    /// Creates a placeholder connection handle.
    #[must_use]
    pub const fn new(remote_addr: SocketAddr) -> Self {
        Self { remote_addr }
    }

    /// Returns the remote socket address.
    #[must_use]
    pub const fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_roundtrips_local_addr() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
        let endpoint = Endpoint::bind(addr);
        assert_eq!(endpoint.local_addr(), addr);
    }

    #[test]
    fn connection_roundtrips_remote_addr() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 9090));
        let connection = Connection::new(addr);
        assert_eq!(connection.remote_addr(), addr);
    }
}
