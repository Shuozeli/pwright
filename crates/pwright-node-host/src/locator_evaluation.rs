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

use pwright_backend_cdp_evaluation::{
    CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpEvaluationServiceFactory,
};
use pwright_backend_cdp_selector_runtime::{
    MAX_COMPILED_SELECTOR_BYTES, SELECTOR_RUNTIME_SYMBOL, SELECTOR_RUNTIME_VERSION,
    compile_selector, selector_runtime_install_script,
};
use pwright_browser_model::FRAME_OBJECT_KIND_ID;
use pwright_cap_evaluation::{EvaluationErrorDetails, EvaluationMode};
use pwright_cap_locator::Selector;
use pwright_composition_chromium::{
    CdpPageServices, ChromiumFrameWorldAccess, ChromiumFrameWorldAccessProfile,
};
#[cfg(all(feature = "selector-registration", feature = "handles"))]
use pwright_composition_chromium_handle_evaluation::ChromiumHandleEvaluationRoute;
use pwright_composition_chromium_locator_evaluation::{
    ElementEvaluationProfile, LocatorElementEvaluationService,
    preparation::{
        LocatorElementPreparationService, PairedWorld, PairedWorldSource, PreparationInputs,
        PreparationProfile,
    },
};
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionProfile,
    ResolutionWaitProfile, StrictResolutionWaiter,
};
use pwright_core::{
    AtomicObjectIdSource, CallMetadata, CancellationReason, Command, CommandCancellation,
    CommandSpec, FailureKind, MethodTag, ObjectId, ObjectKindId, OperationContext, OperationError,
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger, ResourceRequirements,
    ResourceScope, Service,
};
use pwright_runtime::TargetedService;
use pwright_runtime_tokio::{TokioCleanupExecutor, TokioDeadlineScheduler};

#[cfg(feature = "layout-selectors")]
use crate::layout_selectors::LayoutSelectorProgram;
#[cfg(all(feature = "layout-selectors", not(feature = "selector-registration")))]
use crate::layout_selectors::MAX_LAYOUT_SELECTOR_PROGRAM_BYTES;
#[cfg(feature = "react-selectors")]
use crate::locator_query::BrowserSelectorProgram;
#[cfg(feature = "selector-registration")]
use crate::selector_registry::{MAX_REGISTERED_SELECTOR_PROGRAM_BYTES, RegisteredSelectorProgram};
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
// A retained click can hold its creation-world receipt through command
// cancellation independently of its ordinary Handle read admission.
#[cfg(feature = "element-handle-click")]
const RETAINED_CLICK_WORLDS: usize = crate::handles::MAX_CALLS;
#[cfg(not(feature = "element-handle-click"))]
const RETAINED_CLICK_WORLDS: usize = 0;
// Persistent marker worlds and retiring predecessors cannot borrow the
// per-call acquisition slots: they intentionally survive ordinary completion.
#[cfg(feature = "retained-target-marking")]
const MARKER_WORLDS: usize = crate::handles::file_input::markers::WORLD_SLOTS;
#[cfg(not(feature = "retained-target-marking"))]
const MARKER_WORLDS: usize = 0;
const PENDING: usize = 16;
const STATE_SCOPE: ResourceScope = ResourceScope::extension(120, 15);
const STATE_BYTES: u64 = 8192;
pub(crate) const CONVERSION_STATE_BYTES: usize = 4096;
const COMMAND_FUTURE_BYTES: usize = 16 * 1024;
const COLLECTION_WRAPPER_BYTES: usize = 4096;
#[cfg(feature = "selector-registration")]
const OPTIONAL_SELECTOR_BYTES: usize = MAX_REGISTERED_SELECTOR_PROGRAM_BYTES;
#[cfg(all(not(feature = "selector-registration"), feature = "layout-selectors"))]
const OPTIONAL_SELECTOR_BYTES: usize = MAX_LAYOUT_SELECTOR_PROGRAM_BYTES;
#[cfg(not(any(feature = "selector-registration", feature = "layout-selectors")))]
const OPTIONAL_SELECTOR_BYTES: usize = MAX_COMPILED_SELECTOR_BYTES;
const COLLECTION_SELECTOR_BYTES: usize = if OPTIONAL_SELECTOR_BYTES > MAX_COMPILED_SELECTOR_BYTES {
    OPTIONAL_SELECTOR_BYTES
} else {
    MAX_COMPILED_SELECTOR_BYTES
};
pub(crate) const COLLECTION_SOURCE_BYTES: usize =
    COLLECTION_SELECTOR_BYTES + evaluation::SOURCE_BYTES + COLLECTION_WRAPPER_BYTES;
