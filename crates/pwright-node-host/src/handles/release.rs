//! Bounded native-release completion inbox for the existing topology driver.
//! The existing cleanup executor polls native I/O; only the driver commits.
use crate::HostError;
use pwright_cap_handle_lifecycle::HandleLifecycleErrorDetails;
use pwright_composition_chromium::ChromiumNativeWorldLifetime;
use pwright_composition_chromium_handle_lifecycle::{
    ChromiumHandleDisposeCommandService,
    native_release::{NativeHandleReleaseCommit, NativeHandleReleaseReceipt},
};
use pwright_core::{FailureKind, OperationError};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use pwright_runtime::{
    CleanupFailure, ControlledLifecycleHandle,
    disposal_reservations::{DisposalReservationProfile, DisposalReservations},
};
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};
use tokio::sync::{Notify, Semaphore, oneshot};

const PENDING: usize = 8;
const OWNER_SCOPE: ResourceScope = ResourceScope::extension(157, 1);
const RESERVATION_SCOPE: ResourceScope = ResourceScope::extension(157, 2);

struct Completion {
    outcome: ReleaseOutcome,
    reply: oneshot::Sender<Result<(), CleanupFailure>>,
}
enum ReleaseOutcome {
    Released(NativeHandleReleaseReceipt),
    ParentClosing {
        object: Option<ObjectId>,
        lifetime: ChromiumNativeWorldLifetime,
        error: OperationError<HandleLifecycleErrorDetails>,
    },
}
struct Inbox {
    closed: bool,
    ready: [Option<Completion>; PENDING],
}
pub(super) struct Releases {
    service: ChromiumHandleDisposeCommandService,
    runtime: ControlledLifecycleHandle,
    capacity: DisposalReservations,
    slots: Semaphore,
    inbox: Mutex<Inbox>,
    changed: Notify,
    _storage: ResourceLease,
}
fn profile() -> Result<DisposalReservationProfile, HostError> {
    DisposalReservationProfile::new_in_process(
        NonZeroUsize::new(256).unwrap(),
        NonZeroUsize::new(PENDING).unwrap(),
        RESERVATION_SCOPE,
    )
    .and_then(|profile| profile.with_deferred_commit(NonZeroUsize::new(16).unwrap()))
    .map_err(HostError::backend)
}
fn owner_bytes() -> u64 {
    // The inbox and a detached commit batch can coexist. Future/channel waiter
    // storage belongs to the already admitted pending-result owner (8 KiB).
    (std::mem::size_of::<Releases>() + std::mem::size_of::<Inbox>() + 256) as u64
}
fn owner_requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            owner_bytes(),
            owner_bytes(),
        )
        .with_metadata(1, 1)
}
pub(super) fn requirements() -> Result<ResourceRequirements, HostError> {
    Ok(owner_requirements().combine_concurrent(profile()?.requirements()))
}
fn failed(error: impl std::fmt::Display) -> CleanupFailure {
    CleanupFailure::new(format!("pending Handle release failed: {error}"))
}
impl Releases {
    pub fn new(
        service: ChromiumHandleDisposeCommandService,
        runtime: &ControlledLifecycleHandle,
        resources: &ProductionResourceLedger,
    ) -> Result<Arc<Self>, HostError> {
        let storage = resources
            .claim(owner_requirements())
            .map_err(HostError::backend)?
            .reserve(OWNER_SCOPE, ResourceClass::BackendStateBytes, owner_bytes())
            .map_err(HostError::backend)?;
        Ok(Arc::new(Self {
            service,
            runtime: runtime.clone(),
            capacity: profile()?
                .bind(runtime, resources)
                .map_err(HostError::backend)?,
            slots: Semaphore::new(PENDING),
            inbox: Mutex::new(Inbox {
                closed: false,
                ready: std::array::from_fn(|_| None),
            }),
            changed: Notify::new(),
            _storage: storage,
        }))
    }
    pub async fn release(
        &self,
        object: ObjectId,
        lifetime: Option<ChromiumNativeWorldLifetime>,
    ) -> Result<(), CleanupFailure> {
        let _slot = self.slots.acquire().await.map_err(failed)?;
        let capacity = self.capacity.reserve().map_err(failed)?;
        let Some(request) = self
            .service
            .prepare_native_release(object, capacity)
            .map_err(failed)?
        else {
            return Ok(());
        };
        let outcome = match request.release().await {
            Ok(receipt) => ReleaseOutcome::Released(receipt),
            Err(error) => match lifetime {
                Some(lifetime) if error.kind() == FailureKind::TargetClosed => {
                    ReleaseOutcome::ParentClosing {
                        object: Some(object),
                        lifetime,
                        error,
                    }
                }
                _ => return Err(failed(error)),
            },
        };
        let (reply, completed) = oneshot::channel();
        self.park(Completion { outcome, reply }, true)
            .map_err(failed)?;
        completed.await.map_err(failed)?
    }
    /// Unpublished results have no portable object. Only exact native world
    /// retirement may justify a failed native release here.
    pub async fn confirm_unpublished_release(
        &self,
        outcome: Result<(), OperationError<HandleLifecycleErrorDetails>>,
        lifetime: Option<ChromiumNativeWorldLifetime>,
    ) -> Result<(), CleanupFailure> {
        let Err(error) = outcome else { return Ok(()) };
        let Some(lifetime) = lifetime.filter(|_| error.kind() == FailureKind::TargetClosed) else {
            return Err(failed(error));
        };
        let _slot = self.slots.acquire().await.map_err(failed)?;
        let (reply, completed) = oneshot::channel();
        self.park(
            Completion {
                outcome: ReleaseOutcome::ParentClosing {
                    object: None,
                    lifetime,
                    error,
                },
                reply,
            },
            true,
        )
        .map_err(failed)?;
        completed.await.map_err(failed)?
    }
    fn park(&self, completion: Completion, notify: bool) -> Result<(), HostError> {
        {
            let mut inbox = self.inbox.lock().map_err(HostError::backend)?;
            if inbox.closed {
                return Err(HostError::TargetClosed);
            }
            let slot = inbox
                .ready
                .iter_mut()
                .find(|slot| slot.is_none())
                .ok_or_else(|| {
                    HostError::ResourceExhausted("admitted Handle release inbox overflow".into())
                })?;
            *slot = Some(completion);
        }
        if notify {
            self.changed.notify_one();
        }
        Ok(())
    }
    /// Called only between topology/explicit lifecycle transactions, never
    /// from native completion polling. Notifications execute outside all locks.
    pub fn finish_ready(&self) -> Result<(), HostError> {
        let ready = {
            let mut inbox = self.inbox.lock().map_err(HostError::backend)?;
            std::mem::replace(&mut inbox.ready, std::array::from_fn(|_| None))
        };
        for completion in ready.into_iter().flatten() {
            let Completion { outcome, reply } = completion;
            if reply.is_closed() {
                continue;
            }
            let receipt = match outcome {
                ReleaseOutcome::Released(receipt) => receipt,
                ReleaseOutcome::ParentClosing {
                    object,
                    lifetime,
                    error,
                } => {
                    if lifetime.native_objects_retired()
                        && !object
                            .map(|object| self.runtime.is_object_live(object))
                            .transpose()
                            .map_err(HostError::backend)?
                            .unwrap_or(false)
                    {
                        #[cfg(test)]
                        eprintln!(
                            "Handle {object:?}: native world retirement and local retirement confirmed after release failure"
                        );
                        let _ = reply.send(Ok(()));
                    } else {
                        // A detached session is not proof. Keep bounded native
                        // lifetime evidence until actual destruction is observed
                        // by the driver; the existing cleanup deadline still
                        // reports failure if that evidence never arrives.
                        self.park(
                            Completion {
                                outcome: ReleaseOutcome::ParentClosing {
                                    object,
                                    lifetime,
                                    error,
                                },
                                reply,
                            },
                            false,
                        )?;
                    }
                    continue;
                }
            };
            match receipt.commit_or_defer() {
                Ok(NativeHandleReleaseCommit::Committed(mut committed)) => {
                    committed.notify();
                    // Admission must return before waking its native worker.
                    drop(committed);
                    let _ = reply.send(Ok(()));
                }
                Ok(NativeHandleReleaseCommit::Superseded) => {
                    let _ = reply.send(Ok(()));
                }
                Ok(NativeHandleReleaseCommit::Deferred(receipt)) => {
                    // No busy retry loop: a later native/creation poll or driver
                    // transaction boundary is the next opportunity to commit.
                    self.park(
                        Completion {
                            outcome: ReleaseOutcome::Released(receipt),
                            reply,
                        },
                        false,
                    )?;
                }
                Err(error) => {
                    let _ = reply.send(Err(failed(&error)));
                    return Err(HostError::backend(error));
                }
            }
        }
        Ok(())
    }
    pub async fn changed(&self) {
        self.changed.notified().await;
    }
    /// Handle creation publishes synchronously within a native future poll.
    /// Signal after that poll releases its short transaction, including the
    /// abandoned-creation continuation path, so a deferred receipt cannot stall.
    pub fn creation_polled(&self) {
        self.changed.notify_one();
    }
    pub fn close(&self) {
        self.slots.close();
        let ready = {
            let mut inbox = self.inbox.lock().unwrap_or_else(|error| error.into_inner());
            inbox.closed = true;
            std::mem::replace(&mut inbox.ready, std::array::from_fn(|_| None))
        };
        for completion in ready.into_iter().flatten() {
            let error = match completion.outcome {
                ReleaseOutcome::Released(_) => failed("session release coordinator closed"),
                ReleaseOutcome::ParentClosing { error, .. } => failed(error),
            };
            let _ = completion.reply.send(Err(error));
        }
        self.changed.notify_one();
    }
}
impl Drop for Releases {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests;
