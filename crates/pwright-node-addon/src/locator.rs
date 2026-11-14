//! Thin Node value bindings; query ownership and all DOM reads remain Rust.

pub(crate) mod actions;
mod click_steps;
#[cfg(feature = "file-input")]
mod file_input;
mod semantic;

use std::{str::FromStr, sync::Arc};

use napi::{
    Env, JsString, JsValue, Unknown,
    bindgen_prelude::{Either4, Function, JsObjectValue, Object, Promise, PromiseRaw, ToNapiValue},
    sys,
};
use napi_derive::napi;
#[cfg(feature = "locator-bounding-box")]
use pwright_node_host::locator::LocatorBoundingBox;
use pwright_node_host::{
    HostError,
    ffi_budget::{FfiBudget, FfiLane},
    locator::{
        HostLocator, LocatorExpectationState, LocatorRead, LocatorStringExpectationKind,
        LocatorValue, LocatorWaitState, StringMatcher, TextExpectation, TextsExpectation,
    },
    locator_query::{QueryBudget, QueryCombination},
};

use crate::{NativeSession, binding_error, boundary};

const SELECTOR_BYTES: usize = 64 * 1024;
// Covers the largest collection read, one 1 MiB item during Node conversion,
// command inputs, and conservative selector copies. No allocation is made by
// this declaration; all reads share the finite environment settlement budget.
const READ_BYTES: u64 = 6 * 1024 * 1024;

#[napi(object)]
pub struct LocatorMetrics {
    pub recipes: u32,
    pub bytes: u32,
}

#[napi(object)]
pub struct LocatorExpectationResult {
    pub matches: bool,
    pub received: Option<String>,
    pub preview: Option<String>,
    pub missing: bool,
    #[napi(js_name = "timedOut")]
    pub timed_out: bool,
}

#[napi(object)]
pub struct LocatorVisibilityExpectationResult {
    pub matches: bool,
    pub received: bool,
    pub preview: Option<String>,
    pub missing: bool,
    #[napi(js_name = "timedOut")]
    pub timed_out: bool,
}

#[napi(object)]
pub struct LocatorCountExpectationResult {
    pub matches: bool,
    pub received: Option<u32>,
    #[napi(js_name = "timedOut")]
    pub timed_out: bool,
}

#[napi(object)]
pub struct LocatorTextsExpectationResult {
    pub matches: bool,
    pub received: Vec<String>,
    #[napi(js_name = "timedOut")]
    pub timed_out: bool,
}

#[napi(object)]
pub struct LocatorStringMatcherInput {
    pub expected: String,
    pub flags: Option<String>,
}

#[cfg(feature = "locator-bounding-box")]
#[napi(object)]
pub struct LocatorBoundingBoxOutput {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[cfg(feature = "locator-bounding-box")]
impl From<LocatorBoundingBox> for LocatorBoundingBoxOutput {
    fn from(value: LocatorBoundingBox) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

struct LocatorValueExpectationOutput(pwright_node_host::locator::LocatorValueExpectation);

impl ToNapiValue for LocatorValueExpectationOutput {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        let env = Env::from_raw(env);
        let mut result = Object::new(&env)?;
        result.set_named_property("matches", value.0.matches)?;
        result.set_named_property("missing", value.0.missing)?;
        result.set_named_property("timedOut", value.0.timed_out)?;
        if let Some(received) = value.0.received {
            let received = unsafe {
                crate::evaluation_value::EvaluationOutput::to_napi_value(
                    env.raw(),
                    crate::evaluation_value::EvaluationOutput(received),
                )?
            };
            result.set_named_property("received", unsafe {
                Unknown::from_raw_unchecked(env.raw(), received)
            })?;
        }
        Ok(result.raw())
    }
}

#[napi]
pub struct NativeLocator {
    publications: crate::publication_port::PublicationPort,
    locator: HostLocator,
    ffi: FfiBudget,
    #[cfg(feature = "handles")]
    wait_admissions: crate::wait_admission::WaitAdmissions,
    queries: QueryBudget,
    conversions: crate::locator_conversion::LocatorConversions,
    #[cfg(feature = "selector-registration")]
    selectors: pwright_node_host::selector_registry::SelectorRegistry,
}

/// Thin binding for the Rust-owned lazy iframe-owner recipe. The binding
/// retains no resolved child Frame: every operation on a derived Locator asks
/// Rust to resolve the current iframe content Frame again.
#[cfg(feature = "handles")]
#[napi]
pub struct NativeFrameLocator {
    publications: crate::publication_port::PublicationPort,
    locator: pwright_node_host::locator::HostFrameLocator,
    ffi: FfiBudget,
    wait_admissions: crate::wait_admission::WaitAdmissions,
    queries: QueryBudget,
    conversions: crate::locator_conversion::LocatorConversions,
    #[cfg(feature = "selector-registration")]
    selectors: pwright_node_host::selector_registry::SelectorRegistry,
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn locator(
        &self,
        frame_id: JsString<'_>,
        selector: JsString<'_>,
    ) -> napi::Result<NativeLocator> {
        let draft = self
            .queries
            .prepare_leaf(selector.utf8_len()?)
            .map_err(binding_error)?;
        let frame_id = boundary::bounded_string(frame_id, 20, "Frame ID")?;
        let source = boundary::bounded_string(selector, SELECTOR_BYTES, "selector")?;
        #[cfg(feature = "selector-registration")]
        let query = draft
            .css_registered(source, &self.selectors)
            .map_err(binding_error)?;
        #[cfg(not(feature = "selector-registration"))]
        let query = draft.css(source).map_err(binding_error)?;
        Ok(NativeLocator {
            locator: self.host.locator(&frame_id, query).map_err(binding_error)?,
            publications: self.publications.clone(),
            ffi: self.ffi.clone(),
            #[cfg(feature = "handles")]
            wait_admissions: self.wait_admissions.clone(),
            queries: self.queries.clone(),
            conversions: self.locator_conversions.clone(),
            #[cfg(feature = "selector-registration")]
            selectors: self.selectors.clone(),
        })
    }

