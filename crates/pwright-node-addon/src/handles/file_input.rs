//! Selected Node-thread preparation ownership, not public upload conversion.
//! No browser work, filesystem access, Buffer pointer or tracked future here.
use super::{NativeHandle, publication};
use crate::{
    NativeSession, binding_error, evaluation_value::status, node_reference::NodeReference,
};
use napi::{
    Env, JsValue, Unknown,
    bindgen_prelude::{JsObjectValue, Object, PromiseRaw, ToNapiValue},
    sys,
};
use napi_derive::napi;
#[cfg(feature = "retained-serialized-payload-import")]
use pwright_node_host::handles::RetainedSerializedPayload;
#[cfg(feature = "retained-serialized-payload-mutation")]
use pwright_node_host::handles::serialized_payload_mutation_language_error;
#[cfg(feature = "retained-target-prevalidation")]
use pwright_node_host::handles::{
    RetainedPrevalidatedFileInput, RetainedTargetPrevalidationFlags,
    target_prevalidation_language_error,
};
#[cfg(feature = "retained-target-marking")]
use pwright_node_host::{
    FramePublicationFence,
    handles::{
        RetainedFileInputRegistration, RetainedMarkedFileInput, TargetMarkOutcome,
        target_mark_language_error,
    },
};
use pwright_node_host::{
    HostError,
    ffi_budget::FfiBudget,
    handles::{HandleSourceIdentity, TimeoutPolicySession, TimeoutSettings},
};
#[cfg(feature = "retained-target-marking")]
use std::sync::{Arc, Mutex};
use std::{cell::RefCell, ptr, rc::Rc};

mod argument_assembly;
mod channel_validation;
mod materialization;
mod original_handle;
mod preflight;
mod projection;
mod registry;
#[cfg(feature = "retained-serialized-payload-import")]
mod serialized_payload;
mod timeout_policy;
use original_handle::OriginalHandle;
pub(crate) use timeout_policy::NativeFileInputTimeoutSettings;

#[cfg(feature = "retained-target-marking")]
type PreparationProgress = NativeMarkProgress;
#[cfg(not(feature = "retained-target-marking"))]
type PreparationProgress = ();

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Initial,
    WaitingOwner,
    ContextReady,
    WaitingConversion,
    Ready,
    ArgumentsReady,
    ChannelReady,
    ServerDispatchReady,
    ServerParamsReady,
    #[cfg(feature = "retained-target-marking")]
    CallReady,
    ServerMaterializationReady,
    WaitingMaterialization,
    Materialized,
    #[cfg(feature = "retained-target-marking")]
    Marked,
    #[cfg(feature = "retained-target-marking")]
    Projected,
    #[cfg(feature = "retained-target-prevalidation")]
    Prevalidated,
    #[cfg(feature = "retained-input-instrumentation")]
    WaitingInputAction,
    #[cfg(feature = "retained-input-instrumentation")]
    Instrumented,
    #[cfg(feature = "retained-call-argument-serialization")]
    Serialized,
    #[cfg(feature = "retained-serialized-payload-import")]
    PayloadImported,
    #[cfg(feature = "retained-serialized-payload-mutation")]
    Mutated,
}

#[cfg(feature = "retained-target-marking")]
#[derive(Clone, Copy)]
struct ProjectionFlags {
    multiple: bool,
    directory_upload: bool,
}

#[derive(Clone)]
struct InputRecord {
    source: HandleSourceIdentity,
    original_handle: OriginalHandle,
    timeout_settings: TimeoutSettings,
    #[cfg(feature = "retained-target-marking")]
    protocol_call: Option<u64>,
    #[cfg(feature = "retained-target-marking")]
    projection: Option<ProjectionFlags>,
    #[cfg(feature = "retained-serialized-payload-import")]
    serialized_payload: Option<Arc<RetainedSerializedPayload>>,
    // Private own-data array: receiver/files/options, then context, items and
    // awaited conversion. A root-count bound is not a reachable V8 byte bound.
    arguments: Rc<NodeReference>,
    roots: u32,
    stage: Stage,
}

struct Snapshot<'env> {
    source: HandleSourceIdentity,
    original_handle: OriginalHandle,
    timeout_settings: TimeoutSettings,
    #[cfg(feature = "retained-target-marking")]
    protocol_call: Option<u64>,
    #[cfg(feature = "retained-target-marking")]
    projection: Option<ProjectionFlags>,
    #[cfg(feature = "retained-serialized-payload-import")]
    serialized_payload: Option<Arc<RetainedSerializedPayload>>,
    arguments: Unknown<'env>,
}

struct RootSnapshot {
    source: HandleSourceIdentity,
    original_handle: OriginalHandle,
    timeout_settings: TimeoutSettings,
    #[cfg(feature = "retained-target-marking")]
    protocol_call: Option<u64>,
    #[cfg(feature = "retained-target-marking")]
    projection: Option<ProjectionFlags>,
    #[cfg(feature = "retained-serialized-payload-import")]
    serialized_payload: Option<Arc<RetainedSerializedPayload>>,
    arguments: Rc<NodeReference>,
}

impl InputRecord {
    /// Pure Rust inspection only; even reference lookup happens after borrowing.
    fn snapshot(&self, expected: Stage) -> napi::Result<RootSnapshot> {
        if self.stage != expected {
            return Err(binding_error(HostError::InvalidArgument(
                "Upload preparation phase is out of order".into(),
            )));
        }
        Ok(RootSnapshot {
            source: self.source.clone(),
            original_handle: self.original_handle.clone(),
            timeout_settings: self.timeout_settings.clone(),
            #[cfg(feature = "retained-target-marking")]
            protocol_call: self.protocol_call,
            #[cfg(feature = "retained-target-marking")]
            projection: self.projection,
            #[cfg(feature = "retained-serialized-payload-import")]
            serialized_payload: self.serialized_payload.clone(),
            arguments: Rc::clone(&self.arguments),
        })
    }

    #[cfg(feature = "retained-target-marking")]
    fn native_snapshot(&self) -> Result<NativeSnapshot, HostError> {
        if self.stage != Stage::Materialized {
            return Err(HostError::InvalidArgument(
                "Upload preparation phase is out of order".into(),
            ));
        }
        Ok(NativeSnapshot {
            source: self.source.clone(),
            original_handle: self.original_handle.clone(),
            protocol_call: self.protocol_call.ok_or_else(|| {
                HostError::InvalidArgument(
                    "Upload preparation has no public protocol call identity".into(),
                )
            })?,
        })
    }

    #[cfg(feature = "retained-target-prevalidation")]
    fn prevalidation_snapshot(&self) -> Result<NativePrevalidationSnapshot, HostError> {
        if self.stage != Stage::Projected {
            return Err(HostError::InvalidArgument(
                "Upload preparation phase is out of order".into(),
            ));
        }
        let projection = self.projection.ok_or_else(|| {
            HostError::InvalidArgument("Upload preparation has no carrier projection".into())
        })?;
        Ok(NativePrevalidationSnapshot {
            source: self.source.clone(),
            flags: RetainedTargetPrevalidationFlags {
                multiple: projection.multiple,
                directory_upload: projection.directory_upload,
            },
        })
    }

    #[cfg(feature = "retained-input-instrumentation")]
    fn instrumentation_snapshot(&self, expected: Stage) -> napi::Result<RootSnapshot> {
        self.snapshot(expected)
    }

    #[cfg(feature = "retained-serialized-payload-mutation")]
    fn mutation_snapshot(&self) -> Result<NativeMutationSnapshot, HostError> {
        if self.stage != Stage::PayloadImported || self.roots != 0 {
            return Err(HostError::InvalidArgument(
                "Upload preparation is not ready for serialized-payload mutation".into(),
            ));
        }
        Ok(NativeMutationSnapshot {
            source: self.source.clone(),
            serialized_payload: self.serialized_payload.clone().ok_or_else(|| {
                HostError::InvalidArgument(
                    "Upload preparation has no retained serialized payload".into(),
                )
            })?,
        })
    }
}

#[cfg(feature = "retained-target-marking")]
struct NativeSnapshot {
    source: HandleSourceIdentity,
    original_handle: OriginalHandle,
    protocol_call: u64,
}

#[cfg(feature = "retained-target-prevalidation")]
struct NativePrevalidationSnapshot {
    source: HandleSourceIdentity,
    flags: RetainedTargetPrevalidationFlags,
}

#[cfg(feature = "retained-serialized-payload-mutation")]
struct NativeMutationSnapshot {
    source: HandleSourceIdentity,
    serialized_payload: Arc<RetainedSerializedPayload>,
}

#[cfg(feature = "retained-target-marking")]
struct NativeMarkProgress {
    // The parked FFI result owns the registration and completion until the
    // exact Node-thread phase commits or aborts.
    _state: Arc<NativeMarkState>,
}

#[cfg(feature = "retained-target-marking")]
struct NativeMarkState {
    registration: RetainedFileInputRegistration,
    completion: Mutex<Option<Result<NativeMarkCompletion, HostError>>>,
}

