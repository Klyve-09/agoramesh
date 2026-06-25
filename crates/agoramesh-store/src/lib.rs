#![cfg_attr(not(test), warn(missing_docs))]

//! Persistent storage for Agoramesh messages and peer metadata.

pub mod db;
pub mod store;

pub use db::{Connection, SqliteStore};
pub use store::{Error, InMemoryStore, Store};
