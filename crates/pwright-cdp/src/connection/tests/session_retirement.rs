use super::*;
use std::task::{Context, Waker};

fn profile(selected: bool) -> CdpConnectionProductionProfile {
    let profile = CdpConnectionProductionProfile::new(
        ResourceScope::Connection,
        ConnectionConfig {
            pending_command_capacity: 4,
            write_channel_capacity: 4,
            event_channel_capacity: 8,
            max_command_bytes: 256,
            max_response_bytes: 512,
            max_event_bytes: 1024,
            max_websocket_bytes: 2048,
            ..ConnectionConfig::default()
        },
    );
    if selected {
        profile.with_pending_session_retirement(16).unwrap()
    } else {
        profile
    }
}

async fn read(connection: &CdpConnection, messages: Vec<Value>) {
    let resources = connection.resources.as_ref().unwrap();
    let config = resources.profile.config();
    CdpConnection::reader_loop(
        futures_util::stream::iter(messages.into_iter().map(|value| {
            Ok::<_, std::convert::Infallible>(Message::Text(value.to_string().into()))
        })),
        connection.pending.clone(),
        connection.event_delivery.clone(),
        config.max_response_bytes,
        config.max_event_bytes,
        config.max_protocol_error_bytes,
        connection.resources.clone(),
    )
    .await;
}

#[test]
fn retirement_reserves_bounded_identity_storage_without_another_lease_or_cursor() {
    // Arrange
    let plain = profile(false);
    // Act
    let selected = plain.clone().with_pending_session_retirement(16).unwrap();
    // Assert
    let increment = 16 + std::mem::size_of::<Option<Box<str>>>() as u64;
    assert_eq!(
        selected.command_wire_bytes(),
        plain.command_wire_bytes() + increment
    );
    assert_eq!(
        selected
            .requirements()
            .class(ResourceClass::WireBytes)
            .global,
        plain.requirements().class(ResourceClass::WireBytes).global + 4 * increment
    );
    assert_eq!(selected.requirements().leases, plain.requirements().leases);
    assert_eq!(
        selected.reclaiming_event_subscribers(),
        plain.reclaiming_event_subscribers()
    );
    assert!(selected.with_pending_session_retirement(16).is_err());
    assert!(plain.clone().with_pending_session_retirement(0).is_err());
    assert!(plain.with_pending_session_retirement(257).is_err());
}

