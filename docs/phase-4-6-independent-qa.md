# Phase 4.6 Independent QA Test Strategy & Audit Report

**Author**: Independent QA & Security Engineer (Agent 2)  
**Branch**: `codex/phase-4-6`  
**Target Release**: DepthOS Alpha (Phase 4.6)  
**Scope**: Clean-room packaging, clean disposable prefix installation, 0-repo runtime isolation, and 20-point functional qualification matrix.

---

## 1. Executive Summary

This independent qualification report evaluates the standalone packaging, desktop integration, and runtime behavior of DepthOS Spatial Alpha. The testing methodology strictly mandates clean-room execution:
- The release package is installed into a dynamically generated, disposable prefix outside the repository (`/tmp/depthos-cleanroom-test-XXXXXX`).
- The execution environment is purged of all repository paths: `PATH` contains no development tree directories; `PYTHONPATH` and `PYTHONHOME` are removed; `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, and `XDG_CACHE_HOME` are strictly confined to the disposable prefix.
- The installed application is invoked from an arbitrary working directory (e.g., `/tmp`) rather than the project root.
- A 20-point runtime test matrix verifies the application end-to-end without developer assumptions or ambient workspace dependencies.

---

## 2. Test Environment & Sanitation Specification

| Component | Clean-Room Configuration | Verification Method |
|---|---|---|
| **Prefix Location** | `/tmp/depthos-cleanroom-test-XXXXXX/prefix` | Dynamically created via `tempfile.mkdtemp` with `0755` permissions |
| **Sanitized PATH** | `[PREFIX]/bin` + host system `/bin`, `/usr/bin` (0 repo paths) | Regex check ensuring no substring of repo root appears |
| **PYTHONPATH** | Completely unset (`None`) | Validates Python imports only standard library and system site-packages |
| **XDG Isolation** | `XDG_DATA_HOME=[PREFIX]/share`, `XDG_CONFIG_HOME=[PREFIX]/config` | Prevents writing to `~/.local` or `~/.config` |
| **Working Directory** | `/tmp` | Verifies binary and QML asset discovery does not assume current working directory |
| **Display Backend** | `QT_QPA_PLATFORM=offscreen`, `QT_QUICK_BACKEND=software` | Offscreen QtQuick scenegraph rendering for headless deterministic verification |

---

## 3. 20-Point Clean-Room Runtime Verification Matrix

| # | Runtime Test | Target Invariant & Evaluation Scope | Gate | Status |
|---|---|---|:---:|:---:|
| 1 | **Desktop Entry Compliance** | Validated via `desktop-file-validate`. Checks `Type=Application`, `Exec=depthos-spatial-alpha`, `Icon=depthos`. | A | PENDING |
| 2 | **Icon Structure & Scalability** | Verifies `depthos.svg` XML vector structure, valid viewBox, square aspect ratio. | A | PENDING |
| 3 | **Asset Resolution** | QML components resolve strictly from `[PREFIX]/share/depthos/spatial-alpha/`. | A | PENDING |
| 4 | **Binary Discovery** | `depthos` and `depthos-spatial-alpha` exist in `[PREFIX]/bin` with mode `0755` and execute via clean `PATH`. | A | PENDING |
| 5 | **Database Initialization** | `depthos init --default-hierarchy` creates pristine SQLite database; passes `depthos doctor`. | B | PENDING |
| 6 | **First-Run Onboarding Flow** | Status probe on uninitialized database presents onboarding offer; does NOT silently substitute mock data. | B | PENDING |
| 7 | **World Creation** | Management pipeline creates new World record in SQLite and updates active scene model. | B | PENDING |
| 8 | **Workspace Creation** | Management pipeline creates Workspace child under World with project root path. | B | PENDING |
| 9 | **Desktop Creation** | Management pipeline creates Logical Desktop under Workspace with role specialization. | B | PENDING |
| 10 | **Persistence Across Restart** | Complete hierarchy (World, Workspace, Desktop) persists in SQLite and reloads identically upon cold restart. | B | PENDING |
| 11 | **Project Root Inspection** | `depthos project inspect` validates directories safely, handling non-existent paths and symlinks without errors. | B | PENDING |
| 12 | **Search & Query Filtering** | Hierarchy nodes are indexed and searchable via title and metadata queries. | B | PENDING |
| 13 | **Preferences Durability** | Key-value preferences (e.g., `reduced-motion`, `theme`) survive process restart and read back accurately. | B | PENDING |
| 14 | **Archiving & Restoration** | Archived workspaces are excluded from active tree, listed in archived items, and restored safely. | B | PENDING |
| 15 | **Observer Fallback State** | When native observer daemon is offline, UI reflects "Unavailable" / "Local" status without hanging. | B | PENDING |
| 16 | **Clean Process Shutdown** | Application handles `SIGINT` cleanly, terminating in $< 2.0$s with exit code 0 and zero orphan children. | B | PENDING |
| 17 | **CLI Subcommands & Version** | `depthos --version`, `depthos --help`, `depthos doctor`, `depthos-spatial-alpha --help` conform to CLI contract. | B | PENDING |
| 18 | **Path Traversal Refusal** | Asset and project loaders reject `../` traversal outside designated prefixes. | B | PENDING |
| 19 | **Permission Enforcement** | All binaries are strictly `0755`, assets `0644`; zero setuid, setgid, or world-writable files. | A | PENDING |
| 20 | **Offline Isolation & Host Safety**| Zero outbound network calls; zero files created or modified outside disposable prefix. | G | PENDING |

---

## 4. Visual Evidence Artifacts

The clean-room harness captures real offscreen QtQuick scenegraph frames into `artifacts/phase-4-6/clean-room/`:

1. `clean_room_onboarding.png` — Pristine first-run onboarding screen.
2. `clean_room_world_overview.png` — Level A World overview with custom visual identity.
3. `clean_room_workspace_level.png` — Level B Workspace carousel and project metadata card.
4. `clean_room_desktop_level.png` — Level C Logical Desktop layout and window cards.
5. `clean_room_search_palette.png` — Command palette and hierarchy search modal.
6. `clean_room_preferences.png` — Active preferences overlay with reduced motion enabled.
7. `clean_room_archived_drawer.png` — Non-destructive archived items management drawer.
8. `clean_room_restarted_state.png` — Cold restart reload verifying state persistence.

---

## 5. Execution Summary & Audit Verdict

*Note: Formal audit verdict and gate ratings will be populated upon execution against Agent 1's packaging commit.*
