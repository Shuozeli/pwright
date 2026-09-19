//! Bounded exact-world marker cache. No persistent Handle, host operation token,
//! or adoption route owner is stored here. The world factory supplies a NEW,
//! independently admitted snapshot for each persistent marker installation.

mod diagnostic;

use crate::HostError;
use pwright_cdp::CdpClient;
use pwright_composition_chromium_file_input::{
    MarkerCreationFuture, MarkerFuture, MarkerOperationError, MarkerWorld, OwnedTargetMarker,
    TargetMarkingOperationService,
};
use pwright_composition_chromium_locator_resolution::{OwnedElement, ResolutionRouteOwner};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger,
    ResourceRequirements, ResourceScope,
};
use pwright_runtime::{CleanupFailure, CleanupReservation, ReservedCleanupExecutor};
use pwright_runtime_tokio::{TokioCleanupExecutor, TokioCleanupReservation};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};
use tokio::sync::Notify;

pub(crate) const CURRENT_WORLDS: usize = 64;
pub(crate) const RETIRING_WORLDS: usize = 8;
pub(crate) const WORLD_SLOTS: usize = CURRENT_WORLDS + RETIRING_WORLDS;
pub(crate) const CALLS: usize = 8;
const MAX_WORLD_BYTES: usize = 1024;
const ADMIN_BYTES: u64 = 128 * 1024;
const WORLD_BYTES: u64 = 16 * 1024;
const WAITER_BYTES: u64 = 4 * 1024;
const SCOPE: ResourceScope = ResourceScope::extension(183, 1);

type Service = TargetMarkingOperationService<TokioCleanupExecutor>;
type Marker = OwnedTargetMarker<TokioCleanupReservation>;

/// Shared errors preserve native kinds for the eventual language policy layer.
/// No diagnostic-string matching or recoverable/fatal policy lives in this cache.
#[derive(Clone, Debug, thiserror::Error)]
pub enum MarkerCacheError {
    #[error(transparent)]
    Host(Arc<HostError>),
    #[error(transparent)]
    Creation(Arc<MarkerOperationError>),
    #[error("target-marker cleanup was not proven; cache admission is closed")]
    Cleanup,
}
impl From<HostError> for MarkerCacheError {
    fn from(error: HostError) -> Self {
        Self::Host(Arc::new(diagnostic::bounded(error)))
    }
}
fn capacity(message: &'static str) -> MarkerCacheError {
    HostError::ResourceExhausted(message.into()).into()
}
fn closed() -> MarkerCacheError {
    HostError::TargetClosed.into()
}

/// Snapshot-only currentness port. Must not retain a source Handle or running
/// host work token. The cache invokes it only outside its own lock.
pub(crate) trait MarkerRoute: Send + Sync {
    fn is_current(&self) -> bool;
}

pub(crate) struct MarkerKey {
    page: ObjectId,
    frame: ObjectId,
    client: Arc<dyn CdpClient>,
    unique: Arc<str>,
}
impl MarkerKey {
    pub(crate) fn new(
        page: ObjectId,
        frame: ObjectId,
        client: Arc<dyn CdpClient>,
        unique: Arc<str>,
    ) -> Result<Self, MarkerCacheError> {
        if unique.is_empty() || unique.len() > MAX_WORLD_BYTES {
            return Err(HostError::InvalidArgument(
                "marker key needs a bounded unique utility world".into(),
            )
            .into());
        }
        Ok(Self {
            page,
            frame,
            client,
            unique,
        })
    }
    fn same_route(&self, other: &Self) -> bool {
        self.page == other.page && self.frame == other.frame
    }
    fn same_world(&self, other: &Self) -> bool {
        self.same_route(other)
            && Arc::ptr_eq(&self.client, &other.client)
            && self.unique == other.unique
    }
    fn matches(&self, world: &MarkerWorld) -> bool {
        Arc::ptr_eq(&self.client, &world.client) && self.unique == world.utility_world
    }
}

