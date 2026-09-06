//! Canonical, Serde-free binary payloads for Pwright connection negotiation.
//!
//! The first frame's outer protocol version is intentionally not decoded here:
//! the version ranges inside `ClientHello` are the authority for negotiation.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::Utf8Error;

use pwright_codec_binary::{
    BinaryDecodeError, BinaryEncode, BinaryEncodeError, BinaryEncodeSequence,
    BinaryEncodeSequenceRef, BinaryValueContext, BinaryValueDecodeError, BinaryValueEncodeError,
    BinaryValueLimits, BinaryValueResource, BinaryWriter, IntegerWidth, decode_value, encode_value,
};
use pwright_core::ProtocolVersion;
use pwright_transport::{
    CapabilityIdError, CapabilityRequest, CapabilityRequirement, CapabilitySetError, ClientHello,
    EncodedParts, FrameLimits, FrameRoute, NegotiationLimits, ServerHello, VersionPolicy,
    VersionPolicyError, WireCapability, WireCapabilityId, WireCodec, WireFrame,
};

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

pub use generated_schema::{CATALOG as HANDSHAKE_SCHEMA_CATALOG, ROOT_SCHEMA as HANDSHAKE_SCHEMA};
use generated_schema::{
    PwrightHandshake, PwrightHandshakeEnabledCapabilityRef, PwrightHandshakeFrameLimits,
    PwrightHandshakeLegacyRequirement, PwrightHandshakeRangedRequirement,
    PwrightHandshakeRangedRequirementRef, PwrightHandshakeRef, PwrightHandshakeVersion,
    PwrightHandshakeVersionPolicy,
};

const LEGACY_CLIENT_HELLO: u8 = HANDSHAKE_SCHEMA.variants()[0].tag();
const SERVER_HELLO: u8 = HANDSHAKE_SCHEMA.variants()[1].tag();
const RANGED_CLIENT_HELLO: u8 = HANDSHAKE_SCHEMA.variants()[2].tag();

#[derive(Clone, Copy, Debug)]
pub struct HandshakeCodec {
    limits: NegotiationLimits,
}

impl HandshakeCodec {
    pub const fn new(limits: NegotiationLimits) -> Self {
        Self { limits }
    }

    pub const fn limits(self) -> NegotiationLimits {
        self.limits
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelloKind {
    Client,
    Server,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HandshakeCodecError {
    WrongRoute(FrameRoute),
    AttachmentsUnsupported {
        actual: usize,
    },
    WrongHelloKind {
        expected: HelloKind,
        actual: u8,
    },
    TooManyCapabilities {
        actual: usize,
        limit: usize,
    },
    CapabilityIdTooLongForWire {
        actual: usize,
    },
    AllocationFailed {
        entries: usize,
    },
    Encode(BinaryEncodeError),
    ValueEncode(BinaryValueEncodeError),
    ValueDecode(BinaryValueDecodeError),
    InvalidCapabilityId(CapabilityIdError),
    InvalidCapabilitySet(CapabilitySetError),
    InvalidVersionPolicy(VersionPolicyError),
    NonCanonicalCapabilityOrder,
    InvalidUtf8(Utf8Error),
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

impl Display for HandshakeCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => {
                write!(formatter, "hello payload arrived on route {route:?}")
            }
            Self::AttachmentsUnsupported { actual } => write!(
                formatter,
                "hello payload has {actual} attachment(s), but none are allowed"
            ),
            Self::WrongHelloKind { expected, actual } => {
                write!(
                    formatter,
                    "expected {expected:?} hello tag, received {actual}"
                )
            }
            Self::TooManyCapabilities { actual, limit } => write!(
                formatter,
                "hello has {actual} capabilities, exceeding limit {limit}"
            ),
            Self::CapabilityIdTooLongForWire { actual } => write!(
                formatter,
                "capability ID has {actual} bytes, exceeding the u32 wire length"
            ),
            Self::AllocationFailed { entries } => write!(
                formatter,
                "could not allocate storage for {entries} negotiated capabilities"
            ),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::ValueEncode(error) => Display::fmt(error, formatter),
            Self::ValueDecode(error) => Display::fmt(error, formatter),
            Self::InvalidCapabilityId(error) => Display::fmt(error, formatter),
            Self::InvalidCapabilitySet(error) => Display::fmt(error, formatter),
            Self::InvalidVersionPolicy(error) => Display::fmt(error, formatter),
            Self::NonCanonicalCapabilityOrder => {
                formatter.write_str("capability IDs are not in canonical ascending order")
            }
            Self::InvalidUtf8(error) => write!(formatter, "capability ID is not UTF-8: {error}"),
            Self::IntegerOutOfRange { field, value } => {
                write!(
                    formatter,
                    "wire value {value} does not fit host field {field}"
                )
            }
            Self::Truncated {
                offset,
                needed,
                remaining,
            } => write!(
                formatter,
                "hello payload at byte {offset} needs {needed} bytes but only {remaining} remain"
            ),
            Self::TrailingBytes { remaining } => {
                write!(formatter, "hello payload has {remaining} trailing bytes")
            }
        }
    }
}

impl Error for HandshakeCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidCapabilityId(error) => Some(error),
            Self::InvalidCapabilitySet(error) => Some(error),
            Self::InvalidVersionPolicy(error) => Some(error),
            Self::InvalidUtf8(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::ValueEncode(error) => Some(error),
            Self::ValueDecode(error) => Some(error),
            _ => None,
        }
    }
}

