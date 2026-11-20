# DepthOS Phase 4.2 — Visual Design Specification
## NEON SPATIAL × CYBER-TECH

**Target Phase**: Phase 4.2 Implementation  
**Status**: SPECIFICATION APPROVED FOR HANDOFF  
**Visual Reference File**: `design/references/depthos-neon-spatial-cyber-tech.png` (Confirmed unavailable on disk; this document serves as the authoritative visual specification)  
**Authors**: Principal KDE Plasma/KWin Wayland Engineer, Qt/QML Interaction Specialist, Technical Lead  

---

## 1. Design Vision & Aesthetic Synthesis

The DepthOS Phase 4.2 interface embodies **NEON SPATIAL × CYBER-TECH**: a fusion of deep spatial navigation, technical cyberpunk precision, and console-inspired ergonomics. It is an original, production-grade desktop environment layer designed for KDE Plasma on Wayland.

```
┌────────────────────────────────────────────────────────────────────────┐
│                   NEON SPATIAL × CYBER-TECH PILLARS                    │
├───────────────────┬──────────────────────────┬─────────────────────────┤
│    Neon Spatial   │     Cyber-Tech Green     │    Console Usability    │
├───────────────────┼──────────────────────────┼─────────────────────────┤
│ • Immersive depth │ • Emerald active accent  │ • Prominent focus halos │
│ • Curved carousel │ • Technical data grids   │ • D-pad/keyboard flow   │
│ • Violet/cyan glow│ • Restrained telemetry   │ • Progressive disclosure│
│ • Parallax camera │ • Dark slate surfaces    │ • Persistent hints      │
└───────────────────┴──────────────────────────┴─────────────────────────┘
```

---

## 2. Color Palette & Design Tokens

### 2.1 Core Palette

| Token Name | Hex Code | Visual Role | Contrast Ratio (vs Obsidian) |
| :--- | :---: | :--- | :---: |
| **Obsidian** | `#080D17` | Canvas background, deep space backdrop | Base ($1:1$) |
| **Emerald Accent** | `#12E6A0` | Primary active selection, focused borders, live status | $13.2:1$ (WCAG AAA) |
| **Violet Depth** | `#8B5CF6` | Secondary accent, Z-axis depth illumination, hierarchy links | $5.8:1$ (WCAG AA) |
| **Cyan Atmosphere** | `#22C7F2` | Ambient glow, search highlights, telemetry headers | $10.9:1$ (WCAG AAA) |
| **Ice White** | `#E8EFFA` | Primary headings, active card titles, high-emphasis text | $16.1:1$ (WCAG AAA) |

### 2.2 Derived Systematic Tones

```
Surfaces:
  surface-canvas:      #080D17  (100% Obsidian)
  surface-panel:       #0D1527  (Base card / HUD panel)
  surface-card:        #111C33  (Resting card surface)
  surface-card-hover:  #162442  (Hovered card surface)
  surface-card-active: #1A2C52  (Selected card surface)
  surface-overlay:     #080D17E6 (90% Obsidian modal scrim)

Borders & Separators:
  border-subtle:       #1E2C48  (Resting card boundary, 1px)
  border-highlight:    #2E436E  (Hover boundary, 1.5px)
  border-emerald:      #12E6A0  (Active focused boundary, 2.5px)
  border-violet:       #8B5CF6  (Secondary selected boundary, 2px)

Typography & Text:
  text-primary:        #E8EFFA  (Ice White: titles, active text)
  text-secondary:      #94A7C6  (Cool Slate: metadata, subtitles)
  text-muted:          #5A6E8C  (Dark Slate: hints, timestamps, badges)
  text-accent:         #12E6A0  (Emerald: live indicators, active values)

Status Indicators:
  status-live:         #12E6A0  (Emerald Green — live observer)
  status-demo:         #8B5CF6  (Violet — mock / demo data)
  status-stale:        #F59E0B  (Amber — stale observer)
  status-unavailable:  #EF4444  (Crimson — observer offline / missing)
```

---

## 3. Spatial Hierarchy & Navigation Views

DepthOS structures work along a continuous 3D spatial Z-axis with three distinct navigation tiers:

```
Z = +300 (Camera Origin)
      │
      ▼
Level 1: World Carousel (Z = 0)
  └─► Large curved floating cards (Topography)
      │
      ▼ (Double-Click / Enter / Ctrl+Wheel In)
Level 2: Workspace Matrix (Z = -200)
  └─► Project cards with Git branch, linked desktops, window counts
      │
      ▼ (Double-Click / Enter / Ctrl+Wheel In)
Level 3: Logical Desktop Overview (Z = -400)
  └─► Window identity cards, observation telemetry, privacy badges
```