#[derive(Clone)]
enum Outcome {
    Ready(Weak<Marker>),
    Error(MarkerCacheError),
}
struct Flight {
    outcome: Mutex<Option<Outcome>>,
    notify: Notify,
}
impl Flight {
    fn new() -> Self {
        Self {
            outcome: Mutex::new(None),
            notify: Notify::new(),
        }
    }
    fn finish(&self, outcome: Outcome) {
        let rejected = {
            let mut current = self.outcome.lock().unwrap_or_else(|p| p.into_inner());
            if current.is_none() {
                *current = Some(outcome);
                None
            } else {
                Some(outcome)
            }
        };
        drop(rejected);
        self.notify.notify_waiters();
    }
    async fn wait(&self) -> Outcome {
        loop {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let result = self
                .outcome
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .clone();
            if let Some(result) = result {
                return result;
            }
            notified.await;
        }
    }
}

enum Phase {
    Creating,
    Abandoned,
    Ready(Arc<Marker>),
    Retiring,
}
struct Entry {
    key: MarkerKey,
    generation: u64,
    retiring: bool,
    phase: Phase,
    waiters: usize,
    flight: Arc<Flight>,
    route: Option<Weak<dyn MarkerRoute>>,
}
struct State {
    sealed: bool,
    next_generation: u64,
    waiters: usize,
    entries: [Option<Entry>; WORLD_SLOTS],
}
struct Inner {
    state: Mutex<State>,
    healthy: AtomicBool,
    changed: Notify,
    service: Arc<Service>,
    drivers: TokioCleanupExecutor,
    ledger: ResourceLedger,
    _storage: ResourceLease,
}

/// Unique cache owner; pending lookups/leases keep its administration alive,
/// but never keep HostSession or its operation tracker alive.
pub(crate) struct MarkerCache(Arc<Inner>);

// Fixed slots plus all maximum world strings, Flight/Notify/Arc backing,
// detached close arrays and bounded administration/control temporaries.
const FIXED_BYTES: usize = std::mem::size_of::<Inner>()
    + WORLD_SLOTS
        * (MAX_WORLD_BYTES + std::mem::size_of::<Flight>() + 4 * std::mem::size_of::<usize>())
    + 2 * std::mem::size_of::<[Option<Arc<Marker>>; WORLD_SLOTS]>()
    + std::mem::size_of::<[Option<Arc<Flight>>; WORLD_SLOTS]>()
    + 4096;
const _: () = assert!(FIXED_BYTES <= ADMIN_BYTES as usize);

