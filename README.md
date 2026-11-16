# DepthOS

**Current milestone: Phase 2.1.** The Rust core stores Worlds, Workspaces, and Logical Desktops in transactional SQLite. The local CLI manages the logical hierarchy and runs independently of KWin. Phase 1 remains partially accepted.

```bash
cargo build --locked
test_dir=$(mktemp -d)
target/debug/depthos --db "$test_dir/depthos.sqlite3" doctor
cargo test --locked
```

See the [Phase 2.1 architecture, CLI, and validation guide](docs/phase-2.md).

Latest qualification: [POC-02G](docs/poc-02g.md) — **KWIN 6.8.3 ISOLATED QUALIFIED**. 104 local checks and 72 final native isolated assertions pass, including real thumbnails, dynamic observer/model events, EIS input and typing recovery. Watchdog cleanup and exact host preservation pass. Production code is unchanged. Host preparation remains blocked by installed 6.8.3 / running 6.8.1; isolated evidence grants no host authorization. Phase 1 remains partially accepted.

Repeat only the private qualification:

```fish
python3 tests/integration/poc02e/launch.py runner.py --artifact-group poc-02g --expected-version 6.8.3
```

Latest host attempt: [POC-02F](docs/poc-02f.md) is BLOCKED at KDE portal authorization (no response before timeout). No host input/capture occurred; cleanup and original-window checks pass, with 85 local tests passing. Phase 1 remains partially accepted. The earlier terminal-capture diagnosis is corrected by independent decoding of the retained PNG.

Latest acceptance: [POC-02E automated native Wayland tests](docs/poc-02e.md) pass the isolated injected keyboard/mouse scope with 45 assertions and recorded evidence. Phase 1 remains partially accepted: host portal input, physical hardware and frame pacing are separate pending gates. Run `python3 tests/integration/poc02e/launch.py runner.py` for a disposable private-compositor test.

Latest: [POC-02D physical acceptance](docs/poc-02d.md), 2026-11-16 — **PHASE 1 PARTIALLY ACCEPTED**. Escape is verified; the fixed typing marker was not received, so remaining physical groups are pending. Current local checks: 73 passes, zero failures. All 12 cleanup comparisons pass. POC-03 remains on hold. Earlier results below are historical and are not added to these totals.


**PHASE 1 PARTIALLY ACCEPTED — physical interaction acceptance is deferred.**

The latest [POC-02C report](docs/poc-02c.md), dated 2026-11-16, records **71 passing local checks**, 23 native observer checks, 120 completed cycles in 123 attempts, actual Z1/Z2/Z3 screenshots and programmatic reduced-motion validation. A refused-start request guard was added and tested. One transition-time activation trial was refused; physical keyboard/mouse and typing restoration, frame pacing, and complete quiet performance profiles remain pending. All 12 final cleanup checks and exact KDE configuration hashes pass, including recovery of a test-generated empty-desktop tiling layout. POC-03 should wait.

The earlier [live acceptance report](docs/live-acceptance.md) and its failed-run evidence are retained as historical results. Counts overlap and should not be summed.

DepthOS explores hierarchical Linux workspaces: Universe → worlds/projects → project desktops → the active desktop (Z3 → Z0). POC-01 is an independently packaged native KWin effect, `depthos-spatial-poc`. It displays an animated placeholder project card; it does not create or own real workspaces.

## Repository layout

| Path | Responsibility |
| --- | --- |
| `src/controller/` | Foreground controller handle, private worker, and lifecycle unit tests |
| `src/spatial/` | Read-only D-Bus bridge and bridge unit tests |
| `src/context/` | Context coordination, policy, and unit tests |
| `src/action_security/` | Offline authorization model and unit tests |
| `src/sqlite/`, `migrations/` | SQLite store, migration tests, and schema |
| `crates/depthos-observer/` | Observer protocol, replica, and transport |
| `experiments/` | KWin packages and policy prototypes |
| `tests/`, `scripts/`, `tools/` | End-to-end fixtures, validation commands, and local tooling |
| `docs/` | Architecture guides and historical qualification reports |

The [integration readiness report](docs/phase-3-integration-readiness.md) records the verified pre-reorganization snapshot. Its source inventory and rollback instructions refer to that snapshot.

## POC-02B — Native dynamic overview

