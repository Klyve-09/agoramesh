//! Typed Phase 1 object builders.

use serde::{Deserialize, Serialize};

use crate::canonical;
use crate::message::{self, Body};

pub mod category;
pub mod comment;
pub mod post;
pub mod revocation_certificate;
pub mod user_profile;
pub mod validation;

/// Comment parent object kind.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParentKind {
    /// A top-level post.
    Post,
    /// Another comment.
    Comment,
}

pub(crate) fn canonical_body<T>(value: &T) -> Result<Body, message::Error>
where
    T: Serialize + ?Sized,
{
    canonical::to_vec(value)
        .map(Body::from)
        .map_err(|error| message::Error::CanonicalJson(error.to_string()))
}

pub(crate) fn pubkey_hex(pubkey: &[u8; 32]) -> String {
    message::hex_encode(pubkey)
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    message::hex_encode(&hasher.finalize())
}
