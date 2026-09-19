use super::*;

fn storage(capacity: usize) -> ControlOutboxStorageProfile {
    ControlOutboxStorageProfile::new(
        NonZeroUsize::new(capacity).unwrap(),
        ResourceScope::extension(134, 1),
    )
    .unwrap()
}
fn setup() -> (Dispatcher, ProductionResourceLedger) {
    let limits = RuntimeLimits::new(4, 2).with_control_messages(8);
    let plan = limits
        .resource_requirements()
        .combine_concurrent(storage(8).requirements())
        .combine_concurrent(
            profile(1)
                .with_deferred_commit(NonZeroUsize::new(2).unwrap())
                .unwrap()
                .requirements(),
        );
    let resources = resources(plan);
    let runtime =
        ControlledDispatcher::new(PendingService, Cancellation, NoDeadlineScheduler, limits)
            .with_production_resources(&resources)
            .unwrap();
    (runtime, resources)
}
fn capacities(runtime: &Dispatcher) -> (usize, usize) {
    let state = runtime.store.state.lock().unwrap();
    (
        state
            .control_messages
            .as_ref()
            .unwrap()
            .allocated_capacity(),
        state.control_resources.capacity(),
    )
}
fn register(runtime: &Dispatcher, id: u64, parent: Option<u64>) {
    runtime
        .register_object(
            ObjectId::new(id),
            parent.map(ObjectId::new),
            ObjectKindTag::new(1),
        )
        .unwrap();
}

#[test]
fn installation_preserves_queued_messages_leases_and_metrics() {
    // Arrange
    let (runtime, resources) = setup();
    register(&runtime, 1, None);
    register(&runtime, 2, Some(1));
    let metrics = runtime.control_message_metrics().unwrap();
    let items = current(&resources, ResourceClass::EventItem);
    // Act
    storage(8)
        .install(&runtime.lifecycle_handle(), &resources)
        .unwrap();
    // Assert
    assert_eq!(runtime.control_message_metrics().unwrap(), metrics);
    assert_eq!(current(&resources, ResourceClass::EventItem), items);
    assert!(capacities(&runtime).0 >= 8 && capacities(&runtime).1 >= 8);
    let messages = drain(&runtime);
    assert_eq!(
        messages,
        vec![
            ControlMessage::Create {
                object_id: ObjectId::new(1),
                parent: None,
                kind: ObjectKindTag::new(1)
            },
            ControlMessage::Create {
                object_id: ObjectId::new(2),
                parent: Some(ObjectId::new(1)),
                kind: ObjectKindTag::new(1)
            },
        ]
    );
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
    drop(runtime);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn deferred_close_and_queue_refill_do_not_grow_preallocated_buffers() {
    // Arrange
    let (runtime, resources) = setup();
    storage(8)
        .install(&runtime.lifecycle_handle(), &resources)
        .unwrap();
    let capacity = capacities(&runtime);
    let selected = profile(1)
        .with_deferred_commit(NonZeroUsize::new(2).unwrap())
        .unwrap();
    let pool = selected
        .bind(&runtime.lifecycle_handle(), &resources)
        .unwrap();
    for id in 1..=4 {
        register(&runtime, id, (id != 1).then_some(1));
    }
    // Four creation messages coexist with the four reserved disposal messages.
    let prepared = pool
        .reserve()
        .unwrap()
        .prepare(&runtime.lifecycle_handle(), ObjectId::new(1))
        .unwrap();
    // Act
    let mut committed = prepared.commit_deferred().unwrap();
    // Assert
    assert_eq!(
        runtime
            .control_message_metrics()
            .unwrap()
            .unwrap()
            .current_len,
        8
    );
    assert_eq!(capacities(&runtime), capacity);
    committed.notify();
    assert_eq!(drain(&runtime).len(), 8);
    assert_eq!(capacities(&runtime), capacity);
    drop((committed, pool, runtime));
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn storage_lease_follows_runtime_handles_after_pool_drop_and_transport_close() {
    // Arrange
    let (runtime, resources) = setup();
    storage(8)
        .install(&runtime.lifecycle_handle(), &resources)
        .unwrap();
    let pool = profile(1)
        .bind(&runtime.lifecycle_handle(), &resources)
        .unwrap();
    let handle = runtime.lifecycle_handle();
    drop(pool);
    let storage_bytes = storage(8)
        .requirements()
        .class(ResourceClass::BackendStateBytes)
        .global;
    // Act
    runtime.close_transport().unwrap();
    drop(runtime);
    // Assert
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes),
        storage_bytes
    );
    drop(handle);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn duplicate_storage_selection_does_not_reallocate_or_reserve_twice() {
    // Arrange
    let (runtime, resources) = setup();
    storage(8)
        .install(&runtime.lifecycle_handle(), &resources)
        .unwrap();
    let before = resources.metrics().unwrap();
    let capacity = capacities(&runtime);
    // Act
    let result = storage(8).install(&runtime.lifecycle_handle(), &resources);
    // Assert
    assert!(result.is_err());
    assert_eq!(resources.metrics().unwrap(), before);
    assert_eq!(capacities(&runtime), capacity);
}

#[test]
fn wrong_queue_limit_rejects_before_storage_admission() {
    // Arrange
    let (runtime, resources) = setup();
    let before = resources.metrics().unwrap();
    let capacity = capacities(&runtime);
    // Act
    let result = storage(7).install(&runtime.lifecycle_handle(), &resources);
    // Assert
    assert!(result.is_err());
    assert_eq!(resources.metrics().unwrap(), before);
    assert_eq!(capacities(&runtime), capacity);
}

#[test]
fn outstanding_disposal_capacity_prevents_late_outbox_installation() {
    // Arrange
    let (runtime, resources) = setup();
    let pool = profile(1)
        .bind(&runtime.lifecycle_handle(), &resources)
        .unwrap();
    let _reservation = pool.reserve().unwrap();
    let before = resources.metrics().unwrap();
    // Act
    let result = storage(8).install(&runtime.lifecycle_handle(), &resources);
    // Assert
    assert!(result.is_err());
    assert_eq!(capacities(&runtime), (0, 0));
    assert_eq!(resources.metrics().unwrap(), before);
}

#[test]
fn outstanding_message_claim_prevents_installation_without_losing_the_message() {
    // Arrange
    let (runtime, resources) = setup();
    register(&runtime, 1, None);
    let claim = runtime.claim_control_message().unwrap().unwrap();
    let capacity = capacities(&runtime);
    let before = resources.metrics().unwrap();
    // Act
    let result = storage(8).install(&runtime.lifecycle_handle(), &resources);
    // Assert
    assert!(result.is_err());
    assert_eq!(resources.metrics().unwrap(), before);
    assert_eq!(capacities(&runtime), capacity);
    drop(claim);
    assert_eq!(drain(&runtime).len(), 1);
}

#[test]
fn stopped_runtime_rejects_storage_without_allocation() {
    // Arrange
    let (runtime, resources) = setup();
    runtime.close_transport().unwrap();
    let before = resources.metrics().unwrap();
    // Act
    let result = storage(8).install(&runtime.lifecycle_handle(), &resources);
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::TransportClosed
        ))
    ));
    assert_eq!(capacities(&runtime), (0, 0));
    assert_eq!(resources.metrics().unwrap(), before);
}

