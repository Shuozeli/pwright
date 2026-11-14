//! Bounded Python overload conversion for the opt-in Page.select_option bridge.
//! Matching, retries, events, and selected-value ordering remain in Rust host.

use pwright_node_host::HostError;
use pwright_node_host::locator::action::{
    MAX_OPTION_BYTES, MAX_OPTIONS, MAX_VALUE_BYTES, SelectOptionValue,
};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyInt, PyList, PyString, PyStringMethods};

use crate::{binding_error, bounded_string};

#[cfg(feature = "handles")]
fn option_index(value: pwright_node_host::evaluation::EvaluationValue) -> Result<i32, HostError> {
    use pwright_node_host::evaluation::EvaluationValue;

    match value {
        EvaluationValue::String(index) if index == "wrong-select" => Err(
            HostError::InvalidArgument("option handle does not belong to target select".into()),
        ),
        EvaluationValue::String(index) => index
            .parse::<i32>()
            .ok()
            .filter(|index| *index >= 0)
            .ok_or_else(|| {
                HostError::InvalidArgument("option handle is not attached to a select".into())
            }),
        _ => Err(HostError::Backend(
            "option handle evaluation returned a non-string index".into(),
        )),
    }
}

#[cfg(feature = "handles")]
pub(crate) async fn option_handle_index(
    host: &pwright_node_host::HostSession,
    locator: &pwright_node_host::locator::HostLocator,
    handle: &pwright_node_host::handles::PendingHandle,
    timeout_ms: u32,
) -> Result<i32, HostError> {
    // Compatibility slice: this projects a retained option into a positional
    // descriptor. The target identity guard rejects handles from another
    // <select>, but the later action can still race with DOM reordering.
    use std::sync::Arc;

    use pwright_node_host::evaluation::EvaluationValue;
    use pwright_node_host::handles::{HandleArgumentSlot, OwnedHandleArgument};

    let target = Arc::new(locator.element_handle(timeout_ms).await?);
    let argument = OwnedHandleArgument::from_slots(
        EvaluationValue::Undefined,
        vec![HandleArgumentSlot::new(0, target)],
    )?;
    let result = host
        .handle_evaluate_program_owned(
            handle,
            "(option, target) => { const select = target?.tagName === 'LABEL' ? target.control : target; if (option?.tagName !== 'OPTION' || !option.isConnected) return 'invalid-option'; if (option.closest('select') !== select) return 'wrong-select'; return String(option.index); }".into(),
            true,
            argument,
        )
        .await?;
    option_index(result)
}

fn invalid(message: impl Into<String>) -> PyErr {
    binding_error(HostError::InvalidArgument(message.into()))
}

fn descriptor_text(value: &Bound<'_, PyAny>, field: &str) -> PyResult<String> {
    let value = value
        .cast::<PyString>()
        .map_err(|_| invalid(format!("{field}: expected string, got object")))?;
    bounded_string(&value.to_cow()?, MAX_OPTION_BYTES, field)
}

fn descriptor(value: &Bound<'_, PyAny>) -> PyResult<SelectOptionValue> {
    if value.is_instance_of::<PyString>() {
        return Ok(SelectOptionValue::value_or_label(descriptor_text(
            value,
            "select option value",
        )?));
    }
    let fields = value
        .cast::<PyDict>()
        .map_err(|_| invalid("expected string, got object"))?;
    if fields.len() > 3 {
        return Err(invalid("select option descriptor has too many fields"));
    }
    let mut output = SelectOptionValue::new();
    for (key, value) in fields.iter() {
        let key = key
            .cast::<PyString>()
            .map_err(|_| invalid("select option descriptor key must be a string"))?;
        match key.to_cow()?.as_ref() {
            "value" => output.value = Some(descriptor_text(&value, "select option value")?),
            "label" => output.label = Some(descriptor_text(&value, "select option label")?),
            "index" => {
                if value.is_instance_of::<PyBool>() || !value.is_instance_of::<PyInt>() {
                    return Err(invalid("select option index must be an integer"));
                }
                output.index = Some(
                    value
                        .extract::<i32>()
                        .map_err(|_| invalid("select option index exceeds int32"))?,
                );
            }
            _ => return Err(invalid("unknown select option descriptor field")),
        }
    }
    Ok(output)
}

