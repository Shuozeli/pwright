use std::convert::Infallible;
use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use pwright_client_wire::EncodedClientChannel;
use pwright_codec_control::{
    CONTROL_PROTOCOL_CURRENT_VERSION, CONTROL_PROTOCOL_ID, NegotiatedControlCodec,
};
use pwright_core::ProtocolVersion;
use pwright_server_wire::{
    GracefulCloseControlEvent, GracefulCloseControlHandler, ServerControlFrameHandler,
    ServerFrameDisposition, ServerSupervisorCloseReason,
};
use pwright_transport::{
    FrameLimits, FrameTransport, NegotiationLimits, ServerHello, WireCapability, WireCapabilityId,
    WireProtocol,
};
use pwright_transport_memory::MemoryFrameTransport;

#[derive(Clone, Copy)]
struct ApplicationCodec;

struct CleanupControl {
    cleanups: usize,
}

impl ServerControlFrameHandler for CleanupControl {
    type Event = &'static str;
    type Error = Infallible;
    type Future<'a> = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>;
    type CloseFuture<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

    fn handle_control<'a>(&'a mut self, frame: pwright_transport::WireFrame) -> Self::Future<'a> {
        ready(Ok(ServerFrameDisposition::Unhandled(frame)))
    }

    fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
        self.handle_connection_terminated(ServerSupervisorCloseReason::EndOfStream)
    }

    fn handle_connection_terminated<'a>(
        &'a mut self,
        _reason: ServerSupervisorCloseReason,
    ) -> Self::CloseFuture<'a> {
        self.cleanups += 1;
        ready(Ok(Some("cleaned")))
    }
}

fn limits() -> FrameLimits {
    FrameLimits::new(64, 0, 0, 64)
}

fn hello() -> ServerHello {
    ServerHello::new(
        ProtocolVersion::new(7, 3),
        limits(),
        limits(),
        vec![WireCapability::new(
            WireCapabilityId::new(CONTROL_PROTOCOL_ID, 64)
                .expect("control protocol ID should be valid"),
            CONTROL_PROTOCOL_CURRENT_VERSION,
        )],
        NegotiationLimits::new(1, 64),
    )
    .expect("server hello should build")
}

