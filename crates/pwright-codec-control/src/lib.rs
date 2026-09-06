//! Canonical binary payload codec for Pwright `ControlMessage` values.
//!
//! The outer `WireFrame` owns versioning and size limits. This codec owns only
//! the exact control payload representation and accepts no attachments.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_codec_binary::{
    BinaryDecode, BinaryDecodeError, BinaryEncode, BinaryEncodeError, BinaryReader,
    BinaryValueContext, BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits,
    BinaryWriter,
};
use pwright_core::{
    CallId, CancellationId, CancellationReason, ControlMessage, EventTag, ObjectId, ObjectKindTag,
    ProtocolVersion,
};
use pwright_transport::{
    CapabilityIdError, CapabilityRequirement, CapabilitySupport, EncodedParts, FrameRoute,
    ServerHello, VersionPolicy, WireCapabilityId, WireCodec, WireFrame,
};

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

pub use generated_schema::{CATALOG as CONTROL_SCHEMA_CATALOG, ROOT_SCHEMA as CONTROL_SCHEMA};
use generated_schema::{PwrightControl, PwrightControlCancellationReason};

const CONTROL_VALUE_LIMITS: BinaryValueLimits = BinaryValueLimits::new(0, 0, 0, 1, 1);

/// Reserved handshake capability ID for the connection-control protocol family.
pub const CONTROL_PROTOCOL_ID: &str = "pwright.control";
/// Control version understood by peers predating explicit family negotiation.
pub const CONTROL_PROTOCOL_BASELINE_VERSION: ProtocolVersion = ProtocolVersion::new(1, 0);
/// Highest control version implemented by this crate.
pub const CONTROL_PROTOCOL_CURRENT_VERSION: ProtocolVersion = ProtocolVersion::new(1, 1);
/// First control version that supports the graceful-close exchange.
pub const GRACEFUL_CLOSE_VERSION: ProtocolVersion = ProtocolVersion::new(1, 1);

#[derive(Clone, Copy, Debug, Default)]
pub struct ControlCodec;

/// The local endpoint role used to enforce control-message directionality.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlEndpointRole {
    Client,
    Server,
}

/// Stable operation names used by negotiation and direction errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlOperation {
    Create,
    Adopt,
    Dispose,
    Cancel,
    SetSubscription,
    Close,
    CloseAck,
}

/// A role-bound control codec constructed from the handshake result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NegotiatedControlCodec {
    role: ControlEndpointRole,
    version: ProtocolVersion,
}

impl NegotiatedControlCodec {
    pub fn new(
        role: ControlEndpointRole,
        version: ProtocolVersion,
    ) -> Result<Self, ControlCodecError> {
        validate_negotiated_version(version)?;
        Ok(Self { role, version })
    }

    pub fn for_client(hello: &ServerHello) -> Result<Self, ControlCodecError> {
        Self::new(
            ControlEndpointRole::Client,
            negotiated_control_version(hello)?,
        )
    }

    pub fn for_server(hello: &ServerHello) -> Result<Self, ControlCodecError> {
        Self::new(
            ControlEndpointRole::Server,
            negotiated_control_version(hello)?,
        )
    }

    pub const fn role(self) -> ControlEndpointRole {
        self.role
    }

    pub const fn version(self) -> ProtocolVersion {
        self.version
    }
}

/// Returns the optional handshake request that unlocks newer control minors.
pub fn control_protocol_requirement() -> Result<CapabilityRequirement, CapabilityIdError> {
    Ok(CapabilityRequirement::within(
        control_protocol_id()?,
        VersionPolicy::new(
            CONTROL_PROTOCOL_BASELINE_VERSION,
            CONTROL_PROTOCOL_CURRENT_VERSION,
        )
        .expect("the static control protocol range is valid"),
    ))
}

/// Returns the server-side handshake support entry for this control codec.
pub fn control_protocol_support() -> Result<CapabilitySupport, CapabilityIdError> {
    Ok(CapabilitySupport::new(
        control_protocol_id()?,
        VersionPolicy::new(
            CONTROL_PROTOCOL_BASELINE_VERSION,
            CONTROL_PROTOCOL_CURRENT_VERSION,
        )
        .expect("the static control protocol range is valid"),
    ))
}

fn control_protocol_id() -> Result<WireCapabilityId, CapabilityIdError> {
    WireCapabilityId::new(CONTROL_PROTOCOL_ID, CONTROL_PROTOCOL_ID.len())
}

