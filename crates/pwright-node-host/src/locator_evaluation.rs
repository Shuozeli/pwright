//! One controlled Frame command owns selection, delayed conversion, evaluation
//! and group cleanup. A native prepared element never escapes into Node.

mod errors;
mod phase;
mod worlds;

use std::{
    future::{Future, Ready, pending, ready},
    marker::PhantomData,
    num::NonZeroUsize,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use pwright_backend_cdp_evaluation::CdpEvaluationLimits;
use pwright_browser_model::FRAME_OBJECT_KIND_ID;
use pwright_cap_locator::Selector;
use pwright_composition_chromium::{
    CdpPageServices, ChromiumFrameWorldAccess, ChromiumFrameWorldAccessProfile,
};
use pwright_composition_chromium_locator_evaluation::{
    ElementEvaluationProfile, LocatorElementEvaluationService,
    preparation::{LocatorElementPreparationService, PreparationInputs, PreparationProfile},
};
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionProfile,
    ResolutionWaitProfile, StrictResolutionWaiter,
};
use pwright_core::{
    AtomicObjectIdSource, CallMetadata, CancellationReason, Command, CommandCancellation,
    CommandSpec, FailureKind, MethodTag, ObjectId, ObjectKindId, OperationContext, OperationError,
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger, ResourceRequirements,
    ResourceScope,
};
use pwright_runtime::TargetedService;
use pwright_runtime_tokio::{TokioCleanupExecutor, TokioDeadlineScheduler};

use crate::{
    HostError, HostSession,
    evaluation::{self, EvaluateParams, EvaluationValue},
};

const EXECUTING: usize = 8;
// A cancelled submitted upload may retain its group after the runtime lane is
// returned. Selected uploads have one independent continuation slot.
const RESOLVING: usize = EXECUTING + cfg!(feature = "file-input") as usize;
const HANDLE_WORLDS: usize = if cfg!(feature = "handles") { 64 } else { 0 };
// Retained acquisitions may keep one exact snapshot each after their runtime
// command lane is returned. They are independent of Locator retries and the
// already-admitted upload's one additional continuation snapshot.
#[cfg(feature = "retained-file-input")]
const RETAINED_UPLOAD_WORLDS: usize = crate::handles::MAX_CALLS;
#[cfg(not(feature = "retained-file-input"))]
const RETAINED_UPLOAD_WORLDS: usize = 0;
// Persistent marker worlds and retiring predecessors cannot borrow the
// per-call acquisition slots: they intentionally survive ordinary completion.
#[cfg(feature = "retained-target-marking")]
const MARKER_WORLDS: usize = crate::handles::file_input::markers::WORLD_SLOTS;
#[cfg(not(feature = "retained-target-marking"))]
const MARKER_WORLDS: usize = 0;
const PENDING: usize = 16;
const STATE_SCOPE: ResourceScope = ResourceScope::extension(120, 15);
const STATE_BYTES: u64 = 8192;
const CONVERSION_STATE_BYTES: usize = 4096;
const COMMAND_FUTURE_BYTES: usize = 16 * 1024;
// Explicit drag installation can retain one <=32-world move per admitted
// Page/session controller, independently of Locator's executing commands.
#[cfg(feature = "drag-runtime")]
const WORLD_SNAPSHOTS: usize =
    32 * 32 + RESOLVING + HANDLE_WORLDS + RETAINED_UPLOAD_WORLDS + MARKER_WORLDS;
#[cfg(not(feature = "drag-runtime"))]
const WORLD_SNAPSHOTS: usize = RESOLVING + HANDLE_WORLDS + RETAINED_UPLOAD_WORLDS + MARKER_WORLDS;
const CALL_BYTES: u64 = (2
    * (evaluation::MAX_NODES * std::mem::size_of::<EvaluationValue>()
        + evaluation::MAX_ENTRIES * std::mem::size_of::<evaluation::EvaluationProperty>()
        + evaluation::STRING_BYTES
        + evaluation::BINARY_BYTES)
    + evaluation::SOURCE_BYTES
    + 2 * CONVERSION_STATE_BYTES
    + COMMAND_FUTURE_BYTES
    + worlds::RETIREMENT_BYTES
    + 8192) as u64;

