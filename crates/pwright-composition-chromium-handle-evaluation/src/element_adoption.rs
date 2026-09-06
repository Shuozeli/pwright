//! Cancellation-safe ownership around the optional Chromium adoption batch.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use pwright_backend_cdp_handle_evaluation::{
    CdpAdoptedElementHandle, CdpAdoptedElementHandleBatch, CdpElementHandleAdoptionBatchRequest,
};
use pwright_cap_handle_evaluation::HandleEvaluationErrorDetails;
use pwright_core::{FailureKind, OperationError, Service};
use pwright_runtime::{
    CleanupFailure, CleanupReservation, CleanupSubmitError, ReservedCleanupExecutor,
};

type AdoptionOutcome =
    Result<CdpAdoptedElementHandleBatch, OperationError<HandleEvaluationErrorDetails>>;
type BoxAdoptionFuture = Pin<Box<dyn Future<Output = AdoptionOutcome> + Send + 'static>>;
type ReleaseOutcome = Result<(), OperationError<HandleEvaluationErrorDetails>>;
type BoxReleaseFuture = Pin<Box<dyn Future<Output = ReleaseOutcome> + Send + 'static>>;

/// A temporary adopted batch whose cleanup slot was reserved before the first
/// browser side effect.
#[must_use = "an adopted ElementHandle batch must be explicitly released or dropped for continuation cleanup"]
pub struct ContinuableAdoptedElementHandleBatch<Reservation>
where
    Reservation: CleanupReservation,
{
    batch: Option<CdpAdoptedElementHandleBatch>,
    reservation: Option<Reservation>,
}

impl<Reservation> std::fmt::Debug for ContinuableAdoptedElementHandleBatch<Reservation>
where
    Reservation: CleanupReservation,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ContinuableAdoptedElementHandleBatch")
            .field(
                "handle_count",
                &self.batch.as_ref().map_or(0, |batch| batch.handles().len()),
            )
            .finish_non_exhaustive()
    }
}

impl<Reservation> ContinuableAdoptedElementHandleBatch<Reservation>
where
    Reservation: CleanupReservation,
{
    pub fn handles(&self) -> &[CdpAdoptedElementHandle] {
        self.batch
            .as_ref()
            .expect("live adopted batch retains its backend owner")
            .handles()
    }

    /// Starts explicit release. Dropping the returned future still transfers
    /// the in-flight release to the pre-reserved continuation.
    pub fn release(mut self) -> ContinuableAdoptedElementHandleReleaseFuture<Reservation> {
        let batch = self
            .batch
            .take()
            .expect("adopted batch can be released only once");
        let reservation = self
            .reservation
            .take()
            .expect("adopted batch owns one cleanup reservation");
        ContinuableAdoptedElementHandleReleaseFuture {
            inner: Some(Box::pin(batch.release())),
            reservation: Some(reservation),
        }
    }
}

impl<Reservation> Drop for ContinuableAdoptedElementHandleBatch<Reservation>
where
    Reservation: CleanupReservation,
{
    fn drop(&mut self) {
        let Some(batch) = self.batch.take() else {
            return;
        };
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        reservation.submit(Box::pin(async move {
            batch.release().await.map_err(|error| {
                CleanupFailure::new(format!(
                    "abandoned adopted ElementHandle batch cleanup failed: {error}"
                ))
            })
        }));
    }
}

pub struct ContinuableAdoptedElementHandleReleaseFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    inner: Option<BoxReleaseFuture>,
    reservation: Option<Reservation>,
}

impl<Reservation> Unpin for ContinuableAdoptedElementHandleReleaseFuture<Reservation> where
    Reservation: CleanupReservation
{
}

impl<Reservation> Future for ContinuableAdoptedElementHandleReleaseFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    type Output = ReleaseOutcome;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let outcome = self
            .inner
            .as_mut()
            .expect("completed adopted-batch release must not be polled again")
            .as_mut()
            .poll(context);
        if outcome.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        outcome
    }
}

impl<Reservation> Drop for ContinuableAdoptedElementHandleReleaseFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    fn drop(&mut self) {
        let Some(inner) = self.inner.take() else {
            return;
        };
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        reservation.submit(Box::pin(async move {
            inner.await.map_err(|error| {
                CleanupFailure::new(format!(
                    "cancelled adopted ElementHandle batch release failed: {error}"
                ))
            })
        }));
    }
}

pub struct ContinuableElementHandleAdoptionFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    inner: Option<BoxAdoptionFuture>,
    reservation: Option<Reservation>,
    started: bool,
}

impl<Reservation> Unpin for ContinuableElementHandleAdoptionFuture<Reservation> where
    Reservation: CleanupReservation
{
}

