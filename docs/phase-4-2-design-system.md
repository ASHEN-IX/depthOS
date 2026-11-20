# Phase 4.2 design system

[NeonTheme](../experiments/spatial-alpha/theme/NeonTheme.qml) is registered as a local QML singleton. [CyberCard](../experiments/spatial-alpha/CyberCard.qml), [NeonButton](../experiments/spatial-alpha/NeonButton.qml), [NeonBadge](../experiments/spatial-alpha/NeonBadge.qml), [NeonText](../experiments/spatial-alpha/NeonText.qml), and [NeonLabel](../experiments/spatial-alpha/NeonLabel.qml) share surface and type tokens.

| Token | Color | Use |
| --- | --- | --- |
| Background | `#080D17` | Obsidian scene |
| Panel | `#0D1527` | Command and control overlays |
| Surface | `#111C33` | Cards and HUD |
| Emerald | `#12E6A0` | Primary selection and synchronized observation |
| Violet | `#8B5CF6` | Spatial depth and environmental light |
| Cyan | `#22C7F2` | Search, telemetry, and environmental light |
| Text | `#E8EFFA` | Titles, labels, and main content |
| Muted | `#95A5BB` | Secondary text and context |

Cards use 16 px default radii, with more compact radii for controls and badges. Focus uses stroke thickness and a bright outline. Selection also uses elevation, scale, or a labeled state where appropriate. Layout reacts to available logical pixels, with two Workspace columns at 660 px and three at 1040 px when the dataset has enough entries.

The atmosphere is original procedural geometry, with radial lighting, perspective lines, and a small set of static points painted into one Canvas texture. Low effects disables the atmospheric canvas and decorative glow. High contrast supplies dark opaque surfaces and stronger outlines. Reduced motion snaps presentation to its target while keeping the same controls.

The approved concept PNG was absent. The visual review used the [approved visual specification](phase-4-2-visual-design-spec.md).
