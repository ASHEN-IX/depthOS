use depthos_observer::{ObservationHealth as H, Replica};
use serde_json::{Value, json};
fn trace() -> Value {
    serde_json::from_str(include_str!("../../../tests/observer-adapter/trace.json")).unwrap()
}
fn packet(frames: Vec<Value>) -> Value {
    let position = frames
        .last()
        .map(|f| json!({"sequence":f["sequence"],"revision":f["revision"]}))
        .unwrap_or(json!({"sequence":0,"revision":1}));
    json!({"schemaVersion":1,"sessionId":"test-session","consumerId":"test-consumer","frames":frames,"position":position})
}
fn replica() -> Replica {
    let mut r = Replica::new("test-consumer".into());
    r.hello("test-session").unwrap();
    r
}
fn initial() -> Value {
    trace()["frames"][0].clone()
}
fn ready() -> Replica {
    let mut r = replica();
    assert!(!r.push(&packet(vec![initial()]).to_string()).unwrap());
    r
}
#[test]
fn production_trace_all_events_and_snapshot_replacement() {
    let t = trace();
    let mut r = replica();
    for f in t["frames"].as_array().unwrap() {
        assert!(!r.push(&packet(vec![f.clone()]).to_string()).unwrap());
    }
    let actual = r.snapshot().unwrap().state;
    let expected: depthos_observer::NativeState =
        serde_json::from_value(t["final"]["state"].clone()).unwrap();
    assert!(actual == expected);
    assert!(
        !r.push(&packet(vec![t["final"].clone()]).to_string())
            .unwrap()
    );
    let s = r.snapshot().unwrap();
    assert_eq!(
        s.window_identity("window-2").unwrap().session_id,
        "test-session"
    );
}
#[test]
fn valid_invalid_hello_and_disconnect() {
    let mut r = Replica::new("x".into());
    assert_eq!(r.health(), H::Disconnected);
    assert!(r.snapshot().is_err());
    for s in ["".into(), "a".repeat(257)] {
        assert!(r.hello(&s).is_err());
    }
    r.hello("ok").unwrap();
    assert_eq!(r.health(), H::AwaitingSnapshot);
    r.disconnect();
    assert_eq!(r.health(), H::Disconnected);
}
#[test]
fn duplicate_does_not_roll_back() {
    let mut r = ready();
    let seq = r.snapshot().unwrap().sequence;
    assert!(!r.push(&packet(vec![initial()]).to_string()).unwrap());
    assert_eq!(r.snapshot().unwrap().sequence, seq);
}
#[test]
fn duplicate_and_out_of_order_window_events_preserve_the_newest_state() {
    let mut replica = ready();
    let frames = trace();
    let first = frames["frames"][1].clone();
    let latest = frames["frames"][2].clone();
    let current = packet(vec![first.clone(), latest.clone()]);
    assert!(!replica.push(&current.to_string()).unwrap());
    let before = replica.snapshot().unwrap();
    let mut stale = first;
    stale["window"]["applicationId"] = json!("org.example.Older");
    assert!(!replica.push(&packet(vec![latest]).to_string()).unwrap());
    assert!(!replica.push(&packet(vec![stale]).to_string()).unwrap());
    let after = replica.snapshot().unwrap();
    assert_eq!(after.sequence, before.sequence);
    assert_eq!(after.window_generations, before.window_generations);
    assert_eq!(
        after.state.windows[0].application_id,
        before.state.windows[0].application_id
    );
    assert_eq!(
        after.state.windows[0].desktop_ids,
        before.state.windows[0].desktop_ids
    );
}

