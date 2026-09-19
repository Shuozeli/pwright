//! A selected Rust-only native phase, not Node preparation or complete upload.
//! Caller cancellation owns its native continuations; no new timeout is started.
use super::{
    Handles, PendingHandle, TrackedCallAdmission, WorldProof, check_world_parts, file_input,
    map_leaf_error,
    markers::{self, MarkerCache, MarkerCacheError, MarkerKey, MarkerLease},
};
use crate::{HostError, HostSession};
#[cfg(feature = "retained-target-prevalidation")]
use pwright_backend_cdp_file_input::{
    CdpFileInputPreflightService, FileInputPreflightFlags, RetainedPreflightProfile,
};
use pwright_backend_cdp_file_input::{
    CdpTargetMarkingService, MarkerWorld, TargetMarkingError, TargetMarkingProfile,
};
use pwright_browser_model::ELEMENT_HANDLE_OBJECT_KIND_ID;
use pwright_composition_chromium::{
    ChromiumFrameWorldSnapshot, ChromiumNativeUtilityWorldLifetime, ChromiumUtilityWorldSnapshot,
};
#[cfg(feature = "retained-serialized-payload-mutation")]
use pwright_composition_chromium_file_input::{
    CdpSerializedPayloadMutationService, SerializedPayloadMutationOperationService,
    SerializedPayloadMutationProfile, SerializedPayloadOperationError,
};
#[cfg(feature = "retained-target-prevalidation")]
use pwright_composition_chromium_file_input::{
    FileInputPreflightOperationService, OwnedValidatedMarkedFileInput, PrevalidationError,
};
use pwright_composition_chromium_file_input::{
    MarkerOperationError, TargetMarkingOperationService, UploadError, UploadProfile,
};
use pwright_composition_chromium_handle_evaluation::ChromiumRetainedElementSource;
use pwright_composition_chromium_locator_resolution::{
    ObjectGroupDestructionProof, ResolutionError, RetainedElementRequest,
};
use pwright_core::{
    AtomicObjectIdSource, CallMetadata, CancellationReason, Command, CommandCancellation,
    CommandSpec, FailureKind, MethodTag, ObjectId, ObjectKindId, OperationContext, OperationError,
    ProductionResourceLedger, ResourceRequirements, ResourceScope,
};
use pwright_runtime::TargetedService;
use pwright_runtime_tokio::{TokioCleanupExecutor, TokioCleanupReservation};
#[cfg(feature = "retained-serialized-payload-mutation")]
use serde_json::Value;
use std::{
    future::{Future, Ready, ready},
    pin::Pin,
    sync::Arc,
};

/// Typed completion retained through the FFI parking/publication boundary.
/// Embeddings must not flatten this before publishing preceding lifecycle work.
pub type TargetMarkOutcome = Result<(), OperationError<TargetMarkPhaseError>>;
pub type RetainedTargetMarkOutcome =
    Result<RetainedMarkedFileInput, OperationError<TargetMarkPhaseError>>;
type Outcome = TargetMarkOutcome;

type ConcreteMarkedElement = pwright_composition_chromium_file_input::OwnedMarkedElement<
    TokioCleanupReservation,
    TokioCleanupReservation,
>;
#[cfg(feature = "retained-target-prevalidation")]
type ConcreteValidatedElement = OwnedValidatedMarkedFileInput<
    TokioCleanupReservation,
    TokioCleanupReservation,
    TokioCleanupReservation,
>;

#[cfg(feature = "retained-target-prevalidation")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetainedTargetPrevalidationFlags {
    pub multiple: bool,
    pub directory_upload: bool,
}

/// Move-only original target retained after marking for the next native phase.
/// Dropping it follows the already-reserved group-cleanup continuation; it is
/// not a public Handle or authority to bypass source/world revalidation.
#[must_use = "pass to the next retained phase, release, or drop for cleanup"]
pub struct RetainedMarkedFileInput {
    marked: Option<ConcreteMarkedElement>,
    #[cfg(feature = "retained-target-prevalidation")]
    preflight: Arc<FileInputPreflightOperationService<TokioCleanupExecutor>>,
    #[cfg(feature = "retained-serialized-payload-mutation")]
    mutation: Arc<SerializedPayloadMutationOperationService<TokioCleanupExecutor>>,
}