    #[napi]
    pub fn locator_metrics(&self) -> napi::Result<LocatorMetrics> {
        let metrics = self.queries.metrics().map_err(binding_error)?;
        Ok(LocatorMetrics {
            recipes: u32::try_from(metrics.recipes)
                .map_err(|error| binding_error(HostError::Backend(error.to_string())))?,
            bytes: u32::try_from(metrics.bytes)
                .map_err(|error| binding_error(HostError::Backend(error.to_string())))?,
        })
    }
}

#[cfg(feature = "handles")]
#[napi]
impl NativeSession {
    #[napi(js_name = "frameLocator")]
    pub fn frame_locator(
        &self,
        frame_id: JsString<'_>,
        selector: JsString<'_>,
    ) -> napi::Result<NativeFrameLocator> {
        let draft = self
            .queries
            .prepare_leaf(selector.utf8_len()?)
            .map_err(binding_error)?;
        let frame_id = boundary::bounded_string(frame_id, 20, "Frame ID")?;
        let source = boundary::bounded_string(selector, SELECTOR_BYTES, "selector")?;
        #[cfg(feature = "selector-registration")]
        let query = draft
            .css_registered(source, &self.selectors)
            .map_err(binding_error)?;
        #[cfg(not(feature = "selector-registration"))]
        let query = draft.css(source).map_err(binding_error)?;
        let owner = self.host.locator(&frame_id, query).map_err(binding_error)?;
        Ok(NativeFrameLocator {
            locator: owner.content_frame().map_err(binding_error)?,
            publications: self.publications.clone(),
            ffi: self.ffi.clone(),
            wait_admissions: self.wait_admissions.clone(),
            queries: self.queries.clone(),
            conversions: self.locator_conversions.clone(),
            #[cfg(feature = "selector-registration")]
            selectors: self.selectors.clone(),
        })
    }
}

impl NativeLocator {
    fn derive(&self, locator: HostLocator) -> Self {
        Self {
            locator,
            publications: self.publications.clone(),
            ffi: self.ffi.clone(),
            #[cfg(feature = "handles")]
            wait_admissions: self.wait_admissions.clone(),
            queries: self.queries.clone(),
            conversions: self.conversions.clone(),
            #[cfg(feature = "selector-registration")]
            selectors: self.selectors.clone(),
        }
    }

