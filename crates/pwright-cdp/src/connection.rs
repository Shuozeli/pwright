use std::future::Future;
use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use pwright_core::{
    ProductionResourceError, ProductionResourceLedger, ResourceClass, ResourceError, ResourceLease,
    ResourceLedger, ResourceRequest, ResourceRequirements, ResourceScope,
};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, oneshot};
use tokio_tungstenite::tungstenite::Message;
#[cfg(feature = "cdp-handshake-headers")]
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
#[cfg(feature = "cdp-handshake-headers")]
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async_with_config};
use tracing::{debug, error, trace, warn};

use crate::event_fanout::{
    ReclaimingEventFanout, ReclaimingEventReceiver, ReclaimingFanoutMetrics,
    ReclaimingFanoutSendError, ReclaimingFanoutSubscribeError,
};
use crate::events::{CdpEvent, CdpEventReceiver};

#[cfg(feature = "large-commands")]
pub mod large_commands;
mod lifecycle_trace;
mod session_retirement;

/// Errors from the CDP connection layer.
#[derive(Debug, thiserror::Error)]
pub enum CdpError {
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("CDP error {code}: {message}")]
    Protocol { code: i64, message: String },
    #[error("Connection closed")]
    Closed,
    #[error("Native session detached")]
    SessionDetached,
    #[error("Response channel dropped")]
    ChannelDropped,
    #[error("Timeout waiting for response")]
    Timeout,
    #[error("CDP {resource} limit exceeded: {actual} exceeds {limit}")]
    LimitExceeded {
        resource: &'static str,
        actual: usize,
        limit: usize,
    },
    #[error("invalid CDP connection configuration: {0}")]
    InvalidConfiguration(&'static str),
    #[error("CDP command identifier space was exhausted")]
    IdentifierExhausted,
    #[error("CDP resource exhaustion: {0}")]
    Resource(#[from] ResourceError),
    #[error("Element not found for selector: {selector}")]
    ElementNotFound { selector: String },
    #[error("Navigation failed for {url}: {reason}")]
    NavigationFailed { url: String, reason: String },
    #[error("source error: {source}; system error: {system}")]
    Compound {
        source: Box<CdpError>,
        system: Box<CdpError>,
    },
    #[error("Page is closed")]
    PageClosed,
    #[error("Tab not found: {0}")]
    TabNotFound(String),
    #[error("HTTP request failed: {0}")]
    HttpFailed(String),
    #[error("JavaScript exception: {0}")]
    JsException(String),
    #[error("{0}")]
    Other(String),
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CdpEventSubscriptionError {
    #[error("this CDP connection was not configured for reclaiming event fanout")]
    ReclaimingFanoutNotConfigured,
    #[error(transparent)]
    ReclaimingFanout(#[from] ReclaimingFanoutSubscribeError),
}

impl From<tokio_tungstenite::tungstenite::Error> for CdpError {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        CdpError::WebSocket(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CdpError>;

/// Bounded, validated headers for the CDP WebSocket upgrade. Header values
/// are deliberately omitted from Debug and validation errors.
#[cfg(feature = "cdp-handshake-headers")]
#[derive(Clone, Default)]
pub struct CdpHandshakeHeaders(Vec<(HeaderName, HeaderValue)>);

#[cfg(feature = "cdp-handshake-headers")]
pub const CDP_HANDSHAKE_HEADER_MAX_COUNT: usize = 32;
#[cfg(feature = "cdp-handshake-headers")]
pub const CDP_HANDSHAKE_HEADER_MAX_NAME_BYTES: usize = 256;
#[cfg(feature = "cdp-handshake-headers")]
pub const CDP_HANDSHAKE_HEADER_MAX_VALUE_BYTES: usize = 4096;
#[cfg(feature = "cdp-handshake-headers")]
pub const CDP_HANDSHAKE_HEADER_MAX_BATCH_BYTES: usize = 8192;

#[cfg(feature = "cdp-handshake-headers")]
impl std::fmt::Debug for CdpHandshakeHeaders {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CdpHandshakeHeaders")
            .field("count", &self.0.len())
            .finish()
    }
}

#[cfg(feature = "cdp-handshake-headers")]
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CdpHandshakeHeadersError {
    #[error("CDP handshake header count exceeds 32")]
    TooMany,
    #[error("CDP handshake header name exceeds 256 bytes")]
    NameTooLong,
    #[error("CDP handshake header value exceeds 4096 bytes")]
    ValueTooLong,
    #[error("CDP handshake headers exceed 8192 bytes")]
    BatchTooLarge,
    #[error("invalid CDP handshake header name")]
    InvalidName,
    #[error("invalid CDP handshake header value")]
    InvalidValue,
}

#[cfg(feature = "cdp-handshake-headers")]
impl CdpHandshakeHeadersError {
    pub const fn is_limit(self) -> bool {
        matches!(
            self,
            Self::TooMany | Self::NameTooLong | Self::ValueTooLong | Self::BatchTooLarge
        )
    }
}

#[cfg(feature = "cdp-handshake-headers")]
impl CdpHandshakeHeaders {
    pub fn new(
        pairs: Vec<(String, String)>,
    ) -> std::result::Result<Self, CdpHandshakeHeadersError> {
        if pairs.len() > CDP_HANDSHAKE_HEADER_MAX_COUNT {
            return Err(CdpHandshakeHeadersError::TooMany);
        }
        let mut total_bytes = 0usize;
        let mut parsed = Vec::with_capacity(pairs.len());
        for (name, value) in pairs {
            if name.len() > CDP_HANDSHAKE_HEADER_MAX_NAME_BYTES {
                return Err(CdpHandshakeHeadersError::NameTooLong);
            }
            if value.len() > CDP_HANDSHAKE_HEADER_MAX_VALUE_BYTES {
                return Err(CdpHandshakeHeadersError::ValueTooLong);
            }
            total_bytes = total_bytes
                .checked_add(name.len())
                .and_then(|sum| sum.checked_add(value.len()))
                .ok_or(CdpHandshakeHeadersError::BatchTooLarge)?;
            if total_bytes > CDP_HANDSHAKE_HEADER_MAX_BATCH_BYTES {
                return Err(CdpHandshakeHeadersError::BatchTooLarge);
            }
            let name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| CdpHandshakeHeadersError::InvalidName)?;
            let value = HeaderValue::from_str(&value)
                .map_err(|_| CdpHandshakeHeadersError::InvalidValue)?;
            parsed.push((name, value));
        }
        Ok(Self(parsed))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.0.iter().map(|(name, value)| (name, value))
    }
}

type CommandResult = std::result::Result<Value, CdpError>;

struct CommandResource {
    _lease: ResourceLease,
    #[cfg(feature = "large-commands")]
    large: Option<Arc<large_commands::CommandLease>>,
}

struct CommandAdmission {
    max_bytes: usize,
    #[cfg(feature = "large-commands")]
    large: Option<Arc<large_commands::CommandLease>>,
}

struct RetainedCommandResult {
    result: CommandResult,
    _resource: Option<Arc<CommandResource>>,
}

struct PendingCommandSlot {
    active: Arc<AtomicUsize>,
}

impl Drop for PendingCommandSlot {
    fn drop(&mut self) {
        let previous = self.active.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

struct PendingEntry {
    response: oneshot::Sender<RetainedCommandResult>,
    _slot: PendingCommandSlot,
    resource: Option<Arc<CommandResource>>,
    session_id: Option<Box<str>>,
}

type PendingMap = Arc<DashMap<u64, PendingEntry>>;

struct PendingRegistration {
    id: u64,
    pending: PendingMap,
}

impl Drop for PendingRegistration {
    fn drop(&mut self) {
        self.pending.remove(&self.id);
    }
}

struct PreparedCommand {
    raw: String,
    response: oneshot::Receiver<RetainedCommandResult>,
    _registration: PendingRegistration,
    resource: Option<Arc<CommandResource>>,
}

/// A command admitted to the ordered CDP write queue. The caller must retain
/// this handle through its response to preserve the pending-command owner.
pub struct EnqueuedCommand {
    response: EnqueuedResponse,
    _registration: PendingRegistration,
    response_timeout: Option<Duration>,
}

enum EnqueuedResponse {
    Pending(oneshot::Receiver<RetainedCommandResult>),
    Settled(Value),
}

impl EnqueuedCommand {
    pub async fn response(self) -> Result<Value> {
        let Self {
            response,
            _registration,
            response_timeout,
        } = self;
        let response = match response {
            EnqueuedResponse::Settled(value) => return Ok(value),
            EnqueuedResponse::Pending(response) => response,
        };
        let Some(timeout) = response_timeout else {
            return response.await.map_err(|_| CdpError::ChannelDropped)?.result;
        };
        match tokio::time::timeout(timeout, response).await {
            Ok(Ok(retained)) => retained.result,
            Ok(Err(_)) => Err(CdpError::ChannelDropped),
            Err(_) => Err(CdpError::Timeout),
        }
    }
}

struct OutboundMessage {
    message: Message,
    _resource: Option<Arc<CommandResource>>,
}

#[derive(Clone, Debug)]
struct CdpEventDelivery {
    mode: Arc<Mutex<Option<CdpEventDeliveryMode>>>,
}

#[derive(Debug)]
enum CdpEventDeliveryMode {
    Legacy(broadcast::Sender<CdpEvent>),
    Reclaiming(ReclaimingEventFanout<CdpEvent>),
}

impl CdpEventDelivery {
    fn legacy(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            mode: Arc::new(Mutex::new(Some(CdpEventDeliveryMode::Legacy(sender)))),
        }
    }

    fn reclaiming(event_capacity: usize, subscriber_capacity: usize) -> Self {
        let fanout = ReclaimingEventFanout::new(event_capacity, subscriber_capacity)
            .expect("validated production fanout limits must remain nonzero");
        Self {
            mode: Arc::new(Mutex::new(Some(CdpEventDeliveryMode::Reclaiming(fanout)))),
        }
    }

    fn subscribe_legacy(&self) -> Option<broadcast::Receiver<CdpEvent>> {
        let mode = self
            .mode
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match mode.as_ref() {
            Some(CdpEventDeliveryMode::Legacy(sender)) => Some(sender.subscribe()),
            Some(CdpEventDeliveryMode::Reclaiming(_)) | None => None,
        }
    }

    fn subscribe_reclaiming(
        &self,
    ) -> std::result::Result<ReclaimingEventReceiver<CdpEvent>, CdpEventSubscriptionError> {
        let mode = self
            .mode
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match mode.as_ref() {
            Some(CdpEventDeliveryMode::Reclaiming(fanout)) => {
                fanout.subscribe().map_err(CdpEventSubscriptionError::from)
            }
            Some(CdpEventDeliveryMode::Legacy(_)) => {
                Err(CdpEventSubscriptionError::ReclaimingFanoutNotConfigured)
            }
            None => Err(CdpEventSubscriptionError::ReclaimingFanout(
                ReclaimingFanoutSubscribeError::Closed,
            )),
        }
    }

    fn reclaiming_metrics(&self) -> Option<ReclaimingFanoutMetrics> {
        let mode = self
            .mode
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match mode.as_ref() {
            Some(CdpEventDeliveryMode::Reclaiming(fanout)) => Some(fanout.metrics()),
            Some(CdpEventDeliveryMode::Legacy(_)) | None => None,
        }
    }

    fn send(&self, event: CdpEvent) -> std::result::Result<usize, CdpEvent> {
        let mode = self
            .mode
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match mode.as_ref() {
            Some(CdpEventDeliveryMode::Legacy(sender)) => match sender.send(event) {
                Ok(receivers) => Ok(receivers),
                Err(error) => {
                    drop(error.0);
                    Ok(0)
                }
            },
            Some(CdpEventDeliveryMode::Reclaiming(fanout)) => {
                fanout.send(event).map_err(|error| match error {
                    ReclaimingFanoutSendError::Closed(event)
                    | ReclaimingFanoutSendError::SequenceExhausted(event) => event,
                })
            }
            None => Err(event),
        }
    }

    fn close(&self) {
        let mode = self
            .mode
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(CdpEventDeliveryMode::Reclaiming(fanout)) = mode {
            fanout.close();
        }
    }
}

#[derive(Clone)]
struct ConnectionResourceBudget {
    ledger: ResourceLedger,
    profile: CdpConnectionProductionProfile,
}

#[derive(Debug, thiserror::Error)]
pub enum CdpProductionConnectError {
    #[error(transparent)]
    Production(#[from] ProductionResourceError),
    #[error(transparent)]
    Connection(#[from] CdpError),
}

/// Raw CDP WebSocket connection -- one per browser.
///
/// Manages the WebSocket transport, message ID allocation,
/// and dispatches responses/events to the appropriate receivers.
///
/// When dropped, aborts the reader/writer tasks and closes the WebSocket.
pub struct CdpConnection {
    write_tx: tokio::sync::mpsc::Sender<OutboundMessage>,
    next_id: AtomicU64,
    pending: PendingMap,
    active_pending: Arc<AtomicUsize>,
    pending_command_capacity: usize,
    event_delivery: CdpEventDelivery,
    default_timeout: Duration,
    max_command_bytes: usize,
    #[cfg(feature = "large-commands")]
    large_commands: Option<large_commands::Pool>,
    resources: Option<ConnectionResourceBudget>,
    _reader_resource: Option<ResourceLease>,
    _event_delivery_resource: Option<ResourceLease>,
    #[cfg(feature = "cdp-ws-close-reason")]
    peer_close_reason: Arc<Mutex<Option<Box<str>>>>,
    shutdown: tokio_util::sync::CancellationToken,
    io_tasks: tokio_util::task::TaskTracker,
}

impl Drop for CdpConnection {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

/// Default command timeout (30 seconds).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
/// Default write channel capacity.
const DEFAULT_WRITE_CHANNEL_CAPACITY: usize = 256;
/// Default event broadcast channel capacity.
const DEFAULT_EVENT_CHANNEL_CAPACITY: usize = 1024;
/// Default number of commands awaiting write or response.
const DEFAULT_PENDING_COMMAND_CAPACITY: usize = 256;
/// CDP can return base64-encoded binary bodies in one WebSocket frame. Keep
/// the trusted-browser transport bounded while allowing a 50 MiB raw capture
/// plus base64 and JSON overhead.
const MAX_CDP_WEBSOCKET_BYTES: usize = 128 << 20;

fn command_response_timeout(timeout: Duration) -> Option<Duration> {
    (!timeout.is_zero()).then_some(timeout)
}

/// Configuration for a CDP connection.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConnectionConfig {
    /// Timeout for individual CDP commands. Zero disables the response timer.
    pub timeout: Duration,
    /// Capacity of the write channel (commands queued to send).
    pub write_channel_capacity: usize,
    /// Capacity of the event broadcast channel.
    pub event_channel_capacity: usize,
    /// Maximum commands that may be queued or awaiting a response.
    pub pending_command_capacity: usize,
    /// Maximum serialized size of one outgoing command.
    pub max_command_bytes: usize,
    /// Maximum serialized size of one incoming command response.
    pub max_response_bytes: usize,
    /// Maximum serialized size of one incoming event.
    pub max_event_bytes: usize,
    /// Maximum WebSocket frame and reassembled message size.
    pub max_websocket_bytes: usize,
    /// Maximum retained text in a CDP protocol error.
    pub max_protocol_error_bytes: usize,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            write_channel_capacity: DEFAULT_WRITE_CHANNEL_CAPACITY,
            event_channel_capacity: DEFAULT_EVENT_CHANNEL_CAPACITY,
            pending_command_capacity: DEFAULT_PENDING_COMMAND_CAPACITY,
            max_command_bytes: MAX_CDP_WEBSOCKET_BYTES,
            max_response_bytes: MAX_CDP_WEBSOCKET_BYTES,
            max_event_bytes: MAX_CDP_WEBSOCKET_BYTES,
            max_websocket_bytes: MAX_CDP_WEBSOCKET_BYTES,
            max_protocol_error_bytes: 64 * 1024,
        }
    }
}

impl ConnectionConfig {
    fn validate(&self) -> Result<()> {
        for (name, value) in [
            (
                "write channel capacity must be nonzero",
                self.write_channel_capacity,
            ),
            (
                "event channel capacity must be nonzero",
                self.event_channel_capacity,
            ),
            (
                "pending command capacity must be nonzero",
                self.pending_command_capacity,
            ),
            (
                "maximum command bytes must be nonzero",
                self.max_command_bytes,
            ),
            (
                "maximum response bytes must be nonzero",
                self.max_response_bytes,
            ),
            ("maximum event bytes must be nonzero", self.max_event_bytes),
            (
                "maximum WebSocket bytes must be nonzero",
                self.max_websocket_bytes,
            ),
            (
                "maximum protocol error bytes must be nonzero",
                self.max_protocol_error_bytes,
            ),
        ] {
            if value == 0 {
                return Err(CdpError::InvalidConfiguration(name));
            }
        }
        if self.max_response_bytes > self.max_websocket_bytes {
            return Err(CdpError::InvalidConfiguration(
                "maximum response bytes exceed the WebSocket limit",
            ));
        }
        if self.max_command_bytes > self.max_websocket_bytes {
            return Err(CdpError::InvalidConfiguration(
                "maximum command bytes exceed the WebSocket limit",
            ));
        }
        if self.max_event_bytes > self.max_websocket_bytes {
            return Err(CdpError::InvalidConfiguration(
                "maximum event bytes exceed the WebSocket limit",
            ));
        }
        Ok(())
    }
}

/// Resource declaration for one production CDP WebSocket connection.
///
/// The connection holds one fixed reader lease for the maximum raw-message and
/// parsed-JSON overlap. Each admitted command reserves its complete outgoing
/// construction or incoming-result peak. Each distinct event owns an exact
/// item/byte lease shared by every subscriber. The declaration includes one
/// additional event because input allocation precedes bounded queue admission.
/// Reclaim-after-all-read fanout is an explicit opt-in that additionally
/// declares its fixed slot/subscriber bookkeeping and one retained lease.
#[derive(Clone, Debug)]
pub struct CdpConnectionProductionProfile {
    resource_scope: ResourceScope,
    config: ConnectionConfig,
    reader_wire_bytes: u64,
    command_wire_bytes: u64,
    retained_event_bytes: u64,
    reclaiming_event_subscribers: Option<usize>,
    event_delivery_resource_bytes: u64,
    requirements: ResourceRequirements,
    event_admission: Option<crate::CdpEventAdmissionPolicy>,
    pending_session_id_bytes: Option<usize>,
    #[cfg(feature = "large-commands")]
    large_commands: Option<large_commands::LargeCommandProfile>,
}

impl CdpConnectionProductionProfile {
    pub fn new(resource_scope: ResourceScope, config: ConnectionConfig) -> Self {
        let reader_wire_bytes = usize_to_u64(config.max_websocket_bytes).saturating_mul(2);
        let command_wire_bytes = usize_to_u64(config.max_command_bytes)
            .saturating_mul(2)
            .max(usize_to_u64(config.max_response_bytes));
        let pending = usize_to_u64(config.pending_command_capacity);
        let events = usize_to_u64(config.event_channel_capacity).saturating_add(1);
        let wire_bytes =
            reader_wire_bytes.saturating_add(pending.saturating_mul(command_wire_bytes));
        let event_bytes = events.saturating_mul(usize_to_u64(config.max_event_bytes));
        let retained_event_bytes = usize_to_u64(config.event_channel_capacity)
            .saturating_mul(usize_to_u64(config.max_event_bytes));
        let leases = config
            .pending_command_capacity
            .saturating_add(config.event_channel_capacity)
            .saturating_add(2);
        let requirements = ResourceRequirements::none()
            .with_class(ResourceClass::WireBytes, wire_bytes, wire_bytes)
            .with_class(ResourceClass::EventItem, events, events)
            .with_class(ResourceClass::EventBytes, event_bytes, event_bytes)
            .with_metadata(1, leases);
        Self {
            resource_scope,
            config,
            reader_wire_bytes,
            command_wire_bytes,
            retained_event_bytes,
            reclaiming_event_subscribers: None,
            event_delivery_resource_bytes: 0,
            requirements,
            event_admission: None,
            pending_session_id_bytes: None,
            #[cfg(feature = "large-commands")]
            large_commands: None,
        }
    }

    /// Replaces the conservative `event item capacity * maximum event size`
    /// retained-payload declaration with an independent aggregate byte bound.
    ///
    /// The item limit still bounds the number of queued events. The byte limit
    /// may reject the connection earlier when many large events are retained;
    /// one additional maximum-size event remains declared so the reader can
    /// parse and admit the next item before bounded fanout replacement.
    pub fn with_retained_event_byte_capacity(mut self, retained_bytes: u64) -> Result<Self> {
        let one_event = usize_to_u64(self.config.max_event_bytes);
        if retained_bytes < one_event {
            return Err(CdpError::InvalidConfiguration(
                "retained event byte capacity is smaller than one maximum-size event",
            ));
        }
        let event_bytes =
            retained_bytes
                .checked_add(one_event)
                .ok_or(CdpError::InvalidConfiguration(
                    "retained event byte capacity overflow",
                ))?;
        self.requirements =
            self.requirements
                .with_class(ResourceClass::EventBytes, event_bytes, event_bytes);
        self.retained_event_bytes = retained_bytes;
        Ok(self)
    }

    pub fn with_reclaiming_event_fanout(mut self, max_subscribers: usize) -> Result<Self> {
        if max_subscribers == 0 {
            return Err(CdpError::InvalidConfiguration(
                "reclaiming event subscriber capacity must be nonzero",
            ));
        }
        if self.reclaiming_event_subscribers.is_some() {
            return Err(CdpError::InvalidConfiguration(
                "reclaiming event fanout was configured more than once",
            ));
        }
        let retained = ReclaimingEventFanout::<CdpEvent>::maximum_retained_bookkeeping_bytes(
            self.config.event_channel_capacity,
            max_subscribers,
        );
        let existing = self.requirements.class(ResourceClass::BackendStateBytes);
        self.requirements = self
            .requirements
            .with_class(
                ResourceClass::BackendStateBytes,
                existing.global.saturating_add(retained),
                existing.per_scope.saturating_add(retained),
            )
            .with_metadata(
                self.requirements.scopes,
                self.requirements.leases.saturating_add(1),
            );
        self.reclaiming_event_subscribers = Some(max_subscribers);
        self.event_delivery_resource_bytes = retained;
        Ok(self)
    }

    /// Retire already-admitted calls when Chrome announces their exact session
    /// detached. No session registry, tombstone history or event subscriber is
    /// installed. Commands admitted after that event retain native validation.
    pub fn with_pending_session_retirement(mut self, max_native_id_bytes: usize) -> Result<Self> {
        if max_native_id_bytes == 0 || max_native_id_bytes > self.config.max_command_bytes {
            return Err(CdpError::InvalidConfiguration(
                "invalid pending session ID bound",
            ));
        }
        if self.pending_session_id_bytes.is_some() {
            return Err(CdpError::InvalidConfiguration(
                "pending session retirement already selected",
            ));
        }
        // Each command's existing lease owns the compact ID and inline metadata,
        // including overlap with its outgoing bytes or retained native response.
        let retained = usize_to_u64(max_native_id_bytes)
            .saturating_add(usize_to_u64(std::mem::size_of::<Option<Box<str>>>()));
        self.command_wire_bytes = self.command_wire_bytes.saturating_add(retained);
        let added = usize_to_u64(self.config.pending_command_capacity).saturating_mul(retained);
        let existing = self.requirements.class(ResourceClass::WireBytes);
        self.requirements = self.requirements.with_class(
            ResourceClass::WireBytes,
            existing.global.saturating_add(added),
            existing.per_scope.saturating_add(added),
        );
        self.pending_session_id_bytes = Some(max_native_id_bytes);
        Ok(self)
    }

    /// Select a stateless policy before event item/byte admission and fanout.
    /// Raw-message and per-event byte ceilings still apply before selection.
    /// This changes the connection-wide event contract for ALL consumers; the
    /// composition must select a policy compatible with every installed leaf.
    pub fn with_event_admission_policy(
        mut self,
        policy: crate::CdpEventAdmissionPolicy,
    ) -> Result<Self> {
        if self.event_admission.is_some() {
            return Err(CdpError::InvalidConfiguration(
                "event admission policy was configured more than once",
            ));
        }
        self.event_admission = Some(policy);
        Ok(self)
    }

    pub const fn event_admission_policy(&self) -> Option<crate::CdpEventAdmissionPolicy> {
        self.event_admission
    }

    pub const fn resource_scope(&self) -> ResourceScope {
        self.resource_scope
    }

    pub const fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    pub const fn reader_wire_bytes(&self) -> u64 {
        self.reader_wire_bytes
    }

    pub const fn command_wire_bytes(&self) -> u64 {
        self.command_wire_bytes
    }

    pub const fn reclaiming_event_subscribers(&self) -> Option<usize> {
        self.reclaiming_event_subscribers
    }

    pub const fn retained_event_bytes(&self) -> u64 {
        self.retained_event_bytes
    }

    pub const fn event_delivery_resource_bytes(&self) -> u64 {
        self.event_delivery_resource_bytes
    }

    pub const fn requirements(&self) -> ResourceRequirements {
        self.requirements
    }
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

impl CdpConnection {
    /// Resolves when this concrete browser-protocol transport is stopping.
    /// Await io_finished separately to prove both I/O tasks have retired.
    ///
    /// The returned future owns only a cancellation-token clone. It does not
    /// keep the connection or browser alive, and transport loss is terminal:
    /// callers must not use this signal to rebind existing sessions to a new
    /// WebSocket.
    pub fn closed(&self) -> impl Future<Output = ()> + Send + 'static {
        let shutdown = self.shutdown.clone();
        async move { shutdown.cancelled().await }
    }

    pub fn is_closed(&self) -> bool {
        self.shutdown.is_cancelled()
    }

    /// A bounded reason carried by the peer's WebSocket close frame, if any.
    /// Local cancellation, EOF, and transport errors do not fabricate one.
    #[cfg(feature = "cdp-ws-close-reason")]
    pub fn peer_close_reason(&self) -> Option<String> {
        self.peer_close_reason
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|reason| reason.to_string())
    }

    /// Terminally stop this client's I/O and reject pending/new commands.
    /// This sends no Browser.close or target/context disposal command. Pending
    /// writes retain their backing until the I/O tasks actually retire; await
    /// io_finished for that boundary. Transport loss is not native cleanup proof.
    pub fn request_disconnect(&self) {
        self.shutdown.cancel();
        fail_all_pending(&self.pending);
    }

    /// Waits for both native I/O tasks to finish cleanup, not merely receive
    /// cancellation. This owned future retains no connection or WebSocket.
    /// Request disconnect, drop the owner, or wait for peer loss before awaiting it.
    pub fn io_finished(&self) -> impl Future<Output = ()> + Send + 'static {
        let tasks = self.io_tasks.clone();
        async move { tasks.wait().await }
    }

    /// Connect to a Chrome DevTools Protocol endpoint via WebSocket.
    pub async fn connect(ws_url: &str) -> Result<Arc<Self>> {
        Self::connect_with_config(ws_url, ConnectionConfig::default()).await
    }

    /// Connect with a custom command timeout.
    pub async fn connect_with_timeout(ws_url: &str, timeout: Duration) -> Result<Arc<Self>> {
        Self::connect_with_config(
            ws_url,
            ConnectionConfig {
                timeout,
                ..Default::default()
            },
        )
        .await
    }

    /// Connect with full configuration.
    pub async fn connect_with_config(ws_url: &str, config: ConnectionConfig) -> Result<Arc<Self>> {
        let event_delivery = CdpEventDelivery::legacy(config.event_channel_capacity);
        Self::connect_with_parts(
            ws_url,
            config,
            event_delivery,
            None,
            None,
            None,
            #[cfg(feature = "cdp-handshake-headers")]
            None,
        )
        .await
    }

    /// Attach with caller-supplied WebSocket upgrade headers. This opt-in
    /// transport option does not change the CDP command or event profile.
    #[cfg(feature = "cdp-handshake-headers")]
    pub async fn connect_with_config_and_headers(
        ws_url: &str,
        config: ConnectionConfig,
        headers: &CdpHandshakeHeaders,
    ) -> Result<Arc<Self>> {
        let event_delivery = CdpEventDelivery::legacy(config.event_channel_capacity);
        Self::connect_with_parts(
            ws_url,
            config,
            event_delivery,
            None,
            None,
            None,
            Some(headers),
        )
        .await
    }

    /// Connect using an explicitly declared production resource profile.
    pub async fn connect_with_production_resources(
        ws_url: &str,
        profile: CdpConnectionProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> std::result::Result<Arc<Self>, CdpProductionConnectError> {
        Self::connect_production_inner(
            ws_url,
            profile,
            resources,
            #[cfg(feature = "cdp-handshake-headers")]
            None,
        )
        .await
    }

    #[cfg(feature = "cdp-handshake-headers")]
    pub async fn connect_with_production_resources_and_headers(
        ws_url: &str,
        profile: CdpConnectionProductionProfile,
        resources: &ProductionResourceLedger,
        headers: &CdpHandshakeHeaders,
    ) -> std::result::Result<Arc<Self>, CdpProductionConnectError> {
        Self::connect_production_inner(ws_url, profile, resources, Some(headers)).await
    }

    async fn connect_production_inner(
        ws_url: &str,
        profile: CdpConnectionProductionProfile,
        resources: &ProductionResourceLedger,
        #[cfg(feature = "cdp-handshake-headers")] headers: Option<&CdpHandshakeHeaders>,
    ) -> std::result::Result<Arc<Self>, CdpProductionConnectError> {
        profile.config.validate()?;
        let ledger = resources.claim(profile.requirements())?;
        let reader_resource = ledger
            .reserve(
                profile.resource_scope(),
                ResourceClass::WireBytes,
                profile.reader_wire_bytes(),
            )
            .map_err(CdpError::from)?;
        let event_delivery_resource = if profile.event_delivery_resource_bytes() == 0 {
            None
        } else {
            Some(
                ledger
                    .reserve(
                        profile.resource_scope(),
                        ResourceClass::BackendStateBytes,
                        profile.event_delivery_resource_bytes(),
                    )
                    .map_err(CdpError::from)?,
            )
        };
        let config = profile.config.clone();
        let event_delivery = match profile.reclaiming_event_subscribers() {
            Some(subscribers) => {
                CdpEventDelivery::reclaiming(config.event_channel_capacity, subscribers)
            }
            None => CdpEventDelivery::legacy(config.event_channel_capacity),
        };
        let budget = ConnectionResourceBudget { ledger, profile };
        Ok(Self::connect_with_parts(
            ws_url,
            config,
            event_delivery,
            Some(budget),
            Some(reader_resource),
            event_delivery_resource,
            #[cfg(feature = "cdp-handshake-headers")]
            headers,
        )
        .await?)
    }

    async fn connect_with_parts(
        ws_url: &str,
        config: ConnectionConfig,
        event_delivery: CdpEventDelivery,
        resources: Option<ConnectionResourceBudget>,
        reader_resource: Option<ResourceLease>,
        event_delivery_resource: Option<ResourceLease>,
        #[cfg(feature = "cdp-handshake-headers")] headers: Option<&CdpHandshakeHeaders>,
    ) -> Result<Arc<Self>> {
        config.validate()?;
        #[cfg(feature = "large-commands")]
        let large_commands = resources
            .as_ref()
            .and_then(|budget| {
                budget
                    .profile
                    .large_commands
                    .map(|profile| large_commands::Pool::new(profile, &budget.ledger))
            })
            .transpose()?;
        debug!(url = ws_url, "connecting to CDP WebSocket");

        let websocket_config = cdp_websocket_config(config.max_websocket_bytes);
        #[cfg(feature = "cdp-handshake-headers")]
        let (ws_stream, _response) =
            if let Some(headers) = headers.filter(|headers| !headers.is_empty()) {
                let mut request = ws_url.into_client_request()?;
                for (name, value) in headers.iter() {
                    request.headers_mut().insert(name.clone(), value.clone());
                }
                connect_async_with_config(request, Some(websocket_config), false).await?
            } else {
                connect_async_with_config(ws_url, Some(websocket_config), false).await?
            };
        #[cfg(not(feature = "cdp-handshake-headers"))]
        let (ws_stream, _response) =
            connect_async_with_config(ws_url, Some(websocket_config), false).await?;
        let (ws_write, ws_read) = ws_stream.split();

        let (write_tx, write_rx) =
            tokio::sync::mpsc::channel::<OutboundMessage>(config.write_channel_capacity);
        let pending: PendingMap = Arc::new(DashMap::new());
        let active_pending = Arc::new(AtomicUsize::new(0));
        let shutdown = tokio_util::sync::CancellationToken::new();
        let io_tasks = tokio_util::task::TaskTracker::new();

        // Writer task: forwards messages from the channel to the WebSocket
        let shutdown_w = shutdown.clone();
        let shutdown_writer_failsafe = shutdown.clone();
        io_tasks.spawn(async move {
            tokio::select! {
                _ = shutdown_w.cancelled() => {}
                _ = Self::writer_loop(ws_write, write_rx) => {}
            }
            shutdown_writer_failsafe.cancel();
        });

        // Reader task: routes incoming messages to pending responses or event broadcast
        let pending_clone = pending.clone();
        let reader_event_delivery = event_delivery.clone();
        let shutdown_r = shutdown.clone();
        let shutdown_failsafe = shutdown.clone();
        let pending_failsafe = Arc::clone(&pending);
        #[cfg(feature = "cdp-ws-close-reason")]
        let peer_close_reason = Arc::new(Mutex::new(None));
        #[cfg(feature = "cdp-ws-close-reason")]
        let reader_close_reason = Arc::clone(&peer_close_reason);
        let max_response_bytes = config.max_response_bytes;
        let max_event_bytes = config.max_event_bytes;
        let max_protocol_error_bytes = config.max_protocol_error_bytes;
        let reader_resources = resources.clone();
        io_tasks.spawn(async move {
            let close_reason = tokio::select! {
                _ = shutdown_r.cancelled() => None,
                reason = Self::reader_loop(
                    ws_read,
                    pending_clone,
                    reader_event_delivery.clone(),
                    max_response_bytes,
                    max_event_bytes,
                    max_protocol_error_bytes,
                    reader_resources,
                ) => reason,
            };
            #[cfg(feature = "cdp-ws-close-reason")]
            if let Some(reason) = close_reason {
                *reader_close_reason
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(reason);
            }
            #[cfg(not(feature = "cdp-ws-close-reason"))]
            let _ = close_reason;
            // The selected future may be dropped on cancellation, so cleanup
            // must live outside reader_loop and run on every exit path.
            fail_all_pending(&pending_failsafe);
            reader_event_delivery.close();
            shutdown_failsafe.cancel();
        });

        io_tasks.close();
        let conn = Arc::new(Self {
            write_tx,
            next_id: AtomicU64::new(1),
            pending,
            active_pending,
            pending_command_capacity: config.pending_command_capacity,
            event_delivery,
            default_timeout: config.timeout,
            max_command_bytes: config.max_command_bytes,
            #[cfg(feature = "large-commands")]
            large_commands,
            resources,
            _reader_resource: reader_resource,
            _event_delivery_resource: event_delivery_resource,
            #[cfg(feature = "cdp-ws-close-reason")]
            peer_close_reason,
            shutdown,
            io_tasks,
        });

        debug!("CDP WebSocket connected");
        Ok(conn)
    }

    /// Send a CDP command (browser-level, no session).
    pub fn send_command(
        &self,
        method: &str,
        params: Value,
    ) -> impl Future<Output = Result<Value>> + Send + '_ {
        self.send_command_with_session(method, params, None)
    }

    /// Send a CDP command, optionally scoped to a session.
    pub fn send_command_with_session(
        &self,
        method: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> impl Future<Output = Result<Value>> + Send + '_ {
        self.send_command_with_response_timeout(
            method,
            params,
            session_id,
            command_response_timeout(self.default_timeout),
        )
    }

    /// Wait only for bounded write-queue admission, then return the separately
    /// retained response owner. This is for an opt-in pre-resume command fence:
    /// acknowledgement may depend on the paused target's navigation or dialog.
    pub async fn enqueue_command_with_session(
        &self,
        method: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> Result<EnqueuedCommand> {
        let prepared = self.prepare_command(method, params, session_id)?;
        let timeout = if self.default_timeout.is_zero() {
            DEFAULT_TIMEOUT
        } else {
            self.default_timeout
        };
        tokio::time::timeout(
            timeout,
            self.enqueue_prepared(prepared, command_response_timeout(self.default_timeout)),
        )
        .await
        .map_err(|_| CdpError::Timeout)?
    }

    /// Retain the response registration until protocol settlement, without the
    /// ordinary command timer. A composing continuation must retain this future
    /// when its public waiter times out or is cancelled. Dropping this future
    /// still removes its registration and is not native cancellation evidence.
    /// Transport failure is an error, not proof that browser objects retired.
    pub fn send_command_to_settlement_with_session(
        &self,
        method: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> impl Future<Output = Result<Value>> + Send + '_ {
        self.send_command_with_response_timeout(method, params, session_id, None)
    }

    fn send_command_with_response_timeout(
        &self,
        method: &str,
        params: Value,
        session_id: Option<&str>,
        response_timeout: Option<Duration>,
    ) -> impl Future<Output = Result<Value>> + Send + '_ {
        let prepared = self.prepare_command(method, params, session_id);
        self.send_prepared(prepared, response_timeout)
    }

    async fn send_prepared(
        &self,
        prepared: Result<PreparedCommand>,
        response_timeout: Option<Duration>,
    ) -> Result<Value> {
        self.enqueue_prepared(prepared?, response_timeout)
            .await?
            .response()
            .await
    }

    async fn enqueue_prepared(
        &self,
        prepared: PreparedCommand,
        response_timeout: Option<Duration>,
    ) -> Result<EnqueuedCommand> {
        if self.is_closed() {
            return Err(CdpError::Closed);
        }
        let PreparedCommand {
            raw,
            mut response,
            _registration,
            resource,
        } = prepared;

        let enqueue = self.write_tx.send(OutboundMessage {
            message: Message::Text(raw.into()),
            _resource: resource,
        });
        if self
            .resources
            .as_ref()
            .is_some_and(|resources| resources.profile.pending_session_id_bytes.is_some())
        {
            // Detach also settles calls not yet polled or waiting for write
            // admission. Dropping enqueue releases unsent wire ownership.
            tokio::select! {
                biased;
                result = &mut response => return match result.map_err(|_| CdpError::ChannelDropped)?.result {
                    Err(error) => Err(error),
                    Ok(value) => Ok(EnqueuedCommand {
                        response: EnqueuedResponse::Settled(value),
                        _registration,
                        response_timeout,
                    }),
                },
                sent = enqueue => sent.map_err(|_| CdpError::Closed)?,
            }
        } else {
            enqueue.await.map_err(|_| CdpError::Closed)?;
        }

        Ok(EnqueuedCommand {
            response: EnqueuedResponse::Pending(response),
            _registration,
            response_timeout,
        })
    }

    fn prepare_command(
        &self,
        method: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> Result<PreparedCommand> {
        self.prepare_admitted_command(
            method,
            params,
            session_id,
            CommandAdmission {
                max_bytes: self.max_command_bytes,
                #[cfg(feature = "large-commands")]
                large: None,
            },
        )
    }

    fn prepare_admitted_command(
        &self,
        method: &str,
        params: Value,
        session_id: Option<&str>,
        admission: CommandAdmission,
    ) -> Result<PreparedCommand> {
        if self.is_closed() {
            return Err(CdpError::Closed);
        }
        let slot = acquire_pending_slot(
            Arc::clone(&self.active_pending),
            self.pending_command_capacity,
        )?;
        let resource = self
            .resources
            .as_ref()
            .map(|resources| {
                resources.ledger.reserve(
                    resources.profile.resource_scope(),
                    ResourceClass::WireBytes,
                    resources.profile.command_wire_bytes(),
                )
            })
            .transpose()?
            .map(|lease| {
                Arc::new(CommandResource {
                    _lease: lease,
                    #[cfg(feature = "large-commands")]
                    large: admission.large,
                })
            });
        let id = self
            .next_id
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |id| id.checked_add(1))
            .map_err(|_| CdpError::IdentifierExhausted)?;

        let mut msg = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });
        if let Some(sid) = session_id {
            msg["sessionId"] = Value::String(sid.to_string());
        }
        let raw = serialize_bounded(&msg, admission.max_bytes)?;
        let retained_session = match self
            .resources
            .as_ref()
            .and_then(|resources| resources.profile.pending_session_id_bytes)
        {
            Some(limit) => match session_id {
                Some("") => {
                    return Err(CdpError::InvalidConfiguration(
                        "pending session ID must be nonempty",
                    ));
                }
                Some(id) if id.len() > limit => {
                    return Err(CdpError::LimitExceeded {
                        resource: "pending session ID bytes",
                        actual: id.len(),
                        limit,
                    });
                }
                Some(id) => Some(Box::<str>::from(id)),
                None => None,
            },
            None => None,
        };
        trace!(
            id,
            method,
            session_id = session_id.unwrap_or(""),
            "prepared CDP command"
        );
        let (response_tx, response) = oneshot::channel();
        let replaced = self.pending.insert(
            id,
            PendingEntry {
                response: response_tx,
                _slot: slot,
                resource: resource.clone(),
                session_id: retained_session,
            },
        );
        debug_assert!(replaced.is_none(), "monotonic CDP command ID was reused");
        Ok(PreparedCommand {
            raw,
            response,
            _registration: PendingRegistration {
                id,
                pending: Arc::clone(&self.pending),
            },
            resource,
        })
    }

    /// Subscribe to all CDP events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<CdpEvent> {
        self.event_delivery.subscribe_legacy().unwrap_or_else(|| {
            let (event_tx, receiver) = broadcast::channel(1);
            drop(event_tx);
            receiver
        })
    }

    /// Subscribe through the production reclaim-after-all-read event path.
    ///
    /// This is opt-in on [`CdpConnectionProductionProfile`]. It is separate
    /// from the legacy broadcast API so existing v1 callers retain their
    /// current receiver type and replay-ring behavior.
    pub fn subscribe_reclaiming_events(
        &self,
    ) -> std::result::Result<ReclaimingEventReceiver<CdpEvent>, CdpEventSubscriptionError> {
        self.event_delivery.subscribe_reclaiming()
    }

    /// Creates the event cursor selected by this connection's production
    /// profile while presenting one receiver contract to v2 components.
    pub fn subscribe_composable_events(
        &self,
    ) -> std::result::Result<CdpEventReceiver, CdpEventSubscriptionError> {
        match self.subscribe_reclaiming_events() {
            Ok(receiver) => Ok(receiver.into()),
            Err(CdpEventSubscriptionError::ReclaimingFanoutNotConfigured) => {
                Ok(self.subscribe_events().into())
            }
            Err(error) => Err(error),
        }
    }

    pub fn reclaiming_event_metrics(&self) -> Option<ReclaimingFanoutMetrics> {
        self.event_delivery.reclaiming_metrics()
    }

    // Writer loop: drain channel -> websocket
    async fn writer_loop(
        mut ws_write: futures_util::stream::SplitSink<
            WebSocketStream<MaybeTlsStream<TcpStream>>,
            Message,
        >,
        mut write_rx: tokio::sync::mpsc::Receiver<OutboundMessage>,
    ) {
        while let Some(outbound) = write_rx.recv().await {
            #[cfg(feature = "large-commands")]
            if let Some(large) = outbound
                ._resource
                .as_ref()
                .and_then(|resource| resource.large.as_ref())
            {
                if let Err(error) =
                    large_commands::write_fragmented(&mut ws_write, outbound.message, large).await
                {
                    error!(%error, "fragmented CDP WebSocket write failed");
                    break;
                }
                continue;
            }
            if let Err(e) = ws_write.send(outbound.message).await {
                error!("WebSocket write error: {}", e);
                break;
            }
        }
        // Send close frame for clean shutdown
        let _ = ws_write.send(Message::Close(None)).await;
        debug!("CDP writer loop ended");
    }

    // Reader loop: websocket -> route to pending or the selected event fanout
    async fn reader_loop<ReadError: std::fmt::Display>(
        mut ws_read: impl futures_util::Stream<Item = std::result::Result<Message, ReadError>> + Unpin,
        pending: PendingMap,
        event_delivery: CdpEventDelivery,
        max_response_bytes: usize,
        max_event_bytes: usize,
        max_protocol_error_bytes: usize,
        resources: Option<ConnectionResourceBudget>,
    ) -> Option<Box<str>> {
        #[cfg(feature = "cdp-ws-close-reason")]
        let mut close_reason = None;
        while let Some(msg_result) = ws_read.next().await {
            let msg = match msg_result {
                Ok(m) => m,
                Err(e) => {
                    error!("WebSocket read error: {}", e);
                    break;
                }
            };

            let text = match msg {
                Message::Text(t) => t,
                Message::Close(frame) => {
                    #[cfg(feature = "cdp-ws-close-reason")]
                    if let Some(frame) = frame
                        && !frame.reason.is_empty()
                        && frame.reason.len() <= 123
                    {
                        close_reason = Some(frame.reason.to_string().into_boxed_str());
                    }
                    #[cfg(not(feature = "cdp-ws-close-reason"))]
                    let _ = frame;
                    debug!("WebSocket closed by remote");
                    break;
                }
                Message::Binary(_) => {
                    #[cfg(feature = "cdp-ws-invalid-frame")]
                    {
                        // CDP is text/JSON. Do not log or retain the binary
                        // payload: an invalid peer must settle pending calls
                        // promptly rather than waiting for their deadline.
                        warn!("unexpected binary CDP WebSocket frame");
                        break;
                    }
                    #[cfg(not(feature = "cdp-ws-invalid-frame"))]
                    continue;
                }
                _ => continue,
            };

            let value: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(e) => {
                    #[cfg(feature = "cdp-ws-invalid-frame")]
                    {
                        let _ = e;
                        // CDP frames must contain JSON. Keep the peer's text
                        // out of diagnostics and settle any pending command.
                        warn!("invalid JSON CDP WebSocket frame");
                        break;
                    }
                    #[cfg(not(feature = "cdp-ws-invalid-frame"))]
                    {
                        warn!("Failed to parse CDP message: {}", e);
                        continue;
                    }
                }
            };

            let Value::Object(mut object) = value else {
                warn!("ignored non-object CDP message");
                continue;
            };

            // Response to a command (has "id" field)
            if let Some(id) = object.get("id").and_then(Value::as_u64) {
                trace!(id, "received CDP response");
                if let Some((_, entry)) = pending.remove(&id) {
                    let result = if text.len() > max_response_bytes {
                        Err(CdpError::LimitExceeded {
                            resource: "response bytes",
                            actual: text.len(),
                            limit: max_response_bytes,
                        })
                    } else if let Some(err) = object.remove("error") {
                        let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                        let message = err
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("unknown error");
                        if message.len() > max_protocol_error_bytes {
                            Err(CdpError::LimitExceeded {
                                resource: "protocol error bytes",
                                actual: message.len(),
                                limit: max_protocol_error_bytes,
                            })
                        } else {
                            trace!(id, code, "CDP protocol command failed");
                            Err(CdpError::Protocol {
                                code,
                                message: compact_string(message),
                            })
                        }
                    } else {
                        Ok(object.remove("result").unwrap_or(Value::Null))
                    };
                    let _ = entry.response.send(RetainedCommandResult {
                        result,
                        _resource: entry.resource,
                    });
                } else {
                    warn!(id, "received CDP response for unknown/timed-out command");
                }
                continue;
            }

            // Event (has "method" field, no "id")
            if let Some(Value::String(method)) = object.remove("method") {
                if text.len() > max_event_bytes {
                    error!(
                        actual = text.len(),
                        limit = max_event_bytes,
                        "CDP event exceeded the configured byte limit; closing connection"
                    );
                    break;
                }
                if let Some(limit) = resources
                    .as_ref()
                    .and_then(|budget| budget.profile.pending_session_id_bytes)
                {
                    session_retirement::retire_from_event(
                        &pending,
                        &method,
                        object.get("params").unwrap_or(&Value::Null),
                        limit,
                    );
                }
                if let Some(policy) = resources
                    .as_ref()
                    .and_then(|budget| budget.profile.event_admission_policy())
                    && !policy.retains(
                        &method,
                        object.get("params").unwrap_or(&Value::Null),
                        object.get("sessionId").and_then(Value::as_str),
                    )
                {
                    // Reader wire storage was admitted already. Omitted
                    // payloads never acquire an event lease or an Arc/fanout
                    // slot; the following command response still gets read.
                    trace!(target: "pwright_cdp::event_admission", method, policy = policy.name(), "CDP event omitted by selected admission policy");
                    continue;
                }
                let session_id = match object.remove("sessionId") {
                    Some(Value::String(session_id)) => Some(compact_string(&session_id)),
                    _ => None,
                };
                let params = object.remove("params").unwrap_or(Value::Null);
                let event_resource = match &resources {
                    Some(resources) => match resources.ledger.reserve_many(
                        resources.profile.resource_scope(),
                        &[
                            ResourceRequest::new(ResourceClass::EventItem, 1),
                            ResourceRequest::new(
                                ResourceClass::EventBytes,
                                usize_to_u64(text.len()),
                            ),
                        ],
                    ) {
                        Ok(resource) => Some(resource),
                        Err(error) => {
                            tracing::error!(
                                %error,
                                "CDP event resource budget was exhausted; closing connection"
                            );
                            break;
                        }
                    },
                    None => None,
                };
                let event = match event_resource {
                    Some(resource) => CdpEvent::new_budgeted(
                        compact_string(&method),
                        params,
                        session_id,
                        resource,
                    ),
                    None => CdpEvent::new(compact_string(&method), params, session_id),
                };

                trace!(method = event.method, "CDP event received");
                lifecycle_trace::received(&event.method, &event.params);
                match event_delivery.send(event) {
                    Ok(0) => warn!(method, "CDP event dropped (no active subscribers)"),
                    Ok(_) => {}
                    Err(event) => {
                        error!(
                            method = event.method,
                            "CDP event delivery closed; closing connection"
                        );
                        break;
                    }
                }
            }
        }

        debug!("CDP reader loop ended");
        #[cfg(feature = "cdp-ws-close-reason")]
        let result = close_reason;
        #[cfg(not(feature = "cdp-ws-close-reason"))]
        let result = None;
        result
    }
}

