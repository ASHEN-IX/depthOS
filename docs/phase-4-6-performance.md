# Phase 4.6 Installed Application Performance & Resource Benchmark Report

**Author**: Independent QA & Security Engineer (Agent 2)  
**Branch**: `codex/phase-4-6`  
**Test Suite**: `tests/spatial/test_phase_4_6_installed_perf.py`  
**Evaluation Scope**: Startup latency, time to usable UI, steady-state memory (RSS, PSS, Private Dirty), idle CPU consumption, and Wayland frame pacing on the installed application.

---

## 1. Executive Performance Summary

This benchmark qualifies the performance characteristics of DepthOS Spatial Alpha running from an installed release package in a clean disposable prefix, invoked from an arbitrary working directory outside the development workspace.

### Key Performance Budgets & Targets

| Metric | Target Budget | Measured Median | Evaluation Status |
|---|:---:|:---:|:---:|
| **Cold Startup Latency** | $< 250$ ms | TBD | PENDING |
| **Time to Usable UI** | $< 500$ ms | TBD | PENDING |
| **Settled Resident Set Size (RSS)** | $< 120$ MiB | TBD | PENDING |
| **Proportional Set Size (PSS)** | $< 85$ MiB | TBD | PENDING |
| **Private Dirty Memory** | $< 50$ MiB | TBD | PENDING |
| **Idle CPU Utilization** | $< 1.0\%$ | TBD | PENDING |
| **Frame Swap Interval** | $\sim 16.67$ ms (60 Hz) | TBD | PENDING |
| **Frame Swap Jitter (P95 - Median)** | $< 2.0$ ms | TBD | PENDING |
| **Dropped Frames** | 0 frames | TBD | PENDING |

---

## 2. Measurement Methodology

1. **Clean-Room Prefix**: The application is installed via `pkg/install.sh --prefix [PREFIX]`. All benchmarks execute the binary at `[PREFIX]/bin/depthos-spatial-alpha` with sanitized environment variables (zero development paths).
2. **Cold Startup**: Spawns process instances with `--quit-after-ms 1` to measure process initialization, dynamic library resolution, and Qt Quick runtime bootstrap time.
3. **Time to Usable UI**: Measures the wall-clock time from process spawn until the SQLite logical tree is read, scene graph models are populated, and the first frame render completes.
4. **Kernel Memory Rollup**: Memory metrics are sampled directly from `/proc/[PID]/smaps_rollup`, providing hardware-accurate PSS, Private Dirty, Private Clean, and RSS allocations after process warmup.
5. **Idle CPU Sampling**: CPU utilization is measured across a 3.0-second stationary period using `/proc/[PID]/stat` tick differentials divided by `sysconf(_SC_CLK_TCK)`.
6. **Frame Timing & Pacing**: Real swap intervals and render latencies are captured over 100+ consecutive frames using DepthOS's built-in frame logging harness (`--frame-log`).

---

## 3. Telemetry Results & Data Tables

*Telemetry tables will be populated with exact values recorded in `artifacts/phase-4-6/installed_perf.json`.*

### Memory Footprint Breakdown
- **Settled RSS**: TBD
- **Proportional Set Size (PSS)**: TBD
- **Private Dirty (Heap + Anon)**: TBD
- **Private Clean (Mapped Binaries)**: TBD

### Frame Pacing & Presentation
- **Total Frames Measured**: TBD
- **Median Swap Interval**: TBD
- **P95 Swap Interval**: TBD
- **P99 Swap Interval**: TBD
- **Jitter**: TBD
- **Median Render Time**: TBD
- **Dropped Frames**: TBD

---

## 4. Hardware Environment Specification

- **Processor**: Intel Core i7-12700H (14 cores, 20 threads)
- **Graphics**: Intel Alder Lake GT2 Iris Xe Graphics
- **Mesa Driver**: Mesa 26.2.7
- **Compositor**: KWin 6.8.3 Wayland
- **Kernel**: Linux 6.18
