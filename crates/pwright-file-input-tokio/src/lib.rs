//! Bounded optional Tokio adapter for synchronous file-input path preparation.
//! Submission admits both a task slot and request storage before copying input
//! or calling spawn_blocking. No local unbounded queue or per-job runtime.

use pwright_core::{
    ProductionResourceError, ProductionResourceLedger, ResourceClass, ResourceLease,
    ResourceLedger, ResourceRequest, ResourceRequirements, ResourceScope,
};
use pwright_file_input_fs::{
    FileSystem, FsError, MimeLookup, PathServices, PreparedInput, PreparedPaths,
};
use pwright_file_input_model::{BrowserLocation, FileInputError, FileInputLimits};
use pwright_file_input_source::SourceError;
use std::{
    fmt,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};
use tokio::{
    runtime::Handle,
    sync::{Notify, oneshot},
};

mod filesystem;
use filesystem::CancellableFileSystem;
pub use filesystem::CancellableReader;

const CLOSED: usize = 1 << (usize::BITS - 1);

#[derive(Debug)]
pub enum WorkError {
    InvalidProfile,
    Closed,
    Capacity,
    Cancelled,
    WorkerStopped,
    WorkerPanicked,
    Plan(ProductionResourceError),
    Source(SourceError),
    FileSystem(FsError),
}
impl fmt::Display for WorkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile => f.write_str("invalid file-input executor profile"),
            Self::Closed => f.write_str("file-input executor is closed"),
            Self::Capacity => f.write_str("file-input task capacity exhausted"),
            Self::Cancelled => f.write_str("file-input preparation cancelled"),
            Self::WorkerStopped => f.write_str("file-input blocking runtime stopped"),
            Self::WorkerPanicked => f.write_str("file-input worker panicked"),
            Self::Plan(error) => error.fmt(f),
            Self::Source(error) => error.fmt(f),
            Self::FileSystem(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for WorkError {}

#[derive(Clone, Copy, Debug)]
pub struct WorkProfile {
    capacity: usize,
    limits: FileInputLimits,
    scope: ResourceScope,
    job_bytes: u64,
}
impl WorkProfile {
    /// Runtime/channel overhead is explicit embedding policy, not a guessed
    /// exact heap measurement. Tokio thread stacks belong to the runtime owner.
    pub fn new(
        capacity: usize,
        limits: FileInputLimits,
        scope: ResourceScope,
        runtime_overhead_bytes: u64,
    ) -> Result<Self, WorkError> {
        if capacity == 0 || capacity >= CLOSED || runtime_overhead_bytes == 0 {
            return Err(WorkError::InvalidProfile);
        }
        let bytes = limits
            .max_path_bytes()
            .checked_mul(
                limits
                    .max_items()
                    .checked_add(1)
                    .ok_or(WorkError::InvalidProfile)?,
            )
            .and_then(|n| {
                limits
                    .max_items()
                    .checked_mul(std::mem::size_of::<String>() + std::mem::size_of::<&str>())
                    .and_then(|views| n.checked_add(views))
            })
            .and_then(|n| {
                n.checked_add(
                    std::mem::size_of::<Request>()
                        + std::mem::size_of::<JobState>()
                        + 2 * std::mem::size_of::<usize>(),
                )
            })
            .and_then(|n| u64::try_from(n).ok())
            .and_then(|n| n.checked_add(runtime_overhead_bytes))
            .ok_or(WorkError::InvalidProfile)?;
        let result = Self {
            capacity,
            limits,
            scope,
            job_bytes: bytes,
        };
        result.requirements()?;
        Ok(result)
    }
    pub fn requirements(self) -> Result<ResourceRequirements, WorkError> {
        let count = u64::try_from(self.capacity).map_err(|_| WorkError::InvalidProfile)?;
        let bytes = self
            .job_bytes
            .checked_mul(count)
            .and_then(|n| n.checked_add(pool_bytes()))
            .ok_or(WorkError::InvalidProfile)?;
        Ok(ResourceRequirements::none()
            .with_class(ResourceClass::ActiveCommand, count, count)
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(
                1,
                self.capacity
                    .checked_add(1)
                    .ok_or(WorkError::InvalidProfile)?,
            ))
    }
}
struct Pool {
    profile: WorkProfile,
    admission: AtomicUsize,
    drained: Notify,
    queue_ledger: ResourceLedger,
    source_ledger: ResourceLedger,
    runtime: Handle,
    _lease: ResourceLease,
}
fn pool_bytes() -> u64 {
    (std::mem::size_of::<Pool>() + 2 * std::mem::size_of::<usize>() + 512) as u64
}
struct Slot {
    pool: Arc<Pool>,
    lease: Option<ResourceLease>,
}
impl Drop for Slot {
    fn drop(&mut self) {
        // Drain cannot report this slot released while its byte lease is live.
        drop(self.lease.take());
        let previous = self.pool.admission.fetch_sub(1, Ordering::AcqRel);
        if previous & !CLOSED == 1 {
            self.pool.drained.notify_waiters();
        }
    }
}
struct JobState {
    cancelled: AtomicBool,
    slot: Slot,
}
impl JobState {
    fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
            || self.slot.pool.admission.load(Ordering::Acquire) & CLOSED != 0
    }
}

