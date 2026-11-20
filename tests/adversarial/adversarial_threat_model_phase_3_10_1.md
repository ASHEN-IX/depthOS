# Phase 3.10.1 Adversarial Threat Model & Security Architecture Specification

**Author**: DepthOS Phase 3.10.1 Independent Security Reviewer (Agent 2)  
**Target Repository**: `/home/ashenix/Projects/DepthOS-phase-3-9`  
**Target Branch**: `codex/phase-3-10-1`  
**Classification Date**: November 2026  
**Scope**: Real KWin Native Integration, IPC & Process Trust, Host Isolation, Durable Linearization, and Authoritative Reconciliation  

---

## 1. Executive Summary & Purpose

Phase 3.10.1 transitions DepthOS from an in-memory synthetic C++ mock (`SyntheticAuthorityServer`) to a **fully integrated, production-grade cross-process architecture capable of orchestrating real native KWin Wayland compositor instances** while preserving ironclad host isolation, durable crash consistency, peer authentication, and fail-closed reconciliation.

Phase 3.10 established the foundation of the cross-process wire framing and coordinator lease protocols. However, forensic review identified critical gaps:
1. All native testing in Phase 3.10 occurred exclusively against synthetic in-process test doubles; real Wayland compositor integration remained unexercised.
2. Legacy reports cited non-existent test scripts (`tests/phase-3-native/authority-kernel.py` and `tests/phase-3-8-native/test-launcher.py`) and inaccurate test tallies (196 actual passing vs 209 claimed).
3. Complex real-world failure modes—such as native window destruction during flight, window focus loss, compositor crash/restart during execution, input seat lock transitions, symlink/socket substitution, PID reuse, and resource identity masking—require formalization and adversarial test coverage.

This specification establishes the authoritative threat model, attack vectors, and verification requirements for Phase 3.10.1.

---

## 2. Process Topology & Trust Boundaries

The system architecture partitions authority across five distinct operational components:

```mermaid
flowchart TD
    subgraph LauncherProc ["Process 0: DepthOS Supervisor / Launcher"]
        TokenGen["Ephemeral Bootstrap Token Generator\n(CSPRNG 256-bit single-use tokens passed via private FDs)"]
        ProcSupervisor["Process Lifecycle & Incarnation Watchdog"]
    end

    subgraph CoordinatorProc ["Process 1: DepthOS Coordinator (Rust)"]
        DurableJournal["Encapsulated SQLite Journal (WAL)\n(PRAGMA synchronous=FULL, Immediate Tx, No Raw SQL)"]
        LeaseManager["CoordinatorLeaseManager\n(Monotonic Tick Clock, Exclusion Locks, Generation Fencing)"]
        ReconGateway["Authoritative Reconciliation Gateway\n(Strict Provenance, Freshness seq > recorded_seq)"]
        CoordServer["IPC Endpoint (Rust Tokio)\n(SO_PEERCRED EUID/PID, 0700/0600 File Mode, State FSM)"]
    end

    subgraph CompositorProc ["Process 2: KWin Wayland Compositor (C++/Qt)"]
        KWinCore["KWin Compositor Core (Wayland Server)\n(Real Wayland Windows, Scene Graph, Window Lifecycle)"]
        NativeAuthority["Native Authority Plugin / Endpoint\n(Focus Fencing, Seat Lock Verification, Incarnation Checks)"]
    end

    subgraph ObserverProc ["Process 3: DepthOS Authoritative Observer"]
        ObserverEngine["Wayland Window State Observer\n(Authoritative Provenance depthos.observer.authoritative)"]
    end

    subgraph HostSystem ["Host Operating System (Untrusted Boundary)"]
        HostUsers["Other Processes (Same UID or Differing UIDs)"]
        HostCompositor["Production Host KWin / Desktop Session\n(MUST REMAIN COMPLETELY UNTOUCHED)"]
    end

    LauncherProc -->|Private FD / Pipes| CoordinatorProc
    LauncherProc -->|Private FD / Pipes| CompositorProc
    LauncherProc -->|Private FD / Pipes| ObserverProc

    CoordinatorProc <-->|Private UDS (0700 dir, 0600 sock)| CompositorProc
    ObserverProc -->|Private UDS (Signed / Token)| ReconGateway
    HostUsers -.->|Blocked: No Token, SO_PEERCRED, 0700| CoordServer
    HostUsers -.->|Blocked: No Token, SO_PEERCRED, 0700| CompositorProc
```

