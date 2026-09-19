//! The synchronous local half of a natively completed Page/Context close.
//! No command, native wait, event-loop task or publication listener is started.
use crate::*;
use native_close_authority::scheduler::{NativeCloseCommitTarget, NativeCloseEvidence};

type CloseResult<T> = Result<T, OperationError<()>>;
fn error(kind: FailureKind, message: &'static str) -> OperationError<()> {
    OperationError::new(kind, message, None)
}
fn backend(cause: impl Display) -> OperationError<()> {
    OperationError::new(FailureKind::Backend, cause.to_string(), None)
}

fn runtime_failure(cause: RuntimeStateError) -> OperationError<()> {
    map_runtime_context_close_error(cause).map_details(|_| ())
}
fn reservation_failure(
    cause: pwright_runtime::disposal_reservations::DisposalReservationError,
) -> OperationError<()> {
    use pwright_runtime::disposal_reservations::DisposalReservationError;
    match cause {
        DisposalReservationError::Runtime(cause) => runtime_failure(cause),
        DisposalReservationError::SlotsExhausted { .. } => {
            OperationError::new(FailureKind::ResourceExhausted, cause.to_string(), None)
        }
        DisposalReservationError::Invalid(_) => {
            OperationError::new(FailureKind::ProtocolViolation, cause.to_string(), None)
        }
    }
}

fn topology_failure(
    cause: pwright_backend_cdp_topology::transition_reservations::TransitionReservationError,
) -> OperationError<()> {
    use pwright_backend_cdp_topology::transition_reservations::TransitionReservationError;
    match cause {
        TransitionReservationError::Production(cause) => {
            map_topology_context_close_error(cause).map_details(|_| ())
        }
        TransitionReservationError::SlotsExhausted { .. } => {
            OperationError::new(FailureKind::ResourceExhausted, cause.to_string(), None)
        }
        TransitionReservationError::EpochChanged { .. } => {
            OperationError::new(FailureKind::TargetClosed, cause.to_string(), None)
        }
        TransitionReservationError::InvalidScope { .. }
        | TransitionReservationError::ProfileMismatch => {
            OperationError::new(FailureKind::ProtocolViolation, cause.to_string(), None)
        }
    }
}

impl AttachedChromiumComposition {
    /// Consume original capacity under this attachment's sealed native evidence.
    /// Call from the sole lifecycle publication owner, after native completion.
    /// The runtime and its Page services must be the ones installed on this
    /// attachment; runtime reservations additionally enforce exact owner identity.
    ///
    /// `scratch` is caller-admitted stable ObjectId storage, normally an inline
    /// fixed array in the scheduler payload. It must fit the latest runtime
    /// subtree. No scratch vector or second Handle-ID list is allocated here.
    /// Existing topology/runtime preparation storage and diagnostic strings keep
    /// their respective admission contracts; this is not an allocator/RSS bound.
    ///
    /// A selected network registry requires its exact driver registration port.
    /// Lock order is network driver, topology, Handles, routes/subscriptions,
    /// runtime. All fallible local preflight precedes topology/runtime mutation.
    /// Native-world marking records actual native proof, independently of whether
    /// local commit succeeds. Publication must not race another publication owner.
    /// The returned owner retains publication and cancellation notifications;
    /// call `notify` only AFTER releasing the host's outer session-state lock
    /// and all other caller locks. Its lifetime depends on runtime/scratch, not
    /// this attachment reference, so it can leave an attachment lock scope.
    pub fn retire_native_close<'a, Deadlines, Cleanup>(
        &self,
        evidence: &NativeCloseEvidence,
        capacity: ChromiumCloseCapacityBundle,
        runtime: &'a ControlledDispatcher<CdpPageServices, CdpPageServices, Deadlines, Cleanup>,
        scratch: &'a mut [ObjectId],
        network: Option<&ChromiumNetworkPageRegistrationHandle>,
    ) -> CloseResult<CommittedNativeClose<'a>>
    where
        Deadlines: DeadlineScheduler,
        Cleanup: CleanupExecutor,
    {
        let services = runtime.service();
        retire_network(services, network, capacity.target(), || {
            self.with_native_close_topology(evidence, |target, topology| {
                retire_local(
                    target,
                    topology,
                    capacity,
                    runtime.lifecycle_handle(),
                    services,
                    scratch,
                )
            })?
        })
    }
}

/// Local state is retired, but publication and cancellation notifications are
/// still owned. Does not borrow the attachment, native proof or any held lock.
/// Dropping without notification does not roll back the already committed state.
#[must_use = "notify after releasing every caller/session-state lock"]
pub struct CommittedNativeClose<'a> {
    publication: Box<dyn ChromiumCloseReservation>,
    update: CdpTopologyUpdate,
    runtime: pwright_runtime::disposal_reservations::DeferredDisposalCommit,
    routes: close_route_retirement::CloseRouteRetirementWake<'a>,
}
impl CommittedNativeClose<'_> {
    /// Invoke only after releasing all enclosing host/domain locks. Publication
    /// failure is returned after already-committed cancellation notifications
    /// are delivered; the host must treat a failed publication as terminal.
    pub fn notify(mut self) -> CloseResult<()> {
        // Even failed publication cannot abandon already committed cancellation
        // notifications. Return its error after releasing all committed waiters.
        let publication = self.publication.committed(self.update.effects());
        self.routes.notify();
        self.runtime.notify();
        publication
    }
}