#[cfg(feature = "retained-target-marking")]
struct NativeMarkCompletion {
    outcome: Option<NativeMarkOutcome>,
    retained: Option<RetainedMarkedFileInput>,
    fence: Option<FramePublicationFence>,
    #[cfg(feature = "retained-target-prevalidation")]
    prevalidation: Option<Result<(), HostError>>,
    #[cfg(feature = "retained-target-prevalidation")]
    validated: Option<RetainedPrevalidatedFileInput>,
}

#[cfg(feature = "retained-target-marking")]
enum NativeMarkOutcome {
    Host(HostError),
    Target(TargetMarkOutcome),
}

#[cfg(feature = "retained-target-marking")]
impl NativeMarkState {
    fn complete(
        &self,
        completion: Result<NativeMarkCompletion, HostError>,
    ) -> Result<(), HostError> {
        let mut slot = self.completion.lock().map_err(|_| {
            HostError::Backend("Native target-mark completion mutex is poisoned".into())
        })?;
        if slot.is_some() {
            return Err(HostError::Backend(
                "Native target-mark completion was written twice".into(),
            ));
        }
        *slot = Some(completion);
        Ok(())
    }

    fn take_fence(&self) -> Result<FramePublicationFence, HostError> {
        let mut slot = self.completion.lock().map_err(|_| {
            HostError::Backend("Native target-mark completion mutex is poisoned".into())
        })?;
        match slot.as_mut() {
            Some(Ok(completion)) => completion.fence.take().ok_or_else(|| {
                HostError::Backend("Native target-mark publication fence was already taken".into())
            }),
            Some(Err(_)) => match slot.take() {
                Some(Err(error)) => Err(error),
                _ => unreachable!("checked failed native completion remains under lock"),
            },
            None => Err(HostError::Backend(
                "Native target-mark completion is missing".into(),
            )),
        }
    }

    fn finish_outcome(&self) -> Result<(), HostError> {
        let mut slot = self.completion.lock().map_err(|_| {
            HostError::Backend("Native target-mark completion mutex is poisoned".into())
        })?;
        let completion = slot
            .as_mut()
            .ok_or_else(|| HostError::Backend("Native target-mark completion is missing".into()))?
            .as_mut()
            .map_err(|_| {
                HostError::Backend("Native target-mark completion failed before publication".into())
            })?;
        if completion.fence.is_some() {
            return Err(HostError::Backend(
                "Native target-mark outcome preceded ordered publication".into(),
            ));
        }
        let outcome = completion.outcome.take().ok_or_else(|| {
            HostError::Backend("Native target-mark outcome was already consumed".into())
        })?;
        match outcome {
            NativeMarkOutcome::Host(error) => {
                *slot = None;
                Err(error)
            }
            NativeMarkOutcome::Target(Err(error)) => {
                *slot = None;
                Err(target_mark_language_error(error))
            }
            NativeMarkOutcome::Target(Ok(())) => {
                if completion.retained.is_none() {
                    return Err(HostError::Backend(
                        "Successful target marking did not retain its native owner".into(),
                    ));
                }
                Ok(())
            }
        }
    }

    #[cfg(feature = "retained-target-prevalidation")]
    fn take_marked_for_prevalidation(&self) -> Result<RetainedMarkedFileInput, HostError> {
        let mut slot = self.completion.lock().map_err(|_| {
            HostError::Backend("Native target-mark completion mutex is poisoned".into())
        })?;
        let completion = slot
            .as_mut()
            .ok_or_else(|| HostError::Backend("Native target-mark completion is missing".into()))?
            .as_mut()
            .map_err(|_| {
                HostError::Backend("Native target-mark completion failed before projection".into())
            })?;
        if completion.fence.is_some()
            || completion.outcome.is_some()
            || completion.prevalidation.is_some()
            || completion.validated.is_some()
        {
            return Err(HostError::InvalidArgument(
                "Native target is not ready for retained prevalidation".into(),
            ));
        }
        completion.retained.take().ok_or_else(|| {
            HostError::InvalidArgument("Native marked target was already consumed".into())
        })
    }

    #[cfg(feature = "retained-target-prevalidation")]
    fn complete_prevalidation(
        &self,
        result: Result<RetainedPrevalidatedFileInput, HostError>,
    ) -> Result<(), HostError> {
        let mut slot = self.completion.lock().map_err(|_| {
            HostError::Backend("Native target-mark completion mutex is poisoned".into())
        })?;
        let completion = slot
            .as_mut()
            .ok_or_else(|| HostError::Backend("Native target-mark completion is missing".into()))?
            .as_mut()
            .map_err(|_| {
                HostError::Backend("Native target-mark completion failed before projection".into())
            })?;
        if completion.prevalidation.is_some() || completion.validated.is_some() {
            return Err(HostError::Backend(
                "Native target prevalidation completed twice".into(),
            ));
        }
        match result {
            Ok(validated) => {
                completion.prevalidation = Some(Ok(()));
                completion.validated = Some(validated);
            }
            Err(error) => completion.prevalidation = Some(Err(error)),
        }
        Ok(())
    }

    #[cfg(feature = "retained-target-prevalidation")]
    fn finish_prevalidation(&self) -> Result<(), HostError> {
        let mut slot = self.completion.lock().map_err(|_| {
            HostError::Backend("Native target-mark completion mutex is poisoned".into())
        })?;
        let completion = slot
            .as_mut()
            .ok_or_else(|| HostError::Backend("Native target-mark completion is missing".into()))?
            .as_mut()
            .map_err(|_| {
                HostError::Backend("Native target-mark completion failed before projection".into())
            })?;
        match completion.prevalidation.take().ok_or_else(|| {
            HostError::Backend("Native target prevalidation completion is missing".into())
        })? {
            Ok(()) if completion.validated.is_some() => Ok(()),
            Ok(()) => Err(HostError::Backend(
                "Native target prevalidation did not retain its owner".into(),
            )),
            Err(error) => {
                *slot = None;
                Err(error)
            }
        }
    }

    #[cfg(feature = "retained-serialized-payload-mutation")]
    fn take_prevalidated_for_mutation(&self) -> Result<RetainedPrevalidatedFileInput, HostError> {
        let mut slot = self.completion.lock().map_err(|_| {
            HostError::Backend("Native target-mark completion mutex is poisoned".into())
        })?;
        let completion = slot
            .as_mut()
            .ok_or_else(|| HostError::Backend("Native target-mark completion is missing".into()))?
            .as_mut()
            .map_err(|_| {
                HostError::Backend("Native target-mark completion failed before mutation".into())
            })?;
        if completion.fence.is_some()
            || completion.outcome.is_some()
            || completion.retained.is_some()
            || completion.prevalidation.is_some()
        {
            return Err(HostError::InvalidArgument(
                "Native target is not ready for serialized-payload mutation".into(),
            ));
        }
        completion.validated.take().ok_or_else(|| {
            HostError::InvalidArgument("Native prevalidated target was already consumed".into())
        })
    }
}

impl RootSnapshot {
    fn scoped(self, env: &Env) -> napi::Result<Snapshot<'_>> {
        // Protect the reference only through lookup. Release that temporary
        // owner before every language callback, leaving ordinary scoped handles.
        let arguments = self.arguments.get(env)?;
        drop(self.arguments);
        Ok(Snapshot {
            source: self.source,
            original_handle: self.original_handle,
            timeout_settings: self.timeout_settings,
            #[cfg(feature = "retained-target-marking")]
            protocol_call: self.protocol_call,
            #[cfg(feature = "retained-target-marking")]
            projection: self.projection,
            #[cfg(feature = "retained-serialized-payload-import")]
            serialized_payload: self.serialized_payload,
            arguments,
        })
    }
}

