//! A retained DOM prevalidation owner, not an upload or language conversion.
//! No Node root, executor spawn, marking, payload access or mutation is owned.

use super::*;
use pwright_backend_cdp_file_input::{
    CdpFileInputPreflightService, FileInputPreflightFlags, PayloadTarget, PreflightAdmission,
    RetainedPreflightError,
};
use pwright_composition_chromium_locator_resolution::ResolvedElement;

#[derive(Debug, thiserror::Error)]
pub enum PrevalidationError {
    #[error(transparent)]
    Owner(#[from] UploadError),
    #[error(transparent)]
    Backend(#[from] RetainedPreflightError),
    #[error("retained prevalidation requires settlement-owned group cleanup")]
    OrdinaryGroup,
    #[error("retained prevalidation group cleanup failed: {cleanup}; primary: {primary:?}")]
    Cleanup {
        primary: Option<Box<Self>>,
        cleanup: ResolutionError,
    },
    #[cfg(feature = "retained-target-prevalidation")]
    #[error("retained marked-target prevalidation cleanup failed: {cleanup}; primary: {primary:?}")]
    MarkedCleanup {
        primary: Option<Box<Self>>,
        cleanup: MarkerOperationError,
    },
}

impl PrevalidationError {
    fn is_cleanup(&self) -> bool {
        match self {
            Self::Cleanup { .. } => true,
            #[cfg(feature = "retained-target-prevalidation")]
            Self::MarkedCleanup { .. } => true,
            _ => false,
        }
    }

    fn with_cleanup_primary(self, primary: Self) -> Self {
        match self {
            Self::Cleanup { cleanup, .. } => Self::Cleanup {
                primary: Some(Box::new(primary)),
                cleanup,
            },
            #[cfg(feature = "retained-target-prevalidation")]
            Self::MarkedCleanup { cleanup, .. } => Self::MarkedCleanup {
                primary: Some(Box::new(primary)),
                cleanup,
            },
            error => error,
        }
    }
}

trait PreflightTarget: Send + 'static {
    type Cleanup: Send + 'static;
    type Release: Future<Output = Result<(), Self::Cleanup>> + Send + 'static;

    fn element(&self) -> &ResolvedElement;
    fn release(self) -> Self::Release;
    fn cleanup_error(
        primary: Option<Box<PrevalidationError>>,
        cleanup: Self::Cleanup,
    ) -> PrevalidationError;
}

impl<R: CleanupReservation> PreflightTarget for OwnedElement<R> {
    type Cleanup = ResolutionError;
    type Release = pwright_composition_chromium_locator_resolution::ReleaseFuture<R>;

    fn element(&self) -> &ResolvedElement {
        self.element()
    }

    fn release(self) -> Self::Release {
        self.release()
    }

    fn cleanup_error(
        primary: Option<Box<PrevalidationError>>,
        cleanup: Self::Cleanup,
    ) -> PrevalidationError {
        PrevalidationError::Cleanup { primary, cleanup }
    }
}

#[cfg(feature = "retained-target-prevalidation")]
impl<R: CleanupReservation, M: CleanupReservation> PreflightTarget for OwnedMarkedElement<R, M> {
    type Cleanup = MarkerOperationError;
    type Release = MarkerReleaseFuture<M>;

    fn element(&self) -> &ResolvedElement {
        self.element()
    }

    fn release(self) -> Self::Release {
        self.release()
    }

