//! Opt-in server-to-client lifecycle control handling.
//!
//! Create and adopt messages are surfaced to the composing object layer.
//! Dispose messages synchronously invalidate existing client handles.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_client::{CallIdSource, Client, ClientStateError, RemoteInitializer};
use pwright_client_wire::{FrameDisposition, FrameHandler};
use pwright_core::{ControlMessage, ObjectId};
use pwright_transport::{FrameRoute, WireCodec, WireFrame, WireProtocol, WireProtocolError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientLifecycleEvent {
    Created(RemoteInitializer),
    Adopted {
        object_id: ObjectId,
        new_parent: ObjectId,
    },
    Disposed {
        object_id: ObjectId,
        was_live: bool,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum ClientControlError<Codec> {
    Protocol(WireProtocolError<Codec>),
    ClientState(ClientStateError),
    InvalidDirection(ControlMessage),
}

impl<Codec> Display for ClientControlError<Codec>
where
    Codec: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => Display::fmt(error, formatter),
            Self::ClientState(error) => Display::fmt(error, formatter),
            Self::InvalidDirection(message) => write!(
                formatter,
                "control message {message:?} is not valid from runtime to client"
            ),
        }
    }
}

impl<Codec> Error for ClientControlError<Codec>
where
    Codec: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Protocol(error) => Some(error),
            Self::ClientState(error) => Some(error),
            Self::InvalidDirection(_) => None,
        }
    }
}

pub struct ClientLifecycleHandler<Channel, Ids, Codec> {
    client: Client<Channel, Ids>,
    protocol: WireProtocol<Codec>,
}

impl<Channel, Ids, Codec> ClientLifecycleHandler<Channel, Ids, Codec> {
    pub fn new(client: Client<Channel, Ids>, protocol: WireProtocol<Codec>) -> Self
    where
        Ids: CallIdSource,
    {
        client.require_announced_initializers();
        Self { client, protocol }
    }

    pub const fn client(&self) -> &Client<Channel, Ids> {
        &self.client
    }

    pub fn into_parts(self) -> (Client<Channel, Ids>, WireProtocol<Codec>) {
        (self.client, self.protocol)
    }
}

