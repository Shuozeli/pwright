//! Bounded Python value conversion for Rust-owned browser evaluation.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use pwright_node_host::HostError;
use pwright_node_host::evaluation::{self, EvaluationProperty, EvaluationValue, TypedArrayKind};
#[cfg(feature = "handles")]
use pwright_node_host::handles::{HandleArgumentSlot, OwnedHandleArgument};
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{
    PyBool, PyDateTime, PyDict, PyFloat, PyInt, PyList, PyString, PyStringMethods, PyTuple,
};

use crate::binding_error;
#[cfg(feature = "handles")]
use crate::handles::NativeHandle;

fn invalid(message: impl Into<String>) -> PyErr {
    binding_error(HostError::InvalidArgument(message.into()))
}

fn unsupported(message: impl Into<String>) -> PyErr {
    binding_error(HostError::UnsupportedOperation(message.into()))
}

struct Reader {
    seen: BTreeMap<usize, NonZeroU32>,
    next_reference: u32,
    nodes: usize,
    entries: usize,
    string_bytes: usize,
    #[cfg(feature = "handles")]
    handle_slots: Option<Vec<HandleArgumentSlot>>,
}

impl Reader {
    fn new() -> Self {
        Self {
            seen: BTreeMap::new(),
            next_reference: 1,
            nodes: 0,
            entries: 0,
            string_bytes: 0,
            #[cfg(feature = "handles")]
            handle_slots: None,
        }
    }

    #[cfg(feature = "handles")]
    fn with_handles() -> Self {
        Self {
            handle_slots: Some(Vec::new()),
            ..Self::new()
        }
    }

