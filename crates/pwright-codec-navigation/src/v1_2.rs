//! Opt-in navigation 1.2 adapters. Encoding borrows values and canonicalizes
//! headers without cloning response storage. No runtime or backend is installed.

use pwright_cap_navigation::v1_2 as cap;
use pwright_codec_binary::{
    BinaryEncode, BinaryEncodeSequence, BinaryEncodeSequenceRef, BinaryValueContext,
    BinaryValueEncodeError, BinaryWriter, decode_value, encode_value,
};
use pwright_core::{CallId, Command, CommandResult, CommandSpec, ObjectId, OperationError};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

pub use super::{NavigationCodecError, NavigationCodecLimits};
use super::{
    command_parts, command_route, ensure_no_attachments, failure_kind_from_wire,
    failure_kind_to_wire, map_generated_decode, map_generated_encode, metadata_from_wire,
    metadata_to_wire, result_parts, result_route,
};

#[doc(hidden)]
#[rustfmt::skip]
#[path = "generated_schema_1_2.rs"]
mod generated;
pub use generated::*;

/// Select this codec and the matching 1.2 server explicitly. The legacy codec
/// remains a different type, even when Cargo unifies the feature selection.
#[derive(Clone, Copy, Debug)]
pub struct NavigationCodec(super::NavigationCodec);

impl NavigationCodec {
    pub const fn new(limits: NavigationCodecLimits) -> Self {
        Self(super::NavigationCodec::new(limits))
    }

    pub const fn limits(self) -> NavigationCodecLimits {
        self.0.limits()
    }

    fn decode(
        &self,
        frame: &WireFrame,
        expected: u8,
    ) -> Result<PwrightNavigation, NavigationCodecError> {
        ensure_no_attachments(frame)?;
        decode_value(frame.payload(), self.0.value_limits())
            .map_err(|error| map_generated_decode(error, expected))
    }

    fn response<'a>(
        &self,
        response: Option<&'a cap::NavigationResponse>,
        headers: &'a CanonicalHeaders<'a>,
    ) -> Result<Option<PwrightNavigationResponseRef<'a>>, NavigationCodecError> {
        let Some(response) = response else {
            return Ok(None);
        };
        if headers.len() > self.limits().headers {
            return Err(NavigationCodecError::TooManyHeaders {
                actual: headers.len(),
                limit: self.limits().headers,
            });
        }
        self.0.validate_header_bytes(
            response
                .headers
                .iter()
                .map(|h| (h.name.as_str(), h.value.as_str())),
        )?;
        Ok(Some(PwrightNavigationResponseRef {
            url: &response.url,
            status: response.status,
            status_text: &response.status_text,
            headers: BinaryEncodeSequenceRef::new(headers),
        }))
    }

    fn validate_response(
        &self,
        response: Option<&PwrightNavigationResponse>,
    ) -> Result<(), NavigationCodecError> {
        if let Some(response) = response {
            self.0.validate_header_bytes(
                response
                    .headers
                    .iter()
                    .map(|h| (h.name.as_str(), h.value.as_str())),
            )?;
            if response
                .headers
                .windows(2)
                .any(|pair| (&pair[0].name, &pair[0].value) > (&pair[1].name, &pair[1].value))
            {
                return Err(NavigationCodecError::NonCanonicalHeaderOrder);
            }
        }
        Ok(())
    }
}

// Each invocation binds one schema command to its exact envelope and public
// parameter type. Validate duplicated identities before exposing decoded values.
macro_rules! command_codec {
    ($spec:ident, $variant:ident, $tag:literal, $params:ident, $borrow:expr) => {
        impl WireCodec<Command<cap::$spec>> for NavigationCodec {
            type Error = NavigationCodecError;
            fn encode_parts(
                &self,
                command: &Command<cap::$spec>,
            ) -> Result<EncodedParts, Self::Error> {
                let $params = command.params();
                let wire = PwrightNavigationRef::$variant {
                    call_id: command.call_id().get(),
                    object_id: command.object_id().get(),
                    metadata: metadata_to_wire(command.metadata()),
                    params: $borrow,
                };
                let payload =
                    encode_value(&wire, self.0.value_limits()).map_err(map_generated_encode)?;
                Ok(command_parts(command, payload))
            }
            fn decode_frame(&self, frame: WireFrame) -> Result<Command<cap::$spec>, Self::Error> {
                command_route::<cap::$spec>(&frame)?;
                match self.decode(&frame, $tag)? {
                    PwrightNavigation::$variant {
                        call_id,
                        object_id,
                        metadata,
                        params,
                    } => {
                        verify_identity(
                            frame.route(),
                            FrameRoute::Command {
                                call_id: CallId::new(call_id),
                                target: ObjectId::new(object_id),
                                method: cap::$spec::METHOD,
                            },
                        )?;
                        Ok(Command::new(
                            CallId::new(call_id),
                            ObjectId::new(object_id),
                            metadata_from_wire(metadata),
                            params.into(),
                        ))
                    }
                    other => Err(wrong_kind($tag, &other)),
                }
            }
        }
    };
}

