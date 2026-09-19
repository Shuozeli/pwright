//! Persistent, private exact-world marker ownership. The caller chooses the
//! world's lifetime; this module creates no world registry, timer or runtime.
//! Per-call target groups never own the persistent marker state group.

use super::*;
use pwright_backend_cdp_file_input::{MarkerAdmission, MarkerState};
use pwright_composition_chromium_locator_resolution::{ResolutionRouteOwner, ResolvedElement};
use std::sync::Mutex;

const MAX_CALL_ID_BYTES: usize = 128;

#[derive(Debug, thiserror::Error)]
pub enum MarkerOperationError {
    #[error(transparent)]
    Owner(#[from] UploadError),
    #[error(transparent)]
    Backend(#[from] TargetMarkingError),
    #[error("target marking requires settlement-owned target group cleanup")]
    OrdinaryGroup,
    #[error("marker belongs to another operation service")]
    ForeignMarker,
    #[error("target-marking call identifier exceeds the selected limit")]
    CallId,
    #[error("target-marking group cleanup failed: {cleanup}; primary: {primary:?}")]
    Cleanup {
        primary: Option<Box<Self>>,
        cleanup: ResolutionError,
    },
}

struct Pool {
    profile: UploadProfile,
    ledger: ResourceLedger,
    state: Arc<State>,
}
const _: () = assert!(
    std::mem::size_of::<Pool>() + std::mem::size_of::<State>() + 2 * std::mem::size_of::<usize>()
        <= STATE_BYTES as usize
);
impl Pool {
    fn new(
        profile: UploadProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, UploadError> {
        let ledger = resources.claim(profile.requirements())?;
        let lease = ledger.reserve(profile.scope, ResourceClass::BackendStateBytes, STATE_BYTES)?;
        Ok(Self {
            profile,
            ledger,
            state: Arc::new(State {
                active: AtomicUsize::new(0),
                healthy: AtomicBool::new(true),
                _lease: lease,
            }),
        })
    }

    fn admit(&self) -> Result<Owner, UploadError> {
        if !self.state.healthy.load(Ordering::Acquire) {
            return Err(UploadError::Unhealthy);
        }
        self.state
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < self.profile.capacity).then(|| count + 1)
            })
            .map_err(|_| UploadError::Capacity)?;
        let mut owner = Owner {
            state: self.state.clone(),
            lease: None,
            armed: false,
        };
        owner.lease = Some(self.ledger.reserve_many(
            self.profile.scope,
            &[
                ResourceRequest::new(ResourceClass::LiveObject, 1),
                ResourceRequest::new(
                    ResourceClass::BackendStateBytes,
                    self.profile.owner_bytes as u64,
                ),
            ],
        )?);
        Ok(owner)
    }

    fn check_storage(&self, bytes: Option<usize>) -> Result<(), UploadError> {
        if bytes.is_none_or(|bytes| bytes > self.profile.owner_bytes) {
            Err(UploadError::Storage)
        } else {
            Ok(())
        }
    }
}

/// Two independently selected UploadProfiles admit persistent world owners and
/// call owners. Their scopes must be distinct, and backend native capacities,
/// route snapshots and executor reservations are selected separately. No
/// browser-reachable DOM graph byte bound is implied by a native wrapper bound.
pub struct TargetMarkingOperationService<E> {
    states: Pool,
    calls: Pool,
    backend: Arc<CdpTargetMarkingService>,
    cleanup: E,
}
impl<E: ReservedCleanupExecutor> TargetMarkingOperationService<E> {
    pub fn new(
        state_profile: UploadProfile,
        call_profile: UploadProfile,
        resources: &ProductionResourceLedger,
        backend: Arc<CdpTargetMarkingService>,
        cleanup: E,
    ) -> Result<Self, MarkerOperationError> {
        if state_profile.scope == call_profile.scope {
            return Err(UploadError::InvalidProfile.into());
        }
        Ok(Self {
            states: Pool::new(state_profile, resources)?,
            calls: Pool::new(call_profile, resources)?,
            backend,
            cleanup,
        })
    }

    pub fn is_cleanup_healthy(&self) -> bool {
        self.backend.is_cleanup_healthy()
            && self.states.state.healthy.load(Ordering::Acquire)
            && self.calls.state.healthy.load(Ordering::Acquire)
    }

    pub fn active_states(&self) -> usize {
        self.states.state.active.load(Ordering::Acquire)
    }

    pub fn active_calls(&self) -> usize {
        self.calls.state.active.load(Ordering::Acquire)
    }