pub(crate) fn retire_local<'a>(
    native: NativeCloseCommitTarget<'_>,
    topology: &mut ProductionCdpTopology,
    capacity: ChromiumCloseCapacityBundle,
    runtime: ControlledLifecycleHandle,
    services: &'a CdpPageServices,
    scratch: &'a mut [ObjectId],
) -> CloseResult<CommittedNativeClose<'a>> {
    let parts = capacity.into_parts();
    let (target, object) = match native {
        NativeCloseCommitTarget::Page { object, .. } => (ChromiumCloseTarget::Page(object), object),
        NativeCloseCommitTarget::Context { object, .. } => {
            (ChromiumCloseTarget::BrowserContext(object), object)
        }
    };
    if target != parts.target {
        return Err(error(
            FailureKind::ProtocolViolation,
            "native close target differs from admitted intent",
        ));
    }
    let runtime = parts
        .runtime
        .prepare(&runtime, object)
        .map_err(reservation_failure)?;
    let count = runtime.disposal_order().len();
    if count > scratch.len() {
        return Err(error(
            FailureKind::ResourceExhausted,
            "native close disposal exceeds admitted scratch",
        ));
    }
    let ids = &mut scratch[..count];
    ids.copy_from_slice(runtime.disposal_order());
    let epoch = topology.epoch();
    let candidate = parts
        .topology
        .prepare(topology, |next| match native {
            NativeCloseCommitTarget::Page {
                target_id,
                session_id,
                ..
            } => next.detach_target(epoch, session_id, Some(target_id)),
            NativeCloseCommitTarget::Context {
                browser_context_id, ..
            } => next.close_context(epoch, browser_context_id),
        })
        .map_err(topology_failure)?;
    let mut compatibility = services
        .handle_routes
        .as_ref()
        .map(|handles| {
            handles
                .state
                .lock()
                .map_err(|_| error(FailureKind::Backend, "Handle route state is poisoned"))
        })
        .transpose()?;
    let leaf = if compatibility.is_none() {
        services
            .handle_leaf_routes
            .as_ref()
            .map(|handles| handles.borrow_parent_retirement(ids))
            .transpose()
            .map_err(backend)?
    } else {
        None
    };
    let routes = services
        .prepare_close_route_retirement(candidate.topology(), ids)
        .map_err(backend)?;
    let publication = parts
        .publication
        .prepare(ChromiumClosePlan::new(target, candidate.output().effects()))?;
    routes
        .observe_native_destruction(match native {
            NativeCloseCommitTarget::Page { target_id, .. } => {
                native_world_retirement::Destruction::PageTarget(target_id)
            }
            NativeCloseCommitTarget::Context { object, .. } => {
                native_world_retirement::Destruction::Context(object)
            }
        })
        .map_err(backend)?;
    let ((update, routes), runtime) = runtime
        .commit_deferred_with_publication(None, || {
            let update = candidate.commit(topology).map_err(topology_failure)?;
            let routes = routes.commit();
            if let Some(handles) = &mut compatibility {
                retire_handle_routes(
                    handles,
                    ids,
                    ChromiumHandleRetirementReason::ParentDestroyed,
                );
            }
            if let Some(leaf) = leaf {
                leaf.commit();
            }
            Ok::<_, OperationError<()>>((update, routes))
        })
        .map_err(reservation_failure)??;
    drop(compatibility);
    Ok(CommittedNativeClose {
        publication,
        update,
        runtime,
        routes,
    })
}

pub(crate) fn retire_network<T>(
    services: &CdpPageServices,
    registration: Option<&ChromiumNetworkPageRegistrationHandle>,
    target: ChromiumCloseTarget,
    publish: impl FnOnce() -> CloseResult<T>,
) -> CloseResult<T> {
    let (Some(subscriptions), Some(registration)) = (&services.network_subscriptions, registration)
    else {
        return if services.network_subscriptions.is_none() && registration.is_none() {
            publish()
        } else {
            Err(error(
                FailureKind::ProtocolViolation,
                "native close requires the exact selected network driver",
            ))
        };
    };
    let driver = registration.driver().map_err(backend)?;
    if !Arc::ptr_eq(&subscriptions.shared, &driver.subscriptions.shared) {
        return Err(error(
            FailureKind::ProtocolViolation,
            "native close network registry belongs to another runtime",
        ));
    }
    let mut state = driver
        .state
        .lock()
        .map_err(|_| error(FailureKind::Backend, "network driver state is poisoned"))?;
    ensure_network_batch_published(&state).map_err(backend)?;
    if state.pending_page_registration.is_some() || state.pending_page_replacement.is_some() {
        return Err(error(
            FailureKind::Backend,
            "network page transition overlaps native close",
        ));
    }
    let result = publish()?;
    // Runtime subtree disposal already emitted all Request/Response Dispose
    // messages; routes already pruned subscriptions. Do not allocate or replay
    // another cleanup vector. Missing adapters are valid ancestor-first joins.
    state.adapters.retain(|page, adapter| match target {
        ChromiumCloseTarget::Page(object) => *page != object,
        ChromiumCloseTarget::BrowserContext(object) => adapter.browser_context_object_id != object,
    });
    Ok(result)
}
