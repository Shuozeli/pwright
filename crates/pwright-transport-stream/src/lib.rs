//! Canonical binary envelopes for transporting `WireFrame` over byte streams.
//!
//! This crate performs no I/O and has no async-runtime dependency. Stream
//! adapters must length-prefix each encoded envelope and reject that declared
//! outer length before allocating a receive buffer.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_core::{CallId, EventTag, MethodTag, ObjectId, ProtocolVersion, ResourceLedger};
use pwright_transport::{FrameError, FrameLimits, FrameRoute, WireFrame};

pub const STREAM_FRAME_MAGIC: [u8; 4] = *b"PWRF";
pub const STREAM_FRAME_FORMAT_VERSION: u8 = 1;
pub const STREAM_LENGTH_PREFIX_BYTES: usize = 8;
const FIXED_HEADER_BYTES: usize = 44;
const ATTACHMENT_LENGTH_BYTES: usize = 8;

/// Exact maxima for one concurrent send and one concurrent receive.
///
/// Outbound retention includes the caller's budgeted `WireFrame`, the encoded
/// envelope, and the length-prefixed pending write. Inbound retention includes
/// the received envelope and its decoded budgeted frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamRetentionBounds {
    outbound_wire_bytes: u64,
    inbound_wire_bytes: u64,
    full_duplex_wire_bytes: u64,
    scopes: usize,
    leases: usize,
}

impl StreamRetentionBounds {
    pub const fn outbound_wire_bytes(self) -> u64 {
        self.outbound_wire_bytes
    }

    pub const fn inbound_wire_bytes(self) -> u64 {
        self.inbound_wire_bytes
    }

    pub const fn full_duplex_wire_bytes(self) -> u64 {
        self.full_duplex_wire_bytes
    }

    pub const fn scopes(self) -> usize {
        self.scopes
    }

