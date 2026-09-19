//! Bounded synchronous descriptor conversion; native tree wrappers have their
//! own metadata-only last-reference leases, independent of this read guard.
use crate::{NativeSession, PageDescriptor, binding_error, boundary};
use napi::{
    Env, JsString, Unknown,
    bindgen_prelude::{Array, ToNapiValue},
};
use napi_derive::napi;

// At most 16 live Pages/Contexts and 256 retained cache IDs. Covers both host
// and addon DTO vectors, decimal u64 IDs and foreign input copies.
// Native wrapper bodies have separate lasting reservations; V8 storage is not RSS-accounted here.
pub(crate) const CONVERSION_BYTES: usize = 32 * 1024;
// Compile-time proof follows DTO layout changes instead of trusting this comment.
const _: () = assert!(
    CONVERSION_BYTES
        >= 16
            * (std::mem::size_of::<pwright_node_host::PageDescriptor>()
                + std::mem::size_of::<PageDescriptor>()
                + 6 * (std::mem::size_of::<String>() + 32))
            + 256 * (std::mem::size_of::<String>() + 32)
            + 1024
);
// Context and Page reads never coexist in one conversion; prove each separately.
const _: () = assert!(
    CONVERSION_BYTES
        >= 16
            * (std::mem::size_of::<pwright_node_host::ContextDescriptor>()
                + std::mem::size_of::<crate::ContextDescriptor>()
                + 4 * (std::mem::size_of::<String>() + 32))
            + 256 * (std::mem::size_of::<String>() + 32)
            + 1024
);

#[napi(object, object_from_js = false)]
pub struct PageSnapshot {
    pub ids: Vec<String>,
    pub new_pages: Vec<PageDescriptor>,
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn page_snapshot<'env>(
        &self,
        env: &'env Env,
        context_id: JsString<'_>,
        known: Array<'_>,
    ) -> napi::Result<Unknown<'env>> {
        let guard = self
            .host
            .reserve_metadata_conversion(CONVERSION_BYTES)
            .map_err(binding_error)?;
        if known.len() > 256 {
            return Err(binding_error(
                pwright_node_host::HostError::InvalidArgument(
                    "Known Page ID count exceeds 256".into(),
                ),
            ));
        }
        let context_id = boundary::bounded_string(context_id, 20, "context ID")?;
        let mut ids = Vec::with_capacity(known.len() as usize);
        for index in 0..known.len() {
            let id = known
                .get::<JsString<'_>>(index)?
                .ok_or_else(|| napi::Error::from_reason("Missing cached Page ID"))?;
            ids.push(boundary::bounded_string(id, 20, "Page ID")?);
        }
        let snapshot = self
            .host
            .published_page_snapshot(&context_id, &ids)
            .map_err(binding_error)?;
        let value = PageSnapshot {
            ids: snapshot.ids,
            new_pages: snapshot
                .new_pages
                .into_iter()
                .map(PageDescriptor::from)
                .collect(),
        };
        let result = value.into_unknown(env);
        guard.finish();
        result
    }
}
