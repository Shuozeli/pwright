use std::future::{Pending, Ready, pending, ready};
use std::num::{NonZeroU8, NonZeroU32};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
use pwright_cap_locator::{
    AllInnerTexts, AllInnerTextsParams, AllTextContents, AllTextContentsParams, AriaRole, Blur,
    BlurParams, Click, ClickOptions, ClickOptionsV2, ClickParams, ClickParamsV2, Count,
    CountParams, CssSelector, Fill, FillOptions, FillParams, Focus, FocusParams, GetAttribute,
    GetAttributeParams, Hover, HoverOptions, HoverParams, InnerHtml, InnerHtmlParams, InnerText,
    InnerTextParams, InputValue, InputValueParams, IsEnabled, IsEnabledParams, IsVisible,
    IsVisibleParams, KeyboardModifier, LocatorErrorDetails, Position, Press, PressKey,
    PressOptions, PressParams, RoleOptions, ScrollIntoViewIfNeededParams, ScrollMode,
    SelectOptionOptions, SelectOptionParams, SelectOptionValue, Selector, SetChecked,
    SetCheckedOptions, SetCheckedParams, TextContent, TextContentParams, WaitFor, WaitForOptions,
    WaitForParams, WaitForState, v2 as locator_v2,
};
use pwright_client::{
    AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, PageKind, RemoteInitializer,
    RemoteKind,
};
use pwright_client_locator::{
    LocatorAllInnerTextsExt, LocatorAllTextContentsExt, LocatorBlurExt, LocatorCheckExt,
    LocatorClickExt, LocatorCountExt, LocatorEnabledExt, LocatorFactoryExt, LocatorFillExt,
    LocatorFocusExt, LocatorGetAttributeExt, LocatorHoverExt, LocatorInnerHtmlExt,
    LocatorInnerTextExt, LocatorInputValueExt, LocatorPressExt, LocatorTextContentExt,
    LocatorVisibilityExt, LocatorWaitExt,
};
use pwright_client_wire::{DriverEvent, EncodedClientChannel};
use pwright_codec_control::ControlCodec;
use pwright_codec_locator::{LocatorCodec, LocatorCodecLimits, LocatorV2Codec};
use pwright_core::{
    CallId, CallMetadata, CancellationReason, Command, CommandCancellation, CommandSpec, ObjectId,
    OperationContext, OperationError, ProtocolVersion, Service,
};
use pwright_runtime::{ControlledDispatcher, DirectDispatcher, NoDeadlineScheduler, RuntimeLimits};
use pwright_runtime_control::{RuntimeControlEvent, RuntimeControlReceiver};
use pwright_server_wire::{
    ConcurrentTypedCommandHandler, EncodedServerDriver, EncodedServerSupervisor, ServerDriverError,
    ServerDriverEvent, ServerEofPolicy, ServerSupervisorEvent, ServerSupervisorLimits,
    TypedCommandHandler, TypedCommandHandlerError,
};
use pwright_transport::{
    FrameLimits, FrameRoute, FrameTransport, VersionPolicy, WireFrame, WireProtocol,
};
use pwright_transport_stream::{StreamFrameCodec, StreamFrameLimits};
use pwright_transport_tokio::TokioStreamTransport;
use tokio::io::duplex;

const BROWSER_ID: ObjectId = ObjectId::new(1);
const CONTEXT_ID: ObjectId = ObjectId::new(2);
const PAGE_ID: ObjectId = ObjectId::new(3);

fn frame_limits() -> FrameLimits {
    FrameLimits::new(2048, 0, 0, 2048)
}

fn protocol() -> WireProtocol<LocatorCodec> {
    WireProtocol::new(
        LocatorCodec::new(LocatorCodecLimits::new(512))
            .with_capability_version(ProtocolVersion::new(1, 22)),
        VersionPolicy::exact(ProtocolVersion::new(1, 0)),
        ProtocolVersion::new(1, 0),
        frame_limits(),
    )
    .expect("protocol should build")
}

fn protocol_v2() -> WireProtocol<LocatorV2Codec> {
    WireProtocol::new(
        LocatorV2Codec::new(LocatorCodecLimits::new(512))
            .with_capability_version(ProtocolVersion::new(2, 5)),
        VersionPolicy::exact(ProtocolVersion::new(2, 5)),
        ProtocolVersion::new(2, 5),
        frame_limits(),
    )
    .expect("locator v2 protocol should build")
}

fn control_protocol() -> WireProtocol<ControlCodec> {
    WireProtocol::new(
        ControlCodec,
        VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
            .expect("version policy should build"),
        ProtocolVersion::new(1, 0),
        frame_limits(),
    )
    .expect("control protocol should build")
}

struct RecordingClickService {
    recorded: Arc<Mutex<Vec<ClickParams>>>,
}

struct RecordingClickV2Service {
    recorded: Arc<Mutex<Vec<ClickParamsV2>>>,
}

