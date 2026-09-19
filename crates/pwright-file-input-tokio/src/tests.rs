use super::*;
use pwright_core::ResourceLimits;
use pwright_file_input_fs::{FileObservation, PinnedMime, SystemFileSystem};
use pwright_file_input_model::PathKind;
use std::{
    io::{self, Read},
    sync::{Condvar, Mutex},
};

struct Gate {
    entered: Mutex<Option<oneshot::Sender<()>>>,
    released: Mutex<bool>,
    resume: Condvar,
}
struct GateReleaseGuard(Arc<Gate>);
impl std::ops::Deref for GateReleaseGuard {
    type Target = Arc<Gate>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Drop for GateReleaseGuard {
    fn drop(&mut self) {
        self.0.release();
    }
}
impl Gate {
    fn paired() -> (GateReleaseGuard, oneshot::Receiver<()>) {
        let (tx, rx) = oneshot::channel();
        (
            GateReleaseGuard(Arc::new(Self {
                entered: Mutex::new(Some(tx)),
                released: Mutex::new(false),
                resume: Condvar::new(),
            })),
            rx,
        )
    }
    fn wait(&self) {
        if let Some(sender) = self.entered.lock().unwrap().take() {
            sender.send(()).unwrap();
        }
        let mut released = self.released.lock().unwrap();
        while !*released {
            released = self.resume.wait(released).unwrap();
        }
    }
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.resume.notify_all();
    }
}
struct FakeFs {
    size: u64,
    stats: AtomicUsize,
    opens: AtomicUsize,
    readers: Arc<AtomicUsize>,
    reads: Arc<AtomicUsize>,
    max_read: Arc<AtomicUsize>,
    stat_gate: Option<Arc<Gate>>,
    read_gate: Option<Arc<Gate>>,
    panic: bool,
}
impl FakeFs {
    fn new() -> Self {
        Self {
            size: 2,
            stats: AtomicUsize::new(0),
            opens: AtomicUsize::new(0),
            readers: Arc::new(AtomicUsize::new(0)),
            reads: Arc::new(AtomicUsize::new(0)),
            max_read: Arc::new(AtomicUsize::new(0)),
            stat_gate: None,
            read_gate: None,
            panic: false,
        }
    }
}
struct FakeReader {
    remaining: u64,
    live: Arc<AtomicUsize>,
    reads: Arc<AtomicUsize>,
    max_read: Arc<AtomicUsize>,
    gate: Option<Arc<Gate>>,
}
impl Drop for FakeReader {
    fn drop(&mut self) {
        self.live.fetch_sub(1, Ordering::SeqCst);
    }
}
impl Read for FakeReader {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if let Some(gate) = self.gate.take() {
            gate.wait();
        }
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.max_read.fetch_max(bytes.len(), Ordering::SeqCst);
        let length = bytes.len().min(self.remaining as usize);
        bytes[..length].fill(42);
        self.remaining -= length as u64;
        Ok(length)
    }
}
impl FileSystem for FakeFs {
    type Reader = FakeReader;
    fn stat(&self, _: &Path) -> Result<FileObservation, FsError> {
        self.stats.fetch_add(1, Ordering::SeqCst);
        if let Some(gate) = &self.stat_gate {
            gate.wait();
        }
        assert!(!self.panic, "injected worker panic");
        Ok(FileObservation {
            kind: PathKind::File {
                byte_length: self.size,
            },
            modified_ms: 123.5,
        })
    }
    fn open(&self, _: &Path) -> Result<Self::Reader, FsError> {
        self.opens.fetch_add(1, Ordering::SeqCst);
        self.readers.fetch_add(1, Ordering::SeqCst);
        Ok(FakeReader {
            remaining: self.size,
            live: self.readers.clone(),
            reads: self.reads.clone(),
            max_read: self.max_read.clone(),
            gate: self.read_gate.clone(),
        })
    }
}
fn limits() -> FileInputLimits {
    FileInputLimits::new(4, 512, 128, 128, 2048).unwrap()
}
fn profile(capacity: usize) -> WorkProfile {
    WorkProfile::new(capacity, limits(), ResourceScope::Connection, 4096).unwrap()
}
fn queue_plan(profile: WorkProfile) -> ProductionResourceLedger {
    let requirements = profile.requirements().unwrap();
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let amount = requirements.class(class);
        limits = limits.with_class(class, amount.global, amount.per_scope);
    }
    ProductionResourceLedger::new(limits, requirements).unwrap()
}
fn source_ledger() -> ResourceLedger {
    ResourceLedger::new(
        ResourceLimits::deny_all(4, 100)
            .with_class(ResourceClass::LiveObject, 100, 100)
            .with_class(ResourceClass::BackendStateBytes, 1024 * 1024, 1024 * 1024)
            .with_class(ResourceClass::ArtifactBytes, 1024 * 1024, 1024 * 1024),
    )
}
fn executor(
    fs: Arc<FakeFs>,
    capacity: usize,
) -> (
    FileInputExecutor<FakeFs, PinnedMime>,
    ProductionResourceLedger,
    ResourceLedger,
) {
    let profile = profile(capacity);
    let queue = queue_plan(profile);
    let source = source_ledger();
    let executor = FileInputExecutor::new(
        profile,
        &queue,
        source.clone(),
        Handle::current(),
        fs,
        Arc::new(PinnedMime),
    )
    .unwrap();
    (executor, queue, source)
}
fn assert_source_released(ledger: &ResourceLedger) {
    assert_eq!(ledger.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn real_path_preparation_runs_through_the_admitted_executor() {
    // Arrange
    let profile = profile(1);
    let queue = queue_plan(profile);
    let sources = source_ledger();
    let executor = FileInputExecutor::new(
        profile,
        &queue,
        sources.clone(),
        Handle::current(),
        Arc::new(SystemFileSystem),
        Arc::new(PinnedMime),
    )
    .unwrap();
    // Act
    let job = executor
        .submit_paths(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            &["../pwright-file-input-source/tests/fixture.txt"],
            BrowserLocation::Separate,
        )
        .unwrap();
    let PreparedInput::Payloads(result) = job.await.unwrap() else {
        panic!("expected file bytes")
    };
    // Assert
    assert_eq!(
        result.entries()[0].bytes(),
        include_bytes!("../../pwright-file-input-source/tests/fixture.txt")
    );
    assert_eq!(result.entries()[0].mime_type(), "text/plain");
    drop(result);
    executor.close_and_drain().await;
    assert_source_released(&sources);
    drop(executor);
    assert_eq!(queue.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn capacity_rejection_does_not_enqueue_or_touch_the_filesystem() {
    // Arrange
    let (gate, entered) = Gate::paired();
    let mut fs = FakeFs::new();
    fs.stat_gate = Some(gate.clone());
    let fs = Arc::new(fs);
    let (executor, queue, source) = executor(fs.clone(), 1);
    let first = executor
        .submit_paths(Path::new("/base"), &["a.txt"], BrowserLocation::Separate)
        .unwrap();
    entered.await.unwrap();
    // Act
    let rejected = executor.submit_paths(Path::new("/base"), &["b.txt"], BrowserLocation::Separate);
    // Assert
    assert!(matches!(rejected, Err(WorkError::Capacity)));
    assert_eq!(executor.outstanding(), 1);
    assert_eq!(fs.stats.load(Ordering::SeqCst), 1);
    drop(first);
    executor.close();
    gate.release();
    executor.close_and_drain().await;
    assert_eq!(fs.opens.load(Ordering::SeqCst), 0);
    assert_source_released(&source);
    drop(executor);
    assert_eq!(queue.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn dropping_waiter_during_read_retains_slot_and_bytes_until_io_exits() {
    // Arrange
    let (gate, entered) = Gate::paired();
    let mut fs = FakeFs::new();
    fs.read_gate = Some(gate.clone());
    fs.size = 131073;
    let fs = Arc::new(fs);
    let (executor, queue, source) = executor(fs.clone(), 1);
    let job = executor
        .submit_paths(Path::new("/base"), &["a.txt"], BrowserLocation::Separate)
        .unwrap();
    entered.await.unwrap();
    // Act
    drop(job);
    // Assert
    assert_eq!(executor.outstanding(), 1);
    assert_eq!(
        source
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current,
        131073
    );
    assert_eq!(fs.readers.load(Ordering::SeqCst), 1);
    gate.release();
    executor.close_and_drain().await;
    assert_eq!(fs.reads.load(Ordering::SeqCst), 1);
    assert_eq!(fs.readers.load(Ordering::SeqCst), 0);
    assert_source_released(&source);
    drop(executor);
    assert_eq!(queue.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn explicit_cancel_is_reported_after_active_io_returns() {
    // Arrange
    let (gate, entered) = Gate::paired();
    let mut fs = FakeFs::new();
    fs.read_gate = Some(gate.clone());
    let fs = Arc::new(fs);
    let (executor, _, source) = executor(fs, 1);
    let job = executor
        .submit_paths(Path::new("/base"), &["a.txt"], BrowserLocation::Separate)
        .unwrap();
    entered.await.unwrap();
    // Act
    job.cancel();
    gate.release();
    let result = job.await;
    // Assert
    assert!(matches!(result, Err(WorkError::Cancelled)));
    executor.close_and_drain().await;
    assert_source_released(&source);
}

#[tokio::test]
async fn close_rejects_new_work_and_cancels_before_the_next_file_operation() {
    // Arrange
    let (gate, entered) = Gate::paired();
    let mut fs = FakeFs::new();
    fs.stat_gate = Some(gate.clone());
    let fs = Arc::new(fs);
    let (executor, _, source) = executor(fs.clone(), 2);
    let job = executor
        .submit_paths(Path::new("/base"), &["a.txt"], BrowserLocation::Separate)
        .unwrap();
    entered.await.unwrap();
    // Act
    executor.close();
    let rejected = executor.submit_paths(Path::new("/base"), &["b.txt"], BrowserLocation::Separate);
    // Assert
    assert!(matches!(rejected, Err(WorkError::Closed)));
    gate.release();
    assert!(matches!(job.await, Err(WorkError::Cancelled)));
    executor.close_and_drain().await;
    assert_eq!(fs.opens.load(Ordering::SeqCst), 0);
    assert_source_released(&source);
}

#[tokio::test]
async fn worker_panic_is_classified_and_reclaims_admission() {
    // Arrange
    let mut fs = FakeFs::new();
    fs.panic = true;
    let fs = Arc::new(fs);
    let (executor, queue, source) = executor(fs, 1);
    // Act
    let result = executor
        .submit_paths(Path::new("/base"), &["a.txt"], BrowserLocation::Separate)
        .unwrap()
        .await;
    // Assert
    assert!(matches!(result, Err(WorkError::WorkerPanicked)));
    executor.close_and_drain().await;
    assert_source_released(&source);
    drop(executor);
    assert_eq!(queue.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn successful_large_reader_never_receives_more_than_one_chunk() {
    // Arrange
    let mut fs = FakeFs::new();
    fs.size = 131073;
    let fs = Arc::new(fs);
    let (executor, _, source) = executor(fs.clone(), 1);
    // Act
    let PreparedInput::Payloads(result) = executor
        .submit_paths(Path::new("/base"), &["a.txt"], BrowserLocation::Separate)
        .unwrap()
        .await
        .unwrap()
    else {
        panic!("expected payload")
    };
    // Assert
    assert_eq!(result.entries()[0].bytes().len(), 131073);
    assert_eq!(fs.max_read.load(Ordering::SeqCst), 65536);
    assert_eq!(fs.reads.load(Ordering::SeqCst), 4);
    drop(result);
    executor.close_and_drain().await;
    assert_source_released(&source);
}

#[tokio::test]
async fn queue_budget_rejection_rolls_back_the_reserved_task_slot() {
    // Arrange
    let fs = Arc::new(FakeFs::new());
    let (executor, queue, source) = executor(fs.clone(), 1);
    let ledger = queue.claim(profile(1).requirements().unwrap()).unwrap();
    let occupied = ledger
        .reserve(ResourceScope::Connection, ResourceClass::ActiveCommand, 1)
        .unwrap();
    // Act
    let result = executor.submit_paths(Path::new("/base"), &["a.txt"], BrowserLocation::Separate);
    // Assert
    assert!(matches!(
        result,
        Err(WorkError::Source(SourceError::Budget(_)))
    ));
    assert_eq!(executor.outstanding(), 0);
    assert_eq!(fs.stats.load(Ordering::SeqCst), 0);
    drop(occupied);
    executor.close_and_drain().await;
    assert_source_released(&source);
}

#[tokio::test]
async fn dropping_executor_closes_admission_for_retained_jobs() {
    // Arrange
    let (gate, entered) = Gate::paired();
    let mut fs = FakeFs::new();
    fs.stat_gate = Some(gate.clone());
    let (executor, queue, source) = executor(Arc::new(fs), 1);
    let job = executor
        .submit_paths(Path::new("/base"), &["a.txt"], BrowserLocation::Separate)
        .unwrap();
    entered.await.unwrap();
    // Act
    drop(executor);
    gate.release();
    let result = job.await;
    // Assert
    assert!(matches!(result, Err(WorkError::Cancelled)));
    // Wait for the worker's final Arc release without claiming the await itself
    // has already destroyed its enclosing blocking-task capture.
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while queue.metrics().unwrap().active_leases != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("worker final ownership should release");
    assert_source_released(&source);
}

#[test]
fn cancelled_queued_job_does_not_begin_filesystem_work() {
    // Arrange
    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let (gate, entered) = Gate::paired();
        let blocker_gate = gate.clone();
        let blocker = Handle::current().spawn_blocking(move || blocker_gate.wait());
        entered.await.unwrap();
        let fs = Arc::new(FakeFs::new());
        let (executor, queue, source) = executor(fs.clone(), 1);
        let job = executor
            .submit_paths(Path::new("/base"), &["a.txt"], BrowserLocation::Separate)
            .unwrap();
        // Act
        drop(job);
        // Assert
        assert_eq!(executor.outstanding(), 1);
        assert_eq!(fs.stats.load(Ordering::SeqCst), 0);
        gate.release();
        blocker.await.unwrap();
        executor.close_and_drain().await;
        assert_eq!(fs.stats.load(Ordering::SeqCst), 0);
        assert_source_released(&source);
        drop(executor);
        assert_eq!(queue.metrics().unwrap().active_leases, 0);
    });
}

#[tokio::test]
async fn buffered_unconsumed_result_still_occupies_a_task_slot() {
    // Arrange
    let fs = Arc::new(FakeFs::new());
    let (executor, queue, source) = executor(fs.clone(), 1);
    let job = executor
        .submit_paths(Path::new("/base"), &["a.txt"], BrowserLocation::Separate)
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !job.has_result() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("prepared result should be buffered");
    // Act
    let result = executor.submit_paths(Path::new("/base"), &["b.txt"], BrowserLocation::Separate);
    // Assert
    assert!(matches!(result, Err(WorkError::Capacity)));
    assert_eq!(fs.opens.load(Ordering::SeqCst), 1);
    assert_eq!(
        source
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current,
        2
    );
    drop(job);
    executor.close_and_drain().await;
    assert_source_released(&source);
    drop(executor);
    assert_eq!(queue.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn consuming_result_releases_slot_even_if_completed_future_is_retained() {
    // Arrange
    let (executor, _, source) = executor(Arc::new(FakeFs::new()), 1);
    let mut job = executor
        .submit_paths(Path::new("/base"), &["a.txt"], BrowserLocation::Separate)
        .unwrap();
    // Act
    let result = (&mut job).await.unwrap();
    // Assert
    executor.close_and_drain().await;
    assert_eq!(executor.outstanding(), 0);
    assert!(!job.has_result());
    // Delivered content has an independent lease, not an unaccounted escape.
    assert_eq!(
        source
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current,
        2
    );
    drop(result);
    assert_source_released(&source);
}

#[tokio::test]
async fn stopped_runtime_returns_worker_stopped_without_leaking_a_slot() {
    // Arrange
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let handle = runtime.handle().clone();
    runtime.shutdown_background();
    let profile = profile(1);
    let queue = queue_plan(profile);
    let source = source_ledger();
    let fs = Arc::new(FakeFs::new());
    let executor = FileInputExecutor::new(
        profile,
        &queue,
        source.clone(),
        handle,
        fs.clone(),
        Arc::new(PinnedMime),
    )
    .unwrap();
    // Act
    let result = executor
        .submit_paths(Path::new("/base"), &["a.txt"], BrowserLocation::Separate)
        .unwrap()
        .await;
    // Assert
    assert!(matches!(result, Err(WorkError::WorkerStopped)));
    executor.close_and_drain().await;
    assert_eq!(fs.stats.load(Ordering::SeqCst), 0);
    assert_source_released(&source);
    drop(executor);
    assert_eq!(queue.metrics().unwrap().active_leases, 0);
}

#[test]
fn invalid_or_overflowing_executor_profiles_reject_before_allocation() {
    // Arrange
    let cases = [(0, 4096), (usize::MAX, 4096), (1, 0), (1, u64::MAX)];
    // Act
    let results: Vec<_> = cases
        .into_iter()
        .map(|(count, bytes)| WorkProfile::new(count, limits(), ResourceScope::Connection, bytes))
        .collect();
    // Assert
    assert!(
        results
            .into_iter()
            .all(|result| matches!(result, Err(WorkError::InvalidProfile)))
    );
}

#[tokio::test]
async fn omitted_executor_in_production_plan_cannot_claim_a_queue() {
    // Arrange
    let limits = ResourceLimits::deny_all(2, 8)
        .with_class(ResourceClass::ActiveCommand, 2, 2)
        .with_class(ResourceClass::BackendStateBytes, 100000, 100000);
    let queue = ProductionResourceLedger::new(limits, ResourceRequirements::none()).unwrap();
    // Act
    let result = FileInputExecutor::new(
        profile(1),
        &queue,
        source_ledger(),
        Handle::current(),
        Arc::new(FakeFs::new()),
        Arc::new(PinnedMime),
    );
    // Assert
    assert!(matches!(result, Err(WorkError::Plan(_))));
    assert_eq!(queue.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn queue_and_source_owners_can_share_one_explicit_production_plan() {
    // Arrange
    let profile = profile(1);
    let source_requirements = ResourceRequirements::none()
        .with_class(ResourceClass::LiveObject, 8, 8)
        .with_class(ResourceClass::BackendStateBytes, 100000, 100000)
        .with_class(ResourceClass::ArtifactBytes, 100, 100)
        .with_metadata(1, 4);
    let requirements = profile
        .requirements()
        .unwrap()
        .combine_concurrent(source_requirements);
    let mut configured = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let amount = requirements.class(class);
        configured = configured.with_class(class, amount.global, amount.per_scope);
    }
    let production = ProductionResourceLedger::new(configured, requirements).unwrap();
    let sources = production.claim(source_requirements).unwrap();
    let executor = FileInputExecutor::new(
        profile,
        &production,
        sources,
        Handle::current(),
        Arc::new(FakeFs::new()),
        Arc::new(PinnedMime),
    )
    .unwrap();
    // Act
    let result = executor
        .submit_paths(Path::new("/base"), &["a.txt"], BrowserLocation::Separate)
        .unwrap()
        .await
        .unwrap();
    // Assert
    assert_eq!(
        production
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current,
        2
    );
    drop(result);
    executor.close_and_drain().await;
    drop(executor);
    assert_eq!(production.metrics().unwrap().active_leases, 0);
}
