# DepthOS live acceptance — POC-01 / POC-02A / POC-02B

## POC-02G — 2026-11-16

**KWIN 6.8.3 ISOLATED QUALIFIED; PHASE 1 PARTIALLY ACCEPTED.** [POC-02G](poc-02g.md) adds a release-specific API matrix, 104 passing local checks and 72 passing final isolated-native assertions. Native textures, dynamic windows, keyboard/mouse, focus/typing recovery, policy fallbacks, contention, bounded cycles and watchdog cleanup pass. Production source and the exact host baseline are unchanged. Installed 6.8.3 / running 6.8.1 still blocks host preparation; isolated qualification grants no host-session acceptance or portal authorization. Presentation timing/hardware and host-specific gates remain pending. Phase 2/POC-03 remains on hold.

## POC-02F — host authorization attempt, 2026-11-16

**PHASE 1 PARTIALLY ACCEPTED.** [POC-02F](poc-02f.md) executed host audit, empty-desktop preparation and a nonpersistent public RemoteDesktop/ScreenCast request. Start received no authorization response before the 180-second timeout: **host input BLOCKED, zero injected events, zero host frames, zero native host input assertions**. Cleanup passes all 12 checks, original-window comparison and process/session removal. Local regressions total 85 passes. Re-decoding the unchanged POC-02E marker PNG shows full terminals and recognizes the exact marker/acknowledgement; the previous partial-file diagnosis is corrected. No production fix was justified. Phase 2 remains on hold.

## POC-02E — native injected input, 2026-11-16

**PHASE 1 PARTIALLY ACCEPTED.** The [POC-02E report](poc-02e.md) and `artifacts/poc-02e/run-20261116T005728Z-5cbce7/` add automated keyboard/pointer acceptance in a private virtual KWin, with 45 machine assertions, gate screenshots, sampled video, exact marker acknowledgement, refusal/reopening checks and scoped recovery. This establishes **AUTOMATED NATIVE INPUT PASS** and **ISOLATED COMPOSITOR PASS** for the tested scenarios. Host input and physical hardware are separate pending gates. No host EIS bypass or host capture was used. Local regressions now total 77 passes; frame pacing remains pending. Production code is unchanged. Historical operator failures below remain scoped to their original host runs.

## POC-02D follow-up — 2026-11-16

**Current verdict: PHASE 1 PARTIALLY ACCEPTED.** See [poc-02d.md](poc-02d.md) for the latest bounded physical attempt. All required regressions pass (73 named checks after two test-harness additions). Real Escape reached Z1 and dismissed the overview. No `depthos-02d-ok` acknowledgement arrived before the disposable input programs ended; normal typing/mouse recovery and subsequent guided groups remain PENDING, rather than inferred from “dine” or elapsed time.

The current native contention check verified refusal/reset and success on one later explicit invocation. A preview performance capture was aborted by its privacy guard and is not accepted as a quiet sample. All 12 final cleanup comparisons and exact configuration hashes pass; original application memberships match and no test PID remains. Previous reports and failed evidence below are preserved. POC-03 remains on hold.


## POC-02C follow-up — 2026-11-16

**Current verdict: PHASE 1 PARTIALLY ACCEPTED.** The [POC-02C report](poc-02c.md) is the current acceptance authority. It records 71 passing local tests, 23 retested native observer gates, 120 completed native cycles across 123 attempts (3 declines/interruptions), programmatic Z1/Z2/Z3 and reduced-motion captures, and a targeted refused-start guard. The 9-trial load/transition experiment had 8 successful activations and one refused transition-time activation; that failed expectation is retained.

Physical interaction/typing was explicitly deferred by the user. Native Z2/Z3 rendering and inactive-desktop fallback are now independently observed through diagnostic scene calls, which do not establish physical key/mouse delivery. Only short endpoint-valid 1/2/4-preview resource samples were obtained; 0/6 profiles and frame pacing remain pending. All 12 final cleanup predicates and exact configuration hashes pass after restoring an empty test desktop's automatically generated tiling tree through guarded public APIs. The previous evidence below remains intact and is not added to the new totals.

## Original POC-01 / POC-02A / POC-02B acceptance run


Report date: **2026-11-16, Africa/Tunis**. Run `20261115T224144Z-5a370ed1` began on 2026-11-15 UTC / local time and crossed local midnight. This report supersedes earlier “not installed/tested” statements in the retained historical POC reports.

