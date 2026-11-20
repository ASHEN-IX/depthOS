# Phase 3.10.1 Quarantine Identity Consistency Specification

## 1. Threat Model & Dual-Key Hazard

In DepthOS action security, an uncertain dispatch outcome (such as a lost ACK, transport disconnection post-intent, or coordinator crash after intent commit) places the target window into a **quarantine lock**.

### The Identity Aliasing Hazard
A window in Wayland and KWin has multiple identity facets:
1. `WindowKey`: Consists of `(epoch: Uuid, runtime_id: String, generation: String)`.
2. `native_uuid`: A 128-bit persistent UUID assigned by the compositor/observer.
3. Live QObject pointer / file descriptor / Wayland surface handle.

If the coordinator's exclusion policy checks only exact `WindowKey` equality:
- An attacker or racing application could trigger an internal generation bump (e.g. from `gen-1` to `gen-2`).
- A naive coordinator comparing `WindowKey` JSON strings would consider `gen-2` "unlocked" and issue a conflicting reservation for the same uncertain physical resource!
- Conversely, if a window is closed and its ephemeral runtime ID (e.g. `"window-1"`) is reused by the compositor for an unrelated application window, operations on the new window might be improperly blocked or misattributed.

---

## 2. Authoritative Exclusion Policy

DepthOS Phase 3.10.1 enforces a unified, authoritative dual-key exclusion policy across both the in-memory lease manager (`CoordinatorLeaseManager`) and persistent storage (`DurableOperationJournal` in SQLite).

### Identity Classification Taxonomy

| Scenario | Identity Relationship | Exclusion Action | Rationale |
| :--- | :--- | :--- | :--- |
| **1. Same Live Native Object** | Identical `native_uuid` and matching `WindowKey` | **BLOCKED** (`WindowQuarantined`) | Active quarantine lock in place. |
| **2. Generation Drift** | Identical `native_uuid`, different `generation` string | **BLOCKED** (`WindowQuarantined`) | Generation bumps do NOT clear quarantine. The physical surface remains uncertain. |
| **3. Ephemeral Runtime ID Reuse** | Matching `(epoch, runtime_id)`, different `native_uuid` | **BLOCKED** (`WindowQuarantined`) | Within the same observer epoch, a reused runtime ID is ambiguous and cannot be safely distinguished from in-flight aliasing. |
| **4. Proven New Identity** | Different `native_uuid` AND distinct `(epoch, runtime_id)` | **ALLOWED** | Proven separate resource; no overlap with quarantined object. |
| **5. Observer Epoch Change** | New `epoch` after full observer restart & fresh snapshot | **ALLOWED** (provided `native_uuid` is not quarantined) | Fresh epoch guarantees runtime IDs are re-anchored. |
| **6. Missing Observer Continuity** | Disconnected or lagging observer (`epoch` loss or sequence gap) | **FAIL-CLOSED** (`StaleAuthority`) | Operations refused until fresh snapshot re-establishes continuity. |
| **7. Coordinator Restart** | Coordinator reboots while quarantine is recorded in SQLite | **RESTORED** (`WindowQuarantined`) | Startup scan populates in-memory locks from SQLite WAL before any lease is acquired. |

---

## 3. Implementation Details

### In-Memory Tracking (`src/action_security/lease.rs`)
`CoordinatorLeaseManager` maintains three synchronized exclusion sets:
```rust
pub struct CoordinatorLeaseManager {
    // 1. Exact WindowKey exclusion
    quarantined_windows: BTreeMap<WindowKey, LeaseId>,
    // 2. Persistent Native UUID exclusion
    quarantined_native_uuids: BTreeMap<Uuid, (LeaseId, WindowKey)>,
    // 3. Ephemeral Runtime ID exclusion per observer epoch
    quarantined_runtime_ids: BTreeMap<(Uuid, String), (LeaseId, WindowKey)>,
    ...
}
```

During `check_resource_available`:
```rust
// 1. Exact WindowKey check
if let Some(lease_id) = self.quarantined_windows.get(window) {
    return Err(LeaseError::WindowQuarantined(window.clone(), *lease_id));
}
// 2. Native UUID check
if let Some((lease_id, _)) = self.quarantined_native_uuids.get(&native_uuid) {
    return Err(LeaseError::WindowQuarantined(window.clone(), *lease_id));
}
// 3. Runtime ID per epoch check
let runtime_id_key = (window.epoch, window.runtime_id.clone());
if let Some((lease_id, _)) = self.quarantined_runtime_ids.get(&runtime_id_key) {
    return Err(LeaseError::WindowQuarantined(window.clone(), *lease_id));
}
```

### Persistent SQLite Enforcement (`src/action_security/durable.rs`)
In SQLite, `check_resource_available` and `acquire_reservation` inspect all uncompleted operations in `operation_journal`:
1. Check `native_uuid == ?`.
2. Check `window_key` JSON string equality.
3. Parse `window_key` of existing operations: if `existing.epoch == candidate.epoch && existing.runtime_id == candidate.runtime_id`, reject with `DurableError::WindowQuarantined`.

### Restart Safety
When `DurableCoordinatorSession::new()` initializes:
1. It queries `operation_journal` for all records where `durable_state = 'Quarantined'`.
2. It calls `lease_manager.restore_quarantined_record(&rec.window_key, rec.native_uuid, rec.lease_id)`.
3. This populates `quarantined_windows`, `quarantined_native_uuids`, and `quarantined_runtime_ids` **before** any incoming requests can be evaluated.
4. Restarting the coordinator never clears quarantine!

---

## 4. Release Rules

Quarantine can **only** be released by:
1. **Authoritative Reconciliation**: Receipt of `ReconcileEvidenceMessage` carrying `depthos.observer.authoritative`, a valid launcher observer token, fresh sequence number (`seq > recorded`), and an authoritative evidence source (`CompositorExecutionReceipt` or `ProvenNonExecution`).
2. An optimistic generation change or observer version tick **never** releases quarantine.
