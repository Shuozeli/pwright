//! Selected client metadata and scoped language-value timeout resolution.
//! No browser, native-use authority, JS root or running task is retained here.

use super::super::publication;
use crate::{
    NativeSession, binding_error, boundary, environment::NativeEnvironment,
    evaluation_value::status,
};
use napi::{Env, JsString, JsValue, Unknown, ValueType, bindgen_prelude::ToNapiValue, sys};
use napi_derive::napi;
use pwright_node_host::{
    HostError,
    handles::{ScopedTimeoutSettings, TimeoutSettings},
};
use std::ptr;

/// Internal client settings. Only canonical session-issued Page policies may
/// enter production Handle creation; unscoped settings remain diagnostic-only.
#[napi]
pub struct NativeFileInputTimeoutSettings {
    pub(super) settings: TimeoutSettings,
    scoped: Option<ScopedTimeoutSettings>,
}

pub struct TimeoutSettingsOutput(NativeFileInputTimeoutSettings);

impl TimeoutSettingsOutput {
    fn scoped(policy: ScopedTimeoutSettings) -> Self {
        Self(NativeFileInputTimeoutSettings {
            settings: policy.settings().clone(),
            scoped: Some(policy),
        })
    }
}

impl ToNapiValue for TimeoutSettingsOutput {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        // Failed construction/tag/wrap must release the newly admitted cell.
        unsafe { publication::to_napi_class(env, "NativeFileInputTimeoutSettings\0", value.0) }
    }
}

#[napi]
impl NativeEnvironment {
    /// Inject the cached debug classification explicitly. This creates a
    /// Context/root metadata cell, not a Context or a browser connection.
    #[napi]
    pub fn create_file_input_timeout_settings(
        &self,
        debug: bool,
    ) -> napi::Result<TimeoutSettingsOutput> {
        let settings = self
            .file_input_timeouts
            .root(debug)
            .map_err(binding_error)?;
        Ok(TimeoutSettingsOutput(NativeFileInputTimeoutSettings {
            settings,
            scoped: None,
        }))
    }
}

#[napi]
impl NativeFileInputTimeoutSettings {
    /// A Page inherits this Context/root's live defaults. The host rejects
    /// deeper trees; this internal model is the normal Page -> Context chain.
    #[napi]
    pub fn child(&self) -> napi::Result<TimeoutSettingsOutput> {
        if self.scoped.is_some() {
            return Err(binding_error(HostError::InvalidArgument(
                "Session-bound Page settings must be issued by their session".into(),
            )));
        }
        let settings = self.settings.child().map_err(binding_error)?;
        Ok(TimeoutSettingsOutput(NativeFileInputTimeoutSettings {
            settings,
            scoped: None,
        }))
    }

    /// Typed internal metadata update, not public Page.setDefaultTimeout or its
    /// channel validation. Undefined clears the setting; numbers stay exact.
    #[napi]
    pub fn set_default_timeout(&self, timeout: Option<f64>) -> napi::Result<()> {
        self.settings
            .set_default_timeout(timeout)
            .map_err(binding_error)
    }
}

#[napi]
impl NativeSession {
    /// The facade captures the three raw environment values at module load.
    /// Rust owns fallback/classification; no process environment is read here.
    #[napi]
    pub fn initialize_file_input_timeouts(
        &self,
        direct: Option<JsString<'_>>,
        npm_config: Option<JsString<'_>>,
        npm_package: Option<JsString<'_>>,
    ) -> napi::Result<()> {
        let debug = classify_debug([direct, npm_config, npm_package], |value| {
            let length = value.utf8_len()?;
            // Only the three excluded short spellings need a string copy.
            // Arbitrarily large raw environment values do not cause Rust copies.
            Ok(if length == 0 {
                false
            } else if matches!(length, 1 | 5 | 7) {
                is_inspector(value.into_utf8()?.as_str()?)
            } else {
                true
            })
        })?;
        self.file_input_scope
            .initialize_timeouts(&self.file_input_timeout_budget, &self.host, debug)
            .map_err(binding_error)
    }

    #[napi]
    pub fn file_input_context_timeout_settings(
        &self,
        context_id: JsString<'_>,
    ) -> napi::Result<TimeoutSettingsOutput> {
        let id = boundary::bounded_string(context_id, 20, "Context ID")?;
        let policies = self
            .file_input_scope
            .timeout_policies()
            .map_err(binding_error)?;
        policies
            .context(&self.host, &id)
            .map(TimeoutSettingsOutput::scoped)
            .map_err(binding_error)
    }

