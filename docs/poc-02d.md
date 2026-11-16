# POC-02D — Final physical interaction and Phase 1 acceptance

Date: **2026-11-16, Africa/Tunis**. New run: `20261116T000352Z-46b4b4cf`.

## Verdict

**PHASE 1 PARTIALLY ACCEPTED.** Do not begin POC-03. Physical Escape delivery and safe dismissal are verified. The designated terminal did **not** record `depthos-02d-ok`; normal typing/mouse recovery therefore remains a critical PENDING gate. Later physical keyboard, mouse, native depth-navigation and reduced-motion groups were not started without that explicit first-group result.

Current local regressions: **73 passed, zero failed**. Current native checks establish real startup/delegate/rendering, Escape delivery, safe fullscreen refusal/reset and successful later explicit activation. All **12 final cleanup comparisons pass**, including exact `kwinrc`/`kglobalshortcutsrc` hashes and preserved custom/POC-01 package bytes. No test-owned process remains. This closes one physical dismissal gate, not all Phase 1 interaction requirements.

The previous POC-02C 71-local/23-observer/120-cycle results are historical context; they were not re-counted as current physical acceptance. No Rust orchestration, routing, SQLite, AI or other product phase was implemented.

## Pre-flight and regression

The seven requested reports and existing harness were reviewed. The prior run was marked cleaned with all 12 predicates true. It and earlier failed evidence remain intact. Git remains untracked as found; no commit/push/reset occurred. [Current baseline](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/baseline.json) records owner `:1.9`, bus ID, KWin PID 1701, KWin 6.8.1 Wayland, four existing desktops, one Activity, eDP-2 at 1920×1080/scale 1/~144 Hz, effects, action names, package inventory, configuration hashes and relevant saved tiling values. Original POC-01 remained installed and initially unloaded.

All required suites were rerun:

```fish
python3 tests/validate-poc.py
fish scripts/validate-observer.fish
fish scripts/validate-overview.fish
python3 -m unittest discover -s tests/integration -p test_live_acceptance.py
```

**19 JavaScript + 11 replica + 7 package + 18 overview QtTest + 10 POC-01 QtTest + 8 harness tests = 73.** Qt totals include init/cleanup. The initial unchanged suites gave 71; the two new receiver/recovery tests brought the final total to 73. Qt engine, child-context/cache, private D-Bus authorization, fish syntax and Python parsing also pass and are not invented extra named cases. Local logs are under `artifacts/poc-02d-local/`.

No production QML changed, so the unchanged immutable revision **`depthos-overview-dev-c0d9409bfce8`** (source hash `c0d9409bfce8afd08c9a805cdc94dfd7ed4bc393dde3888c223ea6b602792d79`) was reused without overwriting cached URLs. No additional 120-cycle stress run was needed. The existing observer acceptance record still matches the same compositor owner/bus/source; this run also verified actual initial snapshot delivery. The previous full 23-check native observer suite is not claimed as rerun.

## Controlled setup and genuine physical evidence

The independent guard is pinned to this new run and checks session identity. The POC-02D watchdog uses the same scoped cleanup path, including safe known-empty tiling recovery if required. Target clients require the recorded native UUID/PID/application triple. Screenshots require a test-only native context. No native desktop-control tool was available; the computer-use skill's operator-guided fallback was used. No X11 input injection or unrestricted virtual input device was created.

Two disposable Konsole windows on Desktop 3 ran `tests/integration/marker-terminal.py`. It is **not a shell**: it compares input only to the exact marker, never evaluates it, never writes non-marker content and exits after a bounded 900 seconds. Its acknowledgement records only the fixed marker, a boolean and timestamp. The unit test covers matching input, extra/command-like text rejection and no non-marker output retention.

Before asking for input, KWin reported the effect active and created a 1920×1080 output delegate. The [actual inspected screenshot](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/02d-escape-start.png) shows Z1 with two real test Konsole cards and native previews. Requested visible=true alone was not accepted as proof.

The first operator group asked only: press Escape; click a test Konsole; type `depthos-02d-ok`; press Enter; report whether “INPUT VERIFIED” appeared. The reply was **“dine”**. It was preserved literally and not interpreted as proof of marker receipt.

The [native interaction log](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/interaction.log) records **`key=16777216 level=1` at 01:06:15 local**, then exit completion and `visible request=false`. With the initial active/delegate/screenshot evidence and no synthetic input source, this verifies prompted physical Escape dismissal. It does not establish ordinary typing or pointer recovery.

Both acknowledgement files remained absent. A short clarification directed the operator to Desktop 3 and the exact marker. No answer to that clarification was available before the two fixed-lifetime input programs exited. Their windows existed during the bounded attempt; no marker acknowledgement was received. The [operator result](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/operator-result.json) records this missing result. **Typing recovery is PENDING**, rather than a presumed pass or an attributed product defect. The exact terminal target/action could not be confirmed.

