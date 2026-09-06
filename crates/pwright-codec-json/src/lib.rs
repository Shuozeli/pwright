//! Optional Serde JSON payload codec for `pwright-transport`.
//!
//! Installing this crate opts into Serde and JSON. It supports payload-only
//! frames; binary attachment mapping belongs in a separate schema decision.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_transport::{EncodedParts, FrameRoute, RoutedMessage, WireCodec, WireFrame};
use serde::Serialize;
use serde::de::DeserializeOwned;

#[derive(Clone, Copy, Debug, Default)]
pub struct JsonCodec;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JsonCodecError {
    Serialize(String),
    Deserialize(String),
    BinaryAttachmentsUnsupported {
        actual: usize,
    },
    RouteMismatch {
        outer: FrameRoute,
        decoded: FrameRoute,
    },
}

impl Display for JsonCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialize(message) => write!(formatter, "JSON serialization failed: {message}"),
            Self::Deserialize(message) => {
                write!(formatter, "JSON deserialization failed: {message}")
            }
            Self::BinaryAttachmentsUnsupported { actual } => write!(
                formatter,
                "JSON codec does not support {actual} binary attachment(s)"
            ),
            Self::RouteMismatch { outer, decoded } => write!(
                formatter,
                "decoded message route {decoded:?} does not match outer route {outer:?}"
            ),
        }
    }
}

impl Error for JsonCodecError {}

impl<Message> WireCodec<Message> for JsonCodec
where
    Message: RoutedMessage + Serialize + DeserializeOwned,
{
    type Error = JsonCodecError;

    fn encode_parts(&self, message: &Message) -> Result<EncodedParts, Self::Error> {
        let payload = serde_json::to_vec(message)
            .map_err(|error| JsonCodecError::Serialize(error.to_string()))?;
        Ok(EncodedParts::new(message.route(), payload, Vec::new()))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Message, Self::Error> {
        if !frame.attachments().is_empty() {
            return Err(JsonCodecError::BinaryAttachmentsUnsupported {
                actual: frame.attachments().len(),
            });
        }
        let outer = frame.route();
        let message: Message = serde_json::from_slice(frame.payload())
            .map_err(|error| JsonCodecError::Deserialize(error.to_string()))?;
        let decoded = message.route();
        if decoded != outer {
            return Err(JsonCodecError::RouteMismatch { outer, decoded });
        }
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{CallId, MethodTag, ObjectId, ProtocolVersion};
    use pwright_transport::{
        FrameLimits, FrameRoute, RoutedMessage, VersionPolicy, WireCodec, WireFrame, WireProtocol,
        WireProtocolError,
    };
    use serde::{Deserialize, Serialize};

    use super::{JsonCodec, JsonCodecError};

    #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct TestMessage {
        call_id: u64,
        target: u64,
        method: u32,
        value: String,
    }

    impl RoutedMessage for TestMessage {
        fn route(&self) -> FrameRoute {
            FrameRoute::Command {
                call_id: CallId::new(self.call_id),
                target: ObjectId::new(self.target),
                method: MethodTag::new(self.method),
            }
        }
    }

    fn protocol() -> WireProtocol<JsonCodec> {
        WireProtocol::new(
            JsonCodec,
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 1))
                .expect("version policy should build"),
            ProtocolVersion::new(1, 1),
            FrameLimits::new(256, 0, 0, 256),
        )
        .expect("wire protocol should build")
    }

    fn message() -> TestMessage {
        TestMessage {
            call_id: 1,
            target: 2,
            method: 3,
            value: "hello".to_string(),
        }
    }

    #[test]
    fn json_codec_round_trips_through_wire_protocol() {
        // Arrange
        let protocol = protocol();
        let message = message();

        // Act
        let frame = protocol.encode(&message).expect("message should encode");
        let decoded = protocol
            .decode::<TestMessage>(frame)
            .expect("message should decode");

        // Assert
        assert_eq!(decoded, message);
    }

    #[test]
    fn json_codec_rejects_malformed_payload() {
        // Arrange
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 1),
            message().route(),
            b"{".to_vec(),
            Vec::new(),
            FrameLimits::new(256, 0, 0, 256),
        )
        .expect("bounded malformed frame should build");

        // Act
        let decoded: Result<TestMessage, JsonCodecError> = JsonCodec.decode_frame(frame);
        let error = decoded.expect_err("malformed JSON should fail");

        // Assert
        assert!(matches!(error, JsonCodecError::Deserialize(_)));
    }

    #[test]
    fn json_codec_rejects_decoded_route_mismatch() {
        // Arrange
        let payload = serde_json::to_vec(&message()).expect("message should serialize");
        let outer = FrameRoute::Result {
            call_id: CallId::new(1),
        };
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 1),
            outer,
            payload,
            Vec::new(),
            FrameLimits::new(256, 0, 0, 256),
        )
        .expect("bounded frame should build");

        // Act
        let decoded: Result<TestMessage, JsonCodecError> = JsonCodec.decode_frame(frame);
        let error = decoded.expect_err("route mismatch should fail");

        // Assert
        assert_eq!(
            error,
            JsonCodecError::RouteMismatch {
                outer,
                decoded: message().route(),
            }
        );
    }

    #[test]
    fn json_codec_rejects_binary_attachments() {
        // Arrange
        let payload = serde_json::to_vec(&message()).expect("message should serialize");
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 1),
            message().route(),
            payload,
            vec![vec![1, 2]],
            FrameLimits::new(256, 1, 2, 258),
        )
        .expect("bounded attachment frame should build");

        // Act
        let decoded: Result<TestMessage, JsonCodecError> = JsonCodec.decode_frame(frame);
        let error = decoded.expect_err("attachments should be rejected");

        // Assert
        assert_eq!(
            error,
            JsonCodecError::BinaryAttachmentsUnsupported { actual: 1 }
        );
    }

    #[test]
    fn wire_protocol_applies_limits_to_json_output() {
        // Arrange
        let protocol = WireProtocol::new(
            JsonCodec,
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("version policy should build"),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(4, 0, 0, 4),
        )
        .expect("wire protocol should build");

        // Act
        let error = protocol
            .encode(&message())
            .expect_err("encoded JSON should exceed frame limit");

        // Assert
        assert!(matches!(error, WireProtocolError::Frame(_)));
    }
}
