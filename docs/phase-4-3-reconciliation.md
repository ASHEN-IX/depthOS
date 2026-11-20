# Phase 4.3 reconciliation

The controller builds the shelf projection from the same synchronized snapshot used for its Workspace report. The query reads hierarchy and association rules in one SQLite transaction. Freshness is a projection property. A synchronized empty result means no eligible windows matched the rules. A gap or disconnected source produces a stale or unavailable response with no window rows.

| Input | Effect |
| --- | --- |
| Initial authenticated snapshot or authenticated replacement | Join present identities to current rules and discard missing entries |
| Add, remove, focus, or native membership change | Reevaluate current membership. Reuse the opaque generation ID during continuous observations of the same window |
| Duplicate or old frame | The replica retains newer evidence and skips the old frame |
| Missing event or stale position | The replica marks observations desynchronized. Shelves clear until a valid replacement snapshot arrives |
| Observer or KWin owner loss | Invalidate observation availability while retaining the SQLite hierarchy and rules |
| Reconnect, UUID reuse, or remove and re-add | Use receiver-generated continuity evidence. The new generation receives a new row ID |
| Offline CLI rule edit or target deletion | Reevaluate the next consistent logical read, without changing KWin |

The existing receiver authenticates the unique KWin owner and the current UID. Each transport packet has at most 32 frames. Neither the UI nor a rule writer can inject window rows into the controller. The QML client never restores a stale window or uses the observer's runtime identifier to select a real window.
