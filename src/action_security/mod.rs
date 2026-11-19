//! Offline authorization reference model. No transport, native executor, persistence,
//! wall clock or callbacks. Production interaction proof issuance is unavailable.
//! Synthetic consent exists only in this module’s tests.
pub use crate::context::policy::WindowKey;
use crate::{context::Version, domain::WorkspaceId};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use uuid::Uuid;

mod proposal;
pub use proposal::NativeMoveProposal;

pub mod durable;
pub mod lease;

pub const MAX_ACTIONS: usize = 64;
pub const CONSENT_TTL_MS: u64 = 5_000;
pub const ACK_TTL_MS: u64 = 1_000;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RequestId(pub Uuid);
/// Authority-owned monotonic milliseconds in the model, never a wire timestamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Tick(pub u64);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Client {
    pub bus_id: Uuid,
    pub unique_name: String,
    pub uid: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compositor {
    pub bus_id: Uuid,
    pub unique_owner: String,
    pub incarnation: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NativeDesktop(pub String);
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub desktop: NativeDesktop,
    pub activity: String,
}
/// These flags are authoritative model inputs, not caller assertions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowState {
    pub native_uuid: Uuid,
    pub eligible: bool,
    pub visible: bool,
    pub capture_allowed: Option<bool>,
    pub placement: Placement,
    pub logical: BTreeSet<WorkspaceId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    /// Empty destination explicitly unassigns; >1 target explicitly shares.
    Associate {
        from: BTreeSet<WorkspaceId>,
        to: BTreeSet<WorkspaceId>,
    },
    /// Does not switch the user's desktop, focus a window or change Activity.
    MoveDesktop { from: Placement, to: Placement },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionRequest {
    pub id: RequestId,
    pub window: WindowKey,
    pub native_uuid: Uuid,
    pub compositor: Compositor,
    pub expected: Version,
    pub topology_revision: Uuid,
    pub operation: Operation,
}
/// Entirely synthetic input. This is not an alternate observer implementation.
#[derive(Clone, Debug)]
pub struct World {
    pub synchronized: bool,
    pub compositor: Compositor,
    pub version: Version,
    pub topology_revision: Uuid,
    pub windows: BTreeMap<WindowKey, WindowState>,
    pub desktops: BTreeSet<NativeDesktop>,
    pub workspaces: BTreeSet<WorkspaceId>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    UnverifiedInteraction,
    WrongAuthority,
    WrongClient,
    ConsentScope,
    Replay,
    Expired,
    ClockReversed,
    DuplicateRequest,
    Conflict,
    Capacity,
    InvalidRequest,
    Unavailable,
    Replaced,
    StaleVersion,
    Ineligible,
    SourceChanged,
    DestinationMissing,
    UnknownAction,
    InvalidState,
    CounterExhausted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Uncertainty {
    NoAcknowledgement,
    Timeout,
    CancelledAfterDispatch,
    PartialExecution,
    PostconditionMismatch,
    Replaced,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    #[cfg(test)]
    WouldDispatch,
    Authorized,
    InFlight,
    Succeeded,
    Rejected,
    Cancelled,
    Expired,
    Invalidated,
    Uncertain(Uncertainty),
    /// Observed state after ambiguity; does not attribute success to this action.
    Reconciled {
        destination_observed: bool,
    },
}
impl State {
    fn locks_window(self) -> bool {
        matches!(self, Self::Authorized | Self::InFlight | Self::Uncertain(_))
    }
}
/// No public constructor, Clone, Deserialize, or serializable bearer credential.
/// ```compile_fail
/// use depthos_core::action_security::Consent;
/// fn inspect_nonce(proof: &Consent) -> u64 {
///     proof.nonce
/// }
/// ```
pub struct Consent {
    authority: Arc<()>,
    nonce: u64,
    request: ActionRequest,
    client: Client,
    issued: Tick,
    expires: Tick,
}
/// In-memory linear handle; Debug deliberately reveals no nonce or request.
pub struct Capability {
    authority: Arc<()>,
    nonce: u64,
    id: RequestId,
}
impl std::fmt::Debug for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Capability([redacted])")
    }
}
struct Record {
    client: Client,
    request: ActionRequest,
    before: WindowState,
    state: State,
    capability_nonce: u64,
    expires: Tick,
    dispatch_deadline: Option<Tick>,
}
struct Ticket {
    authority: Arc<()>,
    id: RequestId,
    nonce: u64,
}
/// All variants affect only a mutable synthetic World owned by the caller.
#[derive(Clone, Copy, Debug)]
pub enum SyntheticOutcome {
    ApplyAndAck,
    RejectWithoutEffect,
    ApplyWithoutAck,
    PartialExecution,
    AckWithoutEffect,
    Defer,
}
#[derive(Clone, Copy)]
enum Ack {
    Applied,
    Rejected,
}

