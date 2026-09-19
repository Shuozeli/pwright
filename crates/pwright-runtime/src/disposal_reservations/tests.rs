use super::*;
use pwright_core::{MethodTag, ResourceLimits};

const SCOPE: ResourceScope = ResourceScope::extension(125, 1);
mod in_process;
mod outbox_storage;
struct PendingService;
impl Service<usize> for PendingService {
    type Response = usize;
    type Error = OperationError<()>;
    type Future<'a> = Pending<Result<usize, OperationError<()>>>;
    fn call(&self, _: usize) -> Self::Future<'_> {
        std::future::pending()
    }
}
struct TestCommand;
impl CommandSpec for TestCommand {
    const METHOD: MethodTag = MethodTag::new(1);
    type Params = usize;
    type Result = usize;
    type ErrorDetails = ();
}
struct Cancellation;
impl CommandCancellation<TestCommand> for Cancellation {
    type Future<'a> = std::future::Ready<Result<(), OperationError<()>>>;
    fn cancel(&self, _: OperationContext, _: CancellationReason) -> Self::Future<'_> {
        std::future::ready(Ok(()))
    }
}
type Dispatcher = ControlledDispatcher<PendingService, Cancellation, NoDeadlineScheduler>;

fn profile(pending: usize) -> DisposalReservationProfile {
    DisposalReservationProfile::new(
        NonZeroUsize::new(4).unwrap(),
        NonZeroUsize::new(pending).unwrap(),
        SCOPE,
    )
    .unwrap()
}
fn resources(plan: ResourceRequirements) -> ProductionResourceLedger {
    let mut limits = ResourceLimits::deny_all(plan.scopes, plan.leases);
    for class in ResourceClass::ALL {
        let amount = plan.class(class);
        limits = limits.with_class(class, amount.global, amount.per_scope);
    }
    ProductionResourceLedger::new(limits, plan).unwrap()
}
fn fixture(
    queue: usize,
    pending: usize,
) -> (Dispatcher, DisposalReservations, ProductionResourceLedger) {
    fixture_with_profile(queue, profile(pending))
}
fn fixture_with_profile(
    queue: usize,
    selected: DisposalReservationProfile,
) -> (Dispatcher, DisposalReservations, ProductionResourceLedger) {
    let limits = RuntimeLimits::new(4, 2).with_control_messages(queue);
    let resources = resources(
        limits
            .resource_requirements()
            .combine_concurrent(selected.requirements()),
    );
    let dispatcher =
        ControlledDispatcher::new(PendingService, Cancellation, NoDeadlineScheduler, limits)
            .with_production_resources(&resources)
            .unwrap();
    for (id, parent) in [(1, None), (2, Some(1)), (3, Some(2))] {
        dispatcher
            .register_object(
                ObjectId::new(id),
                parent.map(ObjectId::new),
                ObjectKindTag::new(1),
            )
            .unwrap();
    }
    drain(&dispatcher);
    let pool = selected
        .bind(&dispatcher.lifecycle_handle(), &resources)
        .unwrap();
    (dispatcher, pool, resources)
}
fn drain(dispatcher: &Dispatcher) -> Vec<ControlMessage> {
    let mut messages = Vec::new();
    while let Some(message) = dispatcher.pop_control_message().unwrap() {
        messages.push(message);
    }
    messages
}
fn current(resources: &ProductionResourceLedger, class: ResourceClass) -> u64 {
    resources.metrics().unwrap().class(class).current
}
fn slots(dispatcher: &Dispatcher) -> usize {
    dispatcher
        .store
        .state
        .lock()
        .unwrap()
        .reserved_control_slots
}
fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

mod deferred;

#[test]
fn waiting_reservation_owns_queue_and_event_capacity_without_a_gate() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(8, 1);
    // Act
    let _reservation = pool.reserve().unwrap();
    // Assert
    assert_eq!(slots(&dispatcher), 4);
    assert_eq!(current(&resources, ResourceClass::EventItem), 4);
    assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(false));
    assert_eq!(dispatcher.live_object_count(), Ok(3));
    assert_eq!(pool.pending(), 1);
}

