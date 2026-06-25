#![cfg_attr(not(test), warn(missing_docs))]

//! Core primitives shared across the Agoramesh mesh.

pub mod canonical;
pub mod identity;
pub mod message;

pub use identity::{Identity, Keypair};
pub use message::{
    Body, Clock, Message, MessageId, SignedPayload, SkewWarning, SystemClock, Timestamp,
    TransportMetadata, Verification,
};
