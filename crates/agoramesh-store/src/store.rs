//! Verified message storage for Agoramesh.
//!
//! This module defines the `Store` trait and an in-memory implementation.
//! Every concrete store must verify a message with a `Clock` before accepting
//! it, ensuring only valid, well-formed messages can be persisted or served.

use std::fmt;

use agoramesh_core::{Clock, Message, MessageId, Verification};

/// The public contract implemented by every Agoramesh message store.
///
/// All operations are synchronous in Phase 1; async variants may be added later.
pub trait Store: fmt::Debug {
    /// Inserts a message into the store after verifying it.
    ///
    /// # Errors
    ///
    /// Returns an error if the message fails verification or if it is already
    /// present under the same `object_id`.
    fn insert(&mut self, message: Message, clock: &dyn Clock) -> Result<(), Error>;

    /// Returns the message with the given object ID, if any.
    fn get(&self, id: MessageId) -> Option<Message>;

    /// Returns all messages in the given scope, oldest first by `created_at`.
    fn list_by_scope(&self, scope: &str) -> Vec<Message>;

    /// Returns all messages of the given signed payload type, oldest first by
    /// `created_at`.
    fn list_by_type(&self, kind: &str) -> Vec<Message>;

    /// Returns all messages sorted by `created_at` ascending.
    fn list_by_created_at(&self) -> Vec<Message>;
}

/// An in-memory message store backed by a map keyed by `object_id`.
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
    fn insert(&mut self, message: Message, clock: &dyn Clock) -> Result<(), Error> {
        match message.verify(clock) {
            Verification::Accepted | Verification::AcceptedWithWarning(_) => {
                if self.messages.contains_key(&message.id()) {
                    return Err(Error::DuplicateObjectId(message.id()));
                }
                self.messages.insert(message.id(), message);
                Ok(())
            }
            Verification::Rejected(error) => Err(Error::Rejected(error)),
        }
    }

    fn get(&self, id: MessageId) -> Option<Message> {
        self.messages.get(&id).cloned()
    }

    fn list_by_scope(&self, scope: &str) -> Vec<Message> {
        let mut result: Vec<_> = self
            .messages
            .values()
            .filter(|message| message.signed_payload().scope() == scope)
            .cloned()
            .collect();
        sort_by_created_at(&mut result);
        result
    }

    fn list_by_type(&self, kind: &str) -> Vec<Message> {
        let mut result: Vec<_> = self
            .messages
            .values()
            .filter(|message| message.signed_payload().kind() == kind)
            .cloned()
            .collect();
        sort_by_created_at(&mut result);
        result
    }

    fn list_by_created_at(&self) -> Vec<Message> {
        let mut result: Vec<_> = self.messages.values().cloned().collect();
        sort_by_created_at(&mut result);
        result
    }
}

fn sort_by_created_at(messages: &mut [Message]) {
    messages.sort_by_key(|message| message.signed_payload().created_at());
}

