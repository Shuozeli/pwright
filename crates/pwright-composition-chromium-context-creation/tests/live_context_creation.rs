use std::sync::{Arc, Mutex};

use pwright_backend_cdp_topology::{
    CdpTopologyLimits, CdpTopologyProductionProfile, ConnectionEpoch, ProductionCdpTopology,
};
use pwright_browser_model::{BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cap_context_creation::BrowserNewContext;
use pwright_cap_context_lifecycle::BrowserContextClose;
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpClient, CdpSession};
use pwright_composition_chromium_context_creation::{
    AtomicObjectIdSource, CdpContextCreationLimits, ChromiumContextContinuations,
    ChromiumContextCoreBindings, ChromiumContextCreationInstallation,
    ChromiumContextCreationProfile, ChromiumContextLifecycleProfile, ContextCreationCodecLimits,
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
async fn paired_context_composition_creates_and_closes_on_real_chrome() {
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

    let runtime_limits = RuntimeLimits::new(8, 2)
        .with_active_commands(1)
        .with_control_messages(8);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(8, 2, 2, 4, 2, 1024),
        ResourceScope::extension(94, 1),
    )
    .expect("topology profile should validate");
    let frames = FrameLimits::new(16 * 1024, 0, 0, 16 * 1024);
    let lifecycle = ChromiumContextLifecycleProfile::new(
        ContextLifecycleCodecLimits::new(1024),
        frames,
        frames,
    );
    let profile = ChromiumContextCreationProfile::new(
        CdpContextCreationLimits::new(1024),
        ContextCreationCodecLimits::new(1024),
        frames,
        frames,
        lifecycle,
    );
    let requirements = topology_profile
        .requirements()
        .combine_concurrent(runtime_limits.resource_requirements())
        .combine_concurrent(profile.requirements());
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("exact Context resources should build");
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
    runtime
        .register_object(ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag)
        .expect("Browser should register");
    assert!(
        runtime
            .pop_control_message()
            .expect("control queue should load")
            .is_some()
    );
    let topology = Arc::new(Mutex::new(
        ProductionCdpTopology::new(
            ObjectId::new(1),
            ConnectionEpoch::new(1),
            topology_profile,
            &resources,
        )
        .expect("topology should install"),
    ));
    let browser_client: Arc<dyn CdpClient> = Arc::new(CdpSession::browser(Arc::clone(&connection)));
    let installation = ChromiumContextCreationInstallation::install_without_plugins(
        ChromiumContextCoreBindings {
            browser_client,
            topology: Arc::clone(&topology),
            runtime: runtime.lifecycle_handle(),
            object_ids: Arc::new(
                AtomicObjectIdSource::new(2).expect("Context identity should start at two"),
            ),
            browser_object_id: ObjectId::new(1),
        },
        ChromiumContextContinuations {
            creation: CleanupQueue::new(1),
            close: CleanupQueue::new(1),
        },
        profile,
        &resources,
    )
    .expect("paired Context composition should install");

    // Act
    let initializer = installation
        .creation_service()
        .call(Command::<BrowserNewContext>::new(
            CallId::new(1),
            ObjectId::new(1),
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("creation service is infallible")
        .into_outcome()
        .expect("real BrowserContext should be created");
    let native_context_id = topology
        .lock()
        .expect("topology should remain available")
        .browser_context_route(initializer.object_id)
        .expect("created Context should have a native route")
        .browser_context_id()
        .to_string();
    assert!(
        browser
            .target_get_browser_contexts()
            .await
            .expect("browser contexts should load")
            .contains(&native_context_id)
    );
    installation
        .lifecycle()
        .service()
        .call(Command::<BrowserContextClose>::new(
            CallId::new(2),
            initializer.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("lifecycle service is infallible")
        .into_outcome()
        .expect("created BrowserContext should close");
    let _handlers = installation.handlers();
    drop(installation);

    // Assert
    assert_eq!(runtime.is_object_live(initializer.object_id), Ok(false));
    assert_eq!(
        topology
            .lock()
            .expect("topology should remain available")
            .browser_context_route(initializer.object_id),
        None
    );
    assert!(
        !browser
            .target_get_browser_contexts()
            .await
            .expect("browser contexts should remain observable")
            .contains(&native_context_id)
    );
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("external browser should remain alive after composition drop");
}
