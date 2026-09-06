//! Caller-polled delivery for Chromium-normalized HTTP lifecycle events.
//!
//! Request, response, finished, and failed events intentionally share one
//! queue and one sequence source. This leaf also owns their creation fence and
//! retained receive/publish/acknowledge turn through injected batch-driver and
//! subscription-selection ports. CDP receiving and normalization, listener
//! mutation, topology ownership, executors, tasks, browsers, and processes
//! remain injected or outside this crate.

mod lifecycle;

pub use lifecycle::{
    ChromiumDirectNetworkEventScopeSelector, ChromiumNetworkBatchReceiveFuture,
    ChromiumNetworkEventBatchDriver, ChromiumNetworkEventScopeSelector, ChromiumNetworkEventScopes,
    ChromiumNetworkLifecycleBackend, ChromiumNetworkLifecycleBackendError,
    ChromiumNetworkLifecycleBackendOutcome, ChromiumNetworkLifecyclePublication,
    ChromiumNetworkLifecyclePublishError, ChromiumNetworkLifecyclePublisher,
    ChromiumNetworkLifecyclePumpError, ChromiumNetworkLifecycleRuntime,
    drive_chromium_network_lifecycle_once,
};

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use pwright_cap_network_events::{NetworkEventStream, network_stream_event_tag};
pub use pwright_codec_network_events::{NetworkEventsCodec, NetworkEventsCodecLimits};
use pwright_core::{
    OverflowPolicy, ProductionResourceLedger, ProtocolVersion, ResourceRequirements,
};
use pwright_server_events::{
    ServerEventProductionError, ServerEventProductionProfile, TypedEventOutbound, TypedEventSender,
};
pub use pwright_transport::FrameLimits;
use pwright_transport::{ProductionFrameTransport, WireProtocol};

pub type ChromiumNetworkEventSender = TypedEventSender<NetworkEventStream>;
pub type ChromiumNetworkEventOutbound<Transport> =
    TypedEventOutbound<NetworkEventStream, NetworkEventsCodec, Arc<Transport>>;

/// Exact wire, codec, queue, and decoded-retention bounds for one globally
/// ordered network-event stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumNetworkEventDeliveryProfile {
    wire_version: ProtocolVersion,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
    codec: NetworkEventsCodecLimits,
    capacity: usize,
    max_retained_bytes_per_event: usize,
}

impl ChromiumNetworkEventDeliveryProfile {
    pub const fn new(
        wire_version: ProtocolVersion,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
        codec: NetworkEventsCodecLimits,
        capacity: usize,
        max_retained_bytes_per_event: usize,
    ) -> Self {
        Self {
            wire_version,
            send_limits,
            receive_limits,
            codec,
            capacity,
            max_retained_bytes_per_event,
        }
    }

    pub const fn wire_version(self) -> ProtocolVersion {
        self.wire_version
    }

    pub const fn send_limits(self) -> FrameLimits {
        self.send_limits
    }

    pub const fn receive_limits(self) -> FrameLimits {
        self.receive_limits
    }

    pub const fn codec(self) -> NetworkEventsCodecLimits {
        self.codec
    }

    pub const fn capacity(self) -> usize {
        self.capacity
    }

    pub const fn max_retained_bytes_per_event(self) -> usize {
        self.max_retained_bytes_per_event
    }

    pub fn requirements(self) -> ResourceRequirements {
        self.production().requirements()
    }

    fn validate(self) -> Result<Self, ChromiumNetworkEventDeliverySetupError> {
        if self.capacity == 0 {
            return Err(ChromiumNetworkEventDeliverySetupError::ZeroCapacity);
        }
        let required_payload_bytes = self.codec.maximum_payload_bytes();
        if self.send_limits.payload_bytes < required_payload_bytes
            || self.send_limits.total_bytes < required_payload_bytes
        {
            return Err(
                ChromiumNetworkEventDeliverySetupError::OutboundFrameLimitTooSmall {
                    required_payload_bytes,
                    actual: self.send_limits,
                },
            );
        }
        let required = self.codec.maximum_retained_event_bytes();
        if self.max_retained_bytes_per_event < required {
            return Err(
                ChromiumNetworkEventDeliverySetupError::RetainedBytesTooSmall {
                    required,
                    actual: self.max_retained_bytes_per_event,
                },
            );
        }
        Ok(self)
    }

    fn protocol(self) -> WireProtocol<NetworkEventsCodec> {
        WireProtocol::new_exact_directional(
            NetworkEventsCodec::new(self.codec),
            self.wire_version,
            self.send_limits,
            self.receive_limits,
        )
    }

