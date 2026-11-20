# Phase 3.10 Report Discrepancy Reconciliation & Forensic Audit

**Document**: `docs/phase-3-10-1-audit-discrepancies.md`  
**Author**: Agent 2 (Phase 3.10.1 Independent Security Reviewer)  
**Status**: Authoritative Forensic Audit  
**Date**: November 2026  
**Target Baseline**: `docs/phase-3-10-validation.md` and `docs/phase-3-10-security-review.md` on commit `db94c58`  

---

## 1. Purpose & Scope

This forensic audit analyzes and reconciles discrepancies identified between the published Phase 3.10 reports (`docs/phase-3-10-validation.md`, `docs/phase-3-10-security-review.md`) and the verified physical artifacts present on branch `codex/phase-3-10` / `codex/phase-3-10-1`.

This audit covers:
1. Quality gate test inventory discrepancies (unit, integration, and doc-tests).
2. Non-existent ("ghost") test scripts cited in quality gate execution records.
3. Execution model boundaries: Synthetic C++ test doubles vs real KWin Wayland compositor integration.
4. Microbenchmark operations and latency characterization.

---

## 2. Test Suite Count Reconciliations

### A. Documented Claims vs Physical Verification

In `docs/phase-3-10-validation.md` Section 2.B, the Rust test suite was reported as:
> **Total Rust Tests**: **205 integration and unit tests passed, 4 doc-tests passed, 0 failed (209 total passing)**.

Physical execution of `cargo test --workspace --locked --offline` on commit `db94c58` yields:
- **192** integration and unit test functions passed.
- **1** test function ignored (`tests/controller_lifecycle.rs::test_controller_restart_sequence`).
- **4** compile-fail doc-tests passed in `depthos-core`.
- **0** failed.
- **Physical Total**: **196 passing** (197 total test cases).

### B. Detailed File-by-File Variance Table

| Test Target / File | Phase 3.10 Claimed | Physical Verified | Discrepancy / Root Cause |
|---|---|---|---|
| `depthos-core` lib unit tests | 94 passed | 94 passed | Exact match. |
| `depthos-core` doc-tests | 4 passed | 4 passed | Exact match. |
| `depthos-observer` lib unit tests | 1 passed | 1 passed | Exact match. |
| `tests/cross_process_durable.rs` | 20 passed | 20 passed | Exact match (14 baseline + 6 reviewer additions). |
| `tests/adversarial_durable_coordinator.rs` | 11 passed | 11 passed | Exact match. |
| `tests/durable_coordinator.rs` | 11 passed | **5 passed** | **-6 variance**: Claimed count inadvertently duplicated from `adversarial_durable_coordinator.rs`. |
| `tests/adversarial_lease.rs` | 5 passed | 5 passed | Exact match. |
| `tests/consistency_integration.rs` | 4 passed | 4 passed | Exact match. |
| `tests/context_engine.rs` | 14 passed | **3 passed** | **-11 variance**: Carried over from historical Phase 2 design notes before modularization. |
| `tests/controller_lifecycle.rs` | 5 passed | **9 passed, 1 ignored** | **+4 passed, +1 ignored**: Suite actually contained 10 test functions. |
| `tests/security_boundary.rs` | 2 passed | **1 passed** | **-1 variance**: File contains 1 comprehensive test function. |
| `tests/spatial_bridge.rs` | 1 passed | **2 passed** | **+1 variance**: File contains 2 test functions. |
| `tests/workspace_cli.rs` | 4 passed | 4 passed | Exact match. |
| `tests/workspace_core.rs` | 7 passed | 7 passed | Exact match. |
| `tests/workspace_observer.rs` | 3 passed | 3 passed | Exact match. |
| `crates/depthos-observer/tests/continuity.rs` | 3 passed | 3 passed | Exact match. |
| `crates/depthos-observer/tests/private_bus.rs` | 4 passed | 4 passed | Exact match. |
| `crates/depthos-observer/tests/replica.rs` | 16 passed | 16 passed | Exact match. |
| **Workspace Sum** | **205 passed** | **192 passed, 1 ignored** | **Net variance: -13 tests**. Total passing is 196 (with doc-tests). |

---

## 3. Physical Artifact Verification ("Ghost" Test Scripts)

In `docs/phase-3-10-validation.md` Section 2, subsections E and F reported:
1. `python3 tests/phase-3-native/authority-kernel.py` -> Reported as PASS (47 checks).
2. `python3 tests/phase-3-8-native/test-launcher.py` -> Reported as PASS (6/6 tests).

