//! Independent Adversarial Attack Suite for the Coordinator Lease Protocol.
//!
//! Evaluates concurrency, race conditions, incarnation fencing, crash-cut recovery,
//! deadlocks, monotonic clock edge cases, and the uncertain-completion quarantine invariant.

use depthos_core::{
    action_security::{
        NativeDesktop, Operation, Placement, RequestId, Tick,
        lease::{
            CoordinatorLeaseManager, CountingSyntheticSink, LeaseConfig, LeaseError, LeaseRequest,
            SyntheticSinkAction,
        },
    },
    context::policy::WindowKey,
};
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn test_config() -> LeaseConfig {
    LeaseConfig {
        coordinator_incarnation: id(100),
        coordinator_epoch: 1,
        checkpoint_epoch: id(200),
        checkpoint_sequence: 50,
        observer_incarnation: id(300),
        observer_version_seq: 10,
        policy_revision: id(400),
        topology_revision: id(500),
        compositor_incarnation: id(600),
    }
}

fn sample_req(window: WindowKey, native_uuid: Uuid, op_id: Uuid) -> LeaseRequest {
    LeaseRequest {
        operation_id: RequestId(op_id),
        window,
        native_uuid,
        operation: Operation::MoveDesktop {
            from: Placement {
                desktop: NativeDesktop("desktop-1".into()),
                activity: "activity-default".into(),
            },
            to: Placement {
                desktop: NativeDesktop("desktop-2".into()),
                activity: "activity-default".into(),
            },
        },
        ttl_ms: 5_000,
    }
}

fn sample_window(n: u128) -> WindowKey {
    WindowKey {
        epoch: id(200),
        runtime_id: format!("runtime-{}", n),
        generation: format!("gen-{}", n),
    }
}