impl<'env> Snapshot<'env> {
    fn at(&self, env: &'env Env, index: u32) -> napi::Result<Unknown<'env>> {
        let mut value = ptr::null_mut();
        // These inaccessible arrays contain only our own data properties;
        // fixed indices are selected by the already-checked stage, not users.
        status(unsafe {
            sys::napi_get_element(env.raw(), self.arguments.raw(), index, &mut value)
        })?;
        Ok(unsafe { Unknown::from_raw_unchecked(env.raw(), value) })
    }
}

#[derive(Clone)]
pub(crate) struct FileInputPreparations(registry::Registry<InputRecord>);
#[derive(Clone)]
pub(crate) struct PreparationScope(registry::Scope<InputRecord>, Rc<RefCell<TimeoutPolicySlot>>);

enum TimeoutPolicySlot {
    Uninitialized,
    Open(TimeoutPolicySession),
    Closed,
}

// The existing registry's 4096-byte administrative slack covers the sixteen
// live scope-slot allocations; policy index/cell backing has separate leases.
// Retained closed JS/weak tombstones are not bounded by the live-scope limit.
const _: () = assert!(
    16 * (std::mem::size_of::<RefCell<TimeoutPolicySlot>>() + 2 * std::mem::size_of::<usize>())
        <= 4096
);

// The runtime Registry<InputRecord> check includes all 32 enlarged entries in
// its unchanged 16 KiB administrative allowance. Replacement/snapshot overlap
// must also fit the independent 16 KiB phase allowance: a reentrant callback can
// revoke the 1 MiB FFI admission while synchronous temporaries still survive.
// Weak capture/clone allocates no new backing or native/JS owner. Neither this
// bound nor a returned lease proves that stale Weak tombstones were freed.
const PREPARATION_TRANSIENT_BYTES: usize = 2 * std::mem::size_of::<InputRecord>()
    + std::mem::size_of::<RootSnapshot>()
    + std::mem::size_of::<Snapshot<'static>>()
    + 2 * std::mem::size_of::<OriginalHandle>()
    + 2 * std::mem::size_of::<NodeReference>()
    + 4 * std::mem::size_of::<usize>()
    + 4096;
const _: () = assert!(PREPARATION_TRANSIENT_BYTES <= registry::CALL_BYTES as usize);
const _: () = assert!(
    PREPARATION_TRANSIENT_BYTES
        <= pwright_node_host::ffi_budget::PublicationPhaseStorage::CAPACITY_BYTES
);

#[cfg(feature = "retained-target-marking")]
const NATIVE_MARK_FIXED_BYTES: usize = std::mem::size_of::<NativeSnapshot>()
    + std::mem::size_of::<NativeMarkProgress>()
    + std::mem::size_of::<NativeMarkState>()
    + std::mem::size_of::<NativeMarkCompletion>()
    + std::mem::size_of::<NativeMarkOutcome>()
    + std::mem::size_of::<TargetMarkOutcome>()
    + std::mem::size_of::<FramePublicationFence>()
    + std::mem::size_of::<HostError>()
    + 2 * std::mem::size_of::<usize>()
    + 128
    + 8192;
#[cfg(feature = "retained-target-marking")]
const _: () = assert!(NATIVE_MARK_FIXED_BYTES <= registry::CALL_BYTES as usize);

#[napi(object, object_from_js = false)]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct FileInputPreparationMetrics {
    pub pending: u32,
    pub root_containers: u32,
    pub rooted_values: u32,
}

#[cfg(feature = "retained-serialized-payload-import")]
#[napi(object, object_from_js = false)]
pub struct FileInputSerializedPayloadMetrics {
    pub depth: u32,
    pub nodes: u32,
    pub entries: u32,
    pub string_bytes: u32,
}

#[cfg(feature = "retained-target-marking")]
#[napi(object, object_from_js = false)]
pub struct FileInputProjectionFlags {
    pub multiple: bool,
    pub directory_upload: bool,
}

impl FileInputPreparations {
    pub(crate) fn new() -> Result<Self, HostError> {
        registry::Registry::new().map(Self)
    }
    pub(crate) fn scope(&self) -> Result<PreparationScope, HostError> {
        self.0.scope().map(|scope| {
            PreparationScope(
                scope,
                Rc::new(RefCell::new(TimeoutPolicySlot::Uninitialized)),
            )
        })
    }
    pub(crate) fn shutdown(&self) {
        self.0.shutdown();
    }
    pub(crate) fn seal(&self) {
        self.0.seal();
    }
    pub(crate) fn metrics(&self) -> Result<FileInputPreparationMetrics, HostError> {
        let (pending, containers) = self.0.metrics();
        Ok(FileInputPreparationMetrics {
            pending,
            root_containers: containers,
            rooted_values: self.0.rooted_values(|record| record.roots)?,
        })
    }
}
impl PreparationScope {
    pub(crate) fn close(&self) {
        // Seal client lookup before deleting Node roots: finalization must not
        // reenter an apparently uninitialized/open metadata scope during close.
        let previous = self.1.replace(TimeoutPolicySlot::Closed);
        if let TimeoutPolicySlot::Open(policies) = previous {
            policies.close();
        }
        self.0.close();
    }

    fn initialize_timeouts(
        &self,
        budget: &pwright_node_host::handles::TimeoutPolicyBudget,
        host: &pwright_node_host::HostSession,
        debug: bool,
    ) -> Result<(), HostError> {
        self.0.ensure_live()?;
        let mut slot = self.1.borrow_mut();
        match &*slot {
            TimeoutPolicySlot::Uninitialized => {}
            TimeoutPolicySlot::Open(_) => {
                return Err(HostError::InvalidArgument(
                    "File input timeout settings are already initialized".into(),
                ));
            }
            TimeoutPolicySlot::Closed => return Err(HostError::TargetClosed),
        }
        *slot = TimeoutPolicySlot::Open(budget.session(host, debug)?);
        Ok(())
    }

    pub(crate) fn timeout_policies(&self) -> Result<TimeoutPolicySession, HostError> {
        self.0.ensure_live()?;
        match &*self.1.borrow() {
            TimeoutPolicySlot::Open(policies) => Ok(policies.clone()),
            TimeoutPolicySlot::Closed => Err(HostError::TargetClosed),
            TimeoutPolicySlot::Uninitialized => Err(HostError::InvalidArgument(
                "File input timeout settings have not been initialized".into(),
            )),
        }
    }
    fn admit(&self, budget: &FfiBudget) -> Result<NativeFileInputPreparation, HostError> {
        self.0
            .admit(budget)
            .map(|ticket| NativeFileInputPreparation { ticket })
    }

    #[cfg(feature = "retained-target-marking")]
    fn next_protocol_call(&self) -> Result<u64, HostError> {
        self.0.next_protocol_call()
    }
}

/// A weak, session-bound receipt. No raw key is exposed as authority.
#[napi]
pub struct NativeFileInputPreparation {
    ticket: registry::Ticket<InputRecord>,
}

