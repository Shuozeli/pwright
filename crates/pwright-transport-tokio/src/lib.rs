//! Optional Tokio `AsyncRead`/`AsyncWrite` transport for Pwright frames.
//!
//! Send and receive halves are locked independently, allowing one reader and
//! one writer to progress concurrently while serializing writers. No socket,
//! task, or runtime is created by this crate.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::io;
use std::pin::Pin;

use pwright_core::{
    ProductionResourceError, ProductionResourceLedger, ResourceClass, ResourceError, ResourceLease,
    ResourceLedger, ResourceRequirements, ResourceScope,
};
use pwright_transport::{
    FrameTransport, ProductionFrameTransport, ProductionTransportResources, ShutdownFrameTransport,
    WireFrame,
};
use pwright_transport_stream::{STREAM_LENGTH_PREFIX_BYTES, StreamFrameCodec, StreamFrameError};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf, split};
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum TokioTransportError {
    Io(io::Error),
    Frame(StreamFrameError),
    DeclaredEnvelopeOutOfRange { declared: u64 },
    DeclaredEnvelopeTooLarge { declared: usize, limit: usize },
    ResourceLengthOutOfRange { bytes: usize },
    LengthOverflow,
    AllocationFailed { bytes: usize },
    UnbudgetedOutboundFrame,
    ProductionResource(ProductionResourceError),
    Resource(ResourceError),
}

impl Display for TokioTransportError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => Display::fmt(error, formatter),
            Self::Frame(error) => Display::fmt(error, formatter),
            Self::DeclaredEnvelopeOutOfRange { declared } => write!(
                formatter,
                "declared stream envelope length {declared} does not fit this platform"
            ),
            Self::DeclaredEnvelopeTooLarge { declared, limit } => write!(
                formatter,
                "declared stream envelope length {declared} exceeds limit {limit}"
            ),
            Self::ResourceLengthOutOfRange { bytes } => write!(
                formatter,
                "resource byte length {bytes} cannot be represented by the resource ledger"
            ),
            Self::LengthOverflow => formatter.write_str("stream write length overflowed"),
            Self::AllocationFailed { bytes } => {
                write!(formatter, "could not reserve {bytes} receive bytes")
            }
            Self::UnbudgetedOutboundFrame => formatter.write_str(
                "production stream transport requires an outbound frame budgeted by the same connection ledger",
            ),
            Self::ProductionResource(error) => Display::fmt(error, formatter),
            Self::Resource(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for TokioTransportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Frame(error) => Some(error),
            Self::ProductionResource(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::DeclaredEnvelopeOutOfRange { .. }
            | Self::DeclaredEnvelopeTooLarge { .. }
            | Self::ResourceLengthOutOfRange { .. }
            | Self::LengthOverflow
            | Self::AllocationFailed { .. }
            | Self::UnbudgetedOutboundFrame => None,
        }
    }
}

impl From<io::Error> for TokioTransportError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<StreamFrameError> for TokioTransportError {
    fn from(error: StreamFrameError) -> Self {
        Self::Frame(error)
    }
}

impl From<ResourceError> for TokioTransportError {
    fn from(error: ResourceError) -> Self {
        Self::Resource(error)
    }
}

impl From<ProductionResourceError> for TokioTransportError {
    fn from(error: ProductionResourceError) -> Self {
        Self::ProductionResource(error)
    }
}

/// A full-duplex frame transport over caller-owned Tokio I/O halves.
pub struct TokioStreamTransport<Reader, Writer> {
    reader: Mutex<Reader>,
    writer: Mutex<WriterState<Writer>>,
    codec: StreamFrameCodec,
    resource_ledger: Option<ResourceLedger>,
}

/// Type-level proof that this Tokio transport claimed its complete production
/// retention profile from one connection ledger.
pub struct ProductionTokioStreamTransport<Reader, Writer> {
    inner: TokioStreamTransport<Reader, Writer>,
    requirements: ResourceRequirements,
}

struct PendingWrite {
    bytes: Vec<u8>,
    offset: usize,
    _wire_bytes: Option<ResourceLease>,
}

struct WriterState<Writer> {
    writer: Writer,
    pending: Option<PendingWrite>,
}

