//! Explicit-close receipts, admitted before the native close transaction.
use super::*;
use crate::{CollectionMember, CollectionView, LifecycleCause, LifecycleReservation};
use pwright_backend_cdp_topology::CdpTopologyEffect;
use pwright_composition_chromium::{
    ChromiumCloseObserver, ChromiumClosePlan, ChromiumCloseReservation,
};
use pwright_core::{ControlMessage, FailureKind, OperationError};

const NOTICES: usize = 32;

mod deferred;

struct Observer {
    owner: Arc<FrameTrees>,
    _storage: Storage,
}

struct Notice {
    member: CollectionMember,
    reservation: LifecycleReservation,
}

struct Prepared {
    owner: Arc<FrameTrees>,
    before: Option<CollectionView>,
    notices: Vec<Notice>,
    _storage: Storage,
}

pub(super) fn disposed(effects: &[CdpTopologyEffect], id: ObjectId) -> bool {
    effects.iter().any(|effect| {
        matches!(effect,
        CdpTopologyEffect::Lifecycle(ControlMessage::Dispose { object_id }) if *object_id == id)
    })
}

fn operation(error: HostError) -> OperationError<()> {
    let kind = match &error {
        HostError::ResourceExhausted(_) => FailureKind::ResourceExhausted,
        HostError::TargetClosed => FailureKind::TargetClosed,
        _ => FailureKind::Backend,
    };
    OperationError::new(kind, error.to_string(), None)
}

impl FrameTrees {
    pub(crate) fn with_explicit_close_publication(self) -> Result<Self, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed || !state.pages.is_empty() {
            return Err(HostError::InvalidArgument(
                "close events must be selected before initialization".into(),
            ));
        }
        let collection = state
            .collections
            .as_mut()
            .ok_or_else(|| HostError::InvalidArgument("close events require collections".into()))?;
        if collection.terminal.is_none() || collection.close_events {
            return Err(HostError::InvalidArgument(
                "close events require a unique terminal selection".into(),
            ));
        }
        collection.close_events = true;
        drop(state);
        Ok(self)
    }

    pub(crate) fn close_observer(
        self: &Arc<Self>,
    ) -> Result<Option<Arc<dyn ChromiumCloseObserver>>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if !state
            .collections
            .as_ref()
            .is_some_and(|value| value.close_events)
        {
            return Ok(None);
        }
        let storage = self.factory.reserve_collection_storage(
            std::mem::size_of::<Observer>() + 2 * std::mem::size_of::<usize>(),
        )?;
        Ok(Some(Arc::new(Observer {
            owner: self.clone(),
            _storage: storage,
        })))
    }
}

impl Observer {
    fn prepare_close(&self, effects: &[CdpTopologyEffect]) -> Result<Prepared, HostError> {
        let state = self.owner.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let before = state
            .collections
            .as_ref()
            .and_then(|value| value.planned.clone());
        let storage = self.owner.factory.reserve_collection_storage(
            std::mem::size_of::<Prepared>() + NOTICES * std::mem::size_of::<Notice>(),
        )?;
        let mut notices = Vec::with_capacity(NOTICES);
        if let Some(view) = &before {
            let mut planned = view.clone();
            for context in view.contexts() {
                for page in view.pages().iter().filter(|page| {
                    page.context == context.id
                        && (disposed(effects, page.id) || disposed(effects, context.id))
                }) {
                    let member = CollectionMember::Page(page.id);
                    planned = planned
                        .with_visibility(member, false)
                        .map_err(HostError::backend)?;
                    notices.push(Notice {
                        member,
                        reservation: self
                            .owner
                            .urls
                            .publications()
                            .reserve_lifecycle(&self.owner.factory)?,
                    });
                }
                if disposed(effects, context.id) {
                    let member = CollectionMember::Context(context.id);
                    // Native Context disposal owns its whole descendant subtree;
                    // the actual topology receipt need not repeat Page disposals.
                    planned = planned
                        .with_visibility(member, false)
                        .map_err(HostError::backend)?;
                    notices.push(Notice {
                        member,
                        reservation: self
                            .owner
                            .urls
                            .publications()
                            .reserve_lifecycle(&self.owner.factory)?,
                    });
                }
            }
        }
        Ok(Prepared {
            owner: self.owner.clone(),
            before,
            notices,
            _storage: storage,
        })
    }
}

