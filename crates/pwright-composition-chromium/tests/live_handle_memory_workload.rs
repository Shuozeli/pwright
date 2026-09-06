use std::sync::Arc;
use std::time::{Duration, Instant};

mod support;

use pwright_backend_cdp::{
    CdpContextCreationLimits, CdpEvaluationLimits, CdpEvaluationProductionProfile,
    CdpPageCloseLimits, CdpPageCreationLimits, CdpTopologyLimits, CdpTopologyProductionProfile,
    ConnectionEpoch, ImmediatePageClosePoll,
};
use pwright_cap_context_creation::BrowserNewContext;
use pwright_cap_context_lifecycle::BrowserContextClose;
use pwright_cap_handle_evaluation::{EvaluateHandle, EvaluateHandleParams, EvaluationValueLimits};
use pwright_cap_handle_lifecycle::HandleDispose;
use pwright_cap_page_creation::BrowserContextNewPage;
use pwright_cap_page_lifecycle::PageClose;
use pwright_cdp::CdpConnectionProductionProfile;
use pwright_cdp::connection::ConnectionConfig;
use pwright_codec_context_creation::ContextCreationCodecLimits;
use pwright_codec_context_lifecycle::ContextLifecycleCodecLimits;
use pwright_codec_handle_evaluation::HandleEvaluationCodecLimits;
use pwright_codec_handle_lifecycle::HandleLifecycleCodecLimits;
use pwright_codec_page_creation::PageCreationCodecLimits;
use pwright_codec_page_lifecycle::PageLifecycleCodecLimits;
use pwright_composition_chromium::{
    CapabilityWireProfile, CdpPageRouteProductionProfile, ChromiumAttachProductionProfile,
    ChromiumHandleRouteProductionProfile, ContextCreationInstallationProfile,
    ContextLifecycleInstallationProfile, HandleEvaluationInstallationProfile,
    HandleLifecycleInstallationProfile, PageCreationInstallationProfile,
    PageLifecycleInstallationProfile, PreparedChromiumAttach,
};
use pwright_core::{
    CallId, CallMetadata, Command, ObjectId, ProtocolVersion, ResourceClass, ResourceScope, Service,
};
use pwright_process_metrics::{
    MemoryWorkloadKind, MemoryWorkloadLimits, MemoryWorkloadPhase, MemoryWorkloadPlan,
    MemoryWorkloadRecorder,
};
use pwright_provider_cdp::CdpWebSocketEndpoint;
use pwright_runtime::{CleanupQueue, NoDeadlineScheduler, RuntimeLimits};
use pwright_server_wire::ServerSupervisorLimits;
use pwright_transport::FrameLimits;
use support::{
    browser_process_id_from_env, correlated_observation, exact_limits, system_process_probe,
    wait_for_browser_process_count_at_most, wait_for_browser_process_count_stable,
};

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process, PWRIGHT_CDP_WS_URL, and PWRIGHT_BROWSER_PID"]
async fn retained_handle_workload_accounts_explicit_and_parent_cleanup() {
    // Arrange only Page lifecycle plus the optional Handle plugins.
    let websocket_url = std::env::var("PWRIGHT_CDP_WS_URL")
        .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile");
    let browser_process_id = browser_process_id_from_env();
    let browser_object = ObjectId::new(1);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(8, 4, 4, 8, 8, 256),
        ResourceScope::extension(88, 1),
    )
    .expect("topology profile should validate");
    let page_routes = CdpPageRouteProductionProfile::from_topology(
        ResourceScope::extension(88, 2),
        topology_profile,
    );
    let wire = CapabilityWireProfile::new(
        ProtocolVersion::new(1, 0),
        FrameLimits::new(8192, 0, 0, 8192),
        FrameLimits::new(8192, 0, 0, 8192),
    );
    let value_limits = EvaluationValueLimits::new(8, 64, 64, 2048, 2048)
        .expect("Handle value limits should validate");
    let profile = ChromiumAttachProductionProfile::new(
        CdpConnectionProductionProfile::new(ResourceScope::Connection, ConnectionConfig::default()),
        topology_profile,
        page_routes,
        RuntimeLimits::new(16, 4)
            .with_active_commands(1)
            .with_control_messages(20),
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
    ))
    .with_handle_lifecycle(HandleLifecycleInstallationProfile::new(
        ChromiumHandleRouteProductionProfile::new(ResourceScope::extension(88, 3), 8, 128, 256),
        HandleLifecycleCodecLimits::new(1024),
        wire,
    ))
    .with_handle_evaluation(HandleEvaluationInstallationProfile::new(
        CdpEvaluationProductionProfile::new(
            ResourceScope::extension(88, 4),
            CdpEvaluationLimits::new(2048, 32 * 1024, 1024, value_limits),
            1,
        ),
        128,
        256,
        HandleEvaluationCodecLimits::new(2048, 1024, value_limits)
            .expect("Handle codec limits should validate"),
        wire,
    ));
    let requirements = profile.requirements();
    let prepared = PreparedChromiumAttach::new(profile, exact_limits(requirements))
        .expect("Handle workload composition should prepare");
    let mut attached = prepared
        .attach(
            CdpWebSocketEndpoint::parse(websocket_url)
                .expect("dedicated Chrome endpoint should validate"),
            browser_object,
            ConnectionEpoch::new(1),
        )
        .await
        .expect("production attach should connect");
    let continuation = CleanupQueue::new(8);
    let runtime = attached
        .new_controlled_page_runtime(NoDeadlineScheduler, continuation.clone())
        .expect("Handle-aware Page runtime should install");
    while runtime
        .pop_control_message()
        .expect("control queue should remain readable")
        .is_some()
    {}
    let context_creation = attached
        .browser_new_context_service(runtime.lifecycle_handle(), continuation.clone())
        .expect("context backend should construct")
        .expect("context creation should be installed");
    let page_creation = attached
        .browser_context_new_page_service(&runtime, continuation.clone())
        .expect("page backend should construct")
        .expect("page creation should be installed");
    let (page_close, _) = attached
        .page_lifecycle_services(
            &runtime,
            continuation.clone(),
            Arc::new(ImmediatePageClosePoll),
        )
        .expect("page lifecycle should be installed");
    let context_close = attached
        .browser_context_close_service_for_page_runtime(&runtime, continuation)
        .expect("context lifecycle should be installed");
    let (handle_create, handle_dispose, _) = attached
        .handle_services(&runtime)
        .expect("Handle services should construct")
        .expect("Handle plugins should be installed");
    let handle_routes = runtime
        .service()
        .handle_routes()
        .expect("Handle-aware runtime should expose its route table");
    let probe = system_process_probe();
    let plan = MemoryWorkloadPlan::new(
        MemoryWorkloadKind::Handles,
        1,
        3,
        10,
        5_000,
        MemoryWorkloadLimits::new(8, 4).expect("workload limits should validate"),
    )
    .expect("workload plan should validate");
    let mut recorder = MemoryWorkloadRecorder::new(plan).expect("recorder should allocate");
    let started = Instant::now();
    let observe = |phase| {
        let topology = attached
            .topology()
            .expect("topology should remain readable")
            .metrics();
        let control_messages = runtime
            .control_message_metrics()
            .expect("control metrics should remain readable")
            .map_or(0, |metrics| metrics.current_len);
        correlated_observation(
            phase,
            &started,
            &probe,
            browser_process_id,
            attached
                .resources()
                .metrics()
                .expect("resource metrics should remain readable"),
            topology,
            runtime
                .pending_call_count()
                .expect("runtime should remain readable"),
            control_messages,
        )
    };
    let baseline = observe(MemoryWorkloadPhase::Baseline);
    recorder.record(baseline).expect("baseline should record");

    let warmup_context = context_creation
        .call(Command::<BrowserNewContext>::new(
            CallId::new(1),
            browser_object,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("context service should be infallible")
        .into_outcome()
        .expect("warmup BrowserContext should be created");
    let warmup_page = page_creation
        .call(Command::<BrowserContextNewPage>::new(
            CallId::new(2),
            warmup_context.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("page service should be infallible")
        .into_outcome()
        .expect("warmup Page should be created");
    let warmup_handle = handle_create
        .call(Command::<EvaluateHandle>::new(
            CallId::new(3),
            warmup_page.main_frame_id,
            CallMetadata::default(),
            EvaluateHandleParams::expression("document.createElement('section')"),
        ))
        .await
        .expect("Handle service should be infallible")
        .into_outcome()
        .expect("warmup ElementHandle should be created");
    handle_dispose
        .call(Command::<HandleDispose>::new(
            CallId::new(4),
            warmup_handle.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("dispose service should be infallible")
        .into_outcome()
        .expect("warmup Handle should explicitly dispose");
    page_close
        .call(Command::<PageClose>::new(
            CallId::new(5),
            warmup_page.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("page close should be infallible")
        .into_outcome()
        .expect("warmup Page should close");
    context_close
        .call(Command::<BrowserContextClose>::new(
            CallId::new(6),
            warmup_context.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("context close should be infallible")
        .into_outcome()
        .expect("warmup BrowserContext should close");
    while runtime
        .pop_control_message()
        .expect("control queue should remain readable")
        .is_some()
    {}
    wait_for_browser_process_count_stable(
        &probe,
        browser_process_id,
        Duration::from_millis(250),
        Duration::from_millis(5_000),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(plan.sample_interval_millis())).await;
    let warmup = observe(MemoryWorkloadPhase::Warmup);
    let warm_browser_processes = warmup.process.browser.total.processes;
    recorder
        .record(warmup)
        .expect("clean warmup observation should record");

    let context = context_creation
        .call(Command::<BrowserNewContext>::new(
            CallId::new(7),
            browser_object,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("context service should be infallible")
        .into_outcome()
        .expect("measured BrowserContext should be created");
    let page = page_creation
        .call(Command::<BrowserContextNewPage>::new(
            CallId::new(8),
            context.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("page service should be infallible")
        .into_outcome()
        .expect("measured Page should be created");
    while runtime
        .pop_control_message()
        .expect("control queue should remain readable")
        .is_some()
    {}
    let mut handles = Vec::with_capacity(3);
    for iteration in 0..3_u64 {
        let handle = handle_create
            .call(Command::<EvaluateHandle>::new(
                CallId::new(9 + iteration),
                page.main_frame_id,
                CallMetadata::default(),
                EvaluateHandleParams::expression(format!(
                    "document.createElement('pwright-memory-{iteration}')"
                )),
            ))
            .await
            .expect("Handle service should be infallible")
            .into_outcome()
            .expect("measured ElementHandle should be created");
        handles.push(handle.object_id);
        while runtime
            .pop_control_message()
            .expect("control queue should remain readable")
            .is_some()
        {}
        tokio::time::sleep(Duration::from_millis(plan.sample_interval_millis())).await;
        let observation = observe(MemoryWorkloadPhase::Steady);
        let expected_handles = usize::try_from(iteration + 1).expect("three handles should fit");
        assert_eq!(observation.objects.handles, expected_handles);
        assert_eq!(handle_routes.len(), Ok(expected_handles));
        assert_eq!(runtime.live_object_count(), Ok(4 + expected_handles));
        assert_eq!(
            observation
                .resources
                .class(ResourceClass::RemoteHandle)
                .current,
            iteration + 1
        );
        recorder
            .record(observation)
            .expect("steady Handle observation should record");
    }
    recorder
        .record(observe(MemoryWorkloadPhase::PreTeardown))
        .expect("pre-teardown should record");
    handle_dispose
        .call(Command::<HandleDispose>::new(
            CallId::new(12),
            handles[0],
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("dispose service should be infallible")
        .into_outcome()
        .expect("first measured Handle should explicitly dispose");
    page_close
        .call(Command::<PageClose>::new(
            CallId::new(13),
            page.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("page close should be infallible")
        .into_outcome()
        .expect("measured Page should close and retire child Handles");
    context_close
        .call(Command::<BrowserContextClose>::new(
            CallId::new(14),
            context.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("context close should be infallible")
        .into_outcome()
        .expect("measured BrowserContext should close");
    while runtime
        .pop_control_message()
        .expect("control queue should remain readable")
        .is_some()
    {}
    wait_for_browser_process_count_at_most(
        &probe,
        browser_process_id,
        warm_browser_processes,
        Duration::from_millis(4_000),
    )
    .await;
    recorder
        .record(observe(MemoryWorkloadPhase::PostTeardown))
        .expect("post-teardown should record");

    // Act: seal correlated Handle-retention observations.
    let report = recorder.finish().expect("complete workload should finish");

    // Assert both explicit and parent cleanup returned every retention boundary.
    let route_metrics = handle_routes
        .metrics()
        .expect("Handle route metrics should remain readable");
    assert!(report.logical_state_returned_to_baseline());
    assert!(report.browser_process_tree_returned_to_baseline());
    assert_eq!(report.post_teardown().objects.handles, 0);
    assert_eq!(handle_routes.len(), Ok(0));
    assert_eq!(runtime.live_object_count(), Ok(1));
    assert_eq!(route_metrics.current, 0);
    assert_eq!(route_metrics.created, 4);
    assert_eq!(route_metrics.explicitly_disposed, 2);
    assert_eq!(route_metrics.parent_retired, 2);
    assert_eq!(route_metrics.retained_bytes, 0);
    assert_eq!(
        report
            .post_teardown()
            .resources
            .class(ResourceClass::RemoteHandle)
            .current,
        0
    );
    eprintln!(
        "pwright Handle workload: observations={}, peak_handles={}, explicit={}, parent_retired={}, runtime_delta={}, browser_delta={}",
        report.observations().len(),
        route_metrics.peak,
        route_metrics.explicitly_disposed,
        route_metrics.parent_retired,
        report.runtime_rss_delta(),
        report.browser_rss_sum_delta()
    );
}
