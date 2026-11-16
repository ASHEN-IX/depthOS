//! Ephemeral logical context. Reads verified snapshots and logical IDs; no native
//! action executor, transport owner, SQLite writer, or metadata classifier.
pub mod policy;
use crate::{
    application::Application,
    domain::{Universe, WorkspaceId, WorldId},
    observed::{
        Classification, ObservationError, ObservationHealth, ObservedSnapshot, ObservedStateSource,
    },
    service::Store,
};
use policy::{Claim, Decision, PolicyWindow, Source, Status, WindowKey};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ObservationPosition {
    pub observer_epoch: String,
    pub session_id: String,
    pub sequence: u64,
    pub revision: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Version {
    pub epoch: Uuid,
    pub observation: ObservationPosition,
    pub policy_revision: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Association {
    pub window: WindowKey,
    pub version: Version,
    pub decision: Decision,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextReport {
    pub version: Version,
    pub associations: Vec<Association>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    All,
    Unassigned,
    Conflicted,
    Suggestions,
}
#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("native observations unavailable: {0:?}")]
    Unavailable(ObservationHealth),
    #[error("logical hierarchy unavailable")]
    Hierarchy,
    #[error("compare-and-set preconditions no longer match")]
    StaleOperation,
    #[error("window is unavailable or outside policy scope")]
    IneligibleWindow,
    #[error("assignment targets must be current logical workspaces")]
    InvalidTargets,
    #[error("invalid observer continuity metadata")]
    InvalidObservation,
    #[error("policy revision exhausted")]
    RevisionExhausted,
}
struct View {
    position: ObservationPosition,
    epoch: Uuid,
    windows: BTreeMap<WindowKey, PolicyWindow>,
    workspaces: BTreeSet<WorkspaceId>,
}
/// A foreground caller owns this coordinator and the Application. Every read and
/// mutation refreshes both inputs. Subscribe to Application.observations() for
/// wakeups; CRUD callers can immediately reevaluate without a background daemon.
#[derive(Default)]
pub struct ContextCoordinator {
    revision: u64,
    workspace_versions: BTreeMap<WorkspaceId, (WorldId, i64)>,
    view: Option<View>,
    overrides: BTreeMap<WindowKey, Claim>,
    advisory: BTreeMap<WindowKey, Claim>,
    prepared: Vec<Claim>,
}
impl ContextCoordinator {
    pub fn new() -> Self {
        Self::default()
    }
    fn bump(&mut self) -> Result<(), ContextError> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(ContextError::RevisionExhausted)?;
        Ok(())
    }
    fn invalidate(&mut self) -> Result<(), ContextError> {
        if self.view.is_some()
            || !self.overrides.is_empty()
            || !self.advisory.is_empty()
            || !self.prepared.is_empty()
        {
            self.bump()?;
        }
        self.view = None;
        self.overrides.clear();
        self.advisory.clear();
        self.prepared.clear();
        Ok(())
    }
    fn refresh<S: Store, O: ObservedStateSource>(
        &mut self,
        application: &Application<S, O>,
    ) -> Result<(), ContextError> {
        let hierarchy = application.hierarchy();
        let snapshot = hierarchy
            .as_ref()
            .ok()
            .map(|_| application.observations().snapshot());
        self.synchronize_from(
            hierarchy.as_ref().ok(),
            snapshot.as_ref().map(Result::as_ref),
        )?;
        Ok(())
    }

    pub(crate) fn synchronize_from(
        &mut self,
        hierarchy: Option<&Universe>,
        snapshot: Option<Result<&ObservedSnapshot, &ObservationError>>,
    ) -> Result<Version, ContextError> {
        let Some(hierarchy) = hierarchy else {
            self.invalidate()?;
            return Err(ContextError::Hierarchy);
        };
        let workspace_versions: BTreeMap<_, _> = hierarchy
            .worlds
            .iter()
            .flat_map(|world| {
                world.workspaces.iter().map(|w| {
                    (
                        w.workspace.id,
                        (w.workspace.world_id, w.workspace.updated_at_ms),
                    )
                })
            })
            .collect();
        if workspace_versions != self.workspace_versions {
            self.bump()?;
            self.workspace_versions = workspace_versions;
            self.prepared.clear();
            self.advisory.clear();
        }
        let snapshot = match snapshot.expect("hierarchy read precedes snapshot") {
            Ok(s) => s,
            Err(e) => {
                self.invalidate()?;
                return Err(ContextError::Unavailable(match e {
                    ObservationError::Unavailable(h) => *h,
                    _ => ObservationHealth::Desynchronized,
                }));
            }
        };
        if snapshot.schema_version != 1
            || snapshot.observer_epoch.is_empty()
            || snapshot.session_id.is_empty()
            || snapshot.sequence == 0
            || snapshot.revision == 0
            || snapshot.state.windows.iter().any(|w| {
                snapshot
                    .window_generations
                    .get(&w.runtime_id)
                    .is_none_or(String::is_empty)
            })
        {
            self.invalidate()?;
            return Err(ContextError::InvalidObservation);
        }
        let position = ObservationPosition {
            observer_epoch: snapshot.observer_epoch.clone(),
            session_id: snapshot.session_id.clone(),
            sequence: snapshot.sequence,
            revision: snapshot.revision,
        };
        let continuous = self.view.as_ref().is_some_and(|v| {
            v.position.observer_epoch == position.observer_epoch
                && v.position.session_id == position.session_id
                && position.sequence >= v.position.sequence
                && position.revision >= v.position.revision
        });
        let epoch = if continuous {
            self.view.as_ref().unwrap().epoch
        } else {
            self.invalidate()?;
            Uuid::new_v4()
        };
        if self.view.as_ref().is_some_and(|v| v.position != position)
            && (!self.prepared.is_empty() || !self.advisory.is_empty())
        {
            self.bump()?;
            self.prepared.clear();
            self.advisory.clear();
        }
        let windows = project(snapshot, epoch);
        let before = self.overrides.len();
        self.overrides.retain(|k, _| windows.contains_key(k));
        if before != self.overrides.len() {
            self.bump()?;
        }
        self.view = Some(View {
            position,
            epoch,
            windows,
            workspaces: self.workspace_versions.keys().copied().collect(),
        });
        Ok(self.version())
    }
    fn version(&self) -> Version {
        let view = self.view.as_ref().unwrap();
        Version {
            epoch: view.epoch,
            observation: view.position.clone(),
            policy_revision: self.revision,
        }
    }
    fn evaluate(&self, key: &WindowKey) -> Decision {
        let view = self.view.as_ref().unwrap();
        let claims_for = |key: &WindowKey| {
            self.overrides
                .get(key)
                .into_iter()
                .chain(self.advisory.get(key))
                .chain(self.prepared.iter().filter(|c| &c.window == key))
                .cloned()
                .collect::<Vec<_>>()
        };
        let mut claims = claims_for(key);
        if policy::parent_is_valid(key, &view.windows)
            && let Some(parent) = view.windows[key].parent.as_ref()
        {
            // Derive the hint from the parent's independently confirmed decision
            // at this exact observation; suggestions can never bootstrap ownership.
            let parent_decision = policy::decide(
                parent,
                view.epoch,
                true,
                &view.windows,
                &view.workspaces,
                &claims_for(parent),
            );
            if parent_decision.status == Status::Confirmed && parent_decision.targets.len() == 1 {
                claims.push(
                    Claim::prepared(
                        key.clone(),
                        Source::ParentHint,
                        parent_decision.targets.into_iter().collect(),
                    )
                    .unwrap(),
                );
            }
        }
        policy::decide(
            key,
            view.epoch,
            true,
            &view.windows,
            &view.workspaces,
            &claims,
        )
    }
    /// Refresh continuity and logical revisions without materializing decisions.
    pub fn synchronize<S: Store, O: ObservedStateSource>(
        &mut self,
        application: &Application<S, O>,
    ) -> Result<Version, ContextError> {
        self.refresh(application)?;
        Ok(self.version())
    }

    pub fn inspect<S: Store, O: ObservedStateSource>(
        &mut self,
        application: &Application<S, O>,
        key: &WindowKey,
    ) -> Result<Association, ContextError> {
        self.refresh(application)?;
        Ok(Association {
            window: key.clone(),
            version: self.version(),
            decision: self.evaluate(key),
        })
    }
    pub fn report<S: Store, O: ObservedStateSource>(
        &mut self,
        application: &Application<S, O>,
        selection: Selection,
    ) -> Result<ContextReport, ContextError> {
        self.refresh(application)?;
        Ok(self.current_report(selection))
    }

    pub(crate) fn current_report(&self, selection: Selection) -> ContextReport {
        let version = self.version();
        let associations = self
            .view
            .as_ref()
            .unwrap()
            .windows
            .iter()
            .filter(|(_, w)| w.eligible)
            .map(|(key, _)| Association {
                window: key.clone(),
                version: version.clone(),
                decision: self.evaluate(key),
            })
            .filter(|a| match selection {
                Selection::All => true,
                Selection::Unassigned => {
                    matches!(a.decision.status, Status::Unknown | Status::Stale)
                }
                Selection::Conflicted => a.decision.status == Status::Conflicted,
                Selection::Suggestions => a.decision.status == Status::Suggested,
            })
            .collect();
        ContextReport {
            version,
            associations,
        }
    }
    fn check<S: Store, O: ObservedStateSource>(
        &mut self,
        application: &Application<S, O>,
        key: &WindowKey,
        expected: &Version,
    ) -> Result<(), ContextError> {
        self.refresh(application)?;
        if self.version() != *expected {
            return Err(ContextError::StaleOperation);
        }
        if !self
            .view
            .as_ref()
            .unwrap()
            .windows
            .get(key)
            .is_some_and(|w| w.eligible)
            || key.epoch != expected.epoch
        {
            return Err(ContextError::IneligibleWindow);
        }
        Ok(())
    }
    pub fn assign<S: Store, O: ObservedStateSource>(
        &mut self,
        application: &Application<S, O>,
        key: &WindowKey,
        targets: BTreeSet<WorkspaceId>,
        expected: &Version,
    ) -> Result<Association, ContextError> {
        self.check(application, key, expected)?;
        if targets.is_empty() || !targets.is_subset(&self.view.as_ref().unwrap().workspaces) {
            return Err(ContextError::InvalidTargets);
        }
        self.bump()?;
        self.overrides.insert(
            key.clone(),
            Claim::prepared(key.clone(), Source::User, targets).unwrap(),
        );
        Ok(Association {
            window: key.clone(),
            version: self.version(),
            decision: self.evaluate(key),
        })
    }
    /// Explicit suppression of every lower-precedence source.
    pub fn unassign<S: Store, O: ObservedStateSource>(
        &mut self,
        application: &Application<S, O>,
        key: &WindowKey,
        expected: &Version,
    ) -> Result<Association, ContextError> {
        self.check(application, key, expected)?;
        self.bump()?;
        self.overrides.insert(
            key.clone(),
            Claim::prepared(key.clone(), Source::User, BTreeSet::new()).unwrap(),
        );
        Ok(Association {
            window: key.clone(),
            version: self.version(),
            decision: self.evaluate(key),
        })
    }
    /// Remove suppression/assignment so current policy evidence can apply again.
    pub fn clear_override<S: Store, O: ObservedStateSource>(
        &mut self,
        application: &Application<S, O>,
        key: &WindowKey,
        expected: &Version,
    ) -> Result<Association, ContextError> {
        self.check(application, key, expected)?;
        self.bump()?;
        self.overrides.remove(key);
        Ok(Association {
            window: key.clone(),
            version: self.version(),
            decision: self.evaluate(key),
        })
    }
    /// Advisory-only, exact-position hint. Cannot mint verified provenance.
    pub fn suggest<S: Store, O: ObservedStateSource>(
        &mut self,
        application: &Application<S, O>,
        key: &WindowKey,
        target: WorkspaceId,
        expected: &Version,
    ) -> Result<Association, ContextError> {
        self.check(application, key, expected)?;
        if !self.view.as_ref().unwrap().workspaces.contains(&target) {
            return Err(ContextError::InvalidTargets);
        }
        self.bump()?;
        self.advisory.insert(
            key.clone(),
            Claim::prepared(key.clone(), Source::Heuristic, BTreeSet::from([target])).unwrap(),
        );
        Ok(Association {
            window: key.clone(),
            version: self.version(),
            decision: self.evaluate(key),
        })
    }
    // No production verifier exists. This internal entry is compiled only into
    // crate tests; no observer field, deserializer, CLI flag, or public producer
    // can manufacture verified launch/scoped-policy claims in normal operation.
    #[cfg(test)]
    fn accept_prevalidated<S: Store, O: ObservedStateSource>(
        &mut self,
        application: &Application<S, O>,
        claims: Vec<Claim>,
        expected: &Version,
    ) -> Result<(), ContextError> {
        self.refresh(application)?;
        if self.version() != *expected {
            return Err(ContextError::StaleOperation);
        }
        if claims.iter().any(|c| {
            c.source == Source::User || !self.view.as_ref().unwrap().windows.contains_key(&c.window)
        }) {
            return Err(ContextError::IneligibleWindow);
        }
        self.bump()?;
        self.prepared = claims;
        Ok(())
    }
}
/// Privacy boundary: no title, app/resource identifier, native UUID, geometry,
/// PID, path or claimed provenance crosses into policy evaluation.
pub fn project(snapshot: &ObservedSnapshot, epoch: Uuid) -> BTreeMap<WindowKey, PolicyWindow> {
    let keys: BTreeMap<_, _> = snapshot
        .state
        .windows
        .iter()
        .map(|w| {
            (
                w.runtime_id.clone(),
                WindowKey {
                    epoch,
                    runtime_id: w.runtime_id.clone(),
                    generation: snapshot
                        .window_generations
                        .get(&w.runtime_id)
                        .cloned()
                        .unwrap_or_default(),
                },
            )
        })
        .collect();
    snapshot
        .state
        .windows
        .iter()
        .map(|w| {
            let parent = w.parent_runtime_id.as_ref().map(|id| {
                keys.get(id).cloned().unwrap_or(WindowKey {
                    epoch,
                    runtime_id: id.clone(),
                    generation: String::new(),
                })
            });
            (
                keys[&w.runtime_id].clone(),
                PolicyWindow {
                    eligible: matches!(
                        w.classification,
                        Classification::Application | Classification::Dialog
                    ) && w.exclude_from_capture == Some(false)
                        && w.skip_taskbar == Some(false),
                    dialog: w.classification == Classification::Dialog,
                    parent,
                },
            )
        })
        .collect()
}
#[cfg(test)]
mod tests;
