//! Per-Page creation acknowledgement shared with a later native popup cause.
use crate::{
    HostError,
    frame_tree::{FrameTreeFactory, Storage},
};
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

struct State {
    outcome: AtomicU8,
    _storage: Storage,
}

#[derive(Clone)]
pub(crate) struct CreationDelivery(Arc<State>);

impl CreationDelivery {
    pub(crate) fn reserve(factory: &FrameTreeFactory) -> Result<Self, HostError> {
        let storage = factory.reserve_collection_storage(
            std::mem::size_of::<State>() + 2 * std::mem::size_of::<usize>(),
        )?;
        Ok(Self(Arc::new(State {
            outcome: AtomicU8::new(0),
            _storage: storage,
        })))
    }

    pub(super) fn acknowledge(&self, delivered: bool) {
        self.0
            .outcome
            .store(if delivered { 1 } else { 2 }, Ordering::Release);
    }

    pub(super) fn permits_popup(&self) -> Result<bool, HostError> {
        match self.0.outcome.load(Ordering::Acquire) {
            1 => Ok(true),
            2 => Ok(false),
            _ => Err(HostError::Backend(
                "popup precedes creation acknowledgement".into(),
            )),
        }
    }
}
