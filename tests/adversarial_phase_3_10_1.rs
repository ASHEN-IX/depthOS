//! Phase 3.10.1 Independent Adversarial Integration Test Suite
//!
//! # Attack Vectors Covered:
//! 1. Lost ACK vs delayed ACK race & quarantine persistence.
//! 2. Duplicate ACK rejection after terminal state.
//! 3. Quarantine persistence across resource identity aliases (WindowKey vs Native UUID).
//! 4. Authoritative reconciliation provenance fuzzing (rejecting fake provenance strings).
//! 5. Reconciliation stolen / invalid observer token rejection.
//! 6. Reconciliation stale / equal sequence rejection (`seq <= recorded_seq`).
//! 7. Reconciliation observer incarnation fencing.

use depthos_core::action_security::{
    NativeDesktop, Operation, Placement, RequestId, Tick, Uncertainty, WindowKey,
    durable::{
        DurableCoordinatorSession, DurableOperationJournal, DurableOperationState, EvidenceSource,
        ReconciliationEvidence, TerminalClaim,
    },
    ipc::ReconcileEvidenceMessage,
    lease::{LeaseConfig, LeaseRequest, TerminalOutcome},
    reconciliation::{
        AUTHORITATIVE_PROVENANCE, AuthoritativeReconciliationGateway, ReconciliationError,
    },
};
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn sample_window(n: u128) -> WindowKey {
    WindowKey {
        epoch: id(200),
        runtime_id: format!("win-runtime-{n}"),
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
                desktop: NativeDesktop("desk-src".into()),
                activity: "act-main".into(),
            },
            to: Placement {
                desktop: NativeDesktop("desk-dst".into()),
                activity: "act-main".into(),
            },
        },
        ttl_ms: 5000,
    }
}

struct TestEnv {
    _temp_dir: TempDir,
    db_path: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("durable.sqlite3");
        Self {
            _temp_dir: temp_dir,
            db_path,
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Lost ACK vs Delayed ACK Race & Quarantine Persistence
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_3_10_1_delayed_ack_cannot_unquarantine_resource() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let coord_inc = id(1001);
    let comp_inc = id(2001);
    let config = sample_lease_config(coord_inc, comp_inc);
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let op_id = RequestId(id(101));
    let win = sample_window(101);
    let native_uuid = id(5001);
    let op = sample_request(op_id, win.clone(), native_uuid);

    // 1. Acquire lease and prepare intent
    let lease_id = session.acquire(op, Tick(100), comp_inc).unwrap();
    session
        .journal_mut()
        .prepare_operation(op_id, Tick(100), coord_inc, 1, comp_inc)
        .unwrap();
    session
        .journal_mut()
        .persist_dispatch_intent(op_id, Tick(100))
        .unwrap();

    // 2. Timeout occurs -> uncertainty quarantined
    session
        .quarantine_lease(
            lease_id,
            op_id,
            "Coordinator timeout waiting for ACK",
            Uncertainty::NoAcknowledgement,
            Tick(150),
        )
        .unwrap();

    let entry = session.journal().get_record(op_id).unwrap();
    assert_eq!(entry.durable_state, DurableOperationState::Quarantined);
    assert!(session.lease_manager().is_window_quarantined(&win));

    // 3. Delayed ACK arrives claiming Applied
    // Attempting to complete a quarantined operation in the journal must fail
    let complete_res =
        session
            .journal_mut()
            .record_acknowledgment(op_id, TerminalOutcome::Applied, comp_inc);
    assert!(
        complete_res.is_err(),
        "Completing quarantined operation via delayed ACK must be rejected by journal"
    );

    // Verify window and native_uuid remain quarantined
    assert!(
        session.lease_manager().is_window_quarantined(&win),
        "Window must remain quarantined"
    );
    assert!(
        session.lease_manager().is_native_quarantined(&native_uuid),
        "Native UUID must remain quarantined"
    );
}

// ---------------------------------------------------------------------------
// 2. Duplicate ACK Rejection After Terminal State
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_3_10_1_duplicate_ack_rejection_after_completion() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let coord_inc = id(1001);
    let comp_inc = id(2001);
    let config = sample_lease_config(coord_inc, comp_inc);
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let op_id = RequestId(id(102));
    let win = sample_window(102);
    let native_uuid = id(5002);
    let op = sample_request(op_id, win.clone(), native_uuid);

