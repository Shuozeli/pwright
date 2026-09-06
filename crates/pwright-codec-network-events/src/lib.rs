//! Canonical bounded binary codec for one globally ordered HTTP lifecycle stream.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::Utf8Error;

use pwright_cap_network_events::{
    HttpHeader, NetworkEventPayload, NetworkEventStream, NetworkLifecycleLimits, RequestFailed,
    RequestFinished, RequestInitializer, ResponseInitializer, is_network_event_tag,
};
use pwright_codec_binary::{BinaryDecodeError, BinaryEncodeError, BinaryReader, BinaryWriter};
use pwright_core::{Event, EventSequence, EventTag, ObjectId, SequencedEvent};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const REQUEST_KIND: u8 = 1;
const RESPONSE_KIND: u8 = 2;
const FINISHED_KIND: u8 = 3;
const FAILED_KIND: u8 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetworkEventsCodecLimits {
    pub url_bytes: usize,
    pub method_bytes: usize,
    pub resource_type_bytes: usize,
    pub header_count: usize,
    pub header_bytes: usize,
    pub post_data_bytes: usize,
    pub status_text_bytes: usize,
    pub failure_text_bytes: usize,
}

impl NetworkEventsCodecLimits {
    pub const fn from_lifecycle(limits: NetworkLifecycleLimits) -> Self {
        Self {
            url_bytes: limits.url_bytes,
            method_bytes: limits.method_bytes,
            resource_type_bytes: limits.resource_type_bytes,
            header_count: limits.header_count,
            header_bytes: limits.header_bytes,
            post_data_bytes: limits.post_data_bytes,
            status_text_bytes: limits.status_text_bytes,
            failure_text_bytes: limits.failure_text_bytes,
        }
    }

    /// Maximum dynamic metadata capacity retained by one decoded event.
    /// Terminal failure is the largest variant because it owns complete
    /// Request and optional Response snapshots plus bounded failure text.
    pub const fn maximum_retained_event_bytes(self) -> usize {
        self.url_bytes
            .saturating_mul(2)
            .saturating_add(self.method_bytes)
            .saturating_add(self.resource_type_bytes)
            .saturating_add(self.header_bytes.saturating_mul(2))
            .saturating_add(self.post_data_bytes)
            .saturating_add(self.status_text_bytes)
            .saturating_add(self.failure_text_bytes)
    }