command_codec!(
    BeginNavigation,
    BeginCommand,
    1,
    params,
    PwrightNavigationBeginNavigationParamsRef {
        url: &params.url,
        referrer: params.referrer.as_deref(),
    }
);
command_codec!(
    Goto,
    GotoCommand,
    3,
    params,
    PwrightNavigationGotoParamsRef {
        url: &params.url,
        referrer: params.referrer.as_deref(),
        wait_until: params.wait_until.into(),
    }
);
command_codec!(
    Reload,
    ReloadCommand,
    5,
    params,
    PwrightNavigationReloadParams {
        wait_until: params.wait_until.into(),
    }
);
command_codec!(
    FrameGoto,
    FrameGotoCommand,
    7,
    params,
    PwrightNavigationGotoParamsRef {
        url: &params.url,
        referrer: params.referrer.as_deref(),
        wait_until: params.wait_until.into(),
    }
);

macro_rules! result_codec {
    ($spec:ident, $variant:ident, $tag:literal, $outcome:ident, $outcome_ref:ident,
     $codec:ident, $value:ident, $headers:ident, $response:expr, $borrow:expr, $decoded:ident, $validate:expr) => {
        impl WireCodec<CommandResult<cap::$spec>> for NavigationCodec {
            type Error = NavigationCodecError;
            fn encode_parts(
                &self,
                result: &CommandResult<cap::$spec>,
            ) -> Result<EncodedParts, Self::Error> {
                let $codec = self;
                let headers = CanonicalHeaders(
                    result
                        .outcome()
                        .as_ref()
                        .ok()
                        .and_then(|$value| $response)
                        .map_or(&[][..], |r: &cap::NavigationResponse| r.headers.as_slice()),
                );
                let $headers = &headers;
                let outcome = match result.outcome() {
                    Ok($value) => $outcome_ref::Success { result: $borrow },
                    Err(error) => $outcome_ref::Error {
                        error: error_ref(error),
                    },
                };
                let wire = PwrightNavigationRef::$variant {
                    call_id: result.call_id().get(),
                    outcome,
                };
                let payload =
                    encode_value(&wire, $codec.0.value_limits()).map_err(map_generated_encode)?;
                Ok(result_parts(result.call_id(), payload))
            }
            fn decode_frame(
                &self,
                frame: WireFrame,
            ) -> Result<CommandResult<cap::$spec>, Self::Error> {
                let $codec = self;
                result_route(&frame)?;
                match $codec.decode(&frame, $tag)? {
                    PwrightNavigation::$variant { call_id, outcome } => {
                        verify_identity(
                            frame.route(),
                            FrameRoute::Result {
                                call_id: CallId::new(call_id),
                            },
                        )?;
                        let outcome = match outcome {
                            $outcome::Success { result: $decoded } => {
                                $validate;
                                Ok($decoded.into())
                            }
                            $outcome::Error { error } => Err(OperationError::new(
                                failure_kind_from_wire(error.failure_kind),
                                error.message,
                                error.details.map(Into::into),
                            )),
                        };
                        Ok(CommandResult::new(CallId::new(call_id), outcome))
                    }
                    other => Err(wrong_kind($tag, &other)),
                }
            }
        }
    };
}