    fn production(self) -> ServerEventProductionProfile {
        ServerEventProductionProfile::new(
            &self.protocol(),
            self.capacity,
            self.max_retained_bytes_per_event,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumNetworkEventDeliverySetupError {
    ZeroCapacity,
    OutboundFrameLimitTooSmall {
        required_payload_bytes: usize,
        actual: FrameLimits,
    },
    RetainedBytesTooSmall {
        required: usize,
        actual: usize,
    },
}

impl Display for ChromiumNetworkEventDeliverySetupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCapacity => {
                formatter.write_str("network-event delivery requires nonzero queue capacity")
            }
            Self::OutboundFrameLimitTooSmall {
                required_payload_bytes,
                actual,
            } => write!(
                formatter,
                "network-event delivery requires outbound payload and total-frame limits of at least {required_payload_bytes} bytes, got {actual:?}"
            ),
            Self::RetainedBytesTooSmall { required, actual } => write!(
                formatter,
                "network-event delivery requires at least {required} retained bytes per event, got {actual}"
            ),
        }
    }
}

impl Error for ChromiumNetworkEventDeliverySetupError {}

#[derive(Debug)]
pub enum ChromiumNetworkEventDeliveryInstallError {
    Setup(ChromiumNetworkEventDeliverySetupError),
    Production(ServerEventProductionError),
}

impl Display for ChromiumNetworkEventDeliveryInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setup(error) => Display::fmt(error, formatter),
            Self::Production(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumNetworkEventDeliveryInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Setup(error) => Some(error),
            Self::Production(error) => Some(error),
        }
    }
}

impl From<ChromiumNetworkEventDeliverySetupError> for ChromiumNetworkEventDeliveryInstallError {
    fn from(error: ChromiumNetworkEventDeliverySetupError) -> Self {
        Self::Setup(error)
    }
}

impl From<ServerEventProductionError> for ChromiumNetworkEventDeliveryInstallError {
    fn from(error: ServerEventProductionError) -> Self {
        Self::Production(error)
    }
}

/// Fully bound ordered queue. It remains inert until a caller emits through
/// the sender or polls the outbound lane.
pub struct ChromiumNetworkEventDeliveryInstallation<Transport: ?Sized> {
    sender: ChromiumNetworkEventSender,
    outbound: ChromiumNetworkEventOutbound<Transport>,
}

