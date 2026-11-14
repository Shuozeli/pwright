//! Synchronous published metadata only; no browser or runtime access.
use napi::{Env, JsString, Unknown, bindgen_prelude::ToNapiValue};
use napi_derive::napi;
use pwright_node_host::frame_tree::{FrameBinding, FrameTreeBinding};

use crate::binding_error;

mod glob;
mod lookup;

#[napi]
pub struct NativeFrameTree {
    pub(crate) binding: FrameTreeBinding,
}

#[napi(object, object_from_js = false)]
pub struct FrameIdentity {
    pub id: String,
    pub parent_id: Option<String>,
}

#[napi]
impl NativeFrameTree {
    #[napi]
    pub fn is_closed(&self) -> napi::Result<bool> {
        self.binding.tree().is_closed().map_err(binding_error)
    }

    #[napi]
    pub fn frames<'env>(&self, env: &'env Env) -> napi::Result<Unknown<'env>> {
        // Two u64 decimal strings, descriptor storage and Vec element storage.
        // V8 objects are JS-owned; this bounds temporary native conversion bytes.
        let read = self
            .binding
            .tree()
            .read(std::mem::size_of::<FrameIdentity>() + 40)
            .map_err(binding_error)?;
        let identities: Vec<_> = read
            .frames()
            .iter()
            .map(|node| FrameIdentity {
                id: node.id().to_string(),
                parent_id: node.parent_id().map(|id| id.to_string()),
            })
            .collect();
        let result = identities.into_unknown(env);
        drop(read);
        result
    }

    #[napi]
    pub fn metadata(&self, id: JsString<'_>) -> napi::Result<NativeFrameNode> {
        if id.utf8_len()? > 20 {
            return Err(napi::Error::from_reason("Invalid Frame ID"));
        }
        // Admit the bounded ID copy before allocating UTF-8 input storage.
        // The returned binding separately carries its own last-reference lease.
        let _read = self.binding.tree().read(21).map_err(binding_error)?;
        let id = id
            .into_utf8()?
            .as_str()?
            .parse::<u64>()
            .map_err(|_| napi::Error::from_reason("Invalid Frame ID"))?;
        self.binding
            .tree()
            .bind(id)
            .map(|binding| NativeFrameNode { binding })
            .map_err(binding_error)
    }
}

#[cfg(feature = "frame-detach-events")]
#[napi]
impl NativeFrameTree {
    #[napi]
    pub fn retired_metadata(&self, id: JsString<'_>) -> napi::Result<NativeFrameNode> {
        if id.utf8_len()? > 20 {
            return Err(napi::Error::from_reason("Invalid Frame ID"));
        }
        // The child Frame owns a one-shot binding and 21-byte ID allowance
        // admitted before its topology retirement.
        let id = id
            .into_utf8()?
            .as_str()?
            .parse::<u64>()
            .map_err(|_| napi::Error::from_reason("Invalid Frame ID"))?;
        self.binding
            .tree()
            .bind_retired(id)
            .map(|binding| NativeFrameNode { binding })
            .map_err(binding_error)
    }

    #[napi]
    pub fn event_metadata(&self, id: JsString<'_>) -> napi::Result<NativeFrameNode> {
        if id.utf8_len()? > 20 {
            return Err(napi::Error::from_reason("Invalid Frame ID"));
        }
        let id = id
            .into_utf8()?
            .as_str()?
            .parse::<u64>()
            .map_err(|_| napi::Error::from_reason("Invalid Frame ID"))?;
        self.binding
            .tree()
            .bind_detach_ancestor(id)
            .map(|binding| NativeFrameNode { binding })
            .map_err(binding_error)
    }

    #[napi]
    pub fn finish_detached(&self, id: JsString<'_>) -> napi::Result<()> {
        if id.utf8_len()? > 20 {
            return Err(napi::Error::from_reason("Invalid Frame ID"));
        }
        let id = id
            .into_utf8()?
            .as_str()?
            .parse::<u64>()
            .map_err(|_| napi::Error::from_reason("Invalid Frame ID"))?;
        self.binding
            .tree()
            .finish_detached(id)
            .map_err(binding_error)
    }
}

#[napi]
pub struct NativeFrameNode {
    pub(crate) binding: FrameBinding,
}

#[napi]
impl NativeFrameNode {
    #[napi]
    pub fn name<'env>(&self, env: &'env Env) -> napi::Result<JsString<'env>> {
        self.binding
            .node()
            .with_name(|name| env.create_string(name))
            .map_err(binding_error)?
    }

    #[napi]
    pub fn url(&self) -> napi::Result<String> {
        self.binding.node().url().map_err(binding_error)
    }
    #[napi]
    pub fn has_unpublished_url(&self) -> napi::Result<bool> {
        self.binding
            .node()
            .has_unpublished_url()
            .map_err(binding_error)
    }
    #[napi]
    pub fn is_detached(&self) -> bool {
        self.binding.node().is_detached()
    }
}

#[cfg(feature = "frame-detach-events")]
#[napi]
impl NativeFrameNode {
    #[napi]
    pub fn parent_id(&self) -> Option<String> {
        self.binding.node().parent_id().map(|id| id.to_string())
    }
}