result_codec!(
    BeginNavigation,
    BeginResult,
    2,
    PwrightNavigationBeginOutcome,
    PwrightNavigationBeginOutcomeRef,
    codec,
    value,
    _headers,
    {
        let _ = value;
        None
    },
    PwrightNavigationBeginNavigationResultRef {
        frame_id: &value.frame_id,
        loader_id: value.loader_id.as_deref(),
        same_document: value.same_document,
        is_download: value.is_download,
    },
    decoded,
    codec.validate_response(None)?
);
result_codec!(
    Goto,
    GotoResult,
    4,
    PwrightNavigationGotoOutcome,
    PwrightNavigationGotoOutcomeRef,
    codec,
    value,
    headers,
    value.response.as_ref(),
    PwrightNavigationGotoResultRef {
        frame_id: &value.frame_id,
        loader_id: value.loader_id.as_deref(),
        url: &value.url,
        same_document: value.same_document,
        response: codec.response(value.response.as_ref(), headers)?,
    },
    decoded,
    codec.validate_response(decoded.response.as_ref())?
);
result_codec!(
    Reload,
    ReloadResult,
    6,
    PwrightNavigationGotoOutcome,
    PwrightNavigationGotoOutcomeRef,
    codec,
    value,
    headers,
    value.response.as_ref(),
    PwrightNavigationGotoResultRef {
        frame_id: &value.frame_id,
        loader_id: value.loader_id.as_deref(),
        url: &value.url,
        same_document: value.same_document,
        response: codec.response(value.response.as_ref(), headers)?,
    },
    decoded,
    codec.validate_response(decoded.response.as_ref())?
);
result_codec!(
    FrameGoto,
    FrameGotoResult,
    8,
    PwrightNavigationFrameGotoOutcome,
    PwrightNavigationFrameGotoOutcomeRef,
    codec,
    value,
    headers,
    value.response.as_ref(),
    PwrightNavigationFrameGotoResultRef {
        frame_id: value.frame_id,
        response: codec.response(value.response.as_ref(), headers)?,
    },
    decoded,
    codec.validate_response(decoded.response.as_ref())?
);

fn error_ref(value: &OperationError<cap::NavigationErrorDetails>) -> PwrightNavigationErrorRef<'_> {
    PwrightNavigationErrorRef {
        failure_kind: failure_kind_to_wire(value.kind()),
        message: value.message(),
        details: value.details().map(|details| match details {
            cap::NavigationErrorDetails::Network(message) => {
                PwrightNavigationErrorDetailsRef::Network { message }
            }
            cap::NavigationErrorDetails::InvalidBackendResponse(message) => {
                PwrightNavigationErrorDetailsRef::InvalidBackendResponse { message }
            }
            cap::NavigationErrorDetails::UnsupportedOption(message) => {
                PwrightNavigationErrorDetailsRef::UnsupportedOption { message }
            }
            cap::NavigationErrorDetails::Backend(message) => {
                PwrightNavigationErrorDetailsRef::Backend { message }
            }
        }),
    }
}

fn wrong_kind(expected: u8, value: &PwrightNavigation) -> NavigationCodecError {
    let actual = match value {
        PwrightNavigation::BeginCommand { .. } => 1,
        PwrightNavigation::BeginResult { .. } => 2,
        PwrightNavigation::GotoCommand { .. } => 3,
        PwrightNavigation::GotoResult { .. } => 4,
        PwrightNavigation::ReloadCommand { .. } => 5,
        PwrightNavigation::ReloadResult { .. } => 6,
        PwrightNavigation::FrameGotoCommand { .. } => 7,
        PwrightNavigation::FrameGotoResult { .. } => 8,
    };
    NavigationCodecError::WrongPayloadKind { expected, actual }
}

fn verify_identity(outer: FrameRoute, inner: FrameRoute) -> Result<(), NavigationCodecError> {
    if outer == inner {
        Ok(())
    } else {
        Err(NavigationCodecError::RouteIdentityMismatch { outer, inner })
    }
}

// Stable (name, value, original index) ordering, with no allocated index/copy.
struct CanonicalHeaders<'a>(&'a [cap::Header]);
impl BinaryEncodeSequence for CanonicalHeaders<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn encode_items(
        &self,
        writer: &mut BinaryWriter,
        context: &mut BinaryValueContext,
    ) -> Result<(), BinaryValueEncodeError> {
        let key = |i: usize| (&self.0[i].name, &self.0[i].value, i);
        let mut previous = None;
        for _ in 0..self.len() {
            let next = (0..self.len())
                .filter(|i| previous.is_none_or(|p| key(*i) > key(p)))
                .min_by_key(|i| key(*i));
            if let Some(i) = next {
                PwrightNavigationHeaderRef {
                    name: &self.0[i].name,
                    value: &self.0[i].value,
                }
                .encode_binary(writer, context)?;
                previous = Some(i);
            }
        }
        Ok(())
    }
}
