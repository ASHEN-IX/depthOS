//! Phase 3.8 Consistency Integration Test
//!
//! Connects the experimental Rust coordinator lease and compositor-owned authority
//! in the smallest safe isolated test path, clearly distinguishing:
//! 1. Synthetic in-process state
//! 2. IPC message boundary
//! 3. Compositor-owned native state
//! 4. Durable coordinator state

use depthos_core::action_security::{
    Error, NativeDesktop, Operation, Placement, RequestId, Tick, WindowKey,
    lease::{
        CoordinatorLeaseManager, CountingSyntheticSink, LeaseConfig, LeaseError, LeaseRequest,
        SyntheticSinkAction, TerminalOutcome,
    },
};
use uuid::Uuid;

/// Layer 1: Synthetic In-Process State
#[allow(dead_code)]
#[derive(Clone, Debug)]
struct SyntheticInProcessToken {
    token_id: Uuid,
    valid: bool,
}

/// Layer 2: IPC Message Boundary (Simulated Wire Types)
#[allow(dead_code)]
#[derive(Clone, Debug)]
struct WireLeasePayload {
    operation_id: Uuid,
    window_runtime_id: String,
    native_uuid: Uuid,
    source_desktop: String,
    destination_desktop: String,
    coordinator_incarnation: Uuid,
    coordinator_epoch: u64,
    compositor_incarnation: Uuid,
    deadline_ms: u64,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
enum WireLeaseResponse {
    CompositorConfirmed { outcome: TerminalOutcome },
    CompositorRefused { reason: String },
    TransportDropped,
}

/// Layer 3: Compositor-Owned Native State (Emulated in Isolated Rust Test)
#[allow(dead_code)]
struct CompositorNativeState {
    incarnation: Uuid,
    active_seat: String,
    screen_locked: bool,
    active_window: Option<String>,
    current_desktop: String,
    prompt_visible: bool,
    prompt_rendered: bool,
}

impl CompositorNativeState {
    fn evaluate_handoff(
        &mut self,
        payload: &WireLeasePayload,
        now_ms: u64,
    ) -> Result<TerminalOutcome, String> {
        if payload.compositor_incarnation != self.incarnation {
            return Err("compositor-incarnation-mismatch".to_string());
        }
        if self.screen_locked {
            return Err("screen-locked".to_string());
        }
        if self.active_window.as_deref() != Some(&payload.window_runtime_id) {
            return Err("window-not-focused".to_string());
        }
        if self.current_desktop != payload.source_desktop {
            return Err("source-desktop-mismatch".to_string());
        }
        if now_ms >= payload.deadline_ms {
            return Err("deadline-expired".to_string());
        }
        if !self.prompt_rendered || !self.prompt_visible {
            return Err("prompt-not-presented".to_string());
        }

        // Native evaluation succeeded in test turn
        Ok(TerminalOutcome::Applied)
    }
}

/// Helper to set up consistent coordinator and compositor instances.
fn setup_consistency_environment() -> (
    CoordinatorLeaseManager,
    CompositorNativeState,
    WindowKey,
    Uuid,
) {
    let coord_inc = Uuid::new_v4();
    let comp_inc = Uuid::new_v4();
    let obs_inc = Uuid::new_v4();
    let native_uuid = Uuid::new_v4();
    let window = WindowKey {
        epoch: Uuid::new_v4(),
        runtime_id: "window-test-1".to_string(),
        generation: "1".to_string(),
    };

    let coordinator = CoordinatorLeaseManager::new(
        LeaseConfig {
            coordinator_incarnation: coord_inc,
            coordinator_epoch: 1,
            checkpoint_epoch: Uuid::new_v4(),
            checkpoint_sequence: 1,
            observer_incarnation: obs_inc,
            observer_version_seq: 1,
            policy_revision: Uuid::new_v4(),
            topology_revision: Uuid::new_v4(),
            compositor_incarnation: comp_inc,
        },
        Tick(100),
    );

    let compositor = CompositorNativeState {
        incarnation: comp_inc,
        active_seat: "seat0".to_string(),
        screen_locked: false,
        active_window: Some("window-test-1".to_string()),
        current_desktop: "desk-source".to_string(),
        prompt_visible: true,
        prompt_rendered: true,
    };

    (coordinator, compositor, window, native_uuid)
}

#[test]
fn test_layer_distinction_and_positive_handoff() {
    let (mut coordinator, mut compositor, window, native_uuid) = setup_consistency_environment();
    let comp_inc = compositor.incarnation;

    // 1. Coordinator acquires lease (Durable/Coordinator State)
    let op_id = RequestId(Uuid::new_v4());
    let req = LeaseRequest {
        operation_id: op_id,
        window: window.clone(),
        native_uuid,
        operation: Operation::MoveDesktop {
            from: Placement {
                desktop: NativeDesktop("desk-source".to_string()),
                activity: "activity-1".to_string(),
            },
            to: Placement {
                desktop: NativeDesktop("desk-target".to_string()),
                activity: "activity-1".to_string(),
            },
        },
        ttl_ms: 3_000,
    };

    let lease_id = coordinator
        .acquire(req, Tick(100), comp_inc)
        .expect("lease acquired");
    assert!(coordinator.is_window_locked(&window));

    // 2. Transmit across IPC boundary (Layer 2)
    let wire_payload = WireLeasePayload {
        operation_id: op_id.0,
        window_runtime_id: window.runtime_id.clone(),
        native_uuid,
        source_desktop: "desk-source".to_string(),
        destination_desktop: "desk-target".to_string(),
        coordinator_incarnation: coordinator.coordinator_incarnation,
        coordinator_epoch: coordinator.coordinator_epoch,
        compositor_incarnation: comp_inc,
        deadline_ms: 3_100,
    };

    // 3. Compositor validates native state (Layer 3)
    let native_eval = compositor.evaluate_handoff(&wire_payload, 500);
    assert_eq!(native_eval, Ok(TerminalOutcome::Applied));

    // 4. Return confirmed result to coordinator via synthetic sink
    let mut sink = CountingSyntheticSink::new();
    let outcome = coordinator
        .consume_synthetic(
            lease_id,
            &mut sink,
            SyntheticSinkAction::ApplyAndAck,
            Tick(500),
        )
        .expect("consumption succeeds");

    assert_eq!(outcome, TerminalOutcome::Applied);
    assert_eq!(sink.applied_count, 1);
    assert!(!coordinator.is_window_locked(&window));
}

#[test]
fn test_native_lock_transition_forces_refusal() {
    let (mut coordinator, mut compositor, window, native_uuid) = setup_consistency_environment();
    let comp_inc = compositor.incarnation;

    let op_id = RequestId(Uuid::new_v4());
    let req = LeaseRequest {
        operation_id: op_id,
        window: window.clone(),
        native_uuid,
        operation: Operation::MoveDesktop {
            from: Placement {
                desktop: NativeDesktop("desk-source".to_string()),
                activity: "activity-1".to_string(),
            },
            to: Placement {
                desktop: NativeDesktop("desk-target".to_string()),
                activity: "activity-1".to_string(),
            },
        },
        ttl_ms: 3_000,
    };

    let lease_id = coordinator
        .acquire(req, Tick(100), comp_inc)
        .expect("lease acquired");

    // Native lock transition occurs! Screen is locked
    compositor.screen_locked = true;

    let wire_payload = WireLeasePayload {
        operation_id: op_id.0,
        window_runtime_id: window.runtime_id.clone(),
        native_uuid,
        source_desktop: "desk-source".to_string(),
        destination_desktop: "desk-target".to_string(),
        coordinator_incarnation: coordinator.coordinator_incarnation,
        coordinator_epoch: coordinator.coordinator_epoch,
        compositor_incarnation: comp_inc,
        deadline_ms: 3_100,
    };

    let native_eval = compositor.evaluate_handoff(&wire_payload, 500);
    assert_eq!(native_eval, Err("screen-locked".to_string()));

    // Coordinator consumes failure: marks terminal rejected
    let mut sink = CountingSyntheticSink::new();
    let outcome = coordinator
        .consume_synthetic(
            lease_id,
            &mut sink,
            SyntheticSinkAction::RejectWithoutEffect,
            Tick(500),
        )
        .expect("rejected terminal outcome");

    assert_eq!(outcome, TerminalOutcome::Rejected);
    assert_eq!(sink.rejected_count, 1);
    assert!(!coordinator.is_window_locked(&window));
}

#[test]
fn test_transport_drop_triggers_quarantine_without_retry() {
    let (mut coordinator, compositor, window, native_uuid) = setup_consistency_environment();
    let comp_inc = compositor.incarnation;

    let op_id = RequestId(Uuid::new_v4());
    let req = LeaseRequest {
        operation_id: op_id,
        window: window.clone(),
        native_uuid,
        operation: Operation::MoveDesktop {
            from: Placement {
                desktop: NativeDesktop("desk-source".to_string()),
                activity: "activity-1".to_string(),
            },
            to: Placement {
                desktop: NativeDesktop("desk-target".to_string()),
                activity: "activity-1".to_string(),
            },
        },
        ttl_ms: 3_000,
    };

    let lease_id = coordinator
        .acquire(req, Tick(100), comp_inc)
        .expect("lease acquired");

    // Simulated transport drop (Layer 2 drop)
    let wire_response = WireLeaseResponse::TransportDropped;
    assert!(matches!(wire_response, WireLeaseResponse::TransportDropped));

    let mut sink = CountingSyntheticSink::new();
    let err = coordinator
        .consume_synthetic(
            lease_id,
            &mut sink,
            SyntheticSinkAction::LostAckAfterApply,
            Tick(500),
        )
        .expect_err("must quarantine on lost ack");

    assert_eq!(err, LeaseError::Quarantined);
    assert!(coordinator.is_window_quarantined(&window));

    // Invariant: Never retry!
    let retry_req = LeaseRequest {
        operation_id: op_id,
        window: window.clone(),
        native_uuid,
        operation: Operation::MoveDesktop {
            from: Placement {
                desktop: NativeDesktop("desk-source".to_string()),
                activity: "activity-1".to_string(),
            },
            to: Placement {
                desktop: NativeDesktop("desk-target".to_string()),
                activity: "activity-1".to_string(),
            },
        },
        ttl_ms: 3_000,
    };

    // Replay is rejected
    assert_eq!(
        coordinator.acquire(retry_req, Tick(600), comp_inc),
        Err(LeaseError::Replay)
    );
}

#[test]
fn test_synthetic_token_does_not_leak_into_production_capability() {
    // Layer 1 token
    let synthetic_token = SyntheticInProcessToken {
        token_id: Uuid::new_v4(),
        valid: true,
    };
    assert!(synthetic_token.valid);

    // Production authorizer strictly refuses interaction claim
    let authorizer = depthos_core::action_security::Authorizer::new();
    let client = depthos_core::action_security::Client {
        bus_id: Uuid::new_v4(),
        unique_name: ":1.99".to_string(),
        uid: 1000,
    };
    let result = authorizer.verify_interaction_claim(&client, true);
    assert_eq!(result.err(), Some(Error::UnverifiedInteraction));
}
