# POC-02F — host Wayland authorization and Phase 1 sign-off

> Current preflight: installed KWin 6.8.3 differs from running KWin 6.8.1 after an OS package update. Live preparation is blocked before portal authorization. See the [version-gate repair](poc-02f-version-preflight.md); the historical attempt below remains unchanged.

Date: 2026-11-16. Run: `20261116T011647Z-31f348e3`.

**PHASE 1 PARTIALLY ACCEPTED. HOST SESSION AUTOMATED NATIVE INPUT: BLOCKED. CLEANUP: PASS.** The public RemoteDesktop authorization request received no Response signal before its bounded 180-second timeout. No host input events were sent, no host frames were consumed, and no private input/capture fallback was attempted. This was an authorization timeout, not an observed user denial and not a failed keyboard-delivery assertion. Phase 2 remains on hold.

[Evidence index](../artifacts/poc-02f/20261116T011647Z-31f348e3/evidence-index.json) · [Portal result](../artifacts/poc-02f/20261116T011647Z-31f348e3/portal-start-result.json) · [Cleanup](../artifacts/poc-02f/20261116T011647Z-31f348e3/cleanup.json) · [Original windows](../artifacts/poc-02f/20261116T011647Z-31f348e3/original-window-comparison.json).

## Current host audit

The new run reuses the existing native acceptance baseline, authenticated foreground observer, process start-time checks, independent lease guard, idempotent cleanup and guarded known-empty tiling recovery. It uses a separate `artifacts/poc-02f/` registry; no previously cleaned run was reused for system changes.

Observed identity: KWin PID 1701, bus owner `:1.9`, session bus ID `93c71310c066bc725c60e4bc5aaccad0`. Actual KWin is 6.8.1 and both running/compiled Qt versions are 6.12.0. Host compositing is OpenGL ES 3.2 via EGL on **Intel Mesa 26.2.5**, unlike POC-02E's virtual compositor using NVIDIA. PCI enumeration also confirms the RTX 4050 Mobile is available; that does not mean the host compositor uses it.

The host retains four desktops, one Activity, and one enabled eDP-2 output at 1920×1080, scale 1, reported refresh about 143.999 Hz. Desktop and Activity IDs, original active desktop, effects, shortcut names, package files, exact `kwinrc`/`kglobalshortcutsrc` hashes and scoped tiling values are recorded in `baseline.json`. The observer recorded the original Brave and Codex UUIDs/memberships; title text is redacted. No browser/document contents were captured.

Existing Desktop 3 (`e3205213-75ad-48fd-925e-e7217cd1129f`) was empty of original application/dialog windows and had a saved empty tile layout. No desktop or Activity was created/deleted. The runner switched only the current context and launched two disposable Konsole processes:

- PID 137958, start tick 1422782, native UUID `aa9e96fb-2128-407e-87db-b5e93ef71390`.
- PID 137976, start tick 1422843, native UUID `cc1dea0d-2076-43fb-9f32-fbb90e725add`.

Both were verified as `org.kde.konsole` on the test desktop. Their receiver compares only `depthos-02f-ok`; it never executes input and retains no arbitrary lines. Original application windows were not moved, minimized, activated or closed by the runner.

## Portal result and security boundary

Actual public interfaces advertise RemoteDesktop version 2/device mask 7 and ScreenCast version 6/source mask 7/cursor mask 7. GStreamer has `pipewiresrc`, `videoconvert`, `vp8enc`, `webmmux` and `appsink`. These are capabilities, not authorization.

The executed client called CreateSession, SelectDevices for keyboard/pointer with `persist_mode=0`, ScreenCast.SelectSources for one monitor, then RemoteDesktop.Start. Clipboard and persistent restore permissions were not requested. The client waited for the operating system's authorization result. The [portal contract](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html) makes Start's Response and granted devices authoritative; only afterward may ConnectToEIS provide the sender fd.

