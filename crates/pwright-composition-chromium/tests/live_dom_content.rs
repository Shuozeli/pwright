use std::sync::Arc;
use std::time::Duration;

use pwright_backend_cdp::{
    CdpDomContentProductionProfile, CdpTopologyLimits, CdpTopologyProductionProfile,
    ConnectionEpoch,
};
use pwright_cap_dom_content::{DomContentErrorDetails, DomContentParams};
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpConnectionProductionProfile, CdpSession};
use pwright_codec_dom_content::DomContentCodecLimits;
use pwright_composition_chromium::{
    CapabilityWireProfile, CdpPageRouteProductionProfile, ChromiumAttachProductionProfile,
    ChromiumPageDiscoveryProductionProfile, ChromiumTopologyEventDriverProductionProfile,
    ChromiumUtilityWorldProfile, DomContentInstallationProfile, PreparedChromiumAttach,
};
use pwright_core::{
    FailureKind, ObjectId, ProtocolVersion, ResourceClass, ResourceLimits, ResourceRequirements,
    ResourceScope,
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

async fn close_target_and_verify(browser: &CdpSession, target_id: &str) {
    browser
        .target_close(target_id)
        .await
        .expect("temporary target should accept close");
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
    panic!("temporary DOM-content target remained live after close");
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn frame_content_matches_chrome_doctype_and_child_dom_without_owning_browser() {
    // Arrange an opt-in DOM-content composition around externally owned Chrome.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let connection =
        CdpConnectionProductionProfile::new(ResourceScope::Connection, ConnectionConfig::default());
    let topology = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(64, 4, 16, 32, 64, 512),
        ResourceScope::extension(99, 1),
    )
    .expect("topology profile should validate");
    let page_routes =
        CdpPageRouteProductionProfile::from_topology(ResourceScope::extension(99, 2), topology);
    let topology_events = ChromiumTopologyEventDriverProductionProfile::from_profiles(
        ResourceScope::extension(99, 3),
        &connection,
        topology,
    )
    .expect("topology-event profile should validate");
    let discovery = ChromiumPageDiscoveryProductionProfile::from_profiles(&connection, topology);
    let max_content_bytes = 1024 * 1024;
    let wire = CapabilityWireProfile::new(
        ProtocolVersion::new(1, 0),
        FrameLimits::new(
            16 * 1024,
            1,
            max_content_bytes,
            max_content_bytes + 16 * 1024,
        ),
        FrameLimits::new(
            16 * 1024,
            1,
            max_content_bytes,
            max_content_bytes + 16 * 1024,
        ),
    );
    let profile = ChromiumAttachProductionProfile::new(
        connection,
        topology,
        page_routes,
        RuntimeLimits::new(64, 2),
        ServerSupervisorLimits::new(1).expect("supervisor limits should validate"),
    )
    .with_dom_content(DomContentInstallationProfile::new(
        CdpDomContentProductionProfile::new(ResourceScope::extension(99, 4), max_content_bytes, 2),
        DomContentCodecLimits::new(max_content_bytes, 1024),
        wire,
    ))
    .with_topology_events(topology_events)
    .with_page_discovery(discovery)
    .with_utility_world(ChromiumUtilityWorldProfile::playwright_compatible());
    let requirements = profile.requirements();
    let prepared = PreparedChromiumAttach::new(profile, exact_limits(requirements))
        .expect("DOM-content composition should prepare");
    let mut composition = prepared
        .attach(endpoint, ObjectId::new(2_000), ConnectionEpoch::new(1))
        .await
        .expect("composition should attach without owning Chrome");
    let browser = CdpSession::browser(Arc::clone(composition.connection()));
    let target_id = browser
        .target_create("about:blank")
        .await
        .expect("temporary target should open");
    let runtime = composition
        .new_page_runtime()
        .expect("Page runtime should install");
    let mut driver = composition
        .topology_event_driver()
        .expect("topology-event driver should build")
        .expect("topology events should be selected");

    let (session_id, main_frame, initial_utility_context) =
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                driver
                    .receive_once(&runtime)
                    .await
                    .expect("browser topology event should apply");
                let topology = composition
                    .topology()
                    .expect("topology should be available");
                let Some(page) = topology
                    .topology()
                    .page_session_routes()
                    .find(|route| route.target_id() == target_id)
                else {
                    continue;
                };
                let Some(main_frame) = topology.topology().main_frame_object_id(&target_id) else {
                    continue;
                };
                let Some(world) = topology
                    .topology()
                    .frame_utility_execution_routes()
                    .find(|route| route.object_id() == main_frame)
                else {
                    continue;
                };
                break (
                    page.session_id().to_owned(),
                    main_frame,
                    world.unique_context_id().to_owned(),
                );
            }
        })
        .await
        .expect("main Frame utility world should be observed");
    let page = CdpSession::new(
        Arc::clone(composition.connection()),
        session_id,
        target_id.clone(),
    );
    page.runtime_evaluate(
        r#"document.open(); document.write('<!DOCTYPE html><html><head><title>Pwright</title></head><body><iframe srcdoc="<p>child</p>"></iframe><iframe src="javascript:console.log(1)"></iframe></body></html>'); document.close(); true"#,
    )
    .await
    .expect("test document should install");
    let child_frames = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            driver
                .receive_once(&runtime)
                .await
                .expect("child topology events should apply");
            let topology = composition
                .topology()
                .expect("topology should be available");
            let mut children = topology
                .topology()
                .frame_utility_execution_routes()
                .filter(|route| route.target_id() == target_id && route.object_id() != main_frame)
                .map(|route| route.object_id())
                .collect::<Vec<_>>();
            children.sort_unstable();
            children.dedup();
            if children.len() == 2 {
                break children;
            }
        }
    })
    .await
    .expect("both child Frame utility worlds should be observed");

    // Act through the portable Frame routes.
    let main = runtime
        .service()
        .call_for(main_frame, DomContentParams)
        .await
        .expect("main Frame content should serialize");
    let mut child_results = Vec::new();
    for child_frame in child_frames {
        child_results.push(
            runtime
                .service()
                .call_for(child_frame, DomContentParams)
                .await
                .expect("child Frame content should serialize independently"),
        );
    }

    // Assert HTML5 doctype ordering and both non-empty and source-less child
    // Frame normalization.
    assert!(main.as_str().starts_with("<!DOCTYPE html><html>"));
    assert!(main.as_str().contains("<title>Pwright</title>"));
    assert!(main.as_str().contains("<iframe"));
    let child_texts = child_results
        .iter()
        .map(|content| content.as_str())
        .collect::<Vec<_>>();
    assert!(child_texts.contains(&"<html><head></head><body><p>child</p></body></html>"));
    assert!(child_texts.contains(&"<html><head></head><body></body></html>"));
    drop((main, child_results));

    // Act and assert the remaining upstream doctype cases on the same Frame.
    page.runtime_evaluate(
        r#"document.open(); document.write('<div>hello</div>'); document.close(); true"#,
    )
    .await
    .expect("doctype-free document should install");
    let without_doctype = runtime
        .service()
        .call_for(main_frame, DomContentParams)
        .await
        .expect("doctype-free content should serialize");
    assert_eq!(
        without_doctype.as_str(),
        "<html><head></head><body><div>hello</div></body></html>"
    );
    page.runtime_evaluate(
        r#"document.open(); document.write('<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01//EN" "http://www.w3.org/TR/html4/strict.dtd"><div>hello</div>'); document.close(); true"#,
    )
    .await
    .expect("HTML4 document should install");
    let html4 = runtime
        .service()
        .call_for(main_frame, DomContentParams)
        .await
        .expect("HTML4 content should serialize");
    assert_eq!(
        html4.as_str(),
        "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01//EN\" \"http://www.w3.org/TR/html4/strict.dtd\"><html><head></head><body><div>hello</div></body></html>"
    );
    drop((without_doctype, html4));

    // A committed cross-document navigation destroys the old utility context.
    // Before the topology driver publishes its replacement, content must fail
    // once with the stable Playwright-shaped race category and never retry.
    page.page_navigate("data:text/html,<div id=new>new</div>")
        .await
        .expect("cross-document navigation should commit");
    let navigating = runtime
        .service()
        .call_for(main_frame, DomContentParams)
        .await
        .expect_err("the stale utility context should not be retried implicitly");
    assert_eq!(navigating.kind(), FailureKind::Backend);
    assert!(
        matches!(
            navigating.details(),
            Some(DomContentErrorDetails::ContentChangedDuringNavigation(_))
        ),
        "unexpected navigation race error: {navigating:?}"
    );
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            driver
                .receive_once(&runtime)
                .await
                .expect("navigation topology event should apply");
            let topology = composition
                .topology()
                .expect("topology should remain available");
            let Some(world) = topology
                .topology()
                .frame_utility_execution_routes()
                .find(|route| route.object_id() == main_frame)
            else {
                continue;
            };
            if world.unique_context_id() != initial_utility_context {
                break;
            }
        }
    })
    .await
    .expect("replacement utility context should be published");
    let after_navigation = runtime
        .service()
        .call_for(main_frame, DomContentParams)
        .await
        .expect("the stable Frame identity should use the replacement context");
    assert_eq!(
        after_navigation.as_str(),
        "<html><head></head><body><div id=\"new\">new</div></body></html>"
    );
    drop(after_navigation);

    // Exact target cleanup must not terminate the externally owned browser.
    close_target_and_verify(&browser, &target_id).await;
    browser
        .target_get_targets()
        .await
        .expect("external Chrome should remain reachable");
}
