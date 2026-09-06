use std::sync::{Arc, Mutex};

use pwright_backend_cdp_topology::{
    CdpContextOwnership, CdpPageObjectIds, CdpPageTarget, CdpTopologyLimits,
    CdpTopologyProductionProfile, ConnectionEpoch, ProductionCdpTopology,
};
use pwright_browser_model::{
    BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG,
    FRAME_OBJECT_KIND, PAGE_OBJECT_KIND,
};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cap_context_lifecycle::BrowserContextClose;
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpClient, CdpSession};
use pwright_composition_chromium_context_lifecycle::{
    ChromiumContextLifecycleInstallation, ChromiumContextLifecycleProfile,
    ContextLifecycleCodecLimits, FrameLimits,
};
use pwright_core::{
    CallId, CallMetadata, Command, ObjectId, ProductionResourceLedger, ResourceClass,
    ResourceLimits, ResourceRequirements, ResourceScope, Service,
};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use pwright_runtime::{
    CleanupQueue, ControlledDispatcher, NoDeadlineScheduler, ObjectTopologyMode, RuntimeLimits,
};

fn exact_limits(requirements: ResourceRequirements) -> ResourceLimits {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let required = requirements.class(class);
        limits = limits.with_class(class, required.global, required.per_scope);
    }
    limits
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn context_lifecycle_composition_closes_only_owned_context() {
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

    let runtime_limits = RuntimeLimits::new(8, 2)
        .with_active_commands(1)
        .with_control_messages(8);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(8, 2, 2, 4, 2, 1024),
        ResourceScope::extension(92, 1),
    )
    .expect("topology profile should validate");
    let frames = FrameLimits::new(16 * 1024, 0, 0, 16 * 1024);
    let profile = ChromiumContextLifecycleProfile::new(
        ContextLifecycleCodecLimits::new(1024),
        frames,
        frames,
    );
    let requirements = topology_profile
        .requirements()
        .combine_concurrent(runtime_limits.resource_requirements())
        .combine_concurrent(profile.requirements());
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("exact Context-lifecycle resources should build");
    let runtime = ControlledDispatcher::with_object_kinds_and_topology(
        (),
        (),
        NoDeadlineScheduler,
        runtime_limits,
        BROWSER_OBJECT_KIND_CATALOG,
        ObjectTopologyMode::Complete,
    )
    .expect("browser object catalog should validate")
    .with_production_resources(&resources)
    .expect("runtime resources should fit");
    for (object_id, parent, kind) in [
        (ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag),
        (
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            BROWSER_CONTEXT_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(3),
            Some(ObjectId::new(2)),
            PAGE_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(4),
            Some(ObjectId::new(3)),
            FRAME_OBJECT_KIND.tag,
        ),
    ] {
        runtime
            .register_object(object_id, parent, kind)
            .expect("test object should register");
    }
    while runtime
        .pop_control_message()
        .expect("control queue should load")
        .is_some()
    {}
    let epoch = ConnectionEpoch::new(1);
    let mut topology =
        ProductionCdpTopology::new(ObjectId::new(1), epoch, topology_profile, &resources)
            .expect("topology should install");
    topology
        .register_context(
            epoch,
            browser_context_id.clone(),
            ObjectId::new(2),
            CdpContextOwnership::Owned,
        )
        .expect("owned context should register");
    topology
        .attach_page(
            epoch,
            CdpPageTarget::new(target_id.clone(), session_id, browser_context_id.clone()),
            CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
        )
        .expect("temporary Page should attach");
    let topology = Arc::new(Mutex::new(topology));
    let browser_client: Arc<dyn CdpClient> = Arc::new(CdpSession::browser(Arc::clone(&connection)));
    let installation = ChromiumContextLifecycleInstallation::install_without_retirement_plugins(
        browser_client,
        Arc::clone(&topology),
        runtime.lifecycle_handle(),
        CleanupQueue::new(1),
        profile,
        &resources,
    )
    .expect("Context-lifecycle composition should install");

    // Act
    installation
        .service()
        .call(Command::<BrowserContextClose>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("command service is infallible")
        .into_outcome()
        .expect("owned context should close through the composition");
    let _handlers = installation.handlers();
    drop(installation);
    let targets = browser
        .target_get_targets()
        .await
        .expect("target list should remain available");

    // Assert
    assert!(!targets.iter().any(|target| target.target_id == target_id));
    assert_eq!(runtime.is_object_live(ObjectId::new(2)), Ok(false));
    assert_eq!(
        topology
            .lock()
            .expect("topology should remain available")
            .context_object_id(&browser_context_id),
        None
    );
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("external browser should remain alive after composition drop");
}
