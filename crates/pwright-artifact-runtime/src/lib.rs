//! Atomic runtime lifecycle and ordered-control composition for Artifacts.
//!
//! The bounded registry remains independently usable and executor-neutral.
//! This opt-in layer couples its mutations to a controlled runtime object graph
//! and fences lifecycle control frames before successful command results.

use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::sync::Arc;

use pwright_artifact_service::{
    ArtifactRegistry, ArtifactRegistryError, artifact_registry_operation_error,
};
use pwright_browser_model::{ARTIFACT_OBJECT_KIND, STREAM_OBJECT_KIND};
use pwright_cap_artifact::{
    ArtifactDelete, ArtifactErrorDetails, ArtifactFailure, ArtifactOpenStream, StreamClose,
    StreamRead,
};
use pwright_core::{
    BudgetedBytes, Command, CommandResult, FailureKind, ObjectGraphError, ObjectId, ObjectIdSource,
    OperationError, Service,
};
use pwright_runtime::{ControlledLifecycleHandle, RuntimeStateError};
use pwright_runtime_control::{ControlMessageSource, OrderedRuntimeControl};
use pwright_server_artifact::ArtifactServerServices;
use pwright_transport::{FrameTransport, WireCodec};

/// Registry access coupled to one controlled runtime object graph.
pub struct RuntimeArtifactService<Ids> {
    registry: Arc<ArtifactRegistry>,
    object_ids: Ids,
    lifecycle: ControlledLifecycleHandle,
}

impl<Ids> RuntimeArtifactService<Ids> {
    pub const fn new(
        registry: Arc<ArtifactRegistry>,
        object_ids: Ids,
        lifecycle: ControlledLifecycleHandle,
    ) -> Self {
        Self {
            registry,
            object_ids,
            lifecycle,
        }
    }

    pub const fn registry(&self) -> &Arc<ArtifactRegistry> {
        &self.registry
    }

    /// Allocates the next connection-global object identity. Producers use
    /// this instead of owning a second ID source that could collide with
    /// Stream identities.
    pub fn allocate_object_id(&self) -> Result<ObjectId, RuntimeArtifactError>
    where
        Ids: ObjectIdSource,
    {
        self.object_ids
            .next_object_id()
            .map_err(|error| RuntimeArtifactError::ObjectId(error.to_string()))
    }

    /// Publishes one successful Artifact atomically to the registry and
    /// runtime. The caller must fence its queued `Create` before exposing an
    /// initializer in a separate producer command result.
    pub fn register_bytes(
        &self,
        artifact_id: ObjectId,
        parent: ObjectId,
        source: &[u8],
    ) -> Result<(), RuntimeArtifactError> {
        let runtime = self.lifecycle.prepare_object_creation(
            artifact_id,
            Some(parent),
            ARTIFACT_OBJECT_KIND.tag,
        )?;
        let registry = self
            .registry
            .prepare_register_bytes(artifact_id, parent, source)?;
        runtime.commit()?;
        registry.commit();
        Ok(())
    }

    /// Publishes one failed Artifact with the same lifecycle transaction used
    /// for byte-backed Artifacts.
    pub fn register_failure(
        &self,
        artifact_id: ObjectId,
        parent: ObjectId,
        error: &str,
    ) -> Result<(), RuntimeArtifactError> {
        let runtime = self.lifecycle.prepare_object_creation(
            artifact_id,
            Some(parent),
            ARTIFACT_OBJECT_KIND.tag,
        )?;
        let registry = self
            .registry
            .prepare_register_failure(artifact_id, parent, error)?;
        runtime.commit()?;
        registry.commit();
        Ok(())
    }

    /// Publishes a pre-budgeted Artifact source without a staging copy.
    pub fn register_budgeted_bytes(
        &self,
        artifact_id: ObjectId,
        parent: ObjectId,
        source: BudgetedBytes,
    ) -> Result<(), RuntimeArtifactError> {
        let runtime = self.lifecycle.prepare_object_creation(
            artifact_id,
            Some(parent),
            ARTIFACT_OBJECT_KIND.tag,
        )?;
        let registry =
            self.registry
                .prepare_register_budgeted_bytes(artifact_id, parent, source)?;
        runtime.commit()?;
        registry.commit();
        Ok(())
    }

