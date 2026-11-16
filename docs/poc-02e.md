# POC-02E — automated native Wayland acceptance

> POC-02F correction: the unchanged final K06 PNG decodes and displays complete terminals with the marker and INPUT VERIFIED. The partial-file assessment below was not reproduced; see [independent verification](poc-02f.md#terminal-capture-investigation-and-correction). Historical input classifications are unchanged.

**PHASE 1 PARTIALLY ACCEPTED — 2026-11-16.** Native injected keyboard and pointer input now exercise production DepthOS in a disposable KWin compositor. This is **AUTOMATED NATIVE INPUT PASS** and **ISOLATED COMPOSITOR PASS** for the scenarios below. It is neither HOST SESSION PASS nor PHYSICAL HARDWARE PASS. No POC-03 or backend implementation was started.

## Reproduce

From the repository, run:

```sh
python3 tests/integration/poc02e/launch.py runner.py
python3 tests/integration/poc02e/launch.py watchdog-test.py
python3 -m unittest discover -s tests/integration/poc02e -p 'test_*.py'
```

The launcher compiles our small `native-input.c` against installed libei, creates a private home/config/data/cache/runtime directory and a new D-Bus session, installs only copied test packages there, and starts `kwin_wayland --virtual`. It does not install third-party executables or change host packages/preferences. Existing acceptance `state()` supplies host baselines and exact comparisons. The repository has no committed HEAD; production files and the read-only probe are SHA-256 recorded instead of inventing a git revision.

The private bus's KWin owner must belong to the compositor PID created by this runner before it requests an EIS connection. Input uses installed libei 1.6.0: sender handshake, capability binding, resumed devices, evdev keys, absolute pointer, button frames. The private endpoint requests all six conventional capabilities because requesting only absolute/keyboard/button negotiated only a keyboard here; the sender binds and emits only keyboard, absolute pointer and button events. EOF/SIGTERM releases tracked held input and stops the sender. No X11/XTEST or uinput is involved.

Production QML is copied unchanged apart from a read-only acceptance probe and delegate references needed by that probe. The probe walks real QQuickItems to read transformed geometry, grid index, selection, hover, context identity, native focus/membership and preview policy. It has no controller setter and never invokes a QML input handler. All navigation, clicks, Escape and typing use the compositor's EIS pipeline. Opening uses the registered KGlobalAccel action as required by K01. Model fixtures are not used in these native assertions.

## Results and evidence

The final run is `run-20261116T005728Z-5cbce7`: [evidence index](../artifacts/poc-02e/run-20261116T005728Z-5cbce7/evidence-index.json), [recording](../artifacts/poc-02e/run-20261116T005728Z-5cbce7/recording.webm), [assertions](../artifacts/poc-02e/run-20261116T005728Z-5cbce7/assertions.json), [host comparison](../artifacts/poc-02e/run-20261116T005728Z-5cbce7/host-comparison.json). It contains 45 passing machine assertions, 39 gate screenshots, native state and title-free logs, an evidence index, a sampled VP9 recording, resource samples, and cleanup/host comparisons. Read `evidence-index.json`, `actions.jsonl`, `assertions.json`, `gate-captures.jsonl` and `capture-timestamps.jsonl` together. Each action records its input method, target, timestamps, expectation, observation, screenshot and verdict.

- K01: actual output delegate, activeEffects membership, Z1, two native test-window cards and thumbnails.
- K02: Right/Left/Down/Up update the actual selected grid index; native active UUID, geometry, desktop and Activity memberships remain unchanged.
- K03: Enter and Space set local selected UUID without activating or moving a native window.
- K04: minus/Backspace, equals/Shift+equals traverse Z1/Z2/Z3 and back. Native Right/Enter and Left/Space inspect real desktops; Space inspects the real Activity. Four desktop IDs equal the isolated compositor's configured IDs; the Activity ID comes from its private Activity manager.
- K05: native Escape from each depth removes output delegates and activeEffects membership. Reopening, two rapid native navigation/cancellation sequences, and final Escape also pass.
- K06: a native pointer click targets the verified foreground disposable Konsole; evdev typing sends exactly `depthos-02e-ok` and Enter. The corresponding non-shell receiver acknowledges the marker and the expected native UUID has focus. Arbitrary lines are discarded and never executed or retained.
- Mouse: actual hover on both cards, selecting both cards, desktop and Activity inspection, both zoom controls, reduced-motion toggle, close button and scene backdrop. Targets come from current mapped scene geometry. The normal pointer click and typing after dismissal also pass.
- Contention: a separate copied POC-01 fullscreen effect remains active while overview startup is refused. After 1000 ms, the overview request resets with no delegate, no active overview and no automatic retry. Native Escape closes POC-01; the next explicit overview invocation succeeds. The static Projects card's text was visually inspected in the contention capture.

Earlier successful runs include `run-20261116T004500Z-ea03e0` (32 assertions), `run-20261116T004701Z-6e2125` (42), and expanded 45-assertion runs. They establish repetition without another 120-cycle stress campaign. Historical POC-02C stress evidence remains separate.

The checked frames show changed selection borders with the native focus label preserved, four desktop cards, the actual Activity card, native Konsole previews, reduced-motion control text, metadata fallback after disabling previews, and dismissal. The marker receiver and `INPUT VERIFIED` are visibly confirmed in the earlier 32-assertion run and recorder frame `run-20261116T005149Z-4ee2ef/recorded-frames/00103.png`. The final K06 gate PNG contains only a cursor/text fragment against black; visual normal-window restoration is INCONCLUSIVE in that frame despite successful UUID-specific acknowledgement. Machine state complements these visual observations. A protected native client was not provisioned: capture-exclusion fallback remains covered by existing policy/mock regressions, not a new protected-client native PASS. Single snapshots and sampled video do not establish correctness of every animation frame.

## Capture and recording

Only the private compositor is captured. ScreenShot2 `CaptureWorkspace` writes raw frames through a pipe drained concurrently; QImage converts the documented metadata format, stride and dimensions. A single producer requests about five captures per second. Gate captures copy a fresh, nonblack producer frame after the gate request; source frame and request timestamps are retained. Every source frame remains in the recording, including any blank source frames. Nonblack is only a capture sanity check, not the visual acceptance oracle.

The recording covers the actual input sequence, not a reconstruction from QML state. Three quiet measurement intervals deliberately pause screenshot requests; no input occurs during those intervals. The concat durations preserve those elapsed gaps as held frames. `recording.json` describes this limitation. The result is sampled visual evidence, not a full-refresh-rate screen recorder or validated presentation-time collector.

Earlier dual-producer runs preserved an intermittent black gate PNG while an adjacent recording frame showed the terminal and `INPUT VERIFIED`. One producer simplifies ordering but did not eliminate partial normal-window captures: the final K06 PNG still lacks the full terminal. This is an unresolved virtual capture/render observation, not a confirmed product defect or a resolved capture fix. Native input delivery passes independently; complete visual restoration remains INCONCLUSIVE for affected frames. Original ambiguous frames remain. [KWin 6.8.1 screenshot source](https://raw.githubusercontent.com/KDE/kwin/v6.8.1/src/plugins/screenshot/screenshot.cpp) creates a separate scene view and requests full damage; [virtual backend source](https://raw.githubusercontent.com/KDE/kwin/v6.8.1/src/backends/virtual/virtual_egl_backend.cpp) exposes buffer-age behavior. Reading these does not establish the anomaly's cause; no host rendering workaround was applied.

## Tooling and compatibility review

Observed system: KWin/Plasma 6.8.1, Qt 6.12.0, Wayland, installed libei/libeis 1.6.0. The isolated support report identifies OpenGL ES 3.2 on the NVIDIA RTX 4050 driver. An initial software-rendering environment override was removed; final results are not software-renderer claims. Unique copied package paths avoid shared-engine same-URL caching. Package installation precedes compositor startup: packages created after startup were not discovered in the first probes.

Source was downloaded and reviewed, not installed or run:

- [waytool](https://github.com/mx-shift/waytool/tree/007baab54cc9142d27c353790976cefcf62224e6), Cargo version 1.0.1, Apache-2.0, Rust 1.89. Portal/libei input, PipeWire capture and KDE scripting are relevant. Its daemon/installer/service and broad automation surface are unnecessary for this bounded test. No daemon, autostart or token was installed.
- [wdotool](https://github.com/cushycush/wdotool/tree/662d7079b669de164797f46fd437c0cf7854bf82), workspace version 0.5.3, MIT/Apache-2.0, Rust 1.82. Its portal/libei backend follows compositor permissions; its KDE documentation labels real-target verification experimental. Neither its installer nor release binaries were executed. Compatibility advertised by a repository is not a local runtime PASS.
- [kwin-mcp](https://github.com/isac322/kwin-mcp/tree/68a4aad47380331e3c355ec080f9182123730c41), MIT, Python. Its isolated virtual-session, EIS handshake and ScreenShot2 source informed the small test helpers. Its live input path uses private KWin EIS; that path must not be used on the normal desktop. No MCP server, dependencies or permission-bypass flags were installed into the host session.

The selected backend is our bounded sender against the already-installed [libei API](https://libinput.pages.freedesktop.org/libei/api/). Process isolation and explicit PID/bus/runtime checks restrict the private API to the disposable compositor. This source review is not a comprehensive security audit of those projects.

For a future host run, use [XDG RemoteDesktop](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html): CreateSession, SelectDevices, Start with explicit portal consent, then ConnectToEIS. Read-only host introspection reports RemoteDesktop version 2 and available device types 7. Capability discovery does not establish consent or successful input. No host portal session or persistent restore token was created; no host private EIS call was made.

Spectacle is installed and advertises screen/window/region recording. It remains an option for a separately authorized host capture; it was not launched to record private host content. ScreenShot2 was sufficient inside the isolated session. Qt/Python AT-SPI GI is available, but no trustworthy accessibility tree for the in-process KWin scene was validated; actual QQuickItem geometry was used instead. CUA native desktop controls are unavailable in this conversation.

## Performance and presentation timing

`quiet-performance.json` records separate settled three-second samples with screenshot requests paused and no input. The final run measured about 0.67% / 0.33% / 0.33% of one core with previews on / off / overview closed, around 452 MiB RSS. Its short samples overlapped a second private watchdog compositor trial, so they are observations rather than a single-workload benchmark; earlier expanded run `run-20261116T005149Z-4ee2ef` measured about 1.00% of one CPU core with two previews, 0.33% with previews disabled, and 0.33% with the overlay closed. KWin RSS was about 454 MiB throughout those short samples. These include the entire isolated compositor and are not a host overhead comparison, leak-duration study, GPU cost measurement or performance guarantee. Continuous resource samples explicitly include capture overhead.

**Frame pacing remains PENDING.** No validated public presentation-timestamp collector was available. FramePresented exists internally in KWin, but an injected profiler/plugin or compositor rebuild was not introduced. Median/p95/p99 display intervals, late frames and true activation presentation latency were not measured. No FPS claim follows from host refresh rate, screenshot timing, video encoding rate, or these CPU samples.

## Recovery, failures and cleanup

The launcher is independent of the input worker and bounds its private child process group to 240 seconds. On timeout, it verifies the saved PID start times and private compositor bus/PID identity, terminates input first, attempts scoped effect unload through the separate D-Bus path, then ends only that isolated process group. Normal cleanup stops input, stops capture, unloads copied effects, closes disposable apps, removes installed private packages, terminates the private compositor and its bus services, then encodes saved frames. No observer/receiver transport, host shortcut, portal session or persistent host setting was installed.

The deliberate stalled-worker test `run-20261116T005757Z-e50d6c` held a key in the isolated compositor and stalled the worker. The independent recovery stopped input, unloaded the active copied POC-01 through the separate effect control path, terminated the worker, and retained exact host and original-window comparisons. An earlier recovery check conservatively reported an exited-but-unreaped helper as still running; `running_process` now distinguishes zombie state, with regression coverage. PID start-time mismatch still refuses a reused PID.

The stricter K06 check failed in `run-20261116T004843Z-0014e7`: the chosen background terminal was covered by the other owned terminal, which received the marker. The failed UUID-specific assertion was preserved. The runner now targets the verified foreground UUID and requires its corresponding receiver acknowledgement. No production code was changed for this harness defect.

One expanded run and the first watchdog trial detected a host current-desktop difference during their intervals. Its cause was not attributed from timestamps alone; the runner did not force a desktop switch to manufacture equality. Other recorded fields matched. Later complete runs and the final watchdog trial matched the full baseline, including original Brave/Codex memberships and geometry, config hashes/tiling, desktop/Activity state, loaded effects, shortcuts and preserved packages.

An artifact cleanup sweep included the active private package directories of `run-20261116T005438Z-aac537`; that run is excluded from final cleanup-order sign-off. The fresh final run completed the ordered cleanup independently. The final run's cleanup gates pass. No test input/compositor/application process remains. Retired failed-probe installations under private artifact homes were removed; their logs and recordings remain. No global reset, cache deletion, host compositor restart, logout, privileged installation, unknown-layout restoration or original-window mutation was performed.

Local validation: **77 tests passed** (19 observer JS, 11 replica, 7 overview package/policy, 18 overview Qt, 10 POC-01 Qt, 8 existing integration safety, 4 new recovery). Additional native-JS, isolated receiver and child-context/cache probes passed. These local regressions are distinct from the 45 native acceptance assertions.

## Remaining Phase 1 gates

A separately consented host portal run is still needed to establish HOST SESSION PASS for the injected interaction sequence. Physical hardware acceptance remains unverified beyond the prior limited Escape evidence. The partial normal-window capture anomaly, presentation timing, a genuinely capture-protected native client, and relevant multi-output/scaling/suspend/lock scenarios remain open. The isolated controlled Activity and four desktops do not certify the host's topology. Preserve the partial verdict and hold Phase 2 until the required host/stability scope is accepted.
