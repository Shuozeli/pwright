#![cfg(feature = "frame-navigation")]
use pwright_cap_navigation::{self as legacy, v1_2 as cap};
use pwright_codec_binary::{BinaryValueLimits, decode_value, encode_value};
use pwright_codec_navigation::{generated_schema as old, v1_2 as wire};
use pwright_core::{
    CallId, CallMetadata, CancellationId, Command, CommandResult, FailureKind, ObjectId,
    OperationError,
};
use pwright_core::{Capability, CommandSpec, ProtocolVersion};
use pwright_transport::{FrameLimits, FrameRoute, WireFrame, WireProtocol};

fn protocol() -> WireProtocol<wire::NavigationCodec> {
    WireProtocol::new_exact_directional(
        wire::NavigationCodec::new(wire::NavigationCodecLimits::new(256, 4, 256)),
        ProtocolVersion::new(1, 2),
        frame_limits(),
        frame_limits(),
    )
}
fn frame_limits() -> FrameLimits {
    FrameLimits::new(4096, 0, 0, 4096)
}

fn command() -> Command<cap::FrameGoto> {
    Command::new(
        CallId::new(8),
        ObjectId::new(9),
        CallMetadata::new(
            Some(std::time::Duration::from_millis(123)),
            Some(CancellationId::new(17)),
        ),
        cap::GotoParams::new("https://example.test/child").with_wait_until(cap::WaitUntil::Commit),
    )
}

#[test]
fn typed_frame_command_roundtrip_preserves_route_timeout_and_cancellation() {
    // Arrange
    let command = command();
    // Act
    let frame = protocol().encode(&command).unwrap();
    let decoded: Command<cap::FrameGoto> = protocol().decode(frame).unwrap();
    // Assert
    assert_eq!(decoded.call_id(), command.call_id());
    assert_eq!(decoded.object_id(), command.object_id());
    assert_eq!(decoded.metadata(), command.metadata());
    assert_eq!(decoded.params(), command.params());
}

#[test]
fn typed_frame_result_canonicalizes_headers_without_mutating_input() {
    // Arrange
    let result = CommandResult::<cap::FrameGoto>::new(
        CallId::new(8),
        Ok(cap::FrameGotoResult {
            frame_id: 9,
            response: Some(cap::NavigationResponse {
                url: "https://example.test/missing".into(),
                status: 404,
                status_text: "Not Found".into(),
                headers: vec![
                    cap::Header {
                        name: "z".into(),
                        value: "2".into(),
                    },
                    cap::Header {
                        name: "a".into(),
                        value: "1".into(),
                    },
                ],
            }),
        }),
    );
    // Act
    let decoded: CommandResult<cap::FrameGoto> = protocol()
        .decode(protocol().encode(&result).unwrap())
        .unwrap();
    // Assert
    let decoded = decoded.outcome().as_ref().unwrap();
    assert_eq!(decoded.frame_id, 9);
    let response = decoded.response.as_ref().unwrap();
    assert_eq!(response.status, 404);
    assert_eq!(
        response
            .headers
            .iter()
            .map(|h| h.name.as_str())
            .collect::<Vec<_>>(),
        ["a", "z"]
    );
    assert_eq!(
        result
            .outcome()
            .as_ref()
            .unwrap()
            .response
            .as_ref()
            .unwrap()
            .headers[0]
            .name,
        "z"
    );
}

#[test]
fn typed_frame_result_preserves_null_and_each_error_detail() {
    for outcome in [
        Ok(cap::FrameGotoResult {
            frame_id: 9,
            response: None,
        }),
        Err(OperationError::new(FailureKind::Timeout, "deadline", None)),
        Err(OperationError::new(
            FailureKind::Backend,
            "network",
            Some(cap::NavigationErrorDetails::Network("reset".into())),
        )),
        Err(OperationError::new(
            FailureKind::Backend,
            "invalid",
            Some(cap::NavigationErrorDetails::InvalidBackendResponse(
                "missing".into(),
            )),
        )),
        Err(OperationError::new(
            FailureKind::UnsupportedCapability,
            "option",
            Some(cap::NavigationErrorDetails::UnsupportedOption(
                "wait".into(),
            )),
        )),
        Err(OperationError::new(
            FailureKind::Backend,
            "backend",
            Some(cap::NavigationErrorDetails::Backend("closed".into())),
        )),
    ] {
        // Arrange
        let result = CommandResult::<cap::FrameGoto>::new(CallId::new(8), outcome);
        // Act
        let decoded: CommandResult<cap::FrameGoto> = protocol()
            .decode(protocol().encode(&result).unwrap())
            .unwrap();
        // Assert
        assert_eq!(decoded.call_id(), result.call_id());
        assert_eq!(decoded.outcome(), result.outcome());
    }
}

