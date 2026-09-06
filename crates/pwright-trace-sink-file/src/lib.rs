//! Optional synchronous atomic-file sink for bounded Pwright trace chunks.
//!
//! The caller grants one new, sink-owned directory explicitly. Each write is
//! streamed to a create-new temporary file, synced, and atomically renamed to
//! its final name. This crate starts no task, runtime, browser, downloader,
//! archive writer, or network service.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use pwright_core::ObjectId;
use pwright_trace_format::{encoded_trace_chunk_bytes, write_trace_chunk};
use pwright_trace_model::TraceChunk;
use pwright_tracing_service::TraceSink;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileTraceSinkLimits {
    pub max_directory_path_bytes: usize,
    pub max_contexts: usize,
    pub max_chunks_per_context: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes_per_context: u64,
}

impl FileTraceSinkLimits {
    pub const fn new(
        max_directory_path_bytes: usize,
        max_contexts: usize,
        max_chunks_per_context: usize,
        max_file_bytes: u64,
        max_total_bytes_per_context: u64,
    ) -> Self {
        Self {
            max_directory_path_bytes,
            max_contexts,
            max_chunks_per_context,
            max_file_bytes,
            max_total_bytes_per_context,
        }
    }

    fn validate(self) -> Result<(), FileTraceSinkError> {
        if self.max_directory_path_bytes == 0 {
            return Err(FileTraceSinkError::ZeroLimit("directory path bytes"));
        }
        if self.max_contexts == 0 {
            return Err(FileTraceSinkError::ZeroLimit("contexts"));
        }
        if self.max_chunks_per_context == 0 {
            return Err(FileTraceSinkError::ZeroLimit("chunks per Context"));
        }
        if self.max_file_bytes == 0 {
            return Err(FileTraceSinkError::ZeroLimit("file bytes"));
        }
        if self.max_total_bytes_per_context == 0 {
            return Err(FileTraceSinkError::ZeroLimit("total bytes per Context"));
        }
        if self.max_file_bytes > self.max_total_bytes_per_context {
            return Err(FileTraceSinkError::FileLimitExceedsContextLimit {
                file: self.max_file_bytes,
                context: self.max_total_bytes_per_context,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileTraceSinkMetrics {
    pub contexts: usize,
    pub committed_chunks: usize,
    pub committed_bytes: u64,
}

#[derive(Clone, Copy, Debug)]
struct ContextUsage {
    context_id: ObjectId,
    chunks: usize,
    bytes: u64,
    next_file_id: u64,
}

struct SinkState {
    contexts: Vec<ContextUsage>,
}

pub struct FileTraceSink {
    directory: PathBuf,
    limits: FileTraceSinkLimits,
    state: Mutex<SinkState>,
}

impl FileTraceSink {
    /// Creates one fresh directory owned by this sink.
    ///
    /// The path must not already exist. This makes restart/stale-file policy a
    /// caller decision and prevents an unbounded directory scan at startup.
    pub fn create(
        directory: impl Into<PathBuf>,
        limits: FileTraceSinkLimits,
    ) -> Result<Self, FileTraceSinkError> {
        limits.validate()?;
        let directory = directory.into();
        let path_bytes = directory.as_os_str().as_encoded_bytes().len();
        if path_bytes > limits.max_directory_path_bytes {
            return Err(FileTraceSinkError::DirectoryPathTooLarge {
                actual: path_bytes,
                limit: limits.max_directory_path_bytes,
            });
        }
        fs::create_dir(&directory).map_err(|source| FileTraceSinkError::CreateDirectory {
            path: directory.clone(),
            source,
        })?;
        let mut contexts = Vec::new();
        if contexts.try_reserve_exact(limits.max_contexts).is_err() {
            let _ = fs::remove_dir(&directory);
            return Err(FileTraceSinkError::ContextAllocationFailed {
                requested: limits.max_contexts,
            });
        }
        Ok(Self {
            directory,
            limits,
            state: Mutex::new(SinkState { contexts }),
        })
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn limits(&self) -> FileTraceSinkLimits {
        self.limits
    }

    pub fn metrics(&self) -> Result<FileTraceSinkMetrics, FileTraceSinkError> {
        let state = self.lock_state()?;
        let committed_chunks = state
            .contexts
            .iter()
            .try_fold(0_usize, |total, context| total.checked_add(context.chunks))
            .ok_or(FileTraceSinkError::CounterOverflow)?;
        let committed_bytes = state
            .contexts
            .iter()
            .try_fold(0_u64, |total, context| total.checked_add(context.bytes))
            .ok_or(FileTraceSinkError::CounterOverflow)?;
        Ok(FileTraceSinkMetrics {
            contexts: state.contexts.len(),
            committed_chunks,
            committed_bytes,
        })
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, SinkState>, FileTraceSinkError> {
        self.state
            .lock()
            .map_err(|_| FileTraceSinkError::StatePoisoned)
    }
}

impl TraceSink for FileTraceSink {
    type Error = FileTraceSinkError;

    fn write(&self, context_id: ObjectId, chunk: TraceChunk) -> Result<(), Self::Error> {
        let encoded_bytes = encoded_chunk_bytes(&chunk)?;
        if encoded_bytes > self.limits.max_file_bytes {
            return Err(FileTraceSinkError::FileBytesExceeded {
                actual: encoded_bytes,
                limit: self.limits.max_file_bytes,
            });
        }
        let mut state = self.lock_state()?;
        let existing = state
            .contexts
            .iter()
            .position(|context| context.context_id == context_id);
        if existing.is_none() && state.contexts.len() == self.limits.max_contexts {
            return Err(FileTraceSinkError::ContextCapacityExceeded {
                limit: self.limits.max_contexts,
            });
        }
        let usage = existing
            .map(|index| state.contexts[index])
            .unwrap_or(ContextUsage {
                context_id,
                chunks: 0,
                bytes: 0,
                next_file_id: 1,
            });
        if usage.chunks == self.limits.max_chunks_per_context {
            return Err(FileTraceSinkError::ChunkCapacityExceeded {
                context_id,
                limit: self.limits.max_chunks_per_context,
            });
        }
        let following_bytes = usage
            .bytes
            .checked_add(encoded_bytes)
            .ok_or(FileTraceSinkError::CounterOverflow)?;
        if following_bytes > self.limits.max_total_bytes_per_context {
            return Err(FileTraceSinkError::ContextBytesExceeded {
                context_id,
                actual: following_bytes,
                limit: self.limits.max_total_bytes_per_context,
            });
        }
        let following_file_id = usage
            .next_file_id
            .checked_add(1)
            .ok_or(FileTraceSinkError::CounterOverflow)?;
        let temporary = self.directory.join(format!(
            ".context-{}-trace-{}.tmp",
            context_id.get(),
            usage.next_file_id
        ));
        let final_path = self.directory.join(format!(
            "context-{}-trace-{}.pwtrace",
            context_id.get(),
            usage.next_file_id
        ));
        write_atomic_chunk(&temporary, &final_path, context_id, &chunk)?;
        let committed = ContextUsage {
            context_id,
            chunks: usage.chunks + 1,
            bytes: following_bytes,
            next_file_id: following_file_id,
        };
        if let Some(index) = existing {
            state.contexts[index] = committed;
        } else {
            state.contexts.push(committed);
        }
        Ok(())
    }

    fn abort_context(&self, _: ObjectId) -> Result<(), Self::Error> {
        // Writes are synchronous and leave no staging file after returning.
        Ok(())
    }

    fn abort_all(&self) -> Result<(), Self::Error> {
        // Writes are synchronous and leave no staging file after returning.
        Ok(())
    }
}

fn encoded_chunk_bytes(chunk: &TraceChunk) -> Result<u64, FileTraceSinkError> {
    encoded_trace_chunk_bytes(chunk)
        .ok()
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or(FileTraceSinkError::CounterOverflow)
}

fn write_atomic_chunk(
    temporary: &Path,
    final_path: &Path,
    context_id: ObjectId,
    chunk: &TraceChunk,
) -> Result<(), FileTraceSinkError> {
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temporary)?;
        write_trace_chunk(&mut file, context_id, chunk)?;
        file.sync_all()?;
        drop(file);
        fs::rename(temporary, final_path)
    })();
    if let Err(source) = result {
        let cleanup = match fs::remove_file(temporary) {
            Ok(()) => None,
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => Some(error),
        };
        return Err(FileTraceSinkError::Write {
            path: final_path.to_path_buf(),
            source,
            cleanup,
        });
    }
    Ok(())
}

#[derive(Debug)]
pub enum FileTraceSinkError {
    ZeroLimit(&'static str),
    FileLimitExceedsContextLimit {
        file: u64,
        context: u64,
    },
    DirectoryPathTooLarge {
        actual: usize,
        limit: usize,
    },
    ContextAllocationFailed {
        requested: usize,
    },
    CreateDirectory {
        path: PathBuf,
        source: io::Error,
    },
    ContextCapacityExceeded {
        limit: usize,
    },
    ChunkCapacityExceeded {
        context_id: ObjectId,
        limit: usize,
    },
    FileBytesExceeded {
        actual: u64,
        limit: u64,
    },
    ContextBytesExceeded {
        context_id: ObjectId,
        actual: u64,
        limit: u64,
    },
    CounterOverflow,
    StatePoisoned,
    Write {
        path: PathBuf,
        source: io::Error,
        cleanup: Option<io::Error>,
    },
}

impl Display for FileTraceSinkError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit(name) => {
                write!(formatter, "file trace sink {name} limit must be non-zero")
            }
            Self::FileLimitExceedsContextLimit { file, context } => write!(
                formatter,
                "file trace limit {file} exceeds per-Context total {context}"
            ),
            Self::DirectoryPathTooLarge { actual, limit } => write!(
                formatter,
                "trace directory path has {actual} bytes, exceeding limit {limit}"
            ),
            Self::ContextAllocationFailed { requested } => write!(
                formatter,
                "could not reserve {requested} trace sink Context slots"
            ),
            Self::CreateDirectory { path, source } => write!(
                formatter,
                "could not create owned trace directory {}: {source}",
                path.display()
            ),
            Self::ContextCapacityExceeded { limit } => write!(
                formatter,
                "file trace sink reached its {limit}-Context limit"
            ),
            Self::ChunkCapacityExceeded { context_id, limit } => write!(
                formatter,
                "file trace sink Context {context_id} reached its {limit}-chunk limit"
            ),
            Self::FileBytesExceeded { actual, limit } => write!(
                formatter,
                "encoded trace file has {actual} bytes, exceeding limit {limit}"
            ),
            Self::ContextBytesExceeded {
                context_id,
                actual,
                limit,
            } => write!(
                formatter,
                "trace files for Context {context_id} would reach {actual} bytes, exceeding limit {limit}"
            ),
            Self::CounterOverflow => formatter.write_str("file trace sink counter overflowed"),
            Self::StatePoisoned => formatter.write_str("file trace sink state is poisoned"),
            Self::Write {
                path,
                source,
                cleanup,
            } => {
                write!(
                    formatter,
                    "could not atomically write {}: {source}",
                    path.display()
                )?;
                if let Some(cleanup) = cleanup {
                    write!(formatter, "; temporary cleanup failed: {cleanup}")?;
                }
                Ok(())
            }
        }
    }
}

impl Error for FileTraceSinkError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateDirectory { source, .. } | Self::Write { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use pwright_cap_tracing::{
        TraceStart, TraceStartChunk, TraceStartChunkOptions, TraceStartOptions, TraceStop,
        TraceStopChunk, TraceStopChunkMode, TraceStopChunkOptions,
    };
    use pwright_core::{
        CallId, CallMetadata, Command, ProductionResourceLedger, ResourceClass, ResourceLimits,
        ResourceScope, Service,
    };
    use pwright_trace_format::PWRTRACE_MAGIC;
    use pwright_trace_model::{
        TraceEntryKind, TraceLimits, TraceRecorder, TraceRecorderProductionProfile,
    };
    use pwright_tracing_service::{TraceClock, TracingService};

    use super::*;

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestClock(AtomicU64);

    impl TraceClock for TestClock {
        fn monotonic_micros(&self) -> u64 {
            self.0.fetch_add(1, Ordering::Relaxed)
        }
    }

    fn fresh_directory(name: &str) -> PathBuf {
        let unique = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "pwright-trace-sink-{name}-{}-{unique}",
            std::process::id()
        ))
    }

