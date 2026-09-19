//! Single-use buffered uploads whose cleanup survives a dropped public waiter.
//! No selector retry, filesystem work, timer, executor spawn or Node policy.

#[cfg(feature = "retained-target-marking")]
mod retained_target_marking;
#[cfg(feature = "retained-target-marking")]
pub use pwright_backend_cdp_file_input::{
    CdpTargetMarkingService, MarkerWorld, TargetMarkingError, TargetMarkingProfile,
};
#[cfg(feature = "retained-target-marking")]
pub use retained_target_marking::{
    MarkerCreationFuture, MarkerFuture, MarkerOperationError, MarkerReleaseFuture,
    OwnedMarkedElement, OwnedTargetMarker, TargetMarkingOperationService,
};

#[cfg(feature = "retained-preflight")]
mod retained_preflight;
#[cfg(feature = "retained-preflight")]
pub use pwright_backend_cdp_file_input::{
    CdpFileInputPreflightService, FileInputPreflightFlags, RetainedPreflightError,
    RetainedPreflightProfile,
};
#[cfg(feature = "retained-preflight")]
pub use retained_preflight::{
    FileInputPreflightOperationService, OwnedValidatedFileInput, PrevalidationError,
    PrevalidationFuture, PrevalidationReleaseFuture,
};
#[cfg(feature = "retained-target-prevalidation")]
pub use retained_preflight::{MarkedPrevalidationFuture, OwnedValidatedMarkedFileInput};

#[cfg(feature = "retained-serialized-payload-mutation")]
mod serialized_payload_mutation;
#[cfg(feature = "retained-serialized-payload-mutation")]
pub use pwright_backend_cdp_file_input::{
    CdpSerializedPayloadMutationService, SerializedPayloadMutationError,
    SerializedPayloadMutationProfile,
};
#[cfg(feature = "retained-serialized-payload-mutation")]
pub use serialized_payload_mutation::{
    SerializedPayloadMutationFuture, SerializedPayloadMutationOperationService,
    SerializedPayloadOperationError,
};

use pwright_backend_cdp_file_input::{CdpFilePayloadService, PayloadError, PayloadOutcome};
use pwright_composition_chromium_locator_resolution::{OwnedElement, ResolutionError};
use pwright_core::{
    ProductionResourceError, ProductionResourceLedger, ResourceClass, ResourceError, ResourceLease,
    ResourceLedger, ResourceRequest, ResourceRequirements, ResourceScope,
};
use pwright_file_input_source::PreparedPayloads;
use pwright_runtime::{
    CleanupFailure, CleanupReservation, CleanupSubmitError, ReservedCleanupExecutor,
};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

type Outcome = Result<PayloadOutcome, UploadError>;
type Operation = Pin<Box<dyn Future<Output = Outcome> + Send>>;
const STATE_BYTES: u64 = 512;
const CONTROL_ALLOWANCE: usize = 512;

