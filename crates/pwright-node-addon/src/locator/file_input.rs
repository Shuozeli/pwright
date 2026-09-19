//! Callback-scoped value conversion only. No filesystem reads, selector/DOM
//! logic, base64 or CDP lives in this boundary.
use super::NativeLocator;
use crate::NativeSession;
use crate::{binding_error, boundary, evaluation_value::status};
use napi::{
    Env, JsString, JsValue, Unknown, ValueType,
    bindgen_prelude::{Function, JsObjectValue, Object, PromiseRaw},
    sys,
};
use napi_derive::napi;
use pwright_node_host::{
    HostError,
    ffi_budget::FfiLane,
    file_input::{MemoryFile, PreparedFileInput, profile},
};
use std::{
    ptr,
    time::{Duration, Instant},
};

// Bounded metadata, UTF-16/UTF-8 conversion, handles and future storage only.
// Raw owned content is separately admitted by the Rust host before its copy.
const INGRESS_BYTES: u64 = 1024 * 1024;

#[napi(object)]
pub struct SelectorFileInputOptions {
    pub timeout_ms: u32,
    pub strict: bool,
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn frame_set_input_files<'env>(
        &self,
        env: &'env Env,
        frame_id: JsString<'env>,
        selector: JsString<'env>,
        files: Unknown<'env>,
        options: SelectorFileInputOptions,
        is_buffer: Function<'env, Unknown<'env>, bool>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        // The existing query budget admits/copies the selector. Neither the
        // temporary selector nor its nth projection becomes a JS object, so
        // finalization/GC cannot delay its release after public completion.
        let locator = self.locator(frame_id, selector)?;
        let selected = if options.strict {
            locator
        } else {
            locator.nth(0)?
        };
        selected.set_input_files(env, files, options.timeout_ms, is_buffer)
    }
}
fn invalid(message: &str) -> napi::Error {
    binding_error(HostError::InvalidArgument(message.into()))
}
fn text(value: Unknown<'_>, limit: usize, field: &str) -> napi::Result<String> {
    if value.get_type()? != ValueType::String {
        // The pinned Playwright channel validator reports Error, not TypeError,
        // for a payload name/MIME with the wrong language type.
        return Err(napi::Error::from_reason(field));
    }
    // SAFETY: checked string in this callback's handle scope.
    let value = unsafe { value.cast::<JsString<'_>>()? };
    if value.utf8_len()? > limit {
        return Err(invalid("file-input string exceeds selected byte limit"));
    }
    let length = value.utf16_len()?;
    let units = value.into_utf16()?;
    String::from_utf16(&units.as_slice()[..length]).map_err(|_| {
        binding_error(HostError::UnsupportedOperation(
            "unpaired UTF-16 in file-input metadata".into(),
        ))
    })
}
struct Payload<'env> {
    name: String,
    mime: Option<String>,
    buffer: Unknown<'env>,
}
enum Captured<'env> {
    Paths(Vec<String>),
    Payloads(Vec<Payload<'env>>),
}