#[napi]
impl NativeFileInputPreparation {
    /// The returned language value is awaited by the thin JS continuation.
    /// No registry borrow or running-task token spans that await.
    #[napi]
    pub fn begin_owner_lookup<'env>(
        &self,
        env: &'env Env,
        ports: Object<'env>,
    ) -> napi::Result<Unknown<'env>> {
        let phase = self.ticket.phase().map_err(binding_error)?;
        let snapshot = phase
            .inspect(|record| record.snapshot(Stage::Initial))
            .map_err(binding_error)??
            .scoped(env)?;
        let language = preflight::LanguagePorts::read(ports)?;
        phase.inspect(|_| ()).map_err(binding_error)?;
        let owner = preflight::owner(env, snapshot.at(env, 0)?, &language)?;
        // A language callback can revoke this entry. Avoid allocating a new
        // persistent root after that; the phase's separate scratch admission
        // also covers finalizer reentry during the allocation itself.
        phase.inspect(|_| ()).map_err(binding_error)?;
        phase
            .replace(InputRecord {
                source: snapshot.source,
                original_handle: snapshot.original_handle,
                timeout_settings: snapshot.timeout_settings,
                #[cfg(feature = "retained-target-marking")]
                protocol_call: snapshot.protocol_call,
                #[cfg(feature = "retained-target-marking")]
                projection: snapshot.projection,
                arguments: Rc::new(NodeReference::new(env, snapshot.arguments)?),
                #[cfg(feature = "retained-serialized-payload-import")]
                serialized_payload: snapshot.serialized_payload,
                roots: 3,
                stage: Stage::WaitingOwner,
            })
            .map_err(binding_error)?;
        Ok(owner)
    }

    /// Runs outside the async input converter, after awaiting public ownerFrame.
    #[napi]
    pub fn prepare_context<'env>(
        &self,
        env: &'env Env,
        owner: Unknown<'env>,
        ports: Object<'env>,
    ) -> napi::Result<()> {
        let phase = self.ticket.phase().map_err(binding_error)?;
        let snapshot = phase
            .inspect(|record| record.snapshot(Stage::WaitingOwner))
            .map_err(binding_error)??
            .scoped(env)?;
        let language = preflight::LanguagePorts::read(ports)?;
        phase.inspect(|_| ()).map_err(binding_error)?;
        let context = preflight::context(env, owner, &language)?;
        phase.inspect(|_| ()).map_err(binding_error)?;
        let arguments = arguments(
            env,
            [
                snapshot.at(env, 0)?,
                snapshot.at(env, 1)?,
                snapshot.at(env, 2)?,
                context,
            ],
        )?;
        phase
            .replace(InputRecord {
                source: snapshot.source,
                original_handle: snapshot.original_handle,
                timeout_settings: snapshot.timeout_settings,
                #[cfg(feature = "retained-target-marking")]
                protocol_call: snapshot.protocol_call,
                #[cfg(feature = "retained-target-marking")]
                projection: snapshot.projection,
                arguments,
                #[cfg(feature = "retained-serialized-payload-import")]
                serialized_payload: snapshot.serialized_payload,
                roots: 4,
                stage: Stage::ContextReady,
            })
            .map_err(binding_error)
    }

    /// Called inside the language async converter. Its exact ordinary payload
    /// carrier participates in async-return assimilation before finishConversion.
    #[napi]
    pub fn convert_input<'env>(
        &self,
        env: &'env Env,
        ports: Object<'env>,
    ) -> napi::Result<Unknown<'env>> {
        let phase = self.ticket.phase().map_err(binding_error)?;
        let snapshot = phase
            .inspect(|record| record.snapshot(Stage::ContextReady))
            .map_err(binding_error)??
            .scoped(env)?;
        let language = preflight::LanguagePorts::read(ports)?;
        phase.inspect(|_| ()).map_err(binding_error)?;
        let converted =
            preflight::convert(env, snapshot.at(env, 3)?, snapshot.at(env, 1)?, &language)?;
        phase.inspect(|_| ()).map_err(binding_error)?;
        let value = payload_result(env, &converted)?;
        let arguments = arguments(
            env,
            [
                snapshot.at(env, 0)?,
                snapshot.at(env, 1)?,
                snapshot.at(env, 2)?,
                converted.context,
                converted.items,
            ],
        )?;
        phase
            .replace(InputRecord {
                source: snapshot.source,
                original_handle: snapshot.original_handle,
                timeout_settings: snapshot.timeout_settings,
                #[cfg(feature = "retained-target-marking")]
                protocol_call: snapshot.protocol_call,
                #[cfg(feature = "retained-target-marking")]
                projection: snapshot.projection,
                arguments,
                #[cfg(feature = "retained-serialized-payload-import")]
                serialized_payload: snapshot.serialized_payload,
                roots: 5,
                stage: Stage::WaitingConversion,
            })
            .map_err(binding_error)?;
        Ok(value)
    }

    #[napi]
    pub fn finish_conversion<'env>(
        &self,
        env: &'env Env,
        converted: Unknown<'env>,
    ) -> napi::Result<()> {
        let phase = self.ticket.phase().map_err(binding_error)?;
        let snapshot = phase
            .inspect(|record| record.snapshot(Stage::WaitingConversion))
            .map_err(binding_error)??
            .scoped(env)?;
        // Assimilation can replace the result. Preserve that value without
        // channel validation, name/options getters, or byte materialization.
        phase.inspect(|_| ()).map_err(binding_error)?;
        let arguments = arguments(
            env,
            [
                snapshot.at(env, 0)?,
                snapshot.at(env, 1)?,
                snapshot.at(env, 2)?,
                snapshot.at(env, 3)?,
                snapshot.at(env, 4)?,
                converted,
            ],
        )?;
        phase
            .replace(InputRecord {
                source: snapshot.source,
                original_handle: snapshot.original_handle,
                timeout_settings: snapshot.timeout_settings,
                #[cfg(feature = "retained-target-marking")]
                protocol_call: snapshot.protocol_call,
                #[cfg(feature = "retained-target-marking")]
                projection: snapshot.projection,
                arguments,
                #[cfg(feature = "retained-serialized-payload-import")]
                serialized_payload: snapshot.serialized_payload,
                roots: 6,
                stage: Stage::Ready,
            })
            .map_err(binding_error)
    }

    #[napi]
    pub fn abort(&self) -> bool {
        self.ticket.abort()
    }

    /// Typed language deadline delivery. The Node timer owns the public
    /// TimeoutError; this synchronous edge revokes the exact parked/running
    /// preparation without pretending that Rust observed time while JS was
    /// blocked.
    #[cfg(feature = "retained-target-marking")]
    #[napi]
    pub fn abort_deadline(&self) -> bool {
        self.ticket.abort()
    }
    #[napi]
    pub fn is_pending(&self) -> bool {
        self.ticket.is_pending()
    }

    /// Internal post-conversion phase. Settings were bound at Handle creation
    /// and copied into preparation before observable conversion; the caller
    /// cannot substitute another Page's metadata during this phase.
    #[napi]
    pub fn assemble_arguments<'env>(
        &self,
        env: &'env Env,
        ports: Object<'env>,
    ) -> napi::Result<()> {
        let phase = self.ticket.phase().map_err(binding_error)?;
        let snapshot = phase
            .inspect(|record| record.snapshot(Stage::Ready))
            .map_err(binding_error)??
            .scoped(env)?;
        let value = assemble(
            env,
            snapshot.at(env, 5)?,
            snapshot.at(env, 2)?,
            &snapshot.timeout_settings,
            ports,
        )?;
        phase.inspect(|_| ()).map_err(binding_error)?;
        let arguments = arguments(
            env,
            [
                snapshot.at(env, 0)?,
                snapshot.at(env, 1)?,
                snapshot.at(env, 2)?,
                snapshot.at(env, 3)?,
                snapshot.at(env, 4)?,
                value,
            ],
        )?;
        phase
            .replace(InputRecord {
                source: snapshot.source,
                original_handle: snapshot.original_handle,
                timeout_settings: snapshot.timeout_settings,
                #[cfg(feature = "retained-target-marking")]
                protocol_call: snapshot.protocol_call,
                #[cfg(feature = "retained-target-marking")]
                projection: snapshot.projection,
                arguments,
                #[cfg(feature = "retained-serialized-payload-import")]
                serialized_payload: snapshot.serialized_payload,
                roots: 6,
                stage: Stage::ArgumentsReady,
            })
            .map_err(binding_error)
    }

    /// Internal raw-buffer client channel validation. No byte snapshot,
    /// server preparation, deadline or native-use authority is acquired here.
    /// Public API exception wrapping remains outside this synchronous phase.
    #[napi]
    pub fn validate_channel<'env>(&self, env: &'env Env, ports: Object<'env>) -> napi::Result<()> {
        let phase = self.ticket.phase().map_err(binding_error)?;
        let snapshot = phase
            .inspect(|record| record.snapshot(Stage::ArgumentsReady))
            .map_err(binding_error)??
            .scoped(env)?;
        let value = channel_validation::validate(env, snapshot.at(env, 5)?, ports)?;
        // User map/valueOf/getter/setter callbacks may revoke the preparation.
        // Recheck before allocating replacement roots, outside registry borrows.
        phase.inspect(|_| ()).map_err(binding_error)?;
        let arguments = arguments(
            env,
            [
                snapshot.at(env, 0)?,
                snapshot.at(env, 1)?,
                snapshot.at(env, 2)?,
                snapshot.at(env, 3)?,
                snapshot.at(env, 4)?,
                value,
            ],
        )?;
        phase
            .replace(InputRecord {
                source: snapshot.source,
                original_handle: snapshot.original_handle,
                timeout_settings: snapshot.timeout_settings,
                #[cfg(feature = "retained-target-marking")]
                protocol_call: snapshot.protocol_call,
                #[cfg(feature = "retained-target-marking")]
                projection: snapshot.projection,
                arguments,
                #[cfg(feature = "retained-serialized-payload-import")]
                serialized_payload: snapshot.serialized_payload,
                roots: 6,
                stage: Stage::ChannelReady,
            })
            .map_err(binding_error)
    }

    /// Internal server-direction schema kernel, gated by an exact-session
    /// registration check. Deferred scheduling and server error transport are
    /// separate; the check does not acquire a native-use pin or dispatcher.
    #[napi]
    pub fn validate_server_params<'env>(
        &self,
        env: &'env Env,
        ports: Object<'env>,
    ) -> napi::Result<()> {
        let phase = self.ticket.phase().map_err(binding_error)?;
        let snapshot = phase
            .inspect(|record| record.snapshot(Stage::ServerDispatchReady))
            .map_err(binding_error)??
            .scoped(env)?;
        let value = channel_validation::validate_server(env, snapshot.at(env, 5)?, ports)?;
        replace_language_phase(env, phase, snapshot, value, Stage::ServerParamsReady)
    }

    /// The thin JS async wrapper assimilates the returned ordinary carrier.
    /// Nested serializer/map results remain raw. A second registration check
    /// gates this phase. No Buffer pointer, byte copy, native-use lease or
    /// tracked execution survives a language callback.
    #[napi]
    pub fn begin_materialization<'env>(
        &self,
        env: &'env Env,
        ports: Object<'env>,
    ) -> napi::Result<Unknown<'env>> {
        let phase = self.ticket.phase().map_err(binding_error)?;
        let snapshot = phase
            .inspect(|record| record.snapshot(Stage::ServerMaterializationReady))
            .map_err(binding_error)??
            .scoped(env)?;
        let params = snapshot.at(env, 5)?;
        let value = materialization::materialize(env, params, ports)?;
        replace_language_phase(env, phase, snapshot, params, Stage::WaitingMaterialization)?;
        Ok(value)
    }

    /// Capture the awaited outer carrier without inspecting nested values.
    /// Keep server params (including timeout) in the fifth root for the later
    /// native conversion; the sixth root holds the possibly replaced carrier.
    #[napi]
    pub fn finish_materialization<'env>(
        &self,
        env: &'env Env,
        value: Unknown<'env>,
    ) -> napi::Result<()> {
        let phase = self.ticket.phase().map_err(binding_error)?;
        let snapshot = phase
            .inspect(|record| record.snapshot(Stage::WaitingMaterialization))
            .map_err(binding_error)??
            .scoped(env)?;
        phase.inspect(|_| ()).map_err(binding_error)?;
        let arguments = arguments(
            env,
            [
                snapshot.at(env, 0)?,
                snapshot.at(env, 1)?,
                snapshot.at(env, 2)?,
                snapshot.at(env, 3)?,
                snapshot.at(env, 5)?,
                value,
            ],
        )?;
        phase
            .replace(InputRecord {
                source: snapshot.source,
                original_handle: snapshot.original_handle,
                timeout_settings: snapshot.timeout_settings,
                #[cfg(feature = "retained-target-marking")]
                protocol_call: snapshot.protocol_call,
                #[cfg(feature = "retained-target-marking")]
                projection: snapshot.projection,
                arguments,
                #[cfg(feature = "retained-serialized-payload-import")]
                serialized_payload: snapshot.serialized_payload,
                roots: 6,
                stage: Stage::Materialized,
            })
            .map_err(binding_error)
    }

    /// Read the raw materialized carrier only after ordered target marking.
    /// The parked native owner remains inside the same FFI result while Node
    /// getters run. Four raw projected values extend the logical root set from
    /// six to ten; the native truth flags stay in the Rust record.
    #[cfg(feature = "retained-target-marking")]
    #[napi]
    pub fn project_after_marking<'env>(
        &self,
        env: &'env Env,
        ports: Object<'env>,
    ) -> napi::Result<FileInputProjectionFlags> {
        let phase = self.ticket.phase().map_err(binding_error)?;
        let snapshot = phase
            .inspect(|record| record.snapshot(Stage::Marked))
            .map_err(binding_error)??
            .scoped(env)?;
        if snapshot.projection.is_some() {
            return Err(binding_error(HostError::InvalidArgument(
                "Upload preparation carrier was already projected".into(),
            )));
        }
        let projected = projection::project(env, snapshot.at(env, 5)?, ports)?;
        let flags = ProjectionFlags {
            multiple: projected.multiple,
            directory_upload: projected.directory_upload,
        };
        // Projection getters/coercion may synchronously abort the preparation.
        // Recheck before allocating replacement roots and before committing.
        phase.inspect(|_| ()).map_err(binding_error)?;
        let arguments = arguments(
            env,
            [
                snapshot.at(env, 0)?,
                snapshot.at(env, 1)?,
                snapshot.at(env, 2)?,
                snapshot.at(env, 3)?,
                snapshot.at(env, 4)?,
                snapshot.at(env, 5)?,
                projected.file_payloads,
                projected.local_paths,
                projected.local_directory,
                projected.raw_multiple,
            ],
        )?;
        phase
            .replace(InputRecord {
                source: snapshot.source,
                original_handle: snapshot.original_handle,
                timeout_settings: snapshot.timeout_settings,
                protocol_call: snapshot.protocol_call,
                projection: Some(flags),
                arguments,
                #[cfg(feature = "retained-serialized-payload-import")]
                serialized_payload: snapshot.serialized_payload,
                roots: 10,
                stage: Stage::Projected,
            })
            .map_err(binding_error)?;
        Ok(FileInputProjectionFlags {
            multiple: flags.multiple,
            directory_upload: flags.directory_upload,
        })
    }
}

