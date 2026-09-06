//! Isolated Chromium CDP adapter for Playwright-shaped Page events.
//!
//! The leaf activates and normalizes Page-local lifecycle events and maps
//! command-free topology effects. It owns no queue, reader task, runtime,
//! transport, browser process, or unrelated capability.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use pwright_cap_page_events::{
    BrowserContextPage, BrowserContextPageEvent, PageLoad, PageLoadEvent, PagePopup, PagePopupEvent,
};
use pwright_cdp::connection::CdpError;
use pwright_cdp::{CdpEvent, SessionFactory};
use pwright_core::{Event, ObjectId};

use pwright_backend_cdp_topology::{CdpTopology, CdpTopologyEffect};

/// Explicit CDP activation and normalization for Playwright-shaped Page events.
///
/// The adapter owns no queue and starts no event reader. The composition root
/// decides when to call `set_page_load_subscription` and where to enqueue the
/// normalized event returned by `map_page_load_event`.
#[derive(Clone)]
pub struct CdpPageEventAdapter {
    session_factory: Arc<dyn SessionFactory>,
}

impl CdpPageEventAdapter {
    pub fn new(session_factory: Arc<dyn SessionFactory>) -> Self {
        Self { session_factory }
    }

    /// Activates only the CDP lifecycle stream needed by `PageLoad`.
    ///
    /// Enabling first makes the Page domain available and then enables
    /// lifecycle delivery. Disabling leaves the shared Page domain active so
    /// an independently installed navigation capability is never disrupted.
    pub async fn set_page_load_subscription(
        &self,
        topology: &CdpTopology,
        object_id: ObjectId,
        enabled: bool,
    ) -> Result<(), CdpPageEventError> {
        let route = topology
            .page_session_routes()
            .find(|route| route.object_id() == object_id)
            .ok_or(CdpPageEventError::PageRouteUnavailable { object_id })?;
        let client = self
            .session_factory
            .create_session(route.session_id().to_owned(), route.target_id().to_owned());
        if enabled {
            client.page_enable().await.map_err(CdpPageEventError::Cdp)?;
        }
        client
            .page_set_lifecycle_events_enabled(enabled)
            .await
            .map_err(CdpPageEventError::Cdp)
    }

    /// Converts a current main-frame CDP `load` lifecycle event into the
    /// payload-free Playwright Page event. Unrelated, child-frame, and stale
    /// session events are ignored.
    pub fn map_page_load_event(
        &self,
        topology: &CdpTopology,
        event: &CdpEvent,
    ) -> Result<Option<Event<PageLoad>>, CdpPageEventError> {
        if event.method != "Page.lifecycleEvent" {
            return Ok(None);
        }
        let name = event
            .params
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or(CdpPageEventError::InvalidLifecycleEvent { field: "name" })?;
        if name != "load" {
            return Ok(None);
        }
        let frame_id = event
            .params
            .get("frameId")
            .and_then(|value| value.as_str())
            .ok_or(CdpPageEventError::InvalidLifecycleEvent { field: "frameId" })?;
        let Some(session_id) = event.session_id.as_deref() else {
            return Err(CdpPageEventError::InvalidLifecycleEvent { field: "sessionId" });
        };
        let Some(route) = topology
            .page_session_routes()
            .find(|route| route.session_id() == session_id)
        else {
            return Ok(None);
        };
        if topology.main_frame_native_id(route.object_id()) != Some(frame_id) {
            return Ok(None);
        }
        Ok(Some(Event::new(route.object_id(), PageLoadEvent)))
    }

    /// Converts a topology `PageReady` effect into the portable
    /// `browser_context.on("page")` contract. Other effects are ignored.
    pub fn map_browser_context_page_effect(
        &self,
        effect: &CdpTopologyEffect,
    ) -> Option<Event<BrowserContextPage>> {
        map_browser_context_page_effect(effect)
    }

    /// Converts a topology `PopupLinked` effect into the portable
    /// `page.on("popup")` contract. The event source is the opener and the
    /// payload identifies the newly opened Page.
    pub fn map_page_popup_effect(&self, effect: &CdpTopologyEffect) -> Option<Event<PagePopup>> {
        map_page_popup_effect(effect)
    }
}

/// Pure backend normalization for a topology `PageReady` effect.
pub fn map_browser_context_page_effect(
    effect: &CdpTopologyEffect,
) -> Option<Event<BrowserContextPage>> {
    let CdpTopologyEffect::PageReady { page, context } = effect else {
        return None;
    };
    Some(Event::new(
        *context,
        BrowserContextPageEvent { page: *page },
    ))
}

/// Pure backend normalization for a topology `PopupLinked` effect.
pub fn map_page_popup_effect(effect: &CdpTopologyEffect) -> Option<Event<PagePopup>> {
    let CdpTopologyEffect::PopupLinked { opener, popup } = effect else {
        return None;
    };
    Some(Event::new(*opener, PagePopupEvent { page: *popup }))
}

