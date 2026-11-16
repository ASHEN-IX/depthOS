# Phase 2.3B — ephemeral project context engine

**PHASE 2.3B ACCEPTED** for Rust policy, ephemeral application integration, and private-D-Bus fixtures. This is not host-native acceptance. Phase 2.4 has not begun.

## Architecture and ownership

`src/context/policy.rs` ports the accepted Phase 2.3A deterministic policy. It receives only live keys, eligibility, dialog/parent relationships, logical workspace IDs and prepared claims. It performs no I/O. `src/context/mod.rs` adds `ContextCoordinator` above the existing `Application<S, O>`, reading the logical hierarchy and the adapter's synchronized snapshot. The coordinator neither owns a receiver nor reconstructs native state; `depthos_observer::Replica` remains the only observer reducer. No dependency, Cargo manifest, lockfile, database service, SQLite schema or migration changed.

The coordinator reads current workspace IDs with World IDs and update revisions as an invalidation fingerprint. The policy function receives workspace IDs only. Names, roots, descriptions and desktop details do not enter decisions. Rename and World movement retain an existing user assignment's workspace ID, but advance the policy revision and reject operations prepared against the older logical state. Deleting a winning target produces Stale with no fallback or partial shared assignment.

Every report, inspection and assignment operation refreshes the two read inputs. Consumers can subscribe to `application.observations().subscribe()` and request a new report after a wakeup; logical CRUD callers can request one immediately after their own transaction. There is no background task, daemon, event bus or SQLite lock held across D-Bus awaits. The coordinator uses the adapter's in-process snapshot API, not D-Bus itself. Logical CRUD remains usable when observations or context decisions are unavailable.

## Decisions, reasons and provenance

The public `Decision` contains a categorical `Status`, stable `Reason`, sorted source categories, sorted target workspace IDs and sorted conflict alternatives. `Association` adds the evaluated `WindowKey` and a `Version`; `ContextReport` groups the current version and deterministic associations. There is no numerical confidence score.

Statuses are Confirmed, Suggested, Conflicted, Unknown and Stale. Only `effective_targets()` on Confirmed returns ownership; Suggested targets are advisory. Unknown, Stale and Conflicted have no effective owners. Report selections expose all eligible windows, unassigned/stale windows, conflicted windows or suggestions. `inspect` can return an excluded-window reason or Stale for an obsolete key; unavailable current observations return an explicit `ContextError::Unavailable` instead of returning cached decisions.

Precedence remains:

1. Explicit live user assignment or suppression.
2. Verified attribution of a DepthOS launch to this exact live window.
3. Explicit verified scoped policy.
4. Advisory heuristic and eligible-parent hint, as peers.
5. Unknown.

Equal-tier claims with identical sets agree. Different top-tier sets yield Conflicted, never a first-arrival winner. One explicitly shared claim is distinct from independent claims for two workspaces; those claims are not merged. Missing workspace references in any winning claim produce Stale before conflict resolution or fallback.

Stable reason codes are `observation_unavailable_or_replaced`, `window_outside_policy_scope`, `no_applicable_evidence`, `workspace_removed`, `equal_precedence_disagreement`, `user_unassigned`, `review_required`, `user_confirmed`, `verified_window_launch`, and `explicit_binding_matched`. Sources serialize as `user`, `verified_launch`, `explicit_policy`, `heuristic`, and `parent_hint`.

## Epoch and live-window identity

The old snapshot contract had no explicit continuity evidence beyond session/sequence/revision, and coalesced watch delivery could hide an unhealthy interval or a removal/re-addition. This phase adds **receiver-local metadata**, not a replacement producer protocol:

- `ObservedSnapshot.observer_epoch`: a UUID marker renewed on Hello, disconnect/invalidation, desynchronization, and snapshots that cannot prove a contiguous unchanged-revision baseline.
- `ObservedSnapshot.window_generations`: one opaque generation per live runtime ID. Added windows get fresh generations; removal deletes the entry. Native UUID changes also replace the generation. The map is bounded to the current windows and contains no retained ID history.

The epoch remains stable across healthy ordered incremental updates. A snapshot can preserve it only when sequence is exactly next and revision unchanged; each retained window additionally needs unchanged native-ID corroboration to keep its generation. A snapshot jump, gap recovery or new Hello cannot restore an old epoch. A native UUID alone never restores a key or joins windows. Missing/duplicate native IDs provide no cross-epoch continuity evidence.

`WindowKey` contains a coordinator epoch UUID, runtime ID and receiver-local window generation. The coordinator epoch is fresh for each coordinator instance and each observed epoch/session replacement or uncertainty. Decision versions also contain observer epoch, session ID, sequence/revision and policy revision. These opaque UUIDs are ephemeral local invalidation markers, not cryptographic authentication or native identities.

