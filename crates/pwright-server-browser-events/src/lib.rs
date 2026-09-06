//! One-shot, caller-driven publication of Browser terminal events.
//!
//! Unlike telemetry or repeatable lifecycle notifications, Browser
//! disconnection happens at most once. This publisher therefore owns no event
//! queue and starts no task. The caller places its single send inside the
//! ordered Browser-disposal barrier.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use pwright_cap_browser_events::{
    BROWSER_DISCONNECTED_EVENT_TAG, BrowserDisconnected, BrowserDisconnectedEvent,
};
use pwright_codec_browser_events::{BrowserEventsCodec, BrowserEventsCodecError};
use pwright_core::{Event, EventSequence, ObjectId, SequencedEvent};
use pwright_runtime::{ControlledLifecycleHandle, RuntimeStateError};
use pwright_runtime_control::{
    ControlMessageSource, OrderedControlGateError, OrderedRuntimeControl,
    OrderedRuntimeControlError,
};
use pwright_server_wire::{
    ServerControlFrameHandler, ServerFrameDisposition, ServerSupervisorCloseReason,
};
use pwright_transport::{FrameRoute, FrameTransport, WireCodec, WireProtocol, WireProtocolError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserDisconnectedPublisherState {
    Ready,
    Publishing,
    Published,
    Abandoned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrowserDisconnectedPublication {
    already_published: bool,
}

impl BrowserDisconnectedPublication {
    pub const fn already_published(self) -> bool {
        self.already_published
    }
}

#[derive(Debug)]
pub enum BrowserDisconnectedPublishError<Codec, Transport> {
    StateUnavailable,
    PublishInProgress,
    PreviousPublishAbandoned,
    Protocol(WireProtocolError<Codec>),
    EncodedRouteMismatch {
        expected: FrameRoute,
        actual: FrameRoute,
    },
    Transport(Transport),
}

impl<Codec, Transport> Display for BrowserDisconnectedPublishError<Codec, Transport>
where
    Codec: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::StateUnavailable => {
                formatter.write_str("Browser disconnected publication state is unavailable")
            }
            Self::PublishInProgress => {
                formatter.write_str("Browser disconnected publication is already in progress")
            }
            Self::PreviousPublishAbandoned => formatter.write_str(
                "Browser disconnected publication was abandoned and cannot be retried on this transport",
            ),
            Self::Protocol(error) => Display::fmt(error, formatter),
            Self::EncodedRouteMismatch { expected, actual } => write!(
                formatter,
                "Browser disconnected codec emitted route {actual:?}, expected {expected:?}"
            ),
            Self::Transport(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Codec, Transport> Error for BrowserDisconnectedPublishError<Codec, Transport>
where
    Codec: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Protocol(error) => Some(error),
            Self::Transport(error) => Some(error),
            Self::StateUnavailable
            | Self::PublishInProgress
            | Self::PreviousPublishAbandoned
            | Self::EncodedRouteMismatch { .. } => None,
        }
    }
}

struct PublicationClaim {
    state: Arc<Mutex<BrowserDisconnectedPublisherState>>,
    finished: bool,
}

impl PublicationClaim {
    fn commit(mut self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *state = BrowserDisconnectedPublisherState::Published;
        self.finished = true;
    }
}

impl Drop for PublicationClaim {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if *state == BrowserDisconnectedPublisherState::Publishing {
            *state = BrowserDisconnectedPublisherState::Abandoned;
        }
    }
}

/// Cloneable handle to one Browser's at-most-once disconnected event.
///
/// A polled send which is dropped or fails is terminal. Retrying after a
/// partial stream write could corrupt framing or duplicate the event.
pub struct BrowserDisconnectedPublisher<Codec, Transport> {
    browser: ObjectId,
    protocol: Arc<WireProtocol<Codec>>,
    transport: Transport,
    state: Arc<Mutex<BrowserDisconnectedPublisherState>>,
}

