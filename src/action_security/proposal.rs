use super::{
    ActionRequest, Authorizer, CONSENT_TTL_MS, Error, NativeDesktop, Operation, Placement,
    RequestId, Tick, Uuid, WindowKey, World,
};

/// Immutable desktop move derived from an authority-owned synthetic snapshot.
/// Capturing or displaying a proposal grants no authority to execute it.
pub struct NativeMoveProposal {
    request: ActionRequest,
    created: Tick,
    expires: Tick,
}

impl NativeMoveProposal {
    pub fn capture(
        world: &World,
        window: &WindowKey,
        destination: NativeDesktop,
        now: Tick,
    ) -> Result<Self, Error> {
        if !world.synchronized {
            return Err(Error::Unavailable);
        }
        if world.compositor.bus_id.is_nil()
            || world.compositor.incarnation.is_nil()
            || world.version.epoch.is_nil()
            || world.topology_revision.is_nil()
        {
            return Err(Error::InvalidRequest);
        }
        let state = world.windows.get(window).ok_or(Error::Replaced)?;
        let request = ActionRequest {
            id: RequestId(Uuid::new_v4()),
            window: window.clone(),
            native_uuid: state.native_uuid,
            compositor: world.compositor.clone(),
            expected: world.version.clone(),
            topology_revision: world.topology_revision,
            operation: Operation::MoveDesktop {
                from: state.placement.clone(),
                to: Placement {
                    desktop: destination,
                    activity: state.placement.activity.clone(),
                },
            },
        };
        Authorizer::validate(&request, world)?;
        let expires = Tick(
            now.0
                .checked_add(CONSENT_TTL_MS)
                .ok_or(Error::CounterExhausted)?,
        );
        Ok(Self {
            request,
            created: now,
            expires,
        })
    }

    pub fn request(&self) -> &ActionRequest {
        &self.request
    }

    pub fn created(&self) -> Tick {
        self.created
    }

    pub fn expires(&self) -> Tick {
        self.expires
    }

    pub fn revalidate(&self, world: &World, now: Tick) -> Result<(), Error> {
        if now < self.created {
            return Err(Error::ClockReversed);
        }
        if now >= self.expires {
            return Err(Error::Expired);
        }
        Authorizer::validate(&self.request, world).map(|_| ())
    }
}

#[cfg(test)]
impl Authorizer {
    /// Models an internal fixture confirmation within the synthetic authority.
    pub(super) fn authorize_native_fixture(
        &mut self,
        caller: &super::Client,
        proposal: &NativeMoveProposal,
        world: &World,
        now: Tick,
    ) -> Result<super::Capability, Error> {
        self.advance(now)?;
        proposal.revalidate(world, now)?;
        let proof = super::Consent {
            authority: self.authority.clone(),
            nonce: self.nonce()?,
            request: proposal.request.clone(),
            client: caller.clone(),
            issued: now,
            expires: proposal.expires,
        };
        self.authorize(caller, proposal.request.clone(), proof, world, now)
    }
}

#[cfg(test)]
mod tests;
