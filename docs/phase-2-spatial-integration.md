# Phase 2.5 — native spatial UI and foreground Rust controller

**PHASE 2.5 ACCEPTED** for isolated KWin 6.8.3 integration. Phase 1 and physical host acceptance remain partially accepted. This report does not authorize a host installation or begin Phase 3.

## Boundary and ownership

KWin still owns native windows, thumbnails, input, geometry, desktops and Activities. The foreground Rust controller owns the SQLite hierarchy and ephemeral context decisions. The effect reads redacted logical data asynchronously through the installed Plasma QML D-Bus component. Neither component launches the other. There is no HTTP server, network listener, systemd service, autostart entry, new shortcut binding, SQLite access in QML or native action endpoint.

The UI endpoint is deliberately separate from the authenticated producer transport:

- UI service: `org.depthos.Controller`.
- UI object: `/org/depthos/Controller`.
- UI interface: `org.depthos.Controller1`.
- Observer transport remains `org.depthos.StateInspector` at its existing path/interface, with the existing current-KWin-owner and UID authentication unchanged.

The controller executable owns both connections. UI registration uses DoNotQueue and never replaces an owner. A collision fails foreground startup and releases the newly started controller. An initially missing bus preserves bounded degraded controller startup; UI registration is retried once if receiver registration subsequently succeeds. Loss of an established UI connection ends this foreground process with an explicit error; restarting it creates a new instance. Controller termination aborts and joins the UI monitor and releases its name before controller shutdown. No separate permanent worker is installed.

## Version 1 read contract

Every method requires the exact path/interface and a caller UID matching the foreground process. This is a same-user read policy, **not proof of interactive user intent**. Introspection contains no assignment/unassignment/override/selection write methods. There are no exported VerifiedLaunch constructors. Another same-user process can read the bounded hierarchy; it cannot turn these reads into assignments.

Methods return a D-Bus string containing a JSON envelope with `schemaVersion: 1`, opaque process `instanceId`, opaque `revision`, boolean `available` and `data`. Revisions are cache tokens, not persistent database or window identities.

- `GetCapabilities() -> s`: readOnly=true, assignmentMutations=false, selectionOwner=client, pageSize=64, maxRows=4096.
- `GetControllerHealth() -> s`: current lifecycle, fixed reason, availability and context checkpoint; no raw observer packet.
- `GetHierarchy(cursor: u, expectedRevision: s) -> s`: flat items containing kind, logical UUID, parentId and name. Kinds are world, workspace and desktop. Workspace paths, descriptions and desktop roles are omitted.
- `GetContextReport(cursor: u, expectedRevision: s) -> s`: eligible WindowKey, unique current native UUID when available, status, fixed reason/sources, workspace targets and competing alternatives. No titles, application IDs, PIDs, launch tokens, native geometry or arbitrary metadata.
- `GetCurrentLogicalContext() -> s`: owner=client and null IDs. The server does not pretend to know the effect's local selection.
- `Invalidated(instance: s, scopes: u)`: scope bit 1 means hierarchy, bit 2 means context/health. Clients fetch the current snapshot; signals carry no window arrays.

A page contains at most 64 rows and a nextCursor or null. Cursor 0 with an empty expected revision starts a read; subsequent pages echo the returned revision. Stale revisions, oversized revision strings and invalid cursors fail. Each response is capped at 128 KiB, and each collection at 4,096 rows. Overflow becomes explicitly unavailable, rather than a plausible truncated hierarchy. Eight concurrent refresh reads are admitted. The underlying existing controller/store still constructs its hierarchy internally; these transport caps are not a database-size quota.

Controller health notifications drive a coalesced 100 ms invalidation monitor. Successful in-process logical operations also notify that monitor, including World and logical-desktop edits that do not change workspace context versions. A 500 ms fallback discovers edits made through a separate trusted CLI process. Delivery therefore does not rely solely on client polling. Geometry/sequence changes alone do not invalidate semantically identical association rows. Getter-triggered refreshes retain pending invalidations for other clients. Unchanged synchronized versions reuse redacted context rows; epoch or availability changes invalidate that cache.