impl From<BinaryEncodeError> for HandshakeCodecError {
    fn from(error: BinaryEncodeError) -> Self {
        Self::Encode(error)
    }
}

impl From<BinaryDecodeError> for HandshakeCodecError {
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

impl WireCodec<ClientHello> for HandshakeCodec {
    type Error = HandshakeCodecError;

    fn encode_parts(&self, message: &ClientHello) -> Result<EncodedParts, Self::Error> {
        let required = RangedRequirements::required(message.capabilities());
        let optional = RangedRequirements::optional(message.capabilities());
        self.ensure_capability_count(required.len().saturating_add(optional.len()))?;
        for (id, _) in message
            .capabilities()
            .required_ranges()
            .chain(message.capabilities().optional_ranges())
        {
            self.validate_capability_id_for_encode(id)?;
        }
        let wire = PwrightHandshakeRef::RangedClientHello {
            versions: policy_to_wire(message.versions()),
            receive_limits: frame_limits_to_wire(message.receive_limits())?,
            required_count: wire_count(required.len())?,
            optional_count: wire_count(optional.len())?,
            required_capabilities: BinaryEncodeSequenceRef::new(&required),
            optional_capabilities: BinaryEncodeSequenceRef::new(&optional),
        };
        let payload = encode_value(&wire, self.value_limits()).map_err(map_generated_encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Handshake,
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<ClientHello, Self::Error> {
        let payload = self.payload(&frame)?;
        let wire = decode_value::<PwrightHandshake>(payload, self.value_limits())
            .map_err(|error| map_generated_decode(error, HelloKind::Client))?;
        let (versions, receive_limits, required, optional) = match wire {
            PwrightHandshake::LegacyClientHello {
                versions,
                receive_limits,
                required_count: _,
                optional_count: _,
                required_capabilities,
                optional_capabilities,
            } => (
                policy_from_wire(versions)?,
                frame_limits_from_wire(receive_limits)?,
                self.legacy_requirements_from_wire(required_capabilities)?,
                self.legacy_requirements_from_wire(optional_capabilities)?,
            ),
            PwrightHandshake::RangedClientHello {
                versions,
                receive_limits,
                required_count: _,
                optional_count: _,
                required_capabilities,
                optional_capabilities,
            } => (
                policy_from_wire(versions)?,
                frame_limits_from_wire(receive_limits)?,
                self.ranged_requirements_from_wire(required_capabilities)?,
                self.ranged_requirements_from_wire(optional_capabilities)?,
            ),
            other => {
                return Err(HandshakeCodecError::WrongHelloKind {
                    expected: HelloKind::Client,
                    actual: handshake_wire_kind(&other),
                });
            }
        };
        self.ensure_capability_count(required.len().saturating_add(optional.len()))?;
        let capabilities = CapabilityRequest::new(required, optional, self.limits)
            .map_err(HandshakeCodecError::InvalidCapabilitySet)?;
        Ok(ClientHello::new(versions, receive_limits, capabilities))
    }
}

impl WireCodec<ServerHello> for HandshakeCodec {
    type Error = HandshakeCodecError;

    fn encode_parts(&self, message: &ServerHello) -> Result<EncodedParts, Self::Error> {
        self.ensure_capability_count(message.enabled_capabilities().len())?;
        for capability in message.enabled_capabilities() {
            self.validate_capability_id_for_encode(capability.id())?;
        }
        let enabled = EnabledCapabilities::new(message.enabled_capabilities());
        let wire = PwrightHandshakeRef::ServerHello {
            selected_version: version_to_wire(message.selected_version()),
            client_receive_limits: frame_limits_to_wire(message.client_receive_limits())?,
            server_receive_limits: frame_limits_to_wire(message.server_receive_limits())?,
            enabled_count: wire_count(enabled.len())?,
            enabled_capabilities: BinaryEncodeSequenceRef::new(&enabled),
        };
        let payload = encode_value(&wire, self.value_limits()).map_err(map_generated_encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Handshake,
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<ServerHello, Self::Error> {
        let payload = self.payload(&frame)?;
        let wire = decode_value::<PwrightHandshake>(payload, self.value_limits())
            .map_err(|error| map_generated_decode(error, HelloKind::Server))?;
        let PwrightHandshake::ServerHello {
            selected_version,
            client_receive_limits,
            server_receive_limits,
            enabled_count: _,
            enabled_capabilities,
        } = wire
        else {
            return Err(HandshakeCodecError::WrongHelloKind {
                expected: HelloKind::Server,
                actual: handshake_wire_kind(&wire),
            });
        };
        self.ensure_capability_count(enabled_capabilities.len())?;
        let enabled_capabilities = self.capabilities_from_wire(enabled_capabilities)?;
        ServerHello::new(
            version_from_wire(selected_version),
            frame_limits_from_wire(client_receive_limits)?,
            frame_limits_from_wire(server_receive_limits)?,
            enabled_capabilities,
            self.limits,
        )
        .map_err(HandshakeCodecError::InvalidCapabilitySet)
    }
}

impl HandshakeCodec {
    fn payload<'a>(&self, frame: &'a WireFrame) -> Result<&'a [u8], HandshakeCodecError> {
        if frame.route() != FrameRoute::Handshake {
            return Err(HandshakeCodecError::WrongRoute(frame.route()));
        }
        if !frame.attachments().is_empty() {
            return Err(HandshakeCodecError::AttachmentsUnsupported {
                actual: frame.attachments().len(),
            });
        }
        Ok(frame.payload())
    }

    fn ensure_capability_count(&self, actual: usize) -> Result<(), HandshakeCodecError> {
        if actual > self.limits.capabilities {
            return Err(HandshakeCodecError::TooManyCapabilities {
                actual,
                limit: self.limits.capabilities,
            });
        }
        Ok(())
    }

    fn value_limits(&self) -> BinaryValueLimits {
        BinaryValueLimits::new(
            self.limits.capability_id_bytes,
            0,
            self.limits.capabilities,
            4,
            self.limits.capabilities.saturating_mul(8).saturating_add(5),
        )
    }

    fn validate_capability_id_for_encode(
        &self,
        id: &WireCapabilityId,
    ) -> Result<(), HandshakeCodecError> {
        let actual = id.as_str().len();
        if actual > self.limits.capability_id_bytes {
            return Err(HandshakeCodecError::InvalidCapabilityId(
                CapabilityIdError::TooLong {
                    actual,
                    limit: self.limits.capability_id_bytes,
                },
            ));
        }
        u32::try_from(actual)
            .map_err(|_| HandshakeCodecError::CapabilityIdTooLongForWire { actual })?;
        Ok(())
    }

    fn legacy_requirements_from_wire(
        &self,
        values: Vec<PwrightHandshakeLegacyRequirement>,
    ) -> Result<Vec<CapabilityRequirement>, HandshakeCodecError> {
        let mut items: Vec<CapabilityRequirement> = Vec::new();
        items.try_reserve_exact(values.len()).map_err(|_| {
            HandshakeCodecError::AllocationFailed {
                entries: values.len(),
            }
        })?;
        for value in values {
            let id = self.capability_id_from_wire(value.id, items.last().map(|item| item.id()))?;
            items.push(CapabilityRequirement::new(
                id,
                version_from_wire(value.version),
            ));
        }
        Ok(items)
    }

    fn ranged_requirements_from_wire(
        &self,
        values: Vec<PwrightHandshakeRangedRequirement>,
    ) -> Result<Vec<CapabilityRequirement>, HandshakeCodecError> {
        let mut items: Vec<CapabilityRequirement> = Vec::new();
        items.try_reserve_exact(values.len()).map_err(|_| {
            HandshakeCodecError::AllocationFailed {
                entries: values.len(),
            }
        })?;
        for value in values {
            let id = self.capability_id_from_wire(value.id, items.last().map(|item| item.id()))?;
            items.push(CapabilityRequirement::within(
                id,
                policy_from_wire(value.versions)?,
            ));
        }
        Ok(items)
    }

    fn capabilities_from_wire(
        &self,
        values: Vec<generated_schema::PwrightHandshakeEnabledCapability>,
    ) -> Result<Vec<WireCapability>, HandshakeCodecError> {
        let mut items: Vec<WireCapability> = Vec::new();
        items.try_reserve_exact(values.len()).map_err(|_| {
            HandshakeCodecError::AllocationFailed {
                entries: values.len(),
            }
        })?;
        for value in values {
            let id =
                self.capability_id_from_wire(value.id, items.last().map(WireCapability::id))?;
            items.push(WireCapability::new(id, version_from_wire(value.version)));
        }
        Ok(items)
    }

    fn capability_id_from_wire(
        &self,
        value: String,
        previous: Option<&WireCapabilityId>,
    ) -> Result<WireCapabilityId, HandshakeCodecError> {
        let id = WireCapabilityId::new(value, self.limits.capability_id_bytes)
            .map_err(HandshakeCodecError::InvalidCapabilityId)?;
        if previous.is_some_and(|previous| previous >= &id) {
            return Err(HandshakeCodecError::NonCanonicalCapabilityOrder);
        }
        Ok(id)
    }
}

#[derive(Clone, Copy)]
enum RequirementSet {
    Required,
    Optional,
}

struct RangedRequirements<'a> {
    request: &'a CapabilityRequest,
    set: RequirementSet,
}

impl<'a> RangedRequirements<'a> {
    const fn required(request: &'a CapabilityRequest) -> Self {
        Self {
            request,
            set: RequirementSet::Required,
        }
    }