### Trust Boundary Rules:
1. **Host Isolation Invariant**: The production host session (including host `/usr/bin/kwin_wayland`, host D-Bus, active host windows, and user desktop) is an immutable boundary. All KWin integration testing in Phase 3.10.1 must run against isolated, virtual/headless instances on private Wayland socket names (`wayland-depthos-test-*`) and private D-Bus daemon instances.
2. **Mutual Process Authentication**: Every IPC connection across process boundaries must prove EUID match via `SO_PEERCRED`, verified strictly positive PID, and present an unguessable 256-bit bootstrap token delivered out-of-band by the launcher.
3. **Point-of-No-Return Linearization**: The coordinator must commit every state transition to SQLite WAL before emitting the corresponding wire frame.
4. **Compositor Authority Exclusivity**: Only the authorized compositor process holds the capability to manipulate native Wayland surfaces. All synthetic requests without an unconsumed native proposal must fail closed.
5. **Reconciliation Exclusivity**: Only the designated authoritative observer can clear uncertainty quarantine; all other entities (including clients and unverified compositors) are strictly forbidden from clearing exclusion locks.

---

## 3. Failure Taxonomies (Classes A–E)

| Failure Class | Scope / Description | Expected System Behavior | Supported in Testbed? |
|---|---|---|:---:|
| **Class A** | Transport Failure (UDS disconnect, socket drop, frame truncation, packet corruption). | Immediate connection drop, session termination, uncertainty quarantine on in-flight ops. | **YES** |
| **Class B** | Process Failure (Coordinator crash, Compositor crash, Observer crash). | Multi-process incarnation fencing, clean recovery from SQLite WAL, automatic quarantine. | **YES** |
| **Class C** | State Divergence (ACK says Applied, Observer sees unchanged or rejected state). | Fail-closed escalation to permanent security anomaly quarantine; requires operator intervention. | **YES** |
| **Class D** | Concurrency / Race (Input seat stolen during move, window destroyed in-flight, focus lost). | Atomic proposal abortion, non-mutation guarantee, resource release or quarantine. | **YES** |
| **Class E** | Hardware / Power Loss (Sudden kernel panic, hardware power cut, catastrophic filesystem loss). | Best-effort SQLite WAL integrity via `PRAGMA synchronous = FULL` (2). **Formally unsupported**. | **NO** (Unexercisable) |

---

## 4. Attack Vectors & Verification Requirements

### Attack Vector 1: IPC and Process Trust Boundaries

#### 1.1 Unauthorized Same-UID Attack
- **Threat**: In a multi-tasking desktop environment, untrusted scripts or compromised apps running under the same user UID attempt to connect to the DepthOS private Unix Domain Socket.
- **Defense**:
  * Socket directory must be created with `0700` (`S_IRUSR | S_IWUSR | S_IXUSR`) owned strictly by `getuid()`.
  * Socket file must have permissions `0600` (`S_IRUSR | S_IWUSR`) owned strictly by `getuid()`.
  * Transport layer must query `SO_PEERCRED` on connection accept, verifying `cred.uid == getuid()`.
  * Transport layer requires immediate receipt of `Hello` message carrying the correct 256-bit ephemeral bootstrap token within 500 ms; failure results in immediate connection reset.
- **Verification**: Unauthorized client connecting with valid UID but empty, incorrect, or missing bootstrap token is rejected with `InvalidToken` and socket is closed.

#### 1.2 Invalid PID Credentials & PID Reuse
- **Threat**: Attacker attempts to spoof credentials or exploits Linux PID rollover to present a stale or unmapped process ID (`pid <= 0`, negative PID in unmapped user namespace, or reused PID).
- **Defense**:
  * Strict validation that `raw_pid > 0` before cast to integer types.
  * Validation that PID exists in `/proc/$PID`.
  * Ephemeral bootstrap tokens are single-use and bound to the launcher session, rendering PID reuse attacks toothless because an imposter process cannot obtain the token.
