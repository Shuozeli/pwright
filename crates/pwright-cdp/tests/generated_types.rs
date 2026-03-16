//! Tests for generated CDP types — serialization, deserialization, and round-trips.
//!
//! These verify that generated types work correctly with Chrome's CDP protocol
//! and HTTP debug endpoints.

use serde_json::json;

/// Verify generated params skip None fields during serialization.
/// Chrome rejects null values as invalid parameters.
#[test]
fn params_skip_none_fields() {
    let params = pwright_cdp::generated::dom::FocusParams {
        node_id: Some(42),
        ..Default::default()
    };
    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json, json!({"nodeId": 42}));
    assert!(json.get("backendNodeId").is_none());
    assert!(json.get("objectId").is_none());
}

/// Verify CallArgument omits None fields (was causing -32602 errors).
#[test]
fn call_argument_skips_none_fields() {
    let arg = pwright_cdp::generated::runtime::CallArgument {
        value: Some(json!("hello")),
        ..Default::default()
    };
    let json = serde_json::to_value(&arg).unwrap();
    assert_eq!(json, json!({"value": "hello"}));
    assert!(json.get("unserializableValue").is_none());
    assert!(json.get("objectId").is_none());
}

/// Verify command params with all optional fields serialize to empty object.
#[test]
fn empty_optional_params_serialize_to_empty_object() {
    let params = pwright_cdp::generated::page::ReloadParams::default();
    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json, json!({}));
}

/// Verify command params with required fields always serialize them.
#[test]
fn required_fields_always_serialized() {
    let params = pwright_cdp::generated::target::CreateTargetParams {
        url: "about:blank".into(),
        ..Default::default()
    };
    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["url"], "about:blank");
}

/// Verify TargetInfo deserializes from CDP format.
#[test]
fn target_info_deserializes_cdp_format() {
    let cdp_json = json!({
        "targetId": "ABC123",
        "type": "page",
        "title": "Test",
        "url": "https://example.com",
        "attached": false,
        "canAccessOpener": false
    });
    let info: pwright_cdp::generated::target::TargetInfo =
        serde_json::from_value(cdp_json).unwrap();
    assert_eq!(info.target_id, "ABC123");
    assert_eq!(info.r#type, "page");
}

/// Verify TargetInfo alias: "id" works (Chrome HTTP endpoint uses "id" not "targetId").
#[test]
fn target_info_alias_id_field() {
    let http_json = json!({
        "id": "DEF456",
        "type": "page",
        "title": "Test",
        "url": "https://example.com",
        "attached": false,
        "canAccessOpener": false
    });
    let info: pwright_cdp::generated::target::TargetInfo =
        serde_json::from_value(http_json).unwrap();
    assert_eq!(info.target_id, "DEF456");
}

/// Verify returns structs deserialize correctly.
#[test]
fn navigate_returns_deserializes() {
    let json = json!({
        "frameId": "frame-1",
        "loaderId": "loader-1"
    });
    let returns: pwright_cdp::generated::page::NavigateReturns =
        serde_json::from_value(json).unwrap();
    assert_eq!(returns.frame_id, "frame-1");
    assert_eq!(returns.loader_id, Some("loader-1".into()));
    assert!(returns.error_text.is_none());
}

/// Verify enum serialization with serde rename.
#[test]
fn enum_serialization_roundtrip() {
    let val = pwright_cdp::generated::page::TransitionType::Link;
    let json = serde_json::to_value(&val).unwrap();
    assert_eq!(json, json!("link"));
    let restored: pwright_cdp::generated::page::TransitionType =
        serde_json::from_value(json).unwrap();
    assert_eq!(restored, pwright_cdp::generated::page::TransitionType::Link);
}

/// Verify type aliases work correctly.
#[test]
fn type_aliases_are_correct() {
    let _node_id: pwright_cdp::generated::dom::NodeId = 42;
    let _frame_id: pwright_cdp::generated::page::FrameId = "frame-1".to_string();
    let _target_id: pwright_cdp::generated::target::TargetID = "target-1".to_string();
}

/// Verify binary type (screenshot data) maps to String.
#[test]
fn screenshot_returns_has_string_data() {
    let json = json!({ "data": "iVBORw0KGgo=" });
    let returns: pwright_cdp::generated::page::CaptureScreenshotReturns =
        serde_json::from_value(json).unwrap();
    assert_eq!(returns.data, "iVBORw0KGgo=");
}

/// Verify multiple arguments serialize correctly for callFunctionOn.
#[test]
fn call_function_on_params_serialize() {
    let params = pwright_cdp::generated::runtime::CallFunctionOnParams {
        function_declaration: "function(a) { return a; }".into(),
        object_id: Some("obj-1".into()),
        arguments: Some(vec![pwright_cdp::generated::runtime::CallArgument {
            value: Some(json!(42)),
            ..Default::default()
        }]),
        return_by_value: Some(true),
        ..Default::default()
    };
    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["functionDeclaration"], "function(a) { return a; }");
    assert_eq!(json["objectId"], "obj-1");
    assert_eq!(json["arguments"][0]["value"], 42);
    assert!(json["arguments"][0].get("unserializableValue").is_none());
    assert_eq!(json["returnByValue"], true);
}
