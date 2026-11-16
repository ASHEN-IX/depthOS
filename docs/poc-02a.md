# POC-02A — Native discovery and synchronization

**BOUNDED NATIVE ACCEPTANCE PASSED**, updated 2026-11-16.

The [complete live acceptance report](live-acceptance.md) records a new 23-check native run, including disposable-client states, sticky/multiple memberships, protected capture state, reconciliation and receiver/observer recovery. Cleanup restored the exact baseline configuration/effects. This is bounded prerequisite acceptance, not a production stability guarantee.

## Implementation and authority

`depthos-system-observer` is an independent, disabled-by-default `KWin/Script` JavaScript package. KWin remains the authority. The observer reads native window, desktop, Activity, and logical-output objects and emits an ephemeral replica; it never writes compositor properties, registers shortcuts, modifies configuration, or calls window/workspace actions. POC-01 is unchanged and is not needed to run discovery.

The foreground Python inspector is justified by KWin's public asynchronous `callDBus` API: JavaScript scripts cannot register a bidirectional external D-Bus object through this API. Python's already installed `dbus-python` and GLib provide a small session-bus receiver and terminal UI. No daemon, privileged service, Rust controller, database, network connection, telemetry, or AI dependency is added. Stop the foreground process to remove its endpoint.

Workspace signals are subscribed **before** the only full `stackingOrder` enumeration. Per-window signals are subscribed before reading its initial state. Startup and snapshots do not yield the KWin event loop, preventing a subscription/enumeration gap. A 50 ms dirty-set flush reads only affected windows and changed topology, coalescing geometry and redundant signals. It does not repeatedly enumerate native windows. Intermediate values within a flush interval are intentionally not a complete history.

All API-visible windows remain in the diagnostic state. Classification uses documented flags: known desktop/dock/popup/menu/notification/input-method/outline types are `system`; otherwise dialog, utility, normal non-special application, or unknown. Default application IDs exclude skip-taskbar and exclude-from-capture windows. Dialogs retain `parentRuntimeId` when their parent is observable, regardless of initial stacking order. `--all` shows the diagnostic collection. Titles are never used for classification or identity.

Window records are removed on `closed` or `windowRemoved`, with duplicate removal suppressed. Cleanup invalidates the record first, disconnects retained handlers, drops QObject references and dirty entries, and updates affected parent/focus relationships. Callback guards prevent queued updates from dereferencing removed objects. Unloading the script disposes its dedicated JavaScript engine, JS-owned timers, and native signal connections; live cleanup is still an acceptance gate.

## Versioned protocol

Schema version is **1**. The transport sends method calls, not broadcast metadata signals, to `/org/depthos/StateInspector`, interface `org.depthos.StateInspector1`. The receiver owns `org.depthos.StateInspector` on the local user session bus. No TCP or filesystem IPC endpoint is opened.

Before sending metadata, the observer resolves the receiver to a unique bus name and compares its Unix UID with KWin's through the bus daemon. Subsequent calls target that unique name. The receiver independently accepts publishing only from the current owner of `org.kde.KWin` with the current UID. This prevents unrelated bus users from receiving/publishing state through this protocol; it is not a security boundary against all processes already running as the same user.

`Hello(observerSessionId)` returns `{"schemaVersion":1,"consumerId":"<receiver UUID>"}`. `Push(json)` sends the following envelope and returns `{"consumerId":"...","needSnapshot":false}` (or true to request reconciliation):

```json
{
  "schemaVersion": 1,
  "sessionId": "<observer-run token>",
  "consumerId": "<receiver UUID>",
  "frames": [],
  "position": {"sequence": 1, "revision": 1}
}
```

Each frame has `schemaVersion`, `sessionId`, a positive monotonic `sequence`, a positive `revision`, and `type`. Every observed semantic event increments revision and sequence. Snapshots also increment sequence and represent the current revision. The run token changes when the script is reloaded; it is a date/random token, not a persistent or cryptographic identity. Window runtime IDs are non-reused counters within that run (`window-1`, etc.); use **(sessionId, runtimeId)** as the key. `nativeId` preserves KWin's window UUID when its QVariant representation is readable, otherwise null. Neither identity is assumed persistent across a compositor restart. No X11 window ID is used.

