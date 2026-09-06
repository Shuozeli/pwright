//! Backend-neutral, command-free HTTP lifecycle values and correlation state.
//!
//! This crate does not select a browser backend, event queue, codec, runtime,
//! executor, response-body reader, routing policy, WebSocket observer, or HAR
//! recorder. Native backends inject stable portable object IDs and consume the
//! bounded effects produced here. Terminal effects own complete immutable
//! snapshots; callable post-terminal handles are a separate capability.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_core::ObjectId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

impl HttpHeader {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn retained_string_capacity(&self) -> usize {
        self.name.capacity().saturating_add(self.value.capacity())
    }

    fn compact(&mut self) {
        self.name.shrink_to_fit();
        self.value.shrink_to_fit();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestInitializer {
    pub request: ObjectId,
    pub parent: ObjectId,
    pub url: String,
    pub method: String,
    pub resource_type: String,
    pub headers: Vec<HttpHeader>,
    pub post_data: Option<Vec<u8>>,
    pub is_navigation_request: bool,
    pub redirected_from: Option<ObjectId>,
}

impl RequestInitializer {
    pub fn retained_bytes(&self) -> usize {
        let headers = self
            .headers
            .iter()
            .map(HttpHeader::retained_string_capacity)
            .fold(0_usize, usize::saturating_add);
        self.url
            .capacity()
            .saturating_add(self.method.capacity())
            .saturating_add(self.resource_type.capacity())
            .saturating_add(headers)
            .saturating_add(self.post_data.as_ref().map_or(0, Vec::capacity))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponseInitializer {
    pub response: ObjectId,
    pub request: ObjectId,
    pub url: String,
    pub status: i64,
    pub status_text: String,
    pub headers: Vec<HttpHeader>,
    pub from_service_worker: bool,
}

impl ResponseInitializer {
    pub fn retained_bytes(&self) -> usize {
        let headers = self
            .headers
            .iter()
            .map(HttpHeader::retained_string_capacity)
            .fold(0_usize, usize::saturating_add);
        self.url
            .capacity()
            .saturating_add(self.status_text.capacity())
            .saturating_add(headers)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestFinished {
    /// Immutable Request state captured before the backend releases its
    /// native correlation entry. A terminal-only subscriber can therefore
    /// consume this event without a separately retained live Request handle.
    pub request: RequestInitializer,
    /// Immutable Response state when headers were observed before completion.
    pub response: Option<ResponseInitializer>,
    pub encoded_body_bytes: Option<u64>,
}

impl RequestFinished {
    pub fn retained_bytes(&self) -> usize {
        self.request.retained_bytes().saturating_add(
            self.response
                .as_ref()
                .map_or(0, ResponseInitializer::retained_bytes),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestFailed {
    /// Immutable Request state captured before native correlation is removed.
    pub request: RequestInitializer,
    /// Immutable Response state when failure followed response headers.
    pub response: Option<ResponseInitializer>,
    pub failure_text: String,
    pub canceled: bool,
}

impl RequestFailed {
    pub fn retained_bytes(&self) -> usize {
        self.request
            .retained_bytes()
            .saturating_add(
                self.response
                    .as_ref()
                    .map_or(0, ResponseInitializer::retained_bytes),
            )
            .saturating_add(self.failure_text.capacity())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkLifecycleEffect {
    Request(RequestInitializer),
    Response(ResponseInitializer),
    RequestFinished(RequestFinished),
    RequestFailed(RequestFailed),
}

impl NetworkLifecycleEffect {
    pub fn retained_bytes(&self) -> usize {
        match self {
            Self::Request(event) => event.retained_bytes(),
            Self::Response(event) => event.retained_bytes(),
            Self::RequestFinished(event) => event.retained_bytes(),
            Self::RequestFailed(event) => event.retained_bytes(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetworkLifecycleLimits {
    pub active_requests: usize,
    pub native_id_bytes: usize,
    pub url_bytes: usize,
    pub method_bytes: usize,
    pub resource_type_bytes: usize,
    pub header_count: usize,
    pub header_bytes: usize,
    pub post_data_bytes: usize,
    pub status_text_bytes: usize,
    pub failure_text_bytes: usize,
}

impl NetworkLifecycleLimits {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        active_requests: usize,
        native_id_bytes: usize,
        url_bytes: usize,
        method_bytes: usize,
        resource_type_bytes: usize,
        header_count: usize,
        header_bytes: usize,
        post_data_bytes: usize,
        status_text_bytes: usize,
        failure_text_bytes: usize,
    ) -> Self {
        Self {
            active_requests,
            native_id_bytes,
            url_bytes,
            method_bytes,
            resource_type_bytes,
            header_count,
            header_bytes,
            post_data_bytes,
            status_text_bytes,
            failure_text_bytes,
        }
    }

    pub fn validate(self) -> Result<(), NetworkLifecycleError> {
        for (name, value) in [
            ("active request count", self.active_requests),
            ("native request ID bytes", self.native_id_bytes),
            ("URL bytes", self.url_bytes),
            ("method bytes", self.method_bytes),
            ("resource type bytes", self.resource_type_bytes),
            ("header count", self.header_count),
            ("header bytes", self.header_bytes),
            ("status text bytes", self.status_text_bytes),
            ("failure text bytes", self.failure_text_bytes),
        ] {
            if value == 0 {
                return Err(NetworkLifecycleError::ZeroLimit { name });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestStart {
    pub native_request_id: String,
    pub request: ObjectId,
    pub parent: ObjectId,
    pub url: String,
    pub method: String,
    pub resource_type: String,
    pub headers: Vec<HttpHeader>,
    pub post_data: Option<Vec<u8>>,
    pub is_navigation_request: bool,
}

impl RequestStart {
    pub fn new(
        native_request_id: impl Into<String>,
        request: ObjectId,
        parent: ObjectId,
        url: impl Into<String>,
        method: impl Into<String>,
        resource_type: impl Into<String>,
    ) -> Self {
        Self {
            native_request_id: native_request_id.into(),
            request,
            parent,
            url: url.into(),
            method: method.into(),
            resource_type: resource_type.into(),
            headers: Vec::new(),
            post_data: None,
            is_navigation_request: false,
        }
    }

    pub fn with_headers(mut self, headers: Vec<HttpHeader>) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_post_data(mut self, post_data: Vec<u8>) -> Self {
        self.post_data = Some(post_data);
        self
    }

    pub const fn navigation(mut self, is_navigation_request: bool) -> Self {
        self.is_navigation_request = is_navigation_request;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponseStart {
    pub response: ObjectId,
    pub url: String,
    pub status: i64,
    pub status_text: String,
    pub headers: Vec<HttpHeader>,
    pub from_service_worker: bool,
}

impl ResponseStart {
    pub fn new(
        response: ObjectId,
        url: impl Into<String>,
        status: i64,
        status_text: impl Into<String>,
    ) -> Self {
        Self {
            response,
            url: url.into(),
            status,
            status_text: status_text.into(),
            headers: Vec::new(),
            from_service_worker: false,
        }
    }

    pub fn with_headers(mut self, headers: Vec<HttpHeader>) -> Self {
        self.headers = headers;
        self
    }

    pub const fn from_service_worker(mut self, value: bool) -> Self {
        self.from_service_worker = value;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedirectResponse {
    pub response: ObjectId,
    pub metadata: ResponseStart,
}

impl RedirectResponse {
    pub const fn new(response: ObjectId, metadata: ResponseStart) -> Self {
        Self { response, metadata }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveRequest {
    request: RequestInitializer,
    response: Option<ResponseInitializer>,
}

#[derive(Debug)]
pub struct NetworkLifecycleTracker {
    limits: NetworkLifecycleLimits,
    active: BTreeMap<String, ActiveRequest>,
}

impl NetworkLifecycleTracker {
    pub fn new(limits: NetworkLifecycleLimits) -> Result<Self, NetworkLifecycleError> {
        limits.validate()?;
        Ok(Self {
            limits,
            active: BTreeMap::new(),
        })
    }

    pub const fn limits(&self) -> NetworkLifecycleLimits {
        self.limits
    }

    pub fn active_request_count(&self) -> usize {
        self.active.len()
    }

    pub fn retained_native_id_capacity(&self) -> usize {
        self.active
            .keys()
            .map(String::capacity)
            .fold(0_usize, usize::saturating_add)
    }

    pub fn active_objects(&self) -> impl Iterator<Item = (ObjectId, Option<ObjectId>)> + '_ {
        self.active.values().map(|active| {
            (
                active.request.request,
                active.response.as_ref().map(|response| response.response),
            )
        })
    }

    pub fn request_will_be_sent(
        &mut self,
        mut request: RequestStart,
        redirect: Option<RedirectResponse>,
    ) -> Result<Vec<NetworkLifecycleEffect>, NetworkLifecycleError> {
        self.validate_request(&request)?;
        if let Some(redirect) = &redirect {
            self.validate_response(&redirect.metadata)?;
            if redirect.response != redirect.metadata.response {
                return Err(NetworkLifecycleError::RedirectResponseIdentityMismatch {
                    declared: redirect.response,
                    metadata: redirect.metadata.response,
                });
            }
        }
        validate_object_id("Request", request.request)?;
        validate_object_id("Request parent", request.parent)?;

        let previous = self.active.get(&request.native_request_id);
        match (previous, redirect.as_ref()) {
            (Some(_), None) => {
                return Err(NetworkLifecycleError::DuplicateNativeRequest {
                    native_request_id: request.native_request_id,
                });
            }
            (None, Some(_)) => {
                return Err(NetworkLifecycleError::UnknownRedirectPredecessor {
                    native_request_id: request.native_request_id,
                });
            }
            (None, None) if self.active.len() >= self.limits.active_requests => {
                return Err(NetworkLifecycleError::LimitExceeded {
                    field: "active request count",
                    actual: self.active.len().saturating_add(1),
                    limit: self.limits.active_requests,
                });
            }
            _ => {}
        }

        request.native_request_id.shrink_to_fit();
        request.url.shrink_to_fit();
        request.method.shrink_to_fit();
        request.resource_type.shrink_to_fit();
        request.headers.iter_mut().for_each(HttpHeader::compact);
        request.headers.shrink_to_fit();
        if let Some(post_data) = request.post_data.as_mut() {
            post_data.shrink_to_fit();
        }

        let mut effects = Vec::with_capacity(if redirect.is_some() { 3 } else { 1 });
        let redirected_from = if let Some(redirect) = redirect {
            let previous = self
                .active
                .remove(&request.native_request_id)
                .expect("validated redirect predecessor");
            if previous.response.is_some() {
                return Err(NetworkLifecycleError::DuplicateResponse {
                    native_request_id: request.native_request_id,
                });
            }
            let previous_request = previous.request.request;
            let response = compact_response(redirect.metadata, previous.request.request);
            effects.push(NetworkLifecycleEffect::Response(response.clone()));
            effects.push(NetworkLifecycleEffect::RequestFinished(RequestFinished {
                request: previous.request,
                response: Some(response),
                encoded_body_bytes: None,
            }));
            Some(previous_request)
        } else {
            None
        };

        let native_request_id = request.native_request_id;
        let initializer = RequestInitializer {
            request: request.request,
            parent: request.parent,
            url: request.url,
            method: request.method,
            resource_type: request.resource_type,
            headers: request.headers,
            post_data: request.post_data,
            is_navigation_request: request.is_navigation_request,
            redirected_from,
        };
        self.active.insert(
            native_request_id,
            ActiveRequest {
                request: initializer.clone(),
                response: None,
            },
        );
        effects.push(NetworkLifecycleEffect::Request(initializer));
        Ok(effects)
    }

    pub fn response_received(
        &mut self,
        native_request_id: &str,
        response: ResponseStart,
    ) -> Result<NetworkLifecycleEffect, NetworkLifecycleError> {
        validate_bounded(
            "native request ID bytes",
            native_request_id.len(),
            self.limits.native_id_bytes,
        )?;
        self.validate_response(&response)?;
        let active = self.active.get_mut(native_request_id).ok_or_else(|| {
            NetworkLifecycleError::UnknownNativeRequest {
                native_request_id: native_request_id.to_owned(),
            }
        })?;
        if active.response.is_some() {
            return Err(NetworkLifecycleError::DuplicateResponse {
                native_request_id: native_request_id.to_owned(),
            });
        }
        let effect =
            NetworkLifecycleEffect::Response(compact_response(response, active.request.request));
        let NetworkLifecycleEffect::Response(snapshot) = &effect else {
            unreachable!("the constructed network effect is a Response")
        };
        active.response = Some(snapshot.clone());
        Ok(effect)
    }

    pub fn loading_finished(
        &mut self,
        native_request_id: &str,
        encoded_body_bytes: Option<u64>,
    ) -> Result<NetworkLifecycleEffect, NetworkLifecycleError> {
        let active = self.remove_active(native_request_id)?;
        Ok(NetworkLifecycleEffect::RequestFinished(RequestFinished {
            request: active.request,
            response: active.response,
            encoded_body_bytes,
        }))
    }

    pub fn loading_failed(
        &mut self,
        native_request_id: &str,
        mut failure_text: String,
        canceled: bool,
    ) -> Result<NetworkLifecycleEffect, NetworkLifecycleError> {
        validate_bounded(
            "failure text bytes",
            failure_text.len(),
            self.limits.failure_text_bytes,
        )?;
        let active = self.remove_active(native_request_id)?;
        failure_text.shrink_to_fit();
        Ok(NetworkLifecycleEffect::RequestFailed(RequestFailed {
            request: active.request,
            response: active.response,
            failure_text,
            canceled,
        }))
    }

    pub fn drain_active(&mut self) -> Vec<(ObjectId, Option<ObjectId>)> {
        let drained = std::mem::take(&mut self.active);
        drained
            .into_values()
            .map(|active| {
                (
                    active.request.request,
                    active.response.map(|response| response.response),
                )
            })
            .collect()
    }

    fn remove_active(
        &mut self,
        native_request_id: &str,
    ) -> Result<ActiveRequest, NetworkLifecycleError> {
        validate_bounded(
            "native request ID bytes",
            native_request_id.len(),
            self.limits.native_id_bytes,
        )?;
        self.active.remove(native_request_id).ok_or_else(|| {
            NetworkLifecycleError::UnknownNativeRequest {
                native_request_id: native_request_id.to_owned(),
            }
        })
    }

    fn validate_request(&self, request: &RequestStart) -> Result<(), NetworkLifecycleError> {
        validate_bounded(
            "native request ID bytes",
            request.native_request_id.len(),
            self.limits.native_id_bytes,
        )?;
        validate_bounded("URL bytes", request.url.len(), self.limits.url_bytes)?;
        validate_bounded(
            "method bytes",
            request.method.len(),
            self.limits.method_bytes,
        )?;
        validate_bounded(
            "resource type bytes",
            request.resource_type.len(),
            self.limits.resource_type_bytes,
        )?;
        validate_headers(&request.headers, self.limits)?;
        validate_bounded(
            "post data bytes",
            request.post_data.as_ref().map_or(0, Vec::len),
            self.limits.post_data_bytes,
        )
    }

    fn validate_response(&self, response: &ResponseStart) -> Result<(), NetworkLifecycleError> {
        validate_object_id("Response", response.response)?;
        validate_bounded("URL bytes", response.url.len(), self.limits.url_bytes)?;
        validate_bounded(
            "status text bytes",
            response.status_text.len(),
            self.limits.status_text_bytes,
        )?;
        validate_headers(&response.headers, self.limits)
    }
}

fn compact_response(mut response: ResponseStart, request: ObjectId) -> ResponseInitializer {
    response.url.shrink_to_fit();
    response.status_text.shrink_to_fit();
    response.headers.iter_mut().for_each(HttpHeader::compact);
    response.headers.shrink_to_fit();
    ResponseInitializer {
        response: response.response,
        request,
        url: response.url,
        status: response.status,
        status_text: response.status_text,
        headers: response.headers,
        from_service_worker: response.from_service_worker,
    }
}

fn validate_headers(
    headers: &[HttpHeader],
    limits: NetworkLifecycleLimits,
) -> Result<(), NetworkLifecycleError> {
    validate_bounded("header count", headers.len(), limits.header_count)?;
    let mut bytes = 0_usize;
    for (index, header) in headers.iter().enumerate() {
        if header.name.is_empty() {
            return Err(NetworkLifecycleError::EmptyHeaderName { index });
        }
        bytes = bytes
            .checked_add(header.name.len())
            .and_then(|value| value.checked_add(header.value.len()))
            .ok_or(NetworkLifecycleError::SizeOverflow {
                field: "header bytes",
            })?;
        validate_bounded("header bytes", bytes, limits.header_bytes)?;
    }
    Ok(())
}

fn validate_object_id(
    name: &'static str,
    object_id: ObjectId,
) -> Result<(), NetworkLifecycleError> {
    if object_id.get() == 0 {
        return Err(NetworkLifecycleError::ZeroObjectId { name });
    }
    Ok(())
}

fn validate_bounded(
    field: &'static str,
    actual: usize,
    limit: usize,
) -> Result<(), NetworkLifecycleError> {
    if actual > limit {
        return Err(NetworkLifecycleError::LimitExceeded {
            field,
            actual,
            limit,
        });
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkLifecycleError {
    ZeroLimit {
        name: &'static str,
    },
    ZeroObjectId {
        name: &'static str,
    },
    LimitExceeded {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    SizeOverflow {
        field: &'static str,
    },
    EmptyHeaderName {
        index: usize,
    },
    DuplicateNativeRequest {
        native_request_id: String,
    },
    UnknownNativeRequest {
        native_request_id: String,
    },
    UnknownRedirectPredecessor {
        native_request_id: String,
    },
    DuplicateResponse {
        native_request_id: String,
    },
    RedirectResponseIdentityMismatch {
        declared: ObjectId,
        metadata: ObjectId,
    },
}

impl Display for NetworkLifecycleError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit { name } => write!(formatter, "network {name} limit must be nonzero"),
            Self::ZeroObjectId { name } => write!(formatter, "{name} object ID zero is invalid"),
            Self::LimitExceeded {
                field,
                actual,
                limit,
            } => write!(
                formatter,
                "network {field} {actual} exceeds configured limit {limit}"
            ),
            Self::SizeOverflow { field } => {
                write!(formatter, "network {field} calculation overflowed")
            }
            Self::EmptyHeaderName { index } => {
                write!(formatter, "network header {index} has an empty name")
            }
            Self::DuplicateNativeRequest { native_request_id } => write!(
                formatter,
                "native request `{native_request_id}` started twice without a redirect"
            ),
            Self::UnknownNativeRequest { native_request_id } => {
                write!(
                    formatter,
                    "native request `{native_request_id}` is not active"
                )
            }
            Self::UnknownRedirectPredecessor { native_request_id } => write!(
                formatter,
                "native redirect `{native_request_id}` has no active predecessor"
            ),
            Self::DuplicateResponse { native_request_id } => write!(
                formatter,
                "native request `{native_request_id}` already has a Response"
            ),
            Self::RedirectResponseIdentityMismatch { declared, metadata } => write!(
                formatter,
                "redirect Response ID `{declared}` differs from metadata ID `{metadata}`"
            ),
        }
    }
}

impl Error for NetworkLifecycleError {}

#[cfg(test)]
mod tests {
    use pwright_core::ObjectId;

    use super::{
        HttpHeader, NetworkLifecycleEffect, NetworkLifecycleError, NetworkLifecycleLimits,
        NetworkLifecycleTracker, RedirectResponse, RequestStart, ResponseStart,
    };

    fn limits(active_requests: usize) -> NetworkLifecycleLimits {
        NetworkLifecycleLimits::new(active_requests, 32, 256, 16, 32, 8, 256, 64, 64, 128)
    }

    fn request(native: &str, object: u64) -> RequestStart {
        RequestStart::new(
            native,
            ObjectId::new(object),
            ObjectId::new(3),
            "https://example.test/data",
            "GET",
            "Fetch",
        )
        .with_headers(vec![HttpHeader::new("accept", "application/json")])
    }

    fn response(object: u64, status: i64) -> ResponseStart {
        ResponseStart::new(
            ObjectId::new(object),
            "https://example.test/data",
            status,
            if status == 302 { "Found" } else { "OK" },
        )
        .with_headers(vec![HttpHeader::new("content-type", "application/json")])
    }

    #[test]
    fn successful_exchange_uses_one_identity_chain_and_releases_native_state() {
        // Arrange
        let mut tracker = NetworkLifecycleTracker::new(limits(2)).expect("limits should validate");
        let started = tracker
            .request_will_be_sent(request("native-1", 10), None)
            .expect("request should start");

        // Act
        let received = tracker
            .response_received("native-1", response(11, 200))
            .expect("response should arrive");
        let finished = tracker
            .loading_finished("native-1", Some(42))
            .expect("request should finish");

        // Assert
        let NetworkLifecycleEffect::Request(started) = &started[0] else {
            panic!("first effect should be Request");
        };
        assert_eq!(started.request, ObjectId::new(10));
        let NetworkLifecycleEffect::Response(received) = received else {
            panic!("second effect should be Response");
        };
        assert_eq!(received.request, ObjectId::new(10));
        assert_eq!(received.response, ObjectId::new(11));
        let NetworkLifecycleEffect::RequestFinished(finished) = finished else {
            panic!("terminal effect should be request-finished");
        };
        assert_eq!(finished.request.request, ObjectId::new(10));
        assert_eq!(finished.request.url, "https://example.test/data");
        let finished_response = finished
            .response
            .as_ref()
            .expect("terminal snapshot should include the Response");
        assert_eq!(finished_response.response, ObjectId::new(11));
        assert_eq!(finished_response.status, 200);
        assert_eq!(finished.encoded_body_bytes, Some(42));
        assert_eq!(tracker.active_request_count(), 0);
        assert_eq!(tracker.retained_native_id_capacity(), 0);
    }

    #[test]
    fn http_error_status_is_a_response_and_finished_not_a_failure() {
        // Arrange
        let mut tracker = NetworkLifecycleTracker::new(limits(1)).expect("limits should validate");
        tracker
            .request_will_be_sent(request("native-1", 10), None)
            .expect("request should start");

        // Act
        let received = tracker
            .response_received("native-1", response(11, 500))
            .expect("HTTP error response should arrive");
        let finished = tracker
            .loading_finished("native-1", None)
            .expect("HTTP error response should finish");

        // Assert
        assert!(matches!(received, NetworkLifecycleEffect::Response(_)));
        assert!(matches!(
            finished,
            NetworkLifecycleEffect::RequestFinished(_)
        ));
    }

    #[test]
    fn transport_failure_is_terminal_and_drops_native_correlation() {
        // Arrange
        let mut tracker = NetworkLifecycleTracker::new(limits(1)).expect("limits should validate");
        tracker
            .request_will_be_sent(request("native-1", 10), None)
            .expect("request should start");

        // Act
        let failed = tracker
            .loading_failed("native-1", "net::ERR_ABORTED".to_owned(), true)
            .expect("failure should terminate request");

        // Assert
        let NetworkLifecycleEffect::RequestFailed(failed) = failed else {
            panic!("terminal effect should be request-failed");
        };
        assert_eq!(failed.request.request, ObjectId::new(10));
        assert_eq!(failed.request.method, "GET");
        assert_eq!(failed.response, None);
        assert!(failed.canceled);
        assert_eq!(tracker.active_request_count(), 0);
    }

    #[test]
    fn redirect_finishes_old_pair_before_starting_a_new_request_identity() {
        // Arrange
        let mut tracker = NetworkLifecycleTracker::new(limits(1)).expect("limits should validate");
        tracker
            .request_will_be_sent(request("native-1", 10), None)
            .expect("first hop should start");
        let redirect = RedirectResponse::new(ObjectId::new(11), response(11, 302));

        // Act
        let effects = tracker
            .request_will_be_sent(request("native-1", 12), Some(redirect))
            .expect("second hop should replace the first within one active slot");

        // Assert
        assert_eq!(effects.len(), 3);
        assert!(matches!(effects[0], NetworkLifecycleEffect::Response(_)));
        assert!(matches!(
            effects[1],
            NetworkLifecycleEffect::RequestFinished(_)
        ));
        let NetworkLifecycleEffect::Request(second) = &effects[2] else {
            panic!("last redirect effect should start the next Request");
        };
        assert_eq!(second.request, ObjectId::new(12));
        assert_eq!(second.redirected_from, Some(ObjectId::new(10)));
        assert_eq!(tracker.active_request_count(), 1);
        assert_eq!(
            tracker.active_objects().collect::<Vec<_>>(),
            vec![(ObjectId::new(12), None)]
        );
    }

    #[test]
    fn active_limit_rejects_before_mutating_existing_requests() {
        // Arrange
        let mut tracker = NetworkLifecycleTracker::new(limits(1)).expect("limits should validate");
        tracker
            .request_will_be_sent(request("native-1", 10), None)
            .expect("first request should fit");

        // Act
        let error = tracker
            .request_will_be_sent(request("native-2", 20), None)
            .expect_err("second concurrent request should exceed the limit");

        // Assert
        assert_eq!(
            error,
            NetworkLifecycleError::LimitExceeded {
                field: "active request count",
                actual: 2,
                limit: 1,
            }
        );
        assert_eq!(
            tracker.active_objects().collect::<Vec<_>>(),
            vec![(ObjectId::new(10), None)]
        );
    }

    #[test]
    fn oversized_metadata_rejects_before_native_state_is_inserted() {
        // Arrange
        let mut tracker = NetworkLifecycleTracker::new(limits(1)).expect("limits should validate");
        let oversized = RequestStart::new(
            "native-1",
            ObjectId::new(10),
            ObjectId::new(3),
            "x".repeat(257),
            "GET",
            "Fetch",
        );

        // Act
        let error = tracker
            .request_will_be_sent(oversized, None)
            .expect_err("oversized URL should fail");

        // Assert
        assert_eq!(
            error,
            NetworkLifecycleError::LimitExceeded {
                field: "URL bytes",
                actual: 257,
                limit: 256,
            }
        );
        assert_eq!(tracker.active_request_count(), 0);
    }

    #[test]
    fn teardown_drains_every_active_portable_identity_without_history() {
        // Arrange
        let mut tracker = NetworkLifecycleTracker::new(limits(2)).expect("limits should validate");
        tracker
            .request_will_be_sent(request("native-1", 10), None)
            .expect("first request should fit");
        tracker
            .request_will_be_sent(request("native-2", 20), None)
            .expect("second request should fit");
        tracker
            .response_received("native-2", response(21, 200))
            .expect("response should attach");

        // Act
        let drained = tracker.drain_active();

        // Assert
        assert_eq!(
            drained,
            vec![
                (ObjectId::new(10), None),
                (ObjectId::new(20), Some(ObjectId::new(21))),
            ]
        );
        assert_eq!(tracker.active_request_count(), 0);
        assert_eq!(tracker.retained_native_id_capacity(), 0);
    }

    #[test]
    fn sustained_completed_requests_retain_no_history_and_never_exceed_active_bound() {
        // Arrange
        const ITERATIONS: u64 = 4_096;
        let configured = limits(2);
        let mut tracker = NetworkLifecycleTracker::new(configured).expect("limits should validate");
        let mut peak_active = 0usize;
        let mut peak_native_id_capacity = 0usize;

        // Act
        for index in 0..ITERATIONS {
            let native = format!("r-{index}");
            tracker
                .request_will_be_sent(request(&native, 10 + index * 2), None)
                .expect("bounded request should start");
            peak_active = peak_active.max(tracker.active_request_count());
            peak_native_id_capacity =
                peak_native_id_capacity.max(tracker.retained_native_id_capacity());
            tracker
                .response_received(&native, response(11 + index * 2, 200))
                .expect("response should attach");
            tracker
                .loading_finished(&native, Some(42))
                .expect("request should finish");
        }

        // Assert
        assert_eq!(peak_active, 1);
        assert!(peak_active <= configured.active_requests);
        assert!(peak_native_id_capacity <= configured.native_id_bytes);
        assert_eq!(tracker.active_request_count(), 0);
        assert_eq!(tracker.retained_native_id_capacity(), 0);
        assert_eq!(tracker.active_objects().count(), 0);
    }
}