// Explicit drag installation can retain one <=32-world move per admitted
// Page/session controller, independently of Locator's executing commands.
#[cfg(feature = "drag-runtime")]
const WORLD_SNAPSHOTS: usize = 32 * 32
    + RESOLVING
    + HANDLE_WORLDS
    + RETAINED_UPLOAD_WORLDS
    + RETAINED_CLICK_WORLDS
    + MARKER_WORLDS;
#[cfg(not(feature = "drag-runtime"))]
const WORLD_SNAPSHOTS: usize =
    RESOLVING + HANDLE_WORLDS + RETAINED_UPLOAD_WORLDS + RETAINED_CLICK_WORLDS + MARKER_WORLDS;
const CALL_BYTES: u64 = (2
    * (evaluation::MAX_NODES * std::mem::size_of::<EvaluationValue>()
        + evaluation::MAX_ENTRIES * std::mem::size_of::<evaluation::EvaluationProperty>()
        + evaluation::STRING_BYTES
        + evaluation::BINARY_BYTES)
    + COLLECTION_SOURCE_BYTES
    + 2 * CONVERSION_STATE_BYTES
    + COMMAND_FUTURE_BYTES
    + worlds::RETIREMENT_BYTES
    + 8192) as u64;

fn collection_limits() -> CdpEvaluationLimits {
    CdpEvaluationLimits::new(
        COLLECTION_SOURCE_BYTES,
        evaluation::GENERATED_SCRIPT_BYTES,
        evaluation::ERROR_BYTES,
        evaluation::input_value_limits(),
    )
}

pub(crate) struct Profile {
    resolution: ResolutionProfile,
    adoption: ElementEvaluationProfile,
    collection: CdpEvaluationProductionProfile,
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
                    evaluation::input_value_limits(),
                ),
            )
            .map_err(HostError::backend)?
            .with_user_gesture(true),
            collection: CdpEvaluationProductionProfile::new(
                ResourceScope::extension(120, 16),
                collection_limits(),
                EXECUTING,
            )
            .with_user_gesture(true),
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
            .combine_concurrent(self.collection.requirements())
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
    collection: CdpEvaluationServiceFactory,
    waiter: StrictResolutionWaiter,
    preparation: LocatorElementPreparationService,
    access: ChromiumFrameWorldAccess,
    ledger: ResourceLedger,
    slots: Arc<Slots>,
}
impl Installation {
    #[cfg(all(feature = "selector-registration", feature = "handles"))]
    async fn registered_element_program(
        &self,
        frame: ObjectId,
        selector: RegisteredSelectorProgram,
        allow_missing: bool,
    ) -> Result<
        (
            crate::handles::HandleEvaluationProgram,
            ChromiumHandleEvaluationRoute,
        ),
        HostError,
    > {
        let worlds = worlds::Worlds {
            access: self.access.clone(),
            frame,
        };
        let pair = worlds.current().await.map_err(errors::waiting)?;
        let use_main_world = selector.requires_main_world();
        let (execution_context_id, unique_context_id) = if use_main_world {
            (pair.main_execution_context_id(), pair.main_unique_id())
        } else {
            (
                pair.utility_execution_context_id(),
                pair.utility_unique_id(),
            )
        };
        install_collection_selector_runtime(pair.as_ref(), unique_context_id).await?;
        if !pair.is_current().map_err(errors::waiting)? {
            return Err(HostError::TargetClosed);
        }
        let matches = selector.compile("document", false)?;
        let missing = if allow_missing {
            "return null;"
        } else {
            "throw new Error(\"failed to find element matching selector\");"
        };
        let source = format!(
            r#"(() => {{
  const __pw = globalThis[Symbol.for({runtime_symbol:?})];
  if (!__pw || __pw.version !== {runtime_version})
    throw new Error("Pwright selector runtime is unavailable");
  const __pwMatches = {matches};
  const __pwNode = __pwMatches.length === 0 ? null : __pwMatches[0];
  if (__pwNode === null)
    {missing}
  if (!(__pwNode instanceof Node))
    throw new Error(`Expected a Node but got ${{Object.prototype.toString.call(__pwNode)}}`);
  return __pwNode;
}})()"#,
            runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
            runtime_version = SELECTOR_RUNTIME_VERSION,
        );
        if source.len() > COLLECTION_SOURCE_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "registered selector Handle source exceeds {COLLECTION_SOURCE_BYTES} UTF-8 bytes"
            )));
        }
        let evaluation = self.collection.bind_execution_context(
            pair.client(),
            execution_context_id,
            unique_context_id.to_owned(),
        );
        let route = ChromiumHandleEvaluationRoute::from_service(pair.page(), evaluation)
            .map_err(HostError::backend)?;
        Ok((
            crate::handles::EvaluateHandleParams::expression(source).into(),
            route,
        ))
    }

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
            collection: CdpEvaluationServiceFactory::new_with_production_resources(
                profile.collection,
                resources,
            )
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