#[test]
fn selected_codec_rejects_forged_call_and_target_before_exposing_commands() {
    let original = protocol().encode(&command()).unwrap();
    for (call, target) in [(99, 9), (8, 99)] {
        // Arrange
        let forged = WireFrame::new(
            ProtocolVersion::new(1, 2),
            FrameRoute::Command {
                call_id: CallId::new(call),
                target: ObjectId::new(target),
                method: cap::FrameGoto::METHOD,
            },
            original.payload().to_vec(),
            vec![],
            frame_limits(),
        )
        .unwrap();
        // Act
        let error = protocol()
            .decode::<Command<cap::FrameGoto>>(forged)
            .err()
            .unwrap();
        // Assert
        assert!(matches!(
            error,
            pwright_transport::WireProtocolError::Codec(
                wire::NavigationCodecError::RouteIdentityMismatch { .. }
            )
        ));
    }
}

#[test]
fn selected_codec_rejects_forged_result_call_and_frame_payload_on_page_method() {
    // Arrange
    let result = protocol()
        .encode(&CommandResult::<cap::FrameGoto>::new(
            CallId::new(8),
            Ok(cap::FrameGotoResult {
                frame_id: 9,
                response: None,
            }),
        ))
        .unwrap();
    let forged_result = WireFrame::new(
        ProtocolVersion::new(1, 2),
        FrameRoute::Result {
            call_id: CallId::new(99),
        },
        result.payload().to_vec(),
        vec![],
        frame_limits(),
    )
    .unwrap();
    let frame = protocol().encode(&command()).unwrap();
    let forged_command = WireFrame::new(
        ProtocolVersion::new(1, 2),
        FrameRoute::Command {
            call_id: CallId::new(8),
            target: ObjectId::new(9),
            method: cap::Goto::METHOD,
        },
        frame.payload().to_vec(),
        vec![],
        frame_limits(),
    )
    .unwrap();
    // Act
    let result_error = protocol()
        .decode::<CommandResult<cap::FrameGoto>>(forged_result)
        .err()
        .unwrap();
    let command_error = protocol()
        .decode::<Command<cap::Goto>>(forged_command)
        .err()
        .unwrap();
    // Assert
    assert!(matches!(
        result_error,
        pwright_transport::WireProtocolError::Codec(
            wire::NavigationCodecError::RouteIdentityMismatch { .. }
        )
    ));
    assert!(matches!(
        command_error,
        pwright_transport::WireProtocolError::Codec(wire::NavigationCodecError::WrongPayloadKind {
            expected: 3,
            actual: 7
        })
    ));
}

#[test]
fn selected_decoder_rejects_noncanonical_headers_and_aggregate_header_overflow() {
    for (headers, expected_order_error) in [
        (
            vec![
                wire::PwrightNavigationHeader {
                    name: "z".into(),
                    value: "".into(),
                },
                wire::PwrightNavigationHeader {
                    name: "a".into(),
                    value: "".into(),
                },
            ],
            true,
        ),
        (
            vec![wire::PwrightNavigationHeader {
                name: "a".repeat(200),
                value: "v".repeat(200),
            }],
            false,
        ),
    ] {
        // Arrange: raw schema encoding bypasses semantic canonicalization.
        let raw = wire::PwrightNavigation::FrameGotoResult {
            call_id: 8,
            outcome: wire::PwrightNavigationFrameGotoOutcome::Success {
                result: wire::PwrightNavigationFrameGotoResult {
                    frame_id: 9,
                    response: Some(wire::PwrightNavigationResponse {
                        url: "https://example.test".into(),
                        status: 200,
                        status_text: "OK".into(),
                        headers,
                    }),
                },
            },
        };
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 2),
            FrameRoute::Result {
                call_id: CallId::new(8),
            },
            encode_value(&raw, limits()).unwrap(),
            vec![],
            frame_limits(),
        )
        .unwrap();
        // Act
        let error = protocol()
            .decode::<CommandResult<cap::FrameGoto>>(frame)
            .err()
            .unwrap();
        // Assert
        if expected_order_error {
            assert!(matches!(
                error,
                pwright_transport::WireProtocolError::Codec(
                    wire::NavigationCodecError::NonCanonicalHeaderOrder
                )
            ));
        } else {
            assert!(matches!(
                error,
                pwright_transport::WireProtocolError::Codec(
                    wire::NavigationCodecError::HeaderBytesTooLarge { .. }
                )
            ));
        }
    }
}

