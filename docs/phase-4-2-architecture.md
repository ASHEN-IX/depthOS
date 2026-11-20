# Phase 4.2 architecture

DepthOS runs in a resizable Qt Quick window on Wayland. The hierarchy belongs to DepthOS. Navigating Worlds, Workspaces, and Logical Desktops changes selection inside the client.

| Layer | Entry point | Responsibility |
| --- | --- | --- |
| Logical model | [Rust domain](../src/domain.rs), [SQLite store](../src/sqlite/mod.rs) | Store the hierarchy and metadata, and support explicit initialization |
| Read boundary | [Controller1](../src/spatial/mod.rs) | Publish bounded hierarchy and health snapshots over the session bus |
| Launcher | [depthos-spatial-alpha](../tools/depthos-spatial-alpha) | Choose the source, apply provenance, and marshal controller reads into the UI |
| Client state | [SpatialBackend](../experiments/spatial-alpha/SpatialBackend.qml), [SpatialScene](../experiments/spatial-alpha/SpatialScene.qml) | Own the current context, input, search, presentation settings, and navigation |
| Scene | [Worlds](../experiments/spatial-alpha/WorldsView.qml), [Workspaces](../experiments/spatial-alpha/WorkspacesView.qml), [Desktops](../experiments/spatial-alpha/DesktopsView.qml) | Render the three levels and forward selection intents |
| Overlays | [Search palette](../experiments/spatial-alpha/SearchPalette.qml), [Control Center](../experiments/spatial-alpha/ControlCenter.qml) | Offer local search, show data provenance, and control client presentation |

An explicit `--mock` opens the built-in hierarchy. An explicit `--db PATH` reads that SQLite file. With neither flag, the launcher tries the controller then the default SQLite path. A missing hierarchy offers first-run initialization or demo. Initializing the logical database requires the in-app Create action or the CLI's `init` command.

The launcher binds a bounded read to the controller's unique bus owner. One worker fetches the versioned hierarchy and health. Invalidation requests coalesce while a read is in flight. When the owner disappears, the UI keeps the last hierarchy with unavailable connection and observation status. When the owner returns, the launcher refreshes and preserves selection by stable ID where possible. Health changes can update presentation without rebuilding an unchanged hierarchy.

The command palette scores and highlights local hierarchy matches as the displayed lists change. Search accepts `@` for Worlds, `#` for Workspaces, and `!` for Logical Desktops. Opening a result selects its exact hierarchy path. Window titles remain outside the search index.

The controller's current context projection exposes policy associations at Workspace scope. The UI uses source and observer health independently. The [desktop view contract](phase-4-2-desktops.md) describes which fields can be shown on a Logical Desktop shelf.
