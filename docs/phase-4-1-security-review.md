# DepthOS Phase 4.1: Spatial Workspace Alpha Independent Security Review

**Reviewer & Security Lead**: DepthOS Independent UX, Performance & Security Reviewer (Agent 2)  
**Workspace**: `/home/ashenix/Projects/DepthOS-phase-4-1`  
**Branch**: `codex/phase-4-1`  
**Baseline Commit**: `f20aa3d` (Phase 3.10.1 accepted security & native integration)  
**Evaluated Commit**: `6e7c46c6ec52f8c1892994a513aa52221f4ce27a` (Spatial Workspace Alpha UX)  
**Canonical Repository**: `/home/ashenix/Projects/DepthOS` (`main` untouched)  
**Date**: November 2026  

---

## 1. Executive Summary

Phase 4.1 introduces the **Spatial Workspace Alpha**, providing an interactive 3D spatial user interface running over native Wayland desktop environments. As established in Phase 3.10.1, expanding DepthOS into rich visual surfaces introduces distinct security and privacy boundaries that must be independently audited and proven tamper-resistant.

The core mandate of this independent security review is to guarantee:
1. **Zero Native Placement Setters**: The spatial UI exposes zero capabilities to move real windows, switch native virtual desktops, or modify native compositor states.
2. **Strictly Read-Only D-Bus Interface**: The session bus endpoint `org.depthos.Controller1` exposes zero writable methods.
3. **Deny-Only Action Authorization**: No automatic, implicit, or synthetic consent tokens are minted or consumed in the visual layer.
4. **Deterministic Fail-Closed Observation Badging**: Desynchronized, gapped, or offline observations are immediately downgraded to `Stale`, `Reconnecting`, or `Unavailable`. Stale data is **never** presented as `Live`.
5. **Window Identity & Privacy Safeguards**: Window captions default to redacted application identities; control character and BiDi injection attacks are stripped before rendering.
6. **Same-User Peer Authentication**: All D-Bus queries strictly enforce kernel `SO_PEERCRED` / `get_connection_unix_user` EUID checks.

All security invariants have been verified through static code audit, interface inspection, and automated adversarial regression testing.

---

## 2. Interface Audit: `org.depthos.Controller1`

The controller interface exposed by `src/spatial/mod.rs` was audited for method signatures, access controls, pagination limits, and mutability.

### Complete Method Inventory

| Method Name | Signature | Access Control | Mutability | Bounds & Limits |
| :--- | :--- | :--- | :--- | :--- |
| `GetCapabilities` | `() -> (s)` | Same-User EUID | **Read-Only** | Returns static capabilities advertising `readOnly: true`, `assignmentMutations: false`. |
| `GetControllerHealth` | `() -> (s)` | Same-User EUID | **Read-Only** | Returns controller lifecycle and observation health snapshot. |
| `GetHierarchy` | `(u, s) -> (s)` | Same-User EUID | **Read-Only** | Paginated (max 64 items/page, max 4,096 total rows, max 128 KiB response). |
| `GetContextReport` | `(u, s) -> (s)` | Same-User EUID | **Read-Only** | Paginated window context report (max 64 items/page, max 128 KiB response). |
| `GetCurrentLogicalContext` | `() -> (s)` | Same-User EUID | **Read-Only** | Returns client selection ownership (`owner: "client"`). |
| `invalidated` | Signal `(s, u)` | Controller Output | **Signal Only** | Notifies UI clients of data revisions. Accepts zero incoming parameters. |

### Verification Findings
- **Zero Writable Endpoints**: There are **zero methods** accepting mutation commands, window assignment requests, or desktop creation commands over D-Bus.
- **Strict Peer Verification**: Every method calls `authorize(conn, &h)`:
  ```rust
  let uid = DBusProxy::new(conn).await?.get_connection_unix_user(sender.clone().into()).await?;
  if uid != rustix::process::getuid().as_raw() {
      return Err(Error::AccessDenied("Same-user reads only".into()));
  }
  ```
  Cross-user access, even on shared or misconfigured D-Bus sessions, is completely denied at the kernel level.
- **DoS & Memory Exhaustion Defenses**:
  - `MAX_ROWS` is clamped to 4,096 items.
  - Serialized page envelopes exceeding 128 KiB immediately trigger `Error::LimitsExceeded`.
  - Requests are concurrency-gated via `Semaphore::new(8)` to prevent controller starvation.

---

## 3. Native Compositor Isolation Audit

A thorough audit of `experiments/spatial-alpha/` and `src/` verified the complete absence of compositor mutation vectors:

1. **Zero KWin Scripting Setters**:
   - Grep and AST inspection confirmed zero invocations of:
     * `client.desktop = ...`
     * `client.frameGeometry = ...`
     * `workspace.activeWindow = ...`
     * `workspace.currentDesktop = ...`
     * `workspace.sendClientToDesktop(...)`
2. **Visual Viewport Independence**:
   - Navigation keys (`H`/`L`, `J`/`K`, `Enter`, `Esc`, `[`/`]`) strictly move client-side QML camera coordinates (`cameraX`, `cameraY`, `currentScale`, `currentLevel`).
   - The native Wayland compositor virtual desktop and window geometry remain 100% untouched.
