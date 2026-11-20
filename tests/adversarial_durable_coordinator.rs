//! Independent Phase 3.9 Adversarial Test Suite for the Durable Coordinator Protocol.
//!
//! Evaluates:
//! - State machine terminal absorption and illegal resurrection attacks
//! - Dispatch without durable intent prevention
//! - Duplicate operation ID replay prevention across restarts
//! - Stale incarnation and epoch fencing (coordinator and compositor)
//! - Multi-threaded SQLite concurrency and contention stress
//! - Startup recovery idempotency and pre-intent vs post-intent crash-cuts
//! - Quarantine bypass attacks and bogus reconciliation evidence rejection
//! - Database tampering / corrupted row fail-closed handling

use depthos_core::action_security::durable::{
    DurableCoordinatorSession, DurableError, DurableOperationJournal, DurableOperationState,
    ReconciliationEvidence,
};
use depthos_core::action_security::lease::{
    CountingSyntheticSink, LeaseConfig, LeaseId, LeaseRequest, LeaseReservation,
    SyntheticSinkAction, TerminalOutcome,
};
use depthos_core::action_security::{
    NativeDesktop, Operation, Placement, RequestId, Tick, WindowKey,
};
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::TempDir;
use uuid::Uuid;

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn sample_window(n: u128) -> WindowKey {
    WindowKey {
        epoch: id(200),
        runtime_id: format!("runtime-win-{n}"),
        generation: format!("gen-{n}"),
    }
}

fn sample_lease_config(coord_inc: Uuid, comp_inc: Uuid) -> LeaseConfig {
    LeaseConfig {
        coordinator_incarnation: coord_inc,
        coordinator_epoch: 1,
        checkpoint_epoch: id(300),
        checkpoint_sequence: 10,
        observer_incarnation: id(400),
        observer_version_seq: 10,
        policy_revision: id(500),
        topology_revision: id(600),
        compositor_incarnation: comp_inc,
    }
}

