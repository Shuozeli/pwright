//! Opt-in `evaluateHandle` methods for Frame/Page plus Handle-local operations.
//!
//! The returned portable initializer must already have been announced by a
//! remote server. Direct in-process compositions may claim it from the result.

use std::convert::Infallible;
use std::fmt::{self, Debug, Formatter};

use pwright_cap_handle_evaluation::{
    EvaluateHandle, EvaluateHandleParams, GetPropertiesOptions, GetPropertyParams, HandleEvaluate,
    HandleEvaluateHandle, HandleEvaluateParams, HandleEvaluationErrorDetails,
    HandleEvaluationValue, HandleGetProperties, HandleGetProperty, HandleInitializer,
    HandleJsonValue, JsonValueOptions,
};
use pwright_client::{
    AnyHandle, CallIdSource, CommandFuture, ElementHandle, ElementHandleKind, Frame,
    HandleCreationError, JsHandle, JsHandleKind, Page, Remote, RemoteInitializer, RemoteKind,
};
use pwright_core::{CallMetadata, Command, CommandResult, FailureKind, OperationError, Service};

pub enum EvaluatedHandle<Channel, Ids> {
    Js(JsHandle<Channel, Ids>),
    Element(ElementHandle<Channel, Ids>),
}

impl<Channel, Ids> Clone for EvaluatedHandle<Channel, Ids> {
    fn clone(&self) -> Self {
        match self {
            Self::Js(handle) => Self::Js(handle.clone()),
            Self::Element(handle) => Self::Element(handle.clone()),
        }
    }
}

impl<Channel, Ids> Debug for EvaluatedHandle<Channel, Ids> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Js(handle) => formatter.debug_tuple("Js").field(handle).finish(),
            Self::Element(handle) => formatter.debug_tuple("Element").field(handle).finish(),
        }
    }
}

impl<Channel, Ids> PartialEq for EvaluatedHandle<Channel, Ids> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Js(left), Self::Js(right)) => left == right,
            (Self::Element(left), Self::Element(right)) => left == right,
            _ => false,
        }
    }
}

impl<Channel, Ids> Eq for EvaluatedHandle<Channel, Ids> {}

impl<Channel, Ids> EvaluatedHandle<Channel, Ids> {
    pub fn id(&self) -> pwright_core::ObjectId {
        match self {
            Self::Js(handle) => handle.id(),
            Self::Element(handle) => handle.id(),
        }
    }

    pub const fn is_element(&self) -> bool {
        matches!(self, Self::Element(_))
    }

    /// Produces a portable argument leaf without exposing a backend-native ID.
    pub fn as_evaluation_argument(&self) -> HandleEvaluationValue {
        HandleEvaluationValue::Handle(self.id())
    }
}

impl<Channel, Ids> From<&EvaluatedHandle<Channel, Ids>> for HandleEvaluationValue {
    fn from(handle: &EvaluatedHandle<Channel, Ids>) -> Self {
        handle.as_evaluation_argument()
    }
}

impl<Channel, Ids> From<AnyHandle<Channel, Ids>> for EvaluatedHandle<Channel, Ids> {
    fn from(handle: AnyHandle<Channel, Ids>) -> Self {
        match handle {
            AnyHandle::Js(handle) => Self::Js(handle),
            AnyHandle::Element(handle) => Self::Element(handle),
        }
    }
}

pub struct HandleProperty<Channel, Ids> {
    pub name: String,
    pub value: EvaluatedHandle<Channel, Ids>,
}

impl<Channel, Ids> Debug for HandleProperty<Channel, Ids> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HandleProperty")
            .field("name", &self.name)
            .field("value", &self.value)
            .finish()
    }
}

impl<Channel, Ids> PartialEq for HandleProperty<Channel, Ids> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value == other.value
    }
}

impl<Channel, Ids> Eq for HandleProperty<Channel, Ids> {}

pub trait HandleEvaluationExt {
    type Handle;
    type Properties;

    fn evaluate(
        &self,
        params: HandleEvaluateParams,
    ) -> CommandFuture<pwright_cap_handle_evaluation::EvaluationValue, HandleEvaluationErrorDetails>;

