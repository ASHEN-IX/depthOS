# Compatibility and API evidence

## POC-02G — KWin 6.8.3 isolated qualification, 2026-11-16

[The version-specific audit and native report](poc-02g.md) records **6.8.3: ISOLATED_QUALIFIED**, retaining **6.8.1: HISTORICALLY_VALIDATED**. All 104 local checks and 72 final isolated-native assertions pass; watchdog recovery, private process cleanup and exact host comparisons pass. The API matrix distinguishes unchanged contracts, screenshot orientation/input-device ownership changes, source-only cases and pending host/hardware gates. Production files are unchanged.

Installed 6.8.3 / running 6.8.1 still blocks host preparation. A matching 6.8.3 pair also remains blocked by the explicit isolated-only scope until host-only gates and fresh session observer acceptance are reviewed; portal consent is independent. No wildcard or automatic host approval was added. Phase 1 remains partially accepted.

## POC-02F installed/running version preflight

The [structured compatibility gate](poc-02f-version-preflight.md) separately reads the installed CLI and active compositor D-Bus versions. Only matching, audited **6.8.1** currently passes the host version-preparation gate; this is not full host acceptance. The current **installed 6.8.3 / running 6.8.1** mismatch blocks preparation; 6.8.3 now has the separate isolated qualification above, which does not expand host mutation approval. Cleanup is not gated on version compatibility. Historical 6.8.1 evidence is preserved.

## POC-02F host portal boundary — 2026-11-16

[POC-02F](poc-02f.md) confirms the actual host is KWin 6.8.1 / Qt 6.12.0, Intel Mesa 26.2.5 OpenGL ES via EGL, with NVIDIA also available. RemoteDesktop v2/device mask 7 and ScreenCast v6 were queried. CreateSession/SelectDevices/SelectSources completed, but Start returned no authorization response within 180 seconds. ConnectToEIS and PipeWire frame consumption were not reached; the prepared post-consent host paths are unexecuted. No private KWin input fallback or persistent grant was used. Exact host configuration and original windows were restored.

The POC-02E marker PNG previously described as partial contains full terminal data: Qt/FFmpeg RGB decodes match and OCR recognizes its fixed marker/acknowledgement. The earlier viewing discrepancy is not evidence of a confirmed compositor capture defect. No rendering workaround was applied; a consented host comparison and presentation timing remain pending.

## POC-02E native automation compatibility — 2026-11-16

[POC-02E](poc-02e.md) verifies KWin 6.8.1 / Qt 6.12.0 / libei 1.6.0 using a private D-Bus, home/runtime and virtual OpenGL ES compositor. Real EIS keyboard/absolute-pointer/button events reach production Qt Quick scene handlers; the added scene probe reads only geometry/state. All private EIS and screenshot permission overrides are restricted to that child environment and are never set on the host. Private endpoint requests needed all conventional capabilities to negotiate both devices here; the sender binds only its required capabilities. Unique package paths are installed before virtual compositor startup.

ScreenShot2 capture uses one producer. The historical partial-terminal PNG diagnosis was corrected by independent Qt/FFmpeg decoding and OCR in POC-02F; it is not a confirmed rendering defect. Fresh recorder frames supply gate captures. Marker acknowledgement proves input delivery independently; host restoration visuals remain untested. AT-SPI GI and Spectacle recording are available, but neither provides acceptance by mere installation. The host advertises RemoteDesktop v2/devices 7; host input still requires explicit portal Start consent and has not been tested. Source reviews of waytool, wdotool and kwin-mcp are pinned in the report; none was installed. Physical input, display presentation timing and protected-native-client runtime acceptance remain pending. The host preserved effects, bindings, package hashes, desktop/Activity membership and exact tiling/configuration on the final complete comparison.

## POC-02D current interaction boundary — 2026-11-16

See [poc-02d.md](poc-02d.md). The unchanged immutable production revision rendered two native cards and logged actual Escape delivery followed by successful dismissal. This does not complete ordinary typing restoration: no fixed `depthos-02d-ok` acknowledgement arrived before the controlled programs ended. Native desktop-control APIs remained disabled, and no virtual input device or X11 injection was substituted.

Current DepthOS-only contention again verified the bounded request reset and successful later explicit activation, without terminating a competing effect. The implementation supplies a fixed diagnostic log when startup is refused; no external/on-screen unavailable notification was added. This is a bounded transient limitation with user-perceived feedback still pending, not an unconditional activation guarantee.