fn poll_once<F>(future: Pin<&mut F>) -> Poll<F::Output>
where
    F: Future + ?Sized,
{
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn graceful_close_crosses_memory_transport_and_cleans_up_once() {
    // Arrange
    let hello = hello();
    let application = WireProtocol::new_exact_directional(
        ApplicationCodec,
        hello.selected_version(),
        limits(),
        limits(),
    );
    let client_control = WireProtocol::new_exact_directional(
        NegotiatedControlCodec::for_client(&hello)
            .expect("client control negotiation should succeed"),
        hello.selected_version(),
        limits(),
        limits(),
    );
    let server_control = WireProtocol::new_exact_directional(
        NegotiatedControlCodec::for_server(&hello)
            .expect("server control negotiation should succeed"),
        hello.selected_version(),
        limits(),
        limits(),
    );
    let (client_transport, server_transport) = MemoryFrameTransport::pair(4);
    let server_transport = Arc::new(server_transport);
    let (_channel, mut client) =
        EncodedClientChannel::new_with_control(application, client_control, client_transport, 1)
            .expect("client protocols should compose");
    let mut server = GracefulCloseControlHandler::new(
        Arc::clone(&server_transport),
        server_control,
        CleanupControl { cleanups: 0 },
    );

    // Act
    let client_close = client.graceful_close();
    let server_close = async {
        let close = server_transport
            .receive()
            .await
            .expect("server receive should succeed")
            .expect("client should send Close");
        server
            .handle_control(close)
            .await
            .expect("server should acknowledge Close")
    };
    let (client_result, server_result) = tokio::join!(client_close, server_close);

    // Assert
    let client_result = client_result.expect("client should complete graceful close");
    assert_eq!(client_result.woken_calls(), 0);
    assert_eq!(client_result.ignored_frames(), 0);
    assert_eq!(
        server_result,
        ServerFrameDisposition::Handled(GracefulCloseControlEvent::CloseAcknowledged {
            cleanup: Some("cleaned"),
        })
    );
    assert_eq!(server.inner().cleanups, 1);
    assert_eq!(
        server.terminal_close_reason(),
        Some(ServerSupervisorCloseReason::PeerGracefulClose)
    );
    assert!(
        server_transport
            .metrics()
            .expect("memory metrics should remain available")
            .outgoing_shutdown()
    );
}

#[tokio::test]
async fn legacy_baseline_fails_before_sending_close() {
    // Arrange
    let hello = ServerHello::new(
        ProtocolVersion::new(1, 0),
        limits(),
        limits(),
        Vec::new(),
        NegotiationLimits::new(0, 64),
    )
    .expect("legacy hello should build");
    let application = WireProtocol::new_exact_directional(
        ApplicationCodec,
        hello.selected_version(),
        limits(),
        limits(),
    );
    let control = WireProtocol::new_exact_directional(
        NegotiatedControlCodec::for_client(&hello).expect("legacy baseline control should build"),
        hello.selected_version(),
        limits(),
        limits(),
    );
    let (client_transport, server_transport) = MemoryFrameTransport::pair(1);
    let (channel, mut client) =
        EncodedClientChannel::new_with_control(application, control, client_transport, 1)
            .expect("client protocols should compose");

    // Act
    let result = client.graceful_close().await;

    // Assert
    assert!(matches!(
        result,
        Err(pwright_client_wire::ClientGracefulCloseError::Protocol(_))
    ));
    assert_eq!(
        channel
            .metrics()
            .expect("client metrics should remain available")
            .close_reason(),
        None
    );
    assert_eq!(
        server_transport
            .metrics()
            .expect("memory metrics should remain available")
            .incoming_depth(),
        0
    );
}

#[tokio::test]
async fn cancelled_ack_wait_resumes_without_sending_a_second_close() {
    // Arrange
    let hello = hello();
    let application = WireProtocol::new_exact_directional(
        ApplicationCodec,
        hello.selected_version(),
        limits(),
        limits(),
    );
    let client_control = WireProtocol::new_exact_directional(
        NegotiatedControlCodec::for_client(&hello)
            .expect("client control negotiation should succeed"),
        hello.selected_version(),
        limits(),
        limits(),
    );
    let server_control = WireProtocol::new_exact_directional(
        NegotiatedControlCodec::for_server(&hello)
            .expect("server control negotiation should succeed"),
        hello.selected_version(),
        limits(),
        limits(),
    );
    let (client_transport, server_transport) = MemoryFrameTransport::pair(4);
    let server_transport = Arc::new(server_transport);
    let (_channel, mut client) =
        EncodedClientChannel::new_with_control(application, client_control, client_transport, 1)
            .expect("client protocols should compose");
    let mut server = GracefulCloseControlHandler::new(
        Arc::clone(&server_transport),
        server_control,
        CleanupControl { cleanups: 0 },
    );
    let mut first_attempt = Box::pin(client.graceful_close());

    // Act
    assert!(poll_once(first_attempt.as_mut()).is_pending());
    drop(first_attempt);
    let close = server_transport
        .receive()
        .await
        .expect("server receive should succeed")
        .expect("first attempt should have sent Close");
    server
        .handle_control(close)
        .await
        .expect("server should acknowledge Close");
    let resumed = client
        .graceful_close()
        .await
        .expect("client should resume at Ack wait");

    // Assert
    assert_eq!(resumed.ignored_frames(), 0);
    assert_eq!(server.inner().cleanups, 1);
    assert_eq!(
        server_transport
            .metrics()
            .expect("memory metrics should remain available")
            .incoming_depth(),
        0
    );
}
