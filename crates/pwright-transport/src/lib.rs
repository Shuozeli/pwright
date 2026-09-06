//! Codec- and executor-neutral wire contracts for remote Pwright connections.
//!
//! This crate owns encoded frame limits, version validation, and client-side
//! response correlation. It provides no concrete codec, socket, or runtime.

mod codec;
mod correlation;
mod frame;
mod negotiation;

pub use codec::{
    EncodedParts, FrameTransport, ProductionFrameTransport, ProductionTransportResources,
    RoutedMessage, ShutdownFrameTransport, VersionPolicy, VersionPolicyError, WireCodec,
    WireProtocol, WireProtocolError,
};
pub use correlation::{CallCorrelator, CorrelatedCall, CorrelationError};
pub use frame::{
    AssembledBytes, BinaryAttachment, BoundedBytesAssembler, FrameError, FrameLimits, FrameRoute,
    WireFrame, WireFrameReservation,
};
pub use negotiation::{
    CapabilityIdError, CapabilityManifest, CapabilityRequest, CapabilityRequirement,
    CapabilitySetError, CapabilitySupport, ClientHello, ClientHelloValidationError,
    NegotiationError, NegotiationLimits, ServerHandshakeConfig, ServerHello, WireCapability,
    WireCapabilityId, negotiate, validate_server_hello,
};
