//! Opt-in BrowserContext creation methods for `pwright-client` handles.

use std::convert::Infallible;

use pwright_cap_context_creation::{BrowserNewContext, ContextCreationErrorDetails};
use pwright_client::{
    Browser, BrowserContext, CallIdSource, CommandFuture, HandleCreationError, RemoteInitializer,
};
use pwright_core::{CallMetadata, Command, CommandResult, FailureKind, OperationError, Service};

pub trait BrowserNewContextExt {
    type Context;

    fn new_context(&self) -> CommandFuture<Self::Context, ContextCreationErrorDetails>;
}

impl<Channel, Ids> BrowserNewContextExt for Browser<Channel, Ids>
where
    Channel: Service<
            Command<BrowserNewContext>,
            Response = CommandResult<BrowserNewContext>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
{
    type Context = BrowserContext<Channel, Ids>;

    fn new_context(&self) -> CommandFuture<Self::Context, ContextCreationErrorDetails> {
        let browser = self.clone();
        let creation = self
            .as_remote()
            .dispatch::<BrowserNewContext>((), CallMetadata::default());
        Box::pin(async move {
            let result = creation.await?;
            let initializer = RemoteInitializer::new(
                result.object_id,
                Some(result.parent_id),
                result.object_kind,
            );
            browser
                .context_from_result(initializer)
                .map_err(map_handle_creation_error)
        })
    }
}

fn map_handle_creation_error(
    error: HandleCreationError,
) -> OperationError<ContextCreationErrorDetails> {
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
        format!("Browser.newContext returned an invalid object initializer: {error}"),
        Some(ContextCreationErrorDetails::InvalidBackendResponse(
            error.to_string(),
        )),
    )
}

#[cfg(test)]
mod tests {
    use std::future::{Ready, ready};
    use std::sync::{Arc, Mutex};

    use pwright_browser_model::{BrowserContextKind, BrowserKind};
    use pwright_cap_context_creation::{
        BrowserContextInitializer, BrowserNewContext, ContextCreationErrorDetails,
    };
    use pwright_client::{AtomicCallIdSource, Client, RemoteInitializer, RemoteKind};
    use pwright_core::{
        CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId, Service,
    };

    use super::BrowserNewContextExt;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct RecordedCall {
        call_id: CallId,
        object_id: ObjectId,
        metadata: CallMetadata,
    }

    struct RecordingChannel {
        calls: Arc<Mutex<Vec<RecordedCall>>>,
        initializer: BrowserContextInitializer,
    }

    type TestClient = Client<RecordingChannel, AtomicCallIdSource>;
    type TestBrowser = pwright_client::Browser<RecordingChannel, AtomicCallIdSource>;
    type Fixture = (TestClient, TestBrowser, Arc<Mutex<Vec<RecordedCall>>>);

    impl Service<Command<BrowserNewContext>> for RecordingChannel {
        type Response = CommandResult<BrowserNewContext>;
        type Error = std::convert::Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<BrowserNewContext>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("recording calls should be available")
                .push(RecordedCall {
                    call_id: command.call_id(),
                    object_id: command.object_id(),
                    metadata: command.metadata(),
                });
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(self.initializer),
            )))
        }
    }

    fn fixture(initializer: BrowserContextInitializer) -> Fixture {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            RecordingChannel {
                calls: Arc::clone(&calls),
                initializer,
            },
            AtomicCallIdSource::new(10),
        );
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BrowserKind::OBJECT_KIND,
            ))
            .expect("browser should initialize");
        (client, browser, calls)
    }

    #[tokio::test]
    async fn direct_creation_returns_a_typed_context_without_action_metadata() {
        // Arrange
        let initializer = BrowserContextInitializer::new(
            ObjectId::new(2),
            ObjectId::new(1),
            BrowserContextKind::OBJECT_KIND,
        );
        let (_client, browser, calls) = fixture(initializer);

        // Act
        let context = browser
            .new_context()
            .await
            .expect("direct initializer should produce a context");

        // Assert
        assert_eq!(context.id(), ObjectId::new(2));
        assert_eq!(
            calls.lock().expect("calls should be available").as_slice(),
            &[RecordedCall {
                call_id: CallId::new(10),
                object_id: ObjectId::new(1),
                metadata: CallMetadata::default(),
            }]
        );
    }

    #[tokio::test]
    async fn remote_creation_claims_the_previously_announced_initializer() {
        // Arrange
        let initializer = BrowserContextInitializer::new(
            ObjectId::new(2),
            ObjectId::new(1),
            BrowserContextKind::OBJECT_KIND,
        );
        let (client, browser, _calls) = fixture(initializer);
        client.require_announced_initializers();
        client
            .announce(RemoteInitializer::new(
                initializer.object_id,
                Some(initializer.parent_id),
                initializer.object_kind,
            ))
            .expect("wire Create should announce the initializer");

        // Act
        let context = browser
            .new_context()
            .await
            .expect("matching announcement should be claimed");

        // Assert
        assert_eq!(context.id(), ObjectId::new(2));
        assert_eq!(
            client.object_metrics().map(|metrics| metrics.current()),
            Ok(2)
        );
    }

    #[tokio::test]
    async fn disposed_announcement_cannot_be_revived_by_a_late_result() {
        // Arrange
        let initializer = BrowserContextInitializer::new(
            ObjectId::new(2),
            ObjectId::new(1),
            BrowserContextKind::OBJECT_KIND,
        );
        let (client, browser, _calls) = fixture(initializer);
        client.require_announced_initializers();
        client
            .announce(RemoteInitializer::new(
                initializer.object_id,
                Some(initializer.parent_id),
                initializer.object_kind,
            ))
            .expect("wire Create should announce the initializer");
        client
            .invalidate(initializer.object_id)
            .expect("wire Dispose should invalidate the announcement");

        // Act
        let error = browser
            .new_context()
            .await
            .expect_err("late Result must not revive the context");

        // Assert
        assert_eq!(error.kind(), FailureKind::TargetClosed);
        assert!(matches!(
            error.details(),
            Some(ContextCreationErrorDetails::InvalidBackendResponse(message))
                if message.contains("disposed before")
        ));
    }

    #[tokio::test]
    async fn remote_result_without_create_is_a_protocol_violation() {
        // Arrange
        let initializer = BrowserContextInitializer::new(
            ObjectId::new(2),
            ObjectId::new(1),
            BrowserContextKind::OBJECT_KIND,
        );
        let (client, browser, _calls) = fixture(initializer);
        client.require_announced_initializers();

        // Act
        let error = browser
            .new_context()
            .await
            .expect_err("remote Result without Create must fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert!(matches!(
            error.details(),
            Some(ContextCreationErrorDetails::InvalidBackendResponse(message))
                if message.contains("not announced")
        ));
    }
}
