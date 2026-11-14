//! Explicit adoption and single-use evaluation of a previously selected node.
//! No selector retry, timer, Node driver, portable Handle or browser lifecycle.

mod ownership;
pub mod preparation;
mod retirement;
pub use ownership::{AdoptionFuture, ElementEvaluationFuture, MainWorldElement};
pub use retirement::NativeEvaluationRetirement;

use pwright_backend_cdp_evaluation::{
    CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpEvaluationService,
    CdpEvaluationServiceFactory,
};
use pwright_backend_cdp_handle_evaluation::{
    CdpAdoptedElementHandle, CdpElementHandleAdoptionRequest, CdpElementHandleAdoptionService,
    CdpHandleEvaluationLimits, CdpHandleEvaluationRequest, CdpHandleValueEvaluationService,
};
use pwright_cap_evaluation::{EvaluateParams, EvaluationMode, EvaluationValue};
use pwright_cap_handle_evaluation::{
    EvaluateHandleParams, HandleEvaluationErrorDetails, HandleEvaluationValue,
};
use pwright_cdp::CdpClient;
use pwright_composition_chromium_locator_resolution::{OwnedElement, ResolutionError};
use pwright_core::{
    OperationError, ProductionResourceError, ProductionResourceLedger, ResourceClass,
    ResourceError, ResourceLease, ResourceLedger, ResourceRequest, ResourceRequirements,
    ResourceScope, Service,
};
use pwright_runtime::{CleanupFailure, CleanupReservation, ReservedCleanupExecutor};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

const NATIVE_BYTES: usize = 1024;
const STATE_BYTES: u64 = 256;
type Outcome<T> = Result<T, ElementEvaluationError>;