    fn sink_limits() -> FileTraceSinkLimits {
        FileTraceSinkLimits::new(1024, 1, 2, 1024, 2048)
    }

    fn profile(context_id: ObjectId) -> TraceRecorderProductionProfile {
        TraceRecorderProductionProfile::new(
            TraceLimits::new(32, 4, 64, 256, 1),
            ResourceScope::Object(context_id),
        )
        .unwrap()
    }

    fn resources(profile: TraceRecorderProductionProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let trace_bytes = requirements.class(ResourceClass::TraceBytes);
        ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::TraceBytes,
                trace_bytes.global,
                trace_bytes.per_scope,
            ),
            requirements,
        )
        .unwrap()
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

    fn chunk_with_payload(payload: &[u8]) -> (TraceChunk, pwright_core::ResourceLedger) {
        let ledger = pwright_core::ResourceLedger::new(ResourceLimits::deny_all(1, 8).with_class(
            ResourceClass::TraceBytes,
            256,
            256,
        ));
        let mut recorder = TraceRecorder::new(
            TraceLimits::new(32, 4, 128, 256, 1),
            ledger.clone(),
            ResourceScope::Connection,
        )
        .unwrap();
        recorder.start(None).unwrap();
        recorder.start_chunk(None, None).unwrap();
        recorder.append(TraceEntryKind::Log, 1, payload).unwrap();
        (recorder.stop_chunk().unwrap(), ledger)
    }

    #[tokio::test]
    async fn tracing_service_atomically_publishes_one_exact_file() {
        // Arrange
        let directory = fresh_directory("service");
        let sink = Arc::new(FileTraceSink::create(&directory, sink_limits()).unwrap());
        let context_id = ObjectId::new(2);
        let profile = profile(context_id);
        let resources = resources(profile);
        let tracing =
            TracingService::new(TestClock(AtomicU64::new(10)), Arc::clone(&sink), 1).unwrap();
        tracing
            .register_context(context_id, profile.create(&resources).unwrap())
            .unwrap();
        Service::<Command<TraceStart>>::call(
            &tracing,
            command(1, context_id, TraceStartOptions::new(None)),
        )
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
        Service::<Command<TraceStartChunk>>::call(
            &tracing,
            command(2, context_id, TraceStartChunkOptions::new(None, None)),
        )
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
        tracing
            .record_api_call_if_recording(
                context_id,
                TraceEntryKind::ApiCallBefore,
                CallId::new(42),
                "navigation.goto",
            )
            .unwrap();

        // Act
        let summary = Service::<Command<TraceStopChunk>>::call(
            &tracing,
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
        Service::<Command<TraceStop>>::call(&tracing, command(4, context_id, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();

        // Assert
        assert_eq!(summary.entry_count(), 1);
        assert_eq!(summary.payload_bytes(), 28);
        let path = directory.join("context-2-trace-1.pwtrace");
        let bytes = fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 88);
        assert_eq!(&bytes[..8], PWRTRACE_MAGIC);
        assert_eq!(u16::from_be_bytes(bytes[8..10].try_into().unwrap()), 1);
        assert_eq!(u64::from_be_bytes(bytes[10..18].try_into().unwrap()), 2);
        assert_eq!(u64::from_be_bytes(bytes[18..26].try_into().unwrap()), 1);
        assert_eq!(bytes[26], 0);
        assert_eq!(u32::from_be_bytes(bytes[27..31].try_into().unwrap()), 1);
        assert_eq!(u64::from_be_bytes(bytes[31..39].try_into().unwrap()), 28);
        assert_eq!(bytes[55], 1);
        assert_eq!(u32::from_be_bytes(bytes[56..60].try_into().unwrap()), 28);
        assert_eq!(sink.metrics().unwrap().committed_bytes, 88);
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
        drop(tracing);
        drop(sink);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn oversized_file_is_rejected_before_any_path_is_created() {
        // Arrange
        let directory = fresh_directory("oversized");
        let sink = FileTraceSink::create(&directory, FileTraceSinkLimits::new(1024, 1, 1, 60, 60))
            .unwrap();
        let (chunk, ledger) = chunk_with_payload(b"1234");

        // Act
        let error = sink
            .write(ObjectId::new(2), chunk)
            .expect_err("64 encoded bytes must exceed the 60-byte file limit");

        // Assert
        assert!(matches!(
            error,
            FileTraceSinkError::FileBytesExceeded {
                actual: 64,
                limit: 60
            }
        ));
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
        drop(sink);
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn second_chunk_is_rejected_at_the_independent_count_limit() {
        // Arrange
        let directory = fresh_directory("chunk-count");
        let sink =
            FileTraceSink::create(&directory, FileTraceSinkLimits::new(1024, 1, 1, 128, 256))
                .unwrap();
        let (first, first_ledger) = chunk_with_payload(b"one");
        let (second, second_ledger) = chunk_with_payload(b"two");
        sink.write(ObjectId::new(2), first).unwrap();

        // Act
        let error = sink
            .write(ObjectId::new(2), second)
            .expect_err("a second chunk must fail at a one-chunk limit");

        // Assert
        assert!(matches!(
            error,
            FileTraceSinkError::ChunkCapacityExceeded {
                context_id,
                limit: 1
            } if context_id == ObjectId::new(2)
        ));
        assert_eq!(sink.metrics().unwrap().committed_chunks, 1);
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        assert_eq!(
            first_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
        assert_eq!(
            second_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
        drop(sink);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn second_context_is_rejected_at_the_independent_context_limit() {
        // Arrange
        let directory = fresh_directory("context-count");
        let sink = FileTraceSink::create(&directory, sink_limits()).unwrap();
        let (first, first_ledger) = chunk_with_payload(b"one");
        let (second, second_ledger) = chunk_with_payload(b"two");
        sink.write(ObjectId::new(2), first).unwrap();

        // Act
        let error = sink
            .write(ObjectId::new(3), second)
            .expect_err("a second Context must fail at a one-Context limit");

        // Assert
        assert!(matches!(
            error,
            FileTraceSinkError::ContextCapacityExceeded { limit: 1 }
        ));
        assert_eq!(sink.metrics().unwrap().contexts, 1);
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        assert_eq!(
            first_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
        assert_eq!(
            second_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
        drop(sink);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn accumulated_context_bytes_are_checked_before_the_second_file() {
        // Arrange
        let directory = fresh_directory("context-bytes");
        let sink =
            FileTraceSink::create(&directory, FileTraceSinkLimits::new(1024, 1, 2, 100, 100))
                .unwrap();
        let (first, first_ledger) = chunk_with_payload(b"one");
        let (second, second_ledger) = chunk_with_payload(b"two");
        sink.write(ObjectId::new(2), first).unwrap();

        // Act
        let error = sink
            .write(ObjectId::new(2), second)
            .expect_err("two 63-byte files must exceed a 100-byte Context limit");

        // Assert
        assert!(matches!(
            error,
            FileTraceSinkError::ContextBytesExceeded {
                context_id,
                actual: 126,
                limit: 100
            } if context_id == ObjectId::new(2)
        ));
        assert_eq!(sink.metrics().unwrap().committed_bytes, 63);
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        assert_eq!(
            first_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
        assert_eq!(
            second_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
        drop(sink);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn sink_refuses_to_take_over_an_existing_directory() {
        // Arrange
        let directory = fresh_directory("existing");
        fs::create_dir(&directory).unwrap();

        // Act
        let error = match FileTraceSink::create(&directory, sink_limits()) {
            Ok(_) => panic!("existing artifact ownership must remain a caller decision"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(
            error,
            FileTraceSinkError::CreateDirectory { source, .. }
                if source.kind() == io::ErrorKind::AlreadyExists
        ));
        fs::remove_dir(directory).unwrap();
    }
}
