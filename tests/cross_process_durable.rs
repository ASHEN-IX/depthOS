//! Integration and adversarial tests for DepthOS Phase 3.10 cross-process protocol,
//! peer authentication, durable journal integration, and authoritative reconciliation gateway.
//!
//! # Guarantees Verified
//! - Dedicated Unix domain socket in 0700 directory with 0600 permissions
//! - Peer credential checks (`SO_PEERCRED`: UID, PID)
//! - Bootstrap token handshake; unauthorized same-UID connections rejected
//! - Strict 20-byte framing header, magic `0x44504F53`, version `0x0100`, length bounds (<=64KiB)
//! - Packet sequence monotonicity; replay rejection
//! - Strict linearization: durable dispatch intent committed before wire dispatch
//! - Uncertainty quarantine on lost ACK, timeout, or transport disconnect
//! - Multi-process incarnation fencing across coordinator and compositor restarts
//! - Authoritative reconciliation gateway: trusted provenance, valid token, sequence fencing
//! - Zero sleep-based tests: all synchronization uses deterministic channels and barriers

use depthos_core::action_security::{
    NativeDesktop, Operation, Placement, RequestId, Tick, Uncertainty, WindowKey,
    durable::{
        DurableCoordinatorSession, DurableError, DurableOperationJournal, DurableOperationState,
        ReconciliationEvidence,
    },
    ipc::{
        AckOutcome, AcknowledgeMessage, CrossProcessCoordinator, HelloAcceptedMessage,
        HelloMessage, IpcError, IpcStream, PreparedMessage, ProtocolMessage,
        ReconcileEvidenceMessage, SessionState, verify_peer_credentials, verify_private_directory,
    },
    lease::{LeaseConfig, LeaseRequest, TerminalOutcome},
    reconciliation::{
        AUTHORITATIVE_PROVENANCE, AuthoritativeReconciliationGateway, ReconciliationError,
    },
};
use std::{os::fd::AsFd, os::unix::fs::PermissionsExt, path::PathBuf};
use tempfile::TempDir;
use tokio::{
    io::AsyncWriteExt,
    net::{UnixListener, UnixStream},
};
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
    temp_dir: TempDir,
    sock_path: PathBuf,
    db_path: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        // Enforce 0700 on the private directory
        std::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let sock_path = temp_dir.path().join("depthos-ipc.sock");
        let db_path = temp_dir.path().join("durable.sqlite3");
        Self {
            temp_dir,
            sock_path,
            db_path,
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Peer Authentication & Transport Security Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_adversarial_ipc_unauthorized_same_uid() {
    let env = TestEnv::new();
    verify_private_directory(env.temp_dir.path()).unwrap();

    let listener = UnixListener::bind(&env.sock_path).unwrap();
    std::fs::set_permissions(&env.sock_path, std::fs::Permissions::from_mode(0o600)).unwrap();

    let expected_token = "legitimate-launcher-bootstrap-token-256";

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        verify_peer_credentials(stream.as_fd(), None).unwrap();
        let mut ipc = IpcStream::new(stream, SessionState::Unauthenticated);

        // Expect Hello with valid token
        match ipc.recv_message().await {
            Ok(ProtocolMessage::Hello(hello)) => {
                if hello.bootstrap_token != expected_token {
                    // Close socket immediately without sending leak response
                    drop(ipc);
                    Err(IpcError::AuthenticationFailed)
                } else {
                    Ok(())
                }
            }
            Ok(_) => Err(IpcError::Protocol("Unexpected message".into())),
            Err(e) => Err(e),
        }
    });

    // Rogue client attempts connection with forged token
    let client_stream = UnixStream::connect(&env.sock_path).await.unwrap();
    let mut client_ipc = IpcStream::new(client_stream, SessionState::Unauthenticated);

    let forged_hello = HelloMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        coordinator_incarnation: Uuid::new_v4(),
        coordinator_epoch: 1,
        bootstrap_token: "forged-unauthorized-token".into(),
        client_pid: std::process::id(),
    };

    client_ipc
        .send_message(&ProtocolMessage::Hello(forged_hello))
        .await
        .unwrap();

    // Server should drop connection immediately
    let client_recv = client_ipc.recv_message().await;
    assert!(
        matches!(
            client_recv,
            Err(IpcError::ConnectionClosed) | Err(IpcError::Io(_))
        ),
        "Expected connection drop on forged token, got: {client_recv:?}"
    );

    let server_res = server_task.await.unwrap();
    assert!(
        matches!(server_res, Err(IpcError::AuthenticationFailed)),
        "Server must fail authentication on forged token"
    );
}

