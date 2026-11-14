//! Exact retained-target submission. Node's staged language conversion and
//! ownerFrame preflight are separate callers, not simulated by this Rust API.

mod invocations;
#[cfg(feature = "retained-target-marking")]
pub(crate) mod markers;
#[cfg(feature = "retained-target-marking")]
mod marking;
#[cfg(feature = "retained-serialized-payload-mutation")]
pub use marking::serialized_payload_mutation_language_error;
#[cfg(feature = "retained-target-marking")]
pub use marking::{
    RetainedMarkedFileInput, RetainedTargetMarkOutcome, TargetMarkOutcome,
    target_mark_language_error,
};
#[cfg(feature = "retained-target-prevalidation")]
pub use marking::{
    RetainedPrevalidatedFileInput, RetainedTargetPrevalidationFlags,
    target_prevalidation_language_error,
};
#[cfg(feature = "retained-serialized-payload-import")]
pub(crate) mod serialized_payload;
mod timeout_policy;
#[cfg(feature = "retained-serialized-payload-import")]
pub use serialized_payload::{
    RetainedSerializedPayload, SerializedPayloadAdmission, SerializedPayloadMetrics,
    serialized_payload_limits,
};
pub use timeout_policy::{
    ScopedTimeoutSettings, TimeoutPolicyBudget, TimeoutPolicySession, TimeoutSettings,
};

