use std::sync::Arc;
use std::time::Duration;

use pwright_backend_cdp::{
    CdpContextOwnership, CdpFrameAttachment, CdpPageEventAdapter, CdpPageObjectIds, CdpPageTarget,
    CdpTopology, CdpTopologyLimits, ConnectionEpoch,
};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpSession, CdpSessionFactory};
use pwright_core::ObjectId;
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn page_load_adapter_reaches_real_chrome_without_owning_its_event_loop() {
    // Arrange one temporary target and its exact portable topology route.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .expect("provider should attach without taking process ownership");
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let browser_context_id = browser
        .target_create_browser_context(None, None)
        .await
        .expect("temporary browser context should open");
    let target_id = browser
        .target_create_in_context("about:blank", &browser_context_id)
        .await
        .expect("temporary target should open");
    let session_id = browser
        .target_attach(&target_id)
        .await
        .expect("temporary target should attach");
    let page = CdpSession::new(
        Arc::clone(&connection),
        session_id.clone(),
        target_id.clone(),
    );
    let frame_id = page
        .page_get_main_frame_id()
        .await
        .expect("temporary target should expose its main Frame");
    let epoch = ConnectionEpoch::new(1);
    let page_object = ObjectId::new(3);
    let mut topology = CdpTopology::new(
        ObjectId::new(1),
        epoch,
        CdpTopologyLimits::new(8, 2, 2, 4, 4, 256),
    )
    .expect("topology should validate");
    topology
        .register_context(
            epoch,
            &browser_context_id,
            ObjectId::new(2),
            CdpContextOwnership::Owned,
        )
        .expect("temporary context should map");
    topology
        .attach_page(
            epoch,
            CdpPageTarget::new(&target_id, &session_id, &browser_context_id),
            CdpPageObjectIds::new(page_object, ObjectId::new(4)),
        )
        .expect("temporary Page should map");
    topology
        .attach_frame(
            epoch,
            CdpFrameAttachment::main(&target_id, frame_id),
            ObjectId::new(4),
        )
        .expect("main Frame should map");
    let adapter =
        CdpPageEventAdapter::new(Arc::new(CdpSessionFactory::new(Arc::clone(&connection))));
    let mut events = connection.subscribe_events();
    adapter
        .set_page_load_subscription(&topology, page_object, true)
        .await
        .expect("Page load events should activate explicitly");

    // Act through a caller-owned event cursor.
    page.page_navigate("data:text/html,<title>page-event-leaf</title>")
        .await
        .expect("data URL navigation should start");
    let mapped = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = events
                .recv()
                .await
                .expect("CDP event stream should remain live");
            let Some(mapped) = adapter
                .map_page_load_event(&topology, &event)
                .expect("real lifecycle event should normalize")
            else {
                continue;
            };
            let title = page
                .runtime_evaluate("document.title")
                .await
                .expect("loaded document title should be readable");
            if title["result"]["value"] == "page-event-leaf" {
                break mapped;
            }
        }
    })
    .await
    .expect("current main-Frame load should arrive");

    // Assert the portable source is stable and external Chrome remains live.
    assert_eq!(mapped.object_id(), page_object);
    adapter
        .set_page_load_subscription(&topology, page_object, false)
        .await
        .expect("Page load delivery should deactivate explicitly");
    browser
        .target_dispose_browser_context(&browser_context_id)
        .await
        .expect("temporary context should clean up");
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome must remain alive");
}
