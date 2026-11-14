//! Optional bounded Page HTTP lifecycle subscriptions.
//!
//! Chromium normalization and Page/session registration come from the existing
//! network-events composition. This host leaf adds only a fixed waiter table
//! and caller-polled queues for language bindings; it starts no task and owns
//! no response-body or routing authority.

#[cfg(any(
    feature = "request-history",
    feature = "request-events",
    feature = "response-events"
))]
use std::collections::BTreeSet;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

#[cfg(feature = "response-security-details")]
pub use pwright_backend_cdp::CdpResponseSecurityDetails;
use pwright_backend_cdp::{CdpNetworkEventBatch, CdpNetworkEventEffect};
#[cfg(feature = "request-frame")]
use pwright_backend_cdp_topology::CdpTopology;
use pwright_cap_network_events::{
    NETWORK_REQUEST_EVENT_TAG, NETWORK_REQUEST_FAILED_EVENT_TAG,
    NETWORK_REQUEST_FINISHED_EVENT_TAG, NETWORK_RESPONSE_EVENT_TAG,
};
use pwright_composition_chromium::{
    ChromiumNetworkLifecycleRuntime, ChromiumPageSubscriptionActivator,
};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
#[cfg(feature = "request-history")]
use pwright_core::{ResourceLedger, ResourceRequest};
#[cfg(feature = "request-timing")]
pub use pwright_network_model::RequestTiming;
use pwright_network_model::{HttpHeader, RequestInitializer, ResponseInitializer};
use pwright_runtime_control::SubscriptionActivator;

use crate::{CommandRuntime, HostError, HostSession, LiveSession};

const MAX_WAITERS: usize = 32;
const MAX_EVENTS_PER_WAITER: usize = 16;
const MAX_QUEUED_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_REQUEST_SIZE_OWNERS: usize = 64;
const STORAGE_BYTES: u64 = MAX_QUEUED_BYTES as u64 + 64 * 1024;
const SCOPE: ResourceScope = ResourceScope::extension(135, 1);

#[cfg(feature = "response-events")]
pub const MAX_RESPONSE_EVENT_CONTEXTS: usize = 16;
#[cfg(feature = "response-events")]
pub const MAX_RESPONSE_EVENT_NOTICES: usize = 256;
#[cfg(feature = "request-events")]
pub const MAX_REQUEST_EVENT_CONTEXTS: usize = 16;
#[cfg(feature = "request-events")]
pub const MAX_REQUEST_EVENT_NOTICES: usize = 256;
#[cfg(feature = "response-finished")]
pub const MAX_RESPONSE_FINISHED_OWNERS: usize = 256;
#[cfg(feature = "response-finished")]
pub const RESPONSE_FINISHED_STORAGE_BYTES: u64 = 64 * 1024;
#[cfg(not(feature = "response-finished"))]
const RESPONSE_FINISHED_STORAGE_BYTES: u64 = 0;

#[cfg(feature = "request-history")]
const MAX_REQUEST_HISTORY_PAGES: usize = 16;
#[cfg(feature = "request-history")]
pub const MAX_REQUEST_HISTORY_ITEMS_PER_PAGE: usize = 100;
#[cfg(feature = "request-history")]
pub const MAX_REQUEST_HISTORY_NATIVE_ID_BYTES: usize = 4 * 1024;
#[cfg(feature = "request-history")]
pub const MAX_REQUEST_HISTORY_OWNERS_PER_PAGE: u64 = 256;
#[cfg(feature = "request-history")]
pub const MAX_REQUEST_HISTORY_OWNERS: u64 =
    MAX_REQUEST_HISTORY_PAGES as u64 * MAX_REQUEST_HISTORY_OWNERS_PER_PAGE;
#[cfg(feature = "request-history")]
pub const MAX_REQUEST_HISTORY_BYTES: u64 = 64 * 1024 * 1024;
#[cfg(feature = "request-history")]
const REQUEST_HISTORY_STORAGE_BYTES: u64 = (std::mem::size_of::<RequestHistory>()
    + std::mem::size_of::<RequestHistoryState>()
    + MAX_REQUEST_HISTORY_PAGES
        * (std::mem::size_of::<PageRequestHistory>()
            + MAX_REQUEST_HISTORY_ITEMS_PER_PAGE * std::mem::size_of::<Arc<RequestHistoryOwner>>()))
    as u64;
#[cfg(not(feature = "request-history"))]
const REQUEST_HISTORY_STORAGE_BYTES: u64 = 0;

pub(crate) fn requirements() -> ResourceRequirements {
    let requirements = ResourceRequirements::none().with_class(
        ResourceClass::BackendStateBytes,
        STORAGE_BYTES
            + if cfg!(feature = "request-history") {
                REQUEST_HISTORY_STORAGE_BYTES
            } else {
                0
            }
            + RESPONSE_FINISHED_STORAGE_BYTES,
        STORAGE_BYTES
            + if cfg!(feature = "request-history") {
                REQUEST_HISTORY_STORAGE_BYTES
            } else {
                0
            }
            + RESPONSE_FINISHED_STORAGE_BYTES,
    );
    #[cfg(feature = "request-history")]
    let requirements = requirements
        .with_class(
            ResourceClass::EventItem,
            MAX_REQUEST_HISTORY_OWNERS,
            MAX_REQUEST_HISTORY_OWNERS_PER_PAGE,
        )
        .with_class(
            ResourceClass::EventBytes,
            MAX_REQUEST_HISTORY_BYTES,
            MAX_REQUEST_HISTORY_BYTES,
        );
    let metadata_keys = 1usize;
    let metadata_values = 1usize;
    #[cfg(feature = "request-history")]
    let (metadata_keys, metadata_values) = (
        metadata_keys + MAX_REQUEST_HISTORY_PAGES,
        metadata_values + MAX_REQUEST_HISTORY_OWNERS as usize,
    );
    #[cfg(feature = "request-events")]
    let (metadata_keys, metadata_values) = (
        metadata_keys + MAX_REQUEST_EVENT_CONTEXTS,
        metadata_values + MAX_REQUEST_EVENT_NOTICES,
    );
    #[cfg(feature = "response-events")]
    let (metadata_keys, metadata_values) = (
        metadata_keys + MAX_RESPONSE_EVENT_CONTEXTS,
        metadata_values + MAX_RESPONSE_EVENT_NOTICES,
    );
    #[cfg(feature = "response-finished")]
    let (metadata_keys, metadata_values) = (
        metadata_keys + 1,
        metadata_values + MAX_RESPONSE_FINISHED_OWNERS,
    );
    requirements.with_metadata(metadata_keys, metadata_values)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkEventKind {
    Request,
    Response,
    RequestFinished,
    RequestFailed,
}

impl NetworkEventKind {
    pub fn parse(value: &str) -> Result<Self, HostError> {
        match value {
            "request" => Ok(Self::Request),
            "response" => Ok(Self::Response),
            "requestfinished" => Ok(Self::RequestFinished),
            "requestfailed" => Ok(Self::RequestFailed),
            _ => Err(HostError::InvalidArgument(format!(
                "unsupported network event {value:?}"
            ))),
        }
    }

    const fn tag(self) -> pwright_core::EventTag {
        match self {
            Self::Request => NETWORK_REQUEST_EVENT_TAG,
            Self::Response => NETWORK_RESPONSE_EVENT_TAG,
            Self::RequestFinished => NETWORK_REQUEST_FINISHED_EVENT_TAG,
            Self::RequestFailed => NETWORK_REQUEST_FAILED_EVENT_TAG,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Response => "response",
            Self::RequestFinished => "requestfinished",
            Self::RequestFailed => "requestfailed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NetworkWaitScope {
    Page(ObjectId),
    #[cfg(feature = "context-network-events")]
    Context(ObjectId),
}

impl NetworkWaitScope {
    const fn object_id(self) -> ObjectId {
        match self {
            Self::Page(id) => id,
            #[cfg(feature = "context-network-events")]
            Self::Context(id) => id,
        }
    }

    fn matches(self, page: ObjectId, _context: Option<ObjectId>) -> bool {
        match self {
            Self::Page(id) => id == page,
            #[cfg(feature = "context-network-events")]
            Self::Context(id) => Some(id) == _context,
        }
    }

    fn is_active(self, pages: &[ObjectId], _contexts: &[ObjectId]) -> bool {
        match self {
            Self::Page(id) => pages.contains(&id),
            #[cfg(feature = "context-network-events")]
            Self::Context(id) => _contexts.contains(&id),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkRequestRecord {
    pub id: String,
    pub native_request_id: Option<String>,
    pub page_id: String,
    pub url: String,
    pub method: String,
    pub resource_type: String,
    pub headers: Vec<(String, String)>,
    pub post_data: Option<Vec<u8>>,
    pub is_navigation_request: bool,
    pub frame_id: Option<String>,
    pub frame_available: bool,
    pub redirected_from: Option<String>,
    #[cfg(feature = "request-timing")]
    pub timing: RequestTiming,
    pub size_owner: Option<Arc<RequestSizeOwner>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestSizes {
    pub request_body_size: i64,
    pub request_headers_size: i64,
    pub response_body_size: i64,
    pub response_headers_size: i64,
    pub transfer_size: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RequestSizeState {
    Pending,
    Complete(RequestSizes),
    Failed(String),
    Closed,
}

pub struct RequestSizeOwner {
    request: ObjectId,
    page: ObjectId,
    request_body_size: i64,
    state: Mutex<RequestSizeState>,
    notify: tokio::sync::Notify,
    _slot: tokio::sync::OwnedSemaphorePermit,
}

impl std::fmt::Debug for RequestSizeOwner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RequestSizeOwner")
            .field("request", &self.request)
            .field("page", &self.page)
            .finish_non_exhaustive()
    }
}

impl PartialEq for RequestSizeOwner {
    fn eq(&self, other: &Self) -> bool {
        self.request == other.request && self.page == other.page
    }
}

impl Eq for RequestSizeOwner {}

impl RequestSizeOwner {
    fn new(
        page: ObjectId,
        request: ObjectId,
        value: &NetworkRequestRecord,
        slot: tokio::sync::OwnedSemaphorePermit,
    ) -> Self {
        Self {
            request,
            page,
            request_body_size: value
                .post_data
                .as_ref()
                .map_or(0, Vec::len)
                .try_into()
                .expect("network post-data bound fits i64"),
            state: Mutex::new(RequestSizeState::Pending),
            notify: tokio::sync::Notify::new(),
            _slot: slot,
        }
    }

    pub async fn sizes(&self) -> Result<RequestSizes, HostError> {
        loop {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let state = self.state.lock().map_err(HostError::backend)?.clone();
            match state {
                RequestSizeState::Pending => notified.as_mut().await,
                RequestSizeState::Complete(value) => return Ok(value),
                RequestSizeState::Failed(message) => return Err(HostError::Backend(message)),
                RequestSizeState::Closed => return Err(HostError::TargetClosed),
            }
        }
    }

    fn finish(
        &self,
        request: &NetworkRequestRecord,
        response: Option<&NetworkResponseRecord>,
        encoded_body_bytes: Option<u64>,
    ) {
        let outcome = response.map_or_else(
            || Err("Unable to fetch sizes for failed request".to_owned()),
            |response| self.completed_sizes(request, response, encoded_body_bytes),
        );
        self.settle(outcome);
    }

    fn fail(&self) {
        self.settle(Err("Unable to fetch sizes for failed request".to_owned()));
    }

    fn close(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if matches!(*state, RequestSizeState::Pending) {
            *state = RequestSizeState::Closed;
            self.notify.notify_waiters();
        }
    }

    fn completed_sizes(
        &self,
        request: &NetworkRequestRecord,
        response: &NetworkResponseRecord,
        encoded_body_bytes: Option<u64>,
    ) -> Result<RequestSizes, String> {
        let request_headers_size = request_headers_size(request)?;
        let response_headers_size = response_headers_size(response)?;
        let fallback_body_size = response
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.trim().parse::<i64>().ok())
            .unwrap_or(0);
        let (response_body_size, transfer_size) = if let Some(encoded) = encoded_body_bytes {
            let transfer_size = i64::try_from(encoded)
                .map_err(|_| "encoded response size exceeds i64".to_owned())?;
            (transfer_size - response_headers_size, transfer_size)
        } else {
            let transfer_size = response_headers_size
                .checked_add(fallback_body_size)
                .ok_or_else(|| "response transfer size overflow".to_owned())?;
            (fallback_body_size, transfer_size)
        };
        Ok(RequestSizes {
            request_body_size: self.request_body_size,
            request_headers_size,
            response_body_size,
            response_headers_size,
            transfer_size,
        })
    }

    fn settle(&self, outcome: Result<RequestSizes, String>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if !matches!(*state, RequestSizeState::Pending) {
            return;
        }
        *state = match outcome {
            Ok(value) => RequestSizeState::Complete(value),
            Err(message) => RequestSizeState::Failed(message),
        };
        self.notify.notify_waiters();
    }
}

#[cfg(feature = "response-finished")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResponseFinishedState {
    Pending,
    Finished,
    Failed,
    Closed,
}

#[cfg(feature = "response-finished")]
pub struct ResponseFinishedOwner {
    request: ObjectId,
    page: ObjectId,
    state: Mutex<ResponseFinishedState>,
    notify: tokio::sync::Notify,
    _slot: tokio::sync::OwnedSemaphorePermit,
}

#[cfg(feature = "response-finished")]
impl std::fmt::Debug for ResponseFinishedOwner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResponseFinishedOwner")
            .field("request", &self.request)
            .field("page", &self.page)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "response-finished")]
impl PartialEq for ResponseFinishedOwner {
    fn eq(&self, other: &Self) -> bool {
        self.request == other.request && self.page == other.page
    }
}

#[cfg(feature = "response-finished")]
impl Eq for ResponseFinishedOwner {}

#[cfg(feature = "response-finished")]
impl ResponseFinishedOwner {
    fn new(page: ObjectId, request: ObjectId, slot: tokio::sync::OwnedSemaphorePermit) -> Self {
        Self {
            request,
            page,
            state: Mutex::new(ResponseFinishedState::Pending),
            notify: tokio::sync::Notify::new(),
            _slot: slot,
        }
    }

    pub async fn finished(&self) -> Result<(), HostError> {
        loop {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let state = *self.state.lock().map_err(HostError::backend)?;
            match state {
                ResponseFinishedState::Pending => notified.as_mut().await,
                ResponseFinishedState::Finished => return Ok(()),
                ResponseFinishedState::Failed => {
                    return Err(HostError::Backend("Response failed to finish".into()));
                }
                ResponseFinishedState::Closed => return Err(HostError::TargetClosed),
            }
        }
    }

    fn finish(&self) {
        self.settle(ResponseFinishedState::Finished);
    }

    fn fail(&self) {
        self.settle(ResponseFinishedState::Failed);
    }

    fn close(&self) {
        self.settle(ResponseFinishedState::Closed);
    }

    fn settle(&self, outcome: ResponseFinishedState) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if !matches!(*state, ResponseFinishedState::Pending) {
            return;
        }
        *state = outcome;
        self.notify.notify_waiters();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkResponseRecord {
    pub id: String,
    pub request_id: String,
    pub url: String,
    pub status: i64,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub from_service_worker: bool,
    #[cfg(feature = "response-security-details")]
    pub security_details: Option<CdpResponseSecurityDetails>,
    #[cfg(feature = "request-timing")]
    pub timing: RequestTiming,
    pub body: Option<Arc<[u8]>>,
    #[cfg(feature = "response-finished")]
    pub finished_owner: Option<Arc<ResponseFinishedOwner>>,
}

#[cfg(feature = "request-events")]
#[derive(Clone, Debug)]
pub struct RequestNotice {
    pub sequence: u64,
    /// Publication order shared with response notices when both capabilities
    /// are selected. A merged consumer uses this to preserve CDP order.
    #[cfg(feature = "response-events")]
    pub order_sequence: u64,
    pub page: ObjectId,
    pub kind: NetworkEventKind,
    pub request: NetworkRequestRecord,
    pub failure_text: Option<String>,
    #[cfg(feature = "request-history")]
    pub history_owner: Option<Arc<RequestHistoryOwner>>,
}

#[cfg(feature = "request-events")]
impl RequestNotice {
    fn retained_bytes(&self) -> usize {
        self.request
            .retained_bytes()
            .saturating_add(self.failure_text.as_ref().map_or(0, String::len))
    }

    /// Conservative ceiling for converting one immutable notice through a
    /// foreign-language boundary. The native queue charges source retention
    /// separately until the notice is removed.
    pub fn foreign_copy_bytes(&self) -> u64 {
        self.request
            .foreign_copy_bytes()
            .saturating_add(
                u64::try_from(self.failure_text.as_ref().map_or(0, String::len))
                    .unwrap_or(u64::MAX)
                    .saturating_mul(2),
            )
            .saturating_add(4 * 1024)
    }
}

#[cfg(feature = "response-events")]
#[derive(Clone, Debug)]
pub struct ResponseNotice {
    pub sequence: u64,
    #[cfg(feature = "request-events")]
    pub order_sequence: u64,
    pub page: ObjectId,
    pub native_request_id: Option<String>,
    pub response: NetworkResponseRecord,
    #[cfg(feature = "request-history")]
    pub history_owner: Option<Arc<RequestHistoryOwner>>,
}

#[cfg(all(feature = "request-events", feature = "response-events"))]
#[derive(Clone, Debug)]
pub enum NetworkNotice {
    Request(RequestNotice),
    Response(ResponseNotice),
}

#[cfg(feature = "response-events")]
impl ResponseNotice {
    fn retained_bytes(&self) -> usize {
        self.native_request_id
            .as_ref()
            .map_or(0, String::len)
            .saturating_add(self.response.retained_bytes())
    }

    /// Conservative ceiling for converting one immutable response notice
    /// through a foreign-language boundary. Source retention remains charged
    /// until this notice is removed from the bounded native queue.
    pub fn foreign_copy_bytes(&self) -> u64 {
        let response = self.response.foreign_copy_bytes().saturating_add(
            u64::try_from(self.native_request_id.as_ref().map_or(0, String::len))
                .unwrap_or(u64::MAX)
                .saturating_mul(2),
        );
        #[cfg(feature = "request-history")]
        let response = response.saturating_add(
            self.history_owner
                .as_ref()
                .map_or(0, |request| request.foreign_copy_bytes()),
        );
        response.saturating_add(4 * 1024)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkEventRecord {
    Request(NetworkRequestRecord),
    Response(NetworkResponseRecord),
    RequestFinished {
        request: NetworkRequestRecord,
        response: Option<NetworkResponseRecord>,
        encoded_body_bytes: Option<u64>,
    },
    RequestFailed {
        request: NetworkRequestRecord,
        response: Option<NetworkResponseRecord>,
        failure_text: String,
        canceled: bool,
    },
}

#[derive(Clone, Debug)]
pub struct NetworkWaitRecord {
    event: NetworkEventRecord,
    #[cfg(feature = "request-history")]
    history_owner: Option<Arc<RequestHistoryOwner>>,
}

impl NetworkWaitRecord {
    fn retained_bytes(&self) -> usize {
        self.event.retained_bytes()
    }

    #[cfg(feature = "request-history")]
    pub fn into_parts(self) -> (NetworkEventRecord, Option<Arc<RequestHistoryOwner>>) {
        (self.event, self.history_owner)
    }

    #[cfg(not(feature = "request-history"))]
    pub fn into_event(self) -> NetworkEventRecord {
        self.event
    }
}

impl NetworkEventRecord {
    pub const fn kind(&self) -> NetworkEventKind {
        match self {
            Self::Request(_) => NetworkEventKind::Request,
            Self::Response(_) => NetworkEventKind::Response,
            Self::RequestFinished { .. } => NetworkEventKind::RequestFinished,
            Self::RequestFailed { .. } => NetworkEventKind::RequestFailed,
        }
    }

    fn retained_bytes(&self) -> usize {
        match self {
            Self::Request(request) => request.retained_bytes(),
            Self::Response(response) => response.retained_bytes(),
            Self::RequestFinished {
                request, response, ..
            } => request.retained_bytes().saturating_add(
                response
                    .as_ref()
                    .map_or(0, NetworkResponseRecord::retained_bytes),
            ),
            Self::RequestFailed {
                request,
                response,
                failure_text,
                ..
            } => request
                .retained_bytes()
                .saturating_add(
                    response
                        .as_ref()
                        .map_or(0, NetworkResponseRecord::retained_bytes),
                )
                .saturating_add(failure_text.len()),
        }
    }
}

impl NetworkRequestRecord {
    fn retained_bytes(&self) -> usize {
        let retained = self
            .id
            .len()
            .saturating_add(self.native_request_id.as_ref().map_or(0, String::len))
            .saturating_add(self.page_id.len())
            .saturating_add(self.url.len())
            .saturating_add(self.method.len())
            .saturating_add(self.resource_type.len())
            .saturating_add(header_bytes(&self.headers))
            .saturating_add(self.post_data.as_ref().map_or(0, Vec::len))
            .saturating_add(self.redirected_from.as_ref().map_or(0, String::len));
        #[cfg(feature = "request-timing")]
        let retained = retained.saturating_add(std::mem::size_of::<RequestTiming>());
        retained
    }

    /// Conservative ceiling for cloning this immutable request through one
    /// foreign-language conversion, including container and string overhead.
    pub fn foreign_copy_bytes(&self) -> u64 {
        let request_bytes = u64::try_from(self.retained_bytes()).unwrap_or(u64::MAX);
        let header_slots = u64::try_from(self.headers.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(256);
        request_bytes
            .saturating_mul(4)
            .saturating_add(header_slots)
            .saturating_add(8 * 1024)
    }
}

impl NetworkResponseRecord {
    fn retained_bytes(&self) -> usize {
        let retained = self
            .id
            .len()
            .saturating_add(self.request_id.len())
            .saturating_add(self.url.len())
            .saturating_add(self.status_text.len())
            .saturating_add(header_bytes(&self.headers))
            .saturating_add(self.body.as_ref().map_or(0, |body| body.len()));
        #[cfg(feature = "request-timing")]
        let retained = retained.saturating_add(std::mem::size_of::<RequestTiming>());
        #[cfg(feature = "response-security-details")]
        let retained = retained.saturating_add(
            self.security_details
                .as_ref()
                .map_or(0, CdpResponseSecurityDetails::retained_bytes),
        );
        retained
    }

    /// Conservative ceiling for cloning response metadata through one
    /// foreign-language conversion, including header containers and strings.
    pub fn foreign_copy_bytes(&self) -> u64 {
        // Response bodies stay behind the independently selected body owner;
        // a response-event conversion publishes metadata only.
        let metadata_bytes = self
            .id
            .len()
            .saturating_add(self.request_id.len())
            .saturating_add(self.url.len())
            .saturating_add(self.status_text.len())
            .saturating_add(header_bytes(&self.headers));
        #[cfg(feature = "request-timing")]
        let metadata_bytes = metadata_bytes.saturating_add(std::mem::size_of::<RequestTiming>());
        #[cfg(feature = "response-security-details")]
        let metadata_bytes = metadata_bytes.saturating_add(
            self.security_details
                .as_ref()
                .map_or(0, CdpResponseSecurityDetails::retained_bytes),
        );
        let response_bytes = u64::try_from(metadata_bytes).unwrap_or(u64::MAX);
        let header_slots = u64::try_from(self.headers.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(256);
        response_bytes
            .saturating_mul(4)
            .saturating_add(header_slots)
            .saturating_add(8 * 1024)
    }
}

#[cfg(feature = "request-history")]
#[derive(Clone, Debug)]
enum RequestHistoryOutcome {
    Pending,
    Response(Box<NetworkResponseRecord>),
    NoResponse,
    Closed,
}

#[cfg(feature = "request-history")]
#[derive(Clone, Debug, Eq, PartialEq)]
enum RequestTerminalOutcome {
    Pending,
    Finished,
    Failed(String),
    Closed,
}

#[cfg(feature = "request-history")]
struct RequestHistoryOwnerState {
    outcome: RequestHistoryOutcome,
    terminal: RequestTerminalOutcome,
    lease: ResourceLease,
}

#[cfg(feature = "request-history")]
pub struct RequestHistoryOwner {
    page: ObjectId,
    request_id: ObjectId,
    request: NetworkRequestRecord,
    redirected_from: Option<Arc<RequestHistoryOwner>>,
    state: Mutex<RequestHistoryOwnerState>,
    notify: tokio::sync::Notify,
}

#[cfg(feature = "request-history")]
impl std::fmt::Debug for RequestHistoryOwner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RequestHistoryOwner")
            .field("page", &self.page)
            .field("request_id", &self.request_id)
            .field("request", &self.request)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "request-history")]
impl RequestHistoryOwner {
    fn requests(retained_bytes: usize) -> Result<[ResourceRequest; 2], HostError> {
        let bytes = u64::try_from(retained_bytes)
            .map_err(|_| HostError::ResourceExhausted("request history size exceeds u64".into()))?;
        if bytes > MAX_REQUEST_HISTORY_BYTES {
            return Err(HostError::ResourceExhausted(
                "request history owner exceeds the byte capacity".into(),
            ));
        }
        Ok([
            ResourceRequest::new(ResourceClass::EventItem, 1),
            ResourceRequest::new(ResourceClass::EventBytes, bytes),
        ])
    }

    fn new(
        page: ObjectId,
        request_id: ObjectId,
        request: NetworkRequestRecord,
        redirected_from: Option<Arc<RequestHistoryOwner>>,
        lease: ResourceLease,
    ) -> Self {
        Self {
            page,
            request_id,
            request,
            redirected_from,
            state: Mutex::new(RequestHistoryOwnerState {
                outcome: RequestHistoryOutcome::Pending,
                terminal: RequestTerminalOutcome::Pending,
                lease,
            }),
            notify: tokio::sync::Notify::new(),
        }
    }

    pub const fn request(&self) -> &NetworkRequestRecord {
        &self.request
    }

    /// The preceding redirect hop remains available for as long as this
    /// request is retained, even after the Page's bounded history evicts it.
    /// Each predecessor keeps its existing resource lease; a chain therefore
    /// cannot bypass the configured per-Page owner and byte limits.
    pub fn redirected_from(&self) -> Option<Arc<Self>> {
        self.redirected_from.clone()
    }

    /// Conservative ceiling for cloning this immutable request through one
    /// foreign-language conversion. The retained owner keeps the source bytes
    /// charged separately until that conversion finishes.
    pub fn foreign_copy_bytes(&self) -> u64 {
        self.request.foreign_copy_bytes()
    }

    pub async fn response(&self) -> Result<Option<NetworkResponseRecord>, HostError> {
        loop {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let outcome = self
                .state
                .lock()
                .map_err(HostError::backend)?
                .outcome
                .clone();
            match outcome {
                RequestHistoryOutcome::Pending => notified.as_mut().await,
                RequestHistoryOutcome::Response(response) => return Ok(Some(*response)),
                RequestHistoryOutcome::NoResponse => return Ok(None),
                RequestHistoryOutcome::Closed => return Err(HostError::TargetClosed),
            }
        }
    }

    /// Return the observed response only for this exact Page-scoped native
    /// document request. A URL or status match alone is never an identity
    /// proof (successive navigations may have identical values).
    #[cfg(any(
        feature = "navigation-response-finished",
        feature = "response-security-details"
    ))]
    pub(crate) async fn observed_navigation_response(
        &self,
        native_request_id: &str,
        expected_url: &str,
        expected_status: i64,
    ) -> Result<Option<NetworkResponseRecord>, HostError> {
        if !self.request.is_navigation_request
            || self.request.native_request_id.as_deref() != Some(native_request_id)
        {
            return Ok(None);
        }
        let Some(response) = self.response().await? else {
            return Ok(None);
        };
        if response.request_id != self.request.id
            || response.url != expected_url
            || response.status != expected_status
        {
            return Ok(None);
        }
        Ok(Some(response))
    }

    #[cfg(feature = "navigation-response-finished")]
    pub(crate) async fn navigation_finished_owner(
        &self,
        native_request_id: &str,
        expected_url: &str,
        expected_status: i64,
    ) -> Result<Option<Arc<ResponseFinishedOwner>>, HostError> {
        let Some(response) = self
            .observed_navigation_response(native_request_id, expected_url, expected_status)
            .await?
        else {
            return Ok(None);
        };
        let Some(owner) = response.finished_owner else {
            return Ok(None);
        };
        if owner.page != self.page || owner.request != self.request_id {
            return Ok(None);
        }
        Ok(Some(owner))
    }

    pub fn failure_text(&self) -> Result<Option<String>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        Ok(match &state.terminal {
            RequestTerminalOutcome::Failed(text) => Some(text.clone()),
            _ => None,
        })
    }

    fn mark_terminal(&self, failure_text: Option<&str>) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if !matches!(state.terminal, RequestTerminalOutcome::Pending) {
            return Ok(());
        }
        if let Some(text) = failure_text {
            let retained_bytes = self
                .request
                .retained_bytes()
                .checked_add(match &state.outcome {
                    RequestHistoryOutcome::Response(response) => response.retained_bytes(),
                    _ => 0,
                })
                .and_then(|bytes| bytes.checked_add(text.len()))
                .ok_or_else(|| {
                    HostError::ResourceExhausted("request failure size overflow".into())
                })?;
            state
                .lease
                .try_replace(
                    ResourceScope::Object(self.page),
                    &Self::requests(retained_bytes)?,
                )
                .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
            state.terminal = RequestTerminalOutcome::Failed(text.to_owned());
        } else {
            state.terminal = RequestTerminalOutcome::Finished;
        }
        Ok(())
    }

    fn settle(&self, response: Option<NetworkResponseRecord>) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if !matches!(state.outcome, RequestHistoryOutcome::Pending) {
            return Ok(());
        }
        if let Some(response) = response {
            let retained_bytes = self
                .request
                .retained_bytes()
                .checked_add(response.retained_bytes())
                .ok_or_else(|| {
                    HostError::ResourceExhausted("request history size overflow".into())
                })?;
            state
                .lease
                .try_replace(
                    ResourceScope::Object(self.page),
                    &Self::requests(retained_bytes)?,
                )
                .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
            state.outcome = RequestHistoryOutcome::Response(Box::new(response));
        } else {
            state.outcome = RequestHistoryOutcome::NoResponse;
        }
        self.notify.notify_waiters();
        Ok(())
    }

    fn close(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if matches!(state.outcome, RequestHistoryOutcome::Pending) {
            state.outcome = RequestHistoryOutcome::Closed;
            self.notify.notify_waiters();
        }
        if matches!(state.terminal, RequestTerminalOutcome::Pending) {
            state.terminal = RequestTerminalOutcome::Closed;
        }
    }
}