| Frame type | Payload |
| --- | --- |
| `snapshot` | `reason`, `state` (full latest replica) |
| `window.added` | `windowId`, complete serialized `window` |
| `window.removed` | `windowId` only; no destroyed-object reads |
| `window.focus.changed` | `activeWindowId`, including null |
| `window.metadata.changed` | `windowId`, `changedFields`, complete `window` |
| `window.desktop.changed` | Same; desktop membership/all-desktop flag |
| `window.activity.changed` | Same; Activity membership/all-Activities flag |
| `window.geometry.changed` | Same; geometry or output membership |
| `window.state.changed` | Same; minimized/fullscreen |
| `desktop.list.changed` | Complete `desktops` |
| `desktop.active.changed` | `activeDesktopId`, `activeDesktopByScreen` |
| `activity.list.changed` | Complete Activity ID list in `activities` |
| `activity.active.changed` | `activeActivityId` |
| `screen.list.changed` | Complete `screens`, including geometry/scale changes |

Snapshot state contains desktops (`id`, native ID, name), Activity IDs, logical screens (observer-local ID, name, geometry, scale), all serialized windows, default `applicationWindowIds`, active IDs, per-output active desktops, and capability/missing-signal information. Window fields are:

```text
runtimeId, nativeId, windowingSystem, title, applicationId,
resourceClass, resourceName, classification, typeFlags, parentRuntimeId,
desktopIds, activityIds, onAllDesktops, onAllActivities,
geometry {x,y,width,height}, screenId, minimized, fullscreen,
skipTaskbar, excludeFromCapture, truncatedFields
```

Unavailable metadata is **null**, not an invented value. A supported empty desktop list means all desktops; `onAllDesktops` reads KWin's corresponding public flag. A supported empty Activity list means all Activities, verified from KWin's implementation. A missing list gives null and never implies no membership. `applicationId` is the actual `desktopFileName` (possibly empty/null), with resource class/name kept separately rather than fabricated as an application ID. Titles are limited to 4,096 UTF-16 code units and marked in `truncatedFields`; identifier strings are capped at 512. Native geometry is serialized as numbers, not QObject references.

### Ordering, backpressure, and recovery

The ring holds at most 256 event frames; batches contain at most 32. Exactly one asynchronous D-Bus call is outstanding. While disconnected, only the latest native cache is needed. Overflow discards incomplete history and schedules a fresh snapshot at a higher sequence, explicitly marked `overflow`. The first connection or reconnection likewise sends a new snapshot.

The inspector rejects schema/type mismatches, ignores replayed sequence numbers, detects sequence/revision gaps, and requests a baseline. Every two seconds an empty heartbeat reports the sender's position, exposing a gap even if no further window event occurs. A partially drained batch may legitimately be behind that position; only an empty heartbeat is compared for exact position equality. Snapshot reconciliation replaces the replica atomically. An observer run change also requires a snapshot.

KWin 6.8.1 logs D-Bus errors **without invoking the supplied callback**. A 30-second watchdog allows its default 25-second pending call timeout to finish, invalidates late replies with a generation token, and retries discovery on the three-second control cadence. Reconnection can therefore take roughly 33 seconds after receiver loss. These are transport timers, not native-window polling. The current implementation has two 50 ms timer pumps (dirty flush and transport); no native window properties are polled when idle.

Packets are capped at one Mi UTF-16 code units on the sender and four MiB UTF-8 at the receiver. An oversized snapshot sends only a `payload-too-large` status and stops data retries rather than blocking the compositor or allocating an unbounded queue. The inspector displays that status. Snapshot pagination is a future improvement; an exceptionally large environment requires stopping/restarting after reducing the payload or changing this documented development limit. State storage otherwise scales with the number of live windows, not event history.

## Privacy and diagnostic view

