# Phase 4.1.2 Independent QA, Security & Performance Review

Branch `codex/phase-4-1-2`. Audit conducted by Agent 2 against Agent 1 checkpoint `69b32fd98da1ea13fe456d01287d2cef9a79bfd7`.

## Summary Table

| Category | Suite / Check | Result | Evidence |
| :--- | :--- | :---: | :--- |
| **Workspace Build** | `cargo test --workspace --locked --offline` | **PASS** | 233 passed, 0 failed, 1 ignored (`synthetic_1000_window_latency`) |
| **Workspace Lints** | `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | **PASS** | 0 warnings, exit code 0 |
| **Formatting** | `cargo fmt --all --check` | **PASS** | Clean formatting across workspace |
| **QML Tests** | `tst_spatial_alpha.qml` | **PASS** | 17 passed, 0 failed (450ms, offscreen software) |
| **QML Tests** | `tst_phase_4_1_1.qml` | **PASS** | 48 passed, 0 failed (4.4s, offscreen software) |
| **QML Tests** | `tst_phase_4_1_2.qml` | **PASS** | 20 passed, 0 failed (1.2s, offscreen software) |
| **Private KWin Runner** | `tests/spatial/native.py` | **PASS** | 19 native assertions passed, virtual KWin 6.8.3, `CLEANUP {'hostEqual': True, 'changedKeys': [], 'remaining': [], 'exit': 0}` |
| **Native Integration** | `tests/phase-3-native/run_phase_3_10_native.py` | **PASS** | 8 invalidation scenarios passed (focus loss, window destruction, generation bump, desktop removal, screen lock, restart fencing, bootstrap token) |
| **Adversarial Suites** | `tests/adversarial/run_all_adversarial.py` | **PASS** | All 12 independent attack suites passed (13.46s) |
| **Historical Source Audit**| `tests/spatial/verify-source.py` | **PASS** | 143 backup files verified against archive manifest; zero active background writers |
| **Production Allowlist** | `tests/adversarial/test_production_allowlist_adversarial.py` | **PASS** | Exactly 5 read methods (`get_capabilities`, `get_controller_health`, `get_hierarchy`, `get_context_report`, `get_current_logical_context`) and 1 signal (`invalidated`); zero writable endpoints |
| **Host Isolation** | Hash verification of `~/.config/kwinrc`, `kglobalshortcutsrc`, host KWin PID 1727 | **PASS** | Pre- and post-test hashes match bit-for-bit; `~/.local/share/depthos` absent; host scripts clean |

## Detailed Attack Verification

### 1. Pointer Interactions
- **Single-Click vs Double-Click**: Single click selects the target card without changing hierarchy level. Double-click sequence invokes `pointerDrillDown()`, which enforces a 320ms debounce to prevent multi-level punch-through.
- **Rapid Clicking**: Tested 10 rapid repeated clicks on the same card within 100ms and alternating clicks across different cards; verified that selections update synchronously without queue congestion or false double-clicks.
- **Clicks During Animations**: Clicks delivered during 200ms scale transitions complete safely without misrouting or coordinate projection errors.
- **Keyboard Focus Sync**: Mouse selection immediately updates `worldIndex` / `workspaceIndex` / `windowIndex`; subsequent keyboard navigation continues seamlessly from the clicked card.
- **Search Palette**: Search palette retains active focus and text query across window deactivation and reactivation; dismissal cleanly returns focus to the spatial canvas.
- **Card Safety Constraints**: Window cards select only and never drill down or activate native windows; unavailable cards reject clicks and double-clicks.

### 2. Gesture Handling
- **Event Origin Classification**: Synthetic gesture calls (`pinchBegin`, `pinchUpdate`, `pinchEnd`) only modify internal presentation zoom levels; they cannot mint capability tokens or interact with the native compositor.
- **Pan Clamping**: Viewport pan offsets are strictly bounded within `[-panLimitX, 0]` and `[-panLimitY, 0]`; extreme deltas (`±1,000,000 px`) clamp safely without NaN or infinity.
- **Mixed Input & Reduced Motion**: Alternating wheel pan and keyboard navigation maintains visual continuity; reduced-motion mode executes instant transitions with 0ms duration.
- **Hardware Absence**: Complete absence of touchpad gesture events does not hang, deadlock, or crash the UI.

### 3. Observer Deployment
- **Script Identity Verification**: Distinct roles enforced for package ID `depthos-system-observer`, runtime session script `depthos-system-observer-session`, and standalone crate `depthos-observer`.
- **KWin Version Gating**: Installer script `install-observer.fish` strictly matches tested releases (`kwin 6.8.1`, `6.8.2`, `6.8.3`) and rejects unapproved versions (`6.8.0`, `6.8.4`, `7.0`, `-dirty`).
- **Lifecycle & Safety**: Installer refuses to overwrite existing destination packages; uninstaller strictly refuses removal while script is loaded in KWin (`isScriptLoaded == true`). Symlink package attacks are detected and aborted.
- **Privacy & Redaction**: `Snapshot::redact_titles()` replaces sensitive captions with `<redacted>`; windows marked with `excludeFromCapture` are protected.

### 4. Performance & Frame Pacing
- **Timestamp Distinction**: Documentation clearly differentiates render thread buffer swaps (`QQuickWindow.frameSwapped`) from compositor presentation timestamps (`wp_presentation`). Display budgets: 144 Hz @ 6.944ms, 60 Hz @ 16.667ms.
- **Idle Quiescence**: Swap events cease completely during static idle (17 frames observed during initialization burst, 0 frames while stationary).
- **Memory Footprint**: Offscreen UI process memory remains tightly bounded (`VmHWM` ~88 MiB, `VmRSS` ~88 MiB). Historical 522.96 MiB artifact clarified as legacy benchmark residue.

### 5. Security Invariants
- Production native authorization is unconditionally deny-only.
- Production D-Bus interface exports zero mutative methods.
- Zero host system modifications were performed during the entire review process.
