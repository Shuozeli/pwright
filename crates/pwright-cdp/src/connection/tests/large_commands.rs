use super::*;
use crate::connection::large_commands::LargeCommandProfile;
use std::pin::Pin;
use std::task::{Context, Waker};

fn large() -> LargeCommandProfile {
    LargeCommandProfile::new(ResourceScope::extension(147, 1), 1, 1024, 64).unwrap()
}
fn base() -> CdpConnectionProductionProfile {
    CdpConnectionProductionProfile::new(
        ResourceScope::Connection,
        ConnectionConfig {
            timeout: Duration::from_secs(1),
            write_channel_capacity: 2,
            pending_command_capacity: 2,
            max_command_bytes: 256,
            max_response_bytes: 256,
            max_event_bytes: 256,
            max_websocket_bytes: 256,
            event_channel_capacity: 2,
            max_protocol_error_bytes: 128,
        },
    )
}
fn selected() -> (
    CdpConnection,
    tokio::sync::mpsc::Receiver<OutboundMessage>,
    ProductionResourceLedger,
) {
    let profile = base().with_large_commands(large()).unwrap();
    let resources = production_resources(&profile);
    let (connection, wire) = test_production_connection_with_writer(&profile, &resources);
    (connection, wire, resources)
}
fn poll<F: Future>(future: Pin<&mut F>) -> std::task::Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}
fn frame(message: &OutboundMessage) -> Value {
    serde_json::from_str(message.message.to_text().unwrap()).unwrap()
}
async fn replies(connection: &CdpConnection, values: Vec<Value>) {
    CdpConnection::reader_loop(
        futures_util::stream::iter(values.into_iter().map(|value| {
            Ok::<_, std::convert::Infallible>(Message::Text(value.to_string().into()))
        })),
        connection.pending.clone(),
        connection.event_delivery.clone(),
        256,
        256,
        128,
        connection.resources.clone(),
    )
    .await;
    fail_all_pending(&connection.pending);
}

#[test]
fn selection_preserves_ordinary_and_inbound_limits_in_any_builder_order() {
    // Arrange
    let plain = base();
    // Act
    let first = plain
        .clone()
        .with_large_commands(large())
        .unwrap()
        .with_pending_session_retirement(64)
        .unwrap()
        .with_reclaiming_event_fanout(2)
        .unwrap();
    let last = plain
        .clone()
        .with_reclaiming_event_fanout(2)
        .unwrap()
        .with_pending_session_retirement(64)
        .unwrap()
        .with_large_commands(large())
        .unwrap();
    // Assert
    assert_eq!(first.requirements(), last.requirements());
    assert_eq!(
        first.config().max_command_bytes,
        plain.config().max_command_bytes
    );
    assert_eq!(
        first.config().max_websocket_bytes,
        plain.config().max_websocket_bytes
    );
    assert_eq!(
        first.config().max_response_bytes,
        plain.config().max_response_bytes
    );
    assert_eq!(first.reader_wire_bytes(), plain.reader_wire_bytes());
    assert!(first.with_large_commands(large()).is_err());
}

#[tokio::test]
async fn an_unselected_connection_cannot_use_the_large_command_api() {
    // Arrange
    let connection = test_connection(base().config().clone());
    // Act
    let result = connection
        .send_large_command_to_settlement_with_session("test", Value::Null, None)
        .await;
    // Assert
    assert!(matches!(result, Err(CdpError::InvalidConfiguration(_))));
    assert_eq!(connection.large_command_metrics(), None);
}

