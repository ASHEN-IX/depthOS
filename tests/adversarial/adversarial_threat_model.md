# Phase 3.8 Adversarial Threat Model and Security Architecture

## 1. Executive Summary & Purpose

Phase 3.8 establishes the **Protected Consent Boundary and Rust Coordinator Lease** for DepthOS. This threat model details the security assumptions, trust boundaries, attacker profiles, attack vectors, and validation criteria governing:
1. Native Prompt Integrity (Compositor-owned prompt display and binding).
2. Native Lock and Seat Transitions (Synchronous state invalidation and anti-resurrection guarantees).
3. Injected Input Misclassification (Diagnostic classification vs. authorization immunity).
4. Rust Coordinator Lease (Exclusive authority turns, incarnation fencing, deadlock prevention).
5. Crash-Cut & Uncertain Completion (Fault isolation, quarantined state, zero-replay invariant).
6. Plugin & Library Isolation (Private runtime integrity vs. hostile same-UID code).
7. Production Read-Only D-Bus Allowlist (Refusal of unauthorized or synthetic claims).

---

## 2. Trust Boundaries & Security Architecture

```
+-----------------------------------------------------------------------------------+
| Host System (Untrusted by private test fixture; must remain completely untouched) |
+-----------------------------------------------------------------------------------+
       |
       | Process isolation (Private UID, separate PID namespace, private XDG/runtime)
       v
+-----------------------------------------------------------------------------------+
| Private KWin Compositor Instance (Owner-only disposable directories)             |
|                                                                                   |
|  +-----------------------------------------------------------------------------+  |
|  | Native Authority Kernel & Compositor-Owned Prompt                          |  |
|  |  - In-process QObject & QPointer lifetime guards                            |  |
|  |  - Exact window UUID + Generation binding                                   |  |
|  |  - Synchronous lock (`KSldApp::aboutToLock`, `Locked`, `Unlocked`) observer |  |
|  |  - Active seat observer (`seat0`) & focus observer                          |  |
|  |  - Input device ingress spy (libinput, EIS, virtual/fake)                    |  |
|  +-----------------------------------------------------------------------------+  |
|                                                                                   |
|  [CRITICAL LIMITATION: Same-address-space plugins/effects/scripts share memory]  |
+-----------------------------------------------------------------------------------+
       ^
       | Local Unix Socket / Private D-Bus (Scoped protocol, NO consent minting)
       v
+-----------------------------------------------------------------------------------+
| Rust Coordinator & Action Security Layer (depthos-core)                           |
|  - Coordinator Lease Manager (Pinned checkpoint, single-turn exclusive lock)      |
|  - Incarnation Fencing (Compositor UUID, Coordinator Epoch, Topology Rev)         |
|  - One-shot Consumption & Non-replayable Tombstones                               |
|  - Quarantine State Machine for Uncertain Completion                              |
|  - Production Read-Only Allowlist Enforcement                                     |
+-----------------------------------------------------------------------------------+
```

### Trust Boundary Rules
1. **Compositor In-Process vs. External Client**: An unprivileged Wayland client or D-Bus peer cannot forge a prompt, read prompt internal state, or inject approval tokens.
2. **Same-Address-Space In-Process Extensions**: Ordinary loaded native code (KWin effects, plugins, scripts) executes in the compositor's process memory. While private runs isolate plugin search paths, the architectural limitation is explicitly acknowledged: in-process code has memory and render-pipeline access. True isolation requires future out-of-process or microkernel boundaries.
3. **Coordinator vs. Compositor IPC**: IPC is asynchronous and failure-prone. Network/socket timeouts NEVER imply that an operation was not executed by the sink.
4. **Input Ingress vs. Consent Minting**: Input classification (physical vs. EIS vs. fake) is strictly diagnostic metadata. An input classification can NEVER mint an authorization token.

---

## 3. Attack Vectors & Invariant Assertions

### Attack Vector 1: Native Prompt Integrity
- **Attack 1.1: Wayland/QML Content Spoofing & Tampering**
  - *Threat*: A rogue client attempts to forge proposal fields (e.g. altering target window, destination desktop, or activity) or inject a mock prompt into the scene graph.
  - *Mitigation & Invariant*: Immutable binding of proposal ID, window UUID, window generation, source desktop, destination desktop, activity, compositor incarnation, and deadline. Any mismatch at consumption results in immediate denial.
- **Attack 1.2: Ordinary Loaded Native Code Access**
  - *Threat*: Host plugins or effects loaded into KWin inspecting prompt buffers.
  - *Mitigation & Invariant*: Private KWin launches explicitly scrub plugin search directories and reject unauthorized shared objects.
- **Attack 1.3: Rendered-Frame Visibility & Dismissal**
  - *Threat*: Prompt is hidden, 0x0 size, offscreen, or dismissed before confirmation.
  - *Mitigation & Invariant*: Confirmation requires prompt presence, active ownership, and visible state. Dismissal or Escape immediately marks the proposal terminal.

