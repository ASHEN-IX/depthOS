# Phase 2.4 — foreground controller, lifecycle and recovery

**PHASE 2.4 ACCEPTED** for the local Rust controller, deterministic fixtures and private-D-Bus/process tests below. Phase 1 remains partially accepted; no host compositor acceptance gate was changed or exercised. Phase 2.5 has not begun.

## Architecture and runtime ownership

`src/controller.rs` owns one `Application<SqliteStore, ControllerSource>`, its existing SQLite-backed `WorkspaceService`, one optional Rust `Receiver`, the existing observed read handle, one `ContextCoordinator`, and a watch subscription. A single Tokio task serializes controller requests and update processing. The existing receiver retains its one liveness-monitor task and zbus/runtime internals. Receiver registration is a cancellable future inside the controller task, not another spawned worker. Production controller code spawns no processes.

`ControllerSource` holds only an optional verified `Observer` read handle behind a short lock. It contains no window collection or native reducer. The adapter remains the only observed-state reducer; the accepted SQLite implementation and schema v1 remain the only hierarchy store. Existing epoch, per-window generation, eligibility, claim privacy and CAS rules are reused unchanged. The coordinator gained `synchronize()` to refresh continuity and logical revisions without constructing every association decision. The receiver gained a read-only `is_closed()` query so bus failure can be distinguished from absent KWin.

No dependency, Cargo manifest, lockfile, migration, logical domain/store implementation or producer protocol changed. There is no daemon, systemd unit, autostart, network endpoint, privileged service, renderer, routing policy or native action executor.

## Lifecycle state machine and invariants

`ControllerHealth` exposes lifecycle, observation health, receiver connection availability, registration-attempt count, database availability, a fixed recovery reason and an optional context version. Its synchronization checkpoint comes from the same consistent coordinator snapshot read, rather than combining that snapshot with a racing second health read.

- **Starting:** the first bounded registration attempts are pending. Logical commands already work. Connection setup and native synchronization are distinct.
- **WaitingForObserver:** the session-bus receiver is registered, but no synchronized observer snapshot exists. No windows or current associations are fabricated.
- **Synchronized:** both the logical hierarchy read and the verified native context checkpoint succeeded. `context` contains the exact epoch, native position and policy revision used. Registration alone never enters this state.
- **Recovering:** a previously usable observation is unavailable, a new Hello awaits a snapshot, or a receiver is reconnecting after earlier synchronization. Current association reads fail until recovery.
- **Degraded:** receiver registration failed/exhausted, the receiver name is occupied, the database cannot be read, or context/worker availability failed. Reason codes distinguish these conditions.
- **Stopping:** cancellation is announced immediately, current context metadata is removed, new API requests are rejected and queued operations are abandoned when cancellation is observed.
- **Stopped:** normal task cleanup completed and the task/connection have been joined/closed. No application service is available through that controller. A receiver cleanup error is reported explicitly rather than silently relabeled successful cleanup.

Nonterminal working states can transition among Starting, WaitingForObserver, Synchronized, Recovering and Degraded as current inputs change, and can enter Stopping. Stopping normally progresses only to Stopped; abnormal worker interruption can produce Degraded/WorkerLost. Stopped cannot restart. A new controller instance is required after worker/process loss. `Lifecycle::allows` encodes the transition boundary and is tested. Late asynchronous update processing cannot publish a working state over an announced Stopping state.

`receiver_connected` distinguishes the bus connection from `observation`; a registered connection with no observer stays unavailable. A database failure can leave independently synchronized native observations available, while context decisions remain unavailable and lifecycle is Degraded. Fresh snapshot/report calls validate current data, not a previously displayed status.

## Bounded event processing

Controller commands use a 16-entry Tokio channel. Submission uses `try_send`; saturation returns Busy and never silently retries. Logical strings are capped at 4096 bytes, reorder/assignment sets at 4096 entries, and scope/identity strings at bounded lengths before enqueueing. Existing domain validation still applies. Logical hierarchy memory scales with the operator's database; these queue bounds are not an absolute process RSS guarantee.

