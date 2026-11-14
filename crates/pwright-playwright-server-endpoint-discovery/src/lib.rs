//! Opt-in Playwright Server endpoint discovery, independent of browser ownership.
//!
//! Playwright 1.58.0 turns an HTTP(S) `BrowserType.connect` endpoint into a
//! WebSocket endpoint by GETting `<path>/json` with its User-Agent, then
//! appending the returned `wsEndpointPath` to the original URL path. This
//! crate owns that narrow URL policy. It does not launch a browser, add
//! WebSocket handshake headers, follow WebSocket redirects, or create a remote
//! Browser. The live HTTP fetcher is an explicit `live-reqwest` feature.
//!
//! The live fetcher follows a bounded number of HTTP redirects, as upstream's
//! generic HTTP helper does. It rejects malformed or over-limit redirect
//! targets rather than allowing an unbounded redirect chain.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;

use pwright_transport_ws::{
    MAX_CONNECT_TIMEOUT_MS, MAX_ENDPOINT_BYTES, WsEndpoint, WsError, WsHeaders,
};
use url::Url;

#[cfg(feature = "live-reqwest")]
mod live;
#[cfg(feature = "live-reqwest")]
pub use live::ReqwestDiscoveryFetcher;

pub const MAX_DISCOVERY_BODY_BYTES: usize = 64 * 1024;
pub const MAX_DISCOVERY_REDIRECTS: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiscoveryLimits {
    max_body_bytes: usize,
    timeout_ms: u32,
}

impl DiscoveryLimits {
    pub fn new(max_body_bytes: usize, timeout_ms: u32) -> Result<Self, DiscoveryError> {
        if max_body_bytes == 0 || max_body_bytes > MAX_DISCOVERY_BODY_BYTES {
            return Err(DiscoveryError::InvalidLimits(
                "body limit must be between 1 and 65536 bytes",
            ));
        }
        if timeout_ms == 0 || timeout_ms > MAX_CONNECT_TIMEOUT_MS {
            return Err(DiscoveryError::InvalidLimits(
                "timeout must be between 1 and 120000 ms",
            ));
        }
        Ok(Self {
            max_body_bytes,
            timeout_ms,
        })
    }

    pub const fn max_body_bytes(self) -> usize {
        self.max_body_bytes
    }

    pub const fn timeout_ms(self) -> u32 {
        self.timeout_ms
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FetchError {
    Timeout,
    Transport,
    BodyTooLarge,
}

impl Display for FetchError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => formatter.write_str("Playwright Server discovery timed out"),
            Self::Transport => {
                formatter.write_str("Playwright Server discovery HTTP request failed")
            }
            Self::BodyTooLarge => {
                formatter.write_str("Playwright Server discovery body exceeded limit")
            }
        }
    }
}

impl Error for FetchError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiscoveryError {
    InvalidEndpoint(&'static str),
    InvalidUserAgent(WsError),
    InvalidLimits(&'static str),
    Fetch(FetchError),
    HttpStatus(u16),
    BodyTooLarge,
    InvalidResponse(&'static str),
    WebSocket(WsError),
}

impl Display for DiscoveryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEndpoint(reason) => {
                write!(formatter, "invalid Playwright Server endpoint: {reason}")
            }
            Self::InvalidUserAgent(_) => {
                formatter.write_str("invalid Playwright Server User-Agent")
            }
            Self::InvalidLimits(reason) => write!(formatter, "invalid discovery limits: {reason}"),
            Self::Fetch(error) => Display::fmt(error, formatter),
            Self::HttpStatus(status) => write!(
                formatter,
                "Playwright Server discovery returned HTTP {status}"
            ),
            Self::BodyTooLarge => {
                formatter.write_str("Playwright Server discovery body exceeded limit")
            }
            Self::InvalidResponse(reason) => write!(
                formatter,
                "invalid Playwright Server discovery response: {reason}"
            ),
            Self::WebSocket(_) => formatter.write_str("invalid discovered WebSocket endpoint"),
        }
    }
}

impl Error for DiscoveryError {}

/// A URL and exactly one application-supplied header for the discovery GET.
/// The URL may contain query credentials, so its Debug representation is redacted.
#[derive(Clone)]
pub struct DiscoveryRequest {
    url: Url,
    user_agent: String,
    limits: DiscoveryLimits,
}

impl fmt::Debug for DiscoveryRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DiscoveryRequest")
            .field("url", &"<redacted>")
            .field("user_agent", &"<redacted>")
            .field("limits", &self.limits)
            .finish()
    }
}

impl DiscoveryRequest {
    pub const fn url(&self) -> &Url {
        &self.url
    }

    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    pub const fn limits(&self) -> DiscoveryLimits {
        self.limits
    }
}

/// The body remains private and is never included in Debug output.
#[derive(Clone)]
pub struct DiscoveryResponse {
    status: u16,
    body: Vec<u8>,
}

