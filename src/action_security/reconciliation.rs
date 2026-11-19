//! Authoritative reconciliation gateway and boundary for DepthOS action security.
//!
//! # Architecture & Guarantees (Section 3.G)
//! 1. **Isolated Evidence Acquisition**: Rejection of reconciliation requests from arbitrary IPC clients
//!    or public D-Bus interfaces. Reconciliation is accepted *strictly* from the trusted observer channel
//!    carrying provenance `depthos.observer.authoritative` and the unguessable launcher-issued authority token.
//! 2. **Anti-Spoofing & Strict Identity Binding**: Validates exact matching of:
//!    - Target `operation_id` currently in `Quarantined` state
//!    - Verified `window_key` and `native_uuid`
//!    - `observer_incarnation` matching the active observer instance
//! 3. **Strict Monotonic Observer Sequence Fencing**: Replays of pre-quarantine observer snapshots are defeated
//!    by strictly requiring `evidence.observer_sequence > record.observer_version_seq`.
//! 4. **Fail-Closed Permanent Quarantine**: If evidence is absent, unauthenticated, stale, or uncertain,
//!    quarantine is NEVER lifted automatically.

use super::{
    Tick,
    durable::{DurableCoordinatorSession, DurableError, DurableOperationState},
    ipc::{IpcError, ReconcileEvidenceMessage, TerminalReceiptMessage},
};
use uuid::Uuid;

pub const AUTHORITATIVE_PROVENANCE: &str = "depthos.observer.authoritative";

#[derive(thiserror::Error, Debug)]
pub enum ReconciliationError {
    #[error(
        "Unauthorized reconciliation provenance: expected '{AUTHORITATIVE_PROVENANCE}', got '{0}'"
    )]
    UnauthorizedProvenance(String),
    #[error("Invalid observer authority token")]
    UnauthorizedToken,
    #[error("Operation {0:?} is not quarantined")]
    NotQuarantined(uuid::Uuid),
    #[error("Window key mismatch: expected {expected:?}, got {got:?}")]
    WindowKeyMismatch { expected: String, got: String },
    #[error("Native UUID mismatch: expected {expected}, got {got}")]
    NativeUuidMismatch { expected: Uuid, got: Uuid },
    #[error("Observer incarnation mismatch: expected {expected}, got {got}")]
    ObserverIncarnationMismatch { expected: Uuid, got: Uuid },
    #[error(
        "Stale observer sequence number: recorded {recorded}, received {received} (must be strictly greater)"
    )]
    StaleObserverSequence { recorded: u64, received: u64 },
    #[error("Empty or invalid authoritative decision in evidence")]
    EmptyAuthoritativeDecision,
    #[error("Durable storage error: {0}")]
    Durable(#[from] DurableError),
    #[error("IPC protocol error: {0}")]
    Ipc(#[from] IpcError),
}

/// Authoritative Reconciliation Gateway.
pub struct AuthoritativeReconciliationGateway {
    pub trusted_observer_token: String,
    pub active_observer_incarnation: Uuid,
}

impl AuthoritativeReconciliationGateway {
    pub fn new(trusted_observer_token: String, active_observer_incarnation: Uuid) -> Self {
        Self {
            trusted_observer_token,
            active_observer_incarnation,
        }
    }

    /// Process incoming reconciliation evidence against the durable coordinator session.
    pub fn process_reconciliation(
        &self,
        session: &mut DurableCoordinatorSession,
        msg: &ReconcileEvidenceMessage,
        now: Tick,
    ) -> Result<TerminalReceiptMessage, ReconciliationError> {
        // 1. Provenance Verification
        if msg.provenance != AUTHORITATIVE_PROVENANCE {
            return Err(ReconciliationError::UnauthorizedProvenance(
                msg.provenance.clone(),
            ));
        }

        // 2. Observer Authority Token Verification
        if msg.authority_token != self.trusted_observer_token {
            return Err(ReconciliationError::UnauthorizedToken);
        }

        // 3. Fetch durable record from SQLite journal
        let rec = session
            .journal()
            .get_record(msg.operation_id)
            .map_err(ReconciliationError::Durable)?;

        // 4. Verify operation is quarantined
        if rec.durable_state != DurableOperationState::Quarantined {
            return Err(ReconciliationError::NotQuarantined(msg.operation_id.0));
        }

        // 5. Verify window key and native UUID binding
        if msg.evidence.verified_window_key != rec.window_key {
            return Err(ReconciliationError::WindowKeyMismatch {
                expected: rec.window_key.runtime_id,
                got: msg.evidence.verified_window_key.runtime_id.clone(),
            });
        }
        if msg.evidence.observed_native_uuid != rec.native_uuid {
            return Err(ReconciliationError::NativeUuidMismatch {
                expected: rec.native_uuid,
                got: msg.evidence.observed_native_uuid,
            });
        }

        // 6. Verify observer incarnation matches active observer and record
        if msg.evidence.observer_incarnation != self.active_observer_incarnation
            || msg.evidence.observer_incarnation != rec.observer_incarnation
        {
            return Err(ReconciliationError::ObserverIncarnationMismatch {
                expected: rec.observer_incarnation,
                got: msg.evidence.observer_incarnation,
            });
        }

        // 7. Strict fresh sequence enforcement: seq > rec.observer_version_seq
        if msg.evidence.observer_sequence <= rec.observer_version_seq {
            return Err(ReconciliationError::StaleObserverSequence {
                recorded: rec.observer_version_seq,
                received: msg.evidence.observer_sequence,
            });
        }

        // 8. Decision validation
        if msg.evidence.authoritative_decision.trim().is_empty() {
            return Err(ReconciliationError::EmptyAuthoritativeDecision);
        }

        // 9. Commit reconciliation transition in durable SQLite journal
        session
            .journal_mut()
            .reconcile_quarantined_operation(msg.operation_id, msg.evidence.clone(), now)
            .map_err(ReconciliationError::Durable)?;

        // 10. Clear in-memory exclusion lock in lease manager
        session
            .lease_manager_mut()
            .reconcile_quarantined_window(&rec.window_key);

        Ok(TerminalReceiptMessage {
            protocol_version: super::ipc::PROTOCOL_VERSION,
            session_id: msg.session_id,
            operation_id: msg.operation_id,
            lease_id: rec.lease_id,
            final_state: DurableOperationState::Reconciled,
            timestamp_ms: now.0,
        })
    }
}