    pub const fn leases(self) -> usize {
        self.leases
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamFrameLimits {
    frame: FrameLimits,
    envelope_bytes: usize,
}

impl StreamFrameLimits {
    pub const fn new(frame: FrameLimits, envelope_bytes: usize) -> Self {
        Self {
            frame,
            envelope_bytes,
        }
    }

    pub const fn frame(self) -> FrameLimits {
        self.frame
    }

    pub const fn envelope_bytes(self) -> usize {
        self.envelope_bytes
    }

    /// Computes the attainable retained-byte peak instead of adding unrelated
    /// individual maxima. Attachment-header growth and the shrinking envelope
    /// body room are optimized together.
    pub fn retention_bounds(self) -> Result<StreamRetentionBounds, StreamFrameError> {
        if self.envelope_bytes < FIXED_HEADER_BYTES {
            return Err(StreamFrameError::EnvelopeTooLarge {
                actual: FIXED_HEADER_BYTES,
                limit: self.envelope_bytes,
            });
        }
        let frame_bytes = self.max_weighted_retention(0, 1)?;
        let outbound = u128::try_from(STREAM_LENGTH_PREFIX_BYTES)
            .map_err(|_| StreamFrameError::LengthOverflow)?
            .checked_add(self.max_weighted_retention(2, 3)?)
            .ok_or(StreamFrameError::LengthOverflow)?;
        let inbound = self.max_weighted_retention(1, 2)?;
        let full_duplex = outbound
            .checked_add(inbound)
            .ok_or(StreamFrameError::LengthOverflow)?;
        let outbound_wire_bytes =
            u64::try_from(outbound).map_err(|_| StreamFrameError::LengthOverflow)?;
        let inbound_wire_bytes =
            u64::try_from(inbound).map_err(|_| StreamFrameError::LengthOverflow)?;
        let full_duplex_wire_bytes =
            u64::try_from(full_duplex).map_err(|_| StreamFrameError::LengthOverflow)?;
        let has_frame_lease = frame_bytes != 0;
        Ok(StreamRetentionBounds {
            outbound_wire_bytes,
            inbound_wire_bytes,
            full_duplex_wire_bytes,
            scopes: 1 + usize::from(has_frame_lease) * 2,
            leases: 3 + usize::from(has_frame_lease) * 2,
        })
    }

    fn max_weighted_retention(
        self,
        header_weight: u128,
        data_weight: u128,
    ) -> Result<u128, StreamFrameError> {
        let envelope = self.envelope_bytes as u128;
        let fixed = FIXED_HEADER_BYTES as u128;
        let attachment_header = ATTACHMENT_LENGTH_BYTES as u128;
        let attachment_limit =
            (self.frame.attachments as u128).min((envelope - fixed) / attachment_header);
        let payload = self.frame.payload_bytes as u128;
        let attachment_bytes = self.frame.attachment_bytes as u128;
        let total = self.frame.total_bytes as u128;

        let value_at = |attachments: u128| {
            let header = fixed + attachment_header * attachments;
            let data_capacity =
                payload.saturating_add(attachment_bytes.saturating_mul(attachments));
            let data = total.min(data_capacity).min(envelope - header);
            header_weight * header + data_weight * data
        };
        let body_grows_past_room = |attachments: u128| {
            let header = fixed + attachment_header * attachments;
            total.min(payload.saturating_add(attachment_bytes.saturating_mul(attachments)))
                > envelope - header
        };

        let mut low = 0;
        let mut high = attachment_limit;
        while low < high {
            let middle = low + (high - low) / 2;
            if body_grows_past_room(middle) {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        let first_decreasing = low;
        let mut maximum = value_at(attachment_limit);
        maximum = maximum.max(value_at(first_decreasing));
        if first_decreasing > 0 {
            maximum = maximum.max(value_at(first_decreasing - 1));
        }
        Ok(maximum)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StreamFrameError {
    Frame(FrameError),
    EnvelopeTooLarge {
        actual: usize,
        limit: usize,
    },
    InvalidMagic([u8; 4]),
    UnsupportedFormatVersion(u8),
    NonZeroReserved(u16),
    UnknownRoute(u8),
    NonCanonicalRoute(u8),
    Truncated {
        offset: usize,
        needed: usize,
        remaining: usize,
    },
    LengthOutOfRange {
        field: &'static str,
        declared: u64,
    },
    LengthOverflow,
    AllocationFailed {
        bytes: usize,
    },
    TrailingBytes {
        remaining: usize,
    },
}

impl Display for StreamFrameError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame(error) => Display::fmt(error, formatter),
            Self::EnvelopeTooLarge { actual, limit } => {
                write!(
                    formatter,
                    "stream envelope has {actual} bytes, exceeding limit {limit}"
                )
            }
            Self::InvalidMagic(actual) => {
                write!(formatter, "invalid stream frame magic {:02x?}", actual)
            }
            Self::UnsupportedFormatVersion(version) => {
                write!(
                    formatter,
                    "unsupported stream frame format version {version}"
                )
            }
            Self::NonZeroReserved(value) => {
                write!(
                    formatter,
                    "stream frame reserved bits are non-zero: {value}"
                )
            }
            Self::UnknownRoute(route) => write!(formatter, "unknown stream frame route {route}"),
            Self::NonCanonicalRoute(route) => {
                write!(
                    formatter,
                    "stream frame route {route} has non-zero unused fields"
                )
            }
            Self::Truncated {
                offset,
                needed,
                remaining,
            } => write!(
                formatter,
                "stream frame at byte {offset} needs {needed} bytes but only {remaining} remain"
            ),
            Self::LengthOutOfRange { field, declared } => {
                write!(
                    formatter,
                    "declared {field} length {declared} does not fit this platform"
                )
            }
            Self::LengthOverflow => formatter.write_str("stream frame length overflowed"),
            Self::AllocationFailed { bytes } => {
                write!(formatter, "could not reserve {bytes} stream frame bytes")
            }
            Self::TrailingBytes { remaining } => {
                write!(formatter, "stream frame has {remaining} trailing bytes")
            }
        }
    }
}

impl Error for StreamFrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Frame(error) => Some(error),
            Self::EnvelopeTooLarge { .. }
            | Self::InvalidMagic(_)
            | Self::UnsupportedFormatVersion(_)
            | Self::NonZeroReserved(_)
            | Self::UnknownRoute(_)
            | Self::NonCanonicalRoute(_)
            | Self::Truncated { .. }
            | Self::LengthOutOfRange { .. }
            | Self::LengthOverflow
            | Self::AllocationFailed { .. }
            | Self::TrailingBytes { .. } => None,
        }
    }
}