impl fmt::Debug for DiscoveryResponse {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DiscoveryResponse")
            .field("status", &self.status)
            .field("body", &"<redacted>")
            .finish()
    }
}

impl DiscoveryResponse {
    pub const fn new(status: u16, body: Vec<u8>) -> Self {
        Self { status, body }
    }

    pub const fn status(&self) -> u16 {
        self.status
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

/// Injected HTTP transport. It must honor the request timeout and body cap.
/// A resolver also rechecks the body cap, including for a faulty fake.
pub trait HttpDiscoveryFetcher: Send + Sync {
    fn fetch<'a>(
        &'a self,
        request: &'a DiscoveryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<DiscoveryResponse, FetchError>> + Send + 'a>>;
}

/// Resolve a direct WS(S) URL or discover an HTTP(S) Playwright Server URL.
/// WebSocket request headers stay outside this API and never reach the GET.
pub async fn resolve_endpoint<F: HttpDiscoveryFetcher>(
    endpoint: &str,
    user_agent: &str,
    limits: DiscoveryLimits,
    fetcher: &F,
) -> Result<WsEndpoint, DiscoveryError> {
    let url = parse_endpoint(endpoint)?;
    if matches!(url.scheme(), "ws" | "wss") {
        return WsEndpoint::parse(url.to_string()).map_err(DiscoveryError::WebSocket);
    }

    let mut validated_user_agent = WsHeaders::default();
    validated_user_agent
        .push("User-Agent", user_agent)
        .map_err(DiscoveryError::InvalidUserAgent)?;

    let mut discovery_url = url.clone();
    let path = append_path_segment(discovery_url.path(), "json");
    discovery_url.set_path(&path);
    if discovery_url.as_str().len() > MAX_ENDPOINT_BYTES {
        return Err(DiscoveryError::InvalidEndpoint(
            "discovery URL exceeds 8192 bytes",
        ));
    }
    let request = DiscoveryRequest {
        url: discovery_url,
        user_agent: user_agent.to_owned(),
        limits,
    };
    let response = fetcher.fetch(&request).await.map_err(|error| match error {
        FetchError::BodyTooLarge => DiscoveryError::BodyTooLarge,
        other => DiscoveryError::Fetch(other),
    })?;
    if response.status != 200 {
        return Err(DiscoveryError::HttpStatus(response.status));
    }
    if response.body.len() > limits.max_body_bytes {
        return Err(DiscoveryError::BodyTooLarge);
    }
    let json: serde_json::Value = serde_json::from_slice(&response.body)
        .map_err(|_| DiscoveryError::InvalidResponse("body is not valid JSON"))?;
    let ws_path = json
        .get("wsEndpointPath")
        .and_then(serde_json::Value::as_str)
        .ok_or(DiscoveryError::InvalidResponse(
            "wsEndpointPath is missing or not a string",
        ))?;
    let ws_path = ws_path.strip_prefix('/').unwrap_or(ws_path);
    let mut websocket_url = url;
    let path = append_path_segment(websocket_url.path(), ws_path);
    websocket_url.set_path(&path);
    let ws_scheme = if websocket_url.scheme() == "https" {
        "wss"
    } else {
        "ws"
    };
    websocket_url
        .set_scheme(ws_scheme)
        .map_err(|()| DiscoveryError::InvalidEndpoint("cannot convert URL scheme"))?;
    WsEndpoint::parse(websocket_url.to_string()).map_err(DiscoveryError::WebSocket)
}

fn parse_endpoint(endpoint: &str) -> Result<Url, DiscoveryError> {
    if endpoint.len() > MAX_ENDPOINT_BYTES {
        return Err(DiscoveryError::InvalidEndpoint(
            "endpoint exceeds 8192 bytes",
        ));
    }
    if endpoint.bytes().any(|byte| byte <= 0x20 || byte == 0x7f) {
        return Err(DiscoveryError::InvalidEndpoint(
            "unescaped whitespace or controls are unsupported",
        ));
    }
    let url = Url::parse(endpoint)
        .map_err(|_| DiscoveryError::InvalidEndpoint("expected an absolute URL"))?;
    if !matches!(url.scheme(), "http" | "https" | "ws" | "wss") || url.host_str().is_none() {
        return Err(DiscoveryError::InvalidEndpoint(
            "scheme must be HTTP(S) or WS(S) with a host",
        ));
    }
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err(DiscoveryError::InvalidEndpoint(
            "userinfo and fragments are unsupported",
        ));
    }
    if url.as_str().len() > MAX_ENDPOINT_BYTES {
        return Err(DiscoveryError::InvalidEndpoint(
            "canonical endpoint exceeds 8192 bytes",
        ));
    }
    Ok(url)
}

fn append_path_segment(prefix: &str, segment: &str) -> String {
    let mut path = prefix.to_owned();
    if !path.ends_with('/') {
        path.push('/');
    }
    path.push_str(segment);
    path
}

#[cfg(test)]
mod tests;