impl RetainedMarkedFileInput {
    pub async fn release(mut self) -> TargetMarkOutcome {
        self.marked
            .take()
            .expect("live retained marked file input")
            .release()
            .await
            .map_err(mark_error)
    }

    #[cfg(feature = "retained-target-prevalidation")]
    pub async fn prevalidate(
        mut self,
        flags: RetainedTargetPrevalidationFlags,
    ) -> Result<RetainedPrevalidatedFileInput, PrevalidationError> {
        let future = self.preflight.prepare_marked(
            self.marked.take().expect("live retained marked file input"),
            FileInputPreflightFlags {
                multiple: flags.multiple,
                directory_upload: flags.directory_upload,
            },
        )?;
        future.await.map(|validated| RetainedPrevalidatedFileInput {
            validated: Some(validated),
            #[cfg(feature = "retained-serialized-payload-mutation")]
            mutation: Arc::clone(&self.mutation),
        })
    }
}

#[cfg(feature = "retained-target-prevalidation")]
#[must_use = "pass to the next retained phase, release, or drop for cleanup"]
pub struct RetainedPrevalidatedFileInput {
    validated: Option<ConcreteValidatedElement>,
    #[cfg(feature = "retained-serialized-payload-mutation")]
    mutation: Arc<SerializedPayloadMutationOperationService<TokioCleanupExecutor>>,
}

#[cfg(feature = "retained-target-prevalidation")]
impl RetainedPrevalidatedFileInput {
    pub fn object_id(&self) -> &str {
        self.validated
            .as_ref()
            .expect("live retained prevalidated file input")
            .object_id()
    }

    pub async fn release(mut self) -> Result<(), PrevalidationError> {
        self.validated
            .take()
            .expect("live retained prevalidated file input")
            .release()
            .await
    }

    #[cfg(feature = "retained-serialized-payload-mutation")]
    pub async fn mutate_serialized<Retained: Send + 'static>(
        mut self,
        graph: Value,
        retained: Retained,
    ) -> Result<(), SerializedPayloadOperationError> {
        self.mutation
            .prepare_retaining(
                self.validated
                    .take()
                    .expect("live retained prevalidated file input"),
                graph,
                retained,
            )?
            .await
    }
}

#[cfg(feature = "retained-target-prevalidation")]
pub fn target_prevalidation_language_error(error: PrevalidationError) -> HostError {
    HostError::backend(error)
}

#[cfg(feature = "retained-serialized-payload-mutation")]
pub fn serialized_payload_mutation_language_error(
    error: SerializedPayloadOperationError,
) -> HostError {
    HostError::backend(error)
}

/// Preserve native structure for a later language-boundary recoverability
/// policy. Closed routes, protocol bounds and failed cleanup are not inferred
/// from diagnostic strings or silently converted into a disconnected node.
#[derive(Debug, thiserror::Error)]
pub enum TargetMarkPhaseError {
    #[error(transparent)]
    Host(HostError),
    #[error(transparent)]
    Cache(MarkerCacheError),
    #[error(transparent)]
    Adoption(ResolutionError),
    #[error(transparent)]
    Marking(MarkerOperationError),
    #[error("target revalidation failed: {primary}; cleanup also failed: {cleanup}")]
    RevalidationCleanup {
        primary: Box<OperationError<HostError>>,
        cleanup: ResolutionError,
    },
}

/// Apply the Node-facing error-class policy only after ordered publication.
/// Target marking starts no independent deadline, so a timeout here would be
/// an internal protocol violation rather than a new public timeout boundary.
pub fn target_mark_language_error(error: OperationError<TargetMarkPhaseError>) -> HostError {
    match error.kind() {
        FailureKind::TargetClosed | FailureKind::TransportClosed => HostError::TargetClosed,
        FailureKind::ResourceExhausted => HostError::ResourceExhausted(error.message().to_owned()),
        FailureKind::UnsupportedCapability => {
            HostError::UnsupportedOperation(error.message().to_owned())
        }
        FailureKind::Timeout
        | FailureKind::Cancelled
        | FailureKind::BrowserCrashed
        | FailureKind::ProtocolViolation
        | FailureKind::Backend
        | FailureKind::Cleanup => HostError::backend(error),
    }
}