An attempted terminal screenshot was skipped when native context returned to a desktop containing a non-test client; no private screenshot was saved. Remaining guided groups were kept paused to avoid interrupting marker entry. Physical arrow/Enter/Space, minus/Backspace/plus/equals, Z2/Z3 dismissal, hover/card/desktop/Activity selection, visible zoom/close/backdrop controls and reduced-motion UI enable/disable therefore remain **PENDING**. Earlier programmatic POC-02C screenshots are retained in that report, not substituted for these physical tests.

## Transition-time refusal and feedback

After the bounded operator attempt ended, an independent check reused the existing DepthOS-only contention test. [Current result](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/02d-contention.json): POC-01 alone held fullscreen input; the overview was refused; its missing-delegate guard reset the unfulfilled request; after POC-01 closed, one explicit invocation opened the overview. Neither an unrelated fullscreen effect nor KWin/Plasma was forcibly stopped. Original POC-01 was subsequently unloaded to its baseline state.

The title-free [scoped log](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/scoped-kwin.log) contains the fixed activation-unavailable/reset diagnostic. No matching QML TypeError, ReferenceError, assignment, binding-loop or view-construction error was captured. The guard is configured for 1000 ms; logs/state establish recovery, not precise frame timing or universal protection against GPU faults.

**Classification:** refusal while another fullscreen effect owns input is an accepted bounded transient backend limitation when it leaves the current owner intact, resets the request and allows a later explicit invocation. It is not evidence of a stuck overlay in this run. No automatic indefinite retry or focus stealing is used. User-facing clarity remains limited: no on-screen unavailable notification is implemented when no view can start; the fixed diagnostic is logged. The operator did not validate clarity, so that usability gate remains pending. No critical reproducible stuck-input defect was established; critical physical input restoration is still unverified.

## Current performance evidence and limits

An inactive-overview baseline collected five two-second `/proc/PID/stat` and VmRSS samples with the foreground context disclosed in [idle-sample-context.json](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/idle-sample-context.json). KWin averaged **4.80% of one CPU core**, RSS **419,708 KiB**; the inspector averaged **0.20%**, RSS **30,892 KiB**. This is a working-desktop process sample, not causal DepthOS overhead or a certified quiet workload.

A later nominal empty-test-desktop sample is also preserved in [performance.json](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/performance.json), but its workspace context was not stable enough to support an empty/quiet workload conclusion. The subsequent real-preview capture was **aborted by the privacy guard** when a non-test application was present on the selected desktop. No preview screenshot or accepted two-preview CPU sample was obtained; [performance-abort.json](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/performance-abort.json) records the failure. The attempted profile was stopped rather than relabeled as successful or repeatedly taking private captures.

Current [instrumentation inspection](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/02d-instrumentation.json): `intel_gpu_top` and `perf` unavailable, tracefs unreadable, no presentation-frame collector used. No invasive utility or privileged service was installed. **Frame timestamps, FPS, p95/p99 frame times, dropped/late frames, Intel GPU utilization and a quiet preview performance profile remain PENDING.** Refresh rate and animation callbacks were not used to infer FPS. Earlier POC-02C resource/stress values remain historical context only.

## Changes and regression coverage

Production effect/observer sources are unchanged. Added `tests/integration/marker-terminal.py` and `tests/integration/poc02d.py` for small operator groups, explicit observations, an exact-marker receiver, a run-pinned watchdog and scoped cleanup. Added two harness regressions: exact marker/no arbitrary-input retention, and recovery refusing any change that cannot reproduce the original whole-file hash for a known-empty test tile entry.

The optional tile recovery verifies the saved empty layout, current expected tree, no windows on the test desktop or in its tree, single known output and exact baseline hash. It uses public empty-tree removal, never overwrites/reset configurations and never flattens an unknown/user-owned layout. **It was not needed in this run:** the first final cleanup already matched all hashes. Its native precedent remains in the retained POC-02C evidence; current coverage for the new recovery planner is local regression coverage.

There was no confirmed new product defect requiring a QML fix. Privacy/context failures in the capture setup were preserved and the affected measurement was stopped. No production feature, new global key binding or orchestration architecture was introduced.

## Final cleanup

[cleanup.json](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/cleanup.json) has **12 PASS, zero errors** on the first final cleanup attempt. The overview and observer packages are removed, all owned scripts unloaded, receivers stopped, original and later diagnostic test clients closed, and the guard exited. Original effects/actions/packages and custom-effect bytes match the baseline. Desktops, Activities and output state are restored. Exact `kwinrc` and `kglobalshortcutsrc` hashes match, including relevant tiling state. [Original application verification](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/original-applications-after.json) finds the original native UUIDs with unchanged desktop memberships. No test-owned PID remains and no remaining recovery command is required.

The old runs, their failed evidence and this run's [manifest](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/manifest.json), [commands](../artifacts/live-acceptance/20261116T000352Z-46b4b4cf/commands.jsonl), snapshots, privacy-abort record and scoped logs are retained. No old or cleaned manifest was reused for live mutation.

## Acceptance matrix