    let lease_id = session.acquire(op, Tick(100), comp_inc).unwrap();
    session
        .journal_mut()
        .prepare_operation(op_id, Tick(100), coord_inc, 1, comp_inc)
        .unwrap();
    session
        .journal_mut()
        .persist_dispatch_intent(op_id, Tick(100))
        .unwrap();
    session
        .journal_mut()
        .mark_synthetic_dispatch_attempted(op_id)
        .unwrap();

    // First ACK arrives -> Completed
    session
        .complete_lease(
            lease_id,
            op_id,
            TerminalOutcome::Applied,
            comp_inc,
            Tick(120),
        )
        .unwrap();

    let entry = session.journal().get_record(op_id).unwrap();
    assert_eq!(entry.durable_state, DurableOperationState::Completed);

    // Duplicate ACK arrives
    let dup_res = session.complete_lease(
        lease_id,
        op_id,
        TerminalOutcome::Applied,
        comp_inc,
        Tick(130),
    );
    assert!(
        dup_res.is_err(),
        "Duplicate complete_lease must fail because lease is already consumed"
    );

    let final_entry = session.journal().get_record(op_id).unwrap();
    assert_eq!(
        final_entry.durable_state,
        DurableOperationState::Completed,
        "Duplicate ACK must not alter journal state"
    );
}

// ---------------------------------------------------------------------------
// 3. Quarantine Persistence Across Resource Identity Aliases
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_3_10_1_quarantine_persistence_across_identity_aliases() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let coord_inc = id(1001);
    let comp_inc = id(2001);
    let config = sample_lease_config(coord_inc, comp_inc);
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let win_a = sample_window(103);
    let win_b = WindowKey {
        epoch: id(200),
        runtime_id: "aliased-runtime-id".into(),
        generation: "gen-alias".into(),
    };
    let native_uuid = id(5003);

    let op_a = sample_request(RequestId(id(103)), win_a.clone(), native_uuid);

    // Acquire and quarantine
    let lease_id = session.acquire(op_a, Tick(100), comp_inc).unwrap();
    session
        .quarantine_lease(
            lease_id,
            RequestId(id(103)),
            "Quarantine win_a",
            Uncertainty::NoAcknowledgement,
            Tick(150),
        )
        .unwrap();

    assert!(session.lease_manager().is_window_quarantined(&win_a));
    assert!(session.lease_manager().is_native_quarantined(&native_uuid));

    // Attempting to acquire lease for win_b (different key) with same native_uuid must fail!
    let op_b = sample_request(RequestId(id(104)), win_b.clone(), native_uuid);
    let res_b = session.acquire(op_b, Tick(160), comp_inc);
    assert!(
        res_b.is_err(),
        "Acquiring lease on quarantined native UUID via aliased window key must fail"
    );

    // Attempting to acquire lease for win_a with different native_uuid must fail!
    let op_c = sample_request(RequestId(id(105)), win_a.clone(), id(9999));
    let res_c = session.acquire(op_c, Tick(170), comp_inc);
    assert!(
        res_c.is_err(),
        "Acquiring lease on quarantined window key via different native UUID must fail"
    );
}

