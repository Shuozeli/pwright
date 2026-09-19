//! Terminal cause production, admitted before the native lifecycle starts.
//! This consumes a coherent planned view, not a diff of foreign collections.
use super::*;
use crate::frame_metadata::FrameClosePermit;
use crate::frame_tree::FrameTree;

const GROUPS: usize =
    crate::frame_metadata::collections::CONTEXTS + crate::frame_metadata::collections::PAGES;

/// All possible terminal Page/Context groups exist before native initialization.
/// Ordinary creation/close never spends this terminal-only admission.
pub(crate) struct TerminalLifecycle {
    reserved: Vec<LifecycleReservation>,
    _storage: Storage,
}

impl TerminalLifecycle {
    pub(crate) fn reserve(
        book: &FramePublications,
        factory: &FrameTreeFactory,
    ) -> Result<Box<Self>, HostError> {
        let storage = factory.reserve_collection_storage(
            std::mem::size_of::<Self>() + GROUPS * std::mem::size_of::<LifecycleReservation>(),
        )?;
        let mut reserved = Vec::with_capacity(GROUPS);
        for _ in 0..GROUPS {
            reserved.push(book.reserve_lifecycle(factory)?);
        }
        Ok(Box::new(Self {
            reserved,
            _storage: storage,
        }))
    }

    fn append(&mut self, state: &mut MetadataState, prepared: PreparedLifecycle) {
        // CollectionView validates the fixed Context/Page caps and distinct IDs.
        // Exactly one group is produced per visible member, so this cannot grow
        // the preallocated queue or exhaust the terminal-only reservation pool.
        let mut reserved = self
            .reserved
            .pop()
            .expect("one admitted terminal group per member");
        reserved.group.prepared = Some(prepared);
        state.pending.push_back(FrameChange {
            sequence: u64::MAX,
            value: FrameChangeValue::Lifecycle(reserved.group),
            _event: reserved.event,
        });
    }

    /// The native owner calls this once after stopping the driver. This path
    /// reserves/allocates nothing and recovers poisoned native metadata locks,
    /// matching the existing infallible teardown path. No foreign code executes.
    pub(crate) fn stage<'a>(
        mut self,
        book: &FramePublications,
        view: CollectionView,
        pages: impl Iterator<Item = (&'a FrameTree, &'a mut Option<FrameClosePermit>)>,
    ) {
        let mut state = book
            .0
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(start) = state.collection_batch.take() {
            // Failed reconciliation may have staged some raw close flags, but
            // its batch gate has never made them publishable. The terminal cut
            // owns those still-visible Pages now: transfer their publication to
            // the independent terminal groups rather than exposing every flag
            // before the first callback. Preserve all previously committed work.
            state.pending.retain(|change| {
                !(change.sequence > start
                    && matches!(&change.value, FrameChangeValue::Close(close)
                        if view.pages().iter().any(|page| close.is_for(&page.tree))))
            });
        }
        state.sequence = u64::MAX;
        // Also close native metadata for Pages not yet in the coherent public
        // collection. They receive no invented Page/Context notification.
        for (tree, permit) in pages {
            let close = tree.close_for_teardown();
            let visible = view.pages().iter().any(|page| close.is_for(&page.tree));
            if let Some(reserved) = permit.take()
                && !state.discarded
                && !visible
            {
                state.pending.push_back(FrameChange {
                    sequence: u64::MAX,
                    value: FrameChangeValue::Close(close),
                    _event: reserved.event,
                });
            }
        }
        if !state.discarded {
            let mut before = view.clone();
            for context in view.contexts() {
                for page in view
                    .pages()
                    .iter()
                    .filter(|page| page.context == context.id)
                {
                    let after = before
                        .with_visibility(CollectionMember::Page(page.id), false)
                        .expect("validated visible Page is removed once");
                    self.append(
                        &mut state,
                        PreparedLifecycle {
                            before,
                            after: after.clone(),
                            next: LifecycleNotice::PageClose {
                                context: context.id,
                                page: page.id,
                            },
                            dependent: None,
                            close: Some(page.tree.close_for_teardown()),
                            awaiting: false,
                            creation: None,
                        },
                    );
                    before = after;
                }
                let after = before
                    .with_visibility(CollectionMember::Context(context.id), false)
                    .expect("validated Context is removed after its visible Pages");
                self.append(
                    &mut state,
                    PreparedLifecycle {
                        before,
                        after: after.clone(),
                        next: LifecycleNotice::ContextClose {
                            context: context.id,
                        },
                        dependent: None,
                        close: None,
                        awaiting: false,
                        creation: None,
                    },
                );
                before = after;
            }
        }
        drop(state);
        book.0.changed.notify_one();
    }
}

#[cfg(test)]
mod tests;
