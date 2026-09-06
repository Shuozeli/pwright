use std::sync::Arc;
use std::time::{Duration, Instant};

mod support;

use pwright_backend_cdp::{
    CdpContextCreationLimits, CdpPageCloseLimits, CdpPageCreationLimits, CdpTopologyLimits,
    CdpTopologyProductionProfile, ConnectionEpoch, ImmediatePageClosePoll,
};
use pwright_cap_context_creation::BrowserNewContext;
use pwright_cap_context_lifecycle::BrowserContextClose;
use pwright_cap_page_creation::BrowserContextNewPage;
use pwright_cap_page_lifecycle::PageClose;
use pwright_cdp::CdpConnectionProductionProfile;
use pwright_cdp::connection::ConnectionConfig;
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
    CallId, CallMetadata, Command, ObjectId, ProtocolVersion, ResourceScope, Service,
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
async fn repeated_page_and_context_workload_returns_to_its_logical_baseline() {
    // Arrange a caller-driven composition, workload plan, and bounded OS probe.
    let websocket_url = std::env::var("PWRIGHT_CDP_WS_URL")
        .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile");
    let browser_process_id = browser_process_id_from_env();
    let browser_object = ObjectId::new(1);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(16, 4, 4, 8, 8, 256),
        ResourceScope::extension(86, 1),
    )
    .expect("topology profile should validate");
    let page_routes = CdpPageRouteProductionProfile::from_topology(
        ResourceScope::extension(86, 2),
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
        RuntimeLimits::new(16, 4)
            .with_active_commands(1)
            .with_control_messages(16),
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
        .expect("memory workload composition should prepare");
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
        .expect("Page runtime should install");
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
    let probe = system_process_probe();
    let plan = MemoryWorkloadPlan::new(
        MemoryWorkloadKind::RepeatedPageCreation,
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
        let resources = attached
            .resources()
            .metrics()
            .expect("resource metrics should remain readable");
        correlated_observation(
            phase,
            &started,
            &probe,
            browser_process_id,
            resources,
            topology,
            runtime
                .pending_call_count()
                .expect("runtime should remain readable"),
            control_messages,
        )
    };
    let baseline = observe(MemoryWorkloadPhase::Baseline);
    recorder.record(baseline).expect("baseline should record");
    let mut next_call_id = 1_u64;
    let mut comparison_browser_processes = baseline.process.browser.total.processes;

    for iteration in 0..4 {
        let context = context_creation
            .call(Command::<BrowserNewContext>::new(
                CallId::new(next_call_id),
                browser_object,
                CallMetadata::default(),
                (),
            ))
            .await
            .expect("context command service should be infallible")
            .into_outcome()
            .expect("real BrowserContext should be created");
        next_call_id += 1;
        let page = page_creation
            .call(Command::<BrowserContextNewPage>::new(
                CallId::new(next_call_id),
                context.object_id,
                CallMetadata::default(),
                (),
            ))
            .await
            .expect("page command service should be infallible")
            .into_outcome()
            .expect("real Page should be created");
        next_call_id += 1;
        if iteration != 0 {
            recorder
                .record(observe(MemoryWorkloadPhase::Steady))
                .expect("active steady-state observation should record");
        }
        if iteration == 3 {
            recorder
                .record(observe(MemoryWorkloadPhase::PreTeardown))
                .expect("pre-teardown should record");
        }

        page_close
            .call(Command::<PageClose>::new(
                CallId::new(next_call_id),
                page.object_id,
                CallMetadata::default(),
                (),
            ))
            .await
            .expect("page close service should be infallible")
            .into_outcome()
            .expect("real Page should close");
        next_call_id += 1;
        context_close
            .call(Command::<BrowserContextClose>::new(
                CallId::new(next_call_id),
                context.object_id,
                CallMetadata::default(),
                (),
            ))
            .await
            .expect("context close service should be infallible")
            .into_outcome()
            .expect("real BrowserContext should close");
        next_call_id += 1;
        while runtime
            .pop_control_message()
            .expect("control queue should remain readable")
            .is_some()
        {}
        if iteration == 0 {
            wait_for_browser_process_count_stable(
                &probe,
                browser_process_id,
                Duration::from_millis(250),
                Duration::from_millis(5_000),
            )
            .await;
        } else if iteration == 3 {
            wait_for_browser_process_count_at_most(
                &probe,
                browser_process_id,
                comparison_browser_processes,
                Duration::from_millis(4_000),
            )
            .await;
        }
        tokio::time::sleep(Duration::from_millis(plan.sample_interval_millis())).await;
        if iteration == 0 {
            let warmup = observe(MemoryWorkloadPhase::Warmup);
            comparison_browser_processes = warmup.process.browser.total.processes;
            recorder
                .record(warmup)
                .expect("clean warmup observation should record");
        }
    }
    recorder
        .record(observe(MemoryWorkloadPhase::PostTeardown))
        .expect("post-teardown should record");

    // Act: seal the correlated process/resource/object observations.
    let report = recorder.finish().expect("complete workload should finish");

    // Assert repeated native work cleaned up to the attached-browser baseline.
    assert!(report.logical_state_returned_to_baseline());
    assert!(report.browser_process_tree_returned_to_baseline());
    assert_eq!(report.baseline().objects.contexts, 0);
    assert_eq!(report.baseline().objects.pages, 0);
    assert_eq!(report.post_teardown().objects, report.baseline().objects);
    assert!(
        report
            .observations()
            .iter()
            .filter(|observation| observation.phase == MemoryWorkloadPhase::Steady)
            .all(|observation| observation.objects.contexts == 1
                && observation.objects.pages == 1
                && observation.objects.frames == 1)
    );
    eprintln!(
        "pwright repeated-page workload: observations={}, runtime_rss_delta={}, browser_rss_sum_delta={}",
        report.observations().len(),
        report.runtime_rss_delta(),
        report.browser_rss_sum_delta()
    );
}
