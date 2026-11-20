# Phase 4.3 UI integration

The existing Logical Desktop shelf shows verified counts, allowed application IDs, explicit or observed provenance, and observed focus. Live, empty, stale, and unavailable have distinct states. The selected Workspace summary reports Unassigned and Unknown windows with up to three application labels. Those records are excluded from Desktop counts. A generic privacy notice represents protected identities without revealing whether any are present.

The standalone reader binds to the unique controller owner, checks instance and hierarchy revisions, and validates all parent relationships, row IDs, and allowed fields. Unknown shelf versions leave the hierarchy available. On owner loss the reader clears window records and retains the last hierarchy. Local SQLite reads have no observation channel. Demo remains explicitly labeled and uses the existing mock title controls. Production titles are withheld even when the presentation toggle is enabled.

A single background worker coalesces invalidation scopes. Shelf-only changes leave Worlds and Workspaces intact. The shelf ListModel updates existing rows by opaque ID, including focus changes. Removal clears expired cards and selection. The desktop scene and the approved Worlds carousel and Workspace matrix retain their design.

Private KWin screenshots are under `artifacts/phase-4-3/screenshots/`. They include the live shelf, the shelf after moving the disposable window, stale observations, and controller unavailability. Only the application's own window was captured.
