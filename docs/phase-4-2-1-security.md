# Phase 4.2.1 security regression

The production controller, observer, SQLite schema, action authorization, and D-Bus exports have no source diff from the verified Phase 4.2 checkpoint. The UI still changes only its own logical selection. Native placement, activation and desktop switching remain outside its capabilities. Production native authorization remains deny-only.

Window titles start hidden. Explicit reveal passes through the prior sanitized caption policy. Protected identities remain generic. Demo is an explicit source and retains its visible label. Missing SQLite still requires an explicit create action. The real launcher regression checks both startup without a database write and explicitly requested creation in disposable state.

User artwork enters only via the explicit local launcher override. It accepts regular files, refuses final-component symlinks, allowlists PNG, JPEG and WebP by content, and caps both input and decode size. The UI requests category keys from a local Qt image provider. It consumes no hierarchy-supplied file paths or remote URLs. Screenshot capture is opt-in and records only the DepthOS window. The launcher saves a private PNG through an atomic write.

| Check | Result |
| --- | --- |
| Existing Phase 3 adversarial regression suite | Passed all twelve stages, including the private native fixture integration |
| Production Controller1 allowlist | Five read-only methods and one signal, verified by source and integration tests |
| Title handling and protected identity | Existing redaction and control-character regressions passed |
| Controller and SQLite paths | Private Wayland and offscreen launcher tests passed |
| Artwork inputs | A valid image loaded successfully. Checks rejected unknown categories, nonlocal path strings, symlinks, excess pixels, and invalid bytes. |
| Host isolation | Freshly bracketed private runs preserved both host configuration hashes. The installed-script inventory matched across the campaign. |

Raw validation and visual evidence are ignored and owner-only. Private sessions used disposable display sockets, session buses, and XDG roots.

The original campaign-wide comparison found differences in both host checksums after the long interruption. Recorded modification times place those edits outside the private-test windows, and the source of those edits was undetermined. A fresh bracket from 19:50:26 to 19:50:39 Africa/Tunis covered the private EIS, launcher, and benchmark entry points. Both checksums matched within that bracket. The original and recheck evidence are both retained.