fn fail_all_pending(pending: &PendingMap) {
    loop {
        let next_id = {
            let mut entries = pending.iter();
            entries.next().map(|entry| *entry.key())
        };
        let Some(id) = next_id else {
            break;
        };
        if let Some((_, entry)) = pending.remove(&id) {
            let _ = entry.response.send(RetainedCommandResult {
                result: Err(CdpError::Closed),
                _resource: entry.resource,
            });
        }
    }
}

fn cdp_websocket_config(max_websocket_bytes: usize) -> WebSocketConfig {
    WebSocketConfig::default()
        .max_message_size(Some(max_websocket_bytes))
        .max_frame_size(Some(max_websocket_bytes))
}

fn acquire_pending_slot(active: Arc<AtomicUsize>, limit: usize) -> Result<PendingCommandSlot> {
    active
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
            (current < limit).then_some(current + 1)
        })
        .map_err(|current| CdpError::LimitExceeded {
            resource: "pending commands",
            actual: current.saturating_add(1),
            limit,
        })?;
    Ok(PendingCommandSlot { active })
}

fn serialize_bounded(value: &Value, limit: usize) -> Result<String> {
    let mut writer = BoundedJsonWriter::new(limit);
    let result = serde_json::to_writer(&mut writer, value);
    if writer.exceeded {
        return Err(CdpError::LimitExceeded {
            resource: "command bytes",
            actual: limit.saturating_add(1),
            limit,
        });
    }
    result?;
    Ok(String::from_utf8(writer.bytes).expect("JSON serialization always emits UTF-8"))
}

