//! Category ID canonicalization helpers.

use chrono::{DateTime, SecondsFormat, Utc};

use crate::message::{self, PROTOCOL_VERSION};
use crate::objects::sha256_hex;

/// Inputs that make up the stable Phase 1 `category_id` preimage.
#[derive(Clone, Copy, Debug)]
pub struct CategoryIdParts<'a> {
    /// Creator Ed25519 public key encoded as lowercase hex.
    pub creator_pubkey: &'a str,
    /// Exact display name used for the first category object.
    pub display_name: &'a str,
    /// Creation timestamp used for category identity.
    pub created_at: &'a DateTime<Utc>,
    /// Lowercase SHA-256 hex digest of the initial charter anchor.
    pub initial_charter_hash: &'a str,
}

/// Encodes the exact fixed-order byte sequence specified for `category_id`.
///
/// This intentionally does not call the shared canonical JSON key sorter: the
/// category-id spec fixes field order so non-Rust implementations can reproduce
/// the preimage by writing the five documented fields in order.
///
/// # Errors
///
/// Returns an error if a string field cannot be JSON-escaped.
pub fn canonical_bytes(parts: &CategoryIdParts<'_>) -> Result<Vec<u8>, message::Error> {
    let creator_pubkey = json_string(parts.creator_pubkey)?;
    let display_name = json_string(parts.display_name)?;
    let created_at = json_string(&parts.created_at.to_rfc3339_opts(SecondsFormat::Secs, true))?;
    let initial_charter_hash = json_string(parts.initial_charter_hash)?;

    let mut bytes = Vec::with_capacity(
        82_usize
            .saturating_add(creator_pubkey.len())
            .saturating_add(display_name.len())
            .saturating_add(created_at.len())
            .saturating_add(initial_charter_hash.len()),
    );
    bytes.extend_from_slice(b"{\"protocol_version\":");
    bytes.extend_from_slice(PROTOCOL_VERSION.to_string().as_bytes());
    bytes.extend_from_slice(b",\"creator_pubkey\":");
    bytes.extend_from_slice(creator_pubkey.as_bytes());
    bytes.extend_from_slice(b",\"display_name\":");
    bytes.extend_from_slice(display_name.as_bytes());
    bytes.extend_from_slice(b",\"created_at\":");
    bytes.extend_from_slice(created_at.as_bytes());
    bytes.extend_from_slice(b",\"initial_charter_hash\":");
    bytes.extend_from_slice(initial_charter_hash.as_bytes());
    bytes.extend_from_slice(b"}");
    Ok(bytes)
}

/// Computes the lowercase hex `category_id` for the supplied fixed-order parts.
///
/// # Errors
///
/// Returns an error if the preimage cannot be JSON-escaped.
pub fn compute(parts: &CategoryIdParts<'_>) -> Result<String, message::Error> {
    Ok(sha256_hex(&canonical_bytes(parts)?))
}

fn json_string(value: &str) -> Result<String, message::Error> {
    serde_json::to_string(value).map_err(|error| message::Error::CanonicalJson(error.to_string()))
}
