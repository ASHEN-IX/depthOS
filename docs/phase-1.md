# POC-01 engineering report

## POC-02G — 2026-11-16

**KWIN 6.8.3 ISOLATED QUALIFIED; PHASE 1 PARTIALLY ACCEPTED.** [POC-02G](poc-02g.md) adds a release-specific API matrix, 104 passing local checks and 72 passing final isolated-native assertions. Native textures, dynamic windows, keyboard/mouse, focus/typing recovery, policy fallbacks, contention, bounded cycles and watchdog cleanup pass. Production source and the exact host baseline are unchanged. Installed 6.8.3 / running 6.8.1 still blocks host preparation; isolated qualification grants no host-session acceptance or portal authorization. Presentation timing/hardware and host-specific gates remain pending. Phase 2/POC-03 remains on hold.

## POC-02F status — 2026-11-16

**PHASE 1 PARTIALLY ACCEPTED.** The [host authorization attempt](poc-02f.md) timed out without an OS portal Response. Host-native input/focus acceptance is BLOCKED, not inferred from the historical isolated PASS. All 12 cleanup gates, original-window properties and process/session removal pass; 85 local tests pass. The retained POC-02E marker PNG is complete on independent decode, correcting its earlier partial-file assessment. Host software sign-off and presentation timing remain pending. Do not begin Phase 2.

## POC-02E status — 2026-11-16

**PHASE 1 PARTIALLY ACCEPTED.** [POC-02E](poc-02e.md) supplies real EIS/libei keyboard and pointer acceptance inside isolated KWin, including navigation, control clicks, local selection invariants, Escape at each level and exact marker typing recovery. The final run passed 45 native assertions and exact host/original-window comparisons; 77 local regressions pass. Sampled recordings and short capture-paused CPU/RSS measurements are retained. This completes the isolated injected-input scope, not host-session or physical-device sign-off. Presentation timing and protected-client native capture remain pending. Hold Phase 2; no new product feature was implemented.

## POC-02D status — 2026-11-16

**PHASE 1 PARTIALLY ACCEPTED.** The [POC-02D report](poc-02d.md) verifies physical Escape on the dynamic overview, current refused-start recovery and complete cleanup. The exact input marker was never acknowledged in the designated test program; the remaining physical groups therefore were not run. Current regressions total 73 passes. Existing POC-01 and custom effects are preserved, and no Phase 2 implementation began. Critical ordinary-input restoration remains pending.


## POC-02C status — 2026-11-16

**PHASE 1 PARTIALLY ACCEPTED.** See [poc-02c.md](poc-02c.md) for the latest stabilization run, native depth-level screenshots, bounded 120-cycle result, startup-refusal guard and exact rollback verification. Physical keyboard/mouse delivery and normal input restoration were deferred and remain critical sign-off gates. Programmatic reduced motion is verified; actual frame pacing is not. POC-03 was not begun. The historical POC-01 findings below remain scoped to their original runs; the new startup guard belongs to the dynamic overview.


**NATIVE LIFECYCLE AND STATIC RENDERING PASSED — PHYSICAL INPUT PENDING** — updated 2026-11-16.

The [complete live acceptance report](live-acceptance.md) supersedes historical runtime status below. A separate copied package completed 30 native cycles, rapid toggles and reversal; scoped logs establish initialization/delegates/animation completion. The preserved original package supplied a real Spectacle capture showing all expected text. Both were unloaded and the original package/binding restored exactly. No physical input result was inferred from callbacks or screenshots.

## Later read-only evidence observed during POC-02A

On 2026-11-15, the current user-session KWin journal contains the corrected effect's shortcut callback, `screen delegate created 1920x1080`, `visible request=true`, entrance completion, exit completion, and `visible request=false` across multiple cycles (including entries at 21:46 local time). The current installed effect source matches the repository and `isEffectLoaded` is true. The POC-02A task did not load, reload, activate, or otherwise modify this effect. This later evidence supersedes the earlier cache-blocked runtime state below, but does not supply independent visual acceptance, physical mouse/Escape/keyboard delivery testing, or full stress acceptance. POC-01 remains not fully approved.

## Approved live attempt

The user explicitly approved the scoped update and live test. The original installed 0.1.0 package was copied and byte-verified at `/tmp/depthos-approved-live-dvkzd8mu/depthos-spatial-poc`. KPackage successfully upgraded only DepthOS to 0.1.1, and `diff -ru` confirmed installed files exactly match the validated source. No persistent enabled preference was changed.

