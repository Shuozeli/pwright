//! Embedded Chromium session using the existing Pwright Rust v2 composition.
//! Node-API types are deliberately absent from this crate.

#[cfg(feature = "concurrent-close")]
mod close_jobs;
mod close_services;
mod collection_reads;
mod commands;
mod completion;
mod context_defaults;
mod context_lifecycle;
pub mod context_metadata;
#[cfg(feature = "drag-source")]
pub mod drag;
mod driver_diagnostics;
mod endpoint;
pub mod evaluation;
#[cfg(feature = "extra-headers")]
pub mod extra_headers;
pub mod ffi_budget;
#[cfg(feature = "file-input")]
pub mod file_input;
#[cfg(feature = "frame-lifecycle")]
mod frame_lifecycle;
pub mod frame_lookup;
mod frame_metadata;
#[cfg(feature = "frame-navigation")]
mod frame_navigation;
#[cfg(feature = "handles")]
pub mod handles;
#[cfg(feature = "frame-navigation")]
pub use frame_navigation::FrameGotoOutput;
pub mod frame_tree;
mod frame_trees;
pub mod input_start;
pub mod keyboard;
mod lifecycle;
pub mod locator;
mod locator_evaluation;
pub mod locator_query;
pub mod mouse;
mod profile;
mod reload;
pub mod screenshot;
mod shutdown_admission;

use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pwright_cdp::CdpSession;
use pwright_composition_chromium::{AttachedChromiumComposition, CdpPageServices};
use pwright_core::{ProductionResourceLedger, ResourceClass};
use pwright_runtime::ControlledDispatcher;
use pwright_runtime_tokio::{TokioCleanupExecutor, TokioDeadlineScheduler};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

type CommandRuntime = ControlledDispatcher<
    CdpPageServices,
    CdpPageServices,
    TokioDeadlineScheduler,
    TokioCleanupExecutor,
>;
pub use commands::{NavigationResult, ResponseMetadata};
pub use frame_metadata::{
    CollectionBatch, CollectionContext, CollectionMember, CollectionMembers, CollectionPage,
    CollectionRebaseError, CollectionReservation, CollectionRevision, CollectionView,
    CollectionVisibilityError, FrameMetadataMetrics, FramePublicationDrain, FramePublicationFence,
    FramePublications, FrameUrlSnapshot, LifecycleCause, LifecycleDelivery, LifecycleNotice,
    LifecycleNoticeView, LifecycleReservation, PublicationDrainError, PublicationProgress,
    VisibleMembers,
};
use frame_metadata::{FrameMetadata, FrameMetadataLimits};
use lifecycle::{DriverRequest, LifecycleServices};

#[derive(Clone, Copy, Debug)]
pub enum ConnectPhase {
    EndpointDiscovery,
    CdpAttachment,
    BrowserVersion,
    TopologyInitialization,
}

impl std::fmt::Display for ConnectPhase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EndpointDiscovery => "endpoint discovery",
            Self::CdpAttachment => "CDP attachment",
            Self::BrowserVersion => "browser version query",
            Self::TopologyInitialization => "topology initialization",
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    ResourceExhausted(String),
    #[error("{0}")]
    Backend(String),
    #[error("Timeout {0}ms exceeded")]
    Timeout(u32),
    #[error("Timeout {timeout_ms}ms exceeded during {phase}")]
    ConnectTimeout {
        timeout_ms: u32,
        phase: ConnectPhase,
    },
    #[error("Target page, context or browser has been closed")]
    TargetClosed,
    #[error("{0}")]
    UnsupportedOperation(String),
    #[error("{name}: {message}")]
    Javascript {
        name: String,
        message: String,
        stack: Option<String>,
    },
}

impl HostError {
    pub(crate) fn backend(error: impl std::fmt::Display) -> Self {
        let mut message = error.to_string();
        if message.len() > 4096 {
            let mut boundary = 4096;
            while !message.is_char_boundary(boundary) {
                boundary -= 1;
            }
            message.truncate(boundary);
        }
        Self::Backend(message)
    }

    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidArgument(_) => "PWRIGHT_INVALID_ARGUMENT",
            Self::ResourceExhausted(_) => "PWRIGHT_RESOURCE_EXHAUSTED",
            Self::Backend(_) => "PWRIGHT_BACKEND",
            Self::Timeout(_) | Self::ConnectTimeout { .. } => "PWRIGHT_TIMEOUT",
            Self::TargetClosed => "PWRIGHT_TARGET_CLOSED",
            Self::UnsupportedOperation(_) => "PWRIGHT_UNSUPPORTED",
            Self::Javascript { .. } => "PWRIGHT_JAVASCRIPT",
        }
    }
}

#[derive(Debug)]
pub struct ContextDescriptor {
    pub id: String,
    pub settings: context_metadata::ContextSnapshot,
    pub settings_binding: Option<context_metadata::ContextBinding>,
}

