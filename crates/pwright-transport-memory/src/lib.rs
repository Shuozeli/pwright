//! Bounded, executor-neutral in-memory transport for encoded Pwright frames.
//!
//! This crate is a real `FrameTransport` implementation, not a fake browser or
//! a direct-dispatch shortcut. It is useful for embedding two encoded peers and
//! for running the same protocol conformance suite without a socket or Tokio.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use pwright_transport::{FrameTransport, ShutdownFrameTransport, WireFrame};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryTransportError {
    QueueFull { capacity: usize },
    WriteClosed,
    PeerClosed,
    StateUnavailable,
}

impl Display for MemoryTransportError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueFull { capacity } => {
                write!(
                    formatter,
                    "in-memory frame queue capacity {capacity} was reached"
                )
            }
            Self::WriteClosed => formatter.write_str("in-memory frame sender is closed"),
            Self::PeerClosed => formatter.write_str("in-memory frame receiver is closed"),
            Self::StateUnavailable => {
                formatter.write_str("in-memory frame transport state is unavailable")
            }
        }
    }
}

impl Error for MemoryTransportError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryTransportMetrics {
    incoming_depth: usize,
    outgoing_depth: usize,
    capacity_per_direction: usize,
    incoming_sender_closed: bool,
    outgoing_shutdown: bool,
    peer_receiver_closed: bool,
}

impl MemoryTransportMetrics {
    pub const fn incoming_depth(self) -> usize {
        self.incoming_depth
    }

    pub const fn outgoing_depth(self) -> usize {
        self.outgoing_depth
    }

    pub const fn capacity_per_direction(self) -> usize {
        self.capacity_per_direction
    }

    pub const fn incoming_sender_closed(self) -> bool {
        self.incoming_sender_closed
    }

    pub const fn outgoing_shutdown(self) -> bool {
        self.outgoing_shutdown
    }

    pub const fn peer_receiver_closed(self) -> bool {
        self.peer_receiver_closed
    }
}

struct Direction {
    frames: VecDeque<WireFrame>,
    capacity: usize,
    sender_closed: bool,
    receiver_closed: bool,
    receiver_waker: Option<Waker>,
}

impl Direction {
    fn new(capacity: usize) -> Self {
        Self {
            frames: VecDeque::new(),
            capacity,
            sender_closed: false,
            receiver_closed: false,
            receiver_waker: None,
        }
    }
}

/// One endpoint of a bounded full-duplex in-memory frame connection.
pub struct MemoryFrameTransport {
    incoming: Arc<Mutex<Direction>>,
    outgoing: Arc<Mutex<Direction>>,
    capacity_per_direction: usize,
}

impl MemoryFrameTransport {
    /// Creates two peer endpoints with independent queues of the given capacity.
    /// A zero capacity is valid and rejects every send explicitly.
    pub fn pair(capacity_per_direction: usize) -> (Self, Self) {
        let left_to_right = Arc::new(Mutex::new(Direction::new(capacity_per_direction)));
        let right_to_left = Arc::new(Mutex::new(Direction::new(capacity_per_direction)));
        (
            Self {
                incoming: Arc::clone(&right_to_left),
                outgoing: Arc::clone(&left_to_right),
                capacity_per_direction,
            },
            Self {
                incoming: left_to_right,
                outgoing: right_to_left,
                capacity_per_direction,
            },
        )
    }

    pub fn metrics(&self) -> Result<MemoryTransportMetrics, MemoryTransportError> {
        let (incoming_depth, incoming_sender_closed) = {
            let incoming = self
                .incoming
                .lock()
                .map_err(|_| MemoryTransportError::StateUnavailable)?;
            (incoming.frames.len(), incoming.sender_closed)
        };
        let (outgoing_depth, outgoing_shutdown, peer_receiver_closed) = {
            let outgoing = self
                .outgoing
                .lock()
                .map_err(|_| MemoryTransportError::StateUnavailable)?;
            (
                outgoing.frames.len(),
                outgoing.sender_closed,
                outgoing.receiver_closed,
            )
        };
        Ok(MemoryTransportMetrics {
            incoming_depth,
            outgoing_depth,
            capacity_per_direction: self.capacity_per_direction,
            incoming_sender_closed,
            outgoing_shutdown,
            peer_receiver_closed,
        })
    }
}

