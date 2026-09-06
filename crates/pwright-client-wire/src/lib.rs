//! Executor-neutral encoded command channel for Pwright clients.
//!
//! Construction yields a cloneable command channel and one caller-driven
//! receive driver. This crate starts no task, thread, or async runtime.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use pwright_connection_control::{ClientCloseAction, ClientCloseError, ClientCloseMachine};
use pwright_core::{
    CallId, CancellationId, CancellationReason, Command, CommandResult, CommandSpec,
    ControlMessage, FailureKind, OperationError, ProductionResourceError, ProductionResourceLedger,
    ResourceClass, ResourceError, ResourceLease, ResourceLedger, ResourceRequirements,
    ResourceScope, Service,
};
use pwright_transport::{
    CallCorrelator, ClientHello, ClientHelloValidationError, CorrelatedCall, CorrelationError,
    FrameError, FrameLimits, FrameRoute, FrameTransport, ProductionFrameTransport, ServerHello,
    ShutdownFrameTransport, WireCodec, WireFrame, WireProtocol, WireProtocolError,
    validate_server_hello,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientHandshakeConfigurationError {
    VersionPolicyMismatch,
    ReceiveLimitsMismatch {
        protocol: FrameLimits,
        advertised: FrameLimits,
    },
}

impl Display for ClientHandshakeConfigurationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::VersionPolicyMismatch => formatter.write_str(
                "handshake protocol version policy must match the ClientHello version range",
            ),
            Self::ReceiveLimitsMismatch {
                protocol,
                advertised,
            } => write!(
                formatter,
                "handshake protocol receive limits {protocol:?} do not match ClientHello limits {advertised:?}"
            ),
        }
    }
}

impl Error for ClientHandshakeConfigurationError {}

#[derive(Debug)]
pub enum ClientHandshakeError<Encode, Decode, Transport> {
    Configuration(ClientHandshakeConfigurationError),
    EncodeClientHello(Encode),
    EncodedRouteMismatch {
        actual: FrameRoute,
    },
    Send(Transport),
    EndOfStream,
    Receive(Transport),
    HandshakeRequired {
        actual: FrameRoute,
    },
    DecodeServerHello(Decode),
    SelectedVersionMismatch {
        outer: pwright_core::ProtocolVersion,
        selected: pwright_core::ProtocolVersion,
    },
    Rejected(ClientHelloValidationError),
}

impl<Encode, Decode, Transport> Display for ClientHandshakeError<Encode, Decode, Transport>
where
    Encode: Display,
    Decode: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(error) => Display::fmt(error, formatter),
            Self::EncodeClientHello(error) => {
                write!(formatter, "failed to encode client hello: {error}")
            }
            Self::EncodedRouteMismatch { actual } => write!(
                formatter,
                "encoded client hello has route {actual:?}, expected Handshake"
            ),
            Self::Send(error) => write!(formatter, "failed to send client hello: {error}"),
            Self::EndOfStream => formatter.write_str("connection ended before server hello"),
            Self::Receive(error) => write!(formatter, "failed to receive server hello: {error}"),
            Self::HandshakeRequired { actual } => {
                write!(formatter, "expected server hello, received {actual:?}")
            }
            Self::DecodeServerHello(error) => {
                write!(formatter, "failed to decode server hello: {error}")
            }
            Self::SelectedVersionMismatch { outer, selected } => write!(
                formatter,
                "server hello frame uses version {outer}, but its payload selected {selected}"
            ),
            Self::Rejected(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Encode, Decode, Transport> Error for ClientHandshakeError<Encode, Decode, Transport>
where
    Encode: Error + 'static,
    Decode: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Configuration(error) => Some(error),
            Self::EncodeClientHello(error) => Some(error),
            Self::Send(error) | Self::Receive(error) => Some(error),
            Self::DecodeServerHello(error) => Some(error),
            Self::Rejected(error) => Some(error),
            Self::EncodedRouteMismatch { .. }
            | Self::EndOfStream
            | Self::HandshakeRequired { .. }
            | Self::SelectedVersionMismatch { .. } => None,
        }
    }
}

/// Owns a transport until one client/server hello exchange succeeds.
///
/// `establish` consumes this gate. Dropping its future therefore drops the
/// transport too, making an interrupted handshake impossible to reuse after a
/// partial frame write or read.
pub struct EncodedClientHandshake<Transport> {
    transport: Transport,
}

impl<Transport> EncodedClientHandshake<Transport> {
    pub const fn new(transport: Transport) -> Self {
        Self { transport }
    }
}

impl<Transport> EncodedClientHandshake<Transport>
where
    Transport: FrameTransport,
{
    pub async fn establish<Codec>(
        self,
        protocol: WireProtocol<Codec>,
        client: ClientHello,
    ) -> Result<
        NegotiatedClientTransport<Transport>,
        ClientHandshakeError<
            WireProtocolError<<Codec as WireCodec<ClientHello>>::Error>,
            WireProtocolError<<Codec as WireCodec<ServerHello>>::Error>,
            Transport::Error,
        >,
    >
    where
        Codec: WireCodec<ClientHello> + WireCodec<ServerHello>,
        <Codec as WireCodec<ClientHello>>::Error: Display,
        <Codec as WireCodec<ServerHello>>::Error: Display,
    {
        if protocol.versions() != client.versions() {
            return Err(ClientHandshakeError::Configuration(
                ClientHandshakeConfigurationError::VersionPolicyMismatch,
            ));
        }
        if protocol.receive_limits() != client.receive_limits() {
            return Err(ClientHandshakeError::Configuration(
                ClientHandshakeConfigurationError::ReceiveLimitsMismatch {
                    protocol: protocol.receive_limits(),
                    advertised: client.receive_limits(),
                },
            ));
        }

        let frame = protocol
            .encode(&client)
            .map_err(ClientHandshakeError::EncodeClientHello)?;
        if frame.route() != FrameRoute::Handshake {
            return Err(ClientHandshakeError::EncodedRouteMismatch {
                actual: frame.route(),
            });
        }
        self.transport
            .send(frame)
            .await
            .map_err(ClientHandshakeError::Send)?;
        let response = self
            .transport
            .receive()
            .await
            .map_err(ClientHandshakeError::Receive)?
            .ok_or(ClientHandshakeError::EndOfStream)?;
        if response.route() != FrameRoute::Handshake {
            return Err(ClientHandshakeError::HandshakeRequired {
                actual: response.route(),
            });
        }
        let outer = response.version();
        let server = protocol
            .decode::<ServerHello>(response)
            .map_err(ClientHandshakeError::DecodeServerHello)?;
        if outer != server.selected_version() {
            return Err(ClientHandshakeError::SelectedVersionMismatch {
                outer,
                selected: server.selected_version(),
            });
        }
        validate_server_hello(&client, &server).map_err(ClientHandshakeError::Rejected)?;
        Ok(NegotiatedClientTransport {
            transport: self.transport,
            hello: server,
        })
    }
}

pub struct NegotiatedClientTransport<Transport> {
    transport: Transport,
    hello: ServerHello,
}

pub type EncodedClientParts<Codec, Transport> = (
    EncodedClientChannel<Codec, Transport>,
    EncodedClientDriver<Codec, Transport>,
);

impl<Transport> NegotiatedClientTransport<Transport> {
    pub const fn hello(&self) -> &ServerHello {
        &self.hello
    }

    pub fn into_parts(self) -> (Transport, ServerHello) {
        (self.transport, self.hello)
    }

    pub fn into_channel<Codec>(
        self,
        codec: Codec,
        pending_call_limit: usize,
    ) -> EncodedClientParts<Codec, Transport> {
        let protocol = WireProtocol::new_exact_directional(
            codec,
            self.hello.selected_version(),
            self.hello.server_receive_limits(),
            self.hello.client_receive_limits(),
        );
        EncodedClientChannel::new(protocol, self.transport, pending_call_limit)
    }

    pub fn into_channel_with_resource_ledger<Codec>(
        self,
        codec: Codec,
        pending_call_limit: usize,
        resource_ledger: ResourceLedger,
    ) -> EncodedClientParts<Codec, Transport> {
        let protocol = WireProtocol::new_exact_directional(
            codec,
            self.hello.selected_version(),
            self.hello.server_receive_limits(),
            self.hello.client_receive_limits(),
        );
        EncodedClientChannel::new_with_resource_ledger(
            protocol,
            self.transport,
            pending_call_limit,
            resource_ledger,
        )
    }

    pub fn into_channel_with_production_resources<Codec>(
        self,
        codec: Codec,
        pending_call_limit: usize,
        resources: &ProductionResourceLedger,
    ) -> Result<EncodedClientParts<Codec, Transport>, ClientWireProductionError>
    where
        Transport: ProductionFrameTransport,
    {
        let protocol = WireProtocol::new_exact_directional(
            codec,
            self.hello.selected_version(),
            self.hello.server_receive_limits(),
            self.hello.client_receive_limits(),
        );
        EncodedClientChannel::new_with_production_resources(
            protocol,
            self.transport,
            pending_call_limit,
            resources,
        )
    }

    pub fn into_channel_with_control<Codec, Control>(
        self,
        codec: Codec,
        control: Control,
        pending_call_limit: usize,
    ) -> Result<EncodedClientParts<Codec, Transport>, ClientWireConfigurationError>
    where
        Control: WireCodec<ControlMessage> + 'static,
        <Control as WireCodec<ControlMessage>>::Error: Display,
    {
        let version = self.hello.selected_version();
        let send_limits = self.hello.server_receive_limits();
        let receive_limits = self.hello.client_receive_limits();
        EncodedClientChannel::new_with_control(
            WireProtocol::new_exact_directional(codec, version, send_limits, receive_limits),
            WireProtocol::new_exact_directional(control, version, send_limits, receive_limits),
            self.transport,
            pending_call_limit,
        )
    }

    pub fn into_channel_with_control_and_resource_ledger<Codec, Control>(
        self,
        codec: Codec,
        control: Control,
        pending_call_limit: usize,
        resource_ledger: ResourceLedger,
    ) -> Result<EncodedClientParts<Codec, Transport>, ClientWireConfigurationError>
    where
        Control: WireCodec<ControlMessage> + 'static,
        <Control as WireCodec<ControlMessage>>::Error: Display,
    {
        let version = self.hello.selected_version();
        let send_limits = self.hello.server_receive_limits();
        let receive_limits = self.hello.client_receive_limits();
        EncodedClientChannel::new_with_control_and_resource_ledger(
            WireProtocol::new_exact_directional(codec, version, send_limits, receive_limits),
            WireProtocol::new_exact_directional(control, version, send_limits, receive_limits),
            self.transport,
            pending_call_limit,
            resource_ledger,
        )
    }

    pub fn into_channel_with_control_and_production_resources<Codec, Control>(
        self,
        codec: Codec,
        control: Control,
        pending_call_limit: usize,
        resources: &ProductionResourceLedger,
    ) -> Result<EncodedClientParts<Codec, Transport>, ClientWireProductionError>
    where
        Transport: ProductionFrameTransport,
        Control: WireCodec<ControlMessage> + 'static,
        <Control as WireCodec<ControlMessage>>::Error: Display,
    {
        let version = self.hello.selected_version();
        let send_limits = self.hello.server_receive_limits();
        let receive_limits = self.hello.client_receive_limits();
        EncodedClientChannel::new_with_control_and_production_resources(
            WireProtocol::new_exact_directional(codec, version, send_limits, receive_limits),
            WireProtocol::new_exact_directional(control, version, send_limits, receive_limits),
            self.transport,
            pending_call_limit,
            resources,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelCloseReason {
    Explicit,
    EndOfStream,
    SendFailed,
    ReceiveFailed,
    DriverDropped,
    ProtocolViolation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientWireStateError {
    StateUnavailable,
}

#[derive(Debug)]
pub enum ClientTransportCloseError<Transport> {
    State(ClientWireStateError),
    SendCapacityExceeded { limit: usize },
    SendTicketExhausted,
    Transport(Transport),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClientGracefulCloseOutcome {
    woken_calls: usize,
    ignored_frames: usize,
}

impl ClientGracefulCloseOutcome {
    pub const fn woken_calls(self) -> usize {
        self.woken_calls
    }

    pub const fn ignored_frames(self) -> usize {
        self.ignored_frames
    }
}

#[derive(Debug)]
pub enum ClientGracefulCloseError<Transport> {
    State(ClientWireStateError),
    StateMachine(ClientCloseError),
    MissingControlProtocol,
    SendCapacityExceeded {
        limit: usize,
    },
    SendTicketExhausted,
    EncodedRouteMismatch {
        actual: FrameRoute,
    },
    Protocol(String),
    EndOfStreamBeforeAck,
    ApplicationVersionMismatch {
        expected: pwright_core::ProtocolVersion,
        actual: pwright_core::ProtocolVersion,
    },
    IncomingFrame(FrameError),
    Send(Transport),
    Receive(Transport),
    Shutdown(Transport),
    Terminated,
}

impl<Transport> Display for ClientGracefulCloseError<Transport>
where
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::State(error) => Display::fmt(error, formatter),
            Self::StateMachine(error) => Display::fmt(error, formatter),
            Self::MissingControlProtocol => {
                formatter.write_str("graceful close requires a negotiated control protocol")
            }
            Self::SendCapacityExceeded { limit } => write!(
                formatter,
                "graceful close reached the encoded send limit of {limit}"
            ),
            Self::SendTicketExhausted => {
                formatter.write_str("graceful close exhausted encoded send ticket space")
            }
            Self::EncodedRouteMismatch { actual } => write!(
                formatter,
                "encoded close message has route {actual:?}, expected Control"
            ),
            Self::Protocol(error) => write!(formatter, "graceful close protocol failed: {error}"),
            Self::EndOfStreamBeforeAck => formatter.write_str("connection ended before CloseAck"),
            Self::ApplicationVersionMismatch { expected, actual } => write!(
                formatter,
                "frame received during graceful close uses version {actual}, expected {expected}"
            ),
            Self::IncomingFrame(error) => Display::fmt(error, formatter),
            Self::Send(error) => write!(formatter, "failed to send Close: {error}"),
            Self::Receive(error) => write!(formatter, "failed while awaiting CloseAck: {error}"),
            Self::Shutdown(error) => write!(formatter, "failed to shut down write half: {error}"),
            Self::Terminated => formatter.write_str("graceful close state is terminated"),
        }
    }
}

impl<Transport> Error for ClientGracefulCloseError<Transport>
where
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::State(error) => Some(error),
            Self::StateMachine(error) => Some(error),
            Self::IncomingFrame(error) => Some(error),
            Self::Send(error) | Self::Receive(error) | Self::Shutdown(error) => Some(error),
            Self::MissingControlProtocol
            | Self::SendCapacityExceeded { .. }
            | Self::SendTicketExhausted
            | Self::EncodedRouteMismatch { .. }
            | Self::Protocol(_)
            | Self::EndOfStreamBeforeAck
            | Self::ApplicationVersionMismatch { .. }
            | Self::Terminated => None,
        }
    }
}

impl<Transport> Display for ClientTransportCloseError<Transport>
where
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::State(error) => Display::fmt(error, formatter),
            Self::SendCapacityExceeded { limit } => write!(
                formatter,
                "encoded client shutdown send limit of {limit} was reached"
            ),
            Self::SendTicketExhausted => {
                formatter.write_str("encoded client shutdown send ticket space was exhausted")
            }
            Self::Transport(error) => write!(formatter, "transport shutdown failed: {error}"),
        }
    }
}

impl<Transport> Error for ClientTransportCloseError<Transport>
where
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::State(error) => Some(error),
            Self::Transport(error) => Some(error),
            Self::SendCapacityExceeded { .. } | Self::SendTicketExhausted => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientWireConfigurationError {
    ControlVersionMismatch,
    ControlVersionPolicyMismatch,
    ControlFrameLimitsMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClientWireProductionProfile {
    requirements: ResourceRequirements,
}

impl ClientWireProductionProfile {
    /// Combines bounded channel backlog with a production transport profile.
    /// The transport profile already includes its active outbound and inbound
    /// frames, so only additional queued client frames are added.
    pub fn new<Codec>(
        protocol: &WireProtocol<Codec>,
        pending_call_limit: usize,
        transport: ResourceRequirements,
    ) -> Self {
        Self {
            requirements: client_wire_production_requirements(
                protocol,
                pending_call_limit,
                transport,
            ),
        }
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

#[derive(Debug)]
pub enum ClientWireProductionError {
    Configuration(ClientWireConfigurationError),
    Resource(ProductionResourceError),
    TransportLedgerMismatch,
}

impl Display for ClientWireProductionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(error) => Display::fmt(error, formatter),
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::TransportLedgerMismatch => formatter.write_str(
                "client channel and production transport must use the same connection ledger",
            ),
        }
    }
}

