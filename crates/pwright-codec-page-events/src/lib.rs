//! Canonical fixed-size binary codec for optional Page discovery/lifecycle events.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_cap_page_events::{
    BROWSER_CONTEXT_PAGE_EVENT_TAG, BrowserContextPage, BrowserContextPageEvent,
    PAGE_LOAD_EVENT_TAG, PAGE_POPUP_EVENT_TAG, PageLoad, PageLoadEvent, PagePopup, PagePopupEvent,
};
use pwright_core::{Event, EventSequence, ObjectId, SequencedEvent};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const PAGE_LOAD_KIND: u8 = 1;
const BROWSER_CONTEXT_PAGE_KIND: u8 = 2;
const PAGE_POPUP_KIND: u8 = 3;
const PAGE_LOAD_PAYLOAD_BYTES: usize = 17;
const PAGE_WITH_RELATED_OBJECT_PAYLOAD_BYTES: usize = 25;

#[derive(Clone, Copy, Debug, Default)]
pub struct PageEventsCodec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageEventsCodecError {
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

impl Display for PageEventsCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => write!(formatter, "Page event arrived on {route:?}"),
            Self::WrongEventTag { expected, actual } => {
                write!(
                    formatter,
                    "expected Page event tag {expected}, received {actual}"
                )
            }
            Self::WrongPayloadLength { expected, actual } => write!(
                formatter,
                "Page event payload has {actual} bytes, expected {expected}"
            ),
            Self::WrongPayloadKind { expected, actual } => write!(
                formatter,
                "expected Page event payload kind {expected}, received {actual}"
            ),
            Self::SourceMismatch { route, payload } => write!(
                formatter,
                "Page event route source `{route}` differs from payload source `{payload}`"
            ),
            Self::InvalidSequence => formatter.write_str("Page event sequence zero is invalid"),
            Self::AttachmentsUnsupported { actual } => write!(
                formatter,
                "Page event has {actual} attachment(s), but none are allowed"
            ),
        }
    }
}

impl Error for PageEventsCodecError {}

impl WireCodec<SequencedEvent<PageLoad>> for PageEventsCodec {
    type Error = PageEventsCodecError;

