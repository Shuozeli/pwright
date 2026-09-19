use super::*;
use crate::{ChromiumFrameWorldAccess, ChromiumFrameWorldAccessProfile, FrameWorldAccessError};

mod live;
mod numeric_identity;
mod retirement;
mod utility;

fn access_fixture(capacity: usize) -> (Fixture, ChromiumFrameWorldAccess) {
    let profile = ChromiumFrameWorldAccessProfile::new(
        ResourceScope::extension(98, 3),
        NonZeroUsize::new(capacity).unwrap(),
    )
    .unwrap();
    let fixture = fixture_with_world_access(true, Some(profile));
    let access =
        ChromiumFrameWorldAccess::new(&fixture.services, profile, fixture.prepared.resources())
            .unwrap();
    (fixture, access)
}
fn pair(fixture: &mut Fixture, frame: &str, first: i64) {
    attach_world(
        &mut fixture.topology,
        first,
        &format!("main-{first}"),
        frame,
        CdpWorldKind::Main,
    );
    attach_world(
        &mut fixture.topology,
        first + 1,
        &format!("utility-{first}"),
        frame,
        CdpWorldKind::Utility,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
}
fn poll<T>(future: Pin<&mut impl Future<Output = T>>) -> Poll<T> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}
fn bytes(fixture: &Fixture) -> u64 {
    scope_bytes(fixture, ResourceScope::extension(98, 3))
}
fn scope_bytes(fixture: &Fixture, scope: ResourceScope) -> u64 {
    fixture
        .prepared
        .resources()
        .claim(ResourceRequirements::none())
        .unwrap()
        .scope_metrics(scope)
        .unwrap()
        .map_or(0, |metrics| metrics.units(ResourceClass::BackendStateBytes))
}

#[tokio::test]
async fn snapshots_wait_for_both_worlds_and_keep_their_budget_after_readiness() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    let baseline = bytes(&fixture);
    attach_world(
        &mut fixture.topology,
        11,
        "main",
        "frame-a",
        CdpWorldKind::Main,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
    let mut pending = Box::pin(access.snapshot(ObjectId::new(4)));
    assert!(poll(pending.as_mut()).is_pending());
    // Act
    attach_world(
        &mut fixture.topology,
        12,
        "utility",
        "frame-a",
        CdpWorldKind::Utility,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
    let snapshot = pending.await.unwrap();
    // Assert
    assert_eq!(snapshot.frame(), ObjectId::new(4));
    assert_eq!(snapshot.page(), ObjectId::new(3));
    assert_eq!(snapshot.main_execution_context_id(), 11);
    assert_eq!(snapshot.utility_execution_context_id(), 12);
    assert_eq!(snapshot.main_unique_id(), "main");
    assert_eq!(snapshot.utility_unique_id(), "utility");
    assert!(snapshot.is_current().unwrap());
    assert_eq!(bytes(&fixture), baseline + 8192);
    assert!(fixture.factory.client("target-a").calls().is_empty());
    drop(snapshot);
    assert_eq!(bytes(&fixture), baseline);
}

#[tokio::test]
async fn installation_clones_share_capacity_for_unpolled_and_retained_snapshots() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    pair(&mut fixture, "frame-a", 11);
    let first = access.snapshot(ObjectId::new(4));
    let clone = access.clone();
    // Act
    let excess = clone.snapshot(ObjectId::new(4)).await;
    let retained = first.await.unwrap();
    let still_excess = clone.snapshot(ObjectId::new(4)).await;
    drop(retained);
    let replacement = clone.snapshot(ObjectId::new(4)).await;
    // Assert
    assert!(matches!(
        excess,
        Err(FrameWorldAccessError::Capacity { limit: 1 })
    ));
    assert!(matches!(
        still_excess,
        Err(FrameWorldAccessError::Capacity { limit: 1 })
    ));
    assert!(replacement.is_ok());
    assert!(fixture.factory.client("target-a").calls().is_empty());
}

#[tokio::test]
async fn dropping_unpolled_and_pending_snapshot_returns_all_admission() {
    // Arrange
    let (fixture, access) = access_fixture(1);
    let baseline = bytes(&fixture);
    let readiness_bytes = scope_bytes(&fixture, ResourceScope::extension(98, 2));
    let unpolled = access.snapshot(ObjectId::new(4));
    // Act
    drop(unpolled);
    let mut pending = Box::pin(access.snapshot(ObjectId::new(4)));
    assert!(poll(pending.as_mut()).is_pending());
    drop(pending);
    // Assert
    assert_eq!(bytes(&fixture), baseline);
    assert_eq!(
        scope_bytes(&fixture, ResourceScope::extension(98, 2)),
        readiness_bytes
    );
    assert!(fixture.factory.client("target-a").calls().is_empty());
    // A new wait admits after cancellation; no timer or driver was installed.
    drop(access.snapshot(ObjectId::new(4)));
    assert_eq!(bytes(&fixture), baseline);
}

