use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pwright_backend_cdp::{CdpTopologyLimits, CdpTopologyProductionProfile};
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpConnectionProductionProfile, CdpSession};
use pwright_composition_chromium::{
    BrowserEventsInstallationProfile, CapabilityWireProfile, CdpPageRouteProductionProfile,
    ChromiumAttachFactory, ChromiumAttachProductionProfile, ChromiumRemoteControlProfile,
    ChromiumRemoteHandshakeProfile,
};
use pwright_composition_chromium_launch::{
    ChromiumLaunchCompositionFactory, OwnedBrowserCloseExecution,
    OwnedBrowserLifecycleInstallationProfile,
    eventful_owned_browser_lifecycle_handlers_with_report_sink,
};
use pwright_core::{
    CallId, CallMetadata, Command, ObjectId, ProtocolVersion, ResourceClass, ResourceLimits,
    ResourceRequirements, ResourceScope,
};
use pwright_process_metrics::{
    BrowserObjectCounts, MemoryObservation, MemoryWorkloadKind, MemoryWorkloadLimits,
    MemoryWorkloadPhase, MemoryWorkloadPlan, MemoryWorkloadRecorder, MemoryWorkloadSeriesLimits,
    MemoryWorkloadSeriesPlan, OwnedBrowserMemoryWorkloadReport,
    OwnedBrowserMemoryWorkloadSeriesRecorder, OwnedBrowserPostTeardownObservation, ProcessId,
    ProcessMetricsLimits, ProcessMetricsProbe,
};
use pwright_provider_cdp::{CdpConnector, CdpWebSocketEndpoint, ProductionCdpConnector};
use pwright_provider_chromium::{
    ChromiumLaunchRequest, ChromiumShutdownPolicy, ChromiumShutdownReport, discover_system_chromium,
};
use pwright_runtime::{CleanupQueue, NoDeadlineScheduler, RuntimeLimits};
use pwright_server_wire::{
    AcceptedServerHandshake, AllowUnauthenticated, ConcurrentServerBackend,
    ConcurrentServerFrameHandler, EncodedServerSupervisor, NoServerControl,
    ServerControlFrameHandler, ServerFrameDisposition, ServerSupervisorCloseReason,
    ServerSupervisorLimits,
};
use pwright_transport::{
    CapabilityRequest, CapabilityRequirement, FrameLimits, FrameTransport, NegotiationLimits,
    VersionPolicy, WireCapabilityId, WireProtocol,
};
use pwright_transport_memory::MemoryFrameTransport;

struct OwnedProfileDirectory {
    path: PathBuf,
}

impl OwnedProfileDirectory {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock should be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pwright-live-owned-chromium-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("dedicated Chromium profile should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for OwnedProfileDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
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

fn profile(config: ConnectionConfig) -> ChromiumAttachProductionProfile {
    let connection = CdpConnectionProductionProfile::new(ResourceScope::Connection, config);
    let topology = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(8, 2, 2, 4, 4, 128),
        ResourceScope::extension(22, 1),
    )
    .expect("topology profile should validate");
    let routes =
        CdpPageRouteProductionProfile::from_topology(ResourceScope::extension(22, 2), topology);
    ChromiumAttachProductionProfile::new(
        connection,
        topology,
        routes,
        RuntimeLimits::new(8, 2)
            .with_active_commands(1)
            .with_control_messages(8),
        ServerSupervisorLimits::new(1).expect("supervisor profile should validate"),
    )
    .with_page_events()
}

fn attachment_factory(config: ConnectionConfig) -> ChromiumAttachFactory {
    let composition_profile = profile(config);
    let requirements = composition_profile.requirements();
    ChromiumAttachFactory::new(
        composition_profile,
        exact_limits(requirements),
        ObjectId::new(1),
    )
    .expect("attachment factory should validate")
}

fn browser_lifecycle_profile() -> OwnedBrowserLifecycleInstallationProfile {
    let frames = FrameLimits::new(1024, 0, 0, 1024);
    OwnedBrowserLifecycleInstallationProfile::new(
        pwright_codec_browser_lifecycle::BrowserLifecycleCodecLimits::new(256),
        CapabilityWireProfile::new(ProtocolVersion::new(1, 0), frames, frames),
    )
}

fn owned_lifecycle_factory(config: ConnectionConfig) -> ChromiumLaunchCompositionFactory {
    let frames = FrameLimits::new(1024, 0, 0, 1024);
    let lifecycle = browser_lifecycle_profile();
    let composition_profile = profile(config)
        .with_remote_handshake(ChromiumRemoteHandshakeProfile::new(
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0)).unwrap(),
            frames,
            NegotiationLimits::new(4, 128),
            1024,
        ))
        .with_remote_control(ChromiumRemoteControlProfile::new(1024, 2))
        .with_browser_events(BrowserEventsInstallationProfile::new(
            CapabilityWireProfile::new(ProtocolVersion::new(1, 0), frames, frames),
        ));
    let limits = exact_limits(
        composition_profile
            .requirements()
            .combine_concurrent(lifecycle.requirements()),
    );
    let attachment = ChromiumAttachFactory::new(composition_profile, limits, ObjectId::new(1))
        .expect("owned lifecycle attachment factory should validate");
    ChromiumLaunchCompositionFactory::new(attachment).with_remote_browser_lifecycle(lifecycle)
}

