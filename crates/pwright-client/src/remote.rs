use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Arc, Weak};

pub use pwright_browser_model::{
    ArtifactKind, BrowserContextKind, BrowserKind, ElementHandleKind, FrameKind, JsHandleKind,
    PageKind, RequestKind, ResponseKind, StreamKind,
};
use pwright_core::{
    CallMetadata, Command, CommandResult, CommandSpec, FailureKind, ObjectId, ObjectKindSpec,
    ObjectKindTag, OperationError, Service,
};

use crate::lifecycle::{ClientState, ObjectState};
use crate::{CallIdSource, ClientLimits, ClientStateError, ClientTerminalReasonError};

pub type CommandFuture<Output, Details> =
    Pin<Box<dyn Future<Output = Result<Output, OperationError<Details>>> + Send + 'static>>;

pub trait RemoteKind: Send + Sync + 'static {
    const OBJECT_KIND: ObjectKindTag;
    const NAME: &'static str;
}

impl<Kind> RemoteKind for Kind
where
    Kind: ObjectKindSpec,
{
    const OBJECT_KIND: ObjectKindTag = Kind::DESCRIPTOR.tag;
    const NAME: &'static str = Kind::DESCRIPTOR.name;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemoteInitializer {
    id: ObjectId,
    parent: Option<ObjectId>,
    kind: ObjectKindTag,
}

impl RemoteInitializer {
    pub const fn new(id: ObjectId, parent: Option<ObjectId>, kind: ObjectKindTag) -> Self {
        Self { id, parent, kind }
    }

    pub const fn id(self) -> ObjectId {
        self.id
    }

    pub const fn parent(self) -> Option<ObjectId> {
        self.parent
    }

    pub const fn kind(self) -> ObjectKindTag {
        self.kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandleCreationError {
    KindMismatch {
        object_id: ObjectId,
        expected: ObjectKindTag,
        actual: ObjectKindTag,
    },
    RootHasParent {
        object_id: ObjectId,
        actual_parent: ObjectId,
    },
    ParentMismatch {
        object_id: ObjectId,
        expected_parent: ObjectId,
        actual_parent: Option<ObjectId>,
    },
    DuplicateObject {
        object_id: ObjectId,
    },
    MissingAnnouncement {
        object_id: ObjectId,
    },
    AnnouncementDisposed {
        object_id: ObjectId,
    },
    AnnouncedInitializerMismatch {
        object_id: ObjectId,
        expected_parent: Option<ObjectId>,
        actual_parent: Option<ObjectId>,
        expected_kind: ObjectKindTag,
        actual_kind: ObjectKindTag,
    },
    CapacityExceeded {
        limit: usize,
    },
    ParentDisposed {
        object_id: ObjectId,
    },
    StateUnavailable,
    ConnectionClosed,
}

impl Display for HandleCreationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::KindMismatch {
                object_id,
                expected,
                actual,
            } => write!(
                formatter,
                "object `{object_id}` has kind `{actual}`, expected `{expected}`"
            ),
            Self::RootHasParent {
                object_id,
                actual_parent,
            } => write!(
                formatter,
                "root object `{object_id}` unexpectedly has parent `{actual_parent}`"
            ),
            Self::ParentMismatch {
                object_id,
                expected_parent,
                actual_parent,
            } => write!(
                formatter,
                "object `{object_id}` has parent `{actual_parent:?}`, expected `{expected_parent}`"
            ),
            Self::DuplicateObject { object_id } => {
                write!(formatter, "client object `{object_id}` already exists")
            }
            Self::MissingAnnouncement { object_id } => {
                write!(formatter, "client object `{object_id}` was not announced")
            }
            Self::AnnouncementDisposed { object_id } => write!(
                formatter,
                "client object `{object_id}` was disposed before its initializer was claimed"
            ),
            Self::AnnouncedInitializerMismatch {
                object_id,
                expected_parent,
                actual_parent,
                expected_kind,
                actual_kind,
            } => write!(
                formatter,
                "announced object `{object_id}` expected parent {expected_parent:?} and kind `{expected_kind}`, received parent {actual_parent:?} and kind `{actual_kind}`"
            ),
            Self::CapacityExceeded { limit } => {
                write!(formatter, "client live-object limit of {limit} was reached")
            }
            Self::ParentDisposed { object_id } => {
                write!(formatter, "parent object `{object_id}` is disposed")
            }
            Self::StateUnavailable => write!(formatter, "client object state is unavailable"),
            Self::ConnectionClosed => write!(formatter, "Pwright client connection is closed"),
        }
    }
}

impl Error for HandleCreationError {}

