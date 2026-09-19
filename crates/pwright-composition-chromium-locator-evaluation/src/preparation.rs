//! Strict selection and main-world adoption under one caller-owned stop future.
//! Paired topology, pacing, cleanup execution and their budgets are injected.

use super::*;
use pwright_backend_cdp_selector_runtime::{
    MAX_SELECTOR_NODES, MAX_SELECTOR_STRING_BYTES, measure_selector,
};
use pwright_composition_chromium_locator_resolution::{
    ContinuableResolutionService, ResolutionRequest, ResolutionRoute, ResolutionRouteOwner,
    ResolutionRouteSource, ResolutionWaitError, ResolutionWaitStop, ResolvedElement,
    StrictResolutionWaiter,
};
use pwright_runtime::DeadlineScheduler;
use pwright_selector_model::Selector;
use std::{future::Future, future::poll_fn, sync::Mutex, task::Poll, time::Duration};

// Checked against the concrete generated future BEFORE boxing it. Includes
// the outer result wrapper and 1 KiB for the private boxed route adapter.
const PREPARATION_BYTES: u64 = 8192;
const SELECTOR_BYTES: u64 =
    (MAX_SELECTOR_NODES * std::mem::size_of::<Selector>() + MAX_SELECTOR_STRING_BYTES) as u64;

/// One caller-budgeted coherent pair. Both route methods must use the same
/// native client and distinct worlds of one exact Frame. `is_current` must
/// check BOTH worlds and remain false after observed retirement. Retirement
/// alone never proves native object-group destruction.
pub trait PairedWorld: Send + Sync + 'static {
    fn utility_route(&self) -> ResolutionRoute;
    fn main_route(&self) -> MainWorldRoute;
    fn is_current(&self) -> Result<bool, ResolutionWaitError>;
    /// Optional exact-session receipt, budgeted and retained by this pair.
    /// One successful preparation consumes one notification subscriber.
    fn native_evaluation_retirement(&self) -> Option<Box<dyn NativeEvaluationRetirement>> {
        None
    }
}

/// The returned owning reference survives through selection, adoption,
/// evaluation AND native cleanup, including caller cancellation. Its storage,
/// native identifiers and notification future are budgeted by this source.
pub trait PairedWorldSource: Send + Sync {
    type Pair: PairedWorld;
    type Current<'a>: Future<Output = Result<Arc<Self::Pair>, ResolutionWaitError>> + Send
    where
        Self: 'a;
    fn current(&self) -> Self::Current<'_>;
}

#[derive(Debug, thiserror::Error)]
pub enum PreparationError {
    #[error(transparent)]
    Waiting(#[from] ResolutionWaitError),
    #[error(transparent)]
    Resolution(#[from] ResolutionError),
    #[error(transparent)]
    Adoption(#[from] ElementEvaluationError),
}

#[derive(Clone, Copy, Debug)]
pub struct PreparationProfile {
    scope: ResourceScope,
    capacity: usize,
    retry_delay: Duration,
    requirements: ResourceRequirements,
}
impl PreparationProfile {
    pub fn new(scope: ResourceScope, capacity: usize, retry_delay: Duration) -> Outcome<Self> {
        if capacity == 0 || retry_delay.is_zero() {
            return Err(ElementEvaluationError::Setup(
                "preparation capacity and retry delay must be positive",
            ));
        }
        let count = u64::try_from(capacity)
            .map_err(|_| ElementEvaluationError::Setup("preparation capacity overflow"))?;
        let peak = |bytes: u64| {
            bytes
                .checked_mul(count)
                .ok_or(ElementEvaluationError::Setup(
                    "preparation resource ceiling overflow",
                ))
        };
        let state = peak(PREPARATION_BYTES)?.checked_add(STATE_BYTES).ok_or(
            ElementEvaluationError::Setup("preparation state ceiling overflow"),
        )?;
        Ok(Self {
            scope,
            capacity,
            retry_delay,
            requirements: ResourceRequirements::none()
                .with_class(
                    ResourceClass::SelectorBytes,
                    peak(SELECTOR_BYTES)?,
                    peak(SELECTOR_BYTES)?,
                )
                .with_class(ResourceClass::BackendStateBytes, state, state)
                .with_metadata(
                    1,
                    capacity
                        .checked_add(1)
                        .ok_or(ElementEvaluationError::Setup(
                            "preparation lease ceiling overflow",
                        ))?,
                ),
        })
    }
    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

struct Slots {
    active: AtomicUsize,
    _lease: ResourceLease,
}
struct Permit {
    slots: Arc<Slots>,
    _lease: ResourceLease,
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.slots.active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Clone)]
pub struct LocatorElementPreparationService {
    profile: PreparationProfile,
    ledger: ResourceLedger,
    slots: Arc<Slots>,
}

/// Existing leaf services and deployment policies; no new driver or executor.
pub struct PreparationInputs<'a, I, E, S, D> {
    pub attempts: &'a ContinuableResolutionService<I, E>,
    pub waiter: &'a StrictResolutionWaiter,
    pub adoption: &'a LocatorElementEvaluationService<E>,
    pub worlds: &'a S,
    pub delays: &'a D,
}

type PreparedOutcome<R> = Result<MainWorldElement<R, R>, PreparationError>;
type PreparedFuture<'a, R> =
    std::pin::Pin<Box<dyn Future<Output = PreparedOutcome<R>> + Send + 'a>>;

/// No allocation on rejection. Dropping admitted work drops only its exact
/// inner operation; native continuation owners retain routes until cleanup.
pub struct PreparationFuture<'a, R: CleanupReservation> {
    inner: Option<PreparedFuture<'a, R>>,
    error: Option<PreparationError>,
}
impl<R: CleanupReservation> Unpin for PreparationFuture<'_, R> {}
impl<R: CleanupReservation> Future for PreparationFuture<'_, R> {
    type Output = PreparedOutcome<R>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        if let Some(error) = self.error.take() {
            return Poll::Ready(Err(error));
        }
        let result = self
            .inner
            .as_mut()
            .expect("preparation polled after completion")
            .as_mut()
            .poll(cx);
        if result.is_ready() {
            self.inner = None;
        }
        result
    }
}

