//! Wire-safe message types used throughout the mesh.

use std::fmt;

use ed25519_dalek::{Signature, Verifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::canonical;

/// Agoramesh wire protocol version carried in every message.
pub const PROTOCOL_VERSION: u32 = 1;

/// Agoramesh message schema version carried in every message.
pub const SCHEMA_VERSION: u32 = 1;

/// Maximum clock skew accepted before a message is rejected, in seconds.
pub const CLOCK_SKEW_REJECT_SECONDS: i64 = 5 * 60;

/// A stable, content-addressed identifier for a message.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
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
    created_at: i64,
    author_pubkey: [u8; 32],
    scope: String,
    body: Vec<u8>,
}

impl SignedPayload {
    fn canonical_bytes(&self) -> Result<Vec<u8>, Error> {
        canonical::to_vec(self).map_err(|error| Error::CanonicalJson(error.to_string()))
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

/// A source of Unix timestamps used to validate `created_at`.
///
/// Production code uses the system clock; tests can inject a fixed value.
pub trait Clock: fmt::Debug {
    /// Returns the current Unix timestamp in seconds.
    fn now(&self) -> i64;
}

/// System clock implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| {
                i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
            })
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
        created_at: i64,
        scope: String,
        body: &[u8],
    ) -> Result<Self, Error> {
        let author_id = keypair.identity();
        let signed_payload = SignedPayload {
            kind: "message".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            schema_version: SCHEMA_VERSION,
            created_at,
            author_pubkey: *author_id.verifying_key().as_bytes(),
            scope,
            body: body.to_owned(),
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

    /// Verifies the message signature and object ID, and checks clock skew.
    ///
    /// # Errors
    ///
    /// Never returns `Err`; errors are represented as `Verification::Rejected`.
    pub fn verify(&self, clock: &dyn Clock) -> Verification {
        let canonical = match self.signed_payload.canonical_bytes() {
            Ok(bytes) => bytes,
            Err(error) => return Verification::Rejected(error),
        };

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

        classify_skew(self.signed_payload.created_at, clock.now())
    }

    /// Returns the message identifier.
    #[must_use]
    pub const fn id(&self) -> MessageId {
        self.id
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
        &self.signed_payload.body
    }
}

fn classify_skew(created_at: i64, now: i64) -> Verification {
    let delta = created_at.saturating_sub(now);
    let abs_delta = delta.checked_abs().unwrap_or(i64::MAX);

    if delta > CLOCK_SKEW_REJECT_SECONDS {
        return Verification::Rejected(Error::ClockSkewTooLarge { seconds: abs_delta });
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

    /// The signature is invalid for the claimed author and payload.
    #[error("invalid signature: {inner}")]
    InvalidSignature {
        /// Human-readable signature verification error.
        inner: String,
    },

    /// The message timestamp is too far from the verifier's clock.
    #[error("clock skew too large: {seconds} seconds")]
    ClockSkewTooLarge {
        /// Absolute skew in seconds.
        seconds: i64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Keypair;

    #[derive(Debug, Default)]
    struct FixedClock {
        now: i64,
    }

    impl Clock for FixedClock {
        fn now(&self) -> i64 {
            self.now
        }
    }

    #[test]
    fn create_produces_valid_verifiable_message() {
        let keypair = Keypair::generate();
        let now = 1_700_000_000;
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
        let now = 1_700_000_000;
        let body = b"hello mesh".to_vec();
        let mut message =
            Message::create(&keypair, now, "test".to_owned(), &body).expect("create message");
        message.signed_payload.body = b"evil".to_vec();

        let clock = FixedClock { now };
        assert!(matches!(
            message.verify(&clock),
            Verification::Rejected(Error::ObjectIdMismatch | Error::InvalidSignature { .. })
        ));
    }

    #[test]
    fn verify_rejects_tampered_created_at() {
        let keypair = Keypair::generate();
        let now = 1_700_000_000;
        let body = b"hello mesh".to_vec();
        let mut message =
            Message::create(&keypair, now, "test".to_owned(), &body).expect("create message");
        message.signed_payload.created_at = now + 1;

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
        let now = 1_700_000_000;
        let body = b"hello mesh".to_vec();
        let mut message =
            Message::create(&alice, now, "test".to_owned(), &body).expect("create message");
        message.author_id = bob.identity();

        let clock = FixedClock { now };
        assert!(matches!(
            message.verify(&clock),
            Verification::Rejected(Error::ObjectIdMismatch | Error::InvalidSignature { .. })
        ));
    }

    #[test]
    fn verify_warns_on_small_future_skew() {
        let keypair = Keypair::generate();
        let now = 1_700_000_000;
        let created_at = now + 60;
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
        let now = 1_700_000_000;
        let created_at = now - 60;
        let body = b"hello mesh".to_vec();
        let message = Message::create(&keypair, created_at, "test".to_owned(), &body)
            .expect("create message");

        let clock = FixedClock { now };
        assert_eq!(message.verify(&clock), Verification::Accepted);
    }

    #[test]
    fn verify_rejects_large_future_skew() {
        let keypair = Keypair::generate();
        let now = 1_700_000_000;
        let created_at = now + CLOCK_SKEW_REJECT_SECONDS + 1;
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
        let now = 1_700_000_000;
        let created_at = now - CLOCK_SKEW_REJECT_SECONDS - 1;
        let body = b"hello mesh".to_vec();
        let message = Message::create(&keypair, created_at, "test".to_owned(), &body)
            .expect("create message");

        let clock = FixedClock { now };
        assert_eq!(message.verify(&clock), Verification::Accepted);
    }

    #[test]
    fn transport_metadata_does_not_affect_signature() {
        let keypair = Keypair::generate();
        let now = 1_700_000_000;
        let body = b"hello mesh".to_vec();
        let mut message =
            Message::create(&keypair, now, "test".to_owned(), &body).expect("create message");
        message.transport_metadata.hop_count = 42;

        let clock = FixedClock { now };
        assert_eq!(message.verify(&clock), Verification::Accepted);
    }
}
