//! Driver-owned publication with pre-admitted compensation on every exit.
use super::{
    MAX_CALLS,
    creation::{Outcome, failure, map_leaf_error},
    pending::{Admission, Owners, Ticket},
    release::Releases,
};
use crate::HostError;
use pwright_cap_handle_evaluation::HandleInitializer;
use pwright_composition_chromium::ChromiumNativeWorldLifetime;
use pwright_composition_chromium_handle_evaluation::{
    ChromiumHandleEvaluationRouteIdentity,
    native_creation::{
        FailedNativeHandleCreation, NativeHandleCreationCommit, PendingNativeHandleCreation,
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

pub(super) struct PublicationContext {
    pub admission: Admission,
    pub reservation: TokioCleanupReservation,
    pub lifetime: Option<ChromiumNativeWorldLifetime>,
    pub releases: Arc<Releases>,
}
struct Entry {
    native: Option<PendingNativeHandleCreation>,
    context: Option<PublicationContext>,
    reply: Option<oneshot::Sender<Outcome>>,
}
struct Inbox {
    closed: bool,
    ready: [Option<Entry>; MAX_CALLS],
}
pub(super) struct Publications {
    inbox: Mutex<Inbox>,
    changed: Notify,
    owners: Arc<Owners>,
    _storage: ResourceLease,
}
const SCOPE: ResourceScope = ResourceScope::extension(159, 1);
fn owner_bytes() -> u64 {
    // A detached driver batch may coexist with the fixed inbox. Entries retain
    // their existing per-call allowance and per-result owner through cleanup.
    (std::mem::size_of::<Publications>() + std::mem::size_of::<Inbox>() + 256) as u64
}
pub(super) fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            owner_bytes(),
            owner_bytes(),
        )
        .with_metadata(1, 1)
}
impl Publications {
    pub fn new(
        resources: &ProductionResourceLedger,
        owners: Arc<Owners>,
    ) -> Result<Arc<Self>, HostError> {
        let storage = resources
            .claim(requirements())
            .map_err(HostError::backend)?
            .reserve(SCOPE, ResourceClass::BackendStateBytes, owner_bytes())
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
        native: PendingNativeHandleCreation,
        context: PublicationContext,
    ) -> Outcome {
        let (reply, received) = oneshot::channel();
        // No await separates native ownership from its recoverable envelope.
        self.park(
            Entry {
                native: Some(native),
                context: Some(context),
                reply: Some(reply),
            },
            true,
        )?;
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
                        "admitted Handle publication inbox overflow".into(),
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
                // Drop submits compensation, never an unowned native drop.
                continue;
            }
            let native = entry
                .native
                .take()
                .expect("queued creation owns native result");
            let source_world = native.world_identity();
            match native.commit_or_defer() {
                NativeHandleCreationCommit::Deferred(native) => {
                    entry.native = Some(native);
                    if let Err(error) = self.park(entry, false) {
                        failure = Some(HostError::backend(error));
                    }
                }
                NativeHandleCreationCommit::Published(initializer) => {
                    let context = entry.context.take().expect("publication owns context");
                    let result = self.retain(initializer, source_world, context);
                    let _ = entry
                        .reply
                        .take()
                        .expect("publication owns reply")
                        .send(result);
                    // A failed send drops PendingHandle, submitting its already
                    // registered ticket. No bare initializer crosses the channel.
                }
                NativeHandleCreationCommit::Rejected(rejected) => {
                    let context = entry.context.take().expect("rejection owns context");
                    rejected_cleanup(rejected, context, entry.reply.take());
                }
            }
        }
        failure.map_or(Ok(()), Err)
    }
    fn retain(
        &self,
        initializer: HandleInitializer,
        source_world: ChromiumHandleEvaluationRouteIdentity,
        context: PublicationContext,
    ) -> Outcome {
        let PublicationContext {
            admission,
            reservation,
            lifetime,
            releases,
        } = context;
        let Admission { owner, call, work } = admission;
        drop(call);
        let retained = owner.clone();
        let object = initializer.object_id;
        let ticket = Ticket {
            reservation,
            job: Box::pin(async move {
                let (_owner, _work) = (retained, work);
                releases.release(object, lifetime).await
            }),
        };
        self.owners
            .retain(initializer, ticket, owner)
            .map(|handle| handle.with_source_world(source_world))
            .map_err(failure)
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
        drop(ready); // Entry recovery submits outside the inbox lock.
        self.changed.notify_one();
    }
}
impl Drop for Publications {
    fn drop(&mut self) {
        self.close();
    }
}
impl Drop for Entry {
    fn drop(&mut self) {
        if let (Some(native), Some(context)) = (self.native.take(), self.context.take()) {
            let reply = self.reply.take();
            let PublicationContext {
                admission,
                reservation,
                lifetime,
                releases,
                ..
            } = context;
            reservation.submit(Box::pin(async move {
                let _admission = admission;
                let result = releases
                    .confirm_unpublished_release(native.release_native().await, lifetime)
                    .await;
                if let Some(reply) = reply {
                    let _ = reply
                        .send(Err(cleanup_error(&result)
                            .unwrap_or_else(|| failure(HostError::TargetClosed))));
                }
                result
            }));
        }
    }
}
fn cleanup_error(result: &Result<(), CleanupFailure>) -> Option<OperationError<HostError>> {
    result.as_ref().err().map(|error| {
        OperationError::new(
            FailureKind::Cleanup,
            "unpublished Handle compensation failed",
            Some(HostError::backend(error)),
        )
    })
}
fn rejected_cleanup(
    native: FailedNativeHandleCreation,
    context: PublicationContext,
    reply: Option<oneshot::Sender<Outcome>>,
) {
    let PublicationContext {
        admission,
        reservation,
        lifetime,
        releases,
        ..
    } = context;
    reservation.submit(Box::pin(async move {
        let _admission = admission;
        let (primary, native) = native.release_native().await;
        let result = releases.confirm_unpublished_release(native, lifetime).await;
        if let Some(reply) = reply {
            let error = match &result {
                Ok(()) => map_leaf_error(primary),
                Err(cleanup) => OperationError::new(
                    FailureKind::Cleanup,
                    "Handle publication and native compensation failed",
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