    fn cleanup_error(
        primary: Option<Box<PrevalidationError>>,
        cleanup: Self::Cleanup,
    ) -> PrevalidationError {
        PrevalidationError::MarkedCleanup { primary, cleanup }
    }
}

struct Ready<T: PreflightTarget> {
    // Abandonment must publish unhealthy before either backing allowance is
    // returned. It cannot prove native reclamation if the executor discards
    // its own jobs; the original owner can only queue its remaining cleanup.
    owner: Owner,
    element: T,
    admission: PreflightAdmission,
}
type Validation<T> = Pin<Box<dyn Future<Output = Result<Ready<T>, PrevalidationError>> + Send>>;
type Release = Pin<Box<dyn Future<Output = Result<(), PrevalidationError>> + Send>>;

/// Uses a separate caller-selected UploadProfile and backend profile; existing
/// upload admission is unchanged. Capacity remains occupied while a validated
/// owner is parked and throughout release, not merely while its query runs.
pub struct FileInputPreflightOperationService<E> {
    profile: UploadProfile,
    ledger: ResourceLedger,
    state: Arc<State>,
    backend: Arc<CdpFileInputPreflightService>,
    cleanup: E,
}
impl<E: ReservedCleanupExecutor> FileInputPreflightOperationService<E> {
    pub fn new(
        profile: UploadProfile,
        resources: &ProductionResourceLedger,
        backend: Arc<CdpFileInputPreflightService>,
        cleanup: E,
    ) -> Result<Self, PrevalidationError> {
        let ledger = resources
            .claim(profile.requirements())
            .map_err(UploadError::Production)?;
        let lease = ledger
            .reserve(profile.scope, ResourceClass::BackendStateBytes, STATE_BYTES)
            .map_err(UploadError::Resource)?;
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

    pub fn prepare<R: CleanupReservation>(
        &self,
        element: OwnedElement<R>,
        flags: FileInputPreflightFlags,
    ) -> Result<PrevalidationFuture<R, E::Reservation>, PrevalidationError> {
        self.prepare_target(element, flags)
            .map(|inner| PrevalidationFuture { inner })
    }

    /// Continue prevalidation with the exact marked owner. Its marker-call
    /// admission remains nested inside the validated owner until the original
    /// target group reaches terminal cleanup; no release/re-adopt gap exists.
    #[cfg(feature = "retained-target-prevalidation")]
    pub fn prepare_marked<R: CleanupReservation, M: CleanupReservation>(
        &self,
        element: OwnedMarkedElement<R, M>,
        flags: FileInputPreflightFlags,
    ) -> Result<MarkedPrevalidationFuture<R, M, E::Reservation>, PrevalidationError> {
        self.prepare_target(element, flags)
            .map(|inner| MarkedPrevalidationFuture { inner })
    }

    fn prepare_target<T: PreflightTarget>(
        &self,
        element: T,
        flags: FileInputPreflightFlags,
    ) -> Result<ValidationFuture<T, E::Reservation>, PrevalidationError> {
        // A timed ordinary group owner cannot prove settlement of this new
        // retained acquisition. Reject before creating any derived wrapper.
        if !element.element().uses_settlement_release() {
            return Err(PrevalidationError::OrdinaryGroup);
        }
        if !self.is_cleanup_healthy() {
            return Err(UploadError::Unhealthy.into());
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
        owner.lease = Some(
            self.ledger
                .reserve_many(
                    self.profile.scope,
                    &[
                        ResourceRequest::new(ResourceClass::LiveObject, 1),
                        ResourceRequest::new(
                            ResourceClass::BackendStateBytes,
                            self.profile.owner_bytes as u64,
                        ),
                    ],
                )
                .map_err(UploadError::Resource)?,
        );
        let admission = self.backend.admit()?;
        // Independent of OwnedElement's cleanup slot: this continuation must
        // retain the extra admission around element.release().await as well.
        let reservation = self.cleanup.try_reserve().map_err(UploadError::Admission)?;
        let future = async move {
            let mut ready = Ready {
                element,
                admission,
                owner,
            };
            // Once native work can start, dropping the executor's own future
            // without confirmed cleanup poisons this service's admission.
            ready.owner.arm();
            let primary = if ready.owner.state.healthy.load(Ordering::Acquire) {
                let element = ready.element.element();
                let client = element.client();
                match PayloadTarget::new(
                    client.as_ref(),
                    element.utility_world(),
                    element.object_id(),
                    element.object_group(),
                ) {
                    Ok(target) => ready
                        .admission
                        .validate(target, flags)
                        .await
                        .map_err(PrevalidationError::Backend),
                    Err(error) => Err(UploadError::Backend(error).into()),
                }
            } else {
                Err(UploadError::Unhealthy.into())
            };
            match primary {
                Ok(()) => Ok(ready),
                Err(primary) => match release_ready(ready).await {
                    Ok(()) => Err(primary),
                    Err(cleanup) if cleanup.is_cleanup() => {
                        Err(cleanup.with_cleanup_primary(primary))
                    }
                    Err(error) => Err(error),
                },
            }
        };
        let required = std::mem::size_of_val(&future)
            .checked_add(future_storage(release_ready::<T>))
            .and_then(|bytes| {
                bytes.checked_add(std::mem::size_of::<ValidationFuture<T, E::Reservation>>())
            })
            .and_then(|bytes| {
                bytes.checked_add(std::mem::size_of::<Validated<T, E::Reservation>>())
            })
            .and_then(|bytes| {
                bytes.checked_add(std::mem::size_of::<
                    PrevalidationReleaseFuture<E::Reservation>,
                >())
            })
            .and_then(|bytes| bytes.checked_add(2 * CONTROL_ALLOWANCE))
            .ok_or(UploadError::Storage)?;
        if required > self.profile.owner_bytes {
            return Err(UploadError::Storage.into());
        }
        Ok(ValidationFuture {
            inner: Some(Box::pin(future)),
            reservation: Some(reservation),
            started: false,
        })
    }
}

fn future_storage<Input, F: Future>(_: impl FnOnce(Input) -> F) -> usize {
    std::mem::size_of::<F>()
}

async fn release_ready<T: PreflightTarget>(ready: Ready<T>) -> Result<(), PrevalidationError> {
    let Ready {
        element,
        admission,
        owner,
    } = ready;
    // Field drop order is also enforced while this future is suspended: an
    // abandoned cleanup marks the service unhealthy BEFORE freeing the extra
    // native allowance, rather than waiting for a later sibling-local Drop.
    struct Backing {
        owner: Owner,
        _admission: PreflightAdmission,
    }
    let mut backing = Backing {
        owner,
        _admission: admission,
    };
    // Keep the exact returned ID, both derived native units and route owner
    // through the entire original group's terminal ACK/destruction proof.
    let released = element.release().await;
    match released {
        Ok(()) => {
            backing.owner.disarm();
            Ok(())
        }
        Err(cleanup) => {
            backing.owner.state.healthy.store(false, Ordering::Release);
            Err(T::cleanup_error(None, cleanup))
        }
    }
}

fn abandoned_error(result: Result<(), PrevalidationError>) -> Result<(), CleanupFailure> {
    result.map_err(|_| {
        CleanupFailure::new("retained file-input prevalidation cleanup failed; admission closed")
    })
}

struct ValidationFuture<T: PreflightTarget, C: CleanupReservation> {
    inner: Option<Validation<T>>,
    reservation: Option<C>,
    started: bool,
}
impl<T: PreflightTarget, C: CleanupReservation> Unpin for ValidationFuture<T, C> {}
impl<T: PreflightTarget, C: CleanupReservation> Future for ValidationFuture<T, C> {
    type Output = Result<Validated<T, C>, PrevalidationError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        match self
            .inner
            .as_mut()
            .expect("prevalidation polled after completion")
            .as_mut()
            .poll(cx)
        {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                self.inner = None;
                let reservation = self.reservation.take();
                Poll::Ready(result.map(|ready| Validated {
                    ready: Some(ready),
                    reservation,
                }))
            }
        }
    }
}
impl<T: PreflightTarget, C: CleanupReservation> Drop for ValidationFuture<T, C> {
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        if let (Some(inner), Some(reservation)) = (self.inner.take(), self.reservation.take()) {
            reservation.submit(Box::pin(async move {
                match inner.await {
                    Ok(ready) => abandoned_error(release_ready(ready).await),
                    Err(error) if error.is_cleanup() => Err(CleanupFailure::new(
                        "retained file-input prevalidation cleanup failed; admission closed",
                    )),
                    // A failed prevalidation already released its entire group.
                    Err(_) => Ok(()),
                }
            }));
        }
    }
}

