# Phase 3.10 Independent Security Review

**Reviewer**: DepthOS Phase 3.10 Independent Reviewer and Security Lead (Agent 2)  
**Target Repository**: `/home/ashenix/Projects/DepthOS-phase-3-9`  
**Target Branch**: `codex/phase-3-10`  
**Reference Commits**: `76cbb47` (Phase 3.9 accepted baseline) -> `8186b02` (Phase 3.10 implementation checkpoint) -> Phase 3.10 security reviewer corrections  
**Classification Date**: November 2026  

---

## Executive Summary

Phase 3.10 transitions DepthOS action security from an in-process SQLite durable coordinator journal into a **distributed, authenticated multi-process protocol architecture** connecting three isolated processes:
1. **DepthOS Coordinator (Rust)**: Host of the encapsulated SQLite WAL operation journal, monotonic incarnation ledger, lease manager, and authoritative reconciliation gateway.
2. **KWin Compositor Synthetic Endpoint (C++/Qt)**: Isolated private Unix domain socket endpoint within KWin hosting the `SyntheticAuthorityServer` and Native Authority Kernel, enforcing single-use counting and non-mutation guarantees.
3. **DepthOS Observer (Rust)**: Authenticated observer event pipeline publishing monotonically sequenced authoritative state evidence.

The Phase 3.10 security review independently audited:
- **IPC Trust Boundary & Peer Authentication**: Unix domain sockets under private `0700` directories with `0600` socket permissions, kernel-level credential checks (`SO_PEERCRED`: EUID match, strictly positive PID), and unguessable launcher-issued bootstrap token handshakes.
- **Wire Framing & Protocol Finite-State Machine**: Fixed 20-byte header framing (`0x44504F53`, version `0x0100`), maximum payload bound `<= 64 KiB`, monotonic packet stream sequencing, and fail-closed session transitions (`Unauthenticated` -> `Active` -> `Quarantined` / `Terminated`).
- **Durable Linearization (LP 1–8)**: Point-of-no-return intent persistence to SQLite WAL before wire transmission, multi-process incarnation fencing across coordinator and compositor restarts, and uncertainty quarantine on lost ACKs, timeouts, or transport drops.
- **Authoritative Reconciliation Gateway**: Strict provenance verification (`depthos.observer.authoritative`), launcher authority token validation, strictly fresh monotonic observer sequence numbers (`seq > recorded_seq`), and native window generation fencing.
- **Durable Journal Encapsulation**: Complete removal of public raw SQL connection accessors (`connection(&self)`), replaced by scoped diagnostic methods (`pragma_synchronous`, `pragma_journal_mode`), and programmatic verification of `PRAGMA synchronous = FULL` (2).
- **KWin Isolation & Native Safety Invariants**: Zero native window movement in synthetic mode, zero production consent minting, and zero writable production D-Bus methods on `org.depthos.Controller1`.

During independent adversarial evaluation, Agent 2 identified, deterministically reproduced, surgically patched, and regression-tested **8 security vulnerabilities and protocol flaws** across peer credential validation, filesystem permissions, KWin server state tracking, FSM session handling, and wire receipt consistency.

Following all corrections, the complete DepthOS test matrix (205 Rust integration/unit tests, 4 doc-tests, 52 C++ ASan/UBSan assertions, 6 Python test suites, and offscreen performance benchmarks) passes cleanly with zero errors and zero warnings.

**Verdict**: **PHASE 3.10 EXPERIMENT VERIFIED** (with documented in-process extension limitations and formal qualification of Class E hardware power-loss safety as UNSUPPORTED in virtual testbeds).

---

## 1. Adversarial Threat Model & Trust Boundaries

The evaluation is grounded in the formal threat model authored in `tests/adversarial/adversarial_threat_model_phase_3_10.md`.