#[test]
fn ordinary_creation_cannot_consume_reserved_queue_slots() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(4, 1);
    let _reservation = pool.reserve().unwrap();
    // Act
    let result = dispatcher.register_object(
        ObjectId::new(4),
        Some(ObjectId::new(1)),
        ObjectKindTag::new(1),
    );
    // Assert
    assert!(matches!(
        result,
        Err(RuntimeStateError::ControlMessageCapacityExceeded { remaining: 0, .. })
    ));
    assert_eq!(dispatcher.is_object_live(ObjectId::new(4)), Ok(false));
    assert_eq!(current(&resources, ResourceClass::EventItem), 4);
}

#[test]
fn dropped_reservation_restores_queue_and_resource_capacity() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(4, 1);
    let before = current(&resources, ResourceClass::BackendStateBytes);
    let reservation = pool.reserve().unwrap();
    // Act
    drop(reservation);
    // Assert
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes),
        before
    );
    assert!(
        dispatcher
            .register_object(ObjectId::new(4), None, ObjectKindTag::new(1))
            .is_ok()
    );
}

#[test]
fn preparing_uses_the_latest_subtree_including_later_children() {
    // Arrange
    let (dispatcher, pool, _resources) = fixture(8, 1);
    let reservation = pool.reserve().unwrap();
    dispatcher
        .register_object(
            ObjectId::new(4),
            Some(ObjectId::new(2)),
            ObjectKindTag::new(1),
        )
        .unwrap();
    drain(&dispatcher);
    // Act
    let prepared = reservation
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2))
        .unwrap();
    // Assert
    assert_eq!(
        prepared.disposal_order(),
        &[ObjectId::new(3), ObjectId::new(4), ObjectId::new(2)]
    );
    assert_eq!(dispatcher.live_object_count(), Ok(4));
    assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(true));
}

#[test]
fn committed_disposal_transfers_reserved_events_to_the_outbox() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(4, 1);
    let prepared = pool
        .reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(1))
        .unwrap();
    // Act
    let removed = prepared.commit().unwrap();
    // Assert
    assert_eq!(
        removed.iter().map(ObjectRecord::id).collect::<Vec<_>>(),
        [ObjectId::new(3), ObjectId::new(2), ObjectId::new(1)]
    );
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 0);
    assert_eq!(current(&resources, ResourceClass::LiveObject), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 3);
    let ledger = resources.claim(ResourceRequirements::none()).unwrap();
    assert_eq!(
        ledger
            .scope_metrics(SCOPE)
            .unwrap()
            .unwrap()
            .units(ResourceClass::EventItem),
        3
    );
    assert_eq!(
        drain(&dispatcher),
        vec![
            ControlMessage::Dispose {
                object_id: ObjectId::new(3)
            },
            ControlMessage::Dispose {
                object_id: ObjectId::new(2)
            },
            ControlMessage::Dispose {
                object_id: ObjectId::new(1)
            }
        ]
    );
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
}

#[test]
fn prepared_drop_preserves_objects_and_releases_all_admission() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(4, 1);
    let prepared = pool
        .reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(1))
        .unwrap();
    // Act
    drop(prepared);
    // Assert
    assert_eq!(dispatcher.live_object_count(), Ok(3));
    assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(false));
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
}

#[test]
fn another_runtime_cannot_consume_a_disposal_reservation() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(4, 1);
    let (other, _pool, _resources) = fixture(4, 1);
    let reservation = pool.reserve().unwrap();
    // Act
    let result = reservation.prepare(&other.lifecycle_handle(), ObjectId::new(1));
    // Assert
    assert!(matches!(result, Err(DisposalReservationError::Invalid(_))));
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
    assert_eq!(other.live_object_count(), Ok(3));
}

#[test]
fn owner_shutdown_rejects_waiting_preparation_and_reclaims_slots() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(4, 1);
    let reservation = pool.reserve().unwrap();
    dispatcher.close_transport().unwrap();
    // Act
    let result = reservation.prepare(&dispatcher.lifecycle_handle(), ObjectId::new(1));
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::TransportClosed
        ))
    ));
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
}

#[test]
fn owner_shutdown_invalidates_a_prepared_disposal_without_double_releasing_slots() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(4, 1);
    let prepared = pool
        .reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(1))
        .unwrap();
    dispatcher.close_transport().unwrap();
    // Act
    let result = prepared.commit();
    // Assert
    assert_eq!(
        result,
        Err(RuntimeStateError::LifecycleTransactionUnavailable)
    );
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
}

