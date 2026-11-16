use super::*;
use crate::context::ObservationPosition;
fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}
fn ws(n: u128) -> WorkspaceId {
    WorkspaceId(id(n))
}
pub(super) fn fixture() -> (Authorizer, Client, World, ActionRequest) {
    let a = Authorizer::new();
    let c = Client {
        bus_id: id(1),
        unique_name: ":1.2".into(),
        uid: 1000,
    };
    let compositor = Compositor {
        bus_id: id(1),
        unique_owner: ":1.1".into(),
        incarnation: id(2),
    };
    let version = Version {
        epoch: id(3),
        observation: ObservationPosition {
            observer_epoch: "observer-epoch-1".into(),
            session_id: "session-1".into(),
            sequence: 10,
            revision: 8,
        },
        policy_revision: 2,
    };
    let key = WindowKey {
        epoch: id(3),
        runtime_id: "runtime-1".into(),
        generation: "generation-1".into(),
    };
    let window = WindowState {
        native_uuid: id(4),
        eligible: true,
        visible: true,
        capture_allowed: Some(true),
        placement: Placement {
            desktop: NativeDesktop("desktop-a".into()),
            activity: "activity-a".into(),
        },
        logical: [ws(1)].into(),
    };
    let world = World {
        synchronized: true,
        compositor: compositor.clone(),
        version: version.clone(),
        topology_revision: id(5),
        windows: [(key.clone(), window)].into(),
        desktops: [
            NativeDesktop("desktop-a".into()),
            NativeDesktop("desktop-b".into()),
        ]
        .into(),
        workspaces: [ws(1), ws(2), ws(3)].into(),
    };
    let request = ActionRequest {
        id: RequestId(id(6)),
        window: key,
        native_uuid: id(4),
        compositor,
        expected: version,
        topology_revision: id(5),
        operation: Operation::Associate {
            from: [ws(1)].into(),
            to: [ws(2)].into(),
        },
    };
    (a, c, world, request)
}
// TEST-ONLY TRUST ORACLE. This is not a real click verifier or a production API.
fn consent(a: &mut Authorizer, c: &Client, r: &ActionRequest, now: Tick) -> Consent {
    let nonce = a.nonce().unwrap();
    Consent {
        authority: a.authority.clone(),
        nonce,
        request: r.clone(),
        client: c.clone(),
        issued: now,
        expires: Tick(now.0 + CONSENT_TTL_MS),
    }
}
fn copy_consent(p: &Consent) -> Consent {
    Consent {
        authority: p.authority.clone(),
        nonce: p.nonce,
        request: p.request.clone(),
        client: p.client.clone(),
        issued: p.issued,
        expires: p.expires,
    }
}
fn copy_cap(p: &Capability) -> Capability {
    Capability {
        authority: p.authority.clone(),
        nonce: p.nonce,
        id: p.id,
    }
}
fn grant(a: &mut Authorizer, c: &Client, w: &World, r: ActionRequest) -> Capability {
    let p = consent(a, c, &r, Tick(0));
    a.authorize(c, r, p, w, Tick(0)).unwrap()
}
fn moving(mut r: ActionRequest, w: &World) -> ActionRequest {
    let from = w.windows[&r.window].placement.clone();
    let to = Placement {
        desktop: NativeDesktop("desktop-b".into()),
        activity: from.activity.clone(),
    };
    r.operation = Operation::MoveDesktop { from, to };
    r
}
#[test]
fn same_uid_names_app_claims_and_claimed_click_cannot_mint_consent() {
    let (a, c, _, _) = fixture();
    for name in [":1.2", ":1.1", ":1.999"] {
        let mut caller = c.clone();
        caller.unique_name = name.into();
        for click in [false, true] {
            assert!(matches!(
                a.verify_interaction_claim(&caller, click),
                Err(Error::UnverifiedInteraction)
            ));
        }
    }
}
#[test]
fn approval_cannot_cross_authority_client_bus_or_uid() {
    for variant in 0..4 {
        let (mut a, c, w, r) = fixture();
        let p = consent(&mut a, &c, &r, Tick(0));
        let mut forged = c.clone();
        let expected = match variant {
            0 => {
                forged.uid += 1;
                Error::WrongClient
            }
            1 => {
                forged.unique_name = ":1.9".into();
                Error::WrongClient
            }
            2 => {
                forged.bus_id = id(9);
                Error::WrongClient
            }
            _ => Error::WrongAuthority,
        };
        if variant == 3 {
            a = Authorizer::new();
        }
        assert_eq!(
            a.authorize(&forged, r, p, &w, Tick(0)).unwrap_err(),
            expected
        );
    }
}
#[test]
fn full_operation_source_destination_and_revision_are_consent_bound() {
    for variant in 0..6 {
        let (mut a, c, w, mut r) = fixture();
        let p = consent(&mut a, &c, &r, Tick(0));
        match variant {
            0 => {
                r.operation = Operation::Associate {
                    from: [ws(1)].into(),
                    to: [ws(3)].into(),
                }
            }
            1 => r.operation = moving(r.clone(), &w).operation,
            2 => r.window.generation = "replaced".into(),
            3 => r.expected.policy_revision += 1,
            4 => r.native_uuid = id(99),
            _ => r.id = RequestId(id(99)),
        }
        assert_eq!(
            a.authorize(&c, r, p, &w, Tick(0)).unwrap_err(),
            Error::ConsentScope
        );
    }
}
#[test]
fn replayed_consent_and_duplicate_request_ids_are_refused() {
    let (mut a, c, w, r) = fixture();
    let p = consent(&mut a, &c, &r, Tick(0));
    let replay = copy_consent(&p);
    a.authorize(&c, r.clone(), p, &w, Tick(0)).unwrap();
    assert_eq!(
        a.authorize(&c, r.clone(), replay, &w, Tick(0)).unwrap_err(),
        Error::Replay
    );
    let p = consent(&mut a, &c, &r, Tick(0));
    assert_eq!(
        a.authorize(&c, r, p, &w, Tick(0)).unwrap_err(),
        Error::DuplicateRequest
    );
}
#[test]
fn expires_exactly_at_deadline_and_time_cannot_go_backwards() {
    let (mut a, c, w, r) = fixture();
    let p = consent(&mut a, &c, &r, Tick(0));
    assert_eq!(
        a.authorize(&c, r.clone(), p, &w, Tick(CONSENT_TTL_MS))
            .unwrap_err(),
        Error::Expired
    );
    assert_eq!(a.advance(Tick(1)), Err(Error::ClockReversed));
    let (mut a, c, mut w, r) = fixture();
    let cap = grant(&mut a, &c, &w, r.clone());
    assert_eq!(
        a.inspect(&c, r.id, Tick(CONSENT_TTL_MS)).unwrap(),
        State::Expired
    );
    assert_eq!(
        a.execute_synthetic(
            cap,
            &c,
            &mut w,
            Tick(CONSENT_TTL_MS),
            SyntheticOutcome::ApplyAndAck
        ),
        Err(Error::Replay)
    );
}
#[test]
fn excessive_or_future_consent_lifetime_is_refused() {
    for future in [false, true] {
        let (mut a, c, w, r) = fixture();
        let mut p = consent(&mut a, &c, &r, Tick(0));
        if future {
            p.issued = Tick(10);
        } else {
            p.expires = Tick(CONSENT_TTL_MS + 1);
        }
        assert_eq!(
            a.authorize(&c, r, p, &w, Tick(0)).unwrap_err(),
            Error::Expired
        );
    }
}
#[test]
fn conflicting_clients_cannot_race_same_window_or_read_revoke_each_other() {
    let (mut a, c, w, r) = fixture();
    let _cap = grant(&mut a, &c, &w, r.clone());
    let mut other = c.clone();
    other.unique_name = ":1.3".into();
    assert_eq!(a.inspect(&other, r.id, Tick(0)), Err(Error::WrongClient));
    assert_eq!(a.revoke(&other, r.id, Tick(0)), Err(Error::WrongClient));
    let mut second = r.clone();
    second.id = RequestId(id(20));
    let p = consent(&mut a, &other, &second, Tick(0));
    assert_eq!(
        a.authorize(&other, second.clone(), p, &w, Tick(0))
            .unwrap_err(),
        Error::Conflict
    );
    a.revoke(&c, r.id, Tick(0)).unwrap();
    grant(&mut a, &other, &w, second);
}
#[test]
fn every_precondition_is_rechecked_at_dispatch_and_failed_grant_is_spent() {
    for variant in 0..13 {
        let (mut a, c, mut w, r) = fixture();
        let cap = grant(&mut a, &c, &w, r.clone());
        let replay = copy_cap(&cap);
        match variant {
            0 => w.synchronized = false,
            1 => w.version.epoch = id(33),
            2 => w.version.observation.observer_epoch = "new".into(),
            3 => w.version.observation.session_id = "new".into(),
            4 => w.compositor.incarnation = id(34),
            5 => w.compositor.unique_owner = ":1.200".into(),
            6 => w.windows.get_mut(&r.window).unwrap().native_uuid = id(35),
            7 => {
                w.windows.remove(&r.window);
            }
            8 => w.version.observation.sequence += 1,
            9 => w.version.policy_revision += 1,
            10 => w.topology_revision = id(36),
            11 => w.windows.get_mut(&r.window).unwrap().logical.clear(),
            _ => {
                w.workspaces.remove(&ws(2));
            }
        }
        let before = w.windows.clone();
        assert!(
            a.execute_synthetic(cap, &c, &mut w, Tick(0), SyntheticOutcome::ApplyAndAck)
                .is_err()
        );
        assert_eq!(w.windows, before);
        assert_eq!(a.inspect(&c, r.id, Tick(0)).unwrap(), State::Invalidated);
        assert_eq!(
            a.execute_synthetic(replay, &c, &mut w, Tick(0), SyntheticOutcome::ApplyAndAck),
            Err(Error::Replay)
        );
    }
}
#[test]
fn hidden_protected_unknown_and_ineligible_windows_never_authorize() {
    for variant in 0..4 {
        let (mut a, c, mut w, r) = fixture();
        let s = w.windows.get_mut(&r.window).unwrap();
        match variant {
            0 => s.visible = false,
            1 => s.capture_allowed = Some(false),
            2 => s.capture_allowed = None,
            _ => s.eligible = false,
        }
        let p = consent(&mut a, &c, &r, Tick(0));
        assert_eq!(
            a.authorize(&c, r, p, &w, Tick(0)).unwrap_err(),
            Error::Ineligible
        );
    }
}
#[test]
fn removed_recreated_window_with_same_uuid_never_inherits_capability() {
    let (mut a, c, mut w, r) = fixture();
    let cap = grant(&mut a, &c, &w, r.clone());
    let s = w.windows.remove(&r.window).unwrap();
    let mut replacement = r.window.clone();
    replacement.generation = "generation-2".into();
    w.windows.insert(replacement, s);
    assert_eq!(
        a.execute_synthetic(cap, &c, &mut w, Tick(0), SyntheticOutcome::ApplyAndAck),
        Err(Error::Replaced)
    );
}
#[test]
fn synthetic_association_changes_no_native_state_and_is_one_shot() {
    let (mut a, c, mut w, r) = fixture();
    let native = w.windows[&r.window].placement.clone();
    let v = w.version.observation.clone();
    let cap = grant(&mut a, &c, &w, r.clone());
    let replay = copy_cap(&cap);
    assert_eq!(format!("{cap:?}"), "Capability([redacted])");
    assert_eq!(
        a.execute_synthetic(cap, &c, &mut w, Tick(0), SyntheticOutcome::ApplyAndAck)
            .unwrap(),
        State::Succeeded
    );
    assert_eq!(w.windows[&r.window].placement, native);
    assert_eq!(w.version.observation, v);
    assert_eq!(w.windows[&r.window].logical, [ws(2)].into());
    assert_eq!(
        a.execute_synthetic(replay, &c, &mut w, Tick(0), SyntheticOutcome::ApplyAndAck),
        Err(Error::Replay)
    );
}
#[test]
fn synthetic_native_move_is_separate_and_shared_assignment_requires_exact_approval() {
    let (mut a, c, mut w, r) = fixture();
    let r = moving(r, &w);
    let before = w.windows[&r.window].logical.clone();
    let cap = grant(&mut a, &c, &w, r.clone());
    assert_eq!(
        a.execute_synthetic(cap, &c, &mut w, Tick(0), SyntheticOutcome::ApplyAndAck)
            .unwrap(),
        State::Succeeded
    );
    assert_eq!(w.windows[&r.window].logical, before);
    assert_eq!(w.windows[&r.window].placement.desktop.0, "desktop-b");
    let (mut a, c, mut w, mut r) = fixture();
    r.operation = Operation::Associate {
        from: [ws(1)].into(),
        to: [ws(2), ws(3)].into(),
    };
    let cap = grant(&mut a, &c, &w, r.clone());
    a.execute_synthetic(cap, &c, &mut w, Tick(0), SyntheticOutcome::ApplyAndAck)
        .unwrap();
    assert_eq!(w.windows[&r.window].logical, [ws(2), ws(3)].into());
}
#[test]
fn no_implicit_activity_switch_and_destination_deletion_is_refused() {
    for deleted in [false, true] {
        let (mut a, c, mut w, r) = fixture();
        let mut r = moving(r, &w);
        if deleted {
            w.desktops.remove(&NativeDesktop("desktop-b".into()));
        } else if let Operation::MoveDesktop { to, .. } = &mut r.operation {
            to.activity = "different".into();
        }
        let p = consent(&mut a, &c, &r, Tick(0));
        assert_eq!(
            a.authorize(&c, r, p, &w, Tick(0)).unwrap_err(),
            if deleted {
                Error::DestinationMissing
            } else {
                Error::InvalidRequest
            }
        );
    }
}
#[test]
fn ack_is_not_a_postcondition_and_verified_rejection_has_no_effect() {
    for (mode, state) in [
        (
            SyntheticOutcome::AckWithoutEffect,
            State::Uncertain(Uncertainty::PostconditionMismatch),
        ),
        (SyntheticOutcome::RejectWithoutEffect, State::Rejected),
    ] {
        let (mut a, c, mut w, r) = fixture();
        let cap = grant(&mut a, &c, &w, r.clone());
        let before = w.windows.clone();
        assert_eq!(
            a.execute_synthetic(cap, &c, &mut w, Tick(0), mode).unwrap(),
            state
        );
        assert_eq!(w.windows, before);
    }
}
#[test]
fn lost_ack_partial_execution_and_uncertain_actions_are_never_retried() {
    for mode in [
        SyntheticOutcome::ApplyWithoutAck,
        SyntheticOutcome::PartialExecution,
    ] {
        let (mut a, c, mut w, r) = fixture();
        let cap = grant(&mut a, &c, &w, r.clone());
        assert!(matches!(
            a.execute_synthetic(cap, &c, &mut w, Tick(0), mode).unwrap(),
            State::Uncertain(_)
        ));
        let mut next = r.clone();
        next.id = RequestId(id(99));
        next.expected = w.version.clone();
        next.operation = Operation::Associate {
            from: [ws(2)].into(),
            to: [ws(3)].into(),
        };
        let p = consent(&mut a, &c, &next, Tick(0));
        let e = a.authorize(&c, next, p, &w, Tick(0)).unwrap_err();
        assert!(matches!(e, Error::Conflict | Error::Ineligible));
        assert!(matches!(
            a.revoke(&c, r.id, Tick(0)).unwrap(),
            State::Uncertain(_)
        ));
    }
}
#[test]
fn cancellation_before_dispatch_and_after_dispatch_have_distinct_truthful_results() {
    let (mut a, c, mut w, r) = fixture();
    let cap = grant(&mut a, &c, &w, r.clone());
    assert_eq!(a.revoke(&c, r.id, Tick(0)).unwrap(), State::Cancelled);
    assert_eq!(
        a.execute_synthetic(cap, &c, &mut w, Tick(0), SyntheticOutcome::ApplyAndAck),
        Err(Error::Replay)
    );
    let (mut a, c, mut w, r) = fixture();
    let cap = grant(&mut a, &c, &w, r.clone());
    a.execute_synthetic(cap, &c, &mut w, Tick(0), SyntheticOutcome::Defer)
        .unwrap();
    assert_eq!(
        a.revoke(&c, r.id, Tick(1)).unwrap(),
        State::Uncertain(Uncertainty::CancelledAfterDispatch)
    );
}
#[test]
fn timeout_late_ack_and_wrong_attempt_never_become_success() {
    let (mut a, c, w, r) = fixture();
    let cap = grant(&mut a, &c, &w, r.clone());
    let ticket = a.begin(cap, &c, &w, Tick(0)).unwrap();
    let wrong = Ticket {
        authority: ticket.authority.clone(),
        id: ticket.id,
        nonce: ticket.nonce + 1,
    };
    assert_eq!(
        a.finish(wrong, Ack::Applied, &w, Tick(0)),
        Err(Error::Replay)
    );
    assert_eq!(
        a.inspect(&c, r.id, Tick(ACK_TTL_MS)).unwrap(),
        State::Uncertain(Uncertainty::Timeout)
    );
    assert_eq!(
        a.finish(ticket, Ack::Applied, &w, Tick(ACK_TTL_MS)),
        Err(Error::InvalidState)
    );
}
#[test]
fn epoch_loss_after_dispatch_is_uncertain_and_postcondition_requires_fresh_evidence() {
    let (mut a, c, mut w, r) = fixture();
    let cap = grant(&mut a, &c, &w, r.clone());
    let ticket = a.begin(cap, &c, &w, Tick(0)).unwrap();
    w.version.observation.observer_epoch = "new".into();
    assert_eq!(
        a.finish(ticket, Ack::Applied, &w, Tick(0)).unwrap(),
        State::Uncertain(Uncertainty::PostconditionMismatch)
    );
    assert_eq!(
        a.reconcile_synthetic(&c, r.id, &w, Tick(1)),
        Err(Error::Replaced)
    );
    let (mut a, c, mut w, r) = fixture();
    let cap = grant(&mut a, &c, &w, r.clone());
    a.execute_synthetic(cap, &c, &mut w, Tick(0), SyntheticOutcome::ApplyWithoutAck)
        .unwrap();
    assert_eq!(
        a.reconcile_synthetic(&c, r.id, &w, Tick(1)),
        Err(Error::StaleVersion)
    );
    w.version.observation.sequence += 1;
    assert_eq!(
        a.reconcile_synthetic(&c, r.id, &w, Tick(1)).unwrap(),
        State::Reconciled {
            destination_observed: true
        }
    );
}
#[test]
fn disconnect_and_global_invalidation_revoke_only_unused_actions() {
    for disconnected in [false, true] {
        let (mut a, c, w, r) = fixture();
        let cap = grant(&mut a, &c, &w, r.clone());
        if disconnected {
            a.client_disconnected(&c, Tick(0)).unwrap();
        } else {
            a.invalidate(Tick(0)).unwrap();
        }
        assert_eq!(
            a.inspect(&c, r.id, Tick(0)).unwrap(),
            if disconnected {
                State::Cancelled
            } else {
                State::Invalidated
            }
        );
        drop(cap);
        let (mut a, c, w, r) = fixture();
        let cap = grant(&mut a, &c, &w, r.clone());
        a.begin(cap, &c, &w, Tick(0)).unwrap();
        a.invalidate(Tick(0)).unwrap();
        assert_eq!(
            a.inspect(&c, r.id, Tick(0)).unwrap(),
            State::Uncertain(Uncertainty::Replaced)
        );
    }
}
#[test]
fn retained_tombstones_are_bounded_and_prevent_replay_after_cancellation() {
    let (mut a, c, w, r) = fixture();
    for i in 0..MAX_ACTIONS {
        let mut r = r.clone();
        r.id = RequestId(id(100 + i as u128));
        let cap = grant(&mut a, &c, &w, r.clone());
        a.revoke(&c, r.id, Tick(0)).unwrap();
        drop(cap);
    }
    let p = consent(&mut a, &c, &r, Tick(0));
    assert_eq!(
        a.authorize(&c, r, p, &w, Tick(0)).unwrap_err(),
        Error::Capacity
    );
    assert_eq!(a.records.len(), MAX_ACTIONS);
}
#[test]
fn oversized_ids_and_counter_exhaustion_fail_closed() {
    let (mut a, c, w, mut r) = fixture();
    r.window.runtime_id = "x".repeat(513);
    let p = consent(&mut a, &c, &r, Tick(0));
    assert_eq!(
        a.authorize(&c, r, p, &w, Tick(0)).unwrap_err(),
        Error::InvalidRequest
    );
    let (mut a, c, mut w, mut r) = fixture();
    w.version.policy_revision = u64::MAX;
    r.expected = w.version.clone();
    let p = consent(&mut a, &c, &r, Tick(0));
    assert_eq!(
        a.authorize(&c, r, p, &w, Tick(0)).unwrap_err(),
        Error::CounterExhausted
    );
}

