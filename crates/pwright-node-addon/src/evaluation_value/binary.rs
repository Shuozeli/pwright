use std::ptr;

use napi::{Env, JsValue, Unknown, bindgen_prelude::Function, sys};
use pwright_node_host::evaluation::{BINARY_BACKING_BYTES, EvaluationValue};

use super::{invalid, status};
use crate::node_reference::NodeReference;

pub struct BinaryView {
    view: NodeReference,
    length: usize,
}

impl BinaryView {
    pub fn capture(
        env: &Env,
        buffer: Unknown<'_>,
        offset: usize,
        length: usize,
        maximum: Function<'_, (), f64>,
        retained_backing: &mut usize,
    ) -> napi::Result<Self> {
        let bytes = maximum.apply(buffer, ())?;
        if !bytes.is_finite()
            || bytes < 0.0
            || bytes.fract() != 0.0
            || bytes > BINARY_BACKING_BYTES as f64
        {
            return Err(invalid("evaluation backing capacity limit exceeded"));
        }
        let bytes = bytes as usize;
        let total = retained_backing
            .checked_add(bytes)
            .filter(|total| *total <= BINARY_BACKING_BYTES)
            .ok_or_else(|| invalid("evaluation aggregate backing capacity limit exceeded"))?;
        let mut view = ptr::null_mut();
        // SAFETY: a checked ordinary ArrayBuffer, byte offset/range obtained
        // from Node, and no pointer is dereferenced or retained here.
        status(unsafe {
            sys::napi_create_typedarray(
                env.raw(),
                sys::TypedarrayType::uint8_array,
                length,
                buffer.raw(),
                offset,
                &mut view,
            )
        })?;
        let view = unsafe { Unknown::from_raw_unchecked(env.raw(), view) };
        let reference = NodeReference::new(env, view)?;
        *retained_backing = total;
        Ok(Self {
            view: reference,
            length,
        })
    }

    pub fn copy(&self, env: &Env) -> napi::Result<Vec<u8>> {
        let value = self.view.get(env)?;
        let mut kind = 0;
        let mut length = 0;
        let mut data = ptr::null_mut();
        let mut buffer = ptr::null_mut();
        let mut offset = 0;
        // SAFETY: our own fixed Uint8Array view, read only on the Node thread.
        status(unsafe {
            sys::napi_get_typedarray_info(
                env.raw(),
                value.raw(),
                &mut kind,
                &mut length,
                &mut data,
                &mut buffer,
                &mut offset,
            )
        })?;
        let mut detached = false;
        status(unsafe { sys::napi_is_detached_arraybuffer(env.raw(), buffer, &mut detached) })?;
        if detached {
            return Err(invalid(
                "Cannot perform Construct on a detached ArrayBuffer",
            ));
        }
        if kind != sys::TypedarrayType::uint8_array || length > self.length {
            return Err(invalid(
                "evaluation binary view violated its admitted bounds",
            ));
        }
        if length == 0 {
            return Ok(Vec::new());
        }
        if data.is_null() {
            return Err(invalid("evaluation binary view is unavailable"));
        }
        // SAFETY: the fixed view is admitted, non-shared and non-detached. No
        // JS call occurs between the metadata read above and this owned copy.
        Ok(unsafe { std::slice::from_raw_parts(data.cast::<u8>(), length) }.to_vec())
    }
}

pub fn materialize(
    value: &mut EvaluationValue,
    views: &mut std::slice::Iter<'_, BinaryView>,
    env: &Env,
) -> napi::Result<()> {
    match value {
        EvaluationValue::TypedArray { bytes, .. } => {
            *bytes = views
                .next()
                .ok_or_else(|| invalid("missing binary conversion slot"))?
                .copy(env)?;
        }
        EvaluationValue::Array { items, .. } => {
            for item in items {
                materialize(item, views, env)?;
            }
        }
        EvaluationValue::Object { properties, .. } => {
            for property in properties {
                materialize(&mut property.value, views, env)?;
            }
        }
        EvaluationValue::Undefined
        | EvaluationValue::Null
        | EvaluationValue::Bool(_)
        | EvaluationValue::Number(_)
        | EvaluationValue::Nan
        | EvaluationValue::Infinity
        | EvaluationValue::NegativeInfinity
        | EvaluationValue::NegativeZero
        | EvaluationValue::String(_)
        | EvaluationValue::Bigint(_)
        | EvaluationValue::Date(_)
        | EvaluationValue::Url(_)
        | EvaluationValue::Regexp { .. }
        | EvaluationValue::Error { .. }
        | EvaluationValue::Ref(_) => {}
    }
    Ok(())
}