## Verdict

**PARTIAL ACCEPTANCE — NOT READY TO BEGIN POC-03.** Native state/lifecycle and a substantial rendered preview matrix pass. Real physical keyboard/mouse delivery, native Z2/Z3 navigation, reduced motion, frame pacing and extended stability remain unaccepted. No POC-03 code or workspace ownership/routing was implemented.

| Component | Verdict |
| --- | --- |
| POC-01 | PASS for native initialization, callback/animation lifecycle, 30 cycles, reversal, cleanup and actual static-card screenshot; physical interaction PENDING |
| POC-02A | PASS for the bounded native discovery/state/recovery suite, 23 checks; no production/long-duration guarantee |
| POC-02B | PASS for the final 22 native checks and inspected controlled screenshots after a preview-label fix; full interaction/level/performance acceptance PENDING |

Final local checks: **67 passed, 0 failed**: 19 JavaScript, 11 replica, 7 package, 17 overview QtTest, 10 POC-01 QtTest, and 3 harness regression tests. QtTest totals include init/cleanup cases. The QJSEngine, child-context/cache probes, isolated D-Bus receiver, fish syntax and Python syntax checks also pass and are not added to 67 as though they were named test cases.

Final automated native gates A–C: **57 PASS, 0 FAIL** (12 POC-01, 23 observer, 22 overview). Initial failed setup/harness attempts are documented below; this final-rerun count does not erase them. The complete matrix, including screenshots, cleanup and untested gates, contains **104 PASS, 24 PENDING, 4 NOT APPLICABLE and 3 BLOCKED**. These overlapping engineering gates are not independent benchmark repetitions.

## Baseline and safeguards

The requested baseline is [baseline.json](../artifacts/live-acceptance/baseline.json); its immutable run copy is [here](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/baseline.json). It records KWin 6.8.1 Wayland, PID 1701, bus owner `:1.9`, four existing desktops, their IDs/names/current desktop, one Activity, eDP-2 at 1920×1080 / scale 1 / 143.999 Hz, loaded/active effects, installed DepthOS package IDs, shortcut names, eight preserved custom/POC-01 file hashes and whole-file configuration hashes. It contains no window captions, browser contents or authentication data. Repository status was untracked at entry; no commit/push/reset was performed.

The installed `depthos-spatial-poc` 0.1.1 was preserved. The 30-cycle POC-01 test used `depthos-acceptance-poc01-5a370ed1`, a copied package with a new QML URL, run-specific unbound action and diagnostic prefix. The original was separately loaded for the screenshot, then restored to its originally unloaded state; its package/default/custom binding was not edited.

The Python test harness uses a separate guard process with a renewable 180-second lease. Cleanup is explicit and idempotent, not solely a shell trap. Live stages now refuse cleaned, expired or unguarded runs before mutation; a bus-free regression checks the refusal. Its manifest records install intents, exact IDs, PIDs with Linux process start ticks, test window native UUIDs and the guard. Window-control scripts require **UUID + PID + `org.kde.konsole`** to agree, and target only recorded disposable terminals. Public setters are used only on those clients. Existing user applications are never selected for a setter or close action. Desktop switching uses existing desktops only and restores the original desktop. No KWin/Plasma restart, logout, sudo/service, cache deletion, Activity/count change or unrelated effect mutation occurred.

Actual screenshots use the installed Spectacle background API after the user explicitly requested screenshots. Capture-exclusion protections were not bypassed. Dynamic captures were restricted to an existing desktop containing only test-owned normal/dialog clients; privacy guards aborted an attempted capture when native focus moved to a desktop containing a non-test application. No private-browser screenshot was retained.

## Commands and evidence

The persistent run evidence is under `/home/ashenix/Projects/DepthOS/artifacts/live-acceptance/20261115T224144Z-5a370ed1` (private directory, ignored by git). [commands.jsonl](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/commands.jsonl), [manifest.json](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/manifest.json), [scoped-kwin.log](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/scoped-kwin.log), [poc01.json](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/poc01.json), [observer.json](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/observer.json), [overview.json](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview.json), [performance.json](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/performance.json), and [cleanup.json](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/cleanup.json) provide machine-readable evidence. Receiver directories contain bounded title-free events and redacted snapshots using production authentication and reducer code. The operator acceptance file pins the actual observer source hash, bus ID and compositor owner.

Executed entry points (from the repository):

