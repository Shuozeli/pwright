//! Optional Page request-routing leaf.
//!
//! Chromium interception, bounded paused-request ownership, glob selection and
//! terminal continue/abort commands live here. Language bindings retain only
//! user callbacks and public wrapper identity.

use std::borrow::Cow;
use std::error::Error;
use std::future::Future;
#[cfg(feature = "large-route-post-data")]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
#[cfg(feature = "route-request-history")]
use std::time::Duration;

use base64::Engine as _;
use pwright_backend_cdp_topology::CdpTopology;
use pwright_cdp::{
    CdpConnection, CdpEvent, CdpEventReceiver, CdpEventRecvError, connection::CdpError,
    generated::page,
};
use pwright_composition_chromium::SharedChromiumTopology;
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use pwright_plugin_chromium_frame_observer::{ChromiumFrameEventObserver, FrameObservationResult};
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer, ChromiumSessionResumeFence,
};
#[cfg(feature = "route-regexp")]
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value, json};
#[cfg(any(feature = "har-replay", feature = "route-fulfill-path"))]
use tokio::io::AsyncReadExt as _;
use tokio::sync::{Notify, mpsc};
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::{HostError, HostSession};

const SCOPE: ResourceScope = ResourceScope::extension(140, 1);
const MAX_ROUTES: usize = 256;
#[cfg(feature = "route-regexp")]
const MAX_REGEXP_ROUTES: usize = 32;
const MAX_PENDING_CALLS: usize = 64;
const MAX_RETIRED_CALLS: usize = MAX_PENDING_CALLS;
#[cfg(feature = "route-request-history")]
const ROUTE_REQUEST_HISTORY_LOOKUP_TIMEOUT_MS: u32 = 30_000;
#[cfg(feature = "route-redirects")]
const MAX_REDIRECT_HEADER_OVERRIDES: usize = MAX_PENDING_CALLS;
pub const MAX_ROUTE_INITIALIZING_SESSIONS: usize = 64;
const MAX_PATTERN_BYTES: usize = 4 * 1024;
#[cfg(feature = "route-regexp")]
const MAX_REGEXP_FLAGS_BYTES: usize = 4;
#[cfg(feature = "route-regexp")]
const REGEXP_PROGRAM_LIMIT_BYTES: usize = 64 * 1024;
#[cfg(feature = "route-regexp")]
const REGEXP_DFA_LIMIT_BYTES: usize = 64 * 1024;
#[cfg(feature = "route-regexp")]
const REGEXP_STORAGE_BYTES: usize =
    MAX_REGEXP_ROUTES * (REGEXP_PROGRAM_LIMIT_BYTES + REGEXP_DFA_LIMIT_BYTES + MAX_PATTERN_BYTES);
const MAX_CDP_PATTERN_BYTES: usize = 2 * MAX_PATTERN_BYTES + 1;
const MAX_URL_BYTES: usize = 64 * 1024;
const MAX_METHOD_BYTES: usize = 64;
const MAX_RESOURCE_TYPE_BYTES: usize = 64;
const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_HEADER_COUNT: usize = 256;
const MAX_POST_DATA_ENTRIES: usize = 256;
const MAX_CAPTURED_POST_DATA_ENCODED_BYTES: usize =
    4 * (MAX_CAPTURED_POST_DATA_BYTES.div_ceil(3) + MAX_POST_DATA_ENTRIES);
// User-supplied continue overrides stay on their existing small boundary.
const MAX_POST_DATA_BYTES: usize = 256 * 1024;
#[cfg(feature = "large-route-post-data")]
pub(crate) const MAX_CAPTURED_POST_DATA_BYTES: usize = 3 * 1024 * 1024;
#[cfg(not(feature = "large-route-post-data"))]
pub(crate) const MAX_CAPTURED_POST_DATA_BYTES: usize = MAX_POST_DATA_BYTES;
#[cfg(feature = "large-route-post-data")]
const MAX_LARGE_POST_DATA_CALLS: usize = 4;
const MAX_NATIVE_ID_BYTES: usize = 4 * 1024;
#[cfg(feature = "network-events")]
const MAX_FULFILLED_BODIES: usize = 8;
pub const MAX_ROUTE_FULFILL_BODY_BYTES: usize = 1024 * 1024;
#[cfg(feature = "route-fulfill-path")]
pub const MAX_ROUTE_FULFILL_PATH_BYTES: usize = 4 * 1024;
pub const MAX_ROUTE_CONTINUE_INPUT_BYTES: usize = MAX_URL_BYTES
    + MAX_METHOD_BYTES
    + MAX_HEADER_BYTES
    + MAX_POST_DATA_BYTES
    + MAX_HEADER_COUNT * 2 * std::mem::size_of::<String>()
    + 16 * 1024;
#[cfg(feature = "har-replay")]
pub const MAX_HAR_REPLAY_ARCHIVE_BYTES: usize = 8 * 1024 * 1024;
#[cfg(feature = "har-replay")]
pub const MAX_HAR_REPLAY_RETAINED_BYTES: usize = 8 * 1024 * 1024;
#[cfg(feature = "har-replay")]
pub const MAX_HAR_REPLAY_ARCHIVES: usize = 8;
#[cfg(feature = "har-replay")]
const MAX_HAR_PATH_BYTES: usize = 4 * 1024;
#[cfg(feature = "har-replay")]
const HAR_REPLAY_STORAGE_BYTES: usize = MAX_HAR_REPLAY_RETAINED_BYTES;
#[cfg(not(feature = "har-replay"))]
const HAR_REPLAY_STORAGE_BYTES: usize = 0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteCall {
    pub id: u64,
    pub registration_id: u64,
    pub context: ObjectId,
    pub page: ObjectId,
    pub frame: Option<ObjectId>,
    pub url: String,
    pub method: String,
    pub resource_type: String,
    pub headers: Vec<(String, String)>,
    pub post_data: Option<Vec<u8>>,
    pub is_navigation_request: bool,
    pub expires: bool,
    #[cfg(feature = "network-events")]
    pub native_request_id: Option<String>,
}

#[cfg(feature = "route-request-history")]
async fn capture_route_history_owner<T, F, Fut>(
    call: &RouteCall,
    lookup: F,
) -> Result<Option<T>, HostError>
where
    F: FnOnce(String, String) -> Fut,
    Fut: Future<Output = Result<T, HostError>>,
{
    let Some(native_request_id) = &call.native_request_id else {
        return Ok(None);
    };
    let page_id = call.page.get().to_string();
    let lookup = lookup(page_id, native_request_id.clone());
    tokio::time::timeout(
        Duration::from_millis(u64::from(ROUTE_REQUEST_HISTORY_LOOKUP_TIMEOUT_MS)),
        lookup,
    )
    .await
    .map_err(|_| HostError::Timeout(ROUTE_REQUEST_HISTORY_LOOKUP_TIMEOUT_MS))?
    .map(Some)
}

#[derive(Clone)]
struct Registration {
    id: u64,
    scope: RegistrationScope,
    pattern: RoutePattern,
    handler: RegistrationHandler,
    remaining: Option<u64>,
}

#[derive(Clone)]
enum RoutePattern {
    Glob(Box<str>),
    #[cfg(feature = "route-regexp")]
    RegExp(Arc<Regex>),
}

impl From<String> for RoutePattern {
    fn from(pattern: String) -> Self {
        Self::Glob(pattern.into_boxed_str())
    }
}

impl From<&str> for RoutePattern {
    fn from(pattern: &str) -> Self {
        Self::Glob(pattern.into())
    }
}

impl RoutePattern {
    fn matches_url(&self, url: &str) -> Result<bool, HostError> {
        match self {
            Self::Glob(pattern) => route_matches(pattern, url),
            #[cfg(feature = "route-regexp")]
            Self::RegExp(pattern) => Ok(pattern.is_match(url)),
        }
    }

    fn source_len(&self) -> usize {
        match self {
            Self::Glob(pattern) => pattern.len(),
            #[cfg(feature = "route-regexp")]
            Self::RegExp(pattern) => pattern.as_str().len(),
        }
    }

