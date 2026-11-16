# DepthOS Phase 3.0 + Phase 2.5P integration readiness

**INTEGRATION ACCEPTED** for the offline authorization model and isolated read-only spatial integration on KWin 6.8.3.

The canonical checkout contains the Phase 3.0 offline authorization model and the Phase 2.5P spatial bridge. Production authorization remains disabled. The controller exposes its original read-only D-Bus interface, and KWin continues to render and manage native windows. Phase 3.1 has not begun.

## Source provenance

Canonical `main`, security `codex/phase-3-security` and performance `codex/spatial-performance` share the same Git metadata and have unborn HEADs. Both track manifests identified the same 157-file baseline, which matched canonical byte for byte. The security completion manifest and performance final manifest matched their respective trees. The three-way review found disjoint edits and no deletions. Content differences were recorded before integration. Two inventory passes and open-file checks found no source changes or writable source descriptors during the audit.

The [transfer manifest](../artifacts/integration-20261116T155406Z-eI87/transfer-manifest.json) records the audited hashes, source ownership and intended writes. The [content review](../artifacts/integration-20261116T155406Z-eI87/review.diff) records the deltas. The [Phase 2.5 baseline](phase-2-spatial-integration.md), [offline authorization report](phase-3-security.md) and [spatial performance report](phase-2-spatial-performance.md) remain available as historical reports. Their original evidence stays in the respective worktrees. Historical isolation verifiers refer to their original phase boundaries.

| Owner | Paths integrated |
| --- | --- |
| Phase 3.0 authorization | `src/lib.rs`, `src/action_security.rs`, `src/action_security/tests.rs`, `tests/security_boundary.rs` |
| Phase 3.0 provenance | `docs/phase-3-security.md`, `tests/security/isolation.json`, `tests/security/completion.json`, `tests/security/verify-isolation.py` |
| Phase 2.5P bridge and UI | `src/context/mod.rs`, `src/controller.rs`, `src/spatial.rs`, `experiments/kwin-overview/contents/ui/ControllerClient.qml` |
| Phase 2.5P regression and provenance | `tests/integration/poc02e/runner.py`, `tests/spatial/performance.py`, `tests/spatial_bridge.rs`, `docs/phase-2-spatial-performance.md` |
| Integration | `.gitignore`, `docs/phase-3-integration-readiness.md` |

Both `Cargo.toml` and `Cargo.lock` matched the shared baseline. The security module uses the same `WindowKey` and `Version` definitions as the bridge. Its library export is the only production reference outside the module.

## Security boundary

- `Consent` has private fields and no public constructor or deserializer. `verify_interaction_claim` always returns `UnverifiedInteraction`. The compile-fail check verifies that an external safe Rust caller cannot construct the type. Proof issuance in the model tests is confined to test compilation.
- The offline state machine checks exact window identity and generation, native UUID uniqueness, compositor incarnation, observer and coordinator continuity, operation scope, source/destination and eligibility. The proof and capability are one-use. Replay, stale versions and ambiguous identities fail closed. Uncertain execution retains the conservative outcome and supports observation-only reconciliation without automatic retry.
- The private denial fixture sends six unsupported claims from two same-UID connections, including the private compositor name owner. All six are refused. The production controller's version 1 interface still advertises `readOnly=true` and `assignmentMutations=false`. Window text, caller UID and ordinary QML callbacks supply no production proof path. Synthetic execution mutates only the model's in-memory world. No native window setter was added.
- Spatial bridge tests check epoch and generation continuity, pagination fences, controller recovery and redaction. The warm-cache duplicate UUID case withdraws the mapping even when the duplicate is a protected window and leaves that protected row undisclosed.

## Regression results

| Check | Result |
| --- | --- |
| Locked Rust build and tests | 111 passed, one explicit benchmark ignored by the default suite |
| Formatting and strict all-target Clippy | Passed |
| Offline authorization and private-bus denial | 28 model tests, one compile-fail consent test in the full suite, and one denial test covering six claims |
| Python context policy, observer replica, and integration safety | 16 context policy, 11 replica, 21 integration safety/version, 8 host policy, 6 native safety and 4 recovery checks |
| Observer JS and Python/Rust parity | 19 JS tests, 7 parity scenarios and 30 packet comparisons |
| Private-bus interoperability and controller lifecycle | Passed Python-to-Rust interoperability, unified CLI, process lifecycle and recovery |
| Spatial client and model | Passed QML production client lifecycle, version mismatch and decoder checks. Offscreen panel 4, overview 18, original POC 10 and package policy 7 passed. |
| Isolated KWin 6.8.3 spatial suite | 21 assertions passed with a content-addressed overview package and exact host comparison. [Spatial assertions](../artifacts/p25/run-20261116T155459Z-d60f9c/assertions.json) |
| Isolated POC-02G Wayland suite | 72 assertions passed, including native navigation, thumbnails, protected window handling and EIS input. [Wayland assertions](../artifacts/poc-02g/run-20261116T155514Z-e69ae8/assertions.json) |
| Isolated watchdog fault injection | Recovered a deliberately stalled worker, stopped private input, unloaded its private effect, removed its package and left no processes. [Watchdog cleanup](../artifacts/poc-02g/run-20261116T155753Z-ace2f0/cleanup.json) |

[Command logs and evidence](../artifacts/integration-20261116T155406Z-eI87/)

## Performance comparison

Five alternating runs of the original 301-row, 1,000-window fixture compared canonical with the accepted performance worktree. The driver collected the raw hierarchy timings without changing the timed workload. Tests used locked debug Rust builds, a private D-Bus daemon and Qt 6.12 software offscreen rendering. The hierarchy series has 50 samples per configuration. The other metrics have five samples each, so nearest-rank p95 equals maximum for those rows.

