use super::*;
use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
use pwright_cap_keyboard::*;
use pwright_codec_keyboard::{KeyboardCodec, KeyboardCodecLimits};
use pwright_core::{
    CallId, CallMetadata, Command, CommandResult, CommandSpec, MethodTag, ObjectId,
    ProtocolVersion, Service,
};
use pwright_server_wire::{
    ConcurrentServerFrameHandler, ServerCapabilitySource, ServerFrameDisposition,
};
use pwright_transport::{
    FrameLimits, FrameTransport, NegotiationLimits, VersionPolicy, WireCapabilityId, WireFrame,
    WireProtocol,
};
use std::{
    convert::Infallible,
    future::{Ready, ready},
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct Transport(Mutex<Vec<WireFrame>>);
impl FrameTransport for Transport {
    type Error = Infallible;
    type Send<'a> = Ready<Result<(), Self::Error>>;
    type Receive<'a> = Ready<Result<Option<WireFrame>, Self::Error>>;
    fn send(&self, frame: WireFrame) -> Self::Send<'_> {
        self.0.lock().unwrap().push(frame);
        ready(Ok(()))
    }
    fn receive(&self) -> Self::Receive<'_> {
        ready(Ok(None))
    }
}

#[derive(Default)]
struct RecordingService(Mutex<Vec<(MethodTag, ObjectId)>>);
macro_rules! service {
    ($command:ty) => {
        impl Service<Command<$command>> for RecordingService {
            type Response = CommandResult<$command>;
            type Error = Infallible;
            type Future<'a> = Ready<Result<Self::Response, Self::Error>>;
            fn call(&self, command: Command<$command>) -> Self::Future<'_> {
                self.0
                    .lock()
                    .unwrap()
                    .push((<$command>::METHOD, command.object_id()));
                ready(Ok(CommandResult::new(command.call_id(), Ok(()))))
            }
        }
    };
}
service!(KeyboardDown);
service!(KeyboardUp);
service!(KeyboardPress);
service!(KeyboardInsertText);
service!(KeyboardType);

#[tokio::test]
async fn generated_chain_routes_all_five_typed_commands_and_advertises_the_capability() {
    // Arrange
    let version = ProtocolVersion::new(1, 0);
    let limits = FrameLimits::new(4096, 0, 0, 4096);
    let protocol = Arc::new(WireProtocol::new_exact_directional(
        KeyboardCodec::new(KeyboardCodecLimits::new(256)),
        version,
        limits,
        limits,
    ));
    let service = Arc::new(RecordingService::default());
    let handlers = keyboard_server_handlers(
        protocol.clone(),
        KeyboardServerServices {
            keyboard_down: service.clone(),
            keyboard_up: service.clone(),
            keyboard_press: service.clone(),
            keyboard_insert_text: service.clone(),
            keyboard_type: service.clone(),
        },
    );
    let transport = Arc::new(Transport::default());
    let key = KeyboardKey::copy_from_str("Shift+").unwrap();
    let text = KeyboardText::copy_from_str("hello").unwrap();
    let frames = [
        protocol
            .encode(&Command::<KeyboardDown>::new(
                CallId::new(1),
                ObjectId::new(3),
                CallMetadata::default(),
                DownParams { key: key.clone() },
            ))
            .unwrap(),
        protocol
            .encode(&Command::<KeyboardUp>::new(
                CallId::new(2),
                ObjectId::new(3),
                CallMetadata::default(),
                UpParams { key: key.clone() },
            ))
            .unwrap(),
        protocol
            .encode(&Command::<KeyboardPress>::new(
                CallId::new(3),
                ObjectId::new(3),
                CallMetadata::default(),
                PressParams { key, delay: None },
            ))
            .unwrap(),
        protocol
            .encode(&Command::<KeyboardInsertText>::new(
                CallId::new(4),
                ObjectId::new(3),
                CallMetadata::default(),
                InsertTextParams { text: text.clone() },
            ))
            .unwrap(),
        protocol
            .encode(&Command::<KeyboardType>::new(
                CallId::new(5),
                ObjectId::new(3),
                CallMetadata::default(),
                TypeParams { text, delay: None },
            ))
            .unwrap(),
    ];
    // Act
    for frame in frames {
        assert!(matches!(
            handlers
                .handle_concurrently(frame, transport.clone())
                .await
                .unwrap(),
            ServerFrameDisposition::Handled(_)
        ));
    }
    let manifest = handlers
        .capability_manifest_with_object_kinds(
            BROWSER_OBJECT_KIND_CATALOG,
            NegotiationLimits::new(8, 64),
        )
        .unwrap();
    // Assert
    assert_eq!(
        manifest.support(&WireCapabilityId::new("pwright.keyboard", 64).unwrap()),
        Some(VersionPolicy::compatible_through(version))
    );
    assert_eq!(
        *service.0.lock().unwrap(),
        [
            KeyboardDown::METHOD,
            KeyboardUp::METHOD,
            KeyboardPress::METHOD,
            KeyboardInsertText::METHOD,
            KeyboardType::METHOD
        ]
        .map(|method| (method, ObjectId::new(3)))
    );
    let mut replies = transport
        .0
        .lock()
        .unwrap()
        .drain(..)
        .collect::<Vec<_>>()
        .into_iter();
    macro_rules! response {
        ($command:ty, $id:expr) => {{
            let response = protocol
                .decode::<CommandResult<$command>>(replies.next().unwrap())
                .unwrap();
            assert_eq!(response.call_id(), CallId::new($id));
            assert_eq!(response.into_outcome(), Ok(()));
        }};
    }
    response!(KeyboardDown, 1);
    response!(KeyboardUp, 2);
    response!(KeyboardPress, 3);
    response!(KeyboardInsertText, 4);
    response!(KeyboardType, 5);
    assert!(replies.next().is_none());
}
