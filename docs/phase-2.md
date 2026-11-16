# DepthOS Phase 2.1 — Rust workspace orchestration core

Phase 2.1 adds a local Rust library and the `depthos` development CLI. Phase 1 remains partially accepted. The logical core runs independently of the live compositor, so its acceptance can proceed while host input authorization and presentation timing remain pending.

## Logical model and state ownership

The Universe is the single implicit root per database. It needs no row because it has no independent identity, settings, or lifecycle. Worlds belong to that root, workspaces belong to exactly one World, and logical desktops belong to exactly one Workspace. The database starts empty. Each entity has a typed UUID, a name, sibling order, and UTC Unix millisecond creation and update timestamps. Worlds and workspaces support descriptions, workspaces support an optional absolute root directory, and logical desktops support an optional role. Root directories are stored as text and may refer to a location that does not yet exist.

DepthOS owns this desired hierarchy. KWin owns actual virtual desktops, Activities, windows, focus, and outputs. Logical desktops are independent of KWin desktop IDs and the current Z0–Z3 overview levels. The `ObservedStateSource` trait is a read-only contract for a reconciled runtime projection. It carries the observer schema version, session ID, sequence, revision, and session-scoped window IDs. The projection preserves optional membership and focus information. A future adapter will translate the POC-02A wire fields to these Rust types after authentication and reconciliation. Observed state is ephemeral and has no SQLite tables.

Policy categories and an explicit application association type leave room for future ownership rules. The application key still needs a defined namespace and provenance before a user association can be persisted. Window titles and runtime IDs supply no durable ownership evidence.

## Storage and application API

A single Cargo package contains typed entities and errors in `src/domain.rs`, the store contract and domain service in `src/service.rs`, the SQLite adapter in `src/sqlite.rs`, the observed state contract in `src/observed.rs`, and the Clap CLI adapter in `src/main.rs`. The service trims names, rejects controls and names outside 1–120 characters, validates optional metadata, and issues a single storage command per mutation.

SQLite schema v1 is defined in `migrations/001_initial.sql`. Three strict tables hold 16-byte UUID primary keys unique within their typed entity tables. Foreign keys are enabled on every connection. Both parent relationships use `ON DELETE RESTRICT`. Name uniqueness uses SQLite NOCASE, which folds ASCII letters. The scope is the Universe for worlds, the World for workspaces, and the Workspace for logical desktops. Non-ASCII case and normalization variants are distinct. Composite indexes support parent-scoped reads ordered by sort order and UUID. New items append within their scope. A move appends the workspace in its destination and preserves child IDs. Moving to the current parent is a no-op. Reorder accepts a complete permutation and assigns contiguous positions. Parent deletion requires an empty parent.

The CLI opens one synchronous connection per invocation and uses SQLite's default rollback journal. Migrations and mutations use immediate transactions with a five-second busy timeout. The write lock covers any parent validation, destination checks, and order allocation. Failed edits roll back together. List and tree operations use a read snapshot where multiple statements are required. Loading a tree uses three ordered queries and groups children by typed parent ID. Individual mutations query their affected rows or sibling scope. Update timestamps advance even for multiple writes in the same millisecond. A long-lived daemon, connection pool, WAL configuration, Tokio runtime, and tracing subscriber are deferred because there is no asynchronous or concurrent service workload yet.

On open, the adapter checks SQLite's `application_id` and `user_version` under the migration lock. A fresh empty database is initialized and marked in the same transaction as its schema creation. Unsupported version numbers and foreign databases are rejected. Migration errors retain the SQLite cause. Future versions append ordered SQL migration files and advance the version only when the whole transaction commits. The current release is the first schema, so there are no earlier DepthOS schemas to upgrade. `doctor` checks table readability, schema version, foreign key enforcement and violations, and SQLite's quick integrity check.

## CLI and storage paths

The default path is `$XDG_DATA_HOME/depthos/workspaces.sqlite3`, or `$HOME/.local/share/depthos/workspaces.sqlite3` when XDG_DATA_HOME is absent or relative. Its DepthOS directory is created with mode 0700 and its database with mode 0600. An explicit `--db` file can be absolute or relative to the current directory and must have an existing trusted parent writable only by its owner. Database symlinks, hard links, nonregular files, and group- or publicly accessible files are rejected. Keep database files in a directory controlled by the operator. All commands including `doctor` open storage and initialize it if empty. The CLI accepts text as data and uses bound SQL parameters.

Build from the repository on Linux with stable Rust and system SQLite 3.42 or newer available through pkg-config. Rust 1.100.0 and SQLite 3.54.1 were used for validation. The example uses Python 3 to extract the JSON IDs. The lockfile pins the resolved Cargo dependencies. The commands use a disposable database.

```bash
cargo build --locked
test_dir=$(mktemp -d)
db="$test_dir/example.sqlite3"
WORLD=$(target/debug/depthos --db "$db" world create "Projects" |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["id"])')
target/debug/depthos --db "$db" world list
WORKSPACE=$(target/debug/depthos --db "$db" workspace create --world "$WORLD" --name "Example" |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["id"])')
target/debug/depthos --db "$db" desktop create --workspace "$WORKSPACE" --name "Development"
target/debug/depthos --db "$db" tree
target/debug/depthos --db "$db" doctor
```

All entity groups also expose `rename` and `delete`. `workspace move ID --world WORLD` changes the parent, and `desktop reorder --workspace WORKSPACE ID...` takes the full order. Every group has `--help`. Create and list commands, tree, and doctor print JSON. Successful edits and deletions are silent. Argument and domain validation failures report to stderr and exit 2. Storage and diagnostic failures exit 1.

## Validation and Phase 2.2 boundary

```bash
cargo test --locked
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
```

The 12 automated tests cover schema migration and rollback, hierarchy construction, persistent ordering and serialization after reopen, typed validation and SQL constraints, conflicting moves, partial reorder rollback, restricted deletion, schema rejection, and storage file safety. CLI integration runs separate processes against temporary files, including concurrent first open with a duplicate name, and creates and reads the same hierarchy across process exits. Its environment clears display variables, points D-Bus at an absent socket, and redirects HOME and XDG storage into the test directory. The CLI also verifies that names containing shell and SQL syntax persist literally. No production database participates in these checks.

The next integration boundary is a read-only adapter from the authenticated, reconciled POC-02A observer feed to `ObservedStateSource`. It will need to cover compositor session replacement, revision gaps, unavailable snapshots, and redaction. Opening or editing a logical hierarchy does not require that adapter. Launching, routing, restoration, window movement, and durable application bindings remain outside the current milestone.

## Phase 2.2B integration update — 2026-11-16

The historical Phase 2.1 report above is retained. The observer adapter is now a Cargo workspace member, with one root lockfile and shared dependency declarations. `src/observed.rs` re-exports the adapter's complete model and `ObservedStateSource` rather than translating into the earlier unused placeholder types. `src/application.rs` exposes logical CRUD and verified native observations through independent service boundaries. System diagnostics do not open SQLite, and ordinary hierarchy commands require no bus or compositor.

`depthos system health`, `inspect`, and `snapshot --redact-titles` use a foreground, exclusive receiver with a bounded observation window. They never load scripts. See [Phase 2.2B](phase-2-integration.md) for contract migration, CLI examples, test evidence, backup/rollback, and the unchanged host-native acceptance boundary. No Phase 2.3 work is included.
