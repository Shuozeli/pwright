//! Centralized JavaScript snippets for CDP Runtime.evaluate / callFunctionOn.
//!
//! All JS strings used by pwright live here for:
//! - **Auditability** — one place to review all code injected into pages
//! - **Testability** — unit tests verify snippet correctness
//! - **Reuse** — shared across bridge, playwright API, and future modules

/// DOM query snippets.
pub mod dom;
/// Element interaction snippets (scrollBy, setValue, etc.).
pub mod element;
/// Page state snippets (readyState, location, etc.).
pub mod page;
