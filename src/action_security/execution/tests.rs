use super::*;

fn fixture() -> (AuthorityFixture, WindowKey) {
    let (auth, client, world, request) = super::super::tests::fixture();
    (
        AuthorityFixture {
            auth,
            client,
            world,
            input: InputContext {
                lock: Some(false),
                seat: Some("fixture-seat".into()),
                focused_prompt: Some(Uuid::from_u128(1)),
            },
            pending: None,
            sink: CountingSink {
                count: 0,
                receipt: Receipt::Recorded,
            },
            stopped: false,
        },
        request.window,
    )
}

fn present(authority: &mut AuthorityFixture, window: &WindowKey) -> FixtureConfirmation {
    FixtureConfirmation {
        proposal: authority
            .present(window, NativeDesktop("desktop-b".into()), Tick(1))
            .unwrap(),
    }
}

#[test]
fn check_consume_and_count_once_with_no_world_changes() {
    let (mut authority, window) = fixture();
    let state_before = authority.world.windows[&window].clone();
    let version_before = authority.world.version.clone();
    let marker = present(&mut authority, &window);
    assert_eq!(
        authority.confirm(marker, Tick(2)),
        Ok(Execution::WouldDispatch)
    );
    assert_eq!(
        authority.confirm(marker, Tick(3)),
        Err(Error::UnknownAction)
    );
    assert_eq!(authority.sink.count, 1);
    assert_eq!(authority.world.windows[&window], state_before);
    assert_eq!(authority.world.version, version_before);
    assert_eq!(
        authority
            .auth
            .inspect(&authority.client, marker.proposal, Tick(3)),
        Ok(State::WouldDispatch)
    );
}

#[test]
fn lost_ack_is_uncertain_and_does_not_retry() {
    let (mut authority, window) = fixture();
    authority.sink.receipt = Receipt::Lost;
    let marker = present(&mut authority, &window);
    assert_eq!(
        authority.confirm(marker, Tick(2)),
        Ok(Execution::Uncertain(Uncertainty::NoAcknowledgement))
    );
    assert_eq!(
        authority.confirm(marker, Tick(3)),
        Err(Error::UnknownAction)
    );
    assert_eq!(authority.sink.count, 1);
    assert_eq!(
        authority
            .auth
            .inspect(&authority.client, marker.proposal, Tick(3)),
        Ok(State::Uncertain(Uncertainty::NoAcknowledgement))
    );
}

#[test]
fn cancellation_is_scoped_and_terminal_for_the_old_prompt() {
    let (mut authority, window) = fixture();
    let old = present(&mut authority, &window);
    authority.cancel(old.proposal);
    assert_eq!(authority.confirm(old, Tick(1)), Err(Error::UnknownAction));
    assert_eq!(authority.sink.count, 0);

    let new = present(&mut authority, &window);
    authority.cancel(old.proposal);
    assert_eq!(authority.confirm(old, Tick(2)), Err(Error::UnknownAction));
    assert_eq!(
        authority.confirm(new, Tick(2)),
        Ok(Execution::WouldDispatch)
    );
    authority.cancel(new.proposal);
    assert_eq!(authority.sink.count, 1);
}

#[test]
fn live_checkpoint_changes_before_consumption_deny_dispatch() {
    let (mut authority, window) = fixture();
    let marker = present(&mut authority, &window);
    authority.world.version.policy_revision += 1;
    assert_eq!(authority.confirm(marker, Tick(2)), Err(Error::StaleVersion));
    assert_eq!(
        authority.confirm(marker, Tick(2)),
        Err(Error::UnknownAction)
    );
    assert_eq!(authority.sink.count, 0);
}

#[test]
fn expired_prompt_is_discarded_without_confirmation() {
    let (mut authority, window) = fixture();
    let marker = present(&mut authority, &window);
    let expiry = authority.pending.as_ref().unwrap().proposal.expires();
    authority.expire(expiry).unwrap();
    assert_eq!(authority.confirm(marker, expiry), Err(Error::UnknownAction));
    assert_eq!(authority.sink.count, 0);
}

