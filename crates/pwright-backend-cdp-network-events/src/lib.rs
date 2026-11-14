//! Isolated Chromium CDP HTTP lifecycle event normalizer for Pwright.
//!
//! The leaf maps one caller-driven Page/session event lane into bounded
//! portable Request/Response lifecycle batches. It owns no receiver, queue,
//! response body, runtime, task, browser session, or browser process.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use base64::Engine as _;
use pwright_browser_model::{REQUEST_OBJECT_KIND, RESPONSE_OBJECT_KIND};
use pwright_cap_network_events::{
    NetworkRequest, NetworkRequestFailed, NetworkRequestFinished, NetworkResponse,
};
use pwright_cdp::CdpEvent;
use pwright_core::{
    ControlMessage, Event, ObjectId, ObjectIdSource, ObjectIdSourceError, ProductionResourceError,
    ProductionResourceLedger, ResourceClass, ResourceError, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use pwright_network_model::{
    HttpHeader, NetworkLifecycleEffect, NetworkLifecycleError, NetworkLifecycleLimits,
    NetworkLifecycleTracker, RedirectResponse, RequestInitializer, RequestStart,
    ResponseInitializer, ResponseStart,
};
#[cfg(feature = "request-timing")]
use pwright_network_model::{RequestTiming, RequestTimingOrigin, ResponseTiming};

pub enum CdpNetworkEventEffect {
    Request(Event<NetworkRequest>),
    Response(Event<NetworkResponse>),
    RequestFinished(Event<NetworkRequestFinished>),
    RequestFailed(Event<NetworkRequestFailed>),
}

#[cfg(feature = "request-frame")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdpRequestFrame {
    native_id: Option<Box<str>>,
    available: bool,
}

/// The five Chromium TLS fields exposed by Playwright. Certificate chains,
/// SANs, signed timestamps, and the rest of CDP's security payload are never
/// copied into this optional capability.
#[cfg(feature = "response-security-details")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdpResponseSecurityDetails {
    protocol: Box<str>,
    subject_name: Box<str>,
    issuer: Box<str>,
    valid_from_micros: i64,
    valid_to_micros: i64,
}

#[cfg(feature = "response-security-details")]
impl CdpResponseSecurityDetails {
    pub fn protocol(&self) -> &str {
        &self.protocol
    }
    pub fn subject_name(&self) -> &str {
        &self.subject_name
    }
    pub fn issuer(&self) -> &str {
        &self.issuer
    }
    pub fn valid_from_seconds(&self) -> f64 {
        self.valid_from_micros as f64 / 1_000_000.0
    }
    pub fn valid_to_seconds(&self) -> f64 {
        self.valid_to_micros as f64 / 1_000_000.0
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.protocol.len()
            + self.subject_name.len()
            + self.issuer.len()
    }
}

#[cfg(feature = "request-frame")]
impl CdpRequestFrame {
    pub fn native_id(&self) -> Option<&str> {
        self.native_id.as_deref()
    }

    pub const fn available(&self) -> bool {
        self.available
    }
}

impl CdpNetworkEventEffect {
    pub fn source(&self) -> ObjectId {
        match self {
            Self::Request(event) => event.object_id(),
            Self::Response(event) => event.object_id(),
            Self::RequestFinished(event) => event.object_id(),
            Self::RequestFailed(event) => event.object_id(),
        }
    }
}

#[derive(Default)]
pub struct CdpNetworkEventBatch {
    page: Option<ObjectId>,
    session_id: Option<Box<str>>,
    native_request_id: Option<Box<str>>,
    lifecycle: Vec<ControlMessage>,
    events: Vec<CdpNetworkEventEffect>,
    #[cfg(feature = "request-frame")]
    request_frames: Vec<(ObjectId, CdpRequestFrame)>,
    #[cfg(feature = "response-security-details")]
    response_security: Vec<(ObjectId, CdpResponseSecurityDetails)>,
    retirement: Vec<ControlMessage>,
}

const ACTIVE_REQUEST_BOOKKEEPING_BYTES: usize = std::mem::size_of::<String>()
    + std::mem::size_of::<RequestInitializer>()
    + std::mem::size_of::<Option<ResponseInitializer>>()
    + (4 * std::mem::size_of::<usize>());
const PENDING_REQUEST_HEADER_BOOKKEEPING_BYTES: usize = std::mem::size_of::<String>()
    + std::mem::size_of::<Vec<HttpHeader>>()
    + (2 * std::mem::size_of::<usize>());
const SUPPRESSED_DATA_REQUEST_BOOKKEEPING_BYTES: usize = std::mem::size_of::<(String, String)>()
    + std::mem::size_of::<SuppressedDataRequestState>()
    + (3 * std::mem::size_of::<usize>());
const LARGE_POST_DATA_REQUEST_BOOKKEEPING_BYTES: usize =
    std::mem::size_of::<(String, String)>() + (3 * std::mem::size_of::<usize>());
const ADAPTER_BOOKKEEPING_BYTES: usize = 128 + std::mem::size_of::<BTreeSet<(String, String)>>();
#[cfg(feature = "response-security-details")]
const MAX_SECURITY_FIELD_BYTES: usize = 4096;
#[cfg(feature = "response-security-details")]
const MAX_SECURITY_DETAILS_BYTES: usize =
    std::mem::size_of::<CdpResponseSecurityDetails>() + 3 * MAX_SECURITY_FIELD_BYTES;
#[cfg(feature = "response-security-details")]
const SECURITY_ENTRY_BOOKKEEPING_BYTES: usize =
    std::mem::size_of::<ObjectId>() + 3 * std::mem::size_of::<usize>();
#[cfg(feature = "request-frame")]
const MAX_OBSERVED_FRAMES_PER_PAGE: usize = 64;
#[cfg(feature = "request-frame")]
const OBSERVED_FRAME_BOOKKEEPING_BYTES: usize =
    std::mem::size_of::<String>() + (3 * std::mem::size_of::<usize>());
#[cfg(feature = "request-frame")]
const REQUEST_FRAME_BOOKKEEPING_BYTES: usize = std::mem::size_of::<ObjectId>()
    + std::mem::size_of::<CdpRequestFrame>()
    + (3 * std::mem::size_of::<usize>());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpNetworkEventProductionProfile {
    limits: NetworkLifecycleLimits,
    ordinary_post_data_bytes: usize,
    large_post_data_requests: usize,
    resource_scope: ResourceScope,
    maximum_retained_bytes: usize,
    requirements: ResourceRequirements,
}

impl CdpNetworkEventProductionProfile {
    pub fn new(
        limits: NetworkLifecycleLimits,
        resource_scope: ResourceScope,
    ) -> Result<Self, CdpNetworkEventError> {
        Self::with_post_data_budget(limits, resource_scope, limits.post_data_bytes, 0)
    }

    /// Selects an exceptional body allowance without multiplying it by every
    /// active Request. The ordinary limit remains available to all Requests;
    /// at most `large_post_data_requests` may exceed it concurrently per Page.
    pub fn with_bounded_large_post_data(
        limits: NetworkLifecycleLimits,
        resource_scope: ResourceScope,
        ordinary_post_data_bytes: usize,
        large_post_data_requests: usize,
    ) -> Result<Self, CdpNetworkEventError> {
        if ordinary_post_data_bytes == 0
            || ordinary_post_data_bytes >= limits.post_data_bytes
            || large_post_data_requests == 0
            || large_post_data_requests > limits.active_requests
        {
            return Err(CdpNetworkEventError::InvalidPostDataBudget);
        }
        Self::with_post_data_budget(
            limits,
            resource_scope,
            ordinary_post_data_bytes,
            large_post_data_requests,
        )
    }

