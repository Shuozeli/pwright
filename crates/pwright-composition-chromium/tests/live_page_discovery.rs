use std::sync::Arc;
use std::time::Duration;

use pwright_backend_cdp::{
    CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpLocatorProductionProfile,
    CdpTopologyEffect, CdpTopologyLimits, CdpTopologyProductionProfile, ConnectionEpoch,
};
use pwright_cap_evaluation::EvaluationValueLimits;
use pwright_cap_locator::{ClickOptions, ClickParams, Selector};
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpConnectionProductionProfile, CdpSession};
use pwright_codec_evaluation::EvaluationCodecLimits;
use pwright_codec_locator::LocatorCodecLimits;
use pwright_composition_chromium::{
    CapabilityWireProfile, CdpPageRouteProductionProfile, ChromiumAttachProductionProfile,
    ChromiumPageDiscoveryProductionProfile, ChromiumTopologyEventDriverError,
    ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverProductionProfile,
    ChromiumUtilityWorldProfile, EvaluationInstallationProfile, LocatorInstallationProfile,
    PreparedChromiumAttach,
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
        .expect("evaluation value limits should validate");
    EvaluationInstallationProfile::new(
        CdpEvaluationProductionProfile::new(
            ResourceScope::extension(97, 2),
            CdpEvaluationLimits::new(4096, 64 * 1024, 4096, values),
            1,
        ),
        EvaluationCodecLimits::new(4096, 4096, values)
            .expect("evaluation codec limits should validate"),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(128 * 1024, 0, 0, 128 * 1024),
            FrameLimits::new(128 * 1024, 0, 0, 128 * 1024),
        ),
    )
}

fn locator_profile() -> LocatorInstallationProfile {
    LocatorInstallationProfile::new(
        CdpLocatorProductionProfile::new(ResourceScope::extension(97, 3), 1),
        LocatorCodecLimits::new(128 * 1024),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(128 * 1024, 0, 0, 128 * 1024),
            FrameLimits::new(128 * 1024, 0, 0, 128 * 1024),
        ),
    )
}

