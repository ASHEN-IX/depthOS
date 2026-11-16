# Phase 3.0 — native action authorization security

**PHASE 3.0 ACCEPTED** for the offline reference model, denial fixtures and architecture below. This is not acceptance of a production consent verifier, native executor or host integration. Native authorization remains unavailable; Phase 3.1 has not started.

## Isolation and inspected baseline

Work is in `/home/ashenix/Projects/DepthOS-security`, a genuine linked Git worktree on `codex/phase-3-security`. The requested directory did not exist. Git 2.56.1 supports `git worktree add --orphan -b codex/phase-3-security …`, so no initial commit was necessary. Git registered the worktree in shared metadata; canonical source was not edited. Because the repository has an unborn HEAD and untracked project source, a two-pass SHA-256 copy populated this empty worktree with 157 non-ignored source files. `.git`, target/build directories, ignored private evidence and runtime artifacts were not copied. `tests/security/isolation.json` records snapshot time, all original file hashes and the copy method. This is an unborn linked branch with a source snapshot, not a historical committed checkout.

Read against actual implementation: Phase 2.5, context-engine, lifecycle, Phase 1 and POC-02G reports; `src/context/{mod,policy}.rs`, controller, spatial bridge, observer transport and QML client/panel. The copied baseline passed 81 Rust checks with the existing explicit performance benchmark ignored. Historical native evidence was read in the reports, not copied or reclassified as fresh native testing.

Phase 2.5 exports only bounded reads at `org.depthos.Controller1`. `GetCapabilities` reports readOnly=true and assignmentMutations=false. Its local QML `choose()` updates inspection only. The observer authenticates the current `org.kde.KWin` sender and UID; that verifies its transport policy, not human consent. Existing WindowKey and Version already provide receiver/coordinator epochs, exact live generation, observation sequence/revision and policy revision. They are reused rather than replaced with persistent window IDs.

## Threat model and trust assumptions

Protected assets are the exact window selected by the user, native desktop/Activity/focus integrity, logical ownership, protected content, approval freshness and truthful action results. The attacker can send arbitrary same-user bus messages, invent labels/UUIDs/PIDs/approval strings, race native/context updates, replay requests and open competing clients. User-controlled window titles, application IDs and workspace display names are untrusted presentation text. They never establish authority or select the action target.

The reference model assumes its own memory and authoritative synthetic input are intact. A future production authority would additionally need trusted executable/module integrity, a protected interaction path and trustworthy live compositor state. Same-user process injection, mutable authority binaries/QML, unrestricted hostile extensions or a compromised compositor defeat that assumption. Neither this model nor a session-bus secret repairs an already compromised desktop. These conditions must cause deployment refusal, rather than a claim of protection against all same-UID malware.

Threats and required refusals:

- **Same-user D-Bus caller:** UID establishes an OS account only. An authenticated bus connection, including the compositor-name owner, cannot assert a particular user decision. All public interaction claims are refused.
- **Forged QML request:** calling `choose`, emitting a Qt signal, invoking a handler or sending clicked=true is not a trusted input event. There is no exported approval-minting method or writable approval property.
- **Stale UUID/runtime ID:** require the exact WindowKey, native UUID and unique mapping. A UUID or application ID alone cannot recover an absent window. Removal/recreation with the same UUID/runtime ID but a different generation fails.
- **Observer/coordinator replacement:** bind both epochs, session, compositor incarnation and version. Re-Hello, synchronization loss, compositor replacement or an unknown interval invalidates unused approvals. Never restore them on reconnection.
- **TOCTOU:** repeat all identity, source, destination, eligibility and version checks immediately before dispatch. A Rust-side snapshot followed by an asynchronous native setter is insufficient. A native executor needs a single compositor-owned check/consume/act turn with a retained live-object reference; unproven atomicity means refusal.
- **Replay/duplicate requests:** a request ID has one immutable meaning. Consume approval once, retain terminal tombstones and reject duplicate IDs. No automatic retry after dispatch, even on lost acknowledgement.
- **Protected/hidden/unknown targets:** require conservative eligibility, explicit visibility for initial approval and positive capture permission. Unknown permission, minimized/hidden policy state or protected classification refuses authorization. Native eligibility mapping remains a future audit, not a new observer field.
- **Untrusted app IDs:** absent from the action authority model. PID, app ID, icon, title or visual proximity never chooses a replacement target or confirms a launch claim.
- **Conflicting clients:** bind initiating bus instance/unique connection/UID; lock one exact live generation while authorized, in flight or uncertain. Other connections cannot inspect or revoke that action. Global version checks can conservatively invalidate unrelated prepared actions too.
- **Unavailable/restarting compositor:** no authorization or execution while unsynchronized. Owner, bus, incarnation, epoch or topology replacement refuses use. An already dispatched action becomes uncertain; never resend it to the replacement.
- **Cancellation/partial completion:** cancellation before dispatch guarantees no model side effect. Cancellation after dispatch cannot guarantee undo and reports Uncertain. Do not silently roll back: another actor may have changed the state.
- **Flooding/exhaustion:** retain at most 64 action records and 64 consumed approval nonces per authority. Reject further requests; never evict tombstones to permit replay. Proposal throttling and prompt-spam controls must exist before any production request service.

