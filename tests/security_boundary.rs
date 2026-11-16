//! Private fixture only. It never registers a production service or offers execution.
#[path = "support/private_bus.rs"]
mod private_bus;
use depthos_core::action_security::{Authorizer, Client, Error};
use std::sync::Mutex;
use uuid::Uuid;
use zbus::{Connection, Proxy, fdo::DBusProxy, message::Header};
struct DenialFixture(Mutex<Authorizer>);
#[zbus::interface(name = "org.depthos.TestAuthorization1")]
impl DenialFixture {
    async fn request_authorization(
        &self,
        claimed_click: bool,
        claimed_app: &str,
        claimed_pid: u32,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<()> {
        let sender = header
            .sender()
            .ok_or_else(|| zbus::fdo::Error::AccessDenied("No sender".into()))?;
        let bus = DBusProxy::new(conn).await?;
        let uid = bus.get_connection_unix_user(sender.clone().into()).await?;
        let pid = bus
            .get_connection_unix_process_id(sender.clone().into())
            .await?;
        let bus_id = Uuid::parse_str(bus.get_id().await?.as_str()).unwrap();
        let caller = Client {
            bus_id,
            unique_name: sender.to_string(),
            uid,
        };
        // These comparisons establish fixture coverage; none is proof of a click.
        assert_eq!(uid, rustix::process::getuid().as_raw());
        let _plausible_process = pid == claimed_pid;
        let _plausible_app = claimed_app == "org.kde.KWin";
        match self
            .0
            .lock()
            .unwrap()
            .verify_interaction_claim(&caller, claimed_click)
        {
            Err(Error::UnverifiedInteraction) => Err(zbus::fdo::Error::AccessDenied(
                "Interactive authorization unproven".into(),
            )),
            _ => panic!("a public request must never mint proof"),
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_same_uid_connections_and_forged_ui_claims_cannot_authorize() {
    let bus = private_bus::PrivateBus::start();
    let server = zbus::connection::Builder::address(bus.address.as_str())
        .unwrap()
        .serve_at(
            "/org/depthos/TestAuthorization",
            DenialFixture(Mutex::new(Authorizer::new())),
        )
        .unwrap()
        .build()
        .await
        .unwrap();
    server
        .request_name("org.depthos.TestAuthorization")
        .await
        .unwrap();
    let mut names = Vec::new();
    let owner = bus.publisher().await;
    let ordinary = zbus::connection::Builder::address(bus.address.as_str())
        .unwrap()
        .build()
        .await
        .unwrap();
    // Even a current compositor-name owner is not evidence of a particular click.
    for client in [owner, ordinary] {
        names.push(client.unique_name().unwrap().to_string());
        let p = Proxy::new(
            &client,
            "org.depthos.TestAuthorization",
            "/org/depthos/TestAuthorization",
            "org.depthos.TestAuthorization1",
        )
        .await
        .unwrap();
        for (click, app, pid) in [
            (true, "org.kde.KWin", std::process::id()),
            (true, "org.depthos.TrustedQml", u32::MAX),
            (false, "unknown", 0),
        ] {
            let error = p
                .call::<_, _, ()>("RequestAuthorization", &(click, app, pid))
                .await
                .unwrap_err();
            assert!(error.to_string().contains("AccessDenied"), "{error}");
        }
        let introspect = zbus::fdo::IntrospectableProxy::builder(&client)
            .destination("org.depthos.TestAuthorization")
            .unwrap()
            .path("/org/depthos/TestAuthorization")
            .unwrap()
            .build()
            .await
            .unwrap();
        let xml = introspect.introspect().await.unwrap();
        assert!(!xml.contains("Execute"));
        assert!(!xml.contains("MoveWindow"));
        client.close().await.unwrap();
    }
    assert_ne!(names[0], names[1]);
    server.close().await.unwrap();
}