#[cfg(feature = "request-history")]
struct PageRequestHistory {
    page: Option<ObjectId>,
    requests: VecDeque<Arc<RequestHistoryOwner>>,
}

#[cfg(feature = "request-history")]
impl PageRequestHistory {
    fn new() -> Result<Self, HostError> {
        let mut requests = VecDeque::new();
        requests
            .try_reserve_exact(MAX_REQUEST_HISTORY_ITEMS_PER_PAGE)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(Self {
            page: None,
            requests,
        })
    }

    fn clear(&mut self) {
        for owner in &self.requests {
            owner.close();
        }
        self.requests.clear();
        self.page = None;
    }
}

#[cfg(feature = "request-history")]
struct RequestHistoryState {
    pages: Vec<PageRequestHistory>,
    active: BTreeMap<ObjectId, Weak<RequestHistoryOwner>>,
    closed: bool,
}

#[cfg(feature = "request-history")]
struct RequestHistory {
    resources: ResourceLedger,
    state: Mutex<RequestHistoryState>,
    changed: tokio::sync::Notify,
}

#[cfg(feature = "request-history")]
impl RequestHistory {
    fn new(resources: ResourceLedger) -> Result<Self, HostError> {
        let mut pages = Vec::new();
        pages
            .try_reserve_exact(MAX_REQUEST_HISTORY_PAGES)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        for _ in 0..MAX_REQUEST_HISTORY_PAGES {
            pages.push(PageRequestHistory::new()?);
        }
        Ok(Self {
            resources,
            state: Mutex::new(RequestHistoryState {
                pages,
                active: BTreeMap::new(),
                closed: false,
            }),
            changed: tokio::sync::Notify::new(),
        })
    }

    fn record_request(
        &self,
        page: ObjectId,
        request_id: ObjectId,
        request: &NetworkRequestRecord,
    ) -> Result<Arc<RequestHistoryOwner>, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        state.active.retain(|_, owner| owner.strong_count() != 0);
        if state.active.contains_key(&request_id) {
            return Err(HostError::Backend(
                "duplicate active request history identity".into(),
            ));
        }
        let page_index = state
            .pages
            .iter()
            .position(|history| history.page == Some(page))
            .or_else(|| {
                state
                    .pages
                    .iter()
                    .position(|history| history.page.is_none())
            })
            .ok_or_else(|| {
                HostError::ResourceExhausted(format!(
                    "request history exceeds {MAX_REQUEST_HISTORY_PAGES} live Pages"
                ))
            })?;
        let redirected_from = request.redirected_from.as_deref().and_then(|id| {
            state.pages[page_index]
                .requests
                .iter()
                .rev()
                .find(|owner| owner.request().id == id)
                .cloned()
        });
        let requests = RequestHistoryOwner::requests(request.retained_bytes())?;
        let evicted =
            if state.pages[page_index].requests.len() == MAX_REQUEST_HISTORY_ITEMS_PER_PAGE {
                state.pages[page_index].requests.pop_front()
            } else {
                None
            };
        let lease = match self
            .resources
            .reserve_many(ResourceScope::Object(page), &requests)
        {
            Ok(lease) => lease,
            Err(error) => {
                if let Some(evicted) = evicted {
                    state.pages[page_index].requests.push_front(evicted);
                }
                return Err(HostError::ResourceExhausted(error.to_string()));
            }
        };
        let owner = Arc::new(RequestHistoryOwner::new(
            page,
            request_id,
            request.clone(),
            redirected_from,
            lease,
        ));
        state.pages[page_index].page = Some(page);
        state.pages[page_index]
            .requests
            .push_back(Arc::clone(&owner));
        state.active.insert(request_id, Arc::downgrade(&owner));
        drop(state);
        self.changed.notify_waiters();
        Ok(owner)
    }

    async fn owner_by_native_id<F>(
        &self,
        page: ObjectId,
        native_request_id: &str,
        stopped: &tokio_util::sync::CancellationToken,
        page_closed: F,
    ) -> Result<Arc<RequestHistoryOwner>, HostError>
    where
        F: Fn() -> Result<bool, HostError>,
    {
        loop {
            let notified = self.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let owner = {
                let state = self.state.lock().map_err(HostError::backend)?;
                if state.closed {
                    return Err(HostError::TargetClosed);
                }
                state
                    .pages
                    .iter()
                    .find(|history| history.page == Some(page))
                    .and_then(|history| {
                        // CDP reuses the same native RequestId across redirect
                        // hops. The most recent portable request is the final
                        // navigation response's owner.
                        history.requests.iter().rev().find(|owner| {
                            owner.request().native_request_id.as_deref() == Some(native_request_id)
                        })
                    })
                    .cloned()
            };
            if let Some(owner) = owner {
                return Ok(owner);
            }
            if page_closed()? {
                return Err(HostError::TargetClosed);
            }
            tokio::select! {
                () = notified.as_mut() => {}
                () = stopped.cancelled() => return Err(HostError::TargetClosed),
            }
        }
    }

    async fn response_by_native_id<F>(
        &self,
        page: ObjectId,
        native_request_id: &str,
        stopped: &tokio_util::sync::CancellationToken,
        page_closed: F,
    ) -> Result<Option<NetworkResponseRecord>, HostError>
    where
        F: Fn() -> Result<bool, HostError>,
    {
        let owner = self
            .owner_by_native_id(page, native_request_id, stopped, page_closed)
            .await?;
        tokio::select! {
            response = owner.response() => response,
            () = stopped.cancelled() => Err(HostError::TargetClosed),
        }
    }

    fn settle(
        &self,
        request_id: ObjectId,
        response: Option<NetworkResponseRecord>,
        terminal: bool,
    ) -> Result<Option<Arc<RequestHistoryOwner>>, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        let owner = state.active.get(&request_id).and_then(Weak::upgrade);
        if terminal {
            state.active.remove(&request_id);
        }
        drop(state);
        if let Some(owner) = owner {
            owner.settle(response)?;
            return Ok(Some(owner));
        }
        Ok(None)
    }

    fn publish(
        &self,
        page: ObjectId,
        request_id: ObjectId,
        event: &NetworkEventRecord,
    ) -> Result<Option<Arc<RequestHistoryOwner>>, HostError> {
        match event {
            NetworkEventRecord::Request(request) => {
                self.record_request(page, request_id, request).map(Some)
            }
            NetworkEventRecord::Response(response) => {
                self.settle(request_id, Some(response.clone()), false)
            }
            NetworkEventRecord::RequestFinished { response, .. } => {
                let owner = self.settle(request_id, response.clone(), true)?;
                if let Some(owner) = owner.as_ref() {
                    owner.mark_terminal(None)?;
                }
                Ok(owner)
            }
            NetworkEventRecord::RequestFailed {
                response,
                failure_text,
                ..
            } => {
                let owner = self.settle(request_id, response.clone(), true)?;
                if let Some(owner) = owner.as_ref() {
                    owner.mark_terminal(Some(failure_text))?;
                }
                Ok(owner)
            }
        }
    }

    fn snapshot(&self, page: ObjectId) -> Result<Vec<Arc<RequestHistoryOwner>>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        Ok(state
            .pages
            .iter()
            .find(|history| history.page == Some(page))
            .map(|history| history.requests.iter().cloned().collect())
            .unwrap_or_default())
    }

    fn reconcile(&self, active_pages: &[ObjectId]) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        for history in &mut state.pages {
            if history
                .page
                .is_some_and(|page| !active_pages.contains(&page))
            {
                history.clear();
            }
        }
        state.active.retain(|_, owner| {
            let Some(owner) = owner.upgrade() else {
                return false;
            };
            if active_pages.contains(&owner.page) {
                true
            } else {
                owner.close();
                false
            }
        });
        drop(state);
        self.changed.notify_waiters();
        Ok(())
    }

    fn close(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.closed = true;
        for history in &mut state.pages {
            history.clear();
        }
        for owner in state.active.values().filter_map(Weak::upgrade) {
            owner.close();
        }
        state.active.clear();
        drop(state);
        self.changed.notify_waiters();
    }
}

