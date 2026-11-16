# Phase 3 validation

The review starts at `71f827ee140c2c8ca0e4c91a9dc7938995dc16a5` on `codex/phase-3`. The existing suite at that commit records 128 passed tests and one explicitly ignored benchmark. The baseline tree was extracted with `git archive` and built with `cargo build --workspace --locked --offline` (exit 0) as a separate performance control.

The workstation uses Garuda Linux on x86-64, an Intel Core i5-12450H, Rust and Cargo 1.100.0, KWin 6.8.3 and Qt 6.12.0. Rust builds use the workspace lockfile and the local dependency cache. Local evidence is recorded under the ignored `artifacts/phase-3-completion/` directory.

## Evidence boundaries

The Rust authorization and executor suites exercise a synchronous synthetic world. The production D-Bus tests use a private session daemon and inspect the actual read-only controller interface. The native fixture exercises a disposable KWin compositor, owned Wayland surfaces and host pre/post comparisons. Native provenance, seat and lock enforcement, and atomic Rust consumption in a compositor turn require evidence beyond these tests.

## Existing private-bus and offscreen regressions

All nine commands below exited 0 with disposable XDG directories and without an inherited display or session bus. Raw results are in `artifacts/phase-3-completion/unchanged-commands.tsv`. These commands exercise the unchanged observer and spatial UI paths independently of the new native fixture.

```sh
python3 tests/observer-adapter/private-bus.py
python3 tests/observer-adapter/unified-cli.py
python3 tests/observer-adapter/parity.py
python3 tests/spatial/client-smoke.py
python3 tests/spatial/version-mismatch.py
node --test tests/spatial/model.cjs
env QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software dbus-run-session -- /usr/lib/qt6/bin/qmltestrunner -input tests/spatial/tst_panel.qml
dbus-run-session -- fish scripts/validate-overview.fish
dbus-run-session -- fish scripts/validate-observer.fish
```

## Integrated checks after correction

The reviewer verified implementation checkpoint `ed7df94dff74cd193965521fa27b2695f999f151` and committed corrections as `3ba6769`. Each command in the following table exited 0. The ordinary workspace suite passed 128 tests with one explicit benchmark ignored. The example target passed three further tests. The ignored synthetic benchmark passed separately. The production bridge and denial cases are included in the workspace total.

| Command | Result |
| --- | --- |
| `cargo build --workspace --locked --offline` | Default build passed |
| `cargo test --workspace --locked --offline` | 128 tests passed, including compile-fail docs and controller lifecycle |
| `cargo test --locked --offline --example phase3-native-preview` | Three display projection and strict input-envelope cases passed |
| `cargo fmt --all --check` | Passed |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Passed |
| `cargo test --workspace --locked --offline --test spatial_bridge --test security_boundary` | Three private-bus tests passed, exact production method allowlist retained |
| `env QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software /usr/lib/qt6/bin/qmltestrunner -input tests/phase-3` | Eight QtTest results passed, including setup and teardown |
| `python3 tests/phase-3-native/run.py` | Corrected native fixture and eight Wayland QtTest results passed |
| `cargo test --locked --offline --test controller_lifecycle synthetic_1000_window_latency -- --ignored --nocapture` | 100 events, median 13.30 ms, p95 13.90 ms and maximum 14.77 ms |
| `git diff --check 71f827ee140c2c8ca0e4c91a9dc7938995dc16a5` | Passed |
| `git fsck --full --strict` | Passed |

A fresh Cargo JSON artifact inventory showed only the library and normal executable for the default build. `nm -C --defined-only` on the production library found the refusal method and zero matches for the fixture issuer, authority, marker, counting sink or preview adapter. Exact command outcomes are recorded in `reviewer-commands.tsv`, `reviewer-default-artifacts.jsonl` and `reviewer-default-symbols.log`.

## Private compositor and host comparisons

The implementation checkpoint and independent corrected run both passed the native fixture. Each run created private configuration, data, state, cache and runtime roots, a separate empty D-Bus session, a KWin virtual output, two private desktops and one private Activity. Five owned surfaces exercised focus, removal, cancellation, expiry and topology invalidation. Three captured native placement projections were refused by Rust and then rendered on the corresponding owned source surfaces. The correction checks exact event order, source PID and a frame swap for withdrawal.

The first implementation attempt failed an assertion that treated deferred QObject deletion as view visibility, and also detected changed host effects. Subsequent assertions check visibility and a rendered frame. The effect comparison retains strict equality and reports differing nested properties. The final implementation and reviewed runs matched all monitored host fields. A failed comparison triggers failure without restoration.

The host comparison covers the bus identity, KWin owner and PID, virtual desktop properties, effect properties, active output, Activity owner and full Activity IDs, five monitored configuration paths, autostart file contents and installed extension paths. It does not observe host native window inventory or focus. The harness performs read-only host D-Bus calls and does not connect to the host Wayland socket. Host equality applies to the two sampled snapshots for those fields. Private process-group drainage, private socket removal and removal of temporary roots passed.

## Alternating performance control

Five alternating baseline and final runs used the same 301-row and 1,000-window fixture, debug Rust, private D-Bus and the production QML plugin in offscreen software rendering. All ten commands exited 0. Each command used `env -u DISPLAY -u WAYLAND_DISPLAY python3 tests/spatial/performance.py --output <sample-json>` from its respective source tree. Percentiles use nearest rank, so five-sample p95 is the maximum.

| Metric | Baseline median / p95 / maximum | Final median / p95 / maximum | Samples per build |
| --- | ---: | ---: | ---: |
| Initial context to QML (ms) | 419.47 / 584.73 / 584.73 | 419.41 / 531.35 / 531.35 | 5 |
| Identity removal to QML (ms) | 428.91 / 1097.77 / 1097.77 | 418.52 / 836.84 / 836.84 | 5 |
| Hierarchy fetch (ms) | 9.72 / 23.58 / 28.89 | 10.83 / 16.45 / 16.97 | 50 |
| Controller RSS (KiB) | 19,924 / 22,816 / 22,816 | 20,936 / 20,992 / 20,992 | 5 |
| Idle CPU (% of one core) | 2.33 / 2.65 / 2.65 | 2.00 / 2.33 / 2.33 | 5 |

The hierarchy median increased by 1.11 ms while the final tails were lower. Both variants showed wider latency tails than the prior pass on this workstation. The changed source adds an explicit fixture example and native test path, while the production controller, spatial transport and QML client paths retain their existing code. These five-run distributions under ordinary workstation load cannot attribute the observed timing differences to the patch. Both variants had zero geometry-only invalidations in every run. Raw samples are in `baseline-1.json` through `baseline-5.json` and `final-1.json` through `final-5.json`. This is offscreen transport and client evidence. Native compositor latency and physical input performance remain unmeasured.
