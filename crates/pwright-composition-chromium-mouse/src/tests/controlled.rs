use super::*;
#[tokio::test]
async fn all_six_runtime_commands_use_distinct_typed_page_routes() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let (runtime, _) = runtime();
    let x = MouseNumber::try_from(20.0).unwrap();
    let y = MouseNumber::try_from(30.0).unwrap();
    macro_rules! exercise {
        ($command:ty, $id:expr, $params:expr) => {{
            let command =
                Command::<$command>::new(CallId::new($id), ObjectId::new(7), metadata(), $params);
            let invocation =
                Arc::new(ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap());
            // Act
            let result = runtime
                .dispatch_with(command, invocation.as_ref(), invocation.clone())
                .await
                .unwrap();
            // Assert
            result.into_outcome().unwrap();
            idle(&runtime);
            let receipt = installed.retirement().next().await;
            assert_eq!(receipt.context(), invocation.context());
            receipt.into_outcome().unwrap();
            drop(invocation);
            assert_eq!(installed.page().retained_records(), 0);
        }};
    }
    exercise!(
        MouseMove,
        1,
        MoveParams {
            x,
            y,
            steps: MouseMoveSteps::ONE
        }
    );
    exercise!(
        MouseDown,
        2,
        DownParams {
            button: MouseButton::Left,
            click_count: 1
        }
    );
    exercise!(
        MouseUp,
        3,
        UpParams {
            button: MouseButton::Left,
            click_count: 1
        }
    );
    exercise!(
        MouseClick,
        4,
        ClickParams {
            x,
            y,
            button: MouseButton::Left,
            click_count: MouseClickCount::ONE,
            delay: None
        }
    );
    exercise!(
        MouseDblclick,
        5,
        DblclickParams {
            x,
            y,
            button: MouseButton::Right,
            delay: None
        }
    );
    exercise!(
        MouseWheel,
        6,
        WheelParams {
            delta_x: x,
            delta_y: y
        }
    );
    assert_eq!(client.calls().len(), 12);
}

#[tokio::test]
async fn deadline_during_click_delay_does_not_synthesize_button_release() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let (runtime, expired) = runtime();
    let (id, page, _, mut params) = click(1).into_parts();
    params.delay = Some(Duration::from_secs(1));
    let command = Command::<MouseClick>::new(
        id,
        page,
        CallMetadata::new(Some(Duration::from_secs(1)), None),
        params,
    );
    let invocation =
        Arc::new(ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap());
    let mut operation = runtime.dispatch_with(command, invocation.as_ref(), invocation.clone());
    poll_pending(&mut operation);
    // Act
    expired.store(true, Ordering::Release);
    poll_pending(&mut operation);
    installed.retirement().next().await.into_outcome().unwrap();
    // Assert
    assert_eq!(kind(poll(&mut operation)), FailureKind::Timeout);
    let calls = client.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1[0]["type"], "mouseMoved");
    assert_eq!(calls[1].1[0]["type"], "mousePressed");
    idle(&runtime);
}