    #[cfg(feature = "route-regexp")]
    fn regexp(source: String, flags: String) -> Result<Self, HostError> {
        if source.len() > MAX_PATTERN_BYTES {
            return Err(HostError::InvalidArgument(format!(
                "route RegExp source exceeds {MAX_PATTERN_BYTES} UTF-8 bytes"
            )));
        }
        if flags.len() > MAX_REGEXP_FLAGS_BYTES {
            return Err(HostError::InvalidArgument(format!(
                "route RegExp flags exceed {MAX_REGEXP_FLAGS_BYTES} UTF-8 bytes"
            )));
        }
        let mut builder = RegexBuilder::new(&source);
        let mut seen = 0_u8;
        for flag in flags.bytes() {
            let bit = match flag {
                b'i' => {
                    builder.case_insensitive(true);
                    1
                }
                b'm' => {
                    builder.multi_line(true);
                    2
                }
                b's' => {
                    builder.dot_matches_new_line(true);
                    4
                }
                b'u' => 8,
                _ => {
                    return Err(HostError::InvalidArgument(
                        "unsupported route RegExp flag; only i, m, s and u are supported".into(),
                    ));
                }
            };
            if seen & bit != 0 {
                return Err(HostError::InvalidArgument(
                    "duplicate route RegExp flag".into(),
                ));
            }
            seen |= bit;
        }
        builder
            .size_limit(REGEXP_PROGRAM_LIMIT_BYTES)
            .dfa_size_limit(REGEXP_DFA_LIMIT_BYTES)
            .build()
            .map(|pattern| Self::RegExp(Arc::new(pattern)))
            .map_err(|error| {
                HostError::InvalidArgument(format!(
                    "unsupported or invalid route RegExp syntax: {error}"
                ))
            })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RegistrationScope {
    Page(ObjectId),
    Context(ObjectId),
}

impl RegistrationScope {
    fn applies_to(self, page: ObjectId, context: ObjectId) -> bool {
        match self {
            Self::Page(expected) => expected == page,
            Self::Context(expected) => expected == context,
        }
    }

    fn is_active(self, active_pages: &[ObjectId], active_contexts: &[ObjectId]) -> bool {
        match self {
            Self::Page(page) => active_pages.contains(&page),
            Self::Context(context) => active_contexts.contains(&context),
        }
    }
}

#[derive(Clone)]
enum RegistrationHandler {
    Binding,
    #[cfg(feature = "context-storage-state")]
    StorageState,
    #[cfg(feature = "har-replay")]
    Har {
        archive: Arc<pwright_har_replay::ReplayArchive>,
        not_found: HarNotFoundAction,
    },
}

#[derive(Clone)]
struct RegistrationMatch {
    id: u64,
    scope: RegistrationScope,
    handler: RegistrationHandler,
    expires: bool,
    next_cursor: FallbackCursor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FallbackCursor {
    Page {
        before_page: u64,
        before_context: u64,
    },
    Context {
        before_context: u64,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RouteRefresh {
    page: bool,
    context: bool,
}

impl RouteRefresh {
    fn include(&mut self, scope: RegistrationScope) {
        match scope {
            RegistrationScope::Page(_) => self.page = true,
            RegistrationScope::Context(_) => self.context = true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RouteSettlement {
    page: ObjectId,
    context: ObjectId,
    refresh: RouteRefresh,
}

#[cfg(feature = "har-replay")]
struct ReplayCall {
    call: RouteCall,
    archive: Arc<pwright_har_replay::ReplayArchive>,
    not_found: HarNotFoundAction,
}

enum NativeRouteCall {
    Continue {
        session_id: Box<str>,
        request_id: Box<str>,
        options: RouteContinue,
    },
    Abort {
        session_id: Box<str>,
        request_id: Box<str>,
    },
    Fulfill {
        session_id: Box<str>,
        request_id: Box<str>,
        response: RouteFulfill,
    },
    #[cfg(feature = "har-replay")]
    Replay(ReplayCall),
}

struct PendingCall {
    id: u64,
    context: ObjectId,
    page: ObjectId,
    frame: Option<ObjectId>,
    registration_scope: RegistrationScope,
    fallback_cursor: FallbackCursor,
    session_id: Box<str>,
    request_id: Box<str>,
    request_scheme: Box<str>,
    url: String,
    method: String,
    resource_type: String,
    headers: Vec<(String, String)>,
    post_data: Option<Vec<u8>>,
    #[cfg(feature = "large-route-post-data")]
    large_post_data_slot: Option<Arc<LargePostDataSlot>>,
    is_navigation_request: bool,
    fallback_options: RouteContinue,
    refresh: RouteRefresh,
    #[cfg(feature = "network-events")]
    network_request_id: Option<Box<str>>,
}

#[derive(Clone, Copy)]
struct SettlingCall {
    id: u64,
    page: ObjectId,
    registration_scope: RegistrationScope,
}

#[derive(Clone, Copy)]
struct SessionScope {
    page: ObjectId,
    context: ObjectId,
    ownership: pwright_backend_cdp_topology::CdpContextOwnership,
}

struct InitializingSession {
    session_id: Box<str>,
    scope: SessionScope,
}

#[cfg(feature = "route-redirects")]
struct RedirectHeaderOverride {
    page: ObjectId,
    session_id: Box<str>,
    fetch_request_id: Box<str>,
    network_request_id: Option<Box<str>>,
    headers: Vec<(String, String)>,
}

#[cfg(feature = "network-events")]
struct FulfilledBody {
    page: ObjectId,
    session_id: Box<str>,
    network_request_id: Box<str>,
    body: Arc<[u8]>,
}
#[cfg(feature = "network-events")]
type NativeRequestKey = (Box<str>, Box<str>);

struct State {
    next_registration: u64,
    next_call: u64,
    registrations: Vec<Registration>,
    pending: Vec<PendingCall>,
    settling: Vec<SettlingCall>,
    retired: Vec<u64>,
    initializing_sessions: Vec<InitializingSession>,
    #[cfg(feature = "route-redirects")]
    redirect_header_overrides: Vec<RedirectHeaderOverride>,
    #[cfg(feature = "har-replay")]
    har_bytes: usize,
    #[cfg(feature = "network-events")]
    fulfilled: Vec<FulfilledBody>,
}

fn take_pending_action(state: &mut State, id: u64) -> Result<Option<PendingCall>, HostError> {
    if let Some(index) = state.pending.iter().position(|pending| pending.id == id) {
        let pending = state.pending.swap_remove(index);
        state.settling.push(SettlingCall {
            id: pending.id,
            page: pending.page,
            registration_scope: pending.registration_scope,
        });
        return Ok(Some(pending));
    }
    if let Some(index) = state.retired.iter().position(|retired| *retired == id) {
        state.retired.swap_remove(index);
        return Ok(None);
    }
    Err(HostError::InvalidArgument(
        "route is already handled".into(),
    ))
}

#[cfg(feature = "large-route-post-data")]
fn is_large_post_data(post_data_bytes: Option<usize>) -> bool {
    post_data_bytes.is_some_and(|bytes| bytes > MAX_POST_DATA_BYTES)
}

#[cfg(feature = "large-route-post-data")]
#[derive(Debug, Default)]
struct LargePostDataBudget {
    active: AtomicUsize,
}

#[cfg(feature = "large-route-post-data")]
impl LargePostDataBudget {
    fn acquire(
        self: &Arc<Self>,
        post_data_bytes: Option<usize>,
    ) -> Result<Option<Arc<LargePostDataSlot>>, HostError> {
        if !is_large_post_data(post_data_bytes) {
            return Ok(None);
        }
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < MAX_LARGE_POST_DATA_CALLS).then_some(active + 1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted(format!(
                    "large Page route POST-data capacity exceeded ({MAX_LARGE_POST_DATA_CALLS})"
                ))
            })?;
        Ok(Some(Arc::new(LargePostDataSlot {
            budget: Arc::clone(self),
        })))
    }
}

#[cfg(feature = "large-route-post-data")]
#[derive(Debug)]
struct LargePostDataSlot {
    budget: Arc<LargePostDataBudget>,
}

#[cfg(feature = "large-route-post-data")]
impl Drop for LargePostDataSlot {
    fn drop(&mut self) {
        let previous = self.budget.active.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

#[derive(Clone, Copy)]
enum CapturedPostData<'a> {
    Missing,
    Text(&'a str),
    Entries {
        entries: &'a [Value],
        decoded_bytes: usize,
    },
}

#[cfg(feature = "large-route-post-data")]
impl CapturedPostData<'_> {
    const fn bytes(&self) -> Option<usize> {
        match self {
            Self::Missing => None,
            Self::Text(value) => Some(value.len()),
            Self::Entries { decoded_bytes, .. } => Some(*decoded_bytes),
        }
    }
}

fn invalid_post_data(field: &'static str) -> HostError {
    HostError::Backend(format!("Fetch request has invalid {field}"))
}

fn captured_post_data_limit(actual: usize) -> HostError {
    HostError::ResourceExhausted(format!(
        "route POST data exceeds byte limit ({MAX_CAPTURED_POST_DATA_BYTES}): {actual}"
    ))
}

fn decoded_entry_bytes(encoded: &str) -> Result<usize, HostError> {
    let length = encoded.len();
    if !length.is_multiple_of(4) {
        return Err(invalid_post_data("postDataEntries.bytes base64 length"));
    }
    let padding = encoded
        .as_bytes()
        .iter()
        .rev()
        .take_while(|byte| **byte == b'=')
        .count();
    if padding > 2 {
        return Err(invalid_post_data("postDataEntries.bytes base64 padding"));
    }
    length
        .checked_div(4)
        .and_then(|groups| groups.checked_mul(3))
        .and_then(|bytes| bytes.checked_sub(padding))
        .ok_or_else(|| invalid_post_data("postDataEntries.bytes base64 size"))
}

fn captured_post_data_source(
    request: &Map<String, Value>,
) -> Result<CapturedPostData<'_>, HostError> {
    if let Some(entries) = request.get("postDataEntries") {
        let entries = entries
            .as_array()
            .ok_or_else(|| invalid_post_data("postDataEntries"))?;
        if entries.len() > MAX_POST_DATA_ENTRIES {
            return Err(HostError::ResourceExhausted(format!(
                "route POST data entry count exceeds limit ({MAX_POST_DATA_ENTRIES})"
            )));
        }
        let mut decoded_bytes = 0usize;
        let mut encoded_bytes = 0usize;
        for entry in entries {
            let entry = entry
                .as_object()
                .ok_or_else(|| invalid_post_data("postDataEntries entry"))?;
            let Some(encoded) = entry.get("bytes") else {
                continue;
            };
            let encoded = encoded
                .as_str()
                .ok_or_else(|| invalid_post_data("postDataEntries.bytes"))?;
            encoded_bytes = encoded_bytes
                .checked_add(encoded.len())
                .ok_or_else(|| captured_post_data_limit(usize::MAX))?;
            if encoded_bytes > MAX_CAPTURED_POST_DATA_ENCODED_BYTES {
                return Err(HostError::ResourceExhausted(format!(
                    "route POST data base64 exceeds byte limit ({MAX_CAPTURED_POST_DATA_ENCODED_BYTES})"
                )));
            }
            decoded_bytes = decoded_bytes
                .checked_add(decoded_entry_bytes(encoded)?)
                .ok_or_else(|| captured_post_data_limit(usize::MAX))?;
            if decoded_bytes > MAX_CAPTURED_POST_DATA_BYTES {
                return Err(captured_post_data_limit(decoded_bytes));
            }
        }
        return Ok(CapturedPostData::Entries {
            entries,
            decoded_bytes,
        });
    }
    let Some(value) = request.get("postData") else {
        return Ok(CapturedPostData::Missing);
    };
    let value = value
        .as_str()
        .ok_or_else(|| invalid_post_data("postData"))?;
    if value.len() > MAX_CAPTURED_POST_DATA_BYTES {
        return Err(captured_post_data_limit(value.len()));
    }
    Ok(CapturedPostData::Text(value))
}

fn captured_post_data(source: CapturedPostData<'_>) -> Result<Option<Vec<u8>>, HostError> {
    let (bytes, entries) = match source {
        CapturedPostData::Missing => return Ok(None),
        CapturedPostData::Text(value) => (value.len(), None),
        CapturedPostData::Entries {
            entries,
            decoded_bytes,
        } => (decoded_bytes, Some(entries)),
    };
    let mut decoded = Vec::new();
    decoded
        .try_reserve_exact(bytes)
        .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
    if let Some(entries) = entries {
        for entry in entries {
            let Some(encoded) = entry.get("bytes").and_then(Value::as_str) else {
                continue;
            };
            let part_bytes = decoded_entry_bytes(encoded)?;
            let start = decoded.len();
            decoded.resize(start + part_bytes, 0);
            let written = base64::engine::general_purpose::STANDARD
                .decode_slice(encoded, &mut decoded[start..])
                .map_err(|_| invalid_post_data("postDataEntries.bytes base64"))?;
            if written != part_bytes {
                return Err(invalid_post_data("postDataEntries.bytes decoded length"));
            }
        }
        if decoded.is_empty() {
            return Ok(None);
        }
    } else if let CapturedPostData::Text(value) = source {
        decoded.extend_from_slice(value.as_bytes());
    }
    Ok(Some(decoded))
}

fn reconcile_pending(state: &mut State, active: &[ObjectId]) {
    let mut index = 0;
    while index < state.pending.len() {
        if active.contains(&state.pending[index].page) {
            index += 1;
            continue;
        }
        let retired = state.pending.swap_remove(index).id;
        if state.retired.len() == MAX_RETIRED_CALLS {
            state.retired.remove(0);
        }
        state.retired.push(retired);
    }
}

#[cfg(feature = "route-redirects")]
fn advance_redirect_header_override(
    state: &mut State,
    session_id: &str,
    previous_fetch_request_id: &str,
    fetch_request_id: &str,
    network_request_id: Option<&str>,
) -> Result<RouteContinue, HostError> {
    let Some(entry) = state.redirect_header_overrides.iter_mut().find(|entry| {
        entry.session_id.as_ref() == session_id
            && entry.fetch_request_id.as_ref() == previous_fetch_request_id
    }) else {
        return Ok(RouteContinue::default());
    };
    entry.fetch_request_id = fetch_request_id.to_owned().into_boxed_str();
    if let Some(network_request_id) = network_request_id {
        entry.network_request_id = Some(network_request_id.to_owned().into_boxed_str());
    }
    entry
        .headers
        .retain(|(name, _)| !name.eq_ignore_ascii_case("cookie"));
    RouteContinue::new(None, None, Some(entry.headers.clone()), None)
}

#[cfg(feature = "route-redirects")]
fn retain_redirect_header_override(
    state: &mut State,
    pending: &PendingCall,
    headers: &[(String, String)],
) -> Result<(), HostError> {
    if let Some(entry) = state.redirect_header_overrides.iter_mut().find(|entry| {
        entry.session_id.as_ref() == pending.session_id.as_ref()
            && entry.fetch_request_id.as_ref() == pending.request_id.as_ref()
    }) {
        entry.headers = headers.to_vec();
        entry.network_request_id = pending.network_request_id.clone();
        entry.page = pending.page;
        return Ok(());
    }
    if state.redirect_header_overrides.len() == MAX_REDIRECT_HEADER_OVERRIDES {
        return Err(HostError::ResourceExhausted(format!(
            "route redirect header override capacity exceeded ({MAX_REDIRECT_HEADER_OVERRIDES})"
        )));
    }
    state
        .redirect_header_overrides
        .push(RedirectHeaderOverride {
            page: pending.page,
            session_id: pending.session_id.clone(),
            fetch_request_id: pending.request_id.clone(),
            network_request_id: pending.network_request_id.clone(),
            headers: headers.to_vec(),
        });
    Ok(())
}

fn scope_is_idle(state: &State, scope: RegistrationScope) -> bool {
    state
        .pending
        .iter()
        .all(|pending| pending.registration_scope != scope)
        && state
            .settling
            .iter()
            .all(|settling| settling.registration_scope != scope)
}

#[cfg(feature = "network-events")]
const FULFILLED_STORAGE_BYTES: usize = MAX_FULFILLED_BODIES
    * (std::mem::size_of::<FulfilledBody>()
        + (2 * MAX_NATIVE_ID_BYTES)
        + MAX_ROUTE_FULFILL_BODY_BYTES);
#[cfg(not(feature = "network-events"))]
const FULFILLED_STORAGE_BYTES: usize = 0;

#[cfg(feature = "route-redirects")]
const REDIRECT_HEADER_OVERRIDE_STORAGE_BYTES: usize = MAX_REDIRECT_HEADER_OVERRIDES
    * (std::mem::size_of::<RedirectHeaderOverride>()
        + (3 * MAX_NATIVE_ID_BYTES)
        + MAX_HEADER_BYTES
        + MAX_HEADER_COUNT * 2 * std::mem::size_of::<String>());
#[cfg(not(feature = "route-redirects"))]
const REDIRECT_HEADER_OVERRIDE_STORAGE_BYTES: usize = 0;

pub(crate) struct Installation {
    connection: Arc<CdpConnection>,
    #[cfg(feature = "context-http-credentials")]
    http_credentials: Arc<crate::context_http_credentials::Installation>,
    state: Mutex<State>,
    sender: mpsc::Sender<QueuedRouteCall>,
    receiver: tokio::sync::Mutex<mpsc::Receiver<QueuedRouteCall>>,
    #[cfg(feature = "large-route-post-data")]
    large_post_data_budget: Arc<LargePostDataBudget>,
    changed: Notify,
    _storage: ResourceLease,
}

struct QueuedRouteCall {
    call: RouteCall,
    #[cfg(feature = "large-route-post-data")]
    _large_post_data_slot: Option<Arc<LargePostDataSlot>>,
}

pub(crate) const fn requirements() -> ResourceRequirements {
    #[cfg(feature = "large-route-post-data")]
    let large_post_data_storage_bytes =
        MAX_LARGE_POST_DATA_CALLS * 2 * (MAX_CAPTURED_POST_DATA_BYTES - MAX_POST_DATA_BYTES);
    #[cfg(not(feature = "large-route-post-data"))]
    let large_post_data_storage_bytes = 0;
    let bytes = (std::mem::size_of::<Installation>()
        + std::mem::size_of::<State>()
        + MAX_ROUTES * (std::mem::size_of::<Registration>() + MAX_PATTERN_BYTES)
        + MAX_PENDING_CALLS
            * (std::mem::size_of::<PendingCall>()
                + std::mem::size_of::<QueuedRouteCall>()
                + (3 * MAX_NATIVE_ID_BYTES)
                + MAX_URL_BYTES
                + MAX_URL_BYTES
                + MAX_METHOD_BYTES
                + MAX_RESOURCE_TYPE_BYTES
                + MAX_HEADER_BYTES
                + MAX_POST_DATA_BYTES
                + MAX_POST_DATA_BYTES
                + MAX_ROUTE_CONTINUE_INPUT_BYTES)
        + MAX_PENDING_CALLS * std::mem::size_of::<SettlingCall>()
        + std::mem::size_of::<Notify>()
        + MAX_RETIRED_CALLS * std::mem::size_of::<u64>()
        + MAX_ROUTE_INITIALIZING_SESSIONS
            * (std::mem::size_of::<InitializingSession>() + MAX_NATIVE_ID_BYTES)
        + {
            #[cfg(feature = "route-regexp")]
            {
                REGEXP_STORAGE_BYTES
            }
            #[cfg(not(feature = "route-regexp"))]
            {
                0
            }
        }
        + REDIRECT_HEADER_OVERRIDE_STORAGE_BYTES
        + FULFILLED_STORAGE_BYTES
        + HAR_REPLAY_STORAGE_BYTES
        + large_post_data_storage_bytes
        + {
            #[cfg(feature = "large-route-post-data")]
            {
                std::mem::size_of::<LargePostDataBudget>()
                    + MAX_LARGE_POST_DATA_CALLS * std::mem::size_of::<LargePostDataSlot>()
            }
            #[cfg(not(feature = "large-route-post-data"))]
            {
                0
            }
        }) as u64;
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(1, 1)
}

fn take_matching_registration(
    state: &mut State,
    page: ObjectId,
    context: ObjectId,
    url: &str,
    _auth_enabled: bool,
) -> Result<Option<RegistrationMatch>, HostError> {
    let ceiling = state.next_registration;
    take_matching_registration_after(
        state,
        page,
        context,
        url,
        FallbackCursor::Page {
            before_page: ceiling,
            before_context: ceiling,
        },
    )
}

fn has_matching_registration(
    state: &State,
    page: ObjectId,
    context: ObjectId,
    url: &str,
) -> Result<bool, HostError> {
    for registration in state.registrations.iter().rev() {
        if registration.scope.applies_to(page, context) && registration.pattern.matches_url(url)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn take_matching_registration_after(
    state: &mut State,
    page: ObjectId,
    context: ObjectId,
    url: &str,
    cursor: FallbackCursor,
) -> Result<Option<RegistrationMatch>, HostError> {
    let mut selected = None;
    match cursor {
        FallbackCursor::Page {
            before_page,
            before_context,
        } => {
            if let Some(index) =
                matching_registration_index(state, RegistrationScope::Page(page), before_page, url)?
            {
                let id = state.registrations[index].id;
                selected = Some((
                    index,
                    FallbackCursor::Page {
                        before_page: id,
                        before_context,
                    },
                ));
            } else if let Some(index) = matching_registration_index(
                state,
                RegistrationScope::Context(context),
                before_context,
                url,
            )? {
                let id = state.registrations[index].id;
                selected = Some((index, FallbackCursor::Context { before_context: id }));
            }
        }
        FallbackCursor::Context { before_context } => {
            if let Some(index) = matching_registration_index(
                state,
                RegistrationScope::Context(context),
                before_context,
                url,
            )? {
                let id = state.registrations[index].id;
                selected = Some((index, FallbackCursor::Context { before_context: id }));
            }
        }
    }
    let Some((selected_index, next_cursor)) = selected else {
        return Ok(None);
    };
    let registration = &mut state.registrations[selected_index];
    let expires = match registration.remaining {
        Some(1) => true,
        Some(remaining) => {
            registration.remaining = Some(remaining - 1);
            false
        }
        None => false,
    };
    let selected = RegistrationMatch {
        id: registration.id,
        scope: registration.scope,
        handler: registration.handler.clone(),
        expires,
        next_cursor,
    };
    if expires {
        state.registrations.remove(selected_index);
    }
    Ok(Some(selected))
}

fn matching_registration_index(
    state: &State,
    scope: RegistrationScope,
    before: u64,
    url: &str,
) -> Result<Option<usize>, HostError> {
    for (index, registration) in state.registrations.iter().enumerate().rev() {
        if registration.id < before
            && registration.scope == scope
            && registration.pattern.matches_url(url)?
        {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

fn ensure_registration_capacity(
    registrations: &[Registration],
    pattern: &RoutePattern,
) -> Result<(), HostError> {
    if registrations.len() == MAX_ROUTES {
        return Err(HostError::ResourceExhausted(format!(
            "Page route capacity exceeded ({MAX_ROUTES})"
        )));
    }
    #[cfg(feature = "route-regexp")]
    if matches!(pattern, RoutePattern::RegExp(_))
        && registrations
            .iter()
            .filter(|registration| matches!(&registration.pattern, RoutePattern::RegExp(_)))
            .count()
            == MAX_REGEXP_ROUTES
    {
        return Err(HostError::ResourceExhausted(format!(
            "Page route RegExp capacity exceeded ({MAX_REGEXP_ROUTES})"
        )));
    }
    #[cfg(not(feature = "route-regexp"))]
    let _ = pattern;
    Ok(())
}

impl Installation {
    pub(crate) fn new(
        connection: Arc<CdpConnection>,
        resources: &ProductionResourceLedger,
        #[cfg(feature = "context-http-credentials")] http_credentials: Arc<
            crate::context_http_credentials::Installation,
        >,
    ) -> Result<Arc<Self>, HostError> {
        let plan = requirements();
        let ledger = resources.claim(plan).map_err(HostError::backend)?;
        let storage = ledger
            .reserve(
                SCOPE,
                ResourceClass::BackendStateBytes,
                plan.class(ResourceClass::BackendStateBytes).global,
            )
            .map_err(HostError::backend)?;
        let mut registrations = Vec::new();
        registrations
            .try_reserve_exact(MAX_ROUTES)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(MAX_PENDING_CALLS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let mut settling = Vec::new();
        settling
            .try_reserve_exact(MAX_PENDING_CALLS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let mut retired = Vec::new();
        retired
            .try_reserve_exact(MAX_RETIRED_CALLS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let mut initializing_sessions = Vec::new();
        initializing_sessions
            .try_reserve_exact(MAX_ROUTE_INITIALIZING_SESSIONS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        #[cfg(feature = "route-redirects")]
        let mut redirect_header_overrides = Vec::new();
        #[cfg(feature = "route-redirects")]
        redirect_header_overrides
            .try_reserve_exact(MAX_REDIRECT_HEADER_OVERRIDES)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        #[cfg(feature = "network-events")]
        let mut fulfilled = Vec::new();
        #[cfg(feature = "network-events")]
        fulfilled
            .try_reserve_exact(MAX_FULFILLED_BODIES)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let (sender, receiver) = mpsc::channel(MAX_PENDING_CALLS);
        Ok(Arc::new(Self {
            connection,
            #[cfg(feature = "context-http-credentials")]
            http_credentials,
            state: Mutex::new(State {
                next_registration: 1,
                next_call: 1,
                registrations,
                pending,
                settling,
                retired,
                initializing_sessions,
                #[cfg(feature = "route-redirects")]
                redirect_header_overrides,
                #[cfg(feature = "har-replay")]
                har_bytes: 0,
                #[cfg(feature = "network-events")]
                fulfilled,
            }),
            sender,
            receiver: tokio::sync::Mutex::new(receiver),
            #[cfg(feature = "large-route-post-data")]
            large_post_data_budget: Arc::new(LargePostDataBudget::default()),
            changed: Notify::new(),
            _storage: storage,
        }))
    }

    fn add_registration_with_handler(
        &self,
        scope: RegistrationScope,
        pattern: RoutePattern,
        handler: RegistrationHandler,
        times: Option<u64>,
    ) -> Result<u64, HostError> {
        if pattern.source_len() > MAX_PATTERN_BYTES
            || matches!(&pattern, RoutePattern::Glob(glob) if glob.is_empty())
        {
            return Err(HostError::InvalidArgument(format!(
                "route pattern must contain 1..={MAX_PATTERN_BYTES} UTF-8 bytes"
            )));
        }
        let mut state = self.state.lock().map_err(HostError::backend)?;
        ensure_registration_capacity(&state.registrations, &pattern)?;
        #[cfg(feature = "har-replay")]
        if let RegistrationHandler::Har { archive, .. } = &handler {
            let archive_count = state
                .registrations
                .iter()
                .filter(|registration| {
                    matches!(registration.handler, RegistrationHandler::Har { .. })
                })
                .count();
            if archive_count == MAX_HAR_REPLAY_ARCHIVES {
                return Err(HostError::ResourceExhausted(format!(
                    "HAR replay archive capacity exceeded ({MAX_HAR_REPLAY_ARCHIVES})"
                )));
            }
            let next = state
                .har_bytes
                .checked_add(archive.retained_bytes())
                .ok_or_else(|| {
                    HostError::ResourceExhausted("HAR replay retained-byte count overflow".into())
                })?;
            if next > MAX_HAR_REPLAY_RETAINED_BYTES {
                return Err(HostError::ResourceExhausted(format!(
                    "HAR replay retained bytes exceed {MAX_HAR_REPLAY_RETAINED_BYTES}"
                )));
            }
            state.har_bytes = next;
        }
        let id = state.next_registration;
        state.next_registration = id
            .checked_add(1)
            .ok_or_else(|| HostError::ResourceExhausted("route ID space exhausted".into()))?;
        state.registrations.push(Registration {
            id,
            scope,
            pattern,
            handler,
            remaining: times.filter(|times| *times != 0),
        });
        Ok(id)
    }

    fn remove_registration(&self, id: u64) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        state
            .registrations
            .retain(|registration| registration.id != id);
        #[cfg(feature = "har-replay")]
        {
            state.har_bytes = retained_har_bytes(&state.registrations)?;
        }
        Ok(())
    }

    fn remove_scope(&self, scope: RegistrationScope) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        state
            .registrations
            .retain(|registration| registration.scope != scope);
        #[cfg(feature = "har-replay")]
        {
            state.har_bytes = retained_har_bytes(&state.registrations)?;
        }
        Ok(())
    }

    fn remove_registrations(&self, scope: RegistrationScope, ids: &[u64]) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        remove_registrations_from_state(&mut state, scope, ids)
    }
}

fn remove_registrations_from_state(
    state: &mut State,
    scope: RegistrationScope,
    ids: &[u64],
) -> Result<(), HostError> {
    if ids.len() > MAX_ROUTES || ids.contains(&0) {
        return Err(HostError::InvalidArgument(
            "route registration IDs are invalid".into(),
        ));
    }
    if state
        .registrations
        .iter()
        .any(|registration| ids.contains(&registration.id) && registration.scope != scope)
    {
        return Err(HostError::InvalidArgument(
            "route registration does not belong to this owner".into(),
        ));
    }
    state
        .registrations
        .retain(|registration| !ids.contains(&registration.id));
    #[cfg(feature = "har-replay")]
    {
        state.har_bytes = retained_har_bytes(&state.registrations)?;
    }
    Ok(())
}

fn cdp_patterns(
    registrations: &[Registration],
    page: ObjectId,
    context: ObjectId,
) -> Result<Vec<String>, HostError> {
    let applicable = registrations
        .iter()
        .filter(|registration| registration.scope.applies_to(page, context));
    #[cfg(feature = "route-regexp")]
    if applicable
        .clone()
        .any(|registration| matches!(&registration.pattern, RoutePattern::RegExp(_)))
    {
        return Ok(vec!["*".into()]);
    }
    let mut patterns = Vec::new();
    patterns
        .try_reserve_exact(applicable.clone().count())
        .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
    for registration in applicable {
        match &registration.pattern {
            RoutePattern::Glob(pattern) => patterns.push(cdp_pattern(pattern)?),
            #[cfg(feature = "route-regexp")]
            RoutePattern::RegExp(_) => return Ok(vec!["*".into()]),
        }
    }
    Ok(patterns)
}

impl Installation {
    async fn wait_scope_idle(
        &self,
        scope: RegistrationScope,
        stopped: &CancellationToken,
    ) -> Result<(), HostError> {
        loop {
            let notified = self.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let idle = {
                let state = self.state.lock().map_err(HostError::backend)?;
                scope_is_idle(&state, scope)
            };
            if idle {
                return Ok(());
            }
            tokio::select! {
                biased;
                () = stopped.cancelled() => return Err(HostError::TargetClosed),
                () = &mut notified => {}
            }
        }
    }

    fn patterns(&self, page: ObjectId, context: ObjectId) -> Result<Vec<String>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        cdp_patterns(&state.registrations, page, context)
    }

    fn has_routes(&self, page: ObjectId, context: ObjectId) -> Result<bool, HostError> {
        Ok(self
            .state
            .lock()
            .map_err(HostError::backend)?
            .registrations
            .iter()
            .any(|registration| registration.scope.applies_to(page, context)))
    }

    #[cfg(feature = "route-regexp")]
    fn has_regexp_route(&self, page: ObjectId, context: ObjectId) -> Result<bool, HostError> {
        Ok(self
            .state
            .lock()
            .map_err(HostError::backend)?
            .registrations
            .iter()
            .any(|registration| {
                registration.scope.applies_to(page, context)
                    && matches!(&registration.pattern, RoutePattern::RegExp(_))
            }))
    }

    fn remember_initializing_session(
        &self,
        session_id: &str,
        scope: SessionScope,
    ) -> Result<(), HostError> {
        if session_id.is_empty() || session_id.len() > MAX_NATIVE_ID_BYTES {
            return Err(HostError::InvalidArgument(format!(
                "sessionId must contain 1..={MAX_NATIVE_ID_BYTES} UTF-8 bytes"
            )));
        }
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if let Some(entry) = state
            .initializing_sessions
            .iter_mut()
            .find(|entry| entry.session_id.as_ref() == session_id)
        {
            entry.scope = scope;
            return Ok(());
        }
        if state.initializing_sessions.len() == MAX_ROUTE_INITIALIZING_SESSIONS {
            return Err(HostError::ResourceExhausted(format!(
                "Page route initializing session capacity exceeded ({MAX_ROUTE_INITIALIZING_SESSIONS})"
            )));
        }
        state.initializing_sessions.push(InitializingSession {
            session_id: session_id.to_owned().into_boxed_str(),
            scope,
        });
        Ok(())
    }

    fn initializing_session(&self, session_id: &str) -> Result<Option<SessionScope>, HostError> {
        Ok(self
            .state
            .lock()
            .map_err(HostError::backend)?
            .initializing_sessions
            .iter()
            .find(|entry| entry.session_id.as_ref() == session_id)
            .map(|entry| entry.scope))
    }

    fn retire_initializing_session(&self, session_id: &str) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        state
            .initializing_sessions
            .retain(|entry| entry.session_id.as_ref() != session_id);
        #[cfg(feature = "route-redirects")]
        state
            .redirect_header_overrides
            .retain(|entry| entry.session_id.as_ref() != session_id);
        Ok(())
    }

    #[cfg(feature = "route-redirects")]
    fn advance_redirect_header_override(
        &self,
        session_id: &str,
        previous_fetch_request_id: &str,
        fetch_request_id: &str,
        network_request_id: Option<&str>,
    ) -> Result<RouteContinue, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        advance_redirect_header_override(
            &mut state,
            session_id,
            previous_fetch_request_id,
            fetch_request_id,
            network_request_id,
        )
    }

    #[cfg(feature = "route-redirects")]
    fn retain_redirect_header_override(
        &self,
        pending: &PendingCall,
        headers: &[(String, String)],
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        retain_redirect_header_override(&mut state, pending, headers)
    }

    #[cfg(feature = "route-redirects")]
    fn remove_redirect_header_override(
        &self,
        session_id: &str,
        fetch_request_id: &str,
    ) -> Result<(), HostError> {
        self.state
            .lock()
            .map_err(HostError::backend)?
            .redirect_header_overrides
            .retain(|entry| {
                entry.session_id.as_ref() != session_id
                    || entry.fetch_request_id.as_ref() != fetch_request_id
            });
        Ok(())
    }

    #[cfg(feature = "route-redirects")]
    fn retire_redirect_header_override(
        &self,
        session_id: Option<&str>,
        network_request_id: &str,
    ) -> Result<(), HostError> {
        self.state
            .lock()
            .map_err(HostError::backend)?
            .redirect_header_overrides
            .retain(|entry| {
                session_id.is_some_and(|session_id| entry.session_id.as_ref() != session_id)
                    || entry.network_request_id.as_deref() != Some(network_request_id)
            });
        Ok(())
    }

    fn take_matching_registration(
        &self,
        page: ObjectId,
        context: ObjectId,
        url: &str,
        auth_enabled: bool,
    ) -> Result<Option<RegistrationMatch>, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        take_matching_registration(&mut state, page, context, url, auth_enabled)
    }

    fn has_matching_registration(
        &self,
        page: ObjectId,
        context: ObjectId,
        url: &str,
    ) -> Result<bool, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        has_matching_registration(&state, page, context, url)
    }

    async fn update_session(
        &self,
        session_id: &str,
        page: ObjectId,
        context: ObjectId,
        auth_enabled: bool,
    ) -> Result<(), HostError> {
        self.update_session_with_resume_fence(session_id, page, context, auth_enabled, None)
            .await
    }

    async fn update_session_with_resume_fence(
        &self,
        session_id: &str,
        page: ObjectId,
        context: ObjectId,
        auth_enabled: bool,
        resume_fence: Option<&ChromiumSessionResumeFence>,
    ) -> Result<(), HostError> {
        let route_patterns = self.patterns(page, context)?;
        if route_patterns.is_empty() && !auth_enabled {
            self.connection
                .send_command_with_session("Fetch.disable", json!({}), Some(session_id))
                .await
                .map_err(HostError::backend)?;
            return Ok(());
        }
        let patterns = if auth_enabled {
            vec![json!({ "urlPattern": "*", "requestStage": "Request" })]
        } else {
            route_patterns
                .into_iter()
                .map(|url_pattern| json!({ "urlPattern": url_pattern, "requestStage": "Request" }))
                .collect::<Vec<_>>()
        };
        let enable_fetch = async {
            let params = json!({
                "patterns": patterns,
                "handleAuthRequests": auth_enabled
            });
            match resume_fence {
                Some(fence) => {
                    fenced_fetch_enable(
                        fence,
                        self.connection.enqueue_command_with_session(
                            "Fetch.enable",
                            params,
                            Some(session_id),
                        ),
                        |command| async move { command.response().await.map(|_| ()) },
                    )
                    .await
                }
                None => self
                    .connection
                    .send_command_with_session("Fetch.enable", params, Some(session_id))
                    .await
                    .map(|_| ()),
            }
        };
        if auth_enabled {
            let disable_cache = self.connection.send_command_with_session(
                "Network.setCacheDisabled",
                json!({ "cacheDisabled": true }),
                Some(session_id),
            );
            tokio::try_join!(disable_cache, enable_fetch).map_err(HostError::backend)?;
            return Ok(());
        }
        enable_fetch.await.map_err(HostError::backend)
    }

    fn observe_event(
        &self,
        topology: &CdpTopology,
        event: &CdpEvent,
        #[cfg(feature = "extra-headers")] extra_headers: &crate::extra_headers::Installation,
    ) -> Result<Option<NativeRouteCall>, HostError> {
        if event.method == "Target.detachedFromTarget" {
            if let Some(session_id) = event
                .params
                .as_object()
                .and_then(|params| params.get("sessionId"))
                .and_then(Value::as_str)
            {
                self.retire_initializing_session(session_id)?;
            }
            return Ok(None);
        }
        #[cfg(feature = "route-redirects")]
        if matches!(
            event.method.as_str(),
            "Network.loadingFinished" | "Network.loadingFailed"
        ) {
            let params = event.params.as_object().ok_or_else(|| invalid("params"))?;
            let request_id = bounded_field(params, "requestId", MAX_NATIVE_ID_BYTES)?;
            self.retire_redirect_header_override(event.session_id.as_deref(), &request_id)?;
            return Ok(None);
        }
        if event.method != "Fetch.requestPaused" {
            return Ok(None);
        }
        let session_id = event
            .session_id
            .as_deref()
            .ok_or_else(|| invalid("sessionId"))?;
        let scope = if let Some(target) = topology.target_session_route(session_id) {
            let page = topology
                .page_object_id(target.page_target_id())
                .ok_or_else(|| invalid("Page route"))?;
            let context = topology
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .map(|route| route.browser_context_object_id())
                .ok_or_else(|| invalid("BrowserContext route"))?;
            let ownership = topology
                .browser_context_routes()
                .find(|route| route.object_id() == context)
                .map(|route| route.ownership())
                .ok_or_else(|| invalid("BrowserContext ownership"))?;
            SessionScope {
                page,
                context,
                ownership,
            }
        } else {
            self.initializing_session(session_id)?
                .ok_or_else(|| invalid("target route"))?
        };
        let params = event.params.as_object().ok_or_else(|| invalid("params"))?;
        let frame = params
            .get("frameId")
            .map(|value| {
                let frame_id = value
                    .as_str()
                    .ok_or_else(|| HostError::Backend("Fetch frameId is not a string".into()))?;
                if frame_id.len() > MAX_NATIVE_ID_BYTES {
                    return Err(HostError::ResourceExhausted(
                        "Fetch frameId exceeds byte limit".into(),
                    ));
                }
                Ok(topology.frame_object_id(frame_id))
            })
            .transpose()?
            .flatten();
        self.admit_call(
            scope,
            frame,
            session_id,
            params,
            #[cfg(feature = "extra-headers")]
            extra_headers,
        )
    }

    fn admit_call(
        &self,
        scope: SessionScope,
        frame: Option<ObjectId>,
        session_id: &str,
        params: &Map<String, Value>,
        #[cfg(feature = "extra-headers")] extra_headers: &crate::extra_headers::Installation,
    ) -> Result<Option<NativeRouteCall>, HostError> {
        let SessionScope {
            context,
            page,
            ownership: _ownership,
        } = scope;
        let request_id = bounded_field(params, "requestId", MAX_NATIVE_ID_BYTES)?;
        #[cfg(feature = "context-http-credentials")]
        let auth_network_id = params.get("networkId").and_then(Value::as_str);
        #[cfg(feature = "network-events")]
        let network_request_id = params
            .get("networkId")
            .and_then(Value::as_str)
            .map(|value| {
                if value.len() > MAX_NATIVE_ID_BYTES {
                    return Err(HostError::ResourceExhausted(
                        "Fetch networkId exceeds byte limit".into(),
                    ));
                }
                Ok(value.to_owned().into_boxed_str())
            })
            .transpose()?;
        #[cfg(feature = "route-redirects")]
        if let Some(previous_fetch_request_id) =
            params.get("redirectedRequestId").and_then(Value::as_str)
        {
            if previous_fetch_request_id.is_empty()
                || previous_fetch_request_id.len() > MAX_NATIVE_ID_BYTES
            {
                return Err(HostError::ResourceExhausted(
                    "Fetch redirectedRequestId exceeds byte limit".into(),
                ));
            }
            let options = self.advance_redirect_header_override(
                session_id,
                previous_fetch_request_id,
                &request_id,
                network_request_id.as_deref(),
            )?;
            return Ok(Some(NativeRouteCall::Continue {
                session_id: session_id.to_owned().into_boxed_str(),
                request_id: request_id.into_boxed_str(),
                options,
            }));
        }
        let request = params
            .get("request")
            .and_then(Value::as_object)
            .ok_or_else(|| HostError::Backend("Fetch request is missing request".into()))?;
        let url = bounded_field(request, "url", MAX_URL_BYTES)?;
        let request_scheme = Url::parse(&url)
            .map_err(|error| {
                HostError::Backend(format!("Fetch request contains invalid URL: {error}"))
            })?
            .scheme()
            .to_owned()
            .into_boxed_str();
        #[cfg(feature = "route-regexp")]
        if self.has_regexp_route(page, context)?
            && !self.has_matching_registration(page, context, &url)?
        {
            #[cfg(feature = "context-http-credentials")]
            if self.http_credentials.enabled(context, _ownership)? {
                self.http_credentials
                    .continue_request(session_id, &request_id, auth_network_id)?;
                return Ok(None);
            }
            return Ok(Some(NativeRouteCall::Continue {
                session_id: session_id.to_owned().into_boxed_str(),
                request_id: request_id.into_boxed_str(),
                options: RouteContinue::default(),
            }));
        }
        if is_favicon_url(&url) {
            return Ok(Some(NativeRouteCall::Abort {
                session_id: session_id.to_owned().into_boxed_str(),
                request_id: request_id.into_boxed_str(),
            }));
        }
        let method = bounded_field(request, "method", MAX_METHOD_BYTES)?;
        let headers = bounded_headers(request.get("headers"))?;
        #[cfg(feature = "extra-headers")]
        let mut headers = headers;
        #[cfg(feature = "extra-headers")]
        extra_headers.augment_route_request_headers(context, page, &mut headers)?;
        if let Some(response) = cors_preflight_fulfill(&method, &headers)?
            && self.has_matching_registration(page, context, &url)?
        {
            return Ok(Some(NativeRouteCall::Fulfill {
                session_id: session_id.to_owned().into_boxed_str(),
                request_id: request_id.into_boxed_str(),
                response,
            }));
        }
        #[cfg(feature = "context-http-credentials")]
        let auth_enabled = self.http_credentials.enabled(context, _ownership)?;
        #[cfg(not(feature = "context-http-credentials"))]
        let auth_enabled = false;
        let registration = self.take_matching_registration(page, context, &url, auth_enabled)?;
        #[cfg(feature = "context-http-credentials")]
        if registration.is_none() && auth_enabled {
            self.http_credentials
                .continue_request(session_id, &request_id, auth_network_id)?;
            return Ok(None);
        }
        let Some(registration) = registration else {
            return Ok(Some(NativeRouteCall::Continue {
                session_id: session_id.to_owned().into_boxed_str(),
                request_id: request_id.into_boxed_str(),
                options: RouteContinue::default(),
            }));
        };
        let registration_id = registration.id;
        let registration_scope = registration.scope;
        let registration_handler = registration.handler;
        #[cfg(feature = "context-storage-state")]
        if matches!(registration_handler, RegistrationHandler::StorageState) {
            return Ok(Some(NativeRouteCall::Fulfill {
                session_id: session_id.to_owned().into_boxed_str(),
                request_id: request_id.into_boxed_str(),
                response: RouteFulfill::new(
                    200,
                    vec![("content-type".into(), "text/html; charset=utf-8".into())],
                    b"<html></html>".to_vec(),
                )?,
            }));
        }
        let expires = registration.expires;
        let fallback_cursor = registration.next_cursor;
        let resource_type = bounded_field(params, "resourceType", MAX_RESOURCE_TYPE_BYTES)?;
        let post_data_source = captured_post_data_source(request)?;
        #[cfg(feature = "large-route-post-data")]
        let large_post_data_slot = self
            .large_post_data_budget
            .acquire(post_data_source.bytes())?;
        let post_data = captured_post_data(post_data_source)?;
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.pending.len() + state.settling.len() == MAX_PENDING_CALLS {
            return Err(HostError::ResourceExhausted(format!(
                "pending Page route capacity exceeded ({MAX_PENDING_CALLS})"
            )));
        }
        let id = state.next_call;
        state.next_call = id
            .checked_add(1)
            .ok_or_else(|| HostError::ResourceExhausted("route call ID space exhausted".into()))?;
        let resource_type = resource_type.to_ascii_lowercase();
        let is_navigation_request = resource_type == "document";
        let mut refresh = RouteRefresh::default();
        if expires {
            refresh.include(registration_scope);
        }
        #[cfg(feature = "network-events")]
        let call_native_request_id = network_request_id.as_deref().map(str::to_owned);
        state.pending.push(PendingCall {
            id,
            context,
            page,
            frame,
            registration_scope,
            fallback_cursor,
            session_id: session_id.to_owned().into_boxed_str(),
            request_id: request_id.into_boxed_str(),
            request_scheme,
            url: url.clone(),
            method: method.clone(),
            resource_type: resource_type.clone(),
            headers: headers.clone(),
            post_data: post_data.clone(),
            #[cfg(feature = "large-route-post-data")]
            large_post_data_slot: large_post_data_slot.clone(),
            is_navigation_request,
            fallback_options: RouteContinue::default(),
            refresh,
            #[cfg(feature = "network-events")]
            network_request_id,
        });
        let call = RouteCall {
            id,
            registration_id,
            context,
            page,
            frame,
            url,
            method,
            resource_type,
            headers,
            post_data,
            is_navigation_request,
            expires,
            #[cfg(feature = "network-events")]
            native_request_id: call_native_request_id,
        };
        match registration_handler {
            RegistrationHandler::Binding => {
                if let Err(error) = self.sender.try_send(QueuedRouteCall {
                    call,
                    #[cfg(feature = "large-route-post-data")]
                    _large_post_data_slot: large_post_data_slot,
                }) {
                    state.pending.retain(|pending| pending.id != id);
                    drop(state);
                    self.changed.notify_waiters();
                    return Err(HostError::ResourceExhausted(format!(
                        "Page route delivery queue is unavailable: {error}"
                    )));
                }
                Ok(None)
            }
            #[cfg(feature = "context-storage-state")]
            RegistrationHandler::StorageState => Err(HostError::Backend(
                "storage-state route escaped internal settlement".into(),
            )),
            #[cfg(feature = "har-replay")]
            RegistrationHandler::Har { archive, not_found } => {
                Ok(Some(NativeRouteCall::Replay(ReplayCall {
                    call,
                    archive,
                    not_found,
                })))
            }
        }
    }

    async fn resolve_native_call(&self, call: NativeRouteCall) -> Result<(), HostError> {
        match call {
            NativeRouteCall::Continue {
                session_id,
                request_id,
                options,
            } => match self
                .connection
                .send_command_with_session(
                    "Fetch.continueRequest",
                    options.protocol_params(request_id.as_ref()),
                    Some(session_id.as_ref()),
                )
                .await
            {
                Ok(_) => Ok(()),
                Err(error) if terminal_route_error(&error) => Ok(()),
                Err(error) => Err(HostError::backend(error)),
            },
            NativeRouteCall::Abort {
                session_id,
                request_id,
            } => match self
                .connection
                .send_command_with_session(
                    "Fetch.failRequest",
                    json!({
                        "requestId": request_id.as_ref(),
                        "errorReason": "Aborted",
                    }),
                    Some(session_id.as_ref()),
                )
                .await
            {
                Ok(_) => Ok(()),
                Err(error) if terminal_route_error(&error) => Ok(()),
                Err(error) => Err(HostError::backend(error)),
            },
            NativeRouteCall::Fulfill {
                session_id,
                request_id,
                response,
            } => match self
                .connection
                .send_command_with_session(
                    "Fetch.fulfillRequest",
                    response.protocol_params(request_id.as_ref()),
                    Some(session_id.as_ref()),
                )
                .await
            {
                Ok(_) => Ok(()),
                Err(error) if terminal_route_error(&error) => Ok(()),
                Err(error) => Err(HostError::backend(error)),
            },
            #[cfg(feature = "har-replay")]
            NativeRouteCall::Replay(replay) => self.replay(replay).await.map(|_| ()),
        }
    }

    #[cfg(feature = "har-replay")]
    async fn replay(&self, replay: ReplayCall) -> Result<Option<RouteSettlement>, HostError> {
        let action = replay
            .archive
            .lookup(
                &replay.call.url,
                &replay.call.method,
                &replay.call.headers,
                replay.call.post_data.as_deref(),
                replay.call.is_navigation_request,
            )
            .map_err(HostError::backend)?;
        let action = match action {
            pwright_har_replay::ReplayAction::Fulfill(response) => {
                let headers = response
                    .headers()
                    .iter()
                    .map(|(name, value)| (name.to_string(), value.to_string()))
                    .collect();
                RouteAction::Fulfill(RouteFulfill::from_shared(
                    response.status(),
                    headers,
                    response.body(),
                )?)
            }
            pwright_har_replay::ReplayAction::Redirect(url) => RouteAction::Continue(
                RouteContinue::new(Some(url.into_string()), None, None, None)?,
            ),
            pwright_har_replay::ReplayAction::Stall => return Ok(None),
            pwright_har_replay::ReplayAction::NoEntry => match replay.not_found {
                HarNotFoundAction::Abort => RouteAction::Abort(RouteAbortReason::Failed),
                HarNotFoundAction::Fallback => {
                    return Box::pin(self.fallback(replay.call.id, RouteContinue::default())).await;
                }
            },
        };
        self.act(replay.call.id, action).await
    }

    async fn next_call(&self, stopped: &CancellationToken) -> Option<RouteCall> {
        let mut receiver = self.receiver.lock().await;
        tokio::select! {
            biased;
            () = stopped.cancelled() => None,
            value = receiver.recv() => value.map(|queued| queued.call),
        }
    }

    fn take_pending(&self, id: u64) -> Result<Option<PendingCall>, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        take_pending_action(&mut state, id)
    }

    fn finish_settling(&self, id: u64) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.settling.retain(|settling| settling.id != id);
        drop(state);
        self.changed.notify_waiters();
    }

    fn restore_pending(&self, pending: PendingCall) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        let settling = state
            .settling
            .iter()
            .position(|settling| settling.id == pending.id)
            .ok_or_else(|| HostError::Backend("route action lost settling ownership".into()))?;
        state.settling.swap_remove(settling);
        if state.pending.len() + state.settling.len() == MAX_PENDING_CALLS {
            return Err(HostError::ResourceExhausted(
                "pending Page route capacity exceeded while restoring an action".into(),
            ));
        }
        state.pending.push(pending);
        drop(state);
        self.changed.notify_waiters();
        Ok(())
    }

    #[cfg(feature = "network-events")]
    fn retain_fulfilled_body(
        &self,
        pending: &PendingCall,
        body: Arc<[u8]>,
    ) -> Result<Option<NativeRequestKey>, HostError> {
        let Some(network_request_id) = &pending.network_request_id else {
            return Ok(None);
        };
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.fulfilled.len() == MAX_FULFILLED_BODIES {
            return Err(HostError::ResourceExhausted(format!(
                "retained fulfilled response body capacity exceeded ({MAX_FULFILLED_BODIES})"
            )));
        }
        if state.fulfilled.iter().any(|value| {
            value.session_id.as_ref() == pending.session_id.as_ref()
                && value.network_request_id.as_ref() == network_request_id.as_ref()
        }) {
            return Err(HostError::Backend(
                "duplicate fulfilled response body correlation".into(),
            ));
        }
        let session_id = pending.session_id.clone();
        let network_request_id = network_request_id.clone();
        state.fulfilled.push(FulfilledBody {
            page: pending.page,
            session_id: session_id.clone(),
            network_request_id: network_request_id.clone(),
            body,
        });
        Ok(Some((session_id, network_request_id)))
    }

    #[cfg(feature = "network-events")]
    fn remove_fulfilled_body(&self, session_id: &str, network_request_id: &str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.fulfilled.retain(|value| {
            value.session_id.as_ref() != session_id
                || value.network_request_id.as_ref() != network_request_id
        });
    }

    #[cfg(feature = "network-events")]
    pub(crate) fn take_fulfilled_body(
        &self,
        session_id: &str,
        network_request_id: &str,
    ) -> Result<Option<Arc<[u8]>>, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        let Some(index) = state.fulfilled.iter().position(|value| {
            value.session_id.as_ref() == session_id
                && value.network_request_id.as_ref() == network_request_id
        }) else {
            return Ok(None);
        };
        Ok(Some(state.fulfilled.swap_remove(index).body))
    }

    fn reconcile(
        &self,
        active_pages: &[ObjectId],
        active_contexts: &[ObjectId],
        active_sessions: &[String],
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        state
            .registrations
            .retain(|registration| registration.scope.is_active(active_pages, active_contexts));
        #[cfg(feature = "har-replay")]
        {
            state.har_bytes = retained_har_bytes(&state.registrations)?;
        }
        reconcile_pending(&mut state, active_pages);
        state
            .settling
            .retain(|settling| active_pages.contains(&settling.page));
        state.initializing_sessions.retain(|entry| {
            !active_sessions
                .iter()
                .any(|session| session == entry.session_id.as_ref())
        });
        #[cfg(feature = "network-events")]
        state
            .fulfilled
            .retain(|fulfilled| active_pages.contains(&fulfilled.page));
        #[cfg(feature = "route-redirects")]
        state
            .redirect_header_overrides
            .retain(|entry| active_pages.contains(&entry.page));
        drop(state);
        self.changed.notify_waiters();
        Ok(())
    }

    async fn act(
        &self,
        id: u64,
        action: RouteAction,
    ) -> Result<Option<RouteSettlement>, HostError> {
        let Some(pending) = self.take_pending(id)? else {
            return Ok(None);
        };
        let settling = SettlingGuard::new(self, id);
        self.settle_pending(pending, action, settling)
            .await
            .map(Some)
    }

    async fn fallback(
        &self,
        id: u64,
        options: RouteContinue,
    ) -> Result<Option<RouteSettlement>, HostError> {
        let Some(mut pending) = self.take_pending(id)? else {
            return Ok(None);
        };
        let mut settling = SettlingGuard::new(self, id);
        let combined = pending.fallback_options.merged(options);
        if let Err(error) = combined.validate_protocol(pending.request_scheme.as_ref()) {
            self.restore_pending(pending)?;
            settling.disarm();
            return Err(error);
        }
        let permit = match self.sender.try_reserve() {
            Ok(permit) => permit,
            Err(error) => {
                self.restore_pending(pending)?;
                settling.disarm();
                return Err(HostError::ResourceExhausted(format!(
                    "Page route delivery queue is unavailable: {error}"
                )));
            }
        };
        let effective_url = combined.effective_url(&pending.url).to_owned();
        let registration = {
            let mut state = self.state.lock().map_err(HostError::backend)?;
            take_matching_registration_after(
                &mut state,
                pending.page,
                pending.context,
                &effective_url,
                pending.fallback_cursor,
            )
        };
        let registration = match registration {
            Ok(value) => value,
            Err(error) => {
                drop(permit);
                self.restore_pending(pending)?;
                settling.disarm();
                return Err(error);
            }
        };
        let Some(registration) = registration else {
            drop(permit);
            pending.fallback_options = combined;
            return self
                .settle_pending(
                    pending,
                    RouteAction::Continue(RouteContinue::default()),
                    settling,
                )
                .await
                .map(Some);
        };
        let dispatch_state = {
            let mut state = self.state.lock().map_err(HostError::backend)?;
            let new_id = state.next_call;
            state.next_call = new_id.checked_add(1).ok_or_else(|| {
                HostError::ResourceExhausted("route call ID space exhausted".into())
            })?;
            let settling_index = state
                .settling
                .iter()
                .position(|call| call.id == id)
                .ok_or_else(|| {
                    HostError::Backend("route fallback lost settling ownership".into())
                })?;
            if registration.expires {
                pending.refresh.include(registration.scope);
            }
            let call = RouteCall {
                id: new_id,
                registration_id: registration.id,
                context: pending.context,
                page: pending.page,
                frame: pending.frame,
                url: effective_url,
                method: combined.effective_method(&pending.method).to_owned(),
                resource_type: pending.resource_type.clone(),
                headers: combined.effective_headers(&pending.headers).to_vec(),
                post_data: combined
                    .effective_post_data(pending.post_data.as_deref())
                    .map(ToOwned::to_owned),
                is_navigation_request: pending.is_navigation_request,
                expires: registration.expires,
                #[cfg(feature = "network-events")]
                native_request_id: pending.network_request_id.as_deref().map(str::to_owned),
            };
            #[cfg(feature = "large-route-post-data")]
            let queued_large_post_data_slot = pending.large_post_data_slot.clone();
            #[cfg(not(feature = "large-route-post-data"))]
            let queued_large_post_data_slot = ();
            let handler = registration.handler;
            pending.id = new_id;
            pending.registration_scope = registration.scope;
            pending.fallback_cursor = registration.next_cursor;
            pending.fallback_options = combined;
            state.settling.swap_remove(settling_index);
            state.pending.push(pending);
            (call, handler, queued_large_post_data_slot)
        };
        #[cfg(feature = "large-route-post-data")]
        let (call, handler, queued_large_post_data_slot) = dispatch_state;
        #[cfg(not(feature = "large-route-post-data"))]
        let (call, handler, _) = dispatch_state;
        settling.disarm();
        self.changed.notify_waiters();
        match handler {
            RegistrationHandler::Binding => {
                permit.send(QueuedRouteCall {
                    call,
                    #[cfg(feature = "large-route-post-data")]
                    _large_post_data_slot: queued_large_post_data_slot,
                });
                Ok(None)
            }
            #[cfg(feature = "context-storage-state")]
            RegistrationHandler::StorageState => Err(HostError::Backend(
                "storage-state route entered public fallback".into(),
            )),
            #[cfg(feature = "har-replay")]
            RegistrationHandler::Har { archive, not_found } => {
                drop(permit);
                self.replay(ReplayCall {
                    call,
                    archive,
                    not_found,
                })
                .await
            }
        }
    }

    async fn settle_pending(
        &self,
        pending: PendingCall,
        action: RouteAction,
        mut settling: SettlingGuard<'_>,
    ) -> Result<RouteSettlement, HostError> {
        let action = match action {
            RouteAction::Continue(options) => {
                let options = pending.fallback_options.merged(options);
                let options = options
                    .validate_protocol(pending.request_scheme.as_ref())
                    .map(|()| options);
                #[cfg(feature = "route-redirects")]
                let options = options
                    .and_then(|options| options.with_original_cookie_header(&pending.headers));
                let options = match options {
                    Ok(options) => options,
                    Err(error) => {
                        self.restore_pending(pending)?;
                        settling.disarm();
                        return Err(error);
                    }
                };
                RouteAction::Continue(options)
            }
            #[cfg(feature = "route-fulfill-cors")]
            RouteAction::Fulfill(fulfill) => {
                let fulfill = match with_default_cors_headers(fulfill, &pending) {
                    Ok(fulfill) => fulfill,
                    Err(error) => {
                        self.restore_pending(pending)?;
                        settling.disarm();
                        return Err(error);
                    }
                };
                RouteAction::Fulfill(fulfill)
            }
            action => action,
        };
        #[cfg(feature = "route-redirects")]
        let retained_redirect_headers = if let RouteAction::Continue(options) = &action {
            if let Some(headers) = options.headers.as_deref() {
                if let Err(error) = self.retain_redirect_header_override(&pending, headers) {
                    self.restore_pending(pending)?;
                    settling.disarm();
                    return Err(error);
                }
                true
            } else {
                false
            }
        } else {
            false
        };
        #[cfg(feature = "network-events")]
        let retained = if let RouteAction::Fulfill(fulfill) = &action {
            match self.retain_fulfilled_body(&pending, Arc::clone(&fulfill.body)) {
                Ok(value) => value,
                Err(error) => {
                    self.restore_pending(pending)?;
                    settling.disarm();
                    return Err(error);
                }
            }
        } else {
            None
        };
        let (method, params) = match action {
            RouteAction::Continue(options) => (
                "Fetch.continueRequest",
                options.protocol_params(pending.request_id.as_ref()),
            ),
            RouteAction::Abort(reason) => (
                "Fetch.failRequest",
                json!({
                    "requestId": pending.request_id.as_ref(),
                    "errorReason": reason.protocol_name(),
                }),
            ),
            RouteAction::Fulfill(fulfill) => (
                "Fetch.fulfillRequest",
                fulfill.protocol_params(pending.request_id.as_ref()),
            ),
        };
        let result = match self
            .connection
            .send_command_with_session(method, params, Some(&pending.session_id))
            .await
        {
            Ok(_) => Ok(()),
            Err(error) if terminal_route_error(&error) => {
                #[cfg(feature = "route-redirects")]
                if retained_redirect_headers {
                    let _ = self.remove_redirect_header_override(
                        pending.session_id.as_ref(),
                        pending.request_id.as_ref(),
                    );
                }
                #[cfg(feature = "network-events")]
                if let Some((session_id, network_request_id)) = &retained {
                    self.remove_fulfilled_body(session_id, network_request_id);
                }
                Ok(())
            }
            Err(error) => {
                #[cfg(feature = "route-redirects")]
                if retained_redirect_headers {
                    let _ = self.remove_redirect_header_override(
                        pending.session_id.as_ref(),
                        pending.request_id.as_ref(),
                    );
                }
                #[cfg(feature = "network-events")]
                if let Some((session_id, network_request_id)) = &retained {
                    self.remove_fulfilled_body(session_id, network_request_id);
                }
                Err(HostError::backend(error))
            }
        };
        drop(settling);
        result?;
        Ok(RouteSettlement {
            page: pending.page,
            context: pending.context,
            refresh: pending.refresh,
        })
    }
}

struct SettlingGuard<'a> {
    installation: &'a Installation,
    id: u64,
    active: bool,
}

impl<'a> SettlingGuard<'a> {
    fn new(installation: &'a Installation, id: u64) -> Self {
        Self {
            installation,
            id,
            active: true,
        }
    }

    fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for SettlingGuard<'_> {
    fn drop(&mut self) {
        if self.active {
            self.installation.finish_settling(self.id);
        }
    }
}

#[cfg(feature = "har-replay")]
fn retained_har_bytes(registrations: &[Registration]) -> Result<usize, HostError> {
    registrations
        .iter()
        .filter_map(|registration| match &registration.handler {
            RegistrationHandler::Har { archive, .. } => Some(archive.retained_bytes()),
            RegistrationHandler::Binding => None,
            #[cfg(feature = "context-storage-state")]
            RegistrationHandler::StorageState => None,
        })
        .try_fold(0usize, |total, bytes| {
            total.checked_add(bytes).ok_or_else(|| {
                HostError::ResourceExhausted("HAR replay retained-byte count overflow".into())
            })
        })
}

fn bounded_field(
    object: &Map<String, Value>,
    field: &str,
    maximum: usize,
) -> Result<String, HostError> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| HostError::Backend(format!("Fetch request is missing {field}")))?;
    if value.len() > maximum {
        return Err(HostError::ResourceExhausted(format!(
            "Fetch {field} exceeds byte limit"
        )));
    }
    Ok(value.to_owned())
}