The replacement overview source is in `experiments/kwin-overview`. It uses public KWin window/filter models and Workspace data directly for real window cards, desktop/Activity inspection, focus/state indicators, and conservative native previews. No hardcoded project data or external snapshot replica drives this renderer. POC-02A remains the independent diagnostic/cross-check pipeline; POC-01 is preserved for rollback.

Safe checks and local revision preparation:

```fish
fish scripts/validate-overview.fish
python3 tools/overview-dev.py prepare
```

The overview was installed, loaded and tested in approved runs, then removed. **Native POC-02A acceptance is a prerequisite to POC-02B activation**, and both integrations require scoped authorization. The helper prepares immutable content-addressed packages to avoid same-URL QML cache reuse and gates load/toggle on a same-session native observer acceptance record. Its new configurable shortcut has an empty default binding; POC-01's key remains untouched.

See [the POC-02B engineering report](docs/poc-02b.md) for architecture alternatives, API/source evidence, thumbnail limitations, exact fish commands, the acceptance gate, safe iteration/rollback, measured offscreen resources, and tests. `artifacts/overview-fixture.png` is a watermarked **offscreen fixture**, not a screenshot of the user's desktop. No real window activation, desktop switching, movement, ownership or POC-03 work is implemented.

## POC-02A — Native system discovery

The new `depthos-system-observer` is a separate, read-only JavaScript KWin script. It discovers real windows/desktops/Activities/screens, subscribes to native changes, and sends versioned snapshots/events to a foreground terminal inspector over authenticated local session D-Bus. No Rust daemon, database, workspace manipulation, shortcut, or POC-01 rendering dependency is added.

The [POC-02A engineering report](docs/poc-02a.md) contains the protocol, API findings, tests, performance limits, actual read-only preflight data, installation commands, manual acceptance A–G, and rollback. Safe validation:

```fish
fish scripts/validate-observer.fish
```

**Installing/loading the observer requires explicit approval.** The approved native test passed all eleven observer prerequisites (see `docs/poc-02a.md`). The overview subsequently passed a native lifecycle smoke test and was visibly confirmed on Desktop 1; controlled native card/thumbnail cases now pass; physical input and the remaining matrix cases are pending. Both new test packages were removed after testing. For future approved runs, use `install-observer.fish --install`, start `python3 tools/state-inspector/main.py` in a terminal, and run `observer-session.fish --start` in another. Immediate rollback is `observer-session.fish --stop`, followed by Ctrl+C in the inspector; `uninstall-observer.fish --remove` removes only the unloaded observer package. All fish scripts are run through `fish scripts/<name>` from the repository root. No enabled preference is written.

## POC-01 — Spatial overlay

The overlay uses a navy backdrop, violet selection, a 300 ms scale/fade/lift entrance, and a 230 ms exit. Escape, the toggle shortcut, or an outside click dismisses it. Left-click the card or press Space/Enter to toggle its preview selection. Selection stays in the overlay and performs no navigation.

## Requirements and status

Audited on Garuda Linux, Plasma/KWin **6.8.1**, Qt **6.12.0**, Wayland with Intel Mesa OpenGL compositing on 2026-11-15. The installer deliberately refuses other KWin versions until re-audited. Fish, `kpackagetool6`, and `qdbus6` are required. No build, CMake, Ninja, Rust service, database, or AI dependency is needed.

Package recognition, QML parsing, fish syntax, and isolated Qt Quick tests succeeded. The activation investigation corrected a Bound-context incompatibility and installed **0.1.1**. An early same-session test retained stale QML, as recorded below. Subsequent live logs, rechecked read-only during POC-02A, confirm shortcut callbacks, 1920×1080 delegate creation, and completed entrance/exit cycles. The latest POC-02B read-only inspection found the original effect unloaded; this task did not reload or activate it. The latest run documents a real static-card screenshot, 30 native cycles and reversal; physical input remains pending; **POC-01 is not fully approved**. See [the engineering report](docs/phase-1.md) and [compatibility evidence](docs/compatibility.md).

No open-source license has been approved. [LICENSE](LICENSE) records the pending decision; the metadata makes no license claim. Do not publish this as licensed open-source software yet.

## Inspect and validate

Run these from the repository in fish; both are non-disruptive:

```fish
cd /home/ashenix/Projects/DepthOS
fish scripts/inspect-kwin.fish
python3 tests/validate-poc.py
```

