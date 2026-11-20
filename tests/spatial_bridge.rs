#[path = "support/private_bus.rs"]
mod private_bus;
use depthos_core::{
    controller::{Controller, ControllerOptions, LogicalCommand, LogicalReply},
    spatial::{INTERFACE, PATH, SERVICE, UiBridge},
    sqlite::SqliteStore,
};
use futures_lite::StreamExt;
use private_bus::PrivateBus;
use serde_json::{Value, json};
use std::time::Duration;
use zbus::{Connection, Proxy};
async fn client(bus: &PrivateBus) -> Connection {
    zbus::connection::Builder::address(bus.address.as_str())
        .unwrap()
        .build()
        .await
        .unwrap()
}
async fn call(p: &Proxy<'_>, method: &str) -> Value {
    let raw: String = p.call(method, &()).await.unwrap();
    serde_json::from_str(&raw).unwrap()
}
async fn page(p: &Proxy<'_>, method: &str, cursor: u32, revision: &str) -> Value {
    let raw: String = p.call(method, &(cursor, revision)).await.unwrap();
    assert!(raw.len() <= 131072);
    serde_json::from_str(&raw).unwrap()
}
fn controller(bus: &PrivateBus, path: &std::path::Path) -> Controller {
    Controller::start(
        SqliteStore::open(path).unwrap(),
        ControllerOptions {
            bus_address: Some(bus.address.clone()),
            ..Default::default()
        },
    )
    .unwrap()
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schema_paging_invalidation_limits_conflict_and_shutdown() {
    let bus = PrivateBus::start();
    let tmp = tempfile::tempdir().unwrap();
    let c = controller(&bus, &tmp.path().join("db"));
    let bridge = UiBridge::at_address(c.clone(), &bus.address).await.unwrap();
    assert!(UiBridge::at_address(c.clone(), &bus.address).await.is_err());
    assert!(
        UiBridge::at_address(c.clone(), "tcp:host=localhost,port=3")
            .await
            .is_err()
    );
    let conn = client(&bus).await;
    let p = Proxy::new(&conn, SERVICE, PATH, INTERFACE).await.unwrap();
    let cap = call(&p, "GetCapabilities").await;
    assert_eq!(cap["schemaVersion"], 1);
    assert_eq!(cap["data"]["readOnly"], true);
    assert_eq!(cap["data"]["assignmentMutations"], false);
    assert_eq!(cap["data"]["selectionOwner"], "client");
    assert_eq!(
        call(&p, "GetCurrentLogicalContext").await["data"]["workspaceId"],
        Value::Null
    );
    assert_eq!(
        page(&p, "GetHierarchy", 0, "").await["data"]["items"],
        json!([])
    );
    assert_eq!(
        page(&p, "GetContextReport", 0, "").await["available"],
        false
    );
    let mut signal = p.receive_signal("Invalidated").await.unwrap();
    for i in 0..70 {
        c.logical(LogicalCommand::CreateWorld(
            format!("World {i}"),
            Some("SECRET DESCRIPTION".into()),
        ))
        .await
        .unwrap();
    }
    tokio::time::timeout(Duration::from_secs(2), signal.next())
        .await
        .unwrap()
        .unwrap();
    let first = page(&p, "GetHierarchy", 0, "").await;
    assert_eq!(first["data"]["items"].as_array().unwrap().len(), 64);
    assert_eq!(first["data"]["nextCursor"], 64);
    assert!(!first.to_string().contains("SECRET"));
    let rev = first["revision"].as_str().unwrap();
    let second = page(&p, "GetHierarchy", 64, rev).await;
    assert_eq!(second["data"]["items"].as_array().unwrap().len(), 6);
    c.logical(LogicalCommand::CreateWorld("New".into(), None))
        .await
        .unwrap();
    assert!(
        p.call::<_, _, String>("GetHierarchy", &(64_u32, rev))
            .await
            .is_err()
    );
    for (cursor, revision) in [(u32::MAX, ""), (0, "stale"), (0, &"x".repeat(65))] {
        assert!(
            p.call::<_, _, String>("GetHierarchy", &(cursor, revision))
                .await
                .is_err()
        );
    }
    assert!(p.call::<_, _, String>("Assign", &()).await.is_err());
    let xml = zbus::fdo::IntrospectableProxy::builder(&conn)
        .destination(SERVICE)
        .unwrap()
        .path(PATH)
        .unwrap()
        .build()
        .await
        .unwrap()
        .introspect()
        .await
        .unwrap();
    let (interface, _) = xml
        .split_once(&format!("<interface name=\"{INTERFACE}\">"))
        .unwrap()
        .1
        .split_once("</interface>")
        .unwrap();
    let mut methods: Vec<_> = interface
        .split("<method name=\"")
        .skip(1)
        .map(|element| element.split_once('"').unwrap().0)
        .collect();
    methods.sort_unstable();
    assert_eq!(
        methods,
        [
            "GetCapabilities",
            "GetContextReport",
            "GetControllerHealth",
            "GetCurrentLogicalContext",
            "GetDesktopShelves",
            "GetHierarchy",
        ]
    );
    assert!(!interface.contains("<property"));
    let wrong = Proxy::new(&conn, SERVICE, PATH, "org.depthos.Wrong")
        .await
        .unwrap();
    assert!(
        wrong
            .call::<_, _, String>("GetCapabilities", &())
            .await
            .is_err()
    );
    let instance = cap["instanceId"].clone();
    bridge.shutdown().await.unwrap();
    assert!(call_owner(&conn).await.is_err());
    let bridge = UiBridge::at_address(c.clone(), &bus.address).await.unwrap();
    assert_ne!(call(&p, "GetCapabilities").await["instanceId"], instance);
    bridge.shutdown().await.unwrap();
    c.shutdown().await.unwrap();
}
async fn call_owner(c: &Connection) -> zbus::fdo::Result<zbus::names::OwnedUniqueName> {
    zbus::fdo::DBusProxy::new(c)
        .await
        .unwrap()
        .get_name_owner(SERVICE.try_into().unwrap())
        .await
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redacted_native_context_stale_assignments_and_owner_loss() {
    let bus = PrivateBus::start();
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("db");
    let c = controller(&bus, &db);
    let bridge = UiBridge::at_address(c.clone(), &bus.address).await.unwrap();
    let conn = client(&bus).await;
    let p = Proxy::new(&conn, SERVICE, PATH, INTERFACE).await.unwrap();
    let publisher = bus.publisher().await;
    let observer = Proxy::new(
        &publisher,
        depthos_observer::SERVICE,
        depthos_observer::PATH,
        depthos_observer::INTERFACE,
    )
    .await
    .unwrap();
    // Receiver acquisition is asynchronous.
    let hello = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(raw) = observer
                .call::<_, _, String>("Hello", &("test-session",))
                .await
            {
                break serde_json::from_str::<Value>(&raw).unwrap();
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let frame: Value = serde_json::from_str::<Value>(include_str!("observer-adapter/trace.json"))
        .unwrap()["frames"][0]
        .clone();
    let packet = json!({"schemaVersion":1,"sessionId":"test-session","consumerId":hello["consumerId"],"position":{"sequence":1,"revision":1},"frames":[frame]});
    let _: String = observer.call("Push", &(packet.to_string(),)).await.unwrap();
    let report = page(&p, "GetContextReport", 0, "").await;
    assert_eq!(report["available"], true);
    let raw = report.to_string();
    for secret in [
        "Sanitized editor",
        "org.example.Editor",
        "resourceClass",
        "title",
        "pid",
    ] {
        assert!(!raw.contains(secret), "{secret}");
    }
    assert_eq!(report["data"]["items"][0]["status"], "unknown");
    let LogicalReply::WorldCreated(w) = c
        .logical(LogicalCommand::CreateWorld("W".into(), None))
        .await
        .unwrap()
    else {
        panic!()
    };
    let LogicalReply::WorkspaceCreated(target) = c
        .logical(LogicalCommand::CreateWorkspace(
            w,
            "Target".into(),
            None,
            None,
        ))
        .await
        .unwrap()
    else {
        panic!()
    };
    let r = c
        .associations(depthos_core::context::Selection::All)
        .await
        .unwrap();
    c.assign(
        r.associations[0].window.clone(),
        [target].into_iter().collect(),
        r.version,
    )
    .await
    .unwrap();
    assert_eq!(
        page(&p, "GetContextReport", 0, "").await["data"]["items"][0]["status"],
        "confirmed"
    );
    use depthos_core::associations::RuleKind;
    let shelves = page(&p, "GetDesktopShelves", 0, "").await;
    assert_eq!(shelves["data"]["freshness"], "live");
    assert_eq!(shelves["data"]["titlePolicy"], "withheld");
    let entry = &shelves["data"]["items"][0];
    assert_eq!(entry["workspaceId"], json!(target));
    assert!(entry["desktopId"].is_null());
    let window_id = entry["id"].clone();
    let LogicalReply::DesktopCreated(desktop) = c
        .logical(LogicalCommand::CreateDesktop(
            target,
            "Logical".into(),
            None,
        ))
        .await
        .unwrap()
    else {
        panic!("logical desktop")
    };
    SqliteStore::configure_association(
        &db,
        RuleKind::Application,
        "org.example.Editor",
        Some((target, Some(desktop))),
    )
    .unwrap();
    let shelves = page(&p, "GetDesktopShelves", 0, "").await;
    let entry = &shelves["data"]["items"][0];
    assert_eq!(entry["desktopId"], json!(desktop));
    assert_eq!(entry["assignment"], "explicit");
    assert_eq!(entry["id"], window_id);
    assert_eq!(entry.as_object().unwrap().len(), 6);
    assert!(!shelves.to_string().contains("Sanitized editor"));
    assert!(!shelves.to_string().contains("test-session"));
    c.logical(LogicalCommand::DeleteDesktop(desktop))
        .await
        .unwrap();
    assert!(
        p.call::<_, _, String>(
            "GetDesktopShelves",
            &(0_u32, shelves["revision"].as_str().unwrap())
        )
        .await
        .is_err()
    );
    assert!(page(&p, "GetDesktopShelves", 0, "").await["data"]["items"][0]["desktopId"].is_null());
    c.logical(LogicalCommand::DeleteWorkspace(target))
        .await
        .unwrap();
    assert_ne!(
        page(&p, "GetContextReport", 0, "").await["data"]["items"][0]["status"],
        "confirmed"
    );
    // An ineligible duplicate is still an ambiguous native identity. A warm
    // context cache must revoke the previous UUID mapping when it appears.
    assert_eq!(
        page(&p, "GetDesktopShelves", 0, "").await["data"]["items"],
        json!([])
    );
    let current = page(&p, "GetContextReport", 0, "").await;
    let old_revision = current["revision"].as_str().unwrap();
    let old_item = &current["data"]["items"][0];
    assert!(old_item["nativeId"].is_string());

    let mut duplicate = packet.clone();
    let frame = &mut duplicate["frames"][0];
    frame["sequence"] = json!(2);
    let windows = frame["state"]["windows"].as_array_mut().unwrap();
    let mut protected = windows[0].clone();
    protected["runtimeId"] = json!("protected-duplicate");
    protected["excludeFromCapture"] = json!(true);
    windows.push(protected);
    duplicate["position"]["sequence"] = json!(2);
    let _: String = observer
        .call("Push", &(duplicate.to_string(),))
        .await
        .unwrap();

    assert!(
        p.call::<_, _, String>("GetContextReport", &(0_u32, old_revision))
            .await
            .is_err()
    );
    let ambiguous = page(&p, "GetContextReport", 0, "").await;
    let items = ambiguous["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["window"], old_item["window"]);
    assert_eq!(items[0]["nativeId"], Value::Null);

    publisher.release_name("org.kde.KWin").await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if page(&p, "GetContextReport", 0, "").await["available"] == false {
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(page(&p, "GetHierarchy", 0, "").await["available"], true);
    assert_eq!(
        call(&p, "GetControllerHealth").await["data"]["health"]["lifecycle"],
        "Recovering"
    );
    bridge.shutdown().await.unwrap();
    c.shutdown().await.unwrap();
}
