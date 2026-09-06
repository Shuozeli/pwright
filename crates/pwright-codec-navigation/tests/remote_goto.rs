use std::future::{Ready, ready};
use std::sync::{Arc, Mutex};

use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
use pwright_cap_navigation::{
    Goto, GotoParams, GotoResult, Header, NavigationErrorDetails, NavigationResponse, WaitUntil,
};
use pwright_client::{
    AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, PageKind, RemoteInitializer,
    RemoteKind,
};
use pwright_client_navigation::PageGotoExt;
use pwright_client_wire::{DriverEvent, EncodedClientChannel};
use pwright_codec_navigation::{NavigationCodec, NavigationCodecLimits};
use pwright_core::{
    CallId, CallMetadata, CommandSpec, ObjectId, OperationError, ProtocolVersion, Service,
};
use pwright_runtime::{DirectDispatcher, RuntimeLimits};
use pwright_server_wire::{EncodedServerDriver, ServerDriverEvent, TypedCommandHandler};
use pwright_transport::{FrameLimits, VersionPolicy, WireProtocol};
use pwright_transport_stream::{StreamFrameCodec, StreamFrameLimits};
use pwright_transport_tokio::TokioStreamTransport;
use tokio::io::duplex;

const BROWSER_ID: ObjectId = ObjectId::new(1);
const CONTEXT_ID: ObjectId = ObjectId::new(2);
const PAGE_ID: ObjectId = ObjectId::new(3);

fn frame_limits() -> FrameLimits {
    FrameLimits::new(4096, 0, 0, 4096)
}

fn protocol() -> WireProtocol<NavigationCodec> {
    WireProtocol::new(
        NavigationCodec::new(NavigationCodecLimits::new(1024, 16, 2048)),
        VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
            .expect("version policy should build"),
        ProtocolVersion::new(1, 0),
        frame_limits(),
    )
    .expect("protocol should build")
}

struct RecordingGotoService {
    recorded: Arc<Mutex<Vec<GotoParams>>>,
}

impl Service<GotoParams> for RecordingGotoService {
    type Response = GotoResult;
    type Error = OperationError<NavigationErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: GotoParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("recorded navigation should be available")
            .push(params);
        ready(Ok(GotoResult {
            frame_id: "main-frame".to_string(),
            loader_id: Some("document-loader".to_string()),
            url: "https://example.com/final".to_string(),
            same_document: false,
            response: Some(NavigationResponse {
                url: "https://example.com/final".to_string(),
                status: 200,
                status_text: "OK".to_string(),
                headers: vec![Header {
                    name: "content-type".to_string(),
                    value: "text/html".to_string(),
                }],
            }),
        }))
    }
}

#[tokio::test]
async fn page_goto_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(8192);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 8192));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(41));
    let browser = client
        .browser(RemoteInitializer::new(
            BROWSER_ID,
            None,
            BrowserKind::OBJECT_KIND,
        ))
        .expect("browser should initialize");
    let context = browser
        .context(RemoteInitializer::new(
            CONTEXT_ID,
            Some(BROWSER_ID),
            BrowserContextKind::OBJECT_KIND,
        ))
        .expect("context should initialize");
    let page = context
        .page(RemoteInitializer::new(
            PAGE_ID,
            Some(CONTEXT_ID),
            PageKind::OBJECT_KIND,
        ))
        .expect("page should initialize");
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingGotoService {
            recorded: Arc::clone(&recorded),
        },
        RuntimeLimits::new(3, 1),
        BROWSER_OBJECT_KIND_CATALOG,
    )
    .expect("browser object catalog should be valid");
    runtime
        .register_object(BROWSER_ID, None, BrowserKind::OBJECT_KIND)
        .expect("runtime browser should register");
    runtime
        .register_object(
            CONTEXT_ID,
            Some(BROWSER_ID),
            BrowserContextKind::OBJECT_KIND,
        )
        .expect("runtime context should register");
    runtime
        .register_object(PAGE_ID, Some(CONTEXT_ID), PageKind::OBJECT_KIND)
        .expect("runtime page should register");
    let mut handler = TypedCommandHandler::<Goto, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);
    let params = GotoParams::new("https://example.com/start")
        .with_referrer("https://referrer.example")
        .with_wait_until(WaitUntil::NetworkIdle);

    // Act
    let call = page.goto(params.clone(), CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    let result = result.expect("remote goto should succeed");
    assert_eq!(
        recorded
            .lock()
            .expect("recorded navigation should be available")
            .as_slice(),
        &[params]
    );
    assert_eq!(result.url, "https://example.com/final");
    assert_eq!(result.response.expect("response should exist").status, 200);
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the goto command");
    };
    assert_eq!(handled.call_id(), CallId::new(41));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), Goto::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}
