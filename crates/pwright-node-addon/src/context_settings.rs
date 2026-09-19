//! Metadata-only Context wrappers. No browser/runtime ownership crosses here.
use napi::{
    Env, JsString, Unknown,
    bindgen_prelude::{Array, ToNapiValue},
};
use napi_derive::napi;
use pwright_node_host::{HostError, context_metadata::ContextBinding};

use crate::{
    ContextDescriptor, NativeSession, binding_error, boundary, page_snapshot::CONVERSION_BYTES,
};

pub(crate) fn read_base_url(
    env: &Env,
    value: Option<JsString<'_>>,
) -> napi::Result<Option<String>> {
    value
        .map(|value| {
            if value.utf8_len()? > 16 * 1024 {
                // This synchronous boundary precedes the JS Promise mapper.
                // Throw the contract's TypeError here without catching user
                // exceptions from the independent header conversion path.
                env.throw_type_error(
                    "browser.newContext: baseURL exceeds 16384 UTF-8 bytes",
                    Some("PWRIGHT_INVALID_ARGUMENT"),
                )?;
                return Err(napi::Error::new(
                    napi::Status::PendingException,
                    "Context baseURL exceeds byte limit",
                ));
            }
            boundary::bounded_string(value, 16 * 1024, "Context baseURL")
        })
        .transpose()
}

#[napi]
pub struct NativeContextSettings {
    pub(crate) binding: ContextBinding,
}
// The host pre-admits the complete native wrapper before Context creation.
const _: () =
    assert!(std::mem::size_of::<NativeContextSettings>() == std::mem::size_of::<ContextBinding>());

impl TryFrom<pwright_node_host::ContextDescriptor> for ContextDescriptor {
    type Error = HostError;
    fn try_from(value: pwright_node_host::ContextDescriptor) -> Result<Self, Self::Error> {
        let binding = match value.settings_binding {
            Some(binding) => binding,
            None => value.settings.bind()?,
        };
        Ok(Self {
            id: value.id,
            settings: NativeContextSettings { binding },
        })
    }
}

#[napi]
impl NativeContextSettings {
    #[napi]
    pub fn resolve_url<'env>(
        &self,
        env: &'env Env,
        input: JsString<'env>,
    ) -> napi::Result<JsString<'env>> {
        // The Node URL parser owns its scoped foreign values. No Rust input or
        // output copy is made; only the configured base is converted to Node.
        if input.utf8_len()? > 16 * 1024 {
            return Err(binding_error(HostError::InvalidArgument(
                "Navigation URL exceeds byte limit".into(),
            )));
        }
        let snapshot = self.binding.snapshot();
        let _guard = snapshot
            .reserve_conversion(CONVERSION_BYTES)
            .map_err(binding_error)?;
        let base = snapshot
            .base_url()
            .map(|value| env.create_string(value))
            .transpose()?;
        let result = crate::url_value::resolve(env, input, base)?
            .map(|(serialized, _)| serialized)
            .unwrap_or(input);
        if result.utf8_len()? > 16 * 1024 {
            return Err(binding_error(HostError::InvalidArgument(
                "Resolved navigation URL exceeds byte limit".into(),
            )));
        }
        Ok(result)
    }
}

#[napi(object, object_from_js = false)]
pub struct ContextSnapshot {
    pub ids: Vec<String>,
    pub new_contexts: Vec<ContextDescriptor>,
}

#[napi(object)]
pub struct ContextMetadataMetrics {
    pub states: u32,
    pub conversions: u32,
    pub bytes: u32,
    pub active_contexts: u32,
    pub pending_creations: u32,
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn context_snapshot<'env>(
        &self,
        env: &'env Env,
        known: Array<'_>,
    ) -> napi::Result<Unknown<'env>> {
        let guard = self
            .host
            .reserve_context_conversion(CONVERSION_BYTES)
            .map_err(binding_error)?;
        if known.len() > 256 {
            return Err(binding_error(HostError::InvalidArgument(
                "Known Context ID count exceeds 256".into(),
            )));
        }
        let mut known_ids = Vec::with_capacity(known.len() as usize);
        for index in 0..known.len() {
            let id = known
                .get::<JsString<'_>>(index)?
                .ok_or_else(|| napi::Error::from_reason("Missing cached Context ID"))?;
            known_ids.push(boundary::bounded_string(id, 20, "Context ID")?);
        }
        let contexts = self.host.published_contexts().map_err(binding_error)?;
        let mut ids = Vec::with_capacity(contexts.len());
        let mut new_contexts = Vec::with_capacity(contexts.len());
        for context in contexts {
            ids.push(context.id.clone());
            if !known_ids.contains(&context.id) {
                new_contexts.push(ContextDescriptor::try_from(context).map_err(binding_error)?);
            }
        }
        let result = ContextSnapshot { ids, new_contexts }.into_unknown(env);
        guard.finish();
        result
    }

    #[napi]
    pub fn context_metadata_metrics(&self) -> napi::Result<ContextMetadataMetrics> {
        let metrics = self
            .host
            .context_metadata_metrics()
            .map_err(binding_error)?;
        Ok(ContextMetadataMetrics {
            states: metrics.states as u32,
            conversions: metrics.conversions as u32,
            bytes: metrics.bytes as u32,
            active_contexts: metrics.active_contexts as u32,
            pending_creations: metrics.pending_creations as u32,
        })
    }
}
