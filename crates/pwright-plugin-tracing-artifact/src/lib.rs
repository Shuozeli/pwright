//! Opt-in bounded tracing-to-Artifact producer composition.
//!
//! This leaf crate is the only place where base tracing, the PWRTRACE format,
//! Artifact storage, runtime lifecycle, and ordered control publication meet.
//! It starts no task and chooses no transport or executor.

use std::convert::Infallible;
use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::sync::Arc;

use pwright_artifact_runtime::{RuntimeArtifactError, RuntimeArtifactService};
use pwright_browser_model::ARTIFACT_OBJECT_KIND;
use pwright_cap_tracing_artifact::{
    TraceArtifactResult, TraceStopChunkArtifact, TracingArtifactErrorDetails,
};
use pwright_core::{
    Command, CommandResult, FailureKind, ObjectGraphError, ObjectIdSource, OperationError, Service,
};
use pwright_runtime_control::{ControlMessageSource, OrderedRuntimeControl};
use pwright_server_tracing_artifact::TracingArtifactServerServices;
use pwright_trace_format::{TraceArtifactEncodingLimits, TraceFormatError, encode_trace_artifact};
use pwright_tracing_service::{TraceClock, TraceSink, TracingService};
use pwright_transport::{FrameTransport, WireCodec};

pub struct TracingArtifactService<Clock, Sink, Ids> {
    tracing: Arc<TracingService<Clock, Sink>>,
    artifacts: Arc<RuntimeArtifactService<Ids>>,
    encoding_limits: TraceArtifactEncodingLimits,
}

impl<Clock, Sink, Ids> TracingArtifactService<Clock, Sink, Ids> {
    pub const fn new(
        tracing: Arc<TracingService<Clock, Sink>>,
        artifacts: Arc<RuntimeArtifactService<Ids>>,
        encoding_limits: TraceArtifactEncodingLimits,
    ) -> Self {
        Self {
            tracing,
            artifacts,
            encoding_limits,
        }
    }

    pub const fn artifact_service(&self) -> &Arc<RuntimeArtifactService<Ids>> {
        &self.artifacts
    }
}

impl<Clock, Sink, Ids> Service<Command<TraceStopChunkArtifact>>
    for TracingArtifactService<Clock, Sink, Ids>
where
    Clock: TraceClock + 'static,
    Sink: TraceSink + 'static,
    Ids: ObjectIdSource,
{
    type Response = CommandResult<TraceStopChunkArtifact>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<TraceStopChunkArtifact>) -> Self::Future<'_> {
        let result = self.produce(command.object_id());
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Clock, Sink, Ids> TracingArtifactService<Clock, Sink, Ids>
where
    Clock: TraceClock,
    Sink: TraceSink,
    Ids: ObjectIdSource,
{
    fn produce(
        &self,
        context_id: pwright_core::ObjectId,
    ) -> Result<TraceArtifactResult, OperationError<TracingArtifactErrorDetails>> {
        let artifact_id = self
            .artifacts
            .allocate_object_id()
            .map_err(runtime_operation_error)?;
        let chunk = self
            .tracing
            .detach_chunk(context_id)
            .map_err(tracing_operation_error)?;
        let chunk_id = chunk.id().get();
        let entry_count = u64::try_from(chunk.entries().len())
            .map_err(|_| resource_error("trace entry count cannot be represented on the wire"))?;
        let payload_bytes = u64::try_from(chunk.payload_bytes())
            .map_err(|_| resource_error("trace payload size cannot be represented on the wire"))?;
        let encoded = encode_trace_artifact(
            self.encoding_limits,
            self.artifacts.registry().ledger(),
            artifact_id,
            context_id,
            &chunk,
        )
        .map_err(trace_format_operation_error)?;
        let encoded_bytes = u64::try_from(encoded.len())
            .map_err(|_| resource_error("encoded trace size cannot be represented on the wire"))?;
        self.artifacts
            .register_budgeted_bytes(artifact_id, context_id, encoded)
            .map_err(runtime_operation_error)?;
        drop(chunk);
        Ok(TraceArtifactResult::new(
            artifact_id,
            context_id,
            ARTIFACT_OBJECT_KIND.tag,
            chunk_id,
            entry_count,
            payload_bytes,
            encoded_bytes,
        ))
    }
}

fn tracing_operation_error(
    error: OperationError<pwright_cap_tracing::TracingErrorDetails>,
) -> OperationError<TracingArtifactErrorDetails> {
    OperationError::new(
        error.kind(),
        "trace chunk could not be detached for Artifact production",
        Some(TracingArtifactErrorDetails::InvalidState(error.to_string())),
    )
}

