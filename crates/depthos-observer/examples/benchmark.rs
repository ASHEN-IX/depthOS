//! Synthetic reducer cost only; makes no compositor overhead claim.
use depthos_observer::Replica;
use serde_json::{Value, json};
use std::time::Instant;
fn packet(frame: Value) -> String {
    json!({"schemaVersion":1,"sessionId":"test-session","consumerId":"bench","position":{"sequence":frame["sequence"],"revision":frame["revision"]},"frames":[frame]}).to_string()
}
fn main() {
    let trace: Value =
        serde_json::from_str(include_str!("../../../tests/observer-adapter/trace.json")).unwrap();
    let mut initial = trace["frames"][0].clone();
    let template = initial["state"]["windows"][0].clone();
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
    let mut replica = Replica::new("bench".into());
    replica.hello("test-session").unwrap();
    assert!(!replica.push(&packet(initial.clone())).unwrap());
    // Build event strings before timing so measurement isolates decoding/reduction.
    let events:Vec<_>=(0..10_000).map(|n| {
        let id=format!("window-{}",n%1000);let mut w=template.clone();w["runtimeId"]=json!(id);w["geometry"]["x"]=json!(n);
        packet(json!({"schemaVersion":1,"sessionId":"test-session","sequence":initial["sequence"].as_u64().unwrap()+n+1,"revision":initial["revision"].as_u64().unwrap()+n+1,"type":"window.geometry.changed","windowId":id,"changedFields":["geometry"],"window":w}))
    }).collect();
    let start = Instant::now();
    for raw in &events {
        assert!(!replica.push(raw).unwrap());
    }
    let elapsed = start.elapsed();
    assert_eq!(replica.snapshot().unwrap().state.windows.len(), 1000);
    println!(
        "1000 windows / 10000 geometry packets: {:.3}s, {:.1} us/packet, {:.0} packets/s",
        elapsed.as_secs_f64(),
        elapsed.as_secs_f64() * 100.,
        10_000. / elapsed.as_secs_f64()
    );
}
