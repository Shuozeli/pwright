//! Exact retained nodes reuse the same real group owner and continuation.

use super::*;
pub use pwright_backend_cdp_locator_resolution::RetainedElementRequest;

/// A retained-node attempt cannot produce a selector-style absent match.
/// Dropping this wrapper delegates to the ordinary pre-admitted continuation,
/// including cleanup failure reporting and route-owner retention.
pub struct RetainedResolutionFuture<R: CleanupReservation>(ResolutionFuture<R>);

impl<R: CleanupReservation> Unpin for RetainedResolutionFuture<R> {}

impl<R: CleanupReservation> Future for RetainedResolutionFuture<R> {
    type Output = Result<OwnedElement<R>, ResolutionError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx).map(|outcome| {
            outcome.map(|element| {
                element.expect("retained acquisition wraps every successful element in Some")
            })
        })
    }
}

impl<I, E> Service<RetainedElementRequest> for ContinuableResolutionService<I, E>
where
    I: Service<RetainedElementRequest, Response = ResolvedElement, Error = ResolutionError>
        + 'static,
    E: ReservedCleanupExecutor,
{
    type Response = OwnedElement<E::Reservation>;
    type Error = ResolutionError;
    type Future<'a>
        = RetainedResolutionFuture<E::Reservation>
    where
        Self: 'a;

    fn call(&self, request: RetainedElementRequest) -> Self::Future<'_> {
        self.call_retained_retaining(request, None)
    }
}

impl<I, E> ContinuableResolutionService<I, E>
where
    I: Service<RetainedElementRequest, Response = ResolvedElement, Error = ResolutionError>
        + 'static,
    E: ReservedCleanupExecutor,
{
    /// Reserve cleanup before any native admission or acquisition. Retain the
    /// caller-budgeted route/source owner through late settlement and release.
    /// An unpolled attempt performs no browser I/O.
    pub fn call_retained_retaining(
        &self,
        request: RetainedElementRequest,
        route_owner: Option<ResolutionRouteOwner>,
    ) -> RetainedResolutionFuture<E::Reservation> {
        let reservation = match self.executor.try_reserve() {
            Ok(reservation) => reservation,
            Err(_) => {
                return RetainedResolutionFuture(ResolutionFuture {
                    inner: Some(Box::pin(async {
                        Err(ResolutionError::Admission(
                            "cleanup continuation not admitted",
                        ))
                    })),
                    reservation: None,
                    started: false,
                    route_owner,
                });
            }
        };
        let inner = self.inner.clone();
        RetainedResolutionFuture(ResolutionFuture {
            inner: Some(Box::pin(async move { inner.call(request).await.map(Some) })),
            reservation: Some(reservation),
            started: false,
            route_owner,
        })
    }
}

#[cfg(test)]
mod tests;