#[test]
fn test_adversarial_concurrent_lease_race_single_winner() {
    let mgr = Arc::new(Mutex::new(CoordinatorLeaseManager::new(
        test_config(),
        Tick(1_000),
    )));
    let window = sample_window(1);
    let native_uuid = id(1001);
    let comp = id(600);

    let thread_count = 10;
    let mut handles = Vec::new();
    let winner_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let conflict_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    for i in 0..thread_count {
        let mgr = Arc::clone(&mgr);
        let window = window.clone();
        let winners = Arc::clone(&winner_count);
        let conflicts = Arc::clone(&conflict_count);

        handles.push(thread::spawn(move || {
            let req = sample_req(window, native_uuid, id(5000 + i as u128));
            let mut lock = mgr.lock().unwrap();
            match lock.acquire(req, Tick(1_000), comp) {
                Ok(_) => {
                    winners.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                Err(LeaseError::Conflict) => {
                    conflicts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                Err(e) => panic!("Unexpected error in race: {:?}", e),
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(
        winner_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "Exactly ONE thread must acquire the lease on the contested window"
    );
    assert_eq!(
        conflict_count.load(std::sync::atomic::Ordering::SeqCst),
        thread_count - 1,
        "All other threads must fail with LeaseError::Conflict"
    );
}

#[test]
fn test_adversarial_quarantine_prevents_subsequent_leases_until_reconciled() {
    let mut mgr = CoordinatorLeaseManager::new(test_config(), Tick(1_000));
    let window = sample_window(2);
    let native_uuid = id(1002);
    let comp = id(600);

    let req1 = sample_req(window.clone(), native_uuid, id(6001));
    let lease1 = mgr
        .acquire(req1, Tick(1_000), comp)
        .expect("acquire lease1");

    let mut sink = CountingSyntheticSink::new();
    // Simulate crash after consumption (sink applies, but ack is lost)
    let outcome = mgr.consume_synthetic(
        lease1,
        &mut sink,
        SyntheticSinkAction::LostAckAfterApply,
        Tick(1_500),
    );

    assert_eq!(outcome, Err(LeaseError::Quarantined));
    assert_eq!(sink.applied_count, 1);
    assert!(mgr.is_window_quarantined(&window));

    // Attack 1: Attempt to acquire a new lease on the quarantined window with a FRESH operation ID
    let req2 = sample_req(window.clone(), native_uuid, id(6002));
    assert_eq!(
        mgr.acquire(req2, Tick(1_600), comp),
        Err(LeaseError::Quarantined),
        "Quarantined window must reject all new lease attempts!"
    );

    // Attack 2: Attempt to replay the old operation ID
    let req_replay = sample_req(window.clone(), native_uuid, id(6001));
    assert_eq!(
        mgr.acquire(req_replay, Tick(1_600), comp),
        Err(LeaseError::Replay),
        "Replay of consumed/quarantined operation ID must be rejected!"
    );

    // Reconcile and clear quarantine
    assert!(mgr.reconcile_quarantined_window(&window));
    assert!(!mgr.is_window_quarantined(&window));

    // After reconciliation, fresh lease request succeeds
    let req3 = sample_req(window.clone(), native_uuid, id(6003));
    let lease3 = mgr
        .acquire(req3, Tick(2_000), comp)
        .expect("acquire lease3 after reconciliation");
    assert!(lease3 != lease1);
}

#[test]
fn test_adversarial_compositor_and_coordinator_incarnation_fencing() {
    let mut mgr = CoordinatorLeaseManager::new(test_config(), Tick(1_000));
    let window = sample_window(3);
    let native_uuid = id(1003);
    let legitimate_comp = id(600);
    let rogue_comp = id(9999);

    // Attack 1: Rogue or drifted compositor incarnation at acquisition
    let req1 = sample_req(window.clone(), native_uuid, id(7001));
    assert_eq!(
        mgr.acquire(req1.clone(), Tick(1_000), rogue_comp),
        Err(LeaseError::CompositorMismatch),
        "Mismatched compositor incarnation must be rejected at acquisition"
    );

    // Legitimate acquisition
    let lease1 = mgr
        .acquire(req1, Tick(1_000), legitimate_comp)
        .expect("acquire");

    // Attack 2: Compositor restarts before consumption (new compositor incarnation)
    let restart_comp = id(601);
    mgr.restart_compositor(restart_comp, Tick(1_500));

    let mut sink = CountingSyntheticSink::new();
    assert_eq!(
        mgr.consume_synthetic(
            lease1,
            &mut sink,
            SyntheticSinkAction::ApplyAndAck,
            Tick(1_500),
        ),
        Err(LeaseError::NotFound),
        "Compositor restart fencing must invalidate active leases from old incarnation"
    );
    assert_eq!(
        sink.applied_count, 0,
        "No dispatch should occur under stale incarnation"
    );
}

#[test]
fn test_adversarial_clock_reversal_and_overflow() {
    let mut mgr = CoordinatorLeaseManager::new(test_config(), Tick(10_000));
    let window = sample_window(4);
    let native_uuid = id(1004);
    let comp = id(600);

    // Attack 1: Clock reversal at advance_time
    assert_eq!(
        mgr.advance_time(Tick(9_999)),
        Err(LeaseError::ClockReversed),
        "Monotonic clock moving backwards must fail closed"
    );

    // Attack 2: Clock reversal at acquisition
    let req = sample_req(window.clone(), native_uuid, id(8001));
    assert_eq!(
        mgr.acquire(req.clone(), Tick(9_999), comp),
        Err(LeaseError::ClockReversed),
        "Acquisition before current clock must fail closed"
    );

    // Attack 3: Excessive TTL or integer overflow
    let mut overflow_req = req;
    overflow_req.ttl_ms = u64::MAX;
    assert_eq!(
        mgr.acquire(overflow_req, Tick(10_000), comp),
        Err(LeaseError::CounterExhausted),
        "Overflowing TTL must fail closed with CounterExhausted"
    );
}

#[test]
fn test_adversarial_revocation_prevents_consumption() {
    let mut mgr = CoordinatorLeaseManager::new(test_config(), Tick(1_000));
    let window = sample_window(5);
    let native_uuid = id(1005);
    let comp = id(600);

    let req = sample_req(window.clone(), native_uuid, id(9001));
    let lease_id = mgr.acquire(req, Tick(1_000), comp).expect("acquire");

    // Revoke the lease
    mgr.revoke(lease_id, Tick(1_200)).expect("revoke succeeds");

    // Attack: Attempt to consume the revoked lease
    let mut sink = CountingSyntheticSink::new();
    let res = mgr.consume_synthetic(
        lease_id,
        &mut sink,
        SyntheticSinkAction::ApplyAndAck,
        Tick(1_300),
    );

    assert_eq!(
        res,
        Err(LeaseError::NotFound),
        "Consumption of revoked lease must be strictly denied"
    );
    assert_eq!(
        sink.applied_count, 0,
        "Sink must not be called for revoked lease"
    );

    // Verify window is unlocked after revocation
    assert!(!mgr.is_window_locked(&window));

    // Fresh proposal can now acquire a lease on the window
    let req2 = sample_req(window, native_uuid, id(9002));
    assert!(mgr.acquire(req2, Tick(1_400), comp).is_ok());
}
