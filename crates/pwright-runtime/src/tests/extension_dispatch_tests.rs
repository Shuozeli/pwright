//! Installed and per-command handlers must share one admission/lifecycle owner.
use super::*;
use pwright_core::CommandResult;

#[cfg(feature = "lifetime-handoff")]
#[test]
fn handoff_keeps_original_permits_and_completes_after_frame_removal() {
    // Arrange
    let fixture = Fixture::new(1, 2);
    let mut pending = Box::pin(fixture.dispatch(extension(1, 3, None)));
    assert!(poll_once(pending.as_mut()).is_pending());
    let handoff = fixture
        .runtime
        .prepare_lifetime_handoff(CallId::new(1), ObjectId::new(3), ObjectId::new(2))
        .unwrap();
    // Act
    handoff.commit().unwrap();
    fixture.runtime.dispose_object(ObjectId::new(3)).unwrap();
    // Assert
    assert!(poll_once(pending.as_mut()).is_pending());
    assert_eq!(
        fixture
            .runtime
            .connection_metrics()
            .unwrap()
            .active_commands,
        1
    );
    assert_eq!(
        fixture.runtime.scheduler_metrics().unwrap().active_targets,
        1
    );
    assert_eq!(fixture.runtime.pending_call_count().unwrap(), 1);
    assert_eq!(
        *fixture.extension.targets.lock().unwrap(),
        [ObjectId::new(3)]
    );
    fixture.extension.ready.store(true, Ordering::SeqCst);
    let Poll::Ready(Ok(result)) = poll_once(pending.as_mut()) else {
        panic!("Page phase should finish");
    };
    assert_eq!(result.into_outcome().unwrap(), 42);
    assert!(fixture.extension.cancellations.lock().unwrap().is_empty());
    fixture.assert_empty();
}

#[cfg(feature = "lifetime-handoff")]
#[test]
fn transferred_extension_still_obeys_original_deadline() {
    // Arrange
    let fixture = Fixture::new(1, 2);
    let budget = Duration::from_secs(3);
    let mut pending = Box::pin(fixture.dispatch(extension(1, 3, Some(budget))));
    assert!(poll_once(pending.as_mut()).is_pending());
    fixture
        .runtime
        .prepare_lifetime_handoff(CallId::new(1), ObjectId::new(3), ObjectId::new(2))
        .unwrap()
        .commit()
        .unwrap();
    fixture.runtime.dispose_object(ObjectId::new(3)).unwrap();
    // Act
    fixture
        .runtime
        .deadlines
        .state
        .expired
        .store(true, Ordering::SeqCst);
    let result = poll_once(pending.as_mut());
    // Assert
    let Poll::Ready(Ok(result)) = result else {
        panic!("original deadline must fire");
    };
    assert_eq!(
        result.into_outcome().unwrap_err().kind(),
        FailureKind::Timeout
    );
    assert_eq!(*fixture.runtime.deadlines.budgets.lock().unwrap(), [budget]);
    assert_eq!(
        fixture.extension.cancellations.lock().unwrap()[0].2,
        CancellationReason::DeadlineExceeded
    );
    fixture.assert_empty();
}

#[cfg(feature = "lifetime-handoff")]
#[test]
fn transferred_extension_is_cancelled_when_page_closes() {
    // Arrange
    let fixture = Fixture::new(1, 2);
    let mut pending = Box::pin(fixture.dispatch(extension(1, 3, None)));
    assert!(poll_once(pending.as_mut()).is_pending());
    fixture
        .runtime
        .prepare_lifetime_handoff(CallId::new(1), ObjectId::new(3), ObjectId::new(2))
        .unwrap()
        .commit()
        .unwrap();
    // Act
    fixture.runtime.dispose_object(ObjectId::new(2)).unwrap();
    let result = poll_once(pending.as_mut());
    // Assert
    let Poll::Ready(Ok(result)) = result else {
        panic!("Page closure must terminate input");
    };
    assert_eq!(
        result.into_outcome().unwrap_err().kind(),
        FailureKind::TargetClosed
    );
    assert_eq!(
        fixture.extension.cancellations.lock().unwrap()[0].2,
        CancellationReason::ParentClosed
    );
    fixture.assert_empty();
}

