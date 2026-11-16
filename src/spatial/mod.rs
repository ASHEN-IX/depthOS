//! Versioned, same-user read-only UI boundary. No intent authorization is
//! inferred from UID or bus-name ownership; assignment mutations are absent.
use crate::{
    context::{
        Version,
        policy::{Reason, Source, Status, WindowKey},
    },
    controller::{Controller, ControllerHealth, SpatialRead},
    domain::Universe,
};
use serde::Serialize;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};
use tokio::{sync::Semaphore, task::JoinHandle};
use uuid::Uuid;
use zbus::{
    Connection,
    fdo::{DBusProxy, Error},
    message::Header,
    object_server::SignalEmitter,
};
pub const SERVICE: &str = "org.depthos.Controller";
pub const PATH: &str = "/org/depthos/Controller";
pub const INTERFACE: &str = "org.depthos.Controller1";
pub const PAGE: usize = 64;
pub const MAX_ROWS: usize = 4096;
#[derive(Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HierarchyRow {
    pub kind: &'static str,
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
}
#[derive(Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContextRow {
    pub window: WindowKey,
    pub native_id: Option<String>,
    pub status: Status,
    pub reason: Reason,
    pub sources: Vec<Source>,
    pub targets: Vec<String>,
    pub alternatives: Vec<Vec<String>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<'a, T> {
    schema_version: u8,
    instance_id: &'a str,
    revision: &'a str,
    available: bool,
    data: T,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Page<'a, T> {
    items: &'a [T],
    next_cursor: Option<usize>,
    version: Option<&'a Version>,
}