    fn evaluate_handle(
        &self,
        params: HandleEvaluateParams,
    ) -> CommandFuture<Self::Handle, HandleEvaluationErrorDetails>;

    fn json_value(
        &self,
    ) -> CommandFuture<pwright_cap_handle_evaluation::EvaluationValue, HandleEvaluationErrorDetails>;

    fn get_property(
        &self,
        name: impl Into<String>,
    ) -> CommandFuture<Self::Handle, HandleEvaluationErrorDetails>;

    fn get_properties(&self) -> CommandFuture<Self::Properties, HandleEvaluationErrorDetails>;
}

macro_rules! impl_handle_evaluation {
    ($handle:ident) => {
        impl<Channel, Ids> HandleEvaluationExt for $handle<Channel, Ids>
        where
            Channel: Service<
                    Command<HandleEvaluate>,
                    Response = CommandResult<HandleEvaluate>,
                    Error = Infallible,
                > + Service<
                    Command<HandleEvaluateHandle>,
                    Response = CommandResult<HandleEvaluateHandle>,
                    Error = Infallible,
                > + Service<
                    Command<HandleJsonValue>,
                    Response = CommandResult<HandleJsonValue>,
                    Error = Infallible,
                > + Service<
                    Command<HandleGetProperty>,
                    Response = CommandResult<HandleGetProperty>,
                    Error = Infallible,
                > + Service<
                    Command<HandleGetProperties>,
                    Response = CommandResult<HandleGetProperties>,
                    Error = Infallible,
                > + 'static,
            Ids: CallIdSource,
        {
            type Handle = EvaluatedHandle<Channel, Ids>;
            type Properties = Vec<HandleProperty<Channel, Ids>>;

            fn evaluate(
                &self,
                params: HandleEvaluateParams,
            ) -> CommandFuture<
                pwright_cap_handle_evaluation::EvaluationValue,
                HandleEvaluationErrorDetails,
            > {
                let operation = self
                    .as_remote()
                    .dispatch::<HandleEvaluate>(params, CallMetadata::default());
                Box::pin(async move { Ok(operation.await?.value) })
            }

            fn evaluate_handle(
                &self,
                params: HandleEvaluateParams,
            ) -> CommandFuture<Self::Handle, HandleEvaluationErrorDetails> {
                let target = self.clone();
                let operation = self
                    .as_remote()
                    .dispatch::<HandleEvaluateHandle>(params, CallMetadata::default());
                Box::pin(async move {
                    let initializer = operation.await?;
                    materialize_related_handles(
                        target.as_remote(),
                        std::slice::from_ref(&initializer),
                    )
                    .and_then(|mut handles| {
                        handles.pop().ok_or(HandleCreationError::StateUnavailable)
                    })
                    .map_err(map_handle_creation_error)
                })
            }

            fn json_value(
                &self,
            ) -> CommandFuture<
                pwright_cap_handle_evaluation::EvaluationValue,
                HandleEvaluationErrorDetails,
            > {
                let operation = self
                    .as_remote()
                    .dispatch::<HandleJsonValue>(JsonValueOptions, CallMetadata::default());
                Box::pin(async move { Ok(operation.await?.value) })
            }

            fn get_property(
                &self,
                name: impl Into<String>,
            ) -> CommandFuture<Self::Handle, HandleEvaluationErrorDetails> {
                let target = self.clone();
                let operation = self.as_remote().dispatch::<HandleGetProperty>(
                    GetPropertyParams::new(name),
                    CallMetadata::default(),
                );
                Box::pin(async move {
                    let initializer = operation.await?;
                    materialize_related_handles(
                        target.as_remote(),
                        std::slice::from_ref(&initializer),
                    )
                    .and_then(|mut handles| {
                        handles.pop().ok_or(HandleCreationError::StateUnavailable)
                    })
                    .map_err(map_handle_creation_error)
                })
            }

            fn get_properties(
                &self,
            ) -> CommandFuture<Self::Properties, HandleEvaluationErrorDetails> {
                let target = self.clone();
                let operation = self
                    .as_remote()
                    .dispatch::<HandleGetProperties>(GetPropertiesOptions, CallMetadata::default());
                Box::pin(async move {
                    let result = operation.await?;
                    let names = result
                        .properties
                        .iter()
                        .map(|property| property.name.clone())
                        .collect::<Vec<_>>();
                    let initializers = result
                        .properties
                        .into_iter()
                        .map(|property| property.handle)
                        .collect::<Vec<_>>();
                    let handles = materialize_related_handles(target.as_remote(), &initializers)
                        .map_err(map_handle_creation_error)?;
                    Ok(names
                        .into_iter()
                        .zip(handles)
                        .map(|(name, value)| HandleProperty { name, value })
                        .collect())
                })
            }
        }
    };
}