Consumer-ID changes alone establish neither continuity nor replacement. This implementation conservatively drops assignments across receiver restarts/re-Hello because there is no cross-receiver continuity verifier. A coordinator that never sees an intermediate unhealthy notification still sees the changed observer marker in the recovered snapshot. A removed and re-added runtime ID receives a different window generation even if both events happen between coordinator reads; unrelated continuously live windows keep their assignments.

The raw POC-02A Hello/Push envelopes and frames are unchanged. The metadata is added only to the Rust snapshot/export envelope. PID, process start time, executable, activation token and launch-client provenance remain unavailable. No native field was fabricated to supply them.

## Claim validation boundary

`Claim` fields and its prepared constructor are crate-private; it does not implement Deserialize. Normal production code can create claims only through the coordinator's explicit user operations and advisory hint operation. A compile-fail test verifies external code cannot call the prepared constructor to manufacture VerifiedLaunch claims.

A crate-internal `accept_prevalidated` interface exists **only under `cfg(test)`**. It validates current expected version and live keys, then installs synthetic prepared launch/policy claims for policy/coordinator tests. No production launch verifier or verified scoped-policy producer is enabled. There is no CLI flag, observer field or JSON import that can mint verified provenance. The original Python prepared claims and their Rust counterparts are synthetic tests, not evidence of actual launch attribution.

The private prepared interface models the contract for a future verifier: exact current key, observed position, policy revision, explicit target set, and bounded lifetime. Prepared claims and advisory hints expire when observation position changes or the logical workspace fingerprint changes. User overrides are separate, live-key-scoped intent and can survive healthy metadata/geometry updates. Future production evidence issuers require their own verification, freshness, replay, audience and revocation design before being enabled.

A parent hint is derived only for an eligible dialog with an acyclic, same-epoch, fully present and eligible parent chain. Its direct parent must have an independently Confirmed single-workspace association at the current observation. A suggestion never confirms a parent or a child. Shared parent ownership is not implicitly copied; protected, unknown, absent, self, cyclic and replaced chains produce no hint. Parent and heuristic disagreements remain Conflicted.

## Explicit operations and compare-and-set

The application API provides:

- `report(application, selection)` and `inspect(application, key)` for current decisions.
- `assign(application, key, workspace_set, expected_version)` for an exclusive or explicitly shared live assignment.
- `unassign(...)` for explicit suppression of every lower tier.
- `clear_override(...)` to remove assignment/suppression and permit current evidence again.
- `suggest(..., workspace, expected_version)` for an advisory-only exact-position hint.

Mutation methods refresh inputs, require a synchronized snapshot, compare epoch, observer position and policy revision, require the exact current generation/key and conservative eligibility, and validate target workspace IDs. Successful edits advance policy revision. Old user operations cannot target a replacement window or overwrite a newer override. `assign` rejects empty targets; `unassign` is the only way to request explicit suppression. Sources and targets never come from client-asserted ownership fields.

```rust,no_run
use depthos_core::{application::Application, context::{ContextCoordinator, Selection},
    observed::ObservedStateSource, service::Store};
use std::collections::BTreeSet;

fn assign_selected<S: Store, O: ObservedStateSource>(
    application: &Application<S, O>,
    coordinator: &mut ContextCoordinator,
    target: depthos_core::domain::WorkspaceId,
) -> Result<(), depthos_core::context::ContextError> {
    let report = coordinator.report(application, Selection::All)?;
    if let Some(selected) = report.associations.first() {
        let assigned = coordinator.assign(application, &selected.window,
            BTreeSet::from([target]), &report.version)?;
        let suppressed = coordinator.unassign(application, &selected.window, &assigned.version)?;
        coordinator.clear_override(application, &selected.window, &suppressed.version)?;
    }
    Ok(())
}
```

A foreground caller must keep the coordinator and Application alive for assignments to remain meaningful. No misleading one-shot assignment CLI was added. Existing system diagnostics remain read-only and keep their prior lifecycle. The coordinator itself does not own a transport. Dropping it discards all context intent; closing windows expires their overrides; uncertainty discards the relevant epoch. Caller-held reports are historical immutable values and must not be treated as continuously current.

CAS is a local read-time check, not an atomic transaction with KWin or an external SQLite writer. Events can arrive immediately after a successful edit. The next read revalidates health/identity and expires stale decisions. A UI should serialize access to its mutable coordinator, handle rejected operations by refreshing selection, and watch observer notifications. There is no cross-process assignment service or multi-coordinator shared state in this phase.

## Privacy and persistence

`project` copies only the epoch/generation/runtime key, conservative eligibility, dialog flag and parent key into policy. It excludes titles, app IDs, native UUIDs, resource names, geometry, focus, URLs, profiles, paths, commands, environments and process identity. Client-supplied application IDs remain untrusted and unused. Same-app terminals, browsers, editors and missing app IDs therefore require real explicit evidence rather than app-family ownership inference.