The new operator harness compares only the exact marker, retains no non-marker input, and has a run-pinned watchdog with guarded known-empty tiling recovery. The recovery planner requires an exact original whole-file hash and refuses unrelated changes or nonempty/unknown layouts. Recovery was not needed in this run: all 12 cleanup checks and both exact configuration hashes matched on the first final attempt. Presentation timing and a quiet preview workload remain unmeasured.


## Latest native stabilization — POC-02C, 2026-11-16

The [POC-02C report](poc-02c.md) supersedes historical pending-runtime statements below. Real native Z1/Z2/Z3, inactive-desktop fallback and programmatic reduced motion were captured. Physical keyboard/mouse and frame-pacing acceptance remain pending.

KWin's SceneEffect `visible` property is a request. `QuickSceneEffect::startInternal()` may decline startup for fullscreen contention or a keyboard-grab failure without constructing output delegates. The dynamic overview now resets a still-unfulfilled request after 1000 ms with no output constructed, without automatic retry. Actual DepthOS-only contention recovered on the next explicit invocation; an unsettled desktop-transition trial remained declined. This guard is not a texture-readiness signal or a guarantee of activation during other fullscreen work.

`RenderLoop::framePresented` supplies C++ presentation timestamps, but no supported public QML/session-D-Bus collector was exposed in this environment. Offscreen QQuickRenderControl completion is not display presentation. No frame-time/FPS guarantee follows from the display's refresh rate.

The enabled tiling integration retained a generated tile layout on the empty test desktop. A one-key substitution reproduced the exact baseline whole-file hash; guarded `Workspace.rootTile(output, desktop)` / `CustomTile.remove()` calls restored only that empty generated tree, without changing a user window or overwriting configuration. Final configuration hashes match. Future test baselines capture scoped tiling preferences and POC-02C setup rejects saved nonempty layouts. Arbitrary existing layouts must not be flattened as cleanup.

The source contracts were checked against KWin v6.8.1 `scripting/workspace_wrapper.h`, `tiles/tile.h`, `tiles/customtile.h/.cpp`, `core/renderloop.h` and `effect/offscreenquickview.cpp`; links and evidence are in the current report. No compositor restart, invasive profiler or private API bridge was used.


Audit date: **2026-11-15**, on the target machine. **FIX IMPLEMENTED — LIVE VALIDATION PENDING**.

## POC-02B native model and thumbnail audit

**IMPLEMENTED — LIVE VALIDATION PENDING.** The new renderer is source-audited for KWin 6.8.1 with the installed Qt 6.12.0 and Kirigami. No new effect or observer was installed/loaded for this audit. Native POC-02A acceptance is still required before POC-02B activation.

