use super::*;
use pwright_core::{ResourceLimits, ResourceRequest};

const EPOCH: ConnectionEpoch = ConnectionEpoch::new(1);
const SCOPE: ResourceScope = ResourceScope::extension(124, 1);
const RESERVATIONS: ResourceScope = ResourceScope::extension(124, 2);

fn profiles(limit: usize) -> (CdpTopologyProductionProfile, TransitionReservationProfile) {
    let topology =
        CdpTopologyProductionProfile::new(CdpTopologyLimits::new(32, 4, 4, 8, 8, 32), SCOPE)
            .unwrap();
    let reservations = TransitionReservationProfile::new(
        topology,
        RESERVATIONS,
        NonZeroUsize::new(limit).unwrap(),
    )
    .unwrap();
    (topology, reservations)
}

fn resources(requirements: ResourceRequirements) -> ProductionResourceLedger {
    let bytes = requirements.class(ResourceClass::BackendStateBytes);
    let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
        ResourceClass::BackendStateBytes,
        bytes.global,
        bytes.per_scope,
    );
    ProductionResourceLedger::new(limits, requirements).unwrap()
}

fn fixture(
    limit: usize,
) -> (
    ProductionCdpTopology,
    TransitionReservations,
    ProductionResourceLedger,
) {
    let (profile, reservation_profile) = profiles(limit);
    let resources = resources(
        profile
            .requirements()
            .combine_concurrent(reservation_profile.requirements()),
    );
    let mut owner =
        ProductionCdpTopology::new(ObjectId::new(1), EPOCH, profile, &resources).unwrap();
    owner
        .register_context(
            EPOCH,
            "context-a",
            ObjectId::new(2),
            CdpContextOwnership::Owned,
        )
        .unwrap();
    owner
        .attach_page(
            EPOCH,
            CdpPageTarget::new("target-a", "session-a", "context-a"),
            CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
        )
        .unwrap();
    let pool = reservation_profile.bind(&owner, &resources).unwrap();
    (owner, pool, resources)
}

fn bytes(resources: &ProductionResourceLedger) -> u64 {
    resources
        .metrics()
        .unwrap()
        .class(ResourceClass::BackendStateBytes)
        .current
}

#[test]
fn selected_profile_accounts_for_pending_and_committed_shadows() {
    // Arrange
    let (topology, profile) = profiles(2);
    let expected =
        POOL_STATE_BYTES + 3 * (topology.maximum_retained_native_bytes() + RESERVATION_STATE_BYTES);
    // Act
    let requirements = profile.requirements();
    // Assert
    assert_eq!(
        requirements.class(ResourceClass::BackendStateBytes).global,
        expected
    );
    assert_eq!(
        requirements
            .class(ResourceClass::BackendStateBytes)
            .per_scope,
        expected
    );
    assert_eq!((requirements.scopes, requirements.leases), (1, 4));
    assert_eq!(profile.limit(), 2);
    let state = std::mem::size_of::<ReservationState>()
        + 2 * std::mem::size_of::<usize>()
        + std::mem::size_of::<ResourceRequest>();
    let slot =
        std::mem::size_of::<ReservedTopologyTransition>() + std::mem::size_of::<ResourceRequest>();
    assert!(u64::try_from(state).unwrap() <= POOL_STATE_BYTES);
    assert!(u64::try_from(slot).unwrap() <= RESERVATION_STATE_BYTES);
}

#[test]
fn ordinary_topology_scope_cannot_be_reused_for_reservations() {
    // Arrange
    let (topology, _) = profiles(1);
    // Act
    let result = TransitionReservationProfile::new(topology, SCOPE, NonZeroUsize::new(1).unwrap());
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::InvalidScope { .. })
    ));
}

#[test]
fn overflowing_reservation_profile_fails_before_binding() {
    // Arrange
    let (topology, _) = profiles(1);
    // Act
    let result = TransitionReservationProfile::new(
        topology,
        RESERVATIONS,
        NonZeroUsize::new(usize::MAX).unwrap(),
    );
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::Production(
            CdpTopologyProductionError::CapacityOverflow
        ))
    ));
}