| Metric | Accepted Phase 2.5P | Fresh Phase 2.5P control | Integrated |
| --- | ---: | ---: | ---: |
| Full paged hierarchy read (ms) | 10.45 / 13.30 / 14.43 | 9.04 / 13.21 / 16.56 | 9.44 / 13.10 / 14.30 |
| Initial snapshot to QML (ms) | 419.70 / 447.73 / 447.73 | 430.25 / 459.80 / 459.80 | 422.01 / 430.10 / 430.10 |
| Native identity removal to QML (ms) | 439.83 / 453.89 / 453.89 | 445.31 / 467.12 / 467.12 | 436.22 / 471.18 / 471.18 |
| Initial snapshot to invalidation (ms) | 96.18 / 99.29 / 99.29 | 96.14 / 103.91 / 103.91 | 96.72 / 106.10 / 106.10 |
| Publish 30 geometry events (ms) | 101.20 / 113.74 / 113.74 | 117.63 / 140.45 / 140.45 | 117.61 / 121.79 / 121.79 |
| Controller RSS (KiB) | 20912.00 / 24052.00 / 24052.00 | 20940.00 / 23600.00 / 23600.00 | 21116.00 / 24116.00 / 24116.00 |
| QML RSS (KiB) | 46512.00 / 46936.00 / 46936.00 | 46548.00 / 46768.00 / 46768.00 | 46476.00 / 46788.00 / 46788.00 |
| Controller idle CPU (% of one core) | 2.00 / 2.33 / 2.33 | 2.00 / 2.66 / 2.66 | 2.00 / 2.33 / 2.33 |
| QML idle CPU (% of one core) | 0.67 / 1.00 / 1.00 | 0.67 / 0.67 / 0.67 | 0.67 / 1.00 / 1.00 |

The integrated medians were 422.01 ms for initial model load and 436.22 ms for identity removal, against 419.70 ms and 439.83 ms in the accepted results. The same-session control measured 430.25 ms and 445.31 ms. The integrated removal maximum was 471.18 ms versus 467.12 ms in the same-session control. Burst publish medians were effectively identical in the fresh comparison, and every burst caused zero semantic invalidations. These repeats show no material regression in the measured spatial path. [Raw repeats and summary](../artifacts/integration-20261116T155406Z-eI87/benchmark-summary.json)

These timings measure observed model and signal updates in the fixture. They do not measure compositor FPS or physical input latency.

## Host-state verification

The final spatial, Wayland and watchdog runs matched their own host snapshots. The Wayland and watchdog runs also matched the original host window inventory. The settled host snapshot matched across the integration, including the KWin owner/PID, effects, desktops, Activity, outputs, shortcuts, installed packages and four monitored KDE configuration hashes. All input, installation, effect calls and cleanup that change compositor state used disposable processes and the private runtime and bus. [Integration comparison](../artifacts/integration-20261116T155406Z-eI87/host-comparison.json) · [Wayland comparison](../artifacts/poc-02g/run-20261116T155514Z-e69ae8/host-comparison.json) · [Watchdog comparison](../artifacts/poc-02g/run-20261116T155753Z-ace2f0/host-comparison.json)

The first two watchdog attempts completed private recovery and detected a different host current desktop in their instantaneous comparisons. The host window inventory matched both times. No host restoration was performed. The third attempt matched the host snapshot and window inventory. [First attempt](../artifacts/poc-02g/run-20261116T155628Z-f829d5/host-comparison.json) · [Second attempt](../artifacts/poc-02g/run-20261116T155706Z-cd4ae0/host-comparison.json)

## Phase 3.1 prerequisites

A separate review must qualify an exact KWin build and establish a protected user-input path, including authority and script integrity, seat routing, lock-screen and focus behavior, and policy for portal, accessibility and injected input. A private dry-run authority must prove an atomic compositor-owned check/consume turn tied to the live object and exact operation. Race, replay, loss-of-acknowledgement and uncertain-outcome checks must pass before a separately authorized executor milestone. See the [Phase 3.0 plan](phase-3-security.md#concrete-phase-31-plan--separate-approval-required).

## Initial Git baseline and rollback

The reviewed set contains 166 source paths. The [initial-commit file list](../artifacts/integration-20261116T155406Z-eI87/initial-commit-files.txt) and [SHA-256 inventory](../artifacts/integration-20261116T155406Z-eI87/initial-commit-manifest.json) identify the reviewed source set. Historical source manifests and synthetic fixture data are included as provenance. Private captures and recordings, runtime state, build output, caches, local credentials and Git metadata are outside the candidate set. The candidate files were reviewed for private-key material, common service token formats, credential-bearing URLs, binary captures and generated files. No credential matches were found. The root ignore rules now cover local env files, common key and database files, logs and recordings in addition to the existing artifact and build exclusions.

No files were staged or committed. With separate authorization, verify the listed hashes and stage exactly the reviewed paths, then inspect the staged inventory and diff before committing. Run the following from canonical after approval and review:

```sh
git add --pathspec-from-file=artifacts/integration-20261116T155406Z-eI87/initial-commit-files.txt
git diff --cached --stat
git diff --cached --check
git commit -m "Establish verified DepthOS baseline"
```

After the initial commit on canonical `main`, use a new path for each ordinary linked worktree, for example `git worktree add -b codex/<task> ../DepthOS-<task> main`. Retain the two unborn track worktrees and their evidence until their contents are independently accounted for.

The private source backup at `/home/ashenix/Projects/DepthOS-backups/phase-3-integration-2sicio89` contains 157 hash-verified baseline files. To roll back, compare current hashes against the transfer and initial-commit manifests, then restore only the nine replaced paths from the backup and remove the nine integration additions listed in the manifests if they still match. Keep any later edits and historical evidence. The test suites manage their own private processes and packages.
