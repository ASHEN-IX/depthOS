-- Migration 003: Hierarchy management, visual identity, archiving, optimistic concurrency & preferences

ALTER TABLE worlds ADD COLUMN visual_identity TEXT;
ALTER TABLE worlds ADD COLUMN artwork_identifier TEXT;
ALTER TABLE worlds ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;
ALTER TABLE worlds ADD COLUMN archived_at_ms INTEGER;
ALTER TABLE worlds ADD COLUMN version INTEGER NOT NULL DEFAULT 1;

ALTER TABLE workspaces ADD COLUMN visual_identity TEXT;
ALTER TABLE workspaces ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;
ALTER TABLE workspaces ADD COLUMN archived_at_ms INTEGER;
ALTER TABLE workspaces ADD COLUMN version INTEGER NOT NULL DEFAULT 1;

ALTER TABLE desktops ADD COLUMN visual_identity TEXT;
ALTER TABLE desktops ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;
ALTER TABLE desktops ADD COLUMN archived_at_ms INTEGER;
ALTER TABLE desktops ADD COLUMN version INTEGER NOT NULL DEFAULT 1;

CREATE INDEX IF NOT EXISTS worlds_archived ON worlds (is_archived, sort_order, id);
CREATE INDEX IF NOT EXISTS workspaces_archived ON workspaces (world_id, is_archived, sort_order, id);
CREATE INDEX IF NOT EXISTS desktops_archived ON desktops (workspace_id, is_archived, sort_order, id);
CREATE INDEX IF NOT EXISTS workspaces_archived_at ON workspaces (archived_at_ms DESC) WHERE is_archived = 1;
CREATE INDEX IF NOT EXISTS desktops_archived_at ON desktops (archived_at_ms DESC) WHERE is_archived = 1;

CREATE TABLE IF NOT EXISTS preferences (
    key TEXT NOT NULL PRIMARY KEY CHECK (length(key) BETWEEN 1 AND 80),
    value TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))
) STRICT;

CREATE TABLE IF NOT EXISTS operation_audit (
    operation_id TEXT NOT NULL PRIMARY KEY,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('world', 'workspace', 'desktop', 'preference')),
    entity_id TEXT,
    action TEXT NOT NULL,
    details TEXT,
    performed_at_ms INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))
) STRICT;
