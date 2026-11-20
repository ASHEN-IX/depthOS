# DepthOS reference match

The supplied reference and final live Wayland screenshot are both 1672×941 pixels. The wide shell shares the composition across demo and persistent sources. Responsive controls remain available at smaller sizes.

## Run the reference catalog

```sh
QT_QPA_PLATFORM=wayland /home/ashenix/Projects/DepthOS-phase-4-2-1/tools/depthos-spatial-alpha \
  --reference-demo --window-size 1672x941
```

The reference catalog opens at Research and populates the four sample Worlds, 12 Workspaces, and 36 Desktops. The counts are computed from those records. The clock uses the local time. The status badge shows the actual source and observer state. Footer controls use working keyboard hints. The reference catalog is explicitly marked as demo data.

## Screenshot comparison

| Render | Result |
| --- | --- |
| First | Moved the title, status metrics, rail, selected World and workspace panel into the reference composition. The side cards were too wide and the preview art was flat. |
| Second | Brought in closer scene artwork, narrowed and tilted side cards, and separated navigation groups. The projected side-card typography needed its natural proportions back. |
| Third | Set side-card widths directly and added the four preview illustrations. The Focus icon was partially obscured and the summary image had a hard edge. |
| Final | Cleared the Focus identity, feathered the summary art, and added functioning preview menus. The hero, orbit, and workspace panel meet at the same proportions as the supplied concept. Art details, live labels, and breadcrumbs can be seen in the side-by-side capture. |

The production launcher generated the four reference renders under a private virtual KWin Wayland socket. The side-by-side image and iteration contact sheet are in `artifacts/reference-match/`. The `screenshots/` directory includes additional real Wayland captures of Search, Settings, Workspaces, Desktops, and the Workspace options menu.

## Controls

- Top and side hierarchy controls navigate through available Worlds, Workspaces, and Desktops.
- The top Search icon, side Search button, and footer Search action open the existing searchable hierarchy.
- The Settings gear, top bell, and footer Menu action open the Spatial Control Center.
- Paging dots select Worlds. The hero chevron and View All open Workspaces.
- Workspace previews select the workspace. Each ellipsis menu can open the Workspace or its Logical Desktops.

All actions use the existing application scene navigation. The reference layout preserves demo labeling and leaves the production D-Bus and native observer contracts intact. The live clock can cause a bounded redraw as the minute changes.

## Asset provenance

The original reference is kept with the local comparison artifacts. The bundled art uses three built-in imagegen outputs. The atlas images were cropped and downsampled for the runtime. The prompt set follows.

1. Recreate the reference's dark planetary limb and orbital lighting at matching positions, with the cyan and teal horizon, violet accents, two elliptical stages, and bottom dot waves. Omit the cards, panels, glyphs, branding, text, and all other UI.
2. Make a borderless, exact 2×2 atlas of the four World artworks from the reference. Use a violet skyscraper district for Projects, an emerald rocky valley and river under a planet arc for Research, a purple-orange waterfront city for Social, and a cobalt moonlit glass room with a bonsai for Focus. Preserve the composition and darkness of the reference and omit all text and framing.
3. Make a borderless, exact 2×2 atlas of the workspace motifs. Use a luminous teal and violet mesh wave, stacked translucent cyan-edged plates, linked blue and teal glass spheres, and an abstract luminous teal equalizer with no axes or data values. Use the reference framing and dark surfaces with no UI.

The [Worlds implementation](../experiments/spatial-alpha/WorldsView.qml), [wide header](../experiments/spatial-alpha/ReferenceHeader.qml), [side rail](../experiments/spatial-alpha/SpatialRail.qml), and [summary panel](../experiments/spatial-alpha/WorldSummary.qml) render and handle the controls independently of those images.

## Validation

The QtTest suite passed 111 checks. The private Wayland regression run passed 15 checks, including single-click selection, real double-click drill-down, scrolling, focus restoration, interruption, resized layouts, compact settings focus, and 1.25× scaling. A separate native click sequence at 1672×941 opened Search and Settings through both entry points, navigated the top and side levels, and used the preview options menu to open Logical Desktops. The production launcher also passed first-run and private controller restart checks.

The wide clock advances once per minute. In a five-second native sample between minute changes, the settled UI produced zero swaps. The private launcher recheck preserved the two host configuration checksums. An earlier, wider wall-clock checksum window also covered changes to the active host KWin configuration. Raw evidence is retained under `artifacts/reference-match/logs/`.
