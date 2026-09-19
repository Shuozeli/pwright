use super::*;
use crate::native_world_retirement::Destruction;

mod single;

fn utility(fixture: &mut Fixture, frame: &str, id: i64) {
    attach_world(
        &mut fixture.topology,
        id,
        &format!("utility-{id}"),
        frame,
        CdpWorldKind::Utility,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
}
fn native_fixture(capacity: usize) -> (Fixture, ChromiumFrameWorldAccess) {
    let profile = ChromiumFrameWorldAccessProfile::new(
        ResourceScope::extension(98, 3),
        NonZeroUsize::new(capacity).unwrap(),
    )
    .unwrap()
    .with_native_retirement()
    .unwrap();
    let fixture = fixture_with_world_access(true, Some(profile));
    let access =
        ChromiumFrameWorldAccess::new(&fixture.services, profile, fixture.prepared.resources())
            .unwrap();
    (fixture, access)
}
fn destroy(fixture: &mut Fixture, session: &str, id: i64, unique: &str) {
    let runtime = runtime(fixture, ManualDeadline::default());
    crate::transact_topology_update_with_destruction(
        &mut fixture.topology,
        &runtime,
        |topology| topology.destroy_world(ConnectionEpoch::new(1), session, id),
        Some(Destruction::World { session, unique }),
    )
    .unwrap();
}

#[test]
fn absent_worlds_return_an_empty_batch_without_waiting_or_allocating() {
    // Arrange
    let (fixture, access) = access_fixture(2);
    let baseline = bytes(&fixture);
    // Act
    let batch = access
        .existing_page_utility_worlds(ObjectId::new(3), 32)
        .unwrap();
    // Assert
    assert!(batch.worlds().is_empty());
    assert_eq!(bytes(&fixture), baseline);
    assert!(fixture.factory.client("target-a").calls().is_empty());
}

#[test]
fn utility_only_world_does_not_wait_for_a_main_world_or_missing_child() {
    // Arrange
    let (mut fixture, access) = access_fixture(2);
    utility(&mut fixture, "frame-a", 12);
    let baseline = bytes(&fixture);
    // Act
    let batch = access
        .existing_page_utility_worlds(ObjectId::new(3), 32)
        .unwrap();
    // Assert
    assert_eq!(batch.worlds().len(), 1);
    let world = &batch.worlds()[0];
    assert_eq!(world.page(), ObjectId::new(3));
    assert_eq!(world.frame(), ObjectId::new(4));
    assert_eq!(world.utility_unique_id(), "utility-12");
    assert!(world.is_current().unwrap());
    assert!(world.native_lifetime().is_none());
    let expected: Arc<dyn CdpClient> = fixture.factory.client("target-a");
    assert!(Arc::ptr_eq(&world.client(), &expected));
    assert_eq!(bytes(&fixture), baseline + 8192);
    assert!(fixture.factory.client("target-a").calls().is_empty());
    drop(batch);
    assert_eq!(bytes(&fixture), baseline);
}

#[test]
fn local_child_is_selected_without_substituting_a_frame_id_for_page_id() {
    // Arrange
    let (mut fixture, access) = access_fixture(3);
    utility(&mut fixture, "frame-a", 12);
    utility(&mut fixture, "child", 22);
    // Act
    let batch = access
        .existing_page_utility_worlds(ObjectId::new(3), 2)
        .unwrap();
    let invalid = access.existing_page_utility_worlds(ObjectId::new(4), 2);
    // Assert
    assert_eq!(
        batch
            .worlds()
            .iter()
            .map(|world| world.frame())
            .collect::<Vec<_>>(),
        [ObjectId::new(4), ObjectId::new(5)]
    );
    assert!(
        batch
            .worlds()
            .iter()
            .all(|world| world.page() == ObjectId::new(3))
    );
    assert!(matches!(
        invalid,
        Err(FrameWorldAccessError::Topology(
            CdpPageRouteError::WorldTargetClosed { .. }
        ))
    ));
}

#[test]
fn oopif_uses_its_exact_utility_session_even_without_a_main_world() {
    // Arrange
    let (mut fixture, access) = access_fixture(2);
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
    let batch = access
        .existing_page_utility_worlds(ObjectId::new(3), 2)
        .unwrap();
    // Assert
    assert_eq!(batch.worlds().len(), 1);
    assert_eq!(batch.worlds()[0].frame(), ObjectId::new(5));
    assert_eq!(batch.worlds()[0].utility_unique_id(), "remote-utility");
    let routes = fixture.services.routes.lock().unwrap();
    let selected = &routes.frame_contexts[&ObjectId::new(5)];
    assert_eq!(selected.session_id, "remote-session");
    assert!(Arc::ptr_eq(&batch.worlds()[0].client(), &selected.client));
    assert!(batch.worlds()[0].native_lifetime().is_none());
}

#[test]
fn existing_utility_selection_excludes_other_pages_in_the_same_context() {
    // Arrange
    let (mut fixture, access) = access_fixture(3);
    utility(&mut fixture, "frame-a", 12);
    fixture
        .topology
        .attach_page(
            ConnectionEpoch::new(1),
            CdpPageTarget::new("target-b", "session-b", "context-a"),
            CdpPageObjectIds::new(ObjectId::new(6), ObjectId::new(7)),
        )
        .unwrap();
    fixture
        .topology
        .attach_frame(
            ConnectionEpoch::new(1),
            CdpFrameAttachment::main("target-b", "frame-b"),
            ObjectId::new(7),
        )
        .unwrap();
    fixture
        .topology
        .attach_world(
            ConnectionEpoch::new(1),
            CdpWorldAttachment::new(
                "session-b",
                42,
                "foreign-utility",
                "frame-b",
                CdpWorldKind::Utility,
            ),
        )
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    // Act
    let batch = access
        .existing_page_utility_worlds(ObjectId::new(3), 1)
        .unwrap();
    // Assert
    assert_eq!(batch.worlds().len(), 1);
    assert_eq!(batch.worlds()[0].utility_unique_id(), "utility-12");
    assert_eq!(batch.worlds()[0].page(), ObjectId::new(3));
}

#[test]
fn caller_maximum_rejects_the_whole_batch_instead_of_truncating() {
    // Arrange
    let (mut fixture, access) = access_fixture(3);
    utility(&mut fixture, "frame-a", 12);
    utility(&mut fixture, "child", 22);
    let baseline = bytes(&fixture);
    // Act
    let limited = access.existing_page_utility_worlds(ObjectId::new(3), 1);
    // Assert
    assert!(matches!(
        limited,
        Err(FrameWorldAccessError::Capacity { limit: 1 })
    ));
    assert_eq!(bytes(&fixture), baseline);
    assert!(
        access
            .existing_page_utility_worlds(ObjectId::new(3), 0)
            .is_err()
    );
    assert!(
        access
            .existing_page_utility_worlds(ObjectId::new(3), 33)
            .is_err()
    );
}

#[test]
fn partial_admission_rolls_back_when_shared_capacity_is_insufficient() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    utility(&mut fixture, "frame-a", 12);
    utility(&mut fixture, "child", 22);
    let baseline = bytes(&fixture);
    // Act
    let limited = access.existing_page_utility_worlds(ObjectId::new(3), 2);
    // Assert
    assert!(matches!(
        limited,
        Err(FrameWorldAccessError::Capacity { limit: 1 })
    ));
    assert_eq!(bytes(&fixture), baseline);
    fixture
        .topology
        .destroy_world(ConnectionEpoch::new(1), "session-a", 22)
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    assert_eq!(
        access
            .existing_page_utility_worlds(ObjectId::new(3), 2)
            .unwrap()
            .worlds()
            .len(),
        1
    );
    assert_eq!(bytes(&fixture), baseline);
}