pub(super) struct Installation {
    pub(super) cache: MarkerCache,
    service: Arc<TargetMarkingOperationService<TokioCleanupExecutor>>,
    creation: TokioCleanupExecutor,
    cleanup: TokioCleanupExecutor,
    #[cfg(feature = "retained-target-prevalidation")]
    preflight: Arc<FileInputPreflightOperationService<TokioCleanupExecutor>>,
    #[cfg(feature = "retained-target-prevalidation")]
    preflight_cleanup: TokioCleanupExecutor,
    #[cfg(feature = "retained-serialized-payload-mutation")]
    mutation: Arc<SerializedPayloadMutationOperationService<TokioCleanupExecutor>>,
    #[cfg(feature = "retained-serialized-payload-mutation")]
    mutation_cleanup: TokioCleanupExecutor,
}
impl Installation {
    fn backend_profile() -> Result<TargetMarkingProfile, HostError> {
        TargetMarkingProfile::new(
            ResourceScope::extension(183, 2),
            markers::WORLD_SLOTS,
            markers::CALLS,
        )
        .map_err(HostError::backend)
    }
    fn state_profile() -> Result<UploadProfile, HostError> {
        UploadProfile::new(
            ResourceScope::extension(183, 3),
            markers::WORLD_SLOTS,
            16 * 1024,
        )
        .map_err(HostError::backend)
    }
    fn call_profile() -> Result<UploadProfile, HostError> {
        UploadProfile::new(ResourceScope::extension(183, 4), markers::CALLS, 16 * 1024)
            .map_err(HostError::backend)
    }
    #[cfg(feature = "retained-target-prevalidation")]
    fn preflight_backend_profile() -> Result<RetainedPreflightProfile, HostError> {
        RetainedPreflightProfile::new(ResourceScope::extension(187, 1), markers::CALLS)
            .map_err(HostError::backend)
    }
    #[cfg(feature = "retained-target-prevalidation")]
    fn preflight_owner_profile() -> Result<UploadProfile, HostError> {
        UploadProfile::new(ResourceScope::extension(187, 2), markers::CALLS, 16 * 1024)
            .map_err(HostError::backend)
    }
    #[cfg(feature = "retained-serialized-payload-mutation")]
    fn mutation_backend_profile() -> Result<SerializedPayloadMutationProfile, HostError> {
        SerializedPayloadMutationProfile::new(ResourceScope::extension(191, 1), 1)
            .map_err(HostError::backend)
    }
    #[cfg(feature = "retained-serialized-payload-mutation")]
    fn mutation_owner_profile() -> Result<UploadProfile, HostError> {
        UploadProfile::new(ResourceScope::extension(191, 2), 1, 16 * 1024)
            .map_err(HostError::backend)
    }
    pub(super) fn requirements() -> Result<ResourceRequirements, HostError> {
        let requirements = MarkerCache::requirements()
            .combine_concurrent(Self::backend_profile()?.requirements())
            .combine_concurrent(Self::state_profile()?.requirements())
            .combine_concurrent(Self::call_profile()?.requirements());
        #[cfg(feature = "retained-target-prevalidation")]
        let requirements = requirements
            .combine_concurrent(Self::preflight_backend_profile()?.requirements())
            .combine_concurrent(Self::preflight_owner_profile()?.requirements());
        #[cfg(feature = "retained-serialized-payload-mutation")]
        let requirements = requirements
            .combine_concurrent(
                Self::mutation_backend_profile()?
                    .requirements()
                    .map_err(HostError::backend)?,
            )
            .combine_concurrent(Self::mutation_owner_profile()?.requirements());
        Ok(requirements)
    }
    pub(super) fn new(resources: &ProductionResourceLedger) -> Result<Self, HostError> {
        let cleanup = TokioCleanupExecutor::new(markers::WORLD_SLOTS + markers::CALLS);
        let creation = TokioCleanupExecutor::new(markers::WORLD_SLOTS);
        let backend = Arc::new(
            CdpTargetMarkingService::new(
                Self::backend_profile()?,
                resources,
                Arc::new(AtomicObjectIdSource::new(1).map_err(HostError::backend)?),
            )
            .map_err(HostError::backend)?,
        );
        let service = Arc::new(
            TargetMarkingOperationService::new(
                Self::state_profile()?,
                Self::call_profile()?,
                resources,
                backend,
                cleanup.clone(),
            )
            .map_err(HostError::backend)?,
        );
        let cache = MarkerCache::new(resources, service.clone(), creation.clone())
            .map_err(HostError::backend)?;
        #[cfg(feature = "retained-target-prevalidation")]
        let preflight_cleanup = TokioCleanupExecutor::new(markers::CALLS);
        #[cfg(feature = "retained-target-prevalidation")]
        let preflight = Arc::new(
            FileInputPreflightOperationService::new(
                Self::preflight_owner_profile()?,
                resources,
                Arc::new(
                    CdpFileInputPreflightService::new(
                        Self::preflight_backend_profile()?,
                        resources,
                    )
                    .map_err(HostError::backend)?,
                ),
                preflight_cleanup.clone(),
            )
            .map_err(HostError::backend)?,
        );
        #[cfg(feature = "retained-serialized-payload-mutation")]
        let mutation_cleanup = TokioCleanupExecutor::new(1);
        #[cfg(feature = "retained-serialized-payload-mutation")]
        let mutation = Arc::new(
            SerializedPayloadMutationOperationService::new(
                Self::mutation_owner_profile()?,
                resources,
                Arc::new(
                    CdpSerializedPayloadMutationService::new(
                        Self::mutation_backend_profile()?,
                        resources,
                    )
                    .map_err(HostError::backend)?,
                ),
                mutation_cleanup.clone(),
            )
            .map_err(HostError::backend)?,
        );
        Ok(Self {
            cache,
            service,
            creation,
            cleanup,
            #[cfg(feature = "retained-target-prevalidation")]
            preflight,
            #[cfg(feature = "retained-target-prevalidation")]
            preflight_cleanup,
            #[cfg(feature = "retained-serialized-payload-mutation")]
            mutation,
            #[cfg(feature = "retained-serialized-payload-mutation")]
            mutation_cleanup,
        })
    }
    pub(super) fn seal(&self) {
        self.cache.seal();
    }
    pub(super) fn prune(&self) {
        self.cache.prune();
    }
    pub(super) async fn changed(&self) {
        self.cache.changed().await;
    }
    pub(super) async fn shutdown(&self) -> Result<(), HostError> {
        // Called after the host's exact native-session retirement. Always
        // attempt both drains even when an earlier continuation failed.
        self.cache.close();
        let creation = self.creation.shutdown().await;
        let cleanup = self.cleanup.shutdown().await;
        #[cfg(feature = "retained-target-prevalidation")]
        let preflight_cleanup = self.preflight_cleanup.shutdown().await;
        #[cfg(feature = "retained-target-prevalidation")]
        let preflight_failed = preflight_cleanup.metrics.failed != 0
            || preflight_cleanup.metrics.timed_out != 0
            || preflight_cleanup.metrics.cancelled != 0
            || !self.preflight.is_cleanup_healthy();
        #[cfg(not(feature = "retained-target-prevalidation"))]
        let preflight_failed = false;
        #[cfg(feature = "retained-serialized-payload-mutation")]
        let mutation_cleanup = self.mutation_cleanup.shutdown().await;
        #[cfg(feature = "retained-serialized-payload-mutation")]
        let mutation_failed = mutation_cleanup.metrics.failed != 0
            || mutation_cleanup.metrics.timed_out != 0
            || mutation_cleanup.metrics.cancelled != 0
            || !self.mutation.is_cleanup_healthy();
        #[cfg(not(feature = "retained-serialized-payload-mutation"))]
        let mutation_failed = false;
        if creation.metrics.failed != 0
            || creation.metrics.timed_out != 0
            || creation.metrics.cancelled != 0
            || cleanup.metrics.failed != 0
            || cleanup.metrics.timed_out != 0
            || cleanup.metrics.cancelled != 0
            || !self.cache.is_cleanup_healthy()
            || !self.service.is_cleanup_healthy()
            || preflight_failed
            || mutation_failed
        {
            return Err(HostError::backend(
                "persistent target-marker cleanup failed",
            ));
        }
        Ok(())
    }
}
impl Drop for Installation {
    fn drop(&mut self) {
        // Failure teardown cannot await. Pre-admitted continuations retain
        // receipts and report failed native cleanup instead of inventing ACKs.
        self.cache.close();
    }
}

