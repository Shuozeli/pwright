//! Embedded Chromium session using the existing Pwright Rust v2 composition.
//! Node-API types are deliberately absent from this crate.

mod browser_page;
#[cfg(feature = "bypass-csp")]
mod bypass_csp;
#[cfg(feature = "cdp-default-user-agent")]
pub mod cdp_default_user_agent;
#[cfg(feature = "cdp-session")]
mod cdp_sessions;
#[cfg(feature = "concurrent-close")]
mod close_jobs;
mod close_services;
mod collection_reads;
mod commands;
mod completion;
#[cfg(any(feature = "console-events", feature = "console-history"))]
mod console_events;
#[cfg(feature = "context-cookies")]
pub mod context_cookies;
mod context_defaults;
#[cfg(feature = "context-geolocation")]
mod context_geolocation;
#[cfg(feature = "context-geolocation-permission")]
mod context_geolocation_permission;
#[cfg(feature = "context-http-credentials")]
mod context_http_credentials;
mod context_lifecycle;
#[cfg(feature = "context-locale")]
mod context_locale;
pub mod context_metadata;
#[cfg(feature = "context-offline")]
mod context_offline;
#[cfg(feature = "context-proxy")]
mod context_proxy;
#[cfg(feature = "context-storage-state")]
pub mod context_storage_state;
#[cfg(feature = "context-timezone")]
mod context_timezone;
#[cfg(feature = "device-emulation")]
mod device_emulation;
#[cfg(feature = "download-events")]
pub mod download_events;
#[cfg(feature = "drag-source")]
pub mod drag;
mod driver_diagnostics;
#[cfg(feature = "element-handle-click")]
mod element_handle_click;
#[cfg(feature = "element-handle-dom")]
pub mod element_handle_dom;
#[cfg(feature = "element-handle-scroll-into-view")]
mod element_handle_scroll;
#[cfg(feature = "element-handle-select-option-value")]
mod element_handle_select_option_value;
#[cfg(feature = "element-handle-select-text")]
mod element_handle_select_text;
#[cfg(feature = "element-handle-wait-for-selector")]
pub mod element_handle_wait_for_selector;
#[cfg(feature = "emulate-media")]
mod emulate_media;
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
#[cfg(feature = "history-navigation")]
mod history_navigation;
#[cfg(feature = "ignore-https-errors")]
mod ignore_https_errors;
#[cfg(feature = "javascript-disabled")]
mod javascript_disabled;
#[cfg(any(feature = "context-locale", feature = "device-emulation"))]
mod session_initialization_queue;
#[cfg(any(feature = "context-locale", feature = "device-emulation"))]
mod user_agent_override;
#[cfg(feature = "worker-console-events")]
pub mod worker_console_events;
#[cfg(feature = "worker-events")]
pub mod worker_events;
#[cfg(feature = "worker-handle-evaluation")]
pub mod worker_handles;
#[cfg(feature = "frame-navigation")]
pub use frame_navigation::FrameGotoOutput;
#[cfg(feature = "navigation-url-match")]
pub use frame_navigation::NavigationMatchError;
pub mod frame_tree;
mod frame_trees;
#[cfg(feature = "init-scripts")]
pub mod init_scripts;
pub mod input_start;
pub mod keyboard;
#[cfg(feature = "layout-selectors")]
mod layout_selectors;
#[cfg(feature = "touchscreen-input")]
pub mod touchscreen;
#[cfg(feature = "layout-selectors")]
pub use layout_selectors::{MAX_LAYOUT_SELECTOR_PARTS, MAX_LAYOUT_SELECTOR_PROGRAM_BYTES};
mod lifecycle;
pub mod locator;
mod locator_evaluation;
pub mod locator_query;
pub mod mouse;
#[cfg(feature = "network-events")]
pub mod network_events;
#[cfg(feature = "owned-launch")]
mod owned_browser_shutdown;
#[cfg(feature = "page-bindings")]
mod page_bindings;
#[cfg(feature = "page-dialogs")]
pub mod page_dialogs;
#[cfg(feature = "page-errors")]
mod page_errors;
#[cfg(feature = "page-events")]
mod page_events;
#[cfg(feature = "file-chooser-events")]
pub mod page_file_chooser;
#[cfg(feature = "page-pause")]
mod page_pause;
#[cfg(feature = "page-pdf")]
pub mod page_pdf;
#[cfg(feature = "page-routes")]
mod page_routes;
#[cfg(feature = "page-viewport")]
pub mod page_viewport;
mod profile;
#[cfg(feature = "react-selectors")]
mod react_selector;
mod reload;
#[cfg(feature = "request-gc")]
mod request_gc;
#[cfg(feature = "response-body")]
pub mod response_body;
#[cfg(all(
    test,
    feature = "request-history",
    feature = "set-content",
    feature = "locator-dom-actions"
))]
mod scope_diagnostics;
pub mod screenshot;
#[cfg(feature = "selector-registration")]
pub mod selector_registry;
#[cfg(feature = "service-worker-policy")]
mod service_worker_policy;
#[cfg(feature = "set-content")]
mod set_content;
mod shutdown_admission;
#[cfg(feature = "timeout-settings")]
mod timeout_settings;
mod topology_recovery;
#[cfg(feature = "url-wait")]
mod url_wait;
#[cfg(feature = "wait-for-function")]
pub mod wait_for_function;
#[cfg(feature = "wait-for-timeout")]
pub mod wait_for_timeout;
#[cfg(all(
    test,
    feature = "cdp-handshake-headers",
    feature = "cdp-ws-close-reason"
))]
mod ws_close_diagnostics;
#[cfg(all(
    test,
    feature = "cdp-handshake-headers",
    feature = "cdp-ws-invalid-frame"
))]
mod ws_invalid_frame;
#[cfg(feature = "url-wait")]
pub use url_wait::UrlWaitError;

#[cfg(feature = "playwright-trace")]
use std::path::Path;
#[cfg(feature = "owned-launch")]
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(feature = "owned-launch")]
use pwright_browser_provider::LaunchedBrowser;
#[cfg(feature = "cdp-handshake-headers")]
use pwright_cdp::CdpHandshakeHeaders;
use pwright_cdp::CdpSession;
#[cfg(feature = "cdp-handshake-headers")]
pub use pwright_cdp::{
    CDP_HANDSHAKE_HEADER_MAX_BATCH_BYTES, CDP_HANDSHAKE_HEADER_MAX_COUNT,
    CDP_HANDSHAKE_HEADER_MAX_NAME_BYTES, CDP_HANDSHAKE_HEADER_MAX_VALUE_BYTES,
};
#[cfg(feature = "owned-launch")]
use pwright_composition_chromium::ChromiumAttachFactory;
use pwright_composition_chromium::{AttachedChromiumComposition, CdpPageServices};
#[cfg(feature = "owned-launch")]
use pwright_composition_chromium_launch::ChromiumLaunchCompositionFactory;
use pwright_core::{ProductionResourceLedger, ResourceClass};
#[cfg(feature = "owned-launch")]
use pwright_provider_chromium::{
    ChromiumBrowserShutdown, ChromiumLaunchRequest, ChromiumShutdownPolicy, ChromiumShutdownReport,
};
#[cfg(feature = "owned-launch")]
pub use pwright_provider_chromium::{
    ChromiumChannel, ChromiumExecutable, ChromiumPersistentProfile, discover_system_chromium,
};
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
#[cfg(feature = "cdp-session")]
pub use cdp_sessions::{
    CdpSessionEvent, CdpSessionEventBatch, MAX_CDP_SESSION_EVENT_BYTES, MAX_CDP_SESSION_EVENTS,
    MAX_CDP_SESSIONS,
};
pub use commands::{NavigationResult, ResponseMetadata};
#[cfg(feature = "context-geolocation")]
pub use context_geolocation::ContextGeolocation;
#[cfg(feature = "context-http-credentials")]
pub use context_http_credentials::{ContextHttpCredentials, MAX_HTTP_CREDENTIAL_FIELD_BYTES};
pub use context_lifecycle::ContextPolicyOptions;
#[cfg(feature = "context-api-request")]
pub use context_lifecycle::{ContextApiRequestCredentials, ContextApiRequestOptions};
#[cfg(feature = "context-locale")]
pub use context_locale::ContextLocale;
#[cfg(feature = "context-proxy")]
pub use context_proxy::ContextProxy;
#[cfg(feature = "context-storage-state")]
pub use context_storage_state::{
    ContextStorageItem, ContextStorageOrigin, ContextStorageState, ContextStorageStateSnapshot,
    MAX_STORAGE_ITEMS, MAX_STORAGE_ORIGINS, MAX_STORAGE_STATE_BYTES,
};
#[cfg(feature = "context-timezone")]
pub use context_timezone::ContextTimezone;
#[cfg(feature = "device-emulation")]
pub use device_emulation::{DeviceContextOptions, device_descriptors_json};
#[cfg(feature = "download-events")]
pub use download_events::{DownloadNotice, DownloadOwner};
#[cfg(feature = "context-forced-colors")]
pub use emulate_media::ContextForcedColors;
#[cfg(feature = "emulate-media")]
pub use emulate_media::PersistentContextMediaOptions;
#[cfg(feature = "extra-headers")]
pub use extra_headers::PersistentContextExtraHeaders;
#[cfg(feature = "page-dialogs")]
pub use frame_metadata::PageDialog;
#[cfg(feature = "file-chooser-events")]
pub use frame_metadata::PageFileChooser;
pub use frame_metadata::{
    CollectionBatch, CollectionContext, CollectionMember, CollectionMembers, CollectionPage,
    CollectionRebaseError, CollectionReservation, CollectionRevision, CollectionView,
    CollectionVisibilityError, ConsoleMessage, FrameMetadataMetrics, FramePublicationDrain,
    FramePublicationFence, FramePublications, FrameUrlSnapshot, LifecycleCause, LifecycleDelivery,
    LifecycleNotice, LifecycleNoticeView, LifecycleReservation, PageError, PublicationDrainError,
    PublicationProgress, VisibleMembers,
};
#[cfg(feature = "service-worker-policy")]
pub use service_worker_policy::ServiceWorkerPolicy;

#[cfg(feature = "persistent-context-launch-arguments")]
const MAX_PERSISTENT_LAUNCH_ARGUMENTS: usize = 256;
#[cfg(feature = "persistent-context-launch-arguments")]
const MAX_PERSISTENT_LAUNCH_ARGUMENT_BYTES: usize = 65_536;
#[cfg(feature = "persistent-context-launch-arguments")]
const MAX_PERSISTENT_LAUNCH_ARGUMENT_BATCH_BYTES: usize = 1_048_576;

/// Bounded Chromium switches for an explicitly selected persistent launch.
#[cfg(feature = "persistent-context-launch-arguments")]
#[derive(Default)]
pub struct PersistentLaunchArguments(Vec<Box<str>>);