impl From<ClientStateError> for HandleCreationError {
    fn from(error: ClientStateError) -> Self {
        match error {
            ClientStateError::DuplicateObject { object_id } => Self::DuplicateObject { object_id },
            ClientStateError::MissingAnnouncement { object_id } => {
                Self::MissingAnnouncement { object_id }
            }
            ClientStateError::InitializerMismatch {
                object_id,
                expected_parent,
                actual_parent,
                expected_kind,
                actual_kind,
            } => Self::AnnouncedInitializerMismatch {
                object_id,
                expected_parent,
                actual_parent,
                expected_kind,
                actual_kind,
            },
            ClientStateError::AnnouncementDisposed { object_id } => {
                Self::AnnouncementDisposed { object_id }
            }
            ClientStateError::CapacityExceeded { limit } => Self::CapacityExceeded { limit },
            ClientStateError::StateUnavailable => Self::StateUnavailable,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RemoteAccessError {
    ConnectionClosed,
    ObjectDisposed { reason: Option<Arc<str>> },
    StateUnavailable,
}

pub struct Remote<Kind, Channel, Ids> {
    id: ObjectId,
    channel: Weak<Channel>,
    call_ids: Weak<Ids>,
    client_state: Weak<ClientState>,
    object_state: Arc<ObjectState>,
    marker: PhantomData<fn() -> Kind>,
}

pub enum AnyHandle<Channel, Ids> {
    Js(JsHandle<Channel, Ids>),
    Element(ElementHandle<Channel, Ids>),
}

impl<Channel, Ids> AnyHandle<Channel, Ids> {
    pub fn id(&self) -> ObjectId {
        match self {
            Self::Js(handle) => handle.id(),
            Self::Element(handle) => handle.id(),
        }
    }

    pub const fn is_element(&self) -> bool {
        matches!(self, Self::Element(_))
    }
}

impl<Channel, Ids> Clone for AnyHandle<Channel, Ids> {
    fn clone(&self) -> Self {
        match self {
            Self::Js(handle) => Self::Js(handle.clone()),
            Self::Element(handle) => Self::Element(handle.clone()),
        }
    }
}

impl<Channel, Ids> Debug for AnyHandle<Channel, Ids> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Js(handle) => formatter.debug_tuple("Js").field(handle).finish(),
            Self::Element(handle) => formatter.debug_tuple("Element").field(handle).finish(),
        }
    }
}

impl<Channel, Ids> PartialEq for AnyHandle<Channel, Ids> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Js(left), Self::Js(right)) => left == right,
            (Self::Element(left), Self::Element(right)) => left == right,
            _ => false,
        }
    }
}

impl<Channel, Ids> Eq for AnyHandle<Channel, Ids> {}

impl<Kind, Channel, Ids> Remote<Kind, Channel, Ids> {
    fn new(
        id: ObjectId,
        channel: &Arc<Channel>,
        call_ids: &Arc<Ids>,
        client_state: &Arc<ClientState>,
        object_state: Arc<ObjectState>,
    ) -> Self {
        Self {
            id,
            channel: Arc::downgrade(channel),
            call_ids: Arc::downgrade(call_ids),
            client_state: Arc::downgrade(client_state),
            object_state,
            marker: PhantomData,
        }
    }

    pub const fn id(&self) -> ObjectId {
        self.id
    }

    pub fn is_connected(&self) -> bool {
        self.channel.strong_count() > 0
            && self.call_ids.strong_count() > 0
            && self.client_state.strong_count() > 0
    }

    pub fn is_live(&self) -> bool {
        self.object_state.is_live()
    }

    /// Installs the first non-empty bounded terminal reason for this complete
    /// root object family. Child handles share the same allocation.
    pub fn install_terminal_reason(&self, reason: &str) -> Result<bool, ClientTerminalReasonError> {
        self.object_state.install_terminal_reason(reason)
    }

    pub fn terminal_reason(&self) -> Result<Option<Arc<str>>, ClientTerminalReasonError> {
        self.object_state.terminal_reason()
    }

    pub(crate) fn owners(&self) -> Result<(Arc<Channel>, Arc<Ids>), RemoteAccessError> {
        if !self.object_state.is_live() {
            let reason = self
                .object_state
                .terminal_reason()
                .map_err(|_| RemoteAccessError::StateUnavailable)?;
            return Err(RemoteAccessError::ObjectDisposed { reason });
        }
        let Some(channel) = self.channel.upgrade() else {
            return Err(RemoteAccessError::ConnectionClosed);
        };
        let Some(call_ids) = self.call_ids.upgrade() else {
            return Err(RemoteAccessError::ConnectionClosed);
        };
        if self.client_state.upgrade().is_none() {
            return Err(RemoteAccessError::ConnectionClosed);
        }
        Ok((channel, call_ids))
    }