3. **Absence of Action Authorization Emitters**:
   - No production consent tokens (`Consent`) are constructed or issued in `experiments/spatial-alpha/`.
   - The private action security coordinator (`depthos_core::action_security`) remains completely segregated from the spatial UI layer.

---

## 4. Observation Status Badges & Fail-Closed Invariant

In DepthOS, trusting an unverified or stale window observation could cause users to misinterpret process placement. Phase 4.1 strictly enforces the **Fail-Closed Observation Invariant**:

```
[Observation Condition]                [Mandatory Badge]      [Status Color]
Live Synchronization                  -> Live                 Emerald (#22C55E)
Sequence Gap / Desync / Epoch Change  -> Stale                Amber (#F59E0B)
Socket Disconnected / Retrying        -> Reconnecting         Cyan (#38BDF8)
Compositor / Observer Daemon Offline  -> Unavailable          Slate (#64748B)
Unclassified / Pending Claim          -> Unknown              Muted Slate (#94A3B8)
```

### Critical Security Rule: Never Claim Stale Observation is Live
The system guarantees that under:
1. Sequence number jumps or drops in `depthos_observer::replica`,
2. Daemon disconnection or restart,
3. Compositor texture capture exclusion,
the badge immediately downgrades to `Stale`, `Reconnecting`, or `Unavailable`.

This invariant is verified in Rust unit tests (`tests/spatial_alpha.rs::test_observation_badge_security_invariants`) and QML integration tests (`tests/spatial/tst_spatial_alpha.qml::test_observation_status_badges_and_invariants`).

---

## 5. Window Identity & Privacy Protection

Window titles frequently contain sensitive data (file paths, private URLs, encryption keys, confidential document headings). Phase 4.1 implements multi-layered privacy defenses:

1. **Title Redaction by Default**:
   Window cards (`WindowCard.qml`) default to displaying sanitized application identities (`appId` / `desktopFileName`) rather than unvetted title captions.
2. **Explicit User-Commanded Reveal (`T` key)**:
   Window titles are displayed only when the user explicitly commands it via `T` (or the "Show Titles" toggle). The property `showTitles` defaults to `false` upon viewport launch and is never persisted across sessions.
3. **Control Character & BiDi Injection Stripping**:
   All string properties rendered in QML use `Text.PlainText` (preventing HTML/rich-text injection). Control characters (`\u0000`–`\u001f`, `\u007f`–`\u009f`) and bidirectional Unicode override runes (`\u202a`–`\u202e`) are stripped to prevent layout manipulation or UI spoofing attacks.
4. **Fallback Cards for Protected Windows**:
   When windows are minimized, flagged with `excludeFromCapture`, or when compositor texture sharing is unavailable, the UI renders a clean metadata fallback card without attempting unauthorized frame-buffer captures.

---

## 6. Adversarial Attack Vector Evaluation Matrix

| Vector ID | Attack Scenario | Evaluated Component | Defense & Invariant Enforcement | Verdict |
| :--- | :--- | :--- | :--- | :---: |
| **ADV-4.1.1** | Unauthorized window placement or desktop switch via UI | `SpatialScene.qml`, `SpatialBackend.qml` | Zero KWin setters exist in the codebase. Navigation keys update visual camera only. | **DEFEATED** |
| **ADV-4.1.2** | Unauthorized state mutation via D-Bus session bus | `src/spatial/mod.rs` (`org.depthos.Controller1`) | Interface exposes zero writable methods (`readOnly: true`, `assignmentMutations: false`). | **DEFEATED** |
| **ADV-4.1.3** | Cross-user hierarchy snooping on multi-user system | `src/spatial/mod.rs` (`authorize`) | Enforces `SO_PEERCRED` EUID matching `getuid()`. Callers from other UIDs receive `AccessDenied`. | **DEFEATED** |
| **ADV-4.1.4** | Denial-of-Service via huge D-Bus query page | `src/spatial/mod.rs` (`page`) | Pagination clamped to 64 items; response serialized size bounded to 128 KiB; max rows 4,096. | **DEFEATED** |
| **ADV-4.1.5** | Stale observation sequence masked as Live | `StatusBadge.qml`, `replica.rs` | Fail-closed state machine immediately downgrades gapped or desynced data to `Stale` or `Reconnecting`. | **DEFEATED** |
| **ADV-4.1.6** | Window caption HTML / Control character injection | `WindowCard.qml` | `Text.PlainText` rendering format enforced; control runes stripped; zero rich-text evaluation. | **DEFEATED** |
| **ADV-4.1.7** | State machine corruption via rapid zoom/navigation bursts | `SpatialScene.qml` | Declarative `Behavior` blocks anchored to instantaneous values; deterministic index clamping. | **DEFEATED** |

---

## 7. Security Review Conclusion

The DepthOS Spatial Workspace Alpha achieves full compliance with the Phase 4.1 security specification and the established security contracts of Phase 3.10.1.

The visual interface is strictly read-only, enforces multi-layered privacy redaction, operates under fail-closed observation badging, and guarantees zero mutation access to the underlying Wayland compositor.
