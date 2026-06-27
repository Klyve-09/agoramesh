//! Phase 1 acceptance pipeline.

use crate::message::{self, Clock};
use crate::objects::validation;
use crate::{Message, Verification};

/// Protocol and schema integrity policy for acceptance.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProtocolSchemaPolicy;

/// Phase 1 semantic object policy for acceptance.
#[derive(Clone, Copy, Debug, Default)]
pub struct Phase1SemanticObjectPolicy;

/// View of key revocations available to the acceptance pipeline.
pub trait RevocationView: std::fmt::Debug + Send + Sync {
    /// Returns whether the message author is revoked for the message timestamp.
    fn rejects(&self, _message: &Message) -> bool {
        false
    }
}

/// No-op revocation view used until Phase 3 revocation state is available.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopRevocationView;

impl RevocationView for NoopRevocationView {}

/// Inputs required to decide whether a message is acceptable for Phase 1 use.
#[derive(Debug)]
pub struct AcceptanceContext<'a> {
    /// Clock used to enforce timestamp skew policy.
    pub clock: &'a dyn Clock,
    /// Protocol/schema integrity policy.
    pub protocol_schema: ProtocolSchemaPolicy,
    /// Phase 1 semantic object policy.
    pub phase1_semantics: Phase1SemanticObjectPolicy,
    /// Revocation state view; currently no-op in Phase 2.
    pub revocation_view: &'a dyn RevocationView,
}

impl<'a> AcceptanceContext<'a> {
    /// Builds a Phase 1 acceptance context with the no-op revocation view.
    #[must_use]
    pub fn phase1(clock: &'a dyn Clock) -> Self {
        static NOOP_REVOCATION_VIEW: NoopRevocationView = NoopRevocationView;
        Self {
            clock,
            protocol_schema: ProtocolSchemaPolicy,
            phase1_semantics: Phase1SemanticObjectPolicy,
            revocation_view: &NOOP_REVOCATION_VIEW,
        }
    }
}

/// Message proven acceptable by the Phase 1 pipeline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedPhase1Message(Message);

impl AcceptedPhase1Message {
    /// Returns the accepted message.
    #[must_use]
    pub const fn message(&self) -> &Message {
        &self.0
    }

    /// Consumes the wrapper and returns the accepted message.
    #[must_use]
    pub fn into_message(self) -> Message {
        self.0
    }
}

/// Stateless Phase 1 acceptance entry point.
#[derive(Clone, Copy, Debug, Default)]
pub struct Phase1Acceptance;

impl Phase1Acceptance {
    /// Accepts a message after integrity, clock, revocation, and semantic checks.
    ///
    /// # Errors
    /// Returns the first failing acceptance stage in fixed order.
    pub fn accept(
        message: Message,
        context: &AcceptanceContext<'_>,
    ) -> Result<AcceptedPhase1Message, Error> {
        validate_phase1_for_acceptance(&message, context)?;
        Ok(AcceptedPhase1Message(message))
    }
}

/// Validates a message for Phase 1 acceptance without consuming it.
///
/// # Errors
/// Returns the first failing acceptance stage in fixed order.
pub fn validate_phase1_for_acceptance(
    message: &Message,
    context: &AcceptanceContext<'_>,
) -> Result<(), Error> {
    verify_integrity(message)?;
    verify_clock(message, context.clock)?;
    if context.revocation_view.rejects(message) {
        return Err(Error::RevokedAuthor);
    }
    validation::validate_phase1_message(message).map_err(Error::Semantic)
}

/// Verifies signature, object ID, and author consistency.
///
/// # Errors
/// Returns an integrity error when the common message envelope does not verify.
pub fn verify_integrity(message: &Message) -> Result<(), Error> {
    match message.verify() {
        Verification::Accepted | Verification::AcceptedWithWarning(_) => Ok(()),
        Verification::Rejected(error) => Err(Error::Integrity(error)),
    }
}

fn verify_clock(message: &Message, clock: &dyn Clock) -> Result<(), Error> {
    match message.classify_clock_skew(clock) {
        Verification::Accepted | Verification::AcceptedWithWarning(_) => Ok(()),
        Verification::Rejected(error) => Err(Error::Clock(error)),
    }
}

/// Errors returned by Phase 1 acceptance.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum Error {
    /// Signature, object-id, or author-pubkey integrity failed.
    #[error("message integrity failed: {0}")]
    Integrity(message::Error),
    /// Clock skew policy rejected the message.
    #[error("message clock policy failed: {0}")]
    Clock(message::Error),
    /// Revocation policy rejected the author for the message timestamp.
    #[error("message author key is revoked")]
    RevokedAuthor,
    /// Phase 1 semantic object validation failed.
    #[error("object validation failed: {0}")]
    Semantic(validation::Error),
}