fn header_bytes(headers: &[(String, String)]) -> usize {
    headers.iter().fold(0, |total, (name, value)| {
        total.saturating_add(name.len()).saturating_add(value.len())
    })
}

fn headers(values: &[HttpHeader]) -> Vec<(String, String)> {
    values
        .iter()
        .map(|header| (header.name.clone(), header.value.clone()))
        .collect()
}

fn js_len(value: &str) -> Result<i64, String> {
    i64::try_from(value.encode_utf16().count()).map_err(|_| "network size exceeds i64".to_owned())
}

fn checked_size_add(total: i64, value: i64) -> Result<i64, String> {
    total
        .checked_add(value)
        .ok_or_else(|| "network size overflow".to_owned())
}

fn request_headers_size(value: &NetworkRequestRecord) -> Result<i64, String> {
    let url = url::Url::parse(&value.url)
        .map_err(|error| format!("cannot calculate request size for URL: {error}"))?;
    let mut size = 4;
    size = checked_size_add(size, js_len(&value.method)?)?;
    size = checked_size_add(size, js_len(url.path())?)?;
    size = checked_size_add(size, 8)?;
    for (name, value) in &value.headers {
        size = checked_size_add(size, js_len(name)?)?;
        size = checked_size_add(size, js_len(value)?)?;
        size = checked_size_add(size, 4)?;
    }
    Ok(size)
}

fn response_headers_size(value: &NetworkResponseRecord) -> Result<i64, String> {
    let mut size = 4;
    size = checked_size_add(size, 8)?;
    size = checked_size_add(size, 3)?;
    size = checked_size_add(size, js_len(&value.status_text)?)?;
    for (name, value) in &value.headers {
        size = checked_size_add(size, js_len(name)?)?;
        size = checked_size_add(size, js_len(value)?)?;
        size = checked_size_add(size, 4)?;
    }
    checked_size_add(size, 2)
}

fn request(
    value: &RequestInitializer,
    frame: Option<ObjectId>,
    frame_available: bool,
    native_request_id: Option<&str>,
) -> NetworkRequestRecord {
    NetworkRequestRecord {
        id: value.request.get().to_string(),
        native_request_id: native_request_id.map(str::to_owned),
        page_id: value.parent.get().to_string(),
        url: value.url.clone(),
        method: value.method.clone(),
        resource_type: value.resource_type.clone(),
        headers: headers(&value.headers),
        post_data: value.post_data.clone(),
        is_navigation_request: value.is_navigation_request,
        frame_id: frame.map(|id| id.get().to_string()),
        frame_available,
        redirected_from: value.redirected_from.map(|id| id.get().to_string()),
        #[cfg(feature = "request-timing")]
        timing: value
            .timing
            .as_deref()
            .copied()
            .unwrap_or_else(RequestTiming::pending),
        size_owner: None,
    }
}

#[cfg(feature = "request-post-data-json")]
pub fn request_post_data_json(
    value: &NetworkRequestRecord,
) -> Result<Option<serde_json::Value>, String> {
    let Some(bytes) = value.post_data.as_deref() else {
        return Ok(None);
    };
    if bytes.is_empty() {
        return Ok(None);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| {
        format!(
            "POST data is not a valid JSON object: {}",
            String::from_utf8_lossy(bytes)
        )
    })?;
    let content_type = value
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.as_str())
        .unwrap_or_default();
    if content_type.contains("application/x-www-form-urlencoded") {
        let mut object = serde_json::Map::new();
        for (name, field) in url::form_urlencoded::parse(bytes) {
            object.insert(
                name.into_owned(),
                serde_json::Value::String(field.into_owned()),
            );
        }
        return Ok(Some(serde_json::Value::Object(object)));
    }
    serde_json::from_str(text)
        .map(Some)
        .map_err(|_| format!("POST data is not a valid JSON object: {text}"))
}

fn request_identity(effect: &CdpNetworkEventEffect) -> Option<ObjectId> {
    match effect {
        CdpNetworkEventEffect::Request(event) => Some(event.payload().request),
        CdpNetworkEventEffect::Response(event) => Some(event.payload().request),
        CdpNetworkEventEffect::RequestFinished(event) => Some(event.payload().request.request),
        CdpNetworkEventEffect::RequestFailed(event) => Some(event.payload().request.request),
    }
}

fn is_favicon_request(
    effect: &CdpNetworkEventEffect,
    known: &BTreeMap<ObjectId, ObjectId>,
) -> bool {
    match effect {
        CdpNetworkEventEffect::Request(event) => {
            let request = event.payload();
            request.url.ends_with("/favicon.ico")
                || request
                    .redirected_from
                    .is_some_and(|request| known.contains_key(&request))
        }
        CdpNetworkEventEffect::Response(event) => known.contains_key(&event.payload().request),
        CdpNetworkEventEffect::RequestFinished(event) => {
            known.contains_key(&event.payload().request.request)
        }
        CdpNetworkEventEffect::RequestFailed(event) => {
            known.contains_key(&event.payload().request.request)
        }
    }
}

fn is_terminal(effect: &CdpNetworkEventEffect) -> bool {
    matches!(
        effect,
        CdpNetworkEventEffect::RequestFinished(_) | CdpNetworkEventEffect::RequestFailed(_)
    )
}

fn attach_size_owner(event: &mut NetworkEventRecord, owner: Arc<RequestSizeOwner>) {
    match event {
        NetworkEventRecord::Request(request)
        | NetworkEventRecord::RequestFinished { request, .. }
        | NetworkEventRecord::RequestFailed { request, .. } => {
            request.size_owner = Some(owner);
        }
        NetworkEventRecord::Response(_) => {}
    }
}

#[cfg(feature = "response-finished")]
fn event_has_response(event: &NetworkEventRecord) -> bool {
    match event {
        NetworkEventRecord::Response(_) => true,
        NetworkEventRecord::RequestFinished { response, .. }
        | NetworkEventRecord::RequestFailed { response, .. } => response.is_some(),
        NetworkEventRecord::Request(_) => false,
    }
}

#[cfg(feature = "response-finished")]
fn attach_response_finished_owner(
    event: &mut NetworkEventRecord,
    owner: Arc<ResponseFinishedOwner>,
) {
    match event {
        NetworkEventRecord::Response(response) => response.finished_owner = Some(owner),
        NetworkEventRecord::RequestFinished { response, .. }
        | NetworkEventRecord::RequestFailed { response, .. } => {
            if let Some(response) = response {
                response.finished_owner = Some(owner);
            }
        }
        NetworkEventRecord::Request(_) => {}
    }
}

#[cfg(feature = "response-events")]
const fn response_context_subscribes(kind: NetworkEventKind) -> bool {
    match kind {
        NetworkEventKind::Response => true,
        NetworkEventKind::RequestFinished | NetworkEventKind::RequestFailed => {
            cfg!(feature = "response-finished")
        }
        NetworkEventKind::Request => false,
    }
}

#[cfg(all(not(feature = "response-events"), feature = "request-events"))]
const fn response_context_subscribes(_kind: NetworkEventKind) -> bool {
    false
}

#[cfg(all(
    feature = "request-events",
    any(feature = "request-history", feature = "response-events")
))]
const fn request_context_subscribes(kind: NetworkEventKind) -> bool {
    matches!(
        kind,
        NetworkEventKind::Request
            | NetworkEventKind::RequestFinished
            | NetworkEventKind::RequestFailed
    )
}

#[cfg(all(not(feature = "request-events"), feature = "response-events"))]
const fn request_context_subscribes(_kind: NetworkEventKind) -> bool {
    false
}

fn response_record(value: &ResponseInitializer, body: Option<Arc<[u8]>>) -> NetworkResponseRecord {
    NetworkResponseRecord {
        id: value.response.get().to_string(),
        request_id: value.request.get().to_string(),
        url: value.url.clone(),
        status: value.status,
        status_text: value.status_text.clone(),
        headers: headers(&value.headers),
        from_service_worker: value.from_service_worker,
        #[cfg(feature = "response-security-details")]
        security_details: None,
        #[cfg(feature = "request-timing")]
        timing: value
            .timing
            .as_deref()
            .copied()
            .unwrap_or_else(RequestTiming::pending),
        body,
        #[cfg(feature = "response-finished")]
        finished_owner: None,
    }
}

fn record(
    effect: &CdpNetworkEventEffect,
    fulfilled_body: Option<Arc<[u8]>>,
    frame: Option<ObjectId>,
    frame_available: bool,
    native_request_id: Option<&str>,
) -> NetworkEventRecord {
    match effect {
        CdpNetworkEventEffect::Request(event) => NetworkEventRecord::Request(request(
            event.payload(),
            frame,
            frame_available,
            native_request_id,
        )),
        CdpNetworkEventEffect::Response(event) => {
            NetworkEventRecord::Response(response_record(event.payload(), fulfilled_body))
        }
        CdpNetworkEventEffect::RequestFinished(event) => {
            let value = event.payload();
            NetworkEventRecord::RequestFinished {
                request: request(&value.request, frame, frame_available, native_request_id),
                response: value
                    .response
                    .as_ref()
                    .map(|response| response_record(response, None)),
                encoded_body_bytes: value.encoded_body_bytes,
            }
        }
        CdpNetworkEventEffect::RequestFailed(event) => {
            let value = event.payload();
            NetworkEventRecord::RequestFailed {
                request: request(&value.request, frame, frame_available, native_request_id),
                response: value
                    .response
                    .as_ref()
                    .map(|response| response_record(response, None)),
                failure_text: value.failure_text.clone(),
                canceled: value.canceled,
            }
        }
    }
}

#[cfg(feature = "response-security-details")]
fn attach_response_security(
    effect: &CdpNetworkEventEffect,
    event: &mut NetworkEventRecord,
    batch: &CdpNetworkEventBatch,
) {
    let response_id = match effect {
        CdpNetworkEventEffect::Response(value) => Some(value.payload().response),
        CdpNetworkEventEffect::RequestFinished(value) => value
            .payload()
            .response
            .as_ref()
            .map(|response| response.response),
        CdpNetworkEventEffect::RequestFailed(value) => value
            .payload()
            .response
            .as_ref()
            .map(|response| response.response),
        CdpNetworkEventEffect::Request(_) => None,
    };
    let Some(details) = response_id.and_then(|id| batch.response_security(id)) else {
        return;
    };
    match event {
        NetworkEventRecord::Response(response) => response.security_details = Some(details.clone()),
        NetworkEventRecord::RequestFinished { response, .. }
        | NetworkEventRecord::RequestFailed { response, .. } => {
            if let Some(response) = response {
                response.security_details = Some(details.clone());
            }
        }
        NetworkEventRecord::Request(_) => {}
    }
}

struct Waiter {
    scope: NetworkWaitScope,
    kind: NetworkEventKind,
    timeout_ms: u32,
    deadline: Option<tokio::time::Instant>,
    queue: VecDeque<NetworkWaitRecord>,
    queued_bytes: usize,
    retired: bool,
    notify: Arc<tokio::sync::Notify>,
}

struct State {
    next_id: u64,
    waiters: BTreeMap<u64, Waiter>,
    size_owners: BTreeMap<ObjectId, Weak<RequestSizeOwner>>,
    #[cfg(feature = "response-finished")]
    response_finished_owners: BTreeMap<ObjectId, Weak<ResponseFinishedOwner>>,
    favicon_requests: BTreeMap<ObjectId, ObjectId>,
    #[cfg(feature = "request-history")]
    history_contexts: BTreeSet<ObjectId>,
    #[cfg(feature = "request-events")]
    request_contexts: BTreeSet<ObjectId>,
    #[cfg(feature = "response-events")]
    response_contexts: BTreeSet<ObjectId>,
    #[cfg(feature = "request-events")]
    request_notices: VecDeque<RequestNotice>,
    #[cfg(feature = "request-events")]
    next_request_notice_sequence: u64,
    #[cfg(feature = "request-events")]
    last_popped_request_notice_sequence: u64,
    #[cfg(feature = "response-events")]
    response_notices: VecDeque<ResponseNotice>,
    #[cfg(feature = "response-events")]
    next_response_notice_sequence: u64,
    #[cfg(feature = "response-events")]
    last_popped_response_notice_sequence: u64,
    #[cfg(all(feature = "request-events", feature = "response-events"))]
    next_network_notice_sequence: u64,
    queued_bytes: usize,
    closed: bool,
}

#[cfg(feature = "request-events")]
impl State {
    fn enqueue_request_notice(&mut self, mut notice: RequestNotice) -> Result<(), HostError> {
        let bytes = notice.retained_bytes();
        if self.request_notices.len() >= MAX_REQUEST_EVENT_NOTICES
            || self.queued_bytes.saturating_add(bytes) > MAX_QUEUED_BYTES
        {
            return Err(HostError::ResourceExhausted(
                "request event notice capacity exceeded".into(),
            ));
        }
        let sequence = self.next_request_notice_sequence;
        self.next_request_notice_sequence = sequence.checked_add(1).ok_or_else(|| {
            HostError::ResourceExhausted("request event notice sequence exhausted".into())
        })?;
        notice.sequence = sequence;
        #[cfg(feature = "response-events")]
        {
            notice.order_sequence = self.next_network_notice_sequence;
            self.next_network_notice_sequence = self
                .next_network_notice_sequence
                .checked_add(1)
                .ok_or_else(|| {
                    HostError::ResourceExhausted("network notice sequence exhausted".into())
                })?;
        }
        self.request_notices.push_back(notice);
        self.queued_bytes = self.queued_bytes.saturating_add(bytes);
        Ok(())
    }

    fn pop_request_notice(&mut self) -> Option<RequestNotice> {
        let notice = self.request_notices.pop_front()?;
        self.last_popped_request_notice_sequence = notice.sequence;
        self.queued_bytes = self.queued_bytes.saturating_sub(notice.retained_bytes());
        Some(notice)
    }

    fn request_notice_fence(&self) -> u64 {
        self.request_notices
            .back()
            .map_or(self.last_popped_request_notice_sequence, |notice| {
                notice.sequence
            })
    }

    fn reconcile_request_notices(&mut self, active: &[ObjectId]) {
        let mut retired_bytes = 0usize;
        self.request_notices.retain(|notice| {
            let retained = active.contains(&notice.page);
            if !retained {
                retired_bytes = retired_bytes.saturating_add(notice.retained_bytes());
            }
            retained
        });
        self.queued_bytes = self.queued_bytes.saturating_sub(retired_bytes);
    }

    fn clear_request_notices(&mut self) {
        let bytes = self.request_notices.iter().fold(0usize, |total, notice| {
            total.saturating_add(notice.retained_bytes())
        });
        self.request_notices.clear();
        self.queued_bytes = self.queued_bytes.saturating_sub(bytes);
    }
}

#[cfg(all(feature = "request-events", feature = "response-events"))]
impl State {
    fn pop_network_notice(&mut self) -> Option<NetworkNotice> {
        match (self.request_notices.front(), self.response_notices.front()) {
            (Some(request), Some(response)) if request.order_sequence < response.order_sequence => {
                self.pop_request_notice().map(NetworkNotice::Request)
            }
            (Some(_), Some(_)) | (None, Some(_)) => {
                self.pop_response_notice().map(NetworkNotice::Response)
            }
            (Some(_), None) => self.pop_request_notice().map(NetworkNotice::Request),
            (None, None) => None,
        }
    }
}

#[cfg(feature = "response-events")]
impl State {
    fn enqueue_response_notice(&mut self, mut notice: ResponseNotice) -> Result<(), HostError> {
        let bytes = notice.retained_bytes();
        if self.response_notices.len() >= MAX_RESPONSE_EVENT_NOTICES
            || self.queued_bytes.saturating_add(bytes) > MAX_QUEUED_BYTES
        {
            return Err(HostError::ResourceExhausted(
                "response event notice capacity exceeded".into(),
            ));
        }
        let sequence = self.next_response_notice_sequence;
        self.next_response_notice_sequence = sequence.checked_add(1).ok_or_else(|| {
            HostError::ResourceExhausted("response event notice sequence exhausted".into())
        })?;
        notice.sequence = sequence;
        #[cfg(feature = "request-events")]
        {
            notice.order_sequence = self.next_network_notice_sequence;
            self.next_network_notice_sequence = self
                .next_network_notice_sequence
                .checked_add(1)
                .ok_or_else(|| {
                    HostError::ResourceExhausted("network notice sequence exhausted".into())
                })?;
        }
        self.response_notices.push_back(notice);
        self.queued_bytes = self.queued_bytes.saturating_add(bytes);
        Ok(())
    }

    fn pop_response_notice(&mut self) -> Option<ResponseNotice> {
        let notice = self.response_notices.pop_front()?;
        self.last_popped_response_notice_sequence = notice.sequence;
        self.queued_bytes = self.queued_bytes.saturating_sub(notice.retained_bytes());
        Some(notice)
    }

    fn response_notice_fence(&self) -> u64 {
        self.response_notices
            .back()
            .map_or(self.last_popped_response_notice_sequence, |notice| {
                notice.sequence
            })
    }

    fn reconcile_response_notices(&mut self, active: &[ObjectId]) {
        let mut retired_bytes = 0usize;
        self.response_notices.retain(|notice| {
            let retained = active.contains(&notice.page);
            if !retained {
                retired_bytes = retired_bytes.saturating_add(notice.retained_bytes());
            }
            retained
        });
        self.queued_bytes = self.queued_bytes.saturating_sub(retired_bytes);
    }

    fn clear_response_notices(&mut self) {
        let bytes = self.response_notices.iter().fold(0usize, |total, notice| {
            total.saturating_add(notice.retained_bytes())
        });
        self.response_notices.clear();
        self.queued_bytes = self.queued_bytes.saturating_sub(bytes);
    }
}

