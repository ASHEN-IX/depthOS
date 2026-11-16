//! Read-only, session-scoped POC-02A receiver. KWin remains the authority.
mod model;
mod protocol;
mod replica;
mod transport;
pub use model::*;
pub use replica::Replica;
pub use transport::{INTERFACE, ObservedStateSource, Observer, PATH, Receiver, SERVICE};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ObservationHealth {
    Disconnected,
    Connecting,
    AwaitingSnapshot,
    Synchronized,
    Desynchronized,
    Stopping,
}
#[derive(Debug, thiserror::Error)]
pub enum ObservationError {
    #[error("invalid observer packet")]
    InvalidPacket,
    #[error("invalid observer session")]
    InvalidSession,
    #[error("stale consumer identity")]
    StaleConsumer,
    #[error("invalid live-state reference")]
    InvalidReference,
    #[error("observations unavailable: {0:?}")]
    Unavailable(ObservationHealth),
    #[error("session-bus operation failed")]
    Bus(#[from] zbus::Error),
}

impl ObservationError {
    /// A receiver must never queue for or steal the diagnostic service name.
    pub fn is_name_taken(&self) -> bool {
        matches!(self, Self::Bus(zbus::Error::NameTaken))
    }
}