impl LocatorElementPreparationService {
    pub fn new(profile: PreparationProfile, resources: &ProductionResourceLedger) -> Outcome<Self> {
        let ledger = resources.claim(profile.requirements())?;
        let lease = ledger.reserve(profile.scope, ResourceClass::BackendStateBytes, STATE_BYTES)?;
        Ok(Self {
            profile,
            ledger,
            slots: Arc::new(Slots {
                active: AtomicUsize::new(0),
                _lease: lease,
            }),
        })
    }
    fn admit(&self, selector: &Selector) -> Result<Permit, PreparationError> {
        let measured = measure_selector(selector).map_err(ResolutionError::Selector)?;
        if measured.string_capacity() > MAX_SELECTOR_STRING_BYTES {
            return Err(ResolutionError::Admission(
                "retained preparation selector exceeds its bound",
            )
            .into());
        }
        self.slots
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < self.profile.capacity).then(|| n + 1)
            })
            .map_err(|_| ElementEvaluationError::Admission("preparation capacity exhausted"))?;
        let lease = self.ledger.reserve_many(
            self.profile.scope,
            &[
                ResourceRequest::new(ResourceClass::SelectorBytes, SELECTOR_BYTES),
                ResourceRequest::new(ResourceClass::BackendStateBytes, PREPARATION_BYTES),
            ],
        );
        match lease {
            Ok(lease) => Ok(Permit {
                slots: self.slots.clone(),
                _lease: lease,
            }),
            Err(error) => {
                self.slots.active.fetch_sub(1, Ordering::AcqRel);
                Err(ElementEvaluationError::Resource(error).into())
            }
        }
    }

    /// Synchronous admission precedes all copies and native work. The one stop
    /// future wins every poll throughout waiting, adoption and stale cleanup.
    /// A successful return ends the resolution deadline: user evaluation is a
    /// separate, single-use phase and is never replayed by this service.
    pub fn prepare<'a, I, E, S, D, Stop>(
        &'a self,
        selector: Selector,
        inputs: &'a PreparationInputs<'a, I, E, S, D>,
        stop: Stop,
    ) -> PreparationFuture<'a, E::Reservation>
    where
        I: Service<ResolutionRequest, Response = Option<ResolvedElement>, Error = ResolutionError>
            + 'static,
        E: ReservedCleanupExecutor,
        S: PairedWorldSource,
        D: DeadlineScheduler,
        Stop: Future<Output = ResolutionWaitStop> + Send + 'a,
    {
        let permit = match self.admit(&selector) {
            Ok(permit) => permit,
            Err(error) => {
                return PreparationFuture {
                    inner: None,
                    error: Some(error),
                };
            }
        };
        let work = async move {
            let _permit = permit;
            let routes = PairRoutes {
                source: inputs.worlds,
                pair: Mutex::new(None),
            };
            let operation = async {
                loop {
                    let selected = inputs
                        .waiter
                        .wait(
                            selector.clone(),
                            inputs.attempts,
                            &routes,
                            inputs.delays,
                            std::future::pending(),
                        )
                        .await?;
                    let pair = routes.take()?;
                    let utility = pair.utility_route();
                    if !Arc::ptr_eq(&utility.client, &selected.element().client())
                        || utility.utility_world.as_ref() != selected.element().utility_world()
                    {
                        selected.release().await?;
                        return Err(ResolutionError::Setup(
                            "selected element does not belong to its paired route",
                        )
                        .into());
                    }
                    if pair.is_current()? {
                        let adopted = inputs.adoption.adopt(selected, pair.main_route()).await;
                        match adopted {
                            Ok(prepared) => {
                                if pair.is_current()? {
                                    return Ok(prepared
                                        .with_retirement(pair.native_evaluation_retirement()));
                                }
                                prepared.release().await?;
                            }
                            Err(ElementEvaluationError::Backend(error))
                                if error.kind() == pwright_core::FailureKind::Backend
                                    && !pair.is_current()? => {}
                            Err(error) => return Err(error.into()),
                        }
                    } else {
                        selected.release().await?;
                    }
                    // Only pre-user-code work retries, after acknowledged
                    // cleanup. The stop future and its deadline are not reset.
                    drop(pair);
                    inputs
                        .delays
                        .sleep(self.profile.retry_delay)
                        .ok_or(ElementEvaluationError::Setup(
                            "preparation retry delay unavailable",
                        ))?
                        .await;
                }
            };
            let mut operation = std::pin::pin!(operation);
            let mut stop = std::pin::pin!(stop);
            poll_fn(move |cx| {
                if let Poll::Ready(reason) = stop.as_mut().poll(cx) {
                    return Poll::Ready(Err(ResolutionWaitError::Stopped(reason).into()));
                }
                operation.as_mut().poll(cx)
            })
            .await
        };
        let storage = std::mem::size_of_val(&work)
            .checked_add(std::mem::size_of::<PreparationFuture<'_, E::Reservation>>())
            .and_then(|bytes| bytes.checked_add(1024));
        if storage.is_none_or(|bytes| bytes as u64 > PREPARATION_BYTES) {
            return PreparationFuture {
                inner: None,
                error: Some(
                    ElementEvaluationError::Admission(
                        "concrete preparation future exceeds its declared storage bound",
                    )
                    .into(),
                ),
            };
        }
        PreparationFuture {
            inner: Some(Box::pin(work)),
            error: None,
        }
    }
}

