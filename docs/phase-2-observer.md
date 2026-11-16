# Phase 2.2A — standalone native observer adapter

## Scope and verdict

The implementation is in `crates/depthos-observer`. It is a foreground, read-only replacement for the Python diagnostic receiver, with no SQLite, project ownership, window control, installer, autostart, or host compositor operation. Phase 2.2B has not begun.

**PHASE 2.2A ACCEPTED** for the standalone protocol/replica/private-bus scope, subject to the verification commands and bounded limitations below. This does not open any Phase 1 host-native acceptance gate.

## Isolation and provenance

Destination: `/home/ashenix/Projects/DepthOS-observer-adapter`. This is a verified source copy, **not a linked Git worktree**. No Git repository was initialized and no commit, reset, clean, stash, merge, rebase, or push was performed.

`tests/observer-adapter/isolation/source-manifest.json` records the UTC start and verification times and SHA-256 hashes of 96 source files in deterministic path order. Copying used Python's `shutil.copy2` as the equivalent safe copying mechanism, refused an existing destination, rejected source symlinks, and compared the entire selected-file hash inventory before copying, after copying, and in the destination. All three inventories matched before implementation.

Excluded directory components: `.git`, `target`, `node_modules`, `.venv`, `__pycache__`, `.cache`, `build`, `artifacts`. Additional exclusions covered generated binaries, screenshots/recordings, temporary files, logs/PIDs, `.env*`, keys/certificates, and filenames identifying secrets or credentials. Untracked source was included. No private native evidence was copied.

A file copy cannot provide filesystem-level atomicity or lock out Agent A. Equality of the inventories establishes the recorded content baseline; subsequent original-source edits do not update this copy. Final read-only verification found concurrent changes in original `README.md`, `src/domain.rs`, and `src/sqlite.rs`. Every protected copied file still matched the baseline. Agent B's writes were confined to its allowed paths in the destination; the original was never a write target. The original therefore cannot be claimed globally unchanged while another agent works there. `verify-isolation.py` reports these differences separately; protected-copy differences fail verification, while concurrent original changes require attribution review.

## Architecture and dependencies

The adapter separates a deterministic `Replica` reducer, typed native models, an authenticated zbus endpoint, and a cloneable read-only `Observer`. A mutex serializes replica access; D-Bus authentication awaits occur outside that mutex. Publishing methods are serialized by the interface's mutable receiver. A Tokio watch counter delivers coalesced change notifications without copying window collections for each update. Consumers obtain an owned consistent snapshot when they need one.

The crate has its own manifest, lockfile and local `[workspace]` boundary so it builds independently beneath Agent A's root package or workspace. The root manifest is untouched. Direct dependencies are serde, serde_json, thiserror, Tokio, UUID v4, rustix (real UID), and zbus 5.19.0. This version was locally available, its source/API inspected, and its Tokio integration compiled on Rust/Cargo 1.100.0. Newer zbus 6 exists; this implementation uses the tested 5.19 API rather than assuming the major-version APIs are interchangeable. See [zbus 5.19 connection API](https://docs.rs/zbus/5.19.0/zbus/connection/struct.Connection.html).

Window updates use a `BTreeMap<String, Arc<Window>>` for O(log n) lookup/replacement. They never clone or serialize the complete retained state. The state holds no event history. Application-window filtering and ordered window materialization happen on explicit snapshot reads. Topology lists are replaced when the producer publishes their new values.

## Production protocol mapping

The unchanged `experiments/kwin-observer/contents/code/main.js`, its test fixtures, and `tools/state-inspector/{main,state}.py` were the wire specification. Schema version is exactly 1.

Service: `org.depthos.StateInspector`; path: `/org/depthos/StateInspector`; interface: `org.depthos.StateInspector1`. Only `Hello(s) -> s` and `Push(s) -> s` publish observations. Standard zbus introspection/peer/properties interfaces contain no window controls.

