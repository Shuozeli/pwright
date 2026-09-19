use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Wake;

#[test]
fn missing_utility_returns_none_without_admission_even_with_a_main_world() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    attach_world(
        &mut fixture.topology,
        11,
        "main-11",
        "frame-a",
        CdpWorldKind::Main,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
    let before = fixture.prepared.resources().metrics().unwrap();

    // Act
    let result = access
        .existing_frame_utility_world(ObjectId::new(4))
        .unwrap();

    // Assert
    assert!(result.is_none());
    assert_eq!(fixture.prepared.resources().metrics().unwrap(), before);
    assert!(fixture.factory.client("target-a").calls().is_empty());
}

#[test]
fn selects_only_requested_child_without_requiring_a_main_world() {
    // Arrange: two available worlds but only one snapshot slot.
    let (mut fixture, access) = access_fixture(1);
    utility(&mut fixture, "frame-a", 12);
    utility(&mut fixture, "child", 22);
    let baseline = bytes(&fixture);

    // Act
    let world = access
        .existing_frame_utility_world(ObjectId::new(5))
        .unwrap()
        .unwrap();

    // Assert
    assert_eq!(world.page(), ObjectId::new(3));
    assert_eq!(world.frame(), ObjectId::new(5));
    assert_eq!(world.utility_unique_id(), "utility-22");
    let client: Arc<dyn CdpClient> = fixture.factory.client("target-a");
    assert!(Arc::ptr_eq(&world.client(), &client));
    assert!(world.is_current().unwrap());
    assert!(world.native_lifetime().is_none());
    assert_eq!(bytes(&fixture), baseline + 8192);
    assert!(fixture.factory.client("target-a").calls().is_empty());
    drop(world);
    assert_eq!(bytes(&fixture), baseline);
}

#[test]
fn page_or_unknown_object_id_is_not_substituted_with_a_frame() {
    for object in [ObjectId::new(3), ObjectId::new(999)] {
        // Arrange
        let (mut fixture, access) = access_fixture(1);
        utility(&mut fixture, "frame-a", 12);
        let before = fixture.prepared.resources().metrics().unwrap();

        // Act
        let result = access.existing_frame_utility_world(object);

        // Assert
        assert!(matches!(
            result,
            Err(FrameWorldAccessError::Topology(
                CdpPageRouteError::WorldTargetClosed { object_id }
            )) if object_id == object
        ));
        assert_eq!(fixture.prepared.resources().metrics().unwrap(), before);
        assert!(fixture.factory.client("target-a").calls().is_empty());
    }
}

#[test]
fn missing_owning_page_is_a_frame_target_error_without_admission() {
    // Arrange: retained Frame index cannot authorize a missing Page route.
    let (mut fixture, access) = access_fixture(1);
    utility(&mut fixture, "frame-a", 12);
    fixture
        .services
        .routes
        .lock()
        .unwrap()
        .pages
        .remove(&ObjectId::new(3));
    let before = fixture.prepared.resources().metrics().unwrap();

    // Act
    let result = access.existing_frame_utility_world(ObjectId::new(4));

    // Assert
    assert!(matches!(
        result,
        Err(FrameWorldAccessError::Topology(
            CdpPageRouteError::WorldTargetClosed { object_id }
        )) if object_id == ObjectId::new(4)
    ));
    assert_eq!(fixture.prepared.resources().metrics().unwrap(), before);
}

#[test]
fn crashed_page_is_a_frame_scoped_error_without_admission() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    utility(&mut fixture, "frame-a", 12);
    fixture
        .topology
        .crash_target(ConnectionEpoch::new(1), "target-a", "crashed", 1)
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    let before = fixture.prepared.resources().metrics().unwrap();

    // Act
    let result = access.existing_frame_utility_world(ObjectId::new(4));

    // Assert
    assert!(matches!(
        result,
        Err(FrameWorldAccessError::Topology(
            CdpPageRouteError::PageCrashed { object_id }
        )) if object_id == ObjectId::new(4)
    ));
    assert_eq!(fixture.prepared.resources().metrics().unwrap(), before);
}

#[test]
fn oopif_selection_keeps_its_exact_client_and_unique_world() {
    // Arrange
    let (mut fixture, access) = native_fixture(1);
    fixture
        .topology
        .attach_frame(
            ConnectionEpoch::new(1),
            CdpFrameAttachment::remote("target-a", "child", "frame-a", "remote-session"),
            ObjectId::new(5),
        )
        .unwrap();
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

    // Act
    let world = access
        .existing_frame_utility_world(ObjectId::new(5))
        .unwrap()
        .unwrap();

    // Assert
    assert_eq!(world.frame(), ObjectId::new(5));
    assert_eq!(world.page(), ObjectId::new(3));
    assert_eq!(world.utility_unique_id(), "remote-utility");
    assert!(world.is_current().unwrap());
    assert!(world.native_lifetime().is_some());
    let routes = fixture.services.routes.lock().unwrap();
    let selected = &routes.frame_contexts[&ObjectId::new(5)];
    assert_eq!(selected.session_id, "remote-session");
    assert!(Arc::ptr_eq(&world.client(), &selected.client));
}

