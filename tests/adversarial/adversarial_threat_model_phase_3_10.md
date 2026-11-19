# Phase 3.10 Adversarial Threat Model and Security Architecture

**Author**: DepthOS Independent Security Reviewer & Correction Engineer (Agent 2)  
**Target Repository**: `/home/ashenix/Projects/DepthOS-phase-3-9`  
**Target Branch**: `codex/phase-3-10`  
**Classification Date**: November 2026  
**Scope**: Cross-Process Protocol, Peer Authentication, Private KWin Synthetic Endpoint, Journal Encapsulation, and Authoritative Reconciliation

---

## 1. Executive Summary & Purpose

Phase 3.10 establishes the **cross-process wire protocol, transport peer authentication, private KWin synthetic endpoint isolation, strict journal encapsulation, and authoritative reconciliation gateway** for DepthOS.

In prior phases (3.8 and 3.9), the coordinator lease management and durable SQLite operation journal were established as robust in-process and crash-consistent components. Phase 3.10 elevates this architecture to a distributed, multi-process topology where the **DepthOS Coordinator**, **KWin Compositor Synthetic Endpoint**, and **DepthOS Observer** communicate over dedicated local IPC channels across distinct process boundaries.

This document formalizes the adversarial threat model, attacker profiles, trust boundaries, failure classifications, and concrete attack vectors governing Phase 3.10. It establishes the mandatory invariants and independent verification criteria for:
1. **IPC Trust Boundary & Transport Authentication**: Mutual verification of peer credentials (PID, UID, GID, `/proc/$PID/exe`), launcher-provided ephemeral bootstrap tokens, strict frame bounds (<=64 KiB), and protocol state machine fencing.
2. **Synchronization & Acknowledgment Authority**: Linearized point-of-no-return intent persistence prior to wire transmission, multi-process incarnation fencing across coordinator and compositor restarts, duplicate/delayed/forged acknowledgment rejection, and non-mutation guarantees for rejected proposals.
3. **Authoritative Reconciliation Gateway**: Anti-spoofing reconciliation evidence verification, strict monotonic observer sequence enforcement (`seq > recorded_seq`), observer incarnation fencing, window generation/lifecycle verification against UUID reuse, and fail-closed permanent quarantine under observer disconnect.
4. **Durable Journal Encapsulation & SQLite Security**: Absolute private encapsulation of SQLite connections against raw SQL bypasses, programmatic verification of `PRAGMA synchronous = FULL` (2), and rigorous qualification of Class E hardware power-loss safety as formally unsupported.
5. **KWin Isolation & Native Safety**: Zero native window movement in production mode, absolute isolation of synthetic test tokens, and zero writable production D-Bus methods.

---

## 2. Distributed Architecture & Trust Boundaries

The Phase 3.10 system topology spans four distinct processes communicating over private Unix Domain Sockets (UDS):

