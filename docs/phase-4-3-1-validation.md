# Phase 4.3.1 validation

**PHASE 4.3.1 VERIFIED**

The hardened private-session boundary passed. The historical Phase 4.3 writer remains **UNKNOWN**. Test evidence of a clean current run does not retroactively attribute a historical write.

The worktree is `/home/ashenix/Projects/DepthOS-phase-4-3-1` on `codex/phase-4-3-1`, rooted at the verified Phase 4.3 commit `7fa469e489474554dd8b851c0a70ff274b4d8341`. Ancestry to canonical main `6543b65213f24a733012ac5db50e69ed382d1cfb`, both source worktrees, Git integrity, and the deny-only production control surface were checked. There was no merge or push.

## Instrumented private Wayland qualification

| Run | Artifact directory | Host checkpoints | Host events | Outcome |
| --- | --- | ---: | ---: | --- |
| Lifecycle 1 | `w-6jnea87_` | 74 | 0 | Package lifecycle, focused raster fixture, membership, protection, destruction, and observer and controller reconnect |
| Lifecycle 2 | `w-ets7zk97` | 74 | 0 | Same gates on a fresh bus and runtime |
| Lifecycle 3 | `w-owa46erw` | 74 | 0 | Same gates on a fresh bus and runtime |
| Private compositor restart | `w-t4bw1jdk` | 26 | 0 | Previous owner invalidated. New owner acquired and rules retained |
| Interrupted script lifecycle | `w-vn801ttq` | 16 | 0 | TERM after script load. The package and all owned processes and sockets cleared |
| Isolation fault suite | `a-1yuvb01f` | 26 | 0 | Twelve cases passed, including controlled missing configuration, concurrent virtual KWin, and reaping a detached child |

All six comparisons retained one file-metadata and digest state across every snapshot of the three targeted configuration files. Host KWin identity matched at each start and finish. Independent brackets around the launcher, burst, Qt, and Phase 3 tests also remained clean. Every private KWin run recorded its actual daemon ID, endpoint, process identity, and Wayland socket inode. The installed package path stayed under each authorized data root.

## Regression results

- Locked offline build and strict all-target Clippy passed. The Rust workspace reported 237 passing tests with one ignored opt-in benchmark.
- QML lint and 111 Qt tests passed. Standalone launcher first-run, real Wayland, offscreen, three contract checks, and legacy version mismatch passed.
- The production association burst recovered from the sequence gap and repeated three 500-event rounds.
- All 12 existing Phase 3 adversarial stages passed. The allowlist remained six read-only Controller1 methods and one signal. The native Phase 3.10.1 suite passed all eight cases inside the sandbox.
- The historical process attribution remains blocked without retained writer-attributed audit evidence. Installing or enabling elevated audit monitoring is a separate approval decision.

## Reproduce

```sh
cargo build --workspace --locked --offline
dbus-run-session -- cargo test --workspace --locked --offline
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings

for run in 1 2 3; do python3 -I tests/spatial/test_phase_4_3_wayland.py || exit; done
python3 -I tests/spatial/test_phase_4_3_wayland.py --compositor-restart
python3 -I tests/spatial/test_phase_4_3_wayland.py --interrupt
python3 -I tests/spatial/test_phase_4_3_1_isolation.py
python3 -I tests/spatial/test_alpha_launcher.py --wayland
python3 -I tests/spatial/test_alpha_launcher.py
python3 -I tests/spatial/test_phase_4_3_burst.py
python3 tests/spatial/test_phase_4_3_contract.py
python3 tests/spatial/version-mismatch.py

/usr/lib/qt6/bin/qmllint -I experiments/spatial-alpha experiments/spatial-alpha/main.qml
QT_QPA_PLATFORM=offscreen /usr/lib/qt6/bin/qmltestrunner \
    -input tests/spatial -import experiments/spatial-alpha

g++ -std=c++20 -O2 -fPIC -DDEPTHOS_AUTHORITY_UNIT_TEST \
    experiments/kwin-authority/native/kwin_authority_server.cpp \
    -o target/kwin_authority_server $(pkg-config --cflags --libs Qt6Core)
cargo build --example phase-3-10-native-coordinator --locked --offline
(umask 022 && exec dbus-run-session -- python3 tests/adversarial/run_all_adversarial.py)
git fsck --full --no-dangling
```

QtTest qualification ran with the Landlock supervisor's offscreen environment. The compact direct command above reproduces the Qt assertions. The native harness requires a Linux kernel that accepts the Landlock filesystem and IPC rules and passes the live refusal probes. Regressions abort when the boundary cannot be enforced.

## Phase 4.4 readiness

Proceed with Phase 4.4 development and private validation using the hardened harness. Keep production native authorization deny-only and host observer deployment outside that scope. Retain the historical attribution as UNKNOWN. A host rollout or a future process-attribution campaign needs its own approval and evidence.
