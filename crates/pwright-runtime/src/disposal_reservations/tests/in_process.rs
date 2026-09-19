use super::*;

mod boundary;

fn process_profile(pending: usize) -> DisposalReservationProfile {
    DisposalReservationProfile::new_in_process(
        NonZeroUsize::new(4).unwrap(),
        NonZeroUsize::new(pending).unwrap(),
        SCOPE,
    )
    .unwrap()
}

fn process_fixture(
    selected: DisposalReservationProfile,
) -> (Dispatcher, DisposalReservations, ProductionResourceLedger) {
    let limits = RuntimeLimits::new(4, 2);
    let resources = resources(
        limits
            .resource_requirements()
            .combine_concurrent(selected.requirements()),
    );
    let runtime =
        ControlledDispatcher::new(PendingService, Cancellation, NoDeadlineScheduler, limits)
            .with_production_resources(&resources)
            .unwrap();
    for (id, parent) in [(1, None), (2, Some(1)), (3, Some(2))] {
        runtime
            .register_object(
                ObjectId::new(id),
                parent.map(ObjectId::new),
                ObjectKindTag::new(1),
            )
            .unwrap();
    }
    let pool = selected
        .bind(&runtime.lifecycle_handle(), &resources)
        .unwrap();
    (runtime, pool, resources)
}

#[test]
fn in_process_reservations_need_no_control_queue_or_event_leases() {
    // Arrange
    let selected = process_profile(2);
    let (runtime, pool, resources) = process_fixture(selected);
    let before = current(&resources, ResourceClass::BackendStateBytes);

    // Act
    let first = pool.reserve().unwrap();
    let second = pool.reserve().unwrap();

    // Assert
    assert_eq!(pool.pending(), 2);
    assert_eq!(runtime.control_message_metrics().unwrap(), None);
    assert_eq!(runtime.pop_control_message().unwrap(), None);
    assert_eq!(runtime.lifecycle_transaction_active(), Ok(false));
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes) - before,
        selected
            .requirements()
            .class(ResourceClass::BackendStateBytes)
            .global
            - POOL_BYTES
    );
    assert!(matches!(
        pool.reserve(),
        Err(DisposalReservationError::SlotsExhausted { limit: 2 })
    ));
    drop((first, second));
    assert_eq!(pool.pending(), 0);
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes),
        before
    );
}

#[test]
fn in_process_admission_can_overlap_an_ordinary_prepared_transaction() {
    // Arrange
    let (runtime, pool, resources) = process_fixture(process_profile(1));
    let ordinary = runtime.prepare_object_disposal(ObjectId::new(2)).unwrap();
    assert_eq!(runtime.lifecycle_transaction_active(), Ok(true));

    // Act
    let reservation = pool.reserve().unwrap();

    // Assert
    assert_eq!(pool.pending(), 1);
    assert_eq!(runtime.live_object_count(), Ok(3));
    assert_eq!(runtime.lifecycle_transaction_active(), Ok(true));
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
    drop(ordinary);
    drop(reservation);
    assert_eq!(pool.pending(), 0);
    assert_eq!(runtime.lifecycle_transaction_active(), Ok(false));
}

#[test]
fn in_process_commit_uses_the_latest_subtree_without_adding_an_outbox() {
    // Arrange
    let (runtime, pool, resources) = process_fixture(process_profile(1));
    let reservation = pool.reserve().unwrap();
    runtime
        .register_object(
            ObjectId::new(4),
            Some(ObjectId::new(2)),
            ObjectKindTag::new(1),
        )
        .unwrap();

    // Act
    let removed = reservation
        .prepare(&runtime.lifecycle_handle(), ObjectId::new(2))
        .unwrap()
        .commit()
        .unwrap();

    // Assert
    assert_eq!(
        removed.iter().map(ObjectRecord::id).collect::<Vec<_>>(),
        [ObjectId::new(3), ObjectId::new(4), ObjectId::new(2)]
    );
    assert_eq!(runtime.live_object_count(), Ok(1));
    assert_eq!(pool.pending(), 0);
    assert_eq!(runtime.control_message_metrics().unwrap(), None);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::EventItem)
            .peak,
        0
    );
}