The scope concerns operations through DepthOS. Existing desktop APIs and a user's ability to install/run KWin scripts are not hardened by this milestone.

## Identity is not interaction authorization

The D-Bus specification distinguishes credentials such as UnixUserID, ProcessID and optional ProcessFD. ProcessFD can pin a process against PID recycling. These identify a connection/process, not the intent behind an application request. [D-Bus credentials specification](https://dbus.freedesktop.org/doc/dbus-specification.html#bus-messages-get-connection-credentials).

OS identity answers which user owns a connection. Process identity can bind a live process instance, with PID reuse and connection delegation considered. Application identity may describe an executable, sandbox label or registered desktop application; it is not the app ID supplied by a window. Interactive authorization must separately establish that the trusted authority presented this exact action and captured its explicit approval. Neither same UID, executable path, package signature, owning a service name nor knowledge of a token proves the last property.

No approval capability is written to a same-user-readable file, command line, environment variable, JSON reply or broadcast. Filesystem mode 0600 does not isolate a secret from other processes of that same UID. PID allowlists and arbitrary QML callbacks are not substitutes.

## Choosing the interaction authority

**Preferred direction, conditional on proving its prerequisites:** a small native compositor consent broker, independent of the existing shared QML controller bridge. It would hold the exact live native object, present a fixed operation/source/destination confirmation and consume a private approval in the same native authority. It would expose proposals and redacted results only; never a remotely callable approve/execute slot. An external proposal can open an approval request only after throttling and an explicitly reviewed trusted user gesture. Opening a dialog or activating a shortcut is not itself approval.

KWin 6.8.3's QuickSceneEffect requests a keyboard grab, mouse interception and fullscreen-effect ownership. This establishes native input routing in that implementation, but does not demonstrate human origin, an unforgeable action confirmation or protected QML integrity. [Pinned native input implementation](https://github.com/KDE/kwin/blob/3e528155d822d41285a509d09751a88fa956267f/src/effect/quickeffect.cpp).

The same version exposes a script loader and constructs executable scripts from paths; shared scripting/QML code is therefore part of the desktop's trust assumptions, not an isolated authorization compartment. A callback from the KWin process alone cannot prove that the intended consent UI generated it. Hostile extension access and mutable modules must be controlled or the native authorization design stays disabled. [Pinned scripting implementation](https://github.com/KDE/kwin/blob/3e528155d822d41285a509d09751a88fa956267f/src/scripting/scripting.cpp), [scriptable loader declarations](https://github.com/KDE/kwin/blob/3e528155d822d41285a509d09751a88fa956267f/src/scripting/scripting.h).

A **dedicated Qt application** offers process separation and a smaller UI than an unrestricted shared QML engine, but is not presently an authorization solution. Its app identity and ordinary Qt click signals do not attest input provenance to Rust. It also cannot atomically check and mutate KWin's live window from a separate process. It can serve as a preview/proposal client; a native or OS-supported protected consent/execution broker is still required. Adding a secret to that application without a protected distribution/integrity boundary merely moves the same-user problem.

RemoteDesktop portal consent grants selected remote-input devices/session scope. It is not consent for one exact DepthOS window operation; permitted injected input must not be silently reinterpreted as a fresh physical approval. No portal is requested here. [Portal RemoteDesktop lifecycle and granted-device contract](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html).

Current conclusion: the shipped QML/runtime contract does **not** prove a protected native consent path. Production proof acquisition always returns UnverifiedInteraction. Physical-input versus authorized remote-input policy, lock-screen handling, focus/occlusion, seat routing, accessibility, hostile script loading, code integrity and private native-object access are unresolved gates. No “trusted user click” is inferred from any existing bus signal.

## Typed action contract

`src/action_security.rs` adds a pure model to the Rust library without connecting it to the controller, CLI, D-Bus bridge or compositor. It performs no I/O, native callbacks, random generation, persistence or timer reads. Authority-owned monotonic Tick values and an exclusively borrowed synthetic World make time and state changes deterministic. Real adapters must never accept those authority inputs from wire fields.

`ActionRequest` binds RequestId, exact existing WindowKey, native UUID, compositor bus/unique owner/incarnation, existing expected Version, topology revision and one Operation. Topology revision is a proposed authority-local invalidation token for native desktop changes; it is **not** claimed to exist in POC-02A. No observer wire format changes are made.

Operations remain distinct:

- `Associate { from, to }`: exact logical workspace sets. Empty destination explicitly unassigns; more than one target explicitly shares. No implicit union, suggestion promotion, native move or VerifiedLaunch claim. Each set is capped at eight entries in this model.
- `MoveDesktop { from, to }`: exact native desktop and Activity placement. Source must match, destination must exist and Activity must remain identical. It neither changes current KDE desktop/Activity nor activates a window. All-desktops/multiple-placement windows are outside this narrow model until their semantics can be proven.
- `revoke(Client, RequestId, Tick) -> State`: typed cancellation of that client's pending action, not a new native operation.
- `inspect(Client, RequestId, Tick) -> State`: typed, client-bound result inspection without native metadata. Neither read nor cancellation creates consent.

A future real logical operation must call the accepted coordinator's CAS assignment/unassignment path and verify its resulting version. A future native operation must use a separate compositor executor; combining both into one unreviewed transaction is prohibited. Native movement never implies logical ownership, and logical assignment never moves a native window.

## Capability and execution lifecycle

`Consent` has private fields, no public constructor, Clone or Deserialize. Its only public acquisition API refuses interaction claims. The positive proof oracle is a private helper under `cfg(test)` in `src/action_security/tests.rs`. Test-created proof is explicitly synthetic and is not evidence of native user consent.

`authorize` consumes this receipt and matches the entire request, initiating Client and authority. The receipt binds a five-second maximum monotonic lifetime, including its issue time. Even a stale or mis-scoped verified approval is burned, preventing a repaired retry. Pending requests are bounded and exact-generation conflicts fail closed.

`Capability` is a non-Clone, opaque, in-memory handle bound by an unforgeable safe-Rust Arc identity plus a retained nonce/record. Debug is redacted. The nonce is a model counter, **not a cryptographic bearer secret**. No serialized capability protocol exists. A future cross-process boundary must retain authority-side capabilities and use a separately reviewed authenticated private channel with unpredictability, audience and replay protection; serializing this counter would be unsafe.

`execute_synthetic` is the sole public execution demonstration. It can mutate only a caller-owned synthetic World, with no executor trait or arbitrary native callback. Check, one-time consume and simulated mutation occur in one synchronous mutable borrow. This validates the atomic model assumption only. It does not prove an equivalent KWin primitive exists.

The internal phases are:

1. Verify proof and exact request; enter Authorized and reserve the exact generation.
2. Before dispatch, recheck synchronized state, native mapping uniqueness, epochs/incarnation, full expected version, topology, source, eligibility and destinations. Any mismatch invalidates the unused capability.
3. Consume capability once and enter InFlight. The acknowledgement deadline is one second from dispatch. Consent expiry limits dispatch time; it does not pretend to undo an operation already sent.
4. A matching internal attempt acknowledgement plus a fresh postcondition checkpoint can enter Succeeded. Logical operations require a newer policy revision and unchanged native placement. Native moves require newer native sequence/revision, unchanged logical ownership and exact destination placement. Identity and eligibility must remain corroborated.
5. A verified rejection with the exact unchanged precondition enters Rejected. A generic error or applied acknowledgement without the postcondition is Uncertain, not success or guaranteed failure.
6. Lost acknowledgement, partial execution, timeout or cancellation after dispatch remains Uncertain and keeps the window reservation. Duplicate request/capability/attempt use is refused; there is no resend or automatic rollback.
7. `reconcile_synthetic` requires a fresh synchronized checkpoint in the same incarnation/generation. It may produce Reconciled { destination_observed }, explicitly **not** Succeeded: observing the destination does not prove who caused it. Replaced/unavailable/ambiguous identity stays quarantined until authority teardown or a separately reviewed resolution path.

Client disconnect cancels unused actions and leaves sent actions uncertain. Epoch/synchronization invalidation does the same. Clock rollback and counter overflow refuse progress. The authority never evicts terminal records; at capacity it stops accepting approval. Restart creates a new private authority identity, so old handles do not revive. A production restart/recovery design must preserve uncertainty and prevent automatic replay; this model is not a durable exactly-once protocol.

## Verification evidence

Commands in the security worktree:

```sh
cargo build --locked
cargo test --locked
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
python3 -B tests/security/verify-isolation.py
```

The full Rust suite passes 111 checks, with one pre-existing explicit performance benchmark ignored: 81 existing checks, 28 new model tests, one new private-bus test and one new compile-fail privacy check. Test/build/fmt/Clippy logs are preserved under `artifacts/security/`. No new dependency, Cargo manifest, lockfile, SQLite schema, production D-Bus interface, QML or observer script is changed.

The model tests exercise forged callers/authority/bus/UID bindings, exact consent scope, replay and duplicate IDs, exact expiry, future/oversized lifetimes, monotonic rollback/overflow, bounded tombstones, protected/hidden/unknown windows, stale policy/native/topology versions, missing destinations, generation replacement, ambiguous mappings, native-source changes, post-dispatch ambiguity, client conflicts, cancellation, disconnect/invalidation, lost/wrong/late acknowledgements, partial execution and reconciliation without false success. Positive tests separately cover logical association, explicit sharing/unassignment and synthetic native placement.

`tests/security_boundary.rs` creates its own private dbus-daemon and a test-only denial service. Two real same-UID connections, including one owning the private compositor well-known name, send six requests claiming clicks, application identities and real/fabricated PIDs. The fixture obtains actual UID/PID/bus/sender from D-Bus credentials and refuses all six. It exposes neither Execute nor MoveWindow. No production service or compositor is used. This proves the negative authorization boundary, not a working native consent mechanism.

The compile-fail check demonstrates external safe Rust cannot construct a Consent. Unit tests deliberately reconstruct private proof/handle fields to test replay checks independently of Rust move semantics. These privileged test helpers are not compiled into normal builds.

`tests/security/verify-isolation.py` verifies worktree/common-Git identity, copied-source hashes and the declared edit set. Canonical source hashes and the production KWin/QML/scripts remain unchanged by this work. Four monitored KDE configuration file hashes also match the preserved Phase 2.5 configuration baseline. Shared Git worktree registration is the only intentional change to canonical Git metadata; no commit was made.

## Concrete Phase 3.1 plan — separate approval required

1. Establish a fresh isolated baseline and review this contract. Keep both production bridges read-only. Audit the exact tested KWin build and native script/plugin/input surfaces; define a threat boundary that actually controls hostile extensions and authority code replacement. If that cannot be enforced, stop at refusal.
2. Build a disposable **dry-run consent broker** in a private native test compositor, with fixed native confirmation UI, no QML-callable approve function and no real setter. Bind the displayed exact live object, source/destination, observer/coordinator/topology epochs and one immutable proposal. Destroying the object or changing the proposal cancels approval.
3. Demonstrate a protected input-to-consent route. Test forged QObject/QML calls, D-Bus claims, shortcut activation without confirmation, injected/portal/accessibility input, replay, click-through/focus loss, lock screen, multiple seats, timeout and hostile script loading. Establish and document acceptable physical/remote provenance. Failure to distinguish the chosen threat cases must refuse proof issuance.
4. Validate an in-process/private authority channel and the required native atomic check/consume/dry-run turn. Recheck native object and eligibility at that turn, rather than trusting a UUID lookup or previously delivered Rust snapshot. Race removal/recreation, observer loss, desktop deletion and concurrent clients. No window mutation in this stage.
5. Exercise a **synthetic logical association adapter first**, using the existing coordinator CAS and explicit consent receipts. Verify that suggestions never become assignments without that receipt; keep request, revoke and result APIs separate. Do not expose ordinary same-UID assignment writes as a shortcut.
6. Run fault-injection and bounded-resource suites: duplicate/replayed/different-scope receipts, delayed/wrong acknowledgements, worker loss, stale topology, cancel after dispatch, authority restart and unknown outcome. Require one-shot behavior, truthful results, no auto-retry, scoped cleanup and unchanged host hashes.
7. Produce a new review packet identifying every unresolved trust assumption. Only a subsequent explicitly authorized milestone may consider a minimal real native executor. Passing dry-run consent does not authorize production movement, persistence, session restoration or host installation.

Host version/session gates, portal authorization and physical host acceptance remain unchanged. No compositor was loaded, moved, restarted or configured in Phase 3.0. Phase 3.1 has not begun.
