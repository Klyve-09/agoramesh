//! Phase 1 object validation.
//!
//! This module checks the semantic invariants of the five Phase 1 object types
//! after the generic signature / object-id / author checks in [`Message::verify`]
//! have already passed. Validation is intentionally separate from the accepted
//! store so that store policy can stay focused on integrity and clock skew.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::canonical;
use crate::message::{self, Message, MessageId, PROTOCOL_VERSION};

/// Errors returned by [`validate_phase1_message`].
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum Error {
    /// The message type is not a known Phase 1 object type.
    #[error("unknown phase1 object type: {0}")]
    UnknownType(String),

    /// The message is a known object type but not the projection type requested.
    #[error("wrong phase1 object kind: expected {expected}, got {actual}")]
    WrongKind {
        /// Expected signed payload kind.
        expected: String,
        /// Actual signed payload kind.
        actual: String,
    },

    /// The body could not be parsed as the declared object type.
    #[error("invalid body for {kind}: {message}")]
    InvalidBody {
        /// Declared object kind.
        kind: String,
        /// Underlying parse error.
        message: String,
    },

    /// A field inside the body does not match the signed envelope.
    #[error("{field} mismatch: body has {body_value}, envelope has {envelope_value}")]
    FieldMismatch {
        /// Field name.
        field: String,
        /// Value found in the body.
        body_value: String,
        /// Value found in the signed envelope.
        envelope_value: String,
    },

    /// A required string field is empty.
    #[error("{field} must not be empty")]
    EmptyField {
        /// Field name.
        field: String,
    },

    /// The author of the message does not match the object semantics.
    #[error("{field} must match author: expected {expected}, got {actual}")]
    AuthorMismatch {
        /// Field name.
        field: String,
        /// Expected author-related value.
        expected: String,
        /// Actual value found.
        actual: String,
    },

    /// A recomputed hash does not match the value claimed in the body.
    #[error("recomputed {field} does not match body")]
    HashMismatch {
        /// Field name.
        field: String,
    },

    /// A hex-encoded identifier inside the body is not well formed.
    #[error("invalid hex identifier in {field}: {message}")]
    InvalidHex {
        /// Field name.
        field: String,
        /// Underlying error.
        message: String,
    },

    /// A revocation certificate targets a key other than its author's.
    #[error("phase 1 revocation certificates must be self-revocations")]
    ThirdPartyRevocation,
}

/// Validates a message against Phase 1 type-specific invariants.
///
/// This does **not** re-run signature verification. Callers must run
/// [`Message::verify`] and enforce clock policy before reaching accepted store.
///
/// # Errors
///
/// Returns [`Error`] when the object type is unknown or any type-specific
/// invariant is violated.
pub fn validate_phase1_message(message: &Message) -> Result<(), Error> {
    match message.signed_payload().kind() {
        "user_profile" => validate_user_profile(message),
        "category" => validate_category(message),
        "post" => validate_post(message),
        "comment" => validate_comment(message),
        "revocation_certificate" => validate_revocation_certificate(message),
        other => Err(Error::UnknownType(other.to_owned())),
    }
}

fn parse_body<T>(message: &Message, kind: &str) -> Result<T, Error>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_slice(message.body()).map_err(|error| Error::InvalidBody {
        kind: kind.to_owned(),
        message: error.to_string(),
    })
}

fn author_pubkey_hex(message: &Message) -> String {
    message::hex_encode(message.signed_payload().author_pubkey())
}

fn require_same(field: &str, body: &str, envelope: &str) -> Result<(), Error> {
    if body != envelope {
        return Err(Error::FieldMismatch {
            field: field.to_owned(),
            body_value: body.to_owned(),
            envelope_value: envelope.to_owned(),
        });
    }
    Ok(())
}

fn require_non_empty(field: &str, value: &str) -> Result<(), Error> {
    if value.trim().is_empty() {
        return Err(Error::EmptyField {
            field: field.to_owned(),
        });
    }
    Ok(())
}

fn require_author(field: &str, expected: &str, actual: &str) -> Result<(), Error> {
    if expected != actual {
        return Err(Error::AuthorMismatch {
            field: field.to_owned(),
            expected: expected.to_owned(),
            actual: actual.to_owned(),
        });
    }
    Ok(())
}

fn validate_user_profile(message: &Message) -> Result<(), Error> {
    let body: user_profile::Body = parse_body(message, "user_profile")?;
    require_non_empty("display_name", &body.display_name)?;

    let expected_scope = format!("user:{}", author_pubkey_hex(message));
    require_same("scope", message.signed_payload().scope(), &expected_scope)?;
    Ok(())
}