pub(crate) struct Profile {
    resolution: ResolutionProfile,
    adoption: ElementEvaluationProfile,
    waiting: ResolutionWaitProfile,
    preparation: PreparationProfile,
    access: ChromiumFrameWorldAccessProfile,
}
impl Profile {
    pub(crate) fn new() -> Result<Self, HostError> {
        Ok(Self {
            resolution: ResolutionProfile::new(ResourceScope::extension(120, 10), RESOLVING)
                .map_err(HostError::backend)?,
            adoption: ElementEvaluationProfile::new(
                ResourceScope::extension(120, 11),
                EXECUTING,
                CdpEvaluationLimits::new(
                    evaluation::SOURCE_BYTES,
                    evaluation::GENERATED_SCRIPT_BYTES,
                    evaluation::ERROR_BYTES,
                    evaluation::value_limits(),
                ),
            )
            .map_err(HostError::backend)?,
            waiting: ResolutionWaitProfile::new(
                ResourceScope::extension(120, 12),
                EXECUTING,
                [0, 20, 50, 100, 100, 500].map(Duration::from_millis),
            )
            .map_err(HostError::backend)?,
            preparation: PreparationProfile::new(
                ResourceScope::extension(120, 13),
                EXECUTING,
                Duration::from_millis(20),
            )
            .map_err(HostError::backend)?,
            access: ChromiumFrameWorldAccessProfile::new(
                ResourceScope::extension(120, 14),
                NonZeroUsize::new(WORLD_SNAPSHOTS).expect("positive fixed capacity"),
            )
            .and_then(ChromiumFrameWorldAccessProfile::with_native_retirement)
            .map_err(HostError::backend)?,
        })
    }
    fn state_requirements() -> ResourceRequirements {
        ResourceRequirements::none()
            .with_class(
                ResourceClass::BackendStateBytes,
                STATE_BYTES + CALL_BYTES * PENDING as u64,
                STATE_BYTES + CALL_BYTES * PENDING as u64,
            )
            .with_metadata(1, PENDING + 1)
    }
    pub(crate) fn requirements(&self) -> ResourceRequirements {
        Self::state_requirements()
            .combine_concurrent(self.resolution.requirements())
            .combine_concurrent(self.adoption.requirements())
            .combine_concurrent(self.waiting.requirements())
            .combine_concurrent(self.preparation.requirements())
            .combine_concurrent(self.access.requirements())
    }
}

struct Slots {
    active: AtomicUsize,
    _state: ResourceLease,
}
struct CallLease {
    slots: Arc<Slots>,
    _lease: ResourceLease,
}
impl Drop for CallLease {
    fn drop(&mut self) {
        self.slots.active.fetch_sub(1, Ordering::AcqRel);
    }
}
impl CallLease {
    fn admit(slots: Arc<Slots>, ledger: &ResourceLedger) -> Result<Self, HostError> {
        slots
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < PENDING).then(|| n + 1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted("Locator evaluation pending capacity exceeded".into())
            })?;
        match ledger.reserve(STATE_SCOPE, ResourceClass::BackendStateBytes, CALL_BYTES) {
            Ok(lease) => Ok(Self {
                slots,
                _lease: lease,
            }),
            Err(error) => {
                slots.active.fetch_sub(1, Ordering::AcqRel);
                Err(HostError::backend(error))
            }
        }
    }
}

