//! Cross-process native coordinator client for Phase 3.10.1 integration testing.
use depthos_core::action_security::{
    NativeDesktop, Operation, Placement, RequestId, Tick, WindowKey,
    durable::{DurableCoordinatorSession, DurableOperationJournal, DurableOperationState},
    ipc::{CrossProcessCoordinator, IpcStream, SessionState, verify_private_socket},
    lease::{LeaseConfig, LeaseRequest},
};
use std::{env, path::PathBuf, process};
use tokio::net::UnixStream;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let mut socket_path = PathBuf::from("/tmp/depthos-ipc.sock");
    let mut db_path = PathBuf::from("/tmp/durable.sqlite3");
    let mut bootstrap_token = "launcher-bootstrap-token-phase-3-10-1".to_string();
    let mut fixture_token = "fixture-authority-token-kwin-private".to_string();
    let mut scenario = "happy_path".to_string();
    let mut compositor_inc = Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap();
    let mut window_uuid = Uuid::new_v4();
    let mut generation = 1u64;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--socket" if i + 1 < args.len() => {
                socket_path = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--db" if i + 1 < args.len() => {
                db_path = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--scenario" if i + 1 < args.len() => {
                scenario = args[i + 1].clone();
                i += 2;
            }
            "--compositor-incarnation" if i + 1 < args.len() => {
                compositor_inc = Uuid::parse_str(&args[i + 1]).unwrap_or(compositor_inc);
                i += 2;
            }
            "--bootstrap-token" if i + 1 < args.len() => {
                bootstrap_token = args[i + 1].clone();
                i += 2;
            }
            "--fixture-token" if i + 1 < args.len() => {
                fixture_token = args[i + 1].clone();
                i += 2;
            }
            "--window-uuid" if i + 1 < args.len() => {
                window_uuid = Uuid::parse_str(&args[i + 1]).unwrap_or_else(|_| Uuid::new_v4());
                i += 2;
            }
            "--generation" if i + 1 < args.len() => {
                generation = args[i + 1].parse().unwrap_or(1);
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }

    println!(
        "[COORDINATOR] Verifying socket permissions: {}",
        socket_path.display()
    );
    if let Err(e) = verify_private_socket(&socket_path) {
        eprintln!("[COORDINATOR] Socket permission check failed: {e}");
        process::exit(1);
    }

    println!("[COORDINATOR] Connecting to {}", socket_path.display());
    let stream = match UnixStream::connect(&socket_path).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[COORDINATOR] Connect failed: {e}");
            process::exit(1);
        }
    };

    let mut ipc_stream = IpcStream::new(stream, SessionState::Unauthenticated);

    let journal = DurableOperationJournal::open(&db_path).expect("open journal");
    let coord_inc = Uuid::new_v4();
    let _comp_inc = Uuid::new_v4();
    let config = LeaseConfig {
        coordinator_incarnation: coord_inc,
        coordinator_epoch: 1,
        checkpoint_epoch: Uuid::new_v4(),
        checkpoint_sequence: 1,
        observer_incarnation: Uuid::new_v4(),
        observer_version_seq: 1,
        policy_revision: Uuid::new_v4(),
        topology_revision: Uuid::new_v4(),
        compositor_incarnation: compositor_inc,
    };

    let session = DurableCoordinatorSession::new(journal, config, Tick(100)).expect("new session");
    let mut coordinator =
        CrossProcessCoordinator::new(session, bootstrap_token).expect("new coordinator");

    // 1. Handshake
    println!("[COORDINATOR] Performing Hello handshake...");
    let hello_accepted = coordinator
        .perform_handshake(&mut ipc_stream, None)
        .await
        .expect("handshake must succeed");
    println!(
        "[COORDINATOR] Handshake accepted by compositor pid={}",
        hello_accepted.server_pid
    );

    let op_id = RequestId(Uuid::new_v4());
    let window = WindowKey {
        epoch: Uuid::new_v4(),
        runtime_id: "window-target-runtime".into(),
        generation: format!("gen-{generation}"),
    };

    let op = Operation::MoveDesktop {
        from: Placement {
            desktop: NativeDesktop("source".into()),
            activity: "act-1".into(),
        },
        to: Placement {
            desktop: NativeDesktop("destination".into()),
            activity: "act-1".into(),
        },
    };

    let req = LeaseRequest {
        operation_id: op_id,
        window,
        native_uuid: window_uuid,
        operation: op.clone(),
        ttl_ms: 5000,
    };

    match scenario.as_str() {
        "happy_path" => {
            println!("[COORDINATOR] Executing happy_path dispatch...");
            let final_state = coordinator
                .dispatch_operation(
                    &mut ipc_stream,
                    req.clone(),
                    op.clone(),
                    fixture_token.clone(),
                    Tick(110),
                )
                .await
                .expect("dispatch_operation must succeed");
            assert_eq!(final_state, DurableOperationState::Completed);
            println!("[COORDINATOR] PASS: Operation completed with state Completed");

            // Verify replay attempt on same coordinator driver is rejected
            println!("[COORDINATOR] Verifying replay rejection...");
            let replay_res = coordinator
                .dispatch_operation(&mut ipc_stream, req, op, fixture_token, Tick(120))
                .await;
            assert!(replay_res.is_err(), "Replay attempt must be rejected");
            println!("[COORDINATOR] PASS: Replay attempt rejected");
        }
        "prepare_failure" => {
            println!("[COORDINATOR] Executing prepare_failure scenario...");
            let res = coordinator
                .dispatch_operation(&mut ipc_stream, req, op, fixture_token, Tick(110))
                .await;
            assert_eq!(
                res.expect("dispatch_operation should return Ok(Aborted)"),
                DurableOperationState::Aborted,
                "Dispatch should terminate with Aborted when prepare conditions are unmet"
            );
            println!("[COORDINATOR] PASS: Prepare failure handled with refusal");
        }
        "synthetic_denied" => {
            println!("[COORDINATOR] Executing synthetic_denied scenario...");
            let res = coordinator
                .dispatch_operation(&mut ipc_stream, req, op, fixture_token, Tick(110))
                .await;
            assert_eq!(
                res.expect("dispatch_operation should return Ok(Aborted)"),
                DurableOperationState::Aborted,
                "Dispatch should terminate with Aborted when synthetic is denied"
            );
            println!("[COORDINATOR] PASS: Synthetic denial handled correctly");
        }
        other => {
            eprintln!("[COORDINATOR] Unknown scenario: {other}");
            process::exit(2);
        }
    }

    println!("[COORDINATOR] SUCCESS scenario={}", scenario);
}
