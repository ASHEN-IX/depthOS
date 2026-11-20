# Phase 3.7 independent security review

I reviewed the Phase 3.6 authority and production Rust surfaces, the exact release source, and the inherited private launcher independently from the implementation. The trusted computing base includes the compositor executable and its libraries, its startup environment, loaded in-process code, and the kernel's process isolation. A process able to write the compositor's memory, load native code, alter trusted startup configuration, or replace the executable can alter the authority within that base. The private fixture measures a controlled process under the current account. It does not enforce isolation from a hostile same-UID process with those capabilities.

## Input and display trust

In KWin 6.8.3, `KeyboardInputRedirection::processKey` builds a native `KeyboardKeyEvent` with the originating `InputDevice *`, then runs the spies and filters. The accessibility manager can intercept the transition before this point, and repeat events carry a null device. Comparable pointer transitions also enter with an optional device. EIS and fake-input devices can enter this pipeline as device objects. Device presence, native dispatch, key serial, and backend classification cannot establish physical human intent or whether a particular display was seen. Remote and portal paths require independent evidence for the actual route. A QML event cannot supply these guarantees.

`WaylandServer::isScreenLocked`, the server seat and native focus are compositor observations. `isScreenLocked` returns false when lock support is disabled and compares only the terminal `Locked` state when supported. Fixture runs with `--no-lockscreen` therefore supply unknown lock status for authority purposes. A positive path would have to distinguish lock acquisition from a confirmed unlocked session. A seat object supplies identity within that process, while device-to-seat binding and session integrity require separate evidence. An ordinary Qt Quick view does not protect prompt contents, display or approval from unreviewed compositor extensions. Real deployment needs a protected compositor-owned UI and a qualified process-integrity policy.

## Invalidation and coordinator consistency

A proposal must retain the exact live window object with a lifetime guard and independently bind its identity, placement and generation. Signals about focus, window removal, eligibility, desktop and Activity changes must retire affected proposals synchronously. Rechecking snapshots at consumption cannot detect focus loss followed by restoration. The test matrix therefore distinguishes persistent mismatch from observed transitions.

A coordinator lease must pin the checkpoint and exclude concurrent mutation through native consumption. Asynchronous acknowledgments can be lost. A timeout may prevent new dispatch but cannot establish that a turn never occurred. An uncertain result must be terminal for resubmission and reconciled through durable state and operation identities before a successor can safely proceed. A copied epoch and local validation are model evidence until a lease, its restart fencing and both sides of its state transitions are tested.

## Independent baseline controls

| Scenario | Classification | Evidence |
| --- | --- | --- |
| Same-UID D-Bus claims | Model-only evidence for native authority | Two private connections attempt forged UI claims against the Rust consent boundary. Production capability remains unavailable. |
| Ordinary QML approval | Model-only evidence for native authority | The offscreen and private Wayland deny-only prompt interact through cancellation and exit paths. |
| Wrong seat or unknown lock | Model-only evidence | The existing synthetic Rust and C++ fixtures reject missing or changed context. |
| Focus change and restoration | Model-only evidence for the patched authority | Rust and C++ explicitly invalidate and then restore context. The separate installed KWin fixture exercises native baseline focus loss. |
| Window removal, recreation, duplicate UUID | Model-only evidence for the patched authority | Rust tests cover identical-UUID replacement and ambiguous mappings. The installed fixture checks its live window removal. |
| Destination removal and stale topology | Model-only evidence for the patched authority | The Rust proposal tests reject changed destinations and checkpoint state. The installed fixture removes only an unused destination on its private bus. |
| Coordinator epoch drift and compositor restart | Model-only evidence for the patched authority | Rust validates both checkpoint and observed generation. Process restart fencing for the patched authority needs separate validation. |
| Lost receipt, competing cancellation and confirmation, repeat dispatch | Model-only evidence | The Rust turn tests record a bounded counter and cover terminal uncertainty, serialized cancellation and one-use dispatch. |
| Wrong-thread invocation, overflow and deadline edges | Model-only evidence | The standalone C++ test denies off-thread invocation and checks deadline overflow and equality. ASan/UBSan passed. |

The private launcher applies fixture desktop topology changes only through the worker's verified distinct bus. Tests only create or close owned native surfaces. No production controller or native setter is needed for these controls.

## Native patch checkpoint and independent corrections

