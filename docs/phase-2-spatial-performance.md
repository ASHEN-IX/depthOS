# Phase 2.5P — Spatial UI performance

**PHASE 2.5P ACCEPTED** for the isolated Phase 2.5 read-only integration on KWin 6.8.3. The throughput and latency numbers below are debug-build observations on a private bus with a software-rendered Qt Quick client.

## Source and scope

The requested directory was absent and the source repository's `main` had no commits. The implementation uses a separate linked worktree at `/home/ashenix/Projects/DepthOS-performance` on the unborn branch `codex/spatial-performance`. The starting tree matched all 156 hashes in the Phase 2.5 source manifest. No commit was created.

The controller still evaluates the existing context policy against the existing database and observer. The D-Bus service, interface, methods, signatures, same-user checks, 64-row pages, 4,096-row collection limits and 128 KiB response cap retain their Phase 2.5 behavior. The UI continues to use asynchronous calls. Selection, keyboard and mouse behavior, reduced motion, native thumbnails and Z0–Z3 remain owned by the KWin overview. The write-authorization boundary remains as documented in [Phase 2.5](phase-2-spatial-integration.md).

## Profile before changes

A disposable copy of Phase 2.5 ran the 301-row, 1,000-window benchmark with one loaded logical panel. Probes emitted stage names, durations, timestamps and counts. They captured no titles, UUIDs, paths from user data or window metadata. The patched zbus trace and Qt scene probe ran only in the disposable copy. Production runs used the locked dependencies with probes disabled.

| Stage | Baseline observation |
| --- | --- |
| Rust full observation snapshot construction | 1.61 ms median, 3.88 ms max, 282 copies in the diagnostic run |
| Policy context projection | 1.14 ms median, 4.01 ms max, 279 projections |
| Complete bridge refresh | 16.45 ms median, 108.79 ms max, 119 refreshes |
| Page JSON encoding | 0.50 ms median, 2.19 ms max |
| zbus reply assembly and wire encoding | 0.089 ms median, 0.156 ms max before socket send |
| Signal send and delivery | 0.179 ms median to emit through zbus. For the initial full observation and identity-removal signals, the emitter-to-Python callback interval was 4.48 ms and 2.20 ms, including the fixture event pump. |
| QML decode, validation and page merge | 0–1 ms per stage at the clock's 1 ms resolution |
| Serial pagination | Median 30 ms each for hierarchy and for context calls in the instrumented QML run. A full 1,000-window context takes 16 calls. |
| Model replacement | 1 ms for hierarchy and 1–2 ms for context plus badge construction |
| Delegate creation | 1 ms for the one visible World delegate when no World was selected |
| Scene update probe | 14 ms from context replacement to an offscreen panel image callback, which includes render scheduling and readback |

[Raw Rust profile](../artifacts/p25p/profile/rust-profile.log) · [Raw QML profile](../artifacts/p25p/profile/qml-profile.log) · [Signal receipt timestamps](../artifacts/p25p/profile/signal-receive.json)

The dominant cost was the serial page pipeline. Each page re-entered the bridge refresh, which issued separate hierarchy and health commands. The controller synchronized both sides of each command, making full observation copies and constructing policy projections at the same position repeatedly. Report refreshes could incur additional copies. JavaScript parsing, array construction, delegate initialization and scene work did not explain the delay observed in this fixture.

## Changes

- A bridge read uses one controller command. The hierarchy, reported health, policy decisions and UUID lookup use a consistent hierarchy read and one synchronized observer snapshot. A cache hit at the exact context version keeps the existing redacted rows. Every page still revalidates against a current read before accepting its revision.
- Duplicate UUID detection still counts every observed window, including windows excluded from reporting. A protected duplicate revokes an already cached native badge without exposing a protected row. The integration test also checks that the previous page revision is rejected and the unaffected window key retains its generation.
- The page serializer writes the borrowed, redacted row slice directly into the JSON envelope. This removes intermediate JSON values while preserving the wire fields and bounds.
- The QML client binds pagination to the resolved unique owner, retains capabilities for that instance and coalesces the requested scope bits across canceled refreshes. Native badges are withdrawn on context invalidation. The client publishes a context model only after the last page has passed validation. A context refresh keeps the existing hierarchy model and delegates. An unchanged hierarchy revision keeps its model object. Failure and owner changes use full synchronization for recovery.
- The existing ListViews already virtualize the hierarchy. The profile did not justify keyed row edits, association deltas, a larger page contract or changes to native delegate creation. Changed context still uses complete pages. Geometry packets can carry whole-window changes, so cache reuse depends on the existing version and equality checks. Thirty geometry-only events produced zero semantic invalidations in every measured run.