The interactive inspector uses an alternate terminal screen, refreshes four times per second, escapes control/bidi characters, and clears/restores the terminal on exit. It shows active desktop/Activity IDs, normal applications, titles, actual app IDs, memberships and sticky/minimized/fullscreen state, screens, and the last event summary. `--all` includes diagnostic types; `--redact-titles` hides titles.

No title history is written by default. `--snapshot` prints one JSON snapshot with titles redacted by default. Adding `--include-titles` explicitly opts into sensitive output and possible shell redirection/history outside the app's control. Nothing reads browser contents, application documents, or command strings from metadata. Error replies/logs contain only fixed status messages. There are no arbitrary commands in the protocol.

## Installation and approved live lifecycle

**Live testing was approved and started on 2026-11-15. The required native prerequisite checks subsequently passed; see the final live result below.** The observer was installed and delivered a real, title-redacted snapshot. Read-only validation needs no approval:

```fish
cd /home/ashenix/Projects/DepthOS
fish scripts/validate-observer.fish
node tests/observer/benchmark.cjs
```

Development dependencies already found on the host: Node 24.20.0, Python 3, dbus-python, PyGObject/GLib, PyQt6 with Qt 6.12.0, `dbus-run-session`, fish, and `kpackagetool6`. PyQt is used only by the engine smoke test. No dependencies are automatically installed.

After explicit approval, install only the user-local package:

```fish
fish scripts/install-observer.fish --install
```

In terminal A, start the foreground receiver (it never starts KWin code):

```fish
python3 tools/state-inspector/main.py
# Optional private view:
# python3 tools/state-inspector/main.py --redact-titles
```

In terminal B, run only the new script, using its returned `/Scripting/Script<ID>` object:

```fish
fish scripts/observer-session.fish --start
qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.isScriptLoaded depthos-system-observer-session
```

The script uses `loadScript(path, "depthos-system-observer-session")` followed by that script's `org.kde.kwin.Script.run`. It deliberately does **not** call global `/Scripting.start`, touch the POC-01 effect, or persist an enabled preference. A run request/loaded result is not acceptance: the receiver must show an actual native snapshot and live events.

For a redacted one-shot export, stop the first inspector with Ctrl+C, start the following receiver, and allow up to the documented reconnection delay. Only one inspector owns the bus name:

```fish
python3 tools/state-inspector/main.py --snapshot
```

The JSON is printed to stdout, not saved automatically. Stop/restart the observer for a clean fresh-run test if preferred; no normal window operation is needed for this recovery test.

### Safe manual acceptance A–G

1. **Existing state:** compare current open applications, four existing desktops, Activity ID, screens and active window with KDE. Check default filtering and `--all` without changing existing layouts.
2. **New window:** manually open one test Konsole using normal KDE controls. Expect `window.added` and a new runtime ID. The agent does not open an application automatically.
3. **Focus:** manually switch between it and an existing application. Expect `window.focus.changed`; null focus is allowed.
4. **Membership:** manually move only that test window to another existing desktop. Expect `window.desktop.changed`, with the native desktop UUID. Optionally test sticky/multiple membership using normal controls; do not create desktops or Activities.
5. **Removal:** manually close only the test window. Expect one `window.removed`, with no later updates referring to a live record for that ID.
6. **Desktop switching:** manually switch existing desktops; expect `desktop.active.changed`. Titles, geometry, minimization, and fullscreen may be changed manually on the test window to verify their corresponding categories. Activity and output hotplug tests are optional and require the user's own intentional actions.
7. **Recovery:** stop only the observer, leave the receiver open, start it again, and confirm a new session token and consistent baseline. Separately stop/restart the receiver and verify a fresh snapshot after reconnection with no duplicate subscriptions. Compare other effects, window arrangement, desktop/Activity counts and preferences against the baseline.

Do not perform those window or compositor actions automatically without approval. No test requires changing an existing Activity, desktop count, global shortcut, panel, or another effect.

### Stop, rollback, and removal

Immediate rollback stops only the new script:

```fish
fish scripts/observer-session.fish --stop
qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.isScriptLoaded depthos-system-observer-session
# Continue removal only after this returns false (unload deletion is deferred).
fish scripts/uninstall-observer.fish --remove
```

Press Ctrl+C in the inspector terminal to release its bus name and in-memory state. Stop the observer first for prompt cleanup; stopping the receiver alone leaves a read-only observer waiting to reconnect. No configuration entry needs deleting, because none was written. Installation refuses existing packages; removal refuses loaded/unknown state or a symlink package. The separate POC-01 effect and all other KWin packages are untouched. No restart/logout/privileged service is involved.

## Verification evidence and actual data boundary

| Check | Result |
| --- | --- |
| KWin/Script source package recognition | Installed KPackage reports correct observer ID/path |
| JavaScript source tests | **19 passed** against production observer/transport |
| Python replica tests | **11 passed**, including cross-language production JS trace reconciliation |
| Qt 6.12.0 QJSEngine harness | Production source loads; synthetic initial snapshot and title event pass |
| Private D-Bus harness | Actual receiver process accepts fake authorized KWin on an isolated bus, rejects another sender, exports redacted JSON and exits cleanly |
| Fish syntax/help, Python parsing | Passed; no mutation branch executed |
| Existing POC-01 regression suite | **10 passed**, including child-context and cached-engine probes; no live activation performed |
| Live observer discovery/events/removal | **PASS** for all eleven POC-02B prerequisites in this compositor session; evidence below |

The private-bus harness explicitly refuses a populated desktop bus. It creates only a fresh `dbus-run-session` bus and a fake KWin name, never contacts the real compositor, and terminates only its own inspector process. The Qt harness verifies the installed JS engine, not KWin's native object wrapping. No real-KWin integration PASS is inferred from either.

Read-only native preflight on this host returned four desktops (`Desktop 1`–`Desktop 4`), current desktop `9fa20340-f74c-43c5-9fef-dae001b02e59`, one Activity `350b19c9-7963-45a0-b6ff-e9429dc12059`, and logical display eDP-2 at 1920×1080, scale 1, refresh 143999 mHz (KWin support information). This is actual OS data, **not an observer snapshot**. This historical preflight predates the approved native observer run below. The explicit unknown window data in the preflight is:

```json
{
  "kind": "native-read-only-preflight",
  "completeObserverSnapshot": false,
  "activeDesktopId": "9fa20340-f74c-43c5-9fef-dae001b02e59",
  "activeActivityId": "350b19c9-7963-45a0-b6ff-e9429dc12059",
  "windows": null,
  "windowsUnavailableReason": "New observer not authorized to run yet"
}
```

The approved run saved a full title-redacted native snapshot under ignored private `artifacts/live-02a`; the repaired session uses `artifacts/live-02a-retry`. Synthetic fixtures are named as such and contain no copied user window titles.

### Performance findings

One Node 24.20.0 synthetic run with 100 windows measured 10.15 ms for fixture creation/initial snapshot, 10,000 geometry notifications coalesced to 100 events in 2.57 ms including drain, 83,419 bytes for the initial snapshot, and zero overflows. These measurements include mocked JS objects, not native QObjects or compositor frame scheduling. They do not establish desktop overhead, memory cost, frame pacing, or no visible impact. Live CPU/latency and continuous-use measurements are pending. Run the provided microbenchmark to reproduce its local workload; do not interpret it as a desktop benchmark.

## Files and remaining limitations

Added the observer metadata/main.js, the foreground inspector `main.py` and pure replica `state.py`, three explicit install/session/remove scripts plus a safe validation script, JS fixtures/tests/microbenchmark, Python replica tests, and two isolated integration harnesses. Updated README, architecture/compatibility documentation, POC-01 evidence note, and Python-cache ignore rules. No POC-01 effect source was changed.

