use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use pwright_browser_model::{
    BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG,
    ELEMENT_HANDLE_OBJECT_KIND, FRAME_OBJECT_KIND, PAGE_OBJECT_KIND,
};
use pwright_cap_handle_evaluation::{
    EvaluateHandle, EvaluateHandleParams, EvaluationValue, EvaluationValueLimits,
    GetPropertiesOptions, HandleEvaluate, HandleEvaluateHandle, HandleGetProperties,
    HandleGetProperty, HandleInitializer, HandleJsonValue, HandlePropertiesResult,
    HandlePropertyInitializer, HandleValueResult,
};
use pwright_cap_handle_lifecycle::HandleDispose;
use pwright_codec_control::ControlCodec;
use pwright_codec_handle_evaluation::{HandleEvaluationCodec, HandleEvaluationCodecLimits};
use pwright_core::{
    CallId, CallMetadata, Command, CommandResult, ControlMessage, ObjectId, ProtocolVersion,
    Service,
};
use pwright_plugin_ordered_handle_delivery::{
    ContinuableHandleCreationService, OrderedHandleCreationService, OrderedHandleDisposeService,
    OrderedHandleEvaluationHandler,
};
use pwright_runtime::{CleanupQueue, DirectDispatcher, ObjectTopologyMode, RuntimeLimits};
use pwright_runtime_control::{OrderedRuntimeControl, RuntimeControlDriver};
use pwright_server_wire::{ConcurrentServerFrameHandler, ServerFrameDisposition};
use pwright_transport::{
    FrameLimits, FrameRoute, FrameTransport, VersionPolicy, WireFrame, WireProtocol,
};

type TestRuntime = DirectDispatcher<()>;

fn runtime() -> Arc<TestRuntime> {
    let runtime = Arc::new(
        DirectDispatcher::with_object_kinds_and_topology(
            (),
            RuntimeLimits::new(16, 4).with_control_messages(32),
            BROWSER_OBJECT_KIND_CATALOG,
            ObjectTopologyMode::Complete,
        )
        .expect("test runtime should build"),
    );
    for (object_id, parent, kind) in [
        (ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag),
        (
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            BROWSER_CONTEXT_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(3),
            Some(ObjectId::new(2)),
            PAGE_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(4),
            Some(ObjectId::new(3)),
            FRAME_OBJECT_KIND.tag,
        ),
    ] {
        runtime
            .register_object(object_id, parent, kind)
            .expect("test object should register");
    }
    while runtime
        .pop_control_message()
        .expect("control queue should be available")
        .is_some()
    {}
    runtime
}

fn control_protocol() -> WireProtocol<ControlCodec> {
    WireProtocol::new(
        ControlCodec,
        VersionPolicy::exact(ProtocolVersion::new(1, 0)),
        ProtocolVersion::new(1, 0),
        FrameLimits::new(128, 0, 0, 128),
    )
    .expect("control protocol should build")
}

fn evaluation_protocol() -> WireProtocol<HandleEvaluationCodec> {
    let values =
        EvaluationValueLimits::new(8, 64, 64, 2048, 2048).expect("value limits should validate");
    WireProtocol::new_exact_directional(
        HandleEvaluationCodec::new(
            HandleEvaluationCodecLimits::new(2048, 1024, values)
                .expect("codec limits should validate"),
        ),
        ProtocolVersion::new(1, 2),
        FrameLimits::new(32 * 1024, 0, 0, 32 * 1024),
        FrameLimits::new(32 * 1024, 0, 0, 32 * 1024),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TestTransportError {
    ControlRejected,
}

impl Display for TestTransportError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("control frame rejected")
    }
}

impl Error for TestTransportError {}

struct TestSendFuture {
    frame: Option<WireFrame>,
    delivered: Arc<Mutex<Vec<WireFrame>>>,
    result: Option<Result<(), TestTransportError>>,
}

impl Future for TestSendFuture {
    type Output = Result<(), TestTransportError>;

    fn poll(mut self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(result) = self.result.take() else {
            return Poll::Pending;
        };
        if result.is_ok()
            && let Some(frame) = self.frame.take()
        {
            self.delivered
                .lock()
                .expect("delivered frames should be available")
                .push(frame);
        }
        Poll::Ready(result)
    }
}

struct TestTransport {
    attempted: Mutex<Vec<FrameRoute>>,
    delivered: Arc<Mutex<Vec<WireFrame>>>,
    pending_results: bool,
    reject_control: bool,
}

impl TestTransport {
    fn ready() -> Self {
        Self {
            attempted: Mutex::new(Vec::new()),
            delivered: Arc::new(Mutex::new(Vec::new())),
            pending_results: false,
            reject_control: false,
        }
    }

