# DepthOS Phase 2.2B — persistent core and native observations

**PHASE 2.2B ACCEPTED — 2026-11-16**, within the local SQLite, production-protocol replay, and private-D-Bus scope described here. Host-native qualification remains gated. Phase 2.3 has not begun.

## Pre-integration audit and recovery baseline

Integration was performed in `/home/ashenix/Projects/DepthOS`. The canonical Phase 2.1 baseline passed all 12 Rust tests; the isolated `/home/ashenix/Projects/DepthOS-observer-adapter` baseline passed all 21 Phase 2.2A Rust tests. Both engineering reports, manifests/lockfiles, public contracts, and original observer provenance were inspected before source changes.

The observer's source-manifest comparison identified newer canonical `README.md`, `src/domain.rs`, and `src/sqlite.rs`, plus canonical-only `docs/phase-2.md`, `tests/workspace_cli.rs`, and `tests/workspace_core.rs`. These were preserved. No older Agent A source from the isolated copy was transferred.

The available chat inventory showed this integration as the only active DepthOS chat. A `/proc` writable-file-descriptor scan found no source writer, and source hash inventories remained equal across a stability check and backup verification. This is a point-in-time concurrency check, not a lock against another editor starting later. Protected source hashes were checked again after implementation.

Verified recovery backup:

`/home/ashenix/Projects/DepthOS-backups/phase-2.2b-20261116T132835Z/`

Its `source/` contains 99 canonical source files; `manifest.json` records deterministic SHA-256 inventories, the 19 accepted observer deliverables selected for transfer, source evolution since the earlier isolated snapshot, and selected KDE configuration hashes. Copying excluded Git metadata, private evidence/screenshots/recordings, build outputs, caches, temporary runtime files, logs and credential/secret material. Selected source inventories matched before and after copying and matched the backup. No Git operation requiring HEAD, commit, reset, stash, merge, rebase or push was performed.

Transfer used exclusive file creation and verified every transferred hash, after confirming every destination file was absent. Only `crates/depthos-observer/**`, `tests/observer-adapter/**`, and `docs/phase-2-observer.md` were copied. Existing canonical source was not replaced during transfer. Subsequent, explicitly scoped integration edits are recorded in `tests/observer-adapter/isolation/integration-manifest.json`.

## Final Cargo structure

The root remains the `depthos-core` package with the existing `depthos` executable. It is also the Cargo workspace root, with `crates/depthos-observer` as its other member. Resolver 3 and both default members ensure plain root build/test/Clippy commands cover the application and adapter. The adapter's independent `depthos-observer` diagnostic executable remains available.

There is one root `Cargo.lock`. The integrated observer's local lockfile and nested `[workspace]` declaration were removed. Shared serde, serde_json, thiserror, UUID, Tokio, rustix and zbus declarations live under `[workspace.dependencies]`; root-only Clap/SQLite and test-only tempfile remain scoped to their users. Comparing resolved package versions with both accepted lockfiles found **no changed existing dependency version pins**. The lockfile adds the union of the required dependencies rather than upgrading either implementation. UUID's serde/v4 features are shared; zbus retains its Tokio backend without its default runtime features.

The crate can still be tested directly:

```sh
cargo test --locked --manifest-path crates/depthos-observer/Cargo.toml
```

This uses the root workspace lockfile. A copy of only the crate directory is no longer a standalone workspace: its workspace dependency declarations require the root manifest. The untouched isolated Phase 2.2A source remains independently buildable if a standalone baseline is needed.

## One observed-state contract

`depthos_core::observed` re-exports the adapter's `ObservedStateSource`, `ObservedSnapshot`, `NativeState`, window/desktop/output/capability types, health/errors, read handle, and session-scoped `WindowIdentity`. There is one reducer: `depthos_observer::Replica` behind the authenticated receiver.

The earlier Phase 2.1 structs were unused placeholders, with no in-tree consumers beyond their definitions. They were removed rather than maintaining a second projection. This is an intentional pre-release API migration:

- The old `Result<Option<ObservedState>, E>` becomes `Result<ObservedSnapshot, ObservationError>`; unavailable state is explicit through `Unavailable(health)`.
- Session, schema, sequence and revision are on `ObservedSnapshot`; native fields are under `snapshot.state`.
- Window metadata, classification, capture restrictions, capabilities, geometry/scale, memberships and parent references preserve the full verified producer model.
- Fractional native geometry stays floating point; nullable topology stays `Option<Vec<_>>`. Missing native metadata never becomes an invented ID, empty collection, or integer geometry.
- `health()` and coalescing `subscribe()` watch notifications come from the same source that authenticates and reconciles the packets.

