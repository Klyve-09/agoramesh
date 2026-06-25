//! Revocation certificate object builder and validation helper.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::message::{self, Message};
use crate::objects::{canonical_body, pubkey_hex};

/// Signed body for a revocation certificate object.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Body {
    /// Revoked Ed25519 public key encoded as lowercase hex.
    pub revoked_pubkey: String,
    /// Replacement Ed25519 public key encoded as lowercase hex, if available.
    pub replacement_pubkey: Option<String>,
    /// Time from which the revoked key must no longer create accepted objects.
    pub effective_at: DateTime<Utc>,
    /// Machine-readable reason code.
    pub reason_code: String,
}

/// Errors returned while applying a revocation certificate.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum ValidationError {
    /// The supplied certificate is not a revocation certificate object.
    #[error("message is not a revocation certificate")]
    WrongKind,
    /// The certificate body is not valid JSON for this object type.
    #[error("invalid revocation certificate body: {0}")]
    InvalidBody(String),
}

/// Creates a signed revocation certificate object.
///
/// # Errors
///
/// Returns an error if the body or signing payload cannot be canonicalized.
pub fn create(
    keypair: &crate::Keypair,
    revoked_pubkey: [u8; 32],
    replacement_pubkey: Option<[u8; 32]>,
    effective_at: DateTime<Utc>,
    reason_code: impl Into<String>,
) -> Result<Message, message::Error> {
    let revoked_pubkey = pubkey_hex(&revoked_pubkey);
    let body = Body {
        replacement_pubkey: replacement_pubkey.map(|pubkey| pubkey_hex(&pubkey)),
        effective_at,
        reason_code: reason_code.into(),
        revoked_pubkey: revoked_pubkey.clone(),
    };
    Message::create(
        keypair,
        "revocation_certificate",
        effective_at,
        format!("revocation:{revoked_pubkey}"),
        canonical_body(&body)?,
    )
}

/// Returns whether a message is still allowed by the supplied certificate.
///
/// # Errors
///
/// Returns an error if the certificate kind or body is invalid.
pub fn allows_message(certificate: &Message, message: &Message) -> Result<bool, ValidationError> {
    if certificate.signed_payload().kind() != "revocation_certificate" {
        return Err(ValidationError::WrongKind);
    }
    let body: Body = serde_json::from_slice(certificate.body())
        .map_err(|error| ValidationError::InvalidBody(error.to_string()))?;
    let author_pubkey = pubkey_hex(message.author_id().verifying_key().as_bytes());

    Ok(author_pubkey != body.revoked_pubkey
        || message.signed_payload().created_at().datetime() < body.effective_at)
}