/// Exact production requirement for one admitted send and receive over a
/// stream codec. Endpoint/composition layers can declare the transport plan
/// before a socket or I/O half exists.
pub fn production_resource_requirements_for_codec(
    codec: StreamFrameCodec,
) -> Result<ResourceRequirements, StreamFrameError> {
    let bounds = codec.limits().retention_bounds()?;
    Ok(ResourceRequirements::none()
        .with_class(
            ResourceClass::WireBytes,
            bounds.full_duplex_wire_bytes(),
            bounds.full_duplex_wire_bytes(),
        )
        .with_metadata(bounds.scopes(), bounds.leases()))
}

impl<Reader, Writer> TokioStreamTransport<Reader, Writer> {
    pub const fn from_parts(reader: Reader, writer: Writer, codec: StreamFrameCodec) -> Self {
        Self {
            reader: Mutex::const_new(reader),
            writer: Mutex::const_new(WriterState {
                writer,
                pending: None,
            }),
            codec,
            resource_ledger: None,
        }
    }

    pub const fn codec(&self) -> StreamFrameCodec {
        self.codec
    }

    /// Accounts inbound envelope staging and decoded frame storage against one
    /// explicitly shared connection ledger.
    pub fn with_resource_ledger(mut self, resource_ledger: ResourceLedger) -> Self {
        self.resource_ledger = Some(resource_ledger);
        self
    }

    /// Exact production requirement for one admitted send and receive running
    /// concurrently. Extra concurrent sends are rejected by the shared ledger
    /// rather than allocating outside this profile.
    pub fn production_resource_requirements(
        &self,
    ) -> Result<ResourceRequirements, StreamFrameError> {
        production_resource_requirements_for_codec(self.codec)
    }

    /// Production-only installation. Besides staging envelopes, this mode
    /// rejects any outbound frame that was not admitted by the same ledger.
    pub fn with_production_resources(
        mut self,
        resources: &ProductionResourceLedger,
    ) -> Result<ProductionTokioStreamTransport<Reader, Writer>, TokioTransportError> {
        let requirements = self.production_resource_requirements()?;
        let ledger = resources.claim(requirements)?;
        self.resource_ledger = Some(ledger);
        Ok(ProductionTokioStreamTransport {
            inner: self,
            requirements,
        })
    }
}

impl<Reader, Writer> ProductionTokioStreamTransport<Reader, Writer> {
    pub const fn codec(&self) -> StreamFrameCodec {
        self.inner.codec()
    }

    pub const fn requirements(&self) -> ResourceRequirements {
        self.requirements
    }
}

impl<Reader, Writer> ProductionTokioStreamTransport<Reader, Writer>
where
    Writer: AsyncWrite + Unpin,
{
    pub async fn shutdown(&self) -> Result<(), TokioTransportError> {
        self.inner.shutdown().await
    }
}

impl<Reader, Writer> ProductionFrameTransport for ProductionTokioStreamTransport<Reader, Writer>
where
    Reader: AsyncRead + Unpin + Send,
    Writer: AsyncWrite + Unpin + Send,
{
    fn production_resources(&self) -> ProductionTransportResources<'_> {
        ProductionTransportResources::new(
            self.inner
                .resource_ledger
                .as_ref()
                .expect("production transport owns its claimed ledger"),
            self.requirements,
        )
    }
}

impl<Reader, Writer> FrameTransport for ProductionTokioStreamTransport<Reader, Writer>
where
    Reader: AsyncRead + Unpin + Send,
    Writer: AsyncWrite + Unpin + Send,
{
    type Error = TokioTransportError;
    type Send<'a>
        = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>
    where
        Self: 'a;
    type Receive<'a>
        = Pin<Box<dyn Future<Output = Result<Option<WireFrame>, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn send(&self, frame: WireFrame) -> Self::Send<'_> {
        Box::pin(async move {
            let ledger = self
                .inner
                .resource_ledger
                .as_ref()
                .expect("production transport owns its claimed ledger");
            if !frame.is_budgeted_by(ledger) {
                return Err(TokioTransportError::UnbudgetedOutboundFrame);
            }
            self.inner.send(frame).await
        })
    }

    fn receive(&self) -> Self::Receive<'_> {
        Box::pin(async move { self.inner.receive().await })
    }
}

impl<Reader, Writer> ShutdownFrameTransport for ProductionTokioStreamTransport<Reader, Writer>
where
    Reader: AsyncRead + Unpin + Send,
    Writer: AsyncWrite + Unpin + Send,
{
    type Shutdown<'a>
        = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn shutdown(&self) -> Self::Shutdown<'_> {
        Box::pin(async move { self.inner.shutdown().await })
    }
}