#[test]
fn gaps_recover_only_with_snapshot() {
    for key in ["sequence", "revision"] {
        let mut r = ready();
        let mut f = trace()["frames"][1].clone();
        f[key] = json!(f[key].as_u64().unwrap() + 1);
        assert!(r.push(&packet(vec![f]).to_string()).unwrap());
        assert_eq!(r.health(), H::Desynchronized);
        assert!(r.snapshot().is_err());
        assert!(
            !r.push(&packet(vec![trace()["final"].clone()]).to_string())
                .unwrap()
        );
    }
}
#[test]
fn empty_heartbeat_checks_sequence_and_revision_but_batch_may_trail() {
    for key in ["sequence", "revision"] {
        let mut r = ready();
        let s = r.snapshot().unwrap();
        let mut p = packet(vec![]);
        p["position"] = json!({"sequence":s.sequence,"revision":s.revision});
        assert!(!r.push(&p.to_string()).unwrap());
        p["position"][key] = json!(99);
        assert!(r.push(&p.to_string()).unwrap());
    }
    let mut r = replica();
    let mut p = packet(vec![initial()]);
    p["position"] = json!({"sequence":999,"revision":999});
    assert!(!r.push(&p.to_string()).unwrap());
}
#[test]
fn stale_consumer_cannot_mutate() {
    let mut r = ready();
    let s = r.snapshot().unwrap().sequence;
    let mut p = packet(vec![trace()["final"].clone()]);
    p["consumerId"] = json!("obsolete");
    assert!(r.push(&p.to_string()).is_err());
    assert_eq!(r.health(), H::Synchronized);
    assert_eq!(r.snapshot().unwrap().sequence, s);
}
#[test]
fn new_session_invalidates_old_and_requires_hello() {
    let mut r = ready();
    let mut p = packet(vec![]);
    p["sessionId"] = json!("new");
    assert!(r.push(&p.to_string()).unwrap());
    assert!(r.snapshot().is_err());
    r.hello("new").unwrap();
    let mut f = initial();
    f["sessionId"] = json!("new");
    p["frames"] = json!([f]);
    p["position"] = json!({"sequence":1,"revision":1});
    assert!(!r.push(&p.to_string()).unwrap());
    assert_eq!(r.snapshot().unwrap().session_id, "new");
}
#[test]
fn reject_invalid_packets_and_frames() {
    let base = packet(vec![initial()]);
    let mut bad = Vec::new();
    for path in [
        "schemaVersion",
        "frames",
        "position",
        "sessionId",
        "consumerId",
    ] {
        let mut p = base.clone();
        p.as_object_mut().unwrap().remove(path);
        bad.push(p);
    }
    let mut p = base.clone();
    p["schemaVersion"] = json!(2);
    bad.push(p);
    for key in ["schemaVersion", "sequence", "revision", "type", "state"] {
        let mut p = base.clone();
        p["frames"][0].as_object_mut().unwrap().remove(key);
        bad.push(p);
    }
    for (key, value) in [
        ("sequence", json!(0)),
        ("revision", json!(-1)),
        ("type", json!("future.event")),
        ("schemaVersion", json!(2)),
        ("sessionId", json!("other")),
    ] {
        let mut p = base.clone();
        p["frames"][0][key] = value;
        bad.push(p);
    }
    let mut p = base.clone();
    p["frames"] = json!(vec![initial(); 33]);
    bad.push(p);
    for p in bad {
        let mut r = ready();
        assert!(r.push(&p.to_string()).is_err());
        assert_eq!(r.health(), H::Desynchronized);
        assert!(r.snapshot().is_err());
    }
    for raw in [
        "{".into(),
        "{\"x\":NaN}".into(),
        " ".repeat(4 * 1024 * 1024 + 1),
        "[".repeat(200),
    ] {
        assert!(ready().push(&raw).is_err());
    }
}
#[test]
fn missing_required_nullable_fields_rejected_and_nulls_preserved() {
    let mut p = packet(vec![initial()]);
    p["frames"][0]["state"]["windows"][0]["title"] = Value::Null;
    let mut r = replica();
    assert!(!r.push(&p.to_string()).unwrap());
    assert!(r.snapshot().unwrap().state.windows[0].title.is_none());
    for key in ["activeWindowId", "desktops"] {
        let mut p = packet(vec![initial()]);
        p["frames"][0]["state"].as_object_mut().unwrap().remove(key);
        assert!(replica().push(&p.to_string()).is_err());
    }
    let mut p = packet(vec![initial()]);
    p["frames"][0]["state"]["windows"][0]
        .as_object_mut()
        .unwrap()
        .remove("title");
    assert!(replica().push(&p.to_string()).is_err());
    let s = ready().snapshot().unwrap();
    let f = json!({"schemaVersion":1,"sessionId":"test-session","sequence":s.sequence+1,"revision":s.revision+1,"type":"window.focus.changed"});
    assert!(ready().push(&packet(vec![f]).to_string()).is_err());
}
#[test]
fn invalid_references_duplicate_ids_and_regressing_snapshot() {
    let mut p = packet(vec![initial()]);
    let w = p["frames"][0]["state"]["windows"][0].clone();
    p["frames"][0]["state"]["windows"]
        .as_array_mut()
        .unwrap()
        .push(w);
    assert!(replica().push(&p.to_string()).is_err());
    let mut r = ready();
    let s = r.snapshot().unwrap();
    let f = json!({"schemaVersion":1,"sessionId":"test-session","sequence":s.sequence+1,"revision":s.revision+1,"type":"window.focus.changed","activeWindowId":"absent"});
    assert!(r.push(&packet(vec![f]).to_string()).is_err());
    let mut r = replica();
    r.push(&packet(vec![trace()["final"].clone()]).to_string())
        .unwrap();
    let mut f = initial();
    f["sequence"] = json!(999);
    assert!(r.push(&packet(vec![f]).to_string()).is_err());
}
#[test]
fn payload_status_optional_fields_and_private_debug_policy() {
    let mut p = packet(vec![]);
    p["status"] = json!("payload-too-large");
    assert!(ready().push(&p.to_string()).unwrap());
    let mut p = packet(vec![initial()]);
    p["futureOptional"] = json!(true);
    assert!(!replica().push(&p.to_string()).unwrap());
    let mut s = ready().snapshot().unwrap();
    s.redact_titles();
    assert_eq!(s.state.windows[0].title.as_deref(), Some("<redacted>"));
}

