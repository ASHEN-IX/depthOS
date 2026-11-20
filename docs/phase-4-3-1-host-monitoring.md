# Phase 4.3.1 host monitoring

`tests/support/host_isolation_monitor.py` installs inotify directory watches before the baseline. It observes `kwinrc`, `kglobalshortcutsrc`, and `kdeglobals` in the default configuration directory and any distinct active XDG config directory. The parent directory watch continues across atomic replacement of a target. The monitor records move cookies, write, modification, and metadata events. Watch invalidation and queue overflow fail qualification.

Each scenario checkpoint records existence, SHA-256, device and inode, ownership, mode, size, links, nanosecond mtime and ctime, and whether metadata was stable through the read. Files are opened read-only with NOFOLLOW and NOATIME. No contents, complete process environments, credentials, or arbitrary filenames are logged. A missing target is recorded as absent. An unstable or inaccessible target fails the clean comparison.

The process evidence is a limited `/proc` inventory with names, PIDs, and process start ticks. Harness checkpoints add the private owned processes and their bus and socket identities. Inotify supplies an event and observation time, with no writer PID. A synthetic replacement-and-restore test proves that both matching final contents and inode replacement are recognized correctly. The record assigns UNKNOWN to event writers.

Reading kernel audit status without elevation was refused. Retrospective writer attribution is blocked without already-captured audit evidence. Future per-process attribution with privileged audit configuration would require separate approval. The qualification uses no elevated or persistent monitoring.

Raw records are owner-only under `artifacts/phase-4-3-1/`. The monitor never restores host settings.
