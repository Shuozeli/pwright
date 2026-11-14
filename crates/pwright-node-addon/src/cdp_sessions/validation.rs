//! Bounded serialized CDP input accepted by the opt-in Node boundary.

use pwright_node_host::HostError;
use serde_json::Value;

pub const MAX_CDP_PARAMS_JSON_BYTES: usize = 1024 * 1024;
pub const MAX_CDP_RESULT_JSON_BYTES: usize = 4 * 1024 * 1024;

pub fn parse_params_json(source: &str) -> Result<Value, HostError> {
    if source.len() > MAX_CDP_PARAMS_JSON_BYTES {
        return Err(HostError::InvalidArgument(format!(
            "CDP params JSON exceeds {MAX_CDP_PARAMS_JSON_BYTES} UTF-8 bytes"
        )));
    }
    let params = serde_json::from_str::<Value>(source).map_err(|error| {
        HostError::InvalidArgument(format!("CDP params are not valid JSON: {error}"))
    })?;
    if !params.is_object() {
        return Err(HostError::InvalidArgument(
            "CDP params must be a JSON object".into(),
        ));
    }
    Ok(params)
}

pub fn measured_result(value: Value) -> Result<(Value, u64), HostError> {
    if !value.is_object() {
        return Err(HostError::Backend(
            "CDP command returned a non-object result".into(),
        ));
    }
    let bytes = serde_json::to_vec(&value)
        .map_err(|error| HostError::Backend(error.to_string()))?
        .len();
    if bytes > MAX_CDP_RESULT_JSON_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "CDP result exceeds {MAX_CDP_RESULT_JSON_BYTES} bytes"
        )));
    }
    Ok((value, bytes as u64))
}