impl ChromiumCloseObserver for Observer {
    fn reserve_capacity(
        &self,
        target: pwright_composition_chromium::ChromiumCloseTarget,
    ) -> Result<Box<dyn pwright_composition_chromium::ChromiumCloseCapacity>, OperationError<()>>
    {
        Ok(Box::new(
            deferred::Capacity::new(self.owner.clone(), target).map_err(operation)?,
        ))
    }

    fn prepare(
        &self,
        plan: ChromiumClosePlan<'_>,
    ) -> Result<Box<dyn ChromiumCloseReservation>, OperationError<()>> {
        Ok(Box::new(
            self.prepare_close(plan.effects()).map_err(operation)?,
        ))
    }
}

impl Prepared {
    fn commit(&mut self, effects: &[CdpTopologyEffect]) -> Result<(), HostError> {
        let mut state = self.owner.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let Some(mut before) = self.before.take() else {
            return Ok(());
        };
        let current = state
            .collections
            .as_ref()
            .and_then(|value| value.planned.as_ref());
        if !current.is_some_and(|current| current.same_state(&before)) {
            return Err(HostError::Backend(
                "close publication view changed during native transaction".into(),
            ));
        }
        for member in before
            .contexts()
            .iter()
            .map(|value| CollectionMember::Context(value.id))
            .chain(
                before
                    .pages()
                    .iter()
                    .map(|value| CollectionMember::Page(value.id)),
            )
        {
            let affected = match member {
                CollectionMember::Context(id) => disposed(effects, id),
                CollectionMember::Page(id) => {
                    disposed(effects, id)
                        || before
                            .pages()
                            .iter()
                            .find(|page| page.id == id)
                            .is_some_and(|page| disposed(effects, page.context))
                }
            };
            if affected != self.notices.iter().any(|notice| notice.member == member) {
                return Err(HostError::Backend(
                    "committed close effects differ from admitted visible targets".into(),
                ));
            }
        }
        for notice in self.notices.drain(..) {
            before = stage_notice(
                &self.owner,
                &mut state,
                before,
                notice.member,
                notice.reservation,
            )?;
        }
        Ok(())
    }
}

pub(super) fn stage_notice(
    owner: &FrameTrees,
    state: &mut State,
    before: CollectionView,
    member: CollectionMember,
    reservation: LifecycleReservation,
) -> Result<CollectionView, HostError> {
    let cause = match member {
        CollectionMember::Page(id) => {
            let page = state
                .pages
                .iter()
                .find(|entry| entry.id == id)
                .ok_or_else(|| HostError::Backend("close Page tree is missing".into()))?;
            LifecycleCause::PageClosed {
                page: id,
                close: page.tree.close()?,
            }
        }
        CollectionMember::Context(context) => LifecycleCause::ContextClosed { context },
    };
    let after = owner
        .urls
        .publications()
        .stage_lifecycle(reservation, before, cause)?;
    if let CollectionMember::Page(id) = member {
        // Transfer raw flag ownership only after successful group staging.
        state
            .pages
            .iter_mut()
            .find(|entry| entry.id == id)
            .expect("validated Page")
            .close
            .take();
    }
    state
        .collections
        .as_mut()
        .expect("selected collection owner")
        .planned = Some(after.clone());
    Ok(after)
}

impl ChromiumCloseReservation for Prepared {
    fn committed(
        mut self: Box<Self>,
        effects: &[CdpTopologyEffect],
    ) -> Result<(), OperationError<()>> {
        let result = self.commit(effects);
        if result.is_err() {
            // Native close cannot be rolled back. Force the next reconciliation
            // to retire the Session, preserving the last successfully staged mask.
            self.owner
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .closed = true;
        }
        result.map_err(operation)
    }
}

#[cfg(test)]
pub(in crate::frame_trees) mod tests;