- **Verification**: Negative PID, PID 0, or dead PID connections are rejected at credential validation.

#### 1.3 Socket Replacement & Symlink / Ownership Attacks
- **Threat**: Attacker creates a symlink pointing the socket directory or socket path to `/tmp` or another user's directory to redirect or snoop traffic.
- **Defense**:
  * File metadata check enforces `!meta.file_type().is_symlink()`.
  * Path resolution validates every component belongs to the current user.
  * `verify_private_directory` checks `meta.uid() == my_uid` and permissions `0700`.
  * `verify_private_socket` checks `meta.file_type().is_socket()`, `meta.uid() == my_uid`, and permissions `0600`.
- **Verification**: Symlinked socket paths, directories with permissive permissions (`0777`, `0755`), or wrong owner UID fail pre-bind validation.

#### 1.4 Bootstrap Capability Replay
- **Threat**: Attacker observes or intercepts a bootstrap token from `/proc/$PID/environ` or traces, and attempts to open a second connection using the same token.
- **Defense**:
  * Bootstrap tokens are strictly single-use. Once an initial `Hello` is authenticated, the token is consumed and invalidated by the server.
  * Attempting to reuse the token in a secondary or concurrent connection returns `TokenAlreadyConsumed` and drops the connection.
- **Verification**: Multiple sequential or concurrent connections using the same bootstrap token fail closed after the first successful connection.

#### 1.5 Process Restarts & Session FSM Violations
- **Threat**: Process terminates mid-handshake or reconnects with different credentials; client emits duplicate `Hello` or `HelloAccepted` while session is already `Active`.
- **Defense**:
  * Session state machine: `Unauthenticated` -> `Active` -> `Terminated`.
  * Receiving `Hello` or `HelloAccepted` in `Active` state triggers immediate protocol error (`InvalidTransition`), moves session to `Terminated`, and closes socket.
- **Verification**: Injecting duplicate `Hello` or `HelloAccepted` terminates session immediately.

#### 1.6 Malformed & Oversized Wire Frames
- **Threat**: Attacker sends frames with corrupted magic bytes, protocol version mismatch, truncated payload, or payloads exceeding maximum length to cause heap overflow or memory exhaustion.
- **Defense**:
  * Fixed 20-byte wire header:
    - `0..4`: Magic `0x44504F53` ("DPOS")
    - `4..6`: Version `0x0100` (1.0)
    - `6..8`: Message Type
    - `8..16`: Monotonic Stream Sequence Number
    - `16..20`: Payload Length (Big Endian)
  * Frame bounds: `payload_len <= 65536` (64 KiB).
  * Monotonic sequencing: Every packet must have `stream_seq == expected_seq + 1`.
- **Verification**: Frames with bad magic, version mismatch, payload > 64 KiB, or non-monotonic sequence numbers are dropped with fatal `IpcError`.

---

### Attack Vector 2: Native Authority & Real KWin Integration

#### 2.1 SyntheticAttempt Without Native Proposal
- **Threat**: Coordinator or compromised client requests execution of a window action via `SyntheticAttempt` without a prior successful `Reserve` / `prepare` call.
- **Defense**: Native authority checks `kernel_.isPending() && kernel_.pending()->proposalId == opId`. If no proposal is pending, execution is denied.
- **Verification**: Direct invocation of `SyntheticAttempt` returns `ready: false` / `applied: false`.

#### 2.2 SyntheticAttempt After Window Focus Loss
- **Threat**: User shifts focus to a sensitive window (e.g., password manager, terminal) after `Reserve` was issued but before `Attempt` executes.
- **Defense**:
  * Native authority captures active window handle during `Reserve`.
  * Before executing `SyntheticAttempt`, native authority verifies that the target window remains the actively focused window.
  * If focus has shifted, the pending proposal is invalidated and execution returns `Result::Denied(FocusLost)`.
- **Verification**: Simulating focus switch between prepare and attempt aborts execution and increments zero counters.