#[tokio::test]
async fn explicit_disconnect_fails_large_waiters_but_retains_queued_wire_backing() {
    // Arrange
    let (connection, mut wire, resources) = selected();
    let mut future = Box::pin(connection.send_large_command_to_settlement_with_session(
        "Runtime.evaluate",
        serde_json::json!({"expression":"x".repeat(400)}),
        Some("session-a"),
    ));
    assert!(poll(future.as_mut()).is_pending());
    // Act
    connection.request_disconnect();
    // Assert
    assert!(matches!(future.await, Err(CdpError::Closed)));
    assert!(connection.is_closed());
    assert_eq!(connection.large_command_metrics().unwrap().active, 1);
    let queued = wire.try_recv().unwrap();
    assert_eq!(frame(&queued)["method"], "Runtime.evaluate");
    drop(queued);
    assert_eq!(connection.large_command_metrics().unwrap().active, 0);
    let new_call = connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await;
    assert!(matches!(new_call, Err(CdpError::Closed)));
    assert!(wire.try_recv().is_err());
    drop(connection);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn oversized_ordinary_command_stays_rejected_while_selected_large_command_succeeds() {
    // Arrange
    let (connection, mut wire, _) = selected();
    let value = Value::String("x".repeat(300));
    assert!(matches!(
        connection.send_command("test", value.clone()).await,
        Err(CdpError::LimitExceeded {
            resource: "command bytes",
            ..
        })
    ));
    // Act
    let mut native = Box::pin(connection.send_large_command_to_settlement_with_session(
        "test",
        value.clone(),
        Some("native-session"),
    ));
    assert!(poll(native.as_mut()).is_pending());
    let outbound = wire.recv().await.unwrap();
    let sent = frame(&outbound);
    // Assert
    assert_eq!(sent["params"], value);
    assert_eq!(sent["sessionId"], "native-session");
    replies(
        &connection,
        vec![serde_json::json!({"id":sent["id"],"result":{"ok":true}})],
    )
    .await;
    drop(outbound);
    assert_eq!(native.await.unwrap()["ok"], true);
    assert_eq!(connection.large_command_metrics().unwrap().active, 0);
}

#[tokio::test(start_paused = true)]
async fn large_slot_survives_timeout_and_unconsumed_reply_without_blocking_ordinary_admission() {
    // Arrange
    let (connection, mut wire, _) = selected();
    let mut large = Box::pin(connection.send_large_command_to_settlement_with_session(
        "large",
        Value::Null,
        None,
    ));
    assert!(poll(large.as_mut()).is_pending());
    let sent = frame(&wire.recv().await.unwrap());
    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    // Assert
    assert!(poll(large.as_mut()).is_pending());
    assert!(matches!(
        connection
            .send_large_command_to_settlement_with_session("other", Value::Null, None)
            .await,
        Err(CdpError::LimitExceeded {
            resource: "large commands",
            ..
        })
    ));
    let mut ordinary = Box::pin(connection.send_command("ordinary", Value::Null));
    assert!(poll(ordinary.as_mut()).is_pending());
    let other = frame(&wire.recv().await.unwrap());
    replies(
        &connection,
        vec![
            serde_json::json!({"id":sent["id"],"result":{}}),
            serde_json::json!({"id":other["id"],"result":{}}),
        ],
    )
    .await;
    ordinary.await.unwrap();
    assert_eq!(connection.large_command_metrics().unwrap().active, 1);
    large.await.unwrap();
    assert_eq!(connection.large_command_metrics().unwrap().active, 0);
}

#[tokio::test]
async fn cancelling_the_native_wait_does_not_release_a_queued_large_message() {
    // Arrange
    let (connection, mut wire, _) = selected();
    let mut native = Box::pin(connection.send_large_command_to_settlement_with_session(
        "large",
        Value::Null,
        None,
    ));
    assert!(poll(native.as_mut()).is_pending());
    // Act
    drop(native);
    // Assert
    assert_eq!(connection.large_command_metrics().unwrap().active, 1);
    assert!(matches!(
        connection
            .send_large_command_to_settlement_with_session("other", Value::Null, None)
            .await,
        Err(CdpError::LimitExceeded {
            resource: "large commands",
            ..
        })
    ));
    drop(wire.recv().await.unwrap());
    assert_eq!(connection.large_command_metrics().unwrap().active, 0);
}

#[tokio::test]
async fn oversized_selected_message_returns_its_slot_and_wire_admission() {
    // Arrange
    let (connection, mut wire, resources) = selected();
    let before = resources.metrics().unwrap().active_leases;
    // Act
    let result = connection
        .send_large_command_to_settlement_with_session(
            "large",
            Value::String("x".repeat(1024)),
            None,
        )
        .await;
    // Assert
    assert!(matches!(
        result,
        Err(CdpError::LimitExceeded {
            resource: "command bytes",
            ..
        })
    ));
    assert_eq!(connection.large_command_metrics().unwrap().active, 0);
    assert_eq!(resources.metrics().unwrap().active_leases, before);
    assert!(wire.try_recv().is_err());
}

#[tokio::test]
async fn an_unpolled_large_send_releases_without_queueing() {
    // Arrange
    let (connection, mut wire, _) = selected();
    let native =
        connection.send_large_command_to_settlement_with_session("large", Value::Null, None);
    assert_eq!(connection.large_command_metrics().unwrap().active, 1);
    // Act
    drop(native);
    // Assert
    assert_eq!(connection.large_command_metrics().unwrap().active, 0);
    assert!(wire.try_recv().is_err());
}

#[test]
fn large_profiles_reject_zero_overflow_and_invalid_fragment_limits() {
    // Arrange
    let scope = ResourceScope::Connection;
    // Act
    let results = [
        (0, 1024, 64),
        (1, 0, 64),
        (1, 1024, 0),
        (1, 64, 128),
        (1, usize::MAX, 64),
        (usize::MAX, 1024, 64),
    ]
    .map(|(count, bytes, fragment)| LargeCommandProfile::new(scope, count, bytes, fragment));
    // Assert
    assert!(results.iter().all(Result::is_err));
    assert!(
        base()
            .with_large_commands(LargeCommandProfile::new(scope, 1, 256, 64).unwrap())
            .is_err()
    );
    assert!(
        base()
            .with_large_commands(LargeCommandProfile::new(scope, 3, 1024, 64).unwrap())
            .is_err()
    );
    assert!(
        base()
            .with_large_commands(LargeCommandProfile::new(scope, 1, 1024, 512).unwrap())
            .is_err()
    );
}
