use std::sync::Arc;

use pwright_backend_cdp::{
    CdpContextCreationLimits, CdpTopologyLimits, CdpTopologyProductionProfile, ConnectionEpoch,
};
use pwright_cap_context_creation::BrowserNewContext;
use pwright_cap_context_lifecycle::BrowserContextClose;
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpConnectionProductionProfile, CdpSession};
use pwright_codec_context_creation::ContextCreationCodecLimits;
use pwright_codec_context_lifecycle::ContextLifecycleCodecLimits;
use pwright_composition_chromium::{
    CapabilityWireProfile, CdpPageRouteProductionProfile, ChromiumAttachProductionProfile,
    ContextCreationInstallationProfile, ContextLifecycleInstallationProfile,
    PreparedChromiumAttach,
};
use pwright_core::{
    CallId, CallMetadata, Command, ControlMessage, ObjectId, ProtocolVersion, ResourceClass,
    ResourceLimits, ResourceRequirements, ResourceScope, Service,
};
use pwright_provider_cdp::CdpWebSocketEndpoint;
use pwright_runtime::{CleanupQueue, NoDeadlineScheduler, RuntimeLimits};
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

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn explicit_context_creation_reaches_real_chrome_and_closes_without_killing_browser() {
    // Arrange a creation+lifecycle composition without launching or discovering a browser.
    let websocket_url = std::env::var("PWRIGHT_CDP_WS_URL")
        .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile");
    let browser_object = ObjectId::new(1);
    let context_object = ObjectId::new(2);
    let epoch = ConnectionEpoch::new(1);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(8, 2, 2, 4, 4, 256),
        ResourceScope::extension(73, 1),
    )
    .expect("topology profile should validate");
    let page_routes = CdpPageRouteProductionProfile::from_topology(
        ResourceScope::extension(73, 2),
        topology_profile,
    );
    let wire = CapabilityWireProfile::new(
        ProtocolVersion::new(1, 0),
        FrameLimits::new(4096, 0, 0, 4096),
        FrameLimits::new(4096, 0, 0, 4096),
    );
    let context_lifecycle =
        ContextLifecycleInstallationProfile::new(ContextLifecycleCodecLimits::new(1024), wire);
    let context_creation = ContextCreationInstallationProfile::new(
        CdpContextCreationLimits::new(256),
        ContextCreationCodecLimits::new(1024),
        wire,
    );
    let profile = ChromiumAttachProductionProfile::new(
        CdpConnectionProductionProfile::new(ResourceScope::Connection, ConnectionConfig::default()),
        topology_profile,
        page_routes,
        RuntimeLimits::new(8, 2)
            .with_active_commands(1)
            .with_control_messages(8),
        ServerSupervisorLimits::new(1).expect("supervisor limits should validate"),
    )
    .with_context_lifecycle(context_lifecycle)
    .with_context_creation(context_creation);
    let requirements = profile.requirements();
    let prepared = PreparedChromiumAttach::new(profile, exact_limits(requirements))
        .expect("complete production composition should prepare");
    let mut attached = prepared
        .attach(
            CdpWebSocketEndpoint::parse(websocket_url)
                .expect("dedicated Chrome endpoint should validate"),
            browser_object,
            epoch,
        )
        .await
        .expect("production attach should connect");
    let browser = Arc::new(CdpSession::browser(Arc::clone(attached.connection())));
    let continuation = CleanupQueue::new(2);
    let runtime = attached
        .new_controlled_runtime((), (), NoDeadlineScheduler, continuation.clone())
        .expect("attached composition should install one controlled runtime");
    assert!(matches!(
        runtime.pop_control_message(),
        Ok(Some(ControlMessage::Create { object_id, .. })) if object_id == browser_object
    ));
    let creation = attached
        .browser_new_context_service(runtime.lifecycle_handle(), continuation.clone())
        .expect("creation backend should construct")
        .expect("selected creation capability should expose a service");

    // Act: create through the portable command rather than calling CDP directly.
    let initializer = creation
        .call(Command::<BrowserNewContext>::new(
            CallId::new(1),
            browser_object,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("command service is infallible")
        .into_outcome()
        .expect("real BrowserContext should be created");

    // Assert the portable and native identities are both published before success.
    assert_eq!(initializer.object_id, context_object);
    assert_eq!(runtime.is_object_live(context_object), Ok(true));
    assert_eq!(
        runtime.pop_control_message(),
        Ok(Some(ControlMessage::Create {
            object_id: context_object,
            parent: Some(browser_object),
            kind: initializer.object_kind,
        }))
    );
    let native_context_id = attached
        .topology()
        .expect("topology should remain available")
        .browser_context_route(context_object)
        .expect("created context should have a native route")
        .browser_context_id()
        .to_string();
    let target_id = browser
        .target_create_in_context("about:blank", &native_context_id)
        .await
        .expect("created native context should accept a target");

    // Act: close the newly created context through the paired portable lifecycle service.
    let close = attached
        .browser_context_close_service(runtime.lifecycle_handle(), continuation)
        .expect("selected lifecycle capability should expose a close service");
    close
        .call(Command::<BrowserContextClose>::new(
            CallId::new(2),
            context_object,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("command service is infallible")
        .into_outcome()
        .expect("created BrowserContext should close");

    // Assert native cleanup happened while the externally owned browser survived.
    assert_eq!(runtime.is_object_live(context_object), Ok(false));
    assert!(
        attached
            .topology()
            .expect("topology should remain available")
            .browser_context_route(context_object)
            .is_none()
    );
    for _ in 0..20 {
        let targets = browser
            .target_get_targets()
            .await
            .expect("target list should remain available");
        if !targets.iter().any(|target| target.target_id == target_id) {
            attached
                .connection()
                .send_command("Browser.getVersion", serde_json::json!({}))
                .await
                .expect("dedicated external browser must remain alive");
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("target remained after closing the newly created BrowserContext");
}
