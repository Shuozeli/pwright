//! Pre-admitted cleanup across a single Locator resolution attempt and owner.
//! The caller supplies the executor. Optional strict waiting uses injected
//! routes, pacing and termination; neither path adopts or evaluates user code.

#[cfg(feature = "retained-reference")]
mod retained;
mod wait;
#[cfg(feature = "retained-reference")]
pub use retained::{RetainedElementRequest, RetainedResolutionFuture};
pub use wait::{
    ResolutionRoute, ResolutionRouteSource, ResolutionWaitError, ResolutionWaitProfile,
    ResolutionWaitStop, StrictResolutionWaiter,
};

pub use pwright_backend_cdp_locator_resolution::{
    CdpLocatorResolutionService, ObjectGroupDestructionProof, ResolutionError, ResolutionProfile,
    ResolutionRequest, ResolvedElement,
};
use pwright_core::Service;
use pwright_runtime::{CleanupFailure, CleanupReservation, ReservedCleanupExecutor};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

type Outcome = Result<Option<ResolvedElement>, ResolutionError>;
type Attempt = Pin<Box<dyn Future<Output = Outcome> + Send>>;
type Release = Pin<Box<dyn Future<Output = Result<(), ResolutionError>> + Send>>;

/// Caller-budgeted route storage retained through native completion and group
/// cleanup. Cloning this reference does not admit another route/snapshot.
pub type ResolutionRouteOwner = Arc<dyn Send + Sync>;

#[must_use = "drop or explicitly release the resolved element to release its group"]
pub struct OwnedElement<R: CleanupReservation> {
    element: Option<ResolvedElement>,
    reservation: Option<R>,
    route_owner: Option<ResolutionRouteOwner>,
}

impl<R: CleanupReservation> OwnedElement<R> {
    pub fn element(&self) -> &ResolvedElement {
        self.element.as_ref().expect("live element owner")
    }
    pub fn release(mut self) -> ReleaseFuture<R> {
        let element = self.element.take().expect("live element owner");
        let route_owner = self.route_owner.take();
        ReleaseFuture {
            inner: Some(Box::pin(async move {
                let _route_owner = route_owner;
                element.release().await
            })),
            reservation: self.reservation.take(),
        }
    }
}

impl<R: CleanupReservation> Drop for OwnedElement<R> {
    fn drop(&mut self) {
        if let (Some(element), Some(reservation)) = (self.element.take(), self.reservation.take()) {
            let route_owner = self.route_owner.take();
            reservation.submit(Box::pin(async move {
                let _route_owner = route_owner;
                element.release().await.map_err(cleanup_error)
            }));
        }
    }
}

pub struct ReleaseFuture<R: CleanupReservation> {
    inner: Option<Release>,
    reservation: Option<R>,
}
impl<R: CleanupReservation> Unpin for ReleaseFuture<R> {}
impl<R: CleanupReservation> Future for ReleaseFuture<R> {
    type Output = Result<(), ResolutionError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result = self
            .inner
            .as_mut()
            .expect("release cannot be polled after completion")
            .as_mut()
            .poll(cx);
        if result.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        result
    }
}
impl<R: CleanupReservation> Drop for ReleaseFuture<R> {
    fn drop(&mut self) {
        if let (Some(inner), Some(reservation)) = (self.inner.take(), self.reservation.take()) {
            reservation.submit(Box::pin(async move { inner.await.map_err(cleanup_error) }));
        }
    }
}

pub struct ResolutionFuture<R: CleanupReservation> {
    inner: Option<Attempt>,
    reservation: Option<R>,
    started: bool,
    route_owner: Option<ResolutionRouteOwner>,
}
impl<R: CleanupReservation> Unpin for ResolutionFuture<R> {}
impl<R: CleanupReservation> Future for ResolutionFuture<R> {
    type Output = Result<Option<OwnedElement<R>>, ResolutionError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let result = self
            .inner
            .as_mut()
            .expect("resolution cannot be polled after completion")
            .as_mut()
            .poll(cx);
        match result {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(Some(element))) => {
                self.inner = None;
                Poll::Ready(Ok(Some(OwnedElement {
                    element: Some(element),
                    reservation: self.reservation.take(),
                    route_owner: self.route_owner.take(),
                })))
            }
            Poll::Ready(result) => {
                self.inner = None;
                self.reservation = None;
                self.route_owner = None;
                Poll::Ready(result.map(|_| None))
            }
        }
    }
}
impl<R: CleanupReservation> Drop for ResolutionFuture<R> {
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        if let (Some(inner), Some(reservation)) = (self.inner.take(), self.reservation.take()) {
            let route_owner = self.route_owner.take();
            reservation.submit(Box::pin(async move {
                let _route_owner = route_owner;
                match inner.await {
                    Ok(Some(element)) => element.release().await.map_err(cleanup_error),
                    // A completed failed attempt already released its group.
                    // Cleanup failures must remain visible to the executor.
                    Err(error @ ResolutionError::Cleanup { .. }) => Err(cleanup_error(error)),
                    Ok(None) | Err(_) => Ok(()),
                }
            }));
        }
    }
}

fn cleanup_error(error: ResolutionError) -> CleanupFailure {
    CleanupFailure::new(error.to_string())
}

#[derive(Clone)]
pub struct ContinuableResolutionService<I, E> {
    inner: Arc<I>,
    executor: E,
}
impl<I, E> ContinuableResolutionService<I, E> {
    pub const fn new(inner: Arc<I>, executor: E) -> Self {
        Self { inner, executor }
    }
}
impl<I, E> Service<ResolutionRequest> for ContinuableResolutionService<I, E>
where
    I: Service<ResolutionRequest, Response = Option<ResolvedElement>, Error = ResolutionError>
        + 'static,
    E: ReservedCleanupExecutor,
{
    type Response = Option<OwnedElement<E::Reservation>>;
    type Error = ResolutionError;
    type Future<'a>
        = ResolutionFuture<E::Reservation>
    where
        Self: 'a;
    fn call(&self, request: ResolutionRequest) -> Self::Future<'_> {
        self.call_retaining(request, None)
    }
}

impl<I, E> ContinuableResolutionService<I, E>
where
    I: Service<ResolutionRequest, Response = Option<ResolvedElement>, Error = ResolutionError>
        + 'static,
    E: ReservedCleanupExecutor,
{
    /// The route owner follows an unpolled/started attempt, returned element,
    /// explicit release and every admitted continuation. Its native resources
    /// remain caller-budgeted; this method only retains one Arc reference.
    pub fn call_retaining(
        &self,
        request: ResolutionRequest,
        route_owner: Option<ResolutionRouteOwner>,
    ) -> ResolutionFuture<E::Reservation> {
        let reservation = match self.executor.try_reserve() {
            Ok(reservation) => reservation,
            Err(_) => {
                return ResolutionFuture {
                    inner: Some(Box::pin(async {
                        Err(ResolutionError::Admission(
                            "cleanup continuation not admitted",
                        ))
                    })),
                    reservation: None,
                    started: false,
                    route_owner,
                };
            }
        };
        let inner = Arc::clone(&self.inner);
        ResolutionFuture {
            inner: Some(Box::pin(async move { inner.call(request).await })),
            reservation: Some(reservation),
            started: false,
            route_owner,
        }
    }
}

#[cfg(test)]
mod tests;
