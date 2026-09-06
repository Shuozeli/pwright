//! Opt-in Page lifecycle methods for `pwright-client` handles.
//!
//! Ordinary close is idempotent and termination-oriented. Before-unload is an
//! initiation-only operation and deliberately preserves target-closed errors.

use std::convert::Infallible;

use pwright_cap_page_lifecycle::{
    PageClose, PageCloseResult, PageLifecycleErrorDetails, PageRunBeforeUnload,
    PageRunBeforeUnloadResult,
};
use pwright_client::{CallIdSource, CommandFuture, Page};
use pwright_core::{CallMetadata, Command, CommandResult, FailureKind, Service};

pub trait PageCloseExt {
    fn close(&self) -> CommandFuture<PageCloseResult, PageLifecycleErrorDetails>;
}

impl<Channel, Ids> PageCloseExt for Page<Channel, Ids>
where
    Channel: Service<Command<PageClose>, Response = CommandResult<PageClose>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
{
    fn close(&self) -> CommandFuture<PageCloseResult, PageLifecycleErrorDetails> {
        let close = self
            .as_remote()
            .dispatch::<PageClose>((), CallMetadata::default());
        Box::pin(async move {
            match close.await {
                Ok(result) => Ok(result),
                Err(error) if error.kind() == FailureKind::TargetClosed => Ok(()),
                Err(error) => Err(error),
            }
        })
    }
}

pub trait PageRunBeforeUnloadExt {
    fn run_before_unload(
        &self,
    ) -> CommandFuture<PageRunBeforeUnloadResult, PageLifecycleErrorDetails>;
}

impl<Channel, Ids> PageRunBeforeUnloadExt for Page<Channel, Ids>
where
    Channel: Service<
            Command<PageRunBeforeUnload>,
            Response = CommandResult<PageRunBeforeUnload>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
{
    fn run_before_unload(
        &self,
    ) -> CommandFuture<PageRunBeforeUnloadResult, PageLifecycleErrorDetails> {
        self.as_remote()
            .dispatch::<PageRunBeforeUnload>((), CallMetadata::default())
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};
    use std::sync::{Arc, Mutex};

    use pwright_cap_page_lifecycle::{PageClose, PageLifecycleErrorDetails, PageRunBeforeUnload};
    use pwright_client::{
        AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, PageKind, RemoteInitializer,
        RemoteKind,
    };
    use pwright_core::{
        CallId, Command, CommandResult, FailureKind, ObjectId, OperationError, Service,
    };

    use super::{PageCloseExt, PageRunBeforeUnloadExt};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum RecordedMethod {
        Close,
        RunBeforeUnload,
    }

    struct RecordingChannel {
        calls: Arc<Mutex<Vec<(RecordedMethod, CallId, ObjectId)>>>,
        failure: Option<FailureKind>,
    }

    impl Service<Command<PageClose>> for RecordingChannel {
        type Response = CommandResult<PageClose>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<PageClose>) -> Self::Future<'_> {
            self.calls.lock().unwrap().push((
                RecordedMethod::Close,
                command.call_id(),
                command.object_id(),
            ));
            ready(Ok(result(command.call_id(), self.failure)))
        }
    }

    impl Service<Command<PageRunBeforeUnload>> for RecordingChannel {
        type Response = CommandResult<PageRunBeforeUnload>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<PageRunBeforeUnload>) -> Self::Future<'_> {
            self.calls.lock().unwrap().push((
                RecordedMethod::RunBeforeUnload,
                command.call_id(),
                command.object_id(),
            ));
            ready(Ok(result(command.call_id(), self.failure)))
        }
    }

    fn result<Spec>(call_id: CallId, failure: Option<FailureKind>) -> CommandResult<Spec>
    where
        Spec: pwright_core::CommandSpec<
                Params = (),
                Result = (),
                ErrorDetails = PageLifecycleErrorDetails,
            >,
    {
        CommandResult::new(
            call_id,
            failure.map_or_else(
                || Ok(()),
                |kind| Err(OperationError::new(kind, "page lifecycle failed", None)),
            ),
        )
    }

    type TestPage = pwright_client::Page<RecordingChannel, AtomicCallIdSource>;

    fn page(
        failure: Option<FailureKind>,
    ) -> (Client<RecordingChannel, AtomicCallIdSource>, TestPage) {
        let client = Client::new(
            RecordingChannel {
                calls: Arc::new(Mutex::new(Vec::new())),
                failure,
            },
            AtomicCallIdSource::new(20),
        );
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
        let page = context
            .page(RemoteInitializer::new(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                PageKind::OBJECT_KIND,
            ))
            .unwrap();
        (client, page)
    }

    #[tokio::test]
    async fn close_and_before_unload_dispatch_distinct_no_timeout_commands() {
        // Arrange
        let (client, page) = page(None);

        // Act
        page.close().await.unwrap();
        page.run_before_unload().await.unwrap();

        // Assert
        assert_eq!(
            client.channel().calls.lock().unwrap().as_slice(),
            &[
                (RecordedMethod::Close, CallId::new(20), ObjectId::new(3)),
                (
                    RecordedMethod::RunBeforeUnload,
                    CallId::new(21),
                    ObjectId::new(3)
                ),
            ]
        );
    }

    #[tokio::test]
    async fn ordinary_close_is_idempotent_for_a_stale_page() {
        // Arrange
        let (client, page) = page(None);
        client.invalidate(page.id()).unwrap();

        // Act
        let outcome = page.close().await;

        // Assert
        assert_eq!(outcome, Ok(()));
        assert!(client.channel().calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn before_unload_preserves_target_closed_while_close_suppresses_it() {
        // Arrange
        let (_client, page) = page(Some(FailureKind::TargetClosed));

        // Act
        let close = page.close().await;
        let before_unload = page.run_before_unload().await;

        // Assert
        assert_eq!(close, Ok(()));
        assert_eq!(before_unload.unwrap_err().kind(), FailureKind::TargetClosed);
    }
}