impl<Reservation> Future for ContinuableElementHandleAdoptionFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    type Output = Result<
        ContinuableAdoptedElementHandleBatch<Reservation>,
        OperationError<HandleEvaluationErrorDetails>,
    >;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let outcome = self
            .inner
            .as_mut()
            .expect("completed ElementHandle adoption future must not be polled again")
            .as_mut()
            .poll(context);
        match outcome {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(batch)) => {
                self.inner = None;
                let reservation = self
                    .reservation
                    .take()
                    .expect("successful adoption owns one cleanup reservation");
                Poll::Ready(Ok(ContinuableAdoptedElementHandleBatch {
                    batch: Some(batch),
                    reservation: Some(reservation),
                }))
            }
            Poll::Ready(Err(error)) => {
                self.inner = None;
                self.reservation = None;
                Poll::Ready(Err(error))
            }
        }
    }
}

impl<Reservation> Drop for ContinuableElementHandleAdoptionFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        let Some(inner) = self.inner.take() else {
            return;
        };
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        reservation.submit(Box::pin(async move {
            match inner.await {
                Ok(batch) => batch.release().await.map_err(|error| {
                    CleanupFailure::new(format!(
                        "abandoned ElementHandle adoption cleanup failed: {error}"
                    ))
                }),
                Err(_) => Ok(()),
            }
        }));
    }
}

/// Decorates an adoption batch service with caller-owned, pre-admitted cleanup
/// continuation. It performs no browser/runtime selection.
#[derive(Clone)]
pub struct ContinuableElementHandleAdoptionService<Inner, Continuation> {
    inner: Arc<Inner>,
    continuation: Continuation,
}

impl<Inner, Continuation> ContinuableElementHandleAdoptionService<Inner, Continuation> {
    pub const fn new(inner: Arc<Inner>, continuation: Continuation) -> Self {
        Self {
            inner,
            continuation,
        }
    }
}

impl<Inner, Continuation> Service<CdpElementHandleAdoptionBatchRequest>
    for ContinuableElementHandleAdoptionService<Inner, Continuation>
where
    Inner: Service<
            CdpElementHandleAdoptionBatchRequest,
            Response = CdpAdoptedElementHandleBatch,
            Error = OperationError<HandleEvaluationErrorDetails>,
        > + 'static,
    Continuation: ReservedCleanupExecutor,
{
    type Response = ContinuableAdoptedElementHandleBatch<Continuation::Reservation>;
    type Error = OperationError<HandleEvaluationErrorDetails>;
    type Future<'a>
        = ContinuableElementHandleAdoptionFuture<Continuation::Reservation>
    where
        Self: 'a;

    fn call(&self, request: CdpElementHandleAdoptionBatchRequest) -> Self::Future<'_> {
        let reservation = match self.continuation.try_reserve() {
            Ok(reservation) => reservation,
            Err(error) => {
                return ContinuableElementHandleAdoptionFuture {
                    inner: Some(Box::pin(std::future::ready(Err(continuation_failure(
                        error,
                    ))))),
                    reservation: None,
                    started: false,
                };
            }
        };
        let inner = Arc::clone(&self.inner);
        ContinuableElementHandleAdoptionFuture {
            inner: Some(Box::pin(async move { inner.call(request).await })),
            reservation: Some(reservation),
            started: false,
        }
    }
}

