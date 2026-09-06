//! Caller-driven, composable typed command handling for remote Pwright peers.
//!
//! One `TypedCommandHandler` installs exactly one `CommandSpec`. Handlers can
//! be chained without a dynamic registry. `EncodedServerDriver` owns the only
//! receive path and starts no task, thread, timer, socket, or runtime.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::{Future, Pending, Ready, pending, poll_fn, ready};
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use pwright_connection_control::{
    ServerCloseAction, ServerCloseError, ServerCloseMachine, ServerClosePhase,
};
use pwright_core::{
    CallId, Capability, CapabilityCommandDescriptor, CapabilityCommandSpec, CapabilityDescriptor,
    CapabilityEventDescriptor, CapabilityEventSpec, CapabilityId, Command, CommandId,
    CommandResult, CommandSpec, ControlMessage, EventId, EventTag, MethodTag, ObjectId,
    ObjectKindCatalog, ObjectKindCatalogError, ObjectKindId, ProductionResourceError,
    ProductionResourceLedger, ProtocolVersion, RegistryBuilder, RegistryError, ResourceClass,
    ResourceError, ResourceLease, ResourceLedger, ResourceRequirements, ResourceScope, Service,
};
use pwright_transport::{
    CapabilityIdError, CapabilityManifest, CapabilitySetError, CapabilitySupport, ClientHello,
    FrameError, FrameLimits, FrameRoute, FrameTransport, NegotiationError, NegotiationLimits,
    ServerHandshakeConfig, ServerHello, ShutdownFrameTransport, VersionPolicy, WireCapabilityId,
    WireCodec, WireFrame, WireProtocol, WireProtocolError, negotiate,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerFrameDisposition<Event> {
    Handled(Event),
    Unhandled(WireFrame),
}

pub trait ServerFrameHandler<Transport> {
    type Event;
    type Error;
    type Future<'a>: Future<Output = Result<ServerFrameDisposition<Self::Event>, Self::Error>>
    where
        Self: 'a,
        Transport: 'a;

    fn handle<'a>(&'a mut self, frame: WireFrame, transport: &'a Transport) -> Self::Future<'a>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Either<Left, Right> {
    Left(Left),
    Right(Right),
}

#[derive(Debug, Eq, PartialEq)]
pub enum ServerHandlerChainError<Left, Right> {
    Left(Left),
    Right(Right),
}

impl<Left, Right> Display for ServerHandlerChainError<Left, Right>
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

impl<Left, Right> Error for ServerHandlerChainError<Left, Right>
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

pub struct ServerHandlerChain<Left, Right> {
    left: Left,
    right: Right,
}

impl<Left, Right> ServerHandlerChain<Left, Right> {
    pub const fn new(left: Left, right: Right) -> Self {
        Self { left, right }
    }

    pub fn into_parts(self) -> (Left, Right) {
        (self.left, self.right)
    }
}

impl<Transport, Left, Right> ServerFrameHandler<Transport> for ServerHandlerChain<Left, Right>
where
    Left: ServerFrameHandler<Transport>,
    Right: ServerFrameHandler<Transport>,
{
    type Event = Either<Left::Event, Right::Event>;
    type Error = ServerHandlerChainError<Left::Error, Right::Error>;
    type Future<'a>
        =
        Pin<Box<dyn Future<Output = Result<ServerFrameDisposition<Self::Event>, Self::Error>> + 'a>>
    where
        Self: 'a,
        Transport: 'a;

    fn handle<'a>(&'a mut self, frame: WireFrame, transport: &'a Transport) -> Self::Future<'a> {
        Box::pin(async move {
            match self
                .left
                .handle(frame, transport)
                .await
                .map_err(ServerHandlerChainError::Left)?
            {
                ServerFrameDisposition::Handled(event) => {
                    Ok(ServerFrameDisposition::Handled(Either::Left(event)))
                }
                ServerFrameDisposition::Unhandled(frame) => self
                    .right
                    .handle(frame, transport)
                    .await
                    .map(|disposition| match disposition {
                        ServerFrameDisposition::Handled(event) => {
                            ServerFrameDisposition::Handled(Either::Right(event))
                        }
                        ServerFrameDisposition::Unhandled(frame) => {
                            ServerFrameDisposition::Unhandled(frame)
                        }
                    })
                    .map_err(ServerHandlerChainError::Right),
            }
        })
    }
}

pub trait ServerFrameHandlerExt<Transport>: ServerFrameHandler<Transport> + Sized {
    fn or_else<Next>(self, next: Next) -> ServerHandlerChain<Self, Next>
    where
        Next: ServerFrameHandler<Transport>,
    {
        ServerHandlerChain::new(self, next)
    }
}

impl<Transport, Handler> ServerFrameHandlerExt<Transport> for Handler where
    Handler: ServerFrameHandler<Transport>
{
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandshakeEstablished {
    selected_version: ProtocolVersion,
    enabled_capabilities: usize,
}

impl HandshakeEstablished {
    pub const fn selected_version(self) -> ProtocolVersion {
        self.selected_version
    }

    pub const fn enabled_capabilities(self) -> usize {
        self.enabled_capabilities
    }
}

/// Authorizes a bounded decoded hello before capability negotiation. The gate
/// may capture identity established by a Unix socket, mTLS, Tailscale, or an
/// endpoint adapter; credentials do not have to enter the canonical hello.
pub trait ServerHandshakeGate {
    type Authorization;
    type Error;
    type Future<'a>: Future<Output = Result<Self::Authorization, Self::Error>>
    where
        Self: 'a;

    fn authorize<'a>(&'a mut self, hello: &'a ClientHello) -> Self::Future<'a>;
}

/// Explicit opt-in for trusted local/in-process connections with no identity
/// check. Remote endpoint factories should inject a real gate.
#[derive(Clone, Copy, Debug, Default)]
pub struct AllowUnauthenticated;

impl ServerHandshakeGate for AllowUnauthenticated {
    type Authorization = ();
    type Error = std::convert::Infallible;
    type Future<'a> = Ready<Result<Self::Authorization, Self::Error>>;

    fn authorize<'a>(&'a mut self, _hello: &'a ClientHello) -> Self::Future<'a> {
        ready(Ok(()))
    }
}

pub struct AcceptedServerHandshake<Authorization> {
    hello: ServerHello,
    authorization: Authorization,
    resource_ledger: Option<ResourceLedger>,
}

impl<Authorization> AcceptedServerHandshake<Authorization> {
    pub const fn hello(&self) -> &ServerHello {
        &self.hello
    }

    pub const fn authorization(&self) -> &Authorization {
        &self.authorization
    }

    /// Returns whether handshake acceptance retained a production connection
    /// budget identity from its supervisor.
    pub const fn has_production_resource_budget(&self) -> bool {
        self.resource_ledger.is_some()
    }

    /// Returns whether this accepted session was established by a production
    /// supervisor accounting against the supplied connection budget.
    ///
    /// This is an identity comparison, not a comparison of equal limits or
    /// plans. Capability leaves that transfer connection-local authority can
    /// therefore reject an accepted hello produced by another supervisor.
    pub fn shares_production_resource_budget_with(
        &self,
        resources: &ProductionResourceLedger,
    ) -> bool {
        let Some(accepted) = &self.resource_ledger else {
            return false;
        };
        resources
            .claim(ResourceRequirements::none())
            .is_ok_and(|candidate| accepted.shares_budget_with(&candidate))
    }

    /// Builds the server side of the post-handshake application protocol.
    /// Outbound frames obey the client's receive limits and inbound frames
    /// obey the server limits advertised during negotiation.
    pub fn application_protocol<Codec>(&self, codec: Codec) -> WireProtocol<Codec> {
        WireProtocol::new_exact_directional(
            codec,
            self.hello.selected_version(),
            self.hello.client_receive_limits(),
            self.hello.server_receive_limits(),
        )
    }

    pub fn into_parts(self) -> (ServerHello, Authorization) {
        (self.hello, self.authorization)
    }
}

#[derive(Debug)]
pub enum ServerHandshakeError<Decode, Encode, Authorization, Transport> {
    Closed(ServerSupervisorCloseReason),
    AlreadyEstablished,
    EndOfStream,
    Transport(Transport),
    HandshakeRequired { actual: FrameRoute },
    IncomingFrame(FrameError),
    DecodeClientHello(Decode),
    PeerReceiveFrameLimitExceeded { advertised: usize, maximum: usize },
    Authorization(Authorization),
    Negotiate(NegotiationError),
    EncodeServerHello(Encode),
    EncodedRouteMismatch { actual: FrameRoute },
    OutgoingFrame(FrameError),
}

impl<Decode, Encode, Authorization, Transport> Display
    for ServerHandshakeError<Decode, Encode, Authorization, Transport>
where
    Decode: Display,
    Encode: Display,
    Authorization: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed(reason) => write!(formatter, "server supervisor is closed: {reason:?}"),
            Self::AlreadyEstablished => {
                formatter.write_str("server handshake is already established")
            }
            Self::EndOfStream => formatter.write_str("connection ended before client hello"),
            Self::Transport(error) => Display::fmt(error, formatter),
            Self::HandshakeRequired { actual } => {
                write!(formatter, "expected client hello, received {actual:?}")
            }
            Self::IncomingFrame(error) | Self::OutgoingFrame(error) => {
                Display::fmt(error, formatter)
            }
            Self::DecodeClientHello(error) => Display::fmt(error, formatter),
            Self::PeerReceiveFrameLimitExceeded {
                advertised,
                maximum,
            } => write!(
                formatter,
                "peer receive frame limit {advertised} exceeds production maximum {maximum}"
            ),
            Self::Authorization(error) => Display::fmt(error, formatter),
            Self::Negotiate(error) => Display::fmt(error, formatter),
            Self::EncodeServerHello(error) => Display::fmt(error, formatter),
            Self::EncodedRouteMismatch { actual } => write!(
                formatter,
                "encoded server hello has route {actual:?}, expected Handshake"
            ),
        }
    }
}

impl<Decode, Encode, Authorization, Transport> Error
    for ServerHandshakeError<Decode, Encode, Authorization, Transport>
where
    Decode: Error + 'static,
    Encode: Error + 'static,
    Authorization: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            Self::IncomingFrame(error) | Self::OutgoingFrame(error) => Some(error),
            Self::DecodeClientHello(error) => Some(error),
            Self::Authorization(error) => Some(error),
            Self::Negotiate(error) => Some(error),
            Self::EncodeServerHello(error) => Some(error),
            Self::Closed(_)
            | Self::AlreadyEstablished
            | Self::EndOfStream
            | Self::HandshakeRequired { .. }
            | Self::PeerReceiveFrameLimitExceeded { .. }
            | Self::EncodedRouteMismatch { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HandshakeFirstEvent<Event> {
    Established(HandshakeEstablished),
    Application(Event),
}

#[derive(Debug)]
pub enum HandshakeFirstError<Decode, Encode, Transport, Application> {
    HandshakeRequired {
        actual: FrameRoute,
    },
    DuplicateHandshake,
    IncomingFrame(FrameError),
    DecodeClientHello(Decode),
    Negotiate(NegotiationError),
    EncodeServerHello(Encode),
    EncodedRouteMismatch {
        actual: FrameRoute,
    },
    OutgoingFrame(FrameError),
    Transport(Transport),
    VersionMismatch {
        expected: ProtocolVersion,
        actual: ProtocolVersion,
    },
    Application(Application),
}

impl<Decode, Encode, TransportError, Application> Display
    for HandshakeFirstError<Decode, Encode, TransportError, Application>
where
    Decode: Display,
    Encode: Display,
    TransportError: Display,
    Application: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::HandshakeRequired { actual } => {
                write!(
                    formatter,
                    "expected initial handshake frame, received {actual:?}"
                )
            }
            Self::DuplicateHandshake => {
                formatter.write_str("received a second handshake after negotiation")
            }
            Self::IncomingFrame(error) | Self::OutgoingFrame(error) => {
                Display::fmt(error, formatter)
            }
            Self::DecodeClientHello(error) => Display::fmt(error, formatter),
            Self::Negotiate(error) => Display::fmt(error, formatter),
            Self::EncodeServerHello(error) => Display::fmt(error, formatter),
            Self::EncodedRouteMismatch { actual } => write!(
                formatter,
                "encoded server hello has route {actual:?}, expected Handshake"
            ),
            Self::Transport(error) => Display::fmt(error, formatter),
            Self::VersionMismatch { expected, actual } => write!(
                formatter,
                "post-handshake frame uses protocol version {actual}, expected {expected}"
            ),
            Self::Application(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Decode, Encode, TransportError, Application> Error
    for HandshakeFirstError<Decode, Encode, TransportError, Application>
where
    Decode: Error + 'static,
    Encode: Error + 'static,
    TransportError: Error + 'static,
    Application: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IncomingFrame(error) | Self::OutgoingFrame(error) => Some(error),
            Self::DecodeClientHello(error) => Some(error),
            Self::Negotiate(error) => Some(error),
            Self::EncodeServerHello(error) => Some(error),
            Self::Transport(error) => Some(error),
            Self::Application(error) => Some(error),
            Self::HandshakeRequired { .. }
            | Self::DuplicateHandshake
            | Self::EncodedRouteMismatch { .. }
            | Self::VersionMismatch { .. } => None,
        }
    }
}

/// Enforces that a connection negotiates exactly once before application use.
///
/// The initial handshake frame's outer version is deliberately not trusted:
/// compatibility is selected from the version range inside `ClientHello`.
/// Every later frame must use the exact selected version and the server's
/// advertised inbound limits before it reaches the application handler.
pub struct HandshakeFirstHandler<Codec, Application> {
    codec: Codec,
    config: ServerHandshakeConfig,
    application: Application,
    negotiated: Option<ServerHello>,
}

impl<Codec, Application> HandshakeFirstHandler<Codec, Application> {
    pub const fn new(
        codec: Codec,
        config: ServerHandshakeConfig,
        application: Application,
    ) -> Self {
        Self {
            codec,
            config,
            application,
            negotiated: None,
        }
    }

    pub const fn negotiated(&self) -> Option<&ServerHello> {
        self.negotiated.as_ref()
    }

    pub const fn application(&self) -> &Application {
        &self.application
    }

    pub fn into_parts(
        self,
    ) -> (
        Codec,
        ServerHandshakeConfig,
        Application,
        Option<ServerHello>,
    ) {
        (self.codec, self.config, self.application, self.negotiated)
    }
}

impl<Codec, Application, Transport> ServerFrameHandler<Transport>
    for HandshakeFirstHandler<Codec, Application>
