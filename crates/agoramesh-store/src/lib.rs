#![cfg_attr(not(test), warn(missing_docs))]

//! Persistent storage for Agoramesh messages and peer metadata.

pub mod db;

pub use db::{Connection, Store};
