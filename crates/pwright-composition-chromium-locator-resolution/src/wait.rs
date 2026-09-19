//! Strict attached waiting ends at selection, before any user evaluation.

use std::{
    future::Future,
    future::poll_fn,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::Poll,
    time::Duration,
};

use pwright_backend_cdp_selector_runtime::{
    MAX_SELECTOR_NODES, MAX_SELECTOR_STRING_BYTES, measure_selector,
};
use pwright_cdp::CdpClient;
use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger, ResourceRequest,
    ResourceRequirements, ResourceScope, Service,
};
use pwright_runtime::{DeadlineScheduler, ReservedCleanupExecutor};
use pwright_selector_model::Selector;

use crate::{
    ContinuableResolutionService, OwnedElement, ResolutionError, ResolutionRequest,
    ResolutionRouteOwner, ResolvedElement,
};

// Original selector and one command clone coexist. Native attempts separately
// pre-admit their compiler/runtime source and transport envelope storage.
const SELECTOR_BYTES: u64 =
    (2 * (MAX_SELECTOR_NODES * std::mem::size_of::<Selector>() + MAX_SELECTOR_STRING_BYTES)) as u64;
const WAITER_BYTES: u64 = 1024;
const STATE_BYTES: u64 = 256;

/// Exact utility-world snapshot from the caller's authoritative Frame topology.
/// A source must not resolve a replacement Page or silently fall back to main.
#[derive(Clone)]
pub struct ResolutionRoute {
    pub client: Arc<dyn CdpClient>,
    pub utility_world: Arc<str>,
    pub destruction: Option<Arc<dyn crate::ObjectGroupDestructionProof>>,
}

/// Supplies the current world and a synchronous, authoritative invalidation
/// check. Readiness, route storage and notifications have caller-owned budgets.
/// `current` may wait across world replacement but must fail on Frame closure.
pub trait ResolutionRouteSource: Send + Sync {
    type Current<'a>: Future<Output = Result<ResolutionRoute, ResolutionWaitError>> + Send
    where
        Self: 'a;

    fn current(&self) -> Self::Current<'_>;

    /// False must mean this exact session/world was retired, not merely that a
    /// request failed. Closed/crashed Frames return a terminal error instead.
    fn is_current(&self, route: &ResolutionRoute) -> Result<bool, ResolutionWaitError>;

