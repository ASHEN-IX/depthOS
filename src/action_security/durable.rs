//! Durable operation journal, formal state machine, crash recovery, and
//! lease integration for DepthOS action security.
//!
//! # Architecture & Guarantees
//! 1. **Durable Journal**: Backed by SQLite in WAL mode with `TransactionBehavior::Immediate`.
//!    Every operation is uniquely recorded by `operation_id` and cannot be reused across restarts.
//! 2. **Formal Operation State Machine**: Explicit transitions across 11 states:
//!    `Created`, `Reserved`, `Prepared`, `DispatchIntentRecorded`,
//!    `SyntheticDispatchAttempted`, `Acknowledged`, `Completed`, `Uncertain`,
//!    `Quarantined`, `Reconciled`, `Aborted`.
//! 3. **Linearization Points (LP 1-8)**:
//!    - LP-1: Reservation Acquisition (Atomically locks window against concurrent operations).
//!    - LP-2: Lease Fencing (Validates incarnation, epoch, and monotonic deadline).
//!    - LP-3: Dispatch-Intent Persistence (Commits `dispatch_intent = 1` before dispatch).
//!    - LP-4: Synthetic Dispatch Attempt (Synthetic delivery to compositor actor).
//!    - LP-5: Acknowledgment Recording (Authoritative compositor ack recorded; transitions to Completed).
//!    - LP-6: Uncertainty Quarantine (On timeout or loss after intent, locks window in Quarantine).
//!    - LP-7: Authoritative Reconciliation (Authoritative evidence transitions Quarantined to Reconciled).
//!    - LP-8: Final Release of Exclusion (Window lock released ONLY on terminal Completed, Aborted, Reconciled).
//! 4. **Crash Fencing & Startup Recovery**:
//!    - A restart increments the coordinator epoch and issues a fresh incarnation UUID.
//!    - Operations crashed *after* dispatch intent are recovered into `Quarantined`.
//!    - Operations crashed *before* dispatch intent are safely recovered into `Aborted`.
//! 5. **Deny-Only Default**:
//!    - Production capabilities remain completely unissued and deny-only.

use super::{
    Operation, Placement, RequestId, Tick, WindowKey,
    lease::{
        CoordinatorLeaseManager, LeaseConfig, LeaseError, LeaseId, LeaseRequest, LeaseReservation,
        LeaseState, SyntheticNativeSink, SyntheticSinkAction, TerminalOutcome,
    },
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

/// Formal operation state machine states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DurableOperationState {
    Created,
    Reserved,
    Prepared,
    DispatchIntentRecorded,
    SyntheticDispatchAttempted,
    Acknowledged,
    Completed,
    Uncertain,
    Quarantined,
    Reconciled,
    Aborted,
}

impl DurableOperationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::Reserved => "Reserved",
            Self::Prepared => "Prepared",
            Self::DispatchIntentRecorded => "DispatchIntentRecorded",
            Self::SyntheticDispatchAttempted => "SyntheticDispatchAttempted",
            Self::Acknowledged => "Acknowledged",
            Self::Completed => "Completed",
            Self::Uncertain => "Uncertain",
            Self::Quarantined => "Quarantined",
            Self::Reconciled => "Reconciled",
            Self::Aborted => "Aborted",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Aborted | Self::Reconciled)
    }

    pub fn is_quarantined_or_uncertain(&self) -> bool {
        matches!(self, Self::Uncertain | Self::Quarantined)
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Reserved
                | Self::Prepared
                | Self::DispatchIntentRecorded
                | Self::SyntheticDispatchAttempted
                | Self::Acknowledged
        )
    }

    pub fn can_transition_to(&self, next: DurableOperationState) -> bool {
        if self.is_terminal() {
            return false; // Absorbing property
        }
        match (*self, next) {
            (Self::Created, Self::Reserved) => true,
            (Self::Created, Self::Aborted) => true,
            (Self::Reserved, Self::Prepared) => true,
            (Self::Reserved, Self::Aborted) => true,
            (Self::Prepared, Self::DispatchIntentRecorded) => true,
            (Self::Prepared, Self::Aborted) => true,
            (Self::DispatchIntentRecorded, Self::SyntheticDispatchAttempted) => true,
            (Self::DispatchIntentRecorded, Self::Uncertain) => true,
            (Self::DispatchIntentRecorded, Self::Quarantined) => true, // direct crash recovery
            (Self::SyntheticDispatchAttempted, Self::Acknowledged) => true,
            (Self::SyntheticDispatchAttempted, Self::Uncertain) => true,
            (Self::SyntheticDispatchAttempted, Self::Quarantined) => true,
            (Self::Acknowledged, Self::Completed) => true,
            (Self::Uncertain, Self::Quarantined) => true,
            (Self::Quarantined, Self::Reconciled) => true,
            _ => false,
        }
    }
}

impl std::str::FromStr for DurableOperationState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Created" => Ok(Self::Created),
            "Reserved" => Ok(Self::Reserved),
            "Prepared" => Ok(Self::Prepared),
            "DispatchIntentRecorded" => Ok(Self::DispatchIntentRecorded),
            "SyntheticDispatchAttempted" => Ok(Self::SyntheticDispatchAttempted),
            "Acknowledged" => Ok(Self::Acknowledged),
            "Completed" => Ok(Self::Completed),
            "Uncertain" => Ok(Self::Uncertain),
            "Quarantined" => Ok(Self::Quarantined),
            "Reconciled" => Ok(Self::Reconciled),
            "Aborted" => Ok(Self::Aborted),
            _ => Err(()),
        }
    }
}

/// Controlled crash points for simulated failure injection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrashPoint {
    None,
    BeforeAcquisition,
    AfterReservationPersistence,
    BeforeIntentPersistence,
    AfterIntentPersistence,
    BeforeSyntheticConsumption,
    AfterSyntheticConsumption,
    BeforeAck,
    AfterAck,
    DuringQuarantine,
    DuringRecovery,
}

/// Errors returned by the durable coordinator journal and state machine.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DurableError {
    #[error("operation ID already exists; replay denied")]
    Replay,
    #[error("window is locked by an active lease")]
    WindowLocked,
    #[error("window is quarantined due to uncertain outcome")]
    WindowQuarantined,
    #[error("operation not found")]
    NotFound,
    #[error("illegal state machine transition from {0:?} to {1:?}")]
    InvalidTransition(DurableOperationState, DurableOperationState),
    #[error("terminal state {0:?} is absorbing; cannot mutate")]
    TerminalState(DurableOperationState),
    #[error("lease has expired")]
    Expired,
    #[error("monotonic clock reversed")]
    ClockReversed,
    #[error("coordinator incarnation or epoch mismatch")]
    CoordinatorMismatch,
    #[error("compositor incarnation mismatch")]
    CompositorMismatch,
    #[error("stale lease generation or incarnation")]
    StaleIncarnation,
    #[error("dispatch intent must be durably recorded prior to dispatch")]
    MissingDispatchIntent,
    #[error("reconciliation evidence invalid or rejected")]
    ReconciliationRejected,
    #[error("simulated crash triggered at {0:?}")]
    SimulatedCrash(CrashPoint),
    #[error("database error: {0}")]
    Database(String),
}

impl From<rusqlite::Error> for DurableError {
    fn from(err: rusqlite::Error) -> Self {
        DurableError::Database(err.to_string())
    }
}

impl From<LeaseError> for DurableError {
    fn from(err: LeaseError) -> Self {
        match err {
            LeaseError::ClockReversed => DurableError::ClockReversed,
            LeaseError::Expired => DurableError::Expired,
            LeaseError::Conflict => DurableError::WindowLocked,
            LeaseError::Quarantined => DurableError::WindowQuarantined,
            LeaseError::Replay => DurableError::Replay,
            LeaseError::InvalidPrecondition => {
                DurableError::Database("Invalid precondition".into())
            }
            LeaseError::CoordinatorMismatch => DurableError::CoordinatorMismatch,
            LeaseError::ObserverMismatch => DurableError::StaleIncarnation,
            LeaseError::TopologyMismatch => DurableError::StaleIncarnation,
            LeaseError::CompositorMismatch => DurableError::CompositorMismatch,
            LeaseError::NotFound => DurableError::NotFound,
            LeaseError::AlreadyTerminal => {
                DurableError::TerminalState(DurableOperationState::Completed)
            }
            LeaseError::CounterExhausted => DurableError::Database("Counter exhausted".into()),
        }
    }
}

