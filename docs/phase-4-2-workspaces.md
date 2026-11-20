# Phase 4.2 Workspaces

[WorkspacesView](../experiments/spatial-alpha/WorkspacesView.qml) arranges the current World's Workspace cards in a responsive matrix. H/L or Left/Right move horizontally. J/K or Up/Down move by row. A click selects, a double-click or Enter opens Logical Desktops, and Escape or the breadcrumb returns to Worlds. Returning from a Logical Desktop keeps the Workspace selection.

[WorkspaceCard](../experiments/spatial-alpha/WorkspaceCard.qml) renders stored project names, descriptions, paths, and associated Desktops when the source supplies them. SQLite workspace update times and controller hierarchy read times carry different labels. Git branch appears when supplied as verified metadata or as part of the labeled demo fixture. The current persistent schema has no stored Git branch. Native window counts require observed desktop associations. The demo counts come from built-in fixture records.
