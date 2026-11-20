# DepthOS Phase 4.1: Native Observer Integration & Security Contracts

## 1. Scope & Security Invariants

DepthOS Phase 4.1 interfaces with the underlying KDE Wayland compositor (KWin 6) through a strictly read-only observation replica (`depthos_observer`). Under no circumstances does the Spatial Workspace Alpha mutate compositor state.

### Explicit Security Invariants
1. **Zero Window Setters**:
   No KWin scripting setters (`client.desktop`, `client.frameGeometry`, `workspace.activeWindow`, etc.) are exposed or invoked by DepthOS.
2. **Zero Native Desktop Mutations**:
   DepthOS visual navigation changes the camera viewport and focused logical context; it **never calls native desktop switching APIs** (`workspace.currentDesktop = ...`).
3. **No Production Consent Issuer**:
   No automatic or implicit consent tokens are minted for window migration in the Alpha release.
4. **Read-Only D-Bus Interfaces**:
   The controller endpoint `org.depthos.Controller1` exposes read-only query methods (`GetCapabilities`, `GetControllerHealth`, `GetHierarchy`, `GetContextReport`, `GetCurrentLogicalContext`) and accepts zero mutation commands.

---

## 2. Observation Status Badges & Invariant Enforcement

Every observed window and workspace context displays a deterministic freshness indicator:

| Badge | Color Code | Visual Token | Condition / Definition |
| :--- | :--- | :--- | :--- |
| **`Live`** | Emerald Green (`#22C55E`) | Pulsing green dot | Active real-time synchronization. Controller lifecycle is `Synchronized` and window association is confirmed. |
| **`Stale`** | Amber (`#F59E0B`) | Amber warning dot | Desynchronized observation, sequence gap, or revision mismatch. Awaiting fresh snapshot. |
| **`Reconnecting`** | Sky Blue (`#38BDF8`) | Pulsing cyan dot | Observer socket disconnected; background exponential backoff reconnect attempt in progress. |
| **`Unavailable`** | Slate Gray (`#64748B`) | Solid slate dot | Observer daemon or KWin compositor offline, or compositor texture capture excluded. |
| **`Unknown`** | Muted Slate (`#94A3B8`) | Gray dot | Unclassified process, missing metadata claim, or pending classification. |

### Critical Invariant: Never Claim Stale Observation is Live
The system enforces a fail-closed observation invariant:
> If an observation replica experiences a sequence jump, socket drop, epoch change, or desynchronization event, all associated window states are immediately downgraded to `Stale` or `Reconnecting`. Stale data is never masked or represented as `Live`.

---

## 3. Window Identity & Privacy Protection

Window titles frequently contain sensitive information (file paths, document headings, browser tabs, chat contact names). DepthOS enforces strict privacy safeguards:

1. **Title Redaction by Default**:
   Window cards default to displaying sanitized application identities (e.g. `org.kde.kate`, `alacritty`, `firefox`) rather than unvetted window captions.
2. **Explicit Title Redaction Toggle (`showTitles`)**:
   Users can press `T` (or click "Show Titles") to inspect titles temporarily for context. The setting defaults to false upon viewport restart.
3. **Sanitized Resource Class Fallback**:
   Control characters (`\u0000`–`\u001f`, `\u007f`–`\u009f`, BiDi override runes) are stripped before string rendering to prevent spoofing or layout corruption.
4. **Fallback Previews for Protected Windows**:
   When windows are minimized, flagged with `excludeFromCapture`, or when compositor texture sharing is unavailable, the UI renders a clean SVG fallback card indicating that metadata is preserved without texture access.

---

## 4. Reconnection & Desynchronization Recovery

The observer client incorporates a non-blocking state machine for connection management:
1. **Name Loss & Bus Disconnect**:
   If the session bus drops `org.depthos.Controller`, the UI resets client caches, transitions to `Disconnected`, and engages a 2000 ms retry timer.
2. **Epoch Invalidation**:
   When the controller emits an `invalidated(instance, scopes)` signal:
   - Scope `1` refreshes the logical hierarchy from SQLite.
   - Scope `2` refreshes the context report and window associations.
3. **Bounded Payloads**:
   All D-Bus pages are bounded to 64 items and 128 KiB to prevent compositor stutter or memory exhaustion.