    #[napi]
    pub fn file_input_page_timeout_settings(
        &self,
        page_id: JsString<'_>,
    ) -> napi::Result<TimeoutSettingsOutput> {
        let id = boundary::bounded_string(page_id, 20, "Page ID")?;
        let policies = self
            .file_input_scope
            .timeout_policies()
            .map_err(binding_error)?;
        policies
            .page(&self.host, &id)
            .map(TimeoutSettingsOutput::scoped)
            .map_err(binding_error)
    }
}

fn is_inspector(value: &str) -> bool {
    !matches!(value, "" | "0" | "false" | "console")
}

fn classify_debug<T>(
    captured: [Option<T>; 3],
    classify: impl FnOnce(T) -> napi::Result<bool>,
) -> napi::Result<bool> {
    captured
        .into_iter()
        .flatten()
        .next()
        .map_or(Ok(false), classify)
}

/// Reproduce Frame's `options || {}` normalization, then the ordinary
/// Page/Context TimeoutSettings traversal. Policy decisions stay in Rust;
/// the sole language port performs a member read on the original options.
///
/// A numeric typeof result causes a SECOND read, whose raw value may no longer
/// be numeric. Do not coerce, validate, prefix exceptions or truncate it here.
/// Local defaults are read only after the getter/debug branch, so a getter's
/// synchronous metadata mutation affects this very resolution. No metadata
/// lock or registry borrow spans any JavaScript callback.
pub(super) fn resolve<'env>(
    env: &'env Env,
    options: Unknown<'env>,
    settings: &TimeoutSettings,
    timeout_port: Unknown<'env>,
) -> napi::Result<Unknown<'env>> {
    let options = if options.coerce_to_bool()? {
        options
    } else {
        let mut value = ptr::null_mut();
        status(unsafe { sys::napi_create_object(env.raw(), &mut value) })?;
        unsafe { Unknown::from_raw_unchecked(env.raw(), value) }
    };
    let mut current = settings.clone();
    loop {
        let value = read_timeout(env, timeout_port, options)?;
        if value.get_type()? == ValueType::Number {
            return read_timeout(env, timeout_port, options);
        }
        if current.is_debug() {
            return number(env, 0.0);
        }
        if let Some(value) = current.local_default().map_err(binding_error)? {
            return number(env, value);
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return number(env, 30_000.0),
        }
    }
}

fn number<'env>(env: &'env Env, value: f64) -> napi::Result<Unknown<'env>> {
    let mut result = ptr::null_mut();
    status(unsafe { sys::napi_create_double(env.raw(), value, &mut result) })?;
    Ok(unsafe { Unknown::from_raw_unchecked(env.raw(), result) })
}

fn read_timeout<'env>(
    env: &'env Env,
    port: Unknown<'env>,
    options: Unknown<'env>,
) -> napi::Result<Unknown<'env>> {
    let mut receiver = ptr::null_mut();
    let mut result = ptr::null_mut();
    let args = [options.raw()];
    // SAFETY: all values belong to this live, owning callback's handle scope.
    unsafe {
        status(sys::napi_get_undefined(env.raw(), &mut receiver))?;
        status(sys::napi_call_function(
            env.raw(),
            receiver,
            port.raw(),
            args.len(),
            args.as_ptr(),
            &mut result,
        ))?;
        Ok(Unknown::from_raw_unchecked(env.raw(), result))
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_debug, is_inspector};

    #[test]
    fn cached_debug_classification_matches_pinned_inspector_spellings() {
        // Arrange
        let values = [
            "",
            "0",
            "false",
            "console",
            "1",
            "true",
            "inspector",
            "False",
            "00",
            " ",
        ];
        // Act
        let actual = values.map(is_inspector);
        // Assert
        assert_eq!(
            actual,
            [
                false, false, false, false, true, true, true, true, true, true
            ]
        );
    }

    #[test]
    fn debug_environment_fallback_uses_first_defined_even_when_empty_or_disabled() {
        // Arrange
        let cases = [
            ([None, None, None], false),
            ([None, Some("1"), None], true),
            ([None, None, Some("1")], true),
            ([Some(""), Some("1"), Some("1")], false),
            ([Some("0"), Some("1"), Some("1")], false),
            ([None, Some("console"), Some("1")], false),
            ([Some("inspect"), Some("false"), None], true),
        ];
        // Act
        let actual = cases.map(|(captured, _)| {
            classify_debug(captured, |value| Ok(is_inspector(value))).unwrap()
        });
        // Assert
        assert_eq!(actual, cases.map(|(_, expected)| expected));
    }
}
