//! Borrowed, bounded TRACE identities, never complete browser event payloads.
use serde_json::Value;

const MAX_ID_BYTES: usize = 256;

fn identity<'a>(method: &str, params: &'a Value) -> Option<(&'a str, &'a str)> {
    let (key, session) = match method {
        "Target.detachedFromTarget" => ("sessionId", true),
        "Target.targetDestroyed" => ("targetId", false),
        _ => return None,
    };
    let value = params.get(key)?.as_str()?;
    if value.is_empty() || value.len() > MAX_ID_BYTES {
        return None;
    }
    Some(if session { (value, "") } else { ("", value) })
}

pub(super) fn received(method: &str, params: &Value) {
    if !tracing::enabled!(target: "pwright_cdp::lifecycle", tracing::Level::TRACE) {
        return;
    }
    if !matches!(
        method,
        "Target.detachedFromTarget" | "Target.targetDestroyed"
    ) {
        return;
    }
    let ids = identity(method, params);
    let (session_id, target_id) = ids.unwrap_or(("", ""));
    tracing::trace!(target: "pwright_cdp::lifecycle", method, session_id, target_id,
        malformed = ids.is_none(), "native lifecycle identity received");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_detach_uses_the_inner_session_not_an_optional_target() {
        // Arrange
        let params = serde_json::json!({"sessionId":"child", "targetId":"target"});
        // Act
        let observed = identity("Target.detachedFromTarget", &params);
        // Assert
        assert_eq!(observed, Some(("child", "")));
    }

    #[test]
    fn native_target_retirement_does_not_invent_a_session() {
        // Arrange
        let params = serde_json::json!({"targetId":"target", "sessionId":"unrelated"});
        // Act
        let observed = identity("Target.targetDestroyed", &params);
        // Assert
        assert_eq!(observed, Some(("", "target")));
    }

    #[test]
    fn exact_byte_limit_is_borrowed_without_copying() {
        // Arrange
        let params = serde_json::json!({"sessionId":"x".repeat(MAX_ID_BYTES)});
        let original = params["sessionId"].as_str().unwrap();
        // Act
        let observed = identity("Target.detachedFromTarget", &params).unwrap();
        // Assert
        assert_eq!(observed.0.len(), MAX_ID_BYTES);
        assert_eq!(observed.0.as_ptr(), original.as_ptr());
    }

    #[test]
    fn malformed_or_oversized_identity_is_not_traced_as_a_valid_identity() {
        for value in [
            Value::Null,
            serde_json::json!({}),
            serde_json::json!({"sessionId":0}),
            serde_json::json!({"sessionId":""}),
            serde_json::json!({"sessionId":"界".repeat(86)}),
        ] {
            // Arrange
            let method = "Target.detachedFromTarget";
            // Act
            let observed = identity(method, &value);
            // Assert
            assert_eq!(observed, None);
        }
    }

    #[test]
    fn unrelated_events_do_not_supply_retirement_identity() {
        // Arrange
        let params = serde_json::json!({"sessionId":"child", "targetId":"target"});
        // Act
        let observed = identity("Target.targetCrashed", &params);
        // Assert
        assert_eq!(observed, None);
    }
}