// ---------------------------------------------------------------------------
// 4. Reconciliation Provenance Fuzzing (Rejecting Fake Provenance)
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_3_10_1_reconciliation_forged_provenance_fuzzing() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let coord_inc = id(1001);
    let comp_inc = id(2001);
    let obs_inc = id(400);
    let config = sample_lease_config(coord_inc, comp_inc);
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let op_id = RequestId(id(104));
    let win = sample_window(104);
    let native_uuid = id(5004);

    let op = sample_request(op_id, win.clone(), native_uuid);
    let lease_id = session.acquire(op, Tick(100), comp_inc).unwrap();
    session
        .quarantine_lease(
            lease_id,
            op_id,
            "Quarantine for reconciliation test",
            Uncertainty::NoAcknowledgement,
            Tick(150),
        )
        .unwrap();

    let trusted_token = "auth-token-secret-999";
    let gateway = AuthoritativeReconciliationGateway::new(trusted_token.to_string(), obs_inc);

    let forged_provenance_candidates = [
        "",
        "depthos.observer",
        "depthos.observer.untrusted",
        "depthos.observer.synthetic",
        "authoritative",
        "depthos.observer.authoritative.fake",
        "root",
        "administrator",
        "DEPTHOS.OBSERVER.AUTHORITATIVE",
    ];

    for bad_prov in forged_provenance_candidates {
        let msg = ReconcileEvidenceMessage {
            protocol_version: 0x0100,
            session_id: Uuid::new_v4(),
            operation_id: op_id,
            evidence: ReconciliationEvidence::new(
                obs_inc,
                15,
                win.clone(),
                native_uuid,
                Placement {
                    desktop: NativeDesktop("desk-dst".into()),
                    activity: "act-main".into(),
                },
                "forged_prov_test",
            ),
            provenance: bad_prov.to_string(),
            authority_token: trusted_token.to_string(),
        };

        let result = gateway.process_reconciliation(&mut session, &msg, Tick(200));
        assert!(
            matches!(result, Err(ReconciliationError::UnauthorizedProvenance(_))),
            "Provenance '{}' must be rejected as UnauthorizedProvenance, got {:?}",
            bad_prov,
            result
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Reconciliation Stolen / Invalid Observer Token Rejection
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_3_10_1_reconciliation_stolen_token_rejected() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let coord_inc = id(1001);
    let comp_inc = id(2001);
    let obs_inc = id(400);
    let config = sample_lease_config(coord_inc, comp_inc);
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let op_id = RequestId(id(105));
    let win = sample_window(105);
    let native_uuid = id(5005);

    let op = sample_request(op_id, win.clone(), native_uuid);
    let lease_id = session.acquire(op, Tick(100), comp_inc).unwrap();
    session
        .quarantine_lease(
            lease_id,
            op_id,
            "Quarantine for stolen token test",
            Uncertainty::NoAcknowledgement,
            Tick(150),
        )
        .unwrap();

    let trusted_token = "auth-token-secret-999";
    let gateway = AuthoritativeReconciliationGateway::new(trusted_token.to_string(), obs_inc);

    let msg = ReconcileEvidenceMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        operation_id: op_id,
        evidence: ReconciliationEvidence::new(
            obs_inc,
            15,
            win.clone(),
            native_uuid,
            Placement {
                desktop: NativeDesktop("desk-dst".into()),
                activity: "act-main".into(),
            },
            "stolen_token_test",
        ),
        provenance: AUTHORITATIVE_PROVENANCE.to_string(),
        authority_token: "stolen-or-forged-token-xyz".to_string(),
    };

    let result = gateway.process_reconciliation(&mut session, &msg, Tick(200));
    assert!(
        matches!(result, Err(ReconciliationError::UnauthorizedToken)),
        "Forged observer token must return UnauthorizedToken, got {:?}",
        result
    );
    assert!(session.lease_manager().is_window_quarantined(&win));
}