/// Authoritative external evidence required to release a quarantined window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationEvidence {
    pub observer_incarnation: Uuid,
    pub observer_sequence: u64,
    pub verified_window_key: WindowKey,
    pub observed_native_uuid: Uuid,
    pub observed_placement: Placement,
    pub authoritative_decision: String,
}

/// Durable operation record stored in SQLite.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableOperationRecord {
    pub operation_id: RequestId,
    pub lease_id: LeaseId,
    pub window_key: WindowKey,
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
    pub durable_state: DurableOperationState,
    pub created_at_tick: Tick,
    pub expires_at_tick: Tick,
    pub dispatch_intent: bool,
    pub terminal_outcome: Option<TerminalOutcome>,
    pub uncertainty_reason: Option<String>,
    pub reconciliation_evidence: Option<String>,
    pub reconciled_at_tick: Option<Tick>,
    pub recovery_status: String,
}

/// Test-only Durable Operation Journal backed by SQLite.
pub struct DurableOperationJournal {
    conn: Connection,
    active_crash_point: CrashPoint,
}

impl DurableOperationJournal {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, DurableError> {
        let conn = Connection::open(path)?;
        Self::init_with_connection(conn)
    }

    pub fn open_in_memory() -> Result<Self, DurableError> {
        let conn = Connection::open_in_memory()?;
        Self::init_with_connection(conn)
    }

    fn init_with_connection(conn: Connection) -> Result<Self, DurableError> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;

        let sync_val: i64 = conn.query_row("PRAGMA synchronous;", [], |row| row.get(0))?;
        if sync_val != 2 {
            return Err(DurableError::Database(format!(
                "Failed to enforce PRAGMA synchronous = FULL (expected 2, got {sync_val})"
            )));
        }

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS coordinator_incarnations (
                incarnation_id TEXT PRIMARY KEY,
                epoch INTEGER NOT NULL UNIQUE,
                created_at_ms INTEGER NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS operation_journal (
                operation_id TEXT PRIMARY KEY,
                lease_id TEXT NOT NULL UNIQUE,
                window_key TEXT NOT NULL,
                native_uuid TEXT NOT NULL,
                operation_type TEXT NOT NULL,
                operation_json TEXT NOT NULL,
                coordinator_incarnation TEXT NOT NULL,
                coordinator_epoch INTEGER NOT NULL,
                checkpoint_epoch TEXT NOT NULL,
                checkpoint_sequence INTEGER NOT NULL,
                observer_incarnation TEXT NOT NULL,
                observer_version_seq INTEGER NOT NULL,
                policy_revision TEXT NOT NULL,
                topology_revision TEXT NOT NULL,
                compositor_incarnation TEXT NOT NULL,
                durable_state TEXT NOT NULL,
                created_at_tick INTEGER NOT NULL,
                expires_at_tick INTEGER NOT NULL,
                dispatch_intent INTEGER NOT NULL DEFAULT 0,
                terminal_outcome TEXT,
                uncertainty_reason TEXT,
                reconciliation_evidence TEXT,
                reconciled_at_tick INTEGER,
                recovery_status TEXT NOT NULL DEFAULT 'None',
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_journal_window ON operation_journal(window_key);
            CREATE INDEX IF NOT EXISTS idx_journal_native ON operation_journal(native_uuid);
            CREATE INDEX IF NOT EXISTS idx_journal_state ON operation_journal(durable_state);
            CREATE INDEX IF NOT EXISTS idx_journal_lease ON operation_journal(lease_id);",
        )?;

        Ok(Self {
            conn,
            active_crash_point: CrashPoint::None,
        })
    }

    /// Read the SQLite `PRAGMA synchronous` value for durability diagnostics.
    pub fn pragma_synchronous(&self) -> Result<i64, DurableError> {
        let val: i64 = self
            .conn
            .query_row("PRAGMA synchronous;", [], |row| row.get(0))?;
        Ok(val)
    }

    /// Read the SQLite `PRAGMA journal_mode` value for durability diagnostics.
    pub fn pragma_journal_mode(&self) -> Result<String, DurableError> {
        let mode: String = self
            .conn
            .query_row("PRAGMA journal_mode;", [], |row| row.get(0))?;
        Ok(mode)
    }

    pub fn set_crash_point(&mut self, cp: CrashPoint) {
        self.active_crash_point = cp;
    }

    pub fn crash_point(&self) -> CrashPoint {
        self.active_crash_point
    }

    fn check_crash(&self, target: CrashPoint) -> Result<(), DurableError> {
        if self.active_crash_point == target {
            Err(DurableError::SimulatedCrash(target))
        } else {
            Ok(())
        }
    }

    /// Register or resume coordinator incarnation. Monotonically allocates next epoch.
    pub fn establish_coordinator_incarnation(
        &mut self,
        incarnation_id: Uuid,
    ) -> Result<u64, DurableError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let current_max: Option<i64> = tx
            .query_row(
                "SELECT MAX(epoch) FROM coordinator_incarnations",
                [],
                |row| row.get(0),
            )
            .optional()?
            .flatten();

        let next_epoch = (current_max.unwrap_or(0) + 1) as u64;

        tx.execute(
            "UPDATE coordinator_incarnations SET is_active = 0 WHERE is_active = 1",
            [],
        )?;

        tx.execute(
            "INSERT INTO coordinator_incarnations (incarnation_id, epoch, created_at_ms, is_active)
             VALUES (?1, ?2, CAST(unixepoch('subsec') * 1000 AS INTEGER), 1)",
            params![incarnation_id.to_string(), next_epoch as i64],
        )?;

