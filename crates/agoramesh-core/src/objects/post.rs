//! Post object builder.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::message::{self, Message};
use crate::objects::canonical_body;

/// Signed body for a post object.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Body {
    /// Category that contains this post.
    pub category_id: String,
    /// Post text.
    pub text: String,
    /// Post creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Creates a signed post object.
///
/// # Errors
///
/// Returns an error if the body or signing payload cannot be canonicalized.
pub fn create(
    keypair: &crate::Keypair,
    category_id: impl Into<String>,
    text: impl Into<String>,
    created_at: DateTime<Utc>,
) -> Result<Message, message::Error> {
    let category_id = category_id.into();
    let body = Body {
        category_id: category_id.clone(),
        text: text.into(),
        created_at,
    };
    Message::create(
        keypair,
        "post",
        created_at,
        category_id,
        canonical_body(&body)?,
    )
}