struct UtilityProof(ChromiumNativeUtilityWorldLifetime);
impl ObjectGroupDestructionProof for UtilityProof {
    fn all_group_worlds_destroyed(&self) -> bool {
        self.0.world_destroyed()
    }
    fn all_group_objects_retired(&self) -> bool {
        self.0.native_objects_retired()
    }
    fn poll_release_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        self.0.poll_session_release_ready(cx)
    }
}
struct MarkerWorldOwner {
    _snapshot: ChromiumUtilityWorldSnapshot,
}
impl markers::MarkerRoute for MarkerWorldOwner {
    fn is_current(&self) -> bool {
        self._snapshot.is_current().unwrap_or(false)
    }
}

fn independent_world(
    handles: &Handles,
    paired: &ChromiumFrameWorldSnapshot,
) -> Result<(MarkerWorld, Arc<dyn markers::MarkerRoute>), HostError> {
    let snapshot = handles
        .worlds
        .existing_frame_utility_world(paired.frame())
        .map_err(HostError::backend)?
        .ok_or(HostError::TargetClosed)?;
    if snapshot.page() != paired.page()
        || snapshot.frame() != paired.frame()
        || snapshot.utility_unique_id() != paired.utility_unique_id()
        || !Arc::ptr_eq(&snapshot.client(), &paired.client())
        || !snapshot.is_current().map_err(HostError::backend)?
    {
        return Err(HostError::TargetClosed);
    }
    let world = MarkerWorld {
        client: snapshot.client(),
        utility_world: Arc::from(snapshot.utility_unique_id()),
        destruction: snapshot
            .native_lifetime()
            .map(|proof| Arc::new(UtilityProof(proof)) as Arc<dyn ObjectGroupDestructionProof>),
    };
    // This owner deliberately contains no PendingHandle, Handle allowance,
    // upload lease, operation token, or short-lived paired-world receipt.
    Ok((
        world,
        Arc::new(MarkerWorldOwner {
            _snapshot: snapshot,
        }),
    ))
}