    /// Reserve the same continuation which will later release the private
    /// group. Unpolled creation performs no browser work. The supplied route
    /// owner and destruction receipt must describe this exact utility world;
    /// a receipt with a single waiter must not be shared by concurrent releases.
    pub fn create(
        &self,
        world: MarkerWorld,
        route_owner: Option<ResolutionRouteOwner>,
    ) -> Result<MarkerCreationFuture<E::Reservation>, MarkerOperationError> {
        if !self.is_cleanup_healthy() {
            return Err(UploadError::Unhealthy.into());
        }
        let mut owner = self.states.admit()?;
        let reservation = self.cleanup.try_reserve().map_err(UploadError::Admission)?;
        let backend = self.backend.clone();
        let attempt = backend.create(world)?;
        let future = async move {
            owner.arm();
            match attempt.await {
                Ok(marker) => Ok(StateReady {
                    owner,
                    marker,
                    route_owner,
                }),
                Err(error) => {
                    // Backend creation compensates its pre-owned group before
                    // returning. A cleanup failure must remain visible to a
                    // dropped caller and poison admission before lease return.
                    if backend.is_cleanup_healthy() {
                        owner.disarm();
                    }
                    Err(MarkerOperationError::Backend(error))
                }
            }
        };
        let bytes = std::mem::size_of_val(&future)
            .checked_add(future_storage(release_state))
            .and_then(|n| n.checked_add(std::mem::size_of::<StateBacking<E::Reservation>>()))
            .and_then(|n| {
                n.checked_add(std::mem::size_of::<MarkerCreationFuture<E::Reservation>>())
            })
            .and_then(|n| n.checked_add(std::mem::size_of::<MarkerReleaseFuture<E::Reservation>>()))
            .and_then(|n| n.checked_add(4 * CONTROL_ALLOWANCE));
        self.states.check_storage(bytes)?;
        Ok(MarkerCreationFuture {
            inner: Some(Box::pin(future)),
            reservation: Some(reservation),
            started: false,
        })
    }

    /// Mark the original adopted node, never a selector requery or a retargeted
    /// INPUT. The result retains the call's extra native allowance until that
    /// target group's acknowledged cleanup. A normal/canceled mark does not
    /// reset or release the persistent world's marker state.
    pub fn mark<R: CleanupReservation>(
        &self,
        marker: &OwnedTargetMarker<E::Reservation>,
        element: OwnedElement<R>,
        call_id: Arc<str>,
    ) -> Result<MarkerFuture<R, E::Reservation>, MarkerOperationError> {
        if !self.is_cleanup_healthy() {
            return Err(UploadError::Unhealthy.into());
        }
        let backing = marker.backing.as_ref().expect("live marker owner");
        if !Arc::ptr_eq(
            &backing
                .ready
                .as_ref()
                .expect("live marker state")
                .owner
                .state,
            &self.states.state,
        ) {
            return Err(MarkerOperationError::ForeignMarker);
        }
        if !element.element().uses_settlement_release() {
            return Err(MarkerOperationError::OrdinaryGroup);
        }
        if call_id.len() > MAX_CALL_ID_BYTES {
            return Err(MarkerOperationError::CallId);
        }
        let owner = self.calls.admit()?;
        let admission = self.backend.admit()?;
        let reservation = self.cleanup.try_reserve().map_err(UploadError::Admission)?;
        let backing = backing.clone();
        let future = async move {
            let _backing = backing;
            let mut ready = CallReady {
                owner,
                element,
                admission,
            };
            ready.owner.arm();
            let result = ready
                .admission
                .mark(
                    &_backing.ready.as_ref().expect("live marker state").marker,
                    ready.element.element(),
                    &call_id,
                )
                .await;
            match result {
                Ok(()) => Ok(ready),
                Err(error) => {
                    let primary = MarkerOperationError::Backend(error);
                    match release_call(ready).await {
                        Ok(()) => Err(primary),
                        Err(MarkerOperationError::Cleanup { cleanup, .. }) => {
                            Err(MarkerOperationError::Cleanup {
                                primary: Some(Box::new(primary)),
                                cleanup,
                            })
                        }
                        Err(error) => Err(error),
                    }
                }
            }
        };
        let bytes = std::mem::size_of_val(&future)
            .checked_add(future_storage(release_call::<R>))
            .and_then(|n| n.checked_add(std::mem::size_of::<MarkerFuture<R, E::Reservation>>()))
            .and_then(|n| {
                n.checked_add(std::mem::size_of::<OwnedMarkedElement<R, E::Reservation>>())
            })
            .and_then(|n| n.checked_add(std::mem::size_of::<MarkerReleaseFuture<E::Reservation>>()))
            .and_then(|n| n.checked_add(MAX_CALL_ID_BYTES + 4 * CONTROL_ALLOWANCE));
        self.calls.check_storage(bytes)?;
        Ok(MarkerFuture {
            inner: Some(Box::pin(future)),
            reservation: Some(reservation),
            started: false,
        })
    }
}