impl MarkerCache {
    pub(crate) fn requirements() -> ResourceRequirements {
        let bytes = ADMIN_BYTES + WORLD_SLOTS as u64 * WORLD_BYTES + CALLS as u64 * WAITER_BYTES;
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, 1 + WORLD_SLOTS + CALLS)
    }
    pub(crate) fn new(
        resources: &ProductionResourceLedger,
        service: Arc<Service>,
        drivers: TokioCleanupExecutor,
    ) -> Result<Self, MarkerCacheError> {
        if drivers.job_timeout().is_some() || drivers.metrics().capacity < WORLD_SLOTS {
            return Err(HostError::InvalidArgument(
                "marker creation executor needs 72 untimed slots".into(),
            )
            .into());
        }
        let ledger = resources
            .claim(Self::requirements())
            .map_err(HostError::backend)?;
        let storage = ledger
            .reserve(SCOPE, ResourceClass::BackendStateBytes, ADMIN_BYTES)
            .map_err(|e| HostError::ResourceExhausted(e.to_string()))?;
        Ok(Self(Arc::new(Inner {
            state: Mutex::new(State {
                sealed: false,
                next_generation: 0,
                waiters: 0,
                entries: std::array::from_fn(|_| None),
            }),
            healthy: AtomicBool::new(true),
            changed: Notify::new(),
            service,
            drivers,
            ledger,
            _storage: storage,
        })))
    }
    pub(crate) fn is_cleanup_healthy(&self) -> bool {
        self.0.healthy.load(Ordering::Acquire) && self.0.service.is_cleanup_healthy()
    }

    /// One serialized host-driver observer. notify_one retains a permit when
    /// completion happens before the driver starts its next receive/select.
    pub(crate) async fn changed(&self) {
        self.0.changed.notified().await;
    }

    /// Call at the serialized host topology boundary, outside all host locks.
    pub(crate) fn prune(&self) {
        self.sweep();
    }

    /// The factory runs once for a new entry, after its slot/storage admission,
    /// outside cache locks. It must return snapshot-only persistent backing.
    /// Do not supply a PendingHandle, short-lived adoption route, or work token.
    pub(crate) fn acquire(
        &self,
        key: MarkerKey,
        make_world: impl FnOnce() -> Result<(MarkerWorld, Arc<dyn MarkerRoute>), HostError>,
    ) -> Result<MarkerLookup, MarkerCacheError> {
        self.sweep();
        let mut retired_marker = None;
        let mut retired_flight = None;
        let (waiter, flight, fresh) = {
            let mut state = self.0.state.lock().map_err(HostError::backend)?;
            if state.sealed {
                return Err(closed());
            }
            if !self.is_cleanup_healthy() {
                return Err(MarkerCacheError::Cleanup);
            }
            if state.waiters == CALLS {
                return Err(capacity("marker lookup/call capacity exceeded (8)"));
            }
            let waiter_storage = self
                .0
                .ledger
                .reserve(SCOPE, ResourceClass::BackendStateBytes, WAITER_BYTES)
                .map_err(|e| HostError::ResourceExhausted(e.to_string()))?;
            let exact = state
                .entries
                .iter()
                .position(|entry| entry.as_ref().is_some_and(|e| e.key.same_world(&key)));
            let (slot, generation, flight, fresh) = if let Some(slot) = exact {
                let entry = state.entries[slot].as_ref().expect("found entry");
                if entry.retiring || matches!(entry.phase, Phase::Abandoned | Phase::Retiring) {
                    return Err(closed());
                }
                (slot, entry.generation, entry.flight.clone(), None)
            } else {
                let old = state.entries.iter().position(|entry| {
                    entry
                        .as_ref()
                        .is_some_and(|e| !e.retiring && e.key.same_route(&key))
                });
                let current = state
                    .entries
                    .iter()
                    .flatten()
                    .filter(|e| !e.retiring)
                    .count();
                let retiring = state
                    .entries
                    .iter()
                    .flatten()
                    .filter(|e| e.retiring)
                    .count();
                if old.is_some() && retiring == RETIRING_WORLDS {
                    return Err(capacity("marker retirement capacity exceeded (8)"));
                }
                if old.is_none() && current == CURRENT_WORLDS {
                    return Err(capacity("marker world capacity exceeded (64)"));
                }
                let slot = state
                    .entries
                    .iter()
                    .position(Option::is_none)
                    .ok_or_else(|| capacity("marker world slots exhausted"))?;
                let generation = state
                    .next_generation
                    .checked_add(1)
                    .ok_or_else(|| capacity("marker cache generations exhausted"))?;
                let storage = self
                    .0
                    .ledger
                    .reserve(SCOPE, ResourceClass::BackendStateBytes, WORLD_BYTES)
                    .map_err(|e| HostError::ResourceExhausted(e.to_string()))?;
                let flight = Arc::new(Flight::new());
                // All cache-ledger admission precedes old-owner detachment.
                // Executor admission runs later outside this lock, because its
                // reporter and reservation Drop can synchronously call Rust.
                if let Some(old) = old {
                    let entry = state.entries[old].as_mut().expect("old current entry");
                    entry.retiring = true;
                    if let Phase::Ready(marker) =
                        std::mem::replace(&mut entry.phase, Phase::Retiring)
                    {
                        retired_marker = Some(marker);
                    }
                    retired_flight = Some(entry.flight.clone());
                }
                state.next_generation = generation;
                state.entries[slot] = Some(Entry {
                    key,
                    generation,
                    retiring: false,
                    phase: Phase::Creating,
                    waiters: 0,
                    flight: flight.clone(),
                    route: None,
                });
                (
                    slot,
                    generation,
                    flight,
                    Some(Arc::new(WorldStorage { _lease: storage })),
                )
            };
            state.waiters += 1;
            state.entries[slot]
                .as_mut()
                .expect("admitted entry")
                .waiters += 1;
            (
                Waiter {
                    inner: self.0.clone(),
                    slot,
                    generation,
                    storage: Some(waiter_storage),
                },
                flight,
                fresh,
            )
        };
        drop(retired_marker);
        if let Some(old) = retired_flight {
            old.finish(Outcome::Error(closed()));
        }
        if let Some(storage) = fresh {
            let completion = Completion {
                inner: Arc::downgrade(&self.0),
                slot: waiter.slot,
                generation: waiter.generation,
                route: None,
                _storage: storage.clone(),
            };
            let mut init = Init {
                completion: Some(completion),
                flight: flight.clone(),
            };
            let reservation = match self.0.drivers.try_reserve() {
                Ok(reservation) => reservation,
                Err(error) => {
                    let error = MarkerCacheError::from(HostError::backend(error));
                    init.fail(error.clone());
                    return Err(error);
                }
            };
            let (world, route) = match make_world() {
                Ok(world) => world,
                Err(error) => {
                    let error = MarkerCacheError::from(error);
                    init.fail(error.clone());
                    return Err(error);
                }
            };
            let current = route.is_current();
            let valid = {
                let mut state = self.0.state.lock().map_err(HostError::backend)?;
                !state.sealed
                    && current
                    && state.entries[waiter.slot].as_mut().is_some_and(|entry| {
                        let valid = entry.generation == waiter.generation
                            && matches!(entry.phase, Phase::Creating)
                            && entry.key.matches(&world);
                        if valid {
                            entry.route = Some(Arc::downgrade(&route));
                        }
                        valid
                    })
            };
            if !valid {
                let error: MarkerCacheError = HostError::InvalidArgument(
                    "marker world factory returned a stale or foreign world".into(),
                )
                .into();
                init.fail(error.clone());
                return Err(error);
            }
            init.completion.as_mut().expect("initial completion").route = Some(route);
            let completion: ResolutionRouteOwner =
                Arc::new(init.completion.take().expect("initial completion"));
            let attempt = match self.0.service.create(world, Some(completion)) {
                Ok(attempt) => attempt,
                Err(error) => {
                    let error = MarkerCacheError::Creation(Arc::new(error));
                    init.fail(error.clone());
                    return Err(error);
                }
            };
            // Init no longer owns native completion; the service's route owner
            // now retains it through creation and every terminal cleanup path.
            let work = DriverWork {
                driver: Driver {
                    inner: self.0.clone(),
                    flight: flight.clone(),
                    finished: false,
                },
                attempt,
                _storage: storage,
                slot: waiter.slot,
                generation: waiter.generation,
            };
            let future = drive(work);
            const fn check_driver<F>(_: &F) {
                assert!(
                    std::mem::size_of::<F>()
                        + std::mem::size_of::<Completion>()
                        + std::mem::size_of::<Driver>()
                        + 4096
                        < WORLD_BYTES as usize
                );
            }
            check_driver(&future);
            reservation.submit(Box::pin(future));
        }
        Ok(MarkerLookup::new(waiter, flight))
    }

    /// Bounded stale-world sweep, also covering removed Frames and closed Pages.
    /// Route callbacks, strong-route drops, owner drops and wakeups are outside
    /// the cache lock. Generation checks reject stale observations after reuse.
    fn sweep(&self) {
        let observations: [Option<(u64, Weak<dyn MarkerRoute>)>; WORLD_SLOTS] = {
            let state = self.0.state.lock().unwrap_or_else(|p| p.into_inner());
            if state.sealed {
                return;
            }
            std::array::from_fn(|slot| {
                state.entries[slot].as_ref().and_then(|entry| {
                    (!entry.retiring && matches!(entry.phase, Phase::Creating | Phase::Ready(_)))
                        .then(|| {
                            entry
                                .route
                                .as_ref()
                                .map(|route| (entry.generation, route.clone()))
                        })
                        .flatten()
                })
            })
        };
        let stale: [Option<u64>; WORLD_SLOTS] = observations.map(|observation| {
            observation.and_then(|(generation, route)| {
                let current = route.upgrade().is_some_and(|route| route.is_current());
                (!current).then_some(generation)
            })
        });
        let (markers, flights) = {
            let mut state = self.0.state.lock().unwrap_or_else(|p| p.into_inner());
            let mut retiring = state
                .entries
                .iter()
                .flatten()
                .filter(|e| e.retiring)
                .count();
            let mut markers: [Option<Arc<Marker>>; WORLD_SLOTS] = std::array::from_fn(|_| None);
            let mut flights: [Option<Arc<Flight>>; WORLD_SLOTS] = std::array::from_fn(|_| None);
            for (slot, generation) in stale.into_iter().enumerate() {
                let Some(generation) = generation else {
                    continue;
                };
                let Some(entry) = state.entries[slot]
                    .as_mut()
                    .filter(|e| e.generation == generation && !e.retiring)
                else {
                    continue;
                };
                if retiring < RETIRING_WORLDS {
                    entry.retiring = true;
                    retiring += 1;
                }
                // If overflow retirement is full, keep this unusable current
                // slot charged until actual cleanup, rather than free it early.
                if let Phase::Ready(marker) = std::mem::replace(&mut entry.phase, Phase::Abandoned)
                {
                    markers[slot] = Some(marker);
                }
                flights[slot] = Some(entry.flight.clone());
            }
            (markers, flights)
        };
        let changed = flights.iter().any(Option::is_some);
        for flight in flights.into_iter().flatten() {
            flight.finish(Outcome::Error(closed()));
        }
        drop(markers);
        if changed {
            self.0.changed.notify_one();
        }
    }

    /// Safe while the host admission lock is held: no owner Drop or wakeup.
    pub(crate) fn seal(&self) {
        self.0
            .state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .sealed = true;
    }

    /// Call outside host locks AFTER retiring owned inspector sessions. Owners
    /// and waiters are released/woken outside the cache lock as well.
    pub(crate) fn close(&self) {
        let (markers, flights) = {
            let mut state = self.0.state.lock().unwrap_or_else(|p| p.into_inner());
            state.sealed = true;
            let mut markers: [Option<Arc<Marker>>; WORLD_SLOTS] = std::array::from_fn(|_| None);
            let mut flights: [Option<Arc<Flight>>; WORLD_SLOTS] = std::array::from_fn(|_| None);
            for (slot, entry) in state.entries.iter_mut().enumerate() {
                if let Some(entry) = entry {
                    // Once sealed, all 72 occupied slots may drain concurrently;
                    // the 8-slot replacement headroom limits OPEN admission only.
                    entry.retiring = true;
                    if let Phase::Ready(marker) =
                        std::mem::replace(&mut entry.phase, Phase::Retiring)
                    {
                        markers[slot] = Some(marker);
                    }
                    flights[slot] = Some(entry.flight.clone());
                }
            }
            (markers, flights)
        };
        for flight in flights.into_iter().flatten() {
            flight.finish(Outcome::Error(closed()));
        }
        drop(markers);
    }

    #[cfg(test)]
    pub(crate) fn metrics(&self) -> Metrics {
        let state = self.0.state.lock().unwrap_or_else(|p| p.into_inner());
        Metrics {
            current: state
                .entries
                .iter()
                .flatten()
                .filter(|e| !e.retiring)
                .count(),
            creating: state
                .entries
                .iter()
                .flatten()
                .filter(|e| matches!(e.phase, Phase::Creating))
                .count(),
            retiring: state
                .entries
                .iter()
                .flatten()
                .filter(|e| e.retiring)
                .count(),
            waiters: state.waiters,
        }
    }
}
impl Drop for MarkerCache {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Metrics {
    pub(crate) current: usize,
    pub(crate) creating: usize,
    pub(crate) retiring: usize,
    pub(crate) waiters: usize,
}

struct WorldStorage {
    _lease: ResourceLease,
}
struct Completion {
    inner: Weak<Inner>,
    slot: usize,
    generation: u64,
    route: Option<Arc<dyn MarkerRoute>>,
    _storage: Arc<WorldStorage>,
}
impl Drop for Completion {
    fn drop(&mut self) {
        // Drop the independently admitted snapshot before advertising capacity.
        drop(self.route.take());
        if let Some(inner) = self.inner.upgrade() {
            inner.complete(self.slot, self.generation);
        }
    }
}
struct Init {
    completion: Option<Completion>,
    flight: Arc<Flight>,
}
impl Init {
    fn fail(&self, error: MarkerCacheError) {
        self.flight.finish(Outcome::Error(error));
    }
}
impl Drop for Init {
    fn drop(&mut self) {
        if self.completion.is_some() {
            self.flight.finish(Outcome::Error(closed()));
        }
    }
}
struct Driver {
    inner: Arc<Inner>,
    flight: Arc<Flight>,
    finished: bool,
}
// One captured owner before the first poll: an executor abandoning unpolled
// work must wake pending lookups and poison admission before backing returns.
struct DriverWork {
    driver: Driver,
    attempt: MarkerCreationFuture<TokioCleanupReservation>,
    _storage: Arc<WorldStorage>,
    slot: usize,
    generation: u64,
}
async fn drive(work: DriverWork) -> Result<(), CleanupFailure> {
    let DriverWork {
        driver,
        attempt,
        _storage,
        slot,
        generation,
    } = work;
    if !driver.inner.can_create(slot, generation) {
        drop(attempt);
        driver.flight.finish(Outcome::Error(closed()));
        return driver.done(Ok(()));
    }
    match attempt.await {
        Ok(marker) => {
            let marker = Arc::new(marker);
            let published = driver.inner.publish(slot, generation, marker.clone());
            if published {
                driver
                    .flight
                    .finish(Outcome::Ready(Arc::downgrade(&marker)));
            } else {
                driver.flight.finish(Outcome::Error(closed()));
            }
            drop(marker);
            driver.done(Ok(()))
        }
        Err(error) => {
            let failed_cleanup = !driver.inner.service.is_cleanup_healthy();
            driver
                .flight
                .finish(Outcome::Error(MarkerCacheError::Creation(Arc::new(error))));
            driver.done(if failed_cleanup {
                Err(CleanupFailure::new("marker creation cleanup failed"))
            } else {
                Ok(())
            })
        }
    }
}
impl Driver {
    fn done(mut self, result: Result<(), CleanupFailure>) -> Result<(), CleanupFailure> {
        self.finished = true;
        result
    }
}
impl Drop for Driver {
    fn drop(&mut self) {
        if !self.finished {
            self.inner.healthy.store(false, Ordering::Release);
            self.flight
                .finish(Outcome::Error(MarkerCacheError::Cleanup));
        }
    }
}

impl Inner {
    fn route_current(&self, slot: usize, generation: u64) -> bool {
        let route = {
            let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
            state.entries[slot]
                .as_ref()
                .filter(|entry| entry.generation == generation)
                .and_then(|entry| entry.route.clone())
        };
        // This is an arbitrary injected Rust port. Never invoke it or drop its
        // upgraded strong owner while holding the cache/admission mutex.
        route
            .and_then(|route| route.upgrade())
            .is_some_and(|route| route.is_current())
    }
    fn can_create(&self, slot: usize, generation: u64) -> bool {
        let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        !state.sealed
            && state.entries[slot].as_ref().is_some_and(|entry| {
                entry.generation == generation
                    && matches!(entry.phase, Phase::Creating)
                    && entry.waiters != 0
            })
    }
    fn publish(&self, slot: usize, generation: u64, marker: Arc<Marker>) -> bool {
        let mut marker = Some(marker);
        let current = self.route_current(slot, generation);
        let published = {
            let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
            if current && !state.sealed {
                if let Some(entry) = state.entries[slot].as_mut() {
                    if entry.generation == generation
                        && matches!(entry.phase, Phase::Creating)
                        && entry.waiters != 0
                    {
                        entry.phase = Phase::Ready(marker.take().expect("unpublished marker"));
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };
        drop(marker);
        published
    }
    fn complete(&self, slot: usize, generation: u64) {
        let removed = {
            let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
            if state.entries[slot]
                .as_ref()
                .is_none_or(|e| e.generation != generation)
            {
                return;
            }
            if !self.service.is_cleanup_healthy() {
                self.healthy.store(false, Ordering::Release);
                return;
            }
            state.entries[slot].take()
        };
        drop(removed);
        self.changed.notify_one();
    }
}

struct Waiter {
    inner: Arc<Inner>,
    slot: usize,
    generation: u64,
    storage: Option<ResourceLease>,
}
impl Drop for Waiter {
    fn drop(&mut self) {
        // Future/lease payload fields are destroyed before this admission.
        self.storage = None;
        let flight = {
            let mut state = self.inner.state.lock().unwrap_or_else(|p| p.into_inner());
            state.waiters -= 1;
            let retiring = state
                .entries
                .iter()
                .flatten()
                .filter(|e| e.retiring)
                .count();
            if let Some(entry) = state.entries[self.slot]
                .as_mut()
                .filter(|e| e.generation == self.generation)
            {
                entry.waiters -= 1;
                if entry.waiters == 0 && matches!(entry.phase, Phase::Creating) {
                    entry.phase = Phase::Abandoned;
                    if retiring < RETIRING_WORLDS {
                        entry.retiring = true;
                    }
                    Some(entry.flight.clone())
                } else {
                    None
                }
            } else {
                None
            }
        };
        if let Some(flight) = flight {
            flight.finish(Outcome::Error(closed()));
        }
    }
}

#[must_use]
pub(crate) struct MarkerLookup {
    inner: Pin<Box<dyn Future<Output = Result<MarkerLease, MarkerCacheError>> + Send>>,
}
impl MarkerLookup {
    fn new(waiter: Waiter, flight: Arc<Flight>) -> Self {
        let future = async move {
            match flight.wait().await {
                Outcome::Error(error) => Err(error),
                Outcome::Ready(marker) => {
                    let current = waiter.inner.route_current(waiter.slot, waiter.generation);
                    let valid = {
                        let state = waiter.inner.state.lock().map_err(HostError::backend)?;
                        current
                            && !state.sealed
                            && state.entries[waiter.slot].as_ref().is_some_and(|entry| {
                                entry.generation == waiter.generation
                                    && matches!(entry.phase, Phase::Ready(_))
                            })
                    };
                    if !valid {
                        return Err(closed());
                    }
                    let marker = marker.upgrade().ok_or_else(closed)?;
                    Ok(MarkerLease { marker, waiter })
                }
            }
        };
        const fn check<F>(_: &F) {
            assert!(
                std::mem::size_of::<F>()
                    + std::mem::size_of::<MarkerLease>()
                    // Creation cleanup can retain a primary AND cleanup
                    // diagnostic; cache-origin Host errors share one cap.
                    + 2 * diagnostic::BYTES
                    + 512
                    < WAITER_BYTES as usize
            );
        }
        check(&future);
        Self {
            inner: Box::pin(future),
        }
    }
}
impl Future for MarkerLookup {
    type Output = Result<MarkerLease, MarkerCacheError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.as_mut().poll(cx)
    }
}

/// A short-lived admitted user, NOT a persistent cache entry. The host includes
/// an Arc of this lease in the ADOPTED target's route owner so cancellation keeps
/// admission through native target cleanup. Never put that route in the cache.
pub(crate) struct MarkerLease {
    marker: Arc<Marker>,
    waiter: Waiter,
}
impl MarkerLease {
    pub(crate) fn mark(
        &self,
        element: OwnedElement<TokioCleanupReservation>,
        call_id: Arc<str>,
    ) -> Result<MarkerFuture<TokioCleanupReservation, TokioCleanupReservation>, MarkerCacheError>
    {
        let current = self
            .waiter
            .inner
            .route_current(self.waiter.slot, self.waiter.generation);
        let valid = {
            let state = self.waiter.inner.state.lock().map_err(HostError::backend)?;
            current
                && !state.sealed
                && state.entries[self.waiter.slot]
                    .as_ref()
                    .is_some_and(|entry| {
                        entry.generation == self.waiter.generation
                            && !entry.retiring
                            && matches!(entry.phase, Phase::Ready(_))
                    })
        };
        if !valid {
            return Err(closed());
        }
        self.waiter
            .inner
            .service
            .mark(&self.marker, element, call_id)
            .map_err(|error| MarkerCacheError::Creation(Arc::new(error)))
    }
}

#[cfg(test)]
mod tests;
