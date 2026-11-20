# Phase 4.3 association model

A Logical Desktop belongs to a DepthOS Workspace. The observer reports current KDE membership as topology evidence. A KDE virtual desktop ID has meaning for DepthOS only when a user configures a rule.

| Assignment | Required evidence | Placement |
| --- | --- | --- |
| Observed | A capture-eligible application or dialog with exactly one known KDE membership, a unique observed native desktop UUID, and a current rule mapping it to a Logical Desktop | The configured Desktop |
| Explicit | A configured application rule and independent Workspace evidence from the existing confirmed policy or native desktop mapping. The target must agree with any current observed Desktop mapping | The configured Desktop |
| Unassigned | A verified Workspace and known single KDE membership, with no applicable Desktop rule | Workspace only |
| Unknown | A verified Workspace but ambiguous or missing native membership, or two conflicting Desktop rules within that Workspace | Workspace only |

Confirmed Workspace policy and native mapping must agree if both exist. A policy conflict, an invalid parent, or the absence of independently established Workspace scope excludes the identity from the shelf projection. Multiple KDE memberships and sticky windows never establish observed ownership. An independent confirmed Workspace and explicit application rule can still assign a sticky window to a single Logical Desktop.

The controller joins each decision to the exact context epoch, receiver observer epoch, session, generation, sequence, and revision. The durable rule and the current window instance are separate records. Application IDs are bounded display and routing labels observed by KWin. They grant zero authorization. Native window UUIDs, titles, process IDs, geometry, and other private metadata are never used as durable assignment keys.
