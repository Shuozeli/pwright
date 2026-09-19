use super::*;
use std::task::{Context, Waker};

fn connection() -> (
    Arc<CdpConnection>,
    tokio::sync::mpsc::Receiver<OutboundMessage>,
) {
    let mut connection = test_connection(ConnectionConfig {
        timeout: Duration::from_secs(1),
        pending_command_capacity: 1,
        ..ConnectionConfig::default()
    });
    let (sender, wire) = tokio::sync::mpsc::channel(1);
    connection.write_tx = sender;
    (Arc::new(connection), wire)
}

// In-memory peer fixture: feed the real response decoder, not the tested API.
async fn peer_frames(connection: &CdpConnection, frames: Vec<Value>) {
    CdpConnection::reader_loop(
        futures_util::stream::iter(frames.into_iter().map(|frame| {
            Ok::<_, std::convert::Infallible>(Message::Text(frame.to_string().into()))
        })),
        connection.pending.clone(),
        connection.event_delivery.clone(),
        4096,
        4096,
        2048,
        connection.resources.clone(),
    )
    .await;
    // Production's I/O supervisor performs this after reader termination.
    fail_all_pending(&connection.pending);
}

#[tokio::test(start_paused = true)]
async fn group_owned_settlement_survives_ordinary_timeout_and_accepts_late_reply() {
    // Arrange
    let (connection, mut wire) = connection();
    let client = crate::CdpSession::new(connection.clone(), "session".into(), "page".into());
    let mut native = Box::pin(
        client.runtime_call_function_by_value_in_group_to_settlement(
            "world",
            "function(node){return 7}",
            vec![serde_json::json!({"objectId":"node"})],
            "owned-group",
        ),
    );
    assert!(
        native
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let frame: Value =
        serde_json::from_str(wire.recv().await.unwrap().message.to_text().unwrap()).unwrap();
    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    // Assert: real serialized parameters and the still-held admission slot.
    assert_eq!(frame["sessionId"], "session");
    assert_eq!(frame["params"]["objectGroup"], "owned-group");
    assert_eq!(frame["params"]["uniqueContextId"], "world");
    assert_eq!(frame["params"]["returnByValue"], true);
    assert!(
        native
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert!(matches!(
        connection.send_command("test", serde_json::json!({})).await,
        Err(CdpError::LimitExceeded { .. })
    ));
    peer_frames(
        &connection,
        vec![serde_json::json!({"id":frame["id"],"result":{"result":{"value":7}}})],
    )
    .await;
    assert_eq!(native.await.unwrap()["result"]["value"], 7);
    assert!(wire.try_recv().is_err());
}

#[tokio::test(start_paused = true)]
async fn ordinary_commands_keep_their_existing_response_deadline() {
    // Arrange
    let (connection, mut wire) = connection();
    let mut native = Box::pin(connection.send_command("test", serde_json::json!({})));
    assert!(
        native
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let _ = wire.recv().await.unwrap();
    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let result = native.await;
    // Assert
    assert!(matches!(result, Err(CdpError::Timeout)));
    let mut next = Box::pin(connection.send_command("next", serde_json::json!({})));
    assert!(
        next.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let frame: Value =
        serde_json::from_str(wire.recv().await.unwrap().message.to_text().unwrap()).unwrap();
    assert_eq!(frame["method"], "next");
    peer_frames(&connection, vec![]).await;
    assert!(matches!(next.await, Err(CdpError::Closed)));
}

#[tokio::test]
async fn transport_termination_settles_the_retained_wait_as_an_error() {
    // Arrange
    let (connection, mut wire) = connection();
    let mut native = Box::pin(connection.send_command_to_settlement_with_session(
        "test",
        serde_json::json!({}),
        None,
    ));
    assert!(
        native
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let _ = wire.recv().await.unwrap();
    // Act
    peer_frames(&connection, vec![]).await;
    let result = native.await;
    // Assert
    assert!(matches!(result, Err(CdpError::Closed)));
}

#[tokio::test]
async fn grouped_settlement_preserves_the_native_exception() {
    // Arrange
    let (connection, mut wire) = connection();
    let client = crate::CdpSession::browser(connection.clone());
    let mut native = Box::pin(
        client.runtime_call_function_by_value_in_group_to_settlement(
            "world",
            "function(){throw new Error('failed')}",
            vec![],
            "owned-group",
        ),
    );
    assert!(
        native
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let frame: Value =
        serde_json::from_str(wire.recv().await.unwrap().message.to_text().unwrap()).unwrap();
    // Act
    peer_frames(&connection, vec![serde_json::json!({"id":frame["id"],"result":{
        "exceptionDetails":{"text":"Uncaught","exception":{"description":"Error: failed","objectId":"exception"}}
    }})]).await;
    let result = native.await;
    // Assert
    assert_eq!(frame["params"]["objectGroup"], "owned-group");
    assert!(
        matches!(result, Err(CdpError::JsException(message)) if message.contains("Error: failed"))
    );
}

#[tokio::test]
async fn explicitly_selected_session_retirement_settles_its_retained_command() {
    // Arrange
    let profile = CdpConnectionProductionProfile::new(
        ResourceScope::Connection,
        ConnectionConfig {
            pending_command_capacity: 1,
            write_channel_capacity: 1,
            max_command_bytes: 1024,
            max_response_bytes: 4096,
            max_event_bytes: 4096,
            max_websocket_bytes: 8192,
            event_channel_capacity: 2,
            max_protocol_error_bytes: 1024,
            timeout: Duration::from_secs(1),
        },
    )
    .with_pending_session_retirement(64)
    .unwrap();
    let resources = production_resources(&profile);
    let (connection, mut wire) = test_production_connection_with_writer(&profile, &resources);
    let connection = Arc::new(connection);
    let client = crate::CdpSession::new(connection.clone(), "session".into(), "page".into());
    let mut native = Box::pin(client.send_to_settlement("test", serde_json::json!({})));
    assert!(
        native
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let _ = wire.recv().await.unwrap();
    // Act
    peer_frames(&connection, vec![serde_json::json!({"method":"Target.detachedFromTarget","params":{"sessionId":"session"}})]).await;
    let result = native.await;
    // Assert: this is exact registered-session settlement, not a timeout or
    // generic transport-close guess. Source-group destruction remains separate.
    assert!(matches!(result, Err(CdpError::SessionDetached)));
}