The controller subscribes to the actual observer before its first snapshot read. Native updates use the adapter's coalescing watch; each wakeup re-reads the accepted state and refreshes context continuity. There is no polling of individual windows, raw-event history, per-geometry full-state serialization or growing notification queue. Full owned snapshot clones remain part of the accepted API, and context projection is refreshed as needed. Association collections are materialized only on an explicit association request.

The select loop is cancellation-aware and fair among setup, updates and requests. Timers exist only for receiver setup/retry, request deadlines and cleanup; no idle controller polling timer was added. No SQLite or state lock is held across an asynchronous D-Bus operation. The actor owns its SQLite connection and runs short typed logical operations; it accepts no arbitrary user callback or shell command.

## Receiver and context recovery

Default receiver registration allows five attempts. Each connection attempt has a two-second async deadline. Failed attempts back off 100, 200, 400 and 800 ms, with a two-second per-delay cap. The configured default attempt deadlines and delays sum to approximately 11.5 seconds; scheduling or a running synchronous SQLite operation can extend wall time. Immediate absent-socket failures exhaust much sooner. Options have bounded ranges. Logical commands continue while setup/retries await I/O. RetryExhausted remains explicit until `retry_receiver()` is requested or the controller is restarted.

A service-name collision is terminal for automatic registration attempts. The receiver uses its established DoNotQueue request and never replaces the Python inspector or another Rust receiver. The foreground CLI reports the conflict and exits after cleaning up only its own controller. The library can retain logical CRUD in Degraded state and accept an explicit retry later.

Absent KWin on an available bus is not a receiver failure: the registered receiver waits without re-registering. KWin owner loss/replacement, new observer sessions, re-Hello, heartbeat expiry and sequence/revision gaps are handled by the accepted adapter. Context decisions become unavailable and old claims are invalidated according to its epoch/generation evidence. Coalesced removal/recreation is still detected; unrelated continuously live windows retain assignments during proven healthy updates.

A closed session-bus connection detaches the read handle, discards the coordinator, and starts a fresh bounded registration cycle. Retry exhaustion is visible; an explicit retry can register after the private/session endpoint returns. No assignment is replayed during transport recovery. `reconstruct_context()` explicitly discards all overrides and creates a fresh coordinator epoch even if native state is otherwise continuous. Old CAS tokens fail against that fresh epoch.

Hierarchy query failures set DatabaseUnavailable and invalidate context. Fixing availability allows subsequent reads/updates to recover, without starting a second database or restarting KWin. There is no automatic database recreation/failover. A database that cannot be securely opened prevents controller startup; the CLI reports a fixed error. External logical changes are discovered on the next read, request or observer update; no filesystem watcher was added.

## Public API contract

`Controller::start(SqliteStore, ControllerOptions)` requires a Tokio runtime and moves the secure existing store into the owned task. Cloneable handles expose:

- `health()` for a freshly checked checkpoint; `subscribe()` for coalesced health/version summaries; `last_health()` for the latest/terminal summary.
- `hierarchy()` and typed `logical(LogicalCommand)` covering the existing world/workspace/desktop CRUD, move/reorder and scoped-list operations.
- `snapshot()` for the current synchronized native snapshot, and `associations(Selection)` for verified context reports and unassigned/conflicted/advisory views.
- `assign`, `suppress` and `clear_override`, using the existing exact WindowKey and expected Version. Explicit suppression remains distinct from removing an override.
- `retry_receiver`, `reconstruct_context`, and idempotent normal `shutdown`.

No raw verified-launch claim producer is exposed. A compile-fail test protects the lack of native `move_window` control. Inputs are typed local calls, not serialized dispatch from a socket or another process.

Status watch values are checkpoints and may coalesce. Clone borrowed watch values before awaiting; retaining a watch borrow across an await can block publication. They are not leases on current native state; obtain a fresh snapshot/report before consuming ownership decisions. Returned snapshots/reports are internally consistent historical values. Synchronization can change immediately after a read; no atomic transaction with KWin or an external SQLite writer is claimed. Logical CRUD continues during observation outages, while assignments require the current synchronized epoch, live generation, observation position and policy revision.

Requests have a six-second default response deadline. If a queued caller has disappeared, mutation handlers skip it before starting work. If a deadline races an already-started logical/assignment operation, OutcomeUnknown means it may have completed. Do not retry a mutation automatically. Queue saturation, stopped worker, invalid bounds, context rejection, storage failure and cleanup failure have distinct errors with fixed display messages.

