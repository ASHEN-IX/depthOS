# POC-02C — Native interaction, performance and stability acceptance

Date: **2026-11-16, Africa/Tunis**. New run: `20261115T231959Z-4bb81da1`. Repository: `/home/ashenix/Projects/DepthOS`.

## Engineering verdict

**PHASE 1 PARTIALLY ACCEPTED.** Hold POC-03. Native discovery, actual level rendering, programmatic reduced motion, bounded stress and exact cleanup are substantially verified. Critical physical keyboard/mouse navigation and ordinary typing restoration remain pending because the user explicitly deferred the operator checklist. Presented-frame timing and complete uninterrupted preview-count performance profiles are also missing. No project ownership, routing, Rust service, database or AI feature was added.

Final local regression result: **71 passed, 0 failed**. Native observer: **23 checks passed**. Final stress: **120 completed cycles in 123 attempts**, with **3 declined/interrupted attempts** retained, plus **60 rapid toggles**, **10 double reversals**, five disposable create/focus/remove cases and receiver/observer recovery. Stress-stage duration: **195.575 seconds**. This is bounded repeated use, not long-duration production certification.

The activation experiment passed 4 immediate-load and 4 settled-load cases; the first desktop-transition case was refused, so it stopped at **8 successful activations / 9 trials**. The matrix deliberately records that expectation as FAIL rather than silently converting all native trials to PASS. The new guard recovered a refused request; unconditional activation during another fullscreen transition is not promised. The current report matrix is {'PASS': 32, 'PENDING': 20, 'FAIL': 1, 'NOT APPLICABLE': 2}. These gates and the 71 local tests overlap with historical categories; do not add them to the previous 67/57/104 totals as independent repetitions.

## Audit, authorization and isolation

The seven requested reports were reviewed. The earlier directory `artifacts/live-acceptance/20261115T224144Z-5a370ed1/` and its cleanup record exist and remain intact. Git status was untracked at entry; no reset, commit or push was performed. KWin remained 6.8.1 Wayland, PID **1701**, bus owner **`:1.9`**, with four existing desktops and one existing Activity. eDP-2 is 1920×1080, scale 1, approximately 143.999 Hz; the compositor uses Intel Mesa. NVIDIA presence does not make its counters a measure of this compositor.

The [immutable baseline](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/baseline.json) records effects, outputs, desktops/Activities, package inventory, action names, session identity, preserved files and configuration hashes. The run has a separate watchdog, fixed run directory and recorded PID/start-tick identities. Guards now verify **bus ID + owner + compositor PID**, so a reused `:1.x` owner on another bus cannot authorize a stale run. A cleaned manifest refuses further live stages and repeated cleanup makes no resource mutations. Configuration hashes are part of cleanup success, not merely advisory.

Only recorded disposable Konsole clients receive native property writes. Their UUID, PID and `org.kde.konsole` identity must all agree. Screenshots are restricted to test-only desktop content and honor capture exclusion. No X11 input injection, unrestricted uinput device, sudo, service installation, compositor/Plasma restart, logout, cache deletion, desktop-count change or Activity creation/deletion occurred. Diagnostic data was not sent to an external service. Existing app UUIDs and desktop memberships still match the initial native discovery after cleanup.

## Regression and code changes

Executed local suites:

```fish
python3 tests/validate-poc.py
fish scripts/validate-observer.fish
fish scripts/validate-overview.fish
python3 -m unittest discover -s tests/integration -p test_live_acceptance.py
```

Counts: **19 observer JavaScript + 11 replica + 7 overview package + 18 overview QtTest + 10 POC-01 QtTest + 6 harness unit = 71**. Qt totals include init/cleanup. QJSEngine, child-context/cache probes, isolated D-Bus authorization/reconciliation, all fish script syntax and Python parsing also pass; these are not invented extra named test counts. Logs are under `artifacts/poc-02c-local/`. The final overview Qt run took 3.67 s / 78,860 KiB peak RSS on software/mock boundaries; it is not a compositor measurement.