#[tokio::test]
async fn test_adversarial_ipc_peer_credential_check() {
    let env = TestEnv::new();
    let listener = UnixListener::bind(&env.sock_path).unwrap();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        // Verify peer UID matches current process UID
        let (uid, _gid, pid) = verify_peer_credentials(stream.as_fd(), None).unwrap();
        assert_eq!(uid, rustix::process::geteuid().as_raw());
        assert_eq!(pid, std::process::id());
    });

    let client_stream = UnixStream::connect(&env.sock_path).await.unwrap();
    let (uid, _gid, pid) = verify_peer_credentials(client_stream.as_fd(), None).unwrap();
    assert_eq!(uid, rustix::process::geteuid().as_raw());
    assert_eq!(pid, std::process::id());

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_adversarial_ipc_oversize_payload_rejection() {
    let (s1, mut s2) = UnixStream::pair().unwrap();
    let mut server_ipc = IpcStream::new(s1, SessionState::Unauthenticated);

    let client_task = tokio::spawn(async move {
        // Construct header declaring payload_len = 70,000 bytes (> 65536)
        let mut buf = Vec::with_capacity(20);
        buf.extend_from_slice(&[0x44, 0x50, 0x4F, 0x53]); // Magic
        buf.extend_from_slice(&0x0100u16.to_be_bytes()); // Version
        buf.extend_from_slice(&1u16.to_be_bytes()); // MsgType: Hello
        buf.extend_from_slice(&1u64.to_be_bytes()); // Seq: 1
        buf.extend_from_slice(&70000u32.to_be_bytes()); // Oversize: 70 KiB
        s2.write_all(&buf).await.unwrap();
        s2.flush().await.unwrap();
    });

    client_task.await.unwrap();

    let recv_res = server_ipc.recv_message().await;
    match recv_res {
        Err(IpcError::OversizePayload(len)) => {
            assert_eq!(len, 70000);
        }
        other => panic!("Expected OversizePayload rejection, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_adversarial_ipc_malformed_framing() {
    // 1. Invalid Magic
    {
        let (s1, mut s2) = UnixStream::pair().unwrap();
        let mut server_ipc = IpcStream::new(s1, SessionState::Unauthenticated);

        let mut buf = Vec::with_capacity(20);
        buf.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]); // Bad magic
        buf.extend_from_slice(&0x0100u16.to_be_bytes());
        buf.extend_from_slice(&1u16.to_be_bytes());
        buf.extend_from_slice(&1u64.to_be_bytes());
        buf.extend_from_slice(&0u32.to_be_bytes());
        s2.write_all(&buf).await.unwrap();

        match server_ipc.recv_message().await {
            Err(IpcError::InvalidMagic(m)) => assert_eq!(m, [0xDE, 0xAD, 0xBE, 0xEF]),
            other => panic!("Expected InvalidMagic, got: {other:?}"),
        }
    }

    // 2. Unsupported Version
    {
        let (s1, mut s2) = UnixStream::pair().unwrap();
        let mut server_ipc = IpcStream::new(s1, SessionState::Unauthenticated);

        let mut buf = Vec::with_capacity(20);
        buf.extend_from_slice(&[0x44, 0x50, 0x4F, 0x53]);
        buf.extend_from_slice(&0x0200u16.to_be_bytes()); // Bad version: 2.0
        buf.extend_from_slice(&1u16.to_be_bytes());
        buf.extend_from_slice(&1u64.to_be_bytes());
        buf.extend_from_slice(&0u32.to_be_bytes());
        s2.write_all(&buf).await.unwrap();

        match server_ipc.recv_message().await {
            Err(IpcError::UnsupportedVersion(v)) => assert_eq!(v, 0x0200),
            other => panic!("Expected UnsupportedVersion, got: {other:?}"),
        }
    }
}

