# Phase 4.1.1 UX corrections

Agent 1. Branch `codex/phase-4-1-1`. These notes describe what the spatial alpha actually does after the corrections. Live pinch delivery is not claimed.

## DEF-07 — focus returns when the window is active again

`SpatialScene` listens to `Window.active`. When the window becomes active, `restoreFocusAfterActivation()` runs on the next turn (`Qt.callLater`).

- If the search palette is open, focus stays in it (or is put back on the search field).
- If the onboarding panel is open, focus stays on that panel.
- A `TextInput` / `TextEdit` outside the scene (the stand-in for rename and other text entry) keeps focus.
- Otherwise, if the canvas itself does not own the keys, focus is moved back to the scene so H/L/Enter work immediately.

The scene is an `Item`, not a `FocusScope`. `activeFocus` on the scene means the canvas key handler owns the keyboard. A focus scope would stay "active" while the search field inside it was focused, which is the opposite of the required check.

Escape inside the search field was a real bug: the field emitted `closed` and the scene only called `forceActiveFocus()`, so `isOpen` stayed true and later key presses were ignored. Closing the palette now clears `isOpen` and returns focus to the scene. A backdrop click does the same. The search field takes focus only while the palette is open, so a closed palette cannot keep eating keys between tests.

`qmltestrunner` reuses one window for the whole file. `init`/`cleanup` close the palette, clear the foreign text field, destroy the second window, and reset gesture clocks. With the other window created with a null parent, offscreen `Window.active` deactivation was observable in this run: the reactivation assertions passed (47/47). They were not downgraded to Wayland-only. A synthetic test still calls `restoreFocusAfterActivation()` directly so the handler is covered even if a future offscreen QPA stops delivering deactivation.

## DEF-08 — wheel and trackpad pan

One `WheelHandler` on the scene calls `handleWheel` once per event. There is no second `onWheel` path.

- `pixelDelta` is the motion when it is non-zero. `angleDelta` from the same event is ignored, so one touchpad sample is not applied twice.
- 120 angle units (one traditional notch, or eight 15-unit hi-res steps) move the camera by `wheelStepPx` (96).
- Vertical wheel pans vertically. A horizontal wheel, or Shift plus a vertical wheel, pans horizontally.
- The camera is clamped to the overflow of the current level. Values are written directly. There is no `Behavior` on `cameraX` / `cameraY`, including when reduced motion is off, so a notch is not queued behind an animation.
- Ctrl+wheel changes hierarchy level and does not pan. One burst inside `levelStepCooldownMs` (320) is one step. The wheel never closes the window and never cycles windows at the deepest level.
- H/J/K/L, arrows, Enter/Space, Esc/Backspace, `/`, Home, T, M, and `[` / `]` are unchanged. After a pan, horizontal keyboard movement scrolls the focused card back into the viewport.
- The wheel does nothing while the search palette or the onboarding panel is open.

KDE gesture settings are not read or written.

## DEF-09 — pinch

`pinchBegin` / `pinchUpdate` / `pinchEnd` / `cancelGestures` implement the gesture rules, and a `PinchHandler` (`PointerDevice.TouchPad` only) calls those same functions if Qt delivers a pinch:

- Scale at or above 1.25 drills one level down. Scale at or below 0.75 goes one level up.
- The card under the pointer is selected first.
- The direction latches. One gesture does not drill twice and does not reverse.
- A cancelled or interrupted gesture does not commit a transition that has not already latched. Deactivating the window cancels the gesture.
- At the top level a pinch-out does not close the window. At the bottom level a pinch-in does not cycle windows.
- The gesture is ignored while search or onboarding is open.

Live delivery on this desktop is **BLOCKED / UNSUPPORTED-unverified**. No human pinch was available.

`tools/depthos-gesture-probe` (already on the branch) is the passive probe. An unattended 3 second window on the live Wayland session (`wayland-1`, Qt 6.12.0) recorded:

- compositor global `zwp_pointer_gestures_v1` version 3
- the Qt touchpad device advertises `Capability.Position` only (10 points)
- 0 `QNativeGestureEvent`, 0 `PinchHandler` updates, 0 wheel events

The offscreen `--synthetic-self-test` shows the logging pipeline and that a synthetic `QNativeGestureEvent` reaches `PinchHandler`. Every synthetic record is tagged `"synthetic": true` and is not evidence of what KWin delivers. No compositor plugin was added. Mouse, wheel, and keyboard remain the navigation paths.

## DEF-11 — click selects, double-click enters