No Start Response was received within 180 seconds. The client closed the request, recorded **BLOCKED**, and ended. ConnectToEIS, sender negotiation, input delivery and frame consumption were never reached. The requested session interface is confirmed absent after cleanup. The close attempt reported an already-absent session path; this was verified separately rather than treating arbitrary D-Bus errors as successful closure.

No portal store, restore token, private KWin EIS endpoint, uinput permission, autostart, network control listener, privileged service or global rendering setting was used. The user was asked only whether KDE's pending dialog was visible; a chat answer could not grant OS authorization. The run was not automatically retried after timeout.

## Keyboard, mouse, focus, navigation and rendering

**New native host input assertion total: 0.** Groups A–D and host visual capture are BLOCKED by the authorization checkpoint, as recorded in `assertions.json`. They are not PASS and are not evidence of a product failure. In particular:

- Arrows, Enter/Space, depth navigation and Escape were not injected on the host.
- Card hover/clicks, desktop/Activity inspection, zoom, reduced motion and backdrop/close clicks were not executed on the host.
- The exact `depthos-02f-ok` acknowledgement was not attempted; host typing/focus recovery is unverified.
- No copied DepthOS renderer was installed, loaded or activated after the blocked checkpoint. Controlled contention was not run in this host attempt.
- No host screenshot or recording exists for this run. No unrelated content was captured to inspect the consent UI.

POC-02E's **45 isolated native assertions, 39 gate images and recording remain historical ISOLATED COMPOSITOR PASS** for their tested scope. They are not added to this run's host totals. The older limited physical Escape observation likewise does not establish PHYSICAL HARDWARE PASS for the full sequence.

## Prepared automation and validation limits

`tests/integration/poc02f/host.py` prepares a fresh context and independent recovery; `portal.py` implements the public portal flow and accepts bounded project-local input commands only after authorization; `run.py` contains the requested keyboard, mouse, marker and contention sequence. `journal.py` filters diagnostics to this run's fixed prefixes. `policy.py` supplies fail-closed allowlist and capture-scope checks.

The sender reuses POC-02E's installed-libei C code; its identifying text now says “bounded acceptance” so it does not falsely label a potential authorized host connection as isolated. No input behavior changed. Preparation compiles it locally with warnings treated as errors. The sender itself discovers no input endpoint; the portal client must supply its authorized fd.

The prepared capture path opens only the portal PipeWire remote after authorization. Its consumer stays disabled until the runner enables recording on the controlled desktop. It checks actual native state before and after a candidate frame, rejects foreign focus/windows/current desktop, and saves only accepted samples. The sequence derives click positions from actual transformed scene geometry. Saved samples can be encoded locally after system cleanup; capture PTS and recording cadence are not presented-frame measurements.

**The post-authorization host paths are implemented but unexecuted and not runtime-certified.** Syntax, compilation, isolated historical input behavior and pure safety tests are not substitutes for a successful host portal trial. The blocked client ran the earlier authorization-only revision; later capture/control adaptations were not retroactively exercised by that process. Source hashes for the executed authorization/preparation revision were not separately captured at launch; final delivered source hashes are retained as provenance, not proof of what the earlier process loaded.

A future explicit retry starts a new run with `python3 tests/integration/poc02f/accept.py`. That launcher prepares the context, starts the portal client, waits for `AUTHORIZED_READY`, runs the native driver only after that result, and cleans up without automatically retrying a denied/timed-out request. Grant or deny only in KDE. `host.py cleanup` is also available, and the independent lease guard is armed. The one-command launcher is syntax-checked; its complete post-consent path remains unexecuted. Never reuse this cleaned run or interpret a chat reply as portal authorization.

## Terminal capture investigation and correction

The earlier POC-02E report described the final marker PNG as a partial terminal capture. **That file-level assessment is corrected.** In this task, the unchanged saved PNG was reopened and visibly contains both complete Konsole windows, the exact marker and `INPUT VERIFIED`.

