# Phase 3.8 Protected Prompt Lifecycle and Extension Isolation

## Overview

Phase 3.8 establishes a compositor-owned prompt lifecycle within the private KWin 6.8.3 build and the offline authority verification model. In this architecture, authorization prompts cannot be delegated to untrusted Wayland clients, unprivileged helper processes, or mutable QML UI scripts. All prompt state, presentation, lifecycle binding, and retirement are anchored directly to compositor-internal lifecycle invariants.

The prototype strictly enforces deny-only defaults: no ordinary prompt callback, user event, button click, or client request is capable of minting consent or executing privileged desktop movements.

## 1. Compositor-Owned Prompt Lifecycle

### Immutable Content Binding

The native prompt abstraction (`DepthOS::CompositorPrompt` in `depthos_authority_experiment.h` and the C++ kernel in `authority.h` / `authority-kernel.cpp`) binds the following immutable fields at the moment of proposal creation:
- **Proposal ID (`uint64_t proposalId`)**: A strictly monotonically increasing, unique identifier generated per compositor instance.
- **Guarded Window Identity**:
  * Exact native `KWin::Window*` pointer with lifecycle guarded tracking (`QPointer<KWin::Window>`).
  * Window UUID (`QString windowUuid`).
  * Generation counter (`uint64_t generation`).
- **Source and Destination Desktops**:
  * Source desktop identifier (`QString sourceDesktop`).
  * Destination desktop identifier (`QString destinationDesktop`).
  * Immutable desktop distinction requirement (`sourceDesktop != destinationDesktop`).
- **Activity (`QString activity`)**: The exact plasma/compositor activity associated with the window.
- **Compositor Incarnation (`QString compositorIncarnation`)**: The UUID generated upon compositor startup, ensuring prompt proposals cannot survive or be replayed across compositor restarts.
- **Expiration Deadline (`std::chrono::steady_clock::time_point deadline`)**: A strictly monotonic steady-clock deadline bounding prompt validity.

Once created, these fields are immutable and read-only. An ordinary Wayland client or QML script possesses no interface or D-Bus method to mutate the proposed destination, target window, or proposal identity.

### Rendered-Frame Visibility and Dismissal

A prompt proposal cannot be considered active or eligible for confirmation until the compositor has committed and presented its rendered frame. The lifecycle enforces:
1. **Creation**: Prompt instantiated in pending state; initial rendered flag is false (`m_promptRendered == false`).
2. **Presentation / Rendered Verification**: The compositor's render pipeline triggers `markRendered()` upon actual frame presentation (verified via scene item visibility and offscreen output rendering signals).
3. **Dismissal**: Any retirement event (focus change, lock transition, deadline expiration, desktop switch, window destruction, or user rejection) triggers `dismiss()` synchronously:
   * Sets `m_promptVisible = false` and `m_promptRendered = false`.
   * Unlinks the active proposal (`m_pending = nullptr`).
   * Logs native telemetry (`DEPTHOS_NATIVE38 prompt rendered` / `DEPTHOS_NATIVE38 prompt dismissed`).

In the offscreen test runner and private virtual compositor runs (`run.py`), prompt presentation and dismissal were observed and verified across all tested proposals:
- In synthetic fixture mode: 8 prompt presentations and 8 prompt dismissals verified.
- In build mode (pure deny-only): 6 prompt presentations and 6 prompt dismissals verified.

## 2. Extension Isolation and Architectural Limitations

A central question in compositor-owned security architectures is whether prompt presentation and input handling can be isolated from extensions (effects, plugins, and scripts) loaded into the compositor process.

### In-Process Architecture Limitations

In KWin (and similar monolithic Wayland compositors), dynamic plugins, rendering effects, and scripting backends run inside the single compositor address space (`kwin_wayland`):
- **Shared Memory Space**: Any dynamically loaded C++ plugin or effect has unrestricted access to the compositor's heap, global variables, and Qt object trees.
- **Scene Graph Hooking**: KWin effects subclass `KWin::Effect` and hook directly into `prePaintScreen`, `paintScreen`, `prePaintWindow`, and `paintWindow`. A malicious or compromised in-process effect can:
  * Alter, suppress, or obscure the rendered pixels of a prompt.
  * Fabricate scene presentations while concealing the prompt from the physical display.
  * Inspect pointer memory to read internal tokens or proposal state.
- **Input Filter Chain**: KWin routes raw libinput events through an internal chain of `InputEventFilter` objects. Loaded plugins can install filters that observe, drop, or inject synthetic input events.

### Security Conclusion and Architectural Boundary

Because in-process plugins execute with identical operating system privileges (UID, PID, memory map, capabilities) as the core compositor, **true isolation of prompt rendering cannot be achieved inside the compositor process itself**.

To achieve genuine prompt isolation against compromised native components, the security architecture requires an externalized, out-of-process trusted UI architecture or an OS-level capability separation boundary (such as a dedicated privileged prompt agent running under a separate UID with hardware-backed overlay planes or DRM KMS lease isolation).

Within the current monolithic KWin architecture, this limitation is explicitly documented:
> **Documented Limitation**: Loaded native code (plugins, effects, binary scripts) shares KWin's process memory and can replace, inspect, or alter compositor-owned prompts. In this phase, the prompt experiment provides structural and lifecycle integrity against external Wayland clients and unprivileged D-Bus peers, but does not provide sandboxed extension isolation within the compositor process itself.

## 3. Deny-Only Consent Policy

In accordance with Phase 3.8 requirements:
- No ordinary UI callback or Wayland event can mint consent.
- Synthetic consent tokens exist strictly within isolated test fixtures (`authority-kernel.cpp` and `CountingSyntheticSink`).
- In production builds and external interfaces, all consent issuance remains completely disabled, and any attempt to invoke desktop manipulation or capability minting fails closed with unverified interaction errors.
