//! Atomic native batch publication followed by per-object revocable ownership.
use super::{
    MAX_CALLS, NamedPendingHandle,
    creation::{failure, map_leaf_error},
    pending::{Admission, BatchAdmission, Owners, Ticket},
    release::Releases,
};
use crate::HostError;
use pwright_cap_handle_evaluation::HandlePropertiesResult;
use pwright_composition_chromium::ChromiumNativeWorldLifetime;
use pwright_composition_chromium_handle_evaluation::{
    ChromiumHandleEvaluationRouteIdentity,
    native_creation::{
        FailedNativeHandleBatchCreation, NativeHandleBatchCreationCommit,
        PendingNativeHandleBatchCreation,
    },
};
use pwright_core::{
    FailureKind, OperationError, ProductionResourceLedger, ResourceClass, ResourceLease,
    ResourceRequirements, ResourceScope,
};
use pwright_runtime::{CleanupFailure, CleanupReservation};
use pwright_runtime_tokio::TokioCleanupReservation;
use std::sync::{Arc, Mutex};
use tokio::sync::{Notify, oneshot};

pub(super) type Outcome = Result<Vec<NamedPendingHandle>, OperationError<HostError>>;
pub(super) struct BatchContext {
    pub admission: BatchAdmission,
    pub reservations: Vec<TokioCleanupReservation>,
    pub recovery: TokioCleanupReservation,
    pub lifetime: Option<ChromiumNativeWorldLifetime>,
    pub releases: Arc<Releases>,
}
struct Entry {
    native: Option<PendingNativeHandleBatchCreation>,
    context: Option<BatchContext>,
    reply: Option<oneshot::Sender<Outcome>>,
}
struct Inbox {
    closed: bool,
    ready: [Option<Entry>; MAX_CALLS],
}
pub(super) struct BatchPublications {
    inbox: Mutex<Inbox>,
    changed: Notify,
    owners: Arc<Owners>,
    _storage: ResourceLease,
}
const SCOPE: ResourceScope = ResourceScope::extension(166, 1);
fn storage_bytes() -> u64 {
    // Queue and detached driver batch can coexist; vectors/native results stay
    // inside the original shared call allowance and per-child owner admission.
    (std::mem::size_of::<BatchPublications>() + std::mem::size_of::<Inbox>() + 256) as u64
}
pub(super) fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            storage_bytes(),
            storage_bytes(),
        )
        .with_metadata(1, 1)
}
impl BatchPublications {
    pub fn new(
        resources: &ProductionResourceLedger,
        owners: Arc<Owners>,
    ) -> Result<Arc<Self>, HostError> {
        let storage = resources
            .claim(requirements())
            .map_err(HostError::backend)?
            .reserve(SCOPE, ResourceClass::BackendStateBytes, storage_bytes())
            .map_err(HostError::backend)?;
        Ok(Arc::new(Self {
            inbox: Mutex::new(Inbox {
                closed: false,
                ready: std::array::from_fn(|_| None),
            }),
            changed: Notify::new(),
            owners,
            _storage: storage,
        }))
    }
    pub async fn publish(
        &self,
        native: PendingNativeHandleBatchCreation,
        mut context: BatchContext,
    ) -> Outcome {
        let (reply, received) = oneshot::channel();
        // No local/runtime publication may precede admitted sibling ownership.
        let count = native.len();
        if count > context.admission.entries.len() || count > context.reservations.len() {
            drop(Entry {
                native: Some(native),
                context: Some(context),
                reply: Some(reply),
            });
        } else {
            context.admission.entries.truncate(count);
            context.reservations.truncate(count);
            self.park(
                Entry {
                    native: Some(native),
                    context: Some(context),
                    reply: Some(reply),
                },
                true,
            )?;
        }
        received
            .await
            .map_err(|error| failure(HostError::backend(error)))?
    }
    fn park(&self, entry: Entry, notify: bool) -> Result<(), OperationError<HostError>> {
        {
            let mut inbox = self
                .inbox
                .lock()
                .map_err(|error| failure(HostError::backend(error)))?;
            if inbox.closed {
                return Err(failure(HostError::TargetClosed));
            }
            let slot = inbox
                .ready
                .iter_mut()
                .find(|slot| slot.is_none())
                .ok_or_else(|| {
                    failure(HostError::ResourceExhausted(
                        "admitted Handle batch publication inbox overflow".into(),
                    ))
                })?;
            *slot = Some(entry);
        }
        if notify {
            self.changed.notify_one();
        }
        Ok(())
    }
    pub fn finish_ready(&self) -> Result<(), HostError> {
        let ready = {
            let mut inbox = self.inbox.lock().map_err(HostError::backend)?;
            std::mem::replace(&mut inbox.ready, std::array::from_fn(|_| None))
        };
        let mut failure = None;
        for mut entry in ready.into_iter().flatten() {
            if entry.reply.as_ref().is_none_or(oneshot::Sender::is_closed) {
                continue;
            }
            let native = entry
                .native
                .take()
                .expect("queued batch owns native results");
            let source_world = native.world_identity();
            match native.commit_or_defer() {
                NativeHandleBatchCreationCommit::Deferred(native) => {
                    entry.native = Some(native);
                    if let Err(error) = self.park(entry, false) {
                        failure = Some(HostError::backend(error));
                    }
                }
                NativeHandleBatchCreationCommit::Published(initializers) => {
                    let context = entry.context.take().expect("published batch owns context");
                    let result = self.retain(initializers, source_world, context);
                    let _ = entry
                        .reply
                        .take()
                        .expect("published batch owns reply")
                        .send(result);
                }
                NativeHandleBatchCreationCommit::Rejected(native) => {
                    rejected(
                        native,
                        entry.context.take().expect("rejected batch owns context"),
                        entry.reply.take(),
                    );
                }
            }
        }
        failure.map_or(Ok(()), Err)
    }
    fn retain(
        &self,
        initializers: HandlePropertiesResult,
        source_world: ChromiumHandleEvaluationRouteIdentity,
        context: BatchContext,
    ) -> Outcome {
        let BatchContext {
            admission,
            reservations,
            recovery: _recovery,
            lifetime,
            releases,
        } = context;
        let BatchAdmission {
            entries,
            call: _call,
            work: _work,
        } = admission;
        // The leaf's atomic commit returns exactly the admitted native count.
        // Always register/submit ALL siblings, even if owner close races retain.
        let mut retained = Vec::with_capacity(initializers.properties.len());
        let mut first_error = None;
        for ((property, admission), reservation) in initializers
            .properties
            .into_iter()
            .zip(entries)
            .zip(reservations)
        {
            let Admission { owner, call, work } = admission;
            drop(call);
            let kept = owner.clone();
            let object = property.handle.object_id;
            let releases = releases.clone();
            let lifetime = lifetime.clone();
            let ticket = Ticket {
                reservation,
                job: Box::pin(async move {
                    let (_owner, _work) = (kept, work);
                    releases.release(object, lifetime).await
                }),
            };
            match self.owners.retain(property.handle, ticket, owner) {
                Ok(handle) => retained.push(NamedPendingHandle {
                    name: property.name,
                    handle: handle.with_source_world(source_world.clone()),
                    _storage: _call.clone(),
                }),
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }
        match first_error {
            Some(error) => Err(failure(error)),
            None => Ok(retained),
        }
    }
    pub async fn changed(&self) {
        self.changed.notified().await;
    }
    pub fn close(&self) {
        let ready = {
            let mut inbox = self.inbox.lock().unwrap_or_else(|error| error.into_inner());
            inbox.closed = true;
            std::mem::replace(&mut inbox.ready, std::array::from_fn(|_| None))
        };
        drop(ready);
        self.changed.notify_one();
    }
}
impl Drop for BatchPublications {
    fn drop(&mut self) {
        self.close();
    }
}
impl Drop for Entry {
    fn drop(&mut self) {
        if let (Some(native), Some(context)) = (self.native.take(), self.context.take()) {
            let reply = self.reply.take();
            let BatchContext {
                admission,
                reservations,
                recovery,
                lifetime,
                releases,
            } = context;
            recovery.submit(Box::pin(async move {
                let (_admission, _reservations) = (admission, reservations);
                let result = releases
                    .confirm_unpublished_release(native.release_native().await, lifetime)
                    .await;
                if let Some(reply) = reply {
                    let error = match &result {
                        Ok(()) => failure(HostError::TargetClosed),
                        Err(error) => OperationError::new(
                            FailureKind::Cleanup,
                            "unpublished Handle batch compensation failed",
                            Some(HostError::backend(error)),
                        ),
                    };
                    let _ = reply.send(Err(error));
                }
                result
            }));
        }
    }
}
fn rejected(
    native: FailedNativeHandleBatchCreation,
    context: BatchContext,
    reply: Option<oneshot::Sender<Outcome>>,
) {
    let BatchContext {
        admission,
        reservations,
        recovery,
        lifetime,
        releases,
    } = context;
    recovery.submit(Box::pin(async move {
        let (_admission, _reservations) = (admission, reservations);
        let (primary, native) = native.release_native().await;
        let result: Result<(), CleanupFailure> =
            releases.confirm_unpublished_release(native, lifetime).await;
        if let Some(reply) = reply {
            let error = match &result {
                Ok(()) => map_leaf_error(primary),
                Err(cleanup) => OperationError::new(
                    FailureKind::Cleanup,
                    "Handle batch publication and compensation failed",
                    Some(HostError::backend(format!("{primary}; {cleanup}"))),
                ),
            };
            let _ = reply.send(Err(error));
        }
        result
    }));
}

#[cfg(test)]
mod tests;