    const fn optional(request: &'a CapabilityRequest) -> Self {
        Self {
            request,
            set: RequirementSet::Optional,
        }
    }
}

impl BinaryEncodeSequence for RangedRequirements<'_> {
    fn len(&self) -> usize {
        match self.set {
            RequirementSet::Required => self.request.required_ranges().count(),
            RequirementSet::Optional => self.request.optional_ranges().count(),
        }
    }

    fn encode_items(
        &self,
        writer: &mut BinaryWriter,
        context: &mut BinaryValueContext,
    ) -> Result<(), BinaryValueEncodeError> {
        match self.set {
            RequirementSet::Required => {
                for (id, versions) in self.request.required_ranges() {
                    PwrightHandshakeRangedRequirementRef {
                        id: id.as_str(),
                        versions: policy_to_wire(versions),
                    }
                    .encode_binary(writer, context)?;
                }
            }
            RequirementSet::Optional => {
                for (id, versions) in self.request.optional_ranges() {
                    PwrightHandshakeRangedRequirementRef {
                        id: id.as_str(),
                        versions: policy_to_wire(versions),
                    }
                    .encode_binary(writer, context)?;
                }
            }
        }
        Ok(())
    }
}

struct EnabledCapabilities<'a> {
    capabilities: &'a [WireCapability],
}

