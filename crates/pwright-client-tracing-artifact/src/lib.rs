//! Opt-in BrowserContext tracing-to-Artifact client facade.
//!
//! The facade materializes the Artifact only after the runtime's `Create`
//! announcement has arrived when announced-initializer mode is enabled.

use std::convert::Infallible;

use pwright_cap_tracing_artifact::{TraceStopChunkArtifact, TracingArtifactErrorDetails};
use pwright_client::{
    Artifact, BrowserContext, CallIdSource, CommandFuture, HandleCreationError, RemoteInitializer,
};
use pwright_core::{CallMetadata, Command, CommandResult, FailureKind, OperationError, Service};

#[derive(Clone)]
pub struct TraceArtifact<Channel, Ids> {
    pub artifact: Artifact<Channel, Ids>,
    pub chunk_id: u64,
    pub entry_count: u64,
    pub payload_bytes: u64,
    pub encoded_bytes: u64,
}

pub trait BrowserContextTracingArtifactExt {
    type Output;

    fn stop_chunk_to_artifact(&self) -> CommandFuture<Self::Output, TracingArtifactErrorDetails>;
}

impl<Channel, Ids> BrowserContextTracingArtifactExt for BrowserContext<Channel, Ids>
where
    Channel: Service<
            Command<TraceStopChunkArtifact>,
            Response = CommandResult<TraceStopChunkArtifact>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
{
    type Output = TraceArtifact<Channel, Ids>;

    fn stop_chunk_to_artifact(&self) -> CommandFuture<Self::Output, TracingArtifactErrorDetails> {
        let context = self.clone();
        let operation = self
            .as_remote()
            .dispatch::<TraceStopChunkArtifact>((), CallMetadata::default());
        Box::pin(async move {
            let result = operation.await?;
            let artifact = context
                .artifact_from_result(RemoteInitializer::new(
                    result.artifact_id,
                    Some(result.parent_id),
                    result.object_kind,
                ))
                .map_err(map_handle_creation_error)?;
            Ok(TraceArtifact {
                artifact,
                chunk_id: result.chunk_id,
                entry_count: result.entry_count,
                payload_bytes: result.payload_bytes,
                encoded_bytes: result.encoded_bytes,
            })
        })
    }
}

fn map_handle_creation_error(
    error: HandleCreationError,
) -> OperationError<TracingArtifactErrorDetails> {
    let kind = match error {
        HandleCreationError::CapacityExceeded { .. } => FailureKind::ResourceExhausted,
        HandleCreationError::ConnectionClosed => FailureKind::TransportClosed,
        HandleCreationError::ParentDisposed { .. }
        | HandleCreationError::AnnouncementDisposed { .. } => FailureKind::TargetClosed,
        HandleCreationError::KindMismatch { .. }
        | HandleCreationError::RootHasParent { .. }
        | HandleCreationError::ParentMismatch { .. }
        | HandleCreationError::DuplicateObject { .. }
        | HandleCreationError::MissingAnnouncement { .. }
        | HandleCreationError::AnnouncedInitializerMismatch { .. }
        | HandleCreationError::StateUnavailable => FailureKind::ProtocolViolation,
    };
    OperationError::new(
        kind,
        format!("tracing.stopChunkArtifact returned an invalid Artifact initializer: {error}"),
        Some(TracingArtifactErrorDetails::Lifecycle(error.to_string())),
    )
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};

    use pwright_browser_model::{ArtifactKind, BrowserContextKind, BrowserKind};
    use pwright_cap_tracing_artifact::{TraceArtifactResult, TraceStopChunkArtifact};
    use pwright_client::{AtomicCallIdSource, Client, ClientLimits, RemoteInitializer, RemoteKind};
    use pwright_core::{Command, CommandResult, FailureKind, ObjectId, Service};

    use super::BrowserContextTracingArtifactExt;

    struct Channel;

    impl Service<Command<TraceStopChunkArtifact>> for Channel {
        type Response = CommandResult<TraceStopChunkArtifact>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<TraceStopChunkArtifact>) -> Self::Future<'_> {
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(TraceArtifactResult::new(
                    ObjectId::new(3),
                    command.object_id(),
                    ArtifactKind::OBJECT_KIND,
                    7,
                    2,
                    11,
                    64,
                )),
            )))
        }
    }

    fn fixture() -> (
        Client<Channel, AtomicCallIdSource>,
        pwright_client::BrowserContext<Channel, AtomicCallIdSource>,
    ) {
        let client = Client::new(Channel, AtomicCallIdSource::new(10));
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
        (client, context)
    }

    #[tokio::test]
    async fn direct_result_materializes_artifact_and_preserves_summary() {
        // Arrange
        let (_client, context) = fixture();

        // Act
        let result = context.stop_chunk_to_artifact().await.unwrap();

        // Assert
        assert_eq!(result.artifact.id(), ObjectId::new(3));
        assert_eq!((result.chunk_id, result.entry_count), (7, 2));
        assert_eq!((result.payload_bytes, result.encoded_bytes), (11, 64));
    }

    #[tokio::test]
    async fn announced_mode_requires_matching_create_before_result() {
        // Arrange
        let (client, context) = fixture();
        client.require_announced_initializers();

        // Act
        let missing = match context.stop_chunk_to_artifact().await {
            Ok(_) => panic!("missing Create must not materialize an Artifact"),
            Err(error) => error,
        };
        client
            .announce(RemoteInitializer::new(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                ArtifactKind::OBJECT_KIND,
            ))
            .unwrap();
        let created = context.stop_chunk_to_artifact().await.unwrap();

        // Assert
        assert_eq!(missing.kind(), FailureKind::ProtocolViolation);
        assert_eq!(created.artifact.id(), ObjectId::new(3));
    }

    #[tokio::test]
    async fn client_capacity_rejects_artifact_materialization() {
        // Arrange
        let client =
            Client::with_limits(Channel, AtomicCallIdSource::new(10), ClientLimits::new(2));
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

        // Act
        let error = match context.stop_chunk_to_artifact().await {
            Ok(_) => panic!("capacity must reject Artifact materialization"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    }
}