impl FrameTransport for MemoryFrameTransport {
    type Error = MemoryTransportError;
    type Send<'a>
        = MemorySend<'a>
    where
        Self: 'a;
    type Receive<'a>
        = MemoryReceive<'a>
    where
        Self: 'a;

    fn send(&self, frame: WireFrame) -> Self::Send<'_> {
        MemorySend {
            transport: self,
            frame: Some(frame),
        }
    }

    fn receive(&self) -> Self::Receive<'_> {
        MemoryReceive { transport: self }
    }
}

impl ShutdownFrameTransport for MemoryFrameTransport {
    type Shutdown<'a>
        = MemoryShutdown<'a>
    where
        Self: 'a;

    fn shutdown(&self) -> Self::Shutdown<'_> {
        MemoryShutdown {
            transport: self,
            completed: false,
        }
    }
}

impl Drop for MemoryFrameTransport {
    fn drop(&mut self) {
        let outgoing_waker = if let Ok(mut outgoing) = self.outgoing.lock() {
            outgoing.sender_closed = true;
            outgoing.receiver_waker.take()
        } else {
            None
        };
        if let Ok(mut incoming) = self.incoming.lock() {
            incoming.receiver_closed = true;
            incoming.frames.clear();
            incoming.receiver_waker = None;
        }
        if let Some(waker) = outgoing_waker {
            waker.wake();
        }
    }
}

pub struct MemorySend<'a> {
    transport: &'a MemoryFrameTransport,
    frame: Option<WireFrame>,
}

impl Future for MemorySend<'_> {
    type Output = Result<(), MemoryTransportError>;

    fn poll(mut self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        let frame = self
            .frame
            .take()
            .expect("completed in-memory send future must not be polled again");
        let waker = {
            let mut outgoing = match self.transport.outgoing.lock() {
                Ok(outgoing) => outgoing,
                Err(_) => return Poll::Ready(Err(MemoryTransportError::StateUnavailable)),
            };
            if outgoing.sender_closed {
                return Poll::Ready(Err(MemoryTransportError::WriteClosed));
            }
            if outgoing.receiver_closed {
                return Poll::Ready(Err(MemoryTransportError::PeerClosed));
            }
            if outgoing.frames.len() >= outgoing.capacity {
                return Poll::Ready(Err(MemoryTransportError::QueueFull {
                    capacity: outgoing.capacity,
                }));
            }
            outgoing.frames.push_back(frame);
            outgoing.receiver_waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
        Poll::Ready(Ok(()))
    }
}

pub struct MemoryShutdown<'a> {
    transport: &'a MemoryFrameTransport,
    completed: bool,
}

impl Future for MemoryShutdown<'_> {
    type Output = Result<(), MemoryTransportError>;

    fn poll(mut self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        assert!(
            !self.completed,
            "completed in-memory shutdown future must not be polled again"
        );
        self.completed = true;
        let waker = {
            let mut outgoing = match self.transport.outgoing.lock() {
                Ok(outgoing) => outgoing,
                Err(_) => return Poll::Ready(Err(MemoryTransportError::StateUnavailable)),
            };
            outgoing.sender_closed = true;
            outgoing.receiver_waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
        Poll::Ready(Ok(()))
    }
}

pub struct MemoryReceive<'a> {
    transport: &'a MemoryFrameTransport,
}