impl<'a> EnabledCapabilities<'a> {
    const fn new(capabilities: &'a [WireCapability]) -> Self {
        Self { capabilities }
    }
}

impl BinaryEncodeSequence for EnabledCapabilities<'_> {
    fn len(&self) -> usize {
        self.capabilities.len()
    }

    fn encode_items(
        &self,
        writer: &mut BinaryWriter,
        context: &mut BinaryValueContext,
    ) -> Result<(), BinaryValueEncodeError> {
        for capability in self.capabilities {
            PwrightHandshakeEnabledCapabilityRef {
                id: capability.id().as_str(),
                version: version_to_wire(capability.version()),
            }
            .encode_binary(writer, context)?;
        }
        Ok(())
    }
}

fn wire_count(value: usize) -> Result<u32, HandshakeCodecError> {
    u32::try_from(value).map_err(|_| {
        HandshakeCodecError::Encode(BinaryEncodeError::IntegerOutOfRange {
            field: "count",
            value: value as u128,
            target: IntegerWidth::U32,
        })
    })
}

fn version_to_wire(version: ProtocolVersion) -> PwrightHandshakeVersion {
    PwrightHandshakeVersion {
        major: version.major,
        minor: version.minor,
    }
}

fn version_from_wire(version: PwrightHandshakeVersion) -> ProtocolVersion {
    ProtocolVersion::new(version.major, version.minor)
}

fn policy_to_wire(policy: VersionPolicy) -> PwrightHandshakeVersionPolicy {
    PwrightHandshakeVersionPolicy {
        minimum: version_to_wire(policy.minimum()),
        maximum: version_to_wire(policy.maximum()),
    }
}

fn policy_from_wire(
    policy: PwrightHandshakeVersionPolicy,
) -> Result<VersionPolicy, HandshakeCodecError> {
    VersionPolicy::new(
        version_from_wire(policy.minimum),
        version_from_wire(policy.maximum),
    )
    .map_err(HandshakeCodecError::InvalidVersionPolicy)
}

fn frame_limits_to_wire(
    limits: FrameLimits,
) -> Result<PwrightHandshakeFrameLimits, HandshakeCodecError> {
    Ok(PwrightHandshakeFrameLimits {
        payload_bytes: usize_to_wire_u64("payload_bytes", limits.payload_bytes)?,
        attachments: usize_to_wire_u64("attachments", limits.attachments)?,
        attachment_bytes: usize_to_wire_u64("attachment_bytes", limits.attachment_bytes)?,
        total_bytes: usize_to_wire_u64("total_bytes", limits.total_bytes)?,
    })
}

fn frame_limits_from_wire(
    limits: PwrightHandshakeFrameLimits,
) -> Result<FrameLimits, HandshakeCodecError> {
    Ok(FrameLimits::new(
        wire_u64_to_usize("payload_bytes", limits.payload_bytes)?,
        wire_u64_to_usize("attachments", limits.attachments)?,
        wire_u64_to_usize("attachment_bytes", limits.attachment_bytes)?,
        wire_u64_to_usize("total_bytes", limits.total_bytes)?,
    ))
}

fn usize_to_wire_u64(field: &'static str, value: usize) -> Result<u64, HandshakeCodecError> {
    u64::try_from(value).map_err(|_| {
        HandshakeCodecError::Encode(BinaryEncodeError::IntegerOutOfRange {
            field,
            value: value as u128,
            target: IntegerWidth::U64,
        })
    })
}

