//! Opt-in client helpers for one globally ordered HTTP lifecycle stream.

use pwright_cap_network_events::{
    NetworkEventStream, NetworkRequest, NetworkRequestFailed, NetworkRequestFinished,
    NetworkResponse, is_network_event_tag,
};
use pwright_client::{BrowserContext, CallIdSource, Page};
use pwright_client_events::{SubscriptionError, SubscriptionTracker, SubscriptionTransition};
use pwright_client_events::{TypedEventHandler, TypedEventProductionProfile};
use pwright_codec_network_events::NetworkEventsCodec;
use pwright_core::{Event, OverflowPolicy, ProductionResourceError, ProductionResourceLedger};
use pwright_transport::WireProtocol;

pub type NetworkEventHandler = TypedEventHandler<NetworkEventStream, NetworkEventsCodec>;

pub fn network_event_handler(
    protocol: WireProtocol<NetworkEventsCodec>,
    capacity: usize,
) -> NetworkEventHandler {
    TypedEventHandler::new_routed(
        protocol,
        capacity,
        OverflowPolicy::RejectNewest,
        is_network_event_tag,
    )
}

pub fn production_network_event_handler(
    protocol: WireProtocol<NetworkEventsCodec>,
    profile: TypedEventProductionProfile,
    resources: &ProductionResourceLedger,
) -> Result<NetworkEventHandler, ProductionResourceError> {
    TypedEventHandler::new_routed_with_production_resources(
        protocol,
        profile,
        OverflowPolicy::RejectNewest,
        resources,
        retained_network_event_bytes,
        is_network_event_tag,
    )
}

pub fn retained_network_event_bytes(event: &Event<NetworkEventStream>) -> usize {
    event.payload().retained_bytes()
}