pub(crate) fn descriptors(values: &Bound<'_, PyList>) -> PyResult<Vec<SelectOptionValue>> {
    if values.len() > MAX_OPTIONS {
        return Err(invalid("selectOption exceeds 1024 descriptors"));
    }
    let mut output = Vec::with_capacity(values.len());
    let mut string_bytes = 0usize;
    for value in values.iter() {
        let item = descriptor(&value)?;
        for text in [
            item.value_or_label.as_ref(),
            item.value.as_ref(),
            item.label.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            string_bytes = string_bytes.saturating_add(text.len());
            if string_bytes > MAX_VALUE_BYTES {
                return Err(invalid("selectOption strings exceed 1 MiB"));
            }
        }
        output.push(item);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_python(test: impl for<'py> FnOnce(Python<'py>)) {
        Python::initialize();
        Python::attach(test);
    }

    #[test]
    fn parses_strings_and_disjunctive_field_descriptors() {
        with_python(|py| {
            // Arrange
            let by_value = PyDict::new(py);
            by_value.set_item("value", "blue").unwrap();
            let by_label = PyDict::new(py);
            by_label.set_item("label", "Green").unwrap();
            let by_index = PyDict::new(py);
            by_index.set_item("index", 4).unwrap();
            let values = PyList::empty(py);
            values.append("red").unwrap();
            values.append(by_value).unwrap();
            values.append(by_label).unwrap();
            values.append(by_index).unwrap();

            // Act
            let parsed = descriptors(&values).unwrap();

            // Assert
            assert_eq!(
                parsed,
                vec![
                    SelectOptionValue::value_or_label("red"),
                    SelectOptionValue::new().with_value("blue"),
                    SelectOptionValue::new().with_label("Green"),
                    SelectOptionValue::new().with_index(4),
                ]
            );
        });
    }

    #[test]
    fn empty_list_means_deselect_all() {
        with_python(|py| {
            // Arrange
            let values = PyList::empty(py);

            // Act
            let parsed = descriptors(&values).unwrap();

            // Assert
            assert!(parsed.is_empty());
        });
    }

    #[test]
    fn null_list_item_is_rejected_before_browser_work() {
        with_python(|py| {
            // Arrange
            let values = PyList::empty(py);
            values.append("blue").unwrap();
            values.append(py.None()).unwrap();

            // Act
            let error = descriptors(&values).unwrap_err();

            // Assert
            assert!(error.to_string().contains("expected string, got object"));
        });
    }

    #[test]
    fn descriptor_index_rejects_bool() {
        with_python(|py| {
            // Arrange
            let by_index = PyDict::new(py);
            by_index.set_item("index", true).unwrap();
            let values = PyList::empty(py);
            values.append(by_index).unwrap();

            // Act
            let error = descriptors(&values).unwrap_err();

            // Assert
            assert!(error.to_string().contains("index must be an integer"));
        });
    }

    #[test]
    fn empty_label_and_zero_index_are_present_descriptors() {
        with_python(|py| {
            // Arrange
            let by_label = PyDict::new(py);
            by_label.set_item("label", "").unwrap();
            let by_index = PyDict::new(py);
            by_index.set_item("index", 0).unwrap();
            let values = PyList::empty(py);
            values.append(by_label).unwrap();
            values.append(by_index).unwrap();

            // Act
            let parsed = descriptors(&values).unwrap();

            // Assert
            assert_eq!(parsed[0], SelectOptionValue::new().with_label(""));
            assert_eq!(parsed[1], SelectOptionValue::new().with_index(0));
        });
    }

    #[test]
    fn oversized_string_fails_before_locator_action() {
        with_python(|py| {
            // Arrange
            let values = PyList::empty(py);
            values.append("x".repeat(MAX_OPTION_BYTES + 1)).unwrap();

            // Act
            let error = descriptors(&values).unwrap_err();

            // Assert
            assert!(error.to_string().contains("exceeds 65536 UTF-8 bytes"));
        });
    }

    #[test]
    fn oversized_descriptor_count_fails_before_locator_action() {
        with_python(|py| {
            // Arrange
            let values = PyList::empty(py);
            for _ in 0..=MAX_OPTIONS {
                values.append("blue").unwrap();
            }

            // Act
            let error = descriptors(&values).unwrap_err();

            // Assert
            assert!(error.to_string().contains("exceeds 1024 descriptors"));
        });
    }

    #[cfg(feature = "handles")]
    #[test]
    fn other_select_option_handle_cannot_become_an_index_descriptor() {
        // Arrange
        let result = pwright_node_host::evaluation::EvaluationValue::String("wrong-select".into());

        // Act
        let error = option_index(result).unwrap_err();

        // Assert
        assert!(
            error
                .to_string()
                .contains("option handle does not belong to target select")
        );
    }

    #[cfg(feature = "handles")]
    #[test]
    fn attached_option_index_zero_remains_selectable() {
        // Arrange
        let result = pwright_node_host::evaluation::EvaluationValue::String("0".into());

        // Act
        let index = option_index(result).unwrap();

        // Assert
        assert_eq!(index, 0);
    }
}