fn negotiated_control_version(hello: &ServerHello) -> Result<ProtocolVersion, ControlCodecError> {
    let version = hello
        .enabled_capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == CONTROL_PROTOCOL_ID)
        .map_or(CONTROL_PROTOCOL_BASELINE_VERSION, |capability| {
            capability.version()
        });
    validate_negotiated_version(version)?;
    Ok(version)
}

fn validate_negotiated_version(version: ProtocolVersion) -> Result<(), ControlCodecError> {
    if version.major != CONTROL_PROTOCOL_BASELINE_VERSION.major
        || version.minor > CONTROL_PROTOCOL_CURRENT_VERSION.minor
    {
        return Err(ControlCodecError::UnsupportedNegotiatedVersion {
            negotiated: version,
            minimum: CONTROL_PROTOCOL_BASELINE_VERSION,
            maximum: CONTROL_PROTOCOL_CURRENT_VERSION,
        });
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlCodecError {
    WrongRoute(FrameRoute),
    AttachmentsUnsupported {
        actual: usize,
    },
    UnknownMessage(u8),
    InvalidParentFlag(u8),
    NonCanonicalAbsentParent(u64),
    InvalidCancellationReason(u8),
    InvalidBoolean(u8),
    UnsupportedNegotiatedVersion {
        negotiated: ProtocolVersion,
        minimum: ProtocolVersion,
        maximum: ProtocolVersion,
    },
    MessageRequiresVersion {
        operation: ControlOperation,
        negotiated: ProtocolVersion,
        required: ProtocolVersion,
    },
    InvalidDirection {
        role: ControlEndpointRole,
        operation: ControlOperation,
        outgoing: bool,
    },
    Encode(BinaryEncodeError),
    ValueEncode(BinaryValueEncodeError),
    ValueDecode(BinaryValueDecodeError),
    IntegerOutOfRange {
        field: &'static str,
        value: u64,
    },
    Truncated {
        offset: usize,
        needed: usize,
        remaining: usize,
    },
    TrailingBytes {
        remaining: usize,
    },
}

impl Display for ControlCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => {
                write!(formatter, "control payload arrived on route {route:?}")
            }
            Self::AttachmentsUnsupported { actual } => write!(
                formatter,
                "control payload has {actual} attachment(s), but none are allowed"
            ),
            Self::UnknownMessage(tag) => write!(formatter, "unknown control message tag {tag}"),
            Self::InvalidParentFlag(flag) => write!(formatter, "invalid parent flag {flag}"),
            Self::NonCanonicalAbsentParent(parent) => write!(
                formatter,
                "absent parent must encode as zero, received {parent}"
            ),
            Self::InvalidCancellationReason(reason) => {
                write!(formatter, "invalid cancellation reason {reason}")
            }
            Self::InvalidBoolean(value) => write!(formatter, "invalid boolean value {value}"),
            Self::UnsupportedNegotiatedVersion {
                negotiated,
                minimum,
                maximum,
            } => write!(
                formatter,
                "negotiated control version {negotiated} is outside supported range {minimum}..={maximum}"
            ),
            Self::MessageRequiresVersion {
                operation,
                negotiated,
                required,
            } => write!(
                formatter,
                "control operation {operation:?} requires version {required}, negotiated {negotiated}"
            ),
            Self::InvalidDirection {
                role,
                operation,
                outgoing,
            } => write!(
                formatter,
                "control operation {operation:?} is invalid for {role:?} {} traffic",
                if *outgoing { "outgoing" } else { "incoming" }
            ),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::ValueEncode(error) => Display::fmt(error, formatter),
            Self::ValueDecode(error) => Display::fmt(error, formatter),
            Self::IntegerOutOfRange { field, value } => {
                write!(
                    formatter,
                    "wire value {value} does not fit host field `{field}`"
                )
            }
            Self::Truncated {
                offset,
                needed,
                remaining,
            } => write!(
                formatter,
                "control payload at byte {offset} needs {needed} bytes but only {remaining} remain"
            ),
            Self::TrailingBytes { remaining } => {
                write!(formatter, "control payload has {remaining} trailing bytes")
            }
        }
    }
}

impl Error for ControlCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encode(error) => Some(error),
            Self::ValueEncode(error) => Some(error),
            Self::ValueDecode(error) => Some(error),
            _ => None,
        }
    }
}