fn capture(files: Unknown<'_>) -> napi::Result<Captured<'_>> {
    let items = if files.is_array()? {
        // SAFETY: Array is an object and remains rooted by the callback scope.
        let array = unsafe { files.cast::<Object<'_>>()? };
        let count = array.get_array_length()? as usize;
        if count > profile::ITEMS {
            return Err(invalid("file-input count exceeds selected limit"));
        }
        (0..count)
            .map(|i| array.get_element::<Unknown<'_>>(i as u32))
            .collect::<napi::Result<Vec<_>>>()?
    } else {
        vec![files]
    };
    let paths = items
        .iter()
        .map(Unknown::get_type)
        .collect::<napi::Result<Vec<_>>>()?;
    if paths.contains(&ValueType::String) {
        if paths.iter().any(|kind| *kind != ValueType::String) {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "File paths cannot be mixed with buffers",
            ));
        }
        return items
            .into_iter()
            .map(|value| text(value, profile::PATH_BYTES, "file path must be a string"))
            .collect::<napi::Result<Vec<_>>>()
            .map(Captured::Paths);
    }
    let mut payloads = Vec::with_capacity(items.len());
    for value in items {
        if value.get_type()? != ValueType::Object {
            return Err(invalid("file payload must be an object"));
        }
        // SAFETY: checked object; property getters may run arbitrary JS here.
        let object = unsafe { value.cast::<Object<'_>>()? };
        let name = text(
            object.get_named_property("name")?,
            profile::NAME_BYTES,
            "file name must be a string",
        )?;
        let mime = object.get_named_property::<Unknown<'_>>("mimeType")?;
        let mime = if mime.get_type()? == ValueType::Undefined {
            None
        } else {
            Some(text(
                mime,
                profile::MIME_BYTES,
                "file MIME must be a string",
            )?)
        };
        let buffer = object.get_named_property::<Unknown<'_>>("buffer")?;
        payloads.push(Payload { name, mime, buffer });
    }
    Ok(Captured::Payloads(payloads))
}

/// No JS property/function access is allowed after this function starts until
/// HostSession::prepare_file_payloads finishes copying all borrowed byte slices.
fn materialize(
    env: &Env,
    host: &pwright_node_host::HostSession,
    payloads: &[Payload<'_>],
) -> napi::Result<PreparedFileInput> {
    let mut inputs = Vec::with_capacity(payloads.len());
    for payload in payloads {
        let mut is_buffer = false;
        // SAFETY: callback-rooted handle, no JS invocation or pointer escape.
        status(unsafe { sys::napi_is_buffer(env.raw(), payload.buffer.raw(), &mut is_buffer) })?;
        if !is_buffer {
            return Err(invalid("file payload buffer must be a Buffer"));
        }
        let mut kind = 0;
        let mut length = 0;
        let mut data = ptr::null_mut();
        let mut backing = ptr::null_mut();
        let mut offset = 0;
        // SAFETY: metadata only; no dereference before backing checks below.
        status(unsafe {
            sys::napi_get_typedarray_info(
                env.raw(),
                payload.buffer.raw(),
                &mut kind,
                &mut length,
                &mut data,
                &mut backing,
                &mut offset,
            )
        })?;
        let mut ordinary = false;
        status(unsafe { sys::napi_is_arraybuffer(env.raw(), backing, &mut ordinary) })?;
        if !ordinary {
            return Err(binding_error(HostError::UnsupportedOperation(
                "shared file-input Buffer backing".into(),
            )));
        }
        let mut detached = false;
        status(unsafe { sys::napi_is_detached_arraybuffer(env.raw(), backing, &mut detached) })?;
        if detached {
            return Err(invalid(
                "Cannot perform Construct on a detached ArrayBuffer",
            ));
        }
        if kind != sys::TypedarrayType::uint8_array || (length > 0 && data.is_null()) {
            return Err(invalid("file-input Buffer view is unavailable"));
        }
        // SAFETY: non-shared, non-detached, callback-rooted Buffer. No JS can
        // resize/detach it between these native inspections and the Rust copy.
        let buffer = if length == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(data.cast::<u8>(), length) }
        };
        inputs.push(MemoryFile {
            name: &payload.name,
            mime_type: payload.mime.as_deref(),
            buffer,
        });
    }
    host.prepare_file_payloads(&inputs).map_err(binding_error)
}

#[napi]
impl NativeLocator {
    #[napi]
    pub fn set_input_files<'env>(
        &self,
        env: &'env Env,
        files: Unknown<'env>,
        timeout_ms: u32,
        is_buffer: Function<'env, Unknown<'env>, bool>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let started = Instant::now();
        let execution = self
            .ffi
            .admit(FfiLane::Command, INGRESS_BYTES)
            .map_err(binding_error)?;
        let locator = self.locator.clone();
        let host = locator.host().map_err(binding_error)?;
        enum Input {
            Paths(std::path::PathBuf, Vec<String>),
            Payloads(PreparedFileInput),
        }
        let input = match capture(files)? {
            Captured::Paths(paths) => {
                // Environment IO belongs to this Node boundary; the host gets
                // one explicit base snapshot, never a global lookup mid-read.
                let base = std::env::current_dir()
                    .map_err(|error| binding_error(HostError::Backend(error.to_string())))?;
                if base.as_os_str().len() > profile::PATH_BYTES {
                    return Err(invalid("current directory exceeds file-input path limit"));
                }
                Input::Paths(base, paths)
            }
            Captured::Payloads(payloads) => {
                // Node's napi_is_buffer also accepts other ArrayBuffer views.
                // Invoke the scoped language-type port before any raw pointer
                // inspection; even a hostile/reentrant port cannot invalidate
                // a borrowed Rust slice because none exists at this stage.
                for payload in &payloads {
                    if !is_buffer.call(payload.buffer)? {
                        return Err(napi::Error::from_reason(
                            "file payload buffer must be a Buffer",
                        ));
                    }
                }
                Input::Payloads(materialize(env, &host, &payloads)?)
            }
        };
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            let remaining = if timeout_ms == 0 {
                0
            } else {
                let duration = Duration::from_millis(u64::from(timeout_ms))
                    .checked_sub(started.elapsed())
                    .filter(|duration| !duration.is_zero())
                    .ok_or(HostError::Timeout(timeout_ms))?;
                u32::try_from(duration.as_millis().max(1))
                    .map_err(|error| HostError::Backend(error.to_string()))?
            };
            let result = match input {
                Input::Paths(base, paths) => {
                    let views: Vec<_> = paths.iter().map(String::as_str).collect();
                    locator
                        .set_input_files_paths(&base, &views, remaining)
                        .await
                }
                Input::Payloads(payloads) => locator.set_input_files(payloads, remaining).await,
            };
            result.map_err(|error| match error {
                HostError::Timeout(_) => HostError::Timeout(timeout_ms),
                other => other,
            })
        })
    }
}
