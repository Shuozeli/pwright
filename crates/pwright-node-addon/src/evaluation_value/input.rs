use std::num::NonZeroU32;
use std::ptr;

use napi::{
    Env, JsString, JsValue, Unknown, ValueType,
    bindgen_prelude::{Function, JsObjectValue, Object},
    sys,
};
use pwright_node_host::evaluation::{self, EvaluationProperty, EvaluationValue, TypedArrayKind};

use super::binary::{self, BinaryView};
use super::{global, invalid, status, string, typed_kind, typed_name, unsupported};

pub struct PreparedInput {
    value: EvaluationValue,
    views: Vec<BinaryView>,
    pub backing_bytes: usize,
    #[cfg(feature = "handles")]
    handles: Vec<pwright_node_host::handles::HandleArgumentSlot>,
    #[cfg(feature = "handles")]
    disposed_handle: bool,
}

impl PreparedInput {
    pub fn binary_references(&self) -> usize {
        self.views.len()
    }

    pub fn materialize(mut self, env: &Env) -> napi::Result<EvaluationValue> {
        #[cfg(feature = "handles")]
        if self.has_handles() {
            return Err(unsupported(
                "Handle references are not implemented for this evaluation target",
            ));
        }
        self.copy_binary(env)?;
        Ok(self.value)
    }

    #[cfg(feature = "handles")]
    pub fn has_handles(&self) -> bool {
        !self.handles.is_empty() || self.disposed_handle
    }

    #[cfg(feature = "handles")]
    pub fn materialize_owned(
        mut self,
        env: &Env,
    ) -> napi::Result<pwright_node_host::handles::OwnedHandleArgument> {
        self.copy_binary(env)?;
        if self.disposed_handle {
            return Err(crate::binding_error(pwright_node_host::HostError::Backend(
                "JSHandle is disposed".into(),
            )));
        }
        pwright_node_host::handles::OwnedHandleArgument::from_slots(self.value, self.handles)
            .map_err(crate::binding_error)
    }

    fn copy_binary(&mut self, env: &Env) -> napi::Result<()> {
        let mut views = self.views.iter();
        binary::materialize(&mut self.value, &mut views, env)?;
        if views.next().is_some() {
            return Err(invalid("unused binary conversion slot"));
        }
        self.value
            .validate(evaluation::input_value_limits())
            .map_err(|error| invalid(error.to_string()))?;
        Ok(())
    }
}

pub fn read<'env>(
    env: &'env Env,
    value: Unknown<'env>,
    maximum: Function<'env, (), f64>,
) -> napi::Result<PreparedInput> {
    let mut reader = Reader {
        env,
        nodes: 0,
        entries: 0,
        strings: 0,
        binary: 0,
        refs: Vec::new(),
        views: Vec::new(),
        backing: 0,
        maximum,
        #[cfg(feature = "handles")]
        handles: Vec::new(),
        #[cfg(feature = "handles")]
        disposed_handle: false,
    };
    let result = reader.value(value, 0, &mut Vec::new())?;
    result
        .validate(evaluation::input_value_limits())
        .map_err(|error| invalid(error.to_string()))?;
    Ok(PreparedInput {
        value: result,
        views: reader.views,
        backing_bytes: reader.backing,
        #[cfg(feature = "handles")]
        handles: reader.handles,
        #[cfg(feature = "handles")]
        disposed_handle: reader.disposed_handle,
    })
}

struct Reader<'env> {
    env: &'env Env,
    nodes: usize,
    entries: usize,
    strings: usize,
    binary: usize,
    // Callback-scoped handles only: no persistent JS reference crosses an await.
    refs: Vec<Unknown<'env>>,
    views: Vec<BinaryView>,
    backing: usize,
    maximum: Function<'env, (), f64>,
    #[cfg(feature = "handles")]
    handles: Vec<pwright_node_host::handles::HandleArgumentSlot>,
    #[cfg(feature = "handles")]
    disposed_handle: bool,
}

impl<'env> Reader<'env> {
    fn text(&mut self, value: JsString<'_>) -> napi::Result<String> {
        let bytes = value.utf8_len()?;
        claim(
            &mut self.strings,
            bytes,
            evaluation::INPUT_STRING_BYTES,
            "string bytes",
        )?;
        string(value, bytes)
    }

    fn property_string(&mut self, object: &Object<'_>, key: &str) -> napi::Result<String> {
        self.text(object.get_named_property(key)?)
    }