impl Error for ClientWireProductionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Configuration(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::TransportLedgerMismatch => None,
        }
    }
}

impl From<ClientWireConfigurationError> for ClientWireProductionError {
    fn from(error: ClientWireConfigurationError) -> Self {
        Self::Configuration(error)
    }
}

impl From<ProductionResourceError> for ClientWireProductionError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resource(error)
    }
}

fn client_wire_production_requirements<Codec>(
    protocol: &WireProtocol<Codec>,
    pending_call_limit: usize,
    transport: ResourceRequirements,
) -> ResourceRequirements {
    let send_frame = u64::try_from(protocol.send_limits().max_frame_bytes()).unwrap_or(u64::MAX);
    let receive_frame =
        u64::try_from(protocol.receive_limits().max_frame_bytes()).unwrap_or(u64::MAX);
    let frame = send_frame.max(receive_frame);
    let pending = u64::try_from(pending_call_limit).unwrap_or(u64::MAX);
    let transport_wire = transport.class(ResourceClass::WireBytes);

    // During full duplex the transport profile already owns one outbound and
    // one inbound frame. At most N-1 additional channel frames can belong to
    // the N pending slots. Separately, encoding happens before registration,
    // so N retained frames plus one candidate frame is another attainable peak.
    let additional_frames = pending.saturating_sub(1).saturating_mul(frame);
    let full_duplex = transport_wire.global.saturating_add(additional_frames);
    let pre_registration = pending.saturating_add(1).saturating_mul(frame);
    let wire_global = full_duplex.max(pre_registration);
    let duplicate_scope = if pending_call_limit == 0 {
        frame
    } else {
        frame.saturating_mul(2)
    };
    let wire_per_scope = transport_wire.per_scope.max(duplicate_scope);

    let transport_pending = transport.class(ResourceClass::PendingCall);
    let pending_global = transport_pending.global.saturating_add(pending);
    let pending_per_scope = transport_pending
        .per_scope
        .saturating_add(u64::from(pending_call_limit != 0));
    let additional_scopes = pending_call_limit.saturating_sub(1);
    let full_duplex_scopes = transport.scopes.saturating_add(additional_scopes);
    let pre_registration_scopes = pending_call_limit.saturating_add(1);
    let scopes = full_duplex_scopes.max(pre_registration_scopes);
    let full_duplex_leases = transport
        .leases
        .saturating_add(pending_call_limit)
        .saturating_add(additional_scopes);
    let pre_registration_leases = pending_call_limit.saturating_mul(2).saturating_add(1);
    let leases = full_duplex_leases.max(pre_registration_leases);

    transport
        .with_class(
            ResourceClass::PendingCall,
            pending_global,
            pending_per_scope,
        )
        .with_class(ResourceClass::WireBytes, wire_global, wire_per_scope)
        .with_metadata(scopes, leases)
}

impl Display for ClientWireConfigurationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ControlVersionMismatch => {
                formatter.write_str("command and control protocols must send the same wire version")
            }
            Self::ControlVersionPolicyMismatch => formatter
                .write_str("command and control protocols must accept the same wire-version range"),
            Self::ControlFrameLimitsMismatch => {
                formatter.write_str("command and control protocols must use the same frame limits")
            }
        }
    }
}

impl Error for ClientWireConfigurationError {}

impl Display for ClientWireStateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("encoded client channel state is unavailable")
    }
}

impl Error for ClientWireStateError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClientWireMetrics {
    awaiting_results: usize,
    waiting_calls: usize,
    abandoned_calls: usize,
    queued_cancellations: usize,
    sent_cancellations: u64,
    peak_queued_cancellations: usize,
    peak_slots: usize,
    limit: usize,
    close_reason: Option<ChannelCloseReason>,
}

impl ClientWireMetrics {
    pub const fn awaiting_results(self) -> usize {
        self.awaiting_results
    }

    pub const fn waiting_calls(self) -> usize {
        self.waiting_calls
    }

    pub const fn abandoned_calls(self) -> usize {
        self.abandoned_calls
    }

    pub const fn queued_cancellations(self) -> usize {
        self.queued_cancellations
    }

    pub const fn sent_cancellations(self) -> u64 {
        self.sent_cancellations
    }

    pub const fn peak_queued_cancellations(self) -> usize {
        self.peak_queued_cancellations
    }

    pub const fn peak_slots(self) -> usize {
        self.peak_slots
    }

    pub const fn limit(self) -> usize {
        self.limit
    }

    pub const fn close_reason(self) -> Option<ChannelCloseReason> {
        self.close_reason
    }
}

pub enum DriverEvent {
    Delivered(CorrelatedCall),
    IgnoredAbandoned(CallId),
    Unhandled(WireFrame),
    Closed(ChannelCloseReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancellationFlushOutcome {
    Idle,
    Sent {
        call_id: CallId,
        cancellation: CancellationId,
    },
}

#[derive(Debug)]
pub enum CancellationFlushError<Transport> {
    StateUnavailable,
    SendCapacityExceeded { limit: usize },
    SendTicketExhausted,
    MissingControlProtocol,
    Protocol(String),
    InvalidRoute(FrameRoute),
    Transport(Transport),
}

impl<Transport> Display for CancellationFlushError<Transport>
where
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::StateUnavailable => {
                formatter.write_str("encoded client channel state is unavailable")
            }
            Self::SendCapacityExceeded { limit } => {
                write!(
                    formatter,
                    "encoded client send limit of {limit} was reached"
                )
            }
            Self::SendTicketExhausted => {
                formatter.write_str("encoded client send ticket space was exhausted")
            }
            Self::MissingControlProtocol => {
                formatter.write_str("encoded client channel has no cancellation control protocol")
            }
            Self::Protocol(error) => write!(formatter, "cancellation encoding failed: {error}"),
            Self::InvalidRoute(route) => {
                write!(
                    formatter,
                    "cancellation encoded onto invalid route {route:?}"
                )
            }
            Self::Transport(error) => write!(formatter, "cancellation send failed: {error}"),
        }
    }
}

impl<Transport> Error for CancellationFlushError<Transport>
where
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            Self::StateUnavailable
            | Self::SendCapacityExceeded { .. }
            | Self::SendTicketExhausted
            | Self::MissingControlProtocol
            | Self::Protocol(_)
            | Self::InvalidRoute(_) => None,
        }
    }
}

#[derive(Debug)]
pub enum CancellationAwareDriverError<Transport> {
    Driver(DriverError<Transport>),
    Cancellation(CancellationFlushError<Transport>),
}

impl<Transport> Display for CancellationAwareDriverError<Transport>
where
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Driver(error) => Display::fmt(error, formatter),
            Self::Cancellation(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Transport> Error for CancellationAwareDriverError<Transport>
where
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Driver(error) => Some(error),
            Self::Cancellation(error) => Some(error),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameDisposition<Event> {
    Handled(Event),
    Unhandled(WireFrame),
}

pub trait FrameHandler {
    type Event;
    type Error;

    fn handle(&mut self, frame: WireFrame) -> Result<FrameDisposition<Self::Event>, Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Either<Left, Right> {
    Left(Left),
    Right(Right),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HandlerChainError<Left, Right> {
    Left(Left),
    Right(Right),
}

impl<Left, Right> Display for HandlerChainError<Left, Right>
where
    Left: Display,
    Right: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left(error) => Display::fmt(error, formatter),
            Self::Right(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Left, Right> Error for HandlerChainError<Left, Right>
where
    Left: Error + 'static,
    Right: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Left(error) => Some(error),
            Self::Right(error) => Some(error),
        }
    }
}

pub struct HandlerChain<Left, Right> {
    left: Left,
    right: Right,
}

impl<Left, Right> HandlerChain<Left, Right> {
    pub const fn new(left: Left, right: Right) -> Self {
        Self { left, right }
    }

    pub fn into_parts(self) -> (Left, Right) {
        (self.left, self.right)
    }
}

impl<Left, Right> FrameHandler for HandlerChain<Left, Right>
where
    Left: FrameHandler,
    Right: FrameHandler,
{
    type Event = Either<Left::Event, Right::Event>;
    type Error = HandlerChainError<Left::Error, Right::Error>;

    fn handle(&mut self, frame: WireFrame) -> Result<FrameDisposition<Self::Event>, Self::Error> {
        match self.left.handle(frame).map_err(HandlerChainError::Left)? {
            FrameDisposition::Handled(event) => Ok(FrameDisposition::Handled(Either::Left(event))),
            FrameDisposition::Unhandled(frame) => self
                .right
                .handle(frame)
                .map(|disposition| match disposition {
                    FrameDisposition::Handled(event) => {
                        FrameDisposition::Handled(Either::Right(event))
                    }
                    FrameDisposition::Unhandled(frame) => FrameDisposition::Unhandled(frame),
                })
                .map_err(HandlerChainError::Right),
        }
    }
}

pub trait FrameHandlerExt: FrameHandler + Sized {
    fn or_else<Next>(self, next: Next) -> HandlerChain<Self, Next>
    where
        Next: FrameHandler,
    {
        HandlerChain::new(self, next)
    }
}

impl<Handler> FrameHandlerExt for Handler where Handler: FrameHandler {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoutedDriverEvent<Event> {
    Delivered(CorrelatedCall),
    IgnoredAbandoned(CallId),
    Handled(Event),
    Unhandled(WireFrame),
    Closed(ChannelCloseReason),
}

#[derive(Debug)]
pub enum RoutedDriverError<Transport, Handler> {
    Driver(DriverError<Transport>),
    Handler(Handler),
}

impl<Transport, Handler> Display for RoutedDriverError<Transport, Handler>
where
    Transport: Display,
    Handler: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Driver(error) => Display::fmt(error, formatter),
            Self::Handler(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Transport, Handler> Error for RoutedDriverError<Transport, Handler>
where
    Transport: Error + 'static,
    Handler: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Driver(error) => Some(error),
            Self::Handler(error) => Some(error),
        }
    }
}

impl fmt::Debug for DriverEvent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Delivered(call) => formatter.debug_tuple("Delivered").field(call).finish(),
            Self::IgnoredAbandoned(call_id) => formatter
                .debug_tuple("IgnoredAbandoned")
                .field(call_id)
                .finish(),
            Self::Unhandled(frame) => formatter
                .debug_tuple("Unhandled")
                .field(&frame.route())
                .finish(),
            Self::Closed(reason) => formatter.debug_tuple("Closed").field(reason).finish(),
        }
    }
}

#[derive(Debug)]
pub enum DriverError<E> {
    Transport(E),
    Correlation(CorrelationError),
    StateUnavailable,
}

impl<E> Display for DriverError<E>
where
    E: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => write!(formatter, "frame receive failed: {error}"),
            Self::Correlation(error) => Display::fmt(error, formatter),
            Self::StateUnavailable => {
                formatter.write_str("encoded client channel state is unavailable")
            }
        }
    }
}

impl<E> Error for DriverError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            Self::Correlation(error) => Some(error),
            Self::StateUnavailable => None,
        }
    }
}

struct PendingCall {
    frame: Option<WireFrame>,
    waker: Option<Waker>,
    cancellation: Option<CancellationId>,
}

struct ChannelState {
    correlator: CallCorrelator,
    calls: BTreeMap<CallId, PendingCall>,
    abandoned: BTreeSet<CallId>,
    pending_resources: BTreeMap<CallId, ResourceLease>,
    resource_ledger: Option<ResourceLedger>,
    cancellations: BTreeMap<CallId, CancellationId>,
    cancellation_waker: Option<Waker>,
    sent_cancellations: u64,
    peak_queued_cancellations: usize,
    peak_slots: usize,
    limit: usize,
    close_reason: Option<ChannelCloseReason>,
}

impl ChannelState {
    fn new(limit: usize, resource_ledger: Option<ResourceLedger>) -> Self {
        Self {
            correlator: CallCorrelator::new(limit),
            calls: BTreeMap::new(),
            abandoned: BTreeSet::new(),
            pending_resources: BTreeMap::new(),
            resource_ledger,
            cancellations: BTreeMap::new(),
            cancellation_waker: None,
            sent_cancellations: 0,
            peak_queued_cancellations: 0,
            peak_slots: 0,
            limit,
            close_reason: None,
        }
    }

    fn slots(&self) -> usize {
        self.calls.len() + self.abandoned.len()
    }

    fn reserve_pending(&self, call_id: CallId) -> Result<Option<ResourceLease>, RegistrationError> {
        self.resource_ledger
            .as_ref()
            .map(|ledger| {
                ledger.reserve(ResourceScope::Call(call_id), ResourceClass::PendingCall, 1)
            })
            .transpose()
            .map_err(RegistrationError::Resource)
    }

    fn release_pending(&mut self, call_id: CallId) {
        self.pending_resources.remove(&call_id);
    }
}

type ControlEncoder = dyn Fn(&ControlMessage) -> Result<WireFrame, String> + Send + Sync + 'static;
type ControlDecoder = dyn Fn(WireFrame) -> Result<ControlMessage, String> + Send + Sync + 'static;

struct ControlPlane {
    encode: Arc<ControlEncoder>,
    decode: Arc<ControlDecoder>,
}

