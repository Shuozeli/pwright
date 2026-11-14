//! Opt-in, connection-owned Playwright Server Browser bootstrap.
//!
//! The caller supplies a checked WebSocket request, connector, SDK language,
//! and wall-clock metadata. This composition does not launch a browser, choose
//! an endpoint, add headers, or select the live connector by default. It only
//! returns a Browser lease after the upstream Root.initialize reply and the
//! referenced server-owned GUID graph have both been validated.
//!
//! The lease is a protocol foundation, not yet a complete public Browser API:
//! Context/Page adapters and language bindings remain separate capabilities.
//! Pre-reply lifecycle messages and events are replayed in wire order before
//! later messages. A caller can drive the lease directly or enable the
//! `supervisor` feature for a bounded background owner that polls continuously.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::{self, Display};
use std::time::Duration;

use pwright_playwright_server_client_core::{
    ClientCore, ClientError, CloseReason, CloseReport, CommandId, Ingress, MAX_OBJECTS,
    PendingCommand, ProtocolViolation, ReceiveFailure, RemoteError, ReplyOutcome,
};
use pwright_transport_ws::{
    WsCloseFrame, WsConnector, WsError, WsIncoming, WsOutgoing, WsPeer, WsRequest, WsTransport,
};
use serde_json::{Value, json};
use tokio::time::{Instant, timeout, timeout_at};

#[cfg(feature = "supervisor")]
mod supervisor;
#[cfg(feature = "page-events")]
pub use supervisor::PageNotice;
#[cfg(feature = "close-notices")]
pub use supervisor::{CloseNotice, ObjectId};
#[cfg(feature = "supervisor")]
pub use supervisor::{
    EventPolicy, MAX_SUPERVISOR_COMMANDS, MAX_SUPERVISOR_EVENT_BYTES, MAX_SUPERVISOR_EVENTS,
    MAX_SUPERVISOR_OBJECT_BYTES, MAX_SUPERVISOR_OBJECTS, MAX_SUPERVISOR_SEND_BUDGET_MS,
    RemoteBrowserSupervisor, RemoteGraphSnapshot, RemoteObjectSnapshot, SupervisorError,
    SupervisorState, TerminalWatcher,
};

#[cfg(feature = "live-tokio")]
pub use pwright_transport_ws::{LiveWsConnector, LiveWsRejectionBodyConnector};

pub const MAX_BOOTSTRAP_MESSAGES: usize = MAX_OBJECTS + 128;
pub const MAX_BOOTSTRAP_BUDGET_MS: u32 = 30_000;
pub const MAX_BOOTSTRAP_BUFFERED_INGRESS: usize = 1024;
pub const MAX_BOOTSTRAP_BUFFERED_BYTES: usize = 1024 * 1024;
const CLOSE_BUDGET: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SdkLanguage {
    Javascript,
    Python,
    Java,
    Csharp,
}

impl SdkLanguage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Javascript => "javascript",
            Self::Python => "python",
            Self::Java => "java",
            Self::Csharp => "csharp",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionError {
    Client(ClientError),
    WebSocket(WsError),
    Protocol(ProtocolViolation),
    Remote(RemoteError),
    RemoteClose(Option<WsCloseFrame>),
    EndOfStream,
    BinaryMessage,
    MalformedBootstrap(&'static str),
    BootstrapLimit,
    EventQueueFull,
    #[cfg(any(feature = "page-events", feature = "close-notices"))]
    PageEventProjection(pwright_playwright_server_page_events::ProjectionError),
    ObjectGraphLimit,
    ObjectGraphInvalid(&'static str),
    DeadlineExceeded,
    SendDeadlineExceeded,
    BrowserClosed,
}

impl Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Remote text may carry credentials. Keep Display suitable for logs;
        // structured callers can inspect the typed variant deliberately.
        match self {
            Self::Client(_) => f.write_str("Playwright Server client command rejected"),
            Self::WebSocket(_) => f.write_str("Playwright Server WebSocket failed"),
            Self::Protocol(_) => f.write_str("Playwright Server protocol violation"),
            Self::Remote(_) => f.write_str("Playwright Server root initialization failed"),
            Self::RemoteClose(_) | Self::EndOfStream => {
                f.write_str("Playwright Server connection closed")
            }
            Self::BinaryMessage => f.write_str("Playwright Server sent an unexpected binary frame"),
            Self::MalformedBootstrap(reason) => {
                write!(f, "Playwright Server bootstrap is malformed: {reason}")
            }
            Self::BootstrapLimit => {
                f.write_str("Playwright Server bootstrap message limit exceeded")
            }
            Self::EventQueueFull => f.write_str("Playwright Server event queue limit exceeded"),
            #[cfg(any(feature = "page-events", feature = "close-notices"))]
            Self::PageEventProjection(_) => {
                f.write_str("Playwright Server Page event projection failed")
            }
            Self::ObjectGraphLimit => {
                f.write_str("Playwright Server retained object graph limit exceeded")
            }
            Self::ObjectGraphInvalid(reason) => {
                write!(f, "Playwright Server object graph is invalid: {reason}")
            }
            Self::DeadlineExceeded => f.write_str("Playwright Server bootstrap deadline exceeded"),
            Self::SendDeadlineExceeded => {
                f.write_str("Playwright Server command send deadline exceeded")
            }
            Self::BrowserClosed => f.write_str("Remote Browser closed"),
        }
    }
}

