use super::*;
use std::sync::{TryLockError, mpsc};
use std::time::Duration;

fn prepare(runtime: &Dispatcher, pool: &DisposalReservations) -> ReservedPreparedDisposal {
    pool.reserve()
        .unwrap()
        .prepare(&runtime.lifecycle_handle(), ObjectId::new(2))
        .unwrap()
}

#[test]
fn guarded_publication_prevents_transport_retirement_between_local_commits() {
    // Arrange
    let (runtime, pool, resources) = fixture();
    let prepared = prepare(&runtime, &pool);
    let (start, begin) = mpsc::sync_channel(1);
    let (blocked, observed) = mpsc::sync_channel(1);
    let store = runtime.store.clone();
    let terminal = std::thread::spawn(move || {
        begin.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(
            store.state.try_lock(),
            Err(TryLockError::WouldBlock)
        ));
        blocked.send(()).unwrap();
        store.close_transport_and_cancel_calls().unwrap()
    });
    // Act
    let (published, mut committed) = prepared
        .commit_deferred_with_publication(None, || {
            start.send(()).unwrap();
            observed.recv_timeout(Duration::from_secs(5)).unwrap();
            Ok::<_, Infallible>("published")
        })
        .unwrap()
        .unwrap();
    // Assert
    let outcome = terminal.join().unwrap();
    assert_eq!(published, "published");
    assert_eq!(
        committed
            .removed()
            .iter()
            .map(ObjectRecord::id)
            .collect::<Vec<_>>(),
        [ObjectId::new(3), ObjectId::new(2)]
    );
    assert_eq!(
        outcome
            .invalidated_objects
            .iter()
            .map(ObjectRecord::id)
            .collect::<Vec<_>>(),
        [ObjectId::new(1)]
    );
    committed.notify();
    drop((committed, pool, runtime));
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn publication_failure_releases_runtime_gate_without_disposing_or_publishing_control() {
    // Arrange
    let (runtime, pool, resources) = fixture();
    let before_bytes = current(&resources, ResourceClass::BackendStateBytes);
    let prepared = prepare(&runtime, &pool);
    let failure = Arc::new("publication preflight rejected");
    let expected = failure.clone();
    // Act
    let result = prepared
        .commit_deferred_with_publication(None, || Err::<(), _>(failure))
        .unwrap();
    // Assert
    let Err(error) = result else {
        panic!("publication must fail");
    };
    assert!(Arc::ptr_eq(&error, &expected));
    assert_eq!(runtime.live_object_count(), Ok(3));
    assert_eq!(runtime.lifecycle_transaction_active(), Ok(false));
    assert_eq!(pool.pending(), 0);
    assert_eq!(slots(&runtime), 0);
    assert!(drain(&runtime).is_empty());
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes),
        before_bytes
    );
}

#[test]
fn stopped_runtime_never_invokes_local_publication() {
    // Arrange
    let (runtime, pool, resources) = fixture();
    let prepared = prepare(&runtime, &pool);
    runtime.close_transport().unwrap();
    // Act
    let result = prepared.commit_deferred_with_publication(None, || -> Result<(), Infallible> {
        panic!("stale runtime cannot publish");
    });
    // Assert
    assert!(result.is_err());
    assert_eq!(pool.pending(), 0);
    assert_eq!(slots(&runtime), 0);
    assert_eq!(current(&resources, ResourceClass::EventItem), 0);
}

#[test]
fn unselected_deferred_capacity_rejects_before_callback() {
    // Arrange
    let (runtime, pool, _) = super::super::fixture(8, 1);
    let prepared = prepare(&runtime, &pool);
    // Act
    let result = prepared.commit_deferred_with_publication(None, || -> Result<(), Infallible> {
        panic!("deferred capacity not selected");
    });
    // Assert
    assert!(matches!(result, Err(DisposalReservationError::Invalid(_))));
    assert_eq!(runtime.live_object_count(), Ok(3));
    assert_eq!(runtime.lifecycle_transaction_active(), Ok(false));
    assert_eq!(pool.pending(), 0);
}

