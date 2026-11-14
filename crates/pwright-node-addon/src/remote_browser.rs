//! Thin Node-API lease for a supervised Playwright Server connection.
//! Browser protocol state and socket ownership stay in Rust.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use napi::{Env, JsValue, Unknown, bindgen_prelude::PromiseRaw, sys};
use napi_derive::napi;
use pwright_node_host::{
    HostError,
    ffi_budget::{FfiBudget, FfiLane},
};
use pwright_playwright_server_browser_ops::{
    BrowserOperationError, EmptyContextOptions, OperationMetadata, RemoteBrowser, RemoteContext,
    RemotePage,
};
use pwright_playwright_server_session::{RemoteBrowserSupervisor, SupervisorError};

#[cfg(feature = "playwright-server-connect-page-events")]
use crate::remote_page_notice::NativeRemotePageNotice;
#[cfg(feature = "playwright-server-connect-page-events")]
use pwright_node_host::frame_lookup::{GlobUrlResolution, PreparedGlob, URL_WORK_BYTES};
#[cfg(feature = "playwright-server-connect-page-events")]
use pwright_node_host::frame_tree::FrameTreeFactory;

use crate::{binding_error, boundary};

fn supervisor_error(error: SupervisorError) -> HostError {
    match error {
        SupervisorError::Client(error) => HostError::InvalidArgument(error.to_string()),
        SupervisorError::Terminal(failure) => HostError::Backend(failure.to_string()),
        SupervisorError::ActorStopped | SupervisorError::TargetGenerationChanged => {
            HostError::TargetClosed
        }
        SupervisorError::EventStreamDisabled => {
            HostError::Backend("remote event stream is not installed".into())
        }
    }
}

fn operation_error(error: BrowserOperationError) -> HostError {
    match error {
        BrowserOperationError::InvalidMetadata => {
            HostError::InvalidArgument("invalid remote Browser operation metadata".into())
        }
        BrowserOperationError::Disconnected => HostError::TargetClosed,
        BrowserOperationError::Supervisor(error) => supervisor_error(error),
        BrowserOperationError::Remote(_) => {
            HostError::Backend("remote Browser rejected operation".into())
        }
        BrowserOperationError::InvalidResult(reason) => HostError::Backend(reason.into()),
    }
}

fn operation_metadata(api_name: &str) -> Result<OperationMetadata, HostError> {
    let wall_time_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| HostError::Backend("system clock precedes the Unix epoch".into()))?
        .as_millis()
        .try_into()
        .map_err(|_| {
            HostError::Backend("system clock milliseconds exceed the supported range".into())
        })?;
    OperationMetadata::new(api_name, wall_time_ms).map_err(operation_error)
}

#[napi]
pub struct NativeRemoteBrowser {
    pub(crate) supervisor: Arc<RemoteBrowserSupervisor>,
    pub(crate) ffi: FfiBudget,
    #[cfg(feature = "playwright-server-connect-page-events")]
    pub(crate) glob_factory: FrameTreeFactory,
    #[cfg(feature = "playwright-server-connect-page-events")]
    pub(crate) page_events_enabled: bool,
}

#[napi]
impl NativeRemoteBrowser {
    /// Report the original listener exception without rejecting an unrelated
    /// close or terminal-watch Promise.
    #[napi]
    pub fn report_callback_exception(&self, env: Env, exception: Unknown<'_>) -> napi::Result<()> {
        // SAFETY: both values belong to this synchronous Node callback scope.
        let status = unsafe { sys::napi_fatal_exception(env.raw(), exception.raw()) };
        if status == sys::Status::napi_ok {
            Ok(())
        } else {
            Err(napi::Error::new(
                napi::Status::from(status),
                "Node listener exception reporting failed",
            ))
        }
    }

    #[napi]
    pub fn version(&self) -> String {
        self.supervisor.version().to_owned()
    }

    #[napi]
    pub fn name(&self) -> String {
        self.supervisor.name().to_owned()
    }

