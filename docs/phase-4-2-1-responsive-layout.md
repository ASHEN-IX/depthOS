# Phase 4.2.1 responsive layout

| Wayland viewport | Worlds layout |
| --- | --- |
| 1920×1080 | Wide rail, full deck, hierarchy totals, and Workspace previews |
| 1440×900 | Wide rail and carousel with two peripheral depths |
| 1366×768 | Wide rail, a smaller hero, and a full preview row |
| 1024×768 | Header navigation, one peripheral depth, and reflowed preview row |
| 800×600 | One peripheral depth and compact selected-World information |
| 630×470 | Single full-width hero area, paging, selected-World summary, and an explicit open action |

The rail appears at 1160 px with adequate height. Carousel depth is calculated from the width after the rail is reserved. Under 740 px of deck width, the view gives the selected card the entire deck. The folded summary keeps the selected name, counts, and open action. Search and status remain in the header.

The Workspace matrix computes one, two, or three columns from its content width. Desktops use responsive selector cards. The dense levels retain scrollable content bounds for small viewports. Control Center follows keyboard focus inside its scroll viewport, including at 630×470.

The six requested sizes were exercised by the Wayland resize test and captured from the actual Qt window. QtTest additionally checks that the hero, paging and open controls fit each viewport and that keyboard selection still moves after resize. A separate 1366×768 logical-pixel run captured a 1708×960 image at DPR 1.25.

Captures are in `artifacts/phase-4-2-1/screenshots/`. See [validation](phase-4-2-1-validation.md) for reproduction and evidence.
