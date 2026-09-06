//! Opt-in Playwright-shaped Artifact and Stream client methods.
//!
//! Importing this crate adds only typed dispatch helpers. It installs no
//! capability, codec, transport, runtime, producer, filesystem, or task.

use std::convert::Infallible;

use pwright_cap_artifact::{
    ArtifactDelete, ArtifactErrorDetails, ArtifactFailure, ArtifactFailureResult,
    ArtifactOpenStream, ArtifactReadResult, StreamClose, StreamRead, StreamReadOptions,
};
use pwright_client::{
    Artifact, CallIdSource, CommandFuture, HandleCreationError, RemoteInitializer, Stream,
};
use pwright_core::{CallMetadata, Command, CommandResult, FailureKind, OperationError, Service};

pub trait ArtifactExt {
    type Stream;

    fn create_read_stream(&self) -> CommandFuture<Self::Stream, ArtifactErrorDetails>;

    fn failure(&self) -> CommandFuture<ArtifactFailureResult, ArtifactErrorDetails>;

    fn delete(&self) -> CommandFuture<(), ArtifactErrorDetails>;
}

impl<Channel, Ids> ArtifactExt for Artifact<Channel, Ids>
where
    Channel: Service<
            Command<ArtifactOpenStream>,
            Response = CommandResult<ArtifactOpenStream>,
            Error = Infallible,
        > + Service<
            Command<ArtifactFailure>,
            Response = CommandResult<ArtifactFailure>,
            Error = Infallible,
        > + Service<
            Command<ArtifactDelete>,
            Response = CommandResult<ArtifactDelete>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
{
    type Stream = Stream<Channel, Ids>;

    fn create_read_stream(&self) -> CommandFuture<Self::Stream, ArtifactErrorDetails> {
        let artifact = self.clone();
        let creation = self
            .as_remote()
            .dispatch::<ArtifactOpenStream>((), CallMetadata::default());
        Box::pin(async move {
            let result = creation.await?;
            artifact
                .stream_from_result(RemoteInitializer::new(
                    result.object_id,
                    Some(result.parent_id),
                    result.object_kind,
                ))
                .map_err(map_handle_creation_error)
        })
    }

    fn failure(&self) -> CommandFuture<ArtifactFailureResult, ArtifactErrorDetails> {
        self.as_remote()
            .dispatch::<ArtifactFailure>((), CallMetadata::default())
    }

    fn delete(&self) -> CommandFuture<(), ArtifactErrorDetails> {
        idempotent_target_close(
            self.as_remote()
                .dispatch::<ArtifactDelete>((), CallMetadata::default()),
        )
    }
}

pub trait StreamExt {
    fn read(&self, max_bytes: u32) -> CommandFuture<ArtifactReadResult, ArtifactErrorDetails>;

    fn close(&self) -> CommandFuture<(), ArtifactErrorDetails>;
}

impl<Channel, Ids> StreamExt for Stream<Channel, Ids>
where
    Channel: Service<Command<StreamRead>, Response = CommandResult<StreamRead>, Error = Infallible>
        + Service<Command<StreamClose>, Response = CommandResult<StreamClose>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
{
    fn read(&self, max_bytes: u32) -> CommandFuture<ArtifactReadResult, ArtifactErrorDetails> {
        self.as_remote()
            .dispatch::<StreamRead>(StreamReadOptions::new(max_bytes), CallMetadata::default())
    }

    fn close(&self) -> CommandFuture<(), ArtifactErrorDetails> {
        idempotent_target_close(
            self.as_remote()
                .dispatch::<StreamClose>((), CallMetadata::default()),
        )
    }
}

fn idempotent_target_close(
    operation: CommandFuture<(), ArtifactErrorDetails>,
) -> CommandFuture<(), ArtifactErrorDetails> {
    Box::pin(async move {
        match operation.await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == FailureKind::TargetClosed => Ok(()),
            Err(error) => Err(error),
        }
    })
}