impl From<BinaryEncodeError> for ControlCodecError {
    fn from(error: BinaryEncodeError) -> Self {
        Self::Encode(error)
    }
}

impl From<BinaryDecodeError> for ControlCodecError {
    fn from(error: BinaryDecodeError) -> Self {
        match error {
            BinaryDecodeError::IntegerOutOfRange { field, value } => {
                Self::IntegerOutOfRange { field, value }
            }
            BinaryDecodeError::Truncated {
                offset,
                needed,
                remaining,
            } => Self::Truncated {
                offset,
                needed,
                remaining,
            },
            BinaryDecodeError::TrailingBytes { remaining } => Self::TrailingBytes { remaining },
        }
    }
}

impl From<BinaryValueEncodeError> for ControlCodecError {
    fn from(error: BinaryValueEncodeError) -> Self {
        match error {
            BinaryValueEncodeError::Primitive(error) => Self::Encode(error),
            error => Self::ValueEncode(error),
        }
    }
}

impl From<BinaryValueDecodeError> for ControlCodecError {
    fn from(error: BinaryValueDecodeError) -> Self {
        match error {
            BinaryValueDecodeError::Primitive(error) => error.into(),
            BinaryValueDecodeError::InvalidBoolean {
                field: "pwright.control.create.parent_present",
                value,
            } => Self::InvalidParentFlag(value),
            BinaryValueDecodeError::InvalidBoolean { value, .. } => Self::InvalidBoolean(value),
            BinaryValueDecodeError::InvalidVariantTag {
                type_id: "pwright.control",
                value,
            } => Self::UnknownMessage(value),
            BinaryValueDecodeError::InvalidVariantTag {
                type_id: "pwright.control.cancellation_reason",
                value,
            } => Self::InvalidCancellationReason(value),
            error => Self::ValueDecode(error),
        }
    }
}

impl WireCodec<ControlMessage> for ControlCodec {
    type Error = ControlCodecError;

    fn encode_parts(&self, message: &ControlMessage) -> Result<EncodedParts, Self::Error> {
        encode_control(message, CONTROL_PROTOCOL_BASELINE_VERSION, None)
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<ControlMessage, Self::Error> {
        decode_control(frame, CONTROL_PROTOCOL_BASELINE_VERSION, None)
    }
}

impl WireCodec<ControlMessage> for NegotiatedControlCodec {
    type Error = ControlCodecError;

