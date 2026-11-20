# Phase 4.1.1 validation

Branch `codex/phase-4-1-1`. Offscreen Qt Test does not stand in for a Wayland run. Both are listed. Host KWin was not scripted and was not given the observer package.

| Check | Result | Evidence |
| --- | --- | --- |
| `cargo build --workspace --locked --offline` | PASS | dev profile finished, exit 0 |
| `cargo test --workspace --locked --offline` | PASS | 233 passed, 0 failed, 1 ignored |
| `cargo fmt --all --check` | PASS | exit 0 |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | PASS | exit 0 |
| QML Qt Test `tst_phase_4_1_1.qml` | PASS | 48 passed, 0 failed, offscreen software |
| QML Qt Test `tst_spatial_alpha.qml` | PASS | 17 passed, 0 failed, offscreen software |
| Live Wayland UI launch | PASS | `tools/depthos-spatial-alpha --mock --quit-after-ms 2500` on `wayland-1`. Log: `source=demo`, `observer=Native Observer Unavailable`, Intel ADL GT2, vsync 6.94 ms. Exit 0. No file under the disposable `XDG_DATA_HOME`, and `~/.local/share/depthos` stayed absent |
| Focus restoration | PASS | Private KWin 6.8.3. Six script reactivations and four server-side titlebar clicks each left H/L working. Escape in the search field set `searchOpen` false and the next L changed `worldIndex`. Qt Test DEF-07 cases passed. Host-compositor activation was not driven |
| Mouse single click | PASS | Private session, EIS. One click on `worldCard-world-design` kept `currentLevel` at 1 and selected that world |
| Mouse double click | PASS in Qt Test. BLOCKED on EIS | `test_def11_double_click_world_enters_level_b_exactly_once` passed. A private EIS double click on the same card that the single click hit produced no scene events, so this run cannot claim the compositor delivered the second click |
| Pinch capability | BLOCKED | Private KWin advertises `zwp_pointer_gestures_v1`. libei has no pinch. No physical pinch. Synthetic `pinchUpdate(1.4)` on the host window moved level 1 to 2; that is a function call, not a gesture |
| First-run SQLite | PASS | `depthos init` in `tests/first_run_init.rs` and the disposable probe. Directory 0700, file 0600, explicit init only |
| Existing database not modified | PASS | After init, `tree` left sha256 `6c88accc64870094` and the mtime unchanged |
| Empty, corrupt, unsupported schema, permissions, symlink, concurrent read, failed transaction, interrupted init | PASS | Disposable probe plus `tests/first_run_init.rs` (0-byte stays 0, corrupt and user_version 99 unchanged, mode 000 and 0644 refused, symlink not followed, eight concurrent `tree` calls created nothing, `failed_in_place_initialization_rolls_back_to_the_original_zero_bytes`, `interrupted_init_before_publish_leaves_no_database_and_no_residue`) |
| Observer private session | PASS | Stage 1 private KWin 6.8.3 load, snapshot, add, focus, remove, reconnect, uninstall. Script sources unchanged since `901166a`. Not installed on the host. Fish installer still requires the string `kwin 6.8.1` |
| Production D-Bus allowlist | PASS | Exact five read methods and `invalidated` |
| Phase 3 security regression | PASS | Eleven suites in `run_all_adversarial.py`, then the private KWin runner after building `target/kwin_authority_server`. Eight integration cases passed |
| Host isolation | PASS | See the end of this file. No observer package, no `~/.local/share/depthos`, `isScriptLoaded("depthos-observer")` false. Host config hashes were not reverted |
| `git fsck --full` | PASS | Exit 0. Dangling blobs and trees only. No corrupt object |
| Memory | PASS | `docs/phase-4-1-1-memory-profile.md`. Host window VmHWM 217436 KiB. Procedure peak 100056 KiB. 522.96 MiB was not reproduced |
| 144 Hz pacing | PASS as a measurement, not as "zero drops" | `docs/phase-4-1-1-frame-pacing.md`. Animation median 6.940 ms, p95 7.119, p99 7.259, max 35.769, 422/873 over 6.944 ms, 1/873 over 16.667 ms, 143.37 FPS. Idle swaps stopped after 0.20 s |

## Persistence, in one place

Reads (`tree`, `doctor`, `world list`, `workspace list`, `desktop list`, `status`) do not create a database. `init` is the explicit creator and refuses to overwrite a real database. `world create` still creates. That split is what the tests lock.

## Host isolation

Taken after the Wayland launches and before the final commit. The comparison base for this campaign is the Stage 1 after-snapshot, not a guess about earlier drift: `kwinrc` `5c54da5400798ca2454555294ed4f2b5`, `kglobalshortcutsrc` `71ec5a5233ba15711248372fd7529759`, `kdeglobals` `36a764f7f9639b937e88f78f0fb1aff4`, KWin pid 1727. After the Wayland work, at 2026-11-20T14:12:33+01:00: `kwinrc` `5c54da5400798ca2454555294ed4f2b5`, `kglobalshortcutsrc` `71ec5a5233ba15711248372fd7529759`, `kdeglobals` `36a764f7f9639b937e88f78f0fb1aff4`, KWin pid 1727, `isScriptLoaded("depthos-observer")` false. Those match the snapshot taken at the start of this stage. The earlier difference from the Nov 19 file is not attributed to this campaign and was not reverted.
