//! Selected safe-host upload integration. N-API, JS buffers and browser launch
//! remain outside this module. The existing connection owns native topology.
mod command;
#[cfg(feature = "file-input-native-paths")]
mod native_paths;
pub mod profile;

use crate::{HostError, HostSession, locator_evaluation};
use pwright_backend_cdp_file_input::CdpFilePayloadService;
#[cfg(feature = "file-chooser-events")]
use pwright_backend_cdp_file_input::PayloadTarget;
#[cfg(feature = "file-chooser-events")]
use pwright_cdp::CdpClient;
use pwright_composition_chromium_file_input::FilePayloadOperationService;
use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger, ResourceRequest,
};
#[cfg(any(feature = "file-chooser-events", feature = "retained-file-input"))]
use pwright_file_input_fs::PreparedInput;
use pwright_file_input_fs::SystemFileSystem;
pub use pwright_file_input_fs::{MimeLookup, PinnedMime};
use pwright_file_input_source::{Content, PayloadSource, PreparedPayloads};
use pwright_file_input_tokio::{FileInputExecutor, FileInputJob};
#[cfg(feature = "file-chooser-events")]
use pwright_runtime::ReservedCleanupExecutor;
#[cfg(any(feature = "file-chooser-events", feature = "file-input-native-paths"))]
use pwright_runtime::{CleanupFailure, CleanupReservation};
use pwright_runtime_tokio::TokioCleanupExecutor;
#[cfg(any(feature = "file-chooser-events", feature = "file-input-native-paths"))]
use pwright_runtime_tokio::TokioCleanupReservation;
#[cfg(feature = "file-chooser-events")]
use std::sync::atomic::AtomicU64;
#[cfg(any(feature = "file-chooser-events", feature = "file-input-native-paths"))]
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

/// Borrowed bytes only. Node must validate actual non-shared backing before
/// supplying a slice; no pointer may escape the synchronous preparation call.
pub struct MemoryFile<'a> {
    pub name: &'a str,
    pub mime_type: Option<&'a str>,
    pub buffer: &'a [u8],
}
/// Move-only source plus its originating installation. Cannot be submitted to
/// another connection and cannot expose owned storage separately from its lease.
pub struct PreparedFileInput {
    input: PreparedFileInputKind,
    origin: Arc<()>,
    slot: SourceSlot,
}
pub(crate) enum PreparedFileInputKind {
    Payloads(PreparedPayloads),
    #[cfg(feature = "file-input-native-paths")]
    Native(
        pwright_file_input_fs::PreparedPaths<
            pwright_file_input_tokio::CancellableReader<std::fs::File>,
        >,
    ),
}
struct Slots {
    calls: AtomicUsize,
    sources: AtomicUsize,
    _state: ResourceLease,
}
pub(crate) struct SourceSlot(Arc<Slots>);
impl Drop for SourceSlot {
    fn drop(&mut self) {
        self.0.sources.fetch_sub(1, Ordering::AcqRel);
    }
}
pub(crate) struct CallLease {
    slot: Arc<Slots>,
    lease: Option<ResourceLease>,
}
impl Drop for CallLease {
    fn drop(&mut self) {
        self.lease = None;
        self.slot.calls.fetch_sub(1, Ordering::AcqRel);
    }
}
pub(crate) struct Installation {
    resolver: Arc<locator_evaluation::Installation>,
    pub(crate) upload: FilePayloadOperationService<TokioCleanupExecutor>,
    #[cfg(feature = "file-chooser-events")]
    backend: Arc<CdpFilePayloadService>,
    #[cfg(feature = "retained-file-input")]
    pub(crate) retained: crate::handles::file_input::RetainedUploads,
    #[cfg(feature = "retained-serialized-payload-import")]
    pub(crate) serialized: crate::handles::file_input::serialized_payload::Budget,
    worker: FileInputExecutor<SystemFileSystem, PinnedMime>,
    cleanup: TokioCleanupExecutor,
    #[cfg(feature = "file-input-native-paths")]
    pub(crate) native_paths: native_paths::Service,
    source: ResourceLedger,
    state: ResourceLedger,
    origin: Arc<()>,
    slots: Arc<Slots>,
    #[cfg(feature = "file-chooser-events")]
    next_direct_group: AtomicU64,
}

