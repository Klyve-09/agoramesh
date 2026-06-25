-- Initial schema for the Agoramesh store.
-- Stored as a single batch migration; later iterations should move to a
-- proper migration crate once the schema stabilizes.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS identities (
    id BLOB PRIMARY KEY NOT NULL,
    verifying_key BLOB NOT NULL,
    first_seen_at INTEGER NOT NULL DEFAULT (unixepoch())
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS messages (
    id BLOB PRIMARY KEY NOT NULL,
    author_id BLOB NOT NULL,
    payload BLOB NOT NULL,
    received_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (author_id) REFERENCES identities(id) ON DELETE CASCADE
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS idx_messages_author ON messages(author_id);
CREATE INDEX IF NOT EXISTS idx_messages_received_at ON messages(received_at);
