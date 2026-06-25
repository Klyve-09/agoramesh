//! Verified message storage for Agoramesh.
//!
//! This module defines the `Store` trait and an in-memory implementation.
//! Every concrete store must verify a message with a `Clock` before accepting
//! it, ensuring only valid, well-formed messages can be persisted or served.

use std::fmt;

use agoramesh_core::{Clock, Message, MessageId, Verification};
#[cfg(test)]
use chrono::{DateTime, Utc};

/// Whether a store insert wrote a new message or found an identical one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InsertResult {
    /// The message was newly inserted.
    Inserted,
    /// An identical message was already present under the same object ID.
    Duplicate,
}

/// The public contract implemented by every Agoramesh message store.
///
/// All operations are synchronous in Phase 1; async variants may be added later.
///
/// Read operations verify every retrieved message against the supplied clock.
/// A stored message that no longer verifies (because it was corrupted, tampered
/// with, or the schema changed) is surfaced as an error rather than silently
/// dropped.
pub trait Store: fmt::Debug {
    /// Inserts a message into the store after verifying it.
    ///
    /// # Errors
    ///
    /// Returns an error if the message fails verification or if a different
    /// message is already present under the same object ID.
    fn insert(&mut self, message: Message, clock: &dyn Clock) -> Result<InsertResult, Error>;

    /// Returns the message with the given object ID, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend fails, if deserialization fails, or if
    /// the stored message fails verification.
    fn get(&self, id: MessageId, clock: &dyn Clock) -> Result<Option<Message>, Error>;

    /// Returns all messages in the given scope, oldest first.
    ///
    /// Sort order is `(created_at, object_id)` ascending; `object_id` breaks
    /// ties deterministically.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend fails, deserialization fails, or any
    /// stored message fails verification.
    fn list_by_scope(&self, scope: &str, clock: &dyn Clock) -> Result<Vec<Message>, Error>;

    /// Returns all messages of the given signed payload type, oldest first.
    ///
    /// Sort order is `(created_at, object_id)` ascending; `object_id` breaks
    /// ties deterministically.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend fails, deserialization fails, or any
    /// stored message fails verification.
    fn list_by_type(&self, kind: &str, clock: &dyn Clock) -> Result<Vec<Message>, Error>;

    /// Returns all messages sorted by `(created_at, object_id)` ascending.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend fails, deserialization fails, or any
    /// stored message fails verification.
    fn list_by_created_at(&self, clock: &dyn Clock) -> Result<Vec<Message>, Error>;
}

/// An in-memory message store backed by a map keyed by object ID.
#[derive(Debug, Default)]
pub struct InMemoryStore {
    messages: std::collections::BTreeMap<MessageId, Message>,
}

impl InMemoryStore {
    /// Creates a new empty in-memory store.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            messages: std::collections::BTreeMap::new(),
        }
    }
}

impl Store for InMemoryStore {
    fn insert(&mut self, message: Message, _clock: &dyn Clock) -> Result<InsertResult, Error> {
        match message.verify() {
            Verification::Accepted | Verification::AcceptedWithWarning(_) => {
                if let Some(existing) = self.messages.get(&message.id()) {
                    if existing == &message {
                        return Ok(InsertResult::Duplicate);
                    }
                    return Err(Error::DuplicateObjectId(message.id()));
                }
                self.messages.insert(message.id(), message);
                Ok(InsertResult::Inserted)
            }
            Verification::Rejected(error) => Err(Error::Rejected(error)),
        }
    }

    fn get(&self, id: MessageId, clock: &dyn Clock) -> Result<Option<Message>, Error> {
        match self.messages.get(&id) {
            Some(message) => {
                verify_loaded(message, clock)?;
                Ok(Some(message.clone()))
            }
            None => Ok(None),
        }
    }

    fn list_by_scope(&self, scope: &str, clock: &dyn Clock) -> Result<Vec<Message>, Error> {
        let mut result: Vec<_> = self
            .messages
            .values()
            .filter(|message| message.signed_payload().scope() == scope)
            .cloned()
            .collect();
        sort_and_verify(&mut result, clock)
    }

    fn list_by_type(&self, kind: &str, clock: &dyn Clock) -> Result<Vec<Message>, Error> {
        let mut result: Vec<_> = self
            .messages
            .values()
            .filter(|message| message.signed_payload().kind() == kind)
            .cloned()
            .collect();
        sort_and_verify(&mut result, clock)
    }

    fn list_by_created_at(&self, clock: &dyn Clock) -> Result<Vec<Message>, Error> {
        let mut result: Vec<_> = self.messages.values().cloned().collect();
        sort_and_verify(&mut result, clock)
    }
}

fn sort_and_verify(messages: &mut Vec<Message>, clock: &dyn Clock) -> Result<Vec<Message>, Error> {
    sort_by_created_at(messages);
    for message in &mut *messages {
        verify_loaded(message, clock)?;
    }
    Ok(messages.clone())
}

fn verify_loaded(message: &Message, _clock: &dyn Clock) -> Result<(), Error> {
    match message.verify() {
        Verification::Accepted | Verification::AcceptedWithWarning(_) => Ok(()),
        Verification::Rejected(error) => Err(Error::RejectedOnRead(error)),
    }
}