fn replace_language_phase<'env>(
    env: &'env Env,
    phase: registry::Phase<InputRecord>,
    snapshot: Snapshot<'env>,
    value: Unknown<'env>,
    stage: Stage,
) -> napi::Result<()> {
    // Never allocate replacement persistent roots after a callback revoked the
    // entry. The independent synchronous scratch permit spans root finalizers.
    phase.inspect(|_| ()).map_err(binding_error)?;
    let arguments = arguments(
        env,
        [
            snapshot.at(env, 0)?,
            snapshot.at(env, 1)?,
            snapshot.at(env, 2)?,
            snapshot.at(env, 3)?,
            snapshot.at(env, 4)?,
            value,
        ],
    )?;
    phase
        .replace(InputRecord {
            source: snapshot.source,
            original_handle: snapshot.original_handle,
            timeout_settings: snapshot.timeout_settings,
            #[cfg(feature = "retained-target-marking")]
            protocol_call: snapshot.protocol_call,
            #[cfg(feature = "retained-target-marking")]
            projection: snapshot.projection,
            arguments,
            #[cfg(feature = "retained-serialized-payload-import")]
            serialized_payload: snapshot.serialized_payload,
            roots: 6,
            stage,
        })
        .map_err(binding_error)
}

pub struct PreparationOutput(NativeFileInputPreparation);
impl ToNapiValue for PreparationOutput {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        // Failed construction/tag/wrap drops the ticket and its rooted inputs.
        unsafe { publication::to_napi_class(env, "NativeFileInputPreparation\0", value.0) }
    }
}