#[cfg(feature = "file-chooser-native-paths")]
async fn dispatch_colocated_paths(
    client: &dyn CdpClient,
    backend_node_id: i64,
    execution_context_id: i64,
    object_group: &str,
    files: &[String],
) -> Result<(), HostError> {
    let response = client
        .dom_resolve_node_in_group_to_settlement(
            backend_node_id,
            execution_context_id,
            object_group,
        )
        .await
        .map_err(HostError::backend)?;
    let object_id = resolved_node_object_id(&response)?;
    client
        .dom_set_file_input_files_by_object_id(object_id, files)
        .await
        .map_err(HostError::backend)
}

#[cfg(feature = "file-chooser-events")]
fn resolved_node_object_id(response: &serde_json::Value) -> Result<&str, HostError> {
    let object = response
        .get("object")
        .ok_or_else(|| HostError::Backend("file chooser resolve omitted RemoteObject".into()))?;
    if object.get("type").and_then(serde_json::Value::as_str) != Some("object")
        || object.get("subtype").and_then(serde_json::Value::as_str) != Some("node")
        || response.get("exceptionDetails").is_some()
    {
        return Err(HostError::Backend(
            "file chooser resolve did not return a DOM node".into(),
        ));
    }
    object
        .get("objectId")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 1024)
        .ok_or_else(|| {
            HostError::Backend("file chooser resolve returned an invalid object ID".into())
        })
}

