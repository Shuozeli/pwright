use super::*;
use crate::native_world_retirement::Destruction;

#[tokio::test]
async fn partial_detach_failure_does_not_prevent_independent_session_acknowledgement() {
    // Arrange
    let (mut fixture, access) = fixture(2);
    let first = access.snapshot(ObjectId::new(4)).await.unwrap();
    fixture
        .topology
        .attach_frame(
            ConnectionEpoch::new(1),
            CdpFrameAttachment::remote("target-a", "child", "frame-a", "remote-session"),
            ObjectId::new(5),
        )
        .unwrap();
    for (id, unique, kind) in [
        (31, "remote-main", CdpWorldKind::Main),
        (32, "remote-utility", CdpWorldKind::Utility),
    ] {
        fixture
            .topology
            .attach_world(
                ConnectionEpoch::new(1),
                CdpWorldAttachment::new("remote-session", id, unique, "child", kind),
            )
            .unwrap();
    }
    fixture.services.synchronize(&fixture.topology).unwrap();
    let second = access.snapshot(ObjectId::new(5)).await.unwrap();
    let browser = pwright_fake::FakeCdpClient::from_html("");
    browser.set_target_detach_errors([Some("native detach failed".into()), None]);
    // Act
    let result = access.retire_owned_native_sessions(&browser).await;
    // Assert
    assert!(result.is_err());
    assert!(!first.native_lifetime().unwrap().native_objects_retired());
    assert!(second.native_lifetime().unwrap().native_objects_retired());
    assert!(!second.native_lifetime().unwrap().both_worlds_destroyed());
    assert_eq!(
        browser.method_names(),
        ["Target.detachFromTarget", "Target.detachFromTarget"]
    );
}

#[tokio::test]
async fn acknowledged_owned_session_retirement_deduplicates_and_preserves_worlds() {
    // Arrange
    let (_, access) = fixture(2);
    let first = access.snapshot(ObjectId::new(4)).await.unwrap();
    let second = access.snapshot(ObjectId::new(4)).await.unwrap();
    let browser = pwright_fake::FakeCdpClient::from_html("");
    // Act
    access.retire_owned_native_sessions(&browser).await.unwrap();
    // Assert
    for snapshot in [&first, &second] {
        let lifetime = snapshot.native_lifetime().unwrap();
        assert!(lifetime.native_objects_retired());
        assert!(!lifetime.both_worlds_destroyed());
    }
    assert_eq!(browser.method_names(), ["Target.detachFromTarget"]);
    assert_eq!(browser.calls()[0].1, [serde_json::json!("session-a")]);
    drop((first, second));
    assert!(matches!(
        access.snapshot(ObjectId::new(4)).await,
        Err(FrameWorldAccessError::Topology(
            CdpPageRouteError::NativeRetirementSealed
        ))
    ));
    assert!(access.retire_owned_native_sessions(&browser).await.is_err());
    assert_eq!(browser.calls().len(), 1);
}

#[tokio::test]
async fn failed_detach_wakes_release_fence_without_fabricating_or_retrying_a_receipt() {
    // Arrange
    let (_, access) = fixture(2);
    let first = access.snapshot(ObjectId::new(4)).await.unwrap();
    let second = access.snapshot(ObjectId::new(4)).await.unwrap();
    let lifetime = first.native_lifetime().unwrap();
    let browser = pwright_fake::FakeCdpClient::from_html("");
    let gate = Arc::new(tokio::sync::Notify::new());
    browser.set_target_detach_gate(Some(gate.clone()));
    browser.set_target_detach_errors([Some("No session with given id".into())]);
    let mut retirement = Box::pin(access.retire_owned_native_sessions(&browser));
    assert!(poll(retirement.as_mut()).is_pending());
    assert!(
        lifetime
            .poll_session_release_ready(&mut std::task::Context::from_waker(
                std::task::Waker::noop()
            ))
            .is_pending()
    );
    assert!(!lifetime.native_objects_retired());
    // Act
    gate.notify_waiters();
    let result = retirement.await;
    // Assert
    assert!(result.is_err());
    assert!(
        lifetime
            .poll_session_release_ready(&mut std::task::Context::from_waker(
                std::task::Waker::noop()
            ))
            .is_ready()
    );
    assert!(!lifetime.native_objects_retired());
    assert!(!second.native_lifetime().unwrap().native_objects_retired());
    assert_eq!(browser.method_names(), ["Target.detachFromTarget"]);
}