#[test]
fn clone_cannot_admit_beyond_the_shared_pending_limit() {
    // Arrange
    let (_dispatcher, pool, resources) = fixture(8, 1);
    let _first = pool.reserve().unwrap();
    let cloned = pool.clone();
    let before = resources.metrics().unwrap();
    // Act
    let result = cloned.reserve();
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::SlotsExhausted { limit: 1 })
    ));
    assert_eq!(pool.pending(), 1);
    assert_eq!(resources.metrics().unwrap(), before);
}

#[test]
fn reservation_storage_layouts_fit_the_declared_allowances() {
    // Arrange
    let pool = std::mem::size_of::<Pool>()
        + 2 * std::mem::size_of::<usize>()
        + std::mem::size_of::<pwright_core::ResourceRequest>();
    let waiting = std::mem::size_of::<ReservedDisposalCapacity>()
        + std::mem::size_of::<pwright_core::ResourceRequest>();
    let prepared = std::mem::size_of::<ReservedPreparedDisposal>()
        + std::mem::size_of::<pwright_core::ResourceRequest>();
    // Act
    let declared = profile(2).requirements();
    // Assert
    assert!(
        u64::try_from(pool).unwrap() <= POOL_BYTES,
        "pool layout: {pool}"
    );
    assert!(
        u64::try_from(waiting).unwrap() <= RESERVATION_BYTES,
        "waiting layout: {waiting}"
    );
    assert!(
        u64::try_from(prepared).unwrap() <= RESERVATION_BYTES,
        "prepared layout: {prepared}"
    );
    assert_eq!(declared.class(ResourceClass::EventItem).global, 0);
    assert_eq!((declared.scopes, declared.leases), (1, 3));
}

#[test]
fn queued_committed_events_reduce_capacity_for_the_next_reservation() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(4, 1);
    pool.reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(3))
        .unwrap()
        .commit()
        .unwrap();
    // Act
    let result = pool.reserve();
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::ControlMessageCapacityExceeded { remaining: 3, .. }
        ))
    ));
    assert_eq!(pool.pending(), 0);
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 1);
}

#[test]
fn waiting_reservation_can_prepare_after_an_ancestor_has_committed() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(8, 2);
    let parent = pool.reserve().unwrap();
    let child = pool.reserve().unwrap();
    parent
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2))
        .unwrap()
        .commit()
        .unwrap();
    let prepared = child
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(3))
        .unwrap();
    assert!(prepared.disposal_order().is_empty());
    // Act
    let result = prepared.commit().unwrap();
    // Assert
    assert!(result.is_empty());
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 2);
    assert_eq!(drain(&dispatcher).len(), 2);
}

#[test]
fn parent_preparation_after_child_commit_does_not_duplicate_child_disposal() {
    // Arrange
    let (dispatcher, pool, _resources) = fixture(8, 2);
    let parent = pool.reserve().unwrap();
    pool.reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(3))
        .unwrap()
        .commit()
        .unwrap();
    let prepared = parent
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2))
        .unwrap();
    // Act
    let result = prepared.commit().unwrap();
    // Assert
    assert_eq!(
        result.iter().map(ObjectRecord::id).collect::<Vec<_>>(),
        [ObjectId::new(2)]
    );
    assert_eq!(
        drain(&dispatcher),
        vec![
            ControlMessage::Dispose {
                object_id: ObjectId::new(3)
            },
            ControlMessage::Dispose {
                object_id: ObjectId::new(2)
            }
        ]
    );
    assert_eq!(dispatcher.is_object_live(ObjectId::new(1)), Ok(true));
}

#[test]
fn committed_parent_disposal_cancels_pending_descendant_calls() {
    // Arrange
    let (dispatcher, pool, _resources) = fixture(8, 1);
    let mut call = std::pin::pin!(dispatcher.call(Command::<TestCommand>::new(
        CallId::new(1),
        ObjectId::new(3),
        CallMetadata::default(),
        0
    )));
    assert!(poll_once(call.as_mut()).is_pending());
    let prepared = pool
        .reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2))
        .unwrap();
    // Act
    prepared.commit().unwrap();
    // Assert
    let Poll::Ready(Ok(result)) = poll_once(call.as_mut()) else {
        panic!("descendant call must settle");
    };
    assert_eq!(
        result.into_outcome().unwrap_err().kind(),
        FailureKind::TargetClosed
    );
    assert_eq!(dispatcher.pending_call_count(), Ok(0));
}