The observer sees only objects exposed by `workspace.stackingOrder`; it cannot inventory every private Wayland surface. Public Window properties do not expose a reliable Wayland/XWayland discriminator or a dedicated authenticated lock-screen flag. `windowingSystem` remains null, system flags filter the known native categories, and unknown types stay diagnostic-only. There is no title-based lock-screen heuristic. Guaranteeing a separate protocol/lock-screen label requires an upstream public property (or an independently reviewed native extension), not guesses from titles or resourceClass. This is a classification limitation, not a reason to use X11 tools or private APIs.

No persistent identity, project ownership, window movement or workspace automation is implemented. A subsequent POC-02B task has added a separate native-model renderer locally; see [poc-02b.md](poc-02b.md). Its activation remains conditional on native POC-02A acceptance A–G, privacy/filter review and explicit approval. The observer is not coupled to that effect's rendering lifecycle. Native observer prerequisite acceptance subsequently passed, permitting the approved scoped overview test.

### Approved live run: session lifecycle repair

The initial native snapshot contained five API-visible windows, two application windows, four virtual desktops, one Activity and eDP-2 at 1920×1080, scale 1. No required signal was reported missing. Desktop IDs and the current Activity were cross-checked against their native D-Bus managers. Receiver errors were zero. The early completion replies did not yield desktop or membership events. The user then clarified that the manual actions had not actually been performed. Those event checks remain untested; their absence is not evidence of a signal implementation failure.

The first manually started observer became unloaded while the compositor and session-bus identity stayed unchanged. KWin 6.8.1 `Scripting::queryScriptsToLoad()` unloads disabled installed packages by their plugin ID during configuration reconciliation. The exact trigger of this particular unload is unconfirmed. The session helper now loads the same installed source under `depthos-system-observer-session`, distinct from the disabled package ID. It writes no enabled preference. Stop and uninstall guards cover both the legacy ID and session alias; start refuses either already loaded. The repaired observer continues receiving heartbeats, but this alone does not establish all native event acceptance.

The foreground live collector reuses production authentication and reducer logic. It retains bounded event summaries with application identifiers and desktop membership but no titles. A consumer-requested fresh snapshot passed. Explicit stop confirmed the runtime alias unloaded; restarting produced a new session token and consistent baseline. Stopping/restarting only the receiver also recovered a fresh snapshot without reloading the observer, with zero receiver errors. The loaded-effect set, preserved custom/POC-01 file hashes, compositor owner and session-bus identity matched the initial baseline. The subsequent real Konsole sequence established the remaining required native events; see below.

### Final native prerequisite result — PASS, 2026-11-15

The user actually opened a new Konsole after clarifying the earlier responses. The authenticated observer recorded `org.kde.konsole`, native UUID `ebf7ff9a-3568-4f33-b29e-46de210ffc31`, runtime `window-8` in session `kwin-mwiemurq-5xlde4ux5umgbdr1455mt9x75hv380ssj`. Its initial Desktop 1 membership changed to Desktop 2 through `window.desktop.changed`. Focus events identified that window and existing application windows. `desktop.active.changed` recorded real desktop switches. `window.removed` sequence 58 removed this same Konsole; the reconciled snapshot contained no such native UUID. Receiver errors remained zero.

All eleven required checks passed: installation/loading, native initial snapshot, existing windows, window addition, window removal, focus, current desktop, window membership, snapshot reconciliation, clean unloading, and preservation of unrelated state. Explicit stop confirmed both owned script IDs unloaded. Observer restart produced a new session token; receiver restart recovered a fresh baseline. The evidence is local and ignored: `artifacts/live-02a`, `artifacts/live-02a-retry`, `artifacts/live-02a-reconnect`, and `artifacts/observer-acceptance.json`. The acceptance file pins source hash, compositor owner and session-bus ID; it is not transferable to another compositor session.

After conditional POC-02B testing, both new packages were removed, the observer unloaded and foreground receiver stopped. Loaded effects and the eight preserved custom/POC-01 source-file hashes matched the initial baseline. The compositor owner stayed unchanged. No persistent enabled preference was written. These are bounded prerequisite results, not acceptance of hotplug, Activity creation/removal, long-running stability or production performance.
