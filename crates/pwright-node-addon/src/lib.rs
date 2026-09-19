//! JS value conversion only. All browser behavior belongs to the Rust host.

// Generated N-API paths use this crate-wide facade, including registration
// modules. Renaming the dependency removes the competing extern-prelude name.
extern crate self as napi;
pub use napi_runtime::ctor;
pub use napi_runtime::*;
pub use native_napi::bindgen_prelude;

mod boundary;
mod callback_exception;
mod context_settings;
mod deferred_conversion;
mod environment;
mod evaluation_value;
#[cfg(feature = "extra-headers")]
mod extra_headers;
#[cfg(feature = "frame-navigation")]
mod frame_navigation;
mod frame_tree;
#[cfg(feature = "handles")]
mod handles;
mod keyboard;
mod locator;
mod locator_conversion;
mod mouse;
mod native_classes;
mod native_napi;
mod node_reference;
mod page_snapshot;
mod publication_port;
mod reload;
mod screenshot;
mod url_value;

#[cfg(feature = "retained-file-input")]
pub use handles::file_input::{
    inspect_file_input_arguments, inspect_file_input_channel, inspect_file_input_materialization,
    inspect_file_input_preflight_context, inspect_file_input_preflight_conversion,
    inspect_file_input_preflight_owner, inspect_file_input_projection,
    inspect_file_input_server_channel,
};
pub use native_classes::{native_class_matches, native_class_names};

use std::collections::HashMap;
use std::sync::Arc;

use bindgen_prelude::{PromiseRaw, ToNapiValue};
use napi_derive::napi;
use pwright_node_host::ffi_budget::{FfiBudget, FfiLane};
use pwright_node_host::{HostError, HostSession};

/// Build identity only; no session, task, transport or allocation is created.
#[napi]
pub fn drag_runtime_enabled() -> bool {
    cfg!(feature = "drag-runtime")
}

/// Build identity only; Frame navigation remains an explicitly selected leaf.
#[napi]
pub fn frame_navigation_enabled() -> bool {
    cfg!(feature = "frame-navigation")
}

#[napi]
pub fn extra_headers_enabled() -> bool {
    cfg!(feature = "extra-headers")
}

#[napi]
pub fn concurrent_close_enabled() -> bool {
    cfg!(feature = "concurrent-close")
}

#[napi]
pub fn file_input_enabled() -> bool {
    cfg!(feature = "file-input")
}

#[napi]
pub fn handles_enabled() -> bool {
    cfg!(feature = "handles")
}

#[napi]
pub fn retained_file_input_enabled() -> bool {
    cfg!(feature = "retained-file-input")
}

/// Build identity only; actual target marking remains an explicitly selected leaf.
#[napi]
pub fn retained_target_marking_enabled() -> bool {
    cfg!(feature = "retained-target-marking")
}

/// Build identity only; retained DOM prevalidation is independently selected.
#[napi]
pub fn retained_target_prevalidation_enabled() -> bool {
    cfg!(feature = "retained-target-prevalidation")
}

/// Build identity only; the awaited input-action boundary is independently selected.
#[napi]
pub fn retained_input_instrumentation_enabled() -> bool {
    cfg!(feature = "retained-input-instrumentation")
}

/// Build identity only; late call-argument serialization is independently selected.
#[napi]
pub fn retained_call_argument_serialization_enabled() -> bool {
    cfg!(feature = "retained-call-argument-serialization")
}

/// Build identity only; Rust graph import is independently selected.
#[napi]
pub fn retained_serialized_payload_import_enabled() -> bool {
    cfg!(feature = "retained-serialized-payload-import")
}

/// Build identity only; retained graph mutation is independently selected.
#[napi]
pub fn retained_serialized_payload_mutation_enabled() -> bool {
    cfg!(feature = "retained-serialized-payload-mutation")
}

fn binding_error(error: HostError) -> napi::Error {
    napi::Error::new(
        napi::Status::GenericFailure,
        format!("{}: {}", error.code(), error),
    )
}

#[napi(object, object_from_js = false)]
pub struct ContextDescriptor {
    pub id: String,
    pub settings: context_settings::NativeContextSettings,
}