    fn open_stream(
        &self,
        artifact_id: ObjectId,
    ) -> Result<pwright_cap_artifact::StreamInitializer, RuntimeArtifactError>
    where
        Ids: ObjectIdSource,
    {
        let stream_id = self
            .object_ids
            .next_object_id()
            .map_err(|error| RuntimeArtifactError::ObjectId(error.to_string()))?;
        let runtime = self.lifecycle.prepare_object_creation(
            stream_id,
            Some(artifact_id),
            STREAM_OBJECT_KIND.tag,
        )?;
        let registry = self.registry.prepare_open_stream(artifact_id, stream_id)?;
        runtime.commit()?;
        Ok(registry.commit())
    }

    pub fn retire_artifact(&self, artifact_id: ObjectId) -> Result<(), RuntimeArtifactError> {
        let runtime = self.lifecycle.prepare_object_disposal(artifact_id)?;
        let registry = self.registry.prepare_artifact_disposal(artifact_id)?;
        if runtime.disposal_order() != registry.object_ids_child_first() {
            return Err(RuntimeArtifactError::LifecycleDiverged {
                object_id: artifact_id,
            });
        }
        runtime.commit()?;
        registry.commit()?;
        Ok(())
    }

    fn close_stream(&self, stream_id: ObjectId) -> Result<(), RuntimeArtifactError> {
        let runtime = self.lifecycle.prepare_object_disposal(stream_id)?;
        let registry = self.registry.prepare_stream_disposal(stream_id)?;
        if runtime.disposal_order() != [stream_id] {
            return Err(RuntimeArtifactError::LifecycleDiverged {
                object_id: stream_id,
            });
        }
        runtime.commit()?;
        registry.commit()?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum RuntimeArtifactError {
    Registry(ArtifactRegistryError),
    Runtime(RuntimeStateError),
    ObjectId(String),
    LifecycleDiverged { object_id: ObjectId },
}

impl Display for RuntimeArtifactError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry(error) => Display::fmt(error, formatter),
            Self::Runtime(error) => Display::fmt(error, formatter),
            Self::ObjectId(message) => formatter.write_str(message),
            Self::LifecycleDiverged { object_id } => write!(
                formatter,
                "Artifact or Stream {object_id} has different registry and runtime disposal topology"
            ),
        }
    }
}

impl Error for RuntimeArtifactError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Registry(error) => Some(error),
            Self::Runtime(error) => Some(error),
            Self::ObjectId(_) | Self::LifecycleDiverged { .. } => None,
        }
    }
}

impl From<ArtifactRegistryError> for RuntimeArtifactError {
    fn from(error: ArtifactRegistryError) -> Self {
        Self::Registry(error)
    }
}

impl From<RuntimeStateError> for RuntimeArtifactError {
    fn from(error: RuntimeStateError) -> Self {
        Self::Runtime(error)
    }
}

fn runtime_operation_error(error: RuntimeArtifactError) -> OperationError<ArtifactErrorDetails> {
    match error {
        RuntimeArtifactError::Registry(error) => artifact_registry_operation_error(error),
        RuntimeArtifactError::ObjectId(message) => OperationError::new(
            FailureKind::ResourceExhausted,
            "Artifact object ID capacity was unavailable",
            Some(ArtifactErrorDetails::Resource(message)),
        ),
        RuntimeArtifactError::Runtime(error) => {
            let message = error.to_string();
            let kind = match error {
                RuntimeStateError::TransportClosed => FailureKind::TargetClosed,
                RuntimeStateError::ObjectGraph(ObjectGraphError::CapacityExceeded { .. })
                | RuntimeStateError::ControlMessageCapacityExceeded { .. }
                | RuntimeStateError::LifecycleTransactionInProgress
                | RuntimeStateError::LifecycleTransactionIdExhausted
                | RuntimeStateError::ProductionResource(_)
                | RuntimeStateError::Resource(_) => FailureKind::ResourceExhausted,
                _ => FailureKind::Cleanup,
            };
            OperationError::new(
                kind,
                "Artifact lifecycle transaction failed",
                Some(ArtifactErrorDetails::Backend(message)),
            )
        }
        RuntimeArtifactError::LifecycleDiverged { object_id } => OperationError::new(
            FailureKind::Cleanup,
            "Artifact registry and runtime lifecycle diverged",
            Some(ArtifactErrorDetails::Backend(format!(
                "Artifact or Stream {object_id} has different disposal topology"
            ))),
        ),
    }
}