`Hello(observerSessionId)` returns JSON with `schemaVersion: 1` and this receiver instance's UUID `consumerId`. Hello validates and binds the producer session, drops old runtime state, and enters `AwaitingSnapshot`. The producer owns session-token generation; the receiver must not invent a replacement observer token. Every new receiver has a fresh consumer UUID.

`Push` accepts the exact envelope fields `schemaVersion`, `sessionId`, `consumerId`, `frames`, `position {sequence, revision}`, and optional `status: "payload-too-large"`. It returns `{"consumerId":"...","needSnapshot":true|false}`. Authenticated malformed packets request a snapshot using that same reply format. Authentication/invalid-Hello failures are fixed D-Bus errors; Rust uses standard `AccessDenied`/`InvalidArgs` error names rather than Python's interface-prefixed error names. The observer does not interpret error names.

Every frame requires schema/session, positive sequence/revision, and a known type. Supported payloads are:

- `snapshot`: `reason`, full `state`.
- `window.added`: `windowId`, complete `window`.
- `window.removed`: `windowId`.
- `window.focus.changed`: nullable `activeWindowId`.
- `window.metadata.changed`, `window.desktop.changed`, `window.activity.changed`, `window.geometry.changed`, `window.state.changed`: `windowId`, `changedFields`, complete `window`.
- `desktop.list.changed`: nullable `desktops`.
- `desktop.active.changed`: nullable `activeDesktopId`, `activeDesktopByScreen`.
- `activity.list.changed`: nullable `activities`.
- `activity.active.changed`: nullable `activeActivityId`.
- `screen.list.changed`: nullable `screens`.

The snapshot contains windows, desktops, Activity IDs, screens, applicationWindowIds, active IDs, per-screen active desktops, and typed capability/missing-signal metadata. All window fields emitted by production are represented, including `typeFlags`, capture exclusion, nullable native/window-system identity, title/app/resource metadata, parent, memberships and all-context flags, geometry, output, minimized/fullscreen flags, and `truncatedFields`. Focus is represented by the snapshot's `activeWindowId`; no fabricated per-window focus property is introduced. Activities have native IDs only, because the producer does not supply names.

Nullable metadata stays `Option<T>`. Empty Activity/desktop memberships retain their production meaning; missing native information never becomes a guessed application ID or membership. Null topology means unavailable, while an empty list is a known empty enumeration. Required nullable keys must still be present. Optional unknown fields are ignored within the defensive bounds.

All runtime windows must be referenced by `(sessionId, runtimeId)`. `WindowIdentity` and `ObservedSnapshot::window_identity` provide that pair. Screen IDs, fallback desktop IDs, session tokens, native window UUIDs, and even native desktop/Activity IDs are not promises of durable logical identity. No mapping to Agent A's persistent projects is made.

## Ordering and reconciliation

Lifecycle states are `Disconnected`, `Connecting`, `AwaitingSnapshot`, `Synchronized`, `Desynchronized`, and `Stopping`.

A validated snapshot is built separately and replaces the replica atomically. Snapshot sequence may jump forward for resynchronization, and revision may stay equal, but may not regress within the bound session. Incremental frames require both the next sequence and the next revision. Replayed positions are ignored and never restore unhealthy state. Invalid window references, missing frames, malformed required fields, unsupported versions/types, and payload-limit status make the replica unavailable and request a snapshot.

A nonempty packet may contain only the front of the producer's queued events. Its advertised position can be ahead. Only an **empty** heartbeat requires exact sequence and revision equality. The production heartbeat is every two seconds. The receiver's one-second liveness monitor marks a synchronized replica desynchronized after more than five seconds without a valid packet. A fresh higher-sequence snapshot can recover without another Hello under the existing owner/session binding.

New Hello calls invalidate old state even if the token is reused. A different Push session invalidates state and requires Hello before recovery. A changed KWin owner is detected by the monitor within one tick plus bounded bus-call latency, clears the session/owner binding, and makes snapshots unavailable. Every publication also rechecks current owner authentication, so the previous owner cannot publish during that interval. Read health may lag an owner transition until monitoring observes it; it is not an atomic view of the live compositor at read time.