#[tokio::test]
async fn paired_and_utility_snapshots_share_the_same_admission() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    pair(&mut fixture, "frame-a", 11);
    let pending = access.snapshot(ObjectId::new(4));
    // Act
    let limited = access.existing_page_utility_worlds(ObjectId::new(3), 1);
    // Assert
    assert!(matches!(
        limited,
        Err(FrameWorldAccessError::Capacity { limit: 1 })
    ));
    drop(pending);
    let batch = access
        .existing_page_utility_worlds(ObjectId::new(3), 1)
        .unwrap();
    assert!(matches!(
        access.snapshot(ObjectId::new(4)).await,
        Err(FrameWorldAccessError::Capacity { limit: 1 })
    ));
    drop(batch);
    assert!(access.snapshot(ObjectId::new(4)).await.is_ok());
}

#[test]
fn utility_receipt_requires_its_exact_destruction_not_main_or_foreign_session() {
    // Arrange
    let (mut fixture, access) = native_fixture(2);
    pair(&mut fixture, "frame-a", 11);
    let baseline = bytes(&fixture);
    let batch = access
        .existing_page_utility_worlds(ObjectId::new(3), 1)
        .unwrap();
    let lifetime = batch.worlds()[0].native_lifetime().unwrap();
    drop(batch);
    // Act
    destroy(&mut fixture, "other-session", 12, "utility-11");
    destroy(&mut fixture, "session-a", 11, "main-11");
    // Assert
    assert!(!lifetime.world_destroyed());
    assert!(!lifetime.native_objects_retired());
    assert_eq!(bytes(&fixture), baseline + 8192);
    destroy(&mut fixture, "session-a", 12, "utility-11");
    assert!(lifetime.world_destroyed());
    assert!(lifetime.native_objects_retired());
    drop(lifetime);
    assert_eq!(bytes(&fixture), baseline);
}

