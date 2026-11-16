# Controller shutdown publication race

Validated against canonical `main`, baseline `69d549b5927ab52e625ec9df7d39265d8b9443f6`. The Phase 3.1 readiness run reproduced the bug in the stalled-bus handshake test. After `shutdown().await` and the elapsed-time check both succeeded, the controller reported `Stopping` where the test required `Stopped`.

## Lifecycle invariants

| State | Meaning |
| --- | --- |
| `Starting` | The worker is starting or making an initial receiver connection attempt. |
| `WaitingForObserver` | A receiver exists and is waiting for usable observations during initial synchronization. |
| `Synchronized` | The worker has a valid context version derived from available storage and a synchronized observation. |
| `Recovering` | Observation continuity was lost or the worker is reconnecting after previously synchronizing. |
| `Degraded` | A required dependency is unavailable, retry policy prevents progress, or the worker was lost. Inspect the recovery reason to distinguish recoverable conditions from `WorkerLost`. |
| `Stopping` | Cancellation has been announced or worker teardown is underway. Published context and database availability are invalidated. |
| `Stopped` | Normal worker teardown concluded. The terminal reason records either clean `Shutdown` or `CleanupFailed`. |

While active, the worker may move among the five operational states as receiver, observation and database conditions change. Cancellation may move any active state to `Stopping`. Once that is published, an operational refresh cannot replace it. The worker can update stopping details and then publish `Stopped`. An unexpectedly terminated worker can publish `Degraded / WorkerLost` instead. The handle cannot replace that failure with `Stopping`.

A published `Stopped` value is absorbing for this controller instance. The caller owning the join reports success only after a successful join and a stored `Stopped / Shutdown`. Other shutdown callers can finish on the same stored terminal outcome. `Stopped / CleanupFailed` returns `CleanupFailed`, and unexpected worker loss returns `WorkerLost`. The join deadline and its `ShutdownTimeout` outcome are unchanged. Watch receivers may skip intermediate values. A delayed or repeated shutdown cannot change the terminal value after it is visible.

## Root cause and publication ordering

The handle signalled cancellation, borrowed and cloned the current health, then released the watch read lock before publishing a modified clone. The worker could publish `Stopping`, abandon the pending connection and finish teardown with `Stopped` during that gap. The handle then wrote its stale clone as `Stopping`. The join completed, and the old cleanup check returned success because it only looked for `CleanupFailed`. The same stale write could erase the cleanup failure reason. The worker’s regular health publication also checked and wrote under separate watch operations, so a precomputed refresh could overwrite a concurrent stopping announcement.

The handle now calls `send_if_modified` with `ControllerHealth::mark_stopping`. Tokio holds the watch write lock across the state check and update. If the worker publishes first, the handle sees the terminal value and leaves it in place. If the handle publishes first, the worker may complete the transition to `Stopped`. The worker uses the same watch operation for its transition check, equality check and write. A late normal refresh cannot replace `Stopping`, and a write cannot replace `Stopped`.

## Production changes

- `src/controller/mod.rs` shares the stopping-field update between the local `last_health()` projection and the watch publication. It takes the one-use sender out of its mutex before publishing. A repeated stop announcement leaves an existing `Stopping`, `Stopped` or `WorkerLost` outcome intact.
- Shutdown reads the stored outcome, requires `Stopped / Shutdown` for success, preserves `CleanupFailed` and reports an absent or unexpected terminal outcome as `WorkerLost`. Non-joining callers wait for either `Stopped` or `WorkerLost`.
- `src/controller/worker.rs` applies its existing cancellation and duplicate-publication checks under the watch write lock, with an explicit `Stopped` guard. The actor remains the owner of terminal cleanup publication. Existing cancellation, join and receiver-cleanup deadlines still apply.

## Deterministic regression

`delayed_shutdown_announcement_preserves_terminal_outcome` uses a unit-test-only gate just before the handle publishes. The first shutdown pauses at that gate. A second caller takes the join handle and is polled to pending. A controlled worker then publishes `Stopped`, and the test waits for a watch subscriber to observe it before releasing the first caller. For both clean and failed cleanup, the test checks the full observed health value, both concurrent results, a subsequent shutdown result and the final watch value. The existing actual-worker interruption test also checks that repeated shutdown preserves `Degraded / WorkerLost`.

For the negative control, the same regression ran with the old clone-and-replace logic and the gate between its snapshot and write. The test build released the sender mutex after taking the sender so the second caller could acquire the join handle while the first was paused. It failed deterministically when the delayed writer replaced the already observed `Stopped` with `Stopping`. Moving the gate to the entrance of the atomic operation gives the worker the same publication order while forcing the handle to decide from the current value. The existing stalled-handshake integration test and its elapsed-time assertion were left intact.

## Remaining concurrency risks and rollback

This change addresses the publication order. Watch notifications still coalesce. The secondary caller can see the terminal health before the join owner has resumed. Canceling the future that owns the taken join handle still follows the existing Tokio detachment behavior, so other callers rely on worker publication and their existing deadline. The stalled-handshake integration test synchronizes on an attempted connection, which does not establish that a specific handshake byte was exchanged. The deterministic regression complements repeated test runs. Those runs do not exhaust possible schedules.

To roll back, revert only this change’s hunks in `src/controller/mod.rs`, `src/controller/worker.rs` and `src/controller/tests.rs`, and remove this report if appropriate. Preserve any unrelated changes and the three pending cleanup documentation files. The earlier post-join race is possible again after rollback.

## Validation results

| Check | Result |
| --- | --- |
| Deterministic regression with old publication | Failed with exit 101. Observed `Stopped` was replaced by `Stopping`. |
| Deterministic regression with atomic publication | Passed for clean and failed cleanup. |
| Original stalled-handshake cancellation case | 100 of 100 focused runs passed. The one-second assertion was unchanged. |
| Locked offline workspace suite, default parallelism | Five consecutive runs passed. Each passed 112 tests and ignored one explicit benchmark. |
| Locked offline workspace build | Passed. |
| `cargo fmt --all --check` | Passed. |
| Locked offline workspace Clippy, all targets, warnings denied | Passed. |
| Git integrity and whitespace | Strict full fsck and diff whitespace checks passed. The index stayed empty. |

Cargo used a temporary build target outside the repository. Exact command outcomes, the pre-fix failure trace and full test results are under the owner-only ignored directory `artifacts/controller-shutdown-race.IaA38e/`. The pending README, maintenance guide and Phase 2 report matched their preflight SHA-256 digests.