fn bounded_headers(value: Option<&Value>) -> Result<Vec<(String, String)>, HostError> {
    let Some(object) = value.and_then(Value::as_object) else {
        return Ok(Vec::new());
    };
    let mut bytes = 0usize;
    let mut headers = Vec::new();
    headers
        .try_reserve_exact(object.len())
        .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
    for (name, value) in object {
        let value = value
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string());
        bytes = bytes
            .checked_add(name.len())
            .and_then(|total| total.checked_add(value.len()))
            .ok_or_else(|| HostError::ResourceExhausted("route header size overflow".into()))?;
        if bytes > MAX_HEADER_BYTES {
            return Err(HostError::ResourceExhausted(
                "route headers exceed byte limit".into(),
            ));
        }
        headers.push((name.to_ascii_lowercase(), value));
    }
    Ok(headers)
}

fn cdp_pattern(pattern: &str) -> Result<String, HostError> {
    let mut converted_bytes = pattern
        .len()
        .checked_add(usize::from(pattern.starts_with('/')))
        .ok_or_else(|| HostError::ResourceExhausted("route pattern size overflow".into()))?;
    let mut consecutive_backslashes = 0usize;
    for character in pattern.chars() {
        if character == '?' {
            if consecutive_backslashes.is_multiple_of(2) {
                converted_bytes = converted_bytes.checked_add(1).ok_or_else(|| {
                    HostError::ResourceExhausted("route pattern size overflow".into())
                })?;
            }
            consecutive_backslashes = 0;
        } else if character == '\\' {
            consecutive_backslashes += 1;
        } else {
            consecutive_backslashes = 0;
        }
    }
    if converted_bytes > MAX_CDP_PATTERN_BYTES {
        return Err(HostError::ResourceExhausted(
            "compiled CDP route pattern exceeds byte limit".into(),
        ));
    }
    let mut converted = String::new();
    converted
        .try_reserve_exact(converted_bytes)
        .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
    if pattern.starts_with('/') {
        converted.push('*');
    }
    consecutive_backslashes = 0;
    for character in pattern.chars() {
        if character == '?' {
            if consecutive_backslashes.is_multiple_of(2) {
                converted.push('\\');
            }
            converted.push('?');
            consecutive_backslashes = 0;
        } else {
            converted.push(character);
            if character == '\\' {
                consecutive_backslashes += 1;
            } else {
                consecutive_backslashes = 0;
            }
        }
    }
    debug_assert_eq!(converted.len(), converted_bytes);
    Ok(converted)
}