struct EvaluateAll<F>(PhantomData<fn() -> F>);
enum CollectionSelector {
    Standard(Selector),
    #[cfg(feature = "react-selectors")]
    Browser(BrowserSelectorProgram),
    #[cfg(feature = "selector-registration")]
    Registered(RegisteredSelectorProgram),
    #[cfg(feature = "layout-selectors")]
    Layout(LayoutSelectorProgram),
}

#[derive(Clone, Copy)]
enum CollectionMode {
    All,
    One,
    #[cfg(any(feature = "selector-registration", feature = "layout-selectors"))]
    QueryAll,
}

impl CollectionSelector {
    fn use_main_world(&self, mode: CollectionMode) -> bool {
        let _ = mode;
        match self {
            Self::Standard(_) => false,
            #[cfg(feature = "react-selectors")]
            Self::Browser(_) => true,
            #[cfg(feature = "selector-registration")]
            Self::Registered(program) => {
                matches!(mode, CollectionMode::All) || program.requires_main_world()
            }
            #[cfg(feature = "layout-selectors")]
            Self::Layout(_) => false,
        }
    }

    fn compile(&self, mode: CollectionMode) -> Result<String, HostError> {
        let _ = mode;
        match self {
            Self::Standard(selector) => {
                compile_selector(selector, "document").map_err(HostError::backend)
            }
            #[cfg(feature = "react-selectors")]
            Self::Browser(program) => program.compile("document"),
            #[cfg(feature = "selector-registration")]
            Self::Registered(program) => {
                program.compile("document", matches!(mode, CollectionMode::All))
            }
            #[cfg(feature = "layout-selectors")]
            Self::Layout(program) => program.compile("document"),
        }
    }
}

struct AllParams<F> {
    selector: CollectionSelector,
    mode: CollectionMode,
    convert: F,
    preparing: Arc<AtomicBool>,
}
impl<F: Send + 'static> CommandSpec for EvaluateAll<F> {
    // Host-local typed extension, never registered or sent as a wire method.
    const METHOD: MethodTag = MethodTag::new(65002);
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = &[FRAME_OBJECT_KIND_ID];
    type Params = AllParams<F>;
    type Result = EvaluationValue;
    type ErrorDetails = HostError;
}

async fn install_collection_selector_runtime(
    pair: &worlds::Pair,
    unique_context_id: &str,
) -> Result<(), HostError> {
    let response = pair
        .client()
        .runtime_evaluate_in_unique_context(&selector_runtime_install_script(), unique_context_id)
        .await
        .map_err(HostError::backend)?;
    if response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(serde_json::Value::as_bool)
        == Some(true)
    {
        Ok(())
    } else {
        Err(HostError::Backend(
            "selector-runtime installation did not return true".into(),
        ))
    }
}