    fn charge_node(&mut self, depth: usize) -> PyResult<()> {
        let limits = evaluation::value_limits();
        if depth > limits.max_depth() {
            return Err(invalid("evaluation argument depth exceeds limit"));
        }
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| invalid("evaluation argument node count overflow"))?;
        if self.nodes > limits.max_nodes() {
            return Err(invalid("evaluation argument node count exceeds limit"));
        }
        Ok(())
    }

    fn charge_entries(&mut self, count: usize) -> PyResult<()> {
        self.entries = self
            .entries
            .checked_add(count)
            .ok_or_else(|| invalid("evaluation argument entry count overflow"))?;
        if self.entries > evaluation::value_limits().max_entries() {
            return Err(invalid("evaluation argument entry count exceeds limit"));
        }
        Ok(())
    }

    fn string(&mut self, value: &Bound<'_, PyString>) -> PyResult<String> {
        let value = value
            .to_cow()
            .map_err(|_| unsupported("unpaired Unicode surrogate in evaluation value"))?;
        self.string_bytes = self
            .string_bytes
            .checked_add(value.len())
            .ok_or_else(|| invalid("evaluation argument string-byte count overflow"))?;
        if self.string_bytes > evaluation::value_limits().max_string_bytes() {
            return Err(invalid("evaluation argument string bytes exceed limit"));
        }
        Ok(value.into_owned())
    }

    fn reference(&mut self, value: &Bound<'_, PyAny>) -> PyResult<Option<NonZeroU32>> {
        let identity = value.as_ptr() as usize;
        if let Some(reference) = self.seen.get(&identity) {
            return Ok(Some(*reference));
        }
        let reference = NonZeroU32::new(self.next_reference)
            .ok_or_else(|| invalid("evaluation argument reference space exhausted"))?;
        self.next_reference = self
            .next_reference
            .checked_add(1)
            .ok_or_else(|| invalid("evaluation argument reference space exhausted"))?;
        self.seen.insert(identity, reference);
        Ok(None)
    }

    fn optional_exception_attribute(
        &mut self,
        value: &Bound<'_, PyAny>,
        name: &str,
    ) -> PyResult<Option<String>> {
        if !value.hasattr(name)? {
            return Ok(None);
        }
        let attribute = value.getattr(name)?;
        if attribute.is_none() {
            return Ok(None);
        }
        self.string(attribute.cast::<PyString>()?).map(Some)
    }

    fn exception(&mut self, value: &Bound<'_, PyAny>) -> PyResult<EvaluationValue> {
        let message = self.string(&value.str()?)?;
        let (name, stack) = if value.is_instance_of::<crate::Error>() {
            // Official Playwright callers may set `_name` directly. The public
            // Pwright property setter stores `_pwright_name` instead.
            let name = if let Some(name) = self.optional_exception_attribute(value, "_name")? {
                name
            } else {
                self.optional_exception_attribute(value, "_pwright_name")?
                    .unwrap_or_default()
            };
            let stack = if let Some(stack) = self.optional_exception_attribute(value, "_stack")? {
                stack
            } else {
                self.optional_exception_attribute(value, "stack")?
                    .unwrap_or_default()
            };
            (name, stack)
        } else {
            let type_name = value.get_type().getattr("__name__")?;
            let name = self.string(type_name.cast::<PyString>()?)?;
            let formatted = value
                .py()
                .import("traceback")?
                .getattr("format_exception")?
                .call1((value.get_type(), value, value.py().None()))?;
            let mut stack = String::new();
            for line in formatted.try_iter()? {
                stack.push_str(&self.string(line?.cast::<PyString>()?)?);
            }
            (name, stack)
        };
        Ok(EvaluationValue::Error {
            name,
            message,
            stack: Some(stack),
        })
    }

    fn value(&mut self, value: &Bound<'_, PyAny>, depth: usize) -> PyResult<EvaluationValue> {
        self.charge_node(depth)?;
        #[cfg(feature = "handles")]
        if self.handle_slots.is_some()
            && let Ok(handle) = value.extract::<PyRef<'_, NativeHandle>>()
        {
            let index = self.nodes - 1;
            let handle = handle.read_frame().map_err(binding_error)?;
            self.handle_slots
                .as_mut()
                .expect("Handle-aware reader owns its slot storage")
                .push(HandleArgumentSlot::new(index, handle));
            return Ok(EvaluationValue::Undefined);
        }
        if value.is_none() {
            return Ok(EvaluationValue::Null);
        }
        if value.is_instance_of::<PyBool>() {
            return value.extract().map(EvaluationValue::Bool);
        }
        if value.is_instance_of::<PyInt>() || value.is_instance_of::<PyFloat>() {
            let number = value.extract::<f64>()?;
            return Ok(if number.is_nan() {
                EvaluationValue::Nan
            } else if number == f64::INFINITY {
                EvaluationValue::Infinity
            } else if number == f64::NEG_INFINITY {
                EvaluationValue::NegativeInfinity
            } else if number == 0.0 && number.is_sign_negative() {
                EvaluationValue::NegativeZero
            } else {
                EvaluationValue::number(number).map_err(|error| invalid(error.to_string()))?
            });
        }
        if let Ok(value) = value.cast::<PyString>() {
            return self.string(value).map(EvaluationValue::String);
        }
        let py = value.py();
        if value.is_instance_of::<PyDateTime>() {
            let utc = py.import("datetime")?.getattr("timezone")?.getattr("utc")?;
            let iso = value
                .call_method1("astimezone", (utc,))?
                .call_method1("strftime", ("%Y-%m-%dT%H:%M:%S.%fZ",))?;
            return self
                .string(iso.cast::<PyString>()?)
                .map(EvaluationValue::Date);
        }
        if value.is_instance_of::<PyException>() {
            return self.exception(value);
        }
        if value.is_instance_of::<PyTuple>() {
            let urllib_parse = py.import("urllib.parse")?;
            if value.is_instance(&urllib_parse.getattr("ParseResult")?)? {
                let url = urllib_parse.getattr("urlunparse")?.call1((value,))?;
                return self
                    .string(url.cast::<PyString>()?)
                    .map(EvaluationValue::Url);
            }
        }
        if let Ok(value) = value.cast::<PyList>() {
            if let Some(reference) = self.reference(value.as_any())? {
                return Ok(EvaluationValue::Ref(reference));
            }
            self.charge_entries(value.len())?;
            let mut items = Vec::with_capacity(value.len());
            for item in value.iter() {
                items.push(self.value(&item, depth + 1)?);
            }
            let id = self.seen.get(&(value.as_ptr() as usize)).copied();
            return Ok(EvaluationValue::Array { id, items });
        }
        if let Ok(value) = value.cast::<PyTuple>() {
            if let Some(reference) = self.reference(value.as_any())? {
                return Ok(EvaluationValue::Ref(reference));
            }
            self.charge_entries(value.len())?;
            let mut items = Vec::with_capacity(value.len());
            for item in value.iter() {
                items.push(self.value(&item, depth + 1)?);
            }
            let id = self.seen.get(&(value.as_ptr() as usize)).copied();
            return Ok(EvaluationValue::Array { id, items });
        }
        if let Ok(value) = value.cast::<PyDict>() {
            if let Some(reference) = self.reference(value.as_any())? {
                return Ok(EvaluationValue::Ref(reference));
            }
            self.charge_entries(value.len())?;
            let mut properties = Vec::with_capacity(value.len());
            for (key, item) in value.iter() {
                let key = key
                    .cast::<PyString>()
                    .map_err(|_| invalid("evaluation argument dictionary keys must be strings"))?;
                let key = self.string(key)?;
                properties.push(EvaluationProperty::new(key, self.value(&item, depth + 1)?));
            }
            let id = self.seen.get(&(value.as_ptr() as usize)).copied();
            return Ok(EvaluationValue::Object { id, properties });
        }
        // Playwright Python serializes other values as JavaScript undefined.
        Ok(EvaluationValue::Undefined)
    }
}