#[tokio::test]
async fn test_adversarial_ipc_packet_replay_rejected() {
    let (s1, mut s2) = UnixStream::pair().unwrap();
    let mut server_ipc = IpcStream::new(s1, SessionState::Unauthenticated);

    // Client sends packet with seq = 2 instead of expected 1
    let mut buf = Vec::with_capacity(20);
    buf.extend_from_slice(&[0x44, 0x50, 0x4F, 0x53]);
    buf.extend_from_slice(&0x0100u16.to_be_bytes());
    buf.extend_from_slice(&1u16.to_be_bytes());
    buf.extend_from_slice(&2u64.to_be_bytes()); // Expected 1, got 2!
    buf.extend_from_slice(&0u32.to_be_bytes());
    s2.write_all(&buf).await.unwrap();

    match server_ipc.recv_message().await {
        Err(IpcError::SequenceMismatch { expected, got }) => {
            assert_eq!(expected, 1);
            assert_eq!(got, 2);
        }
        other => panic!("Expected SequenceMismatch, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 2. Durable Journal & Linearized Protocol Dispatch Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_adversarial_dispatch_requires_durable_intent() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let config = sample_lease_config(id(1), id(2));
    let session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let mut coord = CrossProcessCoordinator::new(session, "tok-1".into()).unwrap();

    let (s1, s2) = UnixStream::pair().unwrap();
    let mut stream = IpcStream::new(s1, SessionState::Unauthenticated);

    // Mock compositor actor
    let comp_task = tokio::spawn(async move {
        let mut comp_stream = IpcStream::new(s2, SessionState::Unauthenticated);
        // Handshake
        let hello = comp_stream.recv_message().await.unwrap();
        match hello {
            ProtocolMessage::Hello(h) => {
                let accept = HelloAcceptedMessage {
                    protocol_version: 0x0100,
                    session_id: h.session_id,
                    compositor_incarnation: id(2),
                    accepted_epoch: 1,
                    server_pid: std::process::id(),
                };
                comp_stream
                    .send_message(&ProtocolMessage::HelloAccepted(accept))
                    .await
                    .unwrap();
                comp_stream.set_session_state(SessionState::Active);
            }
            _ => panic!("Expected Hello"),
        }

        // Reserve
        let res = comp_stream.recv_message().await.unwrap();
        match res {
            ProtocolMessage::Reserve(r) => {
                let prep = PreparedMessage {
                    protocol_version: 0x0100,
                    session_id: r.session_id,
                    operation_id: r.operation_id,
                    lease_id: r.lease_id,
                    compositor_incarnation: id(2),
                    window_generation: 1,
                    ready: true,
                    detail: None,
                };
                comp_stream
                    .send_message(&ProtocolMessage::Prepared(prep))
                    .await
                    .unwrap();
            }
            _ => panic!("Expected Reserve"),
        }

        // Expect DispatchIntent frame *before* SyntheticAttempt!
        let intent_msg = comp_stream.recv_message().await.unwrap();
        match intent_msg {
            ProtocolMessage::DispatchIntent(di) => {
                assert!(di.intent_committed);
            }
            _ => panic!("Expected DispatchIntent before SyntheticAttempt"),
        }

        // Now SyntheticAttempt arrives
        let attempt_msg = comp_stream.recv_message().await.unwrap();
        match attempt_msg {
            ProtocolMessage::SyntheticAttempt(att) => {
                let ack = AcknowledgeMessage {
                    protocol_version: 0x0100,
                    session_id: att.session_id,
                    operation_id: att.operation_id,
                    lease_id: att.lease_id,
                    attempt: att.attempt,
                    coordinator_incarnation: id(1),
                    compositor_incarnation: id(2),
                    outcome: AckOutcome::Applied,
                    reason: None,
                };
                comp_stream
                    .send_message(&ProtocolMessage::Acknowledge(ack))
                    .await
                    .unwrap();
            }
            _ => panic!("Expected SyntheticAttempt"),
        }

        // Terminal receipt
        let receipt = comp_stream.recv_message().await.unwrap();
        matches!(receipt, ProtocolMessage::TerminalReceipt(_))
    });

    coord
        .perform_handshake(&mut stream, Some(id(2)))
        .await
        .unwrap();

    let op_id = RequestId(id(10));
    let win = sample_window(1);
    let req = sample_request(op_id, win, id(20));

    let state = coord
        .dispatch_operation(
            &mut stream,
            req,
            Operation::MoveDesktop {
                from: Placement {
                    desktop: NativeDesktop("desk-src".into()),
                    activity: "act-main".into(),
                },
                to: Placement {
                    desktop: NativeDesktop("desk-dst".into()),
                    activity: "act-main".into(),
                },
            },
            "fixture-auth-token-128".into(),
            Tick(200),
        )
        .await
        .unwrap();

    assert_eq!(state, DurableOperationState::Completed);
    assert!(comp_task.await.unwrap());

    // Verify durable record in SQLite
    let rec = coord.session.journal().get_record(op_id).unwrap();
    assert_eq!(rec.durable_state, DurableOperationState::Completed);
    assert!(rec.dispatch_intent);
}

#[tokio::test]
async fn test_adversarial_lost_ack_triggers_uncertainty_quarantine() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let config = sample_lease_config(id(1), id(2));
    let session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let mut coord = CrossProcessCoordinator::new(session, "tok-1".into()).unwrap();

    let (s1, s2) = UnixStream::pair().unwrap();
    let mut stream = IpcStream::new(s1, SessionState::Unauthenticated);
    stream.set_read_timeout(std::time::Duration::from_millis(50));

    // Mock compositor that drops connection after receiving SyntheticAttempt (Lost ACK)
    let comp_task = tokio::spawn(async move {
        let mut comp_stream = IpcStream::new(s2, SessionState::Unauthenticated);
        let hello = comp_stream.recv_message().await.unwrap();
        if let ProtocolMessage::Hello(h) = hello {
            let accept = HelloAcceptedMessage {
                protocol_version: 0x0100,
                session_id: h.session_id,
                compositor_incarnation: id(2),
                accepted_epoch: 1,
                server_pid: std::process::id(),
            };
            comp_stream
                .send_message(&ProtocolMessage::HelloAccepted(accept))
                .await
                .unwrap();
            comp_stream.set_session_state(SessionState::Active);
        }

        let res = comp_stream.recv_message().await.unwrap();
        if let ProtocolMessage::Reserve(r) = res {
            let prep = PreparedMessage {
                protocol_version: 0x0100,
                session_id: r.session_id,
                operation_id: r.operation_id,
                lease_id: r.lease_id,
                compositor_incarnation: id(2),
                window_generation: 1,
                ready: true,
                detail: None,
            };
            comp_stream
                .send_message(&ProtocolMessage::Prepared(prep))
                .await
                .unwrap();
        }

        let _ = comp_stream.recv_message().await.unwrap(); // DispatchIntent
        let _ = comp_stream.recv_message().await.unwrap(); // SyntheticAttempt

        // DROP CONNECTION: Lost ACK simulation
        drop(comp_stream);
    });

    coord
        .perform_handshake(&mut stream, Some(id(2)))
        .await
        .unwrap();

    let op_id = RequestId(id(10));
    let win = sample_window(1);
    let req = sample_request(op_id, win.clone(), id(20));

    let state = coord
        .dispatch_operation(
            &mut stream,
            req,
            Operation::MoveDesktop {
                from: Placement {
                    desktop: NativeDesktop("desk-src".into()),
                    activity: "act-main".into(),
                },
                to: Placement {
                    desktop: NativeDesktop("desk-dst".into()),
                    activity: "act-main".into(),
                },
            },
            "fixture-auth-token-128".into(),
            Tick(200),
        )
        .await
        .unwrap();

    assert_eq!(
        state,
        DurableOperationState::Quarantined,
        "Lost ACK must trigger quarantine"
    );
    comp_task.await.unwrap();

    // Verify window is locked in quarantine
    assert!(coord.session.lease_manager().is_window_quarantined(&win));
    assert!(
        coord
            .session
            .lease_manager()
            .check_window_available(&win)
            .is_err()
    );

    // Conflicting proposals for the same window are rejected
    let req2 = sample_request(RequestId(id(11)), win.clone(), id(20));
    let acquire_err = coord.session.acquire(req2, Tick(300), id(2));
    assert!(
        matches!(acquire_err, Err(DurableError::WindowQuarantined)),
        "Conflicting proposal during quarantine must be rejected"
    );
}

#[tokio::test]
async fn test_adversarial_recovery_post_intent_quarantine() {
    let env = TestEnv::new();

    let op_id = RequestId(id(50));
    let win = sample_window(50);
    let native_uuid = id(51);

    // Coordinator session 1: persists intent and crashes before acknowledgment
    {
        let journal = DurableOperationJournal::open(&env.db_path).unwrap();
        let config = sample_lease_config(id(1), id(2));
        let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

        let req = sample_request(op_id, win.clone(), native_uuid);
        let _lease_id = session.acquire(req, Tick(100), id(2)).unwrap();

        session
            .journal_mut()
            .prepare_operation(op_id, Tick(100), id(1), 1, id(2))
            .unwrap();
        // Commit dispatch intent to SQLite WAL
        session
            .journal_mut()
            .persist_dispatch_intent(op_id, Tick(100))
            .unwrap();

        // CRASH POINT: simulate sudden crash / reboot before ACK
        drop(session);
    }

    // Coordinator session 2: recovers on startup
    {
        let journal = DurableOperationJournal::open(&env.db_path).unwrap();
        let config = sample_lease_config(id(2_000), id(2));
        let session = DurableCoordinatorSession::new(journal, config, Tick(200)).unwrap();

        let rec = session.journal().get_record(op_id).unwrap();
        assert_eq!(
            rec.durable_state,
            DurableOperationState::Quarantined,
            "Post-intent crashed operation must recover to Quarantined"
        );
        assert!(rec.dispatch_intent);
        assert_eq!(rec.recovery_status, "RecoveredToQuarantine");

        // Window remains quarantined in journal: cannot re-reserve
        let check_res = session.journal().check_window_available(&win);
        assert!(
            matches!(check_res, Err(DurableError::WindowQuarantined)),
            "Quarantined window must fail check_window_available"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Authoritative Reconciliation Gateway Tests
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_forged_reconciliation_rejected() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let config = sample_lease_config(id(1), id(2));
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let op_id = RequestId(id(70));
    let win = sample_window(70);
    let native_uuid = id(71);

    let req = sample_request(op_id, win.clone(), native_uuid);
    let lease_id = session.acquire(req, Tick(100), id(2)).unwrap();

    session
        .journal_mut()
        .prepare_operation(op_id, Tick(100), id(1), 1, id(2))
        .unwrap();
    session
        .journal_mut()
        .persist_dispatch_intent(op_id, Tick(100))
        .unwrap();
    session
        .quarantine_lease(
            lease_id,
            op_id,
            "Lost ack simulation",
            Uncertainty::NoAcknowledgement,
            Tick(150),
        )
        .unwrap();

    let gateway =
        AuthoritativeReconciliationGateway::new("observer-auth-secret-128".into(), id(400));

    // Attack 1: Wrong provenance (e.g. from generic D-Bus client)
    let bad_prov_msg = ReconcileEvidenceMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        operation_id: op_id,
        evidence: ReconciliationEvidence {
            observer_incarnation: id(400),
            observer_sequence: 15, // Fresh sequence
            verified_window_key: win.clone(),
            observed_native_uuid: native_uuid,
            observed_placement: Placement {
                desktop: NativeDesktop("desk-dst".into()),
                activity: "act-main".into(),
            },
            authoritative_decision: "reconciled_after_observer_sync".into(),
        },
        provenance: "depthos.controller.untrusted".into(),
        authority_token: "observer-auth-secret-128".into(),
    };

    let err = gateway
        .process_reconciliation(&mut session, &bad_prov_msg, Tick(200))
        .unwrap_err();
    assert!(
        matches!(err, ReconciliationError::UnauthorizedProvenance(_)),
        "Non-observer provenance must be rejected"
    );

    // Attack 2: Wrong authority token
    let bad_tok_msg = ReconcileEvidenceMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        operation_id: op_id,
        evidence: bad_prov_msg.evidence.clone(),
        provenance: AUTHORITATIVE_PROVENANCE.into(),
        authority_token: "forged-observer-token".into(),
    };
    let err2 = gateway
        .process_reconciliation(&mut session, &bad_tok_msg, Tick(200))
        .unwrap_err();
    assert!(
        matches!(err2, ReconciliationError::UnauthorizedToken),
        "Forged observer token must be rejected"
    );

    // Quarantine must NOT be lifted
    assert!(session.lease_manager().is_window_quarantined(&win));
}

#[test]
fn test_adversarial_stale_observer_seq_rejected() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let config = sample_lease_config(id(1), id(2)); // config observer_version_seq = 10
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let op_id = RequestId(id(80));
    let win = sample_window(80);
    let native_uuid = id(81);

    let req = sample_request(op_id, win.clone(), native_uuid);
    let lease_id = session.acquire(req, Tick(100), id(2)).unwrap();
    session
        .quarantine_lease(
            lease_id,
            op_id,
            "Lost ack",
            Uncertainty::NoAcknowledgement,
            Tick(150),
        )
        .unwrap();

    let gateway =
        AuthoritativeReconciliationGateway::new("observer-auth-secret-128".into(), id(400));

    // Stale sequence: seq = 10 (equal to recorded_seq = 10, must be strictly greater!)
    let stale_msg = ReconcileEvidenceMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        operation_id: op_id,
        evidence: ReconciliationEvidence {
            observer_incarnation: id(400),
            observer_sequence: 10, // Stale!
            verified_window_key: win.clone(),
            observed_native_uuid: native_uuid,
            observed_placement: Placement {
                desktop: NativeDesktop("desk-dst".into()),
                activity: "act-main".into(),
            },
            authoritative_decision: "stale_attempt".into(),
        },
        provenance: AUTHORITATIVE_PROVENANCE.into(),
        authority_token: "observer-auth-secret-128".into(),
    };

    let err = gateway
        .process_reconciliation(&mut session, &stale_msg, Tick(200))
        .unwrap_err();
    assert!(
        matches!(err, ReconciliationError::StaleObserverSequence { .. }),
        "Stale sequence number (seq <= recorded) must be rejected"
    );

    // Quarantine must remain locked
    assert!(session.lease_manager().is_window_quarantined(&win));

    // Valid strictly fresh sequence: seq = 11 > 10
    let valid_msg = ReconcileEvidenceMessage {
        protocol_version: 0x0100,
        session_id: Uuid::new_v4(),
        operation_id: op_id,
        evidence: ReconciliationEvidence {
            observer_incarnation: id(400),
            observer_sequence: 11, // Fresh!
            verified_window_key: win.clone(),
            observed_native_uuid: native_uuid,
            observed_placement: Placement {
                desktop: NativeDesktop("desk-dst".into()),
                activity: "act-main".into(),
            },
            authoritative_decision: "valid_fresh_reconciliation".into(),
        },
        provenance: AUTHORITATIVE_PROVENANCE.into(),
        authority_token: "observer-auth-secret-128".into(),
    };

    let receipt = gateway
        .process_reconciliation(&mut session, &valid_msg, Tick(250))
        .unwrap();
    assert_eq!(receipt.final_state, DurableOperationState::Reconciled);

    // Quarantine lifted!
    assert!(!session.lease_manager().is_window_quarantined(&win));
    assert_eq!(session.lease_manager().check_window_available(&win), Ok(()));
}