impl From<FrameError> for StreamFrameError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StreamFrameCodec {
    limits: StreamFrameLimits,
}

impl StreamFrameCodec {
    pub const fn new(limits: StreamFrameLimits) -> Self {
        Self { limits }
    }

    pub const fn limits(self) -> StreamFrameLimits {
        self.limits
    }

    /// Returns the exact encoded envelope length after validating all frame
    /// and envelope limits, without allocating the envelope.
    pub fn encoded_len(&self, frame: &WireFrame) -> Result<usize, StreamFrameError> {
        frame.validate(self.limits.frame())?;
        let attachment_lengths_bytes = frame
            .attachments()
            .len()
            .checked_mul(ATTACHMENT_LENGTH_BYTES)
            .ok_or(StreamFrameError::LengthOverflow)?;
        let envelope_bytes = FIXED_HEADER_BYTES
            .checked_add(attachment_lengths_bytes)
            .and_then(|length| length.checked_add(frame.total_bytes()))
            .ok_or(StreamFrameError::LengthOverflow)?;
        if envelope_bytes > self.limits.envelope_bytes() {
            return Err(StreamFrameError::EnvelopeTooLarge {
                actual: envelope_bytes,
                limit: self.limits.envelope_bytes(),
            });
        }
        Ok(envelope_bytes)
    }

