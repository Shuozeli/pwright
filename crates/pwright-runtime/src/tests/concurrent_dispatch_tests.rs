use super::*;
use crate::ControlledRuntimeConfig;
use std::num::NonZeroUsize;

fn configuration(active: usize) -> ControlledRuntimeConfig {
    ControlledRuntimeConfig::new(RuntimeLimits::new(1, 4).with_active_commands(active))
        .with_target_concurrency(NonZeroUsize::new(2).unwrap())
}

#[test]
fn same_target_commands_overlap_and_still_obey_connection_admission() {
    // Arrange
    let calls = Arc::new(AtomicUsize::new(0));
    let ready = Arc::new(AtomicBool::new(false));
    let config = configuration(1);
    let requirements = config.limits().resource_requirements();
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
    let dispatcher = ControlledDispatcher::with_cleanup_executor(
        ManuallyCompletedService {
            calls: calls.clone(),
            ready: ready.clone(),
        },
        RecordingCancellation {
            calls: Arc::new(Mutex::new(Vec::new())),
            operation_drops: Arc::new(AtomicUsize::new(0)),
            fail: false,
        },
        NoDeadlineScheduler,
        config,
        CleanupQueue::new(4),
    )
    .with_production_resources(&resources)
    .unwrap();
    dispatcher
        .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
        .unwrap();
    let mut first = Box::pin(dispatcher.call(command(1, 2, 1, CallMetadata::default())));
    let mut second = Box::pin(dispatcher.call(command(2, 2, 2, CallMetadata::default())));
    assert!(poll_once(first.as_mut()).is_pending());
    assert!(poll_once(second.as_mut()).is_pending());
    assert_eq!(dispatcher.scheduler_metrics().unwrap().waiting_calls, 0);
    assert_eq!(dispatcher.connection_metrics().unwrap().waiting_calls, 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::ActiveCommand)
            .current,
        1
    );
    // Act
    ready.store(true, Ordering::SeqCst);
    let first = poll_once(first.as_mut());
    let second = poll_once(second.as_mut());
    // Assert
    assert!(matches!(first, Poll::Ready(Ok(_))));
    assert!(matches!(second, Poll::Ready(Ok(_))));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(dispatcher.pending_call_count(), Ok(0));
    assert_eq!(dispatcher.scheduler_metrics().unwrap().active_targets, 0);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::PendingCall)
            .current,
        0
    );
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::ActiveCommand)
            .current,
        0
    );
    dispatcher.close_transport().unwrap();
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn concurrent_same_target_deadlines_cancel_every_started_backend_and_drain() {
    // Arrange
    let drops = Arc::new(AtomicUsize::new(0));
    let cancellations = Arc::new(Mutex::new(Vec::new()));
    let deadlines = ManualDeadlineScheduler::new();
    let state = deadlines.state.clone();
    let dispatcher = ControlledDispatcher::with_cleanup_executor(
        DropAwarePendingService {
            drops: drops.clone(),
        },
        RecordingCancellation {
            calls: cancellations.clone(),
            operation_drops: drops.clone(),
            fail: false,
        },
        deadlines,
        configuration(2),
        CleanupQueue::new(4),
    );
    dispatcher
        .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
        .unwrap();
    let metadata = CallMetadata::new(Some(Duration::from_millis(1)), None);
    let mut first = Box::pin(dispatcher.call(command(1, 2, 1, metadata)));
    let mut second = Box::pin(dispatcher.call(command(2, 2, 2, metadata)));
    assert!(poll_once(first.as_mut()).is_pending());
    assert!(poll_once(second.as_mut()).is_pending());
    assert_eq!(dispatcher.connection_metrics().unwrap().active_commands, 2);
    // Act
    state.expired.store(true, Ordering::SeqCst);
    let outcomes = [poll_once(first.as_mut()), poll_once(second.as_mut())];
    // Assert
    for outcome in outcomes {
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("expired work must finish");
        };
        assert_eq!(
            result.into_outcome().unwrap_err().kind(),
            FailureKind::Timeout
        );
    }
    assert_eq!(drops.load(Ordering::SeqCst), 2);
    assert_eq!(cancellations.lock().unwrap().len(), 2);
    assert_eq!(dispatcher.pending_call_count(), Ok(0));
    assert_eq!(dispatcher.connection_metrics().unwrap().active_commands, 0);
}