const EXT_FRAME: ObjectKindDescriptor = FRAME_KIND.with_parent_kinds(&[PAGE_KIND_ID]);
const EXT_KINDS: ObjectKindCatalog = ObjectKindCatalog::new(&[PAGE_KIND, EXT_FRAME]);

struct ExtensionRequest(usize);
struct ExtensionCommand;
impl CommandSpec for ExtensionCommand {
    const METHOD: MethodTag = MethodTag::new(7);
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = &[FRAME_KIND.id];
    type Params = ExtensionRequest;
    type Result = usize;
    type ErrorDetails = ();
}

#[derive(Default)]
struct ExtensionHandler {
    targets: Mutex<Vec<ObjectId>>,
    ready: Arc<AtomicBool>,
    drops: Arc<AtomicUsize>,
    cancellations: Mutex<Vec<(CallId, MethodTag, CancellationReason, usize)>>,
}
struct ExtensionFuture {
    ready: Arc<AtomicBool>,
    drops: Arc<AtomicUsize>,
    value: usize,
}
impl Future for ExtensionFuture {
    type Output = Result<usize, OperationError<()>>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        if self.ready.load(Ordering::SeqCst) {
            Poll::Ready(Ok(self.value))
        } else {
            Poll::Pending
        }
    }
}
impl Drop for ExtensionFuture {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
impl TargetedService<ExtensionRequest> for ExtensionHandler {
    type Response = usize;
    type Error = OperationError<()>;
    type Future<'a> = ExtensionFuture;
    fn call_for(&self, target: ObjectId, request: ExtensionRequest) -> Self::Future<'_> {
        self.targets.lock().unwrap().push(target);
        ExtensionFuture {
            ready: self.ready.clone(),
            drops: self.drops.clone(),
            value: request.0,
        }
    }
}
impl CommandCancellation<ExtensionCommand> for ExtensionHandler {
    type Future<'a> = Ready<Result<(), OperationError<()>>>;
    fn cancel(&self, context: OperationContext, reason: CancellationReason) -> Self::Future<'_> {
        self.cancellations.lock().unwrap().push((
            context.call_id(),
            context.method(),
            reason,
            self.drops.load(Ordering::SeqCst),
        ));
        ready(Ok(()))
    }
}
fn extension(id: u64, target: u64, timeout: Option<Duration>) -> Command<ExtensionCommand> {
    Command::new(
        CallId::new(id),
        ObjectId::new(target),
        CallMetadata::new(timeout, None),
        ExtensionRequest(42),
    )
}

struct Fixture {
    base_calls: Arc<AtomicUsize>,
    base_ready: Arc<AtomicBool>,
    extension: Arc<ExtensionHandler>,
    runtime: ControlledDispatcher<
        ManuallyCompletedService,
        RecordingCancellation,
        ManualDeadlineScheduler,
        CleanupQueue,
    >,
    resources: ProductionResourceLedger,
}
impl Fixture {
    fn new(active: usize, pending: usize) -> Self {
        let base_calls = Arc::new(AtomicUsize::new(0));
        let base_ready = Arc::new(AtomicBool::new(false));
        let limits = RuntimeLimits::new(2, pending).with_active_commands(active);
        let requirements = limits.resource_requirements();
        let mut resource_limits =
            ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let bound = requirements.class(class);
            resource_limits = resource_limits.with_class(class, bound.global, bound.per_scope);
        }
        let resources = ProductionResourceLedger::new(resource_limits, requirements).unwrap();
        let runtime = ControlledDispatcher::with_cleanup_executor_and_object_kinds_and_topology(
            ManuallyCompletedService {
                calls: base_calls.clone(),
                ready: base_ready.clone(),
            },
            RecordingCancellation {
                calls: Arc::new(Mutex::new(Vec::new())),
                operation_drops: Arc::new(AtomicUsize::new(0)),
                fail: false,
            },
            ManualDeadlineScheduler::new(),
            limits,
            CleanupQueue::new(4),
            EXT_KINDS,
            ObjectTopologyMode::Partial,
        )
        .unwrap()
        .with_production_resources(&resources)
        .unwrap();
        runtime
            .register_object(ObjectId::new(2), None, PAGE_KIND.tag)
            .unwrap();
        runtime
            .register_object(ObjectId::new(3), Some(ObjectId::new(2)), FRAME_KIND.tag)
            .unwrap();
        Self {
            base_calls,
            base_ready,
            extension: Arc::new(ExtensionHandler::default()),
            runtime,
            resources,
        }
    }
    fn dispatch(
        &self,
        command: Command<ExtensionCommand>,
    ) -> impl Future<Output = Result<CommandResult<ExtensionCommand>, std::convert::Infallible>> + '_
    {
        self.runtime
            .dispatch_with(command, self.extension.as_ref(), self.extension.clone())
    }
    fn drain(&self) {
        while let Some(mut job) = self.runtime.cleanup_executor().try_take().unwrap() {
            assert!(matches!(poll_once(job.as_mut()), Poll::Ready(Ok(()))));
        }
    }
    fn assert_empty(&self) {
        assert_eq!(self.runtime.pending_call_count(), Ok(0));
        assert_eq!(
            self.runtime.connection_metrics().unwrap().active_commands,
            0
        );
        assert_eq!(self.runtime.scheduler_metrics().unwrap().active_targets, 0);
        let metrics = self.resources.metrics().unwrap();
        assert_eq!(metrics.class(ResourceClass::PendingCall).current, 0);
        assert_eq!(metrics.class(ResourceClass::ActiveCommand).current, 0);
    }
}