impl<Io> TokioStreamTransport<ReadHalf<Io>, WriteHalf<Io>>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    pub fn from_io(io: Io, codec: StreamFrameCodec) -> Self {
        let (reader, writer) = split(io);
        Self::from_parts(reader, writer, codec)
    }
}

impl<Reader, Writer> TokioStreamTransport<Reader, Writer>
where
    Writer: AsyncWrite + Unpin,
{
    /// Flushes and shuts down only this transport's write half.
    pub async fn shutdown(&self) -> Result<(), TokioTransportError> {
        let mut state = self.writer.lock().await;
        finish_pending_write(&mut state).await?;
        state
            .writer
            .shutdown()
            .await
            .map_err(TokioTransportError::Io)
    }
}

impl<Reader, Writer> FrameTransport for TokioStreamTransport<Reader, Writer>
where
    Reader: AsyncRead + Unpin + Send,
    Writer: AsyncWrite + Unpin + Send,
{
    type Error = TokioTransportError;
    type Send<'a>
        = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>
    where
        Self: 'a;
    type Receive<'a>
        = Pin<Box<dyn Future<Output = Result<Option<WireFrame>, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn send(&self, frame: WireFrame) -> Self::Send<'_> {
        Box::pin(async move {
            let envelope_length = self.codec.encoded_len(&frame)?;
            let envelope_wire_bytes =
                reserve_wire_bytes(self.resource_ledger.as_ref(), envelope_length)?;
            let envelope = self.codec.encode(&frame)?;
            let declared = u64::try_from(envelope.len()).map_err(|_| {
                TokioTransportError::DeclaredEnvelopeOutOfRange { declared: u64::MAX }
            })?;
            let pending_length = STREAM_LENGTH_PREFIX_BYTES
                .checked_add(envelope.len())
                .ok_or(TokioTransportError::LengthOverflow)?;
            let pending_wire_bytes =
                reserve_wire_bytes(self.resource_ledger.as_ref(), pending_length)?;
            let mut bytes = Vec::new();
            bytes.try_reserve_exact(pending_length).map_err(|_| {
                TokioTransportError::AllocationFailed {
                    bytes: pending_length,
                }
            })?;
            bytes.extend_from_slice(&declared.to_be_bytes());
            bytes.extend_from_slice(&envelope);
            drop(envelope);
            drop(envelope_wire_bytes);

            let mut state = self.writer.lock().await;
            // A previous send future may have been dropped after writing only
            // part of its envelope. Complete it before starting this frame so
            // cancellation cannot corrupt stream framing.
            finish_pending_write(&mut state).await?;
            state.pending = Some(PendingWrite {
                bytes,
                offset: 0,
                _wire_bytes: pending_wire_bytes,
            });
            finish_pending_write(&mut state).await
        })
    }

    fn receive(&self) -> Self::Receive<'_> {
        Box::pin(async move {
            let mut reader = self.reader.lock().await;
            let mut prefix = [0u8; STREAM_LENGTH_PREFIX_BYTES];
            let first = reader.read(&mut prefix[..1]).await?;
            if first == 0 {
                return Ok(None);
            }
            reader.read_exact(&mut prefix[1..]).await?;
            let declared_u64 = u64::from_be_bytes(prefix);
            let declared = usize::try_from(declared_u64).map_err(|_| {
                TokioTransportError::DeclaredEnvelopeOutOfRange {
                    declared: declared_u64,
                }
            })?;
            let limit = self.codec.limits().envelope_bytes();
            if declared > limit {
                return Err(TokioTransportError::DeclaredEnvelopeTooLarge { declared, limit });
            }
            // Reserve the complete staging buffer before allocating it. This
            // lease remains live while the decoder reserves and copies the
            // retained frame bytes, so metrics expose the real overlap peak.
            let envelope_wire_bytes = reserve_wire_bytes(self.resource_ledger.as_ref(), declared)?;
            let mut envelope = Vec::new();
            envelope
                .try_reserve_exact(declared)
                .map_err(|_| TokioTransportError::AllocationFailed { bytes: declared })?;
            envelope.resize(declared, 0);
            reader.read_exact(&mut envelope).await?;
            let frame = match &self.resource_ledger {
                Some(ledger) => self.codec.decode_budgeted(&envelope, ledger),
                None => self.codec.decode(&envelope),
            }
            .map_err(TokioTransportError::Frame)?;
            drop(envelope);
            drop(envelope_wire_bytes);
            Ok(Some(frame))
        })
    }
}

