use depthos_observer::Replica;
use serde_json::{Value, json};
fn trace() -> Value {
    serde_json::from_str(include_str!("../../../tests/observer-adapter/trace.json")).unwrap()
}
fn packet(frame: Value) -> String {
    json!({"schemaVersion":1,"sessionId":frame["sessionId"],"consumerId":"test","position":{"sequence":frame["sequence"],"revision":frame["revision"]},"frames":[frame]}).to_string()
}
fn ready() -> Replica {
    let mut r = Replica::new("test".into());
    r.hello("test-session").unwrap();
    assert!(!r.push(&packet(trace()["frames"][0].clone())).unwrap());
    r
}
#[test]
fn healthy_incremental_updates_preserve_epoch_and_live_generation() {
    let mut r = ready();
    let before = r.snapshot().unwrap();
    assert!(!r.push(&packet(trace()["frames"][1].clone())).unwrap());
    let after = r.snapshot().unwrap();
    assert_eq!(before.observer_epoch, after.observer_epoch);
    assert_eq!(before.window_generations, after.window_generations);
    let mut duplicate = packet(trace()["frames"][1].clone());
    assert!(!r.push(&duplicate).unwrap());
    assert_eq!(after.observer_epoch, r.snapshot().unwrap().observer_epoch);
    // A stale consumer cannot invalidate continuity.
    duplicate = duplicate.replace("\"consumerId\":\"test\"", "\"consumerId\":\"obsolete\"");
    assert!(r.push(&duplicate).is_err());
    assert_eq!(after.observer_epoch, r.snapshot().unwrap().observer_epoch);
}
#[test]
fn new_hello_and_unobserved_gap_reconciliation_have_fresh_epochs() {
    let mut r = ready();
    let before = r.snapshot().unwrap();
    let mut gap = trace()["frames"][1].clone();
    gap["sequence"] = json!(99);
    assert!(r.push(&packet(gap)).unwrap());
    let mut next = trace()["frames"][0].clone();
    next["sequence"] = json!(100);
    assert!(!r.push(&packet(next)).unwrap());
    assert_ne!(before.observer_epoch, r.snapshot().unwrap().observer_epoch);
    let before = r.snapshot().unwrap();
    r.hello("test-session").unwrap();
    assert!(!r.push(&packet(trace()["frames"][0].clone())).unwrap());
    assert_ne!(before.observer_epoch, r.snapshot().unwrap().observer_epoch);
}
#[test]
fn snapshot_jump_or_native_uuid_change_cannot_restore_a_previous_key() {
    let mut r = ready();
    let before = r.snapshot().unwrap();
    let mut next = trace()["frames"][0].clone();
    next["sequence"] = json!(50);
    assert!(!r.push(&packet(next)).unwrap());
    assert_ne!(before.observer_epoch, r.snapshot().unwrap().observer_epoch);
    let before = r.snapshot().unwrap();
    let mut next = trace()["frames"][0].clone();
    next["sequence"] = json!(51);
    next["state"]["windows"][0]["nativeId"] = Value::Null;
    assert!(!r.push(&packet(next)).unwrap());
    let after = r.snapshot().unwrap();
    assert_eq!(before.observer_epoch, after.observer_epoch);
    assert_ne!(before.window_generations, after.window_generations);
}