fn arguments<const N: usize>(
    env: &Env,
    values: [Unknown<'_>; N],
) -> napi::Result<Rc<NodeReference>> {
    let maximum = if cfg!(feature = "retained-call-argument-serialization") {
        11
    } else if cfg!(feature = "retained-target-marking") {
        10
    } else {
        6
    };
    if N > maximum {
        return Err(binding_error(HostError::ResourceExhausted(
            "Upload preparation root limit exceeded".into(),
        )));
    }
    let mut array = ptr::null_mut();
    // Admission precedes every array/reference allocation. Defining OWN data
    // properties cannot invoke inherited Array.prototype setters or user getters.
    status(unsafe { sys::napi_create_array_with_length(env.raw(), N, &mut array) })?;
    let names = [
        c"0", c"1", c"2", c"3", c"4", c"5", c"6", c"7", c"8", c"9", c"10",
    ];
    let properties: [sys::napi_property_descriptor; N] =
        std::array::from_fn(|index| sys::napi_property_descriptor {
            utf8name: names[index].as_ptr(),
            name: ptr::null_mut(),
            method: None,
            getter: None,
            setter: None,
            value: values[index].raw(),
            attributes: sys::PropertyAttributes::writable
                | sys::PropertyAttributes::enumerable
                | sys::PropertyAttributes::configurable,
            data: ptr::null_mut(),
        });
    status(unsafe {
        sys::napi_define_properties(env.raw(), array, properties.len(), properties.as_ptr())
    })?;
    NodeReference::new(env, unsafe {
        Unknown::from_raw_unchecked(env.raw(), array)
    })
    .map(Rc::new)
}

#[napi]
impl NativeSession {
    /// Internal staging entrypoint. The public ElementHandle method and its
    /// phase/error compatibility are deliberately not wired to this yet.
    #[napi]
    pub fn handle_prepare_file_input(
        &self,
        env: &Env,
        handle: &NativeHandle,
        receiver: Unknown<'_>,
        files: Unknown<'_>,
        options: Unknown<'_>,
    ) -> napi::Result<PreparationOutput> {
        self.host
            .validate_retained_file_input_source(&handle.source)
            .map_err(binding_error)?;
        let timeout_settings = handle
            .file_input_timeout_settings()
            .map_err(binding_error)?;
        let preparation = self
            .file_input_scope
            .admit(&self.ffi)
            .map_err(binding_error)?;
        let phase = preparation.ticket.creation_phase().map_err(binding_error)?;
        let record = InputRecord {
            source: handle.source.clone(),
            // Bind the actual native argument, not the arbitrary JS receiver.
            // Do not upgrade/read State or move disposal checks before getters.
            original_handle: OriginalHandle::capture(handle),
            timeout_settings,
            #[cfg(feature = "retained-target-marking")]
            protocol_call: None,
            #[cfg(feature = "retained-target-marking")]
            projection: None,
            arguments: arguments(env, [receiver, files, options])?,
            #[cfg(feature = "retained-serialized-payload-import")]
            serialized_payload: None,
            roots: 3,
            stage: Stage::Initial,
        };
        phase.replace(record).map_err(binding_error)?;
        Ok(PreparationOutput(preparation))
    }

    /// Exact-session metadata inspection; a same-environment sibling receipt
    /// cannot borrow this session's authority. This grants no native use.
    #[napi]
    pub fn handle_file_input_preparation_frame(
        &self,
        preparation: &NativeFileInputPreparation,
    ) -> napi::Result<String> {
        let source = self
            .file_input_scope
            .0
            .inspect(&preparation.ticket, |record| record.source.clone())
            .map_err(binding_error)?;
        self.host
            .validate_retained_file_input_source(&source)
            .map_err(binding_error)?;
        Ok(source.creation_frame_id().get().to_string())
    }

    /// Allocate the public connection/session call identity after server
    /// schema validation and before instrumentation. This counter is neither a
    /// preparation key nor the Rust command runtime's private CallId.
    #[cfg(feature = "retained-target-marking")]
    #[napi]
    pub fn handle_begin_file_input_call(
        &self,
        preparation: &NativeFileInputPreparation,
    ) -> napi::Result<String> {
        // Exact-session validation precedes exclusive checkout so a foreign
        // session cannot abort the legitimate owner's preparation.
        self.file_input_scope
            .0
            .inspect(&preparation.ticket, |record| record.stage)
            .map_err(binding_error)?;
        let phase = preparation.ticket.phase().map_err(binding_error)?;
        let mut record = phase.inspect(Clone::clone).map_err(binding_error)?;
        if record.stage != Stage::ServerParamsReady || record.protocol_call.is_some() {
            return Err(binding_error(HostError::InvalidArgument(
                "Upload preparation is not ready for public call identity".into(),
            )));
        }
        let call = self
            .file_input_scope
            .next_protocol_call()
            .map_err(binding_error)?;
        record.protocol_call = Some(call);
        record.stage = Stage::CallReady;
        phase.replace(record).map_err(binding_error)?;
        Ok(format!("call@{call}"))
    }

    /// Enter the single public deadline immediately before materialization.
    /// Return the already server-validated raw timeout to the Node timer; Rust
    /// neither coerces it nor starts a competing native clock.
    #[cfg(feature = "retained-target-marking")]
    #[napi]
    pub fn handle_start_file_input_deadline<'env>(
        &self,
        env: &'env Env,
        preparation: &NativeFileInputPreparation,
    ) -> napi::Result<Unknown<'env>> {
        self.file_input_scope
            .0
            .inspect(&preparation.ticket, |record| record.stage)
            .map_err(binding_error)?;
        let phase = preparation.ticket.phase().map_err(binding_error)?;
        let mut record = phase.inspect(Clone::clone).map_err(binding_error)?;
        if record.stage != Stage::CallReady || record.protocol_call.is_none() {
            return Err(binding_error(HostError::InvalidArgument(
                "Upload preparation has no active public call".into(),
            )));
        }
        // This is the selected replacement for the old second dispatch gate:
        // schema- or instrumentation-scheduled retirement must win before any
        // materialization getter or serializer runs.
        self.host
            .check_retained_file_input_dispatch(&record.source)
            .map_err(binding_error)?;
        let arguments = record.arguments.get(env)?;
        let snapshot = Snapshot {
            source: record.source.clone(),
            original_handle: record.original_handle.clone(),
            timeout_settings: record.timeout_settings.clone(),
            protocol_call: record.protocol_call,
            projection: record.projection,
            #[cfg(feature = "retained-serialized-payload-import")]
            serialized_payload: record.serialized_payload.clone(),
            arguments,
        };
        let params = snapshot.at(env, 5)?;
        let mut timeout = ptr::null_mut();
        status(unsafe {
            sys::napi_get_named_property(env.raw(), params.raw(), c"timeout".as_ptr(), &mut timeout)
        })?;
        // Retain exact ticket authority across any unexpected accessor reentry.
        phase.inspect(|_| ()).map_err(binding_error)?;
        record.stage = Stage::ServerMaterializationReady;
        phase.replace(record).map_err(binding_error)?;
        Ok(unsafe { Unknown::from_raw_unchecked(env.raw(), timeout) })
    }

    /// Exact-session point check and typed phase transition, not a native-use
    /// receipt. Rust enforces checks on both sides of server validation; the
    /// language scheduler supplies the intervening async checkpoint. Supplied
    /// object IDs, Handle wrappers or replacement JS receivers are not authority.
    #[napi]
    pub fn handle_check_file_input_dispatch(
        &self,
        preparation: &NativeFileInputPreparation,
    ) -> napi::Result<()> {
        // Resolve the exact scope BEFORE acquiring the ticket's exclusive
        // phase: a foreign session must not abort the legitimate owner's call.
        self.file_input_scope
            .0
            .inspect(&preparation.ticket, |_| ())
            .map_err(binding_error)?;
        let phase = preparation.ticket.phase().map_err(binding_error)?;
        let mut record = phase.inspect(Clone::clone).map_err(binding_error)?;
        let next = match record.stage {
            Stage::ChannelReady => Stage::ServerDispatchReady,
            Stage::ServerParamsReady => {
                #[cfg(feature = "retained-target-marking")]
                {
                    return Err(binding_error(HostError::InvalidArgument(
                        "Upload preparation must establish its public call and deadline".into(),
                    )));
                }
                #[cfg(not(feature = "retained-target-marking"))]
                {
                    Stage::ServerMaterializationReady
                }
            }
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "Upload preparation is not at a server dispatch checkpoint".into(),
                )));
            }
        };
        self.host
            .check_retained_file_input_dispatch(&record.source)
            .map_err(binding_error)?;
        record.stage = next;
        // Clone only the private Rc root container; this gate allocates no new
        // Node reference and drops old records outside every registry borrow.
        phase.replace(record).map_err(binding_error)
    }

    /// Promote the already-materialized preparation onto its original FFI
    /// admission, mark the actual native Handle, publish ordered lifecycle
    /// work, then park that same admission for the next native phase.
    #[cfg(feature = "retained-target-marking")]
    #[napi]
    pub fn handle_mark_file_input_target<'env>(
        &self,
        env: &'env Env,
        preparation: &NativeFileInputPreparation,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        // Establish exact Node-session provenance before acquiring an exclusive
        // phase. A foreign session must never abort the legitimate owner's call.
        self.file_input_scope
            .0
            .inspect(&preparation.ticket, InputRecord::native_snapshot)
            .map_err(binding_error)?
            .map_err(binding_error)?;
        let (native_phase, snapshot, ffi_preparation) = preparation
            .ticket
            .begin_native(InputRecord::native_snapshot)
            .map_err(binding_error)?;
        let call_id = format!("call@{}", snapshot.protocol_call);
        let registration = self
            .host
            .register_retained_file_input(&snapshot.source, ffi_preparation.revoker())
            .map_err(binding_error)?;
        let state = Arc::new(NativeMarkState {
            registration,
            completion: Mutex::new(None),
        });
        let completion = Arc::downgrade(&state);
        let mut execution = ffi_preparation.start(&self.ffi).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let source = snapshot.source;
        let original_handle = snapshot.original_handle;
        let progress = NativeMarkProgress {
            _state: Arc::clone(&state),
        };
        let future = async move {
            let (outcome, retained) = match host
                .check_retained_file_input_registration(&source, &state.registration)
            {
                Err(error) => (NativeMarkOutcome::Host(error), None),
                Ok(()) => match original_handle.upgrade() {
                    Err(error) => (NativeMarkOutcome::Host(error), None),
                    Ok(handle) => match host.handle_mark_target_retaining(handle, &call_id).await {
                        Ok(retained) => (NativeMarkOutcome::Target(Ok(())), Some(retained)),
                        Err(error) => (NativeMarkOutcome::Target(Err(error)), None),
                    },
                },
            };
            let completion = host
                .frame_publications()
                .fence()
                .map(|fence| NativeMarkCompletion {
                    outcome: Some(outcome),
                    retained,
                    fence: Some(fence),
                    #[cfg(feature = "retained-target-prevalidation")]
                    prevalidation: None,
                    #[cfg(feature = "retained-target-prevalidation")]
                    validated: None,
                });
            state.complete(completion)?;
            Ok(progress)
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        let future = Box::pin(future);
        let execution = Box::pin(execution.run(future));
        let ffi = self.ffi.clone();
        let port = self.publications.clone();
        env.spawn_future_with_callback(async move { Ok(execution.await) }, move |env, ticket| {
            // Park before publication so synchronous close/reentry can revoke
            // the exact preparation and its roots through ordinary ownership.
            let parked = ticket.park(&ffi).map_err(binding_error)?;
            let phase = native_phase.park(parked).map_err(binding_error)?;
            let state = completion
                .upgrade()
                .ok_or_else(|| binding_error(HostError::TargetClosed))?;
            let fence = state.take_fence().map_err(binding_error)?;
            drop(state);
            port.publish(env, fence)?;

            // Publication may synchronously close/revoke this preparation.
            phase
                .inspect(InputRecord::native_snapshot)
                .map_err(binding_error)?
                .map_err(binding_error)?;
            let state = completion
                .upgrade()
                .ok_or_else(|| binding_error(HostError::TargetClosed))?;
            state.finish_outcome().map_err(binding_error)?;
            drop(state);

            let mut record = phase.inspect(Clone::clone).map_err(binding_error)?;
            record.stage = Stage::Marked;
            phase.replace(record).map_err(binding_error)?;
            ().into_unknown(env)
        })
    }
}

#[cfg(feature = "retained-target-prevalidation")]
#[napi]
impl NativeSession {
    /// Resume the exact parked FFI admission and consume its marked owner in
    /// native retained prevalidation. No new Handle adoption, protocol call or
    /// deadline is created; success parks the validated receiver for the next
    /// native phase on this same ticket.
    #[napi]
    pub fn handle_prevalidate_file_input_target<'env>(
        &self,
        env: &'env Env,
        preparation: &NativeFileInputPreparation,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        self.file_input_scope
            .0
            .inspect(&preparation.ticket, InputRecord::prevalidation_snapshot)
            .map_err(binding_error)?
            .map_err(binding_error)?;
        let (native_phase, snapshot, ffi_preparation) = preparation
            .ticket
            .begin_native(InputRecord::prevalidation_snapshot)
            .map_err(binding_error)?;
        let (progress, mut execution) = ffi_preparation.resume(&self.ffi).map_err(binding_error)?;
        let state = Arc::clone(&progress._state);
        let retained = state
            .take_marked_for_prevalidation()
            .map_err(binding_error)?;
        let completion = Arc::downgrade(&state);
        let host = Arc::clone(&self.host);
        let source = snapshot.source;
        let flags = snapshot.flags;
        let future = async move {
            let result =
                match host.check_retained_file_input_registration(&source, &state.registration) {
                    Err(error) => Err(error),
                    Ok(()) => retained
                        .prevalidate(flags)
                        .await
                        .map_err(target_prevalidation_language_error),
                };
            state.complete_prevalidation(result)?;
            Ok(progress)
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        let future = Box::pin(future);
        let execution = Box::pin(execution.run(future));
        let ffi = self.ffi.clone();
        env.spawn_future_with_callback(async move { Ok(execution.await) }, move |env, ticket| {
            let parked = ticket.park(&ffi).map_err(binding_error)?;
            let phase = native_phase.park(parked).map_err(binding_error)?;
            phase
                .inspect(InputRecord::prevalidation_snapshot)
                .map_err(binding_error)?
                .map_err(binding_error)?;
            let state = completion
                .upgrade()
                .ok_or_else(|| binding_error(HostError::TargetClosed))?;
            state.finish_prevalidation().map_err(binding_error)?;
            drop(state);

            let mut record = phase.inspect(Clone::clone).map_err(binding_error)?;
            record.stage = Stage::Prevalidated;
            phase.replace(record).map_err(binding_error)?;
            ().into_unknown(env)
        })
    }
}

