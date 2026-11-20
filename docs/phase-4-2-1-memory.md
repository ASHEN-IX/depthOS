# Phase 4.2.1 Wayland performance and memory

The before and after processes ran sequentially within each matched private KWin 6.8.3 session. Both checkouts used Qt 6.12.0 OpenGL, 1440×900 logical pixels at DPR 1, the built-in hierarchy, and the same instrumentation and navigation sequence. The standard and low-effects pairs each used a fresh session. Phase 4.1.2's earlier 200 MiB observation is historical context. All comparisons below use the verified Phase 4.2 checkpoint.

## Allocation and lifecycle

| Profile | Settled RSS, MiB | RSS after navigation, MiB | Peak RSS, MiB | PSS after navigation, MiB | Private dirty after navigation, MiB | Navigation CPU |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Phase 4.2 standard | 290.0 | 293.3 | 293.3 | 166.9 | 140.3 | 9.01% |
| Phase 4.2.1 standard | 235.4 | 241.6 | 241.6 | 116.7 | 96.7 | 10.07% |
| Phase 4.2 low effects | 282.5 | 285.8 | 285.8 | 163.8 | 140.1 | 9.18% |
| Phase 4.2.1 low effects | 200.9 | 218.6 | 218.6 | 97.5 | 77.4 | 9.07% |

Matched standard peak RSS dropped 51.7 MiB (17.6%). Navigation CPU is process CPU time as a fraction of one core. Each matched profile produced zero settled swaps over four seconds. Idle CPU was below 0.01% of one core in these samples.

Four additional launch/exit cycles per standard profile sampled the process after settling and before exit (before 226.9–229.0 MiB, after 233.9–234.3 MiB). All child processes exited. A separate 180-cycle hierarchy and palette soak sampled RSS every 30 round trips (before 291.4–291.9 MiB, after 238.5–238.7 MiB). That exercise covers the shipped demo hierarchy. Longer sessions and more densely populated Workspaces remain profiling targets.

The previous full-window Canvas and per-World Canvas artwork are replaced by bounded 8-bit static textures. The largest background is 960×540, the deck bands are 1024×116, and the four World JPEGs are 640×320. The carousel limits delegate creation to its nearest slots. Workspace and Desktop bodies and their delegates are unloaded after fading out. These changes reduce retained QML objects and render resources. The measured memory reduction is for the combined change. A separate allocation-level attribution would require provider or driver tracing.

## Timing

| After profile, in ms | n | Median | p95 | p99 | Maximum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Native EIS Qt swap intervals | 365 | 16.685 | 16.951 | 18.022 | 18.138 |
| Native EIS render-hook elapsed | 366 | 0.416 | 0.679 | 0.865 | 1.025 |
| Matched standard Qt swap intervals | 381 | 16.679 | 16.895 | 17.037 | 18.179 |
| Matched standard render-thread CPU | 382 | 0.516 | 0.680 | 0.795 | 0.995 |
| Matched low-effects Qt swap intervals | 381 | 16.687 | 16.897 | 16.973 | 18.521 |
| Launch to first Qt swap | 5 | 284.682 | 303.315 | 303.315 | 303.315 |
| Launch to first verified direct Qt key interaction | 5 | 338.767 | 353.993 | 353.993 | 353.993 |
| World → Workspace last swap within 360 ms | 8 | 257.901 | 262.994 | 262.994 | 262.994 |
| Workspace → Desktop last swap within 360 ms | 8 | 256.138 | 263.192 | 263.192 | 263.192 |
| Local search filter | 120 | 0.018 | 0.024 | 0.074 | 0.520 |

In the compositor-delivered EIS run, the first app swap followed QML load by 379.3 ms. The settled interval contained 0 swaps in five seconds. Timings describe Qt swaps, render hooks, and application input. Physical presentation timestamps, 144 Hz scanout, and per-process GPU memory remain unmeasured. The driver exposed no memory entries in the sampled DRM fdinfo.

Raw JSON traces, the repeatable paired driver, and its client are in the ignored `artifacts/phase-4-2-1/harness/` directory. The private EIS traces and staged logs are in that directory and `artifacts/phase-4-2-1/logs/`.