/// Page-scoped listener edges for the four causally ordered HTTP lifecycle
/// events.
pub trait PageNetworkSubscriptionExt {
    fn subscribe_request(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn unsubscribe_request(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn subscribe_response(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn unsubscribe_response(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn subscribe_request_finished(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn unsubscribe_request_finished(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn subscribe_request_failed(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn unsubscribe_request_failed(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;
}

impl<Channel, Ids> PageNetworkSubscriptionExt for Page<Channel, Ids>
where
    Ids: CallIdSource,
{
    fn subscribe_request(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.subscribe(
            self.id(),
            <NetworkRequest as pwright_core::EventSpec>::EVENT,
        )
    }

    fn unsubscribe_request(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.unsubscribe(
            self.id(),
            <NetworkRequest as pwright_core::EventSpec>::EVENT,
        )
    }

    fn subscribe_response(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.subscribe(
            self.id(),
            <NetworkResponse as pwright_core::EventSpec>::EVENT,
        )
    }

    fn unsubscribe_response(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.unsubscribe(
            self.id(),
            <NetworkResponse as pwright_core::EventSpec>::EVENT,
        )
    }

    fn subscribe_request_finished(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.subscribe(
            self.id(),
            <NetworkRequestFinished as pwright_core::EventSpec>::EVENT,
        )
    }

    fn unsubscribe_request_finished(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.unsubscribe(
            self.id(),
            <NetworkRequestFinished as pwright_core::EventSpec>::EVENT,
        )
    }

    fn subscribe_request_failed(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.subscribe(
            self.id(),
            <NetworkRequestFailed as pwright_core::EventSpec>::EVENT,
        )
    }

    fn unsubscribe_request_failed(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.unsubscribe(
            self.id(),
            <NetworkRequestFailed as pwright_core::EventSpec>::EVENT,
        )
    }
}

/// BrowserContext-scoped listener edges share the same event tags and control
/// protocol as Page listeners. The server-side network driver owns atomic
/// current-Page fan-out plus inheritance, replacement, and retirement for
/// Pages that appear while these logical interests remain active.
pub trait BrowserContextNetworkSubscriptionExt {
    fn subscribe_request(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn unsubscribe_request(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn subscribe_response(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn unsubscribe_response(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn subscribe_request_finished(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn unsubscribe_request_finished(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn subscribe_request_failed(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn unsubscribe_request_failed(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;
}

impl<Channel, Ids> BrowserContextNetworkSubscriptionExt for BrowserContext<Channel, Ids>
where
    Ids: CallIdSource,
{
    fn subscribe_request(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.subscribe(
            self.id(),
            <NetworkRequest as pwright_core::EventSpec>::EVENT,
        )
    }

    fn unsubscribe_request(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.unsubscribe(
            self.id(),
            <NetworkRequest as pwright_core::EventSpec>::EVENT,
        )
    }

    fn subscribe_response(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.subscribe(
            self.id(),
            <NetworkResponse as pwright_core::EventSpec>::EVENT,
        )
    }

    fn unsubscribe_response(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.unsubscribe(
            self.id(),
            <NetworkResponse as pwright_core::EventSpec>::EVENT,
        )
    }

    fn subscribe_request_finished(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.subscribe(
            self.id(),
            <NetworkRequestFinished as pwright_core::EventSpec>::EVENT,
        )
    }

    fn unsubscribe_request_finished(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.unsubscribe(
            self.id(),
            <NetworkRequestFinished as pwright_core::EventSpec>::EVENT,
        )
    }

    fn subscribe_request_failed(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.subscribe(
            self.id(),
            <NetworkRequestFailed as pwright_core::EventSpec>::EVENT,
        )
    }

    fn unsubscribe_request_failed(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.unsubscribe(
            self.id(),
            <NetworkRequestFailed as pwright_core::EventSpec>::EVENT,
        )
    }
}

#[cfg(test)]
mod tests {
    use pwright_browser_model::{
        BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, PAGE_OBJECT_KIND,
    };
    use pwright_cap_network_events::{
        HttpHeader, NetworkEventPayload, NetworkEventStream, NetworkLifecycleLimits,
        NetworkRequest, RequestFinished, RequestInitializer, ResponseInitializer,
        network_stream_event_tag,
    };
    use pwright_client::{AtomicCallIdSource, Client, RemoteInitializer};
    use pwright_client_events::{EventContinuity, TypedEventProductionProfile};
    use pwright_client_events::{SubscriptionTracker, SubscriptionTransition};
    use pwright_client_wire::{FrameDisposition, FrameHandler};
    use pwright_codec_network_events::{NetworkEventsCodec, NetworkEventsCodecLimits};
    use pwright_core::{
        ControlMessage, Event, EventSequence, ObjectId, OverflowPolicy, ProductionResourceLedger,
        ProtocolVersion, PushOutcome, ResourceClass, ResourceLimits,
    };
    use pwright_server_events::TypedEventOutbound;
    use pwright_server_wire::ConcurrentServerOutbound;
    use pwright_transport::{FrameLimits, FrameTransport, WireProtocol};
    use pwright_transport_memory::MemoryFrameTransport;

    use super::{
        BrowserContextNetworkSubscriptionExt, PageNetworkSubscriptionExt, network_event_handler,
        production_network_event_handler,
    };

    fn protocol() -> WireProtocol<NetworkEventsCodec> {
        WireProtocol::new_exact_directional(
            NetworkEventsCodec::new(NetworkEventsCodecLimits::from_lifecycle(
                NetworkLifecycleLimits::new(8, 64, 256, 16, 32, 8, 256, 64, 64, 128),
            )),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(2048, 0, 0, 2048),
            FrameLimits::new(2048, 0, 0, 2048),
        )
    }

    fn production_resources(profile: TypedEventProductionProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            limits = limits.with_class(class, required.global, required.per_scope);
        }
        ProductionResourceLedger::new(limits, requirements)
            .expect("exact network event plan should fit")
    }

    fn small_request() -> NetworkEventPayload {
        NetworkEventPayload::Request(request_initializer())
    }

    fn request_initializer() -> RequestInitializer {
        RequestInitializer {
            request: ObjectId::new(10),
            parent: ObjectId::new(3),
            url: "https://example.test/data".to_owned(),
            method: "GET".to_owned(),
            resource_type: "Fetch".to_owned(),
            headers: vec![HttpHeader::new("accept", "application/json")],
            post_data: None,
            is_navigation_request: false,
            redirected_from: None,
        }
    }

    fn response_initializer() -> ResponseInitializer {
        ResponseInitializer {
            response: ObjectId::new(11),
            request: ObjectId::new(10),
            url: "https://example.test/data".to_owned(),
            status: 200,
            status_text: "OK".to_owned(),
            headers: vec![HttpHeader::new("content-type", "application/json")],
            from_service_worker: false,
        }
    }

    #[test]
    fn page_listener_helpers_emit_only_first_and_last_control_edges() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BROWSER_OBJECT_KIND.tag,
            ))
            .expect("browser should initialize");
        let context = browser
            .context(RemoteInitializer::new(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                BROWSER_CONTEXT_OBJECT_KIND.tag,
            ))
            .expect("context should initialize");
        let page = context
            .page(RemoteInitializer::new(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                PAGE_OBJECT_KIND.tag,
            ))
            .expect("Page should initialize");
        let mut subscriptions = SubscriptionTracker::new(1);

        // Act
        let first = page
            .subscribe_request(&mut subscriptions)
            .expect("first listener should fit");
        let second = page
            .subscribe_request(&mut subscriptions)
            .expect("second listener should share the route");
        let remove_second = page
            .unsubscribe_request(&mut subscriptions)
            .expect("one listener should remain");
        let remove_last = page
            .unsubscribe_request(&mut subscriptions)
            .expect("last listener should remove the route");

        // Assert
        assert_eq!(
            first,
            SubscriptionTransition::Send(ControlMessage::SetSubscription {
                object_id: ObjectId::new(3),
                event: <NetworkRequest as pwright_core::EventSpec>::EVENT,
                enabled: true,
            })
        );
        assert_eq!(second, SubscriptionTransition::Unchanged);
        assert_eq!(remove_second, SubscriptionTransition::Unchanged);
        assert_eq!(
            remove_last,
            SubscriptionTransition::Send(ControlMessage::SetSubscription {
                object_id: ObjectId::new(3),
                event: <NetworkRequest as pwright_core::EventSpec>::EVENT,
                enabled: false,
            })
        );
        assert_eq!(subscriptions.metrics().routes(), 0);
        assert_eq!(subscriptions.metrics().peak_listeners(), 2);
    }

    #[test]
    fn browser_context_listener_helpers_target_context_identity_on_first_and_last_edges() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BROWSER_OBJECT_KIND.tag,
            ))
            .expect("browser should initialize");
        let context = browser
            .context(RemoteInitializer::new(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                BROWSER_CONTEXT_OBJECT_KIND.tag,
            ))
            .expect("context should initialize");
        let mut subscriptions = SubscriptionTracker::new(1);

        // Act
        let first = context
            .subscribe_response(&mut subscriptions)
            .expect("first Context listener should fit");
        let second = context
            .subscribe_response(&mut subscriptions)
            .expect("second Context listener should share the route");
        let remove_second = context
            .unsubscribe_response(&mut subscriptions)
            .expect("one Context listener should remain");
        let remove_last = context
            .unsubscribe_response(&mut subscriptions)
            .expect("last Context listener should remove the route");

        // Assert
        assert_eq!(
            first,
            SubscriptionTransition::Send(ControlMessage::SetSubscription {
                object_id: ObjectId::new(2),
                event:
                    <pwright_cap_network_events::NetworkResponse as pwright_core::EventSpec>::EVENT,
                enabled: true,
            })
        );
        assert_eq!(second, SubscriptionTransition::Unchanged);
        assert_eq!(remove_second, SubscriptionTransition::Unchanged);
        assert_eq!(
            remove_last,
            SubscriptionTransition::Send(ControlMessage::SetSubscription {
                object_id: ObjectId::new(2),
                event:
                    <pwright_cap_network_events::NetworkResponse as pwright_core::EventSpec>::EVENT,
                enabled: false,
            })
        );
        assert_eq!(subscriptions.metrics().routes(), 0);
        assert_eq!(subscriptions.metrics().peak_listeners(), 2);
    }

    #[tokio::test]
    async fn one_wire_queue_preserves_request_response_finished_order_and_continuity() {
        // Arrange
        let (server_transport, client_transport) = MemoryFrameTransport::pair(3);
        let (sender, outbound) = TypedEventOutbound::<NetworkEventStream, _, _>::new_routed(
            protocol(),
            server_transport,
            3,
            OverflowPolicy::RejectNewest,
            network_stream_event_tag,
        );
        let request = request_initializer();
        let response = response_initializer();
        let values = [
            NetworkEventPayload::Request(request.clone()),
            NetworkEventPayload::Response(response.clone()),
            NetworkEventPayload::RequestFinished(RequestFinished {
                request,
                response: Some(response),
                encoded_body_bytes: Some(42),
            }),
        ];
        for value in &values {
            assert!(matches!(
                sender.emit(Event::new(ObjectId::new(3), value.clone())),
                Ok(PushOutcome::Enqueued)
            ));
        }
        let mut handler = network_event_handler(protocol(), 3);

        // Act
        for _ in 0..values.len() {
            outbound
                .send_once()
                .await
                .expect("ordered event should send")
                .expect("an event should be queued");
            let frame = client_transport
                .receive()
                .await
                .expect("client receive should succeed")
                .expect("wire should remain open");
            let handled = handler.handle(frame).expect("event should decode");
            assert!(matches!(
                handled,
                FrameDisposition::Handled(PushOutcome::Enqueued)
            ));
        }
        let received = (0..values.len())
            .map(|_| handler.pop().expect("event should remain ordered"))
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(received[0].sequence(), EventSequence::new(1));
        assert_eq!(received[0].continuity(), EventContinuity::Baseline);
        assert_eq!(received[1].sequence(), EventSequence::new(2));
        assert_eq!(received[1].continuity(), EventContinuity::Consecutive);
        assert_eq!(received[2].sequence(), EventSequence::new(3));
        assert_eq!(received[2].continuity(), EventContinuity::Consecutive);
        for (actual, expected) in received.iter().zip(values.iter()) {
            assert_eq!(actual.payload(), expected);
        }
        assert_eq!(handler.sequence_metrics().gaps(), 0);
        assert_eq!(handler.sequence_metrics().missing(), 0);
    }

    #[tokio::test]
    async fn terminal_only_delivery_contains_complete_request_and_response_values() {
        // Arrange
        let (server_transport, client_transport) = MemoryFrameTransport::pair(1);
        let (sender, outbound) = TypedEventOutbound::<NetworkEventStream, _, _>::new_routed(
            protocol(),
            server_transport,
            1,
            OverflowPolicy::RejectNewest,
            network_stream_event_tag,
        );
        let terminal = RequestFinished {
            request: request_initializer(),
            response: Some(response_initializer()),
            encoded_body_bytes: Some(42),
        };
        sender
            .emit(Event::new(
                ObjectId::new(3),
                NetworkEventPayload::RequestFinished(terminal),
            ))
            .expect("terminal snapshot should fit the server queue");
        let mut handler = network_event_handler(protocol(), 1);

        // Act
        outbound
            .send_once()
            .await
            .expect("terminal snapshot should send")
            .expect("one terminal snapshot should be queued");
        let frame = client_transport
            .receive()
            .await
            .expect("terminal frame should arrive")
            .expect("wire should remain open");
        handler.handle(frame).expect("terminal frame should decode");
        let received = handler.pop().expect("terminal snapshot should be retained");

        // Assert
        let NetworkEventPayload::RequestFinished(snapshot) = received.payload() else {
            panic!("terminal route should decode requestfinished");
        };
        assert_eq!(snapshot.request.url, "https://example.test/data");
        assert_eq!(snapshot.request.method, "GET");
        let response = snapshot
            .response
            .as_ref()
            .expect("terminal snapshot should include Response values");
        assert_eq!(response.status, 200);
        assert_eq!(response.status_text, "OK");
        assert_eq!(snapshot.encoded_body_bytes, Some(42));
    }

    #[tokio::test]
    async fn sustained_delivery_releases_every_client_lease_and_keeps_peak_at_one_slot() {
        // Arrange
        const ITERATIONS: usize = 4_096;
        const MAX_RETAINED_BYTES: usize = 512;
        let (server_transport, client_transport) = MemoryFrameTransport::pair(1);
        let (sender, outbound) = TypedEventOutbound::<NetworkEventStream, _, _>::new_routed(
            protocol(),
            server_transport,
            1,
            OverflowPolicy::RejectNewest,
            network_stream_event_tag,
        );
        let profile = TypedEventProductionProfile::new(1, MAX_RETAINED_BYTES);
        let resources = production_resources(profile);
        let mut handler = production_network_event_handler(protocol(), profile, &resources)
            .expect("bounded production handler should build");
        let request = small_request();

        // Act
        for _ in 0..ITERATIONS {
            assert!(matches!(
                sender.emit(Event::new(ObjectId::new(3), request.clone())),
                Ok(PushOutcome::Enqueued)
            ));
            outbound
                .send_once()
                .await
                .expect("event should send")
                .expect("event should be queued");
            let frame = client_transport
                .receive()
                .await
                .expect("receive should succeed")
                .expect("wire should stay open");
            assert!(matches!(
                handler.handle(frame).expect("event should decode"),
                FrameDisposition::Handled(PushOutcome::Enqueued)
            ));
            let received = handler.pop().expect("consumer should drain one event");
            assert!(matches!(
                received.payload(),
                NetworkEventPayload::Request(_)
            ));
        }
        let metrics = resources.metrics().expect("resource metrics should load");

        // Assert
        assert_eq!(handler.metrics().current_len, 0);
        assert_eq!(
            handler.sequence_metrics().last_received(),
            Some(EventSequence::new(ITERATIONS as u64))
        );
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 0);
        assert_eq!(metrics.class(ResourceClass::EventItem).peak, 1);
        assert_eq!(metrics.class(ResourceClass::EventBytes).current, 0);
        assert!(
            metrics.class(ResourceClass::EventBytes).peak
                <= u64::try_from(MAX_RETAINED_BYTES).expect("test limit should fit")
        );
        assert_eq!(metrics.active_leases, 0);
        assert_eq!(metrics.peak_leases, 1);
        assert_eq!(metrics.rejected_reservations, 0);
    }

    #[tokio::test]
    async fn full_client_queue_rejects_newest_without_evicting_the_causal_predecessor() {
        // Arrange
        let (server_transport, client_transport) = MemoryFrameTransport::pair(2);
        let (sender, outbound) = TypedEventOutbound::<NetworkEventStream, _, _>::new_routed(
            protocol(),
            server_transport,
            2,
            OverflowPolicy::RejectNewest,
            network_stream_event_tag,
        );
        let profile = TypedEventProductionProfile::new(1, 512);
        let resources = production_resources(profile);
        let mut handler = production_network_event_handler(protocol(), profile, &resources)
            .expect("bounded production handler should build");
        let values = [
            small_request(),
            NetworkEventPayload::RequestFinished(RequestFinished {
                request: request_initializer(),
                response: None,
                encoded_body_bytes: Some(42),
            }),
        ];
        for value in &values {
            sender
                .emit(Event::new(ObjectId::new(3), value.clone()))
                .expect("server queue should accept both events");
            outbound
                .send_once()
                .await
                .expect("event should send")
                .expect("event should exist");
        }
        let first = client_transport
            .receive()
            .await
            .expect("first receive should succeed")
            .expect("first frame should exist");
        let second = client_transport
            .receive()
            .await
            .expect("second receive should succeed")
            .expect("second frame should exist");
        handler.handle(first).expect("first event should decode");

        // Act
        let overflow = handler.handle(second).expect("overflow should be explicit");
        let retained = handler.pop().expect("oldest event must remain queued");
        let metrics = resources.metrics().expect("resource metrics should load");

        // Assert
        assert!(matches!(
            overflow,
            FrameDisposition::Handled(PushOutcome::Rejected { .. })
        ));
        assert!(matches!(
            retained.payload(),
            NetworkEventPayload::Request(_)
        ));
        assert_eq!(retained.sequence(), EventSequence::new(1));
        assert_eq!(
            handler.sequence_metrics().last_received(),
            Some(EventSequence::new(2))
        );
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 0);
        assert_eq!(metrics.class(ResourceClass::EventItem).peak, 1);
        assert_eq!(metrics.active_leases, 0);
    }
}