async fn accepted_browser_lifecycle_session(
    client_transport: MemoryFrameTransport,
    mut supervisor: EncodedServerSupervisor<MemoryFrameTransport>,
    codec: &pwright_codec_handshake::HandshakeCodec,
    config: &pwright_transport::ServerHandshakeConfig,
    request_browser_close: bool,
) -> AcceptedServerHandshake<()> {
    let frames = FrameLimits::new(1024, 0, 0, 1024);
    let limits = NegotiationLimits::new(4, 128);
    let capability = WireCapabilityId::new("pwright.browser_lifecycle", 128).unwrap();
    let browser_events = WireCapabilityId::new("pwright.browser-events", 128).unwrap();
    let required = if request_browser_close {
        vec![
            CapabilityRequirement::new(capability, ProtocolVersion::new(1, 0)),
            CapabilityRequirement::new(browser_events, ProtocolVersion::new(1, 0)),
        ]
    } else {
        Vec::new()
    };
    let extensions = if request_browser_close {
        vec![
            pwright_codec_control::control_protocol_requirement()
                .expect("control protocol requirement should build"),
        ]
    } else {
        Vec::new()
    };
    let request = CapabilityRequest::new(required, extensions, limits).unwrap();
    let hello = pwright_transport::ClientHello::new(
        VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0)).unwrap(),
        frames,
        request,
    );
    let protocol =
        WireProtocol::new_exact_directional(*codec, ProtocolVersion::new(1, 0), frames, frames);
    client_transport
        .send(protocol.encode(&hello).unwrap())
        .await
        .unwrap();
    supervisor
        .accept_handshake(codec, config, &mut AllowUnauthenticated)
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "launches and terminates a separate installed Chromium process"]
async fn owned_launch_binds_one_budgeted_connection_and_shutdown_reaps_its_process() {
    // Arrange
    let executable = discover_system_chromium()
        .into_iter()
        .next()
        .expect("an installed Chromium-family executable is required");
    let profile_directory = OwnedProfileDirectory::new();
    let config = ConnectionConfig::default();
    let attachment = attachment_factory(config.clone());
    let factory = ChromiumLaunchCompositionFactory::new(attachment);
    let request = ChromiumLaunchRequest::new(executable, profile_directory.path(), config)
        .with_headless(true)
        .with_startup_timeout(Duration::from_secs(10));

    // Act
    let mut owned = factory
        .launch(request)
        .await
        .expect("owned Chromium should launch and bind to the composition");
    let process_id = owned
        .process_lease()
        .expect("fresh launch should retain its process authority")
        .process_id();
    let policy = ChromiumShutdownPolicy::new(
        Duration::from_secs(2),
        Duration::from_secs(5),
        Duration::from_secs(5),
    )
    .expect("shutdown policy should be bounded");
    let unpolled_shutdown = owned.shutdown(policy);
    drop(unpolled_shutdown);
    assert!(owned.process_lease().is_some());
    let targets = CdpSession::browser(std::sync::Arc::clone(owned.composition().connection()))
        .target_get_targets()
        .await
        .expect("the budgeted launched connection should reach its browser");
    let report = owned
        .shutdown(policy)
        .await
        .expect("owned Chromium shutdown should reap its exact process");

    // Assert
    assert!(process_id.is_some());
    assert!(!targets.is_empty());
    assert_eq!(report.process_id, process_id);
    assert_eq!(report.user_data_dir, profile_directory.path());
    assert!(owned.process_lease().is_none());
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn launched_connection_proof_rejects_foreign_plan_and_ledger() {
    // Arrange
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must identify the dedicated Chrome endpoint"),
    )
    .expect("dedicated Chrome endpoint should parse");
    let config = ConnectionConfig::default();
    let same_plan_source = attachment_factory(config.clone()).prepare().unwrap();
    let same_plan_target = attachment_factory(config.clone()).prepare().unwrap();
    let same_plan_connector = ProductionCdpConnector::new(
        same_plan_source.profile().connection().clone(),
        same_plan_source.resources(),
    )
    .expect("source connector should fit its own plan");
    let foreign_ledger_connection = same_plan_connector
        .connect(endpoint.clone(), config.clone())
        .await
        .expect("source connector should reach dedicated Chrome");

    let mut drifted = config;
    drifted.pending_command_capacity = drifted.pending_command_capacity.saturating_add(1);
    let foreign_plan_source = attachment_factory(drifted.clone()).prepare().unwrap();
    let foreign_plan_target = attachment_factory(ConnectionConfig::default())
        .prepare()
        .unwrap();
    let foreign_plan_connector = ProductionCdpConnector::new(
        foreign_plan_source.profile().connection().clone(),
        foreign_plan_source.resources(),
    )
    .expect("foreign-plan connector should fit its own plan");
    let foreign_plan_connection = foreign_plan_connector
        .connect(endpoint, drifted)
        .await
        .expect("foreign-plan connector should reach dedicated Chrome");

    // Act
    let ledger_result = same_plan_target.bind_connection(
        foreign_ledger_connection,
        ObjectId::new(1),
        pwright_backend_cdp::ConnectionEpoch::new(1),
    );
    let plan_result = foreign_plan_target.bind_connection(
        foreign_plan_connection,
        ObjectId::new(1),
        pwright_backend_cdp::ConnectionEpoch::new(1),
    );

    // Assert
    assert!(matches!(
        ledger_result,
        Err(pwright_composition_chromium::ChromiumAttachError::ConnectionLedgerMismatch)
    ));
    assert!(matches!(
        plan_result,
        Err(pwright_composition_chromium::ChromiumAttachError::ConnectionPlanMismatch { .. })
    ));
}

