use crate::{ObservationError, ObservationHealth, model::*, protocol::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Deterministic offline reducer. No I/O or persistence; not a transport authenticator.
pub struct Replica {
    consumer_id: String,
    observer_epoch: String,
    generations: BTreeMap<String, String>,
    session: Option<String>,
    position: Position,
    state: Option<NativeState>,
    windows: BTreeMap<String, Arc<Window>>,
    health: ObservationHealth,
}
impl Replica {
    pub fn new(consumer_id: String) -> Self {
        Self {
            consumer_id,
            observer_epoch: uuid::Uuid::new_v4().to_string(),
            generations: BTreeMap::new(),
            session: None,
            position: Position {
                sequence: 0,
                revision: 0,
            },
            state: None,
            windows: BTreeMap::new(),
            health: ObservationHealth::Disconnected,
        }
    }
    pub fn consumer_id(&self) -> &str {
        &self.consumer_id
    }
    pub fn health(&self) -> ObservationHealth {
        self.health
    }
    pub fn hello(&mut self, session: &str) -> Result<(), ObservationError> {
        if !valid_session(session) {
            return Err(ObservationError::InvalidSession);
        }
        self.invalidate(ObservationHealth::AwaitingSnapshot);
        self.session = Some(session.into());
        Ok(())
    }
    pub(crate) fn invalidate(&mut self, health: ObservationHealth) {
        self.observer_epoch = uuid::Uuid::new_v4().to_string();
        self.generations.clear();
        self.state = None;
        self.windows.clear();
        self.session = None;
        self.position = Position {
            sequence: 0,
            revision: 0,
        };
        self.health = health;
    }
    pub(crate) fn mark_desynchronized(&mut self) {
        self.observer_epoch = uuid::Uuid::new_v4().to_string();
        self.health = ObservationHealth::Desynchronized;
    }
    pub fn disconnect(&mut self) {
        self.invalidate(ObservationHealth::Disconnected);
    }
    pub fn snapshot(&self) -> Result<ObservedSnapshot, ObservationError> {
        if self.health != ObservationHealth::Synchronized {
            return Err(ObservationError::Unavailable(self.health));
        }
        let mut state = self
            .state
            .clone()
            .ok_or(ObservationError::Unavailable(self.health))?;
        state.windows = self.windows.values().map(|w| w.as_ref().clone()).collect();
        state.application_window_ids = self
            .windows
            .values()
            .filter(|w| {
                w.classification == Classification::Application
                    && w.skip_taskbar != Some(true)
                    && w.exclude_from_capture != Some(true)
            })
            .map(|w| w.runtime_id.clone())
            .collect();
        Ok(ObservedSnapshot {
            observer_epoch: self.observer_epoch.clone(),
            window_generations: self.generations.clone(),
            schema_version: 1,
            session_id: self.session.clone().unwrap(),
            sequence: self.position.sequence,
            revision: self.position.revision,
            state,
        })
    }
    /// True requests the producer's next snapshot. Malformed data invalidates health.
    /// A stale consumer packet is rejected without changing the current replica.
    pub fn push(&mut self, raw: &str) -> Result<bool, ObservationError> {
        // Reject a recognizably stale generation before validating its contents.
        // The transport still authenticates the sender on every call.
        #[derive(serde::Deserialize)]
        struct Consumer {
            #[serde(rename = "consumerId")]
            id: String,
        }
        if raw.len() <= MAX_BYTES
            && let Ok(c) = serde_json::from_str::<Consumer>(raw)
            && c.id != self.consumer_id
        {
            return Err(ObservationError::StaleConsumer);
        }
        let p = match validate_raw(raw).and_then(|_| decode(raw)) {
            Ok(p) => p,
            Err(e) => {
                self.mark_desynchronized();
                return Err(e);
            }
        };
        if p.consumer_id != self.consumer_id {
            return Err(ObservationError::StaleConsumer);
        }
        let result = self.accept(p);
        if result.is_err() {
            self.mark_desynchronized();
        }
        result.map(|_| self.health != ObservationHealth::Synchronized)
    }
    fn accept(&mut self, p: Packet) -> Result<(), ObservationError> {
        if self.session.as_deref() != Some(&p.session_id) {
            self.invalidate(ObservationHealth::AwaitingSnapshot);
            // Only Hello binds the observer generation. A Push from another run
            // cannot silently establish a new trusted session.
            return Ok(());
        }
        let empty = p.frames.is_empty();
        if p.status.is_some() {
            self.mark_desynchronized();
        }
        for f in p.frames {
            self.apply(f)?;
        }
        if empty && (self.position != p.position || self.state.is_none()) {
            self.mark_desynchronized();
        }
        Ok(())
    }
    fn apply(&mut self, f: Frame) -> Result<(), ObservationError> {
        if f.sequence <= self.position.sequence {
            return Ok(());
        }
        let position = Position {
            sequence: f.sequence,
            revision: f.revision,
        };
        if let Event::Snapshot { state, reason } = f.event {
            let _ = reason;
            if f.revision < self.position.revision {
                return Err(ObservationError::InvalidPacket);
            }
            let mut state = state;
            let mut windows = BTreeMap::new();
            for w in state.windows.drain(..) {
                if w.runtime_id.is_empty()
                    || windows.insert(w.runtime_id.clone(), Arc::new(w)).is_some()
                {
                    return Err(ObservationError::InvalidReference);
                }
            }
            if state
                .active_window_id
                .as_ref()
                .is_some_and(|id| !windows.contains_key(id))
                || state
                    .application_window_ids
                    .iter()
                    .any(|id| !windows.contains_key(id))
            {
                return Err(ObservationError::InvalidReference);
            }
            unique(
                state
                    .desktops
                    .as_ref()
                    .map(|v| v.iter().map(|d| d.id.as_str()).collect()),
            )?;
            unique(
                state
                    .screens
                    .as_ref()
                    .map(|v| v.iter().map(|d| d.id.as_str()).collect()),
            )?;
            let continuous = self.health == ObservationHealth::Synchronized
                && self.position.sequence.checked_add(1) == Some(f.sequence)
                && self.position.revision == f.revision;
            if self.state.is_some() && !continuous {
                self.observer_epoch = uuid::Uuid::new_v4().to_string();
            }
            let generations = windows
                .iter()
                .map(|(id, window)| {
                    let retained = continuous
                        && self
                            .windows
                            .get(id)
                            .is_some_and(|old| old.native_id == window.native_id);
                    let generation = if retained {
                        self.generations.get(id).cloned()
                    } else {
                        None
                    }
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                    (id.clone(), generation)
                })
                .collect();
            self.generations = generations;
            state.application_window_ids.clear();
            self.windows = windows;
            self.state = Some(state);
            self.position = position;
            self.health = ObservationHealth::Synchronized;
            return Ok(());
        }
        if self.health != ObservationHealth::Synchronized {
            return Ok(());
        }
        if self.position.sequence.checked_add(1) != Some(f.sequence)
            || self.position.revision.checked_add(1) != Some(f.revision)
        {
            self.mark_desynchronized();
            return Ok(());
        }
        let state = self
            .state
            .as_mut()
            .ok_or(ObservationError::InvalidReference)?;
        match f.event {
            Event::Added { id, window } => {
                if self.windows.contains_key(&id)
                    || self.windows.len() >= MAX_ITEMS
                    || id != window.runtime_id
                    || id.is_empty()
                {
                    return Err(ObservationError::InvalidReference);
                }
                self.generations
                    .insert(id.clone(), uuid::Uuid::new_v4().to_string());
                self.windows.insert(id, Arc::new(window));
            }
            Event::Metadata {
                window_id: id,
                window,
                changed_fields,
            }
            | Event::Desktop {
                window_id: id,
                window,
                changed_fields,
            }
            | Event::Activity {
                window_id: id,
                window,
                changed_fields,
            }
            | Event::Geometry {
                window_id: id,
                window,
                changed_fields,
            }
            | Event::State {
                window_id: id,
                window,
                changed_fields,
            } => {
                let _ = changed_fields;
                if !self.windows.contains_key(&id) || id != window.runtime_id {
                    return Err(ObservationError::InvalidReference);
                }
                if self
                    .windows
                    .get(&id)
                    .is_some_and(|old| old.native_id != window.native_id)
                {
                    self.generations
                        .insert(id.clone(), uuid::Uuid::new_v4().to_string());
                }
                self.windows.insert(id, Arc::new(window));
            }
            Event::Removed { id } => {
                self.generations.remove(&id);
                if self.windows.remove(&id).is_none() {
                    return Err(ObservationError::InvalidReference);
                }
                if state.active_window_id.as_deref() == Some(&id) {
                    state.active_window_id = None;
                }
            }
            Event::Focus { id } => {
                if id
                    .0
                    .as_ref()
                    .is_some_and(|id| !self.windows.contains_key(id))
                {
                    return Err(ObservationError::InvalidReference);
                }
                state.active_window_id = id.0;
            }
            Event::Desktops { desktops } => {
                unique(
                    desktops
                        .0
                        .as_ref()
                        .map(|v| v.iter().map(|d| d.id.as_str()).collect()),
                )?;
                state.desktops = desktops.0;
            }
            Event::ActiveDesktop {
                active_desktop_id,
                active_desktop_by_screen,
            } => {
                state.active_desktop_id = active_desktop_id.0;
                state.active_desktop_by_screen = active_desktop_by_screen.0;
            }
            Event::Activities { activities } => state.activities = activities.0,
            Event::ActiveActivity { id } => state.active_activity_id = id.0,
            Event::Screens { screens } => {
                unique(
                    screens
                        .0
                        .as_ref()
                        .map(|v| v.iter().map(|d| d.id.as_str()).collect()),
                )?;
                state.screens = screens.0;
            }
            Event::Snapshot { .. } => unreachable!(),
        }
        self.position = position;
        Ok(())
    }
}
fn unique(ids: Option<Vec<&str>>) -> Result<(), ObservationError> {
    if let Some(ids) = ids {
        let mut seen = BTreeSet::new();
        if ids.into_iter().any(|id| id.is_empty() || !seen.insert(id)) {
            return Err(ObservationError::InvalidReference);
        }
    }
    Ok(())
}
