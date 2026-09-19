use super::*;
use std::sync::atomic::AtomicUsize;
use std::task::Wake;
mod publication;

fn selected() -> DisposalReservationProfile {
    profile(1)
        .with_deferred_commit(NonZeroUsize::new(2).unwrap())
        .unwrap()
}
fn fixture() -> (Dispatcher, DisposalReservations, ProductionResourceLedger) {
    fixture_with_profile(8, selected())
}
struct ReentrantWake {
    outer: Arc<Mutex<()>>,
    handle: ControlledLifecycleHandle,
    count: AtomicUsize,
}
impl Wake for ReentrantWake {
    fn wake(self: Arc<Self>) {
        assert!(
            self.outer.try_lock().is_ok(),
            "outer commit lock still held"
        );
        assert_eq!(self.handle.is_object_live(ObjectId::new(2)), Ok(false));
        self.count.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn deferred_commit_installs_termination_but_wakes_only_after_outer_unlock() {
    // Arrange
    let (dispatcher, pool, resources) = fixture();
    let outer = Arc::new(Mutex::new(()));
    let wake = Arc::new(ReentrantWake {
        outer: outer.clone(),
        handle: dispatcher.lifecycle_handle(),
        count: AtomicUsize::new(0),
    });
    let waker = Waker::from(wake.clone());
    let mut call = std::pin::pin!(dispatcher.call(Command::<TestCommand>::new(
        CallId::new(1),
        ObjectId::new(3),
        CallMetadata::default(),
        0,
    )));
    assert!(
        call.as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    let prepared = pool
        .reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2))
        .unwrap();
    let outer_guard = outer.lock().unwrap();
    // Act
    let mut committed = prepared.commit_deferred().unwrap();
    // Assert
    assert_eq!(wake.count.load(Ordering::SeqCst), 0);
    assert_eq!(dispatcher.live_object_count(), Ok(1));
    assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(false));
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 1);
    assert_eq!(
        committed
            .removed()
            .iter()
            .map(ObjectRecord::id)
            .collect::<Vec<_>>(),
        [ObjectId::new(3), ObjectId::new(2)]
    );
    let signal = dispatcher
        .store
        .state
        .lock()
        .unwrap()
        .cancellation_signals
        .get(&CallId::new(1))
        .unwrap()
        .clone();
    assert!(matches!(
        signal.termination.get(),
        Some(CallTermination::Cancellation(
            CancellationReason::ParentClosed
        ))
    ));
    drop(outer_guard);
    committed.notify();
    committed.notify();
    assert_eq!(wake.count.load(Ordering::SeqCst), 1);
    let Poll::Ready(Ok(result)) = poll_once(call.as_mut()) else {
        panic!("call should settle");
    };
    assert_eq!(
        result.into_outcome().unwrap_err().kind(),
        FailureKind::TargetClosed
    );
    let before_drop = current(&resources, ResourceClass::BackendStateBytes);
    drop(committed);
    assert_eq!(pool.pending(), 0);
    assert_eq!(
        before_drop - current(&resources, ResourceClass::BackendStateBytes),
        selected().bytes
    );
}

#[test]
fn unselected_deferred_commit_rejects_without_mutation_and_returns_capacity() {
    // Arrange
    let (dispatcher, pool, resources) = super::fixture(8, 1);
    let prepared = pool
        .reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2))
        .unwrap();
    // Act
    let result = prepared.commit_deferred();
    // Assert
    assert!(matches!(result, Err(DisposalReservationError::Invalid(_))));
    assert_eq!(dispatcher.live_object_count(), Ok(3));
    assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(false));
    assert_eq!(pool.pending(), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
}

