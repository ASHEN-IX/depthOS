# Phase 4.2 performance

Qt 6.12.0 and OpenGL ran in a private KWin 6.8.3 virtual Wayland session at roughly 60 Hz. The measured app viewport was 1440×900 logical pixels at DPR 1. The native input suite drove compositor-delivered EIS events. A separate benchmark drove an instrumented Qt window using direct Qt key events and local scene methods.

Swap timestamps are from direct Qt `frameSwapped` emissions. Render elapsed and thread CPU below measure the interval between the render hooks. Hardware presentation timestamps, higher-refresh scanout, and per-process GPU memory were unavailable in this setup.

## Frame pacing

| Sample, in ms | Count | Median | p95 | p99 | Max |
| --- | ---: | ---: | ---: | ---: | ---: |
| Integrated EIS swap intervals | 365 | 16.678 | 16.896 | 17.081 | 18.435 |
| Independent EIS swap intervals | 364 | 16.688 | 16.896 | 17.064 | 18.489 |
| Integrated EIS render-hook elapsed | 366 | 0.388 | 0.620 | 0.787 | 1.864 |
| Instrumented standard swap intervals | 380 | 16.676 | 16.871 | 16.947 | 18.515 |
| Instrumented standard render thread CPU | 381 | 0.468 | 0.586 | 0.716 | 0.869 |
| Instrumented low-effects swap intervals | 380 | 16.686 | 16.873 | 16.973 | 18.308 |
| Matched Phase 4.1.2 swap intervals | 378 | 16.684 | 16.901 | 17.079 | 18.328 |

The integrated and independent EIS traces each recorded 0 swaps over five seconds of settled idle. Both had zero intervals above 25 ms. The instrumented standard and low-effects traces had zero intervals above 33.333 ms. The real launcher reported 17 swaps in its six-second idle run, all in the initial render burst.

## Startup, navigation, and search

| Sample, in ms | Count | Median | p95 | p99 | Max |
| --- | ---: | ---: | ---: | ---: | ---: |
| Process start to first Qt swap | 5 | 281.153 | 713.801 | 713.801 | 713.801 |
| Process start to first verified Qt key interaction | 5 | 331.091 | 728.501 | 728.501 | 728.501 |
| World to Workspace dispatch | 8 | 0.593 | 0.775 | 0.775 | 0.775 |
| World to Workspace last swap in 360 ms window | 8 | 248.251 | 256.715 | 256.715 | 256.715 |
| Workspace to Logical Desktop dispatch | 8 | 0.508 | 0.578 | 0.578 | 0.578 |
| Workspace to Logical Desktop last swap in 360 ms window | 8 | 250.660 | 258.042 | 258.042 | 258.042 |
| Local hierarchy filter | 120 | 0.026 | 0.071 | 0.578 | 0.662 |

The independent EIS harness measured 342.1 ms from QML load to first swap. The integrated EIS harness measured 350.1 ms. Startup quantiles describe five launches and should be read with that sample size. Direct Qt events replaced a startup probe that timed out inside QtTest key injection while Python render hooks were installed. The real EIS interaction suite passed independently of that probe.

## CPU and memory

| Profile | Peak RSS, MiB | Highest sampled PSS, MiB | Navigation CPU | Idle CPU | Idle swaps over 4 s |
| --- | ---: | ---: | ---: | ---: | ---: |
| Matched Phase 4.1.2 | 255.6 | 152.9 | 6.31% | 0.002% | 0 |
| Phase 4.2 standard | 294.5 | 180.1 | 8.66% | 0.003% | 0 |
| Phase 4.2 low effects | 285.7 | 175.5 | 8.74% | 0.002% | 0 |
| Phase 4.2 production launcher, Wayland idle | 290.2 | 175.6 | — | — | — |
| Phase 4.2 production launcher, offscreen idle | 106.7 | 53.8 | — | — | — |

CPU percentages are process CPU time divided by elapsed time, where 100% represents one core. PSS is sampled. Both presentation modes exceeded the plan's 220 MiB RSS reference. Low effects reduced the measured footprint modestly. A separate short soak exercised 180 complete hierarchy round trips and palette openings. After the first 30 cycles, six samples stayed within 292.1–292.5 MiB RSS and 177.6–178.0 MiB PSS. This covers the exercised workload. Longer sessions and larger hierarchies remain open performance work.

The opt-in Rust controller benchmark also passed. Its synthetic 1,000-window workload reported 149.7 ms initial synchronization, 146.9 ms recovery, and 12.4 ms median event latency across 100 events.

Private raw evidence is under `artifacts/phase-4-2/performance/` and `artifacts/phase-4-2/logs/`. The benchmark driver in that directory can run in its own virtual session against this checkout.
