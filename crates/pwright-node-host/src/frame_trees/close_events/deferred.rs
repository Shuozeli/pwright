//! Explicit deferred admission. Ordinary observer preparation is unchanged.
use super::*;
use pwright_composition_chromium::{ChromiumCloseCapacity, ChromiumCloseTarget};

pub(super) struct Capacity {
    owner: Arc<FrameTrees>,
    target: ChromiumCloseTarget,
    reservations: Vec<LifecycleReservation>,
    notices: Vec<Notice>,
    storage: Storage,
}

impl Capacity {
    pub(super) fn new(
        owner: Arc<FrameTrees>,
        target: ChromiumCloseTarget,
    ) -> Result<Self, HostError> {
        // The FrameTrees and collection revision limits agree at sixteen Pages.
        // Reserve future descendants too; do not derive capacity from today's view.
        let maximum = match target {
            ChromiumCloseTarget::Page(_) => 1,
            ChromiumCloseTarget::BrowserContext(_) => super::super::PAGES + 1,
        };
        let state = owner.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let storage = owner.factory.reserve_collection_storage(
            std::mem::size_of::<Self>()
                + std::mem::size_of::<Prepared>()
                + maximum
                    * (std::mem::size_of::<Notice>() + std::mem::size_of::<LifecycleReservation>()),
        )?;
        let mut reservations = Vec::with_capacity(maximum);
        let notices = Vec::with_capacity(maximum);
        for _ in 0..maximum {
            reservations.push(
                owner
                    .urls
                    .publications()
                    .reserve_lifecycle(&owner.factory)?,
            );
        }
        drop(state);
        Ok(Self {
            owner,
            target,
            reservations,
            notices,
            storage,
        })
    }

    fn plan(
        mut self,
        plan: ChromiumClosePlan<'_>,
    ) -> Result<Box<dyn ChromiumCloseReservation>, HostError> {
        if plan.target() != self.target {
            return Err(HostError::InvalidArgument(
                "deferred close plan targets another object".into(),
            ));
        }
        let root = match self.target {
            ChromiumCloseTarget::Page(id) | ChromiumCloseTarget::BrowserContext(id) => id,
        };
        let state = self.owner.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let before = state
            .collections
            .as_ref()
            .and_then(|value| value.planned.clone());
        // Native events or an ancestor can already have retired the local
        // target. A sealed native completion then has an empty latest topology
        // delta. Accept only a publication no-op when this target is no longer
        // visible; never fabricate a Dispose or hide a still-visible target.
        // This is local publication validation, not native-death authority.
        let visible = before.as_ref().is_some_and(|view| match self.target {
            ChromiumCloseTarget::Page(id) => view.pages().iter().any(|page| page.id == id),
            ChromiumCloseTarget::BrowserContext(id) => {
                view.contexts().iter().any(|context| context.id == id)
            }
        });
        if !disposed(plan.effects(), root) && (visible || !plan.effects().is_empty()) {
            return Err(HostError::InvalidArgument(
                "deferred close plan has no disposal of its admitted target".into(),
            ));
        }
        // Reject unrelated disposal effects even if their targets are invisible.
        // A Context may additionally list its known Page descendants; its compact
        // root disposal alone already covers all currently visible descendants.
        for effect in plan.effects() {
            if let CdpTopologyEffect::Lifecycle(ControlMessage::Dispose { object_id }) = effect {
                let valid = *object_id == root
                    || match self.target {
                        ChromiumCloseTarget::Page(_) => false,
                        ChromiumCloseTarget::BrowserContext(context) => {
                            before.as_ref().is_some_and(|view| {
                                view.revision()
                                    .pages()
                                    .iter()
                                    .any(|page| page.id == *object_id && page.context == context)
                            })
                        }
                    };
                if !valid {
                    return Err(HostError::InvalidArgument(
                        "deferred close plan includes an unrelated disposal".into(),
                    ));
                }
            }
        }
        if let Some(view) = &before {
            // Work exclusively from the latest planned mask. Earlier committed
            // closes and visibility changes must not be replayed or overwritten.
            let mut planned = view.clone();
            for context in view.contexts() {
                for page in view.pages().iter().filter(|page| {
                    page.context == context.id
                        && (disposed(plan.effects(), page.id)
                            || disposed(plan.effects(), context.id))
                }) {
                    let member = CollectionMember::Page(page.id);
                    planned = planned
                        .with_visibility(member, false)
                        .map_err(HostError::backend)?;
                    let reservation = self.reservations.pop().ok_or_else(capacity_exhausted)?;
                    self.notices.push(Notice {
                        member,
                        reservation,
                    });
                }
                if disposed(plan.effects(), context.id) {
                    let member = CollectionMember::Context(context.id);
                    planned = planned
                        .with_visibility(member, false)
                        .map_err(HostError::backend)?;
                    let reservation = self.reservations.pop().ok_or_else(capacity_exhausted)?;
                    self.notices.push(Notice {
                        member,
                        reservation,
                    });
                }
            }
        }
        drop(state);
        // The envelope pre-admits both box layouts and both vector buffers.
        // No new group or view storage is reserved in this short phase.
        Ok(Box::new(Prepared {
            owner: self.owner,
            before,
            notices: self.notices,
            _storage: self.storage,
        }))
    }
}

fn capacity_exhausted() -> HostError {
    HostError::ResourceExhausted("deferred close exceeded the declared Page/Context bound".into())
}

impl ChromiumCloseCapacity for Capacity {
    fn prepare(
        self: Box<Self>,
        plan: ChromiumClosePlan<'_>,
    ) -> Result<Box<dyn ChromiumCloseReservation>, OperationError<()>> {
        (*self).plan(plan).map_err(operation)
    }
}