impl<Ids> Service<Command<ArtifactOpenStream>> for RuntimeArtifactService<Ids>
where
    Ids: ObjectIdSource,
{
    type Response = CommandResult<ArtifactOpenStream>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<ArtifactOpenStream>) -> Self::Future<'_> {
        let result = self
            .open_stream(command.object_id())
            .map_err(runtime_operation_error);
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Ids> Service<Command<ArtifactFailure>> for RuntimeArtifactService<Ids>
where
    Ids: ObjectIdSource,
{
    type Response = CommandResult<ArtifactFailure>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<ArtifactFailure>) -> Self::Future<'_> {
        let result = self
            .registry
            .failure(command.object_id(), command.call_id())
            .map_err(artifact_registry_operation_error);
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Ids> Service<Command<ArtifactDelete>> for RuntimeArtifactService<Ids>
where
    Ids: ObjectIdSource,
{
    type Response = CommandResult<ArtifactDelete>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<ArtifactDelete>) -> Self::Future<'_> {
        let result = self
            .retire_artifact(command.object_id())
            .map_err(runtime_operation_error);
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Ids> Service<Command<StreamRead>> for RuntimeArtifactService<Ids>
where
    Ids: ObjectIdSource,
{
    type Response = CommandResult<StreamRead>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<StreamRead>) -> Self::Future<'_> {
        let result = usize::try_from(command.params().max_bytes())
            .map_err(|_| ArtifactRegistryError::LimitOverflow)
            .and_then(|requested| {
                self.registry
                    .prepare_read(command.object_id(), requested, command.call_id())
            })
            .and_then(|prepared| prepared.commit().map_err(ArtifactRegistryError::Stream))
            .map_err(artifact_registry_operation_error);
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Ids> Service<Command<StreamClose>> for RuntimeArtifactService<Ids>
where
    Ids: ObjectIdSource,
{
    type Response = CommandResult<StreamClose>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<StreamClose>) -> Self::Future<'_> {
        let result = self
            .close_stream(command.object_id())
            .map_err(runtime_operation_error);
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

/// Command service that holds the ordered control barrier across lifecycle
/// mutation and the exact Create/Dispose publication fence.
pub struct OrderedRuntimeArtifactService<Ids, Source, Codec, Transport> {
    service: Arc<RuntimeArtifactService<Ids>>,
    control: OrderedRuntimeControl<Source, Codec, Transport>,
}

