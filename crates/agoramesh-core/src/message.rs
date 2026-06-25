//! Wire-safe message types used throughout the mesh.

use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, SecondsFormat, Utc};
use ed25519_dalek::{Signature, Verifier};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::canonical;

/// Agoramesh wire protocol version carried in every message.
pub const PROTOCOL_VERSION: u32 = 1;

/// Agoramesh message schema version carried in every message.
pub const SCHEMA_VERSION: u32 = 1;

/// Maximum clock skew accepted before a message is rejected.
pub const CLOCK_SKEW_REJECT_SECONDS: i64 = 5 * 60;

/// A stable, content-addressed identifier for a message.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct MessageId([u8; 32]);

impl MessageId {
    /// Returns the raw 32-byte identifier.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for MessageId {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

/// A signed, self-contained message broadcast through the mesh.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Message {
    id: MessageId,
    author_id: crate::Identity,
    signature: Signature,
    signed_payload: SignedPayload,
    transport_metadata: TransportMetadata,
}

/// The signed payload that defines a message's identity and integrity.
///
/// This is the only data covered by the signature and the object ID.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SignedPayload {
    #[serde(rename = "type")]
    kind: String,
    protocol_version: u32,
    schema_version: u32,
    created_at: Timestamp,
    author_pubkey: [u8; 32],
    scope: String,
    body: Body,
}

impl SignedPayload {
    fn canonical_bytes(&self) -> Result<Vec<u8>, Error> {
        canonical::to_vec(self).map_err(|error| Error::CanonicalJson(error.to_string()))
    }

    /// Returns the payload kind/type.
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns the message scope (category).
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// Returns the creation timestamp.
    #[must_use]
    pub const fn created_at(&self) -> &Timestamp {
        &self.created_at
    }

    /// Returns the opaque body bytes.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body.0
    }
}

/// An RFC 3339 timestamp used in signed payloads.
///
/// ADR 0001 requires timestamps to be RFC 3339 strings in Canonical JSON.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Creates a timestamp from a fixed UTC datetime.
    #[must_use]
    pub const fn new(datetime: DateTime<Utc>) -> Self {
        Self(datetime)
    }

    /// Returns the underlying UTC datetime.
    #[must_use]
    pub const fn datetime(&self) -> DateTime<Utc> {
        self.0
    }
}

impl Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_rfc3339_opts(SecondsFormat::Secs, true))
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&string)
            .map(|datetime| Self(datetime.with_timezone(&Utc)))
            .map_err(serde::de::Error::custom)
    }
}

/// Binary body encoded as base64url string in JSON.
///
/// ADR 0001 requires binary data to be base64url strings inside JSON values.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Body(Vec<u8>);

impl Body {
    /// Creates a body from raw bytes.
    #[must_use]
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    /// Returns the raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for Body {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl From<&[u8]> for Body {
    fn from(value: &[u8]) -> Self {
        Self(value.to_vec())
    }
}

impl From<&Vec<u8>> for Body {
    fn from(value: &Vec<u8>) -> Self {
        Self(value.clone())
    }
}

impl<const N: usize> From<&[u8; N]> for Body {
    fn from(value: &[u8; N]) -> Self {
        Self(value.to_vec())
    }
}

impl Serialize for Body {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&URL_SAFE_NO_PAD.encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for Body {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        URL_SAFE_NO_PAD
            .decode(string)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

/// Non-signed transport metadata.
///
/// This data is not covered by the signature and must not affect the object
/// ID or the validity of the signed payload.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransportMetadata {
    /// An optional hop count or TTL placeholder.
    pub hop_count: u32,
}

/// Result of validating a message against its claimed object ID and signature.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Verification {
    /// The message is fully valid and within the accepted clock skew window.
    Accepted,
    /// The message is valid but its `created_at` is outside the tight skew
    /// window; callers may quarantine or surface a warning.
    AcceptedWithWarning(SkewWarning),
    /// The message failed verification and must be rejected.
    Rejected(Error),
}

/// Warning produced when a message's `created_at` is slightly off.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SkewWarning {
    /// The message was created in the future relative to the verifier clock.
    Future,
    /// The message was created in the past relative to the verifier clock.
    Past,
}