impl<Reader, Writer> ShutdownFrameTransport for TokioStreamTransport<Reader, Writer>
where
    Reader: AsyncRead + Unpin + Send,
    Writer: AsyncWrite + Unpin + Send,
{
    type Shutdown<'a>
        = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn shutdown(&self) -> Self::Shutdown<'_> {
        Box::pin(async move { TokioStreamTransport::shutdown(self).await })
    }
}

fn reserve_wire_bytes(
    ledger: Option<&ResourceLedger>,
    bytes: usize,
) -> Result<Option<ResourceLease>, TokioTransportError> {
    let Some(ledger) = ledger else {
        return Ok(None);
    };
    if bytes == 0 {
        return Ok(None);
    }
    let units = u64::try_from(bytes)
        .map_err(|_| TokioTransportError::ResourceLengthOutOfRange { bytes })?;
    ledger
        .reserve(ResourceScope::Connection, ResourceClass::WireBytes, units)
        .map(Some)
        .map_err(TokioTransportError::Resource)
}

async fn finish_pending_write<Writer>(
    state: &mut WriterState<Writer>,
) -> Result<(), TokioTransportError>
where
    Writer: AsyncWrite + Unpin,
{
    while state
        .pending
        .as_ref()
        .is_some_and(|pending| pending.offset < pending.bytes.len())
    {
        let written = {
            let WriterState { writer, pending } = state;
            let pending = pending
                .as_ref()
                .expect("pending write was checked before polling the writer");
            writer.write(&pending.bytes[pending.offset..]).await?
        };
        if written == 0 {
            return Err(TokioTransportError::Io(io::Error::new(
                io::ErrorKind::WriteZero,
                "could not finish Pwright frame envelope",
            )));
        }
        state
            .pending
            .as_mut()
            .expect("pending write must remain owned by the transport")
            .offset += written;
    }
    if state.pending.is_some() {
        state.writer.flush().await?;
        state.pending = None;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_core::{
        CallId, MethodTag, ObjectId, ProductionResourceLedger, ProtocolVersion,
        ResourceCapacityBoundary, ResourceClass, ResourceError, ResourceLedger, ResourceLimits,
        ResourceRequirements, ResourceScope,
    };
    use pwright_transport::{
        FrameLimits, FrameRoute, FrameTransport, ShutdownFrameTransport, WireFrame,
    };
    use pwright_transport_stream::{
        STREAM_LENGTH_PREFIX_BYTES, StreamFrameCodec, StreamFrameError, StreamFrameLimits,
    };
    use tokio::io::{AsyncWriteExt, duplex};

    use super::{
        TokioStreamTransport, TokioTransportError, production_resource_requirements_for_codec,
    };

    fn codec(envelope_bytes: usize) -> StreamFrameCodec {
        StreamFrameCodec::new(StreamFrameLimits::new(
            FrameLimits::new(32, 1, 16, 48),
            envelope_bytes,
        ))
    }

    fn frame(call_id: u64) -> WireFrame {
        WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Command {
                call_id: CallId::new(call_id),
                target: ObjectId::new(2),
                method: MethodTag::new(3),
            },
            vec![1, 2, 3],
            vec![vec![4, 5]],
            codec(128).limits().frame(),
        )
        .expect("frame should build")
    }

    fn wire_ledger(global: u64) -> ResourceLedger {
        ResourceLedger::new(ResourceLimits::deny_all(8, 8).with_class(
            ResourceClass::WireBytes,
            global,
            global,
        ))
    }

    fn production_resources(requirements: ResourceRequirements) -> ProductionResourceLedger {
        let wire = requirements.class(ResourceClass::WireBytes);
        ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::WireBytes,
                wire.global,
                wire.per_scope,
            ),
            requirements,
        )
        .expect("exact production requirements should fit")
    }

    #[test]
    fn transport_plan_is_available_before_io_exists() {
        // Arrange
        let stream_codec = codec(128);
        let (stream, _peer) = duplex(512);
        let transport = TokioStreamTransport::from_io(stream, stream_codec);

        // Act
        let pre_io = production_resource_requirements_for_codec(stream_codec)
            .expect("valid codec should have a transport plan");
        let attached = transport
            .production_resource_requirements()
            .expect("attached transport should have the same plan");

        // Assert
        assert_eq!(pre_io, attached);
    }

    #[tokio::test]
    async fn duplex_stream_round_trips_a_frame() {
        // Arrange
        let (left, right) = duplex(512);
        let sender = TokioStreamTransport::from_io(left, codec(128));
        let receiver = TokioStreamTransport::from_io(right, codec(128));
        let expected = frame(7);

        // Act
        sender
            .send(expected.clone())
            .await
            .expect("frame should send");
        let actual = receiver
            .receive()
            .await
            .expect("frame should receive")
            .expect("stream should remain open");

        // Assert
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn budgeted_send_measures_encoded_and_pending_buffer_overlap() {
        // Arrange
        let stream_codec = codec(128);
        let expected = frame(7);
        let envelope_bytes = stream_codec
            .encoded_len(&expected)
            .expect("frame should have a valid encoded length");
        let pending_bytes = STREAM_LENGTH_PREFIX_BYTES + envelope_bytes;
        let ledger = wire_ledger((envelope_bytes + pending_bytes) as u64);
        let (left, right) = duplex(512);
        let sender =
            TokioStreamTransport::from_io(left, stream_codec).with_resource_ledger(ledger.clone());
        let receiver = TokioStreamTransport::from_io(right, stream_codec);

        // Act
        sender.send(expected).await.expect("frame should send");
        let actual = receiver
            .receive()
            .await
            .expect("frame should receive")
            .expect("stream should remain open");
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(actual.route(), frame(7).route());
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(
            metrics.class(ResourceClass::WireBytes).peak,
            (envelope_bytes + pending_bytes) as u64
        );
        assert_eq!(metrics.active_leases, 0);
    }

    #[tokio::test]
    async fn production_send_requires_and_accounts_a_same_ledger_frame() {
        // Arrange
        let stream_codec = codec(128);
        let (left, right) = duplex(512);
        let unconfigured = TokioStreamTransport::from_io(left, stream_codec);
        let requirements = unconfigured
            .production_resource_requirements()
            .expect("stream profile should be valid");
        let production = production_resources(requirements);
        let frame_requirements = ResourceRequirements::none()
            .with_class(ResourceClass::WireBytes, 48, 48)
            .with_metadata(1, 1);
        let ledger = production
            .claim(frame_requirements)
            .expect("transport plan should cover one maximum frame");
        let expected = WireFrame::new_budgeted(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            vec![1; 32],
            vec![vec![2; 16]],
            stream_codec.limits().frame(),
            &ledger,
            ResourceScope::Connection,
        )
        .expect("maximum frame should fit");
        let envelope_bytes = stream_codec
            .encoded_len(&expected)
            .expect("frame should encode");
        let sender = unconfigured
            .with_production_resources(&production)
            .expect("declared transport should install");
        let receiver = TokioStreamTransport::from_io(right, stream_codec);

        // Act
        sender.send(expected).await.expect("frame should send");
        let actual = receiver
            .receive()
            .await
            .expect("frame should receive")
            .expect("stream should remain open");
        let metrics = production.metrics().expect("metrics should load");

        // Assert
        assert_eq!(actual.total_bytes(), 48);
        assert_eq!(requirements.class(ResourceClass::WireBytes).global, 404);
        assert_eq!(
            metrics.class(ResourceClass::WireBytes).peak,
            48 + envelope_bytes as u64 + (STREAM_LENGTH_PREFIX_BYTES + envelope_bytes) as u64
        );
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 0);
    }

    #[tokio::test]
    async fn production_send_rejects_an_unbudgeted_outbound_frame() {
        // Arrange
        let (left, _right) = duplex(512);
        let unconfigured = TokioStreamTransport::from_io(left, codec(128));
        let requirements = unconfigured
            .production_resource_requirements()
            .expect("stream profile should be valid");
        let production = production_resources(requirements);
        let sender = unconfigured
            .with_production_resources(&production)
            .expect("declared transport should install");

        // Act
        let error = sender
            .send(frame(7))
            .await
            .expect_err("raw frame must not enter a production transport");
        let metrics = production.metrics().expect("metrics should load");

        // Assert
        assert!(matches!(
            error,
            TokioTransportError::UnbudgetedOutboundFrame
        ));
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(metrics.class(ResourceClass::WireBytes).peak, 0);
    }

    #[test]
    fn production_transport_rejects_an_omitted_full_duplex_profile() {
        // Arrange
        let (left, _right) = duplex(512);
        let transport = TokioStreamTransport::from_io(left, codec(128));
        let production = ProductionResourceLedger::new(
            ResourceLimits::deny_all(3, 5).with_class(ResourceClass::WireBytes, 404, 404),
            ResourceRequirements::none(),
        )
        .expect("empty plan may have stricter configured capacity");

        // Act
        let error = match transport.with_production_resources(&production) {
            Ok(_) => panic!("undeclared stream transport must not install"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(
            error,
            TokioTransportError::ProductionResource(
                pwright_core::ProductionResourceError::InsufficientGlobal {
                    boundary: ResourceCapacityBoundary::DeclaredPlan,
                    class: ResourceClass::WireBytes,
                    required: 404,
                    available: 0,
                }
            )
        ));
    }

    #[tokio::test]
    async fn send_rejects_staging_budget_before_encoding_the_envelope() {
        // Arrange
        let stream_codec = codec(128);
        let expected = frame(7);
        let envelope_bytes = stream_codec
            .encoded_len(&expected)
            .expect("frame should have a valid encoded length");
        let ledger = wire_ledger((envelope_bytes - 1) as u64);
        let (left, _right) = duplex(512);
        let sender =
            TokioStreamTransport::from_io(left, stream_codec).with_resource_ledger(ledger.clone());

        // Act
        let error = sender
            .send(expected)
            .await
            .expect_err("encoded envelope should exceed the shared budget");
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert!(matches!(
            error,
            TokioTransportError::Resource(ResourceError::GlobalLimitExceeded {
                class: ResourceClass::WireBytes,
                current: 0,
                requested,
                limit,
            }) if requested == envelope_bytes as u64 && limit == (envelope_bytes - 1) as u64
        ));
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(metrics.class(ResourceClass::WireBytes).peak, 0);
        assert_eq!(metrics.rejected_reservations, 1);
    }

    #[tokio::test]
    async fn budgeted_receive_measures_staging_and_retained_frame_overlap() {
        // Arrange
        let stream_codec = codec(128);
        let expected = frame(7);
        let envelope_bytes = stream_codec
            .encode(&expected)
            .expect("frame should encode")
            .len();
        let retained_bytes = expected.total_bytes();
        let ledger = wire_ledger((envelope_bytes + retained_bytes) as u64);
        let (left, right) = duplex(512);
        let sender = TokioStreamTransport::from_io(left, stream_codec);
        let receiver =
            TokioStreamTransport::from_io(right, stream_codec).with_resource_ledger(ledger.clone());

        // Act
        sender.send(expected).await.expect("frame should send");
        let actual = receiver
            .receive()
            .await
            .expect("frame should receive")
            .expect("stream should remain open");
        let while_retained = ledger.metrics().expect("metrics should be available");
        let call = ledger
            .scope_metrics(ResourceScope::Call(CallId::new(7)))
            .expect("scope metrics should be available")
            .expect("decoded frame should retain its call scope");
        let connection = ledger
            .scope_metrics(ResourceScope::Connection)
            .expect("scope metrics should be available");
        drop(actual);
        let after_drop = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(
            while_retained.class(ResourceClass::WireBytes).current,
            retained_bytes as u64
        );
        assert_eq!(
            while_retained.class(ResourceClass::WireBytes).peak,
            (envelope_bytes + retained_bytes) as u64
        );
        assert_eq!(call.units(ResourceClass::WireBytes), retained_bytes as u64);
        assert!(connection.is_none());
        assert_eq!(after_drop.class(ResourceClass::WireBytes).current, 0);
    }

    #[tokio::test]
    async fn receive_rejects_envelope_budget_before_reading_its_body() {
        // Arrange
        let stream_codec = codec(128);
        let declared = stream_codec
            .encode(&frame(7))
            .expect("frame should encode")
            .len();
        let ledger = wire_ledger((declared - 1) as u64);
        let (mut left, right) = duplex(64);
        let receiver =
            TokioStreamTransport::from_io(right, stream_codec).with_resource_ledger(ledger.clone());
        left.write_all(&(declared as u64).to_be_bytes())
            .await
            .expect("prefix should write");

        // Act
        let error = receiver
            .receive()
            .await
            .expect_err("declared envelope should exceed the aggregate budget");
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert!(matches!(
            error,
            TokioTransportError::Resource(ResourceError::GlobalLimitExceeded {
                class: ResourceClass::WireBytes,
                current: 0,
                requested,
                limit,
            }) if requested == declared as u64 && limit == (declared - 1) as u64
        ));
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(metrics.class(ResourceClass::WireBytes).peak, 0);
        assert_eq!(metrics.rejected_reservations, 1);
    }

    #[tokio::test]
    async fn cancelling_partial_receive_releases_envelope_budget() {
        // Arrange
        let stream_codec = codec(128);
        let declared = stream_codec
            .encode(&frame(7))
            .expect("frame should encode")
            .len();
        let ledger = wire_ledger(declared as u64);
        let (mut left, right) = duplex(64);
        let receiver =
            TokioStreamTransport::from_io(right, stream_codec).with_resource_ledger(ledger.clone());
        left.write_all(&(declared as u64).to_be_bytes())
            .await
            .expect("prefix should write");
        let mut interrupted = Box::pin(receiver.receive());
        tokio::select! {
            biased;
            result = &mut interrupted => panic!("body-less receive unexpectedly completed: {result:?}"),
            () = tokio::task::yield_now() => {}
        }
        let while_waiting = ledger.metrics().expect("metrics should be available");

        // Act
        drop(interrupted);
        let after_cancel = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(
            while_waiting.class(ResourceClass::WireBytes).current,
            declared as u64
        );
        assert_eq!(while_waiting.active_leases, 1);
        assert_eq!(after_cancel.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(after_cancel.active_leases, 0);
    }

    #[tokio::test]
    async fn sequential_frames_preserve_send_order() {
        // Arrange
        let (left, right) = duplex(512);
        let sender = TokioStreamTransport::from_io(left, codec(128));
        let receiver = TokioStreamTransport::from_io(right, codec(128));

        // Act
        sender
            .send(frame(7))
            .await
            .expect("first frame should send");
        sender
            .send(frame(8))
            .await
            .expect("second frame should send");
        let first = receiver
            .receive()
            .await
            .expect("first frame should receive")
            .expect("first frame should exist");
        let second = receiver
            .receive()
            .await
            .expect("second frame should receive")
            .expect("second frame should exist");

        // Assert
        assert_eq!(
            first.route(),
            FrameRoute::Command {
                call_id: CallId::new(7),
                target: ObjectId::new(2),
                method: MethodTag::new(3),
            }
        );
        assert_eq!(
            second.route(),
            FrameRoute::Command {
                call_id: CallId::new(8),
                target: ObjectId::new(2),
                method: MethodTag::new(3),
            }
        );
    }

    #[tokio::test]
    async fn dropped_partial_send_is_finished_before_the_next_frame() {
        // Arrange
        let (left, right) = duplex(16);
        let stream_codec = codec(128);
        let pending_bytes = STREAM_LENGTH_PREFIX_BYTES
            + stream_codec
                .encoded_len(&frame(7))
                .expect("frame should have a valid encoded length");
        let ledger = wire_ledger(512);
        let sender = Arc::new(
            TokioStreamTransport::from_io(left, stream_codec).with_resource_ledger(ledger.clone()),
        );
        let receiver = TokioStreamTransport::from_io(right, codec(128));
        let expected_first = frame(7);
        let expected_second = frame(8);
        let mut interrupted = Box::pin(sender.send(expected_first.clone()));
        tokio::select! {
            biased;
            result = &mut interrupted => panic!("small pipe unexpectedly accepted the complete frame: {result:?}"),
            () = tokio::task::yield_now() => {}
        }

        // Act
        drop(interrupted);
        let while_pending = ledger.metrics().expect("metrics should be available");
        let resumed_sender = Arc::clone(&sender);
        let send_second = tokio::spawn(async move { resumed_sender.send(expected_second).await });
        let actual_first = receiver
            .receive()
            .await
            .expect("first resumed frame should decode")
            .expect("first frame should exist");
        let actual_second = receiver
            .receive()
            .await
            .expect("second frame should decode")
            .expect("second frame should exist");
        send_second
            .await
            .expect("second send task should join")
            .expect("resumed and second frames should send");
        let after_send = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(actual_first, expected_first);
        assert_eq!(actual_second.route(), frame(8).route());
        assert_eq!(
            while_pending.class(ResourceClass::WireBytes).current,
            pending_bytes as u64
        );
        assert_eq!(while_pending.active_leases, 1);
        assert_eq!(after_send.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(after_send.active_leases, 0);
    }

    #[tokio::test]
    async fn shutdown_finishes_a_dropped_partial_frame_before_clean_eof() {
        // Arrange
        let (left, right) = duplex(16);
        let stream_codec = codec(128);
        let pending_bytes = STREAM_LENGTH_PREFIX_BYTES
            + stream_codec
                .encoded_len(&frame(7))
                .expect("frame should have a valid encoded length");
        let ledger = wire_ledger(512);
        let sender = Arc::new(
            TokioStreamTransport::from_io(left, stream_codec).with_resource_ledger(ledger.clone()),
        );
        let receiver = TokioStreamTransport::from_io(right, codec(128));
        let expected = frame(7);
        let mut interrupted = Box::pin(sender.send(expected.clone()));
        tokio::select! {
            biased;
            result = &mut interrupted => panic!("small pipe unexpectedly accepted the complete frame: {result:?}"),
            () = tokio::task::yield_now() => {}
        }
        drop(interrupted);
        let while_pending = ledger.metrics().expect("metrics should be available");

        // Act
        let shutdown_sender = Arc::clone(&sender);
        let shutdown =
            tokio::spawn(async move { ShutdownFrameTransport::shutdown(&shutdown_sender).await });
        let actual = receiver
            .receive()
            .await
            .expect("pending frame should decode")
            .expect("pending frame should arrive before EOF");
        let eof = receiver.receive().await.expect("EOF should be clean");
        shutdown
            .await
            .expect("shutdown task should join")
            .expect("shutdown should finish");
        let after_shutdown = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(actual, expected);
        assert!(eof.is_none());
        assert_eq!(
            while_pending.class(ResourceClass::WireBytes).current,
            pending_bytes as u64
        );
        assert_eq!(while_pending.active_leases, 1);
        assert_eq!(after_shutdown.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(after_shutdown.active_leases, 0);
    }

    #[tokio::test]
    async fn clean_eof_before_a_prefix_returns_none() {
        // Arrange
        let (left, right) = duplex(64);
        let receiver = TokioStreamTransport::from_io(right, codec(128));
        drop(left);

        // Act
        let outcome = receiver.receive().await;

        // Assert
        assert!(matches!(outcome, Ok(None)));
    }

    #[tokio::test]
    async fn oversized_declared_envelope_is_rejected_before_body_read() {
        // Arrange
        let (mut left, right) = duplex(64);
        let receiver = TokioStreamTransport::from_io(right, codec(128));
        left.write_all(&129u64.to_be_bytes())
            .await
            .expect("prefix should write");

        // Act
        let outcome = receiver.receive().await;

        // Assert
        assert!(matches!(
            outcome,
            Err(TokioTransportError::DeclaredEnvelopeTooLarge {
                declared: 129,
                limit: 128,
            })
        ));
    }

    #[tokio::test]
    async fn malformed_envelope_is_reported_as_a_frame_error() {
        // Arrange
        let (mut left, right) = duplex(64);
        let ledger = wire_ledger(128);
        let receiver =
            TokioStreamTransport::from_io(right, codec(128)).with_resource_ledger(ledger.clone());
        let malformed = [0u8; 4];
        left.write_all(&(malformed.len() as u64).to_be_bytes())
            .await
            .expect("prefix should write");
        left.write_all(&malformed).await.expect("body should write");

        // Act
        let outcome = receiver.receive().await;
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert!(matches!(
            outcome,
            Err(TokioTransportError::Frame(StreamFrameError::InvalidMagic(
                _
            )))
        ));
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(metrics.class(ResourceClass::WireBytes).peak, 4);
        assert_eq!(metrics.active_leases, 0);
    }

    #[tokio::test]
    async fn explicit_shutdown_produces_clean_peer_eof() {
        // Arrange
        let (left, right) = duplex(64);
        let sender = Arc::new(TokioStreamTransport::from_io(left, codec(128)));
        let receiver = TokioStreamTransport::from_io(right, codec(128));

        // Act
        ShutdownFrameTransport::shutdown(&sender)
            .await
            .expect("writer should shut down");
        let outcome = receiver.receive().await;

        // Assert
        assert!(matches!(outcome, Ok(None)));
    }
}
