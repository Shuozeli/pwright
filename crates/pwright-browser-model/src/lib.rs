//! Canonical identities for the base Playwright-shaped browser object graph.
//!
//! This crate contains no browser process, backend, transport, codec, runtime,
//! or executor. Capability and client crates can share these identities without
//! pulling in Node.js or a browser binary.

use pwright_core::{
    ObjectKindCatalog, ObjectKindDescriptor, ObjectKindId, ObjectKindSpec, ObjectKindTag,
};

pub const BROWSER_OBJECT_KIND_ID: ObjectKindId = ObjectKindId::new("pwright.browser.browser");
pub const BROWSER_CONTEXT_OBJECT_KIND_ID: ObjectKindId =
    ObjectKindId::new("pwright.browser.browser_context");
pub const PAGE_OBJECT_KIND_ID: ObjectKindId = ObjectKindId::new("pwright.browser.page");
pub const FRAME_OBJECT_KIND_ID: ObjectKindId = ObjectKindId::new("pwright.browser.frame");
pub const JS_HANDLE_OBJECT_KIND_ID: ObjectKindId = ObjectKindId::new("pwright.browser.js_handle");
pub const ELEMENT_HANDLE_OBJECT_KIND_ID: ObjectKindId =
    ObjectKindId::new("pwright.browser.element_handle");
pub const REQUEST_OBJECT_KIND_ID: ObjectKindId = ObjectKindId::new("pwright.browser.request");
pub const RESPONSE_OBJECT_KIND_ID: ObjectKindId = ObjectKindId::new("pwright.browser.response");
pub const ARTIFACT_OBJECT_KIND_ID: ObjectKindId = ObjectKindId::new("pwright.browser.artifact");
pub const STREAM_OBJECT_KIND_ID: ObjectKindId = ObjectKindId::new("pwright.browser.stream");

const BROWSER_CONTEXT_PARENT_KINDS: &[ObjectKindId] = &[BROWSER_OBJECT_KIND_ID];
const PAGE_PARENT_KINDS: &[ObjectKindId] = &[BROWSER_CONTEXT_OBJECT_KIND_ID];
const FRAME_PARENT_KINDS: &[ObjectKindId] = &[FRAME_OBJECT_KIND_ID, PAGE_OBJECT_KIND_ID];
const HANDLE_PARENT_KINDS: &[ObjectKindId] = &[FRAME_OBJECT_KIND_ID];
const REQUEST_PARENT_KINDS: &[ObjectKindId] = &[
    BROWSER_CONTEXT_OBJECT_KIND_ID,
    FRAME_OBJECT_KIND_ID,
    PAGE_OBJECT_KIND_ID,
];
const RESPONSE_PARENT_KINDS: &[ObjectKindId] = &[REQUEST_OBJECT_KIND_ID];
const ARTIFACT_PARENT_KINDS: &[ObjectKindId] = &[
    BROWSER_OBJECT_KIND_ID,
    BROWSER_CONTEXT_OBJECT_KIND_ID,
    PAGE_OBJECT_KIND_ID,
];
const STREAM_PARENT_KINDS: &[ObjectKindId] = &[ARTIFACT_OBJECT_KIND_ID];

pub const BROWSER_OBJECT_KIND: ObjectKindDescriptor =
    ObjectKindDescriptor::new(BROWSER_OBJECT_KIND_ID, ObjectKindTag::new(1), "Browser");
pub const BROWSER_CONTEXT_OBJECT_KIND: ObjectKindDescriptor = ObjectKindDescriptor::new(
    BROWSER_CONTEXT_OBJECT_KIND_ID,
    ObjectKindTag::new(2),
    "BrowserContext",
)
.with_parent_kinds(BROWSER_CONTEXT_PARENT_KINDS);
pub const PAGE_OBJECT_KIND: ObjectKindDescriptor =
    ObjectKindDescriptor::new(PAGE_OBJECT_KIND_ID, ObjectKindTag::new(3), "Page")
        .with_parent_kinds(PAGE_PARENT_KINDS);
pub const FRAME_OBJECT_KIND: ObjectKindDescriptor =
    ObjectKindDescriptor::new(FRAME_OBJECT_KIND_ID, ObjectKindTag::new(4), "Frame")
        .with_parent_kinds(FRAME_PARENT_KINDS);
pub const JS_HANDLE_OBJECT_KIND: ObjectKindDescriptor =
    ObjectKindDescriptor::new(JS_HANDLE_OBJECT_KIND_ID, ObjectKindTag::new(5), "JSHandle")
        .with_parent_kinds(HANDLE_PARENT_KINDS);
pub const ELEMENT_HANDLE_OBJECT_KIND: ObjectKindDescriptor = ObjectKindDescriptor::new(
    ELEMENT_HANDLE_OBJECT_KIND_ID,
    ObjectKindTag::new(6),
    "ElementHandle",
)
.with_parent_kinds(HANDLE_PARENT_KINDS);
pub const REQUEST_OBJECT_KIND: ObjectKindDescriptor =
    ObjectKindDescriptor::new(REQUEST_OBJECT_KIND_ID, ObjectKindTag::new(7), "Request")
        .with_parent_kinds(REQUEST_PARENT_KINDS);
pub const RESPONSE_OBJECT_KIND: ObjectKindDescriptor =
    ObjectKindDescriptor::new(RESPONSE_OBJECT_KIND_ID, ObjectKindTag::new(8), "Response")
        .with_parent_kinds(RESPONSE_PARENT_KINDS);
pub const ARTIFACT_OBJECT_KIND: ObjectKindDescriptor =
    ObjectKindDescriptor::new(ARTIFACT_OBJECT_KIND_ID, ObjectKindTag::new(9), "Artifact")
        .with_parent_kinds(ARTIFACT_PARENT_KINDS);