#[test]
fn binding_requires_the_declared_topology_profile() {
    // Arrange
    let (owner, _pool, resources) = fixture(1);
    let different =
        CdpTopologyProductionProfile::new(CdpTopologyLimits::new(8, 1, 1, 2, 1, 8), SCOPE).unwrap();
    let profile =
        TransitionReservationProfile::new(different, RESERVATIONS, NonZeroUsize::new(1).unwrap())
            .unwrap();
    let before = bytes(&resources);
    // Act
    let result = profile.bind(&owner, &resources);
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::ProfileMismatch)
    ));
    assert_eq!(bytes(&resources), before);
}

#[test]
fn reservation_does_not_hold_the_topology_gate() {
    // Arrange
    let (mut owner, pool, _resources) = fixture(1);
    let _reservation = pool.reserve(&owner).unwrap();
    // Act
    let result = owner.register_context(
        EPOCH,
        "context-b",
        ObjectId::new(5),
        CdpContextOwnership::Owned,
    );
    // Assert
    assert!(result.is_ok());
    assert!(!owner.transaction_active());
    assert_eq!(owner.context_object_id("context-b"), Some(ObjectId::new(5)));
    assert_eq!(pool.pending(), 1);
}

#[test]
fn commit_uses_latest_state_and_preserves_a_later_sibling() {
    // Arrange
    let (mut owner, pool, _resources) = fixture(1);
    let reservation = pool.reserve(&owner).unwrap();
    owner
        .register_context(
            EPOCH,
            "context-b",
            ObjectId::new(5),
            CdpContextOwnership::Owned,
        )
        .unwrap();
    let prepared = reservation
        .prepare(&mut owner, |next| next.close_context(EPOCH, "context-a"))
        .unwrap();
    assert_eq!(
        prepared.topology().context_object_id("context-b"),
        Some(ObjectId::new(5))
    );
    assert!(!prepared.output().effects().is_empty());
    // Act
    let result = prepared.commit(&mut owner);
    // Assert
    assert!(result.is_ok());
    assert_eq!(owner.context_object_id("context-a"), None);
    assert_eq!(owner.context_object_id("context-b"), Some(ObjectId::new(5)));
    assert_eq!(pool.pending(), 0);
    assert!(!owner.transaction_active());
}

#[test]
fn parent_commit_includes_a_page_created_after_reservation() {
    // Arrange
    let (mut owner, pool, _resources) = fixture(1);
    let reservation = pool.reserve(&owner).unwrap();
    owner
        .attach_page(
            EPOCH,
            CdpPageTarget::new("target-b", "session-b", "context-a"),
            CdpPageObjectIds::new(ObjectId::new(5), ObjectId::new(6)),
        )
        .unwrap();
    let prepared = reservation
        .prepare(&mut owner, |next| next.close_context(EPOCH, "context-a"))
        .unwrap();
    // Act
    let result = prepared.commit(&mut owner);
    // Assert
    assert!(result.is_ok());
    assert_eq!(owner.page_object_id("target-a"), None);
    assert_eq!(owner.page_object_id("target-b"), None);
    assert_eq!(owner.context_object_id("context-a"), None);
}

#[test]
fn second_commit_does_not_resurrect_a_previously_closed_sibling() {
    // Arrange
    let (mut owner, pool, _resources) = fixture(2);
    owner
        .register_context(
            EPOCH,
            "context-b",
            ObjectId::new(5),
            CdpContextOwnership::Owned,
        )
        .unwrap();
    let first = pool.reserve(&owner).unwrap();
    let second = pool.reserve(&owner).unwrap();
    first
        .prepare(&mut owner, |next| next.close_context(EPOCH, "context-a"))
        .unwrap()
        .commit(&mut owner)
        .unwrap();
    let prepared = second
        .prepare(&mut owner, |next| next.close_context(EPOCH, "context-b"))
        .unwrap();
    // Act
    let result = prepared.commit(&mut owner);
    // Assert
    assert!(result.is_ok());
    assert_eq!(owner.context_object_id("context-a"), None);
    assert_eq!(owner.context_object_id("context-b"), None);
    assert_eq!(pool.pending(), 0);
}

#[test]
fn cloned_pool_cannot_borrow_extra_pending_slots() {
    // Arrange
    let (owner, pool, resources) = fixture(1);
    let _first = pool.reserve(&owner).unwrap();
    let other = pool.clone();
    let before = bytes(&resources);
    // Act
    let result = other.reserve(&owner);
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::SlotsExhausted { limit: 1 })
    ));
    assert_eq!(bytes(&resources), before);
    assert_eq!(other.pending(), 1);
}

