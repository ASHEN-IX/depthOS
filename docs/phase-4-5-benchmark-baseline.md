# DepthOS Phase 4.5 — Benchmark Baseline & System Profile

## 1. Baseline Environment Specification

| Parameter | Specification |
|---|---|
| **Git Commit Baseline** | `3f3710b821128e56e727fedf0e4e8875eeb2796a` (Phase 4.4 checkpoint) |
| **Rust Toolchain** | `rustc 1.100.0 (0ef1bc386 2026-11-09) (Arch Linux rust 1:1.100.0-1)` |
| **Cargo** | `cargo 1.100.0` |
| **Qt / PyQt** | `Qt 6.12.0 / PyQt6 6.12.1` |
| **Compositor / KWin** | `kwin 6.8.3` (Wayland native session) |
| **Linux Kernel** | `7.3.4-1-garuda` (x86_64) |
| **Graphics Hardware** | Intel Alder Lake-P GT2 [Iris Xe Graphics] (`i915`) |
| **OpenGL / Mesa** | Mesa 26.2.7-arch1.1 (OpenGL 4.6 Core Profile) |
| **Display Panel** | `eDP-2` physical panel: 1920×1080 @ 144.00 Hz |
| **Rendering Backend** | Qt Quick Scene Graph hardware-accelerated (`QSG_RHI_BACKEND=opengl`) |
| **Database Engine** | SQLite 3.49+ (bundled rusqlite, WAL mode, synchronous=NORMAL, schema v3) |

---

## 2. Deterministic Benchmark Fixture Dimensions

