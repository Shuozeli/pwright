//! Canonical fixed-size binary codec for optional Browser events.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_cap_browser_events::{
    BROWSER_DISCONNECTED_EVENT_TAG, BrowserDisconnected, BrowserDisconnectedEvent,
};
use pwright_core::{Event, EventSequence, ObjectId, SequencedEvent};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const BROWSER_DISCONNECTED_KIND: u8 = 1;
pub const BROWSER_DISCONNECTED_PAYLOAD_BYTES: usize = 17;

#[derive(Clone, Copy, Debug, Default)]
pub struct BrowserEventsCodec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserEventsCodecError {
    WrongRoute(FrameRoute),
    WrongEventTag {
        expected: pwright_core::EventTag,
        actual: pwright_core::EventTag,
    },
    WrongPayloadLength {
        expected: usize,
        actual: usize,
    },
    WrongPayloadKind {
        expected: u8,
        actual: u8,
    },
    SourceMismatch {
        route: ObjectId,
        payload: ObjectId,
    },
    InvalidSequence,
    AttachmentsUnsupported {
        actual: usize,
    },
}

impl Display for BrowserEventsCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => write!(formatter, "Browser event arrived on {route:?}"),
            Self::WrongEventTag { expected, actual } => write!(
                formatter,
                "expected Browser event tag {expected}, received {actual}"
            ),
            Self::WrongPayloadLength { expected, actual } => write!(
                formatter,
                "Browser event payload has {actual} bytes, expected {expected}"
            ),
            Self::WrongPayloadKind { expected, actual } => write!(
                formatter,
                "expected Browser event payload kind {expected}, received {actual}"
            ),
            Self::SourceMismatch { route, payload } => write!(
                formatter,
                "Browser event route source `{route}` differs from payload source `{payload}`"
            ),
            Self::InvalidSequence => formatter.write_str("Browser event sequence zero is invalid"),
            Self::AttachmentsUnsupported { actual } => write!(
                formatter,
                "Browser event has {actual} attachment(s), but none are allowed"
            ),
        }
    }
}

impl Error for BrowserEventsCodecError {}

impl WireCodec<SequencedEvent<BrowserDisconnected>> for BrowserEventsCodec {
    type Error = BrowserEventsCodecError;

    fn encode_parts(
        &self,
        event: &SequencedEvent<BrowserDisconnected>,
    ) -> Result<EncodedParts, Self::Error> {
        if event.sequence().get() == 0 {
            return Err(BrowserEventsCodecError::InvalidSequence);
        }
        let mut payload = Vec::with_capacity(BROWSER_DISCONNECTED_PAYLOAD_BYTES);
        payload.push(BROWSER_DISCONNECTED_KIND);
        payload.extend_from_slice(&event.sequence().get().to_be_bytes());
        payload.extend_from_slice(&event.object_id().get().to_be_bytes());
        Ok(EncodedParts::new(
            FrameRoute::Event {
                source: event.object_id(),
                event: BROWSER_DISCONNECTED_EVENT_TAG,
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<SequencedEvent<BrowserDisconnected>, Self::Error> {
        let FrameRoute::Event { source, event } = frame.route() else {
            return Err(BrowserEventsCodecError::WrongRoute(frame.route()));
        };
        if event != BROWSER_DISCONNECTED_EVENT_TAG {
            return Err(BrowserEventsCodecError::WrongEventTag {
                expected: BROWSER_DISCONNECTED_EVENT_TAG,
                actual: event,
            });
        }
        if !frame.attachments().is_empty() {
            return Err(BrowserEventsCodecError::AttachmentsUnsupported {
                actual: frame.attachments().len(),
            });
        }
        if frame.payload().len() != BROWSER_DISCONNECTED_PAYLOAD_BYTES {
            return Err(BrowserEventsCodecError::WrongPayloadLength {
                expected: BROWSER_DISCONNECTED_PAYLOAD_BYTES,
                actual: frame.payload().len(),
            });
        }
        if frame.payload()[0] != BROWSER_DISCONNECTED_KIND {
            return Err(BrowserEventsCodecError::WrongPayloadKind {
                expected: BROWSER_DISCONNECTED_KIND,
                actual: frame.payload()[0],
            });
        }
        let mut sequence_bytes = [0_u8; 8];
        sequence_bytes.copy_from_slice(&frame.payload()[1..9]);
        let sequence = EventSequence::new(u64::from_be_bytes(sequence_bytes));
        if sequence.get() == 0 {
            return Err(BrowserEventsCodecError::InvalidSequence);
        }
        let mut source_bytes = [0_u8; 8];
        source_bytes.copy_from_slice(&frame.payload()[9..]);
        let payload_source = ObjectId::new(u64::from_be_bytes(source_bytes));
        if payload_source != source {
            return Err(BrowserEventsCodecError::SourceMismatch {
                route: source,
                payload: payload_source,
            });
        }
        Ok(SequencedEvent::new(
            sequence,
            Event::new(source, BrowserDisconnectedEvent),
        ))
    }
}

#[cfg(test)]
mod tests {
    use pwright_cap_browser_events::{
        BROWSER_DISCONNECTED_EVENT_TAG, BrowserDisconnected, BrowserDisconnectedEvent,
    };
    use pwright_core::{Event, EventSequence, EventSpec, ObjectId, SequencedEvent};
    use pwright_transport::{FrameLimits, FrameRoute, WireCodec, WireFrame, WireProtocol};

    use super::{BrowserEventsCodec, BrowserEventsCodecError};

    #[test]
    fn disconnected_round_trips_with_browser_identity_and_sequence() {
        // Arrange
        let protocol = WireProtocol::new_exact_directional(
            BrowserEventsCodec,
            pwright_core::ProtocolVersion::new(1, 0),
            FrameLimits::new(17, 0, 0, 17),
            FrameLimits::new(17, 0, 0, 17),
        );
        let event = SequencedEvent::<BrowserDisconnected>::new(
            EventSequence::new(7),
            Event::new(ObjectId::new(11), BrowserDisconnectedEvent),
        );

        // Act
        let frame = protocol.encode(&event).expect("event should encode");
        let decoded = protocol
            .decode::<SequencedEvent<BrowserDisconnected>>(frame)
            .expect("event should decode");

        // Assert
        assert_eq!(decoded.sequence(), EventSequence::new(7));
        assert_eq!(decoded.object_id(), ObjectId::new(11));
        assert_eq!(decoded.payload(), &BrowserDisconnectedEvent);
    }

    #[test]
    fn disconnected_rejects_a_route_payload_identity_mismatch() {
        // Arrange
        let mut payload = Vec::new();
        payload.push(1);
        payload.extend_from_slice(&1_u64.to_be_bytes());
        payload.extend_from_slice(&9_u64.to_be_bytes());
        let frame = WireFrame::new(
            pwright_core::ProtocolVersion::new(1, 0),
            FrameRoute::Event {
                source: ObjectId::new(8),
                event: BrowserDisconnected::EVENT,
            },
            payload,
            Vec::new(),
            FrameLimits::new(17, 0, 0, 17),
        )
        .expect("bounded malformed frame should build");

        // Act
        let result: Result<SequencedEvent<BrowserDisconnected>, _> =
            BrowserEventsCodec.decode_frame(frame);
        let error = match result {
            Ok(_) => panic!("mismatched source should fail"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(
            error,
            BrowserEventsCodecError::SourceMismatch {
                route: ObjectId::new(8),
                payload: ObjectId::new(9),
            }
        );
        assert_eq!(BROWSER_DISCONNECTED_EVENT_TAG, BrowserDisconnected::EVENT);
    }
}
