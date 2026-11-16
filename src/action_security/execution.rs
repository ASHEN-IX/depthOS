//! Isolated execution-turn fixture. A future native adapter would need to retain
//! the live window on the compositor thread across the check/consume/dispatch
//! sequence. This module is compiled only for unit tests.
use super::{
    Authorizer, Client, Error, NativeDesktop, RequestId, State, Tick, Ticket, Uncertainty,
    WindowKey, World, proposal::NativeMoveProposal,
};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
struct InputContext {
    lock: Option<bool>,
    seat: Option<String>,
    focused_prompt: Option<Uuid>,
}

impl InputContext {
    fn eligible(&self) -> bool {
        self.lock == Some(false)
            && self.seat.as_deref().is_some_and(|seat| !seat.is_empty())
            && self.focused_prompt.is_some_and(|id| !id.is_nil())
    }
}

struct Pending {
    proposal: NativeMoveProposal,
    input: InputContext,
}

/// Constructed by tests in the private authority fixture. This marker has no
/// native event, transport, serialization, or QML representation.
#[derive(Clone, Copy)]
struct FixtureConfirmation {
    proposal: RequestId,
}

#[derive(Clone, Copy)]
enum Receipt {
    Recorded,
    Lost,
}

/// The dispatch contract accepts a consumed ticket and a live proposal from the
/// same authority turn. No native implementation or runtime selector exists.
trait DispatchSink {
    fn would_dispatch(&mut self, ticket: Ticket, proposal: &NativeMoveProposal) -> Receipt;
}

struct CountingSink {
    count: usize,
    receipt: Receipt,
}

impl DispatchSink for CountingSink {
    fn would_dispatch(&mut self, ticket: Ticket, proposal: &NativeMoveProposal) -> Receipt {
        assert_eq!(ticket.id, proposal.request().id);
        self.count += 1;
        self.receipt
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Execution {
    WouldDispatch,
    Uncertain(Uncertainty),
}

struct AuthorityFixture {
    auth: Authorizer,
    client: Client,
    world: World,
    input: InputContext,
    pending: Option<Pending>,
    sink: CountingSink,
    stopped: bool,
}

impl AuthorityFixture {
    fn present(
        &mut self,
        window: &WindowKey,
        destination: NativeDesktop,
        now: Tick,
    ) -> Result<RequestId, Error> {
        self.expire(now)?;
        if self.stopped || !self.input.eligible() {
            return Err(Error::Unavailable);
        }
        if self.pending.is_some() {
            return Err(Error::Conflict);
        }
        let proposal = NativeMoveProposal::capture(&self.world, window, destination, now)?;
        let id = proposal.request().id;
        self.pending = Some(Pending {
            proposal,
            input: self.input.clone(),
        });
        Ok(id)
    }

    fn observe_input(&mut self, input: InputContext) {
        if input != self.input {
            self.pending = None;
        }
        self.input = input;
    }

    fn cancel(&mut self, id: RequestId) {
        if self.pending.as_ref().map(|p| p.proposal.request().id) == Some(id) {
            self.pending = None;
        }
    }

    fn observe_loss(&mut self, now: Tick) -> Result<(), Error> {
        self.pending = None;
        self.world.synchronized = false;
        self.auth.invalidate(now)
    }

    fn teardown(&mut self, now: Tick) -> Result<(), Error> {
        self.stopped = true;
        self.observe_loss(now)
    }

    fn expire(&mut self, now: Tick) -> Result<(), Error> {
        self.auth.advance(now)?;
        if self
            .pending
            .as_ref()
            .is_some_and(|p| now >= p.proposal.expires())
        {
            self.pending = None;
        }
        Ok(())
    }

    /// The fixture owns all live dereferences and holds one exclusive borrow for
    /// the full turn. There are no awaits, callbacks to the presenter, or retries.
    fn confirm(&mut self, marker: FixtureConfirmation, now: Tick) -> Result<Execution, Error> {
        self.auth.advance(now)?;
        let pending = self.pending.as_ref().ok_or(Error::UnknownAction)?;
        if pending.proposal.request().id != marker.proposal {
            return Err(Error::UnknownAction);
        }

        // A matching marker spends the pending prompt even when revalidation fails.
        let pending = self.pending.take().ok_or(Error::UnknownAction)?;
        if !self.input.eligible() || self.input != pending.input {
            return Err(Error::UnverifiedInteraction);
        }
        let proposal = pending.proposal;
        proposal.revalidate(&self.world, now)?;
        let capability =
            self.auth
                .authorize_native_fixture(&self.client, &proposal, &self.world, now)?;
        let ticket = self
            .auth
            .begin(capability, &self.client, &self.world, now)?;
        let id = ticket.id;
        let (state, result) = match self.sink.would_dispatch(ticket, &proposal) {
            Receipt::Recorded => (State::WouldDispatch, Execution::WouldDispatch),
            Receipt::Lost => {
                let uncertainty = Uncertainty::NoAcknowledgement;
                (
                    State::Uncertain(uncertainty),
                    Execution::Uncertain(uncertainty),
                )
            }
        };
        self.auth.records.get_mut(&id).unwrap().state = state;
        Ok(result)
    }
}

mod tests;