## Performance

The historical Phase 2.5 result was 52.21 ms median for a complete hierarchy, 1,459.60 ms for the initial context refresh and 1,050.72 ms for removal. The comparison below uses fresh before and after runs on the same machine with the original benchmark workload. The repeat driver added only raw hierarchy timing output to the original fixture. Runs alternated before and after. The extra size and lifecycle matrix ran separately against the optimized worktree.

All triplets are median / p95 / maximum. There are 50 timed hierarchy reads and five samples for each other metric per configuration. The nearest-rank p95 is the maximum for a five-sample series.

| Metric | Phase 2.5 rerun | Phase 2.5P |
| --- | ---: | ---: |
| Full paged hierarchy fetch (ms) | 31.63 / 44.41 / 53.53 | 10.45 / 13.30 / 14.43 |
| Initial snapshot to complete QML model (ms) | 987.20 / 1083.69 / 1083.69 | 419.70 / 447.73 / 447.73 |
| Initial snapshot to signal dispatch in fixture (ms) | 96.31 / 126.08 / 126.08 | 96.18 / 99.29 / 99.29 |
| Native identity removal to complete QML model (ms) | 983.25 / 1010.31 / 1010.31 | 439.83 / 453.89 / 453.89 |
| Publish 30 geometry events (ms) | 135.57 / 140.66 / 140.66 | 101.20 / 113.74 / 113.74 |
| Controller RSS (KiB) | 25304.00 / 25828.00 / 25828.00 | 20912.00 / 24052.00 / 24052.00 |
| QML process RSS (KiB) | 46800.00 / 47112.00 / 47112.00 | 46512.00 / 46936.00 / 46936.00 |
| Controller idle CPU (% of one core) | 5.32 / 6.32 / 6.32 | 2.00 / 2.33 / 2.33 |
| QML idle CPU (% of one core) | 0.67 / 1.00 / 1.00 | 0.67 / 1.00 / 1.00 |

The median initial context refresh fell 57%, removal fell 55%, and the full hierarchy read fell 67% across these repeats.

| Optimized scenario | Hierarchy fetch (ms) | Initial model (ms) | Identity removal (ms) | Publish 30 geometry events (ms) |
| --- | ---: | ---: | ---: | ---: |
| 0 rows, 100 windows | 1.03 / 1.80 / 1.97 | 218.75 / 221.95 / 221.95 | 213.42 / 245.23 / 245.23 | 79.92 / 278.21 / 278.21 |
| 301 rows, 100 windows | 15.15 / 30.59 / 43.08 | 200.91 / 285.41 / 285.41 | 183.63 / 209.30 / 209.30 | 101.85 / 280.95 / 280.95 |
| 1,501 rows, 40 windows | 162.06 / 248.11 / 311.96 | 593.76 / 1038.11 / 1038.11 | 192.03 / 209.67 / 209.67 | 133.31 / 178.49 / 178.49 |

| Optimized scenario | Controller RSS (KiB) | QML RSS (KiB) | Controller idle CPU (%) | QML idle CPU (%) |
| --- | ---: | ---: | ---: | ---: |
| 0 rows, 100 windows | 14944.00 / 15176.00 / 15176.00 | 44600.00 / 44676.00 / 44676.00 | 0.67 / 0.99 / 0.99 | 0.67 / 1.00 / 1.00 |
| 301 rows, 100 windows | 15192.00 / 15228.00 / 15228.00 | 45192.00 / 45212.00 / 45212.00 | 1.00 / 1.65 / 1.65 | 0.66 / 1.00 / 1.00 |
| 1,501 rows, 40 windows | 16664.00 / 17036.00 / 17036.00 | 46144.00 / 46328.00 / 46328.00 | 2.33 / 2.98 / 2.98 | 0.67 / 0.67 / 0.67 |