#[cfg(feature = "persistent-context-launch-arguments")]
impl PersistentLaunchArguments {
    pub fn new(values: Vec<String>) -> Result<Self, HostError> {
        if values.len() > MAX_PERSISTENT_LAUNCH_ARGUMENTS {
            return Err(HostError::ResourceExhausted(format!(
                "persistent Context launch argument count exceeds {MAX_PERSISTENT_LAUNCH_ARGUMENTS}"
            )));
        }
        let mut bytes = 0usize;
        let mut arguments = Vec::new();
        arguments
            .try_reserve_exact(values.len())
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        for value in values {
            if value.len() > MAX_PERSISTENT_LAUNCH_ARGUMENT_BYTES {
                return Err(HostError::ResourceExhausted(format!(
                    "persistent Context launch argument exceeds {MAX_PERSISTENT_LAUNCH_ARGUMENT_BYTES} UTF-8 bytes"
                )));
            }
            if value.contains('\0') {
                return Err(HostError::InvalidArgument(
                    "persistent Context launch argument must not contain NUL".into(),
                ));
            }
            if !value.starts_with('-') {
                return Err(HostError::InvalidArgument(
                    "Arguments can not specify page to be opened".into(),
                ));
            }
            if [
                "--remote-debugging-address",
                "--remote-debugging-pipe",
                "--remote-debugging-port",
                "--user-data-dir",
            ]
            .iter()
            .any(|reserved| value.starts_with(reserved))
            {
                return Err(HostError::InvalidArgument(
                    "Pwright owns persistent profile and remote debugging arguments".into(),
                ));
            }
            bytes = bytes.checked_add(value.len()).ok_or_else(|| {
                HostError::ResourceExhausted(
                    "persistent Context launch argument byte count overflow".into(),
                )
            })?;
            if bytes > MAX_PERSISTENT_LAUNCH_ARGUMENT_BATCH_BYTES {
                return Err(HostError::ResourceExhausted(format!(
                    "persistent Context launch argument batch exceeds {MAX_PERSISTENT_LAUNCH_ARGUMENT_BATCH_BYTES} UTF-8 bytes"
                )));
            }
            arguments.push(value.into_boxed_str());
        }
        Ok(Self(arguments))
    }

    fn into_values(self) -> impl Iterator<Item = Box<str>> {
        self.0.into_iter()
    }
}

#[cfg(feature = "owned-launch")]
struct PersistentChromiumLaunchOptions<'a> {
    headless: bool,
    base_url: Option<&'a str>,
    context_defaults: PersistentContextDefaults,
    #[cfg(feature = "persistent-context-launch-arguments")]
    launch_arguments: PersistentLaunchArguments,
    timeout_ms: u32,
}
use frame_metadata::{FrameMetadata, FrameMetadataLimits};
pub use lifecycle::ClosePageDisposition;
use lifecycle::{DriverRequest, LifecycleServices};
#[cfg(feature = "page-bindings")]
pub use page_bindings::BindingCall;
#[cfg(feature = "route-fulfill-path")]
pub use page_routes::MAX_ROUTE_FULFILL_PATH_BYTES;
#[cfg(feature = "har-replay")]
pub use page_routes::{
    HarNotFoundAction, MAX_HAR_REPLAY_ARCHIVE_BYTES, MAX_HAR_REPLAY_ARCHIVES,
    MAX_HAR_REPLAY_RETAINED_BYTES,
};
#[cfg(feature = "page-routes")]
pub use page_routes::{
    MAX_ROUTE_CONTINUE_INPUT_BYTES, MAX_ROUTE_FULFILL_BODY_BYTES, MAX_ROUTE_INITIALIZING_SESSIONS,
    RouteAbortReason, RouteAction, RouteCall, RouteContinue, RouteFulfill,
};
#[cfg(feature = "worker-console-events")]
pub use worker_console_events::WorkerConsoleNotice;
#[cfg(feature = "worker-events")]
pub use worker_events::{WorkerNotice, WorkerNoticeKind, WorkerOwner};
#[cfg(feature = "worker-handle-evaluation")]
pub use worker_handles::{WorkerHandle, WorkerHandleRelease};

/// Optional policies admitted for the default Context of an owned persistent
/// Chromium launch. Each field is available only with its independently
/// selected persistent-context capability.
#[derive(Default)]
pub struct PersistentContextDefaults {
    #[cfg(feature = "context-geolocation")]
    geolocation: Option<ContextGeolocation>,
    #[cfg(feature = "context-geolocation-permission")]
    geolocation_permission: bool,
    #[cfg(feature = "context-http-credentials")]
    http_credentials: Option<ContextHttpCredentials>,
    #[cfg(feature = "context-locale")]
    locale: Option<ContextLocale>,
    #[cfg(feature = "context-timezone")]
    timezone: Option<ContextTimezone>,
    #[cfg(feature = "context-offline")]
    offline: bool,
    #[cfg(feature = "device-emulation")]
    device: Option<DeviceContextOptions>,
    #[cfg(feature = "ignore-https-errors")]
    ignore_https_errors: bool,
    #[cfg(feature = "bypass-csp")]
    bypass_csp: bool,
    #[cfg(feature = "javascript-disabled")]
    javascript_disabled: bool,
    #[cfg(feature = "emulate-media")]
    media: Option<PersistentContextMediaOptions>,
    #[cfg(feature = "extra-headers")]
    extra_headers: Option<PersistentContextExtraHeaders>,
}

impl PersistentContextDefaults {
    #[cfg(feature = "persistent-context-geolocation")]
    #[must_use]
    pub const fn with_geolocation(mut self, geolocation: Option<ContextGeolocation>) -> Self {
        self.geolocation = geolocation;
        self
    }

    #[cfg(feature = "persistent-context-geolocation-permission")]
    #[must_use]
    pub const fn with_geolocation_permission(mut self, enabled: bool) -> Self {
        self.geolocation_permission = enabled;
        self
    }

    #[cfg(feature = "persistent-context-http-credentials")]
    #[must_use]
    pub fn with_http_credentials(mut self, credentials: Option<ContextHttpCredentials>) -> Self {
        self.http_credentials = credentials;
        self
    }

    #[cfg(feature = "persistent-context-locale")]
    #[must_use]
    pub fn with_locale(mut self, locale: Option<ContextLocale>) -> Self {
        self.locale = locale;
        self
    }

    #[cfg(feature = "persistent-context-timezone")]
    #[must_use]
    pub fn with_timezone(mut self, timezone: Option<ContextTimezone>) -> Self {
        self.timezone = timezone;
        self
    }

    #[cfg(feature = "persistent-context-offline")]
    #[must_use]
    pub const fn with_offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    #[cfg(feature = "persistent-context-device-emulation")]
    #[must_use]
    pub fn with_device(mut self, device: Option<DeviceContextOptions>) -> Self {
        self.device = device;
        self
    }

    #[cfg(feature = "persistent-context-ignore-https-errors")]
    #[must_use]
    pub const fn with_ignore_https_errors(mut self, ignore_https_errors: bool) -> Self {
        self.ignore_https_errors = ignore_https_errors;
        self
    }

    #[cfg(feature = "persistent-context-bypass-csp")]
    #[must_use]
    pub const fn with_bypass_csp(mut self, bypass_csp: bool) -> Self {
        self.bypass_csp = bypass_csp;
        self
    }

    #[cfg(feature = "persistent-context-javascript-disabled")]
    #[must_use]
    pub const fn with_javascript_disabled(mut self, javascript_disabled: bool) -> Self {
        self.javascript_disabled = javascript_disabled;
        self
    }

    #[cfg(feature = "persistent-context-emulate-media")]
    #[must_use]
    pub const fn with_media(mut self, media: Option<PersistentContextMediaOptions>) -> Self {
        self.media = media;
        self
    }

    #[cfg(feature = "persistent-context-extra-headers")]
    #[must_use]
    pub fn with_extra_headers(mut self, headers: Option<PersistentContextExtraHeaders>) -> Self {
        self.extra_headers = headers;
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ConnectPhase {
    EndpointDiscovery,
    BrowserLaunch,
    CdpAttachment,
    BrowserVersion,
    TopologyInitialization,
}

impl std::fmt::Display for ConnectPhase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EndpointDiscovery => "endpoint discovery",
            Self::BrowserLaunch => "browser launch",
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
    #[error(
        "Timeout {timeout_ms}ms exceeded.\nwaiting for element to be visible, enabled and stable\n{progress}"
    )]
    RetainedClickTimeout { timeout_ms: u32, progress: String },
    #[error("{api}: Timeout {timeout_ms}ms exceeded.\nwaiting for {locator}\n{progress}")]
    ActionTimeout {
        api: &'static str,
        timeout_ms: u32,
        locator: String,
        progress: &'static str,
    },
    #[error("Timeout {timeout_ms}ms exceeded during {phase}")]
    ConnectTimeout {
        timeout_ms: u32,
        phase: ConnectPhase,
    },
    #[error("Target page, context or browser has been closed")]
    TargetClosed,
    #[error("Frame was detached")]
    FrameDetached,
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
            Self::Timeout(_)
            | Self::RetainedClickTimeout { .. }
            | Self::ActionTimeout { .. }
            | Self::ConnectTimeout { .. } => "PWRIGHT_TIMEOUT",
            Self::TargetClosed | Self::FrameDetached => "PWRIGHT_TARGET_CLOSED",
            Self::UnsupportedOperation(_) => "PWRIGHT_UNSUPPORTED",
            Self::Javascript { .. } => "PWRIGHT_JAVASCRIPT",
        }
    }
}