    /// Conservative maximum binary payload emitted by any event variant.
    /// This includes the routed-event sequence/source prefix and every length
    /// or option discriminator, but no transport framing outside the payload.
    pub fn maximum_payload_bytes(self) -> usize {
        let header = self
            .header_count
            .saturating_mul(8)
            .saturating_add(self.header_bytes);
        let request = 47usize
            .saturating_add(self.url_bytes)
            .saturating_add(self.method_bytes)
            .saturating_add(self.resource_type_bytes)
            .saturating_add(header)
            .saturating_add(self.post_data_bytes);
        let response = 37usize
            .saturating_add(self.url_bytes)
            .saturating_add(self.status_text_bytes)
            .saturating_add(header);
        let request_event = 17usize.saturating_add(request);
        let response_event = 17usize.saturating_add(response);
        let finished_event = 27usize.saturating_add(request).saturating_add(response);
        let failed_event = 23usize
            .saturating_add(request)
            .saturating_add(response)
            .saturating_add(self.failure_text_bytes);
        request_event
            .max(response_event)
            .max(finished_event)
            .max(failed_event)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NetworkEventsCodec {
    limits: NetworkEventsCodecLimits,
}

impl NetworkEventsCodec {
    pub const fn new(limits: NetworkEventsCodecLimits) -> Self {
        Self { limits }
    }

    pub const fn limits(self) -> NetworkEventsCodecLimits {
        self.limits
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkEventsCodecError {
    WrongRoute(FrameRoute),
    UnsupportedEventTag(EventTag),
    EventTagMismatch {
        route: EventTag,
        payload: EventTag,
    },
    AttachmentsUnsupported {
        actual: usize,
    },
    InvalidSequence,
    SourceMismatch {
        route: ObjectId,
        payload: ObjectId,
    },
    InvalidPayloadKind(u8),
    InvalidBoolean(u8),
    InvalidOptionFlag(u8),
    FieldLimitExceeded {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    AllocationFailed {
        entries: usize,
    },
    EmptyHeaderName,
    NonCanonicalHeaderOrder,
    InvalidUtf8(Utf8Error),
    Encode(BinaryEncodeError),
    Decode(BinaryDecodeError),
}

impl Display for NetworkEventsCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => write!(formatter, "network event arrived on {route:?}"),
            Self::UnsupportedEventTag(event) => {
                write!(formatter, "unsupported network event tag {event}")
            }
            Self::EventTagMismatch { route, payload } => write!(
                formatter,
                "network route event tag {route} differs from payload tag {payload}"
            ),
            Self::AttachmentsUnsupported { actual } => write!(
                formatter,
                "network event has {actual} attachment(s), but none are allowed"
            ),
            Self::InvalidSequence => formatter.write_str("network event sequence zero is invalid"),
            Self::SourceMismatch { route, payload } => write!(
                formatter,
                "network route source `{route}` differs from payload source `{payload}`"
            ),
            Self::InvalidPayloadKind(kind) => {
                write!(formatter, "invalid network payload kind {kind}")
            }
            Self::InvalidBoolean(value) => write!(formatter, "invalid network boolean {value}"),
            Self::InvalidOptionFlag(value) => {
                write!(formatter, "invalid network option flag {value}")
            }
            Self::FieldLimitExceeded {
                field,
                actual,
                limit,
            } => write!(
                formatter,
                "network {field} uses {actual} bytes/items, exceeding limit {limit}"
            ),
            Self::AllocationFailed { entries } => {
                write!(
                    formatter,
                    "could not allocate {entries} network header entries"
                )
            }
            Self::EmptyHeaderName => formatter.write_str("network header name is empty"),
            Self::NonCanonicalHeaderOrder => {
                formatter.write_str("network headers are not in canonical order")
            }
            Self::InvalidUtf8(error) => write!(formatter, "network string is not UTF-8: {error}"),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::Decode(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for NetworkEventsCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidUtf8(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

impl From<BinaryEncodeError> for NetworkEventsCodecError {
    fn from(error: BinaryEncodeError) -> Self {
        Self::Encode(error)
    }
}

impl From<BinaryDecodeError> for NetworkEventsCodecError {
    fn from(error: BinaryDecodeError) -> Self {
        Self::Decode(error)
    }
}

impl WireCodec<SequencedEvent<NetworkEventStream>> for NetworkEventsCodec {
    type Error = NetworkEventsCodecError;

    fn encode_parts(
        &self,
        event: &SequencedEvent<NetworkEventStream>,
    ) -> Result<EncodedParts, Self::Error> {
        if event.sequence().get() == 0 {
            return Err(NetworkEventsCodecError::InvalidSequence);
        }
        let tag = event.payload().event_tag();
        let mut writer = BinaryWriter::try_with_capacity(64)?;
        writer.u8(payload_kind(event.payload()))?;
        writer.u64(event.sequence().get())?;
        writer.u64(event.object_id().get())?;
        match event.payload() {
            NetworkEventPayload::Request(payload) => self.encode_request(&mut writer, payload)?,
            NetworkEventPayload::Response(payload) => self.encode_response(&mut writer, payload)?,
            NetworkEventPayload::RequestFinished(payload) => {
                self.encode_finished(&mut writer, payload)?
            }
            NetworkEventPayload::RequestFailed(payload) => {
                self.encode_failed(&mut writer, payload)?
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Event {
                source: event.object_id(),
                event: tag,
            },
            writer.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<SequencedEvent<NetworkEventStream>, Self::Error> {
        let FrameRoute::Event { source, event } = frame.route() else {
            return Err(NetworkEventsCodecError::WrongRoute(frame.route()));
        };
        if !is_network_event_tag(event) {
            return Err(NetworkEventsCodecError::UnsupportedEventTag(event));
        }
        if !frame.attachments().is_empty() {
            return Err(NetworkEventsCodecError::AttachmentsUnsupported {
                actual: frame.attachments().len(),
            });
        }
        let mut reader = BinaryReader::new(frame.payload());
        let kind = reader.u8()?;
        let sequence = EventSequence::new(reader.u64()?);
        if sequence.get() == 0 {
            return Err(NetworkEventsCodecError::InvalidSequence);
        }
        let payload_source = ObjectId::new(reader.u64()?);
        if payload_source != source {
            return Err(NetworkEventsCodecError::SourceMismatch {
                route: source,
                payload: payload_source,
            });
        }
        let payload = match kind {
            REQUEST_KIND => NetworkEventPayload::Request(self.decode_request(&mut reader)?),
            RESPONSE_KIND => NetworkEventPayload::Response(self.decode_response(&mut reader)?),
            FINISHED_KIND => {
                NetworkEventPayload::RequestFinished(self.decode_finished(&mut reader)?)
            }
            FAILED_KIND => NetworkEventPayload::RequestFailed(self.decode_failed(&mut reader)?),
            actual => return Err(NetworkEventsCodecError::InvalidPayloadKind(actual)),
        };
        let payload_tag = payload.event_tag();
        if event != payload_tag {
            return Err(NetworkEventsCodecError::EventTagMismatch {
                route: event,
                payload: payload_tag,
            });
        }
        reader.finish()?;
        Ok(SequencedEvent::new(sequence, Event::new(source, payload)))
    }
}

impl NetworkEventsCodec {
    fn encode_request(
        &self,
        writer: &mut BinaryWriter,
        value: &RequestInitializer,
    ) -> Result<(), NetworkEventsCodecError> {
        writer.u64(value.request.get())?;
        writer.u64(value.parent.get())?;
        writer.u8(u8::from(value.is_navigation_request))?;
        encode_optional_object(writer, value.redirected_from)?;
        encode_string(writer, "URL", &value.url, self.limits.url_bytes)?;
        encode_string(writer, "method", &value.method, self.limits.method_bytes)?;
        encode_string(
            writer,
            "resource type",
            &value.resource_type,
            self.limits.resource_type_bytes,
        )?;
        encode_headers(writer, &value.headers, self.limits)?;
        encode_optional_bytes(
            writer,
            "post data",
            value.post_data.as_deref(),
            self.limits.post_data_bytes,
        )
    }

    fn decode_request(
        &self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<RequestInitializer, NetworkEventsCodecError> {
        Ok(RequestInitializer {
            request: ObjectId::new(reader.u64()?),
            parent: ObjectId::new(reader.u64()?),
            is_navigation_request: decode_bool(reader.u8()?)?,
            redirected_from: decode_optional_object(reader)?,
            url: decode_string(reader, "URL", self.limits.url_bytes)?,
            method: decode_string(reader, "method", self.limits.method_bytes)?,
            resource_type: decode_string(reader, "resource type", self.limits.resource_type_bytes)?,
            headers: decode_headers(reader, self.limits)?,
            post_data: decode_optional_bytes(reader, "post data", self.limits.post_data_bytes)?,
        })
    }

    fn encode_response(
        &self,
        writer: &mut BinaryWriter,
        value: &ResponseInitializer,
    ) -> Result<(), NetworkEventsCodecError> {
        writer.u64(value.response.get())?;
        writer.u64(value.request.get())?;
        writer.i64(value.status)?;
        writer.u8(u8::from(value.from_service_worker))?;
        encode_string(writer, "URL", &value.url, self.limits.url_bytes)?;
        encode_string(
            writer,
            "status text",
            &value.status_text,
            self.limits.status_text_bytes,
        )?;
        encode_headers(writer, &value.headers, self.limits)
    }

    fn decode_response(
        &self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<ResponseInitializer, NetworkEventsCodecError> {
        Ok(ResponseInitializer {
            response: ObjectId::new(reader.u64()?),
            request: ObjectId::new(reader.u64()?),
            status: reader.i64()?,
            from_service_worker: decode_bool(reader.u8()?)?,
            url: decode_string(reader, "URL", self.limits.url_bytes)?,
            status_text: decode_string(reader, "status text", self.limits.status_text_bytes)?,
            headers: decode_headers(reader, self.limits)?,
        })
    }

    fn encode_failed(
        &self,
        writer: &mut BinaryWriter,
        value: &RequestFailed,
    ) -> Result<(), NetworkEventsCodecError> {
        self.encode_request(writer, &value.request)?;
        self.encode_optional_response(writer, value.response.as_ref())?;
        writer.u8(u8::from(value.canceled))?;
        encode_string(
            writer,
            "failure text",
            &value.failure_text,
            self.limits.failure_text_bytes,
        )
    }

    fn decode_failed(
        &self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<RequestFailed, NetworkEventsCodecError> {
        Ok(RequestFailed {
            request: self.decode_request(reader)?,
            response: self.decode_optional_response(reader)?,
            canceled: decode_bool(reader.u8()?)?,
            failure_text: decode_string(reader, "failure text", self.limits.failure_text_bytes)?,
        })
    }

    fn encode_finished(
        &self,
        writer: &mut BinaryWriter,
        value: &RequestFinished,
    ) -> Result<(), NetworkEventsCodecError> {
        self.encode_request(writer, &value.request)?;
        self.encode_optional_response(writer, value.response.as_ref())?;
        encode_optional_u64(writer, value.encoded_body_bytes)
    }

    fn decode_finished(
        &self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<RequestFinished, NetworkEventsCodecError> {
        Ok(RequestFinished {
            request: self.decode_request(reader)?,
            response: self.decode_optional_response(reader)?,
            encoded_body_bytes: decode_optional_u64(reader)?,
        })
    }

    fn encode_optional_response(
        &self,
        writer: &mut BinaryWriter,
        value: Option<&ResponseInitializer>,
    ) -> Result<(), NetworkEventsCodecError> {
        match value {
            Some(value) => {
                writer.u8(1)?;
                self.encode_response(writer, value)
            }
            None => writer.u8(0).map_err(NetworkEventsCodecError::Encode),
        }
    }

    fn decode_optional_response(
        &self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<Option<ResponseInitializer>, NetworkEventsCodecError> {
        match reader.u8()? {
            0 => Ok(None),
            1 => self.decode_response(reader).map(Some),
            value => Err(NetworkEventsCodecError::InvalidOptionFlag(value)),
        }
    }
}

fn payload_kind(payload: &NetworkEventPayload) -> u8 {
    match payload {
        NetworkEventPayload::Request(_) => REQUEST_KIND,
        NetworkEventPayload::Response(_) => RESPONSE_KIND,
        NetworkEventPayload::RequestFinished(_) => FINISHED_KIND,
        NetworkEventPayload::RequestFailed(_) => FAILED_KIND,
    }
}

fn encode_headers(
    writer: &mut BinaryWriter,
    headers: &[HttpHeader],
    limits: NetworkEventsCodecLimits,
) -> Result<(), NetworkEventsCodecError> {
    if headers.len() > limits.header_count {
        return Err(NetworkEventsCodecError::FieldLimitExceeded {
            field: "header count",
            actual: headers.len(),
            limit: limits.header_count,
        });
    }
    let mut bytes = 0usize;
    let mut previous: Option<&str> = None;
    for header in headers {
        if header.name.is_empty() {
            return Err(NetworkEventsCodecError::EmptyHeaderName);
        }
        if previous.is_some_and(|name| name > header.name.as_str()) {
            return Err(NetworkEventsCodecError::NonCanonicalHeaderOrder);
        }
        previous = Some(&header.name);
        bytes = bytes
            .saturating_add(header.name.len())
            .saturating_add(header.value.len());
    }
    if bytes > limits.header_bytes {
        return Err(NetworkEventsCodecError::FieldLimitExceeded {
            field: "header bytes",
            actual: bytes,
            limit: limits.header_bytes,
        });
    }
    writer.count_u32("header count", headers.len())?;
    for header in headers {
        encode_string(writer, "header name", &header.name, limits.header_bytes)?;
        encode_string(writer, "header value", &header.value, limits.header_bytes)?;
    }
    Ok(())
}

fn decode_headers(
    reader: &mut BinaryReader<'_>,
    limits: NetworkEventsCodecLimits,
) -> Result<Vec<HttpHeader>, NetworkEventsCodecError> {
    let count = reader.count_u32("header count")?;
    if count > limits.header_count {
        return Err(NetworkEventsCodecError::FieldLimitExceeded {
            field: "header count",
            actual: count,
            limit: limits.header_count,
        });
    }
    reader.ensure_count(count, 8)?;
    let mut headers = Vec::new();
    headers
        .try_reserve_exact(count)
        .map_err(|_| NetworkEventsCodecError::AllocationFailed { entries: count })?;
    let mut bytes = 0usize;
    for _ in 0..count {
        let name = decode_string(reader, "header name", limits.header_bytes)?;
        if name.is_empty() {
            return Err(NetworkEventsCodecError::EmptyHeaderName);
        }
        let value = decode_string(reader, "header value", limits.header_bytes)?;
        bytes = bytes.saturating_add(name.len()).saturating_add(value.len());
        if bytes > limits.header_bytes {
            return Err(NetworkEventsCodecError::FieldLimitExceeded {
                field: "header bytes",
                actual: bytes,
                limit: limits.header_bytes,
            });
        }
        if headers
            .last()
            .is_some_and(|previous: &HttpHeader| previous.name > name)
        {
            return Err(NetworkEventsCodecError::NonCanonicalHeaderOrder);
        }
        headers.push(HttpHeader::new(name, value));
    }
    headers.shrink_to_fit();
    Ok(headers)
}

fn encode_string(
    writer: &mut BinaryWriter,
    field: &'static str,
    value: &str,
    limit: usize,
) -> Result<(), NetworkEventsCodecError> {
    encode_bytes(writer, field, value.as_bytes(), limit)
}

fn encode_bytes(
    writer: &mut BinaryWriter,
    field: &'static str,
    value: &[u8],
    limit: usize,
) -> Result<(), NetworkEventsCodecError> {
    if value.len() > limit {
        return Err(NetworkEventsCodecError::FieldLimitExceeded {
            field,
            actual: value.len(),
            limit,
        });
    }
    writer.count_u32(field, value.len())?;
    writer.raw(value)?;
    Ok(())
}

fn decode_string(
    reader: &mut BinaryReader<'_>,
    field: &'static str,
    limit: usize,
) -> Result<String, NetworkEventsCodecError> {
    let length = reader.count_u32(field)?;
    if length > limit {
        return Err(NetworkEventsCodecError::FieldLimitExceeded {
            field,
            actual: length,
            limit,
        });
    }
    let value =
        std::str::from_utf8(reader.take(length)?).map_err(NetworkEventsCodecError::InvalidUtf8)?;
    let mut owned = value.to_owned();
    owned.shrink_to_fit();
    Ok(owned)
}

fn decode_bytes(
    reader: &mut BinaryReader<'_>,
    field: &'static str,
    limit: usize,
) -> Result<Vec<u8>, NetworkEventsCodecError> {
    let length = reader.count_u32(field)?;
    if length > limit {
        return Err(NetworkEventsCodecError::FieldLimitExceeded {
            field,
            actual: length,
            limit,
        });
    }
    let mut bytes = reader.take(length)?.to_vec();
    bytes.shrink_to_fit();
    Ok(bytes)
}

fn encode_optional_bytes(
    writer: &mut BinaryWriter,
    field: &'static str,
    value: Option<&[u8]>,
    limit: usize,
) -> Result<(), NetworkEventsCodecError> {
    match value {
        Some(value) => {
            writer.u8(1)?;
            encode_bytes(writer, field, value, limit)
        }
        None => {
            writer.u8(0)?;
            Ok(())
        }
    }
}

fn decode_optional_bytes(
    reader: &mut BinaryReader<'_>,
    field: &'static str,
    limit: usize,
) -> Result<Option<Vec<u8>>, NetworkEventsCodecError> {
    match reader.u8()? {
        0 => Ok(None),
        1 => decode_bytes(reader, field, limit).map(Some),
        value => Err(NetworkEventsCodecError::InvalidOptionFlag(value)),
    }
}

fn encode_optional_object(
    writer: &mut BinaryWriter,
    value: Option<ObjectId>,
) -> Result<(), NetworkEventsCodecError> {
    match value {
        Some(value) => {
            writer.u8(1)?;
            writer.u64(value.get())?;
        }
        None => writer.u8(0)?,
    }
    Ok(())
}

fn decode_optional_object(
    reader: &mut BinaryReader<'_>,
) -> Result<Option<ObjectId>, NetworkEventsCodecError> {
    match reader.u8()? {
        0 => Ok(None),
        1 => Ok(Some(ObjectId::new(reader.u64()?))),
        value => Err(NetworkEventsCodecError::InvalidOptionFlag(value)),
    }
}

fn encode_optional_u64(
    writer: &mut BinaryWriter,
    value: Option<u64>,
) -> Result<(), NetworkEventsCodecError> {
    match value {
        Some(value) => {
            writer.u8(1)?;
            writer.u64(value)?;
        }
        None => writer.u8(0)?,
    }
    Ok(())
}

fn decode_optional_u64(
    reader: &mut BinaryReader<'_>,
) -> Result<Option<u64>, NetworkEventsCodecError> {
    match reader.u8()? {
        0 => Ok(None),
        1 => Ok(Some(reader.u64()?)),
        value => Err(NetworkEventsCodecError::InvalidOptionFlag(value)),
    }
}

fn decode_bool(value: u8) -> Result<bool, NetworkEventsCodecError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        value => Err(NetworkEventsCodecError::InvalidBoolean(value)),
    }
}

#[cfg(test)]
mod tests {
    use pwright_cap_network_events::{
        HttpHeader, NETWORK_REQUEST_EVENT_TAG, NETWORK_RESPONSE_EVENT_TAG, NetworkEventPayload,
        NetworkEventStream, NetworkLifecycleLimits, RequestFailed, RequestFinished,
        RequestInitializer, ResponseInitializer,
    };
    use pwright_core::{Event, EventSequence, EventTag, ObjectId, ProtocolVersion, SequencedEvent};
    use pwright_transport::{FrameLimits, FrameRoute, WireCodec, WireFrame, WireProtocol};

    use super::{NetworkEventsCodec, NetworkEventsCodecError, NetworkEventsCodecLimits};

    fn codec() -> NetworkEventsCodec {
        NetworkEventsCodec::new(NetworkEventsCodecLimits::from_lifecycle(
            NetworkLifecycleLimits::new(8, 64, 256, 16, 32, 8, 256, 64, 64, 128),
        ))
    }

    fn protocol() -> WireProtocol<NetworkEventsCodec> {
        WireProtocol::new_exact_directional(
            codec(),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(2048, 0, 0, 2048),
            FrameLimits::new(2048, 0, 0, 2048),
        )
    }

    #[test]
    fn four_variants_round_trip_with_one_global_sequence_and_public_tags() {
        // Arrange
        let request = RequestInitializer {
            request: ObjectId::new(10),
            parent: ObjectId::new(3),
            url: "https://example.test/start".to_owned(),
            method: "POST".to_owned(),
            resource_type: "Fetch".to_owned(),
            headers: vec![HttpHeader::new("accept", "application/json")],
            post_data: Some(b"hello".to_vec()),
            is_navigation_request: false,
            redirected_from: None,
        };
        let response = ResponseInitializer {
            response: ObjectId::new(11),
            request: ObjectId::new(10),
            url: "https://example.test/start".to_owned(),
            status: 500,
            status_text: "Server Error".to_owned(),
            headers: vec![HttpHeader::new("content-type", "application/json")],
            from_service_worker: false,
        };
        let values = [
            NetworkEventPayload::Request(request.clone()),
            NetworkEventPayload::Response(response.clone()),
            NetworkEventPayload::RequestFinished(RequestFinished {
                request: request.clone(),
                response: Some(response),
                encoded_body_bytes: Some(42),
            }),
            NetworkEventPayload::RequestFailed(RequestFailed {
                request: RequestInitializer {
                    request: ObjectId::new(12),
                    ..request
                },
                response: None,
                failure_text: "net::ERR_ABORTED".to_owned(),
                canceled: true,
            }),
        ];
        let protocol = protocol();

        // Act
        let decoded = values
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, payload)| {
                let sequence = EventSequence::new(index as u64 + 1);
                let event = SequencedEvent::new(
                    sequence,
                    Event::<NetworkEventStream>::new(ObjectId::new(3), payload),
                );
                let frame = protocol.encode(&event).expect("event should encode");
                let route = frame.route();
                let decoded = protocol
                    .decode::<SequencedEvent<NetworkEventStream>>(frame)
                    .expect("event should decode");
                (sequence, route, decoded)
            })
            .collect::<Vec<_>>();

        // Assert
        for (index, (sequence, route, decoded)) in decoded.iter().enumerate() {
            assert_eq!(*sequence, EventSequence::new(index as u64 + 1));
            assert_eq!(decoded.sequence(), *sequence);
            assert_eq!(decoded.object_id(), ObjectId::new(3));
            assert_eq!(decoded.payload(), &values[index]);
            assert_eq!(
                *route,
                FrameRoute::Event {
                    source: ObjectId::new(3),
                    event: values[index].event_tag(),
                }
            );
        }
    }

    #[test]
    fn terminal_failure_reaches_the_declared_maximum_payload_bound() {
        // Arrange
        let limits = NetworkLifecycleLimits::new(1, 8, 5, 3, 4, 1, 8, 6, 4, 10);
        let codec_limits = NetworkEventsCodecLimits::from_lifecycle(limits);
        let codec = NetworkEventsCodec::new(codec_limits);
        let request = RequestInitializer {
            request: ObjectId::new(10),
            parent: ObjectId::new(3),
            url: "12345".to_owned(),
            method: "123".to_owned(),
            resource_type: "1234".to_owned(),
            headers: vec![HttpHeader::new("a", "1234567")],
            post_data: Some(vec![0; 6]),
            is_navigation_request: true,
            redirected_from: Some(ObjectId::new(9)),
        };
        let response = ResponseInitializer {
            response: ObjectId::new(11),
            request: ObjectId::new(10),
            url: "12345".to_owned(),
            status: 500,
            status_text: "1234".to_owned(),
            headers: vec![HttpHeader::new("a", "1234567")],
            from_service_worker: true,
        };
        let event = SequencedEvent::new(
            EventSequence::FIRST,
            Event::new(
                ObjectId::new(3),
                NetworkEventPayload::RequestFailed(RequestFailed {
                    request,
                    response: Some(response),
                    failure_text: "1234567890".to_owned(),
                    canceled: true,
                }),
            ),
        );

        let protocol = WireProtocol::new_exact_directional(
            codec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(4096, 0, 0, 4096),
            FrameLimits::new(4096, 0, 0, 4096),
        );

        // Act
        let frame = protocol
            .encode(&event)
            .expect("maximum terminal snapshot should encode");

        // Assert
        assert!(frame.attachments().is_empty());
        assert_eq!(frame.payload().len(), codec_limits.maximum_payload_bytes());
        assert_eq!(codec_limits.maximum_retained_event_bytes(), 53);
    }

    #[test]
    fn payload_kind_must_match_the_public_route_tag() {
        // Arrange
        let event = SequencedEvent::new(
            EventSequence::FIRST,
            Event::<NetworkEventStream>::new(
                ObjectId::new(3),
                NetworkEventPayload::Request(RequestInitializer {
                    request: ObjectId::new(10),
                    parent: ObjectId::new(3),
                    url: "https://example.test".to_owned(),
                    method: "GET".to_owned(),
                    resource_type: "Document".to_owned(),
                    headers: Vec::new(),
                    post_data: None,
                    is_navigation_request: true,
                    redirected_from: None,
                }),
            ),
        );
        let protocol = protocol();
        let encoded = protocol.encode(&event).expect("request should encode");
        let frame = WireFrame::new(
            encoded.version(),
            FrameRoute::Event {
                source: ObjectId::new(3),
                event: NETWORK_RESPONSE_EVENT_TAG,
            },
            encoded.payload().to_vec(),
            Vec::new(),
            FrameLimits::new(2048, 0, 0, 2048),
        )
        .expect("tampered frame should fit");

        // Act
        let error = match codec().decode_frame(frame) {
            Ok(_) => panic!("route and payload variants must agree"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(
            error,
            NetworkEventsCodecError::EventTagMismatch {
                route: NETWORK_RESPONSE_EVENT_TAG,
                payload: NETWORK_REQUEST_EVENT_TAG,
            }
        );
    }

    #[test]
    fn decode_rejects_unknown_network_event_tags_before_allocation() {
        // Arrange
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Event {
                source: ObjectId::new(3),
                event: EventTag::new(99_999),
            },
            vec![0; 17],
            Vec::new(),
            FrameLimits::new(64, 0, 0, 64),
        )
        .expect("unknown event frame should fit");

        // Act
        let error = match codec().decode_frame(frame) {
            Ok(_) => panic!("unknown event must fail before decoding its payload"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(
            error,
            NetworkEventsCodecError::UnsupportedEventTag(EventTag::new(99_999))
        );
    }
}