#### 2.3 Replayed Proposal Execution
- **Threat**: Attacker attempts to replay an already-consumed or aborted proposal ID to duplicate a window mutation.
- **Defense**: Once a proposal is executed or aborted, it is cleared from pending state. Replay attempts find no matching proposal and are rejected.
- **Verification**: Consecutive calls to `SyntheticAttempt` with identical proposal ID fail on the second attempt.

#### 2.4 Target Native Window Destroyed In-Flight
- **Threat**: The target native window closes (user clicks close button or app terminates) while the coordinator lease is active.
- **Defense**:
  * Native authority listens to KWin window closed signal (`windowClosed`).
  * If the target window is destroyed, any pending proposal referencing that window is aborted.
  * Attempt execution returns `Result::Denied(WindowDestroyed)`.
- **Verification**: Calling `SyntheticAttempt` against a destroyed window handle safely aborts without crash, dangling pointer dereference, or memory corruption.

#### 2.5 Recreated Window / UUID Recycling
- **Threat**: Application closes and immediately reopens with the same runtime ID or recycled handle, tricking the coordinator into mutating the new window.
- **Defense**: Window identity is qualified by both runtime handle and generation UUID (`WindowKey { epoch, runtime_id, generation }`). Mismatch between proposal generation and current window generation aborts execution.
- **Verification**: Attempting action after window recreation with new generation fails closed.

#### 2.6 Out-of-Bounds Destination Geometry
- **Threat**: Malicious or buggy coordinator proposes moving a window to negative coordinates, infinite coordinates, or virtual desktops that do not exist.
- **Defense**: Native authority verifies destination coordinates lie within active screen geometry and requested desktop ID exists in KWin desktop list.
- **Verification**: Prepare fails on non-existent desktop or out-of-bounds geometry.

#### 2.7 Expired Proposal / Lease TTL Exceeded
- **Threat**: Coordinator hangs or network delays attempt beyond lease validity period.
- **Defense**: Proposals carry a strict expiration timestamp (`now + proposal_ttl`). Attempt after expiry is rejected as expired.
- **Verification**: Advancing monotonic time past expiration causes `SyntheticAttempt` to reject execution.

#### 2.8 Input Seat Lock Hijack & Lock Transition
- **Threat**: Concurrent user input or another subsystem steals the input seat lock during proposal window.
- **Defense**: Native authority verifies active seat lock ownership before committing any input or layout transition. If the lock was broken or transitioned, operation is denied.
- **Verification**: Breaking the seat lock causes immediate proposal failure.

#### 2.9 Compositor Incarnation Mismatch (Restart)
- **Threat**: KWin compositor crashes and restarts; old coordinator attempts to execute proposals prepared under the previous compositor instance.
- **Defense**: Compositor generates a new 128-bit UUID incarnation upon initialization. Proposals prepared under a previous incarnation fail verification on the new compositor instance.
- **Verification**: Submitting attempt with stale compositor incarnation UUID returns `StaleIncarnation`.

---

### Attack Vector 3: Durable Coordinator Protocol

#### 3.1 Linearized Point-of-No-Return Intent Persistence
- **Threat**: Coordinator transmits wire message before committing intent to SQLite WAL; crash leaves wire action unrecorded on disk.
- **Defense**: Strict ordering:
  1. `DurableOperationJournal::record_intent(op)` committed via `IMMEDIATE` transaction.
  2. WAL flushed to disk with `PRAGMA synchronous = FULL`.
  3. Only upon successful transaction commit is the wire frame written to UDS.
- **Verification**: Intercepting process exit prior to SQLite commit verifies zero frames were written to the transport socket.

#### 3.2 Crash Recovery & Automatic Uncertainty Quarantine
- **Threat**: Coordinator crashes while intent is in flight (after commit, before ACK).
- **Defense**: On startup, coordinator recovery scans for unacknowledged intents. All in-flight windows/resources transition automatically to `Uncertainty::Quarantined`.
- **Verification**: Simulated crash with pending intent leaves resource quarantined upon recovery.

#### 3.3 Lost, Delayed, or Duplicate Acknowledgment
- **Threat**: Compositor ACK is dropped by network, delayed past timeout, or delivered multiple times.
- **Defense**:
  * Lost ACK: Coordinator timeout triggers transition to `Uncertainty::Quarantined`.
  * Delayed ACK (after timeout): Rejected; journal already transitioned to quarantine.
  * Duplicate ACK: First ACK commits `Completed`; second ACK is idempotent or ignored.
