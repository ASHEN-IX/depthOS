use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
};
use zbus::Connection;

pub struct PrivateBus {
    child: Child,
    pub address: String,
}
impl PrivateBus {
    pub fn start() -> Self {
        let mut child = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut address = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        Self {
            child,
            address: address.trim().into(),
        }
    }
    pub async fn publisher(&self) -> Connection {
        let c = zbus::connection::Builder::address(self.address.as_str())
            .unwrap()
            .build()
            .await
            .unwrap();
        c.request_name("org.kde.KWin").await.unwrap();
        c
    }
}
impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
