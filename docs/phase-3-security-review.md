# Phase 3 security review

The review concerns authorization and state integrity through DepthOS. The current production interface exposes read operations. The isolated test environment contains the only positive confirmation path, compiled into the Rust unit tests.

## Trust model

Protected assets include the target window, native placement and focus, logical assignment, the integrity of the operation shown, and the accuracy of execution results. Assume that a same-user process can call available D-Bus methods and that ordinary event delivery and scripted callbacks may have unverified origins. The native tracker and its client run inside a disposable session. The session boundary keeps the fixture away from the host desktop. It does not establish a human consent boundary inside the fixture.

The authorization model treats a proposal as immutable and rechecks the operation and exact current window generation, UUID uniqueness, source and destination, observer session and epoch, compositor incarnation, coordinator epoch, policy revision, topology and expiration. A test-only marker can exercise one-use consumption against the synthetic world. Unknown authority or stale live facts refuse the turn. A lost receipt leaves uncertainty and never triggers retry.

A native preview can show synchronized data while the underlying state changes. The display process may catch up or dismiss after a compositor notification. Production grants remain disabled across that interval. Extending the design to protected input would require primary evidence of event provenance and shared authority over prompt, seat, lock, state validation and consumption.

## Evidence classifications

| Case | Classification | Evidence boundary |
| --- | --- | --- |
| Same-UID clients and private compositor name owner | Verified denial | Private D-Bus denial fixture and exact production bridge introspection |
| Ordinary QML callbacks | Verified denial | Cancellation-only view, local QtTest events and unchanged refusal method |
| EIS, virtual and remote/portal input | Unsupported | No qualified protected native ingress |
| Accessibility-generated events | Unsupported | No qualified protected native ingress |
| Unknown input provenance | Verified denial | Public verifier unconditionally refuses interaction claims. Native attribution remains untested. |
| Expiration, replay and stale checkpoint | Verified denial | Rust authorization and execution model |
| Reused generation, duplicate UUID or runtime identifier | Verified denial | Rust authorization model. Native behavior is exercised only where the fixture can observe the corresponding lifecycle. |
| Window destruction and focus loss | Verified denial | Private fixture revokes the proposal from an owned native window. |
| Seat change and lock transition | Verified denial in the model | Synthetic authority facts. Native seat and lock authorization remain unsupported. |
| Compositor and observer replacement | Verified denial in the model | Stale incarnation and epoch tests. Native continuity checks are bounded by the fixture. |
| One valid internal marker | Verified authorized synthetic behavior | Rust-only one-use consumption and counting sink |
| Competing confirmation and cancellation, lost receipt and worker loss | Verified denial of duplicate or stale execution | Synthetic contention and uncertainty tests plus terminal controller lifecycle tests |

## Independent findings and corrections

- The initial capture reader could discard an intervening invalidation while waiting for the next event. A deterministic queue control showed the old reader could advance and the strict reader refused that ordering. The corrected fixture requires the next expected event and binds capture and removal to the exact owned process. An unexpected transition fails the test.
- The checkpoint verified view state immediately after withdrawal. The correction waits for a Wayland frame swap and requires that the preview be hidden in that frame. A presented response also requires visible labels. These checks improve integration evidence and exercise only owned surfaces.
- The old compile-fail privacy check used an empty initializer, which failed even when a field was public because other fields were omitted. The revised check accesses the private nonce. Compiler controls confirm that the equivalent field access succeeds when a field is public and fails when it is private.
- The Rust projection example now tests extra claim fields in the request envelope as well as in placement data. The default Cargo artifact list contains the library and the normal executable. The production symbol audit found the refusal method and no fixture issuer, confirmation marker, counting sink or projection adapter.

The reviewer found no approval path in the delivered fixture. Native compositor replacement has no dedicated end-to-end test. Observer and coordinator continuity remain model and controller evidence. In particular, expiry and identity observations in the private compositor do not qualify native Rust consumption. No input-injection or remote-session test is presented as provenance evidence.