```mermaid
flowchart TD
    subgraph Launcher ["Process 0: DepthOS Launcher / Supervisor"]
        InitTokens["Bootstrap Token Generator\n(High-entropy CSPRNG tokens passed via private FDs/env)"]
        ProcMonitor["Process Lifecycle & Incarnation Supervisor"]
    end

    subgraph CoordinatorProc ["Process 1: DepthOS Coordinator (Rust)"]
        DurableJournal["Encapsulated SQLite Journal\n(PRAGMA synchronous=FULL, Immediate Tx, No Raw SQL)"]
        LeaseManager["CoordinatorLeaseManager\n(Monotonic Tick Clock, Window Exclusion Locks)"]
        ReconGateway["Authoritative Reconciliation Gateway\n(Observer Seq Validation, Incarnation Matcher)"]
        CoordServer["IPC Coordinator Endpoint\n(Peer Credential Check, Token Handshake)"]
    end

    subgraph CompositorProc ["Process 2: KWin Compositor / Native Authority (C++/Qt)"]
        KWinCore["KWin Compositor TCB (Wayland Server)\n(Real Native Windows, Scene Graph, ScreenLocker)"]
        PrivateEndpoint["Private Synthetic IPC Endpoint\n(Local UDS, Bootstrap Token Auth, Frame Parser <=64KiB)"]
        AuthorityKernel["Native Authority Kernel\n(Synthetic Token Sink, Zero Native Movement)"]
    end

    subgraph ObserverProc ["Process 3: DepthOS Observer (Rust)"]
        ObsEngine["Observer Engine\n(State Change Tracker, Monotonic Sequence Counter)"]
        ObsEndpoint["Observer Private IPC Client\n(Authenticated Stream, Event Publisher)"]
    end

    subgraph HostileEnv ["Hostile Same-UID Environment"]
        RogueProc["Unprivileged Local Rogue Process\n(Attempts UDS connection, forged tokens, frame fuzzing)"]
    end

    Launcher -->|Launches with Token A| CoordinatorProc
    Launcher -->|Launches with Token B| CompositorProc
    Launcher -->|Launches with Token C| ObserverProc

    CoordinatorProc <==>|Private UDS (Token Auth + Framing <=64KiB)| CompositorProc
    ObserverProc ==>|Private UDS (Authenticated Event Stream)| CoordinatorProc

    RogueProc -.->|BLOCKED: Missing Token & Bad Executable| CoordServer
    RogueProc -.->|BLOCKED: Missing Token & Bad Executable| PrivateEndpoint
```

### Trust Boundary Invariants

1. **Local IPC Trust Boundary**: Same-UID execution on Linux is **not** an implicit trust domain. Unprivileged processes running under the same UID (e.g., other desktop applications, browser sandboxes, compromised tools) have access to the filesystem and user sockets unless explicitly fenced. DepthOS endpoints **MUST** authenticate peers via OS credentials (`SO_PEERCRED`) AND verify an ephemeral, high-entropy launcher bootstrap token over the initial handshake.
2. **Process Boundary Isolation**: The Coordinator, Compositor, and Observer run in separate virtual address spaces. Memory corruption or panics in one process must not directly corrupt another process's memory or database.
3. **Wire Point-of-No-Return**: No synthetic mutation bytes may be transmitted across the IPC socket to KWin until the operation has been durably committed to SQLite storage in state `DispatchIntentRecorded` (`dispatch_intent = 1`).
4. **Compositor Incarnation Fencing**: The KWin compositor generates a unique ephemeral `compositor_incarnation` UUID at startup. If KWin restarts or disconnects, all active leases tied to the previous incarnation are immediately fenced and rendered uncompletable.
5. **Authoritative Reconciliation Exclusivity**: The reconciliation gateway in the Coordinator accepts evidence *only* from an authenticated Observer session matching the registered observer incarnation, with an event sequence number strictly higher than the sequence observed when quarantine was initiated.
6. **Journal Encapsulation**: The SQLite database connection is strictly encapsulated within the `DurableOperationJournal` struct. No raw SQL query interface is exposed to callers, and all state modifications must pass through state-machine-validated methods.

---

## 3. Failure & Crash Taxonomies (Classes A–E)

Consistent with Phase 3.9 security contracts, failures are classified according to five formal categories:

| Failure Class | Physical / Logical Mechanism | System Impact | Recovery Expectation | Verification Status |
|---|---|---|---|:---:|
| **Class A: Controlled Disconnection & Interruption** | Explicit socket shutdown (`SHUT_RDWR`), clean peer termination. | In-flight buffers drained or discarded. Database consistent. | Clean lease rollback or safe reconnection. | **VERIFIED** |
| **Class B: Process Crash / SIGKILL** | `kill -9`, panic with abort, sudden process exit. | In-flight socket buffers lost. Kernel closes file descriptors. In-flight SQLite transactions rolled back by WAL recovery. | Pre-intent operations aborted; post-intent operations quarantined. Automated redispatch denied. | **VERIFIED** |
| **Class C: Concurrent IPC & Storage Contention** | Multiple concurrent requests, thread races, SQLite lock contention (`SQLITE_BUSY`). | Serialized via OS socket queues and SQLite `TransactionBehavior::Immediate`. | Single winner for window locks; bounded wait up to `busy_timeout` (5000ms). | **VERIFIED** |
| **Class D: Transport Framing & Storage Corruption** | Truncated packets, garbage frames, bit flips in database or WAL. | Frame parser detects invalid magic, bad length, or malformed payload. SQLite returns `SQLITE_CORRUPT`. | Immediate connection tear-down; database operations fail closed; quarantine active window. | **VERIFIED** |
| **Class E: Operating System Crash & Power Loss** | Sudden kernel panic, physical power cut, NVMe/SSD controller flush failure. | OS page cache dropped; un-flushed dirty hardware blocks lost. Relies on non-volatile drive caches and `fsync`. | SQLite WAL/FULL sync maintains integrity *if and only if* underlying disk hardware honors barrier flush. | **FORMALLY UNSUPPORTED** (Virtual testbeds lack bare-metal PLP verification) |

> [!CAUTION]
> **Class E Hardware Power-Loss Formal Qualification**: Because software virtualization layers and standard consumer storage drives cannot deterministically simulate or guarantee non-volatile write cache persistence under sudden loss of power, **Class E Power-Loss Safety is formally qualified as UNSUPPORTED**. All architectural guarantees in Phase 3.10 cover Classes A through D.

---

## 4. Adversarial Attack Vector Catalog

### Attack Vector 1: IPC Trust Boundary & Transport Authentication

#### 1.1 Unauthorized Same-UID Connection Attempt
- **Threat**: A rogue or compromised process running under the same user UID attempts to open a connection to the Coordinator or KWin private socket.
- **Attack Scenario**: Attacker executes `nc -U /run/user/1000/depthos/kwin-synthetic.sock` or writes a custom binary to send synthetic mutation requests directly to the compositor endpoint.
- **Defensive Invariant**:
  1. The listening socket is created in a private directory with permissions `0700` (mode verified on bind).
  2. Upon connection `accept()`, the listener extracts socket peer credentials via `SO_PEERCRED` (Linux):
     * Peer UID must match current process UID (`geteuid()`).
     * Peer PID is checked against the launcher process table.
  3. The listener requires the very first packet on the connection to be an `AuthHandshake` containing the ephemeral 256-bit bootstrap token issued by the launcher for this specific channel.
  4. Any packet sent before successful authentication, or any failed authentication attempt, results in immediate socket close without error responses that could leak state.

#### 1.2 Forged Peer Identity, PID Reuse & Spoofed Executable Name
- **Threat**: An attacker attempts to exploit PID recycling or spoofed `/proc/$PID/exe` symlinks to pose as an authenticated coordinator or observer.
- **Attack Scenario**: The genuine Coordinator terminates. A malicious process rapidly forks until it claims the recycled PID, then connects to the KWin socket. Alternatively, an attacker names their malicious binary `depthos-coordinator` or executes it from a directory with that name.
- **Defensive Invariant**:
  1. PID verification alone is considered insufficient. Authentication fundamentally relies on the high-entropy (256-bit) ephemeral token delivered exclusively through private file descriptors or inheritance by the launcher.
  2. Peer executable path is read from `/proc/$PID/exe` via `readlink` and canonicalized:
     * Must resolve to the exact expected canonical binary path on disk.
     * Creation time/start time (`starttime` from `/proc/$PID/stat`) must be >= launcher spawn timestamp.
  3. Authentication is single-use per connection: tokens are invalidated or connection sessions are bound to the specific verified socket FD.

#### 1.3 Malformed Packet Headers, Oversize Payloads (>64 KiB), and Frame Fuzzing
- **Threat**: Attacker sends corrupted framing, invalid packet types, or massive payloads to cause memory exhaustion, buffer overflow, or unhandled panics.
- **Attack Scenario**:
  * Packet with declared length `0xFFFFFFFF` (4 GiB) causing memory allocation failure.
  * Packet with declared payload length of 100 bytes, but socket closes after 20 bytes (truncation / stall).
  * Packet with declared length > 64 KiB (`65536` bytes).
  * Packet with invalid magic bytes or unsupported protocol version.