impl_handle_evaluation!(JsHandle);
impl_handle_evaluation!(ElementHandle);

impl<Channel, Ids> HandleEvaluationExt for EvaluatedHandle<Channel, Ids>
where
    Channel: Service<
            Command<HandleEvaluate>,
            Response = CommandResult<HandleEvaluate>,
            Error = Infallible,
        > + Service<
            Command<HandleEvaluateHandle>,
            Response = CommandResult<HandleEvaluateHandle>,
            Error = Infallible,
        > + Service<
            Command<HandleJsonValue>,
            Response = CommandResult<HandleJsonValue>,
            Error = Infallible,
        > + Service<
            Command<HandleGetProperty>,
            Response = CommandResult<HandleGetProperty>,
            Error = Infallible,
        > + Service<
            Command<HandleGetProperties>,
            Response = CommandResult<HandleGetProperties>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
{
    type Handle = Self;
    type Properties = Vec<HandleProperty<Channel, Ids>>;

    fn evaluate(
        &self,
        params: HandleEvaluateParams,
    ) -> CommandFuture<pwright_cap_handle_evaluation::EvaluationValue, HandleEvaluationErrorDetails>
    {
        match self {
            Self::Js(handle) => handle.evaluate(params),
            Self::Element(handle) => handle.evaluate(params),
        }
    }

    fn evaluate_handle(
        &self,
        params: HandleEvaluateParams,
    ) -> CommandFuture<Self::Handle, HandleEvaluationErrorDetails> {
        match self {
            Self::Js(handle) => handle.evaluate_handle(params),
            Self::Element(handle) => handle.evaluate_handle(params),
        }
    }

    fn json_value(
        &self,
    ) -> CommandFuture<pwright_cap_handle_evaluation::EvaluationValue, HandleEvaluationErrorDetails>
    {
        match self {
            Self::Js(handle) => handle.json_value(),
            Self::Element(handle) => handle.json_value(),
        }
    }

    fn get_property(
        &self,
        name: impl Into<String>,
    ) -> CommandFuture<Self::Handle, HandleEvaluationErrorDetails> {
        let name = name.into();
        match self {
            Self::Js(handle) => handle.get_property(name),
            Self::Element(handle) => handle.get_property(name),
        }
    }

    fn get_properties(&self) -> CommandFuture<Self::Properties, HandleEvaluationErrorDetails> {
        match self {
            Self::Js(handle) => handle.get_properties(),
            Self::Element(handle) => handle.get_properties(),
        }
    }
}

pub trait FrameHandleEvaluationExt {
    type Handle;

    fn evaluate_handle(
        &self,
        params: EvaluateHandleParams,
    ) -> CommandFuture<Self::Handle, HandleEvaluationErrorDetails>;
}

impl<Channel, Ids> FrameHandleEvaluationExt for Frame<Channel, Ids>
where
    Channel: Service<
            Command<EvaluateHandle>,
            Response = CommandResult<EvaluateHandle>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
{
    type Handle = EvaluatedHandle<Channel, Ids>;

    fn evaluate_handle(
        &self,
        params: EvaluateHandleParams,
    ) -> CommandFuture<Self::Handle, HandleEvaluationErrorDetails> {
        let frame = self.clone();
        let operation = self
            .as_remote()
            .dispatch::<EvaluateHandle>(params, CallMetadata::default());
        Box::pin(async move {
            let initializer = operation.await?;
            materialize_handle(&frame, initializer).map_err(map_handle_creation_error)
        })
    }
}