impl ControlPlane {
    fn new<Control>(protocol: WireProtocol<Control>) -> Self
    where
        Control: WireCodec<ControlMessage> + 'static,
        <Control as WireCodec<ControlMessage>>::Error: Display,
    {
        let protocol = Arc::new(protocol);
        let encode_protocol = Arc::clone(&protocol);
        Self {
            encode: Arc::new(move |message| {
                encode_protocol
                    .encode(message)
                    .map_err(|error| error.to_string())
            }),
            decode: Arc::new(move |frame| {
                protocol
                    .decode::<ControlMessage>(frame)
                    .map_err(|error| error.to_string())
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SendPriority {
    Command,
    Control,
}

struct SendWaiter {
    ticket: u64,
    waker: Option<Waker>,
}

struct SendGateState {
    active: Option<u64>,
    control: VecDeque<SendWaiter>,
    commands: VecDeque<SendWaiter>,
    next_ticket: u64,
}

impl SendGateState {
    fn waiting(&self) -> usize {
        self.control.len() + self.commands.len()
    }

    fn front_ticket(&self) -> Option<u64> {
        self.control
            .front()
            .or_else(|| self.commands.front())
            .map(|waiter| waiter.ticket)
    }

    fn waiter_mut(&mut self, ticket: u64) -> Option<&mut SendWaiter> {
        self.control
            .iter_mut()
            .chain(self.commands.iter_mut())
            .find(|waiter| waiter.ticket == ticket)
    }

    fn remove_waiter(&mut self, ticket: u64) -> bool {
        for queue in [&mut self.control, &mut self.commands] {
            if let Some(index) = queue.iter().position(|waiter| waiter.ticket == ticket) {
                queue.remove(index);
                return true;
            }
        }
        false
    }

    fn pop_front(&mut self) -> Option<SendWaiter> {
        self.control
            .pop_front()
            .or_else(|| self.commands.pop_front())
    }

    fn next_waker(&self) -> Option<Waker> {
        self.control
            .front()
            .or_else(|| self.commands.front())
            .and_then(|waiter| waiter.waker.clone())
    }
}

struct SendGate {
    state: Mutex<SendGateState>,
    limit: usize,
}

impl SendGate {
    fn new(pending_call_limit: usize) -> Self {
        Self {
            state: Mutex::new(SendGateState {
                active: None,
                control: VecDeque::new(),
                commands: VecDeque::new(),
                next_ticket: 0,
            }),
            // Every pending slot can own one command-send waiter, while the
            // unique receive driver can own one control-send waiter.
            limit: pending_call_limit.saturating_add(1),
        }
    }

    fn acquire(
        self: &Arc<Self>,
        priority: SendPriority,
    ) -> Result<SendPermitFuture, SendGateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| SendGateError::StateUnavailable)?;
        let occupied = usize::from(state.active.is_some()).saturating_add(state.waiting());
        if occupied >= self.limit {
            return Err(SendGateError::CapacityExceeded { limit: self.limit });
        }
        let ticket = state.next_ticket;
        state.next_ticket = state
            .next_ticket
            .checked_add(1)
            .ok_or(SendGateError::TicketExhausted)?;
        let waiter = SendWaiter {
            ticket,
            waker: None,
        };
        match priority {
            SendPriority::Command => state.commands.push_back(waiter),
            SendPriority::Control => state.control.push_back(waiter),
        }
        Ok(SendPermitFuture {
            gate: Arc::clone(self),
            ticket,
            acquired: false,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SendGateError {
    CapacityExceeded { limit: usize },
    StateUnavailable,
    TicketExhausted,
}

struct SendPermitFuture {
    gate: Arc<SendGate>,
    ticket: u64,
    acquired: bool,
}

impl Future for SendPermitFuture {
    type Output = Result<SendPermit, SendGateError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let gate = Arc::clone(&self.gate);
        let mut state = match gate.state.lock() {
            Ok(state) => state,
            Err(_) => return Poll::Ready(Err(SendGateError::StateUnavailable)),
        };
        if state.active.is_none() && state.front_ticket() == Some(self.ticket) {
            let waiter = state
                .pop_front()
                .expect("the ready send waiter must remain queued");
            state.active = Some(waiter.ticket);
            drop(state);
            self.acquired = true;
            return Poll::Ready(Ok(SendPermit {
                gate,
                ticket: self.ticket,
            }));
        }
        let Some(waiter) = state.waiter_mut(self.ticket) else {
            return Poll::Ready(Err(SendGateError::StateUnavailable));
        };
        if waiter
            .waker
            .as_ref()
            .is_none_or(|registered| !registered.will_wake(context.waker()))
        {
            waiter.waker = Some(context.waker().clone());
        }
        Poll::Pending
    }
}

impl Drop for SendPermitFuture {
    fn drop(&mut self) {
        if self.acquired {
            return;
        }
        let waker = {
            let Ok(mut state) = self.gate.state.lock() else {
                return;
            };
            let was_front = state.front_ticket() == Some(self.ticket);
            if !state.remove_waiter(self.ticket) || !was_front || state.active.is_some() {
                None
            } else {
                state.next_waker()
            }
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

struct SendPermit {
    gate: Arc<SendGate>,
    ticket: u64,
}

impl Drop for SendPermit {
    fn drop(&mut self) {
        let waker = {
            let Ok(mut state) = self.gate.state.lock() else {
                return;
            };
            if state.active != Some(self.ticket) {
                return;
            }
            state.active = None;
            state.next_waker()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

struct Shared<C, T> {
    protocol: WireProtocol<C>,
    control: Option<ControlPlane>,
    transport: T,
    send_gate: Arc<SendGate>,
    state: Mutex<ChannelState>,
}

impl<C, T> Shared<C, T> {
    fn register(
        &self,
        call_id: CallId,
        target: pwright_core::ObjectId,
        method: pwright_core::MethodTag,
        cancellation: Option<CancellationId>,
    ) -> Result<(), RegistrationError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RegistrationError::StateUnavailable)?;
        if let Some(reason) = state.close_reason {
            return Err(RegistrationError::Closed(reason));
        }
        if state.slots() >= state.limit {
            return Err(RegistrationError::CapacityExceeded { limit: state.limit });
        }
        if state.calls.contains_key(&call_id) || state.abandoned.contains(&call_id) {
            return Err(RegistrationError::Correlation(
                CorrelationError::DuplicateCall { call_id },
            ));
        }
        let resource = state.reserve_pending(call_id)?;
        state
            .correlator
            .register(call_id, target, method)
            .map_err(RegistrationError::Correlation)?;
        let previous = state.calls.insert(
            call_id,
            PendingCall {
                frame: None,
                waker: None,
                cancellation,
            },
        );
        debug_assert!(previous.is_none());
        if let Some(resource) = resource {
            let previous = state.pending_resources.insert(call_id, resource);
            debug_assert!(previous.is_none());
        }
        state.peak_slots = state.peak_slots.max(state.slots());
        Ok(())
    }

    fn abandon(&self, call_id: CallId, sent: bool) {
        let waker = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            let Some(call) = state.calls.remove(&call_id) else {
                return;
            };
            let was_awaiting = state.correlator.complete(call_id).is_ok();
            if sent && was_awaiting && call.frame.is_none() && state.close_reason.is_none() {
                state.abandoned.insert(call_id);
                if self.control.is_some()
                    && let Some(cancellation) = call.cancellation
                {
                    state.cancellations.insert(call_id, cancellation);
                    state.peak_queued_cancellations = state
                        .peak_queued_cancellations
                        .max(state.cancellations.len());
                    state.cancellation_waker.take()
                } else {
                    None
                }
            } else {
                state.release_pending(call_id);
                None
            }
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn close(&self, reason: ChannelCloseReason) -> Result<usize, ClientWireStateError> {
        let wakers = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| ClientWireStateError::StateUnavailable)?;
            if state.close_reason.is_some() {
                return Ok(0);
            }
            state.close_reason = Some(reason);
            state.correlator.drain();
            state.abandoned.clear();
            state.pending_resources.clear();
            state.cancellations.clear();
            state.cancellation_waker = None;
            let calls = std::mem::take(&mut state.calls);
            calls
                .into_values()
                .filter_map(|call| call.waker)
                .collect::<Vec<_>>()
        };
        let count = wakers.len();
        for waker in wakers {
            waker.wake();
        }
        Ok(count)
    }

    fn close_reason(&self) -> Result<Option<ChannelCloseReason>, ClientWireStateError> {
        self.state
            .lock()
            .map(|state| state.close_reason)
            .map_err(|_| ClientWireStateError::StateUnavailable)
    }

    fn metrics(&self) -> Result<ClientWireMetrics, ClientWireStateError> {
        let state = self
            .state
            .lock()
            .map_err(|_| ClientWireStateError::StateUnavailable)?;
        Ok(ClientWireMetrics {
            awaiting_results: state.correlator.len(),
            waiting_calls: state.calls.len(),
            abandoned_calls: state.abandoned.len(),
            queued_cancellations: state.cancellations.len(),
            sent_cancellations: state.sent_cancellations,
            peak_queued_cancellations: state.peak_queued_cancellations,
            peak_slots: state.peak_slots,
            limit: state.limit,
            close_reason: state.close_reason,
        })
    }

    fn deliver(&self, frame: WireFrame) -> Result<DriverEvent, DriverError<Infallible>> {
        let FrameRoute::Result { call_id } = frame.route() else {
            return Ok(DriverEvent::Unhandled(frame));
        };
        let (call, waker) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| DriverError::StateUnavailable)?;
            if state.abandoned.remove(&call_id) {
                state.cancellations.remove(&call_id);
                state.release_pending(call_id);
                return Ok(DriverEvent::IgnoredAbandoned(call_id));
            }
            let correlated = state
                .correlator
                .complete(call_id)
                .map_err(DriverError::Correlation)?;
            let waker = {
                let pending = state
                    .calls
                    .get_mut(&call_id)
                    .ok_or(DriverError::Correlation(CorrelationError::UnknownResult {
                        call_id,
                    }))?;
                pending.frame = Some(frame);
                pending.waker.take()
            };
            state.release_pending(call_id);
            (correlated, waker)
        };
        if let Some(waker) = waker {
            waker.wake();
        }
        Ok(DriverEvent::Delivered(call))
    }

    fn next_cancellation(&self) -> Result<Option<(CallId, CancellationId)>, ClientWireStateError> {
        self.state
            .lock()
            .map(|state| {
                state
                    .cancellations
                    .first_key_value()
                    .map(|(call_id, cancellation)| (*call_id, *cancellation))
            })
            .map_err(|_| ClientWireStateError::StateUnavailable)
    }

    fn complete_cancellation(&self, call_id: CallId) -> Result<(), ClientWireStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ClientWireStateError::StateUnavailable)?;
        if state.cancellations.remove(&call_id).is_some() {
            state.sent_cancellations = state.sent_cancellations.saturating_add(1);
        }
        Ok(())
    }

    fn poll_cancellation_ready(
        &self,
        context: &mut Context<'_>,
    ) -> Result<bool, ClientWireStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ClientWireStateError::StateUnavailable)?;
        if !state.cancellations.is_empty() {
            return Ok(true);
        }
        if state
            .cancellation_waker
            .as_ref()
            .is_none_or(|registered| !registered.will_wake(context.waker()))
        {
            state.cancellation_waker = Some(context.waker().clone());
        }
        Ok(false)
    }
}

enum RegistrationError {
    Closed(ChannelCloseReason),
    CapacityExceeded { limit: usize },
    Correlation(CorrelationError),
    Resource(ResourceError),
    StateUnavailable,
}

enum WaitError {
    Closed(ChannelCloseReason),
    MissingCall,
    StateUnavailable,
}

struct RawResponse<C, T> {
    shared: Arc<Shared<C, T>>,
    call_id: CallId,
}

impl<C, T> Future for RawResponse<C, T> {
    type Output = Result<WireFrame, WaitError>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = match self.shared.state.lock() {
            Ok(state) => state,
            Err(_) => return Poll::Ready(Err(WaitError::StateUnavailable)),
        };
        if let Some(reason) = state.close_reason {
            return Poll::Ready(Err(WaitError::Closed(reason)));
        }
        let Some(pending) = state.calls.get_mut(&self.call_id) else {
            return Poll::Ready(Err(WaitError::MissingCall));
        };
        if let Some(frame) = pending.frame.take() {
            state.calls.remove(&self.call_id);
            return Poll::Ready(Ok(frame));
        }
        if pending
            .waker
            .as_ref()
            .is_none_or(|registered| !registered.will_wake(context.waker()))
        {
            pending.waker = Some(context.waker().clone());
        }
        Poll::Pending
    }
}

struct RegistrationGuard<C, T> {
    shared: Arc<Shared<C, T>>,
    call_id: CallId,
    sent: bool,
    active: bool,
}

impl<C, T> RegistrationGuard<C, T> {
    fn new(shared: Arc<Shared<C, T>>, call_id: CallId) -> Self {
        Self {
            shared,
            call_id,
            sent: false,
            active: true,
        }
    }

    fn mark_sent(&mut self) {
        self.sent = true;
    }

    fn disarm(&mut self) {
        self.active = false;
    }
}

impl<C, T> Drop for RegistrationGuard<C, T> {
    fn drop(&mut self) {
        if self.active {
            self.shared.abandon(self.call_id, self.sent);
        }
    }
}

pub struct EncodedClientChannel<C, T> {
    shared: Arc<Shared<C, T>>,
}

impl<C, T> EncodedClientChannel<C, T> {
    pub fn new(
        protocol: WireProtocol<C>,
        transport: T,
        pending_call_limit: usize,
    ) -> (Self, EncodedClientDriver<C, T>) {
        Self::assemble(protocol, None, transport, pending_call_limit, None)
    }

    pub fn new_with_resource_ledger(
        protocol: WireProtocol<C>,
        transport: T,
        pending_call_limit: usize,
        resource_ledger: ResourceLedger,
    ) -> (Self, EncodedClientDriver<C, T>) {
        Self::assemble(
            protocol,
            None,
            transport,
            pending_call_limit,
            Some(resource_ledger),
        )
    }

    /// Production-only channel construction. The transport must expose a
    /// bounded production profile and must already use this exact ledger.
    pub fn new_with_production_resources(
        protocol: WireProtocol<C>,
        transport: T,
        pending_call_limit: usize,
        resources: &ProductionResourceLedger,
    ) -> Result<(Self, EncodedClientDriver<C, T>), ClientWireProductionError>
    where
        T: ProductionFrameTransport,
    {
        let transport_resources = transport.production_resources();
        let profile = ClientWireProductionProfile::new(
            &protocol,
            pending_call_limit,
            transport_resources.requirements(),
        );
        let ledger = resources.claim(profile.requirements())?;
        if !ledger.shares_budget_with(transport_resources.ledger()) {
            return Err(ClientWireProductionError::TransportLedgerMismatch);
        }
        Ok(Self::assemble(
            protocol.with_resource_ledger(ledger.clone()),
            None,
            transport,
            pending_call_limit,
            Some(ledger),
        ))
    }

    fn assemble(
        protocol: WireProtocol<C>,
        control: Option<ControlPlane>,
        transport: T,
        pending_call_limit: usize,
        resource_ledger: Option<ResourceLedger>,
    ) -> (Self, EncodedClientDriver<C, T>) {
        let shared = Arc::new(Shared {
            protocol,
            control,
            transport,
            send_gate: Arc::new(SendGate::new(pending_call_limit)),
            state: Mutex::new(ChannelState::new(pending_call_limit, resource_ledger)),
        });
        (
            Self {
                shared: Arc::clone(&shared),
            },
            EncodedClientDriver {
                shared,
                close: ClientCloseMachine::new(),
                close_woken_calls: 0,
                close_ignored_frames: 0,
            },
        )
    }

    pub fn new_with_control<Control>(
        protocol: WireProtocol<C>,
        control_protocol: WireProtocol<Control>,
        transport: T,
        pending_call_limit: usize,
    ) -> Result<(Self, EncodedClientDriver<C, T>), ClientWireConfigurationError>
    where
        Control: WireCodec<ControlMessage> + 'static,
        <Control as WireCodec<ControlMessage>>::Error: Display,
    {
        Self::new_with_optional_control_resource_ledger(
            protocol,
            control_protocol,
            transport,
            pending_call_limit,
            None,
        )
    }

    pub fn new_with_control_and_resource_ledger<Control>(
        protocol: WireProtocol<C>,
        control_protocol: WireProtocol<Control>,
        transport: T,
        pending_call_limit: usize,
        resource_ledger: ResourceLedger,
    ) -> Result<(Self, EncodedClientDriver<C, T>), ClientWireConfigurationError>
    where
        Control: WireCodec<ControlMessage> + 'static,
        <Control as WireCodec<ControlMessage>>::Error: Display,
    {
        Self::new_with_optional_control_resource_ledger(
            protocol,
            control_protocol,
            transport,
            pending_call_limit,
            Some(resource_ledger),
        )
    }

    pub fn new_with_control_and_production_resources<Control>(
        protocol: WireProtocol<C>,
        control_protocol: WireProtocol<Control>,
        transport: T,
        pending_call_limit: usize,
        resources: &ProductionResourceLedger,
    ) -> Result<(Self, EncodedClientDriver<C, T>), ClientWireProductionError>
    where
        T: ProductionFrameTransport,
        Control: WireCodec<ControlMessage> + 'static,
        <Control as WireCodec<ControlMessage>>::Error: Display,
    {
        if protocol.send_version() != control_protocol.send_version() {
            return Err(ClientWireConfigurationError::ControlVersionMismatch.into());
        }
        if protocol.versions() != control_protocol.versions() {
            return Err(ClientWireConfigurationError::ControlVersionPolicyMismatch.into());
        }
        if protocol.send_limits() != control_protocol.send_limits()
            || protocol.receive_limits() != control_protocol.receive_limits()
        {
            return Err(ClientWireConfigurationError::ControlFrameLimitsMismatch.into());
        }
        let transport_resources = transport.production_resources();
        let profile = ClientWireProductionProfile::new(
            &protocol,
            pending_call_limit,
            transport_resources.requirements(),
        );
        let ledger = resources.claim(profile.requirements())?;
        if !ledger.shares_budget_with(transport_resources.ledger()) {
            return Err(ClientWireProductionError::TransportLedgerMismatch);
        }
        let encode_ledger = ledger.clone();
        let control = ControlPlane::new(control_protocol.with_resource_ledger(encode_ledger));
        Ok(Self::assemble(
            protocol.with_resource_ledger(ledger.clone()),
            Some(control),
            transport,
            pending_call_limit,
            Some(ledger),
        ))
    }

    fn new_with_optional_control_resource_ledger<Control>(
        protocol: WireProtocol<C>,
        control_protocol: WireProtocol<Control>,
        transport: T,
        pending_call_limit: usize,
        resource_ledger: Option<ResourceLedger>,
    ) -> Result<(Self, EncodedClientDriver<C, T>), ClientWireConfigurationError>
    where
        Control: WireCodec<ControlMessage> + 'static,
        <Control as WireCodec<ControlMessage>>::Error: Display,
    {
        if protocol.send_version() != control_protocol.send_version() {
            return Err(ClientWireConfigurationError::ControlVersionMismatch);
        }
        if protocol.versions() != control_protocol.versions() {
            return Err(ClientWireConfigurationError::ControlVersionPolicyMismatch);
        }
        if protocol.send_limits() != control_protocol.send_limits()
            || protocol.receive_limits() != control_protocol.receive_limits()
        {
            return Err(ClientWireConfigurationError::ControlFrameLimitsMismatch);
        }
        let control = ControlPlane::new(control_protocol);
        Ok(Self::assemble(
            protocol,
            Some(control),
            transport,
            pending_call_limit,
            resource_ledger,
        ))
    }

    pub fn close(&self) -> Result<usize, ClientWireStateError> {
        self.shared.close(ChannelCloseReason::Explicit)
    }

    pub fn metrics(&self) -> Result<ClientWireMetrics, ClientWireStateError> {
        self.shared.metrics()
    }
}

impl<C, T> Clone for EncodedClientChannel<C, T> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

fn map_cancellation_send_gate_error<Transport>(
    error: SendGateError,
) -> CancellationFlushError<Transport> {
    match error {
        SendGateError::CapacityExceeded { limit } => {
            CancellationFlushError::SendCapacityExceeded { limit }
        }
        SendGateError::StateUnavailable => CancellationFlushError::StateUnavailable,
        SendGateError::TicketExhausted => CancellationFlushError::SendTicketExhausted,
    }
}

fn command_send_gate_failure(error: SendGateError) -> (FailureKind, String) {
    match error {
        SendGateError::CapacityExceeded { limit } => (
            FailureKind::ResourceExhausted,
            format!("encoded client send limit of {limit} was reached"),
        ),
        SendGateError::StateUnavailable => (
            FailureKind::ProtocolViolation,
            "encoded client send state is unavailable".to_string(),
        ),
        SendGateError::TicketExhausted => (
            FailureKind::ResourceExhausted,
            "encoded client send ticket space was exhausted".to_string(),
        ),
    }
}

fn pending_resource_failure(error: ResourceError) -> (FailureKind, String) {
    let kind = match error {
        ResourceError::GlobalLimitExceeded { .. }
        | ResourceError::ScopeLimitExceeded { .. }
        | ResourceError::ScopeCapacityExceeded { .. }
        | ResourceError::LeaseCapacityExceeded { .. } => FailureKind::ResourceExhausted,
        ResourceError::StatePoisoned => FailureKind::Cleanup,
        ResourceError::EmptyRequest
        | ResourceError::ZeroUnits { .. }
        | ResourceError::DuplicateClass { .. }
        | ResourceError::TooManyClasses { .. }
        | ResourceError::ArithmeticOverflow { .. }
        | ResourceError::MissingLeaseClass { .. }
        | ResourceError::ReleasedLease
        | ResourceError::ClassIsNotBytes { .. }
        | ResourceError::ReservationTooSmall { .. } => FailureKind::ProtocolViolation,
    };
    (kind, error.to_string())
}

pub struct EncodedClientDriver<C, T> {
    shared: Arc<Shared<C, T>>,
    close: ClientCloseMachine,
    close_woken_calls: usize,
    close_ignored_frames: usize,
}

impl<C, T> EncodedClientDriver<C, T>
where
    T: FrameTransport,
{
    pub const fn graceful_close_machine(&self) -> ClientCloseMachine {
        self.close
    }

    pub async fn receive_once(&mut self) -> Result<DriverEvent, DriverError<T::Error>> {
        if let Some(reason) = self
            .shared
            .close_reason()
            .map_err(|_| DriverError::StateUnavailable)?
        {
            return Ok(DriverEvent::Closed(reason));
        }
        self.finish_receive(self.shared.transport.receive().await)
    }

    fn finish_receive(
        &self,
        received: Result<Option<WireFrame>, T::Error>,
    ) -> Result<DriverEvent, DriverError<T::Error>> {
        let frame = match received {
            Ok(Some(frame)) => frame,
            Ok(None) => {
                self.shared
                    .close(ChannelCloseReason::EndOfStream)
                    .map_err(|_| DriverError::StateUnavailable)?;
                return Ok(DriverEvent::Closed(ChannelCloseReason::EndOfStream));
            }
            Err(error) => {
                let _ = self.shared.close(ChannelCloseReason::ReceiveFailed);
                return Err(DriverError::Transport(error));
            }
        };
        self.shared.deliver(frame).map_err(|error| match error {
            DriverError::Correlation(error) => DriverError::Correlation(error),
            DriverError::StateUnavailable => DriverError::StateUnavailable,
            DriverError::Transport(never) => match never {},
        })
    }

    /// Sends at most one cancellation created by dropping a sent command
    /// future. The logical message remains queued if encoding fails. A
    /// transport failure closes the channel and clears all pending state.
    pub async fn flush_cancellation_once(
        &mut self,
    ) -> Result<CancellationFlushOutcome, CancellationFlushError<T::Error>> {
        let Some(control) = self.shared.control.as_ref() else {
            return Err(CancellationFlushError::MissingControlProtocol);
        };
        if self
            .shared
            .next_cancellation()
            .map_err(|_| CancellationFlushError::StateUnavailable)?
            .is_none()
        {
            return Ok(CancellationFlushOutcome::Idle);
        }
        let _permit = self
            .shared
            .send_gate
            .acquire(SendPriority::Control)
            .map_err(map_cancellation_send_gate_error)?
            .await
            .map_err(map_cancellation_send_gate_error)?;
        let Some((call_id, cancellation)) = self
            .shared
            .next_cancellation()
            .map_err(|_| CancellationFlushError::StateUnavailable)?
        else {
            return Ok(CancellationFlushOutcome::Idle);
        };
        let message = ControlMessage::Cancel {
            call_id,
            cancellation,
            reason: CancellationReason::Caller,
        };
        let frame = (control.encode)(&message).map_err(CancellationFlushError::Protocol)?;
        if frame.route() != FrameRoute::Control {
            return Err(CancellationFlushError::InvalidRoute(frame.route()));
        }
        if let Err(error) = self.shared.transport.send(frame).await {
            let _ = self.shared.close(ChannelCloseReason::SendFailed);
            return Err(CancellationFlushError::Transport(error));
        }
        self.shared
            .complete_cancellation(call_id)
            .map_err(|_| CancellationFlushError::StateUnavailable)?;
        Ok(CancellationFlushOutcome::Sent {
            call_id,
            cancellation,
        })
    }

    /// Drives inbound frames while also flushing drop-triggered cancellations.
    ///
    /// The receive future remains pinned while a cancellation is written, so
    /// a partially read frame prefix is never discarded. A connection
    /// supervisor that enables control must call this instead of
    /// `receive_once`.
    pub async fn drive_once(
        &mut self,
    ) -> Result<DriverEvent, CancellationAwareDriverError<T::Error>> {
        match self
            .flush_cancellation_once()
            .await
            .map_err(CancellationAwareDriverError::Cancellation)?
        {
            CancellationFlushOutcome::Idle | CancellationFlushOutcome::Sent { .. } => {}
        }

        enum Wake<Receive> {
            Received(Receive),
            Cancellation,
            StateUnavailable,
        }

        let shared = Arc::clone(&self.shared);
        let mut receive = std::pin::pin!(shared.transport.receive());
        loop {
            let wake = poll_fn(|context| {
                if let Poll::Ready(received) = receive.as_mut().poll(context) {
                    return Poll::Ready(Wake::Received(received));
                }
                match shared.poll_cancellation_ready(context) {
                    Ok(true) => Poll::Ready(Wake::Cancellation),
                    Ok(false) => Poll::Pending,
                    Err(_) => Poll::Ready(Wake::StateUnavailable),
                }
            })
            .await;
            match wake {
                Wake::Received(received) => {
                    return self
                        .finish_receive(received)
                        .map_err(CancellationAwareDriverError::Driver);
                }
                Wake::Cancellation => {
                    self.flush_cancellation_once()
                        .await
                        .map_err(CancellationAwareDriverError::Cancellation)?;
                }
                Wake::StateUnavailable => {
                    return Err(CancellationAwareDriverError::Cancellation(
                        CancellationFlushError::StateUnavailable,
                    ));
                }
            }
        }
    }

    /// Cancellation-aware counterpart to `receive_once_with`. Optional
    /// lifecycle and event handlers remain composable without giving up the
    /// drop-to-cancel guarantee.
    pub async fn drive_once_with<Handler>(
        &mut self,
        handler: &mut Handler,
    ) -> Result<
        RoutedDriverEvent<Handler::Event>,
        HandlerChainError<CancellationAwareDriverError<T::Error>, Handler::Error>,
    >
    where
        Handler: FrameHandler,
    {
        match self.drive_once().await.map_err(HandlerChainError::Left)? {
            DriverEvent::Delivered(call) => Ok(RoutedDriverEvent::Delivered(call)),
            DriverEvent::IgnoredAbandoned(call_id) => {
                Ok(RoutedDriverEvent::IgnoredAbandoned(call_id))
            }
            DriverEvent::Closed(reason) => Ok(RoutedDriverEvent::Closed(reason)),
            DriverEvent::Unhandled(frame) => {
                match handler.handle(frame).map_err(HandlerChainError::Right)? {
                    FrameDisposition::Handled(event) => Ok(RoutedDriverEvent::Handled(event)),
                    FrameDisposition::Unhandled(frame) => Ok(RoutedDriverEvent::Unhandled(frame)),
                }
            }
        }
    }

    pub async fn receive_once_with<Handler>(
        &mut self,
        handler: &mut Handler,
    ) -> Result<RoutedDriverEvent<Handler::Event>, RoutedDriverError<T::Error, Handler::Error>>
    where
        Handler: FrameHandler,
    {
        match self
            .receive_once()
            .await
            .map_err(RoutedDriverError::Driver)?
        {
            DriverEvent::Delivered(call) => Ok(RoutedDriverEvent::Delivered(call)),
            DriverEvent::IgnoredAbandoned(call_id) => {
                Ok(RoutedDriverEvent::IgnoredAbandoned(call_id))
            }
            DriverEvent::Closed(reason) => Ok(RoutedDriverEvent::Closed(reason)),
            DriverEvent::Unhandled(frame) => {
                match handler.handle(frame).map_err(RoutedDriverError::Handler)? {
                    FrameDisposition::Handled(event) => Ok(RoutedDriverEvent::Handled(event)),
                    FrameDisposition::Unhandled(frame) => Ok(RoutedDriverEvent::Unhandled(frame)),
                }
            }
        }
    }

    /// Performs the negotiated `Close` / `CloseAck` barrier and then shuts
    /// down the transport write half.
    ///
    /// Starting the exchange closes logical call state immediately, so no new
    /// command can overtake `Close`. Frames already preceding the server's
    /// acknowledgement are validated and discarded one at a time. The close
    /// machine lives on this unique driver, making cancellation while awaiting
    /// the acknowledgement or retrying write shutdown resumable without a
    /// second close request.
    pub async fn graceful_close(
        &mut self,
    ) -> Result<ClientGracefulCloseOutcome, ClientGracefulCloseError<T::Error>>
    where
        T: ShutdownFrameTransport,
    {
        let Some(control) = self.shared.control.as_ref() else {
            return Err(ClientGracefulCloseError::MissingControlProtocol);
        };
        loop {
            match self.close.next_action() {
                ClientCloseAction::SendClose => {
                    let frame = (control.encode)(&ControlMessage::Close)
                        .map_err(ClientGracefulCloseError::Protocol)?;
                    if frame.route() != FrameRoute::Control {
                        return Err(ClientGracefulCloseError::EncodedRouteMismatch {
                            actual: frame.route(),
                        });
                    }
                    let woken = self
                        .shared
                        .close(ChannelCloseReason::Explicit)
                        .map_err(ClientGracefulCloseError::State)?;
                    self.close_woken_calls = self.close_woken_calls.saturating_add(woken);
                    let _permit = self
                        .shared
                        .send_gate
                        .acquire(SendPriority::Control)
                        .map_err(map_client_graceful_close_gate_error)?
                        .await
                        .map_err(map_client_graceful_close_gate_error)?;
                    // `FrameTransport::send` owns any partial frame after its
                    // first pending poll. Mark the close as sent immediately
                    // before awaiting it so cancellation resumes at Ack wait
                    // instead of submitting a duplicate frame.
                    self.close
                        .complete(ClientCloseAction::SendClose)
                        .map_err(ClientGracefulCloseError::StateMachine)?;
                    if let Err(error) = self.shared.transport.send(frame).await {
                        self.close.terminate();
                        return Err(ClientGracefulCloseError::Send(error));
                    }
                }
                ClientCloseAction::AwaitCloseAck => {
                    let frame = match self.shared.transport.receive().await {
                        Ok(Some(frame)) => frame,
                        Ok(None) => {
                            self.close.terminate();
                            return Err(ClientGracefulCloseError::EndOfStreamBeforeAck);
                        }
                        Err(error) => {
                            self.close.terminate();
                            return Err(ClientGracefulCloseError::Receive(error));
                        }
                    };
                    if frame.route() == FrameRoute::Control {
                        let message =
                            (control.decode)(frame).map_err(ClientGracefulCloseError::Protocol)?;
                        if message == ControlMessage::CloseAck {
                            self.close
                                .receive(message)
                                .map_err(ClientGracefulCloseError::StateMachine)?;
                        } else {
                            self.close_ignored_frames = self.close_ignored_frames.saturating_add(1);
                        }
                    } else {
                        let expected = self.shared.protocol.send_version();
                        if frame.version() != expected {
                            return Err(ClientGracefulCloseError::ApplicationVersionMismatch {
                                expected,
                                actual: frame.version(),
                            });
                        }
                        frame
                            .validate(self.shared.protocol.receive_limits())
                            .map_err(ClientGracefulCloseError::IncomingFrame)?;
                        self.close_ignored_frames = self.close_ignored_frames.saturating_add(1);
                    }
                }
                ClientCloseAction::ShutdownWrite => {
                    let _permit = self
                        .shared
                        .send_gate
                        .acquire(SendPriority::Control)
                        .map_err(map_client_graceful_close_gate_error)?
                        .await
                        .map_err(map_client_graceful_close_gate_error)?;
                    self.shared
                        .transport
                        .shutdown()
                        .await
                        .map_err(ClientGracefulCloseError::Shutdown)?;
                    self.close
                        .complete(ClientCloseAction::ShutdownWrite)
                        .map_err(ClientGracefulCloseError::StateMachine)?;
                }
                ClientCloseAction::Complete => {
                    return Ok(ClientGracefulCloseOutcome {
                        woken_calls: self.close_woken_calls,
                        ignored_frames: self.close_ignored_frames,
                    });
                }
                ClientCloseAction::Stop => return Err(ClientGracefulCloseError::Terminated),
            }
        }
    }

    /// Closes the logical channel, waits behind any active frame write, and
    /// then cleanly shuts down the transport write half.
    ///
    /// This operation lives on the unique receive driver so cloned command
    /// handles cannot start an unbounded set of competing shutdown attempts.
    /// Cancelling it leaves the channel logically closed; calling it again
    /// retries the transport shutdown.
    pub async fn close_transport(&mut self) -> Result<usize, ClientTransportCloseError<T::Error>>
    where
        T: ShutdownFrameTransport,
    {
        let woken = self
            .shared
            .close(ChannelCloseReason::Explicit)
            .map_err(ClientTransportCloseError::State)?;
        let _permit = self
            .shared
            .send_gate
            .acquire(SendPriority::Control)
            .map_err(map_client_transport_close_gate_error)?
            .await
            .map_err(map_client_transport_close_gate_error)?;
        self.shared
            .transport
            .shutdown()
            .await
            .map_err(ClientTransportCloseError::Transport)?;
        Ok(woken)
    }
}

impl<C, T> Drop for EncodedClientDriver<C, T> {
    fn drop(&mut self) {
        let _ = self.shared.close(ChannelCloseReason::DriverDropped);
    }
}

fn local_failure<Spec>(
    call_id: CallId,
    kind: FailureKind,
    message: impl Into<String>,
) -> CommandResult<Spec>
where
    Spec: CommandSpec,
{
    CommandResult::new(call_id, Err(OperationError::new(kind, message, None)))
}

impl<C, T, Spec> Service<Command<Spec>> for EncodedClientChannel<C, T>
where
    C: WireCodec<Command<Spec>> + WireCodec<CommandResult<Spec>> + 'static,
    <C as WireCodec<Command<Spec>>>::Error: Display,
    <C as WireCodec<CommandResult<Spec>>>::Error: Display,
    T: FrameTransport + 'static,
    T::Error: Display,
    Spec: CommandSpec,
    Spec::Params: Send + 'static,
{
    type Response = CommandResult<Spec>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<Spec>) -> Self::Future<'_> {
        let shared = Arc::clone(&self.shared);
        Box::pin(async move {
            let (call_id, object_id, mut metadata, params) = command.into_parts();
            if shared.control.is_some() && metadata.cancellation.is_none() {
                metadata.cancellation = Some(CancellationId::new(call_id.get()));
            }
            let command = Command::<Spec>::new(call_id, object_id, metadata, params);
            let expected_route = FrameRoute::Command {
                call_id,
                target: command.object_id(),
                method: command.method(),
            };
            let frame = match shared.protocol.encode(&command) {
                Ok(frame) => frame,
                Err(error) => {
                    return Ok(local_failure::<Spec>(
                        call_id,
                        FailureKind::ProtocolViolation,
                        format!("command encoding failed: {error}"),
                    ));
                }
            };
            if frame.route() != expected_route {
                return Ok(local_failure::<Spec>(
                    call_id,
                    FailureKind::ProtocolViolation,
                    format!(
                        "encoded command route {:?} does not match expected {expected_route:?}",
                        frame.route()
                    ),
                ));
            }
            if let Err(error) = shared.register(
                call_id,
                command.object_id(),
                command.method(),
                command.metadata().cancellation,
            ) {
                let (kind, message) = match error {
                    RegistrationError::Closed(reason) => (
                        FailureKind::TransportClosed,
                        format!("encoded client channel is closed: {reason:?}"),
                    ),
                    RegistrationError::CapacityExceeded { limit } => (
                        FailureKind::ResourceExhausted,
                        format!("encoded client pending-call limit of {limit} was reached"),
                    ),
                    RegistrationError::Correlation(error) => {
                        (FailureKind::ProtocolViolation, error.to_string())
                    }
                    RegistrationError::Resource(error) => pending_resource_failure(error),
                    RegistrationError::StateUnavailable => (
                        FailureKind::ProtocolViolation,
                        "encoded client channel state is unavailable".to_string(),
                    ),
                };
                return Ok(local_failure::<Spec>(call_id, kind, message));
            }
            let mut guard = RegistrationGuard::new(Arc::clone(&shared), call_id);
            let permit = match shared.send_gate.acquire(SendPriority::Command) {
                Ok(permit) => match permit.await {
                    Ok(permit) => permit,
                    Err(error) => {
                        let (kind, message) = command_send_gate_failure(error);
                        return Ok(local_failure::<Spec>(call_id, kind, message));
                    }
                },
                Err(error) => {
                    let (kind, message) = command_send_gate_failure(error);
                    return Ok(local_failure::<Spec>(call_id, kind, message));
                }
            };
            match shared.close_reason() {
                Ok(Some(reason)) => {
                    return Ok(local_failure::<Spec>(
                        call_id,
                        FailureKind::TransportClosed,
                        format!("encoded client channel is closed: {reason:?}"),
                    ));
                }
                Ok(None) => {}
                Err(_) => {
                    return Ok(local_failure::<Spec>(
                        call_id,
                        FailureKind::ProtocolViolation,
                        "encoded client channel state is unavailable",
                    ));
                }
            }
            // Once the unique send permit is held, this channel has no other
            // writer that could keep the transport's write state busy. The
            // first poll of `send` therefore either completes or transfers a
            // partial frame into cancellation-safe transport-owned state.
            guard.mark_sent();
            if let Err(error) = shared.transport.send(frame).await {
                let _ = shared.close(ChannelCloseReason::SendFailed);
                return Ok(local_failure::<Spec>(
                    call_id,
                    FailureKind::TransportClosed,
                    format!("frame send failed: {error}"),
                ));
            }
            drop(permit);
            let response_frame = match (RawResponse {
                shared: Arc::clone(&shared),
                call_id,
            })
            .await
            {
                Ok(frame) => frame,
                Err(WaitError::Closed(reason)) => {
                    return Ok(local_failure::<Spec>(
                        call_id,
                        FailureKind::TransportClosed,
                        format!("encoded client channel is closed: {reason:?}"),
                    ));
                }
                Err(WaitError::MissingCall) => {
                    return Ok(local_failure::<Spec>(
                        call_id,
                        FailureKind::ProtocolViolation,
                        "encoded client call disappeared before completion",
                    ));
                }
                Err(WaitError::StateUnavailable) => {
                    return Ok(local_failure::<Spec>(
                        call_id,
                        FailureKind::ProtocolViolation,
                        "encoded client channel state is unavailable",
                    ));
                }
            };
            guard.disarm();
            let result = match shared
                .protocol
                .decode::<CommandResult<Spec>>(response_frame)
            {
                Ok(result) => result,
                Err(error) => {
                    let _ = shared.close(ChannelCloseReason::ProtocolViolation);
                    return Ok(local_failure::<Spec>(
                        call_id,
                        FailureKind::ProtocolViolation,
                        format!("result decoding failed: {error}"),
                    ));
                }
            };
            if result.call_id() != call_id {
                let _ = shared.close(ChannelCloseReason::ProtocolViolation);
                return Ok(local_failure::<Spec>(
                    call_id,
                    FailureKind::ProtocolViolation,
                    format!(
                        "decoded response call ID `{}` does not match request `{call_id}`",
                        result.call_id()
                    ),
                ));
            }
            Ok(result)
        })
    }
}

