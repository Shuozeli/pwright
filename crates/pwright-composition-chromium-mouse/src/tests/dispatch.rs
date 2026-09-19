use super::*;
use pwright_backend_cdp_input::{MouseDispatchOperation, MouseDispatchRequest};
use pwright_cdp::connection::{CdpError, Result as CdpResult};

/// A controlled provider that represents started setup and asynchronous cleanup.
/// It never emits pointer input: cancellation must not turn cleanup into input.
struct Dispatcher {
    gate: Arc<tokio::sync::Notify>,
    starts: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
    result: ResultMode,
    cleanup_error: bool,
}
#[derive(Clone, Copy)]
enum ResultMode {
    Pending,
    Success,
    Error,
}
struct Operation {
    cleanup: Pin<Box<dyn Future<Output = ()> + Send>>,
    drops: Arc<AtomicUsize>,
    result: ResultMode,
    cleanup_error: bool,
    retired: bool,
    result_waker: Option<Waker>,
}
impl CdpMouseDispatcher for Dispatcher {
    fn start(
        &self,
        _: Arc<dyn CdpClient>,
        _: MouseDispatchRequest,
    ) -> Box<dyn MouseDispatchOperation> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        let gate = self.gate.clone();
        Box::new(Operation {
            cleanup: Box::pin(async move { gate.notified().await }),
            drops: self.drops.clone(),
            result: self.result,
            cleanup_error: self.cleanup_error,
            retired: false,
            result_waker: None,
        })
    }
}
impl MouseDispatchOperation for Operation {
    fn poll_result(&mut self, cx: &mut Context<'_>) -> Poll<CdpResult<()>> {
        match self.result {
            ResultMode::Pending => {
                self.result_waker = Some(cx.waker().clone());
                Poll::Pending
            }
            ResultMode::Success => Poll::Ready(Ok(())),
            ResultMode::Error => Poll::Ready(Err(CdpError::Other("provider action error".into()))),
        }
    }
    fn poll_retire(&mut self, cx: &mut Context<'_>) -> Poll<CdpResult<()>> {
        self.result_waker = None;
        if self.retired {
            return Poll::Ready(Ok(()));
        }
        if self.cleanup.as_mut().poll(cx).is_pending() {
            return Poll::Pending;
        }
        self.retired = true;
        Poll::Ready(if self.cleanup_error {
            Err(CdpError::Other("provider cleanup error".into()))
        } else {
            Ok(())
        })
    }
    fn is_retired(&self) -> bool {
        self.retired
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

type Setup = (
    Arc<ChromiumMousePage>,
    MouseRetirementDriver,
    Arc<Dispatcher>,
    CdpMouseServiceFactory,
    ProductionResourceLedger,
    Arc<FakeCdpClient>,
);
fn selected(mode: ResultMode, cleanup_error: bool) -> Setup {
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let capacity = NonZeroUsize::new(1).unwrap();
    let profile = CdpMouseProductionProfile::new(
        ResourceScope::extension(14, 44),
        CdpMouseServices::required_operation_bytes(client.clone(), &input).unwrap(),
        capacity,
    )
    .unwrap();
    let resources = resources(
        profile
            .requirements()
            .combine_concurrent(ChromiumMousePage::route_requirements(capacity).unwrap()),
    );
    let factory =
        CdpMouseServiceFactory::new_with_production_resources(profile, &resources, Arc::new(Delay))
            .unwrap();
    let dispatcher = Arc::new(Dispatcher {
        gate: Arc::new(tokio::sync::Notify::new()),
        starts: Arc::new(AtomicUsize::new(0)),
        drops: Arc::new(AtomicUsize::new(0)),
        result: mode,
        cleanup_error,
    });
    let (page, driver) = ChromiumMousePage::bind_with_dispatcher(
        ObjectId::new(7),
        client.clone(),
        input,
        &factory,
        capacity,
        &resources,
        dispatcher.clone(),
    )
    .unwrap();
    (page, driver, dispatcher, factory, resources, client)
}

#[tokio::test]
async fn dropped_pending_delegated_response_keeps_native_admission_until_cleanup_receipt_consumed()
{
    // Arrange
    let (page, mut driver, dispatcher, factory, resources, client) =
        selected(ResultMode::Pending, false);
    let mut response = page.call(down(1));
    poll_pending(&mut response);
    // Act
    drop(response);
    let mut retirement = Box::pin(driver.next());
    poll_pending(&mut retirement);
    // Assert
    assert_eq!(dispatcher.drops.load(Ordering::SeqCst), 0);
    assert_eq!(factory.active_operations(), 1);
    assert_eq!(page.queued_operations(), 1);
    drop(retirement);
    dispatcher.gate.notify_one();
    let receipt = driver.next().await;
    assert_eq!(receipt.context(), context(1, MouseDown::METHOD));
    assert_eq!(dispatcher.drops.load(Ordering::SeqCst), 1);
    assert_eq!(factory.active_operations(), 1);
    receipt.into_outcome().unwrap();
    assert_eq!(factory.active_operations(), 0);
    assert_eq!(page.retained_records(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    assert!(client.calls().is_empty());
}

#[tokio::test]
async fn successful_delegated_button_response_does_not_certify_provider_cleanup() {
    // Arrange
    let (page, mut driver, dispatcher, factory, _, _) = selected(ResultMode::Success, false);
    // Act
    page.call(down(1)).await.unwrap().into_outcome().unwrap();
    // Assert
    assert_eq!(factory.active_operations(), 1);
    assert_eq!(dispatcher.drops.load(Ordering::SeqCst), 0);
    let mut receipt = Box::pin(driver.next());
    poll_pending(&mut receipt);
    dispatcher.gate.notify_one();
    receipt.await.into_outcome().unwrap();
    assert_eq!(factory.active_operations(), 0);
    assert_eq!(dispatcher.drops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn delegated_action_and_cleanup_failures_keep_separate_classification_and_identity() {
    // Arrange
    let (page, mut driver, dispatcher, factory, _, _) = selected(ResultMode::Error, true);
    // Act
    let action = page
        .call(down(1))
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(action.kind(), FailureKind::Backend);
    assert!(action.to_string().contains("provider action error"));
    dispatcher.gate.notify_one();
    let receipt = driver.next().await;
    assert_eq!(receipt.context(), context(1, MouseDown::METHOD));
    let cleanup = receipt.into_outcome().unwrap_err();
    assert_eq!(cleanup.kind(), FailureKind::Cleanup);
    assert!(cleanup.to_string().contains("provider cleanup error"));
    assert_eq!(factory.active_operations(), 0);
    assert_eq!(page.retained_records(), 0);
}

#[tokio::test]
async fn unpolled_delegated_response_cancellation_never_constructs_provider() {
    // Arrange
    let (page, mut driver, dispatcher, factory, _, client) = selected(ResultMode::Pending, false);
    let response = page.call(down(1));
    // Act
    drop(response);
    driver.next().await.into_outcome().unwrap();
    // Assert
    assert_eq!(dispatcher.starts.load(Ordering::SeqCst), 0);
    assert_eq!(factory.active_operations(), 0);
    assert!(client.calls().is_empty());
}

#[cfg(feature = "runtime")]
#[tokio::test]
async fn controlled_cancellation_ack_waits_for_delegated_cleanup_and_consumed_receipt() {
    use pwright_core::CommandCancellation;
    use pwright_runtime::TargetedService;
    // Arrange
    let (page, mut driver, dispatcher, factory, _, client) = selected(ResultMode::Pending, false);
    let command = down(1);
    let invocation = ChromiumMouseInvocation::for_command(page.clone(), &command).unwrap();
    let (_, page_id, _, params) = command.into_parts();
    let mut result = invocation.call_for(page_id, params);
    poll_pending(&mut result);
    // Act
    let mut cancelled = invocation.cancel(invocation.context(), CancellationReason::Caller);
    poll_pending(&mut cancelled);
    let mut retirement = Box::pin(driver.next());
    poll_pending(&mut retirement);
    // Assert
    assert_eq!(dispatcher.drops.load(Ordering::SeqCst), 0);
    assert_eq!(factory.active_operations(), 1);
    dispatcher.gate.notify_one();
    let receipt = retirement.await;
    assert_eq!(receipt.context(), invocation.context());
    poll_pending(&mut cancelled);
    assert_eq!(factory.active_operations(), 1);
    receipt.into_outcome().unwrap();
    cancelled.await.unwrap();
    assert_eq!(factory.active_operations(), 0);
    assert_eq!(result.await.unwrap_err().kind(), FailureKind::Cancelled);
    drop(invocation);
    assert_eq!(page.retained_records(), 0);
    assert!(client.calls().is_empty());
}
