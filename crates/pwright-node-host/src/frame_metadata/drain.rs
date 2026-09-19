//! Exclusive, allocation-free advancement of an admitted metadata queue.
//! Metadata advances directly; lifecycle groups require conversion, commit and
//! acknowledgement outside native locks. Producers supply native cause ordering.
use super::{FramePublicationFence, FramePublications, HostError};
use std::num::NonZeroU64;

#[derive(Default)]
pub(super) struct DrainOwnership {
    issued: u64,
    pub(super) active: Option<NonZeroU64>,
}

#[derive(Debug, thiserror::Error)]
pub enum PublicationDrainError {
    #[error("metadata publication has an exclusive drain owner")]
    Busy,
    #[error("exclusive publication requires pre-admitted queue storage")]
    NotSelected,
    #[error("publication drain has finished")]
    Finished,
    #[error("publication endpoint was discarded")]
    Discarded,
    #[error("publication owner sequence exhausted")]
    OwnerSequenceExhausted,
    #[error("lifecycle notice must be converted and committed before continuing")]
    NoticePending,
    #[error("the committed lifecycle notice needs a synchronous delivery acknowledgement")]
    AwaitingAcknowledgement,
    #[error("there is no lifecycle notice at this watermark")]
    NoNotice,
    #[error("lifecycle notice has not been committed")]
    NotCommitted,
    #[error(transparent)]
    Backend(#[from] HostError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationProgress {
    /// Exactly one internal metadata change was installed, not a foreign event.
    Published,
    /// This fixed watermark is exhausted; the claim and metadata owner released.
    Complete,
}

/// Non-cloneable authority over one fixed watermark. Drop preserves uncommitted
/// work, but abandons the dependent remainder of an interrupted committed
/// dispatch. It never executes a callback. No runtime, transport, callback,
/// separate notice journal, or newly allocated queue is retained.
///
/// The selected queue allowance includes one active guard body. Finished guards
/// retain no metadata; caller-owned containers/foreign wrappers need their own
/// admission. All methods return with native locks released.
pub struct FramePublicationDrain {
    pub(super) book: Option<FramePublications>,
    pub(super) sequence: u64,
    pub(super) owner: NonZeroU64,
}

impl FramePublicationFence {
    /// Claims without publishing. Borrowing preserves this exact watermark for
    /// retry after Busy; obtaining a newer fence would change its meaning.
    pub fn try_begin(&self) -> Result<FramePublicationDrain, PublicationDrainError> {
        let mut state = self.book.0.state.lock().map_err(HostError::backend)?;
        if state.discarded {
            return Err(PublicationDrainError::Discarded);
        }
        if state.queue_storage.is_none() {
            return Err(PublicationDrainError::NotSelected);
        }
        if state.drain.active.is_some() {
            return Err(PublicationDrainError::Busy);
        }
        let owner = state
            .drain
            .issued
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .ok_or(PublicationDrainError::OwnerSequenceExhausted)?;
        state.drain.issued = owner.get();
        state.drain.active = Some(owner);
        Ok(FramePublicationDrain {
            book: Some(self.book.clone()),
            sequence: self.sequence,
            owner,
        })
    }
}

impl FramePublicationDrain {
    /// Publishes at most one change. Call until Complete, including after the
    /// final Published result: terminal disconnect readiness is installed only
    /// at Complete. Reentrant attempts to acquire/bypass this owner fail Busy.
    /// Lifecycle groups return NoticePending/AwaitingAcknowledgement without
    /// consuming the group; use the notice conversion/commit/acknowledgement API.
    ///
    /// A metadata application error discards the endpoint rather than allowing
    /// a retry after a partially applied operation. Lock poisoning is also fatal
    /// and requires endpoint teardown, not retry.
    pub fn publish_next(&mut self) -> Result<PublicationProgress, PublicationDrainError> {
        let book = self.book.as_ref().ok_or(PublicationDrainError::Finished)?;
        let mut state = book.0.state.lock().map_err(HostError::backend)?;
        if state.discarded || state.drain.active != Some(self.owner) {
            drop(state);
            self.book = None;
            return Err(PublicationDrainError::Discarded);
        }
        if let Some(group) = super::lifecycle::front_group(&state, self.sequence) {
            if !group.awaiting_acknowledgement() {
                match group.permits_popup() {
                    Ok(false) => {
                        state.pending.pop_front();
                        return Ok(PublicationProgress::Published);
                    }
                    Ok(true) => {}
                    Err(error) => {
                        drop(state);
                        book.discard();
                        self.book = None;
                        return Err(PublicationDrainError::Backend(error));
                    }
                }
            }
            return Err(if group.awaiting_acknowledgement() {
                PublicationDrainError::AwaitingAcknowledgement
            } else {
                PublicationDrainError::NoticePending
            });
        }
        match state.publish_one_through(self.sequence) {
            Ok(true) => return Ok(PublicationProgress::Published),
            Ok(false) => {}
            Err(error) => {
                drop(state);
                book.discard();
                self.book = None;
                return Err(PublicationDrainError::Backend(error));
            }
        }
        state.complete_publication(self.sequence);
        state.drain.active = None;
        drop(state);
        self.book = None;
        Ok(PublicationProgress::Complete)
    }
}

impl Drop for FramePublicationDrain {
    fn drop(&mut self) {
        if let Some(book) = &self.book {
            let mut state = book
                .0
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if state.drain.active == Some(self.owner) {
                // An interrupted foreign dispatch cannot be replayed. Abandon
                // only its remaining dependent notices, not later groups.
                if super::lifecycle::front_group(&state, self.sequence)
                    .is_some_and(|group| group.awaiting_acknowledgement())
                {
                    super::lifecycle::front_group(&state, self.sequence)
                        .expect("checked group")
                        .abandon_delivery();
                    state.pending.pop_front();
                }
                state.drain.active = None;
            }
            drop(state);
            book.0.changed.notify_one();
        }
    }
}

#[cfg(test)]
mod tests;