    #[napi]
    pub fn is_connected(&self) -> bool {
        self.supervisor.is_connected()
    }

    /// Optional additive port: an older session addon may connect, but must
    /// not publish Browser.newPage without implicit-Context cleanup support.
    #[napi]
    pub fn supports_context_close(&self) -> bool {
        true
    }

    /// A graph-backed inventory, never a guessed empty Context list.
    #[napi]
    pub fn context_guids(&self) -> Vec<String> {
        if !self.supervisor.is_connected() {
            return Vec::new();
        }
        let browser_guid = self.supervisor.guid();
        self.supervisor
            .snapshot()
            .objects
            .into_iter()
            .filter(|object| {
                object.object_type == "BrowserContext"
                    && object.parent.as_deref() == Some(browser_guid)
            })
            .map(|object| object.guid)
            .collect()
    }

    /// Only the empty options shape is currently implemented by the typed
    /// Rust browser-ops composition.
    #[napi]
    pub fn new_context<'env>(
        &self,
        env: &'env Env,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let supervisor = Arc::clone(&self.supervisor);
        let ffi = self.ffi.clone();
        #[cfg(feature = "playwright-server-connect-page-events")]
        let glob_factory = self.glob_factory.clone();
        #[cfg(feature = "playwright-server-connect-page-events")]
        let page_events_enabled = self.page_events_enabled;
        let metadata = operation_metadata("browser.newContext").map_err(binding_error)?;
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                let browser =
                    RemoteBrowser::new(Arc::clone(&supervisor)).map_err(operation_error)?;
                let context = browser
                    .new_context(EmptyContextOptions, &metadata)
                    .await
                    .map_err(operation_error)?;
                Ok::<NativeRemoteContext, HostError>(NativeRemoteContext {
                    context,
                    supervisor,
                    ffi,
                    #[cfg(feature = "playwright-server-connect-page-events")]
                    glob_factory,
                    #[cfg(feature = "playwright-server-connect-page-events")]
                    page_events_enabled,
                })
            },
            |_, context| Ok(context),
        )
    }

    /// The watcher holds no socket owner, so a pending JS Promise cannot
    /// prevent a finalized Browser lease from retiring its WebSocket.
    #[napi]
    pub fn wait_terminal<'env>(
        &self,
        env: &'env Env,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Watch, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let mut watcher = self.supervisor.terminal_watcher();
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                watcher.wait_terminal().await;
                Ok::<bool, HostError>(false)
            },
            |_, connected| Ok(connected),
        )
    }

    /// Disconnect only this client's transport; the server owns the browser.
    #[napi]
    pub fn close<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Close, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let supervisor = Arc::clone(&self.supervisor);
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                supervisor.close().await.map_err(supervisor_error)?;
                Ok::<(), HostError>(())
            },
            |_, ()| Ok(()),
        )
    }
}

#[cfg(feature = "playwright-server-connect-page-events")]
#[napi]
impl NativeRemoteBrowser {
    /// Ordered, single-consumer typed Page notices. The session leaf has
    /// already validated and bounded each notice before it crosses Node-API.
    #[napi]
    pub fn next_page_notice<'env>(
        &self,
        env: &'env Env,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Watch, boundary::READ_ALLOWANCE)
            .map_err(binding_error)?;
        let supervisor = Arc::clone(&self.supervisor);
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                let notice = supervisor
                    .next_page_notice()
                    .await
                    .map_err(supervisor_error)?;
                Ok::<NativeRemotePageNotice, HostError>(notice.into())
            },
            |_, notice| Ok(notice),
        )
    }
}

#[napi]
pub struct NativeRemoteContext {
    context: RemoteContext,
    supervisor: Arc<RemoteBrowserSupervisor>,
    ffi: FfiBudget,
    #[cfg(feature = "playwright-server-connect-page-events")]
    glob_factory: FrameTreeFactory,
    #[cfg(feature = "playwright-server-connect-page-events")]
    page_events_enabled: bool,
}

