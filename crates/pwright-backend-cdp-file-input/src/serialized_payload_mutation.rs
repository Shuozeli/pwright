//! Bounded mutation of the package-produced serialized file-payload subset.
//! No selector lookup, retargeting, Handle adoption or public error rewriting.

use super::{PayloadError, PayloadTarget};
use pwright_cdp::connection::CdpError;
use pwright_core::{
    ProductionResourceError, ProductionResourceLedger, ResourceClass, ResourceError,
    ResourceLedger, ResourceRequest, ResourceRequirements, ResourceScope,
};
use serde_json::{Map, Value};
use std::fmt;

const MUTATION_SCRIPT: &str = include_str!("serialized_payload_mutation.js");
const ITEMS: usize = 64;
const NAME_BYTES: usize = 4096;
const MIME_BYTES: usize = 1024;
const METADATA_BYTES: usize = 64 * 1024;
const BASE64_BYTES: usize = 70 * 1024 * 1024;
const CONTROL_BYTES: u64 = 64 * 1024;

#[derive(Debug)]
pub enum SerializedPayloadMutationError {
    InvalidProfile,
    Protocol(&'static str),
    Capacity,
    Budget(ResourceError),
    Plan(ProductionResourceError),
    Target(PayloadError),
    Backend(CdpError),
}

impl fmt::Display for SerializedPayloadMutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile => f.write_str("invalid serialized-payload mutation profile"),
            Self::Protocol(detail) => {
                write!(f, "invalid serialized file-input payload graph: {detail}")
            }
            Self::Capacity => {
                f.write_str("serialized file-input payload exceeds mutation capacity")
            }
            Self::Budget(error) => error.fmt(f),
            Self::Plan(error) => error.fmt(f),
            Self::Target(error) => error.fmt(f),
            Self::Backend(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for SerializedPayloadMutationError {}

#[derive(Clone, Copy, Debug)]
pub struct SerializedPayloadMutationProfile {
    scope: ResourceScope,
    calls: usize,
}

impl SerializedPayloadMutationProfile {
    pub fn new(scope: ResourceScope, calls: usize) -> Result<Self, SerializedPayloadMutationError> {
        if calls == 0 {
            return Err(SerializedPayloadMutationError::InvalidProfile);
        }
        let profile = Self { scope, calls };
        profile.requirements()?;
        Ok(profile)
    }

    pub fn requirements(self) -> Result<ResourceRequirements, SerializedPayloadMutationError> {
        let calls = u64::try_from(self.calls)
            .map_err(|_| SerializedPayloadMutationError::InvalidProfile)?;
        let bytes = CONTROL_BYTES
            .checked_mul(calls)
            .ok_or(SerializedPayloadMutationError::InvalidProfile)?;
        Ok(ResourceRequirements::none()
            .with_class(ResourceClass::ActiveCommand, calls, calls)
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, self.calls))
    }
}

pub struct CdpSerializedPayloadMutationService {
    profile: SerializedPayloadMutationProfile,
    ledger: ResourceLedger,
}

impl CdpSerializedPayloadMutationService {
    pub fn new(
        profile: SerializedPayloadMutationProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, SerializedPayloadMutationError> {
        Ok(Self {
            profile,
            ledger: resources
                .claim(profile.requirements()?)
                .map_err(SerializedPayloadMutationError::Plan)?,
        })
    }

    /// Consume one already-imported graph and mutate the caller-retained,
    /// already-retargeted INPUT in its exact utility world and object group.
    pub async fn apply_on_target(
        &self,
        target: PayloadTarget<'_>,
        graph: Value,
    ) -> Result<(), SerializedPayloadMutationError> {
        validate_graph(&graph)?;
        let _lease = self
            .ledger
            .reserve_many(
                self.profile.scope,
                &[
                    ResourceRequest::new(ResourceClass::ActiveCommand, 1),
                    ResourceRequest::new(ResourceClass::BackendStateBytes, CONTROL_BYTES),
                ],
            )
            .map_err(SerializedPayloadMutationError::Budget)?;
        let arguments = arguments(target.object_id, graph);
        let response = target
            .client
            .runtime_call_function_by_value_in_large_group_with_user_gesture_to_settlement(
                target.world,
                MUTATION_SCRIPT,
                arguments,
                target.group,
                true,
            )
            .await
            .map_err(SerializedPayloadMutationError::Backend)?;
        if response.get("result").is_none() {
            return Err(SerializedPayloadMutationError::Protocol(
                "native response omitted result",
            ));
        }
        // Playwright ignores injected.setInputFiles' return value. A late
        // non-file target is therefore a successful no-op, not a new error.
        Ok(())
    }
}

fn arguments(object_id: &str, graph: Value) -> Vec<Value> {
    let mut target = Map::new();
    target.insert("objectId".into(), Value::String(object_id.to_owned()));
    let mut payload = Map::new();
    payload.insert("value".into(), graph);
    vec![Value::Object(target), Value::Object(payload)]
}

fn exact_object<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Map<String, Value>> {
    let object = value.as_object()?;
    (object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key)))
        .then_some(object)
}

