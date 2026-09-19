use super::*;
use pwright_core::ProtocolVersion;
use pwright_transport::{FrameLimits, WireProtocol};

fn protocol() -> WireProtocol<KeyboardCodec> {
    WireProtocol::new_exact_directional(
        KeyboardCodec::new(KeyboardCodecLimits::new(KEYBOARD_TEXT_MAX_BYTES)),
        ProtocolVersion::new(1, 0),
        limits(),
        limits(),
    )
}
fn limits() -> FrameLimits {
    FrameLimits::new(
        KEYBOARD_TEXT_MAX_BYTES + 1024,
        1,
        8,
        KEYBOARD_TEXT_MAX_BYTES + 2048,
    )
}
fn metadata() -> CallMetadata {
    CallMetadata::new(Some(Duration::new(3, 42)), Some(CancellationId::new(19)))
}
fn key() -> KeyboardKey {
    KeyboardKey::copy_from_str("Shift+").unwrap()
}
fn text() -> KeyboardText {
    KeyboardText::copy_from_str("a中😀").unwrap()
}

macro_rules! roundtrip {
    ($name:ident, $command:ty, $params:expr) => {
        #[test]
        fn $name() {
            // Arrange
            let protocol = protocol();
            let command =
                Command::<$command>::new(CallId::new(7), ObjectId::new(11), metadata(), $params);
            // Act
            let frame = protocol.encode(&command).unwrap();
            let decoded = protocol.decode::<Command<$command>>(frame).unwrap();
            let result = CommandResult::<$command>::new(command.call_id(), Ok(()));
            let result = protocol
                .decode::<CommandResult<$command>>(protocol.encode(&result).unwrap())
                .unwrap();
            // Assert
            assert_eq!(decoded.call_id(), command.call_id());
            assert_eq!(decoded.object_id(), command.object_id());
            assert_eq!(decoded.metadata(), metadata());
            assert_eq!(decoded.params(), command.params());
            assert_eq!(result.call_id(), command.call_id());
            assert_eq!(result.into_outcome(), Ok(()));
        }
    };
}
roundtrip!(
    down_preserves_raw_key_and_metadata,
    KeyboardDown,
    DownParams { key: key() }
);
roundtrip!(
    up_preserves_raw_key_and_metadata,
    KeyboardUp,
    UpParams { key: key() }
);
roundtrip!(
    press_preserves_raw_key_delay_and_metadata,
    KeyboardPress,
    PressParams {
        key: key(),
        delay: Some(Duration::new(2, 43))
    }
);
roundtrip!(
    insert_preserves_utf8_and_metadata,
    KeyboardInsertText,
    InsertTextParams { text: text() }
);
roundtrip!(
    type_preserves_utf8_delay_and_metadata,
    KeyboardType,
    TypeParams {
        text: text(),
        delay: Some(Duration::new(0, 500))
    }
);

#[test]
fn all_failure_kinds_and_details_roundtrip_without_reclassification() {
    // Arrange
    let protocol = protocol();
    let cases = [
        FailureKind::Timeout,
        FailureKind::Cancelled,
        FailureKind::TargetClosed,
        FailureKind::BrowserCrashed,
        FailureKind::UnsupportedCapability,
        FailureKind::ResourceExhausted,
        FailureKind::TransportClosed,
        FailureKind::ProtocolViolation,
        FailureKind::Backend,
        FailureKind::Cleanup,
    ];
    for kind in cases {
        for details in [
            None,
            Some(KeyboardErrorDetails::InvalidKey("unknown".into())),
            Some(KeyboardErrorDetails::Resource("bound".into())),
            Some(KeyboardErrorDetails::Backend("native".into())),
        ] {
            let result = CommandResult::<KeyboardPress>::new(
                CallId::new(1),
                Err(OperationError::new(kind, "operation failed", details)),
            );
            // Act
            let decoded = protocol
                .decode::<CommandResult<KeyboardPress>>(protocol.encode(&result).unwrap())
                .unwrap();
            // Assert
            assert_eq!(decoded.outcome(), result.outcome());
        }
    }
}

fn down_frame() -> WireFrame {
    protocol()
        .encode(&Command::<KeyboardDown>::new(
            CallId::new(7),
            ObjectId::new(11),
            metadata(),
            DownParams { key: key() },
        ))
        .unwrap()
}

#[test]
fn mismatched_outer_call_and_page_id_are_rejected_independently() {
    // Arrange
    for (call_id, target, field) in [(8, 11, "call ID"), (7, 12, "Page ID")] {
        let frame = down_frame();
        let forged = WireFrame::new(
            frame.version(),
            FrameRoute::Command {
                call_id: CallId::new(call_id),
                target: ObjectId::new(target),
                method: KeyboardDown::METHOD,
            },
            frame.payload().to_vec(),
            Vec::new(),
            limits(),
        )
        .unwrap();
        // Act
        let result = <KeyboardCodec as WireCodec<Command<KeyboardDown>>>::decode_frame(
            &KeyboardCodec::new(KeyboardCodecLimits::new(256)),
            forged,
        );
        // Assert
        assert!(
            matches!(result, Err(KeyboardCodecError::IdentityMismatch { field: actual, .. }) if actual == field)
        );
    }
}