#[tokio::test]
async fn owned_detach_does_not_publish_receipt_before_acknowledgement() {
    // Arrange
    let (_, access) = fixture(1);
    let snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    let mut lifetime = snapshot.native_lifetime().unwrap();
    let browser = pwright_fake::FakeCdpClient::from_html("");
    let gate = Arc::new(tokio::sync::Notify::new());
    browser.set_target_detach_gate(Some(gate.clone()));
    let mut retirement = Box::pin(access.retire_owned_native_sessions(&browser));
    assert!(poll(retirement.as_mut()).is_pending());
    assert!(
        lifetime
            .poll_session_retired(&mut std::task::Context::from_waker(std::task::Waker::noop()))
            .is_pending()
    );
    assert!(!lifetime.native_objects_retired());
    // Act
    gate.notify_waiters();
    retirement.await.unwrap();
    // Assert
    assert!(
        lifetime
            .poll_session_retired(&mut std::task::Context::from_waker(std::task::Waker::noop()))
            .is_ready()
    );
    assert!(lifetime.native_objects_retired());
    assert!(!lifetime.both_worlds_destroyed());
}

#[tokio::test]
async fn retirement_never_detaches_an_old_epoch_through_the_current_connection() {
    // Arrange
    let (fixture, access) = fixture(1);
    let snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    let registry = fixture
        .services
        .routes
        .lock()
        .unwrap()
        .native_world_retirement
        .clone()
        .unwrap();
    let browser = pwright_fake::FakeCdpClient::from_html("");
    // Act
    let result = registry
        .retire_sessions(ConnectionEpoch::new(2), &browser)
        .await;
    // Assert
    assert!(result.is_err());
    assert!(browser.calls().is_empty());
    assert!(!snapshot.native_lifetime().unwrap().native_objects_retired());
}

fn fixture(capacity: usize) -> (Fixture, ChromiumFrameWorldAccess) {
    let profile = ChromiumFrameWorldAccessProfile::new(
        ResourceScope::extension(98, 3),
        NonZeroUsize::new(capacity).unwrap(),
    )
    .unwrap()
    .with_native_retirement()
    .unwrap();
    let mut fixture = fixture_with_world_access(true, Some(profile));
    let access =
        ChromiumFrameWorldAccess::new(&fixture.services, profile, fixture.prepared.resources())
            .unwrap();
    pair(&mut fixture, "frame-a", 11);
    (fixture, access)
}

fn destroy_world(fixture: &mut Fixture, session: &str, id: i64, unique: &str) {
    let runtime = runtime(fixture, ManualDeadline::default());
    crate::transact_topology_update_with_destruction(
        &mut fixture.topology,
        &runtime,
        |topology| topology.destroy_world(ConnectionEpoch::new(1), session, id),
        Some(Destruction::World { session, unique }),
    )
    .unwrap();
}

#[tokio::test]
async fn default_snapshot_installs_no_registry_or_receipt() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    pair(&mut fixture, "frame-a", 11);
    // Act
    let snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    // Assert
    assert!(snapshot.native_lifetime().is_none());
    assert!(
        fixture
            .services
            .routes
            .lock()
            .unwrap()
            .native_world_retirement
            .is_none()
    );
}

#[tokio::test]
async fn proof_requires_both_exact_unique_worlds_in_the_original_session() {
    // Arrange
    let (mut fixture, access) = fixture(1);
    let snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    let lifetime = snapshot.native_lifetime().unwrap();
    // Act / Assert: neither a foreign session nor a numeric ID is proof.
    destroy_world(&mut fixture, "other-session", 11, "main-11");
    destroy_world(&mut fixture, "session-a", 11, "other-main");
    assert!(!lifetime.both_worlds_destroyed());
    destroy_world(&mut fixture, "session-a", 11, "main-11");
    assert!(!lifetime.both_worlds_destroyed());
    destroy_world(&mut fixture, "session-a", 12, "utility-11");
    assert!(lifetime.both_worlds_destroyed());
}

#[tokio::test]
async fn retained_receipt_keeps_snapshot_capacity_until_its_last_clone_drops() {
    // Arrange
    let (fixture, access) = fixture(1);
    let baseline = bytes(&fixture);
    let snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    let lifetime = snapshot.native_lifetime().unwrap();
    drop(snapshot);
    // Act
    let rejected = access.snapshot(ObjectId::new(4)).await;
    // Assert
    assert!(matches!(
        rejected,
        Err(FrameWorldAccessError::Capacity { limit: 1 })
    ));
    assert_eq!(bytes(&fixture), baseline + 8192);
    drop(lifetime);
    assert_eq!(bytes(&fixture), baseline);
    let replacement = access.snapshot(ObjectId::new(4)).await.unwrap();
    assert!(
        !replacement
            .native_lifetime()
            .unwrap()
            .both_worlds_destroyed()
    );
}

#[tokio::test]
async fn route_replacement_does_not_discharge_old_native_ownership() {
    // Arrange
    let (mut fixture, access) = fixture(2);
    let snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    let lifetime = snapshot.native_lifetime().unwrap();
    // Act
    pair(&mut fixture, "frame-a", 21);
    // Assert
    assert!(!snapshot.is_current().unwrap());
    assert!(!lifetime.both_worlds_destroyed());
    let replacement = access.snapshot(ObjectId::new(4)).await.unwrap();
    destroy_world(&mut fixture, "session-a", 11, "main-11");
    destroy_world(&mut fixture, "session-a", 12, "utility-11");
    assert!(lifetime.both_worlds_destroyed());
    assert!(
        !replacement
            .native_lifetime()
            .unwrap()
            .both_worlds_destroyed()
    );
}

