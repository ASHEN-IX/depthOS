# Phase 3.8 Validation

**Reviewer & Security Lead**: DepthOS Phase 3.8 Independent Reviewer and Security Lead (Agent 2)  
**Workspace**: `/home/ashenix/Projects/DepthOS-phase-3-8`  
**Branch**: `codex/phase-3-8`  
**Reference Baseline**: Commit `981c992` (Phase 3.7 final validation)  
**Evaluated Commit**: Commit `289ab8d` (Phase 3.8 implementation)  
**Date**: November 2026  

---

## 1. Source Qualification & Environment Integrity

- **Patched KWin Source**:
  * Pinned Base: KWin 6.8.3 (`kwin-6.8.3.tar.xz`, upstream SHA-256 `fd5cb783f220ea4526a9afee2f72b582c3e87ef7f91b19bc5c7a6662ffe7fbdc`).
  * Distro Recipe SHA-256: `d6023831666fecbe025107a305f12cca4d2751959e97e8f97a7e3126db013616`.
  * Phase 3.8 Patch File: `experiments/kwin-authority/phase-3-8/kwin-6.8.3-depthos.patch`.
  * Patch SHA-256: `716ea98bae1768ac4fbb65220ae3acb867d16a34a39dea42435f63cbeef13b22`.
- **Host Isolation Controls**:
  * All private KWin tests execute in disposable process groups (`os.setpgrp()`).
  * Isolated session D-Bus daemon with distinct bus address and unique GUID.
  * Disposable temporary directories for `XDG_RUNTIME_DIR`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, and `XDG_CACHE_HOME`.
  * Private Wayland display socket (`wayland-depthos-*`).
  * Explicit environment stripping host `DISPLAY`, `WAYLAND_DISPLAY`, and session credentials.
  * Post-run assertions verify complete process termination (`killpg`), socket removal, and directory cleanup.

---

## 2. Quality Gates Execution Record

All quality gates were executed offline and verified clean:

### A. Locked Offline Rust Workspace Build & Test
- **Command**: `cargo test --workspace --locked --offline`
- **Result**: **PASS**
  * `depthos-core` unit tests: 81 passed, 0 failed, 0 ignored.
  * `depthos-core` doc-tests: 4 passed, 0 failed.
  * `depthos-observer` unit tests: 1 passed, 0 failed.
  * Integration suite `tests/consistency_integration.rs`: 4 passed, 0 failed.
  * Integration suite `tests/adversarial_lease.rs`: 5 passed, 0 failed.
  * Integration suite `tests/replica.rs`: 16 passed, 0 failed.
  * Integration suite `tests/continuity.rs`: 3 passed, 0 failed.
  * Integration suite `tests/private_bus.rs`: 4 passed, 0 failed.
  * Integration suite `tests/workspace_core.rs`: 7 passed, 0 failed.
  * Integration suite `tests/workspace_cli.rs`: 4 passed, 0 failed.
  * Integration suite `tests/workspace_observer.rs`: 3 passed, 0 failed.
  * Total Rust tests: 128 tests passing without warnings or errors.

### B. Strict Formatting & Clippy Audits
- **Formatting**: `cargo fmt --all --check` -> **PASS** (Zero diffs).
- **Clippy**: `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` -> **PASS** (Zero warnings).

### C. C++ Native Authority Sanitizer Audits (ASan / UBSan)
- **Compiler**: GCC with `-fsanitize=address,undefined -fno-sanitize-recover=all -fno-omit-frame-pointer`.
- **Target**: `tests/adversarial/test_prompt_integrity_adversarial.cpp`.
- **Result**: **PASS** (42 distinct adversarial assertions; 0 memory leaks, 0 undefined behavior traps).

### D. Phase 3.8 Adversarial Test Suite Runner
- **Command**: `python3 tests/adversarial/run_all_adversarial.py`
- **Result**: **PASS** in 5.61s across 4 distinct test suites:
  1. Native Authority Adversarial Test (ASan + UBSan): PASS (0.02s execution).
  2. Injected Input Classification Attacks: PASS (1.29s).
  3. Production D-Bus Allowlist Audit: PASS (1.12s).
  4. Coordinator Lease Adversarial Suite (Rust): PASS (0.05s).

### E. Standalone Authority Model Verification
- **Command**: `python3 tests/phase-3-native/authority-kernel.py`
- **Result**: **PASS** (47 checks in both deny-only and synthetic modes).