impl Installation {
    pub(crate) fn new(
        resources: &ProductionResourceLedger,
        resolver: Arc<locator_evaluation::Installation>,
        runtime: tokio::runtime::Handle,
    ) -> Result<Self, HostError> {
        if std::mem::size_of::<Self>() + std::mem::size_of::<Slots>()
            > profile::STATE_BYTES as usize
        {
            return Err(HostError::ResourceExhausted(
                "file-input installation exceeds state budget".into(),
            ));
        }
        let source = resources
            .claim(profile::source_requirements())
            .map_err(HostError::backend)?;
        let state = resources
            .claim(profile::state_requirements())
            .map_err(HostError::backend)?;
        let lease = state
            .reserve(
                profile::STATE_SCOPE,
                ResourceClass::BackendStateBytes,
                profile::STATE_BYTES,
            )
            .map_err(HostError::backend)?;
        // One started upload; no private runtime or arbitrary cleanup deadline.
        // Native session retirement/transport shutdown precedes this drain.
        let cleanup = TokioCleanupExecutor::new(1);
        let backend = Arc::new(
            CdpFilePayloadService::new(profile::payload()?, resources)
                .map_err(HostError::backend)?,
        );
        Ok(Self {
            resolver,
            #[cfg(feature = "retained-file-input")]
            retained: crate::handles::file_input::RetainedUploads::new(resources)?,
            #[cfg(feature = "retained-serialized-payload-import")]
            serialized: crate::handles::file_input::serialized_payload::Budget::new(resources)?,
            upload: FilePayloadOperationService::new(
                profile::upload()?,
                resources,
                backend.clone(),
                cleanup.clone(),
            )
            .map_err(HostError::backend)?,
            #[cfg(feature = "file-chooser-events")]
            backend,
            worker: FileInputExecutor::new(
                profile::worker()?,
                resources,
                source.clone(),
                runtime,
                Arc::new(SystemFileSystem),
                Arc::new(PinnedMime),
            )
            .map_err(HostError::backend)?,
            cleanup,
            #[cfg(feature = "file-input-native-paths")]
            native_paths: native_paths::Service::new(resources)?,
            source,
            state,
            origin: Arc::new(()),
            slots: Arc::new(Slots {
                calls: AtomicUsize::new(0),
                sources: AtomicUsize::new(0),
                _state: lease,
            }),
            #[cfg(feature = "file-chooser-events")]
            next_direct_group: AtomicU64::new(1),
        })
    }
    fn source_slot(&self) -> Result<SourceSlot, HostError> {
        self.slots
            .sources
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < 2).then(|| count + 1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted("file-input source capacity exhausted".into())
            })?;
        Ok(SourceSlot(self.slots.clone()))
    }
    pub(crate) fn admit(&self) -> Result<CallLease, HostError> {
        self.slots
            .calls
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < profile::CALLS as usize).then(|| count + 1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted("file-input call capacity exhausted".into())
            })?;
        let mut owner = CallLease {
            slot: self.slots.clone(),
            lease: None,
        };
        owner.lease = Some(
            self.state
                .reserve_many(
                    profile::STATE_SCOPE,
                    &[
                        ResourceRequest::new(ResourceClass::ActiveCommand, 1),
                        ResourceRequest::new(ResourceClass::BackendStateBytes, profile::CALL_BYTES),
                    ],
                )
                .map_err(HostError::backend)?,
        );
        Ok(owner)
    }
    pub(crate) fn prepare(&self, files: &[MemoryFile<'_>]) -> Result<PreparedFileInput, HostError> {
        if files.len() > profile::ITEMS {
            return Err(HostError::InvalidArgument(
                "file-input count exceeds selected limit".into(),
            ));
        }
        if files.iter().any(|file| {
            file.name.len() > profile::NAME_BYTES
                || file
                    .mime_type
                    .is_some_and(|mime| mime.len() > profile::MIME_BYTES)
        }) {
            return Err(HostError::InvalidArgument(
                "file-input metadata exceeds selected limit".into(),
            ));
        }
        // Source-view allocation and MIME projection precede materialization,
        // so their own temporary admission cannot depend on the source lease.
        let _view = self.admit()?;
        let slot = self.source_slot()?;
        let mut sources: Vec<_> = files
            .iter()
            .map(|file| PayloadSource {
                name: file.name,
                mime_type: file
                    .mime_type
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| {
                        PinnedMime
                            .lookup(file.name)
                            .unwrap_or("application/octet-stream")
                    }),
                last_modified_ms: None,
                content: Content::Bytes(file.buffer),
            })
            .collect();
        let payloads = PreparedPayloads::materialize(
            &self.source,
            profile::SOURCE_SCOPE,
            profile::limits(),
            &mut sources,
        )
        .map_err(|error| match error {
            pwright_file_input_source::SourceError::Budget(error) => {
                HostError::ResourceExhausted(error.to_string())
            }
            pwright_file_input_source::SourceError::Policy(error) => {
                if error == pwright_file_input_model::FileInputError::PayloadSizeLimit {
                    // Upstream's semantic 50 MiB rejection is Error, not the
                    // TypeError used for malformed boundary arguments.
                    HostError::backend(error)
                } else {
                    HostError::InvalidArgument(error.to_string())
                }
            }
            other => HostError::backend(other),
        })?;
        Ok(PreparedFileInput {
            input: PreparedFileInputKind::Payloads(payloads),
            origin: self.origin.clone(),
            slot,
        })
    }
    fn paths(&self, base: &Path, paths: &[&str]) -> Result<FileInputJob<std::fs::File>, HostError> {
        self.worker
            .submit_paths(
                base,
                paths,
                pwright_file_input_model::BrowserLocation::Separate,
            )
            .map_err(HostError::backend)
    }

    #[cfg(any(
        feature = "retained-file-input",
        all(
            feature = "file-chooser-events",
            not(feature = "file-chooser-native-paths")
        )
    ))]
    pub(crate) async fn prepare_paths(
        &self,
        base: &Path,
        paths: &[&str],
    ) -> Result<PreparedFileInput, HostError> {
        let slot = self.source_slot()?;
        match self.paths(base, paths)?.await.map_err(HostError::backend)? {
            PreparedInput::Payloads(payloads) => Ok(PreparedFileInput {
                input: PreparedFileInputKind::Payloads(payloads),
                origin: self.origin.clone(),
                slot,
            }),
            PreparedInput::Native(prepared) => {
                #[cfg(feature = "file-input-native-paths")]
                {
                    Ok(PreparedFileInput {
                        input: PreparedFileInputKind::Native(prepared),
                        origin: self.origin.clone(),
                        slot,
                    })
                }
                #[cfg(not(feature = "file-input-native-paths"))]
                {
                    let _ = prepared;
                    Err(HostError::UnsupportedOperation(
                        "native directory uploads require a selected shared-filesystem and input-event completion path"
                            .into(),
                    ))
                }
            }
        }
    }

    #[cfg(feature = "file-chooser-native-paths")]
    pub(crate) async fn set_colocated_paths(
        &self,
        client: Arc<dyn CdpClient>,
        utility_world: Arc<str>,
        execution_context_id: i64,
        backend_node_id: i64,
        base: &Path,
        paths: &[&str],
    ) -> Result<(), HostError> {
        if execution_context_id <= 0 || backend_node_id <= 0 || utility_world.is_empty() {
            return Err(HostError::InvalidArgument(
                "file chooser requires a live utility world and backend node".into(),
            ));
        }
        let prepared = self
            .worker
            .submit_paths(
                base,
                paths,
                pwright_file_input_model::BrowserLocation::Colocated,
            )
            .map_err(HostError::backend)?
            .await
            .map_err(HostError::backend)?;
        let PreparedInput::Native(prepared) = prepared else {
            return Err(HostError::Backend(
                "colocated file chooser paths did not select native dispatch".into(),
            ));
        };
        let call = self.admit()?;
        let mut files = Vec::new();
        files
            .try_reserve_exact(prepared.paths().len())
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        for path in prepared.paths() {
            let value = path.to_str().ok_or_else(|| {
                HostError::InvalidArgument("file-input path must be valid UTF-8".into())
            })?;
            files.push(value.to_owned());
        }
        let group_id = self
            .next_direct_group
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| HostError::ResourceExhausted("file-input group IDs exhausted".into()))?;
        let group = format!("pwright.filechooser.{group_id}");
        let reservation = self.cleanup.try_reserve().map_err(HostError::backend)?;
        let operation = async move {
            let _call = call;
            let primary = dispatch_colocated_paths(
                client.as_ref(),
                backend_node_id,
                execution_context_id,
                &group,
                &files,
            )
            .await;
            let cleanup = client
                .runtime_release_object_group_to_settlement(&group)
                .await
                .map_err(HostError::backend);
            match (primary, cleanup) {
                (Ok(()), result) | (result, Ok(())) => result,
                (Err(primary), Err(cleanup)) => Err(HostError::backend(format_args!(
                    "{primary}; file chooser native cleanup failed: {cleanup}"
                ))),
            }
        };
        if std::mem::size_of_val(&operation) > profile::CALL_BYTES as usize {
            return Err(HostError::ResourceExhausted(
                "file chooser native path continuation exceeds admitted storage".into(),
            ));
        }
        DirectUploadFuture {
            inner: Some(Box::pin(operation)),
            reservation: Some(reservation),
            started: false,
        }
        .await
    }
    pub(crate) async fn shutdown(&self) -> Result<(), HostError> {
        self.worker.close_and_drain().await;
        #[cfg(feature = "retained-file-input")]
        let retained = self.retained.shutdown().await;
        let report = self.cleanup.shutdown().await;
        #[cfg(feature = "file-input-native-paths")]
        let native_paths = self.native_paths.shutdown().await;
        let uploaded = if report.metrics.failed != 0
            || report.metrics.timed_out != 0
            || !self.upload.is_cleanup_healthy()
        {
            Err(HostError::Backend(
                "file-input native cleanup failed".into(),
            ))
        } else {
            Ok(())
        };
        #[cfg(feature = "file-input-native-paths")]
        let uploaded = combine_native_cleanup(native_paths, uploaded);
        #[cfg(feature = "retained-file-input")]
        {
            combine_native_cleanup(retained, uploaded)
        }
        #[cfg(not(feature = "retained-file-input"))]
        {
            uploaded
        }
    }

    #[cfg(any(feature = "retained-file-input", feature = "file-chooser-events"))]
    pub(crate) fn take_payloads(
        &self,
        input: PreparedFileInput,
    ) -> Result<(PreparedPayloads, SourceSlot), HostError> {
        if !Arc::ptr_eq(&input.origin, &self.origin) {
            return Err(HostError::InvalidArgument(
                "file input belongs to another session".into(),
            ));
        }
        match input.input {
            PreparedFileInputKind::Payloads(payloads) => Ok((payloads, input.slot)),
            #[cfg(feature = "file-input-native-paths")]
            PreparedFileInputKind::Native(_) => Err(HostError::InvalidArgument(
                "native paths cannot be submitted as file payloads".into(),
            )),
        }
    }

    #[cfg(feature = "retained-file-input")]
    pub(crate) fn take_input(
        &self,
        input: PreparedFileInput,
    ) -> Result<(PreparedFileInputKind, SourceSlot), HostError> {
        if !Arc::ptr_eq(&input.origin, &self.origin) {
            return Err(HostError::InvalidArgument(
                "file input belongs to another session".into(),
            ));
        }
        Ok((input.input, input.slot))
    }

    #[cfg(feature = "file-chooser-events")]
    pub(crate) fn prepare_direct_upload(
        &self,
        client: Arc<dyn CdpClient>,
        utility_world: Arc<str>,
        execution_context_id: i64,
        backend_node_id: i64,
        input: PreparedFileInput,
    ) -> Result<DirectUploadFuture, HostError> {
        if execution_context_id <= 0 || backend_node_id <= 0 || utility_world.is_empty() {
            return Err(HostError::InvalidArgument(
                "file chooser requires a live utility world and backend node".into(),
            ));
        }
        let (payloads, source_slot) = self.take_payloads(input)?;
        let call = self.admit()?;
        let group_id = self
            .next_direct_group
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| HostError::ResourceExhausted("file-input group IDs exhausted".into()))?;
        let group = format!("pwright.filechooser.{group_id}");
        let reservation = self.cleanup.try_reserve().map_err(HostError::backend)?;
        let backend = self.backend.clone();
        let operation = async move {
            let _call = call;
            let _source_slot = source_slot;
            let primary = async {
                let response = client
                    .dom_resolve_node_in_group_to_settlement(
                        backend_node_id,
                        execution_context_id,
                        &group,
                    )
                    .await
                    .map_err(HostError::backend)?;
                let object_id = resolved_node_object_id(&response)?;
                let target =
                    PayloadTarget::new(client.as_ref(), utility_world.as_ref(), object_id, &group)
                        .map_err(HostError::backend)?;
                backend
                    .apply_on_target(target, &payloads)
                    .await
                    .map(|_| ())
                    .map_err(HostError::backend)
            }
            .await;
            let cleanup = client
                .runtime_release_object_group_to_settlement(&group)
                .await
                .map_err(HostError::backend);
            drop(payloads);
            match (primary, cleanup) {
                (Ok(()), result) | (result, Ok(())) => result,
                (Err(primary), Err(cleanup)) => Err(HostError::backend(format_args!(
                    "{primary}; file chooser native cleanup failed: {cleanup}"
                ))),
            }
        };
        if std::mem::size_of_val(&operation) > profile::CALL_BYTES as usize {
            return Err(HostError::ResourceExhausted(
                "file chooser upload continuation exceeds admitted storage".into(),
            ));
        }
        Ok(DirectUploadFuture {
            inner: Some(Box::pin(operation)),
            reservation: Some(reservation),
            started: false,
        })
    }
}

