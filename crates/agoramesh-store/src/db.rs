//! SQLite-backed storage primitives.
//!
//! This module also provides `SqliteStore`, an implementation of the
//! `Store` trait that persists verified messages as JSON blobs.

use std::path::Path;

use agoramesh_core::{Clock, Message, MessageId, Verification};
#[cfg(test)]
use chrono::{DateTime, Utc};
use rusqlite::{Connection as SqliteConnection, OpenFlags, OptionalExtension};

use crate::store::{Error as StoreError, Store};

/// A handle to the underlying `SQLite` connection.
#[derive(Debug)]
pub struct Connection {
    inner: SqliteConnection,
}

impl Connection {
    /// Opens an in-memory connection for tests and ephemeral use.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot initialize the in-memory database.
    pub fn open_in_memory() -> Result<Self, Error> {
        let inner = SqliteConnection::open_in_memory().map_err(Error::Open)?;
        let connection = Self { inner };
        connection.migrate()?;
        Ok(connection)
    }

    /// Opens a connection at the given filesystem path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database file cannot be opened or initialized.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let inner = SqliteConnection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE,
        )
        .map_err(Error::Open)?;
        let connection = Self { inner };
        connection.migrate()?;
        Ok(connection)
    }

    fn migrate(&self) -> Result<(), Error> {
        self.inner
            .execute_batch(include_str!("schema.sql"))
            .map_err(Error::Migrate)
    }

    /// Returns a reference to the underlying `SQLite` connection.
    #[must_use]
    pub const fn as_inner(&self) -> &SqliteConnection {
        &self.inner
    }
}

/// A SQLite-backed store that only persists verified messages.
#[derive(Debug)]
pub struct SqliteStore {
    connection: Connection,
}

impl SqliteStore {
    /// Creates a new SQLite-backed store from an open connection.
    #[must_use]
    pub const fn new(connection: Connection) -> Self {
        Self { connection }
    }

    /// Returns a reference to the underlying connection.
    #[must_use]
    pub const fn connection(&self) -> &Connection {
        &self.connection
    }

    fn message_from_row(row: &rusqlite::Row<'_>) -> Result<Message, rusqlite::Error> {
        let json: Vec<u8> = row.get("json")?;
        let message: Message = serde_json::from_slice(&json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                json.len(),
                rusqlite::types::Type::Blob,
                Box::new(error),
            )
        })?;
        Ok(message)
    }

    fn verify_on_read(message: &Message, clock: &dyn Clock) -> Result<(), StoreError> {
        match message.verify(clock) {
            Verification::Accepted | Verification::AcceptedWithWarning(_) => Ok(()),
            Verification::Rejected(error) => Err(StoreError::RejectedOnRead(error)),
        }
    }

    fn list<P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
        clock: &dyn Clock,
    ) -> Result<Vec<Message>, StoreError> {
        let mut statement = self
            .connection
            .inner
            .prepare(sql)
            .map_err(|error| StoreError::Backend(error.to_string()))?;
        let rows = statement
            .query_map(params, Self::message_from_row)
            .map_err(|error| StoreError::Backend(error.to_string()))?;
        let mut messages = Vec::new();
        for row in rows {
            let message = row.map_err(|error| StoreError::Backend(error.to_string()))?;
            Self::verify_on_read(&message, clock)?;
            messages.push(message);
        }
        sort_by_created_at(&mut messages);
        Ok(messages)
    }
}

impl Store for SqliteStore {
    fn insert(&mut self, message: Message, clock: &dyn Clock) -> Result<(), StoreError> {
        match message.verify(clock) {
            Verification::Accepted | Verification::AcceptedWithWarning(_) => {
                let json = serde_json::to_vec(&message)
                    .map_err(|error| StoreError::Serialization(error.to_string()))?;
                let signed = message.signed_payload();
                let message_id = message.id();
                let id_bytes = message_id.as_bytes();
                let id = id_bytes.as_slice();
                let created_at = signed.created_at().datetime().to_rfc3339();
                self.connection
                    .inner
                    .execute(
                        "INSERT INTO messages (id, kind, scope, created_at, json)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        rusqlite::params![id, signed.kind(), signed.scope(), created_at, json],
                    )
                    .map_err(|error| {
                        if is_unique_violation(&error) {
                            StoreError::DuplicateObjectId(message.id())
                        } else {
                            StoreError::Backend(error.to_string())
                        }
                    })?;
                Ok(())
            }
            Verification::Rejected(error) => Err(StoreError::Rejected(error)),
        }
    }