Eligibility requires Application or Dialog classification, known `excludeFromCapture == false`, and known `skipTaskbar == false`. Unknown classification or protection, protected windows and unsupported types are excluded even when an override exists. Decisions and fixed errors contain IDs/categories only; raw observer packets and personal metadata are never logged by this module.

SQLite schema v1 and all storage code are unchanged. Context claims, overrides, decisions, epochs and suggestions live only in memory. No runtime ID, client app ID, title, URL, profile data or command text is persisted. No directory traversal, process scan, launch, focus, desktop switch or window movement occurs. No durable binding migration is proposed: a trusted rebindable selector and revocation semantics do not yet exist.

## Tests and evidence

Verified commands:

```sh
cargo build --locked
cargo test --locked
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
python3 -B -m unittest discover -s experiments/context-policy -v
python3 tests/observer-adapter/parity.py
python3 tests/observer-adapter/private-bus.py
python3 tests/observer-adapter/unified-cli.py
python3 tests/context-engine/verify-source.py
```

The Rust suite passes **64 checks**: the 37 accepted Phase 2.2B checks, all 16 ported Python scenarios, four coordinator unit tests, three context private-bus integration tests, three receiver continuity tests, and one new compile-fail claim-boundary test. The unchanged integrated Python prototype passes its original 16 scenarios. Production observer parity remains seven scenarios / 30 packet comparisons. Receiver and unified-CLI private-bus interoperability regressions pass, including real Python receiver name collision and redaction.

The end-to-end context fixture creates two workspaces, synchronizes two windows with the same app ID, assigns them independently, verifies decisions and stale-CAS rejection, disconnects, reconnects under a replacement session, and proves old associations cannot attach to reused runtime IDs. Database file bytes and logical hierarchy remain identical after observation/assignment operations and reconnect. Additional private-bus fixtures cover unseen sequence/revision gaps, same-token Hello, and coalesced removal/reuse while preserving another window's assignment. Receiver tests cover healthy continuity, stale-consumer noninterference, snapshot jumps and native UUID replacement.

Ported scenarios cover explicit sharing versus conflicts, precedence, repeated/PID-reuse nonidentity, browser ambiguity, null application IDs, missing workspace targets, invalid parents, explicit suppression, clearing overrides, rename/move retention, advisory-only results, protection, redaction and deterministic sorting. All buses are newly spawned private test daemons or explicit `dbus-run-session` instances. No host-native coverage is claimed.

## Source preservation and rollback

Before edits, source writer/file-descriptor checks found no active writer and repeated source hash inventories agreed. A verified 123-file source backup is at:

`/home/ashenix/Projects/DepthOS-backups/phase-2.3b-20261116T135111Z/`

The backup excludes Git metadata, private evidence, screenshots/recordings, build outputs, caches, logs, runtime files and credentials. This is a point-in-time content verification, not filesystem atomicity or a permanent writer lock. Only the approved Phase 2.3A report and `experiments/context-policy/` reference artifacts were transferred; none of the older copy's Rust, Cargo or SQLite files was copied over canonical source.

`tests/context-engine/manifest.json` records original hashes, reference hashes, modified/added engine files and final verification. The existing reports, manifests, backups, Phase 1 sources, SQLite code/migration, Cargo manifests/lockfile and CLI source are preserved byte for byte. Earlier Phase 2.2B and Phase 2.3A hash verifiers remain historical snapshot checks and can report intentional later evolution; their records were not rewritten to mask this change. The new verifier checks this phase's allowed changes and the remaining protected source. The selected `kwinrc`, `kglobalshortcutsrc`, `kactivitymanagerdrc` and `kwinrulesrc` hashes match the pre-edit baseline. No host KDE mutation command was executed.

Rollback must first compare current engine-file hashes with this phase's final manifest. Preserve/review any later changes before restoration. Restore only the modified pre-existing `src/lib.rs` and observer `src/{model,replica}.rs` from the scoped backup, and remove only matching newly added context/reference/test files. Never reset/clean Git, restore the whole older tree over new work, or modify KDE to undo this logical-only phase. No commits, merges or pushes were created.

## Limitations and Phase 2.4 readiness

The engine answers association questions for its current verified observation and explicit evidence; it is not an access-control boundary. Same-user D-Bus trust, post-framing packet allocation, bounded ownership-change observation and untested real cross-UID execution retain their documented observer limitations. Synthetic prepared claims do not qualify a real launcher or connector. There is no persistence, restoration, browser-profile classifier, app-family binding, automatic routing or native action API.

No broad compositor compatibility or performance claim follows from these tests. Host version/session and authorization gates remain unchanged. The foundation is ready for a separately scoped Phase 2.4 discussion about foreground UI review and/or verified evidence issuers, with fresh policy and native authorization tests as needed. Phase 2.4 has not started.