#[test]
fn shared_capacity_refusal_preserves_the_existing_single_snapshot() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    utility(&mut fixture, "frame-a", 12);
    let baseline = bytes(&fixture);
    let first = access
        .existing_frame_utility_world(ObjectId::new(4))
        .unwrap()
        .unwrap();

    // Act
    let refused = access.existing_frame_utility_world(ObjectId::new(4));

    // Assert
    assert!(matches!(
        refused,
        Err(FrameWorldAccessError::Capacity { limit: 1 })
    ));
    assert_eq!(bytes(&fixture), baseline + 8192);
    assert!(first.is_current().unwrap());
    assert!(
        access
            .existing_frame_utility_world(ObjectId::new(5))
            .unwrap()
            .is_none()
    );
    drop(first);
    let replacement = access
        .existing_frame_utility_world(ObjectId::new(4))
        .unwrap()
        .unwrap();
    drop(replacement);
    assert_eq!(bytes(&fixture), baseline);
    assert!(fixture.factory.client("target-a").calls().is_empty());
}

#[tokio::test]
async fn single_snapshot_shares_capacity_with_paired_and_page_access() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    pair(&mut fixture, "frame-a", 11);
    let single = access
        .existing_frame_utility_world(ObjectId::new(4))
        .unwrap()
        .unwrap();

    // Act
    let paired = access.snapshot(ObjectId::new(4)).await;
    let page = access.existing_page_utility_worlds(ObjectId::new(3), 1);

    // Assert
    assert!(matches!(
        paired,
        Err(FrameWorldAccessError::Capacity { limit: 1 })
    ));
    assert!(matches!(
        page,
        Err(FrameWorldAccessError::Capacity { limit: 1 })
    ));
    drop(single);
    assert!(access.snapshot(ObjectId::new(4)).await.is_ok());
}

#[test]
fn malformed_native_ids_fail_before_snapshot_admission() {
    for field in 0..4 {
        // Arrange
        let (mut fixture, access) = native_fixture(1);
        utility(&mut fixture, "frame-a", 12);
        {
            let mut routes = fixture.services.routes.lock().unwrap();
            let oversized = "x".repeat(1025);
            if field == 3 {
                Arc::make_mut(routes.pages.get_mut(&ObjectId::new(3)).unwrap()).target_id =
                    oversized;
            } else {
                let world =
                    Arc::make_mut(routes.frame_contexts.get_mut(&ObjectId::new(4)).unwrap());
                match field {
                    0 => world.target_id = oversized,
                    1 => world.session_id = oversized,
                    2 => world.unique_context_id = oversized.into(),
                    _ => unreachable!(),
                }
            }
        }
        let before = fixture.prepared.resources().metrics().unwrap();

        // Act
        let result = access.existing_frame_utility_world(ObjectId::new(4));

        // Assert
        assert!(matches!(result, Err(FrameWorldAccessError::Topology(_))));
        assert_eq!(fixture.prepared.resources().metrics().unwrap(), before);
        assert!(fixture.factory.client("target-a").calls().is_empty());
    }
}

