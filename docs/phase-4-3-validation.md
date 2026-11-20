# Phase 4.3 validation

The isolated `codex/phase-4-3` worktree was cut from `64f53e74b81d7de229dc15412c108a07a67f347e`. Baseline refs, ancestor relationships, clean source worktrees, and full Git integrity were checked. Implementation, validation, and self-review ran sequentially in one agent. No refs were rewritten and no merge or push was performed.

## Reproduce

Run from the Phase 4.3 worktree. The Python Wayland scenarios require KWin, Konsole, and PyQt6. They create private D-Bus and XDG roots automatically. Do not aim a fixture at the host compositor.

```sh
cargo build --workspace --locked --offline
dbus-run-session -- cargo test --workspace --locked --offline
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings

/usr/lib/qt6/bin/qmllint -I experiments/spatial-alpha \
    experiments/spatial-alpha/{main,DesktopsView,WorkspacesView,WindowCard,SpatialBackend}.qml
QT_QPA_PLATFORM=offscreen /usr/lib/qt6/bin/qmltestrunner \
    -input tests/spatial -import experiments/spatial-alpha

python3 tests/spatial/test_phase_4_3_contract.py
python3 tests/spatial/version-mismatch.py
python3 tests/spatial/test_alpha_launcher.py --wayland
python3 tests/spatial/test_phase_4_3_wayland.py
python3 tests/spatial/test_phase_4_3_wayland.py --compositor-restart
python3 tests/spatial/test_phase_4_3_burst.py

g++ -std=c++20 -O2 -fPIC -DDEPTHOS_AUTHORITY_UNIT_TEST \
    experiments/kwin-authority/native/kwin_authority_server.cpp \
    -o target/kwin_authority_server $(pkg-config --cflags --libs Qt6Core)
cargo build --example phase-3-10-native-coordinator --locked --offline
dbus-run-session -- python3 tests/adversarial/run_all_adversarial.py
git fsck --full --no-dangling
```

The legacy permission fixture expects a normal shell umask such as 022. Redirect its output to an owner-only log opened separately. The standalone launcher selects a running controller by default. Use `tools/depthos-spatial-alpha --reference-demo` for the separate demo path. The private test scripts write captured application windows and measurements under the ignored `artifacts/phase-4-3/` directory.

## Evidence by case

| Case | Result and evidence |
| --- | --- |
| Duplicate and out-of-order events | Pass in existing Rust replica tests. Replayed frames cannot roll back the position |
| Unauthenticated publisher and stale epoch | Pass in private-bus tests, receiver continuity tests, and a new shelf projection regression |
| Native UUID reuse and generation mismatch | Pass in receiver continuity and projection regressions. Live delegates use receiver generations |
| Destroy during transition | Pass in private KWin. The owned window is removed during the return/drill-in sequence and its delegate disappears |
| Reconnect with different windows | Pass with the KWin observer unloaded and loaded again. The previous window is absent and the new window has a fresh ID |
| KDE desktop membership change and sticky membership | Pass with PID-scoped disposable Konsole under real virtual Wayland |
| Changed Workspace evidence and cross-Workspace scope | Pass in the Rust projection test and the private D-Bus bridge test |
| Logical Desktop removal | Pass in SQLite cascade and private D-Bus projection tests |
| Protected identity and default titles | Pass in private KWin and in schema tests. The shelf receives no protected row or raw title |
| Replacement snapshot missing a window | Pass in replica replacement and native observer restart scenarios |
| Migration rollback, foreign and future database | Pass with temporary fixtures. Existing version-1 reader observes only the committed upgrade |
| Controller and UI version mismatch | Pass in launcher contract tests and existing legacy integration. A compatible hierarchy remains usable without shelf capability |
| Large event burst and missing-event recovery | Pass with synthetic packets on a private bus. Three 500-event rounds settle at 100 unique rows. Gap recovery clears stale rows and replaces all 100 IDs |
| Real compositor restart | Pass with the private compositor terminated and relaunched. Rules survive and stale rows do not |
| Production startup and demo | Pass in private Wayland launcher tests and the QtTest suite. Controller owner loss has a separate unavailable-state capture |
| Native action regression | All 12 Phase 3 adversarial stages pass, including eight private KWin fixture cases |

Locked, offline build and strict Clippy passed. The default Rust run passed 237 tests and had one opt-in benchmark, which passed separately. QtTest passed 111 checks and the launcher contract passed three checks. The owner-only logs and screenshots record the exact observations. [Performance](phase-4-3-performance.md) separates native compositor evidence from offscreen synthetic throughput. Physical presentation timing and large-hierarchy virtualization remain open.

## Qualification

**PHASE 4.3 PARTIALLY VERIFIED**

The implementation, production read-only projections, SQLite and UI integration, private KWin lifecycles, and bounded synthetic performance checks passed. The [security review](phase-4-3-security-review.md) records a transient campaign-wide host `kwinrc` checksum difference. A targeted restart bracket matched both host hashes, and the latest sample again matches the original baseline. The writer was not established.

Phase 4.4 priorities are process-attributed isolation evidence, virtualization for larger window and hierarchy grids, and a matched Wayland memory and physical refresh benchmark. Keep native authorization deny-only.
