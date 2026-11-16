use super::*;
use std::{cell::RefCell, future::poll_fn, pin::pin, sync::mpsc as sync_mpsc, task::Poll};

struct StopPublicationGate {
    reached: oneshot::Sender<()>,
    release: sync_mpsc::Receiver<()>,
}

thread_local! {
    static STOP_PUBLICATION_GATE: RefCell<Option<StopPublicationGate>> = const { RefCell::new(None) };
}

pub(super) fn before_stop_publication() {
    if let Some(gate) = STOP_PUBLICATION_GATE.take() {
        gate.reached.send(()).unwrap();
        gate.release.recv_timeout(Duration::from_secs(5)).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delayed_shutdown_announcement_preserves_terminal_outcome() {
    for reason in [RecoveryReason::Shutdown, RecoveryReason::CleanupFailed] {
        let (tx, _commands) = mpsc::channel(1);
        let (status, health) = watch::channel(ControllerHealth::starting());
        let (cancel, stop) = watch::channel(false);
        let (finish_worker, finish) = oneshot::channel();
        let worker_status = status.clone();
        let task = tokio::spawn(async move {
            finish.await.unwrap();
            assert!(*stop.borrow());
            worker_status.send_modify(|health| {
                health.lifecycle = Lifecycle::Stopped;
                health.reason = reason;
                health.database_available = false;
            });
        });
        let c = Controller {
            inner: Arc::new(Inner {
                tx,
                health,
                announce_stop: Mutex::new(Some(status)),
                cancel,
                task: Mutex::new(Some(task)),
                timeout: Duration::from_secs(1),
            }),
        };
        let mut subscriber = c.subscribe();
        let (reached, at_publication) = oneshot::channel();
        let (resume, release) = sync_mpsc::channel();
        let delayed = tokio::spawn({
            let c = c.clone();
            async move {
                STOP_PUBLICATION_GATE.set(Some(StopPublicationGate { reached, release }));
                c.shutdown().await
            }
        });
        at_publication.await.unwrap();

        // The second caller takes the join handle while the first is paused.
        let mut joining = pin!(c.shutdown());
        poll_fn(|cx| {
            assert!(joining.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        finish_worker.send(()).unwrap();
        let terminal = subscriber
            .wait_for(|health| health.lifecycle == Lifecycle::Stopped)
            .await
            .unwrap()
            .clone();
        assert_eq!(terminal.reason, reason);
        resume.send(()).unwrap();

        let delayed_result = delayed.await.unwrap();
        assert_eq!(c.last_health(), terminal);
        let joining_result = joining.await;
        let repeated_result = c.shutdown().await;
        for result in [delayed_result, joining_result, repeated_result] {
            match reason {
                RecoveryReason::Shutdown => result.unwrap(),
                RecoveryReason::CleanupFailed => {
                    assert!(matches!(result, Err(ControllerError::CleanupFailed)));
                }
                _ => unreachable!(),
            }
        }
        assert_eq!(*subscriber.borrow(), terminal);
        assert!(subscriber.changed().await.is_err());
    }
}

#[test]
fn lifecycle_terminal_transitions_are_explicit() {
    assert!(Lifecycle::Starting.allows(Lifecycle::WaitingForObserver));
    assert!(Lifecycle::Synchronized.allows(Lifecycle::Recovering));
    assert!(!Lifecycle::Stopped.allows(Lifecycle::Starting));
    assert!(!Lifecycle::Stopping.allows(Lifecycle::Synchronized));
}
#[tokio::test]
async fn interrupted_worker_closes_api_and_invalidates_status() {
    let temp = tempfile::tempdir().unwrap();
    let c = Controller::start(
        SqliteStore::open(&temp.path().join("db")).unwrap(),
        ControllerOptions {
            bus_address: Some("unix:path=/nonexistent/depthos-worker-test".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let mut status = c.subscribe();
    status.changed().await.unwrap();
    c.inner.task.lock().unwrap().as_ref().unwrap().abort();
    tokio::time::timeout(Duration::from_secs(2), async {
        while status.borrow().reason != RecoveryReason::WorkerLost {
            status.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert!(c.snapshot().await.is_err());
    assert_eq!(c.last_health().lifecycle, Lifecycle::Degraded);
    assert!(matches!(
        c.shutdown().await,
        Err(ControllerError::WorkerLost)
    ));
    assert!(matches!(
        c.shutdown().await,
        Err(ControllerError::WorkerLost)
    ));
    assert_eq!(status.borrow().reason, RecoveryReason::WorkerLost);
    assert_eq!(status.borrow().lifecycle, Lifecycle::Degraded);
}
#[tokio::test]
async fn bounded_queue_reports_busy_without_enqueuing_a_duplicate_operation() {
    let (tx, mut commands) = mpsc::channel(16);
    let (_status, health) = watch::channel(ControllerHealth::starting());
    let (cancel, _) = watch::channel(false);
    let c = Controller {
        inner: Arc::new(Inner {
            tx,
            health,
            announce_stop: Mutex::new(Some(_status)),
            cancel,
            task: Mutex::new(None),
            timeout: Duration::from_secs(1),
        }),
    };
    for _ in 0..16 {
        let (r, _) = oneshot::channel();
        c.inner
            .tx
            .try_send(Command::Health(r))
            .unwrap_or_else(|_| panic!("queue unexpectedly full"));
    }
    assert!(matches!(c.health().await, Err(ControllerError::Busy)));
    assert!(matches!(
        c.logical(LogicalCommand::CreateWorld("x".repeat(4097), None))
            .await,
        Err(ControllerError::InvalidCommand)
    ));
    for _ in 0..16 {
        assert!(commands.try_recv().is_ok());
    }
    assert!(commands.try_recv().is_err());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interrupted_synchronized_worker_loses_assignments_and_releases_service() {
    mod private_bus {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/private_bus.rs"
        ));
    }
    use crate::context::policy::Status;
    use serde_json::{Value, json};
    let bus = private_bus::PrivateBus::start();
    let publisher = bus.publisher().await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    let options = ControllerOptions {
        bus_address: Some(bus.address.clone()),
        ..Default::default()
    };
    let c = Controller::start(SqliteStore::open(&path).unwrap(), options.clone()).unwrap();
    async fn waiting(c: &Controller) {
        let mut w = c.subscribe();
        tokio::time::timeout(Duration::from_secs(3), async {
            while c.health().await.unwrap().lifecycle != Lifecycle::WaitingForObserver {
                w.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
    }
    async fn publish(p: &zbus::Connection) {
        let endpoint = zbus::Proxy::new(
            p,
            depthos_observer::SERVICE,
            depthos_observer::PATH,
            depthos_observer::INTERFACE,
        )
        .await
        .unwrap();
        let hello: String = endpoint.call("Hello", &("test-session",)).await.unwrap();
        let hello: Value = serde_json::from_str(&hello).unwrap();
        let t: Value =
            serde_json::from_str(include_str!("../../tests/observer-adapter/trace.json")).unwrap();
        let f = &t["frames"][0];
        let raw=json!({"schemaVersion":1,"sessionId":"test-session","consumerId":hello["consumerId"],"frames":[f],"position":{"sequence":f["sequence"],"revision":f["revision"]}}).to_string();
        let _: String = endpoint.call("Push", &(raw,)).await.unwrap();
    }
    waiting(&c).await;
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
    publish(&publisher).await;
    let report = c.associations(Selection::All).await.unwrap();
    c.assign(
        report.associations[0].window.clone(),
        BTreeSet::from([target]),
        report.version.clone(),
    )
    .await
    .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    c.inner.task.lock().unwrap().as_ref().unwrap().abort();
    assert!(matches!(
        c.shutdown().await,
        Err(ControllerError::WorkerLost)
    ));
    assert!(c.snapshot().await.is_err());
    let bus_proxy = zbus::fdo::DBusProxy::new(&publisher).await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while bus_proxy
            .name_has_owner(depthos_observer::SERVICE.try_into().unwrap())
            .await
            .unwrap()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let fresh = Controller::start(SqliteStore::open(&path).unwrap(), options).unwrap();
    waiting(&fresh).await;
    publish(&publisher).await;
    let current = fresh.associations(Selection::All).await.unwrap();
    assert_eq!(current.associations[0].decision.status, Status::Unknown);
    assert_ne!(current.version.epoch, report.version.epoch);
    assert!(
        fresh
            .assign(
                current.associations[0].window.clone(),
                BTreeSet::from([target]),
                report.version
            )
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    fresh.shutdown().await.unwrap();
}