    fn value(
        &mut self,
        value: Unknown<'env>,
        depth: usize,
        path: &mut Vec<DiagnosticAccessor>,
    ) -> napi::Result<EvaluationValue> {
        if depth > evaluation::MAX_DEPTH {
            return Err(invalid("evaluation depth limit exceeded"));
        }
        claim(&mut self.nodes, 1, evaluation::MAX_NODES, "nodes")?;
        Ok(match value.get_type()? {
            ValueType::Undefined | ValueType::Symbol => EvaluationValue::Undefined,
            ValueType::Null => EvaluationValue::Null,
            ValueType::Boolean => EvaluationValue::Bool(unsafe { value.cast::<bool>()? }),
            ValueType::Number => {
                // SAFETY: value's number type was checked in this match arm.
                let number = unsafe { value.cast::<f64>()? };
                if number.is_nan() {
                    EvaluationValue::Nan
                } else if number == f64::INFINITY {
                    EvaluationValue::Infinity
                } else if number == f64::NEG_INFINITY {
                    EvaluationValue::NegativeInfinity
                } else if number == 0.0 && number.is_sign_negative() {
                    EvaluationValue::NegativeZero
                } else {
                    EvaluationValue::number(number).map_err(|error| invalid(error.to_string()))?
                }
            }
            ValueType::String => self
                .text(unsafe { value.cast::<JsString<'_>>()? })
                .map(EvaluationValue::String)?,
            ValueType::BigInt => self
                .text(value.coerce_to_string()?)
                .map(EvaluationValue::Bigint)?,
            ValueType::Object => self.object(value, depth, path)?,
            ValueType::Function => {
                let display = string(value.coerce_to_string()?, evaluation::SOURCE_BYTES)?;
                return Err(invalid(format!(
                    "Attempting to serialize unexpected value{}: {display}",
                    diagnostic_position(path)
                )));
            }
            ValueType::External | ValueType::Unknown => {
                return Err(unsupported("external evaluation argument"));
            }
        })
    }

    fn object(
        &mut self,
        value: Unknown<'env>,
        depth: usize,
        path: &mut Vec<DiagnosticAccessor>,
    ) -> napi::Result<EvaluationValue> {
        #[cfg(feature = "handles")]
        if let Some(reference) = crate::handles::read_argument(self.env, value)? {
            match reference {
                crate::handles::ArgumentReference::Live(handle) => {
                    self.handles
                        .push(pwright_node_host::handles::HandleArgumentSlot::new(
                            self.nodes - 1,
                            handle,
                        ))
                }
                crate::handles::ArgumentReference::Disposed => self.disposed_handle = true,
            }
            return Ok(EvaluationValue::Undefined);
        }
        // SAFETY: this method is only called for a checked Object value.
        let object = unsafe { value.cast::<Object<'_>>()? };
        if value.is_error()? {
            let name = self.property_string(&object, "name")?;
            let message = self.property_string(&object, "message")?;
            let stack: Unknown<'_> = object.get_named_property("stack")?;
            let stack = if stack.coerce_to_bool()? {
                if stack.get_type()? != ValueType::String {
                    return Err(invalid("Error stack must be a string"));
                }
                Some(self.text(unsafe { stack.cast::<JsString<'_>>()? })?)
            } else {
                Some(String::new())
            };
            return Ok(EvaluationValue::Error {
                name,
                message,
                stack,
            });
        }
        if value.is_date()? {
            let method: Function<(), Unknown<'env>> = object.get_named_property("toJSON")?;
            let text = method.apply(value, ())?;
            if text.get_type()? != ValueType::String {
                return Err(unsupported("invalid Date evaluation argument"));
            }
            return self
                .text(unsafe { text.cast::<JsString<'_>>()? })
                .map(EvaluationValue::Date);
        }
        // These checks also recognize values created in another Node vm realm.
        let tag = self.object_tag(value)?;
        if tag == "[object PwrightJSHandle]" {
            return Err(unsupported(
                "Handle argument has no native owner available to this profile",
            ));
        }
        if tag == "[object URL]" {
            let method: Function<(), Unknown<'env>> = object.get_named_property("toJSON")?;
            let text = method.apply(value, ())?;
            if text.get_type()? != ValueType::String {
                return Err(invalid("URL toJSON must return a string"));
            }
            return self
                .text(unsafe { text.cast::<JsString<'_>>()? })
                .map(EvaluationValue::Url);
        }
        if tag == "[object RegExp]" {
            return Ok(EvaluationValue::Regexp {
                pattern: self.property_string(&object, "source")?,
                flags: self.property_string(&object, "flags")?,
            });
        }
        if value.is_typedarray()?
            && let Some(typed) = self.typed_array(value, &object)?
        {
            return Ok(typed);
        }
        for (index, previous) in self.refs.iter().enumerate() {
            if self.env.strict_equals(*previous, value)? {
                return Ok(EvaluationValue::Ref(reference_id(index)?));
            }
        }
        let id = reference_id(self.refs.len())?;
        self.refs.push(value);
        if value.is_array()? {
            let length = object.get_array_length()?;
            claim(
                &mut self.entries,
                length as usize,
                evaluation::MAX_ENTRIES,
                "entries",
            )?;
            let mut items = Vec::with_capacity(length as usize);
            for index in 0..length {
                path.push(DiagnosticAccessor::Index(index));
                let item = self.value(object.get_element(index)?, depth + 1, path);
                path.pop();
                items.push(item?);
            }
            return Ok(EvaluationValue::Array {
                id: Some(id),
                items,
            });
        }
        let keys = own_enumerable_keys(self.env, &object)?;
        let length = keys.get_array_length()?;
        claim(
            &mut self.entries,
            length as usize,
            evaluation::MAX_ENTRIES,
            "entries",
        )?;
        let mut properties = Vec::with_capacity(length as usize);
        for index in 0..length {
            let key: JsString<'_> = keys.get_element(index)?;
            let property = object.get_property(key)?;
            let key = self.text(key)?;
            path.push(DiagnosticAccessor::Property(key.clone()));
            let property = self.value(property, depth + 1, path);
            path.pop();
            properties.push(EvaluationProperty::new(key, property?));
        }
        Ok(EvaluationValue::Object {
            id: Some(id),
            properties,
        })
    }

    fn object_tag(&self, value: Unknown<'env>) -> napi::Result<String> {
        let constructor: Unknown<'_> = global(self.env)?.get_named_property("Object")?;
        let constructor = constructor.coerce_to_object()?;
        let prototype: Object<'_> = constructor.get_named_property("prototype")?;
        let method: Function<(), Unknown<'_>> = prototype.get_named_property("toString")?;
        let tag = method.apply(value, ())?;
        if tag.get_type()? != ValueType::String {
            return Err(invalid("object type tag is not a string"));
        }
        string(unsafe { tag.cast::<JsString<'_>>()? }, 128)
    }

    fn typed_array(
        &mut self,
        value: Unknown<'env>,
        object: &Object<'_>,
    ) -> napi::Result<Option<EvaluationValue>> {
        let mut native = 0;
        let mut length = 0;
        let mut data = ptr::null_mut();
        let mut buffer = ptr::null_mut();
        let mut offset = 0;
        // SAFETY: checked typed array and callback-scoped output pointers.
        status(unsafe {
            sys::napi_get_typedarray_info(
                self.env.raw(),
                value.raw(),
                &mut native,
                &mut length,
                &mut data,
                &mut buffer,
                &mut offset,
            )
        })?;
        let kind: TypedArrayKind = typed_kind(native)?;
        let constructor: Unknown<'_> = object.get_named_property("constructor")?;
        let standard: Unknown<'_> = global(self.env)?.get_named_property(typed_name(kind))?;
        if !self.env.strict_equals(constructor, standard)? {
            return Ok(None);
        }
        // Re-read after getters above: they may detach/resize the buffer.
        status(unsafe {
            sys::napi_get_typedarray_info(
                self.env.raw(),
                value.raw(),
                &mut native,
                &mut length,
                &mut data,
                &mut buffer,
                &mut offset,
            )
        })?;
        let mut ordinary = false;
        status(unsafe { sys::napi_is_arraybuffer(self.env.raw(), buffer, &mut ordinary) })?;
        if !ordinary {
            return Err(unsupported("SharedArrayBuffer-backed evaluation arguments"));
        }
        let size = length
            .checked_mul(kind.element_bytes())
            .ok_or_else(|| invalid("typed array size overflow"))?;
        claim(
            &mut self.binary,
            size,
            evaluation::BINARY_BYTES,
            "binary bytes",
        )?;
        let buffer = unsafe { Unknown::from_raw_unchecked(self.env.raw(), buffer) };
        let view = BinaryView::capture(
            self.env,
            buffer,
            offset,
            size,
            self.maximum,
            &mut self.backing,
        )?;
        self.views.push(view);
        // The owned graph is fixed now; only these admitted byte leaves are
        // populated later, before the graph can enter any Rust future.
        Ok(Some(EvaluationValue::TypedArray {
            kind,
            bytes: Vec::new(),
        }))
    }
}

enum DiagnosticAccessor {
    Property(String),
    Index(u32),
}

fn diagnostic_position(path: &[DiagnosticAccessor]) -> String {
    if path.is_empty() {
        return String::new();
    }
    let mut display = String::new();
    for accessor in path {
        let before = display.len();
        match accessor {
            DiagnosticAccessor::Property(key) => {
                if !display.is_empty() {
                    display.push('.');
                }
                display.push_str(key);
            }
            DiagnosticAccessor::Index(index) => {
                display.push('[');
                display.push_str(&index.to_string());
                display.push(']');
            }
        }
        if display.len() > evaluation::SOURCE_BYTES {
            display.truncate(before);
            display.push_str("...");
            break;
        }
    }
    format!(" at position \"{display}\"")
}

fn reference_id(index: usize) -> napi::Result<NonZeroU32> {
    u32::try_from(index + 1)
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or_else(|| invalid("evaluation reference space exhausted"))
}

fn claim(current: &mut usize, additional: usize, limit: usize, label: &str) -> napi::Result<()> {
    let total = current
        .checked_add(additional)
        .filter(|total| *total <= limit)
        .ok_or_else(|| invalid(format!("evaluation {label} limit exceeded")))?;
    *current = total;
    Ok(())
}

fn own_enumerable_keys<'env>(env: &'env Env, object: &Object<'env>) -> napi::Result<Object<'env>> {
    let mut keys = ptr::null_mut();
    // SAFETY: active env and object; flags match Object.keys (own enumerable strings).
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