struct RecordingHoverService {
    recorded: Arc<Mutex<Vec<HoverParams>>>,
}

struct RecordingFocusService {
    recorded: Arc<Mutex<Vec<FocusParams>>>,
}

struct RecordingBlurService {
    recorded: Arc<Mutex<Vec<BlurParams>>>,
}

struct RecordingScrollIntoViewIfNeededService {
    recorded: Arc<Mutex<Vec<ScrollIntoViewIfNeededParams>>>,
}

struct RecordingPressService {
    recorded: Arc<Mutex<Vec<PressParams>>>,
}

struct RecordingSetCheckedService {
    recorded: Arc<Mutex<Vec<SetCheckedParams>>>,
}

struct RecordingSelectOptionService {
    recorded: Arc<Mutex<Vec<SelectOptionParams>>>,
    result: Vec<String>,
}

impl Service<ClickParams> for RecordingClickService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: ClickParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("click recording should be available")
            .push(params);
        ready(Ok(()))
    }
}

impl Service<ClickParamsV2> for RecordingClickV2Service {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: ClickParamsV2) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("locator v2 click recording should be available")
            .push(params);
        ready(Ok(()))
    }
}

impl Service<HoverParams> for RecordingHoverService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: HoverParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("hover recording should be available")
            .push(params);
        ready(Ok(()))
    }
}

impl Service<FocusParams> for RecordingFocusService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: FocusParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("focus recording should be available")
            .push(params);
        ready(Ok(()))
    }
}

impl Service<BlurParams> for RecordingBlurService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: BlurParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("blur recording should be available")
            .push(params);
        ready(Ok(()))
    }
}

impl Service<ScrollIntoViewIfNeededParams> for RecordingScrollIntoViewIfNeededService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: ScrollIntoViewIfNeededParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("scroll recording should be available")
            .push(params);
        ready(Ok(()))
    }
}

impl Service<PressParams> for RecordingPressService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: PressParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("press recording should be available")
            .push(params);
        ready(Ok(()))
    }
}

impl Service<SetCheckedParams> for RecordingSetCheckedService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: SetCheckedParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("SetChecked recording should be available")
            .push(params);
        ready(Ok(()))
    }
}

impl Service<SelectOptionParams> for RecordingSelectOptionService {
    type Response = Vec<String>;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: SelectOptionParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("SelectOption recording should be available")
            .push(params);
        ready(Ok(self.result.clone()))
    }
}

struct RecordingFillService {
    recorded: Arc<Mutex<Vec<FillParams>>>,
}

struct RecordingCountService {
    recorded: Arc<Mutex<Vec<CountParams>>>,
    count: u32,
}

struct RecordingAllTextContentsService {
    recorded: Arc<Mutex<Vec<AllTextContentsParams>>>,
    values: Vec<String>,
}

struct RecordingAllInnerTextsService {
    recorded: Arc<Mutex<Vec<AllInnerTextsParams>>>,
    values: Vec<String>,
}

struct RecordingVisibilityService {
    recorded: Arc<Mutex<Vec<IsVisibleParams>>>,
    visible: bool,
}

struct RecordingEnabledService {
    recorded: Arc<Mutex<Vec<IsEnabledParams>>>,
    enabled: bool,
}

struct RecordingWaitService {
    recorded: Arc<Mutex<Vec<WaitForParams>>>,
}

struct RecordingTextContentService {
    recorded: Arc<Mutex<Vec<TextContentParams>>>,
}

struct RecordingInnerTextService {
    recorded: Arc<Mutex<Vec<InnerTextParams>>>,
}

struct RecordingInputValueService {
    recorded: Arc<Mutex<Vec<InputValueParams>>>,
}

struct RecordingInnerHtmlService {
    recorded: Arc<Mutex<Vec<InnerHtmlParams>>>,
}

struct RecordingGetAttributeService {
    recorded: Arc<Mutex<Vec<GetAttributeParams>>>,
}

impl Service<GetAttributeParams> for RecordingGetAttributeService {
    type Response = Option<String>;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: GetAttributeParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("get-attribute recording should be available")
            .push(params);
        ready(Ok(Some("ready".to_string())))
    }
}

impl Service<InnerHtmlParams> for RecordingInnerHtmlService {
    type Response = String;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: InnerHtmlParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("inner-HTML recording should be available")
            .push(params);
        ready(Ok("<span>Ready</span>".to_string()))
    }
}

impl Service<InnerTextParams> for RecordingInnerTextService {
    type Response = String;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: InnerTextParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("inner-text recording should be available")
            .push(params);
        ready(Ok("Visible text".to_string()))
    }
}

impl Service<InputValueParams> for RecordingInputValueService {
    type Response = String;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: InputValueParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("input-value recording should be available")
            .push(params);
        ready(Ok("alice@example.test".to_string()))
    }
}

