# Phase 4.3.1 isolation regressions

All endpoint and path fault injections target disposable fixtures. The code never connects to a real host service to test rejection.

| Scenario | Expected enforcement |
| --- | --- |
| Injected host-shaped bus address or Wayland display | Supervisor rejects the supplied environment before creating a bus or starting KWin |
| Missing private config or invalid runtime modes | Reject before bus or KWin startup |
| Missing private bus in internal worker | Guard refuses before connecting to any session bus |
| Missing installer root or explicit outside root | Fail before invoking KPackage |
| Stale socket and symlinked data path | Refuse the private root before launching work |
| Unavailable compositor | No private KWin handle can be constructed |
| Concurrent private KWin sessions | Distinct daemon IDs, compositor PIDs, socket inodes, and overlapping lifetimes |
| Unexpected worker exit with detached child | Supervisor identifies adopted child, escalates after TERM, reaps it, and clears endpoints |
| Interrupt after actual script load | Graceful supervisor signal handling removes the package, shuts down the descendants, and clears the sockets |
| Matching final digest after atomic replacement | Monitor detects the move and write events, preserves UNKNOWN attribution, and fails the clean check |

Run `python3 -I tests/spatial/test_phase_4_3_1_isolation.py` for the twelve automated cases. The real observer interruption runs separately with `python3 -I tests/spatial/test_phase_4_3_wayland.py --interrupt`. Both write isolated raw evidence.