The optional test runner needs Python 3, `g++`, `pkg-config`, Qt 6 Quick/QML development headers, and the installed `/usr/lib/qt6/bin/{qmlformat,qmltestrunner}` plus QtTest. It copies production QML to a temporary directory, supplies **fake KWin types**, and runs Qt Quick offscreen with software rendering. A small C++ probe reproduces KWin's child-context delegate incubation; the former Bound pragma fails, and the corrected Unbound pragma succeeds. It never registers a global shortcut or contacts KWin to activate an effect. These are development-only dependencies; the effect itself still requires no build. Temporary files are automatically removed.

Standalone `qmllint` cannot fully check this effect because the public `org.kde.kwin` types are registered inside KWin rather than supplied as a standalone import on this machine. Do not treat its unresolved-type warnings as a successful full import check, or use `qmlscene` to load the real effect.

## Install and enable — requires your explicit approval

These are instructions, not actions already performed. Review the package and scripts before authorizing installation. Installation does not enable or load the effect:

```fish
fish scripts/install-poc.fish --install
```

The script checks the audited version, queries shortcut availability, refuses an existing DepthOS package, and uses the user-local KPackage root under `$XDG_DATA_HOME/kwin/effects` (default `~/.local/share/kwin/effects`). It never upgrades another package. A shortcut conflict or unknown availability aborts installation. Recheck conflicts immediately before loading: another action could claim the key after inspection.

For the first approved test, load **only for this session**, without changing the persisted enabled state:

```fish
qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.loadEffect depthos-spatial-poc
qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.isEffectLoaded depthos-spatial-poc
```

Loading registers `Toggle DepthOS Spatial POC`. The preferred default is **Meta+Ctrl+Shift+F11**, reported free by KGlobalAccel during the audit. Configure it in **System Settings → Shortcuts → KWin**. Keep any previously saved custom binding; do not accept reassignment of another application's shortcut. If the effect is not listed immediately in settings, close and reopen the settings page. Do not restart KWin or Plasma.

For persistence after successful testing, enable **DepthOS Spatial POC** in **System Settings → Desktop Effects** and Apply. Other effects must retain their existing settings. Enabling/loading does not open the overlay; use the shortcut.

## Live verification procedure

Keep a terminal accessible with the unload command below before the first test. First record `fish scripts/inspect-kwin.fish` and your window, desktop, and Activity state. Do not enter from Overview, Desktop Grid, or another fullscreen effect.

1. Load the package; confirm `isEffectLoaded` returns `true`. Check logs for component/import errors.
2. Press the assigned shortcut. Expect a fullscreen overlay on each output, a card starting near 0.75 scale, and a smooth entrance ending near 1.0 in 300 ms. Check layout at 1920×1080 and your normal display scale.
3. Click the card. Expect a violet selected border and “Selected · preview only”; the overlay remains open. Space/Enter also toggles selection.
4. Press Escape. Expect the card to shrink toward 0.85, fade, and return to the desktop after about 230 ms. Confirm normal keyboard and pointer input in an existing application.
5. Open again, then dismiss with the same shortcut; repeat using an outside click. Clicks inside the card must not dismiss or reach underlying apps.
6. Alternate the shortcut rapidly, including during both animations. Finish by dismissing. Repeat at least 30 normal cycles. Confirm no stuck input, residual overlay, flicker, or persistent visual corruption.
7. Where practical, test multiple outputs, fractional scaling, geometry changes, output removal, suspend/resume, and lock/unlock. These are unverified integration cases, not guarantees.
8. Compare the existing applications, four virtual desktops, one Activity, and other effects against the baseline. Windows must keep their state and ownership. `codex-glass`, `wallpaper-style`, and `wallpaper-style-transparency` must be unchanged (presence does not imply they were all loaded originally).
9. Unload and remove as below. Confirm the effect disappears while the session and other effects remain functional. Record results in `docs/phase-1.md` before marking acceptance complete.

## Activation repair: approved update and diagnostic test

**Historical live-test result, before the later successful lifecycle logs:** the approved upgrade completed, but unload/reload retained the old Bound component. The live journal again reported `Cannot instantiate bound component outside its creation context`; the new initialization diagnostics were absent. KWin 6.8.1 loads this fixed URL through its shared scripting QML engine, with no cache invalidation in effect unload/reload. No supported per-effect cache refresh was found. Repeating upgrades/reloads at this URL cannot be relied upon to refresh the source. The effect was left unloaded then; no logout or compositor restart was performed as an automated remedy. The more recent status is recorded above.

