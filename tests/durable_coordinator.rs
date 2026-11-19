//! Phase 3.9 Integration Suite: Durable Coordinator Protocol, Crash Recovery,
//! and Lost Acknowledgment Fencing.
//!
//! Enforces:
//! - 8 Linearization points (Acquisition, Preparation, Intent, Dispatch, Ack, Quarantine, Reconciliation, Exclusion release)
//! - Controlled crash-cut matrix across separate Actor lifecycles
//! - Lost, duplicate, and delayed acknowledgment handling
//! - Monotonic incarnation fencing across coordinator restarts
//! - Authoritative reconciliation gate

use depthos_core::action_security::durable::{
    CrashPoint, DurableCoordinatorSession, DurableError, DurableOperationJournal,
    DurableOperationState, ReconciliationEvidence,
};
use depthos_core::action_security::lease::{
    CountingSyntheticSink, LeaseConfig, LeaseId, LeaseRequest, SyntheticSinkAction, TerminalOutcome,
};
use depthos_core::action_security::{
    NativeDesktop, Operation, Placement, RequestId, Tick, WindowKey,
};
use depthos_core::domain::WorkspaceId;
use std::collections::BTreeSet;
use tempfile::TempDir;
use uuid::Uuid;

fn make_window(name: &str) -> WindowKey {
    WindowKey {
        epoch: Uuid::new_v4(),
        runtime_id: name.to_string(),
        generation: "gen-phase-3-9".to_string(),
    }
}

fn make_lease_config(coord_inc: Uuid, comp_inc: Uuid) -> LeaseConfig {
    LeaseConfig {
        coordinator_incarnation: coord_inc,
        coordinator_epoch: 1,
        checkpoint_epoch: Uuid::new_v4(),
        checkpoint_sequence: 10,
        observer_incarnation: Uuid::new_v4(),
        observer_version_seq: 10,
        policy_revision: Uuid::new_v4(),
        topology_revision: Uuid::new_v4(),
        compositor_incarnation: comp_inc,
    }
}

fn make_request(op_id: RequestId, window: WindowKey) -> LeaseRequest {
    LeaseRequest {
        operation_id: op_id,
        window,
        native_uuid: Uuid::new_v4(),
        operation: Operation::Associate {
            from: BTreeSet::new(),
            to: [WorkspaceId(Uuid::new_v4())].into_iter().collect(),
        },
        ttl_ms: 5000,
    }
}

#[test]
fn test_crash_points_matrix_consistency() {
    let temp = TempDir::new().unwrap();

    let crash_scenarios = [
        (CrashPoint::BeforeAcquisition, false),
        (CrashPoint::AfterReservationPersistence, false),
        (CrashPoint::BeforeIntentPersistence, false),
        (CrashPoint::AfterIntentPersistence, true),
        (CrashPoint::BeforeSyntheticConsumption, true),
        (CrashPoint::AfterSyntheticConsumption, true),
        (CrashPoint::BeforeAck, true),
        (CrashPoint::AfterAck, false), // AfterAck completes before crash check
    ];

    for (cp, expected_quarantine_on_recovery) in crash_scenarios {
        let db_path = temp.path().join(format!("crash_{cp:?}.sqlite3"));
        let op_id = RequestId(Uuid::new_v4());
        let window = make_window(&format!("win-{cp:?}"));
        let coord_inc = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();

        // 1. Run until simulated crash point
        {
            let mut journal = DurableOperationJournal::open(&db_path).unwrap();
            let _ = journal.establish_coordinator_incarnation(coord_inc);
            journal.set_crash_point(cp);

            let res = depthos_core::action_security::lease::LeaseReservation {
                id: LeaseId(Uuid::new_v4()),
                operation_id: op_id,
                window: window.clone(),
                native_uuid: Uuid::new_v4(),
                operation: Operation::Associate {
                    from: BTreeSet::new(),
                    to: [WorkspaceId(Uuid::new_v4())].into_iter().collect(),
                },
                coordinator_incarnation: coord_inc,
                coordinator_epoch: 1,
                checkpoint_epoch: Uuid::new_v4(),
                checkpoint_sequence: 1,
                observer_incarnation: Uuid::new_v4(),
                observer_version_seq: 1,
                policy_revision: Uuid::new_v4(),
                topology_revision: Uuid::new_v4(),
                compositor_incarnation: comp_inc,
                created_at: Tick(100),
                expires_at: Tick(5000),
            };

            // Real crash halts execution; do not run subsequent steps on error
            if journal.acquire_reservation(&res, Tick(100)).is_ok()
                && journal
                    .prepare_operation(op_id, Tick(110), coord_inc, 1, comp_inc)
                    .is_ok()
                && journal.persist_dispatch_intent(op_id, Tick(120)).is_ok()
                && journal.mark_synthetic_dispatch_attempted(op_id).is_ok()
            {
                let _ = journal.record_acknowledgment(op_id, TerminalOutcome::Applied, comp_inc);
            }
        }

        // 2. Simulate process restart & startup recovery
        {
            let mut journal = DurableOperationJournal::open(&db_path).unwrap();
            let report = journal.recover_on_startup().unwrap();

            if expected_quarantine_on_recovery {
                assert!(
                    report.quarantined_count >= 1,
                    "Expected quarantine for {cp:?}"
                );
                let rec = journal.get_record(op_id).unwrap();
                assert_eq!(
                    rec.durable_state,
                    DurableOperationState::Quarantined,
                    "State mismatch for {cp:?}"
                );
                assert_eq!(
                    journal.check_window_available(&window).unwrap_err(),
                    DurableError::WindowQuarantined
                );
            } else if cp != CrashPoint::BeforeAcquisition && cp != CrashPoint::AfterAck {
                assert!(report.aborted_count >= 1, "Expected abort for {cp:?}");
                let rec = journal.get_record(op_id).unwrap();
                assert_eq!(
                    rec.durable_state,
                    DurableOperationState::Aborted,
                    "State mismatch for {cp:?}"
                );
                assert!(journal.check_window_available(&window).is_ok());
            }
        }
    }
}