#[tokio::test]
async fn page_ids_and_unrelated_frames_never_substitute_the_main_frame() {
    // Arrange
    let (mut fixture, access) = access_fixture(3);
    pair(&mut fixture, "frame-a", 11);
    let baseline = bytes(&fixture);
    // Act
    let page = access.snapshot(ObjectId::new(3)).await;
    let unknown = access.snapshot(ObjectId::new(99)).await;
    let mut child = Box::pin(access.snapshot(ObjectId::new(5)));
    let child_state = poll(child.as_mut());
    drop(child);
    // Assert
    assert!(matches!(
        page,
        Err(FrameWorldAccessError::Topology(
            CdpPageRouteError::WorldTargetClosed { .. }
        ))
    ));
    assert!(matches!(
        unknown,
        Err(FrameWorldAccessError::Topology(
            CdpPageRouteError::WorldTargetClosed { .. }
        ))
    ));
    assert!(child_state.is_pending());
    assert_eq!(bytes(&fixture), baseline);
}

#[tokio::test]
async fn local_child_uses_its_own_worlds_and_the_same_session_client_for_both() {
    // Arrange
    let (mut fixture, access) = access_fixture(2);
    pair(&mut fixture, "frame-a", 11);
    pair(&mut fixture, "child", 21);
    // Act
    let main = access.snapshot(ObjectId::new(4)).await.unwrap();
    let child = access.snapshot(ObjectId::new(5)).await.unwrap();
    // Assert
    assert_eq!(child.frame(), ObjectId::new(5));
    assert_eq!(child.page(), ObjectId::new(3));
    assert_eq!(child.main_unique_id(), "main-21");
    assert_eq!(child.main_execution_context_id(), 21);
    assert_eq!(child.utility_execution_context_id(), 22);
    assert_eq!(child.utility_unique_id(), "utility-21");
    assert!(Arc::ptr_eq(&main.client(), &child.client()));
}

#[tokio::test]
async fn replacing_either_world_invalidates_the_whole_pair() {
    // Arrange
    let (mut fixture, access) = access_fixture(2);
    pair(&mut fixture, "frame-a", 11);
    let first = access.snapshot(ObjectId::new(4)).await.unwrap();
    // Act
    attach_world(
        &mut fixture.topology,
        20,
        "new-main",
        "frame-a",
        CdpWorldKind::Main,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
    let second = access.snapshot(ObjectId::new(4)).await.unwrap();
    attach_world(
        &mut fixture.topology,
        21,
        "new-utility",
        "frame-a",
        CdpWorldKind::Utility,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
    // Assert
    assert!(!first.is_current().unwrap());
    assert!(!second.is_current().unwrap());
}

#[tokio::test]
async fn unrelated_topology_publications_do_not_invalidate_the_pair() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    pair(&mut fixture, "frame-a", 11);
    let snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    // Act
    pair(&mut fixture, "child", 21);
    // Assert
    assert!(snapshot.is_current().unwrap());
}

#[tokio::test]
async fn invalidation_wait_rechecks_commits_and_drops_without_losing_snapshot() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    pair(&mut fixture, "frame-a", 11);
    let mut snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    let baseline = bytes(&fixture);
    let readiness_bytes = scope_bytes(&fixture, ResourceScope::extension(98, 2));
    let mut wait = Box::pin(snapshot.wait_invalidated());
    assert!(poll(wait.as_mut()).is_pending());
    fixture.services.synchronize(&fixture.topology).unwrap();
    assert!(poll(wait.as_mut()).is_pending());
    drop(wait);
    assert_eq!(bytes(&fixture), baseline);
    assert_eq!(
        scope_bytes(&fixture, ResourceScope::extension(98, 2)),
        readiness_bytes
    );
    // Act
    let mut wait = Box::pin(snapshot.wait_invalidated());
    assert!(poll(wait.as_mut()).is_pending());
    fixture
        .topology
        .destroy_world(ConnectionEpoch::new(1), "session-a", 11)
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    let result = wait.await;
    // Assert
    assert!(result.is_ok());
    assert!(!snapshot.is_current().unwrap());
    assert_eq!(bytes(&fixture), baseline);
}