Production changes are limited to `experiments/kwin-overview/contents/ui/main.qml` (startup guard and title-free lifecycle state), `OverviewScene.qml` (handled navigation-key/selection traces), and `WindowCard.qml` (hover/click traces). Ordinary character keys are not logged. No arbitrary window caption or application contents enter these diagnostics. The final production revision is **`depthos-overview-dev-c0d9409bfce8`**, hash `c0d9409bfce8afd08c9a805cdc94dfd7ed4bc393dde3888c223ea6b602792d79`. Changed QML was always prepared at a new content-addressed URL.

Test changes: `tests/overview/tst_overview.qml` adds refused-start recovery/no-auto-retry coverage; `tests/integration/live-acceptance.py` tightens session/cleanup checks and captures only relevant tiling preferences for future rollback; `test_live_acceptance.py` covers session reuse, cleaned-run refusal and tiling snapshot isolation; new `tests/integration/poc02c.py` supplies bounded stages and isolated diagnostic scene controls. These are test facilities, not a product controller or orchestration architecture.

## Activation anomaly: demonstrated failure class and repair

The first current-run attempt logged initialization, shortcut callback, `visible request=true`, and transition completion, but **no output delegate** and **no active effect**. [First failure log](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/activation-first-failure.log). KWin's version-matched [SceneEffect wrapper](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/scriptedquicksceneeffect.cpp) stores the visible request separately. [QuickSceneEffect startup](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/effect/quickeffect.cpp) can return before creating views when another fullscreen effect is active or a keyboard grab fails. Thus visible=true is not startup confirmation, and the old toggle could leave a stale invisible open request.

The smallest fix tracks constructed output delegates and arms a **1000 ms one-shot** guard. If an open request still has no constructed output, it resets requestedOpen/visible, stops the transition and records a fixed unavailable message. It never auto-retries or disables another effect. The fixture verifies reset and a later explicit successful open with a delegate. This is a missing-delegate guard, not a texture/GPU-health monitor; excessively delayed incubation can be cancelled by the bound.

[Controlled contention](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/contention.json) held only the preserved POC-01 fullscreen effect active. The overview was correctly refused, the guard reset the request, and one explicit invocation after POC-01 closed opened the overview. No two DepthOS fullscreen effects were simultaneously active. POC-01's original package/action were preserved and restored.

[Reload trials](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/activation-attempts.json) used unchanged QML at the same immutable revision: 4 immediate and 4 settled loads succeeded. A desktop-transition trial was refused (8/9 overall); the remaining three planned transition trials were not run after that first refusal. This corroborates a startup-contention/timing failure class but does **not** establish whether a fullscreen effect or keyboard grab caused the exact historical incident. The old incident's precise trigger remains unconfirmed. No matching scoped QML TypeError/ReferenceError/binding-loop errors were captured.

## Native interaction and rendering: what was actually exercised

The computer-use skill was consulted. Native desktop APIs are disabled in this session; no suitable Wayland input controller is installed. X11 `xdotool` was not used. The operator was given exact keys and a disposable terminal that recognizes only `depthos-02c-ok`, without executing a shell command or retaining arbitrary input. The user replied: **“we will check them later; continue working for now.”** No marker acknowledgement exists. Escape traces during unrelated live activity are recorded, but are not substituted for the deferred structured keyboard/mouse and typing-restoration test.

To finish the independently testable rendering/state work, an **isolated immutable copy** added run-only unbound diagnostic actions that call the production `zoomOut`, `zoomIn`, `inspectDesktop`, `inspectActivity`, `close` and `open` functions and toggle the existing reducedMotion flag. Its source bytes and action list are recorded in [render-probe-source.json](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/render-probe-source.json). Its ID is `depthos-02c-render-probe-4bb81da1-20c67b`. It uses real native Workspace/window/filter/thumbnail objects. It does **not** synthesize Qt key events, inject hardware input, modify the production architecture or provide physical-input acceptance.

Actual inspected compositor captures establish:

- [Z1](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/02c-native-z1.png): six real Konsole cards with rendered terminal textures.
- [Z2](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/02c-native-z2.png): four native desktop cards and the active desktop marker.
- [Z3](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/02c-native-z3.png): one actual Activity ID, without an invented name.
- [Inactive inspection](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/02c-inactive-inspection.png): actual Desktop 3 remains active while Desktop 4 is inspected; the moved disposable client is metadata-only.
- [Reduced motion](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/02c-reduced-motion.png): local Motion: reduced state, selection distinct from the native active desktop.

[Native probe states](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/render-probe-states.json) record real root level, visible/reveal state, desktop/Activity identities, motionOff and transition duration. Ten programmatic reduced-motion open/close cycles checked native active/inactive state after 50 ms observation waits. Following calls reached **transitionDuration=0**, reveal=1, motionOff=true. Those waits are not presentation-frame latency. Twenty programmatic Z1/Z2/Z3 round trips were issued; native inactive-state assertions also completed after closes from each level. These are direct-function state/rendering gates; key mapping, pointer selection, the UI button click, physical dismissal and ordinary input restoration remain pending.

Kirigami's actual public global duration was **300 ms** and was read without changing permanent preferences. The zero-global-duration native scenario was not forced. Its source/fixture support is distinct from live validation of that preference case.

## Bounded stability and resource evidence

The final [stability record](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/stability.json) completed 120 cycles across 123 attempts in 195.575 s, 60 rapid toggles, 10 double reversals, five new/focus/close cases, observer restart and receiver replacement while the overview was active. Final state was inactive. The three declined/interrupted attempts remain in [stability-attempts.json](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/stability-attempts.json). The earlier run stopped after at least 60 completed cycles at an activation timeout; its [progress](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/stability-first-interrupted.json) and [log](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/stability-first-attempt.log) are retained. Alt/Escape activity and native desktop changes correlate with interruptions; they do not prove the exact cause of every rejection.

KWin RSS across the 20→120 completed-cycle cohort samples was 430,328→430,864 KiB, a **536 KiB increase**; cohort values fluctuate. End-of-stress RSS was 431,576 KiB after disposable/recovery cases. This does not establish a leak or prove its absence. The last five two-second samples after render-probe unload were 399,988 KiB, below the earlier warm baseline sample. Compiled QML caches, working-desktop activity and allocation lifetimes make cross-stage attribution unreliable.

### Resource measurement limits

The first preview-count attempt put extra clients on the previous desktop: its “six” screenshot showed only two. It is explicitly invalid and retained under `profile-attempt-1-invalid-placement/`. Placement/membership checks were corrected. Longer attempts were interrupted by dismissal or native context changes; their raw samples remain but are excluded from clean preview-count conclusions. The isolated probe's longer profile also stopped at the first invalid context. No invalid sample is converted to PASS.

A short follow-up collected five **200 ms** samples per count, checking active effect and native context at the endpoints. Only counts **1, 2 and 4** passed those checks. CPU is `/proc/PID/stat` user+system tick delta / monotonic elapsed / SC_CLK_TCK, percent of one CPU core. RSS is VmRSS. With HZ=100, each 200 ms CPU reading is quantized in **5 percentage-point steps**; even a five-sample mean is coarse. The terminals repaint at roughly 5 Hz. The scene is an instrumented copy with reduced motion enabled; this is not a randomized, controlled or causal overhead benchmark.

| Endpoint-valid preview count | KWin mean CPU, one core | KWin RSS KiB | Inspector mean CPU |
| --- | ---: | ---: | ---: |
| 1 | approximately 3% | 461,360–461,360 | approximately 0% |
| 2 | approximately 5% | 450,404–450,404 | approximately 0% |
| 4 | approximately 5% | 428,220–428,220 | approximately 0% |

The non-monotonic RSS numbers must not be interpreted as more previews using less memory. Background work and cache/release timing dominate this small experiment. Counts 0 and 6 failed endpoint validation, so **their overhead profiles remain PENDING**; actual six-texture rendering is separately visible in the Z1 capture. The inspector's zero-tick short samples do not mean zero overhead. Source samples and validity flags are in [performance.json](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/performance.json) and [quick-profiles.json](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/quick-profiles.json).

### Frame pacing: PENDING