pub fn from_python(value: Option<&Bound<'_, PyAny>>) -> PyResult<EvaluationValue> {
    let Some(value) = value else {
        return Ok(EvaluationValue::Null);
    };
    let mut reader = Reader::new();
    let value = reader.value(value, 0)?;
    value
        .validate(evaluation::value_limits())
        .map_err(|error| invalid(error.to_string()))?;
    Ok(value)
}

#[cfg(feature = "handles")]
pub fn from_python_owned(value: Option<&Bound<'_, PyAny>>) -> PyResult<OwnedHandleArgument> {
    let Some(value) = value else {
        return OwnedHandleArgument::from_slots(EvaluationValue::Null, Vec::new())
            .map_err(binding_error);
    };
    let mut reader = Reader::with_handles();
    let value = reader.value(value, 0)?;
    OwnedHandleArgument::from_slots(
        value,
        reader
            .handle_slots
            .take()
            .expect("Handle-aware reader owns its slot storage"),
    )
    .map_err(binding_error)
}

pub fn is_function_source(source: &str) -> bool {
    let source = source.trim();
    starts_with_function_keyword(source)
        || top_level_arrow_prefix(source).is_some_and(|prefix| {
            let prefix = prefix
                .strip_prefix("async")
                .and_then(|rest| {
                    rest.chars()
                        .next()
                        .filter(|character| character.is_whitespace())
                        .map(|_| rest.trim_start())
                })
                .unwrap_or(prefix);
            is_identifier(prefix)
                || is_wrapped(prefix, '(', ')')
                || is_wrapped(prefix, '[', ']')
                || is_wrapped(prefix, '{', '}')
        })
}

fn starts_with_function_keyword(source: &str) -> bool {
    let source = source
        .strip_prefix("async")
        .and_then(|rest| {
            rest.chars()
                .next()
                .filter(|character| character.is_whitespace())
                .map(|_| rest.trim_start())
        })
        .unwrap_or(source);
    source.strip_prefix("function").is_some_and(|rest| {
        rest.is_empty()
            || rest.chars().next().is_some_and(|character| {
                character.is_whitespace() || matches!(character, '(' | '*')
            })
    })
}

