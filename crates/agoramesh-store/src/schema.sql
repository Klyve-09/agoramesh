-- Single migration for the Phase 1 Agoramesh store.
-- Stores verified messages as JSON blobs keyed by object_id.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS messages (
    id BLOB PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    scope TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    json BLOB NOT NULL
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS idx_messages_scope ON messages(scope);
CREATE INDEX IF NOT EXISTS idx_messages_kind ON messages(kind);
CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at);