#[tokio::test]
#[ignore = "launches and remotely closes a separate installed Chromium process"]
async fn remote_owned_browser_close_consumes_the_real_process_and_runtime_tree() {
    // Arrange
    let executable = discover_system_chromium()
        .into_iter()
        .next()
        .expect("an installed Chromium-family executable is required");
    let profile_directory = OwnedProfileDirectory::new();
    let config = ConnectionConfig::default();
    let factory = owned_lifecycle_factory(config.clone());
    let request = ChromiumLaunchRequest::new(executable, profile_directory.path(), config)
        .with_headless(true)
        .with_startup_timeout(Duration::from_secs(10));
    let mut owned = factory.launch(request).await.unwrap();
    let process_id = owned.process_lease().unwrap().process_id().unwrap();
    let (codec, handshake) = owned.remote_handshake_parts().unwrap();
    let (unprivileged_client, unprivileged_server) = MemoryFrameTransport::pair(2);
    let unprivileged_supervisor = owned
        .composition()
        .new_supervisor(unprivileged_server)
        .unwrap();
    let unprivileged = accepted_browser_lifecycle_session(
        unprivileged_client,
        unprivileged_supervisor,
        &codec,
        &handshake,
        false,
    )
    .await;
    let (accepted_client, accepted_server) = MemoryFrameTransport::pair(2);
    let accepted_supervisor = owned.composition().new_supervisor(accepted_server).unwrap();
    let accepted = accepted_browser_lifecycle_session(
        accepted_client,
        accepted_supervisor,
        &codec,
        &handshake,
        true,
    )
    .await;
    let (unbound_client, unbound_server) = MemoryFrameTransport::pair(2);
    let unbound_supervisor = EncodedServerSupervisor::new_handshake_required(
        unbound_server,
        ServerSupervisorLimits::new(1).unwrap(),
    );
    let unbound = accepted_browser_lifecycle_session(
        unbound_client,
        unbound_supervisor,
        &codec,
        &handshake,
        true,
    )
    .await;
    let lifecycle_profile = factory.browser_lifecycle_profile().unwrap();
    let foreign_profile = factory
        .attachment()
        .profile()
        .clone()
        .with_extension_requirements(lifecycle_profile.requirements());
    let foreign_attachment = ChromiumAttachFactory::new(
        foreign_profile,
        factory.attachment().limits(),
        factory.attachment().browser_object_id(),
    )
    .unwrap();
    let foreign_prepared = foreign_attachment.prepare().unwrap();
    let (foreign_client, foreign_server) = MemoryFrameTransport::pair(2);
    let foreign_supervisor = foreign_prepared.new_supervisor(foreign_server).unwrap();
    let foreign = accepted_browser_lifecycle_session(
        foreign_client,
        foreign_supervisor,
        &codec,
        &handshake,
        true,
    )
    .await;
    let runtime = std::sync::Arc::new(
        owned
            .composition_mut()
            .new_controlled_runtime((), (), NoDeadlineScheduler, CleanupQueue::new(1))
            .unwrap(),
    );
    runtime
        .register_object(
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            pwright_browser_model::BROWSER_CONTEXT_OBJECT_KIND.tag,
        )
        .unwrap();
    runtime
        .register_object(
            ObjectId::new(3),
            Some(ObjectId::new(2)),
            pwright_browser_model::PAGE_OBJECT_KIND.tag,
        )
        .unwrap();
    for _ in 0..3 {
        runtime.pop_control_message().unwrap().unwrap();
    }
    let lifecycle = runtime.lifecycle_handle();
    let browser_protocol = owned
        .browser_lifecycle_protocol(&accepted)
        .unwrap()
        .expect("Browser lifecycle was negotiated");
    let (server_transport, client_transport) = MemoryFrameTransport::pair(8);
    let server_transport = std::sync::Arc::new(server_transport);
    let control = owned
        .composition()
        .remote_control_outbound(
            std::sync::Arc::clone(&runtime),
            std::sync::Arc::clone(&server_transport),
            &accepted,
        )
        .unwrap()
        .expect("ordered control was negotiated");
    let (mut native_control, mut liveness, disconnected) = owned
        .composition()
        .remote_browser_disconnect_parts(
            NoServerControl,
            lifecycle.clone(),
            &control,
            std::sync::Arc::clone(&server_transport),
            &accepted,
        )
        .unwrap()
        .expect("Browser disconnected was negotiated");
    assert!(!liveness.native_disconnect_signal().is_closed());
    let rejection = match owned.into_browser_close_grant(&unprivileged) {
        Ok(_) => panic!("a session without Browser lifecycle received process authority"),
        Err(rejection) => rejection,
    };
    assert_eq!(
        rejection.error(),
        pwright_composition_chromium_launch::OwnedBrowserCloseGrantError::CapabilityNotNegotiated
    );
    let (_, owned_after_rejection) = rejection.into_parts();
    assert_eq!(
        owned_after_rejection.process_lease().unwrap().process_id(),
        Some(process_id)
    );
    let unbound_rejection = match owned_after_rejection.into_browser_close_grant(&unbound) {
        Ok(_) => panic!("an unbound accepted hello received process authority"),
        Err(rejection) => rejection,
    };
    assert_eq!(
        unbound_rejection.error(),
        pwright_composition_chromium_launch::OwnedBrowserCloseGrantError::SessionResourcesUnavailable
    );
    let (_, owned_after_unbound_rejection) = unbound_rejection.into_parts();
    assert_eq!(
        owned_after_unbound_rejection
            .process_lease()
            .unwrap()
            .process_id(),
        Some(process_id)
    );
    let foreign_rejection = match owned_after_unbound_rejection.into_browser_close_grant(&foreign) {
        Ok(_) => panic!("a different production session received process authority"),
        Err(rejection) => rejection,
    };
    assert_eq!(
        foreign_rejection.error(),
        pwright_composition_chromium_launch::OwnedBrowserCloseGrantError::SessionResourceBudgetMismatch
    );
    let (_, owned_after_foreign_rejection) = foreign_rejection.into_parts();
    assert_eq!(
        owned_after_foreign_rejection
            .process_lease()
            .unwrap()
            .process_id(),
        Some(process_id)
    );
    let grant = owned_after_foreign_rejection
        .into_browser_close_grant(&accepted)
        .unwrap();
    let command = browser_protocol
        .encode(
            &Command::<pwright_cap_browser_lifecycle::BrowserClose>::new(
                CallId::new(1),
                ObjectId::new(1),
                CallMetadata::default(),
                pwright_cap_browser_lifecycle::BrowserCloseOptions::default(),
            ),
        )
        .unwrap();
    let shutdown_report = std::sync::Arc::new(std::sync::Mutex::new(None));
    let report_slot = std::sync::Arc::clone(&shutdown_report);
    let report_sink = move |report: ChromiumShutdownReport| {
        *report_slot.lock().expect("shutdown report should lock") = Some(report);
    };
    let handler = eventful_owned_browser_lifecycle_handlers_with_report_sink(
        std::sync::Arc::new(browser_protocol),
        grant,
        lifecycle,
        ChromiumShutdownPolicy::new(
            Duration::from_secs(2),
            Duration::from_secs(5),
            Duration::from_secs(5),
        )
        .unwrap(),
        OwnedBrowserCloseExecution::new(report_sink, CleanupQueue::new(1)),
        control,
        disconnected,
    );

    // Act
    let disposition = handler
        .handle_concurrently(command, std::sync::Arc::clone(&server_transport))
        .await
        .unwrap();
    let mut frames = Vec::new();
    for _ in 0..5 {
        frames.push(client_transport.receive().await.unwrap().unwrap());
    }
    let liveness_result = std::future::poll_fn(|context| liveness.poll_drive_once(context)).await;
    let cleanup = native_control
        .handle_connection_terminated(ServerSupervisorCloseReason::BackendEventFailed)
        .await;

    // Assert
    assert!(matches!(disposition, ServerFrameDisposition::Handled(_)));
    assert_eq!(
        frames.iter().map(|frame| frame.route()).collect::<Vec<_>>(),
        vec![
            pwright_transport::FrameRoute::Control,
            pwright_transport::FrameRoute::Control,
            pwright_transport::FrameRoute::Event {
                source: ObjectId::new(1),
                event: pwright_cap_browser_events::BROWSER_DISCONNECTED_EVENT_TAG,
            },
            pwright_transport::FrameRoute::Control,
            pwright_transport::FrameRoute::Result {
                call_id: CallId::new(1),
            },
        ]
    );
    assert!(liveness_result.is_err());
    assert!(liveness.native_disconnect_signal().is_closed());
    assert!(cleanup.is_ok());
    assert_eq!(client_transport.metrics().unwrap().incoming_depth(), 0);
    assert_eq!(runtime.live_object_count(), Ok(0));
    let report = shutdown_report
        .lock()
        .expect("shutdown report should lock")
        .take()
        .expect("the composition sink should receive the provider report");
    assert_eq!(report.process_id, Some(process_id));
    assert_eq!(report.user_data_dir, profile_directory.path());
    let process_probe = std::process::Command::new("kill")
        .args(["-0", &process_id.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("process liveness probe should execute");
    assert!(!process_probe.success());
}

#[tokio::test]
#[ignore = "launches and measures teardown of a separate installed Chromium process"]
async fn owned_browser_teardown_records_runtime_and_proves_the_browser_pid_exited() {
    // Arrange an exact, pre-reserved three-run series.
    let series_plan = MemoryWorkloadSeriesPlan::new(
        owned_teardown_plan(),
        3,
        MemoryWorkloadSeriesLimits::new(3).expect("series limits should validate"),
    )
    .expect("owned teardown series should validate");
    let mut series = OwnedBrowserMemoryWorkloadSeriesRecorder::new(series_plan)
        .expect("owned teardown series should allocate");

    // Act
    for run in 0..series_plan.runs() {
        let report = run_owned_browser_teardown().await;
        eprintln!(
            "pwright owned teardown run {run}: pre_teardown_browser_rss_sum={}, runtime_delta={}",
            report.pre_teardown_browser_rss_sum(),
            report.runtime_rss_delta()
        );
        series
            .record(report)
            .expect("every owned teardown should release logical state");
    }
    let report = series
        .finish()
        .expect("owned teardown series should finish");

    // Assert
    assert_eq!(report.reports().len(), 3);
    eprintln!(
        "pwright owned teardown series: runtime_delta_range={:?}, pre_teardown_browser_rss_sum_range={:?}",
        report.runtime_rss_delta_range(),
        report.pre_teardown_browser_rss_sum_range()
    );
}

fn owned_teardown_plan() -> MemoryWorkloadPlan {
    MemoryWorkloadPlan::new(
        MemoryWorkloadKind::Teardown,
        1,
        3,
        25,
        15_000,
        MemoryWorkloadLimits::new(8, 4).expect("workload limits should validate"),
    )
    .expect("owned teardown plan should validate")
}

async fn run_owned_browser_teardown() -> OwnedBrowserMemoryWorkloadReport {
    // Arrange an owned browser and a teardown plan whose final observation is
    // allowed to contain an exit proof instead of fabricated browser RSS.
    let executable = discover_system_chromium()
        .into_iter()
        .next()
        .expect("an installed Chromium-family executable is required");
    let profile_directory = OwnedProfileDirectory::new();
    let profile_path = profile_directory.path().to_path_buf();
    let config = ConnectionConfig::default();
    let factory = ChromiumLaunchCompositionFactory::new(attachment_factory(config.clone()));
    let request = ChromiumLaunchRequest::new(executable, profile_directory.path(), config)
        .with_headless(true)
        .with_startup_timeout(Duration::from_secs(10));
    let mut owned = factory
        .launch(request)
        .await
        .expect("Chromium should launch");
    let browser_process_id = ProcessId::new(
        owned
            .process_lease()
            .and_then(|lease| lease.process_id())
            .expect("owned Chromium should expose its process ID"),
    )
    .expect("owned Chromium process ID should be non-zero");
    let resources = owned.composition().resources().clone();
    let probe = ProcessMetricsProbe::system(
        ProcessMetricsLimits::new(8 * 1024 * 1024, 16_384, 64 * 1024)
            .expect("process limits should validate"),
    );
    let plan = owned_teardown_plan();
    let mut recorder = MemoryWorkloadRecorder::new(plan).expect("recorder should allocate");
    let started = std::time::Instant::now();
    let observe = |phase| {
        let topology = owned
            .composition()
            .topology()
            .expect("owned topology should remain readable")
            .metrics();
        MemoryObservation {
            phase,
            monotonic_millis: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            process: probe
                .sample_current_runtime(browser_process_id)
                .expect("owned browser and runtime roots should be sampleable"),
            resources: resources
                .metrics()
                .expect("owned resources should remain readable"),
            objects: BrowserObjectCounts {
                contexts: topology.live_contexts,
                pages: topology.live_pages,
                frames: topology.live_frames,
                ..BrowserObjectCounts::default()
            },
        }
    };
    recorder
        .record(observe(MemoryWorkloadPhase::Baseline))
        .expect("baseline should record");
    tokio::time::sleep(Duration::from_millis(plan.sample_interval_millis())).await;
    recorder
        .record(observe(MemoryWorkloadPhase::Warmup))
        .expect("warmup should record");
    for _ in 0..3 {
        tokio::time::sleep(Duration::from_millis(plan.sample_interval_millis())).await;
        recorder
            .record(observe(MemoryWorkloadPhase::Steady))
            .expect("steady owned-browser observation should record");
    }
    recorder
        .record(observe(MemoryWorkloadPhase::PreTeardown))
        .expect("pre-teardown should record");

    // Act: use the provider's bounded owned shutdown, drop every composition
    // owner, then obtain runtime metrics and PID absence from one snapshot.
    let shutdown = owned
        .shutdown(
            ChromiumShutdownPolicy::new(
                Duration::from_secs(2),
                Duration::from_secs(5),
                Duration::from_secs(5),
            )
            .expect("shutdown policy should validate"),
        )
        .await
        .expect("owned Chromium should terminate");
    drop(owned);
    let exit = probe
        .sample_current_runtime_after_exit(browser_process_id)
        .expect("post-teardown snapshot should prove the browser PID exited");
    let report = recorder
        .finish_owned_browser(OwnedBrowserPostTeardownObservation {
            monotonic_millis: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            process: exit,
            resources: resources
                .metrics()
                .expect("post-teardown resources should be readable"),
            objects: BrowserObjectCounts::default(),
        })
        .expect("owned-browser teardown report should finish");

    // Assert
    assert_eq!(shutdown.process_id, Some(browser_process_id.get()));
    assert_eq!(shutdown.user_data_dir, profile_path);
    assert_eq!(report.observations().len(), 6);
    assert_eq!(
        report.post_teardown().process.exited_process_id(),
        browser_process_id
    );
    assert!(report.logical_state_returned_to_baseline());
    assert_eq!(
        report.post_teardown().resources.active_leases,
        0,
        "dropping the terminated composition should release every lease"
    );
    assert!(profile_directory.path().is_dir());
    eprintln!(
        "pwright owned teardown: pid={}, method={:?}, running_observations={}, pre_teardown_browser_rss_sum={}, runtime_delta={}",
        browser_process_id.get(),
        shutdown.method,
        report.observations().len(),
        report.pre_teardown_browser_rss_sum(),
        report.runtime_rss_delta()
    );
    drop(profile_directory);
    assert!(
        !profile_path.exists(),
        "test-owned Chromium profile should be removed after the process exits"
    );
    report
}
