use std::time::{Duration, Instant};

use pwright_backend_cdp::CdpTopologyMetrics;
use pwright_core::{ResourceClass, ResourceLimits, ResourceMetrics, ResourceRequirements};
use pwright_process_metrics::{
    BrowserObjectCounts, MemoryObservation, MemoryWorkloadPhase, ProcessAttributionMetrics,
    ProcessId, ProcessMetricsLimits, ProcessMetricsProbe,
};

pub fn exact_limits(requirements: ResourceRequirements) -> ResourceLimits {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let capacity = requirements.class(class);
        limits = limits.with_class(class, capacity.global, capacity.per_scope);
    }
    limits
}

pub fn browser_process_id_from_env() -> ProcessId {
    ProcessId::new(
        std::env::var("PWRIGHT_BROWSER_PID")
            .expect("PWRIGHT_BROWSER_PID must identify the dedicated Chrome parent")
            .parse()
            .expect("PWRIGHT_BROWSER_PID should be an integer"),
    )
    .expect("PWRIGHT_BROWSER_PID should be non-zero")
}

pub fn system_process_probe() -> ProcessMetricsProbe {
    ProcessMetricsProbe::system(
        ProcessMetricsLimits::new(8 * 1024 * 1024, 16_384, 64 * 1024)
            .expect("process limits should validate"),
    )
}

pub async fn wait_for_browser_process_count_at_most(
    probe: &ProcessMetricsProbe,
    browser_process_id: ProcessId,
    maximum_processes: usize,
    timeout: Duration,
) -> ProcessAttributionMetrics {
    let started = Instant::now();
    loop {
        let metrics = probe
            .sample_current_runtime(browser_process_id)
            .expect("process roots should remain sampleable while settling");
        if metrics.browser.total.processes <= maximum_processes {
            return metrics;
        }
        assert!(
            started.elapsed() < timeout,
            "browser process tree retained {} processes after {:?}; expected at most {}",
            metrics.browser.total.processes,
            timeout,
            maximum_processes
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

pub async fn wait_for_browser_process_count_stable(
    probe: &ProcessMetricsProbe,
    browser_process_id: ProcessId,
    stable_for: Duration,
    timeout: Duration,
) -> ProcessAttributionMetrics {
    let started = Instant::now();
    let mut last_process_count = None;
    let mut stable_since = Instant::now();
    loop {
        let metrics = probe
            .sample_current_runtime(browser_process_id)
            .expect("process roots should remain sampleable while settling");
        let process_count = metrics.browser.total.processes;
        if last_process_count != Some(process_count) {
            last_process_count = Some(process_count);
            stable_since = Instant::now();
        } else if stable_since.elapsed() >= stable_for {
            return metrics;
        }
        assert!(
            started.elapsed() < timeout,
            "browser process tree did not hold one process count for {stable_for:?} within {timeout:?}; latest count was {process_count}"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn correlated_observation(
    phase: MemoryWorkloadPhase,
    started: &Instant,
    probe: &ProcessMetricsProbe,
    browser_process_id: ProcessId,
    resources: ResourceMetrics,
    topology: CdpTopologyMetrics,
    pending_calls: usize,
    queued_events: usize,
) -> MemoryObservation {
    MemoryObservation {
        phase,
        monotonic_millis: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        process: probe
            .sample_current_runtime(browser_process_id)
            .expect("process roots should remain sampleable"),
        resources,
        objects: BrowserObjectCounts {
            contexts: topology.live_contexts,
            pages: topology.live_pages,
            frames: topology.live_frames,
            workers: 0,
            handles: usize::try_from(resources.class(ResourceClass::RemoteHandle).current)
                .unwrap_or(usize::MAX),
            pending_calls,
            queued_events,
        },
    }
}
