# Phase 3.7 validation

Validation uses the independent `codex/phase-3-7` checkout with reviewed Phase 3.6 commit `4edfed832e08a41308772830f47824a3e42ee599` as the reference. Logs and raw timing samples are ignored under `artifacts/phase-3-7/review/` with owner-only access.

## Source qualification

Independent checks recorded KWin package 6.8.3-1, executable SHA-256 `33350d420b7b196c1e1f90ffa8c1280b4b3102d4e4b60e1929bad525be93df5e` and ELF build ID `6eb58cf1f8d875ef625487ec2f404e8cc969c517`. The upstream 6.8.3 archive SHA-256 is `fd5cb783f220ea4526a9afee2f72b582c3e87ef7f91b19bc5c7a6662ffe7fbdc` and matches the distro recipe. The recipe SHA-256 `d6023831666fecbe025107a305f12cca4d2751959e97e8f97a7e3126db013616` matches the cached package metadata. Runtime proof for an experimental build requires evidence from the running private process.

## Host isolation

The inherited launcher compares host snapshots through read-only D-Bus and file reads. It requires a fresh session bus with only the bus daemon initially registered, a distinct bus ID, private absolute runtime and XDG roots, and a private Wayland socket. The worker starts in a disposable process group. Its explicit environment excludes host display and session variables and directs system D-Bus to a non-socket path. Cleanup checks process-group membership before deleting the temporary directories. These properties cover the owned fixture processes. Host native window inventory and focus are outside the snapshots. Phase 3.6 observed differences in current desktop and effects during some runs. The comparison remains strict and these findings remain in the evidence without causal attribution.

On the inherited fixture, an injected wait timeout after the private Wayland socket appeared passed the process and path cleanup assertions and matched the sampled host fields. An injected preview-adapter failure also drained the group and removed the runtime and XDG roots. Its first two runs failed the strict host comparison, first on effects and then on `effects.activeEffects` and the monitored configuration-content group. The host `kwinrc` mtime fell within the second control window. The preserved snapshots identify a difference without attributing it to a process. A repeat passed both cleanup and sampled host comparison. The fixture never restores host state. Raw failure and repeat logs record each outcome.

## Production regression and standalone model checks

`cargo build --workspace --locked --offline`, `cargo test --workspace --locked --offline`, `cargo fmt --all --check`, and `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` passed. The suite reported 128 passed and one ignored. It includes the controller shutdown regression and synthetic consume/count behavior. `cargo test --locked --offline --test spatial_bridge --test security_boundary` passed the production read-only D-Bus method allowlist and refusal of forged claims by same-UID private bus clients.

`python3 tests/phase-3-native/authority-kernel.py` passed. An independent build of its synthetic C++ executable with GCC AddressSanitizer and UndefinedBehaviorSanitizer (`-fsanitize=address,undefined -fno-sanitize-recover=all -fno-omit-frame-pointer`) also passed. The kernel is standalone Qt Core. These results supply model evidence for deadline edges, thread affinity, irreversible explicit invalidation, wrong-thread refusal, cancellation, one-use counting and no retry after a lost receipt.

Offscreen `qmltestrunner -input tests/phase-3` passed eight QtTest results. `python3 tests/phase-3-native/run.py` against the installed KWin passed the preview lifecycle, eight Wayland QtTest results, sampled host equality, and private process and path cleanup. This is a baseline control.

## Offscreen performance

Five alternating runs per checkout used the 301-row, 1,000-window fixture with debug Rust, private D-Bus, production QML and offscreen software rendering. The control was a fresh archive and locked offline build of `4edfed832e08a41308772830f47824a3e42ee599`. The campaign samples used the same production source and fixture in the independent checkout. `python3 tests/spatial/performance.py --output <sample>` ran with host display and bus addresses removed and fresh XDG roots. All ten runs passed and each had zero geometry-only invalidations. Local workstation load continued, including part of a private fixture cleanup control. The sample size and timing variation do not establish a causal change.

