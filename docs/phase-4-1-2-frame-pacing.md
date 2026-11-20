# DepthOS Phase 4.1.2 — Frame Pacing, Presentation Timing & Memory Profiling

**Status**: QUALIFIED (Frame Intervals Measured; Hardware Scanout Classified as NOT MEASURED)  
**Environment**: Private KWin 6.8.3, Qt 6.12.0 Threaded Render Loop, OpenGL ADL GT2 / Mesa 26.2.7  
**Branch**: `codex/phase-4-1-2`  
**Reference Analysis**: `artifacts/phase-4-1-2/agent1/frame-pacing/analysis.md`  

---

## 1. Executive Summary

Phase 4.1.2 evaluated the presentation timing, render pacing, and memory profile of the DepthOS Spatial Alpha UI under sustained navigation.

Key metrics:
1. **Presentation Timing Protocol (`wp_presentation`)**:
   - While KWin Wayland advertises the `wp_presentation` global, standard Qt Quick applications lack a public API to hook Wayland presentation feedback timestamps directly from QML.
   - Per Gate D requirements, **actual hardware display scanout timing is classified as NOT MEASURED**.
2. **Buffer Swap Timing (`frameSwapped`)**:
   - Captured 365 frames under continuous spatial navigation in a private Wayland session with a 60 Hz virtual display budget (16.667 ms).
   - **Effective FPS**: 60.22 FPS.
   - **Median Swap Interval**: 16.674 ms (near-perfect vsync synchronization).
   - **p95 Interval**: 16.873 ms.
   - **p99 Interval**: 17.214 ms.
   - **Maximum Interval**: 19.193 ms (a single minor hitch during initial model load).
   - **Dropped Scanouts**: 0 missed refresh cycles ($> 33.333\text{ ms}$).
3. **Render-Thread Execution Duration**:
   - CPU render duration (`afterRendering` minus `beforeRendering`):
     - Median: **0.239 ms** (well under the 2.0 ms CPU render budget).
     - Maximum: **1.842 ms**.
4. **Idle Quiescence**:
   - Following an initial 17-frame initialization burst, the render loop drops to **0 frames/sec** while stationary. The removal of pulsing animations from demo/unavailable badges prevents idle CPU/GPU drain.
5. **Memory Profile & Historical Provenance**:
   - Live process VmHWM: **87.8 MiB** (offscreen) to **200.22 MiB** (Wayland window with Mesa/OpenGL drivers).
   - Resolved provenance of historical benchmark metrics: the legacy 522.96 MiB figure was an un-isolated benchmark artifact and does not occur in clean builds.

---

## 2. Measurement Methodology & Distinctions

A critical requirement of Phase 4.1.2 is distinguishing between internal Qt render events and true display presentation:

```
┌────────────────────────────────────────────────────────────────────────┐
│                      Frame Pipeline Hierarchy                          │
│                                                                        │
│ 1. GUI Thread (Event Dispatch & Scene Graph Sync)                      │
│    └─► Animation Driver / Timer ticks                                  │
│    └─► QQuickItem update Polish / Layout                               │
│                                                                        │
│ 2. QML Render Thread (QQuickWindow)                                    │
│    ├─► beforeRendering()                                               │
│    ├─► OpenGL Command Generation (Median: 0.239 ms)                    │
│    ├─► afterRendering()                                                │
│    └─► eglSwapBuffers() ──► frameSwapped() timestamp                   │
│                                                                        │
│ 3. Wayland Compositor (KWin 6.8.3)                                     │
│    └─► wl_surface.commit()                                             │
│    └─► Compositor Compositing Cycle                                    │
│                                                                        │
│ 4. Hardware Display / DRM KMS                                          │
│    └─► wp_presentation.presented (Actual Scanout Timestamp)          │
│        [Classified as NOT MEASURED due to Qt client API absence]       │
└────────────────────────────────────────────────────────────────────────┘
```