pub const STREAM_OBJECT_KIND: ObjectKindDescriptor =
    ObjectKindDescriptor::new(STREAM_OBJECT_KIND_ID, ObjectKindTag::new(10), "Stream")
        .with_parent_kinds(STREAM_PARENT_KINDS);

pub const BROWSER_OBJECT_KINDS: &[ObjectKindDescriptor] = &[
    BROWSER_OBJECT_KIND,
    BROWSER_CONTEXT_OBJECT_KIND,
    PAGE_OBJECT_KIND,
    FRAME_OBJECT_KIND,
    JS_HANDLE_OBJECT_KIND,
    ELEMENT_HANDLE_OBJECT_KIND,
    REQUEST_OBJECT_KIND,
    RESPONSE_OBJECT_KIND,
    ARTIFACT_OBJECT_KIND,
    STREAM_OBJECT_KIND,
];
pub const BROWSER_OBJECT_KIND_CATALOG: ObjectKindCatalog =
    ObjectKindCatalog::new(BROWSER_OBJECT_KINDS);

macro_rules! object_kind {
    ($name:ident, $descriptor:ident) => {
        pub enum $name {}

        impl ObjectKindSpec for $name {
            const DESCRIPTOR: ObjectKindDescriptor = $descriptor;
        }
    };
}

object_kind!(BrowserKind, BROWSER_OBJECT_KIND);
object_kind!(BrowserContextKind, BROWSER_CONTEXT_OBJECT_KIND);
object_kind!(PageKind, PAGE_OBJECT_KIND);
object_kind!(FrameKind, FRAME_OBJECT_KIND);
object_kind!(JsHandleKind, JS_HANDLE_OBJECT_KIND);
object_kind!(ElementHandleKind, ELEMENT_HANDLE_OBJECT_KIND);
object_kind!(RequestKind, REQUEST_OBJECT_KIND);
object_kind!(ResponseKind, RESPONSE_OBJECT_KIND);
object_kind!(ArtifactKind, ARTIFACT_OBJECT_KIND);
object_kind!(StreamKind, STREAM_OBJECT_KIND);

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        ARTIFACT_OBJECT_KIND, BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND,
        BROWSER_OBJECT_KIND_CATALOG, BROWSER_OBJECT_KIND_ID, BROWSER_OBJECT_KINDS,
        ELEMENT_HANDLE_OBJECT_KIND, FRAME_OBJECT_KIND, JS_HANDLE_OBJECT_KIND, PAGE_OBJECT_KIND,
        PageKind, REQUEST_OBJECT_KIND, RESPONSE_OBJECT_KIND, STREAM_OBJECT_KIND,
    };
    use pwright_core::ObjectKindSpec;

    #[test]
    fn browser_object_ids_and_tags_are_unique() {
        // Arrange
        let ids = BROWSER_OBJECT_KINDS
            .iter()
            .map(|descriptor| descriptor.id)
            .collect::<BTreeSet<_>>();
        let tags = BROWSER_OBJECT_KINDS
            .iter()
            .map(|descriptor| descriptor.tag)
            .collect::<BTreeSet<_>>();

        // Act
        let kind_count = BROWSER_OBJECT_KINDS.len();
        let browser = BROWSER_OBJECT_KIND_CATALOG
            .by_id(BROWSER_OBJECT_KIND_ID)
            .expect("browser descriptor should be installed");
        let context = BROWSER_OBJECT_KIND_CATALOG
            .by_id(BROWSER_CONTEXT_OBJECT_KIND.id)
            .expect("context descriptor should be installed");

        // Assert
        assert_eq!(ids.len(), kind_count);
        assert_eq!(tags.len(), kind_count);
        assert_eq!(BROWSER_OBJECT_KIND_CATALOG.validate(), Ok(()));
        assert_eq!(
            BROWSER_OBJECT_KIND_CATALOG.by_tag(PageKind::DESCRIPTOR.tag),
            Some(PageKind::DESCRIPTOR)
        );
        assert!(browser.allows_root);
        assert!(!context.allows_root);
        assert!(context.allows_parent(BROWSER_OBJECT_KIND.id));
        assert!(PAGE_OBJECT_KIND.allows_parent(BROWSER_CONTEXT_OBJECT_KIND.id));
        assert!(FRAME_OBJECT_KIND.allows_parent(PAGE_OBJECT_KIND.id));
        assert!(FRAME_OBJECT_KIND.allows_parent(FRAME_OBJECT_KIND.id));
        assert!(JS_HANDLE_OBJECT_KIND.allows_parent(FRAME_OBJECT_KIND.id));
        assert!(ELEMENT_HANDLE_OBJECT_KIND.allows_parent(FRAME_OBJECT_KIND.id));
        assert!(REQUEST_OBJECT_KIND.allows_parent(BROWSER_CONTEXT_OBJECT_KIND.id));
        assert!(REQUEST_OBJECT_KIND.allows_parent(PAGE_OBJECT_KIND.id));
        assert!(REQUEST_OBJECT_KIND.allows_parent(FRAME_OBJECT_KIND.id));
        assert!(RESPONSE_OBJECT_KIND.allows_parent(REQUEST_OBJECT_KIND.id));
        assert!(ARTIFACT_OBJECT_KIND.allows_parent(BROWSER_OBJECT_KIND.id));
        assert!(ARTIFACT_OBJECT_KIND.allows_parent(BROWSER_CONTEXT_OBJECT_KIND.id));
        assert!(ARTIFACT_OBJECT_KIND.allows_parent(PAGE_OBJECT_KIND.id));
        assert!(STREAM_OBJECT_KIND.allows_parent(ARTIFACT_OBJECT_KIND.id));
    }
}
