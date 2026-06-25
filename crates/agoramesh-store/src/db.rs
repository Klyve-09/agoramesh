//! SQLite-backed storage primitives.
//!
//! This module also provides `SqliteStore`, an implementation of the
//! `Store` trait that persists verified messages as JSON blobs.

use std::path::Path;

use agoramesh_core::{Clock, Message, MessageId, Verification};
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

/// A high-level handle that does not yet implement the message `Store` trait.
///
/// Use `SqliteStore::new` for a store implementation.
#[derive(Debug)]
pub struct ConnectionHandle {
    connection: Connection,
}

impl ConnectionHandle {
    /// Creates a handle backed by an existing connection.
    #[must_use]
    pub const fn new(connection: Connection) -> Self {
        Self { connection }
    }

    /// Returns a reference to the underlying connection.
    #[must_use]
    pub const fn connection(&self) -> &Connection {
        &self.connection
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
                self.connection
                    .inner
                    .execute(
                        "INSERT INTO messages (id, kind, scope, created_at, json)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        rusqlite::params![
                            id,
                            signed.kind(),
                            signed.scope(),
                            signed.created_at(),
                            json
                        ],
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

    fn get(&self, id: MessageId) -> Option<Message> {
        let id_bytes = id.as_bytes();
        let id_slice = id_bytes.as_slice();
        self.connection
            .inner
            .query_row(
                "SELECT json FROM messages WHERE id = ?1",
                [id_slice],
                Self::message_from_row,
            )
            .optional()
            .map_err(|error| StoreError::Backend(error.to_string()))
            .ok()
            .flatten()
    }

    fn list_by_scope(&self, scope: &str) -> Vec<Message> {
        self.list(
            "SELECT json FROM messages WHERE scope = ?1 ORDER BY created_at ASC",
            [scope],
        )
    }

    fn list_by_type(&self, kind: &str) -> Vec<Message> {
        self.list(
            "SELECT json FROM messages WHERE kind = ?1 ORDER BY created_at ASC",
            [kind],
        )
    }

    fn list_by_created_at(&self) -> Vec<Message> {
        self.list("SELECT json FROM messages ORDER BY created_at ASC", [])
    }
}

impl SqliteStore {
    fn list<P: rusqlite::Params>(&self, sql: &str, params: P) -> Vec<Message> {
        let Ok(mut statement) = self.connection.inner.prepare(sql) else {
            return Vec::new();
        };
        let Ok(rows) = statement.query_map(params, Self::message_from_row) else {
            return Vec::new();
        };
        rows.filter_map(std::result::Result::ok).collect()
    }
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
            let _store = ConnectionHandle::new(connection);
        }
        {
            let connection = Connection::open(&path).expect("reopen existing store");
            let _store = ConnectionHandle::new(connection);
        }
    }

    #[test]
    fn store_owns_connection() {
        let connection = Connection::open_in_memory().expect("open in-memory store");
        let store = ConnectionHandle::new(connection);
        let _ = store.connection();
    }

    #[test]
    fn sqlite_store_inserts_and_recoveres_valid_message() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let path = dir.path().join("store.db");
        let message = valid_message("test", 1_700_000_000);

        {
            let connection = Connection::open(&path).expect("open");
            let mut store = SqliteStore::new(connection);
            let clock = FixedClock { now: 1_700_000_000 };
            store.insert(message.clone(), &clock).expect("insert");
        }

        {
            let connection = Connection::open(&path).expect("reopen");
            let store = SqliteStore::new(connection);
            let recovered = store.get(message.id()).expect("recover message");
            assert_eq!(recovered.id(), message.id());
            let clock = FixedClock { now: 1_700_000_000 };
            assert_eq!(recovered.verify(&clock), Verification::Accepted);
        }
    }

    #[test]
    fn sqlite_store_rejects_invalid_signature() {
        let connection = Connection::open_in_memory().expect("open");
        let mut store = SqliteStore::new(connection);
        let message = valid_message("test", 1_700_000_000);
        let mut value: serde_json::Value = serde_json::to_value(&message).expect("serialize");
        let body = value
            .get_mut("signed_payload")
            .and_then(|payload| payload.get_mut("body"))
            .expect("body field");
        *body = serde_json::json!([0]);
        let tampered: Message = serde_json::from_value(value).expect("deserialize tampered");

        let clock = FixedClock { now: 1_700_000_000 };
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
        let message = valid_message("test", 1_700_000_000);
        let clock = FixedClock { now: 1_700_000_000 };
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
        let clock = FixedClock { now: 1_700_000_010 };
        let alpha = valid_message("alpha", 1_700_000_000);
        let beta = valid_message("beta", 1_700_000_001);
        store.insert(alpha.clone(), &clock).expect("insert alpha");
        store.insert(beta.clone(), &clock).expect("insert beta");

        let alpha_list = store.list_by_scope("alpha");
        assert_eq!(alpha_list.len(), 1);
        assert_eq!(alpha_list.first().map(Message::id), Some(alpha.id()));

        let all = store.list_by_created_at();
        assert_eq!(all.len(), 2);
        assert_eq!(all.first().map(Message::id), Some(alpha.id()));
        assert_eq!(all.get(1).map(Message::id), Some(beta.id()));
    }

    #[test]
    fn sqlite_store_lists_by_type() {
        let connection = Connection::open_in_memory().expect("open");
        let mut store = SqliteStore::new(connection);
        let message = valid_message("test", 1_700_000_000);
        let clock = FixedClock { now: 1_700_000_000 };
        store.insert(message, &clock).expect("insert");

        assert_eq!(store.list_by_type("message").len(), 1);
        assert!(store.list_by_type("unknown").is_empty());
    }
}
