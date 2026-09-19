//! Bounded Node argument conversion only; no selector, DOM, input or retry logic.

use super::NativeLocator;
use crate::{binding_error, boundary};
use napi::{
    Env, JsString, JsValue, Unknown, ValueType,
    bindgen_prelude::{Either, JsObjectValue, Object, PromiseRaw},
};
use napi_derive::napi;
use pwright_node_host::{HostError, ffi_budget::FfiLane, locator::action::*};
use std::{num::NonZeroU8, time::Duration};

// Includes bounded owned input/result vectors plus overlapping UTF-16/UTF-8
// conversion and query copies. Shares the existing 64 MiB settlement ceiling.
const ACTION_BYTES: u64 = 6 * 1024 * 1024;

fn invalid(message: impl Into<String>) -> napi::Error {
    binding_error(HostError::InvalidArgument(message.into()))
}

fn text(value: JsString<'_>, maximum: usize) -> napi::Result<String> {
    if value.utf8_len()? > maximum {
        return Err(invalid(format!("action string exceeds {maximum} bytes")));
    }
    let length = value.utf16_len()?;
    let units = value.into_utf16()?;
    String::from_utf16(&units.as_slice()[..length]).map_err(|_| {
        binding_error(HostError::UnsupportedOperation(
            "unpaired UTF-16 in action input".into(),
        ))
    })
}

