# Phase 4 Readiness: Spatial Workspace Manager Alpha Roadmap

## 1. Vision & Alpha Scope

Phase 4 transitions DepthOS from low-level action security and IPC foundations into the user-facing **Spatial Workspace Manager Alpha**.

The goal of Phase 4 is to provide a fluid, keyboard-driven, spatial window and workspace organization environment layered cleanly over native Wayland compositors (starting with KWin 6), without requiring unsafe window mutation permissions and without diverging from the security guarantees established in Phase 3.

---

## 2. Spatial Hierarchy Model: Worlds -> Workspaces -> Logical Desktops

DepthOS models user tasks as a clear 3-tier hierarchy:

```
+-----------------------------------------------------------------------------------+
|                                      WORLD                                        |
|  - High-level domain of life / focus (e.g., "Engineering", "Research", "Personal")|
|  - Encapsulates isolated project scopes, environmental variables, and security tags|
+-----------------------------------------------------------------------------------+
                                          |
                                          v
+-----------------------------------------------------------------------------------+
|                                    WORKSPACE                                      |
|  - Focused context or topic (e.g., "DepthOS-Kernel", "Vulnerability-Audit")       |
|  - Has a dedicated spatial coordinate canvas and context preview metadata         |
+-----------------------------------------------------------------------------------+
                                          |
                                          v
+-----------------------------------------------------------------------------------+
|                                 LOGICAL DESKTOP                                   |
|  - Atomic spatial viewport containing organized window clusters                   |
|  - Mapped 1:1 or 1:N to native virtual desktops (KWin::VirtualDesktop)            |
+-----------------------------------------------------------------------------------+
```

### Architectural Principles:
1. **Single Source of Truth**:
   The logical workspace hierarchy is managed strictly in DepthOS Core (`depthos_core::domain::workspace` backed by SQLite).
   There are **NO duplicate workspace models** in the UI layer, QML, or external daemon scripts.
2. **Read-Only Native Window Observation**:
   The spatial manager observes native window positions, focus, and state strictly through the authenticated observer replica (`depthos_observer`). Native window setters are never called without explicit user-driven interaction.

---

## 3. Core Alpha Features & Deliverables

### A. Animated Spatial Transitions
- **Continuous 2D Grid / Panning**: Logical desktops are arranged on an interactive 2D spatial canvas. Switching workspaces or desktops animates smooth viewport panning and parallax scaling.
- **Spring Physics & Motion Tokens**: Natural easing curves with responsive gesture interrupts (Wayland touchpad gestures and keyboard shortcuts).
- **GPU-Accelerated QML / QtQuick SceneGraph**: Lightweight scene items rendering live thumbnail previews of windows and context state.

### B. Keyboard-First Navigation
- **Hierarchical Navigation**:
  - `Super + H / J / K / L`: Directional spatial navigation across adjacent logical desktops.
  - `Super + 1..9`: Direct jump to indexed workspace in active world.
  - `Super + Tab`: Quick workspace switcher with context overview.
  - `Super + Shift + H / J / K / L`: Move focus or propose window migration.
- **Command Palette (`Super + Space`)**:
  - Instant fuzzy search over Worlds, Workspaces, Open Windows, and Action Security intents.

### C. Context Previews
- **Semantic Workspace Card**: When zooming out to Workspace Overview mode, each workspace renders:
  - Live window layout snapshot (cached from observer snapshots).
  - Associated git branch, open files, and terminal activity indicators.
  - Spatial neighbor indicators.

### D. Integration with Action Security Boundary
- Proposing to move a window across desktops or workspaces generates a `NativeMoveProposal` verified against the durable coordinator (`CrossProcessCoordinator`).
- In Alpha, operations maintain the deny-only invariant for background processes, while user-invoked spatial migrations route through the verified UI interaction boundary.

---

## 4. Phase 4 Implementation Milestones

```
+-----------------------------------------------------------------------------------+
| Milestone 4.1: Spatial Domain & Canvas Model                                      |
| - Connect depthos_core workspace database to QML SpatialCanvas model              |
| - Zero duplicate models: QAbstractListModel projection directly from SQLite       |
+-----------------------------------------------------------------------------------+
                                          |
                                          v
+-----------------------------------------------------------------------------------+
| Milestone 4.2: QML Viewport & Animation Engine                                    |
| - Implement 2D infinite canvas with smooth spring physics                         |
| - Render workspace cards with live thumbnail projections                          |
+-----------------------------------------------------------------------------------+
                                          |
                                          v
+-----------------------------------------------------------------------------------+
| Milestone 4.3: Wayland Global Shortcut & Input Layer                              |
| - Integrate org_kde_kglobalaccel / wayland input grab for Super key navigation   |
| - Directional navigation state machine with boundary clamping                     |
+-----------------------------------------------------------------------------------+
                                          |
                                          v
+-----------------------------------------------------------------------------------+
| Milestone 4.4: Action Security Wire-Up                                            |
| - Wire user drag-and-drop and keyboard migration to DurableCoordinatorSession    |
| - Test recovery, quarantine display, and reconciliation indicators in UI          |
+-----------------------------------------------------------------------------------+
```

---

## 5. Security & Stability Readiness Criteria
- **Zero Raw SQL in UI**: All workspace operations pass through typed Rust methods in `depthos_core`.
- **Observer Isolation**: UI components read observer state through immutable snapshot references.
- **Fail-Safe Viewport**: If native compositor restarts, the spatial manager smoothly transitions to a reconnecting overlay without crashing or losing logical workspace state.