## Shutdown, interruption and limits

Shutdown sets an out-of-band cancellation watch, immediately announces Stopping, prevents new API work, cancels pending registration, discards context and detaches observations. Receiver shutdown has a two-second cleanup deadline. The controller joins its task with an eight-second deadline and a further bounded one-second abort wait. A cleanup error/deadline is returned explicitly. Concurrent/repeated normal shutdown is safe. Dropping the last controller handle also closes cancellation/command ownership; the actor finishes cleanup and its status channel closes.

The existing SQLite busy timeout is five seconds. A running synchronous SQLite call is not interruptible by dropping its async caller. Kernel/filesystem stalls can exceed normal busy-timeout behavior; no promise is made that Tokio can forcibly terminate synchronous OS work. The library returns bounded shutdown errors where possible, rejects further API work, and leaves a timed-out operation's outcome explicit. The CLI also bounds runtime teardown to two seconds after controller completion/error. Applications embedding the library must handle cleanup errors and their runtime/process lifetime deliberately.

A Drop guard detaches observations and publishes WorkerLost if the worker is aborted or panics. Current snapshot/report requests then fail. Tests interrupt a synchronized worker with a live assignment, verify service release, construct a new controller and prove the assignment is gone. A new controller process or coordinator never restores ephemeral assignments from app ID, PID, native UUID or saved IDs.

SIGKILL cannot execute Rust cleanup or publish Stopped. The process test verifies OS D-Bus disconnection releases the name and the same temporary SQLite database remains readable/integral on restart. This does not promise user-space cleanup after an uncatchable signal, power loss or arbitrary kernel/filesystem failure. Production controller code has no child process to orphan; test daemons and CLI children have explicit kill/wait guards.

## Foreground CLI

```sh
cargo run --locked -p depthos-core --bin depthos -- controller run
cargo run --locked -p depthos-core --bin depthos -- --db /trusted/private/workspaces.db controller run
# Optional bounded diagnostic session:
cargo run --locked -p depthos-core --bin depthos -- controller run --timeout-seconds 30
```

This command runs only when explicitly invoked, opens the existing secure logical database path, and emits a sequence of pretty-printed JSON health objects (not JSON Lines). It does not install/load an observer or effect. Without an observer it waits, and a failed bus remains Degraded with bounded retries. The library retains logical commands in that state; this CLI intentionally provides status only, not a misleading one-shot assignment session or an unrestricted remote API.

Ctrl+C, SIGTERM or the optional 1–300 second run window initiate owned cleanup and print the final normal status. Conflict and startup/output/cleanup failures exit 1. CLI diagnostics include only lifecycle/position/reason information. They do not serialize window collections, titles, app IDs, payloads, runtime window IDs or logical workspace paths. Geometry-only position updates do not print another status line when lifecycle/reason/availability are unchanged.

## Validation and process evidence

All **78 regular Rust checks pass**: the accepted 64 checks, four controller unit tests, nine controller integration tests and one new compile-fail control-boundary check. One additional release benchmark is ignored by default and was explicitly executed successfully. Existing policy, Python/Rust parity, private receiver and unified CLI regressions pass unchanged.

```sh
cargo build --locked
cargo test --locked
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
python3 -B -m unittest discover -s experiments/context-policy -v
python3 tests/observer-adapter/parity.py
python3 tests/observer-adapter/private-bus.py
python3 tests/observer-adapter/unified-cli.py
python3 tests/lifecycle/processes.py
cargo test --release --locked --test controller_lifecycle synthetic_1000_window_latency -- --ignored --nocapture
python3 tests/lifecycle/verify-source.py
```

Coverage includes start without KWin; later observer startup and verified sync; live assignment, suppression and clearing; healthy same-epoch updates; owner disconnection/replacement; heartbeat, sequence and revision loss; runtime removal/recreation under coalesced notifications; stale CAS rejection; coordinator reconstruction; failed startup and bounded retry; real Python receiver collision; session-bus loss/rebind; database failure/repair; graceful/cancelled/forced shutdown; last-handle Drop; repeated cycles; bounded queue behavior; redaction; and no native mutation API.