#[test]
fn failed_byte_admission_releases_the_pending_slot() {
    // Arrange
    let (owner, pool, resources) = fixture(1);
    let available = resources
        .plan()
        .class(ResourceClass::BackendStateBytes)
        .global
        - bytes(&resources);
    let ledger = resources.claim(ResourceRequirements::none()).unwrap();
    let _held = ledger
        .reserve(RESERVATIONS, ResourceClass::BackendStateBytes, available)
        .unwrap();
    let before = bytes(&resources);
    // Act
    let result = pool.reserve(&owner);
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::Production(
            CdpTopologyProductionError::Resource(_)
        ))
    ));
    assert_eq!(bytes(&resources), before);
    assert_eq!(pool.pending(), 0);
}

#[test]
fn dropped_reservation_releases_only_its_capacity() {
    // Arrange
    let (owner, pool, resources) = fixture(1);
    let before = bytes(&resources);
    let reservation = pool.reserve(&owner).unwrap();
    // Act
    drop(reservation);
    // Assert
    assert_eq!(bytes(&resources), before);
    assert_eq!(pool.pending(), 0);
    assert!(!owner.transaction_active());
    assert_eq!(owner.context_object_id("context-a"), Some(ObjectId::new(2)));
}

#[test]
fn failed_preparation_releases_its_capacity_without_mutation() {
    // Arrange
    let (mut owner, pool, resources) = fixture(1);
    let before = bytes(&resources);
    let reservation = pool.reserve(&owner).unwrap();
    // Act
    let result = reservation.prepare(&mut owner, |next| {
        next.close_context(EPOCH, "context-a")?;
        Err::<(), _>(CdpTopologyError::UnknownPageObject {
            object_id: ObjectId::new(999),
        })
    });
    // Assert
    assert!(result.is_err());
    assert_eq!(bytes(&resources), before);
    assert_eq!(pool.pending(), 0);
    assert!(!owner.transaction_active());
    assert_eq!(owner.context_object_id("context-a"), Some(ObjectId::new(2)));
}

#[test]
fn dropped_prepared_state_releases_gate_and_capacity() {
    // Arrange
    let (mut owner, pool, resources) = fixture(1);
    let before = bytes(&resources);
    let prepared = pool
        .reserve(&owner)
        .unwrap()
        .prepare(&mut owner, |next| next.close_context(EPOCH, "context-a"))
        .unwrap();
    assert!(owner.transaction_active());
    assert_eq!(pool.pending(), 1);
    // Act
    drop(prepared);
    // Assert
    assert!(!owner.transaction_active());
    assert_eq!(pool.pending(), 0);
    assert_eq!(bytes(&resources), before);
    assert_eq!(owner.context_object_id("context-a"), Some(ObjectId::new(2)));
}

#[test]
fn preparing_against_another_owner_rejects_before_calling_the_mutation() {
    // Arrange
    let (owner, pool, resources) = fixture(1);
    let (mut other, _other_pool, _other_resources) = fixture(1);
    let before = bytes(&resources);
    let reservation = pool.reserve(&owner).unwrap();
    // Act
    let result = reservation.prepare::<()>(&mut other, |_| {
        panic!("foreign owner must be rejected before cloning/mutation")
    });
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::Production(
            CdpTopologyProductionError::TransactionOwnerMismatch
        ))
    ));
    assert_eq!(pool.pending(), 0);
    assert_eq!(bytes(&resources), before);
    assert!(!owner.transaction_active());
    assert!(!other.transaction_active());
}

#[test]
fn old_epoch_reservation_is_rejected_before_calling_the_mutation() {
    // Arrange
    let (mut owner, pool, resources) = fixture(1);
    let reservation = pool.reserve(&owner).unwrap();
    owner.begin_reconnect(ConnectionEpoch::new(2)).unwrap();
    // Act
    let result =
        reservation.prepare::<()>(&mut owner, |_| panic!("stale epoch must not run mutation"));
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::EpochChanged {
            reserved: EPOCH,
            ..
        })
    ));
    assert_eq!(pool.pending(), 0);
    assert!(!owner.transaction_active());
    assert_eq!(
        bytes(&resources),
        u64::try_from(owner.retained_native_bytes()).unwrap() + POOL_STATE_BYTES
    );
}

