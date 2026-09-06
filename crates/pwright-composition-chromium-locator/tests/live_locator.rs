use std::sync::Arc;

use pwright_browser_provider::AttachBrowserProvider;
use pwright_cap_locator::{CssSelector, v2 as locator_v2};
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpClient, CdpSession};
use pwright_composition_chromium_locator::{
    CdpInputCoordinator, CdpLocatorProductionProfile, ChromiumLocatorInstallation,
    ChromiumLocatorProfile, FrameLimits, LocatorCodecLimits,
};
use pwright_core::{
    CallId, CallMetadata, Command, ObjectId, ProductionResourceLedger, ResourceClass,
    ResourceLimits, ResourceScope, Service,
};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};

fn resources(profile: ChromiumLocatorProfile) -> ProductionResourceLedger {
    let requirements = profile.requirements();
    let selectors = requirements.class(ResourceClass::SelectorBytes);
    let wire = requirements.class(ResourceClass::WireBytes);
    let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases)
        .with_class(
            ResourceClass::SelectorBytes,
            selectors.global,
            selectors.per_scope,
        )
        .with_class(ResourceClass::WireBytes, wire.global, wire.per_scope);
    ProductionResourceLedger::new(limits, requirements)
        .expect("exact Locator composition resources should build")
}

async fn dispose_context_and_verify_target(
    browser: &CdpSession,
    browser_context_id: &str,
    target_id: &str,
) {
    browser
        .target_dispose_browser_context(browser_context_id)
        .await
        .expect("owned browser context should dispose");
    for _ in 0..20 {
        let targets = browser
            .target_get_targets()
            .await
            .expect("target list should remain available");
        if !targets.iter().any(|target| target.target_id == target_id) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("temporary target {target_id} remained live after context disposal");
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn locator_v2_composition_reaches_real_chrome_without_owning_it() {
    // Arrange
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
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    page.runtime_evaluate(
        "document.body.innerHTML = '<button id=go onclick=\"window.__clicked=true\">Go</button>'; true",
    )
    .await
    .expect("test button should be installed");
    let frames = FrameLimits::new(16 * 1024 * 1024, 0, 0, 16 * 1024 * 1024);
    let profile = ChromiumLocatorProfile::v2(
        CdpLocatorProductionProfile::new(ResourceScope::Object(ObjectId::new(3)), 1),
        LocatorCodecLimits::new(1024 * 1024),
        frames,
        frames,
    );
    let resources = resources(profile);
    let installation = ChromiumLocatorInstallation::install_main_target(
        ObjectId::new(3),
        Arc::clone(&page),
        CdpInputCoordinator::new(),
        profile,
        &resources,
    )
    .expect("Locator-only composition should install");
    let command = Command::<locator_v2::Click>::new(
        CallId::new(1),
        ObjectId::new(3),
        CallMetadata::default(),
        locator_v2::ClickParams::new(
            CssSelector::new("#go")
                .expect("static selector should be valid")
                .into(),
            locator_v2::ClickOptions::new(),
        ),
    );

    // Act
    installation
        .target()
        .call(command)
        .await
        .expect("command service is infallible")
        .into_outcome()
        .expect("v2 click should reach real Chrome");
    let clicked = page
        .runtime_evaluate("Boolean(window.__clicked)")
        .await
        .expect("click state should be readable");
    let _handlers = installation.handlers();
    drop(installation);
    dispose_context_and_verify_target(&browser, &browser_context_id, &target_id).await;

    // Assert
    assert_eq!(clicked["result"]["value"], true);
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("external browser should remain alive after composition drop");
}