pub struct PageDescriptor {
    pub id: String,
    pub context_id: String,
    pub main_frame_id: String,
    pub main_frame_metadata: FrameUrlSnapshot,
    pub frame_tree: frame_tree::FrameTree,
    /// New-Page response only: reserved before native creation and transferred
    /// exactly once into the Node wrapper. Enumeration does not create Pages.
    pub main_frame_binding: Option<Box<frame_tree::FrameBinding>>,
    pub frame_tree_binding: frame_tree::FrameTreeBinding,
}

pub struct PageSnapshot {
    pub ids: Vec<String>,
    pub new_pages: Vec<PageDescriptor>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionMetrics {
    pub connected: bool,
    pub live_objects: u32,
    pub pending_calls: u32,
    pub topology_drivers: u32,
    pub active_leases: u32,
    pub live_scopes: u32,
    pub retained_event_bytes: u32,
    pub active_operations: u32,
    pub cleanup_in_flight: u32,
}

struct LiveSession {
    #[cfg(feature = "handles")]
    handles: Arc<handles::Installation>,
    #[cfg(feature = "extra-headers")]
    extra_headers: Arc<extra_headers::Installation>,
    #[cfg(feature = "frame-navigation")]
    frame_navigation: frame_navigation::Installation,
    #[cfg(feature = "frame-lifecycle")]
    frame_lifecycle: Arc<pwright_composition_chromium::frame_lifecycle::FrameLifecycleRegistry>,
    composition: AttachedChromiumComposition,
    runtime: Arc<CommandRuntime>,
    locator_evaluation: Arc<locator_evaluation::Installation>,
    #[cfg(feature = "file-input")]
    file_input: Arc<file_input::Installation>,
    screenshot_queue: Arc<screenshot::queue::Queue>,
    keyboard: Arc<keyboard::Installation>,
    mouse: Arc<mouse::Installation>,
    input_start: input_start::Order,
}

/// One environment-local session. Explicit close joins its topology driver.
/// Dropping the owner cancels the driver and releases transport references;
/// it never issues a browser-process close command.
pub struct HostSession {
    browser_id: String,
    state: Arc<Mutex<Option<LiveSession>>>,
    stop: CancellationToken,
    disconnected: CancellationToken,
    driver: tokio::sync::Mutex<Option<JoinHandle<Result<(), HostError>>>>,
    version: String,
    resources: ProductionResourceLedger,
    operations: TaskTracker,
    next_call: AtomicU64,
    cleanup: TokioCleanupExecutor,
    frame_metadata: Arc<FrameMetadata>,
    frame_trees: Arc<frame_trees::FrameTrees>,
    context_metadata: context_metadata::ContextMetadata,
    topology_barriers: tokio::sync::mpsc::Sender<DriverRequest>,
}

struct SessionTeardown {
    state: Arc<Mutex<Option<LiveSession>>>,
    disconnected: CancellationToken,
    frame_metadata: Arc<FrameMetadata>,
    frame_trees: Arc<frame_trees::FrameTrees>,
    context_metadata: context_metadata::ContextMetadata,
}

impl Drop for SessionTeardown {
    fn drop(&mut self) {
        // Poison recovery is cleanup-only: no commands can run on this state.
        let retired = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        #[cfg(feature = "retained-file-input")]
        if let Some(live) = retired.as_ref() {
            // No host lock remains. Failure/peer teardown must also revoke
            // parked FFI owners when no Node callback will ever resume.
            live.file_input.retained.seal_invocations().revoke();
        }
        #[cfg(feature = "handles")]
        if let Some(live) = retired.as_ref() {
            live.handles.close();
        }
        drop(retired);
        self.frame_trees.clear();
        self.context_metadata.clear();
        self.frame_metadata.clear();
        self.disconnected.cancel();
    }
}

#[derive(Clone, Copy)]
enum LifecyclePublication {
    Collections,
    Terminal,
    Events,
}

impl HostSession {
    pub async fn connect_over_cdp(endpoint: &str, timeout_ms: u32) -> Result<Self, HostError> {
        Self::connect_with_publication(endpoint, timeout_ms, LifecyclePublication::Collections)
            .await
    }

    /// Experimental native terminal-event selection. Consumers must use the
    /// lifecycle drain conversion/commit/acknowledgement protocol, not publish
    /// whole fences. This narrower entry point does not select ordinary close sources.
    pub async fn connect_over_cdp_with_terminal_events(
        endpoint: &str,
        timeout_ms: u32,
    ) -> Result<Self, HostError> {
        Self::connect_with_publication(endpoint, timeout_ms, LifecyclePublication::Terminal).await
    }

    /// Selected native ready/linkage and terminal/explicit/external close sources.
    /// Consumers must use notice conversion, commit and acknowledgement.
    pub async fn connect_over_cdp_with_lifecycle_events(
        endpoint: &str,
        timeout_ms: u32,
    ) -> Result<Self, HostError> {
        Self::connect_with_publication(endpoint, timeout_ms, LifecyclePublication::Events).await
    }

