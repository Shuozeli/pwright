//! Browser-independent API request ownership shared by language bindings.
//!
//! This crate deliberately contains no concrete HTTP backend. A distribution
//! selects a transport plugin and may omit the entire capability. The context
//! owns cancellation, bounded response retention and deterministic disposal;
//! language bindings only convert public values.

use std::{
    collections::HashMap,
    fmt,
    net::IpAddr,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use url::Url;

const CANCELLATION_SETTLEMENT_GRACE: Duration = Duration::from_millis(100);

mod payload;

pub use payload::{ApiFilePayload, ApiMultipartField, ApiNameValue, ApiQuery, ApiRequestBody};
use payload::{apply_query, serialize_body};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_MAX_REDIRECTS: u16 = 20;
pub const DEFAULT_USER_AGENT: &str = concat!("Pwright/", env!("CARGO_PKG_VERSION"));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApiRequestLimits {
    pub max_in_flight: usize,
    pub max_live_responses: usize,
    pub max_request_body_bytes: usize,
    pub max_request_body_fields: usize,
    pub max_query_parameters: usize,
    pub max_lookup_addresses: usize,
    pub max_response_body_bytes: usize,
    pub max_retained_body_bytes: usize,
    pub max_headers: usize,
    pub max_header_bytes: usize,
    pub max_method_bytes: usize,
    pub max_url_bytes: usize,
    pub max_status_text_bytes: usize,
    pub max_log_entries: usize,
    pub max_log_bytes: usize,
    pub max_retained_log_bytes: usize,
}

impl Default for ApiRequestLimits {
    fn default() -> Self {
        Self {
            max_in_flight: 8,
            max_live_responses: 64,
            max_request_body_bytes: 16 * 1024 * 1024,
            max_request_body_fields: 1024,
            max_query_parameters: 1024,
            max_lookup_addresses: 64,
            max_response_body_bytes: 16 * 1024 * 1024,
            max_retained_body_bytes: 64 * 1024 * 1024,
            max_headers: 256,
            max_header_bytes: 256 * 1024,
            max_method_bytes: 1024,
            max_url_bytes: 64 * 1024,
            max_status_text_bytes: 8 * 1024,
            max_log_entries: 64,
            max_log_bytes: 256 * 1024,
            max_retained_log_bytes: 1024 * 1024,
        }
    }
}

impl ApiRequestLimits {
    fn validate(self) -> Result<Self, ApiRequestError> {
        let values = [
            self.max_in_flight,
            self.max_live_responses,
            self.max_request_body_bytes,
            self.max_request_body_fields,
            self.max_query_parameters,
            self.max_lookup_addresses,
            self.max_response_body_bytes,
            self.max_retained_body_bytes,
            self.max_headers,
            self.max_header_bytes,
            self.max_method_bytes,
            self.max_url_bytes,
            self.max_status_text_bytes,
            self.max_log_entries,
            self.max_log_bytes,
            self.max_retained_log_bytes,
        ];
        if values.contains(&0) {
            return Err(ApiRequestError::InvalidArgument(
                "API request limits must be nonzero".into(),
            ));
        }
        if self.max_in_flight > Semaphore::MAX_PERMITS {
            return Err(ApiRequestError::InvalidArgument(format!(
                "max_in_flight cannot exceed {}",
                Semaphore::MAX_PERMITS
            )));
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiHeader {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiSameSite {
    Strict,
    Lax,
    None,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApiCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    /// Unix seconds, or `-1.0` for a session cookie.
    pub expires: f64,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: ApiSameSite,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{message}")]
pub struct ApiCookieStoreError {
    pub message: String,
}

impl ApiCookieStoreError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[async_trait]
pub trait ApiCookieStore: Send + Sync + fmt::Debug + 'static {
    /// Return the exact Cookie header value for `url`, if any.
    async fn request_header(&self, url: &Url) -> Result<Option<String>, ApiCookieStoreError>;

    /// Consume raw Set-Cookie values in wire order for one response URL.
    async fn store_response_headers(
        &self,
        url: &Url,
        values: &[String],
    ) -> Result<(), ApiCookieStoreError>;

    /// Snapshot all unexpired cookies in deterministic store order.
    async fn all_cookies(&self) -> Result<Vec<ApiCookie>, ApiCookieStoreError>;

    /// Replace or add structured cookies, used by storage-state import and
    /// `BrowserContext` adapters.
    async fn add_cookies(&self, cookies: Vec<ApiCookie>) -> Result<(), ApiCookieStoreError>;
}

pub trait ApiAuthorizationProvider: Send + Sync + fmt::Debug + 'static {
    /// Return a preemptive Authorization value for `url`, if selected by policy.
    fn preemptive_header(&self, url: &Url) -> Option<String>;

    /// Return an Authorization value after one response challenge, if supported.
    fn challenge_header(&self, url: &Url, www_authenticate: &str) -> Option<String>;
}

impl ApiHeader {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ApiRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<ApiHeader>,
    pub query: ApiQuery,
    pub body: ApiRequestBody,
    /// `None` uses the context default. `Some(Duration::ZERO)` disables the
    /// timeout, matching Playwright's `timeout: 0` behavior.
    pub timeout: Option<Duration>,
    pub fail_on_status_code: Option<bool>,
    pub ignore_https_errors: Option<bool>,
    pub max_redirects: Option<u16>,
    pub max_retries: u16,
    /// Optional request-scoped resolver output. The URL host remains
    /// authoritative for HTTP Host and TLS SNI.
    pub lookup_addresses: Option<Vec<IpAddr>>,
}

impl ApiRequest {
    pub fn new(method: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            url: url.into(),
            headers: Vec::new(),
            query: ApiQuery::None,
            body: ApiRequestBody::None,
            timeout: None,
            fail_on_status_code: None,
            ignore_https_errors: None,
            max_redirects: None,
            max_retries: 0,
            lookup_addresses: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TransportRequest {
    pub method: String,
    pub url: Url,
    pub headers: Vec<ApiHeader>,
    pub body: Option<Vec<u8>>,
    /// `None` means no timeout.
    pub timeout: Option<Duration>,
    pub ignore_https_errors: bool,
    pub cookie_store: Option<Arc<dyn ApiCookieStore>>,
    pub authorization_provider: Option<Arc<dyn ApiAuthorizationProvider>>,
    pub max_request_header_bytes: usize,
    pub max_response_body_bytes: usize,
    pub max_response_headers: usize,
    pub max_response_header_bytes: usize,
    pub max_response_status_text_bytes: usize,
    pub max_response_log_entries: usize,
    pub max_response_log_bytes: usize,
    pub max_redirects: u16,
    pub max_retries: u16,
    pub lookup_addresses: Option<Vec<IpAddr>>,
    pub cancellation: CancellationToken,
}

#[derive(Clone, Debug)]
pub struct TransportResponse {
    pub url: String,
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<ApiHeader>,
    pub body: Vec<u8>,
    pub log: Vec<String>,
}

#[async_trait]
pub trait ApiRequestTransport: Send + Sync + 'static {
    /// Execute one request. Implementations must enforce
    /// `max_response_body_bytes` while streaming, observe `cancellation`, and
    /// return no partially retained response after failure.
    async fn execute(
        &self,
        request: TransportRequest,
    ) -> Result<TransportResponse, ApiRequestError>;
}

#[derive(Clone, Debug)]
pub struct ApiRequestContextOptions {
    pub base_url: Option<String>,
    pub user_agent: String,
    pub extra_headers: Vec<ApiHeader>,
    pub timeout: Duration,
    pub fail_on_status_code: bool,
    pub ignore_https_errors: bool,
    pub cookie_store: Option<Arc<dyn ApiCookieStore>>,
    pub authorization_provider: Option<Arc<dyn ApiAuthorizationProvider>>,
    pub max_redirects: u16,
    pub limits: ApiRequestLimits,
}

impl Default for ApiRequestContextOptions {
    fn default() -> Self {
        Self {
            base_url: None,
            user_agent: DEFAULT_USER_AGENT.into(),
            extra_headers: Vec::new(),
            timeout: DEFAULT_TIMEOUT,
            fail_on_status_code: false,
            ignore_https_errors: false,
            cookie_store: None,
            authorization_provider: None,
            max_redirects: DEFAULT_MAX_REDIRECTS,
            limits: ApiRequestLimits::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApiRequestMetrics {
    pub disposed: bool,
    pub in_flight: usize,
    pub live_responses: usize,
    pub retained_body_bytes: usize,
    pub retained_log_bytes: usize,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum ApiRequestError {
    #[error("invalid API request argument: {0}")]
    InvalidArgument(String),
    #[error("API request context is disposed{0}")]
    Disposed(String),
    #[error("API request was cancelled")]
    Cancelled,
    #[error("Target page, context or browser has been closed")]
    TargetClosed,
    #[error("API request timed out after {0}ms")]
    Timeout(u64),
    #[error("API request transport failed: {0}")]
    Transport(String),
    #[error("API response body is disposed")]
    ResponseDisposed,
    #[error("API request capacity exceeded: {0}")]
    Capacity(String),
    #[error(
        "API request failed with status {status} {status_text} for {url}{response_text_suffix}"
    )]
    Status {
        status: u16,
        status_text: String,
        url: String,
        response_text_suffix: String,
    },
}

#[derive(Default)]
struct ResponseRegistry {
    entries: HashMap<u64, Arc<ResponseState>>,
    retained_body_bytes: usize,
    retained_log_bytes: usize,
}

struct RetainedResponse {
    body: Option<Vec<u8>>,
    log: Option<Vec<String>>,
    log_bytes: usize,
}

struct ResponseState {
    retained: Mutex<RetainedResponse>,
}

struct ApiRequestContextInner {
    transport: Arc<dyn ApiRequestTransport>,
    base_url: Option<String>,
    default_headers: Vec<ApiHeader>,
    default_timeout: Duration,
    default_fail_on_status_code: bool,
    default_ignore_https_errors: bool,
    cookie_store: Option<Arc<dyn ApiCookieStore>>,
    authorization_provider: Option<Arc<dyn ApiAuthorizationProvider>>,
    default_max_redirects: u16,
    limits: ApiRequestLimits,
    disposed: AtomicBool,
    disposal_reason: Mutex<Option<String>>,
    cancellation: CancellationToken,
    in_flight: Arc<Semaphore>,
    next_response_id: AtomicU64,
    next_multipart_boundary_id: AtomicU64,
    responses: Mutex<ResponseRegistry>,
}

#[derive(Clone)]
pub struct ApiRequestContext {
    inner: Arc<ApiRequestContextInner>,
}

impl ApiRequestContext {
    /// Create a request context over the selected transport.
    ///
    /// # Errors
    ///
    /// Returns [`ApiRequestError::InvalidArgument`] when a URL, header or
    /// resource limit in `options` is invalid.
    pub fn new(
        transport: Arc<dyn ApiRequestTransport>,
        options: ApiRequestContextOptions,
    ) -> Result<Self, ApiRequestError> {
        let limits = options.limits.validate()?;
        // Playwright retains even a malformed baseURL and only consults it
        // when a request URL is relative. Absolute request URLs must therefore
        // remain usable with a malformed configured base.
        let base_url = options.base_url;
        let mut default_headers = vec![
            ApiHeader::new("user-agent", options.user_agent),
            ApiHeader::new("accept", "*/*"),
            ApiHeader::new("accept-encoding", "gzip,deflate,br"),
        ];
        for header in options.extra_headers {
            set_header(&mut default_headers, header, false);
        }
        validate_headers(&default_headers, limits)?;
        Ok(Self {
            inner: Arc::new(ApiRequestContextInner {
                transport,
                base_url,
                default_headers,
                default_timeout: options.timeout,
                default_fail_on_status_code: options.fail_on_status_code,
                default_ignore_https_errors: options.ignore_https_errors,
                cookie_store: options.cookie_store,
                authorization_provider: options.authorization_provider,
                default_max_redirects: options.max_redirects,
                limits,
                disposed: AtomicBool::new(false),
                disposal_reason: Mutex::new(None),
                cancellation: CancellationToken::new(),
                in_flight: Arc::new(Semaphore::new(limits.max_in_flight)),
                next_response_id: AtomicU64::new(1),
                next_multipart_boundary_id: AtomicU64::new(1),
                responses: Mutex::new(ResponseRegistry::default()),
            }),
        })
    }

    /// Execute one request and retain its response in this context.
    ///
    /// # Errors
    ///
    /// Returns an [`ApiRequestError`] when validation, admission, transport,
    /// timeout, cancellation, status policy or response retention fails.
    pub async fn execute(&self, request: ApiRequest) -> Result<ApiResponse, ApiRequestError> {
        self.ensure_active()?;
        let prepared = self.prepare(request)?;
        self.ensure_active()?;
        let timeout = prepared.transport.timeout;
        let request_cancellation = prepared.transport.cancellation.clone();
        let operation = self.execute_prepared(prepared);
        tokio::pin!(operation);

        let result = if let Some(timeout) = timeout {
            let timeout_ms = duration_millis(timeout);
            tokio::select! {
                () = self.inner.cancellation.cancelled() => {
                    request_cancellation.cancel();
                    match tokio::time::timeout(CANCELLATION_SETTLEMENT_GRACE, &mut operation).await {
                        Ok(Err(ApiRequestError::TargetClosed)) => Err(ApiRequestError::TargetClosed),
                        _ => Err(self.disposed_error()),
                    }
                }
                () = tokio::time::sleep(timeout) => {
                    request_cancellation.cancel();
                    Err(ApiRequestError::Timeout(timeout_ms))
                }
                result = &mut operation => result,
            }
        } else {
            tokio::select! {
                () = self.inner.cancellation.cancelled() => {
                    request_cancellation.cancel();
                    match tokio::time::timeout(CANCELLATION_SETTLEMENT_GRACE, &mut operation).await {
                        Ok(Err(ApiRequestError::TargetClosed)) => Err(ApiRequestError::TargetClosed),
                        _ => Err(self.disposed_error()),
                    }
                }
                result = &mut operation => result,
            }
        };
        if matches!(result, Err(ApiRequestError::Cancelled))
            && self.inner.disposed.load(Ordering::Acquire)
        {
            Err(self.disposed_error())
        } else {
            result
        }
    }

    async fn execute_prepared(
        &self,
        prepared: PreparedRequest,
    ) -> Result<ApiResponse, ApiRequestError> {
        let permit = self
            .inner
            .in_flight
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| self.disposed_error())?;
        self.ensure_active()?;
        let response = self.inner.transport.execute(prepared.transport).await?;
        drop(permit);
        self.ensure_active()?;
        self.validate_response(&response)?;
        if prepared.fail_on_status_code && status_is_failure(response.status) {
            return Err(status_error(response));
        }
        self.register_response(response)
    }

    fn prepare(&self, request: ApiRequest) -> Result<PreparedRequest, ApiRequestError> {
        let ApiRequest {
            method,
            url: request_url,
            headers: request_headers,
            query,
            body,
            timeout,
            fail_on_status_code,
            ignore_https_errors,
            max_redirects,
            max_retries,
            lookup_addresses,
        } = request;
        let mut url = self.resolve_url(&request_url)?;
        apply_query(
            &mut url,
            query,
            self.inner.limits.max_query_parameters,
            self.inner.limits.max_url_bytes,
        )?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(ApiRequestError::InvalidArgument(
                "request URL must use HTTP or HTTPS".into(),
            ));
        }
        if method.len() > self.inner.limits.max_method_bytes {
            return Err(ApiRequestError::Capacity("request method bytes".into()));
        }
        if method.is_empty() || !method.bytes().all(is_http_token_byte) {
            return Err(ApiRequestError::InvalidArgument(
                "request method must be a non-empty HTTP token".into(),
            ));
        }
        validate_headers(&request_headers, self.inner.limits)?;
        let mut headers = self.inner.default_headers.clone();
        for header in request_headers {
            set_header(&mut headers, header, false);
        }
        let serialized = serialize_body(
            body,
            self.inner.limits.max_request_body_fields,
            self.inner.limits.max_request_body_bytes,
            &self.inner.next_multipart_boundary_id,
        )?;
        if let Some(content_type) = serialized.content_type {
            set_header(
                &mut headers,
                ApiHeader::new("content-type", content_type),
                true,
            );
        }
        if let Some(body) = serialized.bytes.as_ref() {
            set_header(
                &mut headers,
                ApiHeader::new("content-length", body.len().to_string()),
                false,
            );
        }
        validate_headers(&headers, self.inner.limits)?;
        if lookup_addresses
            .as_ref()
            .is_some_and(|addresses| addresses.is_empty())
        {
            return Err(ApiRequestError::InvalidArgument(
                "lookup address list must not be empty".into(),
            ));
        }
        if lookup_addresses
            .as_ref()
            .is_some_and(|addresses| addresses.len() > self.inner.limits.max_lookup_addresses)
        {
            return Err(ApiRequestError::Capacity("lookup address count".into()));
        }

        let timeout = timeout.unwrap_or(self.inner.default_timeout);
        Ok(PreparedRequest {
            transport: TransportRequest {
                method: method.to_ascii_uppercase(),
                url,
                headers,
                body: serialized.bytes,
                timeout: (!timeout.is_zero()).then_some(timeout),
                ignore_https_errors: ignore_https_errors
                    .unwrap_or(self.inner.default_ignore_https_errors),
                cookie_store: self.inner.cookie_store.clone(),
                authorization_provider: self.inner.authorization_provider.clone(),
                max_request_header_bytes: self.inner.limits.max_header_bytes,
                max_response_body_bytes: self.inner.limits.max_response_body_bytes,
                max_response_headers: self.inner.limits.max_headers,
                max_response_header_bytes: self.inner.limits.max_header_bytes,
                max_response_status_text_bytes: self.inner.limits.max_status_text_bytes,
                max_response_log_entries: self.inner.limits.max_log_entries,
                max_response_log_bytes: self.inner.limits.max_log_bytes,
                max_redirects: max_redirects.unwrap_or(self.inner.default_max_redirects),
                max_retries,
                lookup_addresses,
                cancellation: self.inner.cancellation.child_token(),
            },
            fail_on_status_code: fail_on_status_code
                .unwrap_or(self.inner.default_fail_on_status_code),
        })
    }

    fn resolve_url(&self, request_url: &str) -> Result<Url, ApiRequestError> {
        match Url::parse(request_url) {
            Ok(url) => Ok(url),
            Err(url::ParseError::RelativeUrlWithoutBase) => self
                .inner
                .base_url
                .as_ref()
                .ok_or_else(|| {
                    ApiRequestError::InvalidArgument(
                        "relative request URL requires a context base URL".into(),
                    )
                })
                .and_then(|base_url| {
                    Url::parse(base_url).map_err(|error| {
                        ApiRequestError::InvalidArgument(format!("invalid base URL: {error}"))
                    })
                })?
                .join(request_url)
                .map_err(|error| {
                    ApiRequestError::InvalidArgument(format!("invalid request URL: {error}"))
                }),
            Err(error) => Err(ApiRequestError::InvalidArgument(format!(
                "invalid request URL: {error}"
            ))),
        }
    }

    fn validate_response(&self, response: &TransportResponse) -> Result<(), ApiRequestError> {
        if response.url.len() > self.inner.limits.max_url_bytes {
            return Err(ApiRequestError::Capacity("response URL bytes".into()));
        }
        if response.status_text.len() > self.inner.limits.max_status_text_bytes {
            return Err(ApiRequestError::Capacity(
                "response status text bytes".into(),
            ));
        }
        validate_headers(&response.headers, self.inner.limits)?;
        if response.body.len() > self.inner.limits.max_response_body_bytes {
            return Err(ApiRequestError::Capacity("response body bytes".into()));
        }
        if response.log.len() > self.inner.limits.max_log_entries
            || log_bytes(&response.log)? > self.inner.limits.max_log_bytes
        {
            return Err(ApiRequestError::Capacity("response log".into()));
        }
        Ok(())
    }

    fn register_response(
        &self,
        response: TransportResponse,
    ) -> Result<ApiResponse, ApiRequestError> {
        let body_bytes = response.body.len();
        let response_log_bytes = log_bytes(&response.log)?;
        let mut registry = lock(&self.inner.responses);
        self.ensure_active()?;
        if registry.entries.len() >= self.inner.limits.max_live_responses {
            return Err(ApiRequestError::Capacity("live responses".into()));
        }
        let retained_body_bytes = registry
            .retained_body_bytes
            .checked_add(body_bytes)
            .ok_or_else(|| ApiRequestError::Capacity("retained response body bytes".into()))?;
        if retained_body_bytes > self.inner.limits.max_retained_body_bytes {
            return Err(ApiRequestError::Capacity(
                "retained response body bytes".into(),
            ));
        }
        let retained_log_bytes = registry
            .retained_log_bytes
            .checked_add(response_log_bytes)
            .ok_or_else(|| ApiRequestError::Capacity("retained response log bytes".into()))?;
        if retained_log_bytes > self.inner.limits.max_retained_log_bytes {
            return Err(ApiRequestError::Capacity(
                "retained response log bytes".into(),
            ));
        }

        let id = self
            .inner
            .next_response_id
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| ApiRequestError::Capacity("response identity space".into()))?;
        if registry.entries.contains_key(&id) {
            return Err(ApiRequestError::Capacity("response identity space".into()));
        }
        let state = Arc::new(ResponseState {
            retained: Mutex::new(RetainedResponse {
                body: Some(response.body),
                log: Some(response.log),
                log_bytes: response_log_bytes,
            }),
        });
        registry.retained_body_bytes = retained_body_bytes;
        registry.retained_log_bytes = retained_log_bytes;
        registry.entries.insert(id, state.clone());
        Ok(ApiResponse {
            context: Arc::downgrade(&self.inner),
            id,
            state,
            url: response.url,
            status: response.status,
            status_text: response.status_text,
            headers: response.headers,
        })
    }

    pub fn dispose(&self, reason: Option<String>) {
        let mut disposal_reason = lock(&self.inner.disposal_reason);
        if self.inner.disposed.swap(true, Ordering::AcqRel) {
            return;
        }
        *disposal_reason = reason;
        drop(disposal_reason);
        self.inner.cancellation.cancel();
        self.inner.in_flight.close();
        let mut registry = lock(&self.inner.responses);
        for state in registry.entries.values() {
            let mut retained = lock(&state.retained);
            retained.body.take();
            retained.log.take();
            retained.log_bytes = 0;
        }
        registry.entries.clear();
        registry.retained_body_bytes = 0;
        registry.retained_log_bytes = 0;
    }

    #[must_use]
    pub fn metrics(&self) -> ApiRequestMetrics {
        let registry = lock(&self.inner.responses);
        ApiRequestMetrics {
            disposed: self.inner.disposed.load(Ordering::Acquire),
            in_flight: self
                .inner
                .limits
                .max_in_flight
                .saturating_sub(self.inner.in_flight.available_permits()),
            live_responses: registry.entries.len(),
            retained_body_bytes: registry.retained_body_bytes,
            retained_log_bytes: registry.retained_log_bytes,
        }
    }

    /// Snapshot all unexpired cookies owned by the selected cookie store.
    ///
    /// # Errors
    ///
    /// Returns a disposal or cookie-store error.
    pub async fn storage_cookies(&self) -> Result<Vec<ApiCookie>, ApiRequestError> {
        self.ensure_active()?;
        let Some(store) = &self.inner.cookie_store else {
            return Ok(Vec::new());
        };
        let cookies = store.all_cookies().await.map_err(cookie_store_error)?;
        self.ensure_active()?;
        Ok(cookies)
    }

    /// Add structured storage-state or `BrowserContext` cookies.
    ///
    /// # Errors
    ///
    /// Returns a disposal, missing-capability or cookie-store error.
    pub async fn add_cookies(&self, cookies: Vec<ApiCookie>) -> Result<(), ApiRequestError> {
        self.ensure_active()?;
        let store = self.inner.cookie_store.as_ref().ok_or_else(|| {
            ApiRequestError::InvalidArgument("API request cookie store is not selected".into())
        })?;
        store
            .add_cookies(cookies)
            .await
            .map_err(cookie_store_error)?;
        self.ensure_active()
    }

    fn ensure_active(&self) -> Result<(), ApiRequestError> {
        if self.inner.disposed.load(Ordering::Acquire) {
            Err(self.disposed_error())
        } else {
            Ok(())
        }
    }

    fn disposed_error(&self) -> ApiRequestError {
        let reason = lock(&self.inner.disposal_reason)
            .as_ref()
            .map(|value| format!(": {value}"))
            .unwrap_or_default();
        ApiRequestError::Disposed(reason)
    }
}

struct PreparedRequest {
    transport: TransportRequest,
    fail_on_status_code: bool,
}

pub struct ApiResponse {
    context: Weak<ApiRequestContextInner>,
    id: u64,
    state: Arc<ResponseState>,
    url: String,
    status: u16,
    status_text: String,
    headers: Vec<ApiHeader>,
}

impl fmt::Debug for ApiResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApiResponse")
            .field("id", &self.id)
            .field("url", &self.url)
            .field("status", &self.status)
            .field("status_text", &self.status_text)
            .field("headers", &self.headers)
            .finish_non_exhaustive()
    }
}

impl ApiResponse {
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    #[must_use]
    pub fn status_text(&self) -> &str {
        &self.status_text
    }

    #[must_use]
    pub const fn ok(&self) -> bool {
        self.status >= 200 && self.status <= 299
    }

    #[must_use]
    pub fn headers(&self) -> &[ApiHeader] {
        &self.headers
    }

    /// Copy the retained response body.
    ///
    /// # Errors
    ///
    /// Returns [`ApiRequestError::ResponseDisposed`] after either this
    /// response or its owning context is disposed.
    pub fn body(&self) -> Result<Vec<u8>, ApiRequestError> {
        self.body_bounded(usize::MAX)
    }

    /// Copy the retained response body only when it fits the consumer's bound.
    ///
    /// # Errors
    ///
    /// Returns [`ApiRequestError::ResponseDisposed`] after either this
    /// response or its owning context is disposed, or
    /// [`ApiRequestError::Capacity`] without cloning when the body exceeds
    /// `max_bytes`.
    pub fn body_bounded(&self, max_bytes: usize) -> Result<Vec<u8>, ApiRequestError> {
        let retained = lock(&self.state.retained);
        let body = retained
            .body
            .as_ref()
            .ok_or(ApiRequestError::ResponseDisposed)?;
        if body.len() > max_bytes {
            return Err(ApiRequestError::Capacity(format!(
                "response body exceeds consumer byte limit ({max_bytes})"
            )));
        }
        Ok(body.clone())
    }

    /// Copy the retained transport log.
    ///
    /// # Errors
    ///
    /// Returns [`ApiRequestError::ResponseDisposed`] after either this
    /// response or its owning context is disposed.
    pub fn log(&self) -> Result<Vec<String>, ApiRequestError> {
        lock(&self.state.retained)
            .log
            .clone()
            .ok_or(ApiRequestError::ResponseDisposed)
    }

    pub fn dispose(&self) {
        let Some(context) = self.context.upgrade() else {
            let mut retained = lock(&self.state.retained);
            retained.body.take();
            retained.log.take();
            retained.log_bytes = 0;
            return;
        };
        let mut registry = lock(&context.responses);
        if registry.entries.remove(&self.id).is_none() {
            return;
        }
        let mut retained = lock(&self.state.retained);
        if let Some(body) = retained.body.take() {
            registry.retained_body_bytes = registry.retained_body_bytes.saturating_sub(body.len());
        }
        retained.log.take();
        registry.retained_log_bytes = registry
            .retained_log_bytes
            .saturating_sub(retained.log_bytes);
        retained.log_bytes = 0;
    }
}

impl Drop for ApiResponse {
    fn drop(&mut self) {
        self.dispose();
    }
}

fn set_header(headers: &mut Vec<ApiHeader>, header: ApiHeader, keep_existing: bool) {
    if let Some(existing) = headers
        .iter_mut()
        .find(|existing| existing.name.eq_ignore_ascii_case(&header.name))
    {
        if !keep_existing {
            existing.value = header.value;
        }
    } else {
        headers.push(header);
    }
}

fn validate_headers(
    headers: &[ApiHeader],
    limits: ApiRequestLimits,
) -> Result<(), ApiRequestError> {
    if headers.len() > limits.max_headers {
        return Err(ApiRequestError::Capacity("header count".into()));
    }
    let mut bytes = 0usize;
    for header in headers {
        if header.name.is_empty()
            || !header.name.bytes().all(is_http_token_byte)
            || !header.value.chars().all(is_http_header_value_char)
        {
            return Err(ApiRequestError::InvalidArgument(
                "invalid HTTP header name or value".into(),
            ));
        }
        bytes = bytes
            .checked_add(header.name.len())
            .and_then(|value| value.checked_add(header.value.len()))
            .ok_or_else(|| ApiRequestError::Capacity("header bytes".into()))?;
    }
    if bytes > limits.max_header_bytes {
        return Err(ApiRequestError::Capacity("header bytes".into()));
    }
    Ok(())
}

fn log_bytes(log: &[String]) -> Result<usize, ApiRequestError> {
    log.iter().try_fold(0usize, |total, value| {
        total
            .checked_add(value.len())
            .ok_or_else(|| ApiRequestError::Capacity("response log bytes".into()))
    })
}

fn is_http_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

const fn is_http_header_value_char(character: char) -> bool {
    character == '\t'
        || (character >= ' ' && character <= '~')
        || (character >= '\u{80}' && character <= '\u{ff}')
}

const fn status_is_failure(status: u16) -> bool {
    status < 200 || status >= 400
}

fn status_error(response: TransportResponse) -> ApiRequestError {
    let response_text_suffix = if response.body.is_empty() {
        String::new()
    } else {
        let preview_bytes = &response.body[..response.body.len().min(4_000)];
        let text = String::from_utf8_lossy(preview_bytes);
        let mut characters = text.chars();
        let preview: String = characters.by_ref().take(997).collect();
        let preview = if characters.next().is_some() {
            format!("{preview}...")
        } else {
            text.into_owned()
        };
        format!("\nResponse text:\n{preview}")
    };
    ApiRequestError::Status {
        status: response.status,
        status_text: response.status_text,
        url: response.url,
        response_text_suffix,
    }
}

fn cookie_store_error(error: ApiCookieStoreError) -> ApiRequestError {
    let ApiCookieStoreError { message } = error;
    ApiRequestError::Transport(format!("cookie store failed: {message}"))
}

fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        future::pending,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use async_trait::async_trait;
    use tokio::sync::Notify;

    use super::{
        ApiAuthorizationProvider, ApiCookie, ApiCookieStore, ApiCookieStoreError, ApiHeader,
        ApiNameValue, ApiQuery, ApiRequest, ApiRequestBody, ApiRequestContext,
        ApiRequestContextOptions, ApiRequestError, ApiRequestLimits, ApiRequestTransport,
        ApiSameSite, TransportRequest, TransportResponse, lock,
    };

    struct ScriptedTransport {
        requests: Mutex<Vec<TransportRequest>>,
        responses: Mutex<VecDeque<Result<TransportResponse, ApiRequestError>>>,
    }

    impl ScriptedTransport {
        fn new(responses: impl IntoIterator<Item = TransportResponse>) -> Self {
            Self {
                requests: Mutex::new(Vec::new()),
                responses: Mutex::new(responses.into_iter().map(Ok).collect()),
            }
        }

        fn requests(&self) -> Vec<TransportRequest> {
            lock(&self.requests).clone()
        }
    }

    #[async_trait]
    impl ApiRequestTransport for ScriptedTransport {
        async fn execute(
            &self,
            request: TransportRequest,
        ) -> Result<TransportResponse, ApiRequestError> {
            lock(&self.requests).push(request);
            lock(&self.responses)
                .pop_front()
                .expect("test response script exhausted")
        }
    }

    struct PendingTransport {
        started: Notify,
        request: Mutex<Option<TransportRequest>>,
    }

    impl PendingTransport {
        fn new() -> Self {
            Self {
                started: Notify::new(),
                request: Mutex::new(None),
            }
        }
    }

    #[derive(Debug)]
    struct RecordingCookieStore {
        cookies: Mutex<Vec<ApiCookie>>,
    }

    impl RecordingCookieStore {
        fn new(cookies: Vec<ApiCookie>) -> Self {
            Self {
                cookies: Mutex::new(cookies),
            }
        }

        fn cookies(&self) -> Vec<ApiCookie> {
            lock(&self.cookies).clone()
        }
    }

    #[async_trait]
    impl ApiCookieStore for RecordingCookieStore {
        async fn request_header(
            &self,
            _url: &url::Url,
        ) -> Result<Option<String>, ApiCookieStoreError> {
            Ok(None)
        }

        async fn store_response_headers(
            &self,
            _url: &url::Url,
            _values: &[String],
        ) -> Result<(), ApiCookieStoreError> {
            Ok(())
        }

        async fn all_cookies(&self) -> Result<Vec<ApiCookie>, ApiCookieStoreError> {
            Ok(self.cookies())
        }

        async fn add_cookies(&self, cookies: Vec<ApiCookie>) -> Result<(), ApiCookieStoreError> {
            lock(&self.cookies).extend(cookies);
            Ok(())
        }
    }

    #[derive(Debug)]
    struct NoopAuthorizationProvider;

    impl ApiAuthorizationProvider for NoopAuthorizationProvider {
        fn preemptive_header(&self, _url: &url::Url) -> Option<String> {
            None
        }

        fn challenge_header(&self, _url: &url::Url, _www_authenticate: &str) -> Option<String> {
            None
        }
    }

    #[async_trait]
    impl ApiRequestTransport for PendingTransport {
        async fn execute(
            &self,
            request: TransportRequest,
        ) -> Result<TransportResponse, ApiRequestError> {
            *lock(&self.request) = Some(request);
            self.started.notify_one();
            pending().await
        }
    }

    fn response(status: u16, body: impl Into<Vec<u8>>) -> TransportResponse {
        TransportResponse {
            url: "https://example.test/result".into(),
            status,
            status_text: if status == 404 {
                "Not Found".into()
            } else {
                "OK".into()
            },
            headers: vec![ApiHeader::new("content-type", "text/plain")],
            body: body.into(),
            log: vec!["request sent".into()],
        }
    }

    fn context(
        transport: Arc<dyn ApiRequestTransport>,
        options: ApiRequestContextOptions,
    ) -> ApiRequestContext {
        ApiRequestContext::new(transport, options).expect("test context should be valid")
    }

    fn cookie(name: &str, value: &str) -> ApiCookie {
        ApiCookie {
            name: name.into(),
            value: value.into(),
            domain: "example.test".into(),
            path: "/".into(),
            expires: -1.0,
            http_only: false,
            secure: false,
            same_site: ApiSameSite::Lax,
        }
    }

    #[tokio::test]
    async fn add_cookies_delegates_to_selected_store() {
        // Arrange
        let store = Arc::new(RecordingCookieStore::new(Vec::new()));
        let context = context(
            Arc::new(ScriptedTransport::new([])),
            ApiRequestContextOptions {
                cookie_store: Some(store.clone()),
                ..ApiRequestContextOptions::default()
            },
        );

        // Act
        context
            .add_cookies(vec![cookie("a", "v")])
            .await
            .expect("cookie should be delegated");

        // Assert
        assert_eq!(store.cookies(), [cookie("a", "v")]);
    }

    #[tokio::test]
    async fn storage_cookies_reads_selected_store() {
        // Arrange
        let store = Arc::new(RecordingCookieStore::new(vec![cookie("a", "v")]));
        let context = context(
            Arc::new(ScriptedTransport::new([])),
            ApiRequestContextOptions {
                cookie_store: Some(store),
                ..ApiRequestContextOptions::default()
            },
        );

        // Act
        let cookies = context
            .storage_cookies()
            .await
            .expect("cookies should be exported");

        // Assert
        assert_eq!(cookies, [cookie("a", "v")]);
    }

    #[tokio::test]
    async fn add_cookies_requires_an_explicit_cookie_capability() {
        // Arrange
        let context = context(
            Arc::new(ScriptedTransport::new([])),
            ApiRequestContextOptions::default(),
        );

        // Act
        let result = context.add_cookies(vec![cookie("a", "v")]).await;

        // Assert
        assert_eq!(
            result.expect_err("missing cookie store should fail"),
            ApiRequestError::InvalidArgument("API request cookie store is not selected".into())
        );
    }

    #[tokio::test]
    async fn successful_response_is_bounded_and_dispose_is_idempotent() {
        // Arrange
        let transport = Arc::new(ScriptedTransport::new([response(200, b"hello".to_vec())]));
        let context = context(transport, ApiRequestContextOptions::default());

        // Act
        let response = context
            .execute(ApiRequest::new("get", "https://example.test/data"))
            .await
            .expect("request should succeed");

        // Assert
        assert_eq!(response.status(), 200);
        assert!(response.ok());
        assert_eq!(response.body().expect("body should be live"), b"hello");
        assert_eq!(
            response.log().expect("log should be live"),
            ["request sent"]
        );
        assert_eq!(context.metrics().live_responses, 1);
        assert_eq!(context.metrics().retained_body_bytes, 5);
        assert_eq!(context.metrics().retained_log_bytes, 12);
        response.dispose();
        response.dispose();
        assert_eq!(response.body(), Err(ApiRequestError::ResponseDisposed));
        assert_eq!(context.metrics().live_responses, 0);
        assert_eq!(context.metrics().retained_body_bytes, 0);
        assert_eq!(context.metrics().retained_log_bytes, 0);
    }

    #[tokio::test]
    async fn bounded_body_copy_rejects_before_clone_and_preserves_the_response() {
        // Arrange
        let transport = Arc::new(ScriptedTransport::new([response(200, b"hello".to_vec())]));
        let context = context(transport, ApiRequestContextOptions::default());
        let response = context
            .execute(ApiRequest::new("GET", "https://example.test/data"))
            .await
            .expect("request should succeed");

        // Act
        let result = response.body_bounded(4);

        // Assert
        assert_eq!(
            result.expect_err("the consumer bound should reject five bytes"),
            ApiRequestError::Capacity("response body exceeds consumer byte limit (4)".into())
        );
        assert_eq!(response.body_bounded(5).unwrap(), b"hello");
        assert_eq!(context.metrics().retained_body_bytes, 5);
    }

    #[tokio::test]
    async fn request_uses_context_defaults_and_resolves_relative_url() {
        // Arrange
        let transport = Arc::new(ScriptedTransport::new([response(200, Vec::new())]));
        let options = ApiRequestContextOptions {
            base_url: Some("https://example.test/root/".into()),
            user_agent: "Test Agent".into(),
            extra_headers: vec![ApiHeader::new("My-Secret", "Value")],
            timeout: Duration::ZERO,
            fail_on_status_code: true,
            ignore_https_errors: true,
            cookie_store: None,
            authorization_provider: Some(Arc::new(NoopAuthorizationProvider)),
            max_redirects: 7,
            limits: ApiRequestLimits::default(),
        };
        let context = context(transport.clone(), options);

        // Act
        let response = context
            .execute(ApiRequest::new("custom-1", "child?value=1"))
            .await
            .expect("request should succeed");

        // Assert
        let requests = transport.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "CUSTOM-1");
        assert_eq!(
            requests[0].url.as_str(),
            "https://example.test/root/child?value=1"
        );
        assert_eq!(requests[0].timeout, None);
        assert!(requests[0].ignore_https_errors);
        assert!(requests[0].authorization_provider.is_some());
        assert_eq!(requests[0].max_redirects, 7);
        assert_eq!(
            requests[0].headers,
            [
                ApiHeader::new("user-agent", "Test Agent"),
                ApiHeader::new("accept", "*/*"),
                ApiHeader::new("accept-encoding", "gzip,deflate,br"),
                ApiHeader::new("My-Secret", "Value"),
            ]
        );
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn malformed_base_url_does_not_reject_an_absolute_request() {
        // Arrange
        let transport = Arc::new(ScriptedTransport::new([response(200, Vec::new())]));
        let context = context(
            transport.clone(),
            ApiRequestContextOptions {
                base_url: Some("bogus".into()),
                ..ApiRequestContextOptions::default()
            },
        );

        // Act
        let response = context
            .execute(ApiRequest::new("GET", "https://example.test/absolute"))
            .await
            .expect("absolute request should ignore malformed base URL");

        // Assert
        assert_eq!(response.status(), 200);
        assert_eq!(
            transport.requests()[0].url.as_str(),
            "https://example.test/absolute"
        );
    }

    #[tokio::test]
    async fn invalid_header_is_rejected_before_transport() {
        // Arrange
        let transport = Arc::new(ScriptedTransport::new([]));
        let context = context(transport.clone(), ApiRequestContextOptions::default());
        let mut request = ApiRequest::new("GET", "https://example.test/");
        request
            .headers
            .push(ApiHeader::new("x-test", "bad\r\nvalue"));

        // Act
        let result = context.execute(request).await;

        // Assert
        assert!(matches!(result, Err(ApiRequestError::InvalidArgument(_))));
        assert!(transport.requests().is_empty());
    }

    #[tokio::test]
    async fn non_latin1_header_is_rejected_before_transport() {
        // Arrange
        let transport = Arc::new(ScriptedTransport::new([]));
        let context = context(transport.clone(), ApiRequestContextOptions::default());
        let mut request = ApiRequest::new("GET", "https://example.test/");
        request
            .headers
            .push(ApiHeader::new("x-test", "emoji \u{1f642}"));

        // Act
        let result = context.execute(request).await;

        // Assert
        assert_eq!(
            result.expect_err("non-Latin-1 header should fail"),
            ApiRequestError::InvalidArgument("invalid HTTP header name or value".into())
        );
        assert!(transport.requests().is_empty());
    }

    #[tokio::test]
    async fn request_preparation_serializes_query_body_and_header_overrides() {
        // Arrange
        let transport = Arc::new(ScriptedTransport::new([response(200, Vec::new())]));
        let options = ApiRequestContextOptions {
            user_agent: "Context Agent".into(),
            extra_headers: vec![ApiHeader::new("X-Context", "context")],
            ..ApiRequestContextOptions::default()
        };
        let context = context(transport.clone(), options);
        let mut request = ApiRequest::new("POST", "https://example.test/path?existing=yes");
        request.query = ApiQuery::Pairs(vec![
            ApiNameValue::new("name", "John Doe"),
            ApiNameValue::new("name", "Jane"),
        ]);
        request.body = ApiRequestBody::Json(r#"{"ok":true}"#.into());
        request.headers = vec![
            ApiHeader::new("USER-AGENT", "Request Agent"),
            ApiHeader::new("Accept", "text/html"),
            ApiHeader::new("Content-Type", "unknown"),
            ApiHeader::new("Content-Length", "wrong"),
            ApiHeader::new("x-context", "request"),
        ];

        // Act
        let response = context
            .execute(request)
            .await
            .expect("prepared request should succeed");

        // Assert
        let requests = transport.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].url.as_str(),
            "https://example.test/path?existing=yes&name=John+Doe&name=Jane"
        );
        assert_eq!(
            requests[0].body.as_deref(),
            Some(br#"{"ok":true}"#.as_slice())
        );
        assert_eq!(
            requests[0].headers,
            [
                ApiHeader::new("user-agent", "Request Agent"),
                ApiHeader::new("accept", "text/html"),
                ApiHeader::new("accept-encoding", "gzip,deflate,br"),
                ApiHeader::new("X-Context", "request"),
                ApiHeader::new("Content-Type", "unknown"),
                ApiHeader::new("Content-Length", "11"),
            ]
        );
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn fail_on_status_accepts_redirect_and_rejects_client_error_without_retention() {
        // Arrange
        let transport = Arc::new(ScriptedTransport::new([
            response(302, Vec::new()),
            response(404, b"missing".to_vec()),
        ]));
        let options = ApiRequestContextOptions {
            fail_on_status_code: true,
            ..ApiRequestContextOptions::default()
        };
        let context = context(transport, options);

        // Act
        let redirect = context
            .execute(ApiRequest::new("GET", "https://example.test/redirect"))
            .await
            .expect("3xx should not fail failOnStatusCode");
        let failure = context
            .execute(ApiRequest::new("GET", "https://example.test/missing"))
            .await;

        // Assert
        assert_eq!(redirect.status(), 302);
        let error = failure.expect_err("404 should fail failOnStatusCode");
        assert!(error.to_string().contains("404 Not Found"));
        assert!(error.to_string().contains("Response text:\nmissing"));
        assert_eq!(context.metrics().live_responses, 1);
        assert_eq!(context.metrics().retained_body_bytes, 0);
    }

    #[tokio::test]
    async fn timeout_cancels_transport_request() {
        // Arrange
        let transport = Arc::new(PendingTransport::new());
        let context = context(transport.clone(), ApiRequestContextOptions::default());
        let mut request = ApiRequest::new("GET", "https://example.test/slow");
        request.timeout = Some(Duration::from_millis(5));

        // Act
        let result = context.execute(request).await;

        // Assert
        assert_eq!(
            result.expect_err("request should time out"),
            ApiRequestError::Timeout(5)
        );
        let request = lock(&transport.request)
            .clone()
            .expect("transport should receive request");
        assert!(request.cancellation.is_cancelled());
        assert_eq!(context.metrics().in_flight, 0);
    }

    #[tokio::test]
    async fn zero_timeout_allows_request_to_finish() {
        struct DelayedTransport;

        #[async_trait]
        impl ApiRequestTransport for DelayedTransport {
            async fn execute(
                &self,
                _request: TransportRequest,
            ) -> Result<TransportResponse, ApiRequestError> {
                tokio::time::sleep(Duration::from_millis(5)).await;
                Ok(response(200, b"done".to_vec()))
            }
        }

        // Arrange
        let context = context(
            Arc::new(DelayedTransport),
            ApiRequestContextOptions::default(),
        );
        let mut request = ApiRequest::new("GET", "https://example.test/slow");
        request.timeout = Some(Duration::ZERO);

        // Act
        let response = context
            .execute(request)
            .await
            .expect("zero timeout should be disabled");

        // Assert
        assert_eq!(response.body().expect("body should be live"), b"done");
    }

    #[tokio::test]
    async fn context_dispose_cancels_in_flight_and_invalidates_retained_body() {
        // Arrange
        let completed = Arc::new(ScriptedTransport::new([response(200, b"saved".to_vec())]));
        let completed_context = context(completed, ApiRequestContextOptions::default());
        let retained = completed_context
            .execute(ApiRequest::new("GET", "https://example.test/saved"))
            .await
            .expect("first request should succeed");
        let pending = Arc::new(PendingTransport::new());
        let pending_context = context(pending.clone(), ApiRequestContextOptions::default());
        let task_context = pending_context.clone();
        let task = tokio::spawn(async move {
            task_context
                .execute(ApiRequest::new("GET", "https://example.test/pending"))
                .await
        });
        pending.started.notified().await;

        // Act
        completed_context.dispose(Some("test shutdown".into()));
        pending_context.dispose(Some("test shutdown".into()));
        let pending_result = task.await.expect("request task should join");

        // Assert
        assert_eq!(retained.body(), Err(ApiRequestError::ResponseDisposed));
        assert_eq!(completed_context.metrics().live_responses, 0);
        assert_eq!(
            pending_result.expect_err("in-flight request should be cancelled"),
            ApiRequestError::Disposed(": test shutdown".into())
        );
        let later = pending_context
            .execute(ApiRequest::new("GET", "https://example.test/later"))
            .await;
        assert_eq!(
            later.expect_err("disposed context should reject new requests"),
            ApiRequestError::Disposed(": test shutdown".into())
        );
    }

    #[tokio::test]
    async fn aggregate_body_capacity_rejects_unretained_response() {
        // Arrange
        let transport = Arc::new(ScriptedTransport::new([response(200, b"large".to_vec())]));
        let limits = ApiRequestLimits {
            max_retained_body_bytes: 4,
            ..ApiRequestLimits::default()
        };
        let context = context(
            transport,
            ApiRequestContextOptions {
                limits,
                ..ApiRequestContextOptions::default()
            },
        );

        // Act
        let result = context
            .execute(ApiRequest::new("GET", "https://example.test/data"))
            .await;

        // Assert
        assert_eq!(
            result.expect_err("aggregate cap should reject response"),
            ApiRequestError::Capacity("retained response body bytes".into())
        );
        assert_eq!(context.metrics().live_responses, 0);
        assert_eq!(context.metrics().retained_body_bytes, 0);
    }

    #[tokio::test]
    async fn in_flight_limit_serializes_concurrent_transport_calls() {
        struct CountingTransport {
            active: AtomicUsize,
            maximum: AtomicUsize,
        }

        #[async_trait]
        impl ApiRequestTransport for CountingTransport {
            async fn execute(
                &self,
                _request: TransportRequest,
            ) -> Result<TransportResponse, ApiRequestError> {
                let active = self.active.fetch_add(1, Ordering::AcqRel) + 1;
                self.maximum.fetch_max(active, Ordering::AcqRel);
                tokio::time::sleep(Duration::from_millis(2)).await;
                self.active.fetch_sub(1, Ordering::AcqRel);
                Ok(response(200, Vec::new()))
            }
        }

        // Arrange
        let transport = Arc::new(CountingTransport {
            active: AtomicUsize::new(0),
            maximum: AtomicUsize::new(0),
        });
        let limits = ApiRequestLimits {
            max_in_flight: 1,
            ..ApiRequestLimits::default()
        };
        let context = context(
            transport.clone(),
            ApiRequestContextOptions {
                limits,
                ..ApiRequestContextOptions::default()
            },
        );

        // Act
        let (first, second, third) = tokio::join!(
            context.execute(ApiRequest::new("GET", "https://example.test/1")),
            context.execute(ApiRequest::new("GET", "https://example.test/2")),
            context.execute(ApiRequest::new("GET", "https://example.test/3")),
        );

        // Assert
        assert!(first.is_ok());
        assert!(second.is_ok());
        assert!(third.is_ok());
        assert_eq!(transport.maximum.load(Ordering::Acquire), 1);
        assert_eq!(context.metrics().in_flight, 0);
    }

    #[tokio::test]
    async fn disposed_context_rejects_before_request_validation() {
        // Arrange
        let transport = Arc::new(ScriptedTransport::new([]));
        let context = context(transport.clone(), ApiRequestContextOptions::default());
        context.dispose(Some("already closed".into()));

        // Act
        let result = context.execute(ApiRequest::new("", "not a URL")).await;

        // Assert
        assert_eq!(
            result.expect_err("disposed context should reject first"),
            ApiRequestError::Disposed(": already closed".into())
        );
        assert!(transport.requests().is_empty());
    }

    #[tokio::test]
    async fn response_identity_exhaustion_never_wraps_or_registers() {
        // Arrange
        let transport = Arc::new(ScriptedTransport::new([
            response(200, Vec::new()),
            response(200, Vec::new()),
        ]));
        let context = context(transport, ApiRequestContextOptions::default());
        context
            .inner
            .next_response_id
            .store(u64::MAX, Ordering::Release);

        // Act
        let first = context
            .execute(ApiRequest::new("GET", "https://example.test/1"))
            .await;
        let second = context
            .execute(ApiRequest::new("GET", "https://example.test/2"))
            .await;

        // Assert
        assert_eq!(
            first.expect_err("identity space should remain exhausted"),
            ApiRequestError::Capacity("response identity space".into())
        );
        assert_eq!(
            second.expect_err("identity must not wrap to zero or one"),
            ApiRequestError::Capacity("response identity space".into())
        );
        assert_eq!(context.metrics().live_responses, 0);
    }
}