- **Defensive Invariant**:
  1. Strict framing layout: `[Magic: 4 bytes] [Version: 2 bytes] [MsgType: 2 bytes] [Sequence: 8 bytes] [PayloadLen: 4 bytes] [Payload: N bytes]`.
  2. Magic bytes must match `0x44504F53` ("DPOS"). Mismatched magic triggers immediate disconnect.
  3. Protocol version must match supported version (Phase 3.10: `0x0100`).
  4. Payload length `N` has a hard limit: `N <= 65,536` (64 KiB). Any packet with `PayloadLen > 65536` is rejected immediately without allocating payload memory, and the connection is terminated.
  5. Read timeouts are enforced: a sender that stalls mid-frame is disconnected after a bounded deadline (e.g., 2000ms).

#### 1.4 Replayed, Duplicate, or Delayed Protocol Packets
- **Threat**: An attacker snoops or buffers valid packets and replays them at a later time to trigger duplicate actions or state corruption.
- **Attack Scenario**: Attacker captures a valid `SyntheticProposal` packet and replays it 10 seconds later, or injects duplicate packets into an active stream.
- **Defensive Invariant**:
  1. Monotonic stream packet sequence numbers: each packet sent on a connection must carry an incrementing sequence number `seq_num = last_seq_num + 1`. Out-of-order or duplicate sequences trigger connection termination.
  2. Operation-level replay immunity: every proposal carries an immutable `operation_id` (UUIDv4). Even across reconnections, the Coordinator's SQLite journal rejects duplicate `operation_id` values via primary key uniqueness constraints (`DurableError::Replay`).
  3. Expiration deadlines: every proposal carries a monotonic deadline `expiry_tick`. Packets arriving at the compositor after the deadline are rejected immediately with `Outcome::Rejected("Expired")`.

---

### Attack Vector 2: Synchronization & Acknowledgment Authority

#### 2.1 Dispatch Attempt Without Prior Committed Durable Intent
- **Threat**: The coordinator transmits a mutation request across the IPC socket before the intent has been successfully committed to the durable SQLite journal.
- **Attack Scenario**: If the coordinator process crashes immediately after socket write but before database commit, the compositor executes the mutation, but the coordinator on restart has no record of the dispatch, violating crash-consistency.
- **Defensive Invariant**:
  1. **Strict Linearization Sequence**:
     * Step 1: Write `DispatchIntentRecorded` (`dispatch_intent = 1`, state = `DispatchIntentRecorded`) to SQLite.
     * Step 2: Commit SQLite transaction (`FULL` synchronous flush).
     * Step 3: Only *after* transaction commit returns success, invoke IPC transport dispatch.
  2. Code-level assertion: the IPC client dispatch function takes a witness token or proof-of-intent type that can *only* be constructed by `persist_dispatch_intent()` returning a committed transaction.
  3. Attempting to call dispatch with an uncommitted or missing intent returns an unrecoverable error and halts dispatch.

#### 2.2 Coordinator Restart After Intent; Compositor Restart After Count
- **Threat**: A crash occurs during the asynchronous window between intent persistence and acknowledgment receipt.
- **Attack Scenario 1 (Coordinator Crash)**: Coordinator commits `DispatchIntentRecorded`, sends packet, then gets killed with `SIGKILL`. On reboot, what happens to the window?
- **Attack Scenario 2 (Compositor Crash)**: Compositor receives proposal, increments internal counter or alters window, then gets killed before sending ACK.
- **Defensive Invariant**:
  1. **Post-Intent Uncertainty Invariant**: Upon coordinator restart, the recovery scan identifies any operation in `DispatchIntentRecorded` or `SyntheticDispatchAttempted`. Because the coordinator cannot know whether the compositor received or acted on the request, it **MUST NOT** assume failure and **MUST NOT** retry. The window is restored directly into `Quarantined`.
  2. **Compositor Incarnation Invalidation**: When KWin restarts, it generates a fresh `compositor_incarnation` UUID. When the coordinator reconnects, it detects the new compositor incarnation. All pending operations under the old compositor incarnation are immediately invalidated and transitioned to `Quarantined`.
  3. **Counter/State Fencing**: Compositor tracks its own monotonic dispatch counter. Any incoming proposal whose compositor incarnation does not match the current running instance is rejected with `CompositorMismatch`.

