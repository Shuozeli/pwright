//! Conservative, stateless pre-admission for Frame world consumers only.
use pwright_cdp::CdpEventAdmissionPolicy;
use serde_json::Value;

/// Opt in only when every installed consumer needs main/Pwright utility worlds
/// rather than an inventory of arbitrary execution contexts. Actual topology
/// still validates the exact per-Page world name/session; this prefix filter
/// does not confer route ownership or native-world destruction authority.
pub fn chromium_frame_world_event_admission() -> CdpEventAdmissionPolicy {
    CdpEventAdmissionPolicy::new("chromium-main-and-pwright-worlds", retain)
}

fn retain(method: &str, params: &Value, session: Option<&str>) -> bool {
    if method != "Runtime.executionContextCreated" || session.is_none_or(str::is_empty) {
        return true;
    }
    let Some(context) = params.get("context").and_then(Value::as_object) else {
        return true;
    };
    // Preserve generated decoding failures for malformed required fields.
    if context.get("id").and_then(Value::as_i64).is_none()
        || context.get("origin").and_then(Value::as_str).is_none()
        || context.get("uniqueId").and_then(Value::as_str).is_none()
    {
        return true;
    }
    let Some(name) = context.get("name").and_then(Value::as_str) else {
        return true;
    };
    let Some(aux) = context.get("auxData").and_then(Value::as_object) else {
        return true;
    };
    if aux.get("frameId").and_then(Value::as_str).is_none() {
        return true;
    }
    // Missing isDefault means false to the existing topology decoder. Keeping
    // missing here is deliberately conservative, as are non-Frame workers.
    if aux.get("isDefault").and_then(Value::as_bool) != Some(false) {
        return true;
    }
    name.starts_with(super::topology_events::UTILITY_WORLD_NAME_PREFIX)
}

#[cfg(test)]
mod tests;
