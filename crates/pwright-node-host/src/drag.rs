//! Explicit Page world-source composition. No host installation, executor or
//! transport lifecycle is selected here. Native cleanup owners retain snapshots.
use pwright_backend_cdp_drag::{
    DragCaptureFactory, DragListenerFactory, DragMove, DragToggleFuture, DragWorld,
    DragWorldLifetime, mouse::DragMoveSource,
};
use pwright_cdp::{
    CdpClient,
    connection::{CdpError, Result},
};
use pwright_composition_chromium::{ChromiumFrameWorldAccess, ChromiumPageUtilityWorldSnapshot};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger,
    ResourceRequirements, ResourceScope,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// Source/retained-world wrapper budgets. Access snapshots, listener/capture/data
/// factories, native provider futures, wire payloads and errors are separate.
#[derive(Clone, Copy)]
pub struct PageDragSourceProfile {
    source_scope: ResourceScope,
    move_scope: ResourceScope,
    worlds: usize,
    moves: usize,
}
impl PageDragSourceProfile {
    pub fn new(
        source_scope: ResourceScope,
        move_scope: ResourceScope,
        worlds: usize,
        moves: usize,
    ) -> Result<Self> {
        if !(1..=32).contains(&worlds) || !(1..=32).contains(&moves) || source_scope == move_scope {
            return Err(error(
                "drag source needs distinct scopes and 1..=32 worlds/moves",
            ));
        }
        Ok(Self {
            source_scope,
            move_scope,
            worlds,
            moves,
        })
    }
    pub fn requirements(self) -> Result<ResourceRequirements> {
        let bytes = (state_bytes() + self.moves * self.retained_bytes()) as u64;
        let mut required = ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, self.moves + 1);
        for _ in 0..self.moves {
            required = required.combine_concurrent(DragMove::requirements(self.worlds)?);
        }
        Ok(required)
    }
    fn retained_bytes(self) -> usize {
        std::mem::size_of::<RetainedWorlds>()
            + 2 * std::mem::size_of::<usize>()
            + self.worlds * std::mem::size_of::<DragWorld<'_>>()
    }
}
fn state_bytes() -> usize {
    std::mem::size_of::<State>() + 2 * std::mem::size_of::<usize>()
}
fn error(message: &str) -> CdpError {
    CdpError::Other(message.into())
}
struct State {
    access: ChromiumFrameWorldAccess,
    page: ObjectId,
    listeners: DragListenerFactory,
    captures: DragCaptureFactory,
    resources: ProductionResourceLedger,
    ledger: ResourceLedger,
    profile: PageDragSourceProfile,
    active: AtomicUsize,
    _storage: ResourceLease,
}
struct Permit {
    state: Arc<State>,
    _storage: ResourceLease,
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.state.active.fetch_sub(1, Ordering::AcqRel);
    }
}
struct RetainedWorlds {
    snapshot: ChromiumPageUtilityWorldSnapshot,
    _permit: Permit,
}
impl DragWorldLifetime for RetainedWorlds {
    fn world_destroyed(&self, client: &Arc<dyn CdpClient>, unique: &str) -> bool {
        self.snapshot.worlds().iter().any(|world| {
            world.utility_unique_id() == unique
                && Arc::ptr_eq(&world.client(), client)
                && world
                    .native_lifetime()
                    .is_some_and(|proof| proof.world_destroyed())
        })
    }
}

/// Immutable Page/capture binding. A fresh batch is acquired for each move; it
/// follows the move into dispatcher quarantine, never cached as current worlds.
#[derive(Clone)]
pub struct PageDragMoveSource(Arc<State>);
impl PageDragMoveSource {
    pub fn new(
        access: ChromiumFrameWorldAccess,
        page: ObjectId,
        listeners: DragListenerFactory,
        captures: DragCaptureFactory,
        profile: PageDragSourceProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self> {
        validate_binding(&access, page, &captures)?;
        let ledger = resources
            .claim(profile.requirements()?)
            .map_err(|_| error("drag source resource claim failed"))?;
        let storage = ledger
            .reserve(
                profile.source_scope,
                ResourceClass::BackendStateBytes,
                state_bytes() as u64,
            )
            .map_err(|_| error("drag source state admission failed"))?;
        Ok(Self(Arc::new(State {
            access,
            page,
            listeners,
            captures,
            resources: resources.clone(),
            ledger,
            profile,
            active: AtomicUsize::new(0),
            _storage: storage,
        })))
    }
    pub fn retained_moves(&self) -> usize {
        self.0.active.load(Ordering::Acquire)
    }
    fn admit(&self) -> Result<Permit> {
        self.0
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < self.0.profile.moves).then(|| count + 1)
            })
            .map_err(|_| error("drag source retained move capacity exceeded"))?;
        match self.0.ledger.reserve(
            self.0.profile.source_scope,
            ResourceClass::BackendStateBytes,
            self.0.profile.retained_bytes() as u64,
        ) {
            Ok(storage) => Ok(Permit {
                state: self.0.clone(),
                _storage: storage,
            }),
            Err(_) => {
                self.0.active.fetch_sub(1, Ordering::AcqRel);
                Err(error("drag source retained storage admission failed"))
            }
        }
    }
}
fn validate_binding(
    access: &ChromiumFrameWorldAccess,
    page: ObjectId,
    captures: &DragCaptureFactory,
) -> Result<()> {
    let binding = access
        .page_input_binding(page)
        .map_err(|_| error("drag source Page binding unavailable"))?
        .ok_or_else(|| error("drag source requires selected Page input"))?;
    if !captures.uses_cdp_client(&binding.into_parts().0) {
        return Err(error("drag source Page/capture native bindings differ"));
    }
    Ok(())
}
impl DragMoveSource for PageDragMoveSource {
    fn captures(&self) -> &DragCaptureFactory {
        &self.0.captures
    }
    fn start(&self, movement: DragToggleFuture) -> Result<DragMove> {
        // Reserve wrapper/vector payload before snapshot/Arc/vector allocation.
        let permit = self.admit()?;
        validate_binding(&self.0.access, self.0.page, &self.0.captures)?;
        let snapshot = self
            .0
            .access
            .existing_page_utility_worlds(self.0.page, self.0.profile.worlds)
            .map_err(|_| error("drag source utility-world snapshot failed"))?;
        let retained = Arc::new(RetainedWorlds {
            snapshot,
            _permit: permit,
        });
        let worlds: Vec<_> = retained
            .snapshot
            .worlds()
            .iter()
            .map(|world| DragWorld {
                client: world.client(),
                unique_context: world.utility_unique_id(),
            })
            .collect();
        DragMove::start_with_world_lifetime(
            self.0.profile.move_scope,
            &self.0.resources,
            &self.0.listeners,
            &self.0.captures,
            &worlds,
            movement,
            Some(retained.clone()),
        )
    }
}

#[cfg(test)]
mod tests;

#[cfg(feature = "drag-runtime")]
pub(crate) mod host;

#[cfg(all(test, feature = "drag-runtime"))]
mod live_frames;