The inspected [RenderLoop API](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/core/renderloop.h) provides native C++ presentation timestamps, but the inspected public effect QML and current compositor/effect D-Bus interfaces do not export a usable collector. [OffscreenQuickView](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/effect/offscreenquickview.cpp) uses QQuickRenderControl into textures; completion there would not establish display presentation. `intel_gpu_top` and `perf` are absent, tracefs is unreadable, and available KWin DRM fdinfo had no relevant engine counters. NVIDIA's tool is not evidence for Intel compositing. No invasive profiler, plugin, root tracing or new service was installed.

**Median/p95/p99 presented-frame intervals, dropped/late frames, FPS, Intel GPU utilization and GPU memory are unmeasured.** No 144 FPS or performance guarantee is asserted. See [instrumentation.json](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/instrumentation.json).

## Cleanup defect, recovery and harness stabilization

Initial cleanup passed effect/package/action/session/display/desktop checks but failed the whole-file `kwinrc` hash. Investigation isolated one entry on the empty test desktop: `[Tiling][Desktop-3-ID][output-ID]/tiles`. Its populated two-column/vertical subtree was consistent with the enabled tiling integration retaining the disposable-window layout. The exact baseline bytes were recovered **as a hash-verified candidate**, not by assuming empty was the user's preference: substituting only that entry with the empty horizontal root reproduced the immutable full-file SHA-256. No other preference differed. See [mismatch evidence](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/configuration-mismatch-check.json) and [exact reconstruction](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/tiling-reconstruction.json).

The session baseline had hashes but no tiling snapshot, which was an insufficient rollback record for this automatic side effect. The recovery script verified the selected output/desktop, no normal/dialog clients on that desktop, no windows anywhere in the tile subtree, and the exact expected generated shape. It used the documented public `Workspace.rootTile(output, desktop)` and [`CustomTile.remove()`](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/tiles/customtile.cpp) to remove only those empty test-generated children. It did not overwrite/reset `kwinrc`, reconfigure/restart KWin, disable the existing tiling script or change a user window. Native saving then reproduced the exact baseline bytes. [Recovery result](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/tiling-recovery-result.json), [guarded script](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/depthos-02c-tiling-restore-4bb81da1.js), and [scoped recovery log](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/tiling-recovery.log) retain the evidence.

Future baselines now save only relevant tiling `tiles`/`padding` values, excluding unrelated configuration content. The POC-02C setup refuses a saved nonempty test layout instead of assuming every empty desktop has an empty configured grid. Regression tests verify that filtering/refusal. Cleanup now treats mismatched configuration hashes as failure and refuses stale mutations after a run is cleaned. Arbitrary nonempty/user-owned layouts are not automatically flattened; an unrecognized rollback difference must still stop for diagnosis.

Final [cleanup.json](../artifacts/live-acceptance/20261115T231959Z-4bb81da1/cleanup.json): **all 12 comparisons PASS**, including the exact original effect set, packages, action names, custom/POC-01 file bytes, outputs, desktops/current desktop, Activities, session owner, receiver removal, script unload and both configuration hashes. Original application UUIDs/memberships match. All recorded test PIDs and the independent guard exited. The original POC-01 package remains installed and unloaded as at baseline. No remaining cleanup action is required. Earlier failed-run evidence was retained.

## Acceptance matrix

PASS is scoped to the stated evidence. Programmatic/native state tests are intentionally separate from physical input and frame presentation.

