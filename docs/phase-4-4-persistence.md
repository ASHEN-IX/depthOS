# Phase 4.4 Persistence: SQLite Schema v3 and Transactional Integrity

DepthOS Phase 4.4 advances the persistent database schema from version 2 to **version 3** via a strictly forward-only migration: `migrations/003_hierarchy_management_and_preferences.sql`.

## Schema Evolution

```sql
-- Migration 003: Hierarchy Management, Concurrency, and Preferences

-- 1. Entity enhancements for optimistic concurrency and identity
ALTER TABLE worlds ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;
ALTER TABLE worlds ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE worlds ADD COLUMN visual_identity TEXT;
ALTER TABLE worlds ADD COLUMN artwork_identifier TEXT;

ALTER TABLE workspaces ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;
ALTER TABLE workspaces ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE workspaces ADD COLUMN project_root TEXT;
ALTER TABLE workspaces ADD COLUMN visual_identity TEXT;

ALTER TABLE logical_desktops ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;
ALTER TABLE logical_desktops ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE logical_desktops ADD COLUMN visual_identity TEXT;

-- 2. User preferences table
CREATE TABLE IF NOT EXISTS user_preferences (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 3. Management audit log
CREATE TABLE IF NOT EXISTS hierarchy_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id TEXT,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    version INTEGER NOT NULL,
    details TEXT,
    timestamp INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_entity ON hierarchy_audit_log(entity_type, entity_id);
```

## Security and Integrity Guarantees

1. **Single Schema Owner**:
   - `SqliteStore` in `depthos-core` is the single owner of the SQLite schema. Neither Python nor QML direct drivers touch the database file.
   - Database connections are opened with foreign keys enabled (`PRAGMA foreign_keys = ON;`).
   - Private file permissions: Database files must be private regular files (mode `0600`), owned by the current UID, with no hard links.

2. **Forward-Only Migrations**:
   - Migrations are tracked via `PRAGMA user_version`. Version 3 is verified on store open. Read-only commands refuse unmigrated databases without creating or migrating them.

3. **Optimistic Concurrency Control**:
   - Updates to `worlds`, `workspaces`, and `logical_desktops` require an `expectedVersion`.
   - The query verifies `WHERE id = ? AND version = ?`. If rows affected is 0, the store checks if the record exists; if it exists with another version, it returns `Error::Conflict`.

4. **Transactional Rollback**:
   - Sibling reordering and complex multi-item mutations occur inside a single SQLite transaction.
   - Any constraint violation (e.g. non-existent parent, foreign ID in reorder list) immediately rolls back the entire transaction, leaving the database state pristine.
