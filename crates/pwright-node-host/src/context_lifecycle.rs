//! Explicitly selected Context services; ownership stays in Rust topology.

use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_cap_context_creation::BrowserNewContext;
use pwright_cap_context_lifecycle::BrowserContextClose;
use pwright_composition_chromium::{AttachedBrowserNewContextService, AttachedChromiumComposition};
use pwright_core::{CallId, CallMetadata, Command, ObjectId, Service};
use pwright_runtime_tokio::TokioCleanupExecutor;

use crate::lifecycle::{DriverRequest, ordinary_close, outcome};
use crate::{CommandRuntime, ContextDescriptor, HostError, HostSession};

pub(crate) struct ContextLifecycleServices {
    create: AttachedBrowserNewContextService<TokioCleanupExecutor>,
    close: crate::close_services::ContextCloseService,
    metadata: crate::context_metadata::ContextMetadata,
    trees: std::sync::Arc<crate::frame_trees::FrameTrees>,
}

impl ContextLifecycleServices {
    pub fn new(
        composition: &AttachedChromiumComposition,
        runtime: &CommandRuntime,
        cleanup: TokioCleanupExecutor,
        metadata: crate::context_metadata::ContextMetadata,
        trees: std::sync::Arc<crate::frame_trees::FrameTrees>,
    ) -> Result<Self, HostError> {
        let create = composition
            .browser_new_context_service(runtime.lifecycle_handle(), cleanup.clone())
            .map_err(HostError::backend)?
            .ok_or_else(|| {
                HostError::UnsupportedOperation("Context creation profile is not installed".into())
            })?;
        let close =
            crate::close_services::ContextCloseService::new(composition, runtime, cleanup, &trees)?;
        Ok(Self {
            create,
            close,
            metadata,
            trees,
        })
    }

    pub async fn create(
        &self,
        call_id: CallId,
        browser: ObjectId,
        metadata: crate::context_metadata::ContextReservation,
        #[cfg(feature = "extra-headers")] headers: Option<crate::extra_headers::CreationHeaders>,
    ) -> Result<ContextDescriptor, HostError> {
        let collection = self.trees.reserve_creation_collection()?;
        let context = outcome(
            self.create
                .call(Command::<BrowserNewContext>::new(
                    call_id,
                    browser,
                    CallMetadata::default(),
                    (),
                ))
                .await,
        )?;
        #[cfg(feature = "extra-headers")]
        if let Some(headers) = headers
            && let Err(error) = headers.commit(context.object_id)
        {
            self.close(call_id, context.object_id).await.map_err(|cleanup| HostError::Backend(format!(
                "Context header installation failed: {error}; compensation failed: {cleanup}")))?;
            return Err(error);
        }
        let metadata = match metadata.commit(context.object_id).and_then(|metadata| {
            collection.commit()?;
            Ok(metadata)
        }) {
            Ok(metadata) => metadata,
            Err(error) => {
                self.close(call_id, context.object_id).await.map_err(|cleanup| HostError::Backend(format!(
                    "Context metadata commit failed: {error}; compensation failed: {cleanup}")))?;
                return Err(error);
            }
        };
        Ok(ContextDescriptor {
            id: context.object_id.get().to_string(),
            settings: metadata.snapshot,
            settings_binding: Some(metadata.binding),
        })
    }

    pub async fn close(&self, call_id: CallId, context: ObjectId) -> Result<(), HostError> {
        ordinary_close(outcome(
            self.close
                .call(Command::<BrowserContextClose>::new(
                    call_id,
                    context,
                    CallMetadata::default(),
                    (),
                ))
                .await,
        ))?;
        self.metadata.remove(context)
    }
}

impl HostSession {
    pub async fn new_context(&self) -> Result<ContextDescriptor, HostError> {
        self.new_context_with_base_url(None).await
    }

