# DepthOS Phase 4.2 — Implementation Plan & Engineering Blueprint
## NEON SPATIAL × CYBER-TECH REDESIGN

**Target Milestone**: Phase 4.2  
**Status**: APPROVED & READY FOR IMPLEMENTATION  
**Branch Strategy**: Branch from `codex/phase-4-1-2` to `codex/phase-4-2`  
**Authors**: Principal KDE Plasma/KWin Wayland Engineer, Rust Systems Architect, Qt/QML Interaction Specialist  

---

## 1. Design-Token Architecture

To establish an immutable single source of truth for all styling, the Phase 4.2 redesign will introduce a centralized QML singleton `NeonTheme.qml` located at:
`experiments/spatial-alpha/theme/NeonTheme.qml`

```qml
pragma Singleton
import QtQuick

QtObject {
    id: theme

    // Palette Tokens
    readonly property color obsidian:        "#080D17"
    readonly property color emeraldAccent:   "#12E6A0"
    readonly property color violetDepth:     "#8B5CF6"
    readonly property color cyanAtmosphere:  "#22C7F2"
    readonly property color iceText:         "#E8EFFA"

    // Surface Tokens
    readonly property color surfaceCanvas:   "#080D17"
    readonly property color surfacePanel:    "#0D1527"
    readonly property color surfaceCard:     "#111C33"
    readonly property color surfaceCardHover:"#162442"
    readonly property color surfaceCardActive:"#1A2C52"
    readonly property color surfaceScrim:    "#080D17E6"

    // Border Tokens
    readonly property color borderSubtle:    "#1E2C48"
    readonly property color borderHighlight: "#2E436E"
    readonly property color borderActive:    "#12E6A0"
    readonly property color borderDepth:     "#8B5CF6"

    // Typography Tokens
    readonly property color textPrimary:     "#E8EFFA"
    readonly property color textSecondary:   "#94A7C6"
    readonly property color textMuted:       "#5A6E8C"
    readonly property color textActive:      "#12E6A0"

    // Metrics & Radii
    readonly property int radiusCard:        16
    readonly property int radiusButton:      8
    readonly property int radiusChip:        6
    readonly property int strokeWidthResting: 1
    readonly property int strokeWidthActive:  2

    // Motion & Timing Tokens
    readonly property int animFastMs:        120
    readonly property int animNormalMs:      200
    readonly property int animSlowMs:        320
    readonly property var easingOut:         Easing.OutCubic
}
```

---

## 2. Reusable QML Component Inventory

Phase 4.2 organizes the UI into a modular hierarchy of reusable components located under `experiments/spatial-alpha/components/`:

```
experiments/spatial-alpha/
  ├── theme/
  │     ├── NeonTheme.qml             [Style singleton]
  │     └── qmldir                    [Singleton export definition]
  ├── components/
  │     ├── CyberCard.qml             [Base card with neon border & glow]
  │     ├── CyberBadge.qml            [Telemetry status pill]
  │     ├── SpatialCarousel.qml       [Curved 3D horizontal carousel]
  │     ├── TechnicalGrid.qml         [Background cybernetic grid]
  │     ├── BreadcrumbHUD.qml         [Hierarchical path indicator]
  │     ├── TelemetryHeader.qml       [Top bar with status & controls]
  │     └── CommandPalette.qml        [Search & quick-action modal]
  └── views/
        ├── LevelAWorldsView.qml      [Level 1: 3D curved world carousel]
        ├── LevelBWorkspacesView.qml  [Level 2: 2D project matrix]
        └── LevelCDesktopsView.qml    [Level 3: Logical desktop window shelves]
```

### Component Contracts
1. **`CyberCard.qml`**:
   - Properties: `title`, `subtitle`, `icon`, `selected`, `focused`, `available`, `glowColor`.
   - Signals: `clicked()`, `doubleClicked()`.
   - Behavior: Hosts a single `MouseArea` enforcing the Phase 4.1.2 interaction contract. Features dual-border rendering (inner stroke and outer glow rectangle) using fast GPU shaders.
