CREATE TABLE worlds (
    id BLOB NOT NULL PRIMARY KEY CHECK (length(id) = 16),
    name TEXT NOT NULL COLLATE NOCASE UNIQUE CHECK (length(name) BETWEEN 1 AND 120),
    description TEXT,
    sort_order INTEGER NOT NULL CHECK (sort_order >= 0),
    created_at_ms INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at_ms INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))
) STRICT;

CREATE TABLE workspaces (
    id BLOB NOT NULL PRIMARY KEY CHECK (length(id) = 16),
    world_id BLOB NOT NULL REFERENCES worlds(id) ON DELETE RESTRICT,
    name TEXT NOT NULL COLLATE NOCASE CHECK (length(name) BETWEEN 1 AND 120),
    description TEXT,
    root_directory TEXT,
    sort_order INTEGER NOT NULL CHECK (sort_order >= 0),
    created_at_ms INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at_ms INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    UNIQUE (world_id, name)
) STRICT;

CREATE TABLE desktops (
    id BLOB NOT NULL PRIMARY KEY CHECK (length(id) = 16),
    workspace_id BLOB NOT NULL REFERENCES workspaces(id) ON DELETE RESTRICT,
    name TEXT NOT NULL COLLATE NOCASE CHECK (length(name) BETWEEN 1 AND 120),
    role TEXT,
    sort_order INTEGER NOT NULL CHECK (sort_order >= 0),
    created_at_ms INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at_ms INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    UNIQUE (workspace_id, name)
) STRICT;

CREATE INDEX worlds_order ON worlds (sort_order, id);
CREATE INDEX workspaces_order ON workspaces (world_id, sort_order, id);
CREATE INDEX desktops_order ON desktops (workspace_id, sort_order, id);