Session loading returned `true`, and the shortcut action was present. Invoking only `depthos-spatial-poc-toggle` through KGlobalAccel reached delegate creation, but DepthOS did **not** enter `activeEffects`. The live journal reported the original Bound-context error despite the installed Unbound QML. New initialization/callback logs were absent. This means the updated source was not what the shared QML engine executed; package equality alone does not establish running-code equality.

The test aborted at the first failed activation. A `finally` cleanup unloaded only DepthOS, and `isEffectLoaded` returned `false`. Planned further cycles and rapid toggles were **not executed**. No visual/physical input tests were possible: desktop screenshot/input-control tools are unavailable here, and activation itself failed before those checks. The loaded-effects list and hashes of all three existing custom effect packages remain identical to the pre-test baseline.

KWin 6.8.1 `ScriptedEffectLoader::loadDeclarativeEffect()` uses the persistent scripting `QQmlEngine` and the same absolute `contents/ui/main.qml` URL. Neither effect loading nor unloading clears that engine's component cache. No public, per-effect cache-refresh D-Bus API was identified. Broad script/effect changes, private API injection, cache deletion, compositor restart, logout, and reboot were not used as workarounds. The current fixed package remains installed and unloaded; a fresh-session retest is required when a fresh session occurs normally.

The offscreen C++ probe was extended to reproduce same-URL cache retention: after replacing a temporary QML file, a second component in the same engine still reads the original revision, while a fresh engine reads the updated revision. That cache reproduction, the child-context regression/Bound negative control, and all ten QtTest checks pass. This is supporting evidence for the reload limitation, not a successful live test of the corrected renderer.

The exact scoped failure log is `/tmp/depthos-approved-live-dvkzd8mu/live-test.log`. The original package can be restored using the README's fixed-ID remove/reinstall rollback sequence, but it contains the known Bound defect. The approved immediate rollback (unloading) has already completed. The temporary backup/log may be cleared on reboot.

**Live acceptance result: activation FAILED in the existing session due to stale compiled QML; fixed-code live behavior remains unverified.** The historical pre-approval investigation follows.

## Activation failure: diagnosis and repair

The user reported that the original package was supported and loaded (`true`), but the physical shortcut did not show the overlay. Before editing, the installed metadata/QML matched the repository byte-for-byte. The current diagnostic snapshot found DepthOS **unloaded**, the action `depthos-spatial-poc-toggle` retained by KGlobalAccel with key `385876026` (`Meta+Ctrl+Shift+F11`), and the persistent enabled key unset. Active effects were `blur` and `wallpaper-style-transparency`; no active Overview was observed. These are separate facts: retained registration does not prove a live callback, and the enabled preference does not determine whether a session-loaded scene is visible.

**Confirmed defect:** production QML specified `pragma ComponentBehavior: Bound`. KWin 6.8.1 `QuickSceneEffect::addScreen()` creates a new child `QQmlContext` and instantiates the delegate through `QQmlIncubator`. A new offscreen C++ probe using that exact public Qt creation pattern failed against the original production QML with:

```text
Cannot instantiate bound component outside its creation context
```

The earlier fixture used QML `delegate.createObject()`, which preserves the original creation context and missed this incompatibility. The smallest correction is an explicit **Unbound** pragma; no new package metadata, persistent-enable change, separate KWin script, or replacement shortcut API is required. After the change the child-context probe succeeds. A Bound negative control must reproduce the original error on every test run. This confirms an actual initialization defect, but the live session journal did not capture the user's failed attempt: additional physical-key issues cannot yet be ruled out.

Scoped POC logs now identify root initialization, shortcut callback, visibility requests, per-screen delegate construction, and animation completion. `scripts/trigger-poc.fish --toggle` invokes only the existing DepthOS KGlobalAccel action after checking that the effect is loaded and the action is recorded. This supported development trigger bypasses physical keys without exposing arbitrary commands. It exercises the same callback; a public remote setter for the individual QML root's visibility was not found. `/Effects.toggleEffect` toggles plugin loading and is unsuitable for opening the overlay.

The inspection script now distinguishes loaded/active effects, recorded shortcut ownership, preferred-key availability, and persistent configuration, using explicit D-Bus property reads. The renderer still uses supported `visible` activation/deactivation, a focused per-screen delegate, and deferred exit teardown. No window or workspace operations were added. `WindowThumbnail` was verified in the v6.8.1 source but remains unused in POC-01.

