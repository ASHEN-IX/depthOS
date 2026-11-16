//! Offline sanitized-fixture utility. Never opens D-Bus.
use depthos_observer::Replica;
use std::io::Read;
fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let packets: Vec<serde_json::Value> = serde_json::from_str(&input).unwrap();
    let mut r = Replica::new("test-consumer".into());
    r.hello("test-session").unwrap();
    let mut results = Vec::new();
    for p in packets {
        let error = r.push(&p.to_string()).is_err();
        results.push(serde_json::json!({"error":error,"health":r.health(),"snapshot":r.snapshot().ok().map(|mut s|{s.redact_titles();s})}));
    }
    println!("{}", serde_json::to_string(&results).unwrap());
}