Snapshot reads are mutex-consistent and succeed only when synchronized. A connected D-Bus socket alone does not imply synchronized state. Watch delivery can coalesce updates; subscribe before the first read, then reread snapshot/health after each notification. Use snapshot success, not a preceding separate health read, to decide whether its data is available. An owned snapshot is an historical observation, not a lease on live native state.

`WindowIdentity` pairs session and runtime ID. The same `window-1` in another session is a different live identity. No native ID becomes a durable logical desktop, workspace or world key. SQLite entity keys remain typed UUIDs. Session changes, sequence/revision gaps, heartbeat loss, reconciliation and disconnect behavior retain Phase 2.2A's stricter policy.

## Application boundaries

`Application<S, O>` holds a `WorkspaceService<S>` and an `ObservedStateSource` read handle. It provides `hierarchy()`, `logical()`, `logical_mut()`, and `observations()`. The caller retains the `Receiver` lifecycle owner. `Application::disconnected(store)` uses an explicit unavailable source with no runtime window collection and no reducer or bus connection.

```rust,no_run
use depthos_core::{application::Application, observed::ObservedStateSource, sqlite::SqliteStore};
use depthos_observer::Receiver;
use std::path::Path;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let store = SqliteStore::open(Path::new("/trusted/private/directory/workspaces.db"))?;
let receiver = Receiver::session().await?;
let application = Application::new(store, receiver.observer());
let mut updates = application.observations().subscribe();
let logical = application.hierarchy()?;
let observed = application.observations().snapshot(); // may be unavailable
let _ = (logical, observed, &mut updates);
receiver.shutdown().await?;
// application.hierarchy() remains usable after observer shutdown.
# Ok(()) }
```

No cross-store transaction or persistence callback connects these components. Observer events cannot access the SQLite store. The SQLite schema and its migrations were unchanged; no runtime-window table, association, routing, restoration, AI or ownership policy was added. Logical commands use this application boundary with the disconnected source and never initialize Tokio or access D-Bus. The synchronous SQLite service API remains intact.

## Receiver lifecycle and service ownership

Only one Python or Rust receiver may own `org.depthos.StateInspector`. Foreground system commands use the accepted Rust receiver with an explicit nonqueued name request and no replacement flag. An existing receiver causes a clear conflict error and exit 1. The existing receiver is neither stopped nor displaced; a test uses the actual unchanged Python inspector to prove this.

Hello/Push, exact object/interface checks, current KWin unique-owner verification and real-UID credential checks are unchanged. Receiver restarts create a fresh consumer ID; new observer sessions invalidate old state; only reconciled snapshots become available. Every receiver is explicitly shut down on command completion, timeout, Ctrl+C or SIGTERM. Shutdown releases the name and invalidates retained read handles. Drop also invalidates readers on error paths.

No system bus, privileged service, network listener, external socket transport, automatic KDE mutation, KWin loader or effect installer is used. No native window-control method is exposed by the observation contract; a compile-fail documentation test protects that API boundary. Logical desktop CRUD continues to operate only on DepthOS's SQLite hierarchy.

## Unified CLI behavior

Existing `world`, `workspace`, `desktop`, `tree`, and `doctor` syntax/semantics are preserved. The new commands are:

```sh
cargo run --locked -p depthos-core --bin depthos -- system health
cargo run --locked -p depthos-core --bin depthos -- system inspect --timeout-seconds 30
cargo run --locked -p depthos-core --bin depthos -- system snapshot --redact-titles
```

Each is a foreground receiver. `--timeout-seconds` is 1–300, default 5, and bounds the observation window after receiver startup. `health` waits for a verified snapshot or reports unavailability at timeout; `snapshot` emits the first synchronized snapshot or an availability/status object. `inspect` prints JSON summaries/health changes during its observation window. Its output may contain multiple pretty-printed JSON objects, so stream consumers should use a JSON decoder rather than assuming JSON Lines.

Titles are always redacted in snapshot output, including when `--redact-titles` is omitted. Inspect emits only counts/positions and health. Waiting text and fixed errors go to stderr. An unavailable session-bus endpoint returns a `Disconnected` JSON status with `session-bus-unavailable`; an available bus without an observer yields an unavailable status at timeout. These are successful diagnostics (exit 0), not fabricated windows. Snapshot/status shapes are distinct; check `state` or `snapshotAvailable` before treating output as native data. Service-name conflicts and execution/output failures exit 1. Clap retains its argument-error behavior.

System diagnostics never open SQLite, even if `--db` is supplied. `--db` remains a global logical-command option and is ignored by system diagnostics. This lets diagnostics work independently of database availability or permissions. A default session database is never created as a side effect of system observation.

