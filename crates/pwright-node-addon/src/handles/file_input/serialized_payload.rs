//! Synchronous import of the package-owned serialized graph. The graph was
//! produced immediately beforehand and is not exposed to user code.

use crate::{binding_error, evaluation_value::status};
use napi::{
    Env, JsString, JsValue, Unknown, ValueType,
    bindgen_prelude::{JsObjectValue, Object},
    sys,
};
use pwright_node_host::{
    HostError,
    handles::{
        RetainedSerializedPayload, SerializedPayloadAdmission, SerializedPayloadMetrics,
        serialized_payload_limits,
    },
};
use serde_json::{Map, Number, Value};
use std::ptr;

enum Slot {
    Value(Value),
    Omit,
}

pub(super) fn read(
    env: &Env,
    value: Unknown<'_>,
    admission: SerializedPayloadAdmission,
) -> napi::Result<RetainedSerializedPayload> {
    let mut reader = Reader {
        env,
        depth: 0,
        nodes: 0,
        entries: 0,
        string_bytes: 0,
        objects: Vec::new(),
    };
    let Slot::Value(graph) = reader.value(value, 0)? else {
        return Err(binding_error(HostError::InvalidArgument(
            "serialized file-input payload root is not JSON-compatible".into(),
        )));
    };
    admission
        .finish(
            graph,
            SerializedPayloadMetrics {
                depth: reader.depth,
                nodes: reader.nodes,
                entries: reader.entries,
                string_bytes: reader.string_bytes,
            },
        )
        .map_err(binding_error)
}

struct Reader<'env> {
    env: &'env Env,
    depth: usize,
    nodes: usize,
    entries: usize,
    string_bytes: usize,
    objects: Vec<Unknown<'env>>,
}

impl<'env> Reader<'env> {
    fn claim(current: &mut usize, amount: usize, limit: usize, label: &str) -> napi::Result<()> {
        *current = current
            .checked_add(amount)
            .filter(|total| *total <= limit)
            .ok_or_else(|| {
                binding_error(HostError::ResourceExhausted(format!(
                    "serialized file-input payload {label} limit exceeded"
                )))
            })?;
        Ok(())
    }

    fn string(&mut self, value: JsString<'_>) -> napi::Result<String> {
        let bytes = value.utf8_len()?;
        Self::claim(
            &mut self.string_bytes,
            bytes,
            serialized_payload_limits().max_string_bytes,
            "string bytes",
        )?;
        value.into_utf8()?.into_owned()
    }

    fn value(&mut self, value: Unknown<'env>, depth: usize) -> napi::Result<Slot> {
        let limits = serialized_payload_limits();
        if depth > limits.max_depth {
            return Err(binding_error(HostError::ResourceExhausted(
                "serialized file-input payload depth limit exceeded".into(),
            )));
        }
        self.depth = self.depth.max(depth);
        Self::claim(&mut self.nodes, 1, limits.max_nodes, "node count")?;
        Ok(match value.get_type()? {
            ValueType::Undefined | ValueType::Function | ValueType::Symbol => Slot::Omit,
            ValueType::Null => Slot::Value(Value::Null),
            ValueType::Boolean => Slot::Value(Value::Bool(unsafe { value.cast::<bool>()? })),
            ValueType::Number => {
                let number = unsafe { value.cast::<f64>()? };
                Slot::Value(
                    Number::from_f64(number)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                )
            }
            ValueType::String => Slot::Value(Value::String(
                self.string(unsafe { value.cast::<JsString<'_>>()? })?,
            )),
            ValueType::BigInt => {
                return Err(binding_error(HostError::InvalidArgument(
                    "serialized file-input payload contains a raw BigInt".into(),
                )));
            }
            ValueType::Object => self.object(value, depth)?,
            ValueType::External | ValueType::Unknown => {
                return Err(binding_error(HostError::InvalidArgument(
                    "serialized file-input payload contains an unsupported value".into(),
                )));
            }
        })
    }

    fn object(&mut self, value: Unknown<'env>, depth: usize) -> napi::Result<Slot> {
        for previous in &self.objects {
            if self.env.strict_equals(*previous, value)? {
                return Err(binding_error(HostError::InvalidArgument(
                    "serialized file-input payload graph contains a native cycle".into(),
                )));
            }
        }
        self.objects.push(value);
        let object = unsafe { value.cast::<Object<'_>>()? };
        let result = if value.is_array()? {
            let length = object.get_array_length()? as usize;
            Self::claim(
                &mut self.entries,
                length,
                serialized_payload_limits().max_entries,
                "entry count",
            )?;
            let mut items = Vec::new();
            items
                .try_reserve_exact(length)
                .map_err(|error| binding_error(HostError::ResourceExhausted(error.to_string())))?;
            for index in 0..length {
                items.push(
                    match self.value(object.get_element(index as u32)?, depth + 1)? {
                        Slot::Value(value) => value,
                        // Match JSON arrays: unsupported/undefined entries become null.
                        Slot::Omit => Value::Null,
                    },
                );
            }
            Slot::Value(Value::Array(items))
        } else {
            let keys = own_enumerable_keys(self.env, &object)?;
            let length = keys.get_array_length()? as usize;
            Self::claim(
                &mut self.entries,
                length,
                serialized_payload_limits().max_entries,
                "entry count",
            )?;
            let mut properties = Map::new();
            for index in 0..length {
                let key: JsString<'_> = keys.get_element(index as u32)?;
                let property = object.get_property(key)?;
                let key = self.string(key)?;
                if let Slot::Value(property) = self.value(property, depth + 1)? {
                    properties.insert(key, property);
                }
            }
            Slot::Value(Value::Object(properties))
        };
        self.objects.pop();
        Ok(result)
    }
}

fn own_enumerable_keys<'env>(env: &'env Env, object: &Object<'env>) -> napi::Result<Object<'env>> {
    let mut keys = ptr::null_mut();
    status(unsafe {
        sys::napi_get_all_property_names(
            env.raw(),
            object.raw(),
            sys::KeyCollectionMode::own_only,
            sys::KeyFilter::enumerable | sys::KeyFilter::skip_symbols,
            sys::KeyConversion::numbers_to_strings,
            &mut keys,
        )
    })?;
    Ok(Object::from_raw(env.raw(), keys))
}