struct BoundedJsonWriter {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl BoundedJsonWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(8 * 1024)),
            limit,
            exceeded: false,
        }
    }
}

impl Write for BoundedJsonWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(new_len) = self.bytes.len().checked_add(bytes.len()) else {
            self.exceeded = true;
            return Err(io::Error::other("CDP command byte limit exceeded"));
        };
        if new_len > self.limit {
            self.exceeded = true;
            return Err(io::Error::other("CDP command byte limit exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn compact_string(value: &str) -> String {
    let mut compact = String::with_capacity(value.len());
    compact.push_str(value);
    compact
}

#[cfg(test)]
mod tests {
    mod dom_description;
    mod event_admission;
    #[cfg(feature = "cdp-handshake-headers")]
    mod handshake_headers;
    #[cfg(feature = "large-commands")]
    mod large_commands;
    mod retained_result;
    mod session_retirement;
    mod settlement;
    mod ws_invalid_frame;
    use super::*;

    fn test_connection(config: ConnectionConfig) -> CdpConnection {
        let (write_tx, _write_rx) = tokio::sync::mpsc::channel(config.write_channel_capacity);
        let pending = Arc::new(DashMap::new());
        let event_delivery = CdpEventDelivery::legacy(config.event_channel_capacity);
        CdpConnection {
            write_tx,
            next_id: AtomicU64::new(1),
            pending,
            active_pending: Arc::new(AtomicUsize::new(0)),
            pending_command_capacity: config.pending_command_capacity,
            event_delivery,
            default_timeout: config.timeout,
            max_command_bytes: config.max_command_bytes,
            #[cfg(feature = "large-commands")]
            large_commands: None,
            resources: None,
            _reader_resource: None,
            _event_delivery_resource: None,
            #[cfg(feature = "cdp-ws-close-reason")]
            peer_close_reason: Arc::new(Mutex::new(None)),
            shutdown: tokio_util::sync::CancellationToken::new(),
            io_tasks: tokio_util::task::TaskTracker::new(),
        }
    }

    fn production_resources(profile: &CdpConnectionProductionProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let wire = requirements.class(ResourceClass::WireBytes);
        let event_items = requirements.class(ResourceClass::EventItem);
        let event_bytes = requirements.class(ResourceClass::EventBytes);
        let backend_state_bytes = requirements.class(ResourceClass::BackendStateBytes);
        let limits =
            pwright_core::ResourceLimits::deny_all(requirements.scopes, requirements.leases)
                .with_class(ResourceClass::WireBytes, wire.global, wire.per_scope)
                .with_class(
                    ResourceClass::EventItem,
                    event_items.global,
                    event_items.per_scope,
                )
                .with_class(
                    ResourceClass::EventBytes,
                    event_bytes.global,
                    event_bytes.per_scope,
                )
                .with_class(
                    ResourceClass::BackendStateBytes,
                    backend_state_bytes.global,
                    backend_state_bytes.per_scope,
                );
        ProductionResourceLedger::new(limits, requirements)
            .expect("exact production plan should fit")
    }

    fn test_production_connection(
        profile: &CdpConnectionProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> CdpConnection {
        test_production_connection_with_writer(profile, resources).0
    }

    fn test_production_connection_with_writer(
        profile: &CdpConnectionProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> (CdpConnection, tokio::sync::mpsc::Receiver<OutboundMessage>) {
        let config = profile.config().clone();
        let ledger = resources
            .claim(profile.requirements())
            .expect("profile should be declared");
        let reader_resource = ledger
            .reserve(
                profile.resource_scope(),
                ResourceClass::WireBytes,
                profile.reader_wire_bytes(),
            )
            .expect("reader peak should fit");
        let event_delivery_resource = if profile.event_delivery_resource_bytes() == 0 {
            None
        } else {
            Some(
                ledger
                    .reserve(
                        profile.resource_scope(),
                        ResourceClass::BackendStateBytes,
                        profile.event_delivery_resource_bytes(),
                    )
                    .expect("event delivery bookkeeping should fit"),
            )
        };
        let (write_tx, write_rx) = tokio::sync::mpsc::channel(config.write_channel_capacity);
        let pending = Arc::new(DashMap::new());
        let event_delivery = match profile.reclaiming_event_subscribers() {
            Some(subscribers) => {
                CdpEventDelivery::reclaiming(config.event_channel_capacity, subscribers)
            }
            None => CdpEventDelivery::legacy(config.event_channel_capacity),
        };
        (
            CdpConnection {
                write_tx,
                next_id: AtomicU64::new(1),
                pending,
                active_pending: Arc::new(AtomicUsize::new(0)),
                pending_command_capacity: config.pending_command_capacity,
                event_delivery,
                default_timeout: config.timeout,
                max_command_bytes: config.max_command_bytes,
                #[cfg(feature = "large-commands")]
                large_commands: profile.large_commands.map(|large| {
                    crate::connection::large_commands::Pool::new(large, &ledger).unwrap()
                }),
                resources: Some(ConnectionResourceBudget {
                    ledger,
                    profile: profile.clone(),
                }),
                _reader_resource: Some(reader_resource),
                _event_delivery_resource: event_delivery_resource,
                #[cfg(feature = "cdp-ws-close-reason")]
                peer_close_reason: Arc::new(Mutex::new(None)),
                shutdown: tokio_util::sync::CancellationToken::new(),
                io_tasks: tokio_util::task::TaskTracker::new(),
            },
            write_rx,
        )
    }

    #[test]
    fn test_compound_error_display() {
        let err = CdpError::Compound {
            source: Box::new(CdpError::Timeout),
            system: Box::new(CdpError::Closed),
        };
        let msg = format!("{err}");
        assert!(msg.contains("Timeout"), "missing source: {msg}");
        assert!(msg.contains("Connection closed"), "missing system: {msg}");
    }

    #[test]
    fn test_compound_error_pattern_match() {
        let err = CdpError::Compound {
            source: Box::new(CdpError::Timeout),
            system: Box::new(CdpError::Closed),
        };
        match err {
            CdpError::Compound { source, system } => {
                assert!(matches!(*source, CdpError::Timeout));
                assert!(matches!(*system, CdpError::Closed));
            }
            other => panic!("expected Compound, got: {other:?}"),
        }
    }

    #[test]
    fn closed_future_is_owned_and_sticky_after_native_transport_termination() {
        // Arrange
        let connection = test_connection(ConnectionConfig::default());
        let mut closed = Box::pin(connection.closed());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        let before = closed.as_mut().poll(&mut context);

        // Act
        connection.shutdown.cancel();
        let after = closed.as_mut().poll(&mut context);
        let mut later = Box::pin(connection.closed());
        let repeated = later.as_mut().poll(&mut context);

        // Assert
        assert!(before.is_pending());
        assert!(after.is_ready());
        assert!(repeated.is_ready());
        assert!(connection.is_closed());
    }

    #[test]
    fn closed_future_does_not_keep_the_connection_alive() {
        // Arrange
        let connection = test_connection(ConnectionConfig::default());
        let mut closed = Box::pin(connection.closed());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(closed.as_mut().poll(&mut context).is_pending());

        // Act
        drop(connection);
        let after_drop = closed.as_mut().poll(&mut context);

        // Assert
        assert!(after_drop.is_ready());
    }

    #[test]
    fn test_connection_config_default() {
        let config = ConnectionConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.write_channel_capacity, 256);
        assert_eq!(config.event_channel_capacity, 1024);
        assert_eq!(config.pending_command_capacity, 256);
        assert_eq!(config.max_command_bytes, MAX_CDP_WEBSOCKET_BYTES);
        assert_eq!(config.max_response_bytes, MAX_CDP_WEBSOCKET_BYTES);
        assert_eq!(config.max_event_bytes, MAX_CDP_WEBSOCKET_BYTES);
        assert_eq!(config.max_websocket_bytes, MAX_CDP_WEBSOCKET_BYTES);
        assert_eq!(config.max_protocol_error_bytes, 64 * 1024);
    }

    #[test]
    fn zero_command_timeout_disables_the_response_deadline() {
        // Arrange
        let unlimited = Duration::ZERO;
        let bounded = Duration::from_secs(30);

        // Act
        let unlimited_response = command_response_timeout(unlimited);
        let bounded_response = command_response_timeout(bounded);

        // Assert
        assert_eq!(unlimited_response, None);
        assert_eq!(bounded_response, Some(bounded));
    }

    #[test]
    fn io_finished_waits_for_task_cleanup_without_retaining_connection() {
        // Arrange
        let connection = test_connection(ConnectionConfig::default());
        let task = connection.io_tasks.token();
        connection.io_tasks.close();
        let mut finished = Box::pin(connection.io_finished());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        drop(connection);
        assert!(finished.as_mut().poll(&mut context).is_pending());

        // Act
        drop(task);

        // Assert
        assert!(finished.as_mut().poll(&mut context).is_ready());
    }

    #[tokio::test]
    async fn explicit_disconnect_rejects_unpolled_and_new_commands_without_browser_close() {
        // Arrange
        let profile = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig::default(),
        );
        let resources = production_resources(&profile);
        let (connection, mut wire) = test_production_connection_with_writer(&profile, &resources);
        let prepared = connection.send_command("Browser.getVersion", serde_json::json!({}));
        let closed = connection.closed();
        // Act
        connection.request_disconnect();
        // Assert
        closed.await;
        assert!(matches!(prepared.await, Err(CdpError::Closed)));
        let new_call = connection
            .send_command("Browser.getVersion", serde_json::json!({}))
            .await;
        assert!(matches!(new_call, Err(CdpError::Closed)));
        assert!(wire.try_recv().is_err());
        drop(connection);
        assert_eq!(resources.metrics().unwrap().active_leases, 0);
    }

    #[test]
    fn websocket_config_accepts_a_base64_encoded_fifty_megabyte_capture() {
        // Arrange
        let raw_capture_bytes = 50usize << 20;
        let encoded_capture_bytes = raw_capture_bytes.div_ceil(3) * 4;

        // Act
        let config = cdp_websocket_config(MAX_CDP_WEBSOCKET_BYTES);

        // Assert
        assert!(
            config
                .max_frame_size
                .is_some_and(|limit| limit > encoded_capture_bytes)
        );
        assert!(
            config
                .max_message_size
                .is_some_and(|limit| limit > encoded_capture_bytes)
        );
        assert_eq!(config.max_frame_size, Some(MAX_CDP_WEBSOCKET_BYTES));
        assert_eq!(config.max_message_size, Some(MAX_CDP_WEBSOCKET_BYTES));
    }

    #[test]
    fn zero_capacity_configuration_is_rejected_without_panicking() {
        // Arrange
        let config = ConnectionConfig {
            pending_command_capacity: 0,
            ..ConnectionConfig::default()
        };

        // Act
        let error = config.validate().expect_err("zero capacity must fail");

        // Assert
        assert!(matches!(error, CdpError::InvalidConfiguration(_)));
    }

    #[test]
    fn bounded_serializer_stops_before_retaining_an_oversized_command() {
        // Arrange
        let value = serde_json::json!({"payload": "0123456789"});

        // Act
        let error = serialize_bounded(&value, 8).expect_err("payload must exceed limit");

        // Assert
        assert!(matches!(
            error,
            CdpError::LimitExceeded {
                resource: "command bytes",
                limit: 8,
                ..
            }
        ));
    }

    #[test]
    fn dropping_an_unpolled_command_removes_its_pending_entry() {
        // Arrange
        let connection = test_connection(ConnectionConfig::default());

        // Act
        let command = connection.send_command("Runtime.enable", serde_json::json!({}));
        assert_eq!(connection.pending.len(), 1);
        assert_eq!(connection.active_pending.load(Ordering::Acquire), 1);
        drop(command);

        // Assert
        assert!(connection.pending.is_empty());
        assert_eq!(connection.active_pending.load(Ordering::Acquire), 0);
    }

    #[tokio::test]
    async fn pending_capacity_rejects_before_a_second_map_insertion() {
        // Arrange
        let connection = test_connection(ConnectionConfig {
            pending_command_capacity: 1,
            ..ConnectionConfig::default()
        });
        let first = connection.send_command("Runtime.enable", serde_json::json!({}));

        // Act
        let error = connection
            .send_command("Page.enable", serde_json::json!({}))
            .await
            .expect_err("second command must be rejected");

        // Assert
        assert!(matches!(
            error,
            CdpError::LimitExceeded {
                resource: "pending commands",
                actual: 2,
                limit: 1,
            }
        ));
        assert_eq!(connection.pending.len(), 1);
        drop(first);
        assert!(connection.pending.is_empty());
    }

    #[tokio::test]
    async fn shutdown_cleanup_fails_and_releases_every_pending_command() {
        // Arrange
        let active = Arc::new(AtomicUsize::new(0));
        let pending = Arc::new(DashMap::new());
        let mut receivers = Vec::new();
        for id in 1..=2 {
            let slot =
                acquire_pending_slot(Arc::clone(&active), 2).expect("two pending slots should fit");
            let (response, receiver) = oneshot::channel();
            pending.insert(
                id,
                PendingEntry {
                    response,
                    _slot: slot,
                    resource: None,
                    session_id: None,
                },
            );
            receivers.push(receiver);
        }

        // Act
        fail_all_pending(&pending);

        // Assert
        assert!(pending.is_empty());
        assert_eq!(active.load(Ordering::Acquire), 0);
        for receiver in receivers {
            let response = receiver.await.expect("shutdown result should be delivered");
            assert!(matches!(response.result, Err(CdpError::Closed)));
        }
    }

    #[tokio::test]
    async fn oversized_command_releases_its_local_pending_slot() {
        // Arrange
        let connection = test_connection(ConnectionConfig {
            max_command_bytes: 16,
            ..ConnectionConfig::default()
        });

        // Act
        let error = connection
            .send_command(
                "Runtime.evaluate",
                serde_json::json!({"expression": "large"}),
            )
            .await
            .expect_err("serialized command must exceed limit");

        // Assert
        assert!(matches!(error, CdpError::LimitExceeded { .. }));
        assert!(connection.pending.is_empty());
        assert_eq!(connection.active_pending.load(Ordering::Acquire), 0);
    }

    #[tokio::test]
    async fn child_detach_resumes_child_then_addresses_the_announcing_parent() {
        // Arrange: replace the wire with an in-memory transport, keeping the
        // actual CdpSession command serialization and pending-call machinery.
        let mut connection = test_connection(ConnectionConfig::default());
        let (sender, mut wire) = tokio::sync::mpsc::channel(2);
        connection.write_tx = sender;
        let connection = Arc::new(connection);
        let client = crate::CdpSession::browser(connection.clone());
        // Act
        let operation = tokio::spawn(async move {
            client
                .target_detach_in_parent(Some("parent"), "child")
                .await
        });
        // Assert
        for (method, session, params) in [
            (
                "Runtime.runIfWaitingForDebugger",
                "child",
                serde_json::json!({}),
            ),
            (
                "Target.detachFromTarget",
                "parent",
                serde_json::json!({"sessionId":"child"}),
            ),
        ] {
            let outbound = wire.recv().await.unwrap();
            let message: Value = serde_json::from_str(outbound.message.to_text().unwrap()).unwrap();
            assert_eq!(message["method"], method);
            assert_eq!(message["sessionId"], session);
            assert_eq!(message["params"], params);
            let (_, pending) = connection
                .pending
                .remove(&message["id"].as_u64().unwrap())
                .unwrap();
            assert!(
                pending
                    .response
                    .send(RetainedCommandResult {
                        result: Ok(serde_json::json!({})),
                        _resource: pending.resource
                    })
                    .is_ok()
            );
        }
        operation.await.unwrap().unwrap();
        assert!(wire.try_recv().is_err());
        assert!(connection.pending.is_empty());
        assert_eq!(connection.active_pending.load(Ordering::Acquire), 0);
    }

    #[tokio::test]
    async fn child_detach_accepts_only_exact_missing_child_receipt_before_detaching() {
        // Arrange
        let cases = [
            (CdpError::SessionDetached, true),
            (
                CdpError::Protocol {
                    code: -32001,
                    message: "Session with given id not found.".into(),
                },
                true,
            ),
            (
                CdpError::Protocol {
                    code: -32000,
                    message: "Session with given id not found.".into(),
                },
                false,
            ),
            (
                CdpError::Protocol {
                    code: -32001,
                    message: "other failure".into(),
                },
                false,
            ),
            (CdpError::Closed, false),
            (CdpError::Timeout, false),
        ];
        for (error, retired) in cases {
            let expected = error.to_string();
            let mut connection = test_connection(ConnectionConfig::default());
            let (sender, mut wire) = tokio::sync::mpsc::channel(2);
            connection.write_tx = sender;
            let connection = Arc::new(connection);
            let client = crate::CdpSession::browser(connection.clone());
            // Act
            let operation = tokio::spawn(async move {
                client
                    .target_detach_in_parent(Some("parent"), "child")
                    .await
            });
            let outbound = wire.recv().await.unwrap();
            let message: Value = serde_json::from_str(outbound.message.to_text().unwrap()).unwrap();
            assert_eq!(message["method"], "Runtime.runIfWaitingForDebugger");
            assert_eq!(message["sessionId"], "child");
            let (
                _,
                PendingEntry {
                    response,
                    resource,
                    _slot: slot,
                    session_id: _,
                },
            ) = connection
                .pending
                .remove(&message["id"].as_u64().unwrap())
                .unwrap();
            drop(slot);
            assert!(
                response
                    .send(RetainedCommandResult {
                        result: Err(error),
                        _resource: resource,
                    })
                    .is_ok()
            );
            let result = operation.await.unwrap();
            // Assert
            if retired {
                assert!(result.is_ok());
            } else {
                assert_eq!(result.unwrap_err().to_string(), expected);
            }
            assert!(
                wire.try_recv().is_err(),
                "no detach or retry after a failed resume"
            );
            assert!(connection.pending.is_empty());
            assert_eq!(connection.active_pending.load(Ordering::Acquire), 0);
        }
    }

    #[tokio::test]
    async fn child_detach_accepts_only_exact_second_stage_already_detached_receipt() {
        // Arrange: exercise both serialized native commands and real pending
        // registrations; only the injected transport response differs per case.
        let cases = [
            (
                CdpError::Protocol {
                    code: -32602,
                    message: "No session with given id".into(),
                },
                true,
            ),
            (
                CdpError::Protocol {
                    code: -32000,
                    message: "No session with given id".into(),
                },
                false,
            ),
            (
                CdpError::Protocol {
                    code: -32602,
                    message: "No session with given id.".into(),
                },
                false,
            ),
            (
                CdpError::Protocol {
                    code: -32602,
                    message: "other invalid parameters".into(),
                },
                false,
            ),
            (
                CdpError::Protocol {
                    code: -32001,
                    message: "Session with given id not found.".into(),
                },
                false,
            ),
            (CdpError::Closed, false),
            (CdpError::Timeout, false),
            (CdpError::ChannelDropped, false),
        ];
        for (error, retired) in cases {
            let expected = error.to_string();
            let mut connection = test_connection(ConnectionConfig::default());
            let (sender, mut wire) = tokio::sync::mpsc::channel(2);
            connection.write_tx = sender;
            let connection = Arc::new(connection);
            let client = crate::CdpSession::browser(connection.clone());
            // Act
            let operation = tokio::spawn(async move {
                client
                    .target_detach_in_parent(Some("parent"), "child")
                    .await
            });
            for (method, session, result) in [
                (
                    "Runtime.runIfWaitingForDebugger",
                    "child",
                    Ok(serde_json::json!({})),
                ),
                ("Target.detachFromTarget", "parent", Err(error)),
            ] {
                let outbound = wire.recv().await.unwrap();
                let message: Value =
                    serde_json::from_str(outbound.message.to_text().unwrap()).unwrap();
                assert_eq!(message["method"], method);
                assert_eq!(message["sessionId"], session);
                if method == "Target.detachFromTarget" {
                    assert_eq!(
                        message["params"],
                        serde_json::json!({ "sessionId": "child" })
                    );
                }
                let (
                    _,
                    PendingEntry {
                        response,
                        resource,
                        _slot: slot,
                        session_id: _,
                    },
                ) = connection
                    .pending
                    .remove(&message["id"].as_u64().unwrap())
                    .unwrap();
                drop(slot);
                assert!(
                    response
                        .send(RetainedCommandResult {
                            result,
                            _resource: resource
                        })
                        .is_ok()
                );
            }
            let result = operation.await.unwrap();
            // Assert
            if retired {
                assert!(result.is_ok());
            } else {
                assert_eq!(result.unwrap_err().to_string(), expected);
            }
            assert!(
                wire.try_recv().is_err(),
                "no retries or extra cleanup commands"
            );
            assert!(connection.pending.is_empty());
            assert_eq!(connection.active_pending.load(Ordering::Acquire), 0);
        }
    }

    #[test]
    fn production_profile_scales_command_and_event_peaks() {
        // Arrange
        let config = ConnectionConfig {
            pending_command_capacity: 2,
            event_channel_capacity: 3,
            max_command_bytes: 10,
            max_response_bytes: 30,
            max_event_bytes: 7,
            max_websocket_bytes: 50,
            ..ConnectionConfig::default()
        };

        // Act
        let profile = CdpConnectionProductionProfile::new(ResourceScope::Connection, config);

        // Assert
        assert_eq!(profile.reader_wire_bytes(), 100);
        assert_eq!(profile.command_wire_bytes(), 30);
        assert_eq!(
            profile
                .requirements()
                .class(ResourceClass::WireBytes)
                .global,
            160
        );
        assert_eq!(
            profile
                .requirements()
                .class(ResourceClass::EventItem)
                .global,
            4
        );
        assert_eq!(
            profile
                .requirements()
                .class(ResourceClass::EventBytes)
                .global,
            28
        );
        assert_eq!(profile.requirements().leases, 7);
        assert_eq!(profile.retained_event_bytes(), 21);
    }

    #[test]
    fn production_profile_bounds_many_small_events_without_multiplying_maximum_size() {
        // Arrange
        let profile = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig {
                event_channel_capacity: 8,
                max_event_bytes: 7,
                ..ConnectionConfig::default()
            },
        );

        // Act
        let bounded = profile
            .with_retained_event_byte_capacity(21)
            .expect("three maximum-size retained events should validate");

        // Assert
        assert_eq!(bounded.retained_event_bytes(), 21);
        assert_eq!(
            bounded
                .requirements()
                .class(ResourceClass::EventItem)
                .global,
            9
        );
        assert_eq!(
            bounded
                .requirements()
                .class(ResourceClass::EventBytes)
                .global,
            28
        );
    }

    #[test]
    fn production_profile_rejects_event_byte_capacity_smaller_than_one_event() {
        // Arrange
        let profile = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig {
                max_event_bytes: 7,
                ..ConnectionConfig::default()
            },
        );

        // Act
        let error = profile
            .with_retained_event_byte_capacity(6)
            .expect_err("one maximum-size event must fit in retained storage");

        // Assert
        assert_eq!(
            error.to_string(),
            "invalid CDP connection configuration: retained event byte capacity is smaller than one maximum-size event"
        );
    }

    #[test]
    fn reclaiming_fanout_profile_declares_fixed_bookkeeping_and_one_lease() {
        // Arrange
        let legacy = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig {
                event_channel_capacity: 3,
                ..ConnectionConfig::default()
            },
        );

        // Act
        let reclaiming = legacy
            .clone()
            .with_reclaiming_event_fanout(2)
            .expect("reclaiming fanout profile should validate");

        // Assert
        assert_eq!(legacy.reclaiming_event_subscribers(), None);
        assert_eq!(reclaiming.reclaiming_event_subscribers(), Some(2));
        assert!(reclaiming.event_delivery_resource_bytes() > 0);
        assert_eq!(
            reclaiming
                .requirements()
                .class(ResourceClass::BackendStateBytes)
                .global,
            reclaiming.event_delivery_resource_bytes()
        );
        assert_eq!(
            reclaiming.requirements().leases,
            legacy.requirements().leases + 1
        );
    }

    #[test]
    fn reclaiming_connection_releases_budgeted_event_after_all_readers_advance() {
        // Arrange
        let profile = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig {
                event_channel_capacity: 2,
                max_event_bytes: 64,
                ..ConnectionConfig::default()
            },
        )
        .with_reclaiming_event_fanout(2)
        .expect("reclaiming fanout profile should validate");
        let resources = production_resources(&profile);
        let connection = test_production_connection(&profile, &resources);
        let mut first = connection
            .subscribe_reclaiming_events()
            .expect("first subscriber should fit");
        let mut second = connection
            .subscribe_reclaiming_events()
            .expect("second subscriber should fit");
        let ledger = resources
            .claim(profile.requirements())
            .expect("profile should be declared");
        let event_resource = ledger
            .reserve_many(
                ResourceScope::Connection,
                &[
                    ResourceRequest::new(ResourceClass::EventItem, 1),
                    ResourceRequest::new(ResourceClass::EventBytes, 32),
                ],
            )
            .expect("one event should fit");
        let event = CdpEvent::new_budgeted(
            "Network.loadingFinished".to_owned(),
            serde_json::json!({"requestId": "1"}),
            Some("session-1".to_owned()),
            event_resource,
        );
        assert_eq!(
            connection
                .event_delivery
                .send(event)
                .expect("event fanout should remain open"),
            2
        );

        // Act
        let first_event = first.try_recv().expect("first should receive");
        drop(first_event);
        let after_first = resources
            .metrics()
            .expect("resource metrics should be readable");
        let second_event = second.try_recv().expect("second should receive");
        let after_second_read = resources
            .metrics()
            .expect("resource metrics should be readable");
        drop(second_event);
        let after_second_drop = resources
            .metrics()
            .expect("resource metrics should be readable");

        // Assert
        assert_eq!(after_first.class(ResourceClass::EventItem).current, 1);
        assert_eq!(
            after_second_read.class(ResourceClass::EventItem).current,
            1,
            "the delivered value still owns the event"
        );
        assert_eq!(after_second_drop.class(ResourceClass::EventItem).current, 0);
        assert_eq!(
            connection
                .reclaiming_event_metrics()
                .expect("reclaiming metrics should exist")
                .queued_events,
            0
        );
    }

    #[test]
    fn legacy_and_reclaiming_subscription_modes_remain_explicitly_separate() {
        // Arrange
        let legacy = test_connection(ConnectionConfig::default());
        let profile = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig::default(),
        )
        .with_reclaiming_event_fanout(1)
        .expect("reclaiming profile should validate");
        let resources = production_resources(&profile);
        let reclaiming = test_production_connection(&profile, &resources);

        // Act
        let legacy_reclaiming = legacy.subscribe_reclaiming_events();
        let mut reclaiming_legacy = reclaiming.subscribe_events();

        // Assert
        assert_eq!(
            legacy_reclaiming.expect_err("legacy mode should reject reclaiming cursors"),
            CdpEventSubscriptionError::ReclaimingFanoutNotConfigured
        );
        assert!(matches!(
            reclaiming_legacy.try_recv(),
            Err(broadcast::error::TryRecvError::Closed)
        ));
    }

    #[test]
    fn event_profile_covers_allocation_before_broadcast_ring_overwrite() {
        // Arrange
        let profile = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig {
                pending_command_capacity: 1,
                event_channel_capacity: 1,
                max_command_bytes: 64,
                max_response_bytes: 64,
                max_event_bytes: 32,
                max_websocket_bytes: 64,
                ..ConnectionConfig::default()
            },
        );
        let resources = production_resources(&profile);
        let ledger = resources
            .claim(profile.requirements())
            .expect("event requirement should be declared");
        let (sender, mut receiver) = broadcast::channel(1);
        let first = ledger
            .reserve_many(
                ResourceScope::Connection,
                &[
                    ResourceRequest::new(ResourceClass::EventItem, 1),
                    ResourceRequest::new(ResourceClass::EventBytes, 32),
                ],
            )
            .expect("first queued event should fit");
        sender
            .send(CdpEvent::new_budgeted(
                "Page.first".to_owned(),
                Value::Null,
                None,
                first,
            ))
            .expect("first event should enqueue");

        // Act
        let second = ledger.reserve_many(
            ResourceScope::Connection,
            &[
                ResourceRequest::new(ResourceClass::EventItem, 1),
                ResourceRequest::new(ResourceClass::EventBytes, 32),
            ],
        );
        sender
            .send(CdpEvent::new_budgeted(
                "Page.second".to_owned(),
                Value::Null,
                None,
                second.expect("incoming event peak must fit before ring eviction"),
            ))
            .expect("second event should overwrite the full ring");

        // Assert
        assert!(matches!(
            receiver.try_recv(),
            Err(broadcast::error::TryRecvError::Lagged(1))
        ));
        assert_eq!(
            receiver
                .try_recv()
                .expect("newest event should remain in the ring")
                .method,
            "Page.second"
        );
    }

    #[test]
    fn unpolled_production_command_owns_and_releases_its_wire_peak() {
        // Arrange
        let profile = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig {
                pending_command_capacity: 1,
                event_channel_capacity: 1,
                max_command_bytes: 256,
                max_response_bytes: 512,
                max_event_bytes: 64,
                max_websocket_bytes: 1024,
                ..ConnectionConfig::default()
            },
        );
        let resources = production_resources(&profile);
        let connection = test_production_connection(&profile, &resources);
        let reader_bytes = profile.reader_wire_bytes();

        // Act
        let command = connection.send_command("Runtime.enable", serde_json::json!({}));

        // Assert
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should be readable")
                .class(ResourceClass::WireBytes)
                .current,
            reader_bytes + profile.command_wire_bytes()
        );
        drop(command);
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should be readable")
                .class(ResourceClass::WireBytes)
                .current,
            reader_bytes
        );
        drop(connection);
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should be readable")
                .class(ResourceClass::WireBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn cancelled_command_keeps_queued_wire_bytes_charged_until_writer_drop() {
        // Arrange
        let profile = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig {
                timeout: Duration::from_secs(60),
                pending_command_capacity: 1,
                event_channel_capacity: 1,
                max_command_bytes: 256,
                max_response_bytes: 512,
                max_event_bytes: 64,
                max_websocket_bytes: 1024,
                ..ConnectionConfig::default()
            },
        );
        let resources = production_resources(&profile);
        let (connection, mut writer) = test_production_connection_with_writer(&profile, &resources);
        let connection = Arc::new(connection);
        let task_connection = Arc::clone(&connection);
        let task = tokio::spawn(async move {
            task_connection
                .send_command("Runtime.enable", serde_json::json!({}))
                .await
        });
        let queued = writer
            .recv()
            .await
            .expect("command should reach writer queue");

        // Act
        task.abort();
        let _ = task.await;

        // Assert
        assert!(connection.pending.is_empty());
        assert_eq!(connection.active_pending.load(Ordering::Acquire), 0);
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should be readable")
                .class(ResourceClass::WireBytes)
                .current,
            profile.reader_wire_bytes() + profile.command_wire_bytes()
        );
        drop(queued);
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should be readable")
                .class(ResourceClass::WireBytes)
                .current,
            profile.reader_wire_bytes()
        );
    }

    #[test]
    fn event_clones_share_one_resource_lease_until_the_last_drop() {
        // Arrange
        let profile = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig {
                pending_command_capacity: 1,
                event_channel_capacity: 1,
                max_command_bytes: 64,
                max_response_bytes: 64,
                max_event_bytes: 32,
                max_websocket_bytes: 64,
                ..ConnectionConfig::default()
            },
        );
        let resources = production_resources(&profile);
        let ledger = resources
            .claim(profile.requirements())
            .expect("event requirement should be declared");
        let lease = ledger
            .reserve_many(
                ResourceScope::Connection,
                &[
                    ResourceRequest::new(ResourceClass::EventItem, 1),
                    ResourceRequest::new(ResourceClass::EventBytes, 12),
                ],
            )
            .expect("one event should fit");
        let event =
            CdpEvent::new_budgeted("Page.loadEventFired".to_string(), Value::Null, None, lease);

        // Act
        let clone = event.clone();
        drop(event);

        // Assert
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should be readable")
                .class(ResourceClass::EventBytes)
                .current,
            12
        );
        drop(clone);
        let metrics = resources.metrics().expect("metrics should be readable");
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 0);
        assert_eq!(metrics.class(ResourceClass::EventBytes).current, 0);
    }

    #[tokio::test]
    async fn omitted_connection_profile_fails_before_network_access() {
        // Arrange
        let profile = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig {
                pending_command_capacity: 1,
                event_channel_capacity: 1,
                max_command_bytes: 64,
                max_response_bytes: 64,
                max_event_bytes: 64,
                max_websocket_bytes: 64,
                ..ConnectionConfig::default()
            },
        );
        let requirements = profile.requirements();
        let wire = requirements.class(ResourceClass::WireBytes);
        let event_items = requirements.class(ResourceClass::EventItem);
        let event_bytes = requirements.class(ResourceClass::EventBytes);
        let limits = pwright_core::ResourceLimits::deny_all(1, requirements.leases)
            .with_class(ResourceClass::WireBytes, wire.global, wire.per_scope)
            .with_class(
                ResourceClass::EventItem,
                event_items.global,
                event_items.per_scope,
            )
            .with_class(
                ResourceClass::EventBytes,
                event_bytes.global,
                event_bytes.per_scope,
            );
        let resources = ProductionResourceLedger::new(limits, ResourceRequirements::none())
            .expect("empty plan should be valid");

        // Act
        let error = match CdpConnection::connect_with_production_resources(
            "ws://127.0.0.1:1/should-not-connect",
            profile,
            &resources,
        )
        .await
        {
            Ok(_) => panic!("omitted profile must fail"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(error, CdpProductionConnectError::Production(_)));
    }

    #[test]
    fn test_cdp_error_display_websocket() {
        let err = CdpError::WebSocket("connection refused".to_string());
        assert_eq!(format!("{err}"), "WebSocket error: connection refused");
    }

    #[test]
    fn test_cdp_error_display_protocol() {
        let err = CdpError::Protocol {
            code: -32600,
            message: "Invalid Request".to_string(),
        };
        assert_eq!(format!("{err}"), "CDP error -32600: Invalid Request");
    }

    #[test]
    fn test_cdp_error_display_closed() {
        let err = CdpError::Closed;
        assert_eq!(format!("{err}"), "Connection closed");
    }

    #[test]
    fn test_cdp_error_display_channel_dropped() {
        let err = CdpError::ChannelDropped;
        assert_eq!(format!("{err}"), "Response channel dropped");
    }

    #[test]
    fn test_cdp_error_display_timeout() {
        let err = CdpError::Timeout;
        assert_eq!(format!("{err}"), "Timeout waiting for response");
    }

    #[test]
    fn test_cdp_error_display_element_not_found() {
        let err = CdpError::ElementNotFound {
            selector: "div.missing".to_string(),
        };
        assert_eq!(
            format!("{err}"),
            "Element not found for selector: div.missing"
        );
    }

    #[test]
    fn test_cdp_error_display_navigation_failed() {
        let err = CdpError::NavigationFailed {
            url: "https://example.com".to_string(),
            reason: "net::ERR_CONNECTION_REFUSED".to_string(),
        };
        assert_eq!(
            format!("{err}"),
            "Navigation failed for https://example.com: net::ERR_CONNECTION_REFUSED"
        );
    }

    #[test]
    fn test_cdp_error_display_page_closed() {
        let err = CdpError::PageClosed;
        assert_eq!(format!("{err}"), "Page is closed");
    }

    #[test]
    fn test_cdp_error_display_tab_not_found() {
        let err = CdpError::TabNotFound("ABC123".to_string());
        assert_eq!(format!("{err}"), "Tab not found: ABC123");
    }

    #[test]
    fn test_cdp_error_display_http_failed() {
        let err = CdpError::HttpFailed("404 Not Found".to_string());
        assert_eq!(format!("{err}"), "HTTP request failed: 404 Not Found");
    }

    #[test]
    fn test_cdp_error_display_js_exception() {
        let err = CdpError::JsException("TypeError: undefined is not a function".to_string());
        assert_eq!(
            format!("{err}"),
            "JavaScript exception: TypeError: undefined is not a function"
        );
    }

    #[test]
    fn test_cdp_error_display_other() {
        let err = CdpError::Other("something unexpected".to_string());
        assert_eq!(format!("{err}"), "something unexpected");
    }

    #[test]
    fn test_cdp_error_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("not valid json").unwrap_err();
        let expected_msg = format!("{json_err}");
        let cdp_err: CdpError = json_err.into();
        match &cdp_err {
            CdpError::Json(inner) => {
                assert_eq!(format!("{inner}"), expected_msg);
            }
            other => panic!("expected Json variant, got: {other:?}"),
        }
        let display = format!("{cdp_err}");
        assert!(
            display.starts_with("JSON error:"),
            "unexpected display: {display}"
        );
    }

    #[test]
    fn test_cdp_error_from_tungstenite_error() {
        let ws_err = tokio_tungstenite::tungstenite::Error::ConnectionClosed;
        let expected_msg = format!("{ws_err}");
        let cdp_err: CdpError = ws_err.into();
        match &cdp_err {
            CdpError::WebSocket(msg) => {
                assert_eq!(msg, &expected_msg);
            }
            other => panic!("expected WebSocket variant, got: {other:?}"),
        }
    }
}
