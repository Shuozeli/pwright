//! One pre-admitted dependent callback group in the existing publication queue.
//! Native producers, not collection diffs, determine which causes to stage.
use super::{
    CollectionMember, CollectionRevision, CollectionView, FrameChange, FrameChangeValue,
    FrameMetadata, FramePublicationDrain, FramePublications, HostError, MetadataState,
    PublicationDrainError, ResourceClass, ResourceLease, SCOPE,
};
use crate::frame_tree::{FrameClosePublication, FrameTreeFactory, Storage};
use pwright_core::ObjectId;
use std::sync::{Arc, Weak};

mod delivery;
mod terminal;
pub(crate) use delivery::CreationDelivery;
pub(crate) use terminal::TerminalLifecycle;

/// Authoritative native input, not an inferred difference between snapshots.
/// The producer must reserve before native work, supply chronological views,
/// and avoid also staging a legacy close publication for the same cause.
pub enum LifecycleCause {
    PageReady {
        revision: CollectionRevision,
        page: ObjectId,
        opener: Option<ObjectId>,
    },
    PopupLinked {
        opener: ObjectId,
        popup: ObjectId,
    },
    PageClosed {
        page: ObjectId,
        close: FrameClosePublication,
    },
    ContextClosed {
        context: ObjectId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleNotice {
    ContextPage { context: ObjectId, page: ObjectId },
    Popup { opener: ObjectId, popup: ObjectId },
    PageClose { context: ObjectId, page: ObjectId },
    ContextClose { context: ObjectId },
}

/// Synchronous EventEmitter outcome. A returned Promise is Delivered; its
/// eventual rejection is not awaited and cannot suppress another notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleDelivery {
    Delivered,
    ListenerThrew,
}

/// Borrowed metadata for foreign wrapper admission/conversion. It includes
/// retired descriptors, but no host/runtime/transport. No native lock is held.
pub struct LifecycleNoticeView<'a> {
    pub notice: LifecycleNotice,
    pub revision: &'a CollectionRevision,
}

struct PreparedLifecycle {
    before: CollectionView,
    after: CollectionView,
    next: LifecycleNotice,
    dependent: Option<LifecycleNotice>,
    close: Option<FrameClosePublication>,
    awaiting: bool,
    creation: Option<CreationDelivery>,
}

pub(super) struct LifecycleGroup {
    prepared: Option<PreparedLifecycle>,
    _storage: Arc<Storage>,
    creation: Option<CreationDelivery>,
}

impl LifecycleGroup {
    pub(super) fn permits_popup(&self) -> Result<bool, HostError> {
        let prepared = self.prepared.as_ref().expect("staged group");
        if matches!(prepared.next, LifecycleNotice::Popup { .. })
            && let Some(creation) = &prepared.creation
        {
            return creation.permits_popup();
        }
        Ok(true)
    }

    pub(super) fn abandon_delivery(&self) {
        let prepared = self.prepared.as_ref().expect("staged group");
        if matches!(prepared.next, LifecycleNotice::ContextPage { .. })
            && let Some(creation) = &prepared.creation
        {
            creation.acknowledge(false);
        }
    }
    pub(super) fn awaiting_acknowledgement(&self) -> bool {
        self.prepared
            .as_ref()
            .expect("only prepared groups are staged")
            .awaiting
    }
}

/// Owns its bounded queue slot and allocated body before a native transaction.
/// Dropping an unused admission releases both budgets. No side queue is created.
pub struct LifecycleReservation {
    book: Weak<FrameMetadata>,
    event: ResourceLease,
    group: Box<LifecycleGroup>,
}

impl LifecycleReservation {
    pub(crate) fn track_creation(mut self, creation: CreationDelivery) -> Self {
        self.group.creation = Some(creation);
        self
    }
}

const STORAGE_BYTES: usize = std::mem::size_of::<LifecycleGroup>()
    + std::mem::size_of::<LifecycleReservation>()
    + std::mem::size_of::<Storage>()
    + 2 * std::mem::size_of::<usize>()
    // One conversion's temporary revision/notice; retained metadata clones
    // continue to share their original projection's lease.
    + std::mem::size_of::<CollectionRevision>()
    + std::mem::size_of::<LifecycleNoticeView<'static>>();

fn invalid(message: &str) -> HostError {
    HostError::InvalidArgument(message.into())
}

fn validate_popup(
    view: &CollectionView,
    opener: ObjectId,
    popup: ObjectId,
) -> Result<(), HostError> {
    let pages = view.pages();
    let opener_page = pages.iter().find(|page| page.id == opener);
    let popup_page = pages.iter().find(|page| page.id == popup);
    match (opener_page, popup_page) {
        (Some(left), Some(right)) if opener != popup && left.context == right.context => Ok(()),
        _ => Err(invalid(
            "popup requires distinct visible Pages in the same Context",
        )),
    }
}