where
    Codec: WireCodec<ClientHello> + WireCodec<ServerHello>,
    Application: ServerFrameHandler<Transport>,
    Transport: FrameTransport,
{
    type Event = HandshakeFirstEvent<Application::Event>;
    type Error = HandshakeFirstError<
        <Codec as WireCodec<ClientHello>>::Error,
        <Codec as WireCodec<ServerHello>>::Error,
        Transport::Error,
        Application::Error,
    >;
    type Future<'a>
        =
        Pin<Box<dyn Future<Output = Result<ServerFrameDisposition<Self::Event>, Self::Error>> + 'a>>
    where
        Self: 'a,
        Transport: 'a;

    fn handle<'a>(&'a mut self, frame: WireFrame, transport: &'a Transport) -> Self::Future<'a> {
        Box::pin(async move {
            if let Some(hello) = self.negotiated.as_ref() {
                if frame.route() == FrameRoute::Handshake {
                    return Err(HandshakeFirstError::DuplicateHandshake);
                }
                if frame.version() != hello.selected_version() {
                    return Err(HandshakeFirstError::VersionMismatch {
                        expected: hello.selected_version(),
                        actual: frame.version(),
                    });
                }
                frame
                    .validate(hello.server_receive_limits())
                    .map_err(HandshakeFirstError::IncomingFrame)?;
                return self
                    .application
                    .handle(frame, transport)
                    .await
                    .map(|disposition| match disposition {
                        ServerFrameDisposition::Handled(event) => {
                            ServerFrameDisposition::Handled(HandshakeFirstEvent::Application(event))
                        }
                        ServerFrameDisposition::Unhandled(frame) => {
                            ServerFrameDisposition::Unhandled(frame)
                        }
                    })
                    .map_err(HandshakeFirstError::Application);
            }

            if frame.route() != FrameRoute::Handshake {
                return Err(HandshakeFirstError::HandshakeRequired {
                    actual: frame.route(),
                });
            }
            frame
                .validate(self.config.receive_limits())
                .map_err(HandshakeFirstError::IncomingFrame)?;
            let client = self
                .codec
                .decode_frame(frame)
                .map_err(HandshakeFirstError::DecodeClientHello)?;
            let hello = negotiate(&client, &self.config).map_err(HandshakeFirstError::Negotiate)?;
            let parts = self
                .codec
                .encode_parts(&hello)
                .map_err(HandshakeFirstError::EncodeServerHello)?;
            if parts.route() != FrameRoute::Handshake {
                return Err(HandshakeFirstError::EncodedRouteMismatch {
                    actual: parts.route(),
                });
            }
            let response = WireFrame::new(
                hello.selected_version(),
                FrameRoute::Handshake,
                parts.payload().to_vec(),
                parts.attachments().to_vec(),
                hello.client_receive_limits(),
            )
            .map_err(HandshakeFirstError::OutgoingFrame)?;
            transport
                .send(response)
                .await
                .map_err(HandshakeFirstError::Transport)?;
            let event = HandshakeEstablished {
                selected_version: hello.selected_version(),
                enabled_capabilities: hello.enabled_capabilities().len(),
            };
            self.negotiated = Some(hello);
            Ok(ServerFrameDisposition::Handled(
                HandshakeFirstEvent::Established(event),
            ))
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandHandled {
    call_id: CallId,
    object_id: ObjectId,
    method: MethodTag,
}

impl CommandHandled {
    pub const fn call_id(self) -> CallId {
        self.call_id
    }

    pub const fn object_id(self) -> ObjectId {
        self.object_id
    }

    pub const fn method(self) -> MethodTag {
        self.method
    }
}

#[derive(Debug)]
pub enum TypedCommandHandlerError<Decode, Encode, Service, Transport> {
    Decode(WireProtocolError<Decode>),
    DecodedRouteMismatch {
        outer_call: CallId,
        outer_target: ObjectId,
        decoded_call: CallId,
        decoded_target: ObjectId,
    },
    Service(Service),
    ResponseCallMismatch {
        expected: CallId,
        actual: CallId,
    },
    Encode(WireProtocolError<Encode>),
    EncodedRouteMismatch {
        expected: FrameRoute,
        actual: FrameRoute,
    },
    Transport(Transport),
}

impl<Decode, Encode, ServiceError, TransportError> Display
    for TypedCommandHandlerError<Decode, Encode, ServiceError, TransportError>
where
    Decode: Display,
    Encode: Display,
    ServiceError: Display,
    TransportError: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => Display::fmt(error, formatter),
            Self::DecodedRouteMismatch {
                outer_call,
                outer_target,
                decoded_call,
                decoded_target,
            } => write!(
                formatter,
                "decoded command ({decoded_call}, {decoded_target}) does not match outer route ({outer_call}, {outer_target})"
            ),
            Self::Service(error) => Display::fmt(error, formatter),
            Self::ResponseCallMismatch { expected, actual } => write!(
                formatter,
                "service response call `{actual}` does not match request call `{expected}`"
            ),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::EncodedRouteMismatch { expected, actual } => write!(
                formatter,
                "encoded result route {actual:?} does not match expected {expected:?}"
            ),
            Self::Transport(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Decode, Encode, ServiceError, TransportError> Error
    for TypedCommandHandlerError<Decode, Encode, ServiceError, TransportError>
where
    Decode: Error + 'static,
    Encode: Error + 'static,
    ServiceError: Error + 'static,
    TransportError: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            Self::Service(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::Transport(error) => Some(error),
            Self::DecodedRouteMismatch { .. }
            | Self::ResponseCallMismatch { .. }
            | Self::EncodedRouteMismatch { .. } => None,
        }
    }
}

/// Installs one typed command and delegates its decoded envelope to a service.
pub struct TypedCommandHandler<Spec, Codec, CommandService> {
    protocol: WireProtocol<Codec>,
    service: CommandService,
    marker: PhantomData<fn() -> Spec>,
}

async fn handle_typed_command<Spec, Codec, CommandService, Transport>(
    protocol: &WireProtocol<Codec>,
    service: &CommandService,
    frame: WireFrame,
    transport: &Transport,
) -> Result<
    ServerFrameDisposition<CommandHandled>,
    TypedCommandHandlerError<
        <Codec as WireCodec<Command<Spec>>>::Error,
        <Codec as WireCodec<CommandResult<Spec>>>::Error,
        CommandService::Error,
        Transport::Error,
    >,
>
where
    Spec: CommandSpec,
    Codec: WireCodec<Command<Spec>> + WireCodec<CommandResult<Spec>>,
    CommandService: Service<Command<Spec>, Response = CommandResult<Spec>>,
    Transport: FrameTransport,
{
    let FrameRoute::Command {
        call_id: outer_call,
        target: outer_target,
        method,
    } = frame.route()
    else {
        return Ok(ServerFrameDisposition::Unhandled(frame));
    };
    if method != Spec::METHOD {
        return Ok(ServerFrameDisposition::Unhandled(frame));
    }
    let command = protocol
        .decode::<Command<Spec>>(frame)
        .map_err(TypedCommandHandlerError::Decode)?;
    if command.call_id() != outer_call || command.object_id() != outer_target {
        return Err(TypedCommandHandlerError::DecodedRouteMismatch {
            outer_call,
            outer_target,
            decoded_call: command.call_id(),
            decoded_target: command.object_id(),
        });
    }
    let result = service
        .call(command)
        .await
        .map_err(TypedCommandHandlerError::Service)?;
    if result.call_id() != outer_call {
        return Err(TypedCommandHandlerError::ResponseCallMismatch {
            expected: outer_call,
            actual: result.call_id(),
        });
    }
    let frame = protocol
        .encode(&result)
        .map_err(TypedCommandHandlerError::Encode)?;
    let expected = FrameRoute::Result {
        call_id: outer_call,
    };
    if frame.route() != expected {
        return Err(TypedCommandHandlerError::EncodedRouteMismatch {
            expected,
            actual: frame.route(),
        });
    }
    transport
        .send(frame)
        .await
        .map_err(TypedCommandHandlerError::Transport)?;
    Ok(ServerFrameDisposition::Handled(CommandHandled {
        call_id: outer_call,
        object_id: outer_target,
        method,
    }))
}

impl<Spec, Codec, CommandService> TypedCommandHandler<Spec, Codec, CommandService> {
    pub const fn new(protocol: WireProtocol<Codec>, service: CommandService) -> Self {
        Self {
            protocol,
            service,
            marker: PhantomData,
        }
    }

    pub const fn service(&self) -> &CommandService {
        &self.service
    }

    pub fn into_parts(self) -> (WireProtocol<Codec>, CommandService) {
        (self.protocol, self.service)
    }
}

impl<Spec, Codec, CommandService, Transport> ServerFrameHandler<Transport>
    for TypedCommandHandler<Spec, Codec, CommandService>
where
    Spec: CommandSpec,
    Codec: WireCodec<Command<Spec>> + WireCodec<CommandResult<Spec>>,
    CommandService: Service<Command<Spec>, Response = CommandResult<Spec>>,
    Transport: FrameTransport,
{
    type Event = CommandHandled;
    type Error = TypedCommandHandlerError<
        <Codec as WireCodec<Command<Spec>>>::Error,
        <Codec as WireCodec<CommandResult<Spec>>>::Error,
        CommandService::Error,
        Transport::Error,
    >;
    type Future<'a>
        =
        Pin<Box<dyn Future<Output = Result<ServerFrameDisposition<Self::Event>, Self::Error>> + 'a>>
    where
        Self: 'a,
        Transport: 'a;

    fn handle<'a>(&'a mut self, frame: WireFrame, transport: &'a Transport) -> Self::Future<'a> {
        Box::pin(handle_typed_command::<Spec, _, _, _>(
            &self.protocol,
            &self.service,
            frame,
            transport,
        ))
    }
}

/// Produces owned command work that a bounded connection supervisor may poll
/// concurrently without borrowing the router. Implementations start no task;
/// the returned future runs only when its caller polls it.
pub trait ConcurrentServerFrameHandler<Transport>: Send + Sync {
    type Event;
    type Error;
    type Future: Future<Output = Result<ServerFrameDisposition<Self::Event>, Self::Error>>
        + Send
        + 'static;

    fn handle_concurrently(&self, frame: WireFrame, transport: Arc<Transport>) -> Self::Future;
}

/// Owned counterpart to `TypedCommandHandler` for bounded concurrent servers.
pub struct ConcurrentTypedCommandHandler<Spec, Codec, CommandService> {
    protocol: Arc<WireProtocol<Codec>>,
    service: Arc<CommandService>,
    marker: PhantomData<fn() -> Spec>,
}

impl<Spec, Codec, CommandService> ConcurrentTypedCommandHandler<Spec, Codec, CommandService> {
    pub fn new(protocol: WireProtocol<Codec>, service: CommandService) -> Self {
        Self::from_shared(Arc::new(protocol), Arc::new(service))
    }

    pub const fn from_shared(
        protocol: Arc<WireProtocol<Codec>>,
        service: Arc<CommandService>,
    ) -> Self {
        Self {
            protocol,
            service,
            marker: PhantomData,
        }
    }

    pub fn protocol(&self) -> &WireProtocol<Codec> {
        &self.protocol
    }

    pub fn service(&self) -> &CommandService {
        &self.service
    }

    pub fn shared_service(&self) -> Arc<CommandService> {
        Arc::clone(&self.service)
    }

    pub fn into_parts(self) -> (Arc<WireProtocol<Codec>>, Arc<CommandService>) {
        (self.protocol, self.service)
    }
}

impl<Spec, Codec, CommandService, Transport> ConcurrentServerFrameHandler<Transport>
    for ConcurrentTypedCommandHandler<Spec, Codec, CommandService>
where
    Spec: CommandSpec + Send + 'static,
    Spec::Params: Send,
    Spec::Result: Send,
    Spec::ErrorDetails: Send,
    Codec: WireCodec<Command<Spec>> + WireCodec<CommandResult<Spec>> + 'static,
    CommandService: Service<Command<Spec>, Response = CommandResult<Spec>> + 'static,
    CommandService::Error: Send + 'static,
    <Codec as WireCodec<Command<Spec>>>::Error: Send + 'static,
    <Codec as WireCodec<CommandResult<Spec>>>::Error: Send + 'static,
    Transport: FrameTransport + 'static,
    Transport::Error: Send + 'static,
{
    type Event = CommandHandled;
    type Error = TypedCommandHandlerError<
        <Codec as WireCodec<Command<Spec>>>::Error,
        <Codec as WireCodec<CommandResult<Spec>>>::Error,
        CommandService::Error,
        Transport::Error,
    >;
    type Future = Pin<
        Box<
            dyn Future<Output = Result<ServerFrameDisposition<Self::Event>, Self::Error>>
                + Send
                + 'static,
        >,
    >;

    fn handle_concurrently(&self, frame: WireFrame, transport: Arc<Transport>) -> Self::Future {
        let protocol = Arc::clone(&self.protocol);
        let service = Arc::clone(&self.service);
        Box::pin(async move {
            handle_typed_command::<Spec, _, _, _>(
                protocol.as_ref(),
                service.as_ref(),
                frame,
                transport.as_ref(),
            )
            .await
        })
    }
}

pub struct ConcurrentServerHandlerChain<Left, Right> {
    left: Arc<Left>,
    right: Arc<Right>,
}

impl<Left, Right> ConcurrentServerHandlerChain<Left, Right> {
    pub fn new(left: Left, right: Right) -> Self {
        Self {
            left: Arc::new(left),
            right: Arc::new(right),
        }
    }

    pub fn into_parts(self) -> (Arc<Left>, Arc<Right>) {
        (self.left, self.right)
    }
}

impl<Transport, Left, Right> ConcurrentServerFrameHandler<Transport>
    for ConcurrentServerHandlerChain<Left, Right>
where
    Transport: Send + Sync + 'static,
    Left: ConcurrentServerFrameHandler<Transport> + 'static,
    Right: ConcurrentServerFrameHandler<Transport> + 'static,
    Left::Event: Send + 'static,
    Left::Error: Send + 'static,
    Right::Event: Send + 'static,
    Right::Error: Send + 'static,
{
    type Event = Either<Left::Event, Right::Event>;
    type Error = ServerHandlerChainError<Left::Error, Right::Error>;
    type Future = Pin<
        Box<
            dyn Future<Output = Result<ServerFrameDisposition<Self::Event>, Self::Error>>
                + Send
                + 'static,
        >,
    >;

    fn handle_concurrently(&self, frame: WireFrame, transport: Arc<Transport>) -> Self::Future {
        let left = Arc::clone(&self.left);
        let right = Arc::clone(&self.right);
        Box::pin(async move {
            match left
                .handle_concurrently(frame, Arc::clone(&transport))
                .await
                .map_err(ServerHandlerChainError::Left)?
            {
                ServerFrameDisposition::Handled(event) => {
                    Ok(ServerFrameDisposition::Handled(Either::Left(event)))
                }
                ServerFrameDisposition::Unhandled(frame) => right
                    .handle_concurrently(frame, transport)
                    .await
                    .map(|disposition| match disposition {
                        ServerFrameDisposition::Handled(event) => {
                            ServerFrameDisposition::Handled(Either::Right(event))
                        }
                        ServerFrameDisposition::Unhandled(frame) => {
                            ServerFrameDisposition::Unhandled(frame)
                        }
                    })
                    .map_err(ServerHandlerChainError::Right),
            }
        })
    }
}

pub trait ConcurrentServerFrameHandlerExt<Transport>:
    ConcurrentServerFrameHandler<Transport> + Sized
{
    fn or_else_concurrently<Next>(self, next: Next) -> ConcurrentServerHandlerChain<Self, Next>
    where
        Next: ConcurrentServerFrameHandler<Transport>,
    {
        ConcurrentServerHandlerChain::new(self, next)
    }
}

impl<Transport, Handler> ConcurrentServerFrameHandlerExt<Transport> for Handler where
    Handler: ConcurrentServerFrameHandler<Transport>
{
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerCapabilityInventoryError {
    DescriptorConflict {
        capability: CapabilityId,
    },
    CommandNotDeclared {
        capability: CapabilityId,
        command: CommandId,
    },
    CommandTargetsMissing {
        command: CommandId,
    },
    CommandTargetKindUnavailable {
        command: CommandId,
        target: ObjectKindId,
    },
    EventNotDeclared {
        capability: CapabilityId,
        event: EventId,
    },
    EventTargetsMissing {
        event: EventId,
    },
    EventTargetKindUnavailable {
        event: EventId,
        target: ObjectKindId,
    },
    DuplicateEventHandler {
        event: EventId,
    },
    DuplicateEventTagHandler {
        tag: EventTag,
        existing: EventId,
        requested: EventId,
    },
    DuplicateCommandHandler {
        command: CommandId,
    },
    DuplicateMethodHandler {
        method: MethodTag,
        existing: CommandId,
        requested: CommandId,
    },
    IncompleteCapability {
        capability: CapabilityId,
        missing: Vec<CommandId>,
    },
    IncompleteCapabilityEvents {
        capability: CapabilityId,
        missing: Vec<EventId>,
    },
    AllocationFailed {
        entries: usize,
    },
    Registry(RegistryError),
    InvalidWireCapabilityId {
        capability: CapabilityId,
        error: CapabilityIdError,
    },
    Manifest(CapabilitySetError),
    ObjectKindCatalog(ObjectKindCatalogError),
}

impl Display for ServerCapabilityInventoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DescriptorConflict { capability } => write!(
                formatter,
                "installed handlers disagree on descriptor for capability `{capability}`"
            ),
            Self::CommandNotDeclared {
                capability,
                command,
            } => write!(
                formatter,
                "handler command `{command}` is not declared by capability `{capability}`"
            ),
            Self::CommandTargetsMissing { command } => {
                write!(
                    formatter,
                    "handler command `{command}` declares no target object kinds"
                )
            }
            Self::CommandTargetKindUnavailable { command, target } => write!(
                formatter,
                "handler command `{command}` targets unavailable object kind `{target}`"
            ),
            Self::EventNotDeclared { capability, event } => write!(
                formatter,
                "handler event `{event}` is not declared by capability `{capability}`"
            ),
            Self::EventTargetsMissing { event } => {
                write!(
                    formatter,
                    "handler event `{event}` declares no target object kinds"
                )
            }
            Self::EventTargetKindUnavailable { event, target } => write!(
                formatter,
                "handler event `{event}` targets unavailable object kind `{target}`"
            ),
            Self::DuplicateEventHandler { event } => {
                write!(formatter, "event `{event}` has more than one handler")
            }
            Self::DuplicateEventTagHandler {
                tag,
                existing,
                requested,
            } => write!(
                formatter,
                "wire event tag {tag} is handled by both `{existing}` and `{requested}`"
            ),
            Self::DuplicateCommandHandler { command } => {
                write!(formatter, "command `{command}` has more than one handler")
            }
            Self::DuplicateMethodHandler {
                method,
                existing,
                requested,
            } => write!(
                formatter,
                "wire method {method} is handled by both `{existing}` and `{requested}`"
            ),
            Self::IncompleteCapability {
                capability,
                missing,
            } => {
                write!(
                    formatter,
                    "capability `{capability}` is missing handlers for"
                )?;
                for command in missing {
                    write!(formatter, " `{command}`")?;
                }
                Ok(())
            }
            Self::IncompleteCapabilityEvents {
                capability,
                missing,
            } => {
                write!(
                    formatter,
                    "capability `{capability}` is missing event handlers for"
                )?;
                for event in missing {
                    write!(formatter, " `{event}`")?;
                }
                Ok(())
            }
            Self::AllocationFailed { entries } => write!(
                formatter,
                "could not allocate manifest storage for {entries} capabilities"
            ),
            Self::Registry(error) => Display::fmt(error, formatter),
            Self::InvalidWireCapabilityId { error, .. } => Display::fmt(error, formatter),
            Self::Manifest(error) => Display::fmt(error, formatter),
            Self::ObjectKindCatalog(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ServerCapabilityInventoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Registry(error) => Some(error),
            Self::InvalidWireCapabilityId { error, .. } => Some(error),
            Self::Manifest(error) => Some(error),
            Self::ObjectKindCatalog(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
struct InstalledCapabilityHandlers {
    descriptor: CapabilityDescriptor,
    commands: BTreeSet<CommandId>,
    events: BTreeSet<EventId>,
}

/// Validates that the negotiated manifest corresponds exactly to a statically
/// composed command-handler chain.
#[derive(Clone, Debug, Default)]
pub struct ServerCapabilityInventory {
    object_kinds: Option<ObjectKindCatalog>,
    capabilities: BTreeMap<CapabilityId, InstalledCapabilityHandlers>,
    commands: BTreeMap<CommandId, CapabilityCommandDescriptor>,
    methods: BTreeMap<MethodTag, CommandId>,
    events: BTreeMap<EventId, CapabilityEventDescriptor>,
    event_tags: BTreeMap<EventTag, EventId>,
}

impl ServerCapabilityInventory {
    pub const fn new() -> Self {
        Self {
            object_kinds: None,
            capabilities: BTreeMap::new(),
            commands: BTreeMap::new(),
            methods: BTreeMap::new(),
            events: BTreeMap::new(),
            event_tags: BTreeMap::new(),
        }
    }

    pub fn with_object_kinds(
        object_kinds: ObjectKindCatalog,
    ) -> Result<Self, ServerCapabilityInventoryError> {
        object_kinds
            .validate()
            .map_err(ServerCapabilityInventoryError::ObjectKindCatalog)?;
        let mut inventory = Self::new();
        inventory.object_kinds = Some(object_kinds);
        Ok(inventory)
    }

    pub fn register<Spec>(&mut self) -> Result<(), ServerCapabilityInventoryError>
    where
        Spec: CapabilityCommandSpec,
    {
        let descriptor = Spec::Capability::DESCRIPTOR;
        let capability = descriptor.plugin.id;
        if !descriptor.commands.contains(&Spec::COMMAND) {
            return Err(ServerCapabilityInventoryError::CommandNotDeclared {
                capability,
                command: Spec::COMMAND,
            });
        }
        if Spec::TARGET_OBJECT_KINDS.is_empty() {
            return Err(ServerCapabilityInventoryError::CommandTargetsMissing {
                command: Spec::COMMAND,
            });
        }
        if let Some(object_kinds) = self.object_kinds {
            for target in Spec::TARGET_OBJECT_KINDS {
                if object_kinds.by_id(*target).is_none() {
                    return Err(
                        ServerCapabilityInventoryError::CommandTargetKindUnavailable {
                            command: Spec::COMMAND,
                            target: *target,
                        },
                    );
                }
            }
        }
        if let Some(installed) = self.capabilities.get(&capability)
            && installed.descriptor != descriptor
        {
            return Err(ServerCapabilityInventoryError::DescriptorConflict { capability });
        }
        if self.commands.contains_key(&Spec::COMMAND) {
            return Err(ServerCapabilityInventoryError::DuplicateCommandHandler {
                command: Spec::COMMAND,
            });
        }
        if let Some(existing) = self.methods.get(&Spec::METHOD) {
            return Err(ServerCapabilityInventoryError::DuplicateMethodHandler {
                method: Spec::METHOD,
                existing: *existing,
                requested: Spec::COMMAND,
            });
        }

        self.commands.insert(
            Spec::COMMAND,
            CapabilityCommandDescriptor::new(
                Spec::COMMAND,
                Spec::METHOD,
                Spec::TARGET_OBJECT_KINDS,
            ),
        );
        self.methods.insert(Spec::METHOD, Spec::COMMAND);
        self.capabilities
            .entry(capability)
            .or_insert_with(|| InstalledCapabilityHandlers {
                descriptor,
                commands: BTreeSet::new(),
                events: BTreeSet::new(),
            })
            .commands
            .insert(Spec::COMMAND);
        Ok(())
    }

    pub fn register_event<Spec>(&mut self) -> Result<(), ServerCapabilityInventoryError>
    where
        Spec: CapabilityEventSpec,
    {
        let descriptor = Spec::Capability::DESCRIPTOR;
        let capability = descriptor.plugin.id;
        if !descriptor.events.contains(&Spec::EVENT_ID) {
            return Err(ServerCapabilityInventoryError::EventNotDeclared {
                capability,
                event: Spec::EVENT_ID,
            });
        }
        if Spec::TARGET_OBJECT_KINDS.is_empty() {
            return Err(ServerCapabilityInventoryError::EventTargetsMissing {
                event: Spec::EVENT_ID,
            });
        }
        if let Some(object_kinds) = self.object_kinds {
            for target in Spec::TARGET_OBJECT_KINDS {
                if object_kinds.by_id(*target).is_none() {
                    return Err(ServerCapabilityInventoryError::EventTargetKindUnavailable {
                        event: Spec::EVENT_ID,
                        target: *target,
                    });
                }
            }
        }
        if let Some(installed) = self.capabilities.get(&capability)
            && installed.descriptor != descriptor
        {
            return Err(ServerCapabilityInventoryError::DescriptorConflict { capability });
        }
        if self.events.contains_key(&Spec::EVENT_ID) {
            return Err(ServerCapabilityInventoryError::DuplicateEventHandler {
                event: Spec::EVENT_ID,
            });
        }
        if let Some(existing) = self.event_tags.get(&Spec::EVENT) {
            return Err(ServerCapabilityInventoryError::DuplicateEventTagHandler {
                tag: Spec::EVENT,
                existing: *existing,
                requested: Spec::EVENT_ID,
            });
        }
        self.events.insert(
            Spec::EVENT_ID,
            CapabilityEventDescriptor::new(Spec::EVENT_ID, Spec::EVENT, Spec::TARGET_OBJECT_KINDS),
        );
        self.event_tags.insert(Spec::EVENT, Spec::EVENT_ID);
        self.capabilities
            .entry(capability)
            .or_insert_with(|| InstalledCapabilityHandlers {
                descriptor,
                commands: BTreeSet::new(),
                events: BTreeSet::new(),
            })
            .events
            .insert(Spec::EVENT_ID);
        Ok(())
    }

    pub fn command(&self, command: CommandId) -> Option<CapabilityCommandDescriptor> {
        self.commands.get(&command).copied()
    }

    pub fn command_for_method(&self, method: MethodTag) -> Option<CapabilityCommandDescriptor> {
        self.methods
            .get(&method)
            .and_then(|command| self.command(*command))
    }

    pub fn event(&self, event: EventId) -> Option<CapabilityEventDescriptor> {
        self.events.get(&event).copied()
    }

    pub fn event_for_tag(&self, tag: EventTag) -> Option<CapabilityEventDescriptor> {
        self.event_tags
            .get(&tag)
            .and_then(|event| self.event(*event))
    }

    pub fn manifest(
        &self,
        limits: NegotiationLimits,
    ) -> Result<CapabilityManifest, ServerCapabilityInventoryError> {
        if self.capabilities.len() > limits.capabilities {
            return Err(ServerCapabilityInventoryError::Manifest(
                CapabilitySetError::TooManyCapabilities {
                    actual: self.capabilities.len(),
                    limit: limits.capabilities,
                },
            ));
        }
        let mut registry = RegistryBuilder::new();
        let mut support = Vec::new();
        support
            .try_reserve_exact(self.capabilities.len())
            .map_err(|_| ServerCapabilityInventoryError::AllocationFailed {
                entries: self.capabilities.len(),
            })?;

        for installed in self.capabilities.values() {
            registry
                .register_descriptor(installed.descriptor)
                .map_err(ServerCapabilityInventoryError::Registry)?;
            let missing = installed
                .descriptor
                .commands
                .iter()
                .filter(|command| !installed.commands.contains(command))
                .copied()
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                return Err(ServerCapabilityInventoryError::IncompleteCapability {
                    capability: installed.descriptor.plugin.id,
                    missing,
                });
            }
            let missing_events = installed
                .descriptor
                .events
                .iter()
                .filter(|event| !installed.events.contains(event))
                .copied()
                .collect::<Vec<_>>();
            if !missing_events.is_empty() {
                return Err(ServerCapabilityInventoryError::IncompleteCapabilityEvents {
                    capability: installed.descriptor.plugin.id,
                    missing: missing_events,
                });
            }
            let id = WireCapabilityId::new(
                installed.descriptor.plugin.id.as_str(),
                limits.capability_id_bytes,
            )
            .map_err(|error| {
                ServerCapabilityInventoryError::InvalidWireCapabilityId {
                    capability: installed.descriptor.plugin.id,
                    error,
                }
            })?;
            support.push(CapabilitySupport::new(
                id,
                VersionPolicy::compatible_through(installed.descriptor.plugin.version),
            ));
        }
        registry
            .build()
            .map_err(ServerCapabilityInventoryError::Registry)?;
        CapabilityManifest::from_support(support, limits)
            .map_err(ServerCapabilityInventoryError::Manifest)
    }
}

/// Supplies the capability metadata represented by an installed handler or
/// static handler chain.
pub trait ServerCapabilitySource {
    fn register_capabilities(
        &self,
        inventory: &mut ServerCapabilityInventory,
    ) -> Result<(), ServerCapabilityInventoryError>;

    fn capability_manifest(
        &self,
        limits: NegotiationLimits,
    ) -> Result<CapabilityManifest, ServerCapabilityInventoryError> {
        let mut inventory = ServerCapabilityInventory::new();
        self.register_capabilities(&mut inventory)?;
        inventory.manifest(limits)
    }

    fn capability_manifest_with_object_kinds(
        &self,
        object_kinds: ObjectKindCatalog,
        limits: NegotiationLimits,
    ) -> Result<CapabilityManifest, ServerCapabilityInventoryError> {
        let mut inventory = ServerCapabilityInventory::with_object_kinds(object_kinds)?;
        self.register_capabilities(&mut inventory)?;
        inventory.manifest(limits)
    }

    fn handshake_config(
        &self,
        versions: VersionPolicy,
        receive_limits: FrameLimits,
        negotiation_limits: NegotiationLimits,
    ) -> Result<ServerHandshakeConfig, ServerCapabilityInventoryError> {
        Ok(ServerHandshakeConfig::new(
            versions,
            receive_limits,
            self.capability_manifest(negotiation_limits)?,
        ))
    }

    fn handshake_config_with_object_kinds(
        &self,
        versions: VersionPolicy,
        receive_limits: FrameLimits,
        negotiation_limits: NegotiationLimits,
        object_kinds: ObjectKindCatalog,
    ) -> Result<ServerHandshakeConfig, ServerCapabilityInventoryError> {
        Ok(ServerHandshakeConfig::new(
            versions,
            receive_limits,
            self.capability_manifest_with_object_kinds(object_kinds, negotiation_limits)?,
        ))
    }
}

impl<Spec, Codec, CommandService> ServerCapabilitySource
    for TypedCommandHandler<Spec, Codec, CommandService>
where
    Spec: CapabilityCommandSpec,
{
    fn register_capabilities(
        &self,
        inventory: &mut ServerCapabilityInventory,
    ) -> Result<(), ServerCapabilityInventoryError> {
        inventory.register::<Spec>()
    }
}

impl<Spec, Codec, CommandService> ServerCapabilitySource
    for ConcurrentTypedCommandHandler<Spec, Codec, CommandService>
where
    Spec: CapabilityCommandSpec,
{
    fn register_capabilities(
        &self,
        inventory: &mut ServerCapabilityInventory,
    ) -> Result<(), ServerCapabilityInventoryError> {
        inventory.register::<Spec>()
    }
}

impl<Left, Right> ServerCapabilitySource for ServerHandlerChain<Left, Right>
where
    Left: ServerCapabilitySource,
    Right: ServerCapabilitySource,
{
    fn register_capabilities(
        &self,
        inventory: &mut ServerCapabilityInventory,
    ) -> Result<(), ServerCapabilityInventoryError> {
        self.left.register_capabilities(inventory)?;
        self.right.register_capabilities(inventory)
    }
}

impl<Left, Right> ServerCapabilitySource for ConcurrentServerHandlerChain<Left, Right>
where
    Left: ServerCapabilitySource,
    Right: ServerCapabilitySource,
{
    fn register_capabilities(
        &self,
        inventory: &mut ServerCapabilityInventory,
    ) -> Result<(), ServerCapabilityInventoryError> {
        self.left.register_capabilities(inventory)?;
        self.right.register_capabilities(inventory)
    }
}

/// Handles one control frame as an ordered barrier in the receive stream.
/// The supervisor awaits this bounded operation before receiving a later
/// frame, so a command following subscription activation cannot overtake it.
pub trait ServerControlFrameHandler {
    type Event;
    type Error;
    type Future<'a>: Future<Output = Result<ServerFrameDisposition<Self::Event>, Self::Error>>
    where
        Self: 'a;
    type CloseFuture<'a>: Future<Output = Result<Option<Self::Event>, Self::Error>>
    where
        Self: 'a;

    fn handle_control<'a>(&'a mut self, frame: WireFrame) -> Self::Future<'a>;

    /// Declares that after the control future's first pending poll it no
    /// longer owns the input frame. The supervisor may then retain one next
    /// frame while observing EOF without exceeding a one-frame inbound
    /// transport profile. The conservative default disables prefetch.
    fn releases_frame_before_pending(&self) -> bool {
        false
    }

    /// Reports a completed, handler-owned connection close after a control
    /// barrier returns. The supervisor observes the barrier event first and
    /// then terminates without waiting for a later transport EOF.
    fn terminal_close_reason(&self) -> Option<ServerSupervisorCloseReason> {
        None
    }

    /// Applies the transport-owned close transition before active commands are
    /// either drained or dropped according to the supervisor EOF policy. The
    /// supervisor awaits this ordered barrier before completing EOF handling.
    fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a>;

    /// Applies the connection-owned termination transition for any terminal
    /// supervisor outcome. The default gives existing handlers the same
    /// cleanup semantics as transport EOF while allowing adapters to inspect
    /// the exact reason when their backend needs reason-specific shutdown.
    fn handle_connection_terminated<'a>(
        &'a mut self,
        _reason: ServerSupervisorCloseReason,
    ) -> Self::CloseFuture<'a> {
        self.handle_transport_closed()
    }
}

/// Explicitly omits the optional server control plane.
///
/// The handler owns no state, recognizes no control frame, and has no
/// connection-cleanup work. It lets a composition leave control uninstalled
/// without inventing an application-local placeholder implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoServerControl;

impl ServerControlFrameHandler for NoServerControl {
    type Event = std::convert::Infallible;
    type Error = std::convert::Infallible;
    type Future<'a> = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>;
    type CloseFuture<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

    fn handle_control<'a>(&'a mut self, frame: WireFrame) -> Self::Future<'a> {
        ready(Ok(ServerFrameDisposition::Unhandled(frame)))
    }

    fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
        ready(Ok(None))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GracefulCloseControlEvent<Event> {
    Inner(Event),
    CloseAcknowledged { cleanup: Option<Event> },
}

#[derive(Debug)]
pub enum GracefulCloseControlError<Decode, Control, Transport> {
    Protocol(WireProtocolError<Decode>),
    StateMachine(ServerCloseError),
    Control(Control),
    EncodedRouteMismatch { actual: FrameRoute },
    Send(Transport),
    Shutdown(Transport),
}

impl<Decode, Control, Transport> Display for GracefulCloseControlError<Decode, Control, Transport>
where
    Decode: Display,
    Control: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => Display::fmt(error, formatter),
            Self::StateMachine(error) => Display::fmt(error, formatter),
            Self::Control(error) => write!(formatter, "graceful-close cleanup failed: {error}"),
            Self::EncodedRouteMismatch { actual } => write!(
                formatter,
                "encoded CloseAck has route {actual:?}, expected Control"
            ),
            Self::Send(error) => write!(formatter, "failed to send CloseAck: {error}"),
            Self::Shutdown(error) => write!(formatter, "failed to shut down write half: {error}"),
        }
    }
}

impl<Decode, Control, Transport> Error for GracefulCloseControlError<Decode, Control, Transport>
where
    Decode: Error + 'static,
    Control: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Protocol(error) => Some(error),
            Self::StateMachine(error) => Some(error),
            Self::Control(error) => Some(error),
            Self::Send(error) | Self::Shutdown(error) => Some(error),
            Self::EncodedRouteMismatch { .. } => None,
        }
    }
}

/// Decorates an existing control handler with the negotiated server side of
/// the graceful-close exchange.
///
/// Normal control frames remain owned by `inner`. A `Close` frame becomes an
/// ordered barrier: connection cleanup completes, `CloseAck` is sent, and the
/// transport write half is shut down. No task or executor is created.
pub struct GracefulCloseControlHandler<Transport, Codec, Inner> {
    transport: Arc<Transport>,
    protocol: WireProtocol<Codec>,
    inner: Inner,
    close: ServerCloseMachine,
    termination_completed: bool,
}

impl<Transport, Codec, Inner> GracefulCloseControlHandler<Transport, Codec, Inner> {
    pub const fn new(
        transport: Arc<Transport>,
        protocol: WireProtocol<Codec>,
        inner: Inner,
    ) -> Self {
        Self {
            transport,
            protocol,
            inner,
            close: ServerCloseMachine::new(),
            termination_completed: false,
        }
    }

    pub const fn close_machine(&self) -> ServerCloseMachine {
        self.close
    }

    pub const fn inner(&self) -> &Inner {
        &self.inner
    }

    pub fn into_parts(self) -> (Arc<Transport>, WireProtocol<Codec>, Inner) {
        (self.transport, self.protocol, self.inner)
    }
}

impl<Transport, Codec, Inner> ServerControlFrameHandler
    for GracefulCloseControlHandler<Transport, Codec, Inner>
where
    Transport: ShutdownFrameTransport,
    Codec: WireCodec<ControlMessage>,
    Inner: ServerControlFrameHandler,
{
    type Event = GracefulCloseControlEvent<Inner::Event>;
    type Error = GracefulCloseControlError<Codec::Error, Inner::Error, Transport::Error>;
    type Future<'a>
        =
        Pin<Box<dyn Future<Output = Result<ServerFrameDisposition<Self::Event>, Self::Error>> + 'a>>
    where
        Self: 'a;
    type CloseFuture<'a>
        = Pin<Box<dyn Future<Output = Result<Option<Self::Event>, Self::Error>> + 'a>>
    where
        Self: 'a;

    fn handle_control<'a>(&'a mut self, frame: WireFrame) -> Self::Future<'a> {
        Box::pin(async move {
            let message = self
                .protocol
                .decode::<ControlMessage>(frame.clone())
                .map_err(GracefulCloseControlError::Protocol)?;
            if message != ControlMessage::Close {
                return self
                    .inner
                    .handle_control(frame)
                    .await
                    .map(|disposition| match disposition {
                        ServerFrameDisposition::Handled(event) => {
                            ServerFrameDisposition::Handled(GracefulCloseControlEvent::Inner(event))
                        }
                        ServerFrameDisposition::Unhandled(frame) => {
                            ServerFrameDisposition::Unhandled(frame)
                        }
                    })
                    .map_err(GracefulCloseControlError::Control);
            }

            self.close
                .receive(message)
                .map_err(GracefulCloseControlError::StateMachine)?;
            let cleanup = self
                .inner
                .handle_connection_terminated(ServerSupervisorCloseReason::PeerGracefulClose)
                .await;
            self.termination_completed = true;
            let cleanup = match cleanup {
                Ok(event) => event,
                Err(error) => {
                    self.close.terminate();
                    return Err(GracefulCloseControlError::Control(error));
                }
            };
            if let Err(error) = self.close.complete(ServerCloseAction::Cleanup) {
                self.close.terminate();
                return Err(GracefulCloseControlError::StateMachine(error));
            }
            let ack = match self.protocol.encode(&ControlMessage::CloseAck) {
                Ok(ack) => ack,
                Err(error) => {
                    self.close.terminate();
                    return Err(GracefulCloseControlError::Protocol(error));
                }
            };
            if ack.route() != FrameRoute::Control {
                self.close.terminate();
                return Err(GracefulCloseControlError::EncodedRouteMismatch {
                    actual: ack.route(),
                });
            }
            // A pending transport send owns the partial Ack. Advance before
            // awaiting it so cancelling the surrounding run cannot enqueue a
            // duplicate acknowledgement on reuse.
            if let Err(error) = self.close.complete(ServerCloseAction::SendCloseAck) {
                self.close.terminate();
                return Err(GracefulCloseControlError::StateMachine(error));
            }
            if let Err(error) = self.transport.send(ack).await {
                self.close.terminate();
                return Err(GracefulCloseControlError::Send(error));
            }
            if let Err(error) = self.transport.shutdown().await {
                self.close.terminate();
                return Err(GracefulCloseControlError::Shutdown(error));
            }
            if let Err(error) = self.close.complete(ServerCloseAction::ShutdownWrite) {
                self.close.terminate();
                return Err(GracefulCloseControlError::StateMachine(error));
            }
            Ok(ServerFrameDisposition::Handled(
                GracefulCloseControlEvent::CloseAcknowledged { cleanup },
            ))
        })
    }

    fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
        self.handle_connection_terminated(ServerSupervisorCloseReason::EndOfStream)
    }

    fn terminal_close_reason(&self) -> Option<ServerSupervisorCloseReason> {
        if self.close.phase() == ServerClosePhase::Closed {
            Some(ServerSupervisorCloseReason::PeerGracefulClose)
        } else {
            self.inner.terminal_close_reason()
        }
    }

    fn handle_connection_terminated<'a>(
        &'a mut self,
        reason: ServerSupervisorCloseReason,
    ) -> Self::CloseFuture<'a> {
        Box::pin(async move {
            if self.termination_completed {
                return Ok(None);
            }
            let event = self
                .inner
                .handle_connection_terminated(reason)
                .await
                .map_err(GracefulCloseControlError::Control)?;
            self.termination_completed = true;
            self.close.terminate();
            Ok(event.map(GracefulCloseControlEvent::Inner))
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ServerEofPolicy {
    #[default]
    DropActive,
    DrainActive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServerSupervisorLimits {
    in_flight_commands: usize,
    eof_policy: ServerEofPolicy,
}

/// Production declaration for command futures retained above the runtime.
///
/// Each admitted server command owns one `PendingCall` lease in its call
/// scope until its handler future completes or the connection drops it. A
/// runtime may simultaneously own its own lease for the same call, so a
/// composition root must add both requirements.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServerSupervisorProductionProfile {
    limits: ServerSupervisorLimits,
    requirements: ResourceRequirements,
}

impl ServerSupervisorProductionProfile {
    pub fn new(limits: ServerSupervisorLimits) -> Self {
        let commands = u64::try_from(limits.in_flight_commands()).unwrap_or(u64::MAX);
        Self {
            limits,
            requirements: ResourceRequirements::none()
                .with_class(
                    ResourceClass::PendingCall,
                    commands,
                    u64::from(commands != 0),
                )
                .with_metadata(limits.in_flight_commands(), limits.in_flight_commands()),
        }
    }

    pub const fn limits(self) -> ServerSupervisorLimits {
        self.limits
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ServerSupervisorProductionError {
    LimitsMismatch {
        supervisor: ServerSupervisorLimits,
        profile: ServerSupervisorLimits,
    },
    ResourcesAlreadyInstalled,
    Resources(ProductionResourceError),
}

impl Display for ServerSupervisorProductionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitsMismatch {
                supervisor,
                profile,
            } => write!(
                formatter,
                "server supervisor limits {supervisor:?} do not match production profile {profile:?}"
            ),
            Self::ResourcesAlreadyInstalled => {
                formatter.write_str("server supervisor production resources are already installed")
            }
            Self::Resources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ServerSupervisorProductionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resources(error) => Some(error),
            Self::LimitsMismatch { .. } | Self::ResourcesAlreadyInstalled => None,
        }
    }
}

impl From<ProductionResourceError> for ServerSupervisorProductionError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

/// Production bound for the server hello's typed source and encoded frame.
///
/// The peer advertises its receive limits inside `ClientHello`, so production
/// servers must cap that value before using it as an allocation boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServerHandshakeProductionProfile {
    maximum_peer_receive_frame_bytes: usize,
    requirements: ResourceRequirements,
}

impl ServerHandshakeProductionProfile {
    pub fn new(maximum_peer_receive_frame_bytes: usize) -> Self {
        let bytes =
            u64::try_from(maximum_peer_receive_frame_bytes.saturating_mul(2)).unwrap_or(u64::MAX);
        let requirements = if bytes == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::WireBytes, bytes, bytes)
                .with_metadata(1, 1)
        };
        Self {
            maximum_peer_receive_frame_bytes,
            requirements,
        }
    }

    pub const fn maximum_peer_receive_frame_bytes(self) -> usize {
        self.maximum_peer_receive_frame_bytes
    }

    pub const fn maximum_encoding_wire_bytes(self) -> usize {
        self.maximum_peer_receive_frame_bytes.saturating_mul(2)
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ServerHandshakeProductionError {
    ResourcesAlreadyInstalled,
    Resources(ProductionResourceError),
}

impl Display for ServerHandshakeProductionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResourcesAlreadyInstalled => {
                formatter.write_str("server handshake production resources are already installed")
            }
            Self::Resources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ServerHandshakeProductionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resources(error) => Some(error),
            Self::ResourcesAlreadyInstalled => None,
        }
    }
}

impl From<ProductionResourceError> for ServerHandshakeProductionError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

