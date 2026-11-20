# Phase 4.2 security review

This phase uses the existing read-only production boundary. Rust model, action security, observer, controller, SQLite schema, and D-Bus export definitions are unchanged from the Phase 4.1.2 baseline. Changes on the launcher-side controller path are limited to `GetHierarchy`, `GetControllerHealth`, the `Invalidated` signal, and a session-bus service owner watch.

Native setter access and production consent-token issuance remain deny-only. Window tiles use observation metadata and renderer-owned artwork. Titles start hidden. Explicit reveal displays only allowed input after plain-text and directional-control handling. Protected identities remain generic. The launcher confines diagnostic output to source and health labels and does not record window titles. Runtime captures used demo or disposable test hierarchy data.

| Verification | Result |
| --- | --- |
| Existing adversarial suite | All 12 stages passed, including eight private KWin fixture cases |
| Production Controller1 allowlist | Five read-only methods and one signal, verified by source and Rust integration |
| Native observer lifecycle | Private observer load, snapshot, changes, removal, reconnect, and cleanup passed |
| Disposable SQLite launcher | Read-before-create, explicit initialization, read-only startup, and controller loss/recovery covered by the launcher test |

Raw logs and screenshots live under the ignored, owner-only `artifacts/phase-4-2/` directory. Native fixture runs used private Wayland sockets, D-Bus sessions, and XDG roots.

Host `kwinrc` and `kglobalshortcutsrc` SHA-256 values matched after validation. The host KWin script inventory matched. Canonical `main` and the Phase 4.1.2 checkout remained clean at their verified baselines.