fn wire_u64_to_usize(field: &'static str, value: u64) -> Result<usize, HandshakeCodecError> {
    usize::try_from(value).map_err(|_| HandshakeCodecError::IntegerOutOfRange { field, value })
}

fn handshake_wire_kind(value: &PwrightHandshake) -> u8 {
    match value {
        PwrightHandshake::LegacyClientHello { .. } => LEGACY_CLIENT_HELLO,
        PwrightHandshake::ServerHello { .. } => SERVER_HELLO,
        PwrightHandshake::RangedClientHello { .. } => RANGED_CLIENT_HELLO,
    }
}

fn map_generated_encode(error: BinaryValueEncodeError) -> HandshakeCodecError {
    match error {
        BinaryValueEncodeError::Limit(error)
            if error.resource == BinaryValueResource::StringBytes =>
        {
            HandshakeCodecError::InvalidCapabilityId(CapabilityIdError::TooLong {
                actual: error.actual,
                limit: error.limit,
            })
        }
        BinaryValueEncodeError::Limit(error)
            if error.resource == BinaryValueResource::RepeatedItems =>
        {
            HandshakeCodecError::TooManyCapabilities {
                actual: error.actual,
                limit: error.limit,
            }
        }
        BinaryValueEncodeError::Primitive(error) => HandshakeCodecError::Encode(error),
        error => HandshakeCodecError::ValueEncode(error),
    }
}

fn map_generated_decode(error: BinaryValueDecodeError, expected: HelloKind) -> HandshakeCodecError {
    match error {
        BinaryValueDecodeError::Limit(error)
            if error.resource == BinaryValueResource::StringBytes =>
        {
            HandshakeCodecError::InvalidCapabilityId(CapabilityIdError::TooLong {
                actual: error.actual,
                limit: error.limit,
            })
        }
        BinaryValueDecodeError::Limit(error)
            if error.resource == BinaryValueResource::RepeatedItems =>
        {
            HandshakeCodecError::TooManyCapabilities {
                actual: error.actual,
                limit: error.limit,
            }
        }
        BinaryValueDecodeError::Primitive(error) => error.into(),
        BinaryValueDecodeError::InvalidUtf8 { source, .. } => {
            HandshakeCodecError::InvalidUtf8(source)
        }
        BinaryValueDecodeError::InvalidVariantTag {
            type_id: "pwright.handshake",
            value,
        } => HandshakeCodecError::WrongHelloKind {
            expected,
            actual: value,
        },
        BinaryValueDecodeError::AllocationFailed { entries, .. } => {
            HandshakeCodecError::AllocationFailed { entries }
        }
        error => HandshakeCodecError::ValueDecode(error),
    }
}

#[cfg(test)]
mod tests {
    use pwright_codec_binary::{BinaryValueEncodeError, encode_value};
    use pwright_core::ProtocolVersion;
    use pwright_transport::{
        CapabilityManifest, CapabilityRequest, CapabilityRequirement, ClientHello, FrameLimits,
        FrameRoute, NegotiationLimits, ServerHello, VersionPolicy, WireCapability,
        WireCapabilityId, WireCodec, WireFrame,
    };

    use super::{
        HANDSHAKE_SCHEMA, HANDSHAKE_SCHEMA_CATALOG, HandshakeCodec, HandshakeCodecError, HelloKind,
    };
    use crate::generated_schema::{
        PwrightHandshake, PwrightHandshakeFrameLimits, PwrightHandshakeVersion,
    };

    fn limits() -> NegotiationLimits {
        NegotiationLimits::new(4, 32)
    }

    fn frames() -> FrameLimits {
        FrameLimits::new(4096, 0, 0, 4096)
    }

    fn version(minor: u16) -> ProtocolVersion {
        ProtocolVersion::new(1, minor)
    }

    fn policy(minimum: u16, maximum: u16) -> VersionPolicy {
        VersionPolicy::new(version(minimum), version(maximum)).expect("policy should build")
    }

    fn id(value: &str) -> WireCapabilityId {
        WireCapabilityId::new(value, 32).expect("ID should build")
    }

    fn client() -> ClientHello {
        ClientHello::new(
            policy(0, 2),
            FrameLimits::new(100, 2, 200, 300),
            CapabilityRequest::new(
                vec![CapabilityRequirement::new(id("navigation"), version(1))],
                vec![CapabilityRequirement::new(id("tracing"), version(0))],
                limits(),
            )
            .expect("request should build"),
        )
    }

    fn server() -> ServerHello {
        ServerHello::new(
            version(2),
            FrameLimits::new(100, 2, 200, 300),
            FrameLimits::new(400, 3, 500, 600),
            vec![
                WireCapability::new(id("tracing"), version(3)),
                WireCapability::new(id("navigation"), version(2)),
            ],
            limits(),
        )
        .expect("server hello should build")
    }