fn positive_id(value: &Value) -> Option<u64> {
    value.as_u64().filter(|id| *id != 0).or_else(|| {
        value
            .as_f64()
            .filter(|id| {
                id.is_finite() && id.fract() == 0.0 && *id >= 1.0 && *id <= (ITEMS + 1) as f64
            })
            .map(|id| id as u64)
    })
}

fn validate_graph(graph: &Value) -> Result<(), SerializedPayloadMutationError> {
    let root = exact_object(graph, &["a", "id"])
        .ok_or(SerializedPayloadMutationError::Protocol("root shape"))?;
    if positive_id(&root["id"]) != Some(1) {
        return Err(SerializedPayloadMutationError::Protocol("root id"));
    }
    let entries = root["a"]
        .as_array()
        .ok_or(SerializedPayloadMutationError::Protocol("root items"))?;
    if entries.len() > ITEMS {
        return Err(SerializedPayloadMutationError::Capacity);
    }

    let mut base64_bytes = 0usize;
    let mut metadata_bytes = 0usize;
    for (index, entry) in entries.iter().enumerate() {
        let object = exact_object(entry, &["o", "id"])
            .ok_or(SerializedPayloadMutationError::Protocol("item shape"))?;
        let expected_id = u64::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(2))
            .ok_or(SerializedPayloadMutationError::Capacity)?;
        if positive_id(&object["id"]) != Some(expected_id) {
            return Err(SerializedPayloadMutationError::Protocol("item id"));
        }
        let fields = object["o"]
            .as_array()
            .filter(|fields| fields.len() == 4)
            .ok_or(SerializedPayloadMutationError::Protocol("item fields"))?;
        let name = field(fields, 0, "name")?
            .as_str()
            .ok_or(SerializedPayloadMutationError::Protocol("name value"))?;
        let mime = field(fields, 1, "mimeType")?
            .as_str()
            .ok_or(SerializedPayloadMutationError::Protocol("mime value"))?;
        let buffer = field(fields, 2, "buffer")?
            .as_str()
            .ok_or(SerializedPayloadMutationError::Protocol("buffer value"))?;
        validate_modified(field(fields, 3, "lastModifiedMs")?)?;
        if name.len() > NAME_BYTES || mime.len() > MIME_BYTES || !valid_base64(buffer.as_bytes()) {
            return Err(SerializedPayloadMutationError::Capacity);
        }
        metadata_bytes = metadata_bytes
            .checked_add(name.len())
            .and_then(|bytes| bytes.checked_add(mime.len()))
            .ok_or(SerializedPayloadMutationError::Capacity)?;
        base64_bytes = base64_bytes
            .checked_add(buffer.len())
            .ok_or(SerializedPayloadMutationError::Capacity)?;
        if metadata_bytes > METADATA_BYTES || base64_bytes > BASE64_BYTES {
            return Err(SerializedPayloadMutationError::Capacity);
        }
    }
    Ok(())
}

fn field<'a>(
    fields: &'a [Value],
    index: usize,
    expected: &str,
) -> Result<&'a Value, SerializedPayloadMutationError> {
    let field = exact_object(&fields[index], &["k", "v"])
        .ok_or(SerializedPayloadMutationError::Protocol("field shape"))?;
    if field["k"].as_str() != Some(expected) {
        return Err(SerializedPayloadMutationError::Protocol("field order"));
    }
    Ok(&field["v"])
}

fn validate_modified(value: &Value) -> Result<(), SerializedPayloadMutationError> {
    if value.is_number() {
        return Ok(());
    }
    let special = exact_object(value, &["v"])
        .and_then(|object| object["v"].as_str())
        .filter(|value| {
            matches!(
                *value,
                "undefined" | "null" | "NaN" | "Infinity" | "-Infinity" | "-0"
            )
        });
    if special.is_some() {
        Ok(())
    } else {
        Err(SerializedPayloadMutationError::Protocol(
            "lastModifiedMs value",
        ))
    }
}

fn valid_base64(bytes: &[u8]) -> bool {
    if !bytes.len().is_multiple_of(4) {
        return false;
    }
    let padding = bytes.iter().rev().take_while(|byte| **byte == b'=').count();
    if padding > 2 {
        return false;
    }
    let data = bytes.len().saturating_sub(padding);
    bytes[..data]
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'+' | b'/'))
        && bytes[data..].iter().all(|byte| *byte == b'=')
        && !bytes[..data].contains(&b'=')
}

#[cfg(test)]
mod tests;