#[tokio::test]
async fn consuming_receipt_wakes_registered_cleanup_waiter() {
    // Arrange
    let (mut installed, _, _) = setup(1);
    let command = down(1);
    let invocation = ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap();
    let (_, page, _, params) = command.into_parts();
    drop(invocation.call_for(page, params));
    let mut waiting = invocation.cancel(invocation.context(), CancellationReason::Caller);
    let counter = Arc::new(Counter::default());
    let waker = Waker::from(counter.clone());
    assert!(
        waiting
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    let receipt = installed.retirement().next().await;
    let before = counter.0.load(Ordering::Relaxed);
    // Act
    receipt.into_outcome().unwrap();
    // Assert
    assert_eq!(counter.0.load(Ordering::Relaxed), before + 1);
    assert!(matches!(poll(&mut waiting), Poll::Ready(Ok(()))));
}

#[tokio::test]
async fn competing_waiter_is_rejected_without_replacing_the_original() {
    // Arrange
    let (mut installed, _, _) = setup(1);
    let command = down(1);
    let invocation = ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap();
    let (_, page, _, params) = command.into_parts();
    drop(invocation.call_for(page, params));
    let mut first = invocation.cancel(invocation.context(), CancellationReason::Caller);
    poll_pending(&mut first);
    // Act
    let error = invocation
        .cancel(invocation.context(), CancellationReason::Caller)
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    installed.retirement().next().await.into_outcome().unwrap();
    assert!(matches!(poll(&mut first), Poll::Ready(Ok(()))));
}
use pwright_browser_model::{
    BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, FRAME_OBJECT_KIND, PAGE_OBJECT_KIND,
};
use pwright_core::{CommandCancellation, ObjectKindCatalog};
use pwright_runtime::{
    CleanupQueue, ControlledDispatcher, DeadlineScheduler, RuntimeLimits, TargetedService,
};
use std::sync::atomic::AtomicBool;

struct ManualDeadline(Arc<AtomicBool>);
impl DeadlineScheduler for ManualDeadline {
    type Sleep<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
    fn sleep(&self, _: Duration) -> Option<Self::Sleep<'_>> {
        Some(Box::pin(std::future::poll_fn(|_| {
            if self.0.load(Ordering::Acquire) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })))
    }
}
type Runtime = ControlledDispatcher<(), (), ManualDeadline>;
fn runtime() -> (Runtime, Arc<AtomicBool>) {
    let expired = Arc::new(AtomicBool::new(false));
    let limits = RuntimeLimits::new(3, 4).with_active_commands(1);
    let ledger = resources(limits.resource_requirements());
    let runtime = ControlledDispatcher::with_cleanup_executor_and_object_kinds(
        (),
        (),
        ManualDeadline(expired.clone()),
        limits,
        CleanupQueue::new(4),
        ObjectKindCatalog::new(&[
            BROWSER_OBJECT_KIND,
            BROWSER_CONTEXT_OBJECT_KIND,
            PAGE_OBJECT_KIND,
            FRAME_OBJECT_KIND,
        ]),
    )
    .unwrap()
    .with_production_resources(&ledger)
    .unwrap();
    runtime
        .register_object(ObjectId::new(7), None, PAGE_OBJECT_KIND.tag)
        .unwrap();
    runtime
        .register_object(
            ObjectId::new(8),
            Some(ObjectId::new(7)),
            FRAME_OBJECT_KIND.tag,
        )
        .unwrap();
    (runtime, expired)
}
fn poll<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    Pin::new(future).poll(&mut Context::from_waker(Waker::noop()))
}
fn idle(runtime: &Runtime) {
    assert_eq!(runtime.pending_call_count(), Ok(0));
    assert_eq!(runtime.connection_metrics().unwrap().active_commands, 0);
    assert_eq!(runtime.scheduler_metrics().unwrap().active_targets, 0);
}
fn deadline_down(id: u64) -> Command<MouseDown> {
    let (id, page, _, params) = down(id).into_parts();
    Command::new(
        id,
        page,
        CallMetadata::new(Some(Duration::from_secs(1)), metadata().cancellation),
        params,
    )
}
fn kind<C: CommandSpec>(reply: Poll<Result<CommandResult<C>, Infallible>>) -> FailureKind {
    let Poll::Ready(Ok(result)) = reply else {
        panic!("expected completed result")
    };
    match result.into_outcome() {
        Err(error) => error.kind(),
        Ok(_) => panic!("expected error"),
    }
}

#[tokio::test]
async fn deadline_waits_for_consumed_receipt_not_request_or_native_ack_alone() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let (runtime, expired) = runtime();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let command = deadline_down(1);
    let invocation =
        Arc::new(ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap());
    let mut operation = runtime.dispatch_with(command, invocation.as_ref(), invocation.clone());
    poll_pending(&mut operation);
    // Act
    expired.store(true, Ordering::Release);
    poll_pending(&mut operation);
    gate.notify_one();
    let receipt = installed.retirement().next().await;
    // Assert: even a produced receipt is not yet acknowledged to the runtime.
    assert_eq!(runtime.pending_call_count(), Ok(1));
    poll_pending(&mut operation);
    assert_eq!(receipt.context(), invocation.context());
    receipt.into_outcome().unwrap();
    assert_eq!(kind(poll(&mut operation)), FailureKind::Timeout);
    idle(&runtime);
    assert_eq!(client.calls().len(), 1);
    drop(operation);
    assert_eq!(installed.page().retained_records(), 1);
    drop(invocation);
    assert_eq!(installed.page().retained_records(), 0);
}