    fn frame(payload: Vec<u8>) -> WireFrame {
        WireFrame::new(
            version(0),
            FrameRoute::Handshake,
            payload,
            Vec::new(),
            frames(),
        )
        .expect("frame should build")
    }

    fn decode_client(
        codec: &HandshakeCodec,
        frame: WireFrame,
    ) -> Result<ClientHello, HandshakeCodecError> {
        <HandshakeCodec as WireCodec<ClientHello>>::decode_frame(codec, frame)
    }

    #[test]
    fn client_hello_round_trips_canonically() {
        // Arrange
        let codec = HandshakeCodec::new(limits());
        let expected = client();

        // Act
        let parts = codec.encode_parts(&expected).expect("client should encode");
        let actual =
            decode_client(&codec, frame(parts.payload().to_vec())).expect("client should decode");

        // Assert
        assert_eq!(actual, expected);
        assert_eq!(parts.route(), FrameRoute::Handshake);
        assert!(parts.attachments().is_empty());
    }

    #[test]
    fn ranged_client_hello_preserves_capability_minimum_and_maximum() {
        // Arrange
        let codec = HandshakeCodec::new(limits());
        let supported = policy(1, 2);
        let expected = ClientHello::new(
            policy(0, 2),
            frames(),
            CapabilityRequest::new(
                vec![CapabilityRequirement::within(id("navigation"), supported)],
                Vec::new(),
                limits(),
            )
            .expect("ranged request should build"),
        );

        // Act
        let parts = codec.encode_parts(&expected).expect("client should encode");
        let actual = decode_client(&codec, frame(parts.payload().to_vec()))
            .expect("ranged client should decode");

        // Assert
        assert_eq!(HANDSHAKE_SCHEMA.validate(), Ok(()));
        assert_eq!(parts.payload()[0], super::RANGED_CLIENT_HELLO);
        assert_eq!(actual, expected);
        assert_eq!(
            actual
                .capabilities()
                .required_ranges()
                .next()
                .map(|(_, versions)| versions),
            Some(supported)
        );
    }

    #[test]
    fn legacy_minimum_only_client_hello_remains_decodable() {
        // Arrange
        let codec = HandshakeCodec::new(limits());
        let mut payload = vec![super::LEGACY_CLIENT_HELLO];
        for component in [1_u16, 0, 1, 2] {
            payload.extend_from_slice(&component.to_be_bytes());
        }
        for value in [100_u64, 2, 200, 300] {
            payload.extend_from_slice(&value.to_be_bytes());
        }
        payload.extend_from_slice(&1_u32.to_be_bytes());
        payload.extend_from_slice(&0_u32.to_be_bytes());
        payload.extend_from_slice(&3_u32.to_be_bytes());
        payload.extend_from_slice(b"nav");
        payload.extend_from_slice(&1_u16.to_be_bytes());
        payload.extend_from_slice(&1_u16.to_be_bytes());

        // Act
        let decoded = decode_client(&codec, frame(payload)).expect("legacy hello should decode");

        // Assert
        let (_, versions) = decoded
            .capabilities()
            .required_ranges()
            .next()
            .expect("legacy requirement should exist");
        assert_eq!(versions.minimum(), version(1));
        assert_eq!(versions.maximum(), ProtocolVersion::new(1, u16::MAX));
    }

    #[test]
    fn empty_ranged_client_hello_has_stable_golden_bytes() {
        // Arrange
        let codec = HandshakeCodec::new(limits());
        let hello = ClientHello::new(
            VersionPolicy::exact(version(0)),
            FrameLimits::new(1, 2, 3, 4),
            CapabilityRequest::new(Vec::new(), Vec::new(), limits())
                .expect("empty request should build"),
        );
        let mut expected = vec![super::RANGED_CLIENT_HELLO];
        for component in [1_u16, 0, 1, 0] {
            expected.extend_from_slice(&component.to_be_bytes());
        }
        for value in [1_u64, 2, 3, 4] {
            expected.extend_from_slice(&value.to_be_bytes());
        }
        expected.extend_from_slice(&0_u32.to_be_bytes());
        expected.extend_from_slice(&0_u32.to_be_bytes());

        // Act
        let encoded = codec.encode_parts(&hello).expect("hello should encode");

        // Assert
        assert_eq!(encoded.payload(), expected);
    }