    async fn connect_with_publication(
        endpoint: &str,
        timeout_ms: u32,
        publication: LifecyclePublication,
    ) -> Result<Self, HostError> {
        endpoint::validate_endpoint(endpoint)?;
        let mut phase = ConnectPhase::EndpointDiscovery;
        let connect = Self::connect_inner(endpoint, timeout_ms, &mut phase, publication);
        if timeout_ms == 0 {
            connect.await
        } else {
            tokio::time::timeout(Duration::from_millis(u64::from(timeout_ms)), connect)
                .await
                .map_err(|_| HostError::ConnectTimeout { timeout_ms, phase })?
        }
    }

    async fn connect_inner(
        endpoint: &str,
        timeout_ms: u32,
        phase: &mut ConnectPhase,
        publication: LifecyclePublication,
    ) -> Result<Self, HostError> {
        let endpoint = endpoint::resolve(endpoint).await?;
        // Native command timeout remains bounded even for an unlimited public
        // connect wait. This profile choice is reported in compatibility docs.
        let command_timeout =
            Duration::from_millis(u64::from(if timeout_ms == 0 { 30_000 } else { timeout_ms }));
        *phase = ConnectPhase::CdpAttachment;
        let factory = if matches!(publication, LifecyclePublication::Events) {
            profile::attach_factory_with_native_close(command_timeout, true)?
        } else {
            profile::attach_factory(command_timeout)?
        };
        let mut composition = factory
            .attach(endpoint)
            .await
            .map_err(|error| HostError::backend(format_args!("CDP attach failed: {error}")))?;
        *phase = ConnectPhase::BrowserVersion;
        let value = CdpSession::browser(Arc::clone(composition.connection()))
            .send("Browser.getVersion", serde_json::json!({}))
            .await
            .map_err(|error| {
                HostError::backend(format_args!("Browser version query failed: {error}"))
            })?;
        let version: pwright_cdp::generated::browser::GetVersionReturns =
            serde_json::from_value(value).map_err(HostError::backend)?;
        if version.product.len() > 4096 {
            return Err(HostError::ResourceExhausted(
                "browser version metadata exceeds 4096 bytes".into(),
            ));
        }
        let headful = !version.user_agent.contains("Headless");
        let version = version
            .product
            .split_once('/')
            .map_or(version.product.as_str(), |(_, version)| version)
            .to_owned();
        // Existing runtime/lifecycle capacity plus two pre-reserved native
        // continuation slots for each of eight Locator evaluation commands.
        #[cfg(not(feature = "handles"))]
        let cleanup_slots = 32;
        #[cfg(feature = "handles")]
        let cleanup_slots = handles::CLEANUP_SLOTS;
        let cleanup =
            TokioCleanupExecutor::new_with_job_timeout(cleanup_slots, Duration::from_secs(5))
                .map_err(HostError::backend)?;
        #[cfg(not(feature = "handles"))]
        let runtime =
            composition.new_controlled_page_runtime(TokioDeadlineScheduler, cleanup.clone());
        #[cfg(feature = "handles")]
        let runtime = composition
            .new_controlled_page_runtime_with_handle_leaf(TokioDeadlineScheduler, cleanup.clone());
        let runtime = Arc::new(runtime.map_err(HostError::backend)?);
        let locator_evaluation = Arc::new(locator_evaluation::Installation::new(
            runtime.service(),
            composition.resources(),
            cleanup.clone(),
        )?);
        #[cfg(feature = "handles")]
        let handles = Arc::new(handles::Installation::new(
            &composition,
            &runtime,
            cleanup.clone(),
            locator_evaluation.world_access(),
        )?);
        #[cfg(feature = "file-input")]
        let file_input = Arc::new(file_input::Installation::new(
            composition.resources(),
            locator_evaluation.clone(),
            tokio::runtime::Handle::current(),
        )?);
        let frame_metadata = Arc::new(
            FrameMetadata::new(FrameMetadataLimits {
                entries: 128,
                url_bytes: 16 * 1024,
                total_bytes: 2 * 1024 * 1024,
                publications: 256,
            })?
            .with_names(frame_metadata::FrameNameLimits {
                name_bytes: 16 * 1024,
                total_bytes: 2 * 1024 * 1024,
            })?,
        );
        let context_metadata =
            context_metadata::ContextMetadata::new(context_metadata::ContextMetadataLimits {
                active_contexts: 16,
                states: 256,
                conversions: 128,
                base_url_bytes: 16 * 1024,
                bytes: 2 * 1024 * 1024,
            })?;
        let frame_trees = frame_trees::FrameTrees::new(frame_metadata.clone())?
            .with_collections(context_metadata.clone())?;
        let frame_trees = Arc::new(match publication {
            LifecyclePublication::Collections => frame_trees,
            LifecyclePublication::Terminal => frame_trees.with_terminal_publication()?,
            LifecyclePublication::Events => frame_trees
                .with_terminal_publication()?
                .with_explicit_close_publication()?
                .with_external_close_publication()?
                .with_ready_publication()?,
        });
        #[cfg(feature = "frame-lifecycle")]
        let frame_lifecycle = frame_lifecycle::install(
            composition.resources(),
            composition
                .topology()
                .map_err(HostError::backend)?
                .topology()
                .epoch()
                .get(),
        )?;
        #[cfg(feature = "frame-navigation")]
        let frame_navigation =
            frame_navigation::install(runtime.service(), composition.resources())?;
        let defaults = context_defaults::OwnedContextDefaults::new(
            composition.connection().clone(),
            headful,
            std::env::consts::OS,
        );
        #[cfg(feature = "extra-headers")]
        let extra_headers = extra_headers::Installation::new(
            composition.connection().clone(),
            composition.resources(),
        )?;
        #[cfg(feature = "extra-headers")]
        let defaults = extra_headers::Initializer::new(defaults, extra_headers.clone());
        let mut topology_driver = composition
            .topology_event_driver()
            .map_err(HostError::backend)?
            .ok_or_else(|| HostError::Backend("topology driver was not selected".into()))?
            .with_playwright_context_fallback()
            .with_session_initializer(Arc::new(defaults))
            .with_frame_url_observer(frame_trees.clone());
        #[cfg(all(feature = "frame-lifecycle", not(feature = "extra-headers")))]
        {
            topology_driver = topology_driver
                .with_frame_event_observer(frame_lifecycle.clone())
                .map_err(HostError::backend)?;
        }
        #[cfg(feature = "extra-headers")]
        {
            #[cfg(feature = "frame-lifecycle")]
            let frames = Some(frame_lifecycle.clone()
                as Arc<dyn pwright_composition_chromium::ChromiumFrameEventObserver>);
            #[cfg(not(feature = "frame-lifecycle"))]
            let frames = None;
            topology_driver = topology_driver
                .with_frame_event_observer(Arc::new(extra_headers::Observer::new(
                    extra_headers.clone(),
                    frames,
                )))
                .map_err(HostError::backend)?;
        }
        *phase = ConnectPhase::TopologyInitialization;
        topology_driver
            .initialize_snapshot(runtime.as_ref(), 4096)
            .await
            .map_err(|error| {
                HostError::backend(format_args!("Topology initialization failed: {error}"))
            })?;
        let closed = composition.connection().closed();
        let stop = CancellationToken::new();
        #[cfg(feature = "concurrent-close")]
        let mut close_jobs =
            close_jobs::Jobs::new(&composition, &runtime, &frame_trees, stop.clone())?;
        let lifecycle_services = LifecycleServices::new(
            &composition,
            runtime.as_ref(),
            cleanup.clone(),
            frame_metadata.clone(),
            frame_trees.clone(),
            context_metadata.clone(),
            stop.clone(),
        )?;
        let io_finished = composition.connection().io_finished();
        let resources = composition.resources().clone();
        let disconnected = CancellationToken::new();
        let browser_id = composition.browser_object_id().get().to_string();
        let mouse = mouse::Installation::new(composition.resources())?;
        #[cfg(feature = "drag-runtime")]
        let drag = Arc::new(drag::host::Installation::new(
            locator_evaluation.world_access(),
            composition.resources(),
        )?);
        #[cfg(feature = "drag-runtime")]
        let mouse = mouse.with_drag(drag.clone());
        let mouse = Arc::new(mouse);
        let keyboard = keyboard::Installation::new(composition.resources(), std::env::consts::OS)?;
        #[cfg(feature = "drag-runtime")]
        let keyboard = keyboard.with_drag(drag);
        let input_start = input_start::Order::new(composition.resources())?;
        #[cfg(feature = "handles")]
        let task_handles = handles.clone();
        let state = Arc::new(Mutex::new(Some(LiveSession {
            #[cfg(feature = "handles")]
            handles,
            #[cfg(feature = "extra-headers")]
            extra_headers,
            #[cfg(feature = "frame-navigation")]
            frame_navigation,
            #[cfg(feature = "frame-lifecycle")]
            frame_lifecycle,
            mouse: mouse.clone(),
            input_start: input_start.clone(),
            keyboard: Arc::new(keyboard),
            screenshot_queue: Arc::new(screenshot::queue::Queue::new(composition.resources())?),
            composition,
            runtime: Arc::clone(&runtime),
            locator_evaluation,
            #[cfg(feature = "file-input")]
            file_input: file_input.clone(),
        })));
        let task_state = Arc::clone(&state);
        let task_stop = stop.clone();
        let task_disconnected = disconnected.clone();
        let operations = TaskTracker::new();
        let task_operations = operations.clone();
        let task_cleanup = cleanup.clone();
        let task_metadata = frame_metadata.clone();
        let task_frame_trees = frame_trees.clone();
        let task_context_metadata = context_metadata.clone();
        let (topology_barriers, mut barriers) = tokio::sync::mpsc::channel::<DriverRequest>(8);
        let driver = tokio::spawn(async move {
            let teardown = SessionTeardown {
                state: Arc::clone(&task_state),
                disconnected: task_disconnected,
                frame_metadata: task_metadata,
                frame_trees: task_frame_trees,
                context_metadata: task_context_metadata.clone(),
            };
            #[cfg(feature = "extra-headers")]
            let mut header_jobs = extra_headers::Jobs::new();
            #[cfg(feature = "extra-headers")]
            let header_wakeup = header_jobs.wakeup();
            #[cfg(feature = "concurrent-close")]
            let close_wakeup = close_jobs.wakeup();
            // Cooperate with native jobs without cancelling/restarting an
            // in-progress topology or creation transaction. Native completions
            // park until the next serialized local-commit boundary.
            macro_rules! cooperate {
                ($future:expr) => {{
                    let future = $future;
                    #[cfg(feature = "extra-headers")]
                    let future = header_jobs.drive(future);
                    #[cfg(feature = "concurrent-close")]
                    let future = close_jobs.drive(future);
                    future.await
                }};
            }
            let (result, native_available) = tokio::select! {
                biased;
                () = task_stop.cancelled() => (Ok(()), true),
                () = closed => (Ok(()), false),
                result = async {
                    loop {
                        #[cfg(feature = "retained-target-marking")]
                        file_input.retained.prune_markers();
                        #[cfg(feature = "handles")]
                        task_handles.finish_ready()?;
                        #[cfg(feature = "concurrent-close")]
                        let mut reply = close_jobs.finish_next(&task_state)?;
                        #[cfg(not(feature = "concurrent-close"))]
                        let mut reply = None;
                        if reply.is_none() {
                        let mut barrier = None;
                        let received = topology_driver.receive_once_or_signal(runtime.as_ref(), async {
                            #[cfg(any(feature = "extra-headers", feature = "concurrent-close", feature = "handles"))]
                            tokio::select! {
                                received = barriers.recv() => { barrier = received; }
                                () = async {
                                    #[cfg(feature = "handles")]
                                    task_handles.changed().await;
                                    #[cfg(not(feature = "handles"))]
                                    std::future::pending::<()>().await;
                                } => {}
                                () = async {
                                    #[cfg(feature = "retained-target-marking")]
                                    file_input.retained.markers_changed().await;
                                    #[cfg(not(feature = "retained-target-marking"))]
                                    std::future::pending::<()>().await;
                                } => {}
                                () = async {
                                    #[cfg(feature = "extra-headers")]
                                    header_wakeup.notified().await;
                                    #[cfg(not(feature = "extra-headers"))]
                                    std::future::pending::<()>().await;
                                } => {}
                                () = async {
                                    #[cfg(feature = "concurrent-close")]
                                    close_jobs::notified(&close_wakeup).await;
                                    #[cfg(not(feature = "concurrent-close"))]
                                    std::future::pending::<()>().await;
                                } => {}
                            }
                            #[cfg(not(any(feature = "extra-headers", feature = "concurrent-close", feature = "handles")))]
                            { barrier = barriers.recv().await; }
                        });
                        cooperate!(received).map_err(HostError::backend)
                            .map_err(|error| driver_diagnostics::Phase::Receive.annotate(error))?;
                        context_lifecycle::prune_metadata(&task_state, &task_context_metadata)?;
                        #[cfg(feature = "extra-headers")]
                        cooperate!(extra_headers::reconcile_native(&task_state))?;
                        if let Some(barrier) = barrier {
                            #[cfg(feature = "extra-headers")]
                            let barrier = match barrier {
                                DriverRequest::ExtraHeaders(request) => {
                                    header_jobs.push(request)?;
                                    continue;
                                }
                                other => other,
                            };
                            #[cfg(feature = "concurrent-close")]
                            let barrier = match close_jobs.admit(&task_state, barrier)? {
                                close_jobs::Admission::Queued => continue,
                                close_jobs::Admission::Reply(completed) => { reply = Some(completed); None }
                                close_jobs::Admission::Other(request) => Some(request),
                            };
                            #[cfg(not(feature = "concurrent-close"))]
                            let barrier = Some(barrier);
                            if let Some(barrier) = barrier {
                                reply = Some(cooperate!(lifecycle_services.execute(&task_state, barrier)));
                            }
                        }
                        }
                        if let Some(reply) = reply {
                            let initialized = topology_driver.initialize_snapshot(runtime.as_ref(), 4096);
                            cooperate!(initialized).map_err(HostError::backend)
                                .map_err(|error| driver_diagnostics::Phase::Snapshot.annotate(error))?;
                            #[cfg(feature = "extra-headers")]
                            let reply = {
                                let (reply, closed) = cooperate!(lifecycle_services.settle_header_initialization(&task_state, reply))?;
                                if closed {
                                    cooperate!(topology_driver.initialize_snapshot(runtime.as_ref(), 4096)).map_err(HostError::backend)
                                        .map_err(|error| driver_diagnostics::Phase::Snapshot.annotate(error))?;
                                }
                                reply
                            };
                            context_lifecycle::prune_metadata(&task_state, &task_context_metadata)?;
                            #[cfg(feature = "extra-headers")]
                            cooperate!(extra_headers::reconcile_native(&task_state))?;
                            reply.send();
                        }
                    }
                } => (result, true),
            };
            // Stop caller-polled native work before closing the transport.
            // Pending updates record uncertainty; dropping a written CDP call
            // is not claimed as native compensation or a retirement receipt.
            #[cfg(feature = "extra-headers")]
            drop(header_jobs);
            // Queued header requests can retain their installation/connection.
            // Drop them before waiting for native I/O retirement, not when the
            // driver future itself finally drops after io_finished.
            barriers.close();
            while barriers.try_recv().is_ok() {}
            // Peer loss cannot run browser-side cleanup. Owner-requested
            // shutdown can: stop admissions/commands first, retire owned
            // inspector sessions with acknowledgements, then drain cleanup.
            let peer_close = if native_available {
                Ok(())
            } else {
                runtime
                    .close_transport()
                    .map(|_| ())
                    .map_err(HostError::backend)
            };
            #[cfg(feature = "retained-file-input")]
            let revoked = shutdown_admission::cancel_then_seal(
                &task_state,
                &task_stop,
                &task_operations,
                |state| {
                    state
                        .as_ref()
                        .map(|live| live.file_input.retained.seal_invocations())
                },
            )?;
            // Cancelled Running phases settle their own futures; parked
            // owners drop now. Do this BEFORE either caller or Handle drain.
            #[cfg(feature = "retained-file-input")]
            if let Some(batch) = revoked {
                batch.revoke();
            }
            #[cfg(not(feature = "retained-file-input"))]
            shutdown_admission::cancel_then_seal(
                &task_state,
                &task_stop,
                &task_operations,
                |_| (),
            )?;
            input_start.close();
            #[cfg(feature = "concurrent-close")]
            {
                close_jobs.drain().await;
                drop(close_jobs);
            }
            task_operations.wait().await;
            #[cfg(feature = "handles")]
            {
                // Revoke Ready results before executor shutdown. Rust/JS
                // holders can survive close without holding cleanup permits.
                let handles = task_state
                    .lock()
                    .map_err(HostError::backend)?
                    .as_ref()
                    .map(|live| live.handles.clone());
                if let Some(handles) = handles {
                    handles.close();
                }
            }
            drop(input_start);
            // Mouse result completion (or runtime transport loss) is not native
            // retirement. Its bounded independent workers keep polling receipts
            // while controlled cleanup jobs run, and must finish before teardown.
            #[cfg(feature = "drag-runtime")]
            let mouse_retirement = {
                let retirement = mouse.shutdown();
                tokio::pin!(retirement);
                if native_available {
                    // Keep exact world-destruction receipts advancing while
                    // Mouse owners drain. Admissions are already sealed; this
                    // does not resume user commands or lifecycle requests.
                    loop {
                        tokio::select! {
                            biased;
                            result = &mut retirement => break result,
                            event = topology_driver.receive_once(runtime.as_ref()) => {
                                if let Err(error) = event {
                                    let remaining = retirement.await;
                                    break remaining.and(Err(HostError::backend(error)));
                                }
                            }
                        }
                    }
                } else {
                    retirement.await
                }
            };
            #[cfg(not(feature = "drag-runtime"))]
            let mouse_retirement = mouse.shutdown().await;
            drop(mouse);
            // Admissions and caller operations are already stopped. Seal the
            // bounded receipt registry and retire only this connection's owned
            // inspector sessions, without polling topology concurrently. An
            // acknowledged detach retires wrappers, not borrowed Pages/Contexts.
            let retire_native_sessions = async {
                if native_available {
                    let (installation, connection) = {
                        let state = task_state.lock().map_err(HostError::backend)?;
                        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
                        (
                            live.locator_evaluation.clone(),
                            live.composition.connection().clone(),
                        )
                    };
                    installation
                        .retire_owned_native_sessions(&CdpSession::browser(connection))
                        .await
                } else {
                    Ok(())
                }
            };
            #[cfg(feature = "handles")]
            let retirement = retire_native_sessions.await;
            #[cfg(feature = "handles")]
            if retirement.is_err() {
                // Failed detach is not native destruction. Settle untimed
                // acquisitions as transport failures before draining their
                // continuations, and preserve the retirement error below.
                let connection = task_state
                    .lock()
                    .map_err(HostError::backend)?
                    .as_ref()
                    .map(|live| live.composition.connection().clone());
                if let Some(connection) = connection {
                    // Disconnect wakes pending CDP futures synchronously; a
                    // callback may reenter the host's already-closed gate.
                    connection.request_disconnect();
                }
            }
            #[cfg(feature = "handles")]
            let handle_retirement = {
                // Pending acquisitions retain their snapshot until settlement:
                // waiting before detach would strand them until cleanup expiry
                // and discard the registry witness. Ready results are already
                // revoked; their release failures still require BOTH the exact
                // detach acknowledgement and local object retirement. Continue
                // consuming detach events/committing releases until they drain.
                let result = {
                    let waiting = task_handles.wait();
                    tokio::pin!(waiting);
                    if native_available {
                        loop {
                            tokio::select! {
                                biased;
                                result = &mut waiting => break result,
                                event = topology_driver.receive_once(runtime.as_ref()) => {
                                    task_handles.native_event_processed();
                                    if let Err(error) = event {
                                        let remaining = waiting.await;
                                        break remaining.and(Err(HostError::backend(error)));
                                    }
                                }
                            }
                        }
                    } else {
                        waiting.await
                    }
                };
                drop(task_handles);
                result
            };
            #[cfg(not(feature = "handles"))]
            let handle_retirement = Ok(());
            // Keep the existing non-Handle teardown order unchanged.
            #[cfg(not(feature = "handles"))]
            let retirement = retire_native_sessions.await;
            // Native retirement above settles submitted uploads. Drain their
            // separately admitted continuations without an arbitrary job timer.
            #[cfg(all(feature = "file-input", not(feature = "handles")))]
            if retirement.is_err() {
                // Failed detach is not native destruction. End this client's
                // transport so untimed calls can settle as failures; preserve
                // the retirement error and never report clean browser cleanup.
                let connection = task_state
                    .lock()
                    .map_err(HostError::backend)?
                    .as_ref()
                    .map(|live| live.composition.connection().clone());
                if let Some(connection) = connection {
                    connection.request_disconnect();
                }
            }
            #[cfg(feature = "file-input")]
            let upload_cleanup = file_input.shutdown().await;
            #[cfg(feature = "file-input")]
            drop(file_input);
            let cleanup_report = task_cleanup.shutdown().await;
            let cleanup = runtime.close_transport().map_err(HostError::backend);
            // Release all routes and the native connection before publishing
            // terminal state. No live state is kept by the driver itself.
            drop(topology_driver);
            drop(lifecycle_services);
            drop(runtime);
            let retired = task_state.lock().map_err(HostError::backend)?.take();
            drop(retired);
            io_finished.await;
            drop(teardown);
            #[cfg(feature = "file-input")]
            let retirement = retirement.and(upload_cleanup);
            result
                .map_err(|error| driver_diagnostics::Phase::Driver.annotate(error))
                .and(handle_retirement)
                .and(
                    peer_close
                        .map_err(|error| driver_diagnostics::Phase::PeerTransport.annotate(error)),
                )
                .and(
                    mouse_retirement.map_err(|error| {
                        driver_diagnostics::Phase::MouseRetirement.annotate(error)
                    }),
                )
                .and(
                    retirement.map_err(|error| {
                        driver_diagnostics::Phase::SessionRetirement.annotate(error)
                    }),
                )
                .and(
                    cleanup
                        .map(|_| ())
                        .map_err(|error| driver_diagnostics::Phase::Transport.annotate(error)),
                )
                .and(
                    if cleanup_report.metrics.failed > 0 || cleanup_report.metrics.timed_out > 0 {
                        Err(HostError::Backend(
                            "native command cancellation cleanup failed".into(),
                        ))
                    } else {
                        Ok(())
                    },
                )
        });
        Ok(Self {
            browser_id,
            state,
            stop,
            disconnected,
            driver: tokio::sync::Mutex::new(Some(driver)),
            version,
            resources,
            operations,
            next_call: AtomicU64::new(1),
            cleanup,
            frame_metadata,
            frame_trees,
            context_metadata,
            topology_barriers,
        })
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn is_connected(&self) -> bool {
        !self.stop.is_cancelled() && !self.disconnected.is_cancelled()
    }

    pub fn disconnect_signal(&self) -> CancellationToken {
        self.disconnected.clone()
    }

    /// Cancels the environment owner even when in-flight FFI futures still
    /// hold safe Arc references. Joining remains an explicit close operation.
    pub fn request_shutdown(&self) {
        self.stop.cancel();
    }

    pub fn contexts(&self) -> Result<Vec<ContextDescriptor>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let Some(live) = state.as_ref() else {
            return Ok(Vec::new());
        };
        let topology = live.composition.topology().map_err(HostError::backend)?;
        let mut contexts = Vec::with_capacity(16);
        for context in topology.topology().browser_context_routes() {
            if let Some(settings) = self.context_metadata.visible(
                context.object_id(),
                context.ownership() == pwright_backend_cdp_topology::CdpContextOwnership::Owned,
            )? {
                contexts.push(ContextDescriptor {
                    id: context.object_id().get().to_string(),
                    settings,
                    settings_binding: None,
                });
            }
        }
        Ok(contexts)
    }