#[test]
fn ambiguous_uuid_or_runtime_mapping_refuses_authorization() {
    for same_runtime in [false, true] {
        let (mut a, c, mut w, r) = fixture();
        let mut other = r.window.clone();
        if same_runtime {
            other.generation = "other-generation".into();
        } else {
            other.runtime_id = "other-runtime".into();
        }
        w.windows.insert(other, w.windows[&r.window].clone());
        let p = consent(&mut a, &c, &r, Tick(0));
        assert_eq!(
            a.authorize(&c, r, p, &w, Tick(0)).unwrap_err(),
            Error::Replaced
        );
    }
}
#[test]
fn preconditions_also_apply_before_grant_and_bad_approval_cannot_be_repaired() {
    let (mut a, c, mut w, r) = fixture();
    w.version.policy_revision += 1;
    let p = consent(&mut a, &c, &r, Tick(0));
    let replay = copy_consent(&p);
    assert_eq!(
        a.authorize(&c, r.clone(), p, &w, Tick(0)).unwrap_err(),
        Error::StaleVersion
    );
    w.version = r.expected.clone();
    assert_eq!(
        a.authorize(&c, r, replay, &w, Tick(0)).unwrap_err(),
        Error::Replay
    );
}
#[test]
fn dispatcher_cannot_use_another_authoritys_handle() {
    let (mut a, c, w, r) = fixture();
    let cap = grant(&mut a, &c, &w, r);
    let mut other = Authorizer::new();
    assert!(matches!(
        other.begin(cap, &c, &w, Tick(0)),
        Err(Error::WrongAuthority)
    ));
}
#[test]
fn explicit_unassignment_changes_only_logical_context() {
    let (mut a, c, mut w, mut r) = fixture();
    r.operation = Operation::Associate {
        from: [ws(1)].into(),
        to: BTreeSet::new(),
    };
    let cap = grant(&mut a, &c, &w, r.clone());
    assert_eq!(
        a.execute_synthetic(cap, &c, &mut w, Tick(0), SyntheticOutcome::ApplyAndAck)
            .unwrap(),
        State::Succeeded
    );
    assert!(w.windows[&r.window].logical.is_empty());
}