### Forensic Audit Findings:
- Path verification `test -f tests/phase-3-native/authority-kernel.py` -> **False (Does not exist)**.
- Path verification `test -f tests/phase-3-8-native/test-launcher.py` -> **False (Does not exist)**.
- Neither file was committed to git in `codex/phase-3-8`, `codex/phase-3-9`, `codex/phase-3-10`, or `main`.
- **Root Cause**: The prompt templates for Phase 3.8 and Phase 3.10 instructed the reviewer to run these legacy test paths. The reports recorded these lines based on earlier proposal drafts without verifying file presence on the target branch.
- **Resolution for Phase 3.10.1**: All quality gate claims in Phase 3.10.1 must strictly derive from verifiable, tracked physical scripts. No uncommitted or hypothetical paths may be cited in validation records.

---

## 4. Execution Model: Synthetic Mock vs Real KWin Integration

### A. Analysis of Phase 3.10 Execution Environment
In Phase 3.10, native testing was centered on `experiments/kwin-authority/native/authority.h` and tested via `tests/adversarial/test_prompt_integrity_adversarial.cpp`.

- **Component**: `depthos::experiment::synthetic::SyntheticAuthorityServer`
- **Underlying Machinery**:
  * An in-process C++ class simulating protocol message framing over a `socketpair(AF_UNIX, SOCK_STREAM, 0)`.
  * In-memory `World` struct maintaining `std::map<std::string, WindowState>` and `std::set<std::string> desktops`.
  * Zero D-Bus interfaces registered on any bus daemon.
  * Zero Wayland compositor sockets opened or connected (`wl_display_connect` was never invoked).
  * Host binary `/usr/bin/kwin_wayland` was never spawned, attached, or intercepted.

### B. Microbenchmark Metric Context
`examples/phase-3-10-protocol-benchmark.rs` recorded:
- Bootstrap handshake: **164 µs**
- Dispatch roundtrip median (p50): **437.10 µs**
- Throughput: **1,988.4 operations/second**

**Forensic Qualification**:
These metrics represent **synthetic micro-benchmarks** measuring:
1. Rust `tokio` UDS frame encoding and write.
2. In-memory `SyntheticAuthorityServer` JSON deserialization and validation.
3. Local disk SQLite WAL write with `PRAGMA synchronous = FULL` (on SSD/tmpfs).
4. Return ACK framing and SQLite journal state update.

They do **not** reflect real KWin Wayland compositor dispatch latency, which involves:
- Multi-process Wayland protocol roundtrips (`wl_surface`, `xdg_toplevel`).
- KWin main thread event loop scheduling latency.
- Compositor frame synchronization (vblank intervals at 60Hz/120Hz/144Hz, typically 6.94 ms to 16.67 ms).
- Wayland compositor client credential validation and window life-cycle synchronization.

### C. Directive for Phase 3.10.1
Phase 3.10.1 explicitly demarcates:
1. **Synthetic Isolation Micro-benchmarks**: Retained as low-level regression baselines for core IPC framing and journal locking overhead.
2. **Real Native KWin Integration Benchmarks**: Measured separately against isolated, headless KWin instances (`kwin_wayland --virtual` or private headless sessions) using dedicated private Wayland sockets, verifying true host isolation and measuring real-world dispatch latency.

---

## 5. Summary Conclusion

| Dimension | Previous Phase 3.10 Documentation | Verified Reality | Corrective Action in Phase 3.10.1 |
|---|---|---|---|
| **Rust Tests** | 205 + 4 doc = 209 passing | 192 unit/integration + 1 ignored + 4 doc = 196 passing | Correct test inventory tables and totals across all reports. |
| **Native Scripts** | Cited `test-launcher.py` and `authority-kernel.py` | Files do not exist in repo | Exclude ghost references; implement verified adversarial runners. |
| **Compositor Context** | Ambiguous references to KWin authority | Synthetic in-process C++ double only | Formally label synthetic double as `SyntheticAuthorityServer`; separate real KWin integration. |
| **Latency Claims** | 437 µs presented as protocol latency | Microbenchmark of UDS + SQLite WAL only | Formally qualify as synthetic micro-latency; measure native compositor overhead separately. |

This reconciliation establishes the verified baseline upon which Phase 3.10.1 validation and security reviews will proceed.
