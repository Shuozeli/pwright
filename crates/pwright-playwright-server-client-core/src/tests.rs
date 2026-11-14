use serde_json::{Value, json};

use super::{
    ClientActor, ClientCore, ClientError, CloseReason, CommandId, Ingress, MAX_FRAME_BYTES,
    MAX_OUTBOUND_BYTES, MAX_PENDING, ProtocolViolation, ReceiveFailure, RemoteError, ReplyOutcome,
    SendFailure, TextTransport, WireInbound, decode_inbound,
};

#[derive(Default)]
struct FakeTransport {
    frames: Vec<String>,
    fail_next: bool,
}

impl TextTransport for FakeTransport {
    type Error = &'static str;

    fn send_text(&mut self, frame: &str) -> Result<(), Self::Error> {
        if self.fail_next {
            self.fail_next = false;
            return Err("fake send failure");
        }
        self.frames.push(frame.to_owned());
        Ok(())
    }
}

fn empty_object() -> Value {
    json!({})
}

#[test]
fn actor_sends_a_typed_command_with_monotonic_id() {
    // Arrange
    let mut actor = ClientActor::new(FakeTransport::default());
    let params = json!({"sdkLanguage":"python"});
    let metadata = empty_object();
    actor.core.last_id = 40;

    // Act
    let id = actor
        .send_command("", "initialize", &params, &metadata)
        .unwrap();

    // Assert
    assert_eq!(id.get(), 41);
    assert_eq!(actor.core().pending_count(), 1);
    assert_eq!(actor.transport().frames.len(), 1);
    assert_eq!(
        serde_json::from_str::<Value>(&actor.transport().frames[0]).unwrap(),
        json!({"id":41,"guid":"","method":"initialize","params":{"sdkLanguage":"python"},"metadata":{}})
    );
}