| API | Matching evidence and use |
| --- | --- |
| `WindowModel` | [v6.8.1 header](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/windowmodel.h), [implementation](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/windowmodel.cpp): QAbstractListModel, native window/output/desktop/Activity roles, insert/remove connections installed before enumeration, membership role notifications |
| `WindowFilterModel` | Same source: native `windowModel`, VirtualDesktop `desktop`, QString `activity`/`screenName`, Normal/Dialog/Desktop/Dock flags, `minimizedWindows`; native membership predicates handle all-desktop/all-Activity semantics |
| Filtering role changes | [Qt 6.12 proxy implementation](https://code.qt.io/cgit/qt/qtbase.git/tree/src/corelib/itemmodels/qsortfilterproxymodel.cpp?h=v6.12.0): default `dataChangeRelevanceFlags()` treats source changes as relevant to filtering, including the KWin membership roles |
| `Workspace` singleton | [workspace_wrapper.h](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/workspace_wrapper.h): public QML singleton exposes native desktop/Activity/output lists, current properties, activeWindow and read-only `currentDesktopForScreen(output)`; only reads/NOTIFY bindings and that query are used |
| `WindowThumbnail` | [header](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/windowthumbnailitem.h), [implementation](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/windowthumbnailitem.cpp): `client` is Window*, `wId` a QUuid, GL/damage/view lifecycle and QPointer source references. Source inspected supplies no public texture-ready property; application enforces capture-exclusion policy before instantiating it |
| Scene/input/lifecycle | Existing SceneEffect/ShortcutHandler/SceneView source evidence remains applicable. Delegates are Unbound and pass controller/system/model/output explicitly into separate QML components. Root dismissal remains available if the inner Loader fails |
| Icons/reduced motion | Installed `KirigamiPrimitives.qmltypes` confirms Icon's QVariant source/QIcon support, fallback and valid status. Public Kirigami Units duration is read; zero duration and the explicit local reduced-motion control disable animation |

The current [WindowModel](https://api.kde.org/qml-org-kde-kwin-windowmodel.html), [WindowFilterModel](https://api.kde.org/qml-org-kde-kwin-windowfiltermodel.html) and [WindowThumbnail](https://api.kde.org/qml-org-kde-kwin-windowthumbnail.html) references agree on the public approach. One documentation detail should not be assumed: the display role is described as a string, while the inspected 6.8.1 implementation returns a native Window pointer for display/window roles. The renderer deliberately uses **`window`**, not a purported display-title string.

Inactive desktop/Activity and minimized windows are metadata-only even though the native source can reference offscreen rendering. Excluded/unknown-capture windows never instantiate a thumbnail. Fullscreen and sticky windows can be eligible only in the actual active context. No private effects module, portal screenshot bypass, external screenshot stand-in, or promise of inactive textures is used. Live GL privacy, frame availability, closing-window teardown, fractional scale and hotplug remain unverified; source existence is not a runtime PASS.

The Qt regression probe now demonstrates a new directory plus relative QML component loading in the original cached engine, while replacement at an unchanged URL stays stale. `overview-dev.py` therefore prepares source-hash package IDs and retains one stable shortcut identity with an empty default. This is a supported-package iteration strategy validated in installed Qt; native KWin iteration remains pending. It never invalidates global caches, changes existing shortcuts, or restarts the compositor. See [POC-02B](poc-02b.md) for the same-session observer prerequisite gate, exact commands and rollback.

## POC-02A scripting audit (KWin 6.8.1)

**POC-02A: IMPLEMENTED — LIVE VALIDATION PENDING.** This audit is separate from the historical effect audit below. The [KDE scripting API page](https://develop.kde.org/docs/plasma/kwin/api/) labels itself as KWin 6.0, so the actual contracts were cross-checked against release-tagged source and installed headers.

| Native contract | Exact 6.8.1 evidence and implementation |
| --- | --- |
| Workspace window list and signals | [workspace_wrapper.h](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/workspace_wrapper.h), [implementation](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/workspace_wrapper.cpp): `stackingOrder` is a Window pointer list; `windowAdded`, `windowRemoved`, `windowActivated` forward native signals |
| Desktops, Activities, outputs | Same wrapper: desktop/output QObject lists, QStringList Activities, writable native active properties (observer only reads), `desktopsChanged()`, `screensChanged()`, `activitiesChanged(QString)`, `currentActivityChanged(QString)` |
| Per-output desktop changes | `currentDesktopChanged(VirtualDesktop *previous, VirtualDesktop *current, LogicalOutput *output)`; `currentDesktopForScreen(output)` provides a read-only per-output query |
| Window metadata/state | [window.h](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/window.h): public Q_PROPERTY declarations confirm caption, desktopFileName, resourceClass/name, desktops, Activities, output, geometry, classification flags, minimized, fullScreen and corresponding subscribed signals |
| Window lifetime ID | `internalId` is a constant QUuid independent of window system; [window.cpp](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/window.cpp) creates a new UUID per Window constructor. Observer IDs do not depend on successful QUuid conversion |
| All memberships | Inline `Window::isOnAllDesktops()` and `isOnAllActivities()` use empty native lists. `onAllDesktops` is a public property; no public `onAllActivities` property is assumed |
| Desktop naming | [virtualdesktops.h](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/virtualdesktops.h): stable native ID, name, `nameChanged` |
| Logical outputs | [core/output.h](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/core/output.h): geometry, devicePixelRatio, name, `geometryChanged`, `scaleChanged`; hardware UUID is not a public property, so output IDs are observer-local |
| JS runtime/timers/IPC | [scripting.cpp](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/scripting.cpp), [header](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/scripting.h): per-script QJSEngine, invokable ScriptTimer exposed as QTimer, asynchronous callDBus. Errors are logged and skip the callback; transport recovery accounts for this |
| Script packaging | [ScriptsPackage](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/plugins/kpackage/scripts/scripts.cpp): `KWin/Script`, `X-Plasma-API: javascript`, required `contents/code/main.js`; installed KPackage recognizes the source package |
| Individual script run/stop | [Script D-Bus XML](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/org.kde.kwin.Script.xml) and current `/Scripting` introspection: `loadScript(path, pluginName)`, per-script `/Scripting/Script<ID>` `run`, and fixed-name `unloadScript`. No global scripting start is used |

The installed `/usr/include/kwin/window.h`, `core/output.h`, and `virtualdesktops.h` corroborate these contracts. Actual native wrappers remain untested until the observer is approved to run. Qt 6.12.0 production-JS smoke tests and an isolated D-Bus receiver test are supporting checks, not proof of real KWin signal delivery.

There is no public Window property that reliably distinguishes native Wayland from XWayland or authenticates a lock-screen surface. Native UUIDs work across both without X11 IDs. Protocol `windowingSystem` stays null; classifications use only documented flags, and unknown types are excluded by default. Not every Wayland/private/system surface appears in `stackingOrder`. A guaranteed separate lock-screen/backend label would require an upstream public API addition or a separately reviewed native extension. Output hardware identity, Activity names, and persistence are likewise not invented from unavailable metadata.

Read-only preflight confirms four native desktop UUIDs, one Activity, and eDP-2 at 1920×1080/scale 1/143999 mHz. The observer has not been installed/loaded and an actual full window snapshot is pending approval; see [poc-02a.md](poc-02a.md). POC-01's corrected lifecycle logs were rechecked read-only and show completed cycles; physical acceptance remains pending.

## Initial observed environment (before user installation)

| Item | Read-only observation |
| --- | --- |
| Distribution | Garuda Linux (`/etc/os-release`) |
| Plasma / KWin | `plasmashell 6.8.1` / `kwin 6.8.1` |
| Session | Wayland, KDE |
| Qt | Runtime/compile 6.12.0 from KWin support information |
| Compositor | OpenGL ES 3.2 via EGL; Intel Mesa ADL GT2, Mesa 26.2.5 |
| Virtual desktops | Four recorded in `kwinrc`; unchanged |
| Activities | One returned by ActivityManager; unchanged |
| Custom effect packages | `codex-glass`, `wallpaper-style`, `wallpaper-style-transparency` |
| Custom effects loaded | `wallpaper-style-transparency`; the other two were not loaded |
| DepthOS | Not installed or loaded during implementation |
| Preferred shortcut | KGlobalAccel returned `true` for availability |

The supplied 1920×1080/~144 Hz and hybrid Intel/NVIDIA hardware describe the intended target. This work confirmed the active Intel renderer, not physical GPU inventory, refresh rate, frame pacing, or every output configuration. Layout tests use synthetic 1920×1080 and 1280×720 logical geometries. No live performance claim is made.

## Version-specific evidence

The current [KDE effect guide](https://develop.kde.org/docs/plasma/kwineffect/), [SceneEffect reference](https://api.kde.org/qml-org-kde-kwin-sceneeffect.html), and [ShortcutHandler reference](https://api.kde.org/qml-org-kde-kwin-shortcuthandler.html) describe the public approach. Compatibility was cross-checked against **the `v6.8.1` source tag**, rather than inferred from current documentation alone:

| API / behavior | Matching source and finding |
| --- | --- |
| `SceneEffect`, `visible` | [scriptedquicksceneeffect.h](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/scriptedquicksceneeffect.h), [implementation](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/scriptedquicksceneeffect.cpp): `visible=true` requests `setRunning(true)`; false schedules teardown on the event loop |
| Delegate, geometry, focus, input | [quickeffect.h](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/effect/quickeffect.h), [implementation](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/effect/quickeffect.cpp): one item per output, root size follows content size, geometry-change connection, keyboard forwarding, view activation, mouse interception, and input release on stop |
| OpenGL and startup exclusivity | Same implementation: `supported()` requires OpenGL; startup returns early for another fullscreen effect, delegate failure, or a failed keyboard grab |
| Global shortcut | [shortcuthandler.cpp](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/shortcuthandler.cpp): QAction registered with KGlobalAccel, unique name and text required |
| Toggle while grabbed | [input.cpp](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/input.cpp): global shortcut filter precedes effects filter; actual key delivery remains a live acceptance test |
| Package metadata and entry point | [example metadata](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/examples/quick-effect/package/metadata.json), [example QML](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/examples/quick-effect/package/contents/ui/main.qml), [KPackage structure](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/plugins/kpackage/effect/effect.cpp): `KWin/Effect`, `declarativescript`, required `contents/ui/main.qml` |
| Logs | `quickeffect.cpp` reports delegate errors under `libkwineffects`; installed `/usr/share/qlogging-categories6/org_kde_kwin.categories` identifies effect/scripting categories |

The installed `/usr/include/kwin/effect/quickeffect.h` independently confirms the delegate, view, and input interface. The installed package structure plugin recognizes DepthOS with `kpackagetool6 --type KWin/Effect --show <absolute-package-path>`. The live `/Effects` D-Bus introspection confirms `loadEffect`, `unloadEffect`, `isEffectLoaded`, `isEffectSupported`, and the loaded-effects property. `/Scripting` was introspected but is unused by this POC.

Qt Quick `NumberAnimation`, `ParallelAnimation`, `Easing`, `FocusScope`, `Keys`, `MouseArea`, geometry bindings, and interruption behavior were exercised by the installed Qt 6.12.0 engine in an offscreen harness. The harness substitutes the two KWin types; it proves no real KWin import or compositor behavior.

## Activation repair findings

The installed package initially matched the repository. During this investigation it was unloaded, with its preferred action/key still recorded in KGlobalAccel. `getGlobalShortcutsByKey(385876026)` identified `depthos-spatial-poc-toggle` under component `kwin`; `shortcutNames()` also listed it. This establishes saved registration/key ownership, not an active callback. The persistent enabled preference was unset. No configuration mutation was necessary to diagnose the QML defect.

KWin 6.8.1's `QuickSceneEffect::addScreen()` explicitly creates `new QQmlContext(parentContext)` and calls `delegate->create(*incubator, context)`. That is incompatible with a delegate declared in a file with `ComponentBehavior: Bound`. Qt's [ComponentBehavior documentation](https://doc.qt.io/qt-6/qtqml-documents-structure.html#componentbehavior) explains that Bound nested components are restricted to their original creation context. A local Qt 6.12.0 C++ reproduction confirmed the exact rejection, while the original QML `createObject()` fixture passed. Production now explicitly specifies `Unbound`; the same child-context incubation succeeds without QML warnings. A Bound negative control reproduces the error. No change to the writable `SceneEffect.visible` mechanism is needed.

`WindowThumbnail` is also public in the exact release: [windowthumbnailitem.h](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/windowthumbnailitem.h) declares `QML_NAMED_ELEMENT(WindowThumbnail)` with writable `client` and `wId`. This task verifies its existence only; no thumbnail behavior or POC-02 surfaces were implemented/tested.

Live KGlobalAccel introspection exposes `org.kde.kglobalaccel.Component.invokeShortcut(QString)` at `/component/kwin`. The development trigger invokes only `depthos-spatial-poc-toggle`. It bypasses physical-key dispatch while preserving production callback/activation paths. No direct external scene-root visibility setter was identified. The [6.8.1 effect handler source](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/effect/effecthandler.cpp) confirms that `/Effects.toggleEffect` loads/unloads a plugin; it does not toggle a declarative scene's visibility.

Root construction, registration, callback execution, `visible` request, delegate incubation, active-effect status, and observed rendering are distinct stages. Temporary scoped logs expose the QML stages but cannot prove visual rendering or input recovery. Read them from the user journal using the actual KWin PID; system-only `_COMM` queries may miss session messages. No matching historical DepthOS activation error was captured in available live KWin logs, so the reproduction is the evidence for the code defect and physical-key interception remains an untested possibility.

The approved update and live attempt are now recorded in `docs/phase-1.md`: installed 0.1.1 matches the source, loading and diagnostic invocation succeeded, but cached original QML failed delegate creation. Cleanup successfully unloaded DepthOS. Real animation/input behavior and repeated toggling of the fixed implementation remain unverified. Other custom effect hashes and the loaded-effects list match the pre-test baseline.

### Shared QML engine and development reload limitation

The exact [6.8.1 loader](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/effect/effectloader.cpp) resolves `kwin/effects/<id>/contents/ui/main.qml`, obtains `workspace()->scripting()->qmlEngine()`, and loads the same URL into a new `QQmlComponent`. The [scripting implementation](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/scripting.cpp) owns that persistent engine. Effect unloading destroys the effect object, not the shared engine; neither path clears its component cache.

The approved live attempt demonstrated the consequence: the filesystem contained Unbound and new diagnostic logs, but the running component still produced the former Bound error and no new initialization logs. This is distinct from package installation failure or a conflicting physical shortcut. No public per-effect cache-refresh method was found in the installed D-Bus interfaces. A fresh engine/session is required to reliably retest these fixed files under the permitted public interface; no automatic restart/logout or unrelated-effect changes were performed. The package remains installed but unloaded.

An offscreen Qt 6.12.0 cache probe independently confirms same-engine same-URL source retention and successful updated-source loading by a fresh engine. It modifies only a temporary fixture. That probe does not clear or modify KWin's engine, cache, or other scripts.

The installed package manager is **KPackage 6.31.0**. Its [update implementation](https://invent.kde.org/frameworks/kpackage/-/blob/v6.31.0/src/kpackage/private/packagejobthread.cpp) and [version comparison](https://invent.kde.org/frameworks/kpackage/-/blob/v6.31.0/src/kpackage/private/utils.h) require a strictly newer version. The fixed package is therefore 0.1.1, replacing the installed 0.1.0 only after approval. Rollback to the old version requires scoped removal/reinstallation, not `--upgrade`. This version change enables package updating; it does not address activation itself.

## Boundaries and risks

Only public `org.kde.kwin` types and Qt Quick are imported. There is no `org.kde.kwin.private.effects` dependency, C++ plugin, custom shader, or unverified private interface. The upstream sources above are evidence for behavior, not a promise of compatibility with other releases. The installer deliberately pins 6.8.1; updates require another audit and live tests.

Scene effects run inside the compositor process. QML errors can prevent a delegate from appearing, and compositor bugs can affect session responsiveness. KWin may acquire input before all output views finish creation; keep the isolated D-Bus unload command available. No shell script can guarantee recovery from a compositor crash. This work never restarts or kills KWin/Plasma.

`visible` records the request; a rejected startup can leave it true even without an overlay. Exit any competing fullscreen effect, toggle DepthOS off, then reopen. No automatic retries or effects disabling are used. Lock/unlock, suspend, multi-output focus, hotplug, fractional scaling, NVIDIA rendering, shortcut auto-repeat, and visual smoothness still require live testing.

KWin supplies this public module within its process. The local standalone QML tools do not resolve it, so raw `qmllint` emits unresolved import/type diagnostics. Parsing and fixture tests are useful but do not substitute for loading in KWin after approval.

## Manual observer lifecycle

The [6.8.1 scripting source](https://invent.kde.org/plasma/kwin/-/blob/v6.8.1/src/scripting/scripting.cpp) connects workspace configuration changes to `Scripting::start()`. Its package reconciliation unloads disabled installed scripts by plugin ID. Therefore a one-session manual run must use the distinct runtime alias `depthos-system-observer-session` while reading the installed `depthos-system-observer` package. The scoped helper calls only that script's `run`, never global `start`, and persists no enabled preference. Cleanup checks both IDs. The initial approved observer run unexpectedly unloaded; this source behavior identifies a lifecycle hazard, not proof of the exact triggering action.

## Native acceptance stabilization — 2026-11-16

See [live-acceptance.md](live-acceptance.md) for actual Intel/KWin captures and measured bounds. Real normal/fullscreen/sticky/multi-desktop terminal frames were observed inside cards; minimized and capture-excluded clients showed the documented metadata-only fallback. A centered no-frame label incorrectly remained visible with real textures. The label now appears only for metadata-only cases; a neutral status sits below the requested preview, without guessing a texture-ready property. A regression checks separation/protected-state visibility, and a new content-addressed QML URL was retested.

Native test drivers use the exact public Window `pid`, `internalId`, `desktopFileName`, `desktops`, `onAllDesktops`, `minimized`, `fullScreen`, `excludeFromCapture` and `closeWindow` boundaries only on recorded disposable Konsole clients. KWin's overloaded `loadScript` requires explicit `ss` when called through this dbus-python harness. Physical input, inactive-context inspection, GPU timing and long-term stability are not certified by these boundaries.