fn trace_format_operation_error(
    error: TraceFormatError,
) -> OperationError<TracingArtifactErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        "trace chunk could not be encoded within Artifact limits",
        Some(TracingArtifactErrorDetails::Resource(error.to_string())),
    )
}

fn resource_error(message: &str) -> OperationError<TracingArtifactErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        "trace Artifact metadata exceeds protocol limits",
        Some(TracingArtifactErrorDetails::Resource(message.to_owned())),
    )
}

fn runtime_operation_error(
    error: RuntimeArtifactError,
) -> OperationError<TracingArtifactErrorDetails> {
    let message = error.to_string();
    let (kind, details) = match error {
        RuntimeArtifactError::Registry(_) | RuntimeArtifactError::ObjectId(_) => (
            FailureKind::ResourceExhausted,
            TracingArtifactErrorDetails::Resource(message),
        ),
        RuntimeArtifactError::Runtime(runtime) => {
            let kind = match runtime {
                pwright_runtime::RuntimeStateError::TransportClosed => FailureKind::TargetClosed,
                pwright_runtime::RuntimeStateError::ObjectGraph(
                    ObjectGraphError::CapacityExceeded { .. },
                )
                | pwright_runtime::RuntimeStateError::ControlMessageCapacityExceeded { .. }
                | pwright_runtime::RuntimeStateError::LifecycleTransactionInProgress
                | pwright_runtime::RuntimeStateError::LifecycleTransactionIdExhausted
                | pwright_runtime::RuntimeStateError::ProductionResource(_)
                | pwright_runtime::RuntimeStateError::Resource(_) => FailureKind::ResourceExhausted,
                _ => FailureKind::Cleanup,
            };
            (kind, TracingArtifactErrorDetails::Lifecycle(message))
        }
        RuntimeArtifactError::LifecycleDiverged { .. } => (
            FailureKind::Cleanup,
            TracingArtifactErrorDetails::Cleanup(message),
        ),
    };
    OperationError::new(kind, "trace Artifact lifecycle failed", Some(details))
}

pub struct OrderedTracingArtifactService<Clock, Sink, Ids, Source, Codec, Transport> {
    service: Arc<TracingArtifactService<Clock, Sink, Ids>>,
    control: OrderedRuntimeControl<Source, Codec, Transport>,
}

impl<Clock, Sink, Ids, Source, Codec, Transport>
    OrderedTracingArtifactService<Clock, Sink, Ids, Source, Codec, Transport>
{
    pub const fn new(
        service: Arc<TracingArtifactService<Clock, Sink, Ids>>,
        control: OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Self {
        Self { service, control }
    }
}

impl<Clock, Sink, Ids, Source, Codec, Transport> Service<Command<TraceStopChunkArtifact>>
    for OrderedTracingArtifactService<Clock, Sink, Ids, Source, Codec, Transport>
where
    Clock: TraceClock + 'static,
    Sink: TraceSink + 'static,
    Ids: ObjectIdSource,
    Source: ControlMessageSource + Send + Sync + 'static,
    Codec: WireCodec<pwright_core::ControlMessage> + 'static,
    Transport: FrameTransport + 'static,
{
    type Response = CommandResult<TraceStopChunkArtifact>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<TraceStopChunkArtifact>) -> Self::Future<'_> {
        let call_id = command.call_id();
        Box::pin(async move {
            let barrier = match self.control.begin_barrier().await {
                Ok(barrier) => barrier,
                Err(error) => return Ok(ordering_failure(call_id, error.to_string())),
            };
            let result = match self.service.call(command).await {
                Ok(result) => result,
                Err(never) => match never {},
            };
            let initializer = match result.outcome() {
                Ok(initializer) => *initializer,
                Err(_) => return Ok(result),
            };
            if barrier
                .send_through_creation(
                    initializer.artifact_id,
                    Some(initializer.parent_id),
                    initializer.object_kind,
                )
                .await
                .is_ok()
            {
                return Ok(result);
            }
            let retired = self
                .service
                .artifact_service()
                .retire_artifact(initializer.artifact_id)
                .is_ok();
            if retired {
                let _ = barrier.send_through_disposal(initializer.artifact_id).await;
            }
            Ok(ordering_failure(
                call_id,
                if retired {
                    "Artifact Create fence failed; the unpublished Artifact was retired".to_owned()
                } else {
                    "Artifact Create fence failed; the unpublished Artifact could not be retired"
                        .to_owned()
                },
            ))
        })
    }
}