    #[cfg(feature = "handles")]
    fn wait_for_selector_impl<'env>(
        &self,
        env: &'env Env,
        state: JsString<'env>,
        timeout_ms: u32,
        strict: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let wait_permit = self.wait_admissions.admit().map_err(binding_error)?;
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let state = boundary::bounded_string(state, 16, "Page wait state")?;
        let state = match state.as_str() {
            "attached" => LocatorWaitState::Attached,
            "detached" => LocatorWaitState::Detached,
            "visible" => LocatorWaitState::Visible,
            "hidden" => LocatorWaitState::Hidden,
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "invalid Page wait state".into(),
                )));
            }
        };
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published_wait(env, execution, publications, wait_permit, async move {
            locator
                .wait_for_element(state, timeout_ms, strict)
                .await
                .map(|handle| match handle {
                    Some(handle) => crate::deferred_conversion::EvaluationResult::Handle(
                        handle,
                        crate::handles::HandlePolicy::default(),
                    ),
                    None => crate::deferred_conversion::EvaluationResult::Value(
                        crate::evaluation_value::EvaluationOutput(
                            pwright_node_host::evaluation::EvaluationValue::Null,
                        ),
                    ),
                })
        })
    }
}

#[cfg(feature = "handles")]
impl NativeFrameLocator {
    fn derive(&self, locator: pwright_node_host::locator::HostFrameLocator) -> Self {
        Self {
            locator,
            publications: self.publications.clone(),
            ffi: self.ffi.clone(),
            wait_admissions: self.wait_admissions.clone(),
            queries: self.queries.clone(),
            conversions: self.conversions.clone(),
            #[cfg(feature = "selector-registration")]
            selectors: self.selectors.clone(),
        }
    }

    fn derive_locator(&self, locator: HostLocator) -> NativeLocator {
        NativeLocator {
            locator,
            publications: self.publications.clone(),
            ffi: self.ffi.clone(),
            wait_admissions: self.wait_admissions.clone(),
            queries: self.queries.clone(),
            conversions: self.conversions.clone(),
            #[cfg(feature = "selector-registration")]
            selectors: self.selectors.clone(),
        }
    }
}

#[cfg(feature = "handles")]
#[napi]
impl NativeFrameLocator {
    #[napi]
    pub fn locator(&self, selector: JsString<'_>) -> napi::Result<NativeLocator> {
        let draft = self
            .queries
            .prepare_leaf(selector.utf8_len()?)
            .map_err(binding_error)?;
        let source = boundary::bounded_string(selector, SELECTOR_BYTES, "selector")?;
        #[cfg(feature = "selector-registration")]
        let query = draft
            .css_registered(source, &self.selectors)
            .map_err(binding_error)?;
        #[cfg(not(feature = "selector-registration"))]
        let query = draft.css(source).map_err(binding_error)?;
        self.locator
            .locator(query)
            .map(|locator| self.derive_locator(locator))
            .map_err(binding_error)
    }

    #[napi(js_name = "locatorFrom")]
    pub fn locator_from(&self, locator: &NativeLocator) -> napi::Result<NativeLocator> {
        self.locator
            .locator_from(&locator.locator)
            .map(|locator| self.derive_locator(locator))
            .map_err(binding_error)
    }

    #[napi]
    pub fn owner(&self) -> napi::Result<NativeLocator> {
        self.locator
            .owner()
            .map(|locator| self.derive_locator(locator))
            .map_err(binding_error)
    }

    #[napi]
    pub fn nth(&self, index: i32) -> napi::Result<Self> {
        self.locator
            .nth(index)
            .map(|locator| self.derive(locator))
            .map_err(binding_error)
    }

    #[napi(js_name = "frameLocator")]
    pub fn frame_locator(&self, selector: JsString<'_>) -> napi::Result<Self> {
        let draft = self
            .queries
            .prepare_leaf(selector.utf8_len()?)
            .map_err(binding_error)?;
        let source = boundary::bounded_string(selector, SELECTOR_BYTES, "selector")?;
        #[cfg(feature = "selector-registration")]
        let query = draft
            .css_registered(source, &self.selectors)
            .map_err(binding_error)?;
        #[cfg(not(feature = "selector-registration"))]
        let query = draft.css(source).map_err(binding_error)?;
        self.locator
            .frame_locator(query)
            .map(|locator| self.derive(locator))
            .map_err(binding_error)
    }
}

#[cfg(feature = "handles")]
#[napi]
impl NativeLocator {
    #[napi(js_name = "contentFrame")]
    pub fn content_frame(&self) -> napi::Result<NativeFrameLocator> {
        self.locator
            .content_frame()
            .map(|locator| NativeFrameLocator {
                locator,
                publications: self.publications.clone(),
                ffi: self.ffi.clone(),
                wait_admissions: self.wait_admissions.clone(),
                queries: self.queries.clone(),
                conversions: self.conversions.clone(),
                #[cfg(feature = "selector-registration")]
                selectors: self.selectors.clone(),
            })
            .map_err(binding_error)
    }