A stale consumer ID is rejected without mutating the current healthy replica. A receiver restart creates a fresh UUID and starts without a snapshot. Bus loss invalidates readers; construct a new Receiver after the bus is available. The adapter does not silently reconnect while retaining an authoritative-looking cache.

D-Bus handlers can apply a valid prefix of a batch before encountering a semantic error; the mutex prevents intermediate reads and the final unhealthy state makes `snapshot()` fail. A later snapshot is required. The library never exposes the prefix as synchronized after an error.

## Authentication and security assumptions

Every Hello and Push checks the actual sender header, exact object/interface, the bus daemon's current `org.kde.KWin` unique owner, and that sender's Unix UID against the receiver process's real UID. Ownership is rechecked after credential resolution. The producer independently verifies receiver UID and calls its resolved unique name. A newly observed owner must Hello before Push.

The well-known receiver name is requested with explicit `DoNotQueue`, and primary ownership is checked. This matters because the tested zbus builder did not fail a collision when its implicit flags allowed queuing. The Rust and Python receivers are **alternatives**: stop one before launching the other. No multicast scheme is introduced. Future trusted fan-out belongs inside the controller process.

Only a single explicit `unix:` session-bus address is accepted. TCP, address fallback lists, unixexec and autolaunch are rejected. No system bus, network listener, shell execution, telemetry, or metadata persistence is implemented. Errors contain fixed messages, models containing private metadata do not derive Debug, and CLI title output is always redacted. Snapshot serialization in the public API contains native titles unless the caller invokes `redact_titles()`; callers own any deliberate export.

Defensive bounds: 4 MiB UTF-8 JSON packet; 32 frames; 4096 entries per JSON collection and retained windows; 4096 UTF-16 units per string; 512 bytes per object key; 64 KiB serialized per window record; serde_json's recursion limit. Status packets must not contain frames. Retained metadata is bounded by these per-record/count caps, not a measured RSS ceiling. The zbus queue is set to eight; D-Bus framing/string allocation occurs before application-level JSON rejection, so the four-MiB limit is not a pre-allocation transport sandbox. A same-user process may flood or impersonate the KWin service if it can acquire the name. These checks are neither cryptographic authentication nor a security boundary against arbitrary code already running as the same user.

A bus owner can change immediately after a credential check; no claim of an atomic ownership lease is made. Monitor invalidation and subsequent per-call verification provide bounded recovery.

## Public API and Phase 2.2B contract

Primary exports: `Receiver`, cloneable `Observer`, `ObservedStateSource`, `Replica` for offline replay, `ObservationHealth`, `ObservationError`, `ObservedSnapshot`, `NativeState`, `Window`, `WindowIdentity`, `Desktop`, `Output`, `Geometry`, `Classification`, and `Capabilities`.

`ObservedStateSource::snapshot()` returns an owned, mutex-consistent snapshot only when synchronized. Its sequence/revision identify the last successfully applied native position. Error variants contain no titles. `health()` exposes availability separately. `subscribe()` returns a Tokio watch receiver of generation counters: changes coalesce, slow consumers do not retain history, and consumers must reread current health/snapshot. A read can observe a later generation than the notification; sequence/revision remain internally consistent. Keep a consumer-side immutable snapshot only as historical information, never as a continuing guarantee of native health.

```rust,no_run
use depthos_observer::{ObservedStateSource, ObservationHealth, Receiver};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let receiver = Receiver::session().await?;
let source = receiver.observer();
let mut updates = source.subscribe(); // subscribe before the initial read
loop {
    if source.health() == ObservationHealth::Synchronized {
        if let Ok(snapshot) = source.snapshot() {
            // snapshot.session_id scopes every runtime_id.
            // Consume typed state here; do not route windows or persist it.
            let _ = snapshot;
        }
    }
    tokio::select! {
        _ = tokio::signal::ctrl_c() => break,
        changed = updates.changed() => { if changed.is_err() { break; } }
    }
}
receiver.shutdown().await?;
# Ok(()) }
```