/// Errors that can occur when storing or retrieving messages.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum Error {
    /// The message failed verification and was not stored.
    #[error("message rejected: {0}")]
    Rejected(agoramesh_core::message::Error),

    /// A message with the same object ID already exists.
    #[error("duplicate object id: {0:?}")]
    DuplicateObjectId(MessageId),

    /// Failed to serialize or deserialize a stored message.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// The underlying storage backend returned an error.
    #[error("storage backend error: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use agoramesh_core::identity::Keypair;
    use agoramesh_core::message::Error as MessageError;
    use serde_json::Value;

    #[derive(Debug, Default)]
    struct FixedClock {
        now: i64,
    }

    impl Clock for FixedClock {
        fn now(&self) -> i64 {
            self.now
        }
    }

    fn valid_message(scope: &str, created_at: i64) -> Message {
        let keypair = Keypair::generate();
        Message::create(&keypair, created_at, scope.to_owned(), b"hello mesh")
            .expect("create message")
    }

    fn tamper_body(message: &Message) -> Message {
        let mut value: Value = serde_json::to_value(message).expect("serialize");
        let body = value
            .get_mut("signed_payload")
            .and_then(|payload| payload.get_mut("body"))
            .expect("body field");
        *body = Value::Array(vec![Value::Number(0.into())]);
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

    #[test]
    fn valid_message_is_stored() {
        let mut store = InMemoryStore::new();
        let message = valid_message("test", 1_700_000_000);
        let clock = FixedClock { now: 1_700_000_000 };
        assert!(store.insert(message.clone(), &clock).is_ok());
        assert_eq!(
            store.get(message.id()).map(|message| message.id()),
            Some(message.id())
        );
    }

    #[test]
    fn invalid_signature_is_rejected() {
        let mut store = InMemoryStore::new();
        let message = valid_message("test", 1_700_000_000);
        let tampered = tamper_body(&message);

        let clock = FixedClock { now: 1_700_000_000 };
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
        let message = valid_message("test", 1_700_000_000);
        let tampered = tamper_id(&message);

        let clock = FixedClock { now: 1_700_000_000 };
        assert!(matches!(
            store.insert(tampered, &clock),
            Err(Error::Rejected(MessageError::ObjectIdMismatch))
        ));
    }

    #[test]
    fn future_rejected_message_is_not_stored() {
        let mut store = InMemoryStore::new();
        let keypair = Keypair::generate();
        let now = 1_700_000_000;
        let created_at = now + agoramesh_core::message::CLOCK_SKEW_REJECT_SECONDS + 1;
        let message = Message::create(&keypair, created_at, "test".to_owned(), b"hello mesh")
            .expect("create message");

        let clock = FixedClock { now };
        assert!(matches!(
            store.insert(message, &clock),
            Err(Error::Rejected(MessageError::ClockSkewTooLarge { .. }))
        ));
        assert_eq!(store.list_by_created_at().len(), 0);
    }

    #[test]
    fn duplicate_object_id_is_rejected() {
        let mut store = InMemoryStore::new();
        let message = valid_message("test", 1_700_000_000);
        let clock = FixedClock { now: 1_700_000_000 };
        store.insert(message.clone(), &clock).expect("first insert");
        assert!(matches!(
            store.insert(message, &clock),
            Err(Error::DuplicateObjectId(_))
        ));
    }

    #[test]
    fn list_by_scope_returns_only_matching_messages() {
        let mut store = InMemoryStore::new();
        let clock = FixedClock { now: 1_700_000_000 };
        let alpha = valid_message("alpha", 1_700_000_000);
        let beta = valid_message("beta", 1_700_000_001);
        store.insert(alpha.clone(), &clock).expect("insert alpha");
        store.insert(beta, &clock).expect("insert beta");

        let alpha_list = store.list_by_scope("alpha");
        assert_eq!(alpha_list.len(), 1);
        assert_eq!(alpha_list.first().map(Message::id), Some(alpha.id()));
    }

    #[test]
    fn list_by_type_filters_by_payload_kind() {
        let mut store = InMemoryStore::new();
        let clock = FixedClock { now: 1_700_000_000 };
        let message = valid_message("test", 1_700_000_000);
        store.insert(message, &clock).expect("insert");

        assert_eq!(store.list_by_type("message").len(), 1);
        assert!(store.list_by_type("unknown").is_empty());
    }

    #[test]
    fn list_by_created_at_sorts_ascending() {
        let mut store = InMemoryStore::new();
        let clock = FixedClock { now: 1_700_000_010 };
        let second = valid_message("test", 1_700_000_001);
        let first = valid_message("test", 1_700_000_000);
        let third = valid_message("test", 1_700_000_002);
        store.insert(second.clone(), &clock).expect("insert second");
        store.insert(first.clone(), &clock).expect("insert first");
        store.insert(third.clone(), &clock).expect("insert third");

        let list = store.list_by_created_at();
        assert_eq!(list.first().map(Message::id), Some(first.id()));
        assert_eq!(list.get(1).map(Message::id), Some(second.id()));
        assert_eq!(list.get(2).map(Message::id), Some(third.id()));
    }

    #[test]
    fn serde_roundtrip_preserves_verification() {
        let mut store = InMemoryStore::new();
        let clock = FixedClock { now: 1_700_000_000 };
        let message = valid_message("test", 1_700_000_000);
        store.insert(message.clone(), &clock).expect("insert");

        let stored = store.get(message.id()).expect("get");
        let bytes = serde_json::to_vec(&stored).expect("serialize");
        let restored: Message = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(restored.verify(&clock), Verification::Accepted);

        store
            .insert(restored, &clock)
            .expect_err("duplicate after roundtrip");
    }
}
