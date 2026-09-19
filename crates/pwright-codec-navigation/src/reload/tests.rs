use super::*;
use pwright_cap_navigation::{ReloadParams, WaitUntil};
use pwright_core::ProtocolVersion;
use pwright_transport::{FrameLimits, WireProtocol};

fn protocol() -> WireProtocol<NavigationCodec> {
    WireProtocol::new_exact_directional(
        NavigationCodec::new(NavigationCodecLimits::new(256, 4, 256)),
        ProtocolVersion::new(1, 1),
        FrameLimits::new(4096, 0, 0, 4096),
        FrameLimits::new(4096, 0, 0, 4096),
    )
}

#[test]
fn reload_roundtrip_keeps_identity_deadline_and_wait_policy() {
    // Arrange
    let command = Command::<Reload>::new(
        CallId::new(4),
        ObjectId::new(7),
        CallMetadata::new(
            Some(Duration::from_millis(123)),
            Some(CancellationId::new(8)),
        ),
        ReloadParams {
            wait_until: WaitUntil::DomContentLoaded,
        },
    );
    // Act
    let encoded = protocol().encode(&command).unwrap();
    let result: Command<Reload> = protocol().decode(encoded).unwrap();
    // Assert
    assert_eq!(result.call_id(), command.call_id());
    assert_eq!(result.object_id(), command.object_id());
    assert_eq!(result.metadata(), command.metadata());
    assert_eq!(result.params(), command.params());
    assert_eq!(Reload::METHOD.get(), 65539);
}

#[test]
fn reload_success_and_typed_error_roundtrip() {
    for outcome in [
        Ok(GotoResult {
            frame_id: "f".into(),
            loader_id: Some("l".into()),
            url: "about:blank".into(),
            same_document: false,
            response: None,
        }),
        Err(OperationError::new(FailureKind::Timeout, "deadline", None)),
    ] {
        // Arrange
        let result = CommandResult::<Reload>::new(CallId::new(4), outcome);
        // Act
        let decoded: CommandResult<Reload> = protocol()
            .decode(protocol().encode(&result).unwrap())
            .unwrap();
        // Assert
        assert_eq!(decoded.call_id(), result.call_id());
        assert_eq!(decoded.outcome(), result.outcome());
    }
}

#[test]
fn old_goto_decoder_rejects_reload_even_with_a_forged_goto_route() {
    // Arrange
    let command = Command::<Reload>::new(
        CallId::new(4),
        ObjectId::new(7),
        CallMetadata::default(),
        ReloadParams::default(),
    );
    let frame = protocol().encode(&command).unwrap();
    let frame = WireFrame::new(
        ProtocolVersion::new(1, 0),
        FrameRoute::Command {
            call_id: CallId::new(4),
            target: ObjectId::new(7),
            method: Goto::METHOD,
        },
        frame.payload().to_vec(),
        vec![],
        FrameLimits::new(4096, 0, 0, 4096),
    )
    .unwrap();
    // Act
    let codec = NavigationCodec::new(NavigationCodecLimits::new(256, 4, 256));
    let error = <NavigationCodec as WireCodec<Command<Goto>>>::decode_frame(&codec, frame)
        .err()
        .unwrap();
    // Assert
    assert!(matches!(
        error,
        NavigationCodecError::WrongPayloadKind {
            expected: 3,
            actual: 5
        }
    ));
}