    pub fn encode(&self, frame: &WireFrame) -> Result<Vec<u8>, StreamFrameError> {
        let envelope_bytes = self.encoded_len(frame)?;
        let payload_length = u64::try_from(frame.payload().len()).map_err(|_| {
            StreamFrameError::LengthOutOfRange {
                field: "payload",
                declared: u64::MAX,
            }
        })?;
        let attachment_count = u32::try_from(frame.attachments().len()).map_err(|_| {
            StreamFrameError::LengthOutOfRange {
                field: "attachment count",
                declared: u64::MAX,
            }
        })?;
        let (route_tag, route_a, route_b, route_c) = encode_route(frame.route());
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(envelope_bytes).map_err(|_| {
            StreamFrameError::AllocationFailed {
                bytes: envelope_bytes,
            }
        })?;
        bytes.extend_from_slice(&STREAM_FRAME_MAGIC);
        bytes.push(STREAM_FRAME_FORMAT_VERSION);
        bytes.push(route_tag);
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&frame.version().major.to_be_bytes());
        bytes.extend_from_slice(&frame.version().minor.to_be_bytes());
        bytes.extend_from_slice(&route_a.to_be_bytes());
        bytes.extend_from_slice(&route_b.to_be_bytes());
        bytes.extend_from_slice(&route_c.to_be_bytes());
        bytes.extend_from_slice(&payload_length.to_be_bytes());
        bytes.extend_from_slice(&attachment_count.to_be_bytes());
        for attachment in frame.attachments() {
            let length = u64::try_from(attachment.len()).map_err(|_| {
                StreamFrameError::LengthOutOfRange {
                    field: "attachment",
                    declared: u64::MAX,
                }
            })?;
            bytes.extend_from_slice(&length.to_be_bytes());
        }
        bytes.extend_from_slice(frame.payload());
        for attachment in frame.attachments() {
            bytes.extend_from_slice(attachment.as_bytes());
        }
        debug_assert_eq!(bytes.len(), envelope_bytes);
        Ok(bytes)
    }

    pub fn decode(&self, bytes: &[u8]) -> Result<WireFrame, StreamFrameError> {
        self.decode_inner(bytes, None)
    }

    /// Decodes an envelope while reserving retained frame bytes before any
    /// payload or attachment copy is allocated.
    pub fn decode_budgeted(
        &self,
        bytes: &[u8],
        ledger: &ResourceLedger,
    ) -> Result<WireFrame, StreamFrameError> {
        self.decode_inner(bytes, Some(ledger))
    }

    fn decode_inner(
        &self,
        bytes: &[u8],
        ledger: Option<&ResourceLedger>,
    ) -> Result<WireFrame, StreamFrameError> {
        if bytes.len() > self.limits.envelope_bytes() {
            return Err(StreamFrameError::EnvelopeTooLarge {
                actual: bytes.len(),
                limit: self.limits.envelope_bytes(),
            });
        }
        let mut reader = Reader::new(bytes);
        let magic = reader.array::<4>()?;
        if magic != STREAM_FRAME_MAGIC {
            return Err(StreamFrameError::InvalidMagic(magic));
        }
        let format_version = reader.u8()?;
        if format_version != STREAM_FRAME_FORMAT_VERSION {
            return Err(StreamFrameError::UnsupportedFormatVersion(format_version));
        }
        let route_tag = reader.u8()?;
        let reserved = reader.u16()?;
        if reserved != 0 {
            return Err(StreamFrameError::NonZeroReserved(reserved));
        }
        let version = ProtocolVersion::new(reader.u16()?, reader.u16()?);
        let route_a = reader.u64()?;
        let route_b = reader.u64()?;
        let route_c = reader.u32()?;
        let route = decode_route(route_tag, route_a, route_b, route_c)?;
        let payload_length_u64 = reader.u64()?;
        let payload_length = usize::try_from(payload_length_u64).map_err(|_| {
            StreamFrameError::LengthOutOfRange {
                field: "payload",
                declared: payload_length_u64,
            }
        })?;
        if payload_length > self.limits.frame().payload_bytes {
            return Err(FrameError::PayloadTooLarge {
                actual: payload_length,
                limit: self.limits.frame().payload_bytes,
            }
            .into());
        }
        let attachment_count = reader.u32()? as usize;
        if attachment_count > self.limits.frame().attachments {
            return Err(FrameError::TooManyAttachments {
                actual: attachment_count,
                limit: self.limits.frame().attachments,
            }
            .into());
        }
        let mut attachment_lengths = Vec::new();
        attachment_lengths
            .try_reserve_exact(attachment_count)
            .map_err(|_| StreamFrameError::AllocationFailed {
                bytes: attachment_count.saturating_mul(size_of::<usize>()),
            })?;
        let mut total_bytes = payload_length;
        for index in 0..attachment_count {
            let declared = reader.u64()?;
            let length =
                usize::try_from(declared).map_err(|_| StreamFrameError::LengthOutOfRange {
                    field: "attachment",
                    declared,
                })?;
            if length > self.limits.frame().attachment_bytes {
                return Err(FrameError::AttachmentTooLarge {
                    index,
                    actual: length,
                    limit: self.limits.frame().attachment_bytes,
                }
                .into());
            }
            total_bytes = total_bytes
                .checked_add(length)
                .ok_or(StreamFrameError::LengthOverflow)?;
            attachment_lengths.push(length);
        }
        if total_bytes > self.limits.frame().total_bytes {
            return Err(FrameError::TotalBytesTooLarge {
                actual: total_bytes,
                limit: self.limits.frame().total_bytes,
            }
            .into());
        }
        if reader.remaining() < total_bytes {
            return Err(StreamFrameError::Truncated {
                offset: reader.offset,
                needed: total_bytes,
                remaining: reader.remaining(),
            });
        }
        if reader.remaining() > total_bytes {
            return Err(StreamFrameError::TrailingBytes {
                remaining: reader.remaining() - total_bytes,
            });
        }
        let reservation = ledger
            .map(|ledger| WireFrame::reserve_bytes(ledger, route.resource_scope(), total_bytes))
            .transpose()?;
        let payload = copy_bytes(reader.bytes(payload_length)?)?;
        let mut attachments = Vec::new();
        attachments
            .try_reserve_exact(attachment_count)
            .map_err(|_| StreamFrameError::AllocationFailed {
                bytes: attachment_count.saturating_mul(size_of::<Vec<u8>>()),
            })?;
        for length in attachment_lengths {
            attachments.push(copy_bytes(reader.bytes(length)?)?);
        }
        debug_assert_eq!(reader.remaining(), 0);
        match reservation {
            Some(reservation) => WireFrame::new_reserved(
                version,
                route,
                payload,
                attachments,
                self.limits.frame(),
                reservation,
            ),
            None => WireFrame::new(version, route, payload, attachments, self.limits.frame()),
        }
        .map_err(StreamFrameError::Frame)
    }
}