async fn close_targets_and_verify(browser: &CdpSession, target_ids: &[String]) {
    for target_id in target_ids {
        browser
            .target_close(target_id)
            .await
            .expect("temporary target should accept close");
    }
    for _ in 0..20 {
        let targets = browser
            .target_get_targets()
            .await
            .expect("target list should remain available");
        if target_ids
            .iter()
            .all(|target_id| !targets.iter().any(|target| &target.target_id == target_id))
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("temporary discovery targets remained live after close");
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn utility_world_is_created_per_page_and_isolated_from_user_evaluation() {
    // Arrange an explicit utility-world composition around external Chrome.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let connection_profile =
        CdpConnectionProductionProfile::new(ResourceScope::Connection, ConnectionConfig::default());
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(64, 4, 16, 32, 64, 512),
        ResourceScope::extension(98, 1),
    )
    .expect("topology profile should validate");
    let page_routes = CdpPageRouteProductionProfile::from_topology(
        ResourceScope::extension(98, 2),
        topology_profile,
    );
    let topology_events = ChromiumTopologyEventDriverProductionProfile::from_profiles(
        ResourceScope::extension(98, 3),
        &connection_profile,
        topology_profile,
    )
    .expect("topology-event profile should validate");
    let discovery = ChromiumPageDiscoveryProductionProfile::from_profiles(
        &connection_profile,
        topology_profile,
    );
    let profile = ChromiumAttachProductionProfile::new(
        connection_profile,
        topology_profile,
        page_routes,
        RuntimeLimits::new(64, 2),
        ServerSupervisorLimits::new(1).expect("supervisor limits should validate"),
    )
    .with_evaluation(evaluation_profile())
    .with_locator(locator_profile())
    .with_topology_events(topology_events)
    .with_page_discovery(discovery)
    .with_utility_world(ChromiumUtilityWorldProfile::playwright_compatible());
    let requirements = profile.requirements();
    let prepared = PreparedChromiumAttach::new(profile, exact_limits(requirements))
        .expect("utility-world composition should prepare");
    let mut composition = prepared
        .attach(endpoint, ObjectId::new(1_000), ConnectionEpoch::new(1))
        .await
        .expect("composition should attach without owning Chrome");
    let connection = Arc::clone(composition.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
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

    // Act: drive only until the exact Page's utility execution context exists.
    let (session_id, unique_context_id) = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            driver
                .receive_once(&runtime)
                .await
                .expect("browser topology event should apply");
            let topology = composition
                .topology()
                .expect("topology should remain available");
            let Some(page) = topology
                .topology()
                .page_session_routes()
                .find(|route| route.target_id() == target_id)
            else {
                continue;
            };
            let Some(main_frame) = topology.topology().main_frame_object_id(page.target_id())
            else {
                continue;
            };
            let Some(world) = topology
                .topology()
                .frame_utility_execution_routes()
                .find(|world| world.object_id() == main_frame)
            else {
                continue;
            };
            break (
                page.session_id().to_owned(),
                world.unique_context_id().to_owned(),
            );
        }
    })
    .await
    .expect("utility world should be observed");
    let page = CdpSession::new(Arc::clone(&connection), session_id, target_id.clone());
    let utility_result = page
        .runtime_evaluate_in_unique_context(
            "globalThis.__pwright_utility_probe = 41; globalThis.__pwright_utility_probe",
            &unique_context_id,
        )
        .await
        .expect("utility-world evaluation should succeed");
    let main_result = page
        .runtime_evaluate("typeof globalThis.__pwright_utility_probe")
        .await
        .expect("main-world evaluation should succeed");
    page.runtime_evaluate(
        r#"document.body.innerHTML = '<iframe srcdoc="<button id=inside>Inside</button>"></iframe>'; true"#,
    )
    .await
    .expect("local child Frame should be created");
    let child_frame = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            driver
                .receive_once(&runtime)
                .await
                .expect("child Frame topology event should apply");
            let topology = composition
                .topology()
                .expect("topology should remain available");
            let main_frame = topology
                .topology()
                .main_frame_object_id(&target_id)
                .expect("temporary Page should retain its main Frame");
            let Some(route) = topology
                .topology()
                .frame_utility_execution_routes()
                .find(|route| route.target_id() == target_id && route.object_id() != main_frame)
            else {
                continue;
            };
            break route.object_id();
        }
    })
    .await
    .expect("child Frame utility world should be observed");

    // Act through the portable Frame route. The main document intentionally
    // has no matching button, so success cannot come from Page fallback.
    tokio::time::timeout(
        Duration::from_secs(10),
        runtime.service().call_for(
            child_frame,
            ClickParams::new(
                Selector::css("#inside").expect("selector should validate"),
                ClickOptions::new().with_trial(true),
            ),
        ),
    )
    .await
    .expect("child Frame locator should not retry against the main document")
    .expect("child Frame locator should resolve in its own utility world");

    // Assert the isolated global is absent from the user's main world and the
    // external Chrome process survives target cleanup.
    assert_eq!(utility_result["result"]["value"], 41);
    assert_eq!(main_result["result"]["value"], "undefined");
    close_targets_and_verify(&browser, std::slice::from_ref(&target_id)).await;
    browser
        .target_get_targets()
        .await
        .expect("external Chrome should remain reachable");
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn existing_default_page_and_popup_are_discovered_without_manual_topology_setup() {
    // Arrange a composition which explicitly selects browser-root discovery.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let connection_profile =
        CdpConnectionProductionProfile::new(ResourceScope::Connection, ConnectionConfig::default());
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(128, 8, 32, 64, 128, 512),
        ResourceScope::extension(97, 1),
    )
    .expect("topology profile should validate");
    let page_routes = CdpPageRouteProductionProfile::from_topology(
        ResourceScope::extension(97, 3),
        topology_profile,
    );
    let topology_events = ChromiumTopologyEventDriverProductionProfile::from_profiles(
        ResourceScope::extension(97, 4),
        &connection_profile,
        topology_profile,
    )
    .expect("topology-event profile should validate");
    let discovery = ChromiumPageDiscoveryProductionProfile::from_profiles(
        &connection_profile,
        topology_profile,
    );
    let profile = ChromiumAttachProductionProfile::new(
        connection_profile,
        topology_profile,
        page_routes,
        RuntimeLimits::new(128, 2),
        ServerSupervisorLimits::new(1).expect("supervisor limits should validate"),
    )
    .with_evaluation(evaluation_profile())
    .with_topology_events(topology_events)
    .with_page_discovery(discovery);
    let requirements = profile.requirements();
    let prepared = PreparedChromiumAttach::new(profile, exact_limits(requirements))
        .expect("live production composition should prepare");
    let mut composition = prepared
        .attach(endpoint, ObjectId::new(100), ConnectionEpoch::new(1))
        .await
        .expect("composition should attach without taking process ownership");
    let connection = Arc::clone(composition.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let opener_target_id = browser
        .target_create("about:blank")
        .await
        .expect("default-context target should open");
    let runtime = composition
        .new_page_runtime()
        .expect("Page runtime should install");
    let mut driver = composition
        .topology_event_driver()
        .expect("topology-event driver should build")
        .expect("topology events should be selected");

    // Act: discover the already-existing target, then open and discover a popup.
    let (opener_object_id, opener_session_id) =
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                driver
                    .receive_once(&runtime)
                    .await
                    .expect("browser topology event should apply");
                let route = composition
                    .topology()
                    .expect("topology should remain available")
                    .topology()
                    .page_session_routes()
                    .find(|route| route.target_id() == opener_target_id)
                    .map(|route| (route.object_id(), route.session_id().to_owned()));
                if let Some(route) = route {
                    break route;
                }
            }
        })
        .await
        .expect("existing target should be discovered");
    let opener = CdpSession::new(
        Arc::clone(&connection),
        opener_session_id.clone(),
        opener_target_id.clone(),
    );
    let popup_evaluation = opener.send(
        "Runtime.evaluate",
        serde_json::json!({
            "expression": "window.open('about:blank', '_blank'); true",
            "userGesture": true,
            "awaitPromise": true,
            "returnByValue": true
        }),
    );
    let popup_discovery = async {
        loop {
            let outcome = driver
                .receive_once(&runtime)
                .await
                .expect("popup topology event should apply");
            let popup = match outcome {
                ChromiumTopologyEventDriverOutcome::Updated(update) => {
                    update.effects().iter().find_map(|effect| match effect {
                        CdpTopologyEffect::PopupLinked { opener, popup }
                            if *opener == opener_object_id =>
                        {
                            Some(*popup)
                        }
                        _ => None,
                    })
                }
                ChromiumTopologyEventDriverOutcome::Ignored => None,
            };
            if let Some(popup) = popup {
                let target_id = composition
                    .topology()
                    .expect("topology should remain available")
                    .topology()
                    .page_session_routes()
                    .find(|route| route.object_id() == popup)
                    .map(|route| route.target_id().to_owned())
                    .expect("linked popup should retain a Page route");
                break (popup, target_id);
            }
        }
    };
    let (popup_result, (popup_object_id, popup_target_id)) =
        tokio::time::timeout(Duration::from_secs(20), async {
            tokio::join!(popup_evaluation, popup_discovery)
        })
        .await
        .expect("popup command and discovery should complete together");
    popup_result.expect("opener should create a popup");

    // Assert discovery created the default Context and both initialized Pages.
    let popup_session_id = {
        let topology = composition
            .topology()
            .expect("topology should remain available");
        let default_context = topology
            .topology()
            .default_context_route()
            .expect("default context should be represented");
        assert_eq!(default_context.browser_context_id(), "");
        assert_ne!(popup_object_id, ObjectId::new(100));
        assert!(
            topology
                .topology()
                .page_object_id(&opener_target_id)
                .is_some()
        );
        topology
            .topology()
            .page_session_routes()
            .find(|route| route.object_id() == popup_object_id)
            .map(|route| route.session_id().to_owned())
            .expect("popup should retain its initial session")
    };

    // Force bounded event loss, then reconstruct from Chromium's authoritative
    // re-attachment snapshot without changing either portable Page identity.
    opener
        .send(
            "Runtime.evaluate",
            serde_json::json!({
                "expression": "for (let i = 0; i < 1100; ++i) console.log(i); true",
                "returnByValue": true
            }),
        )
        .await
        .expect("console flood should complete");
    let loss = tokio::time::timeout(Duration::from_secs(20), driver.receive_once(&runtime))
        .await
        .expect("event loss should be observed")
        .expect_err("bounded broadcast overwrite must require reconstruction");
    assert!(matches!(
        loss,
        ChromiumTopologyEventDriverError::ResyncRequired { skipped } if skipped > 0
    ));
    let reconstructed = tokio::time::timeout(
        Duration::from_secs(20),
        driver.resynchronize_after_event_loss(&runtime, ConnectionEpoch::new(2)),
    )
    .await
    .expect("authoritative reconstruction should not stall")
    .expect("authoritative reconstruction should succeed");
    let ChromiumTopologyEventDriverOutcome::Updated(reconstructed) = reconstructed else {
        panic!("reconstruction must publish an ordered topology update");
    };
    assert!(matches!(
        reconstructed.effects().first(),
        Some(CdpTopologyEffect::ReconnectStarted { epoch })
            if *epoch == ConnectionEpoch::new(2)
    ));
    assert!(matches!(
        reconstructed.effects().last(),
        Some(CdpTopologyEffect::ReconnectFinished { epoch, .. })
            if *epoch == ConnectionEpoch::new(2)
    ));
    {
        let topology = composition
            .topology()
            .expect("reconstructed topology should remain available");
        let opener_route = topology
            .topology()
            .page_session_routes()
            .find(|route| route.target_id() == opener_target_id)
            .expect("opener should be reconstructed");
        let popup_route = topology
            .topology()
            .page_session_routes()
            .find(|route| route.target_id() == popup_target_id)
            .expect("popup should be reconstructed");
        assert_eq!(opener_route.object_id(), opener_object_id);
        assert_eq!(popup_route.object_id(), popup_object_id);
        assert_ne!(opener_route.session_id(), opener_session_id);
        assert_ne!(popup_route.session_id(), popup_session_id);
    }

    close_targets_and_verify(&browser, &[popup_target_id, opener_target_id.clone()]).await;
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome must remain alive");
}
