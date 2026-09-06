use std::sync::Arc;

use pwright_browser_model::{
    BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG,
    FRAME_OBJECT_KIND, JS_HANDLE_OBJECT_KIND, PAGE_OBJECT_KIND,
};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cap_handle_lifecycle::HandleDispose;
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpClient, CdpSession};
use pwright_composition_chromium_handle_lifecycle::{
    CdpHandleDisposeLimits, ChromiumHandleLifecycleBindings, ChromiumHandleLifecycleInstallation,
    ChromiumHandleLifecycleProfile, ChromiumHandleRouteProfile, ChromiumHandleRouteSpec,
    FrameLimits, HandleLifecycleCodecLimits,
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
async fn independently_installed_handle_lifecycle_releases_one_real_chrome_object() {
    // Arrange one externally owned browser and one test-owned target.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .unwrap();
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .unwrap();
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let context_id = browser
        .target_create_browser_context(None, None)
        .await
        .unwrap();
    let target_id = browser
        .target_create_in_context("about:blank", &context_id)
        .await
        .unwrap();
    let session_id = browser.target_attach(&target_id).await.unwrap();
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id,
    ));
    let response = page
        .runtime_evaluate_as_object("({ pwright: 'handle-lifecycle-live' })")
        .await
        .unwrap();
    let native_id = response["result"]["objectId"]
        .as_str()
        .expect("object evaluation should retain a native ID")
        .to_owned();

    let runtime_limits = RuntimeLimits::new(8, 2)
        .with_active_commands(1)
        .with_control_messages(8);
    let frame_limits = FrameLimits::new(4096, 0, 0, 4096);
    let profile = ChromiumHandleLifecycleProfile::new(
        ChromiumHandleRouteProfile::new(ResourceScope::extension(97, 2), 1, 4096, 4096),
        HandleLifecycleCodecLimits::new(1024),
        frame_limits,
        frame_limits,
    );
    let requirements = runtime_limits
        .resource_requirements()
        .combine_concurrent(profile.requirements());
    let resources =
        ProductionResourceLedger::new(exact_limits(requirements), requirements).unwrap();
    let runtime = ControlledDispatcher::with_object_kinds_and_topology(
        (),
        (),
        NoDeadlineScheduler,
        runtime_limits,
        BROWSER_OBJECT_KIND_CATALOG,
        ObjectTopologyMode::Complete,
    )
    .unwrap()
    .with_production_resources(&resources)
    .unwrap();
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
        (
            ObjectId::new(5),
            Some(ObjectId::new(4)),
            JS_HANDLE_OBJECT_KIND.tag,
        ),
    ] {
        runtime.register_object(object_id, parent, kind).unwrap();
    }
    let installation = ChromiumHandleLifecycleInstallation::install(
        ChromiumHandleLifecycleBindings {
            runtime: runtime.lifecycle_handle(),
            continuation: CleanupQueue::new(1),
        },
        profile,
        &resources,
    )
    .unwrap();
    let route = ChromiumHandleRouteSpec::from_cdp(
        ObjectId::new(4),
        ObjectId::new(3),
        Arc::clone(&page),
        Some(&native_id),
        0,
        CdpHandleDisposeLimits::new(4096),
    )
    .unwrap();
    installation
        .routes()
        .reserve()
        .unwrap()
        .publish(ObjectId::new(5), route)
        .unwrap();

    // Act through the independently installed typed composition service.
    let result = installation
        .dispose_service()
        .call(Command::<HandleDispose>::new(
            CallId::new(1),
            ObjectId::new(5),
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap();

    // Assert local ownership retired and the external browser stayed alive.
    result.outcome().as_ref().expect("dispose should succeed");
    assert!(!runtime.is_object_live(ObjectId::new(5)).unwrap());
    assert!(!installation.routes().contains(ObjectId::new(5)).unwrap());
    browser
        .target_dispose_browser_context(&context_id)
        .await
        .unwrap();
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome must remain alive");
}
