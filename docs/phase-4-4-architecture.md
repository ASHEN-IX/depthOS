# Phase 4.4 Architecture: Interactive Hierarchy Management and Dual Boundary Separation

Phase 4.4 introduces full application-owned hierarchy management (CRUD, reordering, non-destructive archiving, Git project attachment, and preferences persistence) while strictly maintaining the read-only boundary for native KDE Wayland window observations.

```mermaid
flowchart TD
    subgraph UI["Qt Quick Spatial Frontend (tools/depthos-spatial-alpha)"]
        Scene["SpatialScene.qml (Neon Spatial × Cyber-Tech)"]
        Modals["Editors & Wizard Modals\n(World, Workspace, Desktop, Onboarding)"]
        Bridge["HostBridge (Python PyQt6)"]
        Scene --- Modals
        Modals --- Bridge
    end

    subgraph NativeBoundary["Read-Only Observation Plane (DENY-ONLY)"]
        KWin["KWin Wayland Compositor"]
        Observer["depthos-system-observer (main.js)"]
        Ctrl["DepthOS Controller (depthos controller run)"]
        KWin -->|Scripting API| Observer
        Observer -->|Private D-Bus Session| Ctrl
        Ctrl -.->|org.depthos.Controller1 (assignmentMutations: false)| Scene
    end

    subgraph LogicalPlane["Application-Owned Data Plane (Rust & SQLite v3)"]
        CLI["depthos CLI & manage exec Endpoint"]
        Service["depthos-core WorkspaceService"]
        Store["SqliteStore (Schema v3 Owner)"]
        DB[("workspace.sqlite\n(Mode 0600, WAL, FK Enabled)")]
        
        Bridge -->|Child Process CLI / JSON Stdin| CLI
        CLI --> Service
        Service --> Store
        Store --> DB
    end
```

## Architectural Separation of Concerns

1. **Native Observation Plane (Read-Only)**:
   - External window state (native desktop placement, window IDs, minimize/maximize state) is observed via the private KWin script `depthos-system-observer`.
   - The D-Bus endpoint `org.depthos.Controller1` exposes read-only methods (`GetCapabilities`, `GetControllerHealth`, `GetDesktopShelves`).
   - `assignmentMutations` capability property is fixed to `false`.
   - Native authorization remains **DENY-ONLY** in production. At no point does the application mutate native KWin windows, switch host virtual desktops, or modify user session configuration.

2. **Application Data Plane (Logical Management)**:
   - All management operations on organizational entities (`World`, `Workspace`, `LogicalDesktop`) and configuration (`UserPreferences`) are owned exclusively by `depthos-core::service::WorkspaceService`.
   - Persistence is managed through `SqliteStore`, the sole owner of the SQLite schema.
   - Operations from the UI invoke `depthos manage exec --request '<json>'` (or direct CLI commands) over a strict, validated JSON contract.
   - Every mutation is transactional, atomic, audited in `hierarchy_audit_log`, and guarded by optimistic concurrency versions (`version` field).