pub(crate) struct Installation {
    attempts: ContinuableResolutionService<CdpLocatorResolutionService, TokioCleanupExecutor>,
    adoption: LocatorElementEvaluationService<TokioCleanupExecutor>,
    waiter: StrictResolutionWaiter,
    preparation: LocatorElementPreparationService,
    access: ChromiumFrameWorldAccess,
    ledger: ResourceLedger,
    slots: Arc<Slots>,
}
impl Installation {
    #[cfg(feature = "file-input")]
    pub(crate) async fn resolve_for_file_input(
        &self,
        frame: ObjectId,
        selector: Selector,
    ) -> Result<
        pwright_composition_chromium_locator_resolution::OwnedElement<
            pwright_runtime_tokio::TokioCleanupReservation,
        >,
        HostError,
    > {
        let routes = worlds::UploadRoutes::new(worlds::Worlds {
            access: self.access.clone(),
            frame,
        });
        self.waiter
            .wait(
                selector,
                &self.attempts,
                &routes,
                &TokioDeadlineScheduler,
                pending(),
            )
            .await
            .map_err(errors::waiting)
    }
    #[cfg(any(feature = "drag-runtime", feature = "handles"))]
    pub(crate) fn world_access(&self) -> ChromiumFrameWorldAccess {
        self.access.clone()
    }
    pub(crate) async fn retire_owned_native_sessions(
        &self,
        browser: &dyn pwright_cdp::CdpClient,
    ) -> Result<(), HostError> {
        self.access
            .retire_owned_native_sessions(browser)
            .await
            .map_err(HostError::backend)
    }
    pub(crate) fn new(
        services: &CdpPageServices,
        resources: &ProductionResourceLedger,
        cleanup: TokioCleanupExecutor,
    ) -> Result<Self, HostError> {
        if std::mem::size_of::<Self>() + std::mem::size_of::<Slots>() > STATE_BYTES as usize {
            return Err(HostError::ResourceExhausted(
                "Locator evaluation installation exceeds state budget".into(),
            ));
        }
        let profile = Profile::new()?;
        let ledger = resources
            .claim(Profile::state_requirements())
            .map_err(HostError::backend)?;
        let state = ledger
            .reserve(STATE_SCOPE, ResourceClass::BackendStateBytes, STATE_BYTES)
            .map_err(HostError::backend)?;
        Ok(Self {
            attempts: ContinuableResolutionService::new(
                Arc::new(
                    CdpLocatorResolutionService::new(
                        profile.resolution,
                        resources,
                        // One resolver and ID source per native connection; never recreated
                        // while that connection survives, including cleanup continuations.
                        Arc::new(AtomicObjectIdSource::new(1).map_err(HostError::backend)?),
                    )
                    .map_err(HostError::backend)?,
                ),
                cleanup.clone(),
            ),
            adoption: LocatorElementEvaluationService::new(profile.adoption, resources, cleanup)
                .map_err(HostError::backend)?,
            waiter: StrictResolutionWaiter::new(profile.waiting, resources)
                .map_err(HostError::backend)?,
            preparation: LocatorElementPreparationService::new(profile.preparation, resources)
                .map_err(HostError::backend)?,
            access: ChromiumFrameWorldAccess::new(services, profile.access, resources)
                .map_err(HostError::backend)?,
            ledger,
            slots: Arc::new(Slots {
                active: AtomicUsize::new(0),
                _state: state,
            }),
        })
    }
    fn admit(&self) -> Result<CallLease, HostError> {
        CallLease::admit(self.slots.clone(), &self.ledger)
    }
}

