//! Foreground runtime ownership. No native controls or external command endpoint.
//! ```compile_fail
//! use depthos_core::controller::Controller;
//! fn native_control(controller: &Controller) { controller.move_window("window-1", "desktop-1"); }
//! ```
use crate::{
    context::{Association, ContextError, ContextReport, Selection, Version, policy::WindowKey},
    domain::{self, DesktopId, LogicalDesktop, Universe, Workspace, WorkspaceId, World, WorldId},
    observed::{ObservationError, ObservationHealth, ObservedSnapshot},
    service::WorkspaceService,
    sqlite::SqliteStore,
};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
};

mod worker;
use worker::Actor;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Lifecycle {
    Starting,
    WaitingForObserver,
    Synchronized,
    Recovering,
    Degraded,
    Stopping,
    Stopped,
}
impl Lifecycle {
    pub fn allows(self, next: Self) -> bool {
        match self {
            Self::Stopped => next == Self::Stopped,
            Self::Stopping => matches!(next, Self::Stopping | Self::Stopped | Self::Degraded),
            _ => next != Self::Stopped,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RecoveryReason {
    None,
    Connecting,
    Waiting,
    ObservationLost,
    ReceiverUnavailable,
    ReceiverConflict,
    RetryExhausted,
    DatabaseUnavailable,
    ContextUnavailable,
    WorkerLost,
    Shutdown,
    CleanupFailed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ControllerHealth {
    pub lifecycle: Lifecycle,
    pub observation: ObservationHealth,
    pub reason: RecoveryReason,
    pub receiver_attempts: u8,
    pub receiver_connected: bool,
    pub database_available: bool,
    pub context: Option<Version>,
}
impl ControllerHealth {
    fn starting() -> Self {
        Self {
            lifecycle: Lifecycle::Starting,
            observation: ObservationHealth::Disconnected,
            reason: RecoveryReason::Connecting,
            receiver_attempts: 0,
            receiver_connected: false,
            database_available: true,
            context: None,
        }
    }

    fn mark_stopping(&mut self) -> bool {
        if matches!(self.lifecycle, Lifecycle::Stopping | Lifecycle::Stopped)
            || self.reason == RecoveryReason::WorkerLost
        {
            return false;
        }
        self.lifecycle = Lifecycle::Stopping;
        self.observation = ObservationHealth::Stopping;
        self.context = None;
        self.database_available = false;
        self.reason = RecoveryReason::Shutdown;
        true
    }
}
#[derive(Clone)]
pub struct ControllerOptions {
    /// None uses the explicit session address; never a system-bus fallback.
    pub bus_address: Option<String>,
    pub max_receiver_attempts: u8,
    pub retry_delay: Duration,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
}
impl Default for ControllerOptions {
    fn default() -> Self {
        Self {
            bus_address: None,
            max_receiver_attempts: 5,
            retry_delay: Duration::from_millis(100),
            connect_timeout: Duration::from_secs(2),
            request_timeout: Duration::from_secs(6),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ControllerError {
    #[error("controller options are invalid")]
    InvalidOptions,
    #[error("controller command exceeds bounded input limits")]
    InvalidCommand,
    #[error("controller requires a Tokio runtime")]
    NoRuntime,
    #[error("controller command queue is full")]
    Busy,
    #[error("controller stopped or worker unavailable")]
    Stopped,
    #[error(
        "request timed out; a started operation may have completed; do not retry mutations automatically"
    )]
    OutcomeUnknown,
    #[error("controller worker was interrupted")]
    WorkerLost,
    #[error("controller cleanup deadline exceeded")]
    ShutdownTimeout,
    #[error("receiver cleanup did not complete successfully")]
    CleanupFailed,
    #[error("logical operation failed")]
    Logical(#[source] domain::Error),
    #[error("native observations unavailable")]
    Observation(#[source] ObservationError),
    #[error("context operation failed")]
    Context(#[from] ContextError),
}
/// Typed in-process logical operations. There is no deserialization or remote dispatch.
pub enum LogicalCommand {
    CreateWorld(String, Option<String>),
    RenameWorld(WorldId, String),
    DeleteWorld(WorldId),
    Worlds,
    CreateWorkspace(WorldId, String, Option<String>, Option<String>),
    RenameWorkspace(WorkspaceId, String),
    MoveWorkspace(WorkspaceId, WorldId),
    DeleteWorkspace(WorkspaceId),
    Workspaces(WorldId),
    CreateDesktop(WorkspaceId, String, Option<String>),
    RenameDesktop(DesktopId, String),
    ReorderDesktops(WorkspaceId, Vec<DesktopId>),
    DeleteDesktop(DesktopId),
    Desktops(WorkspaceId),
}
#[derive(Debug)]
pub enum LogicalReply {
    WorldCreated(WorldId),
    WorkspaceCreated(WorkspaceId),
    DesktopCreated(DesktopId),
    Worlds(Vec<World>),
    Workspaces(Vec<Workspace>),
    Desktops(Vec<LogicalDesktop>),
    Changed,
}
impl LogicalCommand {
    fn bounded(&self) -> bool {
        let text = |s: &String| s.len() <= 4096;
        let optional = |s: &Option<String>| s.as_ref().is_none_or(text);
        match self {
            Self::CreateWorld(n, d) => text(n) && optional(d),
            Self::CreateWorkspace(_, n, d, r) => text(n) && optional(d) && optional(r),
            Self::CreateDesktop(_, n, r) => text(n) && optional(r),
            Self::RenameWorld(_, n) | Self::RenameWorkspace(_, n) | Self::RenameDesktop(_, n) => {
                text(n)
            }
            Self::ReorderDesktops(_, ids) => ids.len() <= 4096,
            _ => true,
        }
    }
    fn execute(self, s: &mut WorkspaceService<SqliteStore>) -> domain::Result<LogicalReply> {
        Ok(match self {
            Self::CreateWorld(n, d) => LogicalReply::WorldCreated(s.create_world(&n, d)?),
            Self::RenameWorld(id, n) => {
                s.rename_world(id, &n)?;
                LogicalReply::Changed
            }
            Self::DeleteWorld(id) => {
                s.delete_world(id)?;
                LogicalReply::Changed
            }
            Self::Worlds => LogicalReply::Worlds(s.worlds()?),
            Self::CreateWorkspace(w, n, d, r) => {
                LogicalReply::WorkspaceCreated(s.create_workspace(w, &n, d, r)?)
            }
            Self::RenameWorkspace(id, n) => {
                s.rename_workspace(id, &n)?;
                LogicalReply::Changed
            }
            Self::MoveWorkspace(id, w) => {
                s.move_workspace(id, w)?;
                LogicalReply::Changed
            }
            Self::DeleteWorkspace(id) => {
                s.delete_workspace(id)?;
                LogicalReply::Changed
            }
            Self::Workspaces(id) => LogicalReply::Workspaces(s.workspaces(id)?),
            Self::CreateDesktop(w, n, r) => {
                LogicalReply::DesktopCreated(s.create_desktop(w, &n, r)?)
            }
            Self::RenameDesktop(id, n) => {
                s.rename_desktop(id, &n)?;
                LogicalReply::Changed
            }
            Self::ReorderDesktops(w, ids) => {
                s.reorder_desktops(w, ids)?;
                LogicalReply::Changed
            }
            Self::DeleteDesktop(id) => {
                s.delete_desktop(id)?;
                LogicalReply::Changed
            }
            Self::Desktops(id) => LogicalReply::Desktops(s.desktops(id)?),
        })
    }
}
type Reply<T> = oneshot::Sender<Result<T, ControllerError>>;
pub(crate) struct SpatialRead {
    pub hierarchy: Option<Universe>,
    pub health: ControllerHealth,
    pub changed_context: Option<(ContextReport, ObservedSnapshot)>,
}

enum Command {
    Health(Reply<ControllerHealth>),
    Spatial(Option<Version>, Reply<SpatialRead>),
    Hierarchy(Reply<Universe>),
    Logical(LogicalCommand, Reply<LogicalReply>),
    Snapshot(Reply<ObservedSnapshot>),
    Report(Selection, Reply<ContextReport>),
    Assign(
        WindowKey,
        BTreeSet<WorkspaceId>,
        Version,
        Reply<Association>,
    ),
    Suppress(WindowKey, Version, Reply<Association>),
    Clear(WindowKey, Version, Reply<Association>),
    Reconstruct(Reply<()>),
    Retry(Reply<()>),
}
impl Command {
    fn bounded(&self) -> bool {
        let scope = |k: &WindowKey, v: &Version| {
            k.runtime_id.len() <= 4096
                && k.generation.len() <= 256
                && v.observation.observer_epoch.len() <= 256
                && v.observation.session_id.len() <= 4096
        };
        match self {
            Self::Logical(c, _) => c.bounded(),
            Self::Assign(k, t, v, _) => t.len() <= 4096 && scope(k, v),
            Self::Suppress(k, v, _) | Self::Clear(k, v, _) => scope(k, v),
            _ => true,
        }
    }
}
struct Inner {
    tx: mpsc::Sender<Command>,
    health: watch::Receiver<ControllerHealth>,
    announce_stop: Mutex<Option<watch::Sender<ControllerHealth>>>,
    cancel: watch::Sender<bool>,
    task: Mutex<Option<JoinHandle<()>>>,
    timeout: Duration,
}
/// Cloneable local handle; the worker owns all mutable application state.
#[derive(Clone)]
pub struct Controller {
    inner: Arc<Inner>,
}
impl Controller {
    pub fn start(store: SqliteStore, options: ControllerOptions) -> Result<Self, ControllerError> {
        if !(1..=8).contains(&options.max_receiver_attempts)
            || !(Duration::from_millis(10)..=Duration::from_secs(2)).contains(&options.retry_delay)
            || !(Duration::from_millis(100)..=Duration::from_secs(5))
                .contains(&options.connect_timeout)
            || !(Duration::from_secs(1)..=Duration::from_secs(30))
                .contains(&options.request_timeout)
            || options
                .bus_address
                .as_ref()
                .is_some_and(|a| a.len() > 4096 || !a.starts_with("unix:") || a.contains(';'))
        {
            return Err(ControllerError::InvalidOptions);
        }
        tokio::runtime::Handle::try_current().map_err(|_| ControllerError::NoRuntime)?;
        let (tx, commands) = mpsc::channel(16);
        let (status, health) = watch::channel(ControllerHealth::starting());
        let (cancel, stop) = watch::channel(false);
        let timeout = options.request_timeout;
        let announce_stop = Mutex::new(Some(status.clone()));
        let actor = Actor::new(store, status, options);
        let task = tokio::spawn(actor.run(commands, stop));
        Ok(Self {
            inner: Arc::new(Inner {
                tx,
                health,
                announce_stop,
                cancel,
                task: Mutex::new(Some(task)),
                timeout,
            }),
        })
    }
    async fn request<T>(
        &self,
        command: impl FnOnce(Reply<T>) -> Command,
    ) -> Result<T, ControllerError> {
        if *self.inner.cancel.borrow() {
            return Err(ControllerError::Stopped);
        }
        let (reply, response) = oneshot::channel();
        let command = command(reply);
        if !command.bounded() {
            return Err(ControllerError::InvalidCommand);
        }
        self.inner.tx.try_send(command).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => ControllerError::Busy,
            _ => ControllerError::Stopped,
        })?;
        tokio::time::timeout(self.inner.timeout, response)
            .await
            .map_err(|_| ControllerError::OutcomeUnknown)?
            .map_err(|_| ControllerError::Stopped)?
    }
    /// Summaries coalesce. Request health/snapshot/report to validate current data.
    pub fn subscribe(&self) -> watch::Receiver<ControllerHealth> {
        self.inner.health.clone()
    }
    pub async fn health(&self) -> Result<ControllerHealth, ControllerError> {
        self.request(Command::Health).await
    }
    pub(crate) async fn spatial_read(
        &self,
        cached: Option<Version>,
    ) -> Result<SpatialRead, ControllerError> {
        self.request(|reply| Command::Spatial(cached, reply)).await
    }
    pub fn last_health(&self) -> ControllerHealth {
        let mut health = self.inner.health.borrow().clone();
        if *self.inner.cancel.borrow() {
            health.mark_stopping();
        }
        health
    }
    pub async fn hierarchy(&self) -> Result<Universe, ControllerError> {
        self.request(Command::Hierarchy).await
    }
    pub async fn logical(&self, c: LogicalCommand) -> Result<LogicalReply, ControllerError> {
        self.request(|r| Command::Logical(c, r)).await
    }
    pub async fn snapshot(&self) -> Result<ObservedSnapshot, ControllerError> {
        self.request(Command::Snapshot).await
    }
    pub async fn associations(&self, s: Selection) -> Result<ContextReport, ControllerError> {
        self.request(|r| Command::Report(s, r)).await
    }
    pub async fn assign(
        &self,
        k: WindowKey,
        t: BTreeSet<WorkspaceId>,
        v: Version,
    ) -> Result<Association, ControllerError> {
        self.request(|r| Command::Assign(k, t, v, r)).await
    }
    pub async fn suppress(&self, k: WindowKey, v: Version) -> Result<Association, ControllerError> {
        self.request(|r| Command::Suppress(k, v, r)).await
    }
    pub async fn clear_override(
        &self,
        k: WindowKey,
        v: Version,
    ) -> Result<Association, ControllerError> {
        self.request(|r| Command::Clear(k, v, r)).await
    }
    /// Reconstructing deliberately discards every ephemeral assignment.
    pub async fn reconstruct_context(&self) -> Result<(), ControllerError> {
        self.request(Command::Reconstruct).await
    }
    /// Explicitly restart bounded registration attempts when no receiver is held.
    pub async fn retry_receiver(&self) -> Result<(), ControllerError> {
        self.request(Command::Retry).await
    }
    fn cleanup_result(&self) -> Result<(), ControllerError> {
        let health = self.inner.health.borrow();
        match (health.lifecycle, health.reason) {
            (Lifecycle::Stopped, RecoveryReason::CleanupFailed) => {
                Err(ControllerError::CleanupFailed)
            }
            (Lifecycle::Stopped, RecoveryReason::Shutdown) => Ok(()),
            _ => Err(ControllerError::WorkerLost),
        }
    }
    pub async fn shutdown(&self) -> Result<(), ControllerError> {
        self.inner.cancel.send_replace(true);
        let sender = self.inner.announce_stop.lock().unwrap().take();
        if let Some(sender) = sender {
            #[cfg(test)]
            tests::before_stop_publication();
            // The worker may finish while the handle is announcing cancellation.
            sender.send_if_modified(ControllerHealth::mark_stopping);
        }
        let task = self.inner.task.lock().unwrap().take();
        if let Some(mut task) = task {
            match tokio::time::timeout(Duration::from_secs(8), &mut task).await {
                Ok(Ok(())) => self.cleanup_result(),
                Ok(Err(_)) => Err(ControllerError::WorkerLost),
                Err(_) => {
                    task.abort();
                    let _ = tokio::time::timeout(Duration::from_secs(1), task).await;
                    Err(ControllerError::ShutdownTimeout)
                }
            }
        } else {
            let mut health = self.subscribe();
            tokio::time::timeout(Duration::from_secs(8), async {
                health
                    .wait_for(|h| {
                        h.lifecycle == Lifecycle::Stopped || h.reason == RecoveryReason::WorkerLost
                    })
                    .await
                    .map_err(|_| ControllerError::WorkerLost)?;
                self.cleanup_result()
            })
            .await
            .map_err(|_| ControllerError::ShutdownTimeout)?
        }
    }
}
#[cfg(test)]
mod tests;
