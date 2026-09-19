//! Native close work belongs to the one topology task, not caller futures.
//! Polling never commits local state: completions park until the driver owns a
//! transaction boundary. Stop drains written commands before releasing owners.
mod profile;
use crate::lifecycle::{DriverReply, DriverRequest, ordinary_close};
use crate::{CommandRuntime, HostError, LiveSession, frame_trees::FrameTrees};
use profile::MAX_CLOSES;
pub(crate) use profile::{MAX_CALLS, MAX_OBJECTS, Profile};
use pwright_backend_cdp_topology::{
    CdpContextOwnership, transition_reservations::TransitionReservations,
};
use pwright_composition_chromium::ChromiumCloseCapacityBundle;
use pwright_composition_chromium::native_close_authority::scheduler::{
    NativeCloseCompletion, NativeCloseScheduler,
};
use pwright_composition_chromium::{
    AttachedChromiumComposition, ChromiumCloseObserver, ChromiumCloseTarget,
};
use pwright_core::{ObjectId, ResourceLease};
use pwright_runtime::disposal_reservations::DisposalReservations;
use std::{
    future::{Future, poll_fn},
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tokio::sync::{Notify, oneshot};
use tokio_util::sync::CancellationToken;

struct Reply {
    target: ChromiumCloseTarget,
    sender: oneshot::Sender<Result<(), HostError>>,
}
impl Reply {
    fn finish(self, result: Result<(), HostError>) -> DriverReply {
        match self.target {
            ChromiumCloseTarget::Page(_) => {
                DriverReply::ClosePage(self.sender, ordinary_close(result))
            }
            ChromiumCloseTarget::BrowserContext(_) => {
                DriverReply::CloseContext(self.sender, result)
            }
        }
    }
}
struct PendingClose {
    capacity: ChromiumCloseCapacityBundle,
    scratch: [ObjectId; MAX_OBJECTS],
    reply: Reply,
}
struct Selected {
    native: NativeCloseScheduler<PendingClose>,
    topology: TransitionReservations,
    runtime: DisposalReservations,
    observer: Arc<dyn ChromiumCloseObserver>,
    ready: [Option<NativeCloseCompletion<PendingClose>>; MAX_CLOSES],
    head: usize,
    len: usize,
    wakeup: Arc<Notify>,
    _storage: ResourceLease,
}
pub(crate) struct Jobs(Option<Box<Selected>>);
pub(crate) enum Admission {
    Queued,
    Reply(DriverReply),
    Other(DriverRequest),
}
impl Jobs {
    pub(crate) fn new(
        composition: &AttachedChromiumComposition,
        runtime: &CommandRuntime,
        trees: &Arc<FrameTrees>,
        stop: CancellationToken,
    ) -> Result<Self, HostError> {
        let Some(observer) = trees.close_observer()? else {
            return Ok(Self(None));
        };
        let profile = Profile::new(
            composition
                .topology()
                .map_err(HostError::backend)?
                .profile(),
        )?;
        let storage = profile::reserve_owner(composition.resources())?;
        let topology = profile
            .topology
            .bind(
                &*composition.topology().map_err(HostError::backend)?,
                composition.resources(),
            )
            .map_err(HostError::backend)?;
        let reservations = profile
            .runtime
            .bind(&runtime.lifecycle_handle(), composition.resources())
            .map_err(HostError::backend)?;
        let native = composition
            .native_close_scheduler(
                profile.scheduler,
                profile.authority,
                profile.pages,
                crate::close_services::NativeStop(stop),
            )
            .map_err(HostError::backend)?;
        Ok(Self(Some(Box::new(Selected {
            native,
            topology,
            runtime: reservations,
            observer,
            ready: std::array::from_fn(|_| None),
            head: 0,
            len: 0,
            wakeup: Arc::new(Notify::new()),
            _storage: storage,
        }))))
    }
    pub(crate) fn wakeup(&self) -> Option<Arc<Notify>> {
        self.0.as_ref().map(|jobs| jobs.wakeup.clone())
    }
    pub(crate) async fn drive<F: Future>(&mut self, future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        poll_fn(|cx| {
            if let Some(jobs) = &mut self.0 {
                jobs.poll_native(cx);
            }
            future.as_mut().poll(cx)
        })
        .await
    }
    pub(crate) fn admit(
        &mut self,
        state: &Mutex<Option<LiveSession>>,
        request: DriverRequest,
    ) -> Result<Admission, HostError> {
        let Some(jobs) = &mut self.0 else {
            return Ok(Admission::Other(request));
        };
        let target = match &request {
            DriverRequest::ClosePage { page, .. } => ChromiumCloseTarget::Page(*page),
            DriverRequest::CloseContext { context, .. } => {
                ChromiumCloseTarget::BrowserContext(*context)
            }
            _ => return Ok(Admission::Other(request)),
        };
        let state = state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        let topology = live.composition.topology().map_err(HostError::backend)?;
        let context = match target {
            ChromiumCloseTarget::Page(page) => topology
                .topology()
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .map(|route| route.browser_context_object_id()),
            ChromiumCloseTarget::BrowserContext(context) => Some(context),
        };
        if context
            .and_then(|id| topology.topology().browser_context_route(id))
            .is_none_or(|route| route.ownership() != CdpContextOwnership::Owned)
        {
            return Ok(Admission::Other(request));
        }
        let sender = match request {
            DriverRequest::ClosePage { reply, .. } | DriverRequest::CloseContext { reply, .. } => {
                reply
            }
            _ => unreachable!("only close requests reach admission"),
        };
        if sender.is_closed() {
            return Ok(Admission::Queued);
        }
        let reply = Reply { target, sender };
        let capacity = match ChromiumCloseCapacityBundle::reserve(
            target,
            &topology,
            &jobs.topology,
            &jobs.runtime,
            jobs.observer.as_ref(),
        ) {
            Ok(capacity) => capacity,
            Err(error) => return Ok(Admission::Reply(reply.finish(Err(capacity_error(error))))),
        };
        // Scheduler admission obtains the same topology mutex. No lifecycle
        // mutation gate or outer host lock is retained across native polling.
        drop(topology);
        drop(state);
        let payload = PendingClose {
            capacity,
            scratch: [ObjectId::new(0); MAX_OBJECTS],
            reply,
        };
        let result = match target {
            ChromiumCloseTarget::Page(page) => jobs.native.push_page(page, payload),
            ChromiumCloseTarget::BrowserContext(context) => {
                jobs.native.push_context(context, payload)
            }
        };
        Ok(match result {
            Ok(()) => Admission::Queued,
            Err((payload, error)) => {
                Admission::Reply(payload.reply.finish(Err(HostError::operation(error, 0))))
            }
        })
    }
    pub(crate) fn finish_next(
        &mut self,
        state: &Mutex<Option<LiveSession>>,
    ) -> Result<Option<DriverReply>, HostError> {
        let Some(completion) = self.0.as_mut().and_then(|jobs| jobs.pop_ready()) else {
            return Ok(None);
        };
        completion.consume(|payload, result| {
            let evidence = match result {
                Ok(evidence) => evidence,
                Err(error) => {
                    return Ok(Some(payload.reply.finish(Err(HostError::operation(
                        pwright_core::OperationError::<()>::new(
                            error.kind(),
                            error.message(),
                            None,
                        ),
                        0,
                    )))));
                }
            };
            let PendingClose {
                capacity,
                mut scratch,
                reply,
            } = payload;
            let runtime = state
                .lock()
                .map_err(HostError::backend)?
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .runtime
                .clone();
            let committed = {
                let state = state.lock().map_err(HostError::backend)?;
                let live = state.as_ref().ok_or(HostError::TargetClosed)?;
                live.composition
                    .retire_native_close(evidence, capacity, &runtime, &mut scratch, None)
                    .map_err(HostError::backend)?
            };
            // Publication failure after native success is terminal, never a
            // recoverable operation error followed by more lifecycle mutations.
            committed.notify().map_err(HostError::backend)?;
            Ok(Some(reply.finish(Ok(()))))
        })
    }
    pub(crate) async fn drain(&mut self) {
        let Some(jobs) = &mut self.0 else {
            return;
        };
        // Stop is already signalled. Do not locally publish successful close
        // evidence during terminal teardown, but retain every written command.
        while jobs.pop_ready().is_some() {}
        while poll_fn(|cx| jobs.native.poll_next(cx)).await.is_some() {}
    }
}
impl Selected {
    fn poll_native(&mut self, cx: &mut Context<'_>) {
        while self.len < MAX_CLOSES {
            let Poll::Ready(Some(completion)) = self.native.poll_next(cx) else {
                break;
            };
            self.ready[(self.head + self.len) % MAX_CLOSES] = Some(completion);
            self.len += 1;
            self.wakeup.notify_one();
        }
    }
    fn pop_ready(&mut self) -> Option<NativeCloseCompletion<PendingClose>> {
        if self.len == 0 {
            return None;
        }
        let completion = self.ready[self.head].take();
        self.head = (self.head + 1) % MAX_CLOSES;
        self.len -= 1;
        if self.len != 0 {
            self.wakeup.notify_one();
        }
        completion
    }
}
pub(crate) async fn notified(wakeup: &Option<Arc<Notify>>) {
    match wakeup {
        Some(signal) => signal.notified().await,
        None => std::future::pending().await,
    }
}
fn capacity_error(
    error: pwright_composition_chromium::ChromiumCloseCapacityBundleError,
) -> HostError {
    use pwright_backend_cdp_topology::CdpTopologyProductionError;
    use pwright_backend_cdp_topology::transition_reservations::TransitionReservationError as Topology;
    use pwright_composition_chromium::ChromiumCloseCapacityBundleError as Error;
    use pwright_runtime::RuntimeStateError;
    use pwright_runtime::disposal_reservations::DisposalReservationError as Runtime;
    match error {
        Error::Publication(error) => HostError::operation(error, 0),
        Error::Runtime(Runtime::Runtime(RuntimeStateError::TransportClosed))
        | Error::Topology(Topology::EpochChanged { .. }) => HostError::TargetClosed,
        Error::Topology(Topology::SlotsExhausted { .. })
        | Error::Runtime(Runtime::SlotsExhausted { .. })
        | Error::Runtime(Runtime::Runtime(
            RuntimeStateError::ControlMessageCapacityExceeded { .. }
            | RuntimeStateError::LifecycleTransactionInProgress
            | RuntimeStateError::LifecycleTransactionIdExhausted
            | RuntimeStateError::Resource(_),
        ))
        | Error::Topology(Topology::Production(
            CdpTopologyProductionError::Resource(_)
            | CdpTopologyProductionError::TransactionInProgress
            | CdpTopologyProductionError::TransactionIdExhausted,
        )) => HostError::ResourceExhausted(error.to_string()),
        other => HostError::backend(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_composition_chromium::ChromiumCloseCapacityBundleError as Capacity;
    use pwright_runtime::{RuntimeStateError, disposal_reservations::DisposalReservationError};

    #[test]
    fn full_outbox_is_an_admission_error_not_terminal_peer_loss() {
        // Arrange
        let error = Capacity::Runtime(DisposalReservationError::Runtime(
            RuntimeStateError::ControlMessageCapacityExceeded {
                required: 256,
                remaining: 0,
                limit: 256,
            },
        ));
        // Act
        let result = capacity_error(error);
        // Assert
        assert!(matches!(result, HostError::ResourceExhausted(_)));
    }

    #[test]
    fn closed_transport_keeps_its_terminal_classification() {
        // Arrange
        let error = Capacity::Runtime(DisposalReservationError::Runtime(
            RuntimeStateError::TransportClosed,
        ));
        // Act
        let result = capacity_error(error);
        // Assert
        assert!(matches!(result, HostError::TargetClosed));
    }

    #[tokio::test]
    async fn page_terminal_reply_remains_idempotent() {
        // Arrange
        let (sender, receiver) = oneshot::channel();
        let reply = Reply {
            target: ChromiumCloseTarget::Page(ObjectId::new(3)),
            sender,
        };
        // Act
        reply.finish(Err(HostError::TargetClosed)).send();
        // Assert
        assert!(receiver.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn context_terminal_reply_does_not_claim_native_success() {
        // Arrange
        let (sender, receiver) = oneshot::channel();
        let reply = Reply {
            target: ChromiumCloseTarget::BrowserContext(ObjectId::new(2)),
            sender,
        };
        // Act
        reply.finish(Err(HostError::TargetClosed)).send();
        // Assert
        assert!(matches!(
            receiver.await.unwrap(),
            Err(HostError::TargetClosed)
        ));
    }
}