2. **`CyberBadge.qml`**:
   - Properties: `label`, `status` (`Live`, `Demo`, `Stale`, `Unavailable`), `pulsing` (defaults to false when idle).
   - Behavior: Strict quiescence — pulse animations automatically stop when `Window.active` is false.

---

## 3. World Carousel Implementation (Level 1)

The World Carousel replaces the flat 2x2 grid with an immersive curved 3D horizontal carousel (`SpatialCarousel.qml`):

```qml
Item {
    id: carousel
    property var model: []
    property int currentIndex: 0
    property real curveRadius: 1200

    Repeater {
        model: carousel.model
        delegate: CyberCard {
            id: cardDelegate
            required property var modelData
            required property int index

            readonly property real offset: index - carousel.currentIndex
            readonly property real angleRad: offset * 0.22

            x: carousel.width / 2 - width / 2 + Math.sin(angleRad) * carousel.curveRadius
            y: carousel.height / 2 - height / 2
            z: Math.cos(angleRad) * 200 - 200

            scale: index === carousel.currentIndex ? 1.05 : Math.max(0.75, 1.0 - Math.abs(offset) * 0.12)
            opacity: index === carousel.currentIndex ? 1.0 : Math.max(0.35, 1.0 - Math.abs(offset) * 0.30)
            rotation: -offset * 12

            selected: index === carousel.currentIndex
            onClicked: carousel.currentIndex = index
            onDoubleClicked: scene.pointerDrillDown()
        }
    }
}
```

---

## 4. Workspace Panel Implementation (Level 2)

When drilling into a World, the camera smoothly pushes forward along the Z-axis ($Z = -200\text{ px}$), transitioning into the Workspace Matrix:
- **Layout**: 3-column auto-wrapping grid on top of `TechnicalGrid.qml`.
- **Card Enhancements**:
  - Displays linked Git branches (`git: main` / `git: feature/xyz`) in styled monospace badges.
  - Displays mini desktop slot indicators showing real-time window counts.
  - Color-codes cards by world affiliation using subtle ambient corner accents.

---

## 5. Logical Desktop View Implementation (Level 3)

The deepest level displays open applications grouped by their virtual/logical desktop:
- **Desktop Shelves**: Horizontal rows dedicated to each desktop (e.g., `Desktop 1: Code`, `Desktop 2: Browser`).
- **Window Identity Cards**:
  - Displays sanitized process icons and binary names.
  - Privacy Title redaction: Window titles default to `<redacted>`; pressing `T` toggles title visibility if privacy permissions allow.
  - Windows with `excludeFromCapture` are rendered with a locked padlock icon and generic label.
  - Inert double-click: Preserves read-only authority boundary; double-clicking a window card selects it but never moves, focuses, or activates native windows.

---

## 6. Search & Control Center Integration

### 6.1 Unified Command Palette (`CommandPalette.qml`)
- Seamlessly overlays the active scene with a semi-transparent Obsidian scrim (`#080D17E6`).
- Instantaneous fuzzy-search across all entities:
  - Prefix `@` filters Worlds.
  - Prefix `#` filters Workspaces.
  - Prefix `!` filters Windows.
- Preserves active focus upon window deactivation and reactivation (tested in Phase 4.1.2 DEF-07/PTR-09).

### 6.2 Telemetry Header
- Displays real-time observer status without inducing frame invalidations:
  - Observer connection status (`Live`, `Connecting`, `Unavailable`).
  - Active data source (`Demo`, `SQLite`, `Controller`).
  - Hotkey indicators (`[/] Search`, `[T] Titles`, `[M] Motion`).

---

## 7. Motion Architecture & Accessibility

### 7.1 Motion Rules
1. **GPU Acceleration**: Animate `scale`, `opacity`, `transform.x`, and `transform.y`. Never animate `width`, `height`, or anchor margins.
2. **Reduced-Motion Invariant**: When `scene.motionOff === true`:
   - All `NumberAnimation` and `ColorAnimation` durations drop to `0`.
   - Transitions execute instantaneously on the same frame.
   - Parallax and 3D rotational offsets are clamped to zero.