impl<Codec, Transport> Clone for BrowserDisconnectedPublisher<Codec, Transport>
where
    Transport: Clone,
{
    fn clone(&self) -> Self {
        Self {
            browser: self.browser,
            protocol: Arc::clone(&self.protocol),
            transport: self.transport.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

impl<Codec, Transport> BrowserDisconnectedPublisher<Codec, Transport> {
    pub fn new(browser: ObjectId, protocol: WireProtocol<Codec>, transport: Transport) -> Self {
        Self {
            browser,
            protocol: Arc::new(protocol),
            transport,
            state: Arc::new(Mutex::new(BrowserDisconnectedPublisherState::Ready)),
        }
    }

    pub const fn browser(&self) -> ObjectId {
        self.browser
    }

    pub fn protocol(&self) -> &WireProtocol<Codec> {
        self.protocol.as_ref()
    }

    pub const fn transport(&self) -> &Transport {
        &self.transport
    }

    pub fn publication_state(
        &self,
    ) -> Result<
        BrowserDisconnectedPublisherState,
        BrowserDisconnectedPublishError<Codec::Error, Transport::Error>,
    >
    where
        Codec: WireCodec<SequencedEvent<BrowserDisconnected>>,
        Transport: FrameTransport,
    {
        self.state
            .lock()
            .map(|state| *state)
            .map_err(|_| BrowserDisconnectedPublishError::StateUnavailable)
    }

    fn claim(
        &self,
    ) -> Result<
        Option<PublicationClaim>,
        BrowserDisconnectedPublishError<Codec::Error, Transport::Error>,
    >
    where
        Codec: WireCodec<SequencedEvent<BrowserDisconnected>>,
        Transport: FrameTransport,
    {
        let mut state = self
            .state
            .lock()
            .map_err(|_| BrowserDisconnectedPublishError::StateUnavailable)?;
        match *state {
            BrowserDisconnectedPublisherState::Ready => {
                *state = BrowserDisconnectedPublisherState::Publishing;
                Ok(Some(PublicationClaim {
                    state: Arc::clone(&self.state),
                    finished: false,
                }))
            }
            BrowserDisconnectedPublisherState::Published => Ok(None),
            BrowserDisconnectedPublisherState::Publishing => {
                Err(BrowserDisconnectedPublishError::PublishInProgress)
            }
            BrowserDisconnectedPublisherState::Abandoned => {
                Err(BrowserDisconnectedPublishError::PreviousPublishAbandoned)
            }
        }
    }
}

impl<Codec, Transport> BrowserDisconnectedPublisher<Codec, Transport>
where
    Codec: WireCodec<SequencedEvent<BrowserDisconnected>>,
    Transport: FrameTransport,
{
    pub async fn publish(
        &self,
    ) -> Result<
        BrowserDisconnectedPublication,
        BrowserDisconnectedPublishError<Codec::Error, Transport::Error>,
    > {
        let Some(claim) = self.claim()? else {
            return Ok(BrowserDisconnectedPublication {
                already_published: true,
            });
        };
        let event = SequencedEvent::<BrowserDisconnected>::new(
            EventSequence::FIRST,
            Event::new(self.browser, BrowserDisconnectedEvent),
        );
        let expected = FrameRoute::Event {
            source: self.browser,
            event: BROWSER_DISCONNECTED_EVENT_TAG,
        };
        let frame = self
            .protocol
            .encode(&event)
            .map_err(BrowserDisconnectedPublishError::Protocol)?;
        if frame.route() != expected {
            return Err(BrowserDisconnectedPublishError::EncodedRouteMismatch {
                expected,
                actual: frame.route(),
            });
        }
        self.transport
            .send(frame)
            .await
            .map_err(BrowserDisconnectedPublishError::Transport)?;
        claim.commit();
        Ok(BrowserDisconnectedPublication {
            already_published: false,
        })
    }
}

/// Shared proof that the native browser transport, rather than merely the
/// Pwright client connection, has closed.
#[derive(Clone, Default)]
pub struct BrowserNativeDisconnectSignal {
    closed: Arc<AtomicBool>,
}

impl BrowserNativeDisconnectSignal {
    pub fn mark_closed(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

#[derive(Debug)]
pub enum BrowserDisconnectedTerminalError<ControlCodec, Transport> {
    MissingBrowserDisposal(ObjectId),
    Runtime(RuntimeStateError),
    Coordination(OrderedControlGateError),
    Control(OrderedRuntimeControlError<ControlCodec, Transport>),
    Publish(BrowserDisconnectedPublishError<BrowserEventsCodecError, Transport>),
}

impl<ControlCodec, Transport> Display for BrowserDisconnectedTerminalError<ControlCodec, Transport>
where
    ControlCodec: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBrowserDisposal(browser) => write!(
                formatter,
                "Browser `{browser}` was already stale before its disconnected event was published"
            ),
            Self::Runtime(error) => Display::fmt(error, formatter),
            Self::Coordination(error) => Display::fmt(error, formatter),
            Self::Control(error) => Display::fmt(error, formatter),
            Self::Publish(error) => Display::fmt(error, formatter),
        }
    }
}

impl<ControlCodec, Transport> Error for BrowserDisconnectedTerminalError<ControlCodec, Transport>
where
    ControlCodec: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Runtime(error) => Some(error),
            Self::Coordination(error) => Some(error),
            Self::Control(error) => Some(error),
            Self::Publish(error) => Some(error),
            Self::MissingBrowserDisposal(_) => None,
        }
    }
}

