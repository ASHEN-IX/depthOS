# Phase 3.1 — Native consent broker preflight

**PHASE 3.1 BLOCKED.** Canonical `main` is unborn, its index is empty, and neither linked track has a commit. `/home/ashenix/Projects/DepthOS-phase31` does not exist. The approved integration is represented by a source manifest. A normal linked worktree for `codex/phase-3-consent` needs a reviewed commit. This preflight created no branch, commit or prototype.

## Baseline and environment

The [integration readiness report](phase-3-integration-readiness.md) describes the approved combination and its initial-commit inventory. The current tree differs at the documented module moves and README layout addition. The offline authorization model has the same SHA-256 hash at its new path, `src/action_security/mod.rs`. A two-pass source inventory and writable-descriptor check covered 170 files. The backup at `/home/ashenix/Projects/DepthOS-backups/phase31-preflight-09x0a1co` contains those files and their verified hashes. Git metadata, build products, private evidence and host configuration contents were excluded. [Source and configuration hash inventory](../artifacts/phase31-preflight-yCue/source-manifest.json)

The installed package is `kwin 6.8.3-1`, and the binary reports `kwin 6.8.3`. The source review used upstream v6.8.3. Earlier runtime evidence is in the [Phase 1 report](phase-1.md), [POC-02G qualification](poc-02g.md) and [Phase 2.5 read-only spatial integration](phase-2-spatial-integration.md).

## Input provenance and protection

| Observation | Evidence it establishes |
| --- | --- |
| A KWin handler runs | Code executed within the compositor. |
| Input reaches a Quick scene effect | KWin routed it to the effect. Keyboard grab, mouse interception and active fullscreen effect state control routing. |
| Input arrives from a device | Device identity needs a separate policy at the native ingress point. KWin converts grabbed keyboard input into a Qt key event without passing the original KWin input device into that constructor. Pointer forwarding supplies a mapped Qt device, which by itself supplies no approval attestation. |
| A human approves the exact operation | Requires a deliberate confirmation edge bound to the proposal that was actually displayed. A shortcut opening the prompt, an accessibility callback, a QML signal or an action claim from a same-user process provides insufficient evidence. |
| A protected prompt captures approval | Requires verified compositor and broker integrity, controlled scripting and extensions, and input and focus policy that survives hostile callers. |

The EIS backend supports emulated input, including keyboard events sent through a KWin device. Prior private EIS tests demonstrate routing of injected input. The inspected effect interface provides no proof of a protected human approval. A future authority must explicitly reject unknown origins and distinguish emulated, remote and accessibility input according to a separately reviewed policy. [KWin input filter](https://github.com/KDE/kwin/blob/v6.8.3/src/input.cpp#L540-L555) · [Effect routing](https://github.com/KDE/kwin/blob/v6.8.3/src/effect/quickeffect.cpp#L525-L535) · [Pointer forwarding](https://github.com/KDE/kwin/blob/v6.8.3/src/effect/offscreenquickview.cpp#L407-L420) · [EIS input dispatch](https://github.com/KDE/kwin/blob/v6.8.3/src/plugins/eis/eiscontext.cpp#L323-L327)

KWin exposes script loading through its session-bus scripting interface. A protected broker must account for extension access and authority code replacement before trusting in-process callbacks. The lock-screen filter shows that KWin can route locked input separately, but the existing effects do not establish an authority-owned lock and focus policy. A lock transition, unknown lock state, focus or grab loss, and seat mismatch must cancel the proposal. [Scripting declarations](https://github.com/KDE/kwin/blob/v6.8.3/src/scripting/scripting.h#L293-L307) · [Scripting registration](https://github.com/KDE/kwin/blob/v6.8.3/src/scripting/scripting.cpp#L618-L622) · [Lock-screen input filter](https://github.com/KDE/kwin/blob/v6.8.3/src/input.cpp#L279-L328)

## Proposed deny-only experiment

Use a private-compositor native authority to own exactly one immutable pending proposal. Display a safe window label, operation, current placement, proposed destination, a clear dry-run consequence and confirm/cancel controls. Exclude protected window content. Keep confirmation and the synthetic sink within the authority. A shortcut may open the prompt. Every production interaction claim remains on the existing refusal path, and the Phase 3.0 positive path stays test-only. The [offline model](phase-3-security.md) already provides exact request, identity and epoch bindings, scope, expiry, replay protection and uncertainty handling.

KWin exposes an internal window UUID and can represent deleted windows retained for animation. QObject liveness alone is insufficient. Keep a guarded live-object reference, recheck exact object identity, workspace membership, placement, eligibility and current topology, then validate the observer/coordinator checkpoint, consume once and record a dry-run result on the compositor thread in one synchronous turn. Avoid nested event loops and user callbacks across the recheck/consume boundary. This design hypothesis needs runtime evidence. A prior Rust snapshot and a later QML action leave an unchecked interval. [KWin window lifecycle and identity properties](https://github.com/KDE/kwin/blob/v6.8.3/src/window.h#L319-L354)

Cancel on destruction, recreation, privacy or eligibility changes, focus loss, topology or Activity changes, owner replacement, observer loss, expiry and shutdown. Reject a conflicting proposal while one is pending. Cancellation before dispatch produces no sink record. Lost acknowledgement after simulated dispatch remains uncertain and must never trigger retry or retargeting. Native atomicity, real input provenance, protected prompt behavior and Phase 3.1 fault injection are untested. No issuer or adapter was added.

## Regression and host evidence

The unchanged application passed locked build, 111 Rust checks, formatting and strict Clippy. Offline authorization and the same-user D-Bus denial fixture passed. Observer parity passed seven scenarios and 30 packet comparisons. Private-bus interoperability, controller process lifecycle, spatial client lifecycle, schema refusal, JS model checks and offscreen panel and overview tests passed. [Regression logs](../artifacts/phase31-preflight-yCue/)

The existing isolated spatial suite passed 21 assertions. The isolated POC-02G Wayland suite passed 72. These regressions used private test UI and owned fixtures, including the existing Wayland suite's input and window-state exercises. Both passed their host comparisons and process cleanup. The Wayland suite also matched its original host window inventory. Four KDE configuration hashes still match the integration baseline. These results establish baseline regressions only. No confirmation prompt, authority binding, native race or physical-input provenance was exercised for Phase 3.1. [Spatial cleanup](../artifacts/p25/run-20261116T160609Z-13931d/cleanup.json) · [Wayland assertions](../artifacts/poc-02g/run-20261116T160624Z-1694a6/assertions.json) · [Host and window comparison](../artifacts/poc-02g/run-20261116T160624Z-1694a6/host-comparison.json)

## Next gate and rollback

Review the [updated initial-commit candidate](../artifacts/phase31-preflight-yCue/initial-commit-candidate.json) for the modular layout, then establish the committed baseline with explicit authorization and create the named linked worktree from that commit. The first native experiment can be deny-only, using content-addressed packages, private compositor/bus/XDG roots, guarded live-window identity and a synthetic sink. Keep consent issuance disabled until the protected input path and atomic compositor turn are demonstrated. A subsequent Phase 3.2 review requires that evidence, including injection and focus/lock adversarial tests, in addition to the full isolated regressions.

The only checkout addition is this report. Remove it if rolling back the preflight documentation. Keep or remove its private evidence separately. The backup is for restoring explicitly reviewed paths if needed. The existing private regression launchers manage their own packages and processes.
