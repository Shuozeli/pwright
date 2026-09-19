//! Opt-in read-only DOM-content extensions for Frame and Page handles.

use std::convert::Infallible;

use pwright_cap_dom_content::{
    DomContentData, DomContentErrorDetails, DomContentParams, FrameContent, FrameTitle, TitleParams,
};
use pwright_client::{CallIdSource, CommandFuture, Frame, Page};
use pwright_core::{CallMetadata, Command, CommandResult, FailureKind, OperationError, Service};

pub trait FrameContentExt {
    fn content(&self) -> CommandFuture<DomContentData, DomContentErrorDetails>;
}

impl<Channel, Ids> FrameContentExt for Frame<Channel, Ids>
where
    Channel: Service<Command<FrameContent>, Response = CommandResult<FrameContent>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
{
    fn content(&self) -> CommandFuture<DomContentData, DomContentErrorDetails> {
        self.as_remote()
            .dispatch::<FrameContent>(DomContentParams, CallMetadata::default())
    }
}

pub trait PageContentExt {
    fn content(&self) -> CommandFuture<DomContentData, DomContentErrorDetails>;
}

impl<Channel, Ids> PageContentExt for Page<Channel, Ids>
where
    Channel: Service<Command<FrameContent>, Response = CommandResult<FrameContent>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
{
    fn content(&self) -> CommandFuture<DomContentData, DomContentErrorDetails> {
        let Some(frame) = self.main_frame() else {
            return Box::pin(async {
                Err(OperationError::new(
                    FailureKind::ProtocolViolation,
                    "Page.content requires an initialized main Frame",
                    Some(DomContentErrorDetails::InvalidBackendResponse(
                        "Page handle has no initialized main Frame".to_owned(),
                    )),
                ))
            });
        };
        FrameContentExt::content(&frame)
    }
}

/// Opt-in title read; independent of the existing content extension bounds.
pub trait FrameTitleExt {
    fn title(&self) -> CommandFuture<DomContentData, DomContentErrorDetails>;
}

impl<Channel, Ids> FrameTitleExt for Frame<Channel, Ids>
where
    Channel: Service<Command<FrameTitle>, Response = CommandResult<FrameTitle>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
{
    fn title(&self) -> CommandFuture<DomContentData, DomContentErrorDetails> {
        self.as_remote()
            .dispatch::<FrameTitle>(TitleParams, CallMetadata::default())
    }
}

pub trait PageTitleExt {
    fn title(&self) -> CommandFuture<DomContentData, DomContentErrorDetails>;
}

impl<Channel, Ids> PageTitleExt for Page<Channel, Ids>
where
    Channel: Service<Command<FrameTitle>, Response = CommandResult<FrameTitle>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
{
    fn title(&self) -> CommandFuture<DomContentData, DomContentErrorDetails> {
        let Some(frame) = self.main_frame() else {
            return Box::pin(async {
                Err(OperationError::new(
                    FailureKind::ProtocolViolation,
                    "Page.title requires an initialized main Frame",
                    Some(DomContentErrorDetails::InvalidBackendResponse(
                        "Page handle has no initialized main Frame".to_owned(),
                    )),
                ))
            });
        };
        FrameTitleExt::title(&frame)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};
    use std::sync::{Arc, Mutex};

    use pwright_browser_model::{BrowserContextKind, BrowserKind, FrameKind, PageKind};
    use pwright_cap_dom_content::{DomContentData, FrameContent, FrameTitle};
    use pwright_client::{AtomicCallIdSource, Client, RemoteInitializer, RemoteKind};
    use pwright_core::{
        BudgetedUtf8, CallId, CallMetadata, Command, CommandResult, ObjectId, ResourceClass,
        ResourceLedger, ResourceLimits, ResourceScope, Service,
    };

    use super::{FrameContentExt, PageContentExt, PageTitleExt};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct RecordedCall {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
    }

    struct RecordingChannel {
        calls: Arc<Mutex<Vec<RecordedCall>>>,
    }

    macro_rules! recording_read {
        ($command:ty, $text:expr) => {
            impl Service<Command<$command>> for RecordingChannel {
                type Response = CommandResult<$command>;
                type Error = Infallible;
                type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

                fn call(&self, command: Command<$command>) -> Self::Future<'_> {
                    self.calls.lock().unwrap().push(RecordedCall {
                        call_id: command.call_id(),
                        target: command.object_id(),
                        metadata: command.metadata(),
                    });
                    let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
                        ResourceClass::DomContentBytes,
                        64,
                        64,
                    ));
                    let text = BudgetedUtf8::copy_from_str(
                        &ledger,
                        ResourceScope::Call(command.call_id()),
                        ResourceClass::DomContentBytes,
                        $text,
                    )
                    .unwrap();
                    ready(Ok(CommandResult::new(
                        command.call_id(),
                        Ok(DomContentData::new(text)),
                    )))
                }
            }
        };
    }

    recording_read!(FrameContent, "<html></html>");
    recording_read!(FrameTitle, "The title");

    type TestClient = Client<RecordingChannel, AtomicCallIdSource>;

    fn client() -> (TestClient, Arc<Mutex<Vec<RecordedCall>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        (
            Client::new(
                RecordingChannel {
                    calls: Arc::clone(&calls),
                },
                AtomicCallIdSource::new(20),
            ),
            calls,
        )
    }

    fn initialized_page(
        client: &TestClient,
    ) -> pwright_client::Page<RecordingChannel, AtomicCallIdSource> {
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
        context
            .page_from_result(
                RemoteInitializer::new(
                    ObjectId::new(3),
                    Some(ObjectId::new(2)),
                    PageKind::OBJECT_KIND,
                ),
                RemoteInitializer::new(
                    ObjectId::new(4),
                    Some(ObjectId::new(3)),
                    FrameKind::OBJECT_KIND,
                ),
            )
            .unwrap()
    }

    #[tokio::test]
    async fn page_content_dispatches_only_to_the_main_frame() {
        // Arrange
        let (client, calls) = client();
        let page = initialized_page(&client);

        // Act
        let content = PageContentExt::content(&page).await.unwrap();

        // Assert
        assert_eq!(content.as_str(), "<html></html>");
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[RecordedCall {
                call_id: CallId::new(20),
                target: ObjectId::new(4),
                metadata: CallMetadata::default(),
            }]
        );
    }

    #[tokio::test]
    async fn direct_frame_content_preserves_that_frame_identity() {
        // Arrange
        let (client, calls) = client();
        let page = initialized_page(&client);
        let frame = page.main_frame().unwrap();

        // Act
        FrameContentExt::content(&frame).await.unwrap();

        // Assert
        assert_eq!(calls.lock().unwrap()[0].target, ObjectId::new(4));
    }

    #[tokio::test]
    async fn page_title_dispatches_to_main_frame_without_action_timeout() {
        // Arrange
        let (client, calls) = client();
        let page = initialized_page(&client);
        // Act
        let title = PageTitleExt::title(&page).await.unwrap();
        // Assert
        assert_eq!(title.as_str(), "The title");
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[RecordedCall {
                call_id: CallId::new(20),
                target: ObjectId::new(4),
                metadata: CallMetadata::default(),
            }]
        );
    }

    #[tokio::test]
    async fn page_without_main_frame_fails_before_channel_work() {
        // Arrange
        let (client, calls) = client();
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

        // Act
        let error = PageContentExt::content(&page).await.unwrap_err();

        // Assert
        assert_eq!(error.kind(), pwright_core::FailureKind::ProtocolViolation);
        assert!(calls.lock().unwrap().is_empty());
    }
}
