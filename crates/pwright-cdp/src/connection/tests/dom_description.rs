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

#[tokio::test]
async fn raw_object_description_preserves_node_metadata_and_exact_request() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::new(connection.clone(), "session".into(), "page".into());
    let client: &dyn crate::CdpClient = &session;
    let mut request = client.dom_describe_node_by_object_id_raw("retained-root");
    assert!(
        request
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let sent = wire.recv().await.unwrap();
    let sent: Value = serde_json::from_str(sent.message.to_text().unwrap()).unwrap();
    let expected = serde_json::json!({
        "node": {"frameId": "current-owner", "backendNodeId": 42, "custom": [1, 2]},
        "unrecognized": true
    });
    let response = serde_json::json!({"id": sent["id"], "result": expected});

    // Act
    CdpConnection::reader_loop(
        futures_util::stream::iter([Ok::<_, std::convert::Infallible>(Message::Text(
            response.to_string().into(),
        ))]),
        connection.pending.clone(),
        connection.event_delivery.clone(),
        4096,
        4096,
        2048,
        connection.resources.clone(),
    )
    .await;
    let outcome = request.await;

    // Assert
    assert_eq!(sent["method"], "DOM.describeNode");
    assert_eq!(sent["sessionId"], "session");
    assert_eq!(
        sent["params"],
        serde_json::json!({"objectId": "retained-root"})
    );
    assert_eq!(outcome.unwrap(), expected);
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}

#[tokio::test(start_paused = true)]
async fn raw_object_description_keeps_the_ordinary_command_deadline() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::new(connection.clone(), "session".into(), "page".into());
    let client: &dyn crate::CdpClient = &session;
    let mut request = client.dom_describe_node_by_object_id_raw("retained-root");
    assert!(
        request
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let _sent = wire.recv().await.unwrap();

    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let outcome = request.await;

    // Assert
    assert!(matches!(outcome, Err(CdpError::Timeout)));
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}