#[test]
fn test_lost_acknowledgment_triggers_quarantine_and_denies_redispatch() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("lost_ack.sqlite3");
    let journal = DurableOperationJournal::open(&db_path).unwrap();

    let coord_inc = Uuid::new_v4();
    let comp_inc = Uuid::new_v4();
    let config = make_lease_config(coord_inc, comp_inc);

    let mut session = DurableCoordinatorSession::new(journal, config.clone(), Tick(100)).unwrap();
    let window = make_window("win-lost-ack");
    let op_id = RequestId(Uuid::new_v4());

    let req = make_request(op_id, window.clone());
    let lease_id = session.acquire(req, Tick(100), comp_inc).unwrap();

    let mut sink = CountingSyntheticSink::new();
    // Simulate compositor processing but dropping the acknowledgment on the wire
    let err = session
        .execute_synthetic(
            lease_id,
            &mut sink,
            SyntheticSinkAction::LostAckAfterApply,
            Tick(200),
        )
        .unwrap_err();

    assert_eq!(err, DurableError::WindowQuarantined);
    assert_eq!(sink.applied_count, 1);

    // Verify durable quarantine
    let rec = session.journal().get_record(op_id).unwrap();
    assert_eq!(rec.durable_state, DurableOperationState::Quarantined);
    assert_eq!(
        rec.uncertainty_reason.as_deref(),
        Some("SyntheticSinkUncertainty")
    );

    // Conflicting mutations on quarantined window fail closed
    assert_eq!(
        session
            .journal()
            .check_window_available(&window)
            .unwrap_err(),
        DurableError::WindowQuarantined
    );

    // Replay or re-dispatch is denied
    let req2 = make_request(op_id, window.clone());
    let replay_err = session.acquire(req2, Tick(250), comp_inc).unwrap_err();
    assert_eq!(replay_err, DurableError::Replay);
}

#[test]
fn test_duplicate_and_delayed_ack_handling() {
    let mut journal = DurableOperationJournal::open_in_memory().unwrap();
    let coord_inc = Uuid::new_v4();
    let comp_inc = Uuid::new_v4();
    let window = make_window("win-dup-ack");
    let op_id = RequestId(Uuid::new_v4());

    let res = depthos_core::action_security::lease::LeaseReservation {
        id: LeaseId(Uuid::new_v4()),
        operation_id: op_id,
        window,
        native_uuid: Uuid::new_v4(),
        operation: Operation::Associate {
            from: BTreeSet::new(),
            to: [WorkspaceId(Uuid::new_v4())].into_iter().collect(),
        },
        coordinator_incarnation: coord_inc,
        coordinator_epoch: 1,
        checkpoint_epoch: Uuid::new_v4(),
        checkpoint_sequence: 1,
        observer_incarnation: Uuid::new_v4(),
        observer_version_seq: 1,
        policy_revision: Uuid::new_v4(),
        topology_revision: Uuid::new_v4(),
        compositor_incarnation: comp_inc,
        created_at: Tick(100),
        expires_at: Tick(5000),
    };

    journal.acquire_reservation(&res, Tick(100)).unwrap();
    journal
        .prepare_operation(op_id, Tick(120), coord_inc, 1, comp_inc)
        .unwrap();
    journal.persist_dispatch_intent(op_id, Tick(150)).unwrap();
    journal.mark_synthetic_dispatch_attempted(op_id).unwrap();

    // 1. First legitimate ack -> Completed
    journal
        .record_acknowledgment(op_id, TerminalOutcome::Applied, comp_inc)
        .unwrap();
    assert_eq!(
        journal.get_record(op_id).unwrap().durable_state,
        DurableOperationState::Completed
    );

    // 2. Duplicate ack arrives -> Rejected (terminal state is absorbing)
    let dup_err = journal
        .record_acknowledgment(op_id, TerminalOutcome::Applied, comp_inc)
        .unwrap_err();
    assert_eq!(
        dup_err,
        DurableError::TerminalState(DurableOperationState::Completed)
    );

    // 3. Stale compositor ack arrives with wrong incarnation -> Rejected
    let wrong_comp = Uuid::new_v4();
    let op_id2 = RequestId(Uuid::new_v4());
    let mut res2 = res;
    res2.id = LeaseId(Uuid::new_v4());
    res2.operation_id = op_id2;
    res2.window = make_window("win-dup-ack-2");
    journal.acquire_reservation(&res2, Tick(300)).unwrap();
    journal
        .prepare_operation(op_id2, Tick(320), coord_inc, 1, comp_inc)
        .unwrap();
    journal.persist_dispatch_intent(op_id2, Tick(350)).unwrap();
    journal.mark_synthetic_dispatch_attempted(op_id2).unwrap();

    let wrong_comp_err = journal
        .record_acknowledgment(op_id2, TerminalOutcome::Applied, wrong_comp)
        .unwrap_err();
    assert_eq!(wrong_comp_err, DurableError::CompositorMismatch);
}

