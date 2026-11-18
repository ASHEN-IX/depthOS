# Phase 3.8 Input Provenance, Seat Invariants, and Lock Observation

## Overview

Phase 3.8 deepens the compositor authority prototype by formalizing the native environment preconditions required for desktop manipulation proposals:
1. **Direct Screen Locker Observations**: Tracking lock acquisition initiation, active lock, and confirmed unlock states.
2. **Seat Invariants and Focus Tracking**: Binding proposals to active seats and focus transitions.
3. **Non-Resurrection Invariant**: Guaranteeing that state invalidation is terminal and cannot be revived by subsequent state restoration.
4. **Input Provenance Classification Matrix**: Classifying ingress input events across seven distinct origins as diagnostic evidence without translating device labels into authorization capabilities.

## 1. Native Lock State Transitions

### Direct Observations and Signals

KScreenLocker in KDE Plasma / KWin 6.8.3 manages session locking via `KSldApp` and `KWin::WaylandServer::isScreenLocked()`. The Phase 3.8 authority prototype attaches direct signal connections to observe session lock lifecycle events:

- **`KSldApp::aboutToLock`**: Emitted immediately when screen locking is requested or initiated (e.g. idle timer expiration, user lock hotkey, system sleep preparation), prior to the lock screen surface rendering. This early signal allows immediate, preemptive cancellation of pending proposals.
- **`KSldApp::Locked` / `WaylandServer::isScreenLocked() == true`**: The session is actively locked. All pending proposals are forcibly discarded, prompt UI is dismissed, and any incoming proposals are immediately refused.
- **`KSldApp::Unlocked` / `WaylandServer::isScreenLocked() == false`**: Confirmed unlocked session state. Proposals are only considered valid when explicit `KSldApp::Unlocked` confirmation is present.

### Terminal Retirement and Non-Resurrection

A fundamental invariant enforced in both C++ (`authority.h`, `authority-kernel.cpp`, `depthos_authority_experiment.h`) and Rust (`src/action_security/lease.rs`, `tests/consistency_integration.rs`):

> **Non-Resurrection Invariant**: Once a proposal is retired by a lock transition, focus change, seat switch, or timeout, its retirement is permanent. A subsequent restoration of the original state (e.g. unlocking the screen, switching focus back to the original window) **must never resurrect** the retired proposal ID, token, or lease reservation.

Implementation mechanism:
- Upon transition (e.g. `aboutToLock`), `m_pending` is reset to `nullptr` and its rendered state is destroyed.
- The proposal ID is marked as retired and cannot be reused.
- Restoring `unlocked` state does not repopulate `m_pending`. A completely fresh proposal, with a new monotonically incremented proposal ID, new steady-clock deadline, and re-evaluated context snapshot, is strictly required.

### Runtime Compositor Lock Transition Verification

The test harness (`tests/phase-3-native/run.py` and `tests/phase-3-8-native/test-launcher.py`) exercises actual lock transitions in the private compositor instance:
1. Compositor starts with locker active (`lock=fixture-unlocked`).
2. An `aboutToLock` signal is simulated/emitted: the pending proposal is immediately retired (`reason: lock-acquisition-initiated`).
3. The screen lock state transitions to `Locked`: any interaction claim is refused.
4. The screen lock state returns to `Unlocked`: verified that no proposal is resurrected (`pending=absent`).

## 2. Seat Invariants and Device Mapping

### Seat Context and Focus Tracking

Every proposal is bound to:
- **Active Seat (`m_activeSeat`)**: Identified by seat string (e.g. `"seat0"`). A proposal is valid only if input and interaction occur on the exact seat where the window is focused.
- **Device-to-Seat Mapping**: In multi-seat configurations, input devices are assigned to distinct seats. An input event originating from a device mapped to `seat1` cannot satisfy a prompt displayed on `seat0`.
- **Seat/Focus Loss Invalidation**: If the focused surface loses keyboard or pointer focus, or if the active seat changes, the proposal is immediately discarded.

### Multi-Seat Qualification Status

Real physical multi-seat hardware validation (multiple independent physical graphics cards, seats, and physical keyboards/mice) was unavailable in the headless/virtual test environment.

Consequently:
- Model coverage and synthetic fixture tests (`authority-kernel.cpp` and `test-launcher.py`) comprehensively verify multi-seat rejection and device-to-seat mismatch denials.
- **Status Label**: Native multi-seat qualification is formally marked **UNSUPPORTED** in this phase. Multi-seat configurations remain fail-closed (denied).

## 3. Input Provenance Classification Matrix

### Internal Input Ingress Classification

To distinguish between physical input, automated scripting, synthetic events, and remote portals, KWin's input pipeline was augmented with source provenance tracking (`depthos_input_origin.h` and `depthos_authority_experiment.h`).

Seven discrete source origins are classified at input ingress:

| Origin ID | Origin Name | Description | Authority Treatment |
|---|---|---|---|
| 0 | `Unknown` | Default unclassified input or legacy event stream. | **DENIED** |
| 1 | `LocalCandidate` | Events from physical evdev/libinput devices on the local seat. | **DENIED** (Diagnostic evidence only) |
| 2 | `Eis` | Input arrived via Emulated Input Server (libei / EIS protocol). | **DENIED** (Synthetic / Emulated) |
| 3 | `Portal` | Input injected through XDG Desktop Portal remote desktop APIs. | **DENIED** (Remote / Synthetic) |
| 4 | `FakeOrVirtual` | Input injected via internal virtual test interfaces or fake input plugins. | **DENIED** (Synthetic / Test fixture) |
| 5 | `Accessibility` | Input injected via accessibility (AT-SPI) synthesized events. | **DENIED** (Accessibility synthetic) |
| 6 | `Remote` | Remote administration protocols (e.g. VNC, RDP, SSH forwarding). | **DENIED** (Remote session) |

### Invariant: Diagnostic Evidence, Never an Approval Capability

A crucial security principle is upheld:
> **Provenance Invariant**: Input source classification serves strictly as diagnostic telemetry and defensive evidence. Under no circumstances is a `LocalCandidate` classification converted into an authorization capability or used to automatically mint consent.

Even if an input event originates from a confirmed local physical keyboard/mouse:
- It cannot bypass the user consent prompt.
- It cannot mint an interaction token.
- All 7 origins remain completely denied for automated capability issuance.
- Unsupported accessibility, remote/portal, or virtual input streams are rejected unconditionally.
