# DepthOS Phase 4.5 — Management Bridge Security & Operational Hardening

## 1. Architecture Overview

The DepthOS Spatial Management Bridge connects user-driven QML interactions (creating worlds, editing workspaces, reordering logical desktops, managing preferences) to the Rust SQLite engine:

```
┌──────────────────────────────────────────────┐
│ Qt Quick / QML User Interface               │
│ (SpatialScene, Dialogs, ControlCenter)       │
└──────────────────────┬───────────────────────┘
                       │ JSON request string via PyQt slot
                       ▼
┌──────────────────────────────────────────────┐
│ tools/depthos-spatial-alpha (HostBridge)     │
│ - 1 MiB payload bounding                     │
│ - Action whitelisting & validation           │
│ - 10.0s subprocess timeout enforcement      │
│ - Structured error synthesis                 │
└──────────────────────┬───────────────────────┘
                       │ CLI invocation (`depthos manage exec --request ...`)
                       ▼
┌──────────────────────────────────────────────┐
│ depthos core (Rust)                          │
│ - SQLite WAL mode with schema v3             │
│ - Optimistic concurrency validation          │
│ - Scoped duplicate detection                 │
│ - Atomic transactions with rollback          │
└──────────────────────────────────────────────┘
```

---

## 2. Hardened Security Guarantees

### 2.1 Payload Size Bounding
Before initiating process execution, `HostBridge.executeManagement` enforces a strict 1 MiB ($1,048,576$ bytes) ceiling on incoming request strings:
- Rejects oversized payloads immediately with error code `"invalid_payload"`.
- Prevents memory exhaustion attacks and unbounded IPC buffering.

### 2.2 Action Whitelist Verification
The bridge verifies that the requested action is an explicit member of the permitted application-owned set:
```python
allowed_actions = {
    "create", "update", "reorder", "archive", "restore",
    "delete", "inspectProject", "getPreferences", "setPreferences"
}
```
Any action outside this set is refused with error code `"invalid_action"` prior to executing any subprocess.

### 2.3 Subprocess Timeout Enforcement
All calls to the `depthos` CLI pass through `run_depthos` with an explicit `timeout=10.0` parameter:
- If a lock contention or filesystem stall delays execution beyond 10 seconds, `subprocess.TimeoutExpired` is trapped.
- Returns a structured JSON response:
  ```json
  {
    "success": false,
    "error": "depthos command timed out after 10.0 seconds",
    "code": "timeout"
  }
  ```
- Prevents UI thread deadlock or zombie process accumulation.

### 2.4 Strict Native Boundary Invariant
The management bridge operates exclusively on DepthOS-owned logical records in SQLite.
- It contains **zero** native window mutation endpoints.
- It exposes **no** D-Bus setter endpoints for KWin.
- Host KWin observer scripts and host shortcuts are not modified.
- Production authorization remains strictly **DENY-ONLY**.

---

## 3. Structured Error Response Protocol

All responses from `executeManagement` conform to a deterministic JSON schema:

| Error Condition | HTTP-style Error Code | Payload Summary |
|---|---|---|
| Payload > 1 MiB or empty | `"invalid_payload"` | Rejection before parsing |
| Malformed JSON syntax | `"malformed_json"` | Parser exception message |
| Unauthorized action string | `"invalid_action"` | Allowed action set list |
| Database unconfigured | `"no_database"` | User prompt to initialize |
| Missing `depthos` binary | `"missing_binary"` | Binary search failure |
| Execution timeout (>10s) | `"timeout"` | Timeout duration detail |
| Process failure / non-zero exit | `"exec_failed"` | Subprocess stderr / stdout |
| Stale version conflict | `"conflict"` | Upstream optimistic concurrency reject |

---

## 4. Verification and Validation

The hardened bridge was validated using both unit fixtures and the full integration suite:
- `tests/spatial/test_phase_4_4_management.py` (30 checkpoints clean)
- Optimistic concurrency conflict validation (code `"conflict"`)
- Direct payload size injection test ($> 1$ MiB payload successfully rejected)
- Disallowed action rejection test (`"launchNuclearRocket"` successfully rejected)