#[test]
fn extension_uses_the_installed_commands_connection_permit_and_resource_pool() {
    // Arrange
    let fixture = Fixture::new(1, 2);
    fixture.extension.ready.store(true, Ordering::SeqCst);
    let mut base = Box::pin(
        fixture
            .runtime
            .call(command(1, 2, 1, CallMetadata::default())),
    );
    let mut extension = Box::pin(fixture.dispatch(extension(2, 3, None)));
    assert!(poll_once(base.as_mut()).is_pending());
    assert!(poll_once(extension.as_mut()).is_pending());
    assert!(fixture.extension.targets.lock().unwrap().is_empty());
    assert_eq!(fixture.runtime.pending_call_count(), Ok(2));
    assert_eq!(
        fixture.runtime.connection_metrics().unwrap().waiting_calls,
        1
    );
    assert_eq!(
        fixture
            .resources
            .metrics()
            .unwrap()
            .class(ResourceClass::ActiveCommand)
            .current,
        1
    );
    // Act
    fixture.base_ready.store(true, Ordering::SeqCst);
    let base = poll_once(base.as_mut());
    let outcome = poll_once(extension.as_mut());
    // Assert
    assert!(matches!(base, Poll::Ready(Ok(_))));
    let Poll::Ready(Ok(result)) = outcome else {
        panic!("shared permit must be released")
    };
    assert_eq!(result.into_outcome().unwrap(), 42);
    assert_eq!(
        *fixture.extension.targets.lock().unwrap(),
        [ObjectId::new(3)]
    );
    assert_eq!(fixture.base_calls.load(Ordering::SeqCst), 1);
    fixture.assert_empty();
}

#[test]
fn extension_waits_for_the_same_target_even_with_connection_capacity() {
    // Arrange
    let fixture = Fixture::new(2, 2);
    let mut base = Box::pin(
        fixture
            .runtime
            .call(command(1, 3, 1, CallMetadata::default())),
    );
    let mut extension = Box::pin(fixture.dispatch(extension(2, 3, None)));
    assert!(poll_once(base.as_mut()).is_pending());
    // Act
    let outcome = poll_once(extension.as_mut());
    // Assert
    assert!(outcome.is_pending());
    assert!(fixture.extension.targets.lock().unwrap().is_empty());
    assert_eq!(
        fixture.runtime.scheduler_metrics().unwrap().waiting_calls,
        1
    );
    assert_eq!(
        fixture.runtime.connection_metrics().unwrap().waiting_calls,
        0
    );
    drop((extension, base));
    fixture.drain();
    fixture.assert_empty();
}

#[test]
fn extension_cannot_bypass_the_shared_pending_call_limit() {
    // Arrange
    let fixture = Fixture::new(1, 1);
    let mut base = Box::pin(
        fixture
            .runtime
            .call(command(1, 2, 1, CallMetadata::default())),
    );
    assert!(poll_once(base.as_mut()).is_pending());
    let mut extension = Box::pin(fixture.dispatch(extension(2, 3, None)));
    // Act
    let outcome = poll_once(extension.as_mut());
    // Assert
    let Poll::Ready(Ok(result)) = outcome else {
        panic!("admission must reject")
    };
    assert_eq!(
        result.into_outcome().unwrap_err().kind(),
        FailureKind::ResourceExhausted
    );
    assert!(fixture.extension.targets.lock().unwrap().is_empty());
    assert_eq!(fixture.runtime.pending_call_count(), Ok(1));
    drop(base);
    fixture.drain();
    fixture.assert_empty();
}