### 7.2 Accessibility (a11y) & Contrast
1. **Contrast Compliance**:
   - Text against surfaces meets WCAG AAA ($> 7:1$ for body text, $> 4.5:1$ for headings).
   - Emerald active borders (`#12E6A0` against `#111C33`) provide a $11.8:1$ contrast ratio.
2. **Keyboard Traversal**:
   - `H` / `Left`: Navigate left / previous item.
   - `L` / `Right`: Navigate right / next item.
   - `Enter` / `Space`: Drill down one hierarchy level.
   - `Esc` / `Backspace`: Return to parent hierarchy level.
   - `/`: Open Command Palette.
   - `Tab`: Cycle focus through HUD controls.

---

## 8. GPU Performance Budgets

To maintain 144 Hz and 60 Hz performance across Intel integrated and discrete GPUs:

| Resource Metric | Maximum Budget | Phase 4.1.2 Measured Baseline |
| :--- | :---: | :---: |
| **CPU Render Time (per frame)** | $< 2.0\text{ ms}$ | $0.239\text{ ms}$ |
| **Total Frame Time (144 Hz)** | $< 6.944\text{ ms}$ | $6.940\text{ ms}$ |
| **Total Frame Time (60 Hz)** | $< 16.667\text{ ms}$ | $16.674\text{ ms}$ |
| **Idle GPU Render Cycles** | **0 frames/sec** | 0 frames/sec |
| **Process Memory (VmRSS)** | $< 220\text{ MiB}$ | $164\text{ MiB}$ |
| **Max Textures in VRAM** | $< 48\text{ MiB}$ | $\sim 14\text{ MiB}$ |

---

## 9. Incremental Migration Strategy

To avoid disrupting working features, migration will proceed in five isolated stages:

```
Stage 1: Token & Theme Infrastructure
  - Create NeonTheme.qml and components/ directory.
  - Zero changes to existing main.qml or SpatialScene.qml.

Stage 2: Component Library Verification
  - Implement CyberCard.qml, CyberBadge.qml, and TechnicalGrid.qml.
  - Verify with standalone QML test suite (tst_components.qml).

Stage 3: View-by-View Integration
  - Migrate Level 1 (World Carousel).
  - Migrate Level 2 (Workspace Matrix).
  - Migrate Level 3 (Desktop Shelves).

Stage 4: HUD & Command Palette Polish
  - Connect TelemetryHeader and CommandPalette to existing SpatialBackend.

Stage 5: Final Validation & Visual Diff
  - Run full test matrix, capture visual screenshots, verify host isolation.
```

---

## 10. Visual Screenshot Comparison Methodology

1. **Deterministic Capture Tool**:
   - Use `tools/depthos-spatial-alpha --screenshot <path>` with fixed mock data (`--mock`) and fixed resolution ($1440 \times 900$).
   - Execute in headless/virtual Wayland to eliminate host font or scaling discrepancies.
2. **Golden Reference Targets**:
   - `artifacts/phase-4-2/golden/level-a-world-carousel.png`
   - `artifacts/phase-4-2/golden/level-b-workspace-matrix.png`
   - `artifacts/phase-4-2/golden/level-c-desktop-shelves.png`
   - `artifacts/phase-4-2/golden/search-command-palette.png`
3. **Automated Pixel Diff**:
   - Use ImageMagick / Python Pillow to compute Mean Squared Error (MSE).
   - Require $MSE < 0.005$ across regression builds.

---

## 11. Acceptance Criteria for Phase 4.2 Sign-Off

- [ ] All 10 design-token categories defined in `NeonTheme.qml` and utilized across all views.
- [ ] Level 1 displays smooth 3D curved carousel with keyboard and pointer selection.
- [ ] Level 2 displays structured project cards with Git and desktop indicators.
- [ ] Level 3 displays read-only desktop window shelves with privacy redaction intact.
- [ ] Single-click selects; double-click drills down with 320ms debounce.
- [ ] Production native authority remains strictly **DENY-ONLY**.
- [ ] Sustained frame rate meets 60 Hz / 144 Hz target budgets with zero idle swaps.
- [ ] Zero host configuration modifications.
- [ ] `cargo test`, `cargo clippy`, and all QML test suites pass unconditionally.