#### 2.3 Forged, Duplicate, Reordered, or Delayed Acknowledgments
- **Threat**: Compositor responses are fabricated, duplicated, arrived out-of-order, or delayed beyond the operational lease timeout.
- **Attack Scenario**:
  * An unauthenticated peer sends a fake `LeaseAck` packet.
  * A network/scheduler hiccup delays an ACK until after the coordinator has already timed out and quarantined the window.
  * Two ACK packets arrive for the same `operation_id`.
- **Defensive Invariant**:
  1. ACKs are accepted *only* over the authenticated compositor IPC connection.
  2. Every ACK carries `(operation_id, lease_id, coordinator_incarnation, compositor_incarnation, outcome)`.
  3. If an ACK arrives for an operation that is already in a terminal state (`Completed`, `Aborted`, `Quarantined`, `Reconciled`), the state machine rejects the transition with `TerminalState(state)`. Terminal states are strictly absorbing.
  4. If an ACK arrives after timeout (when the operation has been quarantined), the ACK **cannot** lift the quarantine. The window remains quarantined until authoritative reconciliation occurs.

#### 2.4 Conflicting Outcomes & Rejection Non-Mutation Guarantee
- **Threat**: The compositor returns `Rejected`, but the native window state was actually partially modified, or an ACK reports conflicting statuses (`Applied` and `Rejected`) across multiple messages.
- **Attack Scenario**: Compositor encounters an error halfway through a multi-step operation (e.g. desktop switch succeeded, but coordinate change failed) and returns `Rejected`.
- **Defensive Invariant**:
  1. The compositor Native Authority Kernel guarantees that any `Rejected` outcome is an absolute **non-mutation guarantee**: no window coordinates, desktop associations, or scene graph nodes were altered.
  2. If the compositor encounters an unrecoverable internal error where non-mutation cannot be guaranteed, it must return `Outcome::IndeterminateError`.
  3. The Coordinator treats `Outcome::IndeterminateError` as an immediate quarantine trigger.
  4. Rejection responses are terminal: once an operation is marked `Aborted` or `Quarantined`, no subsequent message can convert it to `Completed`.

---

### Attack Vector 3: Authoritative Reconciliation Gateway

#### 3.1 Forged Reconciliation Evidence
- **Threat**: An unauthorized component or local process attempts to lift a window quarantine by presenting forged reconciliation evidence.
- **Attack Scenario**: An attacker knows a window is quarantined under UUID `W-1234`. The attacker connects to the coordinator and submits a `ReconciliationRequest` asserting that the window is safe and in sync.
- **Defensive Invariant**:
  1. The Reconciliation Gateway does **not** accept reconciliation requests from arbitrary IPC clients or public D-Bus interfaces.
  2. Reconciliation evidence is accepted *only* from the authenticated DepthOS Observer channel.
  3. Evidence must include:
     * Verified `window_id` and matching `native_uuid`.
     * `observer_incarnation` UUID matching the current observer instance.
     * Monotonic observer sequence number `observer_seq`.
     * Observer topological fingerprint (hash of verified window geometry, desktop, and activity state).
  4. Unauthenticated or public reconciliation attempts fail closed with `ReconciliationRejected("UnauthorizedOrigin")`.