#[test]
fn route_replacement_does_not_manufacture_destruction_and_staleness_is_sticky() {
    // Arrange
    let (mut fixture, access) = native_fixture(2);
    utility(&mut fixture, "frame-a", 12);
    let batch = access
        .existing_page_utility_worlds(ObjectId::new(3), 1)
        .unwrap();
    let world = &batch.worlds()[0];
    let lifetime = world.native_lifetime().unwrap();
    // Act
    utility(&mut fixture, "frame-a", 13);
    // Assert
    assert!(!world.is_current().unwrap());
    assert!(!lifetime.world_destroyed());
    utility(&mut fixture, "frame-a", 12);
    assert!(!world.is_current().unwrap());
    assert!(!lifetime.world_destroyed());
}

#[tokio::test]
async fn acknowledged_session_retirement_does_not_claim_utility_world_destruction() {
    // Arrange
    let (mut fixture, access) = native_fixture(2);
    utility(&mut fixture, "frame-a", 12);
    let batch = access
        .existing_page_utility_worlds(ObjectId::new(3), 1)
        .unwrap();
    let lifetime = batch.worlds()[0].native_lifetime().unwrap();
    let browser = pwright_fake::FakeCdpClient::from_html("");
    // Act
    access.retire_owned_native_sessions(&browser).await.unwrap();
    // Assert
    assert!(lifetime.native_objects_retired());
    assert!(!lifetime.world_destroyed());
    assert_eq!(browser.method_names(), ["Target.detachFromTarget"]);
    assert!(
        access
            .existing_page_utility_worlds(ObjectId::new(3), 1)
            .is_err()
    );
}

#[test]
fn crashed_pages_fail_without_snapshot_allocation() {
    // Arrange
    let (mut fixture, access) = access_fixture(2);
    utility(&mut fixture, "frame-a", 12);
    fixture
        .topology
        .crash_target(ConnectionEpoch::new(1), "target-a", "crashed", 1)
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    let baseline = bytes(&fixture);
    // Act
    let result = access.existing_page_utility_worlds(ObjectId::new(3), 1);
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
fn utility_snapshot_payload_fits_existing_per_world_allowance() {
    // Arrange
    let actual = std::mem::size_of::<crate::ChromiumUtilityWorldSnapshot>()
        + std::mem::size_of::<crate::ChromiumPageUtilityWorldSnapshot>()
        + 5 * crate::frame_world_access::NATIVE_ID_BYTES
        + 1024;
    // Act
    let fits = actual < crate::frame_world_access::SNAPSHOT_BYTES as usize;
    // Assert
    assert!(fits, "snapshot payload {actual} exceeds admitted allowance");
}