#[napi(object, object_from_js = false)]
pub struct PageDescriptor {
    pub id: String,
    pub context_id: String,
    pub main_frame_id: String,
    pub frame_tree: frame_tree::NativeFrameTree,
    pub main_frame_binding: Option<frame_tree::NativeFrameNode>,
}

impl From<pwright_node_host::PageDescriptor> for PageDescriptor {
    fn from(value: pwright_node_host::PageDescriptor) -> Self {
        Self {
            id: value.id,
            context_id: value.context_id,
            main_frame_id: value.main_frame_id,
            frame_tree: frame_tree::NativeFrameTree {
                binding: value.frame_tree_binding,
            },
            main_frame_binding: value
                .main_frame_binding
                .map(|binding| frame_tree::NativeFrameNode { binding: *binding }),
        }
    }
}

#[napi(object)]
pub struct FrameMetadataMetrics {
    pub entries: u32,
    pub url_bytes: u32,
    pub name_bytes: u32,
    pub active_frames: u32,
    pub pending_publications: u32,
}

#[napi(object)]
pub struct FrameTreeMetrics {
    pub states: u32,
    pub revisions: u32,
    pub bytes: u32,
}

#[napi(object)]
pub struct SessionMetrics {
    pub connected: bool,
    pub live_objects: u32,
    pub pending_calls: u32,
    pub topology_drivers: u32,
    pub active_leases: u32,
    pub live_scopes: u32,
    pub retained_event_bytes: u32,
    pub active_operations: u32,
    pub cleanup_in_flight: u32,
}

#[napi(object)]
pub struct NavigationResponse {
    pub url: String,
    pub status: i64,
    pub status_text: String,
    pub ok: bool,
    pub headers: HashMap<String, String>,
}

#[napi(object)]
pub struct NavigationResult {
    pub url: String,
    pub response: Option<NavigationResponse>,
}

#[napi]
pub struct NativeSessionSignal {
    token: tokio_util::sync::CancellationToken,
    ffi: FfiBudget,
    publications: publication_port::PublicationPort,
}

#[napi]
impl NativeSessionSignal {
    #[napi]
    pub fn wait<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        // Never let an async task borrow a JS-owned native object: Worker
        // teardown can finalize that object before a pending Promise settles.
        let execution = self
            .ffi
            .admit(FfiLane::Watch, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let token = self.token.clone();
        let port = self.publications.clone();
        let publications = port.publications();
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                tokio::select! {
                    biased;
                    () = token.cancelled() => Ok((false, publications.fence()?)),
                    fence = publications.next() => Ok((true, fence?)),
                }
            },
            move |env, (connected, fence)| {
                port.publish(env, fence)?;
                Ok(connected)
            },
        )
    }
}

#[napi]
pub struct NativeSession {
    #[cfg(feature = "retained-file-input")]
    file_input_timeout_budget: pwright_node_host::handles::TimeoutPolicyBudget,
    #[cfg(feature = "retained-file-input")]
    file_input_preparations: handles::file_input::FileInputPreparations,
    #[cfg(feature = "retained-file-input")]
    file_input_scope: handles::file_input::PreparationScope,
    publications: publication_port::PublicationPort,
    host: Arc<HostSession>,
    ffi: FfiBudget,
    conversions: deferred_conversion::DeferredConversions,
    locator_conversions: locator_conversion::LocatorConversions,
    queries: pwright_node_host::locator_query::QueryBudget,
}

#[napi(object)]
pub struct FfiMetrics {
    pub pending: u32,
    pub queued: u32,
    pub queued_commands: u32,
    pub running: u32,
    pub reserved_bytes: u32,
    pub connects: u32,
    pub commands: u32,
    pub closes: u32,
    pub watches: u32,
}

impl Drop for NativeSession {
    fn drop(&mut self) {
        #[cfg(feature = "retained-file-input")]
        self.file_input_scope.close();
        self.host.request_shutdown();
    }
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn set_lifecycle_callback(
        &self,
        env: Env,
        callback: napi::bindgen_prelude::Function<'_>,
    ) -> napi::Result<()> {
        self.publications.register(&env, callback)
    }

    #[napi]
    pub fn version(&self) -> String {
        self.host.version().to_owned()
    }