// ---------------------------------------------------------------------------
// 6. Reconciliation Stale or Equal Sequence Rejection (`seq <= recorded_seq`)
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_3_10_1_reconciliation_stale_or_equal_sequence_rejected() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let coord_inc = id(1001);
    let comp_inc = id(2001);
    let obs_inc = id(400);
    let config = sample_lease_config(coord_inc, comp_inc); // recorded observer_version_seq = 10
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let op_id = RequestId(id(106));
    let win = sample_window(106);
    let native_uuid = id(5006);

    let op = sample_request(op_id, win.clone(), native_uuid);
    let lease_id = session.acquire(op, Tick(100), comp_inc).unwrap();
    session
        .quarantine_lease(
            lease_id,
            op_id,
            "Quarantine for seq test",
            Uncertainty::NoAcknowledgement,
            Tick(150),
        )
        .unwrap();

    let trusted_token = "auth-token-secret-999";
    let gateway = AuthoritativeReconciliationGateway::new(trusted_token.to_string(), obs_inc);

    // Equal sequence (10 <= 10)
    let msg_equal = ReconcileEvidenceMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        operation_id: op_id,
        evidence: ReconciliationEvidence::new(
            obs_inc,
            10,
            win.clone(),
            native_uuid,
            Placement {
                desktop: NativeDesktop("desk-dst".into()),
                activity: "act-main".into(),
            },
            "equal_seq_test",
        ),
        provenance: AUTHORITATIVE_PROVENANCE.to_string(),
        authority_token: trusted_token.to_string(),
    };
    let res_equal = gateway.process_reconciliation(&mut session, &msg_equal, Tick(200));
    assert!(
        matches!(
            res_equal,
            Err(ReconciliationError::StaleObserverSequence { .. })
        ),
        "Equal sequence (10 <= 10) must return StaleObserverSequence"
    );

    // Older sequence (9 < 10)
    let msg_old = ReconcileEvidenceMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        operation_id: op_id,
        evidence: ReconciliationEvidence::new(
            obs_inc,
            9,
            win.clone(),
            native_uuid,
            Placement {
                desktop: NativeDesktop("desk-dst".into()),
                activity: "act-main".into(),
            },
            "old_seq_test",
        ),
        provenance: AUTHORITATIVE_PROVENANCE.to_string(),
        authority_token: trusted_token.to_string(),
    };
    let res_old = gateway.process_reconciliation(&mut session, &msg_old, Tick(200));
    assert!(
        matches!(
            res_old,
            Err(ReconciliationError::StaleObserverSequence { .. })
        ),
        "Older sequence (9 < 10) must return StaleObserverSequence"
    );
}

// ---------------------------------------------------------------------------
// 7. Reconciliation Observer Incarnation Fencing
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_3_10_1_reconciliation_observer_incarnation_mismatch() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let coord_inc = id(1001);
    let comp_inc = id(2001);
    let active_obs_inc = id(400);
    let stale_obs_inc = id(399);
    let config = sample_lease_config(coord_inc, comp_inc);
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let op_id = RequestId(id(107));
    let win = sample_window(107);
    let native_uuid = id(5007);

    let op = sample_request(op_id, win.clone(), native_uuid);
    let lease_id = session.acquire(op, Tick(100), comp_inc).unwrap();
    session
        .quarantine_lease(
            lease_id,
            op_id,
            "Quarantine for incarnation test",
            Uncertainty::NoAcknowledgement,
            Tick(150),
        )
        .unwrap();

    let trusted_token = "auth-token-secret-999";
    let gateway =
        AuthoritativeReconciliationGateway::new(trusted_token.to_string(), active_obs_inc);

    let msg = ReconcileEvidenceMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        operation_id: op_id,
        evidence: ReconciliationEvidence::new(
            stale_obs_inc,
            15,
            win.clone(),
            native_uuid,
            Placement {
                desktop: NativeDesktop("desk-dst".into()),
                activity: "act-main".into(),
            },
            "stale_incarnation_test",
        ),
        provenance: AUTHORITATIVE_PROVENANCE.to_string(),
        authority_token: trusted_token.to_string(),
    };

    let result = gateway.process_reconciliation(&mut session, &msg, Tick(200));
    assert!(
        matches!(
            result,
            Err(ReconciliationError::ObserverIncarnationMismatch { .. })
        ),
        "Stale observer incarnation must return ObserverIncarnationMismatch, got {:?}",
        result
    );
    assert!(session.lease_manager().is_window_quarantined(&win));
}