#[derive(Debug)]
pub enum BrowserDisconnectedControlError<Inner, ControlCodec, Transport> {
    Inner(Inner),
    Terminal(BrowserDisconnectedTerminalError<ControlCodec, Transport>),
    TerminalAndInner {
        terminal: BrowserDisconnectedTerminalError<ControlCodec, Transport>,
        inner: Inner,
    },
}

impl<Inner, ControlCodec, Transport> Display
    for BrowserDisconnectedControlError<Inner, ControlCodec, Transport>
where
    Inner: Display,
    ControlCodec: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inner(error) => Display::fmt(error, formatter),
            Self::Terminal(error) => Display::fmt(error, formatter),
            Self::TerminalAndInner { terminal, inner } => write!(
                formatter,
                "Browser disconnected publication failed ({terminal}); connection cleanup also failed: {inner}"
            ),
        }
    }
}

impl<Inner, ControlCodec, Transport> Error
    for BrowserDisconnectedControlError<Inner, ControlCodec, Transport>
where
    Inner: Error + 'static,
    ControlCodec: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Inner(error) => Some(error),
            Self::Terminal(error) => Some(error),
            Self::TerminalAndInner { terminal, .. } => Some(terminal),
        }
    }
}

/// Decorates the supervisor's existing connection-termination barrier with
/// Playwright-ordered Browser disposal after a proven native disconnect.
pub struct BrowserDisconnectedControlHandler<Inner, Source, ControlCodec, Transport> {
    inner: Inner,
    signal: BrowserNativeDisconnectSignal,
    lifecycle: ControlledLifecycleHandle,
    browser: ObjectId,
    control: OrderedRuntimeControl<Source, ControlCodec, Arc<Transport>>,
    disconnected: BrowserDisconnectedPublisher<BrowserEventsCodec, Arc<Transport>>,
}

