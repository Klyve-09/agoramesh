//! SQLite-backed storage primitives.

use std::path::Path;

use rusqlite::{Connection as SqliteConnection, OpenFlags};

/// A handle to the underlying `SQLite` connection.
#[derive(Debug)]
pub struct Connection {
    inner: SqliteConnection,
}

/// The high-level Agoramesh store.
#[derive(Debug)]
pub struct Store {
    connection: Connection,
}

impl Connection {
    /// Opens an in-memory store for tests and ephemeral use.
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

    /// Opens a store at the given filesystem path.
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

impl Store {
    /// Creates a store backed by an existing connection.
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

/// Errors that can occur when opening or using the store.
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
            let _store = Store::new(connection);
        }
        {
            let connection = Connection::open(&path).expect("reopen existing store");
            let _store = Store::new(connection);
        }
    }

    #[test]
    fn store_owns_connection() {
        let connection = Connection::open_in_memory().expect("open in-memory store");
        let store = Store::new(connection);
        let _ = store.connection();
    }
}