struct Cache {
    hierarchy: Vec<HierarchyRow>,
    context: Vec<ContextRow>,
    hierarchy_revision: String,
    context_revision: String,
    health: Option<ControllerHealth>,
    version: Option<Version>,
    hierarchy_available: bool,
    context_available: bool,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            hierarchy: vec![],
            context: vec![],
            hierarchy_revision: Uuid::new_v4().to_string(),
            context_revision: Uuid::new_v4().to_string(),
            health: None,
            version: None,
            hierarchy_available: false,
            context_available: false,
        }
    }
}
struct State {
    controller: Controller,
    instance: String,
    cache: Mutex<Cache>,
    gate: Arc<Semaphore>,
    refresh: tokio::sync::Mutex<()>,
    pending: AtomicU32,
}
fn hierarchy(tree: Universe) -> Option<Vec<HierarchyRow>> {
    let mut rows = Vec::new();
    for w in tree.worlds {
        rows.push(HierarchyRow {
            kind: "world",
            id: w.world.id.0.to_string(),
            parent_id: None,
            name: w.world.name,
        });
        for p in w.workspaces {
            rows.push(HierarchyRow {
                kind: "workspace",
                id: p.workspace.id.0.to_string(),
                parent_id: Some(p.workspace.world_id.0.to_string()),
                name: p.workspace.name,
            });
            for d in p.desktops {
                rows.push(HierarchyRow {
                    kind: "desktop",
                    id: d.id.0.to_string(),
                    parent_id: Some(d.workspace_id.0.to_string()),
                    name: d.name,
                });
                if rows.len() > MAX_ROWS {
                    return None;
                }
            }
            if rows.len() > MAX_ROWS {
                return None;
            }
        }
        if rows.len() > MAX_ROWS {
            return None;
        }
    }
    Some(rows)
}
impl State {
    async fn update(&self) -> u32 {
        let _serial = self.refresh.lock().await;
        let cached = self.cache.lock().unwrap().version.clone();
        let read = self.controller.spatial_read(cached.clone()).await.ok();
        let (logical, health, changed) = match read {
            Some(SpatialRead {
                hierarchy: tree,
                health,
                changed_context,
            }) => (tree.and_then(hierarchy), Some(health), changed_context),
            None => (None, None, None),
        };
        let version = health.as_ref().and_then(|h| h.context.as_ref());
        let reuse = cached.is_some() && version == cached.as_ref() && changed.is_none();
        let context = changed.and_then(|(report, snapshot)| {
            if report.associations.len() > MAX_ROWS {
                return None;
            }
            let mut counts = std::collections::BTreeMap::new();
            for w in &snapshot.state.windows {
                if let Some(id) = &w.native_id {
                    *counts.entry(id).or_insert(0_u32) += 1;
                }
            }
            let windows: std::collections::BTreeMap<_, _> = snapshot
                .state
                .windows
                .iter()
                .map(|w| (&w.runtime_id, w))
                .collect();
            Some(
                report
                    .associations
                    .into_iter()
                    .map(|a| ContextRow {
                        native_id: windows
                            .get(&a.window.runtime_id)
                            .and_then(|w| w.native_id.as_ref())
                            .filter(|id| counts.get(id) == Some(&1))
                            .cloned(),
                        window: a.window,
                        status: a.decision.status,
                        reason: a.decision.reason,
                        sources: a.decision.sources,
                        targets: a
                            .decision
                            .targets
                            .iter()
                            .map(|id| id.0.to_string())
                            .collect(),
                        alternatives: a
                            .decision
                            .alternatives
                            .iter()
                            .map(|v| v.iter().map(|id| id.0.to_string()).collect())
                            .collect(),
                    })
                    .collect::<Vec<_>>(),
            )
        });
        let context_available = reuse || context.is_some();
        let version = version.filter(|_| context_available).cloned();
        let mut c = self.cache.lock().unwrap();
        let mut mask = 0;
        if c.hierarchy_available != logical.is_some()
            || logical.as_ref().is_some_and(|r| *r != c.hierarchy)
        {
            c.hierarchy_revision = Uuid::new_v4().to_string();
            mask |= 1;
        }
        c.hierarchy_available = logical.is_some();
        c.hierarchy = logical.unwrap_or_default();
        let epoch_changed =
            c.version.as_ref().map(|v| v.epoch) != version.as_ref().map(|v| v.epoch);
        // Observation sequence/geometry alone does not invalidate association rows.
        let health_changed = c.health.as_ref().map(|h| {
            (
                &h.lifecycle,
                &h.reason,
                &h.observation,
                h.database_available,
            )
        }) != health.as_ref().map(|h| {
            (
                &h.lifecycle,
                &h.reason,
                &h.observation,
                h.database_available,
            )
        });
        if c.context_available != context_available
            || context.as_ref().is_some_and(|r| *r != c.context)
            || epoch_changed
            || health_changed
        {
            c.context_revision = Uuid::new_v4().to_string();
            mask |= 2;
        }
        c.context_available = context_available;
        if !reuse {
            c.context = context.unwrap_or_default();
        }
        c.version = version;
        c.health = health;
        self.pending.fetch_or(mask, Ordering::Relaxed);
        mask
    }
    fn envelope(&self, revision: &str, available: bool, data: serde_json::Value) -> String {
        serde_json::json!({"schemaVersion":1,"instanceId":self.instance,"revision":revision,"available":available,"data":data}).to_string()
    }
    fn page<T: Serialize>(
        &self,
        rows: &[T],
        cursor: u32,
        expected: &str,
        revision: &str,
        available: bool,
        version: Option<&Version>,
    ) -> Result<String, Error> {
        if expected.len() > 64 || (!expected.is_empty() && expected != revision) {
            return Err(Error::InvalidArgs("Stale revision".into()));
        }
        let cursor = cursor as usize;
        if cursor > rows.len() || cursor > MAX_ROWS {
            return Err(Error::InvalidArgs("Invalid cursor".into()));
        }
        let end = (cursor + PAGE).min(rows.len());
        let output = serde_json::to_string(&Envelope {
            schema_version: 1,
            instance_id: &self.instance,
            revision,
            available,
            data: Page {
                items: &rows[cursor..end],
                next_cursor: (end < rows.len()).then_some(end),
                version,
            },
        })
        .map_err(|_| Error::Failed("UI serialization failed".into()))?;
        if output.len() > 128 * 1024 {
            return Err(Error::LimitsExceeded("Bounded UI response exceeded".into()));
        }
        Ok(output)
    }
}
struct Endpoint {
    state: Arc<State>,
}
async fn authorize(conn: &Connection, header: &Header<'_>) -> Result<(), Error> {
    if header.path().map(|x| x.as_str()) != Some(PATH)
        || header.interface().map(|x| x.as_str()) != Some(INTERFACE)
    {
        return Err(Error::AccessDenied("Invalid endpoint".into()));
    }
    let sender = header
        .sender()
        .ok_or_else(|| Error::AccessDenied("Missing sender".into()))?;
    let uid = DBusProxy::new(conn)
        .await?
        .get_connection_unix_user(sender.clone().into())
        .await?;
    if uid != rustix::process::getuid().as_raw() {
        return Err(Error::AccessDenied("Same-user reads only".into()));
    }
    Ok(())
}
#[zbus::interface(name = "org.depthos.Controller1")]
impl Endpoint {
    async fn get_capabilities(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] h: Header<'_>,
    ) -> Result<String, Error> {
        authorize(conn, &h).await?;
        Ok(self.state.envelope("1",true,serde_json::json!({"readOnly":true,"assignmentMutations":false,"selectionOwner":"client","pageSize":PAGE,"maxRows":MAX_ROWS})))
    }
    async fn get_controller_health(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] h: Header<'_>,
    ) -> Result<String, Error> {
        authorize(conn, &h).await?;
        let _permit = self
            .state
            .gate
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::LimitsExceeded("Busy".into()))?;
        self.state.update().await;
        let c = self.state.cache.lock().unwrap();
        Ok(self.state.envelope(
            &c.context_revision,
            c.health.is_some(),
            serde_json::json!({"health":c.health}),
        ))
    }
    async fn get_hierarchy(
        &self,
        cursor: u32,
        expected: &str,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] h: Header<'_>,
    ) -> Result<String, Error> {
        authorize(conn, &h).await?;
        let _permit = self
            .state
            .gate
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::LimitsExceeded("Busy".into()))?;
        self.state.update().await;
        let c = self.state.cache.lock().unwrap();
        self.state.page(
            &c.hierarchy,
            cursor,
            expected,
            &c.hierarchy_revision,
            c.hierarchy_available,
            None,
        )
    }
    async fn get_context_report(
        &self,
        cursor: u32,
        expected: &str,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] h: Header<'_>,
    ) -> Result<String, Error> {
        authorize(conn, &h).await?;
        let _permit = self
            .state
            .gate
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::LimitsExceeded("Busy".into()))?;
        self.state.update().await;
        let c = self.state.cache.lock().unwrap();
        self.state.page(
            &c.context,
            cursor,
            expected,
            &c.context_revision,
            c.context_available,
            c.version.as_ref(),
        )
    }
    async fn get_current_logical_context(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] h: Header<'_>,
    ) -> Result<String, Error> {
        authorize(conn, &h).await?;
        Ok(self.state.envelope("1",true,serde_json::json!({"owner":"client","worldId":null,"workspaceId":null,"desktopId":null})))
    }
    #[zbus(signal)]
    async fn invalidated(
        emitter: &SignalEmitter<'_>,
        instance: &str,
        scopes: u32,
    ) -> zbus::Result<()>;
}
/// Foreground-owned read service; dropping aborts its monitor and closes its bus.
pub struct UiBridge {
    connection: Option<Connection>,
    task: JoinHandle<()>,
}
impl UiBridge {
    pub async fn session(
        controller: Controller,
    ) -> Result<Self, crate::controller::ControllerError> {
        let address = std::env::var("DBUS_SESSION_BUS_ADDRESS")
            .map_err(|_| crate::controller::ControllerError::InvalidOptions)?;
        Self::at_address(controller, &address).await
    }
    pub async fn at_address(
        controller: Controller,
        address: &str,
    ) -> Result<Self, crate::controller::ControllerError> {
        use crate::controller::ControllerError as E;
        if !address.starts_with("unix:") || address.contains(';') {
            return Err(E::InvalidOptions);
        }
        let state = Arc::new(State {
            controller,
            instance: Uuid::new_v4().to_string(),
            cache: Mutex::new(Cache::default()),
            gate: Arc::new(Semaphore::new(8)),
            refresh: tokio::sync::Mutex::new(()),
            pending: AtomicU32::new(0),
        });
        let conn = zbus::connection::Builder::address(address)
            .map_err(|e| E::Observation(e.into()))?
            .method_timeout(Duration::from_secs(2))
            .max_queued(8)
            .serve_at(
                PATH,
                Endpoint {
                    state: state.clone(),
                },
            )
            .map_err(|e| E::Observation(e.into()))?
            .build()
            .await
            .map_err(|e| E::Observation(e.into()))?;
        conn.request_name_with_flags(SERVICE, zbus::fdo::RequestNameFlags::DoNotQueue.into())
            .await
            .map_err(|e| E::Observation(e.into()))?;
        let c = conn.clone();
        let task = tokio::spawn(async move {
            let mut updates = state.controller.subscribe();
            let mut timer = tokio::time::interval(Duration::from_millis(100));
            let mut dirty = true;
            let mut ticks = 0;
            loop {
                tokio::select! {
                    _=c.closed()=>break,
                    result=updates.changed()=>{if result.is_err(){break;}dirty=true;},
                    _=timer.tick()=>{ticks+=1;if !dirty && ticks%5!=0 {continue;}dirty=false;state.update().await;let mask=state.pending.swap(0,Ordering::Relaxed);
                        if mask!=0 && let Ok(emitter)=SignalEmitter::new(&c,PATH){let _=Endpoint::invalidated(&emitter,&state.instance,mask).await;}
                    }
                }
            }
        });
        Ok(Self {
            connection: Some(conn),
            task,
        })
    }
    pub async fn closed(&self) {
        if let Some(c) = &self.connection {
            c.closed().await;
        }
    }
    pub async fn shutdown(mut self) -> Result<(), crate::controller::ControllerError> {
        self.task.abort();
        let _ = (&mut self.task).await;
        if let Some(c) = self.connection.take() {
            c.release_name(SERVICE)
                .await
                .map_err(|e| crate::controller::ControllerError::Observation(e.into()))?;
            c.close()
                .await
                .map_err(|e| crate::controller::ControllerError::Observation(e.into()))?;
        }
        Ok(())
    }
}
impl Drop for UiBridge {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(test)]
mod tests;