fn object(value: Unknown<'_>) -> napi::Result<Object<'_>> {
    if value.get_type()? != ValueType::Object {
        return Err(invalid("expected an action option object"));
    }
    // SAFETY: the value is a checked object in the active callback scope.
    unsafe { value.cast::<Object<'_>>() }
}

fn input_text(value: Option<Unknown<'_>>, maximum: usize) -> napi::Result<String> {
    let value = value.ok_or_else(|| invalid("action requires a string"))?;
    if value.get_type()? != ValueType::String {
        return Err(invalid("action requires a string"));
    }
    // SAFETY: the string type was checked; conversion finishes on this callback.
    text(unsafe { value.cast::<JsString<'_>>()? }, maximum)
}

fn integer(value: f64, minimum: f64, maximum: f64, field: &str) -> napi::Result<f64> {
    if !value.is_finite() || value.fract() != 0.0 || value < minimum || value > maximum {
        return Err(invalid(format!(
            "{field} must be an integer in [{minimum}, {maximum}]"
        )));
    }
    Ok(value)
}

fn delay(options: &Object<'_>) -> napi::Result<Option<Duration>> {
    options
        .get_named_property::<Option<f64>>("delay")?
        .map(|value| {
            if !value.is_finite() || value < 0.0 || value > f64::from(u32::MAX) {
                return Err(invalid(
                    "delay must be finite and between 0 and 4294967295 ms",
                ));
            }
            Ok(Duration::from_secs_f64(value / 1000.0))
        })
        .transpose()
}

fn position(options: &Object<'_>) -> napi::Result<Option<Position>> {
    options
        .get_named_property::<Option<Object<'_>>>("position")?
        .map(|point| {
            Position::new(
                point.get_named_property("x")?,
                point.get_named_property("y")?,
            )
            .map_err(|error| invalid(error.to_string()))
        })
        .transpose()
}

fn modifiers(options: &Object<'_>) -> napi::Result<Option<KeyboardModifiers>> {
    let Some(values) = options.get_named_property::<Option<Object<'_>>>("modifiers")? else {
        return Ok(None);
    };
    let length = values.get_array_length()?;
    if length > 5 {
        return Err(invalid("at most five keyboard modifiers are supported"));
    }
    let mut result = KeyboardModifiers::new();
    for index in 0..length {
        let name = text(values.get_element(index)?, 16)?;
        let modifier = match name.as_str() {
            "Alt" => KeyboardModifier::Alt,
            "Control" => KeyboardModifier::Control,
            "ControlOrMeta" => KeyboardModifier::ControlOrMeta,
            "Meta" => KeyboardModifier::Meta,
            "Shift" => KeyboardModifier::Shift,
            _ => return Err(invalid("unknown keyboard modifier")),
        };
        result = result.with(modifier);
    }
    Ok(Some(result))
}

fn click(options: &Object<'_>, double: bool) -> napi::Result<ClickOptionsV2> {
    let button = options
        .get_named_property::<Option<JsString<'_>>>("button")?
        .map(|name| {
            Ok(match text(name, 8)?.as_str() {
                "left" => MouseButton::Left,
                "right" => MouseButton::Right,
                "middle" => MouseButton::Middle,
                _ => return Err(invalid("unknown mouse button")),
            })
        })
        .transpose()?;
    let count = options
        .get_named_property::<Option<f64>>("clickCount")?
        .map(|value| {
            let value = integer(value, 1.0, 255.0, "clickCount")? as u8;
            NonZeroU8::new(value).ok_or_else(|| invalid("clickCount is zero"))
        })
        .transpose()?;
    Ok(ClickOptionsV2 {
        button,
        click_count: if double { NonZeroU8::new(2) } else { count },
        delay: delay(options)?,
        force: options.get_named_property("force")?,
        modifiers: modifiers(options)?,
        no_wait_after: if double {
            Some(true)
        } else {
            options.get_named_property("noWaitAfter")?
        },
        position: position(options)?,
        trial: options.get_named_property("trial")?,
        ..ClickOptionsV2::new()
    })
}

fn selection(value: Option<Unknown<'_>>) -> napi::Result<Vec<SelectOptionValue>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let (array, length) = if value.is_array()? {
        let array = object(value)?;
        let length = array.get_array_length()?;
        (Some(array), length)
    } else {
        (None, 1)
    };
    if length as usize > MAX_OPTIONS {
        return Err(invalid("selectOption exceeds 1024 descriptors"));
    }
    let mut total = 0usize;
    let mut result = Vec::with_capacity(length as usize);
    let mut string_items = None;
    for index in 0..length {
        let item = if let Some(array) = &array {
            array.get_element(index)?
        } else {
            value
        };
        let is_string = item.get_type()? == ValueType::String;
        if string_items.is_some_and(|previous| previous != is_string) {
            // Playwright rejects this at protocol validation with Error, not
            // the TypeError used by our explicit native argument bounds.
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "selectOption cannot mix strings and descriptors",
            ));
        }
        string_items = Some(is_string);
        let mut field = |value: JsString<'_>| {
            let bytes = value.utf8_len()?;
            if bytes > MAX_OPTION_BYTES || bytes > MAX_VALUE_BYTES - total {
                return Err(invalid("selectOption string budget exceeded"));
            }
            total += bytes;
            text(value, MAX_OPTION_BYTES)
        };
        let descriptor = if is_string {
            // SAFETY: the string type was checked in the active callback.
            SelectOptionValue::value_or_label(field(unsafe { item.cast::<JsString<'_>>()? })?)
        } else {
            let item = object(item)?;
            let value = item
                .get_named_property::<Option<JsString<'_>>>("value")?
                .map(&mut field)
                .transpose()?;
            let label = item
                .get_named_property::<Option<JsString<'_>>>("label")?
                .map(&mut field)
                .transpose()?;
            let index = item
                .get_named_property::<Option<f64>>("index")?
                .map(|value| {
                    integer(value, f64::from(i32::MIN), f64::from(i32::MAX), "index")
                        .map(|value| value as i32)
                })
                .transpose()?;
            SelectOptionValue {
                value,
                label,
                index,
                ..SelectOptionValue::new()
            }
        };
        result.push(descriptor);
    }
    Ok(result)
}

fn request(
    operation: &str,
    options: &Object<'_>,
    value: Option<Unknown<'_>>,
) -> napi::Result<LocatorAction> {
    Ok(match operation {
        "type" | "pressSequentially" => {
            // Accepted for compatibility but intentionally has no effect.
            let _: Option<bool> = options.get_named_property("noWaitAfter")?;
            LocatorAction::PressSequentially {
                text: input_text(value, MAX_VALUE_BYTES)?,
                delay: delay(options)?,
            }
        }
        "click" | "dblclick" => LocatorAction::Click(click(options, operation == "dblclick")?),
        "fill" | "clear" => LocatorAction::Fill {
            value: if operation == "clear" {
                String::new()
            } else {
                input_text(value, MAX_VALUE_BYTES)?
            },
            options: FillOptions::new().with_force(
                options
                    .get_named_property::<Option<bool>>("force")?
                    .unwrap_or(false),
            ),
        },
        "press" => LocatorAction::Press {
            key: PressKey::new(input_text(value, 128)?)
                .map_err(|error| invalid(error.to_string()))?,
            options: PressOptions {
                delay: delay(options)?,
                no_wait_after: options
                    .get_named_property::<Option<bool>>("noWaitAfter")?
                    .unwrap_or(false),
            },
        },
        "check" | "uncheck" | "setChecked" => LocatorAction::SetChecked {
            checked: match operation {
                "check" => true,
                "uncheck" => false,
                _ => {
                    let value = value.ok_or_else(|| invalid("setChecked requires a boolean"))?;
                    if value.get_type()? != ValueType::Boolean {
                        return Err(invalid("setChecked requires a boolean"));
                    }
                    // SAFETY: this is a checked Boolean in this Node callback.
                    unsafe { value.cast::<bool>()? }
                }
            },
            options: SetCheckedOptions {
                force: options.get_named_property("force")?,
                position: position(options)?,
                trial: options.get_named_property("trial")?,
                ..SetCheckedOptions::new()
            },
        },
        "selectOption" => LocatorAction::SelectOption {
            values: selection(value)?,
            options: SelectOptionOptions {
                force: options.get_named_property("force")?,
            },
        },
        "hover" => LocatorAction::Hover(HoverOptions {
            force: options
                .get_named_property::<Option<bool>>("force")?
                .unwrap_or(false),
            modifiers: modifiers(options)?,
            position: position(options)?,
            trial: options
                .get_named_property::<Option<bool>>("trial")?
                .unwrap_or(false),
            ..HoverOptions::new()
        }),
        "focus" => LocatorAction::Focus,
        "blur" => LocatorAction::Blur,
        "scrollIntoViewIfNeeded" => LocatorAction::ScrollIntoViewIfNeeded,
        _ => {
            return Err(binding_error(HostError::UnsupportedOperation(
                "unknown Locator action".into(),
            )));
        }
    })
}

#[napi]
impl NativeLocator {
    #[napi]
    pub fn act<'env>(
        &self,
        env: &'env Env,
        operation: JsString<'env>,
        timeout_ms: u32,
        options: Object<'env>,
        value: Option<Unknown<'env>>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, ACTION_BYTES)
            .map_err(binding_error)?;
        let operation = boundary::bounded_string(operation, 32, "Locator action")?;
        let action = request(&operation, &options, value)?;
        action.validate().map_err(binding_error)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            Ok(match locator.act(action, timeout_ms).await? {
                ActionValue::Unit => Either::A(()),
                ActionValue::Strings(values) => Either::B(values),
            })
        })
    }
}
