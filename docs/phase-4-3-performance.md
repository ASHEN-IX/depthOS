# Phase 4.3 performance

The debug controller and production Qt Quick launcher were measured with a private D-Bus publisher replaying valid synthetic observations. The launcher used offscreen software rendering. A separate virtual KWin Wayland run validated real window membership, focus rendering, and settled swaps.

| Measure | Result |
| --- | --- |
| Focus update at 100 windows | 102.36 ms p95 from publisher send to QML across 32 events. The acknowledgment-to-model p95 was 100.75 ms |
| Snapshot, 1 / 10 / 50 / 100 windows | 40.00 / 100.97 / 116.58 / 113.11 ms from acknowledgment to QML |
| First shelf page after snapshot acknowledgment | 1.02–3.35 ms, including controller projection and private bus round trip |
| Consecutive 500-event rounds | 422.57 / 428.75 / 423.18 ms including a 350 ms settle interval. Each round returned to 100 unique cards |
| Synthetic UI peak / final RSS and final PSS | 150096 / 150096 / 94370 KiB |
| Synthetic controller peak / final RSS and final PSS | 16500 / 16500 / 12660 KiB |
| Synthetic settled CPU, one core | 1.20% UI and 0.80% controller during a five-second probe |
| Client outstanding and pending reads | Observed high water 1. A single worker and one coalesced pending mask cap client state at 2 |
| Largest synthetic Qt timer gap | 51.64 ms across the instrumented run |
| Private Wayland settled rendering | Zero swaps during a five-second window |
| Private Wayland UI peak / final RSS and final PSS | 319468 / 293920 / 179220 KiB |
| Private Wayland controller peak / final RSS and final PSS | 14876 / 14876 / 10837 KiB |
| Real native membership change to model | 66.89 ms for one PID-scoped fixture move, including private KWin scripting round trip |

Focus kept the existing delegate. The 500-event run uses 25 batches of 20 add and remove events, repeated three times. The duplicate-count checks and RSS measurements cover this bounded workload. The production controller reuses the existing bounded observer transport and watch-driven invalidation service. The service also rechecks external SQLite changes. Stable observations do not wake QML rendering.

The send-to-model interval bounds receipt-to-model time from above. Acknowledgment follows receiver processing, and the test samples QML state at 6 ms intervals. Publisher round trips for the four snapshots were 1.84–11.46 ms. These are user-space measurements. The first-page timing includes SQLite and D-Bus overhead and does not isolate reconciliation CPU time. The private KWin run uses a different scene and probe from the 241.6 MiB Phase 4.2.1 benchmark. There is no matched peak-memory comparison or hardware scanout measurement.

Run `python3 tests/spatial/test_phase_4_3_burst.py` after the locked workspace build. Raw metrics go to `artifacts/phase-4-3/logs/synthetic-burst.json`.