#[test]
fn extension_rejects_wrong_kind_and_dead_targets_before_handler() {
    // Arrange
    let fixture = Fixture::new(2, 2);
    fixture.runtime.dispose_object(ObjectId::new(3)).unwrap();
    let cases = [
        (2, FailureKind::ProtocolViolation),
        (3, FailureKind::TargetClosed),
    ];
    for (target, expected) in cases {
        let mut extension =
            Box::pin(fixture.dispatch(extension(target, target, Some(Duration::from_secs(1)))));
        // Act
        let outcome = poll_once(extension.as_mut());
        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("target must reject")
        };
        assert_eq!(result.into_outcome().unwrap_err().kind(), expected);
    }
    // Kind validation precedes timer creation; liveness is validated by the
    // shared store after timer creation, exactly as for installed commands.
    assert_eq!(
        *fixture.runtime.deadlines.budgets.lock().unwrap(),
        [Duration::from_secs(1)]
    );
    assert!(fixture.extension.targets.lock().unwrap().is_empty());
    assert!(fixture.extension.cancellations.lock().unwrap().is_empty());
    fixture.assert_empty();
}

#[test]
fn active_extension_deadline_uses_its_own_cancel_after_dropping_the_operation() {
    // Arrange
    let fixture = Fixture::new(1, 2);
    let budget = Duration::from_millis(15);
    let mut extension = Box::pin(fixture.dispatch(extension(1, 3, Some(budget))));
    assert!(poll_once(extension.as_mut()).is_pending());
    // Act
    fixture
        .runtime
        .deadlines
        .state
        .expired
        .store(true, Ordering::SeqCst);
    let outcome = poll_once(extension.as_mut());
    // Assert
    let Poll::Ready(Ok(result)) = outcome else {
        panic!("deadline must finish")
    };
    assert_eq!(
        result.into_outcome().unwrap_err().kind(),
        FailureKind::Timeout
    );
    assert_eq!(*fixture.runtime.deadlines.budgets.lock().unwrap(), [budget]);
    assert_eq!(
        *fixture.extension.cancellations.lock().unwrap(),
        [(
            CallId::new(1),
            ExtensionCommand::METHOD,
            CancellationReason::DeadlineExceeded,
            1
        )]
    );
    assert!(
        fixture
            .runtime
            .cancellation
            .calls
            .lock()
            .unwrap()
            .is_empty()
    );
    fixture.assert_empty();
}

#[test]
fn queued_extension_deadline_never_starts_or_cancels_an_unstarted_handler() {
    // Arrange
    let fixture = Fixture::new(1, 2);
    let mut base = Box::pin(
        fixture
            .runtime
            .call(command(1, 2, 1, CallMetadata::default())),
    );
    let budget = Duration::from_millis(15);
    let mut extension = Box::pin(fixture.dispatch(extension(2, 3, Some(budget))));
    assert!(poll_once(base.as_mut()).is_pending());
    assert!(poll_once(extension.as_mut()).is_pending());
    // Act
    fixture
        .runtime
        .deadlines
        .state
        .expired
        .store(true, Ordering::SeqCst);
    let outcome = poll_once(extension.as_mut());
    // Assert
    let Poll::Ready(Ok(result)) = outcome else {
        panic!("queued deadline must finish")
    };
    assert_eq!(
        result.into_outcome().unwrap_err().kind(),
        FailureKind::Timeout
    );
    assert_eq!(*fixture.runtime.deadlines.budgets.lock().unwrap(), [budget]);
    assert!(fixture.extension.targets.lock().unwrap().is_empty());
    assert!(fixture.extension.cancellations.lock().unwrap().is_empty());
    drop(base);
    fixture.drain();
    fixture.assert_empty();
}

