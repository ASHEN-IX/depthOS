# Phase 4.3 host configuration incident

Historical attribution is **UNKNOWN**. The source is the ignored Phase 4.3 evidence in the previous checkout. Times below are local to Africa/Tunis on 2026-11-20. File timings are recorded metadata and artifact write times. They do not prove when a writer opened or replaced a file.

| Time | Confirmed evidence |
| --- | --- |
| 20:30:09.510 | Initial `kwinrc` SHA-256 started with `873a90b38752`. The host KWin PID and start time were recorded. Shortcut and script inventory baselines were collected |
| 20:44:20.325 | Changed `kwinrc` mtime, recorded by the later `stat` sample. File size 3677 bytes. The exact write operation and inode transition were not captured |
| 20:44:20.604 | Synthetic burst log last write. The source launches an offscreen Qt client and a disposable bus using a scrubbed environment |
| 20:44:20.868 to 20:44:35.536 | User journal messages from private KWin PID 46204. The first message follows the recorded changed-file mtime. The corresponding private run used `wayland-iiuz2_s7/runtime/depthos-phase-43` and loaded an existing observer script by absolute path |
| 20:44:57.222 | The campaign comparison captured changed digest `da40b53d35bb`. The host compositor instance, shortcuts, and installed-script inventory matched |
| 20:45:12.026 to 20:45:15.150 | Changed digest held across the targeted restart bracket. User journal identifies private KWin PIDs 47048 and 47135. The script loaded and exercised a replacement observer. Neither recorded hash identifies a writer |
| 20:45:39.903 | First subsequent sample with digest `873a90b38752`. Restoration occurred at an unknown time between samples |
| 20:46:11.274 and 20:47:57.534 | Longer rerun and screenshot brackets matched the original digest |

The native harness used `dbus-run-session` to create an independent bus. Its exact historical generated addresses, full environments, and most child PIDs were not recorded. The source set HOME and the user-specific XDG directory variables, selected Qt Wayland and a relative socket name, and omitted the inherited session-bus address from the child environment. The restart retained one private bus and socket name. The synthetic benchmark used a temporary runtime and offscreen Qt. The earlier production-launcher fixture used `depthos-launcher-test`. Source evidence establishes those launch intentions. It cannot reconstruct credentials or every connection historical descendants made.

The available user journal identifies the private compositor processes and ordinary desktop services, but contains no file-writer record. The recorded host KWin did not emit a journal line in the queried interval. Journal silence does not exclude a file write. KWin or a KDE settings component could update settings during a desktop session. A test escaping its private environment is also a hypothesis. Neither explanation is supported as the historical cause. No historical syscall audit or paired file content was available. Reading system audit status as the current UID was denied; privileged auditing was left untouched.

Phase 4.1.2 and Phase 4.2 reported matching point-in-time host checksums and script inventories. Phase 4.2.1 separately reported campaign-level checksum drift with stable short brackets. None of those point samples provides historical process attribution.
