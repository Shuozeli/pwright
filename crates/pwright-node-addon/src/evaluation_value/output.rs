use std::{collections::BTreeMap, num::NonZeroU32, ptr};

use napi::{
    Env, JsValue, Unknown,
    bindgen_prelude::{FnArgs, FromNapiValue, Function, JsObjectValue, Null, Object, ToNapiValue},
    sys,
};
use pwright_node_host::evaluation::{self, EvaluationValue, TypedArrayKind};

use super::{global, invalid, js_value, native_kind, status};

pub struct EvaluationOutput(pub EvaluationValue);

impl ToNapiValue for EvaluationOutput {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        // The FFI ticket still owns its result allowance throughout this conversion.
        value
            .0
            .validate(evaluation::value_limits())
            .map_err(|error| invalid(error.to_string()))?;
        let env = Env::from_raw(env);
        let mut writer = Writer {
            env: &env,
            refs: BTreeMap::new(),
        };
        writer.value(value.0).map(|value| value.raw())
    }
}

struct Writer<'env> {
    env: &'env Env,
    refs: BTreeMap<NonZeroU32, Unknown<'env>>,
}

impl<'env> Writer<'env> {
    fn value(&mut self, value: EvaluationValue) -> napi::Result<Unknown<'env>> {
        match value {
            EvaluationValue::Undefined => js_value(self.env, ()),
            EvaluationValue::Null => js_value(self.env, Null),
            EvaluationValue::Bool(value) => js_value(self.env, value),
            EvaluationValue::Number(value) => js_value(self.env, value.get()),
            EvaluationValue::Nan => js_value(self.env, f64::NAN),
            EvaluationValue::Infinity => js_value(self.env, f64::INFINITY),
            EvaluationValue::NegativeInfinity => js_value(self.env, f64::NEG_INFINITY),
            EvaluationValue::NegativeZero => js_value(self.env, -0.0_f64),
            EvaluationValue::String(value) => js_value(self.env, value),
            EvaluationValue::Bigint(value) => {
                let constructor: Function<String, Unknown<'env>> =
                    global(self.env)?.get_named_property("BigInt")?;
                constructor.call(value)
            }
            EvaluationValue::Date(value) => self.construct("Date", value),
            EvaluationValue::Url(value) => self.construct("URL", value),
            EvaluationValue::Regexp { pattern, flags } => {
                let constructor: Function<FnArgs<(String, String)>> =
                    global(self.env)?.get_named_property("RegExp")?;
                constructor.new_instance((pattern, flags).into())
            }
            EvaluationValue::Error {
                name,
                message,
                stack,
            } => {
                let value = self.construct("Error", message)?;
                // SAFETY: Error construction returns an object in this callback scope.
                let mut object = unsafe { value.cast::<Object<'_>>()? };
                object.set_named_property("name", name)?;
                if let Some(stack) = stack {
                    object.set_named_property("stack", stack)?;
                }
                Ok(value)
            }
            EvaluationValue::TypedArray { kind, bytes } => self.typed_array(kind, &bytes),
            EvaluationValue::Array { id, items } => {
                let mut raw = ptr::null_mut();
                // SAFETY: the model has already admitted the entire array length.
                status(unsafe {
                    sys::napi_create_array_with_length(self.env.raw(), items.len(), &mut raw)
                })?;
                let mut object = Object::from_raw(self.env.raw(), raw);
                let value = object.to_unknown();
                if let Some(id) = id {
                    self.refs.insert(id, value);
                }
                for (index, item) in items.into_iter().enumerate() {
                    object.set_element(
                        u32::try_from(index).map_err(|_| invalid("array index overflow"))?,
                        self.value(item)?,
                    )?;
                }
                Ok(value)
            }
            EvaluationValue::Object { id, properties } => {
                let mut object = Object::new(self.env)?;
                let value = object.to_unknown();
                if let Some(id) = id {
                    self.refs.insert(id, value);
                }
                for property in properties {
                    let key = self.env.create_string(property.key)?;
                    object.set_property(key, self.value(property.value)?)?;
                }
                Ok(value)
            }
            EvaluationValue::Ref(id) => self
                .refs
                .get(&id)
                .copied()
                .ok_or_else(|| invalid("unknown evaluation result reference")),
        }
    }

    fn construct(&self, name: &str, argument: String) -> napi::Result<Unknown<'env>> {
        let constructor: Function<String> = global(self.env)?.get_named_property(name)?;
        constructor.new_instance(argument)
    }

    fn typed_array(&self, kind: TypedArrayKind, bytes: &[u8]) -> napi::Result<Unknown<'env>> {
        let mut data = ptr::null_mut();
        let mut buffer = ptr::null_mut();
        let mut array = ptr::null_mut();
        // SAFETY: Node allocates a fresh non-shared buffer of the validated length.
        status(unsafe {
            sys::napi_create_arraybuffer(self.env.raw(), bytes.len(), &mut data, &mut buffer)
        })?;
        if !bytes.is_empty() {
            if data.is_null() {
                return Err(invalid(
                    "Node could not allocate an evaluation result buffer",
                ));
            }
            unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), data.cast::<u8>(), bytes.len()) };
        }
        status(unsafe {
            sys::napi_create_typedarray(
                self.env.raw(),
                native_kind(kind),
                bytes.len() / kind.element_bytes(),
                buffer,
                0,
                &mut array,
            )
        })?;
        // SAFETY: array is the new scoped handle returned by Node above.
        unsafe { Unknown::from_napi_value(self.env.raw(), array) }
    }
}