struct CallOwner {
    snapshot: ChromiumFrameWorldSnapshot,
    source: ChromiumRetainedElementSource,
    handle: Arc<PendingHandle>,
    marker: Arc<MarkerLease>,
    _admission: TrackedCallAdmission,
}
struct MarkTarget;
impl CommandSpec for MarkTarget {
    const METHOD: MethodTag = MethodTag::new(65008);
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = &[ELEMENT_HANDLE_OBJECT_KIND_ID];
    type Params = Params;
    type Result = RetainedMarkedFileInput;
    type ErrorDetails = TargetMarkPhaseError;
}
struct Params {
    handle: Arc<PendingHandle>,
    call_id: Arc<str>,
    admission: TrackedCallAdmission,
}
struct MarkService {
    handles: Arc<Handles>,
    files: Arc<file_input::Installation>,
}

fn host_error(error: HostError) -> OperationError<TargetMarkPhaseError> {
    let kind = match error {
        HostError::TargetClosed => FailureKind::TargetClosed,
        HostError::ResourceExhausted(_) => FailureKind::ResourceExhausted,
        _ => FailureKind::Backend,
    };
    OperationError::new(
        kind,
        "retained target marking failed",
        Some(TargetMarkPhaseError::Host(error)),
    )
}
fn marker_failure_kind(error: &MarkerOperationError) -> FailureKind {
    match error {
        MarkerOperationError::Cleanup { .. }
        | MarkerOperationError::Backend(TargetMarkingError::Cleanup { .. }) => FailureKind::Cleanup,
        MarkerOperationError::Owner(
            UploadError::Capacity
            | UploadError::Storage
            | UploadError::Resource(_)
            | UploadError::Admission(_),
        )
        | MarkerOperationError::Backend(
            TargetMarkingError::Capacity | TargetMarkingError::Budget(_),
        ) => FailureKind::ResourceExhausted,
        _ => FailureKind::Backend,
    }
}
fn mark_error(error: MarkerOperationError) -> OperationError<TargetMarkPhaseError> {
    let kind = marker_failure_kind(&error);
    OperationError::new(
        kind,
        "native target marking failed",
        Some(TargetMarkPhaseError::Marking(error)),
    )
}
fn cache_error(error: MarkerCacheError) -> OperationError<TargetMarkPhaseError> {
    let kind = match &error {
        MarkerCacheError::Cleanup => FailureKind::Cleanup,
        MarkerCacheError::Host(error) => match error.as_ref() {
            HostError::TargetClosed => FailureKind::TargetClosed,
            HostError::ResourceExhausted(_) => FailureKind::ResourceExhausted,
            _ => FailureKind::Backend,
        },
        MarkerCacheError::Creation(error) => marker_failure_kind(error),
    };
    OperationError::new(
        kind,
        "persistent target-marker acquisition failed",
        Some(TargetMarkPhaseError::Cache(error)),
    )
}
fn adoption_error(error: ResolutionError) -> OperationError<TargetMarkPhaseError> {
    let kind = match error {
        ResolutionError::Cleanup { .. } => FailureKind::Cleanup,
        ResolutionError::Resource(_) | ResolutionError::Admission(_) => {
            FailureKind::ResourceExhausted
        }
        _ => FailureKind::Backend,
    };
    OperationError::new(
        kind,
        "target-marker adoption failed",
        Some(TargetMarkPhaseError::Adoption(error)),
    )
}

