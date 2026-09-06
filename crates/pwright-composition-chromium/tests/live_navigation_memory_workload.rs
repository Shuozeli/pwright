use std::sync::Arc;
use std::time::{Duration, Instant};

mod support;

use pwright_backend_cdp::{
    CdpContextCreationLimits, CdpNavigationLimits, CdpNavigationProductionProfile,
    CdpPageCloseLimits, CdpPageCreationLimits, CdpTopologyLimits, CdpTopologyProductionProfile,
    ConnectionEpoch, ImmediatePageClosePoll,
};
use pwright_cap_context_creation::BrowserNewContext;
use pwright_cap_context_lifecycle::BrowserContextClose;
use pwright_cap_navigation::{Goto, GotoParams};
use pwright_cap_page_creation::BrowserContextNewPage;
use pwright_cap_page_lifecycle::PageClose;
use pwright_cdp::CdpConnectionProductionProfile;
use pwright_cdp::connection::ConnectionConfig;
use pwright_codec_context_creation::ContextCreationCodecLimits;
use pwright_codec_context_lifecycle::ContextLifecycleCodecLimits;
use pwright_codec_navigation::NavigationCodecLimits;
use pwright_codec_page_creation::PageCreationCodecLimits;
use pwright_codec_page_lifecycle::PageLifecycleCodecLimits;
use pwright_composition_chromium::{
    CapabilityWireProfile, CdpPageRouteProductionProfile, ChromiumAttachProductionProfile,
    ContextCreationInstallationProfile, ContextLifecycleInstallationProfile,
    NavigationInstallationProfile, PageCreationInstallationProfile,
    PageLifecycleInstallationProfile, PreparedChromiumAttach,
};
use pwright_core::{
    CallId, CallMetadata, Command, ObjectId, ProtocolVersion, ResourceScope, Service,
};
use pwright_process_metrics::{
    MemoryWorkloadKind, MemoryWorkloadLimits, MemoryWorkloadPhase, MemoryWorkloadPlan,
    MemoryWorkloadRecorder, MemoryWorkloadReport, MemoryWorkloadSeriesLimits,
    MemoryWorkloadSeriesPlan, MemoryWorkloadSeriesRecorder,
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
async fn repeated_playwright_shaped_goto_workload_returns_to_its_warm_baseline() {
    // Arrange three independent attachment/runtime runs. Each run has its own
    // cleaned warmup, persistent Page, three navigations, and full teardown.
    let websocket_url = std::env::var("PWRIGHT_CDP_WS_URL")
        .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile");
    let browser_process_id = browser_process_id_from_env();
    let series_plan = MemoryWorkloadSeriesPlan::new(
        navigation_workload_plan(),
        3,
        MemoryWorkloadSeriesLimits::new(3).expect("series limits should validate"),
    )
    .expect("three-run navigation series should validate");
    let mut series =
        MemoryWorkloadSeriesRecorder::new(series_plan).expect("series should allocate");

    // Act
    for run in 0..series_plan.runs() {
        let report = run_navigation_workload(&websocket_url, browser_process_id).await;
        eprintln!(
            "pwright navigation workload run {run}: runtime_delta={}, browser_delta={}",
            report.runtime_rss_delta(),
            report.browser_rss_sum_delta()
        );
        series
            .record(report)
            .expect("every navigation run should cleanly settle");
    }
    let report = series.finish().expect("navigation series should finish");

    // Assert
    assert_eq!(report.reports().len(), 3);
    eprintln!(
        "pwright navigation workload series: runs={}, runtime_delta_range={:?}, browser_delta_range={:?}",
        report.reports().len(),
        report.runtime_rss_delta_range(),
        report.browser_rss_sum_delta_range()
    );
}

fn navigation_workload_plan() -> MemoryWorkloadPlan {
    MemoryWorkloadPlan::new(
        MemoryWorkloadKind::Navigation,
        1,
        3,
        10,
        5_000,
        MemoryWorkloadLimits::new(8, 4).expect("workload limits should validate"),
    )
    .expect("workload plan should validate")
}

async fn run_navigation_workload(
    websocket_url: &str,
    browser_process_id: pwright_process_metrics::ProcessId,
) -> MemoryWorkloadReport {
    // Arrange one persistent Page and a navigation-only optional capability.
    let browser_object = ObjectId::new(1);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(16, 4, 4, 8, 8, 256),
        ResourceScope::extension(87, 1),
    )
    .expect("topology profile should validate");
    let page_routes = CdpPageRouteProductionProfile::from_topology(
        ResourceScope::extension(87, 2),
        topology_profile,
    );
    let wire = CapabilityWireProfile::new(
        ProtocolVersion::new(1, 0),
        FrameLimits::new(8192, 0, 0, 8192),
        FrameLimits::new(8192, 0, 0, 8192),
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
    ))
    .with_navigation(NavigationInstallationProfile::new(
        CdpNavigationProductionProfile::new(
            ResourceScope::extension(87, 3),
            CdpNavigationLimits::new(4096, 256, 16, 16, 4096, 256),
            1,
        ),
        NavigationCodecLimits::new(8192, 16, 4096),
        wire,
    ));
    let requirements = profile.requirements();
    let prepared = PreparedChromiumAttach::new(profile, exact_limits(requirements))
        .expect("navigation workload composition should prepare");
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
    let plan = navigation_workload_plan();
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
        .expect("context command service should be infallible")
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
        .expect("page command service should be infallible")
        .into_outcome()
        .expect("warmup Page should be created");
    let warmup_url = "data:text/html,<title>pwright-memory-warmup</title>";
    let warmup_result = runtime
        .call(Command::<Goto>::new(
            CallId::new(3),
            warmup_page.object_id,
            CallMetadata::default(),
            GotoParams::new(warmup_url),
        ))
        .await
        .expect("runtime dispatch should succeed")
        .into_outcome()
        .expect("warmup goto should reach real Chrome");
    assert_eq!(warmup_result.url, warmup_url);
    page_close
        .call(Command::<PageClose>::new(
            CallId::new(4),
            warmup_page.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("page close service should be infallible")
        .into_outcome()
        .expect("warmup Page should close");
    context_close
        .call(Command::<BrowserContextClose>::new(
            CallId::new(5),
            warmup_context.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("context close service should be infallible")
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
            CallId::new(6),
            browser_object,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("context command service should be infallible")
        .into_outcome()
        .expect("measured BrowserContext should be created");
    let page = page_creation
        .call(Command::<BrowserContextNewPage>::new(
            CallId::new(7),
            context.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("page command service should be infallible")
        .into_outcome()
        .expect("measured Page should be created");
    while runtime
        .pop_control_message()
        .expect("control queue should remain readable")
        .is_some()
    {}

    for iteration in 0..3_u64 {
        let url = format!(
            "data:text/html,<title>pwright-memory-steady-{iteration}</title><main>{iteration}</main>"
        );
        let result = runtime
            .call(Command::<Goto>::new(
                CallId::new(8 + iteration),
                page.object_id,
                CallMetadata::default(),
                GotoParams::new(&url),
            ))
            .await
            .expect("runtime dispatch should succeed")
            .into_outcome()
            .expect("Playwright-shaped goto should reach real Chrome");
        assert_eq!(result.url, url);
        assert!(!result.same_document);
        tokio::time::sleep(Duration::from_millis(plan.sample_interval_millis())).await;
        recorder
            .record(observe(MemoryWorkloadPhase::Steady))
            .expect("completed navigation observation should record");
    }
    recorder
        .record(observe(MemoryWorkloadPhase::PreTeardown))
        .expect("pre-teardown should record");
    page_close
        .call(Command::<PageClose>::new(
            CallId::new(11),
            page.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("page close service should be infallible")
        .into_outcome()
        .expect("real Page should close");
    context_close
        .call(Command::<BrowserContextClose>::new(
            CallId::new(12),
            context.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("context close service should be infallible")
        .into_outcome()
        .expect("real BrowserContext should close");
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

    // Act: seal the correlated navigation observations.
    let report = recorder.finish().expect("complete workload should finish");

    // Assert navigation-local state settled and Page teardown released objects.
    assert!(report.logical_state_returned_to_baseline());
    assert!(report.browser_process_tree_returned_to_baseline());
    assert_eq!(report.baseline().objects.contexts, 0);
    assert_eq!(report.baseline().objects.pages, 0);
    assert_eq!(report.baseline().objects.frames, 0);
    assert_eq!(report.comparison_baseline().objects.contexts, 0);
    assert_eq!(report.comparison_baseline().objects.pages, 0);
    assert_eq!(report.comparison_baseline().objects.frames, 0);
    assert_eq!(report.post_teardown().objects.contexts, 0);
    assert_eq!(report.post_teardown().objects.pages, 0);
    assert_eq!(report.post_teardown().objects.frames, 0);
    assert!(
        report
            .observations()
            .iter()
            .filter(|observation| observation.phase == MemoryWorkloadPhase::Steady)
            .all(|observation| observation.objects.contexts == 1
                && observation.objects.pages == 1
                && observation.objects.frames == 1
                && observation.objects.pending_calls == 0
                && observation.objects.queued_events == 0)
    );
    eprintln!(
        "pwright navigation workload: observations={}, runtime_rss={}→{}, browser_rss_sum={}→{}, browser_processes={}→{}, runtime_delta={}, browser_delta={}",
        report.observations().len(),
        report.baseline().process.runtime.resident_bytes,
        report.post_teardown().process.runtime.resident_bytes,
        report.baseline().process.browser.total.resident_bytes,
        report.post_teardown().process.browser.total.resident_bytes,
        report.baseline().process.browser.total.processes,
        report.post_teardown().process.browser.total.processes,
        report.runtime_rss_delta(),
        report.browser_rss_sum_delta()
    );
    report
}
