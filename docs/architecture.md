# Architecture

The future conceptual hierarchy is Universe → worlds/projects → project desktops → active Linux desktop. Visual layers map to Z3 Universe, Z2 world/project overview, Z1 project desktop overview, and Z0 the ordinary desktop. These layers are product navigation concepts, not additional KWin virtual desktops.

POC-01 consists of a single public-QML KWin effect. Its root `SceneEffect` owns toggle state, selection state, and entrance/exit animations. KWin creates a `FocusScope` delegate per output and supplies its logical geometry. Every delegate reads the same animation and selection properties, so dismissal on one output closes all views. Scale, opacity, and a short vertical movement suggest depth without shaders or real 3D surfaces.

`ShortcutHandler` registers one uniquely named configurable KWin action. `MouseArea` consumes clicks over the card and backdrop; `Keys` handles Escape and selection. The focused delegate owns ordinary Qt Quick keyboard focus. KWin owns the global input grab and releases it when the effect becomes invisible. DepthOS retains visibility throughout the exit animation; reversing a transition stops the previous animation and starts from current property values. Selection resets on a fresh open.

The effect replaces the scene while active. Fading a navy rectangle does not expose the desktop underneath: the replaced scene contains no window surfaces. The final transition back to the desktop occurs when KWin deactivates the effect. A continuous live-desktop crossfade is outside this phase.

## POC-02A native observer

An independently packaged `depthos-system-observer` JavaScript script now maintains an ephemeral read-only cache of native KWin state. Workspace subscriptions precede enumeration; per-window subscriptions precede each window read. Dirty sets coalesce updates without repeated native window enumeration. Native objects never leave the compositor: serialization produces primitive JSON fields, scoped runtime IDs, revisions, and monotonic sequence numbers.

The observer's own QJSEngine and lifetime are independent of the spatial effect's rendering and shared QML engine. A manual foreground Python consumer owns a local session-bus endpoint, validates the current KWin sender, reconciles snapshots/events, and renders diagnostics in a terminal. Receiver identity is also verified by the producer before metadata transmission. Bounded buffering, heartbeat position checks, and consumer-requested snapshots recover from overflow/reconnection. There is no permanent service, title history, database, or production orchestration engine.

KWin owns live state; this replica never becomes an alternative authority or writes properties back. Project hierarchy, persistent ownership, and command authorization remain future controller responsibilities. The effect is not directly coupled to the observer in this phase. A future renderer/controller can consume the versioned schema after native event accuracy and lifecycle acceptance are demonstrated. See [POC-02A](poc-02a.md) for protocol and limitations.

## Separation of responsibilities

POC-02B adds a separate native renderer in `experiments/kwin-overview`. After comparing direct models, shared script state, external control and hybrid approaches, it uses **native models for rendering plus the existing observer for independent diagnostics**. There is no unsupported cross-engine object sharing or Python-to-effect mutation bridge. The native WindowModel owns row changes, Window/Workspace NOTIFY bindings own metadata and active context, and local QML state owns only inspection/intent. It does not consume the observer replica as a second authority. Protocol gap/reconnection handling remains the diagnostic consumer's responsibility; an observer outage does not invalidate still-live KWin models.

Z1 is native normal/dialog windows on a locally selected desktop/Activity, Z2 real virtual desktops, Z3 real Activity IDs when available, and Z0 the ordinary desktop after dismissal. Active context is always distinct from local selection and intended destination. Selecting a card never invokes native activation, movement or desktop/Activity setters. Public native thumbnails are requested only for capture-allowed, non-minimized windows belonging to the actual active context; other cards remain honest metadata-only views. Titles, previews and motion have local controls.

The dynamic replacement is packaged separately to preserve POC-01 and to use immutable source-hash development IDs/new URLs for safe iteration in KWin's cached QML engine. One unbound-by-default shortcut action is reused across revisions; the tool prevents concurrent managed revisions and provides scoped cleanup. Native POC-02A acceptance gates loading/activation. See [POC-02B](poc-02b.md) for field ownership, cache evidence, budgets and pending live acceptance.

| Component | Responsibility | Current implementation |
| --- | --- | --- |
| KWin effect | Render depth layers, animations, compositor-managed surfaces and input | Original POC-01 preserved; dynamic native POC-02B implemented; native lifecycle and controlled preview matrix passed, physical acceptance pending |
| KWin observer script | Discover native state and publish read-only structured observations | Implemented independently in POC-02A; native prerequisites passed in the approved session |
| Future KWin command adapter | Apply narrow supported operations only when authorized by a controller | Absent |
| Rust workspace controller | Own hierarchy, commands, reconciliation, and IPC independently of rendering | Absent |
| SQLite persistence | Store project definitions and explicit window association policies | Absent |
| Optional AI integration | Suggest organization through controller commands, without direct compositor authority | Absent |

Future work should define stable identifiers, ownership rules, and reversible commands before mutating KWin state. KWin Activities and virtual desktops must not be treated as interchangeable with DepthOS projects. The controller should own persistent state; an effect reload must not lose that state. Window movement and assignment must be separate explicit operations from viewing a layer.

There is no permanent service, database schema, or Activity/desktop manipulation. The observer's development IPC/window enumeration and the renderer's conservative public WindowThumbnail requests are implemented, with separate lifecycles and without committing to workspace ownership semantics.

The 2026-11-16 [live acceptance report](live-acceptance.md) records current bounded results and the remaining gates before workspace ownership or automatic management. No POC-03 implementation is included.

## Phase 2.5 — native logical hierarchy integration

The explicitly started foreground Rust controller now offers a separate read-only
session-bus UI boundary: `org.depthos.Controller`, `/org/depthos/Controller`,
`org.depthos.Controller1`. Versioned, paged hierarchy/context reads and coalesced
invalidations connect the existing native overview to actual logical records.
The producer receiver `org.depthos.StateInspector` remains independent and retains
its current-owner/UID authentication. No raw observer packet reaches the UI.

The Plasma QML D-Bus client uses bounded asynchronous requests, instance/revision
fencing and ownership-change recovery. Logical selection belongs to the client
and never switches a KDE desktop/Activity or activates/moves a window. Native
WindowModel/WindowThumbnail rendering continues during backend outages. The new
bus deliberately exposes no assignment mutations: same-user access does not
prove interactive intent. See [Phase 2.5](phase-2-spatial-integration.md) for the
contract, real isolated KWin evidence, measured scaling limits and rollback.
Earlier milestone and host acceptance reports retain their original scope.