World and workspace cards: a click selects and syncs the keyboard index. A double-click enters that level once (`levelTransitions` increments by one). `MouseArea` emits `clicked` as well as `doubleClicked`; only `activated` changes level. Unavailable cards (`unavailable: true`) ignore both. Window cards select only. A double-click does not activate a native window and does not change level.

## Source labels

The shell always shows two strings from `SpatialBackend`:

| `sourceKind` | Source label |
| --- | --- |
| `demo` | Demo |
| `sqlite` | Local SQLite |
| `controller` | Connected Controller |
| anything else | No Source |

The observer label is `Native Observer Live` only when `observerLive` is true, and `Native Observer Unavailable` otherwise. A connected controller sets `observerLive` only when its reported observation health is `Synchronized`. Demo data forces `observerLive` false and `observationHealth` `Unavailable`.

Workspace cards no longer use the hard-coded `index === 1 ? "Stale" : "Live"` badge. Demo cards say `Demo`. A non-live sqlite or controller source says `Unavailable` (or `Stale` / `Reconnecting` when that is the real observation health). `StatusBadge` treats `Demo` as its own color and does not pulse it like `Live`.

Demo mode draws a `DEMO DATA` watermark. It is not shown for sqlite or controller data.

`depthos ui` no longer falls back to `/home/ashenix/Projects/DepthOS-phase-4-1/tools/depthos-spatial-alpha`. It passes `DEPTHOS_BIN` as its own executable so the launcher can run `status`, `init`, and `tree` from the binary that launched it.

## Instrumentation

The launcher accepts `--frame-log`, `--memory-log`, and `--screenshot` (also `DEPTHOS_FRAME_LOG`, `DEPTHOS_MEMORY_LOG`, `DEPTHOS_SCREENSHOT`).

- Frame log: `frameSwapped` interval and the `beforeRendering`/`afterRendering` span, plus `QSGRendererInterface.graphicsApi` when the cast succeeds. This is a startup sample for Agent 2, not a 144 Hz pacing result.
- Memory log: `/proc/self/status`, `smaps_rollup` (Rss, Pss, Private_Dirty, Anonymous, file-backed shared), and up to four DRM `fdinfo` blobs.
- Screenshot: `grabToImage` of the scene only. `grabWindow()` aborted in this session and is not used.

One live Wayland sqlite launch (OpenGL, platform `wayland;xcb`, i915 fdinfo) recorded 32 presented intervals: median about 6.93 ms, 13 above 6.94 ms, 5 above 16.67 ms, max about 1070 ms on the first frames. That sample includes startup. It does not show a sustained 144 Hz pace.

The same process at start was about VmRSS 164 MiB and Pss 60 MiB. That is one sample of the Python UI process, not the Phase 4.1 523 MiB investigation.

## What was verified

| Check | Result |
| --- | --- |
| `cargo test --workspace --locked --offline` | 232 passed, 0 failed, 1 ignored |
| `cargo fmt --all --check` | pass |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | pass |
| `tst_spatial_alpha.qml` | 17 passed |
| `tst_phase_4_1_1.qml` | 47 passed |
| `qmllint` on `experiments/spatial-alpha/*.qml` | pass |
| `tests/spatial/version-mismatch.py`, `client-smoke.py`, `benchmark_spatial.py`, `performance.py`, `validate_alpha.py` | pass |
| Missing disposable database is not created by the launcher | pass |
| `~/.local/share/depthos` absent before and after | pass |
| Host `kwinrc` / `kglobalshortcutsrc` / `kdeglobals` md5 after the runs | equal to the before md5 |
| KWin PID | 1727 throughout |
| `isScriptLoaded("depthos-observer")` | false |
| Autostart entries | unchanged |

`validate_alpha.py` rewrote `artifacts/phase-4-1/*`. Those tracked files were restored to the committed bytes. New frames are under `artifacts/phase-4-1-1/agent1/` only.

Opening the Wayland windows made Polonium rewrite one `[Tiling]` `tiles` line in `kwinrc`. That file was copied back to the pre-launch bytes. The final md5 matches the hash taken before any of this work (`5c54da5400798ca2454555294ed4f2b5`). No DepthOS script, shortcut, or autostart entry was added.

## Not claimed

- A person pinching the touchpad. See DEF-09.
- `tests/spatial/verify-source.py` needs `/home/ashenix/Projects/DepthOS-backups/phase-2.5-20261116T144802Z/manifest.json`, which is not on this machine. That failure is pre-existing and was not "fixed" by inventing a manifest.
- `tests/spatial/native.py` drives the Phase 2.5 overview package inside a private KWin. It was not run here.
- 144 Hz frame pacing, the 523 MiB RSS question, and native observer feasibility are Agent 2's measurements. The hooks above are the instrumentation for that work.
