use std::sync::Arc;
use std::time::Duration;

use pwright_backend_cdp::{
    CdpContextOwnership, CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpPageObjectIds,
    CdpPageTarget, CdpTopologyLimits, CdpTopologyProductionProfile, ConnectionEpoch,
};
use pwright_browser_model::{BROWSER_CONTEXT_OBJECT_KIND, FRAME_OBJECT_KIND, PAGE_OBJECT_KIND};
use pwright_cap_evaluation::{EvaluateParams, EvaluationValue, EvaluationValueLimits};
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpConnectionProductionProfile, CdpSession};
use pwright_codec_evaluation::EvaluationCodecLimits;
use pwright_composition_chromium::{
    CapabilityWireProfile, CdpPageRouteProductionProfile, CdpPageServices,
    ChromiumAttachProductionProfile, ChromiumTopologyEventDriverProductionProfile,
    EvaluationInstallationProfile, PreparedChromiumAttach,
};
use pwright_core::{
    ObjectId, ProtocolVersion, ResourceClass, ResourceLimits, ResourceRequirements, ResourceScope,
};
use pwright_provider_cdp::CdpWebSocketEndpoint;
use pwright_runtime::{RuntimeLimits, TargetedService};
use pwright_server_wire::ServerSupervisorLimits;
use pwright_transport::FrameLimits;

fn exact_limits(requirements: ResourceRequirements) -> ResourceLimits {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let capacity = requirements.class(class);
        limits = limits.with_class(class, capacity.global, capacity.per_scope);
    }
    limits
}

fn evaluation_profile() -> EvaluationInstallationProfile {
    let values = EvaluationValueLimits::new(8, 64, 64, 4096, 4096)
        .expect("live evaluation value limits should validate");
    EvaluationInstallationProfile::new(
        CdpEvaluationProductionProfile::new(
            ResourceScope::extension(96, 2),
            CdpEvaluationLimits::new(4096, 64 * 1024, 4096, values),
            1,
        ),
        EvaluationCodecLimits::new(4096, 4096, values)
            .expect("live evaluation codec limits should validate"),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(128 * 1024, 0, 0, 128 * 1024),
            FrameLimits::new(128 * 1024, 0, 0, 128 * 1024),
        ),
    )
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
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("temporary target {target_id} remained live after context disposal");
}

