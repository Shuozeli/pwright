//! Opt-in client installation for Playwright-shaped Page lifecycle and discovery events.

use pwright_cap_page_events::{
    BrowserContextPage, BrowserContextPageEvent, PageLoad, PageLoadEvent, PagePopup, PagePopupEvent,
};
use pwright_client::{BrowserContext, CallIdSource, Page};
use pwright_client_events::{
    SubscriptionError, SubscriptionTracker, SubscriptionTransition, TypedEventHandler,
};
use pwright_codec_page_events::PageEventsCodec;
use pwright_core::{Event, OverflowPolicy};
use pwright_transport::WireProtocol;

/// Page load is lifecycle state, so overflow rejects the newest notification
/// instead of silently turning the queue into a telemetry window.
pub fn page_load_handler(
    protocol: WireProtocol<PageEventsCodec>,
    capacity: usize,
) -> TypedEventHandler<PageLoad, PageEventsCodec> {
    TypedEventHandler::new(protocol, capacity, OverflowPolicy::RejectNewest)
}

pub fn browser_context_page_handler(
    protocol: WireProtocol<PageEventsCodec>,
    capacity: usize,
) -> TypedEventHandler<BrowserContextPage, PageEventsCodec> {
    TypedEventHandler::new(protocol, capacity, OverflowPolicy::RejectNewest)
}

pub fn page_popup_handler(
    protocol: WireProtocol<PageEventsCodec>,
    capacity: usize,
) -> TypedEventHandler<PagePopup, PageEventsCodec> {
    TypedEventHandler::new(protocol, capacity, OverflowPolicy::RejectNewest)
}

pub const fn retained_page_load_bytes(_event: &Event<PageLoad>) -> usize {
    std::mem::size_of::<PageLoadEvent>()
}

pub const fn retained_browser_context_page_bytes(_event: &Event<BrowserContextPage>) -> usize {
    std::mem::size_of::<BrowserContextPageEvent>()
}

pub const fn retained_page_popup_bytes(_event: &Event<PagePopup>) -> usize {
    std::mem::size_of::<PagePopupEvent>()
}

pub trait PageLoadSubscriptionExt {
    fn subscribe_page_load(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn unsubscribe_page_load(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;
}

impl<Channel, Ids> PageLoadSubscriptionExt for Page<Channel, Ids>
where
    Ids: CallIdSource,
{
    fn subscribe_page_load(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.subscribe(self.id(), <PageLoad as pwright_core::EventSpec>::EVENT)
    }

    fn unsubscribe_page_load(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.unsubscribe(self.id(), <PageLoad as pwright_core::EventSpec>::EVENT)
    }
}

pub trait BrowserContextPageSubscriptionExt {
    fn subscribe_page(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn unsubscribe_page(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;
}

impl<Channel, Ids> BrowserContextPageSubscriptionExt for BrowserContext<Channel, Ids>
where
    Ids: CallIdSource,
{
    fn subscribe_page(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.subscribe(
            self.id(),
            <BrowserContextPage as pwright_core::EventSpec>::EVENT,
        )
    }

    fn unsubscribe_page(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.unsubscribe(
            self.id(),
            <BrowserContextPage as pwright_core::EventSpec>::EVENT,
        )
    }
}

pub trait PagePopupSubscriptionExt {
    fn subscribe_popup(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;

    fn unsubscribe_popup(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError>;
}

impl<Channel, Ids> PagePopupSubscriptionExt for Page<Channel, Ids>
where
    Ids: CallIdSource,
{
    fn subscribe_popup(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.subscribe(self.id(), <PagePopup as pwright_core::EventSpec>::EVENT)
    }

    fn unsubscribe_popup(
        &self,
        subscriptions: &mut SubscriptionTracker,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        subscriptions.unsubscribe(self.id(), <PagePopup as pwright_core::EventSpec>::EVENT)
    }
}

#[cfg(test)]
mod tests {
    use pwright_browser_model::{
        BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, PAGE_OBJECT_KIND,
    };
    use pwright_cap_page_events::{
        BROWSER_CONTEXT_PAGE_EVENT_TAG, PAGE_LOAD_EVENT_TAG, PAGE_POPUP_EVENT_TAG,
    };
    use pwright_client::{AtomicCallIdSource, Client, RemoteInitializer};
    use pwright_client_events::{SubscriptionTracker, SubscriptionTransition};
    use pwright_codec_page_events::PageEventsCodec;
    use pwright_core::{ControlMessage, ObjectId, OverflowPolicy, ProtocolVersion};
    use pwright_transport::{FrameLimits, WireProtocol};

    use super::{
        BrowserContextPageSubscriptionExt, PageLoadSubscriptionExt, PagePopupSubscriptionExt,
        browser_context_page_handler, page_load_handler, page_popup_handler,
    };

    #[test]
    fn page_load_installation_uses_reliable_overflow_and_page_identity() {
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
                ObjectId::new(7),
                Some(ObjectId::new(2)),
                PAGE_OBJECT_KIND.tag,
            ))
            .expect("page should initialize");
        let protocol = WireProtocol::new_exact_directional(
            PageEventsCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
            FrameLimits::new(64, 0, 0, 64),
        );
        let mut subscriptions = SubscriptionTracker::new(1);

        // Act
        let transition = page
            .subscribe_page_load(&mut subscriptions)
            .expect("subscription should fit");

        // Assert
        let handler = page_load_handler(protocol, 4);
        assert_eq!(handler.overflow_policy(), OverflowPolicy::RejectNewest);
        assert_eq!(
            transition,
            SubscriptionTransition::Send(ControlMessage::SetSubscription {
                object_id: ObjectId::new(7),
                event: PAGE_LOAD_EVENT_TAG,
                enabled: true,
            })
        );
    }

    #[test]
    fn discovery_subscriptions_preserve_context_and_opener_identity() {
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
                ObjectId::new(7),
                Some(ObjectId::new(2)),
                PAGE_OBJECT_KIND.tag,
            ))
            .expect("page should initialize");
        let protocol = WireProtocol::new_exact_directional(
            PageEventsCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
            FrameLimits::new(64, 0, 0, 64),
        );
        let popup_protocol = WireProtocol::new_exact_directional(
            PageEventsCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
            FrameLimits::new(64, 0, 0, 64),
        );
        let mut subscriptions = SubscriptionTracker::new(2);

        // Act
        let context_transition = context
            .subscribe_page(&mut subscriptions)
            .expect("context subscription should fit");
        let popup_transition = page
            .subscribe_popup(&mut subscriptions)
            .expect("popup subscription should fit");

        // Assert
        assert_eq!(
            context_transition,
            SubscriptionTransition::Send(ControlMessage::SetSubscription {
                object_id: ObjectId::new(2),
                event: BROWSER_CONTEXT_PAGE_EVENT_TAG,
                enabled: true,
            })
        );
        assert_eq!(
            popup_transition,
            SubscriptionTransition::Send(ControlMessage::SetSubscription {
                object_id: ObjectId::new(7),
                event: PAGE_POPUP_EVENT_TAG,
                enabled: true,
            })
        );
        assert_eq!(
            browser_context_page_handler(protocol, 2).overflow_policy(),
            OverflowPolicy::RejectNewest
        );
        assert_eq!(
            page_popup_handler(popup_protocol, 2).overflow_policy(),
            OverflowPolicy::RejectNewest
        );
    }
}
