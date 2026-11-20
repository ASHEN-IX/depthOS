# Phase 3.10.1 Reconciliation Source of Truth Specification

## 1. Disentangling Placement Observation from Execution Proof

A fundamental hazard in distributed window management systems is treating **observed window placement** as **proof of execution**:

> **Critical Security Insight**:  
> If an observer reports that window $W$ is currently located on desktop $D_{\text{dest}}$, this is **NOT** proof that DepthOS moved the window!  
> The window may have been moved by:
> - Direct user interaction (dragging the title bar or using native KDE shortcuts).
> - An external script or automation tool via KWin scripting.
> - An uncoordinated prior operation.

If DepthOS dispatch outcome is uncertain (e.g., lost ACK post-intent), observing the window on $D_{\text{dest}}$ cannot prove that our specific dispatched operation succeeded. Inferring success from passive placement observations risks masking corrupted state machines or split-brain races.

---

## 2. Evidence Source Taxonomy & Authority Hierarchy

DepthOS Phase 3.10.1 establishes a strict four-tier evidence source taxonomy:

```
                                  [Evidence Source]
                                          |
        +------------------+--------------+--------------+------------------+
        |                  |                             |                  |
        v                  v                             v                  v
[CompositorExecution]  [ProvenNonExecution]    [ObserverSnapshotOnly]   [Uncertain]
    (Receipt from         (Proof that no          (Passive placement       (Ambiguous
     compositor that       action was taken        observation from         or missing
     operation executed)   or intent was aborted)  observer channel)        data)
        |                  |                             |                  |
        v                  v                             v                  v
  LIFTS QUARANTINE   LIFTS QUARANTINE           FAILS CLOSED INTO     FAILS CLOSED INTO
  (ReconciledApplied)(ReconciledAborted)        INDEFINITE QUARANTINE INDEFINITE QUARANTINE
                                                (UnprovenExecution)   (UncertainOutcome)
```

### 1. `EvidenceSource::CompositorExecutionReceipt`
- **Definition**: Direct cryptographic or IPC evidence from the compositor runtime affirming that the specific operation, lease, and attempt was dispatched and executed.
- **Action**: Transitions durable operation to `DurableOperationState::Reconciled` and clears quarantine locks in both SQLite and memory.

### 2. `EvidenceSource::ProvenNonExecution`
- **Definition**: Direct authoritative proof that the compositor rejected the operation or aborted before taking effect (e.g., focus lost or window destroyed prior to consumption).
- **Action**: Transitions durable operation to `DurableOperationState::Reconciled` (aborted outcome) and clears quarantine locks.

### 3. `EvidenceSource::ObserverSnapshotOnly`
- **Definition**: Passive observation of the window's current placement without a corresponding execution receipt from the compositor.
- **Action**: **REJECTED** with `ReconciliationError::UnprovenExecutionObserverSnapshotOnly`. Quarantine is retained indefinitely!

### 4. `EvidenceSource::Uncertain`
- **Definition**: Partial, missing, or ambiguous evidence.
- **Action**: **REJECTED** with `ReconciliationError::UncertainOutcome`. Quarantine is retained indefinitely!

---

## 3. Strict Evidence Protocol Binding

To prevent spoofing, cross-session replays, or accidental misattribution, every `ReconciliationEvidence` packet must validate a comprehensive set of identity and incarnation bindings:

```rust
pub struct ReconciliationEvidence {
    pub observer_incarnation: Uuid,
    pub observer_sequence: u64,
    pub verified_window_key: WindowKey,
    pub observed_native_uuid: Uuid,
    pub observed_placement: Placement,
    pub authoritative_decision: String,
    pub evidence_source: EvidenceSource,
    pub terminal_claim: TerminalClaim,
    pub dispatch_attempt: u32,
    pub operation_id: Option<RequestId>,
    pub lease_id: Option<LeaseId>,
    pub coordinator_incarnation: Option<Uuid>,
    pub compositor_incarnation: Option<Uuid>,
}
```

### Gateway Validation Pipeline (`AuthoritativeReconciliationGateway::process_reconciliation`)
1. **Provenance Check**: `msg.provenance == "depthos.observer.authoritative"`.
2. **Authority Token Check**: `msg.authority_token == self.trusted_observer_token`.
3. **Quarantine State Check**: Record in SQLite must be in `DurableOperationState::Quarantined`.
4. **Dual-Key Check**: `msg.evidence.verified_window_key == rec.window_key` AND `msg.evidence.observed_native_uuid == rec.native_uuid`.
5. **Incarnation Check**: `msg.evidence.observer_incarnation == self.active_observer_incarnation == rec.observer_incarnation`.
6. **Explicit Bindings Check**: If `operation_id`, `lease_id`, `coordinator_incarnation`, or `compositor_incarnation` are provided in evidence, they must strictly match `rec`.
7. **Monotonic Sequence Fencing**: `msg.evidence.observer_sequence > rec.observer_version_seq`. (Replaying pre-quarantine observer snapshots is impossible).
8. **Decision Non-Empty**: `authoritative_decision` must be non-empty after trimming.
9. **Evidence Source Gate**:
   - `ObserverSnapshotOnly` -> Fails with `UnprovenExecutionObserverSnapshotOnly`.
   - `Uncertain` -> Fails with `UncertainOutcome`.
   - `CompositorExecutionReceipt` or `ProvenNonExecution` -> Commits transition to SQLite WAL and clears in-memory lease quarantine.

---

## 4. Verification Evidence

As tested in `tests/adversarial_phase_3_10_1.rs`:
- Test 4: Forged provenance candidate strings rejected.
- Test 5: Forged / stolen observer token rejected.
- Test 6: Stale (`seq < recorded`) and equal (`seq == recorded`) sequence numbers rejected.
- Test 7: Mismatched observer incarnation rejected.
- Test 8: `ObserverSnapshotOnly` and `Uncertain` sources fail closed and keep quarantine locked indefinitely.
- Test 9: `ProvenNonExecution` and matching explicit bindings successfully lift quarantine.
