use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::sync::Arc;

use pwright_core::{
    ProductionResourceError, ProductionResourceLedger, ProtocolVersion, ResourceClass,
    ResourceLedger, ResourceRequirements,
};

use crate::{FrameError, FrameLimits, FrameRoute, WireFrame};

/// Supplies routing metadata independently from a concrete payload codec.
pub trait RoutedMessage {
    fn route(&self) -> FrameRoute;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VersionPolicy {
    minimum: ProtocolVersion,
    maximum: ProtocolVersion,
}

impl VersionPolicy {
    pub const fn exact(version: ProtocolVersion) -> Self {
        Self {
            minimum: version,
            maximum: version,
        }
    }

    /// Declares compatibility with every later minor in the same major line.
    pub const fn compatible_from(minimum: ProtocolVersion) -> Self {
        Self {
            minimum,
            maximum: ProtocolVersion::new(minimum.major, u16::MAX),
        }
    }

    /// Declares compatibility with every earlier minor in the same major line.
    pub const fn compatible_through(maximum: ProtocolVersion) -> Self {
        Self {
            minimum: ProtocolVersion::new(maximum.major, 0),
            maximum,
        }
    }

    pub fn new(
        minimum: ProtocolVersion,
        maximum: ProtocolVersion,
    ) -> Result<Self, VersionPolicyError> {
        if minimum.major != maximum.major || minimum.minor > maximum.minor {
            return Err(VersionPolicyError::InvalidRange { minimum, maximum });
        }
        Ok(Self { minimum, maximum })
    }

    pub const fn minimum(self) -> ProtocolVersion {
        self.minimum
    }

    pub const fn maximum(self) -> ProtocolVersion {
        self.maximum
    }

    pub fn validate(self, received: ProtocolVersion) -> Result<(), VersionPolicyError> {
        if received.major != self.minimum.major
            || received.minor < self.minimum.minor
            || received.minor > self.maximum.minor
        {
            return Err(VersionPolicyError::Unsupported {
                received,
                minimum: self.minimum,
                maximum: self.maximum,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VersionPolicyError {
    InvalidRange {
        minimum: ProtocolVersion,
        maximum: ProtocolVersion,
    },
    Unsupported {
        received: ProtocolVersion,
        minimum: ProtocolVersion,
        maximum: ProtocolVersion,
    },
}

impl Display for VersionPolicyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange { minimum, maximum } => write!(
                formatter,
                "invalid protocol version range {minimum} through {maximum}"
            ),
            Self::Unsupported {
                received,
                minimum,
                maximum,
            } => write!(
                formatter,
                "protocol version {received} is outside supported range {minimum} through {maximum}"
            ),
        }
    }
}

impl Error for VersionPolicyError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedParts {
    route: FrameRoute,
    payload: Vec<u8>,
    attachments: Vec<Vec<u8>>,
}

impl EncodedParts {
    pub fn new(route: FrameRoute, payload: Vec<u8>, attachments: Vec<Vec<u8>>) -> Self {
        Self {
            route,
            payload,
            attachments,
        }
    }

    pub const fn route(&self) -> FrameRoute {
        self.route
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn attachments(&self) -> &[Vec<u8>] {
        &self.attachments
    }

    fn into_parts(self) -> (FrameRoute, Vec<u8>, Vec<Vec<u8>>) {
        (self.route, self.payload, self.attachments)
    }
}

/// Encodes one application message into frame parts and decodes one validated
/// frame. Size and version enforcement is owned by `WireProtocol`, not codecs.
///
/// A codec installed in a production protocol must ensure that variable-size
/// data reachable from its typed source is bounded by the same frame ceiling it
/// advertises. `WireProtocol` reserves twice that ceiling before calling the
/// encoder, covering the typed source and encoded buffers simultaneously.
pub trait WireCodec<Message>: Send + Sync {
    type Error;

    fn encode_parts(&self, message: &Message) -> Result<EncodedParts, Self::Error>;
    fn decode_frame(&self, frame: WireFrame) -> Result<Message, Self::Error>;
}

#[derive(Debug, Eq, PartialEq)]
pub enum WireProtocolError<E> {
    Version(VersionPolicyError),
    Frame(FrameError),
    Codec(E),
}

impl<E> Display for WireProtocolError<E>
where
    E: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Version(error) => Display::fmt(error, formatter),
            Self::Frame(error) => Display::fmt(error, formatter),
            Self::Codec(error) => Display::fmt(error, formatter),
        }
    }
}

impl<E> Error for WireProtocolError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Version(error) => Some(error),
            Self::Frame(error) => Some(error),
            Self::Codec(error) => Some(error),
        }
    }
}

