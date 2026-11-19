# Phase 3.10 Cross-Process Protocol Architecture & Specification

**Status**: Implemented & Independently Verified  
**Date**: November 2026  
**Scope**: Private Cross-Process Wire Protocol, Framing, Message Taxonomy, and Finite-State Machine  

---

## 1. Executive Summary & Purpose

Phase 3.10 implements the private cross-process wire protocol connecting:
```
Rust Durable Coordinator <== Private Unix Domain Socket ==> Private Patched KWin Authority
         ||                                                              ||
   SQLite WAL Journal                                          Synthetic Counting Sink
 (PRAGMA sync=FULL)                                           (count_ in-memory only)
```

In Phase 3.8 and Phase 3.9, the coordinator lease manager and SQLite durable operation journal operated within the coordinator process context. Phase 3.10 establishes the cross-process wire protocol, connecting the Rust coordinator with the private patched KWin compositor synthetic authority endpoint over an authenticated local Unix Domain Socket (UDS).

---

## 2. Wire Framing & Protocol Header Layout

Every packet transmitted across the IPC boundary conforms to a strict 20-byte fixed header followed by a variable-length JSON payload:

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       Magic (0x44504F53)                      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|       Version (0x0100)        |       MessageType (u16)       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                       Sequence Number (u64)                   +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       Payload Length (u32)                    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       Payload (N bytes)                       |
|                             ...                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### Framing Fields & Constraints
1. **Magic (`0x44504F53`)**: ASCII `"DPOS"`. Packets with non-matching magic bytes are rejected immediately, and the connection is closed.
2. **Version (`0x0100`)**: Protocol version 1.0. Mismatched versions result in immediate connection termination.
3. **MessageType (`u16`)**: Big-endian opcode identifying the payload schema (values 1–13).
4. **Sequence Number (`u64`)**: Big-endian strictly monotonic sequence number starting at 1 for each stream direction. Duplicate, out-of-order, or regressing sequence numbers trigger connection termination (`IpcError::SequenceMismatch`).
5. **Payload Length (`u32`)**: Big-endian byte count of the payload. **Strict boundary**: `PayloadLen <= 65,536` bytes (64 KiB). Any packet declaring `PayloadLen > 65536` is rejected immediately without allocating memory buffer for the payload.
6. **Payload**: JSON-serialized payload data.

---

## 3. Message Taxonomy

The protocol defines 13 strongly-typed message types:

| Opcode | Message Type | Sender | Purpose | Fields Bound |
|---|---|---|---|---|
| `1` | `Hello` | Coordinator | Handshake initialization | `protocol_version`, `session_id`, `coordinator_incarnation`, `coordinator_epoch`, `bootstrap_token`, `client_pid` |
| `2` | `HelloAccepted` | Compositor | Handshake acknowledgment | `protocol_version`, `session_id`, `compositor_incarnation`, `accepted_epoch`, `server_pid` |
| `3` | `Reserve` | Coordinator | Window reservation notice | `session_id`, `operation_id`, `lease_id`, `coordinator_incarnation`, `coordinator_epoch`, `window_key`, `native_uuid`, `expiry_tick` |
| `4` | `Prepared` | Compositor | Proposal readiness confirmation | `session_id`, `operation_id`, `lease_id`, `compositor_incarnation`, `window_generation`, `ready`, `detail` |
| `5` | `DispatchIntent` | Coordinator | Committed intent notification | `session_id`, `operation_id`, `lease_id`, `coordinator_incarnation`, `coordinator_epoch`, `intent_committed`, `operation`, `operation_digest` |
| `6` | `SyntheticAttempt` | Coordinator | Mutation execution trigger | `session_id`, `operation_id`, `lease_id`, `attempt`, `coordinator_incarnation`, `coordinator_epoch`, `compositor_incarnation`, `window_key`, `native_uuid`, `operation`, `fixture_authority_token` |
| `7` | `SyntheticResult` | Compositor | Diagnostic execution counter | `session_id`, `operation_id`, `lease_id`, `attempt`, `coordinator_incarnation`, `compositor_incarnation`, `consumed_count`, `outcome` |
| `8` | `Acknowledge` | Compositor | Authoritative mutation outcome | `session_id`, `operation_id`, `lease_id`, `attempt`, `coordinator_incarnation`, `compositor_incarnation`, `outcome` (`Applied`, `Rejected`, `IndeterminateError`), `reason` |
| `9` | `Cancel` | Coordinator | Clean pre-dispatch abort | `session_id`, `operation_id`, `lease_id`, `coordinator_incarnation`, `reason` |
| `10` | `Revoke` | Coordinator | Immediate lease revocation | `session_id`, `lease_id`, `coordinator_incarnation`, `reason` |
| `11` | `QuarantineNotice`| Coordinator | Post-intent uncertainty notice | `session_id`, `operation_id`, `lease_id`, `coordinator_incarnation`, `window_key`, `native_uuid`, `reason`, `quarantine_seq` |
| `12` | `ReconcileEvidence`| Observer | Authoritative reconciliation | `session_id`, `operation_id`, `evidence`, `provenance`, `authority_token` |
| `13` | `TerminalReceipt` | Coordinator | Terminal state confirmation | `session_id`, `operation_id`, `lease_id`, `final_state`, `timestamp_ms` |