struct Request {
    base: PathBuf,
    paths: Vec<String>,
    location: BrowserLocation,
}

pub struct FileInputExecutor<F, M> {
    pool: Arc<Pool>,
    filesystem: Arc<F>,
    mime: Arc<M>,
}
impl<F, M> Drop for FileInputExecutor<F, M> {
    fn drop(&mut self) {
        self.pool.admission.fetch_or(CLOSED, Ordering::AcqRel);
    }
}
impl<F: FileSystem + Send + Sync + 'static, M: MimeLookup + Send + Sync + 'static>
    FileInputExecutor<F, M>
where
    F::Reader: Send + 'static,
{
    pub fn new(
        profile: WorkProfile,
        queue_plan: &ProductionResourceLedger,
        source_ledger: ResourceLedger,
        runtime: Handle,
        filesystem: Arc<F>,
        mime: Arc<M>,
    ) -> Result<Self, WorkError> {
        let ledger = queue_plan
            .claim(profile.requirements()?)
            .map_err(WorkError::Plan)?;
        let lease = ledger
            .reserve(
                profile.scope,
                ResourceClass::BackendStateBytes,
                pool_bytes(),
            )
            .map_err(|e| WorkError::Source(SourceError::Budget(e)))?;
        Ok(Self {
            pool: Arc::new(Pool {
                profile,
                admission: AtomicUsize::new(0),
                drained: Notify::new(),
                queue_ledger: ledger,
                source_ledger,
                runtime,
                _lease: lease,
            }),
            filesystem,
            mime,
        })
    }
    /// Includes queued/running work and completed results not yet consumed.
    pub fn outstanding(&self) -> usize {
        self.pool.admission.load(Ordering::Acquire) & !CLOSED
    }
    pub fn close(&self) {
        self.pool.admission.fetch_or(CLOSED, Ordering::AcqRel);
    }
    /// The embedding caller must consume or drop returned Jobs as well. Merely
    /// closing admission does not take ownership of somebody else's receiver.
    pub async fn close_and_drain(&self) {
        self.close();
        loop {
            let notified = self.pool.drained.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.outstanding() == 0 {
                return;
            }
            notified.await;
        }
    }
    fn admit(&self) -> Result<Arc<JobState>, WorkError> {
        let profile = self.pool.profile;
        self.pool
            .admission
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                (state & CLOSED == 0 && state < profile.capacity).then(|| state + 1)
            })
            .map_err(|state| {
                if state & CLOSED != 0 {
                    WorkError::Closed
                } else {
                    WorkError::Capacity
                }
            })?;
        let mut slot = Slot {
            pool: self.pool.clone(),
            lease: None,
        };
        slot.lease = Some(
            self.pool
                .queue_ledger
                .reserve_many(
                    profile.scope,
                    &[
                        ResourceRequest::new(ResourceClass::ActiveCommand, 1),
                        ResourceRequest::new(ResourceClass::BackendStateBytes, profile.job_bytes),
                    ],
                )
                .map_err(|e| WorkError::Source(SourceError::Budget(e)))?,
        );
        Ok(Arc::new(JobState {
            cancelled: AtomicBool::new(false),
            slot,
        }))
    }
    pub fn submit_paths(
        &self,
        base: &Path,
        paths: &[&str],
        location: BrowserLocation,
    ) -> Result<FileInputJob<F::Reader>, WorkError> {
        let limits = self.pool.profile.limits;
        if paths.len() > limits.max_items() {
            return Err(WorkError::Source(SourceError::Policy(
                FileInputError::CountLimit,
            )));
        }
        if !base.is_absolute() || base.to_str().is_none() {
            return Err(WorkError::FileSystem(FsError::InvalidBase));
        }
        if base.as_os_str().len() > limits.max_path_bytes()
            || paths
                .iter()
                .any(|path| path.len() > limits.max_path_bytes())
        {
            return Err(WorkError::Source(SourceError::Policy(
                FileInputError::PathByteLimit,
            )));
        }
        let state = self.admit()?;
        let mut owned_base = PathBuf::new();
        owned_base
            .try_reserve_exact(base.as_os_str().len())
            .map_err(|e| WorkError::Source(SourceError::Allocation(e)))?;
        owned_base.push(base);
        let mut owned_paths = Vec::new();
        owned_paths
            .try_reserve_exact(paths.len())
            .map_err(|e| WorkError::Source(SourceError::Allocation(e)))?;
        for path in paths {
            let mut owned = String::new();
            owned
                .try_reserve_exact(path.len())
                .map_err(|e| WorkError::Source(SourceError::Allocation(e)))?;
            owned.push_str(path);
            owned_paths.push(owned);
        }
        let request = Request {
            base: owned_base,
            paths: owned_paths,
            location,
        };
        let (sender, receiver) = oneshot::channel();
        let filesystem = self.filesystem.clone();
        let mime = self.mime.clone();
        let worker_state = state.clone();
        self.pool.runtime.spawn_blocking(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                execute(&worker_state, request, filesystem, mime)
            }))
            .unwrap_or(Err(WorkError::WorkerPanicked));
            // A dropped receiver returns the owned result here; dropping that
            // error releases source storage before worker_state's task slot.
            drop(sender.send(result));
        });
        Ok(FileInputJob {
            receiver,
            state: Some(state),
        })
    }
}