fn sample_request(op_id: RequestId, window: WindowKey, native_uuid: Uuid) -> LeaseRequest {
    LeaseRequest {
        operation_id: op_id,
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

fn sample_reservation(
    op_id: RequestId,
    window: WindowKey,
    native_uuid: Uuid,
    coord_inc: Uuid,
    coord_epoch: u64,
    comp_inc: Uuid,
) -> LeaseReservation {
    LeaseReservation {
        id: LeaseId(Uuid::new_v4()),
        operation_id: op_id,
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
        coordinator_incarnation: coord_inc,
        coordinator_epoch: coord_epoch,
        checkpoint_epoch: id(300),
        checkpoint_sequence: 1,
        observer_incarnation: id(400),
        observer_version_seq: 1,
        policy_revision: id(500),
        topology_revision: id(600),
        compositor_incarnation: comp_inc,
        created_at: Tick(100),
        expires_at: Tick(5_000),
    }
}

#[test]
fn test_adversarial_terminal_states_are_absorbing() {
    let mut journal = DurableOperationJournal::open_in_memory().unwrap();
    let coord_inc = id(101);
    let comp_inc = id(601);
    let window = sample_window(1);
    let op_id = RequestId(id(1001));
    let native_uuid = id(2001);

    let res = sample_reservation(op_id, window, native_uuid, coord_inc, 1, comp_inc);
    journal.acquire_reservation(&res, Tick(100)).unwrap();
    journal
        .prepare_operation(op_id, Tick(110), coord_inc, 1, comp_inc)
        .unwrap();
    journal.persist_dispatch_intent(op_id, Tick(120)).unwrap();
    journal.mark_synthetic_dispatch_attempted(op_id).unwrap();
    journal
        .record_acknowledgment(op_id, TerminalOutcome::Applied, comp_inc)
        .unwrap();

    let rec = journal.get_record(op_id).unwrap();
    assert_eq!(rec.durable_state, DurableOperationState::Completed);

    // Attack 1: Attempt to transition Completed -> Reserved
    assert!(
        !rec.durable_state
            .can_transition_to(DurableOperationState::Reserved)
    );

    // Attack 2: Attempt to record acknowledgment again
    let dup_err = journal
        .record_acknowledgment(op_id, TerminalOutcome::Applied, comp_inc)
        .unwrap_err();
    assert_eq!(
        dup_err,
        DurableError::TerminalState(DurableOperationState::Completed)
    );

    // Attack 3: Attempt to abort an already completed operation
    let abort_err = journal
        .abort_operation(op_id, "MaliciousAbort")
        .unwrap_err();
    assert_eq!(
        abort_err,
        DurableError::TerminalState(DurableOperationState::Completed)
    );

    // Attack 4: Attempt to quarantine an already completed operation
    let quar_err = journal
        .quarantine_operation(op_id, "MaliciousQuarantine")
        .unwrap_err();
    assert_eq!(
        quar_err,
        DurableError::TerminalState(DurableOperationState::Completed)
    );
}

#[test]
fn test_adversarial_dispatch_without_durable_intent_rejected() {
    let mut journal = DurableOperationJournal::open_in_memory().unwrap();
    let coord_inc = id(102);
    let comp_inc = id(602);
    let window = sample_window(2);
    let op_id = RequestId(id(1002));
    let native_uuid = id(2002);

    let res = sample_reservation(op_id, window, native_uuid, coord_inc, 1, comp_inc);
    journal.acquire_reservation(&res, Tick(100)).unwrap();
    journal
        .prepare_operation(op_id, Tick(110), coord_inc, 1, comp_inc)
        .unwrap();

    // Attack: Attempt to mark synthetic dispatch attempted WITHOUT persisting intent
    let dispatch_err = journal
        .mark_synthetic_dispatch_attempted(op_id)
        .unwrap_err();
    assert_eq!(dispatch_err, DurableError::MissingDispatchIntent);

    // Verify record state remains Prepared with dispatch_intent = false
    let rec = journal.get_record(op_id).unwrap();
    assert_eq!(rec.durable_state, DurableOperationState::Prepared);
    assert!(!rec.dispatch_intent);
}

#[test]
fn test_adversarial_duplicate_operation_id_replay_across_database_reopen() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("replay_test.sqlite3");
    let coord_inc = id(103);
    let comp_inc = id(603);
    let window = sample_window(3);
    let op_id = RequestId(id(1003));
    let native_uuid = id(2003);

    // Session 1: Acquire and complete operation
    {
        let config = sample_lease_config(coord_inc, comp_inc);
        let journal = DurableOperationJournal::open(&db_path).unwrap();
        let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

        let req = sample_request(op_id, window.clone(), native_uuid);
        let lease_id = session.acquire(req, Tick(100), comp_inc).unwrap();

        let mut sink = CountingSyntheticSink::new();
        let outcome = session
            .execute_synthetic(
                lease_id,
                &mut sink,
                SyntheticSinkAction::ApplyAndAck,
                Tick(200),
            )
            .unwrap();
        assert_eq!(outcome, TerminalOutcome::Applied);
    }

    // Session 2: Database reopened with fresh coordinator incarnation
    {
        let fresh_coord_inc = id(104);
        let config = sample_lease_config(fresh_coord_inc, comp_inc);
        let journal = DurableOperationJournal::open(&db_path).unwrap();
        let mut session = DurableCoordinatorSession::new(journal, config, Tick(300)).unwrap();

        // Attack: Attempt to replay the exact same operation_id on the same window
        let replay_req = sample_request(op_id, window.clone(), native_uuid);
        let err = session
            .acquire(replay_req, Tick(300), comp_inc)
            .unwrap_err();
        assert_eq!(err, DurableError::Replay);

        // Attack: Attempt to replay the exact same operation_id on a DIFFERENT window
        let different_window = sample_window(999);
        let replay_req_diff = sample_request(op_id, different_window, native_uuid);
        let err2 = session
            .acquire(replay_req_diff, Tick(300), comp_inc)
            .unwrap_err();
        assert_eq!(err2, DurableError::Replay);
    }
}

#[test]
fn test_adversarial_stale_incarnation_fencing() {
    let mut journal = DurableOperationJournal::open_in_memory().unwrap();
    let coord_inc = id(105);
    let comp_inc = id(605);
    let window = sample_window(4);
    let op_id = RequestId(id(1004));
    let native_uuid = id(2004);

    let res = sample_reservation(op_id, window, native_uuid, coord_inc, 1, comp_inc);
    journal.acquire_reservation(&res, Tick(100)).unwrap();

    // Attack 1: Prepare with stale coordinator epoch
    let stale_epoch_err = journal
        .prepare_operation(op_id, Tick(110), coord_inc, 999, comp_inc)
        .unwrap_err();
    assert_eq!(stale_epoch_err, DurableError::CoordinatorMismatch);

    // Attack 2: Prepare with stale coordinator incarnation
    let stale_coord_err = journal
        .prepare_operation(op_id, Tick(110), id(9999), 1, comp_inc)
        .unwrap_err();
    assert_eq!(stale_coord_err, DurableError::CoordinatorMismatch);

    // Attack 3: Prepare with stale compositor incarnation
    let stale_comp_err = journal
        .prepare_operation(op_id, Tick(110), coord_inc, 1, id(9999))
        .unwrap_err();
    assert_eq!(stale_comp_err, DurableError::CompositorMismatch);

    // Legitimate prepare
    journal
        .prepare_operation(op_id, Tick(110), coord_inc, 1, comp_inc)
        .unwrap();
    journal.persist_dispatch_intent(op_id, Tick(120)).unwrap();
    journal.mark_synthetic_dispatch_attempted(op_id).unwrap();

    // Attack 4: Ack with wrong compositor incarnation
    let rogue_ack_err = journal
        .record_acknowledgment(op_id, TerminalOutcome::Applied, id(8888))
        .unwrap_err();
    assert_eq!(rogue_ack_err, DurableError::CompositorMismatch);
}

#[test]
fn test_adversarial_quarantine_bypass_and_bogus_reconciliation() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("quarantine_bypass.sqlite3");
    let coord_inc = id(106);
    let comp_inc = id(606);
    let window = sample_window(5);
    let op_id = RequestId(id(1005));
    let native_uuid = id(2005);

    let config = sample_lease_config(coord_inc, comp_inc);
    let journal = DurableOperationJournal::open(&db_path).unwrap();
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let req = sample_request(op_id, window.clone(), native_uuid);
    let lease_id = session.acquire(req, Tick(100), comp_inc).unwrap();

    let mut sink = CountingSyntheticSink::new();
    // Simulate crash after apply (lost ack)
    let err = session
        .execute_synthetic(
            lease_id,
            &mut sink,
            SyntheticSinkAction::LostAckAfterApply,
            Tick(200),
        )
        .unwrap_err();
    assert_eq!(err, DurableError::WindowQuarantined);

    // Attack 1: Attempt to acquire a new lease on the quarantined window with a new op_id
    let req2 = sample_request(RequestId(id(1006)), window.clone(), native_uuid);
    assert_eq!(
        session.acquire(req2, Tick(250), comp_inc).unwrap_err(),
        DurableError::WindowQuarantined
    );

    // Attack 2: Bogus evidence - mismatched window key
    let bogus_win = ReconciliationEvidence::new(
        id(400),
        20,
        sample_window(999),
        native_uuid,
        Placement {
            desktop: NativeDesktop("desktop-2".into()),
            activity: "activity-default".into(),
        },
        "VerifiedCorrectPlacement",
    );
    assert_eq!(
        session
            .reconcile_quarantine(op_id, bogus_win, Tick(300))
            .unwrap_err(),
        DurableError::ReconciliationRejected
    );

    // Attack 3: Bogus evidence - mismatched native UUID
    let bogus_uuid = ReconciliationEvidence::new(
        id(400),
        20,
        window.clone(),
        id(9999),
        Placement {
            desktop: NativeDesktop("desktop-2".into()),
            activity: "activity-default".into(),
        },
        "VerifiedCorrectPlacement",
    );
    assert_eq!(
        session
            .reconcile_quarantine(op_id, bogus_uuid, Tick(300))
            .unwrap_err(),
        DurableError::ReconciliationRejected
    );

    // Attack 4: Bogus evidence - empty / whitespace decision
    let bogus_empty = ReconciliationEvidence::new(
        id(400),
        20,
        window.clone(),
        native_uuid,
        Placement {
            desktop: NativeDesktop("desktop-2".into()),
            activity: "activity-default".into(),
        },
        "   \t\n  ",
    );
    assert_eq!(
        session
            .reconcile_quarantine(op_id, bogus_empty, Tick(300))
            .unwrap_err(),
        DurableError::ReconciliationRejected
    );

    // Window must STILL be quarantined
    assert_eq!(
        session
            .journal()
            .check_window_available(&window)
            .unwrap_err(),
        DurableError::WindowQuarantined
    );

    // Legitimate reconciliation
    let valid_evidence = ReconciliationEvidence::new(
        id(400),
        20,
        window.clone(),
        native_uuid,
        Placement {
            desktop: NativeDesktop("desktop-2".into()),
            activity: "activity-default".into(),
        },
        "AuthoritativeSnapshotReconciliation",
    );
    session
        .reconcile_quarantine(op_id, valid_evidence, Tick(350))
        .unwrap();

    // Now window is available
    assert!(session.journal().check_window_available(&window).is_ok());
}