#[cfg(any(feature = "file-chooser-events", feature = "file-input-native-paths"))]
type DirectOperation = Pin<Box<dyn Future<Output = Result<(), HostError>> + Send>>;

/// A started file-input mutation owns its native group until cleanup settles,
/// even when the language-level waiter is cancelled or timed out.
#[must_use = "await or drop to transfer started work to native cleanup"]
#[cfg(any(feature = "file-chooser-events", feature = "file-input-native-paths"))]
pub(crate) struct DirectUploadFuture {
    inner: Option<DirectOperation>,
    reservation: Option<TokioCleanupReservation>,
    started: bool,
}

#[cfg(any(feature = "file-chooser-events", feature = "file-input-native-paths"))]
impl Unpin for DirectUploadFuture {}

#[cfg(any(feature = "file-chooser-events", feature = "file-input-native-paths"))]
impl Future for DirectUploadFuture {
    type Output = Result<(), HostError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let result = self
            .inner
            .as_mut()
            .expect("file-input upload polled after completion")
            .as_mut()
            .poll(context);
        if result.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        result
    }
}

#[cfg(any(feature = "file-chooser-events", feature = "file-input-native-paths"))]
impl Drop for DirectUploadFuture {
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        if let (Some(operation), Some(reservation)) = (self.inner.take(), self.reservation.take()) {
            reservation.submit(Box::pin(async move {
                operation.await.map_err(|_| {
                    CleanupFailure::new("file-input mutation or native cleanup failed")
                })
            }));
        }
    }
}