struct PairRoutes<'a, S: PairedWorldSource> {
    source: &'a S,
    pair: Mutex<Option<Arc<S::Pair>>>,
}
impl<S: PairedWorldSource> PairRoutes<'_, S> {
    fn take(&self) -> Result<Arc<S::Pair>, ResolutionWaitError> {
        self.pair
            .lock()
            .map_err(|_| ResolutionError::Setup("paired route lock poisoned"))?
            .take()
            .ok_or_else(|| ResolutionError::Setup("selected paired route missing").into())
    }
    fn retained(&self, route: &ResolutionRoute) -> Result<Arc<S::Pair>, ResolutionWaitError> {
        let guard = self
            .pair
            .lock()
            .map_err(|_| ResolutionError::Setup("paired route lock poisoned"))?;
        let pair = guard
            .as_ref()
            .ok_or(ResolutionError::Setup("paired route missing"))?;
        let utility = pair.utility_route();
        if !Arc::ptr_eq(&route.client, &utility.client)
            || route.utility_world != utility.utility_world
        {
            return Err(
                ResolutionError::Setup("paired route identity changed unexpectedly").into(),
            );
        }
        Ok(pair.clone())
    }
}
impl<S: PairedWorldSource> ResolutionRouteSource for PairRoutes<'_, S> {
    type Current<'a>
        = std::pin::Pin<
        Box<dyn Future<Output = Result<ResolutionRoute, ResolutionWaitError>> + Send + 'a>,
    >
    where
        Self: 'a;
    fn current(&self) -> Self::Current<'_> {
        let work = async {
            self.pair
                .lock()
                .map_err(|_| ResolutionError::Setup("paired route lock poisoned"))?
                .take();
            let pair = self.source.current().await?;
            let route = pair.utility_route();
            let main = pair.main_route();
            if !Arc::ptr_eq(&route.client, &main.client)
                || main.execution_context_id <= 0
                || route.utility_world.is_empty()
                || main.unique_context_id.is_empty()
                || route.utility_world == main.unique_context_id
            {
                return Err(
                    ResolutionError::Setup("invalid paired main and utility routes").into(),
                );
            }
            *self
                .pair
                .lock()
                .map_err(|_| ResolutionError::Setup("paired route lock poisoned"))? = Some(pair);
            Ok(route)
        };
        if std::mem::size_of_val(&work) > 1024 {
            return Box::pin(std::future::ready(Err(ResolutionError::Setup(
                "concrete paired route future exceeds its declared storage bound",
            )
            .into())));
        }
        Box::pin(work)
    }
    fn is_current(&self, route: &ResolutionRoute) -> Result<bool, ResolutionWaitError> {
        self.retained(route)?.is_current()
    }
    fn retain_route(
        &self,
        route: &ResolutionRoute,
    ) -> Result<Option<ResolutionRouteOwner>, ResolutionWaitError> {
        Ok(Some(self.retained(route)?))
    }
}
