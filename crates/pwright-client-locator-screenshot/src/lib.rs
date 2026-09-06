//! Opt-in Locator screenshot methods for `pwright-client` values.

use std::convert::Infallible;

use pwright_cap_locator_screenshot::{
    LocatorScreenshot, LocatorScreenshotErrorDetails, LocatorScreenshotParams, ScreenshotData,
    ScreenshotOptions, Selector,
};
use pwright_client::{CallIdSource, CommandFuture, Locator};
use pwright_core::{CallMetadata, Command, CommandResult, Service};

pub trait LocatorScreenshotExt {
    fn screenshot(
        &self,
        options: ScreenshotOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<ScreenshotData, LocatorScreenshotErrorDetails>;
}

impl<Channel, Ids, Query> LocatorScreenshotExt for Locator<Channel, Ids, Query>
where
    Channel: Service<
            Command<LocatorScreenshot>,
            Response = CommandResult<LocatorScreenshot>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn screenshot(
        &self,
        options: ScreenshotOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<ScreenshotData, LocatorScreenshotErrorDetails> {
        self.dispatch::<LocatorScreenshot>(
            LocatorScreenshotParams::new(self.query().clone().into(), options),
            metadata,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use pwright_cap_locator_screenshot::{
        LocatorScreenshot, ScreenshotData, ScreenshotFormat, ScreenshotOptions, Selector,
    };
    use pwright_client::{
        AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, PageKind, RemoteInitializer,
        RemoteKind,
    };
    use pwright_core::{
        BudgetedBytes, CallId, CallMetadata, Command, CommandResult, ObjectId, ResourceClass,
        ResourceLedger, ResourceLimits, ResourceScope, Service,
    };

    use super::LocatorScreenshotExt;

    type RecordedCall = (CallId, ObjectId, Selector, ScreenshotOptions, CallMetadata);

    struct RecordingChannel {
        calls: Arc<Mutex<Vec<RecordedCall>>>,
    }

    impl Service<Command<LocatorScreenshot>> for RecordingChannel {
        type Response = CommandResult<LocatorScreenshot>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<LocatorScreenshot>) -> Self::Future<'_> {
            self.calls.lock().unwrap().push((
                command.call_id(),
                command.object_id(),
                command.params().selector.clone(),
                command.params().options,
                command.metadata(),
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
                b"shot",
            )
            .unwrap();
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(ScreenshotData::new(bytes, ScreenshotFormat::Png)),
            )))
        }
    }

    #[tokio::test]
    async fn extension_dispatches_selector_options_metadata_and_locator_scope() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            RecordingChannel {
                calls: Arc::clone(&calls),
            },
            AtomicCallIdSource::new(21),
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
        let selector = Selector::css("#hero").expect("selector should validate");
        let locator = page.locator(selector.clone());
        let options = ScreenshotOptions {
            format: Some(ScreenshotFormat::Png),
            ..ScreenshotOptions::default()
        };
        let metadata = CallMetadata::new(Some(Duration::from_secs(2)), None);

        // Act
        let screenshot = locator.screenshot(options, metadata).await.unwrap();

        // Assert
        assert_eq!(screenshot.as_bytes(), b"shot");
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[(
                CallId::new(21),
                ObjectId::new(3),
                selector,
                options,
                metadata,
            )]
        );
    }
}