```fish
python3 tests/validate-poc.py
fish scripts/validate-observer.fish
fish scripts/validate-overview.fish
python3 -m unittest discover -s tests/integration -p test_live_acceptance.py
python3 tests/integration/live-acceptance.py baseline
# Independent terminal/process, kept running during live stages:
python3 tests/integration/live-acceptance.py guard
# Control terminal:
python3 tests/integration/live-acceptance.py poc01
python3 tests/integration/live-acceptance.py observer
python3 tests/integration/live-acceptance.py overview
python3 tests/integration/live-acceptance.py cleanup
```

The control harness invokes existing observer install/session/remove scripts and `tools/overview-dev.py prepare/install/load/toggle/unload/remove`. It launches `konsole --separate --workdir /tmp -e python3 -c <fixed test payload>` and records its PID and child PID. Spectacle uses `--background --nonotify --fullscreen --output <run-local-file>`. The full terminal payload and IDs are reviewable in the harness, without arbitrary user commands. Steps were rerun after the documented fixes; logs preserve successful and failed attempts where captured. Recovery instructions are independent of the main test loop:

```fish
python3 tests/integration/live-acceptance.py cleanup
# Current overview emergency unload also works without its acceptance record:
python3 tools/overview-dev.py unload --id depthos-overview-dev-f2b2103c9e9f
fish scripts/observer-session.fish --stop
```

Do not rerun individual stages against an unrelated live run. Start a fresh baseline and guard first, and always finish cleanup. The final tested overview source hash is `f2b2103c9e9f1f7b07bcdc8f8d5a859ae11382f9c877154b181d5642ee381c69`; no modified QML was reloaded from the same cached URL. Final new packages are removed, so further live loading requires scoped reinstallation.

## Native and visual findings

POC-01's isolated log contains 1 initialization, 70 shortcut callbacks, 32 delegate creations, 31 entrance completions and 32 exit completions. Callback counts exceed normal cycles because rapid/reversal cases are included. These are lifecycle events, not visual smoothness or physical-key proof. Its real screenshot independently confirms all expected text and styling.

The observer discovered real existing applications, four desktops, one Activity and one output with no missing required signals. Two disposable native clients established real add/remove/focus, desktop/sticky/multiple membership, minimized/fullscreen/capture exclusion changes. UUIDs were checked through KWin `getWindowInfo`; no caption from that method was stored. A stale closed-window target is refused without touching any other client. Consumer-requested reconciliation, receiver replacement and observer replacement all recovered genuine native snapshots. The final discovery/membership cross-check retains the original observed Brave/Codex UUIDs and memberships.

The overview screenshots below are **real compositor captures**, not the separate watermarked offscreen fixture. They show native card count changes, terminal icons/identifiers, focus, memberships/badges, actual rendered terminal textures, protected/minimized fallback, and correct removal. Only the third thumbnail state (frame actually presented inside the UI) is accepted for the observed normal/fullscreen/sticky/multiple cases. This does not certify arbitrary protected apps, every damage update, inactive-context rendering or frame pacing.

The overview stays active and shows the same real native cards while the independent observer is disconnected/restarted. This is expected for the documented direct-native-model architecture; an external replica is not its renderer source. The tested public boundary follows the version-matched [Workspace wrapper](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/workspace_wrapper.h), [Window properties](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/window.h), [D-Bus window info](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/dbusinterface.cpp), and [ShortcutHandler](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/shortcuthandler.cpp). Local installed headers independently confirm the writable properties used on disposable clients.

### Screenshot index

| Actual capture | Evidence |
| --- | --- |
| [POC-01](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/poc01-live.png) | Static Projects card and all expected text |
| [Empty desktop](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-empty.png) | Correct native empty state on Desktop 3 |
| [One client](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-one-window.png) | Actual terminal card/texture |
| [Two clients](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-two-windows.png) | Two native textures; corrected preview status placement |
| [Focus](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-focused.png) | Focus label follows the controlled native focus |
| [Minimized](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-minimized.png) | Minimized badge and metadata-only fallback |
| [Fullscreen](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-fullscreen.png) | Fullscreen badge and native frame |
| [Membership filter](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-membership-filter.png) | Moved test client disappears from inspected desktop |
| [Multiple desktops](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-multi-desktop.png) | Desktop 1 · Desktop 3 membership |
| [Sticky](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-sticky.png) | All desktops label and eligible frame |
| [Capture excluded](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-protected.png) | No protected-client thumbnail; honest protection text |
| [Observer disconnected](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-observer-disconnected.png) | Native model continues without diagnostic transport |
| [Observer restarted](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-observer-restarted.png) | Same cards after a fresh diagnostic session |
| [One removed](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-window-removed.png) | Only one remaining card |
| [All removed](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-all-removed.png) | Correct empty state after close during reversed transition |