fn limits() -> BinaryValueLimits {
    BinaryValueLimits::new(4096, 0, 64, 16, 128)
}
fn metadata() -> wire::PwrightCommonCallMetadata {
    wire::PwrightCommonCallMetadata {
        timeout: None,
        cancellation_id: Some(17),
    }
}

#[test]
fn selected_contract_has_a_distinct_frame_target_and_keeps_root_page_version() {
    // Arrange/Act
    let selected = cap::FrameGoto::TARGET_OBJECT_KINDS;
    // Assert
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].as_str(), "pwright.browser.frame");
    assert_eq!(cap::FrameGoto::METHOD.get(), 65540);
    assert_eq!(
        legacy::Goto::TARGET_OBJECT_KINDS[0].as_str(),
        "pwright.browser.page"
    );
    assert_eq!(
        legacy::NavigationCapability::DESCRIPTOR.plugin.version,
        ProtocolVersion::new(1, 1)
    );
    assert_eq!(
        cap::NavigationCapability::DESCRIPTOR.plugin.version,
        ProtocolVersion::new(1, 2)
    );
}

#[test]
fn old_page_payloads_are_byte_identical_in_the_selected_minor_schema() {
    let values = [
        old::PwrightNavigation::BeginCommand {
            call_id: 1,
            object_id: 7,
            metadata: metadata(),
            params: old::PwrightNavigationBeginNavigationParams {
                url: "about:blank".into(),
                referrer: Some("https://referrer.test/".into()),
            },
        },
        old::PwrightNavigation::GotoCommand {
            call_id: 2,
            object_id: 7,
            metadata: metadata(),
            params: old::PwrightNavigationGotoParams {
                url: "about:blank".into(),
                referrer: None,
                wait_until: old::PwrightNavigationWaitUntil::Load,
            },
        },
        old::PwrightNavigation::ReloadCommand {
            call_id: 3,
            object_id: 7,
            metadata: metadata(),
            params: old::PwrightNavigationReloadParams {
                wait_until: old::PwrightNavigationWaitUntil::NetworkIdle,
            },
        },
    ];
    for value in values {
        // Arrange
        let bytes = encode_value(&value, limits()).unwrap();
        // Act
        let selected: wire::PwrightNavigation = decode_value(&bytes, limits()).unwrap();
        let encoded = encode_value(&selected, limits()).unwrap();
        // Assert
        assert_eq!(encoded, bytes);
        assert_eq!(
            decode_value::<old::PwrightNavigation>(&encoded, limits()).unwrap(),
            value
        );
    }
}

#[test]
fn frame_command_roundtrip_preserves_identity_and_old_decoder_rejects_its_new_tag() {
    // Arrange
    let command = wire::PwrightNavigation::FrameGotoCommand {
        call_id: 8,
        object_id: 9,
        metadata: metadata(),
        params: cap::GotoParams::new("https://example.test/child")
            .with_wait_until(cap::WaitUntil::Commit)
            .into(),
    };
    // Act
    let bytes = encode_value(&command, limits()).unwrap();
    let decoded = decode_value::<wire::PwrightNavigation>(&bytes, limits()).unwrap();
    // Assert
    assert_eq!(decoded, command);
    assert!(decode_value::<old::PwrightNavigation>(&bytes, limits()).is_err());
}

#[test]
fn frame_result_preserves_portable_identity_http_errors_and_null_response() {
    for response in [
        None,
        Some(cap::NavigationResponse {
            url: "https://example.test/missing".into(),
            status: 404,
            status_text: "Not Found".into(),
            headers: vec![cap::Header {
                name: "content-type".into(),
                value: "text/html".into(),
            }],
        }),
    ] {
        // Arrange
        let result = cap::FrameGotoResult {
            frame_id: 9,
            response,
        };
        let value: wire::PwrightNavigationFrameGotoResult = result.clone().into();
        // Act
        let decoded: wire::PwrightNavigationFrameGotoResult =
            decode_value(&encode_value(&value, limits()).unwrap(), limits()).unwrap();
        // Assert
        assert_eq!(cap::FrameGotoResult::from(decoded), result);
    }
}

#[test]
fn frame_result_rejects_zero_identity_and_input_exceeding_string_budget() {
    // Arrange
    let result = wire::PwrightNavigationFrameGotoResult {
        frame_id: 0,
        response: None,
    };
    let command: wire::PwrightNavigationGotoParams = cap::GotoParams::new("x".repeat(4097)).into();
    // Act
    let bad_id = encode_value(&result, limits());
    let bad_string = encode_value(&command, limits());
    // Assert
    assert!(bad_id.is_err());
    assert!(bad_string.is_err());
}