struct Validated<T: PreflightTarget, C: CleanupReservation> {
    ready: Option<Ready<T>>,
    reservation: Option<C>,
}
impl<T: PreflightTarget, C: CleanupReservation> Validated<T, C> {
    fn object_id(&self) -> &str {
        self.ready
            .as_ref()
            .expect("live validated input owner")
            .admission
            .object_id()
            .expect("successful prevalidation retained its input")
    }

    fn element(&self) -> &ResolvedElement {
        self.ready
            .as_ref()
            .expect("live validated input owner")
            .element
            .element()
    }

    fn release(&mut self) -> PrevalidationReleaseFuture<C> {
        let ready = self.ready.take().expect("live validated input owner");
        PrevalidationReleaseFuture {
            inner: Some(Box::pin(release_ready(ready))),
            reservation: self.reservation.take(),
        }
    }
}
impl<T: PreflightTarget, C: CleanupReservation> Drop for Validated<T, C> {
    fn drop(&mut self) {
        if let (Some(ready), Some(reservation)) = (self.ready.take(), self.reservation.take()) {
            reservation.submit(Box::pin(async move {
                abandoned_error(release_ready(ready).await)
            }));
        }
    }
}

/// A successful query transfers, rather than releases, its admitted owner.
/// Dropping a started query continues only acquisition and cleanup; it can
/// never invoke a payload serializer or proceed to mutation.
#[must_use = "await or drop to transfer started prevalidation to cleanup"]
pub struct PrevalidationFuture<R: CleanupReservation, C: CleanupReservation> {
    inner: ValidationFuture<OwnedElement<R>, C>,
}
impl<R: CleanupReservation, C: CleanupReservation> Unpin for PrevalidationFuture<R, C> {}
impl<R: CleanupReservation, C: CleanupReservation> Future for PrevalidationFuture<R, C> {
    type Output = Result<OwnedValidatedFileInput<R, C>, PrevalidationError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner)
            .poll(cx)
            .map(|result| result.map(|inner| OwnedValidatedFileInput { inner }))
    }
}

