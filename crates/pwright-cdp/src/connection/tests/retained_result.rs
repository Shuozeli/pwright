use super::*;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

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

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

async fn sent(wire: &mut tokio::sync::mpsc::Receiver<OutboundMessage>) -> Value {
    serde_json::from_str(wire.recv().await.unwrap().message.to_text().unwrap()).unwrap()
}

// Feed the real decoder. No fake Runtime implementation can preserve exception
// IDs on behalf of the production methods under test.
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
    // The production I/O supervisor settles outstanding callers after EOF.
    fail_all_pending(&connection.pending);
}

fn exception_response() -> Value {
    serde_json::json!({
        "result": {"type": "object", "objectId": "result-owner"},
        "exceptionDetails": {
            "text": "Uncaught",
            "exception": {
                "type": "object", "objectId": "exception-owner",
                "description": "Error: failed"
            }
        }
    })
}

#[tokio::test(start_paused = true)]
async fn retained_evaluate_preserves_late_raw_result_and_exception_owners() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::new(connection.clone(), "session".into(), "page".into());
    let client: &dyn crate::CdpClient = &session;
    let mut native = client.runtime_evaluate_as_object_to_settlement("throw new Error('failed')");
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;
    let expected = exception_response();

    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let after_deadline = poll_now(native.as_mut());
    let competing = connection.send_command("test", serde_json::json!({})).await;
    peer_frames(
        &connection,
        vec![serde_json::json!({"id": frame["id"], "result": expected})],
    )
    .await;
    let outcome = native.await;

    // Assert
    assert!(after_deadline.is_pending());
    assert!(matches!(competing, Err(CdpError::LimitExceeded { .. })));
    assert_eq!(frame["method"], "Runtime.evaluate");
    assert_eq!(frame["sessionId"], "session");
    assert_eq!(
        frame["params"],
        serde_json::json!({
            "expression": "throw new Error('failed')", "returnByValue": false, "awaitPromise": true,
        })
    );
    assert_eq!(outcome.unwrap(), expected);
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}

#[tokio::test(start_paused = true)]
async fn retained_call_preserves_exact_world_arguments_and_late_raw_exception() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::new(connection.clone(), "session".into(), "page".into());
    let client: &dyn crate::CdpClient = &session;
    let arguments = vec![serde_json::json!({"objectId": "argument-owner"})];
    let mut native = client.runtime_call_function_as_object_in_unique_context_to_settlement(
        "world",
        "function(node){throw node}",
        arguments.clone(),
    );
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;
    let expected = exception_response();

    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let after_deadline = poll_now(native.as_mut());
    let competing = connection.send_command("test", serde_json::json!({})).await;
    peer_frames(
        &connection,
        vec![serde_json::json!({"id": frame["id"], "result": expected})],
    )
    .await;
    let outcome = native.await;

    // Assert
    assert!(after_deadline.is_pending());
    assert!(matches!(competing, Err(CdpError::LimitExceeded { .. })));
    assert_eq!(frame["method"], "Runtime.callFunctionOn");
    assert_eq!(frame["sessionId"], "session");
    assert_eq!(
        frame["params"],
        serde_json::json!({
            "uniqueContextId": "world", "functionDeclaration": "function(node){throw node}",
            "arguments": arguments, "returnByValue": false, "awaitPromise": true,
        })
    );
    assert_eq!(outcome.unwrap(), expected);
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}

#[tokio::test(start_paused = true)]
async fn retained_properties_preserve_late_descriptor_and_exception_owners() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::new(connection.clone(), "session".into(), "page".into());
    let client: &dyn crate::CdpClient = &session;
    let mut native = client.runtime_get_properties_to_settlement("parent-owner");
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;
    let expected = serde_json::json!({
        "result": [{
            "name": "child", "enumerable": true,
            "value": {"type": "object", "objectId": "property-owner"}
        }],
        "exceptionDetails": {
            "text": "Uncaught",
            "exception": {
                "type": "object", "objectId": "exception-owner",
                "description": "Error: failed"
            }
        }
    });

    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let after_deadline = poll_now(native.as_mut());
    let competing = connection.send_command("test", serde_json::json!({})).await;
    peer_frames(
        &connection,
        vec![serde_json::json!({"id": frame["id"], "result": expected})],
    )
    .await;
    let outcome = native.await;

    // Assert
    assert!(after_deadline.is_pending());
    assert!(matches!(competing, Err(CdpError::LimitExceeded { .. })));
    assert_eq!(frame["method"], "Runtime.getProperties");
    assert_eq!(frame["sessionId"], "session");
    assert_eq!(
        frame["params"],
        serde_json::json!({
            "objectId": "parent-owner", "ownProperties": true, "generatePreview": true,
        })
    );
    assert_eq!(outcome.unwrap(), expected);
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}

