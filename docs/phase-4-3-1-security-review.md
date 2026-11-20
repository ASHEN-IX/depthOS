# Phase 4.3.1 security review

The change is limited to regression processes and evidence collection. The Rust controller, observer protocol, Qt application, native action gate, and durable journal retain their Phase 4.3 behavior. Production authorization remains deny-only. Native fixture modifications are scoped to a PID and application ID on a verified private KWin instance.

The old environment checks left test-only fallback and escape paths. No observed historical write has been attributed to those paths. The hardened tests combine independently validated process, IPC, and filesystem boundaries. New subprocesses inherit the kernel restrictions. Service activation is disabled for the shared private daemon, and normal package operations are confined to the disposable data directory. Qualification tests reject bad paths and endpoints before any native interaction.

The Linux rules protect ordinary file writes, truncation, creation, deletion, cross-directory file moves, and new Unix socket connections. They allow reads and a small set of explicitly documented device opens. Existing descriptors, metadata operations outside Linux's handled rights, other processes in the user session, and historical writer attribution are outside the proof. The host monitor checks the selected files for covered events throughout the runs and refuses to qualify any drift or coverage loss. Matching end-state hashes are only one part of the gate.

Internal launch options assume the developer invokes repository test code they trust. The private kernel boundary prevents accidental endpoint fallback. It is not an authorization design for running hostile arbitrary binaries as the same user.
