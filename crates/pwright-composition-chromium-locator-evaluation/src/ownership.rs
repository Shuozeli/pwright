use super::*;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

type Adoption<R> = Pin<Box<dyn Future<Output = Outcome<ElementData<R>>> + Send>>;
type Evaluation = Pin<Box<dyn Future<Output = Outcome<EvaluationValue>> + Send>>;

pub struct AdoptionFuture<R: CleanupReservation, C: CleanupReservation> {
    inner: Option<Adoption<R>>,
    reservation: Option<C>,
    started: bool,
}
impl<R: CleanupReservation, C: CleanupReservation> AdoptionFuture<R, C> {
    pub(super) fn new(inner: Adoption<R>, reservation: Option<C>) -> Self {
        Self {
            inner: Some(inner),
            reservation,
            started: false,
        }
    }
}
impl<R: CleanupReservation, C: CleanupReservation> Unpin for AdoptionFuture<R, C> {}
impl<R: CleanupReservation, C: CleanupReservation> Future for AdoptionFuture<R, C> {
    type Output = Outcome<MainWorldElement<R, C>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let result = self
            .inner
            .as_mut()
            .expect("adoption polled after completion")
            .as_mut()
            .poll(cx);
        match result {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                self.inner = None;
                let reservation = self.reservation.take();
                Poll::Ready(result.map(|data| MainWorldElement {
                    data: Some(data),
                    reservation,
                    retirement: None,
                }))
            }
        }
    }
}
impl<R: CleanupReservation, C: CleanupReservation> Drop for AdoptionFuture<R, C> {
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        if let (Some(inner), Some(reservation)) = (self.inner.take(), self.reservation.take()) {
            reservation.submit(Box::pin(async move {
                match inner.await {
                    Ok(data) => data.finish(Ok(())).await.map_err(cleanup_failure),
                    Err(error) => cleanup_outcome(error),
                }
            }));
        }
    }
}

/// One selected and adopted element, consumed by exactly one evaluation.
/// Drop disposes the group without evaluating anything; no portable Handle ID.
#[must_use = "evaluate once, release, or drop to clean up the temporary group"]
pub struct MainWorldElement<R: CleanupReservation, C: CleanupReservation> {
    pub(super) data: Option<ElementData<R>>,
    reservation: Option<C>,
    retirement: Option<Box<dyn NativeEvaluationRetirement>>,
}
impl<R: CleanupReservation, C: CleanupReservation> MainWorldElement<R, C> {
    pub fn evaluate(mut self, params: &EvaluateParams) -> ElementEvaluationFuture<R, C> {
        let retirement = self.retirement.take();
        self.evaluation(params, retirement)
    }
    pub(crate) fn with_retirement(
        mut self,
        retirement: Option<Box<dyn NativeEvaluationRetirement>>,
    ) -> Self {
        self.retirement = retirement;
        self
    }
    /// Select a caller-budgeted, exact native retirement receipt. This is not
    /// an ordinary cancellation token; see `NativeEvaluationRetirement`.
    pub fn evaluate_with_retirement(
        self,
        params: &EvaluateParams,
        retirement: Box<dyn NativeEvaluationRetirement>,
    ) -> ElementEvaluationFuture<R, C> {
        self.evaluation(params, Some(retirement))
    }
    fn evaluation(
        mut self,
        params: &EvaluateParams,
        retirement: Option<Box<dyn NativeEvaluationRetirement>>,
    ) -> ElementEvaluationFuture<R, C> {
        let data = self.data.take().expect("live main-world element");
        let params = data.params(params);
        ElementEvaluationFuture {
            prepared: Some((data, params)),
            inner: None,
            retirement,
            reservation: self.reservation.take(),
        }
    }
    pub async fn release(mut self) -> Outcome<()> {
        let data = self.data.take().expect("live main-world element");
        // A release-only future uses the same continuation wrapper, but cannot
        // dispatch user code even when it is first polled during cleanup.
        let future = Box::pin(async move { data.finish(Ok(EvaluationValue::Undefined)).await });
        ElementEvaluationFuture::<R, C> {
            prepared: None,
            inner: Some(future),
            retirement: None,
            reservation: self.reservation.take(),
        }
        .await
        .map(|_| ())
    }
}
impl<R: CleanupReservation, C: CleanupReservation> Drop for MainWorldElement<R, C> {
    fn drop(&mut self) {
        if let (Some(data), Some(reservation)) = (self.data.take(), self.reservation.take()) {
            reservation.submit(Box::pin(async move {
                data.finish(Ok(())).await.map_err(cleanup_failure)
            }));
        }
    }
}

pub struct ElementEvaluationFuture<R: CleanupReservation, C: CleanupReservation> {
    prepared: Option<(ElementData<R>, Outcome<EvaluateHandleParams>)>,
    inner: Option<Evaluation>,
    retirement: Option<Box<dyn NativeEvaluationRetirement>>,
    reservation: Option<C>,
}
impl<R: CleanupReservation, C: CleanupReservation> Unpin for ElementEvaluationFuture<R, C> {}
impl<R: CleanupReservation, C: CleanupReservation> Future for ElementEvaluationFuture<R, C> {
    type Output = Outcome<EvaluationValue>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some((data, params)) = self.prepared.take() {
            let retirement = self.retirement.take();
            self.inner = Some(Box::pin(data.evaluate(params, retirement)));
        }
        let result = self
            .inner
            .as_mut()
            .expect("evaluation polled after completion")
            .as_mut()
            .poll(cx);
        if result.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        result
    }
}
impl<R: CleanupReservation, C: CleanupReservation> Drop for ElementEvaluationFuture<R, C> {
    fn drop(&mut self) {
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        if let Some((data, _)) = self.prepared.take() {
            reservation.submit(Box::pin(async move {
                data.finish(Ok(())).await.map_err(cleanup_failure)
            }));
        } else if let Some(inner) = self.inner.take() {
            reservation.submit(Box::pin(async move {
                match inner.await {
                    Ok(_) => Ok(()),
                    Err(error) => cleanup_outcome(error),
                }
            }));
        }
    }
}