#[tokio::test(start_paused = true)]
async fn ordinary_properties_keep_their_response_deadline() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let mut native = Box::pin(session.runtime_get_properties("parent-owner"));
    assert!(poll_now(native.as_mut()).is_pending());
    let _ = sent(&mut wire).await;

    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let outcome = native.await;

    // Assert
    assert!(matches!(outcome, Err(CdpError::Timeout)));
    assert!(connection.pending.is_empty());
}

#[tokio::test]
async fn ordinary_properties_keep_javascript_exception_errors() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let mut native = Box::pin(session.runtime_get_properties("parent-owner"));
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;

    // Act
    peer_frames(
        &connection,
        vec![serde_json::json!({"id": frame["id"], "result": exception_response()})],
    )
    .await;
    let outcome = native.await;

    // Assert
    assert!(
        matches!(outcome, Err(CdpError::JsException(message)) if message.contains("Error: failed"))
    );
}

#[tokio::test(start_paused = true)]
async fn retained_release_waits_for_late_acknowledgement_and_keeps_its_slot() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::new(connection.clone(), "session".into(), "page".into());
    let client: &dyn crate::CdpClient = &session;
    let mut native = client.runtime_release_object_to_settlement("owned");
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;

    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let after_deadline = poll_now(native.as_mut());
    let competing = connection.send_command("test", serde_json::json!({})).await;
    peer_frames(
        &connection,
        vec![serde_json::json!({"id": frame["id"], "result": {}})],
    )
    .await;
    let outcome = native.await;

    // Assert
    assert!(after_deadline.is_pending());
    assert!(matches!(competing, Err(CdpError::LimitExceeded { .. })));
    assert_eq!(frame["method"], "Runtime.releaseObject");
    assert_eq!(frame["sessionId"], "session");
    assert_eq!(frame["params"], serde_json::json!({"objectId": "owned"}));
    assert!(outcome.is_ok());
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}

#[tokio::test]
async fn retained_release_transport_termination_is_not_a_successful_acknowledgement() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let client: &dyn crate::CdpClient = &session;
    let mut native = client.runtime_release_object_to_settlement("owned");
    assert!(poll_now(native.as_mut()).is_pending());
    let _ = sent(&mut wire).await;

    // Act
    peer_frames(&connection, vec![]).await;
    let outcome = native.await;

    // Assert
    assert!(matches!(outcome, Err(CdpError::Closed)));
    assert!(connection.pending.is_empty());
}

#[tokio::test(start_paused = true)]
async fn ordinary_retained_evaluate_keeps_its_response_deadline() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let mut native = Box::pin(session.runtime_evaluate_as_object("({})"));
    assert!(poll_now(native.as_mut()).is_pending());
    let _ = sent(&mut wire).await;

    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let outcome = native.await;

    // Assert
    assert!(matches!(outcome, Err(CdpError::Timeout)));
    assert!(connection.pending.is_empty());
}

#[tokio::test(start_paused = true)]
async fn ordinary_release_keeps_its_response_deadline() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let mut native = Box::pin(session.runtime_release_object("owned"));
    assert!(poll_now(native.as_mut()).is_pending());
    let _ = sent(&mut wire).await;

    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let outcome = native.await;

    // Assert
    assert!(matches!(outcome, Err(CdpError::Timeout)));
    assert!(connection.pending.is_empty());
}

#[tokio::test]
async fn ordinary_retained_evaluate_keeps_javascript_exception_errors() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let mut native = Box::pin(session.runtime_evaluate_as_object("throw new Error('failed')"));
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;

    // Act
    peer_frames(
        &connection,
        vec![serde_json::json!({"id": frame["id"], "result": exception_response()})],
    )
    .await;
    let outcome = native.await;

    // Assert
    assert!(
        matches!(outcome, Err(CdpError::JsException(message)) if message.contains("Error: failed"))
    );
}