    fn with_post_data_budget(
        limits: NetworkLifecycleLimits,
        resource_scope: ResourceScope,
        ordinary_post_data_bytes: usize,
        large_post_data_requests: usize,
    ) -> Result<Self, CdpNetworkEventError> {
        limits.validate().map_err(CdpNetworkEventError::Lifecycle)?;
        let per_request = maximum_active_request_bytes(limits, ordinary_post_data_bytes)?;
        let per_pending_headers = maximum_pending_request_header_bytes(limits)?;
        let active_bytes = per_request.checked_mul(limits.active_requests).ok_or(
            CdpNetworkEventError::SizeOverflow {
                field: "active request state bytes",
            },
        )?;
        let pending_header_bytes = per_pending_headers
            .checked_mul(limits.active_requests)
            .ok_or(CdpNetworkEventError::SizeOverflow {
                field: "pending request header state bytes",
            })?;
        let suppressed_data_request_bytes = SUPPRESSED_DATA_REQUEST_BOOKKEEPING_BYTES
            .checked_add(limits.native_scope_bytes)
            .and_then(|bytes| bytes.checked_add(limits.native_id_bytes))
            .and_then(|bytes| bytes.checked_mul(limits.active_requests))
            .ok_or(CdpNetworkEventError::SizeOverflow {
                field: "suppressed data request state bytes",
            })?;
        let large_request_bytes = limits
            .post_data_bytes
            .checked_sub(ordinary_post_data_bytes)
            .and_then(|extra| extra.checked_add(LARGE_POST_DATA_REQUEST_BOOKKEEPING_BYTES))
            .and_then(|bytes| bytes.checked_add(limits.native_scope_bytes))
            .and_then(|bytes| bytes.checked_add(limits.native_id_bytes))
            .and_then(|bytes| bytes.checked_mul(large_post_data_requests))
            .ok_or(CdpNetworkEventError::SizeOverflow {
                field: "large post-data request state bytes",
            })?;
        let maximum_retained_bytes = ADAPTER_BOOKKEEPING_BYTES
            .checked_add(limits.native_id_bytes)
            .and_then(|base| base.checked_add(active_bytes))
            .and_then(|base| base.checked_add(pending_header_bytes))
            .and_then(|base| base.checked_add(suppressed_data_request_bytes))
            .and_then(|base| base.checked_add(large_request_bytes))
            .ok_or(CdpNetworkEventError::SizeOverflow {
                field: "network adapter state bytes",
            })?;
        #[cfg(feature = "request-frame")]
        let maximum_retained_bytes = maximum_retained_bytes
            .checked_add(maximum_request_frame_bytes(limits)?)
            .ok_or(CdpNetworkEventError::SizeOverflow {
                field: "request frame state bytes",
            })?;
        #[cfg(feature = "response-security-details")]
        let maximum_retained_bytes = maximum_retained_bytes
            .checked_add(
                (MAX_SECURITY_DETAILS_BYTES + SECURITY_ENTRY_BOOKKEEPING_BYTES)
                    .checked_mul(limits.active_requests)
                    .ok_or(CdpNetworkEventError::SizeOverflow {
                        field: "response security state bytes",
                    })?,
            )
            .ok_or(CdpNetworkEventError::SizeOverflow {
                field: "response security state bytes",
            })?;
        let retained = u64::try_from(maximum_retained_bytes).map_err(|_| {
            CdpNetworkEventError::SizeOverflow {
                field: "active request state bytes",
            }
        })?;
        let requirements = ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, retained, retained)
            .with_metadata(1, 1);
        Ok(Self {
            limits,
            ordinary_post_data_bytes,
            large_post_data_requests,
            resource_scope,
            maximum_retained_bytes,
            requirements,
        })
    }

    pub const fn limits(self) -> NetworkLifecycleLimits {
        self.limits
    }

    pub const fn ordinary_post_data_bytes(self) -> usize {
        self.ordinary_post_data_bytes
    }

    pub const fn large_post_data_requests(self) -> usize {
        self.large_post_data_requests
    }

    pub const fn resource_scope(self) -> ResourceScope {
        self.resource_scope
    }

    pub const fn maximum_retained_bytes(self) -> usize {
        self.maximum_retained_bytes
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

fn maximum_active_request_bytes(
    limits: NetworkLifecycleLimits,
    post_data_bytes: usize,
) -> Result<usize, CdpNetworkEventError> {
    let header_entries = limits
        .header_count
        .checked_mul(std::mem::size_of::<HttpHeader>())
        .and_then(|value| value.checked_mul(2));
    [
        Some(ACTIVE_REQUEST_BOOKKEEPING_BYTES),
        Some(limits.native_scope_bytes),
        Some(limits.native_id_bytes),
        limits.url_bytes.checked_mul(2),
        Some(limits.method_bytes),
        Some(limits.resource_type_bytes),
        limits.header_bytes.checked_mul(2),
        header_entries,
        Some(post_data_bytes),
        Some(limits.status_text_bytes),
    ]
    .into_iter()
    .try_fold(0usize, |total, bytes| {
        total.checked_add(bytes.ok_or(())?).ok_or(())
    })
    .map_err(|()| CdpNetworkEventError::SizeOverflow {
        field: "active request state bytes",
    })
}

fn maximum_pending_request_header_bytes(
    limits: NetworkLifecycleLimits,
) -> Result<usize, CdpNetworkEventError> {
    let header_entries = limits
        .header_count
        .checked_mul(std::mem::size_of::<HttpHeader>());
    [
        Some(PENDING_REQUEST_HEADER_BOOKKEEPING_BYTES),
        Some(limits.native_scope_bytes),
        Some(limits.native_id_bytes),
        Some(limits.header_bytes),
        header_entries,
    ]
    .into_iter()
    .try_fold(0usize, |total, bytes| {
        total.checked_add(bytes.ok_or(())?).ok_or(())
    })
    .map_err(|()| CdpNetworkEventError::SizeOverflow {
        field: "pending request header state bytes",
    })
}

#[cfg(feature = "request-frame")]
fn maximum_request_frame_bytes(
    limits: NetworkLifecycleLimits,
) -> Result<usize, CdpNetworkEventError> {
    let observed_frames = OBSERVED_FRAME_BOOKKEEPING_BYTES
        .checked_add(limits.native_id_bytes)
        .and_then(|bytes| bytes.checked_mul(MAX_OBSERVED_FRAMES_PER_PAGE));
    let active_requests = REQUEST_FRAME_BOOKKEEPING_BYTES
        .checked_add(limits.native_id_bytes)
        .and_then(|bytes| bytes.checked_mul(limits.active_requests));
    observed_frames
        .and_then(|bytes| active_requests.and_then(|active| bytes.checked_add(active)))
        .ok_or(CdpNetworkEventError::SizeOverflow {
            field: "request frame state bytes",
        })
}

impl CdpNetworkEventBatch {
    /// Native CDP session which produced this semantic batch. Empty batches do
    /// not retain correlation metadata.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Native Network request identifier which produced this semantic batch.
    /// This is transport correlation only and is never exposed as a portable
    /// Playwright object identity.
    pub fn native_request_id(&self) -> Option<&str> {
        self.native_request_id.as_deref()
    }

    pub fn lifecycle(&self) -> &[ControlMessage] {
        &self.lifecycle
    }

    pub fn events(&self) -> &[CdpNetworkEventEffect] {
        &self.events
    }

    /// Whether the same ordered CDP cursor had observed the request's Frame
    /// before the lifecycle effect was produced. This preserves Playwright's
    /// popup-first-request behavior without consulting a later topology
    /// snapshot that may already have advanced past the request.
    #[cfg(feature = "request-frame")]
    pub fn request_frame_available(&self, request: ObjectId) -> Option<bool> {
        self.request_frame(request).map(CdpRequestFrame::available)
    }

    #[cfg(feature = "request-frame")]
    pub fn request_frame(&self, request: ObjectId) -> Option<&CdpRequestFrame> {
        self.request_frames
            .iter()
            .rev()
            .find_map(|(candidate, frame)| (*candidate == request).then_some(frame))
    }

    #[cfg(feature = "response-security-details")]
    pub fn response_security(&self, response: ObjectId) -> Option<&CdpResponseSecurityDetails> {
        self.response_security
            .iter()
            .rev()
            .find_map(|(candidate, details)| (*candidate == response).then_some(details))
    }

    /// Parent-root disposals committed only after every semantic event in the
    /// batch has been admitted. Terminal payloads are immutable snapshots, so
    /// they do not require these Request/Response objects to remain live.
    pub fn retirement(&self) -> &[ControlMessage] {
        &self.retirement
    }

    pub fn is_empty(&self) -> bool {
        self.lifecycle.is_empty() && self.events.is_empty() && self.retirement.is_empty()
    }

    pub fn page(&self) -> Option<ObjectId> {
        self.page
    }

    /// Constructs a cleanup-only batch for one retired related native
    /// session. It publishes no semantic event, but preserves the owning Page
    /// so the composition driver can keep its publication barrier exact.
    pub fn from_retirement(page: ObjectId, retirement: Vec<ControlMessage>) -> Self {
        Self {
            page: (!retirement.is_empty()).then_some(page),
            retirement,
            ..Self::default()
        }
    }

    /// Transfers the normalized lifecycle and semantic effects to a
    /// composition root. Keeping both vectors in one value prevents callers
    /// from accidentally publishing an event before admitting its object
    /// creation batch.
    pub fn into_parts(
        self,
    ) -> (
        Vec<ControlMessage>,
        Vec<CdpNetworkEventEffect>,
        Vec<ControlMessage>,
    ) {
        (self.lifecycle, self.events, self.retirement)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SuppressedDataRequestState {
    Active,
    Completed,
}

/// One Page/session-scoped, caller-driven CDP HTTP lifecycle normalizer.
///
/// It owns only bounded native request correlation. It does not own a CDP
/// receiver, event queue, runtime, task, response body, or browser session.
pub struct CdpNetworkEventAdapter {
    page: ObjectId,
    session_id: String,
    object_ids: Arc<dyn ObjectIdSource>,
    tracker: NetworkLifecycleTracker,
    suppressed_data_requests: BTreeMap<(String, String), SuppressedDataRequestState>,
    ordinary_post_data_bytes: usize,
    large_post_data_limit: usize,
    large_post_data_requests: BTreeSet<(String, String)>,
    #[cfg(feature = "request-frame")]
    observed_frames: BTreeSet<String>,
    #[cfg(feature = "request-frame")]
    request_frames: BTreeMap<ObjectId, CdpRequestFrame>,
    #[cfg(feature = "response-security-details")]
    response_security: BTreeMap<ObjectId, CdpResponseSecurityDetails>,
    _resource: Option<ResourceLease>,
}

impl CdpNetworkEventAdapter {
    pub fn new(
        page: ObjectId,
        mut session_id: String,
        object_ids: Arc<dyn ObjectIdSource>,
        limits: NetworkLifecycleLimits,
    ) -> Result<Self, CdpNetworkEventError> {
        if page.get() == 0 {
            return Err(CdpNetworkEventError::ZeroPageObject);
        }
        if session_id.is_empty() {
            return Err(CdpNetworkEventError::EmptySessionId);
        }
        if session_id.len() > limits.native_id_bytes {
            return Err(CdpNetworkEventError::FieldLimitExceeded {
                field: "sessionId",
                actual: session_id.len(),
                limit: limits.native_id_bytes,
            });
        }
        session_id.shrink_to_fit();
        Ok(Self {
            page,
            session_id,
            object_ids,
            tracker: NetworkLifecycleTracker::new(limits)
                .map_err(CdpNetworkEventError::Lifecycle)?,
            suppressed_data_requests: BTreeMap::new(),
            ordinary_post_data_bytes: limits.post_data_bytes,
            large_post_data_limit: 0,
            large_post_data_requests: BTreeSet::new(),
            #[cfg(feature = "request-frame")]
            observed_frames: BTreeSet::new(),
            #[cfg(feature = "request-frame")]
            request_frames: BTreeMap::new(),
            #[cfg(feature = "response-security-details")]
            response_security: BTreeMap::new(),
            _resource: None,
        })
    }

    pub fn new_with_production_resources(
        page: ObjectId,
        session_id: String,
        object_ids: Arc<dyn ObjectIdSource>,
        profile: CdpNetworkEventProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, CdpNetworkEventProductionError> {
        let ledger = resources
            .claim(profile.requirements())
            .map_err(CdpNetworkEventProductionError::Requirements)?;
        let resource = ledger
            .reserve(
                profile.resource_scope(),
                ResourceClass::BackendStateBytes,
                u64::try_from(profile.maximum_retained_bytes())
                    .expect("network production profile validates u64 retention"),
            )
            .map_err(CdpNetworkEventProductionError::Resources)?;
        let mut adapter = Self::new(page, session_id, object_ids, profile.limits())
            .map_err(CdpNetworkEventProductionError::Configuration)?;
        adapter.ordinary_post_data_bytes = profile.ordinary_post_data_bytes();
        adapter.large_post_data_limit = profile.large_post_data_requests();
        adapter._resource = Some(resource);
        Ok(adapter)
    }

    pub const fn page(&self) -> ObjectId {
        self.page
    }

    /// Seeds a Frame which existed before this independent network cursor was
    /// installed. Later Frame lifecycle still comes only from ordered CDP
    /// events, preserving popup-first request semantics.
    #[cfg(feature = "request-frame")]
    pub fn observe_existing_frame(&mut self, frame_id: &str) -> Result<(), CdpNetworkEventError> {
        if frame_id.len() > self.tracker.limits().native_id_bytes {
            return Err(CdpNetworkEventError::FieldLimitExceeded {
                field: "frameId",
                actual: frame_id.len(),
                limit: self.tracker.limits().native_id_bytes,
            });
        }
        if !self.observed_frames.contains(frame_id)
            && self.observed_frames.len() == MAX_OBSERVED_FRAMES_PER_PAGE
        {
            return Err(CdpNetworkEventError::FieldLimitExceeded {
                field: "observed frames",
                actual: self.observed_frames.len() + 1,
                limit: MAX_OBSERVED_FRAMES_PER_PAGE,
            });
        }
        self.observed_frames.insert(frame_id.to_owned());
        Ok(())
    }

    #[cfg(feature = "request-frame")]
    pub fn observe_existing_frames(
        &mut self,
        frame_ids: &[String],
    ) -> Result<(), CdpNetworkEventError> {
        for frame_id in frame_ids {
            self.observe_existing_frame(frame_id)?;
        }
        Ok(())
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn active_request_count(&self) -> usize {
        self.tracker.active_request_count()
    }

    pub fn retained_native_id_capacity(&self) -> usize {
        let suppressed = self
            .suppressed_data_requests
            .keys()
            .map(|(scope, request_id)| scope.capacity().saturating_add(request_id.capacity()))
            .fold(0usize, usize::saturating_add);
        self.large_post_data_requests
            .iter()
            .map(|(scope, request_id)| scope.capacity().saturating_add(request_id.capacity()))
            .fold(
                self.tracker
                    .retained_native_id_capacity()
                    .saturating_add(suppressed),
                usize::saturating_add,
            )
    }

    pub fn active_request_cleanup(&self) -> Vec<ControlMessage> {
        self.tracker
            .active_objects()
            .map(|(request, _response)| ControlMessage::Dispose { object_id: request })
            .collect()
    }

    pub fn map_event(
        &mut self,
        event: &CdpEvent,
    ) -> Result<CdpNetworkEventBatch, CdpNetworkEventError> {
        if event.session_id.as_deref() != Some(self.session_id.as_str()) {
            return Ok(CdpNetworkEventBatch::default());
        }
        self.map_event_in_scope(event, "")
    }

    /// Maps a network event from one explicitly related child session through
    /// this Page's bounded lifecycle tracker. The related session becomes part
    /// of the internal native correlation key; portable Request/Response
    /// identities and Page ownership remain unchanged.
    pub fn map_related_session_event(
        &mut self,
        event: &CdpEvent,
    ) -> Result<CdpNetworkEventBatch, CdpNetworkEventError> {
        let session_id = event
            .session_id
            .as_deref()
            .ok_or(CdpNetworkEventError::EmptySessionId)?;
        if session_id == self.session_id {
            return self.map_event(event);
        }
        if session_id.len() > self.tracker.limits().native_scope_bytes {
            return Err(CdpNetworkEventError::FieldLimitExceeded {
                field: "sessionId",
                actual: session_id.len(),
                limit: self.tracker.limits().native_scope_bytes,
            });
        }
        self.map_event_in_scope(event, session_id)
    }

    fn map_event_in_scope(
        &mut self,
        event: &CdpEvent,
        scope: &str,
    ) -> Result<CdpNetworkEventBatch, CdpNetworkEventError> {
        #[cfg(feature = "request-frame")]
        if self.observe_frame_lifecycle(event)? {
            return Ok(CdpNetworkEventBatch::default());
        }
        if self.suppress_data_request_event(event, scope)? {
            return Ok(CdpNetworkEventBatch::default());
        }
        #[cfg(feature = "request-frame")]
        let new_request_frame = if event.method == "Network.requestWillBeSent" {
            Some(self.frame_for_request_event(event)?)
        } else {
            None
        };
        #[cfg(feature = "response-security-details")]
        let event_security = match event.method.as_str() {
            "Network.responseReceived" => {
                let request_id =
                    required_string(event, "requestId", self.tracker.limits().native_id_bytes)?;
                if self.tracker.has_active_request_in_scope(scope, &request_id) {
                    event
                        .params
                        .get("response")
                        .map(parse_response_security)
                        .transpose()?
                        .flatten()
                } else {
                    None
                }
            }
            "Network.requestWillBeSent" => {
                let request_id =
                    required_string(event, "requestId", self.tracker.limits().native_id_bytes)?;
                if self.tracker.has_active_request_in_scope(scope, &request_id) {
                    event
                        .params
                        .get("redirectResponse")
                        .map(parse_response_security)
                        .transpose()?
                        .flatten()
                } else {
                    None
                }
            }
            _ => None,
        };
        let effects = match event.method.as_str() {
            "Network.requestWillBeSent" => self.request_will_be_sent(event, scope)?,
            "Network.requestWillBeSentExtraInfo" => {
                self.request_will_be_sent_extra_info(event, scope)?;
                Vec::new()
            }
            #[cfg(feature = "request-timing")]
            "Network.requestServedFromCache" => {
                self.request_served_from_cache(event, scope)?;
                Vec::new()
            }
            "Network.responseReceived" => {
                self.response_received(event, scope)?.into_iter().collect()
            }
            "Network.loadingFinished" => self.loading_finished(event, scope)?.into_iter().collect(),
            "Network.loadingFailed" => self.loading_failed(event, scope)?.into_iter().collect(),
            _ => return Ok(CdpNetworkEventBatch::default()),
        };
        if !effects.is_empty()
            && matches!(
                event.method.as_str(),
                "Network.loadingFinished" | "Network.loadingFailed"
            )
        {
            let request_id =
                required_string(event, "requestId", self.tracker.limits().native_id_bytes)?;
            self.large_post_data_requests
                .remove(&(scope.to_owned(), request_id));
        }
        let mut batch = self.portable_batch(
            effects,
            #[cfg(feature = "request-frame")]
            new_request_frame,
            #[cfg(feature = "response-security-details")]
            event_security,
        );
        if !batch.is_empty() {
            let request_id =
                required_string(event, "requestId", self.tracker.limits().native_id_bytes)?;
            batch.page = Some(self.page);
            batch.session_id = event.session_id.clone().map(String::into_boxed_str);
            batch.native_request_id = Some(request_id.into_boxed_str());
        }
        Ok(batch)
    }

    fn is_suppressed_data_request(&self, scope: &str, request_id: &str) -> bool {
        self.suppressed_data_requests
            .keys()
            .any(|(candidate_scope, candidate_id)| {
                candidate_scope == scope && candidate_id == request_id
            })
    }

    fn forget_suppressed_data_request(&mut self, scope: &str, request_id: &str) {
        self.suppressed_data_requests
            .retain(|(candidate_scope, candidate_id), _| {
                candidate_scope != scope || candidate_id != request_id
            });
    }

    /// Playwright's Chromium network manager drops data URLs before creating a
    /// public Request. Keep only bounded native correlation so their URL-less
    /// extra-info and terminal tails cannot leak into a later HTTP request.
    fn suppress_data_request_event(
        &mut self,
        event: &CdpEvent,
        scope: &str,
    ) -> Result<bool, CdpNetworkEventError> {
        match event.method.as_str() {
            "Network.requestWillBeSent" => {
                let url = event
                    .params
                    .get("request")
                    .and_then(serde_json::Value::as_object)
                    .and_then(|request| request.get("url"))
                    .and_then(serde_json::Value::as_str);
                let Some(url) = url else {
                    return Ok(false);
                };
                if !url.starts_with("data:") {
                    if let Some(request_id) = event
                        .params
                        .get("requestId")
                        .and_then(serde_json::Value::as_str)
                    {
                        self.forget_suppressed_data_request(scope, request_id);
                    }
                    return Ok(false);
                }
                let request_id =
                    required_string(event, "requestId", self.tracker.limits().native_id_bytes)?;
                if self.tracker.has_active_request_in_scope(scope, &request_id) {
                    return Err(CdpNetworkEventError::InvalidEvent {
                        method: "Network.requestWillBeSent",
                        field: "data URL reuses an active request ID",
                    });
                }
                if !self.is_suppressed_data_request(scope, &request_id)
                    && self.suppressed_data_requests.len() == self.tracker.limits().active_requests
                {
                    let mut evicted = false;
                    self.suppressed_data_requests.retain(|_, state| {
                        if !evicted && *state == SuppressedDataRequestState::Completed {
                            evicted = true;
                            false
                        } else {
                            true
                        }
                    });
                    if !evicted {
                        return Err(CdpNetworkEventError::FieldLimitExceeded {
                            field: "suppressed data requests",
                            actual: self.suppressed_data_requests.len() + 1,
                            limit: self.tracker.limits().active_requests,
                        });
                    }
                }
                self.tracker
                    .discard_pending_request_headers_in_scope(scope, &request_id)
                    .map_err(CdpNetworkEventError::Lifecycle)?;
                self.suppressed_data_requests.insert(
                    (scope.to_owned(), request_id),
                    SuppressedDataRequestState::Active,
                );
                Ok(true)
            }
            "Network.requestWillBeSentExtraInfo"
            | "Network.responseReceived"
            | "Network.loadingFinished"
            | "Network.loadingFailed"
            | "Network.requestServedFromCache" => {
                let Some(request_id) = event
                    .params
                    .get("requestId")
                    .and_then(serde_json::Value::as_str)
                else {
                    return Ok(false);
                };
                if !self.is_suppressed_data_request(scope, request_id) {
                    return Ok(false);
                }
                if matches!(
                    event.method.as_str(),
                    "Network.loadingFinished" | "Network.loadingFailed"
                ) {
                    self.tracker
                        .discard_pending_request_headers_in_scope(scope, request_id)
                        .map_err(CdpNetworkEventError::Lifecycle)?;
                    if let Some((_, state)) = self.suppressed_data_requests.iter_mut().find(
                        |((candidate_scope, candidate_id), _)| {
                            candidate_scope == scope && candidate_id == request_id
                        },
                    ) {
                        *state = SuppressedDataRequestState::Completed;
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    #[cfg(feature = "request-frame")]
    fn observe_frame_lifecycle(&mut self, event: &CdpEvent) -> Result<bool, CdpNetworkEventError> {
        let limits = self.tracker.limits();
        let (frame_id, attached) = match event.method.as_str() {
            "Page.frameAttached" => (
                required_string(event, "frameId", limits.native_id_bytes)?,
                true,
            ),
            "Page.frameNavigated" => {
                let frame = event
                    .params
                    .get("frame")
                    .and_then(serde_json::Value::as_object)
                    .ok_or(CdpNetworkEventError::InvalidEvent {
                        method: "Page.frameNavigated",
                        field: "frame",
                    })?;
                (
                    required_object_string(
                        frame,
                        "id",
                        "Page.frameNavigated",
                        limits.native_id_bytes,
                    )?,
                    true,
                )
            }
            "Page.frameDetached" | "Page.frameSubtreeWillBeDetached" => (
                required_string(event, "frameId", limits.native_id_bytes)?,
                false,
            ),
            _ => return Ok(false),
        };
        if attached {
            if !self.observed_frames.contains(&frame_id)
                && self.observed_frames.len() == MAX_OBSERVED_FRAMES_PER_PAGE
            {
                return Err(CdpNetworkEventError::FieldLimitExceeded {
                    field: "observed frames",
                    actual: self.observed_frames.len() + 1,
                    limit: MAX_OBSERVED_FRAMES_PER_PAGE,
                });
            }
            self.observed_frames.insert(frame_id);
        } else {
            self.observed_frames.remove(&frame_id);
        }
        Ok(true)
    }

    #[cfg(feature = "request-frame")]
    fn frame_for_request_event(
        &self,
        event: &CdpEvent,
    ) -> Result<CdpRequestFrame, CdpNetworkEventError> {
        let frame_id = optional_string(event, "frameId", self.tracker.limits().native_id_bytes)?;
        let available = frame_id
            .as_ref()
            .is_some_and(|frame_id| self.observed_frames.contains(frame_id));
        Ok(CdpRequestFrame {
            native_id: frame_id.map(String::into_boxed_str),
            available,
        })
    }

    /// Produces parent-owned cleanup for requests still in flight when this
    /// Page/session lane is removed. Finished objects are owned by the Page
    /// runtime tree and are retired by its separate lifecycle/lease policy.
    pub fn drain_active(&mut self) -> Vec<ControlMessage> {
        let cleanup = self.active_request_cleanup();
        self.tracker.drain_active();
        self.suppressed_data_requests.clear();
        self.large_post_data_requests.clear();
        #[cfg(feature = "request-frame")]
        self.request_frames.clear();
        #[cfg(feature = "response-security-details")]
        self.response_security.clear();
        cleanup
    }

    pub fn drain_related_session(&mut self, session_id: &str) -> Vec<ControlMessage> {
        let drained = self.tracker.drain_scope(session_id);
        self.suppressed_data_requests
            .retain(|(scope, _), _| scope != session_id);
        self.large_post_data_requests
            .retain(|(scope, _)| scope != session_id);
        #[cfg(feature = "request-frame")]
        for (request, _) in &drained {
            self.request_frames.remove(request);
        }
        #[cfg(feature = "response-security-details")]
        for (request, _) in &drained {
            self.response_security.remove(request);
        }
        drained
            .into_iter()
            .map(|(request, _)| ControlMessage::Dispose { object_id: request })
            .collect()
    }

    fn request_will_be_sent(
        &mut self,
        event: &CdpEvent,
        scope: &str,
    ) -> Result<Vec<NetworkLifecycleEffect>, CdpNetworkEventError> {
        let limits = self.tracker.limits();
        let request_id = required_string(event, "requestId", limits.native_id_bytes)?;
        let request = event
            .params
            .get("request")
            .and_then(serde_json::Value::as_object)
            .ok_or(CdpNetworkEventError::InvalidEvent {
                method: "Network.requestWillBeSent",
                field: "request",
            })?;
        let url = required_object_string(
            request,
            "url",
            "Network.requestWillBeSent",
            limits.url_bytes,
        )?;
        let method = required_object_string(
            request,
            "method",
            "Network.requestWillBeSent",
            limits.method_bytes,
        )?;
        let resource_type = optional_string(event, "type", limits.resource_type_bytes)?
            .unwrap_or_else(|| "Other".to_owned());
        let headers = parse_headers(request.get("headers"), "Network.requestWillBeSent", limits)?;
        let post_data = parse_post_data(request, limits.post_data_bytes)?;
        let large_post_data = post_data
            .as_ref()
            .is_some_and(|data| data.len() > self.ordinary_post_data_bytes);
        let large_request_key = (scope.to_owned(), request_id.clone());
        if large_post_data
            && !self.large_post_data_requests.contains(&large_request_key)
            && self.large_post_data_requests.len() >= self.large_post_data_limit
        {
            return Err(CdpNetworkEventError::FieldLimitExceeded {
                field: "active large post-data requests",
                actual: self.large_post_data_requests.len() + 1,
                limit: self.large_post_data_limit,
            });
        }
        let redirect = event
            .params
            .get("redirectResponse")
            .map(|response| {
                let response_id = self.next_object_id()?;
                let response = parse_response_value(response, response_id, limits)?;
                Ok::<_, CdpNetworkEventError>(RedirectResponse::new(response_id, response))
            })
            .transpose()?;
        let request_object = self.next_object_id()?;
        let request = RequestStart::new(
            request_id,
            request_object,
            self.page,
            url,
            method,
            resource_type,
        )
        .with_headers(headers)
        .navigation(
            event.params.get("type").and_then(serde_json::Value::as_str) == Some("Document"),
        );
        #[cfg(feature = "request-timing")]
        let request = match parse_request_timing_origin(event)? {
            Some(origin) => request.with_timing_origin(origin),
            None => request,
        };
        let request = if let Some(post_data) = post_data {
            request.with_post_data(post_data)
        } else {
            request
        };
        let effects = self
            .tracker
            .request_will_be_sent_in_scope(scope, request, redirect)
            .map_err(CdpNetworkEventError::Lifecycle)?;
        if large_post_data {
            self.large_post_data_requests.insert(large_request_key);
        } else {
            self.large_post_data_requests.remove(&large_request_key);
        }
        Ok(effects)
    }

    fn request_will_be_sent_extra_info(
        &mut self,
        event: &CdpEvent,
        scope: &str,
    ) -> Result<(), CdpNetworkEventError> {
        let limits = self.tracker.limits();
        let request_id = required_string(event, "requestId", limits.native_id_bytes)?;
        let headers = parse_headers(
            event.params.get("headers"),
            "Network.requestWillBeSentExtraInfo",
            limits,
        )?;
        self.tracker
            .request_will_be_sent_extra_info_in_scope(scope, &request_id, headers)
            .map_err(CdpNetworkEventError::Lifecycle)
    }

    #[cfg(feature = "request-timing")]
    fn request_served_from_cache(
        &mut self,
        event: &CdpEvent,
        scope: &str,
    ) -> Result<(), CdpNetworkEventError> {
        let request_id =
            required_string(event, "requestId", self.tracker.limits().native_id_bytes)?;
        if !self.tracker.has_active_request_in_scope(scope, &request_id) {
            return Ok(());
        }
        self.tracker
            .request_served_from_cache_in_scope(scope, &request_id)
            .map_err(CdpNetworkEventError::Lifecycle)
    }

    fn response_received(
        &mut self,
        event: &CdpEvent,
        scope: &str,
    ) -> Result<Option<NetworkLifecycleEffect>, CdpNetworkEventError> {
        let limits = self.tracker.limits();
        let request_id = required_string(event, "requestId", limits.native_id_bytes)?;
        if !self.tracker.has_active_request_in_scope(scope, &request_id) {
            return Ok(None);
        }
        let response_value =
            event
                .params
                .get("response")
                .ok_or(CdpNetworkEventError::InvalidEvent {
                    method: "Network.responseReceived",
                    field: "response",
                })?;
        let response_id = self.next_object_id()?;
        let response = parse_response_value(response_value, response_id, limits)?;
        ignore_unknown_request(self.tracker.response_received_in_scope(
            scope,
            &request_id,
            response,
        ))
    }

    fn loading_finished(
        &mut self,
        event: &CdpEvent,
        scope: &str,
    ) -> Result<Option<NetworkLifecycleEffect>, CdpNetworkEventError> {
        let limits = self.tracker.limits();
        let request_id = required_string(event, "requestId", limits.native_id_bytes)?;
        if !self.tracker.has_active_request_in_scope(scope, &request_id) {
            return Ok(None);
        }
        let encoded_body_bytes = event
            .params
            .get("encodedDataLength")
            .map(|value| parse_byte_count(value, "Network.loadingFinished", "encodedDataLength"))
            .transpose()?;
        #[cfg(feature = "request-timing")]
        if let Some(timestamp) =
            optional_event_seconds_micros(event, "Network.loadingFinished", "timestamp")?
        {
            return ignore_unknown_request(self.tracker.loading_finished_timed_in_scope(
                scope,
                &request_id,
                encoded_body_bytes,
                timestamp,
            ));
        }
        ignore_unknown_request(self.tracker.loading_finished_in_scope(
            scope,
            &request_id,
            encoded_body_bytes,
        ))
    }

    fn loading_failed(
        &mut self,
        event: &CdpEvent,
        scope: &str,
    ) -> Result<Option<NetworkLifecycleEffect>, CdpNetworkEventError> {
        let limits = self.tracker.limits();
        let request_id = required_string(event, "requestId", limits.native_id_bytes)?;
        if !self.tracker.has_active_request_in_scope(scope, &request_id) {
            return Ok(None);
        }
        let failure_text = required_string(event, "errorText", limits.failure_text_bytes)?;
        let canceled = event
            .params
            .get("canceled")
            .map(|value| {
                value.as_bool().ok_or(CdpNetworkEventError::InvalidEvent {
                    method: "Network.loadingFailed",
                    field: "canceled",
                })
            })
            .transpose()?
            .unwrap_or(false);
        #[cfg(feature = "request-timing")]
        if let Some(timestamp) =
            optional_event_seconds_micros(event, "Network.loadingFailed", "timestamp")?
        {
            return ignore_unknown_request(self.tracker.loading_failed_timed_in_scope(
                scope,
                &request_id,
                failure_text,
                canceled,
                timestamp,
            ));
        }
        ignore_unknown_request(self.tracker.loading_failed_in_scope(
            scope,
            &request_id,
            failure_text,
            canceled,
        ))
    }

    fn portable_batch(
        &mut self,
        effects: Vec<NetworkLifecycleEffect>,
        #[cfg(feature = "request-frame")] new_request_frame: Option<CdpRequestFrame>,
        #[cfg(feature = "response-security-details")] event_security: Option<
            CdpResponseSecurityDetails,
        >,
    ) -> CdpNetworkEventBatch {
        let mut lifecycle = Vec::with_capacity(effects.len());
        let mut events = Vec::with_capacity(effects.len());
        #[cfg(feature = "request-frame")]
        let mut request_frames = Vec::with_capacity(effects.len());
        #[cfg(feature = "response-security-details")]
        let mut response_security = Vec::with_capacity(effects.len());
        let mut retirement = Vec::with_capacity(effects.len());
        for effect in effects {
            #[cfg(feature = "request-frame")]
            let request_identity = lifecycle_request_identity(&effect);
            #[cfg(feature = "request-frame")]
            let frame = match &effect {
                NetworkLifecycleEffect::Request(_) => {
                    new_request_frame.clone().unwrap_or(CdpRequestFrame {
                        native_id: None,
                        available: false,
                    })
                }
                _ => self
                    .request_frames
                    .get(&request_identity)
                    .cloned()
                    .unwrap_or(CdpRequestFrame {
                        native_id: None,
                        available: false,
                    }),
            };
            #[cfg(feature = "request-frame")]
            request_frames.push((request_identity, frame.clone()));
            match effect {
                NetworkLifecycleEffect::Request(payload) => {
                    #[cfg(feature = "request-frame")]
                    self.request_frames.insert(payload.request, frame);
                    lifecycle.push(ControlMessage::Create {
                        object_id: payload.request,
                        parent: Some(payload.parent),
                        kind: REQUEST_OBJECT_KIND.tag,
                    });
                    events.push(CdpNetworkEventEffect::Request(Event::new(
                        self.page, payload,
                    )));
                }
                NetworkLifecycleEffect::Response(payload) => {
                    #[cfg(feature = "response-security-details")]
                    if let Some(details) = event_security.as_ref() {
                        self.response_security
                            .insert(payload.request, details.clone());
                        response_security.push((payload.response, details.clone()));
                    }
                    lifecycle.push(ControlMessage::Create {
                        object_id: payload.response,
                        parent: Some(payload.request),
                        kind: RESPONSE_OBJECT_KIND.tag,
                    });
                    events.push(CdpNetworkEventEffect::Response(Event::new(
                        self.page, payload,
                    )));
                }
                NetworkLifecycleEffect::RequestFinished(payload) => {
                    #[cfg(feature = "response-security-details")]
                    if let (Some(response), Some(details)) = (
                        payload.response.as_ref(),
                        self.response_security.remove(&payload.request.request),
                    ) {
                        response_security.push((response.response, details));
                    }
                    #[cfg(feature = "request-frame")]
                    self.request_frames.remove(&payload.request.request);
                    retirement.push(ControlMessage::Dispose {
                        object_id: payload.request.request,
                    });
                    events.push(CdpNetworkEventEffect::RequestFinished(Event::new(
                        self.page, payload,
                    )));
                }
                NetworkLifecycleEffect::RequestFailed(payload) => {
                    #[cfg(feature = "response-security-details")]
                    if let (Some(response), Some(details)) = (
                        payload.response.as_ref(),
                        self.response_security.remove(&payload.request.request),
                    ) {
                        response_security.push((response.response, details));
                    }
                    #[cfg(feature = "request-frame")]
                    self.request_frames.remove(&payload.request.request);
                    retirement.push(ControlMessage::Dispose {
                        object_id: payload.request.request,
                    });
                    events.push(CdpNetworkEventEffect::RequestFailed(Event::new(
                        self.page, payload,
                    )));
                }
            }
        }
        CdpNetworkEventBatch {
            page: Some(self.page),
            session_id: None,
            native_request_id: None,
            lifecycle,
            events,
            #[cfg(feature = "request-frame")]
            request_frames,
            #[cfg(feature = "response-security-details")]
            response_security,
            retirement,
        }
    }

    fn next_object_id(&self) -> Result<ObjectId, CdpNetworkEventError> {
        self.object_ids
            .next_object_id()
            .map_err(CdpNetworkEventError::ObjectIds)
    }
}

#[cfg(feature = "request-frame")]
fn lifecycle_request_identity(effect: &NetworkLifecycleEffect) -> ObjectId {
    match effect {
        NetworkLifecycleEffect::Request(request) => request.request,
        NetworkLifecycleEffect::Response(response) => response.request,
        NetworkLifecycleEffect::RequestFinished(finished) => finished.request.request,
        NetworkLifecycleEffect::RequestFailed(failed) => failed.request.request,
    }
}

fn ignore_unknown_request(
    result: Result<NetworkLifecycleEffect, NetworkLifecycleError>,
) -> Result<Option<NetworkLifecycleEffect>, CdpNetworkEventError> {
    match result {
        Ok(effect) => Ok(Some(effect)),
        Err(NetworkLifecycleError::UnknownNativeRequest { .. }) => Ok(None),
        Err(error) => Err(CdpNetworkEventError::Lifecycle(error)),
    }
}

fn required_string(
    event: &CdpEvent,
    field: &'static str,
    limit: usize,
) -> Result<String, CdpNetworkEventError> {
    let value = event
        .params
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or(CdpNetworkEventError::InvalidEvent {
            method: event_method(event),
            field,
        })?;
    bounded_string(field, value, limit)
}

fn optional_string(
    event: &CdpEvent,
    field: &'static str,
    limit: usize,
) -> Result<Option<String>, CdpNetworkEventError> {
    event
        .params
        .get(field)
        .map(|value| {
            value
                .as_str()
                .ok_or(CdpNetworkEventError::InvalidEvent {
                    method: event_method(event),
                    field,
                })
                .and_then(|value| bounded_string(field, value, limit))
        })
        .transpose()
}

fn required_object_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
    method: &'static str,
    limit: usize,
) -> Result<String, CdpNetworkEventError> {
    let value = object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or(CdpNetworkEventError::InvalidEvent { method, field })?;
    bounded_string(field, value, limit)
}

fn bounded_string(
    field: &'static str,
    value: &str,
    limit: usize,
) -> Result<String, CdpNetworkEventError> {
    if value.len() > limit {
        return Err(CdpNetworkEventError::FieldLimitExceeded {
            field,
            actual: value.len(),
            limit,
        });
    }
    let mut owned = value.to_owned();
    owned.shrink_to_fit();
    Ok(owned)
}

fn bounded_bytes(
    field: &'static str,
    value: &[u8],
    limit: usize,
) -> Result<Vec<u8>, CdpNetworkEventError> {
    if value.len() > limit {
        return Err(CdpNetworkEventError::FieldLimitExceeded {
            field,
            actual: value.len(),
            limit,
        });
    }
    Ok(value.to_vec())
}

fn parse_post_data(
    request: &serde_json::Map<String, serde_json::Value>,
    limit: usize,
) -> Result<Option<Vec<u8>>, CdpNetworkEventError> {
    let Some(entries) = request.get("postDataEntries") else {
        return request
            .get("postData")
            .map(|value| {
                value
                    .as_str()
                    .ok_or(CdpNetworkEventError::InvalidEvent {
                        method: "Network.requestWillBeSent",
                        field: "request.postData",
                    })
                    .and_then(|value| bounded_bytes("request.postData", value.as_bytes(), limit))
            })
            .transpose();
    };
    let entries = entries
        .as_array()
        .ok_or(CdpNetworkEventError::InvalidEvent {
            method: "Network.requestWillBeSent",
            field: "request.postDataEntries",
        })?;
    if entries.len() > limit {
        return Err(CdpNetworkEventError::FieldLimitExceeded {
            field: "request.postDataEntries count",
            actual: entries.len(),
            limit,
        });
    }
    let mut decoded = Vec::new();
    for entry in entries {
        let entry = entry
            .as_object()
            .ok_or(CdpNetworkEventError::InvalidEvent {
                method: "Network.requestWillBeSent",
                field: "request.postDataEntries entry",
            })?;
        let Some(bytes) = entry.get("bytes") else {
            continue;
        };
        let bytes = bytes.as_str().ok_or(CdpNetworkEventError::InvalidEvent {
            method: "Network.requestWillBeSent",
            field: "request.postDataEntries.bytes",
        })?;
        if bytes.is_empty() {
            continue;
        }
        let maximum_encoded_bytes = limit
            .checked_mul(4)
            .and_then(|value| value.checked_div(3))
            .and_then(|value| value.checked_add(4))
            .ok_or(CdpNetworkEventError::SizeOverflow {
                field: "request post-data base64 bound",
            })?;
        if bytes.len() > maximum_encoded_bytes {
            return Err(CdpNetworkEventError::FieldLimitExceeded {
                field: "request.postDataEntries.bytes",
                actual: bytes.len(),
                limit: maximum_encoded_bytes,
            });
        }
        let entry = base64::engine::general_purpose::STANDARD
            .decode(bytes)
            .map_err(|_| CdpNetworkEventError::InvalidEvent {
                method: "Network.requestWillBeSent",
                field: "request.postDataEntries.bytes",
            })?;
        let total =
            decoded
                .len()
                .checked_add(entry.len())
                .ok_or(CdpNetworkEventError::SizeOverflow {
                    field: "request post-data bytes",
                })?;
        if total > limit {
            return Err(CdpNetworkEventError::FieldLimitExceeded {
                field: "request.postData bytes",
                actual: total,
                limit,
            });
        }
        decoded.extend_from_slice(&entry);
    }
    if decoded.is_empty() {
        Ok(None)
    } else {
        decoded.shrink_to_fit();
        Ok(Some(decoded))
    }
}

fn parse_headers(
    value: Option<&serde_json::Value>,
    method: &'static str,
    limits: NetworkLifecycleLimits,
) -> Result<Vec<HttpHeader>, CdpNetworkEventError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let object = value
        .as_object()
        .ok_or(CdpNetworkEventError::InvalidEvent {
            method,
            field: "headers",
        })?;
    if object.len() > limits.header_count {
        return Err(CdpNetworkEventError::FieldLimitExceeded {
            field: "header count",
            actual: object.len(),
            limit: limits.header_count,
        });
    }
    let mut bytes = 0_usize;
    for (name, value) in object {
        if name.is_empty() {
            return Err(CdpNetworkEventError::InvalidEvent {
                method,
                field: "header name",
            });
        }
        let value = value.as_str().ok_or(CdpNetworkEventError::InvalidEvent {
            method,
            field: "header value",
        })?;
        bytes = bytes
            .checked_add(name.len())
            .and_then(|total| total.checked_add(value.len()))
            .ok_or(CdpNetworkEventError::SizeOverflow {
                field: "header bytes",
            })?;
        if bytes > limits.header_bytes {
            return Err(CdpNetworkEventError::FieldLimitExceeded {
                field: "header bytes",
                actual: bytes,
                limit: limits.header_bytes,
            });
        }
    }
    let mut headers = object
        .iter()
        .map(|(name, value)| {
            HttpHeader::new(
                name.clone(),
                value
                    .as_str()
                    .expect("header values were validated")
                    .to_owned(),
            )
        })
        .collect::<Vec<_>>();
    headers.sort_by(|left, right| left.name.cmp(&right.name));
    headers.shrink_to_fit();
    Ok(headers)
}

fn parse_response_value(
    value: &serde_json::Value,
    response: ObjectId,
    limits: NetworkLifecycleLimits,
) -> Result<ResponseStart, CdpNetworkEventError> {
    let object = value
        .as_object()
        .ok_or(CdpNetworkEventError::InvalidEvent {
            method: "Network response",
            field: "response",
        })?;
    let url = required_object_string(object, "url", "Network response", limits.url_bytes)?;
    let status_value = object
        .get("status")
        .ok_or(CdpNetworkEventError::InvalidEvent {
            method: "Network response",
            field: "status",
        })?;
    let status = if let Some(status) = status_value.as_i64() {
        status
    } else {
        let status = status_value
            .as_f64()
            .ok_or(CdpNetworkEventError::InvalidEvent {
                method: "Network response",
                field: "status",
            })?;
        if !status.is_finite()
            || status.fract() != 0.0
            || status < i64::MIN as f64
            || status >= i64::MAX as f64
        {
            return Err(CdpNetworkEventError::InvalidNumber {
                method: "Network response",
                field: "status",
            });
        }
        status as i64
    };
    let status_text = required_object_string(
        object,
        "statusText",
        "Network response",
        limits.status_text_bytes,
    )?;
    let headers = parse_headers(object.get("headers"), "Network response", limits)?;
    let from_service_worker = object
        .get("fromServiceWorker")
        .map(|value| {
            value.as_bool().ok_or(CdpNetworkEventError::InvalidEvent {
                method: "Network response",
                field: "fromServiceWorker",
            })
        })
        .transpose()?
        .unwrap_or(false);
    let response = ResponseStart::new(response, url, status, status_text)
        .with_headers(headers)
        .from_service_worker(from_service_worker);
    #[cfg(feature = "request-timing")]
    let response = match object.get("timing") {
        Some(value) => response.with_timing(parse_response_timing(value)?),
        None => response,
    };
    Ok(response)
}

#[cfg(feature = "response-security-details")]
fn parse_response_security(
    value: &serde_json::Value,
) -> Result<Option<CdpResponseSecurityDetails>, CdpNetworkEventError> {
    let response = value
        .as_object()
        .ok_or(CdpNetworkEventError::InvalidEvent {
            method: "Network response",
            field: "response",
        })?;
    let Some(details) = response.get("securityDetails") else {
        return Ok(None);
    };
    let details = details
        .as_object()
        .ok_or(CdpNetworkEventError::InvalidEvent {
            method: "Network response",
            field: "securityDetails",
        })?;
    let bounded = |name: &'static str| -> Result<Box<str>, CdpNetworkEventError> {
        let value = details
            .get(name)
            .and_then(serde_json::Value::as_str)
            .ok_or(CdpNetworkEventError::InvalidEvent {
                method: "Network response securityDetails",
                field: name,
            })?;
        if value.len() > MAX_SECURITY_FIELD_BYTES {
            return Err(CdpNetworkEventError::FieldLimitExceeded {
                field: name,
                actual: value.len(),
                limit: MAX_SECURITY_FIELD_BYTES,
            });
        }
        Ok(Box::<str>::from(value))
    };
    let micros = |name: &'static str| -> Result<i64, CdpNetworkEventError> {
        let value = details
            .get(name)
            .and_then(serde_json::Value::as_f64)
            .ok_or(CdpNetworkEventError::InvalidEvent {
                method: "Network response securityDetails",
                field: name,
            })?;
        let scaled = value * 1_000_000.0;
        if !value.is_finite()
            || !scaled.is_finite()
            || scaled < i64::MIN as f64
            || scaled >= i64::MAX as f64
        {
            return Err(CdpNetworkEventError::InvalidNumber {
                method: "Network response securityDetails",
                field: name,
            });
        }
        Ok(scaled.round() as i64)
    };
    Ok(Some(CdpResponseSecurityDetails {
        protocol: bounded("protocol")?,
        subject_name: bounded("subjectName")?,
        issuer: bounded("issuer")?,
        valid_from_micros: micros("validFrom")?,
        valid_to_micros: micros("validTo")?,
    }))
}

#[cfg(feature = "request-timing")]
fn parse_request_timing_origin(
    event: &CdpEvent,
) -> Result<Option<RequestTimingOrigin>, CdpNetworkEventError> {
    let timestamp = optional_event_seconds_micros(event, "Network.requestWillBeSent", "timestamp")?;
    let wall_time = optional_event_seconds_micros(event, "Network.requestWillBeSent", "wallTime")?;
    match (timestamp, wall_time) {
        (Some(timestamp), Some(wall_time)) => {
            Ok(Some(RequestTimingOrigin::new(timestamp, wall_time)))
        }
        (None, None) => Ok(None),
        (None, Some(_)) => Err(CdpNetworkEventError::InvalidEvent {
            method: "Network.requestWillBeSent",
            field: "timestamp",
        }),
        (Some(_), None) => Err(CdpNetworkEventError::InvalidEvent {
            method: "Network.requestWillBeSent",
            field: "wallTime",
        }),
    }
}

#[cfg(feature = "request-timing")]
fn parse_response_timing(
    value: &serde_json::Value,
) -> Result<ResponseTiming, CdpNetworkEventError> {
    let object = value
        .as_object()
        .ok_or(CdpNetworkEventError::InvalidEvent {
            method: "Network response timing",
            field: "timing",
        })?;
    Ok(ResponseTiming {
        request_time_micros: required_number_micros(
            object,
            "requestTime",
            "Network response timing",
            1_000_000.0,
        )?,
        domain_lookup_start_micros: required_connection_timing_micros(
            object,
            "dnsStart",
            "Network response timing",
        )?,
        domain_lookup_end_micros: required_connection_timing_micros(
            object,
            "dnsEnd",
            "Network response timing",
        )?,
        connect_start_micros: required_connection_timing_micros(
            object,
            "connectStart",
            "Network response timing",
        )?,
        secure_connection_start_micros: required_connection_timing_micros(
            object,
            "sslStart",
            "Network response timing",
        )?,
        connect_end_micros: required_connection_timing_micros(
            object,
            "connectEnd",
            "Network response timing",
        )?,
        request_start_micros: required_number_micros(
            object,
            "sendStart",
            "Network response timing",
            1_000.0,
        )?,
        response_start_micros: required_number_micros(
            object,
            "receiveHeadersEnd",
            "Network response timing",
            1_000.0,
        )?,
    })
}

#[cfg(feature = "request-timing")]
fn required_connection_timing_micros(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
    method: &'static str,
) -> Result<i64, CdpNetworkEventError> {
    let value = object
        .get(field)
        .ok_or(CdpNetworkEventError::InvalidEvent { method, field })?;
    let micros = number_micros(value, method, field, 1_000.0)?;
    Ok(if value.as_f64() == Some(0.0) {
        RequestTiming::UNAVAILABLE_MICROS
    } else {
        micros
    })
}

#[cfg(feature = "request-timing")]
fn optional_event_seconds_micros(
    event: &CdpEvent,
    method: &'static str,
    field: &'static str,
) -> Result<Option<i64>, CdpNetworkEventError> {
    event
        .params
        .get(field)
        .map(|value| number_micros(value, method, field, 1_000_000.0))
        .transpose()
}

#[cfg(feature = "request-timing")]
fn required_number_micros(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
    method: &'static str,
    scale: f64,
) -> Result<i64, CdpNetworkEventError> {
    let value = object
        .get(field)
        .ok_or(CdpNetworkEventError::InvalidEvent { method, field })?;
    number_micros(value, method, field, scale)
}

#[cfg(feature = "request-timing")]
fn number_micros(
    value: &serde_json::Value,
    method: &'static str,
    field: &'static str,
    scale: f64,
) -> Result<i64, CdpNetworkEventError> {
    let value = value
        .as_f64()
        .ok_or(CdpNetworkEventError::InvalidEvent { method, field })?;
    let scaled = value * scale;
    if !scaled.is_finite() || scaled < i64::MIN as f64 || scaled >= i64::MAX as f64 {
        return Err(CdpNetworkEventError::InvalidNumber { method, field });
    }
    let rounded = scaled.round() as i64;
    Ok(match (rounded, scaled.is_sign_positive(), scaled == 0.0) {
        (0, true, false) => 1,
        (0, false, false) => -1,
        _ => rounded,
    })
}

fn parse_byte_count(
    value: &serde_json::Value,
    method: &'static str,
    field: &'static str,
) -> Result<u64, CdpNetworkEventError> {
    if let Some(value) = value.as_u64() {
        return Ok(value);
    }
    let value = value
        .as_f64()
        .ok_or(CdpNetworkEventError::InvalidEvent { method, field })?;
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value >= u64::MAX as f64 {
        return Err(CdpNetworkEventError::InvalidNumber { method, field });
    }
    Ok(value as u64)
}

fn event_method(event: &CdpEvent) -> &'static str {
    match event.method.as_str() {
        "Network.requestWillBeSent" => "Network.requestWillBeSent",
        "Network.requestWillBeSentExtraInfo" => "Network.requestWillBeSentExtraInfo",
        "Network.requestServedFromCache" => "Network.requestServedFromCache",
        "Network.responseReceived" => "Network.responseReceived",
        "Network.loadingFinished" => "Network.loadingFinished",
        "Network.loadingFailed" => "Network.loadingFailed",
        "Page.frameAttached" => "Page.frameAttached",
        "Page.frameNavigated" => "Page.frameNavigated",
        "Page.frameDetached" => "Page.frameDetached",
        "Page.frameSubtreeWillBeDetached" => "Page.frameSubtreeWillBeDetached",
        _ => "Network event",
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdpNetworkEventError {
    ZeroPageObject,
    EmptySessionId,
    InvalidPostDataBudget,
    InvalidEvent {
        method: &'static str,
        field: &'static str,
    },
    InvalidNumber {
        method: &'static str,
        field: &'static str,
    },
    FieldLimitExceeded {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    SizeOverflow {
        field: &'static str,
    },
    ObjectIds(ObjectIdSourceError),
    Lifecycle(NetworkLifecycleError),
}

impl Display for CdpNetworkEventError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroPageObject => formatter.write_str("network Page object ID zero is invalid"),
            Self::EmptySessionId => formatter.write_str("network CDP session ID is empty"),
            Self::InvalidPostDataBudget => formatter.write_str(
                "network large post-data budget requires a smaller positive ordinary limit and 1..=active-request large slots",
            ),
            Self::InvalidEvent { method, field } => {
                write!(formatter, "{method} has an invalid or missing `{field}`")
            }
            Self::InvalidNumber { method, field } => {
                write!(formatter, "{method} has an invalid numeric `{field}`")
            }
            Self::FieldLimitExceeded {
                field,
                actual,
                limit,
            } => write!(
                formatter,
                "CDP network {field} uses {actual} bytes/items, exceeding limit {limit}"
            ),
            Self::SizeOverflow { field } => {
                write!(formatter, "CDP network {field} calculation overflowed")
            }
            Self::ObjectIds(error) => Display::fmt(error, formatter),
            Self::Lifecycle(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for CdpNetworkEventError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ObjectIds(error) => Some(error),
            Self::Lifecycle(error) => Some(error),
            Self::ZeroPageObject
            | Self::EmptySessionId
            | Self::InvalidPostDataBudget
            | Self::InvalidEvent { .. }
            | Self::InvalidNumber { .. }
            | Self::FieldLimitExceeded { .. }
            | Self::SizeOverflow { .. } => None,
        }
    }
}

#[derive(Debug)]
pub enum CdpNetworkEventProductionError {
    Configuration(CdpNetworkEventError),
    Requirements(ProductionResourceError),
    Resources(ResourceError),
}

impl Display for CdpNetworkEventProductionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(error) => Display::fmt(error, formatter),
            Self::Requirements(error) => Display::fmt(error, formatter),
            Self::Resources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for CdpNetworkEventProductionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Configuration(error) => Some(error),
            Self::Requirements(error) => Some(error),
            Self::Resources(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use base64::Engine as _;
    use pwright_cap_network_events::{NetworkRequest, NetworkRequestFailed, NetworkResponse};
    use pwright_cdp::CdpEvent;
    use pwright_core::{
        AtomicObjectIdSource, ControlMessage, ObjectId, ProductionResourceLedger, ResourceClass,
        ResourceLimits, ResourceScope,
    };
    use pwright_network_model::NetworkLifecycleLimits;
    use serde_json::json;

    #[cfg(feature = "request-frame")]
    use super::CdpRequestFrame;
    use super::{
        CdpNetworkEventAdapter, CdpNetworkEventEffect, CdpNetworkEventError,
        CdpNetworkEventProductionProfile,
    };
    #[cfg(feature = "request-timing")]
    use super::{number_micros, required_connection_timing_micros};

    fn limits(active_requests: usize) -> NetworkLifecycleLimits {
        NetworkLifecycleLimits::new(active_requests, 64, 256, 16, 32, 8, 256, 64, 64, 128)
    }

    fn adapter(active_requests: usize) -> CdpNetworkEventAdapter {
        CdpNetworkEventAdapter::new(
            ObjectId::new(3),
            "session-1".to_owned(),
            Arc::new(AtomicObjectIdSource::new(10).expect("object IDs should validate")),
            limits(active_requests),
        )
        .expect("adapter should build")
    }

    fn event(method: &str, params: serde_json::Value) -> CdpEvent {
        CdpEvent::new(method.to_owned(), params, Some("session-1".to_owned()))
    }

    fn request_event(request_id: &str, url: &str) -> CdpEvent {
        event(
            "Network.requestWillBeSent",
            json!({
                "requestId": request_id,
                "type": "Fetch",
                "request": {
                    "url": url,
                    "method": "GET",
                    "headers": {"z-last": "2", "accept": "application/json"}
                }
            }),
        )
    }

    #[cfg(feature = "response-security-details")]
    fn security_details() -> serde_json::Value {
        json!({
            "protocol": "TLS 1.3",
            "subjectName": "example.test",
            "issuer": "Test Authority",
            "validFrom": 1_700_000_000.25,
            "validTo": 1_800_000_000.5,
            "sanList": ["this large certificate field is deliberately ignored"]
        })
    }

    #[cfg(feature = "response-security-details")]
    fn response_event(request_id: &str, url: &str, details: Option<serde_json::Value>) -> CdpEvent {
        let mut response = json!({
            "url": url,
            "status": 200,
            "statusText": "OK",
            "headers": {}
        });
        if let Some(details) = details {
            response["securityDetails"] = details;
        }
        event(
            "Network.responseReceived",
            json!({"requestId": request_id, "response": response}),
        )
    }

    fn post_data_event(request_id: &str, body: &str) -> CdpEvent {
        event(
            "Network.requestWillBeSent",
            json!({
                "requestId": request_id,
                "type": "Fetch",
                "request": {
                    "url": "https://example.test/submit",
                    "method": "POST",
                    "headers": {},
                    "postData": body
                }
            }),
        )
    }

    fn bounded_large_post_data_adapter(
        active_requests: usize,
        large_requests: usize,
    ) -> CdpNetworkEventAdapter {
        let mut lifecycle = limits(active_requests);
        lifecycle.post_data_bytes = 3 * 1024 * 1024;
        lifecycle.native_scope_bytes = 64;
        let profile = CdpNetworkEventProductionProfile::with_bounded_large_post_data(
            lifecycle,
            ResourceScope::extension(95, 1),
            256 * 1024,
            large_requests,
        )
        .expect("large post-data profile should validate");
        let requirements = profile.requirements();
        let class = requirements.class(ResourceClass::BackendStateBytes);
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::BackendStateBytes,
                class.global,
                class.per_scope,
            ),
            requirements,
        )
        .expect("production resources should validate");
        CdpNetworkEventAdapter::new_with_production_resources(
            ObjectId::new(3),
            "session-1".to_owned(),
            Arc::new(AtomicObjectIdSource::new(10).expect("object IDs should validate")),
            profile,
            &resources,
        )
        .expect("large post-data adapter should reserve its state")
    }

    #[test]
    fn data_url_navigation_has_no_public_request_or_object_allocation() {
        // Arrange
        let mut adapter = adapter(1);
        let data = request_event("data-document", "data:text/html,<div>yo</div>");

        // Act
        let ignored = adapter
            .map_event(&data)
            .expect("data URL should be ignored");
        let next = adapter
            .map_event(&request_event("http-document", "https://example.test/"))
            .expect("ordinary request should still map");

        // Assert
        assert!(ignored.is_empty());
        assert_eq!(adapter.active_request_count(), 1);
        let CdpNetworkEventEffect::Request(request) = &next.events()[0] else {
            panic!("ordinary request should be published");
        };
        assert_eq!(request.payload().request, ObjectId::new(10));
    }

    #[test]
    fn data_url_companion_events_leave_no_pending_headers_or_public_lifecycle() {
        // Arrange
        let mut adapter = adapter(1);
        let early_headers = event(
            "Network.requestWillBeSentExtraInfo",
            json!({"requestId": "reused", "headers": {"x-data": "early"}}),
        );
        adapter
            .map_event(&early_headers)
            .expect("early headers should fit the pending budget");
        let data = request_event("reused", "data:text/html,hello");
        let late_headers = event(
            "Network.requestWillBeSentExtraInfo",
            json!({"requestId": "reused", "headers": {"x-data": "late"}}),
        );
        let response = event(
            "Network.responseReceived",
            json!({"requestId": "reused", "response": {"url": "data:text/html,hello"}}),
        );
        let finished = event(
            "Network.loadingFinished",
            json!({"requestId": "reused", "encodedDataLength": 0}),
        );

        // Act
        let batches = [data, late_headers, response, finished]
            .iter()
            .map(|event| {
                adapter
                    .map_event(event)
                    .expect("data lifecycle should be ignored")
            })
            .collect::<Vec<_>>();
        let after_finish = event(
            "Network.requestWillBeSentExtraInfo",
            json!({"requestId": "reused", "headers": {"x-data": "after-finish"}}),
        );
        let after_finish_batch = adapter
            .map_event(&after_finish)
            .expect("late extra info should still be ignored");
        let pending_after_finish = adapter.tracker.retained_native_id_capacity();
        let ordinary = adapter
            .map_event(&request_event("reused", "https://example.test/"))
            .expect("reused native ID should start an ordinary request");

        // Assert
        assert!(batches.iter().all(|batch| batch.is_empty()));
        assert!(after_finish_batch.is_empty());
        assert_eq!(pending_after_finish, 0);
        assert!(adapter.suppressed_data_requests.is_empty());
        let CdpNetworkEventEffect::Request(request) = &ordinary.events()[0] else {
            panic!("ordinary request should be published");
        };
        assert!(
            request
                .payload()
                .headers
                .iter()
                .all(|header| header.name != "x-data")
        );
    }

    #[test]
    fn data_url_suppression_is_bounded_and_terminal_cleanup_releases_capacity() {
        // Arrange
        let mut adapter = adapter(1);
        let first = request_event("first", "data:text/html,one");
        let second = request_event("second", "data:text/html,two");
        adapter
            .map_event(&first)
            .expect("first data URL should fit");

        // Act
        let overflow = match adapter.map_event(&second) {
            Ok(_) => panic!("a second in-flight data URL must exceed the budget"),
            Err(error) => error,
        };
        let finished = adapter
            .map_event(&event(
                "Network.loadingFinished",
                json!({"requestId": "first"}),
            ))
            .expect("terminal event should clear the suppression");
        let second_result = adapter
            .map_event(&second)
            .expect("capacity should be reusable after terminal cleanup");
        let cleanup = adapter.drain_active();

        // Assert
        assert_eq!(
            overflow,
            CdpNetworkEventError::FieldLimitExceeded {
                field: "suppressed data requests",
                actual: 2,
                limit: 1,
            }
        );
        assert!(finished.is_empty());
        assert!(second_result.is_empty());
        assert!(cleanup.is_empty());
        assert_eq!(adapter.retained_native_id_capacity(), 0);
    }

    #[test]
    fn data_url_suppression_is_scoped_and_related_session_drain_removes_it() {
        // Arrange
        let mut adapter = CdpNetworkEventAdapter::new(
            ObjectId::new(3),
            "session-1".to_owned(),
            Arc::new(AtomicObjectIdSource::new(10).expect("object IDs should validate")),
            limits(2).with_native_scope_bytes(64),
        )
        .expect("adapter should build");
        let ordinary = adapter
            .map_event(&request_event("same", "https://example.test/page"))
            .expect("Page request should start");
        let data = CdpEvent::new(
            "Network.requestWillBeSent".to_owned(),
            json!({
                "requestId": "same",
                "request": {"url": "data:text/html,worker", "method": "GET", "headers": {}}
            }),
            Some("worker-session".to_owned()),
        );

        // Act
        let ignored = adapter
            .map_related_session_event(&data)
            .expect("related data URL should be suppressed");
        let cleanup = adapter.drain_related_session("worker-session");

        // Assert
        assert_eq!(ordinary.events().len(), 1);
        assert!(ignored.is_empty());
        assert!(cleanup.is_empty());
        assert!(adapter.suppressed_data_requests.is_empty());
        assert_eq!(adapter.active_request_count(), 1);
    }

    #[test]
    fn data_url_failure_tail_is_ignored_without_a_public_request() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .map_event(&request_event("data-fail", "data:text/html,hello"))
            .expect("data URL should be suppressed");
        let failure = event(
            "Network.loadingFailed",
            json!({"requestId": "data-fail", "errorText": "net::ERR_ABORTED"}),
        );

        // Act
        let ignored = adapter
            .map_event(&failure)
            .expect("data URL failure should be suppressed");
        let cleanup = adapter.drain_active();

        // Assert
        assert!(ignored.is_empty());
        assert!(cleanup.is_empty());
        assert_eq!(adapter.retained_native_id_capacity(), 0);
    }

    #[cfg(feature = "request-frame")]
    fn document_request_event(request_id: &str, frame_id: &str) -> CdpEvent {
        event(
            "Network.requestWillBeSent",
            json!({
                "requestId": request_id,
                "frameId": frame_id,
                "type": "Document",
                "request": {
                    "url": "https://example.test/document",
                    "method": "GET",
                    "headers": {}
                }
            }),
        )
    }

    #[cfg(feature = "request-frame")]
    #[test]
    fn document_request_before_frame_navigation_has_no_available_frame() {
        // Arrange
        let mut adapter = adapter(1);
        let request = document_request_event("popup-document", "popup-main-frame");

        // Act
        let batch = adapter
            .map_event(&request)
            .expect("popup request should map");

        // Assert
        let CdpNetworkEventEffect::Request(request) = &batch.events()[0] else {
            panic!("request event should be published");
        };
        assert_eq!(
            batch.request_frame_available(request.payload().request),
            Some(false)
        );
        assert_eq!(
            batch
                .request_frame(request.payload().request)
                .and_then(CdpRequestFrame::native_id),
            Some("popup-main-frame")
        );
    }

    #[cfg(feature = "request-frame")]
    #[test]
    fn document_request_after_frame_navigation_has_an_available_frame() {
        // Arrange
        let mut adapter = adapter(1);
        let frame = event(
            "Page.frameNavigated",
            json!({
                "frame": {
                    "id": "page-main-frame",
                    "url": "https://example.test/"
                }
            }),
        );
        let ignored = adapter
            .map_event(&frame)
            .expect("frame navigation should be observed");
        assert!(ignored.is_empty());
        let request = document_request_event("page-document", "page-main-frame");

        // Act
        let batch = adapter
            .map_event(&request)
            .expect("page request should map");

        // Assert
        let CdpNetworkEventEffect::Request(request) = &batch.events()[0] else {
            panic!("request event should be published");
        };
        assert_eq!(
            batch.request_frame_available(request.payload().request),
            Some(true)
        );
        assert_eq!(
            batch
                .request_frame(request.payload().request)
                .and_then(CdpRequestFrame::native_id),
            Some("page-main-frame")
        );
    }

    #[cfg(feature = "request-frame")]
    #[test]
    fn document_request_uses_a_frame_seeded_before_cursor_installation() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .observe_existing_frame("existing-main-frame")
            .expect("the existing Frame fits the bounded native identity");
        let request = document_request_event("document", "existing-main-frame");

        // Act
        let batch = adapter
            .map_event(&request)
            .expect("the first request should map");

        // Assert
        let CdpNetworkEventEffect::Request(request) = &batch.events()[0] else {
            panic!("request event should be published");
        };
        let frame = batch
            .request_frame(request.payload().request)
            .expect("the Request should retain its seeded Frame");
        assert!(frame.available());
        assert_eq!(frame.native_id(), Some("existing-main-frame"));
    }

    #[cfg(feature = "request-frame")]
    #[test]
    fn finished_request_retains_its_frame_after_frame_detach() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .map_event(&event(
                "Page.frameAttached",
                json!({"frameId": "child-frame", "parentFrameId": "main-frame"}),
            ))
            .expect("child Frame should be observed");
        let started = adapter
            .map_event(&document_request_event("child-document", "child-frame"))
            .expect("child Request should start");
        let CdpNetworkEventEffect::Request(request) = &started.events()[0] else {
            panic!("request event should be published");
        };
        let request_id = request.payload().request;
        adapter
            .map_event(&event(
                "Page.frameDetached",
                json!({"frameId": "child-frame", "reason": "remove"}),
            ))
            .expect("child Frame should be removed from observed topology");

        // Act
        let finished = adapter
            .map_event(&event(
                "Network.loadingFinished",
                json!({"requestId": "child-document", "encodedDataLength": 0}),
            ))
            .expect("terminal event should map after Frame detach");

        // Assert
        assert_eq!(started.request_frame_available(request_id), Some(true));
        assert!(!adapter.observed_frames.contains("child-frame"));
        assert!(matches!(
            finished.events(),
            [CdpNetworkEventEffect::RequestFinished(_)]
        ));
        let frame = finished
            .request_frame(request_id)
            .expect("terminal event should retain its Request Frame");
        assert!(frame.available());
        assert_eq!(frame.native_id(), Some("child-frame"));
    }

    #[cfg(feature = "request-frame")]
    #[test]
    fn failed_request_retains_its_frame_after_frame_detach() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .map_event(&event(
                "Page.frameAttached",
                json!({"frameId": "child-frame", "parentFrameId": "main-frame"}),
            ))
            .expect("child Frame should be observed");
        let started = adapter
            .map_event(&document_request_event("child-document", "child-frame"))
            .expect("child Request should start");
        let CdpNetworkEventEffect::Request(request) = &started.events()[0] else {
            panic!("request event should be published");
        };
        let request_id = request.payload().request;
        adapter
            .map_event(&event(
                "Page.frameDetached",
                json!({"frameId": "child-frame", "reason": "remove"}),
            ))
            .expect("child Frame should be removed from observed topology");

        // Act
        let failed = adapter
            .map_event(&event(
                "Network.loadingFailed",
                json!({"requestId": "child-document", "errorText": "net::ERR_ABORTED"}),
            ))
            .expect("terminal event should map after Frame detach");

        // Assert
        assert_eq!(started.request_frame_available(request_id), Some(true));
        assert!(!adapter.observed_frames.contains("child-frame"));
        assert!(matches!(
            failed.events(),
            [CdpNetworkEventEffect::RequestFailed(_)]
        ));
        let frame = failed
            .request_frame(request_id)
            .expect("terminal event should retain its Request Frame");
        assert!(frame.available());
        assert_eq!(frame.native_id(), Some("child-frame"));
    }

    #[test]
    fn request_post_data_entries_decode_into_the_portable_body() {
        // Arrange
        let mut adapter = adapter(1);
        let request = event(
            "Network.requestWillBeSent",
            json!({
                "requestId": "native-1",
                "type": "Fetch",
                "request": {
                    "url": "https://example.test/get",
                    "method": "POST",
                    "headers": {"content-type": "text/plain"},
                    "hasPostData": true,
                    "postDataEntries": [{"bytes": "MTIzNDU="}]
                }
            }),
        );

        // Act
        let batch = adapter
            .map_event(&request)
            .expect("request body should decode");

        // Assert
        let CdpNetworkEventEffect::Request(event) = &batch.events()[0] else {
            panic!("request event should be published");
        };
        assert_eq!(
            event.payload().post_data.as_deref(),
            Some(b"12345".as_slice())
        );
    }

    #[test]
    fn ordinary_profile_rejects_post_data_above_256_kib() {
        // Arrange
        let mut lifecycle = limits(8);
        lifecycle.post_data_bytes = 256 * 1024;
        let mut adapter = CdpNetworkEventAdapter::new(
            ObjectId::new(3),
            "session-1".to_owned(),
            Arc::new(AtomicObjectIdSource::new(10).expect("object IDs should validate")),
            lifecycle,
        )
        .expect("ordinary adapter should build");
        let request = post_data_event("ordinary-over-cap", &"0".repeat(256 * 1024 + 1));

        // Act
        let result = adapter.map_event(&request);

        // Assert
        assert!(matches!(
            result,
            Err(CdpNetworkEventError::FieldLimitExceeded {
                field: "request.postData",
                actual: 262_145,
                limit: 262_144,
            })
        ));
        assert_eq!(adapter.active_request_count(), 0);
    }

    #[test]
    fn bounded_large_post_data_profile_decodes_two_megabyte_entries() {
        // Arrange
        let mut adapter = bounded_large_post_data_adapter(8, 4);
        let body = vec![b'0'; 2 * 1024 * 1024];
        let request = event(
            "Network.requestWillBeSent",
            json!({
                "requestId": "large-entries",
                "type": "Fetch",
                "request": {
                    "url": "https://example.test/submit",
                    "method": "POST",
                    "headers": {},
                    "hasPostData": true,
                    "postDataEntries": [{
                        "bytes": base64::engine::general_purpose::STANDARD.encode(&body)
                    }]
                }
            }),
        );

        // Act
        let batch = adapter.map_event(&request).expect("2 MiB POST should map");

        // Assert
        let CdpNetworkEventEffect::Request(request) = &batch.events()[0] else {
            panic!("large POST should publish a Request");
        };
        assert_eq!(
            request.payload().post_data.as_deref(),
            Some(body.as_slice())
        );
        assert_eq!(adapter.large_post_data_requests.len(), 1);
    }

    #[test]
    fn large_post_data_slots_do_not_reduce_ordinary_request_capacity() {
        // Arrange
        let mut adapter = bounded_large_post_data_adapter(8, 4);
        let large_body = "0".repeat(256 * 1024 + 1);
        for index in 0..4 {
            adapter
                .map_event(&post_data_event(&format!("large-{index}"), &large_body))
                .expect("first four large requests should map");
        }
        adapter
            .map_event(&post_data_event("ordinary", &"0".repeat(256 * 1024)))
            .expect("ordinary request should still map");

        // Act
        let result = adapter.map_event(&post_data_event("large-over-cap", &large_body));

        // Assert
        assert!(matches!(
            result,
            Err(CdpNetworkEventError::FieldLimitExceeded {
                field: "active large post-data requests",
                actual: 5,
                limit: 4,
            })
        ));
        assert_eq!(adapter.active_request_count(), 5);
        assert_eq!(adapter.large_post_data_requests.len(), 4);
    }

    #[test]
    fn failed_large_request_releases_its_slot_for_the_next_request() {
        // Arrange
        let mut adapter = bounded_large_post_data_adapter(8, 1);
        let large_body = "0".repeat(256 * 1024 + 1);
        adapter
            .map_event(&post_data_event("failed-large", &large_body))
            .expect("large request should map");
        adapter
            .map_event(&event(
                "Network.loadingFailed",
                json!({"requestId": "failed-large", "errorText": "net::ERR_FAILED"}),
            ))
            .expect("terminal failure should map");

        // Act
        let result = adapter.map_event(&post_data_event("next-large", &large_body));

        // Assert
        assert!(result.is_ok());
        assert_eq!(adapter.large_post_data_requests.len(), 1);
    }

    #[test]
    fn finished_large_request_releases_its_slot_for_the_next_request() {
        // Arrange
        let mut adapter = bounded_large_post_data_adapter(8, 1);
        let large_body = "0".repeat(256 * 1024 + 1);
        adapter
            .map_event(&post_data_event("finished-large", &large_body))
            .expect("large request should map");
        adapter
            .map_event(&event(
                "Network.loadingFinished",
                json!({"requestId": "finished-large", "encodedDataLength": 0}),
            ))
            .expect("terminal finish should map");

        // Act
        let result = adapter.map_event(&post_data_event("next-large", &large_body));

        // Assert
        assert!(result.is_ok());
        assert_eq!(adapter.large_post_data_requests.len(), 1);
    }

    #[test]
    fn redirect_to_ordinary_body_releases_the_large_request_slot() {
        // Arrange
        let mut adapter = bounded_large_post_data_adapter(8, 1);
        let large_body = "0".repeat(256 * 1024 + 1);
        adapter
            .map_event(&post_data_event("redirected-large", &large_body))
            .expect("large request should map");
        adapter
            .map_event(&event(
                "Network.requestWillBeSent",
                json!({
                    "requestId": "redirected-large",
                    "type": "Document",
                    "redirectResponse": {
                        "url": "https://example.test/submit",
                        "status": 302,
                        "statusText": "Found",
                        "headers": {"location": "https://example.test/next"}
                    },
                    "request": {
                        "url": "https://example.test/next",
                        "method": "GET",
                        "headers": {}
                    }
                }),
            ))
            .expect("redirect should map");

        // Act
        let result = adapter.map_event(&post_data_event("new-large", &large_body));

        // Assert
        assert!(result.is_ok());
        assert_eq!(adapter.large_post_data_requests.len(), 1);
    }

    #[test]
    fn draining_related_session_releases_only_its_large_post_data_slot() {
        // Arrange
        let mut adapter = bounded_large_post_data_adapter(8, 2);
        let large_body = "0".repeat(256 * 1024 + 1);
        adapter
            .map_event(&post_data_event("page-large", &large_body))
            .expect("Page large request should map");
        let related = CdpEvent::new(
            "Network.requestWillBeSent".to_owned(),
            post_data_event("worker-large", &large_body).params.clone(),
            Some("worker-session".to_owned()),
        );
        adapter
            .map_related_session_event(&related)
            .expect("Worker large request should map");

        // Act
        let cleanup = adapter.drain_related_session("worker-session");

        // Assert
        assert_eq!(cleanup.len(), 1);
        assert_eq!(adapter.active_request_count(), 1);
        assert_eq!(adapter.large_post_data_requests.len(), 1);
        assert!(
            adapter
                .large_post_data_requests
                .contains(&(String::new(), "page-large".to_owned()))
        );
    }

    #[test]
    fn bounded_large_post_data_reservation_adds_only_four_body_overages() {
        // Arrange
        let mut ordinary_limits = limits(64);
        ordinary_limits.post_data_bytes = 256 * 1024;
        let mut large_limits = ordinary_limits;
        large_limits.post_data_bytes = 3 * 1024 * 1024;
        let scope = ResourceScope::extension(95, 1);
        let ordinary = CdpNetworkEventProductionProfile::new(ordinary_limits, scope)
            .expect("ordinary profile should validate");

        // Act
        let large = CdpNetworkEventProductionProfile::with_bounded_large_post_data(
            large_limits,
            scope,
            256 * 1024,
            4,
        )
        .expect("large profile should validate");

        // Assert
        assert_eq!(large.ordinary_post_data_bytes(), 256 * 1024);
        assert_eq!(large.large_post_data_requests(), 4);
        assert_eq!(
            large.maximum_retained_bytes() - ordinary.maximum_retained_bytes(),
            4 * (3 * 1024 * 1024 - 256 * 1024
                + super::LARGE_POST_DATA_REQUEST_BOOKKEEPING_BYTES
                + large_limits.native_scope_bytes
                + large_limits.native_id_bytes)
        );
    }

    #[test]
    fn large_post_data_profile_rejects_more_slots_than_active_requests() {
        // Arrange
        let mut lifecycle = limits(2);
        lifecycle.post_data_bytes = 3 * 1024 * 1024;

        // Act
        let result = CdpNetworkEventProductionProfile::with_bounded_large_post_data(
            lifecycle,
            ResourceScope::extension(95, 1),
            256 * 1024,
            3,
        );

        // Assert
        assert_eq!(result, Err(CdpNetworkEventError::InvalidPostDataBudget));
    }

    #[test]
    fn request_extra_info_after_request_replaces_terminal_snapshot_headers() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .map_event(&request_event("native-1", "https://example.test/data"))
            .expect("request should start");
        let extra_info = event(
            "Network.requestWillBeSentExtraInfo",
            json!({
                "requestId": "native-1",
                "headers": {"cookie": "session=raw", "accept": "*/*"}
            }),
        );

        // Act
        let extra_batch = adapter
            .map_event(&extra_info)
            .expect("request extra info should map");
        adapter
            .map_event(&event(
                "Network.responseReceived",
                json!({
                    "requestId": "native-1",
                    "response": {
                        "url": "https://example.test/data",
                        "status": 200,
                        "statusText": "OK",
                        "headers": {}
                    }
                }),
            ))
            .expect("response should map");
        let finished = adapter
            .map_event(&event(
                "Network.loadingFinished",
                json!({"requestId": "native-1", "encodedDataLength": 42}),
            ))
            .expect("request should finish");

        // Assert
        assert!(extra_batch.is_empty());
        let CdpNetworkEventEffect::RequestFinished(finished) = &finished.events()[0] else {
            panic!("terminal event should be request-finished");
        };
        assert_eq!(finished.payload().request.headers[0].name, "accept");
        assert_eq!(finished.payload().request.headers[1].name, "cookie");
        assert_eq!(finished.payload().request.headers[1].value, "session=raw");
    }

    #[test]
    fn request_extra_info_before_request_is_correlated_to_initial_snapshot() {
        // Arrange
        let mut adapter = adapter(1);
        let extra_info = event(
            "Network.requestWillBeSentExtraInfo",
            json!({
                "requestId": "native-1",
                "headers": {"cookie": "session=early"}
            }),
        );
        adapter
            .map_event(&extra_info)
            .expect("early request extra info should be retained");

        // Act
        let request = adapter
            .map_event(&request_event("native-1", "https://example.test/data"))
            .expect("request should consume early extra info");

        // Assert
        let CdpNetworkEventEffect::Request(request) = &request.events()[0] else {
            panic!("event should be a Request");
        };
        assert_eq!(
            request.payload().headers,
            vec![pwright_network_model::HttpHeader::new(
                "cookie",
                "session=early"
            )]
        );
        assert_eq!(adapter.active_request_count(), 1);
    }

    #[test]
    fn request_response_finished_maps_to_ordered_lifecycle_and_typed_events() {
        // Arrange
        let mut adapter = adapter(2);
        let request = request_event("native-1", "https://example.test/data");
        let response = event(
            "Network.responseReceived",
            json!({
                "requestId": "native-1",
                "response": {
                    "url": "https://example.test/data",
                    "status": 500,
                    "statusText": "Server Error",
                    "headers": {"content-type": "application/json"},
                    "fromServiceWorker": false
                }
            }),
        );
        let finished = event(
            "Network.loadingFinished",
            json!({"requestId": "native-1", "encodedDataLength": 42}),
        );

        // Act
        let request_batch = adapter.map_event(&request).expect("request should map");
        let response_batch = adapter.map_event(&response).expect("response should map");
        let finished_batch = adapter.map_event(&finished).expect("finish should map");

        // Assert
        assert_eq!(
            request_batch.lifecycle(),
            &[ControlMessage::Create {
                object_id: ObjectId::new(10),
                parent: Some(ObjectId::new(3)),
                kind: pwright_browser_model::REQUEST_OBJECT_KIND.tag,
            }]
        );
        assert_eq!(request_batch.session_id(), Some("session-1"));
        assert_eq!(request_batch.native_request_id(), Some("native-1"));
        let CdpNetworkEventEffect::Request(request) = &request_batch.events()[0] else {
            panic!("request event should be typed");
        };
        assert_eq!(request.object_id(), ObjectId::new(3));
        assert_eq!(request.payload().request, ObjectId::new(10));
        assert_eq!(request.payload().headers[0].name, "accept");
        assert_eq!(
            <NetworkRequest as pwright_core::EventSpec>::EVENT,
            pwright_cap_network_events::NETWORK_REQUEST_EVENT_TAG
        );
        let CdpNetworkEventEffect::Response(response) = &response_batch.events()[0] else {
            panic!("response event should be typed");
        };
        assert_eq!(response.payload().status, 500);
        assert_eq!(response.payload().request, ObjectId::new(10));
        assert_eq!(response.payload().response, ObjectId::new(11));
        assert_eq!(
            <NetworkResponse as pwright_core::EventSpec>::EVENT,
            pwright_cap_network_events::NETWORK_RESPONSE_EVENT_TAG
        );
        assert!(matches!(
            finished_batch.events()[0],
            CdpNetworkEventEffect::RequestFinished(_)
        ));
        assert!(finished_batch.lifecycle().is_empty());
        assert_eq!(
            finished_batch.retirement(),
            &[ControlMessage::Dispose {
                object_id: ObjectId::new(10),
            }]
        );
        assert_eq!(adapter.active_request_count(), 0);
        assert_eq!(adapter.retained_native_id_capacity(), 0);
    }

    #[cfg(feature = "request-timing")]
    #[test]
    fn positive_submicrosecond_timing_remains_available() {
        // Arrange
        let value = json!(0.0001);

        // Act
        let micros = number_micros(&value, "Network response timing", "dnsStart", 1_000.0)
            .expect("positive timing should normalize");

        // Assert
        assert_eq!(micros, 1);
    }

    #[cfg(feature = "request-timing")]
    #[test]
    fn exact_zero_connection_timing_is_unavailable() {
        // Arrange
        let value = json!({"dnsStart": 0.0});
        let object = value.as_object().expect("timing should be an object");

        // Act
        let micros =
            required_connection_timing_micros(object, "dnsStart", "Network response timing")
                .expect("zero connection timing should normalize");

        // Assert
        assert_eq!(
            micros,
            pwright_network_model::RequestTiming::UNAVAILABLE_MICROS
        );
    }

    #[cfg(feature = "request-timing")]
    #[test]
    fn cached_signal_replaces_cdp_connection_timing_with_unavailable_values() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .map_event(&event(
                "Network.requestWillBeSent",
                json!({
                    "requestId": "cached",
                    "timestamp": 10.0,
                    "wallTime": 1_700_000_000.0,
                    "type": "Document",
                    "request": {
                        "url": "https://example.test/cached",
                        "method": "GET",
                        "headers": {}
                    }
                }),
            ))
            .expect("request should start");
        let cached = adapter
            .map_event(&event(
                "Network.requestServedFromCache",
                json!({"requestId": "cached"}),
            ))
            .expect("cached signal should map");

        // Act
        let response = adapter
            .map_event(&event(
                "Network.responseReceived",
                json!({
                    "requestId": "cached",
                    "response": {
                        "url": "https://example.test/cached",
                        "status": 200,
                        "statusText": "OK",
                        "headers": {},
                        "timing": {
                            "requestTime": 10.0,
                            "dnsStart": 0.0,
                            "dnsEnd": 0.0,
                            "connectStart": 0.0,
                            "sslStart": -1.0,
                            "connectEnd": 0.0,
                            "sendStart": 0.1,
                            "receiveHeadersEnd": 0.2
                        }
                    }
                }),
            ))
            .expect("response should map");

        // Assert
        assert!(cached.is_empty());
        let CdpNetworkEventEffect::Response(response) = &response.events()[0] else {
            panic!("response event should be published");
        };
        let timing = response
            .payload()
            .timing
            .as_deref()
            .expect("cached response should expose fallback timing");
        assert_eq!(timing.start_time_micros, 1_700_000_000_000_000);
        assert_eq!(timing.domain_lookup_start_micros, -1_000);
        assert_eq!(timing.request_start_micros, -1_000);
    }

    #[cfg(feature = "request-timing")]
    #[test]
    fn cdp_timing_is_normalized_into_response_and_terminal_snapshots() {
        // Arrange
        let mut adapter = adapter(1);
        let request = event(
            "Network.requestWillBeSent",
            json!({
                "requestId": "timed",
                "timestamp": 10.0,
                "wallTime": 1_700_000_000.0,
                "type": "Document",
                "request": {
                    "url": "https://example.test/timed",
                    "method": "GET",
                    "headers": {}
                }
            }),
        );
        let response = event(
            "Network.responseReceived",
            json!({
                "requestId": "timed",
                "response": {
                    "url": "https://example.test/timed",
                    "status": 200,
                    "statusText": "OK",
                    "headers": {},
                    "timing": {
                        "requestTime": 10.0005,
                        "dnsStart": 0.1,
                        "dnsEnd": 0.2,
                        "connectStart": 0.2,
                        "sslStart": 0.3,
                        "connectEnd": 0.5,
                        "sendStart": 0.6,
                        "receiveHeadersEnd": 2.5
                    }
                }
            }),
        );
        let finished = event(
            "Network.loadingFinished",
            json!({
                "requestId": "timed",
                "encodedDataLength": 42,
                "timestamp": 10.005
            }),
        );
        adapter
            .map_event(&request)
            .expect("timed request should start");

        // Act
        let response_batch = adapter
            .map_event(&response)
            .expect("timed response should map");
        let finished_batch = adapter
            .map_event(&finished)
            .expect("timed request should finish");

        // Assert
        let CdpNetworkEventEffect::Response(response) = &response_batch.events()[0] else {
            panic!("response event should be published");
        };
        let response_timing = response
            .payload()
            .timing
            .as_deref()
            .expect("response timing should be retained");
        assert_eq!(response_timing.start_time_micros, 1_700_000_000_000_500);
        assert_eq!(response_timing.domain_lookup_start_micros, 100);
        assert_eq!(response_timing.request_start_micros, 600);
        assert_eq!(response_timing.response_start_micros, 2_500);
        assert_eq!(response_timing.response_end_micros, -1_000);
        let CdpNetworkEventEffect::RequestFinished(finished) = &finished_batch.events()[0] else {
            panic!("terminal event should be request-finished");
        };
        assert_eq!(
            finished
                .payload()
                .request
                .timing
                .as_deref()
                .expect("terminal timing should be retained")
                .response_end_micros,
            5_000
        );
        assert_eq!(
            finished
                .payload()
                .response
                .as_ref()
                .expect("response snapshot should be retained")
                .timing,
            finished.payload().request.timing
        );
    }

    #[cfg(feature = "request-timing")]
    #[test]
    fn incomplete_request_timing_origin_rejects_before_lifecycle_state_is_created() {
        // Arrange
        let mut adapter = adapter(1);
        let request = event(
            "Network.requestWillBeSent",
            json!({
                "requestId": "timed",
                "timestamp": 10.0,
                "type": "Document",
                "request": {
                    "url": "https://example.test/timed",
                    "method": "GET",
                    "headers": {}
                }
            }),
        );

        // Act
        let error = match adapter.map_event(&request) {
            Ok(_) => panic!("timestamp without wallTime should be rejected"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(
            error,
            CdpNetworkEventError::InvalidEvent {
                method: "Network.requestWillBeSent",
                field: "wallTime",
            }
        );
        assert_eq!(adapter.active_request_count(), 0);
        assert_eq!(adapter.retained_native_id_capacity(), 0);
    }

    #[test]
    fn redirect_finishes_old_objects_before_creating_the_next_request() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .map_event(&request_event("native-1", "https://example.test/old"))
            .expect("first hop should start");
        let redirect = event(
            "Network.requestWillBeSent",
            json!({
                "requestId": "native-1",
                "type": "Document",
                "redirectResponse": {
                    "url": "https://example.test/old",
                    "status": 302,
                    "statusText": "Found",
                    "headers": {"location": "https://example.test/new"}
                },
                "request": {
                    "url": "https://example.test/new",
                    "method": "GET",
                    "headers": {}
                }
            }),
        );

        // Act
        let batch = adapter.map_event(&redirect).expect("redirect should map");

        // Assert
        assert_eq!(batch.events().len(), 3);
        assert_eq!(batch.lifecycle().len(), 2);
        assert_eq!(
            batch.lifecycle()[0],
            ControlMessage::Create {
                object_id: ObjectId::new(11),
                parent: Some(ObjectId::new(10)),
                kind: pwright_browser_model::RESPONSE_OBJECT_KIND.tag,
            }
        );
        assert!(matches!(
            batch.events()[0],
            CdpNetworkEventEffect::Response(_)
        ));
        assert!(matches!(
            batch.events()[1],
            CdpNetworkEventEffect::RequestFinished(_)
        ));
        assert_eq!(
            batch.retirement(),
            &[ControlMessage::Dispose {
                object_id: ObjectId::new(10),
            }]
        );
        let CdpNetworkEventEffect::Request(next) = &batch.events()[2] else {
            panic!("new hop should be last");
        };
        assert_eq!(next.payload().request, ObjectId::new(12));
        assert_eq!(next.payload().redirected_from, Some(ObjectId::new(10)));
        assert_eq!(adapter.active_request_count(), 1);
    }

    #[cfg(feature = "response-security-details")]
    #[test]
    fn https_response_security_is_typed_bounded_and_preserved_at_terminal() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .map_event(&request_event("tls", "https://example.test/"))
            .expect("request should start");
        let response = response_event("tls", "https://example.test/", Some(security_details()));
        let finished = event(
            "Network.loadingFinished",
            json!({"requestId": "tls", "encodedDataLength": 12}),
        );

        // Act
        let response_batch = adapter.map_event(&response).expect("response should map");
        let terminal_batch = adapter.map_event(&finished).expect("terminal should map");

        // Assert
        let CdpNetworkEventEffect::Response(response) = &response_batch.events()[0] else {
            panic!("response event should be typed");
        };
        let response_id = response.payload().response;
        let details = response_batch
            .response_security(response_id)
            .expect("TLS details should exist");
        assert_eq!(details.protocol(), "TLS 1.3");
        assert_eq!(details.subject_name(), "example.test");
        assert_eq!(details.issuer(), "Test Authority");
        assert_eq!(details.valid_from_seconds(), 1_700_000_000.25);
        assert_eq!(details.valid_to_seconds(), 1_800_000_000.5);
        assert!(details.retained_bytes() < 1024);
        assert_eq!(terminal_batch.response_security(response_id), Some(details));
        assert!(adapter.response_security.is_empty());
    }

    #[cfg(feature = "response-security-details")]
    #[test]
    fn http_response_has_no_security_details_in_response_or_terminal() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .map_event(&request_event("http", "http://example.test/"))
            .expect("request should start");
        let response = response_event("http", "http://example.test/", None);
        let finished = event(
            "Network.loadingFinished",
            json!({"requestId": "http", "encodedDataLength": 0}),
        );

        // Act
        let response_batch = adapter.map_event(&response).expect("response should map");
        let terminal_batch = adapter.map_event(&finished).expect("terminal should map");

        // Assert
        assert!(
            response_batch
                .response_security(ObjectId::new(11))
                .is_none()
        );
        assert!(
            terminal_batch
                .response_security(ObjectId::new(11))
                .is_none()
        );
        assert!(adapter.response_security.is_empty());
    }

    #[cfg(feature = "response-security-details")]
    #[test]
    fn redirect_security_stays_on_old_response_and_is_released() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .map_event(&request_event("redirect", "https://example.test/old"))
            .expect("request should start");
        let redirect = event(
            "Network.requestWillBeSent",
            json!({
                "requestId": "redirect",
                "type": "Document",
                "redirectResponse": {
                    "url": "https://example.test/old", "status": 302,
                    "statusText": "Found", "headers": {},
                    "securityDetails": security_details()
                },
                "request": {"url": "http://example.test/new", "method": "GET", "headers": {}}
            }),
        );

        // Act
        let batch = adapter.map_event(&redirect).expect("redirect should map");

        // Assert
        assert!(matches!(
            batch.events()[0],
            CdpNetworkEventEffect::Response(_)
        ));
        assert!(matches!(
            batch.events()[1],
            CdpNetworkEventEffect::RequestFinished(_)
        ));
        assert!(matches!(
            batch.events()[2],
            CdpNetworkEventEffect::Request(_)
        ));
        let old_response_id = match &batch.events()[0] {
            CdpNetworkEventEffect::Response(value) => value.payload().response,
            _ => unreachable!(),
        };
        assert_eq!(
            batch
                .response_security(old_response_id)
                .map(|value| value.issuer()),
            Some("Test Authority")
        );
        assert!(adapter.response_security.is_empty());
    }

    #[cfg(feature = "response-security-details")]
    #[test]
    fn malformed_security_field_rejects_response_without_publishing_it() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .map_event(&request_event("bad", "https://example.test/"))
            .expect("request should start");
        let mut details = security_details();
        details["validTo"] = json!("not a number");
        let response = response_event("bad", "https://example.test/", Some(details));

        // Act
        let result = adapter.map_event(&response);

        // Assert
        assert!(matches!(
            result,
            Err(CdpNetworkEventError::InvalidEvent {
                field: "validTo",
                ..
            })
        ));
        assert!(adapter.response_security.is_empty());
        assert_eq!(adapter.active_request_count(), 1);
    }

    #[cfg(feature = "response-security-details")]
    #[test]
    fn oversized_security_field_rejects_response_without_retaining_it() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .map_event(&request_event("large", "https://example.test/"))
            .expect("request should start");
        let mut details = security_details();
        details["issuer"] = json!("i".repeat(super::MAX_SECURITY_FIELD_BYTES + 1));
        let response = response_event("large", "https://example.test/", Some(details));

        // Act
        let result = adapter.map_event(&response);

        // Assert
        assert!(matches!(
            result,
            Err(CdpNetworkEventError::FieldLimitExceeded {
                field: "issuer",
                ..
            })
        ));
        assert!(adapter.response_security.is_empty());
        assert_eq!(adapter.active_request_count(), 1);
    }

    #[test]
    fn failure_and_wrong_session_have_distinct_bounded_behavior() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .map_event(&request_event("native-1", "https://example.test/data"))
            .expect("request should start");
        let wrong_session = CdpEvent::new(
            "Network.loadingFailed".to_owned(),
            json!({"requestId": "native-1", "errorText": "ignored"}),
            Some("session-2".to_owned()),
        );
        let failed = event(
            "Network.loadingFailed",
            json!({
                "requestId": "native-1",
                "errorText": "net::ERR_ABORTED",
                "canceled": true
            }),
        );

        // Act
        let ignored = adapter
            .map_event(&wrong_session)
            .expect("other session should be ignored");
        let failed = adapter.map_event(&failed).expect("failure should map");

        // Assert
        assert!(ignored.is_empty());
        let CdpNetworkEventEffect::RequestFailed(failed) = &failed.events()[0] else {
            panic!("failure should be typed");
        };
        assert_eq!(failed.payload().failure_text, "net::ERR_ABORTED");
        assert!(failed.payload().canceled);
        assert_eq!(
            <NetworkRequestFailed as pwright_core::EventSpec>::EVENT,
            pwright_cap_network_events::NETWORK_REQUEST_FAILED_EVENT_TAG
        );
        assert_eq!(adapter.active_request_count(), 0);
        #[cfg(feature = "request-frame")]
        assert!(adapter.request_frames.is_empty());
    }

    #[test]
    fn attach_boundary_ignores_response_and_terminal_tails_without_request_starts() {
        // Arrange
        let mut adapter = adapter(1);
        let response = event(
            "Network.responseReceived",
            json!({
                "requestId": "pre-attach",
                "response": {
                    "url": "https://example.test/already-running",
                    "status": 200,
                    "statusText": "OK",
                    "headers": {}
                }
            }),
        );
        let finished = event(
            "Network.loadingFinished",
            json!({"requestId": "pre-attach", "encodedDataLength": 42}),
        );
        let failed = event(
            "Network.loadingFailed",
            json!({"requestId": "another-pre-attach", "errorText": "net::ERR_ABORTED"}),
        );

        // Act
        let response = adapter
            .map_event(&response)
            .expect("unknown response tail should be ignored");
        let finished = adapter
            .map_event(&finished)
            .expect("unknown finish tail should be ignored");
        let failed = adapter
            .map_event(&failed)
            .expect("unknown failure tail should be ignored");

        // Assert
        assert!(response.is_empty());
        assert!(finished.is_empty());
        assert!(failed.is_empty());
        assert_eq!(adapter.active_request_count(), 0);
        assert_eq!(adapter.retained_native_id_capacity(), 0);
    }

    #[test]
    fn oversized_url_rejects_before_correlation_state_is_created() {
        // Arrange
        let mut adapter = adapter(1);
        let oversized = request_event("native-1", &"x".repeat(257));

        // Act
        let error = match adapter.map_event(&oversized) {
            Ok(_) => panic!("oversized URL should fail"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(
            error,
            CdpNetworkEventError::FieldLimitExceeded {
                field: "url",
                actual: 257,
                limit: 256,
            }
        );
        assert_eq!(adapter.active_request_count(), 0);
        #[cfg(feature = "request-frame")]
        assert!(adapter.request_frames.is_empty());
    }

    #[test]
    fn active_teardown_disposes_only_request_roots() {
        // Arrange
        let mut adapter = adapter(1);
        adapter
            .map_event(&request_event("native-1", "https://example.test/data"))
            .expect("request should start");
        adapter
            .map_event(&event(
                "Network.responseReceived",
                json!({
                    "requestId": "native-1",
                    "response": {
                        "url": "https://example.test/data",
                        "status": 200,
                        "statusText": "OK",
                        "headers": {}
                    }
                }),
            ))
            .expect("response should map");

        // Act
        let disposal = adapter.drain_active();

        // Assert
        assert_eq!(
            disposal,
            vec![ControlMessage::Dispose {
                object_id: ObjectId::new(10),
            }]
        );
        assert_eq!(adapter.active_request_count(), 0);
        #[cfg(feature = "request-frame")]
        assert!(adapter.request_frames.is_empty());
    }

    #[test]
    fn production_adapter_reserves_and_releases_its_complete_active_state_bound() {
        // Arrange
        let profile =
            CdpNetworkEventProductionProfile::new(limits(2), ResourceScope::extension(95, 1))
                .expect("profile should validate");
        let requirements = profile.requirements();
        let class = requirements.class(ResourceClass::BackendStateBytes);
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::BackendStateBytes,
                class.global,
                class.per_scope,
            ),
            requirements,
        )
        .expect("production resources should validate");

        // Act
        let adapter = CdpNetworkEventAdapter::new_with_production_resources(
            ObjectId::new(3),
            "session-1".to_owned(),
            Arc::new(AtomicObjectIdSource::new(10).expect("object IDs should validate")),
            profile,
            &resources,
        )
        .expect("production adapter should reserve its state");

        // Assert
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should be available")
                .class(ResourceClass::BackendStateBytes)
                .current,
            u64::try_from(profile.maximum_retained_bytes()).unwrap()
        );
        drop(adapter);
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should be available")
                .class(ResourceClass::BackendStateBytes)
                .current,
            0
        );
    }

    #[test]
    fn related_session_preserves_native_metadata_and_drains_only_its_requests() {
        // Arrange
        let scoped_limits = limits(2).with_native_scope_bytes(64);
        let mut adapter = CdpNetworkEventAdapter::new(
            ObjectId::new(3),
            "session-1".to_owned(),
            Arc::new(AtomicObjectIdSource::new(10).expect("object IDs should validate")),
            scoped_limits,
        )
        .expect("adapter should build");
        let related = CdpEvent::new(
            "Network.requestWillBeSent".to_owned(),
            json!({
                "requestId": "same-native-id",
                "type": "Fetch",
                "request": {
                    "url": "https://example.test/worker",
                    "method": "GET",
                    "headers": {}
                }
            }),
            Some("worker-session".to_owned()),
        );
        adapter
            .map_event(&request_event(
                "same-native-id",
                "https://example.test/page",
            ))
            .expect("Page request should start");

        // Act
        let batch = adapter
            .map_related_session_event(&related)
            .expect("Worker request should map");
        let cleanup = adapter.drain_related_session("worker-session");

        // Assert
        assert_eq!(batch.page(), Some(ObjectId::new(3)));
        assert_eq!(batch.session_id(), Some("worker-session"));
        assert_eq!(batch.native_request_id(), Some("same-native-id"));
        assert_eq!(
            cleanup,
            vec![ControlMessage::Dispose {
                object_id: ObjectId::new(11),
            }]
        );
        assert_eq!(adapter.active_request_count(), 1);
    }
}