    pub fn child<ChildKind>(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<Remote<ChildKind, Channel, Ids>, HandleCreationError>
    where
        ChildKind: RemoteKind,
    {
        validate_kind::<ChildKind>(initializer)?;
        if initializer.parent != Some(self.id) {
            return Err(HandleCreationError::ParentMismatch {
                object_id: initializer.id,
                expected_parent: self.id,
                actual_parent: initializer.parent,
            });
        }
        if !self.object_state.is_live() {
            return Err(HandleCreationError::ParentDisposed { object_id: self.id });
        }
        let client_state = self
            .client_state
            .upgrade()
            .ok_or(HandleCreationError::ConnectionClosed)?;
        let channel = self
            .channel
            .upgrade()
            .ok_or(HandleCreationError::ConnectionClosed)?;
        let call_ids = self
            .call_ids
            .upgrade()
            .ok_or(HandleCreationError::ConnectionClosed)?;
        let object_state =
            client_state.claim(initializer.id, initializer.parent, initializer.kind)?;
        Ok(Remote::new(
            initializer.id,
            &channel,
            &call_ids,
            &client_state,
            object_state,
        ))
    }

    pub fn announced_child<ChildKind>(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<Remote<ChildKind, Channel, Ids>, HandleCreationError>
    where
        ChildKind: RemoteKind,
    {
        validate_kind::<ChildKind>(initializer)?;
        if initializer.parent != Some(self.id) {
            return Err(HandleCreationError::ParentMismatch {
                object_id: initializer.id,
                expected_parent: self.id,
                actual_parent: initializer.parent,
            });
        }
        if !self.object_state.is_live() {
            return Err(HandleCreationError::ParentDisposed { object_id: self.id });
        }
        let client_state = self
            .client_state
            .upgrade()
            .ok_or(HandleCreationError::ConnectionClosed)?;
        let channel = self
            .channel
            .upgrade()
            .ok_or(HandleCreationError::ConnectionClosed)?;
        let call_ids = self
            .call_ids
            .upgrade()
            .ok_or(HandleCreationError::ConnectionClosed)?;
        let object_state =
            client_state.claim_announced(initializer.id, initializer.parent, initializer.kind)?;
        Ok(Remote::new(
            initializer.id,
            &channel,
            &call_ids,
            &client_state,
            object_state,
        ))
    }

    /// Claims a command-result child using the active connection contract.
    ///
    /// Direct in-process clients may materialize the initializer immediately;
    /// remote lifecycle-controlled clients require an exact prior `Create`.
    pub fn result_child<ChildKind>(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<Remote<ChildKind, Channel, Ids>, HandleCreationError>
    where
        ChildKind: RemoteKind,
    {
        if self
            .client_state
            .upgrade()
            .ok_or(HandleCreationError::ConnectionClosed)?
            .announcements_required()
        {
            self.announced_child(initializer)
        } else {
            self.child(initializer)
        }
    }

    /// Atomically claims a heterogeneous set of JSHandle/ElementHandle
    /// initializers related to this object's existing parent.
    ///
    /// This is used by commands such as JSHandle.getProperties whose outputs
    /// are siblings of the target Handle rather than its children.
    pub fn related_handle_results(
        &self,
        initializers: &[RemoteInitializer],
    ) -> Result<Vec<AnyHandle<Channel, Ids>>, HandleCreationError> {
        if !self.object_state.is_live() {
            return Err(HandleCreationError::ParentDisposed { object_id: self.id });
        }
        let client_state = self
            .client_state
            .upgrade()
            .ok_or(HandleCreationError::ConnectionClosed)?;
        let expected_parent = client_state
            .object_parent(self.id)
            .map_err(HandleCreationError::from)?
            .ok_or(HandleCreationError::StateUnavailable)?;
        for initializer in initializers {
            if initializer.parent != Some(expected_parent) {
                return Err(HandleCreationError::ParentMismatch {
                    object_id: initializer.id,
                    expected_parent,
                    actual_parent: initializer.parent,
                });
            }
            if initializer.kind != JsHandleKind::OBJECT_KIND
                && initializer.kind != ElementHandleKind::OBJECT_KIND
            {
                return Err(HandleCreationError::KindMismatch {
                    object_id: initializer.id,
                    expected: JsHandleKind::OBJECT_KIND,
                    actual: initializer.kind,
                });
            }
        }
        let channel = self
            .channel
            .upgrade()
            .ok_or(HandleCreationError::ConnectionClosed)?;
        let call_ids = self
            .call_ids
            .upgrade()
            .ok_or(HandleCreationError::ConnectionClosed)?;
        let states = client_state
            .claim_result_batch(initializers)
            .map_err(HandleCreationError::from)?;
        Ok(initializers
            .iter()
            .zip(states)
            .map(|(initializer, object_state)| {
                if initializer.kind == ElementHandleKind::OBJECT_KIND {
                    AnyHandle::Element(ElementHandle {
                        remote: Remote::<ElementHandleKind, _, _>::new(
                            initializer.id,
                            &channel,
                            &call_ids,
                            &client_state,
                            object_state,
                        ),
                    })
                } else {
                    AnyHandle::Js(JsHandle {
                        remote: Remote::<JsHandleKind, _, _>::new(
                            initializer.id,
                            &channel,
                            &call_ids,
                            &client_state,
                            object_state,
                        ),
                    })
                }
            })
            .collect())
    }

    pub(crate) fn has_same_identity<OtherKind>(
        &self,
        other: &Remote<OtherKind, Channel, Ids>,
    ) -> bool {
        self.id == other.id && self.channel.ptr_eq(&other.channel)
    }
}

impl<Kind, Channel, Ids> Remote<Kind, Channel, Ids>
where
    Ids: CallIdSource,
{
    pub fn dispatch<Spec>(
        &self,
        params: Spec::Params,
        metadata: CallMetadata,
    ) -> CommandFuture<Spec::Result, Spec::ErrorDetails>
    where
        Spec: CommandSpec,
        Spec::Params: Send + 'static,
        Channel: Service<Command<Spec>, Response = CommandResult<Spec>, Error = std::convert::Infallible>
            + 'static,
    {
        let owners = self.owners();
        let object_id = self.id;
        Box::pin(async move {
            let (channel, call_ids) = match owners {
                Ok(owners) => owners,
                Err(RemoteAccessError::ConnectionClosed) => {
                    return Err(OperationError::new(
                        FailureKind::TransportClosed,
                        "Pwright client connection is no longer available",
                        None,
                    ));
                }
                Err(RemoteAccessError::ObjectDisposed { reason }) => {
                    return Err(OperationError::new(
                        FailureKind::TargetClosed,
                        reason.map_or_else(
                            || "Pwright target is disposed".to_string(),
                            |reason| format!("Pwright target is disposed: {reason}"),
                        ),
                        None,
                    ));
                }
                Err(RemoteAccessError::StateUnavailable) => {
                    return Err(OperationError::new(
                        FailureKind::Cleanup,
                        "Pwright client terminal reason state is unavailable",
                        None,
                    ));
                }
            };
            let call_id = call_ids.next_call_id().map_err(|error| {
                OperationError::new(FailureKind::ResourceExhausted, error.to_string(), None)
            })?;
            let command = Command::<Spec>::new(call_id, object_id, metadata, params);
            let result = match channel.call(command).await {
                Ok(result) => result,
                Err(never) => match never {},
            };
            if result.call_id() != call_id {
                return Err(OperationError::new(
                    FailureKind::ProtocolViolation,
                    format!(
                        "response call ID `{}` does not match request `{call_id}`",
                        result.call_id()
                    ),
                    None,
                ));
            }
            result.into_outcome()
        })
    }
}

fn validate_kind<Kind>(initializer: RemoteInitializer) -> Result<(), HandleCreationError>
where
    Kind: RemoteKind,
{
    if initializer.kind != Kind::OBJECT_KIND {
        return Err(HandleCreationError::KindMismatch {
            object_id: initializer.id,
            expected: Kind::OBJECT_KIND,
            actual: initializer.kind,
        });
    }
    Ok(())
}

impl<Kind, Channel, Ids> Clone for Remote<Kind, Channel, Ids> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            channel: self.channel.clone(),
            call_ids: self.call_ids.clone(),
            client_state: self.client_state.clone(),
            object_state: Arc::clone(&self.object_state),
            marker: PhantomData,
        }
    }
}