#[tokio::test]
async fn invalidation_registered_after_world_removal_completes_without_waiting() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    pair(&mut fixture, "frame-a", 11);
    let mut snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    fixture
        .topology
        .clear_worlds(ConnectionEpoch::new(1), "session-a")
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    // Act
    let result = poll(std::pin::pin!(snapshot.wait_invalidated()));
    // Assert
    assert!(matches!(result, Poll::Ready(Ok(()))));
}

#[tokio::test]
async fn detached_child_is_terminal_for_snapshot_and_invalidation_wait() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    pair(&mut fixture, "child", 21);
    let mut snapshot = access.snapshot(ObjectId::new(5)).await.unwrap();
    // Act
    fixture
        .topology
        .detach_frame(
            ConnectionEpoch::new(1),
            "child",
            CdpFrameDetachReason::Remove,
        )
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    let checked = snapshot.is_current();
    let waited = snapshot.wait_invalidated().await;
    // Assert
    assert!(matches!(
        checked,
        Err(FrameWorldAccessError::Topology(
            CdpPageRouteError::WorldTargetClosed { .. }
        ))
    ));
    assert!(matches!(
        waited,
        Err(FrameWorldAccessError::Topology(
            CdpPageRouteError::WorldTargetClosed { .. }
        ))
    ));
}

#[tokio::test]
async fn target_crash_ends_a_world_wait_without_misreporting_readiness() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    let baseline = bytes(&fixture);
    let mut pending = Box::pin(access.snapshot(ObjectId::new(4)));
    assert!(poll(pending.as_mut()).is_pending());
    // Act
    fixture
        .topology
        .crash_target(ConnectionEpoch::new(1), "target-a", "crashed", 1)
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    let result = pending.await;
    // Assert
    assert!(matches!(
        result,
        Err(FrameWorldAccessError::Topology(
            CdpPageRouteError::PageCrashed { .. }
        ))
    ));
    assert_eq!(bytes(&fixture), baseline);
}

#[test]
fn installation_rejects_missing_prerequisites_and_undeclared_resources() {
    // Arrange
    let profile = ChromiumFrameWorldAccessProfile::new(
        ResourceScope::extension(98, 3),
        NonZeroUsize::new(1).unwrap(),
    )
    .unwrap();
    let missing = fixture(false);
    let present = fixture(true);
    let empty =
        ProductionResourceLedger::new(ResourceLimits::deny_all(0, 0), ResourceRequirements::none())
            .unwrap();
    // Act
    let no_readiness =
        ChromiumFrameWorldAccess::new(&missing.services, profile, missing.prepared.resources());
    let no_resources = ChromiumFrameWorldAccess::new(&present.services, profile, &empty);
    // Assert
    assert!(matches!(no_readiness, Err(FrameWorldAccessError::Setup(_))));
    assert!(matches!(
        no_resources,
        Err(FrameWorldAccessError::Production(_))
    ));
}