#[cfg(feature = "retained-input-instrumentation")]
#[napi]
impl NativeSession {
    /// Enter Playwright's awaited `onBeforeInputAction` boundary only after
    /// retained DOM prevalidation. The returned value is root zero: the
    /// original JavaScript ElementHandle receiver, never the retargeted native
    /// owner and never a replacement object ID supplied by JavaScript. It is
    /// only an internal identity token, not Playwright's full server SdkObject.
    #[napi]
    pub fn handle_begin_file_input_instrumentation<'env>(
        &self,
        env: &'env Env,
        preparation: &NativeFileInputPreparation,
    ) -> napi::Result<Unknown<'env>> {
        // Establish exact-session provenance before exclusive checkout. A
        // foreign session must not abort the legitimate owner's parked ticket.
        self.file_input_scope
            .0
            .inspect(&preparation.ticket, |record| {
                record.instrumentation_snapshot(Stage::Prevalidated)
            })
            .map_err(binding_error)??;
        let phase = preparation.ticket.phase().map_err(binding_error)?;
        let roots = phase
            .inspect(|record| record.instrumentation_snapshot(Stage::Prevalidated))
            .map_err(binding_error)??;
        let receiver = roots.scoped(env)?.at(env, 0)?;
        let mut record = phase.inspect(Clone::clone).map_err(binding_error)?;
        record.stage = Stage::WaitingInputAction;
        phase.replace(record).map_err(binding_error)?;
        Ok(receiver)
    }

    /// Commit successful completion of the exact awaited instrumentation
    /// boundary. This is a synchronous phase gate: it neither resumes nor
    /// replaces the parked FFI admission and allocates no additional root.
    #[napi]
    pub fn handle_finish_file_input_instrumentation(
        &self,
        preparation: &NativeFileInputPreparation,
    ) -> napi::Result<()> {
        self.file_input_scope
            .0
            .inspect(&preparation.ticket, |record| {
                record.instrumentation_snapshot(Stage::WaitingInputAction)
            })
            .map_err(binding_error)??;
        let phase = preparation.ticket.phase().map_err(binding_error)?;
        phase
            .inspect(|record| record.instrumentation_snapshot(Stage::WaitingInputAction))
            .map_err(binding_error)??;
        let mut record = phase.inspect(Clone::clone).map_err(binding_error)?;
        record.stage = Stage::Instrumented;
        phase.replace(record).map_err(binding_error)
    }
}

#[cfg(feature = "retained-call-argument-serialization")]
#[napi]
impl NativeSession {
    /// Run the pinned synchronous JavaScript call-argument serializer only
    /// after the awaited input-action boundary. Rust owns phase/session/root
    /// authority; the supplied frozen package adapter owns JavaScript getter,
    /// `toJSON`, special-value and cycle semantics. The resulting graph stays
    /// a Node-thread root: large base64 strings are referenced, not copied into
    /// Rust or sent to CDP by this leaf.
    #[napi]
    pub fn handle_serialize_file_input_payloads<'env>(
        &self,
        env: &'env Env,
        preparation: &NativeFileInputPreparation,
        serializer: Unknown<'env>,
    ) -> napi::Result<()> {
        // Prove exact-session provenance before exclusive checkout. A foreign
        // session cannot cancel the rightful owner's retained native target.
        self.file_input_scope
            .0
            .inspect(&preparation.ticket, |record| {
                record.instrumentation_snapshot(Stage::Instrumented)
            })
            .map_err(binding_error)??;
        if serializer.get_type()? != napi::ValueType::Function {
            return Err(binding_error(HostError::InvalidArgument(
                "File-input call-argument serializer must be a function".into(),
            )));
        }
        let phase = preparation.ticket.phase().map_err(binding_error)?;
        let snapshot = phase
            .inspect(|record| record.instrumentation_snapshot(Stage::Instrumented))
            .map_err(binding_error)??
            .scoped(env)?;
        let payloads = snapshot.at(env, 6)?;
        let mut receiver = ptr::null_mut();
        let mut serialized = ptr::null_mut();
        status(unsafe { sys::napi_get_undefined(env.raw(), &mut receiver) })?;
        status(unsafe {
            sys::napi_call_function(
                env.raw(),
                receiver,
                serializer.raw(),
                1,
                [payloads.raw()].as_ptr(),
                &mut serialized,
            )
        })?;
        let serialized = unsafe { Unknown::from_raw_unchecked(env.raw(), serialized) };
        // A getter or toJSON call may reenter and revoke the ticket. Never
        // install a fresh persistent root after that revocation.
        phase.inspect(|_| ()).map_err(binding_error)?;
        let arguments = arguments(
            env,
            [
                snapshot.at(env, 0)?,
                snapshot.at(env, 1)?,
                snapshot.at(env, 2)?,
                snapshot.at(env, 3)?,
                snapshot.at(env, 4)?,
                snapshot.at(env, 5)?,
                snapshot.at(env, 6)?,
                snapshot.at(env, 7)?,
                snapshot.at(env, 8)?,
                snapshot.at(env, 9)?,
                serialized,
            ],
        )?;
        phase
            .replace(InputRecord {
                source: snapshot.source,
                original_handle: snapshot.original_handle,
                timeout_settings: snapshot.timeout_settings,
                protocol_call: snapshot.protocol_call,
                projection: snapshot.projection,
                arguments,
                #[cfg(feature = "retained-serialized-payload-import")]
                serialized_payload: snapshot.serialized_payload,
                roots: 11,
                stage: Stage::Serialized,
            })
            .map_err(binding_error)
    }
}

#[cfg(feature = "retained-serialized-payload-import")]
#[napi]
impl NativeSession {
    /// Import the inaccessible serialized graph into one separately admitted
    /// Rust owner. The full 96 MiB allowance is reserved before copying. On
    /// success every V8 input root is released; no browser command is sent.
    #[napi]
    pub fn handle_import_serialized_file_input_payloads(
        &self,
        env: &Env,
        preparation: &NativeFileInputPreparation,
    ) -> napi::Result<FileInputSerializedPayloadMetrics> {
        let source = self
            .file_input_scope
            .0
            .inspect(&preparation.ticket, |record| {
                record
                    .instrumentation_snapshot(Stage::Serialized)
                    .map(|snapshot| snapshot.source)
            })
            .map_err(binding_error)??;
        // Reject unavailable owner capacity before exclusive phase checkout.
        // A caller may retry the same Serialized ticket after an independent
        // retained owner is consumed; public dispatch still aborts on error.
        let admission = self
            .host
            .admit_retained_serialized_file_input(&source)
            .map_err(binding_error)?;
        let phase = preparation.ticket.phase().map_err(binding_error)?;
        let snapshot = phase
            .inspect(|record| record.instrumentation_snapshot(Stage::Serialized))
            .map_err(binding_error)??
            .scoped(env)?;
        let imported = serialized_payload::read(env, snapshot.at(env, 10)?, admission)?;
        let metrics = imported.metrics();

        // Import reads only package-created own data, but keep the ordinary
        // post-language gate: revocation must win before a new owner/root can
        // be installed even if a future adapter gains a reentrant edge.
        phase.inspect(|_| ()).map_err(binding_error)?;
        let arguments = arguments(env, [])?;
        phase
            .replace(InputRecord {
                source: snapshot.source,
                original_handle: snapshot.original_handle,
                timeout_settings: snapshot.timeout_settings,
                protocol_call: snapshot.protocol_call,
                projection: snapshot.projection,
                arguments,
                serialized_payload: Some(Arc::new(imported)),
                roots: 0,
                stage: Stage::PayloadImported,
            })
            .map_err(binding_error)?;
        Ok(FileInputSerializedPayloadMetrics {
            depth: u32::try_from(metrics.depth).map_err(|_| {
                binding_error(HostError::ResourceExhausted(
                    "serialized payload metric overflow".into(),
                ))
            })?,
            nodes: u32::try_from(metrics.nodes).map_err(|_| {
                binding_error(HostError::ResourceExhausted(
                    "serialized payload metric overflow".into(),
                ))
            })?,
            entries: u32::try_from(metrics.entries).map_err(|_| {
                binding_error(HostError::ResourceExhausted(
                    "serialized payload metric overflow".into(),
                ))
            })?,
            string_bytes: u32::try_from(metrics.string_bytes).map_err(|_| {
                binding_error(HostError::ResourceExhausted(
                    "serialized payload metric overflow".into(),
                ))
            })?,
        })
    }
}

