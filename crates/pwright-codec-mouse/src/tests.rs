use super::*;
use pwright_core::ProtocolVersion;
use pwright_transport::{FrameLimits, WireProtocol};
fn number(value: f64) -> MouseNumber {
    MouseNumber::try_from(value).unwrap()
}

fn codec() -> MouseCodec {
    MouseCodec::new(MouseCodecLimits::new(4096))
}
fn limits() -> FrameLimits {
    FrameLimits::new(16384, 1, 8, 16400)
}
fn protocol() -> WireProtocol<MouseCodec> {
    WireProtocol::new_exact_directional(codec(), ProtocolVersion::new(1, 0), limits(), limits())
}
fn metadata() -> CallMetadata {
    CallMetadata::new(Some(Duration::new(3, 42)), Some(CancellationId::new(19)))
}
fn move_frame() -> WireFrame {
    protocol()
        .encode(&Command::<MouseMove>::new(
            CallId::new(7),
            ObjectId::new(11),
            metadata(),
            MoveParams {
                x: number(12.5),
                y: number(-24.25),
                steps: MouseMoveSteps::try_from(-2.0).unwrap(),
            },
        ))
        .unwrap()
}
macro_rules! roundtrip {
    ($name:ident, $command:ty, $params:expr) => {
        #[test]
        fn $name() {
            // Arrange
            let protocol = protocol();
            let command =
                Command::<$command>::new(CallId::new(7), ObjectId::new(11), metadata(), $params);
            let result = CommandResult::<$command>::new(CallId::new(7), Ok(()));
            // Act
            let decoded = protocol
                .decode::<Command<$command>>(protocol.encode(&command).unwrap())
                .unwrap();
            let reply = protocol
                .decode::<CommandResult<$command>>(protocol.encode(&result).unwrap())
                .unwrap();
            // Assert
            assert_eq!(decoded.params(), command.params());
            assert_eq!(decoded.metadata(), metadata());
            assert_eq!(decoded.object_id(), ObjectId::new(11));
            assert_eq!(decoded.call_id(), CallId::new(7));
            assert_eq!(reply.call_id(), CallId::new(7));
            assert_eq!(reply.into_outcome(), Ok(()));
        }
    };
}
roundtrip!(
    move_roundtrips_parameters_and_control_metadata,
    MouseMove,
    MoveParams {
        x: number(12.5),
        y: number(-24.25),
        steps: MouseMoveSteps::try_from(-2.0).unwrap()
    }
);
roundtrip!(
    down_roundtrips_parameters_and_control_metadata,
    MouseDown,
    DownParams {
        button: MouseButton::Right,
        click_count: -2
    }
);
roundtrip!(
    up_roundtrips_parameters_and_control_metadata,
    MouseUp,
    UpParams {
        button: MouseButton::Middle,
        click_count: 2
    }
);
roundtrip!(
    click_roundtrips_parameters_and_control_metadata,
    MouseClick,
    ClickParams {
        x: number(5.5),
        y: number(8.25),
        button: MouseButton::Left,
        click_count: MouseClickCount::try_from(3.0).unwrap(),
        delay: Some(std::time::Duration::new(1, 500_000))
    }
);
roundtrip!(
    dblclick_roundtrips_parameters_and_control_metadata,
    MouseDblclick,
    DblclickParams {
        x: number(5.5),
        y: number(8.25),
        button: MouseButton::Right,
        delay: None
    }
);
roundtrip!(
    wheel_roundtrips_parameters_and_control_metadata,
    MouseWheel,
    WheelParams {
        delta_x: number(-1.5),
        delta_y: number(90.25)
    }
);