#[test]
fn wrong_method_and_forged_payload_are_rejected_independently() {
    // Arrange
    let codec = KeyboardCodec::new(KeyboardCodecLimits::new(256));
    let frame = down_frame();
    let forged = WireFrame::new(
        frame.version(),
        FrameRoute::Command {
            call_id: CallId::new(7),
            target: ObjectId::new(11),
            method: KeyboardUp::METHOD,
        },
        frame.payload().to_vec(),
        Vec::new(),
        limits(),
    )
    .unwrap();
    // Act
    let wrong_method =
        <KeyboardCodec as WireCodec<Command<KeyboardUp>>>::decode_frame(&codec, frame);
    let wrong_payload =
        <KeyboardCodec as WireCodec<Command<KeyboardUp>>>::decode_frame(&codec, forged);
    // Assert
    assert!(matches!(
        wrong_method,
        Err(KeyboardCodecError::WrongMethod { .. })
    ));
    assert!(matches!(
        wrong_payload,
        Err(KeyboardCodecError::WrongPayload { actual: 1, .. })
    ));
}

#[test]
fn result_identity_and_variant_are_checked_without_a_method_in_the_header() {
    // Arrange
    let codec = KeyboardCodec::new(KeyboardCodecLimits::new(256));
    let frame = protocol()
        .encode(&CommandResult::<KeyboardDown>::new(CallId::new(7), Ok(())))
        .unwrap();
    let forged = WireFrame::new(
        frame.version(),
        FrameRoute::Result {
            call_id: CallId::new(8),
        },
        frame.payload().to_vec(),
        Vec::new(),
        limits(),
    )
    .unwrap();
    // Act
    let wrong_id =
        <KeyboardCodec as WireCodec<CommandResult<KeyboardDown>>>::decode_frame(&codec, forged);
    let wrong_variant =
        <KeyboardCodec as WireCodec<CommandResult<KeyboardUp>>>::decode_frame(&codec, frame);
    // Assert
    assert!(matches!(
        wrong_id,
        Err(KeyboardCodecError::IdentityMismatch { .. })
    ));
    assert!(matches!(
        wrong_variant,
        Err(KeyboardCodecError::WrongPayload { actual: 2, .. })
    ));
}

#[test]
fn attachments_are_rejected_before_decoding_the_payload() {
    // Arrange
    let frame = down_frame();
    let forged = WireFrame::new(
        frame.version(),
        frame.route(),
        vec![255],
        vec![vec![1]],
        limits(),
    )
    .unwrap();
    // Act
    let result = <KeyboardCodec as WireCodec<Command<KeyboardDown>>>::decode_frame(
        &KeyboardCodec::new(KeyboardCodecLimits::new(256)),
        forged,
    );
    // Assert
    assert!(matches!(
        result,
        Err(KeyboardCodecError::AttachmentsUnsupported(1))
    ));
}

#[test]
fn schema_key_limit_applies_even_if_codec_budget_is_larger() {
    // Arrange: insert-text and down share the command prefix and one string.
    let frame = protocol()
        .encode(&Command::<KeyboardInsertText>::new(
            CallId::new(7),
            ObjectId::new(11),
            metadata(),
            InsertTextParams {
                text: KeyboardText::copy_from_str(&"x".repeat(KEYBOARD_KEY_MAX_BYTES + 1)).unwrap(),
            },
        ))
        .unwrap();
    let mut payload = frame.payload().to_vec();
    payload[0] = 1;
    let forged = WireFrame::new(
        frame.version(),
        FrameRoute::Command {
            call_id: CallId::new(7),
            target: ObjectId::new(11),
            method: KeyboardDown::METHOD,
        },
        payload,
        Vec::new(),
        limits(),
    )
    .unwrap();
    // Act
    let result = <KeyboardCodec as WireCodec<Command<KeyboardDown>>>::decode_frame(
        &KeyboardCodec::new(KeyboardCodecLimits::new(KEYBOARD_TEXT_MAX_BYTES)),
        forged,
    );
    // Assert
    assert!(matches!(result, Err(KeyboardCodecError::Decode(_))));
}

#[test]
fn configured_string_limit_can_be_stricter_than_the_schema() {
    // Arrange
    let codec = KeyboardCodec::new(KeyboardCodecLimits::new(3));
    let command = Command::<KeyboardDown>::new(
        CallId::new(7),
        ObjectId::new(11),
        metadata(),
        DownParams { key: key() },
    );
    // Act
    let encoded =
        <KeyboardCodec as WireCodec<Command<KeyboardDown>>>::encode_parts(&codec, &command);
    let decoded =
        <KeyboardCodec as WireCodec<Command<KeyboardDown>>>::decode_frame(&codec, down_frame());
    // Assert
    assert!(matches!(encoded, Err(KeyboardCodecError::Encode(_))));
    assert!(matches!(decoded, Err(KeyboardCodecError::Decode(_))));
}

#[test]
fn catalog_preserves_distinct_command_and_result_tags() {
    // Arrange / Act
    let valid = KEYBOARD_SCHEMA_CATALOG.validate();
    // Assert
    assert!(valid.is_ok());
    assert_eq!(KEYBOARD_COMMAND_SCHEMAS.len(), 5);
    assert_eq!(KEYBOARD_SCHEMA.version(), ProtocolVersion::new(1, 0));
}