fn map_handle_creation_error(error: HandleCreationError) -> OperationError<ArtifactErrorDetails> {
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
        format!("Artifact.createReadStream returned an invalid Stream initializer: {error}"),
        Some(ArtifactErrorDetails::Backend(error.to_string())),
    )
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};
    use std::sync::{Arc, Mutex};

    use pwright_browser_model::{ArtifactKind, BrowserKind, StreamKind};
    use pwright_cap_artifact::{
        ArtifactDelete, ArtifactFailure, ArtifactFailureResult, ArtifactOpenStream, StreamClose,
        StreamInitializer, StreamRead,
    };
    use pwright_client::{AtomicCallIdSource, Client, ClientLimits, RemoteInitializer, RemoteKind};
    use pwright_core::{
        BudgetedBytes, CallId, Command, CommandResult, FailureKind, ObjectId, ResourceClass,
        ResourceLedger, ResourceLimits, ResourceScope, Service,
    };

    use super::{ArtifactExt, StreamExt};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Method {
        OpenStream,
        Failure,
        Delete,
        Read(u32),
        Close,
    }

    struct Channel {
        calls: Arc<Mutex<Vec<(Method, CallId, ObjectId)>>>,
    }

    impl Service<Command<ArtifactOpenStream>> for Channel {
        type Response = CommandResult<ArtifactOpenStream>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<ArtifactOpenStream>) -> Self::Future<'_> {
            self.calls.lock().unwrap().push((
                Method::OpenStream,
                command.call_id(),
                command.object_id(),
            ));
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(StreamInitializer::new(
                    ObjectId::new(11),
                    command.object_id(),
                    StreamKind::OBJECT_KIND,
                )),
            )))
        }
    }

    impl Service<Command<ArtifactFailure>> for Channel {
        type Response = CommandResult<ArtifactFailure>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<ArtifactFailure>) -> Self::Future<'_> {
            self.calls.lock().unwrap().push((
                Method::Failure,
                command.call_id(),
                command.object_id(),
            ));
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(ArtifactFailureResult::default()),
            )))
        }
    }

    macro_rules! impl_unit_service {
        ($spec:ty, $method:expr) => {
            impl Service<Command<$spec>> for Channel {
                type Response = CommandResult<$spec>;
                type Error = Infallible;
                type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

                fn call(&self, command: Command<$spec>) -> Self::Future<'_> {
                    self.calls.lock().unwrap().push((
                        $method,
                        command.call_id(),
                        command.object_id(),
                    ));
                    ready(Ok(CommandResult::new(command.call_id(), Ok(()))))
                }
            }
        };
    }

    impl_unit_service!(ArtifactDelete, Method::Delete);
    impl_unit_service!(StreamClose, Method::Close);

    impl Service<Command<StreamRead>> for Channel {
        type Response = CommandResult<StreamRead>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<StreamRead>) -> Self::Future<'_> {
            self.calls.lock().unwrap().push((
                Method::Read(command.params().max_bytes()),
                command.call_id(),
                command.object_id(),
            ));
            let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
                ResourceClass::ArtifactBytes,
                5,
                5,
            ));
            let bytes = BudgetedBytes::copy_from_slice(
                &ledger,
                ResourceScope::Call(command.call_id()),
                ResourceClass::ArtifactBytes,
                b"chunk",
            )
            .unwrap();
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(pwright_cap_artifact::ArtifactReadResult::new(bytes, true)),
            )))
        }
    }

    type TestClient = Client<Channel, AtomicCallIdSource>;
    type TestArtifact = pwright_client::Artifact<Channel, AtomicCallIdSource>;
    type RecordedCalls = Arc<Mutex<Vec<(Method, CallId, ObjectId)>>>;
    type Fixture = (TestClient, TestArtifact, RecordedCalls);

    fn fixture(limits: ClientLimits) -> Fixture {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::with_limits(
            Channel {
                calls: Arc::clone(&calls),
            },
            AtomicCallIdSource::new(20),
            limits,
        );
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BrowserKind::OBJECT_KIND,
            ))
            .unwrap();
        let artifact = browser
            .artifact_from_result(RemoteInitializer::new(
                ObjectId::new(10),
                Some(ObjectId::new(1)),
                ArtifactKind::OBJECT_KIND,
            ))
            .unwrap();
        (client, artifact, calls)
    }

    #[tokio::test]
    async fn artifact_and_stream_facades_dispatch_all_five_typed_commands() {
        // Arrange
        let (_client, artifact, calls) = fixture(ClientLimits::new(3));

        // Act
        let failure = artifact.failure().await.unwrap();
        let stream = artifact.create_read_stream().await.unwrap();
        let read = stream.read(5).await.unwrap();
        stream.close().await.unwrap();
        artifact.delete().await.unwrap();

        // Assert
        assert_eq!(failure.error(), None);
        assert_eq!(read.as_bytes(), b"chunk");
        assert!(read.eof());
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[
                (Method::Failure, CallId::new(20), ObjectId::new(10)),
                (Method::OpenStream, CallId::new(21), ObjectId::new(10)),
                (Method::Read(5), CallId::new(22), ObjectId::new(11)),
                (Method::Close, CallId::new(23), ObjectId::new(11)),
                (Method::Delete, CallId::new(24), ObjectId::new(10)),
            ]
        );
    }

    #[tokio::test]
    async fn remote_stream_result_requires_an_exact_create_announcement() {
        // Arrange
        let (client, artifact, _) = fixture(ClientLimits::new(3));
        client.require_announced_initializers();

        // Act
        let error = artifact
            .create_read_stream()
            .await
            .expect_err("missing Stream Create must fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(client.object_metrics().unwrap().current(), 2);
    }

    #[tokio::test]
    async fn direct_stream_result_respects_the_shared_client_object_limit() {
        // Arrange
        let (client, artifact, _) = fixture(ClientLimits::new(2));

        // Act
        let error = artifact
            .create_read_stream()
            .await
            .expect_err("full client object table must reject Stream");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert_eq!(client.object_metrics().unwrap().current(), 2);
    }
}