#[test]
fn test_adversarial_recovery_idempotency() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("idempotency.sqlite3");
    let coord_inc = id(107);
    let comp_inc = id(607);

    // Insert 2 operations: one pre-intent and one post-intent
    {
        let mut journal = DurableOperationJournal::open(&db_path).unwrap();
        let _ = journal.establish_coordinator_incarnation(coord_inc);

        // Op 1: Pre-intent (stops after reservation)
        let op1 = RequestId(id(3001));
        let win1 = sample_window(101);
        let res1 = sample_reservation(op1, win1, id(4001), coord_inc, 1, comp_inc);
        journal.acquire_reservation(&res1, Tick(100)).unwrap();

        // Op 2: Post-intent (reaches intent recorded)
        let op2 = RequestId(id(3002));
        let win2 = sample_window(102);
        let res2 = sample_reservation(op2, win2, id(4002), coord_inc, 1, comp_inc);
        journal.acquire_reservation(&res2, Tick(100)).unwrap();
        journal
            .prepare_operation(op2, Tick(110), coord_inc, 1, comp_inc)
            .unwrap();
        journal.persist_dispatch_intent(op2, Tick(120)).unwrap();
    }

    // Run recovery 1
    {
        let mut journal = DurableOperationJournal::open(&db_path).unwrap();
        let rep1 = journal.recover_on_startup().unwrap();
        assert_eq!(rep1.aborted_count, 1, "Op 1 pre-intent should be aborted");
        assert_eq!(
            rep1.quarantined_count, 1,
            "Op 2 post-intent should be quarantined"
        );

        let rec1 = journal.get_record(RequestId(id(3001))).unwrap();
        assert_eq!(rec1.durable_state, DurableOperationState::Aborted);
        let rec2 = journal.get_record(RequestId(id(3002))).unwrap();
        assert_eq!(rec2.durable_state, DurableOperationState::Quarantined);
    }

    // Run recovery 2 (IDEMPOTENCY: must produce 0 modifications since already terminal/quarantined)
    {
        let mut journal = DurableOperationJournal::open(&db_path).unwrap();
        let rep2 = journal.recover_on_startup().unwrap();
        assert_eq!(rep2.aborted_count, 0);
        assert_eq!(rep2.quarantined_count, 0);

        let rec1 = journal.get_record(RequestId(id(3001))).unwrap();
        assert_eq!(rec1.durable_state, DurableOperationState::Aborted);
        let rec2 = journal.get_record(RequestId(id(3002))).unwrap();
        assert_eq!(rec2.durable_state, DurableOperationState::Quarantined);
    }
}