#[test]
fn failure_kinds_and_typed_details_roundtrip_without_reclassification() {
    // Arrange
    let kinds = [
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
    for kind in kinds {
        for details in [
            None,
            Some(MouseErrorDetails::InvalidArgument("number".into())),
            Some(MouseErrorDetails::Resource("bound".into())),
            Some(MouseErrorDetails::Backend("native".into())),
        ] {
            let result = CommandResult::<MouseClick>::new(
                CallId::new(7),
                Err(OperationError::new(kind, "failed", details)),
            );
            // Act
            let decoded = protocol()
                .decode::<CommandResult<MouseClick>>(protocol().encode(&result).unwrap())
                .unwrap();
            // Assert
            assert_eq!(decoded.outcome(), result.outcome());
        }
    }
}
#[test]
fn outer_call_and_page_identity_are_verified_independently() {
    // Arrange
    for (call, target, field) in [(8, 11, "call ID"), (7, 12, "Page ID")] {
        let frame = move_frame();
        let forged = WireFrame::new(
            frame.version(),
            FrameRoute::Command {
                call_id: CallId::new(call),
                target: ObjectId::new(target),
                method: MouseMove::METHOD,
            },
            frame.payload().to_vec(),
            Vec::new(),
            limits(),
        )
        .unwrap();
        // Act
        let result = <MouseCodec as WireCodec<Command<MouseMove>>>::decode_frame(&codec(), forged);
        // Assert
        assert!(
            matches!(result,Err(MouseCodecError::IdentityMismatch {field: actual,..}) if actual==field)
        );
    }
}
#[test]
fn method_and_payload_identity_are_verified_independently() {
    // Arrange
    let frame = move_frame();
    let forged = WireFrame::new(
        frame.version(),
        FrameRoute::Command {
            call_id: CallId::new(7),
            target: ObjectId::new(11),
            method: MouseDown::METHOD,
        },
        frame.payload().to_vec(),
        Vec::new(),
        limits(),
    )
    .unwrap();
    // Act
    let method = <MouseCodec as WireCodec<Command<MouseDown>>>::decode_frame(&codec(), frame);
    let payload = <MouseCodec as WireCodec<Command<MouseDown>>>::decode_frame(&codec(), forged);
    // Assert
    assert!(matches!(method, Err(MouseCodecError::WrongMethod { .. })));
    assert!(matches!(
        payload,
        Err(MouseCodecError::WrongPayload { actual: 1, .. })
    ));
}
#[test]
fn result_checks_inner_identity_and_operation_even_without_header_method() {
    // Arrange
    let frame = protocol()
        .encode(&CommandResult::<MouseMove>::new(CallId::new(7), Ok(())))
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
    let identity =
        <MouseCodec as WireCodec<CommandResult<MouseMove>>>::decode_frame(&codec(), forged);
    let variant =
        <MouseCodec as WireCodec<CommandResult<MouseDown>>>::decode_frame(&codec(), frame);
    // Assert
    assert!(matches!(
        identity,
        Err(MouseCodecError::IdentityMismatch { .. })
    ));
    assert!(matches!(
        variant,
        Err(MouseCodecError::WrongPayload { actual: 2, .. })
    ));
}
#[test]
fn attachments_reject_before_payload_decoding() {
    // Arrange
    let frame = move_frame();
    let forged = WireFrame::new(
        frame.version(),
        frame.route(),
        vec![255],
        vec![vec![1]],
        limits(),
    )
    .unwrap();
    // Act
    let result = <MouseCodec as WireCodec<Command<MouseMove>>>::decode_frame(&codec(), forged);
    // Assert
    assert!(matches!(
        result,
        Err(MouseCodecError::AttachmentsUnsupported(1))
    ));
}
fn raw_move(x: f64, steps: f64) -> WireFrame {
    let wire = PwrightMouse::MouseMoveCommand {
        call_id: 7,
        object_id: 11,
        metadata: metadata_to_wire(metadata()),
        params: PwrightMouseMoveParams { x, y: 1.0, steps },
    };
    let payload = encode_value(&wire, codec().value_limits()).unwrap();
    WireFrame::new(
        ProtocolVersion::new(1, 0),
        FrameRoute::Command {
            call_id: CallId::new(7),
            target: ObjectId::new(11),
            method: MouseMove::METHOD,
        },
        payload,
        Vec::new(),
        limits(),
    )
    .unwrap()
}
#[test]
fn malicious_nonfinite_wire_coordinates_cannot_construct_typed_parameters() {
    // Arrange
    let frames = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY].map(|x| raw_move(x, 1.0));
    // Act
    let decoded = frames
        .map(|frame| <MouseCodec as WireCodec<Command<MouseMove>>>::decode_frame(&codec(), frame));
    // Assert
    for result in decoded {
        assert!(matches!(
            result,
            Err(MouseCodecError::Argument(MouseArgumentError::NonFinite))
        ));
    }
}
#[test]
fn wire_steps_reject_fractional_nonfinite_and_excess_work() {
    // Arrange
    let frames = [1.5, f64::INFINITY, 10001.0].map(|steps| raw_move(1.0, steps));
    // Act
    let decoded = frames
        .map(|frame| <MouseCodec as WireCodec<Command<MouseMove>>>::decode_frame(&codec(), frame));
    // Assert
    for result in decoded {
        assert!(matches!(
            result,
            Err(MouseCodecError::Argument(MouseArgumentError::Steps(_)))
        ));
    }
}
#[test]
fn schema_rejects_oversized_click_batch_before_typed_dispatch() {
    // Arrange
    let wire = PwrightMouse::MouseClickCommand {
        call_id: 7,
        object_id: 11,
        metadata: metadata_to_wire(metadata()),
        params: PwrightMouseClickParams {
            x: 1.0,
            y: 2.0,
            button: PwrightMouseButton::Left,
            click_count: 33,
            delay: None,
        },
    };
    // Act
    let encoded = encode_value(&wire, codec().value_limits());
    // Assert
    assert!(encoded.is_err());
}

