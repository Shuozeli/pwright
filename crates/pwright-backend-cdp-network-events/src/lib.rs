//! Isolated Chromium CDP HTTP lifecycle event normalizer for Pwright.
//!
//! The leaf maps one caller-driven Page/session event lane into bounded
//! portable Request/Response lifecycle batches. It owns no receiver, queue,
//! response body, runtime, task, browser session, or browser process.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

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

pub enum CdpNetworkEventEffect {
    Request(Event<NetworkRequest>),
    Response(Event<NetworkResponse>),
    RequestFinished(Event<NetworkRequestFinished>),
    RequestFailed(Event<NetworkRequestFailed>),
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
    lifecycle: Vec<ControlMessage>,
    events: Vec<CdpNetworkEventEffect>,
    retirement: Vec<ControlMessage>,
}

const ACTIVE_REQUEST_BOOKKEEPING_BYTES: usize = std::mem::size_of::<String>()
    + std::mem::size_of::<RequestInitializer>()
    + std::mem::size_of::<Option<ResponseInitializer>>()
    + (4 * std::mem::size_of::<usize>());
const ADAPTER_BOOKKEEPING_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpNetworkEventProductionProfile {
    limits: NetworkLifecycleLimits,
    resource_scope: ResourceScope,
    maximum_retained_bytes: usize,
    requirements: ResourceRequirements,
}

impl CdpNetworkEventProductionProfile {
    pub fn new(
        limits: NetworkLifecycleLimits,
        resource_scope: ResourceScope,
    ) -> Result<Self, CdpNetworkEventError> {
        limits.validate().map_err(CdpNetworkEventError::Lifecycle)?;
        let per_request = maximum_active_request_bytes(limits)?;
        let active_bytes = per_request.checked_mul(limits.active_requests).ok_or(
            CdpNetworkEventError::SizeOverflow {
                field: "active request state bytes",
            },
        )?;
        let maximum_retained_bytes = ADAPTER_BOOKKEEPING_BYTES
            .checked_add(limits.native_id_bytes)
            .and_then(|base| base.checked_add(active_bytes))
            .ok_or(CdpNetworkEventError::SizeOverflow {
                field: "network adapter state bytes",
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
            resource_scope,
            maximum_retained_bytes,
            requirements,
        })
    }