impl TargetedService<Params> for MarkService {
    type Response = RetainedMarkedFileInput;
    type Error = OperationError<TargetMarkPhaseError>;
    type Future<'a> = Pin<Box<dyn Future<Output = RetainedTargetMarkOutcome> + Send + 'a>>;
    fn call_for(&self, target: ObjectId, call: Params) -> Self::Future<'_> {
        let work = async move {
            let snapshot = self
                .handles
                .worlds
                .snapshot(call.handle.initializer().parent_id)
                .await
                .map_err(|error| host_error(HostError::backend(error)))?;
            self.handles
                .owners
                .check_arguments([call.handle.as_ref()])
                .map_err(host_error)?;
            let source = self
                .handles
                .create
                .borrow_element_source(target)
                .map_err(|error| map_leaf_error(error).map_details(TargetMarkPhaseError::Host))?;
            check_world_parts(&snapshot, &source)
                .map_err(|error| error.map_details(TargetMarkPhaseError::Host))?;
            let key = MarkerKey::new(
                snapshot.page(),
                snapshot.frame(),
                snapshot.client(),
                Arc::from(snapshot.utility_unique_id()),
            )
            .map_err(cache_error)?;
            let marker = self
                .files
                .retained
                .markers
                .cache
                .acquire(key, || independent_world(&self.handles, &snapshot))
                .map_err(cache_error)?
                .await
                .map_err(cache_error)?;
            let owner = Arc::new(CallOwner {
                snapshot,
                source,
                handle: call.handle,
                marker: Arc::new(marker),
                _admission: call.admission,
            });
            check_world_parts(&owner.snapshot, &owner.source)
                .map_err(|error| error.map_details(TargetMarkPhaseError::Host))?;
            let request = RetainedElementRequest {
                source_client: owner.source.client(),
                source_object_id: Arc::from(owner.source.object_id()),
                client: owner.snapshot.client(),
                utility_world: Arc::from(owner.snapshot.utility_unique_id()),
                utility_execution_context_id: owner.snapshot.utility_execution_context_id(),
                destruction: owner.snapshot.native_lifetime().map(|proof| {
                    Arc::new(WorldProof(proof)) as Arc<dyn ObjectGroupDestructionProof>
                }),
            };
            let element = self
                .files
                .retained
                .attempts
                .call_retained_retaining(request, Some(owner.clone()))
                .await
                .map_err(adoption_error)?;
            let revalidated = self
                .handles
                .owners
                .check_arguments([owner.handle.as_ref()])
                .map_err(super::failure)
                .and_then(|()| check_world_parts(&owner.snapshot, &owner.source));
            if let Err(primary) = revalidated {
                return match element.release().await {
                    Ok(()) => Err(primary.map_details(TargetMarkPhaseError::Host)),
                    Err(cleanup) => Err(OperationError::new(
                        FailureKind::Cleanup,
                        "target revalidation and cleanup failed",
                        Some(TargetMarkPhaseError::RevalidationCleanup {
                            primary: Box::new(primary),
                            cleanup,
                        }),
                    )),
                };
            }
            let marked = owner
                .marker
                .mark(element, call.call_id)
                .map_err(cache_error)?
                .await
                .map_err(mark_error)?;
            Ok(RetainedMarkedFileInput {
                marked: Some(marked),
                #[cfg(feature = "retained-target-prevalidation")]
                preflight: self.files.retained.markers.preflight.clone(),
                #[cfg(feature = "retained-serialized-payload-mutation")]
                mutation: self.files.retained.markers.mutation.clone(),
            })
        };
        checked_work(work)
    }
}
fn checked_work<F: Future<Output = RetainedTargetMarkOutcome> + Send>(work: F) -> Pin<Box<F>> {
    // The tracked Handle call admits 8 MiB; bound this phase's fixed future,
    // call-id, wrapper and diagnostic overlap independently to 16 KiB.
    const {
        assert!(
            std::mem::size_of::<F>()
                + std::mem::size_of::<CallOwner>()
                + std::mem::size_of::<MarkerWorldOwner>()
                + 8192
                < 16 * 1024
        );
    }
    Box::pin(work)
}
impl CommandCancellation<MarkTarget> for MarkService {
    type Future<'a> = Ready<Outcome>;
    fn cancel(&self, _: OperationContext, _: CancellationReason) -> Self::Future<'_> {
        ready(Ok(()))
    }
}