| Lifecycle at 301 rows, 100 windows | Median / p95 / maximum (ms) |
| --- | ---: |
| Workspace rename | 444.62 / 541.00 / 541.00 |
| Workspace and dependent desktop deletion | 500.96 / 504.31 / 504.31 |
| Sequence gap to unavailable context | 201.82 / 252.31 / 252.31 |
| Recovery snapshot to complete context | 200.01 / 249.28 / 249.28 |
| Controller termination to cleared UI | 48.51 / 49.82 / 49.82 |
| Controller spawn to new instance and recovered hierarchy | 252.39 / 253.12 / 253.12 |

The first set of repeats overlapped verification and showed higher tails, including 864 ms for optimized removal. The alternating repeats above ran after verification completed. Some of the additional size-matrix runs overlapped local build and profiling work. The large hierarchy still has a roughly one-second initial-load maximum in that matrix. [Raw results and summary](../artifacts/p25p/measurements.json)

Initial and removal timings run from submission in the Python producer to the QML timer observing the complete model. Rename/delete timings start before the trusted CLI operation. Restart begins at process spawn and ends when a new instance has loaded the persistent hierarchy. Idle CPU is process time over about three seconds with observer heartbeats. Peak allocations, frame pacing and physical desktop latency were outside these measurements.

## Regression evidence

- Locked Rust build and test suite, formatting and strict all-target Clippy passed. Rust reported 81 passes and one intentionally ignored benchmark. The Rust suite includes stale revisions, bounds, unavailable and restart behavior, write-method absence, protected redaction and the warm-cache duplicate check.
- Python/Rust parity passed seven scenarios and 30 packet comparisons. Private-bus interoperability, unified CLI and controller process lifecycle passed.
- QML model decoding, client lifecycle and schema rejection passed. The offscreen panel and overview suites passed 4 and 18 Qt checks. Package and native safety suites passed 7 and 6 checks.
- The isolated native spatial suite passed 21 assertions, with matching settled host snapshots and no surviving private processes. It checked native labels, selection, read-only external edits, thumbnails, backend and observer outages, restart, removal and Escape. [Native spatial assertions](../artifacts/p25/run-20261116T154308Z-e7f5ff/assertions.json)
- The existing Wayland suite passed 72 assertions after the optimization, with matching host state and window inventory and clean process/package teardown. The benchmark worktree path exceeded the original Unix socket path limit during baseline preflight. A short disposable source copy then passed all 72 baseline assertions and cleaned up. Its instantaneous host `activeEffects` comparison differed, while its host window inventory matched. The final run used a shorter socket name and a content-addressed test package derived from the production sources and probe. The spatial suite also used a content-addressed test package in its private data root. [Final Wayland assertions](../artifacts/poc-02g/run-20261116T154319Z-dfa156/assertions.json) · [Host comparison](../artifacts/poc-02g/run-20261116T154319Z-dfa156/host-comparison.json)

## Source manifest and rollback

| Path | Change |
| --- | --- |
| `src/context/mod.rs` | Reuse context synchronization and report evaluation with captured inputs. |
| `src/controller.rs` | Serve coherent spatial reads through the existing controller actor. |
| `src/spatial.rs` | Reuse redacted rows and serialize pages directly. |
| `experiments/kwin-overview/contents/ui/ControllerClient.qml` | Refresh requested scopes, retain hierarchy model identity and preserve full recovery. |
| `tests/integration/poc02e/runner.py` | Address the private overview package by its content and shorten the socket name. |
| `tests/spatial/performance.py` | Extend the private-bus fixture with size and lifecycle measurements. |
| `tests/spatial_bridge.rs` | Assert UUID refusal and pagination fencing after a protected duplicate arrives. |
| `docs/phase-2-spatial-performance.md` | Record results, source manifest and rollback. |

The [SHA-256 manifest](../artifacts/p25p/source-manifest.json) lists the pre-change and final source hashes. Matching Phase 2.5 copies of each modified source file are under `artifacts/p25p/rollback/` in the performance worktree. To roll back, stop any foreground controller started for this worktree, verify those copies against the manifest, and restore the seven modified paths individually. Remove this report if reverting the entire phase. The isolated launchers already cleaned up the private packages and processes. The original source tree remains available independently.