    pub const fn limits(self) -> NetworkLifecycleLimits {
        self.limits
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
) -> Result<usize, CdpNetworkEventError> {
    let header_entries = limits
        .header_count
        .checked_mul(std::mem::size_of::<HttpHeader>())
        .and_then(|value| value.checked_mul(2));
    [
        Some(ACTIVE_REQUEST_BOOKKEEPING_BYTES),
        Some(limits.native_id_bytes),
        limits.url_bytes.checked_mul(2),
        Some(limits.method_bytes),
        Some(limits.resource_type_bytes),
        limits.header_bytes.checked_mul(2),
        header_entries,
        Some(limits.post_data_bytes),
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

impl CdpNetworkEventBatch {
    pub fn lifecycle(&self) -> &[ControlMessage] {
        &self.lifecycle
    }

    pub fn events(&self) -> &[CdpNetworkEventEffect] {
        &self.events
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
        self.events.first().map(CdpNetworkEventEffect::source)
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

/// One Page/session-scoped, caller-driven CDP HTTP lifecycle normalizer.
///
/// It owns only bounded native request correlation. It does not own a CDP
/// receiver, event queue, runtime, task, response body, or browser session.
pub struct CdpNetworkEventAdapter {
    page: ObjectId,
    session_id: String,
    object_ids: Arc<dyn ObjectIdSource>,
    tracker: NetworkLifecycleTracker,
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
        adapter._resource = Some(resource);
        Ok(adapter)
    }

    pub const fn page(&self) -> ObjectId {
        self.page
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn active_request_count(&self) -> usize {
        self.tracker.active_request_count()
    }

    pub fn retained_native_id_capacity(&self) -> usize {
        self.tracker.retained_native_id_capacity()
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
        let effects = match event.method.as_str() {
            "Network.requestWillBeSent" => self.request_will_be_sent(event)?,
            "Network.responseReceived" => vec![self.response_received(event)?],
            "Network.loadingFinished" => vec![self.loading_finished(event)?],
            "Network.loadingFailed" => vec![self.loading_failed(event)?],
            _ => return Ok(CdpNetworkEventBatch::default()),
        };
        Ok(self.portable_batch(effects))
    }

    /// Produces parent-owned cleanup for requests still in flight when this
    /// Page/session lane is removed. Finished objects are owned by the Page
    /// runtime tree and are retired by its separate lifecycle/lease policy.
    pub fn drain_active(&mut self) -> Vec<ControlMessage> {
        let cleanup = self.active_request_cleanup();
        self.tracker.drain_active();
        cleanup
    }

    fn request_will_be_sent(
        &mut self,
        event: &CdpEvent,
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
        let post_data = request
            .get("postData")
            .map(|value| {
                value
                    .as_str()
                    .ok_or(CdpNetworkEventError::InvalidEvent {
                        method: "Network.requestWillBeSent",
                        field: "request.postData",
                    })
                    .and_then(|value| {
                        bounded_bytes("request.postData", value.as_bytes(), limits.post_data_bytes)
                    })
            })
            .transpose()?;
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
        let request = if let Some(post_data) = post_data {
            request.with_post_data(post_data)
        } else {
            request
        };
        self.tracker
            .request_will_be_sent(request, redirect)
            .map_err(CdpNetworkEventError::Lifecycle)
    }

    fn response_received(
        &mut self,
        event: &CdpEvent,
    ) -> Result<NetworkLifecycleEffect, CdpNetworkEventError> {
        let limits = self.tracker.limits();
        let request_id = required_string(event, "requestId", limits.native_id_bytes)?;
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
        self.tracker
            .response_received(&request_id, response)
            .map_err(CdpNetworkEventError::Lifecycle)
    }

    fn loading_finished(
        &mut self,
        event: &CdpEvent,
    ) -> Result<NetworkLifecycleEffect, CdpNetworkEventError> {
        let limits = self.tracker.limits();
        let request_id = required_string(event, "requestId", limits.native_id_bytes)?;
        let encoded_body_bytes = event
            .params
            .get("encodedDataLength")
            .map(|value| parse_byte_count(value, "Network.loadingFinished", "encodedDataLength"))
            .transpose()?;
        self.tracker
            .loading_finished(&request_id, encoded_body_bytes)
            .map_err(CdpNetworkEventError::Lifecycle)
    }

    fn loading_failed(
        &mut self,
        event: &CdpEvent,
    ) -> Result<NetworkLifecycleEffect, CdpNetworkEventError> {
        let limits = self.tracker.limits();
        let request_id = required_string(event, "requestId", limits.native_id_bytes)?;
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
        self.tracker
            .loading_failed(&request_id, failure_text, canceled)
            .map_err(CdpNetworkEventError::Lifecycle)
    }

    fn portable_batch(&self, effects: Vec<NetworkLifecycleEffect>) -> CdpNetworkEventBatch {
        let mut lifecycle = Vec::with_capacity(effects.len());
        let mut events = Vec::with_capacity(effects.len());
        let mut retirement = Vec::with_capacity(effects.len());
        for effect in effects {
            match effect {
                NetworkLifecycleEffect::Request(payload) => {
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
                    retirement.push(ControlMessage::Dispose {
                        object_id: payload.request.request,
                    });
                    events.push(CdpNetworkEventEffect::RequestFinished(Event::new(
                        self.page, payload,
                    )));
                }
                NetworkLifecycleEffect::RequestFailed(payload) => {
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
            lifecycle,
            events,
            retirement,
        }
    }

    fn next_object_id(&self) -> Result<ObjectId, CdpNetworkEventError> {
        self.object_ids
            .next_object_id()
            .map_err(CdpNetworkEventError::ObjectIds)
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
    Ok(ResponseStart::new(response, url, status, status_text)
        .with_headers(headers)
        .from_service_worker(from_service_worker))
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
        "Network.responseReceived" => "Network.responseReceived",
        "Network.loadingFinished" => "Network.loadingFinished",
        "Network.loadingFailed" => "Network.loadingFailed",
        _ => "Network event",
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdpNetworkEventError {
    ZeroPageObject,
    EmptySessionId,
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

    use pwright_cap_network_events::{NetworkRequest, NetworkRequestFailed, NetworkResponse};
    use pwright_cdp::CdpEvent;
    use pwright_core::{
        AtomicObjectIdSource, ControlMessage, ObjectId, ProductionResourceLedger, ResourceClass,
        ResourceLimits, ResourceScope,
    };
    use pwright_network_model::NetworkLifecycleLimits;
    use serde_json::json;

    use super::{
        CdpNetworkEventAdapter, CdpNetworkEventEffect, CdpNetworkEventError,
        CdpNetworkEventProductionProfile,
    };

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
}