#[test]
fn main_world_replacement_does_not_retire_a_current_utility_snapshot() {
    // Arrange
    let (mut fixture, access) = native_fixture(1);
    pair(&mut fixture, "frame-a", 11);
    let snapshot = access
        .existing_frame_utility_world(ObjectId::new(4))
        .unwrap()
        .unwrap();

    // Act
    attach_world(
        &mut fixture.topology,
        21,
        "new-main",
        "frame-a",
        CdpWorldKind::Main,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();

    // Assert
    assert!(snapshot.is_current().unwrap());
    assert_eq!(snapshot.utility_unique_id(), "utility-11");
    assert!(!snapshot.native_lifetime().unwrap().world_destroyed());
}

#[test]
fn replaced_utility_is_sticky_stale_without_fabricating_native_destruction() {
    // Arrange
    let (mut fixture, access) = native_fixture(1);
    utility(&mut fixture, "frame-a", 12);
    let snapshot = access
        .existing_frame_utility_world(ObjectId::new(4))
        .unwrap()
        .unwrap();
    let receipt = snapshot.native_lifetime().unwrap();

    // Act
    utility(&mut fixture, "frame-a", 13);

    // Assert
    assert!(!snapshot.is_current().unwrap());
    assert!(!receipt.world_destroyed());
    assert!(!receipt.native_objects_retired());
    utility(&mut fixture, "frame-a", 12);
    assert!(!snapshot.is_current().unwrap());
}

#[test]
fn single_receipt_tracks_only_exact_utility_destruction_and_keeps_its_lease() {
    // Arrange
    let (mut fixture, access) = native_fixture(1);
    pair(&mut fixture, "frame-a", 11);
    let baseline = bytes(&fixture);
    let snapshot = access
        .existing_frame_utility_world(ObjectId::new(4))
        .unwrap()
        .unwrap();
    let receipt = snapshot.native_lifetime().unwrap();
    drop(snapshot);
    destroy(&mut fixture, "session-a", 11, "main-11");
    destroy(&mut fixture, "foreign-session", 12, "utility-11");
    assert!(!receipt.world_destroyed());

    // Act
    destroy(&mut fixture, "session-a", 12, "utility-11");

    // Assert
    assert!(receipt.world_destroyed());
    assert!(receipt.native_objects_retired());
    assert_eq!(bytes(&fixture), baseline + 8192);
    drop(receipt);
    assert_eq!(bytes(&fixture), baseline);
}

struct CountWake(AtomicUsize);
impl Wake for CountWake {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn two_single_snapshots_keep_independent_release_waiters_until_one_session_ack() {
    // Arrange
    let (mut fixture, access) = native_fixture(2);
    utility(&mut fixture, "frame-a", 12);
    let first = access
        .existing_frame_utility_world(ObjectId::new(4))
        .unwrap()
        .unwrap();
    let second = access
        .existing_frame_utility_world(ObjectId::new(4))
        .unwrap()
        .unwrap();
    let first_receipt = first.native_lifetime().unwrap();
    let second_receipt = second.native_lifetime().unwrap();
    let browser = pwright_fake::FakeCdpClient::from_html("");
    let gate = Arc::new(tokio::sync::Notify::new());
    browser.set_target_detach_gate(Some(gate.clone()));
    let mut retirement = Box::pin(access.retire_owned_native_sessions(&browser));
    assert!(poll(retirement.as_mut()).is_pending());
    let first_wake = Arc::new(CountWake(AtomicUsize::new(0)));
    let second_wake = Arc::new(CountWake(AtomicUsize::new(0)));
    let first_waker = Waker::from(first_wake.clone());
    let second_waker = Waker::from(second_wake.clone());
    assert!(
        first_receipt
            .poll_session_release_ready(&mut Context::from_waker(&first_waker))
            .is_pending()
    );
    assert!(
        second_receipt
            .poll_session_release_ready(&mut Context::from_waker(&second_waker))
            .is_pending()
    );
    assert!(!first_receipt.native_objects_retired());
    assert!(!second_receipt.native_objects_retired());

    // Act
    gate.notify_one();
    retirement.await.unwrap();

    // Assert: cloning one receipt here would overwrite one waiter and fail.
    assert_eq!(first_wake.0.load(Ordering::SeqCst), 1);
    assert_eq!(second_wake.0.load(Ordering::SeqCst), 1);
    assert!(first_receipt.native_objects_retired());
    assert!(second_receipt.native_objects_retired());
    assert!(!first_receipt.world_destroyed());
    assert!(!second_receipt.world_destroyed());
    assert_eq!(browser.method_names(), ["Target.detachFromTarget"]);
}

#[tokio::test]
async fn sealed_native_registry_refuses_single_snapshot_and_rolls_back_permit() {
    // Arrange
    let (mut fixture, access) = native_fixture(2);
    utility(&mut fixture, "frame-a", 12);
    let snapshot = access
        .existing_frame_utility_world(ObjectId::new(4))
        .unwrap()
        .unwrap();
    let browser = pwright_fake::FakeCdpClient::from_html("");
    access.retire_owned_native_sessions(&browser).await.unwrap();
    let baseline = bytes(&fixture);
    let active = fixture
        .prepared
        .resources()
        .metrics()
        .unwrap()
        .active_leases;

    // Act
    let result = access.existing_frame_utility_world(ObjectId::new(4));

    // Assert
    assert!(matches!(
        result,
        Err(FrameWorldAccessError::Topology(
            CdpPageRouteError::NativeRetirementSealed
        ))
    ));
    assert_eq!(bytes(&fixture), baseline);
    assert_eq!(
        fixture
            .prepared
            .resources()
            .metrics()
            .unwrap()
            .active_leases,
        active
    );
    assert!(snapshot.native_lifetime().unwrap().native_objects_retired());
}