    fn get(&self, id: MessageId, clock: &dyn Clock) -> Result<Option<Message>, StoreError> {
        let id_bytes = id.as_bytes();
        let id_slice = id_bytes.as_slice();
        let maybe_message = self
            .connection
            .inner
            .query_row(
                "SELECT json FROM messages WHERE id = ?1",
                [id_slice],
                Self::message_from_row,
            )
            .optional()
            .map_err(|error| StoreError::Backend(error.to_string()))?;
        if let Some(message) = maybe_message {
            Self::verify_on_read(&message, clock)?;
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }

    fn list_by_scope(&self, scope: &str, clock: &dyn Clock) -> Result<Vec<Message>, StoreError> {
        self.list(
            "SELECT json FROM messages WHERE scope = ?1 ORDER BY created_at ASC, id ASC",
            [scope],
            clock,
        )
    }

    fn list_by_type(&self, kind: &str, clock: &dyn Clock) -> Result<Vec<Message>, StoreError> {
        self.list(
            "SELECT json FROM messages WHERE kind = ?1 ORDER BY created_at ASC, id ASC",
            [kind],
            clock,
        )
    }

    fn list_by_created_at(&self, clock: &dyn Clock) -> Result<Vec<Message>, StoreError> {
        self.list(
            "SELECT json FROM messages ORDER BY created_at ASC, id ASC",
            [],
            clock,
        )
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

fn is_unique_violation(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(sqlite_error, _)
            if sqlite_error.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

/// Errors that can occur when opening or using the database.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to open the database.
    #[error("failed to open database: {0}")]
    Open(#[from] rusqlite::Error),

    /// Failed to run migrations.
    #[error("failed to migrate database: {0}")]
    Migrate(rusqlite::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use agoramesh_core::identity::Keypair;
    use agoramesh_core::message::Error as MessageError;
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

    fn valid_message(scope: &str, created_at: DateTime<Utc>) -> Message {
        let keypair = Keypair::generate();
        Message::create(&keypair, created_at, scope.to_owned(), b"hello mesh")
            .expect("create message")
    }

    #[test]
    fn open_in_memory_succeeds() {
        let connection = Connection::open_in_memory();
        assert!(connection.is_ok());
    }

    #[test]
    fn open_file_succeeds_and_can_reopen() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let path = dir.path().join("store.db");
        {
            let connection = Connection::open(&path).expect("open new store");
            let _store = SqliteStore::new(connection);
        }
        {
            let connection = Connection::open(&path).expect("reopen existing store");
            let _store = SqliteStore::new(connection);
        }
    }

    #[test]
    fn store_owns_connection() {
        let connection = Connection::open_in_memory().expect("open in-memory store");
        let store = SqliteStore::new(connection);
        let _ = store.connection();
    }

    #[test]
    fn sqlite_store_inserts_and_recoveres_valid_message() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let path = dir.path().join("store.db");
        let message = valid_message("test", utc(1_700_000_000));

        {
            let connection = Connection::open(&path).expect("open");
            let mut store = SqliteStore::new(connection);
            let clock = FixedClock {
                now: utc(1_700_000_000),
            };
            store.insert(message.clone(), &clock).expect("insert");
        }

        {
            let connection = Connection::open(&path).expect("reopen");
            let store = SqliteStore::new(connection);
            let recovered = store
                .get(
                    message.id(),
                    &FixedClock {
                        now: utc(1_700_000_000),
                    },
                )
                .expect("recover message");
            assert_eq!(recovered.map(|message| message.id()), Some(message.id()));
        }
    }

    #[test]
    fn sqlite_store_rejects_invalid_signature() {
        let connection = Connection::open_in_memory().expect("open");
        let mut store = SqliteStore::new(connection);
        let message = valid_message("test", utc(1_700_000_000));
        let mut value: serde_json::Value = serde_json::to_value(&message).expect("serialize");
        let body = value
            .get_mut("signed_payload")
            .and_then(|payload| payload.get_mut("body"))
            .expect("body field");
        *body = serde_json::json!("ZXZpbA");
        let tampered: Message = serde_json::from_value(value).expect("deserialize tampered");

        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
        assert!(matches!(
            store.insert(tampered, &clock),
            Err(StoreError::Rejected(
                MessageError::ObjectIdMismatch | MessageError::InvalidSignature { .. }
            ))
        ));
    }

    #[test]
    fn sqlite_store_rejects_duplicate_object_id() {
        let connection = Connection::open_in_memory().expect("open");
        let mut store = SqliteStore::new(connection);
        let message = valid_message("test", utc(1_700_000_000));
        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
        store.insert(message.clone(), &clock).expect("first insert");
        assert!(matches!(
            store.insert(message, &clock),
            Err(StoreError::DuplicateObjectId(_))
        ));
    }

    #[test]
    fn sqlite_store_lists_by_scope_and_created_at() {
        let connection = Connection::open_in_memory().expect("open");
        let mut store = SqliteStore::new(connection);
        let clock = FixedClock {
            now: utc(1_700_000_010),
        };
        let alpha = valid_message("alpha", utc(1_700_000_000));
        let beta = valid_message("beta", utc(1_700_000_001));
        store.insert(alpha.clone(), &clock).expect("insert alpha");
        store.insert(beta.clone(), &clock).expect("insert beta");

        let alpha_list = store.list_by_scope("alpha", &clock).expect("list alpha");
        assert_eq!(alpha_list.len(), 1);
        assert_eq!(alpha_list.first().map(Message::id), Some(alpha.id()));

        let all = store.list_by_created_at(&clock).expect("list all");
        assert_eq!(all.len(), 2);
        assert_eq!(all.first().map(Message::id), Some(alpha.id()));
        assert_eq!(all.get(1).map(Message::id), Some(beta.id()));
    }

    #[test]
    fn sqlite_store_lists_by_type() {
        let connection = Connection::open_in_memory().expect("open");
        let mut store = SqliteStore::new(connection);
        let message = valid_message("test", utc(1_700_000_000));
        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
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
    fn sqlite_store_list_uses_object_id_tie_breaker() {
        let connection = Connection::open_in_memory().expect("open");
        let mut store = SqliteStore::new(connection);
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
    fn sqlite_store_read_rejects_corrupted_json() {
        let connection = Connection::open_in_memory().expect("open");
        let mut store = SqliteStore::new(connection);
        let message = valid_message("test", utc(1_700_000_000));
        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
        store.insert(message.clone(), &clock).expect("insert");

        store
            .connection
            .inner
            .execute(
                "UPDATE messages SET json = ?1 WHERE id = ?2",
                rusqlite::params![b"not json", message.id().as_bytes().as_slice()],
            )
            .expect("corrupt row");

        let result = store.get(message.id(), &clock);
        assert!(matches!(
            result,
            Err(StoreError::Backend(_) | StoreError::Serialization(_))
        ));
    }

    #[test]
    fn sqlite_store_read_rejects_tampered_body() {
        let connection = Connection::open_in_memory().expect("open");
        let mut store = SqliteStore::new(connection);
        let message = valid_message("test", utc(1_700_000_000));
        let clock = FixedClock {
            now: utc(1_700_000_000),
        };
        store.insert(message.clone(), &clock).expect("insert");

        let mut value: serde_json::Value = serde_json::to_value(&message).expect("serialize");
        let body = value
            .get_mut("signed_payload")
            .and_then(|payload| payload.get_mut("body"))
            .expect("body");
        *body = serde_json::json!("Y29ycnVwdGVk");
        let tampered_json = serde_json::to_vec(&value).expect("serialize tampered");

        store
            .connection
            .inner
            .execute(
                "UPDATE messages SET json = ?1 WHERE id = ?2",
                rusqlite::params![tampered_json.as_slice(), message.id().as_bytes().as_slice()],
            )
            .expect("tamper row");

        let result = store.get(message.id(), &clock);
        assert!(matches!(
            result,
            Err(StoreError::RejectedOnRead(MessageError::ObjectIdMismatch))
        ));
    }

    #[test]
    fn sqlite_store_lists_future_message() {
        let connection = Connection::open_in_memory().expect("open");
        let mut store = SqliteStore::new(connection);
        let keypair = Keypair::generate();
        let now = utc(1_700_000_000);
        let created_at =
            now + TimeDelta::seconds(agoramesh_core::message::CLOCK_SKEW_REJECT_SECONDS + 1);
        let message = Message::create(&keypair, created_at, "test".to_owned(), b"hello mesh")
            .expect("create message");

        let write_clock = FixedClock { now };
        assert!(matches!(
            store.insert(message, &write_clock),
            Err(StoreError::Rejected(MessageError::ClockSkewTooLarge { .. }))
        ));
        assert_eq!(
            store
                .list_by_created_at(&FixedClock { now })
                .expect("list")
                .len(),
            0
        );
    }
}
