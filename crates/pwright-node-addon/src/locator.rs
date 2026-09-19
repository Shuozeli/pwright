//! Thin Node value bindings; query ownership and all DOM reads remain Rust.

mod actions;
#[cfg(feature = "file-input")]
mod file_input;
mod semantic;

use std::str::FromStr;

use napi::{
    Env, JsString, Unknown,
    bindgen_prelude::{Either4, Function, PromiseRaw},
};
use napi_derive::napi;
use pwright_node_host::{
    HostError,
    ffi_budget::{FfiBudget, FfiLane},
    locator::{HostLocator, LocatorRead, LocatorValue},
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

#[napi]
pub struct NativeLocator {
    publications: crate::publication_port::PublicationPort,
    locator: HostLocator,
    ffi: FfiBudget,
    queries: QueryBudget,
    conversions: crate::locator_conversion::LocatorConversions,
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
        let query = draft.css(source).map_err(binding_error)?;
        Ok(NativeLocator {
            locator: self.host.locator(&frame_id, query).map_err(binding_error)?,
            publications: self.publications.clone(),
            ffi: self.ffi.clone(),
            queries: self.queries.clone(),
            conversions: self.locator_conversions.clone(),
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

impl NativeLocator {
    fn derive(&self, locator: HostLocator) -> Self {
        Self {
            locator,
            publications: self.publications.clone(),
            ffi: self.ffi.clone(),
            queries: self.queries.clone(),
            conversions: self.conversions.clone(),
        }
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
}
