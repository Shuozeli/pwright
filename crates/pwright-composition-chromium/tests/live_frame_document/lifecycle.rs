use super::TestResult;
use pwright_backend_cdp_navigation::frame_lifecycle::{
    DocumentGeneration, FrameLifecycleProfile,
    native::{CdpFrameLifecycleProfile, NativeLifecycleNotice},
};
use pwright_composition_chromium::{
    ChromiumFrameEventObserver, ChromiumTopologyEventDriver, ChromiumTopologyRuntime,
    FrameObservationResult,
    frame_lifecycle::{
        FrameLifecycleClock, FrameLifecycleNoticeSink, PageLifecycleObservation,
        PageLifecycleObservationProfile,
    },
};
use pwright_core::{ObjectId, ProductionResourceLedger, ResourceScope};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

struct Clock(Instant);
impl FrameLifecycleClock for Clock {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
}
#[derive(Default)]
struct Sink(AtomicUsize);
impl FrameLifecycleNoticeSink for Sink {
    fn observe(&self, notice: NativeLifecycleNotice<'_>) -> FrameObservationResult {
        if matches!(notice, NativeLifecycleNotice::Response { .. }) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }
}
pub fn profile() -> TestResult<PageLifecycleObservationProfile> {
    Ok(PageLifecycleObservationProfile::new(
        CdpFrameLifecycleProfile::new(
            FrameLifecycleProfile::new(ResourceScope::extension(94, 1), 8, 64)?,
            ResourceScope::extension(94, 2),
            128,
        )?,
        ResourceScope::extension(94, 3),
    ))
}
pub struct Probe {
    pub owner: Arc<PageLifecycleObservation>,
    sink: Arc<Sink>,
}
impl Probe {
    pub fn observer(&self) -> Arc<dyn ChromiumFrameEventObserver> {
        Arc::new(Trace(self.owner.clone()))
    }
    pub fn new(resources: &ProductionResourceLedger) -> TestResult<Self> {
        let sink = Arc::new(Sink::default());
        let owner = Arc::new(PageLifecycleObservation::new(
            profile()?,
            ObjectId::new(3),
            1,
            resources,
            Arc::new(Clock(Instant::now())),
            sink.clone(),
        )?);
        Ok(Self { owner, sink })
    }
    pub fn responses(&self) -> usize {
        self.sink.0.load(Ordering::SeqCst)
    }
    pub async fn wait_loaded(
        &self,
        driver: &mut ChromiumTopologyEventDriver,
        runtime: &impl ChromiumTopologyRuntime,
        frame: ObjectId,
        previous: Option<DocumentGeneration>,
    ) -> TestResult<()> {
        loop {
            if self
                .owner
                .snapshot(frame)?
                .is_some_and(|s| s.document != previous && s.load && s.network_idle)
            {
                return Ok(());
            }
            // Use the driver's idle-only signal, not cancellation of a partly
            // applied event. The outer test owns the bounded overall timeout.
            driver
                .receive_once_or_signal(runtime, tokio::time::sleep(Duration::from_millis(25)))
                .await?;
        }
    }
}

// Bounded test-only diagnostics: no event buffering or response payload dump.
struct Trace(Arc<PageLifecycleObservation>);
impl ChromiumFrameEventObserver for Trace {
    fn observes_page(&self, page: ObjectId) -> bool {
        self.0.observes_page(page)
    }
    fn initialize_frame(
        &self,
        topology: &pwright_backend_cdp::CdpTopology,
        session: &str,
        frame: &pwright_cdp::generated::page::Frame,
    ) -> FrameObservationResult {
        println!(
            "lifecycle seed session={session} frame={} loader={}",
            frame.id, frame.loader_id
        );
        self.0.initialize_frame(topology, session, frame)
    }
    fn reconcile(&self, topology: &pwright_backend_cdp::CdpTopology) -> FrameObservationResult {
        self.0.reconcile(topology)
    }
    fn event(
        &self,
        topology: &pwright_backend_cdp::CdpTopology,
        event: &pwright_cdp::CdpEvent,
    ) -> FrameObservationResult {
        if event.method == "Page.frameNavigated" {
            println!(
                "lifecycle commit session={:?} frame={} loader={}",
                event.session_id, event.params["frame"]["id"], event.params["frame"]["loaderId"]
            );
        }
        self.0.event(topology, event)
    }
    fn seal(&self) {
        self.0.seal();
    }
}
