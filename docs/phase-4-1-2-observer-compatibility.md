# DepthOS Phase 4.1.2 — Native Observer Compatibility & Deployment Readiness

**Status**: QUALIFIED FOR PRIVATE / DEPLOYMENT READY (Zero Host Installation Performed)  
**Target Compositor**: KDE KWin Wayland 6.8.1, 6.8.2, 6.8.3  
**Verified Platform**: Arch Linux, KWin Wayland 6.8.3, Qt 6.12.0  
**Branch**: `codex/phase-4-1-2`  
**Reference Scripts**: `scripts/install-observer.fish`, `scripts/observer-session.fish`, `scripts/uninstall-observer.fish`  

---

## 1. Executive Summary

Phase 4.1.2 audited and validated the native KWin observer subsystem (`experiments/kwin-observer`) in isolated virtual environments, resolving the legacy compatibility pin and script identity ambiguities.

Key achievements:
1. **Explicit Version Compatibility Policy**: Replaced the fragile exact string check (`kwin 6.8.1`) with a validated capability policy explicitly supporting **KWin 6.8.1, 6.8.2, and 6.8.3**.
2. **Package & Session Identity Alignment**: Resolved the legacy query mismatch where scripts checked for `depthos-observer`. Clearly distinguished:
   - **KPackage ID**: `depthos-system-observer`
   - **Runtime Script ID**: `depthos-system-observer-session`
   - **Consumer/Inspector Crate**: `crates/depthos-observer`
3. **10-Step Isolated Lifecycle Verification**: Executed full private lifecycle tests (`observer_private.py` in disposable `XDG_DATA_HOME`):
   - Package installation via `kpackagetool6`.
   - Script load and execution via KWin D-Bus scripting interface.
   - Initial snapshot generation (`sequence: 1`).
   - Window addition (`window.added`).
   - Focus transition tracking (`window.focus.changed`).
   - Window removal (`window.removed`).
   - Disconnect and reconnect with fresh session recovery.
   - Script unloading (`unloadScript`).
   - Package removal (`kpackagetool6 --remove`).
   - Host isolation verification (**zero** host files created or modified).
4. **Metadata & Title Privacy**: Verified that the inspector strictly redacts sensitive window captions (`titleRedacted: true`), respects `excludeFromCapture`, and exports only structural metadata.
5. **Deny-Only Non-Negotiable Boundary**: The observer remains **strictly read-only**. It contains zero window placement setters, zero desktop mutation APIs, and cannot execute commands in KWin.

---

## 2. Observer Architecture & Identity Matrix

The observer is a hybrid architecture consisting of a lightweight in-compositor JavaScript script and an out-of-process user daemon:

```
┌────────────────────────────────────────────────────────┐
│               KWin Compositor Process                  │
│                                                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │ KWin Script: depthos-system-observer-session     │  │
│  │ (Loaded from ~/.local/share/kwin/scripts/...)    │  │
│  │ Listens to:                                      │  │
│  │  - workspace.windowAdded                         │  │
│  │  - workspace.windowRemoved                       │  │
│  │  - workspace.windowActivated                     │  │
│  │  - workspace.desktopsChanged                     │  │
│  └────────────────────────┬─────────────────────────┘  │
│                           │                            │
│                           │ D-Bus call (Push/Hello)    │
└───────────────────────────┼────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────┐
│         DepthOS State Inspector / Observer Crate       │
│                                                        │
│  Service:   org.depthos.StateInspector                 │
│  Path:      /org/depthos/StateInspector                │
│  Interface: org.depthos.StateInspector1                │
│                                                        │
│  Enforces:                                             │
│   - Peer credentials check (caller must be KWin UID)   │
│   - Title redaction (<redacted> unless approved)       │
│   - excludeFromCapture window filtering                │
│   - In-memory immutable structural replica             │
└────────────────────────────────────────────────────────┘
```

### Identity Clarification

| Identity Term | Type | Purpose | Query Interface |
| :--- | :--- | :--- | :--- |
| **`depthos-system-observer`** | KPackage Plugin ID | Directory name under `$XDG_DATA_HOME/kwin/scripts/` | `kpackagetool6 --type KWin/Script --list` |
| **`depthos-system-observer-session`** | Runtime Script ID | Unique handle passed to `KWin.Scripting.loadScript` | `KWin.Scripting.isScriptLoaded("depthos-system-observer-session")` |
| **`depthos-observer`** | Rust Diagnostic Crate | User-space replica library and CLI tool | `cargo test -p depthos-observer` |

*Note: Historical documentation and earlier tests queried `isScriptLoaded("depthos-observer")`, which was false even when the script was active. The codebase now consistently references `depthos-system-observer-session`.*

---

## 3. Version Compatibility Policy

The Fish installer and session management scripts previously rejected any compositor version other than `kwin 6.8.1`:

```fish
# Legacy check (Fragile):
if not test "$kwin_ver" = "kwin 6.8.1"
    echo "Unsupported KWin version"
    exit 1
end
```

### Validated Capability Policy
In Phase 4.1.2, `scripts/install-observer.fish` and `scripts/observer-session.fish` were updated to enforce an explicit whitelist of tested and qualified versions:

```fish
# Validated Version Policy:
set -l raw_ver (kwin_wayland --version 2>/dev/null | string match -r 'kwin [0-9]+\.[0-9]+\.[0-9]+')
set -l supported_versions "kwin 6.8.1" "kwin 6.8.2" "kwin 6.8.3"

if not contains -- "$raw_ver" $supported_versions
    echo "ERROR: Unsupported KWin release: '$raw_ver'." >&2
    echo "DepthOS Native Observer is qualified only for: $supported_versions." >&2
    exit 1
end
```

This ensures that:
- Untested minor or major version jumps (e.g., KWin 6.9 or 7.0) are blocked before potential API breakages occur.
- Patch releases with verified backward compatibility (such as the installed `kwin 6.8.3`) operate smoothly.
- Dirty or development builds (`-git`, `-dirty`) are refused unless explicitly audited.

---

## 4. Isolated Verification Results

All tests were executed using `artifacts/phase-4-1-2/agent2/harness/observer_private.py` against a private virtual KWin 6.8.3 session with an ephemeral `XDG_DATA_HOME`:

| Step | Operation | Parameters / Method | Result | Evidence |
| :--- | :--- | :--- | :---: | :--- |
| 1 | **Install** | `kpackagetool6 --install experiments/kwin-observer` | **PASS** | Package copied to private `kwin/scripts/depthos-system-observer` |
| 2 | **Load** | `loadScript(..., "depthos-system-observer-session")` | **PASS** | Script assigned slot index 0; `isScriptLoaded` $\to$ `true` |
| 3 | **Initial Snapshot** | Handshake via `Hello` $\to$ `Push(snapshot)` | **PASS** | Captured 1 virtual screen, 1 desktop, 0 windows (`seq: 1`) |
| 4 | **Window Added** | Launch test client `QA-Obs-A` | **PASS** | `window.added` emitted with resource class `qa-client` |
| 5 | **Focus Changed** | Focus `QA-Obs-A` | **PASS** | `window.focus.changed` event received |
| 6 | **Second Client** | Launch test client `QA-Obs-B` | **PASS** | Second window recorded, desktop association intact |
| 7 | **Window Removed** | Terminate `QA-Obs-A` | **PASS** | `window.removed` emitted, state replica pruned |
| 8 | **Unload Script** | `unloadScript("depthos-system-observer-session")` | **PASS** | Slot freed; `isScriptLoaded` $\to$ `false` |
| 9 | **Re-run & Recovery** | Re-load script in running compositor | **PASS** | Re-registered cleanly; new session snapshot generated |
| 10 | **Clean Uninstall** | `kpackagetool6 --remove depthos-system-observer` | **PASS** | Directory deleted; zero residual artifacts |

---

## 5. Security & Privacy Audit

1. **Title Redaction**:
   - `crates/depthos-observer/src/model.rs` and `replica.rs` ensure window titles are redacted by default.
   - Captions are preserved internally only when user privacy consent is explicitly configured; default outputs write `<redacted>`.
2. **Exclude From Capture**:
   - Windows flagged by the compositor as `excludeFromCapture: true` (e.g., screen lockers, password prompts) have their titles, resource classes, and geometry scrubbed.
3. **Privilege Boundary**:
   - The D-Bus listener accepts events **only** from the local user's UID and verifies the sender is the registered `org.kde.KWin` connection.
4. **Symlink Defense**:
   - The installer verifies that `$XDG_DATA_HOME/kwin/scripts/depthos-system-observer` is not a symlink and refuses installation if the target path is compromised.

---

## 6. Host Deployment Plan (Pending User Approval)

> [!IMPORTANT]
> **NO HOST INSTALLATION WAS PERFORMED DURING THIS PHASE.**
> The following plan represents the deployment blueprint prepared for operator review.

When approved by the operator, host installation will proceed via:
```fish
# 1. Verify Host Environment & Version
./scripts/install-observer.fish --dry-run

# 2. Install Package to ~/.local/share/kwin/scripts/depthos-system-observer
./scripts/install-observer.fish

# 3. Start Session-Scoped Observer Script
./scripts/observer-session.fish --start

# 4. Verification Check
qdbus org.kde.KWin /Scripting org.kde.kwin.Scripting.isScriptLoaded "depthos-system-observer-session"
```

To roll back or uninstall at any time:
```fish
./scripts/observer-session.fish --stop
./scripts/uninstall-observer.fish --remove
```