use super::{
    PendingHandle,
    creation::{Installation as Handles, map_leaf_error},
    pending::{Allowance, TrackedCallAdmission},
};
use crate::{HostError, HostSession, ffi_budget::FfiRevoker, file_input};
use pwright_browser_model::ELEMENT_HANDLE_OBJECT_KIND_ID;
use pwright_composition_chromium::ChromiumFrameWorldSnapshot;
#[cfg(test)]
use pwright_composition_chromium_file_input::FilePayloadOperationService;
use pwright_composition_chromium_file_input::UploadError;
use pwright_composition_chromium_handle_evaluation::ChromiumRetainedElementSource;
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ObjectGroupDestructionProof,
    ResolutionError, ResolutionProfile, ResolutionRouteOwner, RetainedElementRequest,
};
use pwright_core::{
    AtomicObjectIdSource, CallMetadata, CancellationReason, Command, CommandCancellation,
    CommandSpec, FailureKind, MethodTag, ObjectId, ObjectKindId, OperationContext, OperationError,
    ProductionResourceLedger, ResourceRequirements, ResourceScope,
};
#[cfg(test)]
use pwright_file_input_source::PreparedPayloads;
use pwright_runtime::TargetedService;
use pwright_runtime_tokio::TokioCleanupExecutor;
use std::{
    future::{Future, Ready, pending, ready},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

type Outcome = Result<(), OperationError<HostError>>;

/// Separate from both the ordinary five-second cleanup executor and the
/// single upload continuation slot. An adopted owner holds this reservation
/// while the upload reserves its independently admitted continuation.
pub(crate) struct RetainedUploads {
    raw: Arc<CdpLocatorResolutionService>,
    attempts: ContinuableResolutionService<CdpLocatorResolutionService, TokioCleanupExecutor>,
    cleanup: TokioCleanupExecutor,
    invocations: invocations::Invocations,
    #[cfg(feature = "retained-target-marking")]
    markers: marking::Installation,
}

/// Move-only cancellation registration for this exact session and source.
/// It holds weak bookkeeping only, not a native Handle, work token, FFI result
/// or session pin. Keep it with the eventual native continuation through cleanup.
/// Registration does not authorize native use or prove a Node preparation scope.
pub struct RetainedFileInputRegistration {
    source: super::HandleSourceIdentity,
    registration: invocations::Registration,
}

impl RetainedUploads {
    fn profile() -> Result<ResolutionProfile, HostError> {
        ResolutionProfile::new(ResourceScope::extension(169, 1), 2 * super::MAX_CALLS)
            .and_then(ResolutionProfile::with_retained_reference)
            .map_err(HostError::backend)
    }

    pub(crate) fn requirements() -> Result<ResourceRequirements, HostError> {
        let requirements = Self::profile()?
            .requirements()
            .combine_concurrent(invocations::Invocations::requirements());
        #[cfg(feature = "retained-target-marking")]
        let requirements = requirements.combine_concurrent(marking::Installation::requirements()?);
        Ok(requirements)
    }

    pub(crate) fn new(resources: &ProductionResourceLedger) -> Result<Self, HostError> {
        let raw = Arc::new(
            CdpLocatorResolutionService::new(
                Self::profile()?,
                resources,
                Arc::new(AtomicObjectIdSource::new(1).map_err(HostError::backend)?),
            )
            .map_err(HostError::backend)?,
        );
        let cleanup = TokioCleanupExecutor::new(super::MAX_CALLS);
        Ok(Self {
            attempts: ContinuableResolutionService::new(raw.clone(), cleanup.clone()),
            raw,
            cleanup,
            invocations: invocations::Invocations::new(resources)?,
            #[cfg(feature = "retained-target-marking")]
            markers: marking::Installation::new(resources)?,
        })
    }

    /// Pure detachment only. The caller must leave all host locks before
    /// consuming or dropping the returned armed revocation batch.
    pub(crate) fn seal_invocations(&self) -> invocations::RevokeBatch {
        #[cfg(feature = "retained-target-marking")]
        self.markers.seal();
        self.invocations.seal_and_take()
    }

    #[cfg(feature = "retained-target-marking")]
    pub(crate) fn prune_markers(&self) {
        self.markers.prune();
    }

    #[cfg(feature = "retained-target-marking")]
    pub(crate) async fn markers_changed(&self) {
        self.markers.changed().await;
    }

    pub(super) fn register_invocation(
        &self,
        owners: &Arc<super::pending::Owners>,
        source: &super::HandleSourceIdentity,
        revoker: FfiRevoker,
    ) -> Result<RetainedFileInputRegistration, HostError> {
        // Provenance only: do not move disposal/world/DOM errors ahead of their
        // native execution boundary or add a second dispatch-registration gate.
        owners.check_file_input_source(source)?;
        Ok(RetainedFileInputRegistration {
            source: source.clone(),
            registration: self.invocations.register(revoker)?,
        })
    }

    pub(super) fn check_invocation(
        &self,
        owners: &Arc<super::pending::Owners>,
        source: &super::HandleSourceIdentity,
        registration: &RetainedFileInputRegistration,
    ) -> Result<(), HostError> {
        owners.check_file_input_source(source)?;
        owners.check_file_input_source(&registration.source)?;
        if source.target() != registration.source.target()
            || source.creation_frame_id() != registration.source.creation_frame_id()
            || source.creation_page_id() != registration.source.creation_page_id()
        {
            return Err(HostError::InvalidArgument(
                "Retained upload registration belongs to another source".into(),
            ));
        }
        self.invocations.check(&registration.registration)
    }

    pub(crate) async fn shutdown(&self) -> Result<(), HostError> {
        // Standalone/failure fallback; the assembled host must also revoke
        // before waiting for operation and Handle work, not only here.
        self.seal_invocations().revoke();
        #[cfg(feature = "retained-target-marking")]
        let marking = self.markers.shutdown().await;
        let report = self.cleanup.shutdown().await;
        let adoption = if report.metrics.failed != 0
            || report.metrics.timed_out != 0
            || !self.raw.is_cleanup_healthy()
        {
            Err(HostError::backend(
                "retained upload adoption cleanup failed",
            ))
        } else {
            Ok(())
        };
        #[cfg(feature = "retained-target-marking")]
        return file_input::combine_native_cleanup(marking, adoption);
        #[cfg(not(feature = "retained-target-marking"))]
        adoption
    }

    async fn run_prepared<R: Send + 'static>(
        &self,
        files: &file_input::Installation,
        request: RetainedElementRequest,
        input: (file_input::PreparedFileInputKind, R),
        retained: ResolutionRouteOwner,
        revalidate: impl FnOnce() -> Result<(), OperationError<HostError>>,
    ) -> Outcome {
        let element = self
            .attempts
            .call_retained_retaining(request, Some(retained))
            .await
            .map_err(resolution_error)?;
        // Cancellation above continues ONLY acquisition and group cleanup.
        // It must never advance to a previously unsubmitted upload mutation.
        if let Err(primary) = revalidate() {
            return match element.release().await {
                Ok(()) => Err(primary),
                Err(cleanup) => Err(OperationError::new(
                    FailureKind::Cleanup,
                    "retained upload revalidation and group cleanup failed",
                    Some(HostError::backend(format_args!("{primary}; {cleanup}"))),
                )),
            };
        }
        match input.0 {
            file_input::PreparedFileInputKind::Payloads(payloads) => files
                .upload
                .prepare_retaining(element, payloads, input.1)
                .map_err(upload_error)?
                .await
                .map(|_| ())
                .map_err(upload_error),
            #[cfg(feature = "file-input-native-paths")]
            file_input::PreparedFileInputKind::Native(prepared) => files
                .native_paths
                .apply(element, prepared, input.1)
                .await
                .map_err(failure),
        }
    }

    #[cfg(test)]
    async fn run<R: Send + 'static>(
        &self,
        upload: &FilePayloadOperationService<TokioCleanupExecutor>,
        request: RetainedElementRequest,
        input: (PreparedPayloads, R),
        retained: ResolutionRouteOwner,
        revalidate: impl FnOnce() -> Result<(), OperationError<HostError>>,
    ) -> Outcome {
        let element = self
            .attempts
            .call_retained_retaining(request, Some(retained))
            .await
            .map_err(resolution_error)?;
        if let Err(primary) = revalidate() {
            return match element.release().await {
                Ok(()) => Err(primary),
                Err(cleanup) => Err(OperationError::new(
                    FailureKind::Cleanup,
                    "retained upload revalidation and group cleanup failed",
                    Some(HostError::backend(format_args!("{primary}; {cleanup}"))),
                )),
            };
        }
        upload
            .prepare_retaining(element, input.0, input.1)
            .map_err(upload_error)?
            .await
            .map(|_| ())
            .map_err(upload_error)
    }
}