fn top_level_arrow_prefix(source: &str) -> Option<&str> {
    let bytes = source.as_bytes();
    let mut parentheses = 0_u32;
    let mut brackets = 0_u32;
    let mut braces = 0_u32;
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0;
    while index + 1 < bytes.len() {
        let byte = bytes[index];
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }
        match byte {
            b'\'' | b'"' | b'`' => quote = Some(byte),
            b'(' => parentheses = parentheses.saturating_add(1),
            b')' => parentheses = parentheses.saturating_sub(1),
            b'[' => brackets = brackets.saturating_add(1),
            b']' => brackets = brackets.saturating_sub(1),
            b'{' => braces = braces.saturating_add(1),
            b'}' => braces = braces.saturating_sub(1),
            b'=' if bytes[index + 1] == b'>'
                && parentheses == 0
                && brackets == 0
                && braces == 0 =>
            {
                return Some(source[..index].trim());
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn is_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character == '$' || character.is_alphabetic())
        && characters
            .all(|character| character == '_' || character == '$' || character.is_alphanumeric())
}

fn is_wrapped(value: &str, open: char, close: char) -> bool {
    value.starts_with(open) && value.ends_with(close)
}

struct Writer {
    references: BTreeMap<NonZeroU32, Py<PyAny>>,
}

impl Writer {
    fn new() -> Self {
        Self {
            references: BTreeMap::new(),
        }
    }

    fn typed_array(
        &self,
        py: Python<'_>,
        kind: TypedArrayKind,
        bytes: &[u8],
    ) -> PyResult<Py<PyAny>> {
        let elements = bytes.len() / kind.element_bytes();
        if elements > evaluation::value_limits().max_entries() {
            return Err(invalid(
                "evaluation typed array element count exceeds limit",
            ));
        }
        let output = PyList::empty(py);
        for chunk in bytes.chunks_exact(kind.element_bytes()) {
            match kind {
                TypedArrayKind::Int8 => output.append(i8::from_le_bytes([chunk[0]]))?,
                TypedArrayKind::Uint8 | TypedArrayKind::Uint8Clamped => output.append(chunk[0])?,
                TypedArrayKind::Int16 => {
                    output.append(i16::from_le_bytes(chunk.try_into().expect("aligned i16")))?
                }
                TypedArrayKind::Uint16 => {
                    output.append(u16::from_le_bytes(chunk.try_into().expect("aligned u16")))?
                }
                TypedArrayKind::Int32 => {
                    output.append(i32::from_le_bytes(chunk.try_into().expect("aligned i32")))?
                }
                TypedArrayKind::Uint32 => {
                    output.append(u32::from_le_bytes(chunk.try_into().expect("aligned u32")))?
                }
                TypedArrayKind::Float32 => {
                    output.append(f32::from_le_bytes(chunk.try_into().expect("aligned f32")))?
                }
                TypedArrayKind::Float64 => {
                    output.append(f64::from_le_bytes(chunk.try_into().expect("aligned f64")))?
                }
                TypedArrayKind::Bigint64 => {
                    output.append(i64::from_le_bytes(chunk.try_into().expect("aligned i64")))?
                }
                TypedArrayKind::Biguint64 => {
                    output.append(u64::from_le_bytes(chunk.try_into().expect("aligned u64")))?
                }
            }
        }
        Ok(output.into_any().unbind())
    }

    fn value(&mut self, py: Python<'_>, value: EvaluationValue) -> PyResult<Py<PyAny>> {
        match value {
            EvaluationValue::Undefined | EvaluationValue::Null => Ok(py.None()),
            EvaluationValue::Bool(value) => {
                Ok(value.into_pyobject(py)?.to_owned().into_any().unbind())
            }
            EvaluationValue::Number(value) => Ok(value
                .get()
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind()),
            EvaluationValue::Nan => Ok(f64::NAN.into_pyobject(py)?.to_owned().into_any().unbind()),
            EvaluationValue::Infinity => Ok(f64::INFINITY
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind()),
            EvaluationValue::NegativeInfinity => Ok(f64::NEG_INFINITY
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind()),
            EvaluationValue::NegativeZero => {
                Ok((-0.0_f64).into_pyobject(py)?.to_owned().into_any().unbind())
            }
            EvaluationValue::String(value) => Ok(value.into_pyobject(py)?.into_any().unbind()),
            EvaluationValue::Bigint(value) => Ok(py
                .import("builtins")?
                .getattr("int")?
                .call1((value,))?
                .unbind()),
            EvaluationValue::Date(value) => {
                let value = value
                    .strip_suffix('Z')
                    .map_or(value.clone(), |value| format!("{value}+00:00"));
                Ok(py
                    .import("datetime")?
                    .getattr("datetime")?
                    .getattr("fromisoformat")?
                    .call1((value,))?
                    .unbind())
            }
            EvaluationValue::Url(value) => Ok(py
                .import("urllib.parse")?
                .getattr("urlparse")?
                .call1((value,))?
                .unbind()),
            EvaluationValue::Array { id, items } => {
                let output = PyList::empty(py);
                let owned = output.clone().into_any().unbind();
                if let Some(id) = id {
                    self.references.insert(id, owned.clone_ref(py));
                }
                for item in items {
                    output.append(self.value(py, item)?)?;
                }
                Ok(owned)
            }
            EvaluationValue::Object { id, properties } => {
                let output = PyDict::new(py);
                let owned = output.clone().into_any().unbind();
                if let Some(id) = id {
                    self.references.insert(id, owned.clone_ref(py));
                }
                for property in properties {
                    output.set_item(property.key, self.value(py, property.value)?)?;
                }
                Ok(owned)
            }
            EvaluationValue::Ref(id) => self
                .references
                .get(&id)
                .map(|value| value.clone_ref(py))
                .ok_or_else(|| invalid("unknown evaluation result reference")),
            EvaluationValue::Error {
                name,
                message,
                stack,
            } => {
                let error = py.get_type::<crate::Error>().call1((message,))?;
                error.setattr("_pwright_name", name)?;
                error.setattr("_stack", stack.clone())?;
                if !error.hasattr("stack")? {
                    error.setattr("stack", stack)?;
                }
                Ok(error.unbind())
            }
            EvaluationValue::TypedArray { kind, bytes } => self.typed_array(py, kind, &bytes),
            EvaluationValue::Regexp { .. } => Err(unsupported(
                "this evaluation result type is not installed in the Python binding",
            )),
        }
    }
}

pub fn to_python(py: Python<'_>, value: EvaluationValue) -> PyResult<Py<PyAny>> {
    value
        .validate(evaluation::value_limits())
        .map_err(|error| invalid(error.to_string()))?;
    Writer::new().value(py, value)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "handles")]
    use super::from_python_owned;
    use super::{from_python, is_function_source, to_python};
    use crate::{Error, InvalidArgumentError};
    use pwright_node_host::evaluation::{self, EvaluationValue, TypedArrayKind};
    use pyo3::prelude::*;

    #[test]
    fn nested_arrow_expression_is_not_a_function_source() {
        // Arrange
        let source = "JSON.parse(localStorage['react-todos']).map(i => i.title)";

        // Act
        let is_function = is_function_source(source);

        // Assert
        assert!(!is_function);
    }

    #[test]
    fn top_level_arrow_is_a_function_source() {
        // Arrange
        let source = "async ({ value }) => value";

        // Act
        let is_function = is_function_source(source);

        // Assert
        assert!(is_function);
    }

    #[test]
    fn function_keyword_requires_a_token_boundary() {
        // Arrange
        let source = "functionality === 'enabled'";

        // Act
        let is_function = is_function_source(source);

        // Assert
        assert!(!is_function);
    }

    #[test]
    fn aware_datetime_argument_is_normalized_to_utc() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let date = py
                .import("datetime")
                .expect("datetime module")
                .getattr("datetime")
                .expect("datetime type")
                .call_method1("fromisoformat", ("2020-05-27T05:31:38.506+04:00",))
                .expect("aware date");

            // Act
            let serialized = from_python(Some(&date)).expect("date argument");

            // Assert
            assert_eq!(
                serialized,
                EvaluationValue::Date("2020-05-27T01:31:38.506000Z".into())
            );
        });
    }

    #[test]
    fn urllib_parse_result_argument_is_a_url_not_an_array() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let url = "https://user:pass@example.com/?foo=bar#hi";
            let parsed = py
                .import("urllib.parse")
                .expect("urllib.parse")
                .getattr("urlparse")
                .expect("urlparse")
                .call1((url,))
                .expect("parsed URL");

            // Act
            let serialized = from_python(Some(&parsed)).expect("URL argument");

            // Assert
            assert_eq!(serialized, EvaluationValue::Url(url.into()));
        });
    }

    #[cfg(feature = "handles")]
    #[test]
    fn handle_aware_reader_preserves_complex_url_argument() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let url =
                "https://user:password@www.contoso.com:80/Home/Index.htm?q1=v1&q2=v2#FragmentName";
            let parsed = py
                .import("urllib.parse")
                .expect("urllib.parse")
                .getattr("urlparse")
                .expect("urlparse")
                .call1((url,))
                .expect("parsed URL");

            // Act
            let serialized = from_python_owned(Some(&parsed)).expect("URL argument");

            // Assert
            assert!(!serialized.has_handles());
            assert_eq!(
                serialized.into_value().expect("plain URL value"),
                EvaluationValue::Url(url.into())
            );
        });
    }

    #[test]
    fn playwright_error_argument_preserves_message_name_and_stack() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let error = py
                .get_type::<Error>()
                .call1(("error message",))
                .expect("native Playwright error");
            error
                .setattr("_pwright_name", "foobar")
                .expect("custom name");
            error.setattr("_stack", "test stack").expect("custom stack");

            // Act
            let serialized = from_python(Some(&error)).expect("error argument");

            // Assert
            assert_eq!(
                serialized,
                EvaluationValue::Error {
                    name: "foobar".into(),
                    message: "error message".into(),
                    stack: Some("test stack".into()),
                }
            );
        });
    }

    #[test]
    fn playwright_error_argument_reads_official_private_name() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let error = py
                .get_type::<Error>()
                .call1(("error message",))
                .expect("native Playwright error");
            error.setattr("_name", "foobar").expect("official name");
            error
                .setattr("_stack", "test stack")
                .expect("official stack");

            // Act
            let serialized = from_python(Some(&error)).expect("error argument");

            // Assert
            assert_eq!(
                serialized,
                EvaluationValue::Error {
                    name: "foobar".into(),
                    message: "error message".into(),
                    stack: Some("test stack".into()),
                }
            );
        });
    }

    #[test]
    fn ordinary_exception_argument_uses_class_name_and_traceback() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let error = py
                .import("builtins")
                .expect("builtins")
                .getattr("Exception")
                .expect("Exception type")
                .call1(("error message",))
                .expect("ordinary exception");

            // Act
            let serialized = from_python(Some(&error)).expect("error argument");

            // Assert
            match serialized {
                EvaluationValue::Error {
                    name,
                    message,
                    stack: Some(stack),
                } => {
                    assert_eq!(name, "Exception");
                    assert_eq!(message, "error message");
                    assert!(stack.contains("Exception: error message"));
                }
                other => panic!("expected an exception argument, got {other:?}"),
            }
        });
    }

    #[test]
    fn javascript_error_result_is_a_python_playwright_error() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let result = EvaluationValue::Error {
                name: "foobar".into(),
                message: "error message".into(),
                stack: Some("innerFunction at test.js:1".into()),
            };

            // Act
            let converted = to_python(py, result).expect("error result");

            // Assert
            let converted = converted.bind(py);
            assert!(converted.is_instance_of::<Error>());
            assert_eq!(
                converted
                    .str()
                    .expect("message")
                    .extract::<String>()
                    .unwrap(),
                "error message"
            );
            assert_eq!(
                converted
                    .getattr("_pwright_name")
                    .expect("name")
                    .extract::<String>()
                    .unwrap(),
                "foobar"
            );
            assert_eq!(
                converted
                    .getattr("_stack")
                    .expect("stack")
                    .extract::<String>()
                    .unwrap(),
                "innerFunction at test.js:1"
            );
        });
    }

    macro_rules! typed_array_output_case {
        ($name:ident, $kind:expr, $bytes:expr, $output_type:ty, $expected:expr) => {
            #[test]
            fn $name() {
                Python::initialize();
                Python::attach(|py| {
                    // Arrange
                    let result = EvaluationValue::TypedArray {
                        kind: $kind,
                        bytes: $bytes,
                    };

                    // Act
                    let converted = to_python(py, result).expect("typed array result");

                    // Assert
                    assert_eq!(
                        converted
                            .bind(py)
                            .extract::<Vec<$output_type>>()
                            .expect("Python list"),
                        $expected
                    );
                });
            }
        };
    }

    typed_array_output_case!(int8_result, TypedArrayKind::Int8, vec![255], i64, vec![-1]);
    typed_array_output_case!(
        uint8_result,
        TypedArrayKind::Uint8,
        vec![255],
        u64,
        vec![255]
    );
    typed_array_output_case!(
        uint8_clamped_result,
        TypedArrayKind::Uint8Clamped,
        vec![255],
        u64,
        vec![255]
    );
    typed_array_output_case!(
        int16_result,
        TypedArrayKind::Int16,
        (-123i16).to_le_bytes().to_vec(),
        i64,
        vec![-123]
    );
    typed_array_output_case!(
        uint16_result,
        TypedArrayKind::Uint16,
        65_535u16.to_le_bytes().to_vec(),
        u64,
        vec![65_535]
    );
    typed_array_output_case!(
        int32_result,
        TypedArrayKind::Int32,
        (-12_345i32).to_le_bytes().to_vec(),
        i64,
        vec![-12_345]
    );
    typed_array_output_case!(
        uint32_result,
        TypedArrayKind::Uint32,
        4_000_000_000u32.to_le_bytes().to_vec(),
        u64,
        vec![4_000_000_000]
    );
    typed_array_output_case!(
        float32_result,
        TypedArrayKind::Float32,
        1.5f32.to_le_bytes().to_vec(),
        f64,
        vec![1.5]
    );
    typed_array_output_case!(
        float64_result,
        TypedArrayKind::Float64,
        2.5f64.to_le_bytes().to_vec(),
        f64,
        vec![2.5]
    );
    typed_array_output_case!(
        bigint64_result,
        TypedArrayKind::Bigint64,
        (-9i64).to_le_bytes().to_vec(),
        i64,
        vec![-9]
    );
    typed_array_output_case!(
        biguint64_result,
        TypedArrayKind::Biguint64,
        u64::MAX.to_le_bytes().to_vec(),
        u64,
        vec![u64::MAX]
    );

    #[test]
    fn typed_array_python_list_respects_entry_budget() {
        Python::initialize();
        Python::attach(|py| {
            // Arrange
            let result = EvaluationValue::TypedArray {
                kind: TypedArrayKind::Uint8,
                bytes: vec![0; evaluation::MAX_ENTRIES + 1],
            };

            // Act
            let error = to_python(py, result).expect_err("list expansion must be bounded");

            // Assert
            assert!(error.is_instance_of::<InvalidArgumentError>(py));
        });
    }
}
