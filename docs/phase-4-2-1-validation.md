# Phase 4.2.1 validation

The isolated `codex/phase-4-2-1` worktree was created from verified Phase 4.2 commit `43609632c5ecb09dedc2270cdb98d4a0f0f1a2f7`. Canonical `main` was verified at `6543b65213f24a733012ac5db50e69ed382d1cfb`, an ancestor of the baseline. Development and self-review used one agent and separate local milestone commits.

## Launch

```sh
QT_QPA_PLATFORM=wayland /home/ashenix/Projects/DepthOS-phase-4-2-1/tools/depthos-spatial-alpha --mock
```

Omit `--mock` for normal controller or local SQLite source selection. Press / for search and F2 for presentation settings. Title reveal remains an explicit action using T or Control Center.

## Regressions and evidence

| Gate | Result |
| --- | --- |
| Locked offline Rust workspace build, format and strict Clippy | Pass |
| Rust tests in a private D-Bus session | 233 passed, one opt-in benchmark ignored |
| Spatial QML lint and offscreen QtTest | 110 passed and no QML warnings |
| Private KWin 6.8.3 Wayland EIS suite | All 15 checks passed, including compositor-delivered unselected-card double-clicks, both wheel axes, Ctrl-wheel, Alt+Tab search focus, rapid transitions and compact Control Center keyboard focus |
| Private native resize and scaling | Six requested sizes and 1.25× DPR |
| Production launcher under Wayland and offscreen | Explicit first-run creation, read-only startup, real controller loss and restart, and selection preservation passed |
| Native action and controller security regressions | All 12 existing adversarial stages and the Controller1 D-Bus allowlist passed |
| Git and host isolation | Git integrity passed. Baseline checkouts remained clean. The short isolation recheck preserved both host hashes. The installed-script inventory matched. The security report records the campaign-wide checksum discrepancy. |

An early layout iteration hid the second peripheral card's center behind its neighbor. Increasing its arc displacement restored both pointer hit-testing and the synthetic pinch targeting regression. A launcher capture issue surfaced when importing the Qt image provider changed the PyQt wrapper for the root scene. The launcher now captures the typed Qt window and writes the PNG atomically. Private Wayland recaptures followed the final hero alignment and screenshot settle changes.

Raw logs are in `artifacts/phase-4-2-1/logs/`. Benchmark and private KWin drivers are saved in `artifacts/phase-4-2-1/harness/`. Before and after image sets use actual Qt Window captures.

See [visual review](phase-4-2-1-visual-review.md), [responsive layout](phase-4-2-1-responsive-layout.md), [memory](phase-4-2-1-memory.md), and [security](phase-4-2-1-security.md).

## Disposition

**PHASE 4.2.1 VERIFIED**

The reference-driven Worlds layout, responsive controls, deeper shelves, and local overlays were exercised as real Wayland windows. The matched standard peak RSS dropped 17.6%. The private session maintained roughly 60 Hz Qt swap pacing and reached zero settled-idle swaps. This qualification covers the measured private compositor and the shipped navigation contract.

Phase 4.3 priorities are a privacy-reviewed read-only Desktop metadata projection, native hardware presentation measurement at higher refresh rates, and virtualization for large Workspace and window grids. Standard-mode peak RSS remains above the historical 220 MiB optimization reference. Low effects reached 218.6 MiB on the matched run.
