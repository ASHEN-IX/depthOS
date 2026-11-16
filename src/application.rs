//! Persistent logical state and ephemeral observations share an application
//! boundary, without a cross-store transaction or implicit association policy.
use crate::{
    domain::{Result, Universe},
    observed::{DisconnectedObservations, ObservedStateSource},
    service::{Store, WorkspaceService},
};

pub struct Application<S, O = DisconnectedObservations> {
    logical: WorkspaceService<S>,
    observations: O,
}
impl<S: Store> Application<S> {
    /// Open no bus and require no compositor for persistent hierarchy operations.
    pub fn disconnected(store: S) -> Self {
        Self::new(store, DisconnectedObservations::default())
    }
}
impl<S: Store, O: ObservedStateSource> Application<S, O> {
    /// The caller retains the receiver's lifecycle owner and passes its read handle.
    pub fn new(store: S, observations: O) -> Self {
        Self {
            logical: WorkspaceService::new(store),
            observations,
        }
    }
    pub fn hierarchy(&self) -> Result<Universe> {
        self.logical.tree()
    }
    pub fn logical(&self) -> &WorkspaceService<S> {
        &self.logical
    }
    pub fn logical_mut(&mut self) -> &mut WorkspaceService<S> {
        &mut self.logical
    }
    pub fn observations(&self) -> &O {
        &self.observations
    }
}