#[test]
fn concurrent_subtree_close_terminates_active_and_waiting_calls_without_starting_waiters() {
    // Arrange
    let calls = Arc::new(AtomicUsize::new(0));
    let cancellations = Arc::new(Mutex::new(Vec::new()));
    let dispatcher = ControlledDispatcher::with_cleanup_executor(
        CountingPendingService {
            calls: calls.clone(),
        },
        RecordingCancellation {
            calls: cancellations.clone(),
            operation_drops: Arc::new(AtomicUsize::new(0)),
            fail: false,
        },
        NoDeadlineScheduler,
        configuration(2),
        CleanupQueue::new(4),
    );
    dispatcher
        .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
        .unwrap();
    let mut active = Box::pin(dispatcher.call(command(1, 2, 1, CallMetadata::default())));
    let mut sibling = Box::pin(dispatcher.call(command(2, 2, 2, CallMetadata::default())));
    let mut queued = Box::pin(dispatcher.call(command(3, 2, 3, CallMetadata::default())));
    assert!(poll_once(active.as_mut()).is_pending());
    assert!(poll_once(sibling.as_mut()).is_pending());
    assert!(poll_once(queued.as_mut()).is_pending());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    // Act
    dispatcher.dispose_object(ObjectId::new(2)).unwrap();
    let outcomes = [
        poll_once(active.as_mut()),
        poll_once(sibling.as_mut()),
        poll_once(queued.as_mut()),
    ];
    // Assert
    for outcome in outcomes {
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("closed calls must finish");
        };
        assert_eq!(
            result.into_outcome().unwrap_err().kind(),
            FailureKind::TargetClosed
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(cancellations.lock().unwrap().len(), 2);
    assert_eq!(dispatcher.pending_call_count(), Ok(0));
    assert_eq!(dispatcher.scheduler_metrics().unwrap().active_targets, 0);
}

#[test]
fn dropping_concurrent_calls_preserves_each_started_cleanup_and_discards_queued_work() {
    // Arrange
    let drops = Arc::new(AtomicUsize::new(0));
    let cancellations = Arc::new(Mutex::new(Vec::new()));
    let dispatcher = ControlledDispatcher::with_cleanup_executor(
        DropAwarePendingService {
            drops: drops.clone(),
        },
        RecordingCancellation {
            calls: cancellations.clone(),
            operation_drops: drops.clone(),
            fail: false,
        },
        NoDeadlineScheduler,
        configuration(2),
        CleanupQueue::new(4),
    );
    dispatcher
        .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
        .unwrap();
    let mut first = Box::pin(dispatcher.call(command(1, 2, 1, CallMetadata::default())));
    let mut second = Box::pin(dispatcher.call(command(2, 2, 2, CallMetadata::default())));
    let mut queued = Box::pin(dispatcher.call(command(3, 2, 3, CallMetadata::default())));
    assert!(poll_once(first.as_mut()).is_pending());
    assert!(poll_once(second.as_mut()).is_pending());
    assert!(poll_once(queued.as_mut()).is_pending());
    assert_eq!(dispatcher.connection_metrics().unwrap().active_commands, 2);

    // Act
    drop((queued, first, second));

    // Assert
    assert_eq!(drops.load(Ordering::SeqCst), 2);
    assert_eq!(dispatcher.pending_call_count(), Ok(0));
    assert_eq!(dispatcher.connection_metrics().unwrap().active_commands, 0);
    assert_eq!(dispatcher.scheduler_metrics().unwrap().active_targets, 0);
    assert_eq!(dispatcher.cleanup_executor().metrics().unwrap().queued, 2);
    assert_eq!(
        dispatcher.abandoned_cleanup_metrics(),
        AbandonedCleanupMetrics {
            attempted: 2,
            submitted: 2,
            rejected: 0,
        }
    );
    assert!(cancellations.lock().unwrap().is_empty());
    while let Some(mut cleanup) = dispatcher.cleanup_executor().try_take().unwrap() {
        let outcome = poll_once(cleanup.as_mut());
        assert!(matches!(outcome, Poll::Ready(Ok(()))));
    }
    let calls = cancellations.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0.call_id(), CallId::new(1));
    assert_eq!(calls[1].0.call_id(), CallId::new(2));
    assert!(
        calls
            .iter()
            .all(|(_, reason, dropped)| { *reason == CancellationReason::Caller && *dropped == 2 })
    );
    assert_eq!(
        dispatcher.cleanup_executor().metrics().unwrap().completed,
        2
    );
}
