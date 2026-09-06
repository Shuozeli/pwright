//! Opt-in, value-only JavaScript evaluation for Frame and Page handles.
//!
//! Page evaluation delegates to the initialized main Frame. Importing this
//! crate adds no `JSHandle` or `ElementHandle` object type.

use std::convert::Infallible;

use pwright_cap_evaluation::{
    EvaluateParams, EvaluateResult, EvaluateValue, EvaluationErrorDetails,
};
use pwright_client::{CallIdSource, CommandFuture, Frame, Page};
use pwright_core::{CallMetadata, Command, CommandResult, FailureKind, OperationError, Service};

pub trait FrameEvaluationExt {
    fn evaluate(
        &self,
        params: EvaluateParams,
    ) -> CommandFuture<EvaluateResult, EvaluationErrorDetails>;
}

impl<Channel, Ids> FrameEvaluationExt for Frame<Channel, Ids>
where
    Channel: Service<Command<EvaluateValue>, Response = CommandResult<EvaluateValue>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
{
    fn evaluate(
        &self,
        params: EvaluateParams,
    ) -> CommandFuture<EvaluateResult, EvaluationErrorDetails> {
        self.as_remote()
            .dispatch::<EvaluateValue>(params, CallMetadata::default())
    }
}

pub trait PageEvaluationExt {
    fn evaluate(
        &self,
        params: EvaluateParams,
    ) -> CommandFuture<EvaluateResult, EvaluationErrorDetails>;
}

impl<Channel, Ids> PageEvaluationExt for Page<Channel, Ids>
where
    Channel: Service<Command<EvaluateValue>, Response = CommandResult<EvaluateValue>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
{
    fn evaluate(
        &self,
        params: EvaluateParams,
    ) -> CommandFuture<EvaluateResult, EvaluationErrorDetails> {
        let Some(frame) = self.main_frame() else {
            return Box::pin(async {
                Err(OperationError::new(
                    FailureKind::ProtocolViolation,
                    "Page.evaluate requires an initialized main Frame",
                    Some(EvaluationErrorDetails::InvalidBackendResponse(
                        "Page handle has no initialized main Frame".to_string(),
                    )),
                ))
            });
        };
        FrameEvaluationExt::evaluate(&frame, params)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};
    use std::sync::{Arc, Mutex};

    use pwright_browser_model::{BrowserContextKind, BrowserKind, FrameKind, PageKind};
    use pwright_cap_evaluation::{EvaluateParams, EvaluateResult, EvaluateValue, EvaluationValue};
    use pwright_client::{AtomicCallIdSource, Client, RemoteInitializer, RemoteKind};
    use pwright_core::{
        CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId, Service,
    };

    use super::{FrameEvaluationExt, PageEvaluationExt};

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedCall {
        call_id: CallId,
        target: ObjectId,
        params: EvaluateParams,
    }

    struct Channel {
        calls: Arc<Mutex<Vec<RecordedCall>>>,
    }

    impl Service<Command<EvaluateValue>> for Channel {
        type Response = CommandResult<EvaluateValue>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<EvaluateValue>) -> Self::Future<'_> {
            self.calls.lock().unwrap().push(RecordedCall {
                call_id: command.call_id(),
                target: command.object_id(),
                params: command.params().clone(),
            });
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(EvaluateResult::new(EvaluationValue::String(
                    "result".to_string(),
                ))),
            )))
        }
    }

    type TestClient = Client<Channel, AtomicCallIdSource>;
    type TestPage = pwright_client::Page<Channel, AtomicCallIdSource>;

    fn client() -> (TestClient, Arc<Mutex<Vec<RecordedCall>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        (
            Client::new(
                Channel {
                    calls: Arc::clone(&calls),
                },
                AtomicCallIdSource::new(10),
            ),
            calls,
        )
    }

    fn initialized_page(client: &TestClient) -> TestPage {
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
    async fn page_evaluation_dispatches_to_the_main_frame_identity() {
        // Arrange
        let (client, calls) = client();
        let page = initialized_page(&client);
        let params = EvaluateParams::expression("document.title");

        // Act
        let result = PageEvaluationExt::evaluate(&page, params.clone())
            .await
            .expect("evaluation should succeed");

        // Assert
        assert_eq!(result.value, EvaluationValue::String("result".to_string()));
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[RecordedCall {
                call_id: CallId::new(10),
                target: ObjectId::new(4),
                params,
            }]
        );
    }

    #[tokio::test]
    async fn frame_evaluation_uses_the_same_command_without_a_page_variant() {
        // Arrange
        let (client, calls) = client();
        let page = initialized_page(&client);
        let frame = page.main_frame().unwrap();

        // Act
        FrameEvaluationExt::evaluate(&frame, EvaluateParams::expression("1 + 1"))
            .await
            .expect("evaluation should succeed");

        // Assert
        assert_eq!(calls.lock().unwrap()[0].target, ObjectId::new(4));
        assert_eq!(calls.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn legacy_page_without_main_frame_fails_before_channel_work() {
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
        let error = PageEvaluationExt::evaluate(&page, EvaluateParams::expression("1"))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn evaluation_command_uses_no_timeout_metadata() {
        // Arrange
        let (client, calls) = client();
        let page = initialized_page(&client);

        // Act
        let future = PageEvaluationExt::evaluate(&page, EvaluateParams::expression("1"));
        drop(future);

        // Assert
        assert!(calls.lock().unwrap().is_empty());
        assert_eq!(CallMetadata::default(), CallMetadata::new(None, None));
    }
}