    fn encode_parts(&self, event: &SequencedEvent<PageLoad>) -> Result<EncodedParts, Self::Error> {
        if event.sequence().get() == 0 {
            return Err(PageEventsCodecError::InvalidSequence);
        }
        let mut payload = Vec::with_capacity(PAGE_LOAD_PAYLOAD_BYTES);
        payload.push(PAGE_LOAD_KIND);
        payload.extend_from_slice(&event.sequence().get().to_be_bytes());
        payload.extend_from_slice(&event.object_id().get().to_be_bytes());
        Ok(EncodedParts::new(
            FrameRoute::Event {
                source: event.object_id(),
                event: PAGE_LOAD_EVENT_TAG,
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<SequencedEvent<PageLoad>, Self::Error> {
        let FrameRoute::Event { source, event } = frame.route() else {
            return Err(PageEventsCodecError::WrongRoute(frame.route()));
        };
        if event != PAGE_LOAD_EVENT_TAG {
            return Err(PageEventsCodecError::WrongEventTag {
                expected: PAGE_LOAD_EVENT_TAG,
                actual: event,
            });
        }
        if !frame.attachments().is_empty() {
            return Err(PageEventsCodecError::AttachmentsUnsupported {
                actual: frame.attachments().len(),
            });
        }
        if frame.payload().len() != PAGE_LOAD_PAYLOAD_BYTES {
            return Err(PageEventsCodecError::WrongPayloadLength {
                expected: PAGE_LOAD_PAYLOAD_BYTES,
                actual: frame.payload().len(),
            });
        }
        if frame.payload()[0] != PAGE_LOAD_KIND {
            return Err(PageEventsCodecError::WrongPayloadKind {
                expected: PAGE_LOAD_KIND,
                actual: frame.payload()[0],
            });
        }
        let mut sequence_bytes = [0_u8; 8];
        sequence_bytes.copy_from_slice(&frame.payload()[1..9]);
        let sequence = EventSequence::new(u64::from_be_bytes(sequence_bytes));
        if sequence.get() == 0 {
            return Err(PageEventsCodecError::InvalidSequence);
        }
        let mut source_bytes = [0_u8; 8];
        source_bytes.copy_from_slice(&frame.payload()[9..]);
        let payload_source = ObjectId::new(u64::from_be_bytes(source_bytes));
        if payload_source != source {
            return Err(PageEventsCodecError::SourceMismatch {
                route: source,
                payload: payload_source,
            });
        }
        Ok(SequencedEvent::new(
            sequence,
            Event::new(source, PageLoadEvent),
        ))
    }
}

impl WireCodec<SequencedEvent<BrowserContextPage>> for PageEventsCodec {
    type Error = PageEventsCodecError;

    fn encode_parts(
        &self,
        event: &SequencedEvent<BrowserContextPage>,
    ) -> Result<EncodedParts, Self::Error> {
        encode_related_page_event(
            event.sequence(),
            event.object_id(),
            event.payload().page,
            BROWSER_CONTEXT_PAGE_EVENT_TAG,
            BROWSER_CONTEXT_PAGE_KIND,
        )
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<SequencedEvent<BrowserContextPage>, Self::Error> {
        let (sequence, source, page) = decode_related_page_event(
            &frame,
            BROWSER_CONTEXT_PAGE_EVENT_TAG,
            BROWSER_CONTEXT_PAGE_KIND,
        )?;
        Ok(SequencedEvent::new(
            sequence,
            Event::new(source, BrowserContextPageEvent { page }),
        ))
    }
}

impl WireCodec<SequencedEvent<PagePopup>> for PageEventsCodec {
    type Error = PageEventsCodecError;

    fn encode_parts(&self, event: &SequencedEvent<PagePopup>) -> Result<EncodedParts, Self::Error> {
        encode_related_page_event(
            event.sequence(),
            event.object_id(),
            event.payload().page,
            PAGE_POPUP_EVENT_TAG,
            PAGE_POPUP_KIND,
        )
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<SequencedEvent<PagePopup>, Self::Error> {
        let (sequence, source, page) =
            decode_related_page_event(&frame, PAGE_POPUP_EVENT_TAG, PAGE_POPUP_KIND)?;
        Ok(SequencedEvent::new(
            sequence,
            Event::new(source, PagePopupEvent { page }),
        ))
    }
}

fn encode_related_page_event(
    sequence: EventSequence,
    source: ObjectId,
    page: ObjectId,
    event: pwright_core::EventTag,
    kind: u8,
) -> Result<EncodedParts, PageEventsCodecError> {
    if sequence.get() == 0 {
        return Err(PageEventsCodecError::InvalidSequence);
    }
    let mut payload = Vec::with_capacity(PAGE_WITH_RELATED_OBJECT_PAYLOAD_BYTES);
    payload.push(kind);
    payload.extend_from_slice(&sequence.get().to_be_bytes());
    payload.extend_from_slice(&source.get().to_be_bytes());
    payload.extend_from_slice(&page.get().to_be_bytes());
    Ok(EncodedParts::new(
        FrameRoute::Event { source, event },
        payload,
        Vec::new(),
    ))
}

fn decode_related_page_event(
    frame: &WireFrame,
    expected_event: pwright_core::EventTag,
    expected_kind: u8,
) -> Result<(EventSequence, ObjectId, ObjectId), PageEventsCodecError> {
    let FrameRoute::Event { source, event } = frame.route() else {
        return Err(PageEventsCodecError::WrongRoute(frame.route()));
    };
    if event != expected_event {
        return Err(PageEventsCodecError::WrongEventTag {
            expected: expected_event,
            actual: event,
        });
    }
    if !frame.attachments().is_empty() {
        return Err(PageEventsCodecError::AttachmentsUnsupported {
            actual: frame.attachments().len(),
        });
    }
    if frame.payload().len() != PAGE_WITH_RELATED_OBJECT_PAYLOAD_BYTES {
        return Err(PageEventsCodecError::WrongPayloadLength {
            expected: PAGE_WITH_RELATED_OBJECT_PAYLOAD_BYTES,
            actual: frame.payload().len(),
        });
    }
    if frame.payload()[0] != expected_kind {
        return Err(PageEventsCodecError::WrongPayloadKind {
            expected: expected_kind,
            actual: frame.payload()[0],
        });
    }
    let mut sequence_bytes = [0_u8; 8];
    sequence_bytes.copy_from_slice(&frame.payload()[1..9]);
    let sequence = EventSequence::new(u64::from_be_bytes(sequence_bytes));
    if sequence.get() == 0 {
        return Err(PageEventsCodecError::InvalidSequence);
    }
    let mut source_bytes = [0_u8; 8];
    source_bytes.copy_from_slice(&frame.payload()[9..17]);
    let payload_source = ObjectId::new(u64::from_be_bytes(source_bytes));
    if payload_source != source {
        return Err(PageEventsCodecError::SourceMismatch {
            route: source,
            payload: payload_source,
        });
    }
    let mut page_bytes = [0_u8; 8];
    page_bytes.copy_from_slice(&frame.payload()[17..25]);
    Ok((
        sequence,
        source,
        ObjectId::new(u64::from_be_bytes(page_bytes)),
    ))
}

#[cfg(test)]
mod tests {
    use pwright_cap_page_events::{
        BROWSER_CONTEXT_PAGE_EVENT_TAG, BrowserContextPage, BrowserContextPageEvent,
        PAGE_LOAD_EVENT_TAG, PAGE_POPUP_EVENT_TAG, PageLoad, PageLoadEvent, PagePopup,
        PagePopupEvent,
    };
    use pwright_core::{Event, EventSequence, ObjectId, ProtocolVersion, SequencedEvent};
    use pwright_transport::{FrameLimits, FrameRoute, WireCodec, WireFrame, WireProtocol};

    use super::{PAGE_LOAD_KIND, PageEventsCodec, PageEventsCodecError};

    #[test]
    fn page_load_round_trips_with_exact_source_identity() {
        // Arrange
        let protocol = WireProtocol::new_exact_directional(
            PageEventsCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
            FrameLimits::new(64, 0, 0, 64),
        );
        let event = SequencedEvent::new(
            EventSequence::new(7),
            Event::<PageLoad>::new(ObjectId::new(42), PageLoadEvent),
        );

        // Act
        let decoded = protocol
            .decode::<SequencedEvent<PageLoad>>(
                protocol.encode(&event).expect("event should encode"),
            )
            .expect("event should decode");

        // Assert
        assert_eq!(decoded.object_id(), ObjectId::new(42));
        assert_eq!(decoded.sequence(), EventSequence::new(7));
        assert_eq!(decoded.payload(), &PageLoadEvent);
    }

    #[test]
    fn page_load_rejects_a_payload_source_that_differs_from_its_route() {
        // Arrange
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Event {
                source: ObjectId::new(42),
                event: PAGE_LOAD_EVENT_TAG,
            },
            [
                vec![1],
                EventSequence::new(1).get().to_be_bytes().to_vec(),
                ObjectId::new(7).get().to_be_bytes().to_vec(),
            ]
            .concat(),
            Vec::new(),
            FrameLimits::new(64, 0, 0, 64),
        )
        .expect("frame should fit");

        // Act
        let decoded: Result<SequencedEvent<PageLoad>, _> = PageEventsCodec.decode_frame(frame);
        let error = match decoded {
            Ok(_) => panic!("mismatched source should fail"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(
            error,
            PageEventsCodecError::SourceMismatch {
                route: ObjectId::new(42),
                payload: ObjectId::new(7),
            }
        );
    }

    #[test]
    fn page_load_rejects_the_reserved_zero_sequence() {
        // Arrange
        let source = ObjectId::new(42);
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Event {
                source,
                event: PAGE_LOAD_EVENT_TAG,
            },
            [
                vec![PAGE_LOAD_KIND],
                EventSequence::new(0).get().to_be_bytes().to_vec(),
                source.get().to_be_bytes().to_vec(),
            ]
            .concat(),
            Vec::new(),
            FrameLimits::new(64, 0, 0, 64),
        )
        .expect("frame should fit");

        // Act
        let decoded: Result<SequencedEvent<PageLoad>, _> = PageEventsCodec.decode_frame(frame);
        let error = match decoded {
            Ok(_) => panic!("zero sequence should fail"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(error, PageEventsCodecError::InvalidSequence);
    }

    #[test]
    fn context_page_round_trips_with_context_source_and_page_payload() {
        // Arrange
        let protocol = WireProtocol::new_exact_directional(
            PageEventsCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
            FrameLimits::new(64, 0, 0, 64),
        );
        let event = SequencedEvent::new(
            EventSequence::new(8),
            Event::<BrowserContextPage>::new(
                ObjectId::new(2),
                BrowserContextPageEvent {
                    page: ObjectId::new(7),
                },
            ),
        );

        // Act
        let frame = protocol.encode(&event).expect("event should encode");
        let route = frame.route();
        let decoded = protocol
            .decode::<SequencedEvent<BrowserContextPage>>(frame)
            .expect("event should decode");

        // Assert
        assert_eq!(
            route,
            FrameRoute::Event {
                source: ObjectId::new(2),
                event: BROWSER_CONTEXT_PAGE_EVENT_TAG,
            }
        );
        assert_eq!(decoded.object_id(), ObjectId::new(2));
        assert_eq!(decoded.payload().page, ObjectId::new(7));
    }

    #[test]
    fn popup_round_trips_with_opener_source_and_popup_payload() {
        // Arrange
        let protocol = WireProtocol::new_exact_directional(
            PageEventsCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
            FrameLimits::new(64, 0, 0, 64),
        );
        let event = SequencedEvent::new(
            EventSequence::new(9),
            Event::<PagePopup>::new(
                ObjectId::new(7),
                PagePopupEvent {
                    page: ObjectId::new(8),
                },
            ),
        );

        // Act
        let frame = protocol.encode(&event).expect("event should encode");
        let route = frame.route();
        let decoded = protocol
            .decode::<SequencedEvent<PagePopup>>(frame)
            .expect("event should decode");

        // Assert
        assert_eq!(
            route,
            FrameRoute::Event {
                source: ObjectId::new(7),
                event: PAGE_POPUP_EVENT_TAG,
            }
        );
        assert_eq!(decoded.object_id(), ObjectId::new(7));
        assert_eq!(decoded.payload().page, ObjectId::new(8));
    }
}
