# POC-02B — Native dynamic spatial overview

**IMPLEMENTED — FULL LIVE ACCEPTANCE PENDING** — updated 2026-11-16.

The [complete live acceptance report](live-acceptance.md) supersedes historical pending statements below: the final revision passed 22 native checks, 30 cycles, reversal, active unload, and actual screenshots for controlled normal/fullscreen/minimized/protected/sticky/multiple-membership previews and card removal. A fallback label that overlapped real frames was repaired and a regression added (17 Qt checks now pass). All test packages/processes/actions were cleaned and baseline configuration hashes match. Physical input/native Z2/Z3 navigation, frame pacing and extended reliability remain pending.

## Prerequisite and current evidence

POC-02A's documented safe checks were run before implementing the renderer: 19 production-JS tests, 11 replica tests, the Qt 6.12.0 engine harness, and the isolated session-bus receiver harness pass. The user subsequently approved native prerequisite testing and conditional overview testing. All eleven POC-02A prerequisites passed with actual Konsole addition/removal, focus, desktop switching, membership changes, reconciliation and clean unloading, plus matching preserved-effect files and loaded-effect set. A manual observer session lifecycle hazard was repaired; see `poc-02a.md`. The approved versioned overview was then installed, loaded, activated, exercised and removed.

The source and offscreen tests are complete. The integration helper retains its native prerequisite gate for each compositor session. POC-02B must not be activated until native POC-02A acceptance is recorded for the current compositor session and source revision. If native observer testing fails, repair/retest that pipeline first. No mock test is treated as a substitute.

POC-01 and the existing custom effects are preserved. Previous successful POC-01 lifecycle logs remain valid evidence of initialization/animation callbacks, not independent physical visual/input or stress acceptance.

## Architecture decision

| Option | Finding | Decision |
| --- | --- | --- |
| A. Direct native QML models | Public `WindowModel`, `WindowFilterModel`, `Workspace` and `WindowThumbnail` already expose live compositor objects and events | Use for rendering |
| B. Shared KWin-side observer state | No supported shared arbitrary-object registry between a standalone script QJSEngine and the shared QML effect engine was found | Do not invent a singleton bridge |
| C. External controller → effect | The receiver can accept `callDBus`, but arbitrary Python objects cannot directly mutate a `SceneEffect`; a new bridge/control boundary would need validation | No new controller/daemon |
| D. Hybrid | Native models own renderer input; existing POC-02A protocol independently supplies diagnostic/cross-check state | Selected architecture |

This connects the milestones through their common authoritative native KWin objects and acceptance checks, **not** by copying serialized windows into the effect. The renderer never interprets a diagnostic window ID as a native QObject. Restarting/disconnecting the observer leaves the native model live; the UI explicitly labels the observer as independent rather than pretending a transport is connected. The POC-02A consumer retains its existing session/revision/gap-resynchronization logic. The overview is not a new consumer of that protocol, so it has no duplicate sequence counter or stale external snapshot cache.

| State field | Authority / update path |
| --- | --- |
| Existence, membership, source rows | Native `WindowModel` insert/remove/dataChanged; public proxy filters |
| Caption, app ID, QIcon, minimized/fullscreen, capture restriction | Window QObject properties and NOTIFY signals, bound directly to cards |
| Focus | `Workspace.activeWindow` / windowActivated |
| Desktop objects, names, current desktop | `Workspace.desktops`, `currentDesktop`, per-output `currentDesktopForScreen`, desktop name NOTIFY |
| Activity IDs/current Activity | `Workspace.activities`, `currentActivity` |
| Output list/name/geometry/scale | `Workspace.screens`, `SceneView.screen`, native logical-output properties |
| Selected context and intended destination IDs | Effect-local presentation state only; never written to Workspace |
| Observed diagnostic replica | POC-02A observer → same-UID unique session-bus receiver → existing replica reducer |

