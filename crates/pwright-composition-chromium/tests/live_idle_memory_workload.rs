use std::time::{Duration, Instant};

mod support;

use pwright_backend_cdp::{CdpTopologyLimits, CdpTopologyMetrics, CdpTopologyProductionProfile};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cdp::CdpConnectionProductionProfile;
use pwright_cdp::connection::ConnectionConfig;
use pwright_codec_context_lifecycle::ContextLifecycleCodecLimits;
use pwright_composition_chromium::{
    CapabilityWireProfile, CdpPageRouteProductionProfile, ChromiumAttachProductionProfile,
    ContextLifecycleInstallationProfile, PreparedChromiumAttach,
};
use pwright_core::{ObjectId, ProtocolVersion, ResourceClass, ResourceScope};
use pwright_process_metrics::{
    MemoryWorkloadKind, MemoryWorkloadLimits, MemoryWorkloadPhase, MemoryWorkloadPlan,
    MemoryWorkloadRecorder,
};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use pwright_runtime::{CleanupQueue, NoDeadlineScheduler, RuntimeLimits};
use pwright_server_wire::ServerSupervisorLimits;
use pwright_transport::FrameLimits;
use support::{
    browser_process_id_from_env, correlated_observation, exact_limits, system_process_probe,
    wait_for_browser_process_count_at_most, wait_for_browser_process_count_stable,
};

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process, PWRIGHT_CDP_WS_URL, and PWRIGHT_BROWSER_PID"]
async fn idle_external_attachment_releases_pwright_state_without_closing_chrome() {
    // Arrange the smallest valid production profile around an external browser.
    let websocket_url = std::env::var("PWRIGHT_CDP_WS_URL")
        .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile");
    let browser_process_id = browser_process_id_from_env();
    let browser_object = ObjectId::new(1);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(4, 1, 1, 1, 1, 256),
        ResourceScope::extension(89, 1),
    )
    .expect("topology profile should validate");
    let page_routes = CdpPageRouteProductionProfile::from_topology(
        ResourceScope::extension(89, 2),
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
        RuntimeLimits::new(4, 1)
            .with_active_commands(1)
            .with_control_messages(4),
        ServerSupervisorLimits::new(1).expect("supervisor limits should validate"),
    )
    .with_context_lifecycle(ContextLifecycleInstallationProfile::new(
        ContextLifecycleCodecLimits::new(1024),
        wire,
    ));
    let requirements = profile.requirements();
    let prepared = PreparedChromiumAttach::new(profile, exact_limits(requirements))
        .expect("idle composition should prepare");
    let resources = prepared.resources().clone();
    let probe = system_process_probe();
    let plan = MemoryWorkloadPlan::new(
        MemoryWorkloadKind::Idle,
        1,
        3,
        25,
        5_000,
        MemoryWorkloadLimits::new(8, 4).expect("workload limits should validate"),
    )
    .expect("idle workload plan should validate");
    let mut recorder = MemoryWorkloadRecorder::new(plan).expect("recorder should allocate");
    let started = Instant::now();
    let baseline = correlated_observation(
        MemoryWorkloadPhase::Baseline,
        &started,
        &probe,
        browser_process_id,
        resources
            .metrics()
            .expect("pre-attach resources should be readable"),
        CdpTopologyMetrics::default(),
        0,
        0,
    );
    let cold_browser_processes = baseline.process.browser.total.processes;
    recorder
        .record(baseline)
        .expect("cold baseline should record");
    let mut attached = prepared
        .attach(
            CdpWebSocketEndpoint::parse(websocket_url.clone())
                .expect("dedicated Chrome endpoint should validate"),
            browser_object,
            pwright_backend_cdp::ConnectionEpoch::new(1),
        )
        .await
        .expect("production attach should connect");
    let runtime = attached
        .new_controlled_runtime((), (), NoDeadlineScheduler, CleanupQueue::new(1))
        .expect("idle controlled runtime should install");
    while runtime
        .pop_control_message()
        .expect("control queue should remain readable")
        .is_some()
    {}
    let observe_idle = |phase| {
        correlated_observation(
            phase,
            &started,
            &probe,
            browser_process_id,
            resources
                .metrics()
                .expect("idle resources should remain readable"),
            attached
                .topology()
                .expect("idle topology should remain readable")
                .metrics(),
            runtime
                .pending_call_count()
                .expect("idle runtime should remain readable"),
            runtime
                .control_message_metrics()
                .expect("idle control metrics should remain readable")
                .map_or(0, |metrics| metrics.current_len),
        )
    };
    wait_for_browser_process_count_stable(
        &probe,
        browser_process_id,
        Duration::from_millis(250),
        Duration::from_millis(plan.teardown_deadline_millis()),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(plan.sample_interval_millis())).await;
    let warmup = observe_idle(MemoryWorkloadPhase::Warmup);
    let warm_browser_processes = warmup.process.browser.total.processes;
    recorder
        .record(warmup)
        .expect("warm idle observation should record");
    for _ in 0..3 {
        tokio::time::sleep(Duration::from_millis(plan.sample_interval_millis())).await;
        recorder
            .record(observe_idle(MemoryWorkloadPhase::Steady))
            .expect("steady idle observation should record");
    }
    recorder
        .record(observe_idle(MemoryWorkloadPhase::PreTeardown))
        .expect("pre-detach observation should record");
    let closed = attached.connection().closed();
    let weak_connection = std::sync::Arc::downgrade(attached.connection());

    // Act: explicitly close runtime state, then release every CDP connection owner.
    let close = runtime
        .close_transport()
        .expect("idle runtime transport should close");
    drop(runtime);
    drop(attached);
    tokio::time::timeout(Duration::from_secs(2), closed)
        .await
        .expect("dropping every connection owner should close the CDP WebSocket");
    wait_for_browser_process_count_at_most(
        &probe,
        browser_process_id,
        warm_browser_processes.max(cold_browser_processes),
        Duration::from_millis(4_000),
    )
    .await;
    recorder
        .record(correlated_observation(
            MemoryWorkloadPhase::PostTeardown,
            &started,
            &probe,
            browser_process_id,
            resources
                .metrics()
                .expect("post-detach resources should be readable"),
            CdpTopologyMetrics::default(),
            0,
            0,
        ))
        .expect("post-detach observation should record");
    let report = recorder
        .finish()
        .expect("complete idle workload should finish");

    // Assert Pwright released the session while a fresh attach still reaches Chrome.
    assert!(!close.already_closed());
    assert_eq!(close.pending_calls(), 0);
    assert_eq!(close.invalidated_objects().len(), 1);
    assert!(weak_connection.upgrade().is_none());
    assert!(report.logical_state_returned_to_baseline());
    assert!(report.browser_process_tree_returned_to_baseline());
    assert_eq!(
        report
            .baseline()
            .resources
            .class(ResourceClass::LiveObject)
            .current,
        0
    );
    assert_eq!(
        report
            .comparison_baseline()
            .resources
            .class(ResourceClass::LiveObject)
            .current,
        1
    );
    assert_eq!(
        report
            .post_teardown()
            .resources
            .class(ResourceClass::LiveObject)
            .current,
        0
    );
    let reattached = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(
            CdpWebSocketEndpoint::parse(websocket_url)
                .expect("dedicated Chrome endpoint should remain valid"),
            ConnectionConfig::default(),
        ))
        .await
        .expect("external Chrome should accept a fresh attachment");
    reattached
        .connection()
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("idle Pwright detach must not terminate external Chrome");
    eprintln!(
        "pwright idle workload: observations={}, invalidated_objects={}, runtime_delta={}, browser_delta={}",
        report.observations().len(),
        close.invalidated_objects().len(),
        report.runtime_rss_delta(),
        report.browser_rss_sum_delta()
    );
}