    pub fn context_metadata_metrics(
        &self,
    ) -> Result<context_metadata::ContextMetadataMetrics, HostError> {
        self.context_metadata.metrics()
    }
    pub fn reserve_context_conversion(
        &self,
        bytes: usize,
    ) -> Result<context_metadata::ContextConversion, HostError> {
        self.context_metadata.reserve_conversion(bytes)
    }

    pub fn pages(&self, context_id: &str) -> Result<Vec<PageDescriptor>, HostError> {
        Ok(self.page_snapshot(context_id, &[])?.new_pages)
    }

    pub fn reserve_metadata_conversion(
        &self,
        bytes: usize,
    ) -> Result<frame_tree::FrameConversion, HostError> {
        self.frame_trees.reserve_conversion(bytes)
    }

    /// One topology read; cached IDs avoid allocating native descriptor bindings.
    pub fn page_snapshot(
        &self,
        context_id: &str,
        known: &[String],
    ) -> Result<PageSnapshot, HostError> {
        // Cached IDs can include externally closed Pages; the retained-state
        // cap, not the live-Page cap, bounds this input.
        if known.len() > 256 || known.iter().any(|id| id.len() > 20) {
            return Err(HostError::InvalidArgument(
                "Known Page IDs exceed snapshot bounds".into(),
            ));
        }
        let state = self.state.lock().map_err(HostError::backend)?;
        let Some(live) = state.as_ref() else {
            return Ok(PageSnapshot {
                ids: Vec::new(),
                new_pages: Vec::new(),
            });
        };
        let topology = live.composition.topology().map_err(HostError::backend)?;
        let topology = topology.topology();
        let mut ids = Vec::new();
        let mut new_pages = Vec::new();
        for page in topology
            .page_session_routes()
            .filter(|page| page.browser_context_object_id().get().to_string() == context_id)
        {
            let id = page.object_id().get().to_string();
            let cached = known.contains(&id);
            ids.push(id.clone());
            if cached {
                continue;
            }
            let main_frame = topology
                .main_frame_object_id(page.target_id())
                .ok_or_else(|| HostError::Backend("Page has no main Frame".into()))?;
            let tree = self.frame_trees.get(page.object_id())?;
            new_pages.push(PageDescriptor {
                id,
                context_id: context_id.to_owned(),
                main_frame_id: main_frame.get().to_string(),
                main_frame_metadata: self.frame_metadata.get(main_frame)?,
                frame_tree_binding: tree.bind_tree()?,
                frame_tree: tree,
                main_frame_binding: None,
            });
        }
        Ok(PageSnapshot { ids, new_pages })
    }

