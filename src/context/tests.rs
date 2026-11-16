use super::*;
use crate::{
    observed::{NativeState, ObservedSnapshot},
    sqlite::SqliteStore,
};
use policy::{Reason, Source, Status};
use serde_json::{Value, json};
fn alpha() -> WorkspaceId {
    WorkspaceId(Uuid::from_u128(1))
}
fn beta() -> WorkspaceId {
    WorkspaceId(Uuid::from_u128(2))
}
fn a() -> WindowKey {
    WindowKey {
        epoch: Uuid::from_u128(1),
        runtime_id: "window-1".into(),
        generation: "generation-1".into(),
    }
}
fn b() -> WindowKey {
    WindowKey {
        runtime_id: "window-2".into(),
        generation: "generation-2".into(),
        ..a()
    }
}
fn replacement() -> WindowKey {
    WindowKey {
        epoch: Uuid::from_u128(2),
        ..a()
    }
}
fn window() -> PolicyWindow {
    PolicyWindow {
        eligible: true,
        dialog: false,
        parent: None,
    }
}
fn windows() -> BTreeMap<WindowKey, PolicyWindow> {
    BTreeMap::from([(a(), window()), (b(), window())])
}
fn claim(key: WindowKey, source: Source, targets: &[WorkspaceId]) -> Claim {
    Claim::prepared(key, source, targets.iter().copied().collect()).unwrap()
}
fn decide(claims: &[Claim]) -> Decision {
    policy::decide(
        &a(),
        a().epoch,
        true,
        &windows(),
        &BTreeSet::from([alpha(), beta()]),
        claims,
    )
}
fn snapshot() -> ObservedSnapshot {
    let t: Value =
        serde_json::from_str(include_str!("../../tests/observer-adapter/trace.json")).unwrap();
    ObservedSnapshot {
        schema_version: 1,
        observer_epoch: "observer-a".into(),
        window_generations: BTreeMap::from([("window-1".into(), "generation-1".into())]),
        session_id: "test-session".into(),
        sequence: 1,
        revision: 1,
        state: serde_json::from_value::<NativeState>(t["frames"][0]["state"].clone()).unwrap(),
    }
}
#[test]
fn two_editor_windows_with_same_application_id() {
    let claims = [
        claim(a(), Source::User, &[alpha()]),
        claim(b(), Source::User, &[beta()]),
    ];
    assert_eq!(decide(&claims).targets, [alpha()]);
    assert_eq!(
        policy::decide(
            &b(),
            a().epoch,
            true,
            &windows(),
            &BTreeSet::from([alpha(), beta()]),
            &claims
        )
        .targets,
        [beta()]
    );
}
#[test]
fn identical_terminal_app_ids_and_shared_process() {
    let mut s = snapshot();
    s.state.windows[0].application_id = Some("org.kde.konsole".into());
    let mut second = s.state.windows[0].clone();
    second.runtime_id = b().runtime_id;
    s.state.windows.push(second);
    s.window_generations
        .insert("window-2".into(), b().generation);
    let projection = project(&s, a().epoch);
    let claims = [claim(a(), Source::VerifiedLaunch, &[alpha()])];
    assert_eq!(
        policy::decide(
            &a(),
            a().epoch,
            true,
            &projection,
            &BTreeSet::from([alpha()]),
            &claims
        )
        .status,
        Status::Confirmed
    );
    assert_eq!(
        policy::decide(
            &b(),
            a().epoch,
            true,
            &projection,
            &BTreeSet::from([alpha()]),
            &claims
        )
        .status,
        Status::Unknown
    );
}
#[test]
fn browser_unknown_profiles() {
    let mut s = snapshot();
    s.state.windows[0].application_id = Some("org.example.Browser".into());
    s.state.windows[0].title = Some("Private profile A".into());
    let mut second = s.state.windows[0].clone();
    second.runtime_id = "window-2".into();
    second.title = Some("Private profile B".into());
    s.state.windows.push(second);
    s.window_generations
        .insert("window-2".into(), "generation-2".into());
    let projection = project(&s, a().epoch);
    for key in projection.keys() {
        assert_eq!(
            policy::decide(
                key,
                a().epoch,
                true,
                &projection,
                &BTreeSet::from([alpha(), beta()]),
                &[]
            )
            .status,
            Status::Unknown
        );
    }
}
#[test]
fn explicit_shared_target_is_one_claim() {
    let c = claim(a(), Source::ExplicitPolicy, &[beta(), alpha()]);
    assert_eq!(decide(&[c]).targets, [alpha(), beta()]);
    assert_eq!(
        decide(&[
            claim(a(), Source::ExplicitPolicy, &[alpha()]),
            claim(a(), Source::ExplicitPolicy, &[beta()])
        ])
        .status,
        Status::Conflicted
    );
    assert!(Claim::prepared(a(), Source::Heuristic, BTreeSet::from([alpha(), beta()])).is_none());
}
#[test]
fn restart_with_window_uuid_replacement_does_not_inherit() {
    let map = BTreeMap::from([(b(), window())]);
    let claims = [claim(a(), Source::User, &[alpha()])];
    assert_eq!(
        policy::decide(
            &a(),
            a().epoch,
            true,
            &map,
            &BTreeSet::from([alpha()]),
            &claims
        )
        .status,
        Status::Stale
    );
    assert_eq!(
        policy::decide(
            &b(),
            a().epoch,
            true,
            &map,
            &BTreeSet::from([alpha()]),
            &claims
        )
        .status,
        Status::Unknown
    );
}
#[test]
fn pid_reuse_cannot_carry_a_launch_claim() {
    // PID and processStartTick are absent from the production typed projection.
    let mut raw = serde_json::to_value(&snapshot().state.windows[0]).unwrap();
    raw["pid"] = json!(321);
    raw["processStartTick"] = json!(90);
    let parsed: crate::observed::Window = serde_json::from_value(raw).unwrap();
    let mut s = snapshot();
    s.state.windows[0] = parsed;
    assert!(
        !serde_json::to_string(&project(&s, a().epoch).values().collect::<Vec<_>>())
            .unwrap()
            .contains("321")
    );
    assert_eq!(
        policy::decide(
            &b(),
            a().epoch,
            true,
            &windows(),
            &BTreeSet::from([alpha()]),
            &[claim(a(), Source::VerifiedLaunch, &[alpha()])]
        )
        .status,
        Status::Unknown
    );
}
#[test]
fn invalid_parent_references_never_inherit() {
    let hints = [claim(a(), Source::ParentHint, &[alpha()])];
    for parent in [a(), b(), replacement()] {
        let map = BTreeMap::from([(
            a(),
            PolicyWindow {
                dialog: true,
                parent: Some(parent),
                ..window()
            },
        )]);
        assert_eq!(
            policy::decide(
                &a(),
                a().epoch,
                true,
                &map,
                &BTreeSet::from([alpha()]),
                &hints
            )
            .status,
            Status::Unknown
        );
    }
    for parent in [
        PolicyWindow {
            parent: Some(a()),
            ..window()
        },
        PolicyWindow {
            eligible: false,
            ..window()
        },
    ] {
        let map = BTreeMap::from([
            (
                a(),
                PolicyWindow {
                    dialog: true,
                    parent: Some(b()),
                    ..window()
                },
            ),
            (b(), parent),
        ]);
        assert!(!policy::parent_is_valid(&a(), &map));
    }
    let map = BTreeMap::from([
        (
            a(),
            PolicyWindow {
                dialog: true,
                parent: Some(b()),
                ..window()
            },
        ),
        (b(), window()),
    ]);
    assert_eq!(
        policy::decide(
            &a(),
            a().epoch,
            true,
            &map,
            &BTreeSet::from([alpha()]),
            &hints
        )
        .status,
        Status::Suggested
    );
    assert!(!policy::parent_is_valid(&a(), &windows()));
}
#[test]
fn user_reassignment_wins_and_unassignment_suppresses_fallback() {
    let launch = claim(a(), Source::VerifiedLaunch, &[alpha()]);
    assert_eq!(
        decide(&[launch.clone(), claim(a(), Source::User, &[beta()])]).targets,
        [beta()]
    );
    assert_eq!(
        decide(&[launch, claim(a(), Source::User, &[])]).reason,
        Reason::UserUnassigned
    );
    assert_eq!(
        decide(&[
            claim(a(), Source::User, &[alpha()]),
            claim(a(), Source::User, &[beta()])
        ])
        .status,
        Status::Conflicted
    );
}
#[test]
fn verified_launch_precedes_policy_and_duplicate_claims_agree() {
    let launch = claim(a(), Source::VerifiedLaunch, &[beta()]);
    let d = decide(&[
        claim(a(), Source::ExplicitPolicy, &[alpha()]),
        launch.clone(),
        launch.clone(),
    ]);
    assert_eq!(d.reason, Reason::VerifiedWindowLaunch);
    assert_eq!(d.targets, [beta()]);
    assert_eq!(
        decide(&[launch, claim(a(), Source::VerifiedLaunch, &[alpha()])]).status,
        Status::Conflicted
    );
}
#[test]
fn conflicting_automatic_suggestions_are_order_independent() {
    let map = BTreeMap::from([
        (
            a(),
            PolicyWindow {
                dialog: true,
                parent: Some(b()),
                ..window()
            },
        ),
        (b(), window()),
    ]);
    let c = [
        claim(a(), Source::Heuristic, &[beta()]),
        claim(a(), Source::ParentHint, &[alpha()]),
        claim(b(), Source::User, &[alpha()]),
    ];
    let mut results = Vec::new();
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        results.push(policy::decide(
            &a(),
            a().epoch,
            true,
            &map,
            &BTreeSet::from([alpha(), beta()]),
            &order.map(|i| c[i].clone()),
        ));
    }
    assert!(results.iter().all(|d| *d == results[0]));
    assert_eq!(results[0].alternatives, [vec![alpha()], vec![beta()]]);
    assert_eq!(results[0].sources, [Source::Heuristic, Source::ParentHint]);
}
#[test]
fn unknown_application_requires_real_evidence() {
    let mut s = snapshot();
    s.state.windows[0].application_id = None;
    let map = project(&s, a().epoch);
    assert_eq!(
        policy::decide(&a(), a().epoch, true, &map, &BTreeSet::from([alpha()]), &[]).status,
        Status::Unknown
    );
    assert_eq!(
        policy::decide(
            &a(),
            a().epoch,
            true,
            &map,
            &BTreeSet::from([alpha()]),
            &[claim(a(), Source::User, &[alpha()])]
        )
        .status,
        Status::Confirmed
    );
}
#[test]
fn reconnect_requires_reconciled_snapshot_and_retained_epoch() {
    let claims = [claim(a(), Source::User, &[alpha()])];
    let ids = BTreeSet::from([alpha()]);
    assert_eq!(
        policy::decide(&a(), a().epoch, false, &windows(), &ids, &claims).status,
        Status::Stale
    );
    assert_eq!(decide(&claims).status, Status::Confirmed);
    assert_eq!(
        policy::decide(&a(), replacement().epoch, true, &windows(), &ids, &claims).status,
        Status::Stale
    );
    assert_eq!(
        policy::decide(
            &replacement(),
            replacement().epoch,
            true,
            &BTreeMap::from([(replacement(), window())]),
            &ids,
            &claims
        )
        .status,
        Status::Unknown
    );
}
#[test]
fn workspace_deletion_leaves_active_window_unowned() {
    for claims in [
        vec![
            claim(a(), Source::User, &[alpha()]),
            claim(a(), Source::ExplicitPolicy, &[beta()]),
        ],
        vec![claim(a(), Source::User, &[alpha(), beta()])],
    ] {
        let d = policy::decide(
            &a(),
            a().epoch,
            true,
            &windows(),
            &BTreeSet::from([beta()]),
            &claims,
        );
        assert_eq!(d.reason, Reason::WorkspaceRemoved);
        assert!(d.targets.is_empty());
    }
}
#[test]
fn heuristic_has_no_write_path_or_confirmation() {
    let d = decide(&[claim(a(), Source::Heuristic, &[alpha()])]);
    assert_eq!(d.status, Status::Suggested);
    assert!(d.effective_targets().is_empty());
    assert_eq!(decide(&[]).status, Status::Unknown);
}
#[test]
fn projection_redacts_secrets_and_ignores_claim_like_fields() {
    let mut s = snapshot();
    let private = "sensitive-fragment";
    let w = &mut s.state.windows[0];
    w.title = Some(private.into());
    w.application_id = Some(private.into());
    w.resource_class = Some(private.into());
    w.native_id = Some(private.into());
    let map = project(&s, a().epoch);
    let d = policy::decide(&a(), a().epoch, true, &map, &BTreeSet::new(), &[]);
    let output = serde_json::to_string(&(map.values().collect::<Vec<_>>(), d)).unwrap();
    assert!(!output.contains(private));
}
#[test]
fn protected_or_unclassified_windows_are_excluded() {
    for (classification, protection, skip) in [
        (Classification::Application, Some(true), Some(false)),
        (Classification::Application, None, Some(false)),
        (Classification::Application, Some(false), Some(true)),
        (Classification::Application, Some(false), None),
        (Classification::Unknown, Some(false), Some(false)),
        (Classification::System, Some(false), Some(false)),
        (Classification::Utility, Some(false), Some(false)),
    ] {
        let mut s = snapshot();
        let w = &mut s.state.windows[0];
        w.classification = classification;
        w.exclude_from_capture = protection;
        w.skip_taskbar = skip;
        let map = project(&s, a().epoch);
        assert_eq!(
            policy::decide(
                &a(),
                a().epoch,
                true,
                &map,
                &BTreeSet::from([alpha()]),
                &[claim(a(), Source::User, &[alpha()])]
            )
            .reason,
            Reason::WindowOutsidePolicyScope
        );
    }
}
struct TestSource(
    std::cell::RefCell<Option<ObservedSnapshot>>,
    tokio::sync::watch::Sender<u64>,
);
impl ObservedStateSource for TestSource {
    fn snapshot(&self) -> Result<ObservedSnapshot, ObservationError> {
        self.0.borrow().clone().ok_or(ObservationError::Unavailable(
            ObservationHealth::Disconnected,
        ))
    }
    fn health(&self) -> ObservationHealth {
        if self.0.borrow().is_some() {
            ObservationHealth::Synchronized
        } else {
            ObservationHealth::Disconnected
        }
    }
    fn subscribe(&self) -> tokio::sync::watch::Receiver<u64> {
        self.1.subscribe()
    }
}
fn application() -> (
    tempfile::TempDir,
    Application<SqliteStore, TestSource>,
    WorkspaceId,
    WorkspaceId,
) {
    let temp = tempfile::tempdir().unwrap();
    let source = TestSource(
        std::cell::RefCell::new(Some(snapshot())),
        tokio::sync::watch::channel(0).0,
    );
    let mut app = Application::new(SqliteStore::open(&temp.path().join("db")).unwrap(), source);
    let world = app.logical_mut().create_world("World", None).unwrap();
    let alpha = app
        .logical_mut()
        .create_workspace(world, "Alpha", None, None)
        .unwrap();
    let beta = app
        .logical_mut()
        .create_workspace(world, "Beta", None, None)
        .unwrap();
    (temp, app, alpha, beta)
}
#[test]
fn coordinator_internal_verified_claim_boundary_and_overrides() {
    let (_temp, app, alpha, beta) = application();
    let mut c = ContextCoordinator::new();
    let r = c.report(&app, Selection::All).unwrap();
    let key = r.associations[0].window.clone();
    c.accept_prevalidated(
        &app,
        vec![claim(key.clone(), Source::VerifiedLaunch, &[alpha])],
        &r.version,
    )
    .unwrap();
    let current = c.inspect(&app, &key).unwrap();
    assert_eq!(current.decision.reason, Reason::VerifiedWindowLaunch);
    let assigned = c
        .assign(&app, &key, BTreeSet::from([beta]), &current.version)
        .unwrap();
    assert_eq!(assigned.decision.targets, [beta]);
    let suppressed = c.unassign(&app, &key, &assigned.version).unwrap();
    assert_eq!(suppressed.decision.reason, Reason::UserUnassigned);
    let cleared = c.clear_override(&app, &key, &suppressed.version).unwrap();
    assert_eq!(cleared.decision.targets, [alpha]);
    assert!(matches!(
        c.accept_prevalidated(&app, vec![], &r.version),
        Err(ContextError::StaleOperation)
    ));
}
#[test]
fn coordinator_rename_move_delete_and_stale_compare_and_set() {
    let (_temp, mut app, alpha, _) = application();
    let mut c = ContextCoordinator::new();
    let r = c.report(&app, Selection::All).unwrap();
    let key = r.associations[0].window.clone();
    let assigned = c
        .assign(&app, &key, BTreeSet::from([alpha]), &r.version)
        .unwrap();
    app.logical_mut()
        .rename_workspace(alpha, "Renamed")
        .unwrap();
    assert!(matches!(
        c.unassign(&app, &key, &assigned.version),
        Err(ContextError::StaleOperation)
    ));
    assert_eq!(c.inspect(&app, &key).unwrap().decision.targets, [alpha]);
    let world = app.logical_mut().create_world("Other", None).unwrap();
    app.logical_mut().move_workspace(alpha, world).unwrap();
    assert_eq!(c.inspect(&app, &key).unwrap().decision.targets, [alpha]);
    app.logical_mut().delete_workspace(alpha).unwrap();
    assert_eq!(
        c.inspect(&app, &key).unwrap().decision.reason,
        Reason::WorkspaceRemoved
    );
}
#[test]
fn coordinator_epochs_health_and_position_cas_are_conservative() {
    let (_temp, app, alpha, _) = application();
    let mut c = ContextCoordinator::new();
    let r = c.report(&app, Selection::All).unwrap();
    let key = r.associations[0].window.clone();
    let assigned = c
        .assign(&app, &key, BTreeSet::from([alpha]), &r.version)
        .unwrap();
    app.observations().0.borrow_mut().as_mut().unwrap().sequence += 1;
    assert!(matches!(
        c.unassign(&app, &key, &assigned.version),
        Err(ContextError::StaleOperation)
    ));
    assert_eq!(
        c.inspect(&app, &key).unwrap().decision.status,
        Status::Confirmed
    );
    let saved = app.observations().0.borrow_mut().take().unwrap();
    assert!(c.report(&app, Selection::All).is_err());
    *app.observations().0.borrow_mut() = Some(saved);
    assert_eq!(
        c.report(&app, Selection::All).unwrap().associations[0]
            .decision
            .status,
        Status::Unknown
    );
    assert_eq!(
        c.inspect(&app, &key).unwrap().decision.status,
        Status::Stale
    );
}
#[test]
fn coordinator_parent_hint_never_confirms_and_competes_with_heuristics() {
    let (_temp, app, alpha, beta) = application();
    {
        let mut value = app.observations().0.borrow_mut();
        let s = value.as_mut().unwrap();
        let mut child = s.state.windows[0].clone();
        child.runtime_id = "window-2".into();
        child.classification = Classification::Dialog;
        child.parent_runtime_id = Some("window-1".into());
        s.state.windows.push(child);
        s.window_generations
            .insert("window-2".into(), "generation-2".into());
    }
    let mut c = ContextCoordinator::new();
    let r = c.report(&app, Selection::All).unwrap();
    let parent = r.associations[0].window.clone();
    let child = r.associations[1].window.clone();
    let assigned = c
        .assign(&app, &parent, BTreeSet::from([alpha]), &r.version)
        .unwrap();
    let suggested = c.inspect(&app, &child).unwrap();
    assert_eq!(suggested.decision.status, Status::Suggested);
    assert!(suggested.decision.effective_targets().is_empty());
    let conflict = c.suggest(&app, &child, beta, &assigned.version).unwrap();
    assert_eq!(conflict.decision.status, Status::Conflicted);
    assert_eq!(
        c.report(&app, Selection::Conflicted)
            .unwrap()
            .associations
            .len(),
        1
    );
}
