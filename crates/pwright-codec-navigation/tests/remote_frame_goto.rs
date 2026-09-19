#![cfg(feature = "frame-navigation")]

use std::future::{Ready, ready};
use std::sync::{Arc, Mutex};

use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
use pwright_cap_navigation::v1_2::{
    FrameGoto, FrameGotoResult, GotoParams, NavigationErrorDetails, WaitUntil,
};
use pwright_client::{
    AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, FrameKind, PageKind,
    RemoteInitializer, RemoteKind,
};
use pwright_client_navigation::FrameGotoExt;
use pwright_client_wire::{DriverEvent, EncodedClientChannel};
use pwright_codec_navigation::v1_2::{NavigationCodec, NavigationCodecLimits};
use pwright_core::{
    CallId, CallMetadata, CommandSpec, ObjectId, OperationError, ProtocolVersion, Service,
};
use pwright_runtime::{DirectDispatcher, RuntimeLimits};
use pwright_server_wire::{EncodedServerDriver, ServerDriverEvent, TypedCommandHandler};
use pwright_transport::{FrameLimits, WireProtocol};
use pwright_transport_stream::{StreamFrameCodec, StreamFrameLimits};
use pwright_transport_tokio::TokioStreamTransport;

fn limits() -> FrameLimits {
    FrameLimits::new(4096, 0, 0, 4096)
}
fn protocol() -> WireProtocol<NavigationCodec> {
    WireProtocol::new_exact_directional(
        NavigationCodec::new(NavigationCodecLimits::new(256, 4, 256)),
        ProtocolVersion::new(1, 2),
        limits(),
        limits(),
    )
}

struct FrameService(Arc<Mutex<Vec<GotoParams>>>);
impl Service<GotoParams> for FrameService {
    type Response = FrameGotoResult;
    type Error = OperationError<NavigationErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;
    fn call(&self, params: GotoParams) -> Self::Future<'_> {
        self.0.lock().unwrap().push(params);
        ready(Ok(FrameGotoResult {
            frame_id: 4,
            response: None,
        }))
    }
}

#[tokio::test]
async fn frame_extension_roundtrips_through_typed_runtime_and_stream() {
    // Arrange
    let (client_io, server_io) = tokio::io::duplex(8192);
    let stream = StreamFrameCodec::new(StreamFrameLimits::new(limits(), 8192));
    let (channel, mut client_driver) = EncodedClientChannel::new(
        protocol(),
        TokioStreamTransport::from_io(client_io, stream),
        1,
    );
    let client = Client::new(channel, AtomicCallIdSource::new(41));
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
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        FrameService(Arc::clone(&recorded)),
        RuntimeLimits::new(4, 1),
        BROWSER_OBJECT_KIND_CATALOG,
    )
    .unwrap();
    for (id, parent, kind) in [
        (1, None, BrowserKind::OBJECT_KIND),
        (2, Some(1), BrowserContextKind::OBJECT_KIND),
        (3, Some(2), PageKind::OBJECT_KIND),
        (4, Some(3), FrameKind::OBJECT_KIND),
    ] {
        runtime
            .register_object(ObjectId::new(id), parent.map(ObjectId::new), kind)
            .unwrap();
    }
    let mut handler = TypedCommandHandler::<FrameGoto, _, _>::new(protocol(), runtime);
    let mut server_driver =
        EncodedServerDriver::new(TokioStreamTransport::from_io(server_io, stream));
    let params = GotoParams::new("about:blank#child").with_wait_until(WaitUntil::Commit);

    // Act
    let (result, server_event, client_event) = tokio::join!(
        frame.goto(params.clone(), CallMetadata::default()),
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    let result = result.unwrap();
    assert_eq!(result.frame_id, 4);
    assert_eq!(result.response, None);
    assert_eq!(recorded.lock().unwrap().as_slice(), &[params]);
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("Frame command was not handled")
    };
    assert_eq!(handled.call_id(), CallId::new(41));
    assert_eq!(handled.object_id(), ObjectId::new(4));
    assert_eq!(handled.method(), FrameGoto::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}
