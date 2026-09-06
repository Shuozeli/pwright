use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pwright_backend_cdp::CdpNavigationService;
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cap_navigation::{Goto, GotoParams, NavigationErrorDetails};
use pwright_cap_tracing::{
    TraceStart, TraceStartChunk, TraceStartChunkOptions, TraceStartOptions, TraceStop,
    TraceStopChunk, TraceStopChunkMode, TraceStopChunkOptions,
};
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpClient, CdpSession};
use pwright_core::{
    CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId, OperationError,
    ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope, Service, Stack,
};
use pwright_plugin_tracing_navigation::{FixedNavigationTraceRoute, NavigationTracingLayer};
use pwright_process_metrics::{
    BrowserObjectCounts, MemoryObservation, MemoryWorkloadKind, MemoryWorkloadLimits,
    MemoryWorkloadPhase, MemoryWorkloadPlan, MemoryWorkloadRecorder, ProcessId,
    ProcessMetricsLimits, ProcessMetricsProbe,
};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use pwright_trace_model::{
    TraceChunk, TraceEntryKind, TraceLimits, TraceRecorderProductionProfile,
};
use pwright_tracing_service::{TraceClock, TraceSink, TracingService};

struct MonotonicClock(Instant);

impl TraceClock for MonotonicClock {
    fn monotonic_micros(&self) -> u64 {
        u64::try_from(self.0.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}

type RecordedEntries = Arc<Mutex<Vec<(TraceEntryKind, Vec<u8>)>>>;

#[derive(Clone, Default)]
struct RecordingSink {
    entries: RecordedEntries,
}

#[derive(Clone, Default)]
struct DiscardingCountingSink {
    chunks: Arc<AtomicUsize>,
    entries: Arc<AtomicUsize>,
    last_call_id: Arc<AtomicU64>,
}

impl TraceSink for DiscardingCountingSink {
    type Error = Infallible;

    fn write(&self, _: ObjectId, chunk: TraceChunk) -> Result<(), Self::Error> {
        self.chunks.fetch_add(1, Ordering::Relaxed);
        self.entries
            .fetch_add(chunk.entries().len(), Ordering::Relaxed);
        if let Some(entry) = chunk.entries().last()
            && let Ok(bytes) = <[u8; 8]>::try_from(&entry.payload()[1..9])
        {
            self.last_call_id
                .store(u64::from_be_bytes(bytes), Ordering::Relaxed);
        }
        Ok(())
    }

    fn abort_context(&self, _: ObjectId) -> Result<(), Self::Error> {
        Ok(())
    }

    fn abort_all(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl TraceSink for RecordingSink {
    type Error = Infallible;

    fn write(&self, _: ObjectId, chunk: TraceChunk) -> Result<(), Self::Error> {
        self.entries.lock().unwrap().extend(
            chunk
                .entries()
                .iter()
                .map(|entry| (entry.kind(), entry.payload().to_vec())),
        );
        Ok(())
    }

    fn abort_context(&self, _: ObjectId) -> Result<(), Self::Error> {
        Ok(())
    }

    fn abort_all(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct CdpGotoCommandService {
    page_id: ObjectId,
    navigation: CdpNavigationService,
}

impl Service<Command<Goto>> for CdpGotoCommandService {
    type Response = CommandResult<Goto>;
    type Error = Infallible;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, command: Command<Goto>) -> Self::Future<'_> {
        let (call_id, page_id, _, params) = command.into_parts();
        Box::pin(async move {
            if page_id != self.page_id {
                return Ok(CommandResult::new(
                    call_id,
                    Err(OperationError::new(
                        FailureKind::TargetClosed,
                        "unexpected Page target",
                        Some(NavigationErrorDetails::Backend(
                            "the CDP navigation service is bound to another Page".to_owned(),
                        )),
                    )),
                ));
            }
            Ok(CommandResult::new(
                call_id,
                self.navigation.call(params).await,
            ))
        })
    }
}

fn command<Spec: pwright_core::CommandSpec>(
    call_id: u64,
    object_id: ObjectId,
    params: Spec::Params,
) -> Command<Spec> {
    Command::new(
        CallId::new(call_id),
        object_id,
        CallMetadata::default(),
        params,
    )
}

async fn dispose_context_and_verify_target(
    browser: &CdpSession,
    browser_context_id: &str,
    target_id: &str,
) {
    browser
        .target_dispose_browser_context(browser_context_id)
        .await
        .expect("owned browser context should dispose");
    for _ in 0..20 {
        let targets = browser
            .target_get_targets()
            .await
            .expect("target list should remain available");
        if !targets.iter().any(|target| target.target_id == target_id) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("temporary target {target_id} remained live after context disposal");
}

fn browser_process_id_from_env() -> ProcessId {
    ProcessId::new(
        std::env::var("PWRIGHT_BROWSER_PID")
            .expect("PWRIGHT_BROWSER_PID must identify the dedicated Chrome parent")
            .parse()
            .expect("PWRIGHT_BROWSER_PID should be an integer"),
    )
    .expect("PWRIGHT_BROWSER_PID should be non-zero")
}

fn process_probe() -> ProcessMetricsProbe {
    ProcessMetricsProbe::system(
        ProcessMetricsLimits::new(8 * 1024 * 1024, 16_384, 64 * 1024)
            .expect("process limits should validate"),
    )
}

fn observation(
    phase: MemoryWorkloadPhase,
    started: &Instant,
    probe: &ProcessMetricsProbe,
    browser_process_id: ProcessId,
    resources: &ProductionResourceLedger,
) -> MemoryObservation {
    MemoryObservation {
        phase,
        monotonic_millis: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        process: probe
            .sample_current_runtime(browser_process_id)
            .expect("process roots should remain sampleable"),
        resources: resources
            .metrics()
            .expect("trace resource metrics should remain readable"),
        objects: BrowserObjectCounts {
            contexts: 1,
            pages: 1,
            frames: 1,
            ..BrowserObjectCounts::default()
        },
    }
}

async fn wait_for_browser_process_count_at_most(
    probe: &ProcessMetricsProbe,
    browser_process_id: ProcessId,
    maximum: usize,
) {
    let started = Instant::now();
    loop {
        let current = probe
            .sample_current_runtime(browser_process_id)
            .expect("process roots should remain sampleable");
        if current.browser.total.processes <= maximum {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "browser process tree retained {} processes; expected at most {maximum}",
            current.browser.total.processes
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn optional_navigation_plugin_records_one_real_chromium_goto() {
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
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    let context_id = ObjectId::new(2);
    let page_id = ObjectId::new(3);
    let profile = TraceRecorderProductionProfile::new(
        TraceLimits::new(32, 4, 64, 256, 1),
        ResourceScope::Object(context_id),
    )
    .unwrap();
    let requirements = profile.requirements();
    let trace_bytes = requirements.class(ResourceClass::TraceBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
            ResourceClass::TraceBytes,
            trace_bytes.global,
            trace_bytes.per_scope,
        ),
        requirements,
    )
    .unwrap();
    let sink = RecordingSink::default();
    let tracing =
        Arc::new(TracingService::new(MonotonicClock(Instant::now()), sink.clone(), 1).unwrap());
    tracing
        .register_context(context_id, profile.create(&resources).unwrap())
        .unwrap();
    Service::<Command<TraceStart>>::call(
        tracing.as_ref(),
        command(
            1,
            context_id,
            TraceStartOptions::new(Some("live".to_owned())),
        ),
    )
    .await
    .unwrap()
    .into_outcome()
    .unwrap();
    Service::<Command<TraceStartChunk>>::call(
        tracing.as_ref(),
        command(
            2,
            context_id,
            TraceStartChunkOptions::new(Some("chromium".to_owned()), None),
        ),
    )
    .await
    .unwrap()
    .into_outcome()
    .unwrap();
    let service = Stack::new(CdpGotoCommandService {
        page_id,
        navigation: CdpNavigationService::new(page),
    })
    .layer(NavigationTracingLayer::new(
        Arc::clone(&tracing),
        Arc::new(FixedNavigationTraceRoute::new(page_id, context_id)),
    ));

    // Act
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        service.call(command::<Goto>(
            42,
            page_id,
            GotoParams::new("data:text/html,<title>pwright trace proof</title>"),
        )),
    )
    .await
    .expect("real Chromium goto should finish before the deadline")
    .unwrap()
    .into_outcome()
    .expect("real Chromium goto should succeed");
    let summary = Service::<Command<TraceStopChunk>>::call(
        tracing.as_ref(),
        command(
            3,
            context_id,
            TraceStopChunkOptions::new(TraceStopChunkMode::Sink),
        ),
    )
    .await
    .unwrap()
    .into_outcome()
    .unwrap();
    Service::<Command<TraceStop>>::call(tracing.as_ref(), command(4, context_id, ()))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();

    // Assert
    assert!(result.url.starts_with("data:text/html,"));
    assert_eq!(summary.entry_count(), 2);
    {
        let entries = sink.entries.lock().unwrap();
        assert_eq!(entries[0].0, TraceEntryKind::ApiCallBefore);
        assert_eq!(entries[1].0, TraceEntryKind::ApiCallAfter);
        assert_eq!(entries[0].1, entries[1].1);
        assert_eq!(
            u64::from_be_bytes(entries[0].1[1..9].try_into().unwrap()),
            42
        );
    }
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::TraceBytes)
            .current,
        0
    );
    dispose_context_and_verify_target(&browser, &browser_context_id, &target_id).await;
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("external browser should remain alive");
}

#[tokio::test]
#[ignore = "requires dedicated Chrome, PWRIGHT_CDP_WS_URL, and PWRIGHT_BROWSER_PID"]
async fn repeated_trace_chunks_return_to_their_warm_memory_baseline() {
    // Arrange one external Chromium Page and a trace-only retained-state plan.
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
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    let context_id = ObjectId::new(2);
    let page_id = ObjectId::new(3);
    let profile = TraceRecorderProductionProfile::new(
        TraceLimits::new(32, 2, 64, 128, 1),
        ResourceScope::Object(context_id),
    )
    .unwrap();
    let requirements = profile.requirements();
    let trace_bytes = requirements.class(ResourceClass::TraceBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
            ResourceClass::TraceBytes,
            trace_bytes.global,
            trace_bytes.per_scope,
        ),
        requirements,
    )
    .unwrap();
    let sink = DiscardingCountingSink::default();
    let tracing =
        Arc::new(TracingService::new(MonotonicClock(Instant::now()), sink.clone(), 1).unwrap());
    tracing
        .register_context(context_id, profile.create(&resources).unwrap())
        .unwrap();
    let service = Stack::new(CdpGotoCommandService {
        page_id,
        navigation: CdpNavigationService::new(page),
    })
    .layer(NavigationTracingLayer::new(
        Arc::clone(&tracing),
        Arc::new(FixedNavigationTraceRoute::new(page_id, context_id)),
    ));
    let plan = MemoryWorkloadPlan::new(
        MemoryWorkloadKind::Tracing,
        1,
        3,
        10,
        5_000,
        MemoryWorkloadLimits::new(8, 4).unwrap(),
    )
    .unwrap();
    let mut workload = MemoryWorkloadRecorder::new(plan).unwrap();
    let probe = process_probe();
    let browser_process_id = browser_process_id_from_env();
    let started = Instant::now();
    workload
        .record(observation(
            MemoryWorkloadPhase::Baseline,
            &started,
            &probe,
            browser_process_id,
            &resources,
        ))
        .unwrap();
    Service::<Command<TraceStart>>::call(
        tracing.as_ref(),
        command(1, context_id, TraceStartOptions::new(None)),
    )
    .await
    .unwrap()
    .into_outcome()
    .unwrap();

    // Act: one cleaned warmup chunk followed by three measured chunks.
    let mut warm_processes = None;
    for iteration in 0..4_u64 {
        Service::<Command<TraceStartChunk>>::call(
            tracing.as_ref(),
            command(
                2 + iteration * 3,
                context_id,
                TraceStartChunkOptions::new(None, None),
            ),
        )
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
        let call_id = 3 + iteration * 3;
        let url = format!("data:text/html,<title>trace-memory-{iteration}</title>");
        tokio::time::timeout(
            Duration::from_secs(10),
            service.call(command::<Goto>(call_id, page_id, GotoParams::new(&url))),
        )
        .await
        .expect("real Chromium goto should finish before the deadline")
        .unwrap()
        .into_outcome()
        .expect("real Chromium goto should succeed");
        Service::<Command<TraceStopChunk>>::call(
            tracing.as_ref(),
            command(
                4 + iteration * 3,
                context_id,
                TraceStopChunkOptions::new(TraceStopChunkMode::Sink),
            ),
        )
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0,
            "a flushed trace chunk must release every payload lease"
        );
        tokio::time::sleep(Duration::from_millis(plan.sample_interval_millis())).await;
        let phase = if iteration == 0 {
            MemoryWorkloadPhase::Warmup
        } else {
            MemoryWorkloadPhase::Steady
        };
        let observed = observation(phase, &started, &probe, browser_process_id, &resources);
        if phase == MemoryWorkloadPhase::Warmup {
            warm_processes = Some(observed.process.browser.total.processes);
        }
        workload.record(observed).unwrap();
    }
    let warm_processes = warm_processes.expect("warmup observation should exist");
    workload
        .record(observation(
            MemoryWorkloadPhase::PreTeardown,
            &started,
            &probe,
            browser_process_id,
            &resources,
        ))
        .unwrap();
    Service::<Command<TraceStop>>::call(tracing.as_ref(), command(20, context_id, ()))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    assert!(tracing.remove_context(context_id).unwrap());
    dispose_context_and_verify_target(&browser, &browser_context_id, &target_id).await;
    wait_for_browser_process_count_at_most(&probe, browser_process_id, warm_processes).await;
    let mut post_teardown = observation(
        MemoryWorkloadPhase::PostTeardown,
        &started,
        &probe,
        browser_process_id,
        &resources,
    );
    post_teardown.objects = BrowserObjectCounts::default();
    workload.record(post_teardown).unwrap();
    let report = workload.finish().expect("tracing workload should finish");

    // Assert both logical cleanup and the diagnostic process-settlement gate.
    assert!(report.logical_state_returned_to_baseline());
    assert!(report.browser_process_tree_returned_to_baseline());
    assert_eq!(sink.chunks.load(Ordering::Relaxed), 4);
    assert_eq!(sink.entries.load(Ordering::Relaxed), 8);
    assert_eq!(sink.last_call_id.load(Ordering::Relaxed), 12);
    assert!(
        report
            .observations()
            .iter()
            .filter(|entry| entry.phase == MemoryWorkloadPhase::Steady)
            .all(|entry| entry.resources.class(ResourceClass::TraceBytes).current == 0)
    );
    eprintln!(
        "pwright tracing workload: runtime_delta={}, browser_delta={}, browser_processes={}→{}",
        report.runtime_rss_delta(),
        report.browser_rss_sum_delta(),
        report.baseline().process.browser.total.processes,
        report.post_teardown().process.browser.total.processes
    );
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("external browser should remain alive");
}