#[test]
fn deferred_commit_keeps_slot_and_storage_even_after_notification_and_owner_drop() {
    // Arrange
    let (dispatcher, pool, resources) = fixture();
    let mut committed = pool
        .reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2))
        .unwrap()
        .commit_deferred()
        .unwrap();
    committed.notify();
    assert!(matches!(
        pool.reserve(),
        Err(DisposalReservationError::SlotsExhausted { limit: 1 })
    ));
    drop(dispatcher);
    drop(pool);
    // Act
    let retained = resources.metrics().unwrap();
    // Assert
    assert_eq!(
        retained.class(ResourceClass::BackendStateBytes).current,
        POOL_BYTES + selected().bytes
    );
    assert_eq!(committed.removed().len(), 2);
    drop(committed);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    for class in ResourceClass::ALL {
        assert_eq!(current(&resources, class), 0);
    }
}

#[test]
fn latest_descendant_is_included_in_deferred_commit_after_waiting_admission() {
    // Arrange
    let (dispatcher, pool, _resources) = fixture();
    let reservation = pool.reserve().unwrap();
    dispatcher
        .register_object(
            ObjectId::new(4),
            Some(ObjectId::new(2)),
            ObjectKindTag::new(1),
        )
        .unwrap();
    drain(&dispatcher);
    let prepared = reservation
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2))
        .unwrap();
    // Act
    let mut committed = prepared.commit_deferred().unwrap();
    // Assert
    assert_eq!(
        committed
            .removed()
            .iter()
            .map(ObjectRecord::id)
            .collect::<Vec<_>>(),
        [ObjectId::new(3), ObjectId::new(4), ObjectId::new(2)]
    );
    assert_eq!(drain(&dispatcher).len(), 3);
    committed.notify();
}

#[test]
fn deferred_typed_failure_is_installed_before_notification() {
    // Arrange
    let (dispatcher, pool, _resources) = fixture();
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
    let mut committed = prepared
        .commit_deferred_with_failure(RuntimeCallFailure::new(
            FailureKind::Backend,
            "native cleanup failure",
        ))
        .unwrap();
    // Assert
    let Poll::Ready(Ok(result)) = poll_once(call.as_mut()) else {
        panic!("typed failure should already be visible");
    };
    let error = result.into_outcome().unwrap_err();
    assert_eq!(error.kind(), FailureKind::Backend);
    assert_eq!(error.message(), "native cleanup failure");
    committed.notify();
}

#[test]
fn declared_pending_call_bound_must_match_runtime_before_pool_installation() {
    // Arrange
    let (dispatcher, _pool, _resources) = super::fixture(8, 1);
    let selected = profile(1)
        .with_deferred_commit(NonZeroUsize::new(3).unwrap())
        .unwrap();
    let budget = resources(selected.requirements());
    // Act
    let result = selected.bind(&dispatcher.lifecycle_handle(), &budget);
    // Assert
    assert!(matches!(result, Err(DisposalReservationError::Invalid(_))));
    assert_eq!(budget.metrics().unwrap().active_leases, 0);
}

#[test]
fn profile_rejects_reselection_and_overflow_without_wrapping() {
    // Arrange
    let chosen = selected();
    // Act
    let repeated = chosen.with_deferred_commit(NonZeroUsize::new(2).unwrap());
    let overflow = profile(1).with_deferred_commit(NonZeroUsize::new(usize::MAX).unwrap());
    // Assert
    assert!(matches!(
        repeated,
        Err(DisposalReservationError::Invalid(_))
    ));
    assert!(matches!(
        overflow,
        Err(DisposalReservationError::Invalid(_))
    ));
}

#[test]
fn transport_shutdown_invalidates_deferred_commit_and_returns_every_slot() {
    // Arrange
    let (dispatcher, pool, resources) = fixture();
    let prepared = pool
        .reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2))
        .unwrap();
    dispatcher.close_transport().unwrap();
    // Act
    let result = prepared.commit_deferred();
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::LifecycleTransactionUnavailable
        ))
    ));
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
}

#[test]
fn unknown_local_id_has_empty_records_but_retains_admission_until_drop() {
    // Arrange
    let (dispatcher, pool, _resources) = fixture();
    let prepared = pool
        .reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(99))
        .unwrap();
    // Act
    let mut committed = prepared.commit_deferred().unwrap();
    // Assert
    assert!(committed.removed().is_empty());
    assert_eq!(dispatcher.live_object_count(), Ok(3));
    assert_eq!(pool.pending(), 1);
    committed.notify();
    drop(committed);
    assert_eq!(pool.pending(), 0);
}