#[test]
fn undeclared_storage_budget_fails_without_backing_mutation() {
    // Arrange
    let (runtime, _) = setup();
    let undeclared = resources(ResourceRequirements::none());
    let capacity = capacities(&runtime);
    // Act
    let result = storage(8).install(&runtime.lifecycle_handle(), &undeclared);
    // Assert
    assert!(result.is_err());
    assert_eq!(capacities(&runtime), capacity);
    assert_eq!(undeclared.metrics().unwrap().active_leases, 0);
}

#[test]
fn storage_profile_rejects_invalid_scope_and_overflow() {
    // Arrange
    let one = NonZeroUsize::MIN;
    // Act
    let wrong_scope = ControlOutboxStorageProfile::new(one, ResourceScope::Connection);
    let overflow = ControlOutboxStorageProfile::new(NonZeroUsize::new(usize::MAX).unwrap(), SCOPE);
    // Assert
    assert!(wrong_scope.is_err());
    assert!(overflow.is_err());
}

#[test]
fn byte_budget_exhaustion_preserves_both_buffers_and_queued_messages() {
    // Arrange
    let (runtime, resources) = setup();
    register(&runtime, 1, None);
    let ledger = resources.claim(ResourceRequirements::none()).unwrap();
    let _held = ledger
        .reserve(
            SCOPE,
            ResourceClass::BackendStateBytes,
            resources
                .plan()
                .class(ResourceClass::BackendStateBytes)
                .per_scope,
        )
        .unwrap();
    let before = resources.metrics().unwrap();
    let capacity = capacities(&runtime);
    // Act
    let result = storage(8).install(&runtime.lifecycle_handle(), &resources);
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::Resource(_)
        ))
    ));
    assert_eq!(capacities(&runtime), capacity);
    assert_eq!(
        resources.metrics().unwrap().active_leases,
        before.active_leases
    );
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes),
        before.class(ResourceClass::BackendStateBytes).current
    );
    assert_eq!(drain(&runtime).len(), 1);
}

#[test]
fn active_lifecycle_transaction_prevents_outbox_storage_changes() {
    // Arrange
    let (runtime, resources) = setup();
    register(&runtime, 1, None);
    drain(&runtime);
    let _prepared = runtime.prepare_object_disposal(ObjectId::new(1)).unwrap();
    let capacity = capacities(&runtime);
    let before = resources.metrics().unwrap();
    // Act
    let result = storage(8).install(&runtime.lifecycle_handle(), &resources);
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::LifecycleTransactionInProgress
        ))
    ));
    assert_eq!(capacities(&runtime), capacity);
    assert_eq!(resources.metrics().unwrap(), before);
}

#[test]
fn unbudgeted_runtime_cannot_install_outbox_storage() {
    // Arrange
    let runtime = ControlledDispatcher::new(
        PendingService,
        Cancellation,
        NoDeadlineScheduler,
        RuntimeLimits::new(4, 2).with_control_messages(8),
    );
    let resources = resources(storage(8).requirements());
    // Act
    let result = storage(8).install(&runtime.lifecycle_handle(), &resources);
    // Assert
    assert!(result.is_err());
    assert_eq!(capacities(&runtime), (0, 0));
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn runtime_without_control_outbox_cannot_install_storage() {
    // Arrange
    let limits = RuntimeLimits::new(4, 2);
    let resources = resources(
        limits
            .resource_requirements()
            .combine_concurrent(storage(8).requirements()),
    );
    let runtime =
        ControlledDispatcher::new(PendingService, Cancellation, NoDeadlineScheduler, limits)
            .with_production_resources(&resources)
            .unwrap();
    let before = resources.metrics().unwrap();
    // Act
    let result = storage(8).install(&runtime.lifecycle_handle(), &resources);
    // Assert
    assert!(result.is_err());
    assert_eq!(resources.metrics().unwrap(), before);
}