#[test]
fn committed_typed_failure_reaches_pending_descendant_calls() {
    // Arrange
    let (dispatcher, pool, _resources) = fixture(8, 1);
    let mut call = std::pin::pin!(dispatcher.call(Command::<TestCommand>::new(
        CallId::new(1),
        ObjectId::new(3),
        CallMetadata::default(),
        0
    )));
    assert!(poll_once(call.as_mut()).is_pending());
    let prepared = pool
        .reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2))
        .unwrap();
    // Act
    prepared
        .commit_with_failure(RuntimeCallFailure::new(
            FailureKind::Backend,
            "native retirement failed",
        ))
        .unwrap();
    // Assert
    let Poll::Ready(Ok(result)) = poll_once(call.as_mut()) else {
        panic!("descendant call must settle");
    };
    let error = result.into_outcome().unwrap_err();
    assert_eq!(error.kind(), FailureKind::Backend);
    assert_eq!(error.message(), "native retirement failed");
    assert_eq!(dispatcher.pending_call_count(), Ok(0));
}

#[test]
fn an_existing_exclusive_transaction_blocks_admission_without_spending_slots() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(8, 1);
    let _prepared = dispatcher
        .prepare_object_disposal(ObjectId::new(3))
        .unwrap();
    let before = current(&resources, ResourceClass::EventItem);
    // Act
    let result = pool.reserve();
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::LifecycleTransactionInProgress
        ))
    ));
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), before);
}

#[test]
fn a_later_exclusive_transaction_rejects_preparation_and_preserves_its_gate() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(8, 1);
    let reservation = pool.reserve().unwrap();
    let ordinary = dispatcher
        .prepare_object_disposal(ObjectId::new(3))
        .unwrap();
    // Act
    let result = reservation.prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2));
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::LifecycleTransactionInProgress
        ))
    ));
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 1);
    assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(true));
    drop(ordinary);
}

#[test]
fn a_dropped_runtime_is_not_retained_by_the_pool_or_waiting_reservation() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(4, 1);
    let weak = Arc::downgrade(&dispatcher.store);
    let reservation = pool.reserve().unwrap();
    // Act
    drop(dispatcher);
    // Assert
    assert!(weak.upgrade().is_none());
    drop(reservation);
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
    drop(pool);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn transport_closed_pool_rejects_new_reservations() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(4, 1);
    dispatcher.close_transport().unwrap();
    // Act
    let result = pool.reserve();
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::TransportClosed
        ))
    ));
    assert_eq!(pool.pending(), 0);
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
}

#[test]
fn partial_event_admission_failure_releases_every_new_lease_and_slot() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(8, 1);
    let ledger = resources.claim(ResourceRequirements::none()).unwrap();
    let _held = ledger.reserve(SCOPE, ResourceClass::EventItem, 6).unwrap();
    let before_bytes = current(&resources, ResourceClass::BackendStateBytes);
    // Act
    let result = pool.reserve();
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::Resource(_)
        ))
    ));
    assert_eq!(current(&resources, ResourceClass::EventItem), 6);
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes),
        before_bytes
    );
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 0);
}

#[test]
fn metadata_exhaustion_does_not_leak_a_pending_permit() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(8, 1);
    let ledger = resources.claim(ResourceRequirements::none()).unwrap();
    let remaining = resources.plan().leases - resources.metrics().unwrap().active_leases;
    let _held = (0..remaining)
        .map(|_| {
            ledger
                .reserve(SCOPE, ResourceClass::BackendStateBytes, 1)
                .unwrap()
        })
        .collect::<Vec<_>>();
    let before = current(&resources, ResourceClass::BackendStateBytes);
    // Act
    let result = pool.reserve();
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::Resource(_)
        ))
    ));
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes),
        before
    );
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 0);
}

#[test]
fn empty_graph_with_waiting_capacity_cannot_replace_its_resource_ledger() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(8, 1);
    dispatcher.dispose_object(ObjectId::new(1)).unwrap();
    drain(&dispatcher);
    let _reservation = pool.reserve().unwrap();
    let ledger = resources.claim(ResourceRequirements::none()).unwrap();
    // Act
    let result = dispatcher.store.install_resource_ledger(ledger);
    // Assert
    assert_eq!(
        result,
        Err(RuntimeStateError::ResourceLedgerRequiresEmptyState)
    );
    assert_eq!(slots(&dispatcher), 4);
    assert_eq!(current(&resources, ResourceClass::EventItem), 4);
}