| ID | Case | Result | Evidence / limitation |
| --- | --- | --- | --- |
| A1 | Fresh audit, unique manifest and verified prior evidence | **PASS** | New run; old run cleanup/evidence still present |
| A2 | Session identity and independent recovery guard | **PASS** | Owner + bus ID + PID pinning; separate recorded watchdog |
| B1 | All existing local regressions | **PASS** | 71 named checks; engine/cache/private-bus and syntax probes separately pass |
| C1 | Structured physical Escape/dismissal test | **PENDING** | User deferred; unsolicited live Escape traces exist but do not complete the test |
| C2 | Physical arrow navigation | **PENDING** | Operator sequence deferred |
| C3 | Physical Enter/Space selection | **PENDING** | Operator sequence deferred |
| C4 | Physical Backspace/minus/plus/equals | **PENDING** | Operator sequence deferred |
| C5 | Normal terminal typing after dismissal | **PENDING** | No fixed-marker input acknowledgement was received |
| D1 | Pointer hover and card selection | **PENDING** | No native desktop-control API; user deferred operator actions |
| D2 | Backdrop and close-button clicking | **PENDING** | Physical path not exercised as a controlled case |
| D3 | Mouse zoom and multiple-card selection | **PENDING** | Programmatic scene calls are not pointer events |
| D4 | Pointer/focus restoration | **PENDING** | Native inactivity is insufficient proof of ordinary input |
| E1 | Native Z1 six actual terminal frames | **PASS** | 02c-native-z1.png; actual KWin textures |
| E2 | Native Z2 four desktop cards | **PASS** | 02c-native-z2.png and native desktop enumeration |
| E3 | Native Z3 one existing Activity | **PASS** | 02c-native-z3.png; UUID retained, no invented display name |
| E4 | Programmatic depth round trips | **PASS** | 20 rounds issued to the production functions; final live visible state and level trace |
| E5 | Active desktop distinct from local inspection | **PASS** | Desktop 3 remains native; Desktop 4 inspected in screenshot/state |
| E6 | Activity inspection preserves actual Activity | **PASS** | Native and selected Activity IDs match throughout probe states |
| E7 | Inactive-desktop metadata-only fallback | **PASS** | 02c-inactive-inspection.png: no inactive texture forced |
| E8 | Depth selection through real keys/mouse | **PENDING** | Diagnostic actions bypass input delivery |
| F1 | Reduced-motion engine state and zero-duration transition | **PASS** | Native probe states: motionOff=true, transitionDuration=0 |
| F2 | Ten reduced-motion native open/close checks | **PASS** | Actual active/inactive checks after each call; observation wait 50 ms, not a frame metric |
| F3 | Rendered reduced-motion UI state | **PASS** | 02c-reduced-motion.png: Motion: reduced; native live scene |
| F4 | Reduced-motion button click and physical input release | **PENDING** | Flag was changed by a test-only diagnostic action |
| F5 | Global KDE duration read without modification | **PASS** | Actual public Kirigami longDuration=300; config restored exactly |
| F6 | Native global-zero preference case | **PENDING** | Permanent global animation settings were not changed |
| G1 | Presented-frame distribution / median / p95 / p99 | **PENDING** | No validated public presentation timestamp collector available |
| G2 | Dropped/late frames or FPS guarantee | **PENDING** | Callbacks, screenshots and refresh rate are not frame pacing |
| G3 | Intel GPU utilization / GPU memory | **PENDING** | Relevant counters/tools unavailable; no invasive profiler installed |
| G4 | Endpoint-valid 1/2/4-preview resource samples | **PASS** | Five 200 ms /proc samples each; coarse, non-causal observations |
| G5 | Uninterrupted 0-preview performance profile | **PENDING** | Recorded sample was invalidated by active/context change |
| G6 | Uninterrupted 6-preview performance profile | **PENDING** | Recorded sample was invalidated; six rendered textures separately verified |
| G7 | Resource recovery sample after unloading | **PASS** | KWin RSS 399988 KiB in last five two-second samples; processes released |
| G8 | Production performance bound | **PENDING** | No controlled benchmark, frame timing or long-duration evidence |
| H1 | 120 completed native settled cycles | **PASS** | 123 attempts; 3 declines/interruptions explicitly excluded from completed count |
| H2 | Sixty rapid toggles | **PASS** | Ten groups of six ended inactive |
| H3 | Ten double reversals | **PASS** | Interrupted entrance/exit followed by active then inactive checks |
| H4 | Five disposable create/focus/remove cases | **PASS** | Recorded native UUID/PID/application triple only |
| H5 | Observer and receiver restart while active | **PASS** | Fresh snapshots; native overview remained active |
| H6 | Bounded repeated-use survival | **PASS** | 195.575 seconds for final stress stage; no restart/crash, final inactive |
| H7 | Uninterrupted / production long-duration certification | **PENDING** | Three interrupted/declined attempts; minutes are not long-term certification |
| I1 | Reproduce no-delegate activation request mismatch | **PASS** | Actual callback/visible=true with no delegate and no active effect |
| I2 | Reset refused request without automatic retry | **PASS** | New regression and native startup-guard logs; 1000 ms bound |
| I3 | Known DepthOS fullscreen contention recovery | **PASS** | Other DepthOS effect remained active; first explicit invocation after release opened overview |
| I4 | Immediate and settled unchanged-revision reloads | **PASS** | 4 immediate + 4 settled native activations succeeded |
| I5 | Test expecting activation during desktop transition | **FAIL** | Attempt 9 refused after switching desktops; guard resets request. No unconditional activation guarantee |
| I6 | Exact reason for historical old-run refusal | **PENDING** | Failure class demonstrated; exact old keyboard-grab/fullscreen condition not recorded |
| J1 | Clean unloading/removal and state restoration | **PASS** | 12 final cleanup predicates true |
| J2 | Exact KDE configuration restoration | **PASS** | kwinrc and kglobalshortcutsrc SHA-256 match immutable baseline |
| J3 | Test-generated tiling layout recovery | **PASS** | Empty test tree restored via public API; no file reset or unrelated-window setter |
| J4 | Original application UUIDs and memberships | **PASS** | Original observed applications still exist on their original desktops |
| J5 | No test-owned PID or action remains | **PASS** | Guard, receivers, disposable clients and new action names removed |
| J6 | Preserve failed-run evidence | **PASS** | Old run retained; new failed/interrupted logs, samples and mismatch proof retained |
| K1 | Multiple outputs | **NOT APPLICABLE** | Only eDP-2 is connected/enabled; no layout modification |
| K2 | Fractional scaling | **NOT APPLICABLE** | Current scale 1; no global scale change |