    fn pending_results() -> Self {
        Self {
            pending_results: true,
            ..Self::ready()
        }
    }

    fn rejecting_control() -> Self {
        Self {
            reject_control: true,
            ..Self::ready()
        }
    }
}

impl FrameTransport for TestTransport {
    type Error = TestTransportError;
    type Send<'a> = TestSendFuture;
    type Receive<'a> = Ready<Result<Option<WireFrame>, Self::Error>>;

    fn send(&self, frame: WireFrame) -> Self::Send<'_> {
        let route = frame.route();
        self.attempted
            .lock()
            .expect("attempts should be available")
            .push(route);
        let result = if self.pending_results && matches!(route, FrameRoute::Result { .. }) {
            None
        } else if self.reject_control && route == FrameRoute::Control {
            Some(Err(TestTransportError::ControlRejected))
        } else {
            Some(Ok(()))
        };
        TestSendFuture {
            frame: Some(frame),
            delivered: Arc::clone(&self.delivered),
            result,
        }
    }

    fn receive(&self) -> Self::Receive<'_> {
        ready(Ok(None))
    }
}

struct CreatingService {
    runtime: Arc<TestRuntime>,
    object_id: ObjectId,
}

impl Service<Command<EvaluateHandle>> for CreatingService {
    type Response = CommandResult<EvaluateHandle>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<EvaluateHandle>) -> Self::Future<'_> {
        self.runtime
            .register_object(
                self.object_id,
                Some(command.object_id()),
                ELEMENT_HANDLE_OBJECT_KIND.tag,
            )
            .expect("created Handle should publish locally");
        ready(Ok(CommandResult::new(
            command.call_id(),
            Ok(HandleInitializer::new(
                self.object_id,
                command.object_id(),
                ELEMENT_HANDLE_OBJECT_KIND.tag,
                "body",
            )),
        )))
    }
}

struct OperationsService {
    runtime: Arc<TestRuntime>,
}

macro_rules! value_service {
    ($spec:ty) => {
        impl Service<Command<$spec>> for OperationsService {
            type Response = CommandResult<$spec>;
            type Error = Infallible;
            type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

            fn call(&self, command: Command<$spec>) -> Self::Future<'_> {
                ready(Ok(CommandResult::new(
                    command.call_id(),
                    Ok(HandleValueResult::new(EvaluationValue::Null)),
                )))
            }
        }
    };
}

value_service!(HandleEvaluate);
value_service!(HandleJsonValue);

macro_rules! single_handle_service {
    ($spec:ty) => {
        impl Service<Command<$spec>> for OperationsService {
            type Response = CommandResult<$spec>;
            type Error = Infallible;
            type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

            fn call(&self, command: Command<$spec>) -> Self::Future<'_> {
                ready(Ok(CommandResult::new(
                    command.call_id(),
                    Ok(HandleInitializer::new(
                        ObjectId::new(6),
                        command.object_id(),
                        ELEMENT_HANDLE_OBJECT_KIND.tag,
                        "child",
                    )),
                )))
            }
        }
    };
}

single_handle_service!(HandleEvaluateHandle);
single_handle_service!(HandleGetProperty);

impl Service<Command<HandleGetProperties>> for OperationsService {
    type Response = CommandResult<HandleGetProperties>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<HandleGetProperties>) -> Self::Future<'_> {
        for object_id in [ObjectId::new(6), ObjectId::new(7)] {
            self.runtime
                .register_object(
                    object_id,
                    Some(ObjectId::new(4)),
                    ELEMENT_HANDLE_OBJECT_KIND.tag,
                )
                .expect("property Handle should register");
        }
        ready(Ok(CommandResult::new(
            command.call_id(),
            Ok(HandlePropertiesResult::new(vec![
                HandlePropertyInitializer::new(
                    "first",
                    HandleInitializer::new(
                        ObjectId::new(6),
                        ObjectId::new(4),
                        ELEMENT_HANDLE_OBJECT_KIND.tag,
                        "first",
                    ),
                ),
                HandlePropertyInitializer::new(
                    "second",
                    HandleInitializer::new(
                        ObjectId::new(7),
                        ObjectId::new(4),
                        ELEMENT_HANDLE_OBJECT_KIND.tag,
                        "second",
                    ),
                ),
            ])),
        )))
    }
}

struct DisposingService {
    runtime: Arc<TestRuntime>,
    calls: Arc<AtomicUsize>,
}

impl Service<Command<HandleDispose>> for DisposingService {
    type Response = CommandResult<HandleDispose>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<HandleDispose>) -> Self::Future<'_> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.runtime
            .dispose_object(command.object_id())
            .expect("Handle should dispose locally");
        ready(Ok(CommandResult::new(command.call_id(), Ok(()))))
    }
}