#[tokio::test]
async fn ordinary_retained_call_keeps_javascript_exception_errors() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let mut native = Box::pin(session.runtime_call_function_as_object_in_unique_context(
        "world",
        "function(){throw new Error('failed')}",
        vec![],
    ));
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;

    // Act
    peer_frames(
        &connection,
        vec![serde_json::json!({"id": frame["id"], "result": exception_response()})],
    )
    .await;
    let outcome = native.await;

    // Assert
    assert!(
        matches!(outcome, Err(CdpError::JsException(message)) if message.contains("Error: failed"))
    );
}

#[tokio::test(start_paused = true)]
async fn grouped_resolve_retains_exact_world_group_and_late_raw_owner() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::new(connection.clone(), "session".into(), "page".into());
    let client: &dyn crate::CdpClient = &session;
    let mut native = client.dom_resolve_node_in_group_to_settlement(73, 19, "owned-group");
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;
    // Deliberately incomplete RemoteObject: semantic parsing belongs to the
    // group-owning backend, not this transport seam.
    let expected = serde_json::json!({"object":{"objectId":"late-owner"}});

    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let after_deadline = poll_now(native.as_mut());
    let competing = connection.send_command("test", serde_json::json!({})).await;
    peer_frames(
        &connection,
        vec![serde_json::json!({"id":frame["id"], "result":expected})],
    )
    .await;
    let outcome = native.await;

    // Assert
    assert!(after_deadline.is_pending());
    assert!(matches!(competing, Err(CdpError::LimitExceeded { .. })));
    assert_eq!(frame["method"], "DOM.resolveNode");
    assert_eq!(frame["sessionId"], "session");
    assert_eq!(
        frame["params"],
        serde_json::json!({
            "backendNodeId":73, "executionContextId":19, "objectGroup":"owned-group"
        })
    );
    assert_eq!(outcome.unwrap(), expected);
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}

#[tokio::test(start_paused = true)]
async fn ordinary_grouped_resolve_keeps_exact_params_and_response_deadline() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::new(connection.clone(), "session".into(), "page".into());
    let client: &dyn crate::CdpClient = &session;
    let mut native =
        client.dom_resolve_backend_node_in_execution_context_group(73, 19, "owned-group");
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;

    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let outcome = native.await;

    // Assert
    assert_eq!(frame["method"], "DOM.resolveNode");
    assert_eq!(frame["sessionId"], "session");
    assert_eq!(
        frame["params"],
        serde_json::json!({
            "backendNodeId":73, "executionContextId":19, "objectGroup":"owned-group"
        })
    );
    assert!(matches!(outcome, Err(CdpError::Timeout)));
    assert!(connection.pending.is_empty());
}

#[tokio::test]
async fn grouped_resolve_transport_termination_is_not_a_successful_acquisition() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let client: &dyn crate::CdpClient = &session;
    let mut native = client.dom_resolve_node_in_group_to_settlement(73, 19, "owned-group");
    assert!(poll_now(native.as_mut()).is_pending());
    let _ = sent(&mut wire).await;

    // Act
    peer_frames(&connection, vec![]).await;
    let outcome = native.await;

    // Assert
    assert!(matches!(outcome, Err(CdpError::Closed)));
    assert!(connection.pending.is_empty());
}

#[tokio::test(start_paused = true)]
async fn retained_group_release_keeps_exact_group_and_slot_until_late_acknowledgement() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::new(connection.clone(), "session".into(), "page".into());
    let client: &dyn crate::CdpClient = &session;
    let mut native = client.runtime_release_object_group_to_settlement("owned-group");
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;

    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let after_deadline = poll_now(native.as_mut());
    let competing = connection.send_command("test", serde_json::json!({})).await;
    peer_frames(
        &connection,
        vec![serde_json::json!({"id":frame["id"], "result":{}})],
    )
    .await;
    let outcome = native.await;

    // Assert
    assert!(after_deadline.is_pending());
    assert!(matches!(competing, Err(CdpError::LimitExceeded { .. })));
    assert_eq!(frame["method"], "Runtime.releaseObjectGroup");
    assert_eq!(frame["sessionId"], "session");
    assert_eq!(
        frame["params"],
        serde_json::json!({"objectGroup":"owned-group"})
    );
    assert!(outcome.is_ok());
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}