fn failure(error: HostError) -> OperationError<HostError> {
    OperationError::new(
        FailureKind::Backend,
        "retained file input failed",
        Some(error),
    )
}

fn resolution_error(error: ResolutionError) -> OperationError<HostError> {
    let kind = match &error {
        ResolutionError::Cleanup { .. } => FailureKind::Cleanup,
        ResolutionError::Admission(_) | ResolutionError::Resource(_) => {
            FailureKind::ResourceExhausted
        }
        _ => FailureKind::Backend,
    };
    OperationError::new(
        kind,
        "retained element adoption failed",
        Some(HostError::backend(error)),
    )
}

fn upload_error(error: UploadError) -> OperationError<HostError> {
    let kind = if matches!(error, UploadError::Cleanup { .. }) {
        FailureKind::Cleanup
    } else {
        FailureKind::Backend
    };
    OperationError::new(
        kind,
        "retained file input failed",
        Some(HostError::backend(error)),
    )
}

struct WorldProof(pwright_composition_chromium::ChromiumNativeWorldLifetime);
impl ObjectGroupDestructionProof for WorldProof {
    fn all_group_worlds_destroyed(&self) -> bool {
        self.0.both_worlds_destroyed()
    }
    fn all_group_objects_retired(&self) -> bool {
        self.0.native_objects_retired()
    }
    fn poll_release_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        self.0.poll_session_release_ready(cx)
    }
}

struct RouteOwner {
    snapshot: ChromiumFrameWorldSnapshot,
    source: ChromiumRetainedElementSource,
    handle: Arc<PendingHandle>,
    _call: Arc<Allowance>,
    _work: tokio_util::task::task_tracker::TaskTrackerToken,
    _upload_call: file_input::CallLease,
}

fn check_world(owner: &RouteOwner) -> Result<(), OperationError<HostError>> {
    check_world_parts(&owner.snapshot, &owner.source)
}

fn check_world_parts(
    snapshot: &ChromiumFrameWorldSnapshot,
    source: &ChromiumRetainedElementSource,
) -> Result<(), OperationError<HostError>> {
    if snapshot.page() != source.owner_page()
        || snapshot.frame() != source.owner_frame()
        || snapshot.main_execution_context_id() != source.creation_execution_context_id()
        || snapshot.main_unique_id() != source.creation_unique_context_id()
        || !Arc::ptr_eq(&snapshot.client(), &source.client())
        || !snapshot
            .is_current()
            .map_err(|error| failure(HostError::backend(error)))?
    {
        return Err(failure(HostError::TargetClosed));
    }
    Ok(())
}

struct SetRetainedFiles;
impl CommandSpec for SetRetainedFiles {
    const METHOD: MethodTag = MethodTag::new(65007);
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = &[ELEMENT_HANDLE_OBJECT_KIND_ID];
    type Params = Params;
    type Result = ();
    type ErrorDetails = HostError;
}

struct Params {
    handle: Arc<PendingHandle>,
    input: (file_input::PreparedFileInputKind, file_input::SourceSlot),
    admission: TrackedCallAdmission,
    upload_call: file_input::CallLease,
}

