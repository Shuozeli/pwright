//! Selected safe-host upload integration. N-API, JS buffers and browser launch
//! remain outside this module. The existing connection owns native topology.
mod command;
pub mod profile;

use crate::{HostError, HostSession, locator_evaluation};
use pwright_backend_cdp_file_input::CdpFilePayloadService;
use pwright_composition_chromium_file_input::FilePayloadOperationService;
use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger, ResourceRequest,
};
use pwright_file_input_fs::SystemFileSystem;
pub use pwright_file_input_fs::{MimeLookup, PinnedMime};
use pwright_file_input_source::{Content, PayloadSource, PreparedPayloads};
use pwright_file_input_tokio::{FileInputExecutor, FileInputJob};
use pwright_runtime_tokio::TokioCleanupExecutor;
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
    payloads: PreparedPayloads,
    origin: Arc<()>,
    slot: SourceSlot,
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
    #[cfg(feature = "retained-file-input")]
    pub(crate) retained: crate::handles::file_input::RetainedUploads,
    #[cfg(feature = "retained-serialized-payload-import")]
    pub(crate) serialized: crate::handles::file_input::serialized_payload::Budget,
    worker: FileInputExecutor<SystemFileSystem, PinnedMime>,
    cleanup: TokioCleanupExecutor,
    source: ResourceLedger,
    state: ResourceLedger,
    origin: Arc<()>,
    slots: Arc<Slots>,
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
        Ok(Self {
            resolver,
            #[cfg(feature = "retained-file-input")]
            retained: crate::handles::file_input::RetainedUploads::new(resources)?,
            #[cfg(feature = "retained-serialized-payload-import")]
            serialized: crate::handles::file_input::serialized_payload::Budget::new(resources)?,
            upload: FilePayloadOperationService::new(
                profile::upload()?,
                resources,
                Arc::new(
                    CdpFilePayloadService::new(profile::payload()?, resources)
                        .map_err(HostError::backend)?,
                ),
                cleanup.clone(),
            )
            .map_err(HostError::backend)?,
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
            source,
            state,
            origin: Arc::new(()),
            slots: Arc::new(Slots {
                calls: AtomicUsize::new(0),
                sources: AtomicUsize::new(0),
                _state: lease,
            }),
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
    fn prepare(&self, files: &[MemoryFile<'_>]) -> Result<PreparedFileInput, HostError> {
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
            payloads,
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
    pub(crate) async fn shutdown(&self) -> Result<(), HostError> {
        self.worker.close_and_drain().await;
        #[cfg(feature = "retained-file-input")]
        let retained = self.retained.shutdown().await;
        let report = self.cleanup.shutdown().await;
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
        #[cfg(feature = "retained-file-input")]
        {
            combine_native_cleanup(retained, uploaded)
        }
        #[cfg(not(feature = "retained-file-input"))]
        {
            uploaded
        }
    }

    #[cfg(feature = "retained-file-input")]
    pub(crate) fn take_payloads(
        &self,
        input: PreparedFileInput,
    ) -> Result<(PreparedPayloads, SourceSlot), HostError> {
        if !Arc::ptr_eq(&input.origin, &self.origin) {
            return Err(HostError::InvalidArgument(
                "file input belongs to another session".into(),
            ));
        }
        Ok((input.payloads, input.slot))
    }
}

#[cfg(feature = "retained-file-input")]
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