    /// Retain the exact route's caller-budgeted storage through cancellation
    /// and group release, not just through this wait. Stateless routes may
    /// return None; budgeted snapshots must return their owning reference.
    fn retain_route(
        &self,
        _route: &ResolutionRoute,
    ) -> Result<Option<ResolutionRouteOwner>, ResolutionWaitError> {
        Ok(None)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ResolutionWaitStop {
    #[error("Locator resolution timed out")]
    Timeout,
    #[error("Locator target closed")]
    TargetClosed,
    #[error("Locator target crashed")]
    BrowserCrashed,
    #[error("Locator resolution cancelled")]
    Cancelled,
}

#[derive(Debug, thiserror::Error)]
pub enum ResolutionWaitError {
    #[error("Locator route failed: {0}")]
    Route(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)]
    Resolution(#[from] ResolutionError),
    #[error(transparent)]
    Stopped(#[from] ResolutionWaitStop),
}

/// Capacity and retry pacing are explicit; the enclosing command supplies its
/// one original deadline/cancellation future, including any queue delay.
#[derive(Clone, Copy, Debug)]
pub struct ResolutionWaitProfile {
    scope: ResourceScope,
    capacity: usize,
    retry_delays: [Duration; 6],
    requirements: ResourceRequirements,
}

impl ResolutionWaitProfile {
    pub fn new(
        scope: ResourceScope,
        capacity: usize,
        retry_delays: [Duration; 6],
    ) -> Result<Self, ResolutionError> {
        if capacity == 0 || retry_delays[1..].iter().any(Duration::is_zero) {
            return Err(ResolutionError::Setup(
                "wait capacity and all retry delays after the first must be positive",
            ));
        }
        let count = u64::try_from(capacity)
            .map_err(|_| ResolutionError::Setup("wait capacity overflow"))?;
        let peak = |bytes: u64| {
            bytes
                .checked_mul(count)
                .ok_or(ResolutionError::Setup("wait resource ceiling overflow"))
        };
        Ok(Self {
            scope,
            capacity,
            retry_delays,
            requirements: ResourceRequirements::none()
                .with_class(
                    ResourceClass::SelectorBytes,
                    peak(SELECTOR_BYTES)?,
                    peak(SELECTOR_BYTES)?,
                )
                .with_class(
                    ResourceClass::BackendStateBytes,
                    peak(WAITER_BYTES)?
                        .checked_add(STATE_BYTES)
                        .ok_or(ResolutionError::Setup("wait state ceiling overflow"))?,
                    peak(WAITER_BYTES)?
                        .checked_add(STATE_BYTES)
                        .ok_or(ResolutionError::Setup("wait state ceiling overflow"))?,
                )
                .with_metadata(
                    1,
                    capacity
                        .checked_add(1)
                        .ok_or(ResolutionError::Setup("wait lease ceiling overflow"))?,
                ),
        })
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

#[derive(Clone)]
pub struct StrictResolutionWaiter {
    profile: ResolutionWaitProfile,
    ledger: ResourceLedger,
    slots: Arc<WaitSlots>,
}

struct WaitSlots {
    active: AtomicUsize,
    _state: ResourceLease,
}
struct WaitSlot(Arc<WaitSlots>);
impl Drop for WaitSlot {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::AcqRel);
    }
}

impl StrictResolutionWaiter {
    pub fn new(
        profile: ResolutionWaitProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ResolutionError> {
        let ledger = resources.claim(profile.requirements())?;
        let state = ledger.reserve(profile.scope, ResourceClass::BackendStateBytes, STATE_BYTES)?;
        Ok(Self {
            profile,
            ledger,
            slots: Arc::new(WaitSlots {
                active: AtomicUsize::new(0),
                _state: state,
            }),
        })
    }

    fn admit(&self) -> Result<(WaitSlot, ResourceLease), ResolutionError> {
        self.slots
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < self.profile.capacity).then(|| count + 1)
            })
            .map_err(|_| ResolutionError::Admission("Locator wait capacity exhausted"))?;
        let slot = WaitSlot(Arc::clone(&self.slots));
        let lease = self.ledger.reserve_many(
            self.profile.scope,
            &[
                ResourceRequest::new(ResourceClass::SelectorBytes, SELECTOR_BYTES),
                ResourceRequest::new(ResourceClass::BackendStateBytes, WAITER_BYTES),
            ],
        )?;
        Ok((slot, lease))
    }

    /// The stop future is polled before work on every wake and is never reset
    /// between routes/attempts. Dropping this future stops further polling;
    /// a dispatched native attempt transfers into its reserved cleanup slot.
    ///
    /// Returning an owner ends this wait. The caller can pass a borrowed pinned
    /// stop future to retain the same deadline across main-world adoption; user
    /// evaluation starts only after adoption and is outside that deadline.
    /// Route, pacing and stop-future storage belongs to their injected owners,
    /// not to this waiter's fixed state allowance.
    pub fn wait<'a, I, E, S, D, Stop>(
        &'a self,
        selector: Selector,
        attempts: &'a ContinuableResolutionService<I, E>,
        routes: &'a S,
        delays: &'a D,
        stop: Stop,
    ) -> impl Future<Output = Result<OwnedElement<E::Reservation>, ResolutionWaitError>> + Send + 'a
    where
        I: Service<ResolutionRequest, Response = Option<ResolvedElement>, Error = ResolutionError>
            + 'static,
        E: ReservedCleanupExecutor,
        S: ResolutionRouteSource,
        D: DeadlineScheduler,
        Stop: Future<Output = ResolutionWaitStop> + Send + 'a,
    {
        let admitted = measure_selector(&selector)
            .map_err(ResolutionError::Selector)
            .and_then(|storage| {
                if storage.string_capacity() > MAX_SELECTOR_STRING_BYTES {
                    return Err(ResolutionError::Admission(
                        "retained selector strings exceed the wait bound",
                    ));
                }
                self.admit()
            })
            .map(|admission| (selector, admission));
        async move {
            let (selector, _admission) = admitted?;
            let operation = async {
                let mut retry: Option<usize> = None;
                loop {
                    if let Some(index) = retry {
                        let delay = self.profile.retry_delays[index];
                        if !delay.is_zero() {
                            delays
                                .sleep(delay)
                                .ok_or(ResolutionError::Setup(
                                    "Locator retry scheduler cannot provide a delay",
                                ))?
                                .await;
                        }
                        retry = Some((index + 1).min(self.profile.retry_delays.len() - 1));
                    } else {
                        retry = Some(0);
                    }
                    let route = routes.current().await?;
                    if !routes.is_current(&route)? {
                        continue;
                    }
                    let result = attempts
                        .call_retaining(
                            ResolutionRequest {
                                client: Arc::clone(&route.client),
                                utility_world: Arc::clone(&route.utility_world),
                                selector: selector.clone(),
                                destruction: route.destruction.clone(),
                            },
                            routes.retain_route(&route)?,
                        )
                        .await;
                    match result {
                        Ok(Some(element)) => {
                            // A stale successful response still owns a group.
                            // Release before retry, never replay after return.
                            if routes.is_current(&route)? {
                                return Ok(element);
                            }
                            element.release().await?;
                        }
                        Ok(None) => {
                            routes.is_current(&route)?;
                        }
                        Err(error @ ResolutionError::Backend(_)) => {
                            if routes.is_current(&route)? {
                                return Err(error.into());
                            }
                        }
                        // Strict/selector/protocol/resource/cleanup errors do
                        // not become a timeout, even if navigation also occurs.
                        Err(error) => return Err(error.into()),
                    }
                }
            };
            let mut operation = std::pin::pin!(operation);
            let mut stop = std::pin::pin!(stop);
            poll_fn(move |cx| {
                if let Poll::Ready(reason) = stop.as_mut().poll(cx) {
                    return Poll::Ready(Err(reason.into()));
                }
                operation.as_mut().poll(cx)
            })
            .await
        }
    }
}

#[cfg(test)]
mod tests;