#[test]
fn reload_hello_clears_state_even_with_same_session_token() {
    let mut r = ready();
    r.hello("test-session").unwrap();
    assert_eq!(r.health(), H::AwaitingSnapshot);
    assert!(r.snapshot().is_err());
    assert!(!r.push(&packet(vec![initial()]).to_string()).unwrap());
    r.hello("replacement").unwrap();
    assert!(r.snapshot().is_err());
}
#[test]
fn batched_events_have_same_result_as_individual_events() {
    let t = trace();
    let mut r = replica();
    assert!(
        !r.push(&packet(t["frames"].as_array().unwrap().clone()).to_string())
            .unwrap()
    );
    let expected: depthos_observer::NativeState =
        serde_json::from_value(t["final"]["state"].clone()).unwrap();
    assert!(r.snapshot().unwrap().state == expected);
}
#[test]
fn bounded_metadata_and_hostile_types_are_rejected() {
    for (key, value) in [
        ("title", json!("x".repeat(4097))),
        ("minimized", json!(1)),
        ("classification", json!("invented")),
        ("geometry", json!({"x":null,"y":0,"width":1,"height":1})),
        ("desktopIds", json!([5])),
    ] {
        let mut p = packet(vec![initial()]);
        p["frames"][0]["state"]["windows"][0][key] = value;
        assert!(replica().push(&p.to_string()).is_err());
    }
    let mut p = packet(vec![initial()]);
    p["frames"][0]["state"]["windows"][0]["typeFlags"] = json!(vec![false; 4097]);
    assert!(replica().push(&p.to_string()).is_err());
    let mut p = packet(vec![initial()]);
    p["status"] = json!("payload-too-large");
    assert!(replica().push(&p.to_string()).is_err());
}
#[test]
fn stale_consumer_malformed_contents_do_not_invalidate_current_state() {
    let mut r = ready();
    assert!(
        r.push(r#"{"consumerId":"old","frames":"invalid"}"#)
            .is_err()
    );
    assert_eq!(r.health(), H::Synchronized);
}
#[test]
fn unknown_native_topology_and_null_focus_are_valid() {
    let mut p = packet(vec![initial()]);
    for key in [
        "desktops",
        "activities",
        "screens",
        "activeDesktopId",
        "activeActivityId",
        "activeWindowId",
        "activeDesktopByScreen",
    ] {
        p["frames"][0]["state"][key] = Value::Null;
    }
    let mut r = replica();
    assert!(!r.push(&p.to_string()).unwrap());
    let snapshot = r.snapshot().unwrap();
    assert!(snapshot.state.screens.is_none());
    assert!(snapshot.state.active_window_id.is_none());
    let f = json!({"schemaVersion":1,"sessionId":"test-session","sequence":snapshot.sequence+1,"revision":snapshot.revision+1,"type":"window.focus.changed","activeWindowId":null});
    assert!(!r.push(&packet(vec![f]).to_string()).unwrap());
}