    #[napi(js_name = "queryElementHandle")]
    pub fn query_element_handle<'env>(
        &self,
        env: &'env Env,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            locator
                .query_element_handle()
                .await
                .map(|handle| match handle {
                    Some(handle) => crate::deferred_conversion::EvaluationResult::Handle(
                        handle,
                        crate::handles::HandlePolicy::default(),
                    ),
                    None => crate::deferred_conversion::EvaluationResult::Value(
                        crate::evaluation_value::EvaluationOutput(
                            pwright_node_host::evaluation::EvaluationValue::Null,
                        ),
                    ),
                })
        })
    }

    #[napi(js_name = "elementHandle")]
    pub fn element_handle<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            locator.element_handle(timeout_ms).await.map(|handle| {
                crate::deferred_conversion::EvaluationResult::Handle(
                    handle,
                    crate::handles::HandlePolicy::default(),
                )
            })
        })
    }

    /// Legacy internal call, whose Locator may already contain `nth(0)`.
    #[napi(js_name = "waitForElement")]
    pub fn wait_for_element<'env>(
        &self,
        env: &'env Env,
        state: JsString<'env>,
        timeout_ms: u32,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        self.wait_for_selector_impl(env, state, timeout_ms, true)
    }

    /// Page/Frame wait keeps strictness in Rust and never derives `first()` in
    /// the JS binding, so XPath and other browser-program selectors can run.
    #[napi(js_name = "waitForSelectorElement")]
    pub fn wait_for_selector_element<'env>(
        &self,
        env: &'env Env,
        state: JsString<'env>,
        timeout_ms: u32,
        strict: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        self.wait_for_selector_impl(env, state, timeout_ms, strict)
    }

    #[napi(js_name = "evaluateHandle")]
    pub fn evaluate_handle<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        converter: Function<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(
                FfiLane::Command,
                pwright_node_host::evaluation::FFI_ALLOWANCE,
            )
            .map_err(binding_error)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        let (id, conversion) = self.conversions.admit(env, converter)?;
        let pending = crate::locator_conversion::PendingGuard {
            registry: self.conversions.clone(),
            id,
        };
        let book = publications.publications();
        let result = boundary::spawn_with_env(
            env,
            execution,
            async move {
                let outcome = locator
                    .evaluate_handle_with(timeout_ms, || conversion.run())
                    .await;
                Ok((outcome, book.fence()?))
            },
            move |env, (outcome, fence)| {
                let result = pending.registry.finish_handle(pending.id, outcome);
                drop(pending);
                publications.publish(env, fence)?;
                result
            },
        );
        if result.is_err() {
            self.conversions.remove(id);
        }
        result
    }
}

#[cfg(feature = "element-handle-dom")]
#[napi]
impl NativeLocator {
    #[napi(js_name = "queryElementHandles")]
    pub fn query_element_handles<'env>(
        &self,
        env: &'env Env,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            locator.query_element_handles().await.map(|handles| {
                crate::handles::HandlesOutput(handles, crate::handles::HandlePolicy::default())
            })
        })
    }
}

#[cfg(feature = "locator-bounding-box")]
#[napi]
impl NativeLocator {
    #[napi(js_name = "boundingBox")]
    pub fn bounding_box<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            locator
                .bounding_box(timeout_ms)
                .await
                .map(|value| value.map(LocatorBoundingBoxOutput::from))
        })
    }
}

#[cfg(feature = "locator-highlight")]
#[napi]
impl NativeLocator {
    #[napi]
    pub fn highlight<'env>(
        &self,
        env: &'env Env,
        description: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let description =
            boundary::bounded_string(description, SELECTOR_BYTES, "Locator description")?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            locator.highlight(description).await
        })
    }
}

#[cfg(feature = "locator-screenshot")]
#[napi]
impl NativeLocator {
    #[napi]
    pub fn screenshot<'env>(
        &self,
        env: &'env Env,
        options: crate::screenshot::NativeScreenshotOptions,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(
                FfiLane::Command,
                pwright_node_host::screenshot::FFI_ALLOWANCE,
            )
            .map_err(binding_error)?;
        let (options, timeout_ms) = crate::screenshot::locator_screenshot_options(options)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            locator
                .screenshot(options, timeout_ms)
                .await
                .map(crate::screenshot::ScreenshotOutput)
        })
    }
}

