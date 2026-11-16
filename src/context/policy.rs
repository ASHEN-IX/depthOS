//! Pure policy over an allowlisted projection. No native packets, storage or I/O.
use crate::domain::WorkspaceId;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct WindowKey {
    pub epoch: Uuid,
    pub runtime_id: String,
    pub generation: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PolicyWindow {
    pub eligible: bool,
    pub dialog: bool,
    pub parent: Option<WindowKey>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    ExplicitPolicy,
    Heuristic,
    ParentHint,
    User,
    VerifiedLaunch,
}
impl Source {
    fn rank(self) -> u8 {
        match self {
            Self::ParentHint | Self::Heuristic => 1,
            Self::ExplicitPolicy => 2,
            Self::VerifiedLaunch => 3,
            Self::User => 4,
        }
    }
}
/// Prepared evidence cannot be deserialized or constructed by external callers.
/// Production launch/policy producers are deliberately disabled in this phase.
/// ```compile_fail
/// use depthos_core::{context::policy::{Claim, Source, WindowKey}, domain::WorkspaceId};
/// use std::collections::BTreeSet;
/// fn forge(key: WindowKey, targets: BTreeSet<WorkspaceId>) {
///     let _ = Claim::prepared(key, Source::VerifiedLaunch, targets);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Claim {
    pub(crate) window: WindowKey,
    pub(crate) source: Source,
    pub(crate) targets: BTreeSet<WorkspaceId>,
}
impl Claim {
    pub(crate) fn prepared(
        window: WindowKey,
        source: Source,
        targets: BTreeSet<WorkspaceId>,
    ) -> Option<Self> {
        if (targets.is_empty() && source != Source::User)
            || (targets.len() > 1 && !matches!(source, Source::User | Source::ExplicitPolicy))
        {
            return None;
        }
        Some(Self {
            window,
            source,
            targets,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Confirmed,
    Suggested,
    Conflicted,
    Unknown,
    Stale,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    ObservationUnavailableOrReplaced,
    WindowOutsidePolicyScope,
    NoApplicableEvidence,
    WorkspaceRemoved,
    EqualPrecedenceDisagreement,
    UserUnassigned,
    ReviewRequired,
    UserConfirmed,
    VerifiedWindowLaunch,
    ExplicitBindingMatched,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Decision {
    pub status: Status,
    pub reason: Reason,
    pub sources: Vec<Source>,
    /// Suggested targets are advisory. Only Confirmed has effective ownership.
    pub targets: Vec<WorkspaceId>,
    pub alternatives: Vec<Vec<WorkspaceId>>,
}
impl Decision {
    pub fn effective_targets(&self) -> &[WorkspaceId] {
        if self.status == Status::Confirmed {
            &self.targets
        } else {
            &[]
        }
    }
    fn empty(status: Status, reason: Reason) -> Self {
        Self {
            status,
            reason,
            sources: vec![],
            targets: vec![],
            alternatives: vec![],
        }
    }
}
pub fn parent_is_valid(key: &WindowKey, windows: &BTreeMap<WindowKey, PolicyWindow>) -> bool {
    let Some(window) = windows.get(key) else {
        return false;
    };
    if !window.dialog || window.parent.is_none() {
        return false;
    }
    let mut seen = BTreeSet::from([key.clone()]);
    let mut current = window.parent.as_ref();
    while let Some(parent_key) = current {
        if parent_key.epoch != key.epoch || !seen.insert(parent_key.clone()) {
            return false;
        }
        let Some(parent) = windows.get(parent_key) else {
            return false;
        };
        if !parent.eligible {
            return false;
        }
        current = parent.parent.as_ref();
    }
    true
}
pub fn decide(
    key: &WindowKey,
    epoch: Uuid,
    ready: bool,
    windows: &BTreeMap<WindowKey, PolicyWindow>,
    workspaces: &BTreeSet<WorkspaceId>,
    claims: &[Claim],
) -> Decision {
    if !ready || key.epoch != epoch || !windows.contains_key(key) {
        return Decision::empty(Status::Stale, Reason::ObservationUnavailableOrReplaced);
    }
    if !windows[key].eligible {
        return Decision::empty(Status::Unknown, Reason::WindowOutsidePolicyScope);
    }
    let applicable: Vec<_> = claims
        .iter()
        .filter(|c| {
            c.window == *key && (c.source != Source::ParentHint || parent_is_valid(key, windows))
        })
        .collect();
    let Some(rank) = applicable.iter().map(|c| c.source.rank()).max() else {
        return Decision::empty(Status::Unknown, Reason::NoApplicableEvidence);
    };
    let winners: Vec<_> = applicable
        .into_iter()
        .filter(|c| c.source.rank() == rank)
        .collect();
    let sources: BTreeSet<_> = winners.iter().map(|c| c.source).collect();
    let mut result = Decision::empty(Status::Stale, Reason::WorkspaceRemoved);
    result.sources = sources.into_iter().collect();
    if winners.iter().any(|c| !c.targets.is_subset(workspaces)) {
        return result;
    }
    let alternatives: BTreeSet<Vec<_>> = winners
        .iter()
        .map(|c| c.targets.iter().copied().collect())
        .collect();
    if alternatives.len() > 1 {
        result.status = Status::Conflicted;
        result.reason = Reason::EqualPrecedenceDisagreement;
        result.alternatives = alternatives.into_iter().collect();
        return result;
    }
    result.targets = alternatives.into_iter().next().unwrap();
    (result.status, result.reason) = if result.targets.is_empty() {
        (Status::Unknown, Reason::UserUnassigned)
    } else {
        match rank {
            1 => (Status::Suggested, Reason::ReviewRequired),
            2 => (Status::Confirmed, Reason::ExplicitBindingMatched),
            3 => (Status::Confirmed, Reason::VerifiedWindowLaunch),
            _ => (Status::Confirmed, Reason::UserConfirmed),
        }
    };
    result
}
