use std::sync::Arc;
use std::time::Duration;

use pwright_backend_cdp::{
    CdpContextCreationLimits, CdpPageCloseLimits, CdpPageCreationLimits, CdpTopologyLimits,
    CdpTopologyProductionProfile, ConnectionEpoch, ImmediatePageClosePoll,
};
use pwright_cap_context_creation::BrowserNewContext;
use pwright_cap_context_lifecycle::BrowserContextClose;
use pwright_cap_page_creation::BrowserContextNewPage;
use pwright_cap_page_lifecycle::PageClose;
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpConnectionProductionProfile, CdpSession};
use pwright_codec_context_creation::ContextCreationCodecLimits;
use pwright_codec_context_lifecycle::ContextLifecycleCodecLimits;
use pwright_codec_page_creation::PageCreationCodecLimits;
use pwright_codec_page_lifecycle::PageLifecycleCodecLimits;
use pwright_composition_chromium::{
    CapabilityWireProfile, CdpPageRouteProductionProfile, ChromiumAttachProductionProfile,
    ContextCreationInstallationProfile, ContextLifecycleInstallationProfile,
    PageCreationInstallationProfile, PageLifecycleInstallationProfile, PreparedChromiumAttach,
};
use pwright_core::{
    CallId, CallMetadata, Command, ObjectId, ProtocolVersion, ResourceClass, ResourceLimits,
    ResourceRequirements, ResourceScope, Service,
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
async fn explicit_page_creation_reaches_real_chrome_with_an_initialized_main_frame() {
    // Arrange a Page-creation composition around an externally owned Chrome.
    let websocket_url = std::env::var("PWRIGHT_CDP_WS_URL")
        .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile");
    let browser_object = ObjectId::new(1);
    let context_object = ObjectId::new(2);
    let page_object = ObjectId::new(3);
    let frame_object = ObjectId::new(4);
    let epoch = ConnectionEpoch::new(1);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(8, 2, 2, 4, 4, 256),
        ResourceScope::extension(74, 1),
    )
    .expect("topology profile should validate");
    let page_routes = CdpPageRouteProductionProfile::from_topology(
        ResourceScope::extension(74, 2),
        topology_profile,
    );
    let wire = CapabilityWireProfile::new(
        ProtocolVersion::new(1, 0),
        FrameLimits::new(4096, 0, 0, 4096),
        FrameLimits::new(4096, 0, 0, 4096),
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
    .with_context_lifecycle(ContextLifecycleInstallationProfile::new(
        ContextLifecycleCodecLimits::new(1024),
        wire,
    ))
    .with_context_creation(ContextCreationInstallationProfile::new(
        CdpContextCreationLimits::new(256),
        ContextCreationCodecLimits::new(1024),
        wire,
    ))
    .with_page_lifecycle(PageLifecycleInstallationProfile::new(
        CdpPageCloseLimits::new(256, 20),
        PageLifecycleCodecLimits::new(1024),
        wire,
    ))
    .with_page_creation(PageCreationInstallationProfile::new(
        CdpPageCreationLimits::new(256),
        PageCreationCodecLimits::new(1024),
        wire,
    ));
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
        .new_controlled_page_runtime(NoDeadlineScheduler, continuation.clone())
        .expect("Page runtime should install");
    let context_creation = attached
        .browser_new_context_service(runtime.lifecycle_handle(), continuation.clone())
        .expect("context backend should construct")
        .expect("selected context creation should expose a service");
    let context_initializer = context_creation
        .call(Command::<BrowserNewContext>::new(
            CallId::new(1),
            browser_object,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("context service is infallible")
        .into_outcome()
        .expect("parent BrowserContext should initialize");
    assert_eq!(context_initializer.object_id, context_object);
    while runtime.pop_control_message().unwrap().is_some() {}
    let creation = attached
        .browser_context_new_page_service(&runtime, continuation.clone())
        .expect("creation backend should construct")
        .expect("selected Page creation should expose a service");

    // Act through the portable command service.
    let initializer = creation
        .call(Command::<BrowserContextNewPage>::new(
            CallId::new(2),
            context_object,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("command service is infallible")
        .into_outcome()
        .expect("real Page should initialize");

    // Assert Page and main Frame are both usable before success is returned.
    assert_eq!(initializer.object_id, page_object);
    assert_eq!(initializer.main_frame_id, frame_object);
    assert_eq!(runtime.is_object_live(page_object), Ok(true));
    assert_eq!(runtime.is_object_live(frame_object), Ok(true));
    assert_eq!(runtime.service().route_count(), Ok(1));
    let target_id = attached
        .topology()
        .expect("topology should remain available")
        .page_session_routes()
        .find(|route| route.object_id() == page_object)
        .expect("created Page should have a route")
        .target_id()
        .to_string();
    assert!(
        browser
            .target_get_targets()
            .await
            .expect("targets should list")
            .iter()
            .any(|target| target.target_id == target_id && target.url == "about:blank")
    );

    // Cleanup through the paired portable lifecycle service.
    let (close, _) = attached
        .page_lifecycle_services(&runtime, continuation, Arc::new(ImmediatePageClosePoll))
        .expect("Page lifecycle should expose its services");
    close
        .call(Command::<PageClose>::new(
            CallId::new(3),
            page_object,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("close service is infallible")
        .into_outcome()
        .expect("created Page should close");
    for _ in 0..20 {
        if browser
            .target_get_targets()
            .await
            .expect("targets should list")
            .iter()
            .all(|target| target.target_id != target_id)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let context_close = attached
        .browser_context_close_service_for_page_runtime(&runtime, CleanupQueue::new(1))
        .expect("context lifecycle should expose a close service");
    context_close
        .call(Command::<BrowserContextClose>::new(
            CallId::new(4),
            context_object,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("context close service is infallible")
        .into_outcome()
        .expect("temporary context should clean up");
    attached
        .connection()
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome must remain alive");
}
