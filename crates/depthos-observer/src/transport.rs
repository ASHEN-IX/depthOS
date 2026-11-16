use crate::{protocol::valid_session, *};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{sync::watch, task::JoinHandle};
use zbus::{
    Connection,
    fdo::{DBusProxy, Error as BusError},
    message::Header,
};
pub const SERVICE: &str = "org.depthos.StateInspector";
pub const PATH: &str = "/org/depthos/StateInspector";
pub const INTERFACE: &str = "org.depthos.StateInspector1";

struct Shared {
    replica: Replica,
    owner: Option<String>,
    last_packet: Option<Instant>,
    stopped: bool,
}
/// Thread-safe read handle. Changes coalesce; subscribe before reading a snapshot.
#[derive(Clone)]
pub struct Observer {
    shared: Arc<Mutex<Shared>>,
    updates: watch::Sender<u64>,
}
pub trait ObservedStateSource {
    fn snapshot(&self) -> Result<ObservedSnapshot, ObservationError>;
    fn health(&self) -> ObservationHealth;
    fn subscribe(&self) -> watch::Receiver<u64>;
}
impl ObservedStateSource for Observer {
    fn snapshot(&self) -> Result<ObservedSnapshot, ObservationError> {
        self.shared.lock().unwrap().replica.snapshot()
    }
    fn health(&self) -> ObservationHealth {
        self.shared.lock().unwrap().replica.health()
    }
    fn subscribe(&self) -> watch::Receiver<u64> {
        self.updates.subscribe()
    }
}
impl Observer {
    fn notify(&self) {
        self.updates.send_modify(|v| *v = v.wrapping_add(1));
    }
    fn stop(&self) {
        let mut s = self.shared.lock().unwrap();
        s.stopped = true;
        s.replica.invalidate(ObservationHealth::Stopping);
        drop(s);
        self.notify();
    }
}
struct Endpoint {
    observer: Observer,
}
async fn owner(conn: &Connection) -> Result<String, BusError> {
    let proxy = DBusProxy::new(conn).await?;
    Ok(proxy
        .get_name_owner("org.kde.KWin".try_into().unwrap())
        .await?
        .to_string())
}
fn trusted_sender(sender: &str, current_owner: &str, sender_uid: u32, expected_uid: u32) -> bool {
    sender == current_owner && sender_uid == expected_uid
}
async fn authorize(conn: &Connection, header: &Header<'_>) -> Result<String, BusError> {
    if header.path().map(|x| x.as_str()) != Some(PATH)
        || header.interface().map(|x| x.as_str()) != Some(INTERFACE)
    {
        return Err(BusError::AccessDenied(
            "Invalid observation endpoint".into(),
        ));
    }
    let sender = header
        .sender()
        .ok_or_else(|| BusError::AccessDenied("Missing sender".into()))?;
    let proxy = DBusProxy::new(conn).await?;
    let current = owner(conn)
        .await
        .map_err(|_| BusError::AccessDenied("No KWin owner".into()))?;
    let uid = proxy
        .get_connection_unix_user(sender.clone().into())
        .await?;
    // Recheck ownership after the credentials round trip.
    if !trusted_sender(
        sender.as_str(),
        &current,
        uid,
        rustix::process::getuid().as_raw(),
    ) || owner(conn).await? != current
    {
        return Err(BusError::AccessDenied(
            "Only this user's current KWin may publish".into(),
        ));
    }
    Ok(current)
}
#[zbus::interface(name = "org.depthos.StateInspector1")]
impl Endpoint {
    async fn hello(
        &mut self,
        session: &str,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> Result<String, BusError> {
        let sender = authorize(conn, &header).await?;
        if !valid_session(session) {
            return Err(BusError::InvalidArgs("Invalid session".into()));
        }
        let answer = {
            let mut s = self.observer.shared.lock().unwrap();
            if s.stopped {
                return Err(BusError::Failed("Receiver stopping".into()));
            }
            s.replica
                .hello(session)
                .map_err(|_| BusError::InvalidArgs("Invalid session".into()))?;
            s.owner = Some(sender);
            s.last_packet = Some(Instant::now());
            serde_json::json!({"schemaVersion":1,"consumerId":s.replica.consumer_id()}).to_string()
        };
        self.observer.notify();
        Ok(answer)
    }
    async fn push(
        &mut self,
        raw: &str,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> Result<String, BusError> {
        let sender = authorize(conn, &header).await?;
        let answer = {
            let mut s = self.observer.shared.lock().unwrap();
            if s.stopped {
                return Err(BusError::Failed("Receiver stopping".into()));
            }
            if s.owner.as_deref() != Some(&sender) {
                return Err(BusError::AccessDenied(
                    "Hello required for current owner".into(),
                ));
            }
            let need = match s.replica.push(raw) {
                Ok(need) => {
                    s.last_packet = Some(Instant::now());
                    need
                }
                Err(_) => true,
            };
            serde_json::json!({"consumerId":s.replica.consumer_id(),"needSnapshot":need})
                .to_string()
        };
        self.observer.notify();
        Ok(answer)
    }
}
/// Foreground lifecycle owner. Dropping invalidates readers and closes its connection.
/// Shutdown explicitly releases the service and joins the liveness monitor.
pub struct Receiver {
    conn: Option<Connection>,
    observer: Observer,
    monitor: JoinHandle<()>,
}
impl Receiver {
    pub async fn session() -> Result<Self, ObservationError> {
        // Never permit TCP, unixexec, autolaunch, or a system-bus fallback.
        let address = std::env::var("DBUS_SESSION_BUS_ADDRESS")
            .map_err(|_| ObservationError::InvalidSession)?;
        Self::at_address(&address).await
    }
    pub async fn at_address(address: &str) -> Result<Self, ObservationError> {
        if !address.starts_with("unix:") || address.contains(';') {
            return Err(ObservationError::InvalidSession);
        }
        let (updates, _) = watch::channel(0);
        let observer = Observer {
            shared: Arc::new(Mutex::new(Shared {
                replica: Replica::new(uuid::Uuid::new_v4().to_string()),
                owner: None,
                last_packet: None,
                stopped: false,
            })),
            updates,
        };
        observer
            .shared
            .lock()
            .unwrap()
            .replica
            .invalidate(ObservationHealth::Connecting);
        let conn = zbus::connection::Builder::address(address)?
            .max_queued(8)
            .method_timeout(Duration::from_secs(2))
            .serve_at(
                PATH,
                Endpoint {
                    observer: observer.clone(),
                },
            )?
            .build()
            .await?;
        let reply = conn
            .request_name_with_flags(SERVICE, zbus::fdo::RequestNameFlags::DoNotQueue.into())
            .await?;
        if reply != zbus::fdo::RequestNameReply::PrimaryOwner {
            return Err(zbus::Error::NameTaken.into());
        }
        {
            let mut s = observer.shared.lock().unwrap();
            if s.replica.health() == ObservationHealth::Connecting {
                s.replica.disconnect();
            }
        }
        let o = observer.clone();
        let c = conn.clone();
        let monitor = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                tokio::select! {
                    _ = c.closed() => { o.shared.lock().unwrap().replica.disconnect(); o.notify(); break; }
                    _ = interval.tick() => {
                        let expected = o.shared.lock().unwrap().owner.clone();
                        let current = owner(&c).await.ok();
                        let changed = {
                            let mut s = o.shared.lock().unwrap();
                            if s.stopped { break; }
                            if s.owner != expected { false } else if s.owner.is_some() && s.owner != current {
                                s.replica.disconnect(); s.owner = None; s.last_packet = None; true
                            } else if s.last_packet.is_some_and(|t| t.elapsed() > Duration::from_secs(5)) && s.replica.health() == ObservationHealth::Synchronized {
                                // Preserve the Hello binding so a requested snapshot can recover.
                                s.replica.mark_desynchronized(); true
                            } else { false }
                        };
                        if changed { o.notify(); }
                    }
                }
            }
        });
        Ok(Self {
            conn: Some(conn),
            observer,
            monitor,
        })
    }
    /// Distinguish receiver connection loss from an absent KWin publisher.
    pub fn is_closed(&self) -> bool {
        self.conn.as_ref().is_none_or(Connection::is_closed)
    }

    pub fn observer(&self) -> Observer {
        self.observer.clone()
    }
    pub async fn shutdown(mut self) -> Result<(), ObservationError> {
        self.observer.stop();
        self.monitor.abort();
        let _ = (&mut self.monitor).await;
        if let Some(conn) = self.conn.take() {
            conn.release_name(SERVICE).await?;
            conn.close().await?;
        }
        self.observer.shared.lock().unwrap().replica.disconnect();
        self.observer.notify();
        Ok(())
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        self.monitor.abort();
        let mut s = self.observer.shared.lock().unwrap();
        s.stopped = true;
        s.replica.disconnect();
        drop(s);
        self.observer.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::trusted_sender;
    #[test]
    fn same_uid_and_current_owner_are_both_required() {
        assert!(trusted_sender(":1.2", ":1.2", 1000, 1000));
        assert!(!trusted_sender(":1.3", ":1.2", 1000, 1000));
        assert!(!trusted_sender(":1.2", ":1.2", 1001, 1000));
        assert!(!trusted_sender(":1.2", ":1.2", 0, 1000));
    }
}