impl PreparedLifecycle {
    fn new(before: CollectionView, cause: LifecycleCause) -> Result<Self, HostError> {
        let (after, next, dependent, close) = match cause {
            LifecycleCause::PageReady {
                revision,
                page,
                opener,
            } => {
                let mut after = before.try_rebase(&revision).map_err(HostError::backend)?;
                let context = revision
                    .pages()
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("ready Page is missing from native revision"))?
                    .context;
                if !after.contexts().iter().any(|entry| entry.id == context) {
                    after = after
                        .with_visibility(CollectionMember::Context(context), true)
                        .map_err(HostError::backend)?;
                }
                after = after
                    .with_visibility(CollectionMember::Page(page), true)
                    .map_err(HostError::backend)?;
                if let Some(opener) = opener {
                    validate_popup(&after, opener, page)?;
                }
                (
                    after,
                    LifecycleNotice::ContextPage { context, page },
                    opener.map(|opener| LifecycleNotice::Popup {
                        opener,
                        popup: page,
                    }),
                    None,
                )
            }
            LifecycleCause::PopupLinked { opener, popup } => {
                validate_popup(&before, opener, popup)?;
                (
                    before.clone(),
                    LifecycleNotice::Popup { opener, popup },
                    None,
                    None,
                )
            }
            LifecycleCause::PageClosed { page, close } => {
                let pages = before.pages();
                let descriptor = pages
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("closed Page is not visible"))?;
                if !close.is_for(&descriptor.tree) {
                    return Err(invalid("Page close publication belongs to another tree"));
                }
                let after = before
                    .with_visibility(CollectionMember::Page(page), false)
                    .map_err(HostError::backend)?;
                (
                    after,
                    LifecycleNotice::PageClose {
                        context: descriptor.context,
                        page,
                    },
                    None,
                    Some(close),
                )
            }
            LifecycleCause::ContextClosed { context } => {
                let after = before
                    .with_visibility(CollectionMember::Context(context), false)
                    .map_err(HostError::backend)?;
                (after, LifecycleNotice::ContextClose { context }, None, None)
            }
        };
        Ok(Self {
            before,
            after,
            next,
            dependent,
            close,
            awaiting: false,
            creation: None,
        })
    }
}

impl FramePublications {
    pub fn reserve_lifecycle(
        &self,
        factory: &FrameTreeFactory,
    ) -> Result<LifecycleReservation, HostError> {
        let state = self.0.state.lock().map_err(HostError::backend)?;
        if state.discarded || state.sequence == u64::MAX {
            return Err(HostError::TargetClosed);
        }
        if state.queue_storage.is_none() {
            return Err(invalid(
                "lifecycle groups require a pre-admitted publication queue",
            ));
        }
        let event = self
            .0
            .resources
            .reserve(SCOPE, ResourceClass::EventItem, 1)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let storage = factory.reserve_collection_storage(STORAGE_BYTES)?;
        Ok(LifecycleReservation {
            book: Arc::downgrade(&self.0),
            event,
            group: Box::new(LifecycleGroup {
                prepared: None,
                _storage: Arc::new(storage),
                creation: None,
            }),
        })
    }

    /// Stages without publishing. The returned *planned* view sequences the next
    /// native group; it is not the public view. Exact predecessor identity is
    /// checked again at commit. Native source/terminal selection is a separate
    /// integration responsibility; this method cannot stage after teardown.
    pub fn stage_lifecycle(
        &self,
        mut reserved: LifecycleReservation,
        before: CollectionView,
        cause: LifecycleCause,
    ) -> Result<CollectionView, HostError> {
        if !reserved.book.ptr_eq(&Arc::downgrade(&self.0)) || !before.belongs_to(&self.0.resources)
        {
            return Err(invalid(
                "lifecycle reservation/view belongs to another publication queue",
            ));
        }
        if reserved.group.creation.is_some()
            && !matches!(
                cause,
                LifecycleCause::PageReady { .. } | LifecycleCause::PopupLinked { .. }
            )
        {
            return Err(invalid(
                "creation acknowledgement requires a ready or popup cause",
            ));
        }
        let mut prepared = PreparedLifecycle::new(before, cause)?;
        prepared.creation = reserved.group.creation.take();
        let after = prepared.after.clone();
        let mut state = self.0.state.lock().map_err(HostError::backend)?;
        if state.discarded || state.sequence == u64::MAX {
            return Err(HostError::TargetClosed);
        }
        let sequence = state.next_sequence()?;
        reserved.group.prepared = Some(prepared);
        state.pending.push_back(FrameChange {
            sequence,
            value: FrameChangeValue::Lifecycle(reserved.group),
            _event: reserved.event,
        });
        state.sequence = sequence;
        drop(state);
        self.0.changed.notify_one();
        Ok(after)
    }
}

pub(super) fn front_group(state: &MetadataState, sequence: u64) -> Option<&LifecycleGroup> {
    let change = state
        .pending
        .front()
        .filter(|change| change.sequence <= sequence)?;
    match &change.value {
        FrameChangeValue::Lifecycle(group) => Some(group),
        _ => None,
    }
}