fn poll_now<F>(future: Pin<&mut F>) -> Poll<F::Output>
where
    F: Future + ?Sized,
{
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    future.poll(&mut context)
}

#[tokio::test]
async fn remote_creation_sends_create_before_result() {
    // Arrange
    let runtime = runtime();
    let transport = Arc::new(TestTransport::ready());
    let calls = Arc::new(AtomicUsize::new(0));
    let handler = OrderedHandleEvaluationHandler::new(
        Arc::new(evaluation_protocol()),
        Arc::new(CreatingService {
            runtime: Arc::clone(&runtime),
            object_id: ObjectId::new(5),
        }),
        Arc::new(OperationsService {
            runtime: Arc::clone(&runtime),
        }),
        Arc::new(DisposingService {
            runtime: Arc::clone(&runtime),
            calls: Arc::clone(&calls),
        }),
        CleanupQueue::new(1),
        OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(control_protocol(), Arc::clone(&transport)),
            1,
        )
        .expect("ordered control should build"),
    );
    let command = Command::<EvaluateHandle>::new(
        CallId::new(41),
        ObjectId::new(4),
        CallMetadata::default(),
        EvaluateHandleParams::expression("document.body"),
    );

    // Act
    let outcome = handler
        .handle_concurrently(
            evaluation_protocol()
                .encode(&command)
                .expect("command should encode"),
            Arc::clone(&transport),
        )
        .await
        .expect("remote handler should complete");

    // Assert
    assert!(matches!(outcome, ServerFrameDisposition::Handled(_)));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        *transport.attempted.lock().expect("attempts should load"),
        [
            FrameRoute::Control,
            FrameRoute::Result {
                call_id: CallId::new(41),
            },
        ]
    );
}

#[tokio::test]
async fn failed_create_send_compensates_native_handle() {
    // Arrange
    let runtime = runtime();
    let transport = Arc::new(TestTransport::rejecting_control());
    let calls = Arc::new(AtomicUsize::new(0));
    let dispose = Arc::new(DisposingService {
        runtime: Arc::clone(&runtime),
        calls: Arc::clone(&calls),
    });
    let service = OrderedHandleCreationService::new(
        Arc::new(CreatingService {
            runtime: Arc::clone(&runtime),
            object_id: ObjectId::new(5),
        }),
        dispose,
        OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(control_protocol(), transport),
            1,
        )
        .expect("ordered control should build"),
    );

    // Act
    let result = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(41),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("document.body"),
        ))
        .await
        .expect("service is infallible");

    // Assert
    assert!(result.into_outcome().is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(runtime.is_object_live(ObjectId::new(5)), Ok(false));
}

#[test]
fn dropped_result_send_uses_reserved_ordered_disposal() {
    // Arrange
    let runtime = runtime();
    let transport = Arc::new(TestTransport::pending_results());
    let calls = Arc::new(AtomicUsize::new(0));
    let continuation = CleanupQueue::new(1);
    let handler = OrderedHandleEvaluationHandler::new(
        Arc::new(evaluation_protocol()),
        Arc::new(CreatingService {
            runtime: Arc::clone(&runtime),
            object_id: ObjectId::new(5),
        }),
        Arc::new(OperationsService {
            runtime: Arc::clone(&runtime),
        }),
        Arc::new(DisposingService {
            runtime: Arc::clone(&runtime),
            calls: Arc::clone(&calls),
        }),
        continuation.clone(),
        OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(control_protocol(), Arc::clone(&transport)),
            1,
        )
        .expect("ordered control should build"),
    );
    let command = Command::<EvaluateHandle>::new(
        CallId::new(41),
        ObjectId::new(4),
        CallMetadata::default(),
        EvaluateHandleParams::expression("document.body"),
    );
    let mut delivery = Box::pin(
        handler.handle_concurrently(
            evaluation_protocol()
                .encode(&command)
                .expect("command should encode"),
            Arc::clone(&transport),
        ),
    );

    // Act
    assert!(poll_now(delivery.as_mut()).is_pending());
    drop(delivery);
    let mut cleanup = continuation
        .try_take()
        .expect("cleanup queue should load")
        .expect("abandoned result should submit cleanup");
    assert!(matches!(poll_now(cleanup.as_mut()), Poll::Ready(Ok(()))));

    // Assert
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(runtime.is_object_live(ObjectId::new(5)), Ok(false));
    assert_eq!(
        *transport.attempted.lock().expect("attempts should load"),
        [
            FrameRoute::Control,
            FrameRoute::Result {
                call_id: CallId::new(41),
            },
            FrameRoute::Control,
        ]
    );
}