// ---------------------------------------------------------------------------
// 8. Reconciliation Source of Truth: Observer Snapshot Only Cannot Lift Quarantine
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_3_10_1_reconciliation_observer_snapshot_only_cannot_lift_quarantine() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let coord_inc = id(1001);
    let comp_inc = id(2001);
    let obs_inc = id(400);
    let config = sample_lease_config(coord_inc, comp_inc);
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let op_id = RequestId(id(108));
    let win = sample_window(108);
    let native_uuid = id(5008);

    let op = sample_request(op_id, win.clone(), native_uuid);
    let lease_id = session.acquire(op, Tick(100), comp_inc).unwrap();
    session
        .quarantine_lease(
            lease_id,
            op_id,
            "Quarantine for snapshot-only source test",
            Uncertainty::NoAcknowledgement,
            Tick(150),
        )
        .unwrap();

    let trusted_token = "auth-token-secret-999";
    let gateway = AuthoritativeReconciliationGateway::new(trusted_token.to_string(), obs_inc);

    // Snapshot only: observes placement, but cannot prove who moved window -> Must fail closed!
    let mut evidence = ReconciliationEvidence::new(
        obs_inc,
        15,
        win.clone(),
        native_uuid,
        Placement {
            desktop: NativeDesktop("desk-dst".into()),
            activity: "act-main".into(),
        },
        "ObserverPlacementSnapshotOnly",
    );
    evidence.evidence_source = EvidenceSource::ObserverSnapshotOnly;

    let msg = ReconcileEvidenceMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        operation_id: op_id,
        evidence,
        provenance: AUTHORITATIVE_PROVENANCE.to_string(),
        authority_token: trusted_token.to_string(),
    };

    let result = gateway.process_reconciliation(&mut session, &msg, Tick(200));
    assert!(
        matches!(
            result,
            Err(ReconciliationError::UnprovenExecutionObserverSnapshotOnly)
        ),
        "ObserverSnapshotOnly must return UnprovenExecutionObserverSnapshotOnly, got {:?}",
        result
    );
    // Quarantine MUST remain locked
    assert!(session.lease_manager().is_window_quarantined(&win));

    // Uncertain outcome -> Must also fail closed and retain quarantine!
    let mut uncertain_evidence = ReconciliationEvidence::new(
        obs_inc,
        16,
        win.clone(),
        native_uuid,
        Placement {
            desktop: NativeDesktop("desk-dst".into()),
            activity: "act-main".into(),
        },
        "UncertainEvidence",
    );
    uncertain_evidence.evidence_source = EvidenceSource::Uncertain;
    uncertain_evidence.terminal_claim = TerminalClaim::Uncertain;

    let uncertain_msg = ReconcileEvidenceMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        operation_id: op_id,
        evidence: uncertain_evidence,
        provenance: AUTHORITATIVE_PROVENANCE.to_string(),
        authority_token: trusted_token.to_string(),
    };

    let result_uncertain = gateway.process_reconciliation(&mut session, &uncertain_msg, Tick(220));
    assert!(
        matches!(result_uncertain, Err(ReconciliationError::UncertainOutcome)),
        "Uncertain source must return UncertainOutcome, got {:?}",
        result_uncertain
    );
    assert!(session.lease_manager().is_window_quarantined(&win));
}