impl<Ids, Source, Codec, Transport> OrderedRuntimeArtifactService<Ids, Source, Codec, Transport> {
    pub const fn new(
        service: Arc<RuntimeArtifactService<Ids>>,
        control: OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Self {
        Self { service, control }
    }
}

pub type SharedRuntimeArtifactServerServices<Ids, Source, Codec, Transport> =
    ArtifactServerServices<
        OrderedRuntimeArtifactService<Ids, Source, Codec, Transport>,
        OrderedRuntimeArtifactService<Ids, Source, Codec, Transport>,
        OrderedRuntimeArtifactService<Ids, Source, Codec, Transport>,
        OrderedRuntimeArtifactService<Ids, Source, Codec, Transport>,
        OrderedRuntimeArtifactService<Ids, Source, Codec, Transport>,
    >;

pub fn runtime_artifact_server_services<Ids, Source, Codec, Transport>(
    service: Arc<OrderedRuntimeArtifactService<Ids, Source, Codec, Transport>>,
) -> SharedRuntimeArtifactServerServices<Ids, Source, Codec, Transport> {
    ArtifactServerServices {
        artifact_delete: Arc::clone(&service),
        artifact_failure: Arc::clone(&service),
        artifact_open_stream: Arc::clone(&service),
        stream_close: Arc::clone(&service),
        stream_read: service,
    }
}

fn ordering_failure<Spec>(call_id: pwright_core::CallId, message: String) -> CommandResult<Spec>
where
    Spec: pwright_core::CommandSpec<ErrorDetails = ArtifactErrorDetails>,
{
    CommandResult::new(
        call_id,
        Err(OperationError::new(
            FailureKind::Cleanup,
            "Artifact lifecycle publication failed",
            Some(ArtifactErrorDetails::Backend(message)),
        )),
    )
}

impl<Ids, Source, Codec, Transport> Service<Command<ArtifactOpenStream>>
    for OrderedRuntimeArtifactService<Ids, Source, Codec, Transport>
where
    Ids: ObjectIdSource + 'static,
    Source: ControlMessageSource + Send + Sync + 'static,
    Codec: WireCodec<pwright_core::ControlMessage> + 'static,
    Transport: FrameTransport + 'static,
{
    type Response = CommandResult<ArtifactOpenStream>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<ArtifactOpenStream>) -> Self::Future<'_> {
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
            let published = barrier
                .send_through_creation(
                    initializer.object_id,
                    Some(initializer.parent_id),
                    initializer.object_kind,
                )
                .await
                .is_ok();
            if published {
                return Ok(result);
            }
            let cleanup = match self
                .service
                .call(Command::<StreamClose>::new(
                    call_id,
                    initializer.object_id,
                    pwright_core::CallMetadata::default(),
                    (),
                ))
                .await
            {
                Ok(cleanup) => cleanup,
                Err(never) => match never {},
            };
            let cleanup = if cleanup.outcome().is_ok() {
                let _ = barrier.send_through_disposal(initializer.object_id).await;
                "the created Stream was retired"
            } else {
                "the created Stream could not be retired"
            };
            Ok(ordering_failure(
                call_id,
                format!("Stream Create control fence was not delivered; {cleanup}"),
            ))
        })
    }
}

macro_rules! passthrough_service {
    ($spec:ty) => {
        impl<Ids, Source, Codec, Transport> Service<Command<$spec>>
            for OrderedRuntimeArtifactService<Ids, Source, Codec, Transport>
        where
            Ids: ObjectIdSource + 'static,
            Source: Send + Sync + 'static,
            Codec: Send + Sync + 'static,
            Transport: Send + Sync + 'static,
        {
            type Response = CommandResult<$spec>;
            type Error = Infallible;
            type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

            fn call(&self, command: Command<$spec>) -> Self::Future<'_> {
                self.service.call(command)
            }
        }
    };
}

passthrough_service!(ArtifactFailure);
passthrough_service!(StreamRead);

macro_rules! ordered_disposal_service {
    ($spec:ty) => {
        impl<Ids, Source, Codec, Transport> Service<Command<$spec>>
            for OrderedRuntimeArtifactService<Ids, Source, Codec, Transport>
        where
            Ids: ObjectIdSource + 'static,
            Source: ControlMessageSource + Send + Sync + 'static,
            Codec: WireCodec<pwright_core::ControlMessage> + 'static,
            Transport: FrameTransport + 'static,
        {
            type Response = CommandResult<$spec>;
            type Error = Infallible;
            type Future<'a> =
                Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

            fn call(&self, command: Command<$spec>) -> Self::Future<'_> {
                let call_id = command.call_id();
                let object_id = command.object_id();
                Box::pin(async move {
                    let barrier = match self.control.begin_barrier().await {
                        Ok(barrier) => barrier,
                        Err(error) => return Ok(ordering_failure(call_id, error.to_string())),
                    };
                    let result = match self.service.call(command).await {
                        Ok(result) => result,
                        Err(never) => match never {},
                    };
                    if result.outcome().is_err() {
                        return Ok(result);
                    }
                    match barrier.send_through_disposal(object_id).await {
                        Ok(_) => Ok(result),
                        Err(_) => Ok(ordering_failure(
                            call_id,
                            "object Dispose control fence was not delivered".to_owned(),
                        )),
                    }
                })
            }
        }
    };
}