impl<Kind, Channel, Ids> Debug for Remote<Kind, Channel, Ids> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Remote")
            .field("id", &self.id)
            .field("connected", &self.is_connected())
            .field("live", &self.is_live())
            .finish_non_exhaustive()
    }
}

impl<Kind, Channel, Ids> PartialEq for Remote<Kind, Channel, Ids> {
    fn eq(&self, other: &Self) -> bool {
        self.has_same_identity(other)
    }
}

impl<Kind, Channel, Ids> Eq for Remote<Kind, Channel, Ids> {}

macro_rules! remote_handle {
    ($name:ident, $kind:ident) => {
        pub struct $name<Channel, Ids> {
            pub(crate) remote: Remote<$kind, Channel, Ids>,
        }

        impl<Channel, Ids> $name<Channel, Ids> {
            pub const fn id(&self) -> ObjectId {
                self.remote.id()
            }

            pub fn is_connected(&self) -> bool {
                self.remote.is_connected()
            }

            pub fn is_live(&self) -> bool {
                self.remote.is_live()
            }

            pub const fn as_remote(&self) -> &Remote<$kind, Channel, Ids> {
                &self.remote
            }
        }

        impl<Channel, Ids> Clone for $name<Channel, Ids> {
            fn clone(&self) -> Self {
                Self {
                    remote: self.remote.clone(),
                }
            }
        }

        impl<Channel, Ids> Debug for $name<Channel, Ids> {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                Debug::fmt(&self.remote, formatter)
            }
        }

        impl<Channel, Ids> PartialEq for $name<Channel, Ids> {
            fn eq(&self, other: &Self) -> bool {
                self.remote == other.remote
            }
        }

        impl<Channel, Ids> Eq for $name<Channel, Ids> {}
    };
}

