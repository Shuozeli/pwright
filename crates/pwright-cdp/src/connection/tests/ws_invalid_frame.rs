use super::*;
use std::task::{Context, Waker};

#[tokio::test]
async fn binary_frame_handling_is_explicitly_feature_gated() {
    // Arrange
    let mut connection = test_connection(ConnectionConfig {
        timeout: Duration::from_secs(5),
        ..ConnectionConfig::default()
    });
    let (sender, mut wire) = tokio::sync::mpsc::channel(1);
    connection.write_tx = sender;
    let connection = Arc::new(connection);
    let mut command =
        Box::pin(connection.send_command("Browser.getVersion", serde_json::json!({})));
    assert!(
        command
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let sent = wire.recv().await.expect("command should be admitted");
    let sent: Value = serde_json::from_str(sent.message.to_text().unwrap()).unwrap();
    let reply = serde_json::json!({"id": sent["id"], "result": {"product": "Chrome/1"}});
    let frames = [
        Ok::<_, std::convert::Infallible>(Message::Binary(b"private-frame".to_vec().into())),
        Ok(Message::Text(reply.to_string().into())),
    ];

    // Act
    CdpConnection::reader_loop(
        futures_util::stream::iter(frames),
        connection.pending.clone(),
        connection.event_delivery.clone(),
        4096,
        4096,
        2048,
        connection.resources.clone(),
    )
    .await;
    // Production's I/O supervisor performs this when the reader exits.
    fail_all_pending(&connection.pending);
    let result = command.await;

    // Assert
    #[cfg(feature = "cdp-ws-invalid-frame")]
    {
        let error = result.expect_err("binary CDP frame must stop the reader");
        assert!(matches!(error, CdpError::Closed));
        assert!(!error.to_string().contains("private-frame"));
    }
    #[cfg(not(feature = "cdp-ws-invalid-frame"))]
    assert_eq!(
        result.expect("legacy reader must ignore binary and accept next reply"),
        reply["result"]
    );
}

#[tokio::test]
async fn malformed_text_handling_is_explicitly_feature_gated() {
    // Arrange
    let mut connection = test_connection(ConnectionConfig {
        timeout: Duration::from_secs(5),
        ..ConnectionConfig::default()
    });
    let (sender, mut wire) = tokio::sync::mpsc::channel(1);
    connection.write_tx = sender;
    let connection = Arc::new(connection);
    let mut command =
        Box::pin(connection.send_command("Browser.getVersion", serde_json::json!({})));
    assert!(
        command
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let sent = wire.recv().await.expect("command should be admitted");
    let sent: Value = serde_json::from_str(sent.message.to_text().unwrap()).unwrap();
    let reply = serde_json::json!({"id": sent["id"], "result": {"product": "Chrome/1"}});
    let frames = [
        Ok::<_, std::convert::Infallible>(Message::Text("incoming-private-text".into())),
        Ok(Message::Text(reply.to_string().into())),
    ];

    // Act
    CdpConnection::reader_loop(
        futures_util::stream::iter(frames),
        connection.pending.clone(),
        connection.event_delivery.clone(),
        4096,
        4096,
        2048,
        connection.resources.clone(),
    )
    .await;
    // Production's I/O supervisor performs this when the reader exits.
    fail_all_pending(&connection.pending);
    let result = command.await;

    // Assert
    #[cfg(feature = "cdp-ws-invalid-frame")]
    {
        let error = result.expect_err("malformed JSON CDP frame must stop the reader");
        assert!(matches!(error, CdpError::Closed));
        assert!(!error.to_string().contains("incoming-private-text"));
    }
    #[cfg(not(feature = "cdp-ws-invalid-frame"))]
    assert_eq!(
        result.expect("legacy reader must ignore invalid JSON and accept next reply"),
        reply["result"]
    );
}