pub trait PageHandleEvaluationExt {
    type Handle;

    fn evaluate_handle(
        &self,
        params: EvaluateHandleParams,
    ) -> CommandFuture<Self::Handle, HandleEvaluationErrorDetails>;
}

impl<Channel, Ids> PageHandleEvaluationExt for Page<Channel, Ids>
where
    Channel: Service<
            Command<EvaluateHandle>,
            Response = CommandResult<EvaluateHandle>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
{
    type Handle = EvaluatedHandle<Channel, Ids>;

    fn evaluate_handle(
        &self,
        params: EvaluateHandleParams,
    ) -> CommandFuture<Self::Handle, HandleEvaluationErrorDetails> {
        let Some(frame) = self.main_frame() else {
            return Box::pin(async {
                Err(OperationError::new(
                    FailureKind::ProtocolViolation,
                    "Page.evaluateHandle requires an initialized main Frame",
                    Some(HandleEvaluationErrorDetails::InvalidBackendResponse(
                        "Page handle has no initialized main Frame".to_string(),
                    )),
                ))
            });
        };
        FrameHandleEvaluationExt::evaluate_handle(&frame, params)
    }
}

fn materialize_handle<Channel, Ids>(
    frame: &Frame<Channel, Ids>,
    initializer: HandleInitializer,
) -> Result<EvaluatedHandle<Channel, Ids>, HandleCreationError> {
    let remote = RemoteInitializer::new(
        initializer.object_id,
        Some(initializer.parent_id),
        initializer.object_kind,
    );
    if initializer.object_kind == ElementHandleKind::OBJECT_KIND {
        frame
            .element_handle_from_result(remote)
            .map(EvaluatedHandle::Element)
    } else if initializer.object_kind == JsHandleKind::OBJECT_KIND {
        frame.js_handle_from_result(remote).map(EvaluatedHandle::Js)
    } else {
        Err(HandleCreationError::KindMismatch {
            object_id: initializer.object_id,
            expected: JsHandleKind::OBJECT_KIND,
            actual: initializer.object_kind,
        })
    }
}

fn materialize_related_handles<Kind, Channel, Ids>(
    target: &Remote<Kind, Channel, Ids>,
    initializers: &[HandleInitializer],
) -> Result<Vec<EvaluatedHandle<Channel, Ids>>, HandleCreationError> {
    let remote_initializers = initializers
        .iter()
        .map(|initializer| {
            RemoteInitializer::new(
                initializer.object_id,
                Some(initializer.parent_id),
                initializer.object_kind,
            )
        })
        .collect::<Vec<_>>();
    target
        .related_handle_results(&remote_initializers)
        .map(|handles| handles.into_iter().map(EvaluatedHandle::from).collect())
}

