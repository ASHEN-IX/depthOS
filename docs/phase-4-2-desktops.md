# Phase 4.2 Logical Desktops

[DesktopsView](../experiments/spatial-alpha/DesktopsView.qml) presents a desktop selector and a shelf of metadata cards. Logical Desktop labels and roles are hierarchy data. A native association, time, application identity, focus, minimized state, or fullscreen state appears only when supplied by the source. Selecting a Desktop or window tile updates the DepthOS selection. Window double-clicks have no native effect.

[WindowCard](../experiments/spatial-alpha/WindowCard.qml) shows a schematic application tile. It uses a generic identity for protected windows. The T shortcut or the Control Center reveals titles when the supplied record permits it. Text renders in plain-text mode after terminal controls and BiDi formatting controls are stripped. Unavailable and protected captions retain their placeholders after reveal.

The demo populates the shelf with labeled sample data. SQLite contains the logical hierarchy. Production Controller1 reports liveness and policy associations at Workspace scope, and its privacy-bounded projection omits window display metadata. A current observer can therefore coexist with an unavailable per-desktop shelf. A production native-to-logical Desktop mapping and a privacy-reviewed display projection are possible Phase 4.3 work.
