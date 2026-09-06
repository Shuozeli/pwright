use std::sync::Arc;
use std::time::{Duration, Instant};

mod support;

use pwright_backend_cdp::{
    CdpContextCreationLimits, CdpNavigationLimits, CdpNavigationProductionProfile,
    CdpNetworkEventEffect, CdpNetworkEventProductionProfile, CdpPageCloseLimits,
    CdpPageCreationLimits, CdpTopologyLimits, CdpTopologyProductionProfile, ConnectionEpoch,
    ImmediatePageClosePoll,
};
use pwright_cap_context_creation::BrowserNewContext;
use pwright_cap_context_lifecycle::BrowserContextClose;
use pwright_cap_navigation::{Goto, GotoParams};
use pwright_cap_network_events::NETWORK_REQUEST_EVENT_TAG;
use pwright_cap_page_creation::BrowserContextNewPage;
use pwright_cap_page_lifecycle::PageClose;
use pwright_cdp::CdpConnectionProductionProfile;
use pwright_cdp::connection::ConnectionConfig;
use pwright_codec_context_creation::ContextCreationCodecLimits;
use pwright_codec_context_lifecycle::ContextLifecycleCodecLimits;
use pwright_codec_control::ControlCodec;
use pwright_codec_navigation::NavigationCodecLimits;
use pwright_codec_page_creation::PageCreationCodecLimits;
use pwright_codec_page_lifecycle::PageLifecycleCodecLimits;
use pwright_composition_chromium::{
    CapabilityWireProfile, CdpPageRouteProductionProfile, ChromiumAttachProductionProfile,
    ChromiumNetworkEventPoll, ChromiumNetworkEventPublisher,
    ChromiumNetworkEventsProductionProfile, ChromiumPageSubscriptionActivator,
    ContextCreationInstallationProfile, ContextLifecycleInstallationProfile,
    NavigationInstallationProfile, PageCreationInstallationProfile,
    PageLifecycleInstallationProfile, PreparedChromiumAttach,
};
use pwright_core::{
    CallId, CallMetadata, Command, ObjectId, ProtocolVersion, ResourceClass, ResourceScope, Service,
};
use pwright_network_model::NetworkLifecycleLimits;
use pwright_process_metrics::{
    MemoryWorkloadKind, MemoryWorkloadLimits, MemoryWorkloadPhase, MemoryWorkloadPlan,
    MemoryWorkloadRecorder, MemoryWorkloadReport, MemoryWorkloadSeriesLimits,
    MemoryWorkloadSeriesPlan, MemoryWorkloadSeriesRecorder,
};
use pwright_provider_cdp::CdpWebSocketEndpoint;
use pwright_runtime::{CleanupQueue, NoDeadlineScheduler, RuntimeLimits};
use pwright_runtime_control::{OrderedRuntimeControl, RuntimeControlDriver, SubscriptionActivator};
use pwright_server_wire::{ConcurrentServerOutbound, ServerSupervisorLimits};
use pwright_transport::{FrameLimits, FrameTransport, VersionPolicy, WireProtocol};
use pwright_transport_memory::MemoryFrameTransport;
use support::{
    browser_process_id_from_env, correlated_observation, exact_limits, system_process_probe,
    wait_for_browser_process_count_at_most, wait_for_browser_process_count_stable,
};

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process, PWRIGHT_CDP_WS_URL, and PWRIGHT_BROWSER_PID"]
async fn repeated_network_lifecycles_return_to_their_warm_baseline() {
    // Arrange three independent attachment/runtime runs. Every run contains
    // its own cleaned warmup, persistent Page, three network lifecycles, and
    // complete teardown before it can enter the bounded series.
    let websocket_url = std::env::var("PWRIGHT_CDP_WS_URL")
        .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile");
    let browser_process_id = browser_process_id_from_env();
    let series_plan = MemoryWorkloadSeriesPlan::new(
        network_workload_plan(),
        3,
        MemoryWorkloadSeriesLimits::new(3).expect("series limits should validate"),
    )
    .expect("three-run network series should validate");
    let mut series =
        MemoryWorkloadSeriesRecorder::new(series_plan).expect("series should allocate");

    // Act
    for run in 0..series_plan.runs() {
        let report = run_network_workload(&websocket_url, browser_process_id).await;
        eprintln!(
            "pwright network workload run {run}: runtime_delta={}, browser_delta={}",
            report.runtime_rss_delta(),
            report.browser_rss_sum_delta()
        );
        series
            .record(report)
            .expect("every network run should cleanly settle");
    }
    let report = series.finish().expect("network series should finish");

    // Assert
    assert_eq!(report.reports().len(), 3);
    eprintln!(
        "pwright network workload series: runs={}, runtime_delta_range={:?}, browser_delta_range={:?}",
        report.reports().len(),
        report.runtime_rss_delta_range(),
        report.browser_rss_sum_delta_range()
    );
}

