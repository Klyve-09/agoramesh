//! User profile object builder.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::message::{self, Message};
use crate::objects::{canonical_body, pubkey_hex, timestamp_seconds};

/// Signed body for a user profile object.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Body {
    /// Display name shown to users.
    pub display_name: String,
    /// Optional short biography.
    pub bio: Option<String>,
}

/// Creates a signed user profile object.
///
/// # Errors
///
/// Returns an error if the body or signing payload cannot be canonicalized.
pub fn create(
    keypair: &crate::Keypair,
    created_at: DateTime<Utc>,
    display_name: impl Into<String>,
    bio: Option<impl Into<String>>,
) -> Result<Message, message::Error> {
    let created_at = timestamp_seconds(created_at);
    let body = Body {
        display_name: display_name.into(),
        bio: bio.map(Into::into),
    };
    let author_pubkey = pubkey_hex(keypair.identity().verifying_key().as_bytes());
    Message::create(
        keypair,
        "user_profile",
        created_at,
        format!("user:{author_pubkey}"),
        canonical_body(&body)?,
    )
}