fn execute<F: FileSystem, M: MimeLookup>(
    state: &Arc<JobState>,
    request: Request,
    filesystem: Arc<F>,
    mime: Arc<M>,
) -> Result<PreparedInput<CancellableReader<F::Reader>>, WorkError> {
    if state.cancelled() {
        return Err(WorkError::Cancelled);
    }
    let pool = &state.slot.pool;
    let checked = CancellableFileSystem {
        filesystem,
        state: state.clone(),
    };
    let mut paths = Vec::new();
    paths
        .try_reserve_exact(request.paths.len())
        .map_err(|e| WorkError::Source(SourceError::Allocation(e)))?;
    paths.extend(request.paths.iter().map(String::as_str));
    let result = PreparedPaths::prepare(
        &pool.source_ledger,
        pool.profile.scope,
        pool.profile.limits,
        &paths,
        request.location,
        PathServices {
            base: &request.base,
            filesystem: &checked,
            mime: mime.as_ref(),
        },
    )
    .and_then(|prepared| prepared.materialize(&checked));
    if state.cancelled() {
        return Err(WorkError::Cancelled);
    }
    result.map_err(WorkError::FileSystem)
}

pub struct FileInputJob<R> {
    receiver: oneshot::Receiver<Result<PreparedInput<CancellableReader<R>>, WorkError>>,
    state: Option<Arc<JobState>>,
}
impl<R> FileInputJob<R> {
    /// A value is buffered for this receiver. A stopped worker without a value
    /// can still settle the Future with an error; false does not imply liveness.
    pub fn has_result(&self) -> bool {
        !self.receiver.is_empty()
    }
    pub fn cancel(&self) {
        if let Some(state) = &self.state {
            state.cancelled.store(true, Ordering::Release);
        }
    }
}
impl<R> Drop for FileInputJob<R> {
    fn drop(&mut self) {
        self.cancel();
        self.receiver.close();
    }
}
impl<R> Future for FileInputJob<R> {
    type Output = Result<PreparedInput<CancellableReader<R>>, WorkError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match Pin::new(&mut this.receiver).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                let cancelled = this.state.as_ref().is_some_and(|state| state.cancelled());
                let output = if cancelled {
                    drop(result);
                    Err(WorkError::Cancelled)
                } else {
                    result.unwrap_or(Err(WorkError::WorkerStopped))
                };
                this.state.take();
                Poll::Ready(output)
            }
        }
    }
}

#[cfg(test)]
mod tests;