    #[test]
    fn non_empty_client_and_server_hellos_keep_the_published_segmented_layout() {
        // Arrange
        let codec = HandshakeCodec::new(limits());
        let mut expected_client = vec![super::RANGED_CLIENT_HELLO];
        for component in [1_u16, 0, 1, 2] {
            expected_client.extend_from_slice(&component.to_be_bytes());
        }
        for value in [100_u64, 2, 200, 300] {
            expected_client.extend_from_slice(&value.to_be_bytes());
        }
        expected_client.extend_from_slice(&1_u32.to_be_bytes());
        expected_client.extend_from_slice(&1_u32.to_be_bytes());
        for (name, minimum, maximum) in [
            ("navigation", 1_u16, u16::MAX),
            ("tracing", 0_u16, u16::MAX),
        ] {
            expected_client.extend_from_slice(&(name.len() as u32).to_be_bytes());
            expected_client.extend_from_slice(name.as_bytes());
            for component in [1_u16, minimum, 1, maximum] {
                expected_client.extend_from_slice(&component.to_be_bytes());
            }
        }
        let mut expected_server = vec![super::SERVER_HELLO];
        for component in [1_u16, 2] {
            expected_server.extend_from_slice(&component.to_be_bytes());
        }
        for value in [100_u64, 2, 200, 300, 400, 3, 500, 600] {
            expected_server.extend_from_slice(&value.to_be_bytes());
        }
        expected_server.extend_from_slice(&2_u32.to_be_bytes());
        for (name, minor) in [("navigation", 2_u16), ("tracing", 3_u16)] {
            expected_server.extend_from_slice(&(name.len() as u32).to_be_bytes());
            expected_server.extend_from_slice(name.as_bytes());
            for component in [1_u16, minor] {
                expected_server.extend_from_slice(&component.to_be_bytes());
            }
        }

        // Act
        let encoded_client = codec.encode_parts(&client()).expect("client should encode");
        let encoded_server =
            <HandshakeCodec as WireCodec<ServerHello>>::encode_parts(&codec, &server())
                .expect("server should encode");

        // Assert
        assert_eq!(encoded_client.payload(), expected_client);
        assert_eq!(encoded_server.payload(), expected_server);
    }

    #[test]
    fn handshake_catalog_publishes_external_count_relationships() {
        // Arrange
        let variants = HANDSHAKE_SCHEMA.variants();

        // Act
        let validation = HANDSHAKE_SCHEMA_CATALOG.validate();

        // Assert
        assert_eq!(validation, Ok(()));
        assert_eq!(
            variants[0].fields()[4].external_count_field(),
            Some("required_count")
        );
        assert_eq!(
            variants[0].fields()[5].external_count_field(),
            Some("optional_count")
        );
        assert_eq!(
            variants[1].fields()[4].external_count_field(),
            Some("enabled_count")
        );
    }

    #[test]
    fn generated_handshake_encoder_rejects_external_count_mismatch() {
        // Arrange
        let codec = HandshakeCodec::new(limits());
        let wire = PwrightHandshake::ServerHello {
            selected_version: PwrightHandshakeVersion { major: 1, minor: 0 },
            client_receive_limits: PwrightHandshakeFrameLimits {
                payload_bytes: 0,
                attachments: 0,
                attachment_bytes: 0,
                total_bytes: 0,
            },
            server_receive_limits: PwrightHandshakeFrameLimits {
                payload_bytes: 0,
                attachments: 0,
                attachment_bytes: 0,
                total_bytes: 0,
            },
            enabled_count: 1,
            enabled_capabilities: Vec::new(),
        };

        // Act
        let error = encode_value(&wire, codec.value_limits())
            .expect_err("declared external count must equal the sequence length");

        // Assert
        assert!(matches!(error, BinaryValueEncodeError::SizeRange(_)));
    }

    #[test]
    fn server_hello_round_trips_in_sorted_capability_order() {
        // Arrange
        let codec = HandshakeCodec::new(limits());
        let expected = server();

        // Act
        let parts = codec.encode_parts(&expected).expect("server should encode");
        let actual = <HandshakeCodec as WireCodec<ServerHello>>::decode_frame(
            &codec,
            frame(parts.payload().to_vec()),
        )
        .expect("server should decode");

        // Assert
        assert_eq!(actual, expected);
        assert_eq!(actual.enabled_capabilities()[0].id().as_str(), "navigation");
    }

    #[test]
    fn route_attachments_and_message_direction_are_strict() {
        // Arrange
        let codec = HandshakeCodec::new(limits());
        let wrong_route = WireFrame::new(
            version(0),
            FrameRoute::Control,
            vec![1],
            Vec::new(),
            frames(),
        )
        .expect("frame should build");
        let attached = WireFrame::new(
            version(0),
            FrameRoute::Handshake,
            vec![1],
            vec![vec![0]],
            FrameLimits::new(4096, 1, 1, 4096),
        )
        .expect("frame should build");
        let server_payload =
            <HandshakeCodec as WireCodec<ServerHello>>::encode_parts(&codec, &server())
                .expect("server should encode");

        // Act
        let route_error = decode_client(&codec, wrong_route);
        let attachment_error = decode_client(&codec, attached);
        let kind_error = decode_client(&codec, frame(server_payload.payload().to_vec()));

        // Assert
        assert!(matches!(
            route_error,
            Err(HandshakeCodecError::WrongRoute(_))
        ));
        assert!(matches!(
            attachment_error,
            Err(HandshakeCodecError::AttachmentsUnsupported { actual: 1 })
        ));
        assert_eq!(
            kind_error,
            Err(HandshakeCodecError::WrongHelloKind {
                expected: HelloKind::Client,
                actual: 2,
            })
        );
    }

