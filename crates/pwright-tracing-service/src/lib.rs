//! Task-free Context-scoped tracing recorder and injected sink composition.
//!
//! The caller registers one bounded recorder per live BrowserContext and
//! supplies both a monotonic clock and a sink. No runtime, task, filesystem,
//! archive writer, or browser backend is selected here.

use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::{Ready, ready};
use std::sync::{Arc, Mutex, MutexGuard};

use pwright_cap_tracing::{
    TraceGroup, TraceGroupEnd, TraceStart, TraceStartChunk, TraceStartChunkResult, TraceStop,
    TraceStopChunk, TraceStopChunkMode, TraceStopChunkResult, TracingErrorDetails,
};
use pwright_core::{
    CallId, Command, CommandResult, FailureKind, ObjectId, OperationError, Service,
};
use pwright_server_tracing::TracingServerServices;
use pwright_trace_model::{
    TraceChunk, TraceEntryKind, TraceError, TraceRecorder, TraceSourceLocation,
};

pub trait TraceClock: Send + Sync {
    fn monotonic_micros(&self) -> u64;
}

pub trait TraceSink: Send + Sync {
    type Error: Error + Send + Sync + 'static;

    fn write(&self, context_id: ObjectId, chunk: TraceChunk) -> Result<(), Self::Error>;

    fn abort_context(&self, context_id: ObjectId) -> Result<(), Self::Error>;

    fn abort_all(&self) -> Result<(), Self::Error>;
}

impl<Sink> TraceSink for Arc<Sink>
where
    Sink: TraceSink,
{
    type Error = Sink::Error;

    fn write(&self, context_id: ObjectId, chunk: TraceChunk) -> Result<(), Self::Error> {
        self.as_ref().write(context_id, chunk)
    }

    fn abort_context(&self, context_id: ObjectId) -> Result<(), Self::Error> {
        self.as_ref().abort_context(context_id)
    }

    fn abort_all(&self) -> Result<(), Self::Error> {
        self.as_ref().abort_all()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoTraceSink;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoTraceSinkError;

impl Display for NoTraceSinkError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("no trace sink is installed")
    }
}

impl Error for NoTraceSinkError {}

impl TraceSink for NoTraceSink {
    type Error = NoTraceSinkError;

    fn write(&self, _: ObjectId, _: TraceChunk) -> Result<(), Self::Error> {
        Err(NoTraceSinkError)
    }

    fn abort_context(&self, _: ObjectId) -> Result<(), Self::Error> {
        Ok(())
    }

    fn abort_all(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TracingServiceMetrics {
    pub registered_contexts: usize,
    pub max_contexts: usize,
}

#[derive(Debug)]
pub enum TracingServiceError {
    ZeroContextLimit,
    ContextCapacityExceeded {
        limit: usize,
    },
    ContextAlreadyRegistered(ObjectId),
    SinkCleanup {
        context_id: Option<ObjectId>,
        message: String,
    },
    StatePoisoned,
}

impl Display for TracingServiceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroContextLimit => formatter.write_str("tracing context limit must be non-zero"),
            Self::ContextCapacityExceeded { limit } => {
                write!(formatter, "tracing reached its {limit}-Context limit")
            }
            Self::ContextAlreadyRegistered(id) => {
                write!(formatter, "tracing Context {id} is already registered")
            }
            Self::SinkCleanup {
                context_id,
                message,
            } => match context_id {
                Some(id) => write!(
                    formatter,
                    "trace sink cleanup failed for Context {id}: {message}"
                ),
                None => write!(formatter, "trace sink cleanup failed: {message}"),
            },
            Self::StatePoisoned => formatter.write_str("tracing service state is poisoned"),
        }
    }
}

impl Error for TracingServiceError {}

struct TracingServiceState {
    recorders: Vec<(ObjectId, TraceRecorder)>,
}

pub struct TracingService<Clock, Sink> {
    clock: Clock,
    sink: Sink,
    max_contexts: usize,
    state: Mutex<TracingServiceState>,
}