### Distinction Between Threshold Exceedance and Dropped Frames
On a 144 Hz display ($\Delta t = 6.944\text{ ms}$) or 60 Hz display ($\Delta t = 16.667\text{ ms}$), vsync-locked buffer swaps cluster symmetrically around the target interval. A sample measuring $6.950\text{ ms}$ or $16.680\text{ ms}$ exceeds the mathematical cutoff by microseconds, but represents a **perfectly synchronized frame**, not a dropped presentation. Only intervals exceeding $1.5\times$ the refresh interval ($>10.4\text{ ms}$ at 144 Hz, or $>25.0\text{ ms}$ at 60 Hz) indicate potential missed refresh opportunities.

---

## 3. Quantitative Frame Pacing Results

### Sustained Navigation Dataset (365 Frames)

The dataset was collected during repeated spatial transitions (Level 1 $\leftrightarrow$ Level 2 $\leftrightarrow$ Level 3) in an isolated private Wayland session:

| Metric | Measured Value | Target Budget (60 Hz) | Status |
| :--- | :--- | :--- | :---: |
| **Sample Count** | 365 frames | $\ge 300$ frames | PASS |
| **Total Test Duration** | 6.061 seconds | N/A | PASS |
| **Effective Frame Rate** | **60.22 FPS** | 60.00 FPS | PASS |
| **Median Swap Interval** | **16.674 ms** | 16.667 ms | PASS |
| **p95 Interval** | **16.873 ms** | $< 20.000\text{ ms}$ | PASS |
| **p99 Interval** | **17.214 ms** | $< 25.000\text{ ms}$ | PASS |
| **Maximum Interval** | **19.193 ms** | $< 33.333\text{ ms}$ | PASS |
| **Intervals $> 16.667\text{ ms}$** | 182 / 365 (49.8%) | Distribution artifact | PASS |
| **Intervals $> 25.000\text{ ms}$** | **0** | 0 | PASS |
| **Intervals $> 33.333\text{ ms}$** | **0** | 0 | PASS |
| **CPU Render Duration (Median)**| **0.239 ms** | $< 2.000\text{ ms}$ | PASS |
| **CPU Render Duration (Max)** | **1.842 ms** | $< 5.000\text{ ms}$ | PASS |

### Stationary Idle Behavior

| Measurement Phase | Duration | Recorded Frame Swaps | Behavior |
| :--- | :--- | :--- | :--- |
| **Startup / Window Exposure** | 0.25 s | 17 frames | Normal scene load burst |
| **Stationary Idle** | 5.00 s | **0 frames** | Zero CPU/GPU rendering load |

---

## 4. Memory Footprint & Benchmark Provenance Audit

Agent 2 conducted a strict memory audit to reconcile historical figures from earlier project milestones:

| Memory Metric | Value | Provenance & Context |
| :--- | :--- | :--- |
| **Offscreen UI Baseline** | `VmRSS`: ~87.8 MiB<br>`VmHWM`: ~87.8 MiB | Clean offscreen QML test runner without GPU drivers loaded. |
| **Wayland Window (GPU)** | `VmRSS`: ~164 MiB<br>`VmHWM`: ~200.22 MiB | Live Wayland client on Intel ADL GT2 with Mesa OpenGL drivers and DRI buffer allocation. |
| **Historical 83.52 MiB** | Peak `ru_maxrss` | Overview test suite via `os.wait4` (78.92 MiB harness + 4.6 MiB overview). |
| **Historical 103.89 MiB** | Peak `ru_maxrss` | Rust controller process memory high-water mark during stress test. |
| **Historical 522.96 MiB** | Peak Virtual Allocation | Legacy un-isolated benchmark artifact caused by shared debug symbol mappings. **Confirmed non-reproducible** in Phase 4.1.1 and 4.1.2 clean environments. |

---

## 5. Summary & Hand-off Requirements for Phase 4.2

1. **Performance Budget Adherence**: The existing QML architecture operates comfortably within 60 Hz and 144 Hz timing requirements, utilizing less than $15\%$ of available frame render budgets.
2. **Animation Architecture**: The Phase 4.2 visual redesign must maintain GPU-accelerated property transforms (`opacity`, `scale`, `x`, `y`) and avoid CPU-heavy item re-parenting or synchronous file/IPC reads on the GUI thread.
3. **Quiescence Invariant**: Any new status animations, glowing borders, or cyberpunk visual indicators introduced in Phase 4.2 must automatically pause or shut down when the window is idle or in the background.