    #[test]
    fn truncation_and_trailing_bytes_are_rejected() {
        // Arrange
        let codec = HandshakeCodec::new(limits());
        let payload = codec
            .encode_parts(&client())
            .expect("client should encode")
            .payload()
            .to_vec();
        let truncated = payload[..payload.len() - 1].to_vec();
        let mut trailing = payload;
        trailing.push(0);

        // Act
        let truncated_error = decode_client(&codec, frame(truncated));
        let trailing_error = decode_client(&codec, frame(trailing));

        // Assert
        assert!(matches!(
            truncated_error,
            Err(HandshakeCodecError::Truncated { .. })
        ));
        assert_eq!(
            trailing_error,
            Err(HandshakeCodecError::TrailingBytes { remaining: 1 })
        );
    }

    #[test]
    fn oversized_declared_count_is_rejected_before_entries_are_read() {
        // Arrange
        let codec = HandshakeCodec::new(NegotiationLimits::new(1, 32));
        let mut payload = vec![1];
        payload.extend_from_slice(&1_u16.to_be_bytes());
        payload.extend_from_slice(&0_u16.to_be_bytes());
        payload.extend_from_slice(&1_u16.to_be_bytes());
        payload.extend_from_slice(&0_u16.to_be_bytes());
        for _ in 0..4 {
            payload.extend_from_slice(&0_u64.to_be_bytes());
        }
        payload.extend_from_slice(&2_u32.to_be_bytes());
        payload.extend_from_slice(&0_u32.to_be_bytes());

        // Act
        let error = decode_client(&codec, frame(payload));

        // Assert
        assert_eq!(
            error,
            Err(HandshakeCodecError::TooManyCapabilities {
                actual: 2,
                limit: 1,
            })
        );
    }

    #[test]
    fn combined_required_and_optional_counts_obey_the_single_negotiation_limit() {
        // Arrange
        let codec = HandshakeCodec::new(NegotiationLimits::new(2, 32));
        let mut payload = vec![super::LEGACY_CLIENT_HELLO];
        for component in [1_u16, 0, 1, 0] {
            payload.extend_from_slice(&component.to_be_bytes());
        }
        for _ in 0..4 {
            payload.extend_from_slice(&0_u64.to_be_bytes());
        }
        payload.extend_from_slice(&2_u32.to_be_bytes());
        payload.extend_from_slice(&1_u32.to_be_bytes());
        for name in ["a", "b", "c"] {
            payload.extend_from_slice(&1_u32.to_be_bytes());
            payload.extend_from_slice(name.as_bytes());
            payload.extend_from_slice(&1_u16.to_be_bytes());
            payload.extend_from_slice(&0_u16.to_be_bytes());
        }

        // Act
        let error = decode_client(&codec, frame(payload));

        // Assert
        assert_eq!(
            error,
            Err(HandshakeCodecError::TooManyCapabilities {
                actual: 3,
                limit: 2,
            })
        );
    }

    #[test]
    fn noncanonical_capability_order_is_rejected() {
        // Arrange
        let codec = HandshakeCodec::new(limits());
        let mut payload = vec![1];
        for component in [1_u16, 0, 1, 0] {
            payload.extend_from_slice(&component.to_be_bytes());
        }
        for _ in 0..4 {
            payload.extend_from_slice(&0_u64.to_be_bytes());
        }
        payload.extend_from_slice(&2_u32.to_be_bytes());
        payload.extend_from_slice(&0_u32.to_be_bytes());
        for name in ["z", "a"] {
            payload.extend_from_slice(&1_u32.to_be_bytes());
            payload.extend_from_slice(name.as_bytes());
            payload.extend_from_slice(&1_u16.to_be_bytes());
            payload.extend_from_slice(&0_u16.to_be_bytes());
        }

        // Act
        let error = decode_client(&codec, frame(payload));

        // Assert
        assert_eq!(error, Err(HandshakeCodecError::NonCanonicalCapabilityOrder));
    }

    #[test]
    fn server_hello_constructor_rejects_duplicate_capabilities() {
        // Arrange
        let duplicate = WireCapability::new(id("navigation"), version(1));

        // Act
        let result = ServerHello::new(
            version(1),
            frames(),
            frames(),
            vec![duplicate.clone(), duplicate],
            limits(),
        );

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn encoded_manifest_is_accepted_by_server_hello_constructor() {
        // Arrange
        let manifest = CapabilityManifest::new(
            vec![WireCapability::new(id("navigation"), version(2))],
            limits(),
        )
        .expect("manifest should build");
        let enabled = manifest
            .iter()
            .map(|(id, version)| WireCapability::new(id.clone(), version))
            .collect();

        // Act
        let hello = ServerHello::new(version(2), frames(), frames(), enabled, limits());

        // Assert
        assert_eq!(
            hello
                .expect("server hello should build")
                .enabled_capabilities()
                .len(),
            1
        );
    }
}
