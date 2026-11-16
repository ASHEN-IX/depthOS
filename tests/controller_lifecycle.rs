#[path = "support/private_bus.rs"]
mod private_bus;
use depthos_core::{
    context::policy::Status,
    context::{ContextError, Selection},
    controller::{
        Controller, ControllerHealth, ControllerOptions, Lifecycle, LogicalCommand, LogicalReply,
        RecoveryReason,
    },
    sqlite::SqliteStore,
};
use depthos_observer::{INTERFACE, PATH, Receiver, SERVICE};
use private_bus::PrivateBus;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
use zbus::{Connection, Proxy};
fn trace() -> Value {
    serde_json::from_str(include_str!("observer-adapter/trace.json")).unwrap()
}
fn options(bus: &PrivateBus) -> ControllerOptions {
    ControllerOptions {
        bus_address: Some(bus.address.clone()),
        ..Default::default()
    }
}
async fn wait(c: &Controller, predicate: impl Fn(&ControllerHealth) -> bool) -> ControllerHealth {
    let mut changes = c.subscribe();
    tokio::time::timeout(Duration::from_secs(9), async {
        loop {
            let status = c.health().await.unwrap();
            if predicate(&status) {
                return status;
            }
            changes.changed().await.unwrap();
        }
    })
    .await
    .unwrap()
}
async fn endpoint(c: &Connection) -> Proxy<'_> {
    Proxy::new(c, SERVICE, PATH, INTERFACE).await.unwrap()
}
async fn hello(c: &Connection, session: &str) -> String {
    let a: String = endpoint(c).await.call("Hello", &(session,)).await.unwrap();
    serde_json::from_str::<Value>(&a).unwrap()["consumerId"]
        .as_str()
        .unwrap()
        .into()
}
async fn push(c: &Connection, consumer: &str, frame: Value) -> bool {
    let p = json!({"schemaVersion":1,"sessionId":frame["sessionId"],"consumerId":consumer,"position":{"sequence":frame["sequence"],"revision":frame["revision"]},"frames":[frame]});
    let a: String = endpoint(c)
        .await
        .call("Push", &(p.to_string(),))
        .await
        .unwrap();
    serde_json::from_str::<Value>(&a).unwrap()["needSnapshot"]
        .as_bool()
        .unwrap()
}
async fn workspace(c: &Controller) -> depthos_core::domain::WorkspaceId {
    let LogicalReply::WorldCreated(world) = c
        .logical(LogicalCommand::CreateWorld("World".into(), None))
        .await
        .unwrap()
    else {
        panic!()
    };
    let LogicalReply::WorkspaceCreated(target) = c
        .logical(LogicalCommand::CreateWorkspace(
            world,
            "Target".into(),
            None,
            None,
        ))
        .await
        .unwrap()
    else {
        panic!()
    };
    target
}
fn initial() -> Value {
    trace()["frames"][0].clone()
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn waiting_sync_assign_healthy_updates_disconnect_replacement_and_crud() {
    let bus = PrivateBus::start();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    let c = Controller::start(SqliteStore::open(&path).unwrap(), options(&bus)).unwrap();
    wait(&c, |h| h.lifecycle == Lifecycle::WaitingForObserver).await;
    assert!(c.snapshot().await.is_err());
    assert!(c.associations(Selection::All).await.is_err());
    let target = workspace(&c).await;
    let hierarchy = c.hierarchy().await.unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let publisher = bus.publisher().await;
    let consumer = hello(&publisher, "test-session").await;
    assert_ne!(c.health().await.unwrap().lifecycle, Lifecycle::Synchronized);
    assert!(!push(&publisher, &consumer, initial()).await);
    wait(&c, |h| h.lifecycle == Lifecycle::Synchronized).await;
    let r = c.associations(Selection::All).await.unwrap();
    let key = r.associations[0].window.clone();
    let assigned = c
        .assign(key.clone(), BTreeSet::from([target]), r.version.clone())
        .await
        .unwrap();
    assert_eq!(assigned.decision.status, Status::Confirmed);
    assert!(!push(&publisher, &consumer, trace()["frames"][1].clone()).await);
    assert!(matches!(
        c.suppress(key.clone(), assigned.version).await,
        Err(depthos_core::controller::ControllerError::Context(
            ContextError::StaleOperation
        ))
    ));
    let r = c.associations(Selection::All).await.unwrap();
    assert_eq!(r.associations[0].decision.status, Status::Confirmed);
    let suppressed = c.suppress(key.clone(), r.version).await.unwrap();
    assert!(suppressed.decision.effective_targets().is_empty());
    assert_eq!(
        c.clear_override(key.clone(), suppressed.version)
            .await
            .unwrap()
            .decision
            .status,
        Status::Unknown
    );
    let r = c.associations(Selection::All).await.unwrap();
    c.assign(key.clone(), BTreeSet::from([target]), r.version)
        .await
        .unwrap();
    publisher.release_name("org.kde.KWin").await.unwrap();
    wait(&c, |h| h.lifecycle == Lifecycle::Recovering).await;
    assert!(c.associations(Selection::All).await.is_err());
    assert!(c.snapshot().await.is_err());
    assert_eq!(c.hierarchy().await.unwrap(), hierarchy);
    c.logical(LogicalCommand::RenameWorkspace(
        target,
        "Renamed offline".into(),
    ))
    .await
    .unwrap();
    let new_publisher = bus.publisher().await;
    let consumer = hello(&new_publisher, "replacement").await;
    let mut frame = initial();
    frame["sessionId"] = json!("replacement");
    assert!(!push(&new_publisher, &consumer, frame).await);
    assert_eq!(
        c.associations(Selection::All).await.unwrap().associations[0]
            .decision
            .status,
        Status::Unknown
    );
    // Only the explicitly requested rename changed SQLite, never observation/context work.
    c.logical(LogicalCommand::RenameWorkspace(target, "Target".into()))
        .await
        .unwrap();
    let renamed_bytes = std::fs::read(&path).unwrap();
    assert_ne!(renamed_bytes, bytes);
    assert!(
        !push(&new_publisher, &consumer, {
            let mut f = trace()["frames"][1].clone();
            f["sessionId"] = json!("replacement");
            f
        })
        .await
    );
    assert_eq!(std::fs::read(&path).unwrap(), renamed_bytes);
    c.shutdown().await.unwrap();
    c.shutdown().await.unwrap();
    assert_eq!(c.last_health().lifecycle, Lifecycle::Stopped);
    assert!(c.hierarchy().await.is_err());
    let next = Receiver::at_address(&bus.address).await.unwrap();
    next.shutdown().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn heartbeat_loss_gaps_rehello_and_context_reconstruction_expire_assignments() {
    let bus = PrivateBus::start();
    let publisher = bus.publisher().await;
    let temp = tempfile::tempdir().unwrap();
    let c = Controller::start(
        SqliteStore::open(&temp.path().join("db")).unwrap(),
        options(&bus),
    )
    .unwrap();
    wait(&c, |h| h.lifecycle == Lifecycle::WaitingForObserver).await;
    let target = workspace(&c).await;
    let consumer = hello(&publisher, "test-session").await;
    assert!(!push(&publisher, &consumer, initial()).await);
    let r = c.associations(Selection::All).await.unwrap();
    let key = r.associations[0].window.clone();
    c.assign(key, BTreeSet::from([target]), r.version)
        .await
        .unwrap();
    wait(&c, |h| h.lifecycle == Lifecycle::Recovering).await;
    assert!(c.associations(Selection::All).await.is_err());
    for (n, field) in [(100, "sequence"), (200, "revision")] {
        let mut baseline = initial();
        baseline["sequence"] = json!(n);
        assert!(!push(&publisher, &consumer, baseline).await);
        let r = c.associations(Selection::All).await.unwrap();
        assert_eq!(r.associations[0].decision.status, Status::Unknown);
        c.assign(
            r.associations[0].window.clone(),
            BTreeSet::from([target]),
            r.version,
        )
        .await
        .unwrap();
        let mut gap = trace()["frames"][1].clone();
        gap["sequence"] = json!(n + 1);
        gap[field] = json!(n + 10);
        assert!(push(&publisher, &consumer, gap).await);
        assert!(c.associations(Selection::All).await.is_err());
    }
    let consumer = hello(&publisher, "test-session").await;
    assert!(!push(&publisher, &consumer, initial()).await);
    let r = c.associations(Selection::All).await.unwrap();
    c.assign(
        r.associations[0].window.clone(),
        BTreeSet::from([target]),
        r.version.clone(),
    )
    .await
    .unwrap();
    c.reconstruct_context().await.unwrap();
    let new = c.associations(Selection::All).await.unwrap();
    assert_ne!(new.version.epoch, r.version.epoch);
    assert_eq!(new.associations[0].decision.status, Status::Unknown);
    assert!(
        c.assign(
            new.associations[0].window.clone(),
            BTreeSet::from([target]),
            r.version
        )
        .await
        .is_err()
    );
    c.shutdown().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn coalesced_window_recreation_is_not_rebound_and_updates_do_not_write_sqlite() {
    let bus = PrivateBus::start();
    let publisher = bus.publisher().await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    let c = Controller::start(SqliteStore::open(&path).unwrap(), options(&bus)).unwrap();
    wait(&c, |h| h.lifecycle == Lifecycle::WaitingForObserver).await;
    let target = workspace(&c).await;
    let bytes = std::fs::read(&path).unwrap();
    let consumer = hello(&publisher, "test-session").await;
    assert!(!push(&publisher, &consumer, initial()).await);
    let r = c.associations(Selection::All).await.unwrap();
    let old = r.associations[0].window.clone();
    c.assign(old.clone(), BTreeSet::from([target]), r.version.clone())
        .await
        .unwrap();
    assert!(!push(&publisher,&consumer,json!({"schemaVersion":1,"sessionId":"test-session","sequence":2,"revision":2,"type":"window.removed","windowId":"window-1"})).await);
    assert!(!push(&publisher,&consumer,json!({"schemaVersion":1,"sessionId":"test-session","sequence":3,"revision":3,"type":"window.added","windowId":"window-1","window":initial()["state"]["windows"][0]})).await);
    let current = c.associations(Selection::All).await.unwrap();
    assert_ne!(old, current.associations[0].window);
    assert_eq!(current.associations[0].decision.status, Status::Unknown);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let private = serde_json::to_string(&c.health().await.unwrap()).unwrap();
    assert!(!private.contains("Sanitized"));
    assert!(!private.contains("title"));
    assert!(!private.contains("window-1"));
    c.shutdown().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn receiver_failure_bounded_retry_collision_and_logical_availability() {
    let temp = tempfile::tempdir().unwrap();
    let opts = ControllerOptions {
        bus_address: Some("unix:path=/nonexistent/depthos-controller".into()),
        max_receiver_attempts: 2,
        retry_delay: Duration::from_millis(10),
        ..Default::default()
    };
    let c = Controller::start(
        SqliteStore::open(&temp.path().join("unavailable")).unwrap(),
        opts,
    )
    .unwrap();
    let h = wait(&c, |h| h.reason == RecoveryReason::RetryExhausted).await;
    assert_eq!(h.receiver_attempts, 2);
    workspace(&c).await;
    c.retry_receiver().await.unwrap();
    wait(&c, |h| h.reason == RecoveryReason::RetryExhausted).await;
    c.shutdown().await.unwrap();
    let bus = PrivateBus::start();
    let existing = Receiver::at_address(&bus.address).await.unwrap();
    let c = Controller::start(
        SqliteStore::open(&temp.path().join("collision")).unwrap(),
        options(&bus),
    )
    .unwrap();
    let h = wait(&c, |h| h.reason == RecoveryReason::ReceiverConflict).await;
    assert_eq!(h.lifecycle, Lifecycle::Degraded);
    assert_eq!(h.receiver_attempts, 1);
    workspace(&c).await;
    c.shutdown().await.unwrap();
    assert!(!existing.is_closed());
    assert!(Receiver::at_address(&bus.address).await.is_err());
    existing.shutdown().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn database_failure_is_degraded_and_can_recover_without_restarting_observer() {
    let bus = PrivateBus::start();
    let publisher = bus.publisher().await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    let c = Controller::start(SqliteStore::open(&path).unwrap(), options(&bus)).unwrap();
    wait(&c, |h| h.lifecycle == Lifecycle::WaitingForObserver).await;
    let consumer = hello(&publisher, "test-session").await;
    assert!(!push(&publisher, &consumer, initial()).await);
    wait(&c, |h| h.lifecycle == Lifecycle::Synchronized).await;
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("ALTER TABLE worlds RENAME TO temporarily_unavailable")
        .unwrap();
    assert_eq!(
        c.health().await.unwrap().reason,
        RecoveryReason::DatabaseUnavailable
    );
    assert!(c.hierarchy().await.is_err());
    assert!(c.associations(Selection::All).await.is_err());
    assert!(c.snapshot().await.is_ok());
    db.execute_batch("ALTER TABLE temporarily_unavailable RENAME TO worlds")
        .unwrap();
    assert_eq!(c.health().await.unwrap().lifecycle, Lifecycle::Synchronized);
    workspace(&c).await;
    c.shutdown().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repeated_start_stop_closes_watch_and_releases_name_and_database() {
    let bus = PrivateBus::start();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    for _ in 0..4 {
        let c = Controller::start(SqliteStore::open(&path).unwrap(), options(&bus)).unwrap();
        let mut changes = c.subscribe();
        wait(&c, |h| h.lifecycle == Lifecycle::WaitingForObserver).await;
        c.shutdown().await.unwrap();
        tokio::time::timeout(Duration::from_secs(1), async {
            while changes.changed().await.is_ok() {}
        })
        .await
        .unwrap();
        assert_eq!(changes.borrow().lifecycle, Lifecycle::Stopped);
        let receiver = Receiver::at_address(&bus.address).await.unwrap();
        receiver.shutdown().await.unwrap();
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "explicit deterministic synthetic controller benchmark"]
async fn synthetic_1000_window_latency() {
    let bus = PrivateBus::start();
    let publisher = bus.publisher().await;
    let temp = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(&temp.path().join("db")).unwrap();
    let start = Instant::now();
    let c = Controller::start(store, options(&bus)).unwrap();
    wait(&c, |h| h.lifecycle == Lifecycle::WaitingForObserver).await;
    let startup = start.elapsed();
    let consumer = hello(&publisher, "test-session").await;
    let mut initial = initial();
    let template = initial["state"]["windows"][0].clone();
    initial["state"]["activeWindowId"] = Value::Null;
    initial["state"]["windows"] = json!(
        (0..1000)
            .map(|n| {
                let mut w = template.clone();
                w["runtimeId"] = json!(format!("window-{n}"));
                w
            })
            .collect::<Vec<_>>()
    );
    initial["state"]["applicationWindowIds"] =
        json!((0..1000).map(|n| format!("window-{n}")).collect::<Vec<_>>());
    let start = Instant::now();
    assert!(!push(&publisher, &consumer, initial.clone()).await);
    wait(&c, |h| h.lifecycle == Lifecycle::Synchronized).await;
    let sync = start.elapsed();
    let mut samples = Vec::new();
    for n in 0..100_u64 {
        let mut w = template.clone();
        let id = format!("window-{}", n % 1000);
        w["runtimeId"] = json!(id);
        w["geometry"]["x"] = json!(n);
        let frame = json!({"schemaVersion":1,"sessionId":"test-session","sequence":n+2,"revision":n+2,"type":"window.geometry.changed","windowId":id,"changedFields":["geometry"],"window":w});
        let start = Instant::now();
        assert!(!push(&publisher, &consumer, frame).await);
        let h = c.health().await.unwrap();
        assert_eq!(h.context.unwrap().observation.sequence, n + 2);
        samples.push(start.elapsed().as_secs_f64() * 1000.);
    }
    let mut gap = trace()["frames"][1].clone();
    gap["sequence"] = json!(1000);
    assert!(push(&publisher, &consumer, gap).await);
    assert!(c.associations(Selection::All).await.is_err());
    initial["sequence"] = json!(1001);
    initial["revision"] = json!(101);
    let start = Instant::now();
    assert!(!push(&publisher, &consumer, initial).await);
    wait(&c, |h| h.lifecycle == Lifecycle::Synchronized).await;
    let recovery = start.elapsed();
    samples.sort_by(f64::total_cmp);
    println!(
        "PERFORMANCE {}",
        json!({"windows":1000,"events":100,"startupMs":startup.as_secs_f64()*1000.,"initialSyncMs":sync.as_secs_f64()*1000.,"eventMedianMs":samples[50],"eventP95Ms":samples[95],"eventMaxMs":samples[99],"recoveryMs":recovery.as_secs_f64()*1000.})
    );
    c.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn last_handle_drop_stops_worker_and_releases_name() {
    let bus = PrivateBus::start();
    let temp = tempfile::tempdir().unwrap();
    let c = Controller::start(
        SqliteStore::open(&temp.path().join("db")).unwrap(),
        options(&bus),
    )
    .unwrap();
    wait(&c, |h| h.lifecycle == Lifecycle::WaitingForObserver).await;
    let mut status = c.subscribe();
    drop(c);
    tokio::time::timeout(Duration::from_secs(3), async {
        while status.changed().await.is_ok() {}
    })
    .await
    .unwrap();
    assert_eq!(status.borrow().lifecycle, Lifecycle::Stopped);
    let next = Receiver::at_address(&bus.address).await.unwrap();
    next.shutdown().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancellation_interrupts_a_stalled_bus_handshake() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("stalled-bus");
    let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let c = Controller::start(
        SqliteStore::open(&temp.path().join("db")).unwrap(),
        ControllerOptions {
            bus_address: Some(format!("unix:path={}", socket.display())),
            connect_timeout: Duration::from_secs(5),
            ..Default::default()
        },
    )
    .unwrap();
    wait(&c, |h| h.receiver_attempts == 1).await;
    let start = Instant::now();
    c.shutdown().await.unwrap();
    assert!(start.elapsed() < Duration::from_secs(1));
    assert_eq!(c.last_health().lifecycle, Lifecycle::Stopped);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_bus_loss_and_explicit_retry_rebuild_receiver_without_restoring_assignments() {
    use std::{
        io::{BufRead, BufReader},
        os::unix::fs::FileTypeExt,
        path::{Path, PathBuf},
        process::{Child, Command, Stdio},
    };
    struct Bus {
        child: Child,
        socket: PathBuf,
    }
    impl Bus {
        fn start(socket: &Path) -> Self {
            let mut child = Command::new("dbus-daemon")
                .args([
                    "--session",
                    "--nofork",
                    "--print-address=1",
                    &format!("--address=unix:path={}", socket.display()),
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let mut address = String::new();
            BufReader::new(child.stdout.take().unwrap())
                .read_line(&mut address)
                .unwrap();
            assert!(address.starts_with("unix:"));
            Self {
                child,
                socket: socket.into(),
            }
        }
    }
    impl Drop for Bus {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
            if let Ok(m) = std::fs::symlink_metadata(&self.socket) {
                assert!(m.file_type().is_socket());
                std::fs::remove_file(&self.socket).unwrap();
            }
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("private-bus");
    let bus = Bus::start(&socket);
    let address = format!("unix:path={}", socket.display());
    let publisher = zbus::connection::Builder::address(address.as_str())
        .unwrap()
        .build()
        .await
        .unwrap();
    publisher.request_name("org.kde.KWin").await.unwrap();
    let c = Controller::start(
        SqliteStore::open(&temp.path().join("db")).unwrap(),
        ControllerOptions {
            bus_address: Some(address.clone()),
            max_receiver_attempts: 2,
            retry_delay: Duration::from_millis(10),
            ..Default::default()
        },
    )
    .unwrap();
    wait(&c, |h| h.lifecycle == Lifecycle::WaitingForObserver).await;
    let target = workspace(&c).await;
    let consumer = hello(&publisher, "test-session").await;
    assert!(!push(&publisher, &consumer, initial()).await);
    let r = c.associations(Selection::All).await.unwrap();
    c.assign(
        r.associations[0].window.clone(),
        BTreeSet::from([target]),
        r.version.clone(),
    )
    .await
    .unwrap();
    drop(bus);
    wait(&c, |h| h.reason == RecoveryReason::RetryExhausted).await;
    assert!(c.associations(Selection::All).await.is_err());
    assert!(c.snapshot().await.is_err());
    assert!(!c.hierarchy().await.unwrap().worlds.is_empty());
    let _bus = Bus::start(&socket);
    let publisher = zbus::connection::Builder::address(address.as_str())
        .unwrap()
        .build()
        .await
        .unwrap();
    publisher.request_name("org.kde.KWin").await.unwrap();
    c.retry_receiver().await.unwrap();
    wait(&c, |h| {
        h.lifecycle == Lifecycle::Recovering && h.reason == RecoveryReason::ObservationLost
    })
    .await;
    let consumer = hello(&publisher, "test-session").await;
    assert!(!push(&publisher, &consumer, initial()).await);
    let fresh = c.associations(Selection::All).await.unwrap();
    assert_ne!(fresh.version.epoch, r.version.epoch);
    assert_eq!(fresh.associations[0].decision.status, Status::Unknown);
    c.shutdown().await.unwrap();
}
