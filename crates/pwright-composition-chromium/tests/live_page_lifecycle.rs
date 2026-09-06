use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use pwright_backend_cdp::{
    CdpContextOwnership, CdpPageCloseLimits, CdpPageClosePoll, CdpPageObjectIds, CdpPageTarget,
    CdpTopologyLimits, CdpTopologyProductionProfile, ConnectionEpoch,
};
use pwright_browser_model::{BROWSER_CONTEXT_OBJECT_KIND, FRAME_OBJECT_KIND, PAGE_OBJECT_KIND};
use pwright_cap_page_lifecycle::PageClose;
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpConnectionProductionProfile, CdpSession};
use pwright_codec_page_lifecycle::PageLifecycleCodecLimits;
use pwright_composition_chromium::{
    CapabilityWireProfile, CdpPageRouteProductionProfile, ChromiumAttachProductionProfile,
    PageLifecycleInstallationProfile, PreparedChromiumAttach,
};
use pwright_core::{
    CallId, CallMetadata, Command, ObjectId, ProtocolVersion, ResourceClass, ResourceLimits,
    ResourceRequirements, ResourceScope, Service,
};
use pwright_provider_cdp::CdpWebSocketEndpoint;
use pwright_runtime::{CleanupQueue, NoDeadlineScheduler, RuntimeLimits};
use pwright_server_wire::ServerSupervisorLimits;
use pwright_transport::FrameLimits;

#[derive(Clone, Copy)]
struct TokioPageClosePoll(Duration);

impl CdpPageClosePoll for TokioPageClosePoll {
    fn wait(&self, _completed_checks: usize) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(tokio::time::sleep(self.0))
    }
}

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
async fn explicit_page_close_reaches_real_chrome_and_preserves_context_and_browser() {
    // Arrange the complete Page-lifecycle attachment without launching or discovering Chrome.
    let websocket_url = std::env::var("PWRIGHT_CDP_WS_URL")
        .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile");
    let browser_object = ObjectId::new(1);
    let context_object = ObjectId::new(2);
    let page_object = ObjectId::new(3);
    let frame_object = ObjectId::new(4);
    let epoch = ConnectionEpoch::new(1);
    let runtime_limits = RuntimeLimits::new(8, 2)
        .with_active_commands(1)
        .with_control_messages(8);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(8, 2, 2, 4, 4, 256),
        ResourceScope::extension(72, 1),
    )
    .expect("topology profile should validate");
    let page_routes = CdpPageRouteProductionProfile::from_topology(
        ResourceScope::extension(72, 2),
        topology_profile,
    );
    let page_lifecycle = PageLifecycleInstallationProfile::new(
        CdpPageCloseLimits::new(256, 20),
        PageLifecycleCodecLimits::new(1024),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(4096, 0, 0, 4096),
            FrameLimits::new(4096, 0, 0, 4096),
        ),
    );
    let profile = ChromiumAttachProductionProfile::new(
        CdpConnectionProductionProfile::new(ResourceScope::Connection, ConnectionConfig::default()),
        topology_profile,
        page_routes,
        runtime_limits,
        ServerSupervisorLimits::new(1).expect("supervisor limits should validate"),
    )
    .with_page_lifecycle(page_lifecycle);
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
    let browser_context_id = browser
        .target_create_browser_context(None, None)
        .await
        .expect("temporary owned context should open");
    let target_id = browser
        .target_create_in_context("about:blank", &browser_context_id)
        .await
        .expect("temporary target should open");
    let session_id = browser
        .target_attach(&target_id)
        .await
        .expect("temporary target should attach");
    attached
        .with_topology_mut(|topology| {
            topology
                .register_context(
                    epoch,
                    &browser_context_id,
                    context_object,
                    CdpContextOwnership::Owned,
                )
                .expect("owned context should map");
            topology
                .attach_page(
                    epoch,
                    CdpPageTarget::new(&target_id, session_id, &browser_context_id),
                    CdpPageObjectIds::new(page_object, frame_object),
                )
                .expect("temporary page should map");
        })
        .expect("shared attached topology should remain available");
    let continuation = CleanupQueue::new(2);
    let runtime = attached
        .new_controlled_page_runtime(NoDeadlineScheduler, continuation.clone())
        .expect("attached composition should install the Page runtime exactly once");
    runtime
        .register_object(
            context_object,
            Some(browser_object),
            BROWSER_CONTEXT_OBJECT_KIND.tag,
        )
        .expect("context should register");
    runtime
        .register_object(page_object, Some(context_object), PAGE_OBJECT_KIND.tag)
        .expect("page should register");
    runtime
        .register_object(frame_object, Some(page_object), FRAME_OBJECT_KIND.tag)
        .expect("frame should register");
    while runtime
        .pop_control_message()
        .expect("control outbox should remain readable")
        .is_some()
    {}
    let (close_service, _before_unload_service) = attached
        .page_lifecycle_services(
            &runtime,
            continuation,
            Arc::new(TokioPageClosePoll(Duration::from_millis(25))),
        )
        .expect("selected Page lifecycle should expose both attached services");

    // Act
    close_service
        .call(Command::<PageClose>::new(
            CallId::new(1),
            page_object,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("command service is infallible")
        .into_outcome()
        .expect("real target should close");

    // Assert every local owner agrees, while the parent context and browser survive.
    assert_eq!(runtime.is_object_live(context_object), Ok(true));
    assert_eq!(runtime.is_object_live(page_object), Ok(false));
    assert_eq!(runtime.is_object_live(frame_object), Ok(false));
    assert_eq!(runtime.service().route_count(), Ok(0));
    assert_eq!(
        attached
            .topology()
            .expect("topology should remain available")
            .page_object_id(&target_id),
        None
    );
    assert!(
        browser
            .target_get_targets()
            .await
            .expect("target list should remain available")
            .iter()
            .all(|target| target.target_id != target_id)
    );
    attached
        .connection()
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("dedicated external browser must remain alive");
    browser
        .target_dispose_browser_context(&browser_context_id)
        .await
        .expect("surviving temporary context should clean up");
}