impl<Transport> ChromiumNetworkEventDeliveryInstallation<Transport>
where
    Transport: ProductionFrameTransport + ?Sized,
{
    pub fn install(
        transport: Arc<Transport>,
        profile: ChromiumNetworkEventDeliveryProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumNetworkEventDeliveryInstallError> {
        let profile = profile.validate()?;
        let (sender, outbound) =
            TypedEventOutbound::<NetworkEventStream, _, _>::new_routed_with_production_resources(
                profile.protocol(),
                transport,
                profile.production(),
                OverflowPolicy::RejectNewest,
                resources,
                |event| event.payload().retained_bytes(),
                network_stream_event_tag,
            )?;
        Ok(Self { sender, outbound })
    }

    pub fn into_parts(
        self,
    ) -> (
        ChromiumNetworkEventSender,
        ChromiumNetworkEventOutbound<Transport>,
    ) {
        (self.sender, self.outbound)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fmt::{self, Display, Formatter};
    use std::future::{Ready, ready};
    use std::sync::{Arc, Mutex};

    use pwright_cap_network_events::{NetworkEventPayload, NetworkEventStream, RequestInitializer};
    use pwright_core::{
        Event, ObjectId, ProductionResourceLedger, PushOutcome, ResourceClass, ResourceLimits,
        ResourceRequirements, SequencedEvent,
    };
    use pwright_server_wire::ConcurrentServerOutbound;
    use pwright_transport::{
        FrameTransport, ProductionFrameTransport, ProductionTransportResources, WireFrame,
    };

    use super::{
        ChromiumNetworkEventDeliveryInstallation, ChromiumNetworkEventDeliveryProfile,
        ChromiumNetworkEventDeliverySetupError, NetworkEventsCodecLimits,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestTransportError;

    impl Display for TestTransportError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("test transport failed")
        }
    }

    impl Error for TestTransportError {}

    struct TestTransport {
        sent: Arc<Mutex<Vec<WireFrame>>>,
        ledger: pwright_core::ResourceLedger,
    }

    impl FrameTransport for TestTransport {
        type Error = TestTransportError;
        type Send<'a>
            = Ready<Result<(), Self::Error>>
        where
            Self: 'a;
        type Receive<'a>
            = Ready<Result<Option<WireFrame>, Self::Error>>
        where
            Self: 'a;

        fn send(&self, frame: WireFrame) -> Self::Send<'_> {
            self.sent
                .lock()
                .expect("sent-frame state should be available")
                .push(frame);
            ready(Ok(()))
        }

        fn receive(&self) -> Self::Receive<'_> {
            ready(Ok(None))
        }
    }

    impl ProductionFrameTransport for TestTransport {
        fn production_resources(&self) -> ProductionTransportResources<'_> {
            ProductionTransportResources::new(&self.ledger, ResourceRequirements::none())
        }
    }

    fn codec() -> NetworkEventsCodecLimits {
        NetworkEventsCodecLimits {
            url_bytes: 32,
            method_bytes: 8,
            resource_type_bytes: 16,
            header_count: 2,
            header_bytes: 32,
            post_data_bytes: 16,
            status_text_bytes: 16,
            failure_text_bytes: 32,
        }
    }

    fn profile() -> ChromiumNetworkEventDeliveryProfile {
        let codec = codec();
        ChromiumNetworkEventDeliveryProfile::new(
            pwright_core::ProtocolVersion::new(1, 0),
            super::FrameLimits::new(1024, 0, 0, 1024),
            super::FrameLimits::new(1024, 0, 0, 1024),
            codec,
            2,
            codec.maximum_retained_event_bytes(),
        )
    }

    fn resources(profile: ChromiumNetworkEventDeliveryProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let event_items = requirements.class(ResourceClass::EventItem);
        let event_bytes = requirements.class(ResourceClass::EventBytes);
        let wire = requirements.class(ResourceClass::WireBytes);
        let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases)
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
            .with_class(ResourceClass::WireBytes, wire.global, wire.per_scope);
        ProductionResourceLedger::new(limits, requirements)
            .expect("exact network-event delivery resources should build")
    }

    #[test]
    fn ordered_delivery_requires_nonzero_capacity() {
        // Arrange
        let codec = codec();
        let profile = ChromiumNetworkEventDeliveryProfile::new(
            pwright_core::ProtocolVersion::new(1, 0),
            super::FrameLimits::new(1024, 0, 0, 1024),
            super::FrameLimits::new(1024, 0, 0, 1024),
            codec,
            0,
            codec.maximum_retained_event_bytes(),
        );

        // Act
        let error = profile
            .validate()
            .expect_err("a zero-capacity ordered queue should fail");

        // Assert
        assert_eq!(error, ChromiumNetworkEventDeliverySetupError::ZeroCapacity);
    }

    #[test]
    fn ordered_delivery_requires_the_terminal_snapshot_retention_bound() {
        // Arrange
        let codec = codec();
        let required = codec.maximum_retained_event_bytes();
        let profile = ChromiumNetworkEventDeliveryProfile::new(
            pwright_core::ProtocolVersion::new(1, 0),
            super::FrameLimits::new(1024, 0, 0, 1024),
            super::FrameLimits::new(1024, 0, 0, 1024),
            codec,
            1,
            required - 1,
        );

        // Act
        let error = profile
            .validate()
            .expect_err("an under-budgeted terminal snapshot should fail");

        // Assert
        assert_eq!(
            error,
            ChromiumNetworkEventDeliverySetupError::RetainedBytesTooSmall {
                required,
                actual: required - 1,
            }
        );
    }

    #[test]
    fn ordered_delivery_requires_the_largest_terminal_frame_bound() {
        // Arrange
        let codec = codec();
        let required_payload_bytes = codec.maximum_payload_bytes();
        let actual =
            super::FrameLimits::new(required_payload_bytes - 1, 0, 0, required_payload_bytes - 1);
        let profile = ChromiumNetworkEventDeliveryProfile::new(
            pwright_core::ProtocolVersion::new(1, 0),
            actual,
            super::FrameLimits::new(1024, 0, 0, 1024),
            codec,
            1,
            codec.maximum_retained_event_bytes(),
        );

        // Act
        let error = profile
            .validate()
            .expect_err("an undersized terminal frame should fail");

        // Assert
        assert_eq!(
            error,
            ChromiumNetworkEventDeliverySetupError::OutboundFrameLimitTooSmall {
                required_payload_bytes,
                actual,
            }
        );
    }

    #[tokio::test]
    async fn one_queue_preserves_the_public_event_route() {
        // Arrange
        let selected_profile = profile();
        let resources = resources(selected_profile);
        let sent = Arc::new(Mutex::new(Vec::new()));
        let transport = Arc::new(TestTransport {
            sent: Arc::clone(&sent),
            ledger: resources
                .claim(ResourceRequirements::none())
                .expect("test transport should share the delivery ledger"),
        });
        let installation = ChromiumNetworkEventDeliveryInstallation::install(
            transport,
            selected_profile,
            &resources,
        )
        .expect("network-event delivery should install");
        let (sender, outbound) = installation.into_parts();
        let request = RequestInitializer {
            request: ObjectId::new(4),
            parent: ObjectId::new(3),
            url: "https://example.test/".to_owned(),
            method: "GET".to_owned(),
            resource_type: "Document".to_owned(),
            headers: Vec::new(),
            post_data: None,
            is_navigation_request: true,
            redirected_from: None,
        };

        // Act
        let admitted = sender
            .emit(Event::new(
                ObjectId::new(3),
                NetworkEventPayload::Request(request),
            ))
            .expect("request should fit the queue");
        outbound
            .send_once()
            .await
            .expect("request frame should send")
            .expect("request should be queued");

        // Assert
        assert!(matches!(admitted, PushOutcome::Enqueued));
        let frames = sent.lock().expect("sent frames should be available");
        let decoded = selected_profile
            .protocol()
            .decode::<SequencedEvent<NetworkEventStream>>(frames[0].clone())
            .expect("network request frame should decode");
        assert_eq!(decoded.object_id(), ObjectId::new(3));
        assert!(matches!(
            decoded.payload(),
            NetworkEventPayload::Request(payload) if payload.request == ObjectId::new(4)
        ));
    }
}