fn route_matches(pattern: &str, url: &str) -> Result<bool, HostError> {
    let candidate = if pattern.starts_with('/') {
        if let Ok(parsed) = Url::parse(url) {
            let query_bytes = parsed.query().map_or(0, |query| 1 + query.len());
            let capacity = parsed
                .path()
                .len()
                .checked_add(query_bytes)
                .ok_or_else(|| HostError::ResourceExhausted("route URL size overflow".into()))?;
            let mut value = String::new();
            value
                .try_reserve_exact(capacity)
                .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
            value.push_str(parsed.path());
            if let Some(query) = parsed.query() {
                value.push('?');
                value.push_str(query);
            }
            Cow::Owned(value)
        } else {
            Cow::Borrowed(url)
        }
    } else {
        Cow::Borrowed(url)
    };
    wildcard_matches(pattern.as_bytes(), candidate.as_bytes())
}

fn wildcard_matches(pattern: &[u8], value: &[u8]) -> Result<bool, HostError> {
    let (mut pattern_index, mut value_index) = (0, 0);
    let mut stars = Vec::<(usize, usize, bool)>::new();
    stars
        .try_reserve_exact(pattern.iter().filter(|byte| **byte == b'*').count())
        .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
    while value_index < value.len() {
        if pattern_index < pattern.len() && pattern[pattern_index] == b'\\' {
            if pattern_index + 1 < pattern.len() && pattern[pattern_index + 1] == value[value_index]
            {
                pattern_index += 2;
                value_index += 1;
                continue;
            }
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            let double = pattern.get(pattern_index + 1) == Some(&b'*');
            pattern_index += if double { 2 } else { 1 };
            stars.push((pattern_index, value_index, double));
            continue;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == value[value_index] {
            pattern_index += 1;
            value_index += 1;
            continue;
        }
        let mut resumed = false;
        while let Some((after_star, matched, double)) = stars.last_mut() {
            if *matched < value.len() && (*double || value[*matched] != b'/') {
                *matched += 1;
                pattern_index = *after_star;
                value_index = *matched;
                resumed = true;
                break;
            }
            stars.pop();
        }
        if !resumed {
            return Ok(false);
        }
    }
    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }
    Ok(pattern_index == pattern.len())
}

fn terminal_route_error(error: &CdpError) -> bool {
    match error {
        CdpError::Closed | CdpError::SessionDetached | CdpError::PageClosed => true,
        CdpError::Protocol { message, .. } => {
            message.contains("Invalid InterceptionId")
                || message.contains("Target closed")
                || message.contains("Session closed")
                || message.contains("Session with given id not found")
        }
        _ => false,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteFulfill {
    status: i64,
    headers: Vec<(String, String)>,
    body: Arc<[u8]>,
}

impl RouteFulfill {
    pub fn new(
        status: i64,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<Self, HostError> {
        Self::from_shared(status, headers, body.into())
    }

    fn from_shared(
        status: i64,
        headers: Vec<(String, String)>,
        body: Arc<[u8]>,
    ) -> Result<Self, HostError> {
        if !(100..=999).contains(&status) {
            return Err(HostError::InvalidArgument(
                "Route.fulfill status must be between 100 and 999".into(),
            ));
        }
        validate_route_headers("Route.fulfill", &headers)?;
        if body.len() > MAX_ROUTE_FULFILL_BODY_BYTES {
            return Err(HostError::InvalidArgument(format!(
                "Route.fulfill body exceeds byte limit ({MAX_ROUTE_FULFILL_BODY_BYTES})"
            )));
        }
        Ok(Self {
            status,
            headers,
            body,
        })
    }

    #[cfg(feature = "route-fulfill-path")]
    pub async fn from_path(
        status: i64,
        mut headers: Vec<(String, String)>,
        path: &str,
    ) -> Result<Self, HostError> {
        if path.is_empty() || path.len() > MAX_ROUTE_FULFILL_PATH_BYTES {
            return Err(HostError::InvalidArgument(format!(
                "Route.fulfill path must contain 1..={MAX_ROUTE_FULFILL_PATH_BYTES} UTF-8 bytes"
            )));
        }
        let file = tokio::fs::File::open(path).await.map_err(|error| {
            HostError::InvalidArgument(format!("Route.fulfill cannot open path: {error}"))
        })?;
        let metadata = file.metadata().await.map_err(HostError::backend)?;
        if !metadata.is_file() {
            return Err(HostError::InvalidArgument(
                "Route.fulfill path must identify a regular file".into(),
            ));
        }
        let mut body = Vec::new();
        body.try_reserve_exact(MAX_ROUTE_FULFILL_BODY_BYTES.min(64 * 1024))
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        file.take((MAX_ROUTE_FULFILL_BODY_BYTES as u64) + 1)
            .read_to_end(&mut body)
            .await
            .map_err(HostError::backend)?;
        if body.len() > MAX_ROUTE_FULFILL_BODY_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "Route.fulfill file exceeds {MAX_ROUTE_FULFILL_BODY_BYTES} bytes"
            )));
        }
        headers.retain(|(name, _)| !name.eq_ignore_ascii_case("content-type"));
        headers.push((
            "content-type".into(),
            mime_guess::from_path(path)
                .first_or_octet_stream()
                .essence_str()
                .into(),
        ));
        Self::new(status, headers, body)
    }

    fn protocol_params(self, request_id: &str) -> Value {
        json!({
            "requestId": request_id,
            "responseCode": self.status,
            "responsePhrase": playwright_status_text(self.status),
            "responseHeaders": self.headers.into_iter().map(|(name, value)| {
                json!({ "name": name, "value": value })
            }).collect::<Vec<_>>(),
            "body": base64::engine::general_purpose::STANDARD.encode(self.body.as_ref()),
        })
    }
}

#[cfg(feature = "route-fulfill-cors")]
fn with_default_cors_headers(
    mut fulfill: RouteFulfill,
    pending: &PendingCall,
) -> Result<RouteFulfill, HostError> {
    let request_headers = pending.fallback_options.effective_headers(&pending.headers);
    let Some(origin) = request_headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("origin"))
        .map(|(_, value)| value.as_str())
        .filter(|origin| !origin.is_empty())
    else {
        return Ok(fulfill);
    };
    let request_url = pending.fallback_options.effective_url(&pending.url);
    let request_url = Url::parse(request_url).map_err(|error| {
        HostError::InvalidArgument(format!("invalid route URL for CORS fulfillment: {error}"))
    })?;
    if !matches!(request_url.scheme(), "http" | "https")
        || request_url.origin().ascii_serialization() == origin.trim()
        || fulfill
            .headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("access-control-allow-origin"))
    {
        return Ok(fulfill);
    }
    fulfill.headers.try_reserve(3).map_err(|error| {
        HostError::ResourceExhausted(format!(
            "Route.fulfill CORS header allocation failed: {error}"
        ))
    })?;
    fulfill
        .headers
        .push(("access-control-allow-origin".into(), origin.into()));
    fulfill
        .headers
        .push(("access-control-allow-credentials".into(), "true".into()));
    fulfill.headers.push(("vary".into(), "Origin".into()));
    validate_route_headers("Route.fulfill", &fulfill.headers)?;
    Ok(fulfill)
}

fn cors_preflight_fulfill(
    method: &str,
    request_headers: &[(String, String)],
) -> Result<Option<RouteFulfill>, HostError> {
    if !method.eq_ignore_ascii_case("OPTIONS") {
        return Ok(None);
    }
    let Some(request_method) = request_headers
        .iter()
        .find(|(name, _)| name == "access-control-request-method")
        .map(|(_, value)| value.as_str())
    else {
        return Ok(None);
    };
    let origin = request_headers
        .iter()
        .find(|(name, _)| name == "origin")
        .map_or("*", |(_, value)| value.as_str());
    let requested_headers = request_headers
        .iter()
        .find(|(name, _)| name == "access-control-request-headers")
        .map(|(_, value)| value.as_str());
    let mut response_headers = vec![
        ("Access-Control-Allow-Origin".into(), origin.into()),
        ("Access-Control-Allow-Methods".into(), request_method.into()),
        ("Access-Control-Allow-Credentials".into(), "true".into()),
    ];
    if let Some(requested_headers) = requested_headers {
        response_headers.push((
            "Access-Control-Allow-Headers".into(),
            requested_headers.into(),
        ));
    }
    RouteFulfill::new(204, response_headers, Vec::new()).map(Some)
}