## Performance: real process samples

Method: Linux `/proc/PID/stat` user+system CPU tick deltas divided by monotonic elapsed time and `SC_CLK_TCK`, expressed as percent of one CPU core. RSS is `/proc/PID/status` VmRSS. Each state has **3 × approximately 2-second samples**. Receiver CPU is included; the observer runs inside KWin and cannot be isolated as a separate process. KWin uses **OpenGL ES on Mesa Intel Graphics (ADL GT2)**. These are short samples from an interactive desktop, without a controlled workload, confidence intervals, GPU timing or a causal overhead estimate.

| State | KWin CPU mean | KWin RSS KiB | Inspector CPU mean | Inspector RSS KiB |
| --- | ---: | ---: | ---: | ---: |
| idleObserverUnloaded | 5.00% | 418,476–418,556 | 0.17% | 30,720–30,720 |
| idleObserverLoaded | 3.50% | 419,476–419,608 | 0.33% | 30,720–30,720 |
| overviewTwoThumbnails | 2.83% | 439,132–439,132 | 0.17% | 30,732–30,732 |
| afterOverviewUnload | 1.50% | 420,264–420,264 | 0.17% | 30,772–30,772 |
| finalObserverAndOverviewUnloaded | 4.00% | 419,412–419,412 | 0.17% | 30,776–30,776 |

Two-thumbnail KWin RSS was about 19 MiB above the adjacent idle sample, then returned near the idle range after unloading; this one bounded sequence does not establish absence of a leak. Lower CPU during the overview than during idle demonstrates sample noise/background activity, not an optimization benefit. No specific FPS or GPU-memory claim is made. `intel_gpu_top` is absent; `nvidia-smi` is present but targets the discrete GPU rather than this Intel compositor. The inspected compositor D-Bus interface supplied no collected per-frame timing.

Native event sample: **48 accepted frames over 9.583 s** (5.01/s), including one initial snapshot. This is the two-window test workload, not saturation throughput. Consumer reconciliation was **0.451 s**, receiver replacement **31.722 s**, and observer restart baseline **0.301 s**. These polling/file-observation latencies include the collector's 500 ms save cadence. The ~32-second receiver recovery matches the existing documented timeout limitation; it was not hidden or “fixed” by lowering production timeouts.

Across 30 overview cycles, action-to-first-observed-native-active latency had median **147.3 ms**, range **141.7–189.4 ms**. Measurement starts before the Python helper subprocess and includes helper startup, qdbus/IPC and state-read overhead; it does **not** measure first presented frame or completed entrance animation.

Offscreen Qt tests used software/mock KWin types. Their resource measurements remain separate: the final regression run reports 1.41 s elapsed and 78,344 KiB peak RSS. Those numbers are not compositor overhead.

## Defects, failed attempts and repairs

1. **Confirmed presentation defect — repaired:** fallback text asserted “No compositor frame available yet” and overlapped real native textures. The [before-fix capture](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/before-preview-fix/overview-two-windows.png) establishes the failure. Root cause: the same centered label remained visible while a transparent, aspect-preserving WindowThumbnail rendered over only part of the viewport; no public texture-ready signal exists. The fix separates the metadata-only fallback from a neutral footer outside the preview viewport. The new Qt regression verifies non-overlap, no false no-frame assertion and protection-state visibility. The new content-addressed revision was tested natively and the [after-fix capture](../artifacts/live-acceptance/20261115T224144Z-5a370ed1/overview-two-windows.png) confirms it.
2. **Harness D-Bus overload bug — repaired:** dbus-python introspection selected the one-string `loadScript` overload for a two-string call. It failed before a native window action. The harness now sends explicit `ss`; a bus-free mock regression verifies the actual call signature. The failed attempt's own terminals/receiver were cleaned before retry.
3. **Harness state comparison bugs — repaired:** D-Bus structs compared unequal to JSON-deserialized lists; one state comparison also overlapped the explicitly recorded POC-01 temporary load. Native state is now canonicalized, and only recorded test-owned effects are excluded from mid-run unrelated-effect comparison. A regression verifies known-owned additions are allowed and unrelated loss/addition fails. **Final cleanup uses an exact, unfiltered loaded-effect-set comparison** and exact configuration hashes.
4. **Capture/setup coordination — guarded and rerun:** the harness stopped when it found no safe empty desktop or focus changed to an existing application's desktop after minimizing a disposable. No private capture was retained. It now selects a currently empty existing desktop, restores that authorized test desktop before capture, and checks all normal/dialog clients there are test-owned. Existing application windows are never moved to achieve isolation.
5. **Initial activation anomaly — unresolved cause:** an initial older-revision setup reported inactive after invocation without scoped runtime diagnostics. A desktop/fullscreen transition or cached registration/setup state is plausible but unproven. Title-free lifecycle/model-count/preview-policy diagnostics were added, a new immutable URL prepared, and the final revision passed 30 cycles, reversal, same-unchanged-revision reload and cleanup. Do not describe instrumentation/new-URL success as proof of the original anomaly's root cause. Further repeated-session coverage remains pending.

