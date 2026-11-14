//! Explicit composition for a successful Playwright Server connection.
//!
//! An injected HTTP fetcher resolves direct WS(S) or HTTP(S) `/json` endpoints,
//! an injected WebSocket connector establishes and validates the remote Browser
//! lease, and a Tokio supervisor owns the live connection after bootstrap.
//! This crate neither launches browsers nor chooses Node/Python header policy.
//! The caller supplies all WebSocket headers, SDK language and wall-clock value.
//! `live` is an explicit feature; default builds contain no Reqwest connector.
//!
//! Injected WebSocket peers must make `receive` cancellation-safe. The
//! supervisor cancels an in-flight receive when a command or close is selected.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_playwright_server_endpoint_discovery::{
    DiscoveryError, DiscoveryLimits, HttpDiscoveryFetcher, resolve_endpoint,
};
use pwright_playwright_server_session::{
    EventPolicy, RemoteBrowserLease, RemoteBrowserSupervisor, SdkLanguage, SessionError,
    SessionFailure,
};
use pwright_transport_ws::{
    MAX_HTTP_REJECTION_BODY_BYTES, WsConnector, WsError, WsHeaders, WsLimits, WsPeer, WsRequest,
};

/// All required connection inputs. Header values and endpoint query data are
/// redacted in Debug, and no browser-specific defaults are applied here.
#[derive(Clone)]
pub struct ConnectOptions {
    endpoint: String,
    discovery_user_agent: String,
    discovery_limits: DiscoveryLimits,
    websocket_limits: WsLimits,
    websocket_headers: WsHeaders,
    sdk_language: SdkLanguage,
    wall_time_ms: u64,
}

impl fmt::Debug for ConnectOptions {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConnectOptions")
            .field("endpoint", &"<redacted>")
            .field("discovery_user_agent", &"<redacted>")
            .field("discovery_limits", &self.discovery_limits)
            .field("websocket_limits", &self.websocket_limits)
            .field("websocket_headers", &self.websocket_headers)
            .field("sdk_language", &self.sdk_language)
            .field("wall_time_ms", &self.wall_time_ms)
            .finish()
    }
}