fn playwright_status_text(status: i64) -> &'static str {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        102 => "Processing",
        103 => "Early Hints",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "Non-Authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        207 => "Multi-Status",
        208 => "Already Reported",
        226 => "IM Used",
        300 => "Multiple Choices",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        305 => "Use Proxy",
        306 => "Switch Proxy",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        407 => "Proxy Authentication Required",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        412 => "Precondition Failed",
        413 => "Payload Too Large",
        414 => "URI Too Long",
        415 => "Unsupported Media Type",
        416 => "Range Not Satisfiable",
        417 => "Expectation Failed",
        418 => "I'm a teapot",
        421 => "Misdirected Request",
        422 => "Unprocessable Entity",
        423 => "Locked",
        424 => "Failed Dependency",
        425 => "Too Early",
        426 => "Upgrade Required",
        428 => "Precondition Required",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        451 => "Unavailable For Legal Reasons",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "HTTP Version Not Supported",
        506 => "Variant Also Negotiates",
        507 => "Insufficient Storage",
        508 => "Loop Detected",
        510 => "Not Extended",
        511 => "Network Authentication Required",
        _ => "Unknown",
    }
}

fn validate_route_headers(action: &str, headers: &[(String, String)]) -> Result<(), HostError> {
    if headers.len() > MAX_HEADER_COUNT {
        return Err(HostError::InvalidArgument(format!(
            "{action} headers exceed count limit ({MAX_HEADER_COUNT})"
        )));
    }
    let mut header_bytes = 0usize;
    for (name, value) in headers {
        if name.is_empty() || !name.bytes().all(valid_header_name_byte) {
            return Err(HostError::InvalidArgument(format!(
                "{action} contains an invalid header name"
            )));
        }
        if value
            .bytes()
            .any(|byte| matches!(byte, b'\0' | b'\r' | b'\n'))
        {
            return Err(HostError::InvalidArgument(format!(
                "{action} contains an invalid header value"
            )));
        }
        header_bytes = header_bytes
            .checked_add(name.len())
            .and_then(|total| total.checked_add(value.len()))
            .ok_or_else(|| {
                HostError::ResourceExhausted(format!("{action} header size overflow"))
            })?;
    }
    if header_bytes > MAX_HEADER_BYTES {
        return Err(HostError::InvalidArgument(format!(
            "{action} headers exceed byte limit ({MAX_HEADER_BYTES})"
        )));
    }
    Ok(())
}

fn valid_header_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RouteContinue {
    url: Option<String>,
    method: Option<String>,
    headers: Option<Vec<(String, String)>>,
    post_data: Option<Vec<u8>>,
}

impl RouteContinue {
    pub fn new(
        url: Option<String>,
        method: Option<String>,
        headers: Option<Vec<(String, String)>>,
        post_data: Option<Vec<u8>>,
    ) -> Result<Self, HostError> {
        if url
            .as_ref()
            .is_some_and(|value| value.len() > MAX_URL_BYTES)
        {
            return Err(HostError::InvalidArgument(format!(
                "Route.continue_ URL exceeds byte limit ({MAX_URL_BYTES})"
            )));
        }
        if method
            .as_ref()
            .is_some_and(|value| value.len() > MAX_METHOD_BYTES)
        {
            return Err(HostError::InvalidArgument(format!(
                "Route.continue_ method exceeds byte limit ({MAX_METHOD_BYTES})"
            )));
        }
        if let Some(headers) = &headers {
            validate_route_headers("Route.continue_", headers)?;
        }
        if post_data
            .as_ref()
            .is_some_and(|value| value.len() > MAX_POST_DATA_BYTES)
        {
            return Err(HostError::InvalidArgument(format!(
                "Route.continue_ POST data exceeds byte limit ({MAX_POST_DATA_BYTES})"
            )));
        }
        Ok(Self {
            url,
            method,
            headers,
            post_data,
        })
    }

    fn merged(&self, newer: Self) -> Self {
        Self {
            url: newer.url.or_else(|| self.url.clone()),
            method: newer.method.or_else(|| self.method.clone()),
            headers: newer.headers.or_else(|| self.headers.clone()),
            post_data: newer.post_data.or_else(|| self.post_data.clone()),
        }
    }

    #[cfg(feature = "route-redirects")]
    fn with_original_cookie_header(
        mut self,
        request_headers: &[(String, String)],
    ) -> Result<Self, HostError> {
        let Some(headers) = &mut self.headers else {
            return Ok(self);
        };
        // Chromium's Cookie header belongs to the browser cookie jar. A route
        // may replace other headers, but must not replace or inject Cookie.
        headers.retain(|(name, _)| !name.eq_ignore_ascii_case("cookie"));
        headers.extend(
            request_headers
                .iter()
                .filter(|(name, _)| name.eq_ignore_ascii_case("cookie"))
                .cloned(),
        );
        validate_route_headers("Route.continue_", headers)?;
        Ok(self)
    }

    fn effective_url<'a>(&'a self, original: &'a str) -> &'a str {
        self.url.as_deref().unwrap_or(original)
    }

    fn effective_method<'a>(&'a self, original: &'a str) -> &'a str {
        self.method.as_deref().unwrap_or(original)
    }

    fn effective_headers<'a>(&'a self, original: &'a [(String, String)]) -> &'a [(String, String)] {
        self.headers.as_deref().unwrap_or(original)
    }

    fn effective_post_data<'a>(&'a self, original: Option<&'a [u8]>) -> Option<&'a [u8]> {
        self.post_data.as_deref().or(original)
    }

    fn validate_protocol(&self, request_scheme: &str) -> Result<(), HostError> {
        let Some(url) = &self.url else {
            return Ok(());
        };
        let override_scheme = Url::parse(url)
            .map_err(|error| HostError::InvalidArgument(format!("invalid override URL: {error}")))?
            .scheme()
            .to_owned();
        if request_scheme != override_scheme {
            return Err(HostError::InvalidArgument(
                "New URL must have same protocol as overridden URL".into(),
            ));
        }
        Ok(())
    }

    fn protocol_params(self, request_id: &str) -> Value {
        let mut params = Map::new();
        params.insert("requestId".into(), Value::String(request_id.to_owned()));
        if let Some(url) = self.url {
            params.insert("url".into(), Value::String(url));
        }
        if let Some(method) = self.method {
            params.insert("method".into(), Value::String(method));
        }
        if let Some(headers) = self.headers {
            params.insert(
                "headers".into(),
                Value::Array(
                    headers
                        .into_iter()
                        .map(|(name, value)| json!({ "name": name, "value": value }))
                        .collect(),
                ),
            );
        }
        if let Some(post_data) = self.post_data {
            params.insert(
                "postData".into(),
                Value::String(base64::engine::general_purpose::STANDARD.encode(post_data)),
            );
        }
        Value::Object(params)
    }
}

#[derive(Clone)]
pub enum RouteAction {
    Continue(RouteContinue),
    Abort(RouteAbortReason),
    Fulfill(RouteFulfill),
}

#[cfg(feature = "har-replay")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HarNotFoundAction {
    Abort,
    Fallback,
}

#[cfg(feature = "har-replay")]
impl HarNotFoundAction {
    pub fn parse(value: Option<&str>) -> Result<Self, HostError> {
        match value.unwrap_or("abort") {
            "abort" => Ok(Self::Abort),
            "fallback" => Ok(Self::Fallback),
            value => Err(HostError::InvalidArgument(format!(
                "unsupported route_from_har not_found value {value:?}"
            ))),
        }
    }
}

#[derive(Clone, Copy)]
pub enum RouteAbortReason {
    Failed,
    Aborted,
    TimedOut,
    AccessDenied,
    ConnectionClosed,
    ConnectionReset,
    ConnectionRefused,
    ConnectionAborted,
    ConnectionFailed,
    NameNotResolved,
    InternetDisconnected,
    AddressUnreachable,
    BlockedByClient,
    BlockedByResponse,
}

impl RouteAbortReason {
    pub fn parse(value: Option<&str>) -> Result<Self, HostError> {
        match value.unwrap_or("failed").to_ascii_lowercase().as_str() {
            "failed" => Ok(Self::Failed),
            "aborted" => Ok(Self::Aborted),
            "timedout" => Ok(Self::TimedOut),
            "accessdenied" => Ok(Self::AccessDenied),
            "connectionclosed" => Ok(Self::ConnectionClosed),
            "connectionreset" => Ok(Self::ConnectionReset),
            "connectionrefused" => Ok(Self::ConnectionRefused),
            "connectionaborted" => Ok(Self::ConnectionAborted),
            "connectionfailed" => Ok(Self::ConnectionFailed),
            "namenotresolved" => Ok(Self::NameNotResolved),
            "internetdisconnected" => Ok(Self::InternetDisconnected),
            "addressunreachable" => Ok(Self::AddressUnreachable),
            "blockedbyclient" => Ok(Self::BlockedByClient),
            "blockedbyresponse" => Ok(Self::BlockedByResponse),
            _ => Err(HostError::InvalidArgument(format!(
                "unsupported route abort error code {value:?}"
            ))),
        }
    }

    const fn protocol_name(self) -> &'static str {
        match self {
            Self::Failed => "Failed",
            Self::Aborted => "Aborted",
            Self::TimedOut => "TimedOut",
            Self::AccessDenied => "AccessDenied",
            Self::ConnectionClosed => "ConnectionClosed",
            Self::ConnectionReset => "ConnectionReset",
            Self::ConnectionRefused => "ConnectionRefused",
            Self::ConnectionAborted => "ConnectionAborted",
            Self::ConnectionFailed => "ConnectionFailed",
            Self::NameNotResolved => "NameNotResolved",
            Self::InternetDisconnected => "InternetDisconnected",
            Self::AddressUnreachable => "AddressUnreachable",
            Self::BlockedByClient => "BlockedByClient",
            Self::BlockedByResponse => "BlockedByResponse",
        }
    }
}

/// The selected Fetch domain must reach the ordered writer before a paused
/// popup resumes. Its response remains owned until Chrome acknowledges it.
async fn fenced_fetch_enable<Command, Queue, Settle, Response>(
    fence: &ChromiumSessionResumeFence,
    queue: Queue,
    settle: Settle,
) -> Result<(), CdpError>
where
    Queue: Future<Output = Result<Command, CdpError>>,
    Settle: FnOnce(Command) -> Response,
    Response: Future<Output = Result<(), CdpError>>,
{
    fence.require_enqueue();
    let command = queue.await?;
    fence.command_enqueued();
    settle(command).await
}

pub(crate) struct Initializer<Downstream> {
    downstream: Downstream,
    installation: Arc<Installation>,
}

impl<Downstream> Initializer<Downstream> {
    pub(crate) fn new(downstream: Downstream, installation: Arc<Installation>) -> Self {
        Self {
            downstream,
            installation,
        }
    }
}

impl<Downstream> ChromiumSessionInitializer for Initializer<Downstream>
where
    Downstream: ChromiumSessionInitializer,
{
    fn initialize<'a>(
        &'a self,
        session_id: &'a str,
        target_id: &'a str,
        ownership: pwright_backend_cdp_topology::CdpContextOwnership,
        is_page: bool,
    ) -> ChromiumSessionInitializationFuture<'a> {
        self.downstream
            .initialize(session_id, target_id, ownership, is_page)
    }

    fn initialize_scoped<'a>(
        &'a self,
        target: ChromiumSessionInitializationTarget<'a>,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            let routing_enabled = self.installation.has_routes(target.page, target.context)?;
            #[cfg(feature = "context-http-credentials")]
            let auth_enabled = self
                .installation
                .http_credentials
                .enabled(target.context, target.ownership)?;
            #[cfg(not(feature = "context-http-credentials"))]
            let auth_enabled = false;
            if routing_enabled || auth_enabled {
                self.installation.remember_initializing_session(
                    target.session_id,
                    SessionScope {
                        page: target.page,
                        context: target.context,
                        ownership: target.ownership,
                    },
                )?;
            }
            #[cfg(feature = "context-http-credentials")]
            if auth_enabled {
                self.installation
                    .http_credentials
                    .remember_initializing_session(
                        target.session_id,
                        crate::context_http_credentials::SessionScope {
                            page: target.page,
                            context: target.context,
                            ownership: target.ownership,
                        },
                    )?;
            }
            let downstream = self.downstream.initialize_scoped(target);
            let update = async {
                self.installation
                    .update_session_with_resume_fence(
                        target.session_id,
                        target.page,
                        target.context,
                        auth_enabled,
                        target.resume_fence,
                    )
                    .await
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
            };
            let result = tokio::try_join!(downstream, update).map(|_| ());
            if result.is_err() {
                self.installation
                    .retire_initializing_session(target.session_id)?;
            }
            result
        })
    }
}

pub(crate) struct Observer {
    installation: Arc<Installation>,
    downstream: Arc<dyn ChromiumFrameEventObserver>,
}

pub(crate) struct EventDriver {
    installation: Arc<Installation>,
    #[cfg(feature = "extra-headers")]
    extra_headers: Arc<crate::extra_headers::Installation>,
    topology: SharedChromiumTopology,
    receiver: CdpEventReceiver,
}

impl EventDriver {
    pub(crate) fn new(
        installation: Arc<Installation>,
        #[cfg(feature = "extra-headers")] extra_headers: Arc<crate::extra_headers::Installation>,
        topology: SharedChromiumTopology,
        connection: &CdpConnection,
    ) -> Result<Self, HostError> {
        Ok(Self {
            installation,
            #[cfg(feature = "extra-headers")]
            extra_headers,
            topology,
            receiver: connection
                .subscribe_composable_events()
                .map_err(HostError::backend)?,
        })
    }

    pub(crate) async fn run(&mut self, stopped: &CancellationToken) -> Result<(), HostError> {
        loop {
            let event = tokio::select! {
                biased;
                () = stopped.cancelled() => return Ok(()),
                event = self.receiver.recv() => event,
            };
            let event = match event {
                Ok(event) => event,
                Err(CdpEventRecvError::Closed) => return Err(HostError::TargetClosed),
                Err(CdpEventRecvError::Lagged(skipped)) => {
                    return Err(HostError::Backend(format!(
                        "Page route event lane lagged by {skipped} events"
                    )));
                }
            };
            let native_call = {
                let topology = self.topology.lock().map_err(HostError::backend)?;
                #[cfg(feature = "context-http-credentials")]
                self.installation
                    .http_credentials
                    .observe_event(topology.topology(), &event)
                    .map_err(HostError::backend)?;
                self.installation.observe_event(
                    topology.topology(),
                    &event,
                    #[cfg(feature = "extra-headers")]
                    &self.extra_headers,
                )?
            };
            if let Some(call) = native_call {
                self.installation.resolve_native_call(call).await?;
            }
            #[cfg(feature = "context-http-credentials")]
            self.installation.http_credentials.finish_next().await?;
        }
    }
}

impl Observer {
    pub(crate) fn new(
        installation: Arc<Installation>,
        downstream: Arc<dyn ChromiumFrameEventObserver>,
    ) -> Self {
        Self {
            installation,
            downstream,
        }
    }
}

impl ChromiumFrameEventObserver for Observer {
    fn observes_page(&self, page: ObjectId) -> bool {
        self.downstream.observes_page(page)
    }

    fn prepare_page(&self, topology: &CdpTopology, page: ObjectId) -> FrameObservationResult {
        self.downstream.prepare_page(topology, page)
    }

    fn discard_prepared_page(
        &self,
        topology: &CdpTopology,
        page: ObjectId,
    ) -> FrameObservationResult {
        self.downstream.discard_prepared_page(topology, page)
    }

    fn initialize_frame(
        &self,
        topology: &CdpTopology,
        session: &str,
        frame: &page::Frame,
    ) -> FrameObservationResult {
        self.downstream.initialize_frame(topology, session, frame)
    }

    fn reconcile(&self, topology: &CdpTopology) -> FrameObservationResult {
        self.downstream.reconcile(topology)?;
        let active_pages = topology
            .page_session_routes()
            .map(|route| route.object_id())
            .collect::<Vec<_>>();
        let active_contexts = topology
            .browser_context_routes()
            .map(|route| route.object_id())
            .collect::<Vec<_>>();
        let active_sessions = topology
            .target_session_routes()
            .map(|route| route.session_id().to_owned())
            .collect::<Vec<_>>();
        self.installation
            .reconcile(&active_pages, &active_contexts, &active_sessions)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn event(&self, topology: &CdpTopology, event: &CdpEvent) -> FrameObservationResult {
        self.downstream.event(topology, event)
    }

    fn seal(&self) {
        self.downstream.seal();
    }
}

fn invalid(field: &str) -> HostError {
    HostError::Backend(format!("Fetch request event is missing {field}"))
}

fn is_favicon_url(url: &str) -> bool {
    url.ends_with("/favicon.ico")
}

impl HostSession {
    #[cfg(feature = "context-storage-state")]
    pub(crate) async fn install_storage_state_route(
        &self,
        page_id: &str,
    ) -> Result<u64, HostError> {
        self.install_page_route(
            page_id,
            "**/*".into(),
            RegistrationHandler::StorageState,
            None,
        )
        .await
    }

    pub async fn page_route(
        &self,
        page_id: &str,
        pattern: String,
        times: Option<u64>,
    ) -> Result<u64, HostError> {
        self.install_page_route(page_id, pattern, RegistrationHandler::Binding, times)
            .await
    }

    #[cfg(feature = "route-regexp")]
    pub async fn page_route_regexp(
        &self,
        page_id: &str,
        source: String,
        flags: String,
        times: Option<u64>,
    ) -> Result<u64, HostError> {
        let pattern = RoutePattern::regexp(source, flags)?;
        self.install_page_route_matching(page_id, pattern, RegistrationHandler::Binding, times)
            .await
    }

    pub async fn context_route(
        &self,
        context_id: &str,
        pattern: String,
        times: Option<u64>,
    ) -> Result<u64, HostError> {
        self.install_context_route(context_id, pattern, RegistrationHandler::Binding, times)
            .await
    }

    #[cfg(feature = "har-replay")]
    pub async fn page_route_from_har(
        &self,
        page_id: &str,
        path: &str,
        pattern: Option<String>,
        not_found: HarNotFoundAction,
    ) -> Result<u64, HostError> {
        let bytes = read_har_file(path).await?;
        let archive = pwright_har_replay::ReplayArchive::from_json(
            &bytes,
            pwright_har_replay::ReplayLimits::default(),
        )
        .map_err(HostError::backend)?;
        self.install_page_route(
            page_id,
            pattern.unwrap_or_else(|| "**/*".into()),
            RegistrationHandler::Har {
                archive: Arc::new(archive),
                not_found,
            },
            None,
        )
        .await
    }

    #[cfg(feature = "har-replay")]
    pub async fn context_route_from_har(
        &self,
        context_id: &str,
        path: &str,
        pattern: Option<String>,
        not_found: HarNotFoundAction,
    ) -> Result<u64, HostError> {
        let bytes = read_har_file(path).await?;
        let archive = pwright_har_replay::ReplayArchive::from_json(
            &bytes,
            pwright_har_replay::ReplayLimits::default(),
        )
        .map_err(HostError::backend)?;
        self.install_context_route(
            context_id,
            pattern.unwrap_or_else(|| "**/*".into()),
            RegistrationHandler::Har {
                archive: Arc::new(archive),
                not_found,
            },
            None,
        )
        .await
    }

    async fn install_page_route(
        &self,
        page_id: &str,
        mut pattern: String,
        handler: RegistrationHandler,
        times: Option<u64>,
    ) -> Result<u64, HostError> {
        if !pattern
            .bytes()
            .any(|byte| matches!(byte, b'*' | b'?' | b'['))
        {
            pattern = self.page_context_snapshot(page_id)?.resolve_url(&pattern)?;
        }
        self.install_page_route_matching(page_id, pattern.into(), handler, times)
            .await
    }

