//! Opt-in Page screenshot methods for `pwright-client` handles.

use std::convert::Infallible;

use pwright_cap_content::{ContentErrorDetails, PageScreenshot, ScreenshotData, ScreenshotOptions};
use pwright_client::{CallIdSource, CommandFuture, Page};
use pwright_core::{CallMetadata, Command, CommandResult, Service};

pub trait PageScreenshotExt {
    fn screenshot(
        &self,
        options: ScreenshotOptions,
    ) -> CommandFuture<ScreenshotData, ContentErrorDetails>;
}

impl<Channel, Ids> PageScreenshotExt for Page<Channel, Ids>
where
    Channel: Service<
            Command<PageScreenshot>,
            Response = CommandResult<PageScreenshot>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
{
    fn screenshot(
        &self,
        options: ScreenshotOptions,
    ) -> CommandFuture<ScreenshotData, ContentErrorDetails> {
        self.as_remote()
            .dispatch::<PageScreenshot>(options, CallMetadata::default())
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};
    use std::sync::{Arc, Mutex};

    use pwright_cap_content::{
        PageScreenshot, ScreenshotData, ScreenshotFormat, ScreenshotOptions,
    };
    use pwright_client::{
        AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, PageKind, RemoteInitializer,
        RemoteKind,
    };
    use pwright_core::{
        BudgetedBytes, CallId, Command, CommandResult, ObjectId, ResourceClass, ResourceLedger,
        ResourceLimits, ResourceScope, Service,
    };

    use super::PageScreenshotExt;

    struct RecordingChannel {
        calls: Arc<Mutex<Vec<(CallId, ObjectId, ScreenshotOptions)>>>,
    }

    impl Service<Command<PageScreenshot>> for RecordingChannel {
        type Response = CommandResult<PageScreenshot>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<PageScreenshot>) -> Self::Future<'_> {
            self.calls.lock().unwrap().push((
                command.call_id(),
                command.object_id(),
                *command.params(),
            ));
            let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
                ResourceClass::ScreenshotBytes,
                4,
                4,
            ));
            let bytes = BudgetedBytes::copy_from_slice(
                &ledger,
                ResourceScope::Call(command.call_id()),
                ResourceClass::ScreenshotBytes,
                b"mock",
            )
            .unwrap();
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(ScreenshotData::new(bytes, ScreenshotFormat::Png)),
            )))
        }
    }

    #[tokio::test]
    async fn extension_dispatches_to_the_exact_page_with_default_metadata() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            RecordingChannel {
                calls: Arc::clone(&calls),
            },
            AtomicCallIdSource::new(12),
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
        let options = ScreenshotOptions {
            format: Some(ScreenshotFormat::Png),
            full_page: Some(true),
            ..ScreenshotOptions::default()
        };

        // Act
        let result = page.screenshot(options).await.unwrap();

        // Assert
        assert_eq!(result.as_bytes(), b"mock");
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[(CallId::new(12), ObjectId::new(3), options)]
        );
    }
}
