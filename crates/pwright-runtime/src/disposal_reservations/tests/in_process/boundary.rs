use super::*;

fn selected_fixture(outbox: bool) -> (Dispatcher, DisposalReservations, ProductionResourceLedger) {
    if outbox {
        fixture(8, 1)
    } else {
        process_fixture(process_profile(1))
    }
}

#[test]
fn busy_boundary_retains_all_admission_without_changing_the_graph() {
    for outbox in [false, true] {
        // Arrange
        let (runtime, pool, resources) = selected_fixture(outbox);
        let reservation = pool.reserve().unwrap();
        let ordinary = runtime.prepare_object_disposal(ObjectId::new(2)).unwrap();
        let before = resources.metrics().unwrap();

        // Act
        let result = reservation
            .prepare_or_defer(&runtime.lifecycle_handle(), ObjectId::new(2))
            .unwrap();

        // Assert
        assert!(matches!(result, DisposalPreparation::Deferred(_)));
        assert_eq!(resources.metrics().unwrap(), before);
        assert_eq!(pool.pending(), 1);
        assert_eq!(runtime.live_object_count(), Ok(3));
        assert_eq!(runtime.lifecycle_transaction_active(), Ok(true));
        assert!(matches!(
            pool.reserve(),
            Err(DisposalReservationError::SlotsExhausted { limit: 1 })
        ));
        drop((ordinary, result));
    }
}

#[test]
fn deferred_preparation_retries_against_the_latest_subtree() {
    for outbox in [false, true] {
        // Arrange
        let (runtime, pool, _) = selected_fixture(outbox);
        let reservation = pool.reserve().unwrap();
        let ordinary = runtime.prepare_object_disposal(ObjectId::new(2)).unwrap();
        let DisposalPreparation::Deferred(reservation) = reservation
            .prepare_or_defer(&runtime.lifecycle_handle(), ObjectId::new(2))
            .unwrap()
        else {
            panic!("busy boundary must retain admission");
        };
        drop(ordinary);
        runtime
            .register_object(
                ObjectId::new(4),
                Some(ObjectId::new(2)),
                ObjectKindTag::new(1),
            )
            .unwrap();
        drain(&runtime);

        // Act
        let result = reservation
            .prepare_or_defer(&runtime.lifecycle_handle(), ObjectId::new(2))
            .unwrap();

        // Assert
        let DisposalPreparation::Prepared(prepared) = result else {
            panic!("released boundary must permit preparation");
        };
        assert_eq!(
            prepared.disposal_order(),
            [ObjectId::new(3), ObjectId::new(4), ObjectId::new(2)]
        );
        assert_eq!(pool.pending(), 1);
        let removed = prepared.commit().unwrap();
        assert_eq!(removed.len(), 3);
        assert_eq!(runtime.live_object_count(), Ok(1));
        assert_eq!(runtime.lifecycle_transaction_active(), Ok(false));
        assert_eq!(pool.pending(), 0);
        assert_eq!(drain(&runtime).len(), if outbox { 3 } else { 0 });
    }
}

#[test]
fn dropping_deferred_preparation_restores_storage_and_queue_capacity() {
    for outbox in [false, true] {
        // Arrange
        let (runtime, pool, resources) = selected_fixture(outbox);
        let before = current(&resources, ResourceClass::BackendStateBytes);
        let reservation = pool.reserve().unwrap();
        let ordinary = runtime.prepare_object_disposal(ObjectId::new(2)).unwrap();
        let deferred = reservation
            .prepare_or_defer(&runtime.lifecycle_handle(), ObjectId::new(2))
            .unwrap();
        assert!(matches!(deferred, DisposalPreparation::Deferred(_)));
        drop(ordinary);

        // Act
        drop(deferred);

        // Assert
        assert_eq!(pool.pending(), 0);
        assert_eq!(
            current(&resources, ResourceClass::BackendStateBytes),
            before
        );
        assert_eq!(current(&resources, ResourceClass::EventItem), 0);
        let replacement = pool.reserve().unwrap();
        assert_eq!(pool.pending(), 1);
        drop(replacement);
    }
}

#[test]
fn transport_closed_is_terminal_even_when_the_commit_boundary_is_busy() {
    for outbox in [false, true] {
        // Arrange
        let (runtime, pool, resources) = selected_fixture(outbox);
        let reservation = pool.reserve().unwrap();
        let ordinary = runtime.prepare_object_disposal(ObjectId::new(2)).unwrap();
        runtime.close_transport().unwrap();

        // Act
        let result = reservation.prepare_or_defer(&runtime.lifecycle_handle(), ObjectId::new(2));

        // Assert
        assert!(matches!(
            result,
            Err(DisposalReservationError::Runtime(
                RuntimeStateError::TransportClosed
            ))
        ));
        assert_eq!(pool.pending(), 0);
        // The unrelated prepared transaction still owns its own event leases.
        assert_eq!(
            current(&resources, ResourceClass::EventItem),
            if outbox { 2 } else { 0 }
        );
        drop(ordinary);
        assert_eq!(current(&resources, ResourceClass::EventItem), 0);
    }
}

#[test]
fn foreign_runtime_is_terminal_and_releases_the_original_admission() {
    for outbox in [false, true] {
        // Arrange
        let (runtime, pool, resources) = selected_fixture(outbox);
        let (foreign, _, _) = selected_fixture(outbox);
        let reservation = pool.reserve().unwrap();

        // Act
        let result = reservation.prepare_or_defer(&foreign.lifecycle_handle(), ObjectId::new(2));

        // Assert
        assert!(matches!(
            result,
            Err(DisposalReservationError::Invalid(
                "disposal reservation belongs to another runtime"
            ))
        ));
        assert_eq!(pool.pending(), 0);
        assert_eq!(current(&resources, ResourceClass::EventItem), 0);
        assert_eq!(runtime.lifecycle_transaction_active(), Ok(false));
        assert_eq!(foreign.lifecycle_transaction_active(), Ok(false));
    }
}
