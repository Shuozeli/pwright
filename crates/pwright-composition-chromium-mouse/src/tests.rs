use super::*;
#[cfg(feature = "runtime")]
mod controlled;
mod dispatch;
#[tokio::test]
async fn native_admission_failure_rolls_back_the_tentative_route_record() {
    // Arrange
    let (mut installed, _, _) = setup_with_backend(2, 1);
    let page = installed.page();
    let first = page.call(down(1));
    // Act
    let rejected = page
        .call(down(2))
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(rejected.kind(), FailureKind::ResourceExhausted);
    assert_eq!(page.retained_records(), 1);
    assert_eq!(page.queued_operations(), 1);
    drop(first);
    installed.retirement().next().await.into_outcome().unwrap();
    drop(page.call(down(3)));
    installed.retirement().next().await.into_outcome().unwrap();
    assert_eq!(page.retained_records(), 0);
}
#[tokio::test]
async fn first_cancellation_reason_is_not_overwritten_by_later_transport_close() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let page = installed.page();
    let response = page.call(down(1));
    // Act
    page.request_cancel(
        context(1, MouseDown::METHOD),
        CancellationReason::DeadlineExceeded,
    )
    .unwrap();
    page.request_cancel_all(CancellationReason::TransportClosed);
    // Assert
    assert_eq!(
        response.await.unwrap().into_outcome().unwrap_err().kind(),
        FailureKind::Timeout
    );
    installed.retirement().next().await.into_outcome().unwrap();
    assert!(client.calls().is_empty());
}
#[derive(Default)]
struct Transport(Mutex<Option<pwright_transport::WireFrame>>);
impl pwright_transport::FrameTransport for Transport {
    type Error = Infallible;
    type Send<'a> = std::future::Ready<Result<(), Infallible>>;
    type Receive<'a> = std::future::Ready<Result<Option<pwright_transport::WireFrame>, Infallible>>;
    fn send(&self, frame: pwright_transport::WireFrame) -> Self::Send<'_> {
        let previous = self.0.lock().unwrap().replace(frame);
        assert!(previous.is_none(), "one retained reply per case");
        ready(Ok(()))
    }
    fn receive(&self) -> Self::Receive<'_> {
        ready(Ok(None))
    }
}
#[tokio::test]
async fn all_six_generated_wire_routes_execute_and_return_separate_retirement_receipts() {
    use pwright_server_wire::{ConcurrentServerFrameHandler, ServerFrameDisposition};
    // Arrange
    let (mut installed, client, _) = setup(1);
    let protocol = installed.protocol();
    let handlers = installed.handlers();
    let transport = Arc::new(Transport::default());
    let x = MouseNumber::try_from(20.0).unwrap();
    let y = MouseNumber::try_from(30.0).unwrap();
    macro_rules! exercise {
        ($command:ty,$id:expr,$params:expr) => {{
            let command =
                Command::<$command>::new(CallId::new($id), ObjectId::new(7), metadata(), $params);
            let frame = protocol.encode(&command).unwrap();
            // Act
            let handled = handlers
                .handle_concurrently(frame, transport.clone())
                .await
                .unwrap();
            let receipt = installed.retirement().next().await;
            // Assert
            assert!(matches!(handled, ServerFrameDisposition::Handled(_)));
            assert_eq!(receipt.context(), context($id, <$command>::METHOD));
            receipt.into_outcome().unwrap();
            let frame = transport.0.lock().unwrap().take().unwrap();
            let result = protocol.decode::<CommandResult<$command>>(frame).unwrap();
            assert_eq!(result.call_id(), CallId::new($id));
            result.into_outcome().unwrap();
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
use pwright_backend_cdp_input::CdpInputPolicy;
use pwright_backend_cdp_mouse::{CdpMouseServices, MouseNativeFuture};
use pwright_core::{CallMetadata, CancellationId, FailureKind, ResourceLimits};
use pwright_fake::FakeCdpClient;
use std::{
    future::{pending, ready},
    sync::{Mutex, atomic::AtomicUsize},
    task::{Wake, Waker},
    time::Duration,
};
struct Delay;
impl CdpMouseDelay for Delay {
    fn wait(self: Arc<Self>, _: Duration) -> MouseNativeFuture {
        Box::pin(pending())
    }
}
fn resources(required: ResourceRequirements) -> ProductionResourceLedger {
    let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
    for class in ResourceClass::ALL {
        let bytes = required.class(class);
        limits = limits.with_class(class, bytes.global, bytes.per_scope);
    }
    ProductionResourceLedger::new(limits, required).unwrap()
}
fn setup(
    capacity: usize,
) -> (
    ChromiumMouseInstallation,
    Arc<FakeCdpClient>,
    ProductionResourceLedger,
) {
    setup_with_backend(capacity, capacity)
}
fn setup_with_backend(
    capacity: usize,
    backend_count: usize,
) -> (
    ChromiumMouseInstallation,
    Arc<FakeCdpClient>,
    ProductionResourceLedger,
) {
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let capacity = NonZeroUsize::new(capacity).unwrap();
    let bytes = CdpMouseServices::required_operation_bytes(client.clone(), &input).unwrap();
    let backend = CdpMouseProductionProfile::new(
        ResourceScope::extension(14, 44),
        bytes,
        NonZeroUsize::new(backend_count).unwrap(),
    )
    .unwrap();
    let profile = ChromiumMouseProfile::new(
        backend,
        capacity,
        MouseCodecLimits::new(4096),
        ProtocolVersion::new(1, 0),
        FrameLimits::new(16384, 0, 0, 16384),
        FrameLimits::new(16384, 0, 0, 16384),
    )
    .unwrap();
    let resources = resources(profile.requirements());
    let installation = ChromiumMouseInstallation::install(
        ObjectId::new(7),
        client.clone(),
        input,
        profile,
        Arc::new(Delay),
        &resources,
    )
    .unwrap();
    (installation, client, resources)
}
fn metadata() -> CallMetadata {
    CallMetadata::new(None, Some(CancellationId::new(9)))
}
fn down(id: u64) -> Command<MouseDown> {
    Command::new(
        CallId::new(id),
        ObjectId::new(7),
        metadata(),
        DownParams {
            button: MouseButton::Left,
            click_count: 1,
        },
    )
}
fn click(id: u64) -> Command<MouseClick> {
    Command::new(
        CallId::new(id),
        ObjectId::new(7),
        metadata(),
        ClickParams {
            x: MouseNumber::try_from(20.0).unwrap(),
            y: MouseNumber::try_from(30.0).unwrap(),
            button: MouseButton::Left,
            click_count: MouseClickCount::ONE,
            delay: None,
        },
    )
}
fn context(id: u64, method: pwright_core::MethodTag) -> OperationContext {
    OperationContext::new(
        CallId::new(id),
        ObjectId::new(7),
        method,
        metadata().cancellation,
    )
}
fn poll_pending<F: Future + Unpin>(future: &mut F) {
    assert!(
        Pin::new(future)
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
}

#[tokio::test]
async fn wrong_page_rejects_before_native_or_record_admission() {
    // Arrange
    let (installed, client, resources) = setup(1);
    let page = installed.page();
    let command = Command::<MouseDown>::new(
        CallId::new(1),
        ObjectId::new(8),
        metadata(),
        DownParams {
            button: MouseButton::Left,
            click_count: 1,
        },
    );
    // Act
    let result = page
        .call(command)
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(result.kind(), FailureKind::ProtocolViolation);
    assert_eq!(page.retained_records(), 0);
    assert_eq!(page.queued_operations(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    assert!(client.calls().is_empty());
}
#[tokio::test]
async fn duplicate_call_id_does_not_replace_the_original_owner() {
    // Arrange
    let (mut installed, client, _) = setup(2);
    let page = installed.page();
    let original = page.call(down(1));
    // Act
    let rejected = page
        .call(click(1))
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(rejected.kind(), FailureKind::ProtocolViolation);
    assert_eq!(page.queued_operations(), 1);
    drop(original);
    let receipt = installed.retirement().next().await;
    assert_eq!(receipt.context(), context(1, MouseDown::METHOD));
    receipt.into_outcome().unwrap();
    assert!(client.calls().is_empty());
    assert_eq!(page.retained_records(), 0);
}
#[tokio::test]
async fn early_error_publishes_result_without_dropping_pending_tail() {
    // Arrange
    let (mut installed, client, resources) = setup(1);
    let page = installed.page();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_input_errors([Some("move failed".into()), None, None]);
    client.set_pointer_gates([None, Some(gate.clone()), None]);
    // Act
    let result = page
        .call(click(1))
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(result.kind(), FailureKind::Backend);
    assert_eq!(client.calls().len(), 3);
    assert_eq!(page.retained_records(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 3);
    let mut retirement = Box::pin(installed.retirement().next());
    poll_pending(&mut retirement);
    gate.notify_one();
    let receipt = retirement.await;
    assert_eq!(page.queued_operations(), 0);
    assert_eq!(page.retained_records(), 1);
    assert!(receipt.outcome().is_ok());
    drop(receipt);
    assert_eq!(page.retained_records(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    assert_eq!(client.calls().len(), 3);
}
#[tokio::test]
async fn dropping_pending_response_keeps_native_call_until_driver_retirement() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let page = installed.page();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let mut response = page.call(down(1));
    poll_pending(&mut response);
    // Act
    drop(response);
    // Assert
    let mut retirement = Box::pin(installed.retirement().next());
    poll_pending(&mut retirement);
    assert_eq!(client.calls().len(), 1);
    gate.notify_one();
    retirement.await.into_outcome().unwrap();
    assert_eq!(page.retained_records(), 0);
    assert_eq!(client.calls().len(), 1);
}
#[tokio::test]
async fn unpolled_response_drop_never_dispatches_native_work() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let page = installed.page();
    let response = page.call(click(1));
    // Act
    drop(response);
    installed.retirement().next().await.into_outcome().unwrap();
    // Assert
    assert!(client.calls().is_empty());
    assert_eq!(page.retained_records(), 0);
}
#[tokio::test]
async fn retained_receipt_keeps_record_quota_even_after_slot_is_free() {
    // Arrange
    let (mut installed, _, resources) = setup(1);
    let page = installed.page();
    page.call(down(1)).await.unwrap().into_outcome().unwrap();
    let receipt = installed.retirement().next().await;
    // Act
    let rejected = page
        .call(down(2))
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(page.queued_operations(), 0);
    assert_eq!(page.retained_records(), 1);
    assert!(rejected.message().contains("records retained"));
    assert_eq!(resources.metrics().unwrap().active_leases, 3);
    drop(receipt);
    drop(page.call(down(3)));
    installed.retirement().next().await.into_outcome().unwrap();
    assert_eq!(page.retained_records(), 0);
}
#[tokio::test]
async fn cancelled_response_keeps_reason_and_record_after_driver_finishes_first() {
    // Arrange
    let (mut installed, client, resources) = setup(1);
    let page = installed.page();
    let response = page.call(down(1));
    // Act
    page.request_cancel(
        context(1, MouseDown::METHOD),
        CancellationReason::DeadlineExceeded,
    )
    .unwrap();
    installed.retirement().next().await.into_outcome().unwrap();
    // Assert
    assert_eq!(page.queued_operations(), 0);
    assert_eq!(page.retained_records(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 2);
    let result = response.await.unwrap().into_outcome().unwrap_err();
    assert_eq!(result.kind(), FailureKind::Timeout);
    assert_eq!(page.retained_records(), 0);
    assert!(client.calls().is_empty());
}
#[tokio::test]
async fn cancellation_requires_exact_page_method_and_cancellation_identity() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let page = installed.page();
    let response = page.call(down(1));
    let cases = [
        OperationContext::new(
            CallId::new(1),
            ObjectId::new(8),
            MouseDown::METHOD,
            metadata().cancellation,
        ),
        context(1, MouseUp::METHOD),
        OperationContext::new(
            CallId::new(1),
            ObjectId::new(7),
            MouseDown::METHOD,
            Some(CancellationId::new(10)),
        ),
        context(2, MouseDown::METHOD),
    ];
    // Act
    let errors = cases.map(|context| {
        page.request_cancel(context, CancellationReason::Caller)
            .unwrap_err()
    });
    // Assert
    for error in errors {
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    }
    response.await.unwrap().into_outcome().unwrap();
    installed.retirement().next().await.into_outcome().unwrap();
    assert_eq!(client.calls().len(), 1);
}
#[tokio::test]
async fn revocation_rejects_unpolled_and_new_calls_without_native_input() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let page = installed.page();
    let response = page.call(down(1));
    // Act
    page.revoke_admission();
    let original = response.await.unwrap().into_outcome().unwrap_err();
    let new = page
        .call(down(2))
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(original.kind(), FailureKind::TargetClosed);
    assert_eq!(new.kind(), FailureKind::TargetClosed);
    installed.retirement().next().await.into_outcome().unwrap();
    assert!(client.calls().is_empty());
}
#[tokio::test]
async fn revocation_does_not_implicitly_abort_an_already_polled_native_call() {
    // Arrange
    let (mut installed, client, _) = setup(1);
    let page = installed.page();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let mut response = page.call(down(1));
    poll_pending(&mut response);
    // Act
    page.revoke_admission();
    gate.notify_one();
    // Assert
    response.await.unwrap().into_outcome().unwrap();
    installed.retirement().next().await.into_outcome().unwrap();
    assert_eq!(client.calls().len(), 1);
}
#[tokio::test]
async fn driver_does_not_head_of_line_block_ready_retirement_behind_pending_call() {
    // Arrange
    let (mut installed, client, _) = setup(2);
    let page = installed.page();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone()), None]);
    let mut first = page.call(down(1));
    poll_pending(&mut first);
    drop(first);
    page.call(down(2)).await.unwrap().into_outcome().unwrap();
    // Act
    let receipt = installed.retirement().next().await;
    // Assert
    assert_eq!(receipt.context().call_id(), CallId::new(2));
    receipt.into_outcome().unwrap();
    assert_eq!(page.queued_operations(), 1);
    gate.notify_one();
    installed.retirement().next().await.into_outcome().unwrap();
    assert_eq!(page.queued_operations(), 0);
}
#[tokio::test]
async fn each_retirement_failure_has_its_own_receipt() {
    // Arrange
    let (mut installed, client, _) = setup(2);
    let page = installed.page();
    let gates = [
        Arc::new(tokio::sync::Notify::new()),
        Arc::new(tokio::sync::Notify::new()),
    ];
    client.set_input_errors([
        Some("first native failure".into()),
        Some("second native failure".into()),
    ]);
    client.set_pointer_gates(gates.iter().map(|gate| Some(gate.clone())));
    let mut first = page.call(down(1));
    poll_pending(&mut first);
    drop(first);
    let mut second = page.call(down(2));
    poll_pending(&mut second);
    drop(second);
    // Act
    for gate in gates {
        gate.notify_one();
    }
    let first = installed.retirement().next().await;
    let second = installed.retirement().next().await;
    // Assert
    assert_eq!(first.context().call_id(), CallId::new(1));
    assert_eq!(second.context().call_id(), CallId::new(2));
    assert!(
        first
            .outcome()
            .as_ref()
            .unwrap_err()
            .message()
            .contains("first")
    );
    assert!(
        second
            .outcome()
            .as_ref()
            .unwrap_err()
            .message()
            .contains("second")
    );
    assert_eq!(
        first.outcome().as_ref().unwrap_err().kind(),
        FailureKind::Cleanup
    );
    assert_eq!(
        second.outcome().as_ref().unwrap_err().kind(),
        FailureKind::Cleanup
    );
}
#[derive(Default)]
struct Counter(AtomicUsize);
impl Wake for Counter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}
#[tokio::test]
async fn abandoned_response_wakes_idle_caller_owned_driver() {
    // Arrange
    let (mut installed, _, _) = setup(1);
    let page = installed.page();
    let counter = Arc::new(Counter::default());
    let waker = Waker::from(counter.clone());
    assert!(
        installed
            .retirement()
            .poll_next(&mut Context::from_waker(&waker))
            .is_pending()
    );
    let response = page.call(down(1));
    // Act
    drop(response);
    // Assert
    assert_eq!(counter.0.load(Ordering::Relaxed), 1);
    installed.retirement().next().await.into_outcome().unwrap();
}
#[tokio::test]
async fn independent_driver_retains_route_after_installation_and_page_drop() {
    // Arrange
    let (installed, client, resources) = setup(1);
    let (page, protocol, mut driver) = installed.into_parts();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let mut response = page.call(down(1));
    poll_pending(&mut response);
    // Act
    drop(response);
    drop(page);
    drop(protocol);
    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 3);
    gate.notify_one();
    driver.next().await.into_outcome().unwrap();
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    drop(driver);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
#[test]
fn unrepresentable_route_capacity_rejects_before_allocation() {
    // Arrange
    let capacity = NonZeroUsize::new(usize::MAX).unwrap();
    // Act
    let result = ChromiumMousePage::route_requirements(capacity);
    // Assert
    assert!(matches!(
        result,
        Err(ChromiumMouseInstallError::CapacityOverflow)
    ));
}