#[tokio::test]
async fn last_snapshot_retains_its_service_and_resource_owners_after_accessor_drop() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    pair(&mut fixture, "frame-a", 11);
    let resources = fixture.prepared.resources().clone();
    let snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    drop(access);
    drop(fixture);
    // Act
    let current = snapshot.is_current();
    let retained = resources.metrics().unwrap().active_leases;
    drop(snapshot);
    // Assert
    assert!(current.unwrap());
    assert!(retained > 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn oopif_handoff_retires_old_routes_and_waits_for_a_same_session_pair() {
    // Arrange
    let (mut fixture, access) = access_fixture(2);
    pair(&mut fixture, "child", 21);
    let old = access.snapshot(ObjectId::new(5)).await.unwrap();
    // Act: no destroy event yet; the authoritative Frame host already changed.
    fixture
        .topology
        .attach_frame(
            ConnectionEpoch::new(1),
            CdpFrameAttachment::remote("target-a", "child", "frame-a", "remote-session"),
            ObjectId::new(5),
        )
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    let old_is_current = old.is_current().unwrap();
    fixture
        .topology
        .attach_world(
            ConnectionEpoch::new(1),
            CdpWorldAttachment::new(
                "remote-session",
                31,
                "remote-main",
                "child",
                CdpWorldKind::Main,
            ),
        )
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    let mut waiting = Box::pin(access.snapshot(ObjectId::new(5)));
    let partial = poll(waiting.as_mut());
    fixture
        .topology
        .attach_world(
            ConnectionEpoch::new(1),
            CdpWorldAttachment::new(
                "remote-session",
                32,
                "remote-utility",
                "child",
                CdpWorldKind::Utility,
            ),
        )
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    let new = waiting.await.unwrap();
    // Assert
    assert!(!old_is_current);
    assert!(partial.is_pending());
    assert_eq!(new.main_unique_id(), "remote-main");
    assert_eq!(new.main_execution_context_id(), 31);
    assert_eq!(new.utility_execution_context_id(), 32);
    assert_eq!(new.utility_unique_id(), "remote-utility");
    assert!(!Arc::ptr_eq(&old.client(), &new.client()));
    assert!(new.is_current().unwrap());
}

#[tokio::test]
async fn swap_gap_has_no_usable_world_pair_even_before_old_world_destruction() {
    // Arrange
    let (mut fixture, access) = access_fixture(2);
    pair(&mut fixture, "child", 21);
    let snapshot = access.snapshot(ObjectId::new(5)).await.unwrap();
    // Act
    fixture
        .topology
        .detach_frame(ConnectionEpoch::new(1), "child", CdpFrameDetachReason::Swap)
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    let current = snapshot.is_current().unwrap();
    let mut next = Box::pin(access.snapshot(ObjectId::new(5)));
    let next_state = poll(next.as_mut());
    // Assert
    assert!(!current);
    assert!(next_state.is_pending());
    assert_eq!(
        fixture
            .topology
            .topology()
            .world_count_for_frame(ObjectId::new(5)),
        2,
        "route retirement is not a claim that the old native worlds were destroyed"
    );
}

#[tokio::test]
async fn rolled_back_world_pair_publication_does_not_invalidate_an_existing_snapshot() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    pair(&mut fixture, "frame-a", 11);
    let snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    attach_world(
        &mut fixture.topology,
        21,
        "unpublished-main",
        "frame-a",
        CdpWorldKind::Main,
    );
    // Act
    drop(
        fixture
            .services
            .prepare_synchronization(fixture.topology.topology())
            .unwrap(),
    );
    // Assert
    assert!(snapshot.is_current().unwrap());
    assert_eq!(snapshot.main_unique_id(), "main-11");
}

#[tokio::test]
async fn an_observed_retired_snapshot_cannot_revive_when_the_old_route_reappears() {
    // Arrange
    let (mut fixture, access) = access_fixture(2);
    pair(&mut fixture, "child", 21);
    let old = access.snapshot(ObjectId::new(5)).await.unwrap();
    fixture
        .topology
        .detach_frame(ConnectionEpoch::new(1), "child", CdpFrameDetachReason::Swap)
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    assert!(!old.is_current().unwrap());
    // Act: model a same-process rebind before native world destruction.
    fixture
        .topology
        .attach_frame(
            ConnectionEpoch::new(1),
            CdpFrameAttachment::local("target-a", "child", "frame-a"),
            ObjectId::new(5),
        )
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    let new = access.snapshot(ObjectId::new(5)).await.unwrap();
    // Assert
    assert!(!old.is_current().unwrap());
    assert!(new.is_current().unwrap());
}

#[tokio::test]
async fn readiness_rejection_releases_only_the_rejected_snapshot_allowance() {
    // Arrange
    let (fixture, access) = access_fixture(3);
    let baseline = bytes(&fixture);
    let mut first = Box::pin(access.snapshot(ObjectId::new(4)));
    let mut second = Box::pin(access.snapshot(ObjectId::new(5)));
    assert!(poll(first.as_mut()).is_pending());
    assert!(poll(second.as_mut()).is_pending());
    // Act
    let third = access.snapshot(ObjectId::new(4)).await;
    // Assert
    assert!(matches!(
        third,
        Err(FrameWorldAccessError::Topology(
            CdpPageRouteError::WorldReadinessCapacityExceeded { limit: 2 }
        ))
    ));
    assert_eq!(bytes(&fixture), baseline + 2 * 8192);
    drop((first, second));
    assert_eq!(bytes(&fixture), baseline);
}

#[tokio::test]
async fn invalid_numeric_main_context_never_escapes_as_a_usable_snapshot() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    pair(&mut fixture, "frame-a", 0);
    let baseline = bytes(&fixture);
    // Act
    let result = access.snapshot(ObjectId::new(4)).await;
    // Assert
    assert!(matches!(result, Err(FrameWorldAccessError::Setup(_))));
    assert_eq!(bytes(&fixture), baseline);
    assert!(fixture.factory.client("target-a").calls().is_empty());
}
