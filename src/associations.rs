//! Logical display associations derived from a synchronized observer snapshot and
//! explicit application-owned configuration. This module grants no authority.
use crate::{
    context::{
        ContextReport,
        policy::{Status, WindowKey},
    },
    domain::{DesktopId, Universe, WorkspaceId},
    observed::{Classification, ObservedSnapshot},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum RuleKind {
    NativeDesktop,
    Application,
}

impl RuleKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NativeDesktop => "native-desktop",
            Self::Application => "application",
        }
    }

    pub(crate) fn key(self, raw: &str) -> Option<String> {
        match self {
            Self::NativeDesktop => raw
                .parse::<Uuid>()
                .ok()
                .filter(|id| !id.is_nil())
                .map(|id| id.to_string()),
            Self::Application => application_key(raw).map(str::to_owned),
        }
    }
}

fn application_key(raw: &str) -> Option<&str> {
    (!raw.is_empty()
        && raw.len() <= 160
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)))
    .then_some(raw)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Rule {
    pub kind: RuleKind,
    pub key: String,
    pub workspace: WorkspaceId,
    pub desktop: Option<DesktopId>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Assignment {
    Explicit,
    Observed,
    Unassigned,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ShelfRow {
    pub id: Uuid,
    pub workspace_id: WorkspaceId,
    pub desktop_id: Option<DesktopId>,
    pub application_id: Option<String>,
    pub assignment: Assignment,
    pub focused: bool,
}

/// The full continuity identity and evidence stay within the controller.
/// Row IDs use a receiver-minted generation, scoped by the endpoint epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WindowEvidence {
    pub key: WindowKey,
    pub observer_epoch: String,
    pub sequence: u64,
    pub row: ShelfRow,
}

/// Only a known single native desktop can establish observed ownership.
/// Explicit application rules require independent Workspace evidence and can
/// refine a Workspace-only mapping. They cannot contradict a Desktop mapping.
pub(crate) fn reconcile(
    tree: &Universe,
    rules: &[Rule],
    report: &ContextReport,
    snapshot: &ObservedSnapshot,
) -> Vec<WindowEvidence> {
    if report.version.observation.observer_epoch != snapshot.observer_epoch
        || report.version.observation.session_id != snapshot.session_id
        || report.version.observation.sequence != snapshot.sequence
        || report.version.observation.revision != snapshot.revision
    {
        return Vec::new();
    }

    let workspaces: BTreeSet<_> = tree
        .worlds
        .iter()
        .flat_map(|world| &world.workspaces)
        .map(|workspace| workspace.workspace.id)
        .collect();
    let desktops: BTreeMap<_, _> = tree
        .worlds
        .iter()
        .flat_map(|world| &world.workspaces)
        .flat_map(|workspace| &workspace.desktops)
        .map(|desktop| (desktop.id, desktop.workspace_id))
        .collect();
    let rules: BTreeMap<_, _> = rules
        .iter()
        .filter(|rule| {
            workspaces.contains(&rule.workspace)
                && rule
                    .desktop
                    .is_none_or(|id| desktops.get(&id) == Some(&rule.workspace))
        })
        .map(|rule| ((rule.kind, rule.key.as_str()), rule))
        .collect();

    let mut topology = BTreeMap::new();
    let mut seen_native = BTreeSet::new();
    let mut duplicate_native = BTreeSet::new();
    for desktop in snapshot.state.desktops.iter().flatten() {
        if let Some(id) = desktop
            .native_id
            .as_deref()
            .and_then(|id| RuleKind::NativeDesktop.key(id))
        {
            if !seen_native.insert(id.clone()) {
                duplicate_native.insert(id.clone());
            }
            topology
                .entry(desktop.id.as_str())
                .and_modify(|entry| *entry = None)
                .or_insert(Some(id));
        } else {
            topology.insert(desktop.id.as_str(), None);
        }
    }
    let windows: BTreeMap<_, _> = snapshot
        .state
        .windows
        .iter()
        .map(|window| (&window.runtime_id, window))
        .collect();
    let mut seen_rows = BTreeSet::new();
    let mut result = Vec::new();
    for association in &report.associations {
        let key = &association.window;
        if association.version != report.version
            || key.epoch != report.version.epoch
            || snapshot.window_generations.get(&key.runtime_id) != Some(&key.generation)
        {
            continue;
        }
        let (Some(window), Ok(id)) = (windows.get(&key.runtime_id), key.generation.parse::<Uuid>())
        else {
            continue;
        };
        if !matches!(
            window.classification,
            Classification::Application | Classification::Dialog
        ) || window.exclude_from_capture != Some(false)
            || window.skip_taskbar != Some(false)
        {
            continue;
        }
        let membership = match (window.on_all_desktops, window.desktop_ids.as_deref()) {
            (Some(false), Some([desktop])) => topology
                .get(desktop.as_str())
                .and_then(Option::as_ref)
                .filter(|id| !duplicate_native.contains(*id)),
            _ => None,
        };
        let observed = membership.and_then(|id| rules.get(&(RuleKind::NativeDesktop, id.as_str())));
        let confirmed = match (
            association.decision.status,
            association.decision.targets.as_slice(),
        ) {
            (Status::Confirmed, [workspace]) if workspaces.contains(workspace) => Some(*workspace),
            (Status::Confirmed | Status::Conflicted, _) => continue,
            _ => None,
        };
        if let (Some(workspace), Some(rule)) = (confirmed, observed)
            && workspace != rule.workspace
        {
            continue;
        }
        let Some(workspace_id) = confirmed.or_else(|| observed.map(|rule| rule.workspace)) else {
            // No verified Workspace. Never leak the identity into a global shelf.
            continue;
        };
        let application_id = window.application_id.as_deref().and_then(application_key);
        let explicit = application_id
            .and_then(|app| rules.get(&(RuleKind::Application, app)))
            .filter(|rule| rule.workspace == workspace_id)
            .and_then(|rule| rule.desktop);
        let observed_desktop = observed.and_then(|rule| rule.desktop);
        let (assignment, desktop_id) = match (explicit, observed_desktop) {
            (Some(a), Some(b)) if a != b => (Assignment::Unknown, None),
            (Some(a), _) => (Assignment::Explicit, Some(a)),
            (_, Some(b)) => (Assignment::Observed, Some(b)),
            _ if membership.is_none() => (Assignment::Unknown, None),
            _ => (Assignment::Unassigned, None),
        };
        if !seen_rows.insert(id) {
            continue;
        }
        result.push(WindowEvidence {
            key: key.clone(),
            observer_epoch: snapshot.observer_epoch.clone(),
            sequence: snapshot.sequence,
            row: ShelfRow {
                id,
                workspace_id,
                desktop_id,
                application_id: application_id.map(str::to_owned),
                assignment,
                focused: snapshot.state.active_window_id.as_ref() == Some(&key.runtime_id),
            },
        });
    }
    result.sort_unstable_by_key(|evidence| evidence.row.id);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::{ContextCoordinator, Selection},
        service::{Change, Store},
        sqlite::SqliteStore,
    };

    #[test]
    fn reconciliation_rejects_wrong_generation_epoch_and_workspace() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("db");
        let mut store = SqliteStore::open(&path).unwrap();
        let world = crate::domain::WorldId(Uuid::new_v4());
        let workspace = WorkspaceId(Uuid::new_v4());
        let other = WorkspaceId(Uuid::new_v4());
        let desktop = DesktopId(Uuid::new_v4());
        store
            .apply(Change::CreateWorld(world, "World".into(), None))
            .unwrap();
        for (id, name) in [(workspace, "Workspace"), (other, "Other")] {
            store
                .apply(Change::CreateWorkspace(id, world, name.into(), None, None))
                .unwrap();
        }
        store
            .apply(Change::CreateDesktop(
                desktop,
                workspace,
                "Desktop".into(),
                None,
            ))
            .unwrap();

        let native = Uuid::new_v4().to_string();
        let parsed: serde_json::Value =
            serde_json::from_str(include_str!("../tests/observer-adapter/trace.json")).unwrap();
        let mut state: crate::observed::NativeState =
            serde_json::from_value(parsed["frames"][0]["state"].clone()).unwrap();
        let id = state.windows[0].runtime_id.clone();
        state.windows[0].on_all_desktops = Some(false);
        state.windows[0].desktop_ids = Some(vec![native.clone()]);
        state.desktops = Some(vec![crate::observed::Desktop {
            id: native.clone(),
            native_id: Some(native.clone()),
            name: None,
        }]);
        let mut snapshot = ObservedSnapshot {
            observer_epoch: Uuid::new_v4().to_string(),
            window_generations: BTreeMap::from([(id, Uuid::new_v4().to_string())]),
            schema_version: 1,
            session_id: "fixture-session".into(),
            sequence: 2,
            revision: 2,
            state,
        };
        let hierarchy = store.tree().unwrap();
        let mut context = ContextCoordinator::new();
        context
            .synchronize_from(Some(&hierarchy), Some(Ok(&snapshot)))
            .unwrap();
        let mut report = context.current_report(Selection::All);
        let mut rule = Rule {
            kind: RuleKind::NativeDesktop,
            key: native,
            workspace,
            desktop: Some(desktop),
        };
        let original = reconcile(&hierarchy, std::slice::from_ref(&rule), &report, &snapshot);
        assert_eq!(original.len(), 1);
        assert_eq!(original[0].row.desktop_id, Some(desktop));
        let encoded = serde_json::to_value(&original[0].row).unwrap();
        let mut unsolicited = encoded.clone();
        unsolicited
            .as_object_mut()
            .unwrap()
            .insert("title".into(), serde_json::json!("private"));
        assert!(serde_json::from_value::<ShelfRow>(unsolicited).is_err());
        assert_eq!(
            serde_json::from_value::<ShelfRow>(encoded).unwrap(),
            original[0].row
        );

        report.associations[0].decision.status = Status::Confirmed;
        report.associations[0].decision.targets = vec![other];
        assert!(reconcile(&hierarchy, std::slice::from_ref(&rule), &report, &snapshot).is_empty());
        report.associations[0].decision.targets = vec![workspace];
        rule.desktop = None;
        assert_eq!(
            reconcile(&hierarchy, std::slice::from_ref(&rule), &report, &snapshot)[0]
                .row
                .assignment,
            Assignment::Unassigned
        );
        snapshot.state.windows[0].on_all_desktops = Some(true);
        assert_eq!(
            reconcile(&hierarchy, std::slice::from_ref(&rule), &report, &snapshot)[0]
                .row
                .assignment,
            Assignment::Unknown
        );
        snapshot.state.windows[0].on_all_desktops = Some(false);
        rule.desktop = Some(desktop);
        report.associations[0].decision.status = Status::Unknown;
        report.associations[0].decision.targets.clear();
        rule.workspace = other;
        assert!(reconcile(&hierarchy, std::slice::from_ref(&rule), &report, &snapshot).is_empty());
        rule.workspace = workspace;
        snapshot.observer_epoch = Uuid::new_v4().to_string();
        assert!(reconcile(&hierarchy, std::slice::from_ref(&rule), &report, &snapshot).is_empty());
        snapshot.observer_epoch = report.version.observation.observer_epoch.clone();
        report.associations[0].window.generation = Uuid::new_v4().to_string();
        assert!(reconcile(&hierarchy, &[rule], &report, &snapshot).is_empty());
    }
}
