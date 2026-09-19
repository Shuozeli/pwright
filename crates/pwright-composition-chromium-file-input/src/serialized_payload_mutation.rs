//! Cancellation-safe ownership around one retained serialized-payload mutation.

use super::{OwnedValidatedMarkedFileInput, UploadProfile};
use pwright_backend_cdp_file_input::{
    CdpSerializedPayloadMutationService, PayloadTarget, SerializedPayloadMutationError,
};
use pwright_core::{
    ProductionResourceError, ProductionResourceLedger, ResourceClass, ResourceError, ResourceLease,
    ResourceLedger, ResourceRequest,
};
use pwright_runtime::{
    CleanupFailure, CleanupReservation, CleanupSubmitError, ReservedCleanupExecutor,
};
use serde_json::Value;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

type Outcome = Result<(), SerializedPayloadOperationError>;
type Operation = Pin<Box<dyn Future<Output = Outcome> + Send>>;
const STATE_BYTES: u64 = 512;
const CONTROL_ALLOWANCE: usize = 512;

#[derive(Debug, thiserror::Error)]
pub enum SerializedPayloadOperationError {
    #[error("invalid serialized-payload mutation continuation profile")]
    InvalidProfile,
    #[error("serialized-payload mutation continuation storage exceeds its allowance")]
    Storage,
    #[error("serialized-payload mutation continuation capacity exhausted")]
    Capacity,
    #[error("serialized-payload mutation cleanup is unproven; service admission is closed")]
    Unhealthy,
    #[error(transparent)]
    Production(#[from] ProductionResourceError),
    #[error(transparent)]
    Resource(#[from] ResourceError),
    #[error(transparent)]
    Admission(#[from] CleanupSubmitError),
    #[error(transparent)]
    Backend(#[from] SerializedPayloadMutationError),
    #[error("serialized-payload mutation group cleanup failed: {cleanup}; primary: {primary:?}")]
    Cleanup {
        primary: Option<Box<Self>>,
        cleanup: super::PrevalidationError,
    },
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
        self.lease = None;
        self.state.active.fetch_sub(1, Ordering::AcqRel);
    }
}

pub struct SerializedPayloadMutationOperationService<E> {
    profile: UploadProfile,
    ledger: ResourceLedger,
    state: Arc<State>,
    backend: Arc<CdpSerializedPayloadMutationService>,
    cleanup: E,
}

impl<E: ReservedCleanupExecutor> SerializedPayloadMutationOperationService<E> {
    pub fn new(
        profile: UploadProfile,
        resources: &ProductionResourceLedger,
        backend: Arc<CdpSerializedPayloadMutationService>,
        cleanup: E,
    ) -> Result<Self, SerializedPayloadOperationError> {
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
            backend,
            cleanup,
        })
    }

    pub fn is_cleanup_healthy(&self) -> bool {
        self.state.healthy.load(Ordering::Acquire)
    }

    pub fn active_operations(&self) -> usize {
        self.state.active.load(Ordering::Acquire)
    }

    /// Move the graph and exact validated owner into one operation. The
    /// retained value keeps the graph's external byte admission live through
    /// CDP settlement and terminal object-group release.
    pub fn prepare_retaining<
        R: CleanupReservation,
        M: CleanupReservation,
        C: CleanupReservation,
        Retained: Send + 'static,
    >(
        &self,
        element: OwnedValidatedMarkedFileInput<R, M, C>,
        graph: Value,
        retained: Retained,
    ) -> Result<SerializedPayloadMutationFuture<E::Reservation>, SerializedPayloadOperationError>
    {
        if !self.is_cleanup_healthy() {
            return Err(SerializedPayloadOperationError::Unhealthy);
        }
        self.state
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < self.profile.capacity).then(|| count + 1)
            })
            .map_err(|_| SerializedPayloadOperationError::Capacity)?;
        let mut owner = Owner {
            state: Arc::clone(&self.state),
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
        let backend = Arc::clone(&self.backend);
        let future = async move {
            let retained = retained;
            owner.arm();
            let primary = if owner.state.healthy.load(Ordering::Acquire) {
                let client = element.element().client();
                match PayloadTarget::new(
                    client.as_ref(),
                    element.element().utility_world(),
                    element.object_id(),
                    element.element().object_group(),
                ) {
                    Ok(target) => backend
                        .apply_on_target(target, graph)
                        .await
                        .map_err(SerializedPayloadOperationError::Backend),
                    Err(error) => Err(SerializedPayloadOperationError::Backend(
                        SerializedPayloadMutationError::Target(error),
                    )),
                }
            } else {
                Err(SerializedPayloadOperationError::Unhealthy)
            };
            let released = element.release().await;
            drop(retained);
            match released {
                Ok(()) => {
                    owner.disarm();
                    primary
                }
                Err(cleanup) => Err(SerializedPayloadOperationError::Cleanup {
                    primary: primary.err().map(Box::new),
                    cleanup,
                }),
            }
        };
        let required = std::mem::size_of_val(&future)
            .checked_add(std::mem::size_of::<
                SerializedPayloadMutationFuture<E::Reservation>,
            >())
            .and_then(|bytes| bytes.checked_add(CONTROL_ALLOWANCE))
            .ok_or(SerializedPayloadOperationError::Storage)?;
        if required > self.profile.owner_bytes {
            return Err(SerializedPayloadOperationError::Storage);
        }
        Ok(SerializedPayloadMutationFuture {
            inner: Some(Box::pin(future)),
            reservation: Some(reservation),
            started: false,
        })
    }
}

#[must_use = "await or drop to continue a started mutation and group cleanup"]
pub struct SerializedPayloadMutationFuture<R: CleanupReservation> {
    inner: Option<Operation>,
    reservation: Option<R>,
    started: bool,
}

impl<R: CleanupReservation> Unpin for SerializedPayloadMutationFuture<R> {}

impl<R: CleanupReservation> Future for SerializedPayloadMutationFuture<R> {
    type Output = Outcome;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let result = self
            .inner
            .as_mut()
            .expect("serialized-payload mutation polled after completion")
            .as_mut()
            .poll(cx);
        if result.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        result
    }
}

impl<R: CleanupReservation> Drop for SerializedPayloadMutationFuture<R> {
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        if let (Some(inner), Some(reservation)) = (self.inner.take(), self.reservation.take()) {
            reservation.submit(Box::pin(async move {
                match inner.await {
                    Err(SerializedPayloadOperationError::Cleanup { .. }) => {
                        Err(CleanupFailure::new(
                            "serialized-payload mutation group cleanup failed; admission closed",
                        ))
                    }
                    Ok(()) | Err(_) => Ok(()),
                }
            }));
        }
    }
}

#[cfg(test)]
mod tests;