#[cfg(feature = "retained-target-prevalidation")]
#[must_use = "await or drop to transfer marked prevalidation to cleanup"]
pub struct MarkedPrevalidationFuture<
    R: CleanupReservation,
    M: CleanupReservation,
    C: CleanupReservation,
> {
    inner: ValidationFuture<OwnedMarkedElement<R, M>, C>,
}
#[cfg(feature = "retained-target-prevalidation")]
impl<R: CleanupReservation, M: CleanupReservation, C: CleanupReservation> Unpin
    for MarkedPrevalidationFuture<R, M, C>
{
}
#[cfg(feature = "retained-target-prevalidation")]
impl<R: CleanupReservation, M: CleanupReservation, C: CleanupReservation> Future
    for MarkedPrevalidationFuture<R, M, C>
{
    type Output = Result<OwnedValidatedMarkedFileInput<R, M, C>, PrevalidationError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner)
            .poll(cx)
            .map(|result| result.map(|inner| OwnedValidatedMarkedFileInput { inner }))
    }
}

/// Unique group-owned validated INPUT; it is not a public Handle, immutable DOM
/// snapshot, native-use pin against navigation or Node preparation authority.
/// `object_id` is the retargeted node; `element()` retains the original adopted
/// receiver and group. A later selected operation must not retarget again.
#[must_use = "retain, explicitly release, or drop the validated input owner"]
pub struct OwnedValidatedFileInput<R: CleanupReservation, C: CleanupReservation> {
    inner: Validated<OwnedElement<R>, C>,
}
impl<R: CleanupReservation, C: CleanupReservation> OwnedValidatedFileInput<R, C> {
    pub fn object_id(&self) -> &str {
        self.inner.object_id()
    }

    pub fn element(&self) -> &ResolvedElement {
        self.inner.element()
    }

    pub fn release(mut self) -> PrevalidationReleaseFuture<C> {
        self.inner.release()
    }
}

/// Validated receiver which still owns the exact prior marking call. The
/// retargeted input ID and marked original target share one terminal group
/// cleanup; later phases must use this ID without another retarget operation.
#[cfg(feature = "retained-target-prevalidation")]
#[must_use = "retain, explicitly release, or drop the validated marked input owner"]
pub struct OwnedValidatedMarkedFileInput<
    R: CleanupReservation,
    M: CleanupReservation,
    C: CleanupReservation,
> {
    inner: Validated<OwnedMarkedElement<R, M>, C>,
}
#[cfg(feature = "retained-target-prevalidation")]
impl<R: CleanupReservation, M: CleanupReservation, C: CleanupReservation>
    OwnedValidatedMarkedFileInput<R, M, C>
{
    pub fn object_id(&self) -> &str {
        self.inner.object_id()
    }

    pub fn element(&self) -> &ResolvedElement {
        self.inner.element()
    }

    pub fn release(mut self) -> PrevalidationReleaseFuture<C> {
        self.inner.release()
    }
}

/// Explicit release retains the same reserved continuation even when dropped
/// before first poll or while the group release is awaiting settlement.
#[must_use = "await or drop to continue admitted group cleanup"]
pub struct PrevalidationReleaseFuture<C: CleanupReservation> {
    inner: Option<Release>,
    reservation: Option<C>,
}
impl<C: CleanupReservation> Unpin for PrevalidationReleaseFuture<C> {}
impl<C: CleanupReservation> Future for PrevalidationReleaseFuture<C> {
    type Output = Result<(), PrevalidationError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result = self
            .inner
            .as_mut()
            .expect("release polled after completion")
            .as_mut()
            .poll(cx);
        if result.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        result
    }
}
impl<C: CleanupReservation> Drop for PrevalidationReleaseFuture<C> {
    fn drop(&mut self) {
        if let (Some(inner), Some(reservation)) = (self.inner.take(), self.reservation.take()) {
            reservation.submit(Box::pin(async move { abandoned_error(inner.await) }));
        }
    }
}

#[cfg(all(test, feature = "retained-target-prevalidation"))]
mod marked_tests;
#[cfg(test)]
mod tests;