#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("invalid upload continuation profile")]
    InvalidProfile,
    #[error("upload continuation storage exceeds the selected allowance")]
    Storage,
    #[error("upload continuation capacity exhausted")]
    Capacity,
    #[error("upload cleanup is not proven; service cannot admit more work")]
    Unhealthy,
    #[error(transparent)]
    Production(#[from] ProductionResourceError),
    #[error(transparent)]
    Resource(#[from] ResourceError),
    #[error(transparent)]
    Admission(#[from] CleanupSubmitError),
    #[error(transparent)]
    Backend(#[from] PayloadError),
    #[error("upload group cleanup failed: {cleanup}; primary: {primary:?}")]
    Cleanup {
        primary: Option<Box<Self>>,
        cleanup: ResolutionError,
    },
}

/// Owner storage only. Source, backend, route and cleanup executor capacities
/// are planned independently. `owner_bytes` covers both future allocations and
/// embedding-specific cleanup-wrapper overhead; allocator internals are external.
#[derive(Clone, Copy, Debug)]
pub struct UploadProfile {
    scope: ResourceScope,
    capacity: usize,
    owner_bytes: usize,
    requirements: ResourceRequirements,
}
impl UploadProfile {
    pub fn new(
        scope: ResourceScope,
        capacity: usize,
        owner_bytes: usize,
    ) -> Result<Self, UploadError> {
        if capacity == 0 || owner_bytes == 0 {
            return Err(UploadError::InvalidProfile);
        }
        let count = u64::try_from(capacity).map_err(|_| UploadError::InvalidProfile)?;
        let bytes = u64::try_from(owner_bytes)
            .map_err(|_| UploadError::InvalidProfile)?
            .checked_mul(count)
            .and_then(|bytes| bytes.checked_add(STATE_BYTES))
            .ok_or(UploadError::InvalidProfile)?;
        let leases = capacity.checked_add(1).ok_or(UploadError::InvalidProfile)?;
        Ok(Self {
            scope,
            capacity,
            owner_bytes,
            requirements: ResourceRequirements::none()
                .with_class(ResourceClass::LiveObject, count, count)
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(1, leases),
        })
    }
    pub fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

struct State {
    active: AtomicUsize,
    healthy: AtomicBool,
    _lease: ResourceLease,
}
struct Owner {
    state: Arc<State>,
    lease: Option<ResourceLease>,
    armed: bool,
}
impl Owner {
    fn arm(&mut self) {
        self.armed = true;
    }
    fn disarm(&mut self) {
        self.armed = false;
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        if self.armed {
            self.state.healthy.store(false, Ordering::Release);
        }
        // Returning the admission slot must never precede its backing lease.
        self.lease = None;
        self.state.active.fetch_sub(1, Ordering::AcqRel);
    }
}

pub struct FilePayloadOperationService<E> {
    profile: UploadProfile,
    ledger: ResourceLedger,
    state: Arc<State>,
    backend: Arc<CdpFilePayloadService>,
    cleanup: E,
}
impl<E: ReservedCleanupExecutor> FilePayloadOperationService<E> {
    pub fn new(
        profile: UploadProfile,
        resources: &ProductionResourceLedger,
        backend: Arc<CdpFilePayloadService>,
        cleanup: E,
    ) -> Result<Self, UploadError> {
        let ledger = resources.claim(profile.requirements())?;
        let lease = ledger.reserve(profile.scope, ResourceClass::BackendStateBytes, STATE_BYTES)?;
        Ok(Self {
            profile,
            ledger,
            backend,
            cleanup,
            state: Arc::new(State {
                active: AtomicUsize::new(0),
                healthy: AtomicBool::new(true),
                _lease: lease,
            }),
        })
    }
    pub fn is_cleanup_healthy(&self) -> bool {
        self.state.healthy.load(Ordering::Acquire)
    }
    pub fn active_operations(&self) -> usize {
        self.state.active.load(Ordering::Acquire)
    }

    /// Consume exactly one resolved element and source. No mutation is sent
    /// until the returned future is polled. Rejection drops source ownership and
    /// lets OwnedElement submit its already-reserved group cleanup.
    pub fn prepare<R: CleanupReservation>(
        &self,
        element: OwnedElement<R>,
        payloads: PreparedPayloads,
    ) -> Result<UploadFuture<E::Reservation>, UploadError> {
        self.prepare_retaining(element, payloads, ())
    }

    /// Retain caller-owned source admission through native settlement and
    /// cleanup, including a dropped public waiter. Inline retained storage is
    /// included in owner_bytes; external allocations remain caller-budgeted.
    pub fn prepare_retaining<R: CleanupReservation, Retained: Send + 'static>(
        &self,
        element: OwnedElement<R>,
        payloads: PreparedPayloads,
        retained: Retained,
    ) -> Result<UploadFuture<E::Reservation>, UploadError> {
        if !self.is_cleanup_healthy() {
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
        let reservation = self.cleanup.try_reserve()?;
        let backend = self.backend.clone();
        let future = async move {
            let retained = retained;
            owner.arm();
            let primary = if owner.state.healthy.load(Ordering::Acquire) {
                backend
                    .apply(element.element(), &payloads)
                    .await
                    .map_err(UploadError::Backend)
            } else {
                Err(UploadError::Unhealthy)
            };
            // Keep both raw payload backing and the route/group owner through
            // acknowledgement (or authoritative destruction proof) of cleanup.
            let released = element.release().await;
            drop(payloads);
            drop(retained);
            match released {
                Ok(()) => {
                    owner.disarm();
                    primary
                }
                Err(cleanup) => Err(UploadError::Cleanup {
                    primary: primary.err().map(Box::new),
                    cleanup,
                }),
            }
        };
        let required = std::mem::size_of_val(&future)
            .checked_add(std::mem::size_of::<UploadFuture<E::Reservation>>())
            .and_then(|bytes| bytes.checked_add(CONTROL_ALLOWANCE))
            .ok_or(UploadError::Storage)?;
        if required > self.profile.owner_bytes {
            return Err(UploadError::Storage);
        }
        Ok(UploadFuture {
            inner: Some(Box::pin(future)),
            reservation: Some(reservation),
            started: false,
        })
    }
}

/// A public timeout/cancellation may drop this waiter. After first poll its
/// entire operation moves to the pre-admitted cleanup executor, which MUST be
/// driven through completion before runtime shutdown. Dropping an unpolled
/// future releases the input without ever submitting a mutation.
#[must_use = "await or drop to transfer started work to the cleanup executor"]
pub struct UploadFuture<R: CleanupReservation> {
    inner: Option<Operation>,
    reservation: Option<R>,
    started: bool,
}
impl<R: CleanupReservation> Unpin for UploadFuture<R> {}
impl<R: CleanupReservation> Future for UploadFuture<R> {
    type Output = Outcome;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let result = self
            .inner
            .as_mut()
            .expect("upload polled after completion")
            .as_mut()
            .poll(cx);
        if result.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        result
    }
}
impl<R: CleanupReservation> Drop for UploadFuture<R> {
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        if let (Some(inner), Some(reservation)) = (self.inner.take(), self.reservation.take()) {
            reservation.submit(Box::pin(async move {
                match inner.await {
                    Err(UploadError::Cleanup { .. }) => {
                        // Do not duplicate browser-controlled error payloads
                        // into an executor-owned diagnostic allocation.
                        Err(CleanupFailure::new(
                            "file-input group cleanup failed; service admission closed",
                        ))
                    }
                    // The user result has no receiver, but its group was released.
                    Ok(_) | Err(_) => Ok(()),
                }
            }));
        }
    }
}

#[cfg(test)]
mod tests;