Temporary database byte comparisons prove observation and context operations add no SQL writes. Explicit logical CRUD is tested during outages. Repeated service reacquisition and closed watch streams verify owned task cleanup. The private CLI test covers SIGTERM, SIGKILL/restart, integrity checks, name release, and no controller child processes. A deliberately stalled private Unix handshake verifies cancellation before the five-second connection timeout. All D-Bus mutation tests use spawned private daemons or fresh `dbus-run-session`, never the desktop session bus.

The 16 original Python policy scenarios and seven protocol-parity scenarios / 30 packet comparisons remain passing. These tests are not native compositor acceptance.

## Synthetic performance

Measurements and commands are recorded in `tests/lifecycle/performance.json`. The release library fixture uses 1,000 sanitized live windows and 100 geometry events, timing from sender publication through a fresh controller health checkpoint at the expected sequence. Startup timing begins after secure store open. Recovery timing begins with the replacement snapshot after a forced gap. Packet construction is outside individual event timing.

The separate CLI fixture measures process startup through WaitingForObserver, sync/re-Hello processing for 1,000 windows, `/proc` process RSS/high-water memory, and a two-second idle CPU-tick interval. It uses the normal development binary and includes secure SQLite open/schema checking and CLI/runtime startup against the fixture database initialized earlier in the process test. Process-memory numbers are current/high-water observations, not a proven memory ceiling or leak bound. A zero CPU-tick delta means below the sample's scheduling/tick resolution, not literally zero work.

Optimized library fixture: startup 2.093 ms after store open; initial synchronization 32.126 ms; event median 4.954 ms, p95 5.367 ms and maximum 5.490 ms across 100 events; gap-recovery synchronization 29.822 ms.

Development CLI fixture: startup to waiting 6.568 ms; initial 1,000-window synchronization 97.816 ms; re-Hello recovery 97.076 ms. RSS was 13,020 KiB while waiting and 20,192 KiB with 1,000 windows; observed high-water memory was 20,608 KiB. The 2.003-second idle sample included 1 valid heartbeat and measured 0.50% of one core at kernel CPU-tick resolution, with no semantic window updates.

No result above measures host KWin overhead, GPU work, frame pacing or FPS. These are single-run local synthetic/controller observations, not latency guarantees. Idle controller work remains event-driven; the receiver's established liveness monitor still checks the bus. Full snapshot copying/projection costs remain visible in event latency and can be optimized later without adding another reducer or weakening identity validation.

## Preservation, rollback and next phase

The pre-edit audit found no active source writer; repeated source inventories matched before and after backup. The verified 136-file source backup is:

`/home/ashenix/Projects/DepthOS-backups/phase-2.4-20261116T140836Z/`

It excludes Git metadata, private evidence/screenshots/recordings, build artifacts, caches, logs/runtime files and credential material. Source-copy verification is point-in-time content evidence, not a permanent writer lock or atomic filesystem snapshot.

This phase changes only the root library export/CLI, the coordinator's metadata-only synchronization entry, the receiver's connection-status accessor, and newly added lifecycle implementation/tests/report/audits. Domain/service/SQLite code, schema/migrations, native producer/effects, existing tests, Cargo manifests/lockfile, previous reports/manifests and backups remain unchanged. `tests/lifecycle/manifest.json` and its verifier check the allowed delta and protected source. Earlier manifests remain frozen historical snapshots; they may report intentional later evolution rather than being rewritten to hide it. Selected `kwinrc`, `kglobalshortcutsrc`, `kactivitymanagerdrc` and `kwinrulesrc` hashes match the recorded baseline; no KDE mutation command was executed.

For rollback, first stop the foreground controller normally. Compare current changed-file hashes with the lifecycle manifest and preserve/review any later changes. Restore only `src/lib.rs`, `src/main.rs`, `src/context/mod.rs` and `crates/depthos-observer/src/transport.rs` from the scoped backup. Remove only matching newly added lifecycle files. Do not reset/clean Git or copy an entire older tree over newer work. No KDE rollback is required. No commit, merge, rebase or push was created.

The controller is ready for a separately scoped Phase 2.5 integration discussion with a spatial UI or further reviewed local interfaces. Durable live assignments, automatic routing, window/focus/desktop control, verified production launch claims, AI, restoration and autostart remain absent. Phase 2.5 has not started.