/// A source of UTC timestamps used to validate `created_at`.
///
/// Production code uses the system clock; tests can inject a fixed value.
pub trait Clock: fmt::Debug {
    /// Returns the current UTC datetime.
    fn now(&self) -> DateTime<Utc>;
}

/// System clock implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

impl Message {
    /// Creates and signs a new message from its logical parts.
    ///
    /// # Errors
    ///
    /// Returns an error if canonical JSON serialization of the signing
    /// payload fails.
    pub fn create(
        keypair: &crate::Keypair,
        created_at: DateTime<Utc>,
        scope: String,
        body: impl Into<Body>,
    ) -> Result<Self, Error> {
        let author_id = keypair.identity();
        let signed_payload = SignedPayload {
            kind: "message".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            schema_version: SCHEMA_VERSION,
            created_at: Timestamp::new(created_at),
            author_pubkey: *author_id.verifying_key().as_bytes(),
            scope,
            body: body.into(),
        };
        let canonical = signed_payload.canonical_bytes()?;
        let id = hash_bytes(&canonical);
        let signature = keypair.sign(&canonical);
        Ok(Self {
            id,
            author_id,
            signature,
            signed_payload,
            transport_metadata: TransportMetadata::default(),
        })
    }

    /// Verifies the message signature, object ID, and author consistency.
    ///
    /// # Errors
    ///
    /// Never returns `Err`; errors are represented as `Verification::Rejected`.
    pub fn verify(&self, clock: &dyn Clock) -> Verification {
        let canonical = match self.signed_payload.canonical_bytes() {
            Ok(bytes) => bytes,
            Err(error) => return Verification::Rejected(error),
        };

        if self.signed_payload.author_pubkey != *self.author_id.verifying_key().as_bytes() {
            return Verification::Rejected(Error::AuthorPubkeyMismatch);
        }

        let expected_id = hash_bytes(&canonical);
        if expected_id != self.id {
            return Verification::Rejected(Error::ObjectIdMismatch);
        }

        if let Err(error) = self
            .author_id
            .verifying_key()
            .verify(&canonical, &self.signature)
        {
            return Verification::Rejected(Error::InvalidSignature {
                inner: error.to_string(),
            });
        }

        classify_skew(self.signed_payload.created_at.datetime(), clock.now())
    }

    /// Returns the message identifier.
    #[must_use]
    pub const fn id(&self) -> MessageId {
        self.id
    }

    /// Replaces the message identifier.
    ///
    /// # Safety
    ///
    /// This breaks the object ID invariant and should only be used in tests
    /// that intentionally construct malformed messages.
    #[cfg(test)]
    pub const fn set_id(&mut self, id: MessageId) {
        self.id = id;
    }

    /// Returns the author identity.
    #[must_use]
    pub const fn author_id(&self) -> &crate::Identity {
        &self.author_id
    }

    /// Returns the Ed25519 signature over the canonical signing payload.
    #[must_use]
    pub const fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Returns the signed payload.
    #[must_use]
    pub const fn signed_payload(&self) -> &SignedPayload {
        &self.signed_payload
    }

    /// Returns the non-signed transport metadata.
    #[must_use]
    pub const fn transport_metadata(&self) -> &TransportMetadata {
        &self.transport_metadata
    }

    /// Returns the opaque body bytes stored inside the signed payload.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        self.signed_payload.body.as_bytes()
    }

    /// Replaces the body in the signed payload.
    ///
    /// # Safety
    ///
    /// This breaks the signature/object ID invariant and should only be used
    /// in tests that intentionally construct malformed messages.
    #[cfg(any(test, feature = "test-helpers"))]
    pub fn set_body(&mut self, body: Body) {
        self.signed_payload.body = body;
    }

    /// Replaces the creation timestamp in the signed payload.
    ///
    /// # Safety
    ///
    /// This breaks the signature/object ID invariant and should only be used
    /// in tests that intentionally construct malformed messages.
    #[cfg(any(test, feature = "test-helpers"))]
    pub const fn set_created_at(&mut self, created_at: Timestamp) {
        self.signed_payload.created_at = created_at;
    }
}

