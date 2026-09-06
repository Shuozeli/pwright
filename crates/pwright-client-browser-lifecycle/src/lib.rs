//! Opt-in owned Browser lifecycle methods for `pwright-client` handles.
//!
//! Importing this crate adds the public `Browser::close`-shaped operation. A
//! server advertises it only for a session holding owned-process authority.

use std::convert::Infallible;

use pwright_cap_browser_lifecycle::{
    BrowserClose, BrowserCloseOptions, BrowserCloseResult, BrowserLifecycleErrorDetails,
};
use pwright_client::{Browser, CallIdSource, ClientTerminalReasonError, CommandFuture};
use pwright_core::{CallMetadata, Command, CommandResult, FailureKind, Service};

pub trait BrowserCloseExt {
    fn close(&self) -> CommandFuture<BrowserCloseResult, BrowserLifecycleErrorDetails>;

    fn close_with_options(
        &self,
        options: BrowserCloseOptions,
    ) -> CommandFuture<BrowserCloseResult, BrowserLifecycleErrorDetails>;
}

impl<Channel, Ids> BrowserCloseExt for Browser<Channel, Ids>
where
    Channel: Service<Command<BrowserClose>, Response = CommandResult<BrowserClose>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
{
    fn close(&self) -> CommandFuture<BrowserCloseResult, BrowserLifecycleErrorDetails> {
        self.close_with_options(BrowserCloseOptions::default())
    }

    fn close_with_options(
        &self,
        options: BrowserCloseOptions,
    ) -> CommandFuture<BrowserCloseResult, BrowserLifecycleErrorDetails> {
        let browser = self.as_remote().clone();
        Box::pin(async move {
            if let Some(reason) = options.reason() {
                browser.install_terminal_reason(reason).map_err(|error| {
                    let kind = match error {
                        ClientTerminalReasonError::TooLong { .. } => FailureKind::ResourceExhausted,
                        ClientTerminalReasonError::StateUnavailable => FailureKind::Cleanup,
                    };
                    pwright_core::OperationError::new(kind, error.to_string(), None)
                })?;
            }
            let close = browser.dispatch::<BrowserClose>(options, CallMetadata::default());
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
    use std::future::{Pending, Ready, pending, ready};
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Waker};

    use pwright_cap_browser_lifecycle::{
        BrowserClose, BrowserCloseOptions, BrowserLifecycleErrorDetails,
    };
    use pwright_client::{
        AtomicCallIdSource, BrowserKind, Client, ClientLimits, RemoteInitializer, RemoteKind,
    };
    use pwright_core::{Command, CommandResult, FailureKind, ObjectId, OperationError, Service};

    use super::BrowserCloseExt;

    type RecordedCalls = Arc<Mutex<Vec<(ObjectId, Option<String>)>>>;

    struct RecordingChannel {
        calls: RecordedCalls,
        failure: Option<FailureKind>,
    }

    struct PendingChannel;

    impl Service<Command<BrowserClose>> for PendingChannel {
        type Response = CommandResult<BrowserClose>;
        type Error = Infallible;
        type Future<'a> = Pending<Result<Self::Response, Self::Error>>;

        fn call(&self, _command: Command<BrowserClose>) -> Self::Future<'_> {
            pending()
        }
    }

    impl Service<Command<BrowserClose>> for RecordingChannel {
        type Response = CommandResult<BrowserClose>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<BrowserClose>) -> Self::Future<'_> {
            self.calls.lock().unwrap().push((
                command.object_id(),
                command.params().reason().map(str::to_string),
            ));
            let outcome = self.failure.map_or_else(
                || Ok(()),
                |kind| {
                    Err(OperationError::<BrowserLifecycleErrorDetails>::new(
                        kind,
                        "close failed",
                        None,
                    ))
                },
            );
            ready(Ok(CommandResult::new(command.call_id(), outcome)))
        }
    }

    type TestClient = Client<RecordingChannel, AtomicCallIdSource>;
    type TestBrowser = pwright_client::Browser<RecordingChannel, AtomicCallIdSource>;

    fn browser(failure: Option<FailureKind>) -> (TestClient, TestBrowser, RecordedCalls) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            RecordingChannel {
                calls: Arc::clone(&calls),
                failure,
            },
            AtomicCallIdSource::new(40),
        );
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BrowserKind::OBJECT_KIND,
            ))
            .unwrap();
        (client, browser, calls)
    }

    #[tokio::test]
    async fn close_dispatches_to_the_browser_object() {
        // Arrange
        let (_client, browser, calls) = browser(None);

        // Act
        let result = browser.close().await;

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(*calls.lock().unwrap(), [(ObjectId::new(1), None)]);
    }

    #[tokio::test]
    async fn close_options_send_the_playwright_reason() {
        // Arrange
        let (_client, browser, calls) = browser(None);

        // Act
        let result = browser
            .close_with_options(BrowserCloseOptions::with_reason("planned restart"))
            .await;

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(
            *calls.lock().unwrap(),
            [(ObjectId::new(1), Some("planned restart".to_string()))]
        );
    }

    #[test]
    fn close_reason_is_lazy_then_visible_before_the_remote_result() {
        // Arrange
        let client = Client::new(PendingChannel, AtomicCallIdSource::new(1));
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BrowserKind::OBJECT_KIND,
            ))
            .unwrap();
        let mut close =
            browser.close_with_options(BrowserCloseOptions::with_reason("planned restart"));
        assert_eq!(browser.as_remote().terminal_reason().unwrap(), None);

        // Act
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        let poll = close.as_mut().poll(&mut context);

        // Assert
        assert!(poll.is_pending());
        assert_eq!(
            browser.as_remote().terminal_reason().unwrap().as_deref(),
            Some("planned restart")
        );
    }

    #[tokio::test]
    async fn oversized_close_reason_fails_before_channel_dispatch() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::with_limits(
            RecordingChannel {
                calls: Arc::clone(&calls),
                failure: None,
            },
            AtomicCallIdSource::new(1),
            ClientLimits::new(1).with_terminal_reason_bytes(4),
        );
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BrowserKind::OBJECT_KIND,
            ))
            .unwrap();

        // Act
        let error = browser
            .close_with_options(BrowserCloseOptions::with_reason("12345"))
            .await
            .expect_err("oversized reason should fail locally");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(calls.lock().unwrap().is_empty());
        assert_eq!(browser.as_remote().terminal_reason().unwrap(), None);
    }

    #[tokio::test]
    async fn operation_started_after_disposal_reports_the_shared_close_reason() {
        // Arrange
        let (client, browser, calls) = browser(None);
        browser
            .as_remote()
            .install_terminal_reason("planned restart")
            .unwrap();
        client.invalidate(browser.id()).unwrap();

        // Act
        let error = browser
            .as_remote()
            .dispatch::<BrowserClose>(BrowserCloseOptions::default(), Default::default())
            .await
            .expect_err("disposed Browser should reject locally");

        // Assert
        assert_eq!(error.kind(), FailureKind::TargetClosed);
        assert_eq!(
            error.message(),
            "Pwright target is disposed: planned restart"
        );
        assert!(calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn close_is_idempotent_after_remote_disposal() {
        // Arrange
        let (_client, browser, _) = browser(Some(FailureKind::TargetClosed));

        // Act
        let result = browser.close().await;

        // Assert
        assert_eq!(result, Ok(()));
    }
}
