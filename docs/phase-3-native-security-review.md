# Phase 3.6 independent native authority security review

Verdict: **NATIVE AUTHORITY REQUIRES COMPOSITOR CHANGES**. The experiment must remain deny-only. A standalone guarded-QObject synthetic kernel and the existing private KWin live-window tracker exercise separate properties; they do not combine into protected native approval.

## Review independence and scope

Agent 1 owns research and implementation until its explicit stable checkpoint. Agent 2 independently read the eight requested Phase 3 documents, production authorization/controller code, tracker, preview example, native launcher, private-bus denial fixture and production introspection tests before taking correction ownership. Exactly two subagents participate; neither delegates further. The lead verified accepted baseline `f73226097aced5f414dc1d1a901debb3eab510fc`, canonical main `6543b65213f24a733012ac5db50e69ed382d1cfb`, clean trees, ancestry and strict Git integrity. The isolated branch is `codex/phase-3-6`.

The historical native-consent document describes an earlier unborn-main state. Current source and preflight evidence supersede that historical Git observation. The evidence does not supersede its unresolved native trust gates.

## Independent source and build qualification

The reviewer independently observed package `kwin 6.8.3-1`, Qt Base `6.12.0-5`, Qt Declarative `6.12.0-4`, executable SHA-256 `33350d420b7b196c1e1f90ffa8c1280b4b3102d4e4b60e1929bad525be93df5e`, and ELF build ID `6eb58cf1f8d875ef625487ec2f404e8cc969c517`. The reviewed upstream v6.8.3 archive has SHA-256 `79e9cf97f4f40ceb86b426b8adb9a659d5c2b4d77f5a15c09768a51d807deb24`.

The reviewer independently checked the stronger packaging chain supplied by Agent 1: the downloaded distro PKGBUILD hash `d6023831666fecbe025107a305f12cca4d2751959e97e8f97a7e3126db013616` matches the cached package `.BUILDINFO`, the recipe has no patch step, its release archive hash `fd5cb783f220ea4526a9afee2f72b582c3e87ef7f91b19bc5c7a6662ffe7fbdc` matches the downloaded KDE archive, all 19 inspected security-sensitive files match between that release and the upstream tag archive, and the cached package executable matches the installed binary. This establishes correspondence with the declared package inputs. It is not a reproducible rebuild or independent proof of the original builder. Native ABI compatibility or a plugin linked to installed KWin is not demonstrated by compiling a QtCore-only standalone test.

Source cross-checks:

- [EffectsFilter keyboard routing](https://github.com/KDE/kwin/blob/v6.8.3/src/input.cpp#L565-L584) constructs a Qt key event from fields of the internal keyboard event. It does not propagate the original `InputDevice *` as an authenticated origin envelope. Receiving the resulting Qt event cannot establish physical human approval.
- [Internal input events](https://github.com/KDE/kwin/blob/v6.8.3/src/input_event.h) retain device pointers for keyboard and pointer events. The pointer identifies an in-process object; it is not by itself a protected device, session or physical-input attestation.
- [EIS routing](https://github.com/KDE/kwin/blob/v6.8.3/src/plugins/eis/eiscontext.cpp) emits the common device signal pipeline. [EisDevice](https://github.com/KDE/kwin/blob/v6.8.3/src/plugins/eis/eisdevice.h) is an `InputDevice`. Backend distinctions in private C++ code are useful to design a policy, but the experiment does not implement or qualify such a policy.
- [Script registration and loading](https://github.com/KDE/kwin/blob/v6.8.3/src/scripting/scripting.cpp#L671) exposes scriptable session-bus calls. `loadScript` constructs an in-process script from the supplied path. A session name, Unix UID or process identity does not establish a protected approval edge.
- [Offscreen input forwarding](https://github.com/KDE/kwin/blob/v6.8.3/src/effect/offscreenquickview.cpp#L418) uses Qt event delivery. It demonstrates delivery rather than protected prompt or provenance guarantees.

The native compositor must also defend its code and authority state against unauthorized extensions, debugger/process-memory access and untrusted configuration. This campaign does not prove a same-UID process-integrity boundary. Merely placing a component inside KWin would not satisfy that requirement.

## Threat and evidence classifications

The following classifications separate native results from synthetic conditions.

- Same-UID forged requests, claimed compositor owner and claimed click: verified denial by actual private D-Bus connections invoking the production refusal logic. Production introspection verifies the exact five-method read-only controller allowlist.
- Forged QML callbacks and local Qt events: verified denial by cancellation-only QML tests and absence of a proof issuer. The QML/Wayland fixture is an ordinary owned client, not a protected prompt.
- Physical keyboard/pointer, EIS/virtual, remote/portal, accessibility and unknown-origin input: no qualified native approval ingress. Unknown or claimed origins remain denied. Native injection-to-approval separation is unsupported, not positively verified.
- Wrong seat and locked/unknown lock state: synthetic refusal coverage only. The private KWin launcher uses `--no-lockscreen`; native lock and multiseat enforcement are unsupported.
- Focus loss, owned window destruction, cancellation, expiry and destination topology removal: private KWin deny-only tracker evidence. These observations do not authorize a sink operation.
- Native duplicate UUID, native observer replacement, protected prompt replacement, native coordinator drift and native compositor restart across authorization: unsupported as a positive native authority path. Synthetic tests exercise corresponding refusal predicates where represented.
- One-use consumption, lost acknowledgment and concurrent cancellation/confirmation: synthetic-only behavior. An uncertain result cannot be retried with the same pending operation.
- Worker teardown: controller lifecycle regressions and synthetic authority teardown. Native approval teardown cannot be qualified while no such approval component exists.

## Atomicity boundary

The Rust model owns a synthetic world through an exclusive synchronous borrow. The C++ test kernel owns no compositor objects: its QObjects and world facts are test-owned. Its intended guarantee is a synchronous check, pending removal and redacted in-memory count on its creator thread, with no callback, IPC, nested event loop or native setter in that interval. This is a synthetic single-process guarantee only.

The actual native tracker retains a live KWin object for read-only preview validation. It does not own the Rust coordinator checkpoint or a protected proof. Asynchronous IPC cannot extend either local turn into one atomic compositor/coordinator transaction. Any future positive implementation must provide authoritative coordinator state in the compositor turn, or continue to deny.

## Host isolation review

The launcher builds a private environment allowlist; clears inherited display/session endpoints by construction; sets isolated HOME and XDG roots; blocks the system bus; requires a distinct empty private session bus; verifies the private KWin owner's PID; and uses a socket under the private runtime directory. Disposable desktops and Activity belong to that verified private compositor. No extension is installed into host directories, and no native window setter is invoked by this experiment.

Host observation uses read-only D-Bus and local reads. It samples compositor owner/PID, desktops, effects, active output, Activity owner/list, monitored configuration contents, autostart contents and extension paths. Host native window inventory and focus remain unverified. Snapshot equality does not prove uninterrupted equality or attribute unrelated workstation activity.

Earlier Phase 3 runs detected host current-desktop and effects mismatches. These remain findings: strict comparisons failed and no host restoration was performed. Passing later snapshots does not waive or explain those earlier differences.

## Corrections and final validation

Agent 1 handed over commit `145cfde8a75755470f42424220684acc3991e0c8` and stopped editing. The review made these corrections:

- **Deadline overflow (synthetic defect).** An extreme injected `steady_clock::time_point::max()` caused signed overflow during the five-second addition. The checkpoint failed under UBSan with exit 1. Preparation now denies timestamps whose deadline cannot fit; the same control exits 0. A checked-in regression also accepts the last safely representable issue time. The existing check denies confirmation before issue and at expiry.
- **QObject affinity drift (synthetic defect).** Two deterministic controls moved a pending target or prompt to a foreign Qt thread; the checkpoint counted both. The kernel now checks target and prompt affinity on preparation and consumption, and guards its accessors against foreign-thread state reads. Both controls exit 0 after correction. This does not make `QPointer` safe for unsynchronized concurrent object destruction; fixture-owned objects and world changes must remain serialized.
- **Transient state changes.** Explicit cancellation survives restoration of focus or lock state in the test. The kernel samples current state and does not autonomously observe transitions. A future compositor authority must subscribe to changes and invalidate before any restored snapshot can be accepted.
- **Executable qualification (fixture evidence gap).** The inherited harness tied the private D-Bus owner to its child PID and checked reported version; it did not verify the running executable digest. The launcher now requires the absolute, root-owned installed path and exact SHA-256, compares file identity and digest before and after launch, and verifies the exact private D-Bus owner PID and runtime version. A wrong digest control refused before launch. Direct `/proc/<pid>/exe` inspection returned permission denied for this capability-bearing executable; a descriptor-based launch did not initialize Qt’s platform plugin. Both failures are retained in local evidence. The working check qualifies the installed artifact under ordinary trust in the system package path; it does not attest the running process image, every linked library, or extensions.

The corrections affect only the isolated C++ kernel, its tests, and private fixture launcher. The Rust implementation, production bus contract, tracker and QML consent view retain their current behavior. Raw evidence remains in ignored owner-only `artifacts/phase-3-6/`. See [validation](phase-3-native-validation.md) for final command and host results.