#[tokio::test(start_paused = true)]
async fn ordinary_group_release_keeps_its_response_deadline() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let client: &dyn crate::CdpClient = &session;
    let mut native = client.runtime_release_object_group("owned-group");
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;

    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let outcome = native.await;

    // Assert
    assert_eq!(frame["method"], "Runtime.releaseObjectGroup");
    assert_eq!(
        frame["params"],
        serde_json::json!({"objectGroup":"owned-group"})
    );
    assert!(matches!(outcome, Err(CdpError::Timeout)));
    assert!(connection.pending.is_empty());
}

#[tokio::test]
async fn retained_group_release_transport_termination_is_not_an_acknowledgement() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let client: &dyn crate::CdpClient = &session;
    let mut native = client.runtime_release_object_group_to_settlement("owned-group");
    assert!(poll_now(native.as_mut()).is_pending());
    let _ = sent(&mut wire).await;

    // Act
    peer_frames(&connection, vec![]).await;
    let outcome = native.await;

    // Assert
    assert!(matches!(outcome, Err(CdpError::Closed)));
    assert!(connection.pending.is_empty());
}

#[tokio::test(start_paused = true)]
async fn raw_group_value_call_keeps_gesture_await_and_late_exception_ownership() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::new(connection.clone(), "session".into(), "page".into());
    let client: &dyn crate::CdpClient = &session;
    let arguments = vec![
        serde_json::json!({"objectId": "persistent-state"}),
        serde_json::json!({"objectId": "adopted-target"}),
        serde_json::json!({"value": "call@17"}),
    ];
    let mut native = client.runtime_call_function_raw_in_group_to_settlement(
        "exact-world",
        "function(state, node, id){throw node}",
        arguments.clone(),
        "attempt-group",
        true,
    );
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;
    let expected = exception_response();

    // Act: ordinary response expiry cannot release this owned native work.
    tokio::time::advance(Duration::from_secs(2)).await;
    let after_deadline = poll_now(native.as_mut());
    let competing = connection.send_command("test", serde_json::json!({})).await;
    peer_frames(
        &connection,
        vec![serde_json::json!({"id": frame["id"], "result": expected})],
    )
    .await;
    let outcome = native.await;

    // Assert: production serialization and decoder, not fake JS execution.
    assert!(after_deadline.is_pending());
    assert!(matches!(competing, Err(CdpError::LimitExceeded { .. })));
    assert_eq!(frame["method"], "Runtime.callFunctionOn");
    assert_eq!(frame["sessionId"], "session");
    assert_eq!(
        frame["params"],
        serde_json::json!({
            "uniqueContextId": "exact-world",
            "functionDeclaration": "function(state, node, id){throw node}",
            "arguments": arguments, "objectGroup": "attempt-group",
            "returnByValue": true, "awaitPromise": true, "userGesture": true,
        })
    );
    assert_eq!(outcome.unwrap(), expected);
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}

#[tokio::test]
async fn raw_group_value_call_preserves_undefined_and_explicit_no_gesture() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let client: &dyn crate::CdpClient = &session;
    let mut native = client.runtime_call_function_raw_in_group_to_settlement(
        "exact-world",
        "function(){}",
        Vec::new(),
        "attempt-group",
        false,
    );
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;
    let expected = serde_json::json!({"result": {"type": "undefined"}});

    // Act
    peer_frames(
        &connection,
        vec![serde_json::json!({"id": frame["id"], "result": expected})],
    )
    .await;
    let outcome = native.await;

    // Assert
    assert_eq!(frame["params"]["awaitPromise"], true);
    assert_eq!(frame["params"]["returnByValue"], true);
    assert_eq!(frame["params"]["userGesture"], false);
    assert_eq!(frame["params"]["objectGroup"], "attempt-group");
    assert_eq!(outcome.unwrap(), expected);
    assert!(connection.pending.is_empty());
}

#[tokio::test]
async fn raw_group_value_call_rejects_invalid_arguments_without_dispatch() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let client: &dyn crate::CdpClient = &session;

    // Act
    let outcome = client
        .runtime_call_function_raw_in_group_to_settlement(
            "exact-world",
            "function(node){}",
            vec![serde_json::json!({"objectId": 42})],
            "attempt-group",
            true,
        )
        .await;

    // Assert
    assert!(matches!(outcome, Err(CdpError::Json(_))));
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}

#[tokio::test]
async fn raw_group_value_call_transport_termination_is_not_success() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let client: &dyn crate::CdpClient = &session;
    let mut native = client.runtime_call_function_raw_in_group_to_settlement(
        "exact-world",
        "function(){}",
        Vec::new(),
        "attempt-group",
        true,
    );
    assert!(poll_now(native.as_mut()).is_pending());
    let _frame = sent(&mut wire).await;

    // Act
    peer_frames(&connection, Vec::new()).await;
    let outcome = native.await;

    // Assert
    assert!(matches!(outcome, Err(CdpError::Closed)));
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}