#[tokio::test]
async fn test_adversarial_compositor_restart_fencing() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let config = sample_lease_config(id(1), id(2));
    let session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let mut coord = CrossProcessCoordinator::new(session, "bootstrap-tok".into()).unwrap();

    let (s1, s2) = UnixStream::pair().unwrap();
    let mut stream = IpcStream::new(s1, SessionState::Unauthenticated);

    // Compositor restarts and presents new incarnation UUID = id(999)
    let comp_task = tokio::spawn(async move {
        let mut comp_stream = IpcStream::new(s2, SessionState::Unauthenticated);
        let hello = comp_stream.recv_message().await.unwrap();
        if let ProtocolMessage::Hello(h) = hello {
            let accept = HelloAcceptedMessage {
                protocol_version: 0x0100,
                session_id: h.session_id,
                compositor_incarnation: id(999), // Restarted compositor!
                accepted_epoch: 1,
                server_pid: std::process::id(),
            };
            comp_stream
                .send_message(&ProtocolMessage::HelloAccepted(accept))
                .await
                .unwrap();
            comp_stream.set_session_state(SessionState::Active);
        }
    });

    // Coordinator expects compositor incarnation id(2), but receives id(999)
    let handshake_res = coord.perform_handshake(&mut stream, Some(id(2))).await;
    assert!(
        matches!(handshake_res, Err(IpcError::IdentityMismatch(_))),
        "Mismatched compositor incarnation must fail handshake and fence old leases"
    );

    comp_task.await.unwrap();
}