#[napi]
impl NativeRemoteContext {
    #[napi]
    pub fn guid(&self) -> String {
        self.context.guid().to_owned()
    }

    #[napi]
    pub fn is_open(&self) -> bool {
        self.context.is_open()
    }

    /// Return only Page GUIDs actually projected below this Context.
    #[napi]
    pub fn page_guids(&self) -> Vec<String> {
        if !self.context.is_open() {
            return Vec::new();
        }
        let context_guid = self.context.guid();
        self.supervisor
            .snapshot()
            .objects
            .into_iter()
            .filter(|object| {
                object.object_type == "Page" && object.parent.as_deref() == Some(context_guid)
            })
            .map(|object| object.guid)
            .collect()
    }

    #[napi]
    pub fn new_page<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let context = self.context.clone();
        #[cfg(feature = "playwright-server-connect-page-events")]
        let glob_factory = self.glob_factory.clone();
        #[cfg(feature = "playwright-server-connect-page-events")]
        let page_events_enabled = self.page_events_enabled;
        let metadata = operation_metadata("browserContext.newPage").map_err(binding_error)?;
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                let page = context.new_page(&metadata).await.map_err(operation_error)?;
                #[cfg(feature = "playwright-server-connect-page-events")]
                if page_events_enabled {
                    page.enable_network_events(&metadata)
                        .await
                        .map_err(operation_error)?;
                }
                Ok::<NativeRemotePage, HostError>(NativeRemotePage {
                    page,
                    #[cfg(feature = "playwright-server-connect-page-events")]
                    glob_factory,
                })
            },
            |_, page| Ok(page),
        )
    }

    #[napi]
    pub fn close<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Close, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let context = self.context.clone();
        let metadata = operation_metadata("browserContext.close").map_err(binding_error)?;
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                context.close(&metadata).await.map_err(operation_error)?;
                Ok::<(), HostError>(())
            },
            |_, ()| Ok(()),
        )
    }
}

#[cfg(feature = "playwright-server-connect-page-events")]
#[napi]
impl NativeRemoteContext {
    #[napi]
    pub fn created_sequence(&self) -> String {
        self.context.created_sequence().to_string()
    }
}

#[napi]
pub struct NativeRemotePage {
    page: RemotePage,
    #[cfg(feature = "playwright-server-connect-page-events")]
    glob_factory: FrameTreeFactory,
}

#[napi]
impl NativeRemotePage {
    #[napi]
    pub fn guid(&self) -> String {
        self.page.guid().to_owned()
    }

    #[napi]
    pub fn is_open(&self) -> bool {
        self.page.is_open()
    }
}

#[cfg(feature = "playwright-server-connect-page-events")]
#[napi]
impl NativeRemotePage {
    #[napi]
    pub fn created_sequence(&self) -> String {
        self.page.created_sequence().to_string()
    }

    #[napi]
    pub fn url_glob(&self, pattern: napi::JsString<'_>) -> napi::Result<String> {
        let pattern = boundary::bounded_string(pattern, 16 * 1024, "remote URL glob")?;
        let prepared = PreparedGlob::new(&self.glob_factory, &pattern).map_err(binding_error)?;
        let resolution = if let Some(relative) = prepared.relative_path() {
            let _work = self
                .glob_factory
                .reserve_conversion(URL_WORK_BYTES)
                .map_err(binding_error)?;
            url::Url::parse(relative).ok().map(|resolved| {
                let origin = resolved.origin().ascii_serialization();
                (resolved, origin)
            })
        } else {
            None
        };
        let plan = if let Some((resolved, origin)) = resolution.as_ref() {
            prepared.compile(
                &self.glob_factory,
                Some(GlobUrlResolution {
                    serialized: resolved.as_str(),
                    origin,
                }),
            )
        } else {
            prepared.compile(&self.glob_factory, None)
        }
        .map_err(binding_error)?;
        Ok(plan.pattern().to_owned())
    }
}
