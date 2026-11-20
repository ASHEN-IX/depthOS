# Phase 4.4 Hierarchy CRUD and UI Operations

DepthOS Phase 4.4 provides a complete interactive management experience within its Neon Spatial × Cyber-Tech interface, allowing users to build, customize, and maintain their personal workspace tree without writing SQL or manual configuration.

## Organization Hierarchy

The DepthOS hierarchy consists of three application-owned levels:

```
World (Contextual Domain, e.g., "Engineering", "Research")
  └── Workspace (Project / Initiative, e.g., "DepthOS Core", "Kernel Subsystem")
        └── Logical Desktop (Task Context, e.g., "Compiler", "Review", "Terminal")
              └── Observed Window Associations (Read-only native window state)
```

## Management Modals & Workflows

### 1. World Management
- **Creation & Editing**: Accessed via `+ New World` in the Worlds carousel or the World contextual menu (`WorldEditorDialog.qml`).
- **Visual Identity**: Users select tone accents (Cyan, Lavender, Emerald, Amber, Rose, Precision) and artwork motifs ("Neon Grid", "Cyber Matrix", "Orbital", "Deep Space").
- **Navigation Stability**: Adding or updating a World preserves the currently selected World index upon reload.

### 2. Workspace Management
- **Creation & Editing**: Opened via `+ New Workspace` in the Workspace matrix (`WorkspaceEditorDialog.qml`).
- **Project Root**: Integrated directory path picker (`ProjectRootPicker.qml`) with real-time non-interactive Git metadata inspection.
- **Parent Binding**: Workspaces are strictly bound to their parent `WorldId` with foreign-key referential integrity.

### 3. Logical Desktop Management
- **Creation & Editing**: Opened via `+ New Desktop` in the Desktops view (`DesktopEditorDialog.qml`).
- **Cyber-Tech Role Presets**: One-click quick assignment to predefined technical roles: `Code`, `Review`, `Test`, `Terminal`, `Docs`, `Ops`.

### 4. Non-Destructive Archiving and Restoration
- **Impact Assessment**: When archiving an item, `ArchiveConfirmationDialog.qml` queries the backend to calculate the number of affected child entities and active association rules.
- **Safe Archival**: Instead of permanent deletion, records are marked `is_archived = 1`. This preserves all durable window association rules, preventing broken references.
- **Archived Items View**: The drawer (`ArchivedItemsView.qml`) categorizes archived items by type (World, Workspace, Desktop) and allows instant restoration with one click.

### 5. Optimistic Concurrency Conflict Resolution
- When two clients or sessions attempt to modify a record simultaneously, the backend returns a `conflict` response.
- The UI surfaces a non-intrusive toast notification (`OperationStatusIndicator.qml`) indicating:
  `"Conflict: Item was modified concurrently. [Refresh]"`
- Clicking **Refresh** reloads the latest state without corrupting the database.