| Metric | Control median / p95 / max | Campaign median / p95 / max | Samples each |
| --- | --- | --- | ---: |
| Initial snapshot to QML (ms) | 410.53 / 414.49 / 414.49 | 412.58 / 425.35 / 425.35 | 5 |
| Native identity removal to QML (ms) | 412.89 / 429.46 / 429.46 | 420.09 / 425.35 / 425.35 | 5 |
| Paged hierarchy fetch (ms) | 9.69 / 13.91 / 14.48 | 9.03 / 12.39 / 13.56 | 50 |
| Controller RSS (KiB) | 23,988 / 24,080 / 24,080 | 20,920 / 20,980 / 20,980 | 5 |
| Idle CPU (% of one core) | 1.66 / 2.00 / 2.00 | 2.00 / 2.33 / 2.33 | 5 |

The p95 uses nearest rank. The benchmark contains synthetic observations and no native compositor. It does not measure native dispatch, input or prompt latency.

## Full compositor runtime and correction evidence

The full private KWin build produced `kwin_wayland` and `libkwin.so.6.8.3` from the pinned patched source with the experimental compile flag. The initial main-executable compile failed on a missing Plasma Activities include path. The patch added its conditional target dependency and the incremental build passed. Installed and build-specific native plugin ABI differences then caused a startup crash in installed `nightlight.so`. The reviewer inspected only the exact private executable's recorded core and saved a restricted backtrace. The [security review](phase-3-7-security-review.md) describes the correction and evidence.

The corrected launcher uses a disposable hash-matching copy of the private executable, private native-plugin paths and a per-executable `qt.conf`. It checks the running executable via `/proc/PID/exe` and validates the private mapped KWin library and its digest. `python3 tests/phase-3-native/run.py --experiment build <absolute-binary> <sha256>` passed six native guarded proposals, immediate focus retirement, window eligibility/lifetime retirement, monotonic deadline expiry, destination topology retirement and eight Wayland QtTest results. The sampled host fields matched, and the owned process group and private paths were removed.

The first synthetic-mode run recorded `synthetic first=counted replay=denied count=1`, with eight QtTest results and sampled host equality. It then failed the independent native-focus assertion because the first proposal had already been consumed before focus changed. The reviewer preserved that assertion and added two owned surfaces after the expiry case. The launcher waits for a native pending observation before showing the next surface. This supplies an actual native focus-retirement opportunity in the counting run. An intermediate attempt to reorder existing preview cases exposed a strict event-order failure and was replaced by this separate surface pair. The corrected run passed eight native guarded proposals, one count, replay denial, eight QtTest results, sampled host comparison and cleanup.

`python3 tests/phase-3-7-native/test-launcher.py` passed deterministic native-transcript terminal-sequence and restricted-artifact checks. The reviewed source ABI fallback passed before the full build and supplied six guarded native observations. A native EIS device, remote/portal input, accessibility input and qualified local physical input have not been injected or classified at runtime in this campaign. The exact patched EIS code compiled in the private build. Native prompt integrity, real lock transitions and multi-seat behavior retain unsupported classifications. The full KWin build was compiled without sanitizer instrumentation. ASan/UBSan results apply to the standalone synthetic kernel.

The lead repeated the build and synthetic modes sequentially using the integrated launcher and the exact hashes in the [patch report](phase-3-7-kwin-patch.md). Both passed the added pre-teardown library/plugin mapping check, in addition to the startup checks. Build mode produced six guarded proposals and no count; synthetic mode produced eight guarded proposals, one count and denied replay. Each passed eight Wayland QtTest results, the sampled host comparison and private cleanup. The lead also repeated the locked offline Rust suite (128 passed, one ignored), formatting, strict Clippy, the standalone C++ check and the launcher tests. Production sources and the D-Bus method surface remain unchanged. Final raw evidence is under owner-only `artifacts/phase-3-7/lead-build/`.

The lead's separate installed-ABI plugin control also passed: six guarded proposals, zero counts, eight Wayland QtTest results, matched host samples and private cleanup. Strict full Git integrity passed on the independent checkout. A read-only repeat on the original shared store still reported the same empty tool checkpoint described in the [Git preflight](phase-3-7-git-integrity.md).
