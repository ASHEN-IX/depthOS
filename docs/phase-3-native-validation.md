# Phase 3.6 native authority validation

Verdict: **NATIVE AUTHORITY REQUIRES COMPOSITOR CHANGES**. Production remains deny-only and read-only. The standalone C++ kernel supplies synthetic consistency evidence; the separate native KWin fixture supplies deny-only live-window/display evidence. No native protected approval, real placement mutation or persistent assignment mutation is qualified.

## Baseline and environment

The isolated worktree is `/home/ashenix/Projects/DepthOS-phase-3-6`, branch `codex/phase-3-6`, accepted base `f73226097aced5f414dc1d1a901debb3eab510fc`. Canonical main remains `6543b65213f24a733012ac5db50e69ed382d1cfb`; accepted Phase 3 is unchanged. Root's ignored `artifacts/phase-3-6/preflight/baseline.json` records clean-tree, ancestry, integrity and available concurrent-worker checks. No other Phase 3 worker was observed through app-chat state and `/proc` working directories; this is not a proof that no external writer exists.

The machine is Garuda Linux `7.3.0-4-garuda`, x86-64, Intel Core i5-12450H, 12 logical CPUs, Rust/Cargo 1.100.0, KWin 6.8.3-1 and Qt 6.12.0. Builds use the locked local dependency cache. The installed KWin executable SHA-256 is `33350d420b7b196c1e1f90ffa8c1280b4b3102d4e4b60e1929bad525be93df5e`; ELF build ID is `6eb58cf1f8d875ef625487ec2f404e8cc969c517`. Source/build qualifications and their limits are in the feasibility and independent security reports.

Raw logs, source archives, package metadata, host snapshots and performance samples remain under ignored owner-only `artifacts/phase-3-6/`. No host snapshots or window metadata are committed.

## Commands and baseline results

These checks passed before the implementation handoff, against the unchanged production source in the campaign worktree:

- `cargo build --workspace --locked --offline`: exit 0.
- `cargo test --workspace --locked --offline`: exit 0, 128 passed and one explicit performance benchmark ignored; includes production bridge introspection, same-UID refusal and controller lifecycle.
- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: exit 0.
- `cargo test --locked --offline --example phase3-native-preview`: exit 0, three strict display-projection/refusal tests.
- `cargo test --locked --offline --test controller_lifecycle synthetic_1000_window_latency -- --ignored --nocapture`: exit 0, 100 synthetic events, median 15.28 ms, p95 26.21 ms, maximum 27.56 ms. This initial benchmark overlapped other checks and is not a controlled regression comparison.
- `env QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software /usr/lib/qt6/bin/qmltestrunner -input tests/phase-3`: exit 0, eight QtTest results including setup and teardown.
- `python3 tests/observer-adapter/private-bus.py`, `python3 tests/observer-adapter/parity.py`, and `node --test tests/spatial/model.cjs`: exit 0.
- `python3 tests/phase-3-native/run.py`: exit 0, native deny-only tracker/preview assertions, eight private Wayland QML results, monitored host pre/post equality and owned process/socket/XDG cleanup.

Additional broad checks passed: `python3 tests/observer-adapter/unified-cli.py`, `python3 tests/spatial/client-smoke.py`, `python3 tests/spatial/version-mismatch.py`, the offscreen `tests/spatial/tst_panel.qml` suite on a private bus, and `dbus-run-session -- fish scripts/validate-overview.fish` and `validate-observer.fish`. The default Cargo artifact inventory contained the normal library and executable; the symbol audit found the refusal method and no fixture authority, confirmation marker, counting sink or execution module.

## Comparable offscreen performance

The reviewer extracted exact accepted commit `f73226097aced5f414dc1d1a901debb3eab510fc` using `git archive` into ignored `artifacts/phase-3-6/control/` and independently built it with `cargo build --manifest-path artifacts/phase-3-6/control/Cargo.toml --workspace --locked --offline` (exit 0). Five alternating control/current runs used the established 301-row, 1,000-window fixture. Each command ran with inherited display and session-bus variables removed:

```sh
python3 tests/spatial/performance.py --output <absolute-evidence-path>
```

All ten commands exited 0. Percentiles use nearest rank; with five samples p95 equals maximum. Values below give median / p95 / maximum and sample count.

