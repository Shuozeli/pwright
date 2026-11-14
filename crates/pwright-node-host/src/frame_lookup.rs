//! Metadata-only lookup kernel. Foreign callbacks stay synchronous and scoped;
//! traversal retains an admitted published collection, never a browser runtime.

mod glob;
mod glob_base;
pub use glob::{GlobPlan, RESOLVED_GLOB_BYTES};
pub use glob_base::{GlobUrlResolution, PreparedGlob, URL_WORK_BYTES};

use crate::HostError;
use crate::frame_tree::{FrameNode, FrameTree, FrameTreeRead};
use pwright_core::ObjectId;
#[cfg(feature = "url-glob")]
use url::Url;

#[cfg(feature = "url-glob")]
impl crate::HostSession {
    /// Prepare Playwright's URL glob in Rust while retaining its bounded native
    /// allocation through conversion by the language-specific RegExp engine.
    pub fn page_url_glob(&self, page_id: &str, glob: &str) -> Result<GlobPlan, HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|value| *value != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        let tree = self.frame_trees.get(page)?;
        let resources = tree.lookup_resources();
        let context = self.page_context_snapshot(page_id)?;
        let prepared = PreparedGlob::new(resources, glob)?;
        let Some(relative) = prepared.relative_path() else {
            return prepared.compile(resources, None);
        };
        let _resolution = context.reserve_conversion(URL_WORK_BYTES)?;
        let resolved = match context.base_url() {
            Some(base) => Url::parse(base)
                .ok()
                .and_then(|base| base.join(relative).ok()),
            None => Url::parse(relative).ok(),
        };
        let Some(resolved) = resolved else {
            return prepared.compile(resources, None);
        };
        let origin = resolved.origin().ascii_serialization();
        prepared.compile(
            resources,
            Some(GlobUrlResolution {
                serialized: resolved.as_str(),
                origin: &origin,
            }),
        )
    }
}

/// Keeps a caller's exception separate from native metadata failures.
#[derive(Debug)]
pub enum LookupError<E> {
    Metadata(HostError),
    Matcher(E),
}

pub struct FrameLookup {
    read: FrameTreeRead,
}

impl FrameLookup {
    pub fn new(tree: &FrameTree) -> Result<Self, HostError> {
        // FrameLookup adds no storage to its single FrameTreeRead owner.
        const { assert!(std::mem::size_of::<FrameLookup>() == std::mem::size_of::<FrameTreeRead>()) };
        Ok(Self {
            read: tree.read(0)?,
        })
    }

    pub fn first_matching<E>(
        &self,
        mut matches: impl FnMut(&FrameNode) -> Result<bool, E>,
    ) -> Result<Option<&FrameNode>, E> {
        for frame in self.read.frames() {
            if matches(frame)? {
                return Ok(Some(frame));
            }
        }
        Ok(None)
    }

    pub fn by_name(&self, name: &str) -> Result<Option<&FrameNode>, HostError> {
        self.first_matching(|frame| frame.with_name(|value| value == name))
    }

    /// Caller admits descriptor storage; visit only selected-to-root ancestry.
    /// Return false to stop at an existing cached wrapper. No allocation here.
    pub fn visit_path<E>(
        &self,
        id: ObjectId,
        mut visit: impl FnMut(&FrameNode) -> Result<bool, E>,
    ) -> Result<(), LookupError<E>> {
        let frames = self.read.frames();
        let mut cursor = Some(id);
        for _ in 0..frames.len() {
            let Some(id) = cursor else {
                return Ok(());
            };
            let node = frames
                .iter()
                .find(|node| node.id() == id)
                .ok_or(LookupError::Metadata(HostError::TargetClosed))?;
            if !visit(node).map_err(LookupError::Matcher)? {
                return Ok(());
            }
            cursor = node.parent_id();
        }
        if cursor.is_none() {
            Ok(())
        } else {
            Err(LookupError::Metadata(HostError::InvalidArgument(
                "Invalid pinned Frame ancestry".into(),
            )))
        }
    }

    pub fn bind(&self, id: ObjectId) -> Result<crate::frame_tree::FrameBinding, HostError> {
        self.read.bind(id)
    }

    /// The matcher port owns RegExp/predicate-specific semantics. Preserve its
    /// state across candidates and its original error value on short circuit.
    pub fn by_url<E>(
        &self,
        mut matches: impl FnMut(&str) -> Result<bool, E>,
    ) -> Result<Option<&FrameNode>, LookupError<E>> {
        self.first_matching(|frame| {
            frame
                .with_url(&mut matches)
                .map_err(LookupError::Metadata)?
                .map_err(LookupError::Matcher)
        })
    }
}

#[cfg(test)]
mod tests;