### Attack Vector 2: Native Lock, Seat, and Focus Transitions
- **Attack 2.1: Lock Timing Race (`aboutToLock`)**
  - *Threat*: User locks the screen while a proposal is pending. An attacker attempts to confirm the proposal before the lock screen finishes drawing.
  - *Mitigation & Invariant*: `KSldApp::aboutToLock` synchronously cancels all pending proposals before `Locked` state is reached.
- **Attack 2.2: Unknown Lock State Refusal**
  - *Threat*: Headless or test environments run with `--no-lockscreen`, leaving lock status unknown (`None`/`nullopt`).
  - *Mitigation & Invariant*: An unknown lock status is treated as unverified/locked and strictly denied.
- **Attack 2.3: State Restoration & Proposal Resurrection Attack**
  - *Threat*: Proposal is prepared -> focus is lost (or screen locked) -> focus is restored (or screen unlocked) -> attacker attempts confirmation.
  - *Mitigation & Invariant*: Any state transition (focus shift, lock, seat change) calls synchronous cancellation which permanently marks the proposal instance retired. State restoration MUST NEVER resurrect an invalidated proposal!
- **Attack 2.4: Seat Switching & Mismatched Input**
  - *Threat*: Input originates from a secondary seat (`seat1`) or empty seat.
  - *Mitigation & Invariant*: Active seat must match the bound seat (`seat0`) exactly.

### Attack Vector 3: Injected Input Misclassification
- **Attack 3.1: EIS / Remote Portal Device Masquerading**
  - *Threat*: Remote desktop or EIS input injection attempting to pose as local hardware.
  - *Mitigation & Invariant*: Device classifier tags EIS devices as `Eis` / `Portal`.
- **Attack 3.2: Fake Input & Virtual Device Ingress**
  - *Threat*: Virtual input synthesizers injecting key events without a physical sysfs path.
  - *Mitigation & Invariant*: Any device without confirmed sysfs hardware backing is tagged `Unknown` or `Fake/Virtual`.
- **Attack 3.3: Classification Immunity (Non-Minting Invariant)**
  - *Threat*: An attacker manages to obtain a `ClaimedPhysical` classification label and tries to use it as authorization.
  - *Mitigation & Invariant*: Origin classification is purely diagnostic. All origins, including `ClaimedPhysical`, return `Result::Denied` when submitted to authority verification.

### Attack Vector 4: Rust Coordinator Lease
- **Attack 4.1: Concurrent Conflicting Proposals**
  - *Threat*: Multiple background tasks attempt to move the same window or move windows into conflicting arrangements concurrently.
  - *Mitigation & Invariant*: The coordinator lease grants exclusive access to a given window and checkpoint. Any concurrent lease request for an active window is rejected with conflict.
- **Attack 4.2: Incarnation & Epoch Fencing**
  - *Threat*: Compositor restarts (new UUID) or coordinator restarts (new epoch). Stale in-flight leases attempt consumption.
  - *Mitigation & Invariant*: Strict incarnation checking rejects any lease whose compositor UUID, coordinator epoch, or topology revision differs from current live state.
- **Attack 4.3: Expiration & Monotonic Clock Boundaries**
  - *Threat*: Monotonic clock reversal or evaluation at exact expiration timestamp `now == expires`.
  - *Mitigation & Invariant*: Monotonic time checks reject any negative time delta. `now >= expires` is strictly expired and denied. Extreme or overflowing deadlines are rejected at capture.
- **Attack 4.4: Revocation Races**
  - *Threat*: Simultaneous revocation and consumption.
  - *Mitigation & Invariant*: Atomic state transition: once revoked, consumption returns `Denied`; once consumed, revocation cannot reverse dispatch, and replay returns `Denied`.

### Attack Vector 5: Crash-Cut and Uncertain Completion
- **Attack 5.1: Crash Before Consumption**
  - *Threat*: Process or coordinator crashes after proposal capture but before dispatch.
  - *Mitigation & Invariant*: Proposal expires safely in memory; no native sink invocation occurs.
- **Attack 5.2: Lost Acknowledgment / Crash After Consumption**
  - *Threat*: Action is dispatched to native sink, but sink crashes, IPC drops, or acknowledgment is lost.
  - *Mitigation & Invariant*: Outcome is marked `Uncertain`. The coordinator enters a quarantined state for the affected window. Automated retry or resubmission is strictly prohibited. State can only be cleared via authoritative durable state reconciliation.

### Attack Vector 6: Plugin and Library Isolation
- **Attack 6.1: Host Plugin Hijacking in Private Runtime**
  - *Threat*: KWin loading host system plugins or untrusted extensions from default library paths.
  - *Mitigation & Invariant*: Private launcher overrides `QT_PLUGIN_PATH`, `LD_LIBRARY_PATH`, and verifies SHA-256 of loaded libraries.

### Attack Vector 7: Production Authorization Refusal
- **Attack 7.1: Production D-Bus Method Injection**
  - *Threat*: Rogue client on system/session bus calling D-Bus methods to trigger window movement.
  - *Mitigation & Invariant*: Production D-Bus interface contains strictly read-only diagnostic and introspection methods. All mutation/authorization endpoints are absent or return explicit access denied.