fn classify_skew(created_at: DateTime<Utc>, now: DateTime<Utc>) -> Verification {
    let delta = created_at.signed_duration_since(now).num_seconds();

    if delta > CLOCK_SKEW_REJECT_SECONDS {
        return Verification::Rejected(Error::ClockSkewTooLarge { seconds: delta });
    }
    if delta > 0 {
        return Verification::AcceptedWithWarning(SkewWarning::Future);
    }
    Verification::Accepted
}

fn hash_bytes(bytes: &[u8]) -> MessageId {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    MessageId(hasher.finalize().into())
}

/// Errors that can occur while constructing or validating a message.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum Error {
    /// Failed to produce canonical JSON for the signing payload.
    #[error("failed to canonicalize signing payload: {0}")]
    CanonicalJson(String),

    /// The recomputed object ID does not match the claimed ID.
    #[error("object id does not match canonical signing payload")]
    ObjectIdMismatch,

    /// The signed author public key does not match the claimed author identity.
    #[error("signed author public key does not match author identity")]
    AuthorPubkeyMismatch,

    /// The signature is invalid for the claimed author and payload.
    #[error("invalid signature: {inner}")]
    InvalidSignature {
        /// Human-readable signature verification error.
        inner: String,
    },

    /// The message timestamp is too far from the verifier's clock.
    #[error("clock skew too large: {seconds} seconds")]
    ClockSkewTooLarge {
        /// Skew in seconds (positive means future-dated).
        seconds: i64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Keypair;
    use chrono::TimeDelta;

    #[derive(Debug, Default)]
    struct FixedClock {
        now: DateTime<Utc>,
    }

    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.now
        }
    }

    fn utc(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).expect("valid timestamp")
    }

    #[test]
    fn create_produces_valid_verifiable_message() {
        let keypair = Keypair::generate();
        let now = utc(1_700_000_000);
        let body = b"hello mesh".to_vec();
        let message =
            Message::create(&keypair, now, "test".to_owned(), &body).expect("create message");
        assert_eq!(message.author_id(), &keypair.identity());
        assert_eq!(message.body(), body);

        let clock = FixedClock { now };
        assert_eq!(message.verify(&clock), Verification::Accepted);
    }

    #[test]
    fn verify_rejects_tampered_body() {
        let keypair = Keypair::generate();
        let now = utc(1_700_000_000);
        let body = b"hello mesh".to_vec();
        let mut message =
            Message::create(&keypair, now, "test".to_owned(), &body).expect("create message");
        message.signed_payload.body = Body::from(b"evil".as_slice());

        let clock = FixedClock { now };
        assert!(matches!(
            message.verify(&clock),
            Verification::Rejected(Error::ObjectIdMismatch | Error::InvalidSignature { .. })
        ));
    }

    #[test]
    fn verify_rejects_tampered_created_at() {
        let keypair = Keypair::generate();
        let now = utc(1_700_000_000);
        let body = b"hello mesh".to_vec();
        let mut message =
            Message::create(&keypair, now, "test".to_owned(), &body).expect("create message");
        message.signed_payload.created_at = Timestamp::new(now + TimeDelta::seconds(1));

        let clock = FixedClock { now };
        assert!(matches!(
            message.verify(&clock),
            Verification::Rejected(Error::ObjectIdMismatch | Error::InvalidSignature { .. })
        ));
    }

    #[test]
    fn verify_rejects_wrong_author() {
        let alice = Keypair::generate();
        let bob = Keypair::generate();
        let now = utc(1_700_000_000);
        let body = b"hello mesh".to_vec();
        let mut message =
            Message::create(&alice, now, "test".to_owned(), &body).expect("create message");
        message.author_id = bob.identity();

        let clock = FixedClock { now };
        assert!(matches!(
            message.verify(&clock),
            Verification::Rejected(
                Error::ObjectIdMismatch
                    | Error::AuthorPubkeyMismatch
                    | Error::InvalidSignature { .. }
            )
        ));
    }

    #[test]
    fn verify_rejects_author_pubkey_mismatch() {
        let alice = Keypair::generate();
        let bob = Keypair::generate();
        let now = utc(1_700_000_000);
        let body = b"hello mesh".to_vec();
        let mut message =
            Message::create(&alice, now, "test".to_owned(), &body).expect("create message");
        message.signed_payload.author_pubkey = *bob.identity().verifying_key().as_bytes();
        // Keep the original object id and signature valid for Alice, but change
        // the signed pubkey to Bob. The pubkey check must fail before signature.

        let clock = FixedClock { now };
        assert_eq!(
            message.verify(&clock),
            Verification::Rejected(Error::AuthorPubkeyMismatch)
        );
    }

    #[test]
    fn verify_warns_on_small_future_skew() {
        let keypair = Keypair::generate();
        let now = utc(1_700_000_000);
        let created_at = now + TimeDelta::seconds(60);
        let body = b"hello mesh".to_vec();
        let message = Message::create(&keypair, created_at, "test".to_owned(), &body)
            .expect("create message");

        let clock = FixedClock { now };
        assert_eq!(
            message.verify(&clock),
            Verification::AcceptedWithWarning(SkewWarning::Future)
        );
    }

    #[test]
    fn verify_accepts_small_past_skew() {
        let keypair = Keypair::generate();
        let now = utc(1_700_000_000);
        let created_at = now - TimeDelta::seconds(60);
        let body = b"hello mesh".to_vec();
        let message = Message::create(&keypair, created_at, "test".to_owned(), &body)
            .expect("create message");

        let clock = FixedClock { now };
        assert_eq!(message.verify(&clock), Verification::Accepted);
    }

    #[test]
    fn verify_rejects_large_future_skew() {
        let keypair = Keypair::generate();
        let now = utc(1_700_000_000);
        let created_at = now + TimeDelta::seconds(CLOCK_SKEW_REJECT_SECONDS + 1);
        let body = b"hello mesh".to_vec();
        let message = Message::create(&keypair, created_at, "test".to_owned(), &body)
            .expect("create message");

        let clock = FixedClock { now };
        assert!(matches!(
            message.verify(&clock),
            Verification::Rejected(Error::ClockSkewTooLarge { .. })
        ));
    }

    #[test]
    fn verify_accepts_large_past_skew() {
        let keypair = Keypair::generate();
        let now = utc(1_700_000_000);
        let created_at = now - TimeDelta::seconds(CLOCK_SKEW_REJECT_SECONDS + 1);
        let body = b"hello mesh".to_vec();
        let message = Message::create(&keypair, created_at, "test".to_owned(), &body)
            .expect("create message");

        let clock = FixedClock { now };
        assert_eq!(message.verify(&clock), Verification::Accepted);
    }

    #[test]
    fn transport_metadata_does_not_affect_signature() {
        let keypair = Keypair::generate();
        let now = utc(1_700_000_000);
        let body = b"hello mesh".to_vec();
        let mut message =
            Message::create(&keypair, now, "test".to_owned(), &body).expect("create message");
        message.transport_metadata.hop_count = 42;

        let clock = FixedClock { now };
        assert_eq!(message.verify(&clock), Verification::Accepted);
    }

    #[test]
    fn rfc3339_timestamp_roundtrips_through_canonical_json() {
        let keypair = Keypair::generate();
        let now = utc(1_700_000_000);
        let message =
            Message::create(&keypair, now, "test".to_owned(), b"hello mesh").expect("create");
        let json = canonical::to_vec(&message.signed_payload).expect("canonicalize");
        let text = String::from_utf8(json).expect("utf8");
        assert!(
            text.contains("\"created_at\":\"2023-11-14T22:13:20Z\""),
            "actual: {text}"
        );
    }

    #[test]
    fn body_serializes_as_base64url_string() {
        let keypair = Keypair::generate();
        let now = utc(1_700_000_000);
        let message =
            Message::create(&keypair, now, "test".to_owned(), b"hello mesh").expect("create");
        let json = canonical::to_vec(&message.signed_payload).expect("canonicalize");
        let text = String::from_utf8(json).expect("utf8");
        assert!(text.contains("\"body\":\"aGVsbG8gbWVzaA\""));
    }
}