struct Evaluate<F>(PhantomData<fn() -> F>);
struct Params<F> {
    selector: Selector,
    convert: F,
    preparing: Arc<AtomicBool>,
}
impl<F: Send + 'static> CommandSpec for Evaluate<F> {
    // Host-local typed extension, never registered or sent as a wire method.
    const METHOD: MethodTag = MethodTag::new(65001);
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = &[FRAME_OBJECT_KIND_ID];
    type Params = Params<F>;
    type Result = EvaluationValue;
    type ErrorDetails = HostError;
}
type Outcome = Result<EvaluationValue, OperationError<HostError>>;
fn failure(error: HostError) -> OperationError<HostError> {
    OperationError::new(
        FailureKind::Backend,
        "Locator evaluation failed",
        Some(error),
    )
}
impl<F, Converted> TargetedService<Params<F>> for Installation
where
    F: FnOnce() -> Converted + Send + 'static,
    Converted: Future<Output = Result<EvaluateParams, HostError>> + Send + 'static,
{
    type Response = EvaluationValue;
    type Error = OperationError<HostError>;
    type Future<'a> = Pin<Box<dyn Future<Output = Outcome> + Send + 'a>>;
    fn call_for(&self, frame: ObjectId, call: Params<F>) -> Self::Future<'_> {
        let future = async move {
            let worlds = worlds::Worlds {
                access: self.access.clone(),
                frame,
            };
            let inputs = PreparationInputs {
                attempts: &self.attempts,
                waiter: &self.waiter,
                adoption: &self.adoption,
                worlds: &worlds,
                delays: &TokioDeadlineScheduler,
            };
            let element = self
                .preparation
                .prepare(call.selector, &inputs, pending())
                .await
                .map_err(|error| failure(errors::preparation(error)))?;
            // This exact command still holds target/connection admission and
            // runtime liveness. Only its resolution deadline ends here.
            call.preparing.store(false, Ordering::Release);
            let params = match (call.convert)().await.and_then(validate_input) {
                Ok(params) => params,
                Err(primary) => {
                    return match element.release().await {
                        Ok(()) => Err(failure(primary)),
                        Err(cleanup) => Err(failure(HostError::backend(format_args!(
                            "{primary}; temporary element cleanup failed: {cleanup}"
                        )))),
                    };
                }
            };
            element
                .evaluate(&params)
                .await
                .map_err(|error| failure(errors::evaluation(error)))
        };
        if std::mem::size_of_val(&future) > COMMAND_FUTURE_BYTES {
            return Box::pin(ready(Err(failure(HostError::ResourceExhausted(
                "Locator evaluation command future exceeds its budget".into(),
            )))));
        }
        Box::pin(future)
    }
}
impl<F: Send + 'static> CommandCancellation<Evaluate<F>> for Installation {
    type Future<'a> = Ready<Result<(), OperationError<HostError>>>;
    fn cancel(&self, _: OperationContext, _: CancellationReason) -> Self::Future<'_> {
        // The dropped native phase owns its pre-reserved continuation. This
        // acknowledgement is NOT proof of native release; host shutdown drains
        // the shared executor and reports any failed/unproven group cleanup.
        ready(Ok(()))
    }
}

fn validate_input(params: EvaluateParams) -> Result<EvaluateParams, HostError> {
    if params.source().len() > evaluation::SOURCE_BYTES {
        return Err(HostError::InvalidArgument(
            "evaluation source exceeds 32768 UTF-8 bytes".into(),
        ));
    }
    if let Some(argument) = params.argument() {
        argument
            .validate(evaluation::value_limits())
            .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
    }
    Ok(params)
}

impl HostSession {
    pub(crate) async fn locator_evaluate<F, Converted>(
        &self,
        frame: ObjectId,
        selector: Selector,
        timeout_ms: u32,
        convert: F,
    ) -> Result<EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: Future<Output = Result<EvaluateParams, HostError>> + Send + 'static,
    {
        if std::mem::size_of::<F>() > CONVERSION_STATE_BYTES
            || std::mem::size_of::<Converted>() > CONVERSION_STATE_BYTES
        {
            return Err(HostError::ResourceExhausted(
                "Locator conversion state exceeds 4096 bytes".into(),
            ));
        }
        let (_, call_id, _operation) = self.admit_operation(&frame.get().to_string())?;
        let (runtime, installation) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            (live.runtime.clone(), live.locator_evaluation.clone())
        };
        let _lease = installation.admit()?;
        let preparing = Arc::new(AtomicBool::new(true));
        let command = Command::<Evaluate<F>>::new(
            call_id,
            frame,
            CallMetadata::default(),
            Params {
                selector,
                convert,
                preparing: preparing.clone(),
            },
        );
        let dispatch = runtime.dispatch_with(command, installation.as_ref(), installation.clone());
        let operation = async move {
            let result = match dispatch.await {
                Ok(result) => result,
                Err(never) => match never {},
            };
            result.into_outcome().map_err(|error| {
                // Preserve structured JS/conversion errors; runtime liveness
                // failures have no nested detail and use the standard mapper.
                let mut details = None;
                let error = error.map_details(|value| {
                    details = Some(value);
                });
                details.unwrap_or_else(|| HostError::operation(error, timeout_ms))
            })
        };
        let deadline = async {
            if timeout_ms == 0 {
                pending::<()>().await;
            }
            tokio::time::sleep(Duration::from_millis(u64::from(timeout_ms))).await;
        };
        let outcome = phase::run(
            operation,
            preparing,
            deadline,
            self.stop.cancelled(),
            timeout_ms,
        )
        .await;
        crate::completion::publish_command_result(
            outcome,
            self.is_connected(),
            self.flush_topology(),
        )
        .await
    }
}

#[cfg(test)]
mod live;
#[cfg(test)]
mod tests;
