# Phase 4.3 security review

The new Controller1 method is a same-user read. The endpoint allowlist contains six read-only methods and one signal. Phase 3 durable authorization, native action semantics, journal, and production deny-only policy remain intact. The association CLI mutates only explicitly selected rules in the logical SQLite database. The controller and hierarchy queries use read-only database connections.

Observed desktop membership is evaluated only after receiver authentication and continuity checks. A native window UUID cannot restore a logical association or authorize an action. The projection checks Desktop parentage independently of the database foreign key and fails closed on disagreement between Workspace evidence sources. Missing protection flags and protected windows both withhold the entire row. The Qt client validates its allowlist before forwarding any record to QML, and production captions never enter card properties, accessibility text, or search.

The real-window tests run on a private D-Bus, virtual KWin socket, and isolated HOME and XDG roots. Tests that change membership or protection use an owned, PID-checked disposable Konsole fixture on that compositor. A separate private restart test verifies owner loss and replacement. No host observer was installed, and no actual user database was used. The Phase 3 regression runner used only its existing test-compiled synthetic authority fixture.

Adversarial coverage includes stale identities and packets, sequence gaps, invalid publishers, rule cross-scope checks, protected-window omission, title field rejection, version mismatch, migration failure, and burst reconciliation. See [validation](phase-4-3-validation.md) for evidence types and commands.

## Host isolation evidence

The host compositor PID and start time, global shortcut checksum, and installed-script inventory matched. The active host `kwinrc` had a different checksum in one campaign-wide capture. An immediate private compositor-restart recheck had matching before and after checksums. A later bracket around the private launcher, Wayland, restart, and synthetic tests matched the original baseline before and after the run. These samples do not identify the writer. Validation did not restore or edit host configuration. Full campaign-wide host isolation remains unqualified.