ordered_disposal_service!(ArtifactDelete);
ordered_disposal_service!(StreamClose);

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_artifact_service::{
        ArtifactRegistry, ArtifactRegistryLimits, ArtifactRegistryProductionProfile,
    };
    use pwright_browser_model::{BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG};
    use pwright_cap_artifact::{ArtifactDelete, ArtifactOpenStream, StreamClose};
    use pwright_codec_control::ControlCodec;
    use pwright_core::{
        AtomicObjectIdSource, CallId, CallMetadata, Command, ControlMessage, ObjectId,
        ProductionResourceLedger, ProtocolVersion, ResourceClass, ResourceLimits, ResourceScope,
        Service,
    };
    use pwright_runtime::{
        ControlledDispatcher, NoDeadlineScheduler, ObjectTopologyMode, RuntimeLimits,
        RuntimeStateError,
    };
    use pwright_runtime_control::{OrderedRuntimeControl, RuntimeControlDriver};
    use pwright_trace_format::{TraceArtifactEncodingLimits, encode_trace_artifact};
    use pwright_trace_model::{TraceEntryKind, TraceLimits, TraceRecorder};
    use pwright_transport::{FrameLimits, FrameTransport, WireProtocol};
    use pwright_transport_memory::MemoryFrameTransport;

    use super::{OrderedRuntimeArtifactService, RuntimeArtifactService};

    type TestRuntime = ControlledDispatcher<(), (), NoDeadlineScheduler>;

    fn registry() -> Arc<ArtifactRegistry> {
        let profile =
            ArtifactRegistryProductionProfile::new(ArtifactRegistryLimits::new(2, 2, 8, 3, 8, 8))
                .unwrap();
        let requirements = profile.requirements();
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::ArtifactBytes,
                requirements.class(ResourceClass::ArtifactBytes).global,
                requirements.class(ResourceClass::ArtifactBytes).per_scope,
            ),
            requirements,
        )
        .unwrap();
        Arc::new(profile.create(&resources).unwrap())
    }

    fn runtime() -> Arc<TestRuntime> {
        let runtime = Arc::new(
            ControlledDispatcher::with_object_kinds_and_topology(
                (),
                (),
                NoDeadlineScheduler,
                RuntimeLimits::new(5, 2).with_control_messages(8),
                BROWSER_OBJECT_KIND_CATALOG,
                ObjectTopologyMode::Complete,
            )
            .unwrap(),
        );
        runtime
            .register_object(ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag)
            .unwrap();
        runtime
    }

    fn control_protocol() -> WireProtocol<ControlCodec> {
        WireProtocol::new_exact_directional(
            ControlCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
            FrameLimits::new(64, 0, 0, 64),
        )
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

    #[test]
    fn transport_close_rolls_back_prepared_registry_publication() {
        // Arrange
        let registry = registry();
        let runtime = runtime();
        let lifecycle = runtime.lifecycle_handle();
        let runtime_preparation = lifecycle
            .prepare_object_creation(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                pwright_browser_model::ARTIFACT_OBJECT_KIND.tag,
            )
            .unwrap();
        let registry_preparation = registry
            .prepare_register_bytes(ObjectId::new(2), ObjectId::new(1), b"abc")
            .unwrap();

        // Act
        runtime.close_transport().unwrap();
        let commit = runtime_preparation.commit();
        drop(registry_preparation);

        // Assert
        assert_eq!(
            commit,
            Err(RuntimeStateError::LifecycleTransactionUnavailable)
        );
        assert!(!lifecycle.is_object_live(ObjectId::new(2)).unwrap());
        assert_eq!(registry.metrics().unwrap().artifacts, 0);
    }

    #[tokio::test]
    async fn ordered_commands_publish_create_and_child_first_disposals_before_success() {
        // Arrange
        let registry = registry();
        let runtime = runtime();
        let service = Arc::new(RuntimeArtifactService::new(
            Arc::clone(&registry),
            AtomicObjectIdSource::new(3).unwrap(),
            runtime.lifecycle_handle(),
        ));
        service
            .register_bytes(ObjectId::new(2), ObjectId::new(1), b"abcdef")
            .unwrap();
        let (server_transport, client_transport) = MemoryFrameTransport::pair(8);
        let ordered = OrderedRuntimeArtifactService::new(
            Arc::clone(&service),
            OrderedRuntimeControl::new(
                Arc::clone(&runtime),
                RuntimeControlDriver::new(control_protocol(), server_transport),
                2,
            )
            .unwrap(),
        );

        // Act
        let opened = ordered
            .call(command::<ArtifactOpenStream>(1, 2, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        let create_frames = [
            client_transport.receive().await.unwrap().unwrap(),
            client_transport.receive().await.unwrap().unwrap(),
            client_transport.receive().await.unwrap().unwrap(),
        ];
        let closed = ordered
            .call(command::<StreamClose>(2, opened.object_id.get(), ()))
            .await
            .unwrap()
            .into_outcome();
        let stream_dispose = client_transport.receive().await.unwrap().unwrap();
        let deleted = ordered
            .call(command::<ArtifactDelete>(3, 2, ()))
            .await
            .unwrap()
            .into_outcome();
        let artifact_dispose = client_transport.receive().await.unwrap().unwrap();

        // Assert
        let decoded = create_frames
            .into_iter()
            .map(|frame| control_protocol().decode(frame).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            decoded,
            [
                ControlMessage::Create {
                    object_id: ObjectId::new(1),
                    parent: None,
                    kind: BROWSER_OBJECT_KIND.tag,
                },
                ControlMessage::Create {
                    object_id: ObjectId::new(2),
                    parent: Some(ObjectId::new(1)),
                    kind: pwright_browser_model::ARTIFACT_OBJECT_KIND.tag,
                },
                ControlMessage::Create {
                    object_id: ObjectId::new(3),
                    parent: Some(ObjectId::new(2)),
                    kind: pwright_browser_model::STREAM_OBJECT_KIND.tag,
                },
            ]
        );
        assert_eq!(
            control_protocol().decode(stream_dispose).unwrap(),
            ControlMessage::Dispose {
                object_id: ObjectId::new(3)
            }
        );
        assert_eq!(
            control_protocol().decode(artifact_dispose).unwrap(),
            ControlMessage::Dispose {
                object_id: ObjectId::new(2)
            }
        );
        assert!(closed.is_ok());
        assert!(deleted.is_ok());
        assert_eq!(registry.metrics().unwrap().artifacts, 0);
        assert_eq!(registry.metrics().unwrap().streams, 0);
        assert!(!runtime.is_object_live(ObjectId::new(2)).unwrap());
        assert!(!runtime.is_object_live(ObjectId::new(3)).unwrap());
    }

    #[tokio::test]
    async fn failed_create_fence_compensates_the_stream_locally() {
        // Arrange
        let registry = registry();
        let runtime = runtime();
        let service = Arc::new(RuntimeArtifactService::new(
            Arc::clone(&registry),
            AtomicObjectIdSource::new(3).unwrap(),
            runtime.lifecycle_handle(),
        ));
        service
            .register_bytes(ObjectId::new(2), ObjectId::new(1), b"abcdef")
            .unwrap();
        let (server_transport, _client_transport) = MemoryFrameTransport::pair(0);
        let ordered = OrderedRuntimeArtifactService::new(
            Arc::clone(&service),
            OrderedRuntimeControl::new(
                Arc::clone(&runtime),
                RuntimeControlDriver::new(control_protocol(), server_transport),
                1,
            )
            .unwrap(),
        );

        // Act
        let error = ordered
            .call(command::<ArtifactOpenStream>(1, 2, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), pwright_core::FailureKind::Cleanup);
        assert_eq!(registry.metrics().unwrap().streams, 0);
        assert!(!runtime.is_object_live(ObjectId::new(3)).unwrap());
        assert!(runtime.is_object_live(ObjectId::new(2)).unwrap());
    }

    #[tokio::test]
    async fn runtime_object_capacity_rejects_before_stream_registry_mutation() {
        // Arrange
        let registry = registry();
        let runtime = Arc::new(
            ControlledDispatcher::with_object_kinds_and_topology(
                (),
                (),
                NoDeadlineScheduler,
                RuntimeLimits::new(2, 1).with_control_messages(4),
                BROWSER_OBJECT_KIND_CATALOG,
                ObjectTopologyMode::Complete,
            )
            .unwrap(),
        );
        runtime
            .register_object(ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag)
            .unwrap();
        let service = RuntimeArtifactService::new(
            Arc::clone(&registry),
            AtomicObjectIdSource::new(3).unwrap(),
            runtime.lifecycle_handle(),
        );
        service
            .register_bytes(ObjectId::new(2), ObjectId::new(1), b"abcdef")
            .unwrap();

        // Act
        let error = service
            .call(command::<ArtifactOpenStream>(1, 2, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), pwright_core::FailureKind::ResourceExhausted);
        assert_eq!(registry.metrics().unwrap().streams, 0);
        assert!(!runtime.is_object_live(ObjectId::new(3)).unwrap());
    }

    #[tokio::test]
    async fn encoded_trace_buffer_moves_into_artifact_then_releases_trace_bytes() {
        // Arrange
        let profile = ArtifactRegistryProductionProfile::new(ArtifactRegistryLimits::new(
            1, 1, 128, 16, 32, 16,
        ))
        .unwrap();
        let requirements = profile.requirements();
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::ArtifactBytes,
                requirements.class(ResourceClass::ArtifactBytes).global,
                requirements.class(ResourceClass::ArtifactBytes).per_scope,
            ),
            requirements,
        )
        .unwrap();
        let registry = Arc::new(profile.create(&resources).unwrap());
        let runtime = runtime();
        let service = RuntimeArtifactService::new(
            Arc::clone(&registry),
            AtomicObjectIdSource::new(3).unwrap(),
            runtime.lifecycle_handle(),
        );
        let trace_ledger = pwright_core::ResourceLedger::new(
            ResourceLimits::deny_all(1, 8).with_class(ResourceClass::TraceBytes, 128, 128),
        );
        let mut recorder = TraceRecorder::new(
            TraceLimits::new(16, 1, 32, 64, 1),
            trace_ledger.clone(),
            ResourceScope::Connection,
        )
        .unwrap();
        recorder.start(None).unwrap();
        recorder.start_chunk(None, None).unwrap();
        recorder.append(TraceEntryKind::Log, 1, b"trace").unwrap();
        let chunk = recorder.stop_chunk().unwrap();
        let encoded = encode_trace_artifact(
            TraceArtifactEncodingLimits::new(128),
            registry.ledger(),
            ObjectId::new(2),
            ObjectId::new(1),
            &chunk,
        )
        .unwrap();
        let encoded_len = u64::try_from(encoded.len()).unwrap();

        // Act
        service
            .register_budgeted_bytes(ObjectId::new(2), ObjectId::new(1), encoded)
            .unwrap();
        drop(chunk);
        let trace_after_handoff = trace_ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::TraceBytes)
            .current;
        let artifact_after_handoff = resources
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current;
        service
            .call(command::<ArtifactDelete>(2, 2, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();

        // Assert
        assert_eq!(trace_after_handoff, 0);
        assert_eq!(artifact_after_handoff, encoded_len);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            0
        );
    }
}