/// Applies invariant version and size checks around a pluggable codec.
pub struct WireProtocol<C> {
    codec: C,
    versions: VersionPolicy,
    send_version: ProtocolVersion,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
    resource_ledger: Option<ResourceLedger>,
}

impl<C> WireProtocol<C> {
    pub const fn new_exact_directional(
        codec: C,
        version: ProtocolVersion,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
    ) -> Self {
        Self {
            codec,
            versions: VersionPolicy::exact(version),
            send_version: version,
            send_limits,
            receive_limits,
            resource_ledger: None,
        }
    }

    pub fn new(
        codec: C,
        versions: VersionPolicy,
        send_version: ProtocolVersion,
        limits: FrameLimits,
    ) -> Result<Self, VersionPolicyError> {
        Self::new_directional(codec, versions, send_version, limits, limits)
    }

    /// Builds a protocol with independent outbound and inbound frame limits.
    ///
    /// This is the shape required after connection negotiation: outbound
    /// frames obey the peer's advertised receive limits, while inbound frames
    /// are revalidated against this endpoint's advertised receive limits.
    pub fn new_directional(
        codec: C,
        versions: VersionPolicy,
        send_version: ProtocolVersion,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
    ) -> Result<Self, VersionPolicyError> {
        versions.validate(send_version)?;
        Ok(Self {
            codec,
            versions,
            send_version,
            send_limits,
            receive_limits,
            resource_ledger: None,
        })
    }

    /// Accounts every encoded frame against one explicitly injected shared
    /// connection budget. Existing unbudgeted constructors remain available
    /// for isolated codecs and v1 compatibility.
    pub fn with_resource_ledger(mut self, resource_ledger: ResourceLedger) -> Self {
        self.resource_ledger = Some(resource_ledger);
        self
    }

    /// Minimum production capacity for one maximum-size typed source and its
    /// simultaneously allocated outbound frame.
    ///
    /// Production codecs must bound the variable-size source representation by
    /// the frame limit they advertise. Additional simultaneously retained
    /// frames contend against the same connection ledger.
    pub fn production_resource_requirements(&self) -> ResourceRequirements {
        let bytes = u64::try_from(self.maximum_encoding_wire_bytes()).unwrap_or(u64::MAX);
        if bytes == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::WireBytes, bytes, bytes)
                .with_metadata(1, 1)
        }
    }

    /// Conservative source-plus-encoded-buffer overlap for one encode call.
    pub const fn maximum_encoding_wire_bytes(&self) -> usize {
        self.send_limits.max_frame_bytes().saturating_mul(2)
    }

    /// Production-only installation that proves the wire-frame class was
    /// declared before this protocol can emit budgeted frames.
    pub fn with_production_resources(
        self,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        let ledger = resources.claim(self.production_resource_requirements())?;
        Ok(self.with_resource_ledger(ledger))
    }

    pub fn encode<Message>(
        &self,
        message: &Message,
    ) -> Result<WireFrame, WireProtocolError<C::Error>>
    where
        C: WireCodec<Message>,
    {
        match &self.resource_ledger {
            Some(ledger) => {
                let reservation = WireFrame::reserve_bytes(
                    ledger,
                    pwright_core::ResourceScope::Connection,
                    self.maximum_encoding_wire_bytes(),
                )
                .map_err(WireProtocolError::Frame)?;
                let parts = self
                    .codec
                    .encode_parts(message)
                    .map_err(WireProtocolError::Codec)?;
                let (route, payload, attachments) = parts.into_parts();
                WireFrame::new_reserved_up_to(
                    self.send_version,
                    route,
                    payload,
                    attachments,
                    self.send_limits,
                    reservation,
                )
            }
            None => {
                let parts = self
                    .codec
                    .encode_parts(message)
                    .map_err(WireProtocolError::Codec)?;
                let (route, payload, attachments) = parts.into_parts();
                WireFrame::new(
                    self.send_version,
                    route,
                    payload,
                    attachments,
                    self.send_limits,
                )
            }
        }
        .map_err(WireProtocolError::Frame)
    }

    pub fn decode<Message>(&self, frame: WireFrame) -> Result<Message, WireProtocolError<C::Error>>
    where
        C: WireCodec<Message>,
    {
        self.versions
            .validate(frame.version())
            .map_err(WireProtocolError::Version)?;
        frame
            .validate(self.receive_limits)
            .map_err(WireProtocolError::Frame)?;
        self.codec
            .decode_frame(frame)
            .map_err(WireProtocolError::Codec)
    }

    /// Returns the outbound limit.
    ///
    /// This accessor predates directional limits. New code should use
    /// [`Self::send_limits`] and [`Self::receive_limits`] explicitly.
    pub const fn limits(&self) -> FrameLimits {
        self.send_limits
    }

    pub const fn send_limits(&self) -> FrameLimits {
        self.send_limits
    }

    pub const fn receive_limits(&self) -> FrameLimits {
        self.receive_limits
    }

    pub const fn send_version(&self) -> ProtocolVersion {
        self.send_version
    }

    pub const fn versions(&self) -> VersionPolicy {
        self.versions
    }
}