fn ordering_failure(
    call_id: pwright_core::CallId,
    message: String,
) -> CommandResult<TraceStopChunkArtifact> {
    CommandResult::new(
        call_id,
        Err(OperationError::new(
            FailureKind::Cleanup,
            "trace Artifact lifecycle publication failed",
            Some(TracingArtifactErrorDetails::Cleanup(message)),
        )),
    )
}

pub type SharedTracingArtifactServerServices<Clock, Sink, Ids, Source, Codec, Transport> =
    TracingArtifactServerServices<
        OrderedTracingArtifactService<Clock, Sink, Ids, Source, Codec, Transport>,
    >;

pub fn tracing_artifact_server_services<Clock, Sink, Ids, Source, Codec, Transport>(
    service: Arc<OrderedTracingArtifactService<Clock, Sink, Ids, Source, Codec, Transport>>,
) -> SharedTracingArtifactServerServices<Clock, Sink, Ids, Source, Codec, Transport> {
    TracingArtifactServerServices {
        trace_stop_chunk_artifact: service,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use pwright_artifact_runtime::RuntimeArtifactService;
    use pwright_artifact_service::{
        ArtifactRegistry, ArtifactRegistryLimits, ArtifactRegistryProductionProfile,
    };
    use pwright_browser_model::{
        ARTIFACT_OBJECT_KIND, BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND,
        BROWSER_OBJECT_KIND_CATALOG,
    };
    use pwright_cap_artifact::{ArtifactOpenStream, StreamRead, StreamReadOptions};
    use pwright_cap_tracing::{TraceStartChunk, TraceStartChunkOptions};
    use pwright_cap_tracing_artifact::TraceStopChunkArtifact;
    use pwright_client::{
        AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, RemoteInitializer, RemoteKind,
    };
    use pwright_client_tracing_artifact::BrowserContextTracingArtifactExt;
    use pwright_client_wire::{DriverEvent, EncodedClientChannel};
    use pwright_codec_control::ControlCodec;
    use pwright_codec_tracing_artifact::{TracingArtifactCodec, TracingArtifactCodecLimits};
    use pwright_core::{
        AtomicObjectIdSource, CallId, CallMetadata, Command, ControlMessage, FailureKind, ObjectId,
        ProductionResourceLedger, ProtocolVersion, ResourceClass, ResourceLimits, ResourceScope,
        Service,
    };
    use pwright_runtime::{
        ControlledDispatcher, NoDeadlineScheduler, ObjectTopologyMode, RuntimeLimits,
    };
    use pwright_runtime_control::{OrderedRuntimeControl, RuntimeControlDriver};
    use pwright_server_tracing_artifact::{
        TracingArtifactServerServices, tracing_artifact_server_handlers,
    };
    use pwright_server_wire::{ConcurrentServerFrameHandler, ServerFrameDisposition};
    use pwright_trace_format::{PWRTRACE_MAGIC, TraceArtifactEncodingLimits};
    use pwright_trace_model::{TraceEntryKind, TraceLimits, TraceRecorderProductionProfile};
    use pwright_tracing_service::{NoTraceSink, TraceClock, TracingService};
    use pwright_transport::{FrameLimits, FrameTransport, WireProtocol};
    use pwright_transport_memory::MemoryFrameTransport;

    use super::{OrderedTracingArtifactService, TracingArtifactService};

    type TestRuntime = ControlledDispatcher<(), (), NoDeadlineScheduler>;

    struct TestClock(AtomicU64);

    impl TraceClock for TestClock {
        fn monotonic_micros(&self) -> u64 {
            self.0.fetch_add(1, Ordering::Relaxed)
        }
    }

    struct Fixture {
        tracing: Arc<TracingService<TestClock, NoTraceSink>>,
        artifacts: Arc<RuntimeArtifactService<AtomicObjectIdSource>>,
        registry: Arc<ArtifactRegistry>,
        trace_resources: ProductionResourceLedger,
        artifact_resources: ProductionResourceLedger,
        runtime: Arc<TestRuntime>,
    }

    fn fixture() -> Fixture {
        let context_id = ObjectId::new(2);
        let trace_profile = TraceRecorderProductionProfile::new(
            TraceLimits::new(32, 4, 64, 256, 2),
            ResourceScope::Object(context_id),
        )
        .unwrap();
        let trace_requirements = trace_profile.requirements();
        let trace_resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(trace_requirements.scopes, trace_requirements.leases)
                .with_class(
                    ResourceClass::TraceBytes,
                    trace_requirements.class(ResourceClass::TraceBytes).global,
                    trace_requirements
                        .class(ResourceClass::TraceBytes)
                        .per_scope,
                ),
            trace_requirements,
        )
        .unwrap();
        let mut recorder = trace_profile.create(&trace_resources).unwrap();
        recorder.start(None).unwrap();
        recorder.start_chunk(Some("chunk"), Some("title")).unwrap();
        recorder
            .append(TraceEntryKind::Log, 11, b"payload")
            .unwrap();
        let tracing =
            Arc::new(TracingService::new(TestClock(AtomicU64::new(20)), NoTraceSink, 1).unwrap());
        tracing.register_context(context_id, recorder).unwrap();

        let artifact_profile = ArtifactRegistryProductionProfile::new(ArtifactRegistryLimits::new(
            2, 2, 256, 256, 64, 256,
        ))
        .unwrap();
        let artifact_requirements = artifact_profile.requirements();
        let artifact_resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(artifact_requirements.scopes, artifact_requirements.leases)
                .with_class(
                    ResourceClass::ArtifactBytes,
                    artifact_requirements
                        .class(ResourceClass::ArtifactBytes)
                        .global,
                    artifact_requirements
                        .class(ResourceClass::ArtifactBytes)
                        .per_scope,
                ),
            artifact_requirements,
        )
        .unwrap();
        let registry = Arc::new(artifact_profile.create(&artifact_resources).unwrap());
        let runtime = Arc::new(
            ControlledDispatcher::with_object_kinds_and_topology(
                (),
                (),
                NoDeadlineScheduler,
                RuntimeLimits::new(8, 2).with_control_messages(8),
                BROWSER_OBJECT_KIND_CATALOG,
                ObjectTopologyMode::Complete,
            )
            .unwrap(),
        );
        runtime
            .register_object(ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag)
            .unwrap();
        runtime
            .register_object(
                context_id,
                Some(ObjectId::new(1)),
                BROWSER_CONTEXT_OBJECT_KIND.tag,
            )
            .unwrap();
        let artifacts = Arc::new(RuntimeArtifactService::new(
            Arc::clone(&registry),
            AtomicObjectIdSource::new(3).unwrap(),
            runtime.lifecycle_handle(),
        ));
        Fixture {
            tracing,
            artifacts,
            registry,
            trace_resources,
            artifact_resources,
            runtime,
        }
    }

    fn command<Spec: pwright_core::CommandSpec>(
        call_id: u64,
        object_id: u64,
        params: Spec::Params,
    ) -> Command<Spec> {
        Command::new(
            CallId::new(call_id),
            ObjectId::new(object_id),
            CallMetadata::default(),
            params,
        )
    }

    fn control_protocol() -> WireProtocol<ControlCodec> {
        WireProtocol::new_exact_directional(
            ControlCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
            FrameLimits::new(64, 0, 0, 64),
        )
    }

    fn producer_protocol() -> WireProtocol<TracingArtifactCodec> {
        let limits = FrameLimits::new(4_096, 0, 0, 4_096);
        WireProtocol::new_exact_directional(
            TracingArtifactCodec::new(TracingArtifactCodecLimits::new(256)),
            ProtocolVersion::new(1, 0),
            limits,
            limits,
        )
    }

    #[tokio::test]
    async fn typed_client_reaches_producer_through_binary_generated_server() {
        // Arrange
        let fixture = fixture();
        let producer = Arc::new(TracingArtifactService::new(
            Arc::clone(&fixture.tracing),
            Arc::clone(&fixture.artifacts),
            TraceArtifactEncodingLimits::new(256),
        ));
        let (client_transport, server_transport) = MemoryFrameTransport::pair(2);
        let server_transport = Arc::new(server_transport);
        let (channel, mut client_driver) =
            EncodedClientChannel::new(producer_protocol(), client_transport, 1);
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
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                BrowserContextKind::OBJECT_KIND,
            ))
            .unwrap();
        let handler = tracing_artifact_server_handlers(
            Arc::new(producer_protocol()),
            TracingArtifactServerServices {
                trace_stop_chunk_artifact: producer,
            },
        );

        // Act
        let call = context.stop_chunk_to_artifact();
        let server_turn = async {
            let frame = server_transport.receive().await.unwrap().unwrap();
            handler
                .handle_concurrently(frame, Arc::clone(&server_transport))
                .await
                .unwrap()
        };
        let (result, handled, delivered) =
            tokio::join!(call, server_turn, client_driver.receive_once());
        let result = result.unwrap();

        // Assert
        assert!(matches!(handled, ServerFrameDisposition::Handled(_)));
        assert!(matches!(delivered, Ok(DriverEvent::Delivered(_))));
        assert_eq!(result.artifact.id(), ObjectId::new(3));
        assert_eq!(result.entry_count, 1);
        assert_eq!(fixture.registry.metrics().unwrap().artifacts, 1);
        assert_eq!(client.channel().metrics().unwrap().awaiting_results(), 0);
    }

    #[tokio::test]
    async fn producer_hands_off_bytes_and_fences_artifact_create_before_success() {
        // Arrange
        let fixture = fixture();
        let producer = Arc::new(TracingArtifactService::new(
            Arc::clone(&fixture.tracing),
            Arc::clone(&fixture.artifacts),
            TraceArtifactEncodingLimits::new(256),
        ));
        let (server_transport, client_transport) = MemoryFrameTransport::pair(8);
        let ordered = OrderedTracingArtifactService::new(
            producer,
            OrderedRuntimeControl::new(
                Arc::clone(&fixture.runtime),
                RuntimeControlDriver::new(control_protocol(), server_transport),
                1,
            )
            .unwrap(),
        );

        // Act
        let result = ordered
            .call(command::<TraceStopChunkArtifact>(1, 2, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        let controls = [
            client_transport.receive().await.unwrap().unwrap(),
            client_transport.receive().await.unwrap().unwrap(),
            client_transport.receive().await.unwrap().unwrap(),
        ]
        .map(|frame| control_protocol().decode(frame).unwrap());
        let artifact_bytes_after_handoff = fixture
            .artifact_resources
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current;
        let stream = fixture
            .artifacts
            .call(command::<ArtifactOpenStream>(
                2,
                result.artifact_id.get(),
                (),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        let bytes = fixture
            .artifacts
            .call(command::<StreamRead>(
                3,
                stream.object_id.get(),
                StreamReadOptions::new(256),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();

        // Assert
        assert_eq!(
            controls[2],
            ControlMessage::Create {
                object_id: result.artifact_id,
                parent: Some(ObjectId::new(2)),
                kind: ARTIFACT_OBJECT_KIND.tag,
            }
        );
        assert_eq!(&bytes.as_bytes()[..PWRTRACE_MAGIC.len()], PWRTRACE_MAGIC);
        assert_eq!(result.chunk_id, 1);
        assert_eq!(result.entry_count, 1);
        assert_eq!(result.payload_bytes, 7);
        assert_eq!(u64::try_from(bytes.len()).unwrap(), result.encoded_bytes);
        assert_eq!(
            fixture
                .trace_resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
        assert_eq!(artifact_bytes_after_handoff, result.encoded_bytes);
        drop(bytes);
        assert_eq!(
            fixture
                .artifact_resources
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            result.encoded_bytes
        );
    }

    #[tokio::test]
    async fn encoding_rejection_releases_trace_chunk_and_reopens_recorder() {
        // Arrange
        let fixture = fixture();
        let producer = TracingArtifactService::new(
            Arc::clone(&fixture.tracing),
            Arc::clone(&fixture.artifacts),
            TraceArtifactEncodingLimits::new(1),
        );

        // Act
        let error = producer
            .call(command::<TraceStopChunkArtifact>(1, 2, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap_err();
        let restarted = fixture
            .tracing
            .call(command::<TraceStartChunk>(
                2,
                2,
                TraceStartChunkOptions::default(),
            ))
            .await
            .unwrap()
            .into_outcome();

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(restarted.is_ok());
        assert_eq!(fixture.registry.metrics().unwrap().artifacts, 0);
        assert_eq!(
            fixture
                .trace_resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn failed_create_fence_compensates_unpublished_artifact() {
        // Arrange
        let fixture = fixture();
        let producer = Arc::new(TracingArtifactService::new(
            Arc::clone(&fixture.tracing),
            Arc::clone(&fixture.artifacts),
            TraceArtifactEncodingLimits::new(256),
        ));
        let (server_transport, _client_transport) = MemoryFrameTransport::pair(0);
        let ordered = OrderedTracingArtifactService::new(
            producer,
            OrderedRuntimeControl::new(
                Arc::clone(&fixture.runtime),
                RuntimeControlDriver::new(control_protocol(), server_transport),
                1,
            )
            .unwrap(),
        );

        // Act
        let error = ordered
            .call(command::<TraceStopChunkArtifact>(1, 2, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::Cleanup);
        assert_eq!(fixture.registry.metrics().unwrap().artifacts, 0);
        assert!(!fixture.runtime.is_object_live(ObjectId::new(3)).unwrap());
        assert_eq!(
            fixture
                .artifact_resources
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            0
        );
        assert_eq!(
            fixture
                .trace_resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }
}
