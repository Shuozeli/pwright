//! Metadata-only lookup kernel. Foreign callbacks stay synchronous and scoped;
//! traversal retains an admitted published collection, never a browser runtime.

mod glob;
mod glob_base;
pub use glob::{GlobPlan, RESOLVED_GLOB_BYTES};
pub use glob_base::{GlobUrlResolution, PreparedGlob, URL_WORK_BYTES};

use crate::HostError;
use crate::frame_tree::{FrameNode, FrameTree, FrameTreeRead};
use pwright_core::ObjectId;

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