fn sort_by_created_at(messages: &mut [Message]) {
    messages.sort_by(|a, b| {
        let a_payload = a.signed_payload();
        let b_payload = b.signed_payload();
        a_payload
            .created_at()
            .datetime()
            .cmp(&b_payload.created_at().datetime())
            .then_with(|| a.id().cmp(&b.id()))
    });
}

/// Errors that can occur when storing or retrieving messages.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum Error {
    /// The message failed verification and was not stored.
    #[error("message rejected on insert: {0}")]
    Rejected(agoramesh_core::message::Error),

    /// A stored message failed verification when read back.
    #[error("stored message failed verification on read: {0}")]
    RejectedOnRead(agoramesh_core::message::Error),

    /// A message with the same object ID already exists.
    #[error("duplicate object id: {0:?}")]
    DuplicateObjectId(MessageId),

    /// Failed to serialize or deserialize a stored message.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// The underlying storage backend returned an error.
    #[error("storage backend error: {0}")]
    Backend(String),

    /// A stored message's indexed metadata does not match its JSON payload.
    #[error("stored message metadata mismatch: {field}")]
    CorruptStoredMessage {
        /// The metadata field that diverged from the JSON payload.
        field: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use agoramesh_core::identity::Keypair;
    use agoramesh_core::message::Error as MessageError;
    use chrono::TimeDelta;
    use serde_json::Value;

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

    fn valid_message(scope: &str, created_at: DateTime<Utc>) -> Message {
        let keypair = Keypair::generate();
        Message::create(
            &keypair,
            "message",
            created_at,
            scope.to_owned(),
            b"hello mesh",
        )
        .expect("create message")
    }

    fn tamper_body(message: &Message) -> Message {
        let mut value: Value = serde_json::to_value(message).expect("serialize");
        let body = value
            .get_mut("signed_payload")
            .and_then(|payload| payload.get_mut("body"))
            .expect("body field");
        *body = Value::String("ZXZpbA".to_owned());
        serde_json::from_value(value).expect("deserialize tampered")
    }

    fn tamper_id(message: &Message) -> Message {
        let mut value: Value = serde_json::to_value(message).expect("serialize");
        value.as_object_mut().expect("object").insert(
            "id".to_owned(),
            Value::Array(vec![Value::Number(0.into()); 32]),
        );
        serde_json::from_value(value).expect("deserialize tampered")
    }

    fn corrupt_body(message: &Message) -> Message {
        let mut value: Value = serde_json::to_value(message).expect("serialize");
        let body = value
            .get_mut("signed_payload")
            .and_then(|payload| payload.get_mut("body"))
            .expect("body field");
        *body = Value::String("Y29ycnVwdA".to_owned());
        serde_json::from_value(value).expect("deserialize tampered")
    }

    #[test]
    fn valid_message_is_stored() {
        let mut store = InMemoryStore::new();
        let message = valid_message("test", utc(1_700_000_000));
        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
        assert!(store.insert(message.clone(), &clock).is_ok());
        assert_eq!(
            store
                .get(message.id(), &clock)
                .expect("get")
                .map(|message| message.id()),
            Some(message.id())
        );
    }

    #[test]
    fn invalid_signature_is_rejected() {
        let mut store = InMemoryStore::new();
        let message = valid_message("test", utc(1_700_000_000));
        let tampered = tamper_body(&message);

        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
        assert!(matches!(
            store.insert(tampered, &clock),
            Err(Error::Rejected(
                MessageError::ObjectIdMismatch | MessageError::InvalidSignature { .. }
            ))
        ));
    }

    #[test]
    fn object_id_mismatch_is_rejected() {
        let mut store = InMemoryStore::new();
        let message = valid_message("test", utc(1_700_000_000));
        let tampered = tamper_id(&message);

        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
        assert!(matches!(
            store.insert(tampered, &clock),
            Err(Error::Rejected(MessageError::ObjectIdMismatch))
        ));
    }

    #[test]
    fn future_message_is_stored_and_skew_detected_by_sync() {
        let mut store = InMemoryStore::new();
        let keypair = Keypair::generate();
        let now = utc(1_700_000_000);
        let created_at =
            now + TimeDelta::seconds(agoramesh_core::message::CLOCK_SKEW_REJECT_SECONDS + 1);
        let message = Message::create(
            &keypair,
            "message",
            created_at,
            "test".to_owned(),
            b"hello mesh",
        )
        .expect("create message");

        let clock = FixedClock { now };
        assert_eq!(
            store.insert(message.clone(), &clock).expect("insert"),
            InsertResult::Inserted
        );
        assert!(matches!(
            message.classify_clock_skew(&clock),
            Verification::Rejected(MessageError::ClockSkewTooLarge { .. })
        ));
        assert_eq!(store.list_by_created_at(&clock).expect("list").len(), 1);
    }

    #[test]
    fn duplicate_object_id_returns_duplicate_result() {
        let mut store = InMemoryStore::new();
        let message = valid_message("test", utc(1_700_000_000));
        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
        assert_eq!(
            store.insert(message.clone(), &clock).expect("first insert"),
            InsertResult::Inserted
        );
        assert_eq!(
            store.insert(message, &clock).expect("second insert"),
            InsertResult::Duplicate
        );
    }

    #[test]
    fn conflicting_object_id_is_rejected() {
        let mut store = InMemoryStore::new();
        let first = valid_message("test", utc(1_700_000_000));
        let second = tamper_body(&first);
        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
        store.insert(first, &clock).expect("first insert");
        assert!(matches!(
            store.insert(second, &clock),
            Err(Error::Rejected(
                MessageError::ObjectIdMismatch | MessageError::InvalidSignature { .. }
            ))
        ));
    }

    #[test]
    fn list_by_scope_returns_only_matching_messages() {
        let mut store = InMemoryStore::new();
        let clock = FixedClock {
            now: utc(1_700_000_010),
        };
        let alpha = valid_message("alpha", utc(1_700_000_000));
        let beta = valid_message("beta", utc(1_700_000_001));
        store.insert(alpha.clone(), &clock).expect("insert alpha");
        store.insert(beta, &clock).expect("insert beta");

        let alpha_list = store.list_by_scope("alpha", &clock).expect("list alpha");
        assert_eq!(alpha_list.len(), 1);
        assert_eq!(alpha_list.first().map(Message::id), Some(alpha.id()));
    }

    #[test]
    fn list_by_type_filters_by_payload_kind() {
        let mut store = InMemoryStore::new();
        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
        let message = valid_message("test", utc(1_700_000_000));
        store.insert(message, &clock).expect("insert");

        assert_eq!(
            store.list_by_type("message", &clock).expect("list").len(),
            1
        );
        assert!(
            store
                .list_by_type("unknown", &clock)
                .expect("list")
                .is_empty()
        );
    }

    #[test]
    fn list_by_created_at_sorts_ascending() {
        let mut store = InMemoryStore::new();
        let clock = FixedClock {
            now: utc(1_700_000_010),
        };
        let second = valid_message("test", utc(1_700_000_001));
        let first = valid_message("test", utc(1_700_000_000));
        let third = valid_message("test", utc(1_700_000_002));
        store.insert(second.clone(), &clock).expect("insert second");
        store.insert(first.clone(), &clock).expect("insert first");
        store.insert(third.clone(), &clock).expect("insert third");

        let list = store.list_by_created_at(&clock).expect("list");
        assert_eq!(list.first().map(Message::id), Some(first.id()));
        assert_eq!(list.get(1).map(Message::id), Some(second.id()));
        assert_eq!(list.get(2).map(Message::id), Some(third.id()));
    }

    #[test]
    fn list_uses_object_id_tie_breaker() {
        let mut store = InMemoryStore::new();
        let clock = FixedClock {
            now: utc(1_700_000_010),
        };
        let mut messages: Vec<Message> = (0..3)
            .map(|_| valid_message("test", utc(1_700_000_000)))
            .collect();
        messages.sort_by_key(agoramesh_core::Message::id);
        for message in &messages {
            store.insert(message.clone(), &clock).expect("insert");
        }

        let list = store.list_by_created_at(&clock).expect("list");
        assert_eq!(list.len(), 3);
        assert!(
            list.windows(2)
                .all(|window| window.first().map(Message::id) < window.get(1).map(Message::id))
        );
    }

    #[test]
    fn read_rejects_corrupted_message() {
        let mut store = InMemoryStore::new();
        let message = valid_message("test", utc(1_700_000_000));
        let corrupted = corrupt_body(&message);
        store
            .insert(message.clone(), &message_clock(&message))
            .expect("insert");
        store.messages.insert(message.id(), corrupted);

        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
        assert!(matches!(
            store.get(message.id(), &clock),
            Err(Error::RejectedOnRead(_))
        ));
    }

    #[test]
    fn list_rejects_corrupted_message() {
        let mut store = InMemoryStore::new();
        let message = valid_message("test", utc(1_700_000_000));
        let corrupted = corrupt_body(&message);
        store
            .insert(message.clone(), &message_clock(&message))
            .expect("insert");
        store.messages.insert(message.id(), corrupted);

        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
        assert!(matches!(
            store.list_by_created_at(&clock),
            Err(Error::RejectedOnRead(_))
        ));
    }

    #[test]
    fn serde_roundtrip_preserves_verification() {
        let mut store = InMemoryStore::new();
        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
        let message = valid_message("test", utc(1_700_000_000));
        store.insert(message.clone(), &clock).expect("insert");

        let stored = store
            .get(message.id(), &clock)
            .expect("get")
            .expect("present");
        let bytes = serde_json::to_vec(&stored).expect("serialize");
        let restored: Message = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(restored.verify(), Verification::Accepted);

        assert_eq!(
            store
                .insert(restored, &clock)
                .expect("duplicate after roundtrip"),
            InsertResult::Duplicate
        );
    }

    fn message_clock(message: &Message) -> FixedClock {
        FixedClock {
            now: message.signed_payload().created_at().datetime(),
        }
    }
}