### 3.1 Level 1: Worlds (Curved Spatial Carousel)
- **Composition**: Floating cards arranged along an elliptical arc ($R = 1200\text{ px}$).
- **Active World**: Centered at $(X=0, Y=0, Z=0)$, scaled to $1.05\times$, highlighted with an Emerald active border (`#12E6A0`) and subtle outer glow ($8\text{ px}$ radius, $0.35$ opacity).
- **Adjacent Worlds**: Positioned symmetrically left and right, rotated by $\pm 12^\circ$ along the Y-axis, scaled to $0.88\times$, opacity $0.65$.
- **Card Geometry**: $420 \times 280\text{ px}$, corner radius $16\text{ px}$.
- **Card Content**: World icon/badge, title, descriptive summary, count pill ("3 Workspaces"), and workspace chip previews.

### 3.2 Level 2: Workspaces (Project Matrix)
- **Composition**: Structured 2-column or 3-column responsive grid on a fine cyber-grid background ($24\text{ px}$ square grid, `#1E2C48` at $0.25$ opacity).
- **Card Geometry**: $380 \times 260\text{ px}$, corner radius $14\text{ px}$.
- **Card Content**:
  - Top row: Project icon, workspace name, Git branch pill (`git: main` in `#8B5CF6`), and observer status badge.
  - Middle row: Context description (max 2 lines elided).
  - Bottom row: Linked desktop miniatures showing window occupancy dots and observer freshness timer.
- **Card Focus**: Focused workspace features an Emerald accent border with a Violet depth under-glow.

### 3.3 Level 3: Logical Desktops (Window Identity Overview)
- **Composition**: Multi-row shelf organizing open windows by their associated logical desktops.
- **Window Card Geometry**: $320 \times 220\text{ px}$, corner radius $12\text{ px}$.
- **Card Content**:
  - Application icon, application name (`appName` / `appId`).
  - Privacy Title Badge: Shows `<redacted>` or `Protected Identity` when privacy mode is active; shows window caption when `showTitles` is toggled.
  - Window Thumbnail Preview: Stylized schematic wireframe representing window layout with status tags (`Minimized`, `Fullscreen`, `Read-only`).
  - Observer Freshness Pill: Telemetry age indicator (e.g., `Observed 2s ago`).
- **Safety**: Window cards select only; double-clicking is strictly inert.

---

## 4. Global Overlays & Instrumentation Panels

### 4.1 Search Palette (Command HUD)
- **Trigger**: `/` key, or clicking the search button in the top HUD.
- **Layout**: Centered modal card ($640 \times 440\text{ px}$), background `#0D1527` with a $2\text{ px}$ Cyan border (`#22C7F2`).
- **Input Field**: Prominent search input with auto-focus, monospace query styling, and instant search results across Worlds, Workspaces, and Windows.
- **Breadcrumbs**: Hierarchical path pill (e.g., `World: Engineering > Workspace: DepthOS Core`).
- **Keyboard Navigation**: `Up`/`Down` arrows to navigate results; `Enter` to drill directly to item; `Esc` to dismiss and restore canvas focus.

### 4.2 Control Center & Telemetry Header
- **Position**: Anchored to the top edge ($64\text{ px}$ height), spanning full window width.
- **Left**: DepthOS wordmark and current hierarchical breadcrumb (`Level 1: Worlds`).
- **Center**: Spatial level indicator dots (3 dots with animated active pill).
- **Right HUD Telemetry**:
  - Data Source Badge (`Demo Data` in Violet, `Local SQLite` in Cyan, `Controller` in Emerald).
  - Observer Health (`Native Observer Live` in Emerald, `Unavailable` in Red).
  - Privacy Toggle (`Titles Hidden [T]`).
  - Motion Toggle (`Motion On/Off [M]`).

---

## 5. Motion Architecture & Performance Guidelines

### 5.1 Easing & Timing Tokens

| Animation Purpose | Duration | Easing Curve | QML Property Animation |
| :--- | :---: | :--- | :--- |
| **Card Focus / Hover** | 160 ms | `Easing.OutCubic` | `scale`, `border.color` |
| **Level Transition (Zoom)**| 240 ms | `Easing.OutQuad` | `cameraZ`, `opacity`, `transform` |
| **Carousel Slide** | 220 ms | `Easing.OutCubic` | `cameraX`, `rotationY` |
| **Modal Scrim Fade** | 180 ms | `Easing.Linear` | `opacity` |
| **Reduced-Motion Mode** | **0 ms** | Instant Snap | Direct property assignment |

### 5.2 GPU Optimization Principles
1. **Transform & Opacity Only**: All continuous animations must animate `x`, `y`, `scale`, `opacity`, or `rotation`. Never animate `width`, `height`, or margin anchors per-frame.
2. **Layer Caching**: Static background grids and complex vector icons must use `layer.enabled: true` and `layer.smooth: true` to avoid repeated tessellation.
3. **No Synchronous Disk/IPC in Render Loop**: Database reads, D-Bus queries, and filesystem walks must execute asynchronously in worker threads or Rust daemons.
4. **Quiescence**: Frame animations must shut down completely when the scene is static or when the window loses active focus.