#[test]
fn waiting_admission_covers_calls_arriving_later_up_to_runtime_limit() {
    // Arrange
    let (dispatcher, pool, _resources) = fixture();
    let reservation = pool.reserve().unwrap();
    let mut first = std::pin::pin!(dispatcher.call(Command::<TestCommand>::new(
        CallId::new(1),
        ObjectId::new(3),
        CallMetadata::default(),
        0
    )));
    let mut second = std::pin::pin!(dispatcher.call(Command::<TestCommand>::new(
        CallId::new(2),
        ObjectId::new(2),
        CallMetadata::default(),
        0
    )));
    assert!(poll_once(first.as_mut()).is_pending());
    assert!(poll_once(second.as_mut()).is_pending());
    let prepared = reservation
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2))
        .unwrap();
    // Act
    let mut committed = prepared.commit_deferred().unwrap();
    // Assert
    for outcome in [poll_once(first.as_mut()), poll_once(second.as_mut())] {
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("all admitted descendant calls should settle");
        };
        assert_eq!(
            result.into_outcome().unwrap_err().kind(),
            FailureKind::TargetClosed
        );
    }
    assert_eq!(dispatcher.pending_call_count(), Ok(0));
    committed.notify();
}

#[test]
fn deferred_typed_failure_preserves_previously_accepted_caller_cancellation() {
    // Arrange
    let (dispatcher, pool, _resources) = fixture();
    let cancellation = CancellationId::new(1);
    let mut call = std::pin::pin!(dispatcher.call(Command::<TestCommand>::new(
        CallId::new(1),
        ObjectId::new(3),
        CallMetadata::new(None, Some(cancellation)),
        0
    )));
    assert!(poll_once(call.as_mut()).is_pending());
    dispatcher
        .cancel_call(CallId::new(1), cancellation, CancellationReason::Caller)
        .unwrap();
    let prepared = pool
        .reserve()
        .unwrap()
        .prepare(&dispatcher.lifecycle_handle(), ObjectId::new(2))
        .unwrap();
    // Act
    let mut committed = prepared
        .commit_deferred_with_failure(RuntimeCallFailure::new(
            FailureKind::Backend,
            "late native failure",
        ))
        .unwrap();
    // Assert
    let Poll::Ready(Ok(result)) = poll_once(call.as_mut()) else {
        panic!("call should settle");
    };
    assert_eq!(
        result.into_outcome().unwrap_err().kind(),
        FailureKind::Cancelled
    );
    committed.notify();
}

#[test]
fn deferred_peak_admission_failure_releases_the_slot_before_native_work() {
    // Arrange
    let (dispatcher, pool, resources) = fixture();
    let ledger = resources.claim(ResourceRequirements::none()).unwrap();
    let _held = ledger
        .reserve(SCOPE, ResourceClass::BackendStateBytes, selected().bytes)
        .unwrap();
    let before = resources.metrics().unwrap();
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
        before.class(ResourceClass::BackendStateBytes).current
    );
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
    assert_eq!(slots(&dispatcher), 0);
    assert_eq!(pool.pending(), 0);
    assert_eq!(dispatcher.live_object_count(), Ok(3));
}

#[cfg(feature = "lifetime-handoff")]
#[test]
fn deferred_disposal_respects_current_transferred_call_owner() {
    // Arrange
    let (dispatcher, pool, _resources) = fixture();
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
    let mut committed = prepared.commit_deferred().unwrap();
    // Assert
    committed.notify();
    assert!(poll_once(call.as_mut()).is_pending());
    assert_eq!(dispatcher.is_object_live(ObjectId::new(1)), Ok(true));
    dispatcher.close_transport().unwrap();
    assert!(poll_once(call.as_mut()).is_ready());
}
