//! Optional portable events emitted by Playwright-shaped Browser objects.
//!
//! This crate is contract-only. It installs no backend, codec, transport,
//! executor, listener, or queue.

use pwright_browser_model::BROWSER_OBJECT_KIND;
use pwright_core::{
    Capability, CapabilityDescriptor, CapabilityEventDescriptor, CapabilityEventSpec, CapabilityId,
    EventId, EventSpec, EventTag, ObjectKindId, PluginInfo, ProtocolVersion,
};

pub const BROWSER_EVENTS_CAPABILITY_ID: CapabilityId = CapabilityId::new("pwright.browser-events");
pub const BROWSER_DISCONNECTED_EVENT_ID: EventId = EventId::new("browser.disconnected");
pub const BROWSER_DISCONNECTED_EVENT_TAG: EventTag = EventTag::new(65_540);
pub const BROWSER_DISCONNECTED_TARGET_OBJECT_KINDS: &[ObjectKindId] = &[BROWSER_OBJECT_KIND.id];
pub const BROWSER_EVENT_IDS: &[EventId] = &[BROWSER_DISCONNECTED_EVENT_ID];
pub const BROWSER_EVENT_DESCRIPTORS: &[CapabilityEventDescriptor] =
    &[CapabilityEventDescriptor::new(
        BROWSER_DISCONNECTED_EVENT_ID,
        BROWSER_DISCONNECTED_EVENT_TAG,
        BROWSER_DISCONNECTED_TARGET_OBJECT_KINDS,
    )];

pub struct BrowserEventsCapability;

impl Capability for BrowserEventsCapability {
    const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
        PluginInfo::new(
            BROWSER_EVENTS_CAPABILITY_ID,
            "Browser events",
            ProtocolVersion::new(1, 0),
        ),
        &[],
        &[],
    )
    .with_events(BROWSER_EVENT_IDS);
}

/// Playwright's public `browser.on("disconnected")` callback has no payload.
/// The Browser object itself is identified by the event route.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BrowserDisconnectedEvent;

pub struct BrowserDisconnected;

impl EventSpec for BrowserDisconnected {
    const EVENT: EventTag = BROWSER_DISCONNECTED_EVENT_TAG;
    type Payload = BrowserDisconnectedEvent;
}

impl CapabilityEventSpec for BrowserDisconnected {
    type Capability = BrowserEventsCapability;
    const EVENT_ID: EventId = BROWSER_DISCONNECTED_EVENT_ID;
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = BROWSER_DISCONNECTED_TARGET_OBJECT_KINDS;
}

#[cfg(test)]
mod tests {
    use pwright_core::{Capability, CapabilityEventSpec, ProtocolVersion, RegistryBuilder};

    use super::{
        BROWSER_DISCONNECTED_EVENT_ID, BROWSER_DISCONNECTED_EVENT_TAG,
        BROWSER_EVENTS_CAPABILITY_ID, BrowserDisconnected, BrowserEventsCapability,
    };

    #[test]
    fn disconnected_is_owned_by_the_optional_browser_events_capability() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<BrowserEventsCapability>()
            .expect("Browser events should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert_eq!(
            registry.owner_of_event(BROWSER_DISCONNECTED_EVENT_ID),
            Some(BROWSER_EVENTS_CAPABILITY_ID)
        );
        assert_eq!(
            <BrowserDisconnected as pwright_core::EventSpec>::EVENT,
            BROWSER_DISCONNECTED_EVENT_TAG
        );
        assert_eq!(BrowserDisconnected::TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            BrowserEventsCapability::DESCRIPTOR.plugin.version,
            ProtocolVersion::new(1, 0)
        );
    }
}
