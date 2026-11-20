# Phase 4.1.1 memory profile

Measured on the post-fix tree (Agent 2 corrections included, before those corrections were committed). Host session `WAYLAND_DISPLAY=wayland-1`. One Python process loads `experiments/spatial-alpha/main.qml`. `nproc` is 1 in every snapshot. There is no child process to separate from the UI process. `VmHWM` is the Linux high-water mark in KiB, the same unit as `ru_maxrss`.

Renderer for the host window: OpenGL, vendor Intel, `Mesa Intel(R) Graphics (ADL GT2)`, Mesa 26.2.7, Qt 6.12, scene graph OpenGL, vsync 6.94 ms. The GPU accounting key is `drm-driver: i915`. NVIDIA's `libnvidia-gpucomp.so.620.70.06` is still mapped (30408 KiB RSS, 6423 KiB PSS) while Intel draws the window.

## Host Wayland window

| Point | RSS | PSS | Private dirty | Anonymous | File-backed RSS | i915 resident |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| After load | 208668 KiB (203.8 MiB) | 82095 KiB (80.2 MiB) | 61800 KiB (60.4 MiB) | 61800 KiB | 146856 KiB (143.4 MiB) | 58888 KiB |
| After idle | 208764 KiB | 82228 KiB | 61892 KiB | 61892 KiB | 146856 KiB | 42060 KiB |
| During animation | 214432 KiB (209.4 MiB) | 87692 KiB | 66984 KiB | 66984 KiB | 147432 KiB | 55804 KiB |
| After repeated navigation | 217304 KiB (212.2 MiB) | 90554 KiB | 69664 KiB | 69664 KiB | 147624 KiB | 44316 KiB |
| Final, same process | 217364 KiB | 90597 KiB | 69724 KiB | 69724 KiB | 147624 KiB | 44316 KiB |

`VmHWM` at the end of that process is 217436 KiB (212.3 MiB). From after load to final, RSS climbs about 8.5 MiB. Private dirty is anonymous. Most of the RSS is file-backed library mappings, which is why PSS stays near 80–90 MiB while RSS sits near 210 MiB.

Eight hide/show cycles with the frame hooks off stay at 200.6 MiB RSS (private dirty 57.9–58.0 MiB). Five separate process launches, each an idle demo, finish at 197.4, 196.8, 197.2, 197.3, and 197.0 MiB RSS. Launching again does not accumulate.

Offscreen software, hooks off: RSS 87.2 MiB after load, 87.3 MiB final, PSS 43.9 MiB, private dirty 32.8 MiB. No i915 client.

## The 83.5, 104, 200, and 523 MiB figures

They are not four readings of one window.

**83.52 MiB** is the sentence in `docs/phase-4-1-performance.md`: an overview baseline of 78.92 MiB plus a stated +4.6 MiB. `78.92 + 4.6 = 83.52`. That paragraph says the number came from `os.wait4` on a separate process. `tests/spatial/benchmark_spatial.py` does not do that. It reads `resource.getrusage(RUSAGE_SELF).ru_maxrss` in the benchmark process and divides by 1024.

**522.96 MiB** is the historical `peak_rss_mib` copied into `artifacts/phase-4-1/benchmark-results.json`. It is `535516 / 1024` (522.96484375, rounded to 522.96). A fresh run of that procedure does not reach it. It is a high-water mark from an earlier process, not the live window and not this build.

**103.89 MiB** was the Stage 1 fresh procedure on commit `901166a` (106388 KiB after five cold starts and the 1850-entity lists). The same procedure on this build:

| Point | ru_maxrss |
| --- | ---: |
| After import | 36172 KiB (35.32 MiB) |
| After cold start 1 | 81664 KiB (79.75 MiB) |
| After cold start 5 | 83448 KiB (81.49 MiB) |
| After 1850 entities | 100056 KiB (97.71 MiB) |

Cold start 1 (79.75 MiB) sits next to the doc's 78.92 MiB. The procedure now peaks at **97.71 MiB**.

**200.22 MiB** was the Stage 1 live Intel window `VmHWM` (205028 KiB). This build's host window high-water mark is **212.3 MiB** RSS-class (`VmHWM` 217436 KiB), with PSS about 80–90 MiB. The extra RSS against the Stage 1 window is consistent with a longer navigation phase (anonymous grew from 60.4 MiB to 68.1 MiB) plus the NVIDIA compiler library that GLVND maps even though i915 is the renderer.

Hide/show with the frame-timing hooks attached wedged the process in a render-thread wait. Those hooks are outside the application. The no-hook reopen and the five relaunches are the memory evidence for close and relaunch.
