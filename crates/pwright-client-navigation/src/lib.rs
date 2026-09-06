//! Opt-in typed navigation methods for `pwright-client` page handles.
//!
//! Import [`PageNavigationExt`] for the portable backend primitive or
//! [`PageGotoExt`] for the Playwright-shaped operation. The extension traits
//! are independent, so neither navigation surface becomes a dependency of the
//! base client or of consumers that install only the other command.

use pwright_cap_navigation::{
    BeginNavigation, BeginNavigationParams, BeginNavigationResult, Goto, GotoParams, GotoResult,
    NavigationErrorDetails,
};
use pwright_client::{CallIdSource, CommandFuture, Page};
use pwright_core::{CallMetadata, Command, CommandResult, Service};
use std::convert::Infallible;

pub trait PageNavigationExt {
    fn begin_navigation(
        &self,
        params: BeginNavigationParams,
        metadata: CallMetadata,
    ) -> CommandFuture<BeginNavigationResult, NavigationErrorDetails>;
}

impl<Channel, Ids> PageNavigationExt for Page<Channel, Ids>
where
    Channel: Service<
            Command<BeginNavigation>,
            Response = CommandResult<BeginNavigation>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
{
    fn begin_navigation(
        &self,
        params: BeginNavigationParams,
        metadata: CallMetadata,
    ) -> CommandFuture<BeginNavigationResult, NavigationErrorDetails> {
        self.as_remote()
            .dispatch::<BeginNavigation>(params, metadata)
    }
}

pub trait PageGotoExt {
    fn goto(
        &self,
        params: GotoParams,
        metadata: CallMetadata,
    ) -> CommandFuture<GotoResult, NavigationErrorDetails>;
}