Independent Qt and FFmpeg decodes produce identical RGB bytes (SHA-256 `2e21e200267f97c7cda269417a5bd1b6908132de3a79b80643ba302c95cba86a`). The PNG has no alpha channel; representative window pixels contain complete terminal and decoration colors. Tesseract on the original, unmodified file recognizes both fixed strings. Only match booleans are retained. File SHA-256, original mtime, sample metadata and independent decoder results are in `capture-redecode.json`; `capture-ocr-check.json` records the OCR corroboration.

The PNG SHA-256 is `45c8a4f9812e7e17768790511941d2ae49a9282d21477e9b564a2370e47fd458`. It was neither rewritten nor regenerated. Its approximately 23.4% nonblack image area is consistent with terminal windows against a black virtual-desktop background; this number is not itself a visual PASS oracle.

Several earlier marker PNGs also decode to full terminal data. Consequently the retained PNGs do not support blaming capture request concurrency, buffer synchronization, damage tracking or KWin presentation for the earlier viewing discrepancy. The discrepancy's cause remains undetermined; image presentation/inspection is another possible category. A single producer did not prove a compositor fix. No production fix or host rendering workaround is justified by this evidence. An authorized host comparison remains unavailable. This correction does not establish correctness of every source frame or alter host-input acceptance.

## Performance and regression

No validated public collector for KWin's presented-frame timestamps was available. Existing support information and QML lifecycle callbacks do not expose a reliable per-display presentation series; PipeWire timestamps describe capture. No profiler was injected and no compositor rebuild, global FPS effect or privileged tracing tool was introduced. **Frame pacing remains PENDING**: median/p95/p99 intervals, dropped/late frames and activation-to-first-presented-frame latency were not measured.

The 0/1/2/4/6 preview characterization was not executed after authorization failed. A bounded three-second ordinary-host sample with zero test previews observed approximately 5.67% of one CPU core and 242→241 MiB KWin RSS. It overlapped local regression work and was not a verified quiet workload, so it establishes no preview overhead, leak or production performance claim. `performance.json` labels these limitations.

**85 local tests passed, zero failed:** 19 observer JavaScript, 11 replica, 7 overview package/policy, 18 overview Qt, 10 POC-01 Qt, 8 existing integration safety, 4 recovery and 8 new host policy checks. Additional native-JS, authenticated receiver and delegate/cache probes pass. The new tests reject arbitrary shortcuts/buttons, offscreen/nonfinite pointer targets, foreign/sticky windows, wrong PIDs, foreign focus and wrong desktops. The sender compiles with `-Wall -Wextra -Werror`; all new Python files compile. No extra 120-cycle campaign was run.

## Cleanup and engineering verdict

The independent guard was armed before host preparation. Cleanup stops input/capture/portal roles, unloads the scoped observer, closes only recorded disposable processes, removes its package and restores the original current desktop. All **12 existing cleanup comparisons pass**, including exact configuration/tiling hashes, original effects/shortcuts/packages, desktop/Activity/output state and receiver/script removal. The separate original-window comparison also passes, as does the no-owned-process check. The portal session interface is absent. Repeating cleanup is a no-op.

Classifications are deliberately separate:

- **ISOLATED COMPOSITOR PASS:** prior POC-02E tested interactions; retained marker PNG assessment corrected above.
- **HOST SESSION AUTOMATED NATIVE INPUT:** BLOCKED, no authorization response.
- **PHYSICAL HARDWARE PASS:** not established for the requested sequence.
- **FRAME PACING MEASURED:** no; PENDING.
- **CLEANUP PASS:** yes, this host preparation/authorization attempt.

Phase 1 software sign-off still requires an actual authorized host run with marker/focus recovery, native interaction assertions and cleanup. There is no confirmed new production defect to fix from this attempt. Resume through a fresh nonpersistent KDE consent flow when the authorization dialog can be completed; validate the unexecuted host paths and retain actual visual evidence. Phase 2 is not ready to begin under this milestone's acceptance rule. Rust, project ownership/routing, SQLite and AI were not implemented.
