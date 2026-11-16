//! The observation contract exposes no native window-control operations:
//! ```compile_fail
//! use depthos_core::observed::ObservedStateSource;
//! fn move_native_window(source: &dyn ObservedStateSource) {
//!     source.move_window("window-1", "desktop-1");
//! }
//! ```
//!
//! Application-facing native observations use the authenticated adapter's exact
//! model and availability contract. There is no second projection or reducer.
//!
//! Phase 2.1's unused placeholder structs were replaced in Phase 2.2B: geometry
//! stays fractional, null topology stays unknown, and health/watch semantics are
//! provided by the same source that validates session/sequence/revision.
pub use depthos_observer::{
    Capabilities, Classification, Desktop, Geometry, NativeState, ObservationError,
    ObservationHealth, ObservedSnapshot, ObservedStateSource, Observer, Output, Window,
    WindowIdentity,
};
use tokio::sync::watch;

/// Explicit absence of a receiver; useful for headless logical CRUD. This stores
/// no runtime windows and performs no protocol reduction or bus access.
pub struct DisconnectedObservations {
    changes: watch::Sender<u64>,
}
impl Default for DisconnectedObservations {
    fn default() -> Self {
        Self {
            changes: watch::channel(0).0,
        }
    }
}
impl ObservedStateSource for DisconnectedObservations {
    fn snapshot(&self) -> Result<ObservedSnapshot, ObservationError> {
        Err(ObservationError::Unavailable(
            ObservationHealth::Disconnected,
        ))
    }
    fn health(&self) -> ObservationHealth {
        ObservationHealth::Disconnected
    }
    fn subscribe(&self) -> watch::Receiver<u64> {
        self.changes.subscribe()
    }
}