impl ServerSupervisorLimits {
    pub fn new(in_flight_commands: usize) -> Result<Self, ServerSupervisorConfigurationError> {
        if in_flight_commands == 0 {
            return Err(ServerSupervisorConfigurationError::ZeroInFlightCommands);
        }
        Ok(Self {
            in_flight_commands,
            eof_policy: ServerEofPolicy::DropActive,
        })
    }

    pub const fn in_flight_commands(self) -> usize {
        self.in_flight_commands
    }

    pub const fn with_eof_policy(mut self, eof_policy: ServerEofPolicy) -> Self {
        self.eof_policy = eof_policy;
        self
    }

    pub const fn eof_policy(self) -> ServerEofPolicy {
        self.eof_policy
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerSupervisorConfigurationError {
    ZeroInFlightCommands,
}

impl Display for ServerSupervisorConfigurationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("server supervisor in-flight command limit must be greater than zero")
    }
}

impl Error for ServerSupervisorConfigurationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerSupervisorCloseReason {
    Explicit,
    PeerGracefulClose,
    HandshakeFailed,
    HandshakeFutureDropped,
    RunFutureDropped,
    EndOfStream,
    ReceiveFailed,
    CommandFailed,
    ControlFailed,
    ObserverFailed,
    OutboundFailed,
    BackendEventFailed,
    CommandCapacityExceeded,
    CommandResourceExhausted,
    IncomingFrameRejected,
    DrainDeadlineExceeded,
}

#[derive(Debug)]
pub enum ServerTransportCloseError<Control, Transport> {
    Control(Control),
    Transport(Transport),
    ControlAndTransport {
        control: Control,
        transport: Transport,
    },
}

#[derive(Debug)]
pub enum ServerConnectionRunError<Run, Shutdown> {
    Run(Run),
    Shutdown {
        reason: ServerSupervisorCloseReason,
        error: Shutdown,
    },
    RunAndShutdown {
        reason: ServerSupervisorCloseReason,
        run: Run,
        shutdown: Shutdown,
    },
}

impl<Run, Shutdown> Display for ServerConnectionRunError<Run, Shutdown>
where
    Run: Display,
    Shutdown: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Run(error) => Display::fmt(error, formatter),
            Self::Shutdown { reason, error } => {
                write!(
                    formatter,
                    "connection shutdown for {reason:?} failed: {error}"
                )
            }
            Self::RunAndShutdown {
                reason,
                run,
                shutdown,
            } => write!(
                formatter,
                "{run}; connection shutdown for {reason:?} also failed: {shutdown}"
            ),
        }
    }
}

impl<Run, Shutdown> Error for ServerConnectionRunError<Run, Shutdown>
where
    Run: Error + 'static,
    Shutdown: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Run(error) | Self::RunAndShutdown { run: error, .. } => Some(error),
            Self::Shutdown { error, .. } => Some(error),
        }
    }
}

impl<Control, Transport> Display for ServerTransportCloseError<Control, Transport>
where
    Control: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Control(error) => write!(formatter, "server cleanup failed: {error}"),
            Self::Transport(error) => write!(formatter, "transport shutdown failed: {error}"),
            Self::ControlAndTransport { control, transport } => write!(
                formatter,
                "server cleanup failed: {control}; transport shutdown also failed: {transport}"
            ),
        }
    }
}

impl<Control, Transport> Error for ServerTransportCloseError<Control, Transport>
where
    Control: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Control(error) | Self::ControlAndTransport { control: error, .. } => Some(error),
            Self::Transport(error) => Some(error),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerSupervisorPhase {
    AwaitingHandshake,
    Application,
    Closed,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ApplicationFrameError {
    DuplicateHandshake,
    VersionMismatch {
        expected: ProtocolVersion,
        actual: ProtocolVersion,
    },
    Frame(FrameError),
}

impl Display for ApplicationFrameError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateHandshake => {
                formatter.write_str("received a second handshake after negotiation")
            }
            Self::VersionMismatch { expected, actual } => write!(
                formatter,
                "application frame uses protocol version {actual}, expected {expected}"
            ),
            Self::Frame(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ApplicationFrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Frame(error) => Some(error),
            Self::DuplicateHandshake | Self::VersionMismatch { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServerSupervisorMetrics {
    active_commands: usize,
    peak_active_commands: usize,
    accepted_commands: u64,
    completed_commands: u64,
    handled_controls: u64,
    outbound_messages: u64,
    unhandled_frames: u64,
    capacity_failures: u64,
    dropped_commands: u64,
    drain_deadline_expirations: u64,
    limit: usize,
    close_reason: Option<ServerSupervisorCloseReason>,
}

impl ServerSupervisorMetrics {
    pub const fn active_commands(self) -> usize {
        self.active_commands
    }

    pub const fn peak_active_commands(self) -> usize {
        self.peak_active_commands
    }

    pub const fn accepted_commands(self) -> u64 {
        self.accepted_commands
    }

    pub const fn completed_commands(self) -> u64 {
        self.completed_commands
    }

    pub const fn handled_controls(self) -> u64 {
        self.handled_controls
    }

    pub const fn outbound_messages(self) -> u64 {
        self.outbound_messages
    }

    pub const fn unhandled_frames(self) -> u64 {
        self.unhandled_frames
    }

    pub const fn capacity_failures(self) -> u64 {
        self.capacity_failures
    }

    pub const fn dropped_commands(self) -> u64 {
        self.dropped_commands
    }

    pub const fn drain_deadline_expirations(self) -> u64 {
        self.drain_deadline_expirations
    }

    pub const fn limit(self) -> usize {
        self.limit
    }

    pub const fn close_reason(self) -> Option<ServerSupervisorCloseReason> {
        self.close_reason
    }
}

/// One caller-selected outbound lane polled beside receive and command work.
/// `None` means the lane is idle until another connection transition occurs.
/// A lane must report that state without remaining pending merely to wait for
/// future producer work. Once it starts an actual send it may remain pending;
/// a chained outbound will not poll another writer until that attempt finishes.
pub trait ConcurrentServerOutbound {
    type Event;
    type Error;
    type Future<'a>: Future<Output = Result<Option<Self::Event>, Self::Error>> + Send
    where
        Self: 'a;

    fn send_once(&self) -> Self::Future<'_>;

    /// Waits until an idle lane may have work. The default never wakes, so
    /// existing caller-driven sources are retried only after connection work.
    fn wait_until_ready(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(pending())
    }
}

pub trait ConcurrentServerOutboundExt: ConcurrentServerOutbound + Sized {
    /// Serially composes two outbound plugins and alternates the preferred
    /// first poll after successful sends. An idle preferred lane yields to the
    /// other lane; an active pending send retains the single-writer boundary.
    fn chain<Next>(self, next: Next) -> ChainedServerOutbound<Self, Next>
    where
        Next: ConcurrentServerOutbound,
    {
        ChainedServerOutbound::new(self, next)
    }
}

impl<Outbound> ConcurrentServerOutboundExt for Outbound where Outbound: ConcurrentServerOutbound {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChainedServerOutboundEvent<First, Second> {
    First(First),
    Second(Second),
}

#[derive(Debug)]
pub enum ChainedServerOutboundError<First, Second> {
    First(First),
    Second(Second),
}

impl<First, Second> Display for ChainedServerOutboundError<First, Second>
where
    First: Display,
    Second: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::First(error) => Display::fmt(error, formatter),
            Self::Second(error) => Display::fmt(error, formatter),
        }
    }
}

impl<First, Second> Error for ChainedServerOutboundError<First, Second>
where
    First: Error + 'static,
    Second: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::First(error) => Some(error),
            Self::Second(error) => Some(error),
        }
    }
}

pub struct ChainedServerOutbound<First, Second> {
    first: First,
    second: Second,
    prefer_second: AtomicBool,
}

impl<First, Second> ChainedServerOutbound<First, Second> {
    pub const fn new(first: First, second: Second) -> Self {
        Self {
            first,
            second,
            prefer_second: AtomicBool::new(false),
        }
    }

    pub const fn first(&self) -> &First {
        &self.first
    }

    pub const fn second(&self) -> &Second {
        &self.second
    }

    pub fn into_parts(self) -> (First, Second) {
        (self.first, self.second)
    }
}

impl<First, Second> ConcurrentServerOutbound for ChainedServerOutbound<First, Second>
where
    First: ConcurrentServerOutbound + Sync,
    Second: ConcurrentServerOutbound + Sync,
{
    type Event = ChainedServerOutboundEvent<First::Event, Second::Event>;
    type Error = ChainedServerOutboundError<First::Error, Second::Error>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Option<Self::Event>, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn send_once(&self) -> Self::Future<'_> {
        Box::pin(async move {
            let prefer_second = self.prefer_second.load(Ordering::Relaxed);
            if prefer_second {
                match self.second.send_once().await {
                    Ok(Some(event)) => {
                        self.prefer_second.store(false, Ordering::Relaxed);
                        return Ok(Some(ChainedServerOutboundEvent::Second(event)));
                    }
                    Ok(None) => {}
                    Err(error) => return Err(ChainedServerOutboundError::Second(error)),
                }
                self.first
                    .send_once()
                    .await
                    .map(|event| event.map(ChainedServerOutboundEvent::First))
                    .map_err(ChainedServerOutboundError::First)
            } else {
                match self.first.send_once().await {
                    Ok(Some(event)) => {
                        self.prefer_second.store(true, Ordering::Relaxed);
                        return Ok(Some(ChainedServerOutboundEvent::First(event)));
                    }
                    Ok(None) => {}
                    Err(error) => return Err(ChainedServerOutboundError::First(error)),
                }
                self.second
                    .send_once()
                    .await
                    .map(|event| event.map(ChainedServerOutboundEvent::Second))
                    .map_err(ChainedServerOutboundError::Second)
            }
        })
    }

    fn wait_until_ready(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            let mut first = self.first.wait_until_ready();
            let mut second = self.second.wait_until_ready();
            poll_fn(|context| {
                if first.as_mut().poll(context).is_ready()
                    || second.as_mut().poll(context).is_ready()
                {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoServerOutbound;

impl ConcurrentServerOutbound for NoServerOutbound {
    type Event = std::convert::Infallible;
    type Error = std::convert::Infallible;
    type Future<'a> = Pending<Result<Option<Self::Event>, Self::Error>>;

    fn send_once(&self) -> Self::Future<'_> {
        pending()
    }
}

/// One caller-owned backend lane polled beside transport receive, command
/// work, and outbound delivery. The implementation retains any asynchronous
/// turn internally after its first poll; the supervisor keeps polling the same
/// lane without spawning a task or creating another transport reader.
pub trait ConcurrentServerBackend {
    type Event;
    type Error;

    /// Polls one retained backend turn. Implementations must keep any pending
    /// future inside themselves so progress in another supervisor lane never
    /// cancels partially applied backend work.
    fn poll_drive_once(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<Self::Event, Self::Error>>;

    /// Releases connection-scoped backend work after a terminal supervisor
    /// transition. The default is appropriate for stateless lanes.
    fn handle_connection_terminated(&mut self, _reason: ServerSupervisorCloseReason) {}
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoServerBackend;

impl ConcurrentServerBackend for NoServerBackend {
    type Event = std::convert::Infallible;
    type Error = std::convert::Infallible;

    fn poll_drive_once(
        &mut self,
        _context: &mut Context<'_>,
    ) -> Poll<Result<Self::Event, Self::Error>> {
        Poll::Pending
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ServerBackendChainError<Left, Right> {
    Left(Left),
    Right(Right),
}

impl<Left, Right> Display for ServerBackendChainError<Left, Right>
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

impl<Left, Right> Error for ServerBackendChainError<Left, Right>
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

/// Composes two independent backend-progress sources into the supervisor's
/// single backend lane without spawning or cancelling either source.
///
/// Ready sources alternate priority. A pending source is still polled before
/// a repeatedly ready peer on its priority turn, and terminal notification is
/// delivered to both children exactly once by the supervisor's owner.
pub struct ConcurrentServerBackendChain<Left, Right> {
    left: Left,
    right: Right,
    poll_left_first: bool,
}

impl<Left, Right> ConcurrentServerBackendChain<Left, Right> {
    pub const fn new(left: Left, right: Right) -> Self {
        Self {
            left,
            right,
            poll_left_first: true,
        }
    }

    pub fn into_parts(self) -> (Left, Right) {
        (self.left, self.right)
    }
}

impl<Left, Right> ConcurrentServerBackend for ConcurrentServerBackendChain<Left, Right>
where
    Left: ConcurrentServerBackend,
    Right: ConcurrentServerBackend,
{
    type Event = Either<Left::Event, Right::Event>;
    type Error = ServerBackendChainError<Left::Error, Right::Error>;

    fn poll_drive_once(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<Self::Event, Self::Error>> {
        if self.poll_left_first {
            if let Poll::Ready(result) = self.left.poll_drive_once(context) {
                self.poll_left_first = false;
                return Poll::Ready(
                    result
                        .map(Either::Left)
                        .map_err(ServerBackendChainError::Left),
                );
            }
            if let Poll::Ready(result) = self.right.poll_drive_once(context) {
                self.poll_left_first = true;
                return Poll::Ready(
                    result
                        .map(Either::Right)
                        .map_err(ServerBackendChainError::Right),
                );
            }
        } else {
            if let Poll::Ready(result) = self.right.poll_drive_once(context) {
                self.poll_left_first = true;
                return Poll::Ready(
                    result
                        .map(Either::Right)
                        .map_err(ServerBackendChainError::Right),
                );
            }
            if let Poll::Ready(result) = self.left.poll_drive_once(context) {
                self.poll_left_first = false;
                return Poll::Ready(
                    result
                        .map(Either::Left)
                        .map_err(ServerBackendChainError::Left),
                );
            }
        }
        Poll::Pending
    }

    fn handle_connection_terminated(&mut self, reason: ServerSupervisorCloseReason) {
        self.left.handle_connection_terminated(reason);
        self.right.handle_connection_terminated(reason);
    }
}

pub trait ConcurrentServerBackendExt: ConcurrentServerBackend + Sized {
    fn and_backend<Next>(self, next: Next) -> ConcurrentServerBackendChain<Self, Next>
    where
        Next: ConcurrentServerBackend,
    {
        ConcurrentServerBackendChain::new(self, next)
    }
}

impl<Backend> ConcurrentServerBackendExt for Backend where Backend: ConcurrentServerBackend {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerSupervisorEvent<
    CommandEvent,
    ControlEvent,
    OutboundEvent = std::convert::Infallible,
    BackendEvent = std::convert::Infallible,
> {
    Command(CommandEvent),
    Control(ControlEvent),
    Outbound(OutboundEvent),
    Backend(BackendEvent),
    Unhandled(WireFrame),
}

#[derive(Debug)]
pub enum ServerSupervisorError<
    Transport,
    Command,
    Control,
    Observer,
    Outbound = std::convert::Infallible,
    Backend = std::convert::Infallible,
> {
    HandshakeRequired,
    IncomingFrame(ApplicationFrameError),
    Transport(Transport),
    Command(Command),
    Control(Control),
    Observer(Observer),
    Outbound(Outbound),
    Backend(Backend),
    CommandCapacityExceeded {
        limit: usize,
    },
    Resource(ResourceError),
    TerminationCleanup {
        reason: ServerSupervisorCloseReason,
        primary: Box<Self>,
        cleanup: Control,
    },
}

impl<Transport, Command, Control, Observer, Outbound, Backend> Display
    for ServerSupervisorError<Transport, Command, Control, Observer, Outbound, Backend>
where
    Transport: Display,
    Command: Display,
    Control: Display,
    Observer: Display,
    Outbound: Display,
    Backend: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::HandshakeRequired => {
                formatter.write_str("server handshake must complete before application run")
            }
            Self::IncomingFrame(error) => Display::fmt(error, formatter),
            Self::Transport(error) => Display::fmt(error, formatter),
            Self::Command(error) => Display::fmt(error, formatter),
            Self::Control(error) => Display::fmt(error, formatter),
            Self::Observer(error) => Display::fmt(error, formatter),
            Self::Outbound(error) => Display::fmt(error, formatter),
            Self::Backend(error) => Display::fmt(error, formatter),
            Self::CommandCapacityExceeded { limit } => {
                write!(
                    formatter,
                    "server in-flight command limit of {limit} was reached"
                )
            }
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::TerminationCleanup {
                reason,
                primary,
                cleanup,
            } => write!(
                formatter,
                "{primary}; connection cleanup for {reason:?} also failed: {cleanup}"
            ),
        }
    }
}

impl<Transport, Command, Control, Observer, Outbound, Backend> Error
    for ServerSupervisorError<Transport, Command, Control, Observer, Outbound, Backend>
where
    Transport: Error + 'static,
    Command: Error + 'static,
    Control: Error + 'static,
    Observer: Error + 'static,
    Outbound: Error + 'static,
    Backend: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IncomingFrame(error) => Some(error),
            Self::Transport(error) => Some(error),
            Self::Command(error) => Some(error),
            Self::Control(error) => Some(error),
            Self::Observer(error) => Some(error),
            Self::Outbound(error) => Some(error),
            Self::Backend(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::TerminationCleanup { primary, .. } => Some(primary.as_ref()),
            Self::HandshakeRequired | Self::CommandCapacityExceeded { .. } => None,
        }
    }
}

#[derive(Clone, Copy)]
struct NegotiatedInboundPolicy {
    version: ProtocolVersion,
    limits: pwright_transport::FrameLimits,
}

struct ProductionHandshakeResources {
    profile: ServerHandshakeProductionProfile,
    ledger: ResourceLedger,
}

/// Runs one application-phase connection with a single reader, bounded owned
/// command work, and ordered asynchronous control transitions. It starts no task or
/// runtime. Exceeding the admitted-command limit closes the logical connection
/// instead of creating an unbounded queue whose commands cannot yet be
/// cancelled in the runtime.
pub struct EncodedServerSupervisor<Transport> {
    transport: Arc<Transport>,
    limits: ServerSupervisorLimits,
    metrics: ServerSupervisorMetrics,
    phase: ServerSupervisorPhase,
    negotiated: Option<ServerHello>,
    inbound_policy: Option<NegotiatedInboundPolicy>,
    resource_ledger: Option<ResourceLedger>,
    handshake_resources: Option<ProductionHandshakeResources>,
    control_termination_completed: bool,
}

struct ActiveServerCommand<CommandFuture> {
    future: Pin<Box<CommandFuture>>,
    _resource: Option<ResourceLease>,
}

struct SupervisorRunMetricsGuard<'a> {
    metrics: &'a mut ServerSupervisorMetrics,
    terminal: bool,
}

struct SupervisorBackendGuard<'a, Backend>
where
    Backend: ConcurrentServerBackend,
{
    backend: &'a mut Backend,
    terminal: bool,
}

impl<Backend> SupervisorBackendGuard<'_, Backend>
where
    Backend: ConcurrentServerBackend,
{
    fn backend_mut(&mut self) -> &mut Backend {
        self.backend
    }

    fn finish(&mut self, reason: ServerSupervisorCloseReason) {
        self.backend.handle_connection_terminated(reason);
        self.terminal = true;
    }
}

impl<Backend> Drop for SupervisorBackendGuard<'_, Backend>
where
    Backend: ConcurrentServerBackend,
{
    fn drop(&mut self) {
        if !self.terminal {
            self.backend
                .handle_connection_terminated(ServerSupervisorCloseReason::RunFutureDropped);
        }
    }
}

struct SupervisorHandshakeGuard<'a> {
    metrics: &'a mut ServerSupervisorMetrics,
    phase: &'a mut ServerSupervisorPhase,
    terminal: bool,
}

impl SupervisorHandshakeGuard<'_> {
    fn fail(&mut self, reason: ServerSupervisorCloseReason) {
        self.metrics.close_reason = Some(reason);
        *self.phase = ServerSupervisorPhase::Closed;
        self.terminal = true;
    }

    fn succeed(&mut self) {
        *self.phase = ServerSupervisorPhase::Application;
        self.terminal = true;
    }
}

impl Drop for SupervisorHandshakeGuard<'_> {
    fn drop(&mut self) {
        if self.terminal {
            return;
        }
        self.metrics.close_reason = Some(ServerSupervisorCloseReason::HandshakeFutureDropped);
        *self.phase = ServerSupervisorPhase::Closed;
    }
}

impl SupervisorRunMetricsGuard<'_> {
    fn finish(&mut self, reason: ServerSupervisorCloseReason, dropped_commands: usize) {
        self.metrics.dropped_commands = self
            .metrics
            .dropped_commands
            .saturating_add(dropped_commands as u64);
        self.metrics.active_commands = 0;
        self.metrics.close_reason = Some(reason);
        self.terminal = true;
    }
}

impl Drop for SupervisorRunMetricsGuard<'_> {
    fn drop(&mut self) {
        if self.terminal {
            return;
        }
        self.metrics.dropped_commands = self
            .metrics
            .dropped_commands
            .saturating_add(self.metrics.active_commands as u64);
        self.metrics.active_commands = 0;
        self.metrics.close_reason = Some(ServerSupervisorCloseReason::RunFutureDropped);
    }
}

impl<Transport> EncodedServerSupervisor<Transport> {
    pub fn new(transport: Transport, limits: ServerSupervisorLimits) -> Self {
        Self::from_shared(Arc::new(transport), limits)
    }

    pub fn from_shared(transport: Arc<Transport>, limits: ServerSupervisorLimits) -> Self {
        Self::from_shared_with_phase(transport, limits, ServerSupervisorPhase::Application)
    }

    pub fn new_handshake_required(transport: Transport, limits: ServerSupervisorLimits) -> Self {
        Self::from_shared_with_phase(
            Arc::new(transport),
            limits,
            ServerSupervisorPhase::AwaitingHandshake,
        )
    }

    pub fn from_shared_handshake_required(
        transport: Arc<Transport>,
        limits: ServerSupervisorLimits,
    ) -> Self {
        Self::from_shared_with_phase(transport, limits, ServerSupervisorPhase::AwaitingHandshake)
    }

    fn from_shared_with_phase(
        transport: Arc<Transport>,
        limits: ServerSupervisorLimits,
        phase: ServerSupervisorPhase,
    ) -> Self {
        Self {
            transport,
            limits,
            metrics: ServerSupervisorMetrics {
                active_commands: 0,
                peak_active_commands: 0,
                accepted_commands: 0,
                completed_commands: 0,
                handled_controls: 0,
                outbound_messages: 0,
                unhandled_frames: 0,
                capacity_failures: 0,
                dropped_commands: 0,
                drain_deadline_expirations: 0,
                limit: limits.in_flight_commands,
                close_reason: None,
            },
            phase,
            negotiated: None,
            inbound_policy: None,
            resource_ledger: None,
            handshake_resources: None,
            control_termination_completed: false,
        }
    }

    pub const fn metrics(&self) -> ServerSupervisorMetrics {
        self.metrics
    }

    pub fn transport(&self) -> &Arc<Transport> {
        &self.transport
    }

    pub const fn phase(&self) -> ServerSupervisorPhase {
        if self.metrics.close_reason.is_some() {
            ServerSupervisorPhase::Closed
        } else {
            self.phase
        }
    }

    pub const fn negotiated(&self) -> Option<&ServerHello> {
        self.negotiated.as_ref()
    }

    pub fn with_production_resources(
        mut self,
        profile: ServerSupervisorProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ServerSupervisorProductionError> {
        if self.limits != profile.limits() {
            return Err(ServerSupervisorProductionError::LimitsMismatch {
                supervisor: self.limits,
                profile: profile.limits(),
            });
        }
        if self.resource_ledger.is_some() {
            return Err(ServerSupervisorProductionError::ResourcesAlreadyInstalled);
        }
        self.resource_ledger = Some(resources.claim(profile.requirements())?);
        Ok(self)
    }

    pub fn with_production_handshake(
        mut self,
        profile: ServerHandshakeProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ServerHandshakeProductionError> {
        if self.handshake_resources.is_some() {
            return Err(ServerHandshakeProductionError::ResourcesAlreadyInstalled);
        }
        let ledger = resources.claim(profile.requirements())?;
        self.handshake_resources = Some(ProductionHandshakeResources { profile, ledger });
        Ok(self)
    }

    pub fn close(&mut self) -> ServerSupervisorCloseReason {
        self.terminate(ServerSupervisorCloseReason::Explicit)
    }

    /// Closes the logical connection with a caller-classified terminal reason.
    /// A previously recorded reason remains authoritative.
    pub fn terminate(
        &mut self,
        reason: ServerSupervisorCloseReason,
    ) -> ServerSupervisorCloseReason {
        let reason = *self.metrics.close_reason.get_or_insert(reason);
        self.phase = ServerSupervisorPhase::Closed;
        reason
    }

    /// Closes the logical connection and awaits the control/backend cleanup
    /// barrier. Prefer this over [`Self::close`] when a control handler is
    /// available and may own backend subscriptions or runtime objects.
    pub async fn close_with_controls<Controls>(
        &mut self,
        controls: &mut Controls,
    ) -> Result<ServerSupervisorCloseReason, Controls::Error>
    where
        Controls: ServerControlFrameHandler,
    {
        self.terminate_with_controls(ServerSupervisorCloseReason::Explicit, controls)
            .await
    }

    pub async fn terminate_with_controls<Controls>(
        &mut self,
        reason: ServerSupervisorCloseReason,
        controls: &mut Controls,
    ) -> Result<ServerSupervisorCloseReason, Controls::Error>
    where
        Controls: ServerControlFrameHandler,
    {
        let reason = self.terminate(reason);
        if !self.control_termination_completed {
            let result = controls.handle_connection_terminated(reason).await;
            self.control_termination_completed = true;
            let _event = result?;
        }
        Ok(reason)
    }

    /// Closes the logical connection and then cleanly shuts down the
    /// transport write half. A prior terminal reason remains authoritative.
    pub async fn close_transport(&mut self) -> Result<ServerSupervisorCloseReason, Transport::Error>
    where
        Transport: ShutdownFrameTransport,
    {
        self.terminate_transport(ServerSupervisorCloseReason::Explicit)
            .await
    }

    /// Terminates with a classified reason and shuts down the transport write
    /// half after all prior writes complete.
    pub async fn terminate_transport(
        &mut self,
        reason: ServerSupervisorCloseReason,
    ) -> Result<ServerSupervisorCloseReason, Transport::Error>
    where
        Transport: ShutdownFrameTransport,
    {
        let reason = self.terminate(reason);
        self.transport.shutdown().await?;
        Ok(reason)
    }

    /// Runs the reasoned control/backend cleanup barrier and transport
    /// write-half shutdown even when either operation fails.
    pub async fn close_with_controls_and_transport<Controls>(
        &mut self,
        controls: &mut Controls,
    ) -> Result<
        ServerSupervisorCloseReason,
        ServerTransportCloseError<Controls::Error, Transport::Error>,
    >
    where
        Transport: ShutdownFrameTransport,
        Controls: ServerControlFrameHandler,
    {
        self.terminate_with_controls_and_transport(ServerSupervisorCloseReason::Explicit, controls)
            .await
    }

    /// Terminates with a classified reason while independently attempting
    /// both control cleanup and transport shutdown.
    pub async fn terminate_with_controls_and_transport<Controls>(
        &mut self,
        reason: ServerSupervisorCloseReason,
        controls: &mut Controls,
    ) -> Result<
        ServerSupervisorCloseReason,
        ServerTransportCloseError<Controls::Error, Transport::Error>,
    >
    where
        Transport: ShutdownFrameTransport,
        Controls: ServerControlFrameHandler,
    {
        let cleanup = self.terminate_with_controls(reason, controls).await;
        let shutdown = self.transport.shutdown().await;
        match (cleanup, shutdown) {
            (Ok(reason), Ok(())) => Ok(reason),
            (Err(control), Ok(())) => Err(ServerTransportCloseError::Control(control)),
            (Ok(_), Err(transport)) => Err(ServerTransportCloseError::Transport(transport)),
            (Err(control), Err(transport)) => {
                Err(ServerTransportCloseError::ControlAndTransport { control, transport })
            }
        }
    }
}

impl<Transport> EncodedServerSupervisor<Transport>
where
    Transport: FrameTransport + 'static,
{
    pub async fn accept_handshake<Codec, Gate>(
        &mut self,
        codec: &Codec,
        config: &ServerHandshakeConfig,
        gate: &mut Gate,
    ) -> Result<
        AcceptedServerHandshake<Gate::Authorization>,
        ServerHandshakeError<
            <Codec as WireCodec<ClientHello>>::Error,
            <Codec as WireCodec<ServerHello>>::Error,
            Gate::Error,
            Transport::Error,
        >,
    >
    where
        Codec: WireCodec<ClientHello> + WireCodec<ServerHello>,
        Gate: ServerHandshakeGate,
    {
        if let Some(reason) = self.metrics.close_reason {
            return Err(ServerHandshakeError::Closed(reason));
        }
        if self.phase != ServerSupervisorPhase::AwaitingHandshake {
            return Err(ServerHandshakeError::AlreadyEstablished);
        }

        let transport = Arc::clone(&self.transport);
        let resource_ledger = self.resource_ledger.clone();
        let handshake_resources = self
            .handshake_resources
            .as_ref()
            .map(|resources| (resources.profile, resources.ledger.clone()));
        let mut guard = SupervisorHandshakeGuard {
            metrics: &mut self.metrics,
            phase: &mut self.phase,
            terminal: false,
        };
        let frame = match transport.receive().await {
            Ok(Some(frame)) => frame,
            Ok(None) => {
                guard.fail(ServerSupervisorCloseReason::EndOfStream);
                return Err(ServerHandshakeError::EndOfStream);
            }
            Err(error) => {
                guard.fail(ServerSupervisorCloseReason::HandshakeFailed);
                return Err(ServerHandshakeError::Transport(error));
            }
        };
        if frame.route() != FrameRoute::Handshake {
            let actual = frame.route();
            guard.fail(ServerSupervisorCloseReason::HandshakeFailed);
            return Err(ServerHandshakeError::HandshakeRequired { actual });
        }
        if let Err(error) = frame.validate(config.receive_limits()) {
            guard.fail(ServerSupervisorCloseReason::HandshakeFailed);
            return Err(ServerHandshakeError::IncomingFrame(error));
        }
        let client: ClientHello = match codec.decode_frame(frame) {
            Ok(client) => client,
            Err(error) => {
                guard.fail(ServerSupervisorCloseReason::HandshakeFailed);
                return Err(ServerHandshakeError::DecodeClientHello(error));
            }
        };
        if let Some((profile, _)) = &handshake_resources {
            let advertised = client.receive_limits().max_frame_bytes();
            let maximum = profile.maximum_peer_receive_frame_bytes();
            if advertised > maximum {
                guard.fail(ServerSupervisorCloseReason::HandshakeFailed);
                return Err(ServerHandshakeError::PeerReceiveFrameLimitExceeded {
                    advertised,
                    maximum,
                });
            }
        }
        let authorization = match gate.authorize(&client).await {
            Ok(authorization) => authorization,
            Err(error) => {
                guard.fail(ServerSupervisorCloseReason::HandshakeFailed);
                return Err(ServerHandshakeError::Authorization(error));
            }
        };
        let hello = match negotiate(&client, config) {
            Ok(hello) => hello,
            Err(error) => {
                guard.fail(ServerSupervisorCloseReason::HandshakeFailed);
                return Err(ServerHandshakeError::Negotiate(error));
            }
        };
        let reservation = match &handshake_resources {
            Some((profile, ledger)) => match WireFrame::reserve_bytes(
                ledger,
                ResourceScope::Connection,
                profile.maximum_encoding_wire_bytes(),
            ) {
                Ok(reservation) => Some(reservation),
                Err(error) => {
                    guard.fail(ServerSupervisorCloseReason::HandshakeFailed);
                    return Err(ServerHandshakeError::OutgoingFrame(error));
                }
            },
            None => None,
        };
        let parts = match codec.encode_parts(&hello) {
            Ok(parts) => parts,
            Err(error) => {
                guard.fail(ServerSupervisorCloseReason::HandshakeFailed);
                return Err(ServerHandshakeError::EncodeServerHello(error));
            }
        };
        if parts.route() != FrameRoute::Handshake {
            let actual = parts.route();
            guard.fail(ServerSupervisorCloseReason::HandshakeFailed);
            return Err(ServerHandshakeError::EncodedRouteMismatch { actual });
        }
        let response = match reservation {
            Some(reservation) => WireFrame::new_reserved_up_to(
                hello.selected_version(),
                FrameRoute::Handshake,
                parts.payload().to_vec(),
                parts.attachments().to_vec(),
                hello.client_receive_limits(),
                reservation,
            ),
            None => WireFrame::new(
                hello.selected_version(),
                FrameRoute::Handshake,
                parts.payload().to_vec(),
                parts.attachments().to_vec(),
                hello.client_receive_limits(),
            ),
        };
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                guard.fail(ServerSupervisorCloseReason::HandshakeFailed);
                return Err(ServerHandshakeError::OutgoingFrame(error));
            }
        };
        if let Err(error) = transport.send(response).await {
            guard.fail(ServerSupervisorCloseReason::HandshakeFailed);
            return Err(ServerHandshakeError::Transport(error));
        }

        self.inbound_policy = Some(NegotiatedInboundPolicy {
            version: hello.selected_version(),
            limits: hello.server_receive_limits(),
        });
        self.negotiated = Some(hello.clone());
        guard.succeed();
        Ok(AcceptedServerHandshake {
            hello,
            authorization,
            resource_ledger,
        })
    }