pub(crate) struct Installation {
    transitions: tokio::sync::Mutex<()>,
    state: Mutex<State>,
    activator: ChromiumPageSubscriptionActivator,
    size_owner_slots: Arc<tokio::sync::Semaphore>,
    #[cfg(feature = "response-finished")]
    response_finished_owner_slots: Arc<tokio::sync::Semaphore>,
    #[cfg(feature = "request-history")]
    history: RequestHistory,
    #[cfg(feature = "request-events")]
    request_notify: tokio::sync::Notify,
    #[cfg(feature = "response-events")]
    response_notify: tokio::sync::Notify,
    #[cfg(all(feature = "request-events", feature = "response-events"))]
    network_notify: tokio::sync::Notify,
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new(
        runtime: &CommandRuntime,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, HostError> {
        let ledger = resources
            .claim(requirements())
            .map_err(HostError::backend)?;
        let storage = ledger
            .reserve(
                SCOPE,
                ResourceClass::BackendStateBytes,
                STORAGE_BYTES + REQUEST_HISTORY_STORAGE_BYTES + RESPONSE_FINISHED_STORAGE_BYTES,
            )
            .map_err(HostError::backend)?;
        #[cfg(feature = "request-history")]
        let history = RequestHistory::new(ledger.clone())?;
        #[cfg(feature = "request-events")]
        let mut request_notices = VecDeque::new();
        #[cfg(feature = "request-events")]
        request_notices
            .try_reserve_exact(MAX_REQUEST_EVENT_NOTICES)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        #[cfg(feature = "response-events")]
        let mut response_notices = VecDeque::new();
        #[cfg(feature = "response-events")]
        response_notices
            .try_reserve_exact(MAX_RESPONSE_EVENT_NOTICES)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(Self {
            transitions: tokio::sync::Mutex::new(()),
            state: Mutex::new(State {
                next_id: 1,
                waiters: BTreeMap::new(),
                size_owners: BTreeMap::new(),
                #[cfg(feature = "response-finished")]
                response_finished_owners: BTreeMap::new(),
                favicon_requests: BTreeMap::new(),
                #[cfg(feature = "request-history")]
                history_contexts: BTreeSet::new(),
                #[cfg(feature = "request-events")]
                request_contexts: BTreeSet::new(),
                #[cfg(feature = "response-events")]
                response_contexts: BTreeSet::new(),
                #[cfg(feature = "request-events")]
                request_notices,
                #[cfg(feature = "request-events")]
                next_request_notice_sequence: 1,
                #[cfg(feature = "request-events")]
                last_popped_request_notice_sequence: 0,
                #[cfg(feature = "response-events")]
                response_notices,
                #[cfg(feature = "response-events")]
                next_response_notice_sequence: 1,
                #[cfg(feature = "response-events")]
                last_popped_response_notice_sequence: 0,
                #[cfg(all(feature = "request-events", feature = "response-events"))]
                next_network_notice_sequence: 1,
                queued_bytes: 0,
                closed: false,
            }),
            activator: ChromiumPageSubscriptionActivator::new(runtime.service().clone()),
            size_owner_slots: Arc::new(tokio::sync::Semaphore::new(MAX_REQUEST_SIZE_OWNERS)),
            #[cfg(feature = "response-finished")]
            response_finished_owner_slots: Arc::new(tokio::sync::Semaphore::new(
                MAX_RESPONSE_FINISHED_OWNERS,
            )),
            #[cfg(feature = "request-history")]
            history,
            #[cfg(feature = "request-events")]
            request_notify: tokio::sync::Notify::new(),
            #[cfg(feature = "response-events")]
            response_notify: tokio::sync::Notify::new(),
            #[cfg(all(feature = "request-events", feature = "response-events"))]
            network_notify: tokio::sync::Notify::new(),
            _storage: storage,
        })
    }

    #[cfg(feature = "request-history")]
    async fn enable_history_context(&self, context: ObjectId) -> Result<(), HostError> {
        let _transition = self.transitions.lock().await;
        let (_request_already_active, _response_already_active) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            if state.closed {
                return Err(HostError::TargetClosed);
            }
            if state.history_contexts.contains(&context) {
                return Ok(());
            }
            if state.history_contexts.len() == MAX_REQUEST_HISTORY_PAGES {
                return Err(HostError::ResourceExhausted(format!(
                    "request history exceeds {MAX_REQUEST_HISTORY_PAGES} live BrowserContexts"
                )));
            }
            #[cfg(feature = "request-events")]
            let request_active = state.request_contexts.contains(&context);
            #[cfg(not(feature = "request-events"))]
            let request_active = false;
            #[cfg(feature = "response-events")]
            let response_active = state.response_contexts.contains(&context);
            #[cfg(not(feature = "response-events"))]
            let response_active = false;
            (request_active, response_active)
        };
        let kinds = [
            NetworkEventKind::Request,
            NetworkEventKind::Response,
            NetworkEventKind::RequestFinished,
            NetworkEventKind::RequestFailed,
        ];
        let mut activated: [Option<NetworkEventKind>; 4] = [None; 4];
        let mut activated_count = 0;
        for kind in kinds {
            #[cfg(feature = "request-events")]
            let inherited_from_request =
                _request_already_active && request_context_subscribes(kind);
            #[cfg(not(feature = "request-events"))]
            let inherited_from_request = false;
            #[cfg(feature = "response-events")]
            let inherited_from_response =
                _response_already_active && response_context_subscribes(kind);
            #[cfg(not(feature = "response-events"))]
            let inherited_from_response = false;
            if inherited_from_request || inherited_from_response {
                continue;
            }
            if let Err(error) = self
                .activator
                .set_subscription(context, kind.tag(), true)
                .await
            {
                for kind in activated[..activated_count].iter().rev().flatten() {
                    let _ = self
                        .activator
                        .set_subscription(context, kind.tag(), false)
                        .await;
                }
                return Err(HostError::backend(error));
            }
            activated[activated_count] = Some(kind);
            activated_count += 1;
        }
        let inserted = {
            let mut state = self.state.lock().map_err(HostError::backend)?;
            if state.closed || state.history_contexts.len() == MAX_REQUEST_HISTORY_PAGES {
                false
            } else {
                state.history_contexts.insert(context);
                true
            }
        };
        if inserted {
            return Ok(());
        }
        for kind in activated[..activated_count].iter().rev().flatten() {
            let _ = self
                .activator
                .set_subscription(context, kind.tag(), false)
                .await;
        }
        Err(HostError::TargetClosed)
    }

    #[cfg(feature = "request-history")]
    async fn disable_history_context(&self, context: ObjectId) {
        let _transition = self.transitions.lock().await;
        let (_preserve_request, _preserve_response) = self
            .state
            .lock()
            .map(|mut state| {
                state.history_contexts.remove(&context);
                #[cfg(feature = "request-events")]
                let preserve_request = state.request_contexts.contains(&context);
                #[cfg(not(feature = "request-events"))]
                let preserve_request = false;
                #[cfg(feature = "response-events")]
                let preserve_response = state.response_contexts.contains(&context);
                #[cfg(not(feature = "response-events"))]
                let preserve_response = false;
                (preserve_request, preserve_response)
            })
            .unwrap_or((false, false));
        for kind in [
            NetworkEventKind::RequestFailed,
            NetworkEventKind::RequestFinished,
            NetworkEventKind::Response,
            NetworkEventKind::Request,
        ] {
            #[cfg(feature = "request-events")]
            let preserved_by_request = _preserve_request && request_context_subscribes(kind);
            #[cfg(not(feature = "request-events"))]
            let preserved_by_request = false;
            #[cfg(feature = "response-events")]
            let preserved_by_response = _preserve_response && response_context_subscribes(kind);
            #[cfg(not(feature = "response-events"))]
            let preserved_by_response = false;
            if preserved_by_request || preserved_by_response {
                continue;
            }
            let _ = self
                .activator
                .set_subscription(context, kind.tag(), false)
                .await;
        }
    }