impl Service<TextContentParams> for RecordingTextContentService {
    type Response = Option<String>;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: TextContentParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("text-content recording should be available")
            .push(params);
        ready(Ok(Some("  Ready\nnow  ".to_string())))
    }
}

impl Service<WaitForParams> for RecordingWaitService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: WaitForParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("wait recording should be available")
            .push(params);
        ready(Ok(()))
    }
}

impl Service<IsEnabledParams> for RecordingEnabledService {
    type Response = bool;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: IsEnabledParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("enabled-state recording should be available")
            .push(params);
        ready(Ok(self.enabled))
    }
}

impl Service<IsVisibleParams> for RecordingVisibilityService {
    type Response = bool;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: IsVisibleParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("visibility recording should be available")
            .push(params);
        ready(Ok(self.visible))
    }
}

impl Service<CountParams> for RecordingCountService {
    type Response = u32;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: CountParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("count recording should be available")
            .push(params);
        ready(Ok(self.count))
    }
}

impl Service<AllTextContentsParams> for RecordingAllTextContentsService {
    type Response = Vec<String>;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: AllTextContentsParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("all-text-contents recording should be available")
            .push(params);
        ready(Ok(self.values.clone()))
    }
}

impl Service<AllInnerTextsParams> for RecordingAllInnerTextsService {
    type Response = Vec<String>;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: AllInnerTextsParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("all-inner-texts recording should be available")
            .push(params);
        ready(Ok(self.values.clone()))
    }
}

#[derive(Clone)]
struct PendingFillService {
    cancellations: Arc<Mutex<Vec<(OperationContext, CancellationReason)>>>,
}

impl Service<FillParams> for PendingFillService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Pending<Result<Self::Response, Self::Error>>;

    fn call(&self, _params: FillParams) -> Self::Future<'_> {
        pending()
    }
}

impl CommandCancellation<Fill> for PendingFillService {
    type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

    fn cancel(&self, context: OperationContext, reason: CancellationReason) -> Self::Future<'_> {
        self.cancellations
            .lock()
            .expect("cancellations should be available")
            .push((context, reason));
        ready(Ok(()))
    }
}

impl Service<FillParams> for RecordingFillService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, params: FillParams) -> Self::Future<'_> {
        self.recorded
            .lock()
            .expect("fill recording should be available")
            .push(params);
        ready(Ok(()))
    }
}