- Initial context to QML, ms: control **420.99 / 452.25 / 452.25**, current **407.06 / 427.13 / 427.13**; n=5 per variant.
- Native identity removal to QML, ms: control **426.26 / 463.29 / 463.29**, current **414.41 / 443.29 / 443.29**; n=5 per variant.
- Full paged hierarchy fetch, ms: control **8.51 / 10.20 / 13.68**, current **8.35 / 10.20 / 11.00**; n=50 per variant.
- Controller RSS, KiB: control **20,980 / 24,064 / 24,064**, current **20,916 / 24,020 / 24,020**; n=5 per variant.
- Idle controller CPU, percent of one core: control **1.66 / 2.33 / 2.33**, current **2.00 / 2.00 / 2.00**; n=5 per variant.

Each run observed zero invalidations for its 30-event geometry-only burst. These are debug Rust/private D-Bus/production QML client timings with offscreen software rendering and synthetic observations. Ordinary workstation load and development continued. The production controller/spatial sources are unchanged, and these distributions do not establish a causal improvement or regression. Native compositor latency, native protected input latency and FPS remain unmeasured. Raw samples and aggregation are in `review/{baseline,final}-[1-5].json`, `performance-commands.tsv` and `performance-summary.json`.

## Host comparison and unsupported native cases

The reviewer audited the launcher before running it. It uses a private D-Bus session, private Wayland socket, virtual output, isolated HOME/XDG roots and only owned surfaces. The private compositor has `--no-lockscreen`; native lock enforcement and seat ownership are not tested. No experimental component is installed or loaded in host KWin.

The initial campaign native run passed the monitored read-only host pre/post comparison and verified private process-group, socket and temporary-root removal. Host native window inventory and focus remain unobserved. Earlier Phase 3 current-desktop and effects mismatches remain documented findings; they are not waived by subsequent equality. Pre/post equality cannot establish continuous host-state equality.

Native EIS/virtual, remote/portal and accessibility provenance remains unsupported. Unknown input is denied because the only runtime authority is deny-only; absence of a positive path does not establish that a future protected approval path is safe. Guarded synthetic QObjects are not KWin windows. Synthetic lock, seat, prompt and coordinator facts are test-owned assertions and do not authenticate their native equivalents.

## Integrated review results

Agent 1 handed off `145cfde8a75755470f42424220684acc3991e0c8`. Agent 2 corrected synthetic deadline overflow and QObject thread affinity, added explicit cancellation/restoration tests, and tightened private launcher identification. The [security review](phase-3-native-security-review.md) records pre-fix reproductions and scope. The standalone C++ tests remain isolated and the no-test-macro build contains no positive kernel symbols.

- Final `cargo build --workspace --locked --offline`, `cargo test --workspace --locked --offline` (128 passed, one ignored), `cargo fmt --all --check`, and `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` exited 0.
- Final `cargo test --locked --offline --test spatial_bridge --test security_boundary` exited 0. It exercises the production read-only method allowlist and denial through the private same-UID fixture. The workspace suite includes the deterministic controller terminal-publication regression and synthetic confirmation/cancellation ordering test.
- Final `python3 tests/phase-3-native/authority-kernel.py` exited 0, including the deadline, affinity, cancellation/restoration, single-use and uncertain receipt checks. The independent UBSan/thread-affinity controls failed on the handoff and passed after correction.
- A launch identity control verified the pinned installed build and rejected a deliberately wrong expected digest. The final private fixture checks the root-owned installed executable’s hash and file identity before and after startup, the private D-Bus owner’s PID, the runtime version and the private socket. Direct child executable inspection was denied by the host; descriptor-based launch failed to load the Qt platform plugin. The final evidence is an installed-path qualification, not process-image attestation.
- Final `python3 tests/phase-3-native/run.py` exited 0. The exact-object preview, focus/window/removal/expiry/topology cases and eight QtTest results passed. The sampled host fields matched, and the private processes, socket and temporary roots were removed. A preceding run on the same launcher passed private assertions but returned exit 1 because host `effects.activeEffects` changed. This difference remains a finding; no host restoration occurred and the comparator was not weakened. The samples do not identify the cause.
- A fault control provided `/usr/bin/false` instead of the display projection example within the private worker. It failed the expected private step, the launcher drained the group, the Wayland socket and temporary roots were removed, and sampled host fields matched. The command and log are in `review/native-failure-cleanup.{py,log}`.
- `git diff --check f73226097aced5f414dc1d1a901debb3eab510fc` passed. Canonical main and the accepted Phase 3 checkout remained clean at their exact starting commits. Strict shared-repository `git fsck --full --strict` passed at preflight (two dangling trees). After the desktop session resumed, it returned 10 due to one empty app checkpoint reference under `refs/codex/turn-diffs/checkpoints/`. No campaign or accepted branch was identified by that error. The app reference was left untouched; strict validation of the campaign and accepted/main histories uses a separate bundle without the app checkpoint namespace. See local `review/fsck-final.log` and `review/fsck-bundle.log`.
