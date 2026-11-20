CREATE UNIQUE INDEX desktop_workspace_identity ON desktops (workspace_id, id);

CREATE TABLE desktop_association_rules (
    kind TEXT NOT NULL CHECK (kind IN ('native-desktop', 'application')),
    rule_key TEXT NOT NULL CHECK (length(rule_key) BETWEEN 1 AND 160),
    workspace_id BLOB NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    desktop_id BLOB,
    PRIMARY KEY (kind, rule_key),
    FOREIGN KEY (workspace_id, desktop_id) REFERENCES desktops(workspace_id, id) ON DELETE CASCADE,
    CHECK (kind != 'application' OR desktop_id IS NOT NULL)
) STRICT;

CREATE INDEX desktop_association_rules_target ON desktop_association_rules (workspace_id, desktop_id);