#[test]
fn reply_id_correlates_with_the_original_pending_command() {
    // Arrange
    let mut core = ClientCore::new();
    core.begin_command("", "first", &empty_object(), &empty_object())
        .unwrap();
    let (second, _) = core
        .begin_command("", "second", &empty_object(), &empty_object())
        .unwrap();

    // Act
    let reply = core.ingest(r#"{"id":2,"result":{"ok":true}}"#).unwrap();

    // Assert
    assert!(
        matches!(reply, Ingress::Reply { pending, outcome: ReplyOutcome::Result(value) }
        if pending.id == second && pending.method == "second" && value == json!({"ok":true}))
    );
    assert_eq!(core.pending_count(), 1);
}

#[test]
fn remote_error_is_correlated_without_fabricating_success() {
    // Arrange
    let mut core = ClientCore::new();
    let (id, _) = core
        .begin_command("", "initialize", &empty_object(), &empty_object())
        .unwrap();

    // Act
    let reply = core
        .ingest(r#"{"id":1,"error":{"error":{"name":"Error","message":"closed","stack":"stack"}},"log":["line"]}"#)
        .unwrap();

    // Assert
    assert!(
        matches!(reply, Ingress::Reply { pending, outcome: ReplyOutcome::Error(RemoteError::Exception { name, message, stack, log }) }
        if pending.id == id && name == "Error" && message == "closed"
        && stack.as_deref() == Some("stack") && log == Some(json!(["line"])))
    );
    assert_eq!(core.pending_count(), 0);
}

#[test]
fn adoption_moves_child_between_known_parents() {
    // Arrange
    let mut core = ClientCore::new();
    core.ingest(r#"{"guid":"","method":"__create__","params":{"type":"Playwright","guid":"playwright@1","initializer":{}}}"#).unwrap();
    core.ingest(r#"{"guid":"playwright@1","method":"__create__","params":{"type":"BrowserType","guid":"browserType@1","initializer":{}}}"#).unwrap();
    core.ingest(r#"{"guid":"","method":"__create__","params":{"type":"Scope","guid":"scope@1","initializer":{}}}"#).unwrap();

    // Act
    let adopted = core
        .ingest(r#"{"guid":"scope@1","method":"__adopt__","params":{"guid":"browserType@1"}}"#)
        .unwrap();
    // Assert
    assert_eq!(
        adopted,
        Ingress::Adopted {
            parent: "scope@1".into(),
            guid: "browserType@1".into()
        }
    );
    assert_eq!(
        core.object("browserType@1").unwrap().parent.as_deref(),
        Some("scope@1")
    );
    assert!(
        core.object("scope@1")
            .unwrap()
            .children()
            .contains("browserType@1")
    );
    assert_eq!(core.object("playwright@1").unwrap().children().len(), 0);
}

#[test]
fn disposal_recursively_removes_descendant_guid_entries() {
    // Arrange
    let mut core = ClientCore::new();
    core.ingest(r#"{"guid":"","method":"__create__","params":{"type":"Scope","guid":"scope@1","initializer":{}}}"#).unwrap();
    core.ingest(r#"{"guid":"scope@1","method":"__create__","params":{"type":"BrowserType","guid":"browserType@1","initializer":{}}}"#).unwrap();

    // Act
    let disposed = core
        .ingest(r#"{"guid":"scope@1","method":"__dispose__","params":{"reason":"gc"}}"#)
        .unwrap();

    // Assert
    assert_eq!(
        disposed,
        Ingress::Disposed {
            guids: vec!["scope@1".into(), "browserType@1".into()],
            reason: Some("gc".into()),
        }
    );
    assert!(core.object("scope@1").is_none());
    assert!(core.object("browserType@1").is_none());
    assert_eq!(core.object_count(), 1);
}

#[test]
fn created_initializer_is_delivered_without_retention_in_guid_graph() {
    // Arrange
    let mut core = ClientCore::new();
    let frame = r#"{"guid":"","method":"__create__","params":{"type":"Playwright","guid":"pw@1","initializer":{"browserName":"chromium"}}}"#;

    // Act
    let created = core.ingest(frame).unwrap();

    // Assert
    assert_eq!(
        created,
        Ingress::Created {
            parent: String::new(),
            guid: "pw@1".into(),
            object_type: "Playwright".into(),
            initializer: json!({"browserName":"chromium"}),
        }
    );
    assert_eq!(core.object("pw@1").unwrap().object_type, "Playwright");
}

#[test]
fn events_retain_params_and_require_known_guid() {
    // Arrange
    let mut core = ClientCore::new();

    // Act
    let event = core
        .ingest(r#"{"guid":"","method":"close","params":{"reason":"done"}}"#)
        .unwrap();

    // Assert
    assert_eq!(
        event,
        Ingress::Event {
            guid: String::new(),
            method: "close".into(),
            params: json!({"reason":"done"})
        }
    );
}

#[test]
fn remote_close_drains_all_pending_and_forbids_new_commands() {
    // Arrange
    let mut actor = ClientActor::new(FakeTransport::default());
    actor
        .send_command("", "one", &empty_object(), &empty_object())
        .unwrap();
    actor
        .send_command("", "two", &empty_object(), &empty_object())
        .unwrap();

    // Act
    let report = actor.remote_closed("peer went away");

    // Assert
    assert_eq!(report.reason, CloseReason::Remote("peer went away".into()));
    assert_eq!(
        report
            .failed
            .iter()
            .map(|item| item.id.get())
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(actor.core().pending_count(), 0);
    assert_eq!(actor.core().object_count(), 0);
    assert!(matches!(
        actor.send_command("", "three", &empty_object(), &empty_object()),
        Err(SendFailure::Client(ClientError::Closed))
    ));
}

#[test]
fn duplicate_or_unknown_reply_terminates_and_drains_remaining_pending() {
    // Arrange
    let mut core = ClientCore::new();
    core.begin_command("", "one", &empty_object(), &empty_object())
        .unwrap();
    core.begin_command("", "two", &empty_object(), &empty_object())
        .unwrap();

    // Act
    let error = core.ingest(r#"{"id":3,"result":{}}"#).unwrap_err();

    // Assert
    assert!(matches!(error, ReceiveFailure::Protocol {
        violation: ProtocolViolation::UnknownResponse(CommandId(3)), report
    } if report.failed.len() == 2));
    assert_eq!(core.pending_count(), 0);
}

#[test]
fn malformed_id_is_not_misclassified_as_event() {
    // Arrange
    let frame = r#"{"id":0,"result":{}}"#;

    // Act
    let decoded = decode_inbound(frame);

    // Assert
    assert_eq!(decoded, Err(ProtocolViolation::InvalidId));
}

#[test]
fn missing_id_with_reply_payload_is_explicit_violation() {
    // Arrange
    let frame = r#"{"result":{}}"#;

    // Act
    let decoded = decode_inbound(frame);

    // Assert
    assert_eq!(decoded, Err(ProtocolViolation::InvalidId));
}

#[test]
fn unknown_parent_create_terminates_and_drains_pending() {
    // Arrange
    let mut core = ClientCore::new();
    core.begin_command("", "initialize", &empty_object(), &empty_object())
        .unwrap();

    // Act
    let error = core.ingest(r#"{"guid":"missing@1","method":"__create__","params":{"type":"Page","guid":"page@1","initializer":{}}}"#).unwrap_err();

    // Assert
    assert!(matches!(error, ReceiveFailure::Protocol {
        violation: ProtocolViolation::UnknownGuid(guid), report
    } if guid == "missing@1" && report.failed.len() == 1));
}

#[test]
fn duplicate_guid_create_is_fatal() {
    // Arrange
    let mut core = ClientCore::new();
    let frame = r#"{"guid":"","method":"__create__","params":{"type":"Page","guid":"page@1","initializer":{}}}"#;
    core.ingest(frame).unwrap();

    // Act
    let error = core.ingest(frame).unwrap_err();

    // Assert
    assert!(matches!(error, ReceiveFailure::Protocol {
        violation: ProtocolViolation::DuplicateGuid(guid), ..
    } if guid == "page@1"));
}

#[test]
fn adoption_cycle_is_rejected() {
    // Arrange
    let mut core = ClientCore::new();
    core.ingest(r#"{"guid":"","method":"__create__","params":{"type":"Parent","guid":"parent@1","initializer":{}}}"#).unwrap();
    core.ingest(r#"{"guid":"parent@1","method":"__create__","params":{"type":"Child","guid":"child@1","initializer":{}}}"#).unwrap();

    // Act
    let error = core
        .ingest(r#"{"guid":"child@1","method":"__adopt__","params":{"guid":"parent@1"}}"#)
        .unwrap_err();

    // Assert
    assert!(matches!(
        error,
        ReceiveFailure::Protocol {
            violation: ProtocolViolation::AdoptionCycle,
            ..
        }
    ));
}

#[test]
fn root_disposal_is_rejected() {
    // Arrange
    let mut core = ClientCore::new();

    // Act
    let error = core
        .ingest(r#"{"guid":"","method":"__dispose__","params":{}}"#)
        .unwrap_err();

    // Assert
    assert!(matches!(
        error,
        ReceiveFailure::Protocol {
            violation: ProtocolViolation::RootMutation,
            ..
        }
    ));
}

#[test]
fn transport_send_failure_spends_id_but_does_not_leave_pending_callback() {
    // Arrange
    let fake = FakeTransport {
        fail_next: true,
        ..FakeTransport::default()
    };
    let mut actor = ClientActor::new(fake);

    // Act
    let failure = actor.send_command("", "initialize", &empty_object(), &empty_object());

    // Assert
    assert!(matches!(
        failure,
        Err(SendFailure::Transport { error: "fake send failure", report })
        if report.reason == CloseReason::Transport && report.failed.len() == 1
    ));
    assert_eq!(actor.core().pending_count(), 0);
    assert_eq!(actor.core().last_command_id(), 1);
    assert_eq!(actor.transport().frames.len(), 0);
    assert_eq!(actor.core().close_reason(), Some(&CloseReason::Transport));
}

#[test]
fn serialized_non_exception_error_remains_distinct_from_exception() {
    // Arrange
    let frame = r#"{"id":1,"error":{"value":{"s":"thrown string"}},"log":[]}"#;

    // Act
    let decoded = decode_inbound(frame);

    // Assert
    assert!(matches!(decoded, Ok(WireInbound::Reply {
        outcome: ReplyOutcome::Error(RemoteError::Value { serialized, log }), ..
    }) if serialized == json!({"s":"thrown string"}) && log == Some(json!([]))));
}

#[test]
fn reply_with_omitted_result_represents_void_success() {
    // Arrange
    let frame = r#"{"id":1}"#;

    // Act
    let decoded = decode_inbound(frame);

    // Assert
    assert!(matches!(
        decoded,
        Ok(WireInbound::Reply {
            outcome: ReplyOutcome::Result(Value::Null),
            ..
        })
    ));
}

#[test]
fn outbound_limit_rejects_frame_without_consuming_id() {
    // Arrange
    let mut core = ClientCore::new();
    let params = json!({"large":"x".repeat(MAX_OUTBOUND_BYTES)});

    // Act
    let result = core.begin_command("", "initialize", &params, &empty_object());

    // Assert
    assert_eq!(result, Err(ClientError::OutboundTooLarge));
    assert_eq!(core.last_command_id(), 0);
    assert_eq!(core.pending_count(), 0);
}

#[test]
fn pending_limit_rejects_new_command_without_spending_id() {
    // Arrange
    let mut core = ClientCore::new();
    for _ in 0..MAX_PENDING {
        core.begin_command("", "initialize", &empty_object(), &empty_object())
            .unwrap();
    }

    // Act
    let result = core.begin_command("", "initialize", &empty_object(), &empty_object());

    // Assert
    assert_eq!(result, Err(ClientError::PendingLimit));
    assert_eq!(core.last_command_id(), MAX_PENDING as u64);
    assert_eq!(core.pending_count(), MAX_PENDING);
}

#[test]
fn command_id_overflow_fails_closed_without_reuse() {
    // Arrange
    let mut core = ClientCore::new();
    core.last_id = u64::MAX;

    // Act
    let result = core.begin_command("", "initialize", &empty_object(), &empty_object());

    // Assert
    assert_eq!(result, Err(ClientError::CommandIdExhausted));
    assert_eq!(core.pending_count(), 0);
    assert_eq!(core.last_command_id(), u64::MAX);
}

#[test]
fn inbound_limit_rejects_before_json_parse() {
    // Arrange
    let frame = "x".repeat(MAX_FRAME_BYTES + 1);

    // Act
    let result = decode_inbound(&frame);

    // Assert
    assert_eq!(result, Err(ProtocolViolation::FrameTooLarge));
}

#[test]
fn inbound_reply_requires_exactly_one_of_result_or_error() {
    // Arrange
    let frame = r#"{"id":1,"result":{},"error":{"error":{"name":"Error","message":"bad"}}}"#;

    // Act
    let result = decode_inbound(frame);

    // Assert
    assert_eq!(result, Err(ProtocolViolation::InvalidReply));
}

#[test]
fn decode_event_preserves_root_guid() {
    // Arrange
    let frame = r#"{"guid":"","method":"__create__","params":{"type":"Playwright","guid":"pw@1","initializer":{}}}"#;

    // Act
    let result = decode_inbound(frame);

    // Assert
    assert!(matches!(result, Ok(WireInbound::Event { guid, method, .. })
        if guid.is_empty() && method == "__create__"));
}
