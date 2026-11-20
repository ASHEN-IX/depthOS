# Phase 4.2 validation

## Checkout and launch

The implementation lives on `codex/phase-4-2` in `/home/ashenix/Projects/DepthOS-phase-4-2`.

The requested Phase 4.1.2 tip `eaf3c27181f016d93339199dbe6e477e69d7a2fa` did not resolve. The clean `codex/phase-4-1-2` checkout was at `eaf3c27d466b8a3574b9eeb98c0fac3508a4b65e`, with the visual handoff as its tip and both exact agent checkpoints as its preceding commits. Canonical `main` was verified at `6543b65213f24a733012ac5db50e69ed382d1cfb` and is an ancestor of the Phase 4.1.2 tip.

Run the demo in the current Wayland session with built-in fixture data.

```sh
QT_QPA_PLATFORM=wayland /home/ashenix/Projects/DepthOS-phase-4-2/tools/depthos-spatial-alpha --mock
```

Omit `--mock` to follow the normal controller and local SQLite source selection. Run with `--reduced-motion` for immediate depth changes. Press F2 to open the Spatial Control Center, or / for local search.

## Visual comparison

The approved concept PNG was absent. The [visual specification](phase-4-2-visual-design-spec.md) supplied the palette and composition. Running Wayland captures show an emerald hero with smaller receding World cards, a violet and cyan perspective grid, three Workspace columns at 1440×900, a separate Logical Desktop shelf, and dark cyan-edged overlays. The selected-world capture uses a centered selection with visible neighbors on each side. The default shelf uses application-identity tiles and hidden titles. Reduced motion keeps the same layout. Low effects removes the atmospheric texture and simplifies the insignias.

Actual own-window captures are stored under `artifacts/phase-4-2/screenshots/`. The observer-unavailable image came from the production launcher with a real private controller and no observer publisher. The empty image uses an initialized SQLite database with zero Worlds. Other content in the screenshots is built-in demo data.

See the [security review](phase-4-2-security-review.md) for the production boundary and the [performance measurements](phase-4-2-performance.md) for render timing and memory.

## Checkpoints

| Role | Commit |
| --- | --- |
| Launcher and controller integration | `e05fc8a382dfbba752ffe1460d4f3ead0f557b0e` |
| Agent 1 implementation | `4e877062a4ea5d0c2fc30b948fffd65f4f4287f0` |
| Agent 2 independent review and corrections | `7a4fb6ec886b89f6d287bb6df0aaee9510d03abc` |

The independent reviewer reproduced a wheel fallback reported as a 120-unit notch plus 15 pixels, and a horizontal-only scroll event that Qt delivered but the vertical WheelHandler filtered. The revised wheel area accepts both axes and chooses one delta representation per event. The reviewer also fixed compact Control Center keyboard scrolling and cleared World heading/card collisions at small window heights. Each correction has an offscreen regression and was exercised again under private Wayland.

## Test matrix

| Gate | Result |
| --- | --- |
| `cargo build --workspace --locked --offline` | Pass |
| `dbus-run-session -- cargo test --workspace --locked --offline` | 233 passed. The one opt-in synthetic controller benchmark passed in a separate run. |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Pass |
| `cargo fmt --all --check` | Pass |
| Spatial offscreen QtTest and entry/component `qmllint` | 103 passed, zero test warnings or failures. |
| [Production launcher test](../tests/spatial/test_alpha_launcher.py) | Passed offscreen and in private Wayland. Verified first-run initialization, real invalidation delivery, preservation of selection, controller loss and restart, and read-only launch. |
| KWin 6.8.3 Wayland with EIS input | 15 checks passed in both independent and integrated runs. They covered keyboard, single-click and double-click, horizontal and vertical scroll, Ctrl-wheel, search, Alt+Tab focus continuity, compact settings focus, interrupted input, and resize. |
| Responsive layouts | 1920×1080, 1440×900, 1366×768, and 900×640; compact settings and World checks at 630×470; 1366×768 logical pixels at Qt DPR 1.25. |
| Native observer lifecycle | Private ten-step load, snapshot, update, reconnect, and cleanup passed. |
| Security regressions and production allowlist | Twelve suite stages passed, including eight private native fixture cases. Production retained five read-only Controller1 methods and one signal. |
| Historical source manifest | 143 archived files verified. |
| Git integrity and host isolation | Full Git integrity passed. Host KWin and global shortcut hashes and the installed-script inventory matched. |

## Qualification

**PHASE 4.2 PARTIALLY VERIFIED**

All three presentation levels and both overlays run in the private Wayland session. The phase is qualified for the demo and the existing read-only hierarchy. Production window cards still require native-to-logical Desktop associations and a privacy-reviewed metadata projection. Peak RSS exceeded the visual implementation plan's reference budget on this machine. Compositor-delivered EIS input and Qt swap timing were measured in a virtual session. Physical touchpad pinch and hardware scanout remain outside this evidence.

Phase 4.3 priorities are a read-only Desktop display contract, memory profiling through larger hierarchies and longer sessions, and physical-device and display-timing qualification.