    #[napi]
    pub fn is_connected(&self) -> napi::Result<bool> {
        self.host
            .frame_publications()
            .connection_is_published_open()
            .map_err(binding_error)
    }

    #[napi]
    pub fn publish_disconnect(&self) -> napi::Result<bool> {
        self.host
            .frame_publications()
            .publish_disconnect()
            .map_err(binding_error)
    }

    #[napi]
    pub fn session_signal(&self) -> NativeSessionSignal {
        NativeSessionSignal {
            token: self.host.disconnect_signal(),
            ffi: self.ffi.clone(),
            publications: self.publications.clone(),
        }
    }

    #[napi]
    pub fn contexts<'env>(&self, env: &'env Env) -> napi::Result<Unknown<'env>> {
        let guard = self
            .host
            .reserve_context_conversion(page_snapshot::CONVERSION_BYTES)
            .map_err(binding_error)?;
        let contexts: Vec<_> = self
            .host
            .published_contexts()
            .map_err(binding_error)?
            .into_iter()
            .map(ContextDescriptor::try_from)
            .collect::<std::result::Result<_, _>>()
            .map_err(binding_error)?;
        let result = contexts.into_unknown(env);
        guard.finish();
        result
    }

    #[napi]
    pub fn pages<'env>(
        &self,
        env: &'env Env,
        context_id: JsString<'_>,
    ) -> napi::Result<Unknown<'env>> {
        let guard = self
            .host
            .reserve_metadata_conversion(page_snapshot::CONVERSION_BYTES)
            .map_err(binding_error)?;
        let context_id = boundary::bounded_string(context_id, 20, "context ID")?;
        let pages: Vec<_> = self
            .host
            .published_pages(&context_id)
            .map_err(binding_error)?
            .into_iter()
            .map(PageDescriptor::from)
            .collect();
        let result = pages.into_unknown(env);
        guard.finish();
        result
    }

    #[napi]
    pub fn new_context<'env>(
        &self,
        env: &'env Env,
        base_url: Option<JsString<'_>>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let base_url = context_settings::read_base_url(env, base_url)?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            ContextDescriptor::try_from(host.new_context_with_base_url(base_url.as_deref()).await?)
        })
    }

    #[napi]
    pub fn close_context<'env>(
        &self,
        env: &'env Env,
        context_id: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Close, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let context_id = boundary::bounded_string(context_id, 20, "context ID")?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.close_context(&context_id).await
        })
    }

    #[napi]
    pub fn new_page<'env>(
        &self,
        env: &'env Env,
        context_id: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let context_id = boundary::bounded_string(context_id, 20, "context ID")?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.new_page(&context_id).await.map(PageDescriptor::from)
        })
    }

    #[napi]
    pub fn close_page<'env>(
        &self,
        env: &'env Env,
        page_id: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Close, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let page_id = boundary::bounded_string(page_id, 20, "page ID")?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.close_page(&page_id).await
        })
    }

    #[napi]
    pub fn metrics(&self) -> napi::Result<SessionMetrics> {
        let value = self.host.metrics().map_err(binding_error)?;
        Ok(SessionMetrics {
            connected: value.connected,
            live_objects: value.live_objects,
            pending_calls: value.pending_calls,
            topology_drivers: value.topology_drivers,
            active_leases: value.active_leases,
            live_scopes: value.live_scopes,
            retained_event_bytes: value.retained_event_bytes,
            active_operations: value.active_operations,
            cleanup_in_flight: value.cleanup_in_flight,
        })
    }

    #[napi]
    pub fn frame_metadata_metrics(&self) -> napi::Result<FrameMetadataMetrics> {
        let value = self.host.frame_metadata_metrics().map_err(binding_error)?;
        Ok(FrameMetadataMetrics {
            entries: value.entries,
            url_bytes: value.url_bytes,
            name_bytes: value.name_bytes,
            active_frames: value.active_frames,
            pending_publications: value.pending_publications,
        })
    }

    #[napi]
    pub fn frame_tree_metrics(&self) -> napi::Result<FrameTreeMetrics> {
        let value = self.host.frame_tree_metrics().map_err(binding_error)?;
        let count = |value: u64| {
            u32::try_from(value)
                .map_err(|error| binding_error(HostError::Backend(error.to_string())))
        };
        Ok(FrameTreeMetrics {
            states: count(value.states)?,
            revisions: count(value.revisions)?,
            bytes: count(value.bytes)?,
        })
    }

    #[napi]
    pub fn ffi_metrics(&self) -> napi::Result<FfiMetrics> {
        let value = self.ffi.metrics().map_err(binding_error)?;
        Ok(FfiMetrics {
            pending: value.pending,
            queued: value.queued,
            queued_commands: value.queued_commands,
            running: value.running,
            reserved_bytes: u32::try_from(value.reserved_bytes)
                .map_err(|error| binding_error(HostError::Backend(error.to_string())))?,
            connects: value.connects,
            commands: value.commands,
            closes: value.closes,
            watches: value.watches,
        })
    }

    #[napi]
    pub fn conversion_metrics(&self) -> deferred_conversion::ConversionMetrics {
        self.conversions.metrics()
    }

    #[napi]
    pub fn locator_conversion_metrics(&self) -> locator_conversion::LocatorConversionMetrics {
        self.locator_conversions.metrics()
    }

    #[napi]
    pub fn close<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Close, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        #[cfg(feature = "retained-file-input")]
        self.file_input_scope.close();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.close().await
        })
    }

    #[napi]
    pub fn frame_title<'env>(
        &self,
        env: &'env Env,
        frame_id: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::READ_ALLOWANCE)
            .map_err(binding_error)?;
        let frame_id = boundary::bounded_string(frame_id, 20, "frame ID")?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.frame_title(&frame_id).await
        })
    }

    #[napi]
    pub fn frame_content<'env>(
        &self,
        env: &'env Env,
        frame_id: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::READ_ALLOWANCE)
            .map_err(binding_error)?;
        let frame_id = boundary::bounded_string(frame_id, 20, "frame ID")?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.frame_content(&frame_id).await
        })
    }

    #[napi]
    pub fn frame_evaluate<'env>(
        &self,
        env: &'env Env,
        frame_id: JsString<'env>,
        source: JsString<'env>,
        is_function: bool,
        argument: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        use pwright_node_host::evaluation::FFI_ALLOWANCE;
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let frame_id = boundary::bounded_string(frame_id, 20, "frame ID")?;
        let source = evaluation_value::source(env, source, is_function)?;
        // Even expression arguments are validated like the Playwright client,
        // although expression mode does not pass them to the browser program.
        let input = self.conversions.prepare(env, argument)?;
        self.conversions.schedule(
            env,
            deferred_conversion::EvaluationCall {
                publications: self.publications.clone(),
                execution,
                host: self.host.clone(),
                target: deferred_conversion::EvaluationTarget::FrameValue(frame_id),
                source,
                is_function,
                input,
            },
        )
    }

    #[napi]
    pub fn page_goto<'env>(
        &self,
        env: &'env Env,
        page_id: JsString<'env>,
        url: JsString<'env>,
        wait_until: JsString<'env>,
        timeout_ms: u32,
        referrer: Option<JsString<'env>>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::NAVIGATION_ALLOWANCE)
            .map_err(binding_error)?;
        let page_id = boundary::bounded_string(page_id, 20, "page ID")?;
        let url = boundary::bounded_string(url, 16 * 1024, "navigation URL")?;
        let wait_until = boundary::bounded_string(wait_until, 32, "waitUntil")?;
        let referrer = referrer
            .map(|value| boundary::bounded_string(value, 16 * 1024, "referer"))
            .transpose()?;
        let host = Arc::clone(&self.host);
        let publications = self.publications.clone();
        #[cfg(feature = "extra-headers")]
        let pending = host.start_page_goto(page_id, url, wait_until, referrer, timeout_ms);
        #[cfg(not(feature = "extra-headers"))]
        let pending = async move {
            host.page_goto(&page_id, url, &wait_until, referrer, timeout_ms)
                .await
        };
        boundary::spawn_published(env, execution, publications, async move {
            let result = pending.await?;
            Ok(NavigationResult {
                url: result.url,
                response: result.response.map(|response| NavigationResponse {
                    url: response.url,
                    status: response.status,
                    status_text: response.status_text,
                    ok: response.ok,
                    headers: response.headers.into_iter().collect(),
                }),
            })
        })
    }
}