impl Future for MemoryReceive<'_> {
    type Output = Result<Option<WireFrame>, MemoryTransportError>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let mut incoming = match self.transport.incoming.lock() {
            Ok(incoming) => incoming,
            Err(_) => return Poll::Ready(Err(MemoryTransportError::StateUnavailable)),
        };
        if let Some(frame) = incoming.frames.pop_front() {
            return Poll::Ready(Ok(Some(frame)));
        }
        if incoming.sender_closed {
            return Poll::Ready(Ok(None));
        }
        if incoming
            .receiver_waker
            .as_ref()
            .is_none_or(|registered| !registered.will_wake(context.waker()))
        {
            incoming.receiver_waker = Some(context.waker().clone());
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Future, Ready, ready};
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};

    use pwright_client_wire::{DriverEvent, EncodedClientChannel};
    use pwright_core::{
        CallId, CallMetadata, Command, CommandResult, CommandSpec, MethodTag, ObjectId,
        ProtocolVersion, Service,
    };
    use pwright_server_wire::{EncodedServerDriver, ServerDriverEvent, TypedCommandHandler};
    use pwright_transport::{
        EncodedParts, FrameLimits, FrameRoute, FrameTransport, ShutdownFrameTransport, WireCodec,
        WireFrame, WireProtocol,
    };

    use super::{MemoryFrameTransport, MemoryTransportError};

    struct Echo;

    impl CommandSpec for Echo {
        const METHOD: MethodTag = MethodTag::new(7);
        type Params = u64;
        type Result = u64;
        type ErrorDetails = ();
    }

    #[derive(Clone, Copy)]
    struct EchoCodec;

    impl WireCodec<Command<Echo>> for EchoCodec {
        type Error = &'static str;

        fn encode_parts(&self, message: &Command<Echo>) -> Result<EncodedParts, Self::Error> {
            Ok(EncodedParts::new(
                FrameRoute::Command {
                    call_id: message.call_id(),
                    target: message.object_id(),
                    method: message.method(),
                },
                message.params().to_be_bytes().to_vec(),
                Vec::new(),
            ))
        }

        fn decode_frame(&self, frame: WireFrame) -> Result<Command<Echo>, Self::Error> {
            let FrameRoute::Command {
                call_id,
                target,
                method,
            } = frame.route()
            else {
                return Err("expected command");
            };
            if method != Echo::METHOD {
                return Err("wrong method");
            }
            let value = u64::from_be_bytes(
                frame
                    .payload()
                    .try_into()
                    .map_err(|_| "wrong command length")?,
            );
            Ok(Command::new(
                call_id,
                target,
                CallMetadata::default(),
                value,
            ))
        }
    }

    impl WireCodec<CommandResult<Echo>> for EchoCodec {
        type Error = &'static str;

        fn encode_parts(&self, message: &CommandResult<Echo>) -> Result<EncodedParts, Self::Error> {
            let value = message
                .outcome()
                .as_ref()
                .map_err(|_| "test service should succeed")?;
            Ok(EncodedParts::new(
                FrameRoute::Result {
                    call_id: message.call_id(),
                },
                value.to_be_bytes().to_vec(),
                Vec::new(),
            ))
        }

        fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<Echo>, Self::Error> {
            let FrameRoute::Result { call_id } = frame.route() else {
                return Err("expected result");
            };
            let value = u64::from_be_bytes(
                frame
                    .payload()
                    .try_into()
                    .map_err(|_| "wrong result length")?,
            );
            Ok(CommandResult::new(call_id, Ok(value)))
        }
    }

    struct EchoService;

    impl Service<Command<Echo>> for EchoService {
        type Response = CommandResult<Echo>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<Echo>) -> Self::Future<'_> {
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(command.into_params()),
            )))
        }
    }

    fn limits() -> FrameLimits {
        FrameLimits::new(16, 0, 0, 16)
    }

    fn protocol() -> WireProtocol<EchoCodec> {
        WireProtocol::new_exact_directional(
            EchoCodec,
            ProtocolVersion::new(1, 0),
            limits(),
            limits(),
        )
    }

    fn frame(call_id: u64) -> WireFrame {
        WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Result {
                call_id: CallId::new(call_id),
            },
            vec![0; 8],
            Vec::new(),
            limits(),
        )
        .expect("frame should fit")
    }

    fn poll_once<F>(future: Pin<&mut F>) -> Poll<F::Output>
    where
        F: Future + ?Sized,
    {
        future.poll(&mut Context::from_waker(Waker::noop()))
    }

    fn complete<F>(future: F) -> F::Output
    where
        F: Future,
    {
        let mut future = std::pin::pin!(future);
        let Poll::Ready(output) = poll_once(future.as_mut()) else {
            panic!("operation should complete immediately");
        };
        output
    }

    #[test]
    fn bounded_queues_preserve_fifo_and_reject_overflow() {
        // Arrange
        let (left, right) = MemoryFrameTransport::pair(2);

        // Act
        let first = complete(left.send(frame(1)));
        let second = complete(left.send(frame(2)));
        let overflow = complete(left.send(frame(3)));
        let mut receive_first = Box::pin(right.receive());
        let received_first = poll_once(receive_first.as_mut());
        drop(receive_first);
        let mut receive_second = Box::pin(right.receive());
        let received_second = poll_once(receive_second.as_mut());

        // Assert
        assert_eq!(first, Ok(()));
        assert_eq!(second, Ok(()));
        assert_eq!(
            overflow,
            Err(MemoryTransportError::QueueFull { capacity: 2 })
        );
        assert!(matches!(
            received_first,
            Poll::Ready(Ok(Some(frame))) if frame.route() == FrameRoute::Result { call_id: CallId::new(1) }
        ));
        assert!(matches!(
            received_second,
            Poll::Ready(Ok(Some(frame))) if frame.route() == FrameRoute::Result { call_id: CallId::new(2) }
        ));
    }

    #[test]
    fn unpolled_send_has_no_transport_side_effect() {
        // Arrange
        let (left, right) = MemoryFrameTransport::pair(1);
        let unsent = left.send(frame(1));

        // Act
        drop(unsent);
        let metrics = right.metrics().expect("transport metrics should load");
        let mut receive = Box::pin(right.receive());
        let receive = poll_once(receive.as_mut());

        // Assert
        assert_eq!(metrics.incoming_depth(), 0);
        assert!(receive.is_pending());
    }

    #[test]
    fn dropping_the_peer_rejects_future_sends_and_releases_queued_frames() {
        // Arrange
        let (left, right) = MemoryFrameTransport::pair(2);
        assert_eq!(complete(left.send(frame(1))), Ok(()));

        // Act
        drop(right);
        let send = complete(left.send(frame(2)));
        let metrics = left.metrics().expect("transport metrics should load");

        // Assert
        assert_eq!(send, Err(MemoryTransportError::PeerClosed));
        assert_eq!(metrics.outgoing_depth(), 0);
        assert!(metrics.peer_receiver_closed());
    }

    #[test]
    fn shutdown_drains_preceding_frames_then_reports_clean_eof() {
        // Arrange
        let (left, right) = MemoryFrameTransport::pair(1);
        assert_eq!(complete(left.send(frame(1))), Ok(()));

        // Act
        let shutdown = complete(ShutdownFrameTransport::shutdown(&left));
        let mut first = Box::pin(right.receive());
        let first = poll_once(first.as_mut());
        let mut eof = Box::pin(right.receive());
        let eof = poll_once(eof.as_mut());

        // Assert
        assert_eq!(shutdown, Ok(()));
        assert!(matches!(first, Poll::Ready(Ok(Some(_)))));
        assert!(matches!(eof, Poll::Ready(Ok(None))));
        assert!(matches!(
            complete(left.send(frame(2))),
            Err(MemoryTransportError::WriteClosed)
        ));
    }

    #[tokio::test]
    async fn typed_client_and_server_are_transport_independent() {
        // Arrange
        let (client_transport, server_transport) = MemoryFrameTransport::pair(2);
        let (channel, mut client_driver) =
            EncodedClientChannel::new(protocol(), client_transport, 1);
        let mut server_driver = EncodedServerDriver::new(server_transport);
        let mut handler = TypedCommandHandler::<Echo, _, _>::new(protocol(), EchoService);
        let command = Command::<Echo>::new(
            CallId::new(9),
            ObjectId::new(4),
            CallMetadata::default(),
            42,
        );

        // Act
        let (result, server, client) = tokio::join!(
            channel.call(command),
            server_driver.receive_once_with(&mut handler),
            client_driver.receive_once(),
        );

        // Assert
        assert_eq!(
            result.expect("client service is infallible").into_outcome(),
            Ok(42)
        );
        let Ok(ServerDriverEvent::Handled(handled)) = server else {
            panic!("server should handle the typed command");
        };
        assert_eq!(handled.call_id(), CallId::new(9));
        assert_eq!(handled.object_id(), ObjectId::new(4));
        assert_eq!(handled.method(), Echo::METHOD);
        assert!(matches!(client, Ok(DriverEvent::Delivered(_))));
    }
}