#[cfg(feature = "react-selectors")]
async fn install_react_selector_runtime(
    pair: &worlds::Pair,
    unique_context_id: &str,
) -> Result<(), HostError> {
    let response = pair
        .client()
        .runtime_evaluate_in_unique_context(
            &crate::react_selector::install_script(),
            unique_context_id,
        )
        .await
        .map_err(HostError::backend)?;
    if response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(serde_json::Value::as_bool)
        == Some(true)
    {
        Ok(())
    } else {
        Err(HostError::Backend(
            "React selector runtime installation did not return true".into(),
        ))
    }
}

fn collection_params(
    selector: &CollectionSelector,
    mode: CollectionMode,
    params: EvaluateParams,
) -> Result<EvaluateParams, HostError> {
    if params.mode() != EvaluationMode::Function {
        return Err(HostError::InvalidArgument(
            "locator.evaluateAll requires a function".into(),
        ));
    }
    let matches = selector.compile(mode)?;
    #[cfg(feature = "react-selectors")]
    let deprecated_react = matches!(selector, CollectionSelector::Browser(_));
    let invocation = match mode {
        CollectionMode::All => "return await __pwFunction(__pwMatches, __pwArgument);".to_owned(),
        #[cfg(any(feature = "selector-registration", feature = "layout-selectors"))]
        CollectionMode::QueryAll => {
            "return await __pwFunction(__pwMatches, __pwArgument);".to_owned()
        }
        CollectionMode::One => r#"if (__pwMatches.length === 0)
    throw new Error("failed to find element matching selector");
  return await __pwFunction(__pwMatches[0], __pwArgument);"#
            .to_owned(),
    };
    let source = format!(
        r#"async (__pwArgument) => {{
  const __pw = globalThis[Symbol.for({runtime_symbol:?})];
  if (!__pw || __pw.version !== {runtime_version})
    throw new Error("Pwright selector runtime is unavailable");
  const __pwMatches = {matches};
  {deprecated_check}
  const __pwFunction = ({function});
  {invocation}
}}"#,
        runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
        runtime_version = SELECTOR_RUNTIME_VERSION,
        function = params.source(),
        deprecated_check = {
            #[cfg(feature = "react-selectors")]
            {
                if deprecated_react {
                    r#"if (__pwMatches.length) throw new Error(`"_react" selector is not supported: resolved to ${__pwMatches.length} element${__pwMatches.length === 1 ? '' : 's'}`);"#
                } else {
                    ""
                }
            }
            #[cfg(not(feature = "react-selectors"))]
            {
                ""
            }
        },
    );
    if source.len() > COLLECTION_SOURCE_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "locator.evaluateAll generated source exceeds {COLLECTION_SOURCE_BYTES} UTF-8 bytes"
        )));
    }
    Ok(match params.argument() {
        Some(argument) => EvaluateParams::function(source, argument.clone()),
        None => EvaluateParams::function_without_argument(source),
    })
}

fn collection_evaluation_error(error: OperationError<EvaluationErrorDetails>) -> HostError {
    if let Some(EvaluationErrorDetails::Javascript {
        name,
        message,
        stack,
    }) = error.details()
    {
        HostError::Javascript {
            name: name.clone(),
            message: message.clone(),
            stack: stack.clone(),
        }
    } else {
        HostError::operation(error, 0)
    }
}

