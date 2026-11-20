# Phase 4.3 read-only controller contract

Controller1 advertises `desktopShelvesVersion: 1` through its existing capabilities method. `GetDesktopShelves(cursor: u32, expected: string) -> string` returns a JSON envelope with schema version 1, controller instance, shelf revision, availability, and data. Start at cursor 0 with an empty expected revision. Subsequent pages pass the returned revision. A changed revision or invalid cursor fails the read. Each page contains at most 64 items, with at most 4096 total. Identifiers and fields are bounded.

| Field | Semantics |
| --- | --- |
| `data.freshness` | `live`, `stale`, or `unavailable` |
| `data.hierarchyRevision` | Revision of the hierarchy used for the join |
| `data.lastValidatedSequence` | Decimal string, or null when unavailable. Sequence is local to observer continuity |
| `data.titlePolicy` | `withheld` |
| `data.items[]` | `id`, `workspaceId`, optional `desktopId`, optional `applicationId`, `assignment`, `focused` |
| `data.nextCursor` | Next cursor or null |

Rows never contain raw handles, native window or desktop UUIDs, title fields, observation credentials, or protected-window payloads. The opaque ID is minted by the receiver for the current generation. A page with `available: true` and zero rows is a valid empty projection. An unavailable page always contains zero rows.

The existing `Invalidated` signal adds scope bit 4 for shelves. Bits 1 and 2 retain their hierarchy and context meanings. Stable changes to geometry or observation sequence alone do not emit shelf invalidation. Legacy clients can continue using existing reads. The new client keeps the hierarchy on an older schema-1 controller without shelf capability or with a future shelf version. It suppresses incompatible window data. Phase 3 authorizations and action methods remain outside Controller1.