    pub async fn run<Commands, Controls, Observe, ObserverError>(
        &mut self,
        commands: &Commands,
        controls: &mut Controls,
        observe: Observe,
    ) -> Result<
        ServerSupervisorCloseReason,
        ServerSupervisorError<Transport::Error, Commands::Error, Controls::Error, ObserverError>,
    >
    where
        Commands: ConcurrentServerFrameHandler<Transport>,
        Controls: ServerControlFrameHandler,
        Observe: FnMut(
            ServerSupervisorEvent<Commands::Event, Controls::Event>,
        ) -> Result<(), ObserverError>,
    {
        self.run_with_drain_deadline(commands, controls, observe, pending())
            .await
    }

    pub async fn run_with_drain_deadline<
        Commands,
        Controls,
        Observe,
        ObserverError,
        DrainDeadline,
    >(
        &mut self,
        commands: &Commands,
        controls: &mut Controls,
        observe: Observe,
        drain_deadline: DrainDeadline,
    ) -> Result<
        ServerSupervisorCloseReason,
        ServerSupervisorError<Transport::Error, Commands::Error, Controls::Error, ObserverError>,
    >
    where
        Commands: ConcurrentServerFrameHandler<Transport>,
        Controls: ServerControlFrameHandler,
        Observe: FnMut(
            ServerSupervisorEvent<Commands::Event, Controls::Event>,
        ) -> Result<(), ObserverError>,
        DrainDeadline: Future<Output = ()>,
    {
        self.run_with_outbound_and_drain_deadline(
            commands,
            controls,
            &NoServerOutbound,
            observe,
            drain_deadline,
        )
        .await
    }

    pub async fn run_with_outbound<Commands, Controls, Outbound, Observe, ObserverError>(
        &mut self,
        commands: &Commands,
        controls: &mut Controls,
        outbound: &Outbound,
        observe: Observe,
    ) -> Result<
        ServerSupervisorCloseReason,
        ServerSupervisorError<
            Transport::Error,
            Commands::Error,
            Controls::Error,
            ObserverError,
            Outbound::Error,
        >,
    >
    where
        Commands: ConcurrentServerFrameHandler<Transport>,
        Controls: ServerControlFrameHandler,
        Outbound: ConcurrentServerOutbound,
        Observe: FnMut(
            ServerSupervisorEvent<Commands::Event, Controls::Event, Outbound::Event>,
        ) -> Result<(), ObserverError>,
    {
        self.run_with_outbound_and_drain_deadline(commands, controls, outbound, observe, pending())
            .await
    }

    /// Runs receive, bounded command futures, and one caller-selected
    /// outbound lane with rotating priority. An idle outbound lane is parked
    /// until receive or command progress occurs, avoiding a ready-loop while
    /// preserving caller-driven execution and a single transport reader.
    pub async fn run_with_outbound_and_drain_deadline<
        Commands,
        Controls,
        Outbound,
        Observe,
        ObserverError,
        DrainDeadline,
    >(
        &mut self,
        commands: &Commands,
        controls: &mut Controls,
        outbound: &Outbound,
        observe: Observe,
        drain_deadline: DrainDeadline,
    ) -> Result<
        ServerSupervisorCloseReason,
        ServerSupervisorError<
            Transport::Error,
            Commands::Error,
            Controls::Error,
            ObserverError,
            Outbound::Error,
        >,
    >
    where
        Commands: ConcurrentServerFrameHandler<Transport>,
        Controls: ServerControlFrameHandler,
        Outbound: ConcurrentServerOutbound,
        Observe: FnMut(
            ServerSupervisorEvent<Commands::Event, Controls::Event, Outbound::Event>,
        ) -> Result<(), ObserverError>,
        DrainDeadline: Future<Output = ()>,
    {
        let mut backend = NoServerBackend;
        self.run_with_outbound_backend_and_drain_deadline(
            commands,
            controls,
            outbound,
            &mut backend,
            observe,
            drain_deadline,
        )
        .await
    }

    pub async fn run_with_outbound_and_backend<
        Commands,
        Controls,
        Outbound,
        Backend,
        Observe,
        ObserverError,
    >(
        &mut self,
        commands: &Commands,
        controls: &mut Controls,
        outbound: &Outbound,
        backend: &mut Backend,
        observe: Observe,
    ) -> Result<
        ServerSupervisorCloseReason,
        ServerSupervisorError<
            Transport::Error,
            Commands::Error,
            Controls::Error,
            ObserverError,
            Outbound::Error,
            Backend::Error,
        >,
    >
    where
        Commands: ConcurrentServerFrameHandler<Transport>,
        Controls: ServerControlFrameHandler,
        Outbound: ConcurrentServerOutbound,
        Backend: ConcurrentServerBackend,
        Observe: FnMut(
            ServerSupervisorEvent<Commands::Event, Controls::Event, Outbound::Event, Backend::Event>,
        ) -> Result<(), ObserverError>,
    {
        self.run_with_outbound_backend_and_drain_deadline(
            commands,
            controls,
            outbound,
            backend,
            observe,
            pending(),
        )
        .await
    }

    /// Runs the unique transport reader, bounded command set, serial outbound
    /// writer, and one caller-owned backend future with rotating priority.
    /// The backend future is never cancelled merely because another lane made
    /// progress; after its first poll it remains owned until it completes or
    /// the complete connection run is abandoned.
    pub async fn run_with_outbound_backend_and_drain_deadline<
        Commands,
        Controls,
        Outbound,
        Backend,
        Observe,
        ObserverError,
        DrainDeadline,
    >(
        &mut self,
        commands: &Commands,
        controls: &mut Controls,
        outbound: &Outbound,
        backend: &mut Backend,
        mut observe: Observe,
        drain_deadline: DrainDeadline,
    ) -> Result<
        ServerSupervisorCloseReason,
        ServerSupervisorError<
            Transport::Error,
            Commands::Error,
            Controls::Error,
            ObserverError,
            Outbound::Error,
            Backend::Error,
        >,
    >
    where
        Commands: ConcurrentServerFrameHandler<Transport>,
        Controls: ServerControlFrameHandler,
        Outbound: ConcurrentServerOutbound,
        Backend: ConcurrentServerBackend,
        Observe: FnMut(
            ServerSupervisorEvent<Commands::Event, Controls::Event, Outbound::Event, Backend::Event>,
        ) -> Result<(), ObserverError>,
        DrainDeadline: Future<Output = ()>,
    {
        if let Some(reason) = self.metrics.close_reason {
            backend.handle_connection_terminated(reason);
            if !self.control_termination_completed {
                let result = controls.handle_connection_terminated(reason).await;
                self.control_termination_completed = true;
                let _event = result.map_err(ServerSupervisorError::Control)?;
            }
            return Ok(reason);
        }
        if self.phase == ServerSupervisorPhase::AwaitingHandshake {
            return Err(ServerSupervisorError::HandshakeRequired);
        }

        enum Wake<Receive, Command, Outbound, Backend> {
            Receive(Receive),
            Command { index: usize, output: Command },
            Outbound(Outbound),
            OutboundReady,
            Backend(Backend),
            DrainDeadline,
        }

        let transport = Arc::clone(&self.transport);
        let limits = self.limits;
        let inbound_policy = self.inbound_policy;
        let mut backend_run = SupervisorBackendGuard {
            backend,
            terminal: false,
        };
        let mut run_metrics = SupervisorRunMetricsGuard {
            metrics: &mut self.metrics,
            terminal: false,
        };
        let mut receive = Some(Box::pin(transport.receive()));
        let mut active: Vec<ActiveServerCommand<Commands::Future>> =
            Vec::with_capacity(limits.in_flight_commands);
        let mut drain_deadline = Box::pin(drain_deadline);
        let mut outbound_send = Some(Box::pin(outbound.send_once()));
        let mut outbound_ready: Option<Pin<Box<dyn Future<Output = ()> + Send + '_>>> = None;
        let mut next_command = 0usize;
        let mut next_lane = 0usize;
        let mut prefetched_frame = None;
        let control_termination_completed = &mut self.control_termination_completed;

        macro_rules! cleanup_controls_once {
            ($reason:expr) => {{
                if *control_termination_completed {
                    None
                } else {
                    let result = controls.handle_connection_terminated($reason).await;
                    *control_termination_completed = true;
                    result.err()
                }
            }};
        }

        macro_rules! fail_with_cleanup {
            ($reason:expr, $primary:expr) => {{
                let reason = $reason;
                let primary = $primary;
                let error = match cleanup_controls_once!(reason) {
                    Some(cleanup) => ServerSupervisorError::TerminationCleanup {
                        reason,
                        primary: Box::new(primary),
                        cleanup,
                    },
                    None => primary,
                };
                let dropped = active.len();
                active.clear();
                backend_run.finish(reason);
                run_metrics.finish(reason, dropped);
                return Err(error);
            }};
        }

        macro_rules! handle_clean_eof {
            () => {{
                receive = None;
                let close_event = if *control_termination_completed {
                    None
                } else {
                    let result = controls
                        .handle_connection_terminated(ServerSupervisorCloseReason::EndOfStream)
                        .await;
                    *control_termination_completed = true;
                    match result {
                        Ok(event) => event,
                        Err(error) => {
                            let dropped = active.len();
                            active.clear();
                            backend_run.finish(ServerSupervisorCloseReason::ControlFailed);
                            run_metrics.finish(ServerSupervisorCloseReason::ControlFailed, dropped);
                            return Err(ServerSupervisorError::Control(error));
                        }
                    }
                };
                if let Some(event) = close_event {
                    run_metrics.metrics.handled_controls =
                        run_metrics.metrics.handled_controls.saturating_add(1);
                    if let Err(error) = observe(ServerSupervisorEvent::Control(event)) {
                        let dropped = active.len();
                        active.clear();
                        backend_run.finish(ServerSupervisorCloseReason::ObserverFailed);
                        run_metrics.finish(ServerSupervisorCloseReason::ObserverFailed, dropped);
                        return Err(ServerSupervisorError::Observer(error));
                    }
                }
                if limits.eof_policy == ServerEofPolicy::DropActive || active.is_empty() {
                    let dropped = active.len();
                    active.clear();
                    backend_run.finish(ServerSupervisorCloseReason::EndOfStream);
                    run_metrics.finish(ServerSupervisorCloseReason::EndOfStream, dropped);
                    return Ok(ServerSupervisorCloseReason::EndOfStream);
                }
                continue;
            }};
        }

        loop {
            let wake = if let Some(frame) = prefetched_frame.take() {
                Wake::Receive(Ok(Some(frame)))
            } else {
                poll_fn(|context| {
                    for lane_offset in 0..4 {
                        match (next_lane + lane_offset) % 4 {
                            0 => {
                                if let Some(receive) = receive.as_mut()
                                    && let Poll::Ready(received) = receive.as_mut().poll(context)
                                {
                                    return Poll::Ready(Wake::Receive(received));
                                }
                            }
                            1 => {
                                for offset in 0..active.len() {
                                    let index = (next_command + offset) % active.len();
                                    if let Poll::Ready(output) =
                                        active[index].future.as_mut().poll(context)
                                    {
                                        return Poll::Ready(Wake::Command { index, output });
                                    }
                                }
                            }
                            2 => {
                                if let Some(send) = outbound_send.as_mut()
                                    && let Poll::Ready(output) = send.as_mut().poll(context)
                                {
                                    return Poll::Ready(Wake::Outbound(output));
                                }
                                if let Some(ready) = outbound_ready.as_mut()
                                    && let Poll::Ready(()) = ready.as_mut().poll(context)
                                {
                                    return Poll::Ready(Wake::OutboundReady);
                                }
                            }
                            _ => {
                                if let Poll::Ready(output) =
                                    backend_run.backend_mut().poll_drive_once(context)
                                {
                                    return Poll::Ready(Wake::Backend(output));
                                }
                            }
                        }
                    }
                    if receive.is_none()
                        && !active.is_empty()
                        && drain_deadline.as_mut().poll(context).is_ready()
                    {
                        return Poll::Ready(Wake::DrainDeadline);
                    }
                    Poll::Pending
                })
                .await
            };

            match wake {
                Wake::Receive(received) => {
                    next_lane = 1;
                    let frame = match received {
                        Ok(Some(frame)) => {
                            receive = Some(Box::pin(transport.receive()));
                            if outbound_send.is_none() {
                                outbound_ready = None;
                                outbound_send = Some(Box::pin(outbound.send_once()));
                            }
                            frame
                        }
                        Ok(None) => {
                            handle_clean_eof!();
                        }
                        Err(error) => {
                            fail_with_cleanup!(
                                ServerSupervisorCloseReason::ReceiveFailed,
                                ServerSupervisorError::Transport(error)
                            );
                        }
                    };

                    if let Some(policy) = inbound_policy {
                        let validation = if frame.route() == FrameRoute::Handshake {
                            Err(ApplicationFrameError::DuplicateHandshake)
                        } else if frame.version() != policy.version {
                            Err(ApplicationFrameError::VersionMismatch {
                                expected: policy.version,
                                actual: frame.version(),
                            })
                        } else {
                            frame
                                .validate(policy.limits)
                                .map_err(ApplicationFrameError::Frame)
                        };
                        if let Err(error) = validation {
                            fail_with_cleanup!(
                                ServerSupervisorCloseReason::IncomingFrameRejected,
                                ServerSupervisorError::IncomingFrame(error)
                            );
                        }
                    }

                    match frame.route() {
                        FrameRoute::Control => {
                            enum ControlBarrier<Control, Transport> {
                                Control(Control),
                                EndOfStream,
                                Transport(Transport),
                            }

                            let allow_prefetch = controls.releases_frame_before_pending();
                            let mut control = Box::pin(controls.handle_control(frame));
                            let mut barrier_prefetch = None;
                            let mut control_started = false;
                            let barrier = poll_fn(|context| {
                                let first_control_poll = !control_started;
                                if first_control_poll {
                                    control_started = true;
                                    if let Poll::Ready(output) = control.as_mut().poll(context) {
                                        return Poll::Ready(ControlBarrier::Control(output));
                                    }
                                }
                                if allow_prefetch
                                    && barrier_prefetch.is_none()
                                    && let Some(receive_future) = receive.as_mut()
                                    && let Poll::Ready(received) =
                                        receive_future.as_mut().poll(context)
                                {
                                    receive = None;
                                    match received {
                                        Ok(Some(frame)) => barrier_prefetch = Some(frame),
                                        Ok(None) => {
                                            return Poll::Ready(ControlBarrier::EndOfStream);
                                        }
                                        Err(error) => {
                                            return Poll::Ready(ControlBarrier::Transport(error));
                                        }
                                    }
                                }
                                if first_control_poll {
                                    Poll::Pending
                                } else {
                                    control.as_mut().poll(context).map(ControlBarrier::Control)
                                }
                            })
                            .await;
                            drop(control);

                            let output = match barrier {
                                ControlBarrier::Control(output) => {
                                    prefetched_frame = barrier_prefetch;
                                    output
                                }
                                ControlBarrier::EndOfStream => {
                                    handle_clean_eof!();
                                }
                                ControlBarrier::Transport(error) => {
                                    fail_with_cleanup!(
                                        ServerSupervisorCloseReason::ReceiveFailed,
                                        ServerSupervisorError::Transport(error)
                                    );
                                }
                            };
                            let disposition = match output {
                                Ok(disposition) => disposition,
                                Err(error) => {
                                    fail_with_cleanup!(
                                        ServerSupervisorCloseReason::ControlFailed,
                                        ServerSupervisorError::Control(error)
                                    );
                                }
                            };
                            let event = match disposition {
                                ServerFrameDisposition::Handled(event) => {
                                    run_metrics.metrics.handled_controls =
                                        run_metrics.metrics.handled_controls.saturating_add(1);
                                    ServerSupervisorEvent::Control(event)
                                }
                                ServerFrameDisposition::Unhandled(frame) => {
                                    run_metrics.metrics.unhandled_frames =
                                        run_metrics.metrics.unhandled_frames.saturating_add(1);
                                    ServerSupervisorEvent::Unhandled(frame)
                                }
                            };
                            if let Err(error) = observe(event) {
                                fail_with_cleanup!(
                                    ServerSupervisorCloseReason::ObserverFailed,
                                    ServerSupervisorError::Observer(error)
                                );
                            }
                            if let Some(reason) = controls.terminal_close_reason() {
                                *control_termination_completed = true;
                                let dropped = active.len();
                                active.clear();
                                backend_run.finish(reason);
                                run_metrics.finish(reason, dropped);
                                return Ok(reason);
                            }
                        }
                        FrameRoute::Command { .. } => {
                            if active.len() >= limits.in_flight_commands {
                                run_metrics.metrics.capacity_failures =
                                    run_metrics.metrics.capacity_failures.saturating_add(1);
                                fail_with_cleanup!(
                                    ServerSupervisorCloseReason::CommandCapacityExceeded,
                                    ServerSupervisorError::CommandCapacityExceeded {
                                        limit: limits.in_flight_commands,
                                    }
                                );
                            }
                            let resource = match (&self.resource_ledger, frame.route()) {
                                (
                                    Some(ledger),
                                    FrameRoute::Command {
                                        call_id,
                                        method: _,
                                        target: _,
                                    },
                                ) => match ledger.reserve(
                                    ResourceScope::Call(call_id),
                                    ResourceClass::PendingCall,
                                    1,
                                ) {
                                    Ok(resource) => Some(resource),
                                    Err(error) => {
                                        run_metrics.metrics.capacity_failures =
                                            run_metrics.metrics.capacity_failures.saturating_add(1);
                                        fail_with_cleanup!(
                                            ServerSupervisorCloseReason::CommandResourceExhausted,
                                            ServerSupervisorError::Resource(error)
                                        );
                                    }
                                },
                                _ => None,
                            };
                            active.push(ActiveServerCommand {
                                future: Box::pin(
                                    commands.handle_concurrently(frame, Arc::clone(&transport)),
                                ),
                                _resource: resource,
                            });
                            next_command = active.len() - 1;
                            run_metrics.metrics.active_commands = active.len();
                            run_metrics.metrics.accepted_commands =
                                run_metrics.metrics.accepted_commands.saturating_add(1);
                            run_metrics.metrics.peak_active_commands =
                                run_metrics.metrics.peak_active_commands.max(active.len());
                        }
                        _ => {
                            run_metrics.metrics.unhandled_frames =
                                run_metrics.metrics.unhandled_frames.saturating_add(1);
                            if let Err(error) = observe(ServerSupervisorEvent::Unhandled(frame)) {
                                fail_with_cleanup!(
                                    ServerSupervisorCloseReason::ObserverFailed,
                                    ServerSupervisorError::Observer(error)
                                );
                            }
                        }
                    }
                }
                Wake::Command { index, output } => {
                    next_lane = 2;
                    drop(active.remove(index));
                    run_metrics.metrics.active_commands = active.len();
                    next_command = if active.is_empty() {
                        0
                    } else {
                        index % active.len()
                    };
                    let disposition = match output {
                        Ok(disposition) => disposition,
                        Err(error) => {
                            fail_with_cleanup!(
                                ServerSupervisorCloseReason::CommandFailed,
                                ServerSupervisorError::Command(error)
                            );
                        }
                    };
                    run_metrics.metrics.completed_commands =
                        run_metrics.metrics.completed_commands.saturating_add(1);
                    let event = match disposition {
                        ServerFrameDisposition::Handled(event) => {
                            ServerSupervisorEvent::Command(event)
                        }
                        ServerFrameDisposition::Unhandled(frame) => {
                            run_metrics.metrics.unhandled_frames =
                                run_metrics.metrics.unhandled_frames.saturating_add(1);
                            ServerSupervisorEvent::Unhandled(frame)
                        }
                    };
                    if let Err(error) = observe(event) {
                        fail_with_cleanup!(
                            ServerSupervisorCloseReason::ObserverFailed,
                            ServerSupervisorError::Observer(error)
                        );
                    }
                    if outbound_send.is_none() {
                        outbound_ready = None;
                        outbound_send = Some(Box::pin(outbound.send_once()));
                    }
                    if receive.is_none() && active.is_empty() {
                        backend_run.finish(ServerSupervisorCloseReason::EndOfStream);
                        run_metrics.finish(ServerSupervisorCloseReason::EndOfStream, 0);
                        return Ok(ServerSupervisorCloseReason::EndOfStream);
                    }
                }
                Wake::Outbound(output) => {
                    next_lane = 3;
                    outbound_send = None;
                    let event = match output {
                        Ok(event) => event,
                        Err(error) => {
                            fail_with_cleanup!(
                                ServerSupervisorCloseReason::OutboundFailed,
                                ServerSupervisorError::Outbound(error)
                            );
                        }
                    };
                    if let Some(event) = event {
                        run_metrics.metrics.outbound_messages =
                            run_metrics.metrics.outbound_messages.saturating_add(1);
                        if let Err(error) = observe(ServerSupervisorEvent::Outbound(event)) {
                            fail_with_cleanup!(
                                ServerSupervisorCloseReason::ObserverFailed,
                                ServerSupervisorError::Observer(error)
                            );
                        }
                        outbound_send = Some(Box::pin(outbound.send_once()));
                    } else {
                        outbound_ready = Some(outbound.wait_until_ready());
                    }
                }
                Wake::OutboundReady => {
                    next_lane = 3;
                    outbound_ready = None;
                    outbound_send = Some(Box::pin(outbound.send_once()));
                }
                Wake::Backend(output) => {
                    next_lane = 0;
                    let event = match output {
                        Ok(event) => event,
                        Err(error) => {
                            fail_with_cleanup!(
                                ServerSupervisorCloseReason::BackendEventFailed,
                                ServerSupervisorError::Backend(error)
                            );
                        }
                    };
                    if let Err(error) = observe(ServerSupervisorEvent::Backend(event)) {
                        fail_with_cleanup!(
                            ServerSupervisorCloseReason::ObserverFailed,
                            ServerSupervisorError::Observer(error)
                        );
                    }
                }
                Wake::DrainDeadline => {
                    run_metrics.metrics.drain_deadline_expirations = run_metrics
                        .metrics
                        .drain_deadline_expirations
                        .saturating_add(1);
                    let dropped = active.len();
                    active.clear();
                    backend_run.finish(ServerSupervisorCloseReason::DrainDeadlineExceeded);
                    run_metrics.finish(ServerSupervisorCloseReason::DrainDeadlineExceeded, dropped);
                    return Ok(ServerSupervisorCloseReason::DrainDeadlineExceeded);
                }
            }
        }
    }
}

