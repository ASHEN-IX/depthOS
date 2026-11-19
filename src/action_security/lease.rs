//! Isolated test-only coordinator lease protocol for DepthOS action security.
//!
//! This protocol establishes an immutable reservation in the coordinator that binds
//! an exact operation to the coordinator epoch/incarnation, observer stream,
//! policy/topology revisions, compositor incarnation, monotonic deadline, and window.
//!
//! # Linearization Points
//! 1. **Acquisition**: Atomically validates all precondition versions and places the
//!    reservation into `Active`, locking the window against any concurrent mutations.
//! 2. **Revocation**: Atomically transitions an `Active` reservation to `Revoked`,
//!    releasing the window lock before dispatch occurs.
//! 3. **Expiration**: When monotonic `Tick` advances past `expires_at`, atomically
//!    transitions an `Active` reservation to `Expired`, releasing the window lock.
//! 4. **Dispatch / Consumption**: The reservation is consumed by the synthetic sink.
//!    If dispatch is acknowledged, it transitions to `Terminal(Applied)`. If receipt
//!    is lost or completion is uncertain, the window transitions to `Quarantined`.
//! 5. **Quarantine / Uncertainty**: An uncertain dispatch outcome MUST quarantine the
//!    affected window to prevent conflicting coordinator mutations until an authoritative
//!    epoch reconciliation takes place. Timeout or lost ack is never interpreted as non-dispatch.
//!
//! No production capability issuer, writable D-Bus method, or persistent native setter exists.