fn encode_route(route: FrameRoute) -> (u8, u64, u64, u32) {
    match route {
        FrameRoute::Handshake => (0, 0, 0, 0),
        FrameRoute::Command {
            call_id,
            target,
            method,
        } => (1, call_id.get(), target.get(), method.get()),
        FrameRoute::Result { call_id } => (2, call_id.get(), 0, 0),
        FrameRoute::Event { source, event } => (3, source.get(), 0, event.get()),
        FrameRoute::Control => (4, 0, 0, 0),
    }
}

fn decode_route(
    route: u8,
    route_a: u64,
    route_b: u64,
    route_c: u32,
) -> Result<FrameRoute, StreamFrameError> {
    match route {
        0 if route_a == 0 && route_b == 0 && route_c == 0 => Ok(FrameRoute::Handshake),
        1 => Ok(FrameRoute::Command {
            call_id: CallId::new(route_a),
            target: ObjectId::new(route_b),
            method: MethodTag::new(route_c),
        }),
        2 if route_b == 0 && route_c == 0 => Ok(FrameRoute::Result {
            call_id: CallId::new(route_a),
        }),
        3 if route_b == 0 => Ok(FrameRoute::Event {
            source: ObjectId::new(route_a),
            event: EventTag::new(route_c),
        }),
        4 if route_a == 0 && route_b == 0 && route_c == 0 => Ok(FrameRoute::Control),
        0 | 2 | 3 | 4 => Err(StreamFrameError::NonCanonicalRoute(route)),
        _ => Err(StreamFrameError::UnknownRoute(route)),
    }
}