impl std::error::Error for SessionError {}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionFailure {
    pub error: SessionError,
    pub report: Box<CloseReport>,
}

impl Display for SessionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.error, f)
    }
}

impl std::error::Error for SessionFailure {}

#[derive(Clone, Debug, PartialEq)]
pub enum SendError {
    Client(ClientError),
    Terminal(SessionFailure),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CloseOutcome {
    pub report: CloseReport,
    pub transport_error: Option<WsError>,
}

#[derive(Clone, Debug)]
struct BrowserInitializer {
    name: String,
    version: String,
}

#[derive(Default)]
struct BootstrapObjects {
    playwright: Option<(String, Value)>,
    browser_types: BTreeMap<String, String>,
    browsers: BTreeMap<String, BrowserInitializer>,
}

impl BootstrapObjects {
    fn record(&mut self, ingress: &Ingress) -> Result<(), &'static str> {
        match ingress {
            Ingress::Created {
                guid,
                object_type,
                initializer,
                ..
            } => match object_type.as_str() {
                "Playwright" => {
                    if self.playwright.is_some() {
                        return Err("multiple Playwright objects");
                    }
                    self.playwright = Some((guid.clone(), initializer.clone()));
                }
                "BrowserType" => {
                    let name = nonempty_field(initializer, "name")?;
                    initializer
                        .get("executablePath")
                        .and_then(Value::as_str)
                        .ok_or("BrowserType.executablePath missing")?;
                    self.browser_types.insert(guid.clone(), name.to_owned());
                }
                "Browser" => {
                    let name = nonempty_field(initializer, "name")?.to_owned();
                    let version = nonempty_field(initializer, "version")?.to_owned();
                    self.browsers
                        .insert(guid.clone(), BrowserInitializer { name, version });
                }
                _ => {}
            },
            Ingress::Disposed { guids, .. } => {
                for guid in guids {
                    if self
                        .playwright
                        .as_ref()
                        .is_some_and(|(known, _)| known == guid)
                    {
                        self.playwright = None;
                    }
                    self.browser_types.remove(guid);
                    self.browsers.remove(guid);
                }
            }
            Ingress::Reply { .. } | Ingress::Adopted { .. } | Ingress::Event { .. } => {}
        }
        Ok(())
    }

    fn validate(
        &self,
        core: &ClientCore,
        result: &Value,
        root_id: CommandId,
        pending: &PendingCommand,
    ) -> Result<(String, BrowserInitializer), &'static str> {
        if pending.id != root_id || !pending.guid.is_empty() || pending.method != "initialize" {
            return Err("reply is not Root.initialize");
        }
        let playwright_guid = channel_ref(result, "playwright")?;
        let (recorded_guid, initializer) = self
            .playwright
            .as_ref()
            .ok_or("Playwright object was not created")?;
        if recorded_guid != playwright_guid {
            return Err("Root.initialize references a different Playwright object");
        }
        known_type(core, playwright_guid, "Playwright")?;
        if core
            .object(playwright_guid)
            .and_then(|item| item.parent.as_deref())
            != Some("")
        {
            return Err("Playwright object is not owned by Root");
        }

