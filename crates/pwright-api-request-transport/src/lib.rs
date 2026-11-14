//! Connection-agnostic transport policy for Playwright-compatible API requests.
//!
//! The transport owns one-exchange connections, redirect semantics and
//! reset-only retries. DNS, sockets, TLS, proxies and decompression remain
//! injected capabilities so distributions can select them independently.

use std::{net::IpAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
use pwright_api_request::{
    ApiHeader, ApiRequestError, ApiRequestTransport, TransportRequest, TransportResponse,
};
use pwright_api_request_http1::{
    Http1CodecError, Http1Response, Http1ResponseDecoder, Http1ResponseEvent, Http1ResponseHead,
    encode_request,
};
use tokio_util::sync::CancellationToken;
use url::Url;

const READ_BUFFER_BYTES: usize = 32 * 1024;
const INITIAL_RESET_BACKOFF: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiConnectionErrorKind {
    Cancelled,
    Reset,
    Dns,
    Refused,
    Tls,
    Proxy,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{message}")]
pub struct ApiConnectionError {
    pub kind: ApiConnectionErrorKind,
    pub message: String,
}

impl ApiConnectionError {
    #[must_use]
    pub fn new(kind: ApiConnectionErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// One connection owned by exactly one HTTP exchange.
#[async_trait]
pub trait ApiConnection: Send {
    /// Write the complete encoded request.
    ///
    /// # Errors
    ///
    /// Returns a classified connection error. Implementations must not retain
    /// `bytes` or spawn a detached write after returning.
    async fn write_all(
        &mut self,
        bytes: &[u8],
        cancellation: &CancellationToken,
    ) -> Result<(), ApiConnectionError>;

    /// Read up to `output.len()` bytes, returning zero only for EOF.
    ///
    /// # Errors
    ///
    /// Returns a classified connection error. Implementations must observe
    /// cancellation without leaving a detached read.
    async fn read(
        &mut self,
        output: &mut [u8],
        cancellation: &CancellationToken,
    ) -> Result<usize, ApiConnectionError>;
}

/// Selectable DNS/socket/TLS/proxy connector.
#[derive(Clone, Copy, Debug)]
pub struct ApiConnectionTarget<'a> {
    pub url: &'a Url,
    pub ignore_https_errors: bool,
    pub resolved_addresses: Option<&'a [IpAddr]>,
}

#[async_trait]
pub trait ApiConnector: Send + Sync + 'static {
    /// Connect to the requested origin with request-scoped TLS policy.
    ///
    /// # Errors
    ///
    /// Returns a classified connection error. A successful connection belongs
    /// only to the returned request exchange.
    async fn connect(
        &self,
        target: ApiConnectionTarget<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Box<dyn ApiConnection>, ApiConnectionError>;
}

/// Optional TLS composition over an already-established proxy tunnel.
///
/// Proxy connectors own only tunnel negotiation. A selected TLS capability
/// implements this port so HTTP CONNECT and SOCKS leaves do not depend on a
/// concrete TLS stack or duplicate certificate policy.
#[async_trait]
pub trait ApiTunnelTlsUpgrader: Send + Sync + 'static {
    /// Complete target-origin TLS over `connection`.
    ///
    /// `target.resolved_addresses` is intentionally irrelevant: the proxy has
    /// already resolved or connected the target. Implementations must use the
    /// target URL for SNI, certificate selection and request-scoped verification
    /// policy, and must not open another socket.
    ///
    /// # Errors
    ///
    /// Returns a classified TLS, connection or cancellation error.
    async fn upgrade(
        &self,
        connection: Box<dyn ApiConnection>,
        target: ApiConnectionTarget<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Box<dyn ApiConnection>, ApiConnectionError>;
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{message}")]
pub struct ApiBodyDecodeError {
    pub message: String,
}

impl ApiBodyDecodeError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub trait ApiResponseBodyDecoder: Send + Sync + 'static {
    /// Decode one final response body under the public decoded-body cap.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected content encoding is invalid or its
    /// decoded output would exceed `max_output_bytes`.
    fn decode(
        &self,
        content_encoding: Option<&str>,
        body: Vec<u8>,
        max_output_bytes: usize,
    ) -> Result<Vec<u8>, ApiBodyDecodeError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IdentityBodyDecoder;

impl ApiResponseBodyDecoder for IdentityBodyDecoder {
    fn decode(
        &self,
        _content_encoding: Option<&str>,
        body: Vec<u8>,
        _max_output_bytes: usize,
    ) -> Result<Vec<u8>, ApiBodyDecodeError> {
        Ok(body)
    }
}

pub struct ConnectionApiRequestTransport {
    connector: Arc<dyn ApiConnector>,
    body_decoder: Arc<dyn ApiResponseBodyDecoder>,
}

impl ConnectionApiRequestTransport {
    #[must_use]
    pub fn new(connector: Arc<dyn ApiConnector>) -> Self {
        Self {
            connector,
            body_decoder: Arc::new(IdentityBodyDecoder),
        }
    }

    #[must_use]
    pub fn with_body_decoder(
        connector: Arc<dyn ApiConnector>,
        body_decoder: Arc<dyn ApiResponseBodyDecoder>,
    ) -> Self {
        Self {
            connector,
            body_decoder,
        }
    }

    async fn execute_with_retries(
        &self,
        request: TransportRequest,
    ) -> Result<TransportResponse, ApiRequestError> {
        let max_retries = u32::from(request.max_retries);
        let mut backoff = INITIAL_RESET_BACKOFF;
        let mut log = BoundedLog::new(
            request.max_response_log_entries,
            request.max_response_log_bytes,
        );

        for attempt in 0..=max_retries {
            match self.execute_redirect_chain(request.clone(), &mut log).await {
                Ok(response) => {
                    return Ok(TransportResponse {
                        url: response.url,
                        status: response.response.status,
                        status_text: response.response.status_text,
                        headers: response.response.headers,
                        body: response.response.body,
                        log: log.entries,
                    });
                }
                Err(AttemptError::Connection(error)) => {
                    if error.kind == ApiConnectionErrorKind::Cancelled {
                        return Err(ApiRequestError::Cancelled);
                    }
                    if max_retries == 0 {
                        return Err(connection_error(&error));
                    }
                    if attempt == max_retries {
                        return Err(ApiRequestError::Transport(format!(
                            "Failed after {} attempt(s): {error}",
                            attempt + 1
                        )));
                    }
                    if error.kind != ApiConnectionErrorKind::Reset {
                        return Err(connection_error(&error));
                    }
                    log.push(format!(
                        "  Received ECONNRESET, will retry after {}ms.",
                        backoff.as_millis()
                    ));
                    wait_for_backoff(backoff, &request.cancellation).await?;
                    backoff = backoff.saturating_mul(2);
                }
                Err(AttemptError::Codec(error)) => {
                    return Err(ApiRequestError::Transport(error.to_string()));
                }
                Err(AttemptError::Policy(message)) => {
                    return Err(ApiRequestError::Transport(message));
                }
                Err(AttemptError::Cancelled) => return Err(ApiRequestError::Cancelled),
                Err(AttemptError::RedirectCancelled) => {
                    return Err(ApiRequestError::TargetClosed);
                }
            }
        }
        Err(ApiRequestError::Transport("unreachable retry state".into()))
    }

    async fn execute_redirect_chain(
        &self,
        mut request: TransportRequest,
        log: &mut BoundedLog,
    ) -> Result<CompletedExchange, AttemptError> {
        let mut redirect_budget = if request.max_redirects == 0 {
            RedirectBudget::Disabled
        } else {
            RedirectBudget::Remaining(request.max_redirects)
        };
        let mut followed_redirect = false;
        if !has_header(&request.headers, "authorization")
            && let Some(provider) = &request.authorization_provider
            && let Some(value) = provider.preemptive_header(&request.url)
        {
            request.headers.push(ApiHeader::new("authorization", value));
        }

        loop {
            ensure_active(&request.cancellation)
                .map_err(|error| classify_redirect_cancellation(error, followed_redirect))?;
            let pending = self
                .start_exchange(&request, log)
                .await
                .map_err(|error| classify_redirect_cancellation(error, followed_redirect))?;
            if pending.head.status == 401
                && !has_header(&request.headers, "authorization")
                && let Some(challenge) = header_value(&pending.head.headers, "www-authenticate")
                && let Some(provider) = &request.authorization_provider
                && let Some(value) = provider.challenge_header(&request.url, &challenge)
            {
                request.headers.push(ApiHeader::new("authorization", value));
                continue;
            }
            if !is_redirect(pending.head.status) {
                return self
                    .finish_and_complete(&request, pending)
                    .await
                    .map_err(|error| classify_redirect_cancellation(error, followed_redirect));
            }
            let Some(location) = header_value(&pending.head.headers, "location") else {
                return self
                    .finish_and_complete(&request, pending)
                    .await
                    .map_err(|error| classify_redirect_cancellation(error, followed_redirect));
            };
            if location.is_empty() || matches!(redirect_budget, RedirectBudget::Disabled) {
                return self
                    .finish_and_complete(&request, pending)
                    .await
                    .map_err(|error| classify_redirect_cancellation(error, followed_redirect));
            }
            match redirect_budget {
                RedirectBudget::Disabled => unreachable!("disabled redirects returned above"),
                RedirectBudget::Remaining(0) => {
                    return Err(AttemptError::Policy("Max redirect count exceeded".into()));
                }
                RedirectBudget::Remaining(remaining) => {
                    redirect_budget = RedirectBudget::Remaining(remaining - 1);
                }
            }

            let location = latin1_location_to_utf8(&location);
            let next_url = request.url.join(&location).map_err(|_| {
                AttemptError::Policy(format!(
                    "uri requested responds with an invalid redirect URL: {location}"
                ))
            })?;
            remove_header(&mut request.headers, "cookie");
            if should_rewrite_to_get(pending.head.status, &request.method) {
                request.method = "GET".into();
                request.body = None;
                for name in [
                    "content-encoding",
                    "content-language",
                    "content-length",
                    "content-location",
                    "content-type",
                ] {
                    remove_header(&mut request.headers, name);
                }
            }
            if let Some(host) = request
                .headers
                .iter_mut()
                .find(|header| header.name.eq_ignore_ascii_case("host"))
            {
                host.value = host_value(&next_url)?;
            }
            request.url = next_url;
            followed_redirect = true;
        }
    }

    async fn finish_and_complete(
        &self,
        request: &TransportRequest,
        pending: PendingExchange,
    ) -> Result<CompletedExchange, AttemptError> {
        let response = Self::finish_exchange(request, pending).await?;
        self.complete_response(request, response)
    }

    fn complete_response(
        &self,
        request: &TransportRequest,
        mut response: Http1Response,
    ) -> Result<CompletedExchange, AttemptError> {
        let content_encoding = header_value(&response.headers, "content-encoding");
        response.body = self
            .body_decoder
            .decode(
                content_encoding.as_deref(),
                response.body,
                request.max_response_body_bytes,
            )
            .map_err(|error| AttemptError::Policy(error.to_string()))?;
        if response.body.len() > request.max_response_body_bytes {
            return Err(AttemptError::Policy(
                "decoded response body exceeded its configured limit".into(),
            ));
        }
        Ok(CompletedExchange {
            url: request.url.to_string(),
            response,
        })
    }

    async fn start_exchange(
        &self,
        request: &TransportRequest,
        log: &mut BoundedLog,
    ) -> Result<PendingExchange, AttemptError> {
        let mut wire_request = request.clone();
        if !has_header(&wire_request.headers, "cookie")
            && let Some(store) = &wire_request.cookie_store
            && let Some(value) = store
                .request_header(&wire_request.url)
                .await
                .map_err(|error| AttemptError::Policy(format!("cookie store failed: {error}")))?
        {
            wire_request.headers.push(ApiHeader::new("cookie", value));
        }
        log.push(format!("→ {} {}", wire_request.method, wire_request.url));
        for header in &wire_request.headers {
            log.push(format!("  {}: {}", header.name, header.value));
        }
        let encoded = encode_request(&wire_request)?;
        let mut connection = self
            .connector
            .connect(
                ApiConnectionTarget {
                    url: &wire_request.url,
                    ignore_https_errors: wire_request.ignore_https_errors,
                    resolved_addresses: wire_request.lookup_addresses.as_deref(),
                },
                &wire_request.cancellation,
            )
            .await?;
        ensure_active(&wire_request.cancellation)?;
        connection
            .write_all(&encoded, &wire_request.cancellation)
            .await?;

        let mut decoder = Http1ResponseDecoder::new(&wire_request);
        let mut buffer = vec![0; READ_BUFFER_BYTES];
        let mut received_response_bytes = false;
        let head = loop {
            ensure_active(&wire_request.cancellation)?;
            let bytes = connection
                .read(&mut buffer, &wire_request.cancellation)
                .await?;
            if bytes > buffer.len() {
                return Err(AttemptError::Policy(
                    "connector returned a read length larger than its buffer".into(),
                ));
            }
            if bytes == 0 && !received_response_bytes {
                return Err(AttemptError::Connection(ApiConnectionError::new(
                    ApiConnectionErrorKind::Reset,
                    "connection closed before response headers",
                )));
            }
            received_response_bytes |= bytes != 0;
            let event = if bytes == 0 {
                decoder.finish_event()?
            } else {
                decoder.feed_event(&buffer[..bytes])?
            };
            match event {
                Some(Http1ResponseEvent::Head(head)) => break head,
                Some(Http1ResponseEvent::Complete(_)) => {
                    return Err(AttemptError::Policy(
                        "HTTP/1 decoder completed before exposing response headers".into(),
                    ));
                }
                None if bytes == 0 => {
                    return Err(AttemptError::Codec(Http1CodecError::Truncated(
                        "response headers",
                    )));
                }
                None => {}
            }
        };
        log.push(format!("← {} {}", head.status, head.status_text));
        for header in &head.headers {
            log.push(format!("  {}: {}", header.name, header.value));
        }
        if let Some(store) = &wire_request.cookie_store {
            let values = head
                .headers
                .iter()
                .filter(|header| header.name.eq_ignore_ascii_case("set-cookie"))
                .map(|header| header.value.clone())
                .collect::<Vec<_>>();
            if !values.is_empty() {
                store
                    .store_response_headers(&wire_request.url, &values)
                    .await
                    .map_err(|error| {
                        AttemptError::Policy(format!("cookie store failed: {error}"))
                    })?;
            }
        }
        Ok(PendingExchange {
            head,
            connection,
            decoder,
            buffer,
        })
    }

    async fn finish_exchange(
        request: &TransportRequest,
        mut pending: PendingExchange,
    ) -> Result<Http1Response, AttemptError> {
        let mut event = pending.decoder.feed_event(&[])?;
        loop {
            match event {
                Some(Http1ResponseEvent::Complete(response)) => return Ok(response),
                Some(Http1ResponseEvent::Head(_)) => {
                    return Err(AttemptError::Policy(
                        "HTTP/1 decoder exposed a duplicate final response head".into(),
                    ));
                }
                None => {
                    ensure_active(&request.cancellation)?;
                    let bytes = pending
                        .connection
                        .read(&mut pending.buffer, &request.cancellation)
                        .await?;
                    if bytes > pending.buffer.len() {
                        return Err(AttemptError::Policy(
                            "connector returned a read length larger than its buffer".into(),
                        ));
                    }
                    event = if bytes == 0 {
                        pending.decoder.finish_event()?
                    } else {
                        pending.decoder.feed_event(&pending.buffer[..bytes])?
                    };
                    if event.is_none() && bytes == 0 {
                        return Err(AttemptError::Codec(Http1CodecError::Truncated(
                            "response body",
                        )));
                    }
                }
            }
        }
    }
}

impl std::fmt::Debug for ConnectionApiRequestTransport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ConnectionApiRequestTransport")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl ApiRequestTransport for ConnectionApiRequestTransport {
    async fn execute(
        &self,
        request: TransportRequest,
    ) -> Result<TransportResponse, ApiRequestError> {
        self.execute_with_retries(request).await
    }
}

#[derive(Clone, Copy)]
enum RedirectBudget {
    Disabled,
    Remaining(u16),
}

struct CompletedExchange {
    url: String,
    response: Http1Response,
}

struct PendingExchange {
    head: Http1ResponseHead,
    connection: Box<dyn ApiConnection>,
    decoder: Http1ResponseDecoder,
    buffer: Vec<u8>,
}

enum AttemptError {
    Connection(ApiConnectionError),
    Codec(Http1CodecError),
    Policy(String),
    Cancelled,
    RedirectCancelled,
}

impl From<ApiConnectionError> for AttemptError {
    fn from(error: ApiConnectionError) -> Self {
        Self::Connection(error)
    }
}

impl From<Http1CodecError> for AttemptError {
    fn from(error: Http1CodecError) -> Self {
        Self::Codec(error)
    }
}

struct BoundedLog {
    entries: Vec<String>,
    bytes: usize,
    max_entries: usize,
    max_bytes: usize,
    truncated: bool,
}

impl BoundedLog {
    const fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: Vec::new(),
            bytes: 0,
            max_entries,
            max_bytes,
            truncated: false,
        }
    }

    fn push(&mut self, entry: String) {
        if self.truncated {
            return;
        }
        let Some(next_entries) = self.entries.len().checked_add(1) else {
            self.truncate();
            return;
        };
        let Some(next_bytes) = self.bytes.checked_add(entry.len()) else {
            self.truncate();
            return;
        };
        if next_entries > self.max_entries || next_bytes > self.max_bytes {
            self.truncate();
            return;
        }
        self.entries.push(entry);
        self.bytes = next_bytes;
    }

    fn truncate(&mut self) {
        const SENTINEL: &str = "  [log truncated]";
        self.truncated = true;
        while !self.entries.is_empty()
            && (self.entries.len() >= self.max_entries
                || self.bytes.saturating_add(SENTINEL.len()) > self.max_bytes)
        {
            if let Some(removed) = self.entries.pop() {
                self.bytes = self.bytes.saturating_sub(removed.len());
            }
        }
        if self.entries.len() < self.max_entries
            && self.bytes.saturating_add(SENTINEL.len()) <= self.max_bytes
        {
            self.entries.push(SENTINEL.into());
            self.bytes += SENTINEL.len();
        }
    }
}

fn connection_error(error: &ApiConnectionError) -> ApiRequestError {
    ApiRequestError::Transport(error.to_string())
}

fn ensure_active(cancellation: &CancellationToken) -> Result<(), AttemptError> {
    if cancellation.is_cancelled() {
        Err(AttemptError::Cancelled)
    } else {
        Ok(())
    }
}

fn classify_redirect_cancellation(error: AttemptError, followed_redirect: bool) -> AttemptError {
    if followed_redirect
        && matches!(
            error,
            AttemptError::Cancelled
                | AttemptError::Connection(ApiConnectionError {
                    kind: ApiConnectionErrorKind::Cancelled,
                    ..
                })
        )
    {
        AttemptError::RedirectCancelled
    } else {
        error
    }
}

async fn wait_for_backoff(
    duration: Duration,
    cancellation: &CancellationToken,
) -> Result<(), ApiRequestError> {
    tokio::select! {
        () = tokio::time::sleep(duration) => Ok(()),
        () = cancellation.cancelled() => Err(ApiRequestError::Cancelled),
    }
}

const fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn should_rewrite_to_get(status: u16, method: &str) -> bool {
    ((status == 301 || status == 302) && method == "POST")
        || (status == 303 && method != "GET" && method != "HEAD")
}

fn header_value(headers: &[ApiHeader], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(name))
        .map(|header| header.value.clone())
}