#[derive(Debug)]
pub enum CdpPageEventError {
    PageRouteUnavailable { object_id: ObjectId },
    InvalidLifecycleEvent { field: &'static str },
    Cdp(CdpError),
}

impl Display for CdpPageEventError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::PageRouteUnavailable { object_id } => {
                write!(formatter, "page `{object_id}` has no active CDP session")
            }
            Self::InvalidLifecycleEvent { field } => {
                write!(formatter, "Page.lifecycleEvent is missing `{field}`")
            }
            Self::Cdp(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for CdpPageEventError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Cdp(error) => Some(error),
            Self::PageRouteUnavailable { .. } | Self::InvalidLifecycleEvent { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cdp::{CdpClient, CdpEvent, SessionFactory};
    use pwright_core::ObjectId;
    use pwright_fake::FakeCdpClient;

    use pwright_backend_cdp_topology::{
        CdpContextOwnership, CdpFrameAttachment, CdpPageObjectIds, CdpPageTarget, CdpTopology,
        CdpTopologyEffect, CdpTopologyLimits, ConnectionEpoch,
    };

    use super::CdpPageEventAdapter;

    struct SharedFakeFactory {
        client: Arc<FakeCdpClient>,
    }

    impl SessionFactory for SharedFakeFactory {
        fn create_session(&self, _session_id: String, _target_id: String) -> Arc<dyn CdpClient> {
            self.client.clone()
        }
    }

    fn topology() -> CdpTopology {
        let epoch = ConnectionEpoch::new(1);
        let mut topology = CdpTopology::new(
            ObjectId::new(1),
            epoch,
            CdpTopologyLimits::new(32, 4, 4, 8, 8, 64),
        )
        .expect("topology should fit");
        topology
            .register_context(
                epoch,
                "context-1",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("context should register");
        topology
            .attach_page(
                epoch,
                CdpPageTarget::new("target-1", "session-1", "context-1"),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
            .expect("page should attach");
        topology
            .attach_frame(
                epoch,
                CdpFrameAttachment::main("target-1", "frame-1"),
                ObjectId::new(4),
            )
            .expect("main-frame native ID should synchronize");
        topology
    }

    #[tokio::test]
    async fn enabling_page_load_activates_only_the_required_cdp_lifecycle_stream() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<main></main>"));
        let adapter = CdpPageEventAdapter::new(Arc::new(SharedFakeFactory {
            client: client.clone(),
        }));
        let topology = topology();

        // Act
        adapter
            .set_page_load_subscription(&topology, ObjectId::new(3), true)
            .await
            .expect("subscription should activate");

        // Assert
        assert_eq!(
            client.calls(),
            vec![
                ("Page.enable".to_owned(), vec![]),
                (
                    "Page.setLifecycleEventsEnabled".to_owned(),
                    vec![serde_json::Value::Bool(true)],
                ),
            ]
        );
    }

    #[test]
    fn main_frame_load_maps_to_the_stable_page_object() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<main></main>"));
        let adapter = CdpPageEventAdapter::new(Arc::new(SharedFakeFactory { client }));
        let topology = topology();
        let native = CdpEvent::new(
            "Page.lifecycleEvent".to_owned(),
            serde_json::json!({
                "frameId": "frame-1",
                "loaderId": "loader-1",
                "name": "load",
                "timestamp": 1.0,
            }),
            Some("session-1".to_owned()),
        );

        // Act
        let mapped = adapter
            .map_page_load_event(&topology, &native)
            .expect("valid event should map")
            .expect("main-frame load should emit");

        // Assert
        assert_eq!(mapped.object_id(), ObjectId::new(3));
    }

    #[test]
    fn child_frame_load_does_not_emit_a_page_load() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<main></main>"));
        let adapter = CdpPageEventAdapter::new(Arc::new(SharedFakeFactory { client }));
        let topology = topology();
        let native = CdpEvent::new(
            "Page.lifecycleEvent".to_owned(),
            serde_json::json!({"frameId": "child-frame", "name": "load"}),
            Some("session-1".to_owned()),
        );

        // Act
        let mapped = adapter
            .map_page_load_event(&topology, &native)
            .expect("valid event should map");

        // Assert
        assert!(mapped.is_none());
    }

    #[test]
    fn topology_discovery_effects_map_to_context_page_and_opener_popup_events() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<main></main>"));
        let adapter = CdpPageEventAdapter::new(Arc::new(SharedFakeFactory { client }));
        let ready = CdpTopologyEffect::PageReady {
            page: ObjectId::new(7),
            context: ObjectId::new(2),
        };
        let linked = CdpTopologyEffect::PopupLinked {
            opener: ObjectId::new(7),
            popup: ObjectId::new(8),
        };

        // Act
        let context_page = adapter
            .map_browser_context_page_effect(&ready)
            .expect("PageReady should map");
        let popup = adapter
            .map_page_popup_effect(&linked)
            .expect("PopupLinked should map");

        // Assert
        assert_eq!(context_page.object_id(), ObjectId::new(2));
        assert_eq!(context_page.payload().page, ObjectId::new(7));
        assert_eq!(popup.object_id(), ObjectId::new(7));
        assert_eq!(popup.payload().page, ObjectId::new(8));
        assert!(adapter.map_page_popup_effect(&ready).is_none());
        assert!(adapter.map_browser_context_page_effect(&linked).is_none());
    }
}
