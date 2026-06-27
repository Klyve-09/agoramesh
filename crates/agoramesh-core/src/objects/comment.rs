//! Comment object builder.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::message::{self, Message, MessageId};
use crate::objects::{ParentKind, canonical_body, timestamp_seconds};

/// Signed body for a comment object.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Body {
    /// Category that contains this comment.
    pub category_id: String,
    /// Parent object kind.
    pub parent_kind: ParentKind,
    /// Parent object ID encoded as lowercase hex.
    pub parent_id: String,
    /// Comment text.
    pub text: String,
    /// Comment creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Creates a signed comment object.
///
/// # Errors
///
/// Returns an error if the body or signing payload cannot be canonicalized.
pub fn create(
    keypair: &crate::Keypair,
    category_id: impl Into<String>,
    parent_kind: ParentKind,
    parent_id: MessageId,
    text: impl Into<String>,
    created_at: DateTime<Utc>,
) -> Result<Message, message::Error> {
    let created_at = timestamp_seconds(created_at);
    let category_id = category_id.into();
    let body = Body {
        category_id: category_id.clone(),
        parent_kind,
        parent_id: parent_id.to_hex(),
        text: text.into(),
        created_at,
    };
    Message::create(
        keypair,
        "comment",
        created_at,
        category_id,
        canonical_body(&body)?,
    )
}
