//! Optional portable HTTP lifecycle events emitted by Page and BrowserContext.
//!
//! Request and Response identities are live only while an exchange is active.
//! Terminal events carry complete immutable value snapshots, so this base
//! capability does not retain Playwright-style handles after completion.
//!
//! This contract installs no response-body reader, request routing, WebSocket
//! observer, HAR recorder, backend, codec, transport, runtime, task, or queue.

use pwright_browser_model::{BROWSER_CONTEXT_OBJECT_KIND, PAGE_OBJECT_KIND};
use pwright_core::{
    Capability, CapabilityDescriptor, CapabilityEventDescriptor, CapabilityEventSpec, CapabilityId,
    Event, EventId, EventSpec, EventTag, ObjectKindId, PluginInfo, ProtocolVersion,
};
pub use pwright_network_model::{
    HttpHeader, NetworkLifecycleLimits, RequestFailed, RequestFinished, RequestInitializer,
    RequestTiming, ResponseInitializer,
};

pub const NETWORK_EVENTS_CAPABILITY_ID: CapabilityId = CapabilityId::new("pwright.network-events");
pub const NETWORK_REQUEST_EVENT_ID: EventId = EventId::new("network.request");
pub const NETWORK_RESPONSE_EVENT_ID: EventId = EventId::new("network.response");
pub const NETWORK_REQUEST_FINISHED_EVENT_ID: EventId = EventId::new("network.request-finished");
pub const NETWORK_REQUEST_FAILED_EVENT_ID: EventId = EventId::new("network.request-failed");
pub const NETWORK_REQUEST_EVENT_TAG: EventTag = EventTag::new(65_541);
pub const NETWORK_RESPONSE_EVENT_TAG: EventTag = EventTag::new(65_542);
pub const NETWORK_REQUEST_FINISHED_EVENT_TAG: EventTag = EventTag::new(65_543);
pub const NETWORK_REQUEST_FAILED_EVENT_TAG: EventTag = EventTag::new(65_544);
pub const NETWORK_EVENT_TARGET_OBJECT_KINDS: &[ObjectKindId] =
    &[BROWSER_CONTEXT_OBJECT_KIND.id, PAGE_OBJECT_KIND.id];
pub const NETWORK_EVENT_IDS: &[EventId] = &[
    NETWORK_REQUEST_EVENT_ID,
    NETWORK_RESPONSE_EVENT_ID,
    NETWORK_REQUEST_FINISHED_EVENT_ID,
    NETWORK_REQUEST_FAILED_EVENT_ID,
];
pub const NETWORK_EVENT_DESCRIPTORS: &[CapabilityEventDescriptor] = &[
    CapabilityEventDescriptor::new(
        NETWORK_REQUEST_EVENT_ID,
        NETWORK_REQUEST_EVENT_TAG,
        NETWORK_EVENT_TARGET_OBJECT_KINDS,
    ),
    CapabilityEventDescriptor::new(
        NETWORK_RESPONSE_EVENT_ID,
        NETWORK_RESPONSE_EVENT_TAG,
        NETWORK_EVENT_TARGET_OBJECT_KINDS,
    ),
    CapabilityEventDescriptor::new(
        NETWORK_REQUEST_FINISHED_EVENT_ID,
        NETWORK_REQUEST_FINISHED_EVENT_TAG,
        NETWORK_EVENT_TARGET_OBJECT_KINDS,
    ),
    CapabilityEventDescriptor::new(
        NETWORK_REQUEST_FAILED_EVENT_ID,
        NETWORK_REQUEST_FAILED_EVENT_TAG,
        NETWORK_EVENT_TARGET_OBJECT_KINDS,
    ),
];

pub struct NetworkEventsCapability;

impl Capability for NetworkEventsCapability {
    const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
        PluginInfo::new(
            NETWORK_EVENTS_CAPABILITY_ID,
            "HTTP network lifecycle events",
            ProtocolVersion::new(1, 0),
        ),
        &[],
        &[],
    )
    .with_events(NETWORK_EVENT_IDS);
}

pub struct NetworkRequest;

impl EventSpec for NetworkRequest {
    const EVENT: EventTag = NETWORK_REQUEST_EVENT_TAG;
    type Payload = RequestInitializer;
}

impl CapabilityEventSpec for NetworkRequest {
    type Capability = NetworkEventsCapability;
    const EVENT_ID: EventId = NETWORK_REQUEST_EVENT_ID;
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = NETWORK_EVENT_TARGET_OBJECT_KINDS;
}

pub struct NetworkResponse;

impl EventSpec for NetworkResponse {
    const EVENT: EventTag = NETWORK_RESPONSE_EVENT_TAG;
    type Payload = ResponseInitializer;
}

impl CapabilityEventSpec for NetworkResponse {
    type Capability = NetworkEventsCapability;
    const EVENT_ID: EventId = NETWORK_RESPONSE_EVENT_ID;
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = NETWORK_EVENT_TARGET_OBJECT_KINDS;
}

pub struct NetworkRequestFinished;