    pub fn metrics(&self) -> Result<SessionMetrics, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let resources = self.resources.metrics().map_err(HostError::backend)?;
        let baseline = SessionMetrics {
            active_operations: u32::try_from(self.operations.len()).map_err(HostError::backend)?,
            cleanup_in_flight: u32::try_from(self.cleanup.metrics().in_flight)
                .map_err(HostError::backend)?,
            active_leases: u32::try_from(resources.active_leases).map_err(HostError::backend)?,
            live_scopes: u32::try_from(resources.live_scopes).map_err(HostError::backend)?,
            retained_event_bytes: u32::try_from(resources.class(ResourceClass::EventBytes).current)
                .map_err(HostError::backend)?,
            ..SessionMetrics::default()
        };
        let Some(live) = state.as_ref() else {
            return Ok(baseline);
        };
        Ok(SessionMetrics {
            connected: self.is_connected(),
            live_objects: u32::try_from(
                live.runtime
                    .live_object_count()
                    .map_err(HostError::backend)?,
            )
            .map_err(HostError::backend)?,
            pending_calls: u32::try_from(
                live.runtime
                    .pending_call_count()
                    .map_err(HostError::backend)?,
            )
            .map_err(HostError::backend)?,
            topology_drivers: 1,
            ..baseline
        })
    }