#[test]
fn dropped_property_batch_disposes_every_handle_after_create_fences() {
    // Arrange
    let runtime = runtime();
    runtime
        .register_object(
            ObjectId::new(5),
            Some(ObjectId::new(4)),
            ELEMENT_HANDLE_OBJECT_KIND.tag,
        )
        .expect("target Handle should register");
    while runtime
        .pop_control_message()
        .expect("control queue should load")
        .is_some()
    {}
    let transport = Arc::new(TestTransport::pending_results());
    let calls = Arc::new(AtomicUsize::new(0));
    let continuation = CleanupQueue::new(1);
    let handler = OrderedHandleEvaluationHandler::new(
        Arc::new(evaluation_protocol()),
        Arc::new(CreatingService {
            runtime: Arc::clone(&runtime),
            object_id: ObjectId::new(8),
        }),
        Arc::new(OperationsService {
            runtime: Arc::clone(&runtime),
        }),
        Arc::new(DisposingService {
            runtime: Arc::clone(&runtime),
            calls: Arc::clone(&calls),
        }),
        continuation.clone(),
        OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(control_protocol(), Arc::clone(&transport)),
            1,
        )
        .expect("ordered control should build"),
    );
    let command = Command::<HandleGetProperties>::new(
        CallId::new(43),
        ObjectId::new(5),
        CallMetadata::default(),
        GetPropertiesOptions,
    );
    let mut delivery = Box::pin(
        handler.handle_concurrently(
            evaluation_protocol()
                .encode(&command)
                .expect("command should encode"),
            Arc::clone(&transport),
        ),
    );

    // Act
    assert!(poll_now(delivery.as_mut()).is_pending());
    drop(delivery);
    let mut cleanup = continuation
        .try_take()
        .expect("cleanup queue should load")
        .expect("abandoned batch should submit cleanup");
    assert!(matches!(poll_now(cleanup.as_mut()), Poll::Ready(Ok(()))));

    // Assert
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(runtime.is_object_live(ObjectId::new(6)), Ok(false));
    assert_eq!(runtime.is_object_live(ObjectId::new(7)), Ok(false));
    assert_eq!(
        *transport.attempted.lock().expect("attempts should load"),
        [
            FrameRoute::Control,
            FrameRoute::Control,
            FrameRoute::Result {
                call_id: CallId::new(43),
            },
            FrameRoute::Control,
            FrameRoute::Control,
        ]
    );
}

#[tokio::test]
async fn unavailable_continuation_rejects_before_handle_creation() {
    // Arrange
    let runtime = runtime();
    let calls = Arc::new(AtomicUsize::new(0));
    let service = ContinuableHandleCreationService::new(
        Arc::new(CreatingService {
            runtime: Arc::clone(&runtime),
            object_id: ObjectId::new(5),
        }),
        Arc::new(DisposingService {
            runtime: Arc::clone(&runtime),
            calls: Arc::clone(&calls),
        }),
        CleanupQueue::new(0),
    );

    // Act
    let result = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(44),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("document.body"),
        ))
        .await
        .expect("service is infallible");

    // Assert
    assert!(result.into_outcome().is_err());
    assert_eq!(runtime.is_object_live(ObjectId::new(5)), Ok(false));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn explicit_dispose_waits_for_dispose_control_frame() {
    // Arrange
    let runtime = runtime();
    runtime
        .register_object(
            ObjectId::new(5),
            Some(ObjectId::new(4)),
            ELEMENT_HANDLE_OBJECT_KIND.tag,
        )
        .expect("Handle should register");
    while runtime
        .pop_control_message()
        .expect("control queue should load")
        .is_some()
    {}
    let transport = Arc::new(TestTransport::ready());
    let calls = Arc::new(AtomicUsize::new(0));
    let service = OrderedHandleDisposeService::new(
        Arc::new(DisposingService {
            runtime: Arc::clone(&runtime),
            calls: Arc::clone(&calls),
        }),
        OrderedRuntimeControl::new(
            runtime,
            RuntimeControlDriver::new(control_protocol(), Arc::clone(&transport)),
            1,
        )
        .expect("ordered control should build"),
    );

    // Act
    let result = service
        .call(Command::<HandleDispose>::new(
            CallId::new(42),
            ObjectId::new(5),
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("service is infallible");

    // Assert
    assert_eq!(result.into_outcome(), Ok(()));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let delivered = transport
        .delivered
        .lock()
        .expect("delivered frames should load");
    assert_eq!(delivered.len(), 1);
    assert_eq!(
        control_protocol()
            .decode::<ControlMessage>(delivered[0].clone())
            .expect("control frame should decode"),
        ControlMessage::Dispose {
            object_id: ObjectId::new(5),
        }
    );
}
