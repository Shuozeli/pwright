//! Borrowed navigation routing, independent of execution-context availability.
use super::{
    CdpPageState, CdpTopology, CdpTopologyError, CdpTopologyProductionError, CdpTopologyUpdate,
    ConnectionEpoch, ObjectId, ProductionCdpTopology, SessionOwner,
};

/// A current Frame-to-renderer route, borrowing one immutable topology revision.
///
/// This is not an owned dispatch lease. A caller retaining any part across a
/// topology update must first admit its storage and retain/revalidate the exact
/// connection and Frame lifetime. In particular, a session swap is not a license
/// to retry a navigation that may already have been sent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpFrameNavigationRoute<'a> {
    epoch: ConnectionEpoch,
    object_id: ObjectId,
    page_object_id: ObjectId,
    frame_id: &'a str,
    target_id: &'a str,
    session_id: &'a str,
}

impl<'a> CdpFrameNavigationRoute<'a> {
    pub const fn epoch(self) -> ConnectionEpoch {
        self.epoch
    }
    pub const fn object_id(self) -> ObjectId {
        self.object_id
    }
    pub const fn page_object_id(self) -> ObjectId {
        self.page_object_id
    }
    pub const fn frame_id(self) -> &'a str {
        self.frame_id
    }
    pub const fn target_id(self) -> &'a str {
        self.target_id
    }
    pub const fn session_id(self) -> &'a str {
        self.session_id
    }
}

impl CdpTopology {
    /// Apply the topology part of an ordered native new-document commit.
    /// Preserve the selected Frame, but retire its old descendant Frames and
    /// their worlds/sessions. Same-document navigation must not call this.
    /// The current renderer must own the exact native Frame; stale/unknown
    /// routes do nothing, including a main-session event for an OOPIF.
    pub fn frame_document_committed(
        &mut self,
        epoch: ConnectionEpoch,
        session: &str,
        native_frame: &str,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        self.validate_native_id("session ID", session)?;
        self.validate_native_id("frame ID", native_frame)?;
        let Some(frame) = self.frame_object_id(native_frame) else {
            return Ok(CdpTopologyUpdate::idempotent());
        };
        if !self
            .frame_navigation_route(frame)
            .is_some_and(|route| route.session_id() == session)
        {
            return Ok(CdpTopologyUpdate::idempotent());
        }
        let mut effects = Vec::new();
        self.remove_frame_descendants(frame, &mut effects);
        // Pending descendants need retirement even when no materialized child
        // exists. They must not appear later under the new document.
        self.remove_pending_frame_descendants(native_frame);
        self.refresh_metrics();
        Ok(CdpTopologyUpdate::applied(effects))
    }

    /// Resolve an existing portable Frame without creating or requiring a world.
    ///
    /// Local descendants use their nearest remote/main ancestor's session, but
    /// keep their own native Frame ID. Missing/detached Frames, crashed Pages,
    /// reconnect gaps, and ambiguous renderer swaps have no route. This does
    /// not resolve a Page ID as its main Frame or fall back to a Page session.
    /// Reads borrow existing bounded maps; no route index, allocation or I/O is
    /// added. The caller must interpret absence with its lifecycle policy.
    pub fn frame_navigation_route(
        &self,
        object_id: ObjectId,
    ) -> Option<CdpFrameNavigationRoute<'_>> {
        let frame_id = self.frame_native_id(object_id)?;
        let frame = self.frames.get(frame_id)?;
        let page = self.pages.get(&frame.page_target_id)?;
        if matches!(page.state, CdpPageState::Crashed { .. }) {
            return None;
        }
        let session_id = self.frame_session_id(object_id)?;
        let target_id = match self.sessions.get(session_id)? {
            SessionOwner::Page(target) => target.as_str(),
            SessionOwner::Frame(frame) => frame.as_str(),
            SessionOwner::PendingPage(_) | SessionOwner::PendingFrame(_) => return None,
        };
        Some(CdpFrameNavigationRoute {
            epoch: self.epoch(),
            object_id,
            page_object_id: page.object_ids.page,
            frame_id,
            target_id,
            session_id,
        })
    }
}

impl ProductionCdpTopology {
    pub fn frame_document_committed(
        &mut self,
        epoch: ConnectionEpoch,
        session: &str,
        native_frame: &str,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(|candidate| {
            candidate.frame_document_committed(epoch, session, native_frame)
        })
    }
}

#[cfg(test)]
mod tests;
