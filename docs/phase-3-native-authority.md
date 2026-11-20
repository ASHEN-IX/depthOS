# Phase 3 native authority

The completion pass starts at `71f827ee140c2c8ca0e4c91a9dc7938995dc16a5`. KWin remains the authority for native windows, focus and placement. DepthOS uses the inspected scripting surface to track a window and present a deny-only preview inside a private fixture.

## KWin integration surface

The qualified KWin executable is from package `kwin 6.8.3-1`, build ID `6eb58cf1f8d875ef625487ec2f404e8cc969c517`. The workspace wrapper exposes window membership, activation, desktop and Activity changes. The tracker uses the live window object, internal UUID, local generation and current workspace membership. Closing and removing a window invalidate the tracked object even if an older snapshot survives. Loading uses the script interface on the isolated bus. [Workspace wrapper](https://github.com/KDE/kwin/blob/v6.8.3/src/scripting/workspace_wrapper.h) · [Native window properties](https://github.com/KDE/kwin/blob/v6.8.3/src/window.h) · [Scripting interface](https://github.com/KDE/kwin/blob/v6.8.3/src/scripting/scripting.h)

The Quick effect path forwards Qt input events to its views. That API establishes event delivery. It supplies no protected attestation of a physical action bound to the exact proposal. The lock-screen and seat policies for a protected interaction remain unqualified. Script callbacks and same-user session services can supply observations to a fixture but carry no approval authority. [Quick effect input routing](https://github.com/KDE/kwin/blob/v6.8.3/src/effect/quickeffect.cpp) · [Input event forwarding](https://github.com/KDE/kwin/blob/v6.8.3/src/effect/offscreenquickview.cpp)

## Authorization consistency

The compositor's runtime window object belongs to KWin. Any copy that reaches a separate process can become stale after the compositor emits it. A presentation process can display or dismiss a correlated copy. The Rust authorization model still requires the exact coordinator and observer state, operation, window identity, topology and monotonic validity at consumption. Public interaction verification returns `UnverifiedInteraction`; a live preview and a matching display copy confer no capability.

A protected operation requires a compositor-owned turn with a retained live object, validated consent provenance and prompt ownership, current seat and lock facts, an authoritative coordinator checkpoint, and one-use consumption. The synthetic Rust world provides these facts within a synchronous exclusive borrow. Comparing data before and after an asynchronous transport does not produce that same guarantee.

## Deny-only preview handoff

1. The private launcher verifies a distinct empty D-Bus session, the KWin name owner's exact PID, KWin's executable version and the private Wayland socket. It loads the tracker against owned fixture surfaces.
2. The tracker retains the source object, validates UUID uniqueness and current placement, and freezes a snapshot. Its generation, runtime label and launcher-generated incarnation correlate one preview within that run. They do not represent an observer identity or coordinator checkpoint.
3. The launcher passes only source, destination and Activity strings to the Rust example along with the fixture bus identity. The example always calls the production refusal method, rejects unexpected fields, and returns the display projection only after receiving `UnverifiedInteraction`. It accepts no action, approval or capability fields. It is an explicit example target with stdin and stdout, without a D-Bus method.
4. The owned Qt Quick fixture attaches the projection to the source surface. The launcher compares the actual labels after a frame swap. Cancellation closes the source fixture, which KWin observes as window removal. The tracker invalidates on focus loss, native removal and topology drift. The independent compositor timer ends the pending preview and the launcher withdraws the matching view.
5. The Rust executor remains in the existing test-only authority. Native preview events cannot supply its internal confirmation marker.

The renderer is an ordinary Wayland client. Its frame swap establishes display within the owned fixture. The timer establishes delivery of an expiry event in the private KWin loop. Neither qualifies a trusted native authorization deadline, protected prompt placement or physical input. Propagation can lag between the compositor and the client, so the renderer is unsuitable as the authority for the current window state.

## Host isolation

The launcher compares KWin's owner and PID, virtual desktop identities and current desktop, effect properties and active output, Activity owner and IDs, the monitored configuration contents, autostart contents and extension paths. The host connection uses read-only D-Bus calls. The current host interface cannot enumerate windows and focus through these calls. The fixture does not connect to the host Wayland socket.

Desktop mismatches are reported by exact property path. Historical runs observed `desktopProperties.current` change within a test interval. This evidence shows a pre/post difference. It neither attributes the change to the fixture nor proves uninterrupted equality between snapshots. Checks fail on differences and never rewrite the host state. The native validation report records run outcomes and the private process and socket cleanup.