#[test]
fn parent_disposal_cancels_active_extension_and_discards_its_queued_sibling() {
    // Arrange
    let fixture = Fixture::new(2, 3);
    let mut base = Box::pin(
        fixture
            .runtime
            .call(command(1, 2, 1, CallMetadata::default())),
    );
    let mut active = Box::pin(fixture.dispatch(extension(2, 3, None)));
    let mut queued = Box::pin(fixture.dispatch(extension(3, 3, None)));
    assert!(poll_once(base.as_mut()).is_pending());
    assert!(poll_once(active.as_mut()).is_pending());
    assert!(poll_once(queued.as_mut()).is_pending());
    // Act
    fixture.runtime.dispose_object(ObjectId::new(2)).unwrap();
    let outcome = poll_once(base.as_mut());
    let extended = [poll_once(active.as_mut()), poll_once(queued.as_mut())];
    // Assert
    let Poll::Ready(Ok(result)) = outcome else {
        panic!("base must close")
    };
    assert_eq!(
        result.into_outcome().unwrap_err().kind(),
        FailureKind::TargetClosed
    );
    for outcome in extended {
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("extension must close")
        };
        assert_eq!(
            result.into_outcome().unwrap_err().kind(),
            FailureKind::TargetClosed
        );
    }
    assert_eq!(fixture.extension.targets.lock().unwrap().len(), 1);
    assert_eq!(fixture.extension.cancellations.lock().unwrap().len(), 1);
    assert_eq!(fixture.runtime.cancellation.calls.lock().unwrap().len(), 1);
    fixture.assert_empty();
    assert_eq!(fixture.resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn abandoned_extension_uses_shared_cleanup_queue_and_keeps_its_matching_handler_alive() {
    // Arrange
    let fixture = Fixture::new(2, 3);
    let mut base = Box::pin(
        fixture
            .runtime
            .call(command(1, 2, 1, CallMetadata::default())),
    );
    let mut active = Box::pin(fixture.dispatch(extension(2, 3, None)));
    let mut queued = Box::pin(fixture.dispatch(extension(3, 3, None)));
    assert!(poll_once(base.as_mut()).is_pending());
    assert!(poll_once(active.as_mut()).is_pending());
    assert!(poll_once(queued.as_mut()).is_pending());
    // Act
    drop((queued, active, base));
    // Assert
    assert_eq!(fixture.extension.drops.load(Ordering::SeqCst), 1);
    assert_eq!(
        fixture.runtime.cleanup_executor().metrics().unwrap().queued,
        2
    );
    assert!(fixture.extension.cancellations.lock().unwrap().is_empty());
    assert!(Arc::strong_count(&fixture.extension) > 1);
    fixture.drain();
    assert_eq!(
        *fixture.extension.cancellations.lock().unwrap(),
        [(
            CallId::new(2),
            ExtensionCommand::METHOD,
            CancellationReason::Caller,
            1
        )]
    );
    assert_eq!(fixture.runtime.cancellation.calls.lock().unwrap().len(), 1);
    assert_eq!(Arc::strong_count(&fixture.extension), 1);
    fixture.assert_empty();
}

#[test]
fn transport_close_retires_active_and_queued_extensions_without_native_cancellation() {
    // Arrange
    let fixture = Fixture::new(1, 2);
    let mut active = Box::pin(fixture.dispatch(extension(1, 3, None)));
    let mut queued = Box::pin(fixture.dispatch(extension(2, 3, None)));
    assert!(poll_once(active.as_mut()).is_pending());
    assert!(poll_once(queued.as_mut()).is_pending());
    // Act
    fixture.runtime.close_transport().unwrap();
    let outcomes = [poll_once(active.as_mut()), poll_once(queued.as_mut())];
    // Assert
    for outcome in outcomes {
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("transport close must finish")
        };
        assert_eq!(
            result.into_outcome().unwrap_err().kind(),
            FailureKind::TransportClosed
        );
    }
    assert_eq!(fixture.extension.targets.lock().unwrap().len(), 1);
    assert_eq!(fixture.extension.drops.load(Ordering::SeqCst), 1);
    assert!(fixture.extension.cancellations.lock().unwrap().is_empty());
    assert_eq!(
        fixture.runtime.cleanup_executor().metrics().unwrap().queued,
        0
    );
    fixture.assert_empty();
    assert_eq!(fixture.resources.metrics().unwrap().active_leases, 0);
}