That earlier attempt stored its original 0.1.0 backup at `/tmp/depthos-approved-live-dvkzd8mu/depthos-spatial-poc` and scoped failure log at `/tmp/depthos-approved-live-dvkzd8mu/live-test.log`. These were temporary artifacts and may no longer exist after a reboot. The earlier cleanup preserved other custom effect files and loaded states.

For a fresh-session retest, the update step is already complete: start with session loading, registration inspection, and the diagnostic toggle below. Confirm the **new** `initialized; shortcut handler constructed` log before treating the running code as current. If only the old error returns, unload DepthOS and stop; neither physical-key testing nor repeated toggles can establish the fixed implementation's behavior in that state. The following update/rollback commands are retained for reproducibility; do not re-run an equal-version upgrade.

The original `ComponentBehavior: Bound` pragma is incompatible with KWin 6.8.1's child-context delegate creation. The production QML now explicitly uses `Unbound`. No additional metadata fields, separate script renderer, or persistent-enable change is required to address this defect. The package version is bumped from 0.1.0 to **0.1.1** because the installed KPackage 6.31.0 requires a strictly newer version for `--upgrade`. Offscreen tests reproduced the original error and pass with the correction; live acceptance is still pending.

The current installed package must be updated to run the fix. **Obtain explicit approval before executing this section:** it replaces only DepthOS, loads it temporarily, and captures the screen/input while the overlay is open. It does not persistently enable DepthOS. Review the source diff and keep a terminal available for rollback. Execute commands individually and stop on failure.

```fish
cd /home/ashenix/Projects/DepthOS
set -l poc_root "$HOME/.local/share/kwin/effects"
if set -q XDG_DATA_HOME; and test -n "$XDG_DATA_HOME"
    set poc_root "$XDG_DATA_HOME/kwin/effects"
end
set -l poc_backup (mktemp -d /tmp/depthos-poc-backup.XXXXXXXX)
cp -a -- "$poc_root/depthos-spatial-poc" "$poc_backup/depthos-spatial-poc"
printf 'Rollback package: %s\n' "$poc_backup/depthos-spatial-poc"

# Unload only DepthOS before replacing its files (safe even if already unloaded).
qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.unloadEffect depthos-spatial-poc
qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.isEffectLoaded depthos-spatial-poc
# Continue only if false. Backup must exist before upgrading.
kpackagetool6 --type KWin/Effect --packageroot "$poc_root" --upgrade "$PWD/experiments/kwin-effect"
diff -ru -- "$PWD/experiments/kwin-effect" "$poc_root/depthos-spatial-poc"
# Continue only if diff reports no differences.
qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.loadEffect depthos-spatial-poc
qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.isEffectLoaded depthos-spatial-poc
```

Now inspect registration separately from loading:

```fish
fish scripts/inspect-kwin.fish
qdbus6 org.kde.kglobalaccel /component/kwin org.kde.kglobalaccel.Component.shortcutNames | string match -x depthos-spatial-poc-toggle
qdbus6 --literal org.kde.kglobalaccel /kglobalaccel org.kde.KGlobalAccel.getGlobalShortcutsByKey 385876026
```

A recorded action can survive unloading; neither its presence nor a saved key proves a live callback. `depthos-spatial-pocEnabled=false` or an unset value controls automatic loading and does **not** prevent explicit session loading/activation.

For a temporary diagnostic trigger independent of pressing physical keys:

```fish
fish scripts/trigger-poc.fish --toggle
# Equivalent fixed-action invocation:
# qdbus6 org.kde.kglobalaccel /component/kwin org.kde.kglobalaccel.Component.invokeShortcut depthos-spatial-poc-toggle
```

This script only invokes the already registered DepthOS action. It does not load the effect, register new shortcuts, or change preferences. It is development-only and reversible by Escape, a second invocation, or unloading DepthOS. It bypasses physical keyboard delivery but intentionally exercises the same `onActivated` callback. No public D-Bus setter for this QML instance's `visible` property was found. **Do not use `/Effects.toggleEffect` to open the overlay:** that API toggles plugin loading, not scene visibility.