fn copy_bytes(bytes: &[u8]) -> Result<Vec<u8>, StreamFrameError> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())
        .map_err(|_| StreamFrameError::AllocationFailed { bytes: bytes.len() })?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn bytes(&mut self, length: usize) -> Result<&'a [u8], StreamFrameError> {
        let remaining = self.remaining();
        if length > remaining {
            return Err(StreamFrameError::Truncated {
                offset: self.offset,
                needed: length,
                remaining,
            });
        }
        let start = self.offset;
        self.offset += length;
        Ok(&self.bytes[start..self.offset])
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], StreamFrameError> {
        self.bytes(N)?
            .try_into()
            .map_err(|_| StreamFrameError::Truncated {
                offset: self.offset,
                needed: N,
                remaining: 0,
            })
    }

    fn u8(&mut self) -> Result<u8, StreamFrameError> {
        Ok(self.array::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, StreamFrameError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, StreamFrameError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, StreamFrameError> {
        Ok(u64::from_be_bytes(self.array()?))
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{
        CallId, EventTag, MethodTag, ObjectId, ProtocolVersion, ResourceClass, ResourceError,
        ResourceLedger, ResourceLimits, ResourceScope,
    };
    use pwright_transport::{FrameError, FrameLimits, FrameRoute, WireFrame};

    use super::{
        STREAM_FRAME_FORMAT_VERSION, StreamFrameCodec, StreamFrameError, StreamFrameLimits,
    };

    fn limits() -> StreamFrameLimits {
        StreamFrameLimits::new(FrameLimits::new(16, 2, 8, 24), 128)
    }

    fn frame(route: FrameRoute) -> WireFrame {
        WireFrame::new(
            ProtocolVersion::new(1, 2),
            route,
            vec![1, 2, 3],
            vec![vec![4, 5], vec![6]],
            limits().frame(),
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

    #[test]
    fn retention_bounds_measure_attainable_full_duplex_overlap() {
        // Arrange
        let limits = StreamFrameLimits::new(FrameLimits::new(16, 2, 8, 24), 128);

        // Act
        let bounds = limits
            .retention_bounds()
            .expect("stream limits should admit an empty frame");

        // Assert
        assert_eq!(bounds.outbound_wire_bytes(), 200);
        assert_eq!(bounds.inbound_wire_bytes(), 108);
        assert_eq!(bounds.full_duplex_wire_bytes(), 308);
        assert_eq!(bounds.scopes(), 3);
        assert_eq!(bounds.leases(), 5);
    }

    #[test]
    fn retention_bounds_reject_an_envelope_smaller_than_its_fixed_header() {
        // Arrange
        let limits = StreamFrameLimits::new(FrameLimits::new(0, 0, 0, 0), 43);

        // Act
        let error = limits
            .retention_bounds()
            .expect_err("no canonical frame fits below the fixed header");

        // Assert
        assert_eq!(
            error,
            StreamFrameError::EnvelopeTooLarge {
                actual: 44,
                limit: 43,
            }
        );
    }

    #[test]
    fn retention_bounds_choose_fewer_attachments_when_headers_reduce_body_room() {
        // Arrange
        let limits = StreamFrameLimits::new(FrameLimits::new(100, 10, 100, 100), 60);

        // Act
        let bounds = limits
            .retention_bounds()
            .expect("a header plus sixteen payload bytes should fit");

        // Assert
        assert_eq!(bounds.outbound_wire_bytes(), 144);
        assert_eq!(bounds.inbound_wire_bytes(), 76);
        assert_eq!(bounds.full_duplex_wire_bytes(), 220);
    }

    #[test]
    fn every_route_round_trips_with_payload_and_attachments() {
        // Arrange
        let codec = StreamFrameCodec::new(limits());
        let routes = [
            FrameRoute::Handshake,
            FrameRoute::Command {
                call_id: CallId::new(7),
                target: ObjectId::new(8),
                method: MethodTag::new(9),
            },
            FrameRoute::Result {
                call_id: CallId::new(7),
            },
            FrameRoute::Event {
                source: ObjectId::new(8),
                event: EventTag::new(10),
            },
            FrameRoute::Control,
        ];

        // Act
        let decoded = routes
            .into_iter()
            .map(|route| {
                let expected = frame(route);
                let bytes = codec.encode(&expected).expect("frame should encode");
                let actual = codec.decode(&bytes).expect("frame should decode");
                (expected, actual)
            })
            .collect::<Vec<_>>();

        // Assert
        for (expected, actual) in decoded {
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn budgeted_decode_reserves_before_copy_and_attributes_retained_frame() {
        // Arrange
        let codec = StreamFrameCodec::new(limits());
        let expected = frame(FrameRoute::Command {
            call_id: CallId::new(7),
            target: ObjectId::new(8),
            method: MethodTag::new(9),
        });
        let bytes = codec.encode(&expected).expect("frame should encode");
        let ledger = wire_ledger(6);

        // Act
        let decoded = codec
            .decode_budgeted(&bytes, &ledger)
            .expect("six retained bytes should fit");
        let retained = ledger
            .scope_metrics(ResourceScope::Call(CallId::new(7)))
            .expect("metrics should be available")
            .expect("call scope should be retained");
        drop(decoded);
        let released = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(retained.units(ResourceClass::WireBytes), 6);
        assert_eq!(retained.active_leases, 1);
        assert_eq!(released.class(ResourceClass::WireBytes).current, 0);
    }

    #[test]
    fn budgeted_decode_rejects_before_retained_byte_allocation() {
        // Arrange
        let codec = StreamFrameCodec::new(limits());
        let bytes = codec
            .encode(&frame(FrameRoute::Control))
            .expect("frame should encode");
        let ledger = wire_ledger(5);

        // Act
        let error = codec
            .decode_budgeted(&bytes, &ledger)
            .expect_err("six retained bytes should exceed the shared budget");
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(
            error,
            StreamFrameError::Frame(FrameError::Resource(ResourceError::GlobalLimitExceeded {
                class: ResourceClass::WireBytes,
                current: 0,
                requested: 6,
                limit: 5,
            }))
        );
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(metrics.class(ResourceClass::WireBytes).peak, 0);
        assert_eq!(metrics.rejected_reservations, 1);
    }

    #[test]
    fn envelope_limit_is_checked_on_encode_and_decode() {
        // Arrange
        let frame = frame(FrameRoute::Control);
        let roomy = StreamFrameCodec::new(limits());
        let bytes = roomy.encode(&frame).expect("frame should encode");
        let tight_limits = StreamFrameLimits::new(limits().frame(), bytes.len() - 1);
        let tight = StreamFrameCodec::new(tight_limits);

        // Act
        let encode_error = tight.encode(&frame);
        let decode_error = tight.decode(&bytes);

        // Assert
        let expected = StreamFrameError::EnvelopeTooLarge {
            actual: bytes.len(),
            limit: bytes.len() - 1,
        };
        assert_eq!(encode_error, Err(expected.clone()));
        assert_eq!(decode_error, Err(expected));
    }

    #[test]
    fn oversized_declared_payload_is_rejected_before_copying_body() {
        // Arrange
        let codec = StreamFrameCodec::new(limits());
        let mut bytes = codec
            .encode(&frame(FrameRoute::Control))
            .expect("frame should encode");
        bytes[32..40].copy_from_slice(&17u64.to_be_bytes());

        // Act
        let outcome = codec.decode(&bytes);

        // Assert
        assert_eq!(
            outcome,
            Err(StreamFrameError::Frame(FrameError::PayloadTooLarge {
                actual: 17,
                limit: 16,
            }))
        );
    }

    #[test]
    fn attachment_count_is_rejected_before_length_table_allocation() {
        // Arrange
        let codec = StreamFrameCodec::new(limits());
        let mut bytes = codec
            .encode(&frame(FrameRoute::Control))
            .expect("frame should encode");
        bytes[40..44].copy_from_slice(&3u32.to_be_bytes());

        // Act
        let outcome = codec.decode(&bytes);

        // Assert
        assert_eq!(
            outcome,
            Err(StreamFrameError::Frame(FrameError::TooManyAttachments {
                actual: 3,
                limit: 2,
            }))
        );
    }

    #[test]
    fn truncated_and_trailing_envelopes_are_rejected() {
        // Arrange
        let codec = StreamFrameCodec::new(limits());
        let bytes = codec
            .encode(&frame(FrameRoute::Control))
            .expect("frame should encode");
        let truncated = &bytes[..bytes.len() - 1];
        let mut trailing = bytes.clone();
        trailing.push(0);

        // Act
        let truncated_error = codec.decode(truncated);
        let trailing_error = codec.decode(&trailing);

        // Assert
        assert!(matches!(
            truncated_error,
            Err(StreamFrameError::Truncated { .. })
        ));
        assert_eq!(
            trailing_error,
            Err(StreamFrameError::TrailingBytes { remaining: 1 })
        );
    }

    #[test]
    fn malformed_header_and_noncanonical_route_are_rejected() {
        // Arrange
        let codec = StreamFrameCodec::new(limits());
        let canonical = codec
            .encode(&frame(FrameRoute::Control))
            .expect("frame should encode");
        let mut bad_magic = canonical.clone();
        bad_magic[0] = b'X';
        let mut bad_version = canonical.clone();
        bad_version[4] = STREAM_FRAME_FORMAT_VERSION + 1;
        let mut bad_route = canonical;
        bad_route[5] = 4;
        bad_route[12..20].copy_from_slice(&1u64.to_be_bytes());

        // Act
        let magic_error = codec.decode(&bad_magic);
        let version_error = codec.decode(&bad_version);
        let route_error = codec.decode(&bad_route);

        // Assert
        assert!(matches!(
            magic_error,
            Err(StreamFrameError::InvalidMagic(_))
        ));
        assert_eq!(
            version_error,
            Err(StreamFrameError::UnsupportedFormatVersion(
                STREAM_FRAME_FORMAT_VERSION + 1
            ))
        );
        assert_eq!(route_error, Err(StreamFrameError::NonCanonicalRoute(4)));
    }
}