impl<F, Converted> TargetedService<AllParams<F>> for Installation
where
    F: FnOnce() -> Converted + Send + 'static,
    Converted: Future<Output = Result<EvaluateParams, HostError>> + Send + 'static,
{
    type Response = EvaluationValue;
    type Error = OperationError<HostError>;
    type Future<'a> = Pin<Box<dyn Future<Output = Outcome> + Send + 'a>>;

    fn call_for(&self, frame: ObjectId, call: AllParams<F>) -> Self::Future<'_> {
        let future = async move {
            let worlds = worlds::Worlds {
                access: self.access.clone(),
                frame,
            };
            let pair = worlds
                .current()
                .await
                .map_err(|error| failure(errors::waiting(error)))?;
            let use_main_world = call.selector.use_main_world(call.mode);
            let (execution_context_id, unique_context_id) = if use_main_world {
                (pair.main_execution_context_id(), pair.main_unique_id())
            } else {
                (
                    pair.utility_execution_context_id(),
                    pair.utility_unique_id(),
                )
            };
            install_collection_selector_runtime(pair.as_ref(), unique_context_id)
                .await
                .map_err(failure)?;
            #[cfg(feature = "react-selectors")]
            if matches!(&call.selector, CollectionSelector::Browser(_)) {
                install_react_selector_runtime(pair.as_ref(), unique_context_id)
                    .await
                    .map_err(failure)?;
            }
            if !pair
                .is_current()
                .map_err(|error| failure(errors::waiting(error)))?
            {
                return Err(failure(HostError::Backend(
                    "Frame execution world changed before locator.evaluateAll".into(),
                )));
            }
            call.preparing.store(false, Ordering::Release);
            let params = (call.convert)()
                .await
                .and_then(validate_input)
                .and_then(|params| collection_params(&call.selector, call.mode, params))
                .map_err(failure)?;
            let service = self.collection.bind_execution_context(
                pair.client(),
                execution_context_id,
                unique_context_id.to_owned(),
            );
            service
                .call(params)
                .await
                .map(pwright_cap_evaluation::EvaluateResult::into_value)
                .map_err(|error| failure(collection_evaluation_error(error)))
        };
        if std::mem::size_of_val(&future) > COMMAND_FUTURE_BYTES {
            return Box::pin(ready(Err(failure(HostError::ResourceExhausted(
                "Locator collection evaluation command future exceeds its budget".into(),
            )))));
        }
        Box::pin(future)
    }
}

impl<F: Send + 'static> CommandCancellation<EvaluateAll<F>> for Installation {
    type Future<'a> = Ready<Result<(), OperationError<HostError>>>;

    fn cancel(&self, _: OperationContext, _: CancellationReason) -> Self::Future<'_> {
        // Collection evaluation retains no DOM identity or object group after
        // its one awaited value result settles.
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
            .validate(evaluation::input_value_limits())
            .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
    }
    Ok(params)
}

