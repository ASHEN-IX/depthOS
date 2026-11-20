# Phase 4.2 Worlds

[WorldsView](../experiments/spatial-alpha/WorldsView.qml) lays out a centered emerald World on a curved horizontal arc. Nearby Worlds recede with smaller scale, Y-axis tilt, opacity, and vertical offset. World insignias and the atmospheric grid are procedural. Card size and spacing follow the viewport. The selected World owns the summary panel, which can show its workspace names, and its card shows Workspace and Logical Desktop counts from the hierarchy.

Use H/L or Left/Right to move between Worlds, Enter to open the selected World, and the wheel or touchpad scroll to navigate. The selected index updates on the first pointer click. The carousel keeps the clicked card in place during the platform double-click interval, then recenters if the selection is still current. Double-clicking selects and enters a World once. The selected World can also be opened from the summary panel or the search palette.

The viewport instantiates at most nine full World cards near the current carousel index. At compact heights the title, card and controls reserve separate vertical space so keyboard hints and hierarchy counts remain visible.