impl HostSession {
    /// Mark and retain the exact adopted target for a later same-operation
    /// phase. No fresh timeout is started. Dropping the returned owner retains
    /// the pre-admitted native cleanup path through terminal group settlement.
    pub async fn handle_mark_target_retaining(
        &self,
        handle: Arc<PendingHandle>,
        call_id: &str,
    ) -> RetainedTargetMarkOutcome {
        if call_id.is_empty() {
            return Err(host_error(HostError::InvalidArgument(
                "retained target marking requires a non-empty public call ID".into(),
            )));
        }
        if !handle.is_element() || call_id.len() > 128 {
            return Err(host_error(HostError::InvalidArgument(
                "target marking needs an ElementHandle and a call ID of at most 128 bytes".into(),
            )));
        }
        let (target, runtime_call_id, _operation) = self
            .admit_operation(&handle.initializer().object_id.get().to_string())
            .map_err(host_error)?;
        let (runtime, handles, files) = {
            let state = self
                .state
                .lock()
                .map_err(|error| host_error(HostError::backend(error)))?;
            let live = state
                .as_ref()
                .ok_or_else(|| host_error(HostError::TargetClosed))?;
            (
                live.runtime.clone(),
                live.handles.clone(),
                live.file_input.clone(),
            )
        };
        let admission = handles
            .owners
            .admit_tracked_call(&handle)
            .map_err(host_error)?;
        let _caller_call = admission.call.clone();
        let service = Arc::new(MarkService { handles, files });
        let params = Params {
            handle,
            call_id: Arc::from(call_id),
            admission,
        };
        let command =
            Command::<MarkTarget>::new(runtime_call_id, target, CallMetadata::default(), params);
        let dispatch = runtime.dispatch_with(command, service.as_ref(), service.clone());
        tokio::select! {
            biased;
            () = self.stop.cancelled() => Err(host_error(HostError::TargetClosed)),
            outcome = dispatch => outcome.unwrap_or_else(|never| match never {}).into_outcome(),
        }
    }

    /// Execute only native original-target marking and temporary-group cleanup.
    /// The supplied call ID comes from the language protocol; the runtime's
    /// internal command ID is NOT substituted. No timeout is reset here. The
    /// caller owns one original deadline, exact FFI registration and subsequent
    /// publication/conversion phases. Dropping this future cancels the waiter,
    /// while separately admitted native creation/adoption/marking cleans up.
    pub async fn handle_mark_target(&self, handle: Arc<PendingHandle>, call_id: &str) -> Outcome {
        // Pinned _markAsTargetElement exits before evaluateInUtility when
        // metadata.id is absent. Do not create a world or inspect native state.
        if call_id.is_empty() {
            return Ok(());
        }
        self.handle_mark_target_retaining(handle, call_id)
            .await?
            .release()
            .await
    }
}

#[cfg(test)]
mod errors;
#[cfg(test)]
mod live;
