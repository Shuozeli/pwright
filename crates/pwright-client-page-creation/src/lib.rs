//! Opt-in initialized Page creation methods for `pwright-client` handles.

use std::convert::Infallible;

use pwright_cap_page_creation::{BrowserContextNewPage, PageCreationErrorDetails};
use pwright_client::{
    BrowserContext, CallIdSource, CommandFuture, HandleCreationError, Page, RemoteInitializer,
};
use pwright_core::{CallMetadata, Command, CommandResult, FailureKind, OperationError, Service};

pub trait BrowserContextNewPageExt {
    type Page;

    fn new_page(&self) -> CommandFuture<Self::Page, PageCreationErrorDetails>;
}

impl<Channel, Ids> BrowserContextNewPageExt for BrowserContext<Channel, Ids>
where
    Channel: Service<
            Command<BrowserContextNewPage>,
            Response = CommandResult<BrowserContextNewPage>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
{
    type Page = Page<Channel, Ids>;

    fn new_page(&self) -> CommandFuture<Self::Page, PageCreationErrorDetails> {
        let context = self.clone();
        let creation = self
            .as_remote()
            .dispatch::<BrowserContextNewPage>((), CallMetadata::default());
        Box::pin(async move {
            let result = creation.await?;
            let page = RemoteInitializer::new(
                result.object_id,
                Some(result.parent_id),
                result.object_kind,
            );
            let main_frame = RemoteInitializer::new(
                result.main_frame_id,
                Some(result.main_frame_parent_id),
                result.main_frame_kind,
            );
            context
                .page_from_result(page, main_frame)
                .map_err(map_handle_creation_error)
        })
    }
}

fn map_handle_creation_error(
    error: HandleCreationError,
) -> OperationError<PageCreationErrorDetails> {
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
        format!("BrowserContext.newPage returned an invalid object initializer: {error}"),
        Some(PageCreationErrorDetails::InvalidBackendResponse(
            error.to_string(),
        )),
    )
}

#[cfg(test)]
mod tests {
    use std::future::{Ready, ready};

    use pwright_browser_model::{BrowserContextKind, BrowserKind, FrameKind, PageKind};
    use pwright_cap_page_creation::{BrowserContextNewPage, PageInitializer};
    use pwright_client::{AtomicCallIdSource, Client, ClientLimits, RemoteInitializer, RemoteKind};
    use pwright_core::{Command, CommandResult, FailureKind, ObjectId, Service};

    use super::BrowserContextNewPageExt;

    #[derive(Clone, Copy)]
    struct Channel {
        initializer: PageInitializer,
    }

    impl Service<Command<BrowserContextNewPage>> for Channel {
        type Response = CommandResult<BrowserContextNewPage>;
        type Error = std::convert::Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<BrowserContextNewPage>) -> Self::Future<'_> {
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(self.initializer),
            )))
        }
    }

    type TestClient = Client<Channel, AtomicCallIdSource>;
    type TestContext = pwright_client::BrowserContext<Channel, AtomicCallIdSource>;

    fn initializer() -> PageInitializer {
        PageInitializer::new(
            ObjectId::new(3),
            ObjectId::new(2),
            PageKind::OBJECT_KIND,
            ObjectId::new(4),
            ObjectId::new(3),
            FrameKind::OBJECT_KIND,
        )
    }

    fn fixture(limits: ClientLimits) -> (TestClient, TestContext) {
        let client = Client::with_limits(
            Channel {
                initializer: initializer(),
            },
            AtomicCallIdSource::new(10),
            limits,
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
        (client, context)
    }

    #[tokio::test]
    async fn direct_result_atomically_materializes_page_and_main_frame() {
        // Arrange
        let (client, context) = fixture(ClientLimits::new(4));

        // Act
        let page = context.new_page().await.expect("page should initialize");

        // Assert
        assert_eq!(page.id(), ObjectId::new(3));
        assert_eq!(
            page.main_frame().map(|frame| frame.id()),
            Some(ObjectId::new(4))
        );
        assert_eq!(
            client.object_metrics().map(|metrics| metrics.current()),
            Ok(4)
        );
    }

    #[tokio::test]
    async fn remote_result_requires_both_create_announcements() {
        // Arrange
        let (client, context) = fixture(ClientLimits::new(4));
        client.require_announced_initializers();
        client
            .announce(RemoteInitializer::new(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                PageKind::OBJECT_KIND,
            ))
            .expect("page announcement should fit");

        // Act
        let error = context
            .new_page()
            .await
            .expect_err("missing frame announcement must fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(
            client.object_metrics().map(|metrics| metrics.current()),
            Ok(3)
        );
    }

    #[tokio::test]
    async fn direct_result_reserves_capacity_for_both_objects_before_claiming_either() {
        // Arrange
        let (client, context) = fixture(ClientLimits::new(3));

        // Act
        let error = context
            .new_page()
            .await
            .expect_err("one remaining slot cannot hold a Page plus Frame");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert_eq!(
            client.object_metrics().map(|metrics| metrics.current()),
            Ok(2)
        );
    }
}
