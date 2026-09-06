use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pwright_backend_cdp_topology::{
    CdpContextOwnership, CdpTopologyLimits, CdpTopologyProductionProfile, ConnectionEpoch,
    ProductionCdpTopology,
};
use pwright_browser_model::{
    BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG,
};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cap_page_creation::BrowserContextNewPage;
use pwright_cap_page_lifecycle::PageClose;
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpClient, CdpSession, CdpSessionFactory};
use pwright_composition_chromium_page_creation::{
    AtomicObjectIdSource, CdpPageCloseLimits, CdpPageClosePoll, CdpPageCreationLimits,
    ChromiumPageContinuations, ChromiumPageCoreBindings, ChromiumPageCreationInstallation,
    ChromiumPageCreationProfile, ChromiumPageLifecycleProfile, FrameLimits,
    PageCreationCodecLimits, PageLifecycleCodecLimits, SessionFactoryChromiumPageResolver,
};
use pwright_core::{
    CallId, CallMetadata, Command, ObjectId, ProductionResourceLedger, ResourceClass,
    ResourceLimits, ResourceRequirements, ResourceScope, Service,
};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use pwright_runtime::{
    CleanupQueue, ControlledDispatcher, NoDeadlineScheduler, ObjectTopologyMode, RuntimeLimits,
};

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
        let required = requirements.class(class);
        limits = limits.with_class(class, required.global, required.per_scope);
    }
    limits
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn paired_page_creation_and_close_reach_real_chrome_without_owning_it() {
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
        .expect("temporary Context should open");

    let runtime_limits = RuntimeLimits::new(8, 2)
        .with_active_commands(1)
        .with_control_messages(8);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(8, 2, 2, 4, 2, 1024),
        ResourceScope::extension(96, 1),
    )
    .expect("topology profile should validate");
    let frames = FrameLimits::new(16 * 1024, 0, 0, 16 * 1024);
    let lifecycle = ChromiumPageLifecycleProfile::new(
        CdpPageCloseLimits::new(1024, 20),
        PageLifecycleCodecLimits::new(1024),
        frames,
        frames,
    );
    let profile = ChromiumPageCreationProfile::new(
        CdpPageCreationLimits::new(1024).with_cleanup_confirmation_attempts(20),
        PageCreationCodecLimits::new(1024),
        frames,
        frames,
        lifecycle,
    );
    let requirements = topology_profile
        .requirements()
        .combine_concurrent(runtime_limits.resource_requirements())
        .combine_concurrent(profile.requirements());
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("exact Page-creation resources should build");
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
    runtime
        .register_object(
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            BROWSER_CONTEXT_OBJECT_KIND.tag,
        )
        .expect("Context should register");
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
        .expect("temporary Context should register");
    let topology = Arc::new(Mutex::new(topology));
    let browser_client: Arc<dyn CdpClient> = Arc::new(CdpSession::browser(Arc::clone(&connection)));
    let session_factory = Arc::new(CdpSessionFactory::new(Arc::clone(&connection)));
    let poll = Arc::new(TokioPageClosePoll(Duration::from_millis(25)));
    let installation = ChromiumPageCreationInstallation::install_without_plugins(
        ChromiumPageCoreBindings {
            browser_client,
            session_factory: session_factory.clone(),
            session_resolver: Arc::new(SessionFactoryChromiumPageResolver::new(session_factory)),
            topology: Arc::clone(&topology),
            runtime: runtime.lifecycle_handle(),
            object_ids: Arc::new(
                AtomicObjectIdSource::new(3).expect("Page IDs should start at three"),
            ),
        },
        ChromiumPageContinuations {
            creation: CleanupQueue::new(1),
            close: CleanupQueue::new(1),
        },
        poll,
        profile,
        &resources,
    )
    .expect("paired Page composition should install");

    // Act
    let initializer = installation
        .creation_service()
        .call(Command::<BrowserContextNewPage>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("creation service is infallible")
        .into_outcome()
        .expect("Page should create through the composition");
    let target_id = topology
        .lock()
        .expect("topology should remain available")
        .topology()
        .page_session_routes()
        .find(|route| route.object_id() == initializer.object_id)
        .expect("created Page should have a route")
        .target_id()
        .to_owned();
    installation
        .lifecycle()
        .close_service()
        .call(Command::<PageClose>::new(
            CallId::new(2),
            initializer.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("close service is infallible")
        .into_outcome()
        .expect("created Page should close through paired lifecycle");
    let _handlers = installation.handlers();
    drop(installation);
    let targets = browser
        .target_get_targets()
        .await
        .expect("target list should remain available");

    // Assert
    assert!(!targets.iter().any(|target| target.target_id == target_id));
    assert_eq!(runtime.is_object_live(ObjectId::new(2)), Ok(true));
    assert_eq!(runtime.is_object_live(initializer.object_id), Ok(false));
    assert_eq!(runtime.is_object_live(initializer.main_frame_id), Ok(false));
    assert_eq!(
        topology
            .lock()
            .expect("topology should remain available")
            .page_object_id(&target_id),
        None
    );
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("external browser should remain alive after composition drop");
    browser
        .target_dispose_browser_context(&browser_context_id)
        .await
        .expect("temporary Context should clean up");
}