#[cfg(feature = "locator-dom-actions")]
#[napi]
impl NativeLocator {
    #[napi(js_name = "dispatchEvent")]
    pub fn dispatch_event<'env>(
        &self,
        env: &'env Env,
        event_type: JsString<'env>,
        event_init: Unknown<'env>,
        timeout_ms: u32,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(
                FfiLane::Command,
                pwright_node_host::evaluation::FFI_ALLOWANCE,
            )
            .map_err(binding_error)?;
        let event_type = boundary::bounded_string(
            event_type,
            pwright_node_host::locator::MAX_EVENT_TYPE_BYTES,
            "event type",
        )?;
        let event_init = self.conversions.prepare_input(env, event_init)?;
        #[cfg(all(feature = "handles", feature = "element-handle-dom"))]
        if event_init.has_handles() {
            let event_init = event_init.materialize_owned(env)?;
            let locator = self.locator.clone();
            locator.host().map_err(binding_error)?;
            let publications = self.publications.clone();
            return boundary::spawn_published(env, execution, publications, async move {
                locator
                    .dispatch_event_owned(event_type, event_init, timeout_ms)
                    .await
            });
        }
        let event_init = event_init.materialize(env)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            locator
                .dispatch_event(event_type, event_init, timeout_ms)
                .await
        })
    }

    #[napi(js_name = "selectText")]
    pub fn select_text<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        force: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(
                FfiLane::Command,
                pwright_node_host::evaluation::FFI_ALLOWANCE,
            )
            .map_err(binding_error)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            locator.select_text(timeout_ms, force).await
        })
    }
}