fn validate_category(message: &Message) -> Result<(), Error> {
    let body: category::Body = parse_body(message, "category")?;

    if body.protocol_version != PROTOCOL_VERSION {
        return Err(Error::FieldMismatch {
            field: "protocol_version".to_owned(),
            body_value: body.protocol_version.to_string(),
            envelope_value: PROTOCOL_VERSION.to_string(),
        });
    }

    let author = author_pubkey_hex(message);
    require_author("creator_pubkey", &author, &body.creator_pubkey)?;

    let envelope_created_at = message.signed_payload().created_at().datetime();
    require_same(
        "created_at",
        &rfc3339(&body.created_at),
        &rfc3339(&envelope_created_at),
    )?;

    require_same(
        "category_id",
        &body.category_id,
        message.signed_payload().scope(),
    )?;

    require_non_empty("display_name", &body.display_name)?;

    if body.initial_charter.protocol_version != PROTOCOL_VERSION {
        return Err(Error::FieldMismatch {
            field: "initial_charter.protocol_version".to_owned(),
            body_value: body.initial_charter.protocol_version.to_string(),
            envelope_value: PROTOCOL_VERSION.to_string(),
        });
    }

    require_same(
        "initial_charter.created_at",
        &rfc3339(&body.initial_charter.created_at),
        &rfc3339(&envelope_created_at),
    )?;

    let anchor = category::CharterAnchorBody {
        text: body.initial_charter.text.clone(),
        protocol_version: body.initial_charter.protocol_version,
        created_at: body.initial_charter.created_at,
    };
    let expected_initial_charter_hash =
        hash_canonical(&anchor).map_err(|error| Error::InvalidBody {
            kind: "category".to_owned(),
            message: error.to_string(),
        })?;
    if body.initial_charter_hash != expected_initial_charter_hash {
        return Err(Error::HashMismatch {
            field: "initial_charter_hash".to_owned(),
        });
    }

    let expected_category_id = compute_category_id(&CategoryIdParts {
        creator_pubkey: &author,
        display_name: &body.display_name,
        created_at: &body.created_at,
        initial_charter_hash: &body.initial_charter_hash,
    })?;
    if body.category_id != expected_category_id {
        return Err(Error::HashMismatch {
            field: "category_id".to_owned(),
        });
    }

    Ok(())
}

fn validate_post(message: &Message) -> Result<(), Error> {
    let body: post::Body = parse_body(message, "post")?;
    require_non_empty("text", &body.text)?;

    let envelope_created_at = message.signed_payload().created_at().datetime();
    require_same(
        "created_at",
        &rfc3339(&body.created_at),
        &rfc3339(&envelope_created_at),
    )?;

    require_same(
        "category_id",
        &body.category_id,
        message.signed_payload().scope(),
    )?;

    Ok(())
}

fn validate_comment(message: &Message) -> Result<(), Error> {
    let body: comment::Body = parse_body(message, "comment")?;
    require_non_empty("text", &body.text)?;

    let envelope_created_at = message.signed_payload().created_at().datetime();
    require_same(
        "created_at",
        &rfc3339(&body.created_at),
        &rfc3339(&envelope_created_at),
    )?;

    require_same(
        "category_id",
        &body.category_id,
        message.signed_payload().scope(),
    )?;

    let _parent_id = MessageId::from_hex(&body.parent_id).map_err(|error| Error::InvalidHex {
        field: "parent_id".to_owned(),
        message: error.to_string(),
    })?;

    Ok(())
}

fn validate_revocation_certificate(message: &Message) -> Result<(), Error> {
    let body: revocation_certificate::Body = parse_body(message, "revocation_certificate")?;

    let author = author_pubkey_hex(message);
    require_author("revoked_pubkey", &author, &body.revoked_pubkey)?;

    let expected_scope = format!("revocation:{author}");
    require_same("scope", message.signed_payload().scope(), &expected_scope)?;

    let envelope_created_at = message.signed_payload().created_at().datetime();
    require_same(
        "effective_at",
        &rfc3339(&body.effective_at),
        &rfc3339(&envelope_created_at),
    )?;

    require_non_empty("reason_code", &body.reason_code)?;

    if let Some(replacement) = &body.replacement_pubkey {
        if replacement.len() != 64 {
            return Err(Error::InvalidHex {
                field: "replacement_pubkey".to_owned(),
                message: "expected 64 hex characters".to_owned(),
            });
        }
    }

    if body.revoked_pubkey != author {
        return Err(Error::ThirdPartyRevocation);
    }

    Ok(())
}

fn rfc3339(value: &DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

use crate::objects::category_id::CategoryIdParts;
use crate::objects::{category, comment, post, revocation_certificate, user_profile};
use sha2::{Digest, Sha256};

fn hash_canonical<T>(value: &T) -> Result<String, message::Error>
where
    T: serde::Serialize,
{
    let bytes = canonical::to_vec(value)
        .map_err(|error| message::Error::CanonicalJson(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(message::hex_encode(&hasher.finalize()))
}

fn compute_category_id(parts: &CategoryIdParts<'_>) -> Result<String, Error> {
    crate::objects::category_id::compute(parts).map_err(|error| Error::InvalidBody {
        kind: "category".to_owned(),
        message: error.to_string(),
    })
}