The CLI's synchronized health result includes a session/sequence/revision from a successful consistent snapshot. It does not infer synchronization from transport startup. The standalone adapter CLI remains unchanged for diagnostic compatibility; use `-p depthos-observer --bin depthos-observer` when selecting that executable through Cargo.

## Verification evidence

Commands run successfully:

```sh
cargo build --locked
cargo test --locked
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --manifest-path crates/depthos-observer/Cargo.toml
python3 tests/observer-adapter/parity.py
python3 tests/observer-adapter/private-bus.py
python3 tests/observer-adapter/unified-cli.py
python3 tests/observer-adapter/verify-isolation.py
```

The integrated Rust suite contains **37 passing checks**: 12 unchanged Phase 2.1 tests, 21 unchanged Phase 2.2A tests, three application integration tests, and one compile-fail API-boundary documentation test. The application tests use temporary SQLite files and newly spawned private bus daemons; none uses the inherited desktop bus. They verify offline logical CRUD; observation over an empty hierarchy; both views from one application; byte-for-byte database preservation after initial sync and incremental replay; unchanged three-table schema/typed UUID keys; unknown metadata; notification delivery; session identity replacement; sequence and revision recovery; and hierarchy operations after receiver shutdown.

The existing production-generated sanitized trace remains unchanged and covers all 14 frame types. Python/Rust parity passes seven scenarios and 30 packet comparisons. The observer's Python-to-Rust Hello/Push harness explicitly runs under `dbus-run-session` and verifies redaction and service release. The unified CLI harness separately verifies all system commands with an unavailable endpoint, absent KWin on a private bus, an actual Python inspector collision without owner replacement, synchronized health, default redaction, inspect summaries, timeout/SIGTERM cleanup, and absence of any SQLite creation. Original core CLI tests still run CRUD with no desktop session.

The final isolation manifest records 93 protected canonical source hashes, the transferred source hashes, and 29 final integration-file hashes (excluding the manifest itself). `src/domain.rs`, `src/service.rs`, `src/sqlite.rs`, migrations, Phase 1 observer/QML/effect sources, Python production receiver, existing core tests, and prior Phase 1 reports remain byte-identical to the backup. Existing hashes for `~/.config/{kwinrc,kglobalshortcutsrc,kactivitymanagerdrc,kwinrulesrc}` also match. This is the recorded configuration comparison scope; no broad claim about every unrelated KDE file is implied. No host configuration or compositor mutation command was executed.

## Security assumptions and known limitations

The accepted adapter's bounds, malformed-packet rejection, session binding, stale-consumer rejection and unhealthy-state unavailability are preserved. Public native model serialization can include titles when deliberately used by a caller; the integrated CLI redacts them and never logs raw payloads or bus errors. The application adds no telemetry, shell execution or persistent native metadata.

Same-user D-Bus owner/UID validation is not cryptographic authentication or a boundary against arbitrary same-user code. A real different-UID process on a private bus remains untested; UID mismatch rejection is unit-tested in the actual authorization predicate, and real same-UID credentials are tested end to end. Bus ownership can change immediately after a check; monitor invalidation is bounded rather than an atomic ownership lease. JSON limits apply after D-Bus framing allocation. These remain the Phase 2.2A limitations, not new host qualification evidence.

Synthetic/private-bus success does not establish actual host KWin compatibility, native overhead, long-duration reliability, output hotplug, or Activity lifecycle qualification. Existing Phase 1 version/session and authorization gates remain closed as documented. Nothing in this integration installs or starts KWin code. A receiver cannot obtain metadata until an explicitly approved observer is running.

## Safe rollback and Phase 2.3 readiness

The backup is recoverable source, not a Git commit. Before rollback, stop this foreground receiver normally and confirm no other writer is editing the relevant source. Read the integration manifest and compare current hashes with its final integration-file hashes. If any differs, preserve/review that newer change before restoring anything.

Restore only the modified pre-existing files listed by the manifest from the backup's `source/` copies. Remove only newly added integration files whose hashes still match the recorded integration hashes; leave generated build artifacts and unrelated source alone. Do not copy the entire old tree over newer files and do not run Git reset/clean/stash. The original isolated adapter remains available for comparison. The untouched isolated adapter retains its original crate-local Cargo.lock if a standalone observer export is needed. No KDE rollback is required because no KDE mutation occurred.

The foundation is ready for a separate Phase 2.3 design and authorization discussion: it can read a persistent logical hierarchy alongside an explicitly healthy native observation stream without linking their identities or changing the desktop. Routing, durable associations, movement, AI and restoration require their own policy and tests. They have not been started here.