#[test]
fn test_authoritative_reconciliation_workflow() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("reconcile_workflow.sqlite3");
    let journal = DurableOperationJournal::open(&db_path).unwrap();

    let coord_inc = Uuid::new_v4();
    let comp_inc = Uuid::new_v4();
    let config = make_lease_config(coord_inc, comp_inc);

    let mut session = DurableCoordinatorSession::new(journal, config.clone(), Tick(100)).unwrap();
    let window = make_window("win-reconcile");
    let op_id = RequestId(Uuid::new_v4());

    let req = make_request(op_id, window.clone());
    let lease_id = session.acquire(req, Tick(100), comp_inc).unwrap();

    let mut sink = CountingSyntheticSink::new();
    let _ = session.execute_synthetic(
        lease_id,
        &mut sink,
        SyntheticSinkAction::LostAckAfterApply,
        Tick(200),
    );

    // Window is quarantined
    assert_eq!(
        session
            .journal()
            .check_window_available(&window)
            .unwrap_err(),
        DurableError::WindowQuarantined
    );

    // Bogus evidence: empty decision -> Rejected
    let bogus = ReconciliationEvidence {
        observer_incarnation: Uuid::new_v4(),
        observer_sequence: 1,
        verified_window_key: window.clone(),
        observed_native_uuid: Uuid::new_v4(),
        observed_placement: Placement {
            desktop: NativeDesktop("desk-1".to_string()),
            activity: "act-1".to_string(),
        },
        authoritative_decision: "   ".to_string(),
    };
    let bogus_err = session
        .reconcile_quarantine(op_id, bogus, Tick(300))
        .unwrap_err();
    assert_eq!(bogus_err, DurableError::ReconciliationRejected);

    // Valid authoritative evidence -> Reconciled
    let native_uuid = session.journal().get_record(op_id).unwrap().native_uuid;
    let valid = ReconciliationEvidence {
        observer_incarnation: config.observer_incarnation,
        observer_sequence: 12,
        verified_window_key: window.clone(),
        observed_native_uuid: native_uuid,
        observed_placement: Placement {
            desktop: NativeDesktop("desk-2".to_string()),
            activity: "act-1".to_string(),
        },
        authoritative_decision: "AuthoritativeSnapshotReconciledPlacement".to_string(),
    };

    session
        .reconcile_quarantine(op_id, valid, Tick(350))
        .unwrap();

    // Verify reconciled
    let rec = session.journal().get_record(op_id).unwrap();
    assert_eq!(rec.durable_state, DurableOperationState::Reconciled);
    assert_eq!(rec.recovery_status, "AuthoritativelyReconciled");

    // Window exclusion released
    assert!(session.journal().check_window_available(&window).is_ok());

    // Subsequent operation on the same window can now acquire
    let op_id2 = RequestId(Uuid::new_v4());
    let req2 = make_request(op_id2, window);
    let lease2 = session.acquire(req2, Tick(400), comp_inc).unwrap();
    assert_ne!(lease_id, lease2);
}

#[test]
fn test_monotonic_incarnation_epochs_across_restarts() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("incarnation_epochs.sqlite3");

    for expected_epoch in 1..=5 {
        let mut journal = DurableOperationJournal::open(&db_path).unwrap();
        let incarnation_id = Uuid::new_v4();
        let epoch = journal
            .establish_coordinator_incarnation(incarnation_id)
            .unwrap();
        assert_eq!(
            epoch, expected_epoch,
            "Epoch should strictly increment monotonically"
        );

        let (active_id, active_epoch) = journal.get_active_incarnation().unwrap().unwrap();
        assert_eq!(active_id, incarnation_id);
        assert_eq!(active_epoch, expected_epoch);
    }
}
