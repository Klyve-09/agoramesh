//! Category object builder.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::canonical;
use crate::message::{self, Message, PROTOCOL_VERSION};
use crate::objects::{canonical_body, pubkey_hex, sha256_hex};

/// Minimal Phase 1 category charter anchor.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CharterAnchorBody {
    /// Charter text used to anchor the category ID.
    pub text: String,
    /// Protocol version used by the anchor.
    pub protocol_version: u32,
    /// Creation timestamp used by the anchor.
    pub created_at: DateTime<Utc>,
}

/// Signed body for a category object.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Body {
    /// `AgoraMesh` protocol version.
    pub protocol_version: u32,
    /// Creator Ed25519 public key encoded as lowercase hex.
    pub creator_pubkey: String,
    /// Stable lowercase hex category identifier.
    pub category_id: String,
    /// Display name shown to users.
    pub display_name: String,
    /// Short category topic or description.
    pub description: String,
    /// Creation timestamp used for category ID derivation.
    pub created_at: DateTime<Utc>,
    /// Lowercase SHA-256 hex digest of the initial charter anchor body.
    pub initial_charter_hash: String,
    /// Minimal Phase 1 charter anchor body.
    pub initial_charter: CharterAnchorBody,
}

#[derive(Serialize)]
struct CategoryIdInput<'a> {
    protocol_version: u32,
    creator_pubkey: &'a str,
    display_name: &'a str,
    created_at: &'a DateTime<Utc>,
    initial_charter_hash: &'a str,
}

/// Creates a signed category object with a deterministic category ID.
///
/// # Errors
///
/// Returns an error if any canonical JSON serialization step fails.
pub fn create(
    keypair: &crate::Keypair,
    created_at: DateTime<Utc>,
    display_name: impl Into<String>,
    description: impl Into<String>,
    initial_charter_text: impl Into<String>,
) -> Result<Message, message::Error> {
    let display_name = display_name.into();
    let description = description.into();
    let initial_charter = CharterAnchorBody {
        text: initial_charter_text.into(),
        protocol_version: PROTOCOL_VERSION,
        created_at,
    };
    let initial_charter_hash = sha256_hex(
        &canonical::to_vec(&initial_charter)
            .map_err(|error| message::Error::CanonicalJson(error.to_string()))?,
    );
    let creator_pubkey = pubkey_hex(keypair.identity().verifying_key().as_bytes());
    let input = CategoryIdInput {
        protocol_version: PROTOCOL_VERSION,
        creator_pubkey: &creator_pubkey,
        display_name: &display_name,
        created_at: &created_at,
        initial_charter_hash: &initial_charter_hash,
    };
    let category_id = sha256_hex(
        &canonical::to_vec(&input)
            .map_err(|error| message::Error::CanonicalJson(error.to_string()))?,
    );
    let body = Body {
        protocol_version: PROTOCOL_VERSION,
        creator_pubkey,
        category_id: category_id.clone(),
        display_name,
        description,
        created_at,
        initial_charter_hash,
        initial_charter,
    };

    Message::create(
        keypair,
        "category",
        created_at,
        category_id,
        canonical_body(&body)?,
    )
}