struct RetainedUploadService {
    handles: Arc<Handles>,
    files: Arc<file_input::Installation>,
}

impl TargetedService<Params> for RetainedUploadService {
    type Response = ();
    type Error = OperationError<HostError>;
    type Future<'a> = Pin<Box<dyn Future<Output = Outcome> + Send + 'a>>;

    fn call_for(&self, target: ObjectId, call: Params) -> Self::Future<'_> {
        let work = async move {
            // Upstream's server executes in the ORIGINAL creation Frame's
            // utility world. Client ownerFrame chooses conversion Context only.
            let snapshot = self
                .handles
                .worlds
                .snapshot(call.handle.initializer().parent_id)
                .await
                .map_err(|error| failure(HostError::backend(error)))?;
            self.handles
                .owners
                .check_arguments([call.handle.as_ref()])
                .map_err(failure)?;
            let source = self
                .handles
                .create
                .borrow_element_source(target)
                .map_err(map_leaf_error)?;
            let TrackedCallAdmission {
                call: allowance,
                work,
            } = call.admission;
            let owner = Arc::new(RouteOwner {
                snapshot,
                source,
                handle: call.handle,
                _call: allowance,
                _work: work,
                _upload_call: call.upload_call,
            });
            check_world(&owner)?;
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
            self.files
                .retained
                .run_prepared(
                    self.files.as_ref(),
                    request,
                    call.input,
                    owner.clone(),
                    || {
                        self.handles
                            .owners
                            .check_arguments([owner.handle.as_ref()])
                            .map_err(failure)?;
                        self.handles
                            .create
                            .validate_element_source(target, &owner.source)
                            .map_err(map_leaf_error)?;
                        check_world(&owner)
                    },
                )
                .await
        };
        // Heap-owned IDs, route projections and both future layers fit the
        // already-held Handle call allowance; payload bytes have their own lease.
        const {
            assert!(std::mem::size_of::<RouteOwner>() + 4096 < super::CREATION_FUTURE_BYTES);
        }
        check_future(&work);
        Box::pin(work)
    }
}

fn check_future<F>(_: &F) {
    const {
        assert!(std::mem::size_of::<F>() + 4096 < super::CREATION_FUTURE_BYTES);
    }
}

/// Registration policy, separate from session availability and native-world
/// routing. The injected reader is asked only about the immutable creation Page.
pub(super) fn check_dispatch_registration(
    owners: &Arc<super::pending::Owners>,
    source: &super::HandleSourceIdentity,
    page_exists: impl FnOnce(ObjectId) -> Result<bool, HostError>,
) -> Result<(), HostError> {
    owners.check_file_input_dispatch_source(source)?;
    let page = source.creation_page_id().ok_or_else(|| {
        HostError::InvalidArgument("retained file input requires creation-Page identity".into())
    })?;
    if !page_exists(page)? {
        return Err(HostError::TargetClosed);
    }
    Ok(())
}

impl CommandCancellation<SetRetainedFiles> for RetainedUploadService {
    type Future<'a> = Ready<Outcome>;
    fn cancel(&self, _: OperationContext, _: CancellationReason) -> Self::Future<'_> {
        // Each native phase owns cleanup. Never resume the whole workflow.
        ready(Ok(()))
    }
}