#[tokio::test(start_paused = true)]
async fn group_retained_call_preserves_exact_routing_and_late_exception_owners() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::new(connection.clone(), "session".into(), "page".into());
    let client: &dyn crate::CdpClient = &session;
    let arguments = vec![
        serde_json::json!({"objectId": "original-owned-node"}),
        serde_json::json!({"value": {"multiple": true, "directoryUpload": false}}),
    ];
    let mut native = client.runtime_call_function_as_object_in_group_to_settlement(
        "exact-world",
        "function(node, flags){throw node}",
        arguments.clone(),
        "owned-group",
        true,
    );
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;
    let expected = exception_response();

    // Act
    tokio::time::advance(Duration::from_secs(2)).await;
    let after_deadline = poll_now(native.as_mut());
    let competing = connection.send_command("test", serde_json::json!({})).await;
    peer_frames(
        &connection,
        vec![serde_json::json!({"id": frame["id"], "result": expected})],
    )
    .await;
    let outcome = native.await;

    // Assert: real serialization/decoder evidence, not fake method forwarding.
    assert!(after_deadline.is_pending());
    assert!(matches!(competing, Err(CdpError::LimitExceeded { .. })));
    assert_eq!(frame["method"], "Runtime.callFunctionOn");
    assert_eq!(frame["sessionId"], "session");
    assert_eq!(
        frame["params"],
        serde_json::json!({
            "uniqueContextId": "exact-world",
            "functionDeclaration": "function(node, flags){throw node}",
            "arguments": arguments, "objectGroup": "owned-group",
            "returnByValue": false, "awaitPromise": false,
            "userGesture": true,
        })
    );
    assert_eq!(outcome.unwrap(), expected);
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}

#[tokio::test]
async fn group_retained_call_preserves_a_returned_promise_without_assimilation() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let client: &dyn crate::CdpClient = &session;
    let mut native = client.runtime_call_function_as_object_in_group_to_settlement(
        "exact-world",
        "function(){return Promise.resolve(1)}",
        Vec::new(),
        "owned-group",
        false,
    );
    assert!(poll_now(native.as_mut()).is_pending());
    let frame = sent(&mut wire).await;
    let expected = serde_json::json!({"result": {
        "type": "object", "subtype": "promise", "objectId": "promise-owner"
    }});

    // Act
    peer_frames(
        &connection,
        vec![serde_json::json!({"id": frame["id"], "result": expected})],
    )
    .await;
    let outcome = native.await;

    // Assert: this verifies the protocol flag and raw response, not browser JS.
    assert_eq!(frame["params"]["awaitPromise"], false);
    assert_eq!(frame["params"]["objectGroup"], "owned-group");
    assert_eq!(frame["params"]["userGesture"], false);
    assert_eq!(outcome.unwrap(), expected);
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}

#[tokio::test]
async fn group_retained_call_rejects_invalid_arguments_before_native_dispatch() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let client: &dyn crate::CdpClient = &session;

    // Act
    let outcome = client
        .runtime_call_function_as_object_in_group_to_settlement(
            "exact-world",
            "function(node){return node}",
            vec![serde_json::json!({"objectId": 42})],
            "owned-group",
            true,
        )
        .await;

    // Assert
    assert!(matches!(outcome, Err(CdpError::Json(_))));
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}

#[tokio::test]
async fn group_retained_call_transport_termination_never_reports_success() {
    // Arrange
    let (connection, mut wire) = connection();
    let session = crate::CdpSession::browser(connection.clone());
    let client: &dyn crate::CdpClient = &session;
    let mut native = client.runtime_call_function_as_object_in_group_to_settlement(
        "exact-world",
        "function(node){return node}",
        vec![serde_json::json!({"objectId": "original-owned-node"})],
        "owned-group",
        true,
    );
    assert!(poll_now(native.as_mut()).is_pending());
    let _ = sent(&mut wire).await;

    // Act
    peer_frames(&connection, Vec::new()).await;
    let outcome = native.await;

    // Assert
    assert!(matches!(outcome, Err(CdpError::Closed)));
    assert!(connection.pending.is_empty());
    assert!(wire.try_recv().is_err());
}