#[test]
fn changed_flags_destinations_and_native_source_are_rechecked_before_move() {
    for variant in 0..5 {
        let (mut a, c, mut w, r) = fixture();
        let r = moving(r, &w);
        let cap = grant(&mut a, &c, &w, r.clone());
        match variant {
            0 => w.windows.get_mut(&r.window).unwrap().visible = false,
            1 => w.windows.get_mut(&r.window).unwrap().capture_allowed = Some(false),
            2 => {
                w.desktops.remove(&NativeDesktop("desktop-b".into()));
            }
            3 => w.windows.get_mut(&r.window).unwrap().placement.activity = "other".into(),
            _ => w.version.observation.revision += 1,
        }
        let before = w.windows.clone();
        assert!(
            a.execute_synthetic(cap, &c, &mut w, Tick(0), SyntheticOutcome::ApplyAndAck)
                .is_err()
        );
        assert_eq!(w.windows, before);
    }
}
#[test]
fn ambiguity_introduced_after_dispatch_cannot_verify_success() {
    let (mut a, c, mut w, r) = fixture();
    let cap = grant(&mut a, &c, &w, r.clone());
    let ticket = a.begin(cap, &c, &w, Tick(0)).unwrap();
    w.windows.get_mut(&r.window).unwrap().logical = [ws(2)].into();
    w.version.policy_revision += 1;
    let mut other = r.window.clone();
    other.runtime_id = "duplicate-uuid".into();
    w.windows.insert(other, w.windows[&r.window].clone());
    assert_eq!(
        a.finish(ticket, Ack::Applied, &w, Tick(0)).unwrap(),
        State::Uncertain(Uncertainty::PostconditionMismatch)
    );
}
#[test]
fn clock_overflow_cannot_leave_a_reusable_authorization() {
    let (mut a, c, mut w, r) = fixture();
    let issued = Tick(u64::MAX - CONSENT_TTL_MS);
    let proof = consent(&mut a, &c, &r, issued);
    let cap = a.authorize(&c, r.clone(), proof, &w, issued).unwrap();
    let replay = copy_cap(&cap);
    assert_eq!(
        a.execute_synthetic(
            cap,
            &c,
            &mut w,
            Tick(u64::MAX - 10),
            SyntheticOutcome::ApplyAndAck
        ),
        Err(Error::CounterExhausted)
    );
    assert_eq!(
        a.inspect(&c, r.id, Tick(u64::MAX - 10)).unwrap(),
        State::Invalidated
    );
    assert_eq!(
        a.execute_synthetic(
            replay,
            &c,
            &mut w,
            Tick(u64::MAX - 10),
            SyntheticOutcome::ApplyAndAck
        ),
        Err(Error::Replay)
    );
}
