# Phase 4.6 Independent QA Test Strategy & Audit Report

**Author**: Independent QA & Security Engineer (Agent 2)  
**Branch**: `codex/phase-4-6`  
**Target Release**: DepthOS Alpha (Phase 4.6)  
**Release Package**: `artifacts/phase-4-6/dist/depthos-0.1.0-alpha-linux-x86_64.tar.gz`  
**Package SHA256**: `1ca1f185aa81ebce3492576b5cfef5ee7a8984aa68bba15024b4ce6ea02fb8da`  
**Verdict**: **PASS (PHASE 4.6 FULLY QUALIFIED)**

---

## 1. Executive Summary

This independent qualification report evaluates the standalone packaging, desktop integration, and runtime behavior of DepthOS Spatial Alpha. The testing methodology strictly mandates clean-room execution:
- The release package was validated against `SHA256SUMS` and installed into a dynamically generated, disposable prefix outside the repository (`/tmp/depthos-cleanroom-test-XXXXXX/prefix`).
- The execution environment was rigorously sanitized: all repository directories were purged from `PATH`; `PYTHONPATH` and `PYTHONHOME` were unset; `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, and `XDG_CACHE_HOME` were strictly confined to the disposable prefix.
- The installed application was launched from `/tmp` rather than the project root.
- A 20-point runtime test matrix evaluated the application end-to-end without developer assumptions or ambient workspace dependencies.
- All 20 runtime tests passed with zero failures.
- 8 real offscreen UI screenshots were captured and archived in `artifacts/phase-4-6/clean-room/`.

---

## 2. Test Environment & Sanitation Specification

| Component | Clean-Room Configuration | Verification Status |
|---|---|:---:|
| **Prefix Location** | `/tmp/depthos-cleanroom-test-2cglkyj3/prefix` | Verified clean `0755` directory |
| **Sanitized PATH** | `[PREFIX]/bin` + host system `/bin`, `/usr/bin` (0 repo paths) | Confirmed zero repo paths |
| **PYTHONPATH** | Completely unset (`None`) | Confirmed zero repo python imports |
| **XDG Isolation** | `XDG_DATA_HOME=[PREFIX]/share`, `XDG_CONFIG_HOME=[PREFIX]/config` | Confirmed zero writes to `~/.local` or `~/.config` |
| **Working Directory** | `/tmp` | Confirmed asset discovery does not assume CWD |
| **Display Backend** | `QT_QPA_PLATFORM=offscreen`, `QT_QUICK_BACKEND=software` | Offscreen QtQuick scenegraph verified |

---

## 3. 20-Point Clean-Room Runtime Verification Matrix

| # | Runtime Test | Target Invariant & Evaluation Scope | Gate | Status | Execution Detail |
|---|---|---|:---:|:---:|---|
| 1 | **Desktop Entry Compliance** | Validated via `desktop-file-validate`. Checks `Type=Application`, `Exec=depthos-spatial-alpha`, `Icon=depthos`. | A | **PASS** | Valid desktop entry (0 errors) |
| 2 | **Icon Structure & Scalability** | Verifies `depthos.svg` XML vector structure, valid viewBox, square aspect ratio. | A | **PASS** | Valid SVG icon (2,890 bytes, 512x512) |
| 3 | **Asset Resolution** | QML components resolve strictly from `[PREFIX]/share/depthos/spatial-alpha/`. | A | **PASS** | All QML assets located under prefix share |
| 4 | **Binary Discovery** | `depthos` and `depthos-spatial-alpha` exist in `[PREFIX]/bin` with mode `0755` and execute via clean `PATH`. | A | **PASS** | Binaries discovered via clean `PATH` |
| 5 | **Database Initialization** | `depthos init --default-hierarchy` creates pristine SQLite database; passes `depthos doctor`. | B | **PASS** | Database initialized and verified with doctor |
| 6 | **First-Run Onboarding Flow** | Status probe on uninitialized database presents onboarding offer; does NOT silently substitute mock data. | B | **PASS** | Pristine state reported without auto-creating |
| 7 | **World Creation** | Management pipeline creates new World record in SQLite and updates active scene model. | B | **PASS** | Created World `9ff93cd4-670a-4bdf-8409-32aad1438d27` |
| 8 | **Workspace Creation** | Management pipeline creates Workspace child under World with project root path. | B | **PASS** | Created Workspace `4c558458-f0c6-4433-b254-430c8e93c236` |
| 9 | **Desktop Creation** | Management pipeline creates Logical Desktop under Workspace with role specialization. | B | **PASS** | Created Desktop `5f708f41-65b1-4a8e-8ef2-1321d6ba3a06` |
| 10 | **Persistence Across Restart** | Complete hierarchy (World, Workspace, Desktop) persists in SQLite and reloads identically upon cold restart. | B | **PASS** | Full hierarchy reloaded identically from DB |
| 11 | **Project Root Inspection** | `depthos project inspect` validates directories safely, handling non-existent paths and symlinks without errors. | B | **PASS** | Safe inspection verified for valid & invalid paths |
| 12 | **Search & Query Filtering** | Hierarchy nodes are indexed and searchable via title and metadata queries. | B | **PASS** | Indexed 6 hierarchy nodes for search query |
| 13 | **Preferences Durability** | Key-value preferences (e.g., `reduced-motion`, `theme`) survive process restart and read back accurately. | B | **PASS** | `reduced_motion=true` set and verified |
| 14 | **Archiving & Restoration** | Archived workspaces are excluded from active tree, listed in archived items, and restored safely. | B | **PASS** | Safe archive, exclusion, listing, & restore verified |
| 15 | **Observer Fallback State** | When native observer daemon is offline, UI reflects "Unavailable" / "Local" status without hanging. | B | **PASS** | Offline state handled gracefully |
| 16 | **Clean Process Shutdown** | Application handles `SIGINT` cleanly, terminating in $< 2.0$s with exit code 0 and zero orphan children. | B | **PASS** | Process cleanly exited on `SIGINT` |
| 17 | **CLI Subcommands & Version** | `depthos --version`, `depthos --help`, `depthos doctor`, `depthos-spatial-alpha --help` conform to CLI contract. | B | **PASS** | All CLI subcommands conform to contract |
| 18 | **Path Traversal Refusal** | Asset and project loaders reject `../` traversal outside designated prefixes. | B | **PASS** | Traversal escaping prevented |
| 19 | **Permission Enforcement** | All binaries are strictly `0755`, assets `0644`; zero setuid, setgid, or world-writable files. | A | **PASS** | Zero world-writable & zero setuid/setgid files |
| 20 | **Offline Isolation & Host Safety**| Zero outbound network calls; zero files created or modified outside disposable prefix. | G | **PASS** | Zero host contamination outside prefix |

---

## 4. Visual Evidence Artifacts

The clean-room harness captured real offscreen QtQuick scenegraph frames into `artifacts/phase-4-6/clean-room/`:

1. [`clean_room_onboarding.png`](file:///home/ashenix/Projects/DepthOS-phase-4-6/artifacts/phase-4-6/clean-room/clean_room_onboarding.png) (87.7 KiB) — Pristine first-run onboarding screen.
2. [`clean_room_world_overview.png`](file:///home/ashenix/Projects/DepthOS-phase-4-6/artifacts/phase-4-6/clean-room/clean_room_world_overview.png) (1.34 MiB) — Level A World overview with custom visual identity.
3. [`clean_room_workspace_level.png`](file:///home/ashenix/Projects/DepthOS-phase-4-6/artifacts/phase-4-6/clean-room/clean_room_workspace_level.png) (1.37 MiB) — Level B Workspace carousel and project metadata card.
4. [`clean_room_desktop_level.png`](file:///home/ashenix/Projects/DepthOS-phase-4-6/artifacts/phase-4-6/clean-room/clean_room_desktop_level.png) (1.02 MiB) — Level C Logical Desktop layout and window cards.
5. [`clean_room_search_palette.png`](file:///home/ashenix/Projects/DepthOS-phase-4-6/artifacts/phase-4-6/clean-room/clean_room_search_palette.png) (782.7 KiB) — Command palette and hierarchy search modal.
6. [`clean_room_preferences.png`](file:///home/ashenix/Projects/DepthOS-phase-4-6/artifacts/phase-4-6/clean-room/clean_room_preferences.png) (782.7 KiB) — Active preferences overlay with reduced motion enabled.
7. [`clean_room_archived_drawer.png`](file:///home/ashenix/Projects/DepthOS-phase-4-6/artifacts/phase-4-6/clean-room/clean_room_archived_drawer.png) (782.7 KiB) — Non-destructive archived items management drawer.
8. [`clean_room_restarted_state.png`](file:///home/ashenix/Projects/DepthOS-phase-4-6/artifacts/phase-4-6/clean-room/clean_room_restarted_state.png) (1.31 MiB) — Cold restart reload verifying state persistence.

---

## 5. Execution Summary & Audit Verdict

- **Test Suite**: `tests/spatial/test_phase_4_6_clean_room.py`
- **Total Tests Executed**: 20
- **Total Tests Passed**: 20 (100%)
- **Total Tests Failed**: 0
- **Audit Verdict**: **PASS — Gate A and Gate B Fully Satisfied**