impl<Transport> EncodedServerSupervisor<Transport>
where
    Transport: FrameTransport + ShutdownFrameTransport + 'static,
{
    /// Owns one complete application-phase connection run and shuts down the
    /// transport write half after every terminal outcome. A pre-handshake call
    /// is rejected without shutdown because the supervisor remains reusable
    /// for handshake acceptance.
    pub async fn run_connection_with_outbound_and_backend<
        Commands,
        Controls,
        Outbound,
        Backend,
        Observe,
        ObserverError,
    >(
        &mut self,
        commands: &Commands,
        controls: &mut Controls,
        outbound: &Outbound,
        backend: &mut Backend,
        observe: Observe,
    ) -> Result<
        ServerSupervisorCloseReason,
        ServerConnectionRunError<
            ServerSupervisorError<
                Transport::Error,
                Commands::Error,
                Controls::Error,
                ObserverError,
                Outbound::Error,
                Backend::Error,
            >,
            Transport::Error,
        >,
    >
    where
        Commands: ConcurrentServerFrameHandler<Transport>,
        Controls: ServerControlFrameHandler,
        Outbound: ConcurrentServerOutbound,
        Backend: ConcurrentServerBackend,
        Observe: FnMut(
            ServerSupervisorEvent<Commands::Event, Controls::Event, Outbound::Event, Backend::Event>,
        ) -> Result<(), ObserverError>,
    {
        self.run_connection_with_outbound_backend_and_drain_deadline(
            commands,
            controls,
            outbound,
            backend,
            observe,
            pending(),
        )
        .await
    }

    pub async fn run_connection_with_outbound_backend_and_drain_deadline<
        Commands,
        Controls,
        Outbound,
        Backend,
        Observe,
        ObserverError,
        DrainDeadline,
    >(
        &mut self,
        commands: &Commands,
        controls: &mut Controls,
        outbound: &Outbound,
        backend: &mut Backend,
        observe: Observe,
        drain_deadline: DrainDeadline,
    ) -> Result<
        ServerSupervisorCloseReason,
        ServerConnectionRunError<
            ServerSupervisorError<
                Transport::Error,
                Commands::Error,
                Controls::Error,
                ObserverError,
                Outbound::Error,
                Backend::Error,
            >,
            Transport::Error,
        >,
    >
    where
        Commands: ConcurrentServerFrameHandler<Transport>,
        Controls: ServerControlFrameHandler,
        Outbound: ConcurrentServerOutbound,
        Backend: ConcurrentServerBackend,
        Observe: FnMut(
            ServerSupervisorEvent<Commands::Event, Controls::Event, Outbound::Event, Backend::Event>,
        ) -> Result<(), ObserverError>,
        DrainDeadline: Future<Output = ()>,
    {
        let run = self
            .run_with_outbound_backend_and_drain_deadline(
                commands,
                controls,
                outbound,
                backend,
                observe,
                drain_deadline,
            )
            .await;
        let Some(reason) = self.metrics.close_reason else {
            return run.map_err(ServerConnectionRunError::Run);
        };
        let shutdown = self.transport.shutdown().await;
        match (run, shutdown) {
            (Ok(reason), Ok(())) => Ok(reason),
            (Err(run), Ok(())) => Err(ServerConnectionRunError::Run(run)),
            (Ok(_), Err(error)) => Err(ServerConnectionRunError::Shutdown { reason, error }),
            (Err(run), Err(shutdown)) => Err(ServerConnectionRunError::RunAndShutdown {
                reason,
                run,
                shutdown,
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerCloseReason {
    Explicit,
    EndOfStream,
    ReceiveFailed,
    HandlerFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerDriverEvent<Event> {
    Handled(Event),
    Unhandled(WireFrame),
    Closed(ServerCloseReason),
}

#[derive(Debug)]
pub enum ServerDriverError<Transport, Handler> {
    Transport(Transport),
    Handler(Handler),
}

impl<Transport, Handler> Display for ServerDriverError<Transport, Handler>
where
    Transport: Display,
    Handler: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => Display::fmt(error, formatter),
            Self::Handler(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Transport, Handler> Error for ServerDriverError<Transport, Handler>
where
    Transport: Error + 'static,
    Handler: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            Self::Handler(error) => Some(error),
        }
    }
}

/// Owns the server connection's only receive path.
pub struct EncodedServerDriver<Transport> {
    transport: Transport,
    close_reason: Option<ServerCloseReason>,
}

impl<Transport> EncodedServerDriver<Transport> {
    pub const fn new(transport: Transport) -> Self {
        Self {
            transport,
            close_reason: None,
        }
    }

    pub const fn close_reason(&self) -> Option<ServerCloseReason> {
        self.close_reason
    }

    pub fn close(&mut self) -> ServerCloseReason {
        *self.close_reason.get_or_insert(ServerCloseReason::Explicit)
    }

    pub fn into_transport(self) -> Transport {
        self.transport
    }
}

impl<Transport> EncodedServerDriver<Transport>
where
    Transport: FrameTransport,
{
    pub async fn receive_once_with<Handler>(
        &mut self,
        handler: &mut Handler,
    ) -> Result<
        ServerDriverEvent<Handler::Event>,
        ServerDriverError<Transport::Error, Handler::Error>,
    >
    where
        Handler: ServerFrameHandler<Transport>,
    {
        if let Some(reason) = self.close_reason {
            return Ok(ServerDriverEvent::Closed(reason));
        }
        let frame = match self.transport.receive().await {
            Ok(Some(frame)) => frame,
            Ok(None) => {
                self.close_reason = Some(ServerCloseReason::EndOfStream);
                return Ok(ServerDriverEvent::Closed(ServerCloseReason::EndOfStream));
            }
            Err(error) => {
                self.close_reason = Some(ServerCloseReason::ReceiveFailed);
                return Err(ServerDriverError::Transport(error));
            }
        };
        let disposition = match handler.handle(frame, &self.transport).await {
            Ok(disposition) => disposition,
            Err(error) => {
                self.close_reason = Some(ServerCloseReason::HandlerFailed);
                return Err(ServerDriverError::Handler(error));
            }
        };
        match disposition {
            ServerFrameDisposition::Handled(event) => Ok(ServerDriverEvent::Handled(event)),
            ServerFrameDisposition::Unhandled(frame) => Ok(ServerDriverEvent::Unhandled(frame)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::convert::Infallible;
    use std::error::Error;
    use std::fmt::{self, Display, Formatter};
    use std::future::{Future, Pending, Ready, pending, poll_fn, ready};
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    use pwright_client::{AtomicCallIdSource, Client, RemoteInitializer, RemoteKind};
    use pwright_client_wire::{DriverEvent, EncodedClientChannel, EncodedClientHandshake};
    use pwright_codec_handshake::HandshakeCodec as CanonicalHandshakeCodec;
    use pwright_core::{
        CallId, CallMetadata, Capability, CapabilityCommandSpec, CapabilityDescriptor,
        CapabilityEventSpec, CapabilityId, Command, CommandId, CommandResult, CommandSpec, EventId,
        EventSpec, EventTag, MethodTag, ObjectId, ObjectKindCatalog, ObjectKindDescriptor,
        ObjectKindId, ObjectKindTag, OperationError, PluginInfo, ProductionResourceLedger,
        ProtocolVersion, ResourceClass, ResourceError, ResourceLimits, ResourceScope, Service,
    };
    use pwright_runtime::{DirectDispatcher, RuntimeLimits};
    use pwright_transport::{
        CapabilityManifest, CapabilityRequest, ClientHello, EncodedParts, FrameLimits, FrameRoute,
        FrameTransport, NegotiationLimits, ServerHandshakeConfig, ServerHello,
        ShutdownFrameTransport, VersionPolicy, WireCapabilityId, WireCodec, WireFrame,
        WireProtocol,
    };
    use pwright_transport_stream::{StreamFrameCodec, StreamFrameLimits};
    use pwright_transport_tokio::TokioStreamTransport;
    use tokio::io::duplex;

    use super::{
        AllowUnauthenticated, ChainedServerOutboundEvent, CommandHandled, ConcurrentServerBackend,
        ConcurrentServerBackendExt, ConcurrentServerOutbound, ConcurrentServerOutboundExt,
        ConcurrentTypedCommandHandler, Either, EncodedServerDriver, EncodedServerSupervisor,
        HandshakeEstablished, HandshakeFirstError, HandshakeFirstEvent, HandshakeFirstHandler,
        NoServerControl, ServerCloseReason, ServerConnectionRunError, ServerControlFrameHandler,
        ServerDriverEvent, ServerEofPolicy, ServerFrameDisposition, ServerFrameHandler,
        ServerFrameHandlerExt, ServerHandshakeGate, ServerHandshakeProductionProfile,
        ServerSupervisorCloseReason, ServerSupervisorError, ServerSupervisorEvent,
        ServerSupervisorLimits, ServerSupervisorPhase, ServerSupervisorProductionError,
        ServerSupervisorProductionProfile, ServerTransportCloseError, TypedCommandHandler,
        TypedCommandHandlerError,
    };
    use super::{
        ConcurrentServerFrameHandler, ConcurrentServerFrameHandlerExt, ServerCapabilityInventory,
        ServerCapabilityInventoryError, ServerCapabilitySource,
    };

    const ADD_CAPABILITY_ID: CapabilityId = CapabilityId::new("test.add");
    const ADD_COMMAND_ID: CommandId = CommandId::new("test.add.execute");
    const CALCULATOR_OBJECT_KIND_ID: ObjectKindId = ObjectKindId::new("test.calculator");
    const CALCULATOR_TARGETS: &[ObjectKindId] = &[CALCULATOR_OBJECT_KIND_ID];
    const CALCULATOR_OBJECT_KIND: ObjectKindDescriptor = ObjectKindDescriptor::new(
        CALCULATOR_OBJECT_KIND_ID,
        ObjectKindTag::new(77),
        "Calculator",
    );
    const CALCULATOR_OBJECT_KINDS: ObjectKindCatalog =
        ObjectKindCatalog::new(&[CALCULATOR_OBJECT_KIND]);

    struct AddCapability;

    impl Capability for AddCapability {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(
                ADD_CAPABILITY_ID,
                "Test addition",
                ProtocolVersion::new(1, 3),
            ),
            &[],
            &[ADD_COMMAND_ID],
        );
    }

    struct Add;

    impl CommandSpec for Add {
        const METHOD: MethodTag = MethodTag::new(17);
        const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = CALCULATOR_TARGETS;
        type Params = u32;
        type Result = u32;
        type ErrorDetails = ();
    }

    impl CapabilityCommandSpec for Add {
        type Capability = AddCapability;
        const COMMAND: CommandId = ADD_COMMAND_ID;
    }

    const CHANGED_EVENT_ID: EventId = EventId::new("test.calculator.changed");

    struct CalculatorEvents;

    impl Capability for CalculatorEvents {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(
                CapabilityId::new("test.calculator-events"),
                "Calculator events",
                ProtocolVersion::new(1, 0),
            ),
            &[],
            &[],
        )
        .with_events(&[CHANGED_EVENT_ID]);
    }

    struct CalculatorChanged;

    impl EventSpec for CalculatorChanged {
        const EVENT: EventTag = EventTag::new(23);
        type Payload = ();
    }

    impl CapabilityEventSpec for CalculatorChanged {
        type Capability = CalculatorEvents;
        const EVENT_ID: EventId = CHANGED_EVENT_ID;
        const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = CALCULATOR_TARGETS;
    }

    struct UntargetedAdd;

    impl CommandSpec for UntargetedAdd {
        const METHOD: MethodTag = MethodTag::new(117);
        type Params = u32;
        type Result = u32;
        type ErrorDetails = ();
    }

    impl CapabilityCommandSpec for UntargetedAdd {
        type Capability = AddCapability;
        const COMMAND: CommandId = ADD_COMMAND_ID;
    }

    const PARTIAL_CAPABILITY_ID: CapabilityId = CapabilityId::new("test.partial");
    const PARTIAL_COMMAND_ID: CommandId = CommandId::new("test.partial.present");
    const MISSING_COMMAND_ID: CommandId = CommandId::new("test.partial.missing");

    struct PartialCapability;

    impl Capability for PartialCapability {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(
                PARTIAL_CAPABILITY_ID,
                "Partial test capability",
                ProtocolVersion::new(1, 0),
            ),
            &[],
            &[PARTIAL_COMMAND_ID, MISSING_COMMAND_ID],
        );
    }

    struct PartialCommand;

    impl CommandSpec for PartialCommand {
        const METHOD: MethodTag = MethodTag::new(18);
        const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = CALCULATOR_TARGETS;
        type Params = ();
        type Result = ();
        type ErrorDetails = ();
    }

    impl CapabilityCommandSpec for PartialCommand {
        type Capability = PartialCapability;
        const COMMAND: CommandId = PARTIAL_COMMAND_ID;
    }

    const CONFLICT_CAPABILITY_ID: CapabilityId = CapabilityId::new("test.conflict");
    const CONFLICT_COMMAND_ID: CommandId = CommandId::new("test.conflict.execute");

    struct ConflictCapability;

    impl Capability for ConflictCapability {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(
                CONFLICT_CAPABILITY_ID,
                "Conflicting test capability",
                ProtocolVersion::new(1, 0),
            ),
            &[],
            &[CONFLICT_COMMAND_ID],
        );
    }

    struct ConflictingMethod;

    impl CommandSpec for ConflictingMethod {
        const METHOD: MethodTag = Add::METHOD;
        const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = CALCULATOR_TARGETS;
        type Params = ();
        type Result = ();
        type ErrorDetails = ();
    }

    impl CapabilityCommandSpec for ConflictingMethod {
        type Capability = ConflictCapability;
        const COMMAND: CommandId = CONFLICT_COMMAND_ID;
    }

    const SECOND_CAPABILITY_ID: CapabilityId = CapabilityId::new("test.second");
    const SECOND_COMMAND_ID: CommandId = CommandId::new("test.second.execute");

    struct SecondCapability;

    impl Capability for SecondCapability {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(
                SECOND_CAPABILITY_ID,
                "Second test capability",
                ProtocolVersion::new(2, 1),
            ),
            &[],
            &[SECOND_COMMAND_ID],
        );
    }

    struct SecondCommand;

    impl CommandSpec for SecondCommand {
        const METHOD: MethodTag = MethodTag::new(19);
        const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = CALCULATOR_TARGETS;
        type Params = ();
        type Result = ();
        type ErrorDetails = ();
    }

    impl CapabilityCommandSpec for SecondCommand {
        type Capability = SecondCapability;
        const COMMAND: CommandId = SECOND_COMMAND_ID;
    }

    enum CalculatorKind {}

    impl RemoteKind for CalculatorKind {
        const OBJECT_KIND: ObjectKindTag = CALCULATOR_OBJECT_KIND.tag;
        const NAME: &'static str = CALCULATOR_OBJECT_KIND.name;
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum CodecError {
        InvalidRoute,
        InvalidPayload,
        OperationFailureUnsupported,
        UnsupportedDirection,
    }

    impl Display for CodecError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            write!(formatter, "test codec error: {self:?}")
        }
    }

    impl Error for CodecError {}

    #[derive(Clone, Copy)]
    struct TestCodec {
        decoded_call_offset: u64,
        result_route_offset: u64,
    }

    impl TestCodec {
        const fn valid() -> Self {
            Self {
                decoded_call_offset: 0,
                result_route_offset: 0,
            }
        }
    }

    impl WireCodec<Command<Add>> for TestCodec {
        type Error = CodecError;

        fn encode_parts(&self, message: &Command<Add>) -> Result<EncodedParts, Self::Error> {
            Ok(EncodedParts::new(
                FrameRoute::Command {
                    call_id: message.call_id(),
                    target: message.object_id(),
                    method: Add::METHOD,
                },
                message.params().to_be_bytes().to_vec(),
                Vec::new(),
            ))
        }

        fn decode_frame(&self, frame: WireFrame) -> Result<Command<Add>, Self::Error> {
            let FrameRoute::Command {
                call_id,
                target,
                method,
            } = frame.route()
            else {
                return Err(CodecError::InvalidRoute);
            };
            if method != Add::METHOD || frame.payload().len() != 4 {
                return Err(CodecError::InvalidPayload);
            }
            let value = u32::from_be_bytes(
                frame
                    .payload()
                    .try_into()
                    .map_err(|_| CodecError::InvalidPayload)?,
            );
            Ok(Command::new(
                CallId::new(call_id.get() + self.decoded_call_offset),
                target,
                CallMetadata::default(),
                value,
            ))
        }
    }

    impl WireCodec<CommandResult<Add>> for TestCodec {
        type Error = CodecError;

        fn encode_parts(&self, message: &CommandResult<Add>) -> Result<EncodedParts, Self::Error> {
            let value = message
                .outcome()
                .as_ref()
                .map_err(|_| CodecError::OperationFailureUnsupported)?;
            Ok(EncodedParts::new(
                FrameRoute::Result {
                    call_id: CallId::new(message.call_id().get() + self.result_route_offset),
                },
                value.to_be_bytes().to_vec(),
                Vec::new(),
            ))
        }

        fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<Add>, Self::Error> {
            let FrameRoute::Result { call_id } = frame.route() else {
                return Err(CodecError::InvalidRoute);
            };
            if frame.payload().len() != 4 {
                return Err(CodecError::InvalidPayload);
            }
            let value = u32::from_be_bytes(
                frame
                    .payload()
                    .try_into()
                    .map_err(|_| CodecError::InvalidPayload)?,
            );
            Ok(CommandResult::new(call_id, Ok(value)))
        }
    }

    #[derive(Clone)]
    struct HandshakeCodec {
        client: ClientHello,
        server_route: FrameRoute,
    }

    impl WireCodec<ClientHello> for HandshakeCodec {
        type Error = CodecError;

        fn encode_parts(&self, _message: &ClientHello) -> Result<EncodedParts, Self::Error> {
            Err(CodecError::UnsupportedDirection)
        }

        fn decode_frame(&self, frame: WireFrame) -> Result<ClientHello, Self::Error> {
            if frame.route() != FrameRoute::Handshake || !frame.attachments().is_empty() {
                return Err(CodecError::InvalidRoute);
            }
            Ok(self.client.clone())
        }
    }

    impl WireCodec<ServerHello> for HandshakeCodec {
        type Error = CodecError;

        fn encode_parts(&self, message: &ServerHello) -> Result<EncodedParts, Self::Error> {
            Ok(EncodedParts::new(
                self.server_route,
                vec![message.selected_version().minor as u8],
                Vec::new(),
            ))
        }

        fn decode_frame(&self, _frame: WireFrame) -> Result<ServerHello, Self::Error> {
            Err(CodecError::UnsupportedDirection)
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct AuthorizationError;

    impl Display for AuthorizationError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("test authorization rejected")
        }
    }

    impl Error for AuthorizationError {}

    struct RecordingGate {
        accepted: bool,
        calls: Arc<AtomicUsize>,
    }

    impl ServerHandshakeGate for RecordingGate {
        type Authorization = &'static str;
        type Error = AuthorizationError;
        type Future<'a> = Ready<Result<Self::Authorization, Self::Error>>;

        fn authorize<'a>(&'a mut self, _hello: &'a ClientHello) -> Self::Future<'a> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ready(if self.accepted {
                Ok("test-principal")
            } else {
                Err(AuthorizationError)
            })
        }
    }

    struct PendingGate;

    impl ServerHandshakeGate for PendingGate {
        type Authorization = ();
        type Error = AuthorizationError;
        type Future<'a> = Pending<Result<Self::Authorization, Self::Error>>;

        fn authorize<'a>(&'a mut self, _hello: &'a ClientHello) -> Self::Future<'a> {
            pending()
        }
    }

    fn frame_limits() -> FrameLimits {
        FrameLimits::new(64, 0, 0, 64)
    }

    fn handshake_limits() -> FrameLimits {
        FrameLimits::new(256, 0, 0, 256)
    }

    fn protocol(codec: TestCodec) -> WireProtocol<TestCodec> {
        WireProtocol::new(
            codec,
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("version policy should build"),
            ProtocolVersion::new(1, 0),
            frame_limits(),
        )
        .expect("wire protocol should build")
    }

    fn command_frame(codec: TestCodec, call_id: u64, value: u32) -> WireFrame {
        protocol(codec)
            .encode(&Command::<Add>::new(
                CallId::new(call_id),
                ObjectId::new(2),
                CallMetadata::default(),
                value,
            ))
            .expect("command should encode")
    }

    #[test]
    fn omitted_control_plane_preserves_an_unrecognized_frame() {
        // Arrange
        let frame = command_frame(TestCodec::valid(), 7, 41);
        let expected = frame.clone();
        let mut controls = NoServerControl;

        // Act
        let result = block_on(controls.handle_control(frame));

        // Assert
        assert_eq!(result, Ok(ServerFrameDisposition::Unhandled(expected)));
    }

    #[test]
    fn omitted_control_plane_has_no_transport_cleanup_event() {
        // Arrange
        let mut controls = NoServerControl;

        // Act
        let result = block_on(controls.handle_transport_closed());

        // Assert
        assert_eq!(result, Ok(None));
    }

    fn supervisor_resources(
        profile: ServerSupervisorProductionProfile,
    ) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let pending = requirements.class(ResourceClass::PendingCall);
        ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::PendingCall,
                pending.global,
                pending.per_scope,
            ),
            requirements,
        )
        .expect("exact supervisor production profile should fit")
    }

    fn handshake_resources(profile: ServerHandshakeProductionProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let wire = requirements.class(ResourceClass::WireBytes);
        ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::WireBytes,
                wire.global,
                wire.per_scope,
            ),
            requirements,
        )
        .expect("exact handshake production profile should fit")
    }

    fn versions(minimum: u16, maximum: u16) -> VersionPolicy {
        VersionPolicy::new(
            ProtocolVersion::new(1, minimum),
            ProtocolVersion::new(1, maximum),
        )
        .expect("test version policy should build")
    }

    fn negotiation_limits() -> NegotiationLimits {
        NegotiationLimits::new(8, 64)
    }

    fn client_hello(minimum: u16, maximum: u16) -> ClientHello {
        ClientHello::new(
            versions(minimum, maximum),
            handshake_limits(),
            CapabilityRequest::new(Vec::new(), Vec::new(), negotiation_limits())
                .expect("empty capability request should build"),
        )
    }

    fn server_config(minimum: u16, maximum: u16) -> ServerHandshakeConfig {
        ServerHandshakeConfig::new(
            versions(minimum, maximum),
            handshake_limits(),
            CapabilityManifest::new(Vec::new(), negotiation_limits())
                .expect("empty capability manifest should build"),
        )
    }

    fn handshake_frame(version: ProtocolVersion) -> WireFrame {
        WireFrame::new(
            version,
            FrameRoute::Handshake,
            Vec::new(),
            Vec::new(),
            frame_limits(),
        )
        .expect("handshake frame should build")
    }

    struct AddService {
        response_call_offset: u64,
    }

    struct PendingAddService;

    impl Service<Command<Add>> for PendingAddService {
        type Response = CommandResult<Add>;
        type Error = Infallible;
        type Future<'a> = Pending<Result<Self::Response, Self::Error>>;

        fn call(&self, _command: Command<Add>) -> Self::Future<'_> {
            pending()
        }
    }

    struct ReleaseState {
        released: AtomicBool,
        waker: Mutex<Option<Waker>>,
    }

    struct ReleasedAddFuture {
        state: Arc<ReleaseState>,
        call_id: CallId,
        value: u32,
    }

    impl Future for ReleasedAddFuture {
        type Output = Result<CommandResult<Add>, Infallible>;

        fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
            if self.state.released.load(Ordering::SeqCst) {
                return Poll::Ready(Ok(CommandResult::new(self.call_id, Ok(self.value + 1))));
            }
            *self
                .state
                .waker
                .lock()
                .expect("release waker should be available") = Some(context.waker().clone());
            Poll::Pending
        }
    }

    struct ControlReleasedAddService {
        state: Arc<ReleaseState>,
    }

    impl Service<Command<Add>> for ControlReleasedAddService {
        type Response = CommandResult<Add>;
        type Error = Infallible;
        type Future<'a> = ReleasedAddFuture;

        fn call(&self, command: Command<Add>) -> Self::Future<'_> {
            ReleasedAddFuture {
                state: Arc::clone(&self.state),
                call_id: command.call_id(),
                value: command.into_params(),
            }
        }
    }

    struct ReleaseControl {
        state: Arc<ReleaseState>,
    }

    impl ServerControlFrameHandler for ReleaseControl {
        type Event = &'static str;
        type Error = Infallible;
        type Future<'a> = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>;
        type CloseFuture<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

        fn handle_control<'a>(&'a mut self, frame: WireFrame) -> Self::Future<'a> {
            if frame.route() != FrameRoute::Control {
                return ready(Ok(ServerFrameDisposition::Unhandled(frame)));
            }
            self.state.released.store(true, Ordering::SeqCst);
            if let Some(waker) = self
                .state
                .waker
                .lock()
                .expect("release waker should be available")
                .take()
            {
                waker.wake();
            }
            ready(Ok(ServerFrameDisposition::Handled("released")))
        }

        fn releases_frame_before_pending(&self) -> bool {
            true
        }

        fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
            ready(Ok(None))
        }
    }

    struct CloseReleaseControl {
        state: Arc<ReleaseState>,
    }

    impl ServerControlFrameHandler for CloseReleaseControl {
        type Event = &'static str;
        type Error = Infallible;
        type Future<'a> = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>;
        type CloseFuture<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

        fn handle_control<'a>(&'a mut self, frame: WireFrame) -> Self::Future<'a> {
            ready(Ok(ServerFrameDisposition::Unhandled(frame)))
        }

        fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
            self.state.released.store(true, Ordering::SeqCst);
            if let Some(waker) = self
                .state
                .waker
                .lock()
                .expect("release waker should be available")
                .take()
            {
                waker.wake();
            }
            ready(Ok(Some("transport closed")))
        }
    }

    struct PreserveControl;

    impl ServerControlFrameHandler for PreserveControl {
        type Event = ();
        type Error = Infallible;
        type Future<'a> = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>;
        type CloseFuture<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

        fn handle_control<'a>(&'a mut self, frame: WireFrame) -> Self::Future<'a> {
            ready(Ok(ServerFrameDisposition::Unhandled(frame)))
        }

        fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
            ready(Ok(None))
        }
    }

    struct CountingCloseControl {
        close_calls: Arc<AtomicUsize>,
    }

    impl ServerControlFrameHandler for CountingCloseControl {
        type Event = ();
        type Error = Infallible;
        type Future<'a> = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>;
        type CloseFuture<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

        fn handle_control<'a>(&'a mut self, frame: WireFrame) -> Self::Future<'a> {
            ready(Ok(ServerFrameDisposition::Unhandled(frame)))
        }

        fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
            self.close_calls.fetch_add(1, Ordering::SeqCst);
            ready(Ok(None))
        }
    }

    struct PeerGracefulCloseControl;

    impl ServerControlFrameHandler for PeerGracefulCloseControl {
        type Event = &'static str;
        type Error = Infallible;
        type Future<'a> = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>;
        type CloseFuture<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

        fn handle_control<'a>(&'a mut self, _frame: WireFrame) -> Self::Future<'a> {
            ready(Ok(ServerFrameDisposition::Handled("close acknowledged")))
        }

        fn terminal_close_reason(&self) -> Option<ServerSupervisorCloseReason> {
            Some(ServerSupervisorCloseReason::PeerGracefulClose)
        }

        fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
            ready(Ok(None))
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum TestControlError {
        Handle,
        Cleanup,
    }

    impl Display for TestControlError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            write!(formatter, "test control error: {self:?}")
        }
    }

    impl Error for TestControlError {}

    struct TrackingTerminationControl {
        reasons: Arc<Mutex<Vec<ServerSupervisorCloseReason>>>,
        fail_handle: bool,
        fail_cleanup: bool,
    }

    impl TrackingTerminationControl {
        fn new(reasons: Arc<Mutex<Vec<ServerSupervisorCloseReason>>>) -> Self {
            Self {
                reasons,
                fail_handle: false,
                fail_cleanup: false,
            }
        }
    }

    impl ServerControlFrameHandler for TrackingTerminationControl {
        type Event = ();
        type Error = TestControlError;
        type Future<'a> = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>;
        type CloseFuture<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

        fn handle_control<'a>(&'a mut self, frame: WireFrame) -> Self::Future<'a> {
            if self.fail_handle {
                ready(Err(TestControlError::Handle))
            } else {
                ready(Ok(ServerFrameDisposition::Unhandled(frame)))
            }
        }

        fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
            ready(Ok(None))
        }

        fn handle_connection_terminated<'a>(
            &'a mut self,
            reason: ServerSupervisorCloseReason,
        ) -> Self::CloseFuture<'a> {
            self.reasons
                .lock()
                .expect("termination reasons should be available")
                .push(reason);
            if self.fail_cleanup {
                ready(Err(TestControlError::Cleanup))
            } else {
                ready(Ok(None))
            }
        }
    }

    struct YieldOnceClose {
        yielded: bool,
    }

    impl Future for YieldOnceClose {
        type Output = Result<Option<()>, Infallible>;

        fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
            if self.yielded {
                Poll::Ready(Ok(Some(())))
            } else {
                self.yielded = true;
                context.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    struct YieldingCloseControl {
        started: Arc<AtomicBool>,
    }

    impl ServerControlFrameHandler for YieldingCloseControl {
        type Event = ();
        type Error = Infallible;
        type Future<'a> = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>;
        type CloseFuture<'a> = YieldOnceClose;

        fn handle_control<'a>(&'a mut self, frame: WireFrame) -> Self::Future<'a> {
            ready(Ok(ServerFrameDisposition::Unhandled(frame)))
        }

        fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
            self.started.store(true, Ordering::SeqCst);
            YieldOnceClose { yielded: false }
        }
    }

    struct SwitchCloseFuture {
        ready: Arc<AtomicBool>,
    }

    impl Future for SwitchCloseFuture {
        type Output = Result<Option<()>, Infallible>;

        fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
            if self.ready.load(Ordering::SeqCst) {
                Poll::Ready(Ok(None))
            } else {
                Poll::Pending
            }
        }
    }

    struct RetryableCloseControl {
        calls: Arc<AtomicUsize>,
        ready: Arc<AtomicBool>,
    }

    impl ServerControlFrameHandler for RetryableCloseControl {
        type Event = ();
        type Error = Infallible;
        type Future<'a> = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>;
        type CloseFuture<'a> = SwitchCloseFuture;

        fn handle_control<'a>(&'a mut self, frame: WireFrame) -> Self::Future<'a> {
            ready(Ok(ServerFrameDisposition::Unhandled(frame)))
        }

        fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            SwitchCloseFuture {
                ready: Arc::clone(&self.ready),
            }
        }
    }

    struct PendingControlState {
        started: AtomicBool,
        dropped: AtomicBool,
        closed: AtomicBool,
    }

    struct TrackedPendingControlFuture {
        state: Arc<PendingControlState>,
    }

    impl Future for TrackedPendingControlFuture {
        type Output = Result<ServerFrameDisposition<&'static str>, Infallible>;

        fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
            self.state.started.store(true, Ordering::SeqCst);
            context.waker().wake_by_ref();
            Poll::Pending
        }
    }

    impl Drop for TrackedPendingControlFuture {
        fn drop(&mut self) {
            self.state.dropped.store(true, Ordering::SeqCst);
        }
    }

    struct EofCompensationControl {
        state: Arc<PendingControlState>,
    }

    impl ServerControlFrameHandler for EofCompensationControl {
        type Event = &'static str;
        type Error = Infallible;
        type Future<'a> = TrackedPendingControlFuture;
        type CloseFuture<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

        fn handle_control<'a>(&'a mut self, _frame: WireFrame) -> Self::Future<'a> {
            TrackedPendingControlFuture {
                state: Arc::clone(&self.state),
            }
        }

        fn releases_frame_before_pending(&self) -> bool {
            true
        }

        fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
            self.state.closed.store(true, Ordering::SeqCst);
            ready(Ok(Some("transport closed")))
        }
    }

    struct YieldingEofReceive {
        value: Option<Option<WireFrame>>,
        yield_once: bool,
    }

    impl Future for YieldingEofReceive {
        type Output = Result<Option<WireFrame>, Infallible>;

        fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
            if self.yield_once {
                self.yield_once = false;
                context.waker().wake_by_ref();
                return Poll::Pending;
            }
            Poll::Ready(Ok(self.value.take().unwrap_or(None)))
        }
    }

    struct YieldingEofTransport {
        first: Mutex<Option<WireFrame>>,
        receives: AtomicUsize,
    }

    impl YieldingEofTransport {
        fn new(first: WireFrame) -> Self {
            Self {
                first: Mutex::new(Some(first)),
                receives: AtomicUsize::new(0),
            }
        }
    }

    impl FrameTransport for YieldingEofTransport {
        type Error = Infallible;
        type Send<'a> = Ready<Result<(), Self::Error>>;
        type Receive<'a> = YieldingEofReceive;

        fn send(&self, _frame: WireFrame) -> Self::Send<'_> {
            ready(Ok(()))
        }

        fn receive(&self) -> Self::Receive<'_> {
            let index = self.receives.fetch_add(1, Ordering::SeqCst);
            YieldingEofReceive {
                value: if index == 0 {
                    Some(
                        self.first
                            .lock()
                            .expect("first frame should be available")
                            .take(),
                    )
                } else {
                    Some(None)
                },
                yield_once: index != 0,
            }
        }
    }

    struct CountingDeadline {
        polls: Arc<AtomicUsize>,
    }

    impl Future for CountingDeadline {
        type Output = ();

        fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
            self.polls.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(())
        }
    }

    impl Service<Command<Add>> for AddService {
        type Response = CommandResult<Add>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<Add>) -> Self::Future<'_> {
            ready(Ok(CommandResult::new(
                CallId::new(command.call_id().get() + self.response_call_offset),
                Ok(command.into_params() + 1),
            )))
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TransportError;

    impl Display for TransportError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("test transport error")
        }
    }

    impl Error for TransportError {}

    struct MemoryTransport {
        incoming: Mutex<VecDeque<Result<Option<WireFrame>, TransportError>>>,
        sent: Mutex<Vec<WireFrame>>,
        reject_send: bool,
    }

    impl MemoryTransport {
        fn new(
            incoming: impl IntoIterator<Item = Result<Option<WireFrame>, TransportError>>,
        ) -> Self {
            Self {
                incoming: Mutex::new(incoming.into_iter().collect()),
                sent: Mutex::new(Vec::new()),
                reject_send: false,
            }
        }

        fn rejecting_send() -> Self {
            Self {
                incoming: Mutex::new(VecDeque::new()),
                sent: Mutex::new(Vec::new()),
                reject_send: true,
            }
        }
    }

    impl FrameTransport for MemoryTransport {
        type Error = TransportError;
        type Send<'a> = Ready<Result<(), Self::Error>>;
        type Receive<'a> = Ready<Result<Option<WireFrame>, Self::Error>>;

        fn send(&self, frame: WireFrame) -> Self::Send<'_> {
            if self.reject_send {
                return ready(Err(TransportError));
            }
            self.sent
                .lock()
                .expect("sent frames should be available")
                .push(frame);
            ready(Ok(()))
        }

        fn receive(&self) -> Self::Receive<'_> {
            ready(
                self.incoming
                    .lock()
                    .expect("incoming frames should be available")
                    .pop_front()
                    .unwrap_or(Ok(None)),
            )
        }
    }

    impl ShutdownFrameTransport for MemoryTransport {
        type Shutdown<'a> = Ready<Result<(), Self::Error>>;

        fn shutdown(&self) -> Self::Shutdown<'_> {
            if self.reject_send {
                ready(Err(TransportError))
            } else {
                ready(Ok(()))
            }
        }
    }

    struct PendingReceiveTransport;

    impl FrameTransport for PendingReceiveTransport {
        type Error = Infallible;
        type Send<'a> = Ready<Result<(), Self::Error>>;
        type Receive<'a> = Pending<Result<Option<WireFrame>, Self::Error>>;

        fn send(&self, _frame: WireFrame) -> Self::Send<'_> {
            ready(Ok(()))
        }

        fn receive(&self) -> Self::Receive<'_> {
            pending()
        }
    }

    struct PendingShutdownTransport {
        shutdowns: Arc<AtomicUsize>,
        fail_shutdown: bool,
    }

    impl FrameTransport for PendingShutdownTransport {
        type Error = TransportError;
        type Send<'a> = Ready<Result<(), Self::Error>>;
        type Receive<'a> = Pending<Result<Option<WireFrame>, Self::Error>>;

        fn send(&self, _frame: WireFrame) -> Self::Send<'_> {
            ready(Ok(()))
        }

        fn receive(&self) -> Self::Receive<'_> {
            pending()
        }
    }

    impl ShutdownFrameTransport for PendingShutdownTransport {
        type Shutdown<'a> = Ready<Result<(), Self::Error>>;

        fn shutdown(&self) -> Self::Shutdown<'_> {
            self.shutdowns.fetch_add(1, Ordering::SeqCst);
            if self.fail_shutdown {
                ready(Err(TransportError))
            } else {
                ready(Ok(()))
            }
        }
    }

    fn block_on<Output>(future: impl std::future::Future<Output = Output>) -> Output {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => output,
            std::task::Poll::Pending => panic!("test future should be ready"),
        }
    }

    struct PreserveHandler;

    impl<Transport> ServerFrameHandler<Transport> for PreserveHandler {
        type Event = ();
        type Error = Infallible;
        type Future<'a>
            = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>
        where
            Self: 'a,
            Transport: 'a;

        fn handle<'a>(
            &'a mut self,
            frame: WireFrame,
            _transport: &'a Transport,
        ) -> Self::Future<'a> {
            ready(Ok(ServerFrameDisposition::Unhandled(frame)))
        }
    }

    struct ConcurrentRouteHandler {
        method: MethodTag,
        event: &'static str,
    }

    impl ConcurrentServerFrameHandler<MemoryTransport> for ConcurrentRouteHandler {
        type Event = &'static str;
        type Error = Infallible;
        type Future = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>;

        fn handle_concurrently(
            &self,
            frame: WireFrame,
            _transport: Arc<MemoryTransport>,
        ) -> Self::Future {
            if matches!(frame.route(), FrameRoute::Command { method, .. } if method == self.method)
            {
                ready(Ok(ServerFrameDisposition::Handled(self.event)))
            } else {
                ready(Ok(ServerFrameDisposition::Unhandled(frame)))
            }
        }
    }

    struct QueuedOutbound {
        events: Mutex<VecDeque<Option<&'static str>>>,
    }

    impl QueuedOutbound {
        fn new(events: impl IntoIterator<Item = Option<&'static str>>) -> Self {
            Self {
                events: Mutex::new(events.into_iter().collect()),
            }
        }
    }

    impl ConcurrentServerOutbound for QueuedOutbound {
        type Event = &'static str;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

        fn send_once(&self) -> Self::Future<'_> {
            ready(Ok(self
                .events
                .lock()
                .expect("outbound events should be available")
                .pop_front()
                .flatten()))
        }
    }

    struct QueuedBackend {
        events: VecDeque<Result<&'static str, BackendError>>,
        polls: Arc<AtomicUsize>,
    }

    impl QueuedBackend {
        fn new(
            events: impl IntoIterator<Item = Result<&'static str, BackendError>>,
            polls: Arc<AtomicUsize>,
        ) -> Self {
            Self {
                events: events.into_iter().collect(),
                polls,
            }
        }
    }

    impl ConcurrentServerBackend for QueuedBackend {
        type Event = &'static str;
        type Error = BackendError;

        fn poll_drive_once(
            &mut self,
            _context: &mut Context<'_>,
        ) -> Poll<Result<Self::Event, Self::Error>> {
            self.polls.fetch_add(1, Ordering::SeqCst);
            self.events.pop_front().map_or(Poll::Pending, Poll::Ready)
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct BackendError;

    impl Display for BackendError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("test backend error")
        }
    }

    impl Error for BackendError {}

    struct PendingTrackingBackend {
        polls: Arc<AtomicUsize>,
        terminations: Arc<Mutex<Vec<ServerSupervisorCloseReason>>>,
    }

    impl ConcurrentServerBackend for PendingTrackingBackend {
        type Event = ();
        type Error = Infallible;

        fn poll_drive_once(
            &mut self,
            _context: &mut Context<'_>,
        ) -> Poll<Result<Self::Event, Self::Error>> {
            self.polls.fetch_add(1, Ordering::SeqCst);
            Poll::Pending
        }

        fn handle_connection_terminated(&mut self, reason: ServerSupervisorCloseReason) {
            self.terminations
                .lock()
                .expect("backend terminations should be available")
                .push(reason);
        }
    }

    #[test]
    fn backend_chain_rotates_priority_between_ready_plugins() {
        // Arrange
        let left_polls = Arc::new(AtomicUsize::new(0));
        let right_polls = Arc::new(AtomicUsize::new(0));
        let left = QueuedBackend::new([Ok("left-1"), Ok("left-2")], Arc::clone(&left_polls));
        let right = QueuedBackend::new([Ok("right-1"), Ok("right-2")], Arc::clone(&right_polls));
        let mut chain = left.and_backend(right);
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let outcomes = (0..4)
            .map(|_| chain.poll_drive_once(&mut context))
            .collect::<Vec<_>>();

        // Assert
        assert!(matches!(
            outcomes.as_slice(),
            [
                Poll::Ready(Ok(Either::Left("left-1"))),
                Poll::Ready(Ok(Either::Right("right-1"))),
                Poll::Ready(Ok(Either::Left("left-2"))),
                Poll::Ready(Ok(Either::Right("right-2")))
            ]
        ));
        assert_eq!(left_polls.load(Ordering::SeqCst), 2);
        assert_eq!(right_polls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn backend_chain_forwards_terminal_reason_to_every_plugin() {
        // Arrange
        let left_terminations = Arc::new(Mutex::new(Vec::new()));
        let right_terminations = Arc::new(Mutex::new(Vec::new()));
        let left = PendingTrackingBackend {
            polls: Arc::new(AtomicUsize::new(0)),
            terminations: Arc::clone(&left_terminations),
        };
        let right = PendingTrackingBackend {
            polls: Arc::new(AtomicUsize::new(0)),
            terminations: Arc::clone(&right_terminations),
        };
        let mut chain = left.and_backend(right);

        // Act
        chain.handle_connection_terminated(ServerSupervisorCloseReason::BackendEventFailed);

        // Assert
        assert_eq!(
            *left_terminations
                .lock()
                .expect("left termination should be available"),
            vec![ServerSupervisorCloseReason::BackendEventFailed]
        );
        assert_eq!(
            *right_terminations
                .lock()
                .expect("right termination should be available"),
            vec![ServerSupervisorCloseReason::BackendEventFailed]
        );
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct OutboundError;

    impl Display for OutboundError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("test outbound error")
        }
    }

    impl Error for OutboundError {}

    struct FailingOutbound;

    impl ConcurrentServerOutbound for FailingOutbound {
        type Event = ();
        type Error = OutboundError;
        type Future<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

        fn send_once(&self) -> Self::Future<'_> {
            ready(Err(OutboundError))
        }
    }

    struct IdleOutbound {
        calls: Arc<AtomicUsize>,
    }

    impl ConcurrentServerOutbound for IdleOutbound {
        type Event = ();
        type Error = Infallible;
        type Future<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

        fn send_once(&self) -> Self::Future<'_> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ready(Ok(None))
        }
    }

    struct PendingOutbound {
        calls: Arc<AtomicUsize>,
    }

    impl ConcurrentServerOutbound for PendingOutbound {
        type Event = ();
        type Error = Infallible;
        type Future<'a> = Pending<Result<Option<Self::Event>, Self::Error>>;

        fn send_once(&self) -> Self::Future<'_> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            pending()
        }
    }

    #[derive(Default)]
    struct WakeableOutboundState {
        queued: bool,
        waiter: Option<Waker>,
    }

    struct WakeableOutbound {
        state: Mutex<WakeableOutboundState>,
    }

    impl WakeableOutbound {
        fn new() -> Self {
            Self {
                state: Mutex::new(WakeableOutboundState::default()),
            }
        }

        fn enqueue(&self) {
            let waiter = {
                let mut state = self
                    .state
                    .lock()
                    .expect("outbound state should be available");
                state.queued = true;
                state.waiter.take()
            };
            if let Some(waiter) = waiter {
                waiter.wake();
            }
        }
    }

    impl ConcurrentServerOutbound for WakeableOutbound {
        type Event = &'static str;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

        fn send_once(&self) -> Self::Future<'_> {
            let mut state = self
                .state
                .lock()
                .expect("outbound state should be available");
            if state.queued {
                state.queued = false;
                ready(Ok(Some("event")))
            } else {
                ready(Ok(None))
            }
        }

        fn wait_until_ready(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
            Box::pin(poll_fn(|context| {
                let mut state = self
                    .state
                    .lock()
                    .expect("outbound state should be available");
                if state.queued {
                    Poll::Ready(())
                } else {
                    state.waiter = Some(context.waker().clone());
                    Poll::Pending
                }
            }))
        }
    }

    #[test]
    fn installed_handler_builds_its_capability_handshake_manifest() {
        // Arrange
        let handler = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let capability = WireCapabilityId::new("test.add", 64).expect("ID should build");

        // Act
        let config = handler
            .handshake_config(versions(0, 2), handshake_limits(), negotiation_limits())
            .expect("complete installed capability should produce config");

        // Assert
        assert_eq!(
            config.capabilities().support(&capability),
            Some(
                VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 3))
                    .expect("range should build")
            )
        );
    }

    #[test]
    fn installed_event_builds_capability_manifest_and_static_route_metadata() {
        // Arrange
        let mut inventory = ServerCapabilityInventory::with_object_kinds(CALCULATOR_OBJECT_KINDS)
            .expect("calculator catalog should be valid");
        inventory
            .register_event::<CalculatorChanged>()
            .expect("event metadata should register");
        let capability = WireCapabilityId::new("test.calculator-events", 64)
            .expect("capability ID should build");

        // Act
        let manifest = inventory
            .manifest(negotiation_limits())
            .expect("complete event capability should advertise");

        // Assert
        assert!(manifest.support(&capability).is_some());
        let descriptor = inventory
            .event_for_tag(CalculatorChanged::EVENT)
            .expect("event tag should resolve");
        assert_eq!(descriptor.event(), CHANGED_EVENT_ID);
        assert_eq!(descriptor.target_object_kinds(), CALCULATOR_TARGETS);
    }

    #[test]
    fn handshake_manifest_preflights_command_targets_against_installed_object_kinds() {
        // Arrange
        let handler = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let empty_catalog = ObjectKindCatalog::new(&[]);

        // Act
        let unavailable = handler
            .handshake_config_with_object_kinds(
                versions(0, 2),
                handshake_limits(),
                negotiation_limits(),
                empty_catalog,
            )
            .expect_err("an unavailable command target must block advertisement");
        let available = handler.handshake_config_with_object_kinds(
            versions(0, 2),
            handshake_limits(),
            negotiation_limits(),
            CALCULATOR_OBJECT_KINDS,
        );
        let mut inventory = ServerCapabilityInventory::with_object_kinds(CALCULATOR_OBJECT_KINDS)
            .expect("calculator catalog should be valid");
        handler
            .register_capabilities(&mut inventory)
            .expect("handler metadata should register");
        let route = inventory
            .command_for_method(Add::METHOD)
            .expect("method should resolve to static command metadata");

        // Assert
        assert_eq!(
            unavailable,
            ServerCapabilityInventoryError::CommandTargetKindUnavailable {
                command: ADD_COMMAND_ID,
                target: CALCULATOR_OBJECT_KIND_ID,
            }
        );
        assert!(available.is_ok());
        assert_eq!(route.command(), ADD_COMMAND_ID);
        assert_eq!(route.method(), Add::METHOD);
        assert_eq!(route.targets(), CALCULATOR_TARGETS);
    }

    #[test]
    fn remotely_advertised_command_requires_target_object_metadata() {
        // Arrange
        let mut inventory = ServerCapabilityInventory::new();

        // Act
        let error = inventory
            .register::<UntargetedAdd>()
            .expect_err("untargeted remote command must fail");

        // Assert
        assert_eq!(
            error,
            ServerCapabilityInventoryError::CommandTargetsMissing {
                command: ADD_COMMAND_ID,
            }
        );
    }

    #[test]
    fn incomplete_capability_cannot_be_advertised() {
        // Arrange
        let mut inventory = ServerCapabilityInventory::new();
        inventory
            .register::<PartialCommand>()
            .expect("present command should register");

        // Act
        let error = inventory
            .manifest(negotiation_limits())
            .expect_err("missing handler must reject manifest");

        // Assert
        assert_eq!(
            error,
            ServerCapabilityInventoryError::IncompleteCapability {
                capability: PARTIAL_CAPABILITY_ID,
                missing: vec![MISSING_COMMAND_ID],
            }
        );
    }

    #[test]
    fn duplicate_wire_method_cannot_enter_the_handler_inventory() {
        // Arrange
        let mut inventory = ServerCapabilityInventory::new();
        inventory
            .register::<Add>()
            .expect("first command should register");

        // Act
        let error = inventory
            .register::<ConflictingMethod>()
            .expect_err("duplicate method must fail");

        // Assert
        assert_eq!(
            error,
            ServerCapabilityInventoryError::DuplicateMethodHandler {
                method: Add::METHOD,
                existing: ADD_COMMAND_ID,
                requested: CONFLICT_COMMAND_ID,
            }
        );
    }

    #[test]
    fn concurrent_handler_chain_builds_a_deterministic_composed_manifest() {
        // Arrange
        let first =
            ConcurrentTypedCommandHandler::<Add, _, _>::new(protocol(TestCodec::valid()), ());
        let second = ConcurrentTypedCommandHandler::<SecondCommand, _, _>::new(
            protocol(TestCodec::valid()),
            (),
        );
        let chain = super::ConcurrentServerHandlerChain::new(first, second);

        // Act
        let manifest = chain
            .capability_manifest(negotiation_limits())
            .expect("complete handler chain should build manifest");

        // Assert
        let capabilities = manifest
            .iter()
            .map(|(id, version)| (id.as_str(), version))
            .collect::<Vec<_>>();
        assert_eq!(
            capabilities,
            vec![
                ("test.add", ProtocolVersion::new(1, 3)),
                ("test.second", ProtocolVersion::new(2, 1)),
            ]
        );
    }

    #[test]
    fn application_frame_is_rejected_before_handshake() {
        // Arrange
        let codec = HandshakeCodec {
            client: client_hello(0, 0),
            server_route: FrameRoute::Handshake,
        };
        let mut handler = HandshakeFirstHandler::new(codec, server_config(0, 1), PreserveHandler);
        let transport = MemoryTransport::new([]);
        let frame = command_frame(TestCodec::valid(), 7, 41);
        let route = frame.route();

        // Act
        let error = block_on(handler.handle(frame, &transport))
            .expect_err("application traffic before hello should fail");

        // Assert
        assert!(matches!(
            error,
            HandshakeFirstError::HandshakeRequired { actual } if actual == route
        ));
        assert!(handler.negotiated().is_none());
        assert!(
            transport
                .sent
                .lock()
                .expect("sent frames should be available")
                .is_empty()
        );
    }

    #[test]
    fn handshake_selects_version_sends_hello_and_unlocks_application_handler() {
        // Arrange
        let codec = HandshakeCodec {
            client: client_hello(0, 2),
            server_route: FrameRoute::Handshake,
        };
        let mut handler = HandshakeFirstHandler::new(codec, server_config(0, 1), PreserveHandler);
        let transport = MemoryTransport::new([]);

        // Act
        let established =
            block_on(handler.handle(handshake_frame(ProtocolVersion::new(99, 99)), &transport))
                .expect("compatible peers should negotiate");
        let application_frame = WireFrame::new(
            ProtocolVersion::new(1, 1),
            FrameRoute::Control,
            Vec::new(),
            Vec::new(),
            frame_limits(),
        )
        .expect("application frame should build");
        let application = block_on(handler.handle(application_frame.clone(), &transport))
            .expect("negotiated traffic should reach application handler");

        // Assert
        assert_eq!(
            established,
            ServerFrameDisposition::Handled(HandshakeFirstEvent::Established(
                HandshakeEstablished {
                    selected_version: ProtocolVersion::new(1, 1),
                    enabled_capabilities: 0,
                }
            ))
        );
        assert_eq!(
            application,
            ServerFrameDisposition::Unhandled(application_frame)
        );
        let sent = transport
            .sent
            .lock()
            .expect("sent frames should be available");
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].route(), FrameRoute::Handshake);
        assert_eq!(sent[0].version(), ProtocolVersion::new(1, 1));
        assert_eq!(sent[0].payload(), &[1]);
        assert_eq!(
            handler
                .negotiated()
                .expect("handler should retain negotiated state")
                .selected_version(),
            ProtocolVersion::new(1, 1)
        );
    }

    #[test]
    fn canonical_hello_codec_composes_with_handshake_first_handler() {
        // Arrange
        let codec = CanonicalHandshakeCodec::new(negotiation_limits());
        let client = client_hello(0, 2);
        let parts = codec
            .encode_parts(&client)
            .expect("client hello should encode");
        let frame = WireFrame::new(
            ProtocolVersion::new(99, 99),
            parts.route(),
            parts.payload().to_vec(),
            parts.attachments().to_vec(),
            frame_limits(),
        )
        .expect("client hello frame should build");
        let mut handler = HandshakeFirstHandler::new(codec, server_config(0, 1), PreserveHandler);
        let transport = MemoryTransport::new([]);

        // Act
        let outcome =
            block_on(handler.handle(frame, &transport)).expect("canonical hello should negotiate");

        // Assert
        assert!(matches!(
            outcome,
            ServerFrameDisposition::Handled(HandshakeFirstEvent::Established(
                HandshakeEstablished {
                    selected_version,
                    enabled_capabilities: 0,
                }
            )) if selected_version == ProtocolVersion::new(1, 1)
        ));
        let sent = transport
            .sent
            .lock()
            .expect("sent frames should be available");
        let server = <CanonicalHandshakeCodec as WireCodec<ServerHello>>::decode_frame(
            &codec,
            sent[0].clone(),
        )
        .expect("server hello should decode");
        assert_eq!(server.selected_version(), ProtocolVersion::new(1, 1));
    }

    #[test]
    fn handshake_production_profile_bounds_source_and_encoded_frame() {
        // Arrange / Act
        let profile = ServerHandshakeProductionProfile::new(256);
        let wire = profile.requirements().class(ResourceClass::WireBytes);

        // Assert
        assert_eq!(profile.maximum_peer_receive_frame_bytes(), 256);
        assert_eq!(profile.maximum_encoding_wire_bytes(), 512);
        assert_eq!(wire.global, 512);
        assert_eq!(wire.per_scope, 512);
        assert_eq!(profile.requirements().scopes, 1);
        assert_eq!(profile.requirements().leases, 1);
    }

    #[test]
    fn production_handshake_sends_a_frame_owned_by_the_declared_ledger() {
        // Arrange
        let profile = ServerHandshakeProductionProfile::new(256);
        let resources = handshake_resources(profile);
        let ledger = resources
            .claim(profile.requirements())
            .expect("handshake profile should be declared");
        let codec = HandshakeCodec {
            client: client_hello(0, 1),
            server_route: FrameRoute::Handshake,
        };
        let transport =
            MemoryTransport::new([Ok(Some(handshake_frame(ProtocolVersion::new(99, 99))))]);
        let mut supervisor = EncodedServerSupervisor::new_handshake_required(
            transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        )
        .with_production_handshake(profile, &resources)
        .expect("handshake production resources should install");
        let mut gate = RecordingGate {
            accepted: true,
            calls: Arc::new(AtomicUsize::new(0)),
        };

        // Act
        let accepted =
            block_on(supervisor.accept_handshake(&codec, &server_config(0, 1), &mut gate));

        // Assert
        assert!(accepted.is_ok());
        let sent = supervisor
            .transport()
            .sent
            .lock()
            .expect("sent frames should be available");
        assert_eq!(sent.len(), 1);
        assert!(sent[0].is_budgeted_by(&ledger));
        let wire = resources
            .metrics()
            .expect("resource metrics should load")
            .class(ResourceClass::WireBytes);
        assert_eq!(wire.current, 1);
        assert_eq!(wire.peak, 512);
    }

    #[test]
    fn accepted_handshake_retains_only_its_supervisors_resource_budget_identity() {
        // Arrange
        let limits = ServerSupervisorLimits::new(1).expect("limit should be valid");
        let profile = ServerSupervisorProductionProfile::new(limits);
        let resources = supervisor_resources(profile);
        let foreign_resources = supervisor_resources(profile);
        let codec = HandshakeCodec {
            client: client_hello(0, 1),
            server_route: FrameRoute::Handshake,
        };
        let transport =
            MemoryTransport::new([Ok(Some(handshake_frame(ProtocolVersion::new(99, 99))))]);
        let mut supervisor = EncodedServerSupervisor::new_handshake_required(transport, limits)
            .with_production_resources(profile, &resources)
            .expect("supervisor production resources should install");
        let mut gate = RecordingGate {
            accepted: true,
            calls: Arc::new(AtomicUsize::new(0)),
        };

        // Act
        let accepted =
            block_on(supervisor.accept_handshake(&codec, &server_config(0, 1), &mut gate))
                .expect("compatible handshake should be accepted");

        // Assert
        assert!(accepted.has_production_resource_budget());
        assert!(accepted.shares_production_resource_budget_with(&resources));
        assert!(!accepted.shares_production_resource_budget_with(&foreign_resources));
    }

    #[test]
    fn accepted_handshake_without_production_supervisor_resources_is_unbound() {
        // Arrange
        let limits = ServerSupervisorLimits::new(1).expect("limit should be valid");
        let codec = HandshakeCodec {
            client: client_hello(0, 1),
            server_route: FrameRoute::Handshake,
        };
        let transport =
            MemoryTransport::new([Ok(Some(handshake_frame(ProtocolVersion::new(99, 99))))]);
        let mut supervisor = EncodedServerSupervisor::new_handshake_required(transport, limits);
        let mut gate = RecordingGate {
            accepted: true,
            calls: Arc::new(AtomicUsize::new(0)),
        };
        let resources = supervisor_resources(ServerSupervisorProductionProfile::new(limits));

        // Act
        let accepted =
            block_on(supervisor.accept_handshake(&codec, &server_config(0, 1), &mut gate))
                .expect("compatible handshake should be accepted");

        // Assert
        assert!(!accepted.has_production_resource_budget());
        assert!(!accepted.shares_production_resource_budget_with(&resources));
    }

    #[test]
    fn production_handshake_rejects_an_unbounded_peer_receive_offer() {
        // Arrange
        let profile = ServerHandshakeProductionProfile::new(64);
        let resources = handshake_resources(profile);
        let client = ClientHello::new(
            versions(0, 1),
            FrameLimits::new(128, 0, 0, 128),
            CapabilityRequest::new(Vec::new(), Vec::new(), negotiation_limits())
                .expect("empty capability request should build"),
        );
        let codec = HandshakeCodec {
            client,
            server_route: FrameRoute::Handshake,
        };
        let transport =
            MemoryTransport::new([Ok(Some(handshake_frame(ProtocolVersion::new(99, 99))))]);
        let mut supervisor = EncodedServerSupervisor::new_handshake_required(
            transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        )
        .with_production_handshake(profile, &resources)
        .expect("handshake production resources should install");
        let gate_calls = Arc::new(AtomicUsize::new(0));
        let mut gate = RecordingGate {
            accepted: true,
            calls: Arc::clone(&gate_calls),
        };

        // Act
        let error = block_on(supervisor.accept_handshake(&codec, &server_config(0, 1), &mut gate));

        // Assert
        assert!(matches!(
            error,
            Err(super::ServerHandshakeError::PeerReceiveFrameLimitExceeded {
                advertised: 128,
                maximum: 64,
            })
        ));
        assert_eq!(gate_calls.load(Ordering::SeqCst), 0);
        assert_eq!(supervisor.phase(), ServerSupervisorPhase::Closed);
        assert_eq!(
            supervisor.metrics().close_reason(),
            Some(ServerSupervisorCloseReason::HandshakeFailed)
        );
    }

    #[test]
    fn supervisor_handshake_authorizes_then_unlocks_validated_application_work() {
        // Arrange
        let codec = HandshakeCodec {
            client: client_hello(0, 1),
            server_route: FrameRoute::Handshake,
        };
        let transport = MemoryTransport::new([
            Ok(Some(handshake_frame(ProtocolVersion::new(99, 99)))),
            Ok(Some(command_frame(TestCodec::valid(), 7, 41))),
            Ok(None),
        ]);
        let mut supervisor = EncodedServerSupervisor::new_handshake_required(
            transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let gate_calls = Arc::new(AtomicUsize::new(0));
        let mut gate = RecordingGate {
            accepted: true,
            calls: Arc::clone(&gate_calls),
        };

        // Act
        let accepted =
            block_on(supervisor.accept_handshake(&codec, &server_config(0, 0), &mut gate))
                .expect("authorized compatible hello should establish");
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let release = Arc::new(ReleaseState {
            released: AtomicBool::new(false),
            waker: Mutex::new(None),
        });
        let mut controls = ReleaseControl { state: release };
        let mut events = Vec::new();
        let closed = block_on(supervisor.run(&commands, &mut controls, |event| {
            events.push(event);
            Ok::<_, Infallible>(())
        }));

        // Assert
        assert_eq!(accepted.authorization(), &"test-principal");
        assert_eq!(
            accepted.hello().selected_version(),
            ProtocolVersion::new(1, 0)
        );
        assert_eq!(gate_calls.load(Ordering::SeqCst), 1);
        assert!(matches!(
            closed,
            Ok(ServerSupervisorCloseReason::EndOfStream)
        ));
        assert!(matches!(
            events.as_slice(),
            [ServerSupervisorEvent::Command(CommandHandled { call_id, .. })]
                if *call_id == CallId::new(7)
        ));
        assert_eq!(supervisor.phase(), ServerSupervisorPhase::Closed);
        assert_eq!(
            supervisor
                .transport()
                .sent
                .lock()
                .expect("sent frames should be available")
                .len(),
            2
        );
    }

    #[test]
    fn supervisor_rejects_application_run_before_required_handshake() {
        // Arrange
        let transport = MemoryTransport::new([Ok(Some(command_frame(TestCodec::valid(), 7, 41)))]);
        let mut supervisor = EncodedServerSupervisor::new_handshake_required(
            transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let release = Arc::new(ReleaseState {
            released: AtomicBool::new(false),
            waker: Mutex::new(None),
        });
        let mut controls = ReleaseControl { state: release };

        // Act
        let result =
            block_on(supervisor.run(&commands, &mut controls, |_| Ok::<_, Infallible>(())));

        // Assert
        assert!(matches!(
            result,
            Err(ServerSupervisorError::HandshakeRequired)
        ));
        assert_eq!(supervisor.phase(), ServerSupervisorPhase::AwaitingHandshake);
        assert_eq!(
            supervisor
                .transport()
                .incoming
                .lock()
                .expect("incoming frames should be available")
                .len(),
            1
        );
    }

    #[test]
    fn negotiated_supervisor_rejects_application_version_drift_before_dispatch() {
        // Arrange
        let codec = HandshakeCodec {
            client: client_hello(0, 0),
            server_route: FrameRoute::Handshake,
        };
        let valid = command_frame(TestCodec::valid(), 7, 41);
        let wrong_version = WireFrame::new(
            ProtocolVersion::new(1, 1),
            valid.route(),
            valid.payload().to_vec(),
            Vec::new(),
            handshake_limits(),
        )
        .expect("version-drift frame should remain structurally bounded");
        let transport = MemoryTransport::new([
            Ok(Some(handshake_frame(ProtocolVersion::new(9, 9)))),
            Ok(Some(wrong_version)),
        ]);
        let mut supervisor = EncodedServerSupervisor::new_handshake_required(
            transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut gate = RecordingGate {
            accepted: true,
            calls: Arc::new(AtomicUsize::new(0)),
        };
        block_on(supervisor.accept_handshake(&codec, &server_config(0, 0), &mut gate))
            .expect("handshake should establish before version drift");
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let termination_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut controls = TrackingTerminationControl::new(Arc::clone(&termination_reasons));

        // Act
        let result =
            block_on(supervisor.run(&commands, &mut controls, |_| Ok::<_, Infallible>(())));

        // Assert
        assert!(matches!(
            result,
            Err(ServerSupervisorError::IncomingFrame(
                super::ApplicationFrameError::VersionMismatch { expected, actual }
            )) if expected == ProtocolVersion::new(1, 0)
                && actual == ProtocolVersion::new(1, 1)
        ));
        assert_eq!(supervisor.metrics().accepted_commands(), 0);
        assert_eq!(
            supervisor.metrics().close_reason(),
            Some(ServerSupervisorCloseReason::IncomingFrameRejected)
        );
        assert_eq!(
            *termination_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::IncomingFrameRejected]
        );
    }

    #[test]
    fn rejected_authorization_is_terminal_and_sends_no_server_hello() {
        // Arrange
        let codec = HandshakeCodec {
            client: client_hello(0, 1),
            server_route: FrameRoute::Handshake,
        };
        let transport =
            MemoryTransport::new([Ok(Some(handshake_frame(ProtocolVersion::new(9, 9))))]);
        let mut supervisor = EncodedServerSupervisor::new_handshake_required(
            transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut gate = RecordingGate {
            accepted: false,
            calls: Arc::new(AtomicUsize::new(0)),
        };

        // Act
        let result = block_on(supervisor.accept_handshake(&codec, &server_config(0, 1), &mut gate));

        // Assert
        assert!(matches!(
            result,
            Err(super::ServerHandshakeError::Authorization(
                AuthorizationError
            ))
        ));
        assert_eq!(supervisor.phase(), ServerSupervisorPhase::Closed);
        assert_eq!(
            supervisor.metrics().close_reason(),
            Some(ServerSupervisorCloseReason::HandshakeFailed)
        );
        assert!(
            supervisor
                .transport()
                .sent
                .lock()
                .expect("sent frames should be available")
                .is_empty()
        );
    }

    #[test]
    fn dropping_pending_handshake_authorization_is_terminal() {
        // Arrange
        let codec = HandshakeCodec {
            client: client_hello(0, 1),
            server_route: FrameRoute::Handshake,
        };
        let transport =
            MemoryTransport::new([Ok(Some(handshake_frame(ProtocolVersion::new(9, 9))))]);
        let mut supervisor = EncodedServerSupervisor::new_handshake_required(
            transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut gate = PendingGate;
        let config = server_config(0, 1);
        let mut handshake = Box::pin(supervisor.accept_handshake(&codec, &config, &mut gate));
        let mut context = Context::from_waker(Waker::noop());
        assert!(handshake.as_mut().poll(&mut context).is_pending());

        // Act
        drop(handshake);

        // Assert
        assert_eq!(supervisor.phase(), ServerSupervisorPhase::Closed);
        assert_eq!(
            supervisor.metrics().close_reason(),
            Some(ServerSupervisorCloseReason::HandshakeFutureDropped)
        );
        assert!(
            supervisor
                .transport()
                .sent
                .lock()
                .expect("sent frames should be available")
                .is_empty()
        );
    }

    #[test]
    fn second_handshake_is_rejected_after_negotiation() {
        // Arrange
        let codec = HandshakeCodec {
            client: client_hello(0, 0),
            server_route: FrameRoute::Handshake,
        };
        let mut handler = HandshakeFirstHandler::new(codec, server_config(0, 0), PreserveHandler);
        let transport = MemoryTransport::new([]);
        block_on(handler.handle(handshake_frame(ProtocolVersion::new(1, 0)), &transport))
            .expect("first handshake should succeed");

        // Act
        let error =
            block_on(handler.handle(handshake_frame(ProtocolVersion::new(1, 0)), &transport))
                .expect_err("second handshake should fail");

        // Assert
        assert!(matches!(error, HandshakeFirstError::DuplicateHandshake));
    }

    #[test]
    fn post_handshake_frame_must_use_exact_negotiated_version() {
        // Arrange
        let codec = HandshakeCodec {
            client: client_hello(0, 1),
            server_route: FrameRoute::Handshake,
        };
        let mut handler = HandshakeFirstHandler::new(codec, server_config(0, 1), PreserveHandler);
        let transport = MemoryTransport::new([]);
        block_on(handler.handle(handshake_frame(ProtocolVersion::new(1, 0)), &transport))
            .expect("handshake should succeed");
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            Vec::new(),
            Vec::new(),
            frame_limits(),
        )
        .expect("application frame should build");

        // Act
        let error = block_on(handler.handle(frame, &transport))
            .expect_err("wrong negotiated version should fail");

        // Assert
        assert!(matches!(
            error,
            HandshakeFirstError::VersionMismatch {
                expected,
                actual
            } if expected == ProtocolVersion::new(1, 1)
                && actual == ProtocolVersion::new(1, 0)
        ));
    }

    #[test]
    fn failed_negotiation_does_not_establish_or_send() {
        // Arrange
        let codec = HandshakeCodec {
            client: client_hello(2, 3),
            server_route: FrameRoute::Handshake,
        };
        let mut handler = HandshakeFirstHandler::new(codec, server_config(0, 1), PreserveHandler);
        let transport = MemoryTransport::new([]);

        // Act
        let error =
            block_on(handler.handle(handshake_frame(ProtocolVersion::new(1, 0)), &transport))
                .expect_err("disjoint version ranges should fail");

        // Assert
        assert!(matches!(error, HandshakeFirstError::Negotiate(_)));
        assert!(handler.negotiated().is_none());
        assert!(
            transport
                .sent
                .lock()
                .expect("sent frames should be available")
                .is_empty()
        );
    }

    #[test]
    fn typed_handler_dispatches_and_sends_correlated_result() {
        // Arrange
        let codec = TestCodec::valid();
        let mut handler = TypedCommandHandler::<Add, _, _>::new(
            protocol(codec),
            AddService {
                response_call_offset: 0,
            },
        );
        let transport = MemoryTransport::new([]);

        // Act
        let outcome = block_on(handler.handle(command_frame(codec, 7, 41), &transport))
            .expect("command should dispatch");

        // Assert
        assert_eq!(
            outcome,
            ServerFrameDisposition::Handled(CommandHandled {
                call_id: CallId::new(7),
                object_id: ObjectId::new(2),
                method: Add::METHOD,
            })
        );
        let sent = transport
            .sent
            .lock()
            .expect("sent frames should be available");
        assert_eq!(sent.len(), 1);
        let result = protocol(codec)
            .decode::<CommandResult<Add>>(sent[0].clone())
            .expect("result should decode");
        assert_eq!(result.call_id(), CallId::new(7));
        assert_eq!(result.into_outcome(), Ok(42));
    }

    #[test]
    fn different_command_method_is_preserved_without_dispatch() {
        // Arrange
        let codec = TestCodec::valid();
        let mut frame = command_frame(codec, 7, 41);
        frame = WireFrame::new(
            frame.version(),
            FrameRoute::Command {
                call_id: CallId::new(7),
                target: ObjectId::new(2),
                method: MethodTag::new(99),
            },
            frame.payload().to_vec(),
            Vec::new(),
            frame_limits(),
        )
        .expect("alternate command frame should build");
        let mut handler = TypedCommandHandler::<Add, _, _>::new(
            protocol(codec),
            AddService {
                response_call_offset: 0,
            },
        );
        let transport = MemoryTransport::new([]);

        // Act
        let outcome = block_on(handler.handle(frame, &transport))
            .expect("other method should remain unhandled");

        // Assert
        let ServerFrameDisposition::Unhandled(frame) = outcome else {
            panic!("different method should remain unhandled");
        };
        assert_eq!(
            frame.route(),
            FrameRoute::Command {
                call_id: CallId::new(7),
                target: ObjectId::new(2),
                method: MethodTag::new(99),
            }
        );
        assert!(
            transport
                .sent
                .lock()
                .expect("sent frames should be available")
                .is_empty()
        );
    }

    #[test]
    fn handler_chain_routes_an_unclaimed_frame_to_the_next_handler() {
        // Arrange
        let codec = TestCodec::valid();
        let typed = TypedCommandHandler::<Add, _, _>::new(
            protocol(codec),
            AddService {
                response_call_offset: 0,
            },
        );
        let mut chain = <PreserveHandler as ServerFrameHandlerExt<MemoryTransport>>::or_else(
            PreserveHandler,
            typed,
        );
        let transport = MemoryTransport::new([]);

        // Act
        let outcome = block_on(chain.handle(command_frame(codec, 7, 41), &transport))
            .expect("second handler should accept command");

        // Assert
        assert!(matches!(
            outcome,
            ServerFrameDisposition::Handled(super::Either::Right(CommandHandled {
                call_id,
                object_id,
                method,
            })) if call_id == CallId::new(7)
                && object_id == ObjectId::new(2)
                && method == Add::METHOD
        ));
    }

    #[test]
    fn concurrent_handler_chain_routes_without_erasing_owned_work_types() {
        // Arrange
        let left = ConcurrentRouteHandler {
            method: MethodTag::new(99),
            event: "left",
        };
        let right = ConcurrentRouteHandler {
            method: Add::METHOD,
            event: "right",
        };
        let chain = <ConcurrentRouteHandler as ConcurrentServerFrameHandlerExt<
            MemoryTransport,
        >>::or_else_concurrently(left, right);
        let transport = Arc::new(MemoryTransport::new([]));

        // Act
        let outcome = block_on(chain.handle_concurrently(
            command_frame(TestCodec::valid(), 7, 41),
            Arc::clone(&transport),
        ))
        .expect("second concurrent handler should claim the command");

        // Assert
        assert!(matches!(
            outcome,
            ServerFrameDisposition::Handled(super::Either::Right("right"))
        ));
        assert!(
            transport
                .sent
                .lock()
                .expect("sent frames should be available")
                .is_empty()
        );
    }

    #[test]
    fn decoded_and_response_identity_mismatches_fail_before_send() {
        // Arrange
        let mut decoded_mismatch = TypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec {
                decoded_call_offset: 1,
                result_route_offset: 0,
            }),
            AddService {
                response_call_offset: 0,
            },
        );
        let mut response_mismatch = TypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 1,
            },
        );
        let transport = MemoryTransport::new([]);

        // Act
        let decoded_error =
            block_on(decoded_mismatch.handle(command_frame(TestCodec::valid(), 7, 41), &transport));
        let response_error = block_on(
            response_mismatch.handle(command_frame(TestCodec::valid(), 7, 41), &transport),
        );

        // Assert
        assert!(matches!(
            decoded_error,
            Err(TypedCommandHandlerError::DecodedRouteMismatch { .. })
        ));
        assert!(matches!(
            response_error,
            Err(TypedCommandHandlerError::ResponseCallMismatch { .. })
        ));
        assert!(
            transport
                .sent
                .lock()
                .expect("sent frames should be available")
                .is_empty()
        );
    }

    #[test]
    fn encoded_result_route_and_transport_failures_are_explicit() {
        // Arrange
        let mut route_mismatch = TypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec {
                decoded_call_offset: 0,
                result_route_offset: 1,
            }),
            AddService {
                response_call_offset: 0,
            },
        );
        let mut send_failure = TypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let recording = MemoryTransport::new([]);
        let rejecting = MemoryTransport::rejecting_send();

        // Act
        let route_error =
            block_on(route_mismatch.handle(command_frame(TestCodec::valid(), 7, 41), &recording));
        let send_error =
            block_on(send_failure.handle(command_frame(TestCodec::valid(), 7, 41), &rejecting));

        // Assert
        assert!(matches!(
            route_error,
            Err(TypedCommandHandlerError::EncodedRouteMismatch { .. })
        ));
        assert!(matches!(
            send_error,
            Err(TypedCommandHandlerError::Transport(TransportError))
        ));
    }

    #[test]
    fn server_driver_owns_receive_and_makes_closure_terminal() {
        // Arrange
        let codec = TestCodec::valid();
        let transport = MemoryTransport::new([
            Ok(Some(command_frame(codec, 7, 41))),
            Ok(None),
            Ok(Some(command_frame(codec, 8, 42))),
        ]);
        let mut driver = EncodedServerDriver::new(transport);
        let mut handler = TypedCommandHandler::<Add, _, _>::new(
            protocol(codec),
            AddService {
                response_call_offset: 0,
            },
        );

        // Act
        let first = block_on(driver.receive_once_with(&mut handler));
        let eof = block_on(driver.receive_once_with(&mut handler));
        let after_eof = block_on(driver.receive_once_with(&mut handler));

        // Assert
        assert!(matches!(first, Ok(ServerDriverEvent::Handled(_))));
        assert!(matches!(
            eof,
            Ok(ServerDriverEvent::Closed(ServerCloseReason::EndOfStream))
        ));
        assert!(matches!(
            after_eof,
            Ok(ServerDriverEvent::Closed(ServerCloseReason::EndOfStream))
        ));
        assert_eq!(driver.close_reason(), Some(ServerCloseReason::EndOfStream));
    }

    #[test]
    fn receive_failure_is_terminal_and_distinct_from_eof() {
        // Arrange
        let transport = MemoryTransport::new([Err(TransportError)]);
        let mut driver = EncodedServerDriver::new(transport);
        let mut handler = TypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );

        // Act
        let failed = block_on(driver.receive_once_with(&mut handler));
        let after_failure = block_on(driver.receive_once_with(&mut handler));

        // Assert
        assert!(matches!(
            failed,
            Err(super::ServerDriverError::Transport(TransportError))
        ));
        assert!(matches!(
            after_failure,
            Ok(ServerDriverEvent::Closed(ServerCloseReason::ReceiveFailed))
        ));
    }

    #[test]
    fn handler_failure_is_terminal_for_the_unique_receive_driver() {
        // Arrange
        let wire_codec = TestCodec::valid();
        let transport = MemoryTransport::new([
            Ok(Some(command_frame(wire_codec, 7, 41))),
            Ok(Some(command_frame(wire_codec, 8, 42))),
        ]);
        let mut driver = EncodedServerDriver::new(transport);
        let mut handler = TypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec {
                decoded_call_offset: 1,
                result_route_offset: 0,
            }),
            AddService {
                response_call_offset: 0,
            },
        );

        // Act
        let failed = block_on(driver.receive_once_with(&mut handler));
        let after_failure = block_on(driver.receive_once_with(&mut handler));

        // Assert
        assert!(matches!(
            failed,
            Err(super::ServerDriverError::Handler(
                TypedCommandHandlerError::DecodedRouteMismatch { .. }
            ))
        ));
        assert!(matches!(
            after_failure,
            Ok(ServerDriverEvent::Closed(ServerCloseReason::HandlerFailed))
        ));
        assert_eq!(
            driver.close_reason(),
            Some(ServerCloseReason::HandlerFailed)
        );
    }

    #[test]
    fn supervisor_production_profile_declares_each_retained_command() {
        // Arrange
        let limits = ServerSupervisorLimits::new(3).expect("limit should be valid");

        // Act
        let profile = ServerSupervisorProductionProfile::new(limits);
        let requirements = profile.requirements();
        let pending = requirements.class(ResourceClass::PendingCall);

        // Assert
        assert_eq!(profile.limits(), limits);
        assert_eq!(pending.global, 3);
        assert_eq!(pending.per_scope, 1);
        assert_eq!(requirements.scopes, 3);
        assert_eq!(requirements.leases, 3);
    }

    #[test]
    fn supervisor_rejects_duplicate_active_call_at_the_resource_boundary() {
        // Arrange
        let limits = ServerSupervisorLimits::new(2).expect("limit should be valid");
        let profile = ServerSupervisorProductionProfile::new(limits);
        let resources = supervisor_resources(profile);
        let transport = MemoryTransport::new([
            Ok(Some(command_frame(TestCodec::valid(), 7, 41))),
            Ok(Some(command_frame(TestCodec::valid(), 7, 42))),
        ]);
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            PendingAddService,
        );
        let termination_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut controls = TrackingTerminationControl::new(Arc::clone(&termination_reasons));
        let mut supervisor = EncodedServerSupervisor::new(transport, limits)
            .with_production_resources(profile, &resources)
            .expect("matching production resources should install");

        // Act
        let result =
            block_on(supervisor.run(&commands, &mut controls, |_| Ok::<_, Infallible>(())));

        // Assert
        assert!(matches!(
            result,
            Err(ServerSupervisorError::Resource(
                ResourceError::ScopeLimitExceeded {
                    scope: ResourceScope::Call(call_id),
                    class: ResourceClass::PendingCall,
                    current: 1,
                    requested: 1,
                    limit: 1,
                }
            )) if call_id == CallId::new(7)
        ));
        assert_eq!(
            supervisor.metrics().close_reason(),
            Some(ServerSupervisorCloseReason::CommandResourceExhausted)
        );
        assert_eq!(supervisor.metrics().accepted_commands(), 1);
        assert_eq!(supervisor.metrics().dropped_commands(), 1);
        assert_eq!(supervisor.metrics().capacity_failures(), 1);
        let metrics = resources.metrics().expect("resource metrics should load");
        let pending = metrics.class(ResourceClass::PendingCall);
        assert_eq!(pending.current, 0);
        assert_eq!(pending.peak, 1);
        assert_eq!(pending.rejected, 1);
        assert_eq!(metrics.active_leases, 0);
        assert_eq!(
            *termination_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::CommandResourceExhausted]
        );
    }

    #[test]
    fn supervisor_requires_its_production_profile_to_match_runtime_limits() {
        // Arrange
        let supervisor_limits =
            ServerSupervisorLimits::new(1).expect("supervisor limit should be valid");
        let profile_limits = ServerSupervisorLimits::new(2).expect("profile limit should be valid");
        let profile = ServerSupervisorProductionProfile::new(profile_limits);
        let resources = supervisor_resources(profile);
        let supervisor = EncodedServerSupervisor::new(MemoryTransport::new([]), supervisor_limits);

        // Act
        let error = match supervisor.with_production_resources(profile, &resources) {
            Ok(_) => panic!("mismatched limits must reject production installation"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(
            error,
            ServerSupervisorProductionError::LimitsMismatch {
                supervisor: supervisor_limits,
                profile: profile_limits,
            }
        );
    }

    #[test]
    fn supervisor_rotates_receive_command_and_outbound_lanes() {
        // Arrange
        let transport = MemoryTransport::new([
            Ok(Some(command_frame(TestCodec::valid(), 7, 41))),
            Ok(Some(command_frame(TestCodec::valid(), 8, 42))),
            Ok(None),
        ]);
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let release = Arc::new(ReleaseState {
            released: AtomicBool::new(false),
            waker: Mutex::new(None),
        });
        let mut controls = ReleaseControl { state: release };
        let outbound = QueuedOutbound::new([Some("lifecycle-1"), Some("lifecycle-2"), None]);
        let mut supervisor = EncodedServerSupervisor::new(
            transport,
            ServerSupervisorLimits::new(2).expect("limit should be valid"),
        );
        let mut events = Vec::new();

        // Act
        let result = block_on(supervisor.run_with_outbound_and_drain_deadline(
            &commands,
            &mut controls,
            &outbound,
            |event| {
                events.push(match event {
                    ServerSupervisorEvent::Command(CommandHandled { call_id, .. }) => {
                        if call_id == CallId::new(7) {
                            "command-7"
                        } else {
                            "command-8"
                        }
                    }
                    ServerSupervisorEvent::Outbound(event) => event,
                    ServerSupervisorEvent::Control(_) => "control",
                    ServerSupervisorEvent::Unhandled(_) => "unhandled",
                });
                Ok::<_, Infallible>(())
            },
            pending(),
        ));

        // Assert
        assert!(matches!(
            result,
            Ok(ServerSupervisorCloseReason::EndOfStream)
        ));
        assert_eq!(
            events,
            vec!["command-7", "lifecycle-1", "command-8", "lifecycle-2"]
        );
        assert_eq!(supervisor.metrics().completed_commands(), 2);
        assert_eq!(supervisor.metrics().outbound_messages(), 2);
    }

    #[test]
    fn supervisor_rotates_backend_with_receive_command_and_outbound_lanes() {
        // Arrange
        let transport = MemoryTransport::new([
            Ok(Some(command_frame(TestCodec::valid(), 7, 41))),
            Ok(Some(command_frame(TestCodec::valid(), 8, 42))),
            Ok(None),
        ]);
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let release = Arc::new(ReleaseState {
            released: AtomicBool::new(false),
            waker: Mutex::new(None),
        });
        let mut controls = ReleaseControl { state: release };
        let outbound = QueuedOutbound::new([Some("outbound-1"), Some("outbound-2"), None]);
        let backend_polls = Arc::new(AtomicUsize::new(0));
        let mut backend = QueuedBackend::new(
            [Ok("backend-1"), Ok("backend-2")],
            Arc::clone(&backend_polls),
        );
        let mut supervisor = EncodedServerSupervisor::new(
            transport,
            ServerSupervisorLimits::new(2).expect("limit should be valid"),
        );
        let mut events = Vec::new();

        // Act
        let result = block_on(supervisor.run_with_outbound_and_backend(
            &commands,
            &mut controls,
            &outbound,
            &mut backend,
            |event| {
                events.push(match event {
                    ServerSupervisorEvent::Command(CommandHandled { call_id, .. }) => {
                        if call_id == CallId::new(7) {
                            "command-7"
                        } else {
                            "command-8"
                        }
                    }
                    ServerSupervisorEvent::Outbound(event) => event,
                    ServerSupervisorEvent::Backend(event) => event,
                    ServerSupervisorEvent::Control(_) => "control",
                    ServerSupervisorEvent::Unhandled(_) => "unhandled",
                });
                Ok::<_, Infallible>(())
            },
        ));

        // Assert
        assert!(matches!(
            result,
            Ok(ServerSupervisorCloseReason::EndOfStream)
        ));
        assert_eq!(
            events,
            vec![
                "command-7",
                "outbound-1",
                "backend-1",
                "command-8",
                "outbound-2",
                "backend-2",
            ]
        );
        assert!(backend_polls.load(Ordering::SeqCst) >= 2);
        assert_eq!(supervisor.metrics().completed_commands(), 2);
        assert_eq!(supervisor.metrics().outbound_messages(), 2);
    }

    #[test]
    fn backend_failure_is_terminal_and_runs_reasoned_control_cleanup() {
        // Arrange
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            PendingAddService,
        );
        let termination_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut controls = TrackingTerminationControl::new(Arc::clone(&termination_reasons));
        let backend_polls = Arc::new(AtomicUsize::new(0));
        let mut backend = QueuedBackend::new([Err(BackendError)], backend_polls);
        let mut supervisor = EncodedServerSupervisor::new(
            PendingReceiveTransport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );

        // Act
        let result = block_on(supervisor.run_with_outbound_and_backend(
            &commands,
            &mut controls,
            &super::NoServerOutbound,
            &mut backend,
            |_| Ok::<_, Infallible>(()),
        ));

        // Assert
        assert!(matches!(
            result,
            Err(ServerSupervisorError::Backend(BackendError))
        ));
        assert_eq!(
            supervisor.metrics().close_reason(),
            Some(ServerSupervisorCloseReason::BackendEventFailed)
        );
        assert_eq!(
            *termination_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::BackendEventFailed]
        );
    }

    #[test]
    fn connection_run_preserves_backend_and_shutdown_failures() {
        // Arrange
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            PendingAddService,
        );
        let termination_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut controls = TrackingTerminationControl::new(Arc::clone(&termination_reasons));
        let backend_polls = Arc::new(AtomicUsize::new(0));
        let mut backend = QueuedBackend::new([Err(BackendError)], backend_polls);
        let shutdowns = Arc::new(AtomicUsize::new(0));
        let mut supervisor = EncodedServerSupervisor::new(
            PendingShutdownTransport {
                shutdowns: Arc::clone(&shutdowns),
                fail_shutdown: true,
            },
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );

        // Act
        let result = block_on(supervisor.run_connection_with_outbound_and_backend(
            &commands,
            &mut controls,
            &super::NoServerOutbound,
            &mut backend,
            |_| Ok::<_, Infallible>(()),
        ));

        // Assert
        assert!(matches!(
            result,
            Err(ServerConnectionRunError::RunAndShutdown {
                reason: ServerSupervisorCloseReason::BackendEventFailed,
                run,
                shutdown: TransportError,
            }) if matches!(run, ServerSupervisorError::Backend(BackendError))
        ));
        assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
        assert_eq!(
            *termination_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::BackendEventFailed]
        );
    }

    #[test]
    fn dropping_connection_run_releases_the_retained_backend_turn() {
        // Arrange
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            PendingAddService,
        );
        let release = Arc::new(ReleaseState {
            released: AtomicBool::new(false),
            waker: Mutex::new(None),
        });
        let mut controls = ReleaseControl { state: release };
        let polls = Arc::new(AtomicUsize::new(0));
        let terminations = Arc::new(Mutex::new(Vec::new()));
        let mut backend = PendingTrackingBackend {
            polls: Arc::clone(&polls),
            terminations: Arc::clone(&terminations),
        };
        let mut supervisor = EncodedServerSupervisor::new(
            PendingReceiveTransport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut run = Box::pin(supervisor.run_with_outbound_and_backend(
            &commands,
            &mut controls,
            &super::NoServerOutbound,
            &mut backend,
            |_| Ok::<_, Infallible>(()),
        ));
        let mut context = Context::from_waker(Waker::noop());
        assert!(run.as_mut().poll(&mut context).is_pending());

        // Act
        drop(run);

        // Assert
        assert!(polls.load(Ordering::SeqCst) >= 1);
        assert_eq!(
            *terminations
                .lock()
                .expect("backend terminations should be available"),
            vec![ServerSupervisorCloseReason::RunFutureDropped]
        );
        assert_eq!(
            supervisor.metrics().close_reason(),
            Some(ServerSupervisorCloseReason::RunFutureDropped)
        );
    }

    #[test]
    fn outbound_chain_alternates_ready_plugin_lanes() {
        // Arrange
        let outbound = QueuedOutbound::new([Some("first-1"), Some("first-2"), None]).chain(
            QueuedOutbound::new([Some("second-1"), Some("second-2"), None]),
        );

        // Act
        let outcomes = (0..5)
            .map(|_| block_on(outbound.send_once()).expect("outbound lane should not fail"))
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(
            outcomes,
            vec![
                Some(ChainedServerOutboundEvent::First("first-1")),
                Some(ChainedServerOutboundEvent::Second("second-1")),
                Some(ChainedServerOutboundEvent::First("first-2")),
                Some(ChainedServerOutboundEvent::Second("second-2")),
                None,
            ]
        );
    }

    #[test]
    fn outbound_chain_never_starts_another_lane_during_an_active_send() {
        // Arrange
        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let outbound = PendingOutbound {
            calls: Arc::clone(&first_calls),
        }
        .chain(IdleOutbound {
            calls: Arc::clone(&second_calls),
        });
        let mut send = Box::pin(outbound.send_once());
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);

        // Act
        let state = send.as_mut().poll(&mut context);

        // Assert
        assert!(state.is_pending());
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn outbound_failure_closes_the_supervisor_and_drops_active_work() {
        // Arrange
        let transport = MemoryTransport::new([
            Ok(Some(command_frame(TestCodec::valid(), 7, 41))),
            Ok(Some(command_frame(TestCodec::valid(), 8, 42))),
        ]);
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            PendingAddService,
        );
        let termination_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut controls = TrackingTerminationControl::new(Arc::clone(&termination_reasons));
        let mut supervisor = EncodedServerSupervisor::new(
            transport,
            ServerSupervisorLimits::new(2).expect("limit should be valid"),
        );

        // Act
        let result = block_on(supervisor.run_with_outbound(
            &commands,
            &mut controls,
            &FailingOutbound,
            |_| Ok::<_, Infallible>(()),
        ));

        // Assert
        assert!(matches!(
            result,
            Err(ServerSupervisorError::Outbound(OutboundError))
        ));
        assert_eq!(supervisor.metrics().accepted_commands(), 1);
        assert_eq!(supervisor.metrics().dropped_commands(), 1);
        assert_eq!(supervisor.metrics().active_commands(), 0);
        assert_eq!(
            supervisor.metrics().close_reason(),
            Some(ServerSupervisorCloseReason::OutboundFailed)
        );
        assert_eq!(
            *termination_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::OutboundFailed]
        );
    }

    #[test]
    fn idle_outbound_lane_parks_without_a_ready_loop() {
        // Arrange
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            PendingAddService,
        );
        let release = Arc::new(ReleaseState {
            released: AtomicBool::new(false),
            waker: Mutex::new(None),
        });
        let mut controls = ReleaseControl { state: release };
        let calls = Arc::new(AtomicUsize::new(0));
        let outbound = IdleOutbound {
            calls: Arc::clone(&calls),
        };
        let mut supervisor = EncodedServerSupervisor::new(
            PendingReceiveTransport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut run = Box::pin(supervisor.run_with_outbound(
            &commands,
            &mut controls,
            &outbound,
            |_| Ok::<_, Infallible>(()),
        ));
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let poll = run.as_mut().poll(&mut context);

        // Assert
        assert!(poll.is_pending());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        drop(run);
        assert_eq!(
            supervisor.metrics().close_reason(),
            Some(ServerSupervisorCloseReason::RunFutureDropped)
        );
    }

    #[test]
    fn readiness_wakes_a_parked_supervisor_without_connection_progress() {
        // Arrange
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            PendingAddService,
        );
        let release = Arc::new(ReleaseState {
            released: AtomicBool::new(false),
            waker: Mutex::new(None),
        });
        let mut controls = ReleaseControl { state: release };
        let outbound = WakeableOutbound::new();
        let mut events = Vec::new();
        let mut supervisor = EncodedServerSupervisor::new(
            PendingReceiveTransport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut run =
            Box::pin(
                supervisor.run_with_outbound(&commands, &mut controls, &outbound, |event| {
                    events.push(event);
                    Ok::<_, Infallible>(())
                }),
            );
        let mut context = Context::from_waker(Waker::noop());
        assert!(run.as_mut().poll(&mut context).is_pending());

        // Act
        outbound.enqueue();

        // Assert
        assert!(run.as_mut().poll(&mut context).is_pending());
        drop(run);
        assert!(matches!(
            events.as_slice(),
            [ServerSupervisorEvent::Outbound("event")]
        ));
    }

    #[test]
    fn supervisor_polls_control_while_command_work_is_pending() {
        // Arrange
        let release = Arc::new(ReleaseState {
            released: AtomicBool::new(false),
            waker: Mutex::new(None),
        });
        let control_frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            Vec::new(),
            Vec::new(),
            frame_limits(),
        )
        .expect("control frame should fit");
        let transport = MemoryTransport::new([
            Ok(Some(command_frame(TestCodec::valid(), 7, 41))),
            Ok(Some(control_frame)),
            Ok(None),
        ]);
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            ControlReleasedAddService {
                state: Arc::clone(&release),
            },
        );
        let mut controls = ReleaseControl { state: release };
        let mut supervisor = EncodedServerSupervisor::new(
            transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut events = Vec::new();

        // Act
        let result = block_on(supervisor.run(&commands, &mut controls, |event| {
            events.push(event);
            Ok::<_, Infallible>(())
        }));

        // Assert
        assert!(matches!(
            result,
            Ok(ServerSupervisorCloseReason::EndOfStream)
        ));
        assert!(matches!(
            events.as_slice(),
            [
                ServerSupervisorEvent::Control("released"),
                ServerSupervisorEvent::Command(CommandHandled { call_id, .. }),
            ] if *call_id == CallId::new(7)
        ));
        assert_eq!(
            supervisor
                .transport()
                .sent
                .lock()
                .expect("sent frames should be available")
                .len(),
            1
        );
        let metrics = supervisor.metrics();
        assert_eq!(metrics.active_commands(), 0);
        assert_eq!(metrics.peak_active_commands(), 1);
        assert_eq!(metrics.accepted_commands(), 1);
        assert_eq!(metrics.completed_commands(), 1);
        assert_eq!(metrics.handled_controls(), 1);
        assert_eq!(metrics.dropped_commands(), 0);
        assert_eq!(
            metrics.close_reason(),
            Some(ServerSupervisorCloseReason::EndOfStream)
        );
    }

    #[test]
    fn supervisor_stops_on_handler_reported_peer_graceful_close() {
        // Arrange
        let control_frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            Vec::new(),
            Vec::new(),
            frame_limits(),
        )
        .expect("control frame should fit");
        let transport = MemoryTransport::new([Ok(Some(control_frame)), Ok(None)]);
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let mut controls = PeerGracefulCloseControl;
        let mut supervisor = EncodedServerSupervisor::new(
            transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut events = Vec::new();

        // Act
        let result = block_on(supervisor.run(&commands, &mut controls, |event| {
            events.push(event);
            Ok::<_, Infallible>(())
        }));

        // Assert
        assert!(matches!(
            result,
            Ok(ServerSupervisorCloseReason::PeerGracefulClose)
        ));
        assert!(matches!(
            events.as_slice(),
            [ServerSupervisorEvent::Control("close acknowledged")]
        ));
        assert_eq!(
            supervisor.metrics().close_reason(),
            Some(ServerSupervisorCloseReason::PeerGracefulClose)
        );
    }

    #[test]
    fn control_barrier_prefetches_at_most_one_frame_without_overtaking() {
        // Arrange
        let control_frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            Vec::new(),
            Vec::new(),
            frame_limits(),
        )
        .expect("control frame should fit");
        let transport = MemoryTransport::new([
            Ok(Some(control_frame)),
            Ok(Some(command_frame(TestCodec::valid(), 7, 41))),
            Ok(None),
        ]);
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let release = Arc::new(ReleaseState {
            released: AtomicBool::new(false),
            waker: Mutex::new(None),
        });
        let mut controls = ReleaseControl { state: release };
        let mut supervisor = EncodedServerSupervisor::new(
            transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut events = Vec::new();

        // Act
        let result = block_on(supervisor.run(&commands, &mut controls, |event| {
            events.push(match event {
                ServerSupervisorEvent::Control("released") => "control",
                ServerSupervisorEvent::Command(CommandHandled { .. }) => "command",
                ServerSupervisorEvent::Control(_) => "unexpected-control",
                ServerSupervisorEvent::Unhandled(_) => "unhandled",
                ServerSupervisorEvent::Outbound(event) => match event {},
            });
            Ok::<_, Infallible>(())
        }));

        // Assert
        assert!(matches!(
            result,
            Ok(ServerSupervisorCloseReason::EndOfStream)
        ));
        assert_eq!(events, vec!["control", "command"]);
        assert_eq!(supervisor.metrics().handled_controls(), 1);
        assert_eq!(supervisor.metrics().accepted_commands(), 1);
        assert_eq!(supervisor.metrics().completed_commands(), 1);
    }

    #[test]
    fn supervisor_closes_instead_of_queueing_an_uncancellable_command() {
        // Arrange
        let transport = MemoryTransport::new([
            Ok(Some(command_frame(TestCodec::valid(), 7, 41))),
            Ok(Some(command_frame(TestCodec::valid(), 8, 42))),
        ]);
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            PendingAddService,
        );
        let termination_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut controls = TrackingTerminationControl::new(Arc::clone(&termination_reasons));
        let mut supervisor = EncodedServerSupervisor::new(
            transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );

        // Act
        let result =
            block_on(supervisor.run(&commands, &mut controls, |_| Ok::<_, Infallible>(())));

        // Assert
        assert!(matches!(
            result,
            Err(ServerSupervisorError::CommandCapacityExceeded { limit: 1 })
        ));
        let metrics = supervisor.metrics();
        assert_eq!(metrics.active_commands(), 0);
        assert_eq!(metrics.peak_active_commands(), 1);
        assert_eq!(metrics.accepted_commands(), 1);
        assert_eq!(metrics.completed_commands(), 0);
        assert_eq!(metrics.capacity_failures(), 1);
        assert_eq!(metrics.dropped_commands(), 1);
        assert_eq!(
            metrics.close_reason(),
            Some(ServerSupervisorCloseReason::CommandCapacityExceeded)
        );
        assert_eq!(
            *termination_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::CommandCapacityExceeded]
        );
    }

    #[test]
    fn terminal_failures_run_reasoned_control_cleanup_once() {
        // Arrange
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let receive_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut receive_controls = TrackingTerminationControl::new(Arc::clone(&receive_reasons));
        let mut receive_supervisor = EncodedServerSupervisor::new(
            MemoryTransport::new([Err(TransportError)]),
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );

        let control_frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            Vec::new(),
            Vec::new(),
            frame_limits(),
        )
        .expect("control frame should fit");
        let control_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut control_controls = TrackingTerminationControl::new(Arc::clone(&control_reasons));
        control_controls.fail_handle = true;
        let mut control_supervisor = EncodedServerSupervisor::new(
            MemoryTransport::new([Ok(Some(control_frame))]),
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );

        let command_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut command_controls = TrackingTerminationControl::new(Arc::clone(&command_reasons));
        let failing_commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec {
                decoded_call_offset: 1,
                result_route_offset: 0,
            }),
            AddService {
                response_call_offset: 0,
            },
        );
        let mut command_supervisor = EncodedServerSupervisor::new(
            MemoryTransport::new([Ok(Some(command_frame(TestCodec::valid(), 7, 41)))]),
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );

        let observer_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut observer_controls = TrackingTerminationControl::new(Arc::clone(&observer_reasons));
        let mut observer_supervisor = EncodedServerSupervisor::new(
            MemoryTransport::new([Ok(Some(command_frame(TestCodec::valid(), 8, 42)))]),
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );

        // Act
        let receive =
            block_on(
                receive_supervisor.run(&commands, &mut receive_controls, |_| {
                    Ok::<_, Infallible>(())
                }),
            );
        let control =
            block_on(
                control_supervisor.run(&commands, &mut control_controls, |_| {
                    Ok::<_, Infallible>(())
                }),
            );
        let command = block_on(command_supervisor.run(
            &failing_commands,
            &mut command_controls,
            |_| Ok::<_, Infallible>(()),
        ));
        let observer = block_on(
            observer_supervisor.run(&commands, &mut observer_controls, |_| {
                Err(TestControlError::Handle)
            }),
        );

        // Assert
        assert!(matches!(
            receive,
            Err(ServerSupervisorError::Transport(TransportError))
        ));
        assert!(matches!(
            control,
            Err(ServerSupervisorError::Control(TestControlError::Handle))
        ));
        assert!(matches!(
            command,
            Err(ServerSupervisorError::Command(
                TypedCommandHandlerError::DecodedRouteMismatch { .. }
            ))
        ));
        assert!(matches!(
            observer,
            Err(ServerSupervisorError::Observer(TestControlError::Handle))
        ));
        assert_eq!(
            *receive_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::ReceiveFailed]
        );
        assert_eq!(
            *control_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::ControlFailed]
        );
        assert_eq!(
            *command_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::CommandFailed]
        );
        assert_eq!(
            *observer_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::ObserverFailed]
        );
    }

    #[test]
    fn primary_and_cleanup_failures_are_both_reported() {
        // Arrange
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let termination_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut controls = TrackingTerminationControl::new(Arc::clone(&termination_reasons));
        controls.fail_cleanup = true;
        let mut supervisor = EncodedServerSupervisor::new(
            MemoryTransport::new([Err(TransportError)]),
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );

        // Act
        let result =
            block_on(supervisor.run(&commands, &mut controls, |_| Ok::<_, Infallible>(())));

        // Assert
        assert!(matches!(
            result,
            Err(ServerSupervisorError::TerminationCleanup {
                reason: ServerSupervisorCloseReason::ReceiveFailed,
                primary,
                cleanup: TestControlError::Cleanup,
            }) if matches!(*primary, ServerSupervisorError::Transport(TransportError))
        ));
        assert_eq!(
            *termination_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::ReceiveFailed]
        );
    }

    #[test]
    fn explicit_async_close_runs_cleanup_at_most_once() {
        // Arrange
        let termination_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut controls = TrackingTerminationControl::new(Arc::clone(&termination_reasons));
        let mut supervisor = EncodedServerSupervisor::new(
            MemoryTransport::new([]),
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );

        // Act
        let first = block_on(supervisor.close_with_controls(&mut controls));
        let second = block_on(supervisor.close_with_controls(&mut controls));

        // Assert
        assert_eq!(first, Ok(ServerSupervisorCloseReason::Explicit));
        assert_eq!(second, Ok(ServerSupervisorCloseReason::Explicit));
        assert_eq!(
            *termination_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::Explicit]
        );
    }

    #[test]
    fn classified_backend_event_termination_is_preserved_through_cleanup() {
        // Arrange
        let termination_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut controls = TrackingTerminationControl::new(Arc::clone(&termination_reasons));
        let mut supervisor = EncodedServerSupervisor::new(
            MemoryTransport::new([]),
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );

        // Act
        let first = block_on(supervisor.terminate_with_controls(
            ServerSupervisorCloseReason::BackendEventFailed,
            &mut controls,
        ));
        let repeated = block_on(supervisor.close_with_controls(&mut controls));

        // Assert
        assert_eq!(first, Ok(ServerSupervisorCloseReason::BackendEventFailed));
        assert_eq!(
            repeated,
            Ok(ServerSupervisorCloseReason::BackendEventFailed)
        );
        assert_eq!(
            *termination_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::BackendEventFailed]
        );
        assert_eq!(
            supervisor.metrics().close_reason(),
            Some(ServerSupervisorCloseReason::BackendEventFailed)
        );
    }

    #[test]
    fn explicit_close_preserves_cleanup_and_transport_shutdown_failures() {
        // Arrange
        let termination_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut controls = TrackingTerminationControl::new(Arc::clone(&termination_reasons));
        controls.fail_cleanup = true;
        let mut supervisor = EncodedServerSupervisor::new(
            MemoryTransport::rejecting_send(),
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );

        // Act
        let result = block_on(supervisor.close_with_controls_and_transport(&mut controls));

        // Assert
        assert!(matches!(
            result,
            Err(ServerTransportCloseError::ControlAndTransport {
                control: TestControlError::Cleanup,
                transport: TransportError,
            })
        ));
        assert_eq!(
            *termination_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::Explicit]
        );
        assert_eq!(
            supervisor.metrics().close_reason(),
            Some(ServerSupervisorCloseReason::Explicit)
        );
    }

    #[test]
    fn cancelled_async_close_can_retry_the_unfinished_cleanup_barrier() {
        // Arrange
        let calls = Arc::new(AtomicUsize::new(0));
        let ready = Arc::new(AtomicBool::new(false));
        let mut controls = RetryableCloseControl {
            calls: Arc::clone(&calls),
            ready: Arc::clone(&ready),
        };
        let mut supervisor = EncodedServerSupervisor::new(
            MemoryTransport::new([]),
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut first = Box::pin(supervisor.close_with_controls(&mut controls));
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let first_poll = first.as_mut().poll(&mut context);
        drop(first);
        ready.store(true, Ordering::SeqCst);
        let retried = block_on(supervisor.close_with_controls(&mut controls));

        // Assert
        assert!(first_poll.is_pending());
        assert_eq!(retried, Ok(ServerSupervisorCloseReason::Explicit));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn supervisor_awaits_the_async_transport_close_barrier() {
        // Arrange
        let transport = MemoryTransport::new([Ok(None)]);
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let started = Arc::new(AtomicBool::new(false));
        let mut controls = YieldingCloseControl {
            started: Arc::clone(&started),
        };
        let mut supervisor = EncodedServerSupervisor::new(
            transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut run =
            std::pin::pin!(supervisor.run(&commands, &mut controls, |_| Ok::<_, Infallible>(()),));
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let first = run.as_mut().poll(&mut context);
        let second = run.as_mut().poll(&mut context);

        // Assert
        assert!(first.is_pending());
        assert!(started.load(Ordering::SeqCst));
        assert!(matches!(
            second,
            Poll::Ready(Ok(ServerSupervisorCloseReason::EndOfStream))
        ));
    }

    #[test]
    fn eof_cancels_a_pending_control_barrier_before_running_close_compensation() {
        // Arrange
        let control_frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            Vec::new(),
            Vec::new(),
            frame_limits(),
        )
        .expect("control frame should fit");
        let transport = YieldingEofTransport::new(control_frame);
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            AddService {
                response_call_offset: 0,
            },
        );
        let state = Arc::new(PendingControlState {
            started: AtomicBool::new(false),
            dropped: AtomicBool::new(false),
            closed: AtomicBool::new(false),
        });
        let mut controls = EofCompensationControl {
            state: Arc::clone(&state),
        };
        let mut supervisor = EncodedServerSupervisor::new(
            transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut run =
            std::pin::pin!(supervisor.run(&commands, &mut controls, |_| Ok::<_, Infallible>(()),));
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let first = run.as_mut().poll(&mut context);
        let started_after_first = state.started.load(Ordering::SeqCst);
        let dropped_after_first = state.dropped.load(Ordering::SeqCst);
        let closed_after_first = state.closed.load(Ordering::SeqCst);
        let second = run.as_mut().poll(&mut context);

        // Assert
        assert!(first.is_pending());
        assert!(started_after_first);
        assert!(!dropped_after_first);
        assert!(!closed_after_first);
        assert!(matches!(
            second,
            Poll::Ready(Ok(ServerSupervisorCloseReason::EndOfStream))
        ));
        assert!(state.dropped.load(Ordering::SeqCst));
        assert!(state.closed.load(Ordering::SeqCst));
    }

    #[test]
    fn drain_deadline_bounds_work_that_remains_pending_after_eof() {
        // Arrange
        let transport =
            MemoryTransport::new([Ok(Some(command_frame(TestCodec::valid(), 7, 41))), Ok(None)]);
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            PendingAddService,
        );
        let close_calls = Arc::new(AtomicUsize::new(0));
        let mut controls = CountingCloseControl {
            close_calls: Arc::clone(&close_calls),
        };
        let deadline_polls = Arc::new(AtomicUsize::new(0));
        let deadline = CountingDeadline {
            polls: Arc::clone(&deadline_polls),
        };
        let limits = ServerSupervisorLimits::new(1)
            .expect("limit should be valid")
            .with_eof_policy(ServerEofPolicy::DrainActive);
        let mut supervisor = EncodedServerSupervisor::new(transport, limits);

        // Act
        let result = block_on(supervisor.run_with_drain_deadline(
            &commands,
            &mut controls,
            |_| Ok::<_, Infallible>(()),
            deadline,
        ));

        // Assert
        assert!(matches!(
            result,
            Ok(ServerSupervisorCloseReason::DrainDeadlineExceeded)
        ));
        assert_eq!(close_calls.load(Ordering::SeqCst), 1);
        assert_eq!(deadline_polls.load(Ordering::SeqCst), 1);
        let metrics = supervisor.metrics();
        assert_eq!(metrics.active_commands(), 0);
        assert_eq!(metrics.accepted_commands(), 1);
        assert_eq!(metrics.completed_commands(), 0);
        assert_eq!(metrics.dropped_commands(), 1);
        assert_eq!(metrics.drain_deadline_expirations(), 1);
        assert_eq!(
            metrics.close_reason(),
            Some(ServerSupervisorCloseReason::DrainDeadlineExceeded)
        );
    }

    #[test]
    fn close_released_command_finishes_before_a_ready_drain_deadline() {
        // Arrange
        let release = Arc::new(ReleaseState {
            released: AtomicBool::new(false),
            waker: Mutex::new(None),
        });
        let transport =
            MemoryTransport::new([Ok(Some(command_frame(TestCodec::valid(), 7, 41))), Ok(None)]);
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            ControlReleasedAddService {
                state: Arc::clone(&release),
            },
        );
        let mut controls = CloseReleaseControl { state: release };
        let deadline_polls = Arc::new(AtomicUsize::new(0));
        let deadline = CountingDeadline {
            polls: Arc::clone(&deadline_polls),
        };
        let limits = ServerSupervisorLimits::new(1)
            .expect("limit should be valid")
            .with_eof_policy(ServerEofPolicy::DrainActive);
        let mut supervisor = EncodedServerSupervisor::new(transport, limits);

        // Act
        let result = block_on(supervisor.run_with_drain_deadline(
            &commands,
            &mut controls,
            |_| Ok::<_, Infallible>(()),
            deadline,
        ));

        // Assert
        assert!(matches!(
            result,
            Ok(ServerSupervisorCloseReason::EndOfStream)
        ));
        assert_eq!(deadline_polls.load(Ordering::SeqCst), 0);
        let metrics = supervisor.metrics();
        assert_eq!(metrics.completed_commands(), 1);
        assert_eq!(metrics.dropped_commands(), 0);
        assert_eq!(metrics.drain_deadline_expirations(), 0);
    }

    #[tokio::test]
    async fn drain_eof_policy_finishes_transport_cancelled_work_before_closing() {
        // Arrange
        let (client_io, server_io) = duplex(512);
        let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 128));
        let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
        let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
        client_transport
            .send(command_frame(TestCodec::valid(), 7, 41))
            .await
            .expect("command should reach the supervisor");
        client_transport
            .shutdown()
            .await
            .expect("client write half should close cleanly");
        let release = Arc::new(ReleaseState {
            released: AtomicBool::new(false),
            waker: Mutex::new(None),
        });
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            ControlReleasedAddService {
                state: Arc::clone(&release),
            },
        );
        let mut controls = CloseReleaseControl { state: release };
        let limits = ServerSupervisorLimits::new(1)
            .expect("limit should be valid")
            .with_eof_policy(super::ServerEofPolicy::DrainActive);
        let mut supervisor = EncodedServerSupervisor::new(server_transport, limits);
        let mut events = Vec::new();

        // Act
        let closed = supervisor
            .run(&commands, &mut controls, |event| {
                events.push(event);
                Ok::<_, Infallible>(())
            })
            .await;
        let shutdown_reason = supervisor
            .close_transport()
            .await
            .expect("server write half should close cleanly");
        let result_frame = client_transport
            .receive()
            .await
            .expect("client receive should remain usable after write half-close")
            .expect("drained command result should be sent");
        let peer_eof = client_transport
            .receive()
            .await
            .expect("client should observe a clean server EOF");

        // Assert
        assert!(matches!(
            closed,
            Ok(ServerSupervisorCloseReason::EndOfStream)
        ));
        assert_eq!(shutdown_reason, ServerSupervisorCloseReason::EndOfStream);
        assert!(peer_eof.is_none());
        assert!(matches!(
            events.as_slice(),
            [
                ServerSupervisorEvent::Control("transport closed"),
                ServerSupervisorEvent::Command(CommandHandled { call_id, .. }),
            ] if *call_id == CallId::new(7)
        ));
        let result = protocol(TestCodec::valid())
            .decode::<CommandResult<Add>>(result_frame)
            .expect("drained result should decode");
        assert_eq!(result.into_outcome(), Ok(42));
        let metrics = supervisor.metrics();
        assert_eq!(metrics.active_commands(), 0);
        assert_eq!(metrics.completed_commands(), 1);
        assert_eq!(metrics.handled_controls(), 1);
        assert_eq!(metrics.dropped_commands(), 0);
        assert_eq!(
            metrics.close_reason(),
            Some(ServerSupervisorCloseReason::EndOfStream)
        );
    }

    #[test]
    fn supervisor_rejects_a_zero_in_flight_limit() {
        // Arrange / Act
        let result = ServerSupervisorLimits::new(0);

        // Assert
        assert_eq!(
            result,
            Err(super::ServerSupervisorConfigurationError::ZeroInFlightCommands)
        );
    }

    #[tokio::test]
    async fn dropping_the_supervisor_run_future_is_terminal_and_releases_work() {
        // Arrange
        let (client_io, server_io) = duplex(512);
        let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 128));
        let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
        let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
        client_transport
            .send(command_frame(TestCodec::valid(), 7, 41))
            .await
            .expect("command should reach the supervisor");
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            protocol(TestCodec::valid()),
            PendingAddService,
        );
        let termination_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut controls = TrackingTerminationControl::new(Arc::clone(&termination_reasons));
        let mut supervisor = EncodedServerSupervisor::new(
            server_transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut run =
            Box::pin(supervisor.run(&commands, &mut controls, |_| Ok::<_, Infallible>(())));
        tokio::select! {
            result = &mut run => panic!("pending command should keep the supervisor open: {}", result.is_ok()),
            () = tokio::task::yield_now() => {}
        }

        // Act
        drop(run);
        let metrics = supervisor.metrics();
        let restarted = supervisor
            .run(&commands, &mut controls, |_| Ok::<_, Infallible>(()))
            .await;

        // Assert
        assert_eq!(metrics.active_commands(), 0);
        assert_eq!(metrics.accepted_commands(), 1);
        assert_eq!(metrics.completed_commands(), 0);
        assert_eq!(metrics.dropped_commands(), 1);
        assert_eq!(
            metrics.close_reason(),
            Some(ServerSupervisorCloseReason::RunFutureDropped)
        );
        assert!(matches!(
            restarted,
            Ok(ServerSupervisorCloseReason::RunFutureDropped)
        ));
        assert_eq!(
            *termination_reasons
                .lock()
                .expect("termination reasons should be available"),
            vec![ServerSupervisorCloseReason::RunFutureDropped]
        );
    }

    struct AddBackend;

    impl Service<u32> for AddBackend {
        type Response = u32;
        type Error = OperationError<()>;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, request: u32) -> Self::Future<'_> {
            ready(Ok(request + 1))
        }
    }

    #[tokio::test]
    async fn client_and_supervisor_negotiate_then_round_trip_on_the_same_stream() {
        // Arrange
        let client_receive = FrameLimits::new(128, 0, 0, 128);
        let server_receive = frame_limits();
        let client_hello = ClientHello::new(
            versions(0, 2),
            client_receive,
            CapabilityRequest::new(Vec::new(), Vec::new(), negotiation_limits())
                .expect("empty capability request should build"),
        );
        let server_config = ServerHandshakeConfig::new(
            versions(1, 1),
            server_receive,
            CapabilityManifest::new(Vec::new(), negotiation_limits())
                .expect("empty capability manifest should build"),
        );
        let (client_io, server_io) = duplex(1024);
        let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(handshake_limits(), 512));
        let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
        let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
        let client_gate = EncodedClientHandshake::new(client_transport);
        let client_protocol = WireProtocol::new_directional(
            CanonicalHandshakeCodec::new(negotiation_limits()),
            client_hello.versions(),
            client_hello.versions().maximum(),
            server_receive,
            client_receive,
        )
        .expect("client handshake protocol should build");
        let mut supervisor = EncodedServerSupervisor::new_handshake_required(
            server_transport,
            ServerSupervisorLimits::new(1).expect("limit should be valid"),
        );
        let mut gate = AllowUnauthenticated;
        let handshake_codec = CanonicalHandshakeCodec::new(negotiation_limits());

        // Act
        let (client_handshake, server_handshake) = tokio::join!(
            client_gate.establish(client_protocol, client_hello),
            supervisor.accept_handshake(&handshake_codec, &server_config, &mut gate),
        );
        let negotiated = client_handshake.expect("client should accept server hello");
        let accepted = server_handshake.expect("server should accept client hello");
        let selected = accepted.hello().selected_version();
        let (channel, mut client_driver) = negotiated.into_channel(TestCodec::valid(), 1);
        let client = Client::new(channel, AtomicCallIdSource::new(7));
        let remote = client
            .root::<CalculatorKind>(RemoteInitializer::new(
                ObjectId::new(2),
                None,
                CalculatorKind::OBJECT_KIND,
            ))
            .expect("typed calculator should initialize");
        let runtime = DirectDispatcher::with_object_kinds(
            AddBackend,
            RuntimeLimits::new(1, 1),
            CALCULATOR_OBJECT_KINDS,
        )
        .expect("calculator object catalog should be valid");
        runtime
            .register_object(ObjectId::new(2), None, CalculatorKind::OBJECT_KIND)
            .expect("runtime calculator should register");
        let commands = ConcurrentTypedCommandHandler::<Add, _, _>::new(
            accepted.application_protocol(TestCodec::valid()),
            runtime,
        );
        let mut controls = PreserveControl;
        let mut server_run =
            Box::pin(supervisor.run(&commands, &mut controls, |_| Ok::<_, Infallible>(())));
        let client_flow = async {
            tokio::join!(
                remote.dispatch::<Add>(41, CallMetadata::default()),
                client_driver.receive_once(),
            )
        };
        tokio::pin!(client_flow);
        let (result, event) = tokio::select! {
            flow = &mut client_flow => flow,
            result = &mut server_run => panic!("server closed before round trip: {result:?}"),
        };
        drop(server_run);

        // Assert
        assert_eq!(selected, ProtocolVersion::new(1, 1));
        assert_eq!(result, Ok(42));
        assert!(matches!(event, Ok(DriverEvent::Delivered(_))));
        assert_eq!(supervisor.metrics().completed_commands(), 1);
        assert_eq!(
            supervisor.metrics().close_reason(),
            Some(ServerSupervisorCloseReason::RunFutureDropped)
        );
    }

    #[tokio::test]
    async fn typed_client_round_trips_through_server_runtime_over_duplex_stream() {
        // Arrange
        let (client_io, server_io) = duplex(512);
        let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 128));
        let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
        let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
        let (channel, mut client_driver) =
            EncodedClientChannel::new(protocol(TestCodec::valid()), client_transport, 1);
        let client = Client::new(channel, AtomicCallIdSource::new(7));
        let remote = client
            .root::<CalculatorKind>(RemoteInitializer::new(
                ObjectId::new(2),
                None,
                CalculatorKind::OBJECT_KIND,
            ))
            .expect("typed calculator should initialize");
        let runtime = DirectDispatcher::with_object_kinds(
            AddBackend,
            RuntimeLimits::new(1, 1),
            CALCULATOR_OBJECT_KINDS,
        )
        .expect("calculator object catalog should be valid");
        runtime
            .register_object(ObjectId::new(2), None, CalculatorKind::OBJECT_KIND)
            .expect("runtime calculator should register");
        let mut handler =
            TypedCommandHandler::<Add, _, _>::new(protocol(TestCodec::valid()), runtime);
        let mut server_driver = EncodedServerDriver::new(server_transport);

        // Act
        let call = remote.dispatch::<Add>(41, CallMetadata::default());
        let (result, server_event, client_event) = tokio::join!(
            call,
            server_driver.receive_once_with(&mut handler),
            client_driver.receive_once(),
        );

        // Assert
        assert_eq!(result, Ok(42));
        assert!(matches!(
            server_event,
            Ok(ServerDriverEvent::Handled(CommandHandled {
                call_id,
                object_id,
                method,
            })) if call_id == CallId::new(7)
                && object_id == ObjectId::new(2)
                && method == Add::METHOD
        ));
        assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
    }
}
