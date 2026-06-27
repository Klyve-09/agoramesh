#![cfg_attr(not(test), warn(missing_docs))]

//! Core primitives shared across the Agoramesh mesh.

pub mod canonical;
pub mod identity;
pub mod message;
pub mod objects;

pub use identity::{Identity, Keypair};
pub use message::{Clock, Message, MessageId, SkewWarning, SystemClock, Verification};
pub use objects::acceptance::{AcceptanceContext, AcceptedPhase1Message, Phase1Acceptance};
