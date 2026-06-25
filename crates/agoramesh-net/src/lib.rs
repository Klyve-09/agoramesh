#![cfg_attr(not(test), warn(missing_docs))]

//! Network layer for the Agoramesh peer-to-peer mesh.

pub mod node;
pub mod transport;

pub use node::{Node, NodeConfig};
