#[path = "support/private_bus.rs"]
mod private_bus;
use depthos_core::{
    application::Application,
    observed::{ObservationHealth as H, ObservedStateSource},
    sqlite::SqliteStore,
};
use depthos_observer::{INTERFACE, PATH, Receiver, SERVICE};
use private_bus::PrivateBus;
use serde_json::{Value, json};
use zbus::{Connection, Proxy};
fn trace() -> Value {
    serde_json::from_str(include_str!("observer-adapter/trace.json")).unwrap()
}
async fn endpoint(c: &Connection) -> Proxy<'_> {
    Proxy::new(c, SERVICE, PATH, INTERFACE).await.unwrap()
}
async fn hello(c: &Connection, s: &str) -> String {
    let h: String = endpoint(c).await.call("Hello", &(s,)).await.unwrap();
    serde_json::from_str::<Value>(&h).unwrap()["consumerId"]
        .as_str()
        .unwrap()
        .into()
}
async fn push(c: &Connection, id: &str, frame: Value) -> bool {
    let raw=json!({"schemaVersion":1,"sessionId":frame["sessionId"],"consumerId":id,"position":{"sequence":frame["sequence"],"revision":frame["revision"]},"frames":[frame]}).to_string();
    let answer: String = endpoint(c).await.call("Push", &(raw,)).await.unwrap();
    serde_json::from_str::<Value>(&answer).unwrap()["needSnapshot"]
        .as_bool()
        .unwrap()
}
#[test]
fn disconnected_observations_do_not_affect_logical_crud() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(&tmp.path().join("workspaces.db")).unwrap();
    let mut app = Application::disconnected(store);
    assert_eq!(app.observations().health(), H::Disconnected);
    assert!(app.observations().snapshot().is_err());
    let world = app.logical_mut().create_world("Offline", None).unwrap();
    let workspace = app
        .logical_mut()
        .create_workspace(world, "Code", None, None)
        .unwrap();
    app.logical_mut()
        .create_desktop(workspace, "Edit", None)
        .unwrap();
    assert_eq!(
        app.hierarchy().unwrap().worlds[0].workspaces[0]
            .desktops
            .len(),
        1
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shared_application_reads_empty_and_populated_hierarchy_without_persisting_events() {
    let bus = PrivateBus::start();
    let publisher = bus.publisher().await;
    let receiver = Receiver::at_address(&bus.address).await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("workspaces.db");
    let mut app = Application::new(SqliteStore::open(&db).unwrap(), receiver.observer());
    let mut updates = app.observations().subscribe();
    let empty_bytes = std::fs::read(&db).unwrap();
    let id = hello(&publisher, "test-session").await;
    let t = trace();
    assert!(!push(&publisher, &id, t["frames"][0].clone()).await);
    assert!(app.hierarchy().unwrap().worlds.is_empty());
    assert!(app.observations().snapshot().is_ok());
    assert_eq!(empty_bytes, std::fs::read(&db).unwrap());
    tokio::time::timeout(std::time::Duration::from_secs(2), updates.changed())
        .await
        .unwrap()
        .unwrap();
    let world = app.logical_mut().create_world("Local", None).unwrap();
    let workspace = app
        .logical_mut()
        .create_workspace(world, "Work", None, None)
        .unwrap();
    let desktop = app
        .logical_mut()
        .create_desktop(workspace, "Desk", None)
        .unwrap();
    let before = std::fs::read(&db).unwrap();
    let hierarchy = app.hierarchy().unwrap();
    for frame in t["frames"].as_array().unwrap().iter().skip(1) {
        assert!(!push(&publisher, &id, frame.clone()).await);
    }
    assert_eq!(hierarchy, app.hierarchy().unwrap());
    assert_eq!(before, std::fs::read(&db).unwrap());
    let snapshot = app.observations().snapshot().unwrap();
    assert_eq!(snapshot.state.windows[0].runtime_id, "window-2");
    assert!(snapshot.state.windows[0].application_id.is_none());
    assert!(snapshot.state.windows[0].title.is_none());
    let conn = rusqlite::Connection::open(&db).unwrap();
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(
        tables,
        [
            "desktop_association_rules",
            "desktops",
            "operation_audit",
            "preferences",
            "workspaces",
            "worlds"
        ]
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM desktop_association_rules", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap(),
        0
    );
    for (table, id) in [
        ("worlds", world.0),
        ("workspaces", workspace.0),
        ("desktops", desktop.0),
    ] {
        let stored: Vec<u8> = conn
            .query_row(&format!("SELECT id FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(stored, id.as_bytes());
    }
    receiver.shutdown().await.unwrap();
    assert_eq!(app.observations().health(), H::Disconnected);
    assert!(app.observations().snapshot().is_err());
    app.logical_mut()
        .rename_world(world, "Still local")
        .unwrap();
    assert_eq!(app.hierarchy().unwrap().worlds[0].world.name, "Still local");
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn application_preserves_session_scope_and_gap_recovery() {
    let bus = PrivateBus::start();
    let publisher = bus.publisher().await;
    let receiver = Receiver::at_address(&bus.address).await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let app = Application::new(
        SqliteStore::open(&tmp.path().join("db")).unwrap(),
        receiver.observer(),
    );
    let t = trace();
    let consumer = hello(&publisher, "test-session").await;
    assert!(!push(&publisher, &consumer, t["frames"][0].clone()).await);
    let old = app
        .observations()
        .snapshot()
        .unwrap()
        .window_identity("window-1")
        .unwrap();
    for field in ["sequence", "revision"] {
        let consumer = hello(&publisher, "test-session").await;
        assert!(!push(&publisher, &consumer, t["frames"][0].clone()).await);
        let mut gap = t["frames"][1].clone();
        gap[field] = json!(gap[field].as_u64().unwrap() + 5);
        assert!(push(&publisher, &consumer, gap).await);
        assert_eq!(app.observations().health(), H::Desynchronized);
        assert!(app.observations().snapshot().is_err());
        assert!(!push(&publisher, &consumer, t["final"].clone()).await);
    }
    let consumer = hello(&publisher, "new-session").await;
    assert!(app.observations().snapshot().is_err());
    let mut replacement = t["frames"][0].clone();
    replacement["sessionId"] = json!("new-session");
    assert!(!push(&publisher, &consumer, replacement).await);
    let new = app
        .observations()
        .snapshot()
        .unwrap()
        .window_identity("window-1")
        .unwrap();
    assert_ne!(old, new);
    assert!(app.hierarchy().unwrap().worlds.is_empty());
    receiver.shutdown().await.unwrap();
}
