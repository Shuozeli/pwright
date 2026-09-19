use super::*;
use crate::close_confirmation::RetirePageCloseOnOwnerLoss;
use crate::{
    ConfirmedChromiumPageCloseCommandService, PageCloseRetirement, RequirePageCloseSuccess,
};
use pwright_backend_cdp_page_lifecycle::{
    CdpPageCloseConfirmation, CdpPreparedPageClose, PageCloseFuture,
};
use pwright_cap_page_lifecycle::PageLifecycleErrorDetails;
use pwright_cdp::CdpClient;

struct Reject;
impl CdpPageCloseConfirmation for Reject {
    fn prepare(
        &self,
        _: &str,
    ) -> Result<Box<dyn CdpPreparedPageClose>, OperationError<PageLifecycleErrorDetails>> {
        Err(OperationError::new(
            FailureKind::ResourceExhausted,
            "no receipt capacity",
            None,
        ))
    }
}

#[test]
fn selected_retirement_keeps_operational_errors_and_strict_default_distinct() {
    for kind in [
        FailureKind::Cancelled,
        FailureKind::TargetClosed,
        FailureKind::TransportClosed,
        FailureKind::Backend,
        FailureKind::Timeout,
    ] {
        // Arrange
        let outcome = Err(OperationError::new(kind, "unconfirmed", None));
        // Act
        let selected = RetirePageCloseOnOwnerLoss::retired(&outcome);
        let ordinary = RequirePageCloseSuccess::retired(&outcome);
        // Assert
        assert_eq!(
            selected,
            matches!(
                kind,
                FailureKind::Cancelled | FailureKind::TargetClosed | FailureKind::TransportClosed
            )
        );
        assert!(!ordinary);
        assert!(
            outcome.is_err(),
            "cleanup policy does not rewrite operation success"
        );
    }
}

#[test]
fn owner_loss_retires_abandoned_wait_without_committing_page_disposal() {
    // Arrange
    let (runtime, topology, browser, _) = page_close_state();
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        None,
        browser.clone(),
        reservation_bytes(),
    ));
    let (sent, received) = tokio::sync::oneshot::channel();
    let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
        ResourceClass::BackendStateBytes,
        1024,
        1024,
    ));
    let confirmation = Arc::new(Native {
        browser: browser.clone(),
        receipt: Mutex::new(Some(received)),
        ledger: ledger.clone(),
    });
    let raw = ConfirmedChromiumPageCloseCommandService::new(
        page_service(&runtime, topology.clone(), browser),
        observer.clone(),
        confirmation,
    );
    let cleanup = CleanupQueue::new(1);
    let service = ContinuablePageCloseCommandService::with_retirement_policy::<
        RetirePageCloseOnOwnerLoss,
    >(Arc::new(raw), cleanup.clone());
    let mut call = Box::pin(service.call(page_command()));
    assert!(poll_now(call.as_mut()).is_pending());
    drop(call);
    sent.send(Err(FailureKind::Cancelled)).unwrap();
    let mut continuation = cleanup.try_take().unwrap().unwrap();
    // Act
    let retired = poll_now(continuation.as_mut());
    // Assert
    assert!(matches!(retired, Poll::Ready(Ok(()))));
    assert_eq!(runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert!(!runtime.lifecycle_transaction_active().unwrap());
    assert!(!topology.lock().unwrap().transaction_active());
    assert_eq!(observer.record.lock().unwrap().commits, 0);
    assert_eq!(observer.record.lock().unwrap().abandoned, 1);
    assert_eq!(observer.retained(), 0);
    assert_eq!(ledger.metrics().unwrap().active_leases, 0);
}