remote_handle!(Browser, BrowserKind);
remote_handle!(BrowserContext, BrowserContextKind);
remote_handle!(Frame, FrameKind);
remote_handle!(JsHandle, JsHandleKind);
remote_handle!(ElementHandle, ElementHandleKind);
remote_handle!(Request, RequestKind);
remote_handle!(Response, ResponseKind);
remote_handle!(Artifact, ArtifactKind);
remote_handle!(Stream, StreamKind);

pub struct Page<Channel, Ids> {
    pub(crate) remote: Remote<PageKind, Channel, Ids>,
    main_frame: Option<Frame<Channel, Ids>>,
}

impl<Channel, Ids> Page<Channel, Ids> {
    pub const fn id(&self) -> ObjectId {
        self.remote.id()
    }

    pub fn is_connected(&self) -> bool {
        self.remote.is_connected()
    }

    pub fn is_live(&self) -> bool {
        self.remote.is_live()
    }

    pub const fn as_remote(&self) -> &Remote<PageKind, Channel, Ids> {
        &self.remote
    }

    /// Returns the initialized main frame when the Page came from
    /// `BrowserContext.newPage`. Legacy/manual Page factories may not have one.
    pub fn main_frame(&self) -> Option<Frame<Channel, Ids>> {
        self.main_frame.clone()
    }
}

impl<Channel, Ids> Clone for Page<Channel, Ids> {
    fn clone(&self) -> Self {
        Self {
            remote: self.remote.clone(),
            main_frame: self.main_frame.clone(),
        }
    }
}

impl<Channel, Ids> Debug for Page<Channel, Ids> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.remote, formatter)
    }
}

impl<Channel, Ids> PartialEq for Page<Channel, Ids> {
    fn eq(&self, other: &Self) -> bool {
        self.remote == other.remote
    }
}

impl<Channel, Ids> Eq for Page<Channel, Ids> {}