Added title-free overview diagnostics record only initialization, callback/visible/transition state, delegate geometry, filtered row counts and fixed preview-policy messages. They log no captions/application content and make no texture-ready claim. The final scoped journal contains **0 matching QML errors**; this is not proof that every unrelated KWin component was error-free.

Files changed in this stabilization task: `experiments/kwin-overview/contents/ui/{main.qml,OverviewScene.qml,WindowCard.qml}`, `tests/overview/tst_overview.qml`, `tests/integration/live-observer.py`, new `tests/integration/live-acceptance.py`, new `tests/integration/test_live_acceptance.py`, and this report/status documentation. Existing POC-01/observer production source and all custom-effect files remain unchanged.

## Complete acceptance matrix

PASS requires evidence of that case, not a nearby callback or mock. PENDING means the relevant native/physical result is missing. NOT APPLICABLE means the configuration is absent. BLOCKED identifies a prohibited action or privacy conflict. Screenshot and lifecycle gates deliberately remain distinct.

| ID | Case | Result | Evidence / limit |
| --- | --- | --- | --- |
| A01 | Load isolated POC-01 | **PASS** | poc01.json; scoped lifecycle log |
| A02 | QML initialization | **PASS** | poc01.json; scoped lifecycle log |
| A03 | Shortcut registration | **PASS** | poc01.json; scoped lifecycle log |
| A04 | Activation callback | **PASS** | poc01.json; scoped lifecycle log |
| A05 | Screen delegate creation | **PASS** | poc01.json; scoped lifecycle log |
| A06 | Entrance completion callback | **PASS** | poc01.json; scoped lifecycle log |
| A07 | Exit completion callback | **PASS** | poc01.json; scoped lifecycle log |
| A08 | Reverse during entrance/exit | **PASS** | poc01.json; scoped lifecycle log |
| A09 | Six rapid toggles | **PASS** | poc01.json; scoped lifecycle log |
| A10 | Thirty native cycles | **PASS** | poc01.json; scoped lifecycle log |
| A11 | Final inactive | **PASS** | poc01.json; scoped lifecycle log |
| A12 | Clean unload | **PASS** | poc01.json; scoped lifecycle log |
| B01 | initialSnapshot | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B02 | existingWindows | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B03 | desktopEnumeration | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B04 | activityEnumeration | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B05 | outputEnumeration | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B06 | outputGeometry | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B07 | windowAdded | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B08 | focusChanged | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B09 | membershipChanged | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B10 | desktopChanged | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B11 | stickyMembership | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B12 | multipleDesktopMembership | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B13 | minimizedState | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B14 | fullscreenState | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B15 | protectedState | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B16 | windowRemoved | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B17 | staleIdGuard | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B18 | sessionRevisionConsistent | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B19 | reconciliation | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B20 | receiverRestart | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B21 | cleanUnload | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B22 | observerRestart | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| B23 | unrelatedStatePreserved | **PASS** | observer.json; authenticated production replica; UUID/PID-checked test clients |
| C01 | load | **PASS** | overview.json; native effect state / test client properties |
| C02 | duplicateInstallRejected | **PASS** | overview.json; native effect state / test client properties |
| C03 | duplicateLoadRejected | **PASS** | overview.json; native effect state / test client properties |
| C04 | activeOnEmptyDesktop | **PASS** | overview.json; native effect state / test client properties |
| C05 | twoNativeTestWindows | **PASS** | overview.json; native effect state / test client properties |
| C06 | nativeMinimizedState | **PASS** | overview.json; native effect state / test client properties |
| C07 | nativeFullscreenState | **PASS** | overview.json; native effect state / test client properties |
| C08 | nativeMembershipChanged | **PASS** | overview.json; native effect state / test client properties |
| C09 | nativeMultipleMembership | **PASS** | overview.json; native effect state / test client properties |
| C10 | nativeStickyState | **PASS** | overview.json; native effect state / test client properties |
| C11 | nativeCaptureRestriction | **PASS** | overview.json; native effect state / test client properties |
| C12 | overviewActiveObserverDisconnected | **PASS** | overview.json; native effect state / test client properties |
| C13 | overviewActiveObserverRestarted | **PASS** | overview.json; native effect state / test client properties |
| C14 | nativeRemovalDuringPreview | **PASS** | overview.json; native effect state / test client properties |
| C15 | nativeRemovalDuringTransition | **PASS** | overview.json; native effect state / test client properties |
| C16 | thirtyCycles | **PASS** | overview.json; native effect state / test client properties |
| C17 | sixRapidTogglesInactive | **PASS** | overview.json; native effect state / test client properties |
| C18 | reversalReopens | **PASS** | overview.json; native effect state / test client properties |
| C19 | activeUnload | **PASS** | overview.json; native effect state / test client properties |
| C20 | reloadUnchangedRevision | **PASS** | overview.json; native effect state / test client properties |
| C21 | finalInactive | **PASS** | overview.json; native effect state / test client properties |
| C22 | unloadInactive | **PASS** | overview.json; native effect state / test client properties |
| V01 | POC-01 actual card text and layout | **PASS** | poc01-live.png: title, Projects, Spatial Layer, selection hint, Escape hint |
| V02 | Native overview desktop/output/Activity identity | **PASS** | overview-empty.png: Desktop 3, active Activity UUID, eDP-2; matches native state |
| V03 | Native model renders zero/one/two cards | **PASS** | overview-empty / one-window / two-windows.png; controlled application count |
| V04 | Disposable card identity and icon | **PASS** | org.kde.konsole labels and terminal icons; separate native UUID/PID pairs |
| V05 | Focus indicator follows controlled focus | **PASS** | overview-two-windows / focused.png; Focused moves to the native active terminal |
| V06 | Card disappears after desktop membership change | **PASS** | overview-membership-filter.png: only the remaining terminal |
| V07 | Multiple/sticky membership labels | **PASS** | overview-multi-desktop / sticky.png: Desktop 1 · Desktop 3 / All desktops |
| V08 | No cards remain after both test clients close | **PASS** | overview-window-removed / all-removed.png: one then zero; no phantom card |
| V09 | Existing Brave/Codex card pixels | **BLOCKED** | Private-content restriction; independent native window discovery verified without capturing their contents |
| V10 | Missing application ID presentation | **PENDING** | Native discovery finds an empty app ID; fallback fixture passes, private application card not visually inspected |
| V11 | Native Z2 desktop count / Z3 Activity count rendering | **PENDING** | Observer enumerates 4/1; actual Z2/Z3 rendering not exercised through real input |
| V12 | No fake project/application cards in dynamic runtime | **PASS** | Empty and two-terminal native screenshots; source model is native, static Projects belongs only to POC-01 |
| D01 | Active normal window: thumbnail instantiated | **PASS** | Preview Loader policy plus actual native frame in screenshots |
| D02 | Active normal window: frame observed and correctly placed | **PASS** | overview-one-window / two-windows.png: real terminal pixels below header, status below viewport |
| D03 | Multiple simultaneous native textures | **PASS** | overview-two-windows.png: two separate terminal surfaces |
| D04 | Fullscreen native frame and badge | **PASS** | overview-fullscreen.png; KWin getWindowInfo confirms fullscreen |
| D05 | Minimized metadata-only fallback | **PASS** | overview-minimized.png: no preview for minimized client, Minimized badge |
| D06 | Inactive desktop metadata-only preview fallback | **PENDING** | Native membership filtering verified; inspection of an inactive selected context needs real navigation |
| D07 | Inactive Activity window | **NOT APPLICABLE** | Only one existing Activity; creating/switching a new Activity was not permitted |
| D08 | Sticky/multiple desktop preview | **PASS** | overview-sticky / multi-desktop.png: membership labels and eligible native frame |
| D09 | Capture-excluded preview and caption protection | **PASS** | overview-protected.png: metadata-only protection text; native excludeFromCapture true |
| D10 | Closing an existing preview / during reversed transition | **PASS** | overview-window-removed / all-removed.png; controlled native close actions |
| D11 | Exact asynchronous preview-creation teardown race | **PENDING** | Transition removal tested; cannot prove the close coincided with GPU-source creation |
| E01 | Open through registered action | **PASS** | Native activeEffects plus actual screenshots |
| E02 | Physical POC-01 assigned shortcut | **PENDING** | Original key preserved; no explicit observed key result |
| E03 | Physical POC-02B shortcut | **NOT APPLICABLE** | New action deliberately has no assigned key |
| E04 | Escape and focus/typing restoration | **PENDING** | User confirmed card visibility only, then requested self-testing; no actual key result |
| E05 | Arrow-key selection | **PENDING** | Qt fixture passes; physical delivery unverified |
| E06 | Enter and Space | **PENDING** | Qt fixture passes; physical delivery unverified |
| E07 | Zoom inward/outward controls | **PENDING** | Qt fixture passes; native control input unverified |
| E08 | Backspace/minus/plus | **PENDING** | Qt fixture passes; physical delivery unverified |
| E09 | Mouse hover | **PENDING** | No permitted desktop-input tool available |
| E10 | Mouse selection | **PENDING** | No permitted desktop-input tool available |
| E11 | Outside-click dismissal | **PENDING** | Qt fixture passes; actual pointer path unverified |
| E12 | Input after 30 repeated cycles | **PENDING** | Native inactive/unloaded checks pass, physical typing is not established by them |
| E13 | Selection does not activate/move actual windows | **PENDING** | Production source and fixtures pass; native selection input was not delivered |
| F01 | Entrance/exit and reversal lifecycle | **PASS** | Native POC-01 logs and both effect state sequences |
| F02 | Rendered animation smoothness | **PENDING** | Still screenshots/callbacks cannot establish smoothness or FPS |
| F03 | Six rapid toggles / thirty settled cycles | **PASS** | Both native effects tested separately; final inactive |
| F04 | Switch Z1/Z2/Z3 | **PENDING** | Real navigation input unavailable |
| F05 | Close from Z2/Z3 | **PENDING** | Real level navigation not exercised |
| F06 | Reduced-motion control in native UI | **PENDING** | Production Qt fixture passes; native control activation unverified |
| F07 | Repeated selection changes in native UI | **PENDING** | Production Qt fixture passes; native input unavailable |
| F08 | Window removal during reversed transition | **PASS** | Controlled Konsole closes; overview-all-removed.png |
| F09 | Observer stop/restart while overview remains current | **PASS** | overview-observer-disconnected / restarted.png and new observer token |
| G01 | Current geometry 1920×1080 | **PASS** | Actual 1920×1080 screenshot and kscreen/native observer cross-check |
| G02 | Current scale 1 and eDP-2 output | **PASS** | kscreen + native observer + screenshot footer |
| G03 | One output delegate and native desktop context | **PASS** | Single output geometry/logs; active/inspected Desktop 3 distinction in UI |
| G04 | Multiple outputs / per-output placement | **NOT APPLICABLE** | One connected/enabled output; offscreen two-output fixtures pass |
| G05 | Fractional scaling | **NOT APPLICABLE** | Current scale 1; global scaling was not changed |
| G06 | Native output geometry change / hotplug | **BLOCKED** | Would require changing the user display layout or manual hardware action |
| G07 | No duplicate/orphaned views after teardown | **PASS** | Single visible scene, final native inactivity/unload, no remaining owned native script/process |
| H01 | Real KWin/inspector CPU samples | **PASS** | performance.json; three 2-second samples per state |
| H02 | Real resident memory samples | **PASS** | /proc/PID/status VmRSS, not GPU memory |
| H03 | Native disposable-event throughput sample | **PASS** | 48 frames / 9.583 s; observed workload, not maximum capacity |
| H04 | Reconciliation/reconnect timing | **PASS** | 0.451 s resync, 31.722 s receiver reconnect, 0.301 s observer restart; polling/save cadence disclosed |
| H05 | Action-to-active-state timing | **PASS** | 30 helper/IPC-inclusive samples; not time to first presented frame |
| H06 | Resource cleanup | **PASS** | No test-owned PID remains; native components unloaded; receiver name released |
| H07 | Frame pacing / Intel GPU utilization and memory | **PENDING** | Intel compositor; intel_gpu_top absent; no frame timing interface collected; nvidia-smi targets the other GPU |
| H08 | Production overhead bound / extended stability | **PENDING** | Short, noisy samples on a working desktop do not establish attributable overhead or long-term stability |
| I01 | Independent watchdog armed; explicit recovery path | **PASS** | Separate guard PID; 180-second lease; no shell-trap dependency; active unload and cleanup verified |
| I02 | Watchdog expiry while overlay active | **PENDING** | Guard was not deliberately allowed to expire on an active overlay |
| I03 | Initial activation/setup anomaly diagnosis | **PENDING** | Early older-revision activation observed inactive without scoped logs; setup collision/desktop changes possible, exact cause unconfirmed |
| I04 | Desktop/Activity creation-removal behavior | **BLOCKED** | Existing counts must not be changed; fixture coverage only |
| I10 | ownerUnchanged | **PASS** | cleanup.json and exact baseline comparison |
| I11 | loadedEffectsRestored | **PASS** | cleanup.json and exact baseline comparison |
| I12 | preservedFilesUnchanged | **PASS** | cleanup.json and exact baseline comparison |
| I13 | desktopsUnchanged | **PASS** | cleanup.json and exact baseline comparison |
| I14 | currentDesktopRestored | **PASS** | cleanup.json and exact baseline comparison |
| I15 | activitiesUnchanged | **PASS** | cleanup.json and exact baseline comparison |
| I16 | outputsUnchanged | **PASS** | cleanup.json and exact baseline comparison |
| I17 | shortcutNamesRestored | **PASS** | cleanup.json and exact baseline comparison |
| I18 | packagesRestored | **PASS** | cleanup.json and exact baseline comparison |
| I19 | receiverStopped | **PASS** | cleanup.json and exact baseline comparison |
| I20 | ownedScriptsUnloaded | **PASS** | cleanup.json and exact baseline comparison |
| I21 | KWin and shortcut configuration file hashes unchanged | **PASS** | kwinrc and kglobalshortcutsrc whole-file hashes match |
| I22 | Original application discovery/membership cross-check | **PASS** | existing-window-crosscheck.json: initial observed application UUIDs still exist with same memberships |
| I23 | No test-owned process remains | **PASS** | PID and start-tick verification after cleanup; guard exited |