fn front_prepared_mut(
    state: &mut MetadataState,
    sequence: u64,
) -> Result<&mut PreparedLifecycle, PublicationDrainError> {
    let change = state
        .pending
        .front_mut()
        .filter(|change| change.sequence <= sequence)
        .ok_or(PublicationDrainError::NoNotice)?;
    match &mut change.value {
        FrameChangeValue::Lifecycle(group) => Ok(group.prepared.as_mut().expect("staged group")),
        _ => Err(PublicationDrainError::NoNotice),
    }
}

impl FramePublicationDrain {
    fn validate_notice_owner(&self, state: &MetadataState) -> Result<(), PublicationDrainError> {
        if state.discarded || state.drain.active != Some(self.owner) {
            return Err(PublicationDrainError::Discarded);
        }
        Ok(())
    }

    /// Prepare/admit the foreign wrapper before commit. The closure runs without
    /// native locks and may fail without consuming the notice or changing state.
    /// Returning does not commit; call commit_notice immediately before emit.
    pub fn with_next_notice<T>(
        &self,
        convert: impl FnOnce(LifecycleNoticeView<'_>) -> T,
    ) -> Result<T, PublicationDrainError> {
        let book = self.book.as_ref().ok_or(PublicationDrainError::Finished)?;
        let state = book.0.state.lock().map_err(HostError::backend)?;
        self.validate_notice_owner(&state)?;
        let group = front_group(&state, self.sequence).ok_or(PublicationDrainError::NoNotice)?;
        if !group.permits_popup()? {
            return Err(PublicationDrainError::NoNotice);
        }
        let prepared = group.prepared.as_ref().expect("staged group");
        if prepared.awaiting {
            return Err(PublicationDrainError::AwaitingAcknowledgement);
        }
        let notice = prepared.next;
        let revision = prepared.after.revision().clone();
        // Conversion may synchronously discard the endpoint. Keep its admitted
        // temporary-body allowance until the closure returns or unwinds.
        let storage = group._storage.clone();
        drop(state);
        let result = convert(LifecycleNoticeView {
            notice,
            revision: &revision,
        });
        drop(storage);
        Ok(result)
    }

    /// Installs callback-visible state and consumes this notice exactly once,
    /// before foreign code executes. A listener throw cannot roll this back.
    pub fn commit_notice(&mut self) -> Result<LifecycleNotice, PublicationDrainError> {
        let book = self.book.as_ref().ok_or(PublicationDrainError::Finished)?;
        let mut state = book.0.state.lock().map_err(HostError::backend)?;
        self.validate_notice_owner(&state)?;
        let current = state.collections.clone();
        let group = front_group(&state, self.sequence).ok_or(PublicationDrainError::NoNotice)?;
        if !group.permits_popup()? {
            return Err(PublicationDrainError::NoNotice);
        }
        let prepared = front_prepared_mut(&mut state, self.sequence)?;
        if prepared.awaiting {
            return Err(PublicationDrainError::AwaitingAcknowledgement);
        }
        let result = if !current
            .as_ref()
            .is_some_and(|view| view.same_state(&prepared.before))
        {
            Err(HostError::Backend(
                "lifecycle publication predecessor changed".into(),
            ))
        } else if let Some(close) = prepared.close.take() {
            close.publish()
        } else {
            Ok(())
        };
        if let Err(error) = result {
            drop(state);
            book.discard();
            self.book = None;
            return Err(PublicationDrainError::Backend(error));
        }
        prepared.awaiting = true;
        let notice = prepared.next;
        state.collections = Some(prepared.after.clone());
        Ok(notice)
    }

    /// Report only the synchronous emit result. ListenerThrew abandons this
    /// group's dependent popup, never independent later close groups. Dropping
    /// a guard while acknowledgement is outstanding has the same skip policy.
    pub fn acknowledge_notice(
        &mut self,
        delivery: LifecycleDelivery,
    ) -> Result<(), PublicationDrainError> {
        let book = self.book.as_ref().ok_or(PublicationDrainError::Finished)?;
        let mut state = book.0.state.lock().map_err(HostError::backend)?;
        self.validate_notice_owner(&state)?;
        let prepared = front_prepared_mut(&mut state, self.sequence)?;
        if !prepared.awaiting {
            return Err(PublicationDrainError::NotCommitted);
        }
        if matches!(prepared.next, LifecycleNotice::ContextPage { .. })
            && let Some(creation) = &prepared.creation
        {
            creation.acknowledge(delivery == LifecycleDelivery::Delivered);
        }
        if let Some(next) = prepared
            .dependent
            .take()
            .filter(|_| delivery == LifecycleDelivery::Delivered)
        {
            prepared.next = next;
            prepared.before = prepared.after.clone();
            prepared.awaiting = false;
        } else {
            state.pending.pop_front();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