#[test]
fn malicious_wire_count_above_work_limit_rejects_on_decode() {
    // Arrange
    let wire = PwrightMouse::MouseClickCommand {
        call_id: 7,
        object_id: 11,
        metadata: metadata_to_wire(metadata()),
        params: PwrightMouseClickParams {
            x: 1.0,
            y: 2.0,
            button: PwrightMouseButton::Left,
            click_count: 32,
            delay: None,
        },
    };
    let mut payload = encode_value(&wire, codec().value_limits()).unwrap();
    // In this schema the last two bytes are the u8 count and absent delay tag.
    let offset = payload.len() - 2;
    assert_eq!(&payload[offset..], &[32, 0]);
    payload[offset] = 33;
    let frame = WireFrame::new(
        ProtocolVersion::new(1, 0),
        FrameRoute::Command {
            call_id: CallId::new(7),
            target: ObjectId::new(11),
            method: MouseClick::METHOD,
        },
        payload,
        Vec::new(),
        limits(),
    )
    .unwrap();
    // Act
    let decoded = <MouseCodec as WireCodec<Command<MouseClick>>>::decode_frame(&codec(), frame);
    // Assert
    assert!(matches!(decoded, Err(MouseCodecError::Decode(_))));
}

#[test]
fn schema_error_message_bound_applies_above_configured_budget_floor() {
    // Arrange
    let broad = MouseCodec::new(MouseCodecLimits::new(8192));
    let reply = CommandResult::<MouseClick>::new(
        CallId::new(7),
        Err(OperationError::new(
            FailureKind::Backend,
            "x".repeat(4097),
            None,
        )),
    );
    // Act
    let encoded =
        <MouseCodec as WireCodec<CommandResult<MouseClick>>>::encode_parts(&broad, &reply);
    // Assert
    assert!(matches!(encoded, Err(MouseCodecError::Encode(_))));
}
#[test]
fn configured_error_string_budget_can_be_stricter_than_schema() {
    // Arrange
    let strict = MouseCodec::new(MouseCodecLimits::new(3));
    let reply = CommandResult::<MouseClick>::new(
        CallId::new(7),
        Err(OperationError::new(
            FailureKind::Backend,
            "four",
            Some(MouseErrorDetails::Backend("tail".into())),
        )),
    );
    let frame = protocol().encode(&reply).unwrap();
    // Act
    let encoded =
        <MouseCodec as WireCodec<CommandResult<MouseClick>>>::encode_parts(&strict, &reply);
    let decoded =
        <MouseCodec as WireCodec<CommandResult<MouseClick>>>::decode_frame(&strict, frame);
    // Assert
    assert!(matches!(encoded, Err(MouseCodecError::Encode(_))));
    assert!(matches!(decoded, Err(MouseCodecError::Decode(_))));
}
#[test]
fn catalog_has_six_distinct_commands_and_one_initial_version() {
    // Arrange / Act
    let valid = MOUSE_SCHEMA_CATALOG.validate();
    // Assert
    assert!(valid.is_ok());
    assert_eq!(MOUSE_COMMAND_SCHEMAS.len(), 6);
    assert_eq!(MOUSE_SCHEMA.version(), ProtocolVersion::new(1, 0));
}
