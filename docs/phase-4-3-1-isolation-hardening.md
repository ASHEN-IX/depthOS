# Phase 4.3.1 isolation hardening

`tests/support/private_session.py` creates the Linux regression sandbox before starting the bus or any KDE process. The supervisor validates the entire generated environment, directory ownership and modes, and absence of stale endpoints. A kernel without required Landlock capabilities fails before startup. The verified kernel exposed Landlock ABI 11. Runtime probes against disposable sibling paths and sockets are the gate, independent of the reported ABI.

The private bus uses EXTERNAL authentication and no service activation directories. Its manifest contains the pathname socket device and inode, daemon PID and start tick, and bus ID. Internal test entry points verify this identity before opening Qt or invoking an observer operation. Wayland operations bind to the spawned compositor with the same class of PID and socket checks, plus a unique D-Bus name. A restart must acquire a new, verified handle.

The observer is installed with `kpackagetool6 --type KWin/Script --packageroot <run>/data/kwin/scripts`. Passing no root or an escaping path fails before calling the tool. The actual installed file is loaded by the verified private compositor. Removing and reinstalling the package is part of the normal lifecycle. The separate private configuration disables global shortcuts during the run through the KWin command line.

A supervisor owns the temporary service process tree. It is a child subreaper and sends signals through pidfds after checking descendant membership. It reaps adopted descendants, removes the owned package after interruption, clears private socket and lock names, and records the final process inventory. Screenshots and test output stay in disposable roots. An optional launcher export accepts a new destination under repository artifacts.

With a host monitor enabled, scenario markers are acknowledged after an independent parent records the host snapshot and drains the inotify queue. The sandbox applies to the inner processes. The outer host monitor reads only the selected files and writes evidence into the private artifact directory.