/// Sends and receives already-bounded frames without selecting an executor.
///
/// `send` is a cancellation boundary: dropping its future must either leave
/// the stream untouched or retain enough transport-owned state to finish the
/// same frame before any later frame. An implementation must never let a
/// dropped partial write corrupt framing. `receive` futures may likewise be
/// retained by a higher-level driver while it sends on the independent write
/// path; callers must not start two receives concurrently. A higher-level
/// channel that needs to distinguish an unsent queued call from an interrupted
/// active send must serialize all writers before polling `send`.
pub trait FrameTransport: Send + Sync {
    type Error;
    type Send<'a>: Future<Output = Result<(), Self::Error>> + Send
    where
        Self: 'a;
    type Receive<'a>: Future<Output = Result<Option<WireFrame>, Self::Error>> + Send
    where
        Self: 'a;

    fn send(&self, frame: WireFrame) -> Self::Send<'_>;
    fn receive(&self) -> Self::Receive<'_>;
}

/// Opt-in write-half shutdown for transports with an owned connection boundary.
///
/// Shutdown must serialize with [`FrameTransport::send`], finish any transport-
/// owned partial frame, flush it, and then make the peer observe clean EOF after
/// all preceding frames. Dropping a shutdown future must leave enough state for
/// a later shutdown call to resume safely. Higher-level connection owners must
/// prevent a new send from starting after shutdown completes.
pub trait ShutdownFrameTransport: FrameTransport {
    type Shutdown<'a>: Future<Output = Result<(), Self::Error>> + Send
    where
        Self: 'a;

    fn shutdown(&self) -> Self::Shutdown<'_>;
}

impl<Transport> FrameTransport for Arc<Transport>
where
    Transport: FrameTransport + ?Sized,
{
    type Error = Transport::Error;
    type Send<'a>
        = Transport::Send<'a>
    where
        Self: 'a;
    type Receive<'a>
        = Transport::Receive<'a>
    where
        Self: 'a;

    fn send(&self, frame: WireFrame) -> Self::Send<'_> {
        Transport::send(self, frame)
    }

    fn receive(&self) -> Self::Receive<'_> {
        Transport::receive(self)
    }
}

impl<Transport> ShutdownFrameTransport for Arc<Transport>
where
    Transport: ShutdownFrameTransport + ?Sized,
{
    type Shutdown<'a>
        = Transport::Shutdown<'a>
    where
        Self: 'a;

    fn shutdown(&self) -> Self::Shutdown<'_> {
        Transport::shutdown(self.as_ref())
    }
}

/// Evidence exposed by an explicitly resource-bounded production transport.
#[derive(Clone, Copy)]
pub struct ProductionTransportResources<'a> {
    ledger: &'a ResourceLedger,
    requirements: ResourceRequirements,
}

