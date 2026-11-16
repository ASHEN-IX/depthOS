#[path = "support/private_bus.rs"]
mod private_bus;
use depthos_core::{
    application::Application,
    context::policy::{Reason, Status},
    context::{ContextCoordinator, ContextError, Selection},
    observed::ObservedStateSource,
    sqlite::SqliteStore,
};
use depthos_observer::{INTERFACE, PATH, Receiver, SERVICE};
use private_bus::PrivateBus;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use zbus::{Connection, Proxy};
fn trace() -> Value {
    serde_json::from_str(include_str!("observer-adapter/trace.json")).unwrap()
}
async fn endpoint(c: &Connection) -> Proxy<'_> {
    Proxy::new(c, SERVICE, PATH, INTERFACE).await.unwrap()
}
async fn hello(c: &Connection, session: &str) -> String {
    let answer: String = endpoint(c).await.call("Hello", &(session,)).await.unwrap();
    serde_json::from_str::<Value>(&answer).unwrap()["consumerId"]
        .as_str()
        .unwrap()
        .into()
}
async fn push(c: &Connection, consumer: &str, frame: Value) -> bool {
    let packet = json!({"schemaVersion":1,"consumerId":consumer,"sessionId":frame["sessionId"],"position":{"sequence":frame["sequence"],"revision":frame["revision"]},"frames":[frame]});
    let answer: String = endpoint(c)
        .await
        .call("Push", &(packet.to_string(),))
        .await
        .unwrap();
    serde_json::from_str::<Value>(&answer).unwrap()["needSnapshot"]
        .as_bool()
        .unwrap()
}
fn two_windows(session: &str) -> Value {
    let mut f = trace()["frames"][0].clone();
    f["sessionId"] = json!(session);
    let mut second = f["state"]["windows"][0].clone();
    second["runtimeId"] = json!("window-2");
    second["nativeId"] = Value::Null;
    second["applicationId"] = f["state"]["windows"][0]["applicationId"].clone();
    f["state"]["windows"].as_array_mut().unwrap().push(second);
    f["state"]["applicationWindowIds"] = json!(["window-1", "window-2"]);
    f
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_projects_same_app_disconnect_replacement_and_no_database_writes() {
    let bus = PrivateBus::start();
    let publisher = bus.publisher().await;
    let receiver = Receiver::at_address(&bus.address).await.unwrap();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("hierarchy.db");
    let mut app = Application::new(SqliteStore::open(&path).unwrap(), receiver.observer());
    let world = app.logical_mut().create_world("Projects", None).unwrap();
    let alpha = app
        .logical_mut()
        .create_workspace(world, "Alpha", None, None)
        .unwrap();
    let beta = app
        .logical_mut()
        .create_workspace(world, "Beta", None, None)
        .unwrap();
    let hierarchy = app.hierarchy().unwrap();
    let baseline = std::fs::read(&path).unwrap();
    let consumer = hello(&publisher, "test-session").await;
    assert!(!push(&publisher, &consumer, two_windows("test-session")).await);
    let mut coordinator = ContextCoordinator::new();
    let current = coordinator.report(&app, Selection::All).unwrap();
    assert_eq!(current.associations.len(), 2);
    let a = current.associations[0].window.clone();
    let b = current.associations[1].window.clone();
    let first = coordinator
        .assign(&app, &a, BTreeSet::from([alpha]), &current.version)
        .unwrap();
    assert!(matches!(
        coordinator.assign(&app, &b, BTreeSet::from([beta]), &current.version),
        Err(ContextError::StaleOperation)
    ));
    let second = coordinator
        .assign(&app, &b, BTreeSet::from([beta]), &first.version)
        .unwrap();
    assert_eq!(second.decision.targets, [beta]);
    assert_eq!(
        coordinator.inspect(&app, &a).unwrap().decision.targets,
        [alpha]
    );
    let output = serde_json::to_string(&coordinator.report(&app, Selection::All).unwrap()).unwrap();
    assert!(!output.contains("Sanitized"));
    assert!(!output.contains("applicationId"));
    assert!(!output.contains("nativeId"));
    let mut unknown = two_windows("test-session");
    unknown["sequence"] = json!(2);
    unknown["state"]["windows"][0]["applicationId"] = Value::Null;
    assert!(!push(&publisher, &consumer, unknown).await);
    assert_eq!(
        coordinator.inspect(&app, &a).unwrap().decision.status,
        Status::Confirmed
    );
    receiver.shutdown().await.unwrap();
    assert!(matches!(
        coordinator.report(&app, Selection::All),
        Err(ContextError::Unavailable(_))
    ));
    assert_eq!(app.hierarchy().unwrap(), hierarchy);
    assert_eq!(std::fs::read(&path).unwrap(), baseline);
    let replacement = Receiver::at_address(&bus.address).await.unwrap();
    let app = Application::new(SqliteStore::open(&path).unwrap(), replacement.observer());
    let consumer = hello(&publisher, "replacement-session").await;
    assert!(!push(&publisher, &consumer, two_windows("replacement-session")).await);
    let fresh = coordinator.report(&app, Selection::All).unwrap();
    assert!(
        fresh
            .associations
            .iter()
            .all(|a| a.decision.status == Status::Unknown)
    );
    assert_eq!(
        coordinator.inspect(&app, &a).unwrap().decision.status,
        Status::Stale
    );
    assert!(
        coordinator
            .assign(
                &app,
                &fresh.associations[0].window,
                BTreeSet::from([alpha]),
                &second.version
            )
            .is_err()
    );
    assert_eq!(app.hierarchy().unwrap(), hierarchy);
    assert_eq!(std::fs::read(&path).unwrap(), baseline);
    replacement.shutdown().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unseen_gap_recovery_and_same_token_hello_cannot_restore_overrides() {
    let bus = PrivateBus::start();
    let publisher = bus.publisher().await;
    let receiver = Receiver::at_address(&bus.address).await.unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut app = Application::new(
        SqliteStore::open(&temp.path().join("db")).unwrap(),
        receiver.observer(),
    );
    let world = app.logical_mut().create_world("World", None).unwrap();
    let target = app
        .logical_mut()
        .create_workspace(world, "Target", None, None)
        .unwrap();
    let mut coordinator = ContextCoordinator::new();
    for field in ["sequence", "revision"] {
        let consumer = hello(&publisher, "test-session").await;
        assert!(!push(&publisher, &consumer, trace()["frames"][0].clone()).await);
        let view = coordinator.report(&app, Selection::All).unwrap();
        let old = view.associations[0].window.clone();
        coordinator
            .assign(&app, &old, BTreeSet::from([target]), &view.version)
            .unwrap();
        let mut gap = trace()["frames"][1].clone();
        gap[field] = json!(100);
        assert!(push(&publisher, &consumer, gap).await);
        // Deliberately do NOT poll coordinator while unhealthy: watch may coalesce.
        let mut baseline = trace()["frames"][0].clone();
        baseline["sequence"] = json!(101);
        assert!(!push(&publisher, &consumer, baseline).await);
        let recovered = coordinator.report(&app, Selection::All).unwrap();
        assert_eq!(recovered.associations[0].decision.status, Status::Unknown);
        assert_ne!(recovered.version.epoch, old.epoch);
    }
    let before = coordinator.report(&app, Selection::All).unwrap();
    let key = before.associations[0].window.clone();
    coordinator
        .assign(&app, &key, BTreeSet::from([target]), &before.version)
        .unwrap();
    let consumer = hello(&publisher, "test-session").await;
    assert!(!push(&publisher, &consumer, trace()["frames"][0].clone()).await);
    assert_eq!(
        coordinator
            .report(&app, Selection::All)
            .unwrap()
            .associations[0]
            .decision
            .status,
        Status::Unknown
    );
    receiver.shutdown().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn coalesced_runtime_id_reuse_has_new_generation_while_other_assignments_survive() {
    let bus = PrivateBus::start();
    let publisher = bus.publisher().await;
    let receiver = Receiver::at_address(&bus.address).await.unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut app = Application::new(
        SqliteStore::open(&temp.path().join("db")).unwrap(),
        receiver.observer(),
    );
    let world = app.logical_mut().create_world("World", None).unwrap();
    let target = app
        .logical_mut()
        .create_workspace(world, "Target", None, None)
        .unwrap();
    let consumer = hello(&publisher, "test-session").await;
    let initial = two_windows("test-session");
    assert!(!push(&publisher, &consumer, initial.clone()).await);
    let mut coordinator = ContextCoordinator::new();
    let r = coordinator.report(&app, Selection::All).unwrap();
    let a = r.associations[0].window.clone();
    let b = r.associations[1].window.clone();
    let assigned = coordinator
        .assign(&app, &a, BTreeSet::from([target]), &r.version)
        .unwrap();
    coordinator
        .assign(&app, &b, BTreeSet::from([target]), &assigned.version)
        .unwrap();
    let removed = json!({"schemaVersion":1,"sessionId":"test-session","sequence":2,"revision":2,"type":"window.removed","windowId":"window-1"});
    assert!(!push(&publisher, &consumer, removed).await);
    let added = json!({"schemaVersion":1,"sessionId":"test-session","sequence":3,"revision":3,"type":"window.added","windowId":"window-1","window":initial["state"]["windows"][0]});
    assert!(!push(&publisher, &consumer, added).await);
    // Both changes happened before the coordinator read. Runtime reuse is still caught.
    assert_eq!(
        coordinator.inspect(&app, &a).unwrap().decision.status,
        Status::Stale
    );
    assert_eq!(
        coordinator.inspect(&app, &b).unwrap().decision.status,
        Status::Confirmed
    );
    assert_eq!(
        coordinator
            .report(&app, Selection::Unassigned)
            .unwrap()
            .associations
            .len(),
        1
    );
    assert!(
        coordinator
            .report(&app, Selection::All)
            .unwrap()
            .associations
            .iter()
            .any(|r| r.window.runtime_id == "window-1"
                && r.decision.reason == Reason::NoApplicableEvidence)
    );
    assert!(app.observations().snapshot().is_ok());
    receiver.shutdown().await.unwrap();
}