struct Native {
    browser: Arc<FakeCdpClient>,
    receipt: Mutex<Option<tokio::sync::oneshot::Receiver<Result<(), FailureKind>>>>,
    ledger: ResourceLedger,
}
struct Prepared {
    browser: Arc<FakeCdpClient>,
    receipt: tokio::sync::oneshot::Receiver<Result<(), FailureKind>>,
    _storage: ResourceLease,
}
impl CdpPageCloseConfirmation for Native {
    fn prepare(
        &self,
        target: &str,
    ) -> Result<Box<dyn CdpPreparedPageClose>, OperationError<PageLifecycleErrorDetails>> {
        assert_eq!(target, "target-a");
        let storage = self
            .ledger
            .reserve(SCOPE, ResourceClass::BackendStateBytes, 1024)
            .unwrap();
        Ok(Box::new(Prepared {
            browser: self.browser.clone(),
            receipt: self.receipt.lock().unwrap().take().unwrap(),
            _storage: storage,
        }))
    }
}
impl CdpPreparedPageClose for Prepared {
    fn start(
        self: Box<Self>,
    ) -> Result<PageCloseFuture, OperationError<PageLifecycleErrorDetails>> {
        Ok(Box::pin(async move {
            let Prepared {
                browser,
                receipt,
                _storage: storage,
            } = *self;
            browser.target_close("target-a").await.unwrap();
            receipt
                .await
                .unwrap()
                .map_err(|kind| OperationError::new(kind, "owner retired", None))?;
            drop(storage);
            Ok(())
        }))
    }
}

#[tokio::test]
async fn confirmation_admission_failure_precedes_native_mutation_and_releases_other_reservations() {
    // Arrange
    let (runtime, topology, browser, _) = page_close_state();
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        None,
        browser.clone(),
        reservation_bytes(),
    ));
    let service = ConfirmedChromiumPageCloseCommandService::new(
        page_service(&runtime, topology.clone(), browser.clone()),
        observer.clone(),
        Arc::new(Reject),
    );
    // Act
    let result = service.call(page_command()).await.unwrap().into_outcome();
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::ResourceExhausted);
    assert!(browser.method_names().is_empty());
    assert_eq!(runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert!(!runtime.lifecycle_transaction_active().unwrap());
    assert!(!topology.lock().unwrap().transaction_active());
    assert_eq!(observer.record.lock().unwrap().commits, 0);
    assert_eq!(observer.record.lock().unwrap().abandoned, 1);
    assert_eq!(observer.retained(), 0);
}

#[test]
fn dropped_call_transfers_native_receipt_wait_to_cleanup_before_publication() {
    // Arrange
    let (runtime, topology, browser, _) = page_close_state();
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        Some(runtime.service().clone()),
        browser.clone(),
        reservation_bytes(),
    ));
    let (sent, received) = tokio::sync::oneshot::channel();
    let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
        ResourceClass::BackendStateBytes,
        1024,
        1024,
    ));
    let confirmation = Arc::new(Native {
        browser: browser.clone(),
        receipt: Mutex::new(Some(received)),
        ledger: ledger.clone(),
    });
    let raw = ConfirmedChromiumPageCloseCommandService::new(
        page_service(&runtime, topology.clone(), browser.clone()),
        observer.clone(),
        confirmation,
    );
    let cleanup = CleanupQueue::new(1);
    let service = ContinuablePageCloseCommandService::new(Arc::new(raw), cleanup.clone());
    let mut future = Box::pin(service.call(page_command()));
    assert!(poll_now(future.as_mut()).is_pending());
    assert_eq!(browser.method_names(), ["Target.closeTarget"]);
    // Act
    drop(future);
    // Assert: command acknowledgement and public cancellation are not closure.
    assert_eq!(runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert!(topology.lock().unwrap().transaction_active());
    assert_eq!(observer.record.lock().unwrap().commits, 0);
    assert_eq!(observer.record.lock().unwrap().abandoned, 0);
    assert_eq!(observer.retained(), reservation_bytes());
    assert_eq!(ledger.metrics().unwrap().active_leases, 1);
    sent.send(Ok(())).unwrap();
    let mut continuation = cleanup.try_take().unwrap().unwrap();
    assert!(matches!(
        poll_now(continuation.as_mut()),
        Poll::Ready(Ok(()))
    ));
    assert_eq!(runtime.is_object_live(ObjectId::new(3)), Ok(false));
    assert_eq!(observer.record.lock().unwrap().commits, 1);
    assert_eq!(observer.retained(), 0);
    assert_eq!(ledger.metrics().unwrap().active_leases, 0);
}