#[cfg(feature = "retained-serialized-payload-mutation")]
#[napi]
impl NativeSession {
    /// Resume the same FFI admission, move the Rust-owned graph and exact
    /// prevalidated target into one native mutation, then terminally remove the
    /// preparation only after mutation and object-group cleanup have settled.
    #[napi]
    pub fn handle_mutate_serialized_file_input_payloads<'env>(
        &self,
        env: &'env Env,
        preparation: &NativeFileInputPreparation,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        self.file_input_scope
            .0
            .inspect(&preparation.ticket, InputRecord::mutation_snapshot)
            .map_err(binding_error)?
            .map_err(binding_error)?;
        let (native_phase, snapshot, ffi_preparation) = preparation
            .ticket
            .begin_native(InputRecord::mutation_snapshot)
            .map_err(binding_error)?;
        let (progress, mut execution) = ffi_preparation.resume(&self.ffi).map_err(binding_error)?;
        let state = Arc::clone(&progress._state);
        let validated = state
            .take_prevalidated_for_mutation()
            .map_err(binding_error)?;
        let serialized_payload = snapshot.serialized_payload;
        let graph = serialized_payload.take().map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let source = snapshot.source;
        let future = async move {
            host.check_retained_file_input_registration(&source, &state.registration)?;
            validated
                .mutate_serialized(graph, serialized_payload)
                .await
                .map_err(serialized_payload_mutation_language_error)?;
            Ok(progress)
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        let execution = Box::pin(execution.run(Box::pin(future)));
        let ffi = self.ffi.clone();
        env.spawn_future_with_callback(async move { Ok(execution.await) }, move |env, ticket| {
            let parked = ticket.park(&ffi).map_err(binding_error)?;
            let phase = native_phase.park(parked).map_err(binding_error)?;
            phase
                .inspect(InputRecord::mutation_snapshot)
                .map_err(binding_error)?
                .map_err(binding_error)?;
            phase
                .finish_terminal(|record| {
                    record.stage = Stage::Mutated;
                    record.serialized_payload = None;
                    record.roots = 0;
                    Ok(())
                })
                .map_err(binding_error)?;
            ().into_unknown(env)
        })
    }
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn file_input_preparation_metrics(&self) -> napi::Result<FileInputPreparationMetrics> {
        self.file_input_preparations
            .metrics()
            .map_err(binding_error)
    }
}

fn payload_result<'env>(
    env: &'env Env,
    converted: &preflight::Converted<'env>,
) -> napi::Result<Unknown<'env>> {
    if converted.kind == preflight::InputKind::Paths {
        return Err(binding_error(HostError::UnsupportedOperation(
            "retained upload path preparation is not implemented".into(),
        )));
    }
    let mut value = ptr::null_mut();
    status(unsafe { sys::napi_create_object(env.raw(), &mut value) })?;
    let property = sys::napi_property_descriptor {
        utf8name: c"payloads".as_ptr(),
        name: ptr::null_mut(),
        method: None,
        getter: None,
        setter: None,
        value: converted.items.raw(),
        attributes: sys::PropertyAttributes::writable
            | sys::PropertyAttributes::enumerable
            | sys::PropertyAttributes::configurable,
        data: ptr::null_mut(),
    };
    // Match an ordinary object literal, not a null-prototype diagnostics DTO.
    // Own data definition bypasses setters; inherited then remains observable.
    status(unsafe { sys::napi_define_properties(env.raw(), value, 1, &property) })?;
    Ok(unsafe { Unknown::from_raw_unchecked(env.raw(), value) })
}

/// Stateless language-kernel diagnostic: grants no session, ticket or native
/// authority and retains no Node roots or task across callbacks.
#[napi]
pub fn inspect_file_input_preflight_owner<'env>(
    env: &'env Env,
    receiver: Unknown<'env>,
    ports: Object<'env>,
) -> napi::Result<Unknown<'env>> {
    preflight::owner(env, receiver, &preflight::LanguagePorts::read(ports)?)
}

#[napi]
pub fn inspect_file_input_preflight_context<'env>(
    env: &'env Env,
    owner: Unknown<'env>,
    ports: Object<'env>,
) -> napi::Result<Unknown<'env>> {
    preflight::context(env, owner, &preflight::LanguagePorts::read(ports)?)
}

#[napi]
pub fn inspect_file_input_preflight_conversion<'env>(
    env: &'env Env,
    context: Unknown<'env>,
    files: Unknown<'env>,
    ports: Object<'env>,
) -> napi::Result<Unknown<'env>> {
    let converted =
        preflight::convert(env, context, files, &preflight::LanguagePorts::read(ports)?)?;
    payload_result(env, &converted)
}

fn assemble<'env>(
    env: &'env Env,
    converted: Unknown<'env>,
    options: Unknown<'env>,
    settings: &TimeoutSettings,
    ports: Object<'env>,
) -> napi::Result<Unknown<'env>> {
    let timeout = ports.get_named_property("timeout")?;
    let language = argument_assembly::LanguagePorts::read(ports)?;
    argument_assembly::assemble(env, converted, options, &language, || {
        timeout_policy::resolve(env, options, settings, timeout)
    })
}

/// Offline shared-kernel entrypoint. Typed client settings grant no browser or
/// preparation authority. No input root or asynchronous task is retained.
#[napi]
pub fn inspect_file_input_arguments<'env>(
    env: &'env Env,
    converted: Unknown<'env>,
    options: Unknown<'env>,
    settings: &NativeFileInputTimeoutSettings,
    ports: Object<'env>,
) -> napi::Result<Unknown<'env>> {
    assemble(env, converted, options, &settings.settings, ports)
}

/// Offline raw-buffer channel-kernel diagnostic. Returns a synchronous raw
/// language value, never assimilating a user map/valueOf result or then getter.
/// This grants no session/native/stream authority and stores no persistent root.
#[napi]
pub fn inspect_file_input_channel<'env>(
    env: &'env Env,
    params: Unknown<'env>,
    ports: Object<'env>,
) -> napi::Result<Unknown<'env>> {
    channel_validation::validate(env, params, ports)
}

/// Server-direction raw-buffer schema diagnostic, without dispatcher authority.
#[napi]
pub fn inspect_file_input_server_channel<'env>(
    env: &'env Env,
    params: Unknown<'env>,
    ports: Object<'env>,
) -> napi::Result<Unknown<'env>> {
    channel_validation::validate_server(env, params, ports)
}

/// Synchronous in-memory payload helper diagnostic. Its outer return must cross
/// a real JS async return to model upstream assimilation. Not native submission.
#[napi]
pub fn inspect_file_input_materialization<'env>(
    env: &'env Env,
    params: Unknown<'env>,
    ports: Object<'env>,
) -> napi::Result<Unknown<'env>> {
    materialization::materialize(env, params, ports)
}

/// Synchronous raw-carrier projection diagnostic. No roots or native authority
/// survive this call; actual preparation integration must first finish marking.
#[napi]
pub fn inspect_file_input_projection<'env>(
    env: &'env Env,
    items: Unknown<'env>,
    ports: Object<'env>,
) -> napi::Result<Unknown<'env>> {
    projection::inspect(env, items, ports)
}

#[cfg(test)]
mod original_handle_tests;

#[cfg(test)]
mod timeout_scope_tests {
    use super::*;

    #[test]
    fn environment_seal_blocks_every_scope_before_payload_destruction() {
        // Arrange
        let preparations = FileInputPreparations::new().unwrap();
        let first = preparations.scope().unwrap();
        let second = preparations.scope().unwrap();
        // Act
        preparations.seal();
        // Assert
        assert!(matches!(
            first.timeout_policies(),
            Err(HostError::TargetClosed)
        ));
        assert!(matches!(
            second.timeout_policies(),
            Err(HostError::TargetClosed)
        ));
        assert!(matches!(preparations.scope(), Err(HostError::TargetClosed)));
        assert!(matches!(
            &*second.1.borrow(),
            TimeoutPolicySlot::Uninitialized
        ));
    }

    #[test]
    fn uninitialized_scope_rejects_client_policy_lookup() {
        // Arrange
        let preparations = FileInputPreparations::new().unwrap();
        let scope = preparations.scope().unwrap();
        // Act
        let result = scope.timeout_policies();
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn closing_a_scope_seals_timeout_lookup_through_every_clone() {
        // Arrange
        let preparations = FileInputPreparations::new().unwrap();
        let scope = preparations.scope().unwrap();
        let retained = scope.clone();
        // Act
        scope.close();
        // Assert
        assert!(matches!(
            retained.timeout_policies(),
            Err(HostError::TargetClosed)
        ));
        assert!(matches!(&*retained.1.borrow(), TimeoutPolicySlot::Closed));
    }

    #[test]
    fn scope_retirement_is_idempotent_and_does_not_seal_a_new_scope() {
        // Arrange
        let preparations = FileInputPreparations::new().unwrap();
        let old = preparations.scope().unwrap();
        old.close();
        let new = preparations.scope().unwrap();
        // Act
        old.close();
        // Assert
        assert!(matches!(
            old.timeout_policies(),
            Err(HostError::TargetClosed)
        ));
        assert!(matches!(
            new.timeout_policies(),
            Err(HostError::InvalidArgument(_))
        ));
    }
}