        for (field, expected_name) in [
            ("chromium", "chromium"),
            ("firefox", "firefox"),
            ("webkit", "webkit"),
        ] {
            let guid = channel_ref(initializer, field)?;
            known_type(core, guid, "BrowserType")?;
            if core.object(guid).and_then(|item| item.parent.as_deref()) != Some("") {
                return Err("BrowserType is not owned by Root");
            }
            if self.browser_types.get(guid).map(String::as_str) != Some(expected_name) {
                return Err("BrowserType initializer name disagrees with Playwright");
            }
        }
        for (field, expected_type) in [("android", "Android"), ("electron", "Electron")] {
            let guid = channel_ref(initializer, field)?;
            known_type(core, guid, expected_type)?;
            if core.object(guid).and_then(|item| item.parent.as_deref()) != Some("") {
                return Err("platform dispatcher is not owned by Root");
            }
        }

        let browser_guid = channel_ref(initializer, "preLaunchedBrowser")?;
        known_type(core, browser_guid, "Browser")?;
        let browser = self
            .browsers
            .get(browser_guid)
            .ok_or("Browser initializer was not observed")?;
        let parent_guid = core
            .object(browser_guid)
            .and_then(|item| item.parent.as_deref())
            .ok_or("Browser has no parent BrowserType")?;
        known_type(core, parent_guid, "BrowserType")?;
        if self.browser_types.get(parent_guid).map(String::as_str) != Some(browser.name.as_str()) {
            return Err("Browser name disagrees with its BrowserType");
        }
        if channel_ref(initializer, &browser.name)? != parent_guid {
            return Err("Browser parent is not this Playwright BrowserType");
        }
        Ok((browser_guid.to_owned(), browser.clone()))
    }
}

fn nonempty_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, &'static str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty() && text.len() <= 256)
        .ok_or("required initializer string missing or invalid")
}

fn channel_ref<'a>(value: &'a Value, field: &str) -> Result<&'a str, &'static str> {
    value
        .get(field)
        .and_then(Value::as_object)
        .and_then(|channel| channel.get("guid"))
        .and_then(Value::as_str)
        .filter(|guid| !guid.is_empty())
        .ok_or("required channel reference missing")
}

fn known_type(core: &ClientCore, guid: &str, expected: &str) -> Result<(), &'static str> {
    let entry = core
        .object(guid)
        .ok_or("channel reference has no live object")?;
    if entry.object_type != expected {
        return Err("channel reference has wrong object type");
    }
    Ok(())
}

async fn close_transport<P: WsPeer>(transport: &mut WsTransport<P>) -> Option<WsError> {
    match timeout(CLOSE_BUDGET, transport.close()).await {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error),
        Err(_) => Some(WsError::IoFailed),
    }
}

async fn terminal<P: WsPeer>(
    transport: &mut WsTransport<P>,
    core: &mut ClientCore,
    error: SessionError,
    reason: CloseReason,
) -> SessionFailure {
    let report = core.close(reason);
    let _ = close_transport(transport).await;
    SessionFailure {
        error,
        report: Box::new(report),
    }
}

pub struct RemoteBrowserLease<P: WsPeer> {
    transport: WsTransport<P>,
    core: ClientCore,
    bootstrap_ingress: VecDeque<Ingress>,
    browser_guid: String,
    browser_name: String,
    browser_version: String,
    connected: bool,
}

impl<P: WsPeer> RemoteBrowserLease<P> {
    #[cfg(feature = "supervisor")]
    async fn fail(&mut self, error: SessionError, reason: CloseReason) -> SessionFailure {
        self.connected = false;
        self.bootstrap_ingress.clear();
        terminal(&mut self.transport, &mut self.core, error, reason).await
    }