        tx.commit()?;
        Ok(next_epoch)
    }

    /// Retrieve the current active coordinator incarnation and epoch.
    pub fn get_active_incarnation(&self) -> Result<Option<(Uuid, u64)>, DurableError> {
        let res = self
            .conn
            .query_row(
                "SELECT incarnation_id, epoch FROM coordinator_incarnations WHERE is_active = 1 LIMIT 1",
                [],
                |row| {
                    let id_str: String = row.get(0)?;
                    let epoch_i: i64 = row.get(1)?;
                    Ok((id_str, epoch_i as u64))
                },
            )
            .optional()?;

        match res {
            Some((id_str, epoch)) => {
                let id = Uuid::parse_str(&id_str).map_err(|e| {
                    DurableError::Database(format!("corrupted incarnation UUID: {e}"))
                })?;
                Ok(Some((id, epoch)))
            }
            None => Ok(None),
        }
    }

    /// Check whether a window is available for coordinator operations.
    /// Fails closed if the window is locked by an active lease or quarantined.
    pub fn check_window_available(&self, window: &WindowKey) -> Result<(), DurableError> {
        let window_json = serde_json::to_string(window)
            .map_err(|e| DurableError::Database(format!("JSON serialization error: {e}")))?;

        let mut stmt = self.conn.prepare(
            "SELECT operation_id, durable_state FROM operation_journal
             WHERE window_key = ?1 AND durable_state NOT IN ('Completed', 'Aborted', 'Reconciled')
             LIMIT 1",
        )?;

        let res = stmt
            .query_row(params![window_json], |row| {
                let op_id: String = row.get(0)?;
                let state_str: String = row.get(1)?;
                Ok((op_id, state_str))
            })
            .optional()?;

        if let Some((_, state_str)) = res {
            let state = DurableOperationState::parse_str(&state_str)
                .unwrap_or(DurableOperationState::Quarantined);
            if state.is_quarantined_or_uncertain() {
                return Err(DurableError::WindowQuarantined);
            } else {
                return Err(DurableError::WindowLocked);
            }
        }

        Ok(())
    }

    /// Check whether a resource (by window key and native UUID) is available for coordinator operations.
    /// Fails closed if the resource is locked by an active lease or quarantined.
    pub fn check_resource_available(
        &self,
        window: &WindowKey,
        native_uuid: Uuid,
    ) -> Result<(), DurableError> {
        let window_json = serde_json::to_string(window)
            .map_err(|e| DurableError::Database(format!("JSON serialization error: {e}")))?;

        let mut stmt = self.conn.prepare(
            "SELECT operation_id, durable_state FROM operation_journal
             WHERE (window_key = ?1 OR native_uuid = ?2) AND durable_state NOT IN ('Completed', 'Aborted', 'Reconciled')
             LIMIT 1",
        )?;

        let res = stmt
            .query_row(params![window_json, native_uuid.to_string()], |row| {
                let op_id: String = row.get(0)?;
                let state_str: String = row.get(1)?;
                Ok((op_id, state_str))
            })
            .optional()?;

        if let Some((_, state_str)) = res {
            let state = DurableOperationState::parse_str(&state_str)
                .unwrap_or(DurableOperationState::Quarantined);
            if state.is_quarantined_or_uncertain() {
                return Err(DurableError::WindowQuarantined);
            } else {
                return Err(DurableError::WindowLocked);
            }
        }

        Ok(())
    }

    /// Linearization Point 1: Reservation Acquisition.
    /// Inserts a durable reservation in SQLite, verifying replay immunity and window exclusion.
    pub fn acquire_reservation(
        &mut self,
        res: &LeaseReservation,
        now: Tick,
    ) -> Result<(), DurableError> {
        self.check_crash(CrashPoint::BeforeAcquisition)?;

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        // Replay Check: operation_id must be globally unique across restarts
        let existing: Option<String> = tx
            .query_row(
                "SELECT durable_state FROM operation_journal WHERE operation_id = ?1",
                params![res.operation_id.0.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        if existing.is_some() {
            return Err(DurableError::Replay);
        }

        // Window Exclusion Check
        let window_json = serde_json::to_string(&res.window)
            .map_err(|e| DurableError::Database(format!("JSON serialization error: {e}")))?;

        let active_state: Option<String> = tx
            .query_row(
                "SELECT durable_state FROM operation_journal
                 WHERE (window_key = ?1 OR native_uuid = ?2) AND durable_state NOT IN ('Completed', 'Aborted', 'Reconciled')
                 LIMIT 1",
                params![window_json, res.native_uuid.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(state_str) = active_state {
            let state = DurableOperationState::parse_str(&state_str)
                .unwrap_or(DurableOperationState::Quarantined);
            if state.is_quarantined_or_uncertain() {
                return Err(DurableError::WindowQuarantined);
            } else {
                return Err(DurableError::WindowLocked);
            }
        }

        let op_type = match &res.operation {
            Operation::Associate { .. } => "Associate",
            Operation::MoveDesktop { .. } => "MoveDesktop",
        };
        let op_json = serde_json::to_string(&res.operation)
            .map_err(|e| DurableError::Database(format!("operation serialization error: {e}")))?;

        let initial_state = DurableOperationState::Reserved;

        tx.execute(
            "INSERT INTO operation_journal (
                operation_id, lease_id, window_key, native_uuid, operation_type, operation_json,
                coordinator_incarnation, coordinator_epoch, checkpoint_epoch, checkpoint_sequence,
                observer_incarnation, observer_version_seq, policy_revision, topology_revision,
                compositor_incarnation, durable_state, created_at_tick, expires_at_tick,
                dispatch_intent, recovery_status, created_at_ms, updated_at_ms
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14,
                ?15, ?16, ?17, ?18,
                0, 'None', CAST(unixepoch('subsec') * 1000 AS INTEGER), CAST(unixepoch('subsec') * 1000 AS INTEGER)
            )",
            params![
                res.operation_id.0.to_string(),
                res.id.0.to_string(),
                window_json,
                res.native_uuid.to_string(),
                op_type,
                op_json,
                res.coordinator_incarnation.to_string(),
                res.coordinator_epoch as i64,
                res.checkpoint_epoch.to_string(),
                res.checkpoint_sequence as i64,
                res.observer_incarnation.to_string(),
                res.observer_version_seq as i64,
                res.policy_revision.to_string(),
                res.topology_revision.to_string(),
                res.compositor_incarnation.to_string(),
                initial_state.as_str(),
                now.0 as i64,
                res.expires_at.0 as i64,
            ],
        )?;

        tx.commit()?;

        self.check_crash(CrashPoint::AfterReservationPersistence)?;
        Ok(())
    }

    /// Linearization Point 2: Lease Fencing & Preparation.
    pub fn prepare_operation(
        &mut self,
        operation_id: RequestId,
        now: Tick,
        active_coordinator: Uuid,
        active_epoch: u64,
        active_compositor: Uuid,
    ) -> Result<(), DurableError> {
        let rec = self.get_record(operation_id)?;

        if rec.durable_state.is_terminal() {
            return Err(DurableError::TerminalState(rec.durable_state));
        }
        if !rec
            .durable_state
            .can_transition_to(DurableOperationState::Prepared)
        {
            return Err(DurableError::InvalidTransition(
                rec.durable_state,
                DurableOperationState::Prepared,
            ));
        }

        if now >= rec.expires_at_tick {
            self.abort_operation(operation_id, "ExpiredBeforePreparation")?;
            return Err(DurableError::Expired);
        }

        if rec.coordinator_incarnation != active_coordinator
            || rec.coordinator_epoch != active_epoch
        {
            return Err(DurableError::CoordinatorMismatch);
        }
        if rec.compositor_incarnation != active_compositor {
            return Err(DurableError::CompositorMismatch);
        }

        self.update_state(operation_id, DurableOperationState::Prepared, None, None)?;
        Ok(())
    }

    /// Linearization Point 3: Dispatch-Intent Persistence.
    /// Formally records `dispatch_intent = 1` in SQLite prior to synthetic dispatch.
    pub fn persist_dispatch_intent(
        &mut self,
        operation_id: RequestId,
        now: Tick,
    ) -> Result<(), DurableError> {
        self.check_crash(CrashPoint::BeforeIntentPersistence)?;

        let rec = self.get_record(operation_id)?;
        if rec.durable_state.is_terminal() {
            return Err(DurableError::TerminalState(rec.durable_state));
        }
        if now >= rec.expires_at_tick {
            self.abort_operation(operation_id, "ExpiredBeforeIntent")?;
            return Err(DurableError::Expired);
        }

        if !rec
            .durable_state
            .can_transition_to(DurableOperationState::DispatchIntentRecorded)
        {
            return Err(DurableError::InvalidTransition(
                rec.durable_state,
                DurableOperationState::DispatchIntentRecorded,
            ));
        }

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        tx.execute(
            "UPDATE operation_journal
             SET durable_state = ?1, dispatch_intent = 1, updated_at_ms = CAST(unixepoch('subsec') * 1000 AS INTEGER)
             WHERE operation_id = ?2",
            params![
                DurableOperationState::DispatchIntentRecorded.as_str(),
                operation_id.0.to_string(),
            ],
        )?;

        tx.commit()?;

        self.check_crash(CrashPoint::AfterIntentPersistence)?;
        Ok(())
    }

    /// Linearization Point 4: Synthetic Dispatch Attempt.
    /// Transitions state to `SyntheticDispatchAttempted` verifying that dispatch intent was persisted.
    pub fn mark_synthetic_dispatch_attempted(
        &mut self,
        operation_id: RequestId,
    ) -> Result<(), DurableError> {
        self.check_crash(CrashPoint::BeforeSyntheticConsumption)?;

        let rec = self.get_record(operation_id)?;
        if !rec.dispatch_intent {
            return Err(DurableError::MissingDispatchIntent);
        }
        if !rec
            .durable_state
            .can_transition_to(DurableOperationState::SyntheticDispatchAttempted)
        {
            return Err(DurableError::InvalidTransition(
                rec.durable_state,
                DurableOperationState::SyntheticDispatchAttempted,
            ));
        }

        self.update_state(
            operation_id,
            DurableOperationState::SyntheticDispatchAttempted,
            None,
            None,
        )?;

        self.check_crash(CrashPoint::AfterSyntheticConsumption)?;
        Ok(())
    }

    /// Linearization Point 5: Acknowledgment Recording and Completion.
    pub fn record_acknowledgment(
        &mut self,
        operation_id: RequestId,
        outcome: TerminalOutcome,
        compositor_incarnation: Uuid,
    ) -> Result<(), DurableError> {
        self.check_crash(CrashPoint::BeforeAck)?;

        let rec = self.get_record(operation_id)?;
        if rec.durable_state.is_terminal() {
            return Err(DurableError::TerminalState(rec.durable_state));
        }
        if rec.durable_state.is_quarantined_or_uncertain() {
            return Err(DurableError::WindowQuarantined);
        }
        if rec.compositor_incarnation != compositor_incarnation {
            return Err(DurableError::CompositorMismatch);
        }

        if !rec
            .durable_state
            .can_transition_to(DurableOperationState::Acknowledged)
        {
            return Err(DurableError::InvalidTransition(
                rec.durable_state,
                DurableOperationState::Acknowledged,
            ));
        }

        let outcome_str = match outcome {
            TerminalOutcome::Applied => "Applied",
            TerminalOutcome::Rejected => "Rejected",
        };

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        // First transition to Acknowledged
        tx.execute(
            "UPDATE operation_journal
             SET durable_state = ?1, terminal_outcome = ?2, updated_at_ms = CAST(unixepoch('subsec') * 1000 AS INTEGER)
             WHERE operation_id = ?3",
            params![
                DurableOperationState::Acknowledged.as_str(),
                outcome_str,
                operation_id.0.to_string(),
            ],
        )?;

        // Then atomically to terminal Completed
        tx.execute(
            "UPDATE operation_journal
             SET durable_state = ?1, updated_at_ms = CAST(unixepoch('subsec') * 1000 AS INTEGER)
             WHERE operation_id = ?2",
            params![
                DurableOperationState::Completed.as_str(),
                operation_id.0.to_string(),
            ],
        )?;

        tx.commit()?;

        self.check_crash(CrashPoint::AfterAck)?;
        Ok(())
    }

    /// Linearization Point 6: Uncertainty Quarantine.
    /// Locks the window into Quarantine upon lost ack, timeout, or crash after intent.
    pub fn quarantine_operation(
        &mut self,
        operation_id: RequestId,
        reason: &str,
    ) -> Result<(), DurableError> {
        self.check_crash(CrashPoint::DuringQuarantine)?;

        let rec = self.get_record(operation_id)?;
        if rec.durable_state.is_terminal() {
            return Err(DurableError::TerminalState(rec.durable_state));
        }

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        tx.execute(
            "UPDATE operation_journal
             SET durable_state = ?1, uncertainty_reason = ?2, updated_at_ms = CAST(unixepoch('subsec') * 1000 AS INTEGER)
             WHERE operation_id = ?3",
            params![
                DurableOperationState::Quarantined.as_str(),
                reason,
                operation_id.0.to_string(),
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Linearization Point 7 & 8: Authoritative Reconciliation and Final Release of Exclusion.
    pub fn reconcile_quarantined_operation(
        &mut self,
        operation_id: RequestId,
        evidence: ReconciliationEvidence,
        now: Tick,
    ) -> Result<(), DurableError> {
        let rec = self.get_record(operation_id)?;
        if rec.durable_state != DurableOperationState::Quarantined {
            return Err(DurableError::InvalidTransition(
                rec.durable_state,
                DurableOperationState::Reconciled,
            ));
        }

        // Validate evidence correspondence
        if evidence.verified_window_key != rec.window_key
            || evidence.observed_native_uuid != rec.native_uuid
        {
            return Err(DurableError::ReconciliationRejected);
        }
        if evidence.observer_incarnation != rec.observer_incarnation {
            return Err(DurableError::ReconciliationRejected);
        }
        if evidence.observer_sequence <= rec.observer_version_seq {
            return Err(DurableError::ReconciliationRejected);
        }
        if evidence.authoritative_decision.trim().is_empty() {
            return Err(DurableError::ReconciliationRejected);
        }

        let evidence_json = serde_json::to_string(&evidence)
            .map_err(|e| DurableError::Database(format!("JSON serialization error: {e}")))?;

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        tx.execute(
            "UPDATE operation_journal
             SET durable_state = ?1, reconciliation_evidence = ?2, reconciled_at_tick = ?3,
                 recovery_status = 'AuthoritativelyReconciled',
                 updated_at_ms = CAST(unixepoch('subsec') * 1000 AS INTEGER)
             WHERE operation_id = ?4",
            params![
                DurableOperationState::Reconciled.as_str(),
                evidence_json,
                now.0 as i64,
                operation_id.0.to_string(),
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Abort an un-dispatched operation prior to dispatch intent.
    pub fn abort_operation(
        &mut self,
        operation_id: RequestId,
        reason: &str,
    ) -> Result<(), DurableError> {
        let rec = self.get_record(operation_id)?;
        if rec.durable_state.is_terminal() {
            return Err(DurableError::TerminalState(rec.durable_state));
        }
        if rec.dispatch_intent {
            // Cannot abort after dispatch intent has been durably recorded!
            return Err(DurableError::InvalidTransition(
                rec.durable_state,
                DurableOperationState::Aborted,
            ));
        }

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        tx.execute(
            "UPDATE operation_journal
             SET durable_state = ?1, uncertainty_reason = ?2, recovery_status = 'AbortedPreDispatch',
                 updated_at_ms = CAST(unixepoch('subsec') * 1000 AS INTEGER)
             WHERE operation_id = ?3",
            params![
                DurableOperationState::Aborted.as_str(),
                reason,
                operation_id.0.to_string(),
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Startup Crash Recovery Routine (Section 3.E).
    /// Recovers journal state following a crash or restart:
    /// - Operations with `dispatch_intent == 1` transition to `Quarantined`.
    /// - Operations with `dispatch_intent == 0` transition to `Aborted`.
    pub fn recover_on_startup(&mut self) -> Result<RecoveryReport, DurableError> {
        self.check_crash(CrashPoint::DuringRecovery)?;

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let mut report = RecoveryReport {
            quarantined_count: 0,
            aborted_count: 0,
        };

        let mut rows_to_recover = Vec::new();
        {
            let mut stmt = tx.prepare(
                "SELECT operation_id, dispatch_intent, durable_state FROM operation_journal
                 WHERE durable_state NOT IN ('Completed', 'Aborted', 'Reconciled', 'Quarantined')",
            )?;

            let rows = stmt.query_map([], |row| {
                let op_id: String = row.get(0)?;
                let intent: i64 = row.get(1)?;
                let state: String = row.get(2)?;
                Ok((op_id, intent == 1, state))
            })?;

            for r in rows {
                rows_to_recover.push(r?);
            }
        }

        for (op_id, intent, current_state) in rows_to_recover {
            if intent
                || current_state == "DispatchIntentRecorded"
                || current_state == "SyntheticDispatchAttempted"
                || current_state == "Acknowledged"
            {
                tx.execute(
                    "UPDATE operation_journal
                     SET durable_state = ?1, uncertainty_reason = 'CoordinatorCrashAfterDispatchIntent',
                         recovery_status = 'RecoveredToQuarantine',
                         updated_at_ms = CAST(unixepoch('subsec') * 1000 AS INTEGER)
                     WHERE operation_id = ?2",
                    params![DurableOperationState::Quarantined.as_str(), op_id],
                )?;
                report.quarantined_count += 1;
            } else {
                tx.execute(
                    "UPDATE operation_journal
                     SET durable_state = ?1, uncertainty_reason = 'CoordinatorCrashBeforeDispatchIntent',
                         recovery_status = 'AbortedPreIntentCrash',
                         updated_at_ms = CAST(unixepoch('subsec') * 1000 AS INTEGER)
                     WHERE operation_id = ?2",
                    params![DurableOperationState::Aborted.as_str(), op_id],
                )?;
                report.aborted_count += 1;
            }
        }

        tx.commit()?;
        Ok(report)
    }

    /// Retrieve full record for an operation.
    pub fn get_record(
        &self,
        operation_id: RequestId,
    ) -> Result<DurableOperationRecord, DurableError> {
        let mut stmt = self.conn.prepare(
            "SELECT operation_id, lease_id, window_key, native_uuid, operation_type, operation_json,
                    coordinator_incarnation, coordinator_epoch, checkpoint_epoch, checkpoint_sequence,
                    observer_incarnation, observer_version_seq, policy_revision, topology_revision,
                    compositor_incarnation, durable_state, created_at_tick, expires_at_tick,
                    dispatch_intent, terminal_outcome, uncertainty_reason, reconciliation_evidence,
                    reconciled_at_tick, recovery_status
             FROM operation_journal WHERE operation_id = ?1",
        )?;

        let rec = stmt
            .query_row(params![operation_id.0.to_string()], |row| {
                let op_id_str: String = row.get(0)?;
                let lease_id_str: String = row.get(1)?;
                let window_key_str: String = row.get(2)?;
                let native_uuid_str: String = row.get(3)?;
                let _op_type: String = row.get(4)?;
                let op_json: String = row.get(5)?;
                let coord_inc_str: String = row.get(6)?;
                let coord_epoch_i: i64 = row.get(7)?;
                let chk_epoch_str: String = row.get(8)?;
                let chk_seq_i: i64 = row.get(9)?;
                let obs_inc_str: String = row.get(10)?;
                let obs_seq_i: i64 = row.get(11)?;
                let pol_rev_str: String = row.get(12)?;
                let topo_rev_str: String = row.get(13)?;
                let comp_inc_str: String = row.get(14)?;
                let state_str: String = row.get(15)?;
                let created_tick_i: i64 = row.get(16)?;
                let expires_tick_i: i64 = row.get(17)?;
                let intent: i64 = row.get(18)?;
                let outcome_str: Option<String> = row.get(19)?;
                let uncertainty: Option<String> = row.get(20)?;
                let reconc_evidence: Option<String> = row.get(21)?;
                let reconc_tick_i: Option<i64> = row.get(22)?;
                let rec_status: String = row.get(23)?;

                let coord_epoch = coord_epoch_i as u64;
                let chk_seq = chk_seq_i as u64;
                let obs_seq = obs_seq_i as u64;
                let created_tick = created_tick_i as u64;
                let expires_tick = expires_tick_i as u64;
                let reconc_tick = reconc_tick_i.map(|t| t as u64);

                let op_id = Uuid::parse_str(&op_id_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                let lease_id = Uuid::parse_str(&lease_id_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                let window_key: WindowKey = serde_json::from_str(&window_key_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                let native_uuid = Uuid::parse_str(&native_uuid_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                let operation: Operation = serde_json::from_str(&op_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                let coord_inc = Uuid::parse_str(&coord_inc_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        6,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                let chk_epoch = Uuid::parse_str(&chk_epoch_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        8,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                let obs_inc = Uuid::parse_str(&obs_inc_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        10,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                let pol_rev = Uuid::parse_str(&pol_rev_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        12,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                let topo_rev = Uuid::parse_str(&topo_rev_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        13,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                let comp_inc = Uuid::parse_str(&comp_inc_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        14,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                let state = DurableOperationState::parse_str(&state_str).ok_or_else(|| {
                    rusqlite::Error::FromSqlConversionFailure(
                        15,
                        rusqlite::types::Type::Text,
                        format!("Unknown durable state: {state_str}").into(),
                    )
                })?;

                let terminal_outcome = outcome_str.and_then(|s| match s.as_str() {
                    "Applied" => Some(TerminalOutcome::Applied),
                    "Rejected" => Some(TerminalOutcome::Rejected),
                    _ => None,
                });

                Ok(DurableOperationRecord {
                    operation_id: RequestId(op_id),
                    lease_id: LeaseId(lease_id),
                    window_key,
                    native_uuid,
                    operation,
                    coordinator_incarnation: coord_inc,
                    coordinator_epoch: coord_epoch,
                    checkpoint_epoch: chk_epoch,
                    checkpoint_sequence: chk_seq,
                    observer_incarnation: obs_inc,
                    observer_version_seq: obs_seq,
                    policy_revision: pol_rev,
                    topology_revision: topo_rev,
                    compositor_incarnation: comp_inc,
                    durable_state: state,
                    created_at_tick: Tick(created_tick),
                    expires_at_tick: Tick(expires_tick),
                    dispatch_intent: intent == 1,
                    terminal_outcome,
                    uncertainty_reason: uncertainty,
                    reconciliation_evidence: reconc_evidence,
                    reconciled_at_tick: reconc_tick.map(Tick),
                    recovery_status: rec_status,
                })
            })
            .optional()?;

        rec.ok_or(DurableError::NotFound)
    }

    fn update_state(
        &mut self,
        operation_id: RequestId,
        next_state: DurableOperationState,
        outcome: Option<TerminalOutcome>,
        uncertainty: Option<&str>,
    ) -> Result<(), DurableError> {
        let outcome_str = outcome.map(|o| match o {
            TerminalOutcome::Applied => "Applied",
            TerminalOutcome::Rejected => "Rejected",
        });

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        tx.execute(
            "UPDATE operation_journal
             SET durable_state = ?1, terminal_outcome = COALESCE(?2, terminal_outcome),
                 uncertainty_reason = COALESCE(?3, uncertainty_reason),
                 updated_at_ms = CAST(unixepoch('subsec') * 1000 AS INTEGER)
             WHERE operation_id = ?4",
            params![
                next_state.as_str(),
                outcome_str,
                uncertainty,
                operation_id.0.to_string(),
            ],
        )?;

        tx.commit()?;
        Ok(())
    }
}

/// Recovery statistics produced by startup crash recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveryReport {
    pub quarantined_count: usize,
    pub aborted_count: usize,
}

/// Integrated Durable Coordinator Session combining the SQLite journal
/// with the in-memory `CoordinatorLeaseManager`.
pub struct DurableCoordinatorSession {
    journal: DurableOperationJournal,
    lease_manager: CoordinatorLeaseManager,
}

impl DurableCoordinatorSession {
    pub fn new(
        mut journal: DurableOperationJournal,
        config: LeaseConfig,
        start_time: Tick,
    ) -> Result<Self, DurableError> {
        // Run startup recovery first
        let _report = journal.recover_on_startup()?;

        // Establish coordinator incarnation monotonically
        let assigned_epoch =
            journal.establish_coordinator_incarnation(config.coordinator_incarnation)?;

        let mut lease_config = config;
        lease_config.coordinator_epoch = assigned_epoch;

        let lease_manager = CoordinatorLeaseManager::new(lease_config, start_time);

        Ok(Self {
            journal,
            lease_manager,
        })
    }

    pub fn journal(&self) -> &DurableOperationJournal {
        &self.journal
    }

    pub fn journal_mut(&mut self) -> &mut DurableOperationJournal {
        &mut self.journal
    }

    pub fn lease_manager(&self) -> &CoordinatorLeaseManager {
        &self.lease_manager
    }

    pub fn lease_manager_mut(&mut self) -> &mut CoordinatorLeaseManager {
        &mut self.lease_manager
    }

    pub fn current_time(&self) -> Tick {
        self.lease_manager.current_time()
    }

    pub fn advance_time(&mut self, now: Tick) -> Result<(), DurableError> {
        self.lease_manager.advance_time(now)?;
        Ok(())
    }

    /// Acquire durable lease reservation (LP-1).
    pub fn acquire(
        &mut self,
        req: LeaseRequest,
        now: Tick,
        expected_compositor: Uuid,
    ) -> Result<LeaseId, DurableError> {
        // 1. In-Memory check & acquisition
        let lease_id = self
            .lease_manager
            .acquire(req.clone(), now, expected_compositor)?;

        // 2. Fetch constructed reservation
        let state = self
            .lease_manager
            .get_reservation_state(&lease_id)
            .ok_or(DurableError::NotFound)?;

        let reservation = match state {
            LeaseState::Active(r) => r.clone(),
            _ => return Err(DurableError::NotFound),
        };

        // 3. Persist in SQLite journal
        if let Err(e) = self.journal.acquire_reservation(&reservation, now) {
            // Roll back in-memory reservation on journal failure
            let _ = self.lease_manager.revoke(lease_id, now);
            return Err(e);
        }

        Ok(lease_id)
    }

    /// Execute synthetic dispatch through the durable 8-point linearization pipeline.
    pub fn execute_synthetic<S: SyntheticNativeSink>(
        &mut self,
        lease_id: LeaseId,
        sink: &mut S,
        action: SyntheticSinkAction,
        now: Tick,
    ) -> Result<TerminalOutcome, DurableError> {
        self.advance_time(now)?;

        let state = self
            .lease_manager
            .get_reservation_state(&lease_id)
            .ok_or(DurableError::NotFound)?;

        let reservation = match state {
            LeaseState::Active(r) => (**r).clone(),
            _ => return Err(DurableError::NotFound),
        };

        let op_id = reservation.operation_id;

        // LP-2: Lease fencing & preparation
        self.journal.prepare_operation(
            op_id,
            now,
            self.lease_manager.coordinator_incarnation,
            self.lease_manager.coordinator_epoch,
            self.lease_manager.compositor_incarnation,
        )?;

        // LP-3: Dispatch-intent persistence
        self.journal.persist_dispatch_intent(op_id, now)?;

        // LP-4: Synthetic dispatch attempt
        self.journal.mark_synthetic_dispatch_attempted(op_id)?;

        // Consumption through synthetic native sink
        match self
            .lease_manager
            .consume_synthetic(lease_id, sink, action, now)
        {
            Ok(outcome) => {
                // LP-5: Authoritative acknowledgment recorded -> Completed
                self.journal.record_acknowledgment(
                    op_id,
                    outcome,
                    self.lease_manager.compositor_incarnation,
                )?;
                Ok(outcome)
            }
            Err(LeaseError::Quarantined) => {
                // LP-6: Quarantine on uncertainty / lost ack
                self.journal
                    .quarantine_operation(op_id, "SyntheticSinkUncertainty")?;
                Err(DurableError::WindowQuarantined)
            }
            Err(e) => {
                self.journal
                    .quarantine_operation(op_id, "SyntheticSinkError")?;
                Err(DurableError::from(e))
            }
        }
    }

    /// Authoritative reconciliation of a quarantined window (LP-7 & LP-8).
    pub fn reconcile_quarantine(
        &mut self,
        operation_id: RequestId,
        evidence: ReconciliationEvidence,
        now: Tick,
    ) -> Result<(), DurableError> {
        // 1. Authoritative check and SQLite transition
        self.journal
            .reconcile_quarantined_operation(operation_id, evidence.clone(), now)?;

        // 2. Clear in-memory quarantine
        self.lease_manager
            .reconcile_quarantined_window(&evidence.verified_window_key);

        Ok(())
    }

    /// Complete an operation and its lease with terminal outcome (Applied or Rejected).
    pub fn complete_lease(
        &mut self,
        lease_id: LeaseId,
        op_id: RequestId,
        outcome: TerminalOutcome,
        compositor_incarnation: Uuid,
        now: Tick,
    ) -> Result<(), DurableError> {
        self.journal
            .record_acknowledgment(op_id, outcome, compositor_incarnation)?;
        self.lease_manager.complete_lease(lease_id, outcome, now)?;
        Ok(())
    }

    /// Transition an operation and its lease to Quarantined on uncertainty or lost ack.
    pub fn quarantine_lease(
        &mut self,
        lease_id: LeaseId,
        op_id: RequestId,
        reason: &str,
        uncertainty: crate::action_security::Uncertainty,
        now: Tick,
    ) -> Result<(), DurableError> {
        self.journal.quarantine_operation(op_id, reason)?;
        self.lease_manager.quarantine(lease_id, uncertainty, now)?;
        Ok(())
    }

    /// Abort an operation and release its reservation before dispatch.
    pub fn abort_lease(
        &mut self,
        lease_id: LeaseId,
        op_id: RequestId,
        reason: &str,
        now: Tick,
    ) -> Result<(), DurableError> {
        self.journal.abort_operation(op_id, reason)?;
        self.lease_manager.revoke(lease_id, now)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_security::lease::CountingSyntheticSink;
    use crate::action_security::{NativeDesktop, Placement};
    use crate::domain::WorkspaceId;
    use std::collections::BTreeSet;

    fn sample_window(id: &str) -> WindowKey {
        WindowKey {
            epoch: Uuid::new_v4(),
            runtime_id: id.to_string(),
            generation: "gen-1".to_string(),
        }
    }

    fn sample_reservation(
        op_id: RequestId,
        window: WindowKey,
        coord_inc: Uuid,
        coord_epoch: u64,
        comp_inc: Uuid,
    ) -> LeaseReservation {
        LeaseReservation {
            id: LeaseId(Uuid::new_v4()),
            operation_id: op_id,
            window,
            native_uuid: Uuid::new_v4(),
            operation: Operation::Associate {
                from: BTreeSet::new(),
                to: [WorkspaceId(Uuid::new_v4())].into_iter().collect(),
            },
            coordinator_incarnation: coord_inc,
            coordinator_epoch: coord_epoch,
            checkpoint_epoch: Uuid::new_v4(),
            checkpoint_sequence: 1,
            observer_incarnation: Uuid::new_v4(),
            observer_version_seq: 1,
            policy_revision: Uuid::new_v4(),
            topology_revision: Uuid::new_v4(),
            compositor_incarnation: comp_inc,
            created_at: Tick(100),
            expires_at: Tick(1000),
        }
    }

    #[test]
    fn test_state_machine_transitions_and_absorbing_property() {
        use DurableOperationState::*;

        // Legal transitions
        assert!(Created.can_transition_to(Reserved));
        assert!(Created.can_transition_to(Aborted));
        assert!(Reserved.can_transition_to(Prepared));
        assert!(Reserved.can_transition_to(Aborted));
        assert!(Prepared.can_transition_to(DispatchIntentRecorded));
        assert!(Prepared.can_transition_to(Aborted));
        assert!(DispatchIntentRecorded.can_transition_to(SyntheticDispatchAttempted));
        assert!(DispatchIntentRecorded.can_transition_to(Uncertain));
        assert!(DispatchIntentRecorded.can_transition_to(Quarantined));
        assert!(SyntheticDispatchAttempted.can_transition_to(Acknowledged));
        assert!(SyntheticDispatchAttempted.can_transition_to(Uncertain));
        assert!(SyntheticDispatchAttempted.can_transition_to(Quarantined));
        assert!(Acknowledged.can_transition_to(Completed));
        assert!(Uncertain.can_transition_to(Quarantined));
        assert!(Quarantined.can_transition_to(Reconciled));

        // Illegal reverse or shortcut transitions
        assert!(!Completed.can_transition_to(Created));
        assert!(!Completed.can_transition_to(Reserved));
        assert!(!Aborted.can_transition_to(Prepared));
        assert!(!Reconciled.can_transition_to(DispatchIntentRecorded));
        assert!(!Quarantined.can_transition_to(Completed));
        assert!(!Quarantined.can_transition_to(DispatchIntentRecorded));
        assert!(!Uncertain.can_transition_to(SyntheticDispatchAttempted));
        assert!(!Created.can_transition_to(SyntheticDispatchAttempted));

        // Terminal absorbing property
        assert!(Completed.is_terminal());
        assert!(Aborted.is_terminal());
        assert!(Reconciled.is_terminal());
        assert!(!Reserved.is_terminal());
        assert!(!Quarantined.is_terminal());
    }

    #[test]
    fn test_operation_id_replay_prevention() {
        let mut journal = DurableOperationJournal::open_in_memory().unwrap();
        let coord_inc = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();
        let op_id = RequestId(Uuid::new_v4());
        let window = sample_window("win-1");

        let res = sample_reservation(op_id, window.clone(), coord_inc, 1, comp_inc);
        journal.acquire_reservation(&res, Tick(100)).unwrap();

        // Second acquisition with exact same operation_id must fail closed with Replay
        let res2 = sample_reservation(op_id, sample_window("win-2"), coord_inc, 1, comp_inc);
        let err = journal.acquire_reservation(&res2, Tick(100)).unwrap_err();
        assert_eq!(err, DurableError::Replay);
    }

    #[test]
    fn test_window_exclusion_locking() {
        let mut journal = DurableOperationJournal::open_in_memory().unwrap();
        let coord_inc = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();
        let window = sample_window("win-1");

        let op1 = RequestId(Uuid::new_v4());
        let res1 = sample_reservation(op1, window.clone(), coord_inc, 1, comp_inc);
        journal.acquire_reservation(&res1, Tick(100)).unwrap();

        // Check availability
        assert_eq!(
            journal.check_window_available(&window).unwrap_err(),
            DurableError::WindowLocked
        );

        // Concurrent operation on same window fails
        let op2 = RequestId(Uuid::new_v4());
        let res2 = sample_reservation(op2, window.clone(), coord_inc, 1, comp_inc);
        let err = journal.acquire_reservation(&res2, Tick(100)).unwrap_err();
        assert_eq!(err, DurableError::WindowLocked);

        // Aborting op1 releases window
        journal.abort_operation(op1, "UserCancelled").unwrap();
        assert!(journal.check_window_available(&window).is_ok());

        // Now op2 can acquire
        let op3 = RequestId(Uuid::new_v4());
        let res3 = sample_reservation(op3, window.clone(), coord_inc, 1, comp_inc);
        assert!(journal.acquire_reservation(&res3, Tick(100)).is_ok());
    }

    #[test]
    fn test_linearization_pipeline_full_success() {
        let mut journal = DurableOperationJournal::open_in_memory().unwrap();
        let coord_inc = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();
        let window = sample_window("win-1");
        let op_id = RequestId(Uuid::new_v4());

        let res = sample_reservation(op_id, window.clone(), coord_inc, 1, comp_inc);

        // LP-1: Acquire
        journal.acquire_reservation(&res, Tick(100)).unwrap();
        let rec = journal.get_record(op_id).unwrap();
        assert_eq!(rec.durable_state, DurableOperationState::Reserved);

        // LP-2: Prepare
        journal
            .prepare_operation(op_id, Tick(120), coord_inc, 1, comp_inc)
            .unwrap();
        let rec = journal.get_record(op_id).unwrap();
        assert_eq!(rec.durable_state, DurableOperationState::Prepared);

        // LP-3: Persist Dispatch Intent
        journal.persist_dispatch_intent(op_id, Tick(150)).unwrap();
        let rec = journal.get_record(op_id).unwrap();
        assert_eq!(
            rec.durable_state,
            DurableOperationState::DispatchIntentRecorded
        );
        assert!(rec.dispatch_intent);

        // LP-4: Synthetic Dispatch Attempt
        journal.mark_synthetic_dispatch_attempted(op_id).unwrap();
        let rec = journal.get_record(op_id).unwrap();
        assert_eq!(
            rec.durable_state,
            DurableOperationState::SyntheticDispatchAttempted
        );

        // LP-5: Acknowledgment Recording -> Completed
        journal
            .record_acknowledgment(op_id, TerminalOutcome::Applied, comp_inc)
            .unwrap();
        let rec = journal.get_record(op_id).unwrap();
        assert_eq!(rec.durable_state, DurableOperationState::Completed);
        assert_eq!(rec.terminal_outcome, Some(TerminalOutcome::Applied));

        // LP-8: Final Release of Exclusion - Window is now available
        assert!(journal.check_window_available(&window).is_ok());

        // Absorbing property: Attempting further changes fails
        let err = journal
            .prepare_operation(op_id, Tick(200), coord_inc, 1, comp_inc)
            .unwrap_err();
        assert_eq!(
            err,
            DurableError::TerminalState(DurableOperationState::Completed)
        );
    }

    #[test]
    fn test_intent_precedes_dispatch_enforcement() {
        let mut journal = DurableOperationJournal::open_in_memory().unwrap();
        let coord_inc = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();
        let window = sample_window("win-1");
        let op_id = RequestId(Uuid::new_v4());

        let res = sample_reservation(op_id, window, coord_inc, 1, comp_inc);
        journal.acquire_reservation(&res, Tick(100)).unwrap();
        journal
            .prepare_operation(op_id, Tick(120), coord_inc, 1, comp_inc)
            .unwrap();

        // Attempting dispatch without persist_dispatch_intent must fail
        let err = journal
            .mark_synthetic_dispatch_attempted(op_id)
            .unwrap_err();
        assert_eq!(err, DurableError::MissingDispatchIntent);
    }

    #[test]
    fn test_lost_ack_quarantines_and_denies_retry() {
        let mut journal = DurableOperationJournal::open_in_memory().unwrap();
        let coord_inc = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();
        let window = sample_window("win-1");
        let op_id = RequestId(Uuid::new_v4());

        let res = sample_reservation(op_id, window.clone(), coord_inc, 1, comp_inc);
        journal.acquire_reservation(&res, Tick(100)).unwrap();
        journal
            .prepare_operation(op_id, Tick(120), coord_inc, 1, comp_inc)
            .unwrap();
        journal.persist_dispatch_intent(op_id, Tick(150)).unwrap();
        journal.mark_synthetic_dispatch_attempted(op_id).unwrap();

        // Simulate lost ack timeout -> LP-6 Quarantine
        journal
            .quarantine_operation(op_id, "AckTimeoutTwoGenerals")
            .unwrap();

        let rec = journal.get_record(op_id).unwrap();
        assert_eq!(rec.durable_state, DurableOperationState::Quarantined);
        assert_eq!(
            rec.uncertainty_reason.as_deref(),
            Some("AckTimeoutTwoGenerals")
        );

        // Window remains locked by quarantine
        assert_eq!(
            journal.check_window_available(&window).unwrap_err(),
            DurableError::WindowQuarantined
        );

        // Attempting to re-dispatch or re-prepare uncertain operation must fail
        let err = journal
            .mark_synthetic_dispatch_attempted(op_id)
            .unwrap_err();
        assert!(matches!(err, DurableError::InvalidTransition(_, _)));
    }

    #[test]
    fn test_startup_recovery_pre_intent_aborts() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("test_pre_intent.sqlite3");

        let coord_inc = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();
        let window = sample_window("win-1");
        let op_id = RequestId(Uuid::new_v4());

        {
            let mut journal = DurableOperationJournal::open(&db_path).unwrap();
            let res = sample_reservation(op_id, window.clone(), coord_inc, 1, comp_inc);
            journal.acquire_reservation(&res, Tick(100)).unwrap();
            journal
                .prepare_operation(op_id, Tick(120), coord_inc, 1, comp_inc)
                .unwrap();
            // Process crashes BEFORE dispatch intent is persisted
        }

        // Restart process: open journal and run startup recovery
        {
            let mut journal = DurableOperationJournal::open(&db_path).unwrap();
            let report = journal.recover_on_startup().unwrap();
            assert_eq!(report.aborted_count, 1);
            assert_eq!(report.quarantined_count, 0);

            let rec = journal.get_record(op_id).unwrap();
            assert_eq!(rec.durable_state, DurableOperationState::Aborted);
            assert_eq!(rec.recovery_status, "AbortedPreIntentCrash");

            // Window is safely available again without spurious quarantine
            assert!(journal.check_window_available(&window).is_ok());
        }
    }

    #[test]
    fn test_startup_recovery_post_intent_quarantines() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("test_post_intent.sqlite3");

        let coord_inc = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();
        let window = sample_window("win-1");
        let op_id = RequestId(Uuid::new_v4());

        {
            let mut journal = DurableOperationJournal::open(&db_path).unwrap();
            let res = sample_reservation(op_id, window.clone(), coord_inc, 1, comp_inc);
            journal.acquire_reservation(&res, Tick(100)).unwrap();
            journal
                .prepare_operation(op_id, Tick(120), coord_inc, 1, comp_inc)
                .unwrap();
            journal.persist_dispatch_intent(op_id, Tick(150)).unwrap();
            // Process crashes AFTER dispatch intent is committed to disk
        }

        // Restart process
        {
            let mut journal = DurableOperationJournal::open(&db_path).unwrap();
            let report = journal.recover_on_startup().unwrap();
            assert_eq!(report.quarantined_count, 1);
            assert_eq!(report.aborted_count, 0);

            let rec = journal.get_record(op_id).unwrap();
            assert_eq!(rec.durable_state, DurableOperationState::Quarantined);
            assert_eq!(rec.recovery_status, "RecoveredToQuarantine");

            // Window remains locked by quarantine!
            assert_eq!(
                journal.check_window_available(&window).unwrap_err(),
                DurableError::WindowQuarantined
            );
        }
    }

    #[test]
    fn test_authoritative_reconciliation() {
        let mut journal = DurableOperationJournal::open_in_memory().unwrap();
        let coord_inc = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();
        let window = sample_window("win-1");
        let op_id = RequestId(Uuid::new_v4());

        let res = sample_reservation(op_id, window.clone(), coord_inc, 1, comp_inc);
        journal.acquire_reservation(&res, Tick(100)).unwrap();
        journal
            .prepare_operation(op_id, Tick(110), coord_inc, 1, comp_inc)
            .unwrap();
        journal.persist_dispatch_intent(op_id, Tick(120)).unwrap();
        journal
            .quarantine_operation(op_id, "SimulatedLoss")
            .unwrap();

        // Reconciliation with matching evidence succeeds
        let evidence = ReconciliationEvidence {
            observer_incarnation: res.observer_incarnation,
            observer_sequence: 42,
            verified_window_key: window.clone(),
            observed_native_uuid: res.native_uuid,
            observed_placement: Placement {
                desktop: NativeDesktop("desktop-2".to_string()),
                activity: "default".to_string(),
            },
            authoritative_decision: "ConfirmedAppliedByObserverSnapshot".to_string(),
        };

        journal
            .reconcile_quarantined_operation(op_id, evidence, Tick(500))
            .unwrap();
        let rec = journal.get_record(op_id).unwrap();
        assert_eq!(rec.durable_state, DurableOperationState::Reconciled);
        assert_eq!(rec.recovery_status, "AuthoritativelyReconciled");

        // Window is now released
        assert!(journal.check_window_available(&window).is_ok());
    }

    #[test]
    fn test_reconciliation_rejection_on_mismatched_evidence() {
        let mut journal = DurableOperationJournal::open_in_memory().unwrap();
        let coord_inc = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();
        let window = sample_window("win-1");
        let op_id = RequestId(Uuid::new_v4());

        let res = sample_reservation(op_id, window.clone(), coord_inc, 1, comp_inc);
        journal.acquire_reservation(&res, Tick(100)).unwrap();
        journal
            .prepare_operation(op_id, Tick(110), coord_inc, 1, comp_inc)
            .unwrap();
        journal.persist_dispatch_intent(op_id, Tick(120)).unwrap();
        journal
            .quarantine_operation(op_id, "SimulatedLoss")
            .unwrap();

        // Mismatched window key
        let bogus_evidence = ReconciliationEvidence {
            observer_incarnation: Uuid::new_v4(),
            observer_sequence: 42,
            verified_window_key: sample_window("wrong-window"),
            observed_native_uuid: res.native_uuid,
            observed_placement: Placement {
                desktop: NativeDesktop("desktop-2".to_string()),
                activity: "default".to_string(),
            },
            authoritative_decision: "FakeClaim".to_string(),
        };

        let err = journal
            .reconcile_quarantined_operation(op_id, bogus_evidence, Tick(500))
            .unwrap_err();
        assert_eq!(err, DurableError::ReconciliationRejected);

        // Window remains quarantined
        assert_eq!(
            journal.check_window_available(&window).unwrap_err(),
            DurableError::WindowQuarantined
        );
    }

    #[test]
    fn test_incarnation_fencing_stale_coordinator_and_compositor() {
        let mut journal = DurableOperationJournal::open_in_memory().unwrap();
        let coord_inc = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();
        let window = sample_window("win-1");
        let op_id = RequestId(Uuid::new_v4());

        let res = sample_reservation(op_id, window, coord_inc, 1, comp_inc);
        journal.acquire_reservation(&res, Tick(100)).unwrap();

        // Stale coordinator incarnation
        let stale_coord = Uuid::new_v4();
        let err = journal
            .prepare_operation(op_id, Tick(120), stale_coord, 1, comp_inc)
            .unwrap_err();
        assert_eq!(err, DurableError::CoordinatorMismatch);

        // Stale compositor incarnation
        let stale_comp = Uuid::new_v4();
        let err = journal
            .prepare_operation(op_id, Tick(120), coord_inc, 1, stale_comp)
            .unwrap_err();
        assert_eq!(err, DurableError::CompositorMismatch);
    }

    #[test]
    fn test_controlled_crash_points_simulation() {
        let mut journal = DurableOperationJournal::open_in_memory().unwrap();
        let coord_inc = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();
        let window = sample_window("win-1");
        let op_id = RequestId(Uuid::new_v4());

        let res = sample_reservation(op_id, window.clone(), coord_inc, 1, comp_inc);

        // Crash before acquisition
        journal.set_crash_point(CrashPoint::BeforeAcquisition);
        let err = journal.acquire_reservation(&res, Tick(100)).unwrap_err();
        assert_eq!(
            err,
            DurableError::SimulatedCrash(CrashPoint::BeforeAcquisition)
        );

        // Reset crash point and acquire
        journal.set_crash_point(CrashPoint::None);
        journal.acquire_reservation(&res, Tick(100)).unwrap();

        // Crash before intent persistence
        journal.set_crash_point(CrashPoint::BeforeIntentPersistence);
        let err = journal
            .persist_dispatch_intent(op_id, Tick(150))
            .unwrap_err();
        assert_eq!(
            err,
            DurableError::SimulatedCrash(CrashPoint::BeforeIntentPersistence)
        );

        // Record should remain in Reserved
        let rec = journal.get_record(op_id).unwrap();
        assert_eq!(rec.durable_state, DurableOperationState::Reserved);
    }

    #[test]
    fn test_durable_coordinator_session_end_to_end() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("session.sqlite3");
        let journal = DurableOperationJournal::open(&db_path).unwrap();

        let coord_inc = Uuid::new_v4();
        let comp_inc = Uuid::new_v4();
        let obs_inc = Uuid::new_v4();

        let config = LeaseConfig {
            coordinator_incarnation: coord_inc,
            coordinator_epoch: 1,
            checkpoint_epoch: Uuid::new_v4(),
            checkpoint_sequence: 1,
            observer_incarnation: obs_inc,
            observer_version_seq: 1,
            policy_revision: Uuid::new_v4(),
            topology_revision: Uuid::new_v4(),
            compositor_incarnation: comp_inc,
        };

        let mut session = DurableCoordinatorSession::new(journal, config, Tick(100)).unwrap();
        let window = sample_window("win-1");
        let op_id = RequestId(Uuid::new_v4());

        let req = LeaseRequest {
            operation_id: op_id,
            window: window.clone(),
            native_uuid: Uuid::new_v4(),
            operation: Operation::Associate {
                from: BTreeSet::new(),
                to: [WorkspaceId(Uuid::new_v4())].into_iter().collect(),
            },
            ttl_ms: 1000,
        };

        let lease_id = session.acquire(req, Tick(100), comp_inc).unwrap();

        let mut sink = CountingSyntheticSink::new();
        let outcome = session
            .execute_synthetic(
                lease_id,
                &mut sink,
                SyntheticSinkAction::ApplyAndAck,
                Tick(150),
            )
            .unwrap();

        assert_eq!(outcome, TerminalOutcome::Applied);
        assert_eq!(sink.applied_count, 1);

        // Verify journal has completed record
        let rec = session.journal().get_record(op_id).unwrap();
        assert_eq!(rec.durable_state, DurableOperationState::Completed);

        // Window is unlocked
        assert!(session.journal().check_window_available(&window).is_ok());
    }
}