#[derive(Debug, thiserror::Error)]
pub enum ElementEvaluationError {
    #[error("invalid Locator element evaluation setup: {0}")]
    Setup(&'static str),
    #[error("Locator element evaluation admission rejected: {0}")]
    Admission(&'static str),
    #[error("native Locator evaluation session was authoritatively retired")]
    NativeRetired,
    #[error(transparent)]
    Production(#[from] ProductionResourceError),
    #[error(transparent)]
    Resource(#[from] ResourceError),
    #[error("Locator element evaluation failed: {0}")]
    Backend(#[from] OperationError<HandleEvaluationErrorDetails>),
    #[error("Locator temporary group cleanup failed: {cleanup}; primary: {primary:?}")]
    Cleanup {
        primary: Option<Box<Self>>,
        cleanup: ResolutionError,
    },
}

/// Exact main-world route from the composing Frame topology. Numeric ID is
/// used only for adoption; user evaluation targets the stable unique ID.
pub struct MainWorldRoute {
    pub client: Arc<dyn CdpClient>,
    pub execution_context_id: i64,
    pub unique_context_id: Arc<str>,
}

#[derive(Clone, Copy, Debug)]
pub struct ElementEvaluationProfile {
    scope: ResourceScope,
    capacity: usize,
    limits: CdpEvaluationLimits,
    owner_bytes: u64,
    owner_requirements: ResourceRequirements,
    evaluation: CdpEvaluationProductionProfile,
}
impl ElementEvaluationProfile {
    pub fn new(
        scope: ResourceScope,
        capacity: usize,
        limits: CdpEvaluationLimits,
    ) -> Outcome<Self> {
        if capacity == 0
            || limits.source_bytes == 0
            || limits.generated_script_bytes == 0
            || limits.error_bytes == 0
        {
            return Err(ElementEvaluationError::Setup(
                "capacity and evaluation limits must be positive",
            ));
        }
        let tree_bytes = limits
            .value
            .max_nodes()
            .checked_mul(
                std::mem::size_of::<EvaluationValue>()
                    + std::mem::size_of::<HandleEvaluationValue>(),
            )
            .and_then(|bytes| bytes.checked_mul(4))
            .ok_or(ElementEvaluationError::Setup("value tree ceiling overflow"))?;
        let owner_bytes = limits
            .maximum_retained_bytes_per_operation()
            .checked_add(
                u64::try_from(tree_bytes)
                    .map_err(|_| ElementEvaluationError::Setup("value tree ceiling overflow"))?,
            )
            .and_then(|bytes| bytes.checked_add(8192))
            .ok_or(ElementEvaluationError::Setup("owner ceiling overflow"))?;
        let count = u64::try_from(capacity)
            .map_err(|_| ElementEvaluationError::Setup("capacity overflow"))?;
        let bytes = owner_bytes
            .checked_mul(count)
            .and_then(|bytes| bytes.checked_add(STATE_BYTES))
            .ok_or(ElementEvaluationError::Setup("resource ceiling overflow"))?;
        let evaluation = CdpEvaluationProductionProfile::new(scope, limits, capacity);
        // The composition combines simultaneous owner and evaluator peaks;
        // reject an unrepresentable sum instead of relying on saturation.
        limits
            .maximum_retained_bytes_per_operation()
            .checked_mul(count)
            .and_then(|evaluation_bytes| bytes.checked_add(evaluation_bytes))
            .ok_or(ElementEvaluationError::Setup(
                "combined resource ceiling overflow",
            ))?;
        Ok(Self {
            scope,
            capacity,
            limits,
            owner_bytes,
            evaluation,
            owner_requirements: ResourceRequirements::none()
                .with_class(ResourceClass::RemoteHandle, count, count)
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(
                    1,
                    capacity
                        .checked_add(1)
                        .ok_or(ElementEvaluationError::Setup("lease ceiling overflow"))?,
                ),
        })
    }

    pub fn with_user_gesture(mut self, enabled: bool) -> Self {
        self.evaluation = self.evaluation.with_user_gesture(enabled);
        self
    }

    pub const fn uses_user_gesture(&self) -> bool {
        self.evaluation.uses_user_gesture()
    }
    pub fn requirements(self) -> ResourceRequirements {
        self.owner_requirements
            .combine_concurrent(self.evaluation.requirements())
    }
}

struct State {
    active: AtomicUsize,
    healthy: AtomicBool,
    _lease: ResourceLease,
}
struct OwnerLease {
    state: Arc<State>,
    armed: bool,
    _lease: ResourceLease,
}
impl OwnerLease {
    fn disarm(&mut self) {
        self.armed = false;
    }
}
impl Drop for OwnerLease {
    fn drop(&mut self) {
        if self.armed {
            self.state.healthy.store(false, Ordering::Release);
        }
        self.state.active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Clone)]
pub struct LocatorElementEvaluationService<E> {
    profile: ElementEvaluationProfile,
    ledger: ResourceLedger,
    state: Arc<State>,
    evaluation: CdpEvaluationServiceFactory,
    cleanup: E,
}
impl<E: ReservedCleanupExecutor> LocatorElementEvaluationService<E> {
    pub fn new(
        profile: ElementEvaluationProfile,
        resources: &ProductionResourceLedger,
        cleanup: E,
    ) -> Outcome<Self> {
        let ledger = resources.claim(profile.owner_requirements)?;
        let state = ledger.reserve(profile.scope, ResourceClass::BackendStateBytes, STATE_BYTES)?;
        let evaluation = CdpEvaluationServiceFactory::new_with_production_resources(
            profile.evaluation,
            resources,
        )?;
        Ok(Self {
            profile,
            ledger,
            evaluation,
            cleanup,
            state: Arc::new(State {
                active: AtomicUsize::new(0),
                healthy: AtomicBool::new(true),
                _lease: state,
            }),
        })
    }
    pub fn is_cleanup_healthy(&self) -> bool {
        self.state.healthy.load(Ordering::Acquire)
    }

    fn reserve(&self) -> Outcome<OwnerLease> {
        if !self.is_cleanup_healthy() {
            return Err(ElementEvaluationError::Admission(
                "native group cleanup is not proven",
            ));
        }
        self.state
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < self.profile.capacity).then(|| count + 1)
            })
            .map_err(|_| {
                ElementEvaluationError::Admission("temporary element capacity exhausted")
            })?;
        let lease = self.ledger.reserve_many(
            self.profile.scope,
            &[
                ResourceRequest::new(ResourceClass::RemoteHandle, 1),
                ResourceRequest::new(ResourceClass::BackendStateBytes, self.profile.owner_bytes),
            ],
        );
        match lease {
            Ok(lease) => Ok(OwnerLease {
                state: self.state.clone(),
                armed: false,
                _lease: lease,
            }),
            Err(error) => {
                self.state.active.fetch_sub(1, Ordering::AcqRel);
                Err(error.into())
            }
        }
    }

    /// Consumes selection ownership but executes no user function. A caller
    /// must keep this phase under the original resolution deadline. Both native
    /// wrappers join the original group, including malformed/lost responses.
    pub fn adopt<R: CleanupReservation>(
        &self,
        source: OwnedElement<R>,
        route: MainWorldRoute,
    ) -> AdoptionFuture<R, E::Reservation> {
        let prepared = (|| {
            if route.execution_context_id <= 0
                || route.unique_context_id.is_empty()
                || route.unique_context_id.len() > NATIVE_BYTES
            {
                return Err(ElementEvaluationError::Setup(
                    "exact main-world route is missing or oversized",
                ));
            }
            if !Arc::ptr_eq(&source.element().client(), &route.client) {
                return Err(ElementEvaluationError::Setup(
                    "element adoption cannot cross a CDP session",
                ));
            }
            let reservation = self.cleanup.try_reserve().map_err(|_| {
                ElementEvaluationError::Admission("cleanup continuation not admitted")
            })?;
            let owner = self.reserve()?;
            let evaluation = self.evaluation.bind_shared_execution_context(
                route.client,
                route.execution_context_id,
                route.unique_context_id,
            );
            Ok((reservation, owner, evaluation))
        })();
        match prepared {
            Err(error) => AdoptionFuture::new(
                Box::pin(async move { finish_source(source, Err(error)).await }),
                None,
            ),
            Ok((reservation, mut owner, evaluation)) => {
                let limits = CdpHandleEvaluationLimits::new(self.profile.limits, NATIVE_BYTES, 0);
                AdoptionFuture::new(
                    Box::pin(async move {
                        if !owner.state.healthy.load(Ordering::Acquire) {
                            return finish_source(
                                source,
                                Err(ElementEvaluationError::Admission(
                                    "native group cleanup is not proven",
                                )),
                            )
                            .await;
                        }
                        owner.armed = true;
                        let request = CdpElementHandleAdoptionRequest::from_element_reference(
                            source.element().client(),
                            source.element().object_id(),
                            evaluation.client(),
                            evaluation
                                .execution_context_id()
                                .expect("validated main context"),
                            NATIVE_BYTES,
                        )
                        .map(|request| {
                            request.with_object_group(Arc::from(source.element().object_group()))
                        });
                        let adopted = match request {
                            Ok(request) => {
                                CdpElementHandleAdoptionService::new(limits)
                                    .call(request)
                                    .await
                            }
                            Err(error) => Err(error),
                        };
                        match adopted {
                            Ok(adopted) => Ok(ElementData {
                                source: Some(source),
                                adopted: Some(adopted),
                                evaluation,
                                owner,
                            }),
                            Err(error) => {
                                let result =
                                    finish_source(source, Err::<ElementData<R>, _>(error.into()))
                                        .await;
                                if !matches!(result, Err(ElementEvaluationError::Cleanup { .. })) {
                                    owner.disarm();
                                }
                                result
                            }
                        }
                    }),
                    Some(reservation),
                )
            }
        }
    }
}

struct ElementData<R: CleanupReservation> {
    source: Option<OwnedElement<R>>,
    adopted: Option<CdpAdoptedElementHandle>,
    evaluation: CdpEvaluationService,
    owner: OwnerLease,
}
impl<R: CleanupReservation> ElementData<R> {
    fn params(&self, params: &EvaluateParams) -> Outcome<EvaluateHandleParams> {
        let limits = self.evaluation.limits();
        if params.source().len() > limits.source_bytes {
            return Err(ElementEvaluationError::Admission(
                "evaluation source exceeds its bound",
            ));
        }
        if let Some(arg) = params.argument() {
            arg.validate(limits.value).map_err(|_| {
                ElementEvaluationError::Admission("evaluation argument exceeds its value contract")
            })?;
        }
        // Borrowed caller input stays caller-owned. Only validated, bounded
        // clones enter the single-use native future, under this owner's lease.
        Ok(match (params.mode(), params.argument()) {
            (EvaluationMode::Expression, _) => EvaluateHandleParams::expression(params.source()),
            (EvaluationMode::Function, Some(argument)) => {
                EvaluateHandleParams::function(params.source(), argument.clone())
            }
            (EvaluationMode::Function, None) => {
                EvaluateHandleParams::function_without_argument(params.source())
            }
        })
    }
    async fn evaluate(
        self,
        params: Outcome<EvaluateHandleParams>,
        mut retirement: Option<Box<dyn NativeEvaluationRetirement>>,
    ) -> Outcome<EvaluationValue> {
        let outcome = match params {
            Ok(params) => {
                let receiver = self
                    .adopted
                    .as_ref()
                    .expect("prepared native receiver")
                    .handle()
                    .clone();
                let service = CdpHandleValueEvaluationService::from_evaluation(
                    self.evaluation.clone(),
                    NATIVE_BYTES,
                    0,
                );
                // Merely constructing this future cannot dispatch user code.
                // An already-acknowledged retirement is checked before its
                // first poll, and after cancellation the same race continues
                // under the pre-reserved cleanup job and owner lease.
                let native = service.call(CdpHandleEvaluationRequest::new_on_native_handle(
                    params, receiver,
                ));
                retirement::wait(
                    native,
                    retirement.as_deref_mut(),
                    &self.evaluation.client(),
                    self.evaluation
                        .unique_context_id()
                        .expect("validated main world"),
                )
                .await
            }
            Err(error) => Err(error),
        };
        self.finish(outcome).await
    }
    async fn finish<T>(mut self, primary: Outcome<T>) -> Outcome<T> {
        let result = finish_source(self.source.take().expect("live group owner"), primary).await;
        if !matches!(result, Err(ElementEvaluationError::Cleanup { .. })) {
            self.owner.disarm();
        }
        // The acknowledged group release physically invalidates BOTH objects.
        // Retire the adopted wrapper only afterward, without a second release.
        drop(self.adopted.take());
        result
    }
}

async fn finish_source<R: CleanupReservation, T>(
    source: OwnedElement<R>,
    primary: Outcome<T>,
) -> Outcome<T> {
    match source.release().await {
        Ok(()) => primary,
        Err(cleanup) => Err(ElementEvaluationError::Cleanup {
            primary: primary.err().map(Box::new),
            cleanup,
        }),
    }
}
fn cleanup_failure(error: ElementEvaluationError) -> CleanupFailure {
    CleanupFailure::new(error.to_string())
}
fn cleanup_outcome(error: ElementEvaluationError) -> Result<(), CleanupFailure> {
    if matches!(&error, ElementEvaluationError::Cleanup { .. })
        || matches!(&error, ElementEvaluationError::Backend(error) if error.kind() == pwright_core::FailureKind::Cleanup)
    {
        Err(cleanup_failure(error))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod live;
#[cfg(test)]
mod preparation_live;
#[cfg(test)]
mod tests;