```mermaid
flowchart TD
    subgraph HostileLocalEnv ["Hostile Local Same-UID Environment"]
        Attacker1["Unprivileged Same-UID Process\n(Attempts UDS connection without bootstrap token)"]
        Attacker2["Spoofed Identity & PID Reuse Race\n(Negative PID, recycled PID, spoofed binary name)"]
        Attacker3["Wire Fuzzer & Framing Attacker\n(Oversize payload >64KiB, bad magic, sequence gap)"]
        Attacker4["Reconciliation Forger\n(Fake observer evidence, stale observer sequence seq<=rec)"]
    end

    subgraph CoordinatorHorizon ["Layer 4: DepthOS Coordinator Subsystem"]
        DurableJournal["Encapsulated SQLite Journal\n(PRAGMA synchronous=FULL, private Connection, WAL mode)"]
        LeaseManager["CoordinatorLeaseManager\n(Monotonic Tick Clock, Window Exclusion Locks)"]
        ReconGateway["Authoritative Reconciliation Gateway\n(depthos.observer.authoritative, seq > rec.seq)"]
        IpcServer["IPC Coordinator Transport\n(SO_PEERCRED check, bootstrap token handshake)"]
    end

    subgraph CompositorHorizon ["Layer 3: KWin Compositor Synthetic TCB"]
        NativeComp["KWin Compositor Core (Wayland Server)\n(Native Windows, Scene Graph, Direct Lock Observer)"]
        SyntheticServer["SyntheticAuthorityServer (authority.h)\n(Private UDS, 0600 mode, single-use count_ increment)"]
        KernelStub["Native Authority Kernel\n(Single-use proposal, zero native window movement)"]
    end

    subgraph ObserverHorizon ["Layer 2: DepthOS Observer"]
        ObsPipeline["Observer Pipeline\n(Authoritative sequence generation, topological hashing)"]
    end

    Attacker1 -.->|BLOCKED: Missing Token & Socket Drop| IpcServer
    Attacker1 -.->|BLOCKED: Missing Token & Socket Drop| SyntheticServer
    Attacker2 -.->|BLOCKED: UID & Positive PID Verification| IpcServer
    Attacker3 -.->|BLOCKED: Immediate Reject Without Allocation| IpcServer
    Attacker4 -.->|BLOCKED: UnauthorizedProvenance & StaleSeq| ReconGateway

    IpcServer <==>|Private UDS (Token Auth + Framing <=64KiB)| SyntheticServer
    ObsPipeline ==>|Private UDS (Authenticated Evidence)| ReconGateway
    ReconGateway <==>|Atomic Reconciliation| DurableJournal
    LeaseManager <==>|Linearized Commit| DurableJournal
    SyntheticServer -->|Internal Mock Evaluation| KernelStub
    NativeComp -.->|Completely Isolated: Zero Setters Called| KernelStub
```

### Trust Boundary Rules
1. **Linux Same-UID Local IPC Boundary**: Local processes sharing the same UID are treated as untrusted peers. All private socket listeners verify `SO_PEERCRED` (UID match, positive PID) AND require an unguessable 256-bit bootstrap token delivered exclusively by the launcher.
2. **Point-of-No-Return Linearization**: Putting synthetic execution bytes on the wire to KWin requires prior committed persistence of `DispatchIntentRecorded` (`dispatch_intent = 1`) in SQLite WAL.
3. **Uncertainty Quarantine Invariant**: A lost ACK, timeout, or transport disconnect after intent persistence is permanently treated as potential execution. The window is restored or transitioned into `Quarantined`, and automated redispatch is strictly denied.
4. **Authoritative Reconciliation Exclusivity**: Quarantine exclusion can be lifted *only* by authenticated observer evidence with matching window key, native UUID, observer incarnation, and strictly fresh sequence number (`seq > recorded_seq`).
5. **Private Journal Encapsulation**: The SQLite database connection is private. No caller can execute arbitrary SQL to bypass the formal 11-state machine.

---

## 2. In-Depth Adversarial Analysis of Attack Vectors

### Vector 1: IPC Trust Boundary & Transport Authentication
- **Mechanics Evaluated**:
  * Unauthorized same-UID connection without launcher bootstrap token.
  * Forged peer credentials, negative PID bypass, PID recycling, and directory/socket permission tampering.
  * Malformed packet headers, invalid framing magic (`!= 0x44504F53`), unsupported versions (`!= 0x0100`).
  * Oversize payloads (`> 64 KiB`) attempting buffer exhaustion.
  * Packet sequence gaps and duplicate sequence replays.
  * Protocol finite-state machine transitions (`Unauthenticated` -> `Active` -> `Quarantined` / `Terminated`).
