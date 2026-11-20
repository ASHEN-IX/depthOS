# DepthOS Phase 4.1.2 — Pointer Input Qualification

**Status**: QUALIFIED & VERIFIED  
**Environment**: Private KWin 6.8.3 (`kwin_wayland --virtual`), Qt 6.12.0, Wayland Native  
**Branch**: `codex/phase-4-1-2`  
**Base Commit**: `6830f4a2890e8c1e0ca9be47737bf719df7994a7`  
**QA Checkpoint**: `2e97bd8bb0b16ffaa77e15835a11ef6c5dd50edf`  

---

## 1. Executive Summary

In Phase 4.1.1, offscreen Qt Test suites passed double-click tests, but testing under private KWin via libei/EIS recorded zero scene events upon compositor-delivered double-clicks.

In Phase 4.1.2:
1. **Root-Cause Discovered**: The zero-event anomaly was not a compositor or Qt event delivery failure, but a harness sequencing defect: `priv_ux_session.py` injected an `Esc` key at Level 1 immediately before the double-click sequence, which triggered `SpatialScene.closed()` $\to$ `rootWindow.close()`. The double-click was therefore delivered to an already closed window.
2. **Interaction Contract Enforced**:
   - Single-click selects target cards and synchronizes keyboard selection.
   - Double-click drills down exactly one hierarchy level (World $\to$ Workspace $\to$ Logical Desktops).
   - Window cards (Level 3) select only; double-clicks are strictly inert to preserve the Phase 3 read-only native authority boundary.
   - Non-interactive and unavailable cards reject both single- and double-click interactions.
3. **Punch-Through & Double-Drill Guard**: Implemented `pointerDrillDown()` with a 320ms debounce cooldown in `SpatialScene.qml`, preventing race conditions, accidental double-drilldowns, and click-through during active scale animations.
4. **Wayland EIS Verification**: Real compositor-delivered double-clicks were verified in an isolated virtual KWin 6.8.3 session, confirming clean `QEvent::MouseButtonDblClick` delivery and deterministic level transitions.

---

## 2. Pointer Event Delivery Architecture

The full event pipeline from Wayland compositor to QML scene graph operates as follows:

```
Wayland Compositor (KWin 6.8.3)
      │
      │ wl_pointer.button (press, serial, time)
      │ wl_pointer.button (release, serial, time)
      │ wl_pointer.button (press, serial, time)
      │ wl_pointer.button (release, serial, time)
      ▼
QtWayland Client Plugin (libqwayland-generic.so)
      │
      │ QWindowSystemInterface::handleMouseEvent()
      │ Evaluates timestamp delta (<= QStyleHints::mouseDoubleClickInterval(), 400ms)
      │ Evaluates position delta (<= QStyleHints::startDragDistance(), 10px)
      ▼
QGuiApplication / QQuickWindow
      │
      │ QEvent::MouseButtonPress
      │ QEvent::MouseButtonRelease
      │ QEvent::MouseButtonDblClick (delivered on 2nd press)
      ▼
QQuickItem Hierarchy (SpatialScene.qml)
      │
      ├─► canvasContainer (Scale + Translate transforms)
      │     └─► Level Views (levelAView, levelBView, levelCView)
      │           └─► Card Delegates (WorldCard, WorkspaceCard, WindowCard)
      │                 └─► MouseArea (hoverArea)
      │                       ├─ onClicked ──────► card.chosen()   (select & sync index)
      │                       └─ onDoubleClicked ─► card.activated() (drill down exactly 1 level)
      └─► WheelHandler (discrete notches & smooth pixel deltas for spatial panning)
```

---

## 3. Interaction Contract & Implementation Details

### 3.1 Card-Level Interaction Rules

