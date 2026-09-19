use super::*;
use pwright_backend_cdp::CdpWorldAttachment;
use pwright_cdp::generated::runtime::ExecutionContextCreatedEvent;

fn world(name: &str, default: bool) -> Value {
    serde_json::json!({"context": {
        "id": 3, "origin": "fixture", "name": name, "uniqueId": "unique",
        "auxData": {"frameId": "frame", "isDefault": default, "type": "isolated"}
    }})
}

#[test]
fn well_formed_foreign_world_omission_agrees_with_the_existing_topology_decoder() {
    // Arrange
    let utility = "__pwright_utility_0000000000000003";
    for name in [
        "__playwright_utility_world_page@old",
        "extension",
        "",
        "other",
    ] {
        let event = world(name, false);
        let typed: ExecutionContextCreatedEvent = serde_json::from_value(event.clone()).unwrap();
        // Act
        let retained = retain("Runtime.executionContextCreated", &event, Some("session"));
        let attachment =
            CdpWorldAttachment::from_execution_context("session", &typed, Some(utility)).unwrap();
        // Assert
        assert!(!retained);
        assert!(attachment.is_none());
    }
}

#[test]
fn main_world_is_kept_regardless_of_its_name() {
    // Arrange
    let event = world("__playwright_utility_world_unusual-main-name", true);
    // Act
    let retained = retain("Runtime.executionContextCreated", &event, Some("session"));
    // Assert
    assert!(retained);
}

#[test]
fn potential_pwright_worlds_remain_for_exact_route_validation() {
    // Arrange
    for name in [
        "__pwright_utility_0000000000000003",
        "__pwright_utility_0000000000000004",
        "__pwright_utility_unknown",
    ] {
        let event = world(name, false);
        // Act
        let retained = retain("Runtime.executionContextCreated", &event, Some("session"));
        // Assert: the prefix is deliberately not an ownership decision.
        assert!(retained);
    }
}

#[test]
fn missing_or_invalid_required_context_fields_reach_the_original_decoder() {
    // Arrange
    for field in ["id", "origin", "name", "uniqueId"] {
        for replacement in [None, Some(Value::Null), Some(serde_json::json!([]))] {
            let mut event = world("foreign", false);
            let context = event["context"].as_object_mut().unwrap();
            if let Some(value) = replacement {
                context.insert(field.into(), value);
            } else {
                context.remove(field);
            }
            // Act
            let retained = retain("Runtime.executionContextCreated", &event, Some("session"));
            let decoded = serde_json::from_value::<ExecutionContextCreatedEvent>(event);
            // Assert
            assert!(retained, "{field} must not be hidden");
            assert!(decoded.is_err());
        }
    }
}

#[test]
fn missing_unknown_and_malformed_auxiliary_fields_are_conservatively_kept() {
    // Arrange
    let values = [
        Value::Null,
        serde_json::json!([]),
        serde_json::json!({}),
        serde_json::json!({"frameId": 1, "isDefault":false}),
        serde_json::json!({"frameId":"frame", "isDefault":"false"}),
        serde_json::json!({"frameId":"frame"}),
        serde_json::json!({"type":"worker", "isDefault":false}),
    ];
    for aux in values {
        let mut event = world("foreign", false);
        event["context"]["auxData"] = aux;
        // Act
        let retained = retain("Runtime.executionContextCreated", &event, Some("session"));
        // Assert
        assert!(retained);
    }
}

#[test]
fn missing_session_or_context_is_never_filtered_away() {
    // Arrange
    let event = world("foreign", false);
    let cases = [
        (Some("session"), Value::Null),
        (Some("session"), serde_json::json!({"context":42})),
        (None, event.clone()),
        (Some(""), event),
    ];
    for (session, event) in cases {
        // Act
        let retained = retain("Runtime.executionContextCreated", &event, session);
        // Assert
        assert!(retained);
    }
}

#[test]
fn destruction_navigation_and_all_other_event_methods_remain_unfiltered() {
    // Arrange
    let misleading = world("foreign", false);
    for method in [
        "Runtime.executionContextDestroyed",
        "Runtime.executionContextsCleared",
        "Runtime.consoleAPICalled",
        "Target.detachedFromTarget",
        "Page.frameDetached",
        "Page.frameNavigated",
        "Unknown.event",
    ] {
        // Act
        let retained = retain(method, &misleading, Some("session"));
        // Assert
        assert!(retained);
    }
}
