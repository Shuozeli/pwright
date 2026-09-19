use pwright_composition_chromium_handle_lifecycle::{
    ChromiumHandleRouteError, ChromiumHandleRouteProfile, ChromiumHandleRoutes,
};
use pwright_core::{ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope};

fn fixture() -> (ChromiumHandleRoutes, ProductionResourceLedger) {
    let profile = ChromiumHandleRouteProfile::new(ResourceScope::extension(166, 1), 3, 64, 128);
    let requirements = profile.requirements();
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let required = requirements.class(class);
        limits = limits.with_class(class, required.global, required.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
    let routes = ChromiumHandleRoutes::new_with_production_resources(profile, &resources).unwrap();
    (routes, resources)
}

#[test]
fn trimming_three_reservations_to_one_releases_two_leases_and_reusable_capacity() {
    // Arrange
    let (routes, resources) = fixture();
    let mut batch = routes.reserve_batch(3).unwrap();
    let before = resources.metrics().unwrap();
    // Act
    batch.trim_to(1).unwrap();
    let after = resources.metrics().unwrap();
    let replacement = routes.reserve_batch(2);
    // Assert
    assert_eq!(before.active_leases, 3);
    assert_eq!(after.active_leases, 1);
    assert_eq!(after.class(ResourceClass::RemoteHandle).current, 1);
    assert_eq!(
        after.class(ResourceClass::BackendStateBytes).current * 3,
        before.class(ResourceClass::BackendStateBytes).current
    );
    assert!(replacement.is_ok());
    assert_eq!(routes.metrics().unwrap().reserved, 3);
    assert_eq!(routes.metrics().unwrap().created, 0);
    assert_eq!(routes.metrics().unwrap().current, 0);
}

#[test]
fn trimming_cannot_grow_a_batch_or_change_its_existing_admission() {
    // Arrange
    let (routes, resources) = fixture();
    let mut batch = routes.reserve_batch(2).unwrap();
    let resource_before = resources.metrics().unwrap();
    let routes_before = routes.metrics().unwrap();
    // Act
    let rejected = batch.trim_to(3);
    // Assert
    assert!(matches!(
        rejected,
        Err(ChromiumHandleRouteError::BatchLengthMismatch {
            reserved: 2,
            actual: 3,
        })
    ));
    assert_eq!(resources.metrics().unwrap(), resource_before);
    assert_eq!(routes.metrics().unwrap(), routes_before);
}

#[test]
fn trimming_to_zero_releases_all_admission_without_publishing_routes() {
    // Arrange
    let (routes, resources) = fixture();
    let mut batch = routes.reserve_batch(3).unwrap();
    // Act
    batch.trim_to(0).unwrap();
    let resource_after = resources.metrics().unwrap();
    let route_after = routes.metrics().unwrap();
    let replacement = routes.reserve_batch(3);
    // Assert
    assert_eq!(resource_after.active_leases, 0);
    assert_eq!(resource_after.live_scopes, 0);
    assert_eq!(resource_after.class(ResourceClass::RemoteHandle).current, 0);
    assert_eq!(
        resource_after
            .class(ResourceClass::BackendStateBytes)
            .current,
        0
    );
    assert_eq!(route_after.reserved, 0);
    assert_eq!(route_after.current, 0);
    assert_eq!(route_after.created, 0);
    assert!(replacement.is_ok());
}

#[test]
fn dropping_a_trimmed_batch_releases_only_its_remaining_admission() {
    // Arrange
    let (routes, resources) = fixture();
    let other = routes.reserve_batch(1).unwrap();
    let mut batch = routes.reserve_batch(2).unwrap();
    batch.trim_to(1).unwrap();
    // Act
    drop(batch);
    let while_other_lives = resources.metrics().unwrap();
    let remaining_routes = routes.metrics().unwrap();
    drop(other);
    let after = resources.metrics().unwrap();
    // Assert
    assert_eq!(while_other_lives.active_leases, 1);
    assert_eq!(
        while_other_lives.class(ResourceClass::RemoteHandle).current,
        1
    );
    assert_eq!(remaining_routes.reserved, 1);
    assert_eq!(remaining_routes.current, 0);
    assert_eq!(after.active_leases, 0);
    assert_eq!(after.live_scopes, 0);
    assert_eq!(after.class(ResourceClass::RemoteHandle).current, 0);
    assert_eq!(after.class(ResourceClass::BackendStateBytes).current, 0);
    assert_eq!(routes.metrics().unwrap().reserved, 0);
}