Diagnostic logs use the temporary prefix `[depthos-spatial-poc]`. The expected stages are `initialized`, `shortcut callback`, `visible request=true`, `screen delegate created`, `entrance finished`, then `exit finished` and `visible request=false`. Delegate creation and visibility logs may interleave during incubation. None establishes that pixels actually appeared; observe the overlay and desktop input separately.

```fish
set -l poc_kwin_pid (pgrep -n -x kwin_wayland)
journalctl --user -b _PID="$poc_kwin_pid" --since '10 minutes ago' --no-pager | rg 'depthos-spatial-poc|Cannot instantiate bound|Could not create a view|ReferenceError|TypeError'
qdbus6 org.kde.KWin /Effects org.freedesktop.DBus.Properties.Get org.kde.kwin.Effects activeEffects
```

If invocation produces a callback/delegate/overlay but the key does not, investigate the saved binding, Fn/F11 translation, keyboard layout, and keyboard interception. If there is no callback, recheck the loaded state and KGlobalAccel registration. If the callback runs without a delegate, inspect QML errors and competing fullscreen effects. Never disable other effects automatically. Finish with the manual test matrix above, then unload DepthOS to return to the original session state.

Immediate rollback is `qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.unloadEffect depthos-spatial-poc`. To restore the original package as well, after confirming it is unloaded:

```fish
# KPackage rejects downgrades through --upgrade. Check the backup before removal.
kpackagetool6 --type KWin/Effect --show "$poc_backup/depthos-spatial-poc"
kpackagetool6 --type KWin/Effect --packageroot "$poc_root" --remove depthos-spatial-poc
kpackagetool6 --type KWin/Effect --packageroot "$poc_root" --install "$poc_backup/depthos-spatial-poc"
diff -ru -- "$poc_backup/depthos-spatial-poc" "$poc_root/depthos-spatial-poc"
```

Keep the backup until the fixed package is accepted; `/tmp` may be cleared on reboot. The original package contains the known context defect, so leave it unloaded after rollback. No other files or KDE preferences need changing. The installer remains intentionally first-install-only; this approved, scoped upgrade procedure preserves any saved DepthOS shortcut.

## Disable, troubleshoot, and remove

Escape normally exits the overlay. If it fails, unload just this effect from a terminal or another available session:

```fish
qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.unloadEffect depthos-spatial-poc
qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.isEffectLoaded depthos-spatial-poc
```

Uncheck **DepthOS Spatial POC** in Desktop Effects and Apply if you enabled it persistently. Unloading alone is session-only. Once `isEffectLoaded` returns `false`, remove the user-local package:

```fish
fish scripts/uninstall-poc.fish --remove
```

Removal refuses a loaded effect, unknown D-Bus state, or a symlink package. It only removes `depthos-spatial-poc` from the explicit user root and never recursively deletes a supplied path. It leaves KDE's inert disabled-state/shortcut preferences intact. For complete preference cleanup, after disabling, unloading, and removing the package, these optional commands target only DepthOS:

```fish
qdbus6 org.kde.kglobalaccel /kglobalaccel org.kde.KGlobalAccel.unregister kwin depthos-spatial-poc-toggle
kwriteconfig6 --file kwinrc --group Plugins --key depthos-spatial-pocEnabled --delete
```

Run preference cleanup only with explicit approval; it removes any saved custom binding. Do not delete `kwinrc`, `kglobalshortcutsrc`, or other effect directories. Use the approved update sequence above for an existing installation; the installer intentionally has no automatic upgrade behavior.

For failures, inspect read-only logs and state:

```fish
journalctl --user -b --since '10 minutes ago' --no-pager | rg -i 'depthos|qml|kwin|libkwineffects'
qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.isEffectSupported depthos-spatial-poc
qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.loadedEffects
```

Journal routing varies; an empty user journal does not establish absence of errors. KWin logs QML view failures under `libkwineffects` and scripting issues under `kwin_scripting`/`kwin_core`. If another fullscreen effect or a keyboard grab prevents startup, exit that effect, toggle DepthOS off, then try opening it again. Do not kill/restart the compositor or log out as an automated recovery step.

POC-02 is only a future investigation into compositor-managed thumbnails and inactive-desktop previews. It is not implemented here.