fn map_client_transport_close_gate_error<Transport>(
    error: SendGateError,
) -> ClientTransportCloseError<Transport> {
    match error {
        SendGateError::CapacityExceeded { limit } => {
            ClientTransportCloseError::SendCapacityExceeded { limit }
        }
        SendGateError::StateUnavailable => {
            ClientTransportCloseError::State(ClientWireStateError::StateUnavailable)
        }
        SendGateError::TicketExhausted => ClientTransportCloseError::SendTicketExhausted,
    }
}

fn map_client_graceful_close_gate_error<Transport>(
    error: SendGateError,
) -> ClientGracefulCloseError<Transport> {
    match error {
        SendGateError::CapacityExceeded { limit } => {
            ClientGracefulCloseError::SendCapacityExceeded { limit }
        }
        SendGateError::StateUnavailable => {
            ClientGracefulCloseError::State(ClientWireStateError::StateUnavailable)
        }
        SendGateError::TicketExhausted => ClientGracefulCloseError::SendTicketExhausted,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::future::{Future, Pending, Ready, pending, ready};
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    use pwright_codec_handshake::HandshakeCodec;
    use pwright_core::{
        CallId, CallMetadata, CancellationId, CancellationReason, Command, CommandResult,
        CommandSpec, ControlMessage, EventTag, FailureKind, MethodTag, ObjectId, OperationError,
        ProductionResourceLedger, ProtocolVersion, ResourceClass, ResourceLedger, ResourceLimits,
        ResourceRequirements, ResourceScope, Service,
    };
    use pwright_transport::{
        CapabilityRequest, ClientHello, ClientHelloValidationError, EncodedParts, FrameLimits,
        FrameRoute, FrameTransport, NegotiationLimits, ProductionFrameTransport,
        ProductionTransportResources, ServerHello, ShutdownFrameTransport, VersionPolicy,
        WireCodec, WireFrame, WireProtocol,
    };

    use super::{
        CancellationFlushError, CancellationFlushOutcome, ChannelCloseReason,
        ClientHandshakeConfigurationError, ClientHandshakeError, ClientWireConfigurationError,
        ClientWireProductionError, ClientWireProductionProfile, DriverError, DriverEvent, Either,
        EncodedClientChannel, EncodedClientDriver, EncodedClientHandshake, FrameDisposition,
        FrameHandler, FrameHandlerExt, RoutedDriverEvent,
    };

    use pwright_client::{AtomicCallIdSource, Client, RemoteInitializer, RemoteKind};

    struct Echo;

    impl CommandSpec for Echo {
        const METHOD: MethodTag = MethodTag::new(7);
        type Params = String;
        type Result = String;
        type ErrorDetails = String;
    }

    enum EchoObject {}

    impl RemoteKind for EchoObject {
        const OBJECT_KIND: pwright_core::ObjectKindTag = pwright_core::ObjectKindTag::new(77);
        const NAME: &'static str = "EchoObject";
    }

    struct RouteHandler {
        route: FrameRoute,
        event: &'static str,
    }

    impl FrameHandler for RouteHandler {
        type Event = &'static str;
        type Error = std::convert::Infallible;

        fn handle(
            &mut self,
            frame: WireFrame,
        ) -> Result<FrameDisposition<Self::Event>, Self::Error> {
            if frame.route() == self.route {
                Ok(FrameDisposition::Handled(self.event))
            } else {
                Ok(FrameDisposition::Unhandled(frame))
            }
        }
    }

    #[derive(Clone, Copy)]
    struct TestCodec;

    impl WireCodec<Command<Echo>> for TestCodec {
        type Error = &'static str;

        fn encode_parts(&self, message: &Command<Echo>) -> Result<EncodedParts, Self::Error> {
            Ok(EncodedParts::new(
                FrameRoute::Command {
                    call_id: message.call_id(),
                    target: message.object_id(),
                    method: message.method(),
                },
                message.params().as_bytes().to_vec(),
                Vec::new(),
            ))
        }

        fn decode_frame(&self, _frame: WireFrame) -> Result<Command<Echo>, Self::Error> {
            Err("command decoding is server-side")
        }
    }

    impl WireCodec<CommandResult<Echo>> for TestCodec {
        type Error = &'static str;

        fn encode_parts(&self, message: &CommandResult<Echo>) -> Result<EncodedParts, Self::Error> {
            let value = message
                .outcome()
                .as_ref()
                .map_err(|_| "test codec only encodes successful results")?;
            Ok(EncodedParts::new(
                FrameRoute::Result {
                    call_id: message.call_id(),
                },
                value.as_bytes().to_vec(),
                Vec::new(),
            ))
        }

        fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<Echo>, Self::Error> {
            let FrameRoute::Result { call_id } = frame.route() else {
                return Err("expected result route");
            };
            let value = String::from_utf8(frame.payload().to_vec()).map_err(|_| "invalid utf-8")?;
            Ok(CommandResult::new(call_id, Ok(value)))
        }
    }

    impl WireCodec<ControlMessage> for TestCodec {
        type Error = &'static str;

        fn encode_parts(&self, message: &ControlMessage) -> Result<EncodedParts, Self::Error> {
            let ControlMessage::Cancel {
                call_id,
                cancellation,
                reason,
            } = *message
            else {
                return Err("test codec only supports cancellation");
            };
            let mut payload = Vec::with_capacity(17);
            payload.extend_from_slice(&call_id.get().to_be_bytes());
            payload.extend_from_slice(&cancellation.get().to_be_bytes());
            payload.push(match reason {
                CancellationReason::Caller => 0,
                CancellationReason::DeadlineExceeded => 1,
                CancellationReason::ParentClosed => 2,
                CancellationReason::TransportClosed => 3,
            });
            Ok(EncodedParts::new(FrameRoute::Control, payload, Vec::new()))
        }

        fn decode_frame(&self, frame: WireFrame) -> Result<ControlMessage, Self::Error> {
            if frame.route() != FrameRoute::Control || frame.payload().len() != 17 {
                return Err("invalid cancellation frame");
            }
            let call_id = u64::from_be_bytes(
                frame.payload()[0..8]
                    .try_into()
                    .map_err(|_| "invalid call ID")?,
            );
            let cancellation = u64::from_be_bytes(
                frame.payload()[8..16]
                    .try_into()
                    .map_err(|_| "invalid cancellation ID")?,
            );
            let reason = match frame.payload()[16] {
                0 => CancellationReason::Caller,
                1 => CancellationReason::DeadlineExceeded,
                2 => CancellationReason::ParentClosed,
                3 => CancellationReason::TransportClosed,
                _ => return Err("invalid cancellation reason"),
            };
            Ok(ControlMessage::Cancel {
                call_id: CallId::new(call_id),
                cancellation: CancellationId::new(cancellation),
                reason,
            })
        }
    }

    #[derive(Clone, Copy)]
    struct FailingControlCodec;

    impl WireCodec<ControlMessage> for FailingControlCodec {
        type Error = &'static str;

        fn encode_parts(&self, _message: &ControlMessage) -> Result<EncodedParts, Self::Error> {
            Err("control encoding failed")
        }

        fn decode_frame(&self, _frame: WireFrame) -> Result<ControlMessage, Self::Error> {
            Err("control decoding is unused")
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestTransportError(&'static str);

    impl std::fmt::Display for TestTransportError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl std::error::Error for TestTransportError {}

    struct MemoryState {
        sent: VecDeque<WireFrame>,
        incoming: VecDeque<Result<Option<WireFrame>, TestTransportError>>,
        fail_send: bool,
    }

    struct BlockingSendState {
        send_polls: usize,
        shutdowns: usize,
        sent: VecDeque<WireFrame>,
    }

    struct BlockingSend {
        state: Arc<Mutex<BlockingSendState>>,
        frame: Option<WireFrame>,
    }

    impl Future for BlockingSend {
        type Output = Result<(), TestTransportError>;

        fn poll(mut self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
            let state = Arc::clone(&self.state);
            let mut state = state.lock().expect("blocking transport should be open");
            state.send_polls += 1;
            if matches!(
                self.frame.as_ref().map(WireFrame::route),
                Some(FrameRoute::Command { call_id, .. }) if call_id == CallId::new(1)
            ) {
                return Poll::Pending;
            }
            state
                .sent
                .push_back(self.frame.take().expect("frame should be sent once"));
            Poll::Ready(Ok(()))
        }
    }

    #[derive(Clone)]
    struct BlockingSendTransport {
        state: Arc<Mutex<BlockingSendState>>,
    }

    impl FrameTransport for BlockingSendTransport {
        type Error = TestTransportError;
        type Send<'a> = BlockingSend;
        type Receive<'a> = Ready<Result<Option<WireFrame>, Self::Error>>;

        fn send(&self, frame: WireFrame) -> Self::Send<'_> {
            BlockingSend {
                state: Arc::clone(&self.state),
                frame: Some(frame),
            }
        }

        fn receive(&self) -> Self::Receive<'_> {
            ready(Ok(None))
        }
    }

    impl ShutdownFrameTransport for BlockingSendTransport {
        type Shutdown<'a> = Ready<Result<(), Self::Error>>;

        fn shutdown(&self) -> Self::Shutdown<'_> {
            self.state
                .lock()
                .expect("blocking transport should be open")
                .shutdowns += 1;
            ready(Ok(()))
        }
    }

    #[derive(Clone)]
    struct MemoryTransport {
        state: Arc<Mutex<MemoryState>>,
    }

    impl FrameTransport for MemoryTransport {
        type Error = TestTransportError;
        type Send<'a> = Ready<Result<(), Self::Error>>;
        type Receive<'a> = Ready<Result<Option<WireFrame>, Self::Error>>;

        fn send(&self, frame: WireFrame) -> Self::Send<'_> {
            let mut state = self.state.lock().expect("memory transport should be open");
            if state.fail_send {
                return ready(Err(TestTransportError("send failed")));
            }
            state.sent.push_back(frame);
            ready(Ok(()))
        }

        fn receive(&self) -> Self::Receive<'_> {
            ready(
                self.state
                    .lock()
                    .expect("memory transport should be open")
                    .incoming
                    .pop_front()
                    .unwrap_or(Ok(None)),
            )
        }
    }

    struct ProductionMemoryTransport {
        inner: MemoryTransport,
        ledger: ResourceLedger,
        requirements: ResourceRequirements,
    }

    impl FrameTransport for ProductionMemoryTransport {
        type Error = TestTransportError;
        type Send<'a> = Ready<Result<(), Self::Error>>;
        type Receive<'a> = Ready<Result<Option<WireFrame>, Self::Error>>;

        fn send(&self, frame: WireFrame) -> Self::Send<'_> {
            if !frame.is_budgeted_by(&self.ledger) {
                return ready(Err(TestTransportError("frame uses another ledger")));
            }
            self.inner.send(frame)
        }

        fn receive(&self) -> Self::Receive<'_> {
            self.inner.receive()
        }
    }

    impl ProductionFrameTransport for ProductionMemoryTransport {
        fn production_resources(&self) -> ProductionTransportResources<'_> {
            ProductionTransportResources::new(&self.ledger, self.requirements)
        }
    }

    struct DropTrackedTransport {
        dropped: Arc<AtomicBool>,
    }

    impl Drop for DropTrackedTransport {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    impl FrameTransport for DropTrackedTransport {
        type Error = TestTransportError;
        type Send<'a> = Pending<Result<(), Self::Error>>;
        type Receive<'a> = Ready<Result<Option<WireFrame>, Self::Error>>;

        fn send(&self, _frame: WireFrame) -> Self::Send<'_> {
            pending()
        }

        fn receive(&self) -> Self::Receive<'_> {
            ready(Ok(None))
        }
    }

    fn poll_once<F>(future: Pin<&mut F>) -> Poll<F::Output>
    where
        F: Future + ?Sized,
    {
        future.poll(&mut Context::from_waker(Waker::noop()))
    }

    fn limits() -> FrameLimits {
        FrameLimits::new(256, 0, 0, 256)
    }

    fn payload_limits(bytes: usize) -> FrameLimits {
        FrameLimits::new(bytes, 0, 0, bytes)
    }

    fn negotiation_limits() -> NegotiationLimits {
        NegotiationLimits::new(8, 64)
    }

    fn handshake_versions() -> VersionPolicy {
        VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 2))
            .expect("handshake version range should build")
    }

    fn empty_capabilities() -> CapabilityRequest {
        CapabilityRequest::new(Vec::new(), Vec::new(), negotiation_limits())
            .expect("empty capability request should build")
    }

    fn client_hello(receive_limits: FrameLimits) -> ClientHello {
        ClientHello::new(handshake_versions(), receive_limits, empty_capabilities())
    }

    fn server_hello(
        selected: ProtocolVersion,
        client_receive_limits: FrameLimits,
        server_receive_limits: FrameLimits,
    ) -> ServerHello {
        ServerHello::new(
            selected,
            client_receive_limits,
            server_receive_limits,
            Vec::new(),
            negotiation_limits(),
        )
        .expect("server hello should build")
    }

    fn client_handshake_protocol(
        client_receive_limits: FrameLimits,
    ) -> WireProtocol<HandshakeCodec> {
        WireProtocol::new_directional(
            HandshakeCodec::new(negotiation_limits()),
            handshake_versions(),
            ProtocolVersion::new(1, 2),
            limits(),
            client_receive_limits,
        )
        .expect("client handshake protocol should build")
    }

    fn encode_server_hello(hello: &ServerHello) -> WireFrame {
        WireProtocol::new_exact_directional(
            HandshakeCodec::new(negotiation_limits()),
            hello.selected_version(),
            hello.client_receive_limits(),
            limits(),
        )
        .encode(hello)
        .expect("server hello should encode")
    }

    fn protocol() -> WireProtocol<TestCodec> {
        WireProtocol::new(
            TestCodec,
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("version range should build"),
            ProtocolVersion::new(1, 0),
            limits(),
        )
        .expect("protocol should build")
    }

    fn transport_requirements() -> ResourceRequirements {
        ResourceRequirements::none()
            .with_class(ResourceClass::WireBytes, 404, 404)
            .with_metadata(3, 5)
    }

    fn production_root(profile: ClientWireProductionProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let pending = requirements.class(ResourceClass::PendingCall);
        let wire = requirements.class(ResourceClass::WireBytes);
        ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases)
                .with_class(
                    ResourceClass::PendingCall,
                    pending.global,
                    pending.per_scope,
                )
                .with_class(ResourceClass::WireBytes, wire.global, wire.per_scope),
            requirements,
        )
        .expect("exact client stack profile should fit")
    }

    fn production_memory_transport(
        state: Arc<Mutex<MemoryState>>,
        resources: &ProductionResourceLedger,
    ) -> ProductionMemoryTransport {
        let requirements = transport_requirements();
        ProductionMemoryTransport {
            inner: MemoryTransport { state },
            ledger: resources
                .claim(requirements)
                .expect("client stack plan should include transport"),
            requirements,
        }
    }

    fn custom_protocol(
        versions: VersionPolicy,
        send_version: ProtocolVersion,
        limits: FrameLimits,
    ) -> WireProtocol<TestCodec> {
        WireProtocol::new(TestCodec, versions, send_version, limits)
            .expect("custom protocol should build")
    }

    type TestChannel = EncodedClientChannel<TestCodec, MemoryTransport>;
    type TestDriver = EncodedClientDriver<TestCodec, MemoryTransport>;
    type TestSetup = (TestChannel, TestDriver, Arc<Mutex<MemoryState>>);

    fn setup(limit: usize) -> TestSetup {
        let state = Arc::new(Mutex::new(MemoryState {
            sent: VecDeque::new(),
            incoming: VecDeque::new(),
            fail_send: false,
        }));
        let (channel, driver) = EncodedClientChannel::new(
            protocol(),
            MemoryTransport {
                state: Arc::clone(&state),
            },
            limit,
        );
        (channel, driver, state)
    }

    fn setup_with_control(limit: usize) -> TestSetup {
        let state = Arc::new(Mutex::new(MemoryState {
            sent: VecDeque::new(),
            incoming: VecDeque::new(),
            fail_send: false,
        }));
        let (channel, driver) = EncodedClientChannel::new_with_control(
            protocol(),
            protocol(),
            MemoryTransport {
                state: Arc::clone(&state),
            },
            limit,
        )
        .expect("matching command and control protocols should compose");
        (channel, driver, state)
    }

    fn pending_ledger(global: u64) -> ResourceLedger {
        ResourceLedger::new(ResourceLimits::deny_all(8, 8).with_class(
            ResourceClass::PendingCall,
            global,
            1,
        ))
    }

    fn setup_budgeted(limit: usize, global: u64) -> (TestSetup, ResourceLedger) {
        let state = Arc::new(Mutex::new(MemoryState {
            sent: VecDeque::new(),
            incoming: VecDeque::new(),
            fail_send: false,
        }));
        let ledger = pending_ledger(global);
        let (channel, driver) = EncodedClientChannel::new_with_resource_ledger(
            protocol(),
            MemoryTransport {
                state: Arc::clone(&state),
            },
            limit,
            ledger.clone(),
        );
        ((channel, driver, state), ledger)
    }

    fn setup_budgeted_with_control(limit: usize, global: u64) -> (TestSetup, ResourceLedger) {
        let state = Arc::new(Mutex::new(MemoryState {
            sent: VecDeque::new(),
            incoming: VecDeque::new(),
            fail_send: false,
        }));
        let ledger = pending_ledger(global);
        let (channel, driver) = EncodedClientChannel::new_with_control_and_resource_ledger(
            protocol(),
            protocol(),
            MemoryTransport {
                state: Arc::clone(&state),
            },
            limit,
            ledger.clone(),
        )
        .expect("matching command and control protocols should compose");
        ((channel, driver, state), ledger)
    }

    #[test]
    fn successful_client_handshake_builds_a_directionally_bounded_channel() {
        // Arrange
        let client_receive = payload_limits(128);
        let server_receive = payload_limits(16);
        let selected = ProtocolVersion::new(1, 1);
        let hello = server_hello(selected, client_receive, server_receive);
        let state = Arc::new(Mutex::new(MemoryState {
            sent: VecDeque::new(),
            incoming: VecDeque::from([Ok(Some(encode_server_hello(&hello)))]),
            fail_send: false,
        }));
        let gate = EncodedClientHandshake::new(MemoryTransport {
            state: Arc::clone(&state),
        });
        let mut handshake = Box::pin(gate.establish(
            client_handshake_protocol(client_receive),
            client_hello(client_receive),
        ));

        // Act
        let Poll::Ready(Ok(negotiated)) = poll_once(handshake.as_mut()) else {
            panic!("ready memory handshake should succeed")
        };
        drop(handshake);
        let (channel, mut driver) = negotiated.into_channel(TestCodec, 1);
        let mut call = Box::pin(channel.call(command(1, "0123456789abcdef")));
        assert!(poll_once(call.as_mut()).is_pending());
        state
            .lock()
            .expect("memory transport should be available")
            .incoming
            .push_back(Ok(Some(
                WireFrame::new(
                    selected,
                    FrameRoute::Result {
                        call_id: CallId::new(1),
                    },
                    vec![b'r'; 64],
                    Vec::new(),
                    client_receive,
                )
                .expect("result should fit the client's receive limit"),
            )));
        let mut receive = Box::pin(driver.receive_once());
        let event = poll_once(receive.as_mut());
        drop(receive);
        let result = poll_once(call.as_mut());

        // Assert
        let state = state.lock().expect("memory transport should be available");
        assert_eq!(state.sent.len(), 2);
        assert_eq!(state.sent[0].route(), FrameRoute::Handshake);
        assert_eq!(state.sent[1].version(), selected);
        assert_eq!(state.sent[1].payload().len(), server_receive.payload_bytes);
        assert!(matches!(
            event,
            Poll::Ready(Ok(DriverEvent::Delivered(call))) if call.call_id() == CallId::new(1)
        ));
        let Poll::Ready(Ok(result)) = result else {
            panic!("delivered result should complete the call")
        };
        assert_eq!(
            result
                .into_outcome()
                .expect("result should be successful")
                .len(),
            64
        );
    }

    #[test]
    fn client_handshake_rejects_server_changes_to_advertised_receive_limits() {
        // Arrange
        let client_receive = payload_limits(128);
        let hello = server_hello(
            ProtocolVersion::new(1, 1),
            payload_limits(127),
            payload_limits(16),
        );
        let state = Arc::new(Mutex::new(MemoryState {
            sent: VecDeque::new(),
            incoming: VecDeque::from([Ok(Some(encode_server_hello(&hello)))]),
            fail_send: false,
        }));
        let gate = EncodedClientHandshake::new(MemoryTransport {
            state: Arc::clone(&state),
        });
        let mut handshake = Box::pin(gate.establish(
            client_handshake_protocol(client_receive),
            client_hello(client_receive),
        ));

        // Act
        let result = poll_once(handshake.as_mut());

        // Assert
        assert!(matches!(
            result,
            Poll::Ready(Err(ClientHandshakeError::Rejected(
                ClientHelloValidationError::ClientReceiveLimitsMismatch { .. }
            )))
        ));
        assert_eq!(
            state
                .lock()
                .expect("memory transport should be available")
                .sent
                .len(),
            1
        );
    }

    #[test]
    fn client_handshake_rejects_outer_and_selected_version_disagreement() {
        // Arrange
        let client_receive = payload_limits(128);
        let selected = ProtocolVersion::new(1, 1);
        let encoded =
            encode_server_hello(&server_hello(selected, client_receive, payload_limits(16)));
        let mismatched = WireFrame::new(
            ProtocolVersion::new(1, 2),
            encoded.route(),
            encoded.payload().to_vec(),
            Vec::new(),
            client_receive,
        )
        .expect("mismatched response should be syntactically valid");
        let state = Arc::new(Mutex::new(MemoryState {
            sent: VecDeque::new(),
            incoming: VecDeque::from([Ok(Some(mismatched))]),
            fail_send: false,
        }));
        let gate = EncodedClientHandshake::new(MemoryTransport { state });
        let mut handshake = Box::pin(gate.establish(
            client_handshake_protocol(client_receive),
            client_hello(client_receive),
        ));

        // Act
        let result = poll_once(handshake.as_mut());

        // Assert
        assert!(matches!(
            result,
            Poll::Ready(Err(ClientHandshakeError::SelectedVersionMismatch {
                outer,
                selected: payload,
            })) if outer == ProtocolVersion::new(1, 2) && payload == selected
        ));
    }

    #[test]
    fn client_handshake_rejects_local_receive_limit_mismatch_before_io() {
        // Arrange
        let protocol_receive = payload_limits(128);
        let advertised = payload_limits(129);
        let state = Arc::new(Mutex::new(MemoryState {
            sent: VecDeque::new(),
            incoming: VecDeque::new(),
            fail_send: false,
        }));
        let gate = EncodedClientHandshake::new(MemoryTransport {
            state: Arc::clone(&state),
        });
        let mut handshake = Box::pin(gate.establish(
            client_handshake_protocol(protocol_receive),
            client_hello(advertised),
        ));

        // Act
        let result = poll_once(handshake.as_mut());

        // Assert
        assert!(matches!(
            result,
            Poll::Ready(Err(ClientHandshakeError::Configuration(
                ClientHandshakeConfigurationError::ReceiveLimitsMismatch { .. }
            )))
        ));
        assert!(
            state
                .lock()
                .expect("memory transport should be available")
                .sent
                .is_empty()
        );
    }

    #[test]
    fn dropping_a_pending_client_handshake_drops_its_transport() {
        // Arrange
        let dropped = Arc::new(AtomicBool::new(false));
        let gate = EncodedClientHandshake::new(DropTrackedTransport {
            dropped: Arc::clone(&dropped),
        });
        let client_receive = payload_limits(128);
        let mut handshake = Box::pin(gate.establish(
            client_handshake_protocol(client_receive),
            client_hello(client_receive),
        ));
        assert!(poll_once(handshake.as_mut()).is_pending());

        // Act
        drop(handshake);

        // Assert
        assert!(dropped.load(Ordering::SeqCst));
    }

    fn command(call_id: u64, value: &str) -> Command<Echo> {
        Command::new(
            CallId::new(call_id),
            ObjectId::new(3),
            CallMetadata::default(),
            value.to_string(),
        )
    }

    #[test]
    fn production_profile_covers_full_duplex_and_pre_registration_peaks() {
        // Arrange
        let protocol = protocol();

        // Act
        let profile = ClientWireProductionProfile::new(&protocol, 2, transport_requirements());
        let requirements = profile.requirements();

        // Assert
        assert_eq!(
            requirements.class(ResourceClass::PendingCall),
            pwright_core::ResourceLimit::new(2, 1)
        );
        assert_eq!(
            requirements.class(ResourceClass::WireBytes),
            pwright_core::ResourceLimit::new(768, 512)
        );
        assert_eq!(requirements.scopes, 4);
        assert_eq!(requirements.leases, 8);
    }

    #[test]
    fn production_channel_shares_transport_ledger_for_calls_and_frames() {
        // Arrange
        let protocol = protocol();
        let profile = ClientWireProductionProfile::new(&protocol, 2, transport_requirements());
        let resources = production_root(profile);
        let ledger = resources
            .claim(profile.requirements())
            .expect("test should observe the production ledger");
        let state = Arc::new(Mutex::new(MemoryState {
            sent: VecDeque::new(),
            incoming: VecDeque::new(),
            fail_send: false,
        }));
        let transport = production_memory_transport(Arc::clone(&state), &resources);
        let (channel, _driver) =
            EncodedClientChannel::new_with_production_resources(protocol, transport, 2, &resources)
                .expect("matching production stack should build");
        let mut call = Box::pin(channel.call(command(1, "ping")));

        // Act
        let poll = poll_once(call.as_mut());
        let metrics = resources.metrics().expect("metrics should load");
        let sent_is_budgeted = state
            .lock()
            .expect("memory transport should be available")
            .sent
            .front()
            .expect("command frame should have been sent")
            .is_budgeted_by(&ledger);

        // Assert
        assert!(poll.is_pending());
        assert!(sent_is_budgeted);
        assert_eq!(metrics.class(ResourceClass::PendingCall).current, 1);
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 4);
    }

    #[test]
    fn production_channel_rejects_a_transport_from_another_connection_ledger() {
        // Arrange
        let protocol = protocol();
        let profile = ClientWireProductionProfile::new(&protocol, 2, transport_requirements());
        let client_resources = production_root(profile);
        let transport_resources = production_root(profile);
        let state = Arc::new(Mutex::new(MemoryState {
            sent: VecDeque::new(),
            incoming: VecDeque::new(),
            fail_send: false,
        }));
        let transport = production_memory_transport(state, &transport_resources);

        // Act
        let error = match EncodedClientChannel::new_with_production_resources(
            protocol,
            transport,
            2,
            &client_resources,
        ) {
            Ok(_) => panic!("different connection ledgers must not compose"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(
            error,
            ClientWireProductionError::TransportLedgerMismatch
        ));
        assert_eq!(
            client_resources
                .metrics()
                .expect("metrics should load")
                .active_leases,
            0
        );
    }

    fn response(call_id: u64, value: &[u8]) -> WireFrame {
        WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Result {
                call_id: CallId::new(call_id),
            },
            value.to_vec(),
            Vec::new(),
            limits(),
        )
        .expect("response frame should fit")
    }

    #[test]
    fn caller_driven_driver_round_trips_one_typed_command() {
        // Arrange
        let (channel, mut driver, transport) = setup(2);
        let mut call = Box::pin(channel.call(command(1, "ping")));

        // Act
        assert!(poll_once(call.as_mut()).is_pending());
        let sent_route = transport
            .lock()
            .expect("transport should be available")
            .sent
            .front()
            .expect("one command should be sent")
            .route();
        transport
            .lock()
            .expect("transport should be available")
            .incoming
            .push_back(Ok(Some(response(1, b"pong"))));
        let mut receive = Box::pin(driver.receive_once());
        let event = poll_once(receive.as_mut());
        drop(receive);
        let result = poll_once(call.as_mut());

        // Assert
        assert_eq!(
            sent_route,
            FrameRoute::Command {
                call_id: CallId::new(1),
                target: ObjectId::new(3),
                method: MethodTag::new(7),
            }
        );
        assert!(matches!(event, Poll::Ready(Ok(DriverEvent::Delivered(_)))));
        let Poll::Ready(Ok(result)) = result else {
            panic!("correlated call should complete");
        };
        assert_eq!(result.into_outcome(), Ok("pong".to_string()));
        let metrics = channel.metrics().expect("metrics should be available");
        assert_eq!(metrics.awaiting_results(), 0);
        assert_eq!(metrics.waiting_calls(), 0);
        assert_eq!(metrics.peak_slots(), 1);
    }

    #[test]
    fn delivered_result_releases_pending_budget_before_response_consumption() {
        // Arrange
        let ((channel, mut driver, transport), ledger) = setup_budgeted(1, 1);
        let mut call = Box::pin(channel.call(command(1, "ping")));
        assert!(poll_once(call.as_mut()).is_pending());
        let while_pending = ledger.metrics().expect("resource metrics should load");
        transport
            .lock()
            .expect("transport should be available")
            .incoming
            .push_back(Ok(Some(response(1, b"pong"))));

        // Act
        let mut receive = Box::pin(driver.receive_once());
        let event = poll_once(receive.as_mut());
        drop(receive);
        let after_delivery = ledger.metrics().expect("resource metrics should load");
        let scope_after_delivery = ledger
            .scope_metrics(ResourceScope::Call(CallId::new(1)))
            .expect("call scope metrics should load");
        let result = poll_once(call.as_mut());

        // Assert
        assert!(matches!(event, Poll::Ready(Ok(DriverEvent::Delivered(_)))));
        let Poll::Ready(Ok(result)) = result else {
            panic!("delivered result should complete the call");
        };
        assert_eq!(result.into_outcome(), Ok("pong".to_string()));
        assert_eq!(while_pending.class(ResourceClass::PendingCall).current, 1);
        assert_eq!(after_delivery.class(ResourceClass::PendingCall).current, 0);
        assert_eq!(after_delivery.class(ResourceClass::PendingCall).peak, 1);
        assert_eq!(after_delivery.active_leases, 0);
        assert!(scope_after_delivery.is_none());
    }

    #[test]
    fn pending_budget_rejects_before_client_state_or_transport_mutation() {
        // Arrange
        let ((channel, _driver, transport), ledger) = setup_budgeted(1, 0);
        let mut call = Box::pin(channel.call(command(1, "ping")));

        // Act
        let outcome = poll_once(call.as_mut());
        let channel_metrics = channel.metrics().expect("channel metrics should load");
        let resource_metrics = ledger.metrics().expect("resource metrics should load");
        let sent_frames = transport
            .lock()
            .expect("transport should be available")
            .sent
            .len();

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("resource rejection should complete locally");
        };
        assert_eq!(
            result
                .into_outcome()
                .expect_err("pending-call budget should reject")
                .kind(),
            FailureKind::ResourceExhausted
        );
        assert_eq!(channel_metrics.awaiting_results(), 0);
        assert_eq!(channel_metrics.waiting_calls(), 0);
        assert_eq!(channel_metrics.abandoned_calls(), 0);
        assert_eq!(channel_metrics.peak_slots(), 0);
        assert_eq!(sent_frames, 0);
        assert_eq!(
            resource_metrics.class(ResourceClass::PendingCall).current,
            0
        );
        assert_eq!(resource_metrics.class(ResourceClass::PendingCall).peak, 0);
        assert_eq!(resource_metrics.rejected_reservations, 1);
    }

    #[test]
    fn cancelled_sent_call_retains_pending_budget_until_terminal_result() {
        // Arrange
        let ((channel, mut driver, transport), ledger) = setup_budgeted_with_control(1, 1);
        let mut call = Box::pin(channel.call(command(1, "ping")));
        assert!(poll_once(call.as_mut()).is_pending());
        drop(call);
        let after_drop = ledger.metrics().expect("resource metrics should load");
        let mut flush = Box::pin(driver.flush_cancellation_once());
        let flush_outcome = poll_once(flush.as_mut());
        drop(flush);
        let after_cancel_send = ledger.metrics().expect("resource metrics should load");
        transport
            .lock()
            .expect("transport should be available")
            .incoming
            .push_back(Ok(Some(response(1, b"cancelled"))));

        // Act
        let mut receive = Box::pin(driver.receive_once());
        let event = poll_once(receive.as_mut());
        drop(receive);
        let after_result = ledger.metrics().expect("resource metrics should load");

        // Assert
        assert!(matches!(
            flush_outcome,
            Poll::Ready(Ok(CancellationFlushOutcome::Sent {
                call_id,
                cancellation,
            })) if call_id == CallId::new(1) && cancellation == CancellationId::new(1)
        ));
        assert!(matches!(
            event,
            Poll::Ready(Ok(DriverEvent::IgnoredAbandoned(call_id)))
                if call_id == CallId::new(1)
        ));
        assert_eq!(after_drop.class(ResourceClass::PendingCall).current, 1);
        assert_eq!(
            after_cancel_send.class(ResourceClass::PendingCall).current,
            1
        );
        assert_eq!(after_result.class(ResourceClass::PendingCall).current, 0);
        assert_eq!(after_result.active_leases, 0);
    }

    #[test]
    fn send_failure_closes_channel_and_releases_pending_budget() {
        // Arrange
        let ((channel, _driver, transport), ledger) = setup_budgeted(1, 1);
        transport
            .lock()
            .expect("transport should be available")
            .fail_send = true;
        let mut call = Box::pin(channel.call(command(1, "ping")));

        // Act
        let outcome = poll_once(call.as_mut());
        let resource_metrics = ledger.metrics().expect("resource metrics should load");

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("send failure should complete locally");
        };
        assert_eq!(
            result
                .into_outcome()
                .expect_err("send failure should fail")
                .kind(),
            FailureKind::TransportClosed
        );
        assert_eq!(
            resource_metrics.class(ResourceClass::PendingCall).current,
            0
        );
        assert_eq!(resource_metrics.class(ResourceClass::PendingCall).peak, 1);
        assert_eq!(resource_metrics.active_leases, 0);
        assert_eq!(
            channel
                .metrics()
                .expect("channel metrics should load")
                .close_reason(),
            Some(ChannelCloseReason::SendFailed)
        );
    }

    #[test]
    fn explicit_close_releases_every_pending_call_budget() {
        // Arrange
        let ((channel, _driver, _transport), ledger) = setup_budgeted(2, 2);
        let mut first = Box::pin(channel.call(command(1, "first")));
        let mut second = Box::pin(channel.call(command(2, "second")));
        assert!(poll_once(first.as_mut()).is_pending());
        assert!(poll_once(second.as_mut()).is_pending());
        let before_close = ledger.metrics().expect("resource metrics should load");

        // Act
        let woken = channel.close().expect("explicit close should succeed");
        let after_close = ledger.metrics().expect("resource metrics should load");
        let first_outcome = poll_once(first.as_mut());
        let second_outcome = poll_once(second.as_mut());

        // Assert
        assert_eq!(woken, 2);
        assert_eq!(before_close.class(ResourceClass::PendingCall).current, 2);
        assert_eq!(after_close.class(ResourceClass::PendingCall).current, 0);
        assert_eq!(after_close.class(ResourceClass::PendingCall).peak, 2);
        assert_eq!(after_close.active_leases, 0);
        for outcome in [first_outcome, second_outcome] {
            let Poll::Ready(Ok(result)) = outcome else {
                panic!("closed call should complete");
            };
            assert_eq!(
                result
                    .into_outcome()
                    .expect_err("closed call should fail")
                    .kind(),
                FailureKind::TransportClosed
            );
        }
    }

    #[test]
    fn pwright_remote_dispatch_crosses_the_encoded_channel() {
        // Arrange
        let (channel, mut driver, transport) = setup(2);
        let client = Client::new(channel, AtomicCallIdSource::new(40));
        let remote = client
            .root::<EchoObject>(RemoteInitializer::new(
                ObjectId::new(3),
                None,
                EchoObject::OBJECT_KIND,
            ))
            .expect("typed remote should initialize");
        let mut call = remote.dispatch::<Echo>("ping".to_string(), CallMetadata::default());

        // Act
        assert!(poll_once(call.as_mut()).is_pending());
        transport
            .lock()
            .expect("transport should be available")
            .incoming
            .push_back(Ok(Some(response(40, b"pong"))));
        let mut receive = Box::pin(driver.receive_once());
        assert!(matches!(
            poll_once(receive.as_mut()),
            Poll::Ready(Ok(DriverEvent::Delivered(_)))
        ));
        drop(receive);
        let result = poll_once(call.as_mut());

        // Assert
        assert_eq!(result, Poll::Ready(Ok("pong".to_string())));
    }

    #[test]
    fn completed_frame_send_releases_the_gate_before_waiting_for_its_result() {
        // Arrange
        let (channel, _driver, transport) = setup(2);
        let mut first = Box::pin(channel.call(command(1, "first")));
        let mut second = Box::pin(channel.call(command(2, "second")));

        // Act
        let first_poll = poll_once(first.as_mut());
        let second_poll = poll_once(second.as_mut());

        // Assert
        assert!(first_poll.is_pending());
        assert!(second_poll.is_pending());
        let state = transport.lock().expect("transport should be available");
        assert_eq!(state.sent.len(), 2);
        assert_eq!(
            state.sent.iter().map(WireFrame::route).collect::<Vec<_>>(),
            vec![
                FrameRoute::Command {
                    call_id: CallId::new(1),
                    target: ObjectId::new(3),
                    method: MethodTag::new(7),
                },
                FrameRoute::Command {
                    call_id: CallId::new(2),
                    target: ObjectId::new(3),
                    method: MethodTag::new(7),
                },
            ]
        );
    }

    #[test]
    fn dropped_sent_call_uses_a_bounded_tombstone_for_one_late_result() {
        // Arrange
        let (channel, mut driver, transport) = setup(1);
        let mut first = Box::pin(channel.call(command(1, "first")));
        assert!(poll_once(first.as_mut()).is_pending());

        // Act
        drop(first);
        let metrics_after_drop = channel.metrics().expect("metrics should be available");
        let mut blocked = Box::pin(channel.call(command(2, "blocked")));
        let blocked_result = poll_once(blocked.as_mut());
        transport
            .lock()
            .expect("transport should be available")
            .incoming
            .push_back(Ok(Some(response(1, b"late"))));
        let mut receive = Box::pin(driver.receive_once());
        let late_event = poll_once(receive.as_mut());
        drop(receive);

        // Assert
        assert_eq!(metrics_after_drop.abandoned_calls(), 1);
        let Poll::Ready(Ok(blocked_result)) = blocked_result else {
            panic!("capacity rejection should be immediate");
        };
        assert_eq!(
            blocked_result
                .into_outcome()
                .expect_err("tombstone should retain the only slot")
                .kind(),
            FailureKind::ResourceExhausted
        );
        assert!(matches!(
            late_event,
            Poll::Ready(Ok(DriverEvent::IgnoredAbandoned(call_id)))
                if call_id == CallId::new(1)
        ));
        assert_eq!(
            channel
                .metrics()
                .expect("metrics should be available")
                .abandoned_calls(),
            0
        );
    }

    #[test]
    fn dropped_controlled_call_queues_and_sends_one_correlated_cancellation() {
        // Arrange
        let (channel, mut driver, transport) = setup_with_control(1);
        let mut call = Box::pin(channel.call(command(1, "first")));
        assert!(poll_once(call.as_mut()).is_pending());

        // Act
        drop(call);
        let metrics_after_drop = channel.metrics().expect("metrics should be available");
        let mut flush = Box::pin(driver.flush_cancellation_once());
        let flushed = poll_once(flush.as_mut());
        drop(flush);
        let cancel_frame = transport
            .lock()
            .expect("transport should be available")
            .sent
            .back()
            .expect("cancellation should be sent")
            .clone();
        let cancellation = protocol()
            .decode::<ControlMessage>(cancel_frame)
            .expect("cancellation should decode");
        transport
            .lock()
            .expect("transport should be available")
            .incoming
            .push_back(Ok(Some(response(1, b"cancelled"))));
        let mut receive = Box::pin(driver.receive_once());
        let late = poll_once(receive.as_mut());
        drop(receive);

        // Assert
        assert_eq!(metrics_after_drop.abandoned_calls(), 1);
        assert_eq!(metrics_after_drop.queued_cancellations(), 1);
        assert_eq!(metrics_after_drop.peak_queued_cancellations(), 1);
        assert!(matches!(
            flushed,
            Poll::Ready(Ok(CancellationFlushOutcome::Sent {
                call_id,
                cancellation,
            })) if call_id == CallId::new(1) && cancellation == CancellationId::new(1)
        ));
        assert_eq!(
            cancellation,
            ControlMessage::Cancel {
                call_id: CallId::new(1),
                cancellation: CancellationId::new(1),
                reason: CancellationReason::Caller,
            }
        );
        assert!(matches!(
            late,
            Poll::Ready(Ok(DriverEvent::IgnoredAbandoned(call_id)))
                if call_id == CallId::new(1)
        ));
        let metrics = channel.metrics().expect("metrics should be available");
        assert_eq!(metrics.abandoned_calls(), 0);
        assert_eq!(metrics.queued_cancellations(), 0);
        assert_eq!(metrics.sent_cancellations(), 1);
    }

    #[test]
    fn dropping_a_call_waiting_for_the_send_gate_does_not_queue_cancellation() {
        // Arrange
        let state = Arc::new(Mutex::new(BlockingSendState {
            send_polls: 0,
            shutdowns: 0,
            sent: VecDeque::new(),
        }));
        let (channel, mut driver) = EncodedClientChannel::new_with_control(
            protocol(),
            protocol(),
            BlockingSendTransport {
                state: Arc::clone(&state),
            },
            2,
        )
        .expect("matching protocols should compose");
        let mut first = Box::pin(channel.call(command(1, "first")));
        let mut waiting = Box::pin(channel.call(command(2, "waiting")));
        assert!(poll_once(first.as_mut()).is_pending());
        assert!(poll_once(waiting.as_mut()).is_pending());
        assert_eq!(
            state.lock().expect("state should be available").send_polls,
            1
        );

        // Act
        drop(waiting);
        let after_waiter_drop = channel.metrics().expect("metrics should be available");
        drop(first);
        let after_sent_drop = channel.metrics().expect("metrics should be available");
        let mut flush = Box::pin(driver.flush_cancellation_once());
        let flushed = poll_once(flush.as_mut());
        drop(flush);

        // Assert
        assert_eq!(after_waiter_drop.waiting_calls(), 1);
        assert_eq!(after_waiter_drop.abandoned_calls(), 0);
        assert_eq!(after_waiter_drop.queued_cancellations(), 0);
        assert_eq!(after_sent_drop.abandoned_calls(), 1);
        assert_eq!(after_sent_drop.queued_cancellations(), 1);
        assert!(matches!(
            flushed,
            Poll::Ready(Ok(CancellationFlushOutcome::Sent { call_id, .. }))
                if call_id == CallId::new(1)
        ));
        let state = state.lock().expect("state should be available");
        assert_eq!(state.send_polls, 2);
        assert_eq!(state.sent.len(), 1);
        assert_eq!(
            state.sent.front().map(WireFrame::route),
            Some(FrameRoute::Control)
        );
    }

    #[test]
    fn driver_shutdown_waits_for_the_active_writer_and_blocks_queued_sends() {
        // Arrange
        let state = Arc::new(Mutex::new(BlockingSendState {
            send_polls: 0,
            shutdowns: 0,
            sent: VecDeque::new(),
        }));
        let transport = BlockingSendTransport {
            state: Arc::clone(&state),
        };
        let (channel, mut driver) = EncodedClientChannel::new(protocol(), transport, 2);
        let mut active = Box::pin(channel.call(command(1, "active")));
        let mut queued = Box::pin(channel.call(command(2, "queued")));
        assert!(poll_once(active.as_mut()).is_pending());
        assert!(poll_once(queued.as_mut()).is_pending());
        let mut close = Box::pin(driver.close_transport());
        assert!(poll_once(close.as_mut()).is_pending());

        // Act
        drop(active);
        let closed = poll_once(close.as_mut());
        drop(close);
        let queued = poll_once(queued.as_mut());
        let metrics = channel.metrics().expect("channel metrics should load");
        let state = state.lock().expect("transport state should load");

        // Assert
        assert!(matches!(closed, Poll::Ready(Ok(0))));
        let Poll::Ready(Ok(queued)) = queued else {
            panic!("queued call should stop after transport shutdown");
        };
        assert_eq!(
            queued
                .into_outcome()
                .expect_err("queued call should fail")
                .kind(),
            FailureKind::TransportClosed
        );
        assert_eq!(metrics.close_reason(), Some(ChannelCloseReason::Explicit));
        assert_eq!(state.send_polls, 1);
        assert_eq!(state.shutdowns, 1);
        assert!(state.sent.is_empty());
    }

    #[test]
    fn cancellation_waiters_take_priority_over_queued_command_sends() {
        // Arrange
        let gate = Arc::new(super::SendGate::new(3));
        let mut active = Box::pin(
            gate.acquire(super::SendPriority::Command)
                .expect("first waiter should fit"),
        );
        let Poll::Ready(Ok(active)) = poll_once(active.as_mut()) else {
            panic!("first command should acquire the send gate");
        };
        let mut command = Box::pin(
            gate.acquire(super::SendPriority::Command)
                .expect("second command should fit"),
        );
        let mut cancellation = Box::pin(
            gate.acquire(super::SendPriority::Control)
                .expect("control waiter should fit"),
        );
        assert!(poll_once(command.as_mut()).is_pending());
        assert!(poll_once(cancellation.as_mut()).is_pending());

        // Act
        drop(active);
        let command_after_release = poll_once(command.as_mut());
        let cancellation_after_release = poll_once(cancellation.as_mut());

        // Assert
        assert!(command_after_release.is_pending());
        assert!(matches!(cancellation_after_release, Poll::Ready(Ok(_))));
    }

    #[test]
    fn explicit_cancellation_identity_is_preserved_and_outbox_stays_bounded() {
        // Arrange
        let (channel, _driver, _transport) = setup_with_control(1);
        let explicit = Command::new(
            CallId::new(7),
            ObjectId::new(3),
            CallMetadata::new(None, Some(CancellationId::new(99))),
            "first".to_string(),
        );
        let mut first = Box::pin(channel.call(explicit));
        assert!(poll_once(first.as_mut()).is_pending());

        // Act
        drop(first);
        let mut blocked = Box::pin(channel.call(command(8, "blocked")));
        let blocked = poll_once(blocked.as_mut());

        // Assert
        let Poll::Ready(Ok(blocked)) = blocked else {
            panic!("bounded tombstone should reject another call");
        };
        assert_eq!(
            blocked
                .into_outcome()
                .expect_err("second call should exceed capacity")
                .kind(),
            FailureKind::ResourceExhausted
        );
        let state = channel
            .shared
            .state
            .lock()
            .expect("state should be available");
        assert_eq!(
            state.cancellations.get(&CallId::new(7)),
            Some(&CancellationId::new(99))
        );
        assert_eq!(state.cancellations.len(), 1);
    }

    #[test]
    fn cancellation_send_failure_closes_and_releases_all_bounded_state() {
        // Arrange
        let (channel, mut driver, transport) = setup_with_control(1);
        let mut call = Box::pin(channel.call(command(1, "first")));
        assert!(poll_once(call.as_mut()).is_pending());
        drop(call);
        transport
            .lock()
            .expect("transport should be available")
            .fail_send = true;

        // Act
        let mut flush = Box::pin(driver.flush_cancellation_once());
        let result = poll_once(flush.as_mut());
        drop(flush);

        // Assert
        assert!(matches!(
            result,
            Poll::Ready(Err(super::CancellationFlushError::Transport(
                TestTransportError("send failed")
            )))
        ));
        let metrics = channel.metrics().expect("metrics should be available");
        assert_eq!(metrics.abandoned_calls(), 0);
        assert_eq!(metrics.queued_cancellations(), 0);
        assert_eq!(metrics.close_reason(), Some(ChannelCloseReason::SendFailed));
    }

    #[test]
    fn control_protocol_must_match_command_version_policy_and_frame_limits() {
        // Arrange
        let transport = || MemoryTransport {
            state: Arc::new(Mutex::new(MemoryState {
                sent: VecDeque::new(),
                incoming: VecDeque::new(),
                fail_send: false,
            })),
        };
        let expanded = VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 1))
            .expect("expanded policy should build");
        let send_mismatch = custom_protocol(expanded, ProtocolVersion::new(1, 1), limits());
        let policy_mismatch = custom_protocol(expanded, ProtocolVersion::new(1, 0), limits());
        let limit_mismatch = custom_protocol(
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("policy should build"),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(255, 0, 0, 255),
        );
        let receive_limit_mismatch = WireProtocol::new_directional(
            TestCodec,
            VersionPolicy::exact(ProtocolVersion::new(1, 0)),
            ProtocolVersion::new(1, 0),
            limits(),
            FrameLimits::new(255, 0, 0, 255),
        )
        .expect("directional control protocol should build");

        // Act
        let version =
            EncodedClientChannel::new_with_control(protocol(), send_mismatch, transport(), 1).err();
        let policy =
            EncodedClientChannel::new_with_control(protocol(), policy_mismatch, transport(), 1)
                .err();
        let frame_limits =
            EncodedClientChannel::new_with_control(protocol(), limit_mismatch, transport(), 1)
                .err();
        let receive_frame_limits = EncodedClientChannel::new_with_control(
            protocol(),
            receive_limit_mismatch,
            transport(),
            1,
        )
        .err();

        // Assert
        assert_eq!(
            version,
            Some(ClientWireConfigurationError::ControlVersionMismatch)
        );
        assert_eq!(
            policy,
            Some(ClientWireConfigurationError::ControlVersionPolicyMismatch)
        );
        assert_eq!(
            frame_limits,
            Some(ClientWireConfigurationError::ControlFrameLimitsMismatch)
        );
        assert_eq!(
            receive_frame_limits,
            Some(ClientWireConfigurationError::ControlFrameLimitsMismatch)
        );
    }

    #[test]
    fn control_encoding_failure_retains_the_cancellation_for_explicit_retry() {
        // Arrange
        let state = Arc::new(Mutex::new(MemoryState {
            sent: VecDeque::new(),
            incoming: VecDeque::new(),
            fail_send: false,
        }));
        let failing = WireProtocol::new(
            FailingControlCodec,
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("policy should build"),
            ProtocolVersion::new(1, 0),
            limits(),
        )
        .expect("failing codec protocol should still configure");
        let (channel, mut driver) = EncodedClientChannel::new_with_control(
            protocol(),
            failing,
            MemoryTransport { state },
            1,
        )
        .expect("matching protocol envelopes should compose");
        let mut call = Box::pin(channel.call(command(1, "first")));
        assert!(poll_once(call.as_mut()).is_pending());
        drop(call);

        // Act
        let mut flush = Box::pin(driver.flush_cancellation_once());
        let result = poll_once(flush.as_mut());
        drop(flush);

        // Assert
        assert!(matches!(
            result,
            Poll::Ready(Err(CancellationFlushError::Protocol(message)))
                if message.contains("control encoding failed")
        ));
        let metrics = channel.metrics().expect("metrics should be available");
        assert_eq!(metrics.abandoned_calls(), 1);
        assert_eq!(metrics.queued_cancellations(), 1);
        assert_eq!(metrics.sent_cancellations(), 0);
        assert_eq!(metrics.close_reason(), None);
    }

    #[test]
    fn end_of_stream_terminally_wakes_pending_calls() {
        // Arrange
        let (channel, mut driver, _transport) = setup(1);
        let mut call = Box::pin(channel.call(command(1, "ping")));
        assert!(poll_once(call.as_mut()).is_pending());

        // Act
        let mut receive = Box::pin(driver.receive_once());
        let close_event = poll_once(receive.as_mut());
        drop(receive);
        let result = poll_once(call.as_mut());

        // Assert
        assert!(matches!(
            close_event,
            Poll::Ready(Ok(DriverEvent::Closed(ChannelCloseReason::EndOfStream)))
        ));
        let Poll::Ready(Ok(result)) = result else {
            panic!("closed call should complete");
        };
        assert_eq!(
            result
                .into_outcome()
                .expect_err("closed transport should fail")
                .kind(),
            FailureKind::TransportClosed
        );
        assert_eq!(
            channel
                .metrics()
                .expect("metrics should be available")
                .close_reason(),
            Some(ChannelCloseReason::EndOfStream)
        );
    }

    #[test]
    fn dropping_the_unique_receive_driver_terminally_wakes_calls() {
        // Arrange
        let (channel, driver, _transport) = setup(1);
        let mut call = Box::pin(channel.call(command(1, "ping")));
        assert!(poll_once(call.as_mut()).is_pending());

        // Act
        drop(driver);
        let result = poll_once(call.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = result else {
            panic!("driver drop should wake the call");
        };
        assert_eq!(
            result
                .into_outcome()
                .expect_err("missing receive driver should close the call")
                .kind(),
            FailureKind::TransportClosed
        );
        assert_eq!(
            channel
                .metrics()
                .expect("metrics should be available")
                .close_reason(),
            Some(ChannelCloseReason::DriverDropped)
        );
    }

    #[test]
    fn receive_failure_terminally_closes_pending_calls() {
        // Arrange
        let (channel, mut driver, transport) = setup(1);
        let mut call = Box::pin(channel.call(command(1, "ping")));
        assert!(poll_once(call.as_mut()).is_pending());
        transport
            .lock()
            .expect("transport should be available")
            .incoming
            .push_back(Err(TestTransportError("receive failed")));

        // Act
        let mut receive = Box::pin(driver.receive_once());
        let driver_result = poll_once(receive.as_mut());
        drop(receive);
        let call_result = poll_once(call.as_mut());

        // Assert
        assert!(matches!(
            driver_result,
            Poll::Ready(Err(DriverError::Transport(TestTransportError(
                "receive failed"
            ))))
        ));
        let Poll::Ready(Ok(call_result)) = call_result else {
            panic!("receive failure should wake the call");
        };
        assert_eq!(
            call_result
                .into_outcome()
                .expect_err("receive failure should close the call")
                .kind(),
            FailureKind::TransportClosed
        );
    }

    #[test]
    fn malformed_result_closes_channel_as_a_protocol_violation() {
        // Arrange
        let (channel, mut driver, transport) = setup(1);
        let mut call = Box::pin(channel.call(command(1, "ping")));
        assert!(poll_once(call.as_mut()).is_pending());
        transport
            .lock()
            .expect("transport should be available")
            .incoming
            .push_back(Ok(Some(response(1, &[0xff]))));

        // Act
        let mut receive = Box::pin(driver.receive_once());
        assert!(matches!(
            poll_once(receive.as_mut()),
            Poll::Ready(Ok(DriverEvent::Delivered(_)))
        ));
        drop(receive);
        let result = poll_once(call.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = result else {
            panic!("malformed result should terminate the call");
        };
        assert_eq!(
            result
                .into_outcome()
                .expect_err("invalid payload should fail")
                .kind(),
            FailureKind::ProtocolViolation
        );
        assert_eq!(
            channel
                .metrics()
                .expect("metrics should be available")
                .close_reason(),
            Some(ChannelCloseReason::ProtocolViolation)
        );
    }

    #[test]
    fn unknown_result_is_reported_without_consuming_a_valid_call() {
        // Arrange
        let (channel, mut driver, transport) = setup(1);
        let mut call = Box::pin(channel.call(command(1, "ping")));
        assert!(poll_once(call.as_mut()).is_pending());
        transport
            .lock()
            .expect("transport should be available")
            .incoming
            .push_back(Ok(Some(response(99, b"unknown"))));

        // Act
        let mut receive = Box::pin(driver.receive_once());
        let event = poll_once(receive.as_mut());
        drop(receive);

        // Assert
        assert!(matches!(
            event,
            Poll::Ready(Err(DriverError::Correlation(_)))
        ));
        assert_eq!(
            channel
                .metrics()
                .expect("metrics should be available")
                .awaiting_results(),
            1
        );
    }

    #[test]
    fn send_failure_terminally_closes_the_channel() {
        // Arrange
        let (channel, _driver, transport) = setup(1);
        transport
            .lock()
            .expect("transport should be available")
            .fail_send = true;
        let mut call = Box::pin(channel.call(command(1, "ping")));

        // Act
        let result = poll_once(call.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = result else {
            panic!("send failure should be immediate");
        };
        assert_eq!(
            result
                .into_outcome()
                .expect_err("send failure should fail")
                .kind(),
            FailureKind::TransportClosed
        );
        assert_eq!(
            channel
                .metrics()
                .expect("metrics should be available")
                .close_reason(),
            Some(ChannelCloseReason::SendFailed)
        );
    }

    #[test]
    fn unhandled_control_frame_is_returned_to_the_composing_driver() {
        // Arrange
        let (_channel, mut driver, transport) = setup(1);
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            Vec::new(),
            Vec::new(),
            limits(),
        )
        .expect("control frame should fit");
        transport
            .lock()
            .expect("transport should be available")
            .incoming
            .push_back(Ok(Some(frame)));

        // Act
        let mut receive = Box::pin(driver.receive_once());
        let event = poll_once(receive.as_mut());

        // Assert
        assert!(matches!(
            event,
            Poll::Ready(Ok(DriverEvent::Unhandled(frame)))
                if frame.route() == FrameRoute::Control
        ));
    }

    #[test]
    fn chained_handlers_route_to_the_first_matching_optional_subsystem() {
        // Arrange
        let (_channel, mut driver, transport) = setup(1);
        let event_route = FrameRoute::Event {
            source: ObjectId::new(3),
            event: EventTag::new(8),
        };
        transport
            .lock()
            .expect("transport should be available")
            .incoming
            .push_back(Ok(Some(
                WireFrame::new(
                    ProtocolVersion::new(1, 0),
                    event_route,
                    Vec::new(),
                    Vec::new(),
                    limits(),
                )
                .expect("event frame should fit"),
            )));
        let mut handlers = RouteHandler {
            route: FrameRoute::Control,
            event: "control",
        }
        .or_else(RouteHandler {
            route: event_route,
            event: "event",
        });

        // Act
        let mut receive = Box::pin(driver.receive_once_with(&mut handlers));
        let routed = poll_once(receive.as_mut());

        // Assert
        assert!(matches!(
            routed,
            Poll::Ready(Ok(RoutedDriverEvent::Handled(Either::Right("event"))))
        ));
    }

    #[test]
    fn handler_chain_preserves_frames_that_no_optional_subsystem_claims() {
        // Arrange
        let (_channel, mut driver, transport) = setup(1);
        transport
            .lock()
            .expect("transport should be available")
            .incoming
            .push_back(Ok(Some(
                WireFrame::new(
                    ProtocolVersion::new(1, 0),
                    FrameRoute::Handshake,
                    Vec::new(),
                    Vec::new(),
                    limits(),
                )
                .expect("handshake frame should fit"),
            )));
        let mut handlers = RouteHandler {
            route: FrameRoute::Control,
            event: "control",
        }
        .or_else(RouteHandler {
            route: FrameRoute::Event {
                source: ObjectId::new(3),
                event: EventTag::new(8),
            },
            event: "event",
        });

        // Act
        let mut receive = Box::pin(driver.receive_once_with(&mut handlers));
        let routed = poll_once(receive.as_mut());

        // Assert
        assert!(matches!(
            routed,
            Poll::Ready(Ok(RoutedDriverEvent::Unhandled(frame)))
                if frame.route() == FrameRoute::Handshake
        ));
    }

    #[test]
    fn explicit_close_is_idempotent_and_rejects_new_calls() {
        // Arrange
        let (channel, _driver, _transport) = setup(1);

        // Act
        let first = channel.close();
        let second = channel.close();
        let mut call = Box::pin(channel.call(command(1, "ping")));
        let result = poll_once(call.as_mut());

        // Assert
        assert_eq!(first, Ok(0));
        assert_eq!(second, Ok(0));
        let Poll::Ready(Ok(result)) = result else {
            panic!("closed channel should reject calls");
        };
        let error: OperationError<String> = result
            .into_outcome()
            .expect_err("closed channel should fail");
        assert_eq!(error.kind(), FailureKind::TransportClosed);
    }
}