All fixtures are synthesized deterministically using [`tests/support/fixture_generator.py`](file:///home/ashenix/Projects/DepthOS-phase-4-5/tests/support/fixture_generator.py) under a dedicated private directory (`mode 0700` required by DepthOS security policy):

| Scenario | Worlds | Workspaces | Desktops | Synthetic Windows | Generation Time | SQLite DB Size |
|---|---|---|---|---|---|---|
| **Small** | 4 | 12 | 36 | 10 | 2.4 ms | 104 KiB (106,496 B) |
| **Medium** | 20 | 100 | 300 | 50 | 2.9 ms | 228 KiB (233,472 B) |
| **Large** | 100 | 500 | 1,500 | 200 | 17.2 ms | 712 KiB (729,088 B) |
| **Stress** | 250 | 1,250 | 5,000 | 1,000 | 33.8 ms | 2,000 KiB (2,048,000 B) |

---

## 3. Baseline Measurements (Unoptimized Phase 4.4 Engine)

### 3.1 SQLite Query Latencies (Rust Core)

| Metric | Small | Medium | Large | Stress |
|---|---|---|---|---|
| **Tree Query Latency (`depthos tree`) Median** | 4.22 ms | 9.28 ms | 33.28 ms | 93.93 ms |
| **Tree Query Latency Min / Max** | 3.29 / 4.30 ms | 8.87 / 10.47 ms | 31.85 / 36.04 ms | 91.52 / 99.10 ms |
| **Workspace List Latency** | 3.03 ms | 3.32 ms | 3.03 ms | 3.12 ms |

### 3.2 Application Management Latencies (`depthos manage exec`)

| Operation | Small | Medium | Large | Stress |
|---|---|---|---|---|
| `createWorld` | 2.54 ms | 3.55 ms | 2.19 ms | 3.54 ms |
| `createWorkspace` | 3.47 ms | 3.36 ms | 2.48 ms | 3.16 ms |
| `createDesktop` | 3.35 ms | 3.41 ms | 3.05 ms | 3.30 ms |
| `reorderDesktops` | 3.37 ms | 3.32 ms | 2.80 ms | 2.90 ms |
| `archiveDesktop` | 2.77 ms | 2.68 ms | 2.80 ms | 2.66 ms |
| `restoreDesktop` | 3.45 ms | 3.60 ms | 2.25 ms | 3.16 ms |
| `inspectProject` | 3.08 ms | 3.12 ms | 1.87 ms | 2.77 ms |

*Observation*: All single-entity transactional CRUD mutations via SQLite v3 WAL execute stably in 2.0 to 3.6 ms regardless of database catalog size. Indexed lookups on foreign keys and primary keys keep transactional write overhead essentially $O(1)$.

### 3.3 UI Lifecycle & Memory Footprint

Measured at initial load and warm-up under `depthos-spatial-alpha`:

| Metric | Small | Medium | Large | Stress |
|---|---|---|---|---|
| **Resident Set Size (RSS)** | 131,100 KiB (~128.0 MiB) | 133,776 KiB (~130.6 MiB) | 144,144 KiB (~140.8 MiB) | 171,728 KiB (~167.7 MiB) |
| **Proportional Set Size (PSS)** | 77,156 KiB (~75.3 MiB) | 79,870 KiB (~78.0 MiB) | 90,121 KiB (~88.0 MiB) | 117,593 KiB (~114.8 MiB) |
| **Private Dirty Memory** | 62,256 KiB (~60.8 MiB) | 64,676 KiB (~63.2 MiB) | 74,848 KiB (~73.1 MiB) | 102,408 KiB (~100.0 MiB) |
| **Peak Virtual Memory (VmHWM)** | 131,100 KiB | 133,776 KiB | 144,144 KiB | 171,728 KiB |
| **Wall Startup Latency** | 1,439 ms | 1,431 ms | 1,480 ms | 1,618 ms |

### 3.4 Frame Rendering & Pacing Baseline

Measured over initial scene instantiation and active animation frames:

| Metric | Small | Medium | Large | Stress |
|---|---|---|---|---|
| **Total Measured Swaps (Initial)** | 17 | 17 | 17 | 17 |
| **Median Swap Interval** | 15.92 ms | 16.03 ms | 16.13 ms | 15.89 ms |
| **P95 Swap Interval** | 20.65 ms | 20.82 ms | 21.29 ms | 20.39 ms |
| **Median Render Time** | 7.99 ms | 8.12 ms | 8.22 ms | 7.96 ms |
| **P95 Render Time** | 9.21 ms | 9.46 ms | 9.10 ms | 8.55 ms |

---

## 4. Key Performance Bottlenecks Identified

1. **Full-Tree Tree Parsing Overhead at Large Catalog Scales**:
   `depthos tree --json` takes ~93.9 ms on 5,000 desktops because it loads and serializes the entire recursive hierarchy in a single multi-table join and nested JSON structure. While 94 ms is acceptable for initial CLI dumps, QML view transitions should query incremental child subsets or cache tree structures to avoid blocking.
2. **QML Element Instantiation in Unvirtualized Views**:
   - `WorldsView.qml`: `Repeater` over all worlds in the catalog instantiates card components for every world. For 250 worlds, 250 items are created and bound.
   - `WorkspacesView.qml`: `Repeater` creates cards for all workspaces in the current world. If a world contains 100+ workspaces, 100 card items are created and rendered.
   - `DesktopsView.qml`: `Flow` with `selector.slots` divides width by desktop count; with >30 desktops, tabs shrink below usable tap width. Window cards also instantiate via unvirtualized `Repeater`.
3. **Synchronous Subprocess Calls in `HostBridge.executeManagement`**:
   `tools/depthos-spatial-alpha` executes `subprocess.run(["depthos", "manage", "exec", ...])` directly on the main/UI thread. Although 2–3 ms operations do not cause noticeable stutter, any heavier operation or disk latency risks dropping frames on 144Hz displays (where frame budget is 6.94 ms).
4. **Dialog Memory Retention**:
   Management dialogs in `SpatialScene.qml` (`CreateWorldDialog`, `CreateWorkspaceDialog`, `CreateDesktopDialog`, `WorkspaceEditorDialog`, `DesktopEditorDialog`, `ArchiveConfirmationDialog`, `PreferencesDialog`) are permanently created in the scene graph with `visible: false` rather than lazily loaded via `Loader`.
