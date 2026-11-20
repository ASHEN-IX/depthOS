# DepthOS Phase 4.1.2 — Touchpad Gesture Feasibility & Qualification

**Status**: QUALIFIED (Pinch: UNSUPPORTED / NOT DELIVERED to client windows; Scroll: SUPPORTED)  
**Environment**: KDE Plasma 6.8.3, KWin Wayland, Qt 6.12.0, libinput, libei 1.6  
**Branch**: `codex/phase-4-1-2`  
**Reference Tool**: `tools/depthos-gesture-probe`  

---

## 1. Executive Summary

Touchpad input qualification in Phase 4.1.2 evaluated the exact event delivery mechanisms available from KWin Wayland to an ordinary Qt Quick application window.

Key findings:
1. **Touchpad Two-Finger Scrolling**: **SUPPORTED**. Delivered cleanly via `wl_pointer.axis` as `QWheelEvent` with high-resolution pixel deltas. Handled by `WheelHandler` for 2D spatial canvas panning.
2. **Depth Zoom via Scroll**: **SUPPORTED**. Delivered as `Ctrl` + Wheel (`QWheelEvent` with `Qt.ControlModifier`). Features a 320ms cooldown and direction latching to prevent runaway transitions.
3. **Physical Touchpad Pinch-to-Zoom**: **UNSUPPORTED / NOT DELIVERED to Client Windows**.
   - KWin Wayland advertises the `zwp_pointer_gestures_v1` global.
   - However, in standard KDE Plasma desktop sessions, 3-finger and 4-finger pinches are intercepted globally by KWin for desktop effects (Overview / Virtual Desktop Grid).
   - 2-finger pinch gestures on touchpads are mapped by libinput to scrolling/pointer events unless explicit gesture grab protocols are granted.
   - Headless and private automated test environments using `libei` (EIS) **do not support** `zwp_pointer_gestures_v1` injection (`libei` exposes only pointer motion, buttons, and scroll).
   - Unattended live session probes recorded 0 `QNativeGestureEvent` pinch updates.
4. **Synthetic Events vs Hardware Reality**: Synthetic calls (`pinchBegin`, `pinchUpdate`, `pinchEnd`) verify mathematical zoom calculations in QML tests, but **do not represent hardware touchpad delivery**.
5. **No Degradation of Core UX**: The absence of physical touchpad pinch does not impair navigation. Robust primary navigation paths remain keyboard (`H/J/K/L`, `Arrows`, `Enter`, `Esc`), mouse click / double-click, and two-finger touchpad scrolling.

---

## 2. Input Event Taxonomy & Wayland Boundary

To prevent confusing simulated success with real hardware capability, DepthOS establishes an explicit input taxonomy:

| Input Modality | Compositor Protocol | Qt Event Type | Qt Quick Handler | DepthOS Capability |
| :--- | :--- | :--- | :--- | :--- |
| **Mouse Wheel (Discrete)** | `wl_pointer.axis` (v8 discrete) | `QWheelEvent` (angleDelta) | `WheelHandler` | **SUPPORTED**: 96px per 120-unit notch |
| **Touchpad 2-Finger Scroll** | `wl_pointer.axis` (continuous) | `QWheelEvent` (pixelDelta) | `WheelHandler` | **SUPPORTED**: 1:1 smooth spatial canvas pan |
| **Ctrl + Scroll (Depth Zoom)** | `wl_pointer.axis` + modifiers | `QWheelEvent` (`ControlModifier`) | `scene.handleWheel()` | **SUPPORTED**: 1 level step per burst (320ms cooldown) |
| **Touchscreen Pinch** | `wl_touch` | `QTouchEvent` | `PinchHandler` (`PointerDevice.TouchScreen`) | **DEFERRED**: Touchscreen hardware not targeted |
| **Touchpad Pinch** | `zwp_pointer_gestures_v1` | `QNativeGestureEvent` / `QPinchEvent` | `PinchHandler` (`PointerDevice.TouchPad`) | **NOT DELIVERED**: Consumed by KWin or unrouted |
| **KWin Desktop Gestures** | Internal KWin `libinput` | None (compositor internal) | N/A | **BYPASS RESTRICTED**: Intercepted before client |
| **Synthetic Test Pinch** | Direct QML Invocation | Function call | `scene.pinchUpdate(scale)` | **TEST ONLY**: Explicitly tagged `synthetic: true` |