fn network_workload_plan() -> MemoryWorkloadPlan {
    MemoryWorkloadPlan::new(
        MemoryWorkloadKind::NetworkTraffic,
        1,
        3,
        10,
        5_000,
        MemoryWorkloadLimits::new(8, 4).expect("workload limits should validate"),
    )
    .expect("workload plan should validate")
}

async fn run_network_workload(
    websocket_url: &str,
    browser_process_id: pwright_process_metrics::ProcessId,
) -> MemoryWorkloadReport {
    // Arrange one explicitly installed network driver with one reusable Page
    // lane. The publisher is intentionally queue-free: the workload isolates
    // native correlation, self-contained terminal snapshots, ordered Create
    // fencing, immediate terminal retirement, and teardown rather than
    // retaining semantic event payloads.
    let browser_object = ObjectId::new(1);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(16, 4, 4, 8, 8, 256),
        ResourceScope::extension(89, 1),
    )
    .expect("topology profile should validate");
    let page_routes = CdpPageRouteProductionProfile::from_topology(
        ResourceScope::extension(89, 2),
        topology_profile,
    );
    let wire = CapabilityWireProfile::new(
        ProtocolVersion::new(1, 0),
        FrameLimits::new(8192, 0, 0, 8192),
        FrameLimits::new(8192, 0, 0, 8192),
    );
    let network_limits = NetworkLifecycleLimits::new(4, 256, 4096, 16, 64, 16, 4096, 512, 256, 512);
    let network_profile = ChromiumNetworkEventsProductionProfile::new_with_context_limit(
        CdpNetworkEventProductionProfile::new(network_limits, ResourceScope::extension(89, 4))
            .expect("network adapter profile should validate"),
        1,
        1,
    )
    .expect("one network Page lane should validate");
    let profile = ChromiumAttachProductionProfile::new(
        CdpConnectionProductionProfile::new(ResourceScope::Connection, ConnectionConfig::default()),
        topology_profile,
        page_routes,
        RuntimeLimits::new(24, 4)
            .with_active_commands(1)
            .with_control_messages(64),
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
            ResourceScope::extension(89, 3),
            CdpNavigationLimits::new(4096, 256, 16, 16, 4096, 256),
            1,
        ),
        NavigationCodecLimits::new(8192, 16, 4096),
        wire,
    ))
    .with_network_events(network_profile)
    .with_derived_reclaiming_event_fanout()
    .expect("selected network and navigation components should derive two event cursors");
    let requirements = profile.requirements();
    let prepared = PreparedChromiumAttach::new(profile, exact_limits(requirements))
        .expect("network workload composition should prepare");
    let mut attached = prepared
        .attach(
            CdpWebSocketEndpoint::parse(websocket_url.to_owned())
                .expect("dedicated Chrome endpoint should validate"),
            browser_object,
            ConnectionEpoch::new(1),
        )
        .await
        .expect("production attach should connect");
    let continuation = CleanupQueue::new(8);
    let runtime = Arc::new(
        attached
            .new_controlled_page_runtime(NoDeadlineScheduler, continuation.clone())
            .expect("network-aware Page runtime should install"),
    );
    while runtime
        .pop_control_message()
        .expect("control queue should remain readable")
        .is_some()
    {}
    let mut network_driver = attached
        .network_event_driver(runtime.service())
        .expect("network driver should construct")
        .expect("network events should be installed");
    let network_registration = network_driver.registration_handle();
    let context_creation = attached
        .browser_new_context_service(runtime.lifecycle_handle(), continuation.clone())
        .expect("context backend should construct")
        .expect("context creation should be installed");
    let page_creation = attached
        .browser_context_new_page_service_with_network(
            runtime.as_ref(),
            network_registration.clone(),
            continuation.clone(),
        )
        .expect("page backend should construct")
        .expect("page creation should be installed");
    let (page_close, _) = attached
        .page_lifecycle_services_with_network(
            runtime.as_ref(),
            network_registration.clone(),
            continuation.clone(),
            Arc::new(ImmediatePageClosePoll),
        )
        .expect("network-aware Page lifecycle should be installed");
    let context_close = attached
        .browser_context_close_service_for_page_runtime_with_network(
            runtime.as_ref(),
            network_registration,
            continuation,
        )
        .expect("network-aware Context lifecycle should be installed");
    let activator = ChromiumPageSubscriptionActivator::new(runtime.service().clone());
    let publisher = ChromiumNetworkEventPublisher::default();
    let (control_transport, peer_transport) = MemoryFrameTransport::pair(128);
    let control_transport = Arc::new(control_transport);
    let peer_transport = Arc::new(peer_transport);
    let control_version = ProtocolVersion::new(1, 0);
    let control_protocol = WireProtocol::new(
        ControlCodec,
        VersionPolicy::exact(control_version),
        control_version,
        FrameLimits::new(1024, 0, 0, 1024),
    )
    .expect("control protocol should validate");
    let control = OrderedRuntimeControl::new(
        Arc::clone(&runtime),
        RuntimeControlDriver::new(control_protocol, control_transport),
        1,
    )
    .expect("ordered network lifecycle fence should construct");
    let probe = system_process_probe();
    let plan = network_workload_plan();
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

    macro_rules! drain_network_cursor_after_barrier {
        () => {{
            attached
                .connection()
                .send_command("Browser.getVersion", serde_json::json!({}))
                .await
                .expect("CDP ordering barrier should succeed");
            loop {
                match network_driver
                    .try_receive_once()
                    .expect("ready native event drain should remain healthy")
                {
                    ChromiumNetworkEventPoll::Empty => break,
                    ChromiumNetworkEventPoll::Ignored => {}
                    ChromiumNetworkEventPoll::Batch(batch) => {
                        let page = batch
                            .page()
                            .expect("a non-empty drained batch should identify its Page");
                        publisher
                            .publish_after_creation_fence(batch, runtime.as_ref(), &control)
                            .await
                            .expect("drained network lifecycle should publish");
                        network_driver
                            .acknowledge_batch_publication(page)
                            .expect("drained Page batch should acknowledge");
                    }
                }
            }
        }};
    }

    macro_rules! run_network_navigation {
        ($call_id:expr, $page:expr, $url:expr) => {{
            let expected_url = $url.to_owned();
            let goto_url = expected_url.clone();
            let goto_runtime = Arc::clone(&runtime);
            let goto = tokio::spawn(async move {
                goto_runtime
                    .call(Command::<Goto>::new(
                        CallId::new($call_id),
                        $page,
                        CallMetadata::default(),
                        GotoParams::new(&goto_url),
                    ))
                    .await
                    .expect("runtime dispatch should succeed")
                    .into_outcome()
                    .expect("network navigation should reach real Chrome")
            });
            let mut request = None;
            let mut saw_response = false;
            let mut terminal = false;
            while !terminal {
                let batch = tokio::time::timeout(
                    Duration::from_millis(plan.teardown_deadline_millis()),
                    network_driver.receive_once(),
                )
                .await
                .expect("network driver should observe the navigation before its deadline")
                .expect("network driver should remain healthy");
                let Some(batch) = batch else {
                    continue;
                };
                for effect in batch.events() {
                    match effect {
                        CdpNetworkEventEffect::Request(event)
                            if event.payload().url == expected_url
                                && event.payload().is_navigation_request =>
                        {
                            request = Some(event.payload().request);
                        }
                        CdpNetworkEventEffect::Response(event)
                            if request == Some(event.payload().request) =>
                        {
                            saw_response = true;
                        }
                        CdpNetworkEventEffect::RequestFinished(event)
                            if request == Some(event.payload().request.request) =>
                        {
                            terminal = true;
                        }
                        CdpNetworkEventEffect::RequestFailed(event)
                            if request == Some(event.payload().request.request) =>
                        {
                            panic!(
                                "network navigation failed: {}",
                                event.payload().failure_text
                            );
                        }
                        _ => {}
                    }
                }
                let page = batch
                    .page()
                    .expect("a non-empty network batch should identify its Page");
                publisher
                    .publish_after_creation_fence(batch, runtime.as_ref(), &control)
                    .await
                    .expect("network lifecycle should publish behind its Create fence");
                network_driver
                    .acknowledge_batch_publication(page)
                    .expect("the exact Page batch should acknowledge");
            }
            let result =
                tokio::time::timeout(Duration::from_millis(plan.teardown_deadline_millis()), goto)
                    .await
                    .expect("goto should complete before its deadline")
                    .expect("goto task should not panic");
            drain_network_cursor_after_barrier!();
            assert_eq!(result.url, expected_url);
            assert!(saw_response, "navigation should materialize a Response");
            assert_eq!(network_driver.active_request_count(), 0);
            while ConcurrentServerOutbound::send_once(&control)
                .await
                .expect("terminal network lifecycle should flush")
                .is_some()
            {}
            let peer_frames = peer_transport
                .metrics()
                .expect("peer transport metrics should remain readable")
                .incoming_depth();
            for _ in 0..peer_frames {
                peer_transport
                    .receive()
                    .await
                    .expect("peer control receive should succeed")
                    .expect("the measured control frame should exist");
            }
            assert_eq!(
                runtime
                    .control_message_metrics()
                    .expect("control metrics should remain readable")
                    .map_or(0, |metrics| metrics.current_len),
                0
            );
            assert_eq!(
                attached
                    .connection()
                    .reclaiming_event_metrics()
                    .expect("reclaiming event fanout should be installed")
                    .queued_events,
                0,
                "all native events through the terminal navigation boundary must be reclaimed"
            );
            result
        }};
    }

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
    activator
        .set_subscription(warmup_page.object_id, NETWORK_REQUEST_EVENT_TAG, true)
        .await
        .expect("warmup Page network domain should activate");
    run_network_navigation!(
        3,
        warmup_page.object_id,
        "data:text/html,<title>network-warmup</title>"
    );
    page_close
        .call(Command::<PageClose>::new(
            CallId::new(4),
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
            CallId::new(5),
            warmup_context.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("context close should be infallible")
        .into_outcome()
        .expect("warmup BrowserContext should close");
    while ConcurrentServerOutbound::send_once(&control)
        .await
        .expect("warmup teardown lifecycle should flush")
        .is_some()
    {}
    let warmup_frames = peer_transport.metrics().unwrap().incoming_depth();
    for _ in 0..warmup_frames {
        peer_transport.receive().await.unwrap().unwrap();
    }
    assert_eq!(network_driver.page_count(), 0);
    wait_for_browser_process_count_stable(
        &probe,
        browser_process_id,
        Duration::from_millis(250),
        Duration::from_millis(plan.teardown_deadline_millis()),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(plan.sample_interval_millis())).await;
    drain_network_cursor_after_barrier!();
    assert_eq!(
        attached
            .connection()
            .reclaiming_event_metrics()
            .expect("reclaiming event fanout should be installed")
            .queued_events,
        0,
        "warmup teardown must leave no unread native event payload"
    );
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
        .expect("context service should be infallible")
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
        .expect("page service should be infallible")
        .into_outcome()
        .expect("measured Page should reuse the network lane");
    activator
        .set_subscription(page.object_id, NETWORK_REQUEST_EVENT_TAG, true)
        .await
        .expect("measured Page network domain should activate");
    assert_eq!(network_driver.page_count(), 1);

    for iteration in 0..3_u64 {
        let url = format!("data:text/html,<title>network-steady-{iteration}</title>");
        run_network_navigation!(8 + iteration, page.object_id, &url);
        tokio::time::sleep(Duration::from_millis(plan.sample_interval_millis())).await;
        recorder
            .record(observe(MemoryWorkloadPhase::Steady))
            .expect("settled network observation should record");
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
        .expect("page close should be infallible")
        .into_outcome()
        .expect("measured Page should close");
    context_close
        .call(Command::<BrowserContextClose>::new(
            CallId::new(12),
            context.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("context close should be infallible")
        .into_outcome()
        .expect("measured BrowserContext should close");
    while ConcurrentServerOutbound::send_once(&control)
        .await
        .expect("measured teardown lifecycle should flush")
        .is_some()
    {}
    let teardown_frames = peer_transport.metrics().unwrap().incoming_depth();
    for _ in 0..teardown_frames {
        peer_transport.receive().await.unwrap().unwrap();
    }
    assert_eq!(network_driver.page_count(), 0);
    assert_eq!(network_driver.active_request_count(), 0);
    wait_for_browser_process_count_at_most(
        &probe,
        browser_process_id,
        warm_browser_processes,
        Duration::from_millis(plan.teardown_deadline_millis()),
    )
    .await;
    drain_network_cursor_after_barrier!();
    assert_eq!(
        attached
            .connection()
            .reclaiming_event_metrics()
            .expect("reclaiming event fanout should be installed")
            .queued_events,
        0,
        "Page and Context teardown must leave no unread native event payload"
    );
    recorder
        .record(observe(MemoryWorkloadPhase::PostTeardown))
        .expect("post-teardown should record");

    // Act: seal the correlated network observations.
    let report = recorder.finish().expect("complete workload should finish");

    // Assert every sampled network turn settled before observation, and the
    // reusable driver lane returned to its original empty installation state.
    let steady_live_objects = report
        .observations()
        .iter()
        .filter(|observation| observation.phase == MemoryWorkloadPhase::Steady)
        .map(|observation| {
            observation
                .resources
                .class(ResourceClass::LiveObject)
                .current
        })
        .collect::<Vec<_>>();
    assert!(
        report.logical_state_returned_to_baseline(),
        "network workload logical state did not settle: baseline={:?}, comparison_baseline={:?}, post_teardown={:?}",
        report.baseline(),
        report.comparison_baseline(),
        report.post_teardown()
    );
    assert!(report.browser_process_tree_returned_to_baseline());
    assert_eq!(report.baseline().objects.contexts, 0);
    assert_eq!(report.baseline().objects.pages, 0);
    assert_eq!(report.post_teardown().objects.contexts, 0);
    assert_eq!(report.post_teardown().objects.pages, 0);
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
    assert_eq!(
        steady_live_objects,
        [4, 4, 4],
        "every terminal snapshot must retire its Request/Response pair while the Page remains live"
    );
    eprintln!(
        "pwright network workload: observations={}, steady_live_objects={steady_live_objects:?}, runtime_rss={}→{}, browser_rss_sum={}→{}, browser_processes={}→{}, runtime_delta={}, browser_delta={}",
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