#[test]
fn in_process_prepare_still_rejects_a_busy_commit_boundary_and_returns_admission() {
    // Arrange
    let (runtime, pool, resources) = process_fixture(process_profile(1));
    let before = current(&resources, ResourceClass::BackendStateBytes);
    let reservation = pool.reserve().unwrap();
    let ordinary = runtime.prepare_object_disposal(ObjectId::new(2)).unwrap();

    // Act
    let result = reservation.prepare(&runtime.lifecycle_handle(), ObjectId::new(2));

    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::LifecycleTransactionInProgress
        ))
    ));
    assert_eq!(pool.pending(), 0);
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes),
        before
    );
    assert_eq!(runtime.live_object_count(), Ok(3));
    assert_eq!(runtime.lifecycle_transaction_active(), Ok(true));
    drop(ordinary);
}

#[test]
fn in_process_profile_rejects_an_enabled_control_queue_before_claiming_storage() {
    // Arrange
    let (runtime, _, resources) = fixture(8, 1);
    let before = resources.metrics().unwrap();

    // Act
    let result = process_profile(1).bind(&runtime.lifecycle_handle(), &resources);

    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Invalid(
            "in-process disposal reservations require a disabled control outbox"
        ))
    ));
    assert_eq!(resources.metrics().unwrap(), before);
}

#[test]
fn legacy_outbox_profile_still_rejects_a_queue_free_runtime() {
    // Arrange
    let (runtime, _, resources) = process_fixture(process_profile(1));
    let before = resources.metrics().unwrap();

    // Act
    let result = profile(1).bind(&runtime.lifecycle_handle(), &resources);

    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Invalid(
            "a controlled outbox is required"
        ))
    ));
    assert_eq!(resources.metrics().unwrap(), before);
}

#[test]
fn in_process_transport_close_is_authoritative_over_reservation_capacity() {
    // Arrange
    let (runtime, pool, resources) = process_fixture(process_profile(1));
    let reservation = pool.reserve().unwrap();
    runtime.close_transport().unwrap();

    // Act
    let result = pool.reserve();

    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::TransportClosed
        ))
    ));
    assert_eq!(pool.pending(), 1);
    drop(reservation);
    assert_eq!(pool.pending(), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
}

#[test]
fn in_process_profile_does_not_charge_for_control_lease_arrays() {
    // Arrange
    let selected = process_profile(2).requirements();
    let wire = profile(2).requirements();

    // Act
    let difference = wire.class(ResourceClass::BackendStateBytes).global
        - selected.class(ResourceClass::BackendStateBytes).global;

    // Assert
    assert_eq!(
        difference,
        (8 * (std::mem::size_of::<ResourceLease>()
            + std::mem::size_of::<pwright_core::ResourceRequest>())) as u64
    );
    assert_eq!(wire.scopes, selected.scopes);
    assert_eq!(wire.leases, selected.leases);
}

#[test]
fn in_process_deferred_commit_keeps_admission_until_notifications_are_released() {
    // Arrange
    let selected = process_profile(1)
        .with_deferred_commit(NonZeroUsize::new(2).unwrap())
        .unwrap();
    let (runtime, pool, resources) = process_fixture(selected);
    let before = current(&resources, ResourceClass::BackendStateBytes);
    let mut call = std::pin::pin!(runtime.call(Command::<TestCommand>::new(
        CallId::new(1),
        ObjectId::new(3),
        CallMetadata::default(),
        0,
    )));
    assert!(poll_once(call.as_mut()).is_pending());
    let prepared = pool
        .reserve()
        .unwrap()
        .prepare(&runtime.lifecycle_handle(), ObjectId::new(2))
        .unwrap();

    // Act
    let ((), mut committed) = prepared
        .commit_deferred_with_publication(None, || Ok::<(), Infallible>(()))
        .unwrap()
        .unwrap();

    // Assert
    assert_eq!(runtime.live_object_count(), Ok(1));
    assert_eq!(runtime.lifecycle_transaction_active(), Ok(false));
    assert_eq!(runtime.control_message_metrics().unwrap(), None);
    assert_eq!(pool.pending(), 1);
    committed.notify();
    let Poll::Ready(Ok(result)) = poll_once(call.as_mut()) else {
        panic!("retired call must finish");
    };
    assert_eq!(
        result.into_outcome().unwrap_err().kind(),
        FailureKind::TargetClosed
    );
    drop(committed);
    assert_eq!(pool.pending(), 0);
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes),
        before
    );
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::EventItem)
            .peak,
        0
    );
}
