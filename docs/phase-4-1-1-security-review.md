# Phase 4.1.1 security review

Reviewer: Agent 2. Range: `901166a..` the Agent 2 commits on `codex/phase-4-1-1`. Agent 1's commits were not rewritten. Nothing was installed into host KWin. No host KDE config file was edited. `~/.local/share/depthos` was not created. Mutable databases were under temporary directories or `--db` paths created for the test.

## What the diff does not add

`git diff 901166a` does not touch `src/spatial/mod.rs`. `org.depthos.Controller1` still exports only `get_capabilities`, `get_controller_health`, `get_hierarchy`, `get_context_report`, `get_current_logical_context`, and the `invalidated` signal. `tests/adversarial/test_production_allowlist_adversarial.py` passed and then ran the Rust security-boundary tests. There is no new native window setter, no production authorization change, and no new writable D-Bus action method.

`Cargo.toml` adds the `fs` feature on `rustix`. That is the filesystem support `depthos init` uses for private permissions. It is not an authorization change.

The spatial launcher no longer has the hard-coded `/home/ashenix/...` database path. Screenshots of this window go through `grabToImage`. `grabWindow()` is not used. The launcher log on the host session printed `source=demo` and `observer=Native Observer Unavailable`. A connected controller is not painted as a live observer. No global shortcut is registered. No screen capture of other windows was taken.

The observer script still copies window captions, including when `excludeFromCapture` is true. That code was not changed in this phase, and it is not loaded on the host. See `docs/phase-4-1-1-observer-feasibility.md`.

## Defects corrected in this review

**Read commands created a missing database.** After Agent 1, `tree` and `status` were read-only, and `doctor`, `world list`, `workspace list`, and `desktop list` still went through `SqliteStore::open`, which creates and migrates. A missing path therefore became a new user database. Those commands now use `SqliteStore::open_read_only`. A missing file fails with `run depthos init` and is not created. An existing file's sha256 and mtime are unchanged by `tree`. A 0-byte file stays 0 bytes. Corrupt bytes, a foreign database, and schema version 99 are refused and left byte-for-byte. Mode `000` and mode `0644` are refused. A symlink database is not followed (the target stayed 0 bytes). A symlinked default directory received no file. Eight concurrent `tree` processes on one missing path created nothing. `doctor` on a missing path created nothing. Write commands such as `world create` still create, which `other_commands_still_create_a_database_exactly_as_before` locks in. Interrupted init, a failed in-place transaction, and concurrent `init` are covered by `tests/first_run_init.rs`, which passed in the workspace run.

**Demo window cards painted Live.** `MockData.js` still carries sample `statusBadge` strings, including Live. `WindowCard` painted that string, and `StatusBadge` pulses for Live. In a demo, the card now paints Demo. An offscreen dump of the live scene showed window-card `paintedStatus` Demo and the header badge Unavailable. The Qt Test `test_demo_window_cards_are_not_painted_live` passed.

## Suites

| Suite | Result |
| --- | --- |
| Production D-Bus allowlist (`test_production_allowlist_adversarial.py`) | PASS |
| Native authority ASan/UBSan unit test | PASS |
| Injected input classification | PASS |
| Coordinator lease adversarial | PASS |
| SQLite durability adversarial | PASS |
| Security contract claims | PASS |
| Durable coordinator protocol | PASS |
| Cross-process durable protocol | PASS |
| Phase 3.10.1 native authority unit test | PASS |
| Phase 3.10.1 IPC trust | PASS |
| Phase 3.10.1 durable reconciliation | PASS |
| Phase 3.10.1 private KWin integration runner | PASS |

The first runner attempt exited because `target/kwin_authority_server` is not produced by `cargo build --workspace`. It was compiled locally with `-DDEPTHOS_AUTHORITY_UNIT_TEST` and the example `phase-3-10-native-coordinator` was built with `cargo build --example --locked --offline`. The rerun passed all eight cases (happy path, focus loss, window destruction, generation bump, desktop removal, screen lock, incarnation fencing, forged bootstrap token). Log: `artifacts/phase-4-1-1/agent2/raw/stage3-phase310-native.log`. The runner uses a private D-Bus daemon, a private `HOME`, and `kwin_wayland --virtual`. It does not install a host script.