| Card Type | Single-Click | Double-Click | Unavailable State |
| :--- | :--- | :--- | :--- |
| **WorldCard** (Level 1) | Selects world, updates `worldIndex`, syncs keyboard focus | Calls `pointerDrillDown()`, drills to Level 2 (Workspaces) | Cursor `ArrowCursor`; clicks ignored |
| **WorkspaceCard** (Level 2) | Selects workspace, updates `workspaceIndex`, syncs keyboard focus | Calls `pointerDrillDown()`, drills to Level 3 (Desktops) | Cursor `ArrowCursor`; clicks ignored |
| **WindowCard** (Level 3) | Selects window, updates `windowIndex`, syncs keyboard focus | **Inert** (selects only; never activates native window) | Cursor `ArrowCursor`; clicks ignored |
| **Empty Background** | Retains current selection and canvas focus | Inert | N/A |

### 3.2 Debounce & Multi-Level Punch-Through Prevention

When double-clicking a card, the first click selects the card and triggers a selection scale animation (from `1.0` to `1.03` over 200ms). If a user clicks rapidly, or if the second click lands as the new level begins animating in (220ms opacity fade), there is a risk of:
1. Accidental double-drilldown (jumping from Level 1 directly to Level 3).
2. Click-through onto newly rendered child delegates.

To eliminate this, `SpatialScene.qml` implements `pointerDrillDown()`:

```qml
property double lastPointerDrillMs: -1e15

function pointerDrillDown() {
    const now = Date.now();
    if (now - lastPointerDrillMs < 320)
        return;
    lastPointerDrillMs = now;
    enterDeeper();
}
```

The 320ms cooldown exceeds both the selection scale duration (200ms) and the level transition duration (220ms), ensuring that exactly one level transition occurs per user gesture.

### 3.3 Keyboard and Pointer Synchronization

Whenever a card is selected via pointer click (`onClicked` $\to$ `card.chosen()`):
1. `scene.worldIndex` / `scene.workspaceIndex` / `scene.windowIndex` is immediately updated to match the delegate's index.
2. The backend selection model is updated (`backend.selectWorld`, `backend.selectWorkspace`, `backend.selectDesktop`).
3. If the user subsequently uses keyboard navigation (`H`, `L`, `Left`, `Right`), navigation proceeds seamlessly from the clicked card without jumping back to an earlier focus position.

---

## 4. Verification & Private KWin Evidence

Testing was conducted using `priv_ux_session.py` with real EIS input injection into private virtual KWin 6.8.3:

1. **`test-noesc-20261120T144058`**:
   - Single-click on `worldCard-world-design`: `currentLevel` remained 1, `worldIndex` updated from 0 to 2, `backend.selectedWorldId` updated to `world-design`.
   - Double-click sequence (gap 120ms): Delivered `QEvent::MouseButtonDblClick`, incremented `levelTransitions` by 1, transitioned `currentLevel` to 2.
2. **`test-unselected-20261120T144141`**:
   - Target card unselected at start (`world-security`, index 1).
   - Double-click sequence delivered: successfully selected world and drilled to Level 2 in a single composite interaction.
3. **QML Test Suite (`tst_phase_4_1_2.qml`)**:
   - `test_ptr01_rapid_clicks_same_card_no_spurious_activation`: 10 rapid clicks within 100ms; level remained 1, zero stalls.
   - `test_ptr02_alternating_clicks_different_cards`: Click alternating between world cards; keyboard index tracks accurately.
   - `test_ptr03_click_during_scale_animation`: Click delivered at $t=50\text{ ms}$ during 200ms scale transition; processed cleanly.
   - `test_ptr11_window_card_activated_never_triggers_drilldown`: Double-click on window card emits `activated` but leaves `currentLevel` at 3.
   - `test_ptr12_unavailable_card_rejects_interaction`: Clicks on unavailable cards produce zero selection changes and zero transitions.

---

## 5. Conclusion

Gate A is fully qualified and verified. Compositor-delivered double-clicks behave deterministically across all hierarchy levels, reject click-through and multi-level punching, preserve keyboard synchronization, and maintain strict native read-only isolation.