#[tokio::test]
async fn caller_cancellation_preserves_exact_identity_until_native_retirement() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let (runtime, _) = runtime();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let command = down(1);
    let invocation =
        Arc::new(ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap());
    let mut operation = runtime.dispatch_with(command, invocation.as_ref(), invocation.clone());
    poll_pending(&mut operation);
    // Act
    runtime
        .cancel_call(
            CallId::new(1),
            CancellationId::new(9),
            CancellationReason::Caller,
        )
        .unwrap();
    poll_pending(&mut operation);
    gate.notify_one();
    installed.retirement().next().await.into_outcome().unwrap();
    // Assert
    assert_eq!(kind(poll(&mut operation)), FailureKind::Cancelled);
    idle(&runtime);
    assert_eq!(client.calls().len(), 1);
}

#[tokio::test]
async fn page_disposal_keeps_cleanup_addressable_on_the_original_revoked_route() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let (runtime, _) = runtime();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let command = down(1);
    let invocation =
        Arc::new(ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap());
    let mut operation = runtime.dispatch_with(command, invocation.as_ref(), invocation.clone());
    poll_pending(&mut operation);
    installed.page().revoke_admission();
    // Act
    runtime.dispose_object(ObjectId::new(7)).unwrap();
    poll_pending(&mut operation);
    gate.notify_one();
    installed.retirement().next().await.into_outcome().unwrap();
    // Assert
    assert_eq!(kind(poll(&mut operation)), FailureKind::TargetClosed);
    idle(&runtime);
    assert_eq!(client.calls().len(), 1);
}

#[tokio::test]
async fn cleanup_error_reaches_both_receipt_consumer_and_runtime() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let (runtime, expired) = runtime();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_input_errors([Some("native retirement failed".into())]);
    client.set_pointer_gates([Some(gate.clone())]);
    let command = deadline_down(1);
    let invocation =
        Arc::new(ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap());
    let mut operation = runtime.dispatch_with(command, invocation.as_ref(), invocation.clone());
    poll_pending(&mut operation);
    // Act
    expired.store(true, Ordering::Release);
    poll_pending(&mut operation);
    gate.notify_one();
    let error = installed
        .retirement()
        .next()
        .await
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    let Poll::Ready(Ok(result)) = poll(&mut operation) else {
        panic!("cleanup reply")
    };
    assert_eq!(result.into_outcome().unwrap_err(), error);
    idle(&runtime);
}

#[tokio::test]
async fn early_click_error_returns_while_driver_still_owns_pending_tail() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let (runtime, _) = runtime();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_input_errors([Some("move failed".into()), None, None]);
    client.set_pointer_gates([None, Some(gate.clone()), None]);
    let command = click(1);
    let invocation =
        Arc::new(ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap());
    // Act
    let result = runtime
        .dispatch_with(command, invocation.as_ref(), invocation.clone())
        .await
        .unwrap();
    // Assert
    assert_eq!(
        result.into_outcome().unwrap_err().kind(),
        FailureKind::Backend
    );
    idle(&runtime);
    drop(invocation);
    assert_eq!(installed.page().queued_operations(), 1);
    assert_eq!(client.calls().len(), 3);
    gate.notify_one();
    installed.retirement().next().await.into_outcome().unwrap();
    assert_eq!(installed.page().retained_records(), 0);
}

#[tokio::test]
async fn abandoned_cleanup_job_keeps_invocation_until_receipt_delivery() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let (runtime, _) = runtime();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let command = down(1);
    let invocation =
        Arc::new(ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap());
    let weak = Arc::downgrade(&invocation);
    let mut operation = runtime.dispatch_with(command, invocation.as_ref(), invocation.clone());
    poll_pending(&mut operation);
    // Act
    drop(operation);
    drop(invocation);
    let mut job = runtime.cleanup_executor().try_take().unwrap().unwrap();
    poll_pending(&mut job);
    // Assert
    assert!(weak.upgrade().is_some());
    gate.notify_one();
    installed.retirement().next().await.into_outcome().unwrap();
    assert!(matches!(poll(&mut job), Poll::Ready(Ok(()))));
    drop(job);
    assert!(weak.upgrade().is_none());
    assert_eq!(installed.page().retained_records(), 0);
    idle(&runtime);
}

