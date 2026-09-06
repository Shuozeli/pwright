use std::sync::Arc;
use std::time::Duration;

use pwright_backend_cdp::{
    CdpFrameClipTranslator, CdpLocatorProductionProfile, CdpLocatorScreenshotResolver,
    CdpLocatorScreenshotService, CdpLocatorServiceFactory, CdpScreenshotProductionProfile,
    CdpScreenshotServiceFactory,
};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cap_content::{ScreenshotFormat, ScreenshotOptions, ScreenshotScale};
use pwright_cap_locator_screenshot::{LocatorScreenshotParams, Selector};
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpClient, CdpSession};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope, Service,
};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn full_page_png_capture_reaches_real_chromium_and_releases_its_lease() {
    // Arrange an isolated temporary Context in the externally owned browser.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .expect("provider should attach without browser ownership");
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let browser_context_id = browser
        .target_create_browser_context(None, None)
        .await
        .expect("temporary Context should open");
    let target_id = browser
        .target_create_in_context(
            "data:text/html,<style>html,body{margin:0}body{width:320px;height:900px;background:rgb(12,34,56)}</style><main>Pwright</main>",
            &browser_context_id,
        )
        .await
        .expect("temporary Page should open");
    let session_id = browser
        .target_attach(&target_id)
        .await
        .expect("temporary Page should attach");
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    let mut document_ready = false;
    for _ in 0..40 {
        let ready = page
            .runtime_evaluate("document.readyState")
            .await
            .expect("fixture readiness should be observable");
        if ready["result"]["value"] == "complete" {
            document_ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(document_ready, "fixture document should finish loading");
    let profile = CdpScreenshotProductionProfile::new(1024 * 1024, 1);
    let requirements = profile.requirements();
    let capacity = requirements.class(ResourceClass::ScreenshotBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
            ResourceClass::ScreenshotBytes,
            capacity.global,
            capacity.per_scope,
        ),
        requirements,
    )
    .expect("screenshot resource plan should be exact");
    let ledger = resources.claim(requirements).unwrap();
    let service = CdpScreenshotServiceFactory::new_with_production_resources(profile, &resources)
        .unwrap()
        .bind(page, ObjectId::new(3));

    // Act through the production adapter and retain the portable result.
    let screenshot = service
        .call(ScreenshotOptions {
            format: Some(ScreenshotFormat::Png),
            full_page: Some(true),
            scale: Some(ScreenshotScale::Css),
            ..ScreenshotOptions::default()
        })
        .await
        .expect("real Chromium should capture a bounded full-page PNG");
    let signature: [u8; 8] = screenshot.as_bytes()[..8].try_into().unwrap();
    let width = u32::from_be_bytes(screenshot.as_bytes()[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(screenshot.as_bytes()[20..24].try_into().unwrap());
    let retained = ledger
        .metrics()
        .unwrap()
        .class(ResourceClass::ScreenshotBytes)
        .current;
    drop(screenshot);
    let released = ledger
        .metrics()
        .unwrap()
        .class(ResourceClass::ScreenshotBytes)
        .current;
    browser
        .target_dispose_browser_context(&browser_context_id)
        .await
        .expect("temporary Context should dispose");

    // Assert real PNG structure, full-page height, accounting, and ownership.
    assert_eq!(signature, *b"\x89PNG\r\n\x1a\n");
    assert!(width >= 320);
    assert!(height >= 900);
    assert!(retained > 24);
    assert_eq!(released, 0);
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome should remain alive");
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn locator_png_capture_proves_visible_scroll_geometry_cleanup_and_browser_ownership() {
    // Arrange an isolated Context and two exactly sized elements.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .expect("provider should attach without browser ownership");
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let browser_context_id = browser
        .target_create_browser_context(None, None)
        .await
        .expect("temporary Context should open");
    let target_id = browser
        .target_create_in_context("about:blank", &browser_context_id)
        .await
        .expect("temporary Page should open");
    let session_id = browser
        .target_attach(&target_id)
        .await
        .expect("temporary Page should attach");
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    page.runtime_evaluate(
        r#"(() => {
          document.documentElement.style.margin = '0';
          document.body.style.margin = '0';
          document.body.style.height = '1800px';
          document.body.innerHTML = `
            <div id="visible" style="position:absolute;left:10px;top:20px;width:120px;height:80px;background:#123456"></div>
            <div id="below" style="position:absolute;left:15px;top:1400px;width:90px;height:60px;background:#654321"></div>`;
          return true;
        })()"#,
    )
    .await
    .expect("fixture should install");
    let locator_profile = CdpLocatorProductionProfile::new(ResourceScope::extension(9, 2), 1);
    let screenshot_profile = CdpScreenshotProductionProfile::new(1024 * 1024, 1);
    let requirements = locator_profile
        .requirements()
        .combine_concurrent(screenshot_profile.requirements());
    let selector_capacity = requirements.class(ResourceClass::SelectorBytes);
    let screenshot_capacity = requirements.class(ResourceClass::ScreenshotBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(requirements.scopes, requirements.leases)
            .with_class(
                ResourceClass::SelectorBytes,
                selector_capacity.global,
                selector_capacity.per_scope,
            )
            .with_class(
                ResourceClass::ScreenshotBytes,
                screenshot_capacity.global,
                screenshot_capacity.per_scope,
            ),
        requirements,
    )
    .expect("combined Locator screenshot resource plan should be exact");
    let ledger = resources.claim(requirements).unwrap();
    let locator =
        CdpLocatorServiceFactory::new_with_production_resources(locator_profile, &resources)
            .unwrap()
            .bind(Arc::clone(&page));
    let screenshot =
        CdpScreenshotServiceFactory::new_with_production_resources(screenshot_profile, &resources)
            .unwrap()
            .bind(Arc::clone(&page), ObjectId::new(3));
    let service = CdpLocatorScreenshotService::new(
        Arc::new(CdpLocatorScreenshotResolver::new(locator)),
        screenshot,
    );
    let options = ScreenshotOptions {
        format: Some(ScreenshotFormat::Png),
        scale: Some(ScreenshotScale::Css),
        ..ScreenshotOptions::default()
    };

    // Act: capture an already-visible element, then a below-fold element.
    let visible = service
        .call(LocatorScreenshotParams::new(
            Selector::css("#visible").expect("selector should validate"),
            options,
        ))
        .await
        .expect("visible Locator should capture");
    let visible_dimensions = png_dimensions(visible.as_bytes());
    drop(visible);
    let below = service
        .call(LocatorScreenshotParams::new(
            Selector::css("#below").expect("selector should validate"),
            options,
        ))
        .await
        .expect("below-fold Locator should scroll and capture");
    let below_dimensions = png_dimensions(below.as_bytes());
    let scroll_y = page
        .runtime_evaluate("scrollY")
        .await
        .expect("scroll position should remain observable")["result"]["value"]
        .as_f64()
        .expect("scroll position should be numeric");
    drop(below);
    let released = ledger
        .metrics()
        .unwrap()
        .class(ResourceClass::ScreenshotBytes)
        .current;
    browser
        .target_dispose_browser_context(&browser_context_id)
        .await
        .expect("temporary Context should dispose");
    let mut target_removed = false;
    for _ in 0..20 {
        let targets = browser
            .target_get_targets()
            .await
            .expect("target list should remain available");
        if !targets.iter().any(|target| target.target_id == target_id) {
            target_removed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Assert exact CSS pixel geometry, scrolling, resource cleanup, and no
    // ownership of the externally managed Chrome process.
    assert_eq!(visible_dimensions, (120, 80));
    assert_eq!(below_dimensions, (90, 60));
    assert!(scroll_y > 0.0, "below-fold capture should scroll the Page");
    assert_eq!(released, 0);
    assert!(target_removed, "temporary target should leave the browser");
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome should remain alive");
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn child_frame_locator_png_uses_native_owner_translation() {
    // Arrange an isolated same-process iframe and observe its exact execution world.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .expect("provider should attach without browser ownership");
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let browser_context_id = browser
        .target_create_browser_context(None, None)
        .await
        .expect("temporary Context should open");
    let target_id = browser
        .target_create_in_context("about:blank", &browser_context_id)
        .await
        .expect("temporary Page should open");
    let session_id = browser
        .target_attach(&target_id)
        .await
        .expect("temporary Page should attach");
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id.clone(),
        target_id.clone(),
    ));
    let main_frame_id = page
        .page_get_main_frame_id()
        .await
        .expect("main Frame should be discoverable");
    let mut events = page.subscribe_events();
    page.page_enable().await.expect("Page domain should enable");
    page.runtime_enable()
        .await
        .expect("Runtime domain should enable");
    page.page_set_document_content(
        &main_frame_id,
        r#"<!doctype html>
        <style>html,body{margin:0;background:rgb(200,0,0)}iframe{position:absolute;left:100px;top:200px;width:300px;height:200px;border:0}</style>
        <iframe srcdoc="<style>html,body{margin:0;background:rgb(0,0,200)}</style><div id='inside' style='position:absolute;left:20px;top:30px;width:70px;height:50px;background:rgb(0,200,0)'></div>"></iframe>"#,
    )
    .await
    .expect("iframe fixture should install");
    let (child_frame_id, child_context_id) = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = events
                .recv()
                .await
                .expect("CDP event stream should remain live");
            if event.method != "Runtime.executionContextCreated"
                || event.session_id.as_deref() != Some(session_id.as_str())
            {
                continue;
            }
            let context = &event.params["context"];
            let frame_id = context["auxData"]["frameId"].as_str();
            let unique_id = context["uniqueId"].as_str();
            let is_default = context["auxData"]["isDefault"].as_bool();
            if let (Some(frame_id), Some(unique_id), Some(true)) = (frame_id, unique_id, is_default)
                && frame_id != main_frame_id
            {
                break (frame_id.to_owned(), unique_id.to_owned());
            }
        }
    })
    .await
    .expect("child Frame execution context should appear");
    let locator_profile = CdpLocatorProductionProfile::new(ResourceScope::extension(9, 3), 1);
    let screenshot_profile = CdpScreenshotProductionProfile::new(1024 * 1024, 1);
    let requirements = locator_profile
        .requirements()
        .combine_concurrent(screenshot_profile.requirements());
    let selector_capacity = requirements.class(ResourceClass::SelectorBytes);
    let screenshot_capacity = requirements.class(ResourceClass::ScreenshotBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(requirements.scopes, requirements.leases)
            .with_class(
                ResourceClass::SelectorBytes,
                selector_capacity.global,
                selector_capacity.per_scope,
            )
            .with_class(
                ResourceClass::ScreenshotBytes,
                screenshot_capacity.global,
                screenshot_capacity.per_scope,
            ),
        requirements,
    )
    .expect("child Locator screenshot resource plan should be exact");
    let ledger = resources.claim(requirements).unwrap();
    let locator =
        CdpLocatorServiceFactory::new_with_production_resources(locator_profile, &resources)
            .unwrap()
            .bind_unique_context(Arc::clone(&page), child_context_id);
    let screenshot =
        CdpScreenshotServiceFactory::new_with_production_resources(screenshot_profile, &resources)
            .unwrap()
            .bind(Arc::clone(&page), ObjectId::new(3));
    let translator = CdpFrameClipTranslator::child(
        &CdpFrameClipTranslator::default(),
        Arc::clone(&page),
        child_frame_id,
    );
    let service = CdpLocatorScreenshotService::new(
        Arc::new(CdpLocatorScreenshotResolver::new(locator)),
        screenshot,
    )
    .with_clip_translator(translator);

    // Act through the child execution world and top-level Page capture.
    let result = service
        .call(LocatorScreenshotParams::new(
            Selector::css("#inside").expect("selector should validate"),
            ScreenshotOptions {
                format: Some(ScreenshotFormat::Png),
                scale: Some(ScreenshotScale::Css),
                ..ScreenshotOptions::default()
            },
        ))
        .await
        .expect("real Chromium should translate and capture a child Locator");
    let dimensions = png_dimensions(result.as_bytes());
    drop(result);
    let released = ledger
        .metrics()
        .unwrap()
        .class(ResourceClass::ScreenshotBytes)
        .current;
    browser
        .target_dispose_browser_context(&browser_context_id)
        .await
        .expect("temporary Context should dispose");

    // Assert exact child element dimensions, cleanup, and external ownership.
    assert_eq!(dimensions, (70, 50));
    assert_eq!(released, 0);
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome should remain alive");
}

fn png_dimensions(bytes: &[u8]) -> (u32, u32) {
    assert!(bytes.len() >= 24, "PNG must contain an IHDR chunk");
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    (
        u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
        u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
    )
}