## Cleanup and remaining work

All eleven cleanup comparisons passed: compositor owner, exact original loaded effects, preserved custom/POC-01 bytes, desktop list/count/current desktop, Activity list/current Activity, outputs, shortcut names, package inventory, receiver removal and owned-script unloading. Whole-file `kwinrc` and `kglobalshortcutsrc` hashes also match. No test-owned PID remains, including the independent guard and all disposable terminal payloads. The installed original POC-01 remains present and unloaded as at baseline. Newly created overview/run-specific actions are removed; its original shortcut action remains. No remaining emergency cleanup action is required.

The input capability audit found only X11 `xdotool`, which is not a native Wayland input test tool; ydotool/dotool/wtype/kdotool were absent. Raw `/dev/uinput` is writable, but no global virtual-input device was created. That would not certify physical hardware input and was not used as a replacement for the unavailable desktop-control tools. See `input-capabilities.json`.

Physical input is the smallest outstanding manual gate: in a later explicitly scoped session, test actual Escape/outside-click focus recovery, arrow/Enter/Space selection, plus/minus/Backspace and Z1/Z2/Z3 navigation/reduced motion, with normal typing/clicking afterward. Do not silently mark these passed because the native inactive flag or Qt fixture passes. This run honored the user's request to self-test with real screenshots instead of repeatedly requesting physical actions.

Before POC-03, close these physical/native-level gates, investigate the initial activation anomaly across fresh isolated revisions/sessions, perform longer repeated-use/memory checks and obtain meaningful Intel frame-pacing evidence. Multi-output/fractional scale/hotplug remain untested on current hardware/layout. Project ownership and automatic workspace management should wait; the bounded discovery/rendering foundation is substantially verified but is not fully accepted for unattended control.