Receiver shutdown marks Stopping, aborts/joins monitoring, releases the service name, closes the connection and publishes Disconnected. Watch coalescing means a slow reader may see only the final state. Drop also permanently stops writes and invalidates readers; explicit async shutdown gives deterministic name-release completion. Read handles remain safe to use afterwards and report unavailable. Mutex-protected access and immutable owned snapshots are thread safe. The offline Replica needs external synchronization when shared.

For Phase 2.2B, copy only the adapter crate, its tests/utilities and this report into the integration checkout after reviewing Agent A's current source. Explicitly reconcile root workspace membership/dependencies; remove the crate's local workspace boundary only when adding it to the root workspace, and decide which lockfile to retain. Add a path dependency in Agent A's crate as an integration patch, not as part of this phase. Have Agent A consume `ObservedStateSource` and health notifications; no raw JSON parsing, SQLite coupling, or imports of unfinished core types are required. Any later project associations need their own policy and authorization design.

## Development CLI and approved future host run

From the isolated copy:

```sh
cargo run --manifest-path crates/depthos-observer/Cargo.toml -- inspect
cargo run --manifest-path crates/depthos-observer/Cargo.toml -- health
cargo run --manifest-path crates/depthos-observer/Cargo.toml -- snapshot --redact-titles
```

Each command starts the foreground receiver and exclusively owns the service while running. `inspect`/`health` print health changes only; `snapshot` waits for a synchronized snapshot, prints redacted JSON and shuts down. All commands announce waiting when no observer is connected. Ctrl+C or SIGTERM performs graceful shutdown. No command loads KWin code, creates a daemon, or installs an autostart entry.

A future host run requires a separate authorization and the existing version/session acceptance gates. Stop the Python inspector first, verify installed/running KWin and the current session prerequisites, launch this receiver, and only then use the existing approved observer lifecycle instructions in `poc-02a.md`. Do not bypass that gate using historical acceptance or the isolated 6.8.3 qualification. Allow the producer's documented approximately 33-second reconnect watchdog delay. This phase did not execute any host run.

## Verification and compatibility evidence

Run from the isolated destination:

```sh
cargo test --manifest-path crates/depthos-observer/Cargo.toml
cargo clippy --manifest-path crates/depthos-observer/Cargo.toml --all-targets -- -D warnings
python3 tests/observer-adapter/parity.py
python3 tests/observer-adapter/private-bus.py
cargo run --release --manifest-path crates/depthos-observer/Cargo.toml --example benchmark
python3 tests/observer-adapter/verify-isolation.py
```

Rust coverage: 16 protocol/replica tests, one UID/owner policy unit test, and four private-D-Bus tests. Private buses are freshly spawned daemons even when running plain cargo test; no inherited session bus is used. The Python interoperability harness explicitly starts `dbus-run-session` and refuses a populated bus. Coverage includes registration/collision, Hello validation, unauthorized Hello/Push, stale consumers, current-owner transfer, required re-Hello, receiver restart, redacted CLI export, SIGTERM/name release, heartbeat expiry and snapshot recovery, network-address rejection, bus loss and shutdown/drop.

A genuinely different-UID process on a private bus was not launched. UID mismatch/root UID rejection is unit-tested in the actual predicate used by transport, and real same-UID bus credentials are exercised end to end. Cross-UID daemon policy/credential behavior remains explicitly untested rather than weakened. This is an allowed test limitation, not evidence of host authentication qualification.

`generate-trace.cjs` executes the unchanged production observer against sanitized fixture objects. The recorded `trace.json` includes **all 14 frame types**; no private evidence is used. `parity.py` verifies fixture drift, then compares Python/Rust state and health for seven scenarios and 30 packet positions: full lifecycle, duplicates, missing sequences, revision gaps, heartbeat final loss, partial-batch positions, receiver starting mid-stream, and snapshot recovery. It compares redacted output and preserves null metadata. Production Python is imported unchanged; neither production implementation is patched.

Intentional differences from Python:

- Rust binds session to Hello, requires complete producer-emitted nullable fields, and enforces revision as well as sequence on empty heartbeats. Python Hello resets without binding the supplied session and heartbeat comparison checks sequence alone.
- Rust rejects invalid removal/focus references and duplicate additions; Python can silently ignore an unknown removal or accept an unknown focus/addition. Topology membership references are not rejected per event because native list and window events can legitimately arrive separately.
- Rust exposes no unhealthy snapshot. Python retains diagnostic stale state with `need_snapshot` separately. Recognizable stale-consumer packets leave Rust's current healthy generation untouched.
- Rust requires positive heartbeat revision, bounded records/collections and known status; Python has looser validation, including a hand-authored test status packet without revision. Production JS includes both position fields.
- Unknown optional fields are ignored; they are not preserved in Rust's typed serialization. The parity fixture uses only actual production fields.

Audit discrepancies: architecture/README historical sections describe Rust as absent, whereas this source snapshot already contains Agent A's in-progress Rust core and SQLite manifest. Older Phase 1/POC-02B pending/live statements coexist with later reports. POC-02G explicitly closes the host gate despite isolated 6.8.3 success; it takes precedence over historical 6.8.1 approval for any future host run. No existing report or gate was rewritten. Source, not older prose or an illustrative guessed wire shape, determined this adapter.

Final verification on 2026-11-16: all **21 Rust tests passed** (16 replica/protocol, 4 private-bus, 1 authorization predicate); fmt and Clippy with warnings denied passed; all seven parity scenarios / 30 packet comparisons passed; Python-to-Rust `dbus-run-session` interoperability and CLI shutdown passed. Isolation verification passed for all 96 protected copied files and reported the three concurrent original-source differences described above.

Created files comprise the crate manifest/lockfile; `src/{lib,model,protocol,replica,transport,main}.rs`; `tests/{replica,private_bus}.rs`; offline `examples/{replay,benchmark}.rs`; the adapter trace generator/recording, parity/private-bus/isolation scripts and provenance manifest under `tests/observer-adapter`; and this document. No existing production source or root integration file was edited.

## Performance and remaining limits

The release benchmark constructs 1000 sanitized windows and applies 10,000 geometry packets, with packet construction excluded from timing. It measures JSON decoding, validation and reducer work only. The measured release run took 0.109 seconds, approximately 10.9 microseconds per packet (91,868 packets/second). Results depend on this machine/build and do not establish KWin CPU/GPU overhead, latency, FPS, sustained daemon memory behavior or host reliability. No full-state serialization occurs for a small event; explicit caller snapshot reads cost O(number of live windows).

The implementation deliberately performs defensive JSON passes and a bounded window-record serialization for validation. It can be optimized after profiling; native transport and correctness are not traded away for synthetic throughput. Watch notifications coalesce burst updates rather than creating an unbounded event backlog. Liveness is monitored at one-second intervals without native-property polling or busy waiting.

Remaining work is Phase 2.2B workspace/dependency integration, caller policy for availability and export, and separately approved host-native qualification. Snapshot pagination for environments beyond producer/receiver limits, mixed-output/native Activity lifecycle qualification, real cross-UID bus testing and long-duration native reliability are outside the verified scope. The existing observer's payload-too-large condition remains fail-closed until its documented restart/reduction recovery; Rust does not alter that protocol.

## Phase 2.2B integration update — 2026-11-16

This report preserves the accepted standalone Phase 2.2A implementation and its original evidence. The canonical repository now contains this crate as a member of the root Cargo workspace. Its nested workspace declaration and local lockfile have been removed in the integrated copy; observer-only tests remain available through `cargo test --locked --manifest-path crates/depthos-observer/Cargo.toml` using the single root lockfile. The original isolated source remains unchanged as provenance.

The application directly adopts the verified model/trait, and adds storage-independent `depthos system` diagnostics. Transport/reducer validation is unchanged. A public error classification helper identifies service-name collisions without logging raw bus errors. Replay helpers use `--locked`, and isolation verification recognizes the integration provenance manifest. See [Phase 2.2B](phase-2-integration.md) for the final application contract, complete verification, safe rollback, and remaining native qualification limits.