    /// Retained wrapper metadata intentionally has a separate lifetime from
    /// live runtime/transport resources, so it must not hide in their zeros.
    pub fn frame_metadata_metrics(&self) -> Result<FrameMetadataMetrics, HostError> {
        self.frame_metadata.metrics()
    }

    pub fn frame_tree_metrics(&self) -> Result<frame_tree::FrameTreeMetrics, HostError> {
        self.frame_trees.metrics()
    }

    pub fn frame_publications(&self) -> FramePublications {
        self.frame_metadata.publications()
    }

    async fn flush_topology(&self) -> Result<(), HostError> {
        let (done, completion) = tokio::sync::oneshot::channel();
        self.send_driver_request(DriverRequest::Fence(done))?;
        tokio::select! {
            biased;
            () = self.stop.cancelled() => Err(HostError::TargetClosed),
            result = completion => result.map_err(|_| HostError::TargetClosed),
        }
    }

    pub async fn close(&self) -> Result<(), HostError> {
        let mut driver = self.driver.lock().await;
        self.stop.cancel();
        if let Some(task) = driver.as_mut() {
            let result = task.await;
            driver.take();
            result.map_err(HostError::backend)??;
        }
        Ok(())
    }
}

impl Drop for HostSession {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::{ConnectPhase, HostError};

    #[test]
    fn connect_deadline_reports_a_bounded_phase_without_changing_timeout_classification() {
        // Arrange
        let error = HostError::ConnectTimeout {
            timeout_ms: 30_000,
            phase: ConnectPhase::TopologyInitialization,
        };
        // Act
        let diagnostic = error.to_string();
        // Assert
        assert_eq!(
            diagnostic,
            "Timeout 30000ms exceeded during topology initialization"
        );
        assert_eq!(error.code(), "PWRIGHT_TIMEOUT");
    }

    #[test]
    fn backend_diagnostics_are_bounded_in_utf8_bytes_not_characters() {
        // Arrange
        let diagnostic = "错误".repeat(4096);
        // Act
        let error = HostError::backend(diagnostic);
        // Assert
        let HostError::Backend(message) = error else {
            panic!("backend diagnostic expected");
        };
        assert!(message.len() <= 4096);
        assert!(message.len() >= 4093);
    }
}