impl<'a> ProductionTransportResources<'a> {
    pub const fn new(ledger: &'a ResourceLedger, requirements: ResourceRequirements) -> Self {
        Self {
            ledger,
            requirements,
        }
    }

    pub const fn ledger(self) -> &'a ResourceLedger {
        self.ledger
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

/// Opt-in marker implemented by transports whose retained production state is
/// charged to an inspectable connection ledger. Its requirements include one
/// simultaneously live outbound input frame and inbound output frame, and its
/// implementation must reject outbound frames charged to another ledger.
pub trait ProductionFrameTransport: FrameTransport {
    fn production_resources(&self) -> ProductionTransportResources<'_>;
}

impl<Transport> ProductionFrameTransport for Arc<Transport>
where
    Transport: ProductionFrameTransport + ?Sized,
{
    fn production_resources(&self) -> ProductionTransportResources<'_> {
        Transport::production_resources(self)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::future::{Future, Ready, ready};
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    use pwright_core::{
        ProductionResourceLedger, ProtocolVersion, ResourceCapacityBoundary, ResourceClass,
        ResourceLedger, ResourceLimits, ResourceRequirements,
    };

    use crate::{FrameError, FrameLimits, FrameRoute, WireFrame};

    use super::{
        EncodedParts, FrameTransport, VersionPolicy, VersionPolicyError, WireCodec, WireProtocol,
        WireProtocolError,
    };

    struct TextCodec {
        decode_calls: Arc<AtomicUsize>,
    }

    impl WireCodec<String> for TextCodec {
        type Error = &'static str;

        fn encode_parts(&self, message: &String) -> Result<EncodedParts, Self::Error> {
            Ok(EncodedParts::new(
                FrameRoute::Control,
                message.as_bytes().to_vec(),
                Vec::new(),
            ))
        }

        fn decode_frame(&self, frame: WireFrame) -> Result<String, Self::Error> {
            self.decode_calls.fetch_add(1, Ordering::SeqCst);
            String::from_utf8(frame.payload().to_vec()).map_err(|_| "invalid utf-8")
        }
    }

    struct InspectingCodec {
        ledger: ResourceLedger,
        observed_wire_bytes: Arc<AtomicUsize>,
        route: FrameRoute,
        fail: bool,
    }

    impl WireCodec<String> for InspectingCodec {
        type Error = &'static str;

        fn encode_parts(&self, message: &String) -> Result<EncodedParts, Self::Error> {
            let current = self
                .ledger
                .metrics()
                .expect("codec should read ledger metrics")
                .class(ResourceClass::WireBytes)
                .current;
            self.observed_wire_bytes.store(
                usize::try_from(current).expect("test count should fit usize"),
                Ordering::SeqCst,
            );
            if self.fail {
                return Err("injected encode failure");
            }
            Ok(EncodedParts::new(
                self.route,
                message.as_bytes().to_vec(),
                Vec::new(),
            ))
        }

        fn decode_frame(&self, frame: WireFrame) -> Result<String, Self::Error> {
            String::from_utf8(frame.payload().to_vec()).map_err(|_| "invalid utf-8")
        }
    }

    fn policy() -> VersionPolicy {
        VersionPolicy::new(ProtocolVersion::new(1, 1), ProtocolVersion::new(1, 3))
            .expect("version range should be valid")
    }

    fn limits() -> FrameLimits {
        FrameLimits::new(4, 0, 0, 4)
    }

    #[test]
    fn version_policy_rejects_invalid_ranges() {
        // Arrange
        let cross_major = (ProtocolVersion::new(1, 0), ProtocolVersion::new(2, 0));
        let descending = (ProtocolVersion::new(1, 3), ProtocolVersion::new(1, 2));

        // Act
        let cross_major_error = VersionPolicy::new(cross_major.0, cross_major.1)
            .expect_err("cross-major range should fail");
        let descending_error = VersionPolicy::new(descending.0, descending.1)
            .expect_err("descending range should fail");

        // Assert
        assert_eq!(
            cross_major_error,
            VersionPolicyError::InvalidRange {
                minimum: cross_major.0,
                maximum: cross_major.1,
            }
        );
        assert_eq!(
            descending_error,
            VersionPolicyError::InvalidRange {
                minimum: descending.0,
                maximum: descending.1,
            }
        );
    }

    #[test]
    fn version_policy_accepts_only_its_inclusive_minor_range() {
        // Arrange
        let policy = policy();

        // Act
        let minimum = policy.validate(ProtocolVersion::new(1, 1));
        let maximum = policy.validate(ProtocolVersion::new(1, 3));
        let too_old = policy.validate(ProtocolVersion::new(1, 0));
        let too_new = policy.validate(ProtocolVersion::new(1, 4));
        let wrong_major = policy.validate(ProtocolVersion::new(2, 1));

        // Assert
        assert_eq!(minimum, Ok(()));
        assert_eq!(maximum, Ok(()));
        assert!(matches!(
            too_old,
            Err(VersionPolicyError::Unsupported { .. })
        ));
        assert!(matches!(
            too_new,
            Err(VersionPolicyError::Unsupported { .. })
        ));
        assert!(matches!(
            wrong_major,
            Err(VersionPolicyError::Unsupported { .. })
        ));
    }

    #[test]
    fn protocol_enforces_frame_limits_after_codec_encoding() {
        // Arrange
        let protocol = WireProtocol::new(
            TextCodec {
                decode_calls: Arc::new(AtomicUsize::new(0)),
            },
            policy(),
            ProtocolVersion::new(1, 3),
            limits(),
        )
        .expect("protocol should build");

        // Act
        let error = protocol
            .encode(&"oversized".to_string())
            .expect_err("codec output should still be bounded");

        // Assert
        assert!(matches!(error, WireProtocolError::Frame(_)));
    }

    #[test]
    fn protocol_with_shared_ledger_bounds_aggregate_encoded_frames() {
        // Arrange
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(2, 2).with_class(
            ResourceClass::WireBytes,
            8,
            8,
        ));
        let protocol = WireProtocol::new(
            TextCodec {
                decode_calls: Arc::new(AtomicUsize::new(0)),
            },
            policy(),
            ProtocolVersion::new(1, 3),
            limits(),
        )
        .expect("protocol should build")
        .with_resource_ledger(ledger.clone());
        let first = protocol
            .encode(&"four".to_owned())
            .expect("first frame should consume the wire budget");

        // Act
        let blocked = protocol
            .encode(&"x".to_owned())
            .expect_err("second retained frame should exceed aggregate budget");
        drop(first);
        let retried = protocol.encode(&"x".to_owned());

        // Assert
        assert!(matches!(
            blocked,
            WireProtocolError::Frame(FrameError::Resource(_))
        ));
        assert!(retried.is_ok());
        assert_eq!(
            ledger
                .metrics()
                .expect("metrics should be available")
                .class(ResourceClass::WireBytes)
                .current,
            1
        );
    }

