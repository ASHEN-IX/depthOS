# Phase 4.3 persistence

Migration `002_desktop_association_rules.sql` adds the strict rule table and indexes. A rule is unique by kind and key. Native desktop keys are canonical UUIDs. Application keys are case-sensitive ASCII identifiers of at most 160 characters. An application rule requires a target Logical Desktop. A native desktop rule can target only a Workspace to support an Unassigned shelf state. A composite foreign key checks that the optional Desktop belongs to the exact Workspace.

Rules contain no observer epochs, native window UUIDs, captions, runtime IDs, credentials, or other window records. Explicit deletion of a target cascades to its rules. Existing hierarchy data is preserved on migration.

A version 1 database can be read without modification. Its rule set is empty. The foreground controller opens an existing database read-only. New databases start at version 2. An explicit association operation against an existing database performs needed migration and rule changes in one IMMEDIATE transaction. Failed validation rolls back both. The normal private-file safeguards require ownership by the current UID, restrictive mode, a regular file, and one link. The existing application ID and schema checks reject foreign or unsupported databases.

Use the CLI with a DepthOS-owned path and IDs.

```sh
target/debug/depthos --db "$fixture_db" association set native-desktop "$kde_id" \
    --workspace "$workspace_id" --desktop "$desktop_id"
target/debug/depthos --db "$fixture_db" association set application org.kde.konsole \
    --workspace "$workspace_id" --desktop "$desktop_id"
target/debug/depthos --db "$fixture_db" association remove application org.kde.konsole
```

Tests use private fixture databases. They cover rollback during version 1 upgrade, idempotent writes, a concurrent read-only handle seeing a committed upgrade, parent validation, rule removal through Desktop deletion, and refusal of missing, foreign, and future schemas.
