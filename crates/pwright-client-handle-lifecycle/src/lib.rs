//! Opt-in explicit lifecycle methods for `pwright-client` handle objects.
//!
//! Importing this crate adds only `dispose` behavior. Dropping a Rust handle
//! remains local and never performs hidden I/O.

use std::convert::Infallible;

use pwright_cap_handle_lifecycle::{
    HandleDispose, HandleDisposeResult, HandleLifecycleErrorDetails,
};
use pwright_client::{CallIdSource, CommandFuture, ElementHandle, JsHandle, Remote, RemoteKind};
use pwright_core::{CallMetadata, Command, CommandResult, FailureKind, Service};

pub trait HandleDisposeExt {
    fn dispose(&self) -> CommandFuture<HandleDisposeResult, HandleLifecycleErrorDetails>;
}

fn dispose<Kind, Channel, Ids>(
    remote: &Remote<Kind, Channel, Ids>,
) -> CommandFuture<HandleDisposeResult, HandleLifecycleErrorDetails>
where
    Kind: RemoteKind,
    Channel: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
{
    let dispose = remote.dispatch::<HandleDispose>((), CallMetadata::default());
    Box::pin(async move {
        match dispose.await {
            Ok(result) => Ok(result),
            Err(error) if error.kind() == FailureKind::TargetClosed => Ok(()),
            Err(error) => Err(error),
        }
    })
}

impl<Channel, Ids> HandleDisposeExt for JsHandle<Channel, Ids>
where
    Channel: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
{
    fn dispose(&self) -> CommandFuture<HandleDisposeResult, HandleLifecycleErrorDetails> {
        dispose(self.as_remote())
    }
}

impl<Channel, Ids> HandleDisposeExt for ElementHandle<Channel, Ids>
where
    Channel: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
{
    fn dispose(&self) -> CommandFuture<HandleDisposeResult, HandleLifecycleErrorDetails> {
        dispose(self.as_remote())
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Future, Ready, ready};
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    use pwright_cap_handle_lifecycle::{HandleDispose, HandleLifecycleErrorDetails};
    use pwright_client::{
        AtomicCallIdSource, BrowserKind, Client, ElementHandle, ElementHandleKind, Frame,
        FrameKind, JsHandle, JsHandleKind, RemoteInitializer, RemoteKind,
    };
    use pwright_core::{
        CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId, OperationError,
        Service,
    };

    use super::HandleDisposeExt;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct RecordedDispose {
        call_id: CallId,
        object_id: ObjectId,
        metadata: CallMetadata,
    }

    struct RecordingChannel {
        calls: Arc<Mutex<Vec<RecordedDispose>>>,
        failure: Option<FailureKind>,
    }

    type TestClient = Client<RecordingChannel, AtomicCallIdSource>;
    type TestFrame = Frame<RecordingChannel, AtomicCallIdSource>;
    type TestJsHandle = JsHandle<RecordingChannel, AtomicCallIdSource>;
    type TestElementHandle = ElementHandle<RecordingChannel, AtomicCallIdSource>;
    type HandleFixture = (
        TestClient,
        TestFrame,
        TestJsHandle,
        TestElementHandle,
        Arc<Mutex<Vec<RecordedDispose>>>,
    );

    impl Service<Command<HandleDispose>> for RecordingChannel {
        type Response = CommandResult<HandleDispose>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<HandleDispose>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("recording channel should be available")
                .push(RecordedDispose {
                    call_id: command.call_id(),
                    object_id: command.object_id(),
                    metadata: command.metadata(),
                });
            let outcome = self.failure.map_or_else(
                || Ok(()),
                |kind| {
                    Err(OperationError::<HandleLifecycleErrorDetails>::new(
                        kind,
                        "dispose failed",
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

    fn handles(failure: Option<FailureKind>) -> HandleFixture {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            RecordingChannel {
                calls: Arc::clone(&calls),
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
            .expect("browser should initialize");
        let context = browser
            .context(RemoteInitializer::new(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                pwright_client::BrowserContextKind::OBJECT_KIND,
            ))
            .expect("context should initialize");
        let page = context
            .page(RemoteInitializer::new(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                pwright_client::PageKind::OBJECT_KIND,
            ))
            .expect("page should initialize");
        let frame = page
            .frame(RemoteInitializer::new(
                ObjectId::new(4),
                Some(ObjectId::new(3)),
                FrameKind::OBJECT_KIND,
            ))
            .expect("frame should initialize");
        let js_handle = frame
            .js_handle(RemoteInitializer::new(
                ObjectId::new(5),
                Some(ObjectId::new(4)),
                JsHandleKind::OBJECT_KIND,
            ))
            .expect("JavaScript handle should initialize");
        let element_handle = frame
            .element_handle(RemoteInitializer::new(
                ObjectId::new(6),
                Some(ObjectId::new(4)),
                ElementHandleKind::OBJECT_KIND,
            ))
            .expect("element handle should initialize");
        (client, frame, js_handle, element_handle, calls)
    }

    #[test]
    fn both_handle_types_dispatch_dispose_without_action_metadata() {
        // Arrange
        let (_client, _frame, js_handle, element_handle, calls) = handles(None);

        // Act
        let js_outcome = poll_ready(js_handle.dispose().as_mut());
        let element_outcome = poll_ready(element_handle.dispose().as_mut());

        // Assert
        assert_eq!(js_outcome, Ok(()));
        assert_eq!(element_outcome, Ok(()));
        assert_eq!(
            calls.lock().expect("calls should be available").as_slice(),
            &[
                RecordedDispose {
                    call_id: CallId::new(20),
                    object_id: ObjectId::new(5),
                    metadata: CallMetadata::default(),
                },
                RecordedDispose {
                    call_id: CallId::new(21),
                    object_id: ObjectId::new(6),
                    metadata: CallMetadata::default(),
                },
            ]
        );
    }

    #[test]
    fn dispose_is_idempotent_after_child_first_family_invalidation() {
        // Arrange
        let (client, frame, js_handle, element_handle, calls) = handles(None);
        client
            .invalidate_all(&[js_handle.id(), element_handle.id(), frame.id()])
            .expect("runtime child-first invalidation should apply");

        // Act
        let js_outcome = poll_ready(js_handle.dispose().as_mut());
        let element_outcome = poll_ready(element_handle.dispose().as_mut());

        // Assert
        assert_eq!(js_outcome, Ok(()));
        assert_eq!(element_outcome, Ok(()));
        assert!(calls.lock().expect("calls should be available").is_empty());
    }

    #[test]
    fn dispose_maps_racing_target_closed_but_preserves_backend_failure() {
        // Arrange
        let (_client, _frame, closed_handle, _, _) = handles(Some(FailureKind::TargetClosed));
        let (_client, _frame, failed_handle, _, _) = handles(Some(FailureKind::Backend));

        // Act
        let closed = poll_ready(closed_handle.dispose().as_mut());
        let failed = poll_ready(failed_handle.dispose().as_mut());

        // Assert
        assert_eq!(closed, Ok(()));
        assert_eq!(
            failed
                .expect_err("backend failure must remain visible")
                .kind(),
            FailureKind::Backend
        );
    }
}
