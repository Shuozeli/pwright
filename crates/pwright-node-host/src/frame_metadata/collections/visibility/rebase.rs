//! Move a visibility view to another admitted revision without guessing event
//! order. Callers must hide retired members first and explicitly reveal additions.
use super::{CollectionMember, CollectionRevision, CollectionView};
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionRebaseError {
    ForeignRevision,
    IdentityChanged(CollectionMember),
    VisibleMemberMissing(CollectionMember),
    VisibleOrderChanged(CollectionMember),
}

impl fmt::Display for CollectionRebaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid collection rebase: {self:?}")
    }
}

impl Error for CollectionRebaseError {}

/// Called only for visible members. The candidate mask is local until every
/// validation succeeds; neither input can be modified on failure.
fn retain_position(
    member: CollectionMember,
    position: Option<usize>,
    mask: &mut u16,
    last: &mut Option<usize>,
) -> Result<(), CollectionRebaseError> {
    let position = position.ok_or(CollectionRebaseError::VisibleMemberMissing(member))?;
    if last.is_some_and(|previous| position <= previous) {
        return Err(CollectionRebaseError::VisibleOrderChanged(member));
    }
    *mask |= 1u16 << position;
    *last = Some(position);
    Ok(())
}

impl CollectionView {
    /// Keeps exactly the same visible identities and order on an already admitted
    /// revision. Additions stay hidden; missing visible members fail. The caller
    /// must supply native cause ordering and publication watermark authorization:
    /// a shared domain proves neither revision chronology nor one-shot delivery.
    pub fn try_rebase(&self, revision: &CollectionRevision) -> Result<Self, CollectionRebaseError> {
        if !self
            .revision
            .0
            .domain
            .shares_budget_with(&revision.0.domain)
        {
            return Err(CollectionRebaseError::ForeignRevision);
        }
        // Check all overlapping descriptors, including currently hidden ones:
        // equal numeric IDs cannot silently replace metadata or change kind.
        for old in self.revision.contexts() {
            let changed_kind = revision.pages().iter().any(|page| page.id == old.id);
            let changed_settings = revision.contexts().iter().any(|next| {
                next.id == old.id && !old.settings.shares_metadata_with(&next.settings)
            });
            if changed_kind || changed_settings {
                return Err(CollectionRebaseError::IdentityChanged(
                    CollectionMember::Context(old.id),
                ));
            }
        }
        for old in self.revision.pages() {
            let changed_kind = revision
                .contexts()
                .iter()
                .any(|context| context.id == old.id);
            let changed_page = revision.pages().iter().any(|next| {
                next.id == old.id
                    && (next.context != old.context
                        || !next.main_frame.same_identity(&old.main_frame))
            });
            if changed_kind || changed_page {
                return Err(CollectionRebaseError::IdentityChanged(
                    CollectionMember::Page(old.id),
                ));
            }
        }
        let mut contexts = 0;
        let mut pages = 0;
        let mut last = None;
        for old in self.contexts() {
            retain_position(
                CollectionMember::Context(old.id),
                revision
                    .contexts()
                    .iter()
                    .position(|next| next.id == old.id),
                &mut contexts,
                &mut last,
            )?;
        }
        last = None;
        for old in self.pages() {
            retain_position(
                CollectionMember::Page(old.id),
                revision.pages().iter().position(|next| next.id == old.id),
                &mut pages,
                &mut last,
            )?;
        }
        Ok(Self {
            revision: revision.clone(),
            contexts,
            pages,
        })
    }
}

#[cfg(test)]
mod tests;