#[cfg(feature = "cdp-ws-close-reason")]
fn browser_version_query_error(
    error: &pwright_cdp::connection::CdpError,
    peer_close_reason: Option<&str>,
) -> HostError {
    match peer_close_reason {
        Some(reason) => HostError::backend(format_args!(
            "Browser version query failed: {error}\nBrowser logs:\n\n{reason}\n"
        )),
        None => HostError::backend(format_args!("Browser version query failed: {error}")),
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

pub struct BrowserPageDescriptor {
    pub context: ContextDescriptor,
    pub page: PageDescriptor,
}

pub struct PageSnapshot {
    pub ids: Vec<String>,
    pub new_pages: Vec<PageDescriptor>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameDescriptor {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub url: String,
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
    #[cfg(feature = "cdp-session")]
    cdp_sessions: Arc<cdp_sessions::Installation>,
    #[cfg(feature = "page-pause")]
    headful: bool,
    #[cfg(feature = "init-scripts")]
    init_scripts: Arc<init_scripts::Installation>,
    #[cfg(feature = "page-bindings")]
    page_bindings: Arc<page_bindings::Installation>,
    #[cfg(feature = "page-routes")]
    page_routes: Arc<page_routes::Installation>,
    #[cfg(feature = "page-errors")]
    page_errors: Arc<page_errors::Installation>,
    #[cfg(feature = "console-history")]
    console_history: Arc<console_events::Installation>,
    #[cfg(feature = "console-events")]
    console_subscriptions: Arc<console_events::Subscriptions>,
    #[cfg(feature = "emulate-media")]
    emulate_media: Arc<emulate_media::Installation>,
    #[cfg(feature = "handles")]
    handles: Arc<handles::Installation>,
    #[cfg(feature = "network-events")]
    network_events: Arc<network_events::Installation>,
    #[cfg(feature = "download-events")]
    download_events: Arc<download_events::Installation>,
    #[cfg(feature = "worker-events")]
    worker_events: Arc<worker_events::Installation>,
    #[cfg(feature = "worker-handle-evaluation")]
    worker_handles: Arc<worker_handles::Installation>,
    #[cfg(feature = "worker-console-events")]
    worker_console_events: Arc<worker_console_events::Installation>,
    #[cfg(feature = "extra-headers")]
    extra_headers: Arc<extra_headers::Installation>,
    #[cfg(feature = "frame-navigation")]
    frame_navigation: frame_navigation::Installation,
    #[cfg(feature = "wait-for-function")]
    wait_for_function: wait_for_function::Installation,
    #[cfg(feature = "wait-for-timeout")]
    wait_for_timeout: wait_for_timeout::Installation,
    #[cfg(feature = "frame-lifecycle")]
    frame_lifecycle: Arc<pwright_plugin_chromium_frame_lifecycle::FrameLifecycleRegistry>,
    #[cfg(feature = "url-wait")]
    url_wait: url_wait::Installation,
    #[cfg(feature = "set-content")]
    set_content: set_content::Installation,
    #[cfg(feature = "timeout-settings")]
    timeout_settings: Arc<timeout_settings::Installation>,
    #[cfg(feature = "device-emulation")]
    device_emulation: Arc<device_emulation::Installation>,
    #[cfg(feature = "context-geolocation")]
    context_geolocation: Arc<context_geolocation::Installation>,
    #[cfg(feature = "context-geolocation-permission")]
    context_geolocation_permission: Arc<context_geolocation_permission::Installation>,
    #[cfg(feature = "context-http-credentials")]
    context_http_credentials: Arc<context_http_credentials::Installation>,
    #[cfg(feature = "context-locale")]
    context_locale: Arc<context_locale::Installation>,
    #[cfg(feature = "context-timezone")]
    context_timezone: Arc<context_timezone::Installation>,
    #[cfg(feature = "context-offline")]
    context_offline: Arc<context_offline::Installation>,
    #[cfg(feature = "context-proxy")]
    context_proxy: Arc<context_proxy::Installation>,
    #[cfg(feature = "ignore-https-errors")]
    ignore_https_errors: Arc<ignore_https_errors::Installation>,
    #[cfg(feature = "bypass-csp")]
    bypass_csp: Arc<bypass_csp::Installation>,
    #[cfg(feature = "javascript-disabled")]
    javascript_disabled: Arc<javascript_disabled::Installation>,
    #[cfg(feature = "page-pdf")]
    page_pdf: Arc<page_pdf::Installation>,
    #[cfg(feature = "page-viewport")]
    page_viewport: Arc<page_viewport::Installation>,
    composition: AttachedChromiumComposition,
    runtime: Arc<CommandRuntime>,
    locator_evaluation: Arc<locator_evaluation::Installation>,
    #[cfg(feature = "file-input")]
    file_input: Arc<file_input::Installation>,
    screenshot_queue: Arc<screenshot::queue::Queue>,
    keyboard: Arc<keyboard::Installation>,
    mouse: Arc<mouse::Installation>,
    #[cfg(feature = "touchscreen-input")]
    touchscreen: Arc<touchscreen::Installation>,
    input_start: input_start::Order,
}

#[cfg(feature = "owned-launch")]
struct OwnedBrowserProcess {
    shutdown: owned_browser_shutdown::OneFlightShutdown<Result<ChromiumShutdownReport, String>>,
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
    #[cfg(feature = "context-api-request")]
    browser_user_agent: String,
    resources: ProductionResourceLedger,
    operations: TaskTracker,
    next_call: AtomicU64,
    cleanup: TokioCleanupExecutor,
    frame_metadata: Arc<FrameMetadata>,
    frame_trees: Arc<frame_trees::FrameTrees>,
    context_metadata: context_metadata::ContextMetadata,
    #[cfg(feature = "context-storage-state")]
    context_storage_state: Arc<context_storage_state::Installation>,
    topology_barriers: tokio::sync::mpsc::Sender<DriverRequest>,
    browser_pages: browser_page::BrowserPages,
    #[cfg(feature = "owned-launch")]
    owned_browser: Option<OwnedBrowserProcess>,
    #[cfg(feature = "playwright-trace")]
    trace_directory: Option<pwright_trace_playwright_v8::PreparedTraceDirectory>,
}

struct SessionTeardown {
    state: Arc<Mutex<Option<LiveSession>>>,
    disconnected: CancellationToken,
    frame_metadata: Arc<FrameMetadata>,
    frame_trees: Arc<frame_trees::FrameTrees>,
    context_metadata: context_metadata::ContextMetadata,
    #[cfg(feature = "context-storage-state")]
    context_storage_state: Arc<context_storage_state::Installation>,
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
        #[cfg(feature = "network-events")]
        if let Some(live) = retired.as_ref() {
            live.network_events.close();
        }
        #[cfg(feature = "context-http-credentials")]
        if let Some(live) = retired.as_ref() {
            live.context_http_credentials.close();
        }
        #[cfg(feature = "download-events")]
        if let Some(live) = retired.as_ref() {
            live.download_events.close();
        }
        #[cfg(feature = "worker-console-events")]
        if let Some(live) = retired.as_ref() {
            live.worker_console_events.close();
        }
        #[cfg(feature = "worker-handle-evaluation")]
        if let Some(live) = retired.as_ref() {
            live.worker_handles.close();
        }
        #[cfg(feature = "worker-events")]
        if let Some(live) = retired.as_ref() {
            live.worker_events.close();
        }
        drop(retired);
        self.frame_trees.clear();
        self.context_metadata.clear();
        #[cfg(feature = "context-storage-state")]
        self.context_storage_state.close();
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

#[derive(Clone, Copy)]
enum SessionContextScope {
    All,
    #[cfg(feature = "owned-launch")]
    OwnedOnly,
}

impl HostSession {
    pub async fn connect_over_cdp(endpoint: &str, timeout_ms: u32) -> Result<Self, HostError> {
        Self::connect_with_publication(endpoint, timeout_ms, LifecyclePublication::Collections)
            .await
    }

    #[cfg(feature = "cdp-handshake-headers")]
    pub async fn connect_over_cdp_with_headers(
        endpoint: &str,
        timeout_ms: u32,
        headers: Vec<(String, String)>,
    ) -> Result<Self, HostError> {
        Self::connect_with_publication_and_headers(
            endpoint,
            timeout_ms,
            LifecyclePublication::Collections,
            headers,
        )
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

    #[cfg(feature = "cdp-handshake-headers")]
    pub async fn connect_over_cdp_with_lifecycle_events_and_headers(
        endpoint: &str,
        timeout_ms: u32,
        headers: Vec<(String, String)>,
    ) -> Result<Self, HostError> {
        Self::connect_with_publication_and_headers(
            endpoint,
            timeout_ms,
            LifecyclePublication::Events,
            headers,
        )
        .await
    }

    async fn connect_with_publication(
        endpoint: &str,
        timeout_ms: u32,
        publication: LifecyclePublication,
    ) -> Result<Self, HostError> {
        endpoint::validate_endpoint(endpoint)?;
        let mut phase = ConnectPhase::EndpointDiscovery;
        let connect = Self::connect_inner(
            endpoint,
            timeout_ms,
            &mut phase,
            publication,
            #[cfg(feature = "cdp-handshake-headers")]
            None,
        );
        if timeout_ms == 0 {
            connect.await
        } else {
            tokio::time::timeout(Duration::from_millis(u64::from(timeout_ms)), connect)
                .await
                .map_err(|_| HostError::ConnectTimeout { timeout_ms, phase })?
        }
    }

    #[cfg(feature = "cdp-handshake-headers")]
    async fn connect_with_publication_and_headers(
        endpoint: &str,
        timeout_ms: u32,
        publication: LifecyclePublication,
        headers: Vec<(String, String)>,
    ) -> Result<Self, HostError> {
        let headers = CdpHandshakeHeaders::new(headers).map_err(|error| {
            if error.is_limit() {
                HostError::ResourceExhausted(error.to_string())
            } else {
                HostError::InvalidArgument(error.to_string())
            }
        })?;
        endpoint::validate_endpoint(endpoint)?;
        let mut phase = ConnectPhase::EndpointDiscovery;
        let connect = Self::connect_inner(
            endpoint,
            timeout_ms,
            &mut phase,
            publication,
            Some(&headers),
        );
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
        #[cfg(feature = "cdp-handshake-headers")] headers: Option<&CdpHandshakeHeaders>,
    ) -> Result<Self, HostError> {
        #[cfg(feature = "cdp-handshake-headers")]
        let endpoint = if let Some(headers) = headers {
            endpoint::resolve_with_headers(endpoint, headers).await?
        } else {
            endpoint::resolve(endpoint).await?
        };
        #[cfg(not(feature = "cdp-handshake-headers"))]
        let endpoint = endpoint::resolve(endpoint).await?;
        // A zero public timeout also disables per-command response timers. CDP
        // admission and transport shutdown remain independently bounded.
        let command_timeout = Duration::from_millis(u64::from(timeout_ms));
        *phase = ConnectPhase::CdpAttachment;
        let factory = if matches!(publication, LifecyclePublication::Events) {
            profile::attach_factory_with_native_close(command_timeout, true)?
        } else {
            profile::attach_factory(command_timeout)?
        };
        #[cfg(feature = "cdp-handshake-headers")]
        let attach = if let Some(headers) = headers {
            factory.attach_with_headers(endpoint, headers).await
        } else {
            factory.attach(endpoint).await
        };
        #[cfg(not(feature = "cdp-handshake-headers"))]
        let attach = factory.attach(endpoint).await;
        let composition = attach
            .map_err(|error| HostError::backend(format_args!("CDP attach failed: {error}")))?;
        Self::from_composition(
            composition,
            phase,
            publication,
            SessionContextScope::All,
            None,
            PersistentContextDefaults::default(),
            #[cfg(feature = "owned-launch")]
            None,
            #[cfg(feature = "playwright-trace")]
            None,
        )
        .await
    }

    /// Launches one caller-selected Chromium executable with an existing,
    /// empty, absolute profile directory. The directory remains caller-owned.
    /// Chromium's native default Context is kept internal; the returned Browser
    /// initially publishes no BrowserContexts. Explicit new_context calls
    /// create the Browser's public Contexts.
    #[cfg(feature = "owned-launch")]
    pub async fn launch_chromium_with_lifecycle_events(
        executable: ChromiumExecutable,
        user_data_dir: PathBuf,
        headless: bool,
        timeout_ms: u32,
    ) -> Result<Self, HostError> {
        Self::launch_chromium_with_lifecycle_events_inner(
            executable,
            user_data_dir,
            headless,
            timeout_ms,
            #[cfg(feature = "playwright-trace")]
            None,
        )
        .await
    }

    #[cfg(feature = "owned-launch")]
    async fn launch_chromium_with_lifecycle_events_inner(
        executable: ChromiumExecutable,
        user_data_dir: PathBuf,
        headless: bool,
        timeout_ms: u32,
        #[cfg(feature = "playwright-trace")] trace_directory: Option<
            pwright_trace_playwright_v8::PreparedTraceDirectory,
        >,
    ) -> Result<Self, HostError> {
        let mut phase = ConnectPhase::BrowserLaunch;
        let launch = Self::launch_fresh_chromium_inner(
            executable,
            user_data_dir,
            headless,
            timeout_ms,
            &mut phase,
            #[cfg(feature = "playwright-trace")]
            trace_directory,
        );
        if timeout_ms == 0 {
            launch.await
        } else {
            tokio::time::timeout(Duration::from_millis(u64::from(timeout_ms)), launch)
                .await
                .map_err(|_| HostError::ConnectTimeout { timeout_ms, phase })?
        }
    }

    /// The caller has already prepared a non-deleting trace directory before
    /// browser profile allocation or process startup. Retain its verified path
    /// with this owned session for a future Context recorder composition.
    #[cfg(feature = "playwright-trace")]
    pub async fn launch_chromium_with_trace_directory(
        executable: ChromiumExecutable,
        user_data_dir: PathBuf,
        headless: bool,
        timeout_ms: u32,
        trace_directory: pwright_trace_playwright_v8::PreparedTraceDirectory,
    ) -> Result<Self, HostError> {
        Self::launch_chromium_with_lifecycle_events_inner(
            executable,
            user_data_dir,
            headless,
            timeout_ms,
            Some(trace_directory),
        )
        .await
    }

    #[cfg(feature = "owned-launch")]
    async fn launch_fresh_chromium_inner(
        executable: ChromiumExecutable,
        user_data_dir: PathBuf,
        headless: bool,
        timeout_ms: u32,
        phase: &mut ConnectPhase,
        #[cfg(feature = "playwright-trace")] trace_directory: Option<
            pwright_trace_playwright_v8::PreparedTraceDirectory,
        >,
    ) -> Result<Self, HostError> {
        let command_timeout = Duration::from_millis(u64::from(timeout_ms));
        let attachment = profile::attach_factory_with_native_close(command_timeout, true)?;
        let connection_config = attachment.profile().connection().config().clone();
        let request = ChromiumLaunchRequest::new(executable, user_data_dir, connection_config)
            .with_headless(headless);
        Self::launch_chromium_request_inner(
            attachment,
            request,
            timeout_ms,
            None,
            PersistentContextDefaults::default(),
            SessionContextScope::OwnedOnly,
            phase,
            #[cfg(feature = "playwright-trace")]
            trace_directory,
        )
        .await
    }

    /// Launches one explicitly selected Chromium executable against one
    /// explicitly initialized caller-owned persistent profile. Unlike CDP
    /// attachment, the returned session owns the unique process lease.
    #[cfg(feature = "owned-launch")]
    pub async fn launch_persistent_chromium_with_lifecycle_events(
        executable: ChromiumExecutable,
        profile: ChromiumPersistentProfile,
        headless: bool,
        base_url: Option<&str>,
        timeout_ms: u32,
    ) -> Result<Self, HostError> {
        let mut phase = ConnectPhase::BrowserLaunch;
        let launch = Self::launch_persistent_chromium_inner(
            executable,
            profile,
            PersistentChromiumLaunchOptions {
                headless,
                base_url,
                context_defaults: PersistentContextDefaults::default(),
                #[cfg(feature = "persistent-context-launch-arguments")]
                launch_arguments: PersistentLaunchArguments::default(),
                timeout_ms,
            },
            &mut phase,
        );
        if timeout_ms == 0 {
            launch.await
        } else {
            tokio::time::timeout(Duration::from_millis(u64::from(timeout_ms)), launch)
                .await
                .map_err(|_| HostError::ConnectTimeout { timeout_ms, phase })?
        }
    }

    /// Launches an owned persistent Chromium whose default Context locale is
    /// admitted before the initial Page topology is initialized.
    #[cfg(feature = "persistent-context-locale")]
    pub async fn launch_persistent_chromium_with_lifecycle_events_and_locale(
        executable: ChromiumExecutable,
        profile: ChromiumPersistentProfile,
        headless: bool,
        base_url: Option<&str>,
        locale: Option<ContextLocale>,
        timeout_ms: u32,
    ) -> Result<Self, HostError> {
        let mut phase = ConnectPhase::BrowserLaunch;
        let launch = Self::launch_persistent_chromium_inner(
            executable,
            profile,
            PersistentChromiumLaunchOptions {
                headless,
                base_url,
                context_defaults: PersistentContextDefaults::default().with_locale(locale),
                #[cfg(feature = "persistent-context-launch-arguments")]
                launch_arguments: PersistentLaunchArguments::default(),
                timeout_ms,
            },
            &mut phase,
        );
        if timeout_ms == 0 {
            launch.await
        } else {
            tokio::time::timeout(Duration::from_millis(u64::from(timeout_ms)), launch)
                .await
                .map_err(|_| HostError::ConnectTimeout { timeout_ms, phase })?
        }
    }

    /// Launches an owned persistent Chromium whose optional default Context
    /// policies are admitted before the initial Page topology is initialized.
    #[cfg(any(
        feature = "persistent-context-locale",
        feature = "persistent-context-timezone",
        feature = "persistent-context-offline",
        feature = "persistent-context-geolocation",
        feature = "persistent-context-geolocation-permission",
        feature = "persistent-context-http-credentials",
        feature = "persistent-context-launch-arguments",
        feature = "persistent-context-ignore-https-errors",
        feature = "persistent-context-device-emulation",
        feature = "persistent-context-emulate-media",
        feature = "persistent-context-extra-headers",
        feature = "persistent-context-bypass-csp",
        feature = "persistent-context-javascript-disabled"
    ))]
    pub async fn launch_persistent_chromium_with_lifecycle_events_and_context_defaults(
        executable: ChromiumExecutable,
        profile: ChromiumPersistentProfile,
        headless: bool,
        base_url: Option<&str>,
        context_defaults: PersistentContextDefaults,
        #[cfg(feature = "persistent-context-launch-arguments")]
        launch_arguments: PersistentLaunchArguments,
        timeout_ms: u32,
    ) -> Result<Self, HostError> {
        let mut phase = ConnectPhase::BrowserLaunch;
        let launch = Self::launch_persistent_chromium_inner(
            executable,
            profile,
            PersistentChromiumLaunchOptions {
                headless,
                base_url,
                context_defaults,
                #[cfg(feature = "persistent-context-launch-arguments")]
                launch_arguments,
                timeout_ms,
            },
            &mut phase,
        );
        if timeout_ms == 0 {
            launch.await
        } else {
            tokio::time::timeout(Duration::from_millis(u64::from(timeout_ms)), launch)
                .await
                .map_err(|_| HostError::ConnectTimeout { timeout_ms, phase })?
        }
    }

    #[cfg(feature = "owned-launch")]
    async fn launch_persistent_chromium_inner(
        executable: ChromiumExecutable,
        profile: ChromiumPersistentProfile,
        options: PersistentChromiumLaunchOptions<'_>,
        phase: &mut ConnectPhase,
    ) -> Result<Self, HostError> {
        let PersistentChromiumLaunchOptions {
            headless,
            base_url,
            context_defaults,
            #[cfg(feature = "persistent-context-launch-arguments")]
            launch_arguments,
            timeout_ms,
        } = options;
        let command_timeout = Duration::from_millis(u64::from(timeout_ms));
        let attachment = profile::attach_factory_with_native_close(command_timeout, true)?;
        let connection_config = attachment.profile().connection().config().clone();
        let request = ChromiumLaunchRequest::new_persistent(executable, profile, connection_config)
            .with_headless(headless);
        #[cfg(feature = "persistent-context-launch-arguments")]
        let request = launch_arguments
            .into_values()
            .fold(request, |request, argument| {
                request.with_argument(argument.into_string())
            });
        Self::launch_chromium_request_inner(
            attachment,
            request,
            timeout_ms,
            base_url,
            context_defaults,
            SessionContextScope::All,
            phase,
            #[cfg(feature = "playwright-trace")]
            None,
        )
        .await
    }

    #[cfg(feature = "owned-launch")]
    async fn launch_chromium_request_inner(
        attachment: ChromiumAttachFactory,
        mut request: ChromiumLaunchRequest,
        timeout_ms: u32,
        base_url: Option<&str>,
        context_defaults: PersistentContextDefaults,
        context_scope: SessionContextScope,
        phase: &mut ConnectPhase,
        #[cfg(feature = "playwright-trace")] trace_directory: Option<
            pwright_trace_playwright_v8::PreparedTraceDirectory,
        >,
    ) -> Result<Self, HostError> {
        if timeout_ms != 0 {
            request = request.with_startup_timeout(Duration::from_millis(u64::from(timeout_ms)));
        }
        *phase = ConnectPhase::BrowserLaunch;
        let owned = ChromiumLaunchCompositionFactory::new(attachment)
            .launch_with_startup_logs(request)
            .await
            .map_err(|error| HostError::backend(format_args!("Chromium launch failed: {error}")))?;
        let (composition, lease) = owned.into_parts();
        let mut lease = lease.ok_or_else(|| {
            HostError::Backend("owned Chromium launch did not retain process authority".into())
        })?;
        // From this point, cancellation or any later construction error must
        // not orphan the process whose unique lease is moving into the host.
        lease.arm_kill_on_drop();
        let browser = LaunchedBrowser::new(Arc::clone(composition.connection()), lease);
        let owned_browser = OwnedBrowserProcess {
            shutdown: owned_browser_shutdown::OneFlightShutdown::new(async move {
                let policy = ChromiumShutdownPolicy::new(
                    Duration::from_secs(5),
                    Duration::from_secs(5),
                    Duration::from_secs(5),
                )
                .expect("fixed owned-browser shutdown phases are nonzero");
                ChromiumBrowserShutdown::new()
                    .shutdown(browser, policy)
                    .await
                    .map_err(|error| error.to_string())
            }),
        };
        Self::from_composition(
            composition,
            phase,
            LifecyclePublication::Events,
            context_scope,
            base_url,
            context_defaults,
            Some(owned_browser),
            #[cfg(feature = "playwright-trace")]
            trace_directory,
        )
        .await
    }

    async fn from_composition(
        mut composition: AttachedChromiumComposition,
        phase: &mut ConnectPhase,
        publication: LifecyclePublication,
        context_scope: SessionContextScope,
        initial_default_base_url: Option<&str>,
        context_defaults: PersistentContextDefaults,
        #[cfg(feature = "owned-launch")] owned_browser: Option<OwnedBrowserProcess>,
        #[cfg(feature = "playwright-trace")] trace_directory: Option<
            pwright_trace_playwright_v8::PreparedTraceDirectory,
        >,
    ) -> Result<Self, HostError> {
        #[cfg(not(any(
            feature = "context-geolocation",
            feature = "context-geolocation-permission",
            feature = "context-locale",
            feature = "context-timezone",
            feature = "context-offline",
            feature = "device-emulation",
            feature = "ignore-https-errors",
            feature = "bypass-csp",
            feature = "javascript-disabled",
            feature = "emulate-media",
            feature = "extra-headers"
        )))]
        let _ = context_defaults;
        *phase = ConnectPhase::BrowserVersion;
        let value = CdpSession::browser(Arc::clone(composition.connection()))
            .send("Browser.getVersion", serde_json::json!({}))
            .await
            .map_err(|error| {
                #[cfg(feature = "cdp-ws-close-reason")]
                let mapped = browser_version_query_error(
                    &error,
                    composition.connection().peer_close_reason().as_deref(),
                );
                #[cfg(not(feature = "cdp-ws-close-reason"))]
                let mapped =
                    HostError::backend(format_args!("Browser version query failed: {error}"));
                mapped
            })?;
        let version: pwright_cdp::generated::browser::GetVersionReturns =
            serde_json::from_value(value).map_err(HostError::backend)?;
        if version.product.len() > 4096 || version.user_agent.len() > 4096 {
            return Err(HostError::ResourceExhausted(
                "browser version metadata exceeds 4096 bytes".into(),
            ));
        }
        let headful = !version.user_agent.contains("Headless");
        #[cfg(feature = "context-api-request")]
        let browser_user_agent = version.user_agent.clone();
        let version = version
            .product
            .split_once('/')
            .map_or(version.product.as_str(), |(_, version)| version)
            .to_owned();
        // Existing runtime/lifecycle capacity plus two pre-reserved native
        // continuation slots for each of eight Locator evaluation commands.
        #[cfg(not(feature = "handles"))]
        let cleanup_slots = 32usize;
        #[cfg(feature = "handles")]
        let cleanup_slots = handles::CLEANUP_SLOTS;
        #[cfg(any(feature = "console-events", feature = "console-history"))]
        let cleanup_slots = cleanup_slots
            .checked_add(console_events::MAX_LOG_RELEASE_JOBS)
            .ok_or_else(|| HostError::ResourceExhausted("cleanup capacity overflow".into()))?;
        #[cfg(feature = "worker-console-events")]
        let cleanup_slots = cleanup_slots
            .checked_add(worker_console_events::MAX_WORKER_CONSOLE_RELEASE_JOBS)
            .ok_or_else(|| HostError::ResourceExhausted("cleanup capacity overflow".into()))?;
        #[cfg(feature = "worker-handle-evaluation")]
        let cleanup_slots = cleanup_slots
            .checked_add(worker_handles::CLEANUP_SLOTS)
            .ok_or_else(|| HostError::ResourceExhausted("cleanup capacity overflow".into()))?;
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
        #[cfg(feature = "network-events")]
        let mut network_driver = composition
            .network_event_driver(runtime.service())
            .map_err(HostError::backend)?
            .ok_or_else(|| HostError::Backend("network event driver was not selected".into()))?;
        #[cfg(feature = "network-events")]
        let network_registration = network_driver.registration_handle();
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
        #[cfg(feature = "file-chooser-events")]
        let page_file_chooser =
            page_file_chooser::Installation::new(composition.resources(), &file_input)?;
        let frame_metadata = Arc::new(
            FrameMetadata::new(FrameMetadataLimits {
                entries: 128,
                url_bytes: 16 * 1024,
                total_bytes: 2 * 1024 * 1024,
                publications: (if cfg!(any(feature = "console-events", feature = "page-errors")) {
                    512
                } else {
                    256
                }) + if cfg!(feature = "frame-detach-events") {
                    254
                } else {
                    0
                } + frame_trees::FRAME_EVENT_RESERVATIONS,
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
        let context_metadata = match context_scope {
            SessionContextScope::All => context_metadata,
            #[cfg(feature = "owned-launch")]
            SessionContextScope::OwnedOnly => context_metadata.with_owned_contexts_only()?,
        };
        let context_metadata = match initial_default_base_url {
            Some(base_url) => context_metadata.with_external_base_url(base_url)?,
            None => context_metadata,
        };
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
        #[cfg(feature = "context-storage-state")]
        let context_storage_state =
            context_storage_state::Installation::new(composition.resources())?;
        #[cfg(feature = "network-events")]
        let network_events = Arc::new(network_events::Installation::new(
            runtime.as_ref(),
            composition.resources(),
        )?);
        #[cfg(feature = "download-events")]
        let download_events = download_events::Installation::new(composition.resources())?;
        #[cfg(feature = "worker-events")]
        let worker_events = worker_events::Installation::new(
            composition.resources(),
            Arc::new(pwright_cdp::CdpSessionFactory::new(
                composition.connection().clone(),
            )),
            context_metadata.clone(),
            frame_trees.clone(),
        )?;
        #[cfg(feature = "worker-handle-evaluation")]
        let worker_handles =
            worker_handles::Installation::new(composition.resources(), cleanup.clone())?;
        #[cfg(feature = "worker-console-events")]
        let worker_console_events = {
            #[cfg(feature = "worker-console-argument-handles")]
            {
                worker_console_events::Installation::new_with_argument_handles(
                    composition.resources(),
                    cleanup.clone(),
                    worker_handles.clone(),
                )?
            }
            #[cfg(not(feature = "worker-console-argument-handles"))]
            {
                worker_console_events::Installation::new(composition.resources(), cleanup.clone())?
            }
        };
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
        #[cfg(feature = "wait-for-function")]
        let wait_for_function = wait_for_function::install(composition.resources())?;
        #[cfg(feature = "wait-for-timeout")]
        let wait_for_timeout = wait_for_timeout::install(composition.resources())?;
        #[cfg(feature = "url-wait")]
        let url_wait = url_wait::install(composition.resources())?;
        #[cfg(feature = "set-content")]
        let set_content = set_content::install(composition.resources())?;
        #[cfg(feature = "emulate-media")]
        let emulate_media = Arc::new(emulate_media::Installation::new_with_default(
            composition.resources(),
            context_defaults.media,
        )?);
        #[cfg(feature = "timeout-settings")]
        let timeout_settings = Arc::new(timeout_settings::Installation::new(
            composition.resources(),
        )?);
        #[cfg(feature = "page-bindings")]
        let page_bindings = page_bindings::Installation::new(
            composition.connection().clone(),
            composition.resources(),
        )?;
        #[cfg(feature = "context-http-credentials")]
        let context_http_credentials = context_http_credentials::Installation::new_with_default(
            composition.resources(),
            composition.connection().clone(),
            context_defaults.http_credentials,
        )?;
        #[cfg(feature = "page-routes")]
        let page_routes = page_routes::Installation::new(
            composition.connection().clone(),
            composition.resources(),
            #[cfg(feature = "context-http-credentials")]
            context_http_credentials.clone(),
        )?;
        #[cfg(feature = "page-dialogs")]
        let page_dialogs = page_dialogs::Installation::new(composition.resources())?;
        #[cfg(feature = "page-errors")]
        let page_errors = page_errors::Installation::new(composition.resources())?;
        #[cfg(feature = "console-history")]
        let console_history = console_events::Installation::new(composition.resources())?;
        #[cfg(feature = "console-events")]
        let console_subscriptions = Arc::new(console_events::Subscriptions::new());
        #[cfg(feature = "init-scripts")]
        let init_scripts = init_scripts::Installation::new(
            composition.connection().clone(),
            composition.resources(),
        )?;
        #[cfg(feature = "device-emulation")]
        let device_emulation = device_emulation::Installation::new_with_default(
            composition.resources(),
            context_defaults.device,
        )?;
        #[cfg(feature = "context-geolocation")]
        let context_geolocation = context_geolocation::Installation::new_with_default(
            composition.resources(),
            context_defaults.geolocation,
        )?;
        #[cfg(feature = "context-geolocation-permission")]
        let context_geolocation_permission =
            context_geolocation_permission::Installation::new_with_default(
                composition.resources(),
                composition.connection().clone(),
                context_defaults.geolocation_permission,
            )?;
        #[cfg(feature = "context-geolocation-permission")]
        context_geolocation_permission.apply_default().await?;
        #[cfg(feature = "context-locale")]
        let context_locale = context_locale::Installation::new_with_default(
            composition.resources(),
            context_defaults.locale,
        )?;
        #[cfg(feature = "context-timezone")]
        let context_timezone = context_timezone::Installation::new_with_default(
            composition.resources(),
            context_defaults.timezone,
        )?;
        #[cfg(feature = "context-offline")]
        let context_offline = context_offline::Installation::new_with_default(
            composition.resources(),
            context_defaults.offline,
        )?;
        #[cfg(feature = "context-proxy")]
        let context_proxy = context_proxy::Installation::new(composition.resources())?;
        #[cfg(feature = "ignore-https-errors")]
        let ignore_https_errors = ignore_https_errors::Installation::new_with_default(
            composition.resources(),
            context_defaults.ignore_https_errors,
        )?;
        #[cfg(feature = "bypass-csp")]
        let bypass_csp = bypass_csp::Installation::new_with_default(
            composition.resources(),
            context_defaults.bypass_csp,
        )?;
        #[cfg(feature = "javascript-disabled")]
        let javascript_disabled = javascript_disabled::Installation::new_with_default(
            composition.resources(),
            context_defaults.javascript_disabled,
        )?;
        #[cfg(feature = "page-pdf")]
        let page_pdf = page_pdf::Installation::new(composition.resources())?;
        #[cfg(feature = "page-viewport")]
        let page_viewport = page_viewport::Installation::new(
            composition.connection().clone(),
            device_emulation.clone(),
            composition.resources(),
            headful,
            std::env::consts::OS,
        )?;
        let defaults = context_defaults::OwnedContextDefaults::new(
            composition.connection().clone(),
            headful,
            std::env::consts::OS,
        );
        #[cfg(any(feature = "console-events", feature = "console-history"))]
        let defaults = console_events::Initializer::new(defaults, composition.connection().clone());
        #[cfg(feature = "page-bindings")]
        let defaults = page_bindings::Initializer::new(defaults, page_bindings.clone());
        #[cfg(feature = "page-routes")]
        let defaults = page_routes::Initializer::new(defaults, page_routes.clone());
        #[cfg(all(feature = "context-http-credentials", not(feature = "page-routes")))]
        let defaults =
            context_http_credentials::Initializer::new(defaults, context_http_credentials.clone());
        #[cfg(feature = "init-scripts")]
        let defaults = init_scripts::Initializer::new(defaults, init_scripts.clone());
        #[cfg(feature = "emulate-media")]
        let defaults = emulate_media::Initializer::new(
            defaults,
            emulate_media.clone(),
            composition.connection().clone(),
        );
        #[cfg(feature = "device-emulation")]
        let defaults = device_emulation::Initializer::new(
            defaults,
            device_emulation.clone(),
            composition.connection().clone(),
            headful,
            std::env::consts::OS,
        );
        #[cfg(feature = "context-geolocation")]
        let defaults = context_geolocation::Initializer::new(
            defaults,
            context_geolocation.clone(),
            composition.connection().clone(),
        );
        #[cfg(feature = "ignore-https-errors")]
        let defaults = ignore_https_errors::Initializer::new(
            defaults,
            ignore_https_errors.clone(),
            composition.connection().clone(),
        );
        #[cfg(feature = "bypass-csp")]
        let defaults = bypass_csp::Initializer::new(
            defaults,
            bypass_csp.clone(),
            composition.connection().clone(),
        );
        #[cfg(feature = "javascript-disabled")]
        let defaults = javascript_disabled::Initializer::new(
            defaults,
            javascript_disabled.clone(),
            composition.connection().clone(),
        );
        #[cfg(feature = "context-timezone")]
        let defaults = context_timezone::Initializer::new(
            defaults,
            context_timezone.clone(),
            composition.connection().clone(),
        );
        #[cfg(feature = "context-offline")]
        let defaults = context_offline::Initializer::new(
            defaults,
            context_offline.clone(),
            composition.connection().clone(),
        );
        #[cfg(feature = "page-viewport")]
        let defaults = page_viewport::Initializer::new(defaults, page_viewport.clone());
        #[cfg(feature = "file-chooser-events")]
        let defaults =
            page_file_chooser::Initializer::new(defaults, composition.connection().clone());
        #[cfg(feature = "extra-headers")]
        let extra_headers = extra_headers::Installation::new_with_default(
            composition.connection().clone(),
            composition.resources(),
            context_defaults.extra_headers,
        )?;
        #[cfg(feature = "extra-headers")]
        let defaults = extra_headers::Initializer::new(defaults, extra_headers.clone());
        #[cfg(feature = "context-locale")]
        let defaults = context_locale::Initializer::new(
            defaults,
            context_locale.clone(),
            composition.connection().clone(),
        );
        #[cfg(any(feature = "context-locale", feature = "device-emulation"))]
        let mut user_agent_defaults =
            user_agent_override::Initializer::new(defaults, composition.connection().clone());
        #[cfg(feature = "device-emulation")]
        user_agent_defaults.set_device_installation(device_emulation.clone());
        #[cfg(feature = "context-locale")]
        user_agent_defaults.set_locale_installation(context_locale.clone());
        #[cfg(any(feature = "context-locale", feature = "device-emulation"))]
        let defaults = user_agent_defaults;
        #[cfg(feature = "cdp-session")]
        let auxiliary_parent = Arc::new(
            pwright_composition_chromium::ChromiumAuxiliarySessionParent::new(256)
                .map_err(HostError::backend)?,
        );
        #[cfg(feature = "cdp-session")]
        let cdp_sessions = Arc::new(cdp_sessions::Installation::new(
            composition.connection().clone(),
            auxiliary_parent.clone(),
            composition.resources(),
        )?);
        let mut topology_driver = composition
            .topology_event_driver()
            .map_err(HostError::backend)?
            .ok_or_else(|| HostError::Backend("topology driver was not selected".into()))?
            .with_playwright_context_fallback()
            .with_session_initializer(Arc::new(defaults))
            .with_frame_url_observer({
                #[cfg(feature = "context-storage-state")]
                {
                    Arc::new(context_storage_state::Observer::new(
                        frame_trees.clone(),
                        context_storage_state.clone(),
                    ))
                }
                #[cfg(not(feature = "context-storage-state"))]
                {
                    frame_trees.clone()
                }
            });
        #[cfg(feature = "cdp-session")]
        {
            topology_driver = topology_driver
                .with_auxiliary_session_parent(auxiliary_parent)
                .map_err(HostError::backend)?;
        }
        #[cfg(feature = "network-events")]
        {
            topology_driver =
                topology_driver.with_network_registration(network_registration.clone());
        }
        #[cfg(feature = "frame-lifecycle")]
        let frame_observer = frame_lifecycle.clone()
            as Arc<dyn pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver>;
        #[cfg(feature = "page-events")]
        let frame_observer = Arc::new(page_events::Observer::new(
            frame_trees.clone(),
            frame_observer,
        ))
            as Arc<dyn pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver>;
        #[cfg(feature = "page-errors")]
        let frame_observer = Arc::new(page_errors::Observer::new(
            frame_trees.clone(),
            page_errors.clone(),
            #[cfg(feature = "worker-error-events")]
            worker_events.clone(),
            frame_observer,
        ))
            as Arc<dyn pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver>;
        #[cfg(feature = "page-dialogs")]
        let task_page_dialogs = page_dialogs.clone();
        #[cfg(feature = "page-dialogs")]
        let frame_observer = Arc::new(page_dialogs::Observer::new(
            frame_trees.clone(),
            page_dialogs,
            frame_observer,
            composition.connection().clone(),
        ))
            as Arc<dyn pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver>;
        #[cfg(feature = "file-chooser-events")]
        let frame_observer = Arc::new(page_file_chooser::Observer::new(
            frame_trees.clone(),
            page_file_chooser,
            frame_observer,
            composition.connection().clone(),
        ))
            as Arc<dyn pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver>;
        #[cfg(feature = "download-events")]
        let frame_observer = Arc::new(download_events::Observer::new(
            download_events.clone(),
            frame_observer,
        ))
            as Arc<dyn pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver>;
        #[cfg(feature = "page-routes")]
        let frame_observer = Arc::new(page_routes::Observer::new(
            page_routes.clone(),
            frame_observer,
        ))
            as Arc<dyn pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver>;
        #[cfg(feature = "context-http-credentials")]
        let frame_observer = Arc::new(context_http_credentials::Observer::new(
            context_http_credentials.clone(),
            frame_observer,
        ))
            as Arc<dyn pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver>;
        #[cfg(any(feature = "console-events", feature = "console-history"))]
        let console_observer = Arc::new(console_events::Observer::new(
            #[cfg(feature = "console-events")]
            frame_trees.clone(),
            #[cfg(feature = "console-events")]
            console_subscriptions.clone(),
            #[cfg(feature = "console-history")]
            console_history.clone(),
            frame_observer,
            composition.connection().clone(),
            cleanup.clone(),
            #[cfg(feature = "console-argument-handles")]
            handles.clone(),
        ));
        #[cfg(feature = "console-argument-handles")]
        let task_console_observer = console_observer.clone();
        #[cfg(any(feature = "console-events", feature = "console-history"))]
        let frame_observer = console_observer
            as Arc<dyn pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver>;
        #[cfg(feature = "page-bindings")]
        let frame_observer = Arc::new(page_bindings::Observer::new(
            page_bindings.clone(),
            frame_observer,
        ))
            as Arc<dyn pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver>;
        // Worker retention must be the outermost selected frame observer: the
        // topology driver queries this opt-in before delivering an attachment
        // through the downstream event chain.
        #[cfg(feature = "worker-events")]
        let frame_observer = Arc::new(worker_events::Observer::new(
            worker_events.clone(),
            frame_observer,
        ))
            as Arc<dyn pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver>;
        #[cfg(feature = "worker-console-events")]
        let frame_observer = Arc::new(worker_console_events::Observer::new(
            worker_console_events.clone(),
            worker_events.clone(),
            frame_observer,
        ))
            as Arc<dyn pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver>;
        #[cfg(all(feature = "frame-lifecycle", not(feature = "extra-headers")))]
        {
            topology_driver = topology_driver
                .with_frame_event_observer(frame_observer)
                .map_err(HostError::backend)?;
        }
        #[cfg(feature = "extra-headers")]
        {
            #[cfg(feature = "frame-lifecycle")]
            let frames = Some(frame_observer);
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
        topology_recovery::initialize_snapshot(&mut topology_driver, runtime.as_ref())
            .await
            .map_err(|error| {
                HostError::backend(format_args!("Topology initialization failed: {error}"))
            })?;
        #[cfg(all(feature = "network-events", feature = "request-frame"))]
        {
            let topology = composition.topology().map_err(HostError::backend)?;
            network_driver
                .observe_existing_frames(topology.topology())
                .map_err(HostError::backend)?;
        }
        #[cfg(feature = "page-routes")]
        let mut fetch_event_driver = page_routes::EventDriver::new(
            page_routes.clone(),
            #[cfg(feature = "extra-headers")]
            extra_headers.clone(),
            composition.shared_topology(),
            composition.connection().as_ref(),
        )?;
        #[cfg(all(feature = "context-http-credentials", not(feature = "page-routes")))]
        let mut fetch_event_driver = context_http_credentials::EventDriver::new(
            context_http_credentials.clone(),
            composition.shared_topology(),
            composition.connection().as_ref(),
        )?;
        #[cfg(any(
            feature = "request-history",
            feature = "request-events",
            feature = "response-events"
        ))]
        {
            #[cfg(feature = "owned-launch")]
            let owns_browser_process = owned_browser.is_some();
            #[cfg(not(feature = "owned-launch"))]
            let owns_browser_process = false;
            let contexts = {
                let topology = composition.topology().map_err(HostError::backend)?;
                let mut contexts = Vec::new();
                for route in topology.topology().browser_context_routes() {
                    if context_metadata
                        .visible(
                            route.object_id(),
                            route.ownership()
                                == pwright_backend_cdp_topology::CdpContextOwnership::Owned,
                        )?
                        .is_some()
                        && eagerly_activate_network_context(route.ownership(), owns_browser_process)
                    {
                        contexts.push(route.object_id());
                    }
                }
                contexts
            };
            #[cfg(feature = "request-history")]
            network_events
                .enable_initial_history_contexts(&contexts)
                .await?;
            #[cfg(feature = "request-events")]
            network_events
                .enable_initial_request_contexts(&contexts)
                .await?;
            #[cfg(feature = "response-events")]
            network_events
                .enable_initial_response_contexts(&contexts)
                .await?;
        }
        let closed = composition.connection().closed();
        let stop = CancellationToken::new();
        #[cfg(feature = "concurrent-close")]
        let mut close_jobs =
            close_jobs::Jobs::new(&composition, &runtime, &frame_trees, stop.clone())?;
        let lifecycle_services = LifecycleServices::new(
            &composition,
            runtime.as_ref(),
            cleanup.clone(),
            lifecycle::LifecycleBindings {
                metadata: frame_metadata.clone(),
                trees: frame_trees.clone(),
                context_metadata: context_metadata.clone(),
                stop: stop.clone(),
                #[cfg(feature = "network-events")]
                network: Some(network_registration),
                #[cfg(not(feature = "network-events"))]
                network: None,
            },
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
        #[cfg(feature = "touchscreen-input")]
        let touchscreen = Arc::new(touchscreen::Installation::new(composition.resources())?);
        let keyboard = keyboard::Installation::new(composition.resources(), std::env::consts::OS)?;
        #[cfg(feature = "drag-runtime")]
        let keyboard = keyboard.with_drag(drag);
        let input_start = input_start::Order::new(composition.resources())?;
        #[cfg(feature = "handles")]
        let task_handles = handles.clone();
        #[cfg(feature = "worker-handle-evaluation")]
        let task_worker_handles = worker_handles.clone();
        #[cfg(all(feature = "network-events", feature = "page-routes"))]
        let task_page_routes = page_routes.clone();
        #[cfg(all(feature = "network-events", feature = "request-frame"))]
        let task_network_topology = composition.shared_topology();
        let state = Arc::new(Mutex::new(Some(LiveSession {
            #[cfg(feature = "cdp-session")]
            cdp_sessions: cdp_sessions.clone(),
            #[cfg(feature = "page-pause")]
            headful,
            #[cfg(feature = "init-scripts")]
            init_scripts,
            #[cfg(feature = "page-bindings")]
            page_bindings,
            #[cfg(feature = "page-routes")]
            page_routes,
            #[cfg(feature = "page-errors")]
            page_errors,
            #[cfg(feature = "console-history")]
            console_history,
            #[cfg(feature = "console-events")]
            console_subscriptions,
            #[cfg(feature = "emulate-media")]
            emulate_media,
            #[cfg(feature = "handles")]
            handles,
            #[cfg(feature = "network-events")]
            network_events: network_events.clone(),
            #[cfg(feature = "download-events")]
            download_events: download_events.clone(),
            #[cfg(feature = "worker-events")]
            worker_events: worker_events.clone(),
            #[cfg(feature = "worker-handle-evaluation")]
            worker_handles: worker_handles.clone(),
            #[cfg(feature = "worker-console-events")]
            worker_console_events: worker_console_events.clone(),
            #[cfg(feature = "extra-headers")]
            extra_headers,
            #[cfg(feature = "frame-navigation")]
            frame_navigation,
            #[cfg(feature = "wait-for-function")]
            wait_for_function,
            #[cfg(feature = "wait-for-timeout")]
            wait_for_timeout,
            #[cfg(feature = "frame-lifecycle")]
            frame_lifecycle,
            #[cfg(feature = "url-wait")]
            url_wait,
            #[cfg(feature = "set-content")]
            set_content,
            #[cfg(feature = "timeout-settings")]
            timeout_settings,
            #[cfg(feature = "device-emulation")]
            device_emulation,
            #[cfg(feature = "context-geolocation")]
            context_geolocation,
            #[cfg(feature = "context-geolocation-permission")]
            context_geolocation_permission,
            #[cfg(feature = "context-http-credentials")]
            context_http_credentials: context_http_credentials.clone(),
            #[cfg(feature = "context-locale")]
            context_locale,
            #[cfg(feature = "ignore-https-errors")]
            ignore_https_errors,
            #[cfg(feature = "bypass-csp")]
            bypass_csp,
            #[cfg(feature = "javascript-disabled")]
            javascript_disabled,
            #[cfg(feature = "context-timezone")]
            context_timezone,
            #[cfg(feature = "context-offline")]
            context_offline,
            #[cfg(feature = "context-proxy")]
            context_proxy,
            #[cfg(feature = "page-pdf")]
            page_pdf,
            #[cfg(feature = "page-viewport")]
            page_viewport,
            mouse: mouse.clone(),
            #[cfg(feature = "touchscreen-input")]
            touchscreen: touchscreen.clone(),
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
        #[cfg(feature = "cdp-session")]
        let task_cdp_sessions = cdp_sessions.clone();
        #[cfg(feature = "context-storage-state")]
        let task_context_storage_state = context_storage_state.clone();
        #[cfg(feature = "network-events")]
        let task_network_events = network_events.clone();
        #[cfg(feature = "context-http-credentials")]
        let task_context_http_credentials = context_http_credentials.clone();
        let (topology_barriers, mut barriers) = tokio::sync::mpsc::channel::<DriverRequest>(8);
        let driver = tokio::spawn(async move {
            let teardown = SessionTeardown {
                state: Arc::clone(&task_state),
                disconnected: task_disconnected,
                frame_metadata: task_metadata,
                frame_trees: task_frame_trees,
                context_metadata: task_context_metadata.clone(),
                #[cfg(feature = "context-storage-state")]
                context_storage_state: task_context_storage_state,
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
                    #[cfg(any(feature = "page-routes", feature = "context-http-credentials"))]
                    {
                        fetch_event_driver.run(&task_stop).await
                    }
                    #[cfg(not(any(feature = "page-routes", feature = "context-http-credentials")))]
                    {
                        std::future::pending::<Result<(), HostError>>().await
                    }
                } => (result, true),
                result = async {
                    loop {
                        #[cfg(feature = "retained-target-marking")]
                        file_input.retained.prune_markers();
                        #[cfg(feature = "handles")]
                        task_handles.finish_ready()?;
                        #[cfg(feature = "console-argument-handles")]
                        task_console_observer.finish_ready()?;
                        #[cfg(feature = "concurrent-close")]
                        let mut reply = close_jobs.finish_next(&task_state)?;
                        #[cfg(not(feature = "concurrent-close"))]
                        let mut reply = None;
                        if reply.is_none() {
                        let mut barrier = None;
                        #[cfg(feature = "network-events")]
                        let mut network_result = None;
                        let received = topology_recovery::receive_once_or_signal(
                            &mut topology_driver,
                            runtime.as_ref(),
                            async {
                            #[cfg(any(feature = "extra-headers", feature = "concurrent-close", feature = "handles", feature = "network-events"))]
                            tokio::select! {
                                received = barriers.recv() => { barrier = received; }
                                _result = async {
                                    #[cfg(feature = "network-events")]
                                    { network_driver.receive_once().await }
                                    #[cfg(not(feature = "network-events"))]
                                    { std::future::pending().await }
                                } => {
                                    #[cfg(feature = "network-events")]
                                    { network_result = Some(_result); }
                                }
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
                            #[cfg(not(any(feature = "extra-headers", feature = "concurrent-close", feature = "handles", feature = "network-events")))]
                            { barrier = barriers.recv().await; }
                            },
                        );
                        cooperate!(received)
                            .map_err(|error| driver_diagnostics::Phase::Receive.annotate(error))?;
                        #[cfg(feature = "network-events")]
                        if let Some(result) = network_result
                            && let Some(batch) = result.map_err(HostError::backend)?
                        {
                            let page = batch.page().ok_or_else(|| {
                                HostError::Backend("network batch has no Page source".into())
                            })?;
                            #[cfg(feature = "page-routes")]
                            let fulfilled_body = match (batch.session_id(), batch.native_request_id()) {
                                (Some(session_id), Some(request_id)) => task_page_routes
                                    .take_fulfilled_body(session_id, request_id)?,
                                _ => None,
                            };
                            #[cfg(not(feature = "page-routes"))]
                            let fulfilled_body = None;
                            #[cfg(feature = "request-frame")]
                            let network_topology = task_network_topology
                                .lock()
                                .map_err(HostError::backend)?;
                            task_network_events.publish(
                                &batch,
                                runtime.as_ref(),
                                #[cfg(feature = "request-frame")]
                                network_topology.topology(),
                                fulfilled_body,
                                // Keep runtime lifecycle and request history for
                                // scratch navigation, but never enqueue a
                                // language-visible notice. Native retirement
                                // waits for the single batch acknowledgement.
                                !teardown.frame_trees.is_internal_page(page)?,
                            )?;
                            network_driver
                                .acknowledge_batch_publication(page)
                                .map_err(HostError::backend)?;
                        }
                        context_lifecycle::prune_metadata(&task_state, &task_context_metadata)?;
                        #[cfg(feature = "cdp-session")]
                        cdp_sessions::reconcile_state(&task_state, &task_cdp_sessions)?;
                        #[cfg(feature = "network-events")]
                        network_events::reconcile_state(&task_state, &task_network_events)?;
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
                            let initialized = topology_recovery::initialize_snapshot(
                                &mut topology_driver,
                                runtime.as_ref(),
                            );
                            cooperate!(initialized)
                                .map_err(|error| driver_diagnostics::Phase::Snapshot.annotate(error))?;
                            #[cfg(feature = "extra-headers")]
                            let reply = {
                                let (reply, closed) = cooperate!(lifecycle_services.settle_header_initialization(&task_state, reply))?;
                                if closed {
                                    cooperate!(topology_recovery::initialize_snapshot(
                                        &mut topology_driver,
                                        runtime.as_ref(),
                                    ))
                                        .map_err(|error| driver_diagnostics::Phase::Snapshot.annotate(error))?;
                                }
                                reply
                            };
                            context_lifecycle::prune_metadata(&task_state, &task_context_metadata)?;
                            #[cfg(feature = "cdp-session")]
                            cdp_sessions::reconcile_state(&task_state, &task_cdp_sessions)?;
                            #[cfg(feature = "extra-headers")]
                            cooperate!(extra_headers::reconcile_native(&task_state))?;
                            // Events observed while executing the command must
                            // publish before its language-visible completion.
                            // In particular, a navigation can surface Log and
                            // Runtime console events while the barrier runs.
                            #[cfg(feature = "handles")]
                            task_handles.finish_ready()?;
                            #[cfg(feature = "console-argument-handles")]
                            task_console_observer.finish_ready()?;
                            if let Some(abandoned) = reply.send()? {
                                // The foreign receiver may stop while native
                                // creation is in flight. Compensate the exact
                                // new owner before accepting another barrier.
                                match abandoned {
                                    lifecycle::AbandonedCreation::Context(call_id, context) => {
                                        cooperate!(lifecycle_services.close_abandoned_context(call_id, context))?;
                                    }
                                    lifecycle::AbandonedCreation::Page(call_id, page) => {
                                        cooperate!(lifecycle_services.close_abandoned_page(call_id, page))?;
                                    }
                                }
                                cooperate!(topology_recovery::initialize_snapshot(
                                    &mut topology_driver,
                                    runtime.as_ref(),
                                ))
                                .map_err(|error| driver_diagnostics::Phase::Snapshot.annotate(error))?;
                            }
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
            #[cfg(feature = "network-events")]
            task_network_events.close();
            #[cfg(feature = "network-events")]
            drop(task_network_events);
            #[cfg(feature = "context-http-credentials")]
            task_context_http_credentials.close();
            input_start.close();
            #[cfg(feature = "concurrent-close")]
            {
                close_jobs.drain().await;
                drop(close_jobs);
            }
            task_operations.wait().await;
            #[cfg(feature = "cdp-session")]
            let cdp_session_retirement = task_cdp_sessions.shutdown(native_available).await;
            #[cfg(feature = "page-dialogs")]
            task_page_dialogs.close()?;
            #[cfg(feature = "page-dialogs")]
            drop(task_page_dialogs);
            #[cfg(feature = "worker-handle-evaluation")]
            {
                // Worker Handle descriptors may outlive their Python wrappers'
                // session. Revoke every native owner before draining the
                // shared cleanup executor, just like Frame Handle owners.
                task_worker_handles.close();
                drop(task_worker_handles);
            }
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
            #[cfg(feature = "touchscreen-input")]
            let touchscreen_retirement = touchscreen.shutdown().await;
            #[cfg(feature = "touchscreen-input")]
            drop(touchscreen);
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
            #[cfg(feature = "cdp-session")]
            drop(task_cdp_sessions);
            #[cfg(any(feature = "page-routes", feature = "context-http-credentials"))]
            drop(fetch_event_driver);
            #[cfg(feature = "context-http-credentials")]
            drop(task_context_http_credentials);
            #[cfg(feature = "console-argument-handles")]
            // The driver-side publication view is the topology observer's only
            // extra owner. It also owns a CDP connection, so it must not survive
            // into the transport's `io_finished` join below.
            drop(task_console_observer);
            #[cfg(feature = "network-events")]
            drop(network_driver);
            #[cfg(all(feature = "network-events", feature = "page-routes"))]
            drop(task_page_routes);
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
                .and({
                    #[cfg(feature = "cdp-session")]
                    {
                        cdp_session_retirement
                    }
                    #[cfg(not(feature = "cdp-session"))]
                    {
                        Ok(())
                    }
                })
                .and(
                    peer_close
                        .map_err(|error| driver_diagnostics::Phase::PeerTransport.annotate(error)),
                )
                .and(
                    mouse_retirement.map_err(|error| {
                        driver_diagnostics::Phase::MouseRetirement.annotate(error)
                    }),
                )
                .and({
                    #[cfg(feature = "touchscreen-input")]
                    {
                        touchscreen_retirement.map_err(|error| {
                            driver_diagnostics::Phase::TouchscreenRetirement.annotate(error)
                        })
                    }
                    #[cfg(not(feature = "touchscreen-input"))]
                    {
                        Ok(())
                    }
                })
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
            #[cfg(feature = "context-api-request")]
            browser_user_agent,
            resources,
            operations,
            next_call: AtomicU64::new(1),
            cleanup,
            frame_metadata,
            frame_trees,
            context_metadata,
            #[cfg(feature = "context-storage-state")]
            context_storage_state,
            topology_barriers,
            browser_pages: browser_page::BrowserPages::new(),
            #[cfg(feature = "owned-launch")]
            owned_browser,
            #[cfg(feature = "playwright-trace")]
            trace_directory,
        })
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    /// The caller-owned directory remains available for future Context trace
    /// composition; Browser shutdown does not delete it.
    #[cfg(feature = "playwright-trace")]
    pub fn trace_directory(&self) -> Option<&Path> {
        self.trace_directory.as_ref().map(|value| value.path())
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
        self.browser_pages.prune(|context_id| {
            topology
                .topology()
                .browser_context_routes()
                .any(|context| context.object_id() == context_id)
        })?;
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

    /// Read one Page's published frame tree without retaining foreign wrapper
    /// state or touching the browser transport.
    pub fn frames(&self, page_id: &str) -> Result<Vec<FrameDescriptor>, HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(pwright_core::ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        self.frame_trees
            .get(page)?
            .frames()?
            .into_iter()
            .map(|frame| {
                Ok(FrameDescriptor {
                    id: frame.id().get().to_string(),
                    parent_id: frame.parent_id().map(|id| id.get().to_string()),
                    name: frame.with_name(str::to_owned)?,
                    url: frame.url()?,
                })
            })
            .collect()
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
        let visible_context = topology
            .browser_context_routes()
            .find(|context| context.object_id().get().to_string() == context_id)
            .map(|context| {
                self.context_metadata.visible(
                    context.object_id(),
                    context.ownership() == pwright_backend_cdp_topology::CdpContextOwnership::Owned,
                )
            })
            .transpose()?
            .flatten()
            .is_some();
        if !visible_context {
            return Ok(PageSnapshot {
                ids: Vec::new(),
                new_pages: Vec::new(),
            });
        }
        let mut ids = Vec::new();
        let mut new_pages = Vec::new();
        for page in topology
            .page_session_routes()
            .filter(|page| page.browser_context_object_id().get().to_string() == context_id)
        {
            #[cfg(feature = "context-storage-state")]
            if self.frame_trees.is_internal_page(page.object_id())? {
                continue;
            }
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

    pub async fn flush_topology(&self) -> Result<(), HostError> {
        let (done, completion) = tokio::sync::oneshot::channel();
        self.send_driver_request(DriverRequest::Fence(done))?;
        tokio::select! {
            biased;
            () = self.stop.cancelled() => Err(HostError::TargetClosed),
            result = completion => result.map_err(|_| HostError::TargetClosed),
        }
    }

    #[cfg(feature = "file-chooser-events")]
    pub async fn set_page_file_chooser_interception(
        &self,
        page_id: &str,
        enabled: bool,
    ) -> Result<(), HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(pwright_core::ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        self.flush_topology().await?;
        let (connection, session_id, target_id) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let route = topology
                .topology()
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .ok_or(HostError::TargetClosed)?;
            (
                Arc::clone(live.composition.connection()),
                route.session_id().to_owned(),
                route.target_id().to_owned(),
            )
        };
        CdpSession::new(connection, session_id, target_id)
            .page_set_intercept_file_chooser_dialog(enabled)
            .await
            .map_err(HostError::backend)
    }

    #[cfg(feature = "console-events")]
    pub fn set_page_console_listener(&self, page_id: &str, enabled: bool) -> Result<(), HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(pwright_core::ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        live.console_subscriptions
            .update_page_listener(page, enabled)
    }

    #[cfg(feature = "console-events")]
    pub fn set_context_console_listener(
        &self,
        context_id: &str,
        enabled: bool,
    ) -> Result<(), HostError> {
        let context = context_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(pwright_core::ObjectId::new)
            .ok_or_else(|| {
                HostError::InvalidArgument("invalid portable BrowserContext ID".into())
            })?;
        self.require_visible_context_id(context)?;
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        live.console_subscriptions
            .update_context_listener(context, enabled)
    }

    #[cfg(feature = "console-events")]
    pub fn subscribe_page_console(&self, page_id: &str) -> Result<u64, HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(pwright_core::ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        live.console_subscriptions.subscribe_page_waiter(page)
    }

    #[cfg(feature = "console-events")]
    pub fn subscribe_context_console(&self, context_id: &str) -> Result<u64, HostError> {
        let context = context_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(pwright_core::ObjectId::new)
            .ok_or_else(|| {
                HostError::InvalidArgument("invalid portable BrowserContext ID".into())
            })?;
        self.require_visible_context_id(context)?;
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        live.console_subscriptions.subscribe_context_waiter(context)
    }

    #[cfg(feature = "console-events")]
    pub fn release_console_subscription(&self, token: u64) -> Result<(), HostError> {
        if token == 0 {
            return Err(HostError::InvalidArgument(
                "invalid console subscription token".into(),
            ));
        }
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        live.console_subscriptions.release_waiter(token)
    }

    #[cfg(feature = "console-events")]
    pub(crate) fn activate_console_subscriptions(&self) -> Result<(), HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        live.console_subscriptions.activate_pending()
    }

    #[cfg(feature = "owned-launch")]
    async fn shutdown_owned_browser(&self) -> Result<(), HostError> {
        let Some(owner) = self.owned_browser.as_ref() else {
            return Ok(());
        };
        // The provider report is retained by the one-flight owner. A second
        // close cannot observe a taken lease as successful shutdown, and a
        // cancelled caller cannot cancel the started provider shutdown.
        let outcome = owner
            .shutdown
            .run(Duration::from_secs(30))
            .await
            .map_err(HostError::backend)?;
        match outcome.as_ref() {
            Ok(_) => Ok(()),
            Err(error) => Err(HostError::backend(format_args!(
                "owned Chromium shutdown failed: {error}"
            ))),
        }
    }

    pub async fn close(&self) -> Result<(), HostError> {
        #[cfg(feature = "owned-launch")]
        let owned_result = self.shutdown_owned_browser().await;
        let mut driver = self.driver.lock().await;
        self.stop.cancel();
        let driver_result = if let Some(task) = driver.as_mut() {
            let result = task.await;
            driver.take();
            result.map_err(HostError::backend)?
        } else {
            Ok(())
        };
        #[cfg(feature = "owned-launch")]
        return owned_result.and(driver_result);
        #[cfg(not(feature = "owned-launch"))]
        driver_result
    }
}

#[cfg(any(
    feature = "request-history",
    feature = "request-events",
    feature = "response-events"
))]
fn eagerly_activate_network_context(
    ownership: pwright_backend_cdp_topology::CdpContextOwnership,
    owns_browser_process: bool,
) -> bool {
    owns_browser_process || ownership == pwright_backend_cdp_topology::CdpContextOwnership::Owned
}

impl Drop for HostSession {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::{ConnectPhase, HostError};

    #[cfg(all(unix, feature = "owned-launch"))]
    #[tokio::test]
    async fn owned_launch_preserves_bounded_process_logs_in_host_error() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        use std::sync::atomic::{AtomicUsize, Ordering};

        use super::{ChromiumChannel, ChromiumExecutable, HostSession};

        struct TestDirectory(std::path::PathBuf);
        impl Drop for TestDirectory {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

        // Arrange
        let path = std::env::temp_dir().join(format!(
            "pwright-node-host-startup-logs-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("synthetic test directory should be created");
        let directory = TestDirectory(path);
        let executable_path = directory.0.join("synthetic-process");
        fs::write(
            &executable_path,
            b"#!/bin/sh\nprintf 'host stdout\\n'\nprintf 'host stderr\\n' >&2\nexit 1\n",
        )
        .expect("synthetic process script should be written");
        fs::set_permissions(&executable_path, fs::Permissions::from_mode(0o700))
            .expect("synthetic process should be executable");
        let profile = directory.0.join("profile");
        fs::create_dir(&profile).expect("empty profile should be created");
        let executable = ChromiumExecutable::from_path(executable_path, ChromiumChannel::Custom)
            .expect("synthetic process should validate");

        // Act
        let result =
            HostSession::launch_chromium_with_lifecycle_events(executable, profile, true, 2_000)
                .await;

        // Assert
        let error = match result {
            Ok(_) => panic!("synthetic process must exit before CDP"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "PWRIGHT_BACKEND");
        let message = error.to_string();
        assert!(message.contains("Browser logs:"));
        assert!(message.contains("[out] host stdout"));
        assert!(message.contains("[err] host stderr"));
        assert!(message.len() <= 4096);
    }

    #[cfg(any(
        feature = "request-history",
        feature = "request-events",
        feature = "response-events"
    ))]
    #[test]
    fn cdp_connect_future_is_send_with_network_initialization() {
        // Arrange
        fn require_send<T: Send>(_: T) {}

        // Act: creating the future does not poll it or contact a browser.
        let future = super::HostSession::connect_over_cdp("", 0);

        // Assert
        require_send(future);
    }

    #[cfg(any(
        feature = "request-history",
        feature = "request-events",
        feature = "response-events"
    ))]
    #[test]
    fn attached_network_history_does_not_eagerly_observe_borrowed_contexts() {
        // Arrange
        use pwright_backend_cdp_topology::CdpContextOwnership;
        let attached = false;

        // Act
        let default =
            super::eagerly_activate_network_context(CdpContextOwnership::Default, attached);
        let external =
            super::eagerly_activate_network_context(CdpContextOwnership::External, attached);
        let owned = super::eagerly_activate_network_context(CdpContextOwnership::Owned, attached);

        // Assert
        assert!(!default);
        assert!(!external);
        assert!(owned);
    }

    #[cfg(all(
        feature = "owned-launch",
        any(
            feature = "request-history",
            feature = "request-events",
            feature = "response-events"
        )
    ))]
    #[test]
    fn owned_browser_eagerly_observes_its_default_context() {
        // Arrange
        use pwright_backend_cdp_topology::CdpContextOwnership;
        let owns_browser_process = true;

        // Act
        let selected = super::eagerly_activate_network_context(
            CdpContextOwnership::Default,
            owns_browser_process,
        );

        // Assert
        assert!(selected);
    }

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