pub struct Authorizer {
    authority: Arc<()>,
    records: BTreeMap<RequestId, Record>,
    used_consent: BTreeSet<u64>,
    next_nonce: u64,
    last_tick: Tick,
}
impl Default for Authorizer {
    fn default() -> Self {
        Self::new()
    }
}
impl Authorizer {
    pub fn new() -> Self {
        Self {
            authority: Arc::new(()),
            records: BTreeMap::new(),
            used_consent: BTreeSet::new(),
            next_nonce: 0,
            last_tick: Tick(0),
        }
    }
    /// UID, process/app identity and claimed QML callbacks prove no interaction.
    /// This is the ONLY public proof acquisition path, and always refuses.
    pub fn verify_interaction_claim(
        &self,
        _caller: &Client,
        _claims_user_click: bool,
    ) -> Result<Consent, Error> {
        Err(Error::UnverifiedInteraction)
    }
    fn nonce(&mut self) -> Result<u64, Error> {
        self.next_nonce = self
            .next_nonce
            .checked_add(1)
            .ok_or(Error::CounterExhausted)?;
        Ok(self.next_nonce)
    }
    pub fn advance(&mut self, now: Tick) -> Result<(), Error> {
        if now < self.last_tick {
            return Err(Error::ClockReversed);
        }
        self.last_tick = now;
        for r in self.records.values_mut() {
            if r.state == State::Authorized && now >= r.expires {
                r.state = State::Expired;
            }
            if r.state == State::InFlight && r.dispatch_deadline.is_some_and(|t| now >= t) {
                r.state = State::Uncertain(Uncertainty::Timeout);
            }
        }
        Ok(())
    }
    fn validate(request: &ActionRequest, world: &World) -> Result<WindowState, Error> {
        let bounded = |s: &str| !s.is_empty() && s.len() <= 512;
        if !bounded(&request.window.runtime_id)
            || !bounded(&request.window.generation)
            || !bounded(&request.expected.observation.observer_epoch)
            || !bounded(&request.expected.observation.session_id)
            || request.window.epoch != request.expected.epoch
            || request.native_uuid.is_nil()
            || !request.compositor.unique_owner.starts_with(':')
            || request.compositor.unique_owner.len() > 256
        {
            return Err(Error::InvalidRequest);
        }
        if !world.synchronized {
            return Err(Error::Unavailable);
        }
        if request.compositor != world.compositor
            || request.expected.epoch != world.version.epoch
            || request.expected.observation.observer_epoch
                != world.version.observation.observer_epoch
            || request.expected.observation.session_id != world.version.observation.session_id
        {
            return Err(Error::Replaced);
        }
        if request.expected != world.version || request.topology_revision != world.topology_revision
        {
            return Err(Error::StaleVersion);
        }
        let w = world.windows.get(&request.window).ok_or(Error::Replaced)?;
        if w.native_uuid != request.native_uuid
            || world
                .windows
                .values()
                .filter(|w| w.native_uuid == request.native_uuid)
                .count()
                != 1
            || world
                .windows
                .keys()
                .filter(|k| k.runtime_id == request.window.runtime_id)
                .count()
                != 1
        {
            return Err(Error::Replaced);
        }
        if !w.eligible || !w.visible || w.capture_allowed != Some(true) {
            return Err(Error::Ineligible);
        }
        if world.version.observation.sequence == u64::MAX
            || world.version.observation.revision == u64::MAX
            || world.version.policy_revision == u64::MAX
        {
            return Err(Error::CounterExhausted);
        }
        match &request.operation {
            Operation::Associate { from, to } => {
                if from.len() > 8 || to.len() > 8 || from == to {
                    return Err(Error::InvalidRequest);
                }
                if from != &w.logical {
                    return Err(Error::SourceChanged);
                }
                if !to.is_subset(&world.workspaces) {
                    return Err(Error::DestinationMissing);
                }
            }
            Operation::MoveDesktop { from, to } => {
                if !bounded(&from.desktop.0)
                    || !bounded(&to.desktop.0)
                    || !bounded(&from.activity)
                    || !bounded(&to.activity)
                    || from == to
                    || from.activity != to.activity
                {
                    return Err(Error::InvalidRequest);
                }
                if from != &w.placement {
                    return Err(Error::SourceChanged);
                }
                if !world.desktops.contains(&from.desktop) || !world.desktops.contains(&to.desktop)
                {
                    return Err(Error::DestinationMissing);
                }
            }
        }
        Ok(w.clone())
    }
    pub fn authorize(
        &mut self,
        caller: &Client,
        request: ActionRequest,
        proof: Consent,
        world: &World,
        now: Tick,
    ) -> Result<Capability, Error> {
        self.advance(now)?;
        if !Arc::ptr_eq(&proof.authority, &self.authority) {
            return Err(Error::WrongAuthority);
        }
        if &proof.client != caller {
            return Err(Error::WrongClient);
        }
        if self.used_consent.contains(&proof.nonce) {
            return Err(Error::Replay);
        }
        if self.records.len() >= MAX_ACTIONS || self.used_consent.len() >= MAX_ACTIONS {
            return Err(Error::Capacity);
        }
        // Even a stale or mis-scoped verified approval is burned. No repair/retry.
        self.used_consent.insert(proof.nonce);
        if proof.request != request {
            return Err(Error::ConsentScope);
        }
        if now < proof.issued
            || now >= proof.expires
            || proof
                .expires
                .0
                .checked_sub(proof.issued.0)
                .is_none_or(|t| t > CONSENT_TTL_MS)
        {
            return Err(Error::Expired);
        }
        if caller.bus_id != request.compositor.bus_id
            || caller.unique_name.len() > 256
            || !caller.unique_name.starts_with(':')
        {
            return Err(Error::InvalidRequest);
        }
        if self.records.contains_key(&request.id) {
            return Err(Error::DuplicateRequest);
        }
        let before = Self::validate(&request, world)?;
        if self
            .records
            .values()
            .any(|r| r.request.window == request.window && r.state.locks_window())
        {
            return Err(Error::Conflict);
        }
        let nonce = self.nonce()?;
        let id = request.id;
        self.records.insert(
            id,
            Record {
                client: caller.clone(),
                request,
                before,
                state: State::Authorized,
                capability_nonce: nonce,
                expires: proof.expires,
                dispatch_deadline: None,
            },
        );
        Ok(Capability {
            authority: self.authority.clone(),
            nonce,
            id,
        })
    }
    /// Inspection/revocation requires the exact initiating connection binding.
    pub fn inspect(&mut self, caller: &Client, id: RequestId, now: Tick) -> Result<State, Error> {
        self.advance(now)?;
        let r = self.records.get(&id).ok_or(Error::UnknownAction)?;
        if &r.client != caller {
            return Err(Error::WrongClient);
        }
        Ok(r.state)
    }
    pub fn revoke(&mut self, caller: &Client, id: RequestId, now: Tick) -> Result<State, Error> {
        self.advance(now)?;
        let r = self.records.get_mut(&id).ok_or(Error::UnknownAction)?;
        if &r.client != caller {
            return Err(Error::WrongClient);
        }
        r.state = match r.state {
            State::Authorized => State::Cancelled,
            State::InFlight => State::Uncertain(Uncertainty::CancelledAfterDispatch),
            other => other,
        };
        Ok(r.state)
    }
    /// Disconnect revokes unused consent; dispatch already sent cannot be undone.
    pub fn client_disconnected(&mut self, caller: &Client, now: Tick) -> Result<(), Error> {
        self.advance(now)?;
        for r in self.records.values_mut().filter(|r| &r.client == caller) {
            r.state = match r.state {
                State::Authorized => State::Cancelled,
                State::InFlight => State::Uncertain(Uncertainty::CancelledAfterDispatch),
                s => s,
            };
        }
        Ok(())
    }
    /// Explicit authority input for a changed/unavailable observer or compositor.
    pub fn invalidate(&mut self, now: Tick) -> Result<(), Error> {
        self.advance(now)?;
        for r in self.records.values_mut() {
            r.state = match r.state {
                State::Authorized => State::Invalidated,
                State::InFlight => State::Uncertain(Uncertainty::Replaced),
                s => s,
            };
        }
        Ok(())
    }
    fn begin(
        &mut self,
        capability: Capability,
        caller: &Client,
        world: &World,
        now: Tick,
    ) -> Result<Ticket, Error> {
        self.advance(now)?;
        if !Arc::ptr_eq(&capability.authority, &self.authority) {
            return Err(Error::WrongAuthority);
        }
        let r = self
            .records
            .get_mut(&capability.id)
            .ok_or(Error::UnknownAction)?;
        if &r.client != caller {
            return Err(Error::WrongClient);
        }
        if r.capability_nonce != capability.nonce || r.state != State::Authorized {
            return Err(Error::Replay);
        }
        if let Err(e) = Self::validate(&r.request, world) {
            r.state = State::Invalidated;
            return Err(e);
        }
        let Some(deadline) = now.0.checked_add(ACK_TTL_MS) else {
            r.state = State::Invalidated;
            return Err(Error::CounterExhausted);
        };
        r.dispatch_deadline = Some(Tick(deadline));
        r.state = State::InFlight;
        Ok(Ticket {
            authority: self.authority.clone(),
            id: capability.id,
            nonce: capability.nonce,
        })
    }
    fn postcondition(r: &Record, world: &World) -> bool {
        let q = &r.request;
        if !world.synchronized
            || world.compositor != q.compositor
            || world.version.epoch != q.expected.epoch
            || world.version.observation.observer_epoch != q.expected.observation.observer_epoch
            || world.version.observation.session_id != q.expected.observation.session_id
            || world.topology_revision != q.topology_revision
        {
            return false;
        }
        let Some(w) = world.windows.get(&q.window) else {
            return false;
        };
        if w.native_uuid != q.native_uuid
            || !w.eligible
            || w.capture_allowed != Some(true)
            || world
                .windows
                .values()
                .filter(|w| w.native_uuid == q.native_uuid)
                .count()
                != 1
            || world
                .windows
                .keys()
                .filter(|k| k.runtime_id == q.window.runtime_id)
                .count()
                != 1
        {
            return false;
        }
        match &q.operation {
            Operation::Associate { to, .. } => {
                w.logical == *to
                    && to.is_subset(&world.workspaces)
                    && w.placement == r.before.placement
                    && world.version.policy_revision > q.expected.policy_revision
                    && world.version.observation.sequence >= q.expected.observation.sequence
                    && world.version.observation.revision >= q.expected.observation.revision
            }
            Operation::MoveDesktop { to, .. } => {
                w.placement == *to
                    && world.desktops.contains(&to.desktop)
                    && w.logical == r.before.logical
                    && world.version.observation.sequence > q.expected.observation.sequence
                    && world.version.observation.revision > q.expected.observation.revision
                    && world.version.policy_revision >= q.expected.policy_revision
            }
        }
    }
    fn finish(
        &mut self,
        ticket: Ticket,
        ack: Ack,
        world: &World,
        now: Tick,
    ) -> Result<State, Error> {
        self.advance(now)?;
        if !Arc::ptr_eq(&ticket.authority, &self.authority) {
            return Err(Error::WrongAuthority);
        }
        let r = self
            .records
            .get_mut(&ticket.id)
            .ok_or(Error::UnknownAction)?;
        if r.capability_nonce != ticket.nonce {
            return Err(Error::Replay);
        }
        if r.state != State::InFlight {
            return Err(Error::InvalidState);
        }
        r.state = match ack {
            Ack::Applied if Self::postcondition(r, world) => State::Succeeded,
            Ack::Rejected if Self::validate(&r.request, world).is_ok() => State::Rejected,
            _ => State::Uncertain(Uncertainty::PostconditionMismatch),
        };
        Ok(r.state)
    }
    /// No native calls or caller callback. Check + synthetic mutation share one
    /// mutable World borrow, modeling a future executor's required atomic turn.
    pub fn execute_synthetic(
        &mut self,
        capability: Capability,
        caller: &Client,
        world: &mut World,
        now: Tick,
        outcome: SyntheticOutcome,
    ) -> Result<State, Error> {
        let ticket = self.begin(capability, caller, world, now)?;
        let r = self.records.get(&ticket.id).unwrap();
        if matches!(
            outcome,
            SyntheticOutcome::ApplyAndAck
                | SyntheticOutcome::ApplyWithoutAck
                | SyntheticOutcome::PartialExecution
        ) {
            let w = world.windows.get_mut(&r.request.window).unwrap();
            match &r.request.operation {
                Operation::Associate { to, .. } => {
                    w.logical = to.clone();
                    world.version.policy_revision += 1;
                }
                Operation::MoveDesktop { to, .. } => {
                    w.placement = to.clone();
                    world.version.observation.sequence += 1;
                    world.version.observation.revision += 1;
                }
            }
        }
        match outcome {
            SyntheticOutcome::ApplyAndAck | SyntheticOutcome::AckWithoutEffect => {
                self.finish(ticket, Ack::Applied, world, now)
            }
            SyntheticOutcome::RejectWithoutEffect => self.finish(ticket, Ack::Rejected, world, now),
            SyntheticOutcome::Defer => Ok(State::InFlight),
            SyntheticOutcome::ApplyWithoutAck | SyntheticOutcome::PartialExecution => {
                let r = self.records.get_mut(&ticket.id).unwrap();
                if matches!(outcome, SyntheticOutcome::PartialExecution) {
                    world.windows.get_mut(&r.request.window).unwrap().eligible = false;
                    r.state = State::Uncertain(Uncertainty::PartialExecution);
                } else {
                    r.state = State::Uncertain(Uncertainty::NoAcknowledgement);
                }
                Ok(r.state)
            }
        }
    }
    /// Reconciliation reports only observed destination, never original success.
    /// Requires a fresh checkpoint and original incarnation/generation. Otherwise
    /// keep the quarantine until authority teardown; never retarget a new window.
    pub fn reconcile_synthetic(
        &mut self,
        caller: &Client,
        id: RequestId,
        world: &World,
        now: Tick,
    ) -> Result<State, Error> {
        self.advance(now)?;
        let r = self.records.get_mut(&id).ok_or(Error::UnknownAction)?;
        if &r.client != caller {
            return Err(Error::WrongClient);
        }
        if !matches!(r.state, State::Uncertain(_)) {
            return Err(Error::InvalidState);
        }
        if !world.synchronized {
            return Err(Error::Unavailable);
        }
        if world.compositor != r.request.compositor
            || world.version.epoch != r.request.expected.epoch
            || world.version.observation.observer_epoch
                != r.request.expected.observation.observer_epoch
            || world.version.observation.session_id != r.request.expected.observation.session_id
            || world.topology_revision != r.request.topology_revision
        {
            return Err(Error::Replaced);
        }
        let w = world
            .windows
            .get(&r.request.window)
            .ok_or(Error::Replaced)?;
        if w.native_uuid != r.request.native_uuid
            || world
                .windows
                .values()
                .filter(|w| w.native_uuid == r.request.native_uuid)
                .count()
                != 1
            || world
                .windows
                .keys()
                .filter(|k| k.runtime_id == r.request.window.runtime_id)
                .count()
                != 1
        {
            return Err(Error::Replaced);
        }
        if world.version.observation.sequence <= r.request.expected.observation.sequence
            || world.version.observation.revision < r.request.expected.observation.revision
            || world.version.policy_revision < r.request.expected.policy_revision
        {
            return Err(Error::StaleVersion);
        }
        r.state = State::Reconciled {
            destination_observed: Self::postcondition(r, world),
        };
        Ok(r.state)
    }
}
#[cfg(test)]
mod tests;

#[cfg(test)]
mod execution;