Agent 1 handed off `a3705e87e5974462efca230891525af3d91aa9a3`. The reviewed patch attaches an observer in the virtual KWin backend, retains `QPointer<Window>` together with guarded source/destination, seat, surface and client identities, and observes native lifetime/focus/topology/visibility signals. Native proposal comparison checks the window UUID and generation, Activity membership, source placement, destination existence, exact seat keyboard surface, compositor incarnation and steady-clock deadline. The built-in fixture has a synchronous private token and a redacted counter, exercised only for the startup-selected owned client. The token is created and consumed internally during capture and the repeated consume is denied.

The native component's source is compiled into the private Wayland executable. Its constructor runs in the private process. The plugin fallback independently provides in-process observation evidence against the installed ABI. The native input origin annotation uses a dynamic property for diagnostic categories and always denies input. The property is mutable by in-process code. Local-candidate classification supplies no physical-input authentication. The private synthetic prompt is an opaque QObject. Its owner and pointer are checked in the turn, while rendered protected prompt integrity remains unsupported.

The native turn opens a pidfd for the startup candidate and compares the Wayland client PID. The launcher keeps its owned child waiting before display connection. The compositor opens that pidfd after startup, so process continuity before that open remains part of the trusted launcher/process-lifecycle assumption. The window's exact ClientConnection and SurfaceInterface guards constrain the later turn. The fixture increment does not participate in the Rust coordinator. Its epoch/checkpoint lease remains unimplemented and the Rust consistency tests are model evidence.

### Reproduced native plugin collision

The first full built compositor crashed after observer startup. The private executable's coredump identified `NightLightManager` construction in installed `nightlight.so`, entered through the private `PluginManager` and private `libkwin`. The private build used different KWin compile options, including disabled X11. Qt's installed plugin search paths allowed an incompatible native extension into that process. The passing ABI-pinned fallback could not establish the full patched runtime's safety.

The reviewer corrected the launcher by copying the checked private executable into the disposable run and supplying an adjacent `qt.conf` that selects an empty private native-plugin directory. `QT_PLUGIN_PATH` selects that same directory. The compositor's static QPA integration remains available. At startup, the launcher now checks the copied executable's digest, `/proc/PID/exe`, the mapped private `libkwin` pathname and digest, and refusal of installed KWin native-plugin mappings. The corrected private build passed the full native observation, Wayland QML and sampled host comparison. The restricted core and backtrace preserve the reproduction. This correction affects only the disposable private runtime.

The reviewer also added deterministic tests for missing retirement, duplicate retirement, replacement of pending proposals and malformed synthetic replay outcomes in the native transcript. A native run must pair each pending observation with one terminal log event before accepting the integration result. The source turn itself supplies the scoped synchronous guarantee. Logs only support runtime coverage of that implementation.

| Additional scenario | Classification | Evidence and limit |
| --- | --- | --- |
| Missing prompt and unknown lock | Verified native denial | The built compositor retired observed proposals without a synthetic turn in deny-only mode. |
| Native focus loss, window eligibility change, deadline and destination removal | Verified native denial | The private compositor retired pending observations. Exact restoration, UUID collision and restart fencing retain the model-only coverage above. |
| Internal fixture prompt, unlocked state and repeat token | Verified synthetic behavior | The built compositor counted once and denied the identical token on the immediately following call. No ordinary event supplied the token. |
| Forged scripts or same-UID calls reaching native approval | Unsupported | The test turn exposes no approval binding. The normal preview script and production read-only interface were exercised; extension integrity against a hostile local process was not established. |
| Actual lock transition, wrong native seat | Unsupported | Source wiring and single-seat fixture observations do not establish live transition or multiple-seat behavior. |
| Virtual/EIS, remote/portal, accessibility, unknown and local physical input | Unsupported | Pre-Qt classification hooks were reviewed and compiled; the private runtime tests did not exercise those sources. No category can authorize. |

## Final scope

**PHASE 3.7 PARTIALLY VERIFIED.** The final private runtime demonstrated native object retention, selected invalidation transitions and one internal synthetic consume. Physical input integrity, remote/EIS/accessibility ingress policy, protected rendered prompt ownership, actual lock transitions, multi-seat behavior and a coordinator lease remain open. The threat cases involving ptrace, native injection, writable compositor startup or executable replacement require a deployment process-integrity policy. The standalone timing, clock-overflow, wrong-thread, uncertainty and race tests retain their model-only classification.

Rollback uses the original reviewed Phase 3.6 checkout or reverses the Phase 3.7 commits in the independent repository. Private binaries, libraries and core evidence remain under ignored artifacts. The original damaged Codex ref stays preserved in its source repository.