#[test]
fn reserved_mutation_cannot_change_connection_epoch() {
    // Arrange
    let (mut owner, pool, resources) = fixture(1);
    let before = bytes(&resources);
    let reservation = pool.reserve(&owner).unwrap();
    // Act
    let result = reservation.prepare(&mut owner, |next| {
        next.begin_reconnect(ConnectionEpoch::new(2))
    });
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::EpochChanged { .. })
    ));
    assert_eq!(owner.epoch(), EPOCH);
    assert_eq!(bytes(&resources), before);
    assert_eq!(pool.pending(), 0);
}

#[test]
fn exclusive_prepared_transaction_is_not_bypassed() {
    // Arrange
    let (mut owner, pool, resources) = fixture(1);
    let reservation = pool.reserve(&owner).unwrap();
    let ordinary = owner
        .prepare_browser_context_close(ObjectId::new(2))
        .unwrap();
    // Act
    let result = reservation.prepare::<()>(&mut owner, |_| {
        panic!("an active exclusive transaction must be respected")
    });
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::Production(
            CdpTopologyProductionError::TransactionInProgress
        ))
    ));
    assert!(owner.transaction_active());
    assert_eq!(pool.pending(), 0);
    drop(ordinary);
    assert!(!owner.transaction_active());
    assert_eq!(
        bytes(&resources),
        u64::try_from(owner.retained_native_bytes()).unwrap() + POOL_STATE_BYTES
    );
}

#[test]
fn committed_pool_lease_does_not_prevent_refilling_all_pending_slots() {
    // Arrange
    let (mut owner, pool, resources) = fixture(2);
    pool.reserve(&owner)
        .unwrap()
        .prepare(&mut owner, |next| next.close_context(EPOCH, "context-a"))
        .unwrap()
        .commit(&mut owner)
        .unwrap();
    let _first = pool.reserve(&owner).unwrap();
    // Act
    let second = pool.reserve(&owner);
    // Assert
    assert!(second.is_ok());
    assert_eq!(pool.pending(), 2);
    assert_eq!(
        bytes(&resources),
        POOL_STATE_BYTES + 3 * pool.state.bytes_per_slot
    );
}

#[test]
fn all_pool_and_committed_ownership_drains_on_drop() {
    // Arrange
    let (mut owner, pool, resources) = fixture(1);
    pool.reserve(&owner)
        .unwrap()
        .prepare(&mut owner, |next| next.close_context(EPOCH, "context-a"))
        .unwrap()
        .commit(&mut owner)
        .unwrap();
    let owners = (owner, pool);
    // Act
    drop(owners);
    // Assert
    assert_eq!(bytes(&resources), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn ordinary_resource_plan_cannot_enable_the_selected_pool() {
    // Arrange
    let (profile, reservation_profile) = profiles(1);
    let resources = resources(profile.requirements());
    let owner = ProductionCdpTopology::new(ObjectId::new(1), EPOCH, profile, &resources).unwrap();
    // Act
    let result = reservation_profile.bind(&owner, &resources);
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::Production(
            CdpTopologyProductionError::ProductionResource(_)
        ))
    ));
    assert_eq!(bytes(&resources), 0);
}

#[test]
fn reservation_on_another_owner_consumes_no_slot_or_bytes() {
    // Arrange
    let (_owner, pool, resources) = fixture(1);
    let (other, _other_pool, _other_resources) = fixture(1);
    let before = bytes(&resources);
    // Act
    let result = pool.reserve(&other);
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::Production(
            CdpTopologyProductionError::TransactionOwnerMismatch
        ))
    ));
    assert_eq!(pool.pending(), 0);
    assert_eq!(bytes(&resources), before);
}

#[test]
fn commit_to_another_owner_rolls_back_the_original_gate() {
    // Arrange
    let (mut owner, pool, resources) = fixture(1);
    let (mut other, _other_pool, _other_resources) = fixture(1);
    let before = bytes(&resources);
    let prepared = pool
        .reserve(&owner)
        .unwrap()
        .prepare(&mut owner, |next| next.close_context(EPOCH, "context-a"))
        .unwrap();
    // Act
    let result = prepared.commit(&mut other);
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::Production(
            CdpTopologyProductionError::TransactionOwnerMismatch
        ))
    ));
    assert!(!owner.transaction_active());
    assert!(!other.transaction_active());
    assert_eq!(owner.context_object_id("context-a"), Some(ObjectId::new(2)));
    assert_eq!(other.context_object_id("context-a"), Some(ObjectId::new(2)));
    assert_eq!(pool.pending(), 0);
    assert_eq!(bytes(&resources), before);
}