impl HostSession {
    #[cfg(all(feature = "selector-registration", feature = "handles"))]
    pub(crate) async fn locator_prepare_registered_element_handle(
        &self,
        frame: ObjectId,
        selector: RegisteredSelectorProgram,
    ) -> Result<crate::handles::PendingHandle, HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .locator_evaluation
                .clone()
        };
        let (program, route) = installation
            .registered_element_program(frame, selector, false)
            .await?;
        self.frame_prepare_element_program_in_world(&frame.get().to_string(), program, route)
            .await
    }

    #[cfg(all(feature = "selector-registration", feature = "handles"))]
    pub(crate) async fn locator_prepare_registered_optional_element_handle(
        &self,
        frame: ObjectId,
        selector: RegisteredSelectorProgram,
    ) -> Result<Option<crate::handles::PendingHandle>, HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .locator_evaluation
                .clone()
        };
        let (program, route) = installation
            .registered_element_program(frame, selector, true)
            .await?;
        self.frame_prepare_optional_element_program_in_world(
            &frame.get().to_string(),
            program,
            route,
        )
        .await
    }

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

    pub(crate) async fn locator_evaluate_all<F, Converted>(
        &self,
        frame: ObjectId,
        selector: Selector,
        timeout_ms: Option<u32>,
        convert: F,
    ) -> Result<EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: Future<Output = Result<EvaluateParams, HostError>> + Send + 'static,
    {
        self.locator_evaluate_collection(
            frame,
            CollectionSelector::Standard(selector),
            CollectionMode::All,
            timeout_ms,
            convert,
        )
        .await
    }

    #[cfg(feature = "react-selectors")]
    pub(crate) async fn locator_evaluate_all_react<F, Converted>(
        &self,
        frame: ObjectId,
        selector: BrowserSelectorProgram,
        timeout_ms: Option<u32>,
        convert: F,
    ) -> Result<EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: Future<Output = Result<EvaluateParams, HostError>> + Send + 'static,
    {
        self.locator_evaluate_collection(
            frame,
            CollectionSelector::Browser(selector),
            CollectionMode::All,
            timeout_ms,
            convert,
        )
        .await
    }

    pub(crate) async fn locator_evaluate_first<F, Converted>(
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
        self.locator_evaluate_collection(
            frame,
            CollectionSelector::Standard(selector),
            CollectionMode::One,
            Some(timeout_ms),
            convert,
        )
        .await
    }

    #[cfg(feature = "selector-registration")]
    pub(crate) async fn locator_evaluate_registered<F, Converted>(
        &self,
        frame: ObjectId,
        selector: RegisteredSelectorProgram,
        timeout_ms: u32,
        convert: F,
    ) -> Result<EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: Future<Output = Result<EvaluateParams, HostError>> + Send + 'static,
    {
        self.locator_evaluate_collection(
            frame,
            CollectionSelector::Registered(selector),
            CollectionMode::One,
            Some(timeout_ms),
            convert,
        )
        .await
    }

    #[cfg(feature = "selector-registration")]
    pub(crate) async fn locator_evaluate_all_registered<F, Converted>(
        &self,
        frame: ObjectId,
        selector: RegisteredSelectorProgram,
        timeout_ms: Option<u32>,
        convert: F,
    ) -> Result<EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: Future<Output = Result<EvaluateParams, HostError>> + Send + 'static,
    {
        self.locator_evaluate_collection(
            frame,
            CollectionSelector::Registered(selector),
            CollectionMode::All,
            timeout_ms,
            convert,
        )
        .await
    }

    #[cfg(feature = "selector-registration")]
    pub(crate) async fn locator_query_all_registered<F, Converted>(
        &self,
        frame: ObjectId,
        selector: RegisteredSelectorProgram,
        convert: F,
    ) -> Result<EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: Future<Output = Result<EvaluateParams, HostError>> + Send + 'static,
    {
        self.locator_evaluate_collection(
            frame,
            CollectionSelector::Registered(selector),
            CollectionMode::QueryAll,
            None,
            convert,
        )
        .await
    }

    #[cfg(feature = "layout-selectors")]
    pub(crate) async fn locator_evaluate_layout<F, Converted>(
        &self,
        frame: ObjectId,
        selector: LayoutSelectorProgram,
        timeout_ms: u32,
        convert: F,
    ) -> Result<EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: Future<Output = Result<EvaluateParams, HostError>> + Send + 'static,
    {
        self.locator_evaluate_collection(
            frame,
            CollectionSelector::Layout(selector),
            CollectionMode::One,
            Some(timeout_ms),
            convert,
        )
        .await
    }

    #[cfg(feature = "layout-selectors")]
    pub(crate) async fn locator_evaluate_all_layout<F, Converted>(
        &self,
        frame: ObjectId,
        selector: LayoutSelectorProgram,
        timeout_ms: Option<u32>,
        convert: F,
    ) -> Result<EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: Future<Output = Result<EvaluateParams, HostError>> + Send + 'static,
    {
        self.locator_evaluate_collection(
            frame,
            CollectionSelector::Layout(selector),
            CollectionMode::All,
            timeout_ms,
            convert,
        )
        .await
    }

    #[cfg(feature = "layout-selectors")]
    pub(crate) async fn locator_query_all_layout<F, Converted>(
        &self,
        frame: ObjectId,
        selector: LayoutSelectorProgram,
        convert: F,
    ) -> Result<EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: Future<Output = Result<EvaluateParams, HostError>> + Send + 'static,
    {
        self.locator_evaluate_collection(
            frame,
            CollectionSelector::Layout(selector),
            CollectionMode::QueryAll,
            None,
            convert,
        )
        .await
    }

    async fn locator_evaluate_collection<F, Converted>(
        &self,
        frame: ObjectId,
        selector: CollectionSelector,
        mode: CollectionMode,
        timeout_ms: Option<u32>,
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
        let command = Command::<EvaluateAll<F>>::new(
            call_id,
            frame,
            CallMetadata::default(),
            AllParams {
                selector,
                mode,
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
                let mut details = None;
                let error = error.map_details(|value| {
                    details = Some(value);
                });
                details.unwrap_or_else(|| HostError::operation(error, timeout_ms.unwrap_or(0)))
            })
        };
        let deadline = async {
            match timeout_ms {
                Some(0) | None => pending::<()>().await,
                Some(timeout_ms) => {
                    tokio::time::sleep(Duration::from_millis(u64::from(timeout_ms))).await;
                }
            }
        };
        let reported_timeout = timeout_ms.unwrap_or(0);
        let outcome = phase::run(
            operation,
            preparing,
            deadline,
            self.stop.cancelled(),
            reported_timeout,
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