    #[cfg(feature = "request-events")]
    async fn enable_request_context(&self, context: ObjectId) -> Result<(), HostError> {
        let _transition = self.transitions.lock().await;
        let (inherited_from_history, inherited_from_response) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            if state.closed {
                return Err(HostError::TargetClosed);
            }
            if state.request_contexts.contains(&context) {
                return Ok(());
            }
            if state.request_contexts.len() == MAX_REQUEST_EVENT_CONTEXTS {
                return Err(HostError::ResourceExhausted(format!(
                    "request events exceed {MAX_REQUEST_EVENT_CONTEXTS} live BrowserContexts"
                )));
            }
            #[cfg(not(feature = "request-history"))]
            let inherited_history = false;
            #[cfg(feature = "request-history")]
            let inherited_history = state.history_contexts.contains(&context);
            #[cfg(feature = "response-events")]
            let inherited_response = state.response_contexts.contains(&context);
            #[cfg(not(feature = "response-events"))]
            let inherited_response = false;
            (inherited_history, inherited_response)
        };
        let kinds = [
            NetworkEventKind::Request,
            NetworkEventKind::RequestFinished,
            NetworkEventKind::RequestFailed,
        ];
        let mut activated: [Option<NetworkEventKind>; 3] = [None; 3];
        let mut activated_count = 0;
        for kind in kinds {
            let inherited_from_response =
                inherited_from_response && response_context_subscribes(kind);
            if inherited_from_history || inherited_from_response {
                continue;
            }
            if let Err(error) = self
                .activator
                .set_subscription(context, kind.tag(), true)
                .await
            {
                for kind in activated[..activated_count].iter().rev().flatten() {
                    let _ = self
                        .activator
                        .set_subscription(context, kind.tag(), false)
                        .await;
                }
                return Err(HostError::backend(error));
            }
            activated[activated_count] = Some(kind);
            activated_count += 1;
        }
        let inserted = {
            let mut state = self.state.lock().map_err(HostError::backend)?;
            if state.closed || state.request_contexts.len() == MAX_REQUEST_EVENT_CONTEXTS {
                false
            } else {
                state.request_contexts.insert(context);
                true
            }
        };
        if inserted {
            return Ok(());
        }
        for kind in activated[..activated_count].iter().rev().flatten() {
            let _ = self
                .activator
                .set_subscription(context, kind.tag(), false)
                .await;
        }
        Err(HostError::TargetClosed)
    }

    #[cfg(feature = "request-events")]
    async fn disable_request_context(&self, context: ObjectId) {
        let _transition = self.transitions.lock().await;
        let (preserve_history, preserve_response) = self
            .state
            .lock()
            .map(|mut state| {
                state.request_contexts.remove(&context);
                #[cfg(feature = "request-history")]
                let preserve_history = state.history_contexts.contains(&context);
                #[cfg(not(feature = "request-history"))]
                let preserve_history = false;
                #[cfg(feature = "response-events")]
                let preserve_response = state.response_contexts.contains(&context);
                #[cfg(not(feature = "response-events"))]
                let preserve_response = false;
                (preserve_history, preserve_response)
            })
            .unwrap_or((false, false));
        for kind in [
            NetworkEventKind::RequestFailed,
            NetworkEventKind::RequestFinished,
            NetworkEventKind::Request,
        ] {
            if preserve_history || (preserve_response && response_context_subscribes(kind)) {
                continue;
            }
            let _ = self
                .activator
                .set_subscription(context, kind.tag(), false)
                .await;
        }
    }

    #[cfg(feature = "request-events")]
    pub(crate) async fn enable_initial_request_contexts(
        &self,
        contexts: &[ObjectId],
    ) -> Result<(), HostError> {
        let mut enabled = Vec::new();
        enabled
            .try_reserve_exact(contexts.len())
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        for context in contexts {
            if let Err(primary) = self.enable_request_context(*context).await {
                for context in enabled.into_iter().rev() {
                    self.disable_request_context(context).await;
                }
                return Err(primary);
            }
            enabled.push(*context);
        }
        Ok(())
    }

    #[cfg(feature = "request-history")]
    pub(crate) async fn enable_initial_history_contexts(
        &self,
        contexts: &[ObjectId],
    ) -> Result<(), HostError> {
        let mut enabled = Vec::new();
        enabled
            .try_reserve_exact(contexts.len())
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        for context in contexts {
            if let Err(primary) = self.enable_history_context(*context).await {
                for context in enabled.into_iter().rev() {
                    self.disable_history_context(context).await;
                }
                return Err(primary);
            }
            enabled.push(*context);
        }
        Ok(())
    }

    #[cfg(feature = "response-events")]
    async fn enable_response_context(&self, context: ObjectId) -> Result<(), HostError> {
        let _transition = self.transitions.lock().await;
        let (inherited_from_history, inherited_from_request) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            if state.closed {
                return Err(HostError::TargetClosed);
            }
            if state.response_contexts.contains(&context) {
                return Ok(());
            }
            if state.response_contexts.len() == MAX_RESPONSE_EVENT_CONTEXTS {
                return Err(HostError::ResourceExhausted(format!(
                    "response events exceed {MAX_RESPONSE_EVENT_CONTEXTS} live BrowserContexts"
                )));
            }
            #[cfg(feature = "request-history")]
            let inherited_history = state.history_contexts.contains(&context);
            #[cfg(not(feature = "request-history"))]
            let inherited_history = false;
            #[cfg(feature = "request-events")]
            let inherited_request = state.request_contexts.contains(&context);
            #[cfg(not(feature = "request-events"))]
            let inherited_request = false;
            (inherited_history, inherited_request)
        };
        let kinds = [
            NetworkEventKind::Response,
            NetworkEventKind::RequestFinished,
            NetworkEventKind::RequestFailed,
        ];
        let mut activated: [Option<NetworkEventKind>; 3] = [None; 3];
        let mut activated_count = 0;
        for kind in kinds {
            if !response_context_subscribes(kind)
                || inherited_from_history
                || (inherited_from_request && request_context_subscribes(kind))
            {
                continue;
            }
            if let Err(error) = self
                .activator
                .set_subscription(context, kind.tag(), true)
                .await
            {
                for kind in activated[..activated_count].iter().rev().flatten() {
                    let _ = self
                        .activator
                        .set_subscription(context, kind.tag(), false)
                        .await;
                }
                return Err(HostError::backend(error));
            }
            activated[activated_count] = Some(kind);
            activated_count += 1;
        }
        let inserted = {
            let mut state = self.state.lock().map_err(HostError::backend)?;
            if state.closed || state.response_contexts.len() == MAX_RESPONSE_EVENT_CONTEXTS {
                false
            } else {
                state.response_contexts.insert(context);
                true
            }
        };
        if inserted {
            return Ok(());
        }
        for kind in activated[..activated_count].iter().rev().flatten() {
            let _ = self
                .activator
                .set_subscription(context, kind.tag(), false)
                .await;
        }
        Err(HostError::TargetClosed)
    }

    #[cfg(feature = "response-events")]
    async fn disable_response_context(&self, context: ObjectId) {
        let _transition = self.transitions.lock().await;
        let (preserve_history, preserve_request) = self
            .state
            .lock()
            .map(|mut state| {
                state.response_contexts.remove(&context);
                #[cfg(feature = "request-history")]
                let preserve_history = state.history_contexts.contains(&context);
                #[cfg(not(feature = "request-history"))]
                let preserve_history = false;
                #[cfg(feature = "request-events")]
                let preserve_request = state.request_contexts.contains(&context);
                #[cfg(not(feature = "request-events"))]
                let preserve_request = false;
                (preserve_history, preserve_request)
            })
            .unwrap_or((false, false));
        for kind in [
            NetworkEventKind::RequestFailed,
            NetworkEventKind::RequestFinished,
            NetworkEventKind::Response,
        ] {
            if !response_context_subscribes(kind)
                || preserve_history
                || (preserve_request && request_context_subscribes(kind))
            {
                continue;
            }
            let _ = self
                .activator
                .set_subscription(context, kind.tag(), false)
                .await;
        }
    }

    #[cfg(feature = "response-events")]
    pub(crate) async fn enable_initial_response_contexts(
        &self,
        contexts: &[ObjectId],
    ) -> Result<(), HostError> {
        let mut enabled = Vec::new();
        enabled
            .try_reserve_exact(contexts.len())
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        for context in contexts {
            if let Err(primary) = self.enable_response_context(*context).await {
                for context in enabled.into_iter().rev() {
                    self.disable_response_context(context).await;
                }
                return Err(primary);
            }
            enabled.push(*context);
        }
        Ok(())
    }

    async fn arm(
        &self,
        scope: NetworkWaitScope,
        kind: NetworkEventKind,
        timeout_ms: u32,
    ) -> Result<u64, HostError> {
        let _transition = self.transitions.lock().await;
        let first = {
            let state = self.state.lock().map_err(HostError::backend)?;
            if state.closed {
                return Err(HostError::TargetClosed);
            }
            if state.waiters.len() >= MAX_WAITERS {
                return Err(HostError::ResourceExhausted(
                    "network waiter capacity exceeds 32".into(),
                ));
            }
            !state
                .waiters
                .values()
                .any(|waiter| waiter.scope == scope && waiter.kind == kind && !waiter.retired)
        };
        if first {
            self.activator
                .set_subscription(scope.object_id(), kind.tag(), true)
                .await
                .map_err(HostError::backend)?;
        }
        let id = {
            let mut state = self.state.lock().map_err(HostError::backend)?;
            if state.closed {
                None
            } else {
                let id = state.next_id;
                state.next_id = state.next_id.checked_add(1).ok_or_else(|| {
                    HostError::ResourceExhausted("network waiter IDs exhausted".into())
                })?;
                state.waiters.insert(
                    id,
                    Waiter {
                        scope,
                        kind,
                        timeout_ms,
                        deadline: (timeout_ms != 0).then(|| {
                            tokio::time::Instant::now()
                                + Duration::from_millis(u64::from(timeout_ms))
                        }),
                        queue: VecDeque::new(),
                        queued_bytes: 0,
                        retired: false,
                        notify: Arc::new(tokio::sync::Notify::new()),
                    },
                );
                Some(id)
            }
        };
        if let Some(id) = id {
            return Ok(id);
        }
        if first {
            let _ = self
                .activator
                .set_subscription(scope.object_id(), kind.tag(), false)
                .await;
        }
        Err(HostError::TargetClosed)
    }

    pub(crate) async fn cancel(&self, id: u64) -> Result<(), HostError> {
        let _transition = self.transitions.lock().await;
        let (scope, kind, last) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let waiter = state
                .waiters
                .get(&id)
                .ok_or_else(|| HostError::InvalidArgument("unknown network waiter ID".into()))?;
            let last = !state.waiters.iter().any(|(candidate, other)| {
                *candidate != id
                    && other.scope == waiter.scope
                    && other.kind == waiter.kind
                    && !other.retired
            });
            (waiter.scope, waiter.kind, last)
        };
        if last {
            self.activator
                .set_subscription(scope.object_id(), kind.tag(), false)
                .await
                .map_err(HostError::backend)?;
        }
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if let Some(waiter) = state.waiters.remove(&id) {
            state.queued_bytes = state.queued_bytes.saturating_sub(waiter.queued_bytes);
            waiter.notify.notify_one();
        }
        state
            .size_owners
            .retain(|_, owner| owner.strong_count() != 0);
        #[cfg(feature = "response-finished")]
        state
            .response_finished_owners
            .retain(|_, owner| owner.strong_count() != 0);
        Ok(())
    }

    pub(crate) async fn next(&self, id: u64) -> Result<NetworkWaitRecord, HostError> {
        loop {
            let (notify, deadline, timeout_ms) = {
                let mut state = self.state.lock().map_err(HostError::backend)?;
                if state.closed {
                    return Err(HostError::TargetClosed);
                }
                let waiter = state.waiters.get_mut(&id).ok_or_else(|| {
                    HostError::InvalidArgument("unknown network waiter ID".into())
                })?;
                if waiter.retired {
                    return Err(HostError::TargetClosed);
                }
                if let Some(event) = waiter.queue.pop_front() {
                    let bytes = event.retained_bytes();
                    waiter.queued_bytes = waiter.queued_bytes.saturating_sub(bytes);
                    state.queued_bytes = state.queued_bytes.saturating_sub(bytes);
                    return Ok(event);
                }
                (waiter.notify.clone(), waiter.deadline, waiter.timeout_ms)
            };
            match deadline {
                Some(deadline) => tokio::time::timeout_at(deadline, notify.notified())
                    .await
                    .map_err(|_| HostError::Timeout(timeout_ms))?,
                None => notify.notified().await,
            }
        }
    }

    #[cfg(feature = "request-events")]
    pub(crate) async fn next_request_notice(&self) -> Result<RequestNotice, HostError> {
        loop {
            let notified = self.request_notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            {
                let mut state = self.state.lock().map_err(HostError::backend)?;
                if state.closed {
                    return Err(HostError::TargetClosed);
                }
                if let Some(notice) = state.pop_request_notice() {
                    return Ok(notice);
                }
            }
            notified.as_mut().await;
        }
    }

    #[cfg(feature = "response-events")]
    pub(crate) async fn next_response_notice(&self) -> Result<ResponseNotice, HostError> {
        loop {
            let notified = self.response_notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            {
                let mut state = self.state.lock().map_err(HostError::backend)?;
                if state.closed {
                    return Err(HostError::TargetClosed);
                }
                if let Some(notice) = state.pop_response_notice() {
                    return Ok(notice);
                }
            }
            notified.as_mut().await;
        }
    }

    #[cfg(all(feature = "request-events", feature = "response-events"))]
    pub(crate) async fn next_network_notice(&self) -> Result<NetworkNotice, HostError> {
        loop {
            let notified = self.network_notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            {
                let mut state = self.state.lock().map_err(HostError::backend)?;
                if state.closed {
                    return Err(HostError::TargetClosed);
                }
                if let Some(notice) = state.pop_network_notice() {
                    return Ok(notice);
                }
            }
            notified.as_mut().await;
        }
    }

    pub(crate) fn notice_fence(&self) -> Result<(u64, u64), HostError> {
        #[cfg(any(feature = "request-events", feature = "response-events"))]
        let state = self.state.lock().map_err(HostError::backend)?;
        #[cfg(feature = "request-events")]
        let request = state.request_notice_fence();
        #[cfg(not(feature = "request-events"))]
        let request = 0;
        #[cfg(feature = "response-events")]
        let response = state.response_notice_fence();
        #[cfg(not(feature = "response-events"))]
        let response = 0;
        Ok((request, response))
    }

    pub(crate) fn publish(
        &self,
        batch: &CdpNetworkEventBatch,
        runtime: &CommandRuntime,
        #[cfg(feature = "request-frame")] topology: &CdpTopology,
        fulfilled_body: Option<Arc<[u8]>>,
        public_notices: bool,
    ) -> Result<(), HostError> {
        runtime
            .apply_network_lifecycle(batch.lifecycle())
            .map_err(HostError::backend)?;
        let page = batch.page().ok_or_else(|| {
            HostError::Backend("non-empty network batch has no Page source".into())
        })?;
        #[cfg(feature = "context-network-events")]
        let context = self
            .activator
            .services()
            .network_subscription_registry()
            .map(|subscriptions| subscriptions.browser_context_for_page(page))
            .transpose()
            .map_err(HostError::backend)?
            .flatten();
        #[cfg(not(feature = "context-network-events"))]
        let context = None;
        let mut state = self.state.lock().map_err(HostError::backend)?;
        let mut retired_favicons = Vec::new();
        let mut fulfilled_body = fulfilled_body;
        for effect in batch.events() {
            let request_id = request_identity(effect);
            #[cfg(feature = "request-frame")]
            let (frame, frame_available) = request_id
                .and_then(|request| batch.request_frame(request))
                .map(|descriptor| {
                    if !descriptor.available() {
                        return Ok((None, false));
                    }
                    let native_id = descriptor.native_id().ok_or_else(|| {
                        HostError::Backend(
                            "available network Request frame has no native identity".into(),
                        )
                    })?;
                    let frame = topology.frame_object_id(native_id).ok_or_else(|| {
                        HostError::Backend(
                            "available network Request frame is absent from topology".into(),
                        )
                    })?;
                    Ok((Some(frame), true))
                })
                .transpose()?
                .unwrap_or((None, false));
            #[cfg(not(feature = "request-frame"))]
            let frame = None;
            #[cfg(not(feature = "request-frame"))]
            let frame_available = true;
            if is_favicon_request(effect, &state.favicon_requests) {
                if matches!(effect, CdpNetworkEventEffect::Request(_)) {
                    state
                        .favicon_requests
                        .insert(request_id.expect("request event has an identity"), page);
                }
                if is_terminal(effect) {
                    retired_favicons.push(request_id.expect("terminal event has an identity"));
                }
                continue;
            }
            state
                .size_owners
                .retain(|_, owner| owner.strong_count() != 0);
            #[cfg(feature = "response-finished")]
            state
                .response_finished_owners
                .retain(|_, owner| owner.strong_count() != 0);
            let body = if matches!(effect, CdpNetworkEventEffect::Response(_)) {
                fulfilled_body.take()
            } else {
                None
            };
            let mut event = record(
                effect,
                body,
                frame,
                frame_available,
                batch.native_request_id(),
            );
            #[cfg(feature = "response-security-details")]
            attach_response_security(effect, &mut event, batch);
            let targets = if public_notices {
                state
                    .waiters
                    .iter()
                    .filter(|(_, waiter)| {
                        !waiter.retired
                            && waiter.scope.matches(page, context)
                            && waiter.kind == event.kind()
                    })
                    .map(|(id, _)| *id)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            #[cfg(feature = "response-finished")]
            {
                let mut owner = request_id.and_then(|request_id| {
                    state
                        .response_finished_owners
                        .get(&request_id)
                        .and_then(Weak::upgrade)
                });
                if let Some(request_id) = request_id
                    && owner.is_none()
                    && event_has_response(&event)
                {
                    let slot = Arc::clone(&self.response_finished_owner_slots)
                        .try_acquire_owned()
                        .map_err(|_| {
                            HostError::ResourceExhausted(format!(
                                "response-finished owner capacity exceeds {MAX_RESPONSE_FINISHED_OWNERS}"
                            ))
                        })?;
                    owner = Some(Arc::new(ResponseFinishedOwner::new(page, request_id, slot)));
                }
                if let (Some(request_id), Some(owner)) = (request_id, owner.as_ref()) {
                    attach_response_finished_owner(&mut event, Arc::clone(owner));
                    match &event {
                        NetworkEventRecord::Response(_) => {
                            state
                                .response_finished_owners
                                .insert(request_id, Arc::downgrade(owner));
                        }
                        NetworkEventRecord::RequestFinished { .. } => {
                            owner.finish();
                            state.response_finished_owners.remove(&request_id);
                        }
                        NetworkEventRecord::RequestFailed { .. } => {
                            owner.fail();
                            state.response_finished_owners.remove(&request_id);
                        }
                        NetworkEventRecord::Request(_) => {}
                    }
                }
            }
            let bytes = event.retained_bytes();
            #[cfg(feature = "request-events")]
            let request_notice_count = usize::from(
                public_notices
                    && matches!(
                        &event,
                        NetworkEventRecord::Request(_)
                            | NetworkEventRecord::RequestFinished { .. }
                            | NetworkEventRecord::RequestFailed { .. }
                    ),
            );
            #[cfg(not(feature = "request-events"))]
            let request_notice_count = 0usize;
            #[cfg(feature = "response-events")]
            let response_notice_count =
                usize::from(public_notices && matches!(&event, NetworkEventRecord::Response(_)));
            #[cfg(not(feature = "response-events"))]
            let response_notice_count = 0usize;
            #[cfg(feature = "request-events")]
            let request_notice_capacity_exhausted = request_notice_count != 0
                && state.request_notices.len() >= MAX_REQUEST_EVENT_NOTICES;
            #[cfg(not(feature = "request-events"))]
            let request_notice_capacity_exhausted = false;
            #[cfg(feature = "response-events")]
            let response_notice_capacity_exhausted = response_notice_count != 0
                && state.response_notices.len() >= MAX_RESPONSE_EVENT_NOTICES;
            #[cfg(not(feature = "response-events"))]
            let response_notice_capacity_exhausted = false;
            let retained_copies = targets
                .len()
                .checked_add(request_notice_count)
                .and_then(|count| count.checked_add(response_notice_count))
                .ok_or_else(|| {
                    HostError::ResourceExhausted("network event queue item count overflow".into())
                })?;
            let total_bytes = bytes.checked_mul(retained_copies).ok_or_else(|| {
                HostError::ResourceExhausted("network event queue byte count overflow".into())
            })?;
            if targets.iter().any(|id| {
                state
                    .waiters
                    .get(id)
                    .is_some_and(|waiter| waiter.queue.len() >= MAX_EVENTS_PER_WAITER)
            }) || state.queued_bytes.saturating_add(total_bytes) > MAX_QUEUED_BYTES
                || request_notice_capacity_exhausted
                || response_notice_capacity_exhausted
            {
                return Err(HostError::ResourceExhausted(
                    "network event queue capacity exceeded".into(),
                ));
            }
            #[cfg(feature = "request-history")]
            let history_owner = request_id
                .map(|request_id| self.history.publish(page, request_id, &event))
                .transpose()?
                .flatten();
            #[cfg(feature = "response-events")]
            let response_notice = match &event {
                NetworkEventRecord::Response(response) => Some(ResponseNotice {
                    sequence: 0,
                    #[cfg(feature = "request-events")]
                    order_sequence: 0,
                    page,
                    native_request_id: batch.native_request_id().map(str::to_owned),
                    response: response.clone(),
                    #[cfg(feature = "request-history")]
                    history_owner: history_owner.clone(),
                }),
                _ => None,
            };
            let mut owner = request_id
                .and_then(|request_id| state.size_owners.get(&request_id).and_then(Weak::upgrade));
            if let Some(request_id) = request_id
                && owner.is_none()
                && (!targets.is_empty() || request_notice_count != 0)
                && matches!(
                    &event,
                    NetworkEventRecord::Request(_)
                        | NetworkEventRecord::RequestFinished { .. }
                        | NetworkEventRecord::RequestFailed { .. }
                )
            {
                let slot = Arc::clone(&self.size_owner_slots)
                    .try_acquire_owned()
                    .map_err(|_| {
                        HostError::ResourceExhausted(
                            "request-size owner capacity exceeds 64".into(),
                        )
                    })?;
                let request = match &event {
                    NetworkEventRecord::Request(request)
                    | NetworkEventRecord::RequestFinished { request, .. }
                    | NetworkEventRecord::RequestFailed { request, .. } => request,
                    NetworkEventRecord::Response(_) => unreachable!("matched request event"),
                };
                owner = Some(Arc::new(RequestSizeOwner::new(
                    page, request_id, request, slot,
                )));
            }
            if let (Some(request_id), Some(owner)) = (request_id, owner.as_ref()) {
                attach_size_owner(&mut event, Arc::clone(owner));
                match &event {
                    NetworkEventRecord::Request(_) => {
                        state.size_owners.insert(request_id, Arc::downgrade(owner));
                    }
                    NetworkEventRecord::RequestFinished {
                        request,
                        response,
                        encoded_body_bytes,
                    } => {
                        owner.finish(request, response.as_ref(), *encoded_body_bytes);
                        state.size_owners.remove(&request_id);
                    }
                    NetworkEventRecord::RequestFailed { .. } => {
                        owner.fail();
                        state.size_owners.remove(&request_id);
                    }
                    NetworkEventRecord::Response(_) => {}
                }
            }

            #[cfg(feature = "request-events")]
            let request_notice = match &event {
                NetworkEventRecord::Request(request) => Some(RequestNotice {
                    sequence: 0,
                    #[cfg(feature = "response-events")]
                    order_sequence: 0,
                    page,
                    kind: NetworkEventKind::Request,
                    request: request.clone(),
                    failure_text: None,
                    #[cfg(feature = "request-history")]
                    history_owner: history_owner.clone(),
                }),
                NetworkEventRecord::RequestFinished { request, .. } => Some(RequestNotice {
                    sequence: 0,
                    #[cfg(feature = "response-events")]
                    order_sequence: 0,
                    page,
                    kind: NetworkEventKind::RequestFinished,
                    request: request.clone(),
                    failure_text: None,
                    #[cfg(feature = "request-history")]
                    history_owner: history_owner.clone(),
                }),
                NetworkEventRecord::RequestFailed {
                    request,
                    failure_text,
                    ..
                } => Some(RequestNotice {
                    sequence: 0,
                    #[cfg(feature = "response-events")]
                    order_sequence: 0,
                    page,
                    kind: NetworkEventKind::RequestFailed,
                    request: request.clone(),
                    failure_text: Some(failure_text.clone()),
                    #[cfg(feature = "request-history")]
                    history_owner: history_owner.clone(),
                }),
                _ => None,
            };
            #[cfg(feature = "request-events")]
            if let Some(notice) = request_notice.filter(|_| public_notices) {
                state.enqueue_request_notice(notice)?;
                self.request_notify.notify_one();
                #[cfg(feature = "response-events")]
                self.network_notify.notify_one();
            }

            #[cfg(feature = "response-events")]
            if let Some(notice) = response_notice.filter(|_| public_notices) {
                state.enqueue_response_notice(notice)?;
                self.response_notify.notify_one();
                #[cfg(feature = "request-events")]
                self.network_notify.notify_one();
            }

            let queued_event = NetworkWaitRecord {
                event,
                #[cfg(feature = "request-history")]
                history_owner,
            };

            for id in targets {
                {
                    let waiter = state
                        .waiters
                        .get_mut(&id)
                        .expect("the snapshotted waiter remains present while locked");
                    waiter.queue.push_back(queued_event.clone());
                    waiter.queued_bytes = waiter.queued_bytes.saturating_add(bytes);
                    waiter.notify.notify_one();
                }
                state.queued_bytes = state.queued_bytes.saturating_add(bytes);
            }
        }
        for request in retired_favicons {
            state.favicon_requests.remove(&request);
        }
        drop(state);
        runtime
            .apply_network_lifecycle(batch.retirement())
            .map_err(HostError::backend)?;
        Ok(())
    }

    pub(crate) fn reconcile(
        &self,
        active: &[ObjectId],
        #[cfg(any(
            feature = "context-network-events",
            feature = "request-history",
            feature = "request-events",
            feature = "response-events"
        ))]
        active_contexts: &[ObjectId],
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        for waiter in state.waiters.values_mut() {
            #[cfg(any(
                feature = "context-network-events",
                feature = "request-history",
                feature = "request-events",
                feature = "response-events"
            ))]
            let contexts = active_contexts;
            #[cfg(not(any(
                feature = "context-network-events",
                feature = "request-history",
                feature = "request-events",
                feature = "response-events"
            )))]
            let contexts = &[];
            if !waiter.scope.is_active(active, contexts) {
                waiter.retired = true;
                waiter.notify.notify_one();
            }
        }
        state.size_owners.retain(|_, owner| {
            let Some(owner) = owner.upgrade() else {
                return false;
            };
            if active.contains(&owner.page) {
                true
            } else {
                owner.close();
                false
            }
        });
        #[cfg(feature = "response-finished")]
        state.response_finished_owners.retain(|_, owner| {
            let Some(owner) = owner.upgrade() else {
                return false;
            };
            if active.contains(&owner.page) {
                true
            } else {
                owner.close();
                false
            }
        });
        state
            .favicon_requests
            .retain(|_, page| active.contains(page));
        #[cfg(feature = "request-history")]
        state
            .history_contexts
            .retain(|context| active_contexts.contains(context));
        #[cfg(feature = "request-events")]
        state
            .request_contexts
            .retain(|context| active_contexts.contains(context));
        #[cfg(feature = "response-events")]
        state
            .response_contexts
            .retain(|context| active_contexts.contains(context));
        #[cfg(feature = "request-events")]
        state.reconcile_request_notices(active);
        #[cfg(feature = "response-events")]
        state.reconcile_response_notices(active);
        drop(state);
        #[cfg(feature = "request-history")]
        self.history.reconcile(active)?;
        Ok(())
    }

    pub(crate) fn close(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.closed = true;
        for waiter in state.waiters.values() {
            waiter.notify.notify_one();
        }
        for owner in state.size_owners.values().filter_map(Weak::upgrade) {
            owner.close();
        }
        state.size_owners.clear();
        #[cfg(feature = "response-finished")]
        {
            for owner in state
                .response_finished_owners
                .values()
                .filter_map(Weak::upgrade)
            {
                owner.close();
            }
            state.response_finished_owners.clear();
        }
        state.favicon_requests.clear();
        #[cfg(feature = "request-history")]
        state.history_contexts.clear();
        #[cfg(feature = "request-events")]
        {
            state.request_contexts.clear();
            state.clear_request_notices();
            self.request_notify.notify_waiters();
        }
        #[cfg(feature = "response-events")]
        {
            state.response_contexts.clear();
            state.clear_response_notices();
            self.response_notify.notify_waiters();
        }
        #[cfg(all(feature = "request-events", feature = "response-events"))]
        self.network_notify.notify_waiters();
        drop(state);
        #[cfg(feature = "request-history")]
        self.history.close();
    }
}

pub(crate) fn reconcile_state(
    state: &Mutex<Option<LiveSession>>,
    installation: &Installation,
) -> Result<(), HostError> {
    let state = state.lock().map_err(HostError::backend)?;
    let live = state.as_ref().ok_or(HostError::TargetClosed)?;
    let topology = live.composition.topology().map_err(HostError::backend)?;
    let active = topology
        .topology()
        .page_session_routes()
        .map(|route| route.object_id())
        .collect::<Vec<_>>();
    #[cfg(any(
        feature = "context-network-events",
        feature = "request-history",
        feature = "request-events",
        feature = "response-events"
    ))]
    let active_contexts = topology
        .topology()
        .browser_context_routes()
        .map(|route| route.object_id())
        .collect::<Vec<_>>();
    installation.reconcile(
        &active,
        #[cfg(any(
            feature = "context-network-events",
            feature = "request-history",
            feature = "request-events",
            feature = "response-events"
        ))]
        &active_contexts,
    )
}