#[tokio::test]
async fn native_detach_retires_only_matching_envelopes_and_preserves_event_and_late_reply_order() {
    // Arrange
    let profile = profile(true);
    let resources = production_resources(&profile);
    {
        let (connection, writer) = test_production_connection_with_writer(&profile, &resources);
        let mut events = connection.subscribe_composable_events().unwrap();
        let mut calls = [Some("child"), Some("child"), Some("parent"), None].map(|session| {
            Box::pin(connection.send_command_with_session(
                "Target.detachFromTarget",
                serde_json::json!({"sessionId":"child"}),
                session,
            ))
        });
        for call in &mut calls {
            assert!(
                call.as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            );
        }
        let event = serde_json::json!({"method":"Target.detachedFromTarget", "sessionId":"parent", "params":{"sessionId":"child"}});
        // Act: the actual reader sees detach before any command response. A late
        // success for the retired child must not revive or settle another command.
        read(
            &connection,
            vec![
                event.clone(),
                serde_json::json!({"id":1,"result":{"late":true}}),
                serde_json::json!({"id":3,"result":{"parent":true}}),
                serde_json::json!({"id":4,"result":{"root":true}}),
            ],
        )
        .await;
        let [first, second, parent, root] = calls;
        // Assert
        assert!(matches!(first.await, Err(CdpError::SessionDetached)));
        assert!(matches!(second.await, Err(CdpError::SessionDetached)));
        assert_eq!(parent.await.unwrap()["parent"], true);
        assert_eq!(root.await.unwrap()["root"], true);
        let delivered = events.try_recv().unwrap();
        assert_eq!(delivered.params, event["params"]);
        assert_eq!(delivered.session_id.as_deref(), Some("parent"));
        assert!(connection.pending.is_empty());
        assert_eq!(connection.active_pending.load(Ordering::Acquire), 0);
        drop(delivered);
        drop(events);
        drop(writer);
    }
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn opt_out_and_unrelated_or_malformed_events_leave_the_call_pending() {
    let cases = [
        (
            false,
            "Target.detachedFromTarget",
            serde_json::json!({"sessionId":"child"}),
        ),
        (
            true,
            "Target.detachedFromTarget",
            serde_json::json!({"sessionId":"other"}),
        ),
        (
            true,
            "Target.targetCrashed",
            serde_json::json!({"sessionId":"child"}),
        ),
        (
            true,
            "Target.detachedFromTarget",
            serde_json::json!({"sessionId":7}),
        ),
        (
            true,
            "Target.detachedFromTarget",
            serde_json::json!({"sessionId":""}),
        ),
        (
            true,
            "Target.detachedFromTarget",
            serde_json::json!({"targetId":"child"}),
        ),
        (
            true,
            "Target.detachedFromTarget",
            serde_json::json!({"sessionId":"x".repeat(17)}),
        ),
    ];
    for (selected, method, params) in cases {
        // Arrange
        let profile = profile(selected);
        let resources = production_resources(&profile);
        let (connection, writer) = test_production_connection_with_writer(&profile, &resources);
        let mut call = Box::pin(connection.send_command_with_session(
            "Page.enable",
            Value::Null,
            Some("child"),
        ));
        assert!(
            call.as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        // Act
        read(
            &connection,
            vec![serde_json::json!({"method":method,"params":params})],
        )
        .await;
        // Assert
        assert!(
            call.as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        assert_eq!(connection.pending.len(), 1);
        drop(call);
        drop(writer);
        drop(connection);
        assert_eq!(resources.metrics().unwrap().active_leases, 0);
    }
}

#[tokio::test]
async fn detach_before_first_poll_discards_unsent_wire_ownership() {
    // Arrange
    let profile = profile(true);
    let resources = production_resources(&profile);
    let (connection, mut writer) = test_production_connection_with_writer(&profile, &resources);
    let call = connection.send_command_with_session("Page.enable", Value::Null, Some("child"));
    // Act
    read(&connection, vec![serde_json::json!({"method":"Target.detachedFromTarget","params":{"sessionId":"child"}})]).await;
    let result = call.await;
    // Assert
    assert!(matches!(result, Err(CdpError::SessionDetached)));
    assert!(matches!(
        writer.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));
    assert!(connection.pending.is_empty());
    drop(writer);
    drop(connection);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn detach_settles_a_call_blocked_on_write_admission_without_sending_it() {
    // Arrange
    let profile = profile(true);
    let resources = production_resources(&profile);
    let mut connection = test_production_connection(&profile, &resources);
    let (sender, mut writer) = tokio::sync::mpsc::channel(1);
    connection.write_tx = sender;
    let mut root = Box::pin(connection.send_command("Target.getTargets", Value::Null));
    assert!(
        root.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let mut child =
        Box::pin(connection.send_command_with_session("Page.enable", Value::Null, Some("child")));
    assert!(
        child
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    read(&connection, vec![serde_json::json!({"method":"Target.detachedFromTarget","params":{"sessionId":"child"}})]).await;
    let result = child.await;
    // Assert
    assert!(matches!(result, Err(CdpError::SessionDetached)));
    assert_eq!(connection.pending.len(), 1);
    let sent: Value =
        serde_json::from_str(writer.try_recv().unwrap().message.to_text().unwrap()).unwrap();
    assert_eq!(sent["method"], "Target.getTargets");
    assert!(matches!(
        writer.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));
    drop(root);
    drop(writer);
    drop(connection);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn native_success_before_detach_remains_success() {
    // Arrange
    let profile = profile(true);
    let resources = production_resources(&profile);
    let (connection, writer) = test_production_connection_with_writer(&profile, &resources);
    let mut call =
        Box::pin(connection.send_command_with_session("Page.enable", Value::Null, Some("child")));
    assert!(
        call.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    read(&connection, vec![serde_json::json!({"id":1,"result":{"answer":42}}),
        serde_json::json!({"method":"Target.detachedFromTarget","params":{"sessionId":"child"}})]).await;
    let result = call.await;
    // Assert
    assert_eq!(result.unwrap()["answer"], 42);
    assert!(connection.pending.is_empty());
    drop(writer);
    drop(connection);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn overlong_session_identity_fails_admission_and_reclaims_the_command_lease() {
    // Arrange
    let profile = profile(true);
    let resources = production_resources(&profile);
    let connection = test_production_connection(&profile, &resources);
    // Act
    let result = connection
        .send_command_with_session("Page.enable", Value::Null, Some("12345678901234567"))
        .await;
    // Assert
    assert!(matches!(
        result,
        Err(CdpError::LimitExceeded {
            resource: "pending session ID bytes",
            ..
        })
    ));
    assert!(connection.pending.is_empty());
    assert_eq!(connection.active_pending.load(Ordering::Acquire), 0);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::WireBytes)
            .current,
        profile.reader_wire_bytes()
    );
}