#[test]
fn test_adversarial_corrupted_row_fails_closed() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("corrupted_row.sqlite3");

    let bad_op_id = Uuid::new_v4();
    let bad_lease_id = Uuid::new_v4();

    // Create journal and insert a corrupted JSON entry directly into SQLite
    {
        let journal = DurableOperationJournal::open(&db_path).unwrap();
        drop(journal);

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            &format!(
                "INSERT INTO operation_journal (
                    operation_id, lease_id, window_key, native_uuid, operation_type, operation_json,
                    coordinator_incarnation, coordinator_epoch, checkpoint_epoch, checkpoint_sequence,
                    observer_incarnation, observer_version_seq, policy_revision, topology_revision,
                    compositor_incarnation, durable_state, created_at_tick, expires_at_tick,
                    dispatch_intent, created_at_ms, updated_at_ms
                ) VALUES (
                    '{bad_op_id}',
                    '{bad_lease_id}',
                    '{{ INVALID JSON }}',
                    '00000000-0000-0000-0000-000000007777',
                    'MoveDesktop',
                    '{{ INVALID OPERATION JSON }}',
                    '00000000-0000-0000-0000-000000006666',
                    1,
                    '00000000-0000-0000-0000-000000005555',
                    1,
                    '00000000-0000-0000-0000-000000004444',
                    1,
                    '00000000-0000-0000-0000-000000003333',
                    '00000000-0000-0000-0000-000000002222',
                    '00000000-0000-0000-0000-000000001111',
                    'Reserved',
                    100, 5000, 0, 1000, 1000
                )"
            ),
            [],
        )
        .unwrap();
    }

    // Reopen via DurableOperationJournal; querying corrupted row must fail closed with Database error
    let journal = DurableOperationJournal::open(&db_path).unwrap();
    let err = journal.get_record(RequestId(bad_op_id)).unwrap_err();
    match err {
        DurableError::Database(msg) => {
            assert!(
                msg.contains("FromSqlConversionFailure") || msg.contains("Conversion error"),
                "Expected deserialization failure on corrupted JSON: {msg}"
            );
        }
        other => panic!("Expected Database error on corrupted row, got: {:?}", other),
    }
}