impl<Channel, Ids> Browser<Channel, Ids> {
    pub fn context(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<BrowserContext<Channel, Ids>, HandleCreationError> {
        Ok(BrowserContext {
            remote: self.remote.child(initializer)?,
        })
    }

    pub fn context_from_announcement(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<BrowserContext<Channel, Ids>, HandleCreationError> {
        Ok(BrowserContext {
            remote: self.remote.announced_child(initializer)?,
        })
    }

    pub fn context_from_result(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<BrowserContext<Channel, Ids>, HandleCreationError> {
        Ok(BrowserContext {
            remote: self.remote.result_child(initializer)?,
        })
    }

    pub fn artifact_from_result(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<Artifact<Channel, Ids>, HandleCreationError> {
        Ok(Artifact {
            remote: self.remote.result_child(initializer)?,
        })
    }
}

impl<Channel, Ids> BrowserContext<Channel, Ids> {
    pub fn page(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<Page<Channel, Ids>, HandleCreationError> {
        Ok(Page {
            remote: self.remote.child(initializer)?,
            main_frame: None,
        })
    }

    pub fn page_from_result(
        &self,
        page: RemoteInitializer,
        main_frame: RemoteInitializer,
    ) -> Result<Page<Channel, Ids>, HandleCreationError> {
        validate_kind::<PageKind>(page)?;
        validate_kind::<FrameKind>(main_frame)?;
        if page.parent != Some(self.id()) {
            return Err(HandleCreationError::ParentMismatch {
                object_id: page.id,
                expected_parent: self.id(),
                actual_parent: page.parent,
            });
        }
        if main_frame.parent != Some(page.id) {
            return Err(HandleCreationError::ParentMismatch {
                object_id: main_frame.id,
                expected_parent: page.id,
                actual_parent: main_frame.parent,
            });
        }
        if !self.is_live() {
            return Err(HandleCreationError::ParentDisposed {
                object_id: self.id(),
            });
        }
        let client_state = self
            .remote
            .client_state
            .upgrade()
            .ok_or(HandleCreationError::ConnectionClosed)?;
        let channel = self
            .remote
            .channel
            .upgrade()
            .ok_or(HandleCreationError::ConnectionClosed)?;
        let call_ids = self
            .remote
            .call_ids
            .upgrade()
            .ok_or(HandleCreationError::ConnectionClosed)?;
        let states = client_state.claim_result_batch(&[page, main_frame])?;
        let page_remote = Remote::new(
            page.id,
            &channel,
            &call_ids,
            &client_state,
            Arc::clone(&states[0]),
        );
        let frame_remote = Remote::new(
            main_frame.id,
            &channel,
            &call_ids,
            &client_state,
            Arc::clone(&states[1]),
        );
        Ok(Page {
            remote: page_remote,
            main_frame: Some(Frame {
                remote: frame_remote,
            }),
        })
    }

    pub fn artifact_from_result(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<Artifact<Channel, Ids>, HandleCreationError> {
        Ok(Artifact {
            remote: self.remote.result_child(initializer)?,
        })
    }
}

impl<Channel, Ids> Page<Channel, Ids> {
    pub fn frame(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<Frame<Channel, Ids>, HandleCreationError> {
        Ok(Frame {
            remote: self.remote.child(initializer)?,
        })
    }

    pub fn request_from_announcement(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<Request<Channel, Ids>, HandleCreationError> {
        Ok(Request {
            remote: self.remote.announced_child(initializer)?,
        })
    }

    pub fn artifact_from_result(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<Artifact<Channel, Ids>, HandleCreationError> {
        Ok(Artifact {
            remote: self.remote.result_child(initializer)?,
        })
    }
}

impl<Channel, Ids> Artifact<Channel, Ids> {
    pub fn stream_from_result(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<Stream<Channel, Ids>, HandleCreationError> {
        Ok(Stream {
            remote: self.remote.result_child(initializer)?,
        })
    }
}

impl<Channel, Ids> Frame<Channel, Ids> {
    pub fn child_frame(&self, initializer: RemoteInitializer) -> Result<Self, HandleCreationError> {
        Ok(Self {
            remote: self.remote.child(initializer)?,
        })
    }

    pub fn js_handle(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<JsHandle<Channel, Ids>, HandleCreationError> {
        Ok(JsHandle {
            remote: self.remote.child(initializer)?,
        })
    }

    pub fn element_handle(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<ElementHandle<Channel, Ids>, HandleCreationError> {
        Ok(ElementHandle {
            remote: self.remote.child(initializer)?,
        })
    }

    /// Claims a JSHandle returned by a command under the active local/remote
    /// lifecycle contract.
    pub fn js_handle_from_result(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<JsHandle<Channel, Ids>, HandleCreationError> {
        Ok(JsHandle {
            remote: self.remote.result_child(initializer)?,
        })
    }

    /// Claims an ElementHandle returned by a command under the active
    /// local/remote lifecycle contract.
    pub fn element_handle_from_result(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<ElementHandle<Channel, Ids>, HandleCreationError> {
        Ok(ElementHandle {
            remote: self.remote.result_child(initializer)?,
        })
    }
}

impl<Channel, Ids> Request<Channel, Ids> {
    pub fn response_from_announcement(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<Response<Channel, Ids>, HandleCreationError> {
        Ok(Response {
            remote: self.remote.announced_child(initializer)?,
        })
    }
}

pub struct Client<Channel, Ids> {
    pub(crate) channel: Arc<Channel>,
    pub(crate) call_ids: Arc<Ids>,
    pub(crate) state: Arc<ClientState>,
}

impl<Channel, Ids> Client<Channel, Ids>
where
    Ids: CallIdSource,
{
    pub fn new(channel: Channel, call_ids: Ids) -> Self {
        Self::with_limits(channel, call_ids, ClientLimits::default())
    }

    pub fn channel(&self) -> &Channel {
        &self.channel
    }

    pub fn with_limits(channel: Channel, call_ids: Ids, limits: ClientLimits) -> Self {
        Self {
            channel: Arc::new(channel),
            call_ids: Arc::new(call_ids),
            state: Arc::new(ClientState::new(limits)),
        }
    }

    pub fn root<Kind>(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<Remote<Kind, Channel, Ids>, HandleCreationError>
    where
        Kind: RemoteKind,
    {
        validate_kind::<Kind>(initializer)?;
        if let Some(actual_parent) = initializer.parent {
            return Err(HandleCreationError::RootHasParent {
                object_id: initializer.id,
                actual_parent,
            });
        }
        let object_state =
            self.state
                .claim(initializer.id, initializer.parent, initializer.kind)?;
        Ok(Remote::new(
            initializer.id,
            &self.channel,
            &self.call_ids,
            &self.state,
            object_state,
        ))
    }

    pub fn browser(
        &self,
        initializer: RemoteInitializer,
    ) -> Result<Browser<Channel, Ids>, HandleCreationError> {
        Ok(Browser {
            remote: self.root(initializer)?,
        })
    }

    #[cfg(test)]
    pub(crate) fn page(&self, id: ObjectId) -> Page<Channel, Ids> {
        let object_state = self
            .state
            .claim(id, None, PageKind::OBJECT_KIND)
            .expect("test page object should register");
        Page {
            remote: Remote::new(id, &self.channel, &self.call_ids, &self.state, object_state),
            main_frame: None,
        }
    }
}

impl<Channel, Ids> Clone for Client<Channel, Ids> {
    fn clone(&self) -> Self {
        Self {
            channel: Arc::clone(&self.channel),
            call_ids: Arc::clone(&self.call_ids),
            state: Arc::clone(&self.state),
        }
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{ObjectId, ObjectKindTag};

    use crate::AtomicCallIdSource;

    use super::{
        ArtifactKind, BrowserContextKind, BrowserKind, Client, ElementHandleKind, FrameKind,
        HandleCreationError, JsHandleKind, PageKind, RemoteInitializer, RemoteKind, RequestKind,
        ResponseKind, StreamKind,
    };

    fn initializer(id: u64, parent: Option<u64>, kind: ObjectKindTag) -> RemoteInitializer {
        RemoteInitializer::new(ObjectId::new(id), parent.map(ObjectId::new), kind)
    }

    #[test]
    fn typed_factories_build_the_expected_object_hierarchy() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));

        // Act
        let browser = client
            .browser(initializer(1, None, BrowserKind::OBJECT_KIND))
            .expect("browser root should initialize");
        let context = browser
            .context(initializer(2, Some(1), BrowserContextKind::OBJECT_KIND))
            .expect("context should initialize below browser");
        let page = context
            .page(initializer(3, Some(2), PageKind::OBJECT_KIND))
            .expect("page should initialize below context");
        let main_frame = page
            .frame(initializer(4, Some(3), FrameKind::OBJECT_KIND))
            .expect("main frame should initialize below page");
        let child_frame = main_frame
            .child_frame(initializer(5, Some(4), FrameKind::OBJECT_KIND))
            .expect("child frame should initialize below frame");
        let js_handle = main_frame
            .js_handle(initializer(6, Some(4), JsHandleKind::OBJECT_KIND))
            .expect("JavaScript handle should initialize below frame");
        let element_handle = main_frame
            .element_handle(initializer(7, Some(4), ElementHandleKind::OBJECT_KIND))
            .expect("element handle should initialize below frame");

        // Assert
        assert_eq!(browser.id(), ObjectId::new(1));
        assert_eq!(context.id(), ObjectId::new(2));
        assert_eq!(page.id(), ObjectId::new(3));
        assert_eq!(main_frame.id(), ObjectId::new(4));
        assert_eq!(child_frame.id(), ObjectId::new(5));
        assert_eq!(js_handle.id(), ObjectId::new(6));
        assert_eq!(element_handle.id(), ObjectId::new(7));
    }

    #[test]
    fn announced_network_objects_form_page_request_response_lifecycle() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(initializer(1, None, BrowserKind::OBJECT_KIND))
            .expect("browser root should initialize");
        let context = browser
            .context(initializer(2, Some(1), BrowserContextKind::OBJECT_KIND))
            .expect("context should initialize");
        let page = context
            .page(initializer(3, Some(2), PageKind::OBJECT_KIND))
            .expect("page should initialize");
        client
            .announce(initializer(10, Some(3), RequestKind::OBJECT_KIND))
            .expect("Request Create should be announced");
        client
            .announce(initializer(11, Some(10), ResponseKind::OBJECT_KIND))
            .expect("Response Create should be announced");

        // Act
        let request = page
            .request_from_announcement(initializer(10, Some(3), RequestKind::OBJECT_KIND))
            .expect("Page should claim its Request");
        let response = request
            .response_from_announcement(initializer(11, Some(10), ResponseKind::OBJECT_KIND))
            .expect("Request should claim its Response");
        let invalidated =
            client.invalidate_all(&[ObjectId::new(11), ObjectId::new(10), ObjectId::new(3)]);

        // Assert
        assert_eq!(request.id(), ObjectId::new(10));
        assert_eq!(response.id(), ObjectId::new(11));
        assert_eq!(invalidated, Ok(3));
        assert!(!request.is_live());
        assert!(!response.is_live());
        assert!(!page.is_live());
    }

    #[test]
    fn artifact_factories_accept_all_three_parent_kinds_and_stream_children() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(initializer(1, None, BrowserKind::OBJECT_KIND))
            .unwrap();
        let context = browser
            .context(initializer(2, Some(1), BrowserContextKind::OBJECT_KIND))
            .unwrap();
        let page = context
            .page(initializer(3, Some(2), PageKind::OBJECT_KIND))
            .unwrap();

        // Act
        let browser_artifact = browser
            .artifact_from_result(initializer(10, Some(1), ArtifactKind::OBJECT_KIND))
            .unwrap();
        let context_artifact = context
            .artifact_from_result(initializer(11, Some(2), ArtifactKind::OBJECT_KIND))
            .unwrap();
        let page_artifact = page
            .artifact_from_result(initializer(12, Some(3), ArtifactKind::OBJECT_KIND))
            .unwrap();
        let stream = page_artifact
            .stream_from_result(initializer(13, Some(12), StreamKind::OBJECT_KIND))
            .unwrap();

        // Assert
        assert_eq!(browser_artifact.id(), ObjectId::new(10));
        assert_eq!(context_artifact.id(), ObjectId::new(11));
        assert_eq!(page_artifact.id(), ObjectId::new(12));
        assert_eq!(stream.id(), ObjectId::new(13));
        assert_eq!(client.object_metrics().unwrap().current(), 7);
    }

    #[test]
    fn typed_factory_rejects_the_wrong_object_kind() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));

        // Act
        let error = client
            .browser(initializer(1, None, PageKind::OBJECT_KIND))
            .expect_err("page kind must not initialize as a browser");

        // Assert
        assert_eq!(
            error,
            HandleCreationError::KindMismatch {
                object_id: ObjectId::new(1),
                expected: BrowserKind::OBJECT_KIND,
                actual: PageKind::OBJECT_KIND,
            }
        );
    }

    #[test]
    fn typed_factory_rejects_the_wrong_parent() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(initializer(1, None, BrowserKind::OBJECT_KIND))
            .expect("browser should initialize");

        // Act
        let error = browser
            .context(initializer(2, Some(9), BrowserContextKind::OBJECT_KIND))
            .expect_err("context parent must match its browser");

        // Assert
        assert_eq!(
            error,
            HandleCreationError::ParentMismatch {
                object_id: ObjectId::new(2),
                expected_parent: ObjectId::new(1),
                actual_parent: Some(ObjectId::new(9)),
            }
        );
    }

    #[test]
    fn root_factory_rejects_an_initializer_with_a_parent() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));

        // Act
        let error = client
            .browser(initializer(1, Some(9), BrowserKind::OBJECT_KIND))
            .expect_err("browser root must not have a parent");

        // Assert
        assert_eq!(
            error,
            HandleCreationError::RootHasParent {
                object_id: ObjectId::new(1),
                actual_parent: ObjectId::new(9),
            }
        );
    }

    #[test]
    fn child_creation_rejects_a_disconnected_parent_handle() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(initializer(1, None, BrowserKind::OBJECT_KIND))
            .expect("browser should initialize");
        drop(client);

        // Act
        let error = browser
            .context(initializer(2, Some(1), BrowserContextKind::OBJECT_KIND))
            .expect_err("disconnected parent must not create a child");

        // Assert
        assert_eq!(error, HandleCreationError::ConnectionClosed);
    }

    #[test]
    fn child_creation_rejects_a_disposed_parent_handle() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(initializer(1, None, BrowserKind::OBJECT_KIND))
            .expect("browser should initialize");
        client
            .invalidate(ObjectId::new(1))
            .expect("browser disposal should apply");

        // Act
        let error = browser
            .context(initializer(2, Some(1), BrowserContextKind::OBJECT_KIND))
            .expect_err("disposed parent must not create a child");

        // Assert
        assert_eq!(
            error,
            HandleCreationError::ParentDisposed {
                object_id: ObjectId::new(1)
            }
        );
    }

    #[test]
    fn page_equality_includes_connection_identity() {
        // Arrange
        let first_client = Client::new((), AtomicCallIdSource::new(1));
        let second_client = Client::new((), AtomicCallIdSource::new(1));
        let page = first_client.page(ObjectId::new(7));

        // Act
        let clone = page.clone();
        let other_connection = second_client.page(ObjectId::new(7));

        // Assert
        assert_eq!(page, clone);
        assert_ne!(page, other_connection);
    }

    #[test]
    fn remote_page_does_not_keep_client_connection_alive() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let page = client.page(ObjectId::new(7));
        assert!(page.is_connected());

        // Act
        drop(client);

        // Assert
        assert!(!page.is_connected());
    }
}