impl<Clock, Sink> TracingService<Clock, Sink>
where
    Clock: TraceClock,
    Sink: TraceSink,
{
    pub fn new(clock: Clock, sink: Sink, max_contexts: usize) -> Result<Self, TracingServiceError> {
        if max_contexts == 0 {
            return Err(TracingServiceError::ZeroContextLimit);
        }
        let mut recorders = Vec::new();
        recorders.try_reserve_exact(max_contexts).map_err(|_| {
            TracingServiceError::ContextCapacityExceeded {
                limit: max_contexts,
            }
        })?;
        Ok(Self {
            clock,
            sink,
            max_contexts,
            state: Mutex::new(TracingServiceState { recorders }),
        })
    }

    pub fn register_context(
        &self,
        context_id: ObjectId,
        recorder: TraceRecorder,
    ) -> Result<(), TracingServiceError> {
        let mut state = self.lock_state()?;
        if state.recorders.iter().any(|(id, _)| *id == context_id) {
            return Err(TracingServiceError::ContextAlreadyRegistered(context_id));
        }
        if state.recorders.len() == self.max_contexts {
            return Err(TracingServiceError::ContextCapacityExceeded {
                limit: self.max_contexts,
            });
        }
        state.recorders.push((context_id, recorder));
        Ok(())
    }

    pub fn remove_context(&self, context_id: ObjectId) -> Result<bool, TracingServiceError> {
        let mut state = self.lock_state()?;
        let Some(index) = state.recorders.iter().position(|(id, _)| *id == context_id) else {
            return Ok(false);
        };
        let (_, mut recorder) = state.recorders.swap_remove(index);
        recorder.abort();
        drop(recorder);
        drop(state);
        self.sink
            .abort_context(context_id)
            .map_err(|error| TracingServiceError::SinkCleanup {
                context_id: Some(context_id),
                message: error.to_string(),
            })?;
        Ok(true)
    }

    pub fn abort_all(&self) -> Result<usize, TracingServiceError> {
        let mut state = self.lock_state()?;
        let count = state.recorders.len();
        for (_, recorder) in &mut state.recorders {
            recorder.abort();
        }
        state.recorders.clear();
        drop(state);
        self.sink
            .abort_all()
            .map_err(|error| TracingServiceError::SinkCleanup {
                context_id: None,
                message: error.to_string(),
            })?;
        Ok(count)
    }

    pub fn metrics(&self) -> Result<TracingServiceMetrics, TracingServiceError> {
        Ok(TracingServiceMetrics {
            registered_contexts: self.lock_state()?.recorders.len(),
            max_contexts: self.max_contexts,
        })
    }

    pub fn record(
        &self,
        context_id: ObjectId,
        kind: TraceEntryKind,
        payload: &[u8],
    ) -> Result<u64, OperationError<TracingErrorDetails>> {
        let timestamp = self.clock.monotonic_micros();
        self.with_recorder(context_id, |recorder| {
            recorder.append(kind, timestamp, payload)
        })
    }

    /// Detaches the active chunk without choosing discard, sink, Artifact, or
    /// transport policy. Exactly one caller owns the returned non-cloneable
    /// chunk and its `TraceBytes` leases until it hands off or drops it.
    pub fn detach_chunk(
        &self,
        context_id: ObjectId,
    ) -> Result<TraceChunk, OperationError<TracingErrorDetails>> {
        self.with_recorder(context_id, TraceRecorder::stop_chunk)
    }

    /// Records a correlated API-call boundary only while a chunk is active.
    /// An idle or started Context returns `Ok(None)`, allowing an installed
    /// contributor to remain inert until the user explicitly starts a chunk.
    pub fn record_api_call_if_recording(
        &self,
        context_id: ObjectId,
        kind: TraceEntryKind,
        call_id: CallId,
        method: &str,
    ) -> Result<Option<u64>, OperationError<TracingErrorDetails>> {
        let timestamp = self.clock.monotonic_micros();
        let mut state = self.state.lock().map_err(|_| poisoned_operation())?;
        let recorder = state
            .recorders
            .iter_mut()
            .find_map(|(id, recorder)| (*id == context_id).then_some(recorder))
            .ok_or_else(|| missing_context(context_id))?;
        if !recorder.is_recording() {
            return Ok(None);
        }
        recorder
            .append_api_call(kind, timestamp, call_id, method)
            .map(Some)
            .map_err(trace_operation_error)
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, TracingServiceState>, TracingServiceError> {
        self.state
            .lock()
            .map_err(|_| TracingServiceError::StatePoisoned)
    }

    fn with_recorder<T>(
        &self,
        context_id: ObjectId,
        operation: impl FnOnce(&mut TraceRecorder) -> Result<T, TraceError>,
    ) -> Result<T, OperationError<TracingErrorDetails>> {
        let mut state = self.state.lock().map_err(|_| poisoned_operation())?;
        let recorder = state
            .recorders
            .iter_mut()
            .find_map(|(id, recorder)| (*id == context_id).then_some(recorder))
            .ok_or_else(|| missing_context(context_id))?;
        operation(recorder).map_err(trace_operation_error)
    }
}

fn missing_context(context_id: ObjectId) -> OperationError<TracingErrorDetails> {
    OperationError::new(
        FailureKind::TargetClosed,
        format!("tracing Context {context_id} is not registered"),
        Some(TracingErrorDetails::InvalidState(
            "BrowserContext is closed or tracing is not installed".to_owned(),
        )),
    )
}

pub type SharedTracingServerServices<Clock, Sink> = TracingServerServices<
    TracingService<Clock, Sink>,
    TracingService<Clock, Sink>,
    TracingService<Clock, Sink>,
    TracingService<Clock, Sink>,
    TracingService<Clock, Sink>,
    TracingService<Clock, Sink>,
>;

pub fn tracing_server_services<Clock, Sink>(
    service: Arc<TracingService<Clock, Sink>>,
) -> SharedTracingServerServices<Clock, Sink>
where
    Clock: TraceClock,
    Sink: TraceSink,
{
    TracingServerServices {
        trace_group: Arc::clone(&service),
        trace_group_end: Arc::clone(&service),
        trace_start: Arc::clone(&service),
        trace_start_chunk: Arc::clone(&service),
        trace_stop: Arc::clone(&service),
        trace_stop_chunk: service,
    }
}

impl<Clock, Sink> Service<Command<TraceStart>> for TracingService<Clock, Sink>
where
    Clock: TraceClock + 'static,
    Sink: TraceSink + 'static,
{
    type Response = CommandResult<TraceStart>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<TraceStart>) -> Self::Future<'_> {
        let result = self.with_recorder(command.object_id(), |recorder| {
            recorder.start(command.params().name())
        });
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Clock, Sink> Service<Command<TraceStartChunk>> for TracingService<Clock, Sink>
where
    Clock: TraceClock + 'static,
    Sink: TraceSink + 'static,
{
    type Response = CommandResult<TraceStartChunk>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<TraceStartChunk>) -> Self::Future<'_> {
        let result = self
            .with_recorder(command.object_id(), |recorder| {
                recorder.start_chunk(command.params().name(), command.params().title())
            })
            .map(|id| TraceStartChunkResult::new(id.get()));
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Clock, Sink> Service<Command<TraceGroup>> for TracingService<Clock, Sink>
where
    Clock: TraceClock + 'static,
    Sink: TraceSink + 'static,
{
    type Response = CommandResult<TraceGroup>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<TraceGroup>) -> Self::Future<'_> {
        let timestamp = self.clock.monotonic_micros();
        let result = self
            .with_recorder(command.object_id(), |recorder| {
                let location = command.params().location().map(|location| {
                    TraceSourceLocation::new(location.file(), location.line(), location.column())
                });
                recorder.begin_group_at(timestamp, command.params().name(), location)
            })
            .map(|_| ());
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Clock, Sink> Service<Command<TraceGroupEnd>> for TracingService<Clock, Sink>
where
    Clock: TraceClock + 'static,
    Sink: TraceSink + 'static,
{
    type Response = CommandResult<TraceGroupEnd>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<TraceGroupEnd>) -> Self::Future<'_> {
        let timestamp = self.clock.monotonic_micros();
        let result = self
            .with_recorder(command.object_id(), |recorder| {
                recorder.end_group(timestamp)
            })
            .map(|_| ());
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Clock, Sink> Service<Command<TraceStopChunk>> for TracingService<Clock, Sink>
where
    Clock: TraceClock + 'static,
    Sink: TraceSink + 'static,
{
    type Response = CommandResult<TraceStopChunk>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<TraceStopChunk>) -> Self::Future<'_> {
        let context_id = command.object_id();
        let result = self.detach_chunk(context_id).and_then(|chunk| {
            let summary = TraceStopChunkResult::new(
                chunk.id().get(),
                u64::try_from(chunk.entries().len()).map_err(|_| trace_limit_overflow())?,
                u64::try_from(chunk.payload_bytes()).map_err(|_| trace_limit_overflow())?,
            );
            match command.params().mode() {
                TraceStopChunkMode::Discard => drop(chunk),
                TraceStopChunkMode::Sink => {
                    self.sink.write(context_id, chunk).map_err(|error| {
                        OperationError::new(
                            FailureKind::Backend,
                            "trace sink rejected the chunk",
                            Some(TracingErrorDetails::Sink(error.to_string())),
                        )
                    })?
                }
            }
            Ok(summary)
        });
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Clock, Sink> Service<Command<TraceStop>> for TracingService<Clock, Sink>
where
    Clock: TraceClock + 'static,
    Sink: TraceSink + 'static,
{
    type Response = CommandResult<TraceStop>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<TraceStop>) -> Self::Future<'_> {
        let result = self.with_recorder(command.object_id(), TraceRecorder::stop);
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

fn poisoned_operation() -> OperationError<TracingErrorDetails> {
    OperationError::new(
        FailureKind::Cleanup,
        "tracing service state is poisoned",
        Some(TracingErrorDetails::InvalidState(
            "shared recorder state is unavailable".to_owned(),
        )),
    )
}

fn trace_limit_overflow() -> OperationError<TracingErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        "trace summary exceeds the portable integer range",
        Some(TracingErrorDetails::Resource(
            "trace summary integer overflow".to_owned(),
        )),
    )
}