#### 3.2 Stale Observer Sequence Numbers (`seq <= recorded_seq`)
- **Threat**: Replay of old observer events to reconcile a quarantined window using pre-quarantine state.
- **Attack Scenario**: An observer event that was generated *before* the failed mutation is replayed or processed after quarantine.
- **Defensive Invariant**:
  1. When a window is quarantined, the Coordinator records the last known observer sequence number for that window: `quarantine_observer_seq`.
  2. Any reconciliation evidence presenting `observer_seq <= quarantine_observer_seq` is **strictly rejected** as stale (`ReconciliationRejected("StaleObserverSequence")`).
  3. Only observer events generated *strictly after* the quarantine point (`observer_seq > quarantine_observer_seq`) can serve as valid reconciliation evidence.

#### 3.3 Recreated Native Window with Same UUID Bypassing Exclusion
- **Threat**: A window that was quarantined is closed and destroyed by the user. A new, unrelated window is created which coincidentally or maliciously reuses the same window ID or UUID, attempting to inherit or bypass the quarantine.
- **Attack Scenario**: Malicious client closes window `W-1` and immediately opens a new Wayland surface claiming UUID `W-1`, attempting to acquire leases while the old quarantine is unresolved.
- **Defensive Invariant**:
  1. Windows are tracked with both logical `window_id` AND immutable native `generation` counter / creation epoch.
  2. When a window is destroyed, the Observer emits a `WindowDestroyed(window_id, generation)` event.
  3. The Coordinator records window destruction, permanently terminating any active lease or quarantine associated with that generation.
  4. A newly created window with the same identifier receives an incremented `generation` (e.g. `generation = generation + 1`). Leases for the new generation cannot inherit or be blocked by stale quarantines of prior generations.

#### 3.4 Disconnected Observer or Unknown Outcome Failing Closed
- **Threat**: The Observer crashes, disconnects, or loses synchronization with the Wayland compositor.
- **Attack Scenario**: While a window is quarantined, the Observer process dies. Without an active observer, the system might be tempted to "time out" the quarantine and release the lock to prevent deadlock.
- **Defensive Invariant**:
  1. **Strict Fail-Closed Principle**: Quarantine **NEVER** times out automatically.
  2. In the absence of authoritative, verified observer evidence, the window remains in `Quarantined` indefinitely.
  3. Automated actions on quarantined windows are strictly prohibited.
  4. The only path out of quarantine without an active observer is explicit manual operator intervention via an authenticated administrative tool or clean application reset.

---

### Attack Vector 4: Durable Journal Encapsulation & SQLite Security

#### 4.1 Direct Raw SQL Execution Bypass
- **Threat**: Internal or external code obtains a reference to the raw SQLite connection or executes arbitrary SQL queries, bypassing state machine transitions and audit logging.
- **Attack Scenario**: A developer or buggy extension executes `conn.execute("UPDATE operations SET state = 'Completed' WHERE id = ?")`, bypassing state machine validation (`can_transition_to`), lease expiration checks, and incarnation fences.
- **Defensive Invariant**:
  1. The `rusqlite::Connection` or connection pool is declared as a `private` field within `DurableOperationJournal`.
  2. No getter or dereference method returns the inner connection.
  3. All database operations are exposed exclusively through domain-specific, strongly-typed methods:
     * `record_intent(...) -> Result<IntentReceipt, DurableError>`
     * `record_acknowledgment(...) -> Result<(), DurableError>`
     * `record_quarantine(...) -> Result<(), DurableError>`
     * `record_reconciliation(...) -> Result<(), DurableError>`
  4. Every mutating method internally validates the transition against `DurableOperationState::can_transition_to` inside an immediate transaction.

#### 4.2 Enforcement of `PRAGMA synchronous = FULL` (2)
- **Threat**: SQLite defaults to `synchronous = NORMAL` in WAL mode, which allows dirty database frames to remain in OS buffer cache, risking rollback or corruption if the OS crashes.
- **Attack Scenario**: SQLite configuration is silently altered or left at default, weakening durability guarantees under process termination or system reset.
- **Defensive Invariant**:
  1. Every database connection opened by DepthOS explicitly executes:
     * `PRAGMA journal_mode = WAL;`
     * `PRAGMA synchronous = FULL;`
     * `PRAGMA busy_timeout = 5000;`
     * `PRAGMA foreign_keys = ON;`
  2. Startup verification: the journal constructor immediately queries `PRAGMA synchronous;` and asserts that the returned value is exactly `2` (`FULL`). If not `2`, initialization panics/fails closed.