### Repair verification

- Metadata, package recognition, QML parsing, and syntax checks for all four fish scripts pass.
- The C++ child-context probe passes with Unbound; Bound reproduces the original error as an expected negative control.
- QtTest: eight functional cases plus initialization/cleanup, **10 passed, 0 failed**. Existing tests were preserved; added cases invoke the production shortcut handler signal and verify cancellation/recovery after external hiding. Unexpected QML warnings now fail fixture tests.
- Existing cases still cover entrance/exit, Escape/selection, outside clicks, exit reversal, 30 rapid toggles, five repeated cycles, and geometry changes. Each case destroys the production root and view during cleanup. These remain **mock-KWin offscreen checks**, not real global shortcut or compositor tests.
- At the pre-approval checkpoint the installed package still had the former Bound QML. It has since been upgraded to 0.1.1 as recorded above. No additional metadata fields or package files are needed for activation. The version bump permits a supported update from installed 0.1.0: KPackage 6.31.0 rejects equal-version upgrades and downgrades.
- Source/installed comparison and hashes of existing custom effect files are checked separately from the fixture suite. Other loaded effects are left as found; no preferences were changed.

Modified for the repair: production `main.qml` and package version in `metadata.json`, `scripts/inspect-kwin.fish`, both existing test files, and README/phase-1/compatibility documentation. Added `scripts/trigger-poc.fish` and `tests/delegate-context.cpp`. No unrelated files were changed.

### Approved live-test plan