#[tokio::test]
async fn detach_is_not_destruction_and_late_context_ack_can_retire_the_receipt() {
    // Arrange
    let (mut fixture, access) = fixture(1);
    let snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    let lifetime = snapshot.native_lifetime().unwrap();
    // Act
    fixture
        .topology
        .detach_frame(
            ConnectionEpoch::new(1),
            "frame-a",
            CdpFrameDetachReason::Remove,
        )
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    // Assert
    assert!(!lifetime.both_worlds_destroyed());
    let prepared = fixture
        .services
        .prepare_synchronization(fixture.topology.topology())
        .unwrap();
    prepared
        .observe_native_destruction(Destruction::Context(ObjectId::new(999)))
        .unwrap();
    assert!(!lifetime.both_worlds_destroyed());
    prepared
        .observe_native_destruction(Destruction::Context(ObjectId::new(2)))
        .unwrap();
    prepared.commit();
    assert!(lifetime.both_worlds_destroyed());
}

#[tokio::test]
async fn cleared_worlds_do_not_poison_snapshots_created_after_the_route_commit() {
    // Arrange
    let (mut fixture, access) = fixture(2);
    let snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    let runtime = runtime(&fixture, ManualDeadline::default());
    // Act
    crate::transact_topology_update_with_destruction(
        &mut fixture.topology,
        &runtime,
        |topology| topology.clear_worlds(ConnectionEpoch::new(1), "session-a"),
        Some(Destruction::SessionWorlds("session-a")),
    )
    .unwrap();
    // Assert
    assert!(snapshot.native_lifetime().unwrap().both_worlds_destroyed());
    let mut next = Box::pin(access.snapshot(ObjectId::new(4)));
    assert!(poll(next.as_mut()).is_pending());
    pair(&mut fixture, "frame-a", 21);
    assert!(
        !next
            .await
            .unwrap()
            .native_lifetime()
            .unwrap()
            .both_worlds_destroyed()
    );
}

#[tokio::test]
async fn last_receipt_releases_all_owners_without_a_registry_cycle() {
    // Arrange
    let (fixture, access) = fixture(1);
    let resources = fixture.prepared.resources().clone();
    let snapshot = access.snapshot(ObjectId::new(4)).await.unwrap();
    let lifetime = snapshot.native_lifetime().unwrap();
    drop((snapshot, access, fixture));
    assert!(resources.metrics().unwrap().active_leases > 0);
    // Act
    drop(lifetime);
    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn reconnect_cannot_transfer_an_old_destruction_ack_to_reused_native_ids() {
    // Arrange
    let (mut fixture, access) = fixture(2);
    let old = access.snapshot(ObjectId::new(4)).await.unwrap();
    let old_topology = fixture.topology.topology().clone();
    let epoch = ConnectionEpoch::new(2);
    fixture.topology.begin_reconnect(epoch).unwrap();
    fixture
        .topology
        .register_context(
            epoch,
            "context-a",
            ObjectId::new(2),
            CdpContextOwnership::External,
        )
        .unwrap();
    fixture
        .topology
        .attach_page(
            epoch,
            CdpPageTarget::new("target-a", "session-a", "context-a"),
            CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
        )
        .unwrap();
    fixture
        .topology
        .attach_frame(
            epoch,
            CdpFrameAttachment::main("target-a", "frame-a"),
            ObjectId::new(4),
        )
        .unwrap();
    for (id, unique, kind) in [
        (11, "main-11", CdpWorldKind::Main),
        (12, "utility-11", CdpWorldKind::Utility),
    ] {
        fixture
            .topology
            .attach_world(
                epoch,
                CdpWorldAttachment::new("session-a", id, unique, "frame-a", kind),
            )
            .unwrap();
    }
    fixture.services.synchronize(&fixture.topology).unwrap();
    let new = access.snapshot(ObjectId::new(4)).await.unwrap();
    // Act: an old native acknowledgement races the newer published routes.
    let prepared = fixture
        .services
        .prepare_synchronization(&old_topology)
        .unwrap();
    prepared
        .observe_native_destruction(Destruction::Context(ObjectId::new(2)))
        .unwrap();
    drop(prepared);
    // Assert: neither route identity nor native proof may cross connection epochs.
    assert!(!old.is_current().unwrap());
    assert!(new.is_current().unwrap());
    assert!(old.native_lifetime().unwrap().both_worlds_destroyed());
    assert!(!new.native_lifetime().unwrap().both_worlds_destroyed());
}
