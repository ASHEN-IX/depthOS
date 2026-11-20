# Phase 4.2.1 Worlds

[WorldsView](../experiments/spatial-alpha/WorldsView.qml) stages a centered hero above luminous orbital bands. The nearest card tilts around its Y axis. Outer cards fall back in scale, opacity, and Z order. Paging and the open action have dedicated space below the deck. The selected card may also be opened through its chevron on wider layouts. Ordinary card clicks still select. The existing pointer hold keeps the hit target stationary during a compositor double-click.

The selected-World panel shows actual hierarchy counts and at most four Workspace previews from the current World. Choosing a preview selects that Workspace and opens the matrix. Counts and labels come from the backend. The wide navigation rail uses the existing Levels, Search, and Control Center actions.

## Artwork

The presentation accepts optional `artCategory` metadata from the UI model. Supported categories are `projects`, `research`, `social`, and `focus`. The demo specifies all four explicitly. Without category metadata, the view assigns a palette position by display order. The persistent and controller hierarchy schemas have no artwork fields.

Each category has a bundled 640×320 JPEG in [art](../experiments/spatial-alpha/art). These are original images generated with the built-in imagegen tool. The prompt called for a borderless 2×2 asset atlas with a violet engineering city, an emerald mountain river basin, a warm coastal sunset city, and a cool glass observatory. It specified cinematic concept art, dark navy shadows, and artwork free of typography, interface, logos, and borders. The four panels were cropped and downsampled for the app. The supplied UI image served as the visual benchmark for the implementation.

The exact artwork prompt appears below.

> Generate a single cohesive asset atlas for an original Linux spatial workspace UI, exact 2 by 2 grid of four edge-to-edge cinematic landscape illustrations in equal quadrants with clean straight division at exact center, no margins and no visible dividers. No typography, no interface, no icons, no labels, no borders, no branding. Top left: violet futuristic engineering city at blue hour with vivid magenta light on close left buildings and distant skyscrapers; top right: astonishing deep emerald mountain basin and a glowing green winding river under bright cloud breaks and a faint planet arc; bottom left: friendly coastal city and warm orange sunset mixing with mauve silhouettes and water reflections; bottom right: tranquil cool blue glass and concrete observatory, with a single windswept tree and snowy peaks across broad windows. Realistic polished cinematic environment concept art, dark navy shadows, refined lighting, distinct subjects readable at thumbnail sizes. The atlas will be physically sliced into four wide pieces for a Qt application.

A launch-time category override is available on the existing launcher.

```sh
QT_QPA_PLATFORM=wayland /home/ashenix/Projects/DepthOS-phase-4-2-1/tools/depthos-spatial-alpha --mock \
  --world-art "research=/absolute/path/research.jpg"
```

Overrides take one local PNG, JPEG, or WebP per category. Maximum compressed size is 8 MiB. Decode bounds are 4,096 pixels per axis and 4,194,304 pixels total. Textures fit within 640×384. The launcher validates and decodes once. The QML loader can only request registered categories. A failed packaged image uses the category glyph. Low-effects and high-contrast modes use glyphs and omit the decoration.

The [atmosphere generator](../experiments/spatial-alpha/art/generate-atmosphere.py) bakes original procedural horizon, orbital, and preview art to 8-bit textures using ImageMagick. It is a development tool. The launcher uses the checked-in assets.