The [README update sequence](../README.md#activation-repair-approved-update-and-diagnostic-test) provides exact fish commands: back up only the installed DepthOS package, unload only DepthOS if needed, upgrade with an explicit user-local package root, compare bytes, session-load, inspect registration, invoke the fixed diagnostic action, test physical keys/mouse/Escape/repeated transitions, and finally unload. Do not persistently enable it during diagnosis. Immediate rollback is an isolated `unloadEffect depthos-spatial-poc`; package rollback removes only the unloaded DepthOS package and reinstalls its saved backup, leaving the former defective package unloaded. A downgrade through `--upgrade` would be rejected. Approval is required **before** any upgrade/load/activation/unload operation.

PASS still requires observing the updated installed code, initialization, physical shortcut, real pixels/animations, input capture and release, repeated activation, unchanged windows/other effects, and safe unloading. No live acceptance is inferred from fixture results.

The sections below retain the initial implementation report and its historical test baseline.

## Scope and decisions

Implemented an isolated `depthos-spatial-poc` declarative effect with the requested navy/slate/violet visual palette, centered Projects card, responsive logical layout, entrance scale 0.75→1.0, opacity and lift over 300 ms, and a 230 ms exit toward 0.85 scale. The title and Escape hint fade with the card. Left-click, Space, and Enter toggle preview selection. Escape, outside clicks, and the same configurable shortcut request dismissal.

KWin's public `SceneEffect` replaces the rendered scene and owns input capture. `ShortcutHandler` supplies KDE shortcut configuration. A shared root owns the animation lifecycle for all screen delegates. Reversal stops the previous animation, retains interpolated values, and prevents an obsolete exit completion from hiding a reopened view. The effect becomes invisible only after exit finishes. These Qt Quick behaviors passed fixture tests; real KWin timing/input and visual quality remain untested.

The API audit preceded implementation. Exact version/source findings and official links are in [compatibility.md](compatibility.md). The fullscreen scene replacement means no live desktop shows through the fade. Real window surfaces are deliberately deferred to POC-02. Existing windows, Activities, desktops, and effects have no mutation paths in the effect code.

Installation/removal require explicit flags, use one fixed package ID and user-local root, and never load, enable, disable, upgrade, or restart anything implicitly. The installer refuses an occupied/unknown default shortcut or an existing DepthOS directory. Removal refuses a loaded effect or unknown load state. No privileged commands or system package installations are needed.

## Files delivered

| File | Purpose |
| --- | --- |
| `README.md` | Inspection, installation, configuration, acceptance tests, troubleshooting, rollback |
| `LICENSE` | Explicit pending license decision, no fabricated license grant |
| `.gitignore` | Editor/build/artifact exclusions |
| `docs/architecture.md` | Current design and future separation of responsibilities |
| `docs/phase-1.md` | This report and acceptance status |
| `docs/compatibility.md` | Machine audit, source evidence, limitations |
| `experiments/kwin-effect/metadata.json` | Disabled-by-default KWin effect package |
| `experiments/kwin-effect/contents/ui/main.qml` | Overlay, transitions, shortcut, local UI input |
| `scripts/inspect-kwin.fish` | Read-only version/package/shortcut/D-Bus inspection |
| `scripts/install-poc.fish` | Explicit user-local installation without automatic activation |
| `scripts/uninstall-poc.fish` | Explicit removal of the unloaded DepthOS package only |
| `scripts/trigger-poc.fish` | Approved development-only invocation of the fixed DepthOS action |
| `tests/validate-poc.py` | Metadata/parser/script checks and isolated Qt Quick test runner |
| `tests/tst_lifecycle.qml` | Production-QML lifecycle and local input fixture tests |
| `tests/delegate-context.cpp` | Offscreen child-context incubation regression probe |

No remote publishing or pushing is part of this task. No service, database, Rust project, Tauri application, or AI integration was added. Python is only an optional development test runner, not an effect dependency or service.

## Initial implementation verification results (before activation repair)

| Test | Result and evidence |
| --- | --- |
| Target environment audit | Confirmed KWin/Plasma 6.8.1 Wayland, Qt 6.12.0, Intel Mesa; live D-Bus interfaces introspected |
| Shortcut conflict query | KGlobalAccel reported `true` (available), Qt key `0x1700003a`; registration not executed |
| Metadata contract | JSON parsed; fixed ID, package structure, disabled default, declarative API, and no unapproved license claim checked |
| Package recognition | Installed KPackage tool accepted `--show` on the source directory and reported the correct ID; actual installation remains pending |
| Fish scripts | `fish --no-execute` succeeded for all three; mutation scripts reviewed for isolated fixed-ID operations; installation/removal branches not executed |
| Non-mutating script entry points | Both installation/removal `--help` paths succeeded; inspection script completed without mutation |
| QML syntax | Installed `qmlformat` parsed production QML successfully |
| Full QML imports | Incomplete: standalone `qmllint` cannot resolve compositor-provided `org.kde.kwin`; no clean full-import claim |
| Qt Quick fixture suite | Six functional tests plus init/cleanup: **8 passed, 0 failed**, QtTest 6.12.0, offscreen/software backend |
| Fixture coverage | Entrance/exit endpoints and deferred hide; exit reversal; 30 rapid toggles; card selection + Space + Escape; outside click + selection reset; five repeated cycles with logical resizing |
| Live installation, shortcut, opening/closing, input release | **PENDING**, not executed |
| Live repeated rendering stability, unchanged application behavior | **PENDING**, not executed |
| Live disabling and uninstall | **PENDING**, not executed |
| Final effect-state comparison | Loaded-effects list exactly matched the read-only audit baseline; DepthOS remained unloaded |

The fixtures run unchanged production QML against two clearly labeled fake KWin types. No global shortcuts are registered and no live compositor effect is loaded. An initial harness issue parented the UI under QtTest's invisible TestCase; moving it to the visible root fixed the fixture, after which all tests passed. This is not evidence of compositor input delivery.

## Acceptance gates still requiring approval and manual testing

Follow the exact sequence and fish commands in [README.md](../README.md): install locally, session-load only DepthOS, verify the preferred/custom shortcut and all dismissal paths, check card animation quality, run 30 normal cycles plus rapid interruptions, compare window/desktop/Activity/effect state, then disable/unload/remove and confirm the session remains functional. Multi-output, scaling, lock/unlock, suspend, and geometry tests extend the basic acceptance matrix.

POC-01 is not marked PASS until real installation, input capture/release, animation rendering, repeated activation, and clean removal have been observed. No installed effect or global configuration was changed during implementation. Existing custom packages were only read; no application window or workspace operation was performed.

## Known limitations and next investigation

The card is a placeholder with transient selection. No real navigation, desktop ownership, previews, persistence, or live-desktop crossfade exists. The effect is source-audited for 6.8.1 only. Competing fullscreen effects can reject startup; explicit toggling off and retrying is documented. The source audit establishes API support, not session stability or frame timing.

Recommended POC-02, after POC-01 acceptance: investigate public `WindowThumbnail` rendering for an existing window, then inactive-desktop surfaces, filtering, damage/repaint behavior, and multi-output/scale behavior. Keep that experiment independently reversible and initially read-only with respect to window/workspace state. Do not introduce a controller/database until surface behavior and ownership requirements are understood. POC-02 has not been started.
