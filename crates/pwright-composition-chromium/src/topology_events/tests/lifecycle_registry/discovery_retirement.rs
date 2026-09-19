use super::super::discovery_retirement::RetirementInitializer;
use super::*;

#[tokio::test]
async fn retired_initial_route_does_not_seal_an_unrelated_page_registry() {
    // Arrange
    let (registry, _) = registry(2, 2);
    let mut f = fixture(8);
    add_second_page(&f);
    let mut initializer = RetirementInitializer::missing();
    initializer.target = "target-a";
    f.driver = f
        .driver
        .with_frame_event_observer(registry.clone())
        .unwrap()
        .with_session_initializer(Arc::new(initializer));
    // Act
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    // Assert
    assert!(registry.page(ObjectId::new(3)).unwrap().is_none());
    let healthy = registry.page(ObjectId::new(12)).unwrap().unwrap();
    assert!(!healthy.is_sealed());
    assert!(healthy.snapshot(ObjectId::new(13)).unwrap().is_some());
    assert_eq!(f.runtime.live_object_count(), Ok(4));
}

#[tokio::test]
async fn missing_discovery_session_reclaims_unpublished_owner_without_sealing_registry() {
    // Arrange
    let (registry, ledger) = registry(1, 1);
    let mut f = discovery_fixture(8);
    let initializer = Arc::new(RetirementInitializer::missing());
    f.driver = f
        .driver
        .with_frame_event_observer(registry.clone())
        .unwrap()
        .with_session_initializer(initializer.clone());
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    let baseline = ledger.metrics().unwrap().active_leases;
    f.sender
        .send(attached_page_event("page-gone", "session-gone", None, None))
        .unwrap();
    // Act
    let result = f.driver.receive_once(&f.runtime).await.unwrap();
    // Assert
    assert!(matches!(
        result,
        ChromiumTopologyEventDriverOutcome::Ignored
    ));
    let reserved = initializer.pages.lock().unwrap()[0];
    assert!(registry.page(reserved).unwrap().is_none());
    assert_eq!(ledger.metrics().unwrap().active_leases, baseline);
    f.sender
        .send(attached_page_event("page-live", "session-live", None, None))
        .unwrap();
    f.driver.receive_once(&f.runtime).await.unwrap();
    let topology = f.topology.lock().unwrap();
    let page = topology.topology().page_object_id("page-live").unwrap();
    let owner = registry.page(page).unwrap().unwrap();
    assert!(!owner.is_sealed());
    assert!(
        owner
            .snapshot(topology.topology().frame_object_id("page-live").unwrap())
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn abandoning_replacement_preparation_does_not_retire_a_published_owner() {
    // Arrange
    let (registry, ledger) = registry(1, 1);
    let f = selected(registry.clone()).await;
    let owner = registry.page(ObjectId::new(3)).unwrap().unwrap();
    let baseline = ledger.metrics().unwrap().active_leases;
    // Act
    registry
        .discard_prepared_page(f.topology.lock().unwrap().topology(), ObjectId::new(3))
        .unwrap();
    // Assert
    assert!(!owner.is_sealed());
    assert!(owner.snapshot(ObjectId::new(4)).unwrap().is_some());
    assert_eq!(ledger.metrics().unwrap().active_leases, baseline);
}

#[tokio::test]
async fn escaped_unpublished_owner_is_sealed_but_keeps_its_lease_until_drop() {
    // Arrange
    let (registry, ledger) = registry(1, 1);
    let f = discovery_fixture(8);
    let topology = f.topology.lock().unwrap();
    let reserved = ObjectId::new(99);
    let baseline = ledger.metrics().unwrap().active_leases;
    registry
        .prepare_page(topology.topology(), reserved)
        .unwrap();
    let owner = registry.page(reserved).unwrap().unwrap();
    // Act
    registry
        .discard_prepared_page(topology.topology(), reserved)
        .unwrap();
    // Assert
    assert!(owner.is_sealed());
    assert!(registry.page(reserved).unwrap().is_none());
    assert!(ledger.metrics().unwrap().active_leases > baseline);
    drop(owner);
    registry.reconcile(topology.topology()).unwrap();
    assert_eq!(ledger.metrics().unwrap().active_leases, baseline);
}
