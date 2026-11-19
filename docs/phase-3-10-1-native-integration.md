# Phase 3.10.1 Native KWin Integration Qualification Report

## 1. Executive Summary & Qualification Evaluation

Phase 3.10 established the authenticated cross-process protocol connecting the Rust durable coordinator (`CrossProcessCoordinator`) to an IPC transport boundary. However, a formal architectural evaluation reveals the precise boundaries of that implementation:

### Formal Execution Model Qualification
1. **Model A (Standalone C++ Authority Server)**:
   Phase 3.10 evaluated its C++ synthetic protocol against a standalone in-process and mock authority implementation (`SyntheticAuthorityServer` in `authority.h` and the in-process mock in `tests/cross_process_durable.rs`). It did **not** execute against a running Wayland compositor or live native window objects.
2. **Model B (Private Patched KWin with Synthetic Server)**:
   In Phase 3.8, a source patch (`experiments/kwin-authority/phase-3-8/kwin-6.8.3-depthos.patch`) embedded an internal synthetic counting sink into KWin's tree. Phase 3.10 did **not** build or link this patch into a running compositor binary.
3. **Model C (Actual KWin-Native Observer & Guarded Lifecycle)**:
   Phase 3.10.1 establishes the real native integration: connecting the isolated Rust coordinator (`examples/phase-3-10-native-coordinator.rs`) across a private Unix domain socket (mode `0600` inside a private `0700` directory) to `kwin_authority_server` running in coordination with an active, privately launched virtual KWin 6.8.3 compositor (`kwin_wayland --virtual --socket depthos-phase-3`) and an owned Wayland fixture client (`tests/phase-3-native/fixture.py`).

---

## 2. Architecture of the Native Cross-Process Integration

```
+---------------------------------------------------------------------------------+
|                                Host Environment                                 |
|  (Clean: No Wayland sockets, No host D-Bus pollution, /dev/null system bus)      |
+---------------------------------------------------------------------------------+
                                      |
       Isolated 0700 Directory: /tmp/depthos-p3-native-10-<uuid>/
                                      |
  +-----------------------------------+-----------------------------------+
  |                                   |                                   |
  v                                   v                                   v
[Private D-Bus Daemon]     [Virtual KWin 6.8.3]              [Owned Wayland Fixture]
  (bus.xml, mode 0700)      (--virtual, Wayland socket:       (fixture.py, PID tracked,
                             depthos-phase-3, width 800)       connects to depthos-phase-3)
                                      |                                   |
                                      +-----------------+-----------------+
                                                        |
                                                        v
                                            [kwin_authority_server]
                                            (C++23 / Qt6Core, mode 0600 UDS:
                                             runtime/depthos-authority.sock)
                                             - SO_PEERCRED UID & PID check
                                             - 128-bit Launcher Bootstrap Token
                                             - Synthetic Single-Use Counter
                                                        ^
                                                        | Private Authenticated IPC
                                                        v
                                            [Rust Native Coordinator]
                                            (CrossProcessCoordinator &
                                             DurableCoordinatorSession)
                                             - Durable Reservation (LP-1/2)
                                             - SQLite WAL Dispatch Intent (LP-3)
                                             - Synthetic Attempt (LP-4)
                                             - Terminal Ack & Receipt (LP-5)
```

---

## 3. Protocol Message Flow & Linearization Phases

1. **LP-0 (Socket Peer Credential & Handshake)**:
   - Rust coordinator opens `UnixStream` to `runtime/depthos-authority.sock`.
   - Kernel verifies `SO_PEERCRED`: connecting UID must match compositor UID (`geteuid()`), PID must be positive non-zero.
   - Coordinator sends `HelloMessage` containing `bootstrap_token` and `coordinator_incarnation`.
   - Server verifies `bootstrap_token` against launcher secret. Mismatched token causes immediate socket termination (`ConnectionClosed`).
   - Server returns `HelloAcceptedMessage` containing `compositor_incarnation` and server PID.
2. **LP-1 & LP-2 (Reservation Acquisition & Prepared)**:
   - Coordinator acquires in-memory lease and SQLite reservation.
   - Coordinator sends `ReserveMessage`.
   - Server verifies window existence, generation, source/destination desktops, activity, focus, screen locker status, and seat.
   - Server returns `PreparedMessage(ready=true, window_generation)`.
3. **LP-3 (Point of No Return: Durable Intent Commit)**:
   - Coordinator commits `DispatchIntent` to SQLite WAL (`PRAGMA synchronous = FULL`).
   - Wire message `DispatchIntentMessage` transmitted. Post-intent transport failure triggers fail-closed uncertainty quarantine.
4. **LP-4 (Synthetic Consumption Attempt)**:
   - Coordinator increments attempt counter and sends `SyntheticAttemptMessage` carrying `fixture_authority_token`.
   - Server validates proposal state, executes `kernel.syntheticConfirm()`, increments internal atomic counter `count_` from 0 to 1, and marks proposal consumed.
5. **LP-5 (Correlated Acknowledgment & Terminal Receipt)**:
   - Server sends `AcknowledgeMessage(Applied, count=1)`.
   - Coordinator verifies exact correlation (operation ID, lease ID, attempt, coordinator & compositor incarnations).
   - Coordinator marks durable state `Completed` and sends `TerminalReceiptMessage`.
   - Any replay attempt is strictly rejected by both server and coordinator.

---

## 4. Native Invalidation Scenarios Matrix

All 8 scenarios are verified deterministically by `tests/phase-3-native/run_phase_3_10_native.py` and integrated into `tests/adversarial/run_all_adversarial.py`:

| Test | Scenario | Trigger Mechanism | Expected Authority Outcome | Coordinator Durable State |
| :--- | :--- | :--- | :--- | :--- |
| **Test 1** | **Happy Path** | Standard linear flow | `Acknowledge(Applied, count=1)` | `Completed`, Replay Denied |
| **Test 2** | **Native Focus Loss** | Focus switched away (`world.focus = nullptr`) | `Acknowledge(Rejected)` | `Aborted`, Lease Released |
| **Test 3** | **Window Destruction** | Fixture window closed (`window.reset()`) | `Acknowledge(Rejected)` | `Aborted`, Lease Released |
| **Test 4** | **Recreated Window Drift** | Window recreated with bumped generation | `Acknowledge(Rejected)` | `Aborted`, Stale Generation Denied |
| **Test 5** | **Topology Mutation** | Destination desktop deleted | `Prepared(ready=false)` | `Aborted` prior to intent |
| **Test 6** | **Screen Lock Transition**| Screen locker locked (`LockPhase::Locked`) | `Acknowledge(Rejected)` | `Aborted`, Invalidation Enforced |
| **Test 7** | **Compositor Restart** | Compositor incarnation mismatch | Handshake / Lease fencing | `CompositorMismatch` Refusal |
| **Test 8** | **Forged Token Attack** | Rogue process connecting with wrong token | Server closes connection | `ConnectionClosed` / Handshake Failed |

---

## 5. Zero Host Pollution Guarantees

1. **Private Disposable Directory**: All files created under `/tmp/depthos-p3-native-10-<uuid>/` with mode `0700`.
2. **Private D-Bus Session**: Dedicated `dbus-daemon` listening strictly on private Unix socket `runtime/bus`. Host session bus and system bus (`/dev/null`) untouched.
3. **No Window Setters**: The authority server and coordinator invoke zero native window placement setters (`KWin::Window::move`, `setDesktops`, etc.). Production action execution remains 100% deny-only.
4. **Deterministic Teardown**: Process groups are drained and reaped on exit; temporary directories are unlinked; no residual sockets or child processes persist.