## Reproduction and deferred operator checklist

From the repository, use a **new baseline** and independent guard; do not reuse this cleaned run for mutations. Execute stages individually and inspect each result. `quick-profiles` is coarse and can be interrupted; an invalid context must remain invalid. The per-run action names and package IDs are in the manifest. Current test packages have been removed.

```fish
python3 tests/integration/live-acceptance.py baseline
# Separate control process/terminal:
python3 tests/integration/live-acceptance.py guard
# Main control terminal:
python3 tests/integration/live-acceptance.py observer
python3 tests/integration/poc02c.py setup
python3 tests/integration/poc02c.py operator
# Only when the operator actually performs the checklist:
python3 tests/integration/poc02c.py operator-finish
python3 tests/integration/poc02c.py contention
python3 tests/integration/poc02c.py stress
python3 tests/integration/poc02c.py anomaly
python3 tests/integration/poc02c.py render-probe
python3 tests/integration/poc02c.py quick-profiles
python3 tests/integration/live-acceptance.py cleanup
```

The new baseline contains a tiling snapshot; nevertheless, inspect any configuration mismatch before restoring anything. Automatic tiling integrations can change persistent empty-desktop layouts when disposable clients appear. The exact guarded recovery used in this run is evidence for this specific empty tree, not a general command to flatten another desktop.

When the user is ready, the minimum pending physical sequence is: arrows/Enter/Space in Z1; minus/Backspace to Z2/Z3 and plus/equal inward; select an inactive desktop and the existing Activity while checking the actual native workspace does not change; Escape from each level; hover/click across two cards, zoom/close/backdrop controls; toggle reduced motion; then type `depthos-02c-ok` into the designated disposable terminal. Record actual observations and acknowledgement, with Escape and the independent unload watchdog as stop paths. This checklist is **deferred**, not presumed completed. No new operator question is pending.

**Recommendation:** retain Phase 1 as partially accepted. Finish physical input/focus restoration, obtain a quiet complete performance sample and investigate transition-time activation policy before Phase 2/POC-03 planning. Multi-output/fractional scale remain future compatibility gates. Do not implement workspace ownership or automatic routing in this milestone.