- **Verification**: Simulating lost, delayed, and duplicate ACKs verifies correct state transitions and zero double-mutations.

#### 3.4 Conflicting Outcome Resolution
- **Threat**: Compositor reports `Applied`, but coordinator lease manager recorded `Rejected` (e.g., local lease expired), or vice versa.
- **Defense**: Any outcome conflict represents an irreconcilable state desynchronization. Coordinator immediately halts execution on that resource and places it in permanent quarantine.
- **Verification**: Inducing outcome mismatch forces target into `Quarantined`.

#### 3.5 Resource Quarantine Persistence Across Identity Aliases
- **Threat**: Caller attempts to bypass window quarantine by changing window title, wrapping in a new struct, or accessing via secondary ID.
- **Defense**: Quarantine is tracked across both high-level `WindowKey` and native compositor `native_uuid`. Both keys are locked simultaneously; removing or mutating one key does not lift the exclusion on the other.
- **Verification**: Attempting to operate on a quarantined native UUID using an alternate window key is rejected.

---

### Attack Vector 4: Authoritative Reconciliation Gateway

#### 4.1 Forged Provenance Attack
- **Threat**: Malicious client sends a fake reconciliation report claiming to be the authoritative observer to lift quarantine.
- **Defense**: Gateway verifies that the report provenance string strictly equals `depthos.observer.authoritative`. Any other string is rejected.
- **Verification**: Reports with `claimed`, `untrusted`, or empty provenance fail verification.

#### 4.2 Stolen or Invalid Observer Token
- **Threat**: Attacker discovers a stale token and attempts to authorize reconciliation.
- **Defense**: Gateway validates the observer token against the launcher-provisioned secret token for the active session.
- **Verification**: Invalid token returns `UnauthorizedObserver`.

#### 4.3 Stale Sequence Number & Replay
- **Threat**: Attacker replays an older valid observer snapshot to reset window state.
- **Defense**: Gateway enforces strict monotonicity: `report.seq > recorded_quarantine_seq`. Reports with `seq <= recorded_seq` are discarded.
- **Verification**: Replaying snapshot with identical or lower sequence is rejected as `StaleSequence`.

#### 4.4 Observer Disconnect & Fail-Closed Behavior
- **Threat**: Observer process terminates or drops socket connection during reconciliation.
- **Defense**: System fails closed; resources remain in permanent quarantine until an authorized observer reconnects and submits fresh authoritative evidence.
- **Verification**: Observer socket closure leaves all pending resources quarantined.

---

### Attack Vector 5: Host Isolation & Native Safety

#### 5.1 Host Package, Compositor, and Configuration Protection
- **Threat**: Test suites inadvertently modify host system files (`/usr`, `/etc`), alter host KDE configurations (`~/.config/kwinrc`), or kill/disrupt the host desktop compositor.
- **Defense**:
  * Disposable temporary directories for `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_RUNTIME_DIR`.
  * Private Wayland socket namespaces (`wayland-depthos-test-*`).
  * Prohibition of modifying host packages or running commands against the host compositor PID.
- **Verification**: Audit verifies zero modifications to host files, zero host D-Bus mutations, and zero disruption to the host desktop session.

---

## 5. Security Gates & Completion Verdict Taxonomy

The Phase 3.10.1 campaign culminates in an independent validation verdict by Agent 2:

1. **`PHASE 3.10.1 QUALIFIED`**: All quality gates pass offline; real KWin native integration operates with verified host isolation; all 5 attack vectors pass adversarial verification; zero regressions; documentation verified.
2. **`PHASE 3.10.1 PARTIALLY QUALIFIED`**: Core protocol passes, but real KWin integration has non-blocking edge-case constraints that require operational workarounds.
3. **`PHASE 3.10.1 REQUIRES ARCHITECTURE CHANGES`**: Structural defect discovered in real KWin integration or protocol requiring re-architecting.
4. **`PHASE 3.10.1 BLOCKED`**: Critical test failure, host contamination, or security vulnerability that cannot be resolved safely.
