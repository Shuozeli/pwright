//! Immutable membership visibility over an already admitted revision. Changing
//! a view allocates nothing and does not mutate another reader's membership.
//! This is not an event dispatcher or a Page-closed-flag transition.
use super::{CONTEXTS, CollectionContext, CollectionPage, CollectionRevision, PAGES};
use pwright_core::ObjectId;
use std::{error::Error, fmt, iter::FusedIterator, ops::Index};

const _: () = assert!(CONTEXTS == 16 && PAGES == 16);

mod rebase;
pub use rebase::CollectionRebaseError;

#[derive(Clone)]
pub struct CollectionView {
    revision: CollectionRevision,
    contexts: u16,
    pages: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionMember {
    Context(ObjectId),
    Page(ObjectId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionVisibilityError {
    Unknown(CollectionMember),
    AlreadyVisible(CollectionMember),
    NotVisible(CollectionMember),
    ContextHasVisiblePages(ObjectId),
    ParentNotVisible(ObjectId),
}

impl fmt::Display for CollectionVisibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid collection visibility transition: {self:?}"
        )
    }
}

impl Error for CollectionVisibilityError {}

fn full_mask(len: usize) -> u16 {
    if len == 0 { 0 } else { u16::MAX >> (16 - len) }
}

impl CollectionView {
    pub(crate) fn revision(&self) -> &CollectionRevision {
        &self.revision
    }

    pub(crate) fn same_state(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.revision.0, &other.revision.0)
            && self.contexts == other.contexts
            && self.pages == other.pages
    }

    pub(in crate::frame_metadata) fn belongs_to(
        &self,
        domain: &pwright_core::ResourceLedger,
    ) -> bool {
        self.revision.0.domain.shares_budget_with(domain)
    }

    pub fn full(revision: CollectionRevision) -> Self {
        Self {
            contexts: full_mask(revision.contexts().len()),
            pages: full_mask(revision.pages().len()),
            revision,
        }
    }

    /// All descriptors remain retained/charged, but none is publicly visible.
    pub fn hidden(revision: CollectionRevision) -> Self {
        Self {
            revision,
            contexts: 0,
            pages: 0,
        }
    }

    pub fn contexts(&self) -> CollectionMembers<'_, CollectionContext> {
        CollectionMembers {
            entries: self.revision.contexts(),
            visible: self.contexts,
        }
    }

    pub fn pages(&self) -> CollectionMembers<'_, CollectionPage> {
        CollectionMembers {
            entries: self.revision.pages(),
            visible: self.pages,
        }
    }

    /// Returns a new view or a typed error. The receiver and all retained views
    /// remain unchanged on success and failure. Native ownership is untouched.
    pub fn with_visibility(
        &self,
        member: CollectionMember,
        visible: bool,
    ) -> Result<Self, CollectionVisibilityError> {
        let (position, mask) = match member {
            CollectionMember::Context(id) => (
                self.revision
                    .contexts()
                    .iter()
                    .position(|entry| entry.id == id),
                self.contexts,
            ),
            CollectionMember::Page(id) => (
                self.revision
                    .pages()
                    .iter()
                    .position(|entry| entry.id == id),
                self.pages,
            ),
        };
        let position = position.ok_or(CollectionVisibilityError::Unknown(member))?;
        let bit = 1u16 << position;
        if (mask & bit != 0) == visible {
            return Err(if visible {
                CollectionVisibilityError::AlreadyVisible(member)
            } else {
                CollectionVisibilityError::NotVisible(member)
            });
        }
        match member {
            CollectionMember::Context(id) if !visible => {
                if self.pages().iter().any(|page| page.context == id) {
                    return Err(CollectionVisibilityError::ContextHasVisiblePages(id));
                }
            }
            CollectionMember::Page(_) if visible => {
                let context = self.revision.pages()[position].context;
                if !self.contexts().iter().any(|entry| entry.id == context) {
                    return Err(CollectionVisibilityError::ParentNotVisible(context));
                }
            }
            CollectionMember::Context(_) | CollectionMember::Page(_) => {}
        }
        let mut next = self.clone();
        let mask = match member {
            CollectionMember::Context(_) => &mut next.contexts,
            CollectionMember::Page(_) => &mut next.pages,
        };
        if visible {
            *mask |= bit;
        } else {
            *mask &= !bit;
        }
        Ok(next)
    }
}

/// Ordered borrowed members. The mask is private and always comes from a
/// validated revision/view, so its set bits cannot exceed the admitted slice.
pub struct CollectionMembers<'a, T> {
    entries: &'a [T],
    visible: u16,
}

impl<'a, T> CollectionMembers<'a, T> {
    pub fn len(&self) -> usize {
        self.visible.count_ones() as usize
    }
    pub fn is_empty(&self) -> bool {
        self.visible == 0
    }
    pub fn iter(&self) -> VisibleMembers<'a, T> {
        VisibleMembers {
            entries: self.entries,
            remaining: self.visible,
        }
    }
    pub fn get(&self, index: usize) -> Option<&'a T> {
        self.iter().nth(index)
    }
}

impl<T> Index<usize> for CollectionMembers<'_, T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        self.get(index)
            .expect("collection member index out of bounds")
    }
}

impl<'a, T> IntoIterator for CollectionMembers<'a, T> {
    type Item = &'a T;
    type IntoIter = VisibleMembers<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct VisibleMembers<'a, T> {
    entries: &'a [T],
    remaining: u16,
}

impl<'a, T> Iterator for VisibleMembers<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let index = self.remaining.trailing_zeros() as usize;
        self.remaining &= self.remaining - 1;
        Some(&self.entries[index])
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.remaining.count_ones() as usize;
        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for VisibleMembers<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let index = 15 - self.remaining.leading_zeros() as usize;
        self.remaining &= !(1u16 << index);
        Some(&self.entries[index])
    }
}

impl<T> ExactSizeIterator for VisibleMembers<'_, T> {}
impl<T> FusedIterator for VisibleMembers<'_, T> {}

#[cfg(test)]
mod tests;