#[test]
fn a_seat_focus_or_lock_transition_invalidates_even_if_original_context_returns() {
    let (mut authority, window) = fixture();
    let original = authority.input.clone();
    let variants = [
        InputContext {
            seat: Some("other-seat".into()),
            ..original.clone()
        },
        InputContext {
            focused_prompt: None,
            ..original.clone()
        },
        InputContext {
            focused_prompt: Some(Uuid::from_u128(2)),
            ..original.clone()
        },
        InputContext {
            lock: Some(true),
            ..original.clone()
        },
        InputContext {
            lock: None,
            ..original.clone()
        },
    ];
    for changed in variants {
        let marker = present(&mut authority, &window);
        authority.observe_input(changed);
        authority.observe_input(original.clone());
        assert_eq!(
            authority.confirm(marker, Tick(1)),
            Err(Error::UnknownAction)
        );
        assert_eq!(authority.sink.count, 0);
    }
}

#[test]
fn unknown_seat_focus_or_lock_prevents_presentation() {
    let (mut authority, window) = fixture();
    let original = authority.input.clone();
    let variants = [
        InputContext {
            seat: None,
            ..original.clone()
        },
        InputContext {
            focused_prompt: None,
            ..original.clone()
        },
        InputContext {
            lock: None,
            ..original.clone()
        },
        InputContext {
            lock: Some(true),
            ..original.clone()
        },
    ];
    for changed in variants {
        authority.observe_input(changed);
        assert_eq!(
            authority.present(&window, NativeDesktop("desktop-b".into()), Tick(1)),
            Err(Error::Unavailable)
        );
        assert_eq!(authority.sink.count, 0);
    }
}

#[test]
fn loss_or_teardown_removes_all_pending_execution() {
    let (mut authority, window) = fixture();
    let old = present(&mut authority, &window);
    authority.observe_loss(Tick(2)).unwrap();
    assert_eq!(authority.confirm(old, Tick(2)), Err(Error::UnknownAction));
    assert_eq!(
        authority.present(&window, NativeDesktop("desktop-b".into()), Tick(2)),
        Err(Error::Unavailable)
    );

    authority.world.synchronized = true;
    let id = authority
        .present(&window, NativeDesktop("desktop-b".into()), Tick(3))
        .unwrap();
    let new = FixtureConfirmation { proposal: id };
    // A failed clock check cannot reopen the stopped fixture.
    assert_eq!(authority.teardown(Tick(2)), Err(Error::ClockReversed));
    assert_eq!(authority.confirm(new, Tick(3)), Err(Error::UnknownAction));
    authority.world.synchronized = true;
    assert_eq!(
        authority.present(&window, NativeDesktop("desktop-b".into()), Tick(4)),
        Err(Error::Unavailable)
    );
    assert_eq!(authority.sink.count, 0);
}

#[test]
fn competing_cancel_and_confirmations_obey_authority_turn_order() {
    use std::{
        sync::{Barrier, Mutex},
        thread,
    };

    for _ in 0..16 {
        let (mut authority, window) = fixture();
        let marker = present(&mut authority, &window);
        let authority = Mutex::new(authority);
        let start = Barrier::new(3);
        let results = thread::scope(|scope| {
            let confirm = || {
                start.wait();
                authority.lock().unwrap().confirm(marker, Tick(2))
            };
            let first = scope.spawn(confirm);
            let second = scope.spawn(confirm);
            start.wait();
            authority.lock().unwrap().cancel(marker.proposal);
            [first.join().unwrap(), second.join().unwrap()]
        });

        let authority = authority.into_inner().unwrap();
        let mut recorded = 0;
        for result in results {
            match result {
                Ok(Execution::WouldDispatch) => recorded += 1,
                Err(Error::UnknownAction) => {}
                other => panic!("unexpected execution result {other:?}"),
            }
        }
        assert!(recorded <= 1);
        assert_eq!(authority.sink.count, recorded);
        assert!(authority.pending.is_none());
    }
}