fn trace_operation_error(error: TraceError) -> OperationError<TracingErrorDetails> {
    let message = error.to_string();
    match error {
        TraceError::LabelTooLarge { .. }
        | TraceError::LabelEncodingOverflow { .. }
        | TraceError::InvalidApiCallKind(_) => OperationError::new(
            FailureKind::ProtocolViolation,
            "invalid tracing options",
            Some(TracingErrorDetails::InvalidOptions(message)),
        ),
        TraceError::LimitOverflow
        | TraceError::EntryTooLarge { .. }
        | TraceError::EntryCapacityExceeded { .. }
        | TraceError::ChunkBytesExceeded { .. }
        | TraceError::GroupDepthExceeded { .. }
        | TraceError::EntryAllocationFailed { .. }
        | TraceError::PayloadAllocationFailed { .. }
        | TraceError::Resources(_) => OperationError::new(
            FailureKind::ResourceExhausted,
            "tracing resource limit reached",
            Some(TracingErrorDetails::Resource(message)),
        ),
        _ => OperationError::new(
            FailureKind::ProtocolViolation,
            "invalid tracing lifecycle transition",
            Some(TracingErrorDetails::InvalidState(message)),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::atomic::{AtomicU64, Ordering};

    use pwright_cap_tracing::{
        TraceGroupOptions, TraceLocation, TraceStartChunkOptions, TraceStartOptions,
        TraceStopChunkOptions,
    };
    use pwright_client::{
        AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, RemoteInitializer, RemoteKind,
    };
    use pwright_client_tracing::BrowserContextTracingExt;
    use pwright_client_wire::{DriverEvent, EncodedClientChannel};
    use pwright_codec_tracing::{TracingCodec, TracingCodecLimits};
    use pwright_core::{
        CallId, CallMetadata, Command, ProductionResourceLedger, ResourceClass, ResourceLimits,
        ResourceScope,
    };
    use pwright_server_tracing::tracing_server_handlers;
    use pwright_server_wire::{ConcurrentServerFrameHandler, ServerFrameDisposition};
    use pwright_trace_model::{TraceLimits, TraceRecorderProductionProfile};
    use pwright_transport::{FrameLimits, FrameTransport, WireProtocol};
    use pwright_transport_memory::MemoryFrameTransport;

    use super::*;

    struct TestClock(AtomicU64);

    impl TraceClock for TestClock {
        fn monotonic_micros(&self) -> u64 {
            self.0.fetch_add(1, Ordering::Relaxed)
        }
    }

    #[derive(Default)]
    struct RecordingSink {
        chunks: Mutex<Vec<(ObjectId, u64, usize, usize)>>,
        aborted_contexts: Mutex<Vec<ObjectId>>,
        aborted_all: AtomicU64,
        fail: bool,
        cleanup_fail: bool,
    }

    impl TraceSink for RecordingSink {
        type Error = io::Error;

        fn write(&self, context_id: ObjectId, chunk: TraceChunk) -> Result<(), Self::Error> {
            self.chunks.lock().unwrap().push((
                context_id,
                chunk.id().get(),
                chunk.entries().len(),
                chunk.payload_bytes(),
            ));
            if self.fail {
                Err(io::Error::other("sink full"))
            } else {
                Ok(())
            }
        }

        fn abort_context(&self, context_id: ObjectId) -> Result<(), Self::Error> {
            self.aborted_contexts.lock().unwrap().push(context_id);
            if self.cleanup_fail {
                Err(io::Error::other("cleanup failed"))
            } else {
                Ok(())
            }
        }

        fn abort_all(&self) -> Result<(), Self::Error> {
            self.aborted_all.fetch_add(1, Ordering::Relaxed);
            if self.cleanup_fail {
                Err(io::Error::other("cleanup failed"))
            } else {
                Ok(())
            }
        }
    }

    fn service(
        sink: RecordingSink,
    ) -> (
        Arc<TracingService<TestClock, RecordingSink>>,
        ProductionResourceLedger,
        ObjectId,
    ) {
        let context_id = ObjectId::new(2);
        let limits = TraceLimits::new(32, 8, 64, 256, 4);
        let profile =
            TraceRecorderProductionProfile::new(limits, ResourceScope::Object(context_id)).unwrap();
        let requirements = profile.requirements();
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::TraceBytes,
                requirements.class(ResourceClass::TraceBytes).global,
                requirements.class(ResourceClass::TraceBytes).per_scope,
            ),
            requirements,
        )
        .unwrap();
        let recorder = profile.create(&resources).unwrap();
        let service =
            Arc::new(TracingService::new(TestClock(AtomicU64::new(10)), sink, 1).unwrap());
        service.register_context(context_id, recorder).unwrap();
        (service, resources, context_id)
    }

    fn command<Spec: pwright_core::CommandSpec>(
        call_id: u64,
        context_id: ObjectId,
        params: Spec::Params,
    ) -> Command<Spec> {
        Command::new(
            CallId::new(call_id),
            context_id,
            CallMetadata::default(),
            params,
        )
    }

    fn protocol() -> WireProtocol<TracingCodec> {
        let version = pwright_core::ProtocolVersion::new(1, 0);
        let limits = FrameLimits::new(4_096, 0, 0, 4_096);
        WireProtocol::new_exact_directional(
            TracingCodec::new(TracingCodecLimits::new(256)),
            version,
            limits,
            limits,
        )
    }

    #[tokio::test]
    async fn six_services_share_one_context_recorder_and_injected_sink() {
        // Arrange
        let (service, resources, context_id) = service(RecordingSink::default());

        // Act
        service
            .call(command::<TraceStart>(
                1,
                context_id,
                TraceStartOptions::new(Some("session".into())),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        let chunk_id = service
            .call(command::<TraceStartChunk>(
                2,
                context_id,
                TraceStartChunkOptions::new(Some("chunk".into()), Some("title".into())),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        service
            .call(command::<TraceGroup>(
                3,
                context_id,
                TraceGroupOptions::new(
                    "checkout",
                    Some(TraceLocation::new("flow.rs", Some(7), Some(3))),
                ),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        service
            .record(context_id, TraceEntryKind::ApiCallBefore, b"click")
            .unwrap();
        service
            .call(command::<TraceGroupEnd>(4, context_id, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        let summary = service
            .call(command::<TraceStopChunk>(
                5,
                context_id,
                TraceStopChunkOptions::new(TraceStopChunkMode::Sink),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        service
            .call(command::<TraceStop>(6, context_id, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();

        // Assert
        assert_eq!(chunk_id.chunk_id(), 1);
        assert_eq!(summary.entry_count(), 3);
        assert_eq!(
            service.sink.chunks.lock().unwrap().as_slice(),
            &[(context_id, 1, 3, 40)]
        );
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn sink_failure_drops_the_chunk_and_allows_a_retry_chunk() {
        // Arrange
        let sink = RecordingSink {
            fail: true,
            ..RecordingSink::default()
        };
        let (service, resources, context_id) = service(sink);
        service
            .call(command::<TraceStart>(
                1,
                context_id,
                TraceStartOptions::default(),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        service
            .call(command::<TraceStartChunk>(
                2,
                context_id,
                TraceStartChunkOptions::default(),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        service
            .record(context_id, TraceEntryKind::Log, b"failed")
            .unwrap();

        // Act
        let error = service
            .call(command::<TraceStopChunk>(
                3,
                context_id,
                TraceStopChunkOptions::new(TraceStopChunkMode::Sink),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap_err();
        let retry = service
            .call(command::<TraceStartChunk>(
                4,
                context_id,
                TraceStartChunkOptions::default(),
            ))
            .await
            .unwrap()
            .into_outcome();

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(retry.unwrap().chunk_id(), 2);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn context_removal_aborts_active_state_and_releases_all_bytes() {
        // Arrange
        let (service, resources, context_id) = service(RecordingSink::default());
        service
            .call(command::<TraceStart>(
                1,
                context_id,
                TraceStartOptions::new(Some("session".into())),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        service
            .call(command::<TraceStartChunk>(
                2,
                context_id,
                TraceStartChunkOptions::new(Some("chunk".into()), None),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        service
            .record(context_id, TraceEntryKind::Log, b"retained")
            .unwrap();

        // Act
        let removed = service.remove_context(context_id).unwrap();
        let stale = service
            .record(context_id, TraceEntryKind::Log, b"late")
            .unwrap_err();

        // Assert
        assert!(removed);
        assert_eq!(stale.kind(), FailureKind::TargetClosed);
        assert_eq!(service.metrics().unwrap().registered_contexts, 0);
        assert_eq!(
            service.sink.aborted_contexts.lock().unwrap().as_slice(),
            &[context_id]
        );
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn sink_cleanup_failure_cannot_retain_the_base_recorder() {
        // Arrange
        let sink = RecordingSink {
            cleanup_fail: true,
            ..RecordingSink::default()
        };
        let (service, resources, context_id) = service(sink);
        service
            .call(command::<TraceStart>(
                1,
                context_id,
                TraceStartOptions::new(Some("session".into())),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();

        // Act
        let error = service.remove_context(context_id).unwrap_err();

        // Assert
        assert!(matches!(
            error,
            TracingServiceError::SinkCleanup {
                context_id: Some(id),
                ..
            } if id == context_id
        ));
        assert_eq!(service.metrics().unwrap().registered_contexts, 0);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn typed_client_round_trips_all_six_commands_through_the_generated_server() {
        macro_rules! round_trip {
            ($call:expr, $handler:expr, $server:expr, $driver:expr) => {{
                let call = $call;
                let server_turn = async {
                    let frame = $server
                        .receive()
                        .await
                        .expect("server receive should succeed")
                        .expect("server should receive command");
                    $handler
                        .handle_concurrently(frame, Arc::clone(&$server))
                        .await
                        .expect("server should handle command")
                };
                let (result, handled, delivered) =
                    tokio::join!(call, server_turn, $driver.receive_once());
                assert!(matches!(handled, ServerFrameDisposition::Handled(_)));
                assert!(matches!(delivered, Ok(DriverEvent::Delivered(_))));
                result
            }};
        }

        // Arrange
        let (service, resources, context_id) = service(RecordingSink::default());
        let (client_transport, server_transport) = MemoryFrameTransport::pair(2);
        let server_transport = Arc::new(server_transport);
        let (channel, mut client_driver) =
            EncodedClientChannel::new(protocol(), client_transport, 1);
        let client = Client::new(channel, AtomicCallIdSource::new(20));
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BrowserKind::OBJECT_KIND,
            ))
            .unwrap();
        let context = browser
            .context(RemoteInitializer::new(
                context_id,
                Some(ObjectId::new(1)),
                BrowserContextKind::OBJECT_KIND,
            ))
            .unwrap();
        let tracing = context.tracing();
        let handler = tracing_server_handlers(
            Arc::new(protocol()),
            tracing_server_services(Arc::clone(&service)),
        );

        // Act
        round_trip!(
            tracing.start(TraceStartOptions::new(Some("remote".into()))),
            handler,
            server_transport,
            client_driver
        )
        .unwrap();
        let chunk = round_trip!(
            tracing.start_chunk(TraceStartChunkOptions::new(
                Some("chunk".into()),
                Some("Remote flow".into()),
            )),
            handler,
            server_transport,
            client_driver
        )
        .unwrap();
        round_trip!(
            tracing.group(TraceGroupOptions::new(
                "checkout",
                Some(TraceLocation::new("remote.rs", Some(4), Some(2))),
            )),
            handler,
            server_transport,
            client_driver
        )
        .unwrap();
        service
            .record(context_id, TraceEntryKind::ApiCallBefore, b"goto")
            .unwrap();
        round_trip!(
            tracing.group_end(),
            handler,
            server_transport,
            client_driver
        )
        .unwrap();
        let summary = round_trip!(
            tracing.stop_chunk(TraceStopChunkOptions::new(TraceStopChunkMode::Sink)),
            handler,
            server_transport,
            client_driver
        )
        .unwrap();
        round_trip!(tracing.stop(), handler, server_transport, client_driver).unwrap();

        // Assert
        assert_eq!(chunk.chunk_id(), 1);
        assert_eq!(summary.entry_count(), 3);
        let wire = client.channel().metrics().unwrap();
        assert_eq!(wire.awaiting_results(), 0);
        assert_eq!(wire.waiting_calls(), 0);
        assert_eq!(wire.abandoned_calls(), 0);
        assert_eq!(server_transport.metrics().unwrap().incoming_depth(), 0);
        assert_eq!(server_transport.metrics().unwrap().outgoing_depth(), 0);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[test]
    fn generated_server_services_share_the_exact_same_instance() {
        // Arrange
        let (service, _, _) = service(RecordingSink::default());

        // Act
        let services = tracing_server_services(Arc::clone(&service));

        // Assert
        assert!(Arc::ptr_eq(&services.trace_group, &service));
        assert!(Arc::ptr_eq(&services.trace_group_end, &service));
        assert!(Arc::ptr_eq(&services.trace_start, &service));
        assert!(Arc::ptr_eq(&services.trace_start_chunk, &service));
        assert!(Arc::ptr_eq(&services.trace_stop, &service));
        assert!(Arc::ptr_eq(&services.trace_stop_chunk, &service));
    }
}