    async fn install_page_route_matching(
        &self,
        page_id: &str,
        pattern: RoutePattern,
        handler: RegistrationHandler,
        times: Option<u64>,
    ) -> Result<u64, HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        let (installation, context, sessions, auth_enabled) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            let route = topology
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .ok_or(HostError::TargetClosed)?;
            let target = route.target_id().to_owned();
            let context = route.browser_context_object_id();
            let sessions = topology
                .target_session_routes()
                .filter(|route| route.page_target_id() == target)
                .map(|route| route.session_id().to_owned())
                .collect::<Vec<_>>();
            #[cfg(feature = "context-http-credentials")]
            let auth_enabled = {
                let ownership = topology
                    .browser_context_routes()
                    .find(|route| route.object_id() == context)
                    .map(|route| route.ownership())
                    .ok_or(HostError::TargetClosed)?;
                live.context_http_credentials.enabled(context, ownership)?
            };
            #[cfg(not(feature = "context-http-credentials"))]
            let auth_enabled = false;
            (live.page_routes.clone(), context, sessions, auth_enabled)
        };
        let registration = installation.add_registration_with_handler(
            RegistrationScope::Page(page),
            pattern,
            handler,
            times,
        )?;
        for session in sessions {
            if let Err(error) = installation
                .update_session(&session, page, context, auth_enabled)
                .await
            {
                installation.remove_registration(registration)?;
                return Err(error);
            }
        }
        Ok(registration)
    }

    async fn install_context_route(
        &self,
        context_id: &str,
        mut pattern: String,
        handler: RegistrationHandler,
        times: Option<u64>,
    ) -> Result<u64, HostError> {
        if !pattern
            .bytes()
            .any(|byte| matches!(byte, b'*' | b'?' | b'['))
        {
            pattern = self.context_snapshot(context_id)?.resolve_url(&pattern)?;
        }
        let context = context_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Context ID".into()))?;
        self.require_visible_context_id(context)?;
        let (installation, sessions, auth_enabled) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            let ownership = topology
                .browser_context_routes()
                .find(|route| route.object_id() == context)
                .map(|route| route.ownership())
                .ok_or(HostError::TargetClosed)?;
            let pages = topology
                .page_session_routes()
                .filter(|route| route.browser_context_object_id() == context)
                .map(|route| (route.object_id(), route.target_id().to_owned()))
                .collect::<Vec<_>>();
            let sessions = pages
                .iter()
                .flat_map(|(page, target)| {
                    topology
                        .target_session_routes()
                        .filter(move |route| route.page_target_id() == target)
                        .map(move |route| (*page, route.session_id().to_owned()))
                })
                .collect::<Vec<_>>();
            #[cfg(feature = "context-http-credentials")]
            let auth_enabled = live.context_http_credentials.enabled(context, ownership)?;
            #[cfg(not(feature = "context-http-credentials"))]
            let auth_enabled = {
                let _ = ownership;
                false
            };
            (live.page_routes.clone(), sessions, auth_enabled)
        };
        let registration = installation.add_registration_with_handler(
            RegistrationScope::Context(context),
            pattern.into(),
            handler,
            times,
        )?;
        for (page, session) in sessions {
            if let Err(error) = installation
                .update_session(&session, page, context, auth_enabled)
                .await
            {
                installation.remove_registration(registration)?;
                return Err(error);
            }
        }
        Ok(registration)
    }

    pub async fn page_unroute(
        &self,
        page_id: &str,
        registration_ids: &[u64],
    ) -> Result<(), HostError> {
        self.page_unroute_impl(page_id, Some(registration_ids), false)
            .await
    }

    pub async fn page_unroute_all(&self, page_id: &str, wait: bool) -> Result<(), HostError> {
        self.page_unroute_impl(page_id, None, wait).await
    }

    async fn page_unroute_impl(
        &self,
        page_id: &str,
        registration_ids: Option<&[u64]>,
        wait: bool,
    ) -> Result<(), HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        let (installation, context, sessions, auth_enabled) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            let route = topology
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .ok_or(HostError::TargetClosed)?;
            let target = route.target_id().to_owned();
            let context = route.browser_context_object_id();
            let sessions = topology
                .target_session_routes()
                .filter(|route| route.page_target_id() == target)
                .map(|route| route.session_id().to_owned())
                .collect::<Vec<_>>();
            #[cfg(feature = "context-http-credentials")]
            let auth_enabled = {
                let ownership = topology
                    .browser_context_routes()
                    .find(|route| route.object_id() == context)
                    .map(|route| route.ownership())
                    .ok_or(HostError::TargetClosed)?;
                live.context_http_credentials.enabled(context, ownership)?
            };
            #[cfg(not(feature = "context-http-credentials"))]
            let auth_enabled = false;
            (live.page_routes.clone(), context, sessions, auth_enabled)
        };
        let scope = RegistrationScope::Page(page);
        if let Some(registration_ids) = registration_ids {
            installation.remove_registrations(scope, registration_ids)?;
        } else {
            installation.remove_scope(scope)?;
        }
        for session in sessions {
            installation
                .update_session(&session, page, context, auth_enabled)
                .await?;
        }
        if wait {
            installation.wait_scope_idle(scope, &self.stop).await?;
        }
        Ok(())
    }

    pub async fn context_unroute(
        &self,
        context_id: &str,
        registration_ids: &[u64],
    ) -> Result<(), HostError> {
        self.context_unroute_impl(context_id, Some(registration_ids), false)
            .await
    }

    pub async fn context_unroute_all(&self, context_id: &str, wait: bool) -> Result<(), HostError> {
        self.context_unroute_impl(context_id, None, wait).await
    }

    async fn context_unroute_impl(
        &self,
        context_id: &str,
        registration_ids: Option<&[u64]>,
        wait: bool,
    ) -> Result<(), HostError> {
        let context = context_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Context ID".into()))?;
        self.require_visible_context_id(context)?;
        let (installation, sessions, auth_enabled) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            let ownership = topology
                .browser_context_routes()
                .find(|route| route.object_id() == context)
                .map(|route| route.ownership())
                .ok_or(HostError::TargetClosed)?;
            let pages = topology
                .page_session_routes()
                .filter(|route| route.browser_context_object_id() == context)
                .map(|route| (route.object_id(), route.target_id().to_owned()))
                .collect::<Vec<_>>();
            let sessions = pages
                .iter()
                .flat_map(|(page, target)| {
                    topology
                        .target_session_routes()
                        .filter(move |route| route.page_target_id() == target)
                        .map(move |route| (*page, route.session_id().to_owned()))
                })
                .collect::<Vec<_>>();
            #[cfg(feature = "context-http-credentials")]
            let auth_enabled = live.context_http_credentials.enabled(context, ownership)?;
            #[cfg(not(feature = "context-http-credentials"))]
            let auth_enabled = {
                let _ = ownership;
                false
            };
            (live.page_routes.clone(), sessions, auth_enabled)
        };
        let scope = RegistrationScope::Context(context);
        if let Some(registration_ids) = registration_ids {
            installation.remove_registrations(scope, registration_ids)?;
        } else {
            installation.remove_scope(scope)?;
        }
        for (page, session) in sessions {
            installation
                .update_session(&session, page, context, auth_enabled)
                .await?;
        }
        if wait {
            installation.wait_scope_idle(scope, &self.stop).await?;
        }
        Ok(())
    }

    pub async fn next_page_route_call(&self) -> Option<RouteCall> {
        let installation = {
            let state = self.state.lock().ok()?;
            state.as_ref()?.page_routes.clone()
        };
        installation.next_call(&self.stop).await
    }

    /// Bind a routed Request to its current network hop before the language
    /// handler can continue the interception or advance a redirect chain.
    /// The returned Arc shares the RequestHistoryOwner's existing resource
    /// lease; this path does not create an uncharged response copy or cache.
    #[cfg(feature = "route-request-history")]
    pub async fn next_page_route_call_with_history(
        &self,
    ) -> Result<
        Option<(
            RouteCall,
            Option<Arc<crate::network_events::RequestHistoryOwner>>,
        )>,
        HostError,
    > {
        let Some(call) = self.next_page_route_call().await else {
            return Ok(None);
        };
        let owner = capture_route_history_owner(&call, |page_id, native_request_id| async move {
            self.request_by_native_id(&page_id, &native_request_id)
                .await
        })
        .await;
        match owner {
            Ok(history_owner) => Ok(Some((call, history_owner))),
            Err(error) => {
                // This route will never reach a user callback. Do not leave a
                // Fetch request paused when correlation or its bound fails.
                let _ = self
                    .page_route_action(call.id, RouteAction::Abort(RouteAbortReason::Failed))
                    .await;
                Err(error)
            }
        }
    }

    async fn refresh_page_route_scope(
        &self,
        installation: &Installation,
        scope: RegistrationScope,
    ) -> Result<(), HostError> {
        let targets = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            match scope {
                RegistrationScope::Page(page) => {
                    let Some(route) = topology
                        .page_session_routes()
                        .find(|route| route.object_id() == page)
                    else {
                        return Ok(());
                    };
                    let context = route.browser_context_object_id();
                    let target = route.target_id().to_owned();
                    let Some(ownership) = topology
                        .browser_context_routes()
                        .find(|route| route.object_id() == context)
                        .map(|route| route.ownership())
                    else {
                        return Ok(());
                    };
                    #[cfg(feature = "context-http-credentials")]
                    let auth_enabled = live.context_http_credentials.enabled(context, ownership)?;
                    #[cfg(not(feature = "context-http-credentials"))]
                    let auth_enabled = {
                        let _ = ownership;
                        false
                    };
                    topology
                        .target_session_routes()
                        .filter(|route| route.page_target_id() == target)
                        .map(|route| (route.session_id().to_owned(), page, context, auth_enabled))
                        .collect::<Vec<_>>()
                }
                RegistrationScope::Context(context) => {
                    let Some(ownership) = topology
                        .browser_context_routes()
                        .find(|route| route.object_id() == context)
                        .map(|route| route.ownership())
                    else {
                        return Ok(());
                    };
                    #[cfg(feature = "context-http-credentials")]
                    let auth_enabled = live.context_http_credentials.enabled(context, ownership)?;
                    #[cfg(not(feature = "context-http-credentials"))]
                    let auth_enabled = {
                        let _ = ownership;
                        false
                    };
                    let pages = topology
                        .page_session_routes()
                        .filter(|route| route.browser_context_object_id() == context)
                        .map(|route| (route.object_id(), route.target_id().to_owned()))
                        .collect::<Vec<_>>();
                    pages
                        .iter()
                        .flat_map(|(page, target)| {
                            topology
                                .target_session_routes()
                                .filter(move |route| route.page_target_id() == target)
                                .map(move |route| {
                                    (route.session_id().to_owned(), *page, context, auth_enabled)
                                })
                        })
                        .collect::<Vec<_>>()
                }
            }
        };
        for (session, page, context, auth_enabled) in targets {
            installation
                .update_session(&session, page, context, auth_enabled)
                .await?;
        }
        Ok(())
    }

    async fn refresh_page_route_settlement(
        &self,
        installation: &Installation,
        settlement: Option<RouteSettlement>,
    ) -> Result<(), HostError> {
        let Some(settlement) = settlement else {
            return Ok(());
        };
        if settlement.refresh.context {
            return self
                .refresh_page_route_scope(
                    installation,
                    RegistrationScope::Context(settlement.context),
                )
                .await;
        }
        if settlement.refresh.page {
            self.refresh_page_route_scope(installation, RegistrationScope::Page(settlement.page))
                .await?;
        }
        Ok(())
    }

    pub async fn page_route_action(
        &self,
        call_id: u64,
        action: RouteAction,
    ) -> Result<(), HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .page_routes
                .clone()
        };
        let settlement = installation.act(call_id, action).await?;
        self.refresh_page_route_settlement(&installation, settlement)
            .await
    }

    pub async fn page_route_fallback(
        &self,
        call_id: u64,
        options: RouteContinue,
    ) -> Result<(), HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .page_routes
                .clone()
        };
        let settlement = installation.fallback(call_id, options).await?;
        self.refresh_page_route_settlement(&installation, settlement)
            .await
    }
}

