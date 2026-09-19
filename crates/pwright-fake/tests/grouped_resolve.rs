use std::sync::Arc;
use std::task::{Context, Waker};

use pwright_cdp::CdpClient;
use pwright_fake::FakeCdpClient;
use serde_json::json;
use tokio::sync::Notify;

#[tokio::test]
async fn grouped_settlement_resolve_records_exact_params_before_adoption_gate() {
    // Arrange
    let fake = FakeCdpClient::from_html("");
    let gate = Arc::new(Notify::new());
    fake.set_adoption_gate(Some(gate.clone()));
    let expected = json!({"object":{"objectId":"unparsed-owner"}});
    fake.set_adopted_node_response(73, 19, expected.clone());

    // Act
    let mut operation = fake.dom_resolve_node_in_group_to_settlement(73, 19, "owned-group");
    let pending = operation
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    let recorded = fake.calls();
    gate.notify_one();
    let response = operation.await.unwrap();

    // Assert
    assert!(pending.is_pending());
    assert_eq!(
        recorded,
        [(
            "DOM.resolveNode".into(),
            vec![json!({
                "backendNodeId":73, "executionContextId":19, "objectGroup":"owned-group"
            })]
        )]
    );
    assert_eq!(response, expected);
}

#[tokio::test]
async fn grouped_settlement_resolve_does_not_invent_an_unconfigured_native_owner() {
    // Arrange
    let fake = FakeCdpClient::from_html("");

    // Act
    let result = fake
        .dom_resolve_node_in_group_to_settlement(73, 19, "owned-group")
        .await;

    // Assert
    assert!(result.is_err());
    assert_eq!(fake.method_names(), ["DOM.resolveNode"]);
}

#[tokio::test]
async fn group_release_settlement_records_before_gate_and_preserves_configured_failure() {
    // Arrange
    let fake = FakeCdpClient::from_html("");
    let gate = Arc::new(Notify::new());
    fake.set_release_object_group_gate(Some(gate.clone()));
    fake.set_release_object_error(Some("release sentinel".into()));

    // Act
    let mut operation = fake.runtime_release_object_group_to_settlement("owned-group");
    let pending = operation
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    let recorded = fake.calls();
    gate.notify_one();
    let result = operation.await;

    // Assert
    assert!(pending.is_pending());
    assert_eq!(
        recorded,
        [(
            "Runtime.releaseObjectGroup".into(),
            vec![json!("owned-group")]
        )]
    );
    assert!(
        matches!(result, Err(pwright_cdp::connection::CdpError::Protocol { message, .. }) if message == "release sentinel")
    );
}

#[tokio::test]
async fn grouped_retained_call_records_exact_flags_and_preserves_raw_exception() {
    // Arrange
    let fake = FakeCdpClient::from_html("");
    let gate = Arc::new(Notify::new());
    fake.set_evaluate_gate(Some(gate.clone()));
    let expected = json!({
        "result": {"type": "object", "objectId": "result-owner"},
        "exceptionDetails": {"exception": {"type": "object", "objectId": "error-owner"}}
    });
    fake.set_evaluate_response(expected.clone());
    let arguments = vec![json!({"objectId": "original-owned-node"})];

    // Act
    let mut operation = fake.runtime_call_function_as_object_in_group_to_settlement(
        "exact-world",
        "function(node){return node}",
        arguments.clone(),
        "owned-group",
        true,
    );
    let pending = operation
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    let recorded = fake.calls();
    gate.notify_one();
    let result = operation.await;

    // Assert
    assert!(pending.is_pending());
    assert_eq!(
        recorded,
        [(
            "Runtime.callFunctionOn".into(),
            vec![json!({
                "uniqueContextId": "exact-world", "functionDeclaration": "function(node){return node}",
                "arguments": arguments, "objectGroup": "owned-group",
                "returnByValue": false, "awaitPromise": false, "userGesture": true
            })]
        )]
    );
    assert_eq!(result.unwrap(), expected);
}

#[tokio::test]
async fn grouped_retained_call_does_not_invent_an_unconfigured_node_result() {
    // Arrange
    let fake = FakeCdpClient::from_html("");

    // Act
    let result = fake
        .runtime_call_function_as_object_in_group_to_settlement(
            "exact-world",
            "function(node){return node}",
            Vec::new(),
            "owned-group",
            false,
        )
        .await;

    // Assert
    assert_eq!(result.unwrap(), json!({"result": {"type": "undefined"}}));
    assert_eq!(fake.method_names(), ["Runtime.callFunctionOn"]);
}
