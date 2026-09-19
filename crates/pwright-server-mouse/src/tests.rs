use super::*;
use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
use pwright_cap_mouse::*;
use pwright_codec_mouse::{MouseCodec, MouseCodecLimits};
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

service!(MouseMove);
service!(MouseDown);
service!(MouseUp);
service!(MouseClick);
service!(MouseDblclick);
service!(MouseWheel);
fn number(value: f64) -> MouseNumber {
    MouseNumber::try_from(value).unwrap()
}

#[tokio::test]
async fn generated_chain_routes_six_commands_and_advertises_mouse() {
    // Arrange
    let version = ProtocolVersion::new(1, 0);
    let limits = FrameLimits::new(4096, 0, 0, 4096);
    let protocol = Arc::new(WireProtocol::new_exact_directional(
        MouseCodec::new(MouseCodecLimits::new(256)),
        version,
        limits,
        limits,
    ));
    let service = Arc::new(RecordingService::default());
    let handlers = mouse_server_handlers(
        protocol.clone(),
        MouseServerServices {
            mouse_move: service.clone(),
            mouse_down: service.clone(),
            mouse_up: service.clone(),
            mouse_click: service.clone(),
            mouse_dblclick: service.clone(),
            mouse_wheel: service.clone(),
        },
    );
    let transport = Arc::new(Transport::default());
    let frames = [
        protocol
            .encode(&Command::<MouseMove>::new(
                CallId::new(1),
                ObjectId::new(3),
                CallMetadata::default(),
                MoveParams {
                    x: number(12.5),
                    y: number(-24.25),
                    steps: MouseMoveSteps::try_from(-2.0).unwrap(),
                },
            ))
            .unwrap(),
        protocol
            .encode(&Command::<MouseDown>::new(
                CallId::new(2),
                ObjectId::new(3),
                CallMetadata::default(),
                DownParams {
                    button: MouseButton::Right,
                    click_count: -2,
                },
            ))
            .unwrap(),
        protocol
            .encode(&Command::<MouseUp>::new(
                CallId::new(3),
                ObjectId::new(3),
                CallMetadata::default(),
                UpParams {
                    button: MouseButton::Middle,
                    click_count: 2,
                },
            ))
            .unwrap(),
        protocol
            .encode(&Command::<MouseClick>::new(
                CallId::new(4),
                ObjectId::new(3),
                CallMetadata::default(),
                ClickParams {
                    x: number(5.5),
                    y: number(8.25),
                    button: MouseButton::Left,
                    click_count: MouseClickCount::try_from(3.0).unwrap(),
                    delay: Some(std::time::Duration::new(1, 500_000)),
                },
            ))
            .unwrap(),
        protocol
            .encode(&Command::<MouseDblclick>::new(
                CallId::new(5),
                ObjectId::new(3),
                CallMetadata::default(),
                DblclickParams {
                    x: number(5.5),
                    y: number(8.25),
                    button: MouseButton::Right,
                    delay: None,
                },
            ))
            .unwrap(),
        protocol
            .encode(&Command::<MouseWheel>::new(
                CallId::new(6),
                ObjectId::new(3),
                CallMetadata::default(),
                WheelParams {
                    delta_x: number(-1.5),
                    delta_y: number(90.25),
                },
            ))
            .unwrap(),
    ];
    // Act
    for frame in frames {
        let disposition = handlers
            .handle_concurrently(frame, transport.clone())
            .await
            .unwrap();
        assert!(matches!(disposition, ServerFrameDisposition::Handled(_)));
    }
    let manifest = handlers
        .capability_manifest_with_object_kinds(
            BROWSER_OBJECT_KIND_CATALOG,
            NegotiationLimits::new(8, 64),
        )
        .unwrap();
    // Assert
    assert_eq!(
        manifest.support(&WireCapabilityId::new("pwright.mouse", 64).unwrap()),
        Some(VersionPolicy::compatible_through(version))
    );
    assert_eq!(
        *service.0.lock().unwrap(),
        [
            MouseMove::METHOD,
            MouseDown::METHOD,
            MouseUp::METHOD,
            MouseClick::METHOD,
            MouseDblclick::METHOD,
            MouseWheel::METHOD
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
    response!(MouseMove, 1);
    response!(MouseDown, 2);
    response!(MouseUp, 3);
    response!(MouseClick, 4);
    response!(MouseDblclick, 5);
    response!(MouseWheel, 6);
    assert!(replies.next().is_none());
}