fn has_header(headers: &[ApiHeader], name: &str) -> bool {
    headers
        .iter()
        .any(|header| header.name.eq_ignore_ascii_case(name))
}

fn remove_header(headers: &mut Vec<ApiHeader>, name: &str) {
    headers.retain(|header| !header.name.eq_ignore_ascii_case(name));
}

fn latin1_location_to_utf8(value: &str) -> String {
    let bytes = value
        .chars()
        .map(|character| character as u8)
        .collect::<Vec<_>>();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn host_value(url: &Url) -> Result<String, AttemptError> {
    let host = match url.host() {
        Some(url::Host::Domain(value)) => value.to_owned(),
        Some(url::Host::Ipv4(value)) => value.to_string(),
        Some(url::Host::Ipv6(value)) => format!("[{value}]"),
        None => return Err(AttemptError::Policy("redirect URL has no host".into())),
    };
    Ok(match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex, MutexGuard},
        time::Duration,
    };

    use async_trait::async_trait;
    use pwright_api_request::{
        ApiCookieStore, ApiHeader, ApiRequestError, ApiRequestTransport, TransportRequest,
    };
    use pwright_api_request_auth_basic::{BasicAuthorizationProvider, BasicAuthorizationSend};
    use pwright_api_request_cookie::MemoryCookieStore;
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use super::{
        ApiBodyDecodeError, ApiConnection, ApiConnectionError, ApiConnectionErrorKind,
        ApiConnectionTarget, ApiConnector, ApiResponseBodyDecoder, ConnectionApiRequestTransport,
    };

    enum ConnectionScript {
        Response(VecDeque<Vec<u8>>),
        ConnectError(ApiConnectionError),
        CancelOnRead,
    }

    struct ScriptedConnector {
        scripts: Mutex<VecDeque<ConnectionScript>>,
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl ScriptedConnector {
        fn new(scripts: impl IntoIterator<Item = ConnectionScript>) -> Self {
            Self {
                scripts: Mutex::new(scripts.into_iter().collect()),
                writes: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn writes(&self) -> Vec<Vec<u8>> {
            lock(&self.writes).clone()
        }
    }

    #[async_trait]
    impl ApiConnector for ScriptedConnector {
        async fn connect(
            &self,
            _target: ApiConnectionTarget<'_>,
            _cancellation: &CancellationToken,
        ) -> Result<Box<dyn ApiConnection>, ApiConnectionError> {
            match lock(&self.scripts)
                .pop_front()
                .expect("test connector script should exist")
            {
                ConnectionScript::Response(reads) => Ok(Box::new(ScriptedConnection {
                    reads,
                    writes: self.writes.clone(),
                })),
                ConnectionScript::ConnectError(error) => Err(error),
                ConnectionScript::CancelOnRead => Ok(Box::new(CancelOnReadConnection {
                    writes: self.writes.clone(),
                })),
            }
        }
    }

    struct ScriptedConnection {
        reads: VecDeque<Vec<u8>>,
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    struct CancelOnReadConnection {
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    struct FixedBodyDecoder(Vec<u8>);

    impl ApiResponseBodyDecoder for FixedBodyDecoder {
        fn decode(
            &self,
            _content_encoding: Option<&str>,
            _body: Vec<u8>,
            _max_output_bytes: usize,
        ) -> Result<Vec<u8>, ApiBodyDecodeError> {
            Ok(self.0.clone())
        }
    }

    #[async_trait]
    impl ApiConnection for ScriptedConnection {
        async fn write_all(
            &mut self,
            bytes: &[u8],
            _cancellation: &CancellationToken,
        ) -> Result<(), ApiConnectionError> {
            lock(&self.writes).push(bytes.to_vec());
            Ok(())
        }

        async fn read(
            &mut self,
            output: &mut [u8],
            _cancellation: &CancellationToken,
        ) -> Result<usize, ApiConnectionError> {
            let Some(mut next) = self.reads.pop_front() else {
                return Ok(0);
            };
            let bytes = next.len().min(output.len());
            output[..bytes].copy_from_slice(&next[..bytes]);
            if bytes != next.len() {
                next.drain(..bytes);
                self.reads.push_front(next);
            }
            Ok(bytes)
        }
    }

    #[async_trait]
    impl ApiConnection for CancelOnReadConnection {
        async fn write_all(
            &mut self,
            bytes: &[u8],
            _cancellation: &CancellationToken,
        ) -> Result<(), ApiConnectionError> {
            lock(&self.writes).push(bytes.to_vec());
            Ok(())
        }

        async fn read(
            &mut self,
            _output: &mut [u8],
            cancellation: &CancellationToken,
        ) -> Result<usize, ApiConnectionError> {
            cancellation.cancelled().await;
            Err(ApiConnectionError::new(
                ApiConnectionErrorKind::Cancelled,
                "cancelled while reading redirected response",
            ))
        }
    }

    fn request(method: &str, url: &str) -> TransportRequest {
        TransportRequest {
            method: method.into(),
            url: Url::parse(url).expect("test URL should parse"),
            headers: vec![ApiHeader::new("user-agent", "test")],
            body: None,
            timeout: Some(Duration::from_secs(1)),
            ignore_https_errors: false,
            cookie_store: None,
            authorization_provider: None,
            max_request_header_bytes: 4096,
            max_response_body_bytes: 4096,
            max_response_headers: 32,
            max_response_header_bytes: 4096,
            max_response_status_text_bytes: 128,
            max_response_log_entries: 64,
            max_response_log_bytes: 8192,
            max_redirects: 20,
            max_retries: 0,
            lookup_addresses: None,
            cancellation: CancellationToken::new(),
        }
    }

    fn response(bytes: &'static [u8]) -> ConnectionScript {
        ConnectionScript::Response(VecDeque::from([bytes.to_vec()]))
    }

    #[tokio::test]
    async fn exchange_returns_raw_headers_body_url_and_log() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([response(
            b"HTTP/1.1 200 Fine\r\nX-Case: one\r\nX-Case: two\r\nContent-Length: 2\r\n\r\nok",
        )]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());

        // Act
        let result = transport
            .execute(request("GET", "http://example.test/path"))
            .await
            .expect("exchange should succeed");

        // Assert
        assert_eq!(result.url, "http://example.test/path");
        assert_eq!(result.status, 200);
        assert_eq!(result.status_text, "Fine");
        assert_eq!(result.headers[0], ApiHeader::new("X-Case", "one"));
        assert_eq!(result.headers[1], ApiHeader::new("X-Case", "two"));
        assert_eq!(result.body, b"ok");
        assert_eq!(connector.writes().len(), 1);
        assert!(result.log.iter().any(|entry| entry == "← 200 Fine"));
    }

    #[tokio::test]
    async fn redirect_rewrites_post_headers_cookie_and_custom_host() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([
            response(
                b"HTTP/1.1 302 Found\r\nLocation: http://other.test/next\r\nContent-Length: 0\r\n\r\n",
            ),
            response(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"),
        ]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let mut input = request("POST", "http://example.test/start");
        input.body = Some(b"payload".to_vec());
        input.headers.extend([
            ApiHeader::new("Host", "example.test"),
            ApiHeader::new("Cookie", "secret=yes"),
            ApiHeader::new("Content-Type", "text/plain"),
            ApiHeader::new("Content-Length", "7"),
        ]);

        // Act
        let result = transport
            .execute(input)
            .await
            .expect("redirect should succeed");

        // Assert
        assert_eq!(result.url, "http://other.test/next");
        let writes = connector.writes();
        assert_eq!(writes.len(), 2);
        let redirected = String::from_utf8_lossy(&writes[1]);
        assert!(redirected.starts_with("GET /next HTTP/1.1\r\n"));
        assert!(redirected.contains("Host: other.test\r\n"));
        assert!(!redirected.to_ascii_lowercase().contains("cookie:"));
        assert!(!redirected.to_ascii_lowercase().contains("content-type:"));
        assert!(!redirected.to_ascii_lowercase().contains("content-length:"));
        assert!(redirected.ends_with("\r\n\r\n"));
    }

    #[tokio::test]
    async fn zero_redirect_limit_returns_original_redirect() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([response(
            b"HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\n\r\n",
        )]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let mut input = request("GET", "http://example.test/start");
        input.max_redirects = 0;

        // Act
        let result = transport
            .execute(input)
            .await
            .expect("disabled redirects should return 302");

        // Assert
        assert_eq!(result.status, 302);
        assert_eq!(result.url, "http://example.test/start");
        assert_eq!(connector.writes().len(), 1);
    }

    #[tokio::test]
    async fn redirect_budget_exhaustion_is_terminal() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([
            response(b"HTTP/1.1 302 Found\r\nLocation: /one\r\nContent-Length: 0\r\n\r\n"),
            response(b"HTTP/1.1 302 Found\r\nLocation: /two\r\nContent-Length: 0\r\n\r\n"),
        ]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let mut input = request("GET", "http://example.test/start");
        input.max_redirects = 1;

        // Act
        let error = transport
            .execute(input)
            .await
            .expect_err("second redirect should exhaust the budget");

        // Assert
        assert_eq!(
            error,
            ApiRequestError::Transport("Max redirect count exceeded".into())
        );
        assert_eq!(connector.writes().len(), 2);
    }

    #[tokio::test]
    async fn only_connection_reset_is_retried() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([
            ConnectionScript::ConnectError(ApiConnectionError::new(
                ApiConnectionErrorKind::Reset,
                "connection reset",
            )),
            response(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"),
        ]));
        let transport = ConnectionApiRequestTransport::new(connector);
        let mut input = request("GET", "http://example.test/");
        input.max_retries = 1;

        // Act
        let result = transport
            .execute(input)
            .await
            .expect("reset should be retried once");

        // Assert
        assert_eq!(result.status, 200);
        assert!(result.log.iter().any(|entry| entry.contains("ECONNRESET")));
    }

    #[tokio::test]
    async fn empty_eof_before_response_headers_is_retried_as_reset() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([
            response(b""),
            response(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"),
        ]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let mut input = request("GET", "http://example.test/");
        input.max_retries = 1;

        // Act
        let result = transport
            .execute(input)
            .await
            .expect("empty pre-header EOF should retry once");

        // Assert
        assert_eq!(result.status, 200);
        assert_eq!(connector.writes().len(), 2);
        assert!(result.log.iter().any(|entry| entry.contains("ECONNRESET")));
    }

    #[tokio::test]
    async fn cancellation_after_following_redirect_is_target_closed() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([
            response(b"HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\n\r\n"),
            ConnectionScript::CancelOnRead,
        ]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let input = request("GET", "http://example.test/start");
        let cancellation = input.cancellation.clone();
        let execution = transport.execute(input);
        tokio::pin!(execution);

        // Act
        tokio::select! {
            () = async {
                while connector.writes().len() < 2 {
                    tokio::task::yield_now().await;
                }
            } => {}
            result = &mut execution => panic!("redirected request settled before cancellation: {result:?}"),
        }
        cancellation.cancel();
        let error = execution
            .await
            .expect_err("redirected cancellation should fail");

        // Assert
        assert_eq!(error, ApiRequestError::TargetClosed);
        assert_eq!(connector.writes().len(), 2);
    }

    #[tokio::test]
    async fn non_reset_error_is_not_retried() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([
            ConnectionScript::ConnectError(ApiConnectionError::new(
                ApiConnectionErrorKind::Dns,
                "lookup failed",
            )),
            response(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"),
        ]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let mut input = request("GET", "http://example.test/");
        input.max_retries = 2;

        // Act
        let error = transport
            .execute(input)
            .await
            .expect_err("DNS errors should not be retried");

        // Assert
        assert_eq!(error, ApiRequestError::Transport("lookup failed".into()));
        assert!(connector.writes().is_empty());
        assert_eq!(lock(&connector.scripts).len(), 1);
    }

    #[tokio::test]
    async fn transport_log_is_truncated_without_failing_public_response() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([response(
            b"HTTP/1.1 200 OK\r\nX-One: 1\r\nX-Two: 2\r\nContent-Length: 0\r\n\r\n",
        )]));
        let transport = ConnectionApiRequestTransport::new(connector);
        let mut input = request("GET", "http://example.test/");
        input.max_response_log_entries = 2;

        // Act
        let result = transport
            .execute(input)
            .await
            .expect("log pressure must not fail the response");

        // Assert
        assert_eq!(result.status, 200);
        assert_eq!(result.log.len(), 2);
        assert_eq!(result.log[1], "  [log truncated]");
    }

    #[tokio::test]
    async fn pre_cancelled_request_never_opens_a_connection() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n",
        )]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let input = request("GET", "http://example.test/");
        input.cancellation.cancel();

        // Act
        let error = transport
            .execute(input)
            .await
            .expect_err("cancelled request should fail before connect");

        // Assert
        assert_eq!(error, ApiRequestError::Cancelled);
        assert!(connector.writes().is_empty());
        assert_eq!(lock(&connector.scripts).len(), 1);
    }

    #[tokio::test]
    async fn selected_body_decoder_owns_final_response_bytes() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([response(
            b"HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: 1\r\n\r\nx",
        )]));
        let transport = ConnectionApiRequestTransport::with_body_decoder(
            connector,
            Arc::new(FixedBodyDecoder(b"decoded".to_vec())),
        );

        // Act
        let result = transport
            .execute(request("GET", "http://example.test/"))
            .await
            .expect("selected decoder should succeed");

        // Assert
        assert_eq!(result.body, b"decoded");
        assert_eq!(
            result.headers[0],
            ApiHeader::new("Content-Encoding", "gzip")
        );
    }

    #[tokio::test]
    async fn decoder_output_is_independently_checked_against_body_limit() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx",
        )]));
        let transport = ConnectionApiRequestTransport::with_body_decoder(
            connector,
            Arc::new(FixedBodyDecoder(b"too large".to_vec())),
        );
        let mut input = request("GET", "http://example.test/");
        input.max_response_body_bytes = 4;

        // Act
        let error = transport
            .execute(input)
            .await
            .expect_err("oversized decoder output should fail");

        // Assert
        assert_eq!(
            error,
            ApiRequestError::Transport(
                "decoded response body exceeded its configured limit".into()
            )
        );
    }

    #[tokio::test]
    async fn redirect_stores_set_cookie_before_composing_next_request() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([
            response(
                b"HTTP/1.1 302 Found\r\nLocation: /next\r\nSet-Cookie: a=v; Path=/\r\nContent-Length: 0\r\n\r\n",
            ),
            response(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"),
        ]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let mut input = request("GET", "http://example.test/start");
        input.cookie_store = Some(Arc::new(
            MemoryCookieStore::with_defaults().expect("test cookie store should build"),
        ));

        // Act
        let result = transport
            .execute(input)
            .await
            .expect("cookie redirect should succeed");

        // Assert
        assert_eq!(result.status, 200);
        let writes = connector.writes();
        assert_eq!(writes.len(), 2);
        let redirected = String::from_utf8_lossy(&writes[1]).to_ascii_lowercase();
        assert!(redirected.contains("cookie: a=v\r\n"));
    }

    #[tokio::test]
    async fn followed_redirect_drops_oversized_body_after_headers() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([
            response(b"HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 100000\r\n\r\n"),
            response(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"),
        ]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let mut input = request("GET", "http://example.test/start");
        input.max_response_body_bytes = 1;

        // Act
        let result = transport
            .execute(input)
            .await
            .expect("redirect body should not be consumed");

        // Assert
        assert_eq!(result.status, 200);
        assert_eq!(connector.writes().len(), 2);
    }

    #[tokio::test]
    async fn explicit_cookie_header_overrides_selected_cookie_store() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n",
        )]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let store =
            Arc::new(MemoryCookieStore::with_defaults().expect("test cookie store should build"));
        let url = Url::parse("http://example.test/start").expect("test URL should parse");
        store
            .store_response_headers(&url, &["stored=value; Path=/".into()])
            .await
            .expect("test cookie should store");
        let mut input = request("GET", url.as_str());
        input
            .headers
            .push(ApiHeader::new("Cookie", "explicit=value"));
        input.cookie_store = Some(store);

        // Act
        transport
            .execute(input)
            .await
            .expect("request should succeed");

        // Assert
        let request = String::from_utf8_lossy(&connector.writes()[0]).to_ascii_lowercase();
        assert!(request.contains("cookie: explicit=value\r\n"));
        assert!(!request.contains("stored=value"));
    }

    #[tokio::test]
    async fn basic_challenge_retries_once_with_authorization() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([
            response(
                b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=test\r\nContent-Length: 0\r\n\r\n",
            ),
            response(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"),
        ]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let mut input = request("GET", "http://example.test/protected");
        input.authorization_provider = Some(Arc::new(
            BasicAuthorizationProvider::new(
                "user",
                "pass",
                None,
                BasicAuthorizationSend::Unauthorized,
            )
            .expect("provider should build"),
        ));

        // Act
        let result = transport
            .execute(input)
            .await
            .expect("challenge should succeed");

        // Assert
        assert_eq!(result.status, 200);
        let writes = connector.writes();
        assert_eq!(writes.len(), 2);
        assert!(
            !String::from_utf8_lossy(&writes[0])
                .to_ascii_lowercase()
                .contains("authorization:")
        );
        assert!(
            String::from_utf8_lossy(&writes[1]).contains("authorization: Basic dXNlcjpwYXNz\r\n")
        );
    }

    #[tokio::test]
    async fn basic_challenge_drops_oversized_body_after_headers() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([
            response(
                b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic\r\nContent-Length: 100000\r\n\r\n",
            ),
            response(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"),
        ]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let mut input = request("GET", "http://example.test/protected");
        input.max_response_body_bytes = 1;
        input.authorization_provider = Some(Arc::new(
            BasicAuthorizationProvider::new(
                "user",
                "pass",
                None,
                BasicAuthorizationSend::Unauthorized,
            )
            .expect("provider should build"),
        ));

        // Act
        let result = transport
            .execute(input)
            .await
            .expect("challenge body should not be consumed");

        // Assert
        assert_eq!(result.status, 200);
        assert_eq!(connector.writes().len(), 2);
    }

    #[tokio::test]
    async fn terminal_response_still_enforces_declared_body_limit() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\n\r\n",
        )]));
        let transport = ConnectionApiRequestTransport::new(connector);
        let mut input = request("GET", "http://example.test/final");
        input.max_response_body_bytes = 1;

        // Act
        let error = transport
            .execute(input)
            .await
            .expect_err("terminal body limit should remain enforced");

        // Assert
        assert_eq!(
            error,
            ApiRequestError::Transport("HTTP/1 response exceeded response body bytes".into())
        );
    }

    #[tokio::test]
    async fn always_basic_authorization_is_sent_on_first_exchange() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n",
        )]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let mut input = request("GET", "http://example.test/protected");
        input.authorization_provider = Some(Arc::new(
            BasicAuthorizationProvider::new(
                "user",
                "pass",
                Some("HTTP://EXAMPLE.TEST"),
                BasicAuthorizationSend::Always,
            )
            .expect("provider should build"),
        ));

        // Act
        transport
            .execute(input)
            .await
            .expect("preemptive request should succeed");

        // Assert
        let writes = connector.writes();
        assert_eq!(writes.len(), 1);
        assert!(
            String::from_utf8_lossy(&writes[0]).contains("authorization: Basic dXNlcjpwYXNz\r\n")
        );
    }

    #[tokio::test]
    async fn explicit_authorization_header_overrides_selected_provider() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n",
        )]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let mut input = request("GET", "http://example.test/protected");
        input
            .headers
            .push(ApiHeader::new("Authorization", "Bearer explicit"));
        input.authorization_provider = Some(Arc::new(
            BasicAuthorizationProvider::new("user", "pass", None, BasicAuthorizationSend::Always)
                .expect("provider should build"),
        ));

        // Act
        transport
            .execute(input)
            .await
            .expect("explicit authorization should succeed");

        // Assert
        let write = String::from_utf8_lossy(&connector.writes()[0]).to_string();
        assert!(write.contains("Authorization: Bearer explicit\r\n"));
        assert!(!write.contains("Basic dXNlcjpwYXNz"));
    }

    #[tokio::test]
    async fn mismatching_basic_origin_does_not_retry_challenge() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([response(
            b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic\r\nContent-Length: 0\r\n\r\n",
        )]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let mut input = request("GET", "http://example.test/protected");
        input.authorization_provider = Some(Arc::new(
            BasicAuthorizationProvider::new(
                "user",
                "pass",
                Some("https://example.test"),
                BasicAuthorizationSend::Unauthorized,
            )
            .expect("provider should build"),
        ));

        // Act
        let result = transport
            .execute(input)
            .await
            .expect("unmatched challenge should return its response");

        // Assert
        assert_eq!(result.status, 401);
        assert_eq!(connector.writes().len(), 1);
    }

    #[tokio::test]
    async fn preemptive_basic_policy_is_evaluated_before_redirect_chain_only() {
        // Arrange
        let connector = Arc::new(ScriptedConnector::new([
            response(
                b"HTTP/1.1 302 Found\r\nLocation: http://destination.test/next\r\nContent-Length: 0\r\n\r\n",
            ),
            response(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"),
        ]));
        let transport = ConnectionApiRequestTransport::new(connector.clone());
        let mut input = request("GET", "http://example.test/start");
        input.authorization_provider = Some(Arc::new(
            BasicAuthorizationProvider::new(
                "user",
                "pass",
                Some("http://destination.test"),
                BasicAuthorizationSend::Always,
            )
            .expect("provider should build"),
        ));

        // Act
        let result = transport
            .execute(input)
            .await
            .expect("redirect should succeed");

        // Assert
        assert_eq!(result.status, 200);
        let writes = connector.writes();
        assert_eq!(writes.len(), 2);
        assert!(writes.iter().all(|write| {
            !String::from_utf8_lossy(write)
                .to_ascii_lowercase()
                .contains("authorization:")
        }));
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