#[tokio::test]
#[ignore = "requires dedicated Chrome, internet, PWRIGHT_CDP_WS_URL, and PWRIGHT_OOPIF_FIXTURE_URL"]
async fn oopif_frame_evaluation_uses_automatic_topology_events() {
    run_oopif_evaluation(false).await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome, internet, PWRIGHT_CDP_WS_URL, and PWRIGHT_OOPIF_FIXTURE_URL"]
async fn recursive_attachment_join_preserves_oopif_world_routing() {
    run_oopif_evaluation(true).await;
}

async fn run_oopif_evaluation(recursive_attachment: bool) {
    // Arrange one production composition around an externally owned Chrome.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let fixture_url = std::env::var("PWRIGHT_OOPIF_FIXTURE_URL")
        .expect("PWRIGHT_OOPIF_FIXTURE_URL must be a served cross-site iframe fixture");
    let connection_profile =
        CdpConnectionProductionProfile::new(ResourceScope::Connection, ConnectionConfig::default());
    // Selecting utility worlds requires both main and utility routes for each
    // of the two Frames. Preserve the main-world-only fixture's original budget.
    let worlds = if recursive_attachment { 4 } else { 2 };
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(8, 1, 1, 2, worlds, 256),
        ResourceScope::extension(96, 1),
    )
    .expect("topology profile should validate");
    let page_routes = CdpPageRouteProductionProfile::from_topology(
        ResourceScope::extension(96, 3),
        topology_profile,
    );
    let topology_events = ChromiumTopologyEventDriverProductionProfile::from_profiles(
        ResourceScope::extension(96, 4),
        &connection_profile,
        topology_profile,
    )
    .expect("topology-event profile should validate");
    let topology_events = if recursive_attachment {
        topology_events
            .with_parallel_recursive_attachment()
            .expect("five-owner startup profile should validate")
    } else {
        topology_events
    };
    let profile = ChromiumAttachProductionProfile::new(
        connection_profile,
        topology_profile,
        page_routes,
        RuntimeLimits::new(8, 1),
        ServerSupervisorLimits::new(1).expect("supervisor limits should validate"),
    )
    .with_evaluation(evaluation_profile())
    .with_topology_events(topology_events);
    let profile = if recursive_attachment {
        profile.with_utility_world(
            pwright_composition_chromium::ChromiumUtilityWorldProfile::playwright_compatible(),
        )
    } else {
        profile
    };
    let requirements = profile.requirements();
    let prepared = PreparedChromiumAttach::new(profile, exact_limits(requirements))
        .expect("live production composition should prepare");
    let epoch = ConnectionEpoch::new(1);
    let mut composition = prepared
        .attach(endpoint, ObjectId::new(100), epoch)
        .await
        .expect("composition should attach without taking process ownership");
    let connection = Arc::clone(composition.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let browser_context_id = browser
        .target_create_browser_context(None, None)
        .await
        .expect("temporary browser context should open");
    let target_id = browser
        .target_create_in_context("about:blank", &browser_context_id)
        .await
        .expect("temporary target should open");
    let page_session_id = browser
        .target_attach(&target_id)
        .await
        .expect("temporary target should attach");
    let page = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        page_session_id.clone(),
        target_id.clone(),
    ));
    let main_frame_id = page
        .page_get_main_frame_id()
        .await
        .expect("main Frame should be discoverable");

    composition
        .with_topology_mut(|topology| {
            topology.register_context(
                epoch,
                browser_context_id.clone(),
                ObjectId::new(2),
                CdpContextOwnership::External,
            )
        })
        .expect("topology should be available")
        .expect("browser context should register");
    composition
        .with_topology_mut(|topology| {
            topology.attach_page(
                epoch,
                CdpPageTarget::new(
                    target_id.clone(),
                    page_session_id.clone(),
                    browser_context_id.clone(),
                ),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
        })
        .expect("topology should be available")
        .expect("Page should attach");
    composition
        .with_topology_mut(|topology| {
            topology.attach_frame(
                epoch,
                pwright_backend_cdp::CdpFrameAttachment::main(target_id.clone(), main_frame_id),
                ObjectId::new(4),
            )
        })
        .expect("topology should be available")
        .expect("main Frame should rebind");

    let runtime = composition
        .new_page_runtime()
        .expect("Page runtime should install");
    runtime
        .register_object(
            ObjectId::new(2),
            Some(ObjectId::new(100)),
            BROWSER_CONTEXT_OBJECT_KIND.tag,
        )
        .expect("BrowserContext should register in runtime");
    runtime
        .register_object(
            ObjectId::new(3),
            Some(ObjectId::new(2)),
            PAGE_OBJECT_KIND.tag,
        )
        .expect("Page should register in runtime");
    runtime
        .register_object(
            ObjectId::new(4),
            Some(ObjectId::new(3)),
            FRAME_OBJECT_KIND.tag,
        )
        .expect("main Frame should register in runtime");
    let mut event_driver = composition
        .topology_event_driver()
        .expect("topology-event driver should build")
        .expect("topology events should be selected");

    // Initialize the Page session before navigating. Runtime.enable emits the
    // existing main execution context, so no separate initialization task is needed.
    tokio::time::timeout(Duration::from_secs(5), event_driver.receive_once(&runtime))
        .await
        .expect("Runtime.enable should publish the existing main world")
        .expect("main-world event should apply");
    page.page_navigate(&fixture_url)
        .await
        .expect("served cross-site iframe fixture should navigate");

    // Act: caller-driven event ingestion discovers, initializes, and routes the OOPIF.
    let oopif_frame = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            event_driver
                .receive_once(&runtime)
                .await
                .expect("CDP topology event should apply");
            let route = composition
                .topology()
                .expect("topology should be available")
                .topology()
                .frame_execution_routes()
                .find(|route| {
                    route.object_id() != ObjectId::new(4) && route.session_id() != page_session_id
                })
                .map(|route| route.object_id());
            if let Some(frame) = route {
                break frame;
            }
        }
    })
    .await
    .expect("driver should publish an OOPIF main-world route");

    let result = <CdpPageServices as TargetedService<EvaluateParams>>::call_for(
        runtime.service(),
        oopif_frame,
        EvaluateParams::expression("location.hostname"),
    )
    .await
    .expect("OOPIF evaluation should use its execution world");

    // Assert the cross-site document answered and the external browser survives cleanup.
    assert!(
        matches!(result.value, EvaluationValue::String(ref host) if host.ends_with("example.com")),
        "unexpected OOPIF location: {:?}",
        result.value
    );
    dispose_context_and_verify_target(&browser, &browser_context_id, &target_id).await;
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("external browser should remain alive");
}