    fn encode_parts(&self, message: &ControlMessage) -> Result<EncodedParts, Self::Error> {
        encode_control(message, self.version, Some((self.role, true)))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<ControlMessage, Self::Error> {
        decode_control(frame, self.version, Some((self.role, false)))
    }
}

fn encode_control(
    message: &ControlMessage,
    version: ProtocolVersion,
    direction: Option<(ControlEndpointRole, bool)>,
) -> Result<EncodedParts, ControlCodecError> {
    validate_message(message, version, direction)?;
    let wire = PwrightControl::from(message);
    let mut payload = BinaryWriter::try_with_capacity(22)?;
    let mut context = BinaryValueContext::new(CONTROL_VALUE_LIMITS);
    wire.encode_binary(&mut payload, &mut context)?;
    Ok(EncodedParts::new(
        FrameRoute::Control,
        payload.into_bytes(),
        Vec::new(),
    ))
}

fn decode_control(
    frame: WireFrame,
    version: ProtocolVersion,
    direction: Option<(ControlEndpointRole, bool)>,
) -> Result<ControlMessage, ControlCodecError> {
    if frame.route() != FrameRoute::Control {
        return Err(ControlCodecError::WrongRoute(frame.route()));
    }
    if !frame.attachments().is_empty() {
        return Err(ControlCodecError::AttachmentsUnsupported {
            actual: frame.attachments().len(),
        });
    }
    let mut reader = BinaryReader::new(frame.payload());
    let mut context = BinaryValueContext::new(CONTROL_VALUE_LIMITS);
    let wire = PwrightControl::decode_binary(&mut reader, &mut context)?;
    reader.finish()?;
    let message = wire.try_into()?;
    validate_message(&message, version, direction)?;
    Ok(message)
}

fn validate_message(
    message: &ControlMessage,
    version: ProtocolVersion,
    direction: Option<(ControlEndpointRole, bool)>,
) -> Result<(), ControlCodecError> {
    let operation = control_operation(message);
    let required = match message {
        ControlMessage::Close | ControlMessage::CloseAck => GRACEFUL_CLOSE_VERSION,
        ControlMessage::Create { .. }
        | ControlMessage::Adopt { .. }
        | ControlMessage::Dispose { .. }
        | ControlMessage::Cancel { .. }
        | ControlMessage::SetSubscription { .. } => CONTROL_PROTOCOL_BASELINE_VERSION,
    };
    if version.major != required.major || version.minor < required.minor {
        return Err(ControlCodecError::MessageRequiresVersion {
            operation,
            negotiated: version,
            required,
        });
    }
    if let Some((role, outgoing)) = direction {
        let sender = if outgoing { role } else { opposite_role(role) };
        let allowed = match sender {
            ControlEndpointRole::Client => matches!(
                message,
                ControlMessage::Cancel { .. }
                    | ControlMessage::SetSubscription { .. }
                    | ControlMessage::Close
            ),
            ControlEndpointRole::Server => matches!(
                message,
                ControlMessage::Create { .. }
                    | ControlMessage::Adopt { .. }
                    | ControlMessage::Dispose { .. }
                    | ControlMessage::CloseAck
            ),
        };
        if !allowed {
            return Err(ControlCodecError::InvalidDirection {
                role,
                operation,
                outgoing,
            });
        }
    }
    Ok(())
}

const fn opposite_role(role: ControlEndpointRole) -> ControlEndpointRole {
    match role {
        ControlEndpointRole::Client => ControlEndpointRole::Server,
        ControlEndpointRole::Server => ControlEndpointRole::Client,
    }
}

const fn control_operation(message: &ControlMessage) -> ControlOperation {
    match message {
        ControlMessage::Create { .. } => ControlOperation::Create,
        ControlMessage::Adopt { .. } => ControlOperation::Adopt,
        ControlMessage::Dispose { .. } => ControlOperation::Dispose,
        ControlMessage::Cancel { .. } => ControlOperation::Cancel,
        ControlMessage::SetSubscription { .. } => ControlOperation::SetSubscription,
        ControlMessage::Close => ControlOperation::Close,
        ControlMessage::CloseAck => ControlOperation::CloseAck,
    }
}

impl From<&ControlMessage> for PwrightControl {
    fn from(message: &ControlMessage) -> Self {
        match *message {
            ControlMessage::Create {
                object_id,
                parent,
                kind,
            } => Self::Create {
                object_id: object_id.get(),
                parent_present: parent.is_some(),
                parent_id: parent.map_or(0, ObjectId::get),
                object_kind: kind.get(),
            },
            ControlMessage::Adopt {
                object_id,
                new_parent,
            } => Self::Adopt {
                object_id: object_id.get(),
                new_parent_id: new_parent.get(),
            },
            ControlMessage::Dispose { object_id } => Self::Dispose {
                object_id: object_id.get(),
            },
            ControlMessage::Cancel {
                call_id,
                cancellation,
                reason,
            } => Self::Cancel {
                call_id: call_id.get(),
                cancellation_id: cancellation.get(),
                reason: reason.into(),
            },
            ControlMessage::SetSubscription {
                object_id,
                event,
                enabled,
            } => Self::SetSubscription {
                object_id: object_id.get(),
                event: event.get(),
                enabled,
            },
            ControlMessage::Close => Self::Close {},
            ControlMessage::CloseAck => Self::CloseAck {},
        }
    }
}

impl TryFrom<PwrightControl> for ControlMessage {
    type Error = ControlCodecError;