| ID | Case | Result | Evidence / limitation |
| --- | --- | --- | --- |
| A1 | Fresh baseline, prior cleanup, session identity | **PASS** | New run; prior run remains cleaned and unchanged |
| A2 | Independent bounded recovery guard | **PASS** | Run-pinned watchdog with cleanup and known-empty tiling recovery |
| A3 | Local regression | **PASS** | 73 named checks, zero failures |
| A4 | Native initial observer snapshot | **PASS** | Current redacted native snapshot received; historical 23-check suite not relabeled as current |
| A5 | Real active effect and output delegate before operator request | **PASS** | 1920×1080 delegate, native active state and inspected screenshot |
| B1 | Physical Escape from Z1 | **PASS** | Prompted native key 16777216 at 01:06:15 local; completed exit and visible=false |
| B2 | Normal typing reaches designated test program | **PENDING** | No depthos-02d-ok acknowledgement before fixed 900-second program lifetime ended |
| B3 | Ordinary mouse interaction after dismissal | **PENDING** | No explicit confirmed result in designated terminal |
| B4 | Physical arrows | **PENDING** | Later guided group not started |
| B5 | Physical Enter/Space card selection | **PENDING** | Later guided group not started |
| B6 | Physical minus/Backspace/plus/equals | **PENDING** | Later guided group not started |
| B7 | Physical dismissal from Z2 and Z3 | **PENDING** | Later guided group not started |
| C1 | Hover and window-card click | **PENDING** | No controlled operator result |
| C2 | Desktop and Activity card clicks | **PENDING** | No controlled operator result |
| C3 | Visible zoom-control clicks | **PENDING** | No controlled operator result |
| C4 | Close-button and backdrop dismissal | **PENDING** | No controlled operator result |
| C5 | Selection avoids actual window activation/movement | **PENDING** | Production code remains read-only for selection; physical path not exercised |
| D1 | Reduced-motion UI enable/disable | **PENDING** | No operator click result; prior programmatic evidence retained separately |
| D2 | Input release in both motion modes | **PENDING** | Critical designated-terminal acknowledgement absent |
| E1 | Yield to competing DepthOS fullscreen effect | **PASS** | Only POC-01 remained active during rejected overview attempt |
| E2 | Bounded refused-request reset | **PASS** | Existing 1000 ms missing-delegate guard logged actual reset |
| E3 | Later explicit invocation succeeds | **PASS** | One invocation after POC-01 closed opened the overview |
| E4 | User-perceived unavailable feedback | **PENDING** | Fixed diagnostic message exists; no on-screen unavailable notification is implemented or operator clarity result obtained |
| F1 | Current inactive-overview CPU/RSS sample | **PASS** | Five two-second samples, context disclosed |
| F2 | Quiet real-preview performance sample | **PENDING** | Privacy guard aborted capture when non-test client was on selected desktop |
| F3 | Presentation timestamps / FPS / p95 / p99 | **PENDING** | No validated collector available; no inference from callbacks or refresh rate |
| F4 | New 120-cycle stress run | **NOT APPLICABLE** | No production QML change or failure requiring repetition |
| G1 | All test components, processes and actions removed | **PASS** | Final cleanup and no recorded live PID |
| G2 | Exact configuration and preserved-effect hashes | **PASS** | Both config hashes and preserved package bytes match baseline |
| G3 | Desktops/Activities/output state restored | **PASS** | Final native comparison |
| G4 | Original application memberships preserved | **PASS** | Native UUIDs still exist with identical desktop memberships |
| G5 | Tiling layout preserved | **PASS** | Exact baseline hash on first cleanup; guarded repair was available but not needed |

Current report matrix counts: {'PASS': 15, 'PENDING': 16, 'NOT APPLICABLE': 1}. These overlap with local and historical categories and are not independent benchmark repetitions.

## Resume and Phase 2 readiness

This bounded attempt is now cleaned. A later physical test must start a **fresh baseline and guard**; the old terminal and cleaned run cannot accept a late marker or authorize further mutation. Ask for one small group at a time and require its observed result before proceeding. The first missing result is normal input reaching the designated controlled terminal with `depthos-02d-ok`; then complete keyboard/depth groups, mouse controls and reduced motion through the real UI. Do not infer these from fixtures, direct scene calls or an inactive flag.

Reproduction entry points (run individually and inspect each result): the overview loader requires genuine observer acceptance for the same source, compositor owner and bus. If that record is no longer valid, complete the documented native observer prerequisite in its own guarded run and clean it before starting a fresh POC-02D baseline; do not fabricate a PASS record.

```fish
python3 tests/integration/live-acceptance.py baseline
# Independent process/terminal:
python3 tests/integration/poc02d.py guard
# Main control terminal:
python3 tests/integration/poc02d.py setup
python3 tests/integration/poc02d.py open --group escape
# Operator presses Escape and enters the marker, then gives an explicit result.
python3 tests/integration/poc02d.py observe --group escape
# Stop/rollback at any time; no acceptance record required for cleanup:
python3 tests/integration/poc02d.py cleanup
```

No operator question or physical result is treated as answered by elapsed time. The fixed-lifetime programs ended without the required acknowledgement; the relevant gates remain pending. **Phase 2/POC-03 is not ready to begin.** Finish critical physical interaction/input recovery and retain the explicitly bounded performance/refusal limitations before foundation sign-off.
