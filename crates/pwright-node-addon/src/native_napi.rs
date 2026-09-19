//! Module-local class identity for the pinned napi-derive conversion boundary.
//!
//! The crate aliases itself as `napi` and reexports this facade. Generated TypeTag
//! implementations then use this function, while constructors, receivers and
//! typed arguments still use the real napi traits and their existing checks.
//! This relies on the audited lexical paths in napi-derive 3.6.3; upgrades must
//! recheck the boundary gate and independently loaded addon tests.

pub mod bindgen_prelude {
    pub use napi_runtime::bindgen_prelude::*;

    /// Rust class layouts are not an ABI, even between identical package versions.
    pub fn type_tag_from_ident(identity: &str) -> sys::napi_type_tag {
        sys::napi_type_tag {
            lower: super::module_identity(),
            upper: napi_runtime::bindgen_prelude::type_tag_from_ident(identity).upper,
        }
    }
}

// Interior mutability prevents constant merging; non-zero size gives each
// independently loaded image its own address. Only identity is observed: no
// pointer is reconstructed or dereferenced from a tag, or exposed to JavaScript.
static MODULE_IDENTITY: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub(crate) fn module_identity() -> u64 {
    std::ptr::addr_of!(MODULE_IDENTITY).addr() as u64
}