#[test]
fn test_adversarial_concurrent_session_window_race() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("concurrent_race.sqlite3");
    let coord_inc = id(108);
    let comp_inc = id(608);

    // Initialize database
    {
        let journal = DurableOperationJournal::open(&db_path).unwrap();
        drop(journal);
    }

    let window = sample_window(777);
    let native_uuid = id(2008);

    let thread_count = 10;
    let mut handles = Vec::new();
    let winners = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let locked_conflicts = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // Shared mutex wrapping coordinator session
    let journal = DurableOperationJournal::open(&db_path).unwrap();
    let config = sample_lease_config(coord_inc, comp_inc);
    let session = Arc::new(Mutex::new(
        DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap(),
    ));

    for i in 0..thread_count {
        let session = Arc::clone(&session);
        let window = window.clone();
        let winners = Arc::clone(&winners);
        let locked_conflicts = Arc::clone(&locked_conflicts);

        handles.push(thread::spawn(move || {
            let op_id = RequestId(id(7000 + i as u128));
            let req = sample_request(op_id, window, native_uuid);

            let mut sess = session.lock().unwrap();
            match sess.acquire(req, Tick(100), comp_inc) {
                Ok(_) => {
                    winners.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                Err(DurableError::WindowLocked) => {
                    locked_conflicts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                Err(e) => panic!("Unexpected error in session race: {:?}", e),
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(
        winners.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "Exactly ONE thread must acquire the lease on contested window"
    );
    assert_eq!(
        locked_conflicts.load(std::sync::atomic::Ordering::SeqCst),
        thread_count - 1,
        "All competing threads must fail closed with DurableError::WindowLocked"
    );
}

#[test]
fn test_regression_vulnerability_reconciliation_stale_seq_and_unverified_authority() {
    let mut journal = DurableOperationJournal::open_in_memory().unwrap();
    let coord_inc = id(109);
    let comp_inc = id(609);
    let obs_inc = id(400);
    let window = sample_window(801);
    let op_id = RequestId(id(1801));
    let native_uuid = id(2801);

    let res = sample_reservation(op_id, window.clone(), native_uuid, coord_inc, 1, comp_inc);
    journal.acquire_reservation(&res, Tick(100)).unwrap();
    journal
        .prepare_operation(op_id, Tick(110), coord_inc, 1, comp_inc)
        .unwrap();
    journal.persist_dispatch_intent(op_id, Tick(120)).unwrap();
    journal
        .quarantine_operation(op_id, "SimulatedLoss")
        .unwrap();

    // VULNERABILITY TEST 1: Stale observer sequence (<= res.observer_version_seq which is 1)
    let stale_seq_evidence = ReconciliationEvidence::new(
        obs_inc,
        1, // Stale! Must be strictly > 1 (the reservation sequence)
        window.clone(),
        native_uuid,
        Placement {
            desktop: NativeDesktop("desktop-2".into()),
            activity: "activity-default".into(),
        },
        "StaleSnapshotClaim",
    );

    // This should fail closed with ReconciliationRejected because sequence is not fresh!
    let err = journal
        .reconcile_quarantined_operation(op_id, stale_seq_evidence, Tick(200))
        .unwrap_err();
    assert_eq!(err, DurableError::ReconciliationRejected);

    // VULNERABILITY TEST 2: Wrong observer incarnation
    let wrong_obs_evidence = ReconciliationEvidence::new(
        id(99999), // Mismatched observer incarnation!
        20,
        window.clone(),
        native_uuid,
        Placement {
            desktop: NativeDesktop("desktop-2".into()),
            activity: "activity-default".into(),
        },
        "ForgedObsClaim",
    );

    let err2 = journal
        .reconcile_quarantined_operation(op_id, wrong_obs_evidence, Tick(200))
        .unwrap_err();
    assert_eq!(err2, DurableError::ReconciliationRejected);
}

#[test]
fn test_regression_vulnerability_native_uuid_quarantine_bypass() {
    let mut journal = DurableOperationJournal::open_in_memory().unwrap();
    let coord_inc = id(110);
    let comp_inc = id(610);
    let window_v1 = sample_window(901); // generation gen-901
    let op_id = RequestId(id(1901));
    let native_uuid = id(2901);

    let res = sample_reservation(
        op_id,
        window_v1.clone(),
        native_uuid,
        coord_inc,
        1,
        comp_inc,
    );
    journal.acquire_reservation(&res, Tick(100)).unwrap();
    journal
        .prepare_operation(op_id, Tick(110), coord_inc, 1, comp_inc)
        .unwrap();
    journal.persist_dispatch_intent(op_id, Tick(120)).unwrap();
    journal.quarantine_operation(op_id, "LostAck").unwrap();

    // VULNERABILITY TEST: Recreated window with same native UUID but new generation / runtime ID
    let window_v2 = WindowKey {
        epoch: window_v1.epoch,
        runtime_id: window_v1.runtime_id.clone(),
        generation: "gen-901-recreated".to_string(),
    };

    // Attempt to acquire lease on window_v2 with the same native_uuid.
    // MUST fail closed because native_uuid is currently quarantined!
    let op_id2 = RequestId(id(1902));
    let res2 = sample_reservation(op_id2, window_v2, native_uuid, coord_inc, 1, comp_inc);
    let err = journal.acquire_reservation(&res2, Tick(200)).unwrap_err();
    assert_eq!(
        err,
        DurableError::WindowQuarantined,
        "Resource quarantine must protect native UUID against recreation bypass!"
    );
}

#[test]
fn test_regression_vulnerability_sqlite_synchronous_full_pragma() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("pragma_test.sqlite3");
    let journal = DurableOperationJournal::open(&db_path).unwrap();

    let synchronous_val = journal.pragma_synchronous().unwrap();

    // PRAGMA synchronous = FULL is 2; NORMAL is 1.
    // For durable coordinator operations, FULL (2) is required.
    assert_eq!(
        synchronous_val, 2,
        "PRAGMA synchronous must be FULL (2) for high-integrity write-ahead barrier safety"
    );
}
