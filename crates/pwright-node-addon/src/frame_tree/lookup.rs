//! Rust selection over one pinned metadata revision; Node owns language values.
use napi::{
    Env, JsString, Unknown,
    bindgen_prelude::{Array, ToNapiValue},
};
use napi_derive::napi;
use pwright_node_host::{
    HostError,
    frame_lookup::{FrameLookup, LookupError},
};

use super::{NativeFrameNode, NativeFrameTree};
use crate::{binding_error, boundary, context_settings::NativeContextSettings};

mod values;

const MAX_KNOWN: usize = 256;
const MAX_PATH: usize = 128;
const CONVERSION_BYTES: usize = 32 * 1024;
// Fixed output capacity, numeric cache hints, one temporary decimal ID copy,
// returned selection ID, Vec bodies and conversion bookkeeping. Native binding
// bodies additionally retain their own leases through finalization.
const _: () = assert!(
    CONVERSION_BYTES
        >= MAX_PATH * (std::mem::size_of::<SelectedFrame>() + 40)
            + MAX_KNOWN * std::mem::size_of::<u64>()
            + 2048
);
const _: () = assert!(
    std::mem::size_of::<NativeFrameNode>()
        == std::mem::size_of::<pwright_node_host::frame_tree::FrameBinding>()
);

#[napi(object, object_from_js = false)]
pub struct SelectedFrame {
    pub id: String,
    pub parent_id: Option<String>,
    pub binding: NativeFrameNode,
}

#[napi(object, object_from_js = false)]
pub struct FrameSelection {
    pub id: String,
    pub new_frames: Vec<SelectedFrame>,
}

fn lookup_error(error: LookupError<napi::Error>) -> napi::Error {
    match error {
        LookupError::Metadata(error) => binding_error(error),
        LookupError::Matcher(error) => error,
    }
}

#[napi]
impl NativeFrameTree {
    #[napi]
    pub fn lookup<'env>(
        &self,
        env: &'env Env,
        name: Unknown<'env>,
        url: Unknown<'env>,
        settings: &NativeContextSettings,
        known: Array<'_>,
    ) -> napi::Result<Unknown<'env>> {
        let _conversion = self
            .binding
            .tree()
            .lookup_resources()
            .reserve_conversion(CONVERSION_BYTES)
            .map_err(binding_error)?;
        let by_name = values::truthy(name)?;
        if !by_name && !values::truthy(url)? {
            return Err(napi::Error::from_reason(
                "Either name or url matcher should be specified",
            ));
        }
        if known.len() as usize > MAX_KNOWN {
            return Err(binding_error(HostError::InvalidArgument(
                "Known Frame ID count exceeds 256".into(),
            )));
        }
        let mut known_ids = Vec::with_capacity(known.len() as usize);
        for index in 0..known.len() {
            let id = known
                .get::<JsString<'_>>(index)?
                .ok_or_else(|| napi::Error::from_reason("Missing cached Frame ID"))?;
            known_ids.push(
                boundary::bounded_string(id, 20, "Frame ID")?
                    .parse::<u64>()
                    .map_err(|_| napi::Error::from_reason("Invalid Frame ID"))?,
            );
        }
        // Pin membership before any caller-supplied RegExp/URL/predicate code.
        let lookup = FrameLookup::new(self.binding.tree()).map_err(binding_error)?;
        let selected = if by_name {
            lookup.first_matching(|node| {
                node.with_name(|text| env.strict_equals(env.create_string(text)?, name))
                    .map_err(binding_error)?
            })?
        } else {
            lookup
                .by_url(|text| values::matches(self, settings, env, url, text))
                .map_err(lookup_error)?
        };
        let Some(selected) = selected else {
            return Option::<FrameSelection>::None.into_unknown(env);
        };
        let mut new_frames = Vec::with_capacity(MAX_PATH);
        lookup
            .visit_path(selected.id(), |node| {
                if known_ids.contains(&node.id().get()) {
                    return Ok(false);
                }
                if new_frames.len() == MAX_PATH {
                    return Err(binding_error(HostError::ResourceExhausted(
                        "Frame ancestry exceeds 128".into(),
                    )));
                }
                new_frames.push(SelectedFrame {
                    id: node.id().to_string(),
                    parent_id: node.parent_id().map(|id| id.to_string()),
                    binding: NativeFrameNode {
                        binding: lookup.bind(node.id()).map_err(binding_error)?,
                    },
                });
                Ok(true)
            })
            .map_err(lookup_error)?;
        new_frames.reverse();
        FrameSelection {
            id: selected.id().to_string(),
            new_frames,
        }
        .into_unknown(env)
    }
}