impl ConnectOptions {
    pub fn new(
        endpoint: impl Into<String>,
        discovery_user_agent: impl Into<String>,
        discovery_limits: DiscoveryLimits,
        websocket_limits: WsLimits,
        sdk_language: SdkLanguage,
        wall_time_ms: u64,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            discovery_user_agent: discovery_user_agent.into(),
            discovery_limits,
            websocket_limits,
            websocket_headers: WsHeaders::default(),
            sdk_language,
            wall_time_ms,
        }
    }

    /// Caller and language adapter own header precedence and defaults. Every
    /// explicit header is checked under the WebSocket transport's shared cap.
    pub fn add_websocket_header(&mut self, name: &str, value: &str) -> Result<(), WsError> {
        self.websocket_headers.push(name, value)
    }

    /// For HTTP(S) endpoint discovery only, retain bounded headers until the
    /// endpoint is resolved. `connect_with_event_policy` then inserts each one
    /// into a strict `WsRequest` before opening a WebSocket. Direct WS(S)
    /// endpoints must use `add_websocket_header` and reject reserved names now.
    pub fn add_websocket_header_after_http_discovery(
        &mut self,
        name: &str,
        value: &str,
    ) -> Result<(), WsError> {
        let http = self
            .endpoint
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"));
        let https = self
            .endpoint
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"));
        if !http && !https {
            return Err(WsError::InvalidEndpoint(
                "deferred WebSocket headers require HTTP(S) discovery",
            ));
        }
        self.websocket_headers.push_for_http_discovery(name, value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConnectError {
    Discovery(DiscoveryError),
    WebSocket(WsError),
    Session(SessionFailure),
}

/// Versions extracted from Playwright Server's HTTP 428 handshake rejection.
/// Only short ASCII `major.minor` values are retained; the remote body is not.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaywrightVersionMismatch {
    server_version: String,
    client_version: String,
}

impl PlaywrightVersionMismatch {
    pub fn server_version(&self) -> &str {
        &self.server_version
    }

    pub fn client_version(&self) -> &str {
        &self.client_version
    }
}

impl ConnectError {
    /// Opt in to the version information in a canonical Playwright Server 428.
    /// Other remote response bodies never become user-facing diagnostics.
    pub fn playwright_version_mismatch(&self) -> Option<PlaywrightVersionMismatch> {
        let Self::Session(SessionFailure {
            error: SessionError::WebSocket(WsError::HttpRejected { status: 428, body }),
            ..
        }) = self
        else {
            return None;
        };
        parse_playwright_version_mismatch(body)
    }
}

fn parse_playwright_version_mismatch(body: &str) -> Option<PlaywrightVersionMismatch> {
    if body.len() > MAX_HTTP_REJECTION_BODY_BYTES {
        return None;
    }

    // Playwright's wrapInASCIIBox uses one space of padding around each line.
    let mut lines = body.lines();
    let top = lines.next()?.strip_prefix('╔')?.strip_suffix('╗')?;
    let width = top.chars().count();
    if !(30..=256).contains(&width) || !top.chars().all(|character| character == '═') {
        return None;
    }
    if boxed_line(lines.next()?, width)? != "Playwright version mismatch:" {
        return None;
    }
    let server_version = boxed_line(lines.next()?, width)?
        .strip_prefix("  - server version: v")
        .and_then(parse_major_minor)?;
    let client_version = boxed_line(lines.next()?, width)?
        .strip_prefix("  - client version: v")
        .and_then(parse_major_minor)?;
    if server_version == client_version {
        return None;
    }

    let mut final_line = lines.next()?;
    for line in lines {
        boxed_line(final_line, width)?;
        final_line = line;
    }
    let bottom = final_line.strip_prefix('╚')?.strip_suffix('╝')?;
    if bottom != top {
        return None;
    }
    Some(PlaywrightVersionMismatch {
        server_version: server_version.to_owned(),
        client_version: client_version.to_owned(),
    })
}

fn boxed_line(line: &str, width: usize) -> Option<&str> {
    let content = line.strip_prefix("║ ")?.strip_suffix('║')?;
    if !content.ends_with(' ') || content.chars().count() + 1 != width {
        return None;
    }
    Some(content.trim_end_matches(' '))
}

fn parse_major_minor(version: &str) -> Option<&str> {
    const MAX_COMPONENT_DIGITS: usize = 5;
    let (major, minor) = version.split_once('.')?;
    if major.is_empty()
        || minor.is_empty()
        || major.len() > MAX_COMPONENT_DIGITS
        || minor.len() > MAX_COMPONENT_DIGITS
        || !major
            .bytes()
            .chain(minor.bytes())
            .all(|digit| digit.is_ascii_digit())
    {
        return None;
    }
    Some(version)
}

impl Display for ConnectError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(error) => Display::fmt(error, formatter),
            Self::WebSocket(WsError::HttpRejected { status, .. }) => {
                write!(
                    formatter,
                    "WebSocket HTTP handshake rejected with status {status}"
                )
            }
            Self::WebSocket(error) => Display::fmt(error, formatter),
            Self::Session(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ConnectError {}

/// Return a live supervised Browser only after endpoint resolution and a
/// validated `Root.initialize`/`preLaunchedBrowser` exchange. This is a
/// protocol handle, not yet a Node/Python Browser/Context/Page adapter.
pub async fn connect<F, C>(
    options: &ConnectOptions,
    fetcher: &F,
    connector: &C,
) -> Result<RemoteBrowserSupervisor, ConnectError>
where
    F: HttpDiscoveryFetcher,
    C: WsConnector,
    C::Peer: WsPeer + 'static,
{
    connect_with_event_policy(options, fetcher, connector, EventPolicy::QueueBounded).await
}

/// Opt into projecting validated ingress without retaining an event stream
/// when no language adapter consumes it.
pub async fn connect_with_event_policy<F, C>(
    options: &ConnectOptions,
    fetcher: &F,
    connector: &C,
    event_policy: EventPolicy,
) -> Result<RemoteBrowserSupervisor, ConnectError>
where
    F: HttpDiscoveryFetcher,
    C: WsConnector,
    C::Peer: WsPeer + 'static,
{
    let endpoint = resolve_endpoint(
        &options.endpoint,
        &options.discovery_user_agent,
        options.discovery_limits,
        fetcher,
    )
    .await
    .map_err(ConnectError::Discovery)?;
    let mut request = WsRequest::new(endpoint, options.websocket_limits);
    for header in options.websocket_headers.iter() {
        request
            .add_header(header.name(), header.value())
            .map_err(ConnectError::WebSocket)?;
    }
    let lease = RemoteBrowserLease::connect(
        connector,
        &request,
        options.sdk_language,
        options.wall_time_ms,
    )
    .await
    .map_err(ConnectError::Session)?;
    RemoteBrowserSupervisor::start_with_event_policy(lease, event_policy)
        .await
        .map_err(ConnectError::Session)
}

/// Real HTTP and WebSocket I/O is available only under an explicit feature.
#[cfg(feature = "live")]
pub async fn connect_live(
    options: &ConnectOptions,
) -> Result<RemoteBrowserSupervisor, ConnectError> {
    connect_live_with_event_policy(options, EventPolicy::QueueBounded).await
}

/// Use the real connector with an explicit event-retention policy.
#[cfg(feature = "live")]
pub async fn connect_live_with_event_policy(
    options: &ConnectOptions,
    event_policy: EventPolicy,
) -> Result<RemoteBrowserSupervisor, ConnectError> {
    use pwright_playwright_server_endpoint_discovery::{DiscoveryError, ReqwestDiscoveryFetcher};
    use pwright_playwright_server_session::LiveWsRejectionBodyConnector;

    let fetcher = ReqwestDiscoveryFetcher::new()
        .map_err(|error| ConnectError::Discovery(DiscoveryError::Fetch(error)))?;
    connect_with_event_policy(
        options,
        &fetcher,
        &LiveWsRejectionBodyConnector,
        event_policy,
    )
    .await
}

#[cfg(test)]
mod tests;