impl HostSession {
    /// Attach cancellation authority immediately before the first native
    /// phase, not during the separately bounded language-only preparation.
    /// The caller must first verify its exact foreign preparation scope. This
    /// registers only a weak FFI receipt; actual native use is revalidated later.
    pub fn register_retained_file_input(
        &self,
        source: &super::HandleSourceIdentity,
        revoker: FfiRevoker,
    ) -> Result<RetainedFileInputRegistration, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state
            .as_ref()
            .filter(|_| self.is_connected())
            .ok_or(HostError::TargetClosed)?;
        live.file_input
            .retained
            .register_invocation(&live.handles.owners, source, revoker)
    }

    /// Check exact registration/source binding, not native liveness or DOM.
    pub fn check_retained_file_input_registration(
        &self,
        source: &super::HandleSourceIdentity,
        registration: &RetainedFileInputRegistration,
    ) -> Result<(), HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state
            .as_ref()
            .filter(|_| self.is_connected())
            .ok_or(HostError::TargetClosed)?;
        live.file_input
            .retained
            .check_invocation(&live.handles.owners, source, registration)
    }

    /// Validate only exact session provenance and Element kind for preparation.
    /// This synchronous check creates no task/native owner and does not inspect
    /// disposal, current execution worlds or DOM ownership. It grants no native
    /// use; `handle_set_input_files` still revalidates before adoption/mutation.
    ///
    /// A closed session rejects preparation because its scope is closing. The
    /// eventual Node API must separately preserve its public phase/error order.
    pub fn validate_retained_file_input_source(
        &self,
        source: &super::HandleSourceIdentity,
    ) -> Result<(), HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state
            .as_ref()
            .filter(|_| !self.stop.is_cancelled())
            .ok_or(HostError::TargetClosed)?;
        live.handles.owners.check_file_input_source(source)
    }

    /// Point-in-time registration check before server language work. This
    /// creates no operation, task, lease, native lookup or execution-world pin.
    /// Check it again after any instrumentation-equivalent async checkpoint.
    ///
    /// The identity is only a lookup key: exact live owner-ticket membership
    /// and the creation Page are checked now. Do not check the Handle or Frame
    /// runtime entry here. Navigation/frame detach may retire those earlier
    /// than upstream removes its dispatcher; native execution checks them later.
    pub fn check_retained_file_input_dispatch(
        &self,
        source: &super::HandleSourceIdentity,
    ) -> Result<(), HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state
            .as_ref()
            .filter(|_| !self.stop.is_cancelled() && !self.disconnected.is_cancelled())
            .ok_or(HostError::TargetClosed)?;
        check_dispatch_registration(&live.handles.owners, source, |page| {
            live.runtime
                .is_object_live(page)
                .map_err(HostError::backend)
        })
    }

    /// Submit already-materialized input to this exact retained ElementHandle.
    /// This is not the public Node conversion/preflight API. A disposed source
    /// can win before adoption; descriptor Arcs are deliberately not native pins.
    pub async fn handle_set_input_files(
        &self,
        handle: Arc<PendingHandle>,
        input: file_input::PreparedFileInput,
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        if !handle.is_element() {
            return Err(HostError::InvalidArgument(
                "setInputFiles requires an ElementHandle".into(),
            ));
        }
        #[cfg(feature = "page-bindings")]
        let frame = handle.initializer().parent_id.get().to_string();
        let deadline_at =
            tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        // This public-waiter token MUST NOT enter the native continuation:
        // shutdown waits it before retiring inspector sessions.
        let (target, call_id, _operation) =
            self.admit_operation(&handle.initializer().object_id.get().to_string())?;
        let (runtime, handles, files) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            (
                live.runtime.clone(),
                live.handles.clone(),
                live.file_input.clone(),
            )
        };
        let admission = handles.owners.admit_tracked_call(&handle)?;
        let _caller_call = admission.call.clone();
        let service = Arc::new(RetainedUploadService { handles, files });
        let params = Params {
            handle,
            input: service.files.take_input(input)?,
            admission,
            upload_call: service.files.admit()?,
        };
        let command =
            Command::<SetRetainedFiles>::new(call_id, target, CallMetadata::default(), params);
        let dispatch = runtime.dispatch_with(command, service.as_ref(), service.clone());
        let deadline = async {
            if timeout_ms == 0 {
                pending::<()>().await;
            }
            tokio::time::sleep_until(deadline_at).await;
        };
        let outcome = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err(HostError::TargetClosed),
            () = deadline => return Err(HostError::Timeout(timeout_ms)),
            outcome = dispatch => outcome.unwrap_or_else(|never| match never {}).into_outcome(),
        };
        let cleanup_failed = outcome
            .as_ref()
            .is_err_and(|error| error.kind() == FailureKind::Cleanup);
        let outcome = outcome.map_err(|error| {
            let mut detail = None;
            let error = error.map_details(|value| {
                detail = Some(value);
            });
            detail.unwrap_or_else(|| HostError::operation(error, timeout_ms))
        });
        if cleanup_failed {
            if self.is_connected() {
                let _ = self.flush_topology().await;
            }
            return outcome;
        }
        let outcome = crate::completion::publish_command_result(
            outcome,
            self.is_connected(),
            self.flush_topology(),
        )
        .await;
        #[cfg(feature = "page-bindings")]
        if outcome.is_ok() {
            self.flush_page_binding_publications(&frame).await?;
        }
        outcome
    }
}

#[cfg(test)]
mod live;
#[cfg(test)]
mod tests;