The overview intentionally includes native **normal and dialog** clients. POC-02A's default terminal collection is narrower (normal applications excluding skip-taskbar/exclude-from-capture); its all-window diagnostic state is the appropriate comparison, not a promise of identical default counts. System-only native window types are excluded from the card grid. No project examples, invented application counts, or workspace ownership are in runtime data.

## Implemented interaction and rendering

- Z0 is the ordinary desktop after closing the effect.
- Opening Z1 backs away from a bounded native surface composition in actual output-relative positions while introducing the native window grid. Public Desktop/Dock surfaces are included when exposed and eligible; missing wallpaper/panels/private surfaces are not synthesized. The composition is a limited native surface layer, **not a guaranteed pixel-exact copy of the entire desktop**.
- Z1 shows normal/dialog windows on the locally selected native desktop and Activity, with actual app identifiers, optional titles, QIcon/theme or generic icon fallback, memberships, focused/minimized/fullscreen indicators, and eligible native previews.
- Z2 shows the real desktop objects and names, marking the actual active desktop separately. Choosing a desktop records local intent and drills into its windows without switching the real desktop.
- Z3 shows actual Activity IDs where available. No Activity display name is invented from its UUID. Choosing one filters local inspection and returns to desktop inspection; it never switches the actual Activity. If Activities are unavailable/empty, the maximum depth is Z2.

Arrow keys move the local selection, Enter/Space inspect/select, minus/Backspace zoom out, plus/equal zoom in, and Escape closes. Mouse hover/click and visible zoom/close controls provide the same local operations. Clicking the backdrop dismisses. Selecting a window records only a local identifier; there is no real activation or window movement action in this milestone.

The effect uses a 300 ms entrance and 230 ms exit, interrupted from current values. Visibility remains true through exit and becomes false only on the current transition's completion; reversing it stops the previous animation. Layer navigation adds a short restrained scale/fade. A reduced-motion toggle skips animation; Kirigami's zero-duration global animation setting is also respected. No particular FPS is claimed.

Titles are hidden on cards by default and can be enabled locally. Protected/unknown-capture windows never display the caption through this control. **Card-title hiding does not redact the contents or titlebars inside eligible thumbnails**; disable previews as well when that is desired. No titles, screenshots, or native window objects are sent to Python by the renderer. No history or global preference is written.

All metadata labels explicitly use `Text.PlainText`, so markup-looking titles cannot become rich-text images or links. Displayed window strings are bounded and control/bidi characters are replaced for presentation. Native data remains authoritative; no string is executed or used as a URL/command.

Missing native source/selected desktop clears the grid and displays an unavailable state, rather than showing cached cards. An empty valid desktop/Activity filter is distinguished from unavailable state. Native removal destroys row delegates; a removal notification also clears local window intent without dereferencing the removed object. Deleted selected desktops/Activities are reconciled against current native lists. Output delegates use logical geometry and native thumbnail scaling, and closing any output view deactivates the effect globally.

## API and thumbnail findings