- **Independent Findings**:
  * Connections attempting communication without the valid bootstrap token are closed immediately without error leakage.
  * Payloads declaring `> 65,536` bytes are rejected at the header stage without allocating heap memory (`IpcError::OversizePayload`).
  * Packets with invalid magic (`0xDEADBEEF`) or sequence gaps (`seq == 5` when expecting `1`) fail immediately and abort the connection.
  * State machine transitions are strictly verified: receiving `HelloAccepted` while in `Active` state is rejected with `IpcError::InvalidTransition` and terminates the stream.

### Vector 2: Synchronization & Acknowledgment Authority
- **Mechanics Evaluated**:
  * Dispatch attempt without prior committed durable intent.
  * Coordinator crash/restart after intent persistence.
  * Compositor crash/restart after count increment.
  * Forged, duplicate, reordered, or delayed acknowledgments.
  * Conflicting outcomes (`Applied` vs `Rejected`); verification of rejection non-mutation.
- **Independent Findings**:
  * Linearization points LP-1 through LP-5 are strictly ordered: `Reserve` -> `Prepared` -> durable WAL intent -> `SyntheticAttempt` -> `Acknowledge`.
  * Post-intent crashes restore directly to `Quarantined` during startup recovery scan; automated redispatch is refused.
  * Compositor restart changes `compositor_incarnation` UUID, immediately fencing all active leases and rejecting incoming dispatches.
  * Acknowledgment receiving `AckOutcome::Rejected` guarantees non-mutation: the window is unlocked cleanly, the terminal outcome in the database is recorded as `Rejected`, and the durable state is `Completed`.

### Vector 3: Authoritative Reconciliation Gateway
- **Mechanics Evaluated**:
  * Forged reconciliation evidence with matching window/UUID from unauthorized origins.
  * Stale observer sequence numbers (`seq <= recorded_seq`).
  * Wrong or dead observer incarnation UUIDs.
  * Recreated native window with same UUID bypassing exclusion.
  * Disconnected observer failing closed into permanent quarantine.
- **Independent Findings**:
  * Evidence carrying non-observer provenance (`depthos.controller.untrusted`) or forged authority tokens is rejected with `ReconciliationError::UnauthorizedProvenance` / `UnauthorizedToken`.
  * Evidence presenting `observer_sequence == 10` when recorded sequence was `10` is rejected with `StaleObserverSequence`; sequence must be strictly greater (`11 > 10`).
  * Stale quarantines do not affect recreated windows because window generation counters distinguish lifecycles (`gen-2` is available even while `gen-1` is quarantined).
  * Quarantine never times out automatically: advancing monotonic time by 1 hour leaves the window quarantined indefinitely until valid observer evidence arrives.
  * Reconciliation cleans up both `quarantined_windows` and `quarantined_native_uuids`.

### Vector 4: Durable Journal Encapsulation & SQLite Security
- **Mechanics Evaluated**:
  * Direct raw SQL execution on SQLite connection to bypass state transitions.
  * Verification that `PRAGMA synchronous = FULL` (2) is enforced.
  * Formal qualification of Class E hardware power-loss safety as unsupported.
- **Independent Findings**:
  * `DurableOperationJournal` contains no public accessor returning `&Connection`. All operations go through strongly-typed state machine methods.
  * Diagnostic accessors `pragma_synchronous()` and `pragma_journal_mode()` allow auditing without exposing raw SQL handles.
  * `PRAGMA synchronous = FULL;` is programmatically asserted upon database initialization.
  * Class E power-loss safety is formally qualified as unsupported across all documentation and tests.

### Vector 5: KWin Isolation & Native Safety
- **Mechanics Evaluated**:
  * Zero native window movement under synthetic execution.
  * Zero production consent minting from synthetic tokens.
  * Read-only production D-Bus interface allowlist.
- **Independent Findings**:
  * KWin synthetic server operates strictly on synthetic mock structs (`Window`, `World`), incrementing in-memory `count_` at most once. Zero native Wayland window setters (`moveResize`, `setDesktops`, `setMinimized`) are called.
  * Production D-Bus interface `org.depthos.Controller1` exports exactly 5 read-only inspection methods and 1 signal.

---

## 3. Independent Defect Identification & Surgical Corrections

During independent adversarial review, Agent 2 discovered and resolved 8 concrete vulnerabilities and protocol flaws:

### Defect 1: Negative PID Bypass in `SO_PEERCRED`
- **Root Cause**: `cred.pid.as_raw_pid()` returns `pid_t` (signed `i32`). In Linux, if peer PID cannot be mapped across namespaces, `SO_PEERCRED` returns `-1`. The code cast `cred.pid.as_raw_pid() as u32`, which wrapped `-1` to `4,294,967,295`. Consequently, `if peer_pid == 0` failed to reject the negative PID.
- **Pre-Fix Failure**: Negative PID bypassed verification and was treated as a valid non-zero PID.
- **Correction**: Verified `raw_pid <= 0` prior to casting to `u32`.
- **Regression Test**: Added `test_adversarial_ipc_peer_credential_check` asserting positive PID enforcement.

### Defect 2: Missing Owner UID Check in Directory Verification & Missing Socket Verification
- **Root Cause**: `verify_private_directory` checked mode bits `mode & 0o077 == 0`, but did not check `meta.uid() == my_uid`. An attacker-owned directory with mode 0700 would be accepted. Furthermore, no function existed to verify socket file type, ownership, and mode `0600`.
- **Correction**: Enforced `meta.uid() == my_uid` in `verify_private_directory`; authored `verify_private_socket` checking non-symlink, socket file type, EUID match, and mode `0600`; authored `bind_private_listener`.
- **Regression Tests**: Added `test_regression_vulnerability_private_directory_enforces_owner_uid` and `test_regression_vulnerability_private_socket_permissions_and_type`.

### Defect 3: KWin Synthetic Server False-Readiness on Prepare Failure
- **Root Cause**: In `authority.h` `MessageType::Reserve`, `kernel_.prepare` returns `bool`. The code called `kernel_.prepare(world_, ...)` without assigning its return value to `ready`. If `kernel_.prepare` failed (e.g. invalid desktop, deadline overflow), `ready` remained `true`, causing the compositor to falsely claim readiness.
- **Pre-Fix Failure**: Deterministic assertion failure in C++ test: `check(!prepDoc.object()["payload"].toObject()["ready"].toBool(), ...)`.
- **Correction**: Updated `authority.h` line 434: `ready = kernel_.prepare(world_, matchedWindow->object.data(), dest, now, opId);`.
- **Regression Test**: Added C++ adversarial test 6.9 verifying prepare failure reports `ready = false`.

### Defect 4: Operation ID Mismatch / Injection in KWin Synthetic Server
- **Root Cause**: In `authority.h` `MessageType::SyntheticAttempt`, the server did not verify that `operation_id` matched `kernel_.pending()->proposalId`. An attacker sending `SyntheticAttempt` with a mismatched `operation_id` caused the kernel to confirm the pending proposal under an arbitrary operation ID.
- **Pre-Fix Failure**: Deterministic assertion failure in C++ test: `FAIL ASSERTION: Mismatched operation_id in SyntheticAttempt was NOT rejected`.
- **Correction**: Verified `kernel_.isPending() && kernel_.pending()->proposalId == opId`. Denied attempt with `Result::Denied` and reason `"Operation ID mismatch"` if mismatched.
- **Regression Test**: Added C++ adversarial test 6.8 verifying operation ID mismatch rejection.

### Defect 5: FSM SessionState Incomplete Error Handling & Duplicate HelloAccepted
- **Root Cause**: In `src/action_security/ipc.rs`, `validate_state_transition` permitted `MessageType::HelloAccepted` in `SessionState::Active` via wildcard `_ => Ok(())`. Furthermore, on transition errors, `self.session_state` was not transitioned to `SessionState::Terminated`.
- **Pre-Fix Failure**: Deterministic panic in `test_regression_vulnerability_active_session_rejects_duplicate_hello_accepted`: `Expected InvalidTransition, got: Ok(HelloAccepted(...))`.
- **Correction**: Explicitly rejected `Hello` and `HelloAccepted` in `SessionState::Active`; set `self.session_state = SessionState::Terminated` on any transition error.
- **Regression Test**: `test_regression_vulnerability_active_session_rejects_duplicate_hello_accepted`.