#[test]
fn profile_that_cannot_fit_all_pending_outbox_slots_is_rejected() {
    // Arrange
    let (dispatcher, _pool, resources) = fixture(4, 1);
    let before = resources.metrics().unwrap();
    // Act
    let result = profile(2).bind(&dispatcher.lifecycle_handle(), &resources);
    // Assert
    assert!(matches!(result, Err(DisposalReservationError::Invalid(_))));
    assert_eq!(resources.metrics().unwrap(), before);
}

#[test]
fn profile_must_match_the_runtime_live_object_limit() {
    // Arrange
    let (dispatcher, _pool, resources) = fixture(8, 1);
    let selected = DisposalReservationProfile::new(
        NonZeroUsize::new(3).unwrap(),
        NonZeroUsize::new(1).unwrap(),
        SCOPE,
    )
    .unwrap();
    // Act
    let result = selected.bind(&dispatcher.lifecycle_handle(), &resources);
    // Assert
    assert!(matches!(result, Err(DisposalReservationError::Invalid(_))));
}

#[test]
fn count_overflow_is_rejected_before_any_pool_allocation() {
    // Arrange
    let maximum = NonZeroUsize::new(usize::MAX).unwrap();
    // Act
    let result = DisposalReservationProfile::new(maximum, maximum, SCOPE);
    // Assert
    assert!(matches!(result, Err(DisposalReservationError::Invalid(_))));
}

#[test]
fn preparing_with_exhausted_transaction_ids_releases_capacity() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(4, 1);
    let reservation = pool.reserve().unwrap();
    dispatcher
        .store
        .state
        .lock()
        .unwrap()
        .next_lifecycle_transaction_id = u64::MAX;
    // Act
    let result = reservation.prepare(&dispatcher.lifecycle_handle(), ObjectId::new(1));
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::LifecycleTransactionIdExhausted
        ))
    ));
    assert_eq!(pool.pending(), 0);
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
    assert_eq!(dispatcher.live_object_count(), Ok(3));
}

#[test]
fn concurrent_clones_share_one_pending_slot() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(8, 1);
    let barrier = std::sync::Barrier::new(3);
    // Act
    let outcomes = std::thread::scope(|scope| {
        let attempt = || {
            let result = pool.reserve();
            barrier.wait();
            barrier.wait();
            result.is_ok()
        };
        let first = scope.spawn(attempt);
        let second = scope.spawn(attempt);
        barrier.wait();
        let pending = pool.pending();
        barrier.wait();
        (first.join().unwrap(), second.join().unwrap(), pending)
    });
    // Assert
    assert_ne!(outcomes.0, outcomes.1);
    assert_eq!(outcomes.2, 1);
    assert_eq!(pool.pending(), 0);
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
}

#[test]
fn transport_closure_takes_precedence_over_a_full_reservation_pool() {
    // Arrange
    let (dispatcher, pool, resources) = fixture(4, 1);
    let _reservation = pool.reserve().unwrap();
    dispatcher.close_transport().unwrap();
    let before = resources.metrics().unwrap();
    // Act
    let result = pool.reserve();
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::TransportClosed
        ))
    ));
    assert_eq!(resources.metrics().unwrap(), before);
}

#[cfg(feature = "lifetime-handoff")]
#[test]
fn disposal_respects_a_pending_calls_transferred_lifetime_owner() {
    // Arrange
    let (dispatcher, pool, _resources) = fixture(8, 1);
    let mut call = std::pin::pin!(dispatcher.call(Command::<TestCommand>::new(
        CallId::new(1),
        ObjectId::new(3),
        CallMetadata::default(),
        0
    )));
    assert!(poll_once(call.as_mut()).is_pending());
    dispatcher
        .prepare_lifetime_handoff(CallId::new(1), ObjectId::new(3), ObjectId::new(1))
        .unwrap()
        .commit()
        .unwrap();
    let prepared = pool
        .reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2))
        .unwrap();
    // Act
    prepared.commit().unwrap();
    // Assert
    assert!(poll_once(call.as_mut()).is_pending());
    assert_eq!(dispatcher.is_object_live(ObjectId::new(1)), Ok(true));
    dispatcher.close_transport().unwrap();
    assert!(poll_once(call.as_mut()).is_ready());
    assert_eq!(dispatcher.pending_call_count(), Ok(0));
}