impl<Channel, Ids, Codec> FrameHandler for ClientLifecycleHandler<Channel, Ids, Codec>
where
    Ids: CallIdSource,
    Codec: WireCodec<ControlMessage>,
{
    type Event = ClientLifecycleEvent;
    type Error = ClientControlError<<Codec as WireCodec<ControlMessage>>::Error>;

    fn handle(&mut self, frame: WireFrame) -> Result<FrameDisposition<Self::Event>, Self::Error> {
        if frame.route() != FrameRoute::Control {
            return Ok(FrameDisposition::Unhandled(frame));
        }
        let message = self
            .protocol
            .decode::<ControlMessage>(frame)
            .map_err(ClientControlError::Protocol)?;
        let event = match message {
            ControlMessage::Create {
                object_id,
                parent,
                kind,
            } => {
                let initializer = RemoteInitializer::new(object_id, parent, kind);
                self.client
                    .announce(initializer)
                    .map_err(ClientControlError::ClientState)?;
                ClientLifecycleEvent::Created(initializer)
            }
            ControlMessage::Adopt {
                object_id,
                new_parent,
            } => ClientLifecycleEvent::Adopted {
                object_id,
                new_parent,
            },
            ControlMessage::Dispose { object_id } => {
                let was_live = self
                    .client
                    .invalidate(object_id)
                    .map_err(ClientControlError::ClientState)?;
                ClientLifecycleEvent::Disposed {
                    object_id,
                    was_live,
                }
            }
            ControlMessage::Cancel { .. }
            | ControlMessage::SetSubscription { .. }
            | ControlMessage::Close
            | ControlMessage::CloseAck => {
                return Err(ClientControlError::InvalidDirection(message));
            }
        };
        Ok(FrameDisposition::Handled(event))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use pwright_client::{
        AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, PageKind, RemoteInitializer,
        RemoteKind,
    };
    use pwright_client_wire::{FrameDisposition, FrameHandler};
    use pwright_core::{
        CallId, CancellationId, CancellationReason, ControlMessage, EventTag, ObjectId,
        ObjectKindTag, ProtocolVersion,
    };
    use pwright_transport::{
        EncodedParts, FrameLimits, FrameRoute, VersionPolicy, WireCodec, WireFrame, WireProtocol,
    };

    use super::{ClientControlError, ClientLifecycleEvent, ClientLifecycleHandler};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct CodecError(&'static str);

    impl std::fmt::Display for CodecError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl std::error::Error for CodecError {}

    struct QueueCodec {
        messages: Mutex<VecDeque<ControlMessage>>,
    }

    impl QueueCodec {
        fn new(messages: impl IntoIterator<Item = ControlMessage>) -> Self {
            Self {
                messages: Mutex::new(messages.into_iter().collect()),
            }
        }
    }

    impl WireCodec<ControlMessage> for QueueCodec {
        type Error = CodecError;

        fn encode_parts(&self, _message: &ControlMessage) -> Result<EncodedParts, Self::Error> {
            Ok(EncodedParts::new(
                FrameRoute::Control,
                Vec::new(),
                Vec::new(),
            ))
        }

        fn decode_frame(&self, _frame: WireFrame) -> Result<ControlMessage, Self::Error> {
            self.messages
                .lock()
                .map_err(|_| CodecError("codec state unavailable"))?
                .pop_front()
                .ok_or(CodecError("no queued control message"))
        }
    }

    fn limits() -> FrameLimits {
        FrameLimits::new(64, 0, 0, 64)
    }

    fn protocol(messages: impl IntoIterator<Item = ControlMessage>) -> WireProtocol<QueueCodec> {
        WireProtocol::new(
            QueueCodec::new(messages),
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("version range should build"),
            ProtocolVersion::new(1, 0),
            limits(),
        )
        .expect("control protocol should build")
    }

    fn frame(route: FrameRoute) -> WireFrame {
        WireFrame::new(
            ProtocolVersion::new(1, 0),
            route,
            Vec::new(),
            Vec::new(),
            limits(),
        )
        .expect("test frame should fit")
    }

    fn initializer(id: u64, parent: Option<u64>, kind: ObjectKindTag) -> RemoteInitializer {
        RemoteInitializer::new(ObjectId::new(id), parent.map(ObjectId::new), kind)
    }

    #[test]
    fn create_event_preserves_initializer_for_the_typed_object_layer() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let message = ControlMessage::Create {
            object_id: ObjectId::new(1),
            parent: None,
            kind: BrowserKind::OBJECT_KIND,
        };
        let mut handler = ClientLifecycleHandler::new(client.clone(), protocol([message]));

        // Act
        let disposition = handler.handle(frame(FrameRoute::Control));

        // Assert
        let Ok(FrameDisposition::Handled(ClientLifecycleEvent::Created(created))) = disposition
        else {
            panic!("create control should become a lifecycle event");
        };
        assert_eq!(created.id(), ObjectId::new(1));
        assert_eq!(created.parent(), None);
        assert_eq!(created.kind(), BrowserKind::OBJECT_KIND);
        client
            .browser(created)
            .expect("typed object layer should bind the initializer");
    }

    #[test]
    fn announced_context_is_claimed_once_with_the_exact_identity() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(initializer(1, None, BrowserKind::OBJECT_KIND))
            .expect("browser should initialize");
        let message = ControlMessage::Create {
            object_id: ObjectId::new(2),
            parent: Some(ObjectId::new(1)),
            kind: BrowserContextKind::OBJECT_KIND,
        };
        let mut handler = ClientLifecycleHandler::new(client.clone(), protocol([message]));

        // Act
        let disposition = handler.handle(frame(FrameRoute::Control));
        let Ok(FrameDisposition::Handled(ClientLifecycleEvent::Created(created))) = disposition
        else {
            panic!("context Create should announce its initializer");
        };
        let context = browser.context_from_announcement(created);
        let second = browser.context_from_announcement(created);

        // Assert
        assert_eq!(context.map(|context| context.id()), Ok(ObjectId::new(2)));
        assert_eq!(
            second.expect_err("an announcement can only be claimed once"),
            pwright_client::HandleCreationError::DuplicateObject {
                object_id: ObjectId::new(2),
            }
        );
        assert_eq!(
            client.object_metrics().map(|metrics| metrics.current()),
            Ok(2)
        );
    }

    #[test]
    fn dispose_before_result_prevents_an_announced_context_from_being_revived() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(initializer(1, None, BrowserKind::OBJECT_KIND))
            .expect("browser should initialize");
        let create = ControlMessage::Create {
            object_id: ObjectId::new(2),
            parent: Some(ObjectId::new(1)),
            kind: BrowserContextKind::OBJECT_KIND,
        };
        let dispose = ControlMessage::Dispose {
            object_id: ObjectId::new(2),
        };
        let mut handler = ClientLifecycleHandler::new(client.clone(), protocol([create, dispose]));
        let created = match handler.handle(frame(FrameRoute::Control)) {
            Ok(FrameDisposition::Handled(ClientLifecycleEvent::Created(created))) => created,
            _ => panic!("context Create should announce its initializer"),
        };

        // Act
        let disposed = handler.handle(frame(FrameRoute::Control));
        let claim = browser.context_from_announcement(created);

        // Assert
        assert!(matches!(
            disposed,
            Ok(FrameDisposition::Handled(ClientLifecycleEvent::Disposed {
                object_id,
                was_live: true,
            })) if object_id == ObjectId::new(2)
        ));
        assert_eq!(
            claim.expect_err("a disposed announcement must not be revived"),
            pwright_client::HandleCreationError::AnnouncementDisposed {
                object_id: ObjectId::new(2),
            }
        );
        assert_eq!(
            client.object_metrics().map(|metrics| metrics.current()),
            Ok(1)
        );
    }

    #[test]
    fn duplicate_create_is_rejected_without_replacing_the_announcement() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(initializer(1, None, BrowserKind::OBJECT_KIND))
            .expect("browser should initialize");
        let create = ControlMessage::Create {
            object_id: ObjectId::new(2),
            parent: Some(ObjectId::new(1)),
            kind: BrowserContextKind::OBJECT_KIND,
        };
        let mut handler = ClientLifecycleHandler::new(client, protocol([create, create]));
        let created = match handler.handle(frame(FrameRoute::Control)) {
            Ok(FrameDisposition::Handled(ClientLifecycleEvent::Created(created))) => created,
            _ => panic!("first Create should announce its initializer"),
        };

        // Act
        let duplicate = handler.handle(frame(FrameRoute::Control));

        // Assert
        assert!(matches!(
            duplicate,
            Err(ClientControlError::ClientState(
                pwright_client::ClientStateError::DuplicateObject { object_id }
            )) if object_id == ObjectId::new(2)
        ));
        assert_eq!(
            browser
                .context_from_announcement(created)
                .map(|context| context.id()),
            Ok(ObjectId::new(2))
        );
    }

    #[test]
    fn dispose_event_invalidates_all_existing_handle_clones_idempotently() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(initializer(1, None, BrowserKind::OBJECT_KIND))
            .expect("browser should initialize");
        let context = browser
            .context(initializer(2, Some(1), BrowserContextKind::OBJECT_KIND))
            .expect("context should initialize");
        let page = context
            .page(initializer(3, Some(2), PageKind::OBJECT_KIND))
            .expect("page should initialize");
        let clone = page.clone();
        let dispose = ControlMessage::Dispose {
            object_id: ObjectId::new(3),
        };
        let mut handler = ClientLifecycleHandler::new(client, protocol([dispose, dispose]));

        // Act
        let first = handler.handle(frame(FrameRoute::Control));
        let second = handler.handle(frame(FrameRoute::Control));

        // Assert
        assert!(matches!(
            first,
            Ok(FrameDisposition::Handled(ClientLifecycleEvent::Disposed {
                object_id,
                was_live: true,
            })) if object_id == ObjectId::new(3)
        ));
        assert!(matches!(
            second,
            Ok(FrameDisposition::Handled(ClientLifecycleEvent::Disposed {
                object_id,
                was_live: false,
            })) if object_id == ObjectId::new(3)
        ));
        assert!(!page.is_live());
        assert!(!clone.is_live());
        assert!(context.is_live());
    }

    #[test]
    fn adopt_event_is_forwarded_without_copying_the_runtime_parent_graph() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let adopt = ControlMessage::Adopt {
            object_id: ObjectId::new(3),
            new_parent: ObjectId::new(9),
        };
        let mut handler = ClientLifecycleHandler::new(client, protocol([adopt]));

        // Act
        let disposition = handler.handle(frame(FrameRoute::Control));

        // Assert
        assert!(matches!(
            disposition,
            Ok(FrameDisposition::Handled(ClientLifecycleEvent::Adopted {
                object_id,
                new_parent,
            })) if object_id == ObjectId::new(3) && new_parent == ObjectId::new(9)
        ));
    }

    #[test]
    fn client_handler_rejects_runtime_invalid_control_directions() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let cancel = ControlMessage::Cancel {
            call_id: CallId::new(1),
            cancellation: CancellationId::new(2),
            reason: CancellationReason::Caller,
        };
        let mut handler = ClientLifecycleHandler::new(client, protocol([cancel]));

        // Act
        let error = handler
            .handle(frame(FrameRoute::Control))
            .expect_err("client must reject a client-to-runtime message");

        // Assert
        assert_eq!(error, ClientControlError::InvalidDirection(cancel));
    }

    #[test]
    fn non_control_frames_remain_available_to_later_handlers() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let mut handler = ClientLifecycleHandler::new(client, protocol([]));
        let event_route = FrameRoute::Event {
            source: ObjectId::new(3),
            event: EventTag::new(4),
        };

        // Act
        let disposition = handler.handle(frame(event_route));

        // Assert
        assert!(matches!(
            disposition,
            Ok(FrameDisposition::Unhandled(frame)) if frame.route() == event_route
        ));
    }

    #[test]
    fn subscription_control_is_rejected_in_the_server_to_client_direction() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let subscription = ControlMessage::SetSubscription {
            object_id: ObjectId::new(3),
            event: EventTag::new(4),
            enabled: true,
        };
        let mut handler = ClientLifecycleHandler::new(client, protocol([subscription]));

        // Act
        let error = handler
            .handle(frame(FrameRoute::Control))
            .expect_err("subscription command is client-to-runtime only");

        // Assert
        assert_eq!(error, ClientControlError::InvalidDirection(subscription));
    }
}