#[test]
fn cancellation_waiter_is_not_woken_during_publication_or_outer_lock() {
    // Arrange
    let (runtime, pool, _) = fixture();
    let outer = Arc::new(Mutex::new(()));
    let wake = Arc::new(ReentrantWake {
        outer: outer.clone(),
        handle: runtime.lifecycle_handle(),
        count: AtomicUsize::new(0),
    });
    let waker = Waker::from(wake.clone());
    let mut call = std::pin::pin!(runtime.call(Command::<TestCommand>::new(
        CallId::new(1),
        ObjectId::new(3),
        CallMetadata::default(),
        0
    )));
    assert!(
        call.as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    let prepared = prepare(&runtime, &pool);
    let guard = outer.lock().unwrap();
    // Act
    let ((), mut committed) = prepared
        .commit_deferred_with_publication(None, || {
            assert_eq!(wake.count.load(Ordering::SeqCst), 0);
            assert!(runtime.store.state.try_lock().is_err());
            Ok::<(), Infallible>(())
        })
        .unwrap()
        .unwrap();
    // Assert
    assert_eq!(wake.count.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.is_object_live(ObjectId::new(2)), Ok(false));
    drop(guard);
    committed.notify();
    assert_eq!(wake.count.load(Ordering::SeqCst), 1);
    assert!(poll_once(call.as_mut()).is_ready());
}

#[test]
fn typed_failure_and_late_admitted_call_survive_guarded_publication() {
    // Arrange
    let (runtime, pool, _) = fixture();
    let prepared = prepare(&runtime, &pool);
    let mut call = std::pin::pin!(runtime.call(Command::<TestCommand>::new(
        CallId::new(1),
        ObjectId::new(3),
        CallMetadata::default(),
        0
    )));
    assert!(poll_once(call.as_mut()).is_pending());
    // Act
    let ((), mut committed) = prepared
        .commit_deferred_with_publication(
            Some(RuntimeCallFailure::new(
                FailureKind::Backend,
                "native failure",
            )),
            || Ok::<(), Infallible>(()),
        )
        .unwrap()
        .unwrap();
    // Assert
    let Poll::Ready(Ok(result)) = poll_once(call.as_mut()) else {
        panic!("late call must settle");
    };
    let error = result.into_outcome().unwrap_err();
    assert_eq!(error.kind(), FailureKind::Backend);
    assert_eq!(error.message(), "native failure");
    committed.notify();
}

#[test]
fn missing_cancellation_signal_is_detected_before_external_publication() {
    // Arrange
    let (runtime, pool, _) = fixture();
    let mut call = std::pin::pin!(runtime.call(Command::<TestCommand>::new(
        CallId::new(1),
        ObjectId::new(3),
        CallMetadata::default(),
        0
    )));
    assert!(poll_once(call.as_mut()).is_pending());
    let prepared = prepare(&runtime, &pool);
    let signal = runtime
        .store
        .state
        .lock()
        .unwrap()
        .cancellation_signals
        .remove(&CallId::new(1))
        .unwrap();
    // Act
    let result = prepared.commit_deferred_with_publication(None, || -> Result<(), Infallible> {
        panic!("invalid call inventory must not publish");
    });
    // Assert
    assert!(matches!(
        result,
        Err(DisposalReservationError::Runtime(
            RuntimeStateError::MissingCancellationSignal { .. }
        ))
    ));
    assert_eq!(runtime.live_object_count(), Ok(3));
    runtime
        .store
        .state
        .lock()
        .unwrap()
        .cancellation_signals
        .insert(CallId::new(1), signal);
}

#[test]
fn successful_publication_result_retains_original_disposal_admission_until_drop() {
    // Arrange
    let (runtime, pool, resources) = fixture();
    let prepared = prepare(&runtime, &pool);
    let before_bytes = current(&resources, ResourceClass::BackendStateBytes);
    let output = Arc::new("independently owned publication result");
    let expected = output.clone();
    // Act
    let (output, mut committed) = prepared
        .commit_deferred_with_publication(None, || Ok::<_, Infallible>(output))
        .unwrap()
        .unwrap();
    // Assert
    assert!(Arc::ptr_eq(&output, &expected));
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes),
        before_bytes
    );
    assert_eq!(pool.pending(), 1);
    committed.notify();
    assert_eq!(pool.pending(), 1);
    drop(committed);
    assert_eq!(pool.pending(), 0);
}

#[test]
fn publication_error_does_not_undo_external_mutations_made_by_the_callback() {
    // Arrange
    let (runtime, pool, _) = fixture();
    let prepared = prepare(&runtime, &pool);
    let mut externally_published = false;
    // Act
    let result = prepared
        .commit_deferred_with_publication(None, || {
            externally_published = true;
            Err::<(), _>("caller violated preflight-before-mutation contract")
        })
        .unwrap();
    // Assert
    assert!(result.is_err());
    assert!(externally_published);
    assert_eq!(runtime.live_object_count(), Ok(3));
    assert_eq!(pool.pending(), 0);
}

#[test]
fn publication_panic_poisoning_is_not_reported_as_a_successful_disposal() {
    // Arrange
    let (runtime, pool, resources) = fixture();
    let prepared = prepare(&runtime, &pool);
    // Act
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = prepared.commit_deferred_with_publication(None, || -> Result<(), Infallible> {
            panic!("publication panic");
        });
    }));
    // Assert
    assert!(panic.is_err());
    assert_eq!(
        runtime.live_object_count(),
        Err(RuntimeStateError::StatePoisoned)
    );
    assert_eq!(pool.pending(), 0);
    drop((runtime, pool));
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
