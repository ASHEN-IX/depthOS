use super::{
    Command, ControllerError, ControllerHealth, ControllerOptions, Lifecycle, RecoveryReason,
    SpatialRead,
};
use crate::{
    application::Application,
    context::{ContextCoordinator, ContextError, Selection, Version},
    observed::{
        ObservationError, ObservationHealth, ObservedSnapshot, ObservedStateSource, Observer,
    },
    sqlite::SqliteStore,
};
use depthos_observer::Receiver;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{
    sync::{mpsc, watch},
    time::Instant,
};

#[derive(Clone, Default)]
struct ControllerSource {
    current: Arc<RwLock<Option<Observer>>>,
}
impl ObservedStateSource for ControllerSource {
    fn snapshot(&self) -> Result<ObservedSnapshot, ObservationError> {
        self.current
            .read()
            .unwrap()
            .as_ref()
            .ok_or(ObservationError::Unavailable(
                ObservationHealth::Disconnected,
            ))?
            .snapshot()
    }
    fn health(&self) -> ObservationHealth {
        self.current
            .read()
            .unwrap()
            .as_ref()
            .map_or(ObservationHealth::Disconnected, ObservedStateSource::health)
    }
    fn subscribe(&self) -> watch::Receiver<u64> {
        self.current
            .read()
            .unwrap()
            .as_ref()
            .map_or_else(|| watch::channel(0).1, ObservedStateSource::subscribe)
    }
}
struct WorkerGuard {
    health: watch::Sender<ControllerHealth>,
    source: ControllerSource,
    clean: bool,
}
impl Drop for WorkerGuard {
    fn drop(&mut self) {
        *self.source.current.write().unwrap() = None;
        if !self.clean {
            self.health.send_replace(ControllerHealth {
                lifecycle: Lifecycle::Degraded,
                observation: ObservationHealth::Disconnected,
                reason: RecoveryReason::WorkerLost,
                receiver_attempts: 0,
                receiver_connected: false,
                database_available: false,
                context: None,
            });
        }
    }
}
pub(super) struct Actor {
    app: Application<SqliteStore, ControllerSource>,
    context: ContextCoordinator,
    receiver: Option<Receiver>,
    updates: Option<watch::Receiver<u64>>,
    health: watch::Sender<ControllerHealth>,
    ever_synchronized: bool,
    attempts: u8,
    reason: RecoveryReason,
    options: ControllerOptions,
}
type Connecting = Pin<Box<dyn Future<Output = Result<Receiver, ControllerError>> + Send>>;
impl Actor {
    pub(super) fn new(
        store: SqliteStore,
        status: watch::Sender<ControllerHealth>,
        options: ControllerOptions,
    ) -> Self {
        let source = ControllerSource::default();
        Self {
            app: Application::new(store, source),
            context: ContextCoordinator::new(),
            receiver: None,
            updates: None,
            health: status,
            ever_synchronized: false,
            attempts: 0,
            reason: RecoveryReason::Connecting,
            options,
        }
    }
    fn publish(&self, h: ControllerHealth) {
        let old = self.health.borrow().clone();
        if old.lifecycle == Lifecycle::Stopping
            && !matches!(h.lifecycle, Lifecycle::Stopping | Lifecycle::Stopped)
        {
            return;
        }
        debug_assert!(old.lifecycle.allows(h.lifecycle));
        if old != h {
            self.health.send_replace(h);
        }
    }
    fn refresh(&mut self) {
        let mut result = self.context.synchronize(&self.app);
        let receiver_closed = self.receiver.as_ref().is_some_and(Receiver::is_closed);
        if receiver_closed {
            *self.app.observations().current.write().unwrap() = None;
            self.context = ContextCoordinator::new();
            result = self.context.synchronize(&self.app);
        }
        self.publish_context(result, receiver_closed);
    }
    fn publish_context(&mut self, result: Result<Version, ContextError>, receiver_closed: bool) {
        // Derive synchronized health from the same consistent context read,
        // rather than racing a second native-health read against that snapshot.
        let observation = match &result {
            Ok(_) => ObservationHealth::Synchronized,
            Err(ContextError::Unavailable(health)) => *health,
            _ => self.app.observations().health(),
        };
        let database_available = !matches!(result, Err(ContextError::Hierarchy));
        let context = result.ok();
        let (lifecycle, reason) = if context.is_some() {
            self.ever_synchronized = true;
            (Lifecycle::Synchronized, RecoveryReason::None)
        } else if !database_available {
            (Lifecycle::Degraded, RecoveryReason::DatabaseUnavailable)
        } else if receiver_closed {
            (Lifecycle::Degraded, RecoveryReason::ReceiverUnavailable)
        } else if self.receiver.is_none() {
            (
                if self.reason == RecoveryReason::Connecting {
                    if self.ever_synchronized {
                        Lifecycle::Recovering
                    } else {
                        Lifecycle::Starting
                    }
                } else {
                    Lifecycle::Degraded
                },
                self.reason,
            )
        } else if matches!(observation, ObservationHealth::Synchronized) {
            (Lifecycle::Degraded, RecoveryReason::ContextUnavailable)
        } else if self.ever_synchronized || observation == ObservationHealth::Desynchronized {
            (Lifecycle::Recovering, RecoveryReason::ObservationLost)
        } else {
            (Lifecycle::WaitingForObserver, RecoveryReason::Waiting)
        };
        self.publish(ControllerHealth {
            lifecycle,
            observation,
            reason,
            receiver_attempts: self.attempts,
            receiver_connected: self.receiver.as_ref().is_some_and(|r| !r.is_closed()),
            database_available,
            context,
        });
    }
    fn read_spatial(&mut self, cached: Option<Version>) -> SpatialRead {
        let hierarchy = self.app.hierarchy().ok();
        let mut snapshot = hierarchy
            .as_ref()
            .map(|_| self.app.observations().snapshot());
        let receiver_closed = self.receiver.as_ref().is_some_and(Receiver::is_closed);
        if receiver_closed {
            *self.app.observations().current.write().unwrap() = None;
            self.context = ContextCoordinator::new();
            snapshot = hierarchy
                .as_ref()
                .map(|_| self.app.observations().snapshot());
        }

        // Map native UUIDs and evaluate ownership from the same observation.
        let result = self
            .context
            .synchronize_from(hierarchy.as_ref(), snapshot.as_ref().map(Result::as_ref));
        let changed_context = if result
            .as_ref()
            .is_ok_and(|version| Some(version) != cached.as_ref())
        {
            snapshot
                .and_then(Result::ok)
                .map(|snapshot| (self.context.current_report(Selection::All), snapshot))
        } else {
            None
        };
        self.publish_context(result, receiver_closed);
        SpatialRead {
            hierarchy,
            health: self.health.borrow().clone(),
            changed_context,
        }
    }
    fn disconnected(&mut self) {
        self.receiver = None;
        self.updates = None;
        *self.app.observations().current.write().unwrap() = None;
        self.context = ContextCoordinator::new();
    }
    pub(super) async fn run(
        mut self,
        mut commands: mpsc::Receiver<Command>,
        mut stop: watch::Receiver<bool>,
    ) {
        let mut guard = WorkerGuard {
            health: self.health.clone(),
            source: self.app.observations().clone(),
            clean: false,
        };
        let mut connecting: Option<Connecting> = None;
        let mut retry_at = Some(Instant::now());
        loop {
            if *stop.borrow() || stop.has_changed().is_err() {
                break;
            }
            tokio::select! {
                _ = stop.changed() => break,
                result = async {connecting.as_mut().unwrap().await}, if connecting.is_some() => {
                    connecting=None;
                    match result {
                        Ok(receiver)=>{
                            let source=receiver.observer();self.updates=Some(source.subscribe()); // before first read
                            *self.app.observations().current.write().unwrap()=Some(source);self.receiver=Some(receiver);self.reason=RecoveryReason::None;
                        }
                        Err(ControllerError::Observation(e)) if e.is_name_taken()=>{self.reason=RecoveryReason::ReceiverConflict;retry_at=None;}
                        Err(_)=>{
                            self.reason=if self.attempts>=self.options.max_receiver_attempts {RecoveryReason::RetryExhausted}else{RecoveryReason::ReceiverUnavailable};
                            if self.attempts<self.options.max_receiver_attempts {retry_at=Some(Instant::now()+(self.options.retry_delay*2_u32.pow(u32::from(self.attempts.saturating_sub(1)))).min(Duration::from_secs(2)));}
                        }
                    }
                    self.refresh();
                }
                _ = async {tokio::time::sleep_until(retry_at.unwrap()).await}, if retry_at.is_some() => {
                    retry_at=None;self.attempts+=1;self.reason=RecoveryReason::Connecting;
                    let address=self.options.bus_address.clone();let timeout=self.options.connect_timeout;
                    connecting=Some(Box::pin(async move {
                        tokio::time::timeout(timeout,async {match address {Some(a)=>Receiver::at_address(&a).await,None=>Receiver::session().await}}).await.map_err(|_|ControllerError::OutcomeUnknown)?.map_err(ControllerError::Observation)
                    }));self.refresh();
                }
                changed = async {self.updates.as_mut().unwrap().changed().await}, if self.updates.is_some() => {
                    if changed.is_err() || self.receiver.as_ref().is_some_and(Receiver::is_closed) {
                        self.disconnected();self.attempts=0;self.reason=RecoveryReason::ReceiverUnavailable;retry_at=Some(Instant::now()+self.options.retry_delay);
                    }
                    self.refresh();
                }
                command = commands.recv() => {
                    let Some(command)=command else {break;};
                    if *stop.borrow() {break;}
                    if self.receiver.as_ref().is_some_and(Receiver::is_closed) {self.disconnected();self.attempts=0;self.reason=RecoveryReason::ReceiverUnavailable;retry_at=Some(Instant::now()+self.options.retry_delay);}
                    let command = match command {
                        Command::Spatial(cached, reply) => {
                            if !reply.is_closed() {
                                let _ = reply.send(Ok(self.read_spatial(cached)));
                            }
                            continue;
                        }
                        command => command,
                    };
                    self.refresh();
                    match command {
                        Command::Spatial(..) => unreachable!(),
                        Command::Health(r)=>{let _=r.send(Ok(self.health.borrow().clone()));}
                        Command::Hierarchy(r)=>{if !r.is_closed(){let _=r.send(self.app.hierarchy().map_err(ControllerError::Logical));}}
                        Command::Logical(c,r)=>{if !r.is_closed(){let result=c.execute(self.app.logical_mut()).map_err(ControllerError::Logical);if result.is_ok(){self.health.send_modify(|_| {});}let _=r.send(result);}}
                        Command::Snapshot(r)=>{if !r.is_closed(){let _=r.send(self.app.observations().snapshot().map_err(ControllerError::Observation));}}
                        Command::Report(s,r)=>{if !r.is_closed(){let _=r.send(self.context.report(&self.app,s).map_err(ControllerError::Context));}}
                        Command::Assign(k,t,v,r)=>{if !r.is_closed(){let _=r.send(self.context.assign(&self.app,&k,t,&v).map_err(ControllerError::Context));}}
                        Command::Suppress(k,v,r)=>{if !r.is_closed(){let _=r.send(self.context.unassign(&self.app,&k,&v).map_err(ControllerError::Context));}}
                        Command::Clear(k,v,r)=>{if !r.is_closed(){let _=r.send(self.context.clear_override(&self.app,&k,&v).map_err(ControllerError::Context));}}
                        Command::Reconstruct(r)=>{if !r.is_closed(){self.context=ContextCoordinator::new();let _=r.send(Ok(()));}}
                        Command::Retry(r)=>{if !r.is_closed(){if self.receiver.is_none() && connecting.is_none(){self.attempts=0;self.reason=RecoveryReason::Connecting;retry_at=Some(Instant::now());}let _=r.send(Ok(()));}}
                    }
                    self.refresh();
                }
            }
        }
        self.publish(ControllerHealth {
            lifecycle: Lifecycle::Stopping,
            observation: ObservationHealth::Stopping,
            reason: RecoveryReason::Shutdown,
            receiver_attempts: self.attempts,
            receiver_connected: self.receiver.as_ref().is_some_and(|r| !r.is_closed()),
            database_available: false,
            context: None,
        });
        connecting = None;
        drop(connecting);
        self.context = ContextCoordinator::new();
        *self.app.observations().current.write().unwrap() = None;
        let clean = if let Some(receiver) = self.receiver.take() {
            matches!(
                tokio::time::timeout(Duration::from_secs(2), receiver.shutdown()).await,
                Ok(Ok(()))
            )
        } else {
            true
        };
        self.publish(ControllerHealth {
            lifecycle: Lifecycle::Stopped,
            observation: ObservationHealth::Disconnected,
            reason: if clean {
                RecoveryReason::Shutdown
            } else {
                RecoveryReason::CleanupFailed
            },
            receiver_attempts: self.attempts,
            receiver_connected: false,
            database_available: false,
            context: None,
        });
        guard.clean = true;
    }
}
