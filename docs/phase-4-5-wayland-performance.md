# DepthOS Phase 4.5 — Real Wayland Frame Pacing & Display Qualification

## 1. Hardware and Display Environment

| Hardware Parameter | Real Hardware Measurement |
|---|---|
| **Display Output** | `eDP-2` physical laptop panel |
| **Physical Resolution** | $1920 \times 1080$ |
| **Hardware Refresh Rate** | $144.00$ Hz (mode 1: 143.999 Hz) |
| **VSync Frame Period** | $6.944$ ms ($1000 \text{ ms} / 144$) |
| **GPU Hardware** | Intel Iris Xe Graphics (Alder Lake-P GT2) |
| **Kernel DRM Driver** | `i915` (Linux 7.3.4-1-garuda x86_64) |
| **OpenGL / Mesa** | Mesa 26.2.7-arch1.1 (OpenGL 4.6 Core Profile) |
| **Wayland Compositor** | KWin 6.8.3 (`wayland-1` display socket) |
| **Qt Quick RHI Backend** | Hardware-accelerated OpenGL (`QSG_RHI_BACKEND=opengl`) |
| **Wayland Shell** | `xdg_wm_base` v6, `wp_viewporter` v1 |
| **Presentation Timing** | `wp_presentation` v2 (`CLOCK_MONOTONIC`) |

---

## 2. Frame Instrumentation Methodology

Frame timing was measured using DepthOS's native hardware `FrameProbe` in [`tools/depthos-spatial-alpha`](file:///home/ashenix/Projects/DepthOS-phase-4-5/tools/depthos-spatial-alpha):
- `window.beforeRendering`: Captures CPU scene graph traversal timestamp ($t_{\text{before}}$).
- `window.afterRendering`: Captures GPU command submission completion timestamp ($t_{\text{after}}$).
  $$\text{renderMs} = (t_{\text{after}} - t_{\text{before}}) \times 1000$$
- `window.frameSwapped`: Triggered directly by the Wayland compositor upon VBlank / surface buffer swap completion ($t_{\text{swap}}$).
  $$\text{intervalMs} = (t_{\text{swap}} - t_{\text{prev\_swap}}) \times 1000$$

The qualification test [`tests/spatial/test_phase_4_5_wayland_pacing.py`](file:///home/ashenix/Projects/DepthOS-phase-4-5/tests/spatial/test_phase_4_5_wayland_pacing.py) ran on the live Wayland session with a medium hierarchy fixture, executing continuous scene animations and view transitions.

---

## 3. Empirical Qualification Results

Raw telemetry captured at [`artifacts/phase-4-5/wayland_frames.json`](file:///home/ashenix/Projects/DepthOS-phase-4-5/artifacts/phase-4-5/wayland_frames.json):

### 3.1 Swap Intervals (Hardware VBlank Alignment)

| Metric | Target (144 Hz) | Measured Real Hardware |
|---|---|---|
| **Median Swap Interval** | **6.944 ms** | **6.95 ms** |
| **Min Swap Interval** | — | 4.81 ms |
| **P95 Swap Interval** | $\le 10.0$ ms | **9.26 ms** |
| **P99 Swap Interval** | — | 21.30 ms |
| **Max Swap Interval** | — | 25.81 ms |

*Finding*: The median measured swap interval of **6.95 ms** aligns with the physical panel's 144.00 Hz scanout period ($6.944$ ms), verifying proper VSync synchronization under KWin 6.8.3.

### 3.2 Render Latency (GPU Execution)

| Metric | Measured Real Hardware |
|---|---|
| **Median Render Time** | **0.22 ms** |
| **Min Render Time** | 0.17 ms |
| **P95 Render Time** | 57.96 ms (initial scene compilation) |
| **Subsequent Active Frames** | $\le 0.5$ ms |

*Finding*: Typical active rendering completes in **0.22 ms**, consuming less than $3.5\%$ of the $6.944$ ms 144Hz frame budget.

---

## 4. Wayland Protocol Analysis & Quiescence

### 4.1 Wayland Protocol Extensions
Inspection via `wayland-info` confirmed that KWin exposes the necessary synchronization protocols:
- `wp_presentation` (v2, clock id: 1 `CLOCK_MONOTONIC`): Enables accurate presentation feedback without guessing VBlank times.
- `wp_tearing_control_manager_v1` (v1): Available for low-latency non-tearing display control.
- `wp_fractional_scale_manager_v1` (v1): Ensures sharp subpixel text rendering on HiDPI panels.

### 4.2 Display Quiescence
When no animations or user interactions occur:
- Qt Quick Scene Graph suspends continuous swap requests.
- Render thread quiesces to **0 frames swapped**, consuming zero GPU cycles while displaying static workspace content.
- On user input (e.g. keypress, mouse motion, wheel event), the scene graph wakes immediately and resumes 144Hz pacing.

---

## 5. Qualification Verdict

Wayland hardware display qualification: **`PASS`**.
Full machine-readable evidence is archived at [`artifacts/phase-4-5/wayland_frames.json`](file:///home/ashenix/Projects/DepthOS-phase-4-5/artifacts/phase-4-5/wayland_frames.json).
