# Phase 3.10 Authoritative Reconciliation Gateway Specification

**Status**: Implemented & Independently Verified  
**Date**: November 2026  
**Scope**: Authoritative Reconciliation Boundary, Anti-Spoofing Evidence Verification, and Fail-Closed Quarantine  

---

## 1. Executive Summary & Purpose

When a cross-process mutation enters an uncertain state (due to lost acknowledgment, socket disconnection after intent commitment, or crash during wire dispatch), the affected window is placed in **Uncertainty Quarantine**.

While quarantined:
- The affected window and its native UUID are locked against all subsequent coordinator mutations.
- Concurrent and subsequent lease requests for that window are strictly refused (`DurableError::WindowQuarantined`).
- Automated redispatch or speculative re-execution is strictly prohibited.

The only safe mechanism to lift a quarantine is through the **Authoritative Reconciliation Gateway**, which accepts ground-truth evidence exclusively from the authenticated DepthOS Observer. This document formalizes the gateway architecture, verification invariants, and fail-closed security properties.

---

## 2. Reconciliation Trust Boundary & Gateway Architecture

```
[Untrusted Clients / Public D-Bus]  ───(Rejected)───┐
                                                    │
[Hostile Same-UID Processes]       ───(Rejected)───┼──> [Reconciliation Gateway]
                                                    │           │
[DepthOS Observer Channel]         ──(Verified)───┘           │ Valid Evidence
  ├── Provenance: "depthos.observer.authoritative"              ▼
  ├── Trusted Observer Authority Token               [Durable Coordinator Session]
  └── Sequence: observer_seq > recorded_seq                     │
                                                      ├── SQLite State: Reconciled
                                                      └── Window Lock Released
```

### Key Gateway Invariants
1. **Isolated Evidence Acquisition**: The gateway rejects reconciliation requests arriving from arbitrary IPC clients, public D-Bus interfaces, or unauthorized peers.
2. **Authority Provenance**: Evidence MUST carry the verified provenance string: `depthos.observer.authoritative`.
3. **Observer Authority Token**: The evidence packet must include the unguessable launcher-issued observer authority token matching `trusted_observer_token`.
4. **Operation & Window Binding**: The evidence must explicitly match:
   - `operation_id` currently recorded in `Quarantined` state in SQLite.
   - Exact `verified_window_key` matching `record.window_key`.
   - Exact `observed_native_uuid` matching `record.native_uuid`.
   - `observer_incarnation` matching the active observer instance.

---

## 3. Strict Monotonic Observer Sequence Fencing

A critical vulnerability addressed in Phase 3.10 is the **Replay of Stale Observer Evidence**. If an attacker or delayed message queue submits an observer snapshot that was generated *before* or *during* the failed mutation attempt, the coordinator could falsely believe the window was in a settled state when in reality a mutation was still in flight.

### The Sequence Fencing Rule
1. When an operation is quarantined, the Coordinator records the last known observer sequence number associated with that operation: `rec.observer_version_seq`.
2. Upon receiving reconciliation evidence, the gateway validates:
   $$\text{evidence.observer\_sequence} > \text{rec.observer\_version\_seq}$$
3. Any evidence where $\text{evidence.observer\_sequence} \le \text{rec.observer\_version\_seq}$ is **strictly rejected** with `ReconciliationError::StaleObserverSequence`.
4. Only evidence generated *strictly after* the quarantine point is accepted.

---

## 4. Window Lifecycle & Generation Verification

If a quarantined window is closed and destroyed by the user, a newly created window might subsequently reuse the same window title, coordinates, or runtime identifier.

### Generation Fencing Rules
1. Windows are identified by a tuple: `(epoch, runtime_id, generation)`.
2. When a window is destroyed, any active quarantine is associated specifically with that window's generation (e.g. `gen-1`).
3. A newly spawned window receives an incremented generation (e.g. `gen-2`).
4. Leases for the new generation `gen-2` are NOT blocked by the unresolved quarantine of `gen-1`, preventing denial-of-service across window lifecycles while ensuring the old generation remains quarantined.

---

## 5. Fail-Closed Permanent Quarantine Principle

Under no circumstances does a quarantine "time out" or resolve automatically in the absence of valid observer evidence.

### Permanent Quarantine Rules
1. **No Automatic Expiry**: Advancing monotonic time (even by hours or days) never lifts a window quarantine.
2. **Observer Disconnect**: If the Observer process dies, crashes, or disconnects, all quarantined windows remain permanently locked in `Quarantined`.
3. **No Speculative Re-execution**: The coordinator never retries a quarantined operation on the assumption that it was never applied.
4. **Clean Operator Reset**: The only path out of quarantine without an active observer is a full coordinator application restart or explicit operator administrative reset.

---

## 6. Contract Boundaries & Architectural Limitations

1. **No Distributed 2PC Atomicity**: This reconciliation architecture is an asynchronous, pessimistic at-most-once protocol with quarantine, not a distributed two-phase commit system.
2. **Software Token Verification**: Observer authority tokens and incarnation UUIDs are software CSPRNG tokens, not hardware cryptographic signatures.
3. **In-Process KWin Limitation**: Extensions loaded into KWin share its monolithic address space; memory isolation between in-process components cannot be enforced without process boundaries.
4. **Power-Loss Safety Qualification**: Hardware power cuts (Class E) cannot be deterministically verified on virtual testbeds and are formally qualified as UNSUPPORTED.
