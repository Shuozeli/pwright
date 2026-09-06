//! Optional, portable events emitted by Playwright-shaped browser objects.
//!
//! Installing this crate does not install a browser backend, codec, transport,
//! executor, or event queue. `PageLoad`, `BrowserContextPage`, and `PagePopup`
//! mirror Playwright's `page.on("load")`, `browser_context.on("page")`, and
//! `page.on("popup")` notifications and remain separate from commands.

use pwright_browser_model::{BROWSER_CONTEXT_OBJECT_KIND, PAGE_OBJECT_KIND};
use pwright_core::{
    Capability, CapabilityDescriptor, CapabilityEventDescriptor, CapabilityEventSpec, CapabilityId,
    EventId, EventSpec, EventTag, ObjectKindId, PluginInfo, ProtocolVersion,
};

pub const PAGE_EVENTS_CAPABILITY_ID: CapabilityId = CapabilityId::new("pwright.page-events");
pub const PAGE_LOAD_EVENT_ID: EventId = EventId::new("page.load");
pub const PAGE_LOAD_EVENT_TAG: EventTag = EventTag::new(65_537);
pub const BROWSER_CONTEXT_PAGE_EVENT_ID: EventId = EventId::new("browser_context.page");
pub const BROWSER_CONTEXT_PAGE_EVENT_TAG: EventTag = EventTag::new(65_538);
pub const PAGE_POPUP_EVENT_ID: EventId = EventId::new("page.popup");
pub const PAGE_POPUP_EVENT_TAG: EventTag = EventTag::new(65_539);
pub const PAGE_LOAD_TARGET_OBJECT_KINDS: &[ObjectKindId] = &[PAGE_OBJECT_KIND.id];
pub const BROWSER_CONTEXT_PAGE_TARGET_OBJECT_KINDS: &[ObjectKindId] =
    &[BROWSER_CONTEXT_OBJECT_KIND.id];
pub const PAGE_POPUP_TARGET_OBJECT_KINDS: &[ObjectKindId] = &[PAGE_OBJECT_KIND.id];
pub const PAGE_EVENT_IDS: &[EventId] = &[
    PAGE_LOAD_EVENT_ID,
    BROWSER_CONTEXT_PAGE_EVENT_ID,
    PAGE_POPUP_EVENT_ID,
];
pub const PAGE_EVENT_DESCRIPTORS: &[CapabilityEventDescriptor] = &[
    CapabilityEventDescriptor::new(
        PAGE_LOAD_EVENT_ID,
        PAGE_LOAD_EVENT_TAG,
        PAGE_LOAD_TARGET_OBJECT_KINDS,
    ),
    CapabilityEventDescriptor::new(
        BROWSER_CONTEXT_PAGE_EVENT_ID,
        BROWSER_CONTEXT_PAGE_EVENT_TAG,
        BROWSER_CONTEXT_PAGE_TARGET_OBJECT_KINDS,
    ),
    CapabilityEventDescriptor::new(
        PAGE_POPUP_EVENT_ID,
        PAGE_POPUP_EVENT_TAG,
        PAGE_POPUP_TARGET_OBJECT_KINDS,
    ),
];

pub struct PageEventsCapability;

impl Capability for PageEventsCapability {
    const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
        PluginInfo::new(
            PAGE_EVENTS_CAPABILITY_ID,
            "Page events",
            ProtocolVersion::new(1, 2),
        ),
        &[],
        &[],
    )
    .with_events(PAGE_EVENT_IDS);
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PageLoadEvent;

pub struct PageLoad;

impl EventSpec for PageLoad {
    const EVENT: EventTag = PAGE_LOAD_EVENT_TAG;
    type Payload = PageLoadEvent;
}

impl CapabilityEventSpec for PageLoad {
    type Capability = PageEventsCapability;
    const EVENT_ID: EventId = PAGE_LOAD_EVENT_ID;
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = PAGE_LOAD_TARGET_OBJECT_KINDS;
}

/// Payload delivered by Playwright-shaped `browser_context.on("page")`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrowserContextPageEvent {
    pub page: pwright_core::ObjectId,
}

pub struct BrowserContextPage;

impl EventSpec for BrowserContextPage {
    const EVENT: EventTag = BROWSER_CONTEXT_PAGE_EVENT_TAG;
    type Payload = BrowserContextPageEvent;
}

impl CapabilityEventSpec for BrowserContextPage {
    type Capability = PageEventsCapability;
    const EVENT_ID: EventId = BROWSER_CONTEXT_PAGE_EVENT_ID;
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = BROWSER_CONTEXT_PAGE_TARGET_OBJECT_KINDS;
}

/// Payload delivered by Playwright-shaped `page.on("popup")`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PagePopupEvent {
    pub page: pwright_core::ObjectId,
}

pub struct PagePopup;

impl EventSpec for PagePopup {
    const EVENT: EventTag = PAGE_POPUP_EVENT_TAG;
    type Payload = PagePopupEvent;
}

impl CapabilityEventSpec for PagePopup {
    type Capability = PageEventsCapability;
    const EVENT_ID: EventId = PAGE_POPUP_EVENT_ID;
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = PAGE_POPUP_TARGET_OBJECT_KINDS;
}

#[cfg(test)]
mod tests {
    use pwright_core::{Capability, CapabilityEventSpec, ProtocolVersion, RegistryBuilder};

    use super::{
        BROWSER_CONTEXT_PAGE_EVENT_ID, BROWSER_CONTEXT_PAGE_EVENT_TAG, BrowserContextPage,
        PAGE_EVENTS_CAPABILITY_ID, PAGE_LOAD_EVENT_ID, PAGE_LOAD_EVENT_TAG, PAGE_POPUP_EVENT_ID,
        PAGE_POPUP_EVENT_TAG, PageEventsCapability, PageLoad, PagePopup,
    };

    #[test]
    fn page_load_is_owned_by_the_optional_page_events_capability() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<PageEventsCapability>()
            .expect("page events should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert_eq!(
            registry.owner_of_event(PAGE_LOAD_EVENT_ID),
            Some(PAGE_EVENTS_CAPABILITY_ID)
        );
        assert_eq!(
            registry.owner_of_event(BROWSER_CONTEXT_PAGE_EVENT_ID),
            Some(PAGE_EVENTS_CAPABILITY_ID)
        );
        assert_eq!(
            registry.owner_of_event(PAGE_POPUP_EVENT_ID),
            Some(PAGE_EVENTS_CAPABILITY_ID)
        );
        assert_eq!(
            <PageLoad as pwright_core::EventSpec>::EVENT,
            PAGE_LOAD_EVENT_TAG
        );
        assert_eq!(PageLoad::TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            <BrowserContextPage as pwright_core::EventSpec>::EVENT,
            BROWSER_CONTEXT_PAGE_EVENT_TAG
        );
        assert_eq!(BrowserContextPage::TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            <PagePopup as pwright_core::EventSpec>::EVENT,
            PAGE_POPUP_EVENT_TAG
        );
        assert_eq!(PagePopup::TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            PageEventsCapability::DESCRIPTOR.plugin.version,
            ProtocolVersion::new(1, 2)
        );
    }
}