#[cfg(any(feature = "retained-file-input", feature = "file-input-native-paths"))]
pub(crate) fn combine_native_cleanup(
    adoption: Result<(), HostError>,
    upload: Result<(), HostError>,
) -> Result<(), HostError> {
    match (adoption, upload) {
        (Ok(()), result) | (result, Ok(())) => result,
        (Err(adoption), Err(upload)) => {
            Err(HostError::backend(format_args!("{adoption}; {upload}")))
        }
    }
}
impl HostSession {
    pub fn prepare_file_payloads(
        &self,
        files: &[MemoryFile<'_>],
    ) -> Result<PreparedFileInput, HostError> {
        let installation = self.file_input_installation()?;
        installation.prepare(files)
    }
    #[cfg(feature = "retained-file-input")]
    pub async fn prepare_file_paths(
        &self,
        base: &Path,
        paths: &[&str],
    ) -> Result<PreparedFileInput, HostError> {
        let installation = self.file_input_installation()?;
        installation.prepare_paths(base, paths).await
    }
    fn file_input_installation(&self) -> Result<Arc<Installation>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state
            .as_ref()
            .filter(|_| !self.stop.is_cancelled())
            .ok_or(HostError::TargetClosed)?;
        Ok(live.file_input.clone())
    }
}

#[cfg(test)]
mod live;
#[cfg(test)]
mod tests;