## QML compatibility and local selection

The installed `org.kde.plasma.workspace.dbus` plugin was tested in both real Qt Quick and isolated KWin. Its asynchronous calls need parenthesized signatures, installed `iface` map keys and explicit String/Number normalization of typed D-Bus values. The client supplies both iface/interface keys for compatibility with the inspected upstream variant. SignalWatcher and DBusServiceWatcher deliver invalidation and ownership changes. An asynchronous Loader isolates a missing plugin from the native scene.

Relevant upstream implementation: [KDE Plasma D-Bus component](https://invent.kde.org/plasma/plasma-workspace/-/tree/master/components/dbus). The tested installed runtime, rather than an assumed arbitrary Rust/Qt import into KWin, determined the final adapter.

The client retains one pending call, uses a 2.5 s watchdog, debounces refreshes for 100 ms, and retries unavailable calls after 2 s. It resolves a unique service owner before fetching. Owner changes reset rows, native badges and local selection. Serial fencing rejects responses from an earlier refresh; revision and instance checks prevent mixing pages. Decoding rejects unsupported schemas and oversized/malformed models. Plain-text labels strip control/bidi characters. Duplicate native UUID mappings never produce a badge.

The logical panel displays the real database, including empty state, with virtualized horizontal World, Workspace and LogicalDesktop lists. Selection belongs to the effect client: it survives overview close/reopen while that client lives, and resets on backend replacement, effect unload or deletion of its destination. Deletion of a selected workspace retains its surviving World selection. No selection is persisted or shared between independent clients. Selecting a row never changes KDE desktop, Activity, focus or window geometry.

Z0/Z1/Z2/Z3 retain their native prototype meaning; logical desktops are not KDE virtual desktops. Native cards retain WindowModel/WindowThumbnail rendering and protected-window fallbacks. Cards add an association status; the panel counts unknown/stale, suggested and conflicted decisions. Unknown is unassigned, and suggestions remain advisory. Stale badges disappear on context invalidation or native identity removal. Missing controller/observer state is explicit and never hides live native windows.

Assignment controls are intentionally unavailable. A later write API requires a concrete, tested interactive authorization model in addition to existing WindowKey/version/eligibility validation. Matching the UID or owning a bus name is insufficient. The test fixture makes hierarchy edits through the existing trusted CLI.

## Verification and native evidence

The pre-change baseline passed 78 Rust checks and 18 overview Qt checks. Final verification includes:

- `cargo build --locked`, `cargo test --locked`, `cargo fmt --all --check`, and `cargo clippy --locked --all-targets -- -D warnings`.
- 81 Rust checks including three new bridge checks; the existing explicit performance benchmark remains ignored in the default suite.
- Python/Rust observer parity: seven scenarios, 30 packet comparisons; private-bus Python-to-Rust interoperability, unified CLI checks and controller process recovery.
- New Rust API tests: schema, 64-row pages, stale revisions, payload redaction, absent mutations, wrong interface, invalid cursors, 4,096-row overflow, name conflict, owner loss, stale assignments, shutdown/restart and process instance replacement.
- `tests/spatial/model.cjs`: malformed/versioned payloads, limits, safe names, five association statuses and duplicate/native identity refusal.
- `tests/spatial/client-smoke.py`: production QML with the actual Rust service; initial outage, hierarchy rename/delete, selected-destination removal, controller exit/restart and in-flight response fencing.
- `tests/spatial/version-mismatch.py`: a real private-bus schema-2 peer is rejected by the production client.
- `tests/spatial/tst_panel.qml`: four Qt checks for empty/loading/connected/degraded/unavailable models, actual fixture labels, mouse selection and association counts. These are fixture checks, not native evidence.
- Original overview suite: 18 Qt checks; package policy suite: seven checks; native safety policy suite: six checks.
- Existing isolated native Wayland suite: 72/72 assertions in `artifacts/poc-02g/run-20261116T151845Z-8f314f`. Cleanup passed and the original host window inventory matched. Its instantaneous host activeEffects comparison differed during a host transition; this is retained in its evidence rather than described as an exact host snapshot match.

`tests/spatial/native.py` creates a new private bus, HOME/XDG roots, virtual Wayland KWin, Activity, test application, temporary SQLite database and actual foreground controller. It validates KWin owner PID/version, installs a production-content-hash package in that disposable data root before compositor startup, and loads the unchanged observer only into that private compositor. No installed host package is overwritten. Read-only scene probes and private EIS input verify genuine native selection and Escape dismissal.

The native scenario verifies two Worlds, three Workspaces and three logical desktops; actual labels and unique-UUID card badges; CLI rename invalidation; deletion of the selected workspace; no KDE desktop/Activity/focus change; synchronized associations; observer expiry while hierarchy survives; controller exit/restart; no restored ephemeral assignments; native window disappearance; thumbnails and input release. The launcher inventories and terminates only its own process group and compares settled read-only host snapshots. Earlier failed attempts remain under artifacts/p25, including animation-coordinate mistakes and transient host activeEffects differences. No host restoration was attempted.

The final run directory, assertion count, hashes and exact host/cleanup results are recorded in `tests/spatial/source-manifest.json` and its referenced native evidence.

## Performance and remaining limits

Measurements are isolated observations, not host desktop performance or compositor FPS. The debug-build synthetic fixture in `artifacts/p25/performance.json` uses 301 logical rows, 1,000 native windows, a real private bus and the production Plasma QML client with offscreen software rendering:

- Full paged hierarchy fetch: median 52.21 ms, maximum 68.09 ms over ten reads.
- Initial snapshot submission to invalidation dispatch: 169.22 ms; to final QML context model: 1,459.60 ms.
- Native identity removal to final QML model: 1,050.72 ms.
- Controller RSS: 25,000 KiB; QML test process RSS: 46,756 KiB.
- Three-second connected idle sample with heartbeats: controller 6.99% of one core; QML process 0.67%.
- Thirty geometry events published in 129.86 ms caused zero semantic UI invalidations.

Invalidation timing includes controller processing and fixture dispatch. QML timing includes page transfer/decoding and debounce, rather than display presentation timestamps. The small native fixture's CLI-edit-to-scene latency is recorded separately in each run/performance.json (roughly 0.55–0.67 s in the expanded runs). Debug large-model CPU and full refresh latency need optimization before claiming production-scale performance. No frame pacing or physical-input latency claim is made.

## Protection, rollback and next phase

Before implementation, a two-pass stable source audit and writer check produced the hash-verified 143-file backup at `/home/ashenix/Projects/DepthOS-backups/phase-2.5-20261116T144802Z`. Its manifest records snapshot time, source hashes, no concurrent source writers and four KDE configuration hashes. The completion verifier checks every backup copy, limits edits to the declared Phase 2.5 files, verifies the remaining original files and confirms those KDE hashes. Production observer JavaScript, Python inspector, migrations, core domain/store and historical acceptance reports are preserved.

Changed ownership is limited to Cargo dependency wiring, controller notification/CLI lifecycle wiring, new spatial service, overview QML, narrow test instrumentation and architecture documentation. New files are the spatial service/client/panel/decoder, bridge tests, scoped test tools, this report and the completion manifest/verifier. No Git commit was created.

Rollback is reviewable: stop the explicitly started controller, unload only the tested private package if still present, and restore only the changed paths listed in the completion manifest from the verified backup; remove the listed new source files. Do not run a blanket restore over concurrent future work, reset KDE configuration, delete unrelated artifacts or replace a host-installed package. The test launchers already clean up their own private processes/packages.

Host compositor/version/session qualification, portal authorization, physical input/rendering acceptance and long-running performance remain separate gates. Read-only isolated integration is ready for review; assignment authorization, persistent context selection, launch/restoration, routing, automation and daemon deployment require separate work. Phase 3 has not begun.
