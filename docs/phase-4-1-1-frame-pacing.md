# Phase 4.1.1 frame pacing

Host window on `wayland-1`, Qt 6.12, threaded render loop, `QQuickWindow.frameSwapped`. This is not a `wp_presentation` timestamp. The display budget used here is `1000/144 = 6.944 ms`. `16.667 ms` is the 60 Hz budget and is reported separately. An interval above 6.944 ms is not, by itself, a dropped 144 Hz frame: a vsync lock whose median is 6.94 ms puts about half of the samples just over that cutoff.

Renderer: OpenGL, vendor Intel, `Mesa Intel(R) Graphics (ADL GT2)`, Mesa 26.2.7, `swapInterval` 1. The log line is `Animation Driver: using vsync: 6.94 ms`. The private virtual KWin used for input tests reported `using vsync: 16.67 ms` and rendered with the NVIDIA RTX 4050. Those frames are not this table.

Percentile index: `sorted[round(p/100*(n-1))]`.

## What the new build does when nothing is moving

The pre-fix demo painted observation health as Live. `StatusBadge` runs an infinite pulse for Live and Reconnecting, and that pulse held the window at about 144 FPS while idle (Stage 1: median 6.946 ms, p95 7.084, p99 7.150, max 7.207, 284/553 over 6.944 ms, 0 over 16.667 ms, 144.25 FPS).

On this build the header health is Unavailable and demo window cards paint Demo. Demo does not pulse. After load, the idle phase produced **29** swap intervals spanning **0.20 s**, then swaps stopped for the rest of the 8 s idle wait.

| | Idle burst (then stopped) | Animation, 6.09 s |
| --- | ---: | ---: |
| Intervals | 29 | 873 |
| Median | 6.945 ms | 6.940 ms |
| p95 | 7.098 ms | 7.119 ms |
| p99 | 7.117 ms | 7.259 ms |
| Max | 7.117 ms | 35.769 ms |
| Over 6.944 ms | 15 | 422 |
| Over 13.889 ms | 0 | 1 |
| Over 16.667 ms | 0 | 1 |
| Effective FPS of the intervals that exist | 146.40 | 143.37 |

The idle burst is the window finishing its open. It is not an 8-second 144 Hz lock, and it is not a claim of zero dropped frames. The animation sample is the continuous one. One interval of 35.769 ms is a hitch of about four extra 6.944 ms vsyncs. 422 of 873 animation intervals are over 6.944 ms because the median itself is 6.940 ms.

Repeated keyboard navigation on the GUI thread is not a steady swap stream: 70 intervals, span 0.58 s, median 6.898 ms, p95 24.656 ms, max 35.944 ms, 8 intervals over 16.667 ms, effective 120.70 FPS. The GUI thread was injecting keys. Those long gaps are the thread being busy, not a second idle pulse.

Resize during that animation completed and the process exited. Rapid in-process `QWheelEvent`s posted at the window content item did not reach `WheelHandler` (`wheelEventsHandled` stayed 0). Compositor scroll in the private session did: five EIS notches advanced the counter by 5, and the full wheel pass advanced it by 45. The camera stayed at the origin because `panLimitX` and `panLimitY` were 0 (the mock grid fits the 1440×900 view). The Qt Test wheel cases cover a pan when the content is larger than the viewport.

Synthetic `pinchBegin` / `pinchUpdate(1.4)` / `pinchEnd` moved the scene from level 1 to level 2. That is a direct call. It is not a touchpad pinch. Close during an in-progress level change exited in 4.2 s with the frame hooks off. The same close with the hooks attached aborted inside `QWindow.close` while the GUI thread waited on the render thread. That abort is the hook, and the no-hook close is the application result.