// ---------------------------------------------------------------------------
// 9. Reconciliation Proven Non-Execution and Binding Validations
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_3_10_1_reconciliation_proven_non_execution_and_binding_validations() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let coord_inc = id(1001);
    let comp_inc = id(2001);
    let obs_inc = id(400);
    let config = sample_lease_config(coord_inc, comp_inc);
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let op_id = RequestId(id(109));
    let win = sample_window(109);
    let native_uuid = id(5009);

    let op = sample_request(op_id, win.clone(), native_uuid);
    let lease_id = session.acquire(op, Tick(100), comp_inc).unwrap();
    session
        .quarantine_lease(
            lease_id,
            op_id,
            "Quarantine for proven non-execution test",
            Uncertainty::NoAcknowledgement,
            Tick(150),
        )
        .unwrap();

    let trusted_token = "auth-token-secret-999";
    let gateway = AuthoritativeReconciliationGateway::new(trusted_token.to_string(), obs_inc);

    // Mismatched coordinator incarnation in evidence
    let mut bad_coord_ev = ReconciliationEvidence::new(
        obs_inc,
        15,
        win.clone(),
        native_uuid,
        Placement {
            desktop: NativeDesktop("desk-dst".into()),
            activity: "act-main".into(),
        },
        "BadCoordIncarnation",
    );
    bad_coord_ev.coordinator_incarnation = Some(id(999999));

    let bad_coord_msg = ReconcileEvidenceMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        operation_id: op_id,
        evidence: bad_coord_ev,
        provenance: AUTHORITATIVE_PROVENANCE.to_string(),
        authority_token: trusted_token.to_string(),
    };
    let err_coord = gateway.process_reconciliation(&mut session, &bad_coord_msg, Tick(200));
    assert!(
        matches!(
            err_coord,
            Err(ReconciliationError::CoordinatorIncarnationMismatch { .. })
        ),
        "Mismatched coordinator incarnation must be rejected"
    );

    // Mismatched compositor incarnation in evidence
    let mut bad_comp_ev = ReconciliationEvidence::new(
        obs_inc,
        15,
        win.clone(),
        native_uuid,
        Placement {
            desktop: NativeDesktop("desk-dst".into()),
            activity: "act-main".into(),
        },
        "BadCompIncarnation",
    );
    bad_comp_ev.compositor_incarnation = Some(id(888888));

    let bad_comp_msg = ReconcileEvidenceMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        operation_id: op_id,
        evidence: bad_comp_ev,
        provenance: AUTHORITATIVE_PROVENANCE.to_string(),
        authority_token: trusted_token.to_string(),
    };
    let err_comp = gateway.process_reconciliation(&mut session, &bad_comp_msg, Tick(200));
    assert!(
        matches!(
            err_comp,
            Err(ReconciliationError::CompositorIncarnationMismatch { .. })
        ),
        "Mismatched compositor incarnation must be rejected"
    );

    // Valid Proven Non-Execution -> Lifts quarantine!
    let mut proven_ev = ReconciliationEvidence::new(
        obs_inc,
        15,
        win.clone(),
        native_uuid,
        Placement {
            desktop: NativeDesktop("desk-dst".into()),
            activity: "act-main".into(),
        },
        "ProvenNonExecutionTerminalReceipt",
    );
    proven_ev.evidence_source = EvidenceSource::ProvenNonExecution;
    proven_ev.terminal_claim = TerminalClaim::Rejected;
    proven_ev.coordinator_incarnation = Some(coord_inc);
    proven_ev.compositor_incarnation = Some(comp_inc);
    proven_ev.operation_id = Some(op_id);
    proven_ev.lease_id = Some(lease_id);

    let proven_msg = ReconcileEvidenceMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        operation_id: op_id,
        evidence: proven_ev,
        provenance: AUTHORITATIVE_PROVENANCE.to_string(),
        authority_token: trusted_token.to_string(),
    };
    let receipt = gateway
        .process_reconciliation(&mut session, &proven_msg, Tick(250))
        .expect("ProvenNonExecution must succeed");
    assert_eq!(receipt.final_state, DurableOperationState::Reconciled);

    // Window must now be free from quarantine
    assert!(!session.lease_manager().is_window_quarantined(&win));
}