    pub async fn new_context_with_base_url(
        &self,
        base_url: Option<&str>,
    ) -> Result<ContextDescriptor, HostError> {
        self.new_context_with_options(
            base_url,
            #[cfg(feature = "extra-headers")]
            None,
        )
        .await
    }

    /// Selected creation port. Desired headers are published atomically with the
    /// Context; HTTP syntax is deferred until a Page installs the merged values.
    #[cfg(feature = "extra-headers")]
    pub async fn new_context_with_extra_http_headers_native(
        &self,
        base_url: Option<&str>,
        headers: &[crate::extra_headers::HeaderRef<'_>],
    ) -> Result<ContextDescriptor, HostError> {
        self.new_context_with_options(base_url, Some(headers)).await
    }

    async fn new_context_with_options(
        &self,
        base_url: Option<&str>,
        #[cfg(feature = "extra-headers")] headers: Option<&[crate::extra_headers::HeaderRef<'_>]>,
    ) -> Result<ContextDescriptor, HostError> {
        let (browser, call_id, _operation) = self.admit_operation(&self.browser_id)?;
        prune_metadata(&self.state, &self.context_metadata)?;
        let metadata = self.context_metadata.reserve_creation(base_url)?;
        #[cfg(feature = "extra-headers")]
        let headers = headers
            .map(|input| {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .extra_headers
                    .creation(input)
            })
            .transpose()?;
        let (reply, completion) = tokio::sync::oneshot::channel();
        self.send_driver_request(DriverRequest::NewContext {
            call_id,
            browser,
            // The reservation's body is already byte-admitted before boxing.
            metadata: Box::new(metadata),
            #[cfg(feature = "extra-headers")]
            headers,
            reply,
        })?;
        self.await_driver_reply(completion).await
    }

    pub async fn close_context(&self, context_id: &str) -> Result<(), HostError> {
        // Playwright's CDP default Context closes the attachment, not Chrome.
        // Do not keep an operation token while awaiting the host's own drain.
        match self.context_ownership(context_id)? {
            Some(CdpContextOwnership::Default) => return self.close().await,
            None => return Ok(()),
            Some(CdpContextOwnership::Owned | CdpContextOwnership::External) => {}
        }
        let result = self.close_owned_context(context_id).await;
        crate::lifecycle::settle_context_close(result, &self.stop, &self.disconnected).await
    }

    async fn close_owned_context(&self, context_id: &str) -> Result<(), HostError> {
        let (context, call_id, _operation) = self.admit_operation(context_id)?;
        let (reply, completion) = tokio::sync::oneshot::channel();
        self.send_driver_request(DriverRequest::CloseContext {
            call_id,
            context,
            reply,
        })?;
        self.await_driver_reply(completion).await
    }

    fn context_ownership(
        &self,
        context_id: &str,
    ) -> Result<Option<CdpContextOwnership>, HostError> {
        let id = context_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable object ID".into()))?;
        let state = self.state.lock().map_err(HostError::backend)?;
        let Some(live) = state.as_ref() else {
            return Ok(None);
        };
        let topology = live.composition.topology().map_err(HostError::backend)?;
        Ok(topology
            .topology()
            .browser_context_routes()
            .find(|context| context.object_id().get() == id)
            .map(|context| context.ownership()))
    }
}

pub(crate) fn prune_metadata(
    state: &std::sync::Mutex<Option<crate::LiveSession>>,
    metadata: &crate::context_metadata::ContextMetadata,
) -> Result<(), HostError> {
    let state = state.lock().map_err(HostError::backend)?;
    let Some(live) = state.as_ref() else {
        return Ok(());
    };
    #[cfg(feature = "extra-headers")]
    live.extra_headers.prune(live)?;
    let topology = live.composition.topology().map_err(HostError::backend)?;
    metadata.retain_live(|id| {
        topology
            .topology()
            .browser_context_routes()
            .any(|context| context.object_id() == id)
    })
}
