use super::*;

#[tokio::test]
async fn utility_numeric_replacement_invalidates_a_snapshot_even_when_unique_id_matches() {
    // Arrange
    let (mut fixture, access) = access_fixture(2);
    pair(&mut fixture, "frame-a", 11);
    let original = access.snapshot(ObjectId::new(4)).await.unwrap();

    // Act
    attach_world(
        &mut fixture.topology,
        22,
        "utility-11",
        "frame-a",
        CdpWorldKind::Utility,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
    let replacement = access.snapshot(ObjectId::new(4)).await.unwrap();

    // Assert: the shared unique ID must not cause the route entry to be reused
    // with an obsolete numeric adoption target or keep an old snapshot current.
    assert_eq!(original.utility_execution_context_id(), 12);
    assert_eq!(replacement.utility_execution_context_id(), 22);
    assert_eq!(
        original.utility_unique_id(),
        replacement.utility_unique_id()
    );
    assert_eq!(
        original.main_execution_context_id(),
        replacement.main_execution_context_id()
    );
    assert!(!original.is_current().unwrap());
    assert!(replacement.is_current().unwrap());
    assert!(Arc::ptr_eq(&original.client(), &replacement.client()));
}

#[tokio::test]
async fn pending_snapshot_captures_a_coherent_replacement_numeric_and_unique_pair() {
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
    let mut waiting = Box::pin(access.snapshot(ObjectId::new(4)));
    assert!(poll(waiting.as_mut()).is_pending());

    // Act: replace main while the utility world is absent, then publish both
    // replacement identities together before resuming the already-admitted wait.
    pair(&mut fixture, "frame-a", 31);
    let snapshot = waiting.await.unwrap();

    // Assert
    assert_eq!(snapshot.main_execution_context_id(), 31);
    assert_eq!(snapshot.main_unique_id(), "main-31");
    assert_eq!(snapshot.utility_execution_context_id(), 32);
    assert_eq!(snapshot.utility_unique_id(), "utility-31");
    assert!(snapshot.is_current().unwrap());
    assert!(fixture.factory.client("target-a").calls().is_empty());
}

#[tokio::test]
async fn snapshot_rejects_a_non_positive_observed_utility_numeric_id() {
    // Arrange
    let (mut fixture, access) = access_fixture(1);
    attach_world(
        &mut fixture.topology,
        11,
        "main-11",
        "frame-a",
        CdpWorldKind::Main,
    );
    attach_world(
        &mut fixture.topology,
        0,
        "utility-zero",
        "frame-a",
        CdpWorldKind::Utility,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
    let baseline = bytes(&fixture);

    // Act
    let result = access.snapshot(ObjectId::new(4)).await;

    // Assert
    assert!(matches!(result, Err(FrameWorldAccessError::Setup(_))));
    assert_eq!(bytes(&fixture), baseline);
    assert!(fixture.factory.client("target-a").calls().is_empty());
}