    /// Establish a real protocol connection, without creating or launching a browser.
    ///
    /// `request` includes caller-selected endpoint, headers and bounded limits.
    /// Its connect timeout also bounds the full Root.initialize bootstrap, capped
    /// at 30 seconds. The wall clock is supplied rather than read in this crate.
    pub async fn connect<C: WsConnector<Peer = P>>(
        connector: &C,
        request: &WsRequest,
        sdk_language: SdkLanguage,
        wall_time_ms: u64,
    ) -> Result<Self, SessionFailure> {
        let mut core = ClientCore::new();
        let metadata = json!({
            "wallTime": wall_time_ms,
            "apiName": "Connection.init",
            "internal": false,
        });
        let (root_id, command) = core
            .begin_command(
                "",
                "initialize",
                &json!({"sdkLanguage": sdk_language.as_str()}),
                &metadata,
            )
            .map_err(|error| SessionFailure {
                error: SessionError::Client(error),
                report: Box::new(core.close(CloseReason::Transport)),
            })?;
        let budget = Duration::from_millis(u64::from(
            request
                .limits()
                .connect_timeout_ms()
                .min(MAX_BOOTSTRAP_BUDGET_MS),
        ));
        let deadline = Instant::now() + budget;
        let mut transport =
            match timeout_at(deadline, WsTransport::connect(connector, request)).await {
                Ok(Ok(transport)) => transport,
                Ok(Err(error)) => {
                    return Err(SessionFailure {
                        error: SessionError::WebSocket(error),
                        report: Box::new(core.close(CloseReason::Transport)),
                    });
                }
                Err(_) => {
                    return Err(SessionFailure {
                        error: SessionError::DeadlineExceeded,
                        report: Box::new(core.close(CloseReason::Transport)),
                    });
                }
            };
        match timeout_at(deadline, transport.send(WsOutgoing::Text(command))).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                return Err(terminal(
                    &mut transport,
                    &mut core,
                    SessionError::WebSocket(error),
                    CloseReason::Transport,
                )
                .await);
            }
            Err(_) => {
                return Err(terminal(
                    &mut transport,
                    &mut core,
                    SessionError::DeadlineExceeded,
                    CloseReason::Transport,
                )
                .await);
            }
        }

        let mut objects = BootstrapObjects::default();
        let mut bootstrap_ingress = VecDeque::new();
        let mut bootstrap_bytes = 0usize;
        for _ in 0..MAX_BOOTSTRAP_MESSAGES {
            let frame = match timeout_at(deadline, transport.receive()).await {
                Ok(Ok(frame)) => frame,
                Ok(Err(error)) => {
                    return Err(terminal(
                        &mut transport,
                        &mut core,
                        SessionError::WebSocket(error),
                        CloseReason::Transport,
                    )
                    .await);
                }
                Err(_) => {
                    return Err(terminal(
                        &mut transport,
                        &mut core,
                        SessionError::DeadlineExceeded,
                        CloseReason::Transport,
                    )
                    .await);
                }
            };
            match frame {
                WsIncoming::Text(text) => {
                    let ingress = match core.ingest(&text) {
                        Ok(ingress) => ingress,
                        Err(ReceiveFailure::Protocol { violation, report }) => {
                            let _ = close_transport(&mut transport).await;
                            return Err(SessionFailure {
                                error: SessionError::Protocol(violation),
                                report: Box::new(report),
                            });
                        }
                        Err(ReceiveFailure::Closed) => {
                            return Err(terminal(
                                &mut transport,
                                &mut core,
                                SessionError::BrowserClosed,
                                CloseReason::Transport,
                            )
                            .await);
                        }
                    };
                    match ingress {
                        Ingress::Reply { pending, outcome } => match outcome {
                            ReplyOutcome::Result(result) => {
                                let prelaunched_guid = objects.playwright.as_ref().and_then(
                                    |(playwright_guid, initializer)| {
                                        (pending.id == root_id
                                            && pending.guid.is_empty()
                                            && pending.method == "initialize"
                                            && channel_ref(&result, "playwright").ok()
                                                == Some(playwright_guid.as_str()))
                                        .then(|| {
                                            channel_ref(initializer, "preLaunchedBrowser").ok()
                                        })
                                        .flatten()
                                    },
                                );
                                if prelaunched_guid.is_some_and(|browser_guid| {
                                    bootstrap_ingress.iter().any(|ingress| match ingress {
                                        Ingress::Event { guid, method, .. } => {
                                            guid == browser_guid && method == "close"
                                        }
                                        Ingress::Disposed { guids, .. } => {
                                            guids.iter().any(|guid| guid == browser_guid)
                                        }
                                        _ => false,
                                    })
                                }) {
                                    return Err(terminal(
                                        &mut transport,
                                        &mut core,
                                        SessionError::BrowserClosed,
                                        CloseReason::Remote("Browser closed".into()),
                                    )
                                    .await);
                                }
                                let (browser_guid, browser) = match objects
                                    .validate(&core, &result, root_id, &pending)
                                {
                                    Ok(browser) => browser,
                                    Err(reason) => {
                                        return Err(terminal(
                                            &mut transport,
                                            &mut core,
                                            SessionError::MalformedBootstrap(reason),
                                            CloseReason::Protocol(ProtocolViolation::InvalidReply),
                                        )
                                        .await);
                                    }
                                };
                                return Ok(Self {
                                    transport,
                                    core,
                                    bootstrap_ingress,
                                    browser_guid,
                                    browser_name: browser.name,
                                    browser_version: browser.version,
                                    connected: true,
                                });
                            }
                            ReplyOutcome::Error(error) => {
                                return Err(terminal(
                                    &mut transport,
                                    &mut core,
                                    SessionError::Remote(error),
                                    CloseReason::Remote("Root.initialize failed".into()),
                                )
                                .await);
                            }
                        },
                        other => {
                            if let Err(reason) = objects.record(&other) {
                                return Err(terminal(
                                    &mut transport,
                                    &mut core,
                                    SessionError::MalformedBootstrap(reason),
                                    CloseReason::Protocol(ProtocolViolation::InvalidParams),
                                )
                                .await);
                            }
                            if bootstrap_ingress.len() >= MAX_BOOTSTRAP_BUFFERED_INGRESS
                                || text.len()
                                    > MAX_BOOTSTRAP_BUFFERED_BYTES.saturating_sub(bootstrap_bytes)
                            {
                                return Err(terminal(
                                    &mut transport,
                                    &mut core,
                                    SessionError::BootstrapLimit,
                                    CloseReason::ResourceLimit,
                                )
                                .await);
                            }
                            bootstrap_bytes += text.len();
                            bootstrap_ingress.push_back(other);
                        }
                    }
                }
                WsIncoming::RemoteClose(frame) => {
                    let reason = frame.as_ref().map_or("", WsCloseFrame::reason);
                    let report = core.remote_closed(reason);
                    let _ = close_transport(&mut transport).await;
                    return Err(SessionFailure {
                        error: SessionError::RemoteClose(frame),
                        report: Box::new(report),
                    });
                }
                WsIncoming::EndOfStream => {
                    return Err(terminal(
                        &mut transport,
                        &mut core,
                        SessionError::EndOfStream,
                        CloseReason::Remote("end of stream".into()),
                    )
                    .await);
                }
                WsIncoming::Binary(_) => {
                    return Err(terminal(
                        &mut transport,
                        &mut core,
                        SessionError::BinaryMessage,
                        CloseReason::Protocol(ProtocolViolation::InvalidEnvelope),
                    )
                    .await);
                }
            }
        }
        Err(terminal(
            &mut transport,
            &mut core,
            SessionError::BootstrapLimit,
            CloseReason::Protocol(ProtocolViolation::ObjectLimit),
        )
        .await)
    }

    pub fn guid(&self) -> &str {
        &self.browser_guid
    }

    pub fn name(&self) -> &str {
        &self.browser_name
    }

    pub fn version(&self) -> &str {
        &self.browser_version
    }

    pub fn is_connected(&self) -> bool {
        self.connected && self.core.close_reason().is_none()
    }

    pub fn pending_count(&self) -> usize {
        self.core.pending_count()
    }

    pub fn bootstrap_pending_count(&self) -> usize {
        self.bootstrap_ingress.len()
    }

    /// Low-level command surface for later typed capabilities. Callers must
    /// separately drive `receive_next` and resolve the returned correlation.
    pub async fn send_command(
        &mut self,
        guid: &str,
        method: &str,
        params: &Value,
        metadata: &Value,
    ) -> Result<CommandId, SendError> {
        let (id, frame) = self
            .core
            .begin_command(guid, method, params, metadata)
            .map_err(SendError::Client)?;
        if let Err(error) = self.transport.send(WsOutgoing::Text(frame)).await {
            self.connected = false;
            self.bootstrap_ingress.clear();
            return Err(SendError::Terminal(
                terminal(
                    &mut self.transport,
                    &mut self.core,
                    SessionError::WebSocket(error),
                    CloseReason::Transport,
                )
                .await,
            ));
        }
        Ok(id)
    }

    /// Replay one startup message, then drive one inbound message. No hidden
    /// task or background receiver exists.
    pub async fn receive_next(&mut self) -> Result<Ingress, SessionFailure> {
        if !self.is_connected() {
            self.bootstrap_ingress.clear();
            return Err(SessionFailure {
                error: SessionError::BrowserClosed,
                report: Box::new(self.core.close(CloseReason::Transport)),
            });
        }
        if let Some(ingress) = self.bootstrap_ingress.pop_front() {
            return Ok(ingress);
        }
        let frame = match self.transport.receive().await {
            Ok(frame) => frame,
            Err(error) => {
                self.connected = false;
                return Err(terminal(
                    &mut self.transport,
                    &mut self.core,
                    SessionError::WebSocket(error),
                    CloseReason::Transport,
                )
                .await);
            }
        };
        match frame {
            WsIncoming::Text(text) => match self.core.ingest(&text) {
                Ok(ingress) => {
                    let browser_closed = match &ingress {
                        Ingress::Disposed { guids, .. } => guids.contains(&self.browser_guid),
                        Ingress::Event { guid, method, .. } => {
                            guid == &self.browser_guid && method == "close"
                        }
                        _ => false,
                    };
                    if browser_closed {
                        self.connected = false;
                        let report = self
                            .core
                            .close(CloseReason::Remote("Browser closed".into()));
                        let _ = close_transport(&mut self.transport).await;
                        return Err(SessionFailure {
                            error: SessionError::BrowserClosed,
                            report: Box::new(report),
                        });
                    }
                    Ok(ingress)
                }
                Err(ReceiveFailure::Protocol { violation, report }) => {
                    self.connected = false;
                    let _ = close_transport(&mut self.transport).await;
                    Err(SessionFailure {
                        error: SessionError::Protocol(violation),
                        report: Box::new(report),
                    })
                }
                Err(ReceiveFailure::Closed) => {
                    self.connected = false;
                    Err(SessionFailure {
                        error: SessionError::BrowserClosed,
                        report: Box::new(self.core.close(CloseReason::Transport)),
                    })
                }
            },
            WsIncoming::RemoteClose(frame) => {
                self.connected = false;
                let report = self
                    .core
                    .remote_closed(frame.as_ref().map_or("", WsCloseFrame::reason));
                let _ = close_transport(&mut self.transport).await;
                Err(SessionFailure {
                    error: SessionError::RemoteClose(frame),
                    report: Box::new(report),
                })
            }
            WsIncoming::EndOfStream => {
                self.connected = false;
                Err(terminal(
                    &mut self.transport,
                    &mut self.core,
                    SessionError::EndOfStream,
                    CloseReason::Remote("end of stream".into()),
                )
                .await)
            }
            WsIncoming::Binary(_) => {
                self.connected = false;
                Err(terminal(
                    &mut self.transport,
                    &mut self.core,
                    SessionError::BinaryMessage,
                    CloseReason::Protocol(ProtocolViolation::InvalidEnvelope),
                )
                .await)
            }
        }
    }

    /// Local Browser.close closes its connection, rather than issuing a
    /// Browser.close command to the pre-launched server-side browser.
    pub async fn close(&mut self) -> CloseOutcome {
        self.connected = false;
        self.bootstrap_ingress.clear();
        let report = self.core.close(CloseReason::Transport);
        let transport_error = close_transport(&mut self.transport).await;
        CloseOutcome {
            report,
            transport_error,
        }
    }
}

#[cfg(test)]
mod tests;