fn future_storage<Input, F: Future>(_: impl FnOnce(Input) -> F) -> usize {
    std::mem::size_of::<F>()
}

// Armed owner precedes all backing allowances so an executor which discards
// its work publishes unhealthy before capacity returns. This cannot establish
// native cleanup when the supplied executor abandons its promised jobs.
struct StateReady {
    owner: Owner,
    marker: MarkerState,
    route_owner: Option<ResolutionRouteOwner>,
}
struct CallReady<R: CleanupReservation> {
    owner: Owner,
    element: OwnedElement<R>,
    admission: MarkerAdmission,
}
type Creation = Pin<Box<dyn Future<Output = Result<StateReady, MarkerOperationError>> + Send>>;
type Mark<R> = Pin<Box<dyn Future<Output = Result<CallReady<R>, MarkerOperationError>> + Send>>;
type Release = Pin<Box<dyn Future<Output = Result<(), MarkerOperationError>> + Send>>;

async fn release_state(ready: StateReady) -> Result<(), MarkerOperationError> {
    let StateReady {
        mut owner,
        marker,
        route_owner: _route_owner,
    } = ready;
    match marker.release().await {
        Ok(()) => {
            owner.disarm();
            Ok(())
        }
        Err(error) => {
            owner.state.healthy.store(false, Ordering::Release);
            Err(MarkerOperationError::Backend(error))
        }
    }
}

async fn release_call<R: CleanupReservation>(
    ready: CallReady<R>,
) -> Result<(), MarkerOperationError> {
    let CallReady {
        owner,
        element,
        admission,
    } = ready;
    struct Backing {
        owner: Owner,
        _admission: MarkerAdmission,
    }
    let mut backing = Backing {
        owner,
        _admission: admission,
    };
    match element.release().await {
        Ok(()) => {
            backing._admission.complete_cleanup();
            backing.owner.disarm();
            Ok(())
        }
        Err(cleanup) => {
            backing.owner.state.healthy.store(false, Ordering::Release);
            Err(MarkerOperationError::Cleanup {
                primary: None,
                cleanup,
            })
        }
    }
}

fn cleanup_error(_: MarkerOperationError) -> CleanupFailure {
    CleanupFailure::new("target-marking group cleanup failed; admission closed")
}

struct StateBacking<C: CleanupReservation> {
    ready: Option<StateReady>,
    // CleanupReservation promises Send, not Sync. No lock is held across a
    // native call or executor callback; only exclusive final ownership takes it.
    reservation: Mutex<Option<C>>,
}
impl<C: CleanupReservation> Drop for StateBacking<C> {
    fn drop(&mut self) {
        if let (Some(ready), Some(reservation)) = (
            self.ready.take(),
            self.reservation
                .get_mut()
                .unwrap_or_else(|p| p.into_inner())
                .take(),
        ) {
            reservation.submit(Box::pin(async move {
                release_state(ready).await.map_err(cleanup_error)
            }));
        }
    }
}

/// One persistent private utility-world state. It is deliberately not Clone;
/// only admitted in-flight operations retain its internal backing. Drop queues
/// group cleanup after the last such operation, without clearing state between
/// calls. World-keying, replacement and explicit session shutdown are callers'
/// responsibilities; this is not a native-use pin against world destruction.
#[must_use = "retain the marker for its exact world, then release or drop it"]
pub struct OwnedTargetMarker<C: CleanupReservation> {
    backing: Option<Arc<StateBacking<C>>>,
}
impl<C: CleanupReservation> OwnedTargetMarker<C> {
    /// Explicit cleanup is available only when no native mark is in flight.
    /// Busy returns the SAME owner unchanged; dropping it still safely defers
    /// cleanup until outstanding mark continuations relinquish their references.
    pub fn release(mut self) -> Result<MarkerReleaseFuture<C>, Self> {
        let backing = self.backing.take().expect("live marker owner");
        match Arc::try_unwrap(backing) {
            Ok(mut backing) => Ok(MarkerReleaseFuture {
                inner: Some(Box::pin(release_state(
                    backing.ready.take().expect("live marker state"),
                ))),
                reservation: backing
                    .reservation
                    .get_mut()
                    .unwrap_or_else(|p| p.into_inner())
                    .take(),
            }),
            Err(backing) => {
                self.backing = Some(backing);
                Err(self)
            }
        }
    }
}