fn continuation_failure(error: CleanupSubmitError) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        format!("ElementHandle adoption cleanup continuation was not admitted: {error}"),
        Some(HandleEvaluationErrorDetails::InvalidValue(
            "ElementHandle adoption cleanup continuation capacity is exhausted".to_owned(),
        )),
    )
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};

    use pwright_backend_cdp_handle_evaluation::{
        CdpElementHandleAdoptionBatchRequest, CdpElementHandleAdoptionBatchService,
        CdpElementHandleAdoptionProductionProfile, CdpElementHandleAdoptionRequest,
        CdpHandleEvaluationLimits, CdpHandleEvaluationService,
    };
    use pwright_cap_handle_evaluation::EvaluateHandleParams;
    use pwright_core::{
        FailureKind, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceRequirements,
        ResourceScope, Service,
    };
    use pwright_fake::FakeCdpClient;
    use pwright_runtime::CleanupQueue;

    use super::ContinuableElementHandleAdoptionService;

    fn exact_resources(requirements: ResourceRequirements) -> ProductionResourceLedger {
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            limits = limits.with_class(class, required.global, required.per_scope);
        }
        ProductionResourceLedger::new(limits, requirements)
            .expect("exact adoption resource plan should build")
    }

    async fn fixture(
        continuation: CleanupQueue,
    ) -> (
        ContinuableElementHandleAdoptionService<CdpElementHandleAdoptionBatchService, CleanupQueue>,
        CdpElementHandleAdoptionBatchRequest,
        ProductionResourceLedger,
        Arc<FakeCdpClient>,
    ) {
        let profile = CdpElementHandleAdoptionProductionProfile::new(
            ResourceScope::extension(78, 1),
            CdpHandleEvaluationLimits::default(),
            1,
            1,
        );
        let resources = exact_resources(profile.requirements());
        let raw = Arc::new(
            CdpElementHandleAdoptionBatchService::new_with_production_resources(
                profile, &resources,
            )
            .unwrap(),
        );
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(serde_json::json!({
            "result": {
                "type": "object",
                "subtype": "node",
                "description": "source",
                "objectId": "source-node"
            }
        }));
        let source = CdpHandleEvaluationService::new(client.clone())
            .call(EvaluateHandleParams::expression("document.body"))
            .await
            .unwrap();
        client.set_object_backend_node("source-node", 71);
        client.set_adopted_node_response(
            71,
            29,
            serde_json::json!({
                "object": {
                    "type": "object",
                    "subtype": "node",
                    "description": "adopted",
                    "objectId": "adopted-node"
                }
            }),
        );
        let request =
            CdpElementHandleAdoptionBatchRequest::new(vec![CdpElementHandleAdoptionRequest::new(
                source,
                client.clone(),
                29,
            )]);
        (
            ContinuableElementHandleAdoptionService::new(raw, continuation),
            request,
            resources,
            client,
        )
    }

    fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
        let mut context = Context::from_waker(Waker::noop());
        future.poll(&mut context)
    }

    #[tokio::test]
    async fn missing_continuation_rejects_before_dom_io_and_resource_admission() {
        // Arrange
        let continuation = CleanupQueue::new(0);
        let (service, request, resources, client) = fixture(continuation).await;
        let calls_before = client.calls().len();

        // Act
        let error = service
            .call(request)
            .await
            .expect_err("missing continuation must reject adoption");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert_eq!(client.calls().len(), calls_before);
        assert_eq!(resources.metrics().unwrap().active_leases, 0);
    }

    #[tokio::test]
    async fn dropped_started_adoption_finishes_then_releases_through_reservation() {
        // Arrange
        let continuation = CleanupQueue::new(1);
        let (service, request, resources, client) = fixture(continuation.clone()).await;
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_adoption_gate(Some(Arc::clone(&gate)));
        let mut abandoned = Box::pin(service.call(request));

        // Act
        assert!(poll_now(abandoned.as_mut()).is_pending());
        assert_eq!(client.method_names().last().unwrap(), "DOM.resolveNode");
        drop(abandoned);
        client.set_adoption_gate(None);
        gate.notify_waiters();
        let mut cleanup = continuation
            .try_take()
            .unwrap()
            .expect("started adoption must transfer to continuation");
        let outcome = poll_now(cleanup.as_mut());

        // Assert
        assert!(matches!(outcome, Poll::Ready(Ok(()))));
        assert_eq!(
            client.method_names().last().unwrap(),
            "Runtime.releaseObject"
        );
        assert_eq!(resources.metrics().unwrap().active_leases, 0);
        assert_eq!(continuation.metrics().unwrap().completed, 1);
    }

    #[tokio::test]
    async fn dropping_a_completed_batch_submits_exact_release() {
        // Arrange
        let continuation = CleanupQueue::new(1);
        let (service, request, resources, client) = fixture(continuation.clone()).await;
        let batch = service.call(request).await.unwrap();
        assert_eq!(batch.handles().len(), 1);

        // Act
        drop(batch);
        let mut cleanup = continuation
            .try_take()
            .unwrap()
            .expect("dropped batch must submit release");
        let outcome = poll_now(cleanup.as_mut());

        // Assert
        assert!(matches!(outcome, Poll::Ready(Ok(()))));
        assert_eq!(
            client.method_names().last().unwrap(),
            "Runtime.releaseObject"
        );
        assert_eq!(resources.metrics().unwrap().active_leases, 0);
    }

    #[tokio::test]
    async fn dropping_an_unpolled_explicit_release_still_submits_cleanup() {
        // Arrange
        let continuation = CleanupQueue::new(1);
        let (service, request, resources, client) = fixture(continuation.clone()).await;
        let batch = service.call(request).await.unwrap();
        let release = batch.release();

        // Act
        drop(release);
        let mut cleanup = continuation
            .try_take()
            .unwrap()
            .expect("cancelled explicit release must submit cleanup");
        let outcome = poll_now(cleanup.as_mut());

        // Assert
        assert!(matches!(outcome, Poll::Ready(Ok(()))));
        assert_eq!(
            client.method_names().last().unwrap(),
            "Runtime.releaseObject"
        );
        assert_eq!(resources.metrics().unwrap().active_leases, 0);
    }
}
