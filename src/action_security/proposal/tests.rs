use super::*;
use crate::action_security::{State, SyntheticOutcome, tests::fixture};

fn destination() -> NativeDesktop {
    NativeDesktop("desktop-b".into())
}

fn proposal(world: &World, key: &WindowKey) -> NativeMoveProposal {
    NativeMoveProposal::capture(world, key, destination(), Tick(10)).unwrap()
}

#[test]
fn capture_binds_version_identity_and_placement() {
    let (_, _, world, request) = fixture();
    let operation = proposal(&world, &request.window);
    let captured = operation.request();
    assert_eq!(captured.window, request.window);
    assert_eq!(captured.native_uuid, request.native_uuid);
    assert_eq!(captured.compositor, world.compositor);
    assert_eq!(captured.expected, world.version);
    assert_eq!(captured.topology_revision, world.topology_revision);
    assert_eq!(
        captured.operation,
        Operation::MoveDesktop {
            from: world.windows[&request.window].placement.clone(),
            to: Placement {
                desktop: destination(),
                activity: "activity-a".into(),
            },
        }
    );
    assert_eq!(operation.created(), Tick(10));
    assert_eq!(operation.expires(), Tick(10 + CONSENT_TTL_MS));
}

#[test]
fn capture_rejects_missing_or_ambiguous_identity() {
    let (_, _, baseline, request) = fixture();
    let mut world = baseline.clone();
    world.windows.get_mut(&request.window).unwrap().native_uuid = Uuid::nil();
    let capture =
        |world: &World| NativeMoveProposal::capture(world, &request.window, destination(), Tick(0));
    assert!(matches!(capture(&world), Err(Error::InvalidRequest)));

    let mut world = baseline.clone();
    let mut duplicate = request.window.clone();
    duplicate.runtime_id = "duplicate".into();
    world
        .windows
        .insert(duplicate, baseline.windows[&request.window].clone());
    assert!(matches!(capture(&world), Err(Error::Replaced)));

    let mut world = baseline.clone();
    let mut duplicate = request.window.clone();
    duplicate.generation = "duplicate".into();
    let mut state = baseline.windows[&request.window].clone();
    state.native_uuid = Uuid::from_u128(99);
    world.windows.insert(duplicate, state);
    assert!(matches!(capture(&world), Err(Error::Replaced)));
}

#[test]
fn revalidation_refuses_destruction_or_reincarnation() {
    let (_, _, mut world, request) = fixture();
    let operation = proposal(&world, &request.window);
    let state = world.windows.remove(&request.window).unwrap();
    assert_eq!(operation.revalidate(&world, Tick(11)), Err(Error::Replaced));

    let mut recreated = request.window;
    recreated.generation = "generation-2".into();
    world.windows.insert(recreated, state);
    assert_eq!(operation.revalidate(&world, Tick(11)), Err(Error::Replaced));
}

#[test]
fn revalidation_refuses_changed_placement_or_destination() {
    let (_, _, baseline, request) = fixture();
    let operation = proposal(&baseline, &request.window);
    let mut world = baseline.clone();
    world
        .windows
        .get_mut(&request.window)
        .unwrap()
        .placement
        .desktop = destination();
    assert_eq!(
        operation.revalidate(&world, Tick(11)),
        Err(Error::SourceChanged)
    );

    let mut world = baseline.clone();
    world
        .windows
        .get_mut(&request.window)
        .unwrap()
        .placement
        .activity = "other-activity".into();
    assert_eq!(
        operation.revalidate(&world, Tick(11)),
        Err(Error::SourceChanged)
    );

    let mut world = baseline;
    world.desktops.remove(&destination());
    assert_eq!(
        operation.revalidate(&world, Tick(11)),
        Err(Error::DestinationMissing)
    );
}

#[test]
fn revalidation_refuses_stale_authority_and_checkpoint() {
    let (_, _, baseline, request) = fixture();
    let operation = proposal(&baseline, &request.window);

    let mut world = baseline.clone();
    world.version.observation.observer_epoch = "other-epoch".into();
    assert_eq!(operation.revalidate(&world, Tick(11)), Err(Error::Replaced));

    let mut world = baseline.clone();
    world.version.observation.session_id = "other-session".into();
    assert_eq!(operation.revalidate(&world, Tick(11)), Err(Error::Replaced));

    let mut world = baseline.clone();
    world.compositor.incarnation = Uuid::from_u128(7);
    assert_eq!(operation.revalidate(&world, Tick(11)), Err(Error::Replaced));

    let mut world = baseline.clone();
    world.version.epoch = Uuid::from_u128(7);
    assert_eq!(operation.revalidate(&world, Tick(11)), Err(Error::Replaced));

    let mut world = baseline.clone();
    world.topology_revision = Uuid::from_u128(7);
    assert_eq!(
        operation.revalidate(&world, Tick(11)),
        Err(Error::StaleVersion)
    );

    let mut world = baseline.clone();
    world.version.observation.sequence += 1;
    assert_eq!(
        operation.revalidate(&world, Tick(11)),
        Err(Error::StaleVersion)
    );

    let mut world = baseline;
    world.version.policy_revision += 1;
    assert_eq!(
        operation.revalidate(&world, Tick(11)),
        Err(Error::StaleVersion)
    );
}

#[test]
fn fixture_confirmation_never_extends_proposal_deadline() {
    let (mut authority, client, world, request) = fixture();
    let operation = proposal(&world, &request.window);
    assert_eq!(
        operation.revalidate(&world, Tick(9)),
        Err(Error::ClockReversed)
    );
    let deadline = operation.expires();
    assert_eq!(operation.revalidate(&world, deadline), Err(Error::Expired));
    let _cap = authority
        .authorize_native_fixture(&client, &operation, &world, Tick(deadline.0 - 1))
        .unwrap();
    assert_eq!(
        authority.inspect(&client, operation.request().id, deadline),
        Ok(State::Expired)
    );
    assert!(matches!(
        authority.authorize_native_fixture(&client, &operation, &world, deadline),
        Err(Error::Expired)
    ));
}

#[test]
fn cancelled_confirmation_cannot_dispatch_or_be_reissued() {
    let (mut authority, client, mut world, request) = fixture();
    let operation = proposal(&world, &request.window);
    let cap = authority
        .authorize_native_fixture(&client, &operation, &world, Tick(10))
        .unwrap();
    assert_eq!(
        authority.revoke(&client, operation.request().id, Tick(11)),
        Ok(State::Cancelled)
    );
    assert_eq!(
        authority.execute_synthetic(
            cap,
            &client,
            &mut world,
            Tick(11),
            SyntheticOutcome::ApplyAndAck
        ),
        Err(Error::Replay)
    );
    assert!(matches!(
        authority.authorize_native_fixture(&client, &operation, &world, Tick(11)),
        Err(Error::DuplicateRequest)
    ));
}
