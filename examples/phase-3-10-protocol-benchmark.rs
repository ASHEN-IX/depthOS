//! Phase 3.10 Cross-Process Protocol Latency Benchmark
//!
//! Measures end-to-end roundtrip latency for:
//! 1. Peer credential verification & bootstrap handshake
//! 2. Linearized dispatch operation (Reserve -> Prepared -> WAL Commit -> Attempt -> Ack -> Receipt)
//!
//! Computes p50, p90, p99, min, max, mean, and throughput over multiple iterations.

use depthos_core::action_security::{
    NativeDesktop, Operation, Placement, RequestId, Tick, WindowKey,
    durable::{DurableCoordinatorSession, DurableOperationJournal, DurableOperationState},
    ipc::{
        AckOutcome, AcknowledgeMessage, CrossProcessCoordinator, HelloAcceptedMessage, IpcStream,
        PreparedMessage, ProtocolMessage, SessionState,
    },
    lease::{LeaseConfig, LeaseRequest},
};
use std::time::Instant;
use tempfile::TempDir;
use tokio::net::UnixStream;
use uuid::Uuid;

fn sample_lease_config(coord_inc: Uuid, comp_inc: Uuid) -> LeaseConfig {
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

#[tokio::main]
async fn main() {
    println!("=== DepthOS Phase 3.10 Cross-Process Protocol Latency Benchmark ===");

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench_journal.sqlite3");
    let journal = DurableOperationJournal::open(&db_path).unwrap();

    let coord_inc = Uuid::new_v4();
    let comp_inc = Uuid::new_v4();
    let config = sample_lease_config(coord_inc, comp_inc);
    let session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();

    let mut coord = CrossProcessCoordinator::new(session, "bench-token-128".into()).unwrap();

    let (s1, s2) = UnixStream::pair().unwrap();
    let mut client_ipc = IpcStream::new(s1, SessionState::Unauthenticated);

    // Compositor Mock Server Task
    let comp_inc_clone = comp_inc;
    let srv_task = tokio::spawn(async move {
        let mut srv_ipc = IpcStream::new(s2, SessionState::Unauthenticated);

        // Handshake
        match srv_ipc.recv_message().await.unwrap() {
            ProtocolMessage::Hello(h) => {
                let accepted = HelloAcceptedMessage {
                    protocol_version: 0x0100,
                    session_id: h.session_id,
                    compositor_incarnation: comp_inc_clone,
                    accepted_epoch: 1,
                    server_pid: std::process::id(),
                };
                srv_ipc
                    .send_message(&ProtocolMessage::HelloAccepted(accepted))
                    .await
                    .unwrap();
                srv_ipc.set_session_state(SessionState::Active);
            }
            _ => panic!("Expected Hello"),
        }

        // Loop for benchmark dispatches
        loop {
            // Reserve
            let r = match srv_ipc.recv_message().await {
                Ok(ProtocolMessage::Reserve(r)) => r,
                Ok(_) => break,
                Err(_) => break,
            };
            let prep = PreparedMessage {
                protocol_version: 0x0100,
                session_id: r.session_id,
                operation_id: r.operation_id,
                lease_id: r.lease_id,
                compositor_incarnation: comp_inc_clone,
                window_generation: 1,
                ready: true,
                detail: None,
            };
            srv_ipc
                .send_message(&ProtocolMessage::Prepared(prep))
                .await
                .unwrap();

            // DispatchIntent
            match srv_ipc.recv_message().await {
                Ok(ProtocolMessage::DispatchIntent(_)) => {}
                _ => break,
            }

            // SyntheticAttempt
            let att = match srv_ipc.recv_message().await {
                Ok(ProtocolMessage::SyntheticAttempt(att)) => att,
                _ => break,
            };
            let ack = AcknowledgeMessage {
                protocol_version: 0x0100,
                session_id: att.session_id,
                operation_id: att.operation_id,
                lease_id: att.lease_id,
                attempt: att.attempt,
                coordinator_incarnation: att.coordinator_incarnation,
                compositor_incarnation: comp_inc_clone,
                outcome: AckOutcome::Applied,
                reason: None,
            };
            srv_ipc
                .send_message(&ProtocolMessage::Acknowledge(ack))
                .await
                .unwrap();

            // TerminalReceipt
            match srv_ipc.recv_message().await {
                Ok(ProtocolMessage::TerminalReceipt(_)) => {}
                _ => break,
            }
        }
    });

    // 1. Measure Handshake Latency
    let t_start = Instant::now();
    let accepted = coord
        .perform_handshake(&mut client_ipc, Some(comp_inc))
        .await
        .unwrap();
    let handshake_duration = t_start.elapsed();
    println!(
        "Handshake completed: server_pid={}, duration={:.3} ms ({:.1} µs)",
        accepted.server_pid,
        handshake_duration.as_secs_f64() * 1000.0,
        handshake_duration.as_micros()
    );

    // 2. Measure Dispatch Latency over 200 iterations
    let iterations = 200;
    let mut latencies_us: Vec<f64> = Vec::with_capacity(iterations);

    let base_tick = 1000;
    for i in 0..iterations {
        let op_id = RequestId(Uuid::new_v4());
        let native_uuid = Uuid::new_v4();
        let win = WindowKey {
            epoch: Uuid::new_v4(),
            runtime_id: format!("win-bench-{i}"),
            generation: format!("gen-{i}"),
        };
        let req = LeaseRequest {
            operation_id: op_id,
            window: win,
            native_uuid,
            operation: Operation::MoveDesktop {
                from: Placement {
                    desktop: NativeDesktop("src".into()),
                    activity: "act".into(),
                },
                to: Placement {
                    desktop: NativeDesktop("dst".into()),
                    activity: "act".into(),
                },
            },
            ttl_ms: 10000,
        };

        let start = Instant::now();
        let state = coord
            .dispatch_operation(
                &mut client_ipc,
                req,
                Operation::MoveDesktop {
                    from: Placement {
                        desktop: NativeDesktop("src".into()),
                        activity: "act".into(),
                    },
                    to: Placement {
                        desktop: NativeDesktop("dst".into()),
                        activity: "act".into(),
                    },
                },
                "fixture-token".into(),
                Tick(base_tick + (i as u64 * 10)),
            )
            .await
            .unwrap();

        let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0; // microseconds
        assert_eq!(state, DurableOperationState::Completed);
        latencies_us.push(elapsed);
    }

    latencies_us.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let min = latencies_us.first().copied().unwrap();
    let max = latencies_us.last().copied().unwrap();
    let sum: f64 = latencies_us.iter().sum();
    let mean = sum / (latencies_us.len() as f64);
    let p50 = latencies_us[(latencies_us.len() as f64 * 0.50) as usize];
    let p90 = latencies_us[(latencies_us.len() as f64 * 0.90) as usize];
    let p99 = latencies_us[(latencies_us.len() as f64 * 0.99) as usize];

    println!(
        "\n--- Dispatch Roundtrip Latency (Linearized LP 1-5 + WAL Commit + Framing + UDS) ---"
    );
    println!("Iterations: {}", iterations);
    println!("Min:        {:.2} µs ({:.4} ms)", min, min / 1000.0);
    println!("p50:        {:.2} µs ({:.4} ms)", p50, p50 / 1000.0);
    println!("p90:        {:.2} µs ({:.4} ms)", p90, p90 / 1000.0);
    println!("p99:        {:.2} µs ({:.4} ms)", p99, p99 / 1000.0);
    println!("Max:        {:.2} µs ({:.4} ms)", max, max / 1000.0);
    println!("Mean:       {:.2} µs ({:.4} ms)", mean, mean / 1000.0);
    println!("Throughput: {:.1} ops/sec", 1_000_000.0 / mean);

    drop(client_ipc);
    let _ = srv_task.await;
}