impl HostSession {
    pub async fn page_arm_network_wait(
        &self,
        page_id: &str,
        kind: NetworkEventKind,
        timeout_ms: u32,
    ) -> Result<u64, HostError> {
        let (page, _call, _operation) = self.admit_operation(page_id)?;
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .network_events
                .clone()
        };
        installation
            .arm(NetworkWaitScope::Page(page), kind, timeout_ms)
            .await
    }

    #[cfg(feature = "context-network-events")]
    pub async fn context_arm_network_wait(
        &self,
        context_id: &str,
        kind: NetworkEventKind,
        timeout_ms: u32,
    ) -> Result<u64, HostError> {
        let (context, _call, _operation) = self.admit_operation(context_id)?;
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            if topology.topology().browser_context_route(context).is_none() {
                return Err(HostError::TargetClosed);
            }
            Arc::clone(&live.network_events)
        };
        installation
            .arm(NetworkWaitScope::Context(context), kind, timeout_ms)
            .await
    }

    pub async fn page_next_network_event(
        &self,
        waiter_id: u64,
    ) -> Result<NetworkWaitRecord, HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .network_events
                .clone()
        };
        installation.next(waiter_id).await
    }

    #[cfg(feature = "request-events")]
    pub async fn next_request_notice(&self) -> Result<RequestNotice, HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .network_events
                .clone()
        };
        installation.next_request_notice().await
    }

    #[cfg(feature = "response-events")]
    pub async fn next_response_notice(&self) -> Result<ResponseNotice, HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .network_events
                .clone()
        };
        installation.next_response_notice().await
    }

    #[cfg(all(feature = "request-events", feature = "response-events"))]
    pub async fn next_network_notice(&self) -> Result<NetworkNotice, HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .network_events
                .clone()
        };
        installation.next_network_notice().await
    }

    pub fn network_notice_fence(&self) -> Result<(u64, u64), HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .network_events
                .clone()
        };
        installation.notice_fence()
    }

    pub async fn page_cancel_network_wait(&self, waiter_id: u64) -> Result<(), HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .network_events
                .clone()
        };
        installation.cancel(waiter_id).await
    }

    #[cfg(feature = "request-history")]
    pub(crate) async fn enable_request_history_context(
        &self,
        context_id: &str,
    ) -> Result<(), HostError> {
        let context = context_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Context ID".into()))?;
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            // Empty owned Context creation publishes topology synchronously but
            // has no Page target event to drive the aggregate route refresh.
            // Publish that Context into the exact route snapshot before the
            // Context-scoped network transaction records inherited interest for
            // its future Pages.
            let topology = live.composition.topology().map_err(HostError::backend)?;
            live.runtime
                .service()
                .synchronize(&topology)
                .map_err(HostError::backend)?;
            live.network_events.clone()
        };
        installation.enable_history_context(context).await
    }

    #[cfg(feature = "request-events")]
    pub(crate) async fn enable_request_event_context(
        &self,
        context_id: &str,
    ) -> Result<(), HostError> {
        let context = context_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Context ID".into()))?;
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            live.runtime
                .service()
                .synchronize(&topology)
                .map_err(HostError::backend)?;
            live.network_events.clone()
        };
        installation.enable_request_context(context).await
    }

    #[cfg(feature = "response-events")]
    pub(crate) async fn enable_response_event_context(
        &self,
        context_id: &str,
    ) -> Result<(), HostError> {
        let context = context_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Context ID".into()))?;
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            live.runtime
                .service()
                .synchronize(&topology)
                .map_err(HostError::backend)?;
            live.network_events.clone()
        };
        installation.enable_response_context(context).await
    }

    #[cfg(feature = "request-history")]
    pub fn page_requests(&self, page_id: &str) -> Result<Vec<Arc<RequestHistoryOwner>>, HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .network_events
                .clone()
        };
        self.frame_trees.get(page)?;
        installation.history.snapshot(page)
    }

    #[cfg(feature = "request-history")]
    pub async fn request_response_by_native_id(
        &self,
        page_id: &str,
        native_request_id: &str,
    ) -> Result<Option<NetworkResponseRecord>, HostError> {
        if native_request_id.is_empty()
            || native_request_id.len() > MAX_REQUEST_HISTORY_NATIVE_ID_BYTES
        {
            return Err(HostError::InvalidArgument(format!(
                "native request ID must contain 1..={MAX_REQUEST_HISTORY_NATIVE_ID_BYTES} UTF-8 bytes"
            )));
        }
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .network_events
                .clone()
        };
        let page_tree = self.frame_trees.get(page)?;
        let stopped = self.stop.clone();
        installation
            .history
            .response_by_native_id(page, native_request_id, &stopped, || page_tree.is_closed())
            .await
    }

    #[cfg(feature = "request-history")]
    pub async fn request_by_native_id(
        &self,
        page_id: &str,
        native_request_id: &str,
    ) -> Result<Arc<RequestHistoryOwner>, HostError> {
        if native_request_id.is_empty()
            || native_request_id.len() > MAX_REQUEST_HISTORY_NATIVE_ID_BYTES
        {
            return Err(HostError::InvalidArgument(format!(
                "native request ID must contain 1..={MAX_REQUEST_HISTORY_NATIVE_ID_BYTES} UTF-8 bytes"
            )));
        }
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .network_events
                .clone()
        };
        let page_tree = self.frame_trees.get(page)?;
        let stopped = self.stop.clone();
        installation
            .history
            .owner_by_native_id(page, native_request_id, &stopped, || page_tree.is_closed())
            .await
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "navigation-response-finished")]
    use std::future::Future;
    use std::sync::Arc;

    #[cfg(feature = "response-security-details")]
    use pwright_cdp::CdpEvent;
    #[cfg(feature = "response-security-details")]
    use pwright_core::AtomicObjectIdSource;
    use pwright_core::ObjectId;
    #[cfg(feature = "request-history")]
    use pwright_core::{
        ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceRequirements,
        ResourceScope,
    };
    #[cfg(feature = "response-security-details")]
    use pwright_network_model::NetworkLifecycleLimits;
    #[cfg(feature = "request-timing")]
    use pwright_network_model::{RequestInitializer, RequestTiming, ResponseInitializer};

    #[cfg(all(feature = "request-events", feature = "response-events"))]
    use super::NetworkNotice;
    #[cfg(feature = "context-network-events")]
    use super::NetworkWaitScope;
    #[cfg(any(feature = "request-events", feature = "response-events"))]
    use super::State;
    #[cfg(feature = "request-post-data-json")]
    use super::request_post_data_json;
    #[cfg(feature = "request-events")]
    use super::{MAX_REQUEST_EVENT_NOTICES, RequestNotice};
    #[cfg(feature = "response-events")]
    use super::{MAX_RESPONSE_EVENT_NOTICES, ResponseNotice};
    #[cfg(feature = "response-finished")]
    use super::{MAX_RESPONSE_FINISHED_OWNERS, ResponseFinishedOwner};
    use super::{
        NetworkEventKind, NetworkRequestRecord, NetworkResponseRecord, RequestSizeOwner,
        RequestSizes,
    };
    #[cfg(feature = "request-history")]
    use super::{NetworkEventRecord, RequestHistory, requirements};
    #[cfg(feature = "response-security-details")]
    use super::{attach_response_security, record};
    #[cfg(feature = "request-timing")]
    use super::{request as request_record, response_record};
    use crate::HostError;

    #[test]
    fn request_event_name_selects_the_request_lane() {
        // Arrange
        let name = "request";

        // Act
        let kind = NetworkEventKind::parse(name).expect("request is supported");

        // Assert
        assert_eq!(kind, NetworkEventKind::Request);
    }

    #[test]
    fn unknown_event_name_is_rejected_before_subscription_activation() {
        // Arrange
        let name = "websocket";

        // Act
        let error = NetworkEventKind::parse(name).expect_err("unknown event must fail");

        // Assert
        assert!(matches!(error, HostError::InvalidArgument(_)));
    }

    fn request() -> NetworkRequestRecord {
        NetworkRequestRecord {
            id: "2".into(),
            native_request_id: Some("native-2".into()),
            page_id: "1".into(),
            url: "https://example.test/get?ignored=true".into(),
            method: "POST".into(),
            resource_type: "fetch".into(),
            headers: vec![("accept".into(), "*/*".into())],
            post_data: Some(b"12345".to_vec()),
            is_navigation_request: false,
            frame_id: Some("3".into()),
            frame_available: true,
            redirected_from: None,
            #[cfg(feature = "request-timing")]
            timing: RequestTiming::pending(),
            size_owner: None,
        }
    }

    #[cfg(feature = "request-timing")]
    #[test]
    fn request_record_preserves_the_normalized_timing_sidecar() {
        // Arrange
        let timing = Arc::new(RequestTiming {
            start_time_micros: 1_700_000_000_000_500,
            domain_lookup_start_micros: 100,
            domain_lookup_end_micros: 200,
            connect_start_micros: 200,
            secure_connection_start_micros: 300,
            connect_end_micros: 500,
            request_start_micros: 600,
            response_start_micros: 2_500,
            response_end_micros: 5_000,
        });
        let value = RequestInitializer {
            request: ObjectId::new(2),
            parent: ObjectId::new(1),
            url: "https://example.test/timed".into(),
            method: "GET".into(),
            resource_type: "Document".into(),
            headers: Vec::new(),
            post_data: None,
            is_navigation_request: true,
            redirected_from: None,
            timing: Some(timing),
        };

        // Act
        let record = request_record(&value, None, true, Some("native-timed"));

        // Assert
        assert_eq!(record.timing.start_time_micros, 1_700_000_000_000_500);
        assert_eq!(record.timing.response_end_micros, 5_000);
    }

    #[cfg(feature = "request-timing")]
    #[test]
    fn response_record_preserves_the_same_request_timing_snapshot() {
        // Arrange
        let timing = Arc::new(RequestTiming {
            start_time_micros: 1_700_000_000_000_500,
            domain_lookup_start_micros: 100,
            domain_lookup_end_micros: 200,
            connect_start_micros: 200,
            secure_connection_start_micros: 300,
            connect_end_micros: 500,
            request_start_micros: 600,
            response_start_micros: 2_500,
            response_end_micros: 5_000,
        });
        let value = ResponseInitializer {
            response: ObjectId::new(3),
            request: ObjectId::new(2),
            url: "https://example.test/timed".into(),
            status: 200,
            status_text: "OK".into(),
            headers: Vec::new(),
            from_service_worker: false,
            timing: Some(timing),
        };

        // Act
        let record = response_record(&value, None);

        // Assert
        assert_eq!(record.timing.response_start_micros, 2_500);
        assert_eq!(record.timing.response_end_micros, 5_000);
    }

    #[cfg(feature = "request-post-data-json")]
    #[test]
    fn form_post_data_json_accepts_content_type_parameters() {
        // Arrange
        let mut value = request();
        value.headers = vec![(
            "Content-Type".into(),
            "application/x-www-form-urlencoded; charset=UTF-8".into(),
        )];
        value.post_data = Some(b"foo=bar&baz=123".to_vec());

        // Act
        let parsed = request_post_data_json(&value).expect("form data parses");

        // Assert
        assert_eq!(
            parsed,
            Some(serde_json::json!({"foo": "bar", "baz": "123"}))
        );
    }

    #[cfg(feature = "request-post-data-json")]
    #[test]
    fn json_post_data_json_preserves_nested_types() {
        // Arrange
        let mut value = request();
        value.headers = vec![("content-type".into(), "application/json".into())];
        value.post_data = Some(br#"{"value":42,"items":[true,null]}"#.to_vec());

        // Act
        let parsed = request_post_data_json(&value).expect("JSON data parses");

        // Assert
        assert_eq!(
            parsed,
            Some(serde_json::json!({"value": 42, "items": [true, null]}))
        );
    }

    #[cfg(feature = "request-post-data-json")]
    #[test]
    fn invalid_post_data_json_reports_the_official_payload_error() {
        // Arrange
        let mut value = request();
        value.post_data = Some(b"not-json".to_vec());

        // Act
        let error = request_post_data_json(&value).expect_err("invalid JSON fails");

        // Assert
        assert_eq!(error, "POST data is not a valid JSON object: not-json");
    }

    #[cfg(feature = "context-network-events")]
    #[test]
    fn context_network_scope_matches_only_pages_from_the_same_context() {
        // Arrange
        let scope = NetworkWaitScope::Context(ObjectId::new(9));

        // Act
        let same_context = scope.matches(ObjectId::new(2), Some(ObjectId::new(9)));
        let other_context = scope.matches(ObjectId::new(2), Some(ObjectId::new(10)));

        // Assert
        assert!(same_context);
        assert!(!other_context);
    }

    #[cfg(feature = "context-network-events")]
    #[test]
    fn context_network_scope_liveness_uses_context_topology() {
        // Arrange
        let scope = NetworkWaitScope::Context(ObjectId::new(9));

        // Act
        let active = scope.is_active(&[], &[ObjectId::new(9)]);
        let retired = scope.is_active(&[ObjectId::new(9)], &[]);

        // Assert
        assert!(active);
        assert!(!retired);
    }

    fn response() -> NetworkResponseRecord {
        NetworkResponseRecord {
            id: "3".into(),
            request_id: "2".into(),
            url: "https://example.test/get?ignored=true".into(),
            status: 200,
            status_text: "OK".into(),
            headers: vec![("content-length".into(), "6".into())],
            from_service_worker: false,
            #[cfg(feature = "request-timing")]
            timing: RequestTiming::pending(),
            body: None,
            #[cfg(feature = "response-finished")]
            finished_owner: None,
            #[cfg(feature = "response-security-details")]
            security_details: None,
        }
    }

    #[cfg(feature = "response-security-details")]
    fn cdp_security_batches() -> (
        pwright_backend_cdp::CdpNetworkEventBatch,
        pwright_backend_cdp::CdpNetworkEventBatch,
    ) {
        let mut adapter = pwright_backend_cdp::CdpNetworkEventAdapter::new(
            ObjectId::new(7),
            "session-1".into(),
            Arc::new(AtomicObjectIdSource::new(10).unwrap()),
            NetworkLifecycleLimits::new(1, 64, 256, 16, 32, 8, 256, 64, 64, 128),
        )
        .unwrap();
        let event =
            |method: &str, params| CdpEvent::new(method.into(), params, Some("session-1".into()));
        adapter.map_event(&event("Network.requestWillBeSent", serde_json::json!({
            "requestId": "native-1", "type": "Document",
            "request": {"url": "https://example.test/fetch?1", "method": "GET", "headers": {}}
        }))).unwrap();
        let response = adapter
            .map_event(&event(
                "Network.responseReceived",
                serde_json::json!({
                    "requestId": "native-1",
                    "response": {
                        "url": "https://example.test/fetch?1", "status": 200,
                        "statusText": "OK", "headers": {},
                        "securityDetails": {
                            "protocol": "TLS 1.3", "subjectName": "example.test",
                            "issuer": "Test Authority", "validFrom": 1.0, "validTo": 2.0
                        }
                    }
                }),
            ))
            .unwrap();
        let finished = adapter
            .map_event(&event(
                "Network.loadingFinished",
                serde_json::json!({
                    "requestId": "native-1", "encodedDataLength": 0
                }),
            ))
            .unwrap();
        (response, finished)
    }

    #[cfg(feature = "response-security-details")]
    #[test]
    fn response_and_terminal_records_keep_the_same_typed_security_snapshot() {
        // Arrange
        let (response_batch, terminal_batch) = cdp_security_batches();

        // Act
        let mut response = record(
            &response_batch.events()[0],
            None,
            None,
            true,
            response_batch.native_request_id(),
        );
        attach_response_security(&response_batch.events()[0], &mut response, &response_batch);
        let mut terminal = record(
            &terminal_batch.events()[0],
            None,
            None,
            true,
            terminal_batch.native_request_id(),
        );
        attach_response_security(&terminal_batch.events()[0], &mut terminal, &terminal_batch);

        // Assert
        let NetworkEventRecord::Response(response) = response else {
            panic!("response expected");
        };
        let NetworkEventRecord::RequestFinished {
            response: Some(terminal),
            ..
        } = terminal
        else {
            panic!("terminal response expected");
        };
        assert_eq!(response.security_details, terminal.security_details);
        assert_eq!(
            response
                .security_details
                .as_ref()
                .map(|value| value.issuer()),
            Some("Test Authority")
        );
        assert!(
            response.foreign_copy_bytes()
                > response.security_details.as_ref().unwrap().retained_bytes() as u64
        );
    }

    #[cfg(any(feature = "request-events", feature = "response-events"))]
    fn event_state() -> State {
        State {
            next_id: 1,
            waiters: std::collections::BTreeMap::new(),
            size_owners: std::collections::BTreeMap::new(),
            #[cfg(feature = "response-finished")]
            response_finished_owners: std::collections::BTreeMap::new(),
            favicon_requests: std::collections::BTreeMap::new(),
            #[cfg(feature = "request-history")]
            history_contexts: std::collections::BTreeSet::new(),
            #[cfg(feature = "request-events")]
            request_contexts: std::collections::BTreeSet::new(),
            #[cfg(feature = "request-events")]
            request_notices: std::collections::VecDeque::with_capacity(MAX_REQUEST_EVENT_NOTICES),
            #[cfg(feature = "request-events")]
            next_request_notice_sequence: 1,
            #[cfg(feature = "request-events")]
            last_popped_request_notice_sequence: 0,
            #[cfg(feature = "response-events")]
            response_contexts: std::collections::BTreeSet::new(),
            #[cfg(feature = "response-events")]
            response_notices: std::collections::VecDeque::with_capacity(MAX_RESPONSE_EVENT_NOTICES),
            #[cfg(feature = "response-events")]
            next_response_notice_sequence: 1,
            #[cfg(feature = "response-events")]
            last_popped_response_notice_sequence: 0,
            #[cfg(all(feature = "request-events", feature = "response-events"))]
            next_network_notice_sequence: 1,
            queued_bytes: 0,
            closed: false,
        }
    }

    #[cfg(all(feature = "request-events", feature = "response-events"))]
    #[test]
    fn merged_network_notices_preserve_publication_order_and_charge_both_queues() {
        // Arrange
        let mut state = event_state();
        state
            .enqueue_request_notice(RequestNotice {
                sequence: 0,
                order_sequence: 0,
                page: ObjectId::new(7),
                kind: NetworkEventKind::Request,
                request: request(),
                failure_text: None,
                #[cfg(feature = "request-history")]
                history_owner: None,
            })
            .expect("request notice fits");
        state
            .enqueue_response_notice(ResponseNotice {
                sequence: 0,
                order_sequence: 0,
                page: ObjectId::new(7),
                native_request_id: Some("native-1".into()),
                response: response(),
                #[cfg(feature = "request-history")]
                history_owner: None,
            })
            .expect("response notice fits");
        state
            .enqueue_request_notice(RequestNotice {
                sequence: 0,
                order_sequence: 0,
                page: ObjectId::new(7),
                kind: NetworkEventKind::RequestFinished,
                request: request(),
                failure_text: None,
                #[cfg(feature = "request-history")]
                history_owner: None,
            })
            .expect("terminal notice fits");

        // Act
        let first = state.pop_network_notice();
        let second = state.pop_network_notice();
        let third = state.pop_network_notice();

        // Assert
        assert!(matches!(
            first,
            Some(NetworkNotice::Request(RequestNotice {
                kind: NetworkEventKind::Request,
                order_sequence: 1,
                ..
            }))
        ));
        assert!(matches!(
            second,
            Some(NetworkNotice::Response(ResponseNotice {
                order_sequence: 2,
                ..
            }))
        ));
        assert!(matches!(
            third,
            Some(NetworkNotice::Request(RequestNotice {
                kind: NetworkEventKind::RequestFinished,
                order_sequence: 3,
                ..
            }))
        ));
        assert!(state.pop_network_notice().is_none());
        assert_eq!(state.queued_bytes, 0);
    }

    #[cfg(feature = "response-events")]
    #[test]
    fn response_notice_foreign_copy_budget_excludes_native_body_bytes() {
        // Arrange
        let metadata_only = ResponseNotice {
            sequence: 1,
            #[cfg(feature = "request-events")]
            order_sequence: 1,
            page: ObjectId::new(7),
            native_request_id: Some("native-9".into()),
            response: response(),
            #[cfg(feature = "request-history")]
            history_owner: None,
        };
        let mut with_body = metadata_only.clone();
        with_body.response.body = Some(std::sync::Arc::from(vec![7_u8; 1024 * 1024]));

        // Act
        let metadata_budget = metadata_only.foreign_copy_bytes();
        let body_budget = with_body.foreign_copy_bytes();

        // Assert
        assert_eq!(body_budget, metadata_budget);
    }

    #[cfg(feature = "response-events")]
    #[test]
    fn response_notice_queue_is_fifo_and_rejects_the_two_hundred_fifty_seventh_item() {
        // Arrange
        let mut state = event_state();
        for index in 0..MAX_RESPONSE_EVENT_NOTICES {
            let mut value = response();
            value.id = index.to_string();
            state
                .enqueue_response_notice(ResponseNotice {
                    sequence: 0,
                    #[cfg(feature = "request-events")]
                    order_sequence: 0,
                    page: ObjectId::new(7),
                    native_request_id: Some(format!("native-{index}")),
                    response: value,
                    #[cfg(feature = "request-history")]
                    history_owner: None,
                })
                .expect("the fixed response notice capacity accepts this item");
        }
        let queued_bytes = state.queued_bytes;

        // Act
        let overflow = state.enqueue_response_notice(ResponseNotice {
            sequence: 0,
            #[cfg(feature = "request-events")]
            order_sequence: 0,
            page: ObjectId::new(7),
            native_request_id: Some("native-overflow".into()),
            response: response(),
            #[cfg(feature = "request-history")]
            history_owner: None,
        });
        let first = state
            .pop_response_notice()
            .expect("the first queued notice exists");

        // Assert
        assert!(matches!(overflow, Err(HostError::ResourceExhausted(_))));
        assert_eq!(first.sequence, 1);
        assert_eq!(
            state.response_notice_fence(),
            MAX_RESPONSE_EVENT_NOTICES as u64
        );
        assert_eq!(first.response.id, "0");
        assert_eq!(state.response_notices.len(), MAX_RESPONSE_EVENT_NOTICES - 1);
        assert!(state.queued_bytes < queued_bytes);
    }

    #[cfg(feature = "response-events")]
    #[test]
    fn response_notice_reconciliation_drops_only_retired_pages_and_releases_bytes() {
        // Arrange
        let mut state = event_state();
        for page in [ObjectId::new(7), ObjectId::new(8)] {
            state
                .enqueue_response_notice(ResponseNotice {
                    sequence: 0,
                    #[cfg(feature = "request-events")]
                    order_sequence: 0,
                    page,
                    native_request_id: Some("native-response".into()),
                    response: response(),
                    #[cfg(feature = "request-history")]
                    history_owner: None,
                })
                .expect("two notices fit the fixed queue");
        }
        let queued_bytes = state.queued_bytes;

        // Act
        state.reconcile_response_notices(&[ObjectId::new(8)]);

        // Assert
        assert_eq!(state.response_notices.len(), 1);
        assert_eq!(state.response_notices[0].page, ObjectId::new(8));
        assert!(state.queued_bytes < queued_bytes);
        assert_ne!(state.queued_bytes, 0);
    }

    #[cfg(feature = "request-events")]
    #[test]
    fn request_notice_queue_is_fifo_and_rejects_the_two_hundred_fifty_seventh_item() {
        // Arrange
        let mut state = event_state();
        for index in 0..MAX_REQUEST_EVENT_NOTICES {
            let mut value = request();
            value.id = index.to_string();
            state
                .enqueue_request_notice(RequestNotice {
                    sequence: 0,
                    #[cfg(feature = "response-events")]
                    order_sequence: 0,
                    page: ObjectId::new(7),
                    kind: NetworkEventKind::Request,
                    request: value,
                    failure_text: None,
                    #[cfg(feature = "request-history")]
                    history_owner: None,
                })
                .expect("the fixed request notice capacity accepts this item");
        }
        let queued_bytes = state.queued_bytes;

        // Act
        let overflow = state.enqueue_request_notice(RequestNotice {
            sequence: 0,
            #[cfg(feature = "response-events")]
            order_sequence: 0,
            page: ObjectId::new(7),
            kind: NetworkEventKind::Request,
            request: request(),
            failure_text: None,
            #[cfg(feature = "request-history")]
            history_owner: None,
        });
        let first = state
            .pop_request_notice()
            .expect("the first queued notice exists");

        // Assert
        assert!(matches!(overflow, Err(HostError::ResourceExhausted(_))));
        assert_eq!(first.sequence, 1);
        assert_eq!(
            state.request_notice_fence(),
            MAX_REQUEST_EVENT_NOTICES as u64
        );
        assert_eq!(first.request.id, "0");
        assert_eq!(state.request_notices.len(), MAX_REQUEST_EVENT_NOTICES - 1);
        assert!(state.queued_bytes < queued_bytes);
    }

    #[cfg(feature = "request-events")]
    #[test]
    fn request_notice_foreign_copy_budget_covers_snapshot_and_failure_text() {
        // Arrange
        let mut request = request();
        request.headers = vec![("content-type".into(), "application/json".into())];
        request.post_data = Some(vec![7; 4096]);
        let retained_bytes = request.retained_bytes() as u64;
        let notice = RequestNotice {
            sequence: 1,
            #[cfg(feature = "response-events")]
            order_sequence: 1,
            page: ObjectId::new(7),
            kind: NetworkEventKind::RequestFailed,
            request,
            failure_text: Some("connection reset".into()),
            #[cfg(feature = "request-history")]
            history_owner: None,
        };

        // Act
        let allowance = notice.foreign_copy_bytes();

        // Assert
        assert!(allowance >= retained_bytes.saturating_mul(4));
        assert!(allowance >= 12 * 1024);
    }

    #[cfg(feature = "request-events")]
    #[test]
    fn request_notice_reconciliation_drops_only_retired_pages_and_releases_bytes() {
        // Arrange
        let mut state = event_state();
        for page in [ObjectId::new(7), ObjectId::new(8)] {
            state
                .enqueue_request_notice(RequestNotice {
                    sequence: 0,
                    #[cfg(feature = "response-events")]
                    order_sequence: 0,
                    page,
                    kind: NetworkEventKind::Request,
                    request: request(),
                    failure_text: None,
                    #[cfg(feature = "request-history")]
                    history_owner: None,
                })
                .expect("two notices fit the fixed queue");
        }
        let queued_bytes = state.queued_bytes;

        // Act
        state.reconcile_request_notices(&[ObjectId::new(8)]);

        // Assert
        assert_eq!(state.request_notices.len(), 1);
        assert_eq!(state.request_notices[0].page, ObjectId::new(8));
        assert!(state.queued_bytes < queued_bytes);
        assert_ne!(state.queued_bytes, 0);
    }

    #[cfg(feature = "request-events")]
    #[test]
    fn request_failure_notice_retains_the_terminal_kind_and_failure_text() {
        // Arrange
        let request = request();
        let request_bytes = request.retained_bytes();
        let failure = "net::ERR_INTERNET_DISCONNECTED".to_owned();
        let notice = RequestNotice {
            sequence: 0,
            #[cfg(feature = "response-events")]
            order_sequence: 0,
            page: ObjectId::new(7),
            kind: NetworkEventKind::RequestFailed,
            request,
            failure_text: Some(failure.clone()),
            #[cfg(feature = "request-history")]
            history_owner: None,
        };

        // Act
        let retained_bytes = notice.retained_bytes();

        // Assert
        assert_eq!(notice.kind.as_str(), "requestfailed");
        assert_eq!(notice.failure_text.as_deref(), Some(failure.as_str()));
        assert_eq!(retained_bytes, request_bytes + failure.len());
    }

    #[test]
    fn retained_response_body_bytes_are_charged_to_the_waiter_queue() {
        // Arrange
        let without_body = response();
        let mut with_body = response();
        with_body.body = Some(Arc::from(b"fulfilled".as_slice()));

        // Act
        let baseline = without_body.retained_bytes();
        let retained = with_body.retained_bytes();

        // Assert
        assert_eq!(retained - baseline, b"fulfilled".len());
    }

    fn owner() -> Arc<RequestSizeOwner> {
        let slots = Arc::new(tokio::sync::Semaphore::new(1));
        let slot = slots
            .try_acquire_owned()
            .expect("one test owner fits its bounded slot");
        Arc::new(RequestSizeOwner::new(
            ObjectId::new(1),
            ObjectId::new(2),
            &request(),
            slot,
        ))
    }

    #[tokio::test]
    async fn completed_request_reports_playwright_wire_sizes() {
        // Arrange
        let owner = owner();
        let response = response();

        // Act
        owner.finish(&request(), Some(&response), Some(44));
        let sizes = owner.sizes().await.expect("completed request has sizes");

        // Assert
        assert_eq!(
            sizes,
            RequestSizes {
                request_body_size: 5,
                request_headers_size: 33,
                response_body_size: 6,
                response_headers_size: 38,
                transfer_size: 44,
            }
        );
    }

    #[tokio::test]
    async fn failed_request_reports_the_official_sizes_error() {
        // Arrange
        let owner = owner();

        // Act
        owner.fail();
        let error = owner
            .sizes()
            .await
            .expect_err("failed request has no sizes");

        // Assert
        assert_eq!(
            error.to_string(),
            "Unable to fetch sizes for failed request"
        );
    }

    #[tokio::test]
    async fn closed_owner_wakes_a_pending_sizes_call() {
        // Arrange
        let owner = owner();
        let waiting_owner = Arc::clone(&owner);
        let waiter = tokio::spawn(async move { waiting_owner.sizes().await });
        tokio::task::yield_now().await;

        // Act
        owner.close();
        let result = waiter.await.expect("size waiter joins");

        // Assert
        assert!(matches!(result, Err(HostError::TargetClosed)));
    }

    #[cfg(feature = "response-finished")]
    fn response_finished_owner_for(
        page: ObjectId,
        request: ObjectId,
    ) -> Arc<ResponseFinishedOwner> {
        let slots = Arc::new(tokio::sync::Semaphore::new(1));
        let slot = slots
            .try_acquire_owned()
            .expect("one response-finished owner fits its bounded slot");
        Arc::new(ResponseFinishedOwner::new(page, request, slot))
    }

    #[cfg(feature = "response-finished")]
    fn response_finished_owner() -> Arc<ResponseFinishedOwner> {
        response_finished_owner_for(ObjectId::new(1), ObjectId::new(2))
    }

    #[cfg(feature = "response-finished")]
    #[tokio::test]
    async fn response_finished_resolves_after_terminal_success() {
        // Arrange
        let owner = response_finished_owner();

        // Act
        owner.finish();
        let result = owner.finished().await;

        // Assert
        assert!(result.is_ok());
    }

    #[cfg(feature = "response-finished")]
    #[tokio::test]
    async fn response_finished_rejects_after_terminal_failure() {
        // Arrange
        let owner = response_finished_owner();

        // Act
        owner.fail();
        let error = owner
            .finished()
            .await
            .expect_err("failed response cannot finish successfully");

        // Assert
        assert_eq!(error.to_string(), "Response failed to finish");
    }

    #[cfg(feature = "response-finished")]
    #[tokio::test]
    async fn closed_response_owner_wakes_a_pending_finished_call() {
        // Arrange
        let owner = response_finished_owner();
        let waiting_owner = Arc::clone(&owner);
        let waiter = tokio::spawn(async move { waiting_owner.finished().await });
        tokio::task::yield_now().await;

        // Act
        owner.close();
        let result = waiter.await.expect("response-finished waiter joins");

        // Assert
        assert!(matches!(result, Err(HostError::TargetClosed)));
    }

    #[cfg(feature = "response-finished")]
    #[test]
    fn response_finished_owner_slots_have_a_fixed_reclaimable_capacity() {
        // Arrange
        let slots = Arc::new(tokio::sync::Semaphore::new(MAX_RESPONSE_FINISHED_OWNERS));
        let mut owners = Vec::with_capacity(MAX_RESPONSE_FINISHED_OWNERS);
        for index in 0..MAX_RESPONSE_FINISHED_OWNERS {
            let slot = Arc::clone(&slots)
                .try_acquire_owned()
                .expect("declared response-finished owner fits");
            owners.push(ResponseFinishedOwner::new(
                ObjectId::new(1),
                ObjectId::new(index as u64 + 1),
                slot,
            ));
        }

        // Act
        let overflow = Arc::clone(&slots).try_acquire_owned();
        owners.pop();
        let reclaimed = Arc::clone(&slots).try_acquire_owned();

        // Assert
        assert!(overflow.is_err());
        assert!(reclaimed.is_ok());
    }

    #[cfg(feature = "request-history")]
    fn history() -> (ProductionResourceLedger, RequestHistory) {
        let required = requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let bound = required.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        let production = ProductionResourceLedger::new(limits, required)
            .expect("valid request-history resource plan");
        let ledger = production
            .claim(required)
            .expect("request-history claim fits the production plan");
        let history = RequestHistory::new(ledger).expect("bounded history allocates");
        (production, history)
    }

    #[cfg(feature = "request-history")]
    fn indexed_request(index: u64) -> NetworkRequestRecord {
        NetworkRequestRecord {
            id: index.to_string(),
            native_request_id: Some(format!("native-{index}")),
            page_id: "7".into(),
            url: format!("https://example.test/fetch?{index}"),
            method: "GET".into(),
            resource_type: "fetch".into(),
            headers: Vec::new(),
            post_data: None,
            is_navigation_request: index == 1,
            frame_id: Some("8".into()),
            frame_available: true,
            redirected_from: None,
            #[cfg(feature = "request-timing")]
            timing: RequestTiming::pending(),
            size_owner: None,
        }
    }

    #[cfg(feature = "request-history")]
    fn indexed_response(index: u64) -> NetworkResponseRecord {
        let url = format!("https://example.test/fetch?{index}");
        NetworkResponseRecord {
            id: (10_000 + index).to_string(),
            request_id: index.to_string(),
            url: url.clone(),
            status: 200,
            status_text: "OK".into(),
            headers: Vec::new(),
            from_service_worker: false,
            #[cfg(feature = "request-timing")]
            timing: RequestTiming::pending(),
            body: Some(Arc::from(format!("url:{url}").into_bytes())),
            #[cfg(feature = "response-finished")]
            finished_owner: None,
            #[cfg(feature = "response-security-details")]
            security_details: None,
        }
    }

    #[cfg(feature = "response-security-details")]
    #[tokio::test]
    async fn navigation_security_requires_the_exact_page_native_request_and_response() {
        // Arrange
        let (response_batch, _) = cdp_security_batches();
        let response_id = match &response_batch.events()[0] {
            pwright_backend_cdp::CdpNetworkEventEffect::Response(value) => value.payload().response,
            _ => panic!("response expected"),
        };
        let details = response_batch
            .response_security(response_id)
            .unwrap()
            .clone();
        let (_resources, history) = history();
        let owner = history
            .record_request(ObjectId::new(7), ObjectId::new(1), &indexed_request(1))
            .unwrap();
        let mut response = indexed_response(1);
        response.security_details = Some(details);
        owner.settle(Some(response)).unwrap();

        // Act
        let matched = owner
            .observed_navigation_response("native-1", "https://example.test/fetch?1", 200)
            .await
            .unwrap();
        let wrong_native = owner
            .observed_navigation_response("native-2", "https://example.test/fetch?1", 200)
            .await
            .unwrap();
        let wrong_status = owner
            .observed_navigation_response("native-1", "https://example.test/fetch?1", 404)
            .await
            .unwrap();

        // Assert
        assert_eq!(
            matched
                .unwrap()
                .security_details
                .as_ref()
                .map(|value| value.protocol()),
            Some("TLS 1.3")
        );
        assert!(wrong_native.is_none());
        assert!(wrong_status.is_none());
    }

    #[cfg(feature = "navigation-response-finished")]
    fn navigation_response(
        index: u64,
        finished: Arc<ResponseFinishedOwner>,
    ) -> NetworkResponseRecord {
        let mut response = indexed_response(index);
        response.finished_owner = Some(finished);
        response
    }

    #[cfg(feature = "navigation-response-finished")]
    #[tokio::test]
    async fn navigation_finished_owner_waits_for_commit_response_then_real_terminal() {
        // Arrange
        let (_resources, history) = history();
        let page = ObjectId::new(7);
        let request_id = ObjectId::new(1);
        let request = history
            .record_request(page, request_id, &indexed_request(1))
            .unwrap();
        let finished = response_finished_owner_for(page, request_id);
        let mut pending = Box::pin(request.navigation_finished_owner(
            "native-1",
            "https://example.test/fetch?1",
            200,
        ));
        let first_poll = pending
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()));

        // Act
        request
            .settle(Some(navigation_response(1, Arc::clone(&finished))))
            .unwrap();
        let projected = pending.await.unwrap().unwrap();
        let mut terminal = Box::pin(projected.finished());
        let before_terminal = terminal
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()));
        finished.finish();
        let after_terminal = terminal.await;

        // Assert
        assert!(first_poll.is_pending());
        assert!(Arc::ptr_eq(&projected, &finished));
        assert!(before_terminal.is_pending());
        assert!(after_terminal.is_ok());
    }

    #[cfg(feature = "navigation-response-finished")]
    #[tokio::test]
    async fn navigation_finished_owner_preserves_load_terminal_failure() {
        // Arrange
        let (_resources, history) = history();
        let page = ObjectId::new(7);
        let request_id = ObjectId::new(1);
        let request = history
            .record_request(page, request_id, &indexed_request(1))
            .unwrap();
        let finished = response_finished_owner_for(page, request_id);
        request
            .settle(Some(navigation_response(1, Arc::clone(&finished))))
            .unwrap();
        finished.fail();

        // Act
        let projected = request
            .navigation_finished_owner("native-1", "https://example.test/fetch?1", 200)
            .await
            .unwrap()
            .unwrap();
        let result = projected.finished().await;

        // Assert
        assert!(Arc::ptr_eq(&projected, &finished));
        assert_eq!(result.unwrap_err().to_string(), "Response failed to finish");
    }

    #[cfg(feature = "navigation-response-finished")]
    #[tokio::test]
    async fn navigation_finished_owner_propagates_page_close() {
        // Arrange
        let (_resources, history) = history();
        let page = ObjectId::new(7);
        let request_id = ObjectId::new(1);
        let request = history
            .record_request(page, request_id, &indexed_request(1))
            .unwrap();
        let finished = response_finished_owner_for(page, request_id);
        request
            .settle(Some(navigation_response(1, Arc::clone(&finished))))
            .unwrap();
        let projected = request
            .navigation_finished_owner("native-1", "https://example.test/fetch?1", 200)
            .await
            .unwrap()
            .unwrap();

        // Act
        finished.close();
        let result = projected.finished().await;

        // Assert
        assert!(Arc::ptr_eq(&projected, &finished));
        assert!(matches!(result, Err(HostError::TargetClosed)));
    }

    #[cfg(feature = "navigation-response-finished")]
    #[tokio::test]
    async fn navigation_finished_owner_uses_final_redirect_hop() {
        // Arrange
        let (_resources, history) = history();
        let page = ObjectId::new(7);
        let first = indexed_request(1);
        let mut final_hop = indexed_request(2);
        final_hop.native_request_id = first.native_request_id.clone();
        final_hop.is_navigation_request = true;
        final_hop.redirected_from = Some(first.id.clone());
        history
            .record_request(page, ObjectId::new(1), &first)
            .unwrap();
        let final_request = history
            .record_request(page, ObjectId::new(2), &final_hop)
            .unwrap();
        let finished = response_finished_owner_for(page, ObjectId::new(2));
        final_request
            .settle(Some(navigation_response(2, Arc::clone(&finished))))
            .unwrap();

        // Act
        let looked_up = history
            .owner_by_native_id(
                page,
                "native-1",
                &tokio_util::sync::CancellationToken::new(),
                || Ok(false),
            )
            .await
            .unwrap();
        let projected = looked_up
            .navigation_finished_owner("native-1", "https://example.test/fetch?2", 200)
            .await
            .unwrap()
            .unwrap();

        // Assert
        assert!(Arc::ptr_eq(&looked_up, &final_request));
        assert!(Arc::ptr_eq(&projected, &finished));
    }

    #[cfg(feature = "navigation-response-finished")]
    #[tokio::test]
    async fn same_url_successive_navigations_need_distinct_native_request_ids() {
        // Arrange
        let (_resources, history) = history();
        let page = ObjectId::new(7);
        let first = history
            .record_request(page, ObjectId::new(1), &indexed_request(1))
            .unwrap();
        let mut second_request = indexed_request(2);
        second_request.is_navigation_request = true;
        second_request.url = "https://example.test/fetch?1".into();
        let second = history
            .record_request(page, ObjectId::new(2), &second_request)
            .unwrap();
        let first_finished = response_finished_owner_for(page, ObjectId::new(1));
        let second_finished = response_finished_owner_for(page, ObjectId::new(2));
        first
            .settle(Some(navigation_response(1, Arc::clone(&first_finished))))
            .unwrap();
        let mut second_response = navigation_response(2, Arc::clone(&second_finished));
        second_response.url = "https://example.test/fetch?1".into();
        second.settle(Some(second_response)).unwrap();

        // Act
        let wrong = first
            .navigation_finished_owner("native-2", "https://example.test/fetch?1", 200)
            .await
            .unwrap();
        let right = second
            .navigation_finished_owner("native-2", "https://example.test/fetch?1", 200)
            .await
            .unwrap()
            .unwrap();

        // Assert
        assert!(wrong.is_none());
        assert!(Arc::ptr_eq(&right, &second_finished));
        assert!(!Arc::ptr_eq(&right, &first_finished));
    }

    #[cfg(feature = "navigation-response-finished")]
    #[tokio::test]
    async fn navigation_finished_owner_rejects_wrong_response_or_terminal_owner() {
        // Arrange
        let (_resources, history) = history();
        let page = ObjectId::new(7);
        let first = history
            .record_request(page, ObjectId::new(1), &indexed_request(1))
            .unwrap();
        let wrong_owner = response_finished_owner_for(ObjectId::new(8), ObjectId::new(1));
        first
            .settle(Some(navigation_response(1, wrong_owner)))
            .unwrap();

        // Act
        let wrong_page = first
            .navigation_finished_owner("native-1", "https://example.test/fetch?1", 200)
            .await
            .unwrap();
        let wrong_status = first
            .navigation_finished_owner("native-1", "https://example.test/fetch?1", 201)
            .await
            .unwrap();

        // Assert
        assert!(wrong_page.is_none());
        assert!(wrong_status.is_none());
    }

    #[cfg(feature = "request-history")]
    #[tokio::test]
    async fn closed_history_keeps_a_retained_request_response_and_its_page_lease() {
        // Arrange
        let (resources, history) = history();
        let page = ObjectId::new(7);
        let request_id = ObjectId::new(1);
        let owner = history
            .record_request(page, request_id, &indexed_request(1))
            .expect("request records");
        history
            .settle(request_id, Some(indexed_response(1)), true)
            .expect("response settles");
        let scope = ResourceScope::Object(page);
        let ledger = resources
            .claim(ResourceRequirements::none())
            .expect("read-only ledger claim fits");
        let before = ledger
            .scope_metrics(scope)
            .expect("Page scope reads")
            .expect("request owns its Page scope");

        // Act
        history.close();

        // Assert
        let after = ledger
            .scope_metrics(scope)
            .expect("Page scope reads after close")
            .expect("retained Request still owns its Page scope");
        let response = owner
            .response()
            .await
            .expect("settled response survives history close")
            .expect("response remains available");
        assert_eq!(owner.request().id, "1");
        assert_eq!(response.status, 200);
        assert_eq!(after.active_leases, 1);
        assert_eq!(after.units(ResourceClass::EventItem), 1);
        assert_eq!(
            after.units(ResourceClass::EventBytes),
            before.units(ResourceClass::EventBytes)
        );
    }

    #[cfg(feature = "request-history")]
    #[test]
    fn dropping_the_last_retained_request_releases_its_page_scope_after_history_close() {
        // Arrange
        let (resources, history) = history();
        let page = ObjectId::new(7);
        let owner = history
            .record_request(page, ObjectId::new(1), &indexed_request(1))
            .expect("request records");
        history.close();
        let scope = ResourceScope::Object(page);
        let ledger = resources
            .claim(ResourceRequirements::none())
            .expect("read-only ledger claim fits");
        assert_eq!(
            ledger
                .scope_metrics(scope)
                .expect("Page scope reads")
                .expect("retained Request still owns its Page scope")
                .active_leases,
            1
        );

        // Act
        drop(owner);

        // Assert
        assert!(
            ledger
                .scope_metrics(scope)
                .expect("Page scope reads")
                .is_none()
        );
        assert_eq!(resources.metrics().expect("ledger reads").active_leases, 0);
    }

    #[cfg(feature = "request-history")]
    #[tokio::test]
    async fn native_request_lookup_waits_for_the_exact_network_identity() {
        // Arrange
        let (_resources, history) = history();
        let history = Arc::new(history);
        let page = ObjectId::new(7);
        history
            .record_request(page, ObjectId::new(1), &indexed_request(1))
            .expect("nonmatching history request records");
        let stopped = tokio_util::sync::CancellationToken::new();
        let waiting_history = Arc::clone(&history);
        let waiting_stop = stopped.clone();
        let waiter = tokio::spawn(async move {
            waiting_history
                .owner_by_native_id(page, "native-2", &waiting_stop, || Ok(false))
                .await
        });
        tokio::task::yield_now().await;

        // Act
        history
            .record_request(page, ObjectId::new(2), &indexed_request(2))
            .expect("matching history request records");
        let owner = waiter
            .await
            .expect("native identity waiter joins")
            .expect("native identity lookup remains live");

        // Assert
        assert_eq!(owner.request().id, "2");
        assert_eq!(
            owner.request().native_request_id.as_deref(),
            Some("native-2")
        );
    }

    #[cfg(feature = "request-history")]
    #[tokio::test]
    async fn native_request_lookup_uses_final_portable_redirect_hop() {
        // Arrange
        let (_resources, history) = history();
        let page = ObjectId::new(7);
        let first = indexed_request(1);
        let mut final_hop = indexed_request(2);
        final_hop.native_request_id = first.native_request_id.clone();
        final_hop.redirected_from = Some(first.id.clone());
        history
            .record_request(page, ObjectId::new(1), &first)
            .expect("first hop records");
        history
            .record_request(page, ObjectId::new(2), &final_hop)
            .expect("final hop records");

        // Act
        let owner = history
            .owner_by_native_id(
                page,
                "native-1",
                &tokio_util::sync::CancellationToken::new(),
                || Ok(false),
            )
            .await
            .expect("native identity resolves");

        // Assert
        assert_eq!(owner.request().id, "2");
        assert_eq!(owner.request().redirected_from.as_deref(), Some("1"));
    }

    #[cfg(feature = "request-history")]
    #[test]
    fn redirect_owner_exposes_the_exact_three_hop_predecessor_chain() {
        // Arrange
        let (_resources, history) = history();
        let page = ObjectId::new(7);
        let first = history
            .record_request(page, ObjectId::new(1), &indexed_request(1))
            .expect("first hop records");
        let mut second_request = indexed_request(2);
        second_request.redirected_from = Some(first.request().id.clone());
        let second = history
            .record_request(page, ObjectId::new(2), &second_request)
            .expect("second hop records");
        let mut third_request = indexed_request(3);
        third_request.redirected_from = Some(second.request().id.clone());
        let third = history
            .record_request(page, ObjectId::new(3), &third_request)
            .expect("third hop records");

        // Act
        let predecessor = third
            .redirected_from()
            .expect("third hop has a predecessor");
        let oldest = predecessor
            .redirected_from()
            .expect("second hop has a predecessor");

        // Assert
        assert!(Arc::ptr_eq(&predecessor, &second));
        assert!(Arc::ptr_eq(&oldest, &first));
        assert!(oldest.redirected_from().is_none());
    }

    #[cfg(feature = "request-history")]
    #[test]
    fn redirect_chain_survives_history_eviction_and_releases_its_leases() {
        // Arrange
        let (resources, history) = history();
        let page = ObjectId::new(7);
        let first = history
            .record_request(page, ObjectId::new(1), &indexed_request(1))
            .expect("first hop records");
        let mut second_request = indexed_request(2);
        second_request.redirected_from = Some(first.request().id.clone());
        let second = history
            .record_request(page, ObjectId::new(2), &second_request)
            .expect("second hop records");
        let mut third_request = indexed_request(3);
        third_request.redirected_from = Some(second.request().id.clone());
        let third = history
            .record_request(page, ObjectId::new(3), &third_request)
            .expect("third hop records");
        let first_weak = Arc::downgrade(&first);
        let second_weak = Arc::downgrade(&second);
        drop(first);
        drop(second);
        for index in 4..=103 {
            history
                .record_request(page, ObjectId::new(index), &indexed_request(index))
                .expect("later request records");
        }

        // Act
        let predecessor = third
            .redirected_from()
            .expect("evicted second hop survives");
        let oldest = predecessor
            .redirected_from()
            .expect("evicted first hop survives");
        let remaining_ids = history
            .snapshot(page)
            .expect("Page history remains live")
            .iter()
            .map(|owner| owner.request().id.clone())
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(predecessor.request().id, "2");
        assert_eq!(oldest.request().id, "1");
        assert_eq!(remaining_ids.first().map(String::as_str), Some("4"));
        assert_eq!(remaining_ids.last().map(String::as_str), Some("103"));
        assert_eq!(
            resources
                .metrics()
                .expect("resource accounting is readable")
                .class(ResourceClass::EventItem)
                .current,
            103
        );
        drop(oldest);
        drop(predecessor);
        drop(third);
        assert!(first_weak.upgrade().is_none());
        assert!(second_weak.upgrade().is_none());
        assert_eq!(
            resources
                .metrics()
                .expect("released resource accounting is readable")
                .class(ResourceClass::EventItem)
                .current,
            100
        );
    }

    #[cfg(feature = "request-history")]
    #[test]
    fn retained_redirect_chain_cannot_exceed_the_per_page_owner_limit() {
        // Arrange
        let (resources, history) = history();
        let page = ObjectId::new(7);
        let mut preceding_id = None;
        for index in 1..=super::MAX_REQUEST_HISTORY_OWNERS_PER_PAGE {
            let mut request = indexed_request(index);
            request.redirected_from = preceding_id.take();
            let owner = history
                .record_request(page, ObjectId::new(index), &request)
                .expect("request within the per-Page limit records");
            preceding_id = Some(owner.request().id.clone());
        }
        let mut overflow_request = indexed_request(super::MAX_REQUEST_HISTORY_OWNERS_PER_PAGE + 1);
        overflow_request.redirected_from = preceding_id;

        // Act
        let overflow = history.record_request(
            page,
            ObjectId::new(super::MAX_REQUEST_HISTORY_OWNERS_PER_PAGE + 1),
            &overflow_request,
        );
        let current = resources
            .metrics()
            .expect("resource accounting is readable")
            .class(ResourceClass::EventItem)
            .current;
        let final_history = history.snapshot(page).expect("Page history remains live");

        // Assert
        assert!(matches!(overflow, Err(HostError::ResourceExhausted(_))));
        assert_eq!(current, super::MAX_REQUEST_HISTORY_OWNERS_PER_PAGE);
        assert_eq!(
            final_history.len(),
            super::MAX_REQUEST_HISTORY_ITEMS_PER_PAGE
        );
        assert_eq!(final_history.last().unwrap().request().id, "256");
    }

    #[cfg(feature = "request-history")]
    #[tokio::test]
    async fn request_history_keeps_response_and_failure_as_independent_states() {
        // Arrange
        let (_resources, history) = history();
        let page = ObjectId::new(7);
        let request_id = ObjectId::new(1);
        let owner = history
            .record_request(page, request_id, &indexed_request(1))
            .expect("request records");
        history
            .settle(request_id, Some(indexed_response(1)), false)
            .expect("response settles before terminal event");

        // Act
        let response_before_terminal = owner.response().await.expect("response is available");
        let failure_before_terminal = owner.failure_text().expect("failure snapshot reads");
        owner
            .mark_terminal(Some("net::ERR_ABORTED"))
            .expect("failure records");
        let response_after_terminal = owner.response().await.expect("response remains available");
        let failure_after_terminal = owner.failure_text().expect("failure snapshot reads");

        // Assert
        assert_eq!(
            response_before_terminal
                .as_ref()
                .map(|response| response.request_id.as_str()),
            Some("1")
        );
        assert_eq!(failure_before_terminal, None);
        assert_eq!(
            response_after_terminal
                .as_ref()
                .map(|response| response.request_id.as_str()),
            Some("1")
        );
        assert_eq!(failure_after_terminal.as_deref(), Some("net::ERR_ABORTED"));
    }

    #[cfg(feature = "request-history")]
    #[test]
    fn request_owner_foreign_copy_budget_covers_strings_headers_and_body() {
        // Arrange
        let (_resources, history) = history();
        let page = ObjectId::new(7);
        let mut request = indexed_request(1);
        request.headers = vec![("Referer".into(), "https://example.test/source".into())];
        request.post_data = Some(vec![7; 4096]);
        let retained_bytes = request.retained_bytes() as u64;
        let owner = history
            .record_request(page, ObjectId::new(1), &request)
            .expect("request history owns the bounded snapshot");

        // Act
        let allowance = owner.foreign_copy_bytes();

        // Assert
        assert!(allowance >= retained_bytes.saturating_mul(4));
        assert!(allowance >= 8 * 1024);
    }

    #[cfg(feature = "request-history")]
    #[tokio::test]
    async fn native_request_response_waits_for_the_exact_network_identity() {
        // Arrange
        let (_resources, history) = history();
        let history = Arc::new(history);
        let page = ObjectId::new(7);
        let first_request_id = ObjectId::new(1);
        let second_request_id = ObjectId::new(2);
        let mut first_request = indexed_request(1);
        first_request.url = "https://example.test/shared".into();
        history
            .record_request(page, first_request_id, &first_request)
            .expect("nonmatching history request records");
        history
            .settle(first_request_id, Some(indexed_response(1)), true)
            .expect("nonmatching history request settles");
        let stopped = tokio_util::sync::CancellationToken::new();
        let waiting_history = Arc::clone(&history);
        let waiting_stop = stopped.clone();
        let waiter = tokio::spawn(async move {
            waiting_history
                .response_by_native_id(page, "native-2", &waiting_stop, || Ok(false))
                .await
        });
        tokio::task::yield_now().await;
        let mut second_request = indexed_request(2);
        second_request.url = "https://example.test/shared".into();

        // Act
        history
            .record_request(page, second_request_id, &second_request)
            .expect("matching history request records");
        history
            .settle(second_request_id, Some(indexed_response(2)), true)
            .expect("matching history request settles");
        let response = waiter
            .await
            .expect("native identity waiter joins")
            .expect("native identity lookup remains live")
            .expect("matching request has a response");

        // Assert
        assert_eq!(response.request_id, "2");
        assert_eq!(response.status, 200);
    }

    #[cfg(feature = "request-history")]
    #[tokio::test]
    async fn network_wait_history_owner_observes_the_published_response() {
        // Arrange
        let (_resources, history) = history();
        let page = ObjectId::new(7);
        let request_id = ObjectId::new(1);
        let request = NetworkEventRecord::Request(indexed_request(1));
        let response = NetworkEventRecord::Response(indexed_response(1));

        // Act
        let request_owner = history
            .publish(page, request_id, &request)
            .expect("request history publishes")
            .expect("request event creates a history owner");
        let response_owner = history
            .publish(page, request_id, &response)
            .expect("response history publishes")
            .expect("response event resolves the existing history owner");
        let observed = request_owner
            .response()
            .await
            .expect("history owner remains live")
            .expect("published response is available");

        // Assert
        assert!(Arc::ptr_eq(&request_owner, &response_owner));
        assert_eq!(observed.status, 200);
        assert_eq!(observed.request_id, "1");
    }

    #[cfg(feature = "request-history")]
    #[tokio::test]
    async fn last_hundred_requests_and_retained_evictions_remain_functional() {
        // Arrange
        let (_resources, history) = history();
        let page = ObjectId::new(7);
        for index in 1..=100 {
            let request_id = ObjectId::new(index);
            history
                .record_request(page, request_id, &indexed_request(index))
                .unwrap();
            history
                .settle(request_id, Some(indexed_response(index)), true)
                .unwrap();
        }
        let first_snapshot = history.snapshot(page).unwrap();
        let retained_first = first_snapshot.into_iter().skip(1).collect::<Vec<_>>();

        // Act
        for index in 101..=200 {
            let request_id = ObjectId::new(index);
            history
                .record_request(page, request_id, &indexed_request(index))
                .unwrap();
            history
                .settle(request_id, Some(indexed_response(index)), true)
                .unwrap();
        }
        let last_snapshot = history.snapshot(page).unwrap();
        let retained_response = retained_first[0]
            .response()
            .await
            .unwrap()
            .expect("evicted request keeps its response");

        // Assert
        assert_eq!(last_snapshot.len(), 100);
        assert_eq!(
            last_snapshot.first().unwrap().request().url,
            "https://example.test/fetch?101"
        );
        assert_eq!(
            last_snapshot.last().unwrap().request().url,
            "https://example.test/fetch?200"
        );
        assert_eq!(retained_first.len() + last_snapshot.len(), 199);
        assert_eq!(
            retained_response.body.as_deref(),
            Some(b"url:https://example.test/fetch?2".as_slice())
        );
    }
}