#[must_use = "await or drop to continue admitted marker creation and cleanup"]
pub struct MarkerCreationFuture<C: CleanupReservation> {
    inner: Option<Creation>,
    reservation: Option<C>,
    started: bool,
}
impl<C: CleanupReservation> Unpin for MarkerCreationFuture<C> {}
impl<C: CleanupReservation> Future for MarkerCreationFuture<C> {
    type Output = Result<OwnedTargetMarker<C>, MarkerOperationError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        match self
            .inner
            .as_mut()
            .expect("creation completed")
            .as_mut()
            .poll(cx)
        {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                self.inner = None;
                let reservation = self.reservation.take();
                Poll::Ready(result.map(|ready| OwnedTargetMarker {
                    backing: Some(Arc::new(StateBacking {
                        ready: Some(ready),
                        reservation: Mutex::new(reservation),
                    })),
                }))
            }
        }
    }
}
impl<C: CleanupReservation> Drop for MarkerCreationFuture<C> {
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        if let (Some(inner), Some(reservation)) = (self.inner.take(), self.reservation.take()) {
            reservation.submit(Box::pin(async move {
                match inner.await {
                    Ok(ready) => release_state(ready).await.map_err(cleanup_error),
                    Err(MarkerOperationError::Backend(error)) if error.is_cleanup_failure() => {
                        Err(cleanup_error(MarkerOperationError::Backend(error)))
                    }
                    Err(_) => Ok(()),
                }
            }));
        }
    }
}

#[must_use = "await or drop to continue admitted marking and target cleanup"]
pub struct MarkerFuture<R: CleanupReservation, C: CleanupReservation> {
    inner: Option<Mark<R>>,
    reservation: Option<C>,
    started: bool,
}
impl<R: CleanupReservation, C: CleanupReservation> Unpin for MarkerFuture<R, C> {}
impl<R: CleanupReservation, C: CleanupReservation> Future for MarkerFuture<R, C> {
    type Output = Result<OwnedMarkedElement<R, C>, MarkerOperationError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        match self
            .inner
            .as_mut()
            .expect("mark completed")
            .as_mut()
            .poll(cx)
        {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                self.inner = None;
                let reservation = self.reservation.take();
                Poll::Ready(result.map(|ready| OwnedMarkedElement {
                    ready: Some(ready),
                    reservation,
                }))
            }
        }
    }
}
impl<R: CleanupReservation, C: CleanupReservation> Drop for MarkerFuture<R, C> {
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        if let (Some(inner), Some(reservation)) = (self.inner.take(), self.reservation.take()) {
            reservation.submit(Box::pin(async move {
                match inner.await {
                    Ok(ready) => release_call(ready).await.map_err(cleanup_error),
                    Err(error @ MarkerOperationError::Cleanup { .. }) => Err(cleanup_error(error)),
                    Err(_) => Ok(()),
                }
            }));
        }
    }
}

/// Exact adopted target with its marking-call admission. No conversion can
/// detach the element from that allowance before terminal group cleanup.
#[must_use = "retain, release, or drop the marked target owner"]
pub struct OwnedMarkedElement<R: CleanupReservation, C: CleanupReservation> {
    ready: Option<CallReady<R>>,
    reservation: Option<C>,
}
impl<R: CleanupReservation, C: CleanupReservation> OwnedMarkedElement<R, C> {
    pub fn element(&self) -> &ResolvedElement {
        self.ready
            .as_ref()
            .expect("live marked target")
            .element
            .element()
    }

    pub fn release(mut self) -> MarkerReleaseFuture<C> {
        MarkerReleaseFuture {
            inner: Some(Box::pin(release_call(
                self.ready.take().expect("live marked target"),
            ))),
            reservation: self.reservation.take(),
        }
    }
}
impl<R: CleanupReservation, C: CleanupReservation> Drop for OwnedMarkedElement<R, C> {
    fn drop(&mut self) {
        if let (Some(ready), Some(reservation)) = (self.ready.take(), self.reservation.take()) {
            reservation.submit(Box::pin(async move {
                release_call(ready).await.map_err(cleanup_error)
            }));
        }
    }
}

/// The original cleanup reservation follows explicit release even when it is
/// dropped before polling or while native group release is still pending.
#[must_use = "await or drop to continue admitted group cleanup"]
pub struct MarkerReleaseFuture<C: CleanupReservation> {
    inner: Option<Release>,
    reservation: Option<C>,
}
impl<C: CleanupReservation> Unpin for MarkerReleaseFuture<C> {}
impl<C: CleanupReservation> Future for MarkerReleaseFuture<C> {
    type Output = Result<(), MarkerOperationError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result = self
            .inner
            .as_mut()
            .expect("release completed")
            .as_mut()
            .poll(cx);
        if result.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        result
    }
}
impl<C: CleanupReservation> Drop for MarkerReleaseFuture<C> {
    fn drop(&mut self) {
        if let (Some(inner), Some(reservation)) = (self.inner.take(), self.reservation.take()) {
            reservation.submit(Box::pin(async move { inner.await.map_err(cleanup_error) }));
        }
    }
}

#[cfg(test)]
mod live;
#[cfg(test)]
mod tests;