Source verification uses KWin **v6.8.1**, installed headers/types, and Qt **6.12.0**, not documentation for an assumed older version. Links and the role/filter details are in [compatibility.md](compatibility.md#poc-02b-native-model-and-thumbnail-audit).

`WindowModel` subscribes to additions/removals before initializing its native list. It forwards desktop/output/Activity changes as role changes. `WindowFilterModel.windowModel` accepts a WindowModel, `desktop` a VirtualDesktop object, `activity` a string, `screenName` a logical-output name, `windowType` the Normal/Dialog/etc. flags, and `minimizedWindows` controls visibility. The role `window` carries the native Window object. The installed Qt proxy treats source role changes as relevant to filtering. Caption/focus/state bindings use Window/Workspace NOTIFY directly; there is no full-list polling per frame.

`WindowThumbnail.client` accepts that native object; `wId` is a QUuid, not an X11 ID. Its source maintains QPointers and updates from damage/geometry with GL resources tied to the view/client. The inspected thumbnail path does not supply a public texture-ready property, and the source path inspected does not itself provide the capture-exclusion guard this UI needs. Therefore:

| Case | Implemented policy | Real compositor validation |
| --- | --- | --- |
| Active desktop/Activity, known capture-allowed, non-minimized | Request a native WindowThumbnail, subject to budget | Pending |
| Inactive selected desktop | Honest metadata-only card; no request to force an inactive surface | Pending metadata comparison |
| Inactive Activity | Metadata-only; no forced capture | Pending metadata comparison |
| Minimized or unknown minimized state | Metadata-only | Pending |
| Fullscreen active/capture-allowed | Native preview may be requested; state indicator retained | Pending |
| Sticky/multi-desktop | Native membership filter; preview only if also on actual active desktop/Activity | Offscreen policy passed; native pending |
| Excluded from capture or flag unknown | No thumbnail component instantiated; caption hidden | Offscreen policy passed; native pending |
| Missing icon/title/native frame | Generic icon/identifier and explicit preview-request/fallback text | Offscreen passed; native pending |
| Closing while preview exists | Native row removal/QPointer and Loader teardown; no frozen screenshot substitute | Offscreen removal passed; GL pending |
| Multiple outputs/scales | Per-output native view/geometry; no synthetic screenshots | Offscreen geometry passed; GPU/native pending |

The preview area retains fallback text behind the native item. It states that a frame may not yet be available; allocating the item or reading its implicit size is **not** presented as proof that a texture rendered. No screenshot portal, private WindowHeap, custom shader, or unsupported inactive-window force-rendering is used.

GridView virtualizes card delegates and limits requests to the first six filtered rows per view. Higher rows remain metadata-only. During the entrance/exit, a native desktop composition may request another six sources per output (prioritizing the focused window and first eligible rows); KWin may share sources for identical view/client pairs. Each output's actual desktop is queried for surface eligibility and the desktop composition, with global currentDesktop as a fallback; global active desktop, output desktop and local inspection are distinguished. The composition Loader is destroyed at steady Z1. Thumbnail textures can be full window resolution regardless of card size, so these are deliberate resource bounds, not a GPU memory estimate.

## Repeatable iteration without compositor restart

The source package stays under `experiments/kwin-overview`; POC-01's installed package is not overwritten. `tools/overview-dev.py prepare` copies a reviewed source revision into ignored local artifacts with ID **`depthos-overview-dev-<12 hex source hash>`**, and sets the disabled-by-default metadata accordingly. Relative component URLs also lie in this unique directory. Existing prepared revisions cannot be overwritten if modified. Installation refuses an existing revision instead of upgrading a cached URL.

The shared-engine regression probe verifies both sides: replacing a file at the same URL remains cached, whereas a new package directory plus relative component loads into that already-used engine. This is an installed-Qt test, not proof of a complete live KWin upgrade. Old loaded revisions must be explicitly unloaded/removed; the tool refuses loading another overview revision concurrently. It never flushes global caches or restarts any process.

All revisions use **one** configurable action, `depthos-spatial-overview-toggle`, with an empty default binding. This avoids accumulating per-revision shortcut actions and preserves POC-01's action/binding. Invoke the action through the helper for an approved test, or assign a free shortcut manually in KDE settings after checking conflicts. No global key is reassigned by code. An inert saved binding may remain after final removal; optional cleanup of this one action is explicit below.

## Exact safe checks and approved runtime sequence (fish)

Safe local work, already performed:

```fish
cd /home/ashenix/Projects/DepthOS
fish scripts/validate-observer.fish
fish scripts/validate-overview.fish
python3 tools/overview-dev.py prepare
python3 tools/overview-dev.py status
```

`prepare` writes only ignored local artifacts and a **PENDING** template. It does not install or load anything. `status` only inspects this overview's revision namespace. The offscreen fixture image at `artifacts/overview-fixture.png` is explicitly watermarked as synthetic test data, not a live desktop or successful native preview.

### First: separately approved POC-02A acceptance

Follow the complete [POC-02A acceptance A–G](poc-02a.md#safe-manual-acceptance-ag). In particular, native install/load, actual initial snapshot/existing windows, added/removed windows, focus, desktop switching and membership, snapshot reconciliation/recovery, clean unload, and unchanged unrelated state must be evidenced. The agent does not create/move/close test windows automatically; manual user actions are required for those checks.

```fish
# Only after explicit approval:
fish scripts/install-observer.fish --install
# Terminal A: foreground receiver
python3 tools/state-inspector/main.py
# Terminal B: only this observer, never global Scripting.start
fish scripts/observer-session.fish --start
# After the manual event/recovery checks:
fish scripts/observer-session.fish --stop
qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.isScriptLoaded depthos-system-observer-session
```

Use the inspector's redacted `--snapshot` export as documented in POC-02A for an actual data sample. Do not copy mock results into a native acceptance record. After all checks actually pass, copy `artifacts/observer-acceptance.template.json` to `artifacts/observer-acceptance.json`, mark only evidenced checks true, set status PASS, and record evidence references. The template is regenerated PENDING by prepare; the actual record is never overwritten. Record these real values as `compositorOwner`, `sessionBusId`, and `observerSourceHash`:

```fish
qdbus6 org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus.GetNameOwner org.kde.KWin
qdbus6 org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus.GetId
sha256sum experiments/kwin-observer/contents/code/main.js
```

The helper rejects missing/PENDING/partial records, changed observer source, a changed compositor owner, or a different session bus. The record is an operator evidence gate, not a cryptographic proof or substitute for approval. A genuine same-session PASS record was subsequently created after native checks; see the later acceptance reports.

### Then: separately approved POC-02B test

Only after the native prerequisite passes and the new overview test is explicitly approved:

```fish
python3 tools/overview-dev.py install
python3 tools/overview-dev.py load
python3 tools/overview-dev.py toggle
```

Install alone does not load. Load alone does not activate. Toggle targets only the stable overview action. No permanent enabled preference is written. The existing POC-01 effect may remain loaded but must not be actively fullscreen during the test. Do not disable unrelated effects if startup is blocked; exit the competing fullscreen mode manually and investigate.

Keep a terminal available with the emergency command below. Verify actual window discovery against the observer's diagnostic all-window data and native KDE; manually open/close only a test Konsole, change focus and existing-desktop membership/switching, verify titles/states/privacy on that test window, traverse Z1/Z2/Z3 by keyboard/mouse, confirm selection never changes native state, exercise interrupted open/close and at least 30 ordinary cycles, then confirm normal application input after Escape/outside-click/toggle dismissal. Test captured/protected, minimized, fullscreen, sticky, inactive and multi-output scenarios per the table. Do not infer real textures from item creation.

Restart the observer using its scoped lifecycle and confirm the native overview remains current while diagnostic state obtains a new session/baseline. Record visual/input and live event results. All native acceptance remains pending until these are actually performed.

### Emergency rollback and revision cleanup

```fish
# Safe escape hatch: only the current overview revision, no acceptance file needed.
python3 tools/overview-dev.py unload
python3 tools/overview-dev.py remove
python3 tools/overview-dev.py status
```

Rollback reads the last prepared manifest without parsing edited source. If a newer prepare replaced that manifest while an older revision is still loaded, select the one loaded revision explicitly:

```fish
set -l old_overview_id (python3 tools/overview-dev.py status | string match -r -g '^(depthos-overview-dev-[0-9a-f]{12}) loaded=True$')
if test (count $old_overview_id) -eq 1
    python3 tools/overview-dev.py unload --id "$old_overview_id[1]"
    python3 tools/overview-dev.py remove --id "$old_overview_id[1]"
end
```

For a direct emergency alternative after obtaining that same `old_overview_id`, run `qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.unloadEffect "$old_overview_id[1]"` only when exactly one ID was returned. Cleanup accepts only this tool's revision namespace/ownership metadata, refuses symlink packages, and refuses removing a loaded effect. It never removes POC-01, the observer, or another effect. After each edit: prepare → inspect old revisions → explicitly unload/remove the old revision → approve/install/load the new one. Local ignored artifact directories may be retained for review; they are not installed packages.

If the observer was installed for acceptance, stop it, stop the receiver with Ctrl+C, and remove only its unloaded package via `fish scripts/uninstall-observer.fish --remove`. If intentionally removing the overview's saved shortcut after all its packages are unloaded/removed, the optional separate command is:

```fish
qdbus6 org.kde.kglobalaccel /kglobalaccel org.kde.KGlobalAccel.unregister kwin depthos-spatial-overview-toggle
```

That deletes only this new action/binding and requires explicit approval. No existing configuration file is deleted or reset. No restart, logout, privileged action, window operation, Activity or desktop modification is part of rollback.

## Verification and performance report

| Check | Result |
| --- | --- |
| POC-02A safe prerequisite suite | 19 JS tests + 11 replica tests, Qt-engine and private-bus harnesses passed; native prerequisite passed; retested in live-acceptance.md |
| New package/iteration tests | **7 passed**: immutable preparation, content changes, pending/same-session gating, namespace/symlink cleanup, stable empty-default shortcut and source-independent emergency unload |
| New offscreen QtTest suite | **16 passed** (14 functional/capture cases + init/cleanup), production QML with mocked native boundary |
| Child-context, same-URL cache and new-directory URL probes | Passed with installed Qt 6.12.0 |
| Package recognition, QML parsing, fish/Python validation | Passed |
| Existing POC-01 regressions | **10 passed**, with the extended new-directory cache probe; original effect source unchanged |
| Real native model/texture/input integration | Controlled native model/texture matrix passed in the later acceptance run; physical input remains pending; see live-acceptance.md |

Coverage includes initial/empty/multiple windows, additions/removals, actual QObject-property title/state/focus changes in fixtures, desktop and Activity filtering/membership, missing metadata, thumbnail policy/fallback, unavailable native source, local selection, keyboard/mouse controls, cancellation/reversal/reduced motion, geometry and two output views. Existing POC-02A tests cover protocol gaps/reconnection/reconciliation. They are not relabeled as renderer transport tests: this renderer has no observer transport to disconnect.

The latest measured offscreen QtTest process used software rendering and mocked native types: **1.35 s elapsed, 0.20 s user CPU, 0.03 s system CPU, 78,764 KiB peak RSS** in one run (including test waits, Qt/Kirigami initialization and fixtures). These are process-level fixture measurements, not compositor overhead, GPU memory, interactive latency or FPS. The runner reports measurements per run. Native CPU/GPU/frame pacing measurements are pending and are required before asserting smoothness on Intel graphics.

## Files and remaining work before POC-03

Added `experiments/kwin-overview/metadata.json` and its `contents/ui/{main.qml,OverviewScene.qml,WindowCard.qml,DesktopCanvas.qml,Action.qml,Policy.js}`, `tools/overview-dev.py`, `scripts/validate-overview.fish`, and `tests/overview/` fixtures/runner/package tests. Extended the existing `tests/delegate-context.cpp` cache probe without changing POC-01 source. Updated README, architecture and compatibility documentation, plus the POC-02A forward reference; existing engineering reports are retained. Prepared packages and preview images are ignored local artifacts. No license was fabricated.

Remaining limitations: native prerequisites and full live acceptance are not complete; Activity names are unavailable through this chosen public interface; inactive/minimized/protected surfaces deliberately remain metadata-only; no public texture-readiness signal is assumed; surface budget/normal-dialog policy may differ from a desktop screenshot/task switcher; native multi-output/hotplug/scaling/privacy/teardown must be tested; system-wide motion behavior is observed only through public Kirigami duration; no physical shortcut is assigned automatically. No workspace ownership, routing, actual application activation, desktop switching, AI, database or Rust daemon was added.

Before POC-03, finish POC-02A acceptance, then approve and document POC-02B native rendering/input/privacy and iteration cleanup, measure actual overhead, and resolve any native API discrepancies. POC-03 has not been started.

## Approved native overview run — partial acceptance, 2026-11-15

Revision `depthos-overview-dev-65b251f72362` was installed from the immutable prepared package after genuine POC-02A acceptance was recorded. KWin confirmed it loaded and listed it among active effects after invoking only `depthos-spatial-overview-toggle`. The user confirmed the overview opened and showed Desktop 1. No matching DepthOS/QML TypeError, ReferenceError or assignment/type error appeared in the checked KWin journal interval. Absence of log errors does not prove every visual element rendered correctly.

At the user's request to automate the remaining test, the native virtual desktop D-Bus property returned the session to its existing Desktop 1. One separate disposable Konsole was launched with a 20-second Python sleep command in `/tmp`; its command exited normally and the Konsole process exited with status 0. Native observer evidence recorded application events, and the user reported Konsole appeared. That response does not independently establish whether the user meant the application itself or its overview card, so detailed card insertion/removal and thumbnail acceptance remain pending. The automated test targeted no existing application with a window movement, activation or termination action. Earlier window movement and focus tests were performed manually by the user.

Automated native state checks confirmed: closed before the sequence, inactive after six rapid toggles, active after a settled open, and inactive after a settled close. A 120-second independent scoped unload guard was armed during testing and cancelled before final explicit cleanup. The guard and rapid-toggle results establish bounded lifecycle behavior, not measured frame smoothness or physical input recovery. Local evidence is in ignored private `artifacts/live-02b/runtime-checks.json` and `cleanup.json`.

Cleanup confirmed both new packages removed, both observer runtime IDs unloaded, receiver bus name released, initial loaded-effect set restored, preserved custom/POC-01 file hashes unchanged and the compositor owner unchanged. The new unbound `depthos-spatial-overview-toggle` registration persisted after unload; it was explicitly removed using the scoped `org.kde.KGlobalAccel.unregister("kwin", "depthos-spatial-overview-toggle")` method. The original POC-01 action remains registered. No KWin/Plasma restart, logout, privileged action, enabled preference or existing shortcut modification was performed.

**Remaining live acceptance:** explicit Brave/Codex card identity/count and dynamic card removal, focused indicator, keyboard/mouse navigation and Escape input recovery, thumbnails and their privacy/fallback cases, multiple outputs/fractional scale/hotplug, full animation cancellation and continuous-use behavior. The overview remains **IMPLEMENTED — LIVE VALIDATION PENDING** for full acceptance. The bounded native smoke test above is successful; mocks and active-effect flags are not substituted for these outstanding checks.

To repeat the already prepared revision after explicit approval, from the repository root:

```fish
fish scripts/install-observer.fish --install
# In a separate foreground terminal:
python3 tools/state-inspector/main.py
# Back in the control terminal:
fish scripts/observer-session.fish --start
# Revalidate native prerequisites if the compositor session/source changed.
python3 tools/overview-dev.py install
python3 tools/overview-dev.py load
python3 tools/overview-dev.py toggle
# Scoped rollback/cleanup:
python3 tools/overview-dev.py unload
python3 tools/overview-dev.py remove
fish scripts/observer-session.fish --stop
# Ctrl+C in the foreground inspector terminal.
fish scripts/uninstall-observer.fish --remove
```

Before POC-03, finish these physical acceptance checks and the remaining native preview/output matrix. Workspace ownership, routing, AI and a Rust orchestration daemon remain outside this milestone.