---

## 3. Supported In-Window Spatial Behaviors

### 3.1 Two-Finger Touchpad Panning
- **Single Handler Contract**: A single `WheelHandler` on `SpatialScene` processes all axis events, ensuring that one sample is never processed twice by both angle and pixel handlers.
- **Priority**: When `pixelDelta` is non-zero, it is applied directly; `angleDelta` is ignored. When `pixelDelta` is zero (traditional discrete mouse), `angleDelta` is scaled by `wheelStepPx = 96`.
- **Clamping**: Camera position is strictly clamped to the content overflow of the active level:
  $$\text{cameraX} \in [-\text{panLimitX}, 0], \quad \text{cameraY} \in [-\text{panLimitY}, 0]$$
  If content fits within the 1440×900 viewport, `panLimitX = 0` and `panLimitY = 0`, anchoring the camera at the origin.
- **Directionality**: Vertical scroll pans vertically. Horizontal scroll, or `Shift` + vertical scroll, pans horizontally.
- **Suppression**: Scrolling produces no motion when the Search Palette or Onboarding Dialog is open.

### 3.2 Zoom / Depth Level Changes via Wheel
- `Ctrl` + Wheel performs depth transitions:
  - Scroll Up / Forward: Zooms in $\to$ `enterDeeper()` (e.g., Level 1 $\to$ Level 2 $\to$ Level 3).
  - Scroll Down / Backward: Zooms out $\to$ `returnToParent()` (e.g., Level 3 $\to$ Level 2 $\to$ Level 1).
- **Hysteresis & Cooldown**: Governed by `levelStepCooldownMs = 320`. A continuous wheel flick inside 320ms advances exactly one level, preventing runaway zooming.
- **Safety Boundaries**: Zooming out at Level 1 never closes the window. Zooming in at Level 3 never activates native windows.

---

## 4. Why Physical Touchpad Pinch is Not Claimed

1. **Wayland Client Protocol Limitations**:
   While `kwin_wayland` supports `zwp_pointer_gestures_v1`, standard Qt Quick applications without custom Wayland shell extensions or proprietary gesture protocols rarely receive raw touchpad pinch events.
2. **KWin Compositor Interception**:
   Under KDE Plasma 6.8.x, multi-finger touchpad pinch gestures are captured by KWin's internal gesture recognition engine for desktop overview and desktop grid switching. KWin does not propagate these gestures down to client surfaces.
3. **Automated Testing Failsafe**:
   Neither `libei` nor `KWin.FakeInput` exposes `zwp_pointer_gestures_v1` interfaces. Thus, private automated test harnesses cannot simulate physical touchpad pinch gestures.
4. **Honest Qualification**:
   DepthOS strictly avoids claiming hardware support based solely on synthetic in-process function calls. Synthetic test results are recorded as `synthetic: true`, while physical touchpad pinch is officially classified as **UNSUPPORTED / NOT DELIVERED**.

---

## 5. Manual Hardware Test Plan for Operators

For human operators evaluating on physical laptop hardware with a touchpad:

1. **Launch DepthOS Spatial Alpha**:
   ```bash
   ./tools/depthos-spatial-alpha --mock
   ```
2. **Test 1: Two-Finger Scroll (Pan)**
   - Place two fingers on the touchpad and slide vertically or horizontally.
   - **Expected**: The canvas smoothly pans if content overflows the screen; no erratic jumps.
3. **Test 2: Ctrl + Two-Finger Scroll (Zoom)**
   - Hold `Ctrl` and scroll forward/backward with two fingers.
   - **Expected**: View transitions exactly one level per continuous flick (Level 1 $\to$ Level 2 $\to$ Level 3).
4. **Test 3: Hardware Pinch Attempt**
   - Place two fingers on the touchpad and pinch together or expand apart.
   - **Observation**: Note whether the window zooms or if KWin triggers the desktop overview. Record findings in the hardware compatibility log.
5. **Test 4: Keyboard Fallback**
   - Press `Enter` to drill down, `Esc` to return to parent.
   - **Expected**: Immediate response with zero dependence on pointer gestures.