fn map_handle_creation_error(
    error: HandleCreationError,
) -> OperationError<HandleEvaluationErrorDetails> {
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
        format!("evaluateHandle returned an invalid handle initializer: {error}"),
        Some(HandleEvaluationErrorDetails::InvalidBackendResponse(
            error.to_string(),
        )),
    )
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};

    use pwright_browser_model::{
        BrowserContextKind, BrowserKind, ElementHandleKind, FrameKind, JsHandleKind, PageKind,
    };
    use pwright_cap_handle_evaluation::{
        EvaluateHandle, EvaluateHandleParams, HandleEvaluate, HandleEvaluateHandle,
        HandleEvaluateParams, HandleGetProperties, HandleGetProperty, HandleInitializer,
        HandleJsonValue, HandlePropertiesResult, HandlePropertyInitializer, HandleValueResult,
    };
    use pwright_client::{AtomicCallIdSource, Client, RemoteInitializer, RemoteKind};
    use pwright_core::{Command, CommandResult, FailureKind, ObjectId, Service};

    use super::{EvaluatedHandle, FrameHandleEvaluationExt, HandleEvaluationExt};

    #[derive(Clone)]
    struct Channel {
        initializer: HandleInitializer,
    }

    impl Service<Command<EvaluateHandle>> for Channel {
        type Response = CommandResult<EvaluateHandle>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<EvaluateHandle>) -> Self::Future<'_> {
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(self.initializer.clone()),
            )))
        }
    }

    impl Service<Command<HandleEvaluate>> for Channel {
        type Response = CommandResult<HandleEvaluate>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<HandleEvaluate>) -> Self::Future<'_> {
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(HandleValueResult::new(
                    pwright_cap_handle_evaluation::EvaluationValue::String("evaluated".to_owned()),
                )),
            )))
        }
    }

    impl Service<Command<HandleEvaluateHandle>> for Channel {
        type Response = CommandResult<HandleEvaluateHandle>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<HandleEvaluateHandle>) -> Self::Future<'_> {
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(HandleInitializer::new(
                    ObjectId::new(command.call_id().get().saturating_add(100)),
                    ObjectId::new(4),
                    JsHandleKind::OBJECT_KIND,
                    "JSHandle@object",
                )),
            )))
        }
    }

    impl Service<Command<HandleJsonValue>> for Channel {
        type Response = CommandResult<HandleJsonValue>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<HandleJsonValue>) -> Self::Future<'_> {
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(HandleValueResult::new(
                    pwright_cap_handle_evaluation::EvaluationValue::NegativeZero,
                )),
            )))
        }
    }

    impl Service<Command<HandleGetProperty>> for Channel {
        type Response = CommandResult<HandleGetProperty>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<HandleGetProperty>) -> Self::Future<'_> {
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(HandleInitializer::new(
                    ObjectId::new(command.call_id().get().saturating_add(100)),
                    ObjectId::new(4),
                    JsHandleKind::OBJECT_KIND,
                    "undefined",
                )),
            )))
        }
    }

    impl Service<Command<HandleGetProperties>> for Channel {
        type Response = CommandResult<HandleGetProperties>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<HandleGetProperties>) -> Self::Future<'_> {
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(HandlePropertiesResult::new(vec![
                    HandlePropertyInitializer::new(
                        "first",
                        HandleInitializer::new(
                            ObjectId::new(command.call_id().get().saturating_add(100)),
                            ObjectId::new(4),
                            JsHandleKind::OBJECT_KIND,
                            "1",
                        ),
                    ),
                    HandlePropertyInitializer::new(
                        "second",
                        HandleInitializer::new(
                            ObjectId::new(command.call_id().get().saturating_add(101)),
                            ObjectId::new(4),
                            ElementHandleKind::OBJECT_KIND,
                            "button",
                        ),
                    ),
                ])),
            )))
        }
    }

    fn fixture() -> (
        Client<Channel, AtomicCallIdSource>,
        pwright_client::Frame<Channel, AtomicCallIdSource>,
    ) {
        let client = Client::new(
            Channel {
                initializer: HandleInitializer::new(
                    ObjectId::new(5),
                    ObjectId::new(4),
                    ElementHandleKind::OBJECT_KIND,
                    "body",
                ),
            },
            AtomicCallIdSource::new(10),
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
        let frame = page
            .frame(RemoteInitializer::new(
                ObjectId::new(4),
                Some(ObjectId::new(3)),
                FrameKind::OBJECT_KIND,
            ))
            .unwrap();
        (client, frame)
    }

    #[tokio::test]
    async fn direct_result_materializes_the_concrete_element_handle() {
        // Arrange
        let (_client, frame) = fixture();

        // Act
        let handle = frame
            .evaluate_handle(EvaluateHandleParams::expression("document.body"))
            .await
            .unwrap();

        // Assert
        assert!(matches!(handle, EvaluatedHandle::Element(_)));
        assert_eq!(handle.id(), ObjectId::new(5));
        assert_eq!(
            handle.as_evaluation_argument(),
            pwright_cap_handle_evaluation::HandleEvaluationValue::Handle(ObjectId::new(5))
        );
    }

    #[tokio::test]
    async fn remote_result_requires_matching_create_announcement() {
        // Arrange
        let (client, frame) = fixture();
        client.require_announced_initializers();

        // Act
        let error = frame
            .evaluate_handle(EvaluateHandleParams::expression("document.body"))
            .await
            .expect_err("Result without Create must fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    }

    #[tokio::test]
    async fn remote_result_claims_the_exact_prior_create() {
        // Arrange
        let (client, frame) = fixture();
        client.require_announced_initializers();
        client
            .announce(RemoteInitializer::new(
                ObjectId::new(5),
                Some(ObjectId::new(4)),
                ElementHandleKind::OBJECT_KIND,
            ))
            .unwrap();

        // Act
        let handle = frame
            .evaluate_handle(EvaluateHandleParams::expression("document.body"))
            .await
            .unwrap();

        // Assert
        assert!(handle.is_element());
        assert_eq!(handle.id(), ObjectId::new(5));
    }

    #[tokio::test]
    async fn js_handle_evaluate_returns_a_by_value_result() {
        // Arrange
        let (_client, frame) = fixture();
        let target = frame
            .js_handle(RemoteInitializer::new(
                ObjectId::new(6),
                Some(ObjectId::new(4)),
                JsHandleKind::OBJECT_KIND,
            ))
            .unwrap();

        // Act
        let value = target
            .evaluate(HandleEvaluateParams::function_without_argument(
                "target => target.value",
            ))
            .await
            .unwrap();

        // Assert
        assert_eq!(
            value,
            pwright_cap_handle_evaluation::EvaluationValue::String("evaluated".to_owned())
        );
    }

    #[tokio::test]
    async fn js_handle_json_value_preserves_special_numbers() {
        // Arrange
        let (_client, frame) = fixture();
        let target = frame
            .js_handle(RemoteInitializer::new(
                ObjectId::new(6),
                Some(ObjectId::new(4)),
                JsHandleKind::OBJECT_KIND,
            ))
            .unwrap();

        // Act
        let json = target.json_value().await.unwrap();

        // Assert
        assert_eq!(
            json,
            pwright_cap_handle_evaluation::EvaluationValue::NegativeZero
        );
    }

    #[tokio::test]
    async fn js_handle_evaluate_handle_materializes_a_related_handle() {
        // Arrange
        let (_client, frame) = fixture();
        let target = frame
            .js_handle(RemoteInitializer::new(
                ObjectId::new(6),
                Some(ObjectId::new(4)),
                JsHandleKind::OBJECT_KIND,
            ))
            .unwrap();

        // Act
        let evaluated_handle = target
            .evaluate_handle(HandleEvaluateParams::function_without_argument(
                "target => target.child",
            ))
            .await
            .unwrap();

        // Assert
        assert!(!evaluated_handle.is_element());
    }

    #[tokio::test]
    async fn js_handle_get_property_materializes_undefined_as_a_handle() {
        // Arrange
        let (_client, frame) = fixture();
        let target = frame
            .js_handle(RemoteInitializer::new(
                ObjectId::new(6),
                Some(ObjectId::new(4)),
                JsHandleKind::OBJECT_KIND,
            ))
            .unwrap();

        // Act
        let property = target.get_property("missing").await.unwrap();

        // Assert
        assert!(!property.is_element());
    }

    #[tokio::test]
    async fn js_handle_get_properties_atomically_materializes_mixed_handle_kinds() {
        // Arrange
        let (_client, frame) = fixture();
        let target = frame
            .js_handle(RemoteInitializer::new(
                ObjectId::new(6),
                Some(ObjectId::new(4)),
                JsHandleKind::OBJECT_KIND,
            ))
            .unwrap();

        // Act
        let properties = target.get_properties().await.unwrap();

        // Assert
        assert_eq!(
            properties
                .iter()
                .map(|property| property.name.as_str())
                .collect::<Vec<_>>(),
            ["first", "second"]
        );
        assert!(!properties[0].value.is_element());
        assert!(properties[1].value.is_element());
    }
}