#[test]
fn reserved_preparation_needs_no_additional_byte_lease() {
    // Arrange
    let (mut owner, pool, resources) = fixture(1);
    let reservation = pool.reserve(&owner).unwrap();
    let ledger = resources.claim(ResourceRequirements::none()).unwrap();
    let available = resources
        .plan()
        .class(ResourceClass::BackendStateBytes)
        .global
        - bytes(&resources);
    let _held = ledger
        .reserve(RESERVATIONS, ResourceClass::BackendStateBytes, available)
        .unwrap();
    let before = bytes(&resources);
    // Act
    let result = reservation.prepare(&mut owner, |next| next.close_context(EPOCH, "context-a"));
    // Assert
    assert!(result.is_ok());
    assert_eq!(bytes(&resources), before);
    assert!(owner.transaction_active());
    assert_eq!(pool.pending(), 1);
}

#[test]
fn failed_metadata_admission_releases_the_pending_slot() {
    // Arrange
    let (owner, pool, resources) = fixture(1);
    let ledger = resources.claim(ResourceRequirements::none()).unwrap();
    let available = resources.plan().leases - resources.metrics().unwrap().active_leases;
    let _held = (0..available)
        .map(|_| {
            ledger
                .reserve(RESERVATIONS, ResourceClass::BackendStateBytes, 1)
                .unwrap()
        })
        .collect::<Vec<_>>();
    let before = bytes(&resources);
    // Act
    let result = pool.reserve(&owner);
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::Production(
            CdpTopologyProductionError::Resource(_)
        ))
    ));
    assert_eq!(bytes(&resources), before);
    assert_eq!(pool.pending(), 0);
}

#[test]
fn exhausted_transaction_ids_do_not_leak_a_gate_or_reservation() {
    // Arrange
    let (mut owner, pool, resources) = fixture(1);
    owner.next_transaction_id = u64::MAX;
    let before = bytes(&resources);
    let reservation = pool.reserve(&owner).unwrap();
    // Act
    let result = reservation.prepare(&mut owner, |next| next.close_context(EPOCH, "context-a"));
    // Assert
    assert!(matches!(
        result,
        Err(TransitionReservationError::Production(
            CdpTopologyProductionError::TransactionIdExhausted
        ))
    ));
    assert!(!owner.transaction_active());
    assert_eq!(owner.context_object_id("context-a"), Some(ObjectId::new(2)));
    assert_eq!(pool.pending(), 0);
    assert_eq!(bytes(&resources), before);
}

#[test]
fn concurrent_reservations_share_the_exact_pending_limit() {
    // Arrange
    let (owner, pool, resources) = fixture(1);
    let before = bytes(&resources);
    let barrier = std::sync::Barrier::new(3);
    // Act
    let successes = std::thread::scope(|scope| {
        let attempt = || {
            let reservation = pool.reserve(&owner);
            barrier.wait();
            barrier.wait();
            reservation.is_ok()
        };
        let first = scope.spawn(attempt);
        let second = scope.spawn(attempt);
        barrier.wait();
        let pending = pool.pending();
        barrier.wait();
        (first.join().unwrap(), second.join().unwrap(), pending)
    });
    // Assert
    assert_ne!(successes.0, successes.1);
    assert_eq!(successes.2, 1);
    assert_eq!(pool.pending(), 0);
    assert_eq!(bytes(&resources), before);
}

#[test]
fn existing_pool_can_reserve_a_fresh_epoch_without_reusing_an_old_reservation() {
    // Arrange
    let (mut owner, pool, _resources) = fixture(1);
    owner.begin_reconnect(ConnectionEpoch::new(2)).unwrap();
    let reservation = pool.reserve(&owner).unwrap();
    // Act
    let result = reservation.prepare(&mut owner, |next| Ok(next.epoch()));
    // Assert
    assert_eq!(*result.as_ref().unwrap().output(), ConnectionEpoch::new(2));
    assert_eq!(pool.pending(), 1);
}