#[tokio::test]
async fn transport_close_returns_promptly_but_is_not_native_retirement_proof() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let (runtime, _) = runtime();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let command = down(1);
    let invocation =
        Arc::new(ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap());
    let mut operation = runtime.dispatch_with(command, invocation.as_ref(), invocation.clone());
    poll_pending(&mut operation);
    // Act
    runtime.close_transport().unwrap();
    // Assert: runtime skips cancellation callbacks on transport loss.
    assert_eq!(kind(poll(&mut operation)), FailureKind::TransportClosed);
    assert_eq!(runtime.cleanup_executor().metrics().unwrap().queued, 0);
    assert_eq!(installed.page().queued_operations(), 1);
    drop(operation);
    drop(invocation);
    gate.notify_one();
    installed.retirement().next().await.into_outcome().unwrap();
    assert_eq!(installed.page().retained_records(), 0);
    idle(&runtime);
}

#[tokio::test]
async fn queued_deadline_expires_without_route_or_native_admission() {
    // Arrange
    let (mut installed, client, _) = setup(2);
    let (runtime, expired) = runtime();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let command = down(1);
    let first = Arc::new(ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap());
    let mut active = runtime.dispatch_with(command, first.as_ref(), first.clone());
    poll_pending(&mut active);
    let command = deadline_down(2);
    let second =
        Arc::new(ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap());
    let mut queued = runtime.dispatch_with(command, second.as_ref(), second.clone());
    poll_pending(&mut queued);
    // Act
    expired.store(true, Ordering::Release);
    // Assert
    assert_eq!(kind(poll(&mut queued)), FailureKind::Timeout);
    assert_eq!(installed.page().retained_records(), 1);
    assert_eq!(client.calls().len(), 1);
    drop(active);
    let mut job = runtime.cleanup_executor().try_take().unwrap().unwrap();
    poll_pending(&mut job);
    gate.notify_one();
    installed.retirement().next().await.into_outcome().unwrap();
    assert!(matches!(poll(&mut job), Poll::Ready(Ok(()))));
    idle(&runtime);
}

#[tokio::test]
async fn cancelled_wait_can_resume_without_consuming_a_different_call_receipt() {
    // Arrange
    let (mut installed, _, _) = setup(1);
    let command = down(1);
    let invocation = ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap();
    let (_, page, _, params) = command.into_parts();
    let response = invocation.call_for(page, params);
    drop(response);
    let mut first = invocation.cancel(invocation.context(), CancellationReason::Caller);
    poll_pending(&mut first);
    // Act
    drop(first);
    let mut resumed = invocation.cancel(invocation.context(), CancellationReason::Caller);
    poll_pending(&mut resumed);
    installed.retirement().next().await.into_outcome().unwrap();
    // Assert
    assert!(matches!(poll(&mut resumed), Poll::Ready(Ok(()))));
    let repeated = invocation
        .cancel(invocation.context(), CancellationReason::Caller)
        .await
        .unwrap_err();
    assert_eq!(repeated.kind(), FailureKind::ProtocolViolation);
}

#[tokio::test]
async fn forged_cancellation_identity_has_no_effect_on_fresh_invocation() {
    // Arrange
    let (installed, client, _) = setup(1);
    let command = down(1);
    let invocation = ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap();
    let contexts = [
        context(2, MouseDown::METHOD),
        context(1, MouseUp::METHOD),
        OperationContext::new(
            CallId::new(1),
            ObjectId::new(8),
            MouseDown::METHOD,
            metadata().cancellation,
        ),
        OperationContext::new(CallId::new(1), ObjectId::new(7), MouseDown::METHOD, None),
    ];
    // Act / Assert
    for context in contexts {
        let error = invocation
            .cancel(context, CancellationReason::Caller)
            .await
            .unwrap_err();
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    }
    assert!(client.calls().is_empty());
    assert_eq!(installed.page().retained_records(), 0);
}

#[tokio::test]
async fn cancellation_before_start_prevents_later_native_admission() {
    // Arrange
    let (installed, client, _) = setup(1);
    let command = down(1);
    let invocation = ChromiumMouseInvocation::for_command(installed.page(), &command).unwrap();
    invocation
        .cancel(invocation.context(), CancellationReason::DeadlineExceeded)
        .await
        .unwrap();
    let (_, page, _, params) = command.into_parts();
    // Act
    let error = invocation.call_for(page, params).await.unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Timeout);
    assert_eq!(installed.page().retained_records(), 0);
    assert!(client.calls().is_empty());
}