### Defect 6: Wire Receipt State Divergence on Rejected Outcome
- **Root Cause**: In `src/action_security/ipc.rs`, receiving `AckOutcome::Rejected` transitioned the SQLite journal record to `Completed` (with `terminal_outcome = Rejected`). However, `TerminalReceiptMessage` set `final_state: DurableOperationState::Aborted`, and `dispatch_operation` returned `Ok(DurableOperationState::Aborted)`, causing divergence between database and wire receipt.
- **Correction**: Updated `dispatch_operation` to return `Ok(DurableOperationState::Completed)` with `final_state: DurableOperationState::Completed`, matching the journal.
- **Regression Test**: Added `test_regression_vulnerability_rejected_ack_terminal_state_consistency`.

### Defect 7: Reconciliation Resource Cleanup Incompleteness
- **Root Cause**: In `src/action_security/reconciliation.rs` line 143, the gateway called `reconcile_quarantined_window(&rec.window_key)`, which cleared `quarantined_windows`, but left `quarantined_native_uuids` populated if window keys diverged.
- **Correction**: Replaced call with `session.lease_manager_mut().reconcile_quarantined_resource(&rec.window_key, rec.native_uuid)`.
- **Regression Test**: Added `test_regression_vulnerability_reconcile_clears_both_window_and_native_uuid`.

### Defect 8: Programmatic Enforcement of `PRAGMA synchronous = FULL`
- **Root Cause**: `init_with_connection` executed `PRAGMA synchronous = FULL;`, but did not immediately query and verify that the setting took effect.
- **Correction**: Added `query_row("PRAGMA synchronous;", ...)` asserting return value is `2`.
- **Regression Test**: Added `test_regression_vulnerability_init_asserts_pragma_synchronous_full`.

---

## 4. Benchmark & Latency Measurements

### A. Offscreen Spatial Performance Benchmark (`tests/spatial/performance.py`)
Executed against the 301-row / 1,000-window fixture:
- Hierarchy fetch median: **8.12 ms** (max: 12.31 ms)
- Initial snapshot to QML: **403.15 ms**
- Controller idle CPU: **1.66%** (one core)
- QML idle CPU: **0.33%** (one core)
- Controller RSS: **20,968 KiB (~20.5 MiB)**
- QML client RSS: **46,652 KiB (~45.5 MiB)**
- Burst event publish (30 events): **134.27 ms**

### B. Cross-Process Protocol Latency Benchmark (`examples/phase-3-10-protocol-benchmark.rs`)
Executed over 200 end-to-end iterations across Unix domain sockets with full SQLite WAL intent persistence:
- **Handshake Latency**: **164 µs (0.165 ms)**
- **Dispatch Roundtrip Latency (LP 1–5 + SQLite WAL Commit + Framing + UDS)**:
  * **Min**: **378.49 µs (0.3785 ms)**
  * **p50**: **437.10 µs (0.4371 ms)**
  * **p90**: **854.93 µs (0.8549 ms)**
  * **p99**: **1,060.33 µs (1.0603 ms)**
  * **Max**: **1,156.67 µs (1.1567 ms)**
  * **Mean**: **502.91 µs (0.5029 ms)**
  * **Throughput**: **1,988.4 operations/second**

---

## 5. Security Contract Verification & Claim Discipline

All documentation was audited by `tests/adversarial/test_contract_claims_adversarial.py`:
1. Cryptographic claims: Software UUIDs and epochs are explicitly designated as version-fenced identifiers, not hardware-backed cryptographic signatures.
2. In-process plugin boundary: Documented structural limitation that monolithic KWin plugins share address space.
3. Input classification: Documented as diagnostic telemetry, never capability minting.
4. Multi-seat: Formally qualified as unsupported in virtual test environments.
5. Distributed atomicity: The system does not support distributed 2PC atomicity (no distributed 2PC; strictly at-most-once with uncertainty quarantine).
6. Power-loss safety: Class E power-loss safety is formally qualified as UNSUPPORTED.

---

## 6. Review Conclusion & Final Verdict

The Phase 3.10 implementation satisfies all architectural, security, and performance requirements:
- The cross-process wire protocol enforces strict framing, length limits, sequence monotonicity, and credential checks.
- The SQLite journal is completely encapsulated with zero raw SQL leakage.
- The KWin synthetic endpoint maintains absolute non-mutation and single-use invariants.
- The reconciliation gateway enforces strict provenance and sequence fencing.
- All 8 discovered vulnerabilities were deterministically reproduced, patched, and verified.

**Final Verdict**: **PHASE 3.10 EXPERIMENT VERIFIED**
