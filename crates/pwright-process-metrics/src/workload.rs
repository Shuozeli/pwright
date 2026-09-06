use std::fmt::{self, Display, Formatter};

use pwright_core::{ResourceClass, ResourceMetrics};

use crate::{ProcessAttributionMetrics, ProcessExitMetrics};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWorkloadKind {
    Idle,
    Navigation,
    RepeatedPageCreation,
    Handles,
    NetworkTraffic,
    Tracing,
    Teardown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWorkloadPhase {
    Baseline,
    Warmup,
    Steady,
    PreTeardown,
    PostTeardown,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BrowserObjectCounts {
    pub contexts: usize,
    pub pages: usize,
    pub frames: usize,
    pub workers: usize,
    pub handles: usize,
    pub pending_calls: usize,
    pub queued_events: usize,
}

impl BrowserObjectCounts {
    fn no_more_than(self, baseline: Self) -> bool {
        self.contexts <= baseline.contexts
            && self.pages <= baseline.pages
            && self.frames <= baseline.frames
            && self.workers <= baseline.workers
            && self.handles <= baseline.handles
            && self.pending_calls <= baseline.pending_calls
            && self.queued_events <= baseline.queued_events
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryWorkloadLimits {
    max_observations: usize,
    max_phase_observations: usize,
}

impl MemoryWorkloadLimits {
    pub fn new(
        max_observations: usize,
        max_phase_observations: usize,
    ) -> Result<Self, MemoryWorkloadError> {
        if max_observations == 0 {
            return Err(MemoryWorkloadError::InvalidLimit("observations"));
        }
        if max_phase_observations == 0 {
            return Err(MemoryWorkloadError::InvalidLimit("phase observations"));
        }
        Ok(Self {
            max_observations,
            max_phase_observations,
        })
    }

    pub const fn max_observations(self) -> usize {
        self.max_observations
    }

    pub const fn max_phase_observations(self) -> usize {
        self.max_phase_observations
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryWorkloadPlan {
    kind: MemoryWorkloadKind,
    warmup_observations: usize,
    steady_observations: usize,
    sample_interval_millis: u64,
    teardown_deadline_millis: u64,
    expected_observations: usize,
    reusable_variable_payload_classes: u16,
}

impl MemoryWorkloadPlan {
    pub fn new(
        kind: MemoryWorkloadKind,
        warmup_observations: usize,
        steady_observations: usize,
        sample_interval_millis: u64,
        teardown_deadline_millis: u64,
        limits: MemoryWorkloadLimits,
    ) -> Result<Self, MemoryWorkloadError> {
        if warmup_observations > limits.max_phase_observations {
            return Err(MemoryWorkloadError::PhaseCapacityExceeded {
                phase: MemoryWorkloadPhase::Warmup,
                actual: warmup_observations,
                limit: limits.max_phase_observations,
            });
        }
        if steady_observations == 0 {
            return Err(MemoryWorkloadError::EmptySteadyPhase);
        }
        if steady_observations > limits.max_phase_observations {
            return Err(MemoryWorkloadError::PhaseCapacityExceeded {
                phase: MemoryWorkloadPhase::Steady,
                actual: steady_observations,
                limit: limits.max_phase_observations,
            });
        }
        if sample_interval_millis == 0 {
            return Err(MemoryWorkloadError::InvalidSampleInterval);
        }
        if teardown_deadline_millis == 0 {
            return Err(MemoryWorkloadError::InvalidTeardownDeadline);
        }
        let expected_observations = warmup_observations
            .checked_add(steady_observations)
            .and_then(|count| count.checked_add(3))
            .ok_or(MemoryWorkloadError::ObservationCountOverflow)?;
        if expected_observations > limits.max_observations {
            return Err(MemoryWorkloadError::ObservationCapacityExceeded {
                actual: expected_observations,
                limit: limits.max_observations,
            });
        }
        Ok(Self {
            kind,
            warmup_observations,
            steady_observations,
            sample_interval_millis,
            teardown_deadline_millis,
            expected_observations,
            reusable_variable_payload_classes: 0,
        })
    }

    /// Allows one byte class to vary after warmup while its item owners,
    /// scopes, and leases must still return to their comparison baseline.
    ///
    /// This models reusable bounded payload slots such as a consumed broadcast
    /// ring whose final messages may have different encoded lengths. It is not
    /// an RSS budget and it never relaxes the ledger's configured hard limit.
    pub fn with_reusable_variable_payload_class(
        mut self,
        class: ResourceClass,
    ) -> Result<Self, MemoryWorkloadError> {
        if !class.is_byte_class() {
            return Err(MemoryWorkloadError::InvalidReusablePayloadClass(class));
        }
        self.reusable_variable_payload_classes |= 1_u16 << u32::from(class as u8);
        Ok(self)
    }

    pub const fn kind(self) -> MemoryWorkloadKind {
        self.kind
    }

    pub const fn warmup_observations(self) -> usize {
        self.warmup_observations
    }

    pub const fn steady_observations(self) -> usize {
        self.steady_observations
    }

    /// Target cadence for a caller-driven workload.
    ///
    /// The recorder stores this value for reproducibility but does not sleep or
    /// reject scheduler jitter. The caller remains responsible for scheduling
    /// observations and performing browser work between them.
    pub const fn sample_interval_millis(self) -> u64 {
        self.sample_interval_millis
    }

    pub const fn teardown_deadline_millis(self) -> u64 {
        self.teardown_deadline_millis
    }

    pub const fn expected_observations(self) -> usize {
        self.expected_observations
    }

    pub const fn permits_reusable_variable_payload(self, class: ResourceClass) -> bool {
        self.reusable_variable_payload_classes & (1_u16 << class as u8) != 0
    }

    fn expected_phase(self, index: usize) -> Option<MemoryWorkloadPhase> {
        if index == 0 {
            return Some(MemoryWorkloadPhase::Baseline);
        }
        let warmup_end = 1 + self.warmup_observations;
        if index < warmup_end {
            return Some(MemoryWorkloadPhase::Warmup);
        }
        let steady_end = warmup_end + self.steady_observations;
        if index < steady_end {
            return Some(MemoryWorkloadPhase::Steady);
        }
        if index == steady_end {
            return Some(MemoryWorkloadPhase::PreTeardown);
        }
        if index == steady_end + 1 {
            return Some(MemoryWorkloadPhase::PostTeardown);
        }
        None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryObservation {
    pub phase: MemoryWorkloadPhase,
    pub monotonic_millis: u64,
    pub process: ProcessAttributionMetrics,
    pub resources: ResourceMetrics,
    pub objects: BrowserObjectCounts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OwnedBrowserPostTeardownObservation {
    pub monotonic_millis: u64,
    pub process: ProcessExitMetrics,
    pub resources: ResourceMetrics,
    pub objects: BrowserObjectCounts,
}

#[derive(Debug)]
pub struct MemoryWorkloadRecorder {
    plan: MemoryWorkloadPlan,
    observations: Vec<MemoryObservation>,
}

impl MemoryWorkloadRecorder {
    pub fn new(plan: MemoryWorkloadPlan) -> Result<Self, MemoryWorkloadError> {
        let mut observations = Vec::new();
        observations
            .try_reserve_exact(plan.expected_observations)
            .map_err(|_| MemoryWorkloadError::ObservationAllocationFailed {
                requested: plan.expected_observations,
            })?;
        Ok(Self { plan, observations })
    }

    pub const fn plan(&self) -> MemoryWorkloadPlan {
        self.plan
    }

    pub fn next_phase(&self) -> Option<MemoryWorkloadPhase> {
        self.plan.expected_phase(self.observations.len())
    }

    pub fn record(&mut self, observation: MemoryObservation) -> Result<(), MemoryWorkloadError> {
        let index = self.observations.len();
        let expected = self
            .plan
            .expected_phase(index)
            .ok_or(MemoryWorkloadError::AlreadyComplete)?;
        if observation.phase != expected {
            return Err(MemoryWorkloadError::UnexpectedPhase {
                expected,
                actual: observation.phase,
            });
        }
        if let Some(previous) = self.observations.last()
            && observation.monotonic_millis < previous.monotonic_millis
        {
            return Err(MemoryWorkloadError::NonMonotonicTime {
                previous: previous.monotonic_millis,
                actual: observation.monotonic_millis,
            });
        }
        if let Some(baseline) = self.observations.first()
            && (observation.process.runtime.process_id != baseline.process.runtime.process_id
                || observation.process.browser.process_id != baseline.process.browser.process_id)
        {
            return Err(MemoryWorkloadError::ProcessRootsChanged {
                expected_runtime: baseline.process.runtime.process_id.get(),
                actual_runtime: observation.process.runtime.process_id.get(),
                expected_browser: baseline.process.browser.process_id.get(),
                actual_browser: observation.process.browser.process_id.get(),
            });
        }
        self.observations.push(observation);
        Ok(())
    }

    pub fn finish(self) -> Result<MemoryWorkloadReport, MemoryWorkloadError> {
        if self.observations.len() != self.plan.expected_observations {
            return Err(MemoryWorkloadError::Incomplete {
                actual: self.observations.len(),
                expected: self.plan.expected_observations,
            });
        }
        let pre_teardown = self.observations[self.observations.len() - 2];
        let post_teardown = self.observations[self.observations.len() - 1];
        let elapsed_millis = post_teardown
            .monotonic_millis
            .checked_sub(pre_teardown.monotonic_millis)
            .expect("record rejects non-monotonic observations");
        if elapsed_millis > self.plan.teardown_deadline_millis {
            return Err(MemoryWorkloadError::TeardownDeadlineExceeded {
                elapsed_millis,
                limit_millis: self.plan.teardown_deadline_millis,
            });
        }
        Ok(MemoryWorkloadReport {
            plan: self.plan,
            observations: self.observations,
        })
    }

    /// Finishes a workload whose teardown intentionally terminates the owned
    /// browser root. All earlier observations retain normal two-root
    /// attribution; the final observation must contain a same-snapshot exit
    /// proof produced by `ProcessMetricsProbe`.
    pub fn finish_owned_browser(
        self,
        post_teardown: OwnedBrowserPostTeardownObservation,
    ) -> Result<OwnedBrowserMemoryWorkloadReport, MemoryWorkloadError> {
        let expected_running = self.plan.expected_observations.saturating_sub(1);
        if self.observations.len() != expected_running {
            return Err(MemoryWorkloadError::Incomplete {
                actual: self.observations.len().saturating_add(1),
                expected: self.plan.expected_observations,
            });
        }
        let baseline = self
            .observations
            .first()
            .expect("a valid workload has a baseline before post-teardown");
        let pre_teardown = self
            .observations
            .last()
            .expect("a valid workload has a pre-teardown observation");
        if pre_teardown.phase != MemoryWorkloadPhase::PreTeardown {
            return Err(MemoryWorkloadError::UnexpectedPhase {
                expected: MemoryWorkloadPhase::PreTeardown,
                actual: pre_teardown.phase,
            });
        }
        let post_runtime = post_teardown.process.runtime();
        let exited_browser = post_teardown.process.exited_process_id();
        if post_runtime.process_id != baseline.process.runtime.process_id
            || exited_browser != baseline.process.browser.process_id
        {
            return Err(MemoryWorkloadError::ProcessRootsChanged {
                expected_runtime: baseline.process.runtime.process_id.get(),
                actual_runtime: post_runtime.process_id.get(),
                expected_browser: baseline.process.browser.process_id.get(),
                actual_browser: exited_browser.get(),
            });
        }
        if post_teardown.monotonic_millis < pre_teardown.monotonic_millis {
            return Err(MemoryWorkloadError::NonMonotonicTime {
                previous: pre_teardown.monotonic_millis,
                actual: post_teardown.monotonic_millis,
            });
        }
        let elapsed_millis = post_teardown.monotonic_millis - pre_teardown.monotonic_millis;
        if elapsed_millis > self.plan.teardown_deadline_millis {
            return Err(MemoryWorkloadError::TeardownDeadlineExceeded {
                elapsed_millis,
                limit_millis: self.plan.teardown_deadline_millis,
            });
        }
        Ok(OwnedBrowserMemoryWorkloadReport {
            plan: self.plan,
            observations: self.observations,
            post_teardown,
        })
    }
}

#[derive(Debug)]
pub struct OwnedBrowserMemoryWorkloadReport {
    plan: MemoryWorkloadPlan,
    observations: Vec<MemoryObservation>,
    post_teardown: OwnedBrowserPostTeardownObservation,
}

impl OwnedBrowserMemoryWorkloadReport {
    pub const fn plan(&self) -> MemoryWorkloadPlan {
        self.plan
    }

    pub fn observations(&self) -> &[MemoryObservation] {
        &self.observations
    }

    pub fn baseline(&self) -> MemoryObservation {
        self.observations[0]
    }

    pub const fn post_teardown(&self) -> OwnedBrowserPostTeardownObservation {
        self.post_teardown
    }

    pub fn comparison_baseline(&self) -> MemoryObservation {
        self.observations
            .iter()
            .rev()
            .find(|observation| observation.phase == MemoryWorkloadPhase::Warmup)
            .copied()
            .unwrap_or_else(|| self.baseline())
    }

    pub fn logical_state_returned_to_baseline(&self) -> bool {
        let baseline = self.comparison_baseline();
        let post = self.post_teardown;
        let resources_returned = ResourceClass::ALL.into_iter().all(|class| {
            self.plan.permits_reusable_variable_payload(class)
                || post.resources.class(class).current <= baseline.resources.class(class).current
        }) && post.resources.live_scopes <= baseline.resources.live_scopes
            && post.resources.active_leases <= baseline.resources.active_leases;
        resources_returned && post.objects.no_more_than(baseline.objects)
    }

    pub fn runtime_rss_delta(&self) -> i128 {
        i128::from(self.post_teardown.process.runtime().resident_bytes)
            - i128::from(self.baseline().process.runtime.resident_bytes)
    }

    pub fn pre_teardown_browser_rss_sum(&self) -> u64 {
        self.observations
            .last()
            .expect("owned-browser report always retains pre-teardown")
            .process
            .browser
            .total
            .resident_bytes
    }
}

#[derive(Debug)]
pub struct MemoryWorkloadReport {
    plan: MemoryWorkloadPlan,
    observations: Vec<MemoryObservation>,
}

impl MemoryWorkloadReport {
    pub const fn plan(&self) -> MemoryWorkloadPlan {
        self.plan
    }

    pub fn observations(&self) -> &[MemoryObservation] {
        &self.observations
    }

    pub fn baseline(&self) -> MemoryObservation {
        self.observations[0]
    }

    pub fn post_teardown(&self) -> MemoryObservation {
        *self
            .observations
            .last()
            .expect("a complete workload always has post-teardown")
    }

    /// Returns the state teardown must reach after reusable buffers and other
    /// bounded connection state have been warmed.
    ///
    /// A plan with no warmup observations uses the cold baseline. Callers
    /// should record warmup observations only after each warmup iteration has
    /// cleaned up its temporary browser objects.
    pub fn comparison_baseline(&self) -> MemoryObservation {
        self.observations
            .iter()
            .rev()
            .find(|observation| observation.phase == MemoryWorkloadPhase::Warmup)
            .copied()
            .unwrap_or_else(|| self.baseline())
    }

    pub fn logical_state_returned_to_baseline(&self) -> bool {
        let baseline = self.comparison_baseline();
        let post = self.post_teardown();
        let resources_returned = ResourceClass::ALL.into_iter().all(|class| {
            self.plan.permits_reusable_variable_payload(class)
                || post.resources.class(class).current <= baseline.resources.class(class).current
        }) && post.resources.live_scopes <= baseline.resources.live_scopes
            && post.resources.active_leases <= baseline.resources.active_leases;
        resources_returned && post.objects.no_more_than(baseline.objects)
    }

    /// Returns whether temporary browser processes have exited after teardown.
    ///
    /// This is deliberately separate from logical state and RSS: a browser may
    /// retain allocator memory in an existing process even after its temporary
    /// renderer/helper processes have exited.
    pub fn browser_process_tree_returned_to_baseline(&self) -> bool {
        self.post_teardown().process.browser.total.processes
            <= self.comparison_baseline().process.browser.total.processes
    }

    /// Post-teardown minus cold-baseline runtime RSS.
    pub fn runtime_rss_delta(&self) -> i128 {
        i128::from(self.post_teardown().process.runtime.resident_bytes)
            - i128::from(self.baseline().process.runtime.resident_bytes)
    }

    /// Post-teardown minus cold-baseline browser-tree RSS sum.
    pub fn browser_rss_sum_delta(&self) -> i128 {
        i128::from(self.post_teardown().process.browser.total.resident_bytes)
            - i128::from(self.baseline().process.browser.total.resident_bytes)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryWorkloadSeriesLimits {
    max_runs: usize,
}

impl MemoryWorkloadSeriesLimits {
    pub fn new(max_runs: usize) -> Result<Self, MemoryWorkloadSeriesError> {
        if max_runs == 0 {
            return Err(MemoryWorkloadSeriesError::InvalidLimit("runs"));
        }
        Ok(Self { max_runs })
    }

    pub const fn max_runs(self) -> usize {
        self.max_runs
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryWorkloadSeriesPlan {
    workload: MemoryWorkloadPlan,
    runs: usize,
}

impl MemoryWorkloadSeriesPlan {
    pub fn new(
        workload: MemoryWorkloadPlan,
        runs: usize,
        limits: MemoryWorkloadSeriesLimits,
    ) -> Result<Self, MemoryWorkloadSeriesError> {
        if runs == 0 {
            return Err(MemoryWorkloadSeriesError::EmptySeries);
        }
        if runs > limits.max_runs {
            return Err(MemoryWorkloadSeriesError::RunCapacityExceeded {
                actual: runs,
                limit: limits.max_runs,
            });
        }
        Ok(Self { workload, runs })
    }

    pub const fn workload(self) -> MemoryWorkloadPlan {
        self.workload
    }

    pub const fn runs(self) -> usize {
        self.runs
    }
}

#[derive(Debug)]
pub struct MemoryWorkloadSeriesRecorder {
    plan: MemoryWorkloadSeriesPlan,
    reports: Vec<MemoryWorkloadReport>,
}

impl MemoryWorkloadSeriesRecorder {
    pub fn new(plan: MemoryWorkloadSeriesPlan) -> Result<Self, MemoryWorkloadSeriesError> {
        let mut reports = Vec::new();
        reports.try_reserve_exact(plan.runs).map_err(|_| {
            MemoryWorkloadSeriesError::RunAllocationFailed {
                requested: plan.runs,
            }
        })?;
        Ok(Self { plan, reports })
    }

    pub const fn plan(&self) -> MemoryWorkloadSeriesPlan {
        self.plan
    }

    pub fn completed_runs(&self) -> usize {
        self.reports.len()
    }

    pub fn record(
        &mut self,
        report: MemoryWorkloadReport,
    ) -> Result<(), MemoryWorkloadSeriesError> {
        if self.reports.len() == self.plan.runs {
            return Err(MemoryWorkloadSeriesError::AlreadyComplete);
        }
        if report.plan() != self.plan.workload {
            return Err(MemoryWorkloadSeriesError::WorkloadPlanMismatch {
                expected: self.plan.workload,
                actual: report.plan(),
            });
        }
        let run = self.reports.len();
        if !report.logical_state_returned_to_baseline() {
            return Err(MemoryWorkloadSeriesError::LogicalStateNotReturned { run });
        }
        if !report.browser_process_tree_returned_to_baseline() {
            return Err(MemoryWorkloadSeriesError::BrowserProcessTreeNotSettled { run });
        }
        self.reports.push(report);
        Ok(())
    }

    pub fn finish(self) -> Result<MemoryWorkloadSeriesReport, MemoryWorkloadSeriesError> {
        if self.reports.len() != self.plan.runs {
            return Err(MemoryWorkloadSeriesError::Incomplete {
                actual: self.reports.len(),
                expected: self.plan.runs,
            });
        }
        Ok(MemoryWorkloadSeriesReport {
            plan: self.plan,
            reports: self.reports,
        })
    }
}

#[derive(Debug)]
pub struct MemoryWorkloadSeriesReport {
    plan: MemoryWorkloadSeriesPlan,
    reports: Vec<MemoryWorkloadReport>,
}

impl MemoryWorkloadSeriesReport {
    pub const fn plan(&self) -> MemoryWorkloadSeriesPlan {
        self.plan
    }

    pub fn reports(&self) -> &[MemoryWorkloadReport] {
        &self.reports
    }

    pub fn runtime_rss_delta_range(&self) -> (i128, i128) {
        delta_range(
            self.reports
                .iter()
                .map(MemoryWorkloadReport::runtime_rss_delta),
        )
    }

    pub fn browser_rss_sum_delta_range(&self) -> (i128, i128) {
        delta_range(
            self.reports
                .iter()
                .map(MemoryWorkloadReport::browser_rss_sum_delta),
        )
    }
}

#[derive(Debug)]
pub struct OwnedBrowserMemoryWorkloadSeriesRecorder {
    plan: MemoryWorkloadSeriesPlan,
    reports: Vec<OwnedBrowserMemoryWorkloadReport>,
}

impl OwnedBrowserMemoryWorkloadSeriesRecorder {
    pub fn new(plan: MemoryWorkloadSeriesPlan) -> Result<Self, MemoryWorkloadSeriesError> {
        let mut reports = Vec::new();
        reports.try_reserve_exact(plan.runs).map_err(|_| {
            MemoryWorkloadSeriesError::RunAllocationFailed {
                requested: plan.runs,
            }
        })?;
        Ok(Self { plan, reports })
    }

    pub const fn plan(&self) -> MemoryWorkloadSeriesPlan {
        self.plan
    }

    pub fn completed_runs(&self) -> usize {
        self.reports.len()
    }

    pub fn record(
        &mut self,
        report: OwnedBrowserMemoryWorkloadReport,
    ) -> Result<(), MemoryWorkloadSeriesError> {
        if self.reports.len() == self.plan.runs {
            return Err(MemoryWorkloadSeriesError::AlreadyComplete);
        }
        if report.plan() != self.plan.workload {
            return Err(MemoryWorkloadSeriesError::WorkloadPlanMismatch {
                expected: self.plan.workload,
                actual: report.plan(),
            });
        }
        let run = self.reports.len();
        if !report.logical_state_returned_to_baseline() {
            return Err(MemoryWorkloadSeriesError::LogicalStateNotReturned { run });
        }
        self.reports.push(report);
        Ok(())
    }

    pub fn finish(
        self,
    ) -> Result<OwnedBrowserMemoryWorkloadSeriesReport, MemoryWorkloadSeriesError> {
        if self.reports.len() != self.plan.runs {
            return Err(MemoryWorkloadSeriesError::Incomplete {
                actual: self.reports.len(),
                expected: self.plan.runs,
            });
        }
        Ok(OwnedBrowserMemoryWorkloadSeriesReport {
            plan: self.plan,
            reports: self.reports,
        })
    }
}

#[derive(Debug)]
pub struct OwnedBrowserMemoryWorkloadSeriesReport {
    plan: MemoryWorkloadSeriesPlan,
    reports: Vec<OwnedBrowserMemoryWorkloadReport>,
}

impl OwnedBrowserMemoryWorkloadSeriesReport {
    pub const fn plan(&self) -> MemoryWorkloadSeriesPlan {
        self.plan
    }

    pub fn reports(&self) -> &[OwnedBrowserMemoryWorkloadReport] {
        &self.reports
    }

    pub fn runtime_rss_delta_range(&self) -> (i128, i128) {
        delta_range(
            self.reports
                .iter()
                .map(OwnedBrowserMemoryWorkloadReport::runtime_rss_delta),
        )
    }

    pub fn pre_teardown_browser_rss_sum_range(&self) -> (u64, u64) {
        let mut values = self
            .reports
            .iter()
            .map(OwnedBrowserMemoryWorkloadReport::pre_teardown_browser_rss_sum);
        let first = values
            .next()
            .expect("a completed workload series always contains a run");
        values.fold((first, first), |(minimum, maximum), value| {
            (minimum.min(value), maximum.max(value))
        })
    }
}

fn delta_range(mut deltas: impl Iterator<Item = i128>) -> (i128, i128) {
    let first = deltas
        .next()
        .expect("a completed workload series always contains a run");
    deltas.fold((first, first), |(minimum, maximum), delta| {
        (minimum.min(delta), maximum.max(delta))
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryWorkloadSeriesError {
    InvalidLimit(&'static str),
    EmptySeries,
    RunCapacityExceeded {
        actual: usize,
        limit: usize,
    },
    RunAllocationFailed {
        requested: usize,
    },
    WorkloadPlanMismatch {
        expected: MemoryWorkloadPlan,
        actual: MemoryWorkloadPlan,
    },
    LogicalStateNotReturned {
        run: usize,
    },
    BrowserProcessTreeNotSettled {
        run: usize,
    },
    AlreadyComplete,
    Incomplete {
        actual: usize,
        expected: usize,
    },
}

impl Display for MemoryWorkloadSeriesError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit(name) => write!(formatter, "{name} limit must be non-zero"),
            Self::EmptySeries => formatter.write_str("a memory workload series requires a run"),
            Self::RunCapacityExceeded { actual, limit } => write!(
                formatter,
                "memory workload series needs {actual} runs, exceeding limit {limit}"
            ),
            Self::RunAllocationFailed { requested } => write!(
                formatter,
                "memory workload series could not reserve {requested} runs"
            ),
            Self::WorkloadPlanMismatch { expected, actual } => write!(
                formatter,
                "memory workload series expected plan {expected:?}, received {actual:?}"
            ),
            Self::LogicalStateNotReturned { run } => write!(
                formatter,
                "memory workload series run {run} did not return logical state to baseline"
            ),
            Self::BrowserProcessTreeNotSettled { run } => write!(
                formatter,
                "memory workload series run {run} did not settle the browser process tree"
            ),
            Self::AlreadyComplete => formatter.write_str("memory workload series is complete"),
            Self::Incomplete { actual, expected } => write!(
                formatter,
                "memory workload series has {actual} runs but needs {expected}"
            ),
        }
    }
}

impl std::error::Error for MemoryWorkloadSeriesError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryWorkloadError {
    InvalidLimit(&'static str),
    InvalidReusablePayloadClass(ResourceClass),
    EmptySteadyPhase,
    InvalidSampleInterval,
    InvalidTeardownDeadline,
    PhaseCapacityExceeded {
        phase: MemoryWorkloadPhase,
        actual: usize,
        limit: usize,
    },
    ObservationCountOverflow,
    ObservationCapacityExceeded {
        actual: usize,
        limit: usize,
    },
    ObservationAllocationFailed {
        requested: usize,
    },
    UnexpectedPhase {
        expected: MemoryWorkloadPhase,
        actual: MemoryWorkloadPhase,
    },
    NonMonotonicTime {
        previous: u64,
        actual: u64,
    },
    ProcessRootsChanged {
        expected_runtime: u32,
        actual_runtime: u32,
        expected_browser: u32,
        actual_browser: u32,
    },
    TeardownDeadlineExceeded {
        elapsed_millis: u64,
        limit_millis: u64,
    },
    AlreadyComplete,
    Incomplete {
        actual: usize,
        expected: usize,
    },
}

impl Display for MemoryWorkloadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit(name) => write!(formatter, "{name} limit must be non-zero"),
            Self::InvalidReusablePayloadClass(class) => write!(
                formatter,
                "resource class {class} is not a byte payload class"
            ),
            Self::EmptySteadyPhase => {
                formatter.write_str("a memory workload requires a steady observation")
            }
            Self::InvalidSampleInterval => formatter.write_str("sample interval must be non-zero"),
            Self::InvalidTeardownDeadline => {
                formatter.write_str("teardown deadline must be non-zero")
            }
            Self::PhaseCapacityExceeded {
                phase,
                actual,
                limit,
            } => write!(
                formatter,
                "{phase:?} phase has {actual} observations, exceeding limit {limit}"
            ),
            Self::ObservationCountOverflow => {
                formatter.write_str("memory workload observation count overflowed")
            }
            Self::ObservationCapacityExceeded { actual, limit } => write!(
                formatter,
                "memory workload needs {actual} observations, exceeding limit {limit}"
            ),
            Self::ObservationAllocationFailed { requested } => write!(
                formatter,
                "memory workload could not reserve {requested} observations"
            ),
            Self::UnexpectedPhase { expected, actual } => write!(
                formatter,
                "memory workload expected {expected:?}, received {actual:?}"
            ),
            Self::NonMonotonicTime { previous, actual } => write!(
                formatter,
                "memory observation time moved backward from {previous} to {actual}"
            ),
            Self::ProcessRootsChanged {
                expected_runtime,
                actual_runtime,
                expected_browser,
                actual_browser,
            } => write!(
                formatter,
                "memory workload roots changed from runtime/browser {expected_runtime}/{expected_browser} to {actual_runtime}/{actual_browser}"
            ),
            Self::TeardownDeadlineExceeded {
                elapsed_millis,
                limit_millis,
            } => write!(
                formatter,
                "teardown took {elapsed_millis} ms, exceeding deadline {limit_millis} ms"
            ),
            Self::AlreadyComplete => formatter.write_str("memory workload is already complete"),
            Self::Incomplete { actual, expected } => write!(
                formatter,
                "memory workload has {actual} observations but needs {expected}"
            ),
        }
    }
}

impl std::error::Error for MemoryWorkloadError {}

#[cfg(test)]
mod tests {
    use pwright_core::{ResourceClass, ResourceLedger, ResourceLimits, ResourceScope};

    use super::*;
    use crate::{
        BrowserProcessMetrics, ProcessId, ProcessMetricsLimits, ProcessRoleMetrics,
        ProcessSnapshot, RuntimeProcessMetrics,
    };

    fn limits() -> MemoryWorkloadLimits {
        MemoryWorkloadLimits::new(8, 3).expect("test limits should be valid")
    }

    fn plan() -> MemoryWorkloadPlan {
        MemoryWorkloadPlan::new(
            MemoryWorkloadKind::RepeatedPageCreation,
            1,
            1,
            10,
            50,
            limits(),
        )
        .expect("test plan should be valid")
    }

    fn ledger() -> ResourceLedger {
        ResourceLedger::new(ResourceLimits::deny_all(4, 4).with_class(
            ResourceClass::LiveObject,
            4,
            4,
        ))
    }

    fn process_with_roots(
        runtime_process_id: u32,
        browser_process_id: u32,
        runtime_resident_bytes: u64,
        browser_resident_bytes: u64,
    ) -> ProcessAttributionMetrics {
        ProcessAttributionMetrics {
            runtime: RuntimeProcessMetrics {
                process_id: ProcessId::new(runtime_process_id)
                    .expect("test runtime process id should be valid"),
                resident_bytes: runtime_resident_bytes,
            },
            browser: BrowserProcessMetrics {
                process_id: ProcessId::new(browser_process_id)
                    .expect("test browser process id should be valid"),
                browser_parent: ProcessRoleMetrics {
                    processes: 1,
                    resident_bytes: browser_resident_bytes,
                },
                renderers: ProcessRoleMetrics::default(),
                gpu: ProcessRoleMetrics::default(),
                utility: ProcessRoleMetrics::default(),
                other_children: ProcessRoleMetrics::default(),
                total: ProcessRoleMetrics {
                    processes: 1,
                    resident_bytes: browser_resident_bytes,
                },
            },
        }
    }

    fn observation(
        phase: MemoryWorkloadPhase,
        monotonic_millis: u64,
        resources: ResourceMetrics,
        objects: BrowserObjectCounts,
        runtime_resident_bytes: u64,
        browser_resident_bytes: u64,
    ) -> MemoryObservation {
        MemoryObservation {
            phase,
            monotonic_millis,
            process: process_with_roots(10, 20, runtime_resident_bytes, browser_resident_bytes),
            resources,
            objects,
        }
    }

    fn record_complete_workload(
        post_teardown_millis: u64,
        post_resources: ResourceMetrics,
        post_objects: BrowserObjectCounts,
    ) -> MemoryWorkloadRecorder {
        let ledger = ledger();
        let baseline = ledger
            .metrics()
            .expect("baseline metrics should be readable");
        let lease = ledger
            .reserve(ResourceScope::Connection, ResourceClass::LiveObject, 1)
            .expect("live-object reservation should fit");
        let active = ledger.metrics().expect("active metrics should be readable");
        let mut recorder = MemoryWorkloadRecorder::new(plan()).expect("recorder should allocate");
        recorder
            .record(observation(
                MemoryWorkloadPhase::Baseline,
                0,
                baseline,
                BrowserObjectCounts::default(),
                100,
                1_000,
            ))
            .expect("baseline should record");
        recorder
            .record(observation(
                MemoryWorkloadPhase::Warmup,
                10,
                baseline,
                BrowserObjectCounts::default(),
                120,
                1_100,
            ))
            .expect("warmup should record");
        recorder
            .record(observation(
                MemoryWorkloadPhase::Steady,
                20,
                active,
                BrowserObjectCounts {
                    pages: 1,
                    ..BrowserObjectCounts::default()
                },
                130,
                1_200,
            ))
            .expect("steady state should record");
        recorder
            .record(observation(
                MemoryWorkloadPhase::PreTeardown,
                30,
                active,
                BrowserObjectCounts {
                    pages: 1,
                    ..BrowserObjectCounts::default()
                },
                140,
                1_150,
            ))
            .expect("pre-teardown should record");
        drop(lease);
        recorder
            .record(observation(
                MemoryWorkloadPhase::PostTeardown,
                post_teardown_millis,
                post_resources,
                post_objects,
                130,
                900,
            ))
            .expect("post-teardown should record");
        recorder
    }

    fn complete_owned_workload(
        plan: MemoryWorkloadPlan,
        runtime_post_kib: u64,
        pre_teardown_browser_rss: u64,
    ) -> OwnedBrowserMemoryWorkloadReport {
        let resources = ledger().metrics().expect("metrics should be readable");
        let mut recorder = MemoryWorkloadRecorder::new(plan).expect("recorder should allocate");
        for (phase, millis) in [
            (MemoryWorkloadPhase::Baseline, 0),
            (MemoryWorkloadPhase::Warmup, 10),
            (MemoryWorkloadPhase::Steady, 20),
            (MemoryWorkloadPhase::PreTeardown, 30),
        ] {
            recorder
                .record(observation(
                    phase,
                    millis,
                    resources,
                    BrowserObjectCounts::default(),
                    100,
                    if phase == MemoryWorkloadPhase::PreTeardown {
                        pre_teardown_browser_rss
                    } else {
                        1_000
                    },
                ))
                .expect("running observation should record");
        }
        let snapshot = format!("10 1 {runtime_post_kib} runtime\n30 1 2 unrelated\n");
        let exit = ProcessSnapshot::parse(
            snapshot.as_bytes(),
            ProcessMetricsLimits::new(1024, 8, 128).unwrap(),
        )
        .unwrap()
        .attribute_runtime_after_exit(ProcessId::new(10).unwrap(), ProcessId::new(20).unwrap())
        .expect("missing browser root should produce proof");
        recorder
            .finish_owned_browser(OwnedBrowserPostTeardownObservation {
                monotonic_millis: 40,
                process: exit,
                resources,
                objects: BrowserObjectCounts::default(),
            })
            .expect("owned workload should finish")
    }

    #[test]
    fn complete_workload_correlates_process_resources_and_objects() {
        // Arrange
        let clean_ledger = ledger();
        let post = clean_ledger
            .metrics()
            .expect("post-teardown metrics should be readable");
        let recorder = record_complete_workload(40, post, BrowserObjectCounts::default());

        // Act
        let report = recorder.finish().expect("complete workload should finish");

        // Assert
        assert_eq!(report.observations().len(), 5);
        assert_eq!(report.plan().sample_interval_millis(), 10);
        assert!(report.logical_state_returned_to_baseline());
        assert!(report.browser_process_tree_returned_to_baseline());
        assert_eq!(report.runtime_rss_delta(), 30);
        assert_eq!(report.browser_rss_sum_delta(), -100);
    }

    #[test]
    fn owned_browser_workload_finishes_with_a_typed_exit_proof() {
        // Arrange
        let ledger = ledger();
        let baseline_resources = ledger.metrics().expect("baseline should be readable");
        let lease = ledger
            .reserve(ResourceScope::Connection, ResourceClass::LiveObject, 1)
            .expect("measured browser should fit");
        let active_resources = ledger.metrics().expect("active state should be readable");
        let mut recorder = MemoryWorkloadRecorder::new(
            MemoryWorkloadPlan::new(MemoryWorkloadKind::Teardown, 1, 1, 10, 50, limits())
                .expect("owned-browser plan should validate"),
        )
        .expect("recorder should allocate");
        for (phase, millis, resources) in [
            (MemoryWorkloadPhase::Baseline, 0, baseline_resources),
            (MemoryWorkloadPhase::Warmup, 10, baseline_resources),
            (MemoryWorkloadPhase::Steady, 20, active_resources),
            (MemoryWorkloadPhase::PreTeardown, 30, active_resources),
        ] {
            recorder
                .record(observation(
                    phase,
                    millis,
                    resources,
                    BrowserObjectCounts {
                        pages: usize::from(matches!(
                            phase,
                            MemoryWorkloadPhase::Steady | MemoryWorkloadPhase::PreTeardown
                        )),
                        ..BrowserObjectCounts::default()
                    },
                    100,
                    1_000,
                ))
                .expect("running observation should record");
        }
        drop(lease);
        let exit = ProcessSnapshot::parse(
            b"10 1 1 runtime\n30 1 2 unrelated\n",
            ProcessMetricsLimits::new(1024, 8, 128).unwrap(),
        )
        .unwrap()
        .attribute_runtime_after_exit(ProcessId::new(10).unwrap(), ProcessId::new(20).unwrap())
        .expect("missing browser root should produce proof");

        // Act
        let report = recorder
            .finish_owned_browser(OwnedBrowserPostTeardownObservation {
                monotonic_millis: 40,
                process: exit,
                resources: baseline_resources,
                objects: BrowserObjectCounts::default(),
            })
            .expect("owned-browser workload should finish");

        // Assert
        assert_eq!(report.plan().kind(), MemoryWorkloadKind::Teardown);
        assert_eq!(report.observations().len(), 4);
        assert_eq!(report.post_teardown().process.exited_process_id().get(), 20);
        assert!(report.logical_state_returned_to_baseline());
        assert_eq!(report.runtime_rss_delta(), 924);
        assert_eq!(report.pre_teardown_browser_rss_sum(), 1_000);
    }

    #[test]
    fn owned_browser_series_is_bounded_and_reports_only_measured_ranges() {
        // Arrange
        let workload =
            MemoryWorkloadPlan::new(MemoryWorkloadKind::Teardown, 1, 1, 10, 50, limits())
                .expect("owned workload should validate");
        let plan = MemoryWorkloadSeriesPlan::new(
            workload,
            2,
            MemoryWorkloadSeriesLimits::new(2).expect("series limit should validate"),
        )
        .expect("owned series should validate");
        let mut series = OwnedBrowserMemoryWorkloadSeriesRecorder::new(plan)
            .expect("owned series should allocate");

        // Act
        series
            .record(complete_owned_workload(workload, 2, 1_500))
            .expect("first owned run should record");
        series
            .record(complete_owned_workload(workload, 3, 1_200))
            .expect("second owned run should record");
        let report = series.finish().expect("owned series should finish");

        // Assert
        assert_eq!(report.plan(), plan);
        assert_eq!(report.reports().len(), 2);
        assert_eq!(report.runtime_rss_delta_range(), (1_948, 2_972));
        assert_eq!(report.pre_teardown_browser_rss_sum_range(), (1_200, 1_500));
    }

    #[test]
    fn process_roots_cannot_change_mid_workload() {
        // Arrange
        let ledger = ledger();
        let resources = ledger.metrics().expect("metrics should be readable");
        let mut recorder = MemoryWorkloadRecorder::new(plan()).expect("recorder should allocate");
        recorder
            .record(observation(
                MemoryWorkloadPhase::Baseline,
                0,
                resources,
                BrowserObjectCounts::default(),
                100,
                1_000,
            ))
            .expect("baseline should record");
        let changed = MemoryObservation {
            phase: MemoryWorkloadPhase::Warmup,
            monotonic_millis: 10,
            process: process_with_roots(11, 20, 100, 1_000),
            resources,
            objects: BrowserObjectCounts::default(),
        };

        // Act
        let error = recorder
            .record(changed)
            .expect_err("changed roots should fail");

        // Assert
        assert!(matches!(
            error,
            MemoryWorkloadError::ProcessRootsChanged {
                expected_runtime: 10,
                actual_runtime: 11,
                expected_browser: 20,
                actual_browser: 20,
            }
        ));
    }

    #[test]
    fn teardown_must_finish_within_the_declared_deadline() {
        // Arrange
        let clean_ledger = ledger();
        let post = clean_ledger
            .metrics()
            .expect("post-teardown metrics should be readable");
        let recorder = record_complete_workload(81, post, BrowserObjectCounts::default());

        // Act
        let error = recorder
            .finish()
            .expect_err("slow teardown should exceed the deadline");

        // Assert
        assert_eq!(
            error,
            MemoryWorkloadError::TeardownDeadlineExceeded {
                elapsed_millis: 51,
                limit_millis: 50,
            }
        );
    }

    #[test]
    fn observation_time_cannot_move_backward() {
        // Arrange
        let ledger = ledger();
        let resources = ledger.metrics().expect("metrics should be readable");
        let mut recorder = MemoryWorkloadRecorder::new(plan()).expect("recorder should allocate");
        recorder
            .record(observation(
                MemoryWorkloadPhase::Baseline,
                10,
                resources,
                BrowserObjectCounts::default(),
                100,
                1_000,
            ))
            .expect("baseline should record");
        let backward = observation(
            MemoryWorkloadPhase::Warmup,
            9,
            resources,
            BrowserObjectCounts::default(),
            100,
            1_000,
        );

        // Act
        let error = recorder
            .record(backward)
            .expect_err("backward time should fail");

        // Assert
        assert_eq!(
            error,
            MemoryWorkloadError::NonMonotonicTime {
                previous: 10,
                actual: 9,
            }
        );
    }

    #[test]
    fn observations_must_follow_the_planned_phase_order() {
        // Arrange
        let ledger = ledger();
        let resources = ledger.metrics().expect("metrics should be readable");
        let mut recorder = MemoryWorkloadRecorder::new(plan()).expect("recorder should allocate");
        let out_of_order = observation(
            MemoryWorkloadPhase::Steady,
            0,
            resources,
            BrowserObjectCounts::default(),
            100,
            1_000,
        );

        // Act
        let error = recorder
            .record(out_of_order)
            .expect_err("steady state cannot replace the baseline");

        // Assert
        assert_eq!(
            error,
            MemoryWorkloadError::UnexpectedPhase {
                expected: MemoryWorkloadPhase::Baseline,
                actual: MemoryWorkloadPhase::Steady,
            }
        );
    }

    #[test]
    fn incomplete_workload_cannot_produce_a_report() {
        // Arrange
        let recorder = MemoryWorkloadRecorder::new(plan()).expect("recorder should allocate");

        // Act
        let error = recorder
            .finish()
            .expect_err("incomplete workload should fail");

        // Assert
        assert_eq!(
            error,
            MemoryWorkloadError::Incomplete {
                actual: 0,
                expected: 5,
            }
        );
    }

    #[test]
    fn plan_rejects_total_observations_above_capacity() {
        // Arrange
        let constrained = MemoryWorkloadLimits::new(4, 2).expect("limits should be valid");

        // Act
        let error =
            MemoryWorkloadPlan::new(MemoryWorkloadKind::Navigation, 1, 1, 10, 50, constrained)
                .expect_err("five observations should exceed capacity four");

        // Assert
        assert_eq!(
            error,
            MemoryWorkloadError::ObservationCapacityExceeded {
                actual: 5,
                limit: 4,
            }
        );
    }

    #[test]
    fn plan_rejects_a_zero_sample_interval() {
        // Arrange
        let limits = limits();

        // Act
        let error = MemoryWorkloadPlan::new(MemoryWorkloadKind::Idle, 0, 1, 0, 50, limits)
            .expect_err("zero sample interval should fail");

        // Assert
        assert_eq!(error, MemoryWorkloadError::InvalidSampleInterval);
    }

    #[test]
    fn leaked_browser_objects_fail_logical_baseline_check() {
        // Arrange
        let clean_ledger = ledger();
        let post = clean_ledger
            .metrics()
            .expect("post-teardown metrics should be readable");
        let recorder = record_complete_workload(
            40,
            post,
            BrowserObjectCounts {
                handles: 1,
                ..BrowserObjectCounts::default()
            },
        );
        let report = recorder.finish().expect("workload should finish");

        // Act
        let returned = report.logical_state_returned_to_baseline();

        // Assert
        assert!(!returned);
    }

    #[test]
    fn temporary_browser_processes_must_exit_to_settle_the_tree() {
        // Arrange
        let clean_ledger = ledger();
        let post = clean_ledger
            .metrics()
            .expect("post-teardown metrics should be readable");
        let recorder = record_complete_workload(40, post, BrowserObjectCounts::default());
        let mut report = recorder.finish().expect("workload should finish");
        report
            .observations
            .last_mut()
            .expect("report should have post-teardown")
            .process
            .browser
            .total
            .processes = 2;

        // Act
        let settled = report.browser_process_tree_returned_to_baseline();

        // Assert
        assert!(!settled);
    }

    fn complete_series_report(
        runtime_baseline: u64,
        runtime_post: u64,
        browser_baseline: u64,
        browser_post: u64,
    ) -> MemoryWorkloadReport {
        let clean_ledger = ledger();
        let post = clean_ledger
            .metrics()
            .expect("post-teardown metrics should be readable");
        let recorder = record_complete_workload(40, post, BrowserObjectCounts::default());
        let mut report = recorder.finish().expect("workload should finish");
        report.observations[0].process.runtime.resident_bytes = runtime_baseline;
        report.observations[0].process.browser.total.resident_bytes = browser_baseline;
        let post = report
            .observations
            .last_mut()
            .expect("complete report should have post-teardown");
        post.process.runtime.resident_bytes = runtime_post;
        post.process.browser.total.resident_bytes = browser_post;
        report
    }

    fn event_payload_resources(bytes: u64) -> ResourceMetrics {
        let ledger = ResourceLedger::new(
            ResourceLimits::deny_all(2, 2)
                .with_class(ResourceClass::EventItem, 1, 1)
                .with_class(ResourceClass::EventBytes, 64, 64),
        );
        let _event = ledger
            .reserve_many(
                ResourceScope::Connection,
                &[
                    pwright_core::ResourceRequest::new(ResourceClass::EventItem, 1),
                    pwright_core::ResourceRequest::new(ResourceClass::EventBytes, bytes),
                ],
            )
            .expect("event payload should fit");
        ledger.metrics().expect("event metrics should be readable")
    }

    fn variable_event_payload_report(plan: MemoryWorkloadPlan) -> MemoryWorkloadReport {
        let small = event_payload_resources(10);
        let large = event_payload_resources(20);
        let mut recorder = MemoryWorkloadRecorder::new(plan).expect("recorder should allocate");
        for (phase, millis, resources) in [
            (MemoryWorkloadPhase::Baseline, 0, small),
            (MemoryWorkloadPhase::Warmup, 10, small),
            (MemoryWorkloadPhase::Steady, 20, large),
            (MemoryWorkloadPhase::PreTeardown, 30, large),
            (MemoryWorkloadPhase::PostTeardown, 40, large),
        ] {
            recorder
                .record(observation(
                    phase,
                    millis,
                    resources,
                    BrowserObjectCounts::default(),
                    100,
                    1_000,
                ))
                .expect("observation should record");
        }
        recorder.finish().expect("workload should finish")
    }

    fn series_plan(runs: usize) -> MemoryWorkloadSeriesPlan {
        MemoryWorkloadSeriesPlan::new(
            plan(),
            runs,
            MemoryWorkloadSeriesLimits::new(4).expect("series limits should validate"),
        )
        .expect("series plan should validate")
    }

    #[test]
    fn complete_series_reports_diagnostic_rss_ranges() {
        // Arrange
        let mut recorder = MemoryWorkloadSeriesRecorder::new(series_plan(3))
            .expect("series recorder should allocate");
        let reports = [
            complete_series_report(100, 130, 1_000, 900),
            complete_series_report(200, 210, 2_000, 2_050),
            complete_series_report(300, 280, 3_000, 2_800),
        ];
        for report in reports {
            recorder
                .record(report)
                .expect("clean matching run should record");
        }

        // Act
        let report = recorder.finish().expect("complete series should finish");

        // Assert
        assert_eq!(report.reports().len(), 3);
        assert_eq!(report.runtime_rss_delta_range(), (-20, 30));
        assert_eq!(report.browser_rss_sum_delta_range(), (-200, 50));
    }

    #[test]
    fn series_rejects_a_different_workload_plan_without_consuming_capacity() {
        // Arrange
        let mut recorder = MemoryWorkloadSeriesRecorder::new(series_plan(1))
            .expect("series recorder should allocate");
        let mut report = complete_series_report(100, 100, 1_000, 1_000);
        report.plan.kind = MemoryWorkloadKind::Navigation;

        // Act
        let error = recorder
            .record(report)
            .expect_err("a mismatched workload plan should fail");

        // Assert
        assert!(matches!(
            error,
            MemoryWorkloadSeriesError::WorkloadPlanMismatch { .. }
        ));
        assert_eq!(recorder.completed_runs(), 0);
    }

    #[test]
    fn series_rejects_a_run_with_unreleased_logical_state() {
        // Arrange
        let mut recorder = MemoryWorkloadSeriesRecorder::new(series_plan(1))
            .expect("series recorder should allocate");
        let mut report = complete_series_report(100, 100, 1_000, 1_000);
        report
            .observations
            .last_mut()
            .expect("complete report should have post-teardown")
            .objects
            .handles = 1;

        // Act
        let error = recorder
            .record(report)
            .expect_err("a leaked logical object should fail");

        // Assert
        assert_eq!(
            error,
            MemoryWorkloadSeriesError::LogicalStateNotReturned { run: 0 }
        );
        assert_eq!(recorder.completed_runs(), 0);
    }

    #[test]
    fn series_rejects_a_run_before_the_browser_process_tree_settles() {
        // Arrange
        let mut recorder = MemoryWorkloadSeriesRecorder::new(series_plan(1))
            .expect("series recorder should allocate");
        let mut report = complete_series_report(100, 100, 1_000, 1_000);
        report
            .observations
            .last_mut()
            .expect("complete report should have post-teardown")
            .process
            .browser
            .total
            .processes = 2;

        // Act
        let error = recorder
            .record(report)
            .expect_err("an unsettled native process tree should fail");

        // Assert
        assert_eq!(
            error,
            MemoryWorkloadSeriesError::BrowserProcessTreeNotSettled { run: 0 }
        );
        assert_eq!(recorder.completed_runs(), 0);
    }

    #[test]
    fn series_plan_and_completion_are_bounded() {
        // Arrange
        let limits = MemoryWorkloadSeriesLimits::new(2).expect("series limits should validate");

        // Act
        let capacity_error = MemoryWorkloadSeriesPlan::new(plan(), 3, limits)
            .expect_err("three runs should exceed capacity two");
        let recorder = MemoryWorkloadSeriesRecorder::new(series_plan(2))
            .expect("series recorder should allocate");
        let incomplete_error = recorder
            .finish()
            .expect_err("an empty two-run series should be incomplete");

        // Assert
        assert_eq!(
            capacity_error,
            MemoryWorkloadSeriesError::RunCapacityExceeded {
                actual: 3,
                limit: 2,
            }
        );
        assert_eq!(
            incomplete_error,
            MemoryWorkloadSeriesError::Incomplete {
                actual: 0,
                expected: 2,
            }
        );
    }

    #[test]
    fn reusable_variable_payload_requires_explicit_byte_class_policy() {
        // Arrange
        let strict = variable_event_payload_report(plan());
        let reusable_plan = plan()
            .with_reusable_variable_payload_class(ResourceClass::EventBytes)
            .expect("event bytes should be eligible");
        let reusable = variable_event_payload_report(reusable_plan);

        // Act
        let strict_returned = strict.logical_state_returned_to_baseline();
        let reusable_returned = reusable.logical_state_returned_to_baseline();
        let invalid = plan()
            .with_reusable_variable_payload_class(ResourceClass::EventItem)
            .expect_err("an item count must not be treated as variable payload bytes");

        // Assert
        assert!(!strict_returned);
        assert!(reusable_returned);
        assert_eq!(
            invalid,
            MemoryWorkloadError::InvalidReusablePayloadClass(ResourceClass::EventItem)
        );
        assert_eq!(
            reusable
                .post_teardown()
                .resources
                .class(ResourceClass::EventItem)
                .current,
            reusable
                .comparison_baseline()
                .resources
                .class(ResourceClass::EventItem)
                .current
        );
    }
}
