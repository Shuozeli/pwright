//! Opt-in browser-context lifecycle methods for `pwright-client` handles.
//!
//! Importing this crate adds only `BrowserContext::close`-shaped behavior. It
//! does not install navigation, locators, codecs, a runtime, or a browser.

use std::convert::Infallible;

use pwright_cap_context_lifecycle::{
    BrowserContextClose, BrowserContextCloseResult, ContextLifecycleErrorDetails,
};
use pwright_client::{BrowserContext, CallIdSource, CommandFuture};
use pwright_core::{CallMetadata, Command, CommandResult, FailureKind, Service};

pub trait BrowserContextCloseExt {
    fn close(&self) -> CommandFuture<BrowserContextCloseResult, ContextLifecycleErrorDetails>;
}

impl<Channel, Ids> BrowserContextCloseExt for BrowserContext<Channel, Ids>
where
    Channel: Service<
            Command<BrowserContextClose>,
            Response = CommandResult<BrowserContextClose>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
{
    fn close(&self) -> CommandFuture<BrowserContextCloseResult, ContextLifecycleErrorDetails> {
        let close = self
            .as_remote()
            .dispatch::<BrowserContextClose>((), CallMetadata::default());
        Box::pin(async move {
            match close.await {
                Ok(result) => Ok(result),
                Err(error) if error.kind() == FailureKind::TargetClosed => Ok(()),
                Err(error) => Err(error),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Future, Ready, ready};
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    use pwright_cap_context_lifecycle::{BrowserContextClose, ContextLifecycleErrorDetails};
    use pwright_client::{
        AtomicCallIdSource, BrowserContext, BrowserContextKind, BrowserKind, Client,
        RemoteInitializer, RemoteKind,
    };
    use pwright_core::{
        CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId, OperationError,
        Service,
    };

    use super::BrowserContextCloseExt;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct RecordedClose {
        call_id: CallId,
        object_id: ObjectId,
        metadata: CallMetadata,
    }

    struct RecordingChannel {
        calls: Arc<Mutex<Vec<RecordedClose>>>,
        failure: Option<FailureKind>,
    }

    type TestClient = Client<RecordingChannel, AtomicCallIdSource>;
    type TestContext = BrowserContext<RecordingChannel, AtomicCallIdSource>;
    type ContextFixture = (TestClient, TestContext, Arc<Mutex<Vec<RecordedClose>>>);

    impl Service<Command<BrowserContextClose>> for RecordingChannel {
        type Response = CommandResult<BrowserContextClose>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<BrowserContextClose>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("recording channel should be available")
                .push(RecordedClose {
                    call_id: command.call_id(),
                    object_id: command.object_id(),
                    metadata: command.metadata(),
                });
            let outcome = self.failure.map_or_else(
                || Ok(()),
                |kind| {
                    Err(OperationError::<ContextLifecycleErrorDetails>::new(
                        kind,
                        "close failed",
                        None,
                    ))
                },
            );
            ready(Ok(CommandResult::new(command.call_id(), outcome)))
        }
    }

    fn poll_ready<F>(future: Pin<&mut F>) -> F::Output
    where
        F: Future + ?Sized,
    {
        let mut context = Context::from_waker(Waker::noop());
        let Poll::Ready(output) = future.poll(&mut context) else {
            panic!("ready test operation should complete");
        };
        output
    }

    fn context(failure: Option<FailureKind>) -> ContextFixture {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            RecordingChannel {
                calls: Arc::clone(&calls),
                failure,
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
        let context = browser
            .context(RemoteInitializer::new(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                BrowserContextKind::OBJECT_KIND,
            ))
            .expect("context should initialize");
        (client, context, calls)
    }

    #[test]
    fn close_dispatches_without_action_metadata() {
        // Arrange
        let (_client, context, calls) = context(None);

        // Act
        let mut close = context.close();
        let outcome = poll_ready(close.as_mut());

        // Assert
        assert_eq!(outcome, Ok(()));
        assert_eq!(
            calls.lock().expect("calls should be available").as_slice(),
            &[RecordedClose {
                call_id: CallId::new(10),
                object_id: ObjectId::new(2),
                metadata: CallMetadata::default(),
            }]
        );
    }

    #[test]
    fn close_is_idempotent_for_already_disposed_local_context() {
        // Arrange
        let (client, context, calls) = context(None);
        client
            .invalidate(context.id())
            .expect("invalidation should succeed");

        // Act
        let mut close = context.close();
        let outcome = poll_ready(close.as_mut());

        // Assert
        assert_eq!(outcome, Ok(()));
        assert!(calls.lock().expect("calls should be available").is_empty());
    }

    #[test]
    fn close_maps_racing_target_closed_but_preserves_other_failures() {
        // Arrange
        let (_client, closed_context, _) = context(Some(FailureKind::TargetClosed));
        let (_client, failed_context, _) = context(Some(FailureKind::Backend));

        // Act
        let mut closed = closed_context.close();
        let mut failed = failed_context.close();
        let closed_outcome = poll_ready(closed.as_mut());
        let failed_outcome = poll_ready(failed.as_mut());

        // Assert
        assert_eq!(closed_outcome, Ok(()));
        assert_eq!(
            failed_outcome
                .expect_err("backend failure must remain visible")
                .kind(),
            FailureKind::Backend
        );
    }
}