    #[test]
    fn budgeted_protocol_reserves_before_codec_allocation_then_shrinks_and_moves() {
        // Arrange
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::WireBytes,
            8,
            8,
        ));
        let observed = Arc::new(AtomicUsize::new(0));
        let call_id = pwright_core::CallId::new(9);
        let protocol = WireProtocol::new(
            InspectingCodec {
                ledger: ledger.clone(),
                observed_wire_bytes: Arc::clone(&observed),
                route: FrameRoute::Result { call_id },
                fail: false,
            },
            policy(),
            ProtocolVersion::new(1, 3),
            limits(),
        )
        .expect("protocol should build")
        .with_resource_ledger(ledger.clone());

        // Act
        let frame = protocol
            .encode(&"x".to_string())
            .expect("one-byte result should encode");

        // Assert
        assert_eq!(observed.load(Ordering::SeqCst), 8);
        let metrics = ledger.metrics().expect("metrics should be readable");
        assert_eq!(metrics.class(ResourceClass::WireBytes).peak, 8);
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 1);
        assert!(
            ledger
                .scope_metrics(pwright_core::ResourceScope::Connection)
                .expect("scope metrics should load")
                .is_none()
        );
        assert_eq!(
            ledger
                .scope_metrics(pwright_core::ResourceScope::Call(call_id))
                .expect("scope metrics should load")
                .expect("final call scope should exist")
                .units(ResourceClass::WireBytes),
            1
        );
        drop(frame);
    }

    #[test]
    fn codec_failure_releases_the_preallocation_reservation() {
        // Arrange
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::WireBytes,
            8,
            8,
        ));
        let observed = Arc::new(AtomicUsize::new(0));
        let protocol = WireProtocol::new(
            InspectingCodec {
                ledger: ledger.clone(),
                observed_wire_bytes: Arc::clone(&observed),
                route: FrameRoute::Control,
                fail: true,
            },
            policy(),
            ProtocolVersion::new(1, 3),
            limits(),
        )
        .expect("protocol should build")
        .with_resource_ledger(ledger.clone());

        // Act
        let error = protocol
            .encode(&"x".to_string())
            .expect_err("injected codec failure must surface");

        // Assert
        assert_eq!(error, WireProtocolError::Codec("injected encode failure"));
        assert_eq!(observed.load(Ordering::SeqCst), 8);
        let metrics = ledger.metrics().expect("metrics should be readable");
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(metrics.active_leases, 0);
    }

    #[test]
    fn production_protocol_claims_its_maximum_outbound_frame_requirement() {
        // Arrange
        let protocol = WireProtocol::new(
            TextCodec {
                decode_calls: Arc::new(AtomicUsize::new(0)),
            },
            policy(),
            ProtocolVersion::new(1, 3),
            limits(),
        )
        .expect("protocol should build");
        let requirements = protocol.production_resource_requirements();
        let class = requirements.class(ResourceClass::WireBytes);
        let production = ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::WireBytes,
                class.global,
                class.per_scope,
            ),
            requirements,
        )
        .expect("exact production plan should fit");
        let ledger = production
            .claim(requirements)
            .expect("test should observe the shared ledger");
        let protocol = protocol
            .with_production_resources(&production)
            .expect("declared protocol should install");

        // Act
        let frame = protocol
            .encode(&"four".to_owned())
            .expect("maximum-size frame should fit");
        let metrics = production.metrics().expect("metrics should load");

        // Assert
        assert_eq!(class.global, 8);
        assert!(frame.is_budgeted_by(&ledger));
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 4);
    }

    #[test]
    fn production_protocol_rejects_an_omitted_wire_class() {
        // Arrange
        let protocol = WireProtocol::new(
            TextCodec {
                decode_calls: Arc::new(AtomicUsize::new(0)),
            },
            policy(),
            ProtocolVersion::new(1, 3),
            limits(),
        )
        .expect("protocol should build");
        let production = ProductionResourceLedger::new(
            ResourceLimits::deny_all(1, 1).with_class(ResourceClass::WireBytes, 4, 4),
            ResourceRequirements::none(),
        )
        .expect("empty plan may use stricter configured limits");

        // Act
        let error = match protocol.with_production_resources(&production) {
            Ok(_) => panic!("undeclared protocol must not install"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(
            error,
            pwright_core::ProductionResourceError::InsufficientGlobal {
                boundary: ResourceCapacityBoundary::DeclaredPlan,
                class: ResourceClass::WireBytes,
                required: 8,
                available: 0,
            }
        ));
    }

    #[test]
    fn protocol_applies_directional_send_and_receive_limits_independently() {
        // Arrange
        let send_limits = FrameLimits::new(4, 0, 0, 4);
        let receive_limits = FrameLimits::new(16, 0, 0, 16);
        let protocol = WireProtocol::new_directional(
            TextCodec {
                decode_calls: Arc::new(AtomicUsize::new(0)),
            },
            policy(),
            ProtocolVersion::new(1, 2),
            send_limits,
            receive_limits,
        )
        .expect("directional protocol should build");
        let inbound = WireFrame::new(
            ProtocolVersion::new(1, 2),
            FrameRoute::Control,
            b"eight!!!".to_vec(),
            Vec::new(),
            receive_limits,
        )
        .expect("inbound frame should fit the receive limit");

        // Act
        let outbound = protocol.encode(&"eight!!!".to_string());
        let decoded = protocol.decode::<String>(inbound);

        // Assert
        assert!(matches!(outbound, Err(WireProtocolError::Frame(_))));
        assert_eq!(decoded.as_deref(), Ok("eight!!!"));
        assert_eq!(protocol.limits(), send_limits);
        assert_eq!(protocol.send_limits(), send_limits);
        assert_eq!(protocol.receive_limits(), receive_limits);
    }

    #[test]
    fn protocol_rejects_version_before_invoking_decoder() {
        // Arrange
        let decode_calls = Arc::new(AtomicUsize::new(0));
        let protocol = WireProtocol::new(
            TextCodec {
                decode_calls: Arc::clone(&decode_calls),
            },
            policy(),
            ProtocolVersion::new(1, 2),
            limits(),
        )
        .expect("protocol should build");
        let frame = WireFrame::new(
            ProtocolVersion::new(2, 0),
            FrameRoute::Control,
            b"ok".to_vec(),
            Vec::new(),
            limits(),
        )
        .expect("bounded frame should build");

        // Act
        let error = protocol
            .decode::<String>(frame)
            .expect_err("unsupported version should fail");

        // Assert
        assert!(matches!(error, WireProtocolError::Version(_)));
        assert_eq!(decode_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn protocol_revalidates_received_frame_against_local_limits() {
        // Arrange
        let decode_calls = Arc::new(AtomicUsize::new(0));
        let protocol = WireProtocol::new(
            TextCodec {
                decode_calls: Arc::clone(&decode_calls),
            },
            policy(),
            ProtocolVersion::new(1, 2),
            limits(),
        )
        .expect("protocol should build");
        let permissive_limits = FrameLimits::new(16, 0, 0, 16);
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 2),
            FrameRoute::Control,
            b"too large".to_vec(),
            Vec::new(),
            permissive_limits,
        )
        .expect("upstream transport may use wider limits");

        // Act
        let error = protocol
            .decode::<String>(frame)
            .expect_err("local protocol limit must still apply");

        // Assert
        assert!(matches!(error, WireProtocolError::Frame(_)));
        assert_eq!(decode_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn protocol_round_trips_through_a_pluggable_codec() {
        // Arrange
        let protocol = WireProtocol::new(
            TextCodec {
                decode_calls: Arc::new(AtomicUsize::new(0)),
            },
            policy(),
            ProtocolVersion::new(1, 2),
            limits(),
        )
        .expect("protocol should build");

        // Act
        let frame = protocol
            .encode(&"test".to_string())
            .expect("message should encode");
        let decoded = protocol
            .decode::<String>(frame)
            .expect("message should decode");

        // Assert
        assert_eq!(decoded, "test");
    }

    struct MemoryTransport {
        frames: Mutex<VecDeque<WireFrame>>,
    }

    impl FrameTransport for MemoryTransport {
        type Error = ();
        type Send<'a> = Ready<Result<(), Self::Error>>;
        type Receive<'a> = Ready<Result<Option<WireFrame>, Self::Error>>;

        fn send(&self, frame: WireFrame) -> Self::Send<'_> {
            self.frames
                .lock()
                .expect("memory transport should be available")
                .push_back(frame);
            ready(Ok(()))
        }

        fn receive(&self) -> Self::Receive<'_> {
            ready(Ok(self
                .frames
                .lock()
                .expect("memory transport should be available")
                .pop_front()))
        }
    }

    fn poll_ready<F>(future: &mut F) -> F::Output
    where
        F: Future + Unpin,
    {
        let mut context = Context::from_waker(Waker::noop());
        let Poll::Ready(output) = Pin::new(future).poll(&mut context) else {
            panic!("ready test future should complete");
        };
        output
    }

    #[test]
    fn transport_contract_moves_bounded_frames_without_a_runtime() {
        // Arrange
        let transport = MemoryTransport {
            frames: Mutex::new(VecDeque::new()),
        };
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 2),
            FrameRoute::Handshake,
            b"hi".to_vec(),
            Vec::new(),
            limits(),
        )
        .expect("frame should build");

        // Act
        let mut send = transport.send(frame);
        poll_ready(&mut send).expect("send should succeed");
        let mut receive = transport.receive();
        let received = poll_ready(&mut receive)
            .expect("receive should succeed")
            .expect("one frame should be present");

        // Assert
        assert_eq!(received.route(), FrameRoute::Handshake);
        assert_eq!(received.payload(), b"hi");
    }
}