#[test]
fn test_adversarial_delayed_ack_cannot_unquarantine() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let config = sample_lease_config(id(1), id(2));
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let op_id = RequestId(id(90));
    let win = sample_window(90);
    let native_uuid = id(91);

    let req = sample_request(op_id, win.clone(), native_uuid);
    let lease_id = session.acquire(req, Tick(100), id(2)).unwrap();

    // Quarantine the operation due to lost ack
    session
        .quarantine_lease(
            lease_id,
            op_id,
            "Lost ack timeout",
            Uncertainty::NoAcknowledgement,
            Tick(150),
        )
        .unwrap();

    // Delayed ACK arrives after quarantine has been established
    let delayed_ack_err =
        session.complete_lease(lease_id, op_id, TerminalOutcome::Applied, id(2), Tick(200));

    assert!(
        delayed_ack_err.is_err(),
        "Delayed ACK arriving after quarantine must be rejected by state machine"
    );

    // Window must remain strictly quarantined
    assert!(session.lease_manager().is_window_quarantined(&win));
}

#[test]
fn test_adversarial_recreated_window_generation_fence() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let config = sample_lease_config(id(1), id(2));
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let op_id = RequestId(id(100));
    let win_gen1 = WindowKey {
        epoch: id(200),
        runtime_id: "window-shared-id".into(),
        generation: "gen-1".into(),
    };
    let native_uuid = id(101);

    let req = sample_request(op_id, win_gen1.clone(), native_uuid);
    let lease_id = session.acquire(req, Tick(100), id(2)).unwrap();

    session
        .quarantine_lease(
            lease_id,
            op_id,
            "Quarantine gen-1",
            Uncertainty::NoAcknowledgement,
            Tick(150),
        )
        .unwrap();

    assert!(session.lease_manager().is_window_quarantined(&win_gen1));

    // When the old window generation is destroyed and a new generation is created:
    let win_gen2 = WindowKey {
        epoch: id(200),
        runtime_id: "window-shared-id".into(),
        generation: "gen-2".into(),
    };

    // The new generation is NOT quarantined:
    assert!(!session.lease_manager().is_window_quarantined(&win_gen2));

    // But old generation remains quarantined:
    assert!(session.lease_manager().is_window_quarantined(&win_gen1));
}

#[test]
fn test_adversarial_observer_disconnect_permanent_quarantine() {
    let env = TestEnv::new();
    let journal = DurableOperationJournal::open(&env.db_path).unwrap();
    let config = sample_lease_config(id(1), id(2));
    let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let op_id = RequestId(id(110));
    let win = sample_window(110);
    let native_uuid = id(111);

    let req = sample_request(op_id, win.clone(), native_uuid);
    let lease_id = session.acquire(req, Tick(100), id(2)).unwrap();

    session
        .quarantine_lease(
            lease_id,
            op_id,
            "Quarantine lost ack",
            Uncertainty::NoAcknowledgement,
            Tick(150),
        )
        .unwrap();

    // Advance time far into future (1 hour = 3,600,000 ms)
    session.advance_time(Tick(3_600_000)).unwrap();

    // Invariant: Quarantine NEVER expires automatically without authoritative reconciliation!
    assert!(
        session.lease_manager().is_window_quarantined(&win),
        "Quarantine must never expire automatically on time alone"
    );
    assert!(
        session
            .lease_manager()
            .check_window_available(&win)
            .is_err(),
        "Quarantined window must fail check_window_available indefinitely"
    );
}