impl<Channel, Ids> PageGotoExt for Page<Channel, Ids>
where
    Channel: Service<Command<Goto>, Response = CommandResult<Goto>, Error = Infallible> + 'static,
    Ids: CallIdSource,
{
    fn goto(
        &self,
        params: GotoParams,
        metadata: CallMetadata,
    ) -> CommandFuture<GotoResult, NavigationErrorDetails> {
        self.as_remote().dispatch::<Goto>(params, metadata)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Future, Ready, ready};
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
    use pwright_cap_navigation::{
        BeginNavigation, BeginNavigationParams, BeginNavigationResult, Goto, GotoParams,
        GotoResult, NavigationErrorDetails,
    };
    use pwright_client::{
        AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, Page, PageKind,
        RemoteInitializer, RemoteKind,
    };
    use pwright_core::{
        CallId, CallMetadata, CancellationId, Command, CommandResult, FailureKind, ObjectId,
        ObjectKindTag, OperationError, Service,
    };
    use pwright_runtime::{DirectDispatcher, RuntimeLimits};

    use super::{PageGotoExt, PageNavigationExt};

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedCommand {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        url: String,
    }

    struct RecordingChannel {
        calls: Arc<Mutex<Vec<RecordedCommand>>>,
        fail: bool,
        response_call_id: Option<CallId>,
    }

    impl Service<Command<BeginNavigation>> for RecordingChannel {
        type Response = CommandResult<BeginNavigation>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<BeginNavigation>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("recording channel should be available")
                .push(RecordedCommand {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    url: command.params().url.clone(),
                });
            let outcome = if self.fail {
                Err(OperationError::new(
                    FailureKind::Backend,
                    "navigation failed",
                    Some(NavigationErrorDetails::Network("offline".to_string())),
                ))
            } else {
                Ok(BeginNavigationResult {
                    frame_id: "frame".to_string(),
                    loader_id: Some("loader".to_string()),
                    same_document: false,
                    is_download: false,
                })
            };
            ready(Ok(CommandResult::new(
                self.response_call_id.unwrap_or(command.call_id()),
                outcome,
            )))
        }
    }

    impl Service<Command<Goto>> for RecordingChannel {
        type Response = CommandResult<Goto>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<Goto>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("recording channel should be available")
                .push(RecordedCommand {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    url: command.params().url.clone(),
                });
            ready(Ok(CommandResult::new(
                self.response_call_id.unwrap_or(command.call_id()),
                Ok(GotoResult {
                    frame_id: "frame".to_string(),
                    loader_id: Some("loader".to_string()),
                    url: command.params().url.clone(),
                    same_document: false,
                    response: None,
                }),
            )))
        }
    }

    struct ReadyNavigationService;

    impl Service<BeginNavigationParams> for ReadyNavigationService {
        type Response = BeginNavigationResult;
        type Error = OperationError<NavigationErrorDetails>;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, _params: BeginNavigationParams) -> Self::Future<'_> {
            ready(Ok(BeginNavigationResult {
                frame_id: "frame".to_string(),
                loader_id: Some("loader".to_string()),
                same_document: false,
                is_download: false,
            }))
        }
    }

    impl Service<GotoParams> for ReadyNavigationService {
        type Response = GotoResult;
        type Error = OperationError<NavigationErrorDetails>;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, params: GotoParams) -> Self::Future<'_> {
            ready(Ok(GotoResult {
                frame_id: "frame".to_string(),
                loader_id: Some("loader".to_string()),
                url: params.url,
                same_document: false,
                response: None,
            }))
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

    fn page<Channel, Ids>(client: &Client<Channel, Ids>) -> Page<Channel, Ids>
    where
        Ids: pwright_client::CallIdSource,
    {
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
        context
            .page(RemoteInitializer::new(
                ObjectId::new(7),
                Some(ObjectId::new(2)),
                PageKind::OBJECT_KIND,
            ))
            .expect("page should initialize")
    }

    fn recording_client(
        first_call_id: u64,
        fail: bool,
        response_call_id: Option<CallId>,
    ) -> (
        Client<RecordingChannel, AtomicCallIdSource>,
        Arc<Mutex<Vec<RecordedCommand>>>,
    ) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            RecordingChannel {
                calls: Arc::clone(&calls),
                fail,
                response_call_id,
            },
            AtomicCallIdSource::new(first_call_id),
        );
        (client, calls)
    }

    #[test]
    fn extension_dispatches_typed_navigation_with_identity_and_metadata() {
        // Arrange
        let (client, calls) = recording_client(10, false, None);
        let page = page(&client);
        let metadata = CallMetadata::new(None, Some(CancellationId::new(9)));

        // Act
        let mut future =
            page.begin_navigation(BeginNavigationParams::new("https://example.com"), metadata);
        let result = poll_ready(future.as_mut());

        // Assert
        assert!(result.is_ok());
        assert_eq!(
            *calls.lock().expect("recorded calls should be available"),
            vec![RecordedCommand {
                call_id: CallId::new(10),
                target: ObjectId::new(7),
                metadata,
                url: "https://example.com".to_string(),
            }]
        );
    }

    #[test]
    fn goto_extension_dispatches_the_playwright_level_command_independently() {
        // Arrange
        let (client, calls) = recording_client(20, false, None);
        let page = page(&client);
        let metadata = CallMetadata::new(None, Some(CancellationId::new(12)));

        // Act
        let mut future = page.goto(GotoParams::new("https://example.com/app"), metadata);
        let result = poll_ready(future.as_mut()).expect("goto should succeed");

        // Assert
        assert_eq!(result.url, "https://example.com/app");
        assert_eq!(
            *calls.lock().expect("recorded calls should be available"),
            vec![RecordedCommand {
                call_id: CallId::new(20),
                target: ObjectId::new(7),
                metadata,
                url: "https://example.com/app".to_string(),
            }]
        );
    }

    #[test]
    fn extension_distinguishes_disconnect_disposal_and_id_exhaustion() {
        // Arrange
        let (disconnected_client, _) = recording_client(1, false, None);
        let disconnected_page = page(&disconnected_client);
        drop(disconnected_client);
        let (disposed_client, _) = recording_client(1, false, None);
        let disposed_page = page(&disposed_client);
        disposed_client
            .invalidate(ObjectId::new(7))
            .expect("dispose notification should apply");
        let (exhausted_client, _) = recording_client(u64::MAX, false, None);
        let exhausted_page = page(&exhausted_client);

        // Act
        let mut disconnected = disconnected_page.begin_navigation(
            BeginNavigationParams::new("https://example.com"),
            CallMetadata::default(),
        );
        let mut disposed = disposed_page.begin_navigation(
            BeginNavigationParams::new("https://example.com"),
            CallMetadata::default(),
        );
        let mut exhausted = exhausted_page.begin_navigation(
            BeginNavigationParams::new("https://example.com"),
            CallMetadata::default(),
        );

        // Assert
        assert_eq!(
            poll_ready(disconnected.as_mut())
                .expect_err("disconnected page should fail")
                .kind(),
            FailureKind::TransportClosed
        );
        assert_eq!(
            poll_ready(disposed.as_mut())
                .expect_err("disposed page should fail")
                .kind(),
            FailureKind::TargetClosed
        );
        assert_eq!(
            poll_ready(exhausted.as_mut())
                .expect_err("exhausted IDs should fail")
                .kind(),
            FailureKind::ResourceExhausted
        );
    }

    #[test]
    fn generic_dispatch_preserves_backend_failure_and_rejects_wrong_correlation() {
        // Arrange
        let (failed_client, _) = recording_client(1, true, None);
        let failed_page = page(&failed_client);
        let (mismatched_client, _) = recording_client(1, false, Some(CallId::new(99)));
        let mismatched_page = page(&mismatched_client);

        // Act
        let mut failed = failed_page.begin_navigation(
            BeginNavigationParams::new("https://example.com"),
            CallMetadata::default(),
        );
        let mut mismatched = mismatched_page.begin_navigation(
            BeginNavigationParams::new("https://example.com"),
            CallMetadata::default(),
        );

        // Assert
        assert_eq!(
            poll_ready(failed.as_mut())
                .expect_err("backend failure should pass through")
                .kind(),
            FailureKind::Backend
        );
        assert_eq!(
            poll_ready(mismatched.as_mut())
                .expect_err("wrong response ID should fail")
                .kind(),
            FailureKind::ProtocolViolation
        );
    }

    #[test]
    fn extension_dispatches_in_process_without_a_wire_codec() {
        // Arrange
        let dispatcher = DirectDispatcher::with_object_kinds(
            ReadyNavigationService,
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid");
        let page_id = ObjectId::new(7);
        dispatcher
            .register_object(page_id, None, ObjectKindTag::new(3))
            .expect("runtime page should register");
        let client = Client::new(dispatcher, AtomicCallIdSource::new(1));
        let page = page(&client);

        // Act
        let mut future = page.begin_navigation(
            BeginNavigationParams::new("data:text/html,hello"),
            CallMetadata::default(),
        );
        let result = poll_ready(future.as_mut()).expect("typed in-process call should succeed");

        // Assert
        assert_eq!(result.frame_id, "frame");
        assert_eq!(result.loader_id.as_deref(), Some("loader"));
    }
}