fn assert_legacy_bytes<Old, New>(message: &Old)
where
    pwright_codec_navigation::NavigationCodec: pwright_transport::WireCodec<Old>,
    wire::NavigationCodec: pwright_transport::WireCodec<New>,
{
    let old_protocol = WireProtocol::new_exact_directional(
        pwright_codec_navigation::NavigationCodec::new(wire::NavigationCodecLimits::new(
            256, 4, 256,
        )),
        ProtocolVersion::new(1, 1),
        frame_limits(),
        frame_limits(),
    );
    let old_frame = old_protocol.encode(message).ok().unwrap();
    let selected_frame = WireFrame::new(
        ProtocolVersion::new(1, 2),
        old_frame.route(),
        old_frame.payload().to_vec(),
        vec![],
        frame_limits(),
    )
    .unwrap();
    let selected: New = protocol().decode(selected_frame).ok().unwrap();
    let encoded = protocol().encode(&selected).ok().unwrap();
    assert_eq!(encoded.route(), old_frame.route());
    assert_eq!(encoded.payload(), old_frame.payload());
}

#[test]
fn all_retained_page_typed_adapters_preserve_legacy_payload_bytes() {
    // Arrange
    let meta = CallMetadata::new(
        Some(std::time::Duration::from_millis(123)),
        Some(CancellationId::new(17)),
    );
    let begin = Command::<legacy::BeginNavigation>::new(
        CallId::new(8),
        ObjectId::new(7),
        meta,
        legacy::BeginNavigationParams {
            url: "about:blank".into(),
            referrer: None,
        },
    );
    let goto = Command::<legacy::Goto>::new(
        CallId::new(8),
        ObjectId::new(7),
        meta,
        legacy::GotoParams::new("about:blank"),
    );
    let reload = Command::<legacy::Reload>::new(
        CallId::new(8),
        ObjectId::new(7),
        meta,
        legacy::ReloadParams::default(),
    );
    let begin_result = CommandResult::<legacy::BeginNavigation>::new(
        CallId::new(8),
        Ok(legacy::BeginNavigationResult {
            frame_id: "native-frame".into(),
            loader_id: Some("loader".into()),
            same_document: false,
            is_download: false,
        }),
    );
    let result = legacy::GotoResult {
        frame_id: "native-frame".into(),
        loader_id: None,
        url: "https://example.test".into(),
        same_document: false,
        response: Some(legacy::NavigationResponse {
            url: "https://example.test".into(),
            status: 404,
            status_text: "Not Found".into(),
            headers: vec![
                legacy::Header {
                    name: "z".into(),
                    value: "v".into(),
                },
                legacy::Header {
                    name: "a".into(),
                    value: "v".into(),
                },
            ],
        }),
    };
    // Act/Assert: retained commands and both success/error envelopes use the
    // actual typed codecs, not just generated raw schema encode/decode.
    assert_legacy_bytes::<_, Command<cap::BeginNavigation>>(&begin);
    assert_legacy_bytes::<_, Command<cap::Goto>>(&goto);
    assert_legacy_bytes::<_, Command<cap::Reload>>(&reload);
    assert_legacy_bytes::<_, CommandResult<cap::BeginNavigation>>(&begin_result);
    assert_legacy_bytes::<_, CommandResult<cap::Goto>>(&CommandResult::<legacy::Goto>::new(
        CallId::new(8),
        Ok(result.clone()),
    ));
    assert_legacy_bytes::<_, CommandResult<cap::Reload>>(&CommandResult::<legacy::Reload>::new(
        CallId::new(8),
        Ok(result),
    ));
    let error = OperationError::new(
        FailureKind::Backend,
        "failed",
        Some(legacy::NavigationErrorDetails::Network("reset".into())),
    );
    assert_legacy_bytes::<_, CommandResult<cap::BeginNavigation>>(&CommandResult::<
        legacy::BeginNavigation,
    >::new(
        CallId::new(8),
        Err(error.clone()),
    ));
    assert_legacy_bytes::<_, CommandResult<cap::Goto>>(&CommandResult::<legacy::Goto>::new(
        CallId::new(8),
        Err(error.clone()),
    ));
    assert_legacy_bytes::<_, CommandResult<cap::Reload>>(&CommandResult::<legacy::Reload>::new(
        CallId::new(8),
        Err(error),
    ));
}