impl<Inner, Source, ControlCodec, Transport>
    BrowserDisconnectedControlHandler<Inner, Source, ControlCodec, Transport>
{
    pub const fn new(
        inner: Inner,
        signal: BrowserNativeDisconnectSignal,
        lifecycle: ControlledLifecycleHandle,
        browser: ObjectId,
        control: OrderedRuntimeControl<Source, ControlCodec, Arc<Transport>>,
        disconnected: BrowserDisconnectedPublisher<BrowserEventsCodec, Arc<Transport>>,
    ) -> Self {
        Self {
            inner,
            signal,
            lifecycle,
            browser,
            control,
            disconnected,
        }
    }

    pub const fn inner(&self) -> &Inner {
        &self.inner
    }

    async fn publish_native_disconnect(
        &self,
    ) -> Result<
        (),
        BrowserDisconnectedTerminalError<
            <ControlCodec as WireCodec<pwright_core::ControlMessage>>::Error,
            <Transport as FrameTransport>::Error,
        >,
    >
    where
        Source: ControlMessageSource,
        ControlCodec: WireCodec<pwright_core::ControlMessage>,
        Transport: FrameTransport,
    {
        match self
            .disconnected
            .publication_state()
            .map_err(BrowserDisconnectedTerminalError::Publish)?
        {
            BrowserDisconnectedPublisherState::Published
            | BrowserDisconnectedPublisherState::Abandoned => return Ok(()),
            BrowserDisconnectedPublisherState::Publishing => {
                return self
                    .disconnected
                    .publish()
                    .await
                    .map(|_| ())
                    .map_err(BrowserDisconnectedTerminalError::Publish);
            }
            BrowserDisconnectedPublisherState::Ready => {}
        }
        if !self
            .lifecycle
            .is_object_live(self.browser)
            .map_err(BrowserDisconnectedTerminalError::Runtime)?
        {
            return Err(BrowserDisconnectedTerminalError::MissingBrowserDisposal(
                self.browser,
            ));
        }
        let barrier = self
            .control
            .begin_barrier()
            .await
            .map_err(BrowserDisconnectedTerminalError::Coordination)?;
        let prepared = self
            .lifecycle
            .prepare_object_disposal(self.browser)
            .map_err(BrowserDisconnectedTerminalError::Runtime)?;
        prepared
            .commit()
            .map_err(BrowserDisconnectedTerminalError::Runtime)?;
        barrier
            .send_before_disposal(self.browser)
            .await
            .map_err(BrowserDisconnectedTerminalError::Control)?;
        let publication = self
            .disconnected
            .publish()
            .await
            .map(|_| ())
            .map_err(BrowserDisconnectedTerminalError::Publish);
        let disposal = barrier
            .send_through_disposal(self.browser)
            .await
            .map(|_| ())
            .map_err(BrowserDisconnectedTerminalError::Control);
        match (publication, disposal) {
            (_, Err(error)) => Err(error),
            (Err(error), Ok(())) => Err(error),
            (Ok(()), Ok(())) => Ok(()),
        }
    }
}

impl<Inner, Source, ControlCodec, Transport> ServerControlFrameHandler
    for BrowserDisconnectedControlHandler<Inner, Source, ControlCodec, Transport>
