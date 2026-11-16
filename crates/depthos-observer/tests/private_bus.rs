use depthos_observer::{
    INTERFACE, ObservationHealth as H, ObservedStateSource, PATH, Receiver, SERVICE,
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    time::Duration,
};
use zbus::{Connection, Proxy};
struct PrivateBus {
    child: std::process::Child,
    address: String,
}
impl PrivateBus {
    fn start() -> Self {
        // A fresh daemon is unconditionally spawned: never use inherited desktop bus.
        let mut child = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("dbus-daemon required");
        let mut address = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        Self {
            child,
            address: address.trim().into(),
        }
    }
    async fn connection(&self) -> Connection {
        zbus::connection::Builder::address(self.address.as_str())
            .unwrap()
            .build()
            .await
            .unwrap()
    }
}
impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
async fn endpoint(c: &Connection) -> Proxy<'_> {
    Proxy::new(c, SERVICE, PATH, INTERFACE).await.unwrap()
}
async fn hello(c: &Connection, session: &str) -> zbus::Result<String> {
    endpoint(c).await.call("Hello", &(session,)).await
}
fn fixture() -> Value {
    serde_json::from_str::<Value>(include_str!("../../../tests/observer-adapter/trace.json"))
        .unwrap()["frames"][0]
        .clone()
}
fn packet(consumer: &str) -> String {
    let f = fixture();
    json!({"schemaVersion":1,"sessionId":"test-session","consumerId":consumer,"frames":[f],"position":{"sequence":f["sequence"],"revision":f["revision"]}}).to_string()
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registration_authentication_owner_restart_and_shutdown() {
    let bus = PrivateBus::start();
    let publisher = bus.connection().await;
    publisher.request_name("org.kde.KWin").await.unwrap();
    let receiver = Receiver::at_address(&bus.address).await.unwrap();
    let observer = receiver.observer();
    let mut changes = observer.subscribe();
    assert!(
        Receiver::at_address(&bus.address).await.is_err(),
        "service collision must fail"
    );
    let stranger = bus.connection().await;
    assert!(
        hello(&stranger, "test-session")
            .await
            .unwrap_err()
            .to_string()
            .contains("AccessDenied")
    );
    let denied: zbus::Result<String> = endpoint(&stranger)
        .await
        .call("Push", &(packet("wrong"),))
        .await;
    assert!(denied.unwrap_err().to_string().contains("AccessDenied"));
    assert!(hello(&publisher, "").await.is_err());
    assert!(hello(&publisher, &"x".repeat(257)).await.is_err());
    let h: Value = serde_json::from_str(&hello(&publisher, "test-session").await.unwrap()).unwrap();
    assert_eq!(h["schemaVersion"], 1);
    assert_eq!(observer.health(), H::AwaitingSnapshot);
    let reply: String = endpoint(&publisher)
        .await
        .call("Push", &(packet(h["consumerId"].as_str().unwrap()),))
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&reply).unwrap()["needSnapshot"],
        false
    );
    tokio::time::timeout(Duration::from_secs(2), changes.changed())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(observer.health(), H::Synchronized);
    assert!(observer.snapshot().is_ok());
    let reply: String = endpoint(&publisher)
        .await
        .call("Push", &(packet("stale-consumer"),))
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&reply).unwrap()["needSnapshot"],
        true
    );
    assert_eq!(observer.health(), H::Synchronized);
    // Ownership transfer: old sender loses publishing rights immediately.
    publisher.release_name("org.kde.KWin").await.unwrap();
    stranger.request_name("org.kde.KWin").await.unwrap();
    assert!(
        hello(&publisher, "test-session")
            .await
            .unwrap_err()
            .to_string()
            .contains("AccessDenied")
    );
    let denied: zbus::Result<String> = endpoint(&stranger)
        .await
        .call("Push", &(packet(h["consumerId"].as_str().unwrap()),))
        .await;
    assert!(denied.is_err(), "new owner must Hello");
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert_eq!(observer.health(), H::Disconnected);
    assert!(observer.snapshot().is_err());
    let new_hello: Value =
        serde_json::from_str(&hello(&stranger, "test-session").await.unwrap()).unwrap();
    let _: String = endpoint(&stranger)
        .await
        .call(
            "Push",
            &(packet(new_hello["consumerId"].as_str().unwrap()),),
        )
        .await
        .unwrap();
    assert_eq!(observer.health(), H::Synchronized);
    let old_id = new_hello["consumerId"].clone();
    receiver.shutdown().await.unwrap();
    assert_eq!(observer.health(), H::Disconnected);
    assert!(observer.snapshot().is_err());
    let restarted = Receiver::at_address(&bus.address).await.unwrap();
    let h: Value = serde_json::from_str(&hello(&stranger, "test-session").await.unwrap()).unwrap();
    assert_ne!(old_id, h["consumerId"]);
    let _: String = endpoint(&stranger)
        .await
        .call("Push", &(packet(old_id.as_str().unwrap()),))
        .await
        .unwrap();
    assert!(restarted.observer().snapshot().is_err());
    let _: String = endpoint(&stranger)
        .await
        .call("Push", &(packet(h["consumerId"].as_str().unwrap()),))
        .await
        .unwrap();
    assert_eq!(restarted.observer().health(), H::Synchronized);
    restarted.shutdown().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn liveness_timeout_and_drop_invalidate_readers() {
    let bus = PrivateBus::start();
    let publisher = bus.connection().await;
    publisher.request_name("org.kde.KWin").await.unwrap();
    let receiver = Receiver::at_address(&bus.address).await.unwrap();
    let observer = receiver.observer();
    let h: Value = serde_json::from_str(&hello(&publisher, "test-session").await.unwrap()).unwrap();
    let raw = packet(h["consumerId"].as_str().unwrap());
    let _: String = endpoint(&publisher)
        .await
        .call("Push", &(raw,))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(6200)).await;
    assert_eq!(observer.health(), H::Desynchronized);
    assert!(observer.snapshot().is_err());
    // Snapshot may be a duplicate of the previous baseline: must not heal.
    let raw = packet(h["consumerId"].as_str().unwrap());
    let _: String = endpoint(&publisher)
        .await
        .call("Push", &(raw,))
        .await
        .unwrap();
    assert!(observer.snapshot().is_err());
    let mut p: Value = serde_json::from_str(&packet(h["consumerId"].as_str().unwrap())).unwrap();
    p["frames"][0]["sequence"] = json!(100);
    p["position"]["sequence"] = json!(100);
    let _: String = endpoint(&publisher)
        .await
        .call("Push", &(p.to_string(),))
        .await
        .unwrap();
    assert_eq!(observer.health(), H::Synchronized);
    drop(receiver);
    assert_eq!(observer.health(), H::Disconnected);
}
#[tokio::test]
async fn network_addresses_refused() {
    for address in [
        "tcp:host=localhost,port=1",
        "unixexec:path=/bin/false",
        "unix:path=/tmp/no;tcp:host=localhost,port=1",
    ] {
        assert!(Receiver::at_address(address).await.is_err());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bus_loss_invalidates_existing_snapshot() {
    let mut bus = PrivateBus::start();
    let publisher = bus.connection().await;
    publisher.request_name("org.kde.KWin").await.unwrap();
    let receiver = Receiver::at_address(&bus.address).await.unwrap();
    let observer = receiver.observer();
    let h: Value = serde_json::from_str(&hello(&publisher, "test-session").await.unwrap()).unwrap();
    let _: String = endpoint(&publisher)
        .await
        .call("Push", &(packet(h["consumerId"].as_str().unwrap()),))
        .await
        .unwrap();
    assert!(observer.snapshot().is_ok());
    bus.child.kill().unwrap();
    bus.child.wait().unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while observer.health() != H::Disconnected {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(observer.snapshot().is_err());
    drop(receiver);
}