#[cfg(feature = "har-replay")]
async fn read_har_file(path: &str) -> Result<Vec<u8>, HostError> {
    if path.is_empty() || path.len() > MAX_HAR_PATH_BYTES {
        return Err(HostError::InvalidArgument(format!(
            "HAR path must contain 1..={MAX_HAR_PATH_BYTES} UTF-8 bytes"
        )));
    }
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|error| HostError::InvalidArgument(format!("cannot open HAR file: {error}")))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(MAX_HAR_REPLAY_ARCHIVE_BYTES.min(64 * 1024))
        .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
    file.take((MAX_HAR_REPLAY_ARCHIVE_BYTES as u64) + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(HostError::backend)?;
    if bytes.len() > MAX_HAR_REPLAY_ARCHIVE_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "HAR file exceeds {MAX_HAR_REPLAY_ARCHIVE_BYTES} bytes"
        )));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "route-request-history")]
    use std::sync::{Arc, Mutex};

    use base64::Engine as _;
    use pwright_cdp::connection::CdpError;
    use serde_json::json;
    use tokio::sync::oneshot;

    use super::{ChromiumSessionResumeFence, fenced_fetch_enable};

    #[cfg(any(feature = "route-redirects", feature = "route-request-history"))]
    use super::HostError;
    use super::remove_registrations_from_state;
    use super::{
        FallbackCursor, MAX_CAPTURED_POST_DATA_BYTES, MAX_METHOD_BYTES, MAX_POST_DATA_BYTES,
        MAX_ROUTE_FULFILL_BODY_BYTES, MAX_URL_BYTES, ObjectId, PendingCall, Registration,
        RegistrationHandler, RegistrationScope, RouteAbortReason, RouteContinue, RouteFulfill,
        State, bounded_headers, captured_post_data, captured_post_data_source, cdp_pattern,
        cors_preflight_fulfill, has_matching_registration, is_favicon_url, reconcile_pending,
        route_matches, scope_is_idle, take_matching_registration, take_matching_registration_after,
        take_pending_action, terminal_route_error, wildcard_matches,
    };
    #[cfg(feature = "large-route-post-data")]
    use super::{LargePostDataBudget, MAX_LARGE_POST_DATA_CALLS};
    #[cfg(feature = "route-fulfill-cors")]
    use super::{MAX_HEADER_BYTES, with_default_cors_headers};
    #[cfg(feature = "route-redirects")]
    use super::{
        MAX_HEADER_COUNT, MAX_REDIRECT_HEADER_OVERRIDES, advance_redirect_header_override,
        retain_redirect_header_override,
    };
    #[cfg(feature = "route-regexp")]
    use super::{
        MAX_PATTERN_BYTES, MAX_REGEXP_ROUTES, RoutePattern, cdp_patterns,
        ensure_registration_capacity,
    };
    #[cfg(feature = "route-request-history")]
    use super::{RouteCall, capture_route_history_owner};

    async fn poll_once<T>(
        mut future: std::pin::Pin<&mut impl std::future::Future<Output = T>>,
    ) -> std::task::Poll<T> {
        std::future::poll_fn(|cx| std::task::Poll::Ready(future.as_mut().poll(cx))).await
    }

    #[tokio::test]
    async fn popup_context_route_fetch_enable_precedes_resume_without_headers() {
        // Arrange: the no-header popup still has an active context route.
        let fence = ChromiumSessionResumeFence::default();
        let (admit, admitted) = oneshot::channel::<()>();
        let (ack, settled) = oneshot::channel::<()>();
        let command = fenced_fetch_enable(
            &fence,
            async {
                admitted.await.unwrap();
                Ok::<_, CdpError>(())
            },
            |()| async {
                settled.await.unwrap();
                Ok::<(), CdpError>(())
            },
        );
        tokio::pin!(command);

        // Act: a blocked writer cannot admit Fetch.enable yet.
        assert!(poll_once(command.as_mut()).await.is_pending());
        assert!(fence.is_waiting());
        admit.send(()).unwrap();
        assert!(poll_once(command.as_mut()).await.is_pending());

        // Assert: resume is allowed only after admission, not after the ACK.
        assert!(!fence.is_waiting());
        ack.send(()).unwrap();
        assert!(matches!(
            poll_once(command.as_mut()).await,
            std::task::Poll::Ready(Ok(()))
        ));
    }

    #[cfg(feature = "route-request-history")]
    fn route_history_call(native_request_id: Option<&str>) -> RouteCall {
        RouteCall {
            id: 7,
            registration_id: 3,
            context: ObjectId::new(9),
            page: ObjectId::new(2),
            frame: Some(ObjectId::new(10)),
            url: "https://example.test/first".into(),
            method: "GET".into(),
            resource_type: "document".into(),
            headers: Vec::new(),
            post_data: None,
            is_navigation_request: true,
            expires: false,
            native_request_id: native_request_id.map(str::to_owned),
        }
    }

    #[cfg(feature = "route-request-history")]
    #[tokio::test]
    async fn route_history_capture_retains_the_current_hop_before_native_id_reuse() {
        // Arrange
        let call = route_history_call(Some("network-7"));
        let current_hop = Arc::new(Mutex::new("first-hop"));
        let lookup_hop = Arc::clone(&current_hop);

        // Act
        let captured = capture_route_history_owner(&call, move |page, native_id| async move {
            assert_eq!(page, "2");
            assert_eq!(native_id, "network-7");
            Ok::<_, HostError>(*lookup_hop.lock().unwrap())
        })
        .await
        .unwrap();
        *current_hop.lock().unwrap() = "redirect-hop";

        // Assert
        assert_eq!(captured, Some("first-hop"));
    }

    #[cfg(feature = "route-request-history")]
    #[tokio::test]
    async fn route_history_capture_without_network_id_does_not_guess() {
        // Arrange
        let call = route_history_call(None);

        // Act
        let captured = capture_route_history_owner(&call, |_, _| {
            std::future::ready(Err::<(), _>(HostError::Backend(
                "lookup must not run".into(),
            )))
        })
        .await
        .unwrap();

        // Assert
        assert!(captured.is_none());
    }

    #[cfg(feature = "route-request-history")]
    #[tokio::test(start_paused = true)]
    async fn route_history_capture_bounds_an_unresolved_owner_lookup() {
        // Arrange
        let call = route_history_call(Some("network-7"));

        // Act
        let result = capture_route_history_owner(&call, |_, _| {
            std::future::pending::<Result<(), HostError>>()
        })
        .await;

        // Assert
        assert!(matches!(result, Err(HostError::Timeout(30_000))));
    }

    fn pending_call(id: u64, page: ObjectId) -> PendingCall {
        PendingCall {
            id,
            context: ObjectId::new(9),
            page,
            frame: Some(ObjectId::new(10)),
            registration_scope: RegistrationScope::Page(page),
            fallback_cursor: super::FallbackCursor::Page {
                before_page: 1,
                before_context: 1,
            },
            session_id: "session".into(),
            request_id: "request".into(),
            request_scheme: "https".into(),
            url: "https://example.test/empty.html".into(),
            method: "GET".into(),
            resource_type: "document".into(),
            headers: Vec::new(),
            post_data: None,
            #[cfg(feature = "large-route-post-data")]
            large_post_data_slot: None,
            is_navigation_request: true,
            fallback_options: RouteContinue::default(),
            refresh: super::RouteRefresh::default(),
            #[cfg(feature = "network-events")]
            network_request_id: None,
        }
    }

    fn state_with_pending(pending: Vec<PendingCall>, retired: Vec<u64>) -> State {
        State {
            next_registration: 1,
            next_call: 1,
            registrations: Vec::new(),
            pending,
            settling: Vec::new(),
            retired,
            initializing_sessions: Vec::new(),
            #[cfg(feature = "route-redirects")]
            redirect_header_overrides: Vec::new(),
            #[cfg(feature = "har-replay")]
            har_bytes: 0,
            #[cfg(feature = "network-events")]
            fulfilled: Vec::new(),
        }
    }

    fn binding_registration(scope: RegistrationScope, remaining: Option<u64>) -> Registration {
        Registration {
            id: 7,
            scope,
            pattern: "**/empty.html".into(),
            handler: RegistrationHandler::Binding,
            remaining,
        }
    }

    #[cfg(feature = "context-storage-state")]
    #[test]
    fn temporary_storage_route_precedes_context_user_route_without_consuming_it() {
        // Arrange
        let page = ObjectId::new(3);
        let context = ObjectId::new(4);
        let mut state = state_with_pending(Vec::new(), Vec::new());
        state.next_registration = 3;
        state.registrations.push(Registration {
            id: 1,
            scope: RegistrationScope::Context(context),
            pattern: "**/*".into(),
            handler: RegistrationHandler::Binding,
            remaining: None,
        });
        state.registrations.push(Registration {
            id: 2,
            scope: RegistrationScope::Page(page),
            pattern: "**/*".into(),
            handler: RegistrationHandler::StorageState,
            remaining: None,
        });

        // Act
        let selected =
            take_matching_registration(&mut state, page, context, "https://example.com", false)
                .unwrap()
                .unwrap();

        // Assert
        assert!(matches!(
            selected.handler,
            RegistrationHandler::StorageState
        ));
        assert_eq!(state.registrations.len(), 2);
        assert!(matches!(
            state.registrations[0].handler,
            RegistrationHandler::Binding
        ));
    }

    #[cfg(feature = "context-storage-state")]
    #[test]
    fn temporary_route_cleanup_can_retry_without_touching_context_route() {
        // Arrange
        let page = ObjectId::new(3);
        let context = ObjectId::new(4);
        let mut state = state_with_pending(Vec::new(), Vec::new());
        state.registrations.push(Registration {
            id: 1,
            scope: RegistrationScope::Context(context),
            pattern: "**/*".into(),
            handler: RegistrationHandler::Binding,
            remaining: None,
        });
        state.registrations.push(Registration {
            id: 2,
            scope: RegistrationScope::Page(page),
            pattern: "**/*".into(),
            handler: RegistrationHandler::StorageState,
            remaining: None,
        });

        // Act
        remove_registrations_from_state(&mut state, RegistrationScope::Page(page), &[2]).unwrap();
        remove_registrations_from_state(&mut state, RegistrationScope::Page(page), &[2]).unwrap();

        // Assert
        assert_eq!(state.registrations.len(), 1);
        assert_eq!(state.registrations[0].id, 1);
    }

    fn registration(
        id: u64,
        scope: RegistrationScope,
        pattern: &str,
        remaining: Option<u64>,
    ) -> Registration {
        Registration {
            id,
            scope,
            pattern: pattern.into(),
            handler: RegistrationHandler::Binding,
            remaining,
        }
    }

    #[cfg(feature = "route-regexp")]
    fn regexp_registration(
        id: u64,
        scope: RegistrationScope,
        source: &str,
        times: Option<u64>,
    ) -> Registration {
        Registration {
            id,
            scope,
            pattern: RoutePattern::regexp(source.into(), String::new()).unwrap(),
            handler: RegistrationHandler::Binding,
            remaining: times,
        }
    }

    #[cfg(feature = "route-regexp")]
    #[test]
    fn regexp_route_matches_full_url_without_glob_base_url_resolution() {
        // Arrange
        let literal = RoutePattern::regexp("empty\\.html$".into(), String::new()).unwrap();
        let anchored =
            RoutePattern::regexp("^https://example\\.test/".into(), String::new()).unwrap();

        // Act
        let literal_match = literal
            .matches_url("https://example.test/empty.html")
            .unwrap();
        let anchored_match = anchored
            .matches_url("https://other.test/empty.html")
            .unwrap();

        // Assert
        assert!(literal_match);
        assert!(!anchored_match);
    }

    #[cfg(feature = "route-regexp")]
    #[test]
    fn regexp_route_rejects_unsupported_flags_syntax_and_source_over_limit() {
        // Arrange
        let over_limit = "a".repeat(MAX_PATTERN_BYTES + 1);

        // Act
        let unsupported_flag = RoutePattern::regexp("a".into(), "g".into());
        let duplicate_flag = RoutePattern::regexp("a".into(), "ii".into());
        let unsupported_lookbehind = RoutePattern::regexp("(?<=a)b".into(), String::new());
        let oversized = RoutePattern::regexp(over_limit, String::new());

        // Assert
        assert!(matches!(
            unsupported_flag,
            Err(super::HostError::InvalidArgument(_))
        ));
        assert!(matches!(
            duplicate_flag,
            Err(super::HostError::InvalidArgument(_))
        ));
        assert!(matches!(
            unsupported_lookbehind,
            Err(super::HostError::InvalidArgument(_))
        ));
        assert!(matches!(
            oversized,
            Err(super::HostError::InvalidArgument(_))
        ));
    }

    #[cfg(feature = "route-regexp")]
    #[test]
    fn regexp_flags_select_supported_case_and_multiline_modes() {
        // Arrange
        let pattern = RoutePattern::regexp("^EMPTY\\.HTML$".into(), "im".into()).unwrap();

        // Act
        let matched = pattern
            .matches_url("https://example.test/start\nempty.html\nfinish")
            .unwrap();

        // Assert
        assert!(matched);
    }

    #[cfg(feature = "route-regexp")]
    #[test]
    fn regexp_route_expands_cdp_patterns_only_while_active() {
        // Arrange
        let page = ObjectId::new(3);
        let context = ObjectId::new(4);
        let other_page = ObjectId::new(5);
        let mut state = state_with_pending(Vec::new(), Vec::new());
        state.registrations = vec![
            registration(1, RegistrationScope::Page(page), "**/*.css", None),
            regexp_registration(2, RegistrationScope::Page(page), "\\.html$", None),
        ];
        let broad = cdp_patterns(&state.registrations, page, context).unwrap();

        // Act
        remove_registrations_from_state(&mut state, RegistrationScope::Page(page), &[2]).unwrap();

        // Assert
        assert_eq!(broad, vec!["*"]);
        let restored = cdp_patterns(&state.registrations, page, context).unwrap();
        assert_eq!(restored, vec!["**/*.css"]);
        assert!(
            cdp_patterns(&state.registrations, other_page, context)
                .unwrap()
                .is_empty()
        );
    }

    #[cfg(feature = "route-regexp")]
    #[test]
    fn regexp_route_preserves_page_lifo_fallback_and_finite_times() {
        // Arrange
        let page = ObjectId::new(3);
        let context = ObjectId::new(4);
        let mut state = state_with_pending(Vec::new(), Vec::new());
        state.next_registration = 4;
        state.registrations = vec![
            regexp_registration(1, RegistrationScope::Page(page), "empty\\.html$", Some(1)),
            registration(2, RegistrationScope::Page(page), "**/empty.html", None),
            regexp_registration(3, RegistrationScope::Page(page), "unmatched", None),
        ];

        // Act
        let first = take_matching_registration(
            &mut state,
            page,
            context,
            "https://example.test/empty.html",
            false,
        )
        .unwrap()
        .unwrap();
        let second = take_matching_registration_after(
            &mut state,
            page,
            context,
            "https://example.test/empty.html",
            first.next_cursor,
        )
        .unwrap()
        .unwrap();
        let third = take_matching_registration_after(
            &mut state,
            page,
            context,
            "https://example.test/empty.html",
            second.next_cursor,
        )
        .unwrap();

        // Assert
        assert_eq!([first.id, second.id], [2, 1]);
        assert!(second.expires);
        assert!(third.is_none());
        assert!(
            state
                .registrations
                .iter()
                .all(|registration| registration.id != 1)
        );
    }

    #[cfg(feature = "route-regexp")]
    #[test]
    fn regexp_route_cap_leaves_room_for_glob_registrations() {
        // Arrange
        let page = ObjectId::new(3);
        let registrations = (1..=MAX_REGEXP_ROUTES)
            .map(|id| regexp_registration(id as u64, RegistrationScope::Page(page), "x", None))
            .collect::<Vec<_>>();
        let regexp = RoutePattern::regexp("y".into(), String::new()).unwrap();
        let glob = RoutePattern::from("**/*");

        // Act
        let regexp_result = ensure_registration_capacity(&registrations, &regexp);
        let glob_result = ensure_registration_capacity(&registrations, &glob);

        // Assert
        assert!(matches!(
            regexp_result,
            Err(super::HostError::ResourceExhausted(_))
        ));
        assert!(glob_result.is_ok());
    }

    #[test]
    fn unroute_selected_ids_preserves_other_registration_order_and_times() {
        // Arrange
        let page = ObjectId::new(3);
        let context = ObjectId::new(4);
        let mut state = state_with_pending(Vec::new(), Vec::new());
        state.registrations = vec![
            registration(1, RegistrationScope::Page(page), "**/*", Some(2)),
            registration(2, RegistrationScope::Page(page), "**/*", None),
            registration(3, RegistrationScope::Context(context), "**/*", Some(4)),
            registration(4, RegistrationScope::Page(page), "**/*", Some(1)),
        ];

        // Act
        remove_registrations_from_state(&mut state, RegistrationScope::Page(page), &[1, 4])
            .unwrap();

        // Assert
        assert_eq!(state.registrations.len(), 2);
        assert_eq!(state.registrations[0].id, 2);
        assert_eq!(state.registrations[0].remaining, None);
        assert_eq!(state.registrations[1].id, 3);
        assert_eq!(state.registrations[1].remaining, Some(4));
    }

    #[test]
    fn unroute_rejects_foreign_registration_without_partial_removal() {
        // Arrange
        let page = ObjectId::new(3);
        let other_page = ObjectId::new(5);
        let mut state = state_with_pending(Vec::new(), Vec::new());
        state.registrations = vec![
            registration(1, RegistrationScope::Page(page), "**/*", None),
            registration(2, RegistrationScope::Page(other_page), "**/*", Some(2)),
        ];

        // Act
        let result =
            remove_registrations_from_state(&mut state, RegistrationScope::Page(page), &[1, 2]);

        // Assert
        assert!(matches!(result, Err(super::HostError::InvalidArgument(_))));
        assert_eq!(state.registrations.len(), 2);
        assert_eq!(state.registrations[0].id, 1);
        assert_eq!(state.registrations[1].id, 2);
    }

    #[test]
    fn finite_route_budget_decrements_without_expiring_early() {
        // Arrange
        let page = ObjectId::new(3);
        let context = ObjectId::new(4);
        let mut state = state_with_pending(Vec::new(), Vec::new());
        state.next_registration = 8;
        state
            .registrations
            .push(binding_registration(RegistrationScope::Page(page), Some(2)));

        // Act
        let selected = take_matching_registration(
            &mut state,
            page,
            context,
            "https://example.test/empty.html",
            false,
        )
        .unwrap()
        .unwrap();

        // Assert
        assert_eq!(selected.id, 7);
        assert!(!selected.expires);
        assert_eq!(state.registrations[0].remaining, Some(1));
    }

    #[test]
    fn last_finite_route_use_atomically_removes_registration() {
        // Arrange
        let page = ObjectId::new(3);
        let context = ObjectId::new(4);
        let mut state = state_with_pending(Vec::new(), Vec::new());
        state.next_registration = 8;
        state
            .registrations
            .push(binding_registration(RegistrationScope::Page(page), Some(1)));

        // Act
        let selected = take_matching_registration(
            &mut state,
            page,
            context,
            "https://example.test/empty.html",
            false,
        )
        .unwrap()
        .unwrap();

        // Assert
        assert_eq!(selected.id, 7);
        assert!(selected.expires);
        assert!(state.registrations.is_empty());
    }

    #[test]
    fn stale_cdp_pause_does_not_select_a_nonmatching_registration() {
        // Arrange
        let page = ObjectId::new(3);
        let context = ObjectId::new(4);
        let mut state = state_with_pending(Vec::new(), Vec::new());
        state.registrations.push(binding_registration(
            RegistrationScope::Context(context),
            None,
        ));

        // Act
        let selected = take_matching_registration(
            &mut state,
            page,
            context,
            "https://example.test/other.html",
            false,
        )
        .unwrap();

        // Assert
        assert!(selected.is_none());
        assert_eq!(state.registrations.len(), 1);
    }

    #[test]
    fn fallback_traverses_page_handlers_before_context_handlers_in_lifo_order() {
        // Arrange
        let page = ObjectId::new(3);
        let context = ObjectId::new(4);
        let mut state = state_with_pending(Vec::new(), Vec::new());
        state.next_registration = 5;
        state.registrations = vec![
            registration(1, RegistrationScope::Context(context), "**/*", None),
            registration(2, RegistrationScope::Page(page), "**/*", None),
            registration(3, RegistrationScope::Context(context), "**/*", None),
            registration(4, RegistrationScope::Page(page), "**/*", None),
        ];

        // Act
        let first = take_matching_registration(
            &mut state,
            page,
            context,
            "https://example.test/empty.html",
            false,
        )
        .unwrap()
        .unwrap();
        let second = take_matching_registration_after(
            &mut state,
            page,
            context,
            "https://example.test/empty.html",
            first.next_cursor,
        )
        .unwrap()
        .unwrap();
        let third = take_matching_registration_after(
            &mut state,
            page,
            context,
            "https://example.test/empty.html",
            second.next_cursor,
        )
        .unwrap()
        .unwrap();
        let fourth = take_matching_registration_after(
            &mut state,
            page,
            context,
            "https://example.test/empty.html",
            third.next_cursor,
        )
        .unwrap()
        .unwrap();
        let exhausted = take_matching_registration_after(
            &mut state,
            page,
            context,
            "https://example.test/empty.html",
            fourth.next_cursor,
        )
        .unwrap();

        // Assert
        assert_eq!([first.id, second.id, third.id, fourth.id], [4, 2, 3, 1]);
        assert!(exhausted.is_none());
    }

    #[test]
    fn fallback_matches_older_handlers_against_the_effective_url() {
        // Arrange
        let page = ObjectId::new(3);
        let context = ObjectId::new(4);
        let mut state = state_with_pending(Vec::new(), Vec::new());
        state.next_registration = 3;
        state.registrations = vec![
            registration(1, RegistrationScope::Context(context), "**/bar", None),
            registration(2, RegistrationScope::Page(page), "**/foo", None),
        ];
        let first = take_matching_registration(
            &mut state,
            page,
            context,
            "https://example.test/foo",
            false,
        )
        .unwrap()
        .unwrap();

        // Act
        let second = take_matching_registration_after(
            &mut state,
            page,
            context,
            "https://example.test/bar",
            first.next_cursor,
        )
        .unwrap()
        .unwrap();

        // Assert
        assert_eq!(first.id, 2);
        assert_eq!(second.id, 1);
        assert_eq!(
            second.next_cursor,
            FallbackCursor::Context { before_context: 1 }
        );
    }

    #[test]
    fn page_route_globs_match_playwright_shape() {
        // Arrange
        let url = "https://example.test/assets/img.png";

        // Act / Assert
        assert!(route_matches("**/img.png", url).unwrap());
        assert!(route_matches("**/*", url).unwrap());
        assert!(route_matches("/assets/img.png", url).unwrap());
        assert!(!route_matches("**/empty.html", url).unwrap());
        assert!(wildcard_matches(b"https://*.test/**", url.as_bytes()).unwrap());
    }

    #[test]
    fn route_request_headers_use_playwrights_lowercase_public_shape() {
        // Arrange
        let native = json!({
            "Referer": "https://example.test/",
            "X-Custom-Header": "value"
        });

        // Act
        let headers = bounded_headers(Some(&native)).unwrap();

        // Assert
        assert_eq!(
            headers,
            vec![
                ("referer".to_owned(), "https://example.test/".to_owned()),
                ("x-custom-header".to_owned(), "value".to_owned())
            ]
        );
    }

    #[test]
    fn cors_preflight_fulfill_echoes_browser_request_headers() {
        // Arrange
        let headers = vec![
            ("origin".into(), "https://caller.test".into()),
            ("access-control-request-method".into(), "POST".into()),
            (
                "access-control-request-headers".into(),
                "content-type,x-extra".into(),
            ),
        ];

        // Act
        let params = cors_preflight_fulfill("OPTIONS", &headers)
            .unwrap()
            .unwrap()
            .protocol_params("preflight-7");

        // Assert
        assert_eq!(
            params,
            json!({
                "requestId": "preflight-7",
                "responseCode": 204,
                "responsePhrase": "No Content",
                "responseHeaders": [
                    { "name": "Access-Control-Allow-Origin", "value": "https://caller.test" },
                    { "name": "Access-Control-Allow-Methods", "value": "POST" },
                    { "name": "Access-Control-Allow-Credentials", "value": "true" },
                    { "name": "Access-Control-Allow-Headers", "value": "content-type,x-extra" }
                ],
                "body": "",
            })
        );
    }

    #[test]
    fn ordinary_options_request_is_not_treated_as_a_cors_preflight() {
        // Arrange
        let headers = vec![("origin".into(), "https://caller.test".into())];

        // Act
        let response = cors_preflight_fulfill("OPTIONS", &headers).unwrap();

        // Assert
        assert!(response.is_none());
    }

    #[cfg(feature = "route-fulfill-cors")]
    #[test]
    fn cross_origin_final_fulfill_adds_origin_credentials_and_vary() {
        // Arrange
        let mut pending = pending_call(1, ObjectId::new(3));
        pending.url = "https://api.test/cars".into();
        pending.method = "POST".into();
        pending.headers = vec![("origin".into(), "https://caller.test".into())];
        let fulfill = RouteFulfill::new(
            200,
            vec![("content-type".into(), "application/json".into())],
            b"[\"electric\"]".to_vec(),
        )
        .unwrap();

        // Act
        let fulfill = with_default_cors_headers(fulfill, &pending).unwrap();

        // Assert
        assert_eq!(
            fulfill.headers,
            vec![
                ("content-type".into(), "application/json".into()),
                (
                    "access-control-allow-origin".into(),
                    "https://caller.test".into()
                ),
                ("access-control-allow-credentials".into(), "true".into()),
                ("vary".into(), "Origin".into()),
            ]
        );
        assert_eq!(fulfill.body.as_ref(), b"[\"electric\"]");
    }

    #[cfg(feature = "route-fulfill-cors")]
    #[test]
    fn null_origin_final_fulfill_is_accepted() {
        // Arrange
        let mut pending = pending_call(1, ObjectId::new(3));
        pending.url = "https://api.test/something".into();
        pending.headers = vec![("Origin".into(), "null".into())];
        let fulfill = RouteFulfill::new(200, Vec::new(), b"done".to_vec()).unwrap();

        // Act
        let fulfill = with_default_cors_headers(fulfill, &pending).unwrap();

        // Assert
        assert_eq!(
            fulfill.headers[0],
            ("access-control-allow-origin".into(), "null".into())
        );
        assert_eq!(
            fulfill.headers[1],
            ("access-control-allow-credentials".into(), "true".into())
        );
        assert_eq!(fulfill.headers[2], ("vary".into(), "Origin".into()));
    }

    #[cfg(feature = "route-fulfill-cors")]
    #[test]
    fn explicit_access_control_allow_origin_preserves_all_user_headers() {
        // Arrange
        let mut pending = pending_call(1, ObjectId::new(3));
        pending.url = "https://api.test/cars".into();
        pending.headers = vec![("Origin".into(), "https://caller.test".into())];
        let headers = vec![
            ("Access-Control-Allow-Origin".into(), "*".into()),
            ("vary".into(), "X-Test".into()),
        ];
        let fulfill = RouteFulfill::new(200, headers.clone(), Vec::new()).unwrap();

        // Act
        let fulfill = with_default_cors_headers(fulfill, &pending).unwrap();

        // Assert
        assert_eq!(fulfill.headers, headers);
    }

    #[cfg(feature = "route-fulfill-cors")]
    #[test]
    fn same_origin_final_fulfill_has_no_extra_headers() {
        // Arrange
        let mut pending = pending_call(1, ObjectId::new(3));
        pending.url = "https://caller.test:443/cars".into();
        pending.headers = vec![("origin".into(), " https://caller.test ".into())];
        let fulfill = RouteFulfill::new(200, Vec::new(), Vec::new()).unwrap();

        // Act
        let fulfill = with_default_cors_headers(fulfill, &pending).unwrap();

        // Assert
        assert!(fulfill.headers.is_empty());
    }

    #[cfg(feature = "route-fulfill-cors")]
    #[test]
    fn non_http_final_fulfill_has_no_extra_headers() {
        // Arrange
        let mut pending = pending_call(1, ObjectId::new(3));
        pending.url = "data:text/plain,done".into();
        pending.request_scheme = "data".into();
        pending.headers = vec![("origin".into(), "https://caller.test".into())];
        let fulfill = RouteFulfill::new(200, Vec::new(), Vec::new()).unwrap();

        // Act
        let fulfill = with_default_cors_headers(fulfill, &pending).unwrap();

        // Assert
        assert!(fulfill.headers.is_empty());
    }

    #[cfg(feature = "route-fulfill-cors")]
    #[test]
    fn final_fulfill_without_origin_has_no_extra_headers() {
        // Arrange
        let pending = pending_call(1, ObjectId::new(3));
        let fulfill = RouteFulfill::new(200, Vec::new(), Vec::new()).unwrap();

        // Act
        let fulfill = with_default_cors_headers(fulfill, &pending).unwrap();

        // Assert
        assert!(fulfill.headers.is_empty());
    }

    #[cfg(feature = "route-fulfill-cors")]
    #[test]
    fn final_fulfill_uses_fallback_url_and_origin_overrides() {
        // Arrange
        let mut pending = pending_call(1, ObjectId::new(3));
        pending.url = "https://caller.test/cars".into();
        pending.headers = vec![("origin".into(), "https://caller.test".into())];
        pending.fallback_options = RouteContinue::new(
            Some("https://api.test/cars".into()),
            None,
            Some(vec![("origin".into(), "null".into())]),
            None,
        )
        .unwrap();
        let fulfill = RouteFulfill::new(200, Vec::new(), Vec::new()).unwrap();

        // Act
        let fulfill = with_default_cors_headers(fulfill, &pending).unwrap();

        // Assert
        assert_eq!(
            fulfill.headers[0],
            ("access-control-allow-origin".into(), "null".into())
        );
    }

    #[cfg(feature = "route-fulfill-cors")]
    #[test]
    fn final_fulfill_rejects_cors_header_count_overflow() {
        // Arrange
        let mut pending = pending_call(1, ObjectId::new(3));
        pending.url = "https://api.test/cars".into();
        pending.headers = vec![("origin".into(), "https://caller.test".into())];
        let headers = vec![("x-test".into(), "v".into()); super::MAX_HEADER_COUNT - 2];
        let fulfill = RouteFulfill::new(200, headers, Vec::new()).unwrap();

        // Act
        let result = with_default_cors_headers(fulfill, &pending);

        // Assert
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("headers exceed count limit")
        );
    }

    #[cfg(feature = "route-fulfill-cors")]
    #[test]
    fn final_fulfill_rejects_cors_header_byte_overflow() {
        // Arrange
        let mut pending = pending_call(1, ObjectId::new(3));
        pending.url = "https://api.test/cars".into();
        pending.headers = vec![("origin".into(), "https://caller.test".into())];
        let headers = vec![("x".into(), "v".repeat(MAX_HEADER_BYTES - 1))];
        let fulfill = RouteFulfill::new(200, headers, Vec::new()).unwrap();

        // Act
        let result = with_default_cors_headers(fulfill, &pending);

        // Assert
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("headers exceed byte limit")
        );
    }

    #[test]
    fn cors_preflight_match_does_not_consume_the_user_route_budget() {
        // Arrange
        let page = ObjectId::new(3);
        let context = ObjectId::new(4);
        let mut state = state_with_pending(Vec::new(), Vec::new());
        state
            .registrations
            .push(binding_registration(RegistrationScope::Page(page), Some(1)));

        // Act
        let matched =
            has_matching_registration(&state, page, context, "https://example.test/empty.html")
                .unwrap();

        // Assert
        assert!(matched);
        assert_eq!(state.registrations[0].remaining, Some(1));
    }

    #[test]
    fn playwright_internal_favicon_requests_bypass_user_routes() {
        // Arrange / Act / Assert
        assert!(is_favicon_url("https://example.test/favicon.ico"));
        assert!(!is_favicon_url("https://example.test/favicon.ico?v=1"));
        assert!(!is_favicon_url("https://example.test/assets/favicon.png"));
    }

    #[test]
    fn page_route_question_mark_is_literal_in_playwright_globs() {
        // Arrange
        let literal_question = b"https://example.test/index?hello";
        let arbitrary_character = b"https://example.test/index1hello";

        // Act
        let plain_matches_literal =
            wildcard_matches(b"https://example.test/index?hello", literal_question).unwrap();
        let escaped_matches_literal =
            wildcard_matches(br"https://example.test/index\?hello", literal_question).unwrap();
        let plain_matches_arbitrary =
            wildcard_matches(b"https://example.test/index?hello", arbitrary_character).unwrap();

        // Assert
        assert!(plain_matches_literal);
        assert!(escaped_matches_literal);
        assert!(!plain_matches_arbitrary);
    }

    #[test]
    fn leading_path_becomes_a_cdp_full_url_pattern() {
        // Arrange / Act / Assert
        assert_eq!(cdp_pattern("/img.png").unwrap(), "*/img.png");
        assert_eq!(cdp_pattern("**/img.png").unwrap(), "**/img.png");
        assert_eq!(cdp_pattern("**/index?hello").unwrap(), r"**/index\?hello");
        assert_eq!(cdp_pattern(r"**/index\?hello").unwrap(), r"**/index\?hello");
        assert_eq!(
            cdp_pattern(r"**/index\\?hello").unwrap(),
            r"**/index\\\?hello"
        );
    }

    #[test]
    fn route_abort_codes_are_validated_in_rust() {
        // Arrange / Act / Assert
        assert_eq!(
            RouteAbortReason::parse(None).unwrap().protocol_name(),
            "Failed"
        );
        assert_eq!(
            RouteAbortReason::parse(Some("connectionreset"))
                .unwrap()
                .protocol_name(),
            "ConnectionReset"
        );
        assert!(RouteAbortReason::parse(Some("not-a-code")).is_err());
    }

    #[test]
    fn continue_after_page_close_accepts_chromiums_retired_session_receipt() {
        // Arrange
        let error = CdpError::Protocol {
            code: -32_001,
            message: "Session with given id not found.".to_owned(),
        };

        // Act / Assert
        assert!(terminal_route_error(&error));
    }

    #[test]
    fn closing_a_page_retires_its_pending_route_call() {
        // Arrange
        let page = ObjectId::new(7);
        let mut state = state_with_pending(vec![pending_call(11, page)], Vec::new());

        // Act
        reconcile_pending(&mut state, &[]);

        // Assert
        assert!(state.pending.is_empty());
        assert_eq!(state.retired, vec![11]);
    }

    #[test]
    fn action_after_page_close_consumes_a_terminal_route_receipt() {
        // Arrange
        let mut state = state_with_pending(Vec::new(), vec![11]);

        // Act
        let action = take_pending_action(&mut state, 11).unwrap();

        // Assert
        assert!(action.is_none());
        assert!(state.retired.is_empty());
    }

    #[test]
    fn started_action_keeps_its_registration_scope_busy_until_settlement() {
        // Arrange
        let page = ObjectId::new(7);
        let scope = RegistrationScope::Page(page);
        let mut state = state_with_pending(vec![pending_call(11, page)], Vec::new());

        // Act
        let pending = take_pending_action(&mut state, 11).unwrap().unwrap();

        // Assert
        assert_eq!(pending.registration_scope, scope);
        assert!(state.pending.is_empty());
        assert!(!scope_is_idle(&state, scope));
        state.settling.clear();
        assert!(scope_is_idle(&state, scope));
    }

    #[test]
    fn captured_route_body_preserves_selected_limit_and_rejects_one_more_byte() {
        // Arrange
        let at_limit = json!({ "postData": "0".repeat(MAX_CAPTURED_POST_DATA_BYTES) });
        let over_limit = json!({ "postData": "0".repeat(MAX_CAPTURED_POST_DATA_BYTES + 1) });

        // Act
        let captured =
            captured_post_data_source(at_limit.as_object().unwrap()).and_then(captured_post_data);
        let rejected =
            captured_post_data_source(over_limit.as_object().unwrap()).and_then(captured_post_data);

        // Assert
        assert_eq!(
            captured.unwrap().unwrap().len(),
            MAX_CAPTURED_POST_DATA_BYTES
        );
        assert!(matches!(
            rejected,
            Err(super::HostError::ResourceExhausted(message))
                if message.contains("route POST data exceeds byte limit")
        ));
    }

    #[cfg(feature = "large-route-post-data")]
    #[test]
    fn routed_two_megabyte_body_decodes_ordered_base64_entries_before_delivery() {
        // Arrange
        let first = vec![b'0'; 1024 * 1024];
        let second = vec![b'1'; 1024 * 1024];
        let request = json!({
            "postData": "stale legacy field",
            "postDataEntries": [
                {"bytes": base64::engine::general_purpose::STANDARD.encode(&first)},
                {"bytes": base64::engine::general_purpose::STANDARD.encode(&second)}
            ]
        });
        let budget = std::sync::Arc::new(LargePostDataBudget::default());

        // Act
        let source = captured_post_data_source(request.as_object().unwrap()).unwrap();
        let slot = budget.acquire(source.bytes()).unwrap();
        let captured = captured_post_data(source).unwrap().unwrap();

        // Assert
        assert!(slot.is_some());
        assert_eq!(captured.len(), 2 * 1024 * 1024);
        assert_eq!(&captured[..first.len()], first.as_slice());
        assert_eq!(&captured[first.len()..], second.as_slice());
    }

    #[test]
    fn routed_body_rejects_malformed_base64_entry() {
        // Arrange
        let request = json!({"postDataEntries": [{"bytes": "%%%%"}]});

        // Act
        let result =
            captured_post_data_source(request.as_object().unwrap()).and_then(captured_post_data);

        // Assert
        assert!(matches!(result, Err(super::HostError::Backend(message))
            if message.contains("postDataEntries.bytes base64")));
    }

    #[test]
    fn routed_body_rejects_post_data_entry_count_above_cap() {
        // Arrange
        let request = json!({"postDataEntries": vec![json!({"bytes": ""}); super::MAX_POST_DATA_ENTRIES + 1]});

        // Act
        let result = captured_post_data_source(request.as_object().unwrap());

        // Assert
        assert!(
            matches!(result, Err(super::HostError::ResourceExhausted(message))
            if message.contains("entry count exceeds limit"))
        );
    }

    #[test]
    fn routed_body_rejects_decoded_entries_above_selected_cap() {
        // Arrange
        let encoded =
            base64::engine::general_purpose::STANDARD
                .encode(vec![b'0'; MAX_CAPTURED_POST_DATA_BYTES + 1]);
        let request = json!({"postDataEntries": [{"bytes": encoded}]});

        // Act
        let result = captured_post_data_source(request.as_object().unwrap());

        // Assert
        assert!(
            matches!(result, Err(super::HostError::ResourceExhausted(message))
            if message.contains("route POST data exceeds byte limit"))
        );
    }

    #[cfg(feature = "large-route-post-data")]
    #[test]
    fn large_route_body_slots_survive_pending_removal_until_queued_copy_drops() {
        // Arrange
        let budget = std::sync::Arc::new(LargePostDataBudget::default());
        let body_bytes = MAX_POST_DATA_BYTES + 1;
        let mut pending_slots = Vec::new();
        for _ in 0..MAX_LARGE_POST_DATA_CALLS {
            pending_slots.push(budget.acquire(Some(body_bytes)).unwrap().unwrap());
        }
        let queued_slot = std::sync::Arc::clone(&pending_slots[0]);

        // Act
        let denied_pending = budget.acquire(Some(body_bytes));
        pending_slots.remove(0);
        let denied_queued = budget.acquire(Some(body_bytes));
        drop(queued_slot);
        let admitted_after_release = budget.acquire(Some(body_bytes));

        // Assert
        assert!(matches!(
            denied_pending,
            Err(super::HostError::ResourceExhausted(_))
        ));
        assert!(matches!(
            denied_queued,
            Err(super::HostError::ResourceExhausted(_))
        ));
        assert!(admitted_after_release.is_ok());
    }

    #[test]
    fn context_route_scope_applies_to_future_pages_until_context_close() {
        // Arrange
        let context = ObjectId::new(3);
        let first_page = ObjectId::new(7);
        let second_page = ObjectId::new(8);
        let scope = RegistrationScope::Context(context);

        // Act / Assert
        assert!(scope.applies_to(first_page, context));
        assert!(scope.applies_to(second_page, context));
        assert!(scope.is_active(&[], &[context]));
        assert!(!scope.is_active(&[first_page, second_page], &[]));
    }

    #[test]
    fn page_route_scope_does_not_leak_to_another_page() {
        // Arrange
        let context = ObjectId::new(3);
        let page = ObjectId::new(7);
        let scope = RegistrationScope::Page(page);

        // Act / Assert
        assert!(scope.applies_to(page, context));
        assert!(!scope.applies_to(ObjectId::new(8), context));
        assert!(scope.is_active(&[page], &[context]));
        assert!(!scope.is_active(&[], &[context]));
    }

    #[test]
    fn route_fulfill_validates_status_headers_and_body_before_cdp_dispatch() {
        // Arrange
        let valid_headers = vec![("content-type".into(), "text/plain".into())];

        // Act
        let valid = RouteFulfill::new(200, valid_headers, b"ok".to_vec());
        let invalid_status = RouteFulfill::new(99, Vec::new(), Vec::new());
        let invalid_header =
            RouteFulfill::new(200, vec![("bad header".into(), "value".into())], Vec::new());
        let oversized =
            RouteFulfill::new(200, Vec::new(), vec![0; MAX_ROUTE_FULFILL_BODY_BYTES + 1]);

        // Assert
        assert!(valid.is_ok());
        assert!(invalid_status.is_err());
        assert!(invalid_header.is_err());
        assert!(oversized.is_err());
    }

    #[cfg(feature = "route-fulfill-path")]
    #[tokio::test]
    async fn route_fulfill_path_reads_bounded_bytes_and_uses_extension_content_type() {
        // Arrange
        let path = std::env::temp_dir().join(format!(
            "pwright-route-fulfill-path-{}.png",
            std::process::id()
        ));
        tokio::fs::write(&path, b"png-bytes").await.unwrap();
        let headers = vec![
            ("content-type".into(), "shouldBeIgnored".into()),
            ("x-test".into(), "yes".into()),
        ];

        // Act
        let result = RouteFulfill::from_path(200, headers, path.to_str().unwrap()).await;
        let _ = tokio::fs::remove_file(path).await;
        let fulfill = result.unwrap();

        // Assert
        assert_eq!(fulfill.body.as_ref(), b"png-bytes");
        assert_eq!(
            fulfill.headers,
            vec![
                ("x-test".into(), "yes".into()),
                ("content-type".into(), "image/png".into()),
            ]
        );
    }

    #[test]
    fn route_fulfill_serializes_playwright_status_text_instead_of_browser_default() {
        // Arrange
        let fulfill = RouteFulfill::new(
            422,
            vec![("content-type".into(), "text/plain".into())],
            b"body".to_vec(),
        )
        .unwrap();

        // Act
        let params = fulfill.protocol_params("request-7");

        // Assert
        assert_eq!(
            params,
            json!({
                "requestId": "request-7",
                "responseCode": 422,
                "responsePhrase": "Unprocessable Entity",
                "responseHeaders": [{ "name": "content-type", "value": "text/plain" }],
                "body": "Ym9keQ==",
            })
        );
    }

    #[test]
    fn route_fulfill_uses_playwright_unknown_phrase_for_unlisted_status() {
        // Arrange
        let fulfill = RouteFulfill::new(999, Vec::new(), Vec::new()).unwrap();

        // Act
        let params = fulfill.protocol_params("request-8");

        // Assert
        assert_eq!(params["responsePhrase"], "Unknown");
    }

    #[test]
    fn route_continue_serializes_every_override_for_cdp() {
        // Arrange
        let options = RouteContinue::new(
            Some("https://example.test/next".into()),
            Some("POST".into()),
            Some(vec![("x-test".into(), "yes".into())]),
            Some(vec![0, 1, 2, 3, 4]),
        )
        .unwrap();

        // Act
        let params = options.protocol_params("request-7");

        // Assert
        assert_eq!(
            params,
            serde_json::json!({
                "requestId": "request-7",
                "url": "https://example.test/next",
                "method": "POST",
                "headers": [{ "name": "x-test", "value": "yes" }],
                "postData": "AAECAwQ=",
            })
        );
    }

    #[cfg(feature = "route-redirects")]
    #[test]
    fn route_continue_preserves_browser_cookie_when_other_headers_are_overridden() {
        // Arrange
        let options = RouteContinue::new(
            None,
            None,
            Some(vec![
                ("x-test".into(), "yes".into()),
                ("COOKIE".into(), "overridden=value".into()),
            ]),
            None,
        )
        .unwrap();
        let original = vec![("cookie".into(), "original=value".into())];

        // Act
        let options = options.with_original_cookie_header(&original).unwrap();

        // Assert
        assert_eq!(
            options.protocol_params("request-7")["headers"],
            serde_json::json!([
                { "name": "x-test", "value": "yes" },
                { "name": "cookie", "value": "original=value" },
            ])
        );
    }

    #[cfg(feature = "route-redirects")]
    #[test]
    fn route_continue_discards_injected_cookie_without_original_cookie() {
        // Arrange
        let options = RouteContinue::new(
            None,
            None,
            Some(vec![("Cookie".into(), "injected=value".into())]),
            None,
        )
        .unwrap();

        // Act
        let options = options.with_original_cookie_header(&[]).unwrap();

        // Assert
        assert_eq!(
            options.protocol_params("request-7")["headers"],
            serde_json::json!([])
        );
    }

    #[cfg(feature = "route-redirects")]
    #[test]
    fn route_continue_revalidates_capacity_after_restoring_original_cookie() {
        // Arrange
        let options = RouteContinue::new(
            None,
            None,
            Some(
                (0..MAX_HEADER_COUNT)
                    .map(|index| (format!("x-{index}"), String::new()))
                    .collect(),
            ),
            None,
        )
        .unwrap();
        let original = vec![("cookie".into(), "original=value".into())];

        // Act
        let result = options.with_original_cookie_header(&original);

        // Assert
        assert!(
            matches!(result, Err(HostError::InvalidArgument(message)) if message.contains("headers exceed count limit"))
        );
    }

    #[test]
    fn route_fallback_overrides_compose_field_by_field() {
        // Arrange
        let inherited = RouteContinue::new(
            Some("https://example.test/next".into()),
            Some("POST".into()),
            Some(vec![("x-first".into(), "yes".into())]),
            Some(b"first".to_vec()),
        )
        .unwrap();
        let newer = RouteContinue::new(
            None,
            Some("PATCH".into()),
            Some(vec![("x-second".into(), "yes".into())]),
            None,
        )
        .unwrap();

        // Act
        let combined = inherited.merged(newer);

        // Assert
        assert_eq!(combined.url.as_deref(), Some("https://example.test/next"));
        assert_eq!(combined.method.as_deref(), Some("PATCH"));
        assert_eq!(
            combined.headers.as_deref(),
            Some([("x-second".into(), "yes".into())].as_slice())
        );
        assert_eq!(combined.post_data.as_deref(), Some(b"first".as_slice()));
    }

    #[cfg(feature = "route-redirects")]
    #[test]
    fn redirect_continuation_reuses_explicit_headers_without_cookie() {
        // Arrange
        let mut state = state_with_pending(Vec::new(), Vec::new());
        let mut pending = pending_call(1, ObjectId::new(8));
        pending.request_id = "fetch-1".into();
        pending.network_request_id = Some("network-1".into());
        retain_redirect_header_override(
            &mut state,
            &pending,
            &[
                ("x-test".into(), "yes".into()),
                ("Cookie".into(), "secret=value".into()),
            ],
        )
        .unwrap();

        // Act
        let continued = advance_redirect_header_override(
            &mut state,
            "session",
            "fetch-1",
            "fetch-2",
            Some("network-1"),
        )
        .unwrap();

        // Assert
        assert_eq!(
            continued.protocol_params("fetch-2"),
            json!({
                "requestId": "fetch-2",
                "headers": [{ "name": "x-test", "value": "yes" }],
            })
        );
        assert_eq!(
            state.redirect_header_overrides[0].fetch_request_id.as_ref(),
            "fetch-2"
        );
    }

    #[cfg(feature = "route-redirects")]
    #[test]
    fn untracked_redirect_continues_without_overrides() {
        // Arrange
        let mut state = state_with_pending(Vec::new(), Vec::new());

        // Act
        let continued = advance_redirect_header_override(
            &mut state,
            "session",
            "fetch-1",
            "fetch-2",
            Some("network-1"),
        )
        .unwrap();

        // Assert
        assert_eq!(
            continued.protocol_params("fetch-2"),
            json!({ "requestId": "fetch-2" })
        );
    }

    #[cfg(feature = "route-redirects")]
    #[test]
    fn redirect_header_ownership_has_a_fixed_capacity() {
        // Arrange
        let mut state = state_with_pending(Vec::new(), Vec::new());
        for index in 0..MAX_REDIRECT_HEADER_OVERRIDES {
            let mut pending = pending_call(index as u64 + 1, ObjectId::new(8));
            pending.request_id = format!("fetch-{index}").into_boxed_str();
            retain_redirect_header_override(
                &mut state,
                &pending,
                &[("x-test".into(), "yes".into())],
            )
            .unwrap();
        }
        let mut overflow = pending_call(999, ObjectId::new(8));
        overflow.request_id = "overflow".into();

        // Act
        let outcome = retain_redirect_header_override(
            &mut state,
            &overflow,
            &[("x-test".into(), "yes".into())],
        );

        // Assert
        assert!(matches!(outcome, Err(HostError::ResourceExhausted(_))));
        assert_eq!(
            state.redirect_header_overrides.len(),
            MAX_REDIRECT_HEADER_OVERRIDES
        );
    }

    #[test]
    fn route_continue_rejects_inputs_beyond_its_declared_boundary() {
        // Arrange
        let oversized_url = "u".repeat(MAX_URL_BYTES + 1);
        let oversized_method = "M".repeat(MAX_METHOD_BYTES + 1);
        let invalid_headers = vec![("bad header".into(), "value".into())];
        let oversized_post_data = vec![0; MAX_POST_DATA_BYTES + 1];

        // Act
        let url = RouteContinue::new(Some(oversized_url), None, None, None);
        let method = RouteContinue::new(None, Some(oversized_method), None, None);
        let headers = RouteContinue::new(None, None, Some(invalid_headers), None);
        let post_data = RouteContinue::new(None, None, None, Some(oversized_post_data));

        // Assert
        assert!(url.is_err());
        assert!(method.is_err());
        assert!(headers.is_err());
        assert!(post_data.is_err());
    }

    #[test]
    fn route_continue_rejects_cross_protocol_url_overrides() {
        // Arrange
        let options =
            RouteContinue::new(Some("file:///tmp/does-not-exist".into()), None, None, None)
                .unwrap();

        // Act
        let result = options.validate_protocol("https");

        // Assert
        assert_eq!(
            result.unwrap_err().to_string(),
            "New URL must have same protocol as overridden URL"
        );
    }
}
