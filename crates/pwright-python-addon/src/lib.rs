//! Python value and lifecycle adaptation only. Browser behavior stays in the
//! reusable Rust host.

#[cfg(feature = "playwright-server-connect-session")]
use pwright_playwright_server_browser_ops::{
    BrowserOperationError, EmptyContextOptions, OperationMetadata, RemoteBrowser, RemoteContext,
    RemotePage,
};
#[cfg(feature = "playwright-server-connect-session")]
use pwright_playwright_server_connect::{ConnectError, ConnectOptions};
#[cfg(feature = "playwright-server-connect-session")]
use pwright_playwright_server_endpoint_discovery::{DiscoveryLimits, MAX_DISCOVERY_BODY_BYTES};
#[cfg(feature = "playwright-server-close-notices")]
use pwright_playwright_server_session::{CloseNotice, SupervisorError};
#[cfg(feature = "playwright-server-connect-session")]
use pwright_playwright_server_session::{EventPolicy, RemoteBrowserSupervisor, SdkLanguage};
#[cfg(feature = "playwright-server-connect-session")]
use pwright_transport_ws::WsLimits;
use std::collections::BTreeMap;
use std::num::{NonZeroU8, NonZeroU32};
use std::path::Path;
#[cfg(any(feature = "persistent-context", feature = "fresh-launch"))]
use std::path::PathBuf;
use std::str::FromStr;
#[cfg(feature = "playwright-server-connect-session")]
use std::sync::Weak;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
#[cfg(feature = "owned-process-startup")]
use std::time::Duration;
#[cfg(any(
    feature = "playwright-server-initialize",
    feature = "playwright-server-connect-session"
))]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "frame-navigation")]
use pwright_cap_navigation::WaitUntil as FrameWaitUntil;
#[cfg(feature = "persistent-context")]
use pwright_node_host::ChromiumPersistentProfile;
#[cfg(feature = "worker-console-events")]
use pwright_node_host::WorkerConsoleNotice;
#[cfg(feature = "context-cookies")]
use pwright_node_host::context_cookies::{
    BrowserContextCookieClearFilter, BrowserContextCookieInput, CookieStringMatcher,
};
#[cfg(feature = "context-storage-state")]
use pwright_node_host::context_storage_state::{
    ContextStorageItem, ContextStorageOrigin, ContextStorageState, ContextStorageStateSnapshot,
    MAX_STORAGE_ITEMS, MAX_STORAGE_ORIGINS, MAX_STORAGE_STATE_BYTES,
};
#[cfg(feature = "element-handle-state-predicates")]
use pwright_node_host::element_handle_dom::ElementHandleStatePredicate;
#[cfg(feature = "element-handle-dom-reads")]
use pwright_node_host::element_handle_dom::{ElementHandleDomRead, ElementHandleDomValue};
use pwright_node_host::ffi_budget::{FfiBudget, FfiLane, FfiLimits, FfiMetrics};
#[cfg(feature = "handles")]
use pwright_node_host::locator::HostFrameLocator;
#[cfg(any(feature = "element-handle-dom", feature = "locator-bounding-box"))]
use pwright_node_host::locator::LocatorBoundingBox;
use pwright_node_host::locator::action::{
    ActionValue, ClickOptionsV2, FillOptions, HoverOptions, LocatorAction, MouseButton, Position,
    PressKey, PressOptions, SelectOptionOptions, SelectOptionValue, SetCheckedOptions,
};
#[cfg(any(feature = "locator-click-modifiers", feature = "touchscreen-input"))]
use pwright_node_host::locator::action::{KeyboardModifier, KeyboardModifiers};
#[cfg(feature = "drag-and-drop")]
use pwright_node_host::locator::drag::{DragPosition, DragToOptions};
use pwright_node_host::locator::{
    AttributeExpectation, CssExpectation, HostLocator, LocatorExpectationState, LocatorRead,
    LocatorStringExpectationKind, LocatorValue, LocatorWaitState, StringMatcher, TextExpectation,
    TextsExpectation,
};
#[cfg(feature = "element-handle-dom")]
use pwright_node_host::locator_query::ElementHandleQuery;
use pwright_node_host::locator_query::semantic::{
    RoleQueryOptions, SemanticSelector, TestIdAttributes,
};
use pwright_node_host::locator_query::{LocatorQuery, QueryBudget, QueryCombination, QueryLimits};
#[cfg(feature = "response-security-details")]
use pwright_node_host::network_events::CdpResponseSecurityDetails;
#[cfg(feature = "request-history")]
use pwright_node_host::network_events::RequestHistoryOwner;
#[cfg(feature = "request-events")]
use pwright_node_host::network_events::RequestNotice;
#[cfg(feature = "request-timing")]
use pwright_node_host::network_events::RequestTiming;
#[cfg(feature = "response-finished")]
use pwright_node_host::network_events::ResponseFinishedOwner;
#[cfg(feature = "response-events")]
use pwright_node_host::network_events::ResponseNotice;
#[cfg(feature = "network-events")]
use pwright_node_host::network_events::{
    NetworkEventKind, NetworkEventRecord, NetworkRequestRecord, NetworkResponseRecord,
    NetworkWaitRecord, RequestSizeOwner, RequestSizes,
};
use pwright_node_host::screenshot::{ScreenshotFormat, ScreenshotOptions, ScreenshotScale};
#[cfg(feature = "selector-registration")]
use pwright_node_host::selector_registry::{
    MAX_SELECTOR_ENGINE_NAME_BYTES, MAX_SELECTOR_ENGINE_SOURCE_BYTES, MAX_SELECTOR_ENGINES,
    MAX_SELECTOR_REGISTRY_BYTES, MAX_TEST_ID_ATTRIBUTE_BYTES, SelectorRegistry,
};
#[cfg(feature = "touchscreen-input")]
use pwright_node_host::touchscreen::{SelectorTapOptions, TapParams, TouchscreenCoordinate};
#[cfg(feature = "cdp-handshake-headers")]
use pwright_node_host::{
    CDP_HANDSHAKE_HEADER_MAX_BATCH_BYTES, CDP_HANDSHAKE_HEADER_MAX_COUNT,
    CDP_HANDSHAKE_HEADER_MAX_NAME_BYTES, CDP_HANDSHAKE_HEADER_MAX_VALUE_BYTES,
};
#[cfg(feature = "cdp-session")]
use pwright_node_host::{CdpSessionEvent, CdpSessionEventBatch};
#[cfg(any(feature = "persistent-context", feature = "fresh-launch"))]
use pwright_node_host::{ChromiumChannel, ChromiumExecutable, discover_system_chromium};
use pwright_node_host::{
    ClosePageDisposition, FramePublicationFence, HostError, HostSession, NavigationResult,
    SessionMetrics,
};
#[cfg(feature = "download-events")]
use pwright_node_host::{DownloadNotice, DownloadOwner};
#[cfg(feature = "page-events")]
use pwright_node_host::{
    LifecycleDelivery, LifecycleNotice, LifecycleNoticeView, PublicationDrainError,
    PublicationProgress,
};
#[cfg(feature = "layout-selectors")]
use pwright_node_host::{MAX_LAYOUT_SELECTOR_PARTS, MAX_LAYOUT_SELECTOR_PROGRAM_BYTES};
#[cfg(feature = "worker-events")]
use pwright_node_host::{WorkerNotice, WorkerNoticeKind, WorkerOwner};
use pyo3::PyTypeInfo;
use pyo3::create_exception;
#[cfg(any(
    feature = "wait-for-url",
    feature = "page-events",
    feature = "navigation-url-match"
))]
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::{PyException, PyNotImplementedError};
use pyo3::prelude::*;
#[cfg(any(
    feature = "context-cookies",
    feature = "page-routes",
    feature = "cdp-handshake-headers"
))]
use pyo3::types::PyTuple;
use pyo3::types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyString, PyStringMethods};
use tokio_util::sync::CancellationToken;

#[cfg(feature = "api-request")]
mod api_request;
mod browser_registry;
#[cfg(feature = "context-strict-selectors")]
mod context_strict_selectors;
mod evaluation_value;
#[cfg(feature = "handles")]
mod handles;
#[cfg(feature = "page-select-option")]
mod page_select_option;

const NATIVE_ABI_VERSION: u32 = 131;
const MAX_SESSIONS: usize = 16;
const SMALL_ALLOWANCE: u64 = 32 * 1024;
const READ_ALLOWANCE: u64 = 1024 * 1024 + 4096;
const NAVIGATION_ALLOWANCE: u64 = 256 * 1024;
const ACTION_ALLOWANCE: u64 = 6 * 1024 * 1024;
#[cfg(feature = "cdp-session")]
const MAX_CDP_EVENT_PYTHON_DEPTH: usize = 64;
#[cfg(feature = "cdp-session")]
const MAX_CDP_EVENT_PYTHON_NODES: usize = 65_536;
#[cfg(feature = "extra-headers")]
const EXTRA_HEADERS_ALLOWANCE: u64 = 512 * 1024;
#[cfg(feature = "extra-headers")]
fn validate_new_context_headers(headers: &[(String, String)]) -> Result<(), HostError> {
    if headers.len() > pwright_node_host::extra_headers::MAX_EXTRA_HEADERS {
        return Err(HostError::ResourceExhausted(
            "Browser.new_context extra header count exceeds 256".into(),
        ));
    }
    let bytes = headers.iter().try_fold(0usize, |total, (name, value)| {
        total
            .checked_add(name.len())
            .and_then(|total| total.checked_add(value.len()))
            .ok_or_else(|| {
                HostError::ResourceExhausted(
                    "Browser.new_context extra header bytes overflow".into(),
                )
            })
    })?;
    if bytes > pwright_node_host::extra_headers::MAX_EXTRA_HEADER_BYTES {
        return Err(HostError::ResourceExhausted(
            "Browser.new_context extra header bytes exceed 65536".into(),
        ));
    }
    Ok(())
}
#[cfg(feature = "fresh-launch")]
const FRESH_LAUNCH_ALLOWANCE: u64 = 512 * 1024;
#[cfg(feature = "owned-process-startup")]
const OWNED_PROCESS_STARTUP_ALLOWANCE: u64 = 64 * 1024;
#[cfg(any(
    feature = "playwright-server-initialize",
    feature = "playwright-server-connect-session"
))]
const PLAYWRIGHT_SERVER_INITIALIZE_ALLOWANCE: u64 = 512 * 1024;
#[cfg(any(
    feature = "playwright-server-initialize",
    feature = "playwright-server-connect-session"
))]
const PLAYWRIGHT_SERVER_ENDPOINT_BYTES: usize = 8192;
#[cfg(feature = "persistent-context")]
const PERSISTENT_LAUNCH_ALLOWANCE: u64 = 512 * 1024;
#[cfg(feature = "network-events")]
const NETWORK_EVENT_ALLOWANCE: u64 = 1024 * 1024;
#[cfg(feature = "download-events")]
const DOWNLOAD_EVENT_ALLOWANCE: u64 = 16 * 1024;
#[cfg(feature = "worker-events")]
const WORKER_EVENT_ALLOWANCE: u64 = 64 * 1024;
#[cfg(feature = "context-storage-state")]
const STORAGE_STATE_ALLOWANCE: u64 = 2 * MAX_STORAGE_STATE_BYTES as u64 + 2 * 1024 * 1024;

#[cfg(feature = "mouse")]
enum PythonMouseOperation {
    Move(pwright_node_host::mouse::MoveParams),
    Down(pwright_node_host::mouse::DownParams),
    Up(pwright_node_host::mouse::UpParams),
    Click(pwright_node_host::mouse::ClickParams),
    Dblclick(pwright_node_host::mouse::DblclickParams),
    Wheel(pwright_node_host::mouse::WheelParams),
}

#[cfg(feature = "keyboard")]
enum PythonKeyboardOperation {
    Down(String),
    Up(String),
    Press(String, std::time::Duration),
    InsertText(String),
    Type(String, std::time::Duration),
}

#[cfg(feature = "keyboard")]
fn python_keyboard_delay(value: f64) -> Result<std::time::Duration, HostError> {
    if !value.is_finite() || value < 0.0 || value > f64::from(u32::MAX) {
        return Err(HostError::InvalidArgument(
            "Keyboard delay must be finite and between 0 and 4294967295 ms".into(),
        ));
    }
    Ok(std::time::Duration::from_secs_f64(value / 1000.0))
}

#[cfg(feature = "keyboard")]
fn python_keyboard_operation(
    name: &str,
    value: &str,
    delay_ms: f64,
) -> Result<PythonKeyboardOperation, HostError> {
    let delay = || python_keyboard_delay(delay_ms);
    let key = || {
        if value.len() > pwright_node_host::keyboard::KEYBOARD_KEY_MAX_BYTES {
            return Err(HostError::InvalidArgument(format!(
                "Keyboard key exceeds {} UTF-8 bytes",
                pwright_node_host::keyboard::KEYBOARD_KEY_MAX_BYTES
            )));
        }
        Ok(value.to_owned())
    };
    let text = || {
        if value.len() > pwright_node_host::keyboard::KEYBOARD_TEXT_MAX_BYTES {
            return Err(HostError::InvalidArgument(format!(
                "Keyboard text exceeds {} UTF-8 bytes",
                pwright_node_host::keyboard::KEYBOARD_TEXT_MAX_BYTES
            )));
        }
        Ok(value.to_owned())
    };
    match name {
        "down" => Ok(PythonKeyboardOperation::Down(key()?)),
        "up" => Ok(PythonKeyboardOperation::Up(key()?)),
        "press" => Ok(PythonKeyboardOperation::Press(key()?, delay()?)),
        "insert_text" => Ok(PythonKeyboardOperation::InsertText(text()?)),
        "type" => Ok(PythonKeyboardOperation::Type(text()?, delay()?)),
        _ => Err(HostError::InvalidArgument(
            "unknown Keyboard operation".into(),
        )),
    }
}

#[cfg(feature = "mouse")]
fn invalid_mouse_argument(error: impl std::fmt::Display) -> HostError {
    HostError::InvalidArgument(error.to_string())
}

#[cfg(feature = "mouse")]
fn python_mouse_button(value: f64) -> Result<pwright_node_host::mouse::MouseButton, HostError> {
    match value {
        0.0 => Ok(pwright_node_host::mouse::MouseButton::Left),
        1.0 => Ok(pwright_node_host::mouse::MouseButton::Right),
        2.0 => Ok(pwright_node_host::mouse::MouseButton::Middle),
        _ => Err(invalid_mouse_argument("invalid Mouse button")),
    }
}

#[cfg(feature = "mouse")]
fn python_mouse_count(value: f64) -> Result<i32, HostError> {
    if !value.is_finite()
        || value.fract() != 0.0
        || value < f64::from(i32::MIN)
        || value > f64::from(i32::MAX)
    {
        return Err(invalid_mouse_argument(
            "Mouse clickCount must be a signed 32-bit integer",
        ));
    }
    Ok(value as i32)
}

#[cfg(feature = "mouse")]
fn python_mouse_delay(value: f64) -> Result<Option<std::time::Duration>, HostError> {
    if !value.is_finite() || value < 0.0 || value > f64::from(u32::MAX) {
        return Err(invalid_mouse_argument(
            "Mouse delay must be finite and between 0 and 4294967295 ms",
        ));
    }
    Ok((value > 0.0).then(|| std::time::Duration::from_secs_f64(value / 1000.0)))
}

#[cfg(feature = "mouse")]
fn python_mouse_operation(
    name: &str,
    x: f64,
    y: f64,
    options: (f64, f64, f64, f64),
) -> Result<PythonMouseOperation, HostError> {
    use pwright_node_host::mouse::{
        ClickParams, DblclickParams, DownParams, MoveParams, WheelParams,
    };

    let (button, steps, click_count, delay_ms) = options;
    Ok(match name {
        "move" => PythonMouseOperation::Move(MoveParams {
            x: x.try_into().map_err(invalid_mouse_argument)?,
            y: y.try_into().map_err(invalid_mouse_argument)?,
            steps: steps.try_into().map_err(invalid_mouse_argument)?,
        }),
        "down" => PythonMouseOperation::Down(DownParams {
            button: python_mouse_button(button)?,
            click_count: python_mouse_count(click_count)?,
        }),
        "up" => PythonMouseOperation::Up(pwright_node_host::mouse::UpParams {
            button: python_mouse_button(button)?,
            click_count: python_mouse_count(click_count)?,
        }),
        "click" => PythonMouseOperation::Click(ClickParams {
            x: x.try_into().map_err(invalid_mouse_argument)?,
            y: y.try_into().map_err(invalid_mouse_argument)?,
            button: python_mouse_button(button)?,
            click_count: click_count.try_into().map_err(invalid_mouse_argument)?,
            delay: python_mouse_delay(delay_ms)?,
        }),
        "dblclick" => PythonMouseOperation::Dblclick(DblclickParams {
            x: x.try_into().map_err(invalid_mouse_argument)?,
            y: y.try_into().map_err(invalid_mouse_argument)?,
            button: python_mouse_button(button)?,
            delay: python_mouse_delay(delay_ms)?,
        }),
        "wheel" => PythonMouseOperation::Wheel(WheelParams {
            delta_x: x.try_into().map_err(invalid_mouse_argument)?,
            delta_y: y.try_into().map_err(invalid_mouse_argument)?,
        }),
        _ => return Err(invalid_mouse_argument("unknown Mouse operation")),
    })
}
#[cfg(feature = "worker-console-events")]
const WORKER_CONSOLE_EVENT_ALLOWANCE: u64 = 256 * 1024;
#[cfg(feature = "page-routes")]
const PAGE_ROUTE_ALLOWANCE: u64 = 512 * 1024;
#[cfg(feature = "page-routes")]
const PAGE_ROUTE_CONTINUE_ALLOWANCE: u64 = 512 * 1024;
#[cfg(feature = "page-routes")]
const PAGE_ROUTE_FULFILL_ALLOWANCE: u64 =
    pwright_node_host::MAX_ROUTE_FULFILL_BODY_BYTES as u64 + 128 * 1024;
#[cfg(feature = "page-routes")]
const _: () = assert!(
    PAGE_ROUTE_CONTINUE_ALLOWANCE as usize >= pwright_node_host::MAX_ROUTE_CONTINUE_INPUT_BYTES
);
#[cfg(feature = "request-history")]
const REQUEST_HISTORY_ALLOWANCE: u64 = 40 * 1024 * 1024;
const MAX_FFI_WATCHES: usize = if cfg!(feature = "request-history") {
    256
} else {
    16
};
const MAX_ENDPOINT_BYTES: usize = 8192;
const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_OBJECT_ID_BYTES: usize = 20;
const MAX_URL_BYTES: usize = 64 * 1024;
const MAX_LOCATOR_INPUT_BYTES: usize = 64 * 1024;
#[cfg(feature = "worker-console-events")]
const _: () = assert!(
    WORKER_CONSOLE_EVENT_ALLOWANCE as usize
        >= 2 * MAX_URL_BYTES + pwright_node_host::ConsoleMessage::maximum_payload_bytes() + 4096
);

fn locator_wait_state(value: &str) -> Result<LocatorWaitState, HostError> {
    match value {
        "attached" => Ok(LocatorWaitState::Attached),
        "detached" => Ok(LocatorWaitState::Detached),
        "visible" => Ok(LocatorWaitState::Visible),
        "hidden" => Ok(LocatorWaitState::Hidden),
        _ => Err(HostError::InvalidArgument(
            "invalid Locator wait state".into(),
        )),
    }
}

#[cfg(feature = "frame-navigation")]
fn frame_wait_until(value: &str) -> Result<FrameWaitUntil, HostError> {
    match value {
        "commit" => Ok(FrameWaitUntil::Commit),
        "domcontentloaded" => Ok(FrameWaitUntil::DomContentLoaded),
        "load" => Ok(FrameWaitUntil::Load),
        "networkidle" => Ok(FrameWaitUntil::NetworkIdle),
        _ => Err(HostError::InvalidArgument(
            "wait_until must be commit, domcontentloaded, load, or networkidle".into(),
        )),
    }
}
#[cfg(feature = "page-events")]
const LIFECYCLE_CONVERSION_BYTES: usize = 256 * 1024;
// The descriptor can copy two bounded metadata URLs alongside the largest
// feature-selected event payload before PyO3 owns the resulting objects.
// MAX_URL_BYTES is deliberately the binding's more conservative input bound,
// rather than the host's current 16 KiB metadata bound, so this proof remains
// valid if the host widens metadata URLs independently.
#[cfg(all(feature = "page-events", feature = "page-errors"))]
const _: () = assert!(
    LIFECYCLE_CONVERSION_BYTES
        >= 2 * MAX_URL_BYTES + pwright_node_host::PageError::maximum_payload_bytes() + 4096
);
#[cfg(all(feature = "page-events", feature = "console-events"))]
const _: () = assert!(
    LIFECYCLE_CONVERSION_BYTES
        >= 2 * MAX_URL_BYTES + pwright_node_host::ConsoleMessage::maximum_payload_bytes() + 4096
);
type PythonMediaOptions = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);
type PythonDeviceOptions = (
    Option<String>,
    Option<(u32, u32)>,
    Option<(u32, u32)>,
    Option<f64>,
    Option<bool>,
    Option<bool>,
);
type PythonGeolocation = (f64, f64, f64);
type PythonHttpCredentials = (String, String, Option<String>, bool);
#[cfg(feature = "persistent-context")]
type PythonPersistentMediaOptions = (Option<String>, Option<String>, Option<String>);
#[cfg(feature = "persistent-context")]
type PythonPersistentExtraHeaders = Vec<(String, String)>;
#[cfg(feature = "persistent-context")]
type PythonPersistentContextPolicies = (
    Option<String>,
    Option<String>,
    Option<PythonGeolocation>,
    bool,
    Option<PythonHttpCredentials>,
    bool,
    bool,
    bool,
    bool,
);
#[cfg(feature = "persistent-context")]
type PythonPersistentLaunchOptions = (
    Option<String>,
    PythonPersistentContextPolicies,
    Option<PythonDeviceOptions>,
    Option<PythonPersistentMediaOptions>,
    Option<PythonPersistentExtraHeaders>,
    Vec<String>,
    Option<String>,
    Option<String>,
    u32,
);
#[cfg(feature = "persistent-context-extra-headers")]
const _: () = assert!(
    PERSISTENT_LAUNCH_ALLOWANCE as usize
        >= 2 * pwright_node_host::extra_headers::MAX_EXTRA_HEADER_BYTES
            + 2 * MAX_PATH_BYTES
            + MAX_URL_BYTES
            + pwright_node_host::extra_headers::MAX_EXTRA_HEADERS
                * 2
                * std::mem::size_of::<String>()
            + 64 * 1024
);
type PythonDragOptions = (
    Option<(f64, f64)>,
    Option<(f64, f64)>,
    f64,
    bool,
    bool,
    bool,
);
create_exception!(_native, Error, PyException);
create_exception!(_native, TimeoutError, Error);
create_exception!(_native, TargetClosedError, Error);
create_exception!(_native, ResourceExhaustedError, Error);
create_exception!(_native, JavascriptError, Error);
create_exception!(_native, InvalidArgumentError, Error);
create_exception!(_native, UnsupportedError, PyNotImplementedError);

fn python_exception_message(error: &HostError) -> String {
    let message = error.to_string();
    if matches!(error, HostError::Timeout(_)) {
        format!("{message}.")
    } else {
        message
    }
}

fn binding_error(error: HostError) -> PyErr {
    let message = python_exception_message(&error);
    match error.code() {
        "PWRIGHT_INVALID_ARGUMENT" => InvalidArgumentError::new_err(message),
        "PWRIGHT_RESOURCE_EXHAUSTED" => ResourceExhaustedError::new_err(message),
        "PWRIGHT_TIMEOUT" => TimeoutError::new_err(message),
        "PWRIGHT_TARGET_CLOSED" => TargetClosedError::new_err(message),
        "PWRIGHT_UNSUPPORTED" => UnsupportedError::new_err(message),
        "PWRIGHT_JAVASCRIPT" => JavascriptError::new_err(message),
        _ => Error::new_err(message),
    }
}

fn locator_click_options(
    steps: Option<&Bound<'_, PyAny>>,
    force: Option<&Bound<'_, PyAny>>,
) -> PyResult<ClickOptionsV2> {
    const MAX_CLICK_STEPS: u32 = 10_000;
    const CLICK_STEPS_ERROR: &str = "steps must be an integer in [1, 10000]";

    let mut options = ClickOptionsV2::new();
    if let Some(force) = force {
        if !force.is_instance_of::<PyBool>() {
            return Err(binding_error(HostError::InvalidArgument(
                "Locator.click force must be a boolean".into(),
            )));
        }
        options = options.with_force(force.extract::<bool>()?);
    }
    let Some(steps) = steps else {
        return Ok(options);
    };
    let invalid = || binding_error(HostError::InvalidArgument(CLICK_STEPS_ERROR.into()));
    if steps.is_instance_of::<PyBool>() || !steps.is_instance_of::<PyInt>() {
        return Err(invalid());
    }
    let steps = steps.extract::<u32>().map_err(|_| invalid())?;
    let steps = NonZeroU32::new(steps)
        .filter(|steps| steps.get() <= MAX_CLICK_STEPS)
        .ok_or_else(invalid)?;
    Ok(options.with_steps(steps))
}

fn page_click_options(options: &Bound<'_, PyDict>) -> PyResult<ClickOptionsV2> {
    let invalid = |message: &str| binding_error(HostError::InvalidArgument(message.into()));
    if options.len() > 5 {
        return Err(invalid("Page.click accepts at most five native options"));
    }

    let mut parsed = ClickOptionsV2::new();
    let mut known = 0;
    if let Some(value) = options.get_item("button")? {
        known += 1;
        let value = value
            .cast::<PyString>()
            .map_err(|_| invalid("Page.click button must be left, right, or middle"))?;
        if value.len()? > 8 {
            return Err(invalid("Page.click button must be left, right, or middle"));
        }
        let name = value.to_cow()?;
        parsed.button = Some(match name.as_ref() {
            "left" => MouseButton::Left,
            "right" => MouseButton::Right,
            "middle" => MouseButton::Middle,
            _ => return Err(invalid("Page.click button must be left, right, or middle")),
        });
    }
    if let Some(value) = options.get_item("click_count")? {
        known += 1;
        let invalid_count = || invalid("Page.click click_count must be an integer in [1, 255]");
        if value.is_instance_of::<PyBool>() || !value.is_instance_of::<PyInt>() {
            return Err(invalid_count());
        }
        parsed.click_count = Some(
            value
                .extract::<u8>()
                .ok()
                .and_then(NonZeroU8::new)
                .ok_or_else(invalid_count)?,
        );
    }
    if let Some(value) = options.get_item("force")? {
        known += 1;
        if !value.is_instance_of::<PyBool>() {
            return Err(invalid("Page.click force must be a boolean"));
        }
        parsed.force = Some(value.extract::<bool>()?);
    }
    if let Some(value) = options.get_item("position")? {
        known += 1;
        let point = value
            .cast::<PyDict>()
            .map_err(|_| invalid("Page.click position must have finite x and y"))?;
        if point.len() != 2 {
            return Err(invalid("Page.click position must have finite x and y"));
        }
        let coordinate = |axis: &str| -> PyResult<f64> {
            let value = point
                .get_item(axis)?
                .ok_or_else(|| invalid("Page.click position must have finite x and y"))?;
            if value.is_instance_of::<PyBool>()
                || !(value.is_instance_of::<PyInt>() || value.is_instance_of::<PyFloat>())
            {
                return Err(invalid("Page.click position must have finite x and y"));
            }
            value
                .extract::<f64>()
                .map_err(|_| invalid("Page.click position must have finite x and y"))
        };
        parsed.position = Some(
            Position::new(coordinate("x")?, coordinate("y")?)
                .map_err(|_| invalid("Page.click position must have finite x and y"))?,
        );
    }
    if let Some(value) = options.get_item("modifiers")? {
        #[cfg(feature = "locator-click-modifiers")]
        {
            known += 1;
            let names = value
                .cast::<PyList>()
                .map_err(|_| invalid("Page.click modifiers must be a list"))?;
            parsed.modifiers = Some(click_modifiers(names)?);
        }
        #[cfg(not(feature = "locator-click-modifiers"))]
        {
            let _ = value;
            return Err(binding_error(HostError::UnsupportedOperation(
                "Page.click modifiers require the locator-click-modifiers profile".into(),
            )));
        }
    }
    if known != options.len() {
        return Err(invalid("Page.click contains an unknown native option"));
    }
    Ok(parsed)
}

#[cfg(feature = "element-handle-click")]
fn element_click_options(options: &Bound<'_, PyDict>) -> PyResult<ClickOptionsV2> {
    let invalid = |message: &str| binding_error(HostError::InvalidArgument(message.into()));
    if options.len() > 9 {
        return Err(invalid(
            "ElementHandle.click accepts at most nine native options",
        ));
    }
    let base = PyDict::new(options.py());
    let mut delay = None;
    let mut no_wait_after = None;
    let mut trial = None;
    let mut steps = None;
    for (key, value) in options.iter() {
        let key = key
            .extract::<String>()
            .map_err(|_| invalid("ElementHandle.click option names must be strings"))?;
        match key.as_str() {
            "button" | "click_count" | "force" | "position" | "modifiers" => {
                base.set_item(key, value)?;
            }
            "delay" => {
                if value.is_instance_of::<PyBool>()
                    || !(value.is_instance_of::<PyInt>() || value.is_instance_of::<PyFloat>())
                {
                    return Err(invalid(
                        "ElementHandle.click delay must be nonnegative milliseconds",
                    ));
                }
                let milliseconds = value.extract::<f64>().map_err(|_| {
                    invalid("ElementHandle.click delay must be nonnegative milliseconds")
                })?;
                if !milliseconds.is_finite() || !(0.0..=f64::from(u32::MAX)).contains(&milliseconds)
                {
                    return Err(invalid(
                        "ElementHandle.click delay must be nonnegative milliseconds",
                    ));
                }
                delay = Some(std::time::Duration::from_secs_f64(milliseconds / 1000.0));
            }
            "no_wait_after" => {
                if !value.is_instance_of::<PyBool>() {
                    return Err(invalid(
                        "ElementHandle.click no_wait_after must be a boolean",
                    ));
                }
                no_wait_after = Some(value.extract::<bool>()?);
            }
            "trial" => {
                if !value.is_instance_of::<PyBool>() {
                    return Err(invalid("ElementHandle.click trial must be a boolean"));
                }
                trial = Some(value.extract::<bool>()?);
            }
            "steps" => {
                steps = locator_click_options(Some(&value), None)?.steps;
            }
            _ => {
                return Err(invalid(
                    "ElementHandle.click contains an unknown native option",
                ));
            }
        }
    }
    let mut parsed = page_click_options(&base)?;
    parsed.delay = delay;
    parsed.no_wait_after = no_wait_after;
    parsed.trial = trial;
    parsed.steps = steps;
    Ok(parsed)
}

#[cfg(any(
    feature = "persistent-context",
    feature = "fresh-launch",
    feature = "owned-process-startup",
    feature = "playwright-server-initialize",
    feature = "playwright-server-connect-session"
))]
fn backend_host_error(error: impl std::fmt::Display) -> HostError {
    let mut message = error.to_string();
    if message.len() > 4096 {
        let mut boundary = 4096;
        while !message.is_char_boundary(boundary) {
            boundary -= 1;
        }
        message.truncate(boundary);
    }
    HostError::Backend(message)
}

#[cfg(feature = "playwright-server-initialize")]
fn playwright_server_initialize_host_error(
    error: pwright_playwright_server_initialize::InitializeError,
) -> HostError {
    // Upstream BrowserType.connect reports a failed WebSocket handshake as a
    // WebSocket error. Keep the transport's typed error until this public API
    // boundary so a refused connection is not mistaken for a remote Close.
    match error {
        pwright_playwright_server_initialize::InitializeError::WebSocket(error) => {
            backend_host_error(format!("WebSocket error: {error}"))
        }
        other => backend_host_error(other),
    }
}

#[cfg(feature = "playwright-server-connect-session")]
fn playwright_server_connect_host_error(error: ConnectError) -> HostError {
    match error {
        ConnectError::WebSocket(error) => backend_host_error(format!("WebSocket error: {error}")),
        other => backend_host_error(other),
    }
}

#[cfg(feature = "playwright-server-connect-session")]
fn playwright_server_operation_host_error(error: BrowserOperationError) -> HostError {
    match error {
        BrowserOperationError::Disconnected => HostError::TargetClosed,
        other => backend_host_error(other),
    }
}

#[cfg(feature = "playwright-server-close-notices")]
fn playwright_server_notice_host_error(error: SupervisorError) -> HostError {
    match error {
        SupervisorError::Terminal(_) | SupervisorError::ActorStopped => HostError::TargetClosed,
        other => backend_host_error(format!("{other:?}")),
    }
}

#[cfg(feature = "playwright-server-connect-session")]
fn playwright_server_wall_time() -> Result<u64, HostError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| HostError::Backend("system clock precedes the Unix epoch".into()))?
        .as_millis()
        .try_into()
        .map_err(|_| {
            HostError::Backend("system clock milliseconds exceed the supported range".into())
        })
}

fn bounded_string(value: &str, maximum: usize, field: &str) -> PyResult<String> {
    if value.len() > maximum {
        return Err(binding_error(HostError::InvalidArgument(format!(
            "{field} exceeds {maximum} UTF-8 bytes"
        ))));
    }
    Ok(value.to_owned())
}

#[cfg(feature = "cdp-handshake-headers")]
fn bounded_cdp_handshake_headers(values: &Bound<'_, PyList>) -> PyResult<Vec<(String, String)>> {
    if values.len() > CDP_HANDSHAKE_HEADER_MAX_COUNT {
        return Err(binding_error(HostError::ResourceExhausted(
            "CDP handshake header count exceeds 32".into(),
        )));
    }
    let mut total_bytes = 0usize;
    let mut headers = Vec::with_capacity(values.len());
    for item in values.iter() {
        let pair = item.cast::<PyTuple>()?;
        if pair.len() != 2 {
            return Err(binding_error(HostError::InvalidArgument(
                "CDP handshake header entry must have a name and value".into(),
            )));
        }
        let name_item = pair.get_item(0)?;
        let value_item = pair.get_item(1)?;
        let name = name_item.cast::<PyString>()?;
        let value = value_item.cast::<PyString>()?;
        let name = name.to_cow()?;
        let value = value.to_cow()?;
        if name.len() > CDP_HANDSHAKE_HEADER_MAX_NAME_BYTES {
            return Err(binding_error(HostError::ResourceExhausted(
                "CDP handshake header name exceeds 256 bytes".into(),
            )));
        }
        if value.len() > CDP_HANDSHAKE_HEADER_MAX_VALUE_BYTES {
            return Err(binding_error(HostError::ResourceExhausted(
                "CDP handshake header value exceeds 4096 bytes".into(),
            )));
        }
        total_bytes = total_bytes
            .saturating_add(name.len())
            .saturating_add(value.len());
        if total_bytes > CDP_HANDSHAKE_HEADER_MAX_BATCH_BYTES {
            return Err(binding_error(HostError::ResourceExhausted(
                "CDP handshake headers exceed 8192 bytes".into(),
            )));
        }
        headers.push((name.into_owned(), value.into_owned()));
    }
    Ok(headers)
}

#[cfg(any(feature = "locator-click-modifiers", feature = "touchscreen-input"))]
fn click_modifier(name: &str) -> Result<KeyboardModifier, HostError> {
    match name {
        "Alt" => Ok(KeyboardModifier::Alt),
        "Control" => Ok(KeyboardModifier::Control),
        "ControlOrMeta" => Ok(KeyboardModifier::ControlOrMeta),
        "Meta" => Ok(KeyboardModifier::Meta),
        "Shift" => Ok(KeyboardModifier::Shift),
        _ => Err(HostError::InvalidArgument(
            "unknown keyboard modifier".into(),
        )),
    }
}

#[cfg(any(feature = "locator-click-modifiers", feature = "touchscreen-input"))]
fn click_modifiers(names: &Bound<'_, PyList>) -> PyResult<KeyboardModifiers> {
    if names.len() > 5 {
        return Err(binding_error(HostError::InvalidArgument(
            "at most five keyboard modifiers are supported".into(),
        )));
    }
    let mut modifiers = KeyboardModifiers::new();
    for value in names.iter() {
        let value = value.cast::<PyString>().map_err(|_| {
            binding_error(HostError::InvalidArgument(
                "keyboard modifiers must be strings".into(),
            ))
        })?;
        if value.len()? > 16 {
            return Err(binding_error(HostError::InvalidArgument(
                "keyboard modifier exceeds 16 UTF-8 bytes".into(),
            )));
        }
        let name = value.to_cow()?;
        if name.len() > 16 {
            return Err(binding_error(HostError::InvalidArgument(
                "keyboard modifier exceeds 16 UTF-8 bytes".into(),
            )));
        }
        modifiers = modifiers.with(click_modifier(&name).map_err(binding_error)?);
    }
    Ok(modifiers)
}

#[cfg(feature = "touchscreen-input")]
fn python_tap_options(options: &Bound<'_, PyDict>) -> PyResult<SelectorTapOptions> {
    let invalid = |message: &str| binding_error(HostError::InvalidArgument(message.into()));
    if options.len() > 5 {
        return Err(invalid("tap accepts at most five native options"));
    }
    let mut parsed = SelectorTapOptions::new();
    for (key, value) in options.iter() {
        let key = key
            .extract::<String>()
            .map_err(|_| invalid("tap option names must be strings"))?;
        match key.as_str() {
            "force" | "no_wait_after" | "trial" => {
                if !value.is_instance_of::<PyBool>() {
                    return Err(invalid("tap boolean option must be a boolean"));
                }
                let enabled = value.extract::<bool>()?;
                match key.as_str() {
                    "force" => parsed.force = Some(enabled),
                    "no_wait_after" => parsed.no_wait_after = Some(enabled),
                    _ => parsed.trial = Some(enabled),
                }
            }
            "modifiers" => {
                let names = value
                    .cast::<PyList>()
                    .map_err(|_| invalid("tap modifiers must be a list"))?;
                parsed.modifiers = Some(click_modifiers(names)?);
            }
            "position" => {
                let point = value
                    .cast::<PyDict>()
                    .map_err(|_| invalid("tap position must have finite x and y"))?;
                if point.len() != 2 {
                    return Err(invalid("tap position must have finite x and y"));
                }
                let coordinate = |axis: &str| -> PyResult<f64> {
                    let value = point
                        .get_item(axis)?
                        .ok_or_else(|| invalid("tap position must have finite x and y"))?;
                    if value.is_instance_of::<PyBool>()
                        || !(value.is_instance_of::<PyInt>() || value.is_instance_of::<PyFloat>())
                    {
                        return Err(invalid("tap position must have finite x and y"));
                    }
                    value
                        .extract::<f64>()
                        .map_err(|_| invalid("tap position must have finite x and y"))
                };
                parsed.position = Some(
                    Position::new(coordinate("x")?, coordinate("y")?)
                        .map_err(|_| invalid("tap position must have finite x and y"))?,
                );
            }
            _ => return Err(invalid("tap contains an unknown native option")),
        }
    }
    Ok(parsed)
}

#[cfg(feature = "context-cookies")]
fn context_cookie_urls(value: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    let values = value.cast::<PyList>().map_err(|_| {
        binding_error(HostError::InvalidArgument(
            "BrowserContext.cookies urls must be a list".into(),
        ))
    })?;
    if values.len() > pwright_node_host::context_cookies::MAX_COOKIE_URLS {
        return Err(binding_error(HostError::ResourceExhausted(format!(
            "cookie URL count exceeds {}",
            pwright_node_host::context_cookies::MAX_COOKIE_URLS
        ))));
    }
    let mut output = Vec::new();
    output.try_reserve_exact(values.len()).map_err(|_| {
        binding_error(HostError::ResourceExhausted(
            "cookie URL allocation failed".into(),
        ))
    })?;
    let mut total = 0;
    for value in values.iter() {
        let value = value.cast::<PyString>().map_err(|_| {
            binding_error(HostError::InvalidArgument(
                "BrowserContext.cookies urls must contain strings".into(),
            ))
        })?;
        let value = value.to_cow()?;
        bound_cookie_input(
            &mut total,
            &value,
            pwright_node_host::context_cookies::MAX_COOKIE_URL_BYTES,
            pwright_node_host::context_cookies::MAX_COOKIE_URL_BATCH_BYTES,
            "cookie URL",
        )?;
        output.push(value.into_owned());
    }
    Ok(output)
}

#[cfg(feature = "context-cookies")]
fn context_cookie_inputs(value: &Bound<'_, PyAny>) -> PyResult<Vec<BrowserContextCookieInput>> {
    let values = value.cast::<PyList>().map_err(|_| {
        binding_error(HostError::InvalidArgument(
            "BrowserContext.add_cookies cookies must be a list".into(),
        ))
    })?;
    if values.len() > pwright_node_host::context_cookies::MAX_COOKIES {
        return Err(binding_error(HostError::ResourceExhausted(format!(
            "cookie input count exceeds {}",
            pwright_node_host::context_cookies::MAX_COOKIES
        ))));
    }
    let mut output = Vec::new();
    output.try_reserve_exact(values.len()).map_err(|_| {
        binding_error(HostError::ResourceExhausted(
            "cookie input allocation failed".into(),
        ))
    })?;
    let mut total = 0;
    for value in values.iter() {
        let value = value
            .cast::<PyTuple>()
            .map_err(|_| invalid_cookie_tuple())?;
        if value.len() != 10 {
            return Err(invalid_cookie_tuple());
        }
        output.push(BrowserContextCookieInput {
            name: cookie_input_string(value, 0, "cookie name", &mut total)?,
            value: cookie_input_string(value, 1, "cookie value", &mut total)?,
            url: optional_cookie_input_string(
                value,
                2,
                "cookie URL",
                pwright_node_host::context_cookies::MAX_COOKIE_URL_BYTES,
                &mut total,
            )?,
            domain: optional_cookie_input_string(
                value,
                3,
                "cookie domain",
                pwright_node_host::context_cookies::MAX_COOKIE_FIELD_BYTES,
                &mut total,
            )?,
            path: optional_cookie_input_string(
                value,
                4,
                "cookie path",
                pwright_node_host::context_cookies::MAX_COOKIE_FIELD_BYTES,
                &mut total,
            )?,
            expires: value
                .get_item(5)?
                .extract()
                .map_err(|_| invalid_cookie_tuple())?,
            http_only: value
                .get_item(6)?
                .extract()
                .map_err(|_| invalid_cookie_tuple())?,
            secure: value
                .get_item(7)?
                .extract()
                .map_err(|_| invalid_cookie_tuple())?,
            same_site: optional_cookie_input_string(
                value,
                8,
                "cookie sameSite",
                pwright_node_host::context_cookies::MAX_COOKIE_FIELD_BYTES,
                &mut total,
            )?,
            partition_key: optional_cookie_input_string(
                value,
                9,
                "cookie partition key",
                pwright_node_host::context_cookies::MAX_COOKIE_URL_BYTES,
                &mut total,
            )?,
            partition_has_cross_site_ancestor: None,
        });
    }
    Ok(output)
}

#[cfg(feature = "context-cookies")]
fn invalid_cookie_tuple() -> PyErr {
    binding_error(HostError::InvalidArgument(
        "BrowserContext.add_cookies received an invalid cookie tuple".into(),
    ))
}

#[cfg(feature = "context-cookies")]
fn bound_cookie_input(
    total: &mut usize,
    value: &str,
    field_maximum: usize,
    batch_maximum: usize,
    field: &str,
) -> PyResult<()> {
    if value.len() > field_maximum {
        return Err(binding_error(HostError::ResourceExhausted(format!(
            "{field} exceeds {field_maximum} UTF-8 bytes"
        ))));
    }
    *total = total.checked_add(value.len()).ok_or_else(|| {
        binding_error(HostError::ResourceExhausted(
            "cookie input size overflow".into(),
        ))
    })?;
    if *total > batch_maximum {
        return Err(binding_error(HostError::ResourceExhausted(format!(
            "cookie input batch exceeds {batch_maximum} bytes"
        ))));
    }
    Ok(())
}

#[cfg(feature = "context-cookies")]
fn cookie_input_string(
    value: &Bound<'_, PyTuple>,
    index: usize,
    field: &str,
    total: &mut usize,
) -> PyResult<String> {
    let item = value.get_item(index)?;
    let value = item
        .cast::<PyString>()
        .map_err(|_| invalid_cookie_tuple())?;
    let value = value.to_cow()?;
    bound_cookie_input(
        total,
        &value,
        pwright_node_host::context_cookies::MAX_COOKIE_FIELD_BYTES,
        pwright_node_host::context_cookies::MAX_COOKIE_BATCH_BYTES,
        field,
    )?;
    Ok(value.into_owned())
}

#[cfg(feature = "context-cookies")]
fn optional_cookie_input_string(
    value: &Bound<'_, PyTuple>,
    index: usize,
    field: &str,
    field_maximum: usize,
    total: &mut usize,
) -> PyResult<Option<String>> {
    let item = value.get_item(index)?;
    if item.is_none() {
        return Ok(None);
    }
    let item = item
        .cast::<PyString>()
        .map_err(|_| invalid_cookie_tuple())?;
    let item = item.to_cow()?;
    bound_cookie_input(
        total,
        &item,
        field_maximum,
        pwright_node_host::context_cookies::MAX_COOKIE_BATCH_BYTES,
        field,
    )?;
    Ok(Some(item.into_owned()))
}

#[cfg(feature = "context-cookies")]
fn cookie_matcher(
    value: Option<&str>,
    regex_flags: Option<&str>,
) -> Result<Option<CookieStringMatcher>, HostError> {
    match (value, regex_flags) {
        (None, None) => Ok(None),
        (Some(value), None) => CookieStringMatcher::exact(value.to_owned()).map(Some),
        (Some(value), Some(flags)) => CookieStringMatcher::regex(value.to_owned(), flags).map(Some),
        (None, Some(_)) => Err(HostError::InvalidArgument(
            "cookie clear regex flags require a pattern".into(),
        )),
    }
}

#[cfg(feature = "context-storage-state")]
fn storage_state_object(
    value: serde_json::Value,
    field: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, HostError> {
    match value {
        serde_json::Value::Object(value) => Ok(value),
        _ => Err(HostError::InvalidArgument(format!(
            "storage_state {field} must be an object"
        ))),
    }
}

#[cfg(feature = "context-storage-state")]
fn storage_state_array(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<Vec<serde_json::Value>, HostError> {
    match object.remove(key) {
        None => Ok(Vec::new()),
        Some(serde_json::Value::Array(values)) => Ok(values),
        _ => Err(HostError::InvalidArgument(format!(
            "storage_state {key} must be an array"
        ))),
    }
}

#[cfg(feature = "context-storage-state")]
fn storage_state_string(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<String, HostError> {
    match object.remove(key) {
        Some(serde_json::Value::String(value)) => Ok(value),
        _ => Err(HostError::InvalidArgument(format!(
            "storage_state {key} must be a string"
        ))),
    }
}

#[cfg(feature = "context-storage-state")]
fn storage_state_optional_string(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<Option<String>, HostError> {
    match object.remove(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(value)) => Ok(Some(value)),
        _ => Err(HostError::InvalidArgument(format!(
            "storage_state {key} must be a string"
        ))),
    }
}

#[cfg(feature = "context-storage-state")]
fn storage_state_bool(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<bool, HostError> {
    match object.remove(key) {
        Some(serde_json::Value::Bool(value)) => Ok(value),
        _ => Err(HostError::InvalidArgument(format!(
            "storage_state {key} must be a boolean"
        ))),
    }
}

#[cfg(feature = "context-storage-state")]
fn storage_state_optional_bool(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<Option<bool>, HostError> {
    match object.remove(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Bool(value)) => Ok(Some(value)),
        _ => Err(HostError::InvalidArgument(format!(
            "storage_state {key} must be a boolean"
        ))),
    }
}

#[cfg(feature = "context-storage-state")]
fn parse_context_storage_state(value: &str) -> Result<ContextStorageState, HostError> {
    if value.len() > MAX_STORAGE_STATE_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "storage_state exceeds {MAX_STORAGE_STATE_BYTES} UTF-8 bytes"
        )));
    }
    let parsed: serde_json::Value = serde_json::from_str(value).map_err(|error| {
        HostError::InvalidArgument(format!("storage_state is not valid JSON: {error}"))
    })?;
    let mut state = storage_state_object(parsed, "root")?;
    let cookie_values = storage_state_array(&mut state, "cookies")?;
    if cookie_values.len() > pwright_node_host::context_cookies::MAX_COOKIES {
        return Err(HostError::ResourceExhausted(
            "storage_state cookie count exceeds the installed limit".into(),
        ));
    }
    let origin_values = storage_state_array(&mut state, "origins")?;
    if origin_values.len() > MAX_STORAGE_ORIGINS {
        return Err(HostError::ResourceExhausted(format!(
            "storage_state origin count exceeds {MAX_STORAGE_ORIGINS}"
        )));
    }

    let mut cookies = Vec::new();
    cookies
        .try_reserve_exact(cookie_values.len())
        .map_err(|_| {
            HostError::ResourceExhausted("storage_state cookie allocation failed".into())
        })?;
    for value in cookie_values {
        let mut cookie = storage_state_object(value, "cookie")?;
        let expires = match cookie.remove("expires") {
            Some(serde_json::Value::Number(value)) => {
                value.as_f64().filter(|value| value.is_finite())
            }
            _ => None,
        }
        .ok_or_else(|| {
            HostError::InvalidArgument(
                "storage_state cookie expires must be a finite number".into(),
            )
        })?;
        cookies.push(BrowserContextCookieInput {
            name: storage_state_string(&mut cookie, "name")?,
            value: storage_state_string(&mut cookie, "value")?,
            url: None,
            domain: Some(storage_state_string(&mut cookie, "domain")?),
            path: Some(storage_state_string(&mut cookie, "path")?),
            expires: Some(expires),
            http_only: Some(storage_state_bool(&mut cookie, "httpOnly")?),
            secure: Some(storage_state_bool(&mut cookie, "secure")?),
            same_site: Some(storage_state_string(&mut cookie, "sameSite")?),
            partition_key: storage_state_optional_string(&mut cookie, "partitionKey")?,
            partition_has_cross_site_ancestor: storage_state_optional_bool(
                &mut cookie,
                "_crHasCrossSiteAncestor",
            )?,
        });
    }

    let mut origins = Vec::new();
    origins
        .try_reserve_exact(origin_values.len())
        .map_err(|_| {
            HostError::ResourceExhausted("storage_state origin allocation failed".into())
        })?;
    let mut total_items = 0usize;
    for value in origin_values {
        let mut origin = storage_state_object(value, "origin")?;
        let origin_url = storage_state_string(&mut origin, "origin")?;
        let local_values = storage_state_array(&mut origin, "localStorage")?;
        let indexed_db = match origin.remove("indexedDB") {
            None => None,
            Some(serde_json::Value::Array(values)) => Some(values),
            _ => {
                return Err(HostError::InvalidArgument(
                    "storage_state indexedDB must be an array".into(),
                ));
            }
        };
        total_items = total_items
            .checked_add(local_values.len())
            .and_then(|total| total.checked_add(indexed_db.as_ref().map_or(0, Vec::len)))
            .ok_or_else(|| {
                HostError::ResourceExhausted("storage_state item count overflow".into())
            })?;
        if total_items > MAX_STORAGE_ITEMS {
            return Err(HostError::ResourceExhausted(format!(
                "storage_state item count exceeds {MAX_STORAGE_ITEMS}"
            )));
        }
        let mut local_storage = Vec::new();
        local_storage
            .try_reserve_exact(local_values.len())
            .map_err(|_| {
                HostError::ResourceExhausted("storage_state item allocation failed".into())
            })?;
        for item in local_values {
            let mut item = storage_state_object(item, "localStorage item")?;
            local_storage.push(ContextStorageItem {
                name: storage_state_string(&mut item, "name")?,
                value: storage_state_string(&mut item, "value")?,
            });
        }
        origins.push(ContextStorageOrigin {
            origin: origin_url,
            local_storage,
            indexed_db,
        });
    }
    Ok(ContextStorageState { cookies, origins })
}

#[cfg(feature = "context-storage-state")]
fn context_storage_state_to_python(
    py: Python<'_>,
    state: ContextStorageStateSnapshot,
    indexed_db: bool,
) -> PyResult<Py<PyAny>> {
    let result = PyDict::new(py);
    let cookies = PyList::empty(py);
    for cookie in state.cookies {
        let value = PyDict::new(py);
        value.set_item("name", cookie.name)?;
        value.set_item("value", cookie.value)?;
        value.set_item("domain", cookie.domain)?;
        value.set_item("path", cookie.path)?;
        value.set_item("expires", cookie.expires)?;
        value.set_item("httpOnly", cookie.http_only)?;
        value.set_item("secure", cookie.secure)?;
        value.set_item("sameSite", cookie.same_site)?;
        if let Some(partition_key) = cookie.partition_key {
            value.set_item("partitionKey", partition_key)?;
        }
        if let Some(ancestor) = cookie.partition_has_cross_site_ancestor {
            value.set_item("_crHasCrossSiteAncestor", ancestor)?;
        }
        cookies.append(value)?;
    }
    result.set_item("cookies", cookies)?;

    let origins = PyList::empty(py);
    for origin in state.origins {
        let value = PyDict::new(py);
        value.set_item("origin", origin.origin)?;
        let local_storage = PyList::empty(py);
        for item in origin.local_storage {
            let item_value = PyDict::new(py);
            item_value.set_item("name", item.name)?;
            item_value.set_item("value", item.value)?;
            local_storage.append(item_value)?;
        }
        value.set_item("localStorage", local_storage)?;
        if indexed_db && let Some(database_values) = origin.indexed_db {
            let databases = PyList::empty(py);
            for database in database_values {
                databases.append(json_value_to_python(py, &database)?)?;
            }
            value.set_item("indexedDB", databases)?;
        }
        origins.append(value)?;
    }
    result.set_item("origins", origins)?;
    Ok(result.into_any().unbind())
}

#[cfg(any(feature = "persistent-context", feature = "fresh-launch"))]
fn chromium_channel(value: Option<&str>) -> Result<Option<ChromiumChannel>, HostError> {
    value
        .map(|value| match value {
            "chrome" => Ok(ChromiumChannel::Chrome),
            "chrome-beta" => Ok(ChromiumChannel::ChromeBeta),
            "chrome-dev" => Ok(ChromiumChannel::ChromeDev),
            "chrome-canary" => Ok(ChromiumChannel::ChromeCanary),
            "chromium" => Ok(ChromiumChannel::Chromium),
            "msedge" => Ok(ChromiumChannel::Edge),
            _ => Err(HostError::InvalidArgument(format!(
                "unsupported Chromium channel {value:?}"
            ))),
        })
        .transpose()
}

#[cfg(any(feature = "persistent-context", feature = "fresh-launch"))]
fn chromium_executable(
    executable_path: Option<String>,
    channel: Option<String>,
) -> Result<ChromiumExecutable, HostError> {
    let channel = chromium_channel(channel.as_deref())?;
    match executable_path {
        Some(path) => ChromiumExecutable::from_path(
            PathBuf::from(path),
            channel.unwrap_or(ChromiumChannel::Custom),
        )
        .map_err(backend_host_error),
        None => discover_system_chromium()
            .into_iter()
            .find(|candidate| channel.is_none_or(|channel| candidate.channel() == channel))
            .ok_or_else(|| {
                HostError::Backend(match channel {
                    Some(channel) => {
                        format!("no installed Chromium executable was found for channel {channel}")
                    }
                    None => "no installed Chromium executable was found".into(),
                })
            }),
    }
}

#[cfg(feature = "persistent-context")]
fn persistent_launch_inputs(
    user_data_dir: String,
    executable_path: Option<String>,
    channel: Option<String>,
) -> Result<(ChromiumExecutable, ChromiumPersistentProfile), HostError> {
    let executable = chromium_executable(executable_path, channel)?;
    let profile = ChromiumPersistentProfile::open_or_initialize(PathBuf::from(user_data_dir))
        .map_err(backend_host_error)?;
    Ok((executable, profile))
}

#[cfg(feature = "page-routes")]
fn parse_portable_u64(value: &str, field: &str) -> PyResult<u64> {
    bounded_string(value, MAX_OBJECT_ID_BYTES, field)?
        .parse::<u64>()
        .ok()
        .filter(|value| *value != 0)
        .ok_or_else(|| binding_error(HostError::InvalidArgument(format!("invalid {field}"))))
}

#[pyfunction]
fn native_abi_version() -> u32 {
    NATIVE_ABI_VERSION
}

#[pyfunction]
fn package_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pyfunction]
fn device_descriptors_json() -> &'static str {
    #[cfg(feature = "device-emulation")]
    {
        pwright_node_host::device_descriptors_json()
    }
    #[cfg(not(feature = "device-emulation"))]
    {
        "{}"
    }
}

#[pyfunction]
fn selected_features() -> Vec<&'static str> {
    let mut features = Vec::new();
    if cfg!(feature = "browser-registry") {
        features.push("browser-registry");
    }
    if cfg!(feature = "fresh-launch") {
        features.push("fresh-launch");
    }
    if cfg!(feature = "owned-process-startup") {
        features.push("owned-process-startup");
    }
    if cfg!(feature = "playwright-server-initialize") {
        features.push("playwright-server-initialize");
    }
    if cfg!(feature = "playwright-server-connect-session") {
        features.push("playwright-server-connect-session");
    }
    if cfg!(feature = "playwright-server-close-notices") {
        features.push("playwright-server-close-notices");
    }
    if cfg!(feature = "playwright-server-frame-title") {
        features.push("playwright-server-frame-title");
    }
    if cfg!(feature = "persistent-context") {
        features.push("persistent-context");
    }
    if cfg!(feature = "persistent-context-locale") {
        features.push("persistent-context-locale");
    }
    if cfg!(feature = "persistent-context-timezone") {
        features.push("persistent-context-timezone");
    }
    if cfg!(feature = "persistent-context-offline") {
        features.push("persistent-context-offline");
    }
    if cfg!(feature = "persistent-context-geolocation") {
        features.push("persistent-context-geolocation");
    }
    if cfg!(feature = "persistent-context-geolocation-permission") {
        features.push("persistent-context-geolocation-permission");
    }
    if cfg!(feature = "persistent-context-http-credentials") {
        features.push("persistent-context-http-credentials");
    }
    if cfg!(feature = "persistent-context-launch-arguments") {
        features.push("persistent-context-launch-arguments");
    }
    if cfg!(feature = "persistent-context-ignore-https-errors") {
        features.push("persistent-context-ignore-https-errors");
    }
    if cfg!(feature = "persistent-context-device-emulation") {
        features.push("persistent-context-device-emulation");
    }
    if cfg!(feature = "persistent-context-emulate-media") {
        features.push("persistent-context-emulate-media");
    }
    if cfg!(feature = "persistent-context-extra-headers") {
        features.push("persistent-context-extra-headers");
    }
    if cfg!(feature = "persistent-context-bypass-csp") {
        features.push("persistent-context-bypass-csp");
    }
    if cfg!(feature = "persistent-context-javascript-disabled") {
        features.push("persistent-context-javascript-disabled");
    }
    if cfg!(feature = "context-cookies") {
        features.push("context-cookies");
    }
    if cfg!(feature = "context-strict-selectors") {
        features.push("context-strict-selectors");
    }
    if cfg!(feature = "context-storage-state") {
        features.push("context-storage-state");
    }
    if cfg!(feature = "cdp-session") {
        features.push("cdp-session");
    }
    if cfg!(feature = "cdp-handshake-headers") {
        features.push("cdp-handshake-headers");
    }
    if cfg!(feature = "cdp-ws-close-reason") {
        features.push("cdp-ws-close-reason");
    }
    if cfg!(feature = "cdp-ws-invalid-frame") {
        features.push("cdp-ws-invalid-frame");
    }
    if cfg!(feature = "handles") {
        features.push("handles");
    }
    if cfg!(feature = "element-handle-dom") {
        features.push("element-handle-dom");
    }
    if cfg!(feature = "element-handle-dom-reads") {
        features.push("element-handle-dom-reads");
    }
    if cfg!(feature = "element-handle-scroll-into-view") {
        features.push("element-handle-scroll-into-view");
    }
    if cfg!(feature = "element-handle-select-text") {
        features.push("element-handle-select-text");
    }
    if cfg!(feature = "element-handle-state-predicates") {
        features.push("element-handle-state-predicates");
    }
    if cfg!(feature = "element-handle-state-wait") {
        features.push("element-handle-state-wait");
    }
    if cfg!(feature = "element-handle-click") {
        features.push("element-handle-click");
    }
    if cfg!(feature = "element-handle-tap") {
        features.push("element-handle-tap");
    }
    if cfg!(feature = "element-handle-file-input") {
        features.push("element-handle-file-input");
    }
    if cfg!(feature = "page-bindings") {
        features.push("page-bindings");
    }
    if cfg!(feature = "context-bindings") {
        features.push("context-bindings");
    }
    if cfg!(feature = "page-events") {
        features.push("page-events");
    }
    if cfg!(feature = "frame-events") {
        features.push("frame-events");
    }
    if cfg!(feature = "console-events") {
        features.push("console-events");
    }
    if cfg!(feature = "console-argument-handles") {
        features.push("console-argument-handles");
    }
    if cfg!(feature = "page-dialogs") {
        features.push("page-dialogs");
    }
    if cfg!(feature = "file-chooser-events") {
        features.push("file-chooser-events");
    }
    if cfg!(feature = "file-chooser-native-paths") {
        features.push("file-chooser-native-paths");
    }
    if cfg!(feature = "page-errors") {
        features.push("page-errors");
    }
    if cfg!(feature = "console-history") {
        features.push("console-history");
    }
    if cfg!(feature = "request-history") {
        features.push("request-history");
    }
    if cfg!(feature = "page-routes") {
        features.push("page-routes");
    }
    if cfg!(feature = "route-redirects") {
        features.push("route-redirects");
    }
    if cfg!(feature = "route-fulfill-path") {
        features.push("route-fulfill-path");
    }
    if cfg!(feature = "route-fulfill-api-response") {
        features.push("route-fulfill-api-response");
    }
    if cfg!(feature = "route-fetch") {
        features.push("route-fetch");
    }
    if cfg!(feature = "har-replay") {
        features.push("har-replay");
    }
    if cfg!(feature = "wait-for-function") {
        features.push("wait-for-function");
    }
    if cfg!(feature = "wait-for-timeout") {
        features.push("wait-for-timeout");
    }
    if cfg!(feature = "file-input") {
        features.push("file-input");
    }
    if cfg!(feature = "file-input-native-paths") {
        features.push("file-input-native-paths");
    }
    if cfg!(feature = "mouse") {
        features.push("mouse");
    }
    if cfg!(feature = "keyboard") {
        features.push("keyboard");
    }
    if cfg!(feature = "locator-bounding-box") {
        features.push("locator-bounding-box");
    }
    if cfg!(feature = "locator-highlight") {
        features.push("locator-highlight");
    }
    if cfg!(feature = "locator-screenshot") {
        features.push("locator-screenshot");
    }
    if cfg!(feature = "locator-dom-actions") {
        features.push("locator-dom-actions");
    }
    if cfg!(feature = "locator-click-modifiers") {
        features.push("locator-click-modifiers");
    }
    if cfg!(feature = "page-select-option") {
        features.push("page-select-option");
    }
    if cfg!(feature = "concurrent-close") {
        features.push("concurrent-close");
    }
    if cfg!(feature = "extra-headers") {
        features.push("extra-headers");
    }
    if cfg!(feature = "frame-lifecycle") {
        features.push("frame-lifecycle");
    }
    if cfg!(feature = "frame-navigation") {
        features.push("frame-navigation");
    }
    if cfg!(feature = "navigation-url-match") {
        features.push("navigation-url-match");
    }
    if cfg!(feature = "history-navigation") {
        features.push("history-navigation");
    }
    if cfg!(feature = "response-body") {
        features.push("response-body");
    }
    if cfg!(feature = "wait-for-url") {
        features.push("wait-for-url");
    }
    if cfg!(feature = "set-content") {
        features.push("set-content");
    }
    if cfg!(feature = "selector-registration") {
        features.push("selector-registration");
    }
    if cfg!(feature = "layout-selectors") {
        features.push("layout-selectors");
    }
    if cfg!(feature = "emulate-media") {
        features.push("emulate-media");
    }
    if cfg!(feature = "context-forced-colors") {
        features.push("context-forced-colors");
    }
    if cfg!(feature = "device-emulation") {
        features.push("device-emulation");
    }
    if cfg!(feature = "touchscreen-input") {
        features.push("touchscreen-input");
    }
    if cfg!(feature = "context-locale") {
        features.push("context-locale");
    }
    if cfg!(feature = "context-timezone") {
        features.push("context-timezone");
    }
    if cfg!(feature = "context-offline") {
        features.push("context-offline");
    }
    if cfg!(feature = "context-geolocation") {
        features.push("context-geolocation");
    }
    if cfg!(feature = "context-geolocation-permission") {
        features.push("context-geolocation-permission");
    }
    if cfg!(feature = "context-http-credentials") {
        features.push("context-http-credentials");
    }
    if cfg!(feature = "ignore-https-errors") {
        features.push("ignore-https-errors");
    }
    if cfg!(feature = "bypass-csp") {
        features.push("bypass-csp");
    }
    if cfg!(feature = "javascript-disabled") {
        features.push("javascript-disabled");
    }
    if cfg!(feature = "page-viewport") {
        features.push("page-viewport");
    }
    if cfg!(feature = "timeout-settings") {
        features.push("timeout-settings");
    }
    if cfg!(feature = "page-pause") {
        features.push("page-pause");
    }
    if cfg!(feature = "request-gc") {
        features.push("request-gc");
    }
    if cfg!(feature = "page-pdf") {
        features.push("page-pdf");
    }
    if cfg!(feature = "network-events") {
        features.push("network-events");
    }
    if cfg!(feature = "context-network-events") {
        features.push("context-network-events");
    }
    if cfg!(feature = "request-frame") {
        features.push("request-frame");
    }
    if cfg!(feature = "request-timing") {
        features.push("request-timing");
    }
    if cfg!(feature = "request-post-data-json") {
        features.push("request-post-data-json");
    }
    if cfg!(feature = "request-events") {
        features.push("request-events");
    }
    if cfg!(feature = "response-events") {
        features.push("response-events");
    }
    if cfg!(feature = "response-finished") {
        features.push("response-finished");
    }
    if cfg!(feature = "navigation-response-finished") {
        features.push("navigation-response-finished");
    }
    if cfg!(feature = "response-security-details") {
        features.push("response-security-details");
    }
    if cfg!(feature = "download-events") {
        features.push("download-events");
    }
    if cfg!(feature = "worker-events") {
        features.push("worker-events");
    }
    if cfg!(feature = "worker-handle-evaluation") {
        features.push("worker-handle-evaluation");
    }
    if cfg!(feature = "worker-console-events") {
        features.push("worker-console-events");
    }
    if cfg!(feature = "worker-console-argument-handles") {
        features.push("worker-console-argument-handles");
    }
    if cfg!(feature = "worker-error-events") {
        features.push("worker-error-events");
    }
    if cfg!(feature = "worker-network-events") {
        features.push("worker-network-events");
    }
    if cfg!(feature = "init-scripts") {
        features.push("init-scripts");
    }
    if cfg!(feature = "service-worker-policy") {
        features.push("service-worker-policy");
    }
    if cfg!(feature = "api-request") {
        features.push("api-request");
    }
    if cfg!(feature = "context-api-request") {
        features.push("context-api-request");
    }
    if cfg!(feature = "api-request-client-certificates") {
        features.push("api-request-client-certificates");
    }
    if cfg!(feature = "drag-source") {
        features.push("drag-source");
    }
    if cfg!(feature = "drag-runtime") {
        features.push("drag-runtime");
    }
    if cfg!(feature = "drag-and-drop") {
        features.push("drag-and-drop");
    }
    features
}

#[pyfunction]
fn build_profile() -> String {
    let features = selected_features();
    if features.is_empty() {
        "base".to_owned()
    } else {
        features.join("+")
    }
}

#[pyfunction]
fn resource_limits() -> BTreeMap<&'static str, u64> {
    let limits = BTreeMap::from([
        ("sessions", MAX_SESSIONS as u64),
        ("connects", 16),
        ("commands", 32),
        ("closes", 16),
        ("watches", MAX_FFI_WATCHES as u64),
        ("request_history_items_per_page", 100),
        ("request_history_owners", 4096),
        ("request_history_owners_per_page", 256),
        ("device_contexts", 16),
        ("device_context_bytes", 64 * 1024),
        ("locale_contexts", 16),
        ("locale_bytes_per_context", 256),
        ("timezone_contexts", 16),
        ("timezone_bytes_per_context", 256),
        ("offline_contexts", 16),
        ("geolocation_contexts", 16),
        ("geolocation_permission_contexts", 16),
        ("http_credential_contexts", 16),
        ("http_credential_field_bytes", 64 * 1024),
        ("persistent_launch_arguments", 256),
        ("persistent_launch_argument_bytes", 1024 * 1024),
        ("ignore_https_error_contexts", 16),
        ("bypass_csp_contexts", 16),
        ("javascript_disabled_contexts", 16),
        (
            "screenshot_bytes",
            pwright_node_host::screenshot::MAX_BYTES as u64,
        ),
        (
            "screenshot_concurrent_captures",
            pwright_node_host::screenshot::MAX_CONCURRENT_CAPTURES as u64,
        ),
        ("selector_engines", 64),
        ("selector_engine_name_bytes", 64),
        ("selector_engine_source_bytes", 1024 * 1024),
        ("selector_registry_bytes", 8 * 1024 * 1024),
        ("test_id_attribute_bytes", 1024),
        ("request_size_owners", 64),
        ("request_event_contexts", 16),
        ("request_event_notices", 256),
        ("request_event_bytes", 16 * 1024 * 1024),
        ("response_event_contexts", 16),
        ("response_event_notices", 256),
        ("response_event_bytes", 16 * 1024 * 1024),
        ("response_finished_owners", 256),
        ("response_finished_bytes", 64 * 1024),
        ("download_contexts", 16),
        ("download_owners", 64),
        ("download_notices", 64),
        ("download_path_bytes", 4096),
        ("worker_owners", 64),
        ("worker_notices", 128),
        ("worker_url_bytes", 1024 * 1024),
        ("worker_concurrent_evaluations", 8),
        ("worker_console_notices", 128),
        ("worker_console_bytes", 16 * 1024 * 1024),
        ("reserved_bytes", 64 * 1024 * 1024),
    ]);
    #[cfg(feature = "cdp-session")]
    let limits = {
        let mut limits = limits;
        limits.insert("cdp_sessions", pwright_node_host::MAX_CDP_SESSIONS as u64);
        limits.insert(
            "cdp_session_events",
            pwright_node_host::MAX_CDP_SESSION_EVENTS as u64,
        );
        limits.insert(
            "cdp_session_event_bytes",
            pwright_node_host::MAX_CDP_SESSION_EVENT_BYTES as u64,
        );
        limits
    };
    #[cfg(feature = "page-pdf")]
    let limits = {
        let mut limits = limits;
        limits.insert("pdf_bytes", pwright_node_host::page_pdf::MAX_BYTES as u64);
        limits.insert(
            "pdf_concurrent_captures",
            pwright_node_host::page_pdf::MAX_CONCURRENT_CAPTURES as u64,
        );
        limits
    };
    #[cfg(feature = "har-replay")]
    let limits = {
        let mut limits = limits;
        limits.insert(
            "har_replay_archive_bytes",
            pwright_node_host::MAX_HAR_REPLAY_ARCHIVE_BYTES as u64,
        );
        limits.insert(
            "har_replay_archives",
            pwright_node_host::MAX_HAR_REPLAY_ARCHIVES as u64,
        );
        limits.insert(
            "har_replay_retained_bytes",
            pwright_node_host::MAX_HAR_REPLAY_RETAINED_BYTES as u64,
        );
        limits
    };
    #[cfg(feature = "page-routes")]
    let limits = {
        let mut limits = limits;
        limits.insert(
            "route_fulfill_body_bytes",
            pwright_node_host::MAX_ROUTE_FULFILL_BODY_BYTES as u64,
        );
        limits.insert(
            "route_initializing_sessions",
            pwright_node_host::MAX_ROUTE_INITIALIZING_SESSIONS as u64,
        );
        limits
    };
    #[cfg(feature = "route-fulfill-path")]
    let limits = {
        let mut limits = limits;
        limits.insert(
            "route_fulfill_path_bytes",
            pwright_node_host::MAX_ROUTE_FULFILL_PATH_BYTES as u64,
        );
        limits
    };
    #[cfg(feature = "context-cookies")]
    let limits = {
        let mut limits = limits;
        limits.insert(
            "context_cookie_count",
            pwright_node_host::context_cookies::MAX_COOKIES as u64,
        );
        limits.insert(
            "context_cookie_batch_bytes",
            pwright_node_host::context_cookies::MAX_COOKIE_BATCH_BYTES as u64,
        );
        limits.insert(
            "context_cookie_urls",
            pwright_node_host::context_cookies::MAX_COOKIE_URLS as u64,
        );
        limits.insert(
            "context_cookie_url_bytes",
            pwright_node_host::context_cookies::MAX_COOKIE_URL_BYTES as u64,
        );
        limits
    };
    #[cfg(feature = "context-storage-state")]
    let limits = {
        let mut limits = limits;
        limits.insert(
            "context_storage_state_bytes",
            MAX_STORAGE_STATE_BYTES as u64,
        );
        limits.insert("context_storage_origins", MAX_STORAGE_ORIGINS as u64);
        limits.insert("context_storage_items", MAX_STORAGE_ITEMS as u64);
        limits
    };
    #[cfg(feature = "layout-selectors")]
    let limits = {
        let mut limits = limits;
        limits.insert("layout_selector_parts", MAX_LAYOUT_SELECTOR_PARTS as u64);
        limits.insert(
            "layout_selector_program_bytes",
            MAX_LAYOUT_SELECTOR_PROGRAM_BYTES as u64,
        );
        limits
    };
    #[cfg(feature = "page-viewport")]
    let limits = {
        let mut limits = limits;
        limits.insert(
            "page_viewport_pages",
            pwright_node_host::page_viewport::profile::PAGES as u64,
        );
        limits.insert(
            "page_viewport_bytes",
            pwright_node_host::page_viewport::profile::BYTES,
        );
        limits
    };
    #[cfg(feature = "file-input")]
    let limits = {
        let mut limits = limits;
        limits.insert(
            "file_input_items",
            pwright_node_host::file_input::profile::ITEMS as u64,
        );
        limits.insert(
            "file_input_path_bytes",
            pwright_node_host::file_input::profile::PATH_BYTES as u64,
        );
        limits.insert(
            "file_input_name_bytes",
            pwright_node_host::file_input::profile::NAME_BYTES as u64,
        );
        limits.insert(
            "file_input_mime_bytes",
            pwright_node_host::file_input::profile::MIME_BYTES as u64,
        );
        limits
    };
    #[cfg(feature = "wait-for-timeout")]
    let limits = {
        let mut limits = limits;
        limits.insert(
            "wait_for_timeout_operations",
            pwright_node_host::wait_for_timeout::profile::OPERATIONS,
        );
        limits.insert(
            "wait_for_timeout_bytes",
            pwright_node_host::wait_for_timeout::profile::BYTES,
        );
        limits
    };
    #[cfg(feature = "worker-handle-evaluation")]
    let limits = {
        let mut limits = limits;
        limits.insert(
            "worker_handles",
            pwright_node_host::worker_handles::MAX_WORKER_HANDLES as u64,
        );
        limits.insert(
            "worker_handle_concurrent_calls",
            pwright_node_host::worker_handles::MAX_CONCURRENT_WORKER_HANDLE_CALLS as u64,
        );
        limits
    };
    limits
}

struct SessionRecord {
    host: Arc<HostSession>,
    #[cfg(feature = "fresh-launch")]
    _profile: Option<tempfile::TempDir>,
}

impl SessionRecord {
    fn attached(host: &Arc<HostSession>) -> Self {
        Self {
            host: Arc::clone(host),
            #[cfg(feature = "fresh-launch")]
            _profile: None,
        }
    }

    #[cfg(feature = "fresh-launch")]
    fn fresh(host: &Arc<HostSession>, profile: tempfile::TempDir) -> Self {
        Self {
            host: Arc::clone(host),
            _profile: Some(profile),
        }
    }
}

#[derive(Default)]
struct OwnerState {
    pending: usize,
    sessions: Vec<SessionRecord>,
    #[cfg(feature = "playwright-server-connect-session")]
    remote_sessions: Vec<Weak<RemoteBrowserSupervisor>>,
}

struct EnvironmentOwner {
    stopped: CancellationToken,
    drained: CancellationToken,
    closing: AtomicBool,
    state: Mutex<OwnerState>,
    ffi: FfiBudget,
    queries: QueryBudget,
    #[cfg(feature = "selector-registration")]
    selectors: SelectorRegistry,
    cleanup: Mutex<CleanupReport>,
}

#[derive(Clone, Debug, Default)]
struct CleanupReport {
    sessions_closed: u32,
    #[cfg(feature = "playwright-server-connect-session")]
    remote_sessions_closed: u32,
    sessions_zero: bool,
    ffi_zero: bool,
}

impl EnvironmentOwner {
    fn new() -> Result<Arc<Self>, HostError> {
        Ok(Arc::new(Self {
            stopped: CancellationToken::new(),
            drained: CancellationToken::new(),
            closing: AtomicBool::new(false),
            state: Mutex::new(OwnerState::default()),
            cleanup: Mutex::new(CleanupReport::default()),
            ffi: FfiBudget::new(FfiLimits {
                connects: 16,
                commands: 32,
                closes: 16,
                watches: MAX_FFI_WATCHES,
                reserved_bytes: 64 * 1024 * 1024,
                future_bytes: [1024 * 1024, 128 * 1024, 16 * 1024, 16 * 1024],
            })?,
            queries: QueryBudget::new(QueryLimits {
                recipes: 1024,
                bytes: 8 * 1024 * 1024,
            })?,
            #[cfg(feature = "selector-registration")]
            selectors: SelectorRegistry::default(),
        }))
    }

    fn admit(self: &Arc<Self>) -> Result<Admission, HostError> {
        let mut state = self.state.lock().map_err(|_| {
            HostError::Backend("Python environment ownership is unavailable".into())
        })?;
        if self.stopped.is_cancelled() {
            return Err(HostError::TargetClosed);
        }
        #[cfg(feature = "playwright-server-connect-session")]
        state.remote_sessions.retain(|session| {
            session
                .upgrade()
                .is_some_and(|session| session.is_connected())
        });
        let remote_count = {
            #[cfg(feature = "playwright-server-connect-session")]
            {
                state.remote_sessions.len()
            }
            #[cfg(not(feature = "playwright-server-connect-session"))]
            {
                0
            }
        };
        if state.sessions.len() + remote_count + state.pending >= MAX_SESSIONS {
            return Err(HostError::ResourceExhausted(
                "Python environment session capacity exceeded (16)".into(),
            ));
        }
        state.pending += 1;
        Ok(Admission {
            owner: Arc::clone(self),
            pending: true,
        })
    }

    fn remove(&self, target: &Arc<HostSession>) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(|_| {
            HostError::Backend("Python environment ownership is unavailable".into())
        })?;
        let position = state
            .sessions
            .iter()
            .position(|session| Arc::ptr_eq(&session.host, target));
        let removed = position.map(|position| state.sessions.swap_remove(position));
        drop(state);
        drop(removed);
        Ok(())
    }

    #[cfg(feature = "playwright-server-connect-session")]
    fn remove_remote(&self, target: &Arc<RemoteBrowserSupervisor>) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(|_| {
            HostError::Backend("Python environment ownership is unavailable".into())
        })?;
        state.remote_sessions.retain(|session| {
            session
                .upgrade()
                .is_some_and(|session| !Arc::ptr_eq(&session, target))
        });
        Ok(())
    }

    fn request_shutdown(&self) {
        self.stopped.cancel();
        let sessions = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .sessions
            .iter()
            .map(|session| Arc::clone(&session.host))
            .collect::<Vec<_>>();
        for session in sessions {
            session.request_shutdown();
        }
    }

    async fn stop_and_drain(self: Arc<Self>) -> Result<(), HostError> {
        if self.closing.swap(true, Ordering::AcqRel) {
            self.drained.cancelled().await;
            return Ok(());
        }
        self.request_shutdown();
        #[cfg(feature = "playwright-server-connect-session")]
        let remote_sessions = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            std::mem::take(&mut state.remote_sessions)
                .into_iter()
                .filter_map(|session| session.upgrade())
                .filter(|session| session.is_connected())
                .collect::<Vec<_>>()
        };
        #[cfg(feature = "playwright-server-connect-session")]
        let remote_sessions_closed = u32::try_from(remote_sessions.len()).unwrap_or(u32::MAX);
        #[cfg(feature = "playwright-server-connect-session")]
        let mut remote_close_tasks = tokio::task::JoinSet::new();
        #[cfg(feature = "playwright-server-connect-session")]
        for session in &remote_sessions {
            let session = Arc::clone(session);
            remote_close_tasks.spawn(async move { session.close().await });
        }
        #[cfg(feature = "playwright-server-connect-session")]
        let remote_close_result = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            let mut first_failure = None;
            while let Some(joined) = remote_close_tasks.join_next().await {
                let failure = match joined {
                    Ok(Ok(_)) => None,
                    Ok(Err(error)) => Some(backend_host_error(format!("{error:?}"))),
                    Err(error) => Some(HostError::Backend(format!(
                        "Python remote Browser cleanup task failed: {error}"
                    ))),
                };
                if first_failure.is_none() {
                    first_failure = failure;
                }
            }
            first_failure
        })
        .await;
        #[cfg(feature = "playwright-server-connect-session")]
        let mut failure = match remote_close_result {
            Ok(failure) => failure,
            Err(_) => {
                remote_close_tasks.abort_all();
                Some(HostError::Backend(
                    "Python remote Browser cleanup exceeded 2 seconds".into(),
                ))
            }
        };
        #[cfg(not(feature = "playwright-server-connect-session"))]
        let mut failure: Option<HostError> = None;
        if let Err(error) = self.ffi.shutdown().await
            && failure.is_none()
        {
            failure = Some(error);
        }
        #[cfg(feature = "selector-registration")]
        if let Err(error) = self.selectors.clear()
            && failure.is_none()
        {
            failure = Some(error);
        }
        let sessions = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.pending = 0;
            std::mem::take(&mut state.sessions)
        };
        let sessions_closed = u32::try_from(sessions.len()).unwrap_or(u32::MAX);
        let mut sessions_zero = true;
        #[cfg(feature = "playwright-server-connect-session")]
        if remote_sessions.iter().any(|session| session.is_connected()) {
            sessions_zero = false;
            if failure.is_none() {
                failure = Some(HostError::Backend(
                    "Python environment retained remote Browser connections".into(),
                ));
            }
        }
        for session in &sessions {
            session.host.request_shutdown();
        }
        for session in sessions {
            if let Err(error) = session.host.close().await
                && failure.is_none()
            {
                failure = Some(error);
            }
            match session.host.metrics() {
                Ok(metrics) if metrics == SessionMetrics::default() => {}
                Ok(metrics) => {
                    sessions_zero = false;
                    if failure.is_none() {
                        failure = Some(HostError::Backend(format!(
                            "Python environment retained native resources: {metrics:?}"
                        )));
                    }
                }
                Err(error) => {
                    sessions_zero = false;
                    if failure.is_none() {
                        failure = Some(error);
                    }
                }
            }
        }
        let ffi_zero =
            matches!(self.ffi.metrics(), Ok(metrics) if metrics == FfiMetrics::default());
        if !ffi_zero && failure.is_none() {
            failure = Some(HostError::Backend(
                "Python environment retained FFI resources".into(),
            ));
        }
        *self
            .cleanup
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = CleanupReport {
            sessions_closed,
            #[cfg(feature = "playwright-server-connect-session")]
            remote_sessions_closed,
            sessions_zero,
            ffi_zero,
        };
        self.drained.cancel();
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

fn prepare_locator_query(owner: &EnvironmentOwner, selector: &str) -> PyResult<LocatorQuery> {
    let draft = owner
        .queries
        .prepare_leaf(selector.len())
        .map_err(binding_error)?;
    let selector = bounded_string(selector, 64 * 1024, "selector")?;
    #[cfg(feature = "selector-registration")]
    let query = draft
        .css_registered(selector, &owner.selectors)
        .map_err(binding_error)?;
    #[cfg(not(feature = "selector-registration"))]
    let query = draft.css(selector).map_err(binding_error)?;
    Ok(query)
}

#[cfg(feature = "element-handle-dom")]
fn prepare_element_handle_query(
    owner: &EnvironmentOwner,
    selector: &str,
) -> PyResult<ElementHandleQuery> {
    let draft = owner
        .queries
        .prepare_leaf(selector.len())
        .map_err(binding_error)?;
    let selector = bounded_string(selector, MAX_LOCATOR_INPUT_BYTES, "selector")?;
    draft.element_handle(selector).map_err(binding_error)
}

fn prepare_semantic_query(
    owner: &EnvironmentOwner,
    kind: &str,
    source: &str,
    exact: bool,
    flags: Option<&str>,
) -> PyResult<LocatorQuery> {
    if kind.len() > 16 {
        return Err(binding_error(HostError::InvalidArgument(
            "semantic Locator kind exceeds 16 UTF-8 bytes".into(),
        )));
    }
    if source.len() > MAX_LOCATOR_INPUT_BYTES {
        return Err(binding_error(HostError::InvalidArgument(format!(
            "semantic Locator input exceeds {MAX_LOCATOR_INPUT_BYTES} UTF-8 bytes"
        ))));
    }
    if flags.is_some_and(|value| value.len() > 8) {
        return Err(binding_error(HostError::InvalidArgument(
            "semantic Locator flags exceed 8 UTF-8 bytes".into(),
        )));
    }
    let kind = SemanticSelector::from_str(kind).map_err(binding_error)?;
    #[cfg(feature = "selector-registration")]
    let test_id_attributes = if matches!(kind, SemanticSelector::TestId) {
        owner
            .selectors
            .test_id_attributes()
            .map_err(binding_error)?
    } else {
        TestIdAttributes::standard()
    };
    #[cfg(not(feature = "selector-registration"))]
    let test_id_attributes = TestIdAttributes::standard();
    let implicit_string_bytes = if matches!(kind, SemanticSelector::TestId) {
        test_id_attributes.as_csv().len()
    } else {
        0
    };
    let draft = owner
        .queries
        .prepare_leaf(
            source
                .len()
                .saturating_add(flags.map_or(0, str::len))
                .saturating_add(implicit_string_bytes),
        )
        .map_err(binding_error)?;
    draft
        .semantic_with_test_id_attributes(
            kind,
            source.to_owned(),
            flags.map(str::to_owned),
            exact,
            test_id_attributes,
        )
        .map_err(binding_error)
}

#[allow(clippy::too_many_arguments)]
fn prepare_role_query(
    owner: &EnvironmentOwner,
    role: &str,
    name: Option<&str>,
    name_flags: Option<&str>,
    exact: bool,
    checked: Option<bool>,
    disabled: Option<bool>,
    expanded: Option<bool>,
    include_hidden: bool,
    level: Option<u32>,
    pressed: Option<bool>,
    selected: Option<bool>,
) -> PyResult<LocatorQuery> {
    if role.len() > 64 {
        return Err(binding_error(HostError::InvalidArgument(
            "ARIA role exceeds 64 UTF-8 bytes".into(),
        )));
    }
    if name.is_some_and(|value| value.len() > MAX_LOCATOR_INPUT_BYTES) {
        return Err(binding_error(HostError::InvalidArgument(format!(
            "ARIA accessible name exceeds {MAX_LOCATOR_INPUT_BYTES} UTF-8 bytes"
        ))));
    }
    if name_flags.is_some_and(|value| value.len() > 8) {
        return Err(binding_error(HostError::InvalidArgument(
            "ARIA accessible-name flags exceed 8 UTF-8 bytes".into(),
        )));
    }
    if name.is_none() && name_flags.is_some() {
        return Err(binding_error(HostError::InvalidArgument(
            "ARIA accessible-name flags require a name".into(),
        )));
    }
    let draft = owner
        .queries
        .prepare_leaf(
            role.len()
                .saturating_add(name.map_or(0, str::len))
                .saturating_add(name_flags.map_or(0, str::len)),
        )
        .map_err(binding_error)?;
    draft
        .role(
            role,
            RoleQueryOptions {
                name: name.map(str::to_owned),
                name_flags: name_flags.map(str::to_owned),
                exact,
                checked,
                disabled,
                expanded,
                include_hidden,
                level,
                pressed,
                selected,
            },
        )
        .map_err(binding_error)
}

struct Admission {
    owner: Arc<EnvironmentOwner>,
    pending: bool,
}

impl Admission {
    fn publish(self, session: &Arc<HostSession>) -> Result<(), HostError> {
        self.publish_record(SessionRecord::attached(session))
    }

    fn publish_record(mut self, session: SessionRecord) -> Result<(), HostError> {
        let mut state = self.owner.state.lock().map_err(|_| {
            HostError::Backend("Python environment ownership is unavailable".into())
        })?;
        state.pending = state.pending.saturating_sub(1);
        self.pending = false;
        let host = Arc::clone(&session.host);
        state.sessions.push(session);
        if self.owner.stopped.is_cancelled() {
            host.request_shutdown();
            return Err(HostError::TargetClosed);
        }
        Ok(())
    }

    #[cfg(feature = "playwright-server-connect-session")]
    fn publish_remote(mut self, session: &Arc<RemoteBrowserSupervisor>) -> Result<(), HostError> {
        let mut state = self.owner.state.lock().map_err(|_| {
            HostError::Backend("Python environment ownership is unavailable".into())
        })?;
        state.pending = state.pending.saturating_sub(1);
        self.pending = false;
        if self.owner.stopped.is_cancelled() {
            return Err(HostError::TargetClosed);
        }
        state.remote_sessions.push(Arc::downgrade(session));
        Ok(())
    }
}

impl Drop for Admission {
    fn drop(&mut self) {
        if self.pending {
            let mut state = self
                .owner
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.pending = state.pending.saturating_sub(1);
        }
    }
}

#[pyclass(module = "pwright._native")]
struct NativeEnvironment {
    owner: Arc<EnvironmentOwner>,
}

#[pymethods]
impl NativeEnvironment {
    #[new]
    fn new() -> PyResult<Self> {
        Ok(Self {
            owner: EnvironmentOwner::new().map_err(binding_error)?,
        })
    }

    fn register_selector_engine(
        &self,
        name: &str,
        source: &str,
        content_script: bool,
    ) -> PyResult<()> {
        if self.owner.stopped.is_cancelled() {
            return Err(binding_error(HostError::TargetClosed));
        }
        #[cfg(feature = "selector-registration")]
        {
            debug_assert_eq!(MAX_SELECTOR_ENGINES, 64);
            debug_assert_eq!(MAX_SELECTOR_ENGINE_NAME_BYTES, 64);
            debug_assert_eq!(MAX_SELECTOR_ENGINE_SOURCE_BYTES, 1024 * 1024);
            debug_assert_eq!(MAX_SELECTOR_REGISTRY_BYTES, 8 * 1024 * 1024);
            self.owner
                .selectors
                .register(name, source, content_script)
                .map_err(binding_error)
        }
        #[cfg(not(feature = "selector-registration"))]
        {
            let _ = (name, source, content_script);
            Err(binding_error(HostError::UnsupportedOperation(
                "selector registration capability is not installed".into(),
            )))
        }
    }

    fn set_test_id_attribute(&self, name: &str) -> PyResult<()> {
        if self.owner.stopped.is_cancelled() {
            return Err(binding_error(HostError::TargetClosed));
        }
        #[cfg(feature = "selector-registration")]
        {
            debug_assert_eq!(MAX_TEST_ID_ATTRIBUTE_BYTES, 1024);
            self.owner
                .selectors
                .set_test_id_attribute(name)
                .map_err(binding_error)
        }
        #[cfg(not(feature = "selector-registration"))]
        {
            let _ = name;
            Err(binding_error(HostError::UnsupportedOperation(
                "test-id selector configuration capability is not installed".into(),
            )))
        }
    }

    #[pyo3(signature = (endpoint, timeout_ms=30_000, headers=None))]
    fn connect_over_cdp<'py>(
        &self,
        py: Python<'py>,
        endpoint: &str,
        timeout_ms: u32,
        headers: Option<Bound<'py, PyList>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let endpoint = bounded_string(endpoint, MAX_ENDPOINT_BYTES, "CDP endpoint")?;
        #[cfg(feature = "cdp-handshake-headers")]
        let headers = headers
            .as_ref()
            .map(bounded_cdp_handshake_headers)
            .transpose()?;
        #[cfg(not(feature = "cdp-handshake-headers"))]
        if headers.as_ref().is_some_and(|headers| !headers.is_empty()) {
            return Err(binding_error(HostError::UnsupportedOperation(
                "CDP handshake headers capability is not installed".into(),
            )));
        }
        let admission = self.owner.admit().map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        let future = async move {
            #[cfg(feature = "cdp-handshake-headers")]
            let headers = headers.unwrap_or_default();
            #[cfg(feature = "page-events")]
            let connect = async {
                #[cfg(feature = "cdp-handshake-headers")]
                if !headers.is_empty() {
                    return HostSession::connect_over_cdp_with_lifecycle_events_and_headers(
                        &endpoint, timeout_ms, headers,
                    )
                    .await;
                }
                HostSession::connect_over_cdp_with_lifecycle_events(&endpoint, timeout_ms).await
            };
            #[cfg(not(feature = "page-events"))]
            let connect = async {
                #[cfg(feature = "cdp-handshake-headers")]
                if !headers.is_empty() {
                    return HostSession::connect_over_cdp_with_headers(
                        &endpoint, timeout_ms, headers,
                    )
                    .await;
                }
                HostSession::connect_over_cdp(&endpoint, timeout_ms).await
            };
            let host = tokio::select! {
                biased;
                () = owner.stopped.cancelled() => return Err(HostError::TargetClosed),
                result = connect => Arc::new(result?),
            };
            admission.publish(&host)?;
            Ok(host)
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            let outcome = ticket.deliver(|outcome| outcome).map_err(binding_error)?;
            let host = outcome.map_err(binding_error)?;
            #[cfg(feature = "page-events")]
            let lifecycle_conversion = Some(
                host.reserve_metadata_conversion(LIFECYCLE_CONVERSION_BYTES)
                    .map_err(binding_error)?,
            );
            #[cfg(not(feature = "page-events"))]
            let lifecycle_conversion = None;
            let lifecycle_callback = Arc::new(Mutex::new(None));
            let fence = host.frame_publications().fence().map_err(binding_error)?;
            Python::attach(|py| {
                publish_frame_fence(py, fence, &lifecycle_callback)?;
                Py::new(
                    py,
                    NativeSession {
                        host,
                        owner,
                        lifecycle_callback,
                        _lifecycle_conversion: lifecycle_conversion,
                        #[cfg(feature = "context-strict-selectors")]
                        context_strict_selectors: Arc::new(
                            context_strict_selectors::ContextStrictSelectors::default(),
                        ),
                    },
                )
            })
        })
    }

    #[cfg(feature = "owned-process-startup")]
    #[pyo3(signature = (engine, executable_path, timeout_ms=30_000))]
    fn observe_nonchromium_startup<'py>(
        &self,
        py: Python<'py>,
        engine: &str,
        executable_path: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let webkit = match engine {
            "firefox" => false,
            "webkit" => true,
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "startup observation supports only Firefox or WebKit".into(),
                )));
            }
        };
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, OWNED_PROCESS_STARTUP_ALLOWANCE)
            .map_err(binding_error)?;
        let executable_path =
            bounded_string(executable_path, MAX_PATH_BYTES, "browser executable path")?;
        let admission = self.owner.admit().map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        let future = async move {
            let _admission = admission;
            tokio::select! {
                biased;
                () = owner.stopped.cancelled() => Err(HostError::TargetClosed),
                observed = pwright_owned_process_startup::observe_explicit_exit(
                    Path::new(&executable_path),
                    &[],
                    Duration::from_millis(u64::from(timeout_ms)),
                ) => observed.map_err(backend_host_error),
            }
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            let outcome = ticket.deliver(|outcome| outcome).map_err(binding_error)?;
            let result: PyResult<()> = match outcome.map_err(binding_error)? {
                pwright_owned_process_startup::StartupObservation::ExitedBeforeReady(status) => {
                    let detail = status.code().map_or_else(
                        || "signal termination".to_owned(),
                        |code| format!("exit code {code}"),
                    );
                    let message = format!(
                        "BrowserType.launch: explicit executable exited before browser protocol was ready ({detail})"
                    );
                    if webkit {
                        Err(TargetClosedError::new_err(message))
                    } else {
                        Err(Error::new_err(message))
                    }
                }
                pwright_owned_process_startup::StartupObservation::RunningWithoutProtocol => {
                    Err(UnsupportedError::new_err(
                        "BrowserType.launch: Firefox/WebKit protocol is not installed; the explicitly launched process was stopped after the bounded startup probe",
                    ))
                }
            };
            result
        })
    }

    #[cfg(feature = "playwright-server-initialize")]
    #[pyo3(signature = (endpoint_url, timeout_ms=30_000))]
    fn observe_playwright_server_initialize<'py>(
        &self,
        py: Python<'py>,
        endpoint_url: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, PLAYWRIGHT_SERVER_INITIALIZE_ALLOWANCE)
            .map_err(binding_error)?;
        let endpoint_url = bounded_string(
            endpoint_url,
            PLAYWRIGHT_SERVER_ENDPOINT_BYTES,
            "Playwright Server endpoint",
        )?;
        let wall_time_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| {
                binding_error(HostError::Backend(
                    "system clock precedes the Unix epoch".into(),
                ))
            })?
            .as_millis()
            .try_into()
            .map_err(|_| {
                binding_error(HostError::Backend(
                    "system clock milliseconds exceed the supported range".into(),
                ))
            })?;
        let admission = self.owner.admit().map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        let future = async move {
            let _admission = admission;
            tokio::select! {
                biased;
                () = owner.stopped.cancelled() => Err(HostError::TargetClosed),
                observed = pwright_playwright_server_initialize::observe_root_initialize(
                    &pwright_playwright_server_initialize::LiveWsConnector,
                    &endpoint_url,
                    pwright_playwright_server_initialize::SdkLanguage::Python,
                    wall_time_ms,
                    timeout_ms,
                ) => observed.map_err(playwright_server_initialize_host_error),
            }
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            let outcome = ticket.deliver(|outcome| outcome).map_err(binding_error)?;
            let result: PyResult<()> = match outcome.map_err(binding_error)? {
                pwright_playwright_server_initialize::InitializeObservation::RemoteClose(frame) => {
                    let message = match frame {
                        Some(frame) if !frame.reason().is_empty() => format!(
                            "BrowserType.connect: WebSocket closed (code {}): {}",
                            frame.code(),
                            frame.reason()
                        ),
                        Some(frame) => format!(
                            "BrowserType.connect: WebSocket closed (code {})",
                            frame.code()
                        ),
                        None => "BrowserType.connect: WebSocket closed without a close frame".into(),
                    };
                    Err(Error::new_err(message))
                }
                pwright_playwright_server_initialize::InitializeObservation::RootResponse => {
                    Err(UnsupportedError::new_err(
                        "BrowserType.connect: Playwright Server root responded, but the remote GUID object graph is not installed",
                    ))
                }
                pwright_playwright_server_initialize::InitializeObservation::RemoteError {
                    message,
                    ..
                } => Err(Error::new_err(format!("BrowserType.connect: {message}"))),
                pwright_playwright_server_initialize::InitializeObservation::UnsupportedProtocol => {
                    Err(UnsupportedError::new_err(
                        "BrowserType.connect: unsupported Playwright Server protocol outcome",
                    ))
                }
            };
            result
        })
    }

    #[cfg(feature = "playwright-server-connect-session")]
    #[pyo3(signature = (endpoint_url, browser_name, timeout_ms=30_000))]
    fn connect_playwright_server<'py>(
        &self,
        py: Python<'py>,
        endpoint_url: &str,
        browser_name: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        if timeout_ms == 0 || timeout_ms > 120_000 {
            return Err(binding_error(HostError::InvalidArgument(
                "BrowserType.connect timeout must be between 1 and 120000 milliseconds".into(),
            )));
        }
        if !matches!(browser_name, "chromium" | "firefox" | "webkit") {
            return Err(binding_error(HostError::InvalidArgument(
                "BrowserType.connect browser name is unsupported".into(),
            )));
        }
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, PLAYWRIGHT_SERVER_INITIALIZE_ALLOWANCE)
            .map_err(binding_error)?;
        let endpoint_url = bounded_string(
            endpoint_url,
            PLAYWRIGHT_SERVER_ENDPOINT_BYTES,
            "Playwright Server endpoint",
        )?;
        let wall_time_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| {
                binding_error(HostError::Backend(
                    "system clock precedes the Unix epoch".into(),
                ))
            })?
            .as_millis()
            .try_into()
            .map_err(|_| {
                binding_error(HostError::Backend(
                    "system clock milliseconds exceed the supported range".into(),
                ))
            })?;
        let discovery_limits = DiscoveryLimits::new(MAX_DISCOVERY_BODY_BYTES, timeout_ms)
            .map_err(backend_host_error)
            .map_err(binding_error)?;
        let websocket_limits = WsLimits::new(1024 * 1024, 1024 * 1024, timeout_ms)
            .map_err(backend_host_error)
            .map_err(binding_error)?;
        let mut options = ConnectOptions::new(
            endpoint_url,
            format!("pwright-python/{}", env!("CARGO_PKG_VERSION")),
            discovery_limits,
            websocket_limits,
            SdkLanguage::Python,
            wall_time_ms,
        );
        options
            .add_websocket_header("x-playwright-browser", browser_name)
            .map_err(backend_host_error)
            .map_err(binding_error)?;
        let browser_name = browser_name.to_owned();
        #[cfg(feature = "playwright-server-close-notices")]
        let event_policy = EventPolicy::ProjectCloseNotices;
        #[cfg(not(feature = "playwright-server-close-notices"))]
        let event_policy = EventPolicy::DropUnsupportedEvents;
        let admission = self.owner.admit().map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        let future = async move {
            let session = tokio::select! {
                biased;
                () = owner.stopped.cancelled() => return Err(HostError::TargetClosed),
                result = tokio::time::timeout(
                    std::time::Duration::from_millis(u64::from(timeout_ms)),
                    pwright_playwright_server_connect::connect_live_with_event_policy(
                        &options,
                        event_policy,
                    ),
                ) => match result {
                    Ok(Ok(session)) => Arc::new(session),
                    Ok(Err(error)) => return Err(playwright_server_connect_host_error(error)),
                    Err(_) => return Err(HostError::Timeout(timeout_ms)),
                },
            };
            if session.name() != browser_name {
                let _ = session.close().await;
                return Err(HostError::Backend(
                    "BrowserType.connect returned a different browser type".into(),
                ));
            }
            let browser = match RemoteBrowser::new(Arc::clone(&session)) {
                Ok(browser) => browser,
                Err(error) => {
                    let _ = session.close().await;
                    return Err(playwright_server_operation_host_error(error));
                }
            };
            admission.publish_remote(&session)?;
            Ok((session, browser))
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            let outcome = ticket.deliver(|outcome| outcome).map_err(binding_error)?;
            let (supervisor, browser) = outcome.map_err(binding_error)?;
            Python::attach(|py| {
                Py::new(
                    py,
                    NativeRemoteBrowser {
                        supervisor,
                        browser,
                        owner,
                    },
                )
            })
        })
    }

    #[cfg(feature = "fresh-launch")]
    #[pyo3(signature = (headless=true, executable_path=None, channel=None, timeout_ms=30_000))]
    fn launch_chromium<'py>(
        &self,
        py: Python<'py>,
        headless: bool,
        executable_path: Option<&str>,
        channel: Option<&str>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, FRESH_LAUNCH_ALLOWANCE)
            .map_err(binding_error)?;
        let executable_path = executable_path
            .map(|value| bounded_string(value, MAX_PATH_BYTES, "Chromium executable path"))
            .transpose()?;
        let channel = channel
            .map(|value| bounded_string(value, 64, "Chromium channel"))
            .transpose()?;
        let admission = self.owner.admit().map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        let future = async move {
            let launch = async move {
                let (executable, profile) = tokio::task::spawn_blocking(move || {
                    let executable = chromium_executable(executable_path, channel)?;
                    let profile = tempfile::Builder::new()
                        .prefix("pwright-chromium-")
                        .tempdir()
                        .map_err(backend_host_error)?;
                    Ok::<_, HostError>((executable, profile))
                })
                .await
                .map_err(backend_host_error)??;
                let host = HostSession::launch_chromium_with_lifecycle_events(
                    executable,
                    profile.path().to_path_buf(),
                    headless,
                    timeout_ms,
                )
                .await?;
                Ok::<_, HostError>((Arc::new(host), profile))
            };
            let (host, profile) = tokio::select! {
                biased;
                () = owner.stopped.cancelled() => return Err(HostError::TargetClosed),
                result = launch => result?,
            };
            admission.publish_record(SessionRecord::fresh(&host, profile))?;
            Ok(host)
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            let outcome = ticket.deliver(|outcome| outcome).map_err(binding_error)?;
            let host = outcome.map_err(binding_error)?;
            #[cfg(feature = "page-events")]
            let lifecycle_conversion = Some(
                host.reserve_metadata_conversion(LIFECYCLE_CONVERSION_BYTES)
                    .map_err(binding_error)?,
            );
            #[cfg(not(feature = "page-events"))]
            let lifecycle_conversion = None;
            let lifecycle_callback = Arc::new(Mutex::new(None));
            let fence = host.frame_publications().fence().map_err(binding_error)?;
            Python::attach(|py| {
                publish_frame_fence(py, fence, &lifecycle_callback)?;
                Py::new(
                    py,
                    NativeSession {
                        host,
                        owner,
                        lifecycle_callback,
                        _lifecycle_conversion: lifecycle_conversion,
                        #[cfg(feature = "context-strict-selectors")]
                        context_strict_selectors: Arc::new(
                            context_strict_selectors::ContextStrictSelectors::default(),
                        ),
                    },
                )
            })
        })
    }

    #[cfg(feature = "persistent-context")]
    fn launch_persistent_context<'py>(
        &self,
        py: Python<'py>,
        user_data_dir: &str,
        headless: bool,
        options: PythonPersistentLaunchOptions,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, PERSISTENT_LAUNCH_ALLOWANCE)
            .map_err(binding_error)?;
        let user_data_dir = bounded_string(
            user_data_dir,
            MAX_PATH_BYTES,
            "persistent Chromium user-data directory",
        )?;
        let (
            base_url,
            (
                locale,
                timezone_id,
                geolocation,
                geolocation_permission,
                http_credentials,
                ignore_https_errors,
                offline,
                bypass_csp,
                javascript_enabled,
            ),
            device,
            media,
            extra_headers,
            launch_arguments,
            executable_path,
            channel,
            timeout_ms,
        ) = options;
        let executable_path = executable_path
            .as_deref()
            .map(|value| bounded_string(value, MAX_PATH_BYTES, "Chromium executable path"))
            .transpose()?;
        let channel = channel
            .as_deref()
            .map(|value| bounded_string(value, 64, "Chromium channel"))
            .transpose()?;
        let base_url = base_url
            .as_deref()
            .map(|value| bounded_string(value, MAX_URL_BYTES, "Context baseURL"))
            .transpose()?;
        #[cfg(feature = "persistent-context-locale")]
        let locale = locale
            .map(|value| {
                bounded_string(&value, 256, "locale").and_then(|value| {
                    pwright_node_host::ContextLocale::new(value).map_err(binding_error)
                })
            })
            .transpose()?;
        #[cfg(not(feature = "persistent-context-locale"))]
        if locale.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "persistent Context locale capability is not installed".into(),
            )));
        }
        #[cfg(feature = "persistent-context-timezone")]
        let timezone = timezone_id
            .map(|value| {
                bounded_string(&value, 256, "timezone_id").and_then(|value| {
                    pwright_node_host::ContextTimezone::new(value).map_err(binding_error)
                })
            })
            .transpose()?;
        #[cfg(not(feature = "persistent-context-timezone"))]
        if timezone_id.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "persistent Context timezone capability is not installed".into(),
            )));
        }
        #[cfg(feature = "persistent-context-geolocation")]
        let geolocation = geolocation
            .map(|(longitude, latitude, accuracy)| {
                pwright_node_host::ContextGeolocation::new(longitude, latitude, accuracy)
                    .map_err(binding_error)
            })
            .transpose()?;
        #[cfg(not(feature = "persistent-context-geolocation"))]
        if geolocation.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "persistent Context geolocation capability is not installed".into(),
            )));
        }
        #[cfg(not(feature = "persistent-context-geolocation-permission"))]
        if geolocation_permission {
            return Err(binding_error(HostError::UnsupportedOperation(
                "persistent Context geolocation permission capability is not installed".into(),
            )));
        }
        #[cfg(feature = "persistent-context-http-credentials")]
        let http_credentials = http_credentials
            .map(|(username, password, origin, send_always)| {
                pwright_node_host::ContextHttpCredentials::new_with_send(
                    username,
                    password,
                    origin,
                    send_always,
                )
                .map_err(binding_error)
            })
            .transpose()?;
        #[cfg(not(feature = "persistent-context-http-credentials"))]
        if http_credentials.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "persistent Context HTTP credentials capability is not installed".into(),
            )));
        }
        #[cfg(feature = "persistent-context-launch-arguments")]
        let launch_arguments = pwright_node_host::PersistentLaunchArguments::new(launch_arguments)
            .map_err(binding_error)?;
        #[cfg(not(feature = "persistent-context-launch-arguments"))]
        if !launch_arguments.is_empty() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "persistent Context launch-arguments capability is not installed".into(),
            )));
        }
        #[cfg(not(feature = "persistent-context-offline"))]
        if offline {
            return Err(binding_error(HostError::UnsupportedOperation(
                "persistent Context offline capability is not installed".into(),
            )));
        }
        #[cfg(feature = "persistent-context-device-emulation")]
        let device = device
            .map(|(user_agent, viewport, screen, scale, mobile, touch)| {
                let user_agent = user_agent
                    .map(|value| bounded_string(&value, 4096, "user_agent"))
                    .transpose()?;
                pwright_node_host::DeviceContextOptions::new(
                    user_agent, viewport, screen, scale, mobile, touch,
                )
                .map_err(binding_error)
            })
            .transpose()?;
        #[cfg(not(feature = "persistent-context-device-emulation"))]
        if device.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "persistent Context device emulation capability is not installed".into(),
            )));
        }
        #[cfg(feature = "persistent-context-emulate-media")]
        let media = media
            .map(|(color_scheme, reduced_motion, contrast)| {
                let color_scheme = color_scheme
                    .map(|value| bounded_string(&value, 64, "color_scheme"))
                    .transpose()?;
                let reduced_motion = reduced_motion
                    .map(|value| bounded_string(&value, 64, "reduced_motion"))
                    .transpose()?;
                let contrast = contrast
                    .map(|value| bounded_string(&value, 64, "contrast"))
                    .transpose()?;
                pwright_node_host::PersistentContextMediaOptions::new(
                    color_scheme.as_deref(),
                    reduced_motion.as_deref(),
                    contrast.as_deref(),
                )
                .map_err(binding_error)
            })
            .transpose()?;
        #[cfg(not(feature = "persistent-context-emulate-media"))]
        if media.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "persistent Context media emulation capability is not installed".into(),
            )));
        }
        #[cfg(feature = "persistent-context-extra-headers")]
        let extra_headers = extra_headers
            .map(pwright_node_host::PersistentContextExtraHeaders::new)
            .transpose()
            .map_err(binding_error)?;
        #[cfg(not(feature = "persistent-context-extra-headers"))]
        if extra_headers.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "persistent Context extra-headers capability is not installed".into(),
            )));
        }
        #[cfg(not(feature = "persistent-context-ignore-https-errors"))]
        if ignore_https_errors {
            return Err(binding_error(HostError::UnsupportedOperation(
                "persistent Context ignore-HTTPS-errors capability is not installed".into(),
            )));
        }
        #[cfg(not(feature = "persistent-context-bypass-csp"))]
        if bypass_csp {
            return Err(binding_error(HostError::UnsupportedOperation(
                "persistent Context bypass-CSP capability is not installed".into(),
            )));
        }
        #[cfg(not(feature = "persistent-context-javascript-disabled"))]
        if !javascript_enabled {
            return Err(binding_error(HostError::UnsupportedOperation(
                "persistent Context JavaScript-disabled capability is not installed".into(),
            )));
        }
        let admission = self.owner.admit().map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        let future = async move {
            let launch = async move {
                let (executable, profile) = tokio::task::spawn_blocking(move || {
                    persistent_launch_inputs(user_data_dir, executable_path, channel)
                })
                .await
                .map_err(backend_host_error)??;
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
                let context_defaults = pwright_node_host::PersistentContextDefaults::default();
                #[cfg(feature = "persistent-context-locale")]
                let context_defaults = context_defaults.with_locale(locale);
                #[cfg(feature = "persistent-context-timezone")]
                let context_defaults = context_defaults.with_timezone(timezone);
                #[cfg(feature = "persistent-context-offline")]
                let context_defaults = context_defaults.with_offline(offline);
                #[cfg(feature = "persistent-context-geolocation")]
                let context_defaults = context_defaults.with_geolocation(geolocation);
                #[cfg(feature = "persistent-context-geolocation-permission")]
                let context_defaults =
                    context_defaults.with_geolocation_permission(geolocation_permission);
                #[cfg(feature = "persistent-context-http-credentials")]
                let context_defaults = context_defaults.with_http_credentials(http_credentials);
                #[cfg(feature = "persistent-context-device-emulation")]
                let context_defaults = context_defaults.with_device(device);
                #[cfg(feature = "persistent-context-ignore-https-errors")]
                let context_defaults =
                    context_defaults.with_ignore_https_errors(ignore_https_errors);
                #[cfg(feature = "persistent-context-emulate-media")]
                let context_defaults = context_defaults.with_media(media);
                #[cfg(feature = "persistent-context-extra-headers")]
                let context_defaults = context_defaults.with_extra_headers(extra_headers);
                #[cfg(feature = "persistent-context-bypass-csp")]
                let context_defaults = context_defaults.with_bypass_csp(bypass_csp);
                #[cfg(feature = "persistent-context-javascript-disabled")]
                let context_defaults =
                    context_defaults.with_javascript_disabled(!javascript_enabled);
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
                let launch = HostSession::launch_persistent_chromium_with_lifecycle_events_and_context_defaults(
                        executable,
                        profile,
                        headless,
                        base_url.as_deref(),
                        context_defaults,
                        #[cfg(feature = "persistent-context-launch-arguments")]
                        launch_arguments,
                        timeout_ms,
                    );
                #[cfg(not(any(
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
                )))]
                let launch = HostSession::launch_persistent_chromium_with_lifecycle_events(
                    executable,
                    profile,
                    headless,
                    base_url.as_deref(),
                    timeout_ms,
                );
                launch.await.map(Arc::new)
            };
            let host = tokio::select! {
                biased;
                () = owner.stopped.cancelled() => return Err(HostError::TargetClosed),
                result = launch => result?,
            };
            admission.publish(&host)?;
            Ok(host)
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            let outcome = ticket.deliver(|outcome| outcome).map_err(binding_error)?;
            let host = outcome.map_err(binding_error)?;
            #[cfg(feature = "page-events")]
            let lifecycle_conversion = Some(
                host.reserve_metadata_conversion(LIFECYCLE_CONVERSION_BYTES)
                    .map_err(binding_error)?,
            );
            #[cfg(not(feature = "page-events"))]
            let lifecycle_conversion = None;
            let lifecycle_callback = Arc::new(Mutex::new(None));
            let fence = host.frame_publications().fence().map_err(binding_error)?;
            Python::attach(|py| {
                publish_frame_fence(py, fence, &lifecycle_callback)?;
                Py::new(
                    py,
                    NativeSession {
                        host,
                        owner,
                        lifecycle_callback,
                        _lifecycle_conversion: lifecycle_conversion,
                        #[cfg(feature = "context-strict-selectors")]
                        context_strict_selectors: Arc::new(
                            context_strict_selectors::ContextStrictSelectors::default(),
                        ),
                    },
                )
            })
        })
    }

    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let owner = Arc::clone(&self.owner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            // Spawn only after the PyO3 awaitable has entered its Tokio
            // runtime. Dropping the Python awaiter then detaches this task;
            // native cleanup continues without a Python callback.
            let task = tokio::spawn(owner.stop_and_drain());
            task.await
                .map_err(|error| Error::new_err(format!("native cleanup task failed: {error}")))?
                .map_err(binding_error)?;
            Ok(())
        })
    }

    fn metrics<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        let metrics = self.owner.ffi.metrics().map_err(binding_error)?;
        let result = PyDict::new(py);
        result.set_item("pending", metrics.pending)?;
        result.set_item("queued", metrics.queued)?;
        result.set_item("queued_commands", metrics.queued_commands)?;
        result.set_item("running", metrics.running)?;
        result.set_item("reserved_bytes", metrics.reserved_bytes)?;
        result.set_item("connects", metrics.connects)?;
        result.set_item("commands", metrics.commands)?;
        result.set_item("closes", metrics.closes)?;
        result.set_item("watches", metrics.watches)?;
        Ok(result.into_any().unbind())
    }

    fn cleanup_report<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        let report = self
            .owner
            .cleanup
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let result = PyDict::new(py);
        result.set_item("sessions_closed", report.sessions_closed)?;
        #[cfg(feature = "playwright-server-connect-session")]
        result.set_item("remote_sessions_closed", report.remote_sessions_closed)?;
        result.set_item("sessions_zero", report.sessions_zero)?;
        result.set_item("ffi_zero", report.ffi_zero)?;
        Ok(result.into_any().unbind())
    }
}

impl Drop for NativeEnvironment {
    fn drop(&mut self) {
        self.owner.request_shutdown();
    }
}

#[cfg(feature = "playwright-server-connect-session")]
#[pyclass(module = "pwright._native")]
struct NativeRemoteBrowser {
    supervisor: Arc<RemoteBrowserSupervisor>,
    browser: RemoteBrowser,
    owner: Arc<EnvironmentOwner>,
}

#[cfg(feature = "playwright-server-connect-session")]
#[pymethods]
impl NativeRemoteBrowser {
    #[getter]
    fn guid(&self) -> String {
        self.supervisor.guid().to_owned()
    }

    #[getter]
    fn version(&self) -> String {
        self.supervisor.version().to_owned()
    }

    fn is_connected(&self) -> bool {
        self.browser.is_connected()
    }

    /// A point-in-time inventory of live Context generations under this Browser.
    /// The Python facade refuses unknown generations instead of returning a
    /// misleading partial `browser.contexts` list.
    fn live_contexts(&self) -> Vec<(String, u64)> {
        if !self.browser.is_connected() {
            return Vec::new();
        }
        self.supervisor
            .snapshot()
            .objects
            .into_iter()
            .filter(|object| {
                if object.object_type != "BrowserContext"
                    || object.parent.as_deref() != Some(self.supervisor.guid())
                {
                    return false;
                }
                #[cfg(any(
                    feature = "playwright-server-frame-title",
                    feature = "playwright-server-close-notices"
                ))]
                if object.closed {
                    return false;
                }
                true
            })
            .map(|object| (object.guid, object.created_sequence))
            .collect()
    }

    fn new_context<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Command, SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let browser = self.browser.clone();
        let wall_time_ms = playwright_server_wall_time().map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        let future = async move {
            let metadata = OperationMetadata::new("Browser.newContext", wall_time_ms)
                .map_err(playwright_server_operation_host_error)?;
            browser
                .new_context(EmptyContextOptions, &metadata)
                .await
                .map_err(playwright_server_operation_host_error)
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            let outcome = ticket.deliver(|outcome| outcome).map_err(binding_error)?;
            let context = outcome.map_err(binding_error)?;
            Python::attach(|py| Py::new(py, NativeRemoteContext { context, owner }))
        })
    }

    fn wait_disconnected<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Watch, SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let mut watcher = self.supervisor.terminal_watcher();
        let future = async move {
            watcher.wait_terminal().await;
            Ok(())
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            let outcome = ticket.deliver(|outcome| outcome).map_err(binding_error)?;
            outcome.map_err(binding_error)
        })
    }

    #[cfg(feature = "playwright-server-close-notices")]
    fn next_close_notice<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Watch, SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let supervisor = Arc::clone(&self.supervisor);
        let future = async move {
            let notice = supervisor
                .next_close_notice()
                .await
                .map_err(playwright_server_notice_host_error)?;
            let result = match notice {
                CloseNotice::PageClosed(id) => ("page", Some(id.guid), Some(id.generation)),
                CloseNotice::ContextClosed(id) => ("context", Some(id.guid), Some(id.generation)),
                CloseNotice::BrowserClosed(id) => ("browser", Some(id.guid), Some(id.generation)),
                CloseNotice::Disconnected => ("disconnected", None, None),
            };
            Ok::<_, HostError>(result)
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            let outcome = ticket.deliver(|outcome| outcome).map_err(binding_error)?;
            outcome.map_err(binding_error)
        })
    }

    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Close, SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let supervisor = Arc::clone(&self.supervisor);
        let owner = Arc::clone(&self.owner);
        let future = async move {
            let outcome = supervisor.close().await;
            owner.remove_remote(&supervisor)?;
            outcome.map_err(|error| backend_host_error(format!("{error:?}")))?;
            Ok(())
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            let outcome = ticket.deliver(|outcome| outcome).map_err(binding_error)?;
            outcome.map_err(binding_error)
        })
    }
}

#[cfg(feature = "playwright-server-connect-session")]
#[pyclass(module = "pwright._native")]
struct NativeRemoteContext {
    context: RemoteContext,
    owner: Arc<EnvironmentOwner>,
}

#[cfg(feature = "playwright-server-connect-session")]
#[pymethods]
impl NativeRemoteContext {
    #[getter]
    fn guid(&self) -> String {
        self.context.guid().to_owned()
    }

    #[getter]
    fn created_sequence(&self) -> u64 {
        self.context.created_sequence()
    }

    fn is_open(&self) -> bool {
        self.context.is_open()
    }

    /// A point-in-time inventory of live Page generations under this Context.
    fn live_pages(&self) -> Vec<(String, u64)> {
        if !self.context.is_open() {
            return Vec::new();
        }
        self.context
            .supervisor()
            .snapshot()
            .objects
            .into_iter()
            .filter(|object| {
                if object.object_type != "Page"
                    || object.parent.as_deref() != Some(self.context.guid())
                {
                    return false;
                }
                #[cfg(any(
                    feature = "playwright-server-frame-title",
                    feature = "playwright-server-close-notices"
                ))]
                if object.closed {
                    return false;
                }
                true
            })
            .map(|object| (object.guid, object.created_sequence))
            .collect()
    }

    fn new_page<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Command, SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let context = self.context.clone();
        let wall_time_ms = playwright_server_wall_time().map_err(binding_error)?;
        #[cfg(feature = "playwright-server-frame-title")]
        let owner = Arc::clone(&self.owner);
        let future = async move {
            let metadata = OperationMetadata::new("BrowserContext.newPage", wall_time_ms)
                .map_err(playwright_server_operation_host_error)?;
            context
                .new_page(&metadata)
                .await
                .map_err(playwright_server_operation_host_error)
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            let outcome = ticket.deliver(|outcome| outcome).map_err(binding_error)?;
            let page = outcome.map_err(binding_error)?;
            Python::attach(|py| {
                Py::new(
                    py,
                    NativeRemotePage {
                        page,
                        #[cfg(feature = "playwright-server-frame-title")]
                        owner,
                    },
                )
            })
        })
    }

    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Close, SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let context = self.context.clone();
        let wall_time_ms = playwright_server_wall_time().map_err(binding_error)?;
        let future = async move {
            let metadata = OperationMetadata::new("BrowserContext.close", wall_time_ms)
                .map_err(playwright_server_operation_host_error)?;
            context
                .close(&metadata)
                .await
                .map_err(playwright_server_operation_host_error)
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            let outcome = ticket.deliver(|outcome| outcome).map_err(binding_error)?;
            outcome.map_err(binding_error)
        })
    }
}

#[cfg(feature = "playwright-server-connect-session")]
#[pyclass(module = "pwright._native")]
struct NativeRemotePage {
    page: RemotePage,
    #[cfg(feature = "playwright-server-frame-title")]
    owner: Arc<EnvironmentOwner>,
}

#[cfg(feature = "playwright-server-connect-session")]
#[pymethods]
impl NativeRemotePage {
    #[getter]
    fn guid(&self) -> String {
        self.page.guid().to_owned()
    }

    #[getter]
    fn created_sequence(&self) -> u64 {
        self.page.created_sequence()
    }

    fn is_open(&self) -> bool {
        self.page.is_open()
    }

    #[cfg(feature = "playwright-server-frame-title")]
    fn title<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Command, SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let page = self.page.clone();
        let wall_time_ms = playwright_server_wall_time().map_err(binding_error)?;
        let future = async move {
            let metadata = OperationMetadata::new("Page.title", wall_time_ms)
                .map_err(playwright_server_operation_host_error)?;
            page.title(&metadata)
                .await
                .map_err(playwright_server_operation_host_error)
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            let outcome = ticket.deliver(|outcome| outcome).map_err(binding_error)?;
            outcome.map_err(binding_error)
        })
    }
}

#[cfg(feature = "network-events")]
#[pyclass(frozen, module = "pwright._native")]
struct NativeRequestSizeOwner {
    owner: Arc<RequestSizeOwner>,
}

#[cfg(feature = "network-events")]
#[pyclass(frozen, module = "pwright._native")]
struct NativeResponseBody {
    body: Arc<[u8]>,
}

#[cfg(feature = "response-finished")]
#[pyclass(frozen, module = "pwright._native")]
struct NativeResponseFinishedOwner {
    owner: Arc<ResponseFinishedOwner>,
}

#[cfg(feature = "request-history")]
#[pyclass(frozen, module = "pwright._native")]
struct NativeRequestHistoryOwner {
    owner: Arc<RequestHistoryOwner>,
}

#[pyclass(module = "pwright._native")]
struct NativeSession {
    host: Arc<HostSession>,
    owner: Arc<EnvironmentOwner>,
    lifecycle_callback: Arc<Mutex<Option<Py<PyAny>>>>,
    _lifecycle_conversion: Option<pwright_node_host::frame_tree::FrameConversion>,
    #[cfg(feature = "context-strict-selectors")]
    context_strict_selectors: Arc<context_strict_selectors::ContextStrictSelectors>,
}

/// Owns a newly attached child until the Python facade installs its wrapper.
/// In particular, this value also owns cleanup while an FFI result is queued
/// but not yet consumed by a Python task.
#[cfg(feature = "cdp-session")]
#[pyclass(module = "pwright._native")]
struct NativeCdpSessionLease {
    token: u64,
    cleanup: Option<Box<dyn FnOnce() + Send + Sync>>,
}

#[cfg(feature = "cdp-session")]
impl NativeCdpSessionLease {
    fn new(token: u64, cleanup: impl FnOnce() + Send + Sync + 'static) -> Self {
        Self {
            token,
            cleanup: Some(Box::new(cleanup)),
        }
    }
}

#[cfg(feature = "cdp-session")]
impl Drop for NativeCdpSessionLease {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}

#[cfg(feature = "cdp-session")]
#[pymethods]
impl NativeCdpSessionLease {
    fn token(&self) -> u64 {
        self.token
    }

    fn claim(&mut self) -> PyResult<u64> {
        if self.cleanup.take().is_none() {
            return Err(binding_error(HostError::InvalidArgument(
                "CDP session lease was already claimed".into(),
            )));
        }
        Ok(self.token)
    }
}

/// A bounded native claim. Dropping it rolls back any unacknowledged events.
/// Python conversion is deliberately deferred until each individual peek.
#[cfg(feature = "cdp-session")]
#[pyclass(module = "pwright._native")]
struct NativeCdpEventBatch {
    batch: CdpSessionEventBatch,
}

#[cfg(feature = "cdp-session")]
#[pymethods]
impl NativeCdpEventBatch {
    fn peek(&self, py: Python<'_>) -> PyResult<Option<(String, Py<PyDict>)>> {
        self.batch
            .peek()
            .map_err(binding_error)?
            .map(|event| cdp_event_to_python(py, event))
            .transpose()
    }

    fn ack_next(&self) -> PyResult<()> {
        self.batch.ack_next().map_err(binding_error)
    }

    fn rollback(&self) -> PyResult<()> {
        self.batch.rollback().map_err(binding_error)
    }
}

#[pyclass(frozen, module = "pwright._native")]
struct NativeLocator {
    locator: HostLocator,
    owner: Arc<EnvironmentOwner>,
    lifecycle_callback: Arc<Mutex<Option<Py<PyAny>>>>,
}

#[cfg(feature = "handles")]
#[pyclass(frozen, module = "pwright._native")]
struct NativeFrameLocator {
    locator: HostFrameLocator,
    owner: Arc<EnvironmentOwner>,
    lifecycle_callback: Arc<Mutex<Option<Py<PyAny>>>>,
}

#[pyclass(module = "pwright._native")]
struct NativeDialog {
    #[cfg(feature = "page-dialogs")]
    action: Arc<pwright_node_host::page_dialogs::DialogAction>,
}

#[pyclass(module = "pwright._native")]
struct NativeFileChooser {
    #[cfg(feature = "file-chooser-events")]
    action: Arc<pwright_node_host::page_file_chooser::FileChooserAction>,
}

#[pymethods]
impl NativeFileChooser {
    fn is_element(&self) -> bool {
        true
    }

    fn preview(&self) -> &'static str {
        "JSHandle@<input/>"
    }
}

#[cfg(feature = "download-events")]
#[pyclass(frozen, module = "pwright._native")]
struct NativeDownload {
    owner: Arc<DownloadOwner>,
}

#[cfg(feature = "worker-events")]
#[pyclass(frozen, module = "pwright._native")]
struct NativeWorker {
    owner: Arc<WorkerOwner>,
}

fn screenshot_options(
    format: i8,
    path: Option<&str>,
    quality: Option<u8>,
    full_page: bool,
    scale: u8,
) -> PyResult<ScreenshotOptions> {
    let format = match format {
        -1 => match path.and_then(|path| Path::new(path).extension()) {
            None => ScreenshotFormat::Png,
            Some(extension) if extension.eq_ignore_ascii_case("png") => ScreenshotFormat::Png,
            Some(extension)
                if extension.eq_ignore_ascii_case("jpg")
                    || extension.eq_ignore_ascii_case("jpeg") =>
            {
                ScreenshotFormat::Jpeg
            }
            Some(extension) => {
                return Err(binding_error(HostError::InvalidArgument(format!(
                    "screenshot path has unsupported extension .{}",
                    extension.to_string_lossy()
                ))));
            }
        },
        0 => ScreenshotFormat::Png,
        1 => ScreenshotFormat::Jpeg,
        _ => {
            return Err(binding_error(HostError::InvalidArgument(
                "screenshot format must be png or jpeg".into(),
            )));
        }
    };
    if quality.is_some() && format != ScreenshotFormat::Jpeg {
        return Err(binding_error(HostError::InvalidArgument(
            "quality requires jpeg and an integer in [0, 100]".into(),
        )));
    }
    if quality.is_some_and(|quality| quality > 100) {
        return Err(binding_error(HostError::InvalidArgument(
            "quality requires jpeg and an integer in [0, 100]".into(),
        )));
    }
    let scale = match scale {
        0 => ScreenshotScale::Device,
        1 => ScreenshotScale::Css,
        _ => {
            return Err(binding_error(HostError::InvalidArgument(
                "screenshot scale must be device or css".into(),
            )));
        }
    };
    Ok(ScreenshotOptions {
        format: Some(format),
        quality,
        full_page: full_page.then_some(true),
        clip: None,
        scale: Some(scale),
        document_clip: false,
    })
}

fn screenshot_masks(
    value: Option<&Bound<'_, PyAny>>,
    expected_host: &Arc<HostSession>,
) -> PyResult<Vec<HostLocator>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = value.cast::<PyList>().map_err(|_| {
        binding_error(HostError::InvalidArgument(
            "screenshot mask must be a list of native Locators".into(),
        ))
    })?;
    if values.len() > pwright_node_host::screenshot::MAX_MASK_LOCATORS {
        return Err(binding_error(HostError::ResourceExhausted(format!(
            "screenshot mask list exceeds {} Locators",
            pwright_node_host::screenshot::MAX_MASK_LOCATORS
        ))));
    }
    values
        .iter()
        .map(|value| {
            let locator = value.extract::<PyRef<'_, NativeLocator>>().map_err(|_| {
                binding_error(HostError::InvalidArgument(
                    "screenshot mask must contain native Locators".into(),
                ))
            })?;
            let host = locator.locator.host().map_err(binding_error)?;
            if !Arc::ptr_eq(&host, expected_host) {
                return Err(binding_error(HostError::InvalidArgument(
                    "screenshot mask Locator belongs to another browser session".into(),
                )));
            }
            Ok(locator.locator.clone())
        })
        .collect()
}

type LifecycleCallback = Arc<Mutex<Option<Py<PyAny>>>>;
#[cfg(feature = "page-events")]
type LifecycleDescriptor = (
    u8,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<PythonPageErrorDescriptor>,
    Option<PythonConsoleDescriptor>,
    Option<PythonDialogDescriptor>,
    Option<PythonFileChooserDescriptor>,
    Option<String>,
);
#[cfg(feature = "page-events")]
type PythonPageErrorDescriptor = (String, String, String);
#[cfg(all(feature = "page-events", feature = "console-argument-handles"))]
type PythonConsoleDescriptor = (
    String,
    String,
    String,
    String,
    u32,
    u32,
    u64,
    Vec<Py<handles::NativeHandle>>,
);
#[cfg(all(feature = "page-events", not(feature = "console-argument-handles")))]
type PythonConsoleDescriptor = (String, String, String, String, u32, u32, u64);
#[cfg(feature = "page-events")]
type PythonDialogDescriptor = (Py<NativeDialog>, String, String, String);
#[cfg(feature = "page-events")]
type PythonFileChooserDescriptor = (Py<NativeFileChooser>, bool);

#[cfg(feature = "page-events")]
enum DialogDescriptor {
    None,
    #[cfg(feature = "page-dialogs")]
    Some(Arc<pwright_node_host::PageDialog>),
}

#[cfg(feature = "page-events")]
enum FileChooserDescriptor {
    None,
    #[cfg(feature = "file-chooser-events")]
    Some(Arc<pwright_node_host::PageFileChooser>),
}

#[cfg(feature = "page-events")]
fn lifecycle_descriptor(
    py: Python<'_>,
    view: LifecycleNoticeView<'_>,
) -> PyResult<LifecycleDescriptor> {
    #[cfg(feature = "frame-events")]
    let frame_id = match &view.notice {
        LifecycleNotice::PageFrameAttached { frame, .. }
        | LifecycleNotice::PageFrameNavigated { frame, .. } => Some(frame.get().to_string()),
        _ => None,
    };
    #[cfg(not(feature = "frame-events"))]
    let frame_id: Option<String> = None;
    let (kind, context, page, opener, error, console, dialog, file_chooser) = match &view.notice {
        LifecycleNotice::PageClose { context, page } => (
            0,
            *context,
            Some(*page),
            None,
            None,
            None,
            DialogDescriptor::None,
            FileChooserDescriptor::None,
        ),
        LifecycleNotice::ContextClose { context } => (
            1,
            *context,
            None,
            None,
            None,
            None,
            DialogDescriptor::None,
            FileChooserDescriptor::None,
        ),
        LifecycleNotice::ContextPage { context, page } => (
            2,
            *context,
            Some(*page),
            None,
            None,
            None,
            DialogDescriptor::None,
            FileChooserDescriptor::None,
        ),
        LifecycleNotice::Popup { opener, popup } => {
            let context = view
                .revision
                .pages()
                .iter()
                .find(|value| value.id == *popup)
                .map(|value| value.context)
                .ok_or_else(|| {
                    binding_error(HostError::Backend("popup descriptor is missing".into()))
                })?;
            (
                3,
                context,
                Some(*popup),
                Some(*opener),
                None,
                None,
                DialogDescriptor::None,
                FileChooserDescriptor::None,
            )
        }
        LifecycleNotice::PageDomContentLoaded { context, page } => (
            4,
            *context,
            Some(*page),
            None,
            None,
            None,
            DialogDescriptor::None,
            FileChooserDescriptor::None,
        ),
        LifecycleNotice::PageLoad { context, page } => (
            5,
            *context,
            Some(*page),
            None,
            None,
            None,
            DialogDescriptor::None,
            FileChooserDescriptor::None,
        ),
        LifecycleNotice::PageConsole {
            context,
            page,
            message,
        } => (
            6,
            *context,
            Some(*page),
            None,
            None,
            Some(message.clone()),
            DialogDescriptor::None,
            FileChooserDescriptor::None,
        ),
        LifecycleNotice::PageMainFrameNavigated { context, page } => (
            7,
            *context,
            Some(*page),
            None,
            None,
            None,
            DialogDescriptor::None,
            FileChooserDescriptor::None,
        ),
        #[cfg(feature = "frame-events")]
        LifecycleNotice::PageFrameAttached { context, page, .. } => (
            11,
            *context,
            Some(*page),
            None,
            None,
            None,
            DialogDescriptor::None,
            FileChooserDescriptor::None,
        ),
        #[cfg(feature = "frame-events")]
        LifecycleNotice::PageFrameNavigated { context, page, .. } => (
            12,
            *context,
            Some(*page),
            None,
            None,
            None,
            DialogDescriptor::None,
            FileChooserDescriptor::None,
        ),
        LifecycleNotice::PageError {
            context,
            page,
            error,
        } => (
            8,
            *context,
            Some(*page),
            None,
            Some(error.clone()),
            None,
            DialogDescriptor::None,
            FileChooserDescriptor::None,
        ),
        #[cfg(feature = "page-dialogs")]
        LifecycleNotice::PageDialog {
            context,
            page,
            dialog,
        } => (
            9,
            *context,
            Some(*page),
            None,
            None,
            None,
            DialogDescriptor::Some(dialog.clone()),
            FileChooserDescriptor::None,
        ),
        #[cfg(feature = "file-chooser-events")]
        LifecycleNotice::PageFileChooser {
            context,
            page,
            chooser,
        } => (
            10,
            *context,
            Some(*page),
            None,
            None,
            None,
            DialogDescriptor::None,
            FileChooserDescriptor::Some(chooser.clone()),
        ),
    };
    let context_descriptor = view
        .revision
        .contexts()
        .iter()
        .find(|value| value.id == context);
    let page_descriptor = page.and_then(|page| {
        view.revision
            .pages()
            .iter()
            .find(|value| value.id == page && value.context == context)
    });
    let error_descriptor = error.as_ref().map(|value| {
        (
            value.name().to_owned(),
            value.message().to_owned(),
            value.stack().to_owned(),
        )
    });
    let console_descriptor = console
        .as_ref()
        .map(|value| -> PyResult<PythonConsoleDescriptor> {
            #[cfg(feature = "console-argument-handles")]
            let argument_handles = value
                .argument_handles()
                .iter()
                .cloned()
                .map(|handle| Py::new(py, handles::NativeHandle::from_shared(handle)))
                .collect::<PyResult<Vec<_>>>()?;
            Ok((
                value.message_type().to_owned(),
                value.text().to_owned(),
                value.arguments_json().to_owned(),
                value.location_url().to_owned(),
                value.line_number(),
                value.column_number(),
                value.timestamp_micros(),
                #[cfg(feature = "console-argument-handles")]
                argument_handles,
            ))
        })
        .transpose()?;
    #[cfg(not(feature = "page-dialogs"))]
    let _ = (py, dialog);
    #[cfg(feature = "page-dialogs")]
    let dialog_descriptor = match dialog {
        DialogDescriptor::Some(dialog) => Some((
            Py::new(
                py,
                NativeDialog {
                    action: dialog.action(),
                },
            )?,
            dialog.kind().as_str().to_owned(),
            dialog.message().to_owned(),
            dialog.default_value().to_owned(),
        )),
        DialogDescriptor::None => None,
    };
    #[cfg(not(feature = "page-dialogs"))]
    let dialog_descriptor = None;
    #[cfg(not(feature = "file-chooser-events"))]
    let _ = file_chooser;
    #[cfg(feature = "file-chooser-events")]
    let file_chooser_descriptor = match file_chooser {
        FileChooserDescriptor::Some(chooser) => Some((
            Py::new(
                py,
                NativeFileChooser {
                    action: chooser.action(),
                },
            )?,
            chooser.is_multiple(),
        )),
        FileChooserDescriptor::None => None,
    };
    #[cfg(not(feature = "file-chooser-events"))]
    let file_chooser_descriptor = None;
    Ok((
        kind,
        context.get().to_string(),
        page.map(|value| value.get().to_string()),
        opener.map(|value| value.get().to_string()),
        context_descriptor.and_then(|value| value.settings.base_url().map(str::to_owned)),
        page_descriptor.map(|value| value.main_frame.id().get().to_string()),
        page_descriptor
            .map(|value| value.main_frame.url())
            .transpose()
            .map_err(binding_error)?,
        error_descriptor,
        console_descriptor,
        dialog_descriptor,
        file_chooser_descriptor,
        frame_id,
    ))
}

#[cfg(feature = "page-events")]
fn publication_error(error: PublicationDrainError) -> PyErr {
    binding_error(HostError::Backend(error.to_string()))
}

fn publish_frame_fence(
    py: Python<'_>,
    fence: FramePublicationFence,
    callback: &LifecycleCallback,
) -> PyResult<()> {
    #[cfg(not(feature = "page-events"))]
    {
        let _ = (py, callback);
        fence.publish().map_err(binding_error)
    }
    #[cfg(feature = "page-events")]
    {
        loop {
            let mut drain = fence.try_begin().map_err(publication_error)?;
            loop {
                match drain.publish_next() {
                    Ok(PublicationProgress::Published) => {}
                    Ok(PublicationProgress::Complete) => return Ok(()),
                    Err(PublicationDrainError::NoticePending) => {
                        let descriptor = drain
                            .with_next_notice(|view| lifecycle_descriptor(py, view))
                            .map_err(publication_error)??;
                        drain.commit_notice().map_err(publication_error)?;
                        // Python's callback only schedules delivery on its event loop.
                        // A Python C call may release the GIL, so acknowledge and
                        // release the exclusive native owner before invoking it.
                        drain
                            .acknowledge_notice(LifecycleDelivery::Delivered)
                            .map_err(publication_error)?;
                        drop(drain);
                        let callback = callback
                            .lock()
                            .map_err(|error| Error::new_err(error.to_string()))?
                            .as_ref()
                            .map(|callback| callback.clone_ref(py));
                        if let Some(callback) = callback {
                            callback.call1(py, descriptor)?;
                        }
                        break;
                    }
                    Err(error) => return Err(publication_error(error)),
                }
            }
        }
    }
}

struct PythonCommandContext<'a> {
    owner: &'a Arc<EnvironmentOwner>,
    host: &'a Arc<HostSession>,
    lifecycle_callback: &'a LifecycleCallback,
}

fn async_python_command<'py, T, F, C>(
    py: Python<'py>,
    context: PythonCommandContext<'_>,
    lane: FfiLane,
    allowance: u64,
    future: F,
    convert: C,
) -> PyResult<Bound<'py, PyAny>>
where
    T: Send + 'static,
    F: std::future::Future<Output = Result<T, HostError>> + Send + 'static,
    C: FnOnce(Python<'_>, T) -> PyResult<Py<PyAny>> + Send + 'static,
{
    let mut execution = context
        .owner
        .ffi
        .admit(lane, allowance)
        .map_err(binding_error)?;
    let publications = context.host.frame_publications();
    let lifecycle_callback = Arc::clone(context.lifecycle_callback);
    let future = async move {
        let outcome = future.await;
        Ok((outcome, publications.fence()?))
    };
    execution
        .reserve_future(std::mem::size_of_val(&future))
        .map_err(binding_error)?;
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let ticket = execution.run(future).await;
        ticket
            .deliver(|outcome| {
                let (outcome, fence) = outcome.map_err(binding_error)?;
                Python::attach(|py| {
                    publish_frame_fence(py, fence, &lifecycle_callback)?;
                    let value = outcome.map_err(binding_error)?;
                    convert(py, value)
                })
            })
            .map_err(binding_error)?
    })
}

impl NativeSession {
    fn publish_fence(&self, py: Python<'_>, fence: FramePublicationFence) -> PyResult<()> {
        publish_frame_fence(py, fence, &self.lifecycle_callback)
    }

    #[cfg(feature = "timeout-settings")]
    fn sync_unit_command(
        &self,
        operation: impl FnOnce(&HostSession) -> Result<(), HostError>,
    ) -> PyResult<()> {
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Command, SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let ticket = execution.run_sync(move || operation(&host));
        ticket
            .deliver(|outcome| outcome.map_err(binding_error))
            .map_err(binding_error)?
    }

    #[cfg(feature = "timeout-settings")]
    fn sync_u32_command(
        &self,
        operation: impl FnOnce(&HostSession) -> Result<u32, HostError>,
    ) -> PyResult<u32> {
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Command, SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let ticket = execution.run_sync(move || operation(&host));
        ticket
            .deliver(|outcome| outcome.map_err(binding_error))
            .map_err(binding_error)?
    }

    fn async_command<'py, T, F, C>(
        &self,
        py: Python<'py>,
        lane: FfiLane,
        allowance: u64,
        future: F,
        convert: C,
    ) -> PyResult<Bound<'py, PyAny>>
    where
        T: Send + 'static,
        F: std::future::Future<Output = Result<T, HostError>> + Send + 'static,
        C: FnOnce(Python<'_>, T) -> PyResult<Py<PyAny>> + Send + 'static,
    {
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &self.host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            lane,
            allowance,
            future,
            convert,
        )
    }

    fn prepare_locator_with_strict(
        &self,
        frame_id: &str,
        selector: &str,
        strict: bool,
    ) -> PyResult<HostLocator> {
        let frame_id = bounded_string(frame_id, MAX_OBJECT_ID_BYTES, "Frame ID")?;
        let query = prepare_locator_query(&self.owner, selector)?;
        let query = if strict {
            query
        } else {
            query.nth(0).map_err(binding_error)?
        };
        self.host.locator(&frame_id, query).map_err(binding_error)
    }

    fn prepare_locator(&self, frame_id: &str, selector: &str) -> PyResult<HostLocator> {
        self.prepare_locator_with_strict(frame_id, selector, true)
    }

    fn validate_selector_impl(&self, frame_id: &str, selector: &str) -> PyResult<()> {
        self.prepare_locator(frame_id, selector)?
            .validate()
            .map_err(binding_error)
    }

    fn locator_unit_action<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
        timeout_ms: u32,
        action: LocatorAction,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.locator_unit_action_with_strict(py, frame_id, selector, timeout_ms, true, action)
    }

    fn locator_unit_action_with_strict<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
        timeout_ms: u32,
        strict: bool,
        action: LocatorAction,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit::<(Result<(), HostError>, FramePublicationFence)>(
                FfiLane::Command,
                ACTION_ALLOWANCE,
            )
            .map_err(binding_error)?;
        let locator = self.prepare_locator_with_strict(frame_id, selector, strict)?;
        #[cfg(feature = "selector-registration")]
        let test_id_attributes = self
            .owner
            .selectors
            .test_id_attributes()
            .map_err(binding_error)?;
        let publications = self.host.frame_publications();
        let lifecycle_callback = Arc::clone(&self.lifecycle_callback);
        let future = async move {
            #[cfg(feature = "selector-registration")]
            let action_outcome = locator
                .act_with_strict_error_test_id_attribute(action, timeout_ms, test_id_attributes)
                .await;
            #[cfg(not(feature = "selector-registration"))]
            let action_outcome = locator.act(action, timeout_ms).await;
            let outcome = action_outcome.and_then(|value| match value {
                ActionValue::Unit => Ok(()),
                ActionValue::Strings(_) => Err(HostError::Backend(
                    "unit locator action returned string values".into(),
                )),
            });
            Ok((outcome, publications.fence()?))
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            ticket
                .deliver(|outcome| {
                    let (outcome, fence) = outcome.map_err(binding_error)?;
                    Python::attach(|py| {
                        publish_frame_fence(py, fence, &lifecycle_callback)?;
                        outcome.map_err(binding_error)?;
                        Ok(py.None())
                    })
                })
                .map_err(binding_error)?
        })
    }
}

impl NativeLocator {
    fn derived(&self, locator: HostLocator) -> Self {
        Self {
            locator,
            owner: Arc::clone(&self.owner),
            lifecycle_callback: Arc::clone(&self.lifecycle_callback),
        }
    }

    fn unit_action<'py>(
        &self,
        py: Python<'py>,
        timeout_ms: u32,
        action: LocatorAction,
    ) -> PyResult<Bound<'py, PyAny>> {
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        #[cfg(feature = "selector-registration")]
        let test_id_attributes = self
            .owner
            .selectors
            .test_id_attributes()
            .map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            ACTION_ALLOWANCE,
            async move {
                #[cfg(feature = "selector-registration")]
                let action_outcome = locator
                    .act_with_strict_error_test_id_attribute(action, timeout_ms, test_id_attributes)
                    .await;
                #[cfg(not(feature = "selector-registration"))]
                let action_outcome = locator.act(action, timeout_ms).await;
                action_outcome.and_then(|value| match value {
                    ActionValue::Unit => Ok(()),
                    ActionValue::Strings(_) => Err(HostError::Backend(
                        "unit locator action returned string values".into(),
                    )),
                })
            },
            |py, ()| Ok(py.None()),
        )
    }
}

#[cfg(feature = "handles")]
impl NativeFrameLocator {
    fn derived_locator(&self, locator: HostLocator) -> NativeLocator {
        NativeLocator {
            locator,
            owner: Arc::clone(&self.owner),
            lifecycle_callback: Arc::clone(&self.lifecycle_callback),
        }
    }

    fn derived_frame(&self, locator: HostFrameLocator) -> Self {
        Self {
            locator,
            owner: Arc::clone(&self.owner),
            lifecycle_callback: Arc::clone(&self.lifecycle_callback),
        }
    }
}

#[cfg(feature = "handles")]
#[pymethods]
impl NativeFrameLocator {
    fn locator(&self, selector: &str) -> PyResult<NativeLocator> {
        let query = prepare_locator_query(&self.owner, selector)?;
        self.locator
            .locator(query)
            .map(|locator| self.derived_locator(locator))
            .map_err(binding_error)
    }

    fn locator_native(&self, locator: PyRef<'_, NativeLocator>) -> PyResult<NativeLocator> {
        self.locator
            .locator_from(&locator.locator)
            .map(|locator| self.derived_locator(locator))
            .map_err(binding_error)
    }

    #[pyo3(signature = (kind, source, exact=false, flags=None))]
    fn semantic(
        &self,
        kind: &str,
        source: &str,
        exact: bool,
        flags: Option<&str>,
    ) -> PyResult<NativeLocator> {
        let query = prepare_semantic_query(&self.owner, kind, source, exact, flags)?;
        self.locator
            .locator(query)
            .map(|locator| self.derived_locator(locator))
            .map_err(binding_error)
    }

    #[pyo3(signature = (
        role,
        name=None,
        name_flags=None,
        exact=false,
        checked=None,
        disabled=None,
        expanded=None,
        include_hidden=false,
        level=None,
        pressed=None,
        selected=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn role(
        &self,
        role: &str,
        name: Option<&str>,
        name_flags: Option<&str>,
        exact: bool,
        checked: Option<bool>,
        disabled: Option<bool>,
        expanded: Option<bool>,
        include_hidden: bool,
        level: Option<u32>,
        pressed: Option<bool>,
        selected: Option<bool>,
    ) -> PyResult<NativeLocator> {
        let query = prepare_role_query(
            &self.owner,
            role,
            name,
            name_flags,
            exact,
            checked,
            disabled,
            expanded,
            include_hidden,
            level,
            pressed,
            selected,
        )?;
        self.locator
            .locator(query)
            .map(|locator| self.derived_locator(locator))
            .map_err(binding_error)
    }

    fn frame_locator(&self, selector: &str) -> PyResult<Self> {
        let query = prepare_locator_query(&self.owner, selector)?;
        self.locator
            .frame_locator(query)
            .map(|locator| self.derived_frame(locator))
            .map_err(binding_error)
    }

    fn owner(&self) -> PyResult<NativeLocator> {
        self.locator
            .owner()
            .map(|locator| self.derived_locator(locator))
            .map_err(binding_error)
    }

    fn nth(&self, index: i32) -> PyResult<Self> {
        self.locator
            .nth(index)
            .map(|locator| self.derived_frame(locator))
            .map_err(binding_error)
    }
}

#[pymethods]
impl NativeLocator {
    fn locator(&self, selector: &str) -> PyResult<Self> {
        let query = prepare_locator_query(&self.owner, selector)?;
        self.locator
            .combine_query(query, QueryCombination::Within)
            .map(|locator| self.derived(locator))
            .map_err(binding_error)
    }

    fn nth(&self, index: i32) -> PyResult<Self> {
        self.locator
            .nth(index)
            .map(|locator| self.derived(locator))
            .map_err(binding_error)
    }

    #[cfg(feature = "handles")]
    fn frame_locator(&self, selector: &str) -> PyResult<NativeFrameLocator> {
        let query = prepare_locator_query(&self.owner, selector)?;
        self.locator
            .combine_query(query, QueryCombination::Within)
            .and_then(|locator| locator.content_frame())
            .map(|locator| NativeFrameLocator {
                locator,
                owner: Arc::clone(&self.owner),
                lifecycle_callback: Arc::clone(&self.lifecycle_callback),
            })
            .map_err(binding_error)
    }

    #[cfg(feature = "handles")]
    fn content_frame(&self) -> PyResult<NativeFrameLocator> {
        self.locator
            .content_frame()
            .map(|locator| NativeFrameLocator {
                locator,
                owner: Arc::clone(&self.owner),
                lifecycle_callback: Arc::clone(&self.lifecycle_callback),
            })
            .map_err(binding_error)
    }

    fn visible(&self, visible: bool) -> PyResult<Self> {
        self.locator
            .visible(visible)
            .map(|locator| self.derived(locator))
            .map_err(binding_error)
    }

    fn filter_text(&self, text: &str, exclude: bool) -> PyResult<Self> {
        if text.len() > MAX_LOCATOR_INPUT_BYTES {
            return Err(binding_error(HostError::InvalidArgument(format!(
                "Locator text filter exceeds {MAX_LOCATOR_INPUT_BYTES} UTF-8 bytes"
            ))));
        }
        self.locator
            .filter_text(text, exclude)
            .map(|locator| self.derived(locator))
            .map_err(binding_error)
    }

    fn filter_regex(&self, source: &str, flags: &str, exclude: bool) -> PyResult<Self> {
        if source.len() > MAX_LOCATOR_INPUT_BYTES || flags.len() > 8 {
            return Err(binding_error(HostError::InvalidArgument(
                "Locator regex filter exceeds its input boundary".into(),
            )));
        }
        self.locator
            .filter_regex(source, flags, exclude)
            .map(|locator| self.derived(locator))
            .map_err(binding_error)
    }

    fn combine(&self, other: PyRef<'_, NativeLocator>, kind: &str) -> PyResult<Self> {
        let combination = match kind {
            "and" => QueryCombination::And,
            "or" => QueryCombination::Or,
            "within" => QueryCombination::Within,
            "has" => QueryCombination::Has,
            "hasNot" => QueryCombination::HasNot,
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "invalid Locator combination".into(),
                )));
            }
        };
        self.locator
            .combine(&other.locator, combination)
            .map(|locator| self.derived(locator))
            .map_err(binding_error)
    }

    #[pyo3(signature = (kind, source, exact=false, flags=None))]
    fn semantic(
        &self,
        kind: &str,
        source: &str,
        exact: bool,
        flags: Option<&str>,
    ) -> PyResult<Self> {
        let query = prepare_semantic_query(&self.owner, kind, source, exact, flags)?;
        self.locator
            .combine_query(query, QueryCombination::Within)
            .map(|locator| self.derived(locator))
            .map_err(binding_error)
    }

    #[pyo3(signature = (
        role,
        name=None,
        name_flags=None,
        exact=false,
        checked=None,
        disabled=None,
        expanded=None,
        include_hidden=false,
        level=None,
        pressed=None,
        selected=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn role(
        &self,
        role: &str,
        name: Option<&str>,
        name_flags: Option<&str>,
        exact: bool,
        checked: Option<bool>,
        disabled: Option<bool>,
        expanded: Option<bool>,
        include_hidden: bool,
        level: Option<u32>,
        pressed: Option<bool>,
        selected: Option<bool>,
    ) -> PyResult<Self> {
        let query = prepare_role_query(
            &self.owner,
            role,
            name,
            name_flags,
            exact,
            checked,
            disabled,
            expanded,
            include_hidden,
            level,
            pressed,
            selected,
        )?;
        self.locator
            .combine_query(query, QueryCombination::Within)
            .map(|locator| self.derived(locator))
            .map_err(binding_error)
    }

    #[pyo3(signature = (timeout_ms=30_000, steps=None, force=None))]
    fn click<'py>(
        &self,
        py: Python<'py>,
        timeout_ms: u32,
        steps: Option<&Bound<'_, PyAny>>,
        force: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let options = locator_click_options(steps, force)?;
        self.unit_action(py, timeout_ms, LocatorAction::Click(options))
    }

    #[cfg(feature = "touchscreen-input")]
    #[pyo3(signature = (timeout_ms=30_000, options=None))]
    fn tap<'py>(
        &self,
        py: Python<'py>,
        timeout_ms: u32,
        options: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let options = options
            .map(python_tap_options)
            .transpose()?
            .unwrap_or_default();
        self.unit_action(py, timeout_ms, LocatorAction::Tap(options))
    }

    #[pyo3(signature = (timeout_ms=30_000))]
    fn dblclick<'py>(&self, py: Python<'py>, timeout_ms: u32) -> PyResult<Bound<'py, PyAny>> {
        self.unit_action(
            py,
            timeout_ms,
            LocatorAction::Click(
                ClickOptionsV2::new().with_click_count(NonZeroU8::new(2).expect("two is nonzero")),
            ),
        )
    }

    #[cfg(feature = "locator-bounding-box")]
    #[pyo3(signature = (timeout_ms=30_000))]
    fn bounding_box<'py>(&self, py: Python<'py>, timeout_ms: u32) -> PyResult<Bound<'py, PyAny>> {
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move { locator.bounding_box(timeout_ms).await },
            |py, value: Option<LocatorBoundingBox>| {
                let Some(value) = value else {
                    return Ok(py.None());
                };
                let result = PyDict::new(py);
                result.set_item("x", value.x)?;
                result.set_item("y", value.y)?;
                result.set_item("width", value.width)?;
                result.set_item("height", value.height)?;
                Ok(result.into_any().unbind())
            },
        )
    }

    #[cfg(feature = "locator-screenshot")]
    #[pyo3(signature = (format=-1, path=None, quality=None, scale=0, timeout_ms=30_000, mask=None))]
    #[allow(clippy::too_many_arguments)]
    fn screenshot<'py>(
        &self,
        py: Python<'py>,
        format: i8,
        path: Option<&str>,
        quality: Option<u8>,
        scale: u8,
        timeout_ms: u32,
        mask: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let path = path
            .map(|value| bounded_string(value, MAX_PATH_BYTES, "screenshot output path"))
            .transpose()?;
        let options = screenshot_options(format, path.as_deref(), quality, false, scale)?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        let masks = screenshot_masks(mask, &host)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            pwright_node_host::screenshot::FFI_ALLOWANCE,
            async move {
                let output = locator
                    .screenshot_with_masks(options, masks, timeout_ms)
                    .await?;
                if let Some(path) = path {
                    tokio::fs::write(path, output.as_bytes())
                        .await
                        .map_err(|error| {
                            HostError::Backend(format!(
                                "Locator.screenshot could not write output: {error}"
                            ))
                        })?;
                }
                Ok(output)
            },
            |py, output| Ok(PyBytes::new(py, output.as_bytes()).into_any().unbind()),
        )
    }

    #[cfg(feature = "file-input")]
    #[pyo3(signature = (base, paths, timeout_ms=30_000))]
    fn set_input_files_paths<'py>(
        &self,
        py: Python<'py>,
        base: &str,
        paths: &Bound<'py, PyAny>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let base = bounded_string(
            base,
            pwright_node_host::file_input::profile::PATH_BYTES,
            "file-input base",
        )?;
        let paths = paths.cast::<PyList>().map_err(|_| {
            binding_error(HostError::InvalidArgument(
                "file-input paths must be a list".into(),
            ))
        })?;
        if paths.len() > pwright_node_host::file_input::profile::ITEMS {
            return Err(binding_error(HostError::InvalidArgument(
                "file-input count exceeds selected limit".into(),
            )));
        }
        let mut values = Vec::with_capacity(paths.len());
        for path in paths.iter() {
            let path = path.cast::<PyString>().map_err(|_| {
                binding_error(HostError::InvalidArgument(
                    "file-input path must be a string".into(),
                ))
            })?;
            let path = path.to_cow()?;
            values.push(bounded_string(
                &path,
                pwright_node_host::file_input::profile::PATH_BYTES,
                "file-input path",
            )?);
        }
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            ACTION_ALLOWANCE,
            async move {
                let views = values.iter().map(String::as_str).collect::<Vec<_>>();
                locator
                    .set_input_files_paths(Path::new(&base), &views, timeout_ms)
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "file-input")]
    #[pyo3(signature = (payloads, timeout_ms=30_000))]
    fn set_input_file_payloads<'py>(
        &self,
        py: Python<'py>,
        payloads: &Bound<'py, PyAny>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        struct Payload {
            name: String,
            mime_type: String,
            buffer: Py<PyBytes>,
        }

        let payloads = payloads.cast::<PyList>().map_err(|_| {
            binding_error(HostError::InvalidArgument(
                "file payloads must be a list".into(),
            ))
        })?;
        if payloads.len() > pwright_node_host::file_input::profile::ITEMS {
            return Err(binding_error(HostError::InvalidArgument(
                "file-input count exceeds selected limit".into(),
            )));
        }
        let mut values = Vec::with_capacity(payloads.len());
        for payload in payloads.iter() {
            let payload = payload.cast::<PyDict>().map_err(|_| {
                binding_error(HostError::InvalidArgument(
                    "file payload must be a mapping".into(),
                ))
            })?;
            let name = payload
                .get_item("name")?
                .ok_or_else(|| {
                    binding_error(HostError::InvalidArgument(
                        "file payload name is required".into(),
                    ))
                })?
                .cast_into::<PyString>()
                .map_err(|_| {
                    binding_error(HostError::InvalidArgument(
                        "file payload name must be a string".into(),
                    ))
                })?;
            let mime_type = payload
                .get_item("mimeType")?
                .ok_or_else(|| {
                    binding_error(HostError::InvalidArgument(
                        "file payload MIME is required".into(),
                    ))
                })?
                .cast_into::<PyString>()
                .map_err(|_| {
                    binding_error(HostError::InvalidArgument(
                        "file payload MIME must be a string".into(),
                    ))
                })?;
            let buffer = payload
                .get_item("buffer")?
                .ok_or_else(|| {
                    binding_error(HostError::InvalidArgument(
                        "file payload buffer is required".into(),
                    ))
                })?
                .cast_into::<PyBytes>()
                .map_err(|_| {
                    binding_error(HostError::InvalidArgument(
                        "file payload buffer must be bytes".into(),
                    ))
                })?;
            let name = name.to_cow()?;
            let mime_type = mime_type.to_cow()?;
            values.push(Payload {
                name: bounded_string(
                    &name,
                    pwright_node_host::file_input::profile::NAME_BYTES,
                    "file payload name",
                )?,
                mime_type: bounded_string(
                    &mime_type,
                    pwright_node_host::file_input::profile::MIME_BYTES,
                    "file payload MIME",
                )?,
                buffer: buffer.unbind(),
            });
        }
        let host = self.locator.host().map_err(binding_error)?;
        let inputs = values
            .iter()
            .map(|payload| pwright_node_host::file_input::MemoryFile {
                name: &payload.name,
                mime_type: Some(&payload.mime_type),
                buffer: payload.buffer.bind(py).as_bytes(),
            })
            .collect::<Vec<_>>();
        let input = host.prepare_file_payloads(&inputs).map_err(binding_error)?;
        let locator = self.locator.clone();
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            ACTION_ALLOWANCE,
            async move { locator.set_input_files(input, timeout_ms).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "file-input"))]
    #[pyo3(signature = (_base, _paths, _timeout_ms=30_000))]
    fn set_input_files_paths<'py>(
        &self,
        _py: Python<'py>,
        _base: &str,
        _paths: &Bound<'py, PyAny>,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Locator.set_input_files capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "file-input"))]
    #[pyo3(signature = (_payloads, _timeout_ms=30_000))]
    fn set_input_file_payloads<'py>(
        &self,
        _py: Python<'py>,
        _payloads: &Bound<'py, PyAny>,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Locator.set_input_files capability is not installed".into(),
        )))
    }

    #[cfg(feature = "handles")]
    #[pyo3(signature = (timeout_ms=30_000))]
    fn element_handle<'py>(&self, py: Python<'py>, timeout_ms: u32) -> PyResult<Bound<'py, PyAny>> {
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        let preview_host = Arc::clone(&host);
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                let value = locator.element_handle(timeout_ms).await?;
                handles::with_node_preview(&preview_host, value).await
            },
            |py, (value, preview)| {
                Ok(
                    Py::new(py, handles::NativeHandle::new_with_preview(value, preview))?
                        .into_any(),
                )
            },
        )
    }

    #[cfg(not(feature = "handles"))]
    #[pyo3(signature = (_timeout_ms=30_000))]
    fn element_handle<'py>(
        &self,
        _py: Python<'py>,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Locator.element_handle capability is not installed".into(),
        )))
    }

    #[cfg(feature = "locator-highlight")]
    fn highlight<'py>(&self, py: Python<'py>, description: &str) -> PyResult<Bound<'py, PyAny>> {
        let description = bounded_string(description, 64 * 1024, "Locator description")?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move { locator.highlight(description).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "locator-highlight"))]
    fn highlight<'py>(&self, _py: Python<'py>, _description: &str) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Locator.highlight capability is not installed".into(),
        )))
    }

    #[cfg(feature = "locator-dom-actions")]
    #[pyo3(signature = (event_type, event_init=None, timeout_ms=30_000))]
    fn dispatch_event<'py>(
        &self,
        py: Python<'py>,
        event_type: &str,
        event_init: Option<&Bound<'py, PyAny>>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let event_type = bounded_string(
            event_type,
            pwright_node_host::locator::MAX_EVENT_TYPE_BYTES,
            "event type",
        )?;
        if event_type.is_empty() {
            return Err(binding_error(HostError::InvalidArgument(
                "event type must not be empty".into(),
            )));
        }
        let event_init = evaluation_value::from_python(event_init)?;
        if !matches!(
            event_init,
            pwright_node_host::evaluation::EvaluationValue::Null
                | pwright_node_host::evaluation::EvaluationValue::Object { .. }
        ) {
            return Err(binding_error(HostError::InvalidArgument(
                "event initializer must be a dictionary or None".into(),
            )));
        }
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                locator
                    .dispatch_event(event_type, event_init, timeout_ms)
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "locator-dom-actions"))]
    #[pyo3(signature = (_event_type, _event_init=None, _timeout_ms=30_000))]
    fn dispatch_event<'py>(
        &self,
        _py: Python<'py>,
        _event_type: &str,
        _event_init: Option<&Bound<'py, PyAny>>,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Locator.dispatch_event capability is not installed".into(),
        )))
    }

    #[cfg(all(
        feature = "locator-dom-actions",
        feature = "handles",
        feature = "element-handle-dom"
    ))]
    #[pyo3(signature = (event_type, event_init=None, timeout_ms=30_000))]
    fn dispatch_event_owned<'py>(
        &self,
        py: Python<'py>,
        event_type: &str,
        event_init: Option<&Bound<'py, PyAny>>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let event_type = bounded_string(
            event_type,
            pwright_node_host::locator::MAX_EVENT_TYPE_BYTES,
            "event type",
        )?;
        let event_init = evaluation_value::from_python_owned(event_init)?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                locator
                    .dispatch_event_owned(event_type, event_init, timeout_ms)
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(all(
        feature = "locator-dom-actions",
        feature = "handles",
        feature = "element-handle-dom"
    )))]
    #[pyo3(signature = (_event_type, _event_init=None, _timeout_ms=30_000))]
    fn dispatch_event_owned<'py>(
        &self,
        _py: Python<'py>,
        _event_type: &str,
        _event_init: Option<&Bound<'py, PyAny>>,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Locator.dispatch_event with live Handles requires Handle capabilities".into(),
        )))
    }

    #[cfg(feature = "locator-dom-actions")]
    #[pyo3(signature = (timeout_ms=30_000, force=false))]
    fn select_text<'py>(
        &self,
        py: Python<'py>,
        timeout_ms: u32,
        force: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move { locator.select_text(timeout_ms, force).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "locator-dom-actions"))]
    #[pyo3(signature = (_timeout_ms=30_000, _force=false))]
    fn select_text<'py>(
        &self,
        _py: Python<'py>,
        _timeout_ms: u32,
        _force: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Locator.select_text capability is not installed".into(),
        )))
    }

    #[pyo3(signature = (value, timeout_ms=30_000, force=false))]
    fn fill<'py>(
        &self,
        py: Python<'py>,
        value: &str,
        timeout_ms: u32,
        force: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let value = bounded_string(
            value,
            pwright_node_host::locator::action::MAX_VALUE_BYTES,
            "fill value",
        )?;
        self.unit_action(
            py,
            timeout_ms,
            LocatorAction::Fill {
                value,
                options: FillOptions::new().with_force(force),
            },
        )
    }

    #[pyo3(signature = (key, timeout_ms=30_000, delay_ms=None, no_wait_after=false))]
    fn press<'py>(
        &self,
        py: Python<'py>,
        key: &str,
        timeout_ms: u32,
        delay_ms: Option<u32>,
        no_wait_after: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let key = PressKey::new(key)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
        let mut options = PressOptions::new().with_no_wait_after(no_wait_after);
        if let Some(delay_ms) = delay_ms {
            options = options.with_delay(std::time::Duration::from_millis(u64::from(delay_ms)));
        }
        self.unit_action(py, timeout_ms, LocatorAction::Press { key, options })
    }

    #[pyo3(signature = (checked, timeout_ms=30_000, force=None, trial=None))]
    fn set_checked<'py>(
        &self,
        py: Python<'py>,
        checked: bool,
        timeout_ms: u32,
        force: Option<bool>,
        trial: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut options = SetCheckedOptions::new();
        if let Some(force) = force {
            options = options.with_force(force);
        }
        if let Some(trial) = trial {
            options = options.with_trial(trial);
        }
        self.unit_action(
            py,
            timeout_ms,
            LocatorAction::SetChecked { checked, options },
        )
    }

    #[pyo3(signature = (timeout_ms=30_000))]
    fn focus<'py>(&self, py: Python<'py>, timeout_ms: u32) -> PyResult<Bound<'py, PyAny>> {
        self.unit_action(py, timeout_ms, LocatorAction::Focus)
    }

    #[pyo3(signature = (timeout_ms=30_000))]
    fn blur<'py>(&self, py: Python<'py>, timeout_ms: u32) -> PyResult<Bound<'py, PyAny>> {
        self.unit_action(py, timeout_ms, LocatorAction::Blur)
    }

    #[pyo3(signature = (timeout_ms=30_000, force=false, trial=false))]
    fn hover<'py>(
        &self,
        py: Python<'py>,
        timeout_ms: u32,
        force: bool,
        trial: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.unit_action(
            py,
            timeout_ms,
            LocatorAction::Hover(HoverOptions {
                force,
                trial,
                ..HoverOptions::new()
            }),
        )
    }

    #[pyo3(signature = (timeout_ms=30_000))]
    fn scroll_into_view_if_needed<'py>(
        &self,
        py: Python<'py>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.unit_action(py, timeout_ms, LocatorAction::ScrollIntoViewIfNeeded)
    }

    #[pyo3(signature = (text, timeout_ms=30_000, delay_ms=None))]
    fn press_sequentially<'py>(
        &self,
        py: Python<'py>,
        text: &str,
        timeout_ms: u32,
        delay_ms: Option<u32>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let text = bounded_string(
            text,
            pwright_node_host::locator::action::MAX_VALUE_BYTES,
            "sequential text",
        )?;
        self.unit_action(
            py,
            timeout_ms,
            LocatorAction::PressSequentially {
                text,
                delay: delay_ms.map(|value| std::time::Duration::from_millis(u64::from(value))),
            },
        )
    }

    #[pyo3(signature = (value, timeout_ms=30_000, force=None))]
    fn select_option<'py>(
        &self,
        py: Python<'py>,
        value: &str,
        timeout_ms: u32,
        force: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let value = bounded_string(
            value,
            pwright_node_host::locator::action::MAX_OPTION_BYTES,
            "select option value",
        )?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            ACTION_ALLOWANCE,
            async move {
                locator
                    .act(
                        LocatorAction::SelectOption {
                            values: vec![SelectOptionValue::value_or_label(value)],
                            options: SelectOptionOptions { force },
                        },
                        timeout_ms,
                    )
                    .await
                    .and_then(|value| match value {
                        ActionValue::Strings(values) => Ok(values),
                        ActionValue::Unit => Err(HostError::Backend(
                            "select option locator action omitted string values".into(),
                        )),
                    })
            },
            |py, values| Ok(PyList::new(py, values)?.into_any().unbind()),
        )
    }

    #[cfg(feature = "page-select-option")]
    #[pyo3(signature = (values, timeout_ms=30_000, force=None, element=None))]
    fn select_options<'py>(
        &self,
        py: Python<'py>,
        values: &Bound<'py, PyList>,
        timeout_ms: u32,
        force: Option<bool>,
        element: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let values = page_select_option::descriptors(values)?;
        if element.is_some() && values.len() == pwright_node_host::locator::action::MAX_OPTIONS {
            return Err(binding_error(HostError::InvalidArgument(
                "selectOption exceeds 1024 descriptors".into(),
            )));
        }
        #[cfg(feature = "handles")]
        let element = element
            .map(|value| {
                value
                    .extract::<PyRef<'_, handles::NativeHandle>>()?
                    .read_frame()
                    .map_err(binding_error)
            })
            .transpose()?;
        #[cfg(not(feature = "handles"))]
        if element.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "Page.select_option(element=...) requires handles".into(),
            )));
        }
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        #[cfg(feature = "handles")]
        let host_for_handle = Arc::clone(&host);
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            ACTION_ALLOWANCE,
            async move {
                #[cfg(feature = "handles")]
                let mut values = values;
                #[cfg(feature = "handles")]
                if let Some(element) = element {
                    let index = page_select_option::option_handle_index(
                        &host_for_handle,
                        &locator,
                        &element,
                        timeout_ms,
                    )
                    .await?;
                    values.push(SelectOptionValue::new().with_index(index));
                }
                locator
                    .act(
                        LocatorAction::SelectOption {
                            values,
                            options: SelectOptionOptions { force },
                        },
                        timeout_ms,
                    )
                    .await
                    .and_then(|value| match value {
                        ActionValue::Strings(values) => Ok(values),
                        ActionValue::Unit => Err(HostError::Backend(
                            "select option locator action omitted string values".into(),
                        )),
                    })
            },
            |py, values| Ok(PyList::new(py, values)?.into_any().unbind()),
        )
    }

    #[pyo3(signature = (operation, timeout_ms=30_000, attribute=None))]
    fn read<'py>(
        &self,
        py: Python<'py>,
        operation: &str,
        timeout_ms: u32,
        attribute: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let operation = operation.parse::<LocatorRead>().map_err(binding_error)?;
        let attribute = attribute
            .map(|value| bounded_string(value, 64 * 1024, "attribute"))
            .transpose()?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move { locator.read(operation, timeout_ms, attribute).await },
            locator_value_to_python,
        )
    }

    #[pyo3(signature = (source, argument=None, timeout_ms=30_000))]
    fn evaluate<'py>(
        &self,
        py: Python<'py>,
        source: &str,
        argument: Option<&Bound<'py, PyAny>>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pwright_node_host::evaluation::{EvaluateParams, FFI_ALLOWANCE};

        if !evaluation_value::is_function_source(source) {
            return Err(binding_error(HostError::InvalidArgument(
                "Locator.evaluate expects a JavaScript function".into(),
            )));
        }
        let source = bounded_string(
            source,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "source",
        )?;
        let argument = evaluation_value::from_python(argument)?;
        let params = EvaluateParams::function(source, argument);
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            FFI_ALLOWANCE,
            async move {
                locator
                    .evaluate_with(timeout_ms, move || async move { Ok(params) })
                    .await
            },
            evaluation_value::to_python,
        )
    }

    #[pyo3(signature = (source, argument=None))]
    fn evaluate_all<'py>(
        &self,
        py: Python<'py>,
        source: &str,
        argument: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pwright_node_host::evaluation::{EvaluateParams, FFI_ALLOWANCE};

        if !evaluation_value::is_function_source(source) {
            return Err(binding_error(HostError::InvalidArgument(
                "Locator.evaluate_all expects a JavaScript function".into(),
            )));
        }
        let source = bounded_string(
            source,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "source",
        )?;
        let argument = evaluation_value::from_python(argument)?;
        let params = EvaluateParams::function(source, argument);
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            FFI_ALLOWANCE,
            async move {
                locator
                    .evaluate_all_with(move || async move { Ok(params) })
                    .await
            },
            evaluation_value::to_python,
        )
    }

    #[cfg(feature = "handles")]
    #[pyo3(signature = (source, argument=None, timeout_ms=30_000))]
    fn evaluate_handle<'py>(
        &self,
        py: Python<'py>,
        source: &str,
        argument: Option<&Bound<'py, PyAny>>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let is_function = evaluation_value::is_function_source(source);
        let source = bounded_string(
            source,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "source",
        )?;
        let argument = evaluation_value::from_python(argument)?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                locator
                    .evaluate_handle(source, is_function, argument, timeout_ms)
                    .await
            },
            |py, value| {
                Ok(Py::new(
                    py,
                    handles::NativeHandle::new_with_preview(value.handle, value.preview),
                )?
                .into_any())
            },
        )
    }

    #[cfg(not(feature = "handles"))]
    #[pyo3(signature = (_source, _argument=None, _timeout_ms=30_000))]
    fn evaluate_handle<'py>(
        &self,
        _py: Python<'py>,
        _source: &str,
        _argument: Option<&Bound<'py, PyAny>>,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Locator.evaluate_handle capability is not installed".into(),
        )))
    }

    #[pyo3(signature = (state="visible", timeout_ms=30_000))]
    fn wait_for<'py>(
        &self,
        py: Python<'py>,
        state: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let state = locator_wait_state(state).map_err(binding_error)?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move { locator.wait_for(state, timeout_ms).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "handles")]
    #[pyo3(signature = (state="visible", timeout_ms=30_000))]
    fn wait_for_element<'py>(
        &self,
        py: Python<'py>,
        state: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let state = locator_wait_state(state).map_err(binding_error)?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move { locator.wait_for_element(state, timeout_ms, true).await },
            |py, value| match value {
                Some(value) => Ok(Py::new(py, handles::NativeHandle::new(value))?.into_any()),
                None => Ok(py.None()),
            },
        )
    }

    #[cfg(not(feature = "handles"))]
    #[pyo3(signature = (_state="visible", _timeout_ms=30_000))]
    fn wait_for_element<'py>(
        &self,
        _py: Python<'py>,
        _state: &str,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Locator.wait_for_element capability is not installed".into(),
        )))
    }

    #[pyo3(signature = (visible=true, timeout_ms=5_000, is_not=false))]
    fn expect_visibility<'py>(
        &self,
        py: Python<'py>,
        visible: bool,
        timeout_ms: u32,
        is_not: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move { locator.expect_visibility(timeout_ms, visible, is_not).await },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("received", result.received)?;
                value.set_item("preview", result.preview)?;
                value.set_item("missing", result.missing)?;
                value.set_item("timed_out", result.timed_out)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (
        expected,
        timeout_ms=5_000,
        is_not=false,
        substring=false,
        normalize_whitespace=true,
        ignore_case=false,
        use_inner_text=false
    ))]
    #[allow(clippy::too_many_arguments)]
    fn expect_text<'py>(
        &self,
        py: Python<'py>,
        expected: &str,
        timeout_ms: u32,
        is_not: bool,
        substring: bool,
        normalize_whitespace: bool,
        ignore_case: bool,
        use_inner_text: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let expected = bounded_string(expected, MAX_LOCATOR_INPUT_BYTES, "expected text")?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move {
                locator
                    .expect_text(
                        timeout_ms,
                        TextExpectation {
                            expected: &expected,
                            substring,
                            normalize_whitespace,
                            ignore_case,
                            use_inner_text,
                            is_not,
                        },
                    )
                    .await
            },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("received", result.received)?;
                value.set_item("missing", result.missing)?;
                value.set_item("timed_out", result.timed_out)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (
        source,
        flags="",
        ignore_case=None,
        use_inner_text=false,
        timeout_ms=5_000,
        is_not=false
    ))]
    #[allow(clippy::too_many_arguments)]
    fn expect_text_regex<'py>(
        &self,
        py: Python<'py>,
        source: &str,
        flags: &str,
        ignore_case: Option<bool>,
        use_inner_text: bool,
        timeout_ms: u32,
        is_not: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let source = bounded_string(source, MAX_LOCATOR_INPUT_BYTES, "RegExp source")?;
        let flags = bounded_string(flags, 64, "RegExp flags")?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move {
                locator
                    .expect_text_regex(
                        timeout_ms,
                        &source,
                        &flags,
                        ignore_case,
                        use_inner_text,
                        is_not,
                    )
                    .await
            },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("received", result.received)?;
                value.set_item("preview", result.preview)?;
                value.set_item("missing", result.missing)?;
                value.set_item("timed_out", result.timed_out)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (
        expected,
        timeout_ms=5_000,
        is_not=false,
        substring=false,
        normalize_whitespace=true,
        ignore_case=false,
        use_inner_text=false
    ))]
    #[allow(clippy::too_many_arguments)]
    fn expect_texts<'py>(
        &self,
        py: Python<'py>,
        expected: Vec<String>,
        timeout_ms: u32,
        is_not: bool,
        substring: bool,
        normalize_whitespace: bool,
        ignore_case: bool,
        use_inner_text: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let expected_bytes = expected
            .iter()
            .try_fold(0usize, |total, value| total.checked_add(value.len()))
            .ok_or_else(|| {
                binding_error(HostError::ResourceExhausted(
                    "expected text collection exceeds its byte boundary".into(),
                ))
            })?;
        if expected.len() > 1024 || expected_bytes > 1024 * 1024 {
            return Err(binding_error(HostError::ResourceExhausted(
                "expected text collection exceeds 1024 items or 1048576 UTF-8 bytes".into(),
            )));
        }
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move {
                locator
                    .expect_texts(
                        timeout_ms,
                        TextsExpectation {
                            expected: &expected,
                            substring,
                            normalize_whitespace,
                            ignore_case,
                            use_inner_text,
                            is_not,
                        },
                    )
                    .await
            },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("received", result.received)?;
                value.set_item("timed_out", result.timed_out)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (kind, expected, flags=None, timeout_ms=5_000, is_not=false))]
    fn expect_string<'py>(
        &self,
        py: Python<'py>,
        kind: &str,
        expected: &str,
        flags: Option<&str>,
        timeout_ms: u32,
        is_not: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let kind = match kind {
            "class" => LocatorStringExpectationKind::Class,
            "value" => LocatorStringExpectationKind::Value,
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "invalid string expectation kind".into(),
                )));
            }
        };
        let expected = bounded_string(expected, MAX_LOCATOR_INPUT_BYTES, "expected string")?;
        let flags = flags
            .map(|value| bounded_string(value, 64, "string expectation RegExp flags"))
            .transpose()?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move {
                locator
                    .expect_string(
                        timeout_ms,
                        kind,
                        StringMatcher {
                            expected: &expected,
                            flags: flags.as_deref(),
                        },
                        is_not,
                    )
                    .await
            },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("missing", result.missing)?;
                value.set_item("timed_out", result.timed_out)?;
                let received = result
                    .received
                    .map(|received| evaluation_value::to_python(py, received))
                    .transpose()?
                    .unwrap_or_else(|| py.None());
                value.set_item("received", received)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (expected, timeout_ms=5_000, is_not=false))]
    fn expect_class_names<'py>(
        &self,
        py: Python<'py>,
        expected: Vec<String>,
        timeout_ms: u32,
        is_not: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let expected_bytes = expected
            .iter()
            .try_fold(0usize, |total, value| total.checked_add(value.len()))
            .ok_or_else(|| {
                binding_error(HostError::ResourceExhausted(
                    "expected class collection exceeds its byte boundary".into(),
                ))
            })?;
        if expected.len() > 1024 || expected_bytes > 1024 * 1024 {
            return Err(binding_error(HostError::ResourceExhausted(
                "expected class collection exceeds 1024 items or 1048576 UTF-8 bytes".into(),
            )));
        }
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move {
                locator
                    .expect_class_names(timeout_ms, &expected, is_not)
                    .await
            },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("received", result.received)?;
                value.set_item("timed_out", result.timed_out)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (expected, timeout_ms=5_000, is_not=false))]
    fn expect_class_matchers<'py>(
        &self,
        py: Python<'py>,
        expected: Vec<(String, Option<String>)>,
        timeout_ms: u32,
        is_not: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        if expected.len() > 1024 {
            return Err(binding_error(HostError::ResourceExhausted(
                "expected class collection exceeds 1024 items".into(),
            )));
        }
        let mut bounded = Vec::with_capacity(expected.len());
        let mut total_bytes = 0usize;
        for (value, flags) in expected {
            let value = bounded_string(&value, MAX_LOCATOR_INPUT_BYTES, "expected class")?;
            let flags = flags
                .map(|flags| bounded_string(&flags, 64, "class expectation RegExp flags"))
                .transpose()?;
            total_bytes = total_bytes
                .checked_add(value.len())
                .and_then(|total| total.checked_add(flags.as_ref().map_or(0, String::len)))
                .ok_or_else(|| {
                    binding_error(HostError::ResourceExhausted(
                        "expected class collection exceeds its byte boundary".into(),
                    ))
                })?;
            if total_bytes > 1024 * 1024 {
                return Err(binding_error(HostError::ResourceExhausted(
                    "expected class collection exceeds 1048576 UTF-8 bytes".into(),
                )));
            }
            bounded.push((value, flags));
        }
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move {
                let matchers = bounded
                    .iter()
                    .map(|(expected, flags)| StringMatcher {
                        expected,
                        flags: flags.as_deref(),
                    })
                    .collect::<Vec<_>>();
                locator
                    .expect_class_matchers(timeout_ms, &matchers, is_not)
                    .await
            },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("received", result.received)?;
                value.set_item("timed_out", result.timed_out)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (name, expected, timeout_ms=5_000, is_not=false))]
    fn expect_string_property<'py>(
        &self,
        py: Python<'py>,
        name: &str,
        expected: &str,
        timeout_ms: u32,
        is_not: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let name = bounded_string(name, MAX_LOCATOR_INPUT_BYTES, "property name")?;
        let expected = bounded_string(expected, MAX_LOCATOR_INPUT_BYTES, "expected property")?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move {
                locator
                    .expect_property(
                        timeout_ms,
                        &name,
                        pwright_node_host::evaluation::EvaluationValue::String(expected),
                        is_not,
                    )
                    .await
            },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("missing", result.missing)?;
                value.set_item("timed_out", result.timed_out)?;
                let received = result
                    .received
                    .map(|received| evaluation_value::to_python(py, received))
                    .transpose()?
                    .unwrap_or_else(|| py.None());
                value.set_item("received", received)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (state, timeout_ms=5_000, is_not=false))]
    fn expect_state<'py>(
        &self,
        py: Python<'py>,
        state: &str,
        timeout_ms: u32,
        is_not: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let state = match state {
            "attached" => LocatorExpectationState::Attached,
            "detached" => LocatorExpectationState::Detached,
            "empty" => LocatorExpectationState::Empty,
            "editable" => LocatorExpectationState::Editable,
            "readonly" => LocatorExpectationState::Readonly,
            "enabled" => LocatorExpectationState::Enabled,
            "disabled" => LocatorExpectationState::Disabled,
            "focused" => LocatorExpectationState::Focused,
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "invalid Locator expectation state".into(),
                )));
            }
        };
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move { locator.expect_state(timeout_ms, state, is_not).await },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("received", result.received)?;
                value.set_item("preview", result.preview)?;
                value.set_item("missing", result.missing)?;
                value.set_item("timed_out", result.timed_out)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (checked=None, indeterminate=None, timeout_ms=5_000, is_not=false))]
    fn expect_checked<'py>(
        &self,
        py: Python<'py>,
        checked: Option<bool>,
        indeterminate: Option<bool>,
        timeout_ms: u32,
        is_not: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let state = if indeterminate == Some(true) {
            if checked.is_some() {
                return Err(binding_error(HostError::InvalidArgument(
                    "Can't assert indeterminate and checked at the same time".into(),
                )));
            }
            LocatorExpectationState::Indeterminate
        } else if checked == Some(false) {
            LocatorExpectationState::Unchecked
        } else {
            LocatorExpectationState::Checked
        };
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move { locator.expect_state(timeout_ms, state, is_not).await },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("received", result.received)?;
                value.set_item("preview", result.preview)?;
                value.set_item("missing", result.missing)?;
                value.set_item("timed_out", result.timed_out)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (name, expected, timeout_ms=5_000, is_not=false))]
    fn expect_property<'py>(
        &self,
        py: Python<'py>,
        name: &str,
        expected: &Bound<'py, PyAny>,
        timeout_ms: u32,
        is_not: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let name = bounded_string(name, MAX_LOCATOR_INPUT_BYTES, "property name")?;
        let expected = evaluation_value::from_python(Some(expected))?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move {
                locator
                    .expect_property(timeout_ms, &name, expected, is_not)
                    .await
            },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("missing", result.missing)?;
                value.set_item("timed_out", result.timed_out)?;
                let received = result
                    .received
                    .map(|received| evaluation_value::to_python(py, received))
                    .transpose()?
                    .unwrap_or_else(|| py.None());
                value.set_item("received", received)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (
        name,
        expected,
        match_options=(None, None),
        timeout_ms=5_000,
        is_not=false
    ))]
    fn expect_attribute<'py>(
        &self,
        py: Python<'py>,
        name: &str,
        expected: &str,
        match_options: (Option<String>, Option<bool>),
        timeout_ms: u32,
        is_not: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let name = bounded_string(name, MAX_LOCATOR_INPUT_BYTES, "attribute name")?;
        let expected = bounded_string(expected, MAX_LOCATOR_INPUT_BYTES, "expected attribute")?;
        let (flags, ignore_case) = match_options;
        let flags = flags
            .as_deref()
            .map(|value| bounded_string(value, 8, "attribute RegExp flags"))
            .transpose()?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move {
                locator
                    .expect_attribute(
                        timeout_ms,
                        AttributeExpectation {
                            name: &name,
                            expected: &expected,
                            flags: flags.as_deref(),
                            ignore_case,
                            is_not,
                        },
                    )
                    .await
            },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("missing", result.missing)?;
                value.set_item("timed_out", result.timed_out)?;
                let received = result
                    .received
                    .map(|received| evaluation_value::to_python(py, received))
                    .transpose()?
                    .unwrap_or_else(|| py.None());
                value.set_item("received", received)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (name, expected, flags=None, timeout_ms=5_000, is_not=false))]
    fn expect_css<'py>(
        &self,
        py: Python<'py>,
        name: &str,
        expected: &str,
        flags: Option<&str>,
        timeout_ms: u32,
        is_not: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let name = bounded_string(name, MAX_LOCATOR_INPUT_BYTES, "CSS property name")?;
        let expected = bounded_string(expected, MAX_LOCATOR_INPUT_BYTES, "expected CSS value")?;
        let flags = flags
            .map(|value| bounded_string(value, 8, "CSS RegExp flags"))
            .transpose()?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move {
                locator
                    .expect_css(
                        timeout_ms,
                        CssExpectation {
                            name: &name,
                            expected: &expected,
                            flags: flags.as_deref(),
                            is_not,
                        },
                    )
                    .await
            },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("missing", result.missing)?;
                value.set_item("timed_out", result.timed_out)?;
                let received = result
                    .received
                    .map(|received| evaluation_value::to_python(py, received))
                    .transpose()?
                    .unwrap_or_else(|| py.None());
                value.set_item("received", received)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (expected, timeout_ms=5_000, is_not=false))]
    fn expect_count<'py>(
        &self,
        py: Python<'py>,
        expected: u32,
        timeout_ms: u32,
        is_not: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        async_python_command(
            py,
            PythonCommandContext {
                owner: &self.owner,
                host: &host,
                lifecycle_callback: &self.lifecycle_callback,
            },
            FfiLane::Command,
            READ_ALLOWANCE,
            async move { locator.expect_count(timeout_ms, expected, is_not).await },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("received", result.received)?;
                value.set_item("timed_out", result.timed_out)?;
                Ok(value.into_any().unbind())
            },
        )
    }
}

#[pymethods]
impl NativeSession {
    #[cfg(feature = "context-api-request")]
    fn context_request(&self, context_id: &str) -> PyResult<api_request::NativeApiRequestContext> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let options = self
            .host
            .context_api_request_options(&context_id)
            .map_err(binding_error)?;
        api_request::NativeApiRequestContext::for_browser_context(
            Arc::clone(&self.host),
            context_id,
            options,
        )
    }

    #[cfg(feature = "cdp-session")]
    fn cdp_session_create<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        page_id: &str,
        frame_id: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let frame_id = frame_id
            .map(|value| bounded_string(value, MAX_OBJECT_ID_BYTES, "Frame ID"))
            .transpose()?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move {
                let token = host
                    .cdp_session_create(&context_id, &page_id, frame_id.as_deref())
                    .await?;
                // No suspension point separates receipt from the RAII owner.
                // Dropping the queued FFI result or its Python object schedules
                // nonblocking native cleanup unless the facade claims it.
                let runtime = tokio::runtime::Handle::current();
                Ok(NativeCdpSessionLease::new(token, move || {
                    drop(runtime.spawn(async move {
                        if let Err(error) = host.cdp_session_cleanup_unclaimed(token).await {
                            tracing::warn!(%error, token, "unclaimed CDP child cleanup failed");
                        }
                    }));
                }))
            },
            |py, lease| Ok(Py::new(py, lease)?.into_any()),
        )
    }

    #[cfg(feature = "cdp-session")]
    fn cdp_session_send<'py>(
        &self,
        py: Python<'py>,
        token: u64,
        method: &str,
        params_json: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let method = bounded_string(method, 256, "CDP method")?;
        if method.is_empty() {
            return Err(binding_error(HostError::InvalidArgument(
                "CDP method must be nonempty".into(),
            )));
        }
        let params_json = bounded_string(params_json, 1024 * 1024, "CDP params JSON")?;
        let params: serde_json::Value = serde_json::from_str(&params_json).map_err(|error| {
            binding_error(HostError::InvalidArgument(format!(
                "CDP params are not valid JSON: {error}"
            )))
        })?;
        if !params.is_object() {
            return Err(binding_error(HostError::InvalidArgument(
                "CDP params must be a JSON object".into(),
            )));
        }
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            READ_ALLOWANCE,
            async move { host.cdp_session_send(token, &method, params).await },
            |py, result| json_value_to_python(py, &result),
        )
    }

    #[cfg(feature = "cdp-session")]
    fn cdp_session_claim<'py>(
        &self,
        py: Python<'py>,
        token: u64,
        max_events: usize,
    ) -> PyResult<Bound<'py, PyAny>> {
        if max_events == 0 || max_events > 64 {
            return Err(binding_error(HostError::InvalidArgument(
                "CDP event claim limit must be in 1..=64".into(),
            )));
        }
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.cdp_session_claim(token, max_events).await },
            |py, batch| Ok(Py::new(py, NativeCdpEventBatch { batch })?.into_any()),
        )
    }

    #[cfg(feature = "cdp-session")]
    fn next_cdp_session_event_generation<'py>(
        &self,
        py: Python<'py>,
        after: u64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Watch,
            SMALL_ALLOWANCE,
            async move { host.next_cdp_session_event_generation(after).await },
            |py, generation| Ok(generation.into_pyobject(py)?.into_any().unbind()),
        )
    }

    #[cfg(feature = "cdp-session")]
    fn cdp_session_detach<'py>(&self, py: Python<'py>, token: u64) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.cdp_session_detach(token).await },
            |py, ()| Ok(py.None()),
        )
    }

    fn locator(&self, frame_id: &str, selector: &str) -> PyResult<NativeLocator> {
        Ok(NativeLocator {
            locator: self.prepare_locator(frame_id, selector)?,
            owner: Arc::clone(&self.owner),
            lifecycle_callback: Arc::clone(&self.lifecycle_callback),
        })
    }

    #[cfg(feature = "handles")]
    fn frame_locator(&self, frame_id: &str, selector: &str) -> PyResult<NativeFrameLocator> {
        let locator = self.prepare_locator(frame_id, selector)?;
        Ok(NativeFrameLocator {
            locator: locator.content_frame().map_err(binding_error)?,
            owner: Arc::clone(&self.owner),
            lifecycle_callback: Arc::clone(&self.lifecycle_callback),
        })
    }

    #[pyo3(signature = (frame_id, kind, source, exact=false, flags=None))]
    fn semantic_locator(
        &self,
        frame_id: &str,
        kind: &str,
        source: &str,
        exact: bool,
        flags: Option<&str>,
    ) -> PyResult<NativeLocator> {
        let query = prepare_semantic_query(&self.owner, kind, source, exact, flags)?;
        Ok(NativeLocator {
            locator: self.host.locator(frame_id, query).map_err(binding_error)?,
            owner: Arc::clone(&self.owner),
            lifecycle_callback: Arc::clone(&self.lifecycle_callback),
        })
    }

    #[pyo3(signature = (
        frame_id,
        role,
        name=None,
        name_flags=None,
        exact=false,
        checked=None,
        disabled=None,
        expanded=None,
        include_hidden=false,
        level=None,
        pressed=None,
        selected=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn role_locator(
        &self,
        frame_id: &str,
        role: &str,
        name: Option<&str>,
        name_flags: Option<&str>,
        exact: bool,
        checked: Option<bool>,
        disabled: Option<bool>,
        expanded: Option<bool>,
        include_hidden: bool,
        level: Option<u32>,
        pressed: Option<bool>,
        selected: Option<bool>,
    ) -> PyResult<NativeLocator> {
        let query = prepare_role_query(
            &self.owner,
            role,
            name,
            name_flags,
            exact,
            checked,
            disabled,
            expanded,
            include_hidden,
            level,
            pressed,
            selected,
        )?;
        Ok(NativeLocator {
            locator: self.host.locator(frame_id, query).map_err(binding_error)?,
            owner: Arc::clone(&self.owner),
            lifecycle_callback: Arc::clone(&self.lifecycle_callback),
        })
    }

    fn validate_selector(&self, frame_id: &str, selector: &str) -> PyResult<()> {
        self.validate_selector_impl(frame_id, selector)
    }

    #[cfg(any(feature = "selector-registration", feature = "layout-selectors"))]
    fn registered_selector_match_count<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let locator = self.prepare_locator(frame_id, selector)?;
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { locator.browser_program_match_count().await },
            |py, value| Ok(value.into_pyobject(py)?.into_any().unbind()),
        )
    }

    #[getter]
    fn version(&self) -> String {
        self.host.version().to_owned()
    }

    fn is_connected(&self) -> bool {
        self.host.is_connected()
    }

    #[cfg(feature = "page-events")]
    fn set_lifecycle_callback(&self, callback: &Bound<'_, PyAny>) -> PyResult<()> {
        if !callback.is_callable() {
            return Err(PyTypeError::new_err("lifecycle callback must be callable"));
        }
        let mut registered = self
            .lifecycle_callback
            .lock()
            .map_err(|error| Error::new_err(error.to_string()))?;
        if registered.is_some() {
            return Err(Error::new_err("lifecycle callback is already registered"));
        }
        *registered = Some(callback.clone().unbind());
        Ok(())
    }

    #[cfg(not(feature = "page-events"))]
    fn set_lifecycle_callback(&self, _callback: &Bound<'_, PyAny>) -> PyResult<()> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "page-events")]
    fn next_lifecycle_notices<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit(FfiLane::Watch, SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let publications = self.host.frame_publications();
        let retired = self.host.disconnect_signal();
        let callback = Arc::clone(&self.lifecycle_callback);
        let future = async move {
            tokio::select! {
                biased;
                () = retired.cancelled() => Ok((false, publications.fence()?)),
                fence = publications.next() => Ok((true, fence?)),
            }
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            ticket
                .deliver(|outcome| {
                    let (connected, fence) = outcome.map_err(binding_error)?;
                    Python::attach(|py| {
                        publish_frame_fence(py, fence, &callback)?;
                        Ok(connected)
                    })
                })
                .map_err(binding_error)?
        })
    }

    #[cfg(not(feature = "page-events"))]
    fn next_lifecycle_notices<'py>(&self, _py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "page-events")]
    fn flush_lifecycle_events<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.flush_topology().await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "page-events"))]
    fn flush_lifecycle_events<'py>(&self, _py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "file-chooser-events")]
    fn set_page_file_chooser_interception<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        enabled: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move {
                host.set_page_file_chooser_interception(&page_id, enabled)
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "file-chooser-events"))]
    fn set_page_file_chooser_interception<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _enabled: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page file chooser capability is not installed".into(),
        )))
    }

    #[cfg(feature = "console-events")]
    fn set_page_console_listener(&self, page_id: &str, enabled: bool) -> PyResult<()> {
        self.host
            .set_page_console_listener(page_id, enabled)
            .map_err(binding_error)
    }

    #[cfg(not(feature = "console-events"))]
    fn set_page_console_listener(&self, _page_id: &str, _enabled: bool) -> PyResult<()> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page console event capability is not installed".into(),
        )))
    }

    #[cfg(feature = "console-events")]
    fn set_context_console_listener(&self, context_id: &str, enabled: bool) -> PyResult<()> {
        self.host
            .set_context_console_listener(context_id, enabled)
            .map_err(binding_error)
    }

    #[cfg(not(feature = "console-events"))]
    fn set_context_console_listener(&self, _context_id: &str, _enabled: bool) -> PyResult<()> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext console event capability is not installed".into(),
        )))
    }

    #[cfg(feature = "console-events")]
    fn subscribe_page_console(&self, page_id: &str) -> PyResult<u64> {
        self.host
            .subscribe_page_console(page_id)
            .map_err(binding_error)
    }

    #[cfg(not(feature = "console-events"))]
    fn subscribe_page_console(&self, _page_id: &str) -> PyResult<u64> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page console event capability is not installed".into(),
        )))
    }

    #[cfg(feature = "console-events")]
    fn subscribe_context_console(&self, context_id: &str) -> PyResult<u64> {
        self.host
            .subscribe_context_console(context_id)
            .map_err(binding_error)
    }

    #[cfg(not(feature = "console-events"))]
    fn subscribe_context_console(&self, _context_id: &str) -> PyResult<u64> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext console event capability is not installed".into(),
        )))
    }

    #[cfg(feature = "console-events")]
    fn release_console_subscription(&self, token: u64) -> PyResult<()> {
        self.host
            .release_console_subscription(token)
            .map_err(binding_error)
    }

    #[cfg(not(feature = "console-events"))]
    fn release_console_subscription(&self, _token: u64) -> PyResult<()> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Console event capability is not installed".into(),
        )))
    }

    #[cfg(feature = "context-cookies")]
    fn context_cookies<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        urls: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let urls = context_cookie_urls(urls)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::context_cookies::FFI_ALLOWANCE,
            async move { host.context_cookies(&context_id, urls).await },
            |py, cookies| {
                let values = cookies.into_iter().map(|cookie| {
                    (
                        cookie.name,
                        cookie.value,
                        cookie.domain,
                        cookie.path,
                        cookie.expires,
                        cookie.http_only,
                        cookie.secure,
                        cookie.same_site,
                        cookie.partition_key,
                    )
                });
                Ok(PyList::new(py, values)?.into_any().unbind())
            },
        )
    }

    #[cfg(not(feature = "context-cookies"))]
    fn context_cookies<'py>(
        &self,
        _py: Python<'py>,
        _context_id: &str,
        _urls: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext cookies capability is not installed".into(),
        )))
    }

    #[cfg(feature = "context-cookies")]
    fn context_add_cookies<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        cookies: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let cookies = context_cookie_inputs(cookies)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::context_cookies::FFI_ALLOWANCE,
            async move { host.context_add_cookies(&context_id, cookies).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "context-cookies"))]
    fn context_add_cookies<'py>(
        &self,
        _py: Python<'py>,
        _context_id: &str,
        _cookies: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext cookies capability is not installed".into(),
        )))
    }

    #[cfg(feature = "context-cookies")]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        context_id,
        name=None,
        name_regex_flags=None,
        domain=None,
        domain_regex_flags=None,
        path=None,
        path_regex_flags=None
    ))]
    fn context_clear_cookies<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        name: Option<&str>,
        name_regex_flags: Option<&str>,
        domain: Option<&str>,
        domain_regex_flags: Option<&str>,
        path: Option<&str>,
        path_regex_flags: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let filter = BrowserContextCookieClearFilter {
            name: cookie_matcher(name, name_regex_flags).map_err(binding_error)?,
            domain: cookie_matcher(domain, domain_regex_flags).map_err(binding_error)?,
            path: cookie_matcher(path, path_regex_flags).map_err(binding_error)?,
        };
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::context_cookies::FFI_ALLOWANCE,
            async move { host.context_clear_cookies(&context_id, filter).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "context-cookies"))]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        context_id,
        name=None,
        name_regex_flags=None,
        domain=None,
        domain_regex_flags=None,
        path=None,
        path_regex_flags=None
    ))]
    fn context_clear_cookies<'py>(
        &self,
        _py: Python<'py>,
        context_id: &str,
        name: Option<&str>,
        name_regex_flags: Option<&str>,
        domain: Option<&str>,
        domain_regex_flags: Option<&str>,
        path: Option<&str>,
        path_regex_flags: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let _ = (
            context_id,
            name,
            name_regex_flags,
            domain,
            domain_regex_flags,
            path,
            path_regex_flags,
        );
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext cookies capability is not installed".into(),
        )))
    }

    #[cfg(feature = "context-storage-state")]
    fn context_storage_state<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        indexed_db: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            STORAGE_STATE_ALLOWANCE,
            async move { host.context_storage_state(&context_id, indexed_db).await },
            move |py, state| context_storage_state_to_python(py, state, indexed_db),
        )
    }

    #[cfg(not(feature = "context-storage-state"))]
    fn context_storage_state<'py>(
        &self,
        _py: Python<'py>,
        _context_id: &str,
        _indexed_db: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext storage_state capability is not installed".into(),
        )))
    }

    #[cfg(feature = "context-storage-state")]
    fn context_set_storage_state<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        storage_state_json: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let storage_state_json = storage_state_json.cast::<PyString>().map_err(|_| {
            binding_error(HostError::InvalidArgument(
                "storage_state JSON must be a string".into(),
            ))
        })?;
        if storage_state_json.len()? > MAX_STORAGE_STATE_BYTES {
            return Err(binding_error(HostError::ResourceExhausted(format!(
                "storage_state exceeds {MAX_STORAGE_STATE_BYTES} characters"
            ))));
        }
        let storage_state_json = storage_state_json.to_cow()?;
        let state = parse_context_storage_state(&storage_state_json).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            STORAGE_STATE_ALLOWANCE,
            async move { host.context_set_storage_state(&context_id, state).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "context-storage-state"))]
    fn context_set_storage_state<'py>(
        &self,
        _py: Python<'py>,
        _context_id: &str,
        _storage_state_json: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext storage_state capability is not installed".into(),
        )))
    }

    fn contexts(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Command, READ_ALLOWANCE)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let ticket = execution.run_sync(move || {
            let values = host.contexts().map(|contexts| {
                contexts
                    .into_iter()
                    .map(|context| (context.id, context.settings.base_url().map(str::to_owned)))
                    .collect::<Vec<_>>()
            });
            Ok((values, host.frame_publications().fence()?))
        });
        ticket
            .deliver(|outcome| {
                let (values, fence) = outcome.map_err(binding_error)?;
                self.publish_fence(py, fence)?;
                let values = values.map_err(binding_error)?;
                Ok(PyList::new(py, values)?.into_any().unbind())
            })
            .map_err(binding_error)?
    }

    fn pages(&self, py: Python<'_>, context_id: &str) -> PyResult<Py<PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Command, READ_ALLOWANCE)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let ticket = execution.run_sync(move || host.frame_publications().fence());
        ticket
            .deliver(|outcome| {
                let fence = outcome.map_err(binding_error)?;
                // The published collection excludes internal Pages. Advance
                // it before reading so a newly-created user Page is present.
                self.publish_fence(py, fence)?;
                let values = self.host.published_pages(&context_id).and_then(|pages| {
                    pages
                        .into_iter()
                        .map(|page| {
                            Ok((
                                page.id,
                                page.context_id,
                                page.main_frame_id,
                                page.main_frame_metadata.url()?,
                            ))
                        })
                        .collect::<Result<Vec<_>, HostError>>()
                });
                let values = values.map_err(binding_error)?;
                Ok(PyList::new(py, values)?.into_any().unbind())
            })
            .map_err(binding_error)?
    }

    fn frames(&self, py: Python<'_>, page_id: &str) -> PyResult<Py<PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Command, READ_ALLOWANCE)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let ticket = execution.run_sync(move || {
            let values = host.frames(&page_id).map(|frames| {
                frames
                    .into_iter()
                    .map(|frame| (frame.id, frame.parent_id, frame.name, frame.url))
                    .collect::<Vec<_>>()
            });
            Ok((values, host.frame_publications().fence()?))
        });
        ticket
            .deliver(|outcome| {
                let (values, fence) = outcome.map_err(binding_error)?;
                self.publish_fence(py, fence)?;
                let values = values.map_err(binding_error)?;
                Ok(PyList::new(py, values)?.into_any().unbind())
            })
            .map_err(binding_error)?
    }

    #[cfg(feature = "page-errors")]
    fn page_errors(&self, py: Python<'_>, page_id: &str) -> PyResult<Py<PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Command, READ_ALLOWANCE)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let ticket = execution.run_sync(move || {
            let values = host.page_errors(&page_id).map(|errors| {
                errors
                    .into_iter()
                    .map(|error| {
                        (
                            error.name().to_owned(),
                            error.message().to_owned(),
                            error.stack().to_owned(),
                        )
                    })
                    .collect::<Vec<_>>()
            });
            Ok((values, host.frame_publications().fence()?))
        });
        ticket
            .deliver(|outcome| {
                let (values, fence) = outcome.map_err(binding_error)?;
                self.publish_fence(py, fence)?;
                let values = values.map_err(binding_error)?;
                Ok(PyList::new(py, values)?.into_any().unbind())
            })
            .map_err(binding_error)?
    }

    #[cfg(not(feature = "page-errors"))]
    fn page_errors(&self, _py: Python<'_>, _page_id: &str) -> PyResult<Py<PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.page_errors capability is not installed".into(),
        )))
    }

    #[cfg(feature = "console-history")]
    fn console_messages(&self, py: Python<'_>, page_id: &str) -> PyResult<Py<PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Command, READ_ALLOWANCE)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let ticket = execution.run_sync(move || {
            let values = host.console_messages(&page_id).map(|messages| {
                messages
                    .into_iter()
                    .map(|message| {
                        (
                            message.message_type().to_owned(),
                            message.text().to_owned(),
                            message.arguments_json().to_owned(),
                            message.location_url().to_owned(),
                            message.line_number(),
                            message.column_number(),
                        )
                    })
                    .collect::<Vec<_>>()
            });
            Ok((values, host.frame_publications().fence()?))
        });
        ticket
            .deliver(|outcome| {
                let (values, fence) = outcome.map_err(binding_error)?;
                self.publish_fence(py, fence)?;
                let values = values.map_err(binding_error)?;
                Ok(PyList::new(py, values)?.into_any().unbind())
            })
            .map_err(binding_error)?
    }

    #[cfg(not(feature = "console-history"))]
    fn console_messages(&self, _py: Python<'_>, _page_id: &str) -> PyResult<Py<PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.console_messages capability is not installed".into(),
        )))
    }

    #[cfg(feature = "request-history")]
    fn page_requests(&self, py: Python<'_>, page_id: &str) -> PyResult<Py<PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Command, REQUEST_HISTORY_ALLOWANCE)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let ticket = execution.run_sync(move || host.page_requests(&page_id));
        ticket
            .deliver(|outcome| {
                let values = outcome.map_err(binding_error)?;
                let values = values
                    .into_iter()
                    .map(|owner| request_history_to_python(py, owner))
                    .collect::<PyResult<Vec<_>>>()?;
                Ok(PyList::new(py, values)?.into_any().unbind())
            })
            .map_err(binding_error)?
    }

    #[cfg(not(feature = "request-history"))]
    fn page_requests(&self, _py: Python<'_>, _page_id: &str) -> PyResult<Py<PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.requests capability is not installed".into(),
        )))
    }

    #[cfg(feature = "timeout-settings")]
    fn set_context_default_timeout(
        &self,
        context_id: &str,
        navigation: bool,
        timeout_ms: u32,
    ) -> PyResult<()> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        self.sync_unit_command(move |host| {
            host.set_context_default_timeout(&context_id, navigation, timeout_ms)
        })
    }

    #[cfg(not(feature = "timeout-settings"))]
    fn set_context_default_timeout(
        &self,
        _context_id: &str,
        _navigation: bool,
        _timeout_ms: u32,
    ) -> PyResult<()> {
        Err(binding_error(HostError::UnsupportedOperation(
            "timeout settings capability is not installed".into(),
        )))
    }

    #[cfg(feature = "timeout-settings")]
    fn set_page_default_timeout(
        &self,
        page_id: &str,
        navigation: bool,
        timeout_ms: u32,
    ) -> PyResult<()> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        self.sync_unit_command(move |host| {
            host.set_page_default_timeout(&page_id, navigation, timeout_ms)
        })
    }

    #[cfg(not(feature = "timeout-settings"))]
    fn set_page_default_timeout(
        &self,
        _page_id: &str,
        _navigation: bool,
        _timeout_ms: u32,
    ) -> PyResult<()> {
        Err(binding_error(HostError::UnsupportedOperation(
            "timeout settings capability is not installed".into(),
        )))
    }

    #[cfg(feature = "timeout-settings")]
    fn context_default_timeout(&self, context_id: &str, navigation: bool) -> PyResult<u32> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        self.sync_u32_command(move |host| host.context_default_timeout(&context_id, navigation))
    }

    #[cfg(not(feature = "timeout-settings"))]
    fn context_default_timeout(&self, _context_id: &str, _navigation: bool) -> PyResult<u32> {
        Err(binding_error(HostError::UnsupportedOperation(
            "timeout settings capability is not installed".into(),
        )))
    }

    #[cfg(feature = "timeout-settings")]
    fn page_default_timeout(&self, page_id: &str, navigation: bool) -> PyResult<u32> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        self.sync_u32_command(move |host| host.page_default_timeout(&page_id, navigation))
    }

    #[cfg(not(feature = "timeout-settings"))]
    fn page_default_timeout(&self, _page_id: &str, _navigation: bool) -> PyResult<u32> {
        Err(binding_error(HostError::UnsupportedOperation(
            "timeout settings capability is not installed".into(),
        )))
    }

    #[cfg(feature = "page-pause")]
    fn page_pause<'py>(&self, py: Python<'py>, page_id: &str) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.page_pause(&page_id).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "page-pause"))]
    fn page_pause<'py>(&self, _py: Python<'py>, _page_id: &str) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.pause capability is not installed".into(),
        )))
    }

    #[cfg(feature = "wait-for-timeout")]
    fn wait_for_timeout<'py>(
        &self,
        py: Python<'py>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.wait_for_timeout(timeout_ms).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "wait-for-timeout"))]
    fn wait_for_timeout<'py>(
        &self,
        _py: Python<'py>,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.wait_for_timeout capability is not installed".into(),
        )))
    }

    #[pyo3(signature = (
        page_id,
        format=-1,
        path=None,
        quality=None,
        full_page=false,
        scale=0,
        timeout_ms=30_000,
        mask=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn page_screenshot<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        format: i8,
        path: Option<&str>,
        quality: Option<u8>,
        full_page: bool,
        scale: u8,
        timeout_ms: u32,
        mask: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let path = path
            .map(|value| bounded_string(value, MAX_PATH_BYTES, "screenshot output path"))
            .transpose()?;
        let options = screenshot_options(format, path.as_deref(), quality, full_page, scale)?;
        let masks = screenshot_masks(mask, &self.host)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::screenshot::FFI_ALLOWANCE,
            async move {
                let output = host
                    .page_screenshot_with_masks(&page_id, options, masks, timeout_ms)
                    .await?;
                if let Some(path) = path {
                    tokio::fs::write(path, output.as_bytes())
                        .await
                        .map_err(|error| {
                            HostError::Backend(format!(
                                "Page.screenshot could not write output: {error}"
                            ))
                        })?;
                }
                Ok(output)
            },
            |py, output| Ok(PyBytes::new(py, output.as_bytes()).into_any().unbind()),
        )
    }

    #[cfg(feature = "handles")]
    #[pyo3(signature = (
        element,
        format=-1,
        path=None,
        quality=None,
        scale=0,
        timeout_ms=30_000,
        mask=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn element_screenshot<'py>(
        &self,
        py: Python<'py>,
        element: &handles::NativeHandle,
        format: i8,
        path: Option<&str>,
        quality: Option<u8>,
        scale: u8,
        timeout_ms: u32,
        mask: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let element = element.read_frame().map_err(binding_error)?;
        let path = path
            .map(|value| bounded_string(value, MAX_PATH_BYTES, "screenshot output path"))
            .transpose()?;
        let options = screenshot_options(format, path.as_deref(), quality, false, scale)?;
        let masks = screenshot_masks(mask, &self.host)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::screenshot::FFI_ALLOWANCE,
            async move {
                let output = host
                    .element_screenshot_with_masks(&element, options, masks, timeout_ms)
                    .await?;
                if let Some(path) = path {
                    tokio::fs::write(path, output.as_bytes())
                        .await
                        .map_err(|error| {
                            HostError::Backend(format!(
                                "ElementHandle.screenshot could not write output: {error}"
                            ))
                        })?;
                }
                Ok(output)
            },
            |py, output| Ok(PyBytes::new(py, output.as_bytes()).into_any().unbind()),
        )
    }

    #[cfg(not(feature = "handles"))]
    #[pyo3(signature = (
        element,
        format=-1,
        path=None,
        quality=None,
        scale=0,
        timeout_ms=30_000,
        mask=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn element_screenshot<'py>(
        &self,
        _py: Python<'py>,
        element: &Bound<'py, PyAny>,
        format: i8,
        path: Option<&str>,
        quality: Option<u8>,
        scale: u8,
        timeout_ms: u32,
        mask: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let _ = (element, format, path, quality, scale, timeout_ms, mask);
        Err(binding_error(HostError::UnsupportedOperation(
            "ElementHandle.screenshot requires the Handle capability".into(),
        )))
    }

    #[cfg(feature = "element-handle-file-input")]
    #[pyo3(signature = (element, base, paths, timeout_ms=30_000))]
    fn element_handle_set_input_files_paths<'py>(
        &self,
        py: Python<'py>,
        element: &handles::NativeHandle,
        base: &str,
        paths: &Bound<'py, PyAny>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let element = element.read_frame().map_err(binding_error)?;
        let base = bounded_string(
            base,
            pwright_node_host::file_input::profile::PATH_BYTES,
            "file-input base",
        )?;
        let paths = paths.cast::<PyList>().map_err(|_| {
            binding_error(HostError::InvalidArgument(
                "file-input paths must be a list".into(),
            ))
        })?;
        if paths.len() > pwright_node_host::file_input::profile::ITEMS {
            return Err(binding_error(HostError::InvalidArgument(
                "file-input count exceeds selected limit".into(),
            )));
        }
        let mut values = Vec::with_capacity(paths.len());
        for path in paths.iter() {
            let path = path.cast::<PyString>().map_err(|_| {
                binding_error(HostError::InvalidArgument(
                    "file-input path must be a string".into(),
                ))
            })?;
            values.push(bounded_string(
                &path.to_cow()?,
                pwright_node_host::file_input::profile::PATH_BYTES,
                "file-input path",
            )?);
        }
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            ACTION_ALLOWANCE,
            async move {
                let views = values.iter().map(String::as_str).collect::<Vec<_>>();
                let input = host.prepare_file_paths(Path::new(&base), &views).await?;
                host.handle_set_input_files(element, input, timeout_ms)
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "element-handle-file-input")]
    #[pyo3(signature = (element, payloads, timeout_ms=30_000))]
    fn element_handle_set_input_file_payloads<'py>(
        &self,
        py: Python<'py>,
        element: &handles::NativeHandle,
        payloads: &Bound<'py, PyAny>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        struct Payload {
            name: String,
            mime_type: String,
            buffer: Py<PyBytes>,
        }

        let element = element.read_frame().map_err(binding_error)?;
        let payloads = payloads.cast::<PyList>().map_err(|_| {
            binding_error(HostError::InvalidArgument(
                "file payloads must be a list".into(),
            ))
        })?;
        if payloads.len() > pwright_node_host::file_input::profile::ITEMS {
            return Err(binding_error(HostError::InvalidArgument(
                "file-input count exceeds selected limit".into(),
            )));
        }
        let mut values = Vec::with_capacity(payloads.len());
        for payload in payloads.iter() {
            let payload = payload.cast::<PyDict>().map_err(|_| {
                binding_error(HostError::InvalidArgument(
                    "file payload must be a mapping".into(),
                ))
            })?;
            let name = payload
                .get_item("name")?
                .ok_or_else(|| {
                    binding_error(HostError::InvalidArgument(
                        "file payload name is required".into(),
                    ))
                })?
                .cast_into::<PyString>()
                .map_err(|_| {
                    binding_error(HostError::InvalidArgument(
                        "file payload name must be a string".into(),
                    ))
                })?;
            let mime_type = payload
                .get_item("mimeType")?
                .ok_or_else(|| {
                    binding_error(HostError::InvalidArgument(
                        "file payload MIME is required".into(),
                    ))
                })?
                .cast_into::<PyString>()
                .map_err(|_| {
                    binding_error(HostError::InvalidArgument(
                        "file payload MIME must be a string".into(),
                    ))
                })?;
            let buffer = payload
                .get_item("buffer")?
                .ok_or_else(|| {
                    binding_error(HostError::InvalidArgument(
                        "file payload buffer is required".into(),
                    ))
                })?
                .cast_into::<PyBytes>()
                .map_err(|_| {
                    binding_error(HostError::InvalidArgument(
                        "file payload buffer must be bytes".into(),
                    ))
                })?;
            values.push(Payload {
                name: bounded_string(
                    &name.to_cow()?,
                    pwright_node_host::file_input::profile::NAME_BYTES,
                    "file payload name",
                )?,
                mime_type: bounded_string(
                    &mime_type.to_cow()?,
                    pwright_node_host::file_input::profile::MIME_BYTES,
                    "file payload MIME",
                )?,
                buffer: buffer.unbind(),
            });
        }
        let inputs = values
            .iter()
            .map(|payload| pwright_node_host::file_input::MemoryFile {
                name: &payload.name,
                mime_type: Some(&payload.mime_type),
                buffer: payload.buffer.bind(py).as_bytes(),
            })
            .collect::<Vec<_>>();
        let input = self
            .host
            .prepare_file_payloads(&inputs)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            ACTION_ALLOWANCE,
            async move {
                host.handle_set_input_files(element, input, timeout_ms)
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "element-handle-file-input"))]
    #[pyo3(signature = (_element, _base, _paths, _timeout_ms=30_000))]
    fn element_handle_set_input_files_paths<'py>(
        &self,
        _py: Python<'py>,
        _element: &Bound<'py, PyAny>,
        _base: &str,
        _paths: &Bound<'py, PyAny>,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "ElementHandle.set_input_files capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "element-handle-file-input"))]
    #[pyo3(signature = (_element, _payloads, _timeout_ms=30_000))]
    fn element_handle_set_input_file_payloads<'py>(
        &self,
        _py: Python<'py>,
        _element: &Bound<'py, PyAny>,
        _payloads: &Bound<'py, PyAny>,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "ElementHandle.set_input_files capability is not installed".into(),
        )))
    }

    #[cfg(feature = "page-pdf")]
    #[pyo3(signature = (page_id, path=None, tagged=false, outline=false))]
    fn page_pdf<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        path: Option<&str>,
        tagged: bool,
        outline: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let path = path
            .map(|value| bounded_string(value, MAX_PATH_BYTES, "PDF output path"))
            .transpose()?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::page_pdf::FFI_ALLOWANCE,
            async move {
                let output = host
                    .page_pdf(
                        &page_id,
                        pwright_node_host::page_pdf::PdfOptions { tagged, outline },
                        30_000,
                    )
                    .await?;
                if let Some(path) = path {
                    tokio::fs::write(path, output.as_bytes())
                        .await
                        .map_err(|error| {
                            HostError::Backend(format!("Page.pdf could not write output: {error}"))
                        })?;
                }
                Ok(output)
            },
            |py, output| Ok(PyBytes::new(py, output.as_bytes()).into_any().unbind()),
        )
    }

    #[cfg(not(feature = "page-pdf"))]
    #[pyo3(signature = (page_id, path=None, tagged=false, outline=false))]
    fn page_pdf<'py>(
        &self,
        _py: Python<'py>,
        page_id: &str,
        path: Option<&str>,
        tagged: bool,
        outline: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let _ = (page_id, path, tagged, outline);
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.pdf capability is not installed".into(),
        )))
    }

    #[cfg(feature = "request-gc")]
    fn page_request_gc<'py>(&self, py: Python<'py>, page_id: &str) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.page_request_gc(&page_id).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "request-gc"))]
    fn page_request_gc<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.request_gc capability is not installed".into(),
        )))
    }

    #[cfg(feature = "page-viewport")]
    fn page_set_viewport_size<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        width: u32,
        height: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.page_set_viewport_size(&page_id, width, height).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "page-viewport"))]
    fn page_set_viewport_size<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _width: u32,
        _height: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.set_viewport_size capability is not installed".into(),
        )))
    }

    #[pyo3(signature = (base_url=None, device=None, locale=None, timezone_id=None, geolocation=None, geolocation_permission=false, http_credentials=None, ignore_https_errors=false, offline=false, bypass_csp=false, javascript_enabled=true, accept_downloads=false, block_service_workers=false, forced_colors=None, strict_selectors=false, *, extra_http_headers=None))]
    #[allow(clippy::too_many_arguments)]
    fn new_context<'py>(
        &self,
        py: Python<'py>,
        base_url: Option<&str>,
        device: Option<PythonDeviceOptions>,
        locale: Option<&str>,
        timezone_id: Option<&str>,
        geolocation: Option<PythonGeolocation>,
        geolocation_permission: bool,
        http_credentials: Option<PythonHttpCredentials>,
        ignore_https_errors: bool,
        offline: bool,
        bypass_csp: bool,
        javascript_enabled: bool,
        accept_downloads: bool,
        block_service_workers: bool,
        forced_colors: Option<&str>,
        strict_selectors: bool,
        extra_http_headers: Option<Vec<(String, String)>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let base_url = base_url
            .map(|value| bounded_string(value, MAX_URL_BYTES, "base_url"))
            .transpose()?;
        #[cfg(feature = "device-emulation")]
        let device = device
            .map(|(user_agent, viewport, screen, scale, mobile, touch)| {
                let user_agent = user_agent
                    .map(|value| bounded_string(&value, 4096, "user_agent"))
                    .transpose()?;
                pwright_node_host::DeviceContextOptions::new(
                    user_agent, viewport, screen, scale, mobile, touch,
                )
                .map_err(binding_error)
            })
            .transpose()?;
        #[cfg(not(feature = "device-emulation"))]
        if device.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "device emulation capability is not installed".into(),
            )));
        }
        #[cfg(feature = "context-locale")]
        let locale = locale
            .map(|value| {
                bounded_string(value, 256, "locale").and_then(|value| {
                    pwright_node_host::ContextLocale::new(value).map_err(binding_error)
                })
            })
            .transpose()?;
        #[cfg(not(feature = "context-locale"))]
        if locale.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "Context locale capability is not installed".into(),
            )));
        }
        #[cfg(feature = "context-timezone")]
        let timezone = timezone_id
            .map(|value| {
                bounded_string(value, 256, "timezone_id").and_then(|value| {
                    pwright_node_host::ContextTimezone::new(value).map_err(binding_error)
                })
            })
            .transpose()?;
        #[cfg(not(feature = "context-timezone"))]
        if timezone_id.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "Context timezone capability is not installed".into(),
            )));
        }
        #[cfg(feature = "context-geolocation")]
        let geolocation = geolocation
            .map(|(longitude, latitude, accuracy)| {
                pwright_node_host::ContextGeolocation::new(longitude, latitude, accuracy)
                    .map_err(binding_error)
            })
            .transpose()?;
        #[cfg(not(feature = "context-geolocation"))]
        if geolocation.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "Context geolocation capability is not installed".into(),
            )));
        }
        #[cfg(not(feature = "context-geolocation-permission"))]
        if geolocation_permission {
            return Err(binding_error(HostError::UnsupportedOperation(
                "Context geolocation permission capability is not installed".into(),
            )));
        }
        #[cfg(feature = "context-http-credentials")]
        let http_credentials = http_credentials
            .map(|(username, password, origin, send_always)| {
                pwright_node_host::ContextHttpCredentials::new_with_send(
                    username,
                    password,
                    origin,
                    send_always,
                )
                .map_err(binding_error)
            })
            .transpose()?;
        #[cfg(not(feature = "context-http-credentials"))]
        if http_credentials.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "Context HTTP credentials capability is not installed".into(),
            )));
        }
        #[cfg(not(feature = "ignore-https-errors"))]
        if ignore_https_errors {
            return Err(binding_error(HostError::UnsupportedOperation(
                "ignore HTTPS errors capability is not installed".into(),
            )));
        }
        #[cfg(not(feature = "context-offline"))]
        if offline {
            return Err(binding_error(HostError::UnsupportedOperation(
                "offline capability is not installed".into(),
            )));
        }
        #[cfg(feature = "context-forced-colors")]
        let forced_colors = forced_colors
            .map(|value| {
                bounded_string(value, 32, "forced_colors").and_then(|value| {
                    pwright_node_host::ContextForcedColors::new(&value).map_err(binding_error)
                })
            })
            .transpose()?;
        #[cfg(not(feature = "context-forced-colors"))]
        if forced_colors.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "Context forced colors capability is not installed".into(),
            )));
        }
        #[cfg(not(feature = "context-strict-selectors"))]
        if strict_selectors {
            return Err(binding_error(HostError::UnsupportedOperation(
                "Context strict selectors capability is not installed".into(),
            )));
        }
        #[cfg(not(feature = "extra-headers"))]
        if extra_http_headers.is_some() {
            return Err(binding_error(HostError::UnsupportedOperation(
                "Browser.new_context extra_http_headers capability is not installed".into(),
            )));
        }
        #[cfg(feature = "extra-headers")]
        if let Some(headers) = extra_http_headers.as_deref() {
            validate_new_context_headers(headers).map_err(binding_error)?;
        }
        #[cfg(not(feature = "bypass-csp"))]
        if bypass_csp {
            return Err(binding_error(HostError::UnsupportedOperation(
                "bypass CSP capability is not installed".into(),
            )));
        }
        #[cfg(not(feature = "javascript-disabled"))]
        if !javascript_enabled {
            return Err(binding_error(HostError::UnsupportedOperation(
                "JavaScript-disabled capability is not installed".into(),
            )));
        }
        #[cfg(not(feature = "download-events"))]
        if accept_downloads {
            return Err(binding_error(HostError::UnsupportedOperation(
                "download event capability is not installed".into(),
            )));
        }
        #[cfg(not(feature = "service-worker-policy"))]
        if block_service_workers {
            return Err(binding_error(HostError::UnsupportedOperation(
                "service-worker policy capability is not installed".into(),
            )));
        }
        let host = Arc::clone(&self.host);
        #[cfg(feature = "context-strict-selectors")]
        let context_strict_selectors = Arc::clone(&self.context_strict_selectors);
        #[cfg(feature = "extra-headers")]
        let creation_allowance = if extra_http_headers.is_some() {
            EXTRA_HEADERS_ALLOWANCE
        } else {
            SMALL_ALLOWANCE
        };
        #[cfg(not(feature = "extra-headers"))]
        let creation_allowance = SMALL_ALLOWANCE;
        self.async_command(
            py,
            FfiLane::Command,
            creation_allowance,
            async move {
                #[cfg(any(
                    feature = "extra-headers",
                    feature = "context-timezone",
                    feature = "context-offline",
                    feature = "context-forced-colors",
                    feature = "context-geolocation",
                    feature = "context-geolocation-permission",
                    feature = "context-http-credentials"
                ))]
                let policies = pwright_node_host::ContextPolicyOptions::default();
                #[cfg(all(
                    any(
                        feature = "extra-headers",
                        feature = "context-timezone",
                        feature = "context-offline",
                        feature = "context-forced-colors",
                        feature = "context-geolocation",
                        feature = "context-geolocation-permission",
                        feature = "context-http-credentials"
                    ),
                    feature = "context-locale"
                ))]
                let policies = policies.with_locale(locale);
                #[cfg(feature = "context-timezone")]
                let policies = policies.with_timezone(timezone);
                #[cfg(feature = "context-offline")]
                let policies = policies.with_offline(offline);
                #[cfg(feature = "context-forced-colors")]
                let policies = policies.with_forced_colors(forced_colors);
                #[cfg(feature = "context-geolocation")]
                let policies = policies.with_geolocation(geolocation);
                #[cfg(feature = "context-geolocation-permission")]
                let policies = policies.with_geolocation_permission(geolocation_permission);
                #[cfg(feature = "context-http-credentials")]
                let policies = policies.with_http_credentials(http_credentials);
                #[cfg(all(
                    any(
                        feature = "extra-headers",
                        feature = "context-timezone",
                        feature = "context-offline",
                        feature = "context-forced-colors",
                        feature = "context-geolocation",
                        feature = "context-geolocation-permission",
                        feature = "context-http-credentials"
                    ),
                    feature = "ignore-https-errors"
                ))]
                let policies = policies.with_ignore_https_errors(ignore_https_errors);
                #[cfg(all(
                    any(
                        feature = "extra-headers",
                        feature = "context-timezone",
                        feature = "context-offline",
                        feature = "context-forced-colors",
                        feature = "context-geolocation",
                        feature = "context-geolocation-permission",
                        feature = "context-http-credentials"
                    ),
                    feature = "bypass-csp"
                ))]
                let policies = policies.with_bypass_csp(bypass_csp);
                #[cfg(all(
                    any(
                        feature = "extra-headers",
                        feature = "context-timezone",
                        feature = "context-offline",
                        feature = "context-forced-colors",
                        feature = "context-geolocation",
                        feature = "context-geolocation-permission",
                        feature = "context-http-credentials"
                    ),
                    feature = "javascript-disabled"
                ))]
                let policies = policies.with_javascript_enabled(javascript_enabled);
                #[cfg(any(
                    feature = "extra-headers",
                    feature = "context-timezone",
                    feature = "context-offline",
                    feature = "context-forced-colors",
                    feature = "context-geolocation",
                    feature = "context-geolocation-permission",
                    feature = "context-http-credentials"
                ))]
                let context = {
                    #[cfg(feature = "extra-headers")]
                    {
                        if let Some(headers) = extra_http_headers {
                            let input: Vec<_> = headers
                                .iter()
                                .map(|(name, value)| {
                                    pwright_node_host::extra_headers::HeaderRef { name, value }
                                })
                                .collect();
                            host.new_context_with_policies_and_extra_http_headers(
                                base_url.as_deref(),
                                #[cfg(feature = "device-emulation")]
                                device,
                                policies,
                                &input,
                            )
                            .await?
                        } else {
                            host.new_context_with_policies(
                                base_url.as_deref(),
                                #[cfg(feature = "device-emulation")]
                                device,
                                policies,
                            )
                            .await?
                        }
                    }
                    #[cfg(not(feature = "extra-headers"))]
                    host.new_context_with_policies(
                        base_url.as_deref(),
                        #[cfg(feature = "device-emulation")]
                        device,
                        policies,
                    )
                    .await?
                };
                #[cfg(all(
                    not(feature = "extra-headers"),
                    not(feature = "context-timezone"),
                    not(feature = "context-offline"),
                    not(feature = "context-forced-colors"),
                    not(feature = "context-geolocation"),
                    not(feature = "context-geolocation-permission"),
                    not(feature = "context-http-credentials"),
                    feature = "javascript-disabled"
                ))]
                let context = host
                    .new_context_with_javascript_enabled(
                        base_url.as_deref(),
                        #[cfg(feature = "device-emulation")]
                        device,
                        #[cfg(feature = "context-locale")]
                        locale,
                        #[cfg(feature = "ignore-https-errors")]
                        ignore_https_errors,
                        #[cfg(feature = "bypass-csp")]
                        bypass_csp,
                        javascript_enabled,
                    )
                    .await?;
                #[cfg(all(
                    not(feature = "extra-headers"),
                    not(feature = "context-timezone"),
                    not(feature = "context-offline"),
                    not(feature = "context-forced-colors"),
                    not(feature = "context-geolocation"),
                    not(feature = "context-geolocation-permission"),
                    not(feature = "context-http-credentials"),
                    not(feature = "javascript-disabled"),
                    feature = "bypass-csp"
                ))]
                let context = host
                    .new_context_with_bypass_csp(
                        base_url.as_deref(),
                        #[cfg(feature = "device-emulation")]
                        device,
                        #[cfg(feature = "context-locale")]
                        locale,
                        #[cfg(feature = "ignore-https-errors")]
                        ignore_https_errors,
                        bypass_csp,
                    )
                    .await?;
                #[cfg(all(
                    not(feature = "extra-headers"),
                    not(feature = "context-timezone"),
                    not(feature = "context-offline"),
                    not(feature = "context-forced-colors"),
                    not(feature = "context-geolocation"),
                    not(feature = "context-geolocation-permission"),
                    not(feature = "context-http-credentials"),
                    not(feature = "javascript-disabled"),
                    not(feature = "bypass-csp"),
                    feature = "context-locale"
                ))]
                let context = host
                    .new_context_with_locale(
                        base_url.as_deref(),
                        #[cfg(feature = "device-emulation")]
                        device,
                        locale,
                        #[cfg(feature = "ignore-https-errors")]
                        ignore_https_errors,
                    )
                    .await?;
                #[cfg(all(
                    not(feature = "extra-headers"),
                    not(feature = "context-timezone"),
                    not(feature = "context-offline"),
                    not(feature = "context-forced-colors"),
                    not(feature = "context-geolocation"),
                    not(feature = "context-geolocation-permission"),
                    not(feature = "context-http-credentials"),
                    not(feature = "javascript-disabled"),
                    not(feature = "bypass-csp"),
                    not(feature = "context-locale"),
                    feature = "ignore-https-errors"
                ))]
                let context = host
                    .new_context_with_ignore_https_errors(
                        base_url.as_deref(),
                        #[cfg(feature = "device-emulation")]
                        device,
                        ignore_https_errors,
                    )
                    .await?;
                #[cfg(all(
                    not(feature = "extra-headers"),
                    not(feature = "context-timezone"),
                    not(feature = "context-offline"),
                    not(feature = "context-forced-colors"),
                    not(feature = "context-geolocation"),
                    not(feature = "context-geolocation-permission"),
                    not(feature = "context-http-credentials"),
                    not(feature = "javascript-disabled"),
                    not(feature = "bypass-csp"),
                    not(feature = "context-locale"),
                    not(feature = "ignore-https-errors"),
                    feature = "device-emulation"
                ))]
                let context = host
                    .new_context_with_device_options(base_url.as_deref(), device)
                    .await?;
                #[cfg(all(
                    not(feature = "extra-headers"),
                    not(feature = "context-timezone"),
                    not(feature = "context-offline"),
                    not(feature = "context-forced-colors"),
                    not(feature = "context-geolocation"),
                    not(feature = "context-geolocation-permission"),
                    not(feature = "context-http-credentials"),
                    not(feature = "javascript-disabled"),
                    not(feature = "bypass-csp"),
                    not(feature = "context-locale"),
                    not(feature = "ignore-https-errors"),
                    not(feature = "device-emulation")
                ))]
                let context = host.new_context_with_base_url(base_url.as_deref()).await?;
                #[cfg(feature = "service-worker-policy")]
                if block_service_workers {
                    host.install_service_worker_policy(
                        &context.id,
                        pwright_node_host::ServiceWorkerPolicy::Block,
                    )
                    .await?;
                }
                #[cfg(feature = "download-events")]
                if accept_downloads
                    && let Err(primary) = host.enable_download_context(&context.id).await
                {
                    if let Err(cleanup) = host.close_context(&context.id).await {
                        return Err(HostError::Backend(format!(
                            "download policy installation failed: {primary}; Context cleanup failed: {cleanup}"
                        )));
                    }
                    return Err(primary);
                }
                #[cfg(feature = "context-strict-selectors")]
                if let Err(primary) =
                    context_strict_selectors.install(&context.id, strict_selectors)
                {
                    if let Err(cleanup) = host.close_context(&context.id).await {
                        return Err(HostError::Backend(format!(
                            "strict selector policy installation failed: {primary}; Context cleanup failed: {cleanup}"
                        )));
                    }
                    return Err(primary);
                }
                Ok(context)
            },
            |py, context| {
                let value = (context.id, context.settings.base_url().map(str::to_owned));
                Ok(value.into_pyobject(py)?.into_any().unbind())
            },
        )
    }

    fn new_page<'py>(&self, py: Python<'py>, context_id: &str) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            READ_ALLOWANCE,
            async move { host.new_page(&context_id).await },
            |py, page| {
                let value = (
                    page.id,
                    page.context_id,
                    page.main_frame_id,
                    page.main_frame_metadata.url().map_err(binding_error)?,
                );
                Ok(value.into_pyobject(py)?.into_any().unbind())
            },
        )
    }

    #[cfg(feature = "init-scripts")]
    fn page_add_init_script<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        source: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let source = bounded_string(
            source,
            pwright_node_host::init_scripts::MAX_SOURCE_BYTES,
            "init script",
        )?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.page_add_init_script(&page_id, source).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "init-scripts"))]
    fn page_add_init_script<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _source: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.add_init_script capability is not installed".into(),
        )))
    }

    #[cfg(feature = "init-scripts")]
    fn context_add_init_script<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        source: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let source = bounded_string(
            source,
            pwright_node_host::init_scripts::MAX_SOURCE_BYTES,
            "init script",
        )?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.context_add_init_script(&context_id, source).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "init-scripts"))]
    fn context_add_init_script<'py>(
        &self,
        _py: Python<'py>,
        _context_id: &str,
        _source: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext.add_init_script capability is not installed".into(),
        )))
    }

    #[cfg(feature = "extra-headers")]
    fn set_page_extra_http_headers<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        headers: Vec<(String, String)>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            EXTRA_HEADERS_ALLOWANCE,
            async move {
                let input: Vec<_> = headers
                    .iter()
                    .map(
                        |(name, value)| pwright_node_host::extra_headers::HeaderRef { name, value },
                    )
                    .collect();
                host.start_set_extra_http_headers_native(
                    pwright_node_host::extra_headers::NativeHeaderTarget::Page(&page_id),
                    &input,
                )
                .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "extra-headers"))]
    fn set_page_extra_http_headers<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _headers: Vec<(String, String)>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.set_extra_http_headers capability is not installed".into(),
        )))
    }

    #[cfg(feature = "extra-headers")]
    fn set_context_extra_http_headers<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        headers: Vec<(String, String)>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            EXTRA_HEADERS_ALLOWANCE,
            async move {
                let input: Vec<_> = headers
                    .iter()
                    .map(
                        |(name, value)| pwright_node_host::extra_headers::HeaderRef { name, value },
                    )
                    .collect();
                host.start_set_extra_http_headers_native(
                    pwright_node_host::extra_headers::NativeHeaderTarget::Context(&context_id),
                    &input,
                )
                .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "extra-headers"))]
    fn set_context_extra_http_headers<'py>(
        &self,
        _py: Python<'py>,
        _context_id: &str,
        _headers: Vec<(String, String)>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext.set_extra_http_headers capability is not installed".into(),
        )))
    }

    #[cfg(feature = "context-offline")]
    fn set_context_offline<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        offline: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.set_context_offline(&context_id, offline).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "context-offline"))]
    fn set_context_offline<'py>(
        &self,
        _py: Python<'py>,
        _context_id: &str,
        _offline: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext.set_offline capability is not installed".into(),
        )))
    }

    #[pyo3(signature = (base_url=None, accept_downloads=false))]
    fn new_browser_page<'py>(
        &self,
        py: Python<'py>,
        base_url: Option<&str>,
        accept_downloads: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let base_url = base_url
            .map(|value| bounded_string(value, MAX_URL_BYTES, "base_url"))
            .transpose()?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            READ_ALLOWANCE,
            async move {
                host.new_browser_page_with_downloads(base_url.as_deref(), accept_downloads)
                    .await
            },
            |py, value| {
                let context = (
                    value.context.id,
                    value.context.settings.base_url().map(str::to_owned),
                );
                let page = (
                    value.page.id,
                    value.page.context_id,
                    value.page.main_frame_id,
                    value
                        .page
                        .main_frame_metadata
                        .url()
                        .map_err(binding_error)?,
                );
                Ok((context, page).into_pyobject(py)?.into_any().unbind())
            },
        )
    }

    fn title<'py>(&self, py: Python<'py>, frame_id: &str) -> PyResult<Bound<'py, PyAny>> {
        let frame_id = bounded_string(frame_id, MAX_OBJECT_ID_BYTES, "Frame ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            READ_ALLOWANCE,
            async move { host.frame_title(&frame_id).await },
            |py, value| Ok(value.into_pyobject(py)?.into_any().unbind()),
        )
    }

    fn content<'py>(&self, py: Python<'py>, frame_id: &str) -> PyResult<Bound<'py, PyAny>> {
        let frame_id = bounded_string(frame_id, MAX_OBJECT_ID_BYTES, "Frame ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            READ_ALLOWANCE,
            async move { host.frame_content(&frame_id).await },
            |py, value| Ok(value.into_pyobject(py)?.into_any().unbind()),
        )
    }

    #[cfg(feature = "set-content")]
    #[pyo3(signature = (page_id, html, wait_until="load", timeout_ms=30_000))]
    fn set_content<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        html: &str,
        wait_until: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let html = bounded_string(html, pwright_node_host::evaluation::STRING_BYTES, "html")?;
        let wait_until = bounded_string(wait_until, 32, "wait_until")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                host.page_set_content(&page_id, html, &wait_until, timeout_ms)
                    .await
            },
            |py, url| Ok(url.into_pyobject(py)?.into_any().unbind()),
        )
    }

    #[cfg(feature = "emulate-media")]
    #[pyo3(signature = (page_id, options=None))]
    fn emulate_media<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        options: Option<PythonMediaOptions>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let (media, color_scheme, reduced_motion, forced_colors, contrast) =
            options.unwrap_or((None, None, None, None, None));
        let media = media
            .map(|value| bounded_string(&value, 32, "media"))
            .transpose()?;
        let color_scheme = color_scheme
            .map(|value| bounded_string(&value, 32, "color_scheme"))
            .transpose()?;
        let reduced_motion = reduced_motion
            .map(|value| bounded_string(&value, 32, "reduced_motion"))
            .transpose()?;
        let forced_colors = forced_colors
            .map(|value| bounded_string(&value, 32, "forced_colors"))
            .transpose()?;
        let contrast = contrast
            .map(|value| bounded_string(&value, 32, "contrast"))
            .transpose()?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move {
                host.page_emulate_media(
                    &page_id,
                    media.as_deref(),
                    color_scheme.as_deref(),
                    reduced_motion.as_deref(),
                    forced_colors.as_deref(),
                    contrast.as_deref(),
                )
                .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "emulate-media"))]
    #[pyo3(signature = (_page_id, _options=None))]
    fn emulate_media<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _options: Option<PythonMediaOptions>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.emulate_media capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "set-content"))]
    #[pyo3(signature = (_page_id, _html, _wait_until="load", _timeout_ms=30_000))]
    fn set_content<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _html: &str,
        _wait_until: &str,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.set_content capability is not installed".into(),
        )))
    }

    #[pyo3(signature = (frame_id, selector, timeout_ms=30_000))]
    fn click<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.locator_unit_action(
            py,
            frame_id,
            selector,
            timeout_ms,
            LocatorAction::Click(ClickOptionsV2::new()),
        )
    }

    #[pyo3(signature = (frame_id, selector, options, strict=false, timeout_ms=30_000))]
    fn click_with_options<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
        options: &Bound<'_, PyDict>,
        strict: bool,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let options = page_click_options(options)?;
        self.locator_unit_action_with_strict(
            py,
            frame_id,
            selector,
            timeout_ms,
            strict,
            LocatorAction::Click(options),
        )
    }

    #[cfg(feature = "touchscreen-input")]
    #[pyo3(signature = (frame_id, selector, options, strict=false, timeout_ms=30_000))]
    fn page_tap<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
        options: &Bound<'_, PyDict>,
        strict: bool,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let options = python_tap_options(options)?;
        self.locator_unit_action_with_strict(
            py,
            frame_id,
            selector,
            timeout_ms,
            strict,
            LocatorAction::Tap(options),
        )
    }

    #[cfg(feature = "locator-click-modifiers")]
    #[pyo3(signature = (frame_id, selector, modifiers, timeout_ms=30_000))]
    fn click_with_modifiers<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
        modifiers: &Bound<'_, PyList>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let modifiers = click_modifiers(modifiers)?;
        self.locator_unit_action(
            py,
            frame_id,
            selector,
            timeout_ms,
            LocatorAction::Click(ClickOptionsV2::new().with_modifiers(modifiers)),
        )
    }

    #[cfg(feature = "mouse")]
    #[pyo3(signature = (page_id, operation, x, y, options=(0.0, 1.0, 1.0, 0.0)))]
    fn mouse<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        operation: &str,
        x: f64,
        y: f64,
        options: (f64, f64, f64, f64),
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let operation_name = bounded_string(operation, 16, "Mouse operation")?;
        let operation =
            python_mouse_operation(&operation_name, x, y, options).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let start = host.admit_input_start().map_err(binding_error)?;
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            start.run(async move {
                match operation {
                    PythonMouseOperation::Move(params) => host.mouse_move(&page_id, params).await,
                    PythonMouseOperation::Down(params) => host.mouse_down(&page_id, params).await,
                    PythonMouseOperation::Up(params) => host.mouse_up(&page_id, params).await,
                    PythonMouseOperation::Click(params) => host.mouse_click(&page_id, params).await,
                    PythonMouseOperation::Dblclick(params) => {
                        host.mouse_dblclick(&page_id, params).await
                    }
                    PythonMouseOperation::Wheel(params) => host.mouse_wheel(&page_id, params).await,
                }
            }),
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "touchscreen-input")]
    fn touchscreen_tap<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        x: f64,
        y: f64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let x = TouchscreenCoordinate::try_from(x)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
        let y = TouchscreenCoordinate::try_from(y)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
        let host = Arc::clone(&self.host);
        let start = host.admit_input_start().map_err(binding_error)?;
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            start.run(async move { host.touchscreen_tap(&page_id, TapParams::new(x, y)).await }),
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "keyboard")]
    #[pyo3(signature = (page_id, operation, value, delay_ms=0.0))]
    fn keyboard<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        operation: &str,
        value: &str,
        delay_ms: f64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let operation_name = bounded_string(operation, 16, "Keyboard operation")?;
        let operation =
            python_keyboard_operation(&operation_name, value, delay_ms).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let start = host.admit_input_start().map_err(binding_error)?;
        self.async_command(
            py,
            FfiLane::Command,
            READ_ALLOWANCE,
            start.run(async move {
                match operation {
                    PythonKeyboardOperation::Down(key) => host.keyboard_down(&page_id, &key).await,
                    PythonKeyboardOperation::Up(key) => host.keyboard_up(&page_id, &key).await,
                    PythonKeyboardOperation::Press(key, delay) => {
                        host.keyboard_press(&page_id, &key, delay).await
                    }
                    PythonKeyboardOperation::InsertText(text) => {
                        host.keyboard_insert_text(&page_id, &text).await
                    }
                    PythonKeyboardOperation::Type(text, delay) => {
                        host.keyboard_type(&page_id, &text, delay).await
                    }
                }
            }),
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "keyboard"))]
    #[pyo3(signature = (_page_id, _operation, _value, _delay_ms=0.0))]
    fn keyboard<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _operation: &str,
        _value: &str,
        _delay_ms: f64,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page Keyboard capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "mouse"))]
    #[pyo3(signature = (_page_id, _operation, _x, _y, _options=(0.0, 1.0, 1.0, 0.0)))]
    fn mouse<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _operation: &str,
        _x: f64,
        _y: f64,
        _options: (f64, f64, f64, f64),
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page Mouse capability is not installed".into(),
        )))
    }

    #[cfg(feature = "drag-and-drop")]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (page_id, frame_id, source, target, timeout_ms=30_000, options=None))]
    fn drag_and_drop<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        frame_id: &str,
        source: &str,
        target: &str,
        timeout_ms: u32,
        options: Option<PythonDragOptions>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let (source_position, target_position, steps, trial, strict, force) =
            options.unwrap_or((None, None, 1.0, false, false, false));
        if force {
            return Err(binding_error(HostError::UnsupportedOperation(
                "Drag force mode is not installed".into(),
            )));
        }
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let source = self.prepare_locator_with_strict(frame_id, source, strict)?;
        let target = self.prepare_locator_with_strict(frame_id, target, strict)?;
        let source_position = source_position
            .map(|(x, y)| DragPosition::new(x, y))
            .transpose()
            .map_err(binding_error)?;
        let target_position = target_position
            .map(|(x, y)| DragPosition::new(x, y))
            .transpose()
            .map_err(binding_error)?;
        let options = DragToOptions::new(source_position, target_position, steps, trial)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            ACTION_ALLOWANCE,
            async move {
                let _host_lifetime = host;
                source.drag_to(&page_id, &target, timeout_ms, options).await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "drag-and-drop"))]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (_page_id, _frame_id, _source, _target, _timeout_ms=30_000, _options=None))]
    fn drag_and_drop<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _frame_id: &str,
        _source: &str,
        _target: &str,
        _timeout_ms: u32,
        _options: Option<PythonDragOptions>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.drag_and_drop capability is not installed".into(),
        )))
    }

    #[pyo3(signature = (frame_id, selector, value, timeout_ms=30_000, force=false))]
    fn fill<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
        value: &str,
        timeout_ms: u32,
        force: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let value = bounded_string(
            value,
            pwright_node_host::locator::action::MAX_VALUE_BYTES,
            "fill value",
        )?;
        self.locator_unit_action(
            py,
            frame_id,
            selector,
            timeout_ms,
            LocatorAction::Fill {
                value,
                options: FillOptions::new().with_force(force),
            },
        )
    }

    #[pyo3(signature = (frame_id, selector, key, timeout_ms=30_000, options=None))]
    fn press<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
        key: &str,
        timeout_ms: u32,
        options: Option<(Option<u32>, bool)>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let key = PressKey::new(key)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
        let (delay_ms, no_wait_after) = options.unwrap_or((None, false));
        let mut options = PressOptions::new().with_no_wait_after(no_wait_after);
        if let Some(delay_ms) = delay_ms {
            options = options.with_delay(std::time::Duration::from_millis(u64::from(delay_ms)));
        }
        self.locator_unit_action(
            py,
            frame_id,
            selector,
            timeout_ms,
            LocatorAction::Press { key, options },
        )
    }

    #[pyo3(signature = (frame_id, selector, checked, timeout_ms=30_000, options=None))]
    fn set_checked<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
        checked: bool,
        timeout_ms: u32,
        options: Option<(Option<bool>, Option<bool>)>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let (force, trial) = options.unwrap_or((None, None));
        let mut options = SetCheckedOptions::new();
        if let Some(force) = force {
            options = options.with_force(force);
        }
        if let Some(trial) = trial {
            options = options.with_trial(trial);
        }
        self.locator_unit_action(
            py,
            frame_id,
            selector,
            timeout_ms,
            LocatorAction::SetChecked { checked, options },
        )
    }

    #[pyo3(signature = (frame_id, selector, timeout_ms=30_000))]
    fn focus<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.locator_unit_action(py, frame_id, selector, timeout_ms, LocatorAction::Focus)
    }

    /// Resolve a Page selector call against its Context default. Locators do
    /// not use this policy: they are always strict by construction.
    #[pyo3(signature = (context_id, strict=None))]
    fn context_selector_is_strict(&self, context_id: &str, strict: Option<bool>) -> PyResult<bool> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        #[cfg(feature = "context-strict-selectors")]
        {
            return self
                .context_strict_selectors
                .resolve(&context_id, strict)
                .map_err(binding_error);
        }
        #[cfg(not(feature = "context-strict-selectors"))]
        {
            let _ = context_id;
            Ok(strict.unwrap_or(false))
        }
    }

    #[pyo3(signature = (frame_id, selector, operation, timeout_ms=30_000, attribute=None, strict=true))]
    fn locator_read(
        &self,
        frame_id: &str,
        selector: &str,
        operation: &str,
        timeout_ms: u32,
        attribute: Option<&str>,
        strict: bool,
    ) -> PyResult<Py<PyAny>> {
        let operation = operation.parse::<LocatorRead>().map_err(binding_error)?;
        let attribute = attribute
            .map(|value| bounded_string(value, 64 * 1024, "attribute"))
            .transpose()?;
        let locator = self.prepare_locator_with_strict(frame_id, selector, strict)?;
        Python::attach(|py| {
            self.async_command(
                py,
                FfiLane::Command,
                READ_ALLOWANCE,
                async move { locator.read(operation, timeout_ms, attribute).await },
                locator_value_to_python,
            )
            .map(Bound::unbind)
        })
    }

    #[pyo3(signature = (frame_id, selector, expected, timeout_ms=5_000, is_not=false))]
    fn locator_expect_count<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
        expected: u32,
        timeout_ms: u32,
        is_not: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let locator = self.prepare_locator(frame_id, selector)?;
        self.async_command(
            py,
            FfiLane::Command,
            READ_ALLOWANCE,
            async move { locator.expect_count(timeout_ms, expected, is_not).await },
            |py, result| {
                let value = PyDict::new(py);
                value.set_item("matches", result.matches)?;
                value.set_item("received", result.received)?;
                value.set_item("timed_out", result.timed_out)?;
                Ok(value.into_any().unbind())
            },
        )
    }

    #[pyo3(signature = (frame_id, selector, source, argument=None, timeout_ms=30_000))]
    fn eval_on_selector<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
        source: &str,
        argument: Option<&Bound<'py, PyAny>>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pwright_node_host::evaluation::{EvaluateParams, EvaluationValue, FFI_ALLOWANCE};

        if !evaluation_value::is_function_source(source) {
            return Err(binding_error(HostError::InvalidArgument(
                "Page.eval_on_selector expects a JavaScript function".into(),
            )));
        }
        let mut execution = self
            .owner
            .ffi
            .admit::<(Result<EvaluationValue, HostError>, FramePublicationFence)>(
                FfiLane::Command,
                FFI_ALLOWANCE,
            )
            .map_err(binding_error)?;
        let locator = self.prepare_locator(frame_id, selector)?;
        let source = bounded_string(
            source,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "source",
        )?;
        let argument = evaluation_value::from_python(argument)?;
        let params = EvaluateParams::function(source, argument);
        let publications = self.host.frame_publications();
        let lifecycle_callback = Arc::clone(&self.lifecycle_callback);
        let future = async move {
            let outcome = locator
                .evaluate_first_with(timeout_ms, move || async move { Ok(params) })
                .await;
            Ok((outcome, publications.fence()?))
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            ticket
                .deliver(|outcome| {
                    let (outcome, fence) = outcome.map_err(binding_error)?;
                    Python::attach(|py| {
                        publish_frame_fence(py, fence, &lifecycle_callback)?;
                        let value = outcome.map_err(binding_error)?;
                        evaluation_value::to_python(py, value)
                    })
                })
                .map_err(binding_error)?
        })
    }

    #[pyo3(signature = (frame_id, selector, source, argument=None))]
    fn eval_on_selector_all<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        selector: &str,
        source: &str,
        argument: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pwright_node_host::evaluation::{EvaluateParams, EvaluationValue, FFI_ALLOWANCE};

        if !evaluation_value::is_function_source(source) {
            return Err(binding_error(HostError::InvalidArgument(
                "Page.eval_on_selector_all expects a JavaScript function".into(),
            )));
        }
        let mut execution = self
            .owner
            .ffi
            .admit::<(Result<EvaluationValue, HostError>, FramePublicationFence)>(
                FfiLane::Command,
                FFI_ALLOWANCE,
            )
            .map_err(binding_error)?;
        let locator = self.prepare_locator(frame_id, selector)?;
        let source = bounded_string(
            source,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "source",
        )?;
        let argument = evaluation_value::from_python(argument)?;
        let params = EvaluateParams::function(source, argument);
        let publications = self.host.frame_publications();
        let lifecycle_callback = Arc::clone(&self.lifecycle_callback);
        let future = async move {
            let outcome = locator
                .evaluate_all_with(move || async move { Ok(params) })
                .await;
            Ok((outcome, publications.fence()?))
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            ticket
                .deliver(|outcome| {
                    let (outcome, fence) = outcome.map_err(binding_error)?;
                    Python::attach(|py| {
                        publish_frame_fence(py, fence, &lifecycle_callback)?;
                        let value = outcome.map_err(binding_error)?;
                        evaluation_value::to_python(py, value)
                    })
                })
                .map_err(binding_error)?
        })
    }

    #[pyo3(signature = (frame_id, source, argument=None))]
    fn evaluate<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        source: &str,
        argument: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pwright_node_host::evaluation::EvaluateParams;
        use pwright_node_host::evaluation::{EvaluationValue, FFI_ALLOWANCE};

        let mut execution = self
            .owner
            .ffi
            .admit::<(Result<EvaluationValue, HostError>, FramePublicationFence)>(
                FfiLane::Command,
                FFI_ALLOWANCE,
            )
            .map_err(binding_error)?;
        let frame_id = bounded_string(frame_id, MAX_OBJECT_ID_BYTES, "Frame ID")?;
        let source = bounded_string(
            source,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "source",
        )?;
        let is_function = evaluation_value::is_function_source(&source);
        #[cfg(feature = "handles")]
        let argument = evaluation_value::from_python_owned(argument)?;
        #[cfg(not(feature = "handles"))]
        let params = if is_function {
            EvaluateParams::function(source.clone(), evaluation_value::from_python(argument)?)
        } else {
            EvaluateParams::expression(source.clone())
        };
        let host = Arc::clone(&self.host);
        let publications = self.host.frame_publications();
        let lifecycle_callback = Arc::clone(&self.lifecycle_callback);
        #[cfg(feature = "handles")]
        let future = async move {
            let outcome = if argument.has_handles() {
                host.frame_evaluate_owned(&frame_id, source, is_function, argument)
                    .await
            } else {
                let argument = argument.into_value()?;
                let params = if is_function {
                    EvaluateParams::function(source, argument)
                } else {
                    EvaluateParams::expression(source)
                };
                host.frame_evaluate(&frame_id, params).await
            };
            Ok((outcome, publications.fence()?))
        };
        #[cfg(not(feature = "handles"))]
        let future = async move {
            let outcome = host.frame_evaluate(&frame_id, params).await;
            Ok((outcome, publications.fence()?))
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            ticket
                .deliver(|outcome| {
                    let (outcome, fence) = outcome.map_err(binding_error)?;
                    Python::attach(|py| {
                        publish_frame_fence(py, fence, &lifecycle_callback)?;
                        let value = outcome.map_err(binding_error)?;
                        evaluation_value::to_python(py, value)
                    })
                })
                .map_err(binding_error)?
        })
    }

    #[cfg(feature = "handles")]
    #[pyo3(signature = (frame_id, source, argument=None))]
    fn frame_evaluate_handle<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        source: &str,
        argument: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pwright_node_host::evaluation::FFI_ALLOWANCE;

        let frame_id = bounded_string(frame_id, MAX_OBJECT_ID_BYTES, "Frame ID")?;
        let source = bounded_string(
            source,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "source",
        )?;
        let is_function = evaluation_value::is_function_source(&source);
        let argument = evaluation_value::from_python_owned(argument)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            FFI_ALLOWANCE,
            async move {
                host.frame_prepare_handle_owned(&frame_id, source, is_function, argument)
                    .await
            },
            |py, value| Ok(Py::new(py, handles::NativeHandle::new(value))?.into_any()),
        )
    }

    #[cfg(feature = "wait-for-function")]
    #[pyo3(signature = (frame_id, source, argument=None, polling_ms=0, timeout_ms=30_000))]
    fn frame_wait_for_function<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        source: &str,
        argument: Option<&Bound<'py, PyAny>>,
        polling_ms: u32,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let frame_id = bounded_string(frame_id, MAX_OBJECT_ID_BYTES, "Frame ID")?;
        let source = bounded_string(
            source,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "source",
        )?;
        let is_function = evaluation_value::is_function_source(&source);
        let argument = evaluation_value::from_python(argument)?;
        let polling = if polling_ms == 0 {
            pwright_node_host::wait_for_function::Polling::AnimationFrame
        } else {
            pwright_node_host::wait_for_function::Polling::Interval(polling_ms)
        };
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                host.frame_wait_for_function(
                    &frame_id,
                    source,
                    is_function,
                    argument,
                    polling,
                    timeout_ms,
                )
                .await
            },
            |py, value| Ok(Py::new(py, handles::NativeHandle::new(value))?.into_any()),
        )
    }

    #[cfg(feature = "handles")]
    fn evaluate_handle_argument<'py>(
        &self,
        py: Python<'py>,
        frame_id: &str,
        source: &str,
        handle: &handles::NativeHandle,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pwright_node_host::{
            evaluation::{EvaluationValue, FFI_ALLOWANCE},
            handles::{HandleArgumentSlot, OwnedHandleArgument},
        };

        let frame_id = bounded_string(frame_id, MAX_OBJECT_ID_BYTES, "Frame ID")?;
        let source = bounded_string(
            source,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "source",
        )?;
        let is_function = evaluation_value::is_function_source(&source);
        let argument = OwnedHandleArgument::from_slots(
            EvaluationValue::Undefined,
            vec![HandleArgumentSlot::new(
                0,
                handle.read_frame().map_err(binding_error)?,
            )],
        )
        .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            FFI_ALLOWANCE,
            async move {
                host.frame_evaluate_owned(&frame_id, source, is_function, argument)
                    .await
            },
            evaluation_value::to_python,
        )
    }

    #[cfg(feature = "handles")]
    fn handle_json_value<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
    ) -> PyResult<Bound<'py, PyAny>> {
        #[cfg(feature = "worker-handle-evaluation")]
        let worker = handle.read_worker().map_err(binding_error)?;
        #[cfg(feature = "worker-handle-evaluation")]
        let frame = match worker {
            Some(_) => None,
            None => Some(handle.read_frame().map_err(binding_error)?),
        };
        #[cfg(not(feature = "worker-handle-evaluation"))]
        let frame = handle.read_frame().map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                #[cfg(feature = "worker-handle-evaluation")]
                if let Some(worker) = worker {
                    return host.worker_handle_json_value(&worker).await;
                }
                #[cfg(feature = "worker-handle-evaluation")]
                let frame = frame.expect("non-Worker Handle has a Frame owner");
                host.handle_json_value(&frame).await
            },
            evaluation_value::to_python,
        )
    }

    #[cfg(feature = "handles")]
    #[pyo3(signature = (handle, source, argument=None))]
    fn handle_evaluate<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        source: &str,
        argument: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pwright_node_host::evaluation::FFI_ALLOWANCE;

        let handle = handle.read_frame().map_err(binding_error)?;
        let source = bounded_string(
            source,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "source",
        )?;
        let is_function = evaluation_value::is_function_source(&source);
        let argument = evaluation_value::from_python_owned(argument)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            FFI_ALLOWANCE,
            async move {
                host.handle_evaluate_program_owned(&handle, source, is_function, argument)
                    .await
            },
            evaluation_value::to_python,
        )
    }

    #[cfg(feature = "handles")]
    #[pyo3(signature = (handle, source, argument=None))]
    fn handle_evaluate_handle<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        source: &str,
        argument: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pwright_node_host::evaluation::FFI_ALLOWANCE;

        let handle = handle.read_frame().map_err(binding_error)?;
        let source = bounded_string(
            source,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "source",
        )?;
        let is_function = evaluation_value::is_function_source(&source);
        let argument = evaluation_value::from_python_owned(argument)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            FFI_ALLOWANCE,
            async move {
                let value = host
                    .handle_prepare_owned(&handle, source, is_function, argument)
                    .await?;
                handles::with_node_preview(&host, value).await
            },
            |py, (value, preview)| {
                Ok(
                    Py::new(py, handles::NativeHandle::new_with_preview(value, preview))?
                        .into_any(),
                )
            },
        )
    }

    #[cfg(feature = "handles")]
    fn handle_get_property<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        name: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let name = bounded_string(name, 16 * 1024, "property name")?;
        #[cfg(feature = "worker-handle-evaluation")]
        let worker = handle.read_worker().map_err(binding_error)?;
        #[cfg(feature = "worker-handle-evaluation")]
        let frame = match worker {
            Some(_) => None,
            None => Some(handle.read_frame().map_err(binding_error)?),
        };
        #[cfg(not(feature = "worker-handle-evaluation"))]
        let frame = handle.read_frame().map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                #[cfg(feature = "worker-handle-evaluation")]
                if let Some(worker) = worker {
                    return host
                        .worker_handle_get_property(&worker, name)
                        .await
                        .map(handles::NativeHandle::from_worker);
                }
                #[cfg(feature = "worker-handle-evaluation")]
                let frame = frame.expect("non-Worker Handle has a Frame owner");
                host.handle_get_property(&frame, name)
                    .await
                    .map(handles::NativeHandle::new)
            },
            |py, value| Ok(Py::new(py, value)?.into_any()),
        )
    }

    #[cfg(feature = "handles")]
    fn handle_get_properties<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move { host.handle_get_properties(&handle).await },
            |py, values| {
                let output = PyDict::new(py);
                for value in values {
                    let name = value.name().to_owned();
                    let handle = Py::new(py, handles::NativeHandle::new(value.into_handle()))?;
                    output.set_item(name, handle)?;
                }
                Ok(output.into_any().unbind())
            },
        )
    }

    #[cfg(feature = "element-handle-click")]
    #[pyo3(signature = (handle, options, timeout_ms=30_000))]
    fn element_handle_click<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        options: &Bound<'_, PyDict>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let options = element_click_options(options)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move { host.element_handle_click(handle, options, timeout_ms).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "element-handle-tap")]
    #[pyo3(signature = (handle, options, timeout_ms=30_000))]
    fn element_handle_tap<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        options: &Bound<'_, PyDict>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let options = python_tap_options(options)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move { host.element_handle_tap(&handle, options, timeout_ms).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "element-handle-scroll-into-view")]
    #[pyo3(signature = (handle, timeout_ms=30_000))]
    fn element_handle_scroll_into_view_if_needed<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                host.element_handle_scroll_into_view_if_needed(&handle, timeout_ms)
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "element-handle-scroll-into-view"))]
    #[pyo3(signature = (_handle, _timeout_ms=30_000))]
    fn element_handle_scroll_into_view_if_needed<'py>(
        &self,
        _py: Python<'py>,
        _handle: &Bound<'py, PyAny>,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "ElementHandle.scroll_into_view_if_needed capability is not installed".into(),
        )))
    }

    #[cfg(feature = "element-handle-select-text")]
    #[pyo3(signature = (handle, force=false, timeout_ms=30_000))]
    fn element_handle_select_text<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        force: bool,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                host.element_handle_select_text(&handle, force, timeout_ms)
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "element-handle-select-text"))]
    #[pyo3(signature = (_handle, _force=false, _timeout_ms=30_000))]
    fn element_handle_select_text<'py>(
        &self,
        _py: Python<'py>,
        _handle: &Bound<'py, PyAny>,
        _force: bool,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "ElementHandle.select_text capability is not installed".into(),
        )))
    }

    #[cfg(feature = "element-handle-state-wait")]
    #[pyo3(signature = (handle, state, timeout_ms=30_000))]
    fn element_handle_wait_for_element_state<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        state: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let state = bounded_string(state, 32, "ElementHandle state")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                host.element_handle_wait_for_element_state(&handle, &state, timeout_ms)
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "element-handle-state-predicates")]
    fn element_handle_state_predicate<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        operation: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let operation = bounded_string(operation, 16, "ElementHandle state predicate")?;
        let operation = operation
            .parse::<ElementHandleStatePredicate>()
            .map_err(binding_error)?;
        let handle = handle.read_frame().map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                host.element_handle_state_predicate(&handle, operation)
                    .await
            },
            |py, value| Ok(value.into_pyobject(py)?.to_owned().into_any().unbind()),
        )
    }

    #[cfg(not(feature = "element-handle-state-predicates"))]
    fn element_handle_state_predicate<'py>(
        &self,
        _py: Python<'py>,
        _handle: &Bound<'py, PyAny>,
        _operation: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "ElementHandle state predicates capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "element-handle-state-wait"))]
    #[pyo3(signature = (_handle, _state, _timeout_ms=30_000))]
    fn element_handle_wait_for_element_state<'py>(
        &self,
        _py: Python<'py>,
        _handle: &Bound<'py, PyAny>,
        _state: &str,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "ElementHandle.wait_for_element_state capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "element-handle-click"))]
    #[pyo3(signature = (_handle, _options, _timeout_ms=30_000))]
    fn element_handle_click<'py>(
        &self,
        _py: Python<'py>,
        _handle: &Bound<'py, PyAny>,
        _options: &Bound<'py, PyAny>,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "ElementHandle.click capability is not installed".into(),
        )))
    }

    #[cfg(all(feature = "element-handle-dom", feature = "locator-dom-actions"))]
    #[pyo3(signature = (handle, event_type, event_init=None))]
    fn element_handle_dispatch_event<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        event_type: &str,
        event_init: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let event_type = bounded_string(
            event_type,
            pwright_node_host::locator::MAX_EVENT_TYPE_BYTES,
            "event type",
        )?;
        let event_init = evaluation_value::from_python(event_init)?;
        if !matches!(
            event_init,
            pwright_node_host::evaluation::EvaluationValue::Null
                | pwright_node_host::evaluation::EvaluationValue::Object { .. }
        ) {
            return Err(binding_error(HostError::InvalidArgument(
                "event initializer must be a dictionary or None".into(),
            )));
        }
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                host.element_handle_dispatch_event(&handle, event_type, event_init)
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(all(feature = "element-handle-dom", feature = "locator-dom-actions")))]
    #[pyo3(signature = (_handle, _event_type, _event_init=None))]
    fn element_handle_dispatch_event<'py>(
        &self,
        _py: Python<'py>,
        _handle: &Bound<'py, PyAny>,
        _event_type: &str,
        _event_init: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "ElementHandle.dispatch_event capability is not installed".into(),
        )))
    }

    #[cfg(all(feature = "element-handle-dom", feature = "locator-dom-actions"))]
    #[pyo3(signature = (handle, event_type, event_init=None))]
    fn element_handle_dispatch_event_owned<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        event_type: &str,
        event_init: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let event_type = bounded_string(
            event_type,
            pwright_node_host::locator::MAX_EVENT_TYPE_BYTES,
            "event type",
        )?;
        let event_init = evaluation_value::from_python_owned(event_init)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move {
                host.element_handle_dispatch_event_owned(&handle, event_type, event_init)
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(all(feature = "element-handle-dom", feature = "locator-dom-actions")))]
    #[pyo3(signature = (_handle, _event_type, _event_init=None))]
    fn element_handle_dispatch_event_owned<'py>(
        &self,
        _py: Python<'py>,
        _handle: &Bound<'py, PyAny>,
        _event_type: &str,
        _event_init: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "ElementHandle.dispatch_event with live Handles requires Handle capabilities".into(),
        )))
    }

    #[cfg(feature = "element-handle-dom")]
    fn element_handle_inner_text<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move { host.element_handle_inner_text(&handle).await },
            |py, value| Ok(value.into_pyobject(py)?.into_any().unbind()),
        )
    }

    #[cfg(feature = "element-handle-dom-reads")]
    #[pyo3(signature = (handle, operation, attribute=None))]
    fn element_handle_dom_read<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        operation: &str,
        attribute: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let operation = match operation {
            "getAttribute" => ElementHandleDomRead::GetAttribute,
            "innerHTML" => ElementHandleDomRead::InnerHtml,
            "inputValue" => ElementHandleDomRead::InputValue,
            "isChecked" => ElementHandleDomRead::IsChecked,
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "unknown ElementHandle DOM read operation".into(),
                )));
            }
        };
        let attribute = attribute
            .map(|value| bounded_string(value, 64 * 1024, "attribute name"))
            .transpose()?;
        let handle = handle.read_frame().map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            READ_ALLOWANCE,
            async move {
                host.element_handle_dom_read(&handle, operation, attribute)
                    .await
            },
            |py, value| match value {
                ElementHandleDomValue::Bool(value) => {
                    Ok(value.into_pyobject(py)?.to_owned().into_any().unbind())
                }
                ElementHandleDomValue::Text(Some(value)) => {
                    Ok(value.into_pyobject(py)?.into_any().unbind())
                }
                ElementHandleDomValue::Text(None) => Ok(py.None()),
            },
        )
    }

    #[cfg(not(feature = "element-handle-dom-reads"))]
    #[pyo3(signature = (_handle, _operation, _attribute=None))]
    fn element_handle_dom_read<'py>(
        &self,
        _py: Python<'py>,
        _handle: &Bound<'py, PyAny>,
        _operation: &str,
        _attribute: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "ElementHandle DOM reads capability is not installed".into(),
        )))
    }

    #[cfg(feature = "element-handle-dom")]
    fn element_handle_bounding_box<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move { host.element_handle_bounding_box(&handle).await },
            |py, value: Option<LocatorBoundingBox>| {
                let Some(value) = value else {
                    return Ok(py.None());
                };
                let result = PyDict::new(py);
                result.set_item("x", value.x)?;
                result.set_item("y", value.y)?;
                result.set_item("width", value.width)?;
                result.set_item("height", value.height)?;
                Ok(result.into_any().unbind())
            },
        )
    }

    #[cfg(not(feature = "element-handle-dom"))]
    fn element_handle_bounding_box<'py>(
        &self,
        _py: Python<'py>,
        _handle: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "ElementHandle.bounding_box capability is not installed".into(),
        )))
    }

    #[cfg(feature = "element-handle-dom")]
    fn element_handle_query_selector<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        selector: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let query = prepare_element_handle_query(&self.owner, selector)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move { host.element_handle_query_selector(&handle, query).await },
            |py, value| match value {
                Some(value) => Ok(Py::new(py, handles::NativeHandle::new(value))?.into_any()),
                None => Ok(py.None()),
            },
        )
    }

    #[cfg(feature = "element-handle-dom")]
    fn element_handle_owner_frame<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move { host.handle_owner_frame(&handle).await },
            |py, value| match value {
                Some(value) => Ok(value.into_pyobject(py)?.into_any().unbind()),
                None => Ok(py.None()),
            },
        )
    }

    #[cfg(feature = "element-handle-dom")]
    fn element_handle_content_frame<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move { host.handle_content_frame(&handle).await },
            |py, value| match value {
                Some(value) => Ok(value.into_pyobject(py)?.into_any().unbind()),
                None => Ok(py.None()),
            },
        )
    }

    #[cfg(all(feature = "element-handle-dom", feature = "keyboard"))]
    #[pyo3(signature = (handle, page_id, operation, value, delay_ms=0.0))]
    fn element_handle_keyboard<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
        page_id: &str,
        operation: &str,
        value: &str,
        delay_ms: f64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = handle.read_frame().map_err(binding_error)?;
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let operation_name = bounded_string(operation, 16, "ElementHandle keyboard operation")?;
        let operation =
            python_keyboard_operation(&operation_name, value, delay_ms).map_err(binding_error)?;
        if !matches!(
            &operation,
            PythonKeyboardOperation::Press(_, _) | PythonKeyboardOperation::Type(_, _)
        ) {
            return Err(binding_error(HostError::InvalidArgument(
                "ElementHandle keyboard operation must be press or type".into(),
            )));
        }
        let host = Arc::clone(&self.host);
        let start = host.admit_input_start().map_err(binding_error)?;
        self.async_command(
            py,
            FfiLane::Command,
            READ_ALLOWANCE,
            start.run(async move {
                host.element_handle_focus(&handle).await?;
                match operation {
                    PythonKeyboardOperation::Press(key, delay) => {
                        host.keyboard_press(&page_id, &key, delay).await
                    }
                    PythonKeyboardOperation::Type(text, delay) => {
                        host.keyboard_type(&page_id, &text, delay).await
                    }
                    _ => unreachable!("operation was restricted before scheduling"),
                }
            }),
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(all(feature = "element-handle-dom", feature = "keyboard")))]
    #[pyo3(signature = (_handle, _page_id, _operation, _value, _delay_ms=0.0))]
    fn element_handle_keyboard<'py>(
        &self,
        _py: Python<'py>,
        _handle: &Bound<'py, PyAny>,
        _page_id: &str,
        _operation: &str,
        _value: &str,
        _delay_ms: f64,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "ElementHandle keyboard capability is not installed".into(),
        )))
    }

    #[cfg(feature = "handles")]
    fn handle_dispose<'py>(
        &self,
        py: Python<'py>,
        handle: &handles::NativeHandle,
    ) -> PyResult<Bound<'py, PyAny>> {
        let release = handle.release().map_err(binding_error)?;
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { release.wait().await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "page-bindings")]
    fn page_expose_binding<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        name: &str,
        needs_handle: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let name = bounded_string(name, 256, "binding name")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.page_expose_binding(&page_id, name, needs_handle).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "context-bindings")]
    fn context_expose_binding<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        name: &str,
        needs_handle: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "BrowserContext ID")?;
        let name = bounded_string(name, 256, "binding name")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move {
                host.context_expose_binding(&context_id, name, needs_handle)
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "page-bindings")]
    fn next_page_binding_call<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Watch,
            SMALL_ALLOWANCE,
            async move { Ok(host.next_page_binding_call().await) },
            |py, call| match call {
                Some(call) => Ok((
                    call.id.to_string(),
                    call.context.get().to_string(),
                    call.page.get().to_string(),
                    call.frame.get().to_string(),
                    call.name,
                    call.arguments_json,
                    call.needs_handle,
                )
                    .into_pyobject(py)?
                    .into_any()
                    .unbind()),
                None => Ok(py.None()),
            },
        )
    }

    #[cfg(feature = "page-bindings")]
    fn acknowledge_page_binding_call<'py>(
        &self,
        py: Python<'py>,
        call_id: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let call_id = bounded_string(call_id, MAX_OBJECT_ID_BYTES, "binding call ID")?;
        let call_id = HostSession::parse_page_binding_call_id(&call_id).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.acknowledge_page_binding_call(call_id).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "page-bindings")]
    fn take_page_binding_handle<'py>(
        &self,
        py: Python<'py>,
        call_id: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let call_id = bounded_string(call_id, MAX_OBJECT_ID_BYTES, "binding call ID")?;
        let call_id = HostSession::parse_page_binding_call_id(&call_id).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            pwright_node_host::evaluation::FFI_ALLOWANCE,
            async move { host.take_page_binding_handle(call_id).await },
            |py, value| Ok(Py::new(py, handles::NativeHandle::new(value))?.into_any()),
        )
    }

    #[cfg(feature = "page-bindings")]
    fn deliver_page_binding_result<'py>(
        &self,
        py: Python<'py>,
        call_id: &str,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pwright_node_host::evaluation::{EvaluationProperty, EvaluationValue, FFI_ALLOWANCE};

        let call_id = bounded_string(call_id, MAX_OBJECT_ID_BYTES, "binding call ID")?;
        let call_id = HostSession::parse_page_binding_call_id(&call_id).map_err(binding_error)?;
        let value = evaluation_value::from_python(Some(value))?;
        let payload = EvaluationValue::Object {
            id: None,
            properties: vec![EvaluationProperty::new("result", value)],
        };
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            FFI_ALLOWANCE,
            async move { host.deliver_page_binding_value(call_id, payload).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "page-bindings")]
    fn deliver_page_binding_error<'py>(
        &self,
        py: Python<'py>,
        call_id: &str,
        name: &str,
        message: &str,
        stack: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pwright_node_host::evaluation::{EvaluationProperty, EvaluationValue, FFI_ALLOWANCE};

        let call_id = bounded_string(call_id, MAX_OBJECT_ID_BYTES, "binding call ID")?;
        let call_id = HostSession::parse_page_binding_call_id(&call_id).map_err(binding_error)?;
        let name = bounded_string(name, 1024, "binding error name")?;
        let message = bounded_string(
            message,
            pwright_node_host::evaluation::ERROR_BYTES,
            "binding error message",
        )?;
        let stack = stack
            .map(|value| {
                bounded_string(
                    value,
                    pwright_node_host::evaluation::ERROR_BYTES,
                    "binding error stack",
                )
            })
            .transpose()?;
        let payload = EvaluationValue::Object {
            id: None,
            properties: vec![EvaluationProperty::new(
                "error",
                EvaluationValue::Error {
                    name,
                    message,
                    stack,
                },
            )],
        };
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            FFI_ALLOWANCE,
            async move { host.deliver_page_binding_value(call_id, payload).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "page-routes")]
    #[pyo3(signature = (page_id, pattern, times=None))]
    fn page_route<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        pattern: &str,
        times: Option<u64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let pattern = bounded_string(pattern, 4 * 1024, "route pattern")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.page_route(&page_id, pattern, times).await },
            |py, registration| {
                Ok(registration
                    .to_string()
                    .into_pyobject(py)?
                    .into_any()
                    .unbind())
            },
        )
    }

    #[cfg(feature = "page-routes")]
    #[pyo3(signature = (context_id, pattern, times=None))]
    fn context_route<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        pattern: &str,
        times: Option<u64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let pattern = bounded_string(pattern, 4 * 1024, "route pattern")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.context_route(&context_id, pattern, times).await },
            |py, registration| {
                Ok(registration
                    .to_string()
                    .into_pyobject(py)?
                    .into_any()
                    .unbind())
            },
        )
    }

    #[cfg(feature = "page-routes")]
    fn page_unroute<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        registration_ids: Vec<u64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.page_unroute(&page_id, &registration_ids).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "page-routes")]
    fn context_unroute<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        registration_ids: Vec<u64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.context_unroute(&context_id, &registration_ids).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "page-routes")]
    fn page_unroute_all<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        wait: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.page_unroute_all(&page_id, wait).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "page-routes")]
    fn context_unroute_all<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        wait: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.context_unroute_all(&context_id, wait).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "page-routes"))]
    #[pyo3(signature = (_page_id, _pattern, _times=None))]
    fn page_route<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _pattern: &str,
        _times: Option<u64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.route capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "page-routes"))]
    #[pyo3(signature = (_context_id, _pattern, _times=None))]
    fn context_route<'py>(
        &self,
        _py: Python<'py>,
        _context_id: &str,
        _pattern: &str,
        _times: Option<u64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext.route capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "page-routes"))]
    fn page_unroute<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _registration_ids: Vec<u64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.unroute capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "page-routes"))]
    fn context_unroute<'py>(
        &self,
        _py: Python<'py>,
        _context_id: &str,
        _registration_ids: Vec<u64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext.unroute capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "page-routes"))]
    fn page_unroute_all<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _wait: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.unroute_all capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "page-routes"))]
    fn context_unroute_all<'py>(
        &self,
        _py: Python<'py>,
        _context_id: &str,
        _wait: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext.unroute_all capability is not installed".into(),
        )))
    }

    #[cfg(feature = "har-replay")]
    fn page_route_from_har<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        path: &str,
        pattern: Option<&str>,
        not_found: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let path = bounded_string(path, 4 * 1024, "HAR path")?;
        let pattern = pattern
            .map(|value| bounded_string(value, 4 * 1024, "HAR route pattern"))
            .transpose()?;
        let not_found =
            pwright_node_host::HarNotFoundAction::parse(not_found).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move {
                host.page_route_from_har(&page_id, &path, pattern, not_found)
                    .await
            },
            |py, registration| {
                Ok(registration
                    .to_string()
                    .into_pyobject(py)?
                    .into_any()
                    .unbind())
            },
        )
    }

    #[cfg(feature = "har-replay")]
    fn context_route_from_har<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        path: &str,
        pattern: Option<&str>,
        not_found: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let path = bounded_string(path, 4 * 1024, "HAR path")?;
        let pattern = pattern
            .map(|value| bounded_string(value, 4 * 1024, "HAR route pattern"))
            .transpose()?;
        let not_found =
            pwright_node_host::HarNotFoundAction::parse(not_found).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move {
                host.context_route_from_har(&context_id, &path, pattern, not_found)
                    .await
            },
            |py, registration| {
                Ok(registration
                    .to_string()
                    .into_pyobject(py)?
                    .into_any()
                    .unbind())
            },
        )
    }

    #[cfg(not(feature = "har-replay"))]
    fn context_route_from_har<'py>(
        &self,
        _py: Python<'py>,
        _context_id: &str,
        _path: &str,
        _pattern: Option<&str>,
        _not_found: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext.route_from_har capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "har-replay"))]
    fn page_route_from_har<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _path: &str,
        _pattern: Option<&str>,
        _not_found: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.route_from_har capability is not installed".into(),
        )))
    }

    #[cfg(feature = "page-routes")]
    fn next_page_route_call<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Watch,
            PAGE_ROUTE_ALLOWANCE,
            async move { Ok(host.next_page_route_call().await) },
            page_route_call_to_python,
        )
    }

    #[cfg(not(feature = "page-routes"))]
    fn next_page_route_call<'py>(&self, _py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.route capability is not installed".into(),
        )))
    }

    #[cfg(feature = "page-routes")]
    #[pyo3(signature = (call_id, url=None, method=None, headers=None, post_data=None))]
    fn page_route_fallback<'py>(
        &self,
        py: Python<'py>,
        call_id: &str,
        url: Option<String>,
        method: Option<String>,
        headers: Option<Vec<(String, String)>>,
        post_data: Option<Vec<u8>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let call_id = parse_portable_u64(call_id, "route call ID")?;
        let options = pwright_node_host::RouteContinue::new(url, method, headers, post_data)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            PAGE_ROUTE_CONTINUE_ALLOWANCE,
            async move { host.page_route_fallback(call_id, options).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "page-routes"))]
    #[pyo3(signature = (call_id, url=None, method=None, headers=None, post_data=None))]
    fn page_route_fallback<'py>(
        &self,
        py: Python<'py>,
        call_id: &str,
        url: Option<String>,
        method: Option<String>,
        headers: Option<Vec<(String, String)>>,
        post_data: Option<Vec<u8>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let _ = (py, call_id, url, method, headers, post_data);
        Err(binding_error(HostError::UnsupportedOperation(
            "Route.fallback capability is not installed".into(),
        )))
    }

    #[cfg(feature = "page-routes")]
    #[pyo3(signature = (call_id, url=None, method=None, headers=None, post_data=None))]
    fn page_route_continue<'py>(
        &self,
        py: Python<'py>,
        call_id: &str,
        url: Option<String>,
        method: Option<String>,
        headers: Option<Vec<(String, String)>>,
        post_data: Option<Vec<u8>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let call_id = parse_portable_u64(call_id, "route call ID")?;
        let options = pwright_node_host::RouteContinue::new(url, method, headers, post_data)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            PAGE_ROUTE_CONTINUE_ALLOWANCE,
            async move {
                host.page_route_action(call_id, pwright_node_host::RouteAction::Continue(options))
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "page-routes"))]
    #[pyo3(signature = (call_id, url=None, method=None, headers=None, post_data=None))]
    fn page_route_continue<'py>(
        &self,
        py: Python<'py>,
        call_id: &str,
        url: Option<String>,
        method: Option<String>,
        headers: Option<Vec<(String, String)>>,
        post_data: Option<Vec<u8>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let _ = (py, call_id, url, method, headers, post_data);
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.route capability is not installed".into(),
        )))
    }

    #[cfg(feature = "page-routes")]
    #[pyo3(signature = (call_id, error_code=None))]
    fn page_route_abort<'py>(
        &self,
        py: Python<'py>,
        call_id: &str,
        error_code: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let call_id = parse_portable_u64(call_id, "route call ID")?;
        let reason =
            pwright_node_host::RouteAbortReason::parse(error_code).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move {
                host.page_route_action(call_id, pwright_node_host::RouteAction::Abort(reason))
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "page-routes")]
    fn page_route_fulfill<'py>(
        &self,
        py: Python<'py>,
        call_id: &str,
        status: i64,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let call_id = parse_portable_u64(call_id, "route call ID")?;
        let fulfill =
            pwright_node_host::RouteFulfill::new(status, headers, body).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            PAGE_ROUTE_FULFILL_ALLOWANCE,
            async move {
                host.page_route_action(call_id, pwright_node_host::RouteAction::Fulfill(fulfill))
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "route-fulfill-api-response")]
    fn page_route_fulfill_response<'py>(
        &self,
        py: Python<'py>,
        call_id: &str,
        response: &api_request::NativeApiResponse,
        status: Option<i64>,
        headers: Option<Vec<(String, String)>>,
        content_type: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let call_id = parse_portable_u64(call_id, "route call ID")?;
        let fulfill = response
            .route_fulfill(status, headers, content_type)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            PAGE_ROUTE_FULFILL_ALLOWANCE,
            async move {
                host.page_route_action(call_id, pwright_node_host::RouteAction::Fulfill(fulfill))
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "route-fulfill-path")]
    fn page_route_fulfill_path<'py>(
        &self,
        py: Python<'py>,
        call_id: &str,
        status: i64,
        headers: Vec<(String, String)>,
        path: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let call_id = parse_portable_u64(call_id, "route call ID")?;
        let path = bounded_string(path, 4 * 1024, "Route.fulfill path")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            PAGE_ROUTE_FULFILL_ALLOWANCE,
            async move {
                let fulfill =
                    pwright_node_host::RouteFulfill::from_path(status, headers, &path).await?;
                host.page_route_action(call_id, pwright_node_host::RouteAction::Fulfill(fulfill))
                    .await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "route-fulfill-path"))]
    fn page_route_fulfill_path<'py>(
        &self,
        _py: Python<'py>,
        _call_id: &str,
        _status: i64,
        _headers: Vec<(String, String)>,
        _path: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Route.fulfill path capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "page-routes"))]
    fn page_route_fulfill<'py>(
        &self,
        _py: Python<'py>,
        _call_id: &str,
        _status: i64,
        _headers: Vec<(String, String)>,
        _body: Vec<u8>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Route.fulfill capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "page-routes"))]
    #[pyo3(signature = (_call_id, _error_code=None))]
    fn page_route_abort<'py>(
        &self,
        _py: Python<'py>,
        _call_id: &str,
        _error_code: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.route capability is not installed".into(),
        )))
    }

    #[pyo3(signature = (page_id, url, wait_until="load", referrer=None, timeout_ms=30_000))]
    fn goto<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        url: &str,
        wait_until: &str,
        referrer: Option<&str>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let url = bounded_string(url, MAX_URL_BYTES, "Navigation URL")?;
        let wait_until = bounded_string(wait_until, 32, "wait_until")?;
        let referrer = referrer
            .map(|value| bounded_string(value, MAX_URL_BYTES, "referrer"))
            .transpose()?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            NAVIGATION_ALLOWANCE,
            async move {
                host.page_goto(&page_id, url, &wait_until, referrer, timeout_ms)
                    .await
            },
            navigation_to_python,
        )
    }

    #[cfg(feature = "frame-navigation")]
    #[pyo3(signature = (page_id, frame_id, url, wait_until="load", referrer=None, timeout_ms=30_000))]
    fn frame_goto<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        frame_id: &str,
        url: &str,
        wait_until: &str,
        referrer: Option<&str>,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let frame_id = bounded_string(frame_id, MAX_OBJECT_ID_BYTES, "Frame ID")?;
        let url = bounded_string(url, MAX_URL_BYTES, "Navigation URL")?;
        let wait_until = bounded_string(wait_until, 32, "wait_until")?;
        let referrer = referrer
            .map(|value| bounded_string(value, MAX_URL_BYTES, "referrer"))
            .transpose()?;
        let pending = self
            .host
            .start_frame_goto_public(page_id, frame_id, url, &wait_until, referrer, timeout_ms)
            .map_err(binding_error)?;
        self.async_command(
            py,
            FfiLane::Command,
            NAVIGATION_ALLOWANCE,
            pending,
            frame_navigation_to_python,
        )
    }

    #[cfg(not(feature = "frame-navigation"))]
    #[pyo3(signature = (_page_id, _frame_id, _url, _wait_until="load", _referrer=None, _timeout_ms=30_000))]
    fn frame_goto<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _frame_id: &str,
        _url: &str,
        _wait_until: &str,
        _referrer: Option<&str>,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Frame.goto capability is not installed".into(),
        )))
    }

    #[pyo3(signature = (page_id, wait_until="load", timeout_ms=30_000))]
    fn reload<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        wait_until: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let wait_until = bounded_string(wait_until, 32, "wait_until")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            NAVIGATION_ALLOWANCE,
            async move { host.page_reload(&page_id, &wait_until, timeout_ms).await },
            navigation_to_python,
        )
    }

    #[cfg(feature = "history-navigation")]
    #[pyo3(signature = (page_id, wait_until="load", timeout_ms=30_000))]
    fn go_back<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        wait_until: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let wait_until = bounded_string(wait_until, 32, "wait_until")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            NAVIGATION_ALLOWANCE,
            async move { host.page_go_back(&page_id, &wait_until, timeout_ms).await },
            |py, result| match result {
                Some(result) => navigation_to_python(py, result),
                None => Ok(py.None()),
            },
        )
    }

    #[cfg(not(feature = "history-navigation"))]
    #[pyo3(signature = (_page_id, _wait_until="load", _timeout_ms=30_000))]
    fn go_back<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _wait_until: &str,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.go_back capability is not installed".into(),
        )))
    }

    #[cfg(feature = "history-navigation")]
    #[pyo3(signature = (page_id, wait_until="load", timeout_ms=30_000))]
    fn go_forward<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        wait_until: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let wait_until = bounded_string(wait_until, 32, "wait_until")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            NAVIGATION_ALLOWANCE,
            async move {
                host.page_go_forward(&page_id, &wait_until, timeout_ms)
                    .await
            },
            |py, result| match result {
                Some(result) => navigation_to_python(py, result),
                None => Ok(py.None()),
            },
        )
    }

    #[cfg(not(feature = "history-navigation"))]
    #[pyo3(signature = (_page_id, _wait_until="load", _timeout_ms=30_000))]
    fn go_forward<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _wait_until: &str,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.go_forward capability is not installed".into(),
        )))
    }

    #[cfg(feature = "frame-navigation")]
    #[pyo3(signature = (page_id, frame_id, wait_until="load", timeout_ms=30_000))]
    fn wait_for_navigation<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        frame_id: &str,
        wait_until: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let frame_id = bounded_string(frame_id, MAX_OBJECT_ID_BYTES, "Frame ID")?;
        let wait_until = bounded_string(wait_until, 32, "wait_until")?;
        let wait_until = frame_wait_until(&wait_until).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let pending =
            host.start_wait_for_frame_navigation(page_id, frame_id, wait_until, timeout_ms);
        self.async_command(
            py,
            FfiLane::Command,
            NAVIGATION_ALLOWANCE,
            pending,
            frame_navigation_to_python,
        )
    }

    #[cfg(feature = "navigation-url-match")]
    #[pyo3(signature = (page_id, frame_id, url, wait_until="load", timeout_ms=30_000))]
    fn wait_for_navigation_matching<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        frame_id: &str,
        url: &Bound<'py, PyAny>,
        wait_until: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit::<(
                PyResult<pwright_node_host::FrameGotoOutput>,
                FramePublicationFence,
            )>(FfiLane::Command, NAVIGATION_ALLOWANCE)
            .map_err(binding_error)?;
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let frame_id = bounded_string(frame_id, MAX_OBJECT_ID_BYTES, "Frame ID")?;
        let wait_until = bounded_string(wait_until, 32, "wait_until")?;
        let wait_until = frame_wait_until(&wait_until).map_err(binding_error)?;
        let matcher = if let Ok(glob) = url.extract::<String>() {
            let glob = bounded_string(&glob, MAX_URL_BYTES, "URL glob")?;
            let plan = self
                .host
                .page_url_glob(&page_id, &glob)
                .map_err(binding_error)?;
            py.import("re")?
                .getattr("compile")?
                .call1((plan.pattern(),))?
                .getattr("search")?
                .unbind()
        } else if url.hasattr("search")? {
            let search = url.getattr("search")?;
            if !search.is_callable() {
                return Err(PyTypeError::new_err("url.search must be callable"));
            }
            search.unbind()
        } else if url.is_callable() {
            url.clone().unbind()
        } else {
            return Err(PyTypeError::new_err(
                "url must be a string, compiled regex, or callable",
            ));
        };
        let host = Arc::clone(&self.host);
        let pending = host.start_wait_for_frame_navigation_matching(
            page_id,
            frame_id,
            wait_until,
            timeout_ms,
            move |candidate| {
                // This callback is on the Tokio executor, after the native
                // lifecycle lock has been released. It does not emulate the
                // caller's asyncio-loop execution context.
                Python::attach(|py| matcher.bind(py).call1((candidate,))?.is_truthy())
            },
        );
        let publications = self.host.frame_publications();
        let lifecycle_callback = Arc::clone(&self.lifecycle_callback);
        let future = async move {
            let result = match pending.await {
                Ok(value) => Ok(value),
                Err(pwright_node_host::NavigationMatchError::Host(error)) => {
                    Err(binding_error(error))
                }
                Err(pwright_node_host::NavigationMatchError::Matcher(error)) => Err(error),
            };
            Ok((result, publications.fence()?))
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            ticket
                .deliver(|outcome| {
                    let (outcome, fence) = outcome.map_err(binding_error)?;
                    Python::attach(|py| {
                        publish_frame_fence(py, fence, &lifecycle_callback)?;
                        frame_navigation_to_python(py, outcome?)
                    })
                })
                .map_err(binding_error)?
        })
    }

    #[cfg(not(feature = "navigation-url-match"))]
    #[pyo3(signature = (_page_id, _frame_id, _url, _wait_until="load", _timeout_ms=30_000))]
    fn wait_for_navigation_matching<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _frame_id: &str,
        _url: &Bound<'py, PyAny>,
        _wait_until: &str,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.expect_navigation(url=...) capability is not installed".into(),
        )))
    }

    #[cfg(feature = "frame-navigation")]
    #[pyo3(signature = (page_id, frame_id, wait_until="load", timeout_ms=30_000))]
    fn wait_for_load_state<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        frame_id: &str,
        wait_until: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let frame_id = bounded_string(frame_id, MAX_OBJECT_ID_BYTES, "Frame ID")?;
        let wait_until = bounded_string(wait_until, 32, "wait_until")?;
        let wait_until = frame_wait_until(&wait_until).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let pending =
            host.start_wait_for_frame_load_state(page_id, frame_id, wait_until, timeout_ms);
        self.async_command(
            py,
            FfiLane::Command,
            NAVIGATION_ALLOWANCE,
            pending,
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "frame-navigation"))]
    #[pyo3(signature = (_page_id, _frame_id, _wait_until="load", _timeout_ms=30_000))]
    fn wait_for_navigation<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _frame_id: &str,
        _wait_until: &str,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.expect_navigation capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "frame-navigation"))]
    #[pyo3(signature = (_page_id, _frame_id, _wait_until="load", _timeout_ms=30_000))]
    fn wait_for_load_state<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _frame_id: &str,
        _wait_until: &str,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.wait_for_load_state capability is not installed".into(),
        )))
    }

    #[cfg(feature = "wait-for-url")]
    #[pyo3(signature = (page_id, url, wait_until="load", timeout_ms=30_000))]
    fn wait_for_url<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        url: &Bound<'py, PyAny>,
        wait_until: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut execution = self
            .owner
            .ffi
            .admit::<(PyResult<String>, FramePublicationFence)>(
                FfiLane::Command,
                NAVIGATION_ALLOWANCE,
            )
            .map_err(binding_error)?;
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let wait_until = bounded_string(wait_until, 32, "wait_until")?;
        let matcher = if let Ok(glob) = url.extract::<String>() {
            let glob = bounded_string(&glob, MAX_URL_BYTES, "URL glob")?;
            let plan = self
                .host
                .page_url_glob(&page_id, &glob)
                .map_err(binding_error)?;
            py.import("re")?
                .getattr("compile")?
                .call1((plan.pattern(),))?
                .getattr("search")?
                .unbind()
        } else if url.hasattr("search")? {
            let search = url.getattr("search")?;
            if !search.is_callable() {
                return Err(PyTypeError::new_err("url.search must be callable"));
            }
            search.unbind()
        } else if url.is_callable() {
            url.clone().unbind()
        } else {
            return Err(PyTypeError::new_err(
                "url must be a string, compiled regex, or callable",
            ));
        };
        let host = Arc::clone(&self.host);
        let publications = self.host.frame_publications();
        let lifecycle_callback = Arc::clone(&self.lifecycle_callback);
        let future = async move {
            let result = host
                .page_wait_for_url(&page_id, &wait_until, timeout_ms, move |candidate| {
                    Python::attach(|py| matcher.bind(py).call1((candidate,))?.is_truthy())
                })
                .await;
            let result = match result {
                Ok(value) => Ok(value),
                Err(pwright_node_host::UrlWaitError::Host(error)) => Err(binding_error(error)),
                Err(pwright_node_host::UrlWaitError::Matcher(error)) => Err(error),
            };
            Ok((result, publications.fence()?))
        };
        execution
            .reserve_future(std::mem::size_of_val(&future))
            .map_err(binding_error)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ticket = execution.run(future).await;
            ticket
                .deliver(|outcome| {
                    let (outcome, fence) = outcome.map_err(binding_error)?;
                    Python::attach(|py| {
                        publish_frame_fence(py, fence, &lifecycle_callback)?;
                        let value = outcome?;
                        Ok(value.into_pyobject(py)?.into_any().unbind())
                    })
                })
                .map_err(binding_error)?
        })
    }

    #[cfg(not(feature = "wait-for-url"))]
    #[pyo3(signature = (_page_id, _url, _wait_until="load", _timeout_ms=30_000))]
    fn wait_for_url<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _url: &Bound<'py, PyAny>,
        _wait_until: &str,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page.wait_for_url capability is not installed".into(),
        )))
    }

    #[cfg(feature = "network-events")]
    #[pyo3(signature = (page_id, kind, timeout_ms=30_000))]
    fn arm_network_wait<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        kind: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let kind = NetworkEventKind::parse(kind).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.page_arm_network_wait(&page_id, kind, timeout_ms).await },
            |py, waiter_id| Ok(waiter_id.into_pyobject(py)?.into_any().unbind()),
        )
    }

    #[cfg(not(feature = "network-events"))]
    #[pyo3(signature = (_page_id, _kind, _timeout_ms=30_000))]
    fn arm_network_wait<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _kind: &str,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page network events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "context-network-events")]
    #[pyo3(signature = (context_id, kind, timeout_ms=30_000))]
    fn arm_context_network_wait<'py>(
        &self,
        py: Python<'py>,
        context_id: &str,
        kind: &str,
        timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let kind = NetworkEventKind::parse(kind).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move {
                host.context_arm_network_wait(&context_id, kind, timeout_ms)
                    .await
            },
            |py, waiter_id| Ok(waiter_id.into_pyobject(py)?.into_any().unbind()),
        )
    }

    #[cfg(not(feature = "context-network-events"))]
    #[pyo3(signature = (_context_id, _kind, _timeout_ms=30_000))]
    fn arm_context_network_wait<'py>(
        &self,
        _py: Python<'py>,
        _context_id: &str,
        _kind: &str,
        _timeout_ms: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "BrowserContext network events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "network-events")]
    fn next_network_event<'py>(
        &self,
        py: Python<'py>,
        waiter_id: u64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Watch,
            NETWORK_EVENT_ALLOWANCE,
            async move { host.page_next_network_event(waiter_id).await },
            network_event_to_python,
        )
    }

    #[cfg(not(feature = "network-events"))]
    fn next_network_event<'py>(
        &self,
        _py: Python<'py>,
        _waiter_id: u64,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page network events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "request-events")]
    fn next_request_notice<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Watch,
            NETWORK_EVENT_ALLOWANCE,
            async move { host.next_request_notice().await },
            request_notice_to_python,
        )
    }

    #[cfg(not(feature = "request-events"))]
    fn next_request_notice<'py>(&self, _py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page request events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "response-events")]
    fn next_response_notice<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Watch,
            NETWORK_EVENT_ALLOWANCE,
            async move { host.next_response_notice().await },
            response_notice_to_python,
        )
    }

    #[cfg(feature = "network-events")]
    fn network_notice_fence(&self) -> PyResult<(u64, u64)> {
        self.host.network_notice_fence().map_err(binding_error)
    }

    #[cfg(not(feature = "network-events"))]
    fn network_notice_fence(&self) -> PyResult<(u64, u64)> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page network events capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "response-events"))]
    fn next_response_notice<'py>(&self, _py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page response events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "download-events")]
    fn next_download_notice<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Watch,
            DOWNLOAD_EVENT_ALLOWANCE,
            async move { host.next_download_notice().await },
            download_notice_to_python,
        )
    }

    #[cfg(not(feature = "download-events"))]
    fn next_download_notice<'py>(&self, _py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page download events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "download-events")]
    fn download_path<'py>(
        &self,
        py: Python<'py>,
        download: PyRef<'_, NativeDownload>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let owner = Arc::clone(&download.owner);
        self.async_command(
            py,
            FfiLane::Watch,
            DOWNLOAD_EVENT_ALLOWANCE,
            async move { owner.path().await },
            |py, path| Ok(path.into_pyobject(py)?.into_any().unbind()),
        )
    }

    #[cfg(not(feature = "download-events"))]
    fn download_path<'py>(
        &self,
        _py: Python<'py>,
        _download: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page download events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "worker-events")]
    fn next_worker_notice<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Watch,
            WORKER_EVENT_ALLOWANCE,
            async move { host.next_worker_notice().await },
            worker_notice_to_python,
        )
    }

    #[cfg(not(feature = "worker-events"))]
    fn next_worker_notice<'py>(&self, _py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page Worker events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "worker-console-events")]
    fn next_worker_console_notice<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Watch,
            WORKER_CONSOLE_EVENT_ALLOWANCE,
            async move { host.next_worker_console_notice().await },
            worker_console_notice_to_python,
        )
    }

    #[cfg(not(feature = "worker-console-events"))]
    fn next_worker_console_notice<'py>(&self, _py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Worker console events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "worker-events")]
    fn page_workers<'py>(&self, py: Python<'py>, page_id: &str) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let workers = self.host.page_workers(&page_id).map_err(binding_error)?;
        let values = PyList::empty(py);
        for owner in workers {
            values.append(worker_owner_to_python(py, owner)?)?;
        }
        Ok(values.into_any())
    }

    #[cfg(not(feature = "worker-events"))]
    fn page_workers<'py>(&self, _py: Python<'py>, _page_id: &str) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page Worker events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "worker-events")]
    #[pyo3(signature = (worker, source, argument=None))]
    fn worker_evaluate<'py>(
        &self,
        py: Python<'py>,
        worker: PyRef<'_, NativeWorker>,
        source: &str,
        argument: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pwright_node_host::evaluation::{EvaluateParams, FFI_ALLOWANCE};

        let source = bounded_string(
            source,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "source",
        )?;
        let argument = evaluation_value::from_python(argument)?;
        let params = if evaluation_value::is_function_source(&source) {
            EvaluateParams::function(source, argument)
        } else {
            EvaluateParams::expression(source)
        };
        let owner = Arc::clone(&worker.owner);
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            FFI_ALLOWANCE,
            async move { host.worker_evaluate(owner, params).await },
            evaluation_value::to_python,
        )
    }

    #[cfg(feature = "worker-handle-evaluation")]
    #[pyo3(signature = (worker, source, argument=None))]
    fn worker_evaluate_handle<'py>(
        &self,
        py: Python<'py>,
        worker: PyRef<'_, NativeWorker>,
        source: &str,
        argument: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pwright_node_host::evaluation::FFI_ALLOWANCE;

        let source = bounded_string(
            source,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "source",
        )?;
        let argument = evaluation_value::from_python(argument)?;
        let params = if evaluation_value::is_function_source(&source) {
            pwright_node_host::handles::EvaluateHandleParams::function(source, argument)
        } else {
            pwright_node_host::handles::EvaluateHandleParams::expression(source)
        };
        let owner = Arc::clone(&worker.owner);
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            FFI_ALLOWANCE,
            async move { host.worker_evaluate_handle(owner, params).await },
            |py, value| Ok(Py::new(py, handles::NativeHandle::from_worker(value))?.into_any()),
        )
    }

    #[cfg(not(feature = "worker-handle-evaluation"))]
    #[pyo3(signature = (_worker, _source, _argument=None))]
    fn worker_evaluate_handle<'py>(
        &self,
        _py: Python<'py>,
        _worker: &Bound<'py, PyAny>,
        _source: &str,
        _argument: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Worker Handle evaluation capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "worker-events"))]
    #[pyo3(signature = (_worker, _source, _argument=None))]
    fn worker_evaluate<'py>(
        &self,
        _py: Python<'py>,
        _worker: &Bound<'py, PyAny>,
        _source: &str,
        _argument: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page Worker events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "network-events")]
    fn cancel_network_wait<'py>(
        &self,
        py: Python<'py>,
        waiter_id: u64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { host.page_cancel_network_wait(waiter_id).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "network-events"))]
    fn cancel_network_wait<'py>(
        &self,
        _py: Python<'py>,
        _waiter_id: u64,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page network events capability is not installed".into(),
        )))
    }

    #[cfg(feature = "network-events")]
    fn request_sizes<'py>(
        &self,
        py: Python<'py>,
        owner: PyRef<'_, NativeRequestSizeOwner>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let owner = Arc::clone(&owner.owner);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { owner.sizes().await },
            request_sizes_to_python,
        )
    }

    #[cfg(feature = "network-events")]
    fn response_body<'py>(
        &self,
        py: Python<'py>,
        owner: PyRef<'_, NativeResponseBody>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let body = Arc::clone(&owner.body);
        let allowance = u64::try_from(body.len())
            .map_err(|error| binding_error(HostError::ResourceExhausted(error.to_string())))?
            .checked_add(4096)
            .ok_or_else(|| {
                binding_error(HostError::ResourceExhausted(
                    "response-body FFI allowance overflow".into(),
                ))
            })?;
        self.async_command(
            py,
            FfiLane::Watch,
            allowance,
            async move { Ok(body) },
            |py, body| Ok(PyBytes::new(py, body.as_ref()).into_any().unbind()),
        )
    }

    #[cfg(feature = "response-body")]
    fn frame_response_body<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        frame_id: &str,
        request_id: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let frame_id = bounded_string(frame_id, MAX_OBJECT_ID_BYTES, "Frame ID")?;
        let request_id = bounded_string(request_id, 256, "response request ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Command,
            2 * pwright_node_host::response_body::MAX_BODY_BYTES as u64 + 4096,
            async move {
                host.frame_response_body(&page_id, &frame_id, &request_id)
                    .await
            },
            |py, body| Ok(PyBytes::new(py, body.as_bytes()).into_any().unbind()),
        )
    }

    #[cfg(not(feature = "response-body"))]
    fn frame_response_body<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _frame_id: &str,
        _request_id: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Response.body capability is not installed".into(),
        )))
    }

    #[cfg(feature = "response-finished")]
    fn response_finished<'py>(
        &self,
        py: Python<'py>,
        owner: PyRef<'_, NativeResponseFinishedOwner>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let owner = Arc::clone(&owner.owner);
        self.async_command(
            py,
            FfiLane::Watch,
            SMALL_ALLOWANCE,
            async move { owner.finished().await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(feature = "request-history")]
    fn request_history_response<'py>(
        &self,
        py: Python<'py>,
        owner: PyRef<'_, NativeRequestHistoryOwner>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let owner = Arc::clone(&owner.owner);
        self.async_command(
            py,
            FfiLane::Watch,
            SMALL_ALLOWANCE,
            async move { owner.response().await },
            |py, response| match response {
                Some(response) => Ok(response_record_to_python(py, &response)?
                    .into_any()
                    .unbind()),
                None => Ok(py.None()),
            },
        )
    }

    #[cfg(feature = "request-history")]
    fn request_response_by_native_id<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        native_request_id: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let native_request_id = bounded_string(
            native_request_id,
            pwright_node_host::network_events::MAX_REQUEST_HISTORY_NATIVE_ID_BYTES,
            "native request ID",
        )?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Watch,
            REQUEST_HISTORY_ALLOWANCE,
            async move {
                host.request_response_by_native_id(&page_id, &native_request_id)
                    .await
            },
            |py, response| match response {
                Some(response) => Ok(response_record_to_python(py, &response)?
                    .into_any()
                    .unbind()),
                None => Ok(py.None()),
            },
        )
    }

    #[cfg(feature = "request-history")]
    fn request_by_native_id<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
        native_request_id: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let native_request_id = bounded_string(
            native_request_id,
            pwright_node_host::network_events::MAX_REQUEST_HISTORY_NATIVE_ID_BYTES,
            "native request ID",
        )?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Watch,
            REQUEST_HISTORY_ALLOWANCE,
            async move {
                host.request_by_native_id(&page_id, &native_request_id)
                    .await
            },
            request_history_to_python,
        )
    }

    #[cfg(not(feature = "request-history"))]
    fn request_history_response<'py>(
        &self,
        _py: Python<'py>,
        _owner: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Request.response capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "request-history"))]
    fn request_response_by_native_id<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _native_request_id: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Request.response capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "request-history"))]
    fn request_by_native_id<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
        _native_request_id: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Request history capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "network-events"))]
    fn response_body<'py>(
        &self,
        _py: Python<'py>,
        _owner: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Response.body capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "response-finished"))]
    fn response_finished<'py>(
        &self,
        _py: Python<'py>,
        _owner: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Response.finished capability is not installed".into(),
        )))
    }

    #[cfg(not(feature = "network-events"))]
    fn request_sizes<'py>(
        &self,
        _py: Python<'py>,
        _owner: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Request.sizes capability is not installed".into(),
        )))
    }

    fn close_page<'py>(&self, py: Python<'py>, page_id: &str) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Close,
            SMALL_ALLOWANCE,
            async move { host.close_page_with_disposition(&page_id).await },
            |py, disposition| {
                Ok(
                    PyBool::new(py, matches!(disposition, ClosePageDisposition::Context))
                        .to_owned()
                        .into_any()
                        .unbind(),
                )
            },
        )
    }

    #[cfg(feature = "page-dialogs")]
    fn run_before_unload<'py>(
        &self,
        py: Python<'py>,
        page_id: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let page_id = bounded_string(page_id, MAX_OBJECT_ID_BYTES, "Page ID")?;
        let host = Arc::clone(&self.host);
        self.async_command(
            py,
            FfiLane::Close,
            SMALL_ALLOWANCE,
            async move { host.run_before_unload(&page_id).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "page-dialogs"))]
    fn run_before_unload<'py>(
        &self,
        _py: Python<'py>,
        _page_id: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page dialog capability is not installed".into(),
        )))
    }

    #[cfg(feature = "page-dialogs")]
    #[pyo3(signature = (dialog, accept, prompt_text=None))]
    fn handle_dialog<'py>(
        &self,
        py: Python<'py>,
        dialog: PyRef<'_, NativeDialog>,
        accept: bool,
        prompt_text: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let prompt_text = prompt_text
            .map(|value| bounded_string(value, 16 * 1024, "dialog prompt text"))
            .transpose()?;
        let action = Arc::clone(&dialog.action);
        self.async_command(
            py,
            FfiLane::Command,
            SMALL_ALLOWANCE,
            async move { action.handle(accept, prompt_text.as_deref()).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "page-dialogs"))]
    #[pyo3(signature = (_dialog, _accept, _prompt_text=None))]
    fn handle_dialog<'py>(
        &self,
        _py: Python<'py>,
        _dialog: &Bound<'py, PyAny>,
        _accept: bool,
        _prompt_text: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page dialog capability is not installed".into(),
        )))
    }

    #[cfg(feature = "file-chooser-events")]
    fn set_file_chooser_payloads<'py>(
        &self,
        py: Python<'py>,
        chooser: PyRef<'_, NativeFileChooser>,
        payloads: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        struct Payload {
            name: String,
            mime_type: String,
            buffer: Py<PyBytes>,
        }

        let payloads = payloads.cast::<PyList>().map_err(|_| {
            binding_error(HostError::InvalidArgument(
                "file payloads must be a list".into(),
            ))
        })?;
        if payloads.len() > pwright_node_host::file_input::profile::ITEMS {
            return Err(binding_error(HostError::InvalidArgument(
                "file-input count exceeds selected limit".into(),
            )));
        }
        let mut values = Vec::with_capacity(payloads.len());
        for payload in payloads.iter() {
            let payload = payload.cast::<PyDict>().map_err(|_| {
                binding_error(HostError::InvalidArgument(
                    "file payload must be a mapping".into(),
                ))
            })?;
            let name = payload
                .get_item("name")?
                .ok_or_else(|| {
                    binding_error(HostError::InvalidArgument(
                        "file payload name is required".into(),
                    ))
                })?
                .cast_into::<PyString>()
                .map_err(|_| {
                    binding_error(HostError::InvalidArgument(
                        "file payload name must be a string".into(),
                    ))
                })?;
            let mime_type = payload
                .get_item("mimeType")?
                .ok_or_else(|| {
                    binding_error(HostError::InvalidArgument(
                        "file payload MIME is required".into(),
                    ))
                })?
                .cast_into::<PyString>()
                .map_err(|_| {
                    binding_error(HostError::InvalidArgument(
                        "file payload MIME must be a string".into(),
                    ))
                })?;
            let buffer = payload
                .get_item("buffer")?
                .ok_or_else(|| {
                    binding_error(HostError::InvalidArgument(
                        "file payload buffer is required".into(),
                    ))
                })?
                .cast_into::<PyBytes>()
                .map_err(|_| {
                    binding_error(HostError::InvalidArgument(
                        "file payload buffer must be bytes".into(),
                    ))
                })?;
            values.push(Payload {
                name: bounded_string(
                    &name.to_cow()?,
                    pwright_node_host::file_input::profile::NAME_BYTES,
                    "file payload name",
                )?,
                mime_type: bounded_string(
                    &mime_type.to_cow()?,
                    pwright_node_host::file_input::profile::MIME_BYTES,
                    "file payload MIME",
                )?,
                buffer: buffer.unbind(),
            });
        }
        let inputs = values
            .iter()
            .map(|payload| pwright_node_host::file_input::MemoryFile {
                name: &payload.name,
                mime_type: Some(&payload.mime_type),
                buffer: payload.buffer.bind(py).as_bytes(),
            })
            .collect::<Vec<_>>();
        let input = chooser
            .action
            .prepare_payloads(&inputs)
            .map_err(binding_error)?;
        let action = Arc::clone(&chooser.action);
        self.async_command(
            py,
            FfiLane::Command,
            ACTION_ALLOWANCE,
            async move { action.set_files(input).await },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "file-chooser-events"))]
    fn set_file_chooser_payloads<'py>(
        &self,
        _py: Python<'py>,
        _chooser: &Bound<'py, PyAny>,
        _payloads: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page file chooser capability is not installed".into(),
        )))
    }

    #[cfg(feature = "file-chooser-events")]
    fn set_file_chooser_paths<'py>(
        &self,
        py: Python<'py>,
        chooser: PyRef<'_, NativeFileChooser>,
        base: &str,
        paths: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let base = bounded_string(
            base,
            pwright_node_host::file_input::profile::PATH_BYTES,
            "file-input base",
        )?;
        let paths = paths.cast::<PyList>().map_err(|_| {
            binding_error(HostError::InvalidArgument(
                "file-input paths must be a list".into(),
            ))
        })?;
        if paths.len() > pwright_node_host::file_input::profile::ITEMS {
            return Err(binding_error(HostError::InvalidArgument(
                "file-input count exceeds selected limit".into(),
            )));
        }
        let mut values = Vec::with_capacity(paths.len());
        for path in paths.iter() {
            let path = path.cast::<PyString>().map_err(|_| {
                binding_error(HostError::InvalidArgument(
                    "file-input path must be a string".into(),
                ))
            })?;
            values.push(bounded_string(
                &path.to_cow()?,
                pwright_node_host::file_input::profile::PATH_BYTES,
                "file-input path",
            )?);
        }
        let action = Arc::clone(&chooser.action);
        self.async_command(
            py,
            FfiLane::Command,
            ACTION_ALLOWANCE,
            async move {
                let paths = values.iter().map(String::as_str).collect::<Vec<_>>();
                action.set_paths(Path::new(&base), &paths).await
            },
            |py, ()| Ok(py.None()),
        )
    }

    #[cfg(not(feature = "file-chooser-events"))]
    fn set_file_chooser_paths<'py>(
        &self,
        _py: Python<'py>,
        _chooser: &Bound<'py, PyAny>,
        _base: &str,
        _paths: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(binding_error(HostError::UnsupportedOperation(
            "Page file chooser capability is not installed".into(),
        )))
    }

    fn close_context<'py>(&self, py: Python<'py>, context_id: &str) -> PyResult<Bound<'py, PyAny>> {
        let context_id = bounded_string(context_id, MAX_OBJECT_ID_BYTES, "Context ID")?;
        let host = Arc::clone(&self.host);
        #[cfg(feature = "context-strict-selectors")]
        let context_strict_selectors = Arc::clone(&self.context_strict_selectors);
        self.async_command(
            py,
            FfiLane::Close,
            SMALL_ALLOWANCE,
            async move {
                host.close_context(&context_id).await?;
                #[cfg(feature = "context-strict-selectors")]
                context_strict_selectors.retire(&context_id)?;
                Ok(())
            },
            |py, ()| Ok(py.None()),
        )
    }

    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let host = Arc::clone(&self.host);
        let owner = Arc::clone(&self.owner);
        let target = Arc::clone(&host);
        #[cfg(feature = "context-strict-selectors")]
        let context_strict_selectors = Arc::clone(&self.context_strict_selectors);
        self.async_command(
            py,
            FfiLane::Close,
            SMALL_ALLOWANCE,
            async move {
                host.close().await?;
                #[cfg(feature = "context-strict-selectors")]
                context_strict_selectors.clear()?;
                owner.remove(&host)
            },
            move |py, ()| {
                drop(target);
                Ok(py.None())
            },
        )
    }

    fn metrics(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Command, SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let ticket = execution.run_sync(move || host.metrics());
        ticket
            .deliver(|outcome| session_metrics_to_python(py, outcome.map_err(binding_error)?))
            .map_err(binding_error)?
    }
}

impl Drop for NativeSession {
    fn drop(&mut self) {
        self.host.request_shutdown();
    }
}

#[cfg(feature = "page-routes")]
fn page_route_call_to_python(
    py: Python<'_>,
    value: Option<pwright_node_host::RouteCall>,
) -> PyResult<Py<PyAny>> {
    let Some(value) = value else {
        return Ok(py.None());
    };
    let post_data = value
        .post_data
        .as_deref()
        .map(|bytes| PyBytes::new(py, bytes).into_any().unbind());
    #[cfg(feature = "network-events")]
    let native_request_id = value.native_request_id;
    #[cfg(not(feature = "network-events"))]
    let native_request_id: Option<String> = None;
    let fields = vec![
        value.id.to_string().into_pyobject(py)?.into_any().unbind(),
        value
            .registration_id
            .to_string()
            .into_pyobject(py)?
            .into_any()
            .unbind(),
        value
            .context
            .get()
            .to_string()
            .into_pyobject(py)?
            .into_any()
            .unbind(),
        value
            .page
            .get()
            .to_string()
            .into_pyobject(py)?
            .into_any()
            .unbind(),
        value
            .frame
            .map(|frame| frame.get().to_string())
            .into_pyobject(py)?
            .into_any()
            .unbind(),
        value.url.into_pyobject(py)?.into_any().unbind(),
        value.method.into_pyobject(py)?.into_any().unbind(),
        value.resource_type.into_pyobject(py)?.into_any().unbind(),
        value.headers.into_pyobject(py)?.into_any().unbind(),
        post_data.unwrap_or_else(|| py.None()),
        value
            .is_navigation_request
            .into_pyobject(py)?
            .to_owned()
            .into_any()
            .unbind(),
        value
            .expires
            .into_pyobject(py)?
            .to_owned()
            .into_any()
            .unbind(),
        native_request_id.into_pyobject(py)?.into_any().unbind(),
    ];
    Ok(PyTuple::new(py, fields)?.into_any().unbind())
}

#[cfg(feature = "response-security-details")]
fn security_details_to_python(
    py: Python<'_>,
    details: Option<&CdpResponseSecurityDetails>,
) -> PyResult<Py<PyAny>> {
    let Some(details) = details else {
        return Ok(py.None());
    };
    let value = PyDict::new(py);
    value.set_item("issuer", details.issuer())?;
    value.set_item("protocol", details.protocol())?;
    value.set_item("subjectName", details.subject_name())?;
    value.set_item("validFrom", details.valid_from_seconds())?;
    value.set_item("validTo", details.valid_to_seconds())?;
    Ok(value.into_any().unbind())
}

fn navigation_to_python(py: Python<'_>, value: NavigationResult) -> PyResult<Py<PyAny>> {
    let result = PyDict::new(py);
    result.set_item("url", value.url)?;
    if let Some(response) = value.response {
        let response_value = PyDict::new(py);
        response_value.set_item("request_id", response.request_id)?;
        response_value.set_item("url", response.url)?;
        response_value.set_item("status", response.status)?;
        response_value.set_item("status_text", response.status_text)?;
        response_value.set_item("ok", response.ok)?;
        response_value.set_item(
            "headers",
            response.headers.into_iter().collect::<BTreeMap<_, _>>(),
        )?;
        #[cfg(feature = "navigation-response-finished")]
        if let Some(owner) = response.finished_owner {
            response_value.set_item(
                "finished_owner",
                Py::new(py, NativeResponseFinishedOwner { owner })?,
            )?;
        }
        #[cfg(feature = "response-security-details")]
        if let Some(details) = response.security_details.as_ref() {
            response_value.set_item(
                "security_details",
                security_details_to_python(py, details.as_ref())?,
            )?;
        }
        result.set_item("response", response_value)?;
    } else {
        result.set_item("response", py.None())?;
    }
    Ok(result.into_any().unbind())
}

#[cfg(feature = "frame-navigation")]
fn frame_navigation_to_python(
    py: Python<'_>,
    value: pwright_node_host::FrameGotoOutput,
) -> PyResult<Py<PyAny>> {
    let result = value.value().map_err(binding_error)?;
    let Some(response) = result.response.as_ref() else {
        return Ok(py.None());
    };
    let response_value = PyDict::new(py);
    response_value.set_item("request_id", value.response_request_id())?;
    response_value.set_item("url", &response.url)?;
    response_value.set_item("status", response.status)?;
    response_value.set_item("status_text", &response.status_text)?;
    response_value.set_item("ok", (200..=299).contains(&response.status))?;
    response_value.set_item(
        "headers",
        response
            .headers
            .iter()
            .map(|header| (header.name.to_ascii_lowercase(), header.value.clone()))
            .collect::<BTreeMap<_, _>>(),
    )?;
    #[cfg(feature = "navigation-response-finished")]
    if let Some(owner) = value.response_finished_owner() {
        response_value.set_item(
            "finished_owner",
            Py::new(
                py,
                NativeResponseFinishedOwner {
                    owner: Arc::clone(owner),
                },
            )?,
        )?;
    }
    #[cfg(feature = "response-security-details")]
    if let Some(details) = value.response_security_details() {
        response_value.set_item("security_details", security_details_to_python(py, details)?)?;
    }
    Ok(response_value.into_any().unbind())
}

#[cfg(feature = "cdp-session")]
fn cdp_event_to_python(py: Python<'_>, event: CdpSessionEvent) -> PyResult<(String, Py<PyDict>)> {
    let params = event.params.as_object().ok_or_else(|| {
        binding_error(HostError::InvalidArgument(
            "CDP event parameters must be an object".into(),
        ))
    })?;
    // A transport event is byte-bounded by the host, but Python objects can
    // amplify tiny JSON nodes. Validate one event before allocating its tree.
    let mut pending = vec![(&event.params, 0_usize)];
    let mut nodes = 0_usize;
    while let Some((value, depth)) = pending.pop() {
        nodes += 1;
        if nodes > MAX_CDP_EVENT_PYTHON_NODES || depth > MAX_CDP_EVENT_PYTHON_DEPTH {
            return Err(binding_error(HostError::ResourceExhausted(
                "CDP event parameters exceed Python conversion limits".into(),
            )));
        }
        match value {
            serde_json::Value::Array(values) => {
                pending.extend(values.iter().map(|value| (value, depth + 1)));
            }
            serde_json::Value::Object(values) => {
                pending.extend(values.values().map(|value| (value, depth + 1)));
            }
            _ => {}
        }
    }
    let output = PyDict::new(py);
    for (name, value) in params {
        output.set_item(name, json_value_to_python(py, value)?)?;
    }
    Ok((event.method, output.unbind()))
}

#[cfg(any(
    feature = "request-post-data-json",
    feature = "context-storage-state",
    feature = "cdp-session"
))]
fn json_value_to_python(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(value) => {
            Ok(value.into_pyobject(py)?.to_owned().into_any().unbind())
        }
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(value.into_pyobject(py)?.into_any().unbind())
            } else if let Some(value) = value.as_u64() {
                Ok(value.into_pyobject(py)?.into_any().unbind())
            } else {
                Ok(value
                    .as_f64()
                    .expect("serde_json number is representable")
                    .into_pyobject(py)?
                    .to_owned()
                    .into_any()
                    .unbind())
            }
        }
        serde_json::Value::String(value) => Ok(value.into_pyobject(py)?.into_any().unbind()),
        serde_json::Value::Array(values) => {
            let output = PyList::empty(py);
            for value in values {
                output.append(json_value_to_python(py, value)?)?;
            }
            Ok(output.into_any().unbind())
        }
        serde_json::Value::Object(values) => {
            let output = PyDict::new(py);
            for (name, value) in values {
                output.set_item(name, json_value_to_python(py, value)?)?;
            }
            Ok(output.into_any().unbind())
        }
    }
}

#[cfg(feature = "network-events")]
fn request_record_to_python<'py>(
    py: Python<'py>,
    value: &NetworkRequestRecord,
) -> PyResult<Bound<'py, PyDict>> {
    let result = PyDict::new(py);
    result.set_item("id", &value.id)?;
    result.set_item("native_request_id", value.native_request_id.as_deref())?;
    #[cfg(feature = "request-frame")]
    result.set_item("page_id", &value.page_id)?;
    #[cfg(feature = "request-frame")]
    result.set_item("frame_id", value.frame_id.as_deref())?;
    #[cfg(feature = "request-frame")]
    result.set_item("frame_available", value.frame_available)?;
    result.set_item("url", &value.url)?;
    result.set_item("method", &value.method)?;
    result.set_item("resource_type", value.resource_type.to_ascii_lowercase())?;
    result.set_item(
        "headers",
        value.headers.iter().cloned().collect::<BTreeMap<_, _>>(),
    )?;
    if let Some(post_data) = &value.post_data {
        result.set_item("post_data", String::from_utf8_lossy(post_data).as_ref())?;
        result.set_item("post_data_buffer", PyBytes::new(py, post_data))?;
    } else {
        result.set_item("post_data", py.None())?;
        result.set_item("post_data_buffer", py.None())?;
    }
    result.set_item("is_navigation_request", value.is_navigation_request)?;
    result.set_item("redirected_from_id", value.redirected_from.as_deref())?;
    #[cfg(feature = "request-timing")]
    result.set_item("timing", request_timing_to_python(py, &value.timing)?)?;
    #[cfg(feature = "request-post-data-json")]
    match pwright_node_host::network_events::request_post_data_json(value) {
        Ok(Some(parsed)) => {
            result.set_item("post_data_json", json_value_to_python(py, &parsed)?)?;
        }
        Ok(None) => {
            result.set_item("post_data_json", py.None())?;
        }
        Err(message) => {
            result.set_item("post_data_json", py.None())?;
            result.set_item("post_data_json_error", message)?;
        }
    }
    if let Some(owner) = &value.size_owner {
        result.set_item(
            "sizes_owner",
            Py::new(
                py,
                NativeRequestSizeOwner {
                    owner: Arc::clone(owner),
                },
            )?,
        )?;
    } else {
        result.set_item("sizes_owner", py.None())?;
    }
    Ok(result)
}

#[cfg(feature = "network-events")]
fn request_sizes_to_python(py: Python<'_>, value: RequestSizes) -> PyResult<Py<PyAny>> {
    let result = PyDict::new(py);
    result.set_item("requestBodySize", value.request_body_size)?;
    result.set_item("requestHeadersSize", value.request_headers_size)?;
    result.set_item("responseBodySize", value.response_body_size)?;
    result.set_item("responseHeadersSize", value.response_headers_size)?;
    result.set_item("transferSize", value.transfer_size)?;
    Ok(result.into_any().unbind())
}

#[cfg(feature = "request-timing")]
fn request_timing_to_python<'py>(
    py: Python<'py>,
    value: &RequestTiming,
) -> PyResult<Bound<'py, PyDict>> {
    let result = PyDict::new(py);
    result.set_item("startTime", value.start_time_micros as f64 / 1_000.0)?;
    result.set_item(
        "domainLookupStart",
        value.domain_lookup_start_micros as f64 / 1_000.0,
    )?;
    result.set_item(
        "domainLookupEnd",
        value.domain_lookup_end_micros as f64 / 1_000.0,
    )?;
    result.set_item("connectStart", value.connect_start_micros as f64 / 1_000.0)?;
    result.set_item(
        "secureConnectionStart",
        value.secure_connection_start_micros as f64 / 1_000.0,
    )?;
    result.set_item("connectEnd", value.connect_end_micros as f64 / 1_000.0)?;
    result.set_item("requestStart", value.request_start_micros as f64 / 1_000.0)?;
    result.set_item(
        "responseStart",
        value.response_start_micros as f64 / 1_000.0,
    )?;
    result.set_item("responseEnd", value.response_end_micros as f64 / 1_000.0)?;
    Ok(result)
}

#[cfg(feature = "network-events")]
fn response_record_to_python<'py>(
    py: Python<'py>,
    value: &NetworkResponseRecord,
) -> PyResult<Bound<'py, PyDict>> {
    let result = PyDict::new(py);
    result.set_item("id", &value.id)?;
    result.set_item("request_id", &value.request_id)?;
    result.set_item("url", &value.url)?;
    result.set_item("status", value.status)?;
    result.set_item("status_text", &value.status_text)?;
    result.set_item("ok", (200..=299).contains(&value.status))?;
    result.set_item(
        "headers",
        value.headers.iter().cloned().collect::<BTreeMap<_, _>>(),
    )?;
    result.set_item("from_service_worker", value.from_service_worker)?;
    #[cfg(feature = "response-security-details")]
    result.set_item(
        "security_details",
        security_details_to_python(py, value.security_details.as_ref())?,
    )?;
    #[cfg(feature = "request-timing")]
    result.set_item("timing", request_timing_to_python(py, &value.timing)?)?;
    if let Some(body) = &value.body {
        result.set_item(
            "body_owner",
            Py::new(
                py,
                NativeResponseBody {
                    body: Arc::clone(body),
                },
            )?,
        )?;
    } else {
        result.set_item("body_owner", py.None())?;
    }
    #[cfg(feature = "response-finished")]
    if let Some(owner) = &value.finished_owner {
        result.set_item(
            "finished_owner",
            Py::new(
                py,
                NativeResponseFinishedOwner {
                    owner: Arc::clone(owner),
                },
            )?,
        )?;
    } else {
        result.set_item("finished_owner", py.None())?;
    }
    #[cfg(not(feature = "response-finished"))]
    result.set_item("finished_owner", py.None())?;
    Ok(result)
}

#[cfg(feature = "response-events")]
fn response_notice_to_python(py: Python<'_>, value: ResponseNotice) -> PyResult<Py<PyAny>> {
    let response = response_record_to_python(py, &value.response)?;
    #[cfg(feature = "request-history")]
    if let Some(owner) = value.history_owner {
        response.set_item("request", request_history_to_python(py, owner)?)?;
    }
    Ok((value.page.get().to_string(), value.sequence, response)
        .into_pyobject(py)?
        .into_any()
        .unbind())
}

#[cfg(feature = "request-events")]
fn request_notice_to_python(py: Python<'_>, value: RequestNotice) -> PyResult<Py<PyAny>> {
    let request = request_record_to_python(py, &value.request)?;
    request.set_item("kind", value.kind.as_str())?;
    request.set_item("failure_text", value.failure_text)?;
    #[cfg(feature = "request-history")]
    if let Some(owner) = value.history_owner {
        request.set_item(
            "history_owner",
            Py::new(py, NativeRequestHistoryOwner { owner })?,
        )?;
    }
    Ok((value.page.get().to_string(), value.sequence, request)
        .into_pyobject(py)?
        .into_any()
        .unbind())
}

#[cfg(feature = "download-events")]
fn download_notice_to_python(py: Python<'_>, value: DownloadNotice) -> PyResult<Py<PyAny>> {
    let owner = Py::new(
        py,
        NativeDownload {
            owner: Arc::clone(&value.owner),
        },
    )?;
    Ok((
        value.page.get().to_string(),
        value.owner.url().to_owned(),
        value.owner.suggested_filename().to_owned(),
        owner,
    )
        .into_pyobject(py)?
        .into_any()
        .unbind())
}

#[cfg(feature = "worker-events")]
fn worker_owner_to_python(py: Python<'_>, owner: Arc<WorkerOwner>) -> PyResult<Py<PyAny>> {
    let id = owner.id().to_owned();
    let url = owner.url().to_owned();
    let native = Py::new(py, NativeWorker { owner })?;
    Ok((id, url, native).into_pyobject(py)?.into_any().unbind())
}

#[cfg(feature = "worker-events")]
fn worker_notice_to_python(py: Python<'_>, value: WorkerNotice) -> PyResult<Py<PyAny>> {
    let kind = match value.kind {
        WorkerNoticeKind::Created => 0,
        WorkerNoticeKind::Closed => 1,
    };
    let page = value.page.get().to_string();
    let id = value.owner.id().to_owned();
    let url = value.owner.url().to_owned();
    let native = Py::new(py, NativeWorker { owner: value.owner })?;
    Ok((kind, page, id, url, native)
        .into_pyobject(py)?
        .into_any()
        .unbind())
}

#[cfg(feature = "worker-console-events")]
fn worker_console_notice_to_python(
    py: Python<'_>,
    value: WorkerConsoleNotice,
) -> PyResult<Py<PyAny>> {
    let page = value.page.get().to_string();
    let id = value.owner.id().to_owned();
    let url = value.owner.url().to_owned();
    let native = Py::new(py, NativeWorker { owner: value.owner })?;
    let message = value.message;
    #[cfg(feature = "worker-console-argument-handles")]
    let argument_handles = value
        .argument_handles
        .into_iter()
        .map(|handle| Py::new(py, handles::NativeHandle::from_worker(handle)))
        .collect::<PyResult<Vec<_>>>()?;
    Ok((
        page,
        id,
        url,
        native,
        message.message_type().to_owned(),
        message.text().to_owned(),
        message.arguments_json().to_owned(),
        message.location_url().to_owned(),
        message.line_number(),
        message.column_number(),
        message.timestamp_micros(),
        #[cfg(feature = "worker-console-argument-handles")]
        argument_handles,
    )
        .into_pyobject(py)?
        .into_any()
        .unbind())
}

#[cfg(feature = "request-history")]
fn request_history_to_python(
    py: Python<'_>,
    owner: Arc<RequestHistoryOwner>,
) -> PyResult<Py<PyAny>> {
    let result = request_record_to_python(py, owner.request())?;
    result.set_item(
        "history_owner",
        Py::new(py, NativeRequestHistoryOwner { owner })?,
    )?;
    Ok(result.into_any().unbind())
}

#[cfg(feature = "network-events")]
fn network_event_to_python(py: Python<'_>, value: NetworkWaitRecord) -> PyResult<Py<PyAny>> {
    #[cfg(feature = "request-history")]
    let (value, history_owner) = value.into_parts();
    #[cfg(not(feature = "request-history"))]
    let value = value.into_event();
    let result = match value {
        NetworkEventRecord::Request(request) => {
            let result = request_record_to_python(py, &request)?;
            #[cfg(feature = "request-history")]
            if let Some(owner) = history_owner {
                result.set_item(
                    "history_owner",
                    Py::new(py, NativeRequestHistoryOwner { owner })?,
                )?;
            }
            result.set_item("kind", "request")?;
            result
        }
        NetworkEventRecord::Response(response) => {
            let result = response_record_to_python(py, &response)?;
            #[cfg(feature = "request-history")]
            if let Some(owner) = history_owner {
                result.set_item("request", request_history_to_python(py, owner)?)?;
            }
            result.set_item("kind", "response")?;
            result
        }
        NetworkEventRecord::RequestFinished {
            request,
            response,
            encoded_body_bytes,
        } => {
            let result = request_record_to_python(py, &request)?;
            #[cfg(feature = "request-history")]
            if let Some(owner) = history_owner {
                result.set_item(
                    "history_owner",
                    Py::new(py, NativeRequestHistoryOwner { owner })?,
                )?;
            }
            result.set_item("kind", "requestfinished")?;
            if let Some(response) = response {
                result.set_item("response", response_record_to_python(py, &response)?)?;
            } else {
                result.set_item("response", py.None())?;
            }
            result.set_item("encoded_body_bytes", encoded_body_bytes)?;
            result
        }
        NetworkEventRecord::RequestFailed {
            request,
            response,
            failure_text,
            canceled,
        } => {
            let result = request_record_to_python(py, &request)?;
            #[cfg(feature = "request-history")]
            if let Some(owner) = history_owner {
                result.set_item(
                    "history_owner",
                    Py::new(py, NativeRequestHistoryOwner { owner })?,
                )?;
            }
            result.set_item("kind", "requestfailed")?;
            if let Some(response) = response {
                result.set_item("response", response_record_to_python(py, &response)?)?;
            } else {
                result.set_item("response", py.None())?;
            }
            result.set_item("failure_text", failure_text)?;
            result.set_item("canceled", canceled)?;
            result
        }
    };
    Ok(result.into_any().unbind())
}

fn locator_value_to_python(py: Python<'_>, value: LocatorValue) -> PyResult<Py<PyAny>> {
    match value {
        LocatorValue::Count(value) => Ok(value.into_pyobject(py)?.into_any().unbind()),
        LocatorValue::Bool(value) => Ok(value.into_pyobject(py)?.to_owned().into_any().unbind()),
        LocatorValue::Text(Some(value)) => Ok(value.into_pyobject(py)?.into_any().unbind()),
        LocatorValue::Text(None) => Ok(py.None()),
        LocatorValue::Texts(value) => Ok(PyList::new(py, value)?.into_any().unbind()),
    }
}

fn session_metrics_to_python(py: Python<'_>, metrics: SessionMetrics) -> PyResult<Py<PyAny>> {
    let result = PyDict::new(py);
    result.set_item("connected", metrics.connected)?;
    result.set_item("live_objects", metrics.live_objects)?;
    result.set_item("pending_calls", metrics.pending_calls)?;
    result.set_item("topology_drivers", metrics.topology_drivers)?;
    result.set_item("active_leases", metrics.active_leases)?;
    result.set_item("live_scopes", metrics.live_scopes)?;
    result.set_item("retained_event_bytes", metrics.retained_event_bytes)?;
    result.set_item("active_operations", metrics.active_operations)?;
    result.set_item("cleanup_in_flight", metrics.cleanup_in_flight)?;
    Ok(result.into_any().unbind())
}

fn add_error<T: PyTypeInfo>(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    name: &str,
    code: &str,
) -> PyResult<()> {
    let error_type = py.get_type::<T>();
    error_type.setattr("code", code)?;
    module.add(name, error_type)
}

#[pymodule]
fn _native(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(native_abi_version, module)?)?;
    module.add_function(wrap_pyfunction!(package_version, module)?)?;
    module.add_function(wrap_pyfunction!(device_descriptors_json, module)?)?;
    module.add_function(wrap_pyfunction!(
        browser_registry::browser_executable_path,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(selected_features, module)?)?;
    module.add_function(wrap_pyfunction!(build_profile, module)?)?;
    module.add_function(wrap_pyfunction!(resource_limits, module)?)?;
    module.add_class::<NativeEnvironment>()?;
    module.add_class::<NativeSession>()?;
    #[cfg(feature = "playwright-server-connect-session")]
    module.add_class::<NativeRemoteBrowser>()?;
    #[cfg(feature = "playwright-server-connect-session")]
    module.add_class::<NativeRemoteContext>()?;
    #[cfg(feature = "playwright-server-connect-session")]
    module.add_class::<NativeRemotePage>()?;
    #[cfg(feature = "cdp-session")]
    module.add_class::<NativeCdpSessionLease>()?;
    #[cfg(feature = "cdp-session")]
    module.add_class::<NativeCdpEventBatch>()?;
    module.add_class::<NativeLocator>()?;
    #[cfg(feature = "handles")]
    module.add_class::<NativeFrameLocator>()?;
    module.add_class::<NativeDialog>()?;
    module.add_class::<NativeFileChooser>()?;
    #[cfg(feature = "network-events")]
    module.add_class::<NativeRequestSizeOwner>()?;
    #[cfg(feature = "network-events")]
    module.add_class::<NativeResponseBody>()?;
    #[cfg(feature = "response-finished")]
    module.add_class::<NativeResponseFinishedOwner>()?;
    #[cfg(feature = "request-history")]
    module.add_class::<NativeRequestHistoryOwner>()?;
    #[cfg(feature = "download-events")]
    module.add_class::<NativeDownload>()?;
    #[cfg(feature = "worker-events")]
    module.add_class::<NativeWorker>()?;
    #[cfg(feature = "handles")]
    module.add_class::<handles::NativeHandle>()?;
    #[cfg(feature = "api-request")]
    {
        module.add_class::<api_request::NativeApiRequestContext>()?;
        module.add_class::<api_request::NativeApiResponse>()?;
    }
    add_error::<Error>(py, module, "Error", "PWRIGHT_BACKEND")?;
    add_error::<TimeoutError>(py, module, "TimeoutError", "PWRIGHT_TIMEOUT")?;
    add_error::<TargetClosedError>(py, module, "TargetClosedError", "PWRIGHT_TARGET_CLOSED")?;
    add_error::<ResourceExhaustedError>(
        py,
        module,
        "ResourceExhaustedError",
        "PWRIGHT_RESOURCE_EXHAUSTED",
    )?;
    add_error::<JavascriptError>(py, module, "JavascriptError", "PWRIGHT_JAVASCRIPT")?;
    add_error::<InvalidArgumentError>(
        py,
        module,
        "InvalidArgumentError",
        "PWRIGHT_INVALID_ARGUMENT",
    )?;
    add_error::<UnsupportedError>(py, module, "UnsupportedError", "PWRIGHT_UNSUPPORTED")?;
    Ok(())
}

#[cfg(test)]
mod binding_tests {
    use super::*;

    #[test]
    fn native_abi_is_131() {
        // Act
        let version = native_abi_version();

        // Assert
        assert_eq!(version, 131);
    }

    #[cfg(feature = "playwright-server-initialize")]
    #[test]
    fn refused_playwright_server_connection_is_a_websocket_error() {
        // Arrange
        let failure = pwright_playwright_server_initialize::InitializeError::WebSocket(
            pwright_playwright_server_initialize::WsError::ConnectFailed,
        );

        // Act
        let error = playwright_server_initialize_host_error(failure);

        // Assert
        assert_eq!(
            error.to_string(),
            "WebSocket error: WebSocket connection failed"
        );
    }

    #[cfg(feature = "cdp-handshake-headers")]
    #[test]
    fn native_cdp_handshake_header_conversion_preserves_pairs() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let values = PyList::new(py, [("foo", "bar")]).expect("header list");

            // Act
            let headers = bounded_cdp_handshake_headers(&values).expect("bounded headers");

            // Assert
            assert_eq!(headers, vec![("foo".into(), "bar".into())]);
        });
    }

    #[cfg(feature = "cdp-handshake-headers")]
    #[test]
    fn native_cdp_handshake_header_count_rejects_before_pair_copy() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let values = PyList::new(py, (0..33).map(|_| ("foo", "bar"))).expect("header list");

            // Act
            let error = bounded_cdp_handshake_headers(&values).expect_err("header limit");

            // Assert
            assert!(error.is_instance_of::<ResourceExhaustedError>(py));
        });
    }

    #[cfg(feature = "cdp-handshake-headers")]
    #[test]
    fn native_cdp_handshake_header_value_rejects_before_string_copy() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let oversized = "x".repeat(CDP_HANDSHAKE_HEADER_MAX_VALUE_BYTES + 1);
            let values = PyList::new(py, [("foo", oversized)]).expect("header list");

            // Act
            let error = bounded_cdp_handshake_headers(&values).expect_err("value limit");

            // Assert
            assert!(error.is_instance_of::<ResourceExhaustedError>(py));
        });
    }

    #[cfg(feature = "cdp-session")]
    #[test]
    fn cdp_event_batch_class_exposes_claim_methods() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let module = PyModule::new(py, "native_cdp_batch_probe").expect("module");

            // Act
            module
                .add_class::<NativeCdpEventBatch>()
                .expect("register event batch");
            let batch_type = module.getattr("NativeCdpEventBatch").expect("batch class");

            // Assert
            for name in ["peek", "ack_next", "rollback"] {
                assert!(batch_type.hasattr(name).expect("method lookup"));
            }
        });
    }

    #[cfg(feature = "cdp-session")]
    #[test]
    fn native_session_exposes_cdp_generation_watch() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let module = PyModule::new(py, "native_cdp_watch_probe").expect("module");

            // Act
            module
                .add_class::<NativeSession>()
                .expect("register session");

            // Assert
            let session_type = module.getattr("NativeSession").expect("session class");
            assert!(
                session_type
                    .hasattr("next_cdp_session_event_generation")
                    .expect("watch method")
            );
        });
    }

    #[cfg(feature = "cdp-session")]
    #[test]
    fn cdp_event_conversion_preserves_nested_dictionary() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let event = CdpSessionEvent {
                method: "Network.requestWillBeSent".into(),
                params: serde_json::json!({"requestId": "r1", "request": {"url": "https://example.test"}}),
            };

            // Act
            let (method, params) = cdp_event_to_python(py, event).expect("convert event");

            // Assert
            assert_eq!(method, "Network.requestWillBeSent");
            assert_eq!(
                params
                    .bind(py)
                    .get_item("requestId")
                    .expect("lookup")
                    .expect("request id")
                    .extract::<String>()
                    .expect("string"),
                "r1"
            );
            let request = params
                .bind(py)
                .get_item("request")
                .expect("lookup")
                .expect("request");
            let url = request.get_item("url").expect("url");
            assert_eq!(
                url.extract::<String>().expect("string"),
                "https://example.test"
            );
        });
    }

    #[cfg(feature = "cdp-session")]
    #[test]
    fn cdp_event_conversion_rejects_excessive_depth_before_python_allocation() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let mut nested = serde_json::json!(null);
            for _ in 0..=MAX_CDP_EVENT_PYTHON_DEPTH {
                nested = serde_json::json!([nested]);
            }
            let event = CdpSessionEvent {
                method: "Runtime.consoleAPICalled".into(),
                params: serde_json::json!({"args": nested}),
            };

            // Act
            let error = cdp_event_to_python(py, event).expect_err("depth limit");

            // Assert
            assert!(error.is_instance_of::<ResourceExhaustedError>(py));
        });
    }

    #[cfg(feature = "cdp-session")]
    #[test]
    fn unread_cdp_delivery_lease_runs_cleanup_when_result_is_dropped() {
        // Arrange: the FFI future has produced a result, but Python has not
        // yet resumed and claimed its native owner.
        let cleaned = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = Arc::clone(&cleaned);
        let queued: Result<NativeCdpSessionLease, HostError> =
            Ok(NativeCdpSessionLease::new(7, move || {
                observed.fetch_add(1, Ordering::SeqCst);
            }));

        // Act
        drop(queued);

        // Assert
        assert_eq!(cleaned.load(Ordering::SeqCst), 1);
    }

    #[cfg(feature = "cdp-session")]
    #[test]
    fn unclaimed_python_cdp_delivery_object_runs_cleanup_when_dropped() {
        // Arrange
        Python::initialize();
        let cleaned = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = Arc::clone(&cleaned);

        // Act
        Python::attach(|py| {
            let lease = Py::new(
                py,
                NativeCdpSessionLease::new(7, move || {
                    observed.fetch_add(1, Ordering::SeqCst);
                }),
            )
            .expect("Python lease");
            drop(lease);
        });

        // Assert
        assert_eq!(cleaned.load(Ordering::SeqCst), 1);
    }

    #[cfg(feature = "cdp-session")]
    #[test]
    fn claimed_cdp_delivery_lease_does_not_detach_installed_wrapper() {
        // Arrange
        let cleaned = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = Arc::clone(&cleaned);
        let mut lease = NativeCdpSessionLease::new(7, move || {
            observed.fetch_add(1, Ordering::SeqCst);
        });

        // Act
        let token = lease.claim().expect("first claim");
        drop(lease);

        // Assert
        assert_eq!(token, 7);
        assert_eq!(cleaned.load(Ordering::SeqCst), 0);
    }

    #[cfg(feature = "touchscreen-input")]
    #[test]
    fn tap_options_preserve_modifiers_and_pointer_settings() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let values = PyDict::new(py);
            values.set_item("force", true).expect("force");
            values.set_item("no_wait_after", false).expect("wait");
            values.set_item("trial", true).expect("trial");
            values
                .set_item("modifiers", vec!["Alt"])
                .expect("modifiers");
            let position = PyDict::new(py);
            position.set_item("x", 4.5).expect("x");
            position.set_item("y", 7.25).expect("y");
            values.set_item("position", position).expect("position");

            // Act
            let options = python_tap_options(&values).expect("valid tap options");

            // Assert
            assert_eq!(options.force, Some(true));
            assert_eq!(options.no_wait_after, Some(false));
            assert_eq!(options.trial, Some(true));
            assert_eq!(
                options.position,
                Some(Position::new(4.5, 7.25).expect("point"))
            );
            assert_eq!(
                options.modifiers,
                Some(KeyboardModifiers::new().with(KeyboardModifier::Alt))
            );
        });
    }

    #[cfg(feature = "touchscreen-input")]
    #[test]
    fn tap_options_reject_nonfinite_position() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let values = PyDict::new(py);
            let position = PyDict::new(py);
            position.set_item("x", f64::NAN).expect("x");
            position.set_item("y", 1.0).expect("y");
            values.set_item("position", position).expect("position");

            // Act
            let error = python_tap_options(&values).expect_err("NaN is invalid");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[test]
    fn locator_click_omitted_steps_preserves_native_default() {
        // Arrange
        let steps = None;

        // Act
        let options = locator_click_options(steps, None).expect("omitted steps are valid");

        // Assert
        assert_eq!(options.steps, None);
        assert_eq!(options.force, None);
    }

    #[test]
    fn locator_click_preserves_explicit_force_true_with_steps() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let steps = 5i64.into_pyobject(py).expect("Python integer");
            let force = true.into_pyobject(py).expect("Python bool");

            // Act
            let options = locator_click_options(Some(steps.as_any()), Some(force.as_any()))
                .expect("force and steps are valid");

            // Assert
            assert_eq!(options.steps.map(NonZeroU32::get), Some(5));
            assert_eq!(options.force, Some(true));
        });
    }

    #[test]
    fn locator_click_preserves_explicit_force_false() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let force = false.into_pyobject(py).expect("Python bool");

            // Act
            let options =
                locator_click_options(None, Some(force.as_any())).expect("force is valid");

            // Assert
            assert_eq!(options.steps, None);
            assert_eq!(options.force, Some(false));
        });
    }

    #[test]
    fn locator_click_rejects_non_boolean_force() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let force = "yes".into_pyobject(py).expect("Python string");

            // Act
            let error = locator_click_options(None, Some(force.as_any()))
                .expect_err("non-boolean force is invalid");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[test]
    fn locator_click_accepts_minimum_steps() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let steps = 1i64.into_pyobject(py).expect("Python integer");

            // Act
            let options = locator_click_options(Some(steps.as_any()), None).expect("valid steps");

            // Assert
            assert_eq!(options.steps.map(NonZeroU32::get), Some(1));
        });
    }

    #[test]
    fn locator_click_accepts_maximum_steps() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let steps = 10_000i64.into_pyobject(py).expect("Python integer");

            // Act
            let options = locator_click_options(Some(steps.as_any()), None).expect("valid steps");

            // Assert
            assert_eq!(options.steps.map(NonZeroU32::get), Some(10_000));
        });
    }

    #[test]
    fn locator_click_rejects_zero_steps() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let steps = 0i64.into_pyobject(py).expect("Python integer");

            // Act
            let error =
                locator_click_options(Some(steps.as_any()), None).expect_err("zero is invalid");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[test]
    fn locator_click_rejects_negative_steps() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let steps = (-1i64).into_pyobject(py).expect("Python integer");

            // Act
            let error = locator_click_options(Some(steps.as_any()), None)
                .expect_err("negative steps are invalid");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[test]
    fn locator_click_rejects_steps_above_maximum() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let steps = 10_001i64.into_pyobject(py).expect("Python integer");

            // Act
            let error = locator_click_options(Some(steps.as_any()), None)
                .expect_err("excessive steps are invalid");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[test]
    fn locator_click_rejects_bool_steps() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let steps = true.into_pyobject(py).expect("Python bool");

            // Act
            let error = locator_click_options(Some(steps.as_any()), None)
                .expect_err("bool is not an integer step count");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[test]
    fn locator_click_rejects_fractional_steps() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let steps = 1.5f64.into_pyobject(py).expect("Python float");

            // Act
            let error = locator_click_options(Some(steps.as_any()), None)
                .expect_err("fraction is not an integer step count");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[test]
    fn locator_click_rejects_string_steps() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let steps = "4".into_pyobject(py).expect("Python string");

            // Act
            let error = locator_click_options(Some(steps.as_any()), None)
                .expect_err("string is not an integer step count");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[test]
    fn page_click_options_preserve_bounded_pointer_values() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let position = PyDict::new(py);
            position.set_item("x", 2.5).expect("x");
            position.set_item("y", 3.25).expect("y");
            let values = PyDict::new(py);
            values.set_item("button", "right").expect("button");
            values.set_item("click_count", 2).expect("count");
            values.set_item("force", false).expect("force");
            values.set_item("position", position).expect("position");

            // Act
            let options = page_click_options(&values).expect("valid click options");

            // Assert
            assert_eq!(options.button, Some(MouseButton::Right));
            assert_eq!(options.click_count, NonZeroU8::new(2));
            assert_eq!(options.force, Some(false));
            assert_eq!(
                options.position,
                Some(Position::new(2.5, 3.25).expect("finite position"))
            );
            assert_eq!(options.modifiers, None);
        });
    }

    #[test]
    fn page_click_options_reject_boolean_click_count() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let values = PyDict::new(py);
            values.set_item("click_count", true).expect("count");

            // Act
            let error = page_click_options(&values).expect_err("bool is not a click count");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[test]
    fn page_click_options_reject_click_count_above_wire_limit() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let values = PyDict::new(py);
            values.set_item("click_count", 256).expect("count");

            // Act
            let error = page_click_options(&values).expect_err("count exceeds u8");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[test]
    fn page_click_options_reject_nonfinite_position() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let position = PyDict::new(py);
            position.set_item("x", f64::NAN).expect("x");
            position.set_item("y", 1.0).expect("y");
            let values = PyDict::new(py);
            values.set_item("position", position).expect("position");

            // Act
            let error = page_click_options(&values).expect_err("nonfinite x is invalid");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[test]
    fn page_click_options_reject_unknown_field() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let values = PyDict::new(py);
            values.set_item("unexpected", 1).expect("unknown field");

            // Act
            let error = page_click_options(&values).expect_err("unknown field is invalid");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[cfg(feature = "element-handle-click")]
    #[test]
    fn element_click_options_preserve_pointer_and_wait_settings() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let values = PyDict::new(py);
            values.set_item("button", "right").expect("button");
            values.set_item("delay", 1.25).expect("delay");
            values.set_item("no_wait_after", true).expect("wait");
            values.set_item("trial", false).expect("trial");
            values.set_item("steps", 3).expect("steps");

            // Act
            let options = element_click_options(&values).expect("valid retained click options");

            // Assert
            assert_eq!(options.button, Some(MouseButton::Right));
            assert_eq!(options.delay, Some(std::time::Duration::from_micros(1250)));
            assert_eq!(options.no_wait_after, Some(true));
            assert_eq!(options.trial, Some(false));
            assert_eq!(options.steps.map(NonZeroU32::get), Some(3));
        });
    }

    #[cfg(feature = "element-handle-click")]
    #[test]
    fn element_click_options_reject_boolean_delay() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let values = PyDict::new(py);
            values.set_item("delay", true).expect("delay");

            // Act
            let error = element_click_options(&values).expect_err("boolean delay is invalid");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[cfg(feature = "element-handle-click")]
    #[test]
    fn element_click_options_reject_unknown_option() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let values = PyDict::new(py);
            values.set_item("selector", "button").expect("selector");

            // Act
            let error =
                element_click_options(&values).expect_err("selector reacquisition is invalid");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }

    #[cfg(feature = "locator-click-modifiers")]
    #[test]
    fn page_click_options_compose_with_modifier_set() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let values = PyDict::new(py);
            values.set_item("force", true).expect("force");
            values
                .set_item(
                    "modifiers",
                    PyList::new(py, ["Shift"]).expect("modifier list"),
                )
                .expect("modifiers");

            // Act
            let options = page_click_options(&values).expect("combined options");

            // Assert
            assert_eq!(options.force, Some(true));
            assert!(
                options
                    .modifiers
                    .expect("modifier override")
                    .contains(KeyboardModifier::Shift)
            );
        });
    }

    #[cfg(feature = "locator-click-modifiers")]
    #[test]
    fn click_modifier_parser_preserves_shift_and_control_or_meta() {
        // Arrange
        let names = ["Shift", "ControlOrMeta"];

        // Act
        let parsed = names.map(click_modifier).map(Result::unwrap);

        // Assert
        assert_eq!(
            parsed,
            [KeyboardModifier::Shift, KeyboardModifier::ControlOrMeta]
        );
    }

    #[cfg(feature = "locator-click-modifiers")]
    #[test]
    fn click_modifier_parser_rejects_unknown_name() {
        // Arrange
        let name = "Super";

        // Act
        let error = click_modifier(name).expect_err("unsupported modifier must fail");

        // Assert
        assert!(matches!(error, HostError::InvalidArgument(_)));
    }

    #[cfg(feature = "locator-click-modifiers")]
    #[test]
    fn click_modifier_boundary_preserves_explicit_empty_set() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let names = PyList::empty(py);

            // Act
            let modifiers = click_modifiers(&names).expect("empty list is valid");

            // Assert
            assert_eq!(modifiers, KeyboardModifiers::new());
        });
    }

    #[cfg(feature = "locator-click-modifiers")]
    #[test]
    fn click_modifier_boundary_accepts_control_or_meta_list() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let names = PyList::new(py, ["ControlOrMeta"]).expect("valid Python list");

            // Act
            let modifiers = click_modifiers(&names).expect("known modifier");

            // Assert
            assert!(modifiers.contains(KeyboardModifier::ControlOrMeta));
            assert_eq!(modifiers.len(), 1);
        });
    }

    #[cfg(feature = "locator-click-modifiers")]
    #[test]
    fn click_modifier_boundary_rejects_more_than_five_names() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let names = PyList::new(py, ["Shift"; 6]).expect("valid Python list");

            // Act
            let error = click_modifiers(&names).expect_err("list must be bounded");

            // Assert
            assert!(error.to_string().contains("at most five"));
        });
    }

    #[cfg(feature = "locator-click-modifiers")]
    #[test]
    fn click_modifier_boundary_rejects_non_string_item() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let names = PyList::new(py, [7]).expect("valid Python list");

            // Act
            let error = click_modifiers(&names).expect_err("modifier must be text");

            // Assert
            assert!(error.to_string().contains("must be strings"));
        });
    }

    #[cfg(feature = "context-storage-state")]
    #[test]
    fn storage_state_json_preserves_playwright_origin_shape() {
        // Arrange
        let payload = r#"{"origins":[{"origin":"https://example.com","localStorage":[{"name":"key","value":"value"}],"indexedDB":[{"name":"db"}]}]}"#;

        // Act
        let state = parse_context_storage_state(payload).expect("valid storage state");

        // Assert
        assert!(state.cookies.is_empty());
        assert_eq!(state.origins.len(), 1);
        assert_eq!(state.origins[0].origin, "https://example.com");
        assert_eq!(state.origins[0].local_storage[0].name, "key");
        assert_eq!(state.origins[0].local_storage[0].value, "value");
        assert_eq!(state.origins[0].indexed_db.as_ref().map(Vec::len), Some(1));
    }

    #[cfg(feature = "context-storage-state")]
    #[test]
    fn storage_state_json_keeps_optional_indexed_db_absent() {
        // Arrange
        let payload = r#"{"cookies":[{"name":"session","value":"one","domain":"example.com","path":"/","expires":-1,"httpOnly":false,"secure":true,"sameSite":"Lax"}],"origins":[{"origin":"https://example.com","localStorage":[]}]}"#;

        // Act
        let state = parse_context_storage_state(payload).expect("valid storage state");

        // Assert
        assert_eq!(state.cookies.len(), 1);
        assert_eq!(state.cookies[0].name, "session");
        assert_eq!(state.cookies[0].expires, Some(-1.0));
        assert_eq!(state.origins[0].indexed_db, None);
    }

    #[cfg(feature = "context-storage-state")]
    #[test]
    fn storage_state_json_rejects_non_string_local_storage_value() {
        // Arrange
        let payload = r#"{"origins":[{"origin":"https://example.com","localStorage":[{"name":"key","value":42}]}]}"#;

        // Act
        let error = parse_context_storage_state(payload).expect_err("value must be a string");

        // Assert
        assert!(matches!(error, HostError::InvalidArgument(_)));
    }

    #[cfg(feature = "context-storage-state")]
    #[test]
    fn storage_state_json_rejects_payload_above_byte_cap() {
        // Arrange
        let payload = "x".repeat(MAX_STORAGE_STATE_BYTES + 1);

        // Act
        let error = parse_context_storage_state(&payload).expect_err("payload must be bounded");

        // Assert
        assert!(matches!(error, HostError::ResourceExhausted(_)));
    }

    #[cfg(feature = "keyboard")]
    #[test]
    fn python_keyboard_type_owns_text_and_converts_delay() {
        // Arrange
        let text = String::from("hello");

        // Act
        let operation =
            python_keyboard_operation("type", &text, 12.5).expect("valid Keyboard type operation");

        // Assert
        let PythonKeyboardOperation::Type(observed_text, delay) = operation else {
            panic!("expected Keyboard type operation");
        };
        assert_eq!(observed_text, "hello");
        assert_eq!(delay, std::time::Duration::from_micros(12_500));
    }

    #[cfg(feature = "keyboard")]
    #[test]
    fn python_keyboard_delay_rejects_non_finite_values() {
        // Arrange
        let delay = f64::NAN;

        // Act
        let error = python_keyboard_delay(delay).expect_err("non-finite delay must fail");

        // Assert
        assert!(matches!(error, HostError::InvalidArgument(_)));
    }

    #[test]
    fn python_timeout_message_matches_playwright_punctuation() {
        // Arrange
        let error = HostError::Timeout(456);

        // Act
        let message = python_exception_message(&error);

        // Assert
        assert_eq!(message, "Timeout 456ms exceeded.");
    }

    #[test]
    fn python_non_timeout_message_is_unchanged() {
        // Arrange
        let error = HostError::TargetClosed;

        // Act
        let message = python_exception_message(&error);

        // Assert
        assert_eq!(message, "Target page, context or browser has been closed");
    }
}