where
    Inner: ServerControlFrameHandler,
    Source: ControlMessageSource,
    ControlCodec: WireCodec<pwright_core::ControlMessage>,
    Transport: FrameTransport,
{
    type Event = Inner::Event;
    type Error =
        BrowserDisconnectedControlError<Inner::Error, ControlCodec::Error, Transport::Error>;
    type Future<'a>
        =
        Pin<Box<dyn Future<Output = Result<ServerFrameDisposition<Self::Event>, Self::Error>> + 'a>>
    where
        Self: 'a;
    type CloseFuture<'a>
        = Pin<Box<dyn Future<Output = Result<Option<Self::Event>, Self::Error>> + 'a>>
    where
        Self: 'a;

    fn handle_control<'a>(&'a mut self, frame: pwright_transport::WireFrame) -> Self::Future<'a> {
        Box::pin(async move {
            self.inner
                .handle_control(frame)
                .await
                .map_err(BrowserDisconnectedControlError::Inner)
        })
    }

    fn releases_frame_before_pending(&self) -> bool {
        self.inner.releases_frame_before_pending()
    }

    fn terminal_close_reason(&self) -> Option<ServerSupervisorCloseReason> {
        self.inner.terminal_close_reason()
    }

    fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
        Box::pin(async move {
            self.inner
                .handle_transport_closed()
                .await
                .map_err(BrowserDisconnectedControlError::Inner)
        })
    }

    fn handle_connection_terminated<'a>(
        &'a mut self,
        reason: ServerSupervisorCloseReason,
    ) -> Self::CloseFuture<'a> {
        Box::pin(async move {
            let terminal = if self.signal.is_closed() {
                self.publish_native_disconnect().await.err()
            } else {
                None
            };
            let inner = self.inner.handle_connection_terminated(reason).await;
            match (terminal, inner) {
                (None, Ok(event)) => Ok(event),
                (None, Err(inner)) => Err(BrowserDisconnectedControlError::Inner(inner)),
                (Some(terminal), Ok(_)) => Err(BrowserDisconnectedControlError::Terminal(terminal)),
                (Some(terminal), Err(inner)) => {
                    Err(BrowserDisconnectedControlError::TerminalAndInner { terminal, inner })
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Pending, Ready, pending, ready};
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    use pwright_cap_browser_events::BrowserDisconnected;
    use pwright_codec_browser_events::BrowserEventsCodec;
    use pwright_codec_control::ControlCodec;
    use pwright_core::{ControlMessage, ObjectId, ObjectKindTag, ProtocolVersion, SequencedEvent};
    use pwright_runtime::{ControlledDispatcher, NoDeadlineScheduler, RuntimeLimits};
    use pwright_runtime_control::{OrderedRuntimeControl, RuntimeControlDriver};
    use pwright_server_wire::{
        ServerControlFrameHandler, ServerFrameDisposition, ServerSupervisorCloseReason,
    };
    use pwright_transport::{FrameLimits, FrameRoute, FrameTransport, WireFrame, WireProtocol};
    use pwright_transport_memory::MemoryFrameTransport;

    use super::{
        BrowserDisconnectedControlHandler, BrowserDisconnectedPublishError,
        BrowserDisconnectedPublisher, BrowserNativeDisconnectSignal,
    };

    struct RecordingControl {
        reasons: Arc<Mutex<Vec<ServerSupervisorCloseReason>>>,
    }

    impl ServerControlFrameHandler for RecordingControl {
        type Event = ();
        type Error = Infallible;
        type Future<'a> = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>;
        type CloseFuture<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

        fn handle_control<'a>(&'a mut self, frame: WireFrame) -> Self::Future<'a> {
            ready(Ok(ServerFrameDisposition::Unhandled(frame)))
        }

        fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
            ready(Ok(None))
        }

        fn handle_connection_terminated<'a>(
            &'a mut self,
            reason: ServerSupervisorCloseReason,
        ) -> Self::CloseFuture<'a> {
            self.reasons.lock().unwrap().push(reason);
            ready(Ok(None))
        }
    }

    struct BlockingTransport;

    impl FrameTransport for BlockingTransport {
        type Error = Infallible;
        type Send<'a>
            = Pending<Result<(), Infallible>>
        where
            Self: 'a;
        type Receive<'a>
            = Ready<Result<Option<WireFrame>, Infallible>>
        where
            Self: 'a;

        fn send(&self, _frame: WireFrame) -> Self::Send<'_> {
            pending()
        }

        fn receive(&self) -> Self::Receive<'_> {
            ready(Ok(None))
        }
    }

    fn poll_once<F>(future: Pin<&mut F>) -> Poll<F::Output>
    where
        F: std::future::Future,
    {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        future.poll(&mut context)
    }

    fn protocol() -> WireProtocol<BrowserEventsCodec> {
        WireProtocol::new_exact_directional(
            BrowserEventsCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(17, 0, 0, 17),
            FrameLimits::new(17, 0, 0, 17),
        )
    }

    fn control_protocol() -> WireProtocol<ControlCodec> {
        WireProtocol::new_exact_directional(
            ControlCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
            FrameLimits::new(64, 0, 0, 64),
        )
    }

    #[tokio::test]
    async fn publisher_sends_the_browser_terminal_event_exactly_once() {
        // Arrange
        let (server, client) = MemoryFrameTransport::pair(1);
        let publisher =
            BrowserDisconnectedPublisher::new(ObjectId::new(9), protocol(), Arc::new(server));

        // Act
        let first = publisher
            .publish()
            .await
            .expect("first publish should send");
        let second = publisher
            .publish()
            .await
            .expect("repeated publish should be idempotent");
        let frame = client
            .receive()
            .await
            .expect("transport should remain available")
            .expect("one event should arrive");
        let decoded = protocol()
            .decode::<SequencedEvent<BrowserDisconnected>>(frame)
            .expect("event should decode");

        // Assert
        assert!(!first.already_published());
        assert!(second.already_published());
        assert_eq!(decoded.object_id(), ObjectId::new(9));
        assert_eq!(client.metrics().unwrap().incoming_depth(), 0);
    }

    #[tokio::test]
    async fn dropped_polled_publish_poisoned_the_one_shot_instead_of_retrying() {
        // Arrange
        let publisher =
            BrowserDisconnectedPublisher::new(ObjectId::new(9), protocol(), BlockingTransport);
        let mut first = Box::pin(publisher.publish());
        assert!(poll_once(first.as_mut()).is_pending());

        // Act
        drop(first);
        let second = publisher.publish().await;

        // Assert
        assert!(matches!(
            second,
            Err(BrowserDisconnectedPublishError::PreviousPublishAbandoned)
        ));
    }

    #[tokio::test]
    async fn native_disconnect_cleanup_orders_descendants_event_and_browser_dispose() {
        // Arrange
        let runtime = Arc::new(ControlledDispatcher::new(
            (),
            (),
            NoDeadlineScheduler,
            RuntimeLimits::new(3, 1).with_control_messages(6),
        ));
        runtime
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .unwrap();
        runtime
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .unwrap();
        runtime
            .register_object(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                ObjectKindTag::new(3),
            )
            .unwrap();
        for _ in 0..3 {
            runtime.pop_control_message().unwrap().unwrap();
        }
        let (server_transport, client_transport) = MemoryFrameTransport::pair(8);
        let server_transport = Arc::new(server_transport);
        let control = OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(control_protocol(), Arc::clone(&server_transport)),
            1,
        )
        .unwrap();
        let publisher = BrowserDisconnectedPublisher::new(
            ObjectId::new(1),
            protocol(),
            Arc::clone(&server_transport),
        );
        let signal = BrowserNativeDisconnectSignal::default();
        signal.mark_closed();
        let reasons = Arc::new(Mutex::new(Vec::new()));
        let mut handler = BrowserDisconnectedControlHandler::new(
            RecordingControl {
                reasons: Arc::clone(&reasons),
            },
            signal,
            runtime.lifecycle_handle(),
            ObjectId::new(1),
            control,
            publisher,
        );

        // Act
        let cleanup = handler
            .handle_connection_terminated(ServerSupervisorCloseReason::BackendEventFailed)
            .await;
        let mut frames = Vec::new();
        for _ in 0..4 {
            frames.push(client_transport.receive().await.unwrap().unwrap());
        }

        // Assert
        assert!(cleanup.is_ok());
        assert_eq!(runtime.live_object_count(), Ok(0));
        assert_eq!(frames[0].route(), FrameRoute::Control);
        assert_eq!(frames[1].route(), FrameRoute::Control);
        assert_eq!(
            frames[2].route(),
            FrameRoute::Event {
                source: ObjectId::new(1),
                event: <BrowserDisconnected as pwright_core::EventSpec>::EVENT,
            }
        );
        assert_eq!(frames[3].route(), FrameRoute::Control);
        let first = control_protocol()
            .decode::<ControlMessage>(frames.remove(0))
            .unwrap();
        let second = control_protocol()
            .decode::<ControlMessage>(frames.remove(0))
            .unwrap();
        let _event = protocol()
            .decode::<SequencedEvent<BrowserDisconnected>>(frames.remove(0))
            .unwrap();
        let last = control_protocol()
            .decode::<ControlMessage>(frames.remove(0))
            .unwrap();
        assert_eq!(
            [first, second, last],
            [
                ControlMessage::Dispose {
                    object_id: ObjectId::new(3),
                },
                ControlMessage::Dispose {
                    object_id: ObjectId::new(2),
                },
                ControlMessage::Dispose {
                    object_id: ObjectId::new(1),
                },
            ]
        );
        assert_eq!(
            *reasons.lock().unwrap(),
            [ServerSupervisorCloseReason::BackendEventFailed]
        );
    }
}
