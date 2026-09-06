//! Optional bounded, task-free pull sink for complete Pwright trace chunks.
//!
//! The sink owns chunks only while they are queued. [`PullTraceSink::pop`]
//! transfers one complete chunk to the caller, which can then stream, archive,
//! inspect, or discard it without this crate selecting a runtime or transport.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::{Mutex, MutexGuard};

use pwright_core::ObjectId;
use pwright_trace_model::TraceChunk;
use pwright_tracing_service::TraceSink;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PullTraceSinkLimits {
    pub max_pending_chunks: usize,
    pub max_pending_chunks_per_context: usize,
    pub max_chunk_bytes: usize,
    pub max_pending_bytes: usize,
}

impl PullTraceSinkLimits {
    pub const fn new(
        max_pending_chunks: usize,
        max_pending_chunks_per_context: usize,
        max_chunk_bytes: usize,
        max_pending_bytes: usize,
    ) -> Self {
        Self {
            max_pending_chunks,
            max_pending_chunks_per_context,
            max_chunk_bytes,
            max_pending_bytes,
        }
    }

    fn validate(self) -> Result<(), PullTraceSinkError> {
        if self.max_pending_chunks == 0 {
            return Err(PullTraceSinkError::ZeroLimit("pending chunks"));
        }
        if self.max_pending_chunks_per_context == 0 {
            return Err(PullTraceSinkError::ZeroLimit("pending chunks per Context"));
        }
        if self.max_chunk_bytes == 0 {
            return Err(PullTraceSinkError::ZeroLimit("chunk bytes"));
        }
        if self.max_pending_bytes == 0 {
            return Err(PullTraceSinkError::ZeroLimit("pending bytes"));
        }
        if self.max_pending_chunks_per_context > self.max_pending_chunks {
            return Err(PullTraceSinkError::ContextLimitExceedsGlobalLimit {
                context: self.max_pending_chunks_per_context,
                global: self.max_pending_chunks,
            });
        }
        if self.max_chunk_bytes > self.max_pending_bytes {
            return Err(PullTraceSinkError::ChunkLimitExceedsGlobalLimit {
                chunk: self.max_chunk_bytes,
                global: self.max_pending_bytes,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PullTraceSinkMetrics {
    pub pending_chunks: usize,
    pub pending_bytes: usize,
    pub peak_pending_chunks: usize,
    pub peak_pending_bytes: usize,
    pub accepted_chunks: u64,
    pub rejected_chunks: u64,
}

#[derive(Debug)]
pub struct PulledTraceChunk {
    context_id: ObjectId,
    retained_bytes: usize,
    chunk: TraceChunk,
}

impl PulledTraceChunk {
    pub const fn context_id(&self) -> ObjectId {
        self.context_id
    }

    pub const fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }

    pub const fn chunk(&self) -> &TraceChunk {
        &self.chunk
    }

    pub fn into_chunk(self) -> TraceChunk {
        self.chunk
    }
}

struct PullTraceSinkState {
    pending: VecDeque<PulledTraceChunk>,
    pending_bytes: usize,
    peak_pending_chunks: usize,
    peak_pending_bytes: usize,
    accepted_chunks: u64,
    rejected_chunks: u64,
}

pub struct PullTraceSink {
    limits: PullTraceSinkLimits,
    state: Mutex<PullTraceSinkState>,
}

impl PullTraceSink {
    pub fn new(limits: PullTraceSinkLimits) -> Result<Self, PullTraceSinkError> {
        limits.validate()?;
        let mut pending = VecDeque::new();
        pending
            .try_reserve_exact(limits.max_pending_chunks)
            .map_err(|_| PullTraceSinkError::QueueAllocationFailed {
                requested: limits.max_pending_chunks,
            })?;
        Ok(Self {
            limits,
            state: Mutex::new(PullTraceSinkState {
                pending,
                pending_bytes: 0,
                peak_pending_chunks: 0,
                peak_pending_bytes: 0,
                accepted_chunks: 0,
                rejected_chunks: 0,
            }),
        })
    }

    pub const fn limits(&self) -> PullTraceSinkLimits {
        self.limits
    }

    /// Transfers the oldest complete chunk to the caller.
    ///
    /// Queue admission is released immediately, but the chunk's `TraceBytes`
    /// leases and recorder detachment gate remain held until the caller drops
    /// the returned value.
    pub fn pop(&self) -> Result<Option<PulledTraceChunk>, PullTraceSinkError> {
        let mut state = self.lock_state()?;
        let Some(item) = state.pending.pop_front() else {
            return Ok(None);
        };
        state.pending_bytes = state
            .pending_bytes
            .checked_sub(item.retained_bytes)
            .ok_or(PullTraceSinkError::CounterOverflow)?;
        Ok(Some(item))
    }

    pub fn metrics(&self) -> Result<PullTraceSinkMetrics, PullTraceSinkError> {
        let state = self.lock_state()?;
        Ok(PullTraceSinkMetrics {
            pending_chunks: state.pending.len(),
            pending_bytes: state.pending_bytes,
            peak_pending_chunks: state.peak_pending_chunks,
            peak_pending_bytes: state.peak_pending_bytes,
            accepted_chunks: state.accepted_chunks,
            rejected_chunks: state.rejected_chunks,
        })
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, PullTraceSinkState>, PullTraceSinkError> {
        self.state
            .lock()
            .map_err(|_| PullTraceSinkError::StatePoisoned)
    }

    fn reject(
        state: &mut PullTraceSinkState,
        error: PullTraceSinkError,
    ) -> Result<(), PullTraceSinkError> {
        state.rejected_chunks = state
            .rejected_chunks
            .checked_add(1)
            .ok_or(PullTraceSinkError::CounterOverflow)?;
        Err(error)
    }

    fn discard_context(&self, context_id: ObjectId) -> Result<(), PullTraceSinkError> {
        let mut state = self.lock_state()?;
        let retained_bytes = state
            .pending
            .iter()
            .filter(|item| item.context_id == context_id)
            .try_fold(0_usize, |total, item| {
                total.checked_add(item.retained_bytes)
            })
            .ok_or(PullTraceSinkError::CounterOverflow)?;
        state.pending.retain(|item| item.context_id != context_id);
        state.pending_bytes = state
            .pending_bytes
            .checked_sub(retained_bytes)
            .ok_or(PullTraceSinkError::CounterOverflow)?;
        Ok(())
    }

    fn discard_all(&self) -> Result<(), PullTraceSinkError> {
        let mut state = self.lock_state()?;
        state.pending.clear();
        state.pending_bytes = 0;
        Ok(())
    }
}

impl TraceSink for PullTraceSink {
    type Error = PullTraceSinkError;

    fn write(&self, context_id: ObjectId, chunk: TraceChunk) -> Result<(), Self::Error> {
        let retained_bytes = retained_chunk_bytes(&chunk)?;
        let mut state = self.lock_state()?;
        if retained_bytes > self.limits.max_chunk_bytes {
            return Self::reject(
                &mut state,
                PullTraceSinkError::ChunkBytesExceeded {
                    actual: retained_bytes,
                    limit: self.limits.max_chunk_bytes,
                },
            );
        }
        if state.pending.len() == self.limits.max_pending_chunks {
            return Self::reject(
                &mut state,
                PullTraceSinkError::ChunkCapacityExceeded {
                    limit: self.limits.max_pending_chunks,
                },
            );
        }
        let context_chunks = state
            .pending
            .iter()
            .filter(|item| item.context_id == context_id)
            .count();
        if context_chunks == self.limits.max_pending_chunks_per_context {
            return Self::reject(
                &mut state,
                PullTraceSinkError::ContextChunkCapacityExceeded {
                    context_id,
                    limit: self.limits.max_pending_chunks_per_context,
                },
            );
        }
        let following_bytes = state
            .pending_bytes
            .checked_add(retained_bytes)
            .ok_or(PullTraceSinkError::CounterOverflow)?;
        if following_bytes > self.limits.max_pending_bytes {
            return Self::reject(
                &mut state,
                PullTraceSinkError::PendingBytesExceeded {
                    actual: following_bytes,
                    limit: self.limits.max_pending_bytes,
                },
            );
        }
        let following_accepted = state
            .accepted_chunks
            .checked_add(1)
            .ok_or(PullTraceSinkError::CounterOverflow)?;
        state.pending.push_back(PulledTraceChunk {
            context_id,
            retained_bytes,
            chunk,
        });
        state.pending_bytes = following_bytes;
        state.peak_pending_chunks = state.peak_pending_chunks.max(state.pending.len());
        state.peak_pending_bytes = state.peak_pending_bytes.max(following_bytes);
        state.accepted_chunks = following_accepted;
        Ok(())
    }

    fn abort_context(&self, context_id: ObjectId) -> Result<(), Self::Error> {
        self.discard_context(context_id)
    }

    fn abort_all(&self) -> Result<(), Self::Error> {
        self.discard_all()
    }
}

fn retained_chunk_bytes(chunk: &TraceChunk) -> Result<usize, PullTraceSinkError> {
    chunk
        .name()
        .map_or(0, str::len)
        .checked_add(chunk.title().map_or(0, str::len))
        .and_then(|bytes| bytes.checked_add(chunk.payload_bytes()))
        .ok_or(PullTraceSinkError::CounterOverflow)
}

#[derive(Debug, Eq, PartialEq)]
pub enum PullTraceSinkError {
    ZeroLimit(&'static str),
    ContextLimitExceedsGlobalLimit { context: usize, global: usize },
    ChunkLimitExceedsGlobalLimit { chunk: usize, global: usize },
    QueueAllocationFailed { requested: usize },
    ChunkCapacityExceeded { limit: usize },
    ContextChunkCapacityExceeded { context_id: ObjectId, limit: usize },
    ChunkBytesExceeded { actual: usize, limit: usize },
    PendingBytesExceeded { actual: usize, limit: usize },
    CounterOverflow,
    StatePoisoned,
}

impl Display for PullTraceSinkError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit(name) => {
                write!(formatter, "pull trace sink {name} limit must be non-zero")
            }
            Self::ContextLimitExceedsGlobalLimit { context, global } => write!(
                formatter,
                "per-Context chunk limit {context} exceeds global chunk limit {global}"
            ),
            Self::ChunkLimitExceedsGlobalLimit { chunk, global } => write!(
                formatter,
                "per-chunk byte limit {chunk} exceeds global pending-byte limit {global}"
            ),
            Self::QueueAllocationFailed { requested } => {
                write!(formatter, "could not reserve {requested} trace queue slots")
            }
            Self::ChunkCapacityExceeded { limit } => {
                write!(formatter, "pull trace sink reached its {limit}-chunk limit")
            }
            Self::ContextChunkCapacityExceeded { context_id, limit } => write!(
                formatter,
                "pull trace sink Context {context_id} reached its {limit}-chunk limit"
            ),
            Self::ChunkBytesExceeded { actual, limit } => write!(
                formatter,
                "trace chunk retains {actual} bytes, exceeding limit {limit}"
            ),
            Self::PendingBytesExceeded { actual, limit } => write!(
                formatter,
                "pull trace sink would retain {actual} bytes, exceeding limit {limit}"
            ),
            Self::CounterOverflow => formatter.write_str("pull trace sink counter overflowed"),
            Self::StatePoisoned => formatter.write_str("pull trace sink state is poisoned"),
        }
    }
}

impl Error for PullTraceSinkError {}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cap_tracing::{
        TraceStart, TraceStartChunk, TraceStartChunkOptions, TraceStartOptions, TraceStop,
        TraceStopChunk, TraceStopChunkMode, TraceStopChunkOptions,
    };
    use pwright_core::{
        CallId, CallMetadata, Command, ProductionResourceLedger, ResourceClass, ResourceLedger,
        ResourceLimits, ResourceScope, Service,
    };
    use pwright_trace_model::{
        TraceEntryKind, TraceLimits, TraceRecorder, TraceRecorderProductionProfile,
    };
    use pwright_tracing_service::{TraceClock, TracingService};

    use super::*;

    #[derive(Clone, Copy)]
    struct FixedClock;

    impl TraceClock for FixedClock {
        fn monotonic_micros(&self) -> u64 {
            17
        }
    }

    fn limits() -> PullTraceSinkLimits {
        PullTraceSinkLimits::new(2, 1, 64, 128)
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

    fn profile(context_id: ObjectId) -> TraceRecorderProductionProfile {
        TraceRecorderProductionProfile::new(
            TraceLimits::new(32, 2, 64, 128, 1),
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

    fn chunk(
        name: Option<&str>,
        title: Option<&str>,
        payload: &[u8],
    ) -> (TraceChunk, ResourceLedger) {
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 8).with_class(
            ResourceClass::TraceBytes,
            256,
            256,
        ));
        let mut recorder = TraceRecorder::new(
            TraceLimits::new(32, 2, 64, 128, 1),
            ledger.clone(),
            ResourceScope::Connection,
        )
        .unwrap();
        recorder.start(None).unwrap();
        recorder.start_chunk(name, title).unwrap();
        recorder.append(TraceEntryKind::Log, 1, payload).unwrap();
        (recorder.stop_chunk().unwrap(), ledger)
    }

    #[tokio::test]
    async fn service_handoff_stays_budgeted_until_the_pulled_chunk_is_dropped() {
        // Arrange
        let context_id = ObjectId::new(7);
        let profile = profile(context_id);
        let resources = resources(profile);
        let sink = Arc::new(PullTraceSink::new(limits()).unwrap());
        let tracing = TracingService::new(FixedClock, Arc::clone(&sink), 1).unwrap();
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
        Service::<Command<TraceStopChunk>>::call(
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
        let queued_metrics = sink.metrics().unwrap();
        let pulled = sink.pop().unwrap().unwrap();
        let popped_metrics = sink.metrics().unwrap();

        // Assert
        assert_eq!(queued_metrics.pending_chunks, 1);
        assert_eq!(queued_metrics.pending_bytes, 28);
        assert_eq!(pulled.context_id(), context_id);
        assert_eq!(pulled.retained_bytes(), 28);
        assert_eq!(pulled.chunk().entries().len(), 1);
        assert_eq!(popped_metrics.pending_chunks, 0);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            28
        );
        drop(pulled);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
        Service::<Command<TraceStop>>::call(&tracing, command(4, context_id, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
    }

    #[test]
    fn global_chunk_backpressure_rejects_without_retaining_the_new_chunk() {
        // Arrange
        let sink = PullTraceSink::new(PullTraceSinkLimits::new(1, 1, 64, 64)).unwrap();
        let (first, first_ledger) = chunk(None, None, b"first");
        let (second, second_ledger) = chunk(None, None, b"second");
        TraceSink::write(&sink, ObjectId::new(1), first).unwrap();

        // Act
        let error = TraceSink::write(&sink, ObjectId::new(2), second).unwrap_err();

        // Assert
        assert_eq!(
            error,
            PullTraceSinkError::ChunkCapacityExceeded { limit: 1 }
        );
        assert_eq!(sink.metrics().unwrap().accepted_chunks, 1);
        assert_eq!(sink.metrics().unwrap().rejected_chunks, 1);
        assert_eq!(
            second_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
        TraceSink::abort_all(&sink).unwrap();
        assert_eq!(
            first_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[test]
    fn aggregate_byte_backpressure_is_independent_from_the_chunk_limit() {
        // Arrange
        let sink = PullTraceSink::new(PullTraceSinkLimits::new(2, 1, 8, 10)).unwrap();
        let (first, first_ledger) = chunk(None, None, b"123456");
        let (second, second_ledger) = chunk(None, None, b"abcdef");
        TraceSink::write(&sink, ObjectId::new(1), first).unwrap();

        // Act
        let error = TraceSink::write(&sink, ObjectId::new(2), second).unwrap_err();

        // Assert
        assert_eq!(
            error,
            PullTraceSinkError::PendingBytesExceeded {
                actual: 12,
                limit: 10,
            }
        );
        assert_eq!(sink.metrics().unwrap().pending_bytes, 6);
        assert_eq!(
            second_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
        TraceSink::abort_all(&sink).unwrap();
        assert_eq!(
            first_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[test]
    fn labels_are_included_in_the_independent_per_chunk_byte_limit() {
        // Arrange
        let sink = PullTraceSink::new(PullTraceSinkLimits::new(1, 1, 7, 7)).unwrap();
        let (chunk, ledger) = chunk(Some("name"), Some("ti"), b"xx");

        // Act
        let error = TraceSink::write(&sink, ObjectId::new(1), chunk).unwrap_err();

        // Assert
        assert_eq!(
            error,
            PullTraceSinkError::ChunkBytesExceeded {
                actual: 8,
                limit: 7,
            }
        );
        assert_eq!(sink.metrics().unwrap().pending_chunks, 0);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[test]
    fn context_abort_discards_only_that_contexts_queued_chunk() {
        // Arrange
        let sink = PullTraceSink::new(limits()).unwrap();
        let (first, first_ledger) = chunk(None, None, b"first");
        let (second, second_ledger) = chunk(None, None, b"second");
        TraceSink::write(&sink, ObjectId::new(1), first).unwrap();
        TraceSink::write(&sink, ObjectId::new(2), second).unwrap();

        // Act
        TraceSink::abort_context(&sink, ObjectId::new(1)).unwrap();

        // Assert
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
            6
        );
        let remaining = sink.pop().unwrap().unwrap();
        assert_eq!(remaining.context_id(), ObjectId::new(2));
        drop(remaining);
        assert_eq!(
            second_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[test]
    fn per_context_limit_rejects_a_second_independent_recorder_chunk() {
        // Arrange
        let sink = PullTraceSink::new(limits()).unwrap();
        let context_id = ObjectId::new(3);
        let (first, first_ledger) = chunk(None, None, b"first");
        let (second, second_ledger) = chunk(None, None, b"second");
        TraceSink::write(&sink, context_id, first).unwrap();

        // Act
        let error = TraceSink::write(&sink, context_id, second).unwrap_err();

        // Assert
        assert_eq!(
            error,
            PullTraceSinkError::ContextChunkCapacityExceeded {
                context_id,
                limit: 1,
            }
        );
        assert_eq!(
            second_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
        TraceSink::abort_all(&sink).unwrap();
        assert_eq!(
            first_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }
}