impl EventSpec for NetworkRequestFinished {
    const EVENT: EventTag = NETWORK_REQUEST_FINISHED_EVENT_TAG;
    type Payload = RequestFinished;
}

impl CapabilityEventSpec for NetworkRequestFinished {
    type Capability = NetworkEventsCapability;
    const EVENT_ID: EventId = NETWORK_REQUEST_FINISHED_EVENT_ID;
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = NETWORK_EVENT_TARGET_OBJECT_KINDS;
}

pub struct NetworkRequestFailed;

impl EventSpec for NetworkRequestFailed {
    const EVENT: EventTag = NETWORK_REQUEST_FAILED_EVENT_TAG;
    type Payload = RequestFailed;
}

impl CapabilityEventSpec for NetworkRequestFailed {
    type Capability = NetworkEventsCapability;
    const EVENT_ID: EventId = NETWORK_REQUEST_FAILED_EVENT_ID;
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = NETWORK_EVENT_TARGET_OBJECT_KINDS;
}

/// Transport aggregation for the four causally ordered public HTTP events.
///
/// This is not another public capability event. It lets one queue assign one
/// sequence across Request, Response, Finished, and Failed while the outer
/// frame still carries each variant's public event tag.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkEventPayload {
    Request(RequestInitializer),
    Response(ResponseInitializer),
    RequestFinished(RequestFinished),
    RequestFailed(RequestFailed),
}

impl NetworkEventPayload {
    pub const fn event_tag(&self) -> EventTag {
        match self {
            Self::Request(_) => NETWORK_REQUEST_EVENT_TAG,
            Self::Response(_) => NETWORK_RESPONSE_EVENT_TAG,
            Self::RequestFinished(_) => NETWORK_REQUEST_FINISHED_EVENT_TAG,
            Self::RequestFailed(_) => NETWORK_REQUEST_FAILED_EVENT_TAG,
        }
    }

    pub fn retained_bytes(&self) -> usize {
        match self {
            Self::Request(payload) => payload.retained_bytes(),
            Self::Response(payload) => payload.retained_bytes(),
            Self::RequestFinished(_) => 0,
            Self::RequestFailed(payload) => payload.retained_bytes(),
        }
    }
}

pub struct NetworkEventStream;

impl EventSpec for NetworkEventStream {
    // Routed event plumbing uses `NetworkEventPayload::event_tag`; this value
    // remains a valid member of the family for generic type metadata.
    const EVENT: EventTag = NETWORK_REQUEST_EVENT_TAG;
    type Payload = NetworkEventPayload;
}

pub fn network_stream_event_tag(event: &Event<NetworkEventStream>) -> EventTag {
    event.payload().event_tag()
}

pub const fn is_network_event_tag(event: EventTag) -> bool {
    matches!(
        event,
        NETWORK_REQUEST_EVENT_TAG
            | NETWORK_RESPONSE_EVENT_TAG
            | NETWORK_REQUEST_FINISHED_EVENT_TAG
            | NETWORK_REQUEST_FAILED_EVENT_TAG
    )
}

#[cfg(test)]
mod tests {
    use pwright_core::{Capability, CapabilityEventSpec, ProtocolVersion, RegistryBuilder};

    use super::{
        NETWORK_EVENT_DESCRIPTORS, NETWORK_EVENTS_CAPABILITY_ID, NETWORK_REQUEST_EVENT_ID,
        NETWORK_REQUEST_EVENT_TAG, NETWORK_REQUEST_FAILED_EVENT_ID,
        NETWORK_REQUEST_FINISHED_EVENT_ID, NETWORK_RESPONSE_EVENT_ID, NetworkEventsCapability,
        NetworkRequest,
    };

    #[test]
    fn four_http_events_are_owned_by_one_optional_metadata_capability() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<NetworkEventsCapability>()
            .expect("network events should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        for event in [
            NETWORK_REQUEST_EVENT_ID,
            NETWORK_RESPONSE_EVENT_ID,
            NETWORK_REQUEST_FINISHED_EVENT_ID,
            NETWORK_REQUEST_FAILED_EVENT_ID,
        ] {
            assert_eq!(
                registry.owner_of_event(event),
                Some(NETWORK_EVENTS_CAPABILITY_ID)
            );
        }
        assert_eq!(NETWORK_EVENT_DESCRIPTORS.len(), 4);
        assert_eq!(
            <NetworkRequest as pwright_core::EventSpec>::EVENT,
            NETWORK_REQUEST_EVENT_TAG
        );
        assert_eq!(
            NetworkEventsCapability::DESCRIPTOR.plugin.version,
            ProtocolVersion::new(1, 0)
        );
    }

    #[test]
    fn the_same_event_contract_can_be_subscribed_on_context_or_page() {
        // Arrange
        let targets = NetworkRequest::TARGET_OBJECT_KINDS;

        // Act
        let target_names = targets.iter().map(|kind| kind.as_str()).collect::<Vec<_>>();

        // Assert
        assert_eq!(
            target_names,
            vec!["pwright.browser.browser_context", "pwright.browser.page"]
        );
    }
}