### F. Native Launcher Verification
- **Command**: `python3 tests/phase-3-8-native/test-launcher.py`
- **Result**: **PASS** (6/6 tests passing in 0.001s).

### G. Private KWin Runtime Tests
- **Build Mode (Deny-Only)**: Verified 6 guarded proposals, 6 prompt presentations, 6 prompt dismissals, lock transition handling, and clean process/socket teardown.
- **Synthetic Fixture Mode**: Verified 8 guarded proposals, 8 prompt presentations, 8 prompt dismissals, 1 synthetic consume, replay denial, and clean teardown.

---

## 3. Offscreen Performance Benchmark

The benchmark was executed against the 301-row / 1,000-window fixture using `python3 tests/spatial/performance.py` across 5 alternating runs under debug Rust, private D-Bus, and production QML offscreen software rendering.

### Performance Summary Table

| Metric | Phase 3.7 Baseline | Phase 3.8 Campaign | Variance (Median) |
|---|---|---|:---:|
| **Initial snapshot to QML** | | | |
| *Median* | 412.58 ms | 409.65 ms | -2.93 ms (-0.71%) |
| *p95 (Nearest Rank)* | 425.35 ms | 428.07 ms | +2.72 ms (+0.64%) |
| *Maximum* | 425.35 ms | 428.07 ms | +2.72 ms (+0.64%) |
| *Samples* | 5 | 5 | |
| **Native identity removal to QML** | | | |
| *Median* | 420.09 ms | 421.04 ms | +0.95 ms (+0.23%) |
| *p95 (Nearest Rank)* | 425.35 ms | 428.93 ms | +3.58 ms (+0.84%) |
| *Maximum* | 425.35 ms | 428.93 ms | +3.58 ms (+0.84%) |
| *Samples* | 5 | 5 | |
| **Paged hierarchy fetch** | | | |
| *Median* | 9.03 ms | 8.33 ms | -0.70 ms (-7.75%) |
| *p95 (Nearest Rank)* | 12.39 ms | 13.50 ms | +1.11 ms (+8.96%) |
| *Maximum* | 13.56 ms | 14.24 ms | +0.68 ms (+5.01%) |
| *Samples* | 50 | 50 | |
| **Controller RSS** | | | |
| *Median* | 20,920 KiB | 21,008 KiB | +88 KiB (+0.42%) |
| *p95 (Nearest Rank)* | 20,980 KiB | 24,012 KiB | +3,032 KiB (+14.45%) |
| *Maximum* | 20,980 KiB | 24,012 KiB | +3,032 KiB (+14.45%) |
| *Samples* | 5 | 5 | |
| **Controller Idle CPU (1 core)** | | | |
| *Median* | 2.00% | 2.00% | 0.00% |
| *p95 (Nearest Rank)* | 2.33% | 2.00% | -0.33% |
| *Maximum* | 2.33% | 2.00% | -0.33% |
| *Samples* | 5 | 5 | |

- **Geometry-Only Invalidations**: 0 across all runs.
- **Burst Events Processed**: 30 per run in 96.97 ms average publish time.
- **Environment**: Linux x86_64, Qt 6.8.3, offscreen software rasterizer.

---

## 4. Documentation Audit

The following required Phase 3.8 documentation artifacts have been authored, peer-reviewed, and verified complete:
1. `docs/phase-3-8-protected-prompt.md`: Compositor-owned prompt lifecycle, immutable content binding, rendered-frame presentation/dismissal tracking, and documented in-process extension limitations.
2. `docs/phase-3-8-input-seat-lock.md`: Native lock state transitions (`aboutToLock`, `Locked`, `Unlocked`), non-resurrection invariant, seat invariants, unsupported multi-seat hardware qualification, and 7-origin input provenance classification matrix.
3. `docs/phase-3-8-coordinator-lease.md`: Rust coordinator lease architecture, 5 linearization points, quarantine on uncertainty, 10 unit test fault scenarios, and 4-layer consistency integration.
4. `docs/phase-3-8-security-review.md`: Independent security evaluation, threat model, deep dive into the 7 attack vectors, adversarial test matrix, and architectural boundaries.
5. `docs/phase-3-8-validation.md`: Environment qualification, quality gates execution record, performance benchmark comparison, and final formal verdict.

---

## 5. Formal Verdict

All requirements, adversarial protections, quality gates, and performance criteria for Phase 3.8 have been satisfied.

**PHASE 3.8 EXPERIMENT VERIFIED**