#[tokio::test]
async fn locator_click_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(31));
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
    let selector = Selector::role(
        AriaRole::Dialog,
        RoleOptions::new().with_name("Settings", true),
    )
    .expect("valid dialog role")
    .within(
        Selector::role(AriaRole::Button, RoleOptions::new().with_name("Save", true))
            .expect("valid button role"),
    )
    .and(Selector::label("Primary action", false))
    .filter_text("Save", true)
    .nth(0);
    let locator = page.locator(selector.clone());
    let options = ClickOptions::new().with_force(true).with_trial(true);
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingClickService {
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
    let mut handler = TypedCommandHandler::<Click, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.click(options, CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("click recording should be available")
            .as_slice(),
        &[ClickParams::new(selector, options)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the click command");
    };
    assert_eq!(handled.call_id(), CallId::new(31));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), Click::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_v2_click_round_trips_all_options_through_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) =
        EncodedClientChannel::new(protocol_v2(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(51));
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
    let selector = Selector::css("button.save").expect("selector should be valid");
    let locator = page.locator(selector.clone());
    let options = ClickOptionsV2::new()
        .with_button(pwright_cap_locator::MouseButton::Right)
        .with_click_count(NonZeroU8::new(2).expect("two is nonzero"))
        .with_delay(Duration::from_millis(12))
        .with_force(false)
        .with_modifier(KeyboardModifier::ControlOrMeta)
        .with_no_wait_after(true)
        .with_position(Position::new(8.5, 4.25).expect("finite point should build"))
        .with_scroll(ScrollMode::None)
        .with_steps(NonZeroU32::new(3).expect("three is nonzero"))
        .with_trial(false);
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingClickV2Service {
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
    let mut handler = TypedCommandHandler::<locator_v2::Click, _, _>::new(protocol_v2(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = pwright_client_locator::v2::LocatorClickExt::click(
        &locator,
        options.clone(),
        CallMetadata::default(),
    );
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("locator v2 click recording should be available")
            .as_slice(),
        &[ClickParamsV2::new(selector, options)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the locator v2 click command");
    };
    assert_eq!(handled.call_id(), CallId::new(51));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), locator_v2::Click::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_hover_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(32));
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
    let selector = Selector::css("button").expect("selector should be valid");
    let locator = page.locator(selector.clone());
    let options = HoverOptions::new()
        .with_force(true)
        .with_scroll(ScrollMode::None)
        .with_modifier(KeyboardModifier::ControlOrMeta)
        .with_modifier(KeyboardModifier::Shift)
        .with_position(Position::new(12.5, 3.25).expect("finite position should build"));
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingHoverService {
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
    let mut handler = TypedCommandHandler::<Hover, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.hover(options, CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("hover recording should be available")
            .as_slice(),
        &[HoverParams::new(selector, options)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the hover command");
    };
    assert_eq!(handled.call_id(), CallId::new(32));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), Hover::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_focus_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(33));
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
    let selector = Selector::test_id("focus-target");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingFocusService {
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
    let mut handler = TypedCommandHandler::<Focus, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.focus(CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("focus recording should be available")
            .as_slice(),
        &[FocusParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the focus command");
    };
    assert_eq!(handled.call_id(), CallId::new(33));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), Focus::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_v2_focus_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) =
        EncodedClientChannel::new(protocol_v2(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(53));
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
    let selector = Selector::test_id("focus-target-v2");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingFocusService {
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
    let mut handler = TypedCommandHandler::<locator_v2::Focus, _, _>::new(protocol_v2(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call =
        pwright_client_locator::v2::LocatorFocusExt::focus(&locator, CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("focus recording should be available")
            .as_slice(),
        &[FocusParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the locator v2 focus command");
    };
    assert_eq!(handled.call_id(), CallId::new(53));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), locator_v2::Focus::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_v2_scroll_into_view_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) =
        EncodedClientChannel::new(protocol_v2(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(54));
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
    let selector = Selector::test_id("below-fold-v2");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingScrollIntoViewIfNeededService {
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
    let mut handler = TypedCommandHandler::<locator_v2::ScrollIntoViewIfNeeded, _, _>::new(
        protocol_v2(),
        runtime,
    );
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call =
        pwright_client_locator::v2::LocatorScrollIntoViewIfNeededExt::scroll_into_view_if_needed(
            &locator,
            CallMetadata::default(),
        );
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("scroll recording should be available")
            .as_slice(),
        &[ScrollIntoViewIfNeededParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the locator v2 scroll command");
    };
    assert_eq!(handled.call_id(), CallId::new(54));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), locator_v2::ScrollIntoViewIfNeeded::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_v2_blur_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) =
        EncodedClientChannel::new(protocol_v2(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(54));
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
    let selector = Selector::test_id("blur-target-v2");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingBlurService {
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
    let mut handler = TypedCommandHandler::<locator_v2::Blur, _, _>::new(protocol_v2(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = pwright_client_locator::v2::LocatorBlurExt::blur(&locator, CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("blur recording should be available")
            .as_slice(),
        &[BlurParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the locator v2 blur command");
    };
    assert_eq!(handled.call_id(), CallId::new(54));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), locator_v2::Blur::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_v2_press_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) =
        EncodedClientChannel::new(protocol_v2(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(55));
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
    let selector = Selector::test_id("press-target-v2");
    let key = PressKey::new("ControlOrMeta+Enter").expect("key should build");
    let options = PressOptions::new()
        .with_delay(Duration::from_millis(7))
        .with_no_wait_after(true);
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingPressService {
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
    let mut handler = TypedCommandHandler::<locator_v2::Press, _, _>::new(protocol_v2(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = pwright_client_locator::v2::LocatorPressExt::press(
        &locator,
        key.clone(),
        options,
        CallMetadata::default(),
    );
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("press recording should be available")
            .as_slice(),
        &[PressParams::new(selector, key, options)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the locator v2 press command");
    };
    assert_eq!(handled.call_id(), CallId::new(55));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), locator_v2::Press::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_v2_uncheck_composes_over_one_typed_remote_set_checked_command() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) =
        EncodedClientChannel::new(protocol_v2(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(56));
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
    let selector = Selector::label("Remember me", true);
    let locator = page.locator(selector.clone());
    let options = SetCheckedOptions::new().with_force(false).with_trial(false);
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingSetCheckedService {
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
    let mut handler =
        TypedCommandHandler::<locator_v2::SetChecked, _, _>::new(protocol_v2(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = pwright_client_locator::v2::LocatorUncheckExt::uncheck(
        &locator,
        options,
        CallMetadata::default(),
    );
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("SetChecked recording should be available")
            .as_slice(),
        &[SetCheckedParams::new(selector, false, options)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the locator v2 SetChecked command");
    };
    assert_eq!(handled.call_id(), CallId::new(56));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), locator_v2::SetChecked::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_v2_select_option_round_trips_descriptors_and_values_through_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) =
        EncodedClientChannel::new(protocol_v2(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(57));
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
    let selector = Selector::label("Color", true);
    let locator = page.locator(selector.clone());
    let values = vec![
        SelectOptionValue::value_or_label("Blue"),
        SelectOptionValue::new()
            .with_value("green")
            .with_label("Green"),
    ];
    let options = SelectOptionOptions::new().with_force(false);
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingSelectOptionService {
            recorded: Arc::clone(&recorded),
            result: vec!["blue".to_owned(), "green".to_owned()],
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
    let mut handler =
        TypedCommandHandler::<locator_v2::SelectOption, _, _>::new(protocol_v2(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = pwright_client_locator::v2::LocatorSelectOptionExt::select_option(
        &locator,
        values.clone(),
        options,
        CallMetadata::default(),
    );
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(vec!["blue".to_owned(), "green".to_owned()]));
    assert_eq!(
        recorded
            .lock()
            .expect("SelectOption recording should be available")
            .as_slice(),
        &[SelectOptionParams::new(selector, values, options)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the locator v2 SelectOption command");
    };
    assert_eq!(handled.call_id(), CallId::new(57));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), locator_v2::SelectOption::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_blur_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(34));
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
    let selector = Selector::test_id("blur-target");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingBlurService {
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
    let mut handler = TypedCommandHandler::<Blur, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.blur(CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("blur recording should be available")
            .as_slice(),
        &[BlurParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the blur command");
    };
    assert_eq!(handled.call_id(), CallId::new(34));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), Blur::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_press_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(35));
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
    let selector = Selector::test_id("press-target");
    let key = PressKey::new("Shift++").expect("key should build");
    let options = PressOptions::new().with_delay(Duration::from_millis(3));
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingPressService {
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
    let mut handler = TypedCommandHandler::<Press, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.press(key.clone(), options, CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("press recording should be available")
            .as_slice(),
        &[PressParams::new(selector, key, options)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the press command");
    };
    assert_eq!(handled.call_id(), CallId::new(35));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), Press::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_check_composes_over_one_typed_remote_set_checked_command() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(57));
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
    let selector = Selector::test_id("subscribe");
    let locator = page.locator(selector.clone());
    let options = SetCheckedOptions::new().with_scroll(ScrollMode::None);
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingSetCheckedService {
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
    let mut handler = TypedCommandHandler::<SetChecked, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.check(options, CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("SetChecked recording should be available")
            .as_slice(),
        &[SetCheckedParams::new(selector, true, options)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the SetChecked command");
    };
    assert_eq!(handled.call_id(), CallId::new(57));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), SetChecked::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_count_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(36));
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
    let selector = Selector::css(".row").expect("row selector should be valid");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingCountService {
            recorded: Arc::clone(&recorded),
            count: 3,
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
    let mut handler = TypedCommandHandler::<Count, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.count(CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(3));
    assert_eq!(
        recorded
            .lock()
            .expect("count recording should be available")
            .as_slice(),
        &[CountParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the count command");
    };
    assert_eq!(handled.call_id(), CallId::new(36));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), Count::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_all_inner_texts_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(38));
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
    let selector = Selector::css(".row").expect("row selector should be valid");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingAllInnerTextsService {
            recorded: Arc::clone(&recorded),
            values: vec![
                "rendered first".into(),
                String::new(),
                "rendered third".into(),
            ],
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
    let mut handler = TypedCommandHandler::<AllInnerTexts, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.all_inner_texts(CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(
        result,
        Ok(vec![
            "rendered first".into(),
            "".into(),
            "rendered third".into()
        ])
    );
    assert_eq!(
        recorded
            .lock()
            .expect("all-inner-texts recording should be available")
            .as_slice(),
        &[AllInnerTextsParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the all-inner-texts command");
    };
    assert_eq!(handled.call_id(), CallId::new(38));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), AllInnerTexts::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_all_text_contents_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(37));
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
    let selector = Selector::css(".row").expect("row selector should be valid");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingAllTextContentsService {
            recorded: Arc::clone(&recorded),
            values: vec!["first".into(), String::new(), "third".into()],
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
    let mut handler = TypedCommandHandler::<AllTextContents, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.all_text_contents(CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(vec!["first".into(), "".into(), "third".into()]));
    assert_eq!(
        recorded
            .lock()
            .expect("all-text-contents recording should be available")
            .as_slice(),
        &[AllTextContentsParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the all-text-contents command");
    };
    assert_eq!(handled.call_id(), CallId::new(37));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), AllTextContents::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_hidden_composes_over_one_remote_visibility_command() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(38));
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
    let selector = Selector::test_id("status");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingVisibilityService {
            recorded: Arc::clone(&recorded),
            visible: false,
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
    let mut handler = TypedCommandHandler::<IsVisible, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.is_hidden(CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(true));
    assert_eq!(
        recorded
            .lock()
            .expect("visibility recording should be available")
            .as_slice(),
        &[IsVisibleParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the visibility command");
    };
    assert_eq!(handled.call_id(), CallId::new(38));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), IsVisible::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_disabled_composes_over_one_remote_enabled_command() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(40));
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
    let selector = Selector::test_id("submit");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingEnabledService {
            recorded: Arc::clone(&recorded),
            enabled: false,
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
    let mut handler = TypedCommandHandler::<IsEnabled, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.is_disabled(CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(true));
    assert_eq!(
        recorded
            .lock()
            .expect("enabled-state recording should be available")
            .as_slice(),
        &[IsEnabledParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the enabled-state command");
    };
    assert_eq!(handled.call_id(), CallId::new(40));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), IsEnabled::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_wait_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
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
    let selector = Selector::test_id("status");
    let locator = page.locator(selector.clone());
    let options = WaitForOptions::new().with_state(WaitForState::Hidden);
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingWaitService {
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
    let mut handler = TypedCommandHandler::<WaitFor, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.wait_for(options, CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("wait recording should be available")
            .as_slice(),
        &[WaitForParams::new(selector, options)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the wait command");
    };
    assert_eq!(handled.call_id(), CallId::new(41));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), WaitFor::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_text_content_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(42));
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
    let selector = Selector::test_id("status");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingTextContentService {
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
    let mut handler = TypedCommandHandler::<TextContent, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.text_content(CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(
        result
            .expect("text-content query should succeed")
            .as_deref(),
        Some("  Ready\nnow  ")
    );
    assert_eq!(
        recorded
            .lock()
            .expect("text-content recording should be available")
            .as_slice(),
        &[TextContentParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the text-content command");
    };
    assert_eq!(handled.call_id(), CallId::new(42));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), TextContent::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_inner_html_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(44));
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
    let selector = Selector::test_id("container");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingInnerHtmlService {
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
    let mut handler = TypedCommandHandler::<InnerHtml, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.inner_html(CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok("<span>Ready</span>".to_string()));
    assert_eq!(
        recorded
            .lock()
            .expect("inner-HTML recording should be available")
            .as_slice(),
        &[InnerHtmlParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the inner-HTML command");
    };
    assert_eq!(handled.call_id(), CallId::new(44));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), InnerHtml::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_get_attribute_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(45));
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
    let selector = Selector::test_id("container");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingGetAttributeService {
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
    let mut handler = TypedCommandHandler::<GetAttribute, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.get_attribute("data-state", CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(Some("ready".to_string())));
    assert_eq!(
        recorded
            .lock()
            .expect("get-attribute recording should be available")
            .as_slice(),
        &[GetAttributeParams::new(selector, "data-state")]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the get-attribute command");
    };
    assert_eq!(handled.call_id(), CallId::new(45));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), GetAttribute::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_inner_text_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(43));
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
    let selector = Selector::test_id("status");
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingInnerTextService {
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
    let mut handler = TypedCommandHandler::<InnerText, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.inner_text(CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok("Visible text".to_string()));
    assert_eq!(
        recorded
            .lock()
            .expect("inner-text recording should be available")
            .as_slice(),
        &[InnerTextParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the inner-text command");
    };
    assert_eq!(handled.call_id(), CallId::new(43));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), InnerText::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_input_value_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) = EncodedClientChannel::new(protocol(), client_transport, 1);
    let client = Client::new(channel, AtomicCallIdSource::new(46));
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
    let selector = Selector::label("Account", true);
    let locator = page.locator(selector.clone());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingInputValueService {
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
    let mut handler = TypedCommandHandler::<InputValue, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.input_value(CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok("alice@example.test".to_string()));
    assert_eq!(
        recorded
            .lock()
            .expect("input-value recording should be available")
            .as_slice(),
        &[InputValueParams::new(selector)]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the input-value command");
    };
    assert_eq!(handled.call_id(), CallId::new(46));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), InputValue::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn locator_fill_round_trips_through_typed_client_server_and_stream() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
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
    let locator = page.get_by_label("Email address", true);
    let options = FillOptions::new().with_force(true);
    let expected = FillParams::new(
        Selector::label("Email address", true),
        "user@example.com",
        options,
    );
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingFillService {
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
    let mut handler = TypedCommandHandler::<Fill, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);

    // Act
    let call = locator.fill("user@example.com", options, CallMetadata::default());
    let (result, server_event, client_event) = tokio::join!(
        call,
        server_driver.receive_once_with(&mut handler),
        client_driver.receive_once(),
    );

    // Assert
    assert_eq!(result, Ok(()));
    assert_eq!(
        recorded
            .lock()
            .expect("fill recording should be available")
            .as_slice(),
        &[expected]
    );
    let Ok(ServerDriverEvent::Handled(handled)) = server_event else {
        panic!("server should handle the fill command");
    };
    assert_eq!(handled.call_id(), CallId::new(41));
    assert_eq!(handled.object_id(), PAGE_ID);
    assert_eq!(handled.method(), Fill::METHOD);
    assert!(matches!(client_event, Ok(DriverEvent::Delivered(_))));
}

#[tokio::test]
async fn dropping_public_fill_future_cancels_the_remote_runtime_call() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let (channel, mut client_driver) =
        EncodedClientChannel::new_with_control(protocol(), control_protocol(), client_transport, 1)
            .expect("matching control protocol should compose with locator commands");
    let observed_channel = channel.clone();
    let client = Client::new(channel, AtomicCallIdSource::new(51));
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
    let locator = page.get_by_label("Email", true);
    let cancellations = Arc::new(Mutex::new(Vec::new()));
    let service = PendingFillService {
        cancellations: Arc::clone(&cancellations),
    };
    let runtime = Arc::new(
        ControlledDispatcher::with_object_kinds(
            service.clone(),
            service,
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid"),
    );
    runtime
        .register_object(PAGE_ID, None, PageKind::OBJECT_KIND)
        .expect("runtime page should register");
    let command_handler = ConcurrentTypedCommandHandler::<Fill, _, _>::from_shared(
        Arc::new(protocol()),
        Arc::clone(&runtime),
    );
    let mut control_receiver =
        RuntimeControlReceiver::new(Arc::clone(&runtime), control_protocol(), 0);
    let mut supervisor = EncodedServerSupervisor::new(
        server_transport,
        ServerSupervisorLimits::new(1).expect("one command should be admitted"),
    );
    let mut server_events = Vec::new();
    let public_call = tokio::spawn(async move {
        locator
            .fill("never", FillOptions::new(), CallMetadata::default())
            .await
    });
    let mut server_run =
        Box::pin(
            supervisor.run(&command_handler, &mut control_receiver, |event| {
                server_events.push(event);
                Ok::<_, std::convert::Infallible>(())
            }),
        );
    tokio::select! {
        result = &mut server_run => panic!("server supervisor completed early: {}", result.is_ok()),
        () = tokio::task::yield_now() => {}
    }
    let mut drive = Box::pin(client_driver.drive_once());
    tokio::select! {
        result = &mut drive => panic!("client driver completed before cancellation: {}", result.is_ok()),
        () = tokio::task::yield_now() => {}
    }

    // Act
    public_call.abort();
    assert!(
        public_call
            .await
            .expect_err("aborted public call should drop its wire future")
            .is_cancelled()
    );
    let late = tokio::select! {
        event = &mut drive => event
            .expect("cancellation-aware driver should consume cancelled late result"),
        result = &mut server_run => panic!("server supervisor completed before client result: {}", result.is_ok()),
    };
    drop(server_run);

    // Assert
    assert!(matches!(
        server_events.as_slice(),
        [
            ServerSupervisorEvent::Control(RuntimeControlEvent::Cancellation {
                call_id: control_call,
                ..
            }),
            ServerSupervisorEvent::Command(handled),
        ] if *control_call == CallId::new(51) && handled.call_id() == CallId::new(51)
    ));
    assert!(matches!(
        late,
        DriverEvent::IgnoredAbandoned(call_id) if call_id == CallId::new(51)
    ));
    let cancellations = cancellations
        .lock()
        .expect("cancellations should be available");
    assert_eq!(cancellations.len(), 1);
    assert_eq!(cancellations[0].0.call_id(), CallId::new(51));
    assert_eq!(cancellations[0].0.object_id(), PAGE_ID);
    assert_eq!(cancellations[0].0.method(), Fill::METHOD);
    assert_eq!(
        cancellations[0].0.cancellation(),
        Some(pwright_core::CancellationId::new(51))
    );
    assert_eq!(cancellations[0].1, CancellationReason::Caller);
    assert_eq!(runtime.pending_call_count(), Ok(0));
    let server_metrics = supervisor.metrics();
    assert_eq!(server_metrics.active_commands(), 0);
    assert_eq!(server_metrics.peak_active_commands(), 1);
    assert_eq!(server_metrics.accepted_commands(), 1);
    assert_eq!(server_metrics.completed_commands(), 1);
    assert_eq!(server_metrics.handled_controls(), 1);
    assert_eq!(server_metrics.dropped_commands(), 0);
    let metrics = observed_channel
        .metrics()
        .expect("client metrics should be available");
    assert_eq!(metrics.awaiting_results(), 0);
    assert_eq!(metrics.abandoned_calls(), 0);
    assert_eq!(metrics.queued_cancellations(), 0);
    assert_eq!(metrics.sent_cancellations(), 1);
}

#[tokio::test]
async fn client_write_half_close_cancels_and_drains_a_pending_runtime_call() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let cancellations = Arc::new(Mutex::new(Vec::new()));
    let service = PendingFillService {
        cancellations: Arc::clone(&cancellations),
    };
    let runtime = Arc::new(
        ControlledDispatcher::with_object_kinds(
            service.clone(),
            service,
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid"),
    );
    runtime
        .register_object(PAGE_ID, None, PageKind::OBJECT_KIND)
        .expect("runtime page should register");
    let commands = ConcurrentTypedCommandHandler::<Fill, _, _>::from_shared(
        Arc::new(protocol()),
        Arc::clone(&runtime),
    );
    let mut controls = RuntimeControlReceiver::new(Arc::clone(&runtime), control_protocol(), 0);
    let limits = ServerSupervisorLimits::new(1)
        .expect("one command should be admitted")
        .with_eof_policy(ServerEofPolicy::DrainActive);
    let mut supervisor = EncodedServerSupervisor::new(server_transport, limits);
    let command = Command::<Fill>::new(
        CallId::new(61),
        PAGE_ID,
        CallMetadata::new(None, Some(pwright_core::CancellationId::new(61))),
        FillParams::new(Selector::label("Email", true), "never", FillOptions::new()),
    );
    client_transport
        .send(protocol().encode(&command).expect("command should encode"))
        .await
        .expect("command should reach the server");
    client_transport
        .shutdown()
        .await
        .expect("client write half should close");
    let mut events = Vec::new();

    // Act
    let closed = supervisor
        .run(&commands, &mut controls, |event| {
            events.push(event);
            Ok::<_, std::convert::Infallible>(())
        })
        .await;
    let response = client_transport
        .receive()
        .await
        .expect("client read half should remain available")
        .expect("transport-cancelled result should be drained");
    let result = protocol()
        .decode::<pwright_core::CommandResult<Fill>>(response)
        .expect("transport-cancelled result should decode");

    // Assert
    assert!(matches!(
        closed,
        Ok(pwright_server_wire::ServerSupervisorCloseReason::EndOfStream)
    ));
    assert!(matches!(
        events.as_slice(),
        [
            ServerSupervisorEvent::Control(RuntimeControlEvent::TransportClosed {
                already_closed: false,
                pending_calls: 1,
                invalidated_objects: 1,
            }),
            ServerSupervisorEvent::Command(handled),
        ] if handled.call_id() == CallId::new(61)
    ));
    assert_eq!(
        result
            .into_outcome()
            .expect_err("half-close should terminate pending work")
            .kind(),
        pwright_core::FailureKind::TransportClosed
    );
    let cancellations = cancellations
        .lock()
        .expect("cancellations should be available");
    assert!(
        cancellations.is_empty(),
        "transport loss drops backend work without sending cleanup over the dead owner transport"
    );
    assert_eq!(runtime.pending_call_count(), Ok(0));
    assert_eq!(runtime.live_object_count(), Ok(0));
    let metrics = supervisor.metrics();
    assert_eq!(metrics.completed_commands(), 1);
    assert_eq!(metrics.handled_controls(), 1);
    assert_eq!(metrics.dropped_commands(), 0);
}

#[tokio::test]
async fn server_rejects_payload_identity_that_does_not_match_the_outer_route() {
    // Arrange
    let (client_io, server_io) = duplex(4096);
    let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 4096));
    let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
    let server_transport = TokioStreamTransport::from_io(server_io, stream_codec);
    let encoded = protocol()
        .encode(&Command::<Click>::new(
            CallId::new(7),
            PAGE_ID,
            CallMetadata::default(),
            ClickParams::new(
                CssSelector::new("button").expect("valid CSS"),
                ClickOptions::new(),
            ),
        ))
        .expect("click should encode");
    let tampered = WireFrame::new(
        encoded.version(),
        FrameRoute::Command {
            call_id: CallId::new(8),
            target: PAGE_ID,
            method: Click::METHOD,
        },
        encoded.payload().to_vec(),
        Vec::new(),
        frame_limits(),
    )
    .expect("tampered frame should fit");
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runtime = DirectDispatcher::with_object_kinds(
        RecordingClickService {
            recorded: Arc::clone(&recorded),
        },
        RuntimeLimits::new(1, 1),
        BROWSER_OBJECT_KIND_CATALOG,
    )
    .expect("browser object catalog should be valid");
    runtime
        .register_object(PAGE_ID, None, PageKind::OBJECT_KIND)
        .expect("runtime page should register");
    let mut handler = TypedCommandHandler::<Click, _, _>::new(protocol(), runtime);
    let mut server_driver = EncodedServerDriver::new(server_transport);
    client_transport
        .send(tampered)
        .await
        .expect("tampered frame should reach the server");

    // Act
    let result = server_driver.receive_once_with(&mut handler).await;

    // Assert
    assert!(matches!(
        result,
        Err(ServerDriverError::Handler(
            TypedCommandHandlerError::DecodedRouteMismatch {
                outer_call,
                decoded_call,
                ..
            }
        )) if outer_call == CallId::new(8) && decoded_call == CallId::new(7)
    ));
    assert!(
        recorded
            .lock()
            .expect("click recording should be available")
            .is_empty()
    );
}