#[napi]
impl NativeLocator {
    #[napi]
    pub fn evaluate<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        converter: Function<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(
                FfiLane::Command,
                pwright_node_host::evaluation::FFI_ALLOWANCE,
            )
            .map_err(binding_error)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        let (id, conversion) = self.conversions.admit(env, converter)?;
        let pending = crate::locator_conversion::PendingGuard {
            registry: self.conversions.clone(),
            id,
        };
        let book = publications.publications();
        let result = boundary::spawn_with_env(
            env,
            execution,
            async move {
                let outcome = locator.evaluate_with(timeout_ms, || conversion.run()).await;
                Ok((outcome, book.fence()?))
            },
            move |env, (outcome, fence)| {
                // Drop Node roots even if publication fails. Rust has already
                // completed native cleanup before returning a conversion error.
                let result = pending.registry.finish(pending.id, outcome);
                drop(pending);
                publications.publish(env, fence)?;
                result
            },
        );
        if result.is_err() {
            self.conversions.remove(id);
        }
        result
    }

    #[napi(js_name = "evaluateFirst")]
    pub fn evaluate_first<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        converter: Function<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(
                FfiLane::Command,
                pwright_node_host::evaluation::FFI_ALLOWANCE,
            )
            .map_err(binding_error)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        let (id, conversion) = self.conversions.admit(env, converter)?;
        let pending = crate::locator_conversion::PendingGuard {
            registry: self.conversions.clone(),
            id,
        };
        let book = publications.publications();
        let result = boundary::spawn_with_env(
            env,
            execution,
            async move {
                let outcome = locator
                    .evaluate_first_with(timeout_ms, || conversion.run())
                    .await;
                Ok((outcome, book.fence()?))
            },
            move |env, (outcome, fence)| {
                let result = pending.registry.finish(pending.id, outcome);
                drop(pending);
                publications.publish(env, fence)?;
                result
            },
        );
        if result.is_err() {
            self.conversions.remove(id);
        }
        result
    }

    #[napi(js_name = "evaluateAll")]
    pub fn evaluate_all<'env>(
        &self,
        env: &'env Env,
        converter: Function<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(
                FfiLane::Command,
                pwright_node_host::evaluation::FFI_ALLOWANCE,
            )
            .map_err(binding_error)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        let (id, conversion) = self.conversions.admit(env, converter)?;
        let pending = crate::locator_conversion::PendingGuard {
            registry: self.conversions.clone(),
            id,
        };
        let book = publications.publications();
        let result = boundary::spawn_with_env(
            env,
            execution,
            async move {
                let outcome = locator.evaluate_all_with(|| conversion.run()).await;
                Ok((outcome, book.fence()?))
            },
            move |env, (outcome, fence)| {
                let result = pending.registry.finish(pending.id, outcome);
                drop(pending);
                publications.publish(env, fence)?;
                result
            },
        );
        if result.is_err() {
            self.conversions.remove(id);
        }
        result
    }

    #[napi]
    pub fn copy(&self) -> napi::Result<Self> {
        self.locator
            .copy()
            .map(|locator| self.derive(locator))
            .map_err(binding_error)
    }

    #[napi]
    pub fn nth(&self, index: i32) -> napi::Result<Self> {
        self.locator
            .nth(index)
            .map(|locator| self.derive(locator))
            .map_err(binding_error)
    }

    #[napi]
    pub fn visible(&self, visible: bool) -> napi::Result<Self> {
        self.locator
            .visible(visible)
            .map(|locator| self.derive(locator))
            .map_err(binding_error)
    }

    #[napi]
    pub fn filter_text(&self, text: JsString<'_>, exclude: bool) -> napi::Result<Self> {
        // Charge the temporary owned string before conversion; composition
        // independently admits its retained copy before cloning the old tree.
        let _input = self
            .queries
            .prepare_leaf(text.utf8_len()?)
            .map_err(binding_error)?;
        let text = boundary::bounded_string(text, SELECTOR_BYTES, "filter text")?;
        self.locator
            .filter_text(&text, exclude)
            .map(|locator| self.derive(locator))
            .map_err(binding_error)
    }

    #[napi]
    pub fn filter_regex(
        &self,
        source: JsString<'_>,
        flags: JsString<'_>,
        exclude: bool,
    ) -> napi::Result<Self> {
        let source_len = source.utf8_len()?;
        let flags_len = flags.utf8_len()?;
        let _input = self
            .queries
            .prepare_leaf(source_len.saturating_add(flags_len))
            .map_err(binding_error)?;
        let source = boundary::bounded_string(source, SELECTOR_BYTES, "filter RegExp source")?;
        let flags = boundary::bounded_string(flags, 64, "filter RegExp flags")?;
        self.locator
            .filter_regex(&source, &flags, exclude)
            .map(|locator| self.derive(locator))
            .map_err(binding_error)
    }

    #[napi]
    pub fn combine(&self, other: &NativeLocator, kind: JsString<'_>) -> napi::Result<Self> {
        let kind = boundary::bounded_string(kind, 16, "combination")?;
        let kind = match kind.as_str() {
            "and" => QueryCombination::And,
            "or" => QueryCombination::Or,
            "within" => QueryCombination::Within,
            "has" => QueryCombination::Has,
            "hasNot" => QueryCombination::HasNot,
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "invalid Locator combination".into(),
                )));
            }
        };
        self.locator
            .combine(&other.locator, kind)
            .map(|locator| self.derive(locator))
            .map_err(binding_error)
    }

    #[napi]
    pub fn read<'env>(
        &self,
        env: &'env Env,
        operation: JsString<'env>,
        timeout_ms: u32,
        attribute: Option<JsString<'env>>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let operation = boundary::bounded_string(operation, 32, "Locator operation")?;
        let operation = LocatorRead::from_str(&operation).map_err(binding_error)?;
        let attribute = attribute
            .map(|name| boundary::bounded_string(name, SELECTOR_BYTES, "attribute name"))
            .transpose()?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            let value = locator.read(operation, timeout_ms, attribute).await?;
            Ok(match value {
                LocatorValue::Count(value) => Either4::A(value),
                LocatorValue::Bool(value) => Either4::B(value),
                LocatorValue::Text(value) => Either4::C(value),
                LocatorValue::Texts(value) => Either4::D(value),
            })
        })
    }

    #[napi(js_name = "countWithHook")]
    pub fn count_with_hook<'env>(
        &self,
        env: &'env Env,
        hook: Function<'env, (), Promise<()>>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let hook = hook
            .build_threadsafe_function::<()>()
            .callee_handled::<false>()
            .max_queue_size::<1>()
            .build()?;
        let hook = Arc::new(hook);
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            locator
                .count_with_before_query(move || {
                    let hook = Arc::clone(&hook);
                    async move {
                        let promise = hook.call_async_catch(()).await.map_err(|error| {
                            HostError::Backend(format!("Node query hook failed: {error}"))
                        })?;
                        promise.await.map_err(|error| {
                            HostError::Backend(format!("Node query hook failed: {error}"))
                        })
                    }
                })
                .await
        })
    }

    #[napi]
    pub fn wait_for<'env>(
        &self,
        env: &'env Env,
        state: JsString<'env>,
        timeout_ms: u32,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let state = boundary::bounded_string(state, 16, "Locator wait state")?;
        let state = match state.as_str() {
            "attached" => LocatorWaitState::Attached,
            "detached" => LocatorWaitState::Detached,
            "visible" => LocatorWaitState::Visible,
            "hidden" => LocatorWaitState::Hidden,
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "invalid Locator wait state".into(),
                )));
            }
        };
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            locator.wait_for(state, timeout_ms).await
        })
    }

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn expect_text<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        expected: JsString<'env>,
        substring: bool,
        normalize_whitespace: bool,
        ignore_case: bool,
        use_inner_text: bool,
        is_not: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let expected = boundary::bounded_string(expected, SELECTOR_BYTES, "expected text")?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            let result = locator
                .expect_text(
                    timeout_ms,
                    TextExpectation {
                        expected: &expected,
                        substring,
                        normalize_whitespace,
                        ignore_case,
                        use_inner_text,
                        is_not,
                    },
                )
                .await?;
            Ok(LocatorExpectationResult {
                matches: result.matches,
                received: result.received,
                preview: result.preview,
                missing: result.missing,
                timed_out: result.timed_out,
            })
        })
    }

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn expect_text_regex<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        source: JsString<'env>,
        flags: JsString<'env>,
        ignore_case: Option<bool>,
        use_inner_text: bool,
        is_not: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let source = boundary::bounded_string(source, SELECTOR_BYTES, "RegExp source")?;
        let flags = boundary::bounded_string(flags, 64, "RegExp flags")?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            let result = locator
                .expect_text_regex(
                    timeout_ms,
                    &source,
                    &flags,
                    ignore_case,
                    use_inner_text,
                    is_not,
                )
                .await?;
            Ok(LocatorExpectationResult {
                matches: result.matches,
                received: result.received,
                preview: result.preview,
                missing: result.missing,
                timed_out: result.timed_out,
            })
        })
    }

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn expect_texts<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        expected: Vec<String>,
        substring: bool,
        normalize_whitespace: bool,
        ignore_case: bool,
        use_inner_text: bool,
        is_not: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        if expected.len() > 1024 {
            return Err(binding_error(HostError::ResourceExhausted(
                "expected text collection exceeds 1024 items".into(),
            )));
        }
        let total_bytes = expected
            .iter()
            .try_fold(0usize, |total, value| total.checked_add(value.len()));
        if total_bytes.is_none_or(|total| total > 1024 * 1024) {
            return Err(binding_error(HostError::ResourceExhausted(
                "expected text collection exceeds 1048576 UTF-8 bytes".into(),
            )));
        }
        if expected.iter().any(|value| value.len() > SELECTOR_BYTES) {
            return Err(binding_error(HostError::InvalidArgument(
                "text expectation item exceeds its string boundary".into(),
            )));
        }
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            let result = locator
                .expect_texts(
                    timeout_ms,
                    TextsExpectation {
                        expected: &expected,
                        substring,
                        normalize_whitespace,
                        ignore_case,
                        use_inner_text,
                        is_not,
                    },
                )
                .await?;
            Ok(LocatorTextsExpectationResult {
                matches: result.matches,
                received: result.received,
                timed_out: result.timed_out,
            })
        })
    }

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn expect_string<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        kind: JsString<'env>,
        expected: JsString<'env>,
        flags: Option<JsString<'env>>,
        is_not: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let kind = boundary::bounded_string(kind, 16, "string expectation kind")?;
        let kind = match kind.as_str() {
            "class" => LocatorStringExpectationKind::Class,
            "value" => LocatorStringExpectationKind::Value,
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "invalid string expectation kind".into(),
                )));
            }
        };
        let expected = boundary::bounded_string(expected, SELECTOR_BYTES, "expected string")?;
        let flags = flags
            .map(|flags| boundary::bounded_string(flags, 64, "string expectation RegExp flags"))
            .transpose()?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            locator
                .expect_string(
                    timeout_ms,
                    kind,
                    StringMatcher {
                        expected: &expected,
                        flags: flags.as_deref(),
                    },
                    is_not,
                )
                .await
                .map(LocatorValueExpectationOutput)
        })
    }

    #[napi]
    pub fn expect_class_matchers<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        expected: Vec<LocatorStringMatcherInput>,
        is_not: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        if expected.len() > 1024 {
            return Err(binding_error(HostError::ResourceExhausted(
                "expected class collection exceeds 1024 items".into(),
            )));
        }
        let total_bytes = expected.iter().try_fold(0usize, |total, matcher| {
            total
                .checked_add(matcher.expected.len())
                .and_then(|total| total.checked_add(matcher.flags.as_ref().map_or(0, String::len)))
        });
        if total_bytes.is_none_or(|total| total > 1024 * 1024) {
            return Err(binding_error(HostError::ResourceExhausted(
                "expected class collection exceeds 1048576 UTF-8 bytes".into(),
            )));
        }
        if expected.iter().any(|matcher| {
            matcher.expected.len() > SELECTOR_BYTES
                || matcher.flags.as_ref().is_some_and(|flags| flags.len() > 64)
        }) {
            return Err(binding_error(HostError::InvalidArgument(
                "class expectation item exceeds its string boundary".into(),
            )));
        }
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            let matchers = expected
                .iter()
                .map(|matcher| StringMatcher {
                    expected: &matcher.expected,
                    flags: matcher.flags.as_deref(),
                })
                .collect::<Vec<_>>();
            let result = locator
                .expect_class_matchers(timeout_ms, &matchers, is_not)
                .await?;
            Ok(LocatorTextsExpectationResult {
                matches: result.matches,
                received: result.received,
                timed_out: result.timed_out,
            })
        })
    }

    #[napi]
    pub fn expect_state<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        state: JsString<'env>,
        is_not: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let state = boundary::bounded_string(state, 16, "Locator expectation state")?;
        let state = match state.as_str() {
            "attached" => LocatorExpectationState::Attached,
            "detached" => LocatorExpectationState::Detached,
            "empty" => LocatorExpectationState::Empty,
            "editable" => LocatorExpectationState::Editable,
            "readonly" => LocatorExpectationState::Readonly,
            "enabled" => LocatorExpectationState::Enabled,
            "disabled" => LocatorExpectationState::Disabled,
            "focused" => LocatorExpectationState::Focused,
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "invalid Locator expectation state".into(),
                )));
            }
        };
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            let result = locator.expect_state(timeout_ms, state, is_not).await?;
            Ok(LocatorExpectationResult {
                matches: result.matches,
                received: result.received,
                preview: result.preview,
                missing: result.missing,
                timed_out: result.timed_out,
            })
        })
    }

    #[napi]
    pub fn expect_checked<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        checked: Option<bool>,
        indeterminate: Option<bool>,
        is_not: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let state = if indeterminate == Some(true) {
            if checked.is_some() {
                return Err(binding_error(HostError::InvalidArgument(
                    "Can't assert indeterminate and checked at the same time".into(),
                )));
            }
            LocatorExpectationState::Indeterminate
        } else if checked == Some(false) {
            LocatorExpectationState::Unchecked
        } else {
            LocatorExpectationState::Checked
        };
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            let result = locator.expect_state(timeout_ms, state, is_not).await?;
            Ok(LocatorExpectationResult {
                matches: result.matches,
                received: result.received,
                preview: result.preview,
                missing: result.missing,
                timed_out: result.timed_out,
            })
        })
    }

    #[napi]
    pub fn expect_property<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        name: JsString<'env>,
        expected: Unknown<'env>,
        is_not: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let name = boundary::bounded_string(name, SELECTOR_BYTES, "property name")?;
        let expected = self.conversions.prepare_value(env, expected)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            locator
                .expect_property(timeout_ms, &name, expected, is_not)
                .await
                .map(LocatorValueExpectationOutput)
        })
    }

    #[napi]
    pub fn expect_visibility<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        visible: bool,
        is_not: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            let result = locator
                .expect_visibility(timeout_ms, visible, is_not)
                .await?;
            Ok(LocatorVisibilityExpectationResult {
                matches: result.matches,
                received: result.received,
                preview: result.preview,
                missing: result.missing,
                timed_out: result.timed_out,
            })
        })
    }

    #[napi]
    pub fn expect_count<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
        expected: u32,
        is_not: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, READ_BYTES)
            .map_err(binding_error)?;
        let locator = self.locator.clone();
        locator.host().map_err(binding_error)?;
        let publications = self.publications.clone();
        boundary::spawn_published(env, execution, publications, async move {
            let result = locator.expect_count(timeout_ms, expected, is_not).await?;
            Ok(LocatorCountExpectationResult {
                matches: result.matches,
                received: result.received,
                timed_out: result.timed_out,
            })
        })
    }
}