use super::{Error, Operation, RequestId, Tick, Uncertainty, WindowKey};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LeaseId(pub Uuid);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseReservation {
    pub id: LeaseId,
    pub operation_id: RequestId,
    pub window: WindowKey,
    pub native_uuid: Uuid,
    pub operation: Operation,
    pub coordinator_incarnation: Uuid,
    pub coordinator_epoch: u64,
    pub checkpoint_epoch: Uuid,
    pub checkpoint_sequence: u64,
    pub observer_incarnation: Uuid,
    pub observer_version_seq: u64,
    pub policy_revision: Uuid,
    pub topology_revision: Uuid,
    pub compositor_incarnation: Uuid,
    pub created_at: Tick,
    pub expires_at: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalOutcome {
    Applied,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaseState {
    Active(Box<LeaseReservation>),
    Revoked {
        id: LeaseId,
        window: WindowKey,
        at: Tick,
    },
    Expired {
        id: LeaseId,
        window: WindowKey,
        at: Tick,
    },
    Terminal {
        id: LeaseId,
        window: WindowKey,
        outcome: TerminalOutcome,
        at: Tick,
    },
    Quarantined {
        id: LeaseId,
        window: WindowKey,
        uncertainty: Uncertainty,
        at: Tick,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaseError {
    ClockReversed,
    Expired,
    Conflict,
    Quarantined,
    Replay,
    InvalidPrecondition,
    CoordinatorMismatch,
    ObserverMismatch,
    TopologyMismatch,
    CompositorMismatch,
    NotFound,
    AlreadyTerminal,
    CounterExhausted,
}

impl From<LeaseError> for Error {
    fn from(err: LeaseError) -> Self {
        match err {
            LeaseError::ClockReversed => Error::ClockReversed,
            LeaseError::Expired => Error::Expired,
            LeaseError::Conflict => Error::Conflict,
            LeaseError::Quarantined => Error::Conflict,
            LeaseError::Replay => Error::Replay,
            LeaseError::InvalidPrecondition => Error::InvalidRequest,
            LeaseError::CoordinatorMismatch => Error::WrongAuthority,
            LeaseError::ObserverMismatch => Error::StaleVersion,
            LeaseError::TopologyMismatch => Error::StaleVersion,
            LeaseError::CompositorMismatch => Error::WrongAuthority,
            LeaseError::NotFound => Error::UnknownAction,
            LeaseError::AlreadyTerminal => Error::InvalidState,
            LeaseError::CounterExhausted => Error::CounterExhausted,
        }
    }
}

/// Request parameters to acquire a lease reservation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeaseRequest {
    pub operation_id: RequestId,
    pub window: WindowKey,
    pub native_uuid: Uuid,
    pub operation: Operation,
    pub ttl_ms: u64,
}

/// Private synthetic native sink for testing consumption outcomes.
pub enum SyntheticSinkAction {
    ApplyAndAck,
    RejectWithoutEffect,
    LostAckAfterApply,
    CrashBeforeConsumption,
    CrashAfterConsumption,
}

pub trait SyntheticNativeSink {
    fn consume(
        &mut self,
        reservation: &LeaseReservation,
        action: SyntheticSinkAction,
    ) -> Result<TerminalOutcome, Uncertainty>;
}

pub struct CountingSyntheticSink {
    pub applied_count: usize,
    pub rejected_count: usize,
}

impl Default for CountingSyntheticSink {
    fn default() -> Self {
        Self::new()
    }
}

impl CountingSyntheticSink {
    pub fn new() -> Self {
        Self {
            applied_count: 0,
            rejected_count: 0,
        }
    }
}

impl SyntheticNativeSink for CountingSyntheticSink {
    fn consume(
        &mut self,
        _reservation: &LeaseReservation,
        action: SyntheticSinkAction,
    ) -> Result<TerminalOutcome, Uncertainty> {
        match action {
            SyntheticSinkAction::ApplyAndAck => {
                self.applied_count += 1;
                Ok(TerminalOutcome::Applied)
            }
            SyntheticSinkAction::RejectWithoutEffect => {
                self.rejected_count += 1;
                Ok(TerminalOutcome::Rejected)
            }
            SyntheticSinkAction::LostAckAfterApply => {
                self.applied_count += 1;
                Err(Uncertainty::NoAcknowledgement)
            }
            SyntheticSinkAction::CrashBeforeConsumption => {
                // Crashed before dispatching or modifying native state.
                Err(Uncertainty::CancelledAfterDispatch)
            }
            SyntheticSinkAction::CrashAfterConsumption => {
                // Modified native state, then crashed before emitting acknowledgment.
                self.applied_count += 1;
                Err(Uncertainty::NoAcknowledgement)
            }
        }
    }
}

/// Configuration for initializing a coordinator lease manager.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeaseConfig {
    pub coordinator_incarnation: Uuid,
    pub coordinator_epoch: u64,
    pub checkpoint_epoch: Uuid,
    pub checkpoint_sequence: u64,
    pub observer_incarnation: Uuid,
    pub observer_version_seq: u64,
    pub policy_revision: Uuid,
    pub topology_revision: Uuid,
    pub compositor_incarnation: Uuid,
}

/// Coordinator Lease Manager holding authoritative in-memory reservation state.
pub struct CoordinatorLeaseManager {
    pub coordinator_incarnation: Uuid,
    pub coordinator_epoch: u64,
    pub checkpoint_epoch: Uuid,
    pub checkpoint_sequence: u64,
    pub observer_incarnation: Uuid,
    pub observer_version_seq: u64,
    pub policy_revision: Uuid,
    pub topology_revision: Uuid,
    pub compositor_incarnation: Uuid,

    active_reservations: BTreeMap<LeaseId, LeaseReservation>,
    locked_windows: BTreeMap<WindowKey, LeaseId>,
    quarantined_windows: BTreeMap<WindowKey, (LeaseId, Uncertainty, Tick)>,
    history: BTreeMap<LeaseId, LeaseState>,
    consumed_operations: BTreeSet<RequestId>,
    current_time: Tick,
}

impl CoordinatorLeaseManager {
    pub fn new(config: LeaseConfig, start_time: Tick) -> Self {
        Self {
            coordinator_incarnation: config.coordinator_incarnation,
            coordinator_epoch: config.coordinator_epoch,
            checkpoint_epoch: config.checkpoint_epoch,
            checkpoint_sequence: config.checkpoint_sequence,
            observer_incarnation: config.observer_incarnation,
            observer_version_seq: config.observer_version_seq,
            policy_revision: config.policy_revision,
            topology_revision: config.topology_revision,
            compositor_incarnation: config.compositor_incarnation,
            active_reservations: BTreeMap::new(),
            locked_windows: BTreeMap::new(),
            quarantined_windows: BTreeMap::new(),
            history: BTreeMap::new(),
            consumed_operations: BTreeSet::new(),
            current_time: start_time,
        }
    }

    pub fn current_time(&self) -> Tick {
        self.current_time
    }

    /// Advance coordinator monotonic clock and retire expired reservations.
    pub fn advance_time(&mut self, now: Tick) -> Result<(), LeaseError> {
        if now < self.current_time {
            return Err(LeaseError::ClockReversed);
        }
        self.current_time = now;

        // Linearization point: Expiration of active leases
        let mut expired = Vec::new();
        for (id, res) in &self.active_reservations {
            if now >= res.expires_at {
                expired.push((*id, res.window.clone()));
            }
        }

        for (id, window) in expired {
            self.active_reservations.remove(&id);
            self.locked_windows.remove(&window);
            self.history.insert(
                id,
                LeaseState::Expired {
                    id,
                    window,
                    at: now,
                },
            );
        }

        Ok(())
    }

    /// Check whether a window can undergo coordinator mutation.
    /// Fails if an active lease locks it or if it is currently quarantined.
    pub fn check_window_available(&self, window: &WindowKey) -> Result<(), LeaseError> {
        if self.locked_windows.contains_key(window) {
            return Err(LeaseError::Conflict);
        }
        if self.quarantined_windows.contains_key(window) {
            return Err(LeaseError::Quarantined);
        }
        Ok(())
    }

    /// Acquire an isolated reservation.
    ///
    /// Linearization Point: Preconditions are checked against the coordinator's
    /// exact epochs and checkpoint; the reservation is recorded and locks the
    /// window against conflicting mutations.
    pub fn acquire(
        &mut self,
        req: LeaseRequest,
        now: Tick,
        expected_compositor: Uuid,
    ) -> Result<LeaseId, LeaseError> {
        self.advance_time(now)?;

        if self.consumed_operations.contains(&req.operation_id) {
            return Err(LeaseError::Replay);
        }
        if expected_compositor != self.compositor_incarnation || expected_compositor.is_nil() {
            return Err(LeaseError::CompositorMismatch);
        }
        if self.coordinator_incarnation.is_nil() || self.observer_incarnation.is_nil() {
            return Err(LeaseError::InvalidPrecondition);
        }
        self.check_window_available(&req.window)?;

        let expires_at = Tick(
            now.0
                .checked_add(req.ttl_ms)
                .ok_or(LeaseError::CounterExhausted)?,
        );

        let id = LeaseId(Uuid::new_v4());
        let reservation = LeaseReservation {
            id,
            operation_id: req.operation_id,
            window: req.window.clone(),
            native_uuid: req.native_uuid,
            operation: req.operation,
            coordinator_incarnation: self.coordinator_incarnation,
            coordinator_epoch: self.coordinator_epoch,
            checkpoint_epoch: self.checkpoint_epoch,
            checkpoint_sequence: self.checkpoint_sequence,
            observer_incarnation: self.observer_incarnation,
            observer_version_seq: self.observer_version_seq,
            policy_revision: self.policy_revision,
            topology_revision: self.topology_revision,
            compositor_incarnation: self.compositor_incarnation,
            created_at: now,
            expires_at,
        };

        self.locked_windows.insert(req.window, id);
        self.active_reservations.insert(id, reservation.clone());
        self.history
            .insert(id, LeaseState::Active(Box::new(reservation)));

        Ok(id)
    }

    /// Revoke an active reservation before dispatch.
    ///
    /// Linearization Point: Transition from `Active` to `Revoked`, releasing window lock.
    pub fn revoke(&mut self, id: LeaseId, now: Tick) -> Result<(), LeaseError> {
        self.advance_time(now)?;

        let res = self
            .active_reservations
            .remove(&id)
            .ok_or(LeaseError::NotFound)?;
        self.locked_windows.remove(&res.window);

        self.history.insert(
            id,
            LeaseState::Revoked {
                id,
                window: res.window,
                at: now,
            },
        );

        Ok(())
    }

    /// Consume a reservation via synthetic native sink.
    ///
    /// Linearization Point: Dispatch to synthetic sink.
    /// On success: Terminal(Applied) or Terminal(Rejected).
    /// On failure/loss: Quarantined to prevent conflicting mutations.
    pub fn consume_synthetic<S: SyntheticNativeSink>(
        &mut self,
        id: LeaseId,
        sink: &mut S,
        action: SyntheticSinkAction,
        now: Tick,
    ) -> Result<TerminalOutcome, LeaseError> {
        self.advance_time(now)?;

        let reservation = self
            .active_reservations
            .remove(&id)
            .ok_or(LeaseError::NotFound)?;
        self.locked_windows.remove(&reservation.window);

        // Verify version alignment before dispatch
        if reservation.compositor_incarnation != self.compositor_incarnation {
            self.history.insert(
                id,
                LeaseState::Revoked {
                    id,
                    window: reservation.window,
                    at: now,
                },
            );
            return Err(LeaseError::CompositorMismatch);
        }
        if reservation.coordinator_incarnation != self.coordinator_incarnation
            || reservation.coordinator_epoch != self.coordinator_epoch
        {
            self.history.insert(
                id,
                LeaseState::Revoked {
                    id,
                    window: reservation.window,
                    at: now,
                },
            );
            return Err(LeaseError::CoordinatorMismatch);
        }

        // Linearization point: One-use dispatch
        self.consumed_operations.insert(reservation.operation_id);

        match sink.consume(&reservation, action) {
            Ok(outcome) => {
                self.history.insert(
                    id,
                    LeaseState::Terminal {
                        id,
                        window: reservation.window,
                        outcome,
                        at: now,
                    },
                );
                Ok(outcome)
            }
            Err(uncertainty) => {
                // Quarantine window to prevent conflicting state changes
                self.quarantined_windows
                    .insert(reservation.window.clone(), (id, uncertainty, now));
                self.history.insert(
                    id,
                    LeaseState::Quarantined {
                        id,
                        window: reservation.window,
                        uncertainty,
                        at: now,
                    },
                );
                // Return error reflecting uncertainty; timeout or lost ack is NOT non-dispatch.
                Err(LeaseError::Quarantined)
            }
        }
    }

    /// Simulate coordinator restart: invalidates active reservations, increments epoch.
    /// Quarantined items remain quarantined until reconciliation.
    pub fn restart_coordinator(&mut self, new_incarnation: Uuid, new_epoch: u64, now: Tick) {
        let _ = self.advance_time(now);
        self.coordinator_incarnation = new_incarnation;
        self.coordinator_epoch = new_epoch;

        // Invalidate all active reservations from old coordinator incarnation/epoch
        let active = std::mem::take(&mut self.active_reservations);
        for (id, res) in active {
            self.locked_windows.remove(&res.window);
            self.history.insert(
                id,
                LeaseState::Revoked {
                    id,
                    window: res.window,
                    at: now,
                },
            );
        }
    }

    /// Simulate compositor restart: active reservations bound to the previous compositor
    /// can no longer be dispatched.
    pub fn restart_compositor(&mut self, new_incarnation: Uuid, now: Tick) {
        let _ = self.advance_time(now);
        self.compositor_incarnation = new_incarnation;

        let active = std::mem::take(&mut self.active_reservations);
        for (id, res) in active {
            self.locked_windows.remove(&res.window);
            self.history.insert(
                id,
                LeaseState::Revoked {
                    id,
                    window: res.window,
                    at: now,
                },
            );
        }
    }

    /// Reconcile quarantine for a window using fresh authoritative evidence.
    pub fn reconcile_quarantined_window(&mut self, window: &WindowKey) -> bool {
        self.quarantined_windows.remove(window).is_some()
    }

    pub fn get_reservation_state(&self, id: &LeaseId) -> Option<&LeaseState> {
        self.history.get(id)
    }

    pub fn is_window_quarantined(&self, window: &WindowKey) -> bool {
        self.quarantined_windows.contains_key(window)
    }

    pub fn is_window_locked(&self, window: &WindowKey) -> bool {
        self.locked_windows.contains_key(window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_security::{NativeDesktop, Placement};

    fn create_test_manager() -> (CoordinatorLeaseManager, WindowKey, Uuid) {
        let coord_inc = Uuid::new_v4();
        let check_epoch = Uuid::new_v4();
        let obs_inc = Uuid::new_v4();
        let policy_rev = Uuid::new_v4();
        let topo_rev = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();
        let native_uuid = Uuid::new_v4();

        let window = WindowKey {
            epoch: Uuid::new_v4(),
            runtime_id: "window-1".to_string(),
            generation: "1".to_string(),
        };

        let mgr = CoordinatorLeaseManager::new(
            LeaseConfig {
                coordinator_incarnation: coord_inc,
                coordinator_epoch: 1,
                checkpoint_epoch: check_epoch,
                checkpoint_sequence: 100,
                observer_incarnation: obs_inc,
                observer_version_seq: 10,
                policy_revision: policy_rev,
                topology_revision: topo_rev,
                compositor_incarnation: comp_inc,
            },
            Tick(1_000),
        );

        (mgr, window, native_uuid)
    }

    fn sample_request(window: WindowKey, native_uuid: Uuid) -> LeaseRequest {
        LeaseRequest {
            operation_id: RequestId(Uuid::new_v4()),
            window,
            native_uuid,
            operation: Operation::MoveDesktop {
                from: Placement {
                    desktop: NativeDesktop("desk-1".to_string()),
                    activity: "act-1".to_string(),
                },
                to: Placement {
                    desktop: NativeDesktop("desk-2".to_string()),
                    activity: "act-1".to_string(),
                },
            },
            ttl_ms: 3_000,
        }
    }

    #[test]
    fn test_acquisition_and_linearization() {
        let (mut mgr, window, native_uuid) = create_test_manager();
        let req = sample_request(window.clone(), native_uuid);
        let comp = mgr.compositor_incarnation;

        let lease_id = mgr.acquire(req, Tick(1_000), comp).expect("acquire");
        assert!(mgr.is_window_locked(&window));

        // State is active
        match mgr.get_reservation_state(&lease_id) {
            Some(LeaseState::Active(r)) => {
                assert_eq!(r.id, lease_id);
                assert_eq!(r.expires_at, Tick(4_000));
            }
            other => panic!("expected active state, got {:?}", other),
        }

        // Cannot mutate locked window
        assert_eq!(
            mgr.check_window_available(&window),
            Err(LeaseError::Conflict)
        );
    }

    #[test]
    fn test_revocation_releases_window() {
        let (mut mgr, window, native_uuid) = create_test_manager();
        let req = sample_request(window.clone(), native_uuid);
        let comp = mgr.compositor_incarnation;

        let lease_id = mgr.acquire(req, Tick(1_000), comp).expect("acquire");
        assert!(mgr.is_window_locked(&window));

        mgr.revoke(lease_id, Tick(1_500)).expect("revoke");
        assert!(!mgr.is_window_locked(&window));
        assert_eq!(mgr.check_window_available(&window), Ok(()));

        match mgr.get_reservation_state(&lease_id) {
            Some(LeaseState::Revoked { at, .. }) => assert_eq!(*at, Tick(1_500)),
            other => panic!("expected revoked state, got {:?}", other),
        }

        // Revoking again fails
        assert_eq!(mgr.revoke(lease_id, Tick(1_600)), Err(LeaseError::NotFound));
    }

    #[test]
    fn test_expiration_releases_window() {
        let (mut mgr, window, native_uuid) = create_test_manager();
        let req = sample_request(window.clone(), native_uuid);
        let comp = mgr.compositor_incarnation;

        let lease_id = mgr.acquire(req, Tick(1_000), comp).expect("acquire");
        assert!(mgr.is_window_locked(&window));

        // Advance to just before expiration
        mgr.advance_time(Tick(3_999)).expect("advance");
        assert!(mgr.is_window_locked(&window));

        // Advance past expiration
        mgr.advance_time(Tick(4_000)).expect("advance");
        assert!(!mgr.is_window_locked(&window));
        assert_eq!(mgr.check_window_available(&window), Ok(()));

        match mgr.get_reservation_state(&lease_id) {
            Some(LeaseState::Expired { at, .. }) => assert_eq!(*at, Tick(4_000)),
            other => panic!("expected expired state, got {:?}", other),
        }
    }

    #[test]
    fn test_compositor_restart_invalidates_lease() {
        let (mut mgr, window, native_uuid) = create_test_manager();
        let req = sample_request(window.clone(), native_uuid);
        let comp = mgr.compositor_incarnation;

        let lease_id = mgr.acquire(req, Tick(1_000), comp).expect("acquire");
        assert!(mgr.is_window_locked(&window));

        let new_comp = Uuid::new_v4();
        mgr.restart_compositor(new_comp, Tick(1_200));

        // Active lease is removed and marked revoked
        assert!(!mgr.is_window_locked(&window));
        match mgr.get_reservation_state(&lease_id) {
            Some(LeaseState::Revoked { at, .. }) => assert_eq!(*at, Tick(1_200)),
            other => panic!("expected revoked state, got {:?}", other),
        }

        // Cannot consume with new compositor
        let mut sink = CountingSyntheticSink::new();
        assert_eq!(
            mgr.consume_synthetic(
                lease_id,
                &mut sink,
                SyntheticSinkAction::ApplyAndAck,
                Tick(1_300)
            ),
            Err(LeaseError::NotFound)
        );
    }

    #[test]
    fn test_coordinator_restart_invalidates_lease() {
        let (mut mgr, window, native_uuid) = create_test_manager();
        let req = sample_request(window.clone(), native_uuid);
        let comp = mgr.compositor_incarnation;

        let lease_id = mgr.acquire(req, Tick(1_000), comp).expect("acquire");
        assert!(mgr.is_window_locked(&window));

        let new_coord = Uuid::new_v4();
        mgr.restart_coordinator(new_coord, 2, Tick(1_200));

        assert!(!mgr.is_window_locked(&window));
        match mgr.get_reservation_state(&lease_id) {
            Some(LeaseState::Revoked { at, .. }) => assert_eq!(*at, Tick(1_200)),
            other => panic!("expected revoked state, got {:?}", other),
        }
    }

    #[test]
    fn test_lost_acknowledgment_triggers_quarantine() {
        let (mut mgr, window, native_uuid) = create_test_manager();
        let req = sample_request(window.clone(), native_uuid);
        let comp = mgr.compositor_incarnation;

        let lease_id = mgr.acquire(req, Tick(1_000), comp).expect("acquire");
        let mut sink = CountingSyntheticSink::new();

        // Dispatch with lost ack
        let err = mgr
            .consume_synthetic(
                lease_id,
                &mut sink,
                SyntheticSinkAction::LostAckAfterApply,
                Tick(1_500),
            )
            .expect_err("should fail with quarantine");

        assert_eq!(err, LeaseError::Quarantined);
        assert_eq!(sink.applied_count, 1);

        // Window is quarantined: cannot mutate or acquire new lease!
        assert!(mgr.is_window_quarantined(&window));
        assert_eq!(
            mgr.check_window_available(&window),
            Err(LeaseError::Quarantined)
        );

        // New lease acquisition on the same window is refused
        let req2 = sample_request(window.clone(), native_uuid);
        assert_eq!(
            mgr.acquire(req2, Tick(1_600), comp),
            Err(LeaseError::Quarantined)
        );

        // Authoritative reconciliation lifts quarantine
        assert!(mgr.reconcile_quarantined_window(&window));
        assert!(!mgr.is_window_quarantined(&window));
        assert_eq!(mgr.check_window_available(&window), Ok(()));
    }

    #[test]
    fn test_concurrent_conflicting_proposals_refused() {
        let (mut mgr, window, native_uuid) = create_test_manager();
        let comp = mgr.compositor_incarnation;

        let req1 = sample_request(window.clone(), native_uuid);
        let req2 = sample_request(window.clone(), native_uuid);

        let _lease1 = mgr
            .acquire(req1, Tick(1_000), comp)
            .expect("first acquires");
        let err = mgr
            .acquire(req2, Tick(1_050), comp)
            .expect_err("second must conflict");
        assert_eq!(err, LeaseError::Conflict);
    }

    #[test]
    fn test_crash_before_consumption() {
        let (mut mgr, window, native_uuid) = create_test_manager();
        let req = sample_request(window.clone(), native_uuid);
        let comp = mgr.compositor_incarnation;

        let lease_id = mgr.acquire(req, Tick(1_000), comp).expect("acquire");
        let mut sink = CountingSyntheticSink::new();

        let err = mgr
            .consume_synthetic(
                lease_id,
                &mut sink,
                SyntheticSinkAction::CrashBeforeConsumption,
                Tick(1_500),
            )
            .expect_err("crash before consumption");

        assert_eq!(err, LeaseError::Quarantined);
        assert_eq!(sink.applied_count, 0);
        assert!(mgr.is_window_quarantined(&window));
    }

    #[test]
    fn test_crash_after_synthetic_consumption() {
        let (mut mgr, window, native_uuid) = create_test_manager();
        let req = sample_request(window.clone(), native_uuid);
        let op_id = req.operation_id;
        let comp = mgr.compositor_incarnation;

        let lease_id = mgr.acquire(req, Tick(1_000), comp).expect("acquire");
        let mut sink = CountingSyntheticSink::new();

        let err = mgr
            .consume_synthetic(
                lease_id,
                &mut sink,
                SyntheticSinkAction::CrashAfterConsumption,
                Tick(1_500),
            )
            .expect_err("crash after consumption");

        assert_eq!(err, LeaseError::Quarantined);
        assert_eq!(sink.applied_count, 1);
        assert!(mgr.is_window_quarantined(&window));

        // Replaying same operation ID fails
        let replay_req = sample_request(window.clone(), native_uuid);
        let replay_req = LeaseRequest {
            operation_id: op_id, // reuse id
            ..replay_req
        };
        assert_eq!(
            mgr.acquire(replay_req, Tick(1_600), comp),
            Err(LeaseError::Replay)
        );
    }

    #[test]
    fn test_recovery_without_replay() {
        let (mut mgr, window, native_uuid) = create_test_manager();
        let req = sample_request(window.clone(), native_uuid);
        let op_id = req.operation_id;
        let comp = mgr.compositor_incarnation;

        let lease_id = mgr.acquire(req, Tick(1_000), comp).expect("acquire");
        let mut sink = CountingSyntheticSink::new();

        let outcome = mgr
            .consume_synthetic(
                lease_id,
                &mut sink,
                SyntheticSinkAction::ApplyAndAck,
                Tick(1_500),
            )
            .expect("applied");
        assert_eq!(outcome, TerminalOutcome::Applied);
        assert_eq!(sink.applied_count, 1);

        // Attempting to consume again fails
        assert_eq!(
            mgr.consume_synthetic(
                lease_id,
                &mut sink,
                SyntheticSinkAction::ApplyAndAck,
                Tick(1_600)
            ),
            Err(LeaseError::NotFound)
        );

        // Attempting to re-acquire with same operation ID fails with Replay
        let mut req2 = sample_request(window, native_uuid);
        req2.operation_id = op_id;
        assert_eq!(
            mgr.acquire(req2, Tick(1_700), comp),
            Err(LeaseError::Replay)
        );
    }
}