---

## 4. Finite-State Protocol Machine

The session transitions through four discrete states:

```
[Unauthenticated]
       │
       │ (Hello with valid bootstrap_token)
       ▼
    [Active] <───────── (Dispatch, Ack, Cancel, Revoke)
       │
       │ (Lost ACK / Timeout / Post-Intent Disconnect)
       ▼
 [Quarantined] <─────── (Window locked against mutations)
       │
       │ (ReconcileEvidence from authoritative observer)
       ▼
  [Terminated]
```

### Transition Validation Rules
1. In `Unauthenticated`: Only `Hello` (received by compositor) or `HelloAccepted` (received by coordinator) is permitted. Any other message triggers `IpcError::InvalidTransition` and immediate connection termination.
2. In `Active`: `Reserve` -> `Prepared` -> `DispatchIntent` -> `SyntheticAttempt` -> `Acknowledge` -> `TerminalReceipt`.
3. In `Quarantined`: All subsequent mutations on the quarantined window are refused. The window lock can only be cleared by valid `ReconcileEvidence`.
4. In `Terminated`: The session is terminal and absorbing.

---

## 5. Linearization Sequence (LP 1–8)

The protocol enforces an unbreakable 8-point linearization sequence:

```mermaid
sequenceDiagram
    participant C as Coordinator (Rust)
    participant J as SQLite Journal (WAL)
    participant K as Compositor Endpoint (C++)
    participant O as Observer Gateway

    Note over C,J: LP-1: Lease Reservation (SQLite lock acquired)
    C->>K: Reserve(op_id, lease_id, window_key)
    K-->>C: Prepared(op_id, ready=true, window_gen)
    Note over C,J: LP-2: Lease Fencing Validated
    Note over C,J: LP-3: POINT OF NO RETURN
    C->>J: Commit DispatchIntent (PRAGMA synchronous = FULL)
    Note over C,J: Intent Committed to SQLite WAL
    C->>K: DispatchIntent(op_id, intent_committed=true)
    Note over C,K: LP-4: Synthetic Dispatch Attempt
    C->>K: SyntheticAttempt(op_id, attempt=1, fixture_token)
    Note over K: Kernel evaluates conditions; count_++
    K-->>C: Acknowledge(op_id, outcome=Applied)
    Note over C,J: LP-5: Acknowledgment Persistence
    C->>J: Commit Completed state
    C->>K: TerminalReceipt(op_id, state=Completed)
    Note over C: LP-8: Final Exclusion Lock Released
```

### Uncertainty Handling (LP-6 & LP-7)
If the connection is lost or timed out after LP-3 (DispatchIntent committed to SQLite):
1. **LP-6 (Uncertainty Quarantine)**: Coordinator transitions the operation to `Quarantined` in SQLite and locks the window in the in-memory lease manager. No concurrent or subsequent operation can reserve that window.
2. **Automated Redispatch Strictly Prohibited**: The system does not assume failure or retry.
3. **LP-7 (Authoritative Reconciliation)**: Only fresh evidence from `depthos.observer.authoritative` can release the quarantine.

---

## 6. Security Contract & Claim Boundaries

1. **No Distributed 2PC Atomicity**: This protocol does not provide distributed two-phase commit atomicity across arbitrary networks. It implements at-most-once delivery with pessimistic uncertainty quarantine.
2. **Software Identifiers vs Cryptographic Signatures**: Operation IDs, Lease IDs, and Incarnation UUIDs are software tokens generated via CSPRNG. They are not hardware-backed cryptographic signatures.
3. **Class E Power-Loss Safety Formal Qualification**: Power-loss safety under sudden hardware power cut relies on non-volatile drive caches and hardware write barrier guarantees. Because software virtualization layers lack bare-metal power-loss verification, **Class E safety is formally qualified as UNSUPPORTED**. All guarantees cover Classes A through D (clean disconnection, process crash, concurrency, framing corruption).
4. **Zero Production Consent**: All synthetic operations interact strictly with the private test kernel and increment in-memory `count_`. Zero native window movements or real user consent proofs are issued.