#### 4.3 Raw File Permissions and Symlink Hijacking
- **Threat**: The SQLite database file or directory is manipulated by another same-UID process via symlink attacks or permissive file modes.
- **Attack Scenario**: Attacker places a symlink at `/path/to/depthos-journal.db` pointing to `/etc/shadow` or a sensitive file, or changes file mode to `0666`.
- **Defensive Invariant**:
  1. The database parent directory must be owned by the current EUID and have permissions `0700`.
  2. The database file is opened using `O_NOFOLLOW` / verified via `lstat` to guarantee it is a regular file and not a symlink.
  3. File permissions on database and WAL files are strictly enforced as `0600`.

---

### Attack Vector 5: KWin Isolation & Native Safety

#### 5.1 Verification of Zero Native Window Movement in Production
- **Threat**: Despite synthetic action isolation, synthetic proposals or test endpoints inadvertently trigger real physical window movements or desktop changes in production KWin.
- **Attack Scenario**: A synthetic proposal sent to KWin's private endpoint calls `window->move()` or `workspace()->sendClientToDesktop()` on a production Wayland window.
- **Defensive Invariant**:
  1. The KWin Synthetic Endpoint connects strictly to the **Native Authority Kernel**, which operates in synthetic shadow evaluation mode.
  2. All calls to native KWin geometry mutators (`moveResize`, `setDesktops`, `setMinimized`) are completely disconnected or compiled out in synthetic mode.
  3. Static and dynamic verification: execution of synthetic test suites must demonstrate that native KWin window lists, geometry bounds, and scene graph nodes undergo zero mutations.

#### 5.2 Zero Production Consent Minting
- **Threat**: Synthetic test tokens or automated lease approvals leak into the production user consent framework, auto-authorizing actions without user interaction.
- **Attack Scenario**: Test fixtures set an internal flag or token that remains valid when the system transitions to production, allowing unprompted action execution.
- **Defensive Invariant**:
  1. Synthetic test tokens (`SyntheticToken`, `MockAuth`) are defined inside test-only modules (`#[cfg(test)]`) or gated behind compile-time synthetic definitions (`-DDEPTHOS_AUTHORITY_UNIT_TEST`).
  2. Production builds do not include synthetic token parsing or execution capabilities.
  3. The native prompt and screen locker observers operate in strict fail-closed mode: any unprompted or synthetic consent attempt is rejected with `Result::Denied`.

#### 5.3 Zero Writable Production D-Bus Methods
- **Threat**: The public D-Bus interface exports methods that allow external applications to invoke leases, trigger dispatches, or modify internal state.
- **Attack Scenario**: An attacker calls `gdbus call --session --dest org.depthos.Controller1 --object-path /org/depthos/Controller1 --method org.depthos.Controller1.ExecuteAction ...`.
- **Defensive Invariant**:
  1. The production D-Bus interface (`org.depthos.Controller1`) exports **strictly read-only** inspection methods:
     * `get_capabilities`
     * `get_context_report`
     * `get_controller_health`
     * `get_current_logical_context`
     * `get_hierarchy`
  2. Exactly one signal is exported: `invalidated`.
  3. No method on `org.depthos.Controller1` takes parameters or initiates state mutations.
  4. Any foreign or unapproved method call returns `org.freedesktop.DBus.Error.UnknownMethod`.

---

## 5. Verification Matrix & Gate Criteria

The following matrix defines the mandatory tests required to satisfy Phase 3.10 verification:

| Attack Vector | Test Identifier | Test Harness / Suite | Expected Outcome |
|---|---|---|---|
| **1.1 Unauthorized Connection** | `test_adversarial_ipc_unauthorized_same_uid` | Python / Rust UDS Client | Connection closed immediately upon missing token. |
| **1.2 Peer Forgery / PID Reuse** | `test_adversarial_ipc_peer_credential_check` | Python `SO_PEERCRED` test | Rejection of mismatched binary path or start time. |
| **1.3 Oversize Payload (>64KiB)** | `test_adversarial_ipc_oversize_payload_rejection` | Python raw socket fuzzer | Socket disconnected immediately; no large buffer allocated. |
| **1.3 Malformed Magic / Version** | `test_adversarial_ipc_malformed_framing` | Python raw socket fuzzer | Immediate disconnect on bad magic (`!= 0x44504F53`). |
| **1.4 Duplicate Packet Replay** | `test_adversarial_ipc_packet_replay_rejected` | Rust integration test | Replayed sequence or duplicate operation ID fails closed. |
| **2.1 Dispatch Without Intent** | `test_adversarial_dispatch_requires_durable_intent` | Rust unit test | Dispatch refuses execution without committed intent receipt. |
| **2.2 Coordinator Reboot Recovery** | `test_adversarial_recovery_post_intent_quarantine` | Rust crash-cut simulation | Post-intent unacknowledged lease restored to `Quarantined`. |
| **2.2 Compositor Incarnation Drift**| `test_adversarial_compositor_restart_fencing` | Rust integration test | Stale compositor incarnation rejects dispatch and ACK. |
| **2.3 Delayed / Duplicate ACK** | `test_adversarial_delayed_ack_cannot_unquarantine` | Rust integration test | Delayed ACK arriving after quarantine fails to lift lock. |
| **2.4 Rejection Non-Mutation** | `test_adversarial_rejected_ack_guarantees_non_mutation`| C++ / Rust test | Rejection verified to leave window state completely untouched. |
| **3.1 Forged Reconciliation** | `test_adversarial_forged_reconciliation_rejected` | Rust gateway test | Rejection of reconciliation from non-observer origin. |
| **3.2 Stale Observer Sequence** | `test_adversarial_stale_observer_seq_rejected` | Rust gateway test | `seq <= quarantine_seq` fails with `StaleObserverSequence`. |
| **3.3 UUID Recreated Window** | `test_adversarial_recreated_window_generation_fence` | Rust gateway test | New window generation does not inherit old quarantine. |
| **3.4 Observer Disconnect Quarantine**| `test_adversarial_observer_disconnect_permanent_quarantine`| Rust gateway test | Window remains locked in `Quarantined` indefinitely. |
| **4.1 Raw SQL Bypass** | `test_adversarial_journal_encapsulation_no_raw_sql` | Rust compilation / API audit | No public access to `rusqlite::Connection`. |
| **4.2 Synchronous FULL Pragma** | `test_adversarial_sqlite_synchronous_full_enforced` | Rust unit test | `PRAGMA synchronous` confirmed `2` (`FULL`) on connection. |
| **4.3 Class E Qualification** | `test_contract_claims_adversarial.py` | Python doc scanner | Class E power-loss verified as formally unsupported. |
| **5.1 Zero Native Movement** | `test_adversarial_kwin_synthetic_zero_native_movement`| C++ / Python test | Zero physical window moves under synthetic execution. |
| **5.2 Zero Consent Minting** | `test_adversarial_zero_production_consent` | C++ test (ASan/UBSan) | Synthetic tokens rejected by production authority kernel. |
| **5.3 Read-Only D-Bus Allowlist**| `test_production_allowlist_adversarial.py` | Python D-Bus auditor | Exactly 5 approved read-only methods and 1 signal. |

---

## 6. Conclusion & Next Steps

This threat model establishes the security criteria for Phase 3.10. Upon completion of implementation by Agent 1:
1. Agent 2 will inspect the implementation commits and code changes.
2. The adversarial test suite will be executed against the concrete cross-process IPC endpoints, SQLite journal encapsulation, and reconciliation gateway.
3. Any identified security vulnerabilities will be reproduced deterministically, patched surgically, and covered with regression tests.
4. Integrated validation across all test suites, Clippy, formatting, and performance benchmarks will precede the final security review and verdict.