    fn try_from(message: PwrightControl) -> Result<Self, Self::Error> {
        Ok(match message {
            PwrightControl::Create {
                object_id,
                parent_present,
                parent_id,
                object_kind,
            } => {
                let parent = if parent_present {
                    Some(ObjectId::new(parent_id))
                } else if parent_id == 0 {
                    None
                } else {
                    return Err(ControlCodecError::NonCanonicalAbsentParent(parent_id));
                };
                Self::Create {
                    object_id: ObjectId::new(object_id),
                    parent,
                    kind: ObjectKindTag::new(object_kind),
                }
            }
            PwrightControl::Adopt {
                object_id,
                new_parent_id,
            } => Self::Adopt {
                object_id: ObjectId::new(object_id),
                new_parent: ObjectId::new(new_parent_id),
            },
            PwrightControl::Dispose { object_id } => Self::Dispose {
                object_id: ObjectId::new(object_id),
            },
            PwrightControl::Cancel {
                call_id,
                cancellation_id,
                reason,
            } => Self::Cancel {
                call_id: CallId::new(call_id),
                cancellation: CancellationId::new(cancellation_id),
                reason: reason.into(),
            },
            PwrightControl::SetSubscription {
                object_id,
                event,
                enabled,
            } => Self::SetSubscription {
                object_id: ObjectId::new(object_id),
                event: EventTag::new(event),
                enabled,
            },
            PwrightControl::Close => Self::Close,
            PwrightControl::CloseAck => Self::CloseAck,
        })
    }
}

impl From<CancellationReason> for PwrightControlCancellationReason {
    fn from(reason: CancellationReason) -> Self {
        match reason {
            CancellationReason::Caller => Self::Caller,
            CancellationReason::DeadlineExceeded => Self::DeadlineExceeded,
            CancellationReason::ParentClosed => Self::ParentClosed,
            CancellationReason::TransportClosed => Self::TransportClosed,
        }
    }
}

impl From<PwrightControlCancellationReason> for CancellationReason {
    fn from(reason: PwrightControlCancellationReason) -> Self {
        match reason {
            PwrightControlCancellationReason::Caller => Self::Caller,
            PwrightControlCancellationReason::DeadlineExceeded => Self::DeadlineExceeded,
            PwrightControlCancellationReason::ParentClosed => Self::ParentClosed,
            PwrightControlCancellationReason::TransportClosed => Self::TransportClosed,
        }
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{
        CallId, CancellationId, CancellationReason, ControlMessage, EventTag, ObjectId,
        ObjectKindTag, ProtocolVersion,
    };
    use pwright_transport::{
        CapabilityManifest, CapabilityRequest, FrameLimits, FrameRoute, NegotiationLimits,
        ServerHandshakeConfig, ServerHello, VersionPolicy, WireCapability, WireCapabilityId,
        WireCodec, WireFrame, WireProtocol, negotiate,
    };

    use super::{
        CONTROL_PROTOCOL_BASELINE_VERSION, CONTROL_PROTOCOL_CURRENT_VERSION, CONTROL_PROTOCOL_ID,
        CONTROL_SCHEMA, CONTROL_SCHEMA_CATALOG, ControlCodec, ControlCodecError,
        ControlEndpointRole, ControlOperation, GRACEFUL_CLOSE_VERSION, NegotiatedControlCodec,
        control_protocol_requirement, control_protocol_support,
    };

    fn limits() -> FrameLimits {
        FrameLimits::new(32, 0, 0, 32)
    }

    fn protocol() -> WireProtocol<ControlCodec> {
        WireProtocol::new(
            ControlCodec,
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("version policy should build"),
            ProtocolVersion::new(1, 0),
            limits(),
        )
        .expect("wire protocol should build")
    }

    fn server_hello(control_version: Option<ProtocolVersion>) -> ServerHello {
        let enabled = control_version
            .map(|version| {
                WireCapability::new(
                    WireCapabilityId::new(CONTROL_PROTOCOL_ID, 64)
                        .expect("control protocol ID should be valid"),
                    version,
                )
            })
            .into_iter()
            .collect();
        ServerHello::new(
            ProtocolVersion::new(7, 3),
            limits(),
            limits(),
            enabled,
            NegotiationLimits::new(1, 64),
        )
        .expect("server hello should build")
    }

    #[test]
    fn every_control_message_round_trips_canonically() {
        // Arrange
        let protocol = protocol();
        let messages = [
            ControlMessage::Create {
                object_id: ObjectId::new(1),
                parent: None,
                kind: ObjectKindTag::new(2),
            },
            ControlMessage::Create {
                object_id: ObjectId::new(3),
                parent: Some(ObjectId::new(1)),
                kind: ObjectKindTag::new(4),
            },
            ControlMessage::Adopt {
                object_id: ObjectId::new(3),
                new_parent: ObjectId::new(5),
            },
            ControlMessage::Dispose {
                object_id: ObjectId::new(3),
            },
            ControlMessage::Cancel {
                call_id: CallId::new(6),
                cancellation: CancellationId::new(7),
                reason: CancellationReason::DeadlineExceeded,
            },
            ControlMessage::SetSubscription {
                object_id: ObjectId::new(8),
                event: EventTag::new(9),
                enabled: true,
            },
        ];

        // Act
        let round_trips = messages.map(|message| {
            let frame = protocol.encode(&message).expect("message should encode");
            let decoded = protocol.decode(frame).expect("message should decode");
            (message, decoded)
        });

        // Assert
        for (expected, actual) in round_trips {
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn control_schema_is_valid_and_dispose_bytes_are_stable() {
        // Arrange
        let message = ControlMessage::Dispose {
            object_id: ObjectId::new(0x0102_0304_0506_0708),
        };

        // Act
        let encoded = ControlCodec
            .encode_parts(&message)
            .expect("dispose should encode");

        // Assert
        assert_eq!(CONTROL_SCHEMA.validate(), Ok(()));
        assert_eq!(CONTROL_SCHEMA_CATALOG.validate(), Ok(()));
        assert_eq!(CONTROL_SCHEMA.version(), CONTROL_PROTOCOL_CURRENT_VERSION);
        assert_eq!(
            encoded.payload(),
            &[CONTROL_SCHEMA.variants()[2].tag(), 1, 2, 3, 4, 5, 6, 7, 8,]
        );
    }

    #[test]
    fn baseline_codec_rejects_graceful_close_before_encoding() {
        // Arrange
        let message = ControlMessage::Close;

        // Act
        let result = ControlCodec.encode_parts(&message);

        // Assert
        assert_eq!(
            result,
            Err(ControlCodecError::MessageRequiresVersion {
                operation: ControlOperation::Close,
                negotiated: CONTROL_PROTOCOL_BASELINE_VERSION,
                required: GRACEFUL_CLOSE_VERSION,
            })
        );
    }

    #[test]
    fn negotiated_role_codecs_exchange_close_and_ack_on_application_version() {
        // Arrange
        let hello = server_hello(Some(CONTROL_PROTOCOL_CURRENT_VERSION));
        let client = NegotiatedControlCodec::for_client(&hello)
            .expect("client control codec should use negotiated version");
        let server = NegotiatedControlCodec::for_server(&hello)
            .expect("server control codec should use negotiated version");
        let outer_version = hello.selected_version();
        let client_protocol =
            WireProtocol::new_exact_directional(client, outer_version, limits(), limits());
        let server_protocol =
            WireProtocol::new_exact_directional(server, outer_version, limits(), limits());

        // Act
        let close = client_protocol
            .encode(&ControlMessage::Close)
            .expect("negotiated client should encode close");
        let close_version = close.version();
        let decoded_close = server_protocol
            .decode::<ControlMessage>(close)
            .expect("negotiated server should decode close");
        let ack = server_protocol
            .encode(&ControlMessage::CloseAck)
            .expect("negotiated server should encode close ack");
        let ack_version = ack.version();
        let decoded_ack = client_protocol
            .decode::<ControlMessage>(ack)
            .expect("negotiated client should decode close ack");

        // Assert
        assert_eq!(client.version(), ProtocolVersion::new(1, 1));
        assert_eq!(server.version(), ProtocolVersion::new(1, 1));
        assert_eq!(close_version, ProtocolVersion::new(7, 3));
        assert_eq!(ack_version, ProtocolVersion::new(7, 3));
        assert_eq!(decoded_close, ControlMessage::Close);
        assert_eq!(decoded_ack, ControlMessage::CloseAck);
    }

    #[test]
    fn negotiated_codec_rejects_wrong_direction() {
        // Arrange
        let codec = NegotiatedControlCodec::new(
            ControlEndpointRole::Client,
            CONTROL_PROTOCOL_CURRENT_VERSION,
        )
        .expect("supported version should build");

        // Act
        let result = codec.encode_parts(&ControlMessage::CloseAck);

        // Assert
        assert_eq!(
            result,
            Err(ControlCodecError::InvalidDirection {
                role: ControlEndpointRole::Client,
                operation: ControlOperation::CloseAck,
                outgoing: true,
            })
        );
    }

    #[test]
    fn absent_control_offer_preserves_the_legacy_baseline() {
        // Arrange
        let hello = server_hello(None);

        // Act
        let codec = NegotiatedControlCodec::for_client(&hello)
            .expect("legacy baseline should remain usable");
        let close = codec.encode_parts(&ControlMessage::Close);

        // Assert
        assert_eq!(codec.version(), CONTROL_PROTOCOL_BASELINE_VERSION);
        assert!(matches!(
            close,
            Err(ControlCodecError::MessageRequiresVersion {
                operation: ControlOperation::Close,
                ..
            })
        ));
    }

    #[test]
    fn handshake_selects_control_minor_independently() {
        // Arrange
        let negotiation_limits = NegotiationLimits::new(1, 64);
        let client = pwright_transport::ClientHello::new(
            VersionPolicy::exact(ProtocolVersion::new(3, 4)),
            limits(),
            CapabilityRequest::new(
                Vec::new(),
                vec![control_protocol_requirement().expect("control requirement should be valid")],
                negotiation_limits,
            )
            .expect("client capability request should build"),
        );
        let server = ServerHandshakeConfig::new(
            VersionPolicy::exact(ProtocolVersion::new(3, 4)),
            limits(),
            CapabilityManifest::from_support(
                vec![control_protocol_support().expect("control support should be valid")],
                negotiation_limits,
            )
            .expect("server capability manifest should build"),
        );

        // Act
        let hello = negotiate(&client, &server).expect("control protocol should negotiate");
        let codec = NegotiatedControlCodec::for_client(&hello)
            .expect("selected control codec should build");

        // Assert
        assert_eq!(hello.selected_version(), ProtocolVersion::new(3, 4));
        assert_eq!(codec.version(), ProtocolVersion::new(1, 1));
    }

    #[test]
    fn wrong_route_and_attachments_are_rejected() {
        // Arrange
        let wrong_route = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Handshake,
            vec![2, 0, 0, 0, 0, 0, 0, 0, 1],
            Vec::new(),
            limits(),
        )
        .expect("wrong-route frame should build");
        let attachments = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            vec![2, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![vec![]],
            FrameLimits::new(32, 1, 0, 32),
        )
        .expect("attachment frame should build");

        // Act
        let route_error = ControlCodec.decode_frame(wrong_route);
        let attachment_error = ControlCodec.decode_frame(attachments);

        // Assert
        assert_eq!(
            route_error,
            Err(ControlCodecError::WrongRoute(FrameRoute::Handshake))
        );
        assert_eq!(
            attachment_error,
            Err(ControlCodecError::AttachmentsUnsupported { actual: 1 })
        );
    }

    #[test]
    fn malformed_discriminants_are_rejected() {
        // Arrange
        let frames = [
            (vec![99], ControlCodecError::UnknownMessage(99)),
            (
                vec![3, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 9],
                ControlCodecError::InvalidCancellationReason(9),
            ),
            (
                vec![4, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 9],
                ControlCodecError::InvalidBoolean(9),
            ),
        ];

        // Act / Assert
        for (payload, expected) in frames {
            let frame = WireFrame::new(
                ProtocolVersion::new(1, 0),
                FrameRoute::Control,
                payload,
                Vec::new(),
                limits(),
            )
            .expect("malformed payload should fit frame limits");
            assert_eq!(ControlCodec.decode_frame(frame), Err(expected));
        }
    }

    #[test]
    fn absent_parent_encoding_must_be_canonical() {
        // Arrange
        let payload = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 3,
        ];
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            payload,
            Vec::new(),
            limits(),
        )
        .expect("payload should fit frame limits");

        // Act
        let outcome = ControlCodec.decode_frame(frame);

        // Assert
        assert_eq!(outcome, Err(ControlCodecError::NonCanonicalAbsentParent(2)));
    }

    #[test]
    fn truncated_and_trailing_payloads_are_rejected() {
        // Arrange
        let truncated = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            vec![2, 0],
            Vec::new(),
            limits(),
        )
        .expect("truncated payload should fit limits");
        let trailing = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            vec![2, 0, 0, 0, 0, 0, 0, 0, 1, 0],
            Vec::new(),
            limits(),
        )
        .expect("trailing payload should fit limits");

        // Act
        let truncated_error = ControlCodec.decode_frame(truncated);
        let trailing_error = ControlCodec.decode_frame(trailing);

        // Assert
        assert!(matches!(
            truncated_error,
            Err(ControlCodecError::Truncated { .. })
        ));
        assert_eq!(
            trailing_error,
            Err(ControlCodecError::TrailingBytes { remaining: 1 })
        );
    }
}
