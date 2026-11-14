//! Executor-independent in-process dispatch for typed Pwright capabilities.
//!
//! This crate performs no serialization and starts no tasks. A caller-provided
//! executor polls the future returned by `Service::call`. Relative deadlines
//! are also injected as futures, so this crate never reads a system clock.

mod cleanup;
mod connection_scheduler;
mod controlled_config;
mod target_scheduler;

pub use controlled_config::ControlledRuntimeConfig;

pub use cleanup::{
    CleanupExecutor, CleanupFailure, CleanupJob, CleanupQueue, CleanupQueueMetrics,
    CleanupQueueReservation, CleanupReservation, CleanupSubmitError, ReservedCleanupExecutor,
};
pub use connection_scheduler::{
    ConnectionPermit, ConnectionPermitFuture, ConnectionScheduler, ConnectionSchedulerError,
    ConnectionSchedulerLimits, ConnectionSchedulerMetrics,
};

pub use target_scheduler::{
    TargetPermit, TargetPermitFuture, TargetScheduler, TargetSchedulerError, TargetSchedulerLimits,
    TargetSchedulerMetrics,
};

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::{Future, Pending, poll_fn};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use pwright_core::{
    BoundedQueue, CallId, CallMetadata, CancellationId, CancellationReason,
    CancellationRequestOutcome, Command, CommandCancellation, CommandResult, CommandSpec,
    ControlMessage, EMPTY_OBJECT_KIND_CATALOG, FailureKind, ObjectGraph, ObjectGraphError,
    ObjectId, ObjectKindCatalog, ObjectKindCatalogError, ObjectKindId, ObjectKindTag, ObjectRecord,
    OperationContext, OperationError, OverflowPolicy, PendingCallError, PendingCallState,
    PendingCallTable, ProductionResourceError, ProductionResourceLedger, PushOutcome, QueueMetrics,
    ResourceClass, ResourceError, ResourceLease, ResourceLedger, ResourceRequirements,
    ResourceScope, Service,
};

#[cfg(feature = "lifetime-handoff")]
mod lifetime_handoff;
#[cfg(feature = "lifetime-handoff")]
pub use lifetime_handoff::{CallLifetimeHandoff, LifetimeHandoffError};

#[cfg(feature = "disposal-reservations")]
pub mod disposal_reservations;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeLimits {
    pub live_objects: usize,
    pub pending_calls: usize,
    pub active_commands: usize,
    pub control_messages: Option<usize>,
}

/// A command service that receives the validated portable target identity.
///
/// Browser backends need this boundary to route two `Page` objects to two
/// different native sessions. Plain parameter-only [`Service`] values remain
/// source-compatible through the blanket implementation below.
pub trait TargetedService<Request>: Send + Sync {
    type Response;
    type Error;
    type Future<'a>: Future<Output = Result<Self::Response, Self::Error>> + Send
    where
        Self: 'a;

    fn call_for(&self, object_id: ObjectId, request: Request) -> Self::Future<'_>;
}

impl<Request, S> TargetedService<Request> for S
where
    S: Service<Request> + Send + Sync,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future<'a>
        = S::Future<'a>
    where
        Self: 'a;

    fn call_for(&self, _object_id: ObjectId, request: Request) -> Self::Future<'_> {
        self.call(request)
    }
}

/// Controls whether parentless lifecycle objects represent complete roots or
/// intentionally partial externally attached graphs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ObjectTopologyMode {
    /// Parentless objects of any installed kind are accepted. Whenever a
    /// parent is present, its semantic kind is still validated.
    #[default]
    Partial,
    /// Parentless objects must be declared roots by their kind descriptor.
    Complete,
}

/// A typed terminal failure injected by an object-lifecycle owner without
/// disposing the object graph.
///
/// This is distinct from [`CancellationReason`]: a crashed browser target, for
/// example, must terminate work in that target's subtree as
/// [`FailureKind::BrowserCrashed`] rather than pretending the target was
/// closed. The object remains registered so later calls can observe the
/// backend's durable failed state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeCallFailure {
    kind: FailureKind,
    message: Arc<str>,
}

impl RuntimeCallFailure {
    pub fn new(kind: FailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: Arc::from(message.into()),
        }
    }

    pub const fn kind(&self) -> FailureKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    fn operation_error<D>(&self) -> OperationError<D> {
        OperationError::new(self.kind, self.message.to_string(), None)
    }
}

impl RuntimeLimits {
    pub const fn new(live_objects: usize, pending_calls: usize) -> Self {
        Self {
            live_objects,
            pending_calls,
            active_commands: pending_calls,
            control_messages: None,
        }
    }

    pub const fn with_active_commands(mut self, active_commands: usize) -> Self {
        self.active_commands = active_commands;
        self
    }

    /// Enables a bounded server-to-client lifecycle outbox.
    ///
    /// The default is disabled so applications that use the in-process
    /// dispatcher do not pay for or consume lifecycle messages.
    pub const fn with_control_messages(mut self, capacity: usize) -> Self {
        self.control_messages = Some(capacity);
        self
    }

    /// Declares the maximum count and metadata capacity this runtime profile
    /// needs from a production resource plan.
    pub fn resource_requirements(self) -> ResourceRequirements {
        let live_objects = u64::try_from(self.live_objects).unwrap_or(u64::MAX);
        let pending_calls = u64::try_from(self.pending_calls).unwrap_or(u64::MAX);
        let active_commands =
            u64::try_from(self.active_commands.min(self.pending_calls)).unwrap_or(u64::MAX);
        let control_messages = self.control_messages.unwrap_or(0);
        let control_units = u64::try_from(control_messages).unwrap_or(u64::MAX);
        let requirements = ResourceRequirements::none()
            .with_class(
                ResourceClass::LiveObject,
                live_objects,
                u64::from(self.live_objects != 0),
            )
            .with_class(
                ResourceClass::PendingCall,
                pending_calls,
                u64::from(self.pending_calls != 0),
            )
            .with_class(
                ResourceClass::ActiveCommand,
                active_commands,
                u64::from(active_commands != 0),
            )
            .with_class(ResourceClass::EventItem, control_units, control_units)
            .with_metadata(
                self.live_objects
                    .saturating_add(self.pending_calls)
                    .saturating_add(control_messages),
                self.live_objects
                    .saturating_add(self.pending_calls)
                    .saturating_add(usize::try_from(active_commands).unwrap_or(usize::MAX))
                    .saturating_add(control_messages)
                    .saturating_add(if cfg!(feature = "lifetime-handoff") {
                        self.pending_calls
                    } else {
                        0
                    }),
            );
        #[cfg(feature = "lifetime-handoff")]
        let requirements = requirements.with_class(
            ResourceClass::BackendStateBytes,
            pending_calls.saturating_mul(lifetime_handoff::SIGNAL_BYTES),
            if self.pending_calls == 0 {
                0
            } else {
                lifetime_handoff::SIGNAL_BYTES
            },
        );
        requirements
    }
}

/// Supplies a future that expires after a relative timeout budget.
///
/// Returning `None` means this scheduler cannot enforce deadlines. The runtime
/// then rejects commands carrying a timeout before invoking their service.
pub trait DeadlineScheduler: Send + Sync {
    type Sleep<'a>: Future<Output = ()> + Send
    where
        Self: 'a;

    fn sleep(&self, budget: Duration) -> Option<Self::Sleep<'_>>;
}

/// Deadline policy for the standard direct dispatcher.
///
/// It deliberately supplies no clock and never silently ignores a timeout.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoDeadlineScheduler;

impl DeadlineScheduler for NoDeadlineScheduler {
    type Sleep<'a> = Pending<()>;

    fn sleep(&self, _budget: Duration) -> Option<Self::Sleep<'_>> {
        None
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum RuntimeStateError {
    ObjectGraph(ObjectGraphError),
    UnknownObjectKind {
        tag: ObjectKindTag,
    },
    MissingRequiredParent {
        object_id: ObjectId,
        kind: ObjectKindId,
    },
    InvalidParentKind {
        object_id: ObjectId,
        kind: ObjectKindId,
        parent: ObjectId,
        parent_kind: ObjectKindId,
    },
    UnsupportedLifecycleMessage {
        message: ControlMessage,
    },
    PendingCall(PendingCallError),
    ObjectHasPendingCalls {
        object_id: ObjectId,
        pending_calls: usize,
    },
    PendingCallsPreventTransportClose {
        pending_calls: usize,
    },
    MissingCancellationSignal {
        call_id: CallId,
    },
    ControlMessageCapacityExceeded {
        required: usize,
        remaining: usize,
        limit: usize,
    },
    ControlMessageClaimInProgress,
    ClaimedControlMessageUnavailable,
    LifecycleTransactionInProgress,
    LifecycleTransactionUnavailable,
    LifecycleTransactionIdExhausted,
    EmptyObjectCreationBatch,
    ResourceLedgerAlreadyInstalled,
    ResourceLedgerRequiresEmptyState,
    ConnectionScheduler(ConnectionSchedulerError),
    ProductionResource(ProductionResourceError),
    Resource(ResourceError),
    TransportClosed,
    StatePoisoned,
}

impl Display for RuntimeStateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObjectGraph(error) => Display::fmt(error, formatter),
            Self::UnknownObjectKind { tag } => {
                write!(formatter, "object kind tag `{tag}` is not installed")
            }
            Self::MissingRequiredParent { object_id, kind } => write!(
                formatter,
                "object `{object_id}` of kind `{kind}` requires a parent in a complete graph"
            ),
            Self::InvalidParentKind {
                object_id,
                kind,
                parent,
                parent_kind,
            } => write!(
                formatter,
                "object `{object_id}` of kind `{kind}` cannot use object `{parent}` of kind `{parent_kind}` as its parent"
            ),
            Self::UnsupportedLifecycleMessage { message } => write!(
                formatter,
                "runtime lifecycle batch does not accept control message {message:?}"
            ),
            Self::PendingCall(error) => Display::fmt(error, formatter),
            Self::ObjectHasPendingCalls {
                object_id,
                pending_calls,
            } => write!(
                formatter,
                "object `{object_id}` subtree still has {pending_calls} pending call(s)"
            ),
            Self::PendingCallsPreventTransportClose { pending_calls } => write!(
                formatter,
                "runtime transport still has {pending_calls} pending call(s)"
            ),
            Self::MissingCancellationSignal { call_id } => {
                write!(
                    formatter,
                    "call `{call_id}` has no runtime cancellation signal"
                )
            }
            Self::ControlMessageCapacityExceeded {
                required,
                remaining,
                limit,
            } => write!(
                formatter,
                "lifecycle mutation requires {required} control-message slot(s), but only {remaining} of {limit} remain"
            ),
            Self::ControlMessageClaimInProgress => {
                formatter.write_str("a lifecycle control message is already claimed")
            }
            Self::ClaimedControlMessageUnavailable => {
                formatter.write_str("the claimed lifecycle control message is no longer available")
            }
            Self::LifecycleTransactionInProgress => {
                formatter.write_str("a prepared lifecycle transaction is already in progress")
            }
            Self::LifecycleTransactionUnavailable => formatter
                .write_str("the prepared lifecycle transaction was invalidated before commit"),
            Self::LifecycleTransactionIdExhausted => {
                formatter.write_str("runtime lifecycle transaction IDs are exhausted")
            }
            Self::EmptyObjectCreationBatch => {
                formatter.write_str("object creation batch must contain at least one object")
            }
            Self::ResourceLedgerAlreadyInstalled => {
                formatter.write_str("runtime resource ledger is already installed")
            }
            Self::ResourceLedgerRequiresEmptyState => formatter.write_str(
                "runtime resource ledger must be installed before objects or calls are retained",
            ),
            Self::ConnectionScheduler(error) => Display::fmt(error, formatter),
            Self::ProductionResource(error) => Display::fmt(error, formatter),
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::TransportClosed => formatter.write_str("runtime transport is closed"),
            Self::StatePoisoned => formatter.write_str("runtime state lock was poisoned"),
        }
    }
}

impl Error for RuntimeStateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ObjectGraph(error) => Some(error),
            Self::PendingCall(error) => Some(error),
            Self::ConnectionScheduler(error) => Some(error),
            Self::ProductionResource(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::UnknownObjectKind { .. }
            | Self::MissingRequiredParent { .. }
            | Self::InvalidParentKind { .. }
            | Self::UnsupportedLifecycleMessage { .. }
            | Self::ObjectHasPendingCalls { .. }
            | Self::PendingCallsPreventTransportClose { .. }
            | Self::MissingCancellationSignal { .. }
            | Self::ControlMessageCapacityExceeded { .. }
            | Self::ControlMessageClaimInProgress
            | Self::ClaimedControlMessageUnavailable
            | Self::LifecycleTransactionInProgress
            | Self::LifecycleTransactionUnavailable
            | Self::LifecycleTransactionIdExhausted
            | Self::EmptyObjectCreationBatch
            | Self::ResourceLedgerAlreadyInstalled
            | Self::ResourceLedgerRequiresEmptyState
            | Self::TransportClosed
            | Self::StatePoisoned => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportCloseOutcome {
    already_closed: bool,
    pending_calls: usize,
    invalidated_objects: Vec<ObjectRecord<ObjectKindTag>>,
}

/// Exclusive, rollback-on-drop access to the oldest lifecycle message.
///
/// Encoding can fail without losing the message: only `commit` removes it.
/// At most one claim may exist for a dispatcher at a time.
pub struct ControlMessageClaim {
    store: Arc<RuntimeStore>,
    message: ControlMessage,
    finished: bool,
}

impl ControlMessageClaim {
    pub const fn message(&self) -> ControlMessage {
        self.message
    }

    pub fn commit(mut self) -> Result<(), RuntimeStateError> {
        let mut state = self
            .store
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.transport_closed {
            return Err(RuntimeStateError::TransportClosed);
        }
        if !state.control_message_claimed {
            return Err(RuntimeStateError::ClaimedControlMessageUnavailable);
        }
        let Some(queue) = &mut state.control_messages else {
            return Err(RuntimeStateError::ClaimedControlMessageUnavailable);
        };
        if queue.front().copied() != Some(self.message) {
            return Err(RuntimeStateError::ClaimedControlMessageUnavailable);
        }
        let removed = queue.pop();
        debug_assert_eq!(removed, Some(self.message));
        if state.resource_ledger.is_some() {
            let resource = state
                .control_resources
                .pop_front()
                .expect("every budgeted control message owns one resource lease");
            drop(resource);
        } else {
            debug_assert!(state.control_resources.is_empty());
        }
        state.control_message_claimed = false;
        self.finished = true;
        Ok(())
    }
}

impl Drop for ControlMessageClaim {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if let Ok(mut state) = self.store.state.lock() {
            state.control_message_claimed = false;
        }
    }
}

/// A resource-reserved, rollback-on-drop object creation that has not yet
/// changed the runtime graph.
///
/// The caller may allocate a backend resource between prepare and commit. The
/// object becomes live, and its `Create` message becomes visible, only when
/// commit succeeds. Dropping this value burns the caller-owned object ID while
/// releasing every runtime reservation.
pub struct PreparedObjectCreation {
    store: Arc<RuntimeStore>,
    transaction_id: u64,
    object_id: ObjectId,
    parent: Option<ObjectId>,
    kind: ObjectKindTag,
    object_kinds: ObjectKindCatalog,
    topology: ObjectTopologyMode,
    object_resource: Option<ResourceLease>,
    control_resources: Option<VecDeque<ResourceLease>>,
    finished: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectCreationSpec {
    pub object_id: ObjectId,
    pub parent: Option<ObjectId>,
    pub kind: ObjectKindTag,
}

impl ObjectCreationSpec {
    pub const fn new(object_id: ObjectId, parent: Option<ObjectId>, kind: ObjectKindTag) -> Self {
        Self {
            object_id,
            parent,
            kind,
        }
    }
}

/// A rollback-on-drop transaction that reserves and publishes a related set
/// of objects in exact order. The graph and control outbox remain unchanged
/// until the whole batch commits.
pub struct PreparedObjectCreationBatch {
    store: Arc<RuntimeStore>,
    transaction_id: u64,
    objects: Vec<ObjectCreationSpec>,
    object_kinds: ObjectKindCatalog,
    topology: ObjectTopologyMode,
    object_resources: Vec<ResourceLease>,
    control_resources: Option<VecDeque<ResourceLease>>,
    finished: bool,
}

/// A resource-reserved, rollback-on-drop object disposal that has not yet
/// changed the runtime graph.
///
/// This transaction is intended to cross one caller-owned asynchronous
/// backend close. While it exists, other lifecycle mutations fail explicitly;
/// ordinary calls may finish or queue and are cancelled child-first at commit.
/// Transport closure invalidates the transaction and remains authoritative.
pub struct PreparedObjectDisposal {
    store: Arc<RuntimeStore>,
    transaction_id: u64,
    object_id: ObjectId,
    disposal_order: Vec<ObjectId>,
    control_resources: Option<VecDeque<ResourceLease>>,
    #[cfg(feature = "disposal-reservations")]
    reserved_control_slots: usize,
    finished: bool,
}

/// Cloneable, service-safe access to the controlled runtime's lifecycle owner.
///
/// This handle lets a separately composed lifecycle command coordinate a
/// backend transaction without giving it the dispatcher service or schedulers.
#[derive(Clone)]
pub struct ControlledLifecycleHandle {
    store: Arc<RuntimeStore>,
    object_kinds: ObjectKindCatalog,
    topology: ObjectTopologyMode,
}

impl ControlledLifecycleHandle {
    pub fn prepare_object_creation(
        &self,
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: ObjectKindTag,
    ) -> Result<PreparedObjectCreation, RuntimeStateError> {
        self.store.prepare_object_creation(
            self.object_kinds,
            self.topology,
            object_id,
            parent,
            kind,
        )
    }

    pub fn prepare_object_creation_batch(
        &self,
        objects: &[ObjectCreationSpec],
    ) -> Result<PreparedObjectCreationBatch, RuntimeStateError> {
        self.store
            .prepare_object_creation_batch(self.object_kinds, self.topology, objects)
    }

    pub fn prepare_object_disposal(
        &self,
        object_id: ObjectId,
    ) -> Result<PreparedObjectDisposal, RuntimeStateError> {
        self.store.prepare_object_disposal(object_id)
    }

    pub fn is_object_live(&self, object_id: ObjectId) -> Result<bool, RuntimeStateError> {
        self.store.is_object_live(object_id)
    }

    pub fn lifecycle_transaction_active(&self) -> Result<bool, RuntimeStateError> {
        self.store.lifecycle_transaction_active()
    }
}

impl PreparedObjectCreation {
    pub const fn object_id(&self) -> ObjectId {
        self.object_id
    }

    pub const fn parent(&self) -> Option<ObjectId> {
        self.parent
    }

    pub const fn kind(&self) -> ObjectKindTag {
        self.kind
    }

    /// Atomically publishes the object and its reserved `Create` control
    /// message after the caller's backend creation has succeeded.
    pub fn commit(mut self) -> Result<(), RuntimeStateError> {
        let mut state = self
            .store
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.active_lifecycle_transaction != Some(self.transaction_id) {
            return Err(RuntimeStateError::LifecycleTransactionUnavailable);
        }
        if state.transport_closed {
            return Err(RuntimeStateError::TransportClosed);
        }

        validate_object_parent_in_graph(
            &state.objects,
            self.object_kinds,
            self.topology,
            self.object_id,
            self.kind,
            self.parent,
        )?;
        state.objects.validate_insert(self.object_id, self.parent)?;
        state
            .objects
            .insert(self.object_id, self.parent, self.kind)?;
        if let Some(resource) = self.object_resource.take() {
            let previous = state.object_resources.insert(self.object_id, resource);
            debug_assert!(previous.is_none());
        }
        let message = ControlMessage::Create {
            object_id: self.object_id,
            parent: self.parent,
            kind: self.kind,
        };
        let control_resource = self
            .control_resources
            .as_mut()
            .and_then(VecDeque::pop_front);
        state.enqueue_control_message(message, control_resource);
        state.active_lifecycle_transaction = None;
        self.finished = true;
        Ok(())
    }
}

impl Drop for PreparedObjectCreation {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if let Ok(mut state) = self.store.state.lock()
            && state.active_lifecycle_transaction == Some(self.transaction_id)
        {
            state.active_lifecycle_transaction = None;
        }
    }
}

impl PreparedObjectCreationBatch {
    pub fn objects(&self) -> &[ObjectCreationSpec] {
        &self.objects
    }

    pub fn commit(mut self) -> Result<(), RuntimeStateError> {
        let mut state = self
            .store
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.active_lifecycle_transaction != Some(self.transaction_id) {
            return Err(RuntimeStateError::LifecycleTransactionUnavailable);
        }
        if state.transport_closed {
            return Err(RuntimeStateError::TransportClosed);
        }

        let mut next_objects = state.objects.clone();
        for object in &self.objects {
            validate_object_parent_in_graph(
                &next_objects,
                self.object_kinds,
                self.topology,
                object.object_id,
                object.kind,
                object.parent,
            )?;
            next_objects.insert(object.object_id, object.parent, object.kind)?;
        }
        state.objects = next_objects;

        for (object, resource) in self.objects.iter().zip(self.object_resources.drain(..)) {
            let previous = state.object_resources.insert(object.object_id, resource);
            debug_assert!(previous.is_none());
        }
        for object in &self.objects {
            let message = ControlMessage::Create {
                object_id: object.object_id,
                parent: object.parent,
                kind: object.kind,
            };
            let control_resource = self
                .control_resources
                .as_mut()
                .and_then(VecDeque::pop_front);
            state.enqueue_control_message(message, control_resource);
        }
        state.active_lifecycle_transaction = None;
        self.finished = true;
        Ok(())
    }
}

impl Drop for PreparedObjectCreationBatch {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if let Ok(mut state) = self.store.state.lock()
            && state.active_lifecycle_transaction == Some(self.transaction_id)
        {
            state.active_lifecycle_transaction = None;
        }
    }
}

impl PreparedObjectDisposal {
    pub const fn object_id(&self) -> ObjectId {
        self.object_id
    }

    pub fn disposal_order(&self) -> &[ObjectId] {
        &self.disposal_order
    }

    /// Commits the reserved child-first disposal and terminates every call
    /// whose current lifecycle owner belongs to the removed subtree with ordinary
    /// parent-closed semantics.
    pub fn commit(self) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        self.commit_inner(None)
    }

    /// Commits disposal while preserving one shared, typed terminal diagnostic
    /// for every affected pending call. Backend cleanup still receives the
    /// ordinary parent-closed reason.
    pub fn commit_with_failure(
        self,
        failure: RuntimeCallFailure,
    ) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        self.commit_inner(Some(failure))
    }

    fn commit_inner(
        self,
        failure: Option<RuntimeCallFailure>,
    ) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        let mut committed = self.commit_parts::<false>(failure)?;
        committed.notify();
        Ok(committed.removed)
    }

    fn commit_parts<const DEFER: bool>(
        self,
        failure: Option<RuntimeCallFailure>,
    ) -> Result<CommittedDisposalParts, RuntimeStateError> {
        match self.commit_parts_with::<DEFER, (), Infallible>(failure, || Ok(()))? {
            Ok(((), parts)) => Ok(parts),
            Err(error) => match error {},
        }
    }

    fn commit_parts_with<const DEFER: bool, Output, PublicationError>(
        mut self,
        failure: Option<RuntimeCallFailure>,
        publish: impl FnOnce() -> Result<Output, PublicationError>,
    ) -> Result<Result<(Output, CommittedDisposalParts), PublicationError>, RuntimeStateError> {
        let (output, removed, signals, wakers) = {
            let mut state = self
                .store
                .state
                .lock()
                .map_err(|_| RuntimeStateError::StatePoisoned)?;
            if state.active_lifecycle_transaction != Some(self.transaction_id) {
                return Err(RuntimeStateError::LifecycleTransactionUnavailable);
            }
            if state.transport_closed {
                return Err(RuntimeStateError::TransportClosed);
            }

            let current_order = state.objects.subtree_ids_child_first(self.object_id);
            if current_order != self.disposal_order {
                return Err(RuntimeStateError::LifecycleTransactionUnavailable);
            }
            // The deferred path uses the already admitted order, avoiding a
            // second tree whose node allocations would need separate admission.
            let subtree =
                (!DEFER).then(|| self.disposal_order.iter().copied().collect::<BTreeSet<_>>());
            let affected_calls = state
                .pending_calls
                .iter()
                .filter(|call| {
                    let owner = state.call_lifecycle_owner(call);
                    match &subtree {
                        Some(subtree) => subtree.contains(&owner),
                        None => self.disposal_order.contains(&owner),
                    }
                })
                .map(|call| call.call_id())
                .collect::<Vec<_>>();
            let signals =
                affected_calls
                    .iter()
                    .map(|call_id| {
                        state.cancellation_signals.get(call_id).cloned().ok_or(
                            RuntimeStateError::MissingCancellationSignal { call_id: *call_id },
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;

            // The selecting composition already holds its native/topology and
            // route/Handle guards. Keep this runtime lock across publication so
            // transport close cannot invalidate disposal between local commits.
            // The callback must not reenter runtime, await, or invoke a waker.
            let output = match publish() {
                Ok(output) => output,
                Err(error) => {
                    drop(state);
                    return Ok(Err(error));
                }
            };
            let removed = state.objects.dispose_subtree(self.object_id);
            state.release_object_resources(&removed);
            let mut control_resources = self.control_resources.take();
            for object_id in &self.disposal_order {
                let resource = control_resources.as_mut().and_then(VecDeque::pop_front);
                state.enqueue_control_message(
                    ControlMessage::Dispose {
                        object_id: *object_id,
                    },
                    resource,
                );
            }
            let mut wakers = Vec::new();
            for (call_id, signal) in affected_calls.iter().zip(&signals) {
                let outcome = state
                    .pending_calls
                    .request_runtime_cancellation(*call_id, CancellationReason::ParentClosed)
                    .expect("preflight call remains present under the runtime state lock");
                if outcome == CancellationRequestOutcome::Requested
                    && let Some(failure) = failure.as_ref()
                {
                    let (installed, waker) = signal.install_failure(failure.clone());
                    debug_assert!(installed, "a newly cancelled call has no prior signal");
                    if let Some(waker) = waker {
                        wakers.push(waker);
                    }
                }
                if DEFER && failure.is_none() {
                    // Install termination before publishing the retired graph,
                    // but never invoke a user waker beneath enclosing locks.
                    let (_, waker) = signal.install_termination(CallTermination::Cancellation(
                        CancellationReason::ParentClosed,
                    ));
                    if let Some(waker) = waker {
                        wakers.push(waker);
                    }
                }
            }
            #[cfg(feature = "disposal-reservations")]
            {
                state.reserved_control_slots -= self.reserved_control_slots;
                self.reserved_control_slots = 0;
            }
            state.active_lifecycle_transaction = None;
            self.finished = true;
            let signals = if failure.is_none() && !DEFER {
                signals
            } else {
                Vec::new()
            };
            (output, removed, signals, wakers)
        };

        Ok(Ok((
            output,
            CommittedDisposalParts {
                removed,
                signals,
                wakers,
            },
        )))
    }
}

struct CommittedDisposalParts {
    removed: Vec<ObjectRecord<ObjectKindTag>>,
    signals: Vec<Arc<CancellationSignal>>,
    wakers: Vec<Waker>,
}
impl CommittedDisposalParts {
    fn notify(&mut self) {
        for signal in self.signals.drain(..) {
            signal.request(CancellationReason::ParentClosed);
        }
        for waker in self.wakers.drain(..) {
            waker.wake();
        }
    }
}

impl Drop for PreparedObjectDisposal {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let Ok(mut state) = self.store.state.lock() else {
            return;
        };
        #[cfg(feature = "disposal-reservations")]
        if self.reserved_control_slots != 0 {
            // Release EventItem leases before another operation can use
            // the queue slots that backed this uncommitted disposal.
            drop(self.control_resources.take());
        }
        if state.active_lifecycle_transaction == Some(self.transaction_id) {
            state.active_lifecycle_transaction = None;
        }
        #[cfg(feature = "disposal-reservations")]
        {
            state.reserved_control_slots -= self.reserved_control_slots;
            self.reserved_control_slots = 0;
        }
    }
}

const fn control_message_scope(message: ControlMessage) -> ResourceScope {
    match message {
        ControlMessage::Create { object_id, .. }
        | ControlMessage::Adopt { object_id, .. }
        | ControlMessage::Dispose { object_id }
        | ControlMessage::SetSubscription { object_id, .. } => ResourceScope::Object(object_id),
        ControlMessage::Cancel { call_id, .. } => ResourceScope::Call(call_id),
        ControlMessage::Close | ControlMessage::CloseAck => ResourceScope::Connection,
    }
}

impl TransportCloseOutcome {
    pub const fn already_closed(&self) -> bool {
        self.already_closed
    }

    pub const fn pending_calls(&self) -> usize {
        self.pending_calls
    }

    pub fn invalidated_objects(&self) -> &[ObjectRecord<ObjectKindTag>] {
        &self.invalidated_objects
    }
}

impl From<ObjectGraphError> for RuntimeStateError {
    fn from(error: ObjectGraphError) -> Self {
        Self::ObjectGraph(error)
    }
}

impl From<PendingCallError> for RuntimeStateError {
    fn from(error: PendingCallError) -> Self {
        Self::PendingCall(error)
    }
}

impl From<ResourceError> for RuntimeStateError {
    fn from(error: ResourceError) -> Self {
        Self::Resource(error)
    }
}

impl From<ProductionResourceError> for RuntimeStateError {
    fn from(error: ProductionResourceError) -> Self {
        Self::ProductionResource(error)
    }
}

impl From<ConnectionSchedulerError> for RuntimeStateError {
    fn from(error: ConnectionSchedulerError) -> Self {
        Self::ConnectionScheduler(error)
    }
}

struct RuntimeState {
    objects: ObjectGraph<ObjectKindTag>,
    object_resources: BTreeMap<ObjectId, ResourceLease>,
    call_resources: BTreeMap<CallId, ResourceLease>,
    control_resources: VecDeque<ResourceLease>,
    resource_ledger: Option<ResourceLedger>,
    control_messages: Option<BoundedQueue<ControlMessage>>,
    control_message_claimed: bool,
    #[cfg(feature = "disposal-reservations")]
    reserved_control_slots: usize,
    pending_calls: PendingCallTable,
    cancellation_signals: BTreeMap<CallId, Arc<CancellationSignal>>,
    backend_started: BTreeSet<CallId>,
    next_lifecycle_transaction_id: u64,
    active_lifecycle_transaction: Option<u64>,
    transport_closed: bool,
    // Queue clear/transport shutdown retains its backing allocation. Keep the
    // selected byte lease with the store, not with a short-lived close pool.
    #[cfg(feature = "disposal-reservations")]
    control_outbox_storage: Option<ResourceLease>,
}

impl RuntimeState {
    fn call_lifecycle_owner(&self, call: &pwright_core::PendingCall) -> ObjectId {
        #[cfg(feature = "lifetime-handoff")]
        if let Some(owner) = self
            .cancellation_signals
            .get(&call.call_id())
            .and_then(|signal| signal.lifetime_owner.get())
        {
            return *owner;
        }
        call.object_id()
    }

    fn new(limits: RuntimeLimits) -> Self {
        Self {
            objects: ObjectGraph::new(limits.live_objects),
            object_resources: BTreeMap::new(),
            call_resources: BTreeMap::new(),
            control_resources: VecDeque::new(),
            resource_ledger: None,
            control_messages: limits
                .control_messages
                .map(|capacity| BoundedQueue::new(capacity, OverflowPolicy::RejectNewest)),
            control_message_claimed: false,
            #[cfg(feature = "disposal-reservations")]
            reserved_control_slots: 0,
            pending_calls: PendingCallTable::new(limits.pending_calls),
            cancellation_signals: BTreeMap::new(),
            backend_started: BTreeSet::new(),
            next_lifecycle_transaction_id: 1,
            active_lifecycle_transaction: None,
            transport_closed: false,
            #[cfg(feature = "disposal-reservations")]
            control_outbox_storage: None,
        }
    }

    fn ensure_no_lifecycle_transaction(&self) -> Result<(), RuntimeStateError> {
        if self.active_lifecycle_transaction.is_some() {
            Err(RuntimeStateError::LifecycleTransactionInProgress)
        } else {
            Ok(())
        }
    }

    fn begin_lifecycle_transaction(&mut self) -> Result<u64, RuntimeStateError> {
        self.ensure_no_lifecycle_transaction()?;
        let transaction_id = self.next_lifecycle_transaction_id;
        self.next_lifecycle_transaction_id = transaction_id
            .checked_add(1)
            .ok_or(RuntimeStateError::LifecycleTransactionIdExhausted)?;
        self.active_lifecycle_transaction = Some(transaction_id);
        Ok(transaction_id)
    }

    fn ensure_control_capacity(&self, required: usize) -> Result<(), RuntimeStateError> {
        let Some(queue) = &self.control_messages else {
            return Ok(());
        };
        let remaining = queue.capacity().saturating_sub(queue.len());
        #[cfg(feature = "disposal-reservations")]
        let remaining = remaining.saturating_sub(self.reserved_control_slots);
        if required > remaining {
            return Err(RuntimeStateError::ControlMessageCapacityExceeded {
                required,
                remaining,
                limit: queue.capacity(),
            });
        }
        Ok(())
    }

    fn reserve_control_messages(
        &self,
        messages: &[ControlMessage],
    ) -> Result<Option<VecDeque<ResourceLease>>, RuntimeStateError> {
        if self.control_messages.is_none() || self.resource_ledger.is_none() {
            return Ok(None);
        }
        let ledger = self
            .resource_ledger
            .as_ref()
            .expect("budgeted control messages require an installed ledger");
        let mut resources = VecDeque::with_capacity(messages.len());
        for message in messages {
            resources.push_back(ledger.reserve(
                control_message_scope(*message),
                ResourceClass::EventItem,
                1,
            )?);
        }
        Ok(Some(resources))
    }

    fn enqueue_control_message(
        &mut self,
        message: ControlMessage,
        resource: Option<ResourceLease>,
    ) {
        let Some(queue) = &mut self.control_messages else {
            debug_assert!(resource.is_none());
            return;
        };
        match queue.push(message) {
            PushOutcome::Enqueued => {
                if self.resource_ledger.is_some() {
                    self.control_resources.push_back(
                        resource.expect("every budgeted control message owns one resource lease"),
                    );
                } else {
                    debug_assert!(resource.is_none());
                }
            }
            PushOutcome::DroppedOldest { .. } | PushOutcome::Rejected { .. } => {
                unreachable!("control capacity is admitted while holding the runtime state lock")
            }
        }
    }

    fn clear_control_messages(&mut self) {
        if let Some(queue) = &mut self.control_messages {
            queue.clear();
        }
        self.control_resources.clear();
    }

    fn release_object_resources(&mut self, removed: &[ObjectRecord<ObjectKindTag>]) {
        if self.resource_ledger.is_none() {
            debug_assert!(self.object_resources.is_empty());
            return;
        }
        for record in removed {
            let resource = self
                .object_resources
                .remove(&record.id())
                .expect("every budgeted live object owns one resource lease");
            drop(resource);
        }
    }

    fn release_call_resource(&mut self, call_id: CallId) {
        if self.resource_ledger.is_some() {
            let resource = self
                .call_resources
                .remove(&call_id)
                .expect("every budgeted pending call owns one resource lease");
            drop(resource);
        } else {
            debug_assert!(!self.call_resources.contains_key(&call_id));
        }
    }
}

struct RuntimeStore {
    state: Mutex<RuntimeState>,
}

impl RuntimeStore {
    fn new(limits: RuntimeLimits) -> Self {
        Self {
            state: Mutex::new(RuntimeState::new(limits)),
        }
    }

    fn install_resource_ledger(&self, ledger: ResourceLedger) -> Result<(), RuntimeStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        #[cfg(feature = "disposal-reservations")]
        if state.reserved_control_slots != 0 {
            return Err(RuntimeStateError::ResourceLedgerRequiresEmptyState);
        }
        if state.resource_ledger.is_some() {
            return Err(RuntimeStateError::ResourceLedgerAlreadyInstalled);
        }
        if !state.objects.is_empty()
            || !state.pending_calls.is_empty()
            || !state.cancellation_signals.is_empty()
            || !state.backend_started.is_empty()
            || !state.call_resources.is_empty()
            || !state.control_resources.is_empty()
            || state.transport_closed
        {
            return Err(RuntimeStateError::ResourceLedgerRequiresEmptyState);
        }
        state.resource_ledger = Some(ledger);
        Ok(())
    }

    fn register_object(
        &self,
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: ObjectKindTag,
    ) -> Result<(), RuntimeStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.transport_closed {
            return Err(RuntimeStateError::TransportClosed);
        }
        state.ensure_no_lifecycle_transaction()?;
        state.ensure_control_capacity(1)?;
        state.objects.validate_insert(object_id, parent)?;
        let object_resource = state
            .resource_ledger
            .as_ref()
            .map(|ledger| {
                ledger.reserve(
                    ResourceScope::Object(object_id),
                    ResourceClass::LiveObject,
                    1,
                )
            })
            .transpose()?;
        let message = ControlMessage::Create {
            object_id,
            parent,
            kind,
        };
        let mut control_resources = state.reserve_control_messages(&[message])?;
        let control_resource = control_resources.as_mut().and_then(VecDeque::pop_front);
        state.objects.insert(object_id, parent, kind)?;
        if let Some(resource) = object_resource {
            let previous = state.object_resources.insert(object_id, resource);
            debug_assert!(previous.is_none());
        }
        state.enqueue_control_message(message, control_resource);
        Ok(())
    }

    fn prepare_object_creation(
        self: &Arc<Self>,
        catalog: ObjectKindCatalog,
        topology: ObjectTopologyMode,
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: ObjectKindTag,
    ) -> Result<PreparedObjectCreation, RuntimeStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.transport_closed {
            return Err(RuntimeStateError::TransportClosed);
        }
        state.ensure_no_lifecycle_transaction()?;
        validate_object_parent_in_graph(
            &state.objects,
            catalog,
            topology,
            object_id,
            kind,
            parent,
        )?;
        state.objects.validate_insert(object_id, parent)?;
        state.ensure_control_capacity(1)?;
        let object_resource = state
            .resource_ledger
            .as_ref()
            .map(|ledger| {
                ledger.reserve(
                    ResourceScope::Object(object_id),
                    ResourceClass::LiveObject,
                    1,
                )
            })
            .transpose()?;
        let message = ControlMessage::Create {
            object_id,
            parent,
            kind,
        };
        let control_resources = state.reserve_control_messages(&[message])?;
        let transaction_id = state.begin_lifecycle_transaction()?;
        drop(state);
        Ok(PreparedObjectCreation {
            store: Arc::clone(self),
            transaction_id,
            object_id,
            parent,
            kind,
            object_kinds: catalog,
            topology,
            object_resource,
            control_resources,
            finished: false,
        })
    }

    fn prepare_object_creation_batch(
        self: &Arc<Self>,
        catalog: ObjectKindCatalog,
        topology: ObjectTopologyMode,
        objects: &[ObjectCreationSpec],
    ) -> Result<PreparedObjectCreationBatch, RuntimeStateError> {
        if objects.is_empty() {
            return Err(RuntimeStateError::EmptyObjectCreationBatch);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.transport_closed {
            return Err(RuntimeStateError::TransportClosed);
        }
        state.ensure_no_lifecycle_transaction()?;

        let mut candidate = state.objects.clone();
        // Do not reserve from the caller-provided slice length before the
        // bounded object graph has accepted each candidate. A hostile batch
        // can be arbitrarily large, while this vector must remain bounded by
        // the configured live-object capacity.
        let mut messages = Vec::new();
        for object in objects {
            validate_object_parent_in_graph(
                &candidate,
                catalog,
                topology,
                object.object_id,
                object.kind,
                object.parent,
            )?;
            candidate.insert(object.object_id, object.parent, object.kind)?;
            messages.push(ControlMessage::Create {
                object_id: object.object_id,
                parent: object.parent,
                kind: object.kind,
            });
        }
        state.ensure_control_capacity(objects.len())?;

        let mut object_resources = Vec::with_capacity(objects.len());
        if let Some(ledger) = state.resource_ledger.as_ref() {
            for object in objects {
                object_resources.push(ledger.reserve(
                    ResourceScope::Object(object.object_id),
                    ResourceClass::LiveObject,
                    1,
                )?);
            }
        }
        let control_resources = state.reserve_control_messages(&messages)?;
        let transaction_id = state.begin_lifecycle_transaction()?;
        drop(state);
        Ok(PreparedObjectCreationBatch {
            store: Arc::clone(self),
            transaction_id,
            objects: objects.to_vec(),
            object_kinds: catalog,
            topology,
            object_resources,
            control_resources,
            finished: false,
        })
    }

    fn apply_lifecycle_batch(
        &self,
        messages: &[ControlMessage],
        catalog: ObjectKindCatalog,
        topology: ObjectTopologyMode,
        cancel_pending: bool,
    ) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.transport_closed {
            return Err(RuntimeStateError::TransportClosed);
        }
        state.ensure_no_lifecycle_transaction()?;

        let mut next_objects = state.objects.clone();
        let mut expanded = Vec::new();
        let mut removed = Vec::new();
        let mut removed_existing = BTreeSet::new();
        let mut created_live = BTreeSet::new();
        let mut disposed_ids = BTreeSet::new();

        for message in messages {
            match *message {
                ControlMessage::Create {
                    object_id,
                    parent,
                    kind,
                } => {
                    validate_object_parent_in_graph(
                        &next_objects,
                        catalog,
                        topology,
                        object_id,
                        kind,
                        parent,
                    )?;
                    next_objects.insert(object_id, parent, kind)?;
                    created_live.insert(object_id);
                    expanded.push(*message);
                }
                ControlMessage::Dispose { object_id } => {
                    let subtree = next_objects.subtree_ids_child_first(object_id);
                    if !cancel_pending {
                        let pending_calls = state
                            .pending_calls
                            .iter()
                            .filter(|call| subtree.contains(&state.call_lifecycle_owner(call)))
                            .count();
                        if pending_calls > 0 {
                            return Err(RuntimeStateError::ObjectHasPendingCalls {
                                object_id,
                                pending_calls,
                            });
                        }
                    }
                    for record in next_objects.dispose_subtree(object_id) {
                        let removed_id = record.id();
                        disposed_ids.insert(removed_id);
                        if !created_live.remove(&removed_id) {
                            removed_existing.insert(removed_id);
                        }
                        expanded.push(ControlMessage::Dispose {
                            object_id: removed_id,
                        });
                        removed.push(record);
                    }
                }
                ControlMessage::Adopt { .. }
                | ControlMessage::Cancel { .. }
                | ControlMessage::SetSubscription { .. }
                | ControlMessage::Close
                | ControlMessage::CloseAck => {
                    return Err(RuntimeStateError::UnsupportedLifecycleMessage {
                        message: *message,
                    });
                }
            }
        }

        state.ensure_control_capacity(expanded.len())?;
        let affected_calls = state
            .pending_calls
            .iter()
            .filter(|call| disposed_ids.contains(&state.call_lifecycle_owner(call)))
            .map(|call| call.call_id())
            .collect::<Vec<_>>();
        let signals =
            if cancel_pending {
                affected_calls
                    .iter()
                    .map(|call_id| {
                        state.cancellation_signals.get(call_id).cloned().ok_or(
                            RuntimeStateError::MissingCancellationSignal { call_id: *call_id },
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                Vec::new()
            };

        let mut created_resources = BTreeMap::new();
        if let Some(ledger) = &state.resource_ledger {
            for object_id in &created_live {
                created_resources.insert(
                    *object_id,
                    ledger.reserve(
                        ResourceScope::Object(*object_id),
                        ResourceClass::LiveObject,
                        1,
                    )?,
                );
            }
        }
        let mut control_resources = state.reserve_control_messages(&expanded)?;

        state.objects = next_objects;
        if state.resource_ledger.is_some() {
            for object_id in removed_existing {
                let resource = state
                    .object_resources
                    .remove(&object_id)
                    .expect("every budgeted live object owns one resource lease");
                drop(resource);
            }
            for (object_id, resource) in created_resources {
                let previous = state.object_resources.insert(object_id, resource);
                debug_assert!(previous.is_none());
            }
        } else {
            debug_assert!(state.object_resources.is_empty());
            debug_assert!(created_resources.is_empty());
        }
        for message in expanded {
            let resource = control_resources.as_mut().and_then(VecDeque::pop_front);
            state.enqueue_control_message(message, resource);
        }
        if cancel_pending {
            for call_id in &affected_calls {
                state
                    .pending_calls
                    .request_runtime_cancellation(*call_id, CancellationReason::ParentClosed)?;
            }
        }
        drop(state);
        for signal in signals {
            signal.request(CancellationReason::ParentClosed);
        }
        Ok(removed)
    }

    fn object_kind(&self, object_id: ObjectId) -> Result<Option<ObjectKindTag>, RuntimeStateError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        Ok(state.objects.get(object_id).map(|record| *record.kind()))
    }

    fn adopt_object(
        &self,
        object_id: ObjectId,
        new_parent: ObjectId,
    ) -> Result<bool, RuntimeStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.transport_closed {
            return Err(RuntimeStateError::TransportClosed);
        }
        state.ensure_no_lifecycle_transaction()?;
        let previous_parent = state
            .objects
            .get(object_id)
            .ok_or(ObjectGraphError::MissingObject { object_id })?
            .parent();
        if previous_parent == Some(new_parent) {
            return Ok(false);
        }
        state.ensure_control_capacity(1)?;
        state.objects.validate_adopt(object_id, new_parent)?;
        let message = ControlMessage::Adopt {
            object_id,
            new_parent,
        };
        let mut control_resources = state.reserve_control_messages(&[message])?;
        let control_resource = control_resources.as_mut().and_then(VecDeque::pop_front);
        state.objects.adopt(object_id, new_parent)?;
        state.enqueue_control_message(message, control_resource);
        Ok(true)
    }

    fn pop_control_message(&self) -> Result<Option<ControlMessage>, RuntimeStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.control_message_claimed {
            return Err(RuntimeStateError::ControlMessageClaimInProgress);
        }
        let message = state.control_messages.as_mut().and_then(BoundedQueue::pop);
        if message.is_some() {
            if state.resource_ledger.is_some() {
                let resource = state
                    .control_resources
                    .pop_front()
                    .expect("every budgeted control message owns one resource lease");
                drop(resource);
            } else {
                debug_assert!(state.control_resources.is_empty());
            }
        }
        Ok(message)
    }

    fn claim_control_message(
        self: &Arc<Self>,
    ) -> Result<Option<ControlMessageClaim>, RuntimeStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.control_message_claimed {
            return Err(RuntimeStateError::ControlMessageClaimInProgress);
        }
        let Some(message) = state
            .control_messages
            .as_ref()
            .and_then(BoundedQueue::front)
            .copied()
        else {
            return Ok(None);
        };
        state.control_message_claimed = true;
        Ok(Some(ControlMessageClaim {
            store: Arc::clone(self),
            message,
            finished: false,
        }))
    }

    fn control_message_metrics(&self) -> Result<Option<QueueMetrics>, RuntimeStateError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        Ok(state.control_messages.as_ref().map(BoundedQueue::metrics))
    }

    fn close_transport_if_idle(&self) -> Result<TransportCloseOutcome, RuntimeStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.transport_closed {
            return Ok(TransportCloseOutcome {
                already_closed: true,
                pending_calls: 0,
                invalidated_objects: Vec::new(),
            });
        }
        if !state.pending_calls.is_empty() {
            return Err(RuntimeStateError::PendingCallsPreventTransportClose {
                pending_calls: state.pending_calls.len(),
            });
        }
        state.active_lifecycle_transaction = None;
        state.transport_closed = true;
        state.clear_control_messages();
        let invalidated_objects = state.objects.drain_child_first();
        state.release_object_resources(&invalidated_objects);
        Ok(TransportCloseOutcome {
            already_closed: false,
            pending_calls: 0,
            invalidated_objects,
        })
    }

    fn close_transport_and_cancel_calls(&self) -> Result<TransportCloseOutcome, RuntimeStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.transport_closed {
            return Ok(TransportCloseOutcome {
                already_closed: true,
                pending_calls: 0,
                invalidated_objects: Vec::new(),
            });
        }
        let affected_calls = state
            .pending_calls
            .iter()
            .map(|call| call.call_id())
            .collect::<Vec<_>>();
        let signals = affected_calls
            .iter()
            .map(|call_id| {
                state
                    .cancellation_signals
                    .get(call_id)
                    .cloned()
                    .ok_or(RuntimeStateError::MissingCancellationSignal { call_id: *call_id })
            })
            .collect::<Result<Vec<_>, _>>()?;

        state.active_lifecycle_transaction = None;
        state.transport_closed = true;
        state.clear_control_messages();
        let invalidated_objects = state.objects.drain_child_first();
        state.release_object_resources(&invalidated_objects);
        for call_id in &affected_calls {
            state
                .pending_calls
                .request_runtime_cancellation(*call_id, CancellationReason::TransportClosed)?;
        }
        drop(state);
        for signal in signals {
            signal.request(CancellationReason::TransportClosed);
        }
        Ok(TransportCloseOutcome {
            already_closed: false,
            pending_calls: affected_calls.len(),
            invalidated_objects,
        })
    }

    fn prepare_object_disposal(
        self: &Arc<Self>,
        object_id: ObjectId,
    ) -> Result<PreparedObjectDisposal, RuntimeStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.transport_closed {
            return Err(RuntimeStateError::TransportClosed);
        }
        state.ensure_no_lifecycle_transaction()?;
        let disposal_order = state.objects.subtree_ids_child_first(object_id);
        state.ensure_control_capacity(disposal_order.len())?;
        let messages = disposal_order
            .iter()
            .copied()
            .map(|object_id| ControlMessage::Dispose { object_id })
            .collect::<Vec<_>>();
        let control_resources = state.reserve_control_messages(&messages)?;
        let transaction_id = state.begin_lifecycle_transaction()?;
        drop(state);
        Ok(PreparedObjectDisposal {
            store: Arc::clone(self),
            transaction_id,
            object_id,
            disposal_order,
            control_resources,
            #[cfg(feature = "disposal-reservations")]
            reserved_control_slots: 0,
            finished: false,
        })
    }

    fn dispose_object_if_idle(
        &self,
        object_id: ObjectId,
    ) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        state.ensure_no_lifecycle_transaction()?;
        let disposal_order = state.objects.subtree_ids_child_first(object_id);
        let subtree = disposal_order.iter().copied().collect::<BTreeSet<_>>();
        let pending_calls = state
            .pending_calls
            .iter()
            .filter(|call| subtree.contains(&state.call_lifecycle_owner(call)))
            .count();
        if pending_calls > 0 {
            return Err(RuntimeStateError::ObjectHasPendingCalls {
                object_id,
                pending_calls,
            });
        }
        state.ensure_control_capacity(disposal_order.len())?;
        let messages = disposal_order
            .iter()
            .copied()
            .map(|object_id| ControlMessage::Dispose { object_id })
            .collect::<Vec<_>>();
        let mut control_resources = state.reserve_control_messages(&messages)?;
        let removed = state.objects.dispose_subtree(object_id);
        state.release_object_resources(&removed);
        for message in messages {
            let resource = control_resources.as_mut().and_then(VecDeque::pop_front);
            state.enqueue_control_message(message, resource);
        }
        Ok(removed)
    }

    fn dispose_object_and_cancel_calls(
        &self,
        object_id: ObjectId,
    ) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        state.ensure_no_lifecycle_transaction()?;
        let disposal_order = state.objects.subtree_ids_child_first(object_id);
        let subtree = disposal_order.iter().copied().collect::<BTreeSet<_>>();
        let affected_calls = state
            .pending_calls
            .iter()
            .filter(|call| subtree.contains(&state.call_lifecycle_owner(call)))
            .map(|call| call.call_id())
            .collect::<Vec<_>>();
        let signals = affected_calls
            .iter()
            .map(|call_id| {
                state
                    .cancellation_signals
                    .get(call_id)
                    .cloned()
                    .ok_or(RuntimeStateError::MissingCancellationSignal { call_id: *call_id })
            })
            .collect::<Result<Vec<_>, _>>()?;
        state.ensure_control_capacity(disposal_order.len())?;
        let messages = disposal_order
            .iter()
            .copied()
            .map(|object_id| ControlMessage::Dispose { object_id })
            .collect::<Vec<_>>();
        let mut control_resources = state.reserve_control_messages(&messages)?;
        let removed = state.objects.dispose_subtree(object_id);
        state.release_object_resources(&removed);
        for message in messages {
            let resource = control_resources.as_mut().and_then(VecDeque::pop_front);
            state.enqueue_control_message(message, resource);
        }
        for call_id in affected_calls {
            state
                .pending_calls
                .request_runtime_cancellation(call_id, CancellationReason::ParentClosed)?;
        }
        drop(state);
        for signal in signals {
            signal.request(CancellationReason::ParentClosed);
        }
        Ok(removed)
    }

    fn interrupt_subtree_calls(
        &self,
        object_id: ObjectId,
        failure: RuntimeCallFailure,
    ) -> Result<usize, RuntimeStateError> {
        let (interrupted, wakers) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| RuntimeStateError::StatePoisoned)?;
            if state.transport_closed {
                return Err(RuntimeStateError::TransportClosed);
            }
            state.ensure_no_lifecycle_transaction()?;
            if state.objects.get(object_id).is_none() {
                return Err(ObjectGraphError::MissingObject { object_id }.into());
            }
            let subtree = state
                .objects
                .subtree_ids_child_first(object_id)
                .into_iter()
                .collect::<BTreeSet<_>>();
            let affected_calls = state
                .pending_calls
                .iter()
                .filter(|call| subtree.contains(&state.call_lifecycle_owner(call)))
                .map(|call| call.call_id())
                .collect::<Vec<_>>();
            let mut interrupted = 0;
            let mut wakers = Vec::with_capacity(affected_calls.len());
            for call_id in affected_calls {
                let outcome = state
                    .pending_calls
                    .request_runtime_cancellation(call_id, CancellationReason::ParentClosed)?;
                if outcome == CancellationRequestOutcome::Requested {
                    let signal = state
                        .cancellation_signals
                        .get(&call_id)
                        .ok_or(RuntimeStateError::MissingCancellationSignal { call_id })?;
                    let (installed, waker) = signal.install_failure(failure.clone());
                    debug_assert!(installed, "a newly cancelled call has no prior signal");
                    interrupted += usize::from(installed);
                    if let Some(waker) = waker {
                        wakers.push(waker);
                    }
                }
            }
            (interrupted, wakers)
        };

        for waker in wakers {
            waker.wake();
        }
        Ok(interrupted)
    }

    fn ensure_subtree_idle(&self, object_id: ObjectId) -> Result<(), RuntimeStateError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.objects.get(object_id).is_none() {
            return Err(ObjectGraphError::MissingObject { object_id }.into());
        }
        let subtree = state.objects.subtree_ids_child_first(object_id);
        let pending_calls = state
            .pending_calls
            .iter()
            .filter(|call| subtree.contains(&state.call_lifecycle_owner(call)))
            .count();
        if pending_calls == 0 {
            Ok(())
        } else {
            Err(RuntimeStateError::ObjectHasPendingCalls {
                object_id,
                pending_calls,
            })
        }
    }

    fn live_object_count(&self) -> Result<usize, RuntimeStateError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?
            .objects
            .len())
    }

    fn is_object_live(&self, object_id: ObjectId) -> Result<bool, RuntimeStateError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?
            .objects
            .get(object_id)
            .is_some())
    }

    fn lifecycle_transaction_active(&self) -> Result<bool, RuntimeStateError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?
            .active_lifecycle_transaction
            .is_some())
    }

    fn pending_call_count(&self) -> Result<usize, RuntimeStateError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?
            .pending_calls
            .len())
    }

    fn begin_call<D>(
        self: &Arc<Self>,
        call_id: CallId,
        object_id: ObjectId,
        metadata: CallMetadata,
    ) -> Result<(PendingCallGuard, Arc<CancellationSignal>), OperationError<D>> {
        let mut state = self.state.lock().map_err(|_| state_poisoned_error())?;
        if state.transport_closed {
            return Err(OperationError::new(
                FailureKind::TransportClosed,
                "runtime transport is closed",
                None,
            ));
        }
        if state.objects.get(object_id).is_none() {
            return Err(OperationError::new(
                FailureKind::TargetClosed,
                format!("target object `{object_id}` is not live"),
                None,
            ));
        }
        if state.cancellation_signals.contains_key(&call_id) {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                format!("call `{call_id}` already has a cancellation signal"),
                None,
            ));
        }

        state
            .pending_calls
            .validate_insert(call_id)
            .map_err(map_pending_call_error)?;
        let resource = state
            .resource_ledger
            .as_ref()
            .map(|ledger| {
                ledger.reserve(ResourceScope::Call(call_id), ResourceClass::PendingCall, 1)
            })
            .transpose()
            .map_err(map_resource_error)?;
        #[cfg(feature = "lifetime-handoff")]
        let lifetime_storage = state
            .resource_ledger
            .as_ref()
            .map(|ledger| {
                ledger.reserve(
                    ResourceScope::Call(call_id),
                    ResourceClass::BackendStateBytes,
                    lifetime_handoff::SIGNAL_BYTES,
                )
            })
            .transpose()
            .map_err(map_resource_error)?;
        state
            .pending_calls
            .insert(call_id, object_id, metadata)
            .map_err(map_pending_call_error)?;
        let signal = Arc::new(CancellationSignal::new(
            #[cfg(feature = "lifetime-handoff")]
            lifetime_storage,
        ));
        state
            .cancellation_signals
            .insert(call_id, Arc::clone(&signal));
        if let Some(resource) = resource {
            let previous = state.call_resources.insert(call_id, resource);
            debug_assert!(previous.is_none());
        }
        drop(state);

        Ok((
            PendingCallGuard {
                call_id,
                store: Arc::clone(self),
                armed: true,
            },
            signal,
        ))
    }

    fn cancel_call(
        &self,
        call_id: CallId,
        cancellation: CancellationId,
        reason: CancellationReason,
    ) -> Result<CancellationRequestOutcome, RuntimeStateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        let outcome = state
            .pending_calls
            .request_cancellation(call_id, cancellation, reason)?;
        let signal = state
            .cancellation_signals
            .get(&call_id)
            .cloned()
            .ok_or(RuntimeStateError::MissingCancellationSignal { call_id })?;
        drop(state);

        if outcome == CancellationRequestOutcome::Requested {
            signal.request(reason);
        }
        Ok(outcome)
    }

    fn mark_backend_started<D>(&self, call_id: CallId) -> Result<(), OperationError<D>> {
        let mut state = self.state.lock().map_err(|_| state_poisoned_error())?;
        if state.pending_calls.get(call_id).is_none() {
            return Err(map_pending_call_error(PendingCallError::MissingCall {
                call_id,
            }));
        }
        if !state.backend_started.insert(call_id) {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                format!("call `{call_id}` backend operation already started"),
                None,
            ));
        }
        Ok(())
    }

    fn abandon_call(&self, call_id: CallId) -> Option<AbandonedCall> {
        let Ok(mut state) = self.state.lock() else {
            return None;
        };
        let call = state.pending_calls.complete(call_id).ok()?;
        state.cancellation_signals.remove(&call_id);
        let backend_started = state.backend_started.remove(&call_id);
        state.release_call_resource(call_id);
        let reason = match call.state() {
            PendingCallState::Running => CancellationReason::Caller,
            PendingCallState::CancellationRequested(reason) => reason,
        };
        Some(AbandonedCall {
            backend_started,
            reason,
            transport_closed: state.transport_closed,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AbandonedCall {
    backend_started: bool,
    reason: CancellationReason,
    transport_closed: bool,
}

fn validate_command_target<D>(
    store: &RuntimeStore,
    catalog: ObjectKindCatalog,
    object_id: ObjectId,
    targets: &[ObjectKindId],
) -> Result<(), OperationError<D>> {
    if targets.is_empty() {
        return Ok(());
    }
    let actual_tag = {
        let state = store.state.lock().map_err(|_| state_poisoned_error())?;
        state.objects.get(object_id).map(|record| *record.kind())
    };
    let Some(actual_tag) = actual_tag else {
        return Ok(());
    };
    let Some(actual) = catalog.by_tag(actual_tag) else {
        return Err(OperationError::new(
            FailureKind::UnsupportedCapability,
            format!(
                "command target validation has no object-kind descriptor for tag `{actual_tag}`"
            ),
            None,
        ));
    };
    if targets.contains(&actual.id) {
        return Ok(());
    }
    Err(OperationError::new(
        FailureKind::ProtocolViolation,
        format!(
            "object `{object_id}` has kind `{}`, which cannot receive this command",
            actual.id
        ),
        None,
    ))
}

fn validate_object_parent(
    store: &RuntimeStore,
    catalog: ObjectKindCatalog,
    topology: ObjectTopologyMode,
    object_id: ObjectId,
    kind: ObjectKindTag,
    parent: Option<ObjectId>,
) -> Result<(), RuntimeStateError> {
    let state = store
        .state
        .lock()
        .map_err(|_| RuntimeStateError::StatePoisoned)?;
    validate_object_parent_in_graph(&state.objects, catalog, topology, object_id, kind, parent)
}

fn validate_object_parent_in_graph(
    graph: &ObjectGraph<ObjectKindTag>,
    catalog: ObjectKindCatalog,
    topology: ObjectTopologyMode,
    object_id: ObjectId,
    kind: ObjectKindTag,
    parent: Option<ObjectId>,
) -> Result<(), RuntimeStateError> {
    if catalog.kinds().is_empty() {
        return Ok(());
    }
    let descriptor = catalog
        .by_tag(kind)
        .ok_or(RuntimeStateError::UnknownObjectKind { tag: kind })?;
    let Some(parent) = parent else {
        if topology == ObjectTopologyMode::Complete && !descriptor.allows_root {
            return Err(RuntimeStateError::MissingRequiredParent {
                object_id,
                kind: descriptor.id,
            });
        }
        return Ok(());
    };
    let parent_tag = graph
        .get(parent)
        .map(|record| *record.kind())
        .ok_or(ObjectGraphError::MissingParent { object_id, parent })?;
    let parent_descriptor = catalog
        .by_tag(parent_tag)
        .ok_or(RuntimeStateError::UnknownObjectKind { tag: parent_tag })?;
    if descriptor.allows_parent(parent_descriptor.id) {
        return Ok(());
    }
    Err(RuntimeStateError::InvalidParentKind {
        object_id,
        kind: descriptor.id,
        parent,
        parent_kind: parent_descriptor.id,
    })
}

/// Dispatches typed commands without timeout or cancellation machinery.
///
/// Commands carrying either execution-control field are rejected explicitly.
pub struct DirectDispatcher<S> {
    service: S,
    object_kinds: ObjectKindCatalog,
    topology: ObjectTopologyMode,
    store: Arc<RuntimeStore>,
    scheduler: TargetScheduler,
    connection: ConnectionScheduler,
    limits: RuntimeLimits,
}

impl<S> DirectDispatcher<S> {
    pub fn new(service: S, limits: RuntimeLimits) -> Self {
        Self {
            service,
            object_kinds: EMPTY_OBJECT_KIND_CATALOG,
            topology: ObjectTopologyMode::Partial,
            store: Arc::new(RuntimeStore::new(limits)),
            scheduler: TargetScheduler::new(TargetSchedulerLimits::new(
                limits.live_objects,
                limits.pending_calls,
            )),
            connection: ConnectionScheduler::new(ConnectionSchedulerLimits::new(
                limits.active_commands,
                limits.pending_calls,
            )),
            limits,
        }
    }

    pub const fn service(&self) -> &S {
        &self.service
    }

    /// Installs a connection-local resource ledger before lifecycle or call
    /// state is retained. Existing constructors remain unbudgeted for v1 and
    /// isolated in-process compatibility.
    pub fn with_resource_ledger(self, ledger: ResourceLedger) -> Result<Self, RuntimeStateError> {
        self.store.install_resource_ledger(ledger.clone())?;
        self.connection.install_resource_ledger(ledger)?;
        Ok(self)
    }

    /// Production-only construction path. The runtime profile must be present
    /// in the declared plan and fit its configured ledger before installation.
    pub fn with_production_resources(
        self,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, RuntimeStateError> {
        let ledger = resources.claim(self.limits.resource_requirements())?;
        self.with_resource_ledger(ledger)
    }

    pub fn with_object_kinds(
        service: S,
        limits: RuntimeLimits,
        object_kinds: ObjectKindCatalog,
    ) -> Result<Self, ObjectKindCatalogError> {
        Self::with_object_kinds_and_topology(
            service,
            limits,
            object_kinds,
            ObjectTopologyMode::Partial,
        )
    }

    pub fn with_object_kinds_and_topology(
        service: S,
        limits: RuntimeLimits,
        object_kinds: ObjectKindCatalog,
        topology: ObjectTopologyMode,
    ) -> Result<Self, ObjectKindCatalogError> {
        object_kinds.validate()?;
        let mut dispatcher = Self::new(service, limits);
        dispatcher.object_kinds = object_kinds;
        dispatcher.topology = topology;
        Ok(dispatcher)
    }

    pub fn register_object(
        &self,
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: ObjectKindTag,
    ) -> Result<(), RuntimeStateError> {
        validate_object_parent(
            &self.store,
            self.object_kinds,
            self.topology,
            object_id,
            kind,
            parent,
        )?;
        self.store.register_object(object_id, parent, kind)
    }

    /// Applies a Create/Dispose lifecycle batch atomically while requiring all
    /// affected command targets to be idle.
    pub fn apply_lifecycle_batch(
        &self,
        messages: &[ControlMessage],
    ) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        self.store
            .apply_lifecycle_batch(messages, self.object_kinds, self.topology, false)
    }

    /// Changes protocol ownership and emits one `Adopt` control message when
    /// lifecycle control is enabled. Re-adopting to the current parent is an
    /// idempotent no-op and returns `false`.
    pub fn adopt_object(
        &self,
        object_id: ObjectId,
        new_parent: ObjectId,
    ) -> Result<bool, RuntimeStateError> {
        let kind = self
            .store
            .object_kind(object_id)?
            .ok_or(ObjectGraphError::MissingObject { object_id })?;
        validate_object_parent(
            &self.store,
            self.object_kinds,
            self.topology,
            object_id,
            kind,
            Some(new_parent),
        )?;
        self.store.adopt_object(object_id, new_parent)
    }

    pub fn dispose_object(
        &self,
        object_id: ObjectId,
    ) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        self.store.dispose_object_if_idle(object_id)
    }

    /// Verifies that a live object subtree has no pending calls before a
    /// direct runtime applies a non-disposal lifecycle transition.
    pub fn ensure_subtree_idle(&self, object_id: ObjectId) -> Result<(), RuntimeStateError> {
        self.store.ensure_subtree_idle(object_id)
    }

    pub fn close_transport(&self) -> Result<TransportCloseOutcome, RuntimeStateError> {
        self.store.close_transport_if_idle()
    }

    /// Removes the oldest admitted lifecycle message, if this dispatcher was
    /// configured with `RuntimeLimits::with_control_messages`.
    pub fn pop_control_message(&self) -> Result<Option<ControlMessage>, RuntimeStateError> {
        self.store.pop_control_message()
    }

    /// Claims the oldest lifecycle message without removing it. Dropping the
    /// claim rolls it back; `ControlMessageClaim::commit` acknowledges it.
    pub fn claim_control_message(&self) -> Result<Option<ControlMessageClaim>, RuntimeStateError> {
        self.store.claim_control_message()
    }

    pub fn control_message_metrics(&self) -> Result<Option<QueueMetrics>, RuntimeStateError> {
        self.store.control_message_metrics()
    }

    pub fn live_object_count(&self) -> Result<usize, RuntimeStateError> {
        self.store.live_object_count()
    }

    pub fn is_object_live(&self, object_id: ObjectId) -> Result<bool, RuntimeStateError> {
        self.store.is_object_live(object_id)
    }

    pub fn pending_call_count(&self) -> Result<usize, RuntimeStateError> {
        self.store.pending_call_count()
    }

    pub fn scheduler_metrics(&self) -> Result<TargetSchedulerMetrics, TargetSchedulerError> {
        self.scheduler.metrics()
    }

    pub fn connection_metrics(
        &self,
    ) -> Result<ConnectionSchedulerMetrics, ConnectionSchedulerError> {
        self.connection.metrics()
    }
}

impl<C, S> Service<Command<C>> for DirectDispatcher<S>
where
    C: CommandSpec,
    C::Params: Send + 'static,
    C::Result: Send + 'static,
    C::ErrorDetails: Send + 'static,
    S: TargetedService<C::Params, Response = C::Result, Error = OperationError<C::ErrorDetails>>
        + Send
        + Sync,
{
    type Response = CommandResult<C>;
    type Error = Infallible;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, command: Command<C>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, params) = command.into_parts();
        Box::pin(async move {
            if metadata.timeout.is_some() || metadata.cancellation.is_some() {
                return Ok(CommandResult::new(
                    call_id,
                    Err(OperationError::new(
                        FailureKind::UnsupportedCapability,
                        "direct dispatcher has no deadline or cancellation controller",
                        None,
                    )),
                ));
            }

            if let Err(error) = validate_command_target::<C::ErrorDetails>(
                &self.store,
                self.object_kinds,
                object_id,
                C::TARGET_OBJECT_KINDS,
            ) {
                return Ok(CommandResult::new(call_id, Err(error)));
            }

            let (guard, _signal) = match self.store.begin_call(call_id, object_id, metadata) {
                Ok(call) => call,
                Err(error) => return Ok(CommandResult::new(call_id, Err(error))),
            };
            let admission = match self.scheduler.acquire(object_id, call_id) {
                Ok(admission) => admission,
                Err(error) => {
                    drop(guard);
                    return Ok(CommandResult::new(
                        call_id,
                        Err(map_target_scheduler_error(error)),
                    ));
                }
            };
            let _target_permit = match admission.await {
                Ok(permit) => permit,
                Err(error) => {
                    drop(guard);
                    return Ok(CommandResult::new(
                        call_id,
                        Err(map_target_scheduler_error(error)),
                    ));
                }
            };
            let connection_admission = match self.connection.acquire(call_id) {
                Ok(admission) => admission,
                Err(error) => {
                    drop(guard);
                    return Ok(CommandResult::new(
                        call_id,
                        Err(map_connection_scheduler_error(error)),
                    ));
                }
            };
            let _connection_permit = match connection_admission.await {
                Ok(permit) => permit,
                Err(error) => {
                    drop(guard);
                    return Ok(CommandResult::new(
                        call_id,
                        Err(map_connection_scheduler_error(error)),
                    ));
                }
            };
            let outcome = self.service.call_for(object_id, params).await;
            if let Err(error) = guard.complete() {
                return Ok(CommandResult::new(call_id, Err(error)));
            }
            Ok(CommandResult::new(call_id, outcome))
        })
    }
}

/// Dispatches typed commands with explicit cancellation and injected deadlines.
pub struct ControlledDispatcher<S, Cancellation, Deadlines, Cleanup = CleanupQueue> {
    service: S,
    object_kinds: ObjectKindCatalog,
    topology: ObjectTopologyMode,
    cancellation: Arc<Cancellation>,
    deadlines: Deadlines,
    store: Arc<RuntimeStore>,
    scheduler: TargetScheduler,
    connection: ConnectionScheduler,
    cleanup: Arc<Cleanup>,
    abandoned_cleanup: Arc<AbandonedCleanupState>,
    limits: RuntimeLimits,
}

impl<S, Cancellation, Deadlines> ControlledDispatcher<S, Cancellation, Deadlines, CleanupQueue> {
    pub fn new(
        service: S,
        cancellation: Cancellation,
        deadlines: Deadlines,
        limits: RuntimeLimits,
    ) -> Self {
        Self::with_cleanup_executor(
            service,
            cancellation,
            deadlines,
            limits,
            CleanupQueue::new(limits.pending_calls),
        )
    }

    pub fn with_object_kinds(
        service: S,
        cancellation: Cancellation,
        deadlines: Deadlines,
        limits: RuntimeLimits,
        object_kinds: ObjectKindCatalog,
    ) -> Result<Self, ObjectKindCatalogError> {
        Self::with_object_kinds_and_topology(
            service,
            cancellation,
            deadlines,
            limits,
            object_kinds,
            ObjectTopologyMode::Partial,
        )
    }

    pub fn with_object_kinds_and_topology(
        service: S,
        cancellation: Cancellation,
        deadlines: Deadlines,
        limits: RuntimeLimits,
        object_kinds: ObjectKindCatalog,
        topology: ObjectTopologyMode,
    ) -> Result<Self, ObjectKindCatalogError> {
        Self::with_cleanup_executor_and_object_kinds_and_topology(
            service,
            cancellation,
            deadlines,
            limits,
            CleanupQueue::new(limits.pending_calls),
            object_kinds,
            topology,
        )
    }
}

impl<S, Cancellation, Deadlines, Cleanup>
    ControlledDispatcher<S, Cancellation, Deadlines, Cleanup>
{
    pub const fn service(&self) -> &S {
        &self.service
    }

    pub const fn limits(&self) -> RuntimeLimits {
        self.limits
    }

    /// Installs a connection-local resource ledger before lifecycle or call
    /// state is retained.
    pub fn with_resource_ledger(self, ledger: ResourceLedger) -> Result<Self, RuntimeStateError> {
        self.store.install_resource_ledger(ledger.clone())?;
        self.connection.install_resource_ledger(ledger)?;
        Ok(self)
    }

    /// Production-only construction path. The runtime profile must be present
    /// in the declared plan and fit its configured ledger before installation.
    pub fn with_production_resources(
        self,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, RuntimeStateError> {
        let ledger = resources.claim(self.limits.resource_requirements())?;
        self.with_resource_ledger(ledger)
    }

    pub fn with_cleanup_executor(
        service: S,
        cancellation: Cancellation,
        deadlines: Deadlines,
        config: impl Into<ControlledRuntimeConfig>,
        cleanup: Cleanup,
    ) -> Self {
        let config = config.into();
        let limits = config.limits();
        Self {
            service,
            object_kinds: EMPTY_OBJECT_KIND_CATALOG,
            topology: ObjectTopologyMode::Partial,
            cancellation: Arc::new(cancellation),
            deadlines,
            store: Arc::new(RuntimeStore::new(limits)),
            scheduler: TargetScheduler::with_concurrency(
                TargetSchedulerLimits::new(limits.live_objects, limits.pending_calls),
                config.target_concurrency(),
            ),
            connection: ConnectionScheduler::new(ConnectionSchedulerLimits::new(
                limits.active_commands,
                limits.pending_calls,
            )),
            cleanup: Arc::new(cleanup),
            abandoned_cleanup: Arc::new(AbandonedCleanupState::default()),
            limits,
        }
    }

    pub fn with_cleanup_executor_and_object_kinds(
        service: S,
        cancellation: Cancellation,
        deadlines: Deadlines,
        limits: RuntimeLimits,
        cleanup: Cleanup,
        object_kinds: ObjectKindCatalog,
    ) -> Result<Self, ObjectKindCatalogError> {
        Self::with_cleanup_executor_and_object_kinds_and_topology(
            service,
            cancellation,
            deadlines,
            limits,
            cleanup,
            object_kinds,
            ObjectTopologyMode::Partial,
        )
    }

    pub fn with_cleanup_executor_and_object_kinds_and_topology(
        service: S,
        cancellation: Cancellation,
        deadlines: Deadlines,
        config: impl Into<ControlledRuntimeConfig>,
        cleanup: Cleanup,
        object_kinds: ObjectKindCatalog,
        topology: ObjectTopologyMode,
    ) -> Result<Self, ObjectKindCatalogError> {
        object_kinds.validate()?;
        let mut dispatcher =
            Self::with_cleanup_executor(service, cancellation, deadlines, config, cleanup);
        dispatcher.object_kinds = object_kinds;
        dispatcher.topology = topology;
        Ok(dispatcher)
    }

    pub fn register_object(
        &self,
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: ObjectKindTag,
    ) -> Result<(), RuntimeStateError> {
        validate_object_parent(
            &self.store,
            self.object_kinds,
            self.topology,
            object_id,
            kind,
            parent,
        )?;
        self.store.register_object(object_id, parent, kind)
    }

    /// Applies a Create/Dispose lifecycle batch atomically and requests
    /// cancellation for calls owned by disposed subtrees.
    pub fn apply_lifecycle_batch(
        &self,
        messages: &[ControlMessage],
    ) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        self.store
            .apply_lifecycle_batch(messages, self.object_kinds, self.topology, true)
    }

    /// Changes protocol ownership and emits one `Adopt` control message when
    /// lifecycle control is enabled. Re-adopting to the current parent is an
    /// idempotent no-op and returns `false`.
    pub fn adopt_object(
        &self,
        object_id: ObjectId,
        new_parent: ObjectId,
    ) -> Result<bool, RuntimeStateError> {
        let kind = self
            .store
            .object_kind(object_id)?
            .ok_or(ObjectGraphError::MissingObject { object_id })?;
        validate_object_parent(
            &self.store,
            self.object_kinds,
            self.topology,
            object_id,
            kind,
            Some(new_parent),
        )?;
        self.store.adopt_object(object_id, new_parent)
    }

    pub fn dispose_object(
        &self,
        object_id: ObjectId,
    ) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        self.store.dispose_object_and_cancel_calls(object_id)
    }

    /// Terminates pending calls in one live object subtree with a typed
    /// lifecycle failure while preserving the object graph.
    ///
    /// Only calls that have not already accepted another cancellation win
    /// this interruption. Active backend futures are dropped and receive the
    /// ordinary parent-closed cleanup signal, but their portable result keeps
    /// the supplied failure classification.
    pub fn interrupt_subtree_calls(
        &self,
        object_id: ObjectId,
        failure: RuntimeCallFailure,
    ) -> Result<usize, RuntimeStateError> {
        self.store.interrupt_subtree_calls(object_id, failure)
    }

    /// Reserves a new object's graph and control-message capacity without
    /// publishing it before an asynchronous backend creation succeeds.
    pub fn prepare_object_creation(
        &self,
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: ObjectKindTag,
    ) -> Result<PreparedObjectCreation, RuntimeStateError> {
        self.store.prepare_object_creation(
            self.object_kinds,
            self.topology,
            object_id,
            parent,
            kind,
        )
    }

    /// Reserves graph, live-object, and ordered control-message capacity for
    /// an all-or-nothing related object set.
    pub fn prepare_object_creation_batch(
        &self,
        objects: &[ObjectCreationSpec],
    ) -> Result<PreparedObjectCreationBatch, RuntimeStateError> {
        self.store
            .prepare_object_creation_batch(self.object_kinds, self.topology, objects)
    }

    /// Reserves a child-first disposal without changing liveness yet.
    ///
    /// The caller may perform one asynchronous backend close, then commit only
    /// after that close succeeds. Dropping the preparation preserves the live
    /// graph and releases all reserved control-message resources. This API is
    /// intentionally available only on the controlled dispatcher because
    /// commit may need to cancel calls admitted while the backend was closing.
    pub fn prepare_object_disposal(
        &self,
        object_id: ObjectId,
    ) -> Result<PreparedObjectDisposal, RuntimeStateError> {
        self.store.prepare_object_disposal(object_id)
    }

    /// Returns a cloneable lifecycle-only handle for separately composed
    /// control capabilities such as BrowserContext.close.
    pub fn lifecycle_handle(&self) -> ControlledLifecycleHandle {
        ControlledLifecycleHandle {
            store: Arc::clone(&self.store),
            object_kinds: self.object_kinds,
            topology: self.topology,
        }
    }

    pub fn lifecycle_transaction_active(&self) -> Result<bool, RuntimeStateError> {
        self.store.lifecycle_transaction_active()
    }

    pub fn close_transport(&self) -> Result<TransportCloseOutcome, RuntimeStateError> {
        self.store.close_transport_and_cancel_calls()
    }

    /// Removes the oldest admitted lifecycle message, if this dispatcher was
    /// configured with `RuntimeLimits::with_control_messages`.
    pub fn pop_control_message(&self) -> Result<Option<ControlMessage>, RuntimeStateError> {
        self.store.pop_control_message()
    }

    /// Claims the oldest lifecycle message without removing it. Dropping the
    /// claim rolls it back; `ControlMessageClaim::commit` acknowledges it.
    pub fn claim_control_message(&self) -> Result<Option<ControlMessageClaim>, RuntimeStateError> {
        self.store.claim_control_message()
    }

    pub fn control_message_metrics(&self) -> Result<Option<QueueMetrics>, RuntimeStateError> {
        self.store.control_message_metrics()
    }

    pub fn live_object_count(&self) -> Result<usize, RuntimeStateError> {
        self.store.live_object_count()
    }

    pub fn is_object_live(&self, object_id: ObjectId) -> Result<bool, RuntimeStateError> {
        self.store.is_object_live(object_id)
    }

    pub fn pending_call_count(&self) -> Result<usize, RuntimeStateError> {
        self.store.pending_call_count()
    }

    pub fn scheduler_metrics(&self) -> Result<TargetSchedulerMetrics, TargetSchedulerError> {
        self.scheduler.metrics()
    }

    pub fn connection_metrics(
        &self,
    ) -> Result<ConnectionSchedulerMetrics, ConnectionSchedulerError> {
        self.connection.metrics()
    }

    pub fn cleanup_executor(&self) -> &Cleanup {
        &self.cleanup
    }

    pub fn abandoned_cleanup_metrics(&self) -> AbandonedCleanupMetrics {
        self.abandoned_cleanup.metrics()
    }

    pub fn cancel_call(
        &self,
        call_id: CallId,
        cancellation: CancellationId,
        reason: CancellationReason,
    ) -> Result<CancellationRequestOutcome, RuntimeStateError> {
        self.store.cancel_call(call_id, cancellation, reason)
    }
}

impl<C, S, Cancellation, Deadlines, Cleanup> Service<Command<C>>
    for ControlledDispatcher<S, Cancellation, Deadlines, Cleanup>
where
    C: CommandSpec,
    C::Params: Send + 'static,
    C::Result: Send + 'static,
    C::ErrorDetails: Send + 'static,
    S: TargetedService<C::Params, Response = C::Result, Error = OperationError<C::ErrorDetails>>
        + Send
        + Sync,
    Cancellation: CommandCancellation<C> + Send + Sync + 'static,
    Deadlines: DeadlineScheduler,
    Cleanup: CleanupExecutor,
{
    type Response = CommandResult<C>;
    type Error = Infallible;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, command: Command<C>) -> Self::Future<'_> {
        self.dispatch_with(command, &self.service, Arc::clone(&self.cancellation))
    }
}

impl<S, Cancellation, Deadlines, Cleanup> ControlledDispatcher<S, Cancellation, Deadlines, Cleanup>
where
    S: Send + Sync,
    Cancellation: Send + Sync,
    Deadlines: DeadlineScheduler,
    Cleanup: CleanupExecutor,
{
    /// Dispatch an explicitly supplied typed command handler through this
    /// runtime's SAME object graph, pending-call table, target/connection
    /// schedulers, deadlines and cleanup policy. This does not create another
    /// runtime or register/advertise a wire capability. The caller separately
    /// installs and budgets the handler and its cancellation implementation.
    pub fn dispatch_with<'a, C, Handler, HandlerCancellation>(
        &'a self,
        command: Command<C>,
        handler: &'a Handler,
        handler_cancellation: Arc<HandlerCancellation>,
    ) -> Pin<Box<dyn Future<Output = Result<CommandResult<C>, Infallible>> + Send + 'a>>
    where
        C: CommandSpec,
        C::Params: Send + 'static,
        C::Result: Send + 'static,
        C::ErrorDetails: Send + 'static,
        Handler: TargetedService<
                C::Params,
                Response = C::Result,
                Error = OperationError<C::ErrorDetails>,
            > + Send
            + Sync,
        HandlerCancellation: CommandCancellation<C> + Send + Sync + 'static,
    {
        let (call_id, object_id, metadata, params) = command.into_parts();
        let context = OperationContext::new(call_id, object_id, C::METHOD, metadata.cancellation);
        let cleanup = Arc::clone(&self.cleanup);
        let cleanup: Arc<dyn CleanupExecutor> = cleanup;
        let cancellation_for_abandonment = Arc::clone(&handler_cancellation);
        let abandonment = AbandonedDispatch {
            store: Arc::clone(&self.store),
            cleanup,
            cleanup_metrics: Arc::clone(&self.abandoned_cleanup),
            context,
            job_factory: Some(Box::new(move |reason| {
                Box::pin(async move {
                    cancellation_for_abandonment
                        .cancel(context, reason)
                        .await
                        .map_err(|error| CleanupFailure::new(error.to_string()))
                })
            })),
        };
        let inner = Box::pin(async move {
            if let Err(error) = validate_command_target::<C::ErrorDetails>(
                &self.store,
                self.object_kinds,
                object_id,
                C::TARGET_OBJECT_KINDS,
            ) {
                return Ok(CommandResult::new(call_id, Err(error)));
            }

            let mut deadline = match metadata.timeout {
                Some(budget) => match self.deadlines.sleep(budget) {
                    Some(deadline) => Some(Box::pin(deadline)),
                    None => {
                        return Ok(CommandResult::new(
                            call_id,
                            Err(OperationError::new(
                                FailureKind::UnsupportedCapability,
                                "controlled dispatcher has no deadline scheduler",
                                None,
                            )),
                        ));
                    }
                },
                None => None,
            };

            let (guard, signal) = match self.store.begin_call(call_id, object_id, metadata) {
                Ok(call) => call,
                Err(error) => return Ok(CommandResult::new(call_id, Err(error))),
            };
            let mut cancellation = Box::pin(CancellationWait::new(signal));
            let mut admission = match self.scheduler.acquire(object_id, call_id) {
                Ok(admission) => Box::pin(admission),
                Err(error) => {
                    drop(guard);
                    return Ok(CommandResult::new(
                        call_id,
                        Err(map_target_scheduler_error(error)),
                    ));
                }
            };

            let admission_race = poll_fn(|task| {
                if let Poll::Ready(termination) = cancellation.as_mut().poll(task) {
                    return Poll::Ready(AdmissionRace::Termination(termination));
                }
                if let Some(deadline) = deadline.as_mut()
                    && deadline.as_mut().poll(task).is_ready()
                {
                    return Poll::Ready(AdmissionRace::Deadline);
                }
                if let Poll::Ready(permit) = admission.as_mut().poll(task) {
                    return Poll::Ready(AdmissionRace::Admitted(permit));
                }
                Poll::Pending
            })
            .await;

            let target_permit = match admission_race {
                AdmissionRace::Admitted(Ok(permit)) => permit,
                AdmissionRace::Admitted(Err(error)) => {
                    drop(guard);
                    return Ok(CommandResult::new(
                        call_id,
                        Err(map_target_scheduler_error(error)),
                    ));
                }
                AdmissionRace::Termination(termination) => {
                    let reason =
                        match guard.request_runtime_cancellation(termination.cleanup_reason()) {
                            Ok(reason) => reason,
                            Err(error) => return Ok(CommandResult::new(call_id, Err(error))),
                        };
                    drop(admission);
                    let outcome = if reason == termination.cleanup_reason() {
                        Err(termination.operation_error(metadata.timeout))
                    } else {
                        Err(cancellation_error(reason, metadata.timeout))
                    };
                    if let Err(error) = guard.complete() {
                        return Ok(CommandResult::new(call_id, Err(error)));
                    }
                    return Ok(CommandResult::new(call_id, outcome));
                }
                AdmissionRace::Deadline => {
                    let reason = match guard
                        .request_runtime_cancellation(CancellationReason::DeadlineExceeded)
                    {
                        Ok(reason) => reason,
                        Err(error) => return Ok(CommandResult::new(call_id, Err(error))),
                    };
                    drop(admission);
                    let outcome = Err(cancellation_error(reason, metadata.timeout));
                    if let Err(error) = guard.complete() {
                        return Ok(CommandResult::new(call_id, Err(error)));
                    }
                    return Ok(CommandResult::new(call_id, outcome));
                }
            };
            drop(admission);
            let mut connection_admission = match self.connection.acquire(call_id) {
                Ok(admission) => Box::pin(admission),
                Err(error) => {
                    drop(guard);
                    return Ok(CommandResult::new(
                        call_id,
                        Err(map_connection_scheduler_error(error)),
                    ));
                }
            };
            let connection_race = poll_fn(|task| {
                if let Poll::Ready(termination) = cancellation.as_mut().poll(task) {
                    return Poll::Ready(ConnectionAdmissionRace::Termination(termination));
                }
                if let Some(deadline) = deadline.as_mut()
                    && deadline.as_mut().poll(task).is_ready()
                {
                    return Poll::Ready(ConnectionAdmissionRace::Deadline);
                }
                if let Poll::Ready(permit) = connection_admission.as_mut().poll(task) {
                    return Poll::Ready(ConnectionAdmissionRace::Admitted(permit));
                }
                Poll::Pending
            })
            .await;
            let connection_permit = match connection_race {
                ConnectionAdmissionRace::Admitted(Ok(permit)) => permit,
                ConnectionAdmissionRace::Admitted(Err(error)) => {
                    drop(guard);
                    return Ok(CommandResult::new(
                        call_id,
                        Err(map_connection_scheduler_error(error)),
                    ));
                }
                ConnectionAdmissionRace::Termination(termination) => {
                    let reason =
                        match guard.request_runtime_cancellation(termination.cleanup_reason()) {
                            Ok(reason) => reason,
                            Err(error) => return Ok(CommandResult::new(call_id, Err(error))),
                        };
                    drop(connection_admission);
                    let outcome = if reason == termination.cleanup_reason() {
                        Err(termination.operation_error(metadata.timeout))
                    } else {
                        Err(cancellation_error(reason, metadata.timeout))
                    };
                    if let Err(error) = guard.complete() {
                        return Ok(CommandResult::new(call_id, Err(error)));
                    }
                    return Ok(CommandResult::new(call_id, outcome));
                }
                ConnectionAdmissionRace::Deadline => {
                    let reason = match guard
                        .request_runtime_cancellation(CancellationReason::DeadlineExceeded)
                    {
                        Ok(reason) => reason,
                        Err(error) => return Ok(CommandResult::new(call_id, Err(error))),
                    };
                    drop(connection_admission);
                    let outcome = Err(cancellation_error(reason, metadata.timeout));
                    if let Err(error) = guard.complete() {
                        return Ok(CommandResult::new(call_id, Err(error)));
                    }
                    return Ok(CommandResult::new(call_id, outcome));
                }
            };
            drop(connection_admission);
            if let Err(error) = self.store.mark_backend_started(call_id) {
                if let Err(cleanup_error) = guard.complete() {
                    return Ok(CommandResult::new(call_id, Err(cleanup_error)));
                }
                return Ok(CommandResult::new(call_id, Err(error)));
            }
            let mut operation = Box::pin(handler.call_for(object_id, params));

            let race = poll_fn(|task| {
                if let Some(termination) = cancellation.ready_termination()
                    && is_runtime_termination(&termination)
                {
                    return Poll::Ready(ExecutionRace::Termination(termination));
                }
                if let Poll::Ready(outcome) = operation.as_mut().poll(task) {
                    return Poll::Ready(ExecutionRace::Operation(outcome));
                }
                if let Poll::Ready(termination) = cancellation.as_mut().poll(task) {
                    return Poll::Ready(ExecutionRace::Termination(termination));
                }
                if let Some(deadline) = deadline.as_mut()
                    && deadline.as_mut().poll(task).is_ready()
                {
                    return Poll::Ready(ExecutionRace::Deadline);
                }
                Poll::Pending
            })
            .await;

            let outcome = match race {
                ExecutionRace::Operation(outcome) => outcome,
                ExecutionRace::Termination(termination) => {
                    let reason =
                        match guard.request_runtime_cancellation(termination.cleanup_reason()) {
                            Ok(reason) => reason,
                            Err(error) => return Ok(CommandResult::new(call_id, Err(error))),
                        };
                    drop(operation);
                    if reason == CancellationReason::TransportClosed {
                        Err(cancellation_error(reason, metadata.timeout))
                    } else {
                        match handler_cancellation.cancel(context, reason).await {
                            Ok(()) if reason == termination.cleanup_reason() => {
                                Err(handler_cancellation
                                    .terminal_error_after_cancel(context, reason, metadata.timeout)
                                    .unwrap_or_else(|| {
                                        termination.operation_error(metadata.timeout)
                                    }))
                            }
                            Ok(()) => Err(handler_cancellation
                                .terminal_error_after_cancel(context, reason, metadata.timeout)
                                .unwrap_or_else(|| cancellation_error(reason, metadata.timeout))),
                            Err(error) => Err(error.reclassify(FailureKind::Cleanup)),
                        }
                    }
                }
                ExecutionRace::Deadline => {
                    let reason = match guard
                        .request_runtime_cancellation(CancellationReason::DeadlineExceeded)
                    {
                        Ok(reason) => reason,
                        Err(error) => return Ok(CommandResult::new(call_id, Err(error))),
                    };
                    drop(operation);
                    match handler_cancellation.cancel(context, reason).await {
                        Ok(()) => Err(handler_cancellation
                            .terminal_error_after_cancel(context, reason, metadata.timeout)
                            .unwrap_or_else(|| cancellation_error(reason, metadata.timeout))),
                        Err(error) => Err(error.reclassify(FailureKind::Cleanup)),
                    }
                }
            };

            if let Err(error) = guard.complete() {
                return Ok(CommandResult::new(call_id, Err(error)));
            }
            drop(connection_permit);
            drop(target_permit);
            Ok(CommandResult::new(call_id, outcome))
        });
        Box::pin(ControlledDispatchFuture::new(inner, abandonment))
    }
}

enum AdmissionRace {
    Admitted(Result<TargetPermit, TargetSchedulerError>),
    Termination(CallTermination),
    Deadline,
}

enum ConnectionAdmissionRace {
    Admitted(Result<ConnectionPermit, ConnectionSchedulerError>),
    Termination(CallTermination),
    Deadline,
}

enum ExecutionRace<T, E> {
    Operation(Result<T, OperationError<E>>),
    Termination(CallTermination),
    Deadline,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AbandonedCleanupMetrics {
    pub attempted: u64,
    pub submitted: u64,
    pub rejected: u64,
}

#[derive(Default)]
struct AbandonedCleanupState {
    attempted: AtomicU64,
    submitted: AtomicU64,
    rejected: AtomicU64,
}

impl AbandonedCleanupState {
    fn metrics(&self) -> AbandonedCleanupMetrics {
        AbandonedCleanupMetrics {
            attempted: self.attempted.load(Ordering::Relaxed),
            submitted: self.submitted.load(Ordering::Relaxed),
            rejected: self.rejected.load(Ordering::Relaxed),
        }
    }
}

type DispatchOutput<C> = Result<CommandResult<C>, Infallible>;
type BoxedDispatchFuture<'a, C> = Pin<Box<dyn Future<Output = DispatchOutput<C>> + Send + 'a>>;

struct ControlledDispatchFuture<'a, C>
where
    C: CommandSpec,
{
    inner: Option<BoxedDispatchFuture<'a, C>>,
    abandonment: Option<AbandonedDispatch>,
}

impl<'a, C> ControlledDispatchFuture<'a, C>
where
    C: CommandSpec,
{
    fn new(inner: BoxedDispatchFuture<'a, C>, abandonment: AbandonedDispatch) -> Self {
        Self {
            inner: Some(inner),
            abandonment: Some(abandonment),
        }
    }
}

impl<C> Future for ControlledDispatchFuture<'_, C>
where
    C: CommandSpec,
{
    type Output = DispatchOutput<C>;

    fn poll(mut self: Pin<&mut Self>, task: &mut Context<'_>) -> Poll<Self::Output> {
        let outcome = self
            .inner
            .as_mut()
            .expect("completed dispatch future must not be polled again")
            .as_mut()
            .poll(task);
        if outcome.is_ready() {
            self.abandonment = None;
            self.inner = None;
        }
        outcome
    }
}

impl<C> Drop for ControlledDispatchFuture<'_, C>
where
    C: CommandSpec,
{
    fn drop(&mut self) {
        let Some(abandonment) = self.abandonment.take() else {
            return;
        };
        let abandoned = abandonment.prepare();
        drop(self.inner.take());
        if let Some(abandoned) = abandoned {
            abandonment.submit(abandoned);
        }
    }
}

struct AbandonedDispatch {
    store: Arc<RuntimeStore>,
    cleanup: Arc<dyn CleanupExecutor>,
    cleanup_metrics: Arc<AbandonedCleanupState>,
    context: OperationContext,
    job_factory: Option<Box<dyn FnOnce(CancellationReason) -> CleanupJob + Send + 'static>>,
}

impl AbandonedDispatch {
    fn prepare(&self) -> Option<AbandonedCall> {
        self.store.abandon_call(self.context.call_id())
    }

    fn submit(mut self, abandoned: AbandonedCall) {
        if !abandoned.backend_started
            || abandoned.transport_closed
            || abandoned.reason == CancellationReason::TransportClosed
        {
            return;
        }
        self.cleanup_metrics
            .attempted
            .fetch_add(1, Ordering::Relaxed);
        let job = self
            .job_factory
            .take()
            .expect("abandoned dispatch owns exactly one cleanup factory")(
            abandoned.reason
        );
        match self.cleanup.try_submit(job) {
            Ok(()) => {
                self.cleanup_metrics
                    .submitted
                    .fetch_add(1, Ordering::Relaxed);
            }
            Err(_) => {
                self.cleanup_metrics
                    .rejected
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

struct PendingCallGuard {
    call_id: CallId,
    store: Arc<RuntimeStore>,
    armed: bool,
}

impl PendingCallGuard {
    fn request_runtime_cancellation<D>(
        &self,
        proposed_reason: CancellationReason,
    ) -> Result<CancellationReason, OperationError<D>> {
        let mut state = self
            .store
            .state
            .lock()
            .map_err(|_| state_poisoned_error())?;
        let outcome = state
            .pending_calls
            .request_runtime_cancellation(self.call_id, proposed_reason)
            .map_err(map_pending_call_error)?;
        Ok(match outcome {
            CancellationRequestOutcome::Requested => proposed_reason,
            CancellationRequestOutcome::AlreadyRequested { original_reason } => original_reason,
        })
    }

    fn complete<D>(mut self) -> Result<(), OperationError<D>> {
        let mut state = self
            .store
            .state
            .lock()
            .map_err(|_| state_poisoned_error())?;
        state
            .pending_calls
            .complete(self.call_id)
            .map_err(map_pending_call_error)?;
        let missing_signal = state.cancellation_signals.remove(&self.call_id).is_none();
        state.backend_started.remove(&self.call_id);
        state.release_call_resource(self.call_id);
        self.armed = false;
        if missing_signal {
            return Err(OperationError::new(
                FailureKind::Cleanup,
                format!("call `{}` lost its cancellation signal", self.call_id),
                None,
            ));
        }
        Ok(())
    }
}

impl Drop for PendingCallGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Ok(mut state) = self.store.state.lock() {
            if state.pending_calls.complete(self.call_id).is_ok() {
                state.release_call_resource(self.call_id);
            }
            state.cancellation_signals.remove(&self.call_id);
            state.backend_started.remove(&self.call_id);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CallTermination {
    Cancellation(CancellationReason),
    Failure(RuntimeCallFailure),
}

impl CallTermination {
    const fn cleanup_reason(&self) -> CancellationReason {
        match self {
            Self::Cancellation(reason) => *reason,
            Self::Failure(_) => CancellationReason::ParentClosed,
        }
    }

    fn operation_error<D>(&self, timeout: Option<Duration>) -> OperationError<D> {
        match self {
            Self::Cancellation(reason) => cancellation_error(*reason, timeout),
            Self::Failure(failure) => failure.operation_error(),
        }
    }
}

struct CancellationSignal {
    #[cfg(feature = "lifetime-handoff")]
    _lifetime_storage: Option<ResourceLease>,
    #[cfg(feature = "lifetime-handoff")]
    lifetime_owner: OnceLock<ObjectId>,
    termination: OnceLock<CallTermination>,
    waker: Mutex<Option<Waker>>,
}

impl CancellationSignal {
    const fn new(
        #[cfg(feature = "lifetime-handoff")] lifetime_storage: Option<ResourceLease>,
    ) -> Self {
        Self {
            #[cfg(feature = "lifetime-handoff")]
            _lifetime_storage: lifetime_storage,
            #[cfg(feature = "lifetime-handoff")]
            lifetime_owner: OnceLock::new(),
            termination: OnceLock::new(),
            waker: Mutex::new(None),
        }
    }

    fn request(&self, reason: CancellationReason) {
        let (installed, waker) = self.install_termination(CallTermination::Cancellation(reason));
        if installed && let Some(waker) = waker {
            waker.wake();
        }
    }

    fn install_failure(&self, failure: RuntimeCallFailure) -> (bool, Option<Waker>) {
        self.install_termination(CallTermination::Failure(failure))
    }

    fn install_termination(&self, termination: CallTermination) -> (bool, Option<Waker>) {
        if self.termination.set(termination).is_err() {
            return (false, None);
        }
        let waker = self
            .waker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        (true, waker)
    }
}

struct CancellationWait {
    signal: Arc<CancellationSignal>,
}

impl CancellationWait {
    fn new(signal: Arc<CancellationSignal>) -> Self {
        Self { signal }
    }

    fn ready_termination(&self) -> Option<CallTermination> {
        self.signal.termination.get().cloned()
    }
}

impl Future for CancellationWait {
    type Output = CallTermination;

    fn poll(self: Pin<&mut Self>, task: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if let Some(termination) = self.signal.termination.get() {
            return Poll::Ready(termination.clone());
        }

        let mut waker = self
            .signal
            .waker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(termination) = self.signal.termination.get() {
            return Poll::Ready(termination.clone());
        }
        if waker
            .as_ref()
            .is_none_or(|registered| !registered.will_wake(task.waker()))
        {
            *waker = Some(task.waker().clone());
        }
        Poll::Pending
    }
}

fn cancellation_error<D>(
    reason: CancellationReason,
    timeout: Option<Duration>,
) -> OperationError<D> {
    let (kind, message) = match reason {
        CancellationReason::Caller => (FailureKind::Cancelled, "operation cancelled by caller"),
        CancellationReason::DeadlineExceeded => {
            return OperationError::new(
                FailureKind::Timeout,
                match timeout {
                    Some(timeout) => format!("operation exceeded its {timeout:?} timeout budget"),
                    None => "operation deadline was exceeded".to_string(),
                },
                None,
            );
        }
        CancellationReason::ParentClosed => {
            (FailureKind::TargetClosed, "operation target was closed")
        }
        CancellationReason::TransportClosed => (
            FailureKind::TransportClosed,
            "operation transport was closed",
        ),
    };
    OperationError::new(kind, message, None)
}

fn is_runtime_termination(termination: &CallTermination) -> bool {
    matches!(
        termination,
        CallTermination::Failure(_)
            | CallTermination::Cancellation(
                CancellationReason::ParentClosed | CancellationReason::TransportClosed
            )
    )
}

fn state_poisoned_error<D>() -> OperationError<D> {
    OperationError::new(
        FailureKind::Cleanup,
        "runtime state lock was poisoned",
        None,
    )
}

fn map_pending_call_error<D>(error: PendingCallError) -> OperationError<D> {
    let kind = match error {
        PendingCallError::CapacityExceeded { .. } => FailureKind::ResourceExhausted,
        PendingCallError::DuplicateCall { .. }
        | PendingCallError::MissingCall { .. }
        | PendingCallError::CancellationNotRegistered { .. }
        | PendingCallError::CancellationIdMismatch { .. } => FailureKind::ProtocolViolation,
    };
    OperationError::new(kind, error.to_string(), None)
}

fn map_resource_error<D>(error: ResourceError) -> OperationError<D> {
    let kind = match error {
        ResourceError::GlobalLimitExceeded { .. }
        | ResourceError::ScopeLimitExceeded { .. }
        | ResourceError::ScopeCapacityExceeded { .. }
        | ResourceError::LeaseCapacityExceeded { .. } => FailureKind::ResourceExhausted,
        ResourceError::StatePoisoned => FailureKind::Cleanup,
        ResourceError::EmptyRequest
        | ResourceError::ZeroUnits { .. }
        | ResourceError::DuplicateClass { .. }
        | ResourceError::TooManyClasses { .. }
        | ResourceError::ArithmeticOverflow { .. }
        | ResourceError::MissingLeaseClass { .. }
        | ResourceError::ReleasedLease
        | ResourceError::ClassIsNotBytes { .. }
        | ResourceError::ReservationTooSmall { .. } => FailureKind::ProtocolViolation,
    };
    OperationError::new(kind, error.to_string(), None)
}

fn map_target_scheduler_error<D>(error: TargetSchedulerError) -> OperationError<D> {
    let kind = match error {
        TargetSchedulerError::ActiveTargetCapacityExceeded { .. }
        | TargetSchedulerError::WaitingCallCapacityExceeded { .. } => {
            FailureKind::ResourceExhausted
        }
        TargetSchedulerError::DuplicateCall { .. }
        | TargetSchedulerError::MissingCall { .. }
        | TargetSchedulerError::InconsistentLane { .. } => FailureKind::ProtocolViolation,
        TargetSchedulerError::StatePoisoned => FailureKind::Cleanup,
    };
    OperationError::new(kind, error.to_string(), None)
}

fn map_connection_scheduler_error<D>(error: ConnectionSchedulerError) -> OperationError<D> {
    match error {
        ConnectionSchedulerError::Resource(error) => map_resource_error(error),
        error => {
            let kind = match &error {
                ConnectionSchedulerError::ActiveCommandCapacityExceeded { .. }
                | ConnectionSchedulerError::WaitingCallCapacityExceeded { .. } => {
                    FailureKind::ResourceExhausted
                }
                ConnectionSchedulerError::DuplicateCall { .. }
                | ConnectionSchedulerError::MissingCall { .. }
                | ConnectionSchedulerError::InconsistentState { .. }
                | ConnectionSchedulerError::ResourceLedgerAlreadyInstalled
                | ConnectionSchedulerError::ResourceLedgerRequiresEmptyState => {
                    FailureKind::ProtocolViolation
                }
                ConnectionSchedulerError::StatePoisoned => FailureKind::Cleanup,
                ConnectionSchedulerError::Resource(_) => {
                    unreachable!("resource error returned in the outer match")
                }
            };
            OperationError::new(kind, error.to_string(), None)
        }
    }
}

#[cfg(test)]
mod tests {
    mod concurrent_dispatch_tests;
    mod extension_dispatch_tests;
    use std::future::{Pending, Ready, pending, ready};
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Wake, Waker};
    use std::time::Duration;

    use pwright_core::{
        CallId, CallMetadata, CancellationId, CancellationReason, CancellationRequestOutcome,
        Command, CommandCancellation, CommandSpec, ControlMessage, FailureKind, MethodTag,
        ObjectId, ObjectKindCatalog, ObjectKindDescriptor, ObjectKindId, ObjectKindTag,
        OperationContext, OperationError, PendingCallError, ProductionResourceError,
        ProductionResourceLedger, ResourceCapacityBoundary, ResourceClass, ResourceError,
        ResourceLedger, ResourceLimits, ResourceRequirements, ResourceScope, Service,
    };

    use super::{
        AbandonedCleanupMetrics, CleanupQueue, CleanupQueueMetrics, ControlledDispatcher,
        DeadlineScheduler, DirectDispatcher, NoDeadlineScheduler, ObjectCreationSpec,
        ObjectTopologyMode, RuntimeCallFailure, RuntimeLimits, RuntimeStateError, TargetedService,
    };

    struct TestCommand;

    impl CommandSpec for TestCommand {
        const METHOD: MethodTag = MethodTag::new(1);
        type Params = usize;
        type Result = usize;
        type ErrorDetails = ();
    }

    const PAGE_KIND_ID: ObjectKindId = ObjectKindId::new("test.page");
    const PAGE_KIND: ObjectKindDescriptor =
        ObjectKindDescriptor::new(PAGE_KIND_ID, ObjectKindTag::new(3), "Page");
    const FRAME_KIND: ObjectKindDescriptor = ObjectKindDescriptor::new(
        ObjectKindId::new("test.frame"),
        ObjectKindTag::new(4),
        "Frame",
    );
    const TEST_OBJECT_KINDS: ObjectKindCatalog = ObjectKindCatalog::new(&[PAGE_KIND, FRAME_KIND]);

    const BROWSER_KIND_ID: ObjectKindId = ObjectKindId::new("test.browser");
    const CONTEXT_KIND_ID: ObjectKindId = ObjectKindId::new("test.context");
    const FRAME_KIND_ID: ObjectKindId = ObjectKindId::new("test.topology_frame");
    const TOPOLOGY_PAGE_KIND_ID: ObjectKindId = ObjectKindId::new("test.topology_page");
    const CONTEXT_PARENT_KINDS: &[ObjectKindId] = &[BROWSER_KIND_ID];
    const PAGE_PARENT_KINDS: &[ObjectKindId] = &[CONTEXT_KIND_ID];
    const FRAME_PARENT_KINDS: &[ObjectKindId] = &[FRAME_KIND_ID, TOPOLOGY_PAGE_KIND_ID];
    const BROWSER_KIND: ObjectKindDescriptor =
        ObjectKindDescriptor::new(BROWSER_KIND_ID, ObjectKindTag::new(11), "Browser");
    const CONTEXT_KIND: ObjectKindDescriptor =
        ObjectKindDescriptor::new(CONTEXT_KIND_ID, ObjectKindTag::new(12), "Context")
            .with_parent_kinds(CONTEXT_PARENT_KINDS);
    const TOPOLOGY_PAGE_KIND: ObjectKindDescriptor =
        ObjectKindDescriptor::new(TOPOLOGY_PAGE_KIND_ID, ObjectKindTag::new(13), "Page")
            .with_parent_kinds(PAGE_PARENT_KINDS);
    const TOPOLOGY_FRAME_KIND: ObjectKindDescriptor =
        ObjectKindDescriptor::new(FRAME_KIND_ID, ObjectKindTag::new(14), "Frame")
            .with_parent_kinds(FRAME_PARENT_KINDS);
    const TOPOLOGY_OBJECT_KINDS: ObjectKindCatalog = ObjectKindCatalog::new(&[
        BROWSER_KIND,
        CONTEXT_KIND,
        TOPOLOGY_PAGE_KIND,
        TOPOLOGY_FRAME_KIND,
    ]);

    struct PageCommand;

    impl CommandSpec for PageCommand {
        const METHOD: MethodTag = MethodTag::new(2);
        const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = &[PAGE_KIND_ID];
        type Params = usize;
        type Result = usize;
        type ErrorDetails = ();
    }

    struct ReadyService {
        calls: Arc<AtomicUsize>,
    }

    struct TargetRecordingService {
        targets: Arc<Mutex<Vec<ObjectId>>>,
    }

    impl TargetedService<usize> for TargetRecordingService {
        type Response = usize;
        type Error = OperationError<()>;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call_for(&self, object_id: ObjectId, request: usize) -> Self::Future<'_> {
            self.targets
                .lock()
                .expect("target records should be available")
                .push(object_id);
            ready(Ok(request))
        }
    }

    impl Service<usize> for ReadyService {
        type Response = usize;
        type Error = OperationError<()>;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, request: usize) -> Self::Future<'_> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ready(Ok(request + 1))
        }
    }

    struct PendingService;

    impl Service<usize> for PendingService {
        type Response = usize;
        type Error = OperationError<()>;
        type Future<'a> = Pending<Result<Self::Response, Self::Error>>;

        fn call(&self, _request: usize) -> Self::Future<'_> {
            pending()
        }
    }

    struct CountingPendingService {
        calls: Arc<AtomicUsize>,
    }

    impl Service<usize> for CountingPendingService {
        type Response = usize;
        type Error = OperationError<()>;
        type Future<'a> = Pending<Result<Self::Response, Self::Error>>;

        fn call(&self, _request: usize) -> Self::Future<'_> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            pending()
        }
    }

    struct DropAwarePendingService {
        drops: Arc<AtomicUsize>,
    }

    struct DropAwarePendingFuture {
        drops: Arc<AtomicUsize>,
    }

    impl Future for DropAwarePendingFuture {
        type Output = Result<usize, OperationError<()>>;

        fn poll(self: Pin<&mut Self>, _task: &mut Context<'_>) -> Poll<Self::Output> {
            Poll::Pending
        }
    }

    impl Drop for DropAwarePendingFuture {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl Service<usize> for DropAwarePendingService {
        type Response = usize;
        type Error = OperationError<()>;
        type Future<'a> = DropAwarePendingFuture;

        fn call(&self, _request: usize) -> Self::Future<'_> {
            DropAwarePendingFuture {
                drops: Arc::clone(&self.drops),
            }
        }
    }

    struct ManuallyCompletedService {
        ready: Arc<AtomicBool>,
        calls: Arc<AtomicUsize>,
    }

    struct ManuallyCompletedFuture {
        ready: Arc<AtomicBool>,
    }

    impl Future for ManuallyCompletedFuture {
        type Output = Result<usize, OperationError<()>>;

        fn poll(self: Pin<&mut Self>, _task: &mut Context<'_>) -> Poll<Self::Output> {
            if self.ready.load(Ordering::SeqCst) {
                Poll::Ready(Ok(42))
            } else {
                Poll::Pending
            }
        }
    }

    impl Service<usize> for ManuallyCompletedService {
        type Response = usize;
        type Error = OperationError<()>;
        type Future<'a> = ManuallyCompletedFuture;

        fn call(&self, _request: usize) -> Self::Future<'_> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ManuallyCompletedFuture {
                ready: Arc::clone(&self.ready),
            }
        }
    }

    struct RecordingCancellation {
        calls: Arc<Mutex<Vec<(OperationContext, CancellationReason, usize)>>>,
        operation_drops: Arc<AtomicUsize>,
        fail: bool,
    }

    impl CommandCancellation<TestCommand> for RecordingCancellation {
        type Future<'a> = Ready<Result<(), OperationError<()>>>;

        fn cancel(
            &self,
            context: OperationContext,
            reason: CancellationReason,
        ) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("cancellation records should be available")
                .push((context, reason, self.operation_drops.load(Ordering::SeqCst)));
            if self.fail {
                ready(Err(OperationError::new(
                    FailureKind::Backend,
                    "backend cancellation failed",
                    None,
                )))
            } else {
                ready(Ok(()))
            }
        }
    }

    struct EnrichedCancellation {
        cleanup_calls: Arc<AtomicUsize>,
    }

    impl CommandCancellation<TestCommand> for EnrichedCancellation {
        type Future<'a> = Ready<Result<(), OperationError<()>>>;

        fn cancel(
            &self,
            _context: OperationContext,
            _reason: CancellationReason,
        ) -> Self::Future<'_> {
            self.cleanup_calls.fetch_add(1, Ordering::SeqCst);
            ready(Ok(()))
        }

        fn terminal_error_after_cancel(
            &self,
            _context: OperationContext,
            reason: CancellationReason,
            _timeout: Option<Duration>,
        ) -> Option<OperationError<()>> {
            (reason == CancellationReason::DeadlineExceeded).then(|| {
                OperationError::new(FailureKind::Timeout, "bounded backend progress", Some(()))
            })
        }
    }

    struct ManualDeadlineScheduler {
        state: Arc<ManualDeadlineState>,
        budgets: Arc<Mutex<Vec<Duration>>>,
    }

    struct ManualDeadlineState {
        expired: AtomicBool,
        waker: Mutex<Option<Waker>>,
    }

    impl ManualDeadlineScheduler {
        fn new() -> Self {
            Self {
                state: Arc::new(ManualDeadlineState {
                    expired: AtomicBool::new(false),
                    waker: Mutex::new(None),
                }),
                budgets: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    struct ManualDeadlineFuture {
        state: Arc<ManualDeadlineState>,
    }

    impl Future for ManualDeadlineFuture {
        type Output = ();

        fn poll(self: Pin<&mut Self>, task: &mut Context<'_>) -> Poll<Self::Output> {
            if self.state.expired.load(Ordering::SeqCst) {
                return Poll::Ready(());
            }
            *self
                .state
                .waker
                .lock()
                .expect("manual deadline waker should be available") = Some(task.waker().clone());
            Poll::Pending
        }
    }

    impl DeadlineScheduler for ManualDeadlineScheduler {
        type Sleep<'a> = ManualDeadlineFuture;

        fn sleep(&self, budget: Duration) -> Option<Self::Sleep<'_>> {
            self.budgets
                .lock()
                .expect("deadline budgets should be available")
                .push(budget);
            Some(ManualDeadlineFuture {
                state: Arc::clone(&self.state),
            })
        }
    }

    struct CountingWake {
        wakes: AtomicUsize,
    }

    impl Wake for CountingWake {
        fn wake(self: Arc<Self>) {
            self.wakes.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn poll_once<F>(future: Pin<&mut F>) -> Poll<F::Output>
    where
        F: Future + ?Sized,
    {
        let mut context = Context::from_waker(Waker::noop());
        future.poll(&mut context)
    }

    fn poll_with_counting_waker<F>(future: Pin<&mut F>, wake: Arc<CountingWake>) -> Poll<F::Output>
    where
        F: Future,
    {
        let waker = Waker::from(wake);
        let mut context = Context::from_waker(&waker);
        future.poll(&mut context)
    }

    fn command(
        call_id: u64,
        object_id: u64,
        value: usize,
        metadata: CallMetadata,
    ) -> Command<TestCommand> {
        Command::new(
            CallId::new(call_id),
            ObjectId::new(object_id),
            metadata,
            value,
        )
    }

    fn page_command(call_id: u64, object_id: u64, value: usize) -> Command<PageCommand> {
        Command::new(
            CallId::new(call_id),
            ObjectId::new(object_id),
            CallMetadata::default(),
            value,
        )
    }

    fn register_page<S>(dispatcher: &DirectDispatcher<S>) {
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("target should register");
    }

    fn live_object_ledger(global: u64) -> ResourceLedger {
        ResourceLedger::new(
            ResourceLimits::deny_all(8, 8)
                .with_class(ResourceClass::LiveObject, global, 1)
                .with_class(ResourceClass::EventItem, 8, 8),
        )
    }

    fn controlled_lifecycle_dispatcher() -> (
        ControlledDispatcher<PendingService, (), NoDeadlineScheduler>,
        ResourceLedger,
    ) {
        let ledger = live_object_ledger(4);
        let dispatcher = ControlledDispatcher::new(
            PendingService,
            (),
            NoDeadlineScheduler,
            RuntimeLimits::new(4, 2).with_control_messages(6),
        )
        .with_resource_ledger(ledger.clone())
        .expect("lifecycle test ledger should install");
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("root should register");
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .expect("child should register");
        dispatcher
            .register_object(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                ObjectKindTag::new(3),
            )
            .expect("grandchild should register");
        for _ in 0..3 {
            dispatcher
                .pop_control_message()
                .expect("create event should pop")
                .expect("create event should exist");
        }
        (dispatcher, ledger)
    }

    fn controlled_creation_dispatcher() -> (
        ControlledDispatcher<PendingService, (), NoDeadlineScheduler>,
        ResourceLedger,
    ) {
        let ledger = live_object_ledger(4);
        let dispatcher = ControlledDispatcher::with_object_kinds_and_topology(
            PendingService,
            (),
            NoDeadlineScheduler,
            RuntimeLimits::new(4, 2).with_control_messages(4),
            TOPOLOGY_OBJECT_KINDS,
            ObjectTopologyMode::Complete,
        )
        .expect("topology catalog should be valid")
        .with_resource_ledger(ledger.clone())
        .expect("creation test ledger should install");
        dispatcher
            .register_object(ObjectId::new(1), None, BROWSER_KIND.tag)
            .expect("browser root should register");
        assert_eq!(
            dispatcher.pop_control_message(),
            Ok(Some(ControlMessage::Create {
                object_id: ObjectId::new(1),
                parent: None,
                kind: BROWSER_KIND.tag,
            }))
        );
        (dispatcher, ledger)
    }

    fn runtime_ledger(live_objects: u64, pending_calls: u64) -> ResourceLedger {
        runtime_ledger_with_active(live_objects, pending_calls, pending_calls)
    }

    fn runtime_ledger_with_active(
        live_objects: u64,
        pending_calls: u64,
        active_commands: u64,
    ) -> ResourceLedger {
        let limits = ResourceLimits::deny_all(16, 16)
            .with_class(ResourceClass::LiveObject, live_objects, 1)
            .with_class(ResourceClass::PendingCall, pending_calls, 1)
            .with_class(ResourceClass::ActiveCommand, active_commands, 1);
        #[cfg(feature = "lifetime-handoff")]
        let limits = limits.with_class(
            ResourceClass::BackendStateBytes,
            pending_calls * crate::lifetime_handoff::SIGNAL_BYTES,
            crate::lifetime_handoff::SIGNAL_BYTES,
        );
        ResourceLedger::new(limits)
    }

    fn limits_for_requirements(requirements: ResourceRequirements) -> ResourceLimits {
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            limits = limits.with_class(class, required.global, required.per_scope);
        }
        limits
    }

    #[test]
    fn lifecycle_control_is_opt_in() {
        // Arrange
        let dispatcher = DirectDispatcher::new(PendingService, RuntimeLimits::new(1, 1));

        // Act
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("object should register");

        // Assert
        assert_eq!(dispatcher.pop_control_message(), Ok(None));
        assert_eq!(dispatcher.control_message_metrics(), Ok(None));
    }

    #[test]
    fn budgeted_lifecycle_tracks_adoption_and_child_first_disposal() {
        // Arrange
        let ledger = live_object_ledger(4);
        let dispatcher = DirectDispatcher::new(PendingService, RuntimeLimits::new(4, 1))
            .with_resource_ledger(ledger.clone())
            .expect("ledger should install before lifecycle state");
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("first root should register");
        dispatcher
            .register_object(ObjectId::new(4), None, ObjectKindTag::new(1))
            .expect("second root should register");
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .expect("child should register");
        dispatcher
            .register_object(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                ObjectKindTag::new(3),
            )
            .expect("grandchild should register");
        dispatcher
            .adopt_object(ObjectId::new(2), ObjectId::new(4))
            .expect("subtree should move without changing object identity");

        // Act
        let removed = dispatcher
            .dispose_object(ObjectId::new(4))
            .expect("adopted subtree should dispose");
        let metrics = ledger.metrics().expect("metrics should be available");
        let surviving = ledger
            .scope_metrics(ResourceScope::Object(ObjectId::new(1)))
            .expect("scope metrics should be available")
            .expect("first root should remain live");

        // Assert
        assert_eq!(
            removed.iter().map(|record| record.id()).collect::<Vec<_>>(),
            vec![ObjectId::new(3), ObjectId::new(2), ObjectId::new(4)]
        );
        assert_eq!(metrics.class(ResourceClass::LiveObject).current, 1);
        assert_eq!(metrics.class(ResourceClass::LiveObject).peak, 4);
        assert_eq!(metrics.active_leases, 1);
        assert_eq!(surviving.units(ResourceClass::LiveObject), 1);
        for disposed in [2, 3, 4] {
            assert_eq!(
                ledger
                    .scope_metrics(ResourceScope::Object(ObjectId::new(disposed)))
                    .expect("disposed scope metrics should be available"),
                None
            );
        }
    }

    #[test]
    fn prepared_creation_reserves_resources_without_publishing_liveness() {
        // Arrange
        let (dispatcher, ledger) = controlled_creation_dispatcher();

        // Act
        let prepared = dispatcher
            .prepare_object_creation(ObjectId::new(2), Some(ObjectId::new(1)), CONTEXT_KIND.tag)
            .expect("context creation should prepare");

        // Assert
        assert_eq!(prepared.object_id(), ObjectId::new(2));
        assert_eq!(prepared.parent(), Some(ObjectId::new(1)));
        assert_eq!(prepared.kind(), CONTEXT_KIND.tag);
        assert_eq!(dispatcher.is_object_live(ObjectId::new(2)), Ok(false));
        assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(true));
        assert_eq!(dispatcher.pop_control_message(), Ok(None));
        let metrics = ledger.metrics().expect("resource metrics should load");
        assert_eq!(metrics.class(ResourceClass::LiveObject).current, 2);
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 1);
    }

    #[test]
    fn committing_prepared_creation_publishes_graph_and_create_together() {
        // Arrange
        let (dispatcher, ledger) = controlled_creation_dispatcher();
        let prepared = dispatcher
            .prepare_object_creation(ObjectId::new(2), Some(ObjectId::new(1)), CONTEXT_KIND.tag)
            .expect("context creation should prepare");

        // Act
        let commit = prepared.commit();

        // Assert
        assert_eq!(commit, Ok(()));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(2)), Ok(true));
        assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(false));
        assert_eq!(
            dispatcher.pop_control_message(),
            Ok(Some(ControlMessage::Create {
                object_id: ObjectId::new(2),
                parent: Some(ObjectId::new(1)),
                kind: CONTEXT_KIND.tag,
            }))
        );
        let metrics = ledger.metrics().expect("resource metrics should load");
        assert_eq!(metrics.class(ResourceClass::LiveObject).current, 2);
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 0);
    }

    #[test]
    fn dropping_prepared_creation_rolls_back_every_reservation() {
        // Arrange
        let (dispatcher, ledger) = controlled_creation_dispatcher();
        let prepared = dispatcher
            .prepare_object_creation(ObjectId::new(2), Some(ObjectId::new(1)), CONTEXT_KIND.tag)
            .expect("context creation should prepare");

        // Act
        drop(prepared);

        // Assert
        assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(false));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(2)), Ok(false));
        assert_eq!(dispatcher.pop_control_message(), Ok(None));
        let metrics = ledger.metrics().expect("resource metrics should load");
        assert_eq!(metrics.class(ResourceClass::LiveObject).current, 1);
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 0);
    }

    #[test]
    fn committing_prepared_creation_batch_publishes_page_then_main_frame_atomically() {
        // Arrange
        let (dispatcher, ledger) = controlled_creation_dispatcher();
        dispatcher
            .register_object(ObjectId::new(2), Some(ObjectId::new(1)), CONTEXT_KIND.tag)
            .expect("context should register");
        assert!(dispatcher.pop_control_message().unwrap().is_some());
        let prepared = dispatcher
            .prepare_object_creation_batch(&[
                ObjectCreationSpec::new(
                    ObjectId::new(3),
                    Some(ObjectId::new(2)),
                    TOPOLOGY_PAGE_KIND.tag,
                ),
                ObjectCreationSpec::new(
                    ObjectId::new(4),
                    Some(ObjectId::new(3)),
                    TOPOLOGY_FRAME_KIND.tag,
                ),
            ])
            .expect("page and main frame should reserve together");

        // Act
        let before = (
            dispatcher.is_object_live(ObjectId::new(3)),
            dispatcher.is_object_live(ObjectId::new(4)),
            dispatcher.pop_control_message(),
        );
        let commit = prepared.commit();
        let messages = [
            dispatcher.pop_control_message(),
            dispatcher.pop_control_message(),
        ];

        // Assert
        assert_eq!(before, (Ok(false), Ok(false), Ok(None)));
        assert_eq!(commit, Ok(()));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(3)), Ok(true));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(4)), Ok(true));
        assert_eq!(
            messages,
            [
                Ok(Some(ControlMessage::Create {
                    object_id: ObjectId::new(3),
                    parent: Some(ObjectId::new(2)),
                    kind: TOPOLOGY_PAGE_KIND.tag,
                })),
                Ok(Some(ControlMessage::Create {
                    object_id: ObjectId::new(4),
                    parent: Some(ObjectId::new(3)),
                    kind: TOPOLOGY_FRAME_KIND.tag,
                })),
            ]
        );
        assert_eq!(
            ledger
                .metrics()
                .expect("metrics should load")
                .class(ResourceClass::LiveObject)
                .current,
            4
        );
    }

    #[test]
    fn invalid_second_object_rolls_back_the_entire_creation_batch() {
        // Arrange
        let (dispatcher, ledger) = controlled_creation_dispatcher();
        let before = ledger.metrics().expect("metrics should load");

        // Act
        let result = dispatcher.prepare_object_creation_batch(&[
            ObjectCreationSpec::new(ObjectId::new(2), Some(ObjectId::new(1)), CONTEXT_KIND.tag),
            ObjectCreationSpec::new(
                ObjectId::new(3),
                Some(ObjectId::new(99)),
                TOPOLOGY_PAGE_KIND.tag,
            ),
        ]);
        let after = ledger.metrics().expect("metrics should load");

        // Assert
        assert!(result.is_err());
        assert_eq!(dispatcher.is_object_live(ObjectId::new(2)), Ok(false));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(3)), Ok(false));
        assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(false));
        assert_eq!(dispatcher.pop_control_message(), Ok(None));
        assert_eq!(before, after);
    }

    #[test]
    fn prepared_creation_excludes_competing_lifecycle_mutation() {
        // Arrange
        let (dispatcher, _ledger) = controlled_creation_dispatcher();
        let _prepared = dispatcher
            .prepare_object_creation(ObjectId::new(2), Some(ObjectId::new(1)), CONTEXT_KIND.tag)
            .expect("context creation should prepare");

        // Act
        let competing =
            dispatcher.register_object(ObjectId::new(3), Some(ObjectId::new(1)), CONTEXT_KIND.tag);

        // Assert
        assert_eq!(
            competing,
            Err(RuntimeStateError::LifecycleTransactionInProgress)
        );
        assert_eq!(dispatcher.is_object_live(ObjectId::new(2)), Ok(false));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(3)), Ok(false));
    }

    #[test]
    fn transport_close_invalidates_a_prepared_creation() {
        // Arrange
        let (dispatcher, ledger) = controlled_creation_dispatcher();
        let prepared = dispatcher
            .prepare_object_creation(ObjectId::new(2), Some(ObjectId::new(1)), CONTEXT_KIND.tag)
            .expect("context creation should prepare");

        // Act
        let close = dispatcher
            .close_transport()
            .expect("transport close should remain authoritative");
        let commit = prepared.commit();

        // Assert
        assert_eq!(close.invalidated_objects().len(), 1);
        assert_eq!(
            commit,
            Err(RuntimeStateError::LifecycleTransactionUnavailable)
        );
        assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(false));
        assert_eq!(dispatcher.live_object_count(), Ok(0));
        assert_eq!(
            ledger
                .metrics()
                .expect("resource metrics should load")
                .active_leases,
            0
        );
    }

    #[test]
    fn prepared_creation_rejects_invalid_topology_before_reserving_resources() {
        // Arrange
        let (dispatcher, ledger) = controlled_creation_dispatcher();

        // Act
        let error = match dispatcher.prepare_object_creation(
            ObjectId::new(3),
            Some(ObjectId::new(1)),
            TOPOLOGY_PAGE_KIND.tag,
        ) {
            Ok(_) => panic!("page cannot be created directly below browser"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(
            error,
            RuntimeStateError::InvalidParentKind {
                object_id: ObjectId::new(3),
                kind: TOPOLOGY_PAGE_KIND_ID,
                parent: ObjectId::new(1),
                parent_kind: BROWSER_KIND_ID,
            }
        );
        assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(false));
        let metrics = ledger.metrics().expect("resource metrics should load");
        assert_eq!(metrics.class(ResourceClass::LiveObject).current, 1);
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 0);
    }

    #[test]
    fn prepared_disposal_reserves_outbox_without_changing_liveness() {
        // Arrange
        let (dispatcher, ledger) = controlled_lifecycle_dispatcher();

        // Act
        let prepared = dispatcher
            .prepare_object_disposal(ObjectId::new(1))
            .expect("subtree disposal should prepare");

        // Assert
        assert_eq!(
            prepared.disposal_order(),
            &[ObjectId::new(3), ObjectId::new(2), ObjectId::new(1)]
        );
        assert_eq!(dispatcher.live_object_count(), Ok(3));
        assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(true));
        assert_eq!(
            dispatcher
                .control_message_metrics()
                .expect("outbox metrics should load")
                .expect("outbox should be enabled")
                .current_len,
            0
        );
        assert_eq!(
            ledger
                .metrics()
                .expect("resource metrics should load")
                .class(ResourceClass::EventItem)
                .current,
            3
        );
    }

    #[test]
    fn dropping_prepared_disposal_rolls_back_every_reservation() {
        // Arrange
        let (dispatcher, ledger) = controlled_lifecycle_dispatcher();
        let prepared = dispatcher
            .prepare_object_disposal(ObjectId::new(1))
            .expect("subtree disposal should prepare");

        // Act
        drop(prepared);

        // Assert
        assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(false));
        assert_eq!(dispatcher.live_object_count(), Ok(3));
        assert_eq!(
            ledger
                .metrics()
                .expect("resource metrics should load")
                .class(ResourceClass::EventItem)
                .current,
            0
        );
        assert_eq!(dispatcher.is_object_live(ObjectId::new(1)), Ok(true));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(3)), Ok(true));
    }

    #[test]
    fn prepared_disposal_excludes_competing_lifecycle_mutation() {
        // Arrange
        let (dispatcher, _ledger) = controlled_lifecycle_dispatcher();
        let _prepared = dispatcher
            .prepare_object_disposal(ObjectId::new(1))
            .expect("subtree disposal should prepare");

        // Act
        let competing = dispatcher.register_object(ObjectId::new(4), None, ObjectKindTag::new(1));

        // Assert
        assert_eq!(
            competing,
            Err(RuntimeStateError::LifecycleTransactionInProgress)
        );
        assert_eq!(dispatcher.live_object_count(), Ok(3));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(4)), Ok(false));
    }

    #[test]
    fn committing_prepared_disposal_is_child_first_and_releases_objects() {
        // Arrange
        let (dispatcher, ledger) = controlled_lifecycle_dispatcher();
        let prepared = dispatcher
            .prepare_object_disposal(ObjectId::new(1))
            .expect("subtree disposal should prepare");

        // Act
        let removed = prepared.commit().expect("prepared disposal should commit");

        // Assert
        assert_eq!(
            removed.iter().map(|record| record.id()).collect::<Vec<_>>(),
            vec![ObjectId::new(3), ObjectId::new(2), ObjectId::new(1)]
        );
        assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(false));
        assert_eq!(dispatcher.live_object_count(), Ok(0));
        assert_eq!(
            ledger
                .metrics()
                .expect("resource metrics should load")
                .class(ResourceClass::LiveObject)
                .current,
            0
        );
        assert_eq!(
            [3, 2, 1].map(|expected| {
                dispatcher
                    .pop_control_message()
                    .expect("dispose event should pop")
                    .expect("dispose event should exist")
                    == ControlMessage::Dispose {
                        object_id: ObjectId::new(expected),
                    }
            }),
            [true, true, true]
        );
    }

    #[test]
    fn prepared_disposal_cancels_calls_admitted_before_commit() {
        // Arrange
        let cancellations = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = ControlledDispatcher::new(
            PendingService,
            RecordingCancellation {
                calls: Arc::clone(&cancellations),
                operation_drops: Arc::new(AtomicUsize::new(0)),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(2, 1).with_control_messages(2),
        );
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("root should register");
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .expect("child should register");
        for _ in 0..2 {
            dispatcher
                .pop_control_message()
                .expect("create event should pop")
                .expect("create event should exist");
        }
        let mut call = Box::pin(dispatcher.call(command(8, 2, 1, CallMetadata::default())));
        assert!(poll_once(call.as_mut()).is_pending());
        let prepared = dispatcher
            .prepare_object_disposal(ObjectId::new(1))
            .expect("active subtree disposal should prepare");

        // Act
        let removed = prepared.commit().expect("disposal should commit");
        let result = match poll_once(call.as_mut()) {
            Poll::Ready(Ok(result)) => result,
            _ => panic!("cancelled call should terminate"),
        };

        // Assert
        assert_eq!(removed.len(), 2);
        assert_eq!(
            result
                .into_outcome()
                .expect_err("disposed target call should fail")
                .kind(),
            FailureKind::TargetClosed
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
        assert_eq!(
            cancellations
                .lock()
                .expect("cancellation records should be available")[0]
                .1,
            CancellationReason::ParentClosed
        );
    }

    #[test]
    fn transport_close_invalidates_a_prepared_disposal() {
        // Arrange
        let (dispatcher, ledger) = controlled_lifecycle_dispatcher();
        let prepared = dispatcher
            .prepare_object_disposal(ObjectId::new(1))
            .expect("subtree disposal should prepare");

        // Act
        let close = dispatcher
            .close_transport()
            .expect("transport close should remain authoritative");
        let commit = prepared.commit();

        // Assert
        assert_eq!(close.invalidated_objects().len(), 3);
        assert_eq!(
            commit,
            Err(RuntimeStateError::LifecycleTransactionUnavailable)
        );
        assert_eq!(dispatcher.lifecycle_transaction_active(), Ok(false));
        assert_eq!(dispatcher.live_object_count(), Ok(0));
        assert_eq!(
            ledger
                .metrics()
                .expect("resource metrics should load")
                .active_leases,
            0
        );
    }

    #[test]
    fn shared_live_object_limit_rejects_before_graph_and_outbox_mutation() {
        // Arrange
        let ledger = live_object_ledger(1);
        let dispatcher = DirectDispatcher::new(
            PendingService,
            RuntimeLimits::new(3, 1).with_control_messages(3),
        )
        .with_resource_ledger(ledger.clone())
        .expect("ledger should install before lifecycle state");
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("first object should fit");

        // Act
        let error = dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(1))
            .expect_err("second object should exceed the shared limit");
        let metrics = ledger.metrics().expect("metrics should be available");
        let outbox = dispatcher
            .control_message_metrics()
            .expect("control metrics should load")
            .expect("control outbox should be enabled");

        // Assert
        assert_eq!(
            error,
            RuntimeStateError::Resource(ResourceError::GlobalLimitExceeded {
                class: ResourceClass::LiveObject,
                current: 1,
                requested: 1,
                limit: 1,
            })
        );
        assert_eq!(dispatcher.live_object_count(), Ok(1));
        assert!(
            !dispatcher
                .is_object_live(ObjectId::new(2))
                .expect("object state should load")
        );
        assert_eq!(outbox.current_len, 1);
        assert_eq!(metrics.class(ResourceClass::LiveObject).current, 1);
        assert_eq!(metrics.rejected_reservations, 1);
    }

    #[test]
    fn budgeted_control_claim_retains_event_item_until_commit() {
        // Arrange
        let limits = RuntimeLimits::new(1, 1).with_control_messages(1);
        let requirements = limits.resource_requirements();
        let ledger = ResourceLedger::new(limits_for_requirements(requirements));
        let dispatcher = DirectDispatcher::new(PendingService, limits)
            .with_resource_ledger(ledger.clone())
            .expect("ledger should install before lifecycle state");
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("object and create event should fit");
        let before_claim = ledger.metrics().expect("resource metrics should load");
        let claim = dispatcher
            .claim_control_message()
            .expect("claim should load")
            .expect("create event should be queued");

        // Act
        drop(claim);
        let after_rollback = ledger.metrics().expect("resource metrics should load");
        dispatcher
            .claim_control_message()
            .expect("second claim should load")
            .expect("rolled-back event should remain")
            .commit()
            .expect("commit should consume the event");
        let after_commit = ledger.metrics().expect("resource metrics should load");

        // Assert
        assert_eq!(before_claim.class(ResourceClass::EventItem).current, 1);
        assert_eq!(after_rollback.class(ResourceClass::EventItem).current, 1);
        assert_eq!(after_commit.class(ResourceClass::EventItem).current, 0);
        assert_eq!(after_commit.class(ResourceClass::LiveObject).current, 1);
        assert_eq!(after_commit.active_leases, 1);
    }

    #[test]
    fn control_budget_rejects_dispose_batch_before_graph_mutation() {
        // Arrange
        let ledger = ResourceLedger::new(
            ResourceLimits::deny_all(4, 8)
                .with_class(ResourceClass::LiveObject, 2, 1)
                .with_class(ResourceClass::EventItem, 2, 1),
        );
        let dispatcher = DirectDispatcher::new(
            PendingService,
            RuntimeLimits::new(2, 1).with_control_messages(4),
        )
        .with_resource_ledger(ledger.clone())
        .expect("ledger should install before lifecycle state");
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("root create should fit");
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .expect("child create should fit");

        // Act
        let error = dispatcher
            .dispose_object(ObjectId::new(1))
            .expect_err("two retained create events should exhaust disposal events");
        let metrics = ledger.metrics().expect("resource metrics should load");
        let outbox = dispatcher
            .control_message_metrics()
            .expect("control metrics should load")
            .expect("control outbox should be enabled");

        // Assert
        assert_eq!(
            error,
            RuntimeStateError::Resource(ResourceError::GlobalLimitExceeded {
                class: ResourceClass::EventItem,
                current: 2,
                requested: 1,
                limit: 2,
            })
        );
        assert_eq!(dispatcher.live_object_count(), Ok(2));
        assert!(
            dispatcher
                .is_object_live(ObjectId::new(1))
                .expect("root state should load")
        );
        assert!(
            dispatcher
                .is_object_live(ObjectId::new(2))
                .expect("child state should load")
        );
        assert_eq!(outbox.current_len, 2);
        assert_eq!(metrics.class(ResourceClass::LiveObject).current, 2);
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 2);
        assert_eq!(metrics.rejected_reservations, 1);
    }

    #[test]
    fn production_runtime_claims_every_configured_retention_class() {
        // Arrange
        let runtime_limits = RuntimeLimits::new(2, 2)
            .with_active_commands(1)
            .with_control_messages(2);
        let requirements = runtime_limits.resource_requirements();
        let production =
            ProductionResourceLedger::new(limits_for_requirements(requirements), requirements)
                .expect("runtime plan should fit exact configured limits");
        let dispatcher = DirectDispatcher::new(PendingService, runtime_limits)
            .with_production_resources(&production)
            .expect("declared production runtime should compose");
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("page and create event should fit");
        let mut call = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));

        // Act
        assert!(poll_once(call.as_mut()).is_pending());
        let active = production.metrics().expect("resource metrics should load");
        drop(call);
        let after_call = production.metrics().expect("resource metrics should load");
        let create = dispatcher
            .pop_control_message()
            .expect("control pop should succeed");
        let released = production.metrics().expect("resource metrics should load");

        // Assert
        assert!(matches!(
            create,
            Some(ControlMessage::Create {
                object_id,
                parent: None,
                kind,
            }) if object_id == ObjectId::new(2) && kind == ObjectKindTag::new(3)
        ));
        assert_eq!(active.class(ResourceClass::LiveObject).current, 1);
        assert_eq!(active.class(ResourceClass::EventItem).current, 1);
        assert_eq!(active.class(ResourceClass::PendingCall).current, 1);
        assert_eq!(active.class(ResourceClass::ActiveCommand).current, 1);
        assert_eq!(after_call.class(ResourceClass::PendingCall).current, 0);
        assert_eq!(after_call.class(ResourceClass::ActiveCommand).current, 0);
        assert_eq!(released.class(ResourceClass::EventItem).current, 0);
        assert_eq!(released.class(ResourceClass::LiveObject).current, 1);
    }

    #[test]
    fn production_runtime_rejects_a_profile_omitted_from_the_plan() {
        // Arrange
        let runtime_limits = RuntimeLimits::new(1, 1);
        let incomplete_plan = ResourceRequirements::none()
            .with_class(ResourceClass::LiveObject, 1, 1)
            .with_metadata(2, 3);
        let configured = ResourceLimits::deny_all(2, 3)
            .with_class(ResourceClass::LiveObject, 1, 1)
            .with_class(ResourceClass::PendingCall, 1, 1)
            .with_class(ResourceClass::ActiveCommand, 1, 1);
        let production = ProductionResourceLedger::new(configured, incomplete_plan)
            .expect("configured limits should cover the deliberately incomplete plan");

        // Act
        let error = match DirectDispatcher::new(PendingService, runtime_limits)
            .with_production_resources(&production)
        {
            Err(error) => error,
            Ok(_) => panic!("runtime omitted from the plan must not compose"),
        };

        // Assert
        assert_eq!(
            error,
            RuntimeStateError::ProductionResource(ProductionResourceError::InsufficientGlobal {
                boundary: ResourceCapacityBoundary::DeclaredPlan,
                class: ResourceClass::PendingCall,
                required: 1,
                available: 0,
            })
        );
        assert_eq!(
            production
                .metrics()
                .expect("rejected composition should leave ledger pristine")
                .peak_leases,
            0
        );
    }

    #[test]
    fn lifecycle_validation_failures_do_not_touch_the_shared_budget() {
        // Arrange
        let ledger = live_object_ledger(3);
        let dispatcher = DirectDispatcher::new(PendingService, RuntimeLimits::new(3, 1))
            .with_resource_ledger(ledger.clone())
            .expect("ledger should install before lifecycle state");
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("root should register");

        // Act
        let duplicate = dispatcher.register_object(ObjectId::new(1), None, ObjectKindTag::new(2));
        let missing_parent = dispatcher.register_object(
            ObjectId::new(2),
            Some(ObjectId::new(9)),
            ObjectKindTag::new(2),
        );
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(
            duplicate,
            Err(RuntimeStateError::ObjectGraph(
                pwright_core::ObjectGraphError::DuplicateObject {
                    object_id: ObjectId::new(1),
                }
            ))
        );
        assert_eq!(
            missing_parent,
            Err(RuntimeStateError::ObjectGraph(
                pwright_core::ObjectGraphError::MissingParent {
                    object_id: ObjectId::new(2),
                    parent: ObjectId::new(9),
                }
            ))
        );
        assert_eq!(metrics.class(ResourceClass::LiveObject).current, 1);
        assert_eq!(metrics.class(ResourceClass::LiveObject).peak, 1);
        assert_eq!(metrics.rejected_reservations, 0);
    }

    #[test]
    fn outbox_backpressure_rejects_before_live_object_reservation() {
        // Arrange
        let ledger = live_object_ledger(1);
        let dispatcher = DirectDispatcher::new(
            PendingService,
            RuntimeLimits::new(1, 1).with_control_messages(0),
        )
        .with_resource_ledger(ledger.clone())
        .expect("ledger should install before lifecycle state");

        // Act
        let error = dispatcher.register_object(ObjectId::new(1), None, ObjectKindTag::new(1));
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(
            error,
            Err(RuntimeStateError::ControlMessageCapacityExceeded {
                required: 1,
                remaining: 0,
                limit: 0,
            })
        );
        assert_eq!(dispatcher.live_object_count(), Ok(0));
        assert_eq!(metrics.class(ResourceClass::LiveObject).current, 0);
        assert_eq!(metrics.class(ResourceClass::LiveObject).peak, 0);
        assert_eq!(metrics.rejected_reservations, 0);
    }

    #[test]
    fn transport_close_releases_every_live_object_lease() {
        // Arrange
        let ledger = live_object_ledger(2);
        let dispatcher = DirectDispatcher::new(PendingService, RuntimeLimits::new(2, 1))
            .with_resource_ledger(ledger.clone())
            .expect("ledger should install before lifecycle state");
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("root should register");
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .expect("child should register");

        // Act
        let outcome = dispatcher
            .close_transport()
            .expect("idle transport should close");
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(outcome.invalidated_objects().len(), 2);
        assert_eq!(dispatcher.live_object_count(), Ok(0));
        assert_eq!(metrics.class(ResourceClass::LiveObject).current, 0);
        assert_eq!(metrics.class(ResourceClass::LiveObject).peak, 2);
        assert_eq!(metrics.active_leases, 0);
        assert_eq!(metrics.live_scopes, 0);
    }

    #[test]
    fn resource_ledger_installation_requires_pristine_runtime_state() {
        // Arrange
        let ledger = live_object_ledger(1);
        let dispatcher = DirectDispatcher::new(PendingService, RuntimeLimits::new(1, 1));
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("unbudgeted object should register");

        // Act
        let error = match dispatcher.with_resource_ledger(ledger.clone()) {
            Err(error) => error,
            Ok(_) => panic!("late ledger installation must be rejected"),
        };
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(error, RuntimeStateError::ResourceLedgerRequiresEmptyState);
        assert_eq!(metrics.class(ResourceClass::LiveObject).current, 0);
        assert_eq!(metrics.class(ResourceClass::LiveObject).peak, 0);
    }

    #[test]
    fn lifecycle_control_reports_create_adopt_and_child_first_dispose() {
        // Arrange
        let dispatcher = DirectDispatcher::new(
            PendingService,
            RuntimeLimits::new(4, 1).with_control_messages(4),
        );
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("first root should register");
        dispatcher
            .register_object(ObjectId::new(4), None, ObjectKindTag::new(2))
            .expect("second root should register");
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .expect("child should register");
        dispatcher
            .register_object(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                ObjectKindTag::new(3),
            )
            .expect("grandchild should register");
        let created = (0..4)
            .map(|_| {
                dispatcher
                    .pop_control_message()
                    .expect("control queue should load")
                    .expect("create message should exist")
            })
            .collect::<Vec<_>>();

        // Act
        let adopted = dispatcher
            .adopt_object(ObjectId::new(2), ObjectId::new(4))
            .expect("child should be adopted");
        let adopt_message = dispatcher
            .pop_control_message()
            .expect("control queue should load");
        let disposed = dispatcher
            .dispose_object(ObjectId::new(4))
            .expect("adopted subtree should dispose");
        let disposal_messages = (0..3)
            .map(|_| {
                dispatcher
                    .pop_control_message()
                    .expect("control queue should load")
                    .expect("dispose message should exist")
            })
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(
            created,
            vec![
                ControlMessage::Create {
                    object_id: ObjectId::new(1),
                    parent: None,
                    kind: ObjectKindTag::new(1),
                },
                ControlMessage::Create {
                    object_id: ObjectId::new(4),
                    parent: None,
                    kind: ObjectKindTag::new(2),
                },
                ControlMessage::Create {
                    object_id: ObjectId::new(2),
                    parent: Some(ObjectId::new(1)),
                    kind: ObjectKindTag::new(2),
                },
                ControlMessage::Create {
                    object_id: ObjectId::new(3),
                    parent: Some(ObjectId::new(2)),
                    kind: ObjectKindTag::new(3),
                },
            ]
        );
        assert!(adopted);
        assert_eq!(
            adopt_message,
            Some(ControlMessage::Adopt {
                object_id: ObjectId::new(2),
                new_parent: ObjectId::new(4),
            })
        );
        assert_eq!(
            disposed
                .iter()
                .map(|record| record.id())
                .collect::<Vec<_>>(),
            vec![ObjectId::new(3), ObjectId::new(2), ObjectId::new(4)]
        );
        assert_eq!(
            disposal_messages,
            vec![
                ControlMessage::Dispose {
                    object_id: ObjectId::new(3),
                },
                ControlMessage::Dispose {
                    object_id: ObjectId::new(2),
                },
                ControlMessage::Dispose {
                    object_id: ObjectId::new(4),
                },
            ]
        );
        assert_eq!(dispatcher.live_object_count(), Ok(1));
    }

    #[test]
    fn lifecycle_control_backpressure_preserves_the_live_subtree() {
        // Arrange
        let dispatcher = DirectDispatcher::new(
            PendingService,
            RuntimeLimits::new(2, 1).with_control_messages(2),
        );
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("root should register");
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .expect("child should register");

        // Act
        let full_disposal = dispatcher.dispose_object(ObjectId::new(1));

        // Assert
        assert_eq!(
            full_disposal,
            Err(RuntimeStateError::ControlMessageCapacityExceeded {
                required: 2,
                remaining: 0,
                limit: 2,
            })
        );
        assert_eq!(dispatcher.live_object_count(), Ok(2));
        assert_eq!(
            dispatcher
                .control_message_metrics()
                .expect("metrics should load")
                .expect("control queue should be enabled")
                .current_len,
            2
        );
        assert!(dispatcher.pop_control_message().unwrap().is_some());
        assert!(dispatcher.pop_control_message().unwrap().is_some());
        let disposed = dispatcher
            .dispose_object(ObjectId::new(1))
            .expect("draining capacity should admit the whole subtree");
        assert_eq!(disposed.len(), 2);
    }

    #[test]
    fn lifecycle_control_rejects_create_before_graph_mutation() {
        // Arrange
        let dispatcher = DirectDispatcher::new(
            PendingService,
            RuntimeLimits::new(1, 1).with_control_messages(0),
        );

        // Act
        let registration =
            dispatcher.register_object(ObjectId::new(1), None, ObjectKindTag::new(1));

        // Assert
        assert_eq!(
            registration,
            Err(RuntimeStateError::ControlMessageCapacityExceeded {
                required: 1,
                remaining: 0,
                limit: 0,
            })
        );
        assert_eq!(dispatcher.live_object_count(), Ok(0));
    }

    #[test]
    fn lifecycle_control_rejects_changed_adopt_before_graph_mutation() {
        // Arrange
        let dispatcher = DirectDispatcher::new(
            PendingService,
            RuntimeLimits::new(3, 1).with_control_messages(3),
        );
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("first root should register");
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(2))
            .expect("second root should register");
        dispatcher
            .register_object(
                ObjectId::new(3),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(3),
            )
            .expect("child should register");

        // Act
        let adoption = dispatcher.adopt_object(ObjectId::new(3), ObjectId::new(2));

        // Assert
        assert_eq!(
            adoption,
            Err(RuntimeStateError::ControlMessageCapacityExceeded {
                required: 1,
                remaining: 0,
                limit: 3,
            })
        );
        for _ in 0..3 {
            assert!(
                dispatcher
                    .pop_control_message()
                    .expect("control queue should load")
                    .is_some()
            );
        }
        let disposed = dispatcher
            .dispose_object(ObjectId::new(1))
            .expect("original subtree should remain intact");
        assert_eq!(
            disposed
                .iter()
                .map(|record| record.id())
                .collect::<Vec<_>>(),
            vec![ObjectId::new(3), ObjectId::new(1)]
        );
        assert_eq!(dispatcher.live_object_count(), Ok(1));
    }

    #[test]
    fn transport_close_discards_unsent_lifecycle_messages() {
        // Arrange
        let dispatcher = DirectDispatcher::new(
            PendingService,
            RuntimeLimits::new(1, 1).with_control_messages(1),
        );
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("object should register");

        // Act
        dispatcher
            .close_transport()
            .expect("idle transport should close");

        // Assert
        assert_eq!(dispatcher.pop_control_message(), Ok(None));
        assert_eq!(
            dispatcher
                .control_message_metrics()
                .expect("metrics should load")
                .expect("control queue should be enabled")
                .current_len,
            0
        );
    }

    #[test]
    fn idempotent_adoption_does_not_require_an_outbox_slot() {
        // Arrange
        let dispatcher = DirectDispatcher::new(
            PendingService,
            RuntimeLimits::new(2, 1).with_control_messages(2),
        );
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("root should register");
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .expect("child should register");

        // Act
        let changed = dispatcher.adopt_object(ObjectId::new(2), ObjectId::new(1));

        // Assert
        assert_eq!(changed, Ok(false));
        assert_eq!(
            dispatcher
                .control_message_metrics()
                .expect("metrics should load")
                .expect("control queue should be enabled")
                .current_len,
            2
        );
    }

    #[test]
    fn lifecycle_claim_is_exclusive_and_rolls_back_when_dropped() {
        // Arrange
        let dispatcher = DirectDispatcher::new(
            PendingService,
            RuntimeLimits::new(1, 1).with_control_messages(1),
        );
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("object should register");
        let claim = dispatcher
            .claim_control_message()
            .expect("claim state should load")
            .expect("create message should be claimable");

        // Act
        let concurrent_claim = dispatcher.claim_control_message();
        let concurrent_pop = dispatcher.pop_control_message();
        drop(claim);
        let reclaimed = dispatcher
            .claim_control_message()
            .expect("claim state should load")
            .expect("dropped claim should release the message");
        let message = reclaimed.message();
        reclaimed.commit().expect("claim should commit");

        // Assert
        assert!(matches!(
            concurrent_claim,
            Err(RuntimeStateError::ControlMessageClaimInProgress)
        ));
        assert_eq!(
            concurrent_pop,
            Err(RuntimeStateError::ControlMessageClaimInProgress)
        );
        assert_eq!(
            message,
            ControlMessage::Create {
                object_id: ObjectId::new(1),
                parent: None,
                kind: ObjectKindTag::new(1),
            }
        );
        assert_eq!(dispatcher.pop_control_message(), Ok(None));
    }

    #[test]
    fn controlled_disposal_does_not_cancel_when_control_batch_is_not_admitted() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = ControlledDispatcher::new(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::clone(&operation_drops),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(2, 1).with_control_messages(1),
        );
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("root should register");
        assert!(
            dispatcher
                .pop_control_message()
                .expect("control queue should load")
                .is_some()
        );
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .expect("child should register");
        assert!(
            dispatcher
                .pop_control_message()
                .expect("control queue should load")
                .is_some()
        );
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        assert!(poll_once(future.as_mut()).is_pending());

        // Act
        let disposal = dispatcher.dispose_object(ObjectId::new(1));

        // Assert
        assert_eq!(
            disposal,
            Err(RuntimeStateError::ControlMessageCapacityExceeded {
                required: 2,
                remaining: 1,
                limit: 1,
            })
        );
        assert_eq!(dispatcher.live_object_count(), Ok(2));
        assert_eq!(dispatcher.pending_call_count(), Ok(1));
        assert!(
            cancellation_calls
                .lock()
                .expect("cancellation records should be available")
                .is_empty()
        );
        assert_eq!(operation_drops.load(Ordering::SeqCst), 0);
        assert!(poll_once(future.as_mut()).is_pending());
        drop(future);
    }

    #[test]
    fn dispatch_invokes_the_typed_service_and_correlates_the_result() {
        // Arrange
        let calls = Arc::new(AtomicUsize::new(0));
        let dispatcher = DirectDispatcher::new(
            ReadyService {
                calls: Arc::clone(&calls),
            },
            RuntimeLimits::new(1, 1),
        );
        register_page(&dispatcher);
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));

        // Act
        let outcome = poll_once(future.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("ready service should complete in one poll");
        };
        assert_eq!(result.call_id(), CallId::new(5));
        assert_eq!(result.into_outcome(), Ok(42));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn dispatch_forwards_the_validated_target_to_a_target_aware_service() {
        // Arrange
        let targets = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = DirectDispatcher::new(
            TargetRecordingService {
                targets: Arc::clone(&targets),
            },
            RuntimeLimits::new(1, 1),
        );
        register_page(&dispatcher);
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));

        // Act
        let outcome = poll_once(future.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("target-aware service should complete in one poll");
        };
        assert_eq!(result.into_outcome(), Ok(41));
        assert_eq!(
            *targets.lock().expect("target records should be available"),
            vec![ObjectId::new(2)]
        );
    }

    #[test]
    fn pending_call_budget_is_held_across_execution_and_released_on_drop() {
        // Arrange
        let ledger = runtime_ledger(1, 1);
        let dispatcher = DirectDispatcher::new(PendingService, RuntimeLimits::new(1, 2))
            .with_resource_ledger(ledger.clone())
            .expect("ledger should install before runtime state");
        register_page(&dispatcher);
        let mut first = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        assert!(poll_once(first.as_mut()).is_pending());

        // Act
        let while_pending = ledger
            .scope_metrics(ResourceScope::Call(CallId::new(5)))
            .expect("call metrics should be available")
            .expect("first call should retain its scope");
        let mut second = Box::pin(dispatcher.call(command(6, 2, 42, CallMetadata::default())));
        let second_outcome = poll_once(second.as_mut());
        drop(first);
        let after_drop = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(while_pending.units(ResourceClass::PendingCall), 1);
        let Poll::Ready(Ok(second_result)) = second_outcome else {
            panic!("shared pending-call limit should reject before scheduling");
        };
        assert_eq!(
            second_result
                .outcome()
                .as_ref()
                .expect_err("second call should be rejected")
                .kind(),
            FailureKind::ResourceExhausted
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
        assert_eq!(after_drop.class(ResourceClass::PendingCall).current, 0);
        assert_eq!(after_drop.class(ResourceClass::PendingCall).peak, 1);
        assert_eq!(after_drop.class(ResourceClass::LiveObject).current, 1);
    }

    #[test]
    fn completed_call_releases_pending_budget_but_keeps_target_live() {
        // Arrange
        let ledger = runtime_ledger(1, 1);
        let dispatcher = DirectDispatcher::new(
            ReadyService {
                calls: Arc::new(AtomicUsize::new(0)),
            },
            RuntimeLimits::new(1, 1),
        )
        .with_resource_ledger(ledger.clone())
        .expect("ledger should install before runtime state");
        register_page(&dispatcher);
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));

        // Act
        let outcome = poll_once(future.as_mut());
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("ready call should complete");
        };
        assert_eq!(result.into_outcome(), Ok(42));
        assert_eq!(metrics.class(ResourceClass::PendingCall).current, 0);
        assert_eq!(metrics.class(ResourceClass::PendingCall).peak, 1);
        assert_eq!(metrics.class(ResourceClass::LiveObject).current, 1);
        assert_eq!(metrics.active_leases, 1);
    }

    #[test]
    fn controlled_lifecycle_batch_keeps_pending_budget_until_cancelled_call_finishes() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let ledger = runtime_ledger(2, 1);
        let dispatcher = ControlledDispatcher::new(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            RecordingCancellation {
                calls: Arc::new(Mutex::new(Vec::new())),
                operation_drops: Arc::clone(&operation_drops),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(2, 1),
        )
        .with_resource_ledger(ledger.clone())
        .expect("ledger should install before runtime state");
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("root should register");
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(3),
            )
            .expect("target should register");
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        assert!(poll_once(future.as_mut()).is_pending());

        // Act
        dispatcher
            .apply_lifecycle_batch(&[ControlMessage::Dispose {
                object_id: ObjectId::new(1),
            }])
            .expect("parent close should request call cancellation");
        let after_dispose = ledger.metrics().expect("metrics should be available");
        let outcome = poll_once(future.as_mut());
        let after_completion = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(after_dispose.class(ResourceClass::LiveObject).current, 0);
        assert_eq!(after_dispose.class(ResourceClass::PendingCall).current, 1);
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("parent cancellation should complete the call");
        };
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("parent-closed operation should fail")
                .kind(),
            FailureKind::TargetClosed
        );
        assert_eq!(operation_drops.load(Ordering::SeqCst), 1);
        assert_eq!(
            after_completion.class(ResourceClass::PendingCall).current,
            0
        );
        assert_eq!(after_completion.active_leases, 0);
    }

    #[test]
    fn dispatch_rejects_a_live_object_of_the_wrong_semantic_kind() {
        // Arrange
        let calls = Arc::new(AtomicUsize::new(0));
        let dispatcher = DirectDispatcher::with_object_kinds(
            ReadyService {
                calls: Arc::clone(&calls),
            },
            RuntimeLimits::new(1, 1),
            TEST_OBJECT_KINDS,
        )
        .expect("catalog should be valid");
        dispatcher
            .register_object(ObjectId::new(2), None, FRAME_KIND.tag)
            .expect("frame should register");
        let mut future = Box::pin(dispatcher.call(page_command(5, 2, 41)));

        // Act
        let outcome = poll_once(future.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("target mismatch should complete immediately");
        };
        let error = result
            .outcome()
            .as_ref()
            .expect_err("frame cannot receive page command");
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn configured_runtime_rejects_an_unknown_object_kind_on_registration() {
        // Arrange
        let dispatcher = DirectDispatcher::with_object_kinds(
            PendingService,
            RuntimeLimits::new(1, 1),
            TEST_OBJECT_KINDS,
        )
        .expect("catalog should be valid");

        // Act
        let error = dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(99))
            .expect_err("unknown kind must fail");

        // Assert
        assert_eq!(
            error,
            RuntimeStateError::UnknownObjectKind {
                tag: ObjectKindTag::new(99),
            }
        );
        assert_eq!(dispatcher.live_object_count(), Ok(0));
    }

    #[test]
    fn partial_topology_accepts_an_attached_page_as_a_local_root() {
        // Arrange
        let dispatcher = DirectDispatcher::with_object_kinds(
            PendingService,
            RuntimeLimits::new(1, 1),
            TOPOLOGY_OBJECT_KINDS,
        )
        .expect("catalog should be valid");

        // Act
        let result = dispatcher.register_object(ObjectId::new(3), None, TOPOLOGY_PAGE_KIND.tag);

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(dispatcher.live_object_count(), Ok(1));
    }

    #[test]
    fn complete_topology_requires_roots_and_validates_the_full_browser_chain() {
        // Arrange
        let dispatcher = DirectDispatcher::with_object_kinds_and_topology(
            PendingService,
            RuntimeLimits::new(4, 1),
            TOPOLOGY_OBJECT_KINDS,
            ObjectTopologyMode::Complete,
        )
        .expect("catalog should be valid");

        // Act
        let missing_parent = dispatcher
            .register_object(ObjectId::new(3), None, TOPOLOGY_PAGE_KIND.tag)
            .expect_err("a complete page needs its context");
        dispatcher
            .register_object(ObjectId::new(1), None, BROWSER_KIND.tag)
            .expect("browser root should register");
        dispatcher
            .register_object(ObjectId::new(2), Some(ObjectId::new(1)), CONTEXT_KIND.tag)
            .expect("context should register under browser");
        dispatcher
            .register_object(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                TOPOLOGY_PAGE_KIND.tag,
            )
            .expect("page should register under context");
        let frame = dispatcher.register_object(
            ObjectId::new(4),
            Some(ObjectId::new(3)),
            TOPOLOGY_FRAME_KIND.tag,
        );

        // Assert
        assert_eq!(
            missing_parent,
            RuntimeStateError::MissingRequiredParent {
                object_id: ObjectId::new(3),
                kind: TOPOLOGY_PAGE_KIND_ID,
            }
        );
        assert_eq!(frame, Ok(()));
        assert_eq!(dispatcher.live_object_count(), Ok(4));
    }

    #[test]
    fn lifecycle_batch_rejection_leaves_graph_and_outbox_unchanged() {
        // Arrange
        let dispatcher = DirectDispatcher::with_object_kinds_and_topology(
            PendingService,
            RuntimeLimits::new(4, 1).with_control_messages(1),
            TOPOLOGY_OBJECT_KINDS,
            ObjectTopologyMode::Complete,
        )
        .expect("catalog should be valid");
        dispatcher
            .register_object(ObjectId::new(1), None, BROWSER_KIND.tag)
            .expect("browser root should register");
        assert!(
            dispatcher
                .pop_control_message()
                .expect("outbox should be readable")
                .is_some()
        );
        let batch = [
            ControlMessage::Create {
                object_id: ObjectId::new(2),
                parent: Some(ObjectId::new(1)),
                kind: CONTEXT_KIND.tag,
            },
            ControlMessage::Create {
                object_id: ObjectId::new(3),
                parent: Some(ObjectId::new(2)),
                kind: TOPOLOGY_PAGE_KIND.tag,
            },
        ];

        // Act
        let error = dispatcher
            .apply_lifecycle_batch(&batch)
            .expect_err("two lifecycle messages should not fit one free slot");

        // Assert
        assert_eq!(
            error,
            RuntimeStateError::ControlMessageCapacityExceeded {
                required: 2,
                remaining: 1,
                limit: 1,
            }
        );
        assert_eq!(dispatcher.live_object_count(), Ok(1));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(2)), Ok(false));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(3)), Ok(false));
        assert_eq!(dispatcher.pop_control_message(), Ok(None));
    }

    #[test]
    fn topology_rejects_wrong_parent_kinds_during_create_and_adopt() {
        // Arrange
        let dispatcher = DirectDispatcher::with_object_kinds(
            PendingService,
            RuntimeLimits::new(4, 1),
            TOPOLOGY_OBJECT_KINDS,
        )
        .expect("catalog should be valid");
        dispatcher
            .register_object(ObjectId::new(1), None, BROWSER_KIND.tag)
            .expect("browser should register");
        dispatcher
            .register_object(ObjectId::new(2), Some(ObjectId::new(1)), CONTEXT_KIND.tag)
            .expect("context should register");
        dispatcher
            .register_object(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                TOPOLOGY_PAGE_KIND.tag,
            )
            .expect("page should register");

        // Act
        let create_error = dispatcher
            .register_object(
                ObjectId::new(4),
                Some(ObjectId::new(1)),
                TOPOLOGY_PAGE_KIND.tag,
            )
            .expect_err("page cannot belong directly to browser");
        let adopt_error = dispatcher
            .adopt_object(ObjectId::new(2), ObjectId::new(3))
            .expect_err("context cannot move under page");

        // Assert
        assert_eq!(
            create_error,
            RuntimeStateError::InvalidParentKind {
                object_id: ObjectId::new(4),
                kind: TOPOLOGY_PAGE_KIND_ID,
                parent: ObjectId::new(1),
                parent_kind: BROWSER_KIND_ID,
            }
        );
        assert_eq!(
            adopt_error,
            RuntimeStateError::InvalidParentKind {
                object_id: ObjectId::new(2),
                kind: CONTEXT_KIND_ID,
                parent: ObjectId::new(3),
                parent_kind: TOPOLOGY_PAGE_KIND_ID,
            }
        );
        assert_eq!(dispatcher.live_object_count(), Ok(3));
    }

    #[test]
    fn dispatch_rejects_a_stale_target_before_invoking_the_service() {
        // Arrange
        let calls = Arc::new(AtomicUsize::new(0));
        let dispatcher = DirectDispatcher::new(
            ReadyService {
                calls: Arc::clone(&calls),
            },
            RuntimeLimits::new(1, 1),
        );
        let mut future = Box::pin(dispatcher.call(command(5, 99, 41, CallMetadata::default())));

        // Act
        let outcome = poll_once(future.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("preflight failure should complete in one poll");
        };
        let error = result
            .outcome()
            .as_ref()
            .expect_err("stale target should fail");
        assert_eq!(error.kind(), FailureKind::TargetClosed);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn dropping_an_in_flight_dispatch_releases_its_pending_call() {
        // Arrange
        let dispatcher = DirectDispatcher::new(PendingService, RuntimeLimits::new(1, 1));
        register_page(&dispatcher);
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        assert!(poll_once(future.as_mut()).is_pending());
        assert_eq!(dispatcher.pending_call_count(), Ok(1));

        // Act
        drop(future);

        // Assert
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn direct_dispatcher_rejects_unenforced_execution_control() {
        // Arrange
        let calls = Arc::new(AtomicUsize::new(0));
        let dispatcher = DirectDispatcher::new(
            ReadyService {
                calls: Arc::clone(&calls),
            },
            RuntimeLimits::new(1, 1),
        );
        register_page(&dispatcher);
        let metadata =
            CallMetadata::new(Some(Duration::from_secs(1)), Some(CancellationId::new(7)));
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, metadata)));

        // Act
        let outcome = poll_once(future.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("unsupported execution control should fail immediately");
        };
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("command should fail")
                .kind(),
            FailureKind::UnsupportedCapability
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn explicit_cancellation_drops_operation_then_runs_backend_cleanup() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = ControlledDispatcher::new(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::clone(&operation_drops),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("target should register");
        let metadata = CallMetadata::new(None, Some(CancellationId::new(7)));
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, metadata)));
        let wake = Arc::new(CountingWake {
            wakes: AtomicUsize::new(0),
        });
        assert!(poll_with_counting_waker(future.as_mut(), Arc::clone(&wake)).is_pending());

        // Act
        let cancellation = dispatcher.cancel_call(
            CallId::new(5),
            CancellationId::new(7),
            CancellationReason::Caller,
        );
        let outcome = poll_once(future.as_mut());

        // Assert
        assert_eq!(cancellation, Ok(CancellationRequestOutcome::Requested));
        assert_eq!(wake.wakes.load(Ordering::SeqCst), 1);
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("cancelled operation should complete");
        };
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("operation should be cancelled")
                .kind(),
            FailureKind::Cancelled
        );
        assert_eq!(operation_drops.load(Ordering::SeqCst), 1);
        let calls = cancellation_calls
            .lock()
            .expect("cancellation records should be available");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0.call_id(), CallId::new(5));
        assert_eq!(calls[0].0.method(), MethodTag::new(1));
        assert_eq!(calls[0].1, CancellationReason::Caller);
        assert_eq!(calls[0].2, 1);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn cancellation_rejects_the_wrong_identity_without_waking_the_call() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let dispatcher = ControlledDispatcher::new(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            RecordingCancellation {
                calls: Arc::new(Mutex::new(Vec::new())),
                operation_drops: Arc::clone(&operation_drops),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("target should register");
        let metadata = CallMetadata::new(None, Some(CancellationId::new(7)));
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, metadata)));
        assert!(poll_once(future.as_mut()).is_pending());

        // Act
        let cancellation = dispatcher.cancel_call(
            CallId::new(5),
            CancellationId::new(8),
            CancellationReason::Caller,
        );

        // Assert
        assert_eq!(
            cancellation,
            Err(RuntimeStateError::PendingCall(
                PendingCallError::CancellationIdMismatch {
                    call_id: CallId::new(5),
                    expected: CancellationId::new(7),
                    received: CancellationId::new(8),
                }
            ))
        );
        assert!(poll_once(future.as_mut()).is_pending());
        assert_eq!(dispatcher.pending_call_count(), Ok(1));
        drop(future);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn injected_deadline_cancels_and_cleans_up_a_pending_operation() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let deadlines = ManualDeadlineScheduler::new();
        let deadline_state = Arc::clone(&deadlines.state);
        let deadline_budgets = Arc::clone(&deadlines.budgets);
        let dispatcher = ControlledDispatcher::new(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::clone(&operation_drops),
                fail: false,
            },
            deadlines,
            RuntimeLimits::new(1, 1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("target should register");
        let metadata = CallMetadata::new(Some(Duration::from_millis(250)), None);
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, metadata)));
        assert!(poll_once(future.as_mut()).is_pending());

        // Act
        deadline_state.expired.store(true, Ordering::SeqCst);
        let outcome = poll_once(future.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("expired operation should complete");
        };
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("operation should time out")
                .kind(),
            FailureKind::Timeout
        );
        assert_eq!(
            *deadline_budgets
                .lock()
                .expect("deadline budgets should be available"),
            vec![Duration::from_millis(250)]
        );
        assert_eq!(operation_drops.load(Ordering::SeqCst), 1);
        assert_eq!(
            cancellation_calls
                .lock()
                .expect("cancellation records should be available")[0]
                .1,
            CancellationReason::DeadlineExceeded
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn successful_cleanup_can_enrich_the_original_deadline_error() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let cleanup_calls = Arc::new(AtomicUsize::new(0));
        let deadlines = ManualDeadlineScheduler::new();
        let deadline_state = Arc::clone(&deadlines.state);
        let dispatcher = ControlledDispatcher::new(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            EnrichedCancellation {
                cleanup_calls: Arc::clone(&cleanup_calls),
            },
            deadlines,
            RuntimeLimits::new(1, 1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("target should register");
        let metadata = CallMetadata::new(Some(Duration::from_millis(25)), None);
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, metadata)));
        assert!(poll_once(future.as_mut()).is_pending());

        // Act
        deadline_state.expired.store(true, Ordering::SeqCst);
        let outcome = poll_once(future.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("expired operation should complete");
        };
        let error = result
            .outcome()
            .as_ref()
            .expect_err("operation should time out");
        assert_eq!(error.kind(), FailureKind::Timeout);
        assert_eq!(error.message(), "bounded backend progress");
        assert_eq!(error.details(), Some(&()));
        assert_eq!(operation_drops.load(Ordering::SeqCst), 1);
        assert_eq!(cleanup_calls.load(Ordering::SeqCst), 1);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn backend_cancellation_failure_is_classified_as_cleanup() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let deadlines = ManualDeadlineScheduler::new();
        let deadline_state = Arc::clone(&deadlines.state);
        let dispatcher = ControlledDispatcher::new(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            RecordingCancellation {
                calls: Arc::new(Mutex::new(Vec::new())),
                operation_drops,
                fail: true,
            },
            deadlines,
            RuntimeLimits::new(1, 1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("target should register");
        let metadata = CallMetadata::new(Some(Duration::from_millis(1)), None);
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, metadata)));
        assert!(poll_once(future.as_mut()).is_pending());

        // Act
        deadline_state.expired.store(true, Ordering::SeqCst);
        let outcome = poll_once(future.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("failed cleanup should complete the command");
        };
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("cleanup should fail")
                .kind(),
            FailureKind::Cleanup
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn controlled_dispatcher_rejects_timeout_without_a_scheduler() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let dispatcher = ControlledDispatcher::new(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            RecordingCancellation {
                calls: Arc::new(Mutex::new(Vec::new())),
                operation_drops: Arc::clone(&operation_drops),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("target should register");
        let metadata = CallMetadata::new(Some(Duration::from_millis(1)), None);
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, metadata)));

        // Act
        let outcome = poll_once(future.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("missing scheduler should fail immediately");
        };
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("timeout should be rejected")
                .kind(),
            FailureKind::UnsupportedCapability
        );
        assert_eq!(operation_drops.load(Ordering::SeqCst), 0);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn observable_operation_completion_wins_over_an_expired_deadline() {
        // Arrange
        let service_calls = Arc::new(AtomicUsize::new(0));
        let service_ready = Arc::new(AtomicBool::new(false));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let deadlines = ManualDeadlineScheduler::new();
        let deadline_state = Arc::clone(&deadlines.state);
        let dispatcher = ControlledDispatcher::new(
            ManuallyCompletedService {
                ready: Arc::clone(&service_ready),
                calls: Arc::clone(&service_calls),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::new(AtomicUsize::new(0)),
                fail: false,
            },
            deadlines,
            RuntimeLimits::new(1, 1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("target should register");
        let metadata = CallMetadata::new(Some(Duration::from_millis(1)), None);
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, metadata)));
        assert!(poll_once(future.as_mut()).is_pending());
        assert_eq!(service_calls.load(Ordering::SeqCst), 1);

        // Act
        service_ready.store(true, Ordering::SeqCst);
        deadline_state.expired.store(true, Ordering::SeqCst);
        let outcome = poll_once(future.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("ready operation should win the poll");
        };
        assert_eq!(result.into_outcome(), Ok(42));
        assert_eq!(service_calls.load(Ordering::SeqCst), 1);
        assert!(
            cancellation_calls
                .lock()
                .expect("cancellation records should be available")
                .is_empty()
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn first_accepted_cancellation_reason_wins_a_ready_deadline_race() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let deadlines = ManualDeadlineScheduler::new();
        let deadline_state = Arc::clone(&deadlines.state);
        let dispatcher = ControlledDispatcher::new(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops,
                fail: false,
            },
            deadlines,
            RuntimeLimits::new(1, 1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("target should register");
        let metadata =
            CallMetadata::new(Some(Duration::from_millis(1)), Some(CancellationId::new(7)));
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, metadata)));
        assert!(poll_once(future.as_mut()).is_pending());
        dispatcher
            .cancel_call(
                CallId::new(5),
                CancellationId::new(7),
                CancellationReason::Caller,
            )
            .expect("caller cancellation should be accepted first");
        deadline_state.expired.store(true, Ordering::SeqCst);

        // Act
        let outcome = poll_once(future.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("cancelled operation should complete");
        };
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("operation should be cancelled")
                .kind(),
            FailureKind::Cancelled
        );
        assert_eq!(
            cancellation_calls
                .lock()
                .expect("cancellation records should be available")[0]
                .1,
            CancellationReason::Caller
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn direct_dispatcher_serializes_calls_for_the_same_target() {
        // Arrange
        let service_calls = Arc::new(AtomicUsize::new(0));
        let dispatcher = DirectDispatcher::new(
            CountingPendingService {
                calls: Arc::clone(&service_calls),
            },
            RuntimeLimits::new(1, 2),
        );
        register_page(&dispatcher);
        let mut first = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        let mut second = Box::pin(dispatcher.call(command(6, 2, 42, CallMetadata::default())));
        assert!(poll_once(first.as_mut()).is_pending());
        assert!(poll_once(second.as_mut()).is_pending());
        assert_eq!(service_calls.load(Ordering::SeqCst), 1);

        // Act
        drop(first);
        let second_outcome = poll_once(second.as_mut());

        // Assert
        assert!(second_outcome.is_pending());
        assert_eq!(service_calls.load(Ordering::SeqCst), 2);
        assert_eq!(dispatcher.pending_call_count(), Ok(1));
        assert_eq!(
            dispatcher
                .scheduler_metrics()
                .expect("scheduler metrics should load")
                .waiting_calls,
            0
        );
        drop(second);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn direct_dispatcher_refuses_to_dispose_a_busy_target() {
        // Arrange
        let dispatcher = DirectDispatcher::new(PendingService, RuntimeLimits::new(1, 1));
        register_page(&dispatcher);
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        assert!(poll_once(future.as_mut()).is_pending());

        // Act
        let disposal = dispatcher.dispose_object(ObjectId::new(2));

        // Assert
        assert_eq!(
            disposal,
            Err(RuntimeStateError::ObjectHasPendingCalls {
                object_id: ObjectId::new(2),
                pending_calls: 1,
            })
        );
        assert_eq!(dispatcher.live_object_count(), Ok(1));
        drop(future);
        assert_eq!(
            dispatcher
                .dispose_object(ObjectId::new(2))
                .expect("idle target should dispose")
                .len(),
            1
        );
    }

    #[test]
    fn direct_dispatcher_closes_transport_only_when_idle() {
        // Arrange
        let dispatcher = DirectDispatcher::new(PendingService, RuntimeLimits::new(1, 1));
        register_page(&dispatcher);
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        assert!(poll_once(future.as_mut()).is_pending());

        // Act
        let busy_close = dispatcher.close_transport();
        drop(future);
        let close = dispatcher
            .close_transport()
            .expect("idle direct dispatcher should close");

        // Assert
        assert_eq!(
            busy_close,
            Err(RuntimeStateError::PendingCallsPreventTransportClose { pending_calls: 1 })
        );
        assert!(!close.already_closed());
        assert_eq!(close.pending_calls(), 0);
        assert_eq!(
            close
                .invalidated_objects()
                .iter()
                .map(|record| record.id())
                .collect::<Vec<_>>(),
            vec![ObjectId::new(2)]
        );
        assert_eq!(dispatcher.live_object_count(), Ok(0));
        let mut after_close = Box::pin(dispatcher.call(command(6, 2, 42, CallMetadata::default())));
        let Poll::Ready(Ok(result)) = poll_once(after_close.as_mut()) else {
            panic!("closed direct dispatcher should reject immediately");
        };
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("closed transport should reject command")
                .kind(),
            FailureKind::TransportClosed
        );
    }

    #[test]
    fn controlled_parent_disposal_cancels_calls_across_the_subtree() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = ControlledDispatcher::new(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::clone(&operation_drops),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(3, 2),
        );
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("browser should register");
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .expect("context should register");
        dispatcher
            .register_object(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                ObjectKindTag::new(3),
            )
            .expect("page should register");
        let mut context_call =
            Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        let mut page_call = Box::pin(dispatcher.call(command(6, 3, 42, CallMetadata::default())));
        assert!(poll_once(context_call.as_mut()).is_pending());
        assert!(poll_once(page_call.as_mut()).is_pending());

        // Act
        let disposed = dispatcher
            .dispose_object(ObjectId::new(1))
            .expect("controlled dispatcher should dispose and signal the subtree");
        let context_outcome = poll_once(context_call.as_mut());
        let page_outcome = poll_once(page_call.as_mut());

        // Assert
        assert_eq!(
            disposed
                .iter()
                .map(|record| record.id())
                .collect::<Vec<_>>(),
            vec![ObjectId::new(3), ObjectId::new(2), ObjectId::new(1)]
        );
        for outcome in [context_outcome, page_outcome] {
            let Poll::Ready(Ok(result)) = outcome else {
                panic!("parent-closed call should terminate");
            };
            assert_eq!(
                result
                    .outcome()
                    .as_ref()
                    .expect_err("parent-closed call should fail")
                    .kind(),
                FailureKind::TargetClosed
            );
        }
        assert_eq!(operation_drops.load(Ordering::SeqCst), 2);
        assert!(
            cancellation_calls
                .lock()
                .expect("cancellation records should be available")
                .iter()
                .all(|call| call.1 == CancellationReason::ParentClosed)
        );
        assert_eq!(dispatcher.live_object_count(), Ok(0));
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
        assert_eq!(
            dispatcher
                .scheduler_metrics()
                .expect("scheduler metrics should load")
                .active_targets,
            0
        );
    }

    #[test]
    fn prepared_disposal_shares_one_terminal_failure_across_subtree_calls() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = ControlledDispatcher::new(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::clone(&operation_drops),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(3, 2).with_control_messages(6),
        );
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("browser should register");
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .expect("context should register");
        dispatcher
            .register_object(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                ObjectKindTag::new(3),
            )
            .expect("page should register");
        for _ in 0..3 {
            dispatcher
                .pop_control_message()
                .expect("control queue should be available")
                .expect("create notification should be queued");
        }
        let mut context_call =
            Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        let mut page_call = Box::pin(dispatcher.call(command(6, 3, 42, CallMetadata::default())));
        assert!(poll_once(context_call.as_mut()).is_pending());
        assert!(poll_once(page_call.as_mut()).is_pending());
        let prepared = dispatcher
            .lifecycle_handle()
            .prepare_object_disposal(ObjectId::new(1))
            .expect("browser disposal should reserve child-first notifications");

        // Act
        let disposed = prepared
            .commit_with_failure(RuntimeCallFailure::new(
                FailureKind::TargetClosed,
                "planned restart",
            ))
            .expect("prepared browser disposal should commit");
        let context_outcome = poll_once(context_call.as_mut());
        let page_outcome = poll_once(page_call.as_mut());

        // Assert
        assert_eq!(
            disposed
                .iter()
                .map(|record| record.id())
                .collect::<Vec<_>>(),
            vec![ObjectId::new(3), ObjectId::new(2), ObjectId::new(1)]
        );
        for outcome in [context_outcome, page_outcome] {
            let Poll::Ready(Ok(result)) = outcome else {
                panic!("disposed subtree call should terminate");
            };
            let error = result
                .outcome()
                .as_ref()
                .expect_err("disposed subtree call should fail");
            assert_eq!(error.kind(), FailureKind::TargetClosed);
            assert_eq!(error.message(), "planned restart");
        }
        assert_eq!(operation_drops.load(Ordering::SeqCst), 2);
        assert!(
            cancellation_calls
                .lock()
                .expect("cancellation records should be available")
                .iter()
                .all(|call| call.1 == CancellationReason::ParentClosed)
        );
        assert_eq!(dispatcher.live_object_count(), Ok(0));
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn typed_subtree_interruption_terminates_only_affected_calls_and_keeps_objects_live() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = ControlledDispatcher::new(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::clone(&operation_drops),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(4, 4).with_active_commands(3),
        );
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(3))
            .expect("first page should register");
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(4),
            )
            .expect("first frame should register");
        dispatcher
            .register_object(ObjectId::new(3), None, ObjectKindTag::new(3))
            .expect("second page should register");
        dispatcher
            .register_object(
                ObjectId::new(4),
                Some(ObjectId::new(3)),
                ObjectKindTag::new(4),
            )
            .expect("second frame should register");
        let mut page_call = Box::pin(dispatcher.call(command(5, 1, 41, CallMetadata::default())));
        let mut frame_call = Box::pin(dispatcher.call(command(6, 2, 42, CallMetadata::default())));
        let mut sibling_call =
            Box::pin(dispatcher.call(command(7, 4, 43, CallMetadata::default())));
        let mut queued_frame_call =
            Box::pin(dispatcher.call(command(8, 2, 44, CallMetadata::default())));
        assert!(poll_once(page_call.as_mut()).is_pending());
        assert!(poll_once(frame_call.as_mut()).is_pending());
        assert!(poll_once(sibling_call.as_mut()).is_pending());
        assert!(poll_once(queued_frame_call.as_mut()).is_pending());

        // Act
        let interrupted = dispatcher
            .interrupt_subtree_calls(
                ObjectId::new(1),
                RuntimeCallFailure::new(FailureKind::BrowserCrashed, "page renderer crashed"),
            )
            .expect("live page subtree should be interruptible");
        let page_outcome = poll_once(page_call.as_mut());
        let frame_outcome = poll_once(frame_call.as_mut());
        let queued_frame_outcome = poll_once(queued_frame_call.as_mut());
        let sibling_outcome = poll_once(sibling_call.as_mut());

        // Assert
        assert_eq!(interrupted, 3);
        for outcome in [page_outcome, frame_outcome, queued_frame_outcome] {
            let Poll::Ready(Ok(result)) = outcome else {
                panic!("crashed page call should terminate");
            };
            let error = result
                .outcome()
                .as_ref()
                .expect_err("crashed page call should fail");
            assert_eq!(error.kind(), FailureKind::BrowserCrashed);
            assert_eq!(error.message(), "page renderer crashed");
        }
        assert!(sibling_outcome.is_pending());
        assert_eq!(operation_drops.load(Ordering::SeqCst), 2);
        assert_eq!(dispatcher.live_object_count(), Ok(4));
        assert_eq!(dispatcher.pending_call_count(), Ok(1));
        assert!(
            cancellation_calls
                .lock()
                .expect("cancellation records should be available")
                .iter()
                .all(|call| call.1 == CancellationReason::ParentClosed)
        );
        drop(sibling_call);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn typed_subtree_interruption_preserves_an_already_accepted_caller_cancellation() {
        // Arrange
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = ControlledDispatcher::new(
            PendingService,
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::new(AtomicUsize::new(0)),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("page should register");
        let cancellation_id = CancellationId::new(9);
        let mut call = Box::pin(dispatcher.call(command(
            5,
            2,
            41,
            CallMetadata::new(None, Some(cancellation_id)),
        )));
        assert!(poll_once(call.as_mut()).is_pending());
        dispatcher
            .cancel_call(CallId::new(5), cancellation_id, CancellationReason::Caller)
            .expect("caller cancellation should be accepted first");

        // Act
        let interrupted = dispatcher
            .interrupt_subtree_calls(
                ObjectId::new(2),
                RuntimeCallFailure::new(FailureKind::BrowserCrashed, "page renderer crashed"),
            )
            .expect("later crash notification should be harmless");
        let outcome = poll_once(call.as_mut());

        // Assert
        assert_eq!(interrupted, 0);
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("caller-cancelled operation should terminate");
        };
        assert_eq!(
            result
                .into_outcome()
                .expect_err("caller cancellation should remain authoritative")
                .kind(),
            FailureKind::Cancelled
        );
        assert_eq!(
            cancellation_calls
                .lock()
                .expect("cancellation records should be available")[0]
                .1,
            CancellationReason::Caller
        );
        assert_eq!(dispatcher.live_object_count(), Ok(1));
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn cancelling_a_queued_call_does_not_invoke_backend_cleanup() {
        // Arrange
        let service_calls = Arc::new(AtomicUsize::new(0));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = ControlledDispatcher::new(
            CountingPendingService {
                calls: Arc::clone(&service_calls),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::new(AtomicUsize::new(0)),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 2),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("page should register");
        let mut active = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        let cancellation_id = CancellationId::new(8);
        let mut queued = Box::pin(dispatcher.call(command(
            6,
            2,
            42,
            CallMetadata::new(None, Some(cancellation_id)),
        )));
        assert!(poll_once(active.as_mut()).is_pending());
        assert!(poll_once(queued.as_mut()).is_pending());
        assert_eq!(service_calls.load(Ordering::SeqCst), 1);

        // Act
        dispatcher
            .cancel_call(CallId::new(6), cancellation_id, CancellationReason::Caller)
            .expect("queued call cancellation should be accepted");
        let queued_outcome = poll_once(queued.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = queued_outcome else {
            panic!("queued cancellation should complete");
        };
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("queued command should be cancelled")
                .kind(),
            FailureKind::Cancelled
        );
        assert_eq!(service_calls.load(Ordering::SeqCst), 1);
        assert!(
            cancellation_calls
                .lock()
                .expect("cancellation records should be available")
                .is_empty()
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(1));
        drop(active);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn accepted_parent_close_preempts_a_simultaneously_ready_operation() {
        // Arrange
        let service_ready = Arc::new(AtomicBool::new(false));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = ControlledDispatcher::new(
            ManuallyCompletedService {
                ready: Arc::clone(&service_ready),
                calls: Arc::new(AtomicUsize::new(0)),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::new(AtomicUsize::new(0)),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("page should register");
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        assert!(poll_once(future.as_mut()).is_pending());
        service_ready.store(true, Ordering::SeqCst);

        // Act
        dispatcher
            .dispose_object(ObjectId::new(2))
            .expect("page disposal should be accepted");
        let outcome = poll_once(future.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("disposed target call should terminate");
        };
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("parent close should preempt completion")
                .kind(),
            FailureKind::TargetClosed
        );
        assert_eq!(
            cancellation_calls
                .lock()
                .expect("cancellation records should be available")[0]
                .1,
            CancellationReason::ParentClosed
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn transport_close_invalidates_objects_and_cancels_active_and_queued_calls() {
        // Arrange
        let service_calls = Arc::new(AtomicUsize::new(0));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = ControlledDispatcher::new(
            CountingPendingService {
                calls: Arc::clone(&service_calls),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::new(AtomicUsize::new(0)),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(3, 3),
        );
        dispatcher
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("browser should register");
        dispatcher
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(3),
            )
            .expect("first page should register");
        dispatcher
            .register_object(ObjectId::new(3), None, ObjectKindTag::new(3))
            .expect("second root should register");
        let mut first = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        let mut queued = Box::pin(dispatcher.call(command(6, 2, 42, CallMetadata::default())));
        let mut independent = Box::pin(dispatcher.call(command(7, 3, 43, CallMetadata::default())));
        assert!(poll_once(first.as_mut()).is_pending());
        assert!(poll_once(queued.as_mut()).is_pending());
        assert!(poll_once(independent.as_mut()).is_pending());
        assert_eq!(service_calls.load(Ordering::SeqCst), 2);

        // Act
        let close = dispatcher
            .close_transport()
            .expect("transport close should fan out");
        let outcomes = [
            poll_once(first.as_mut()),
            poll_once(queued.as_mut()),
            poll_once(independent.as_mut()),
        ];

        // Assert
        assert!(!close.already_closed());
        assert_eq!(close.pending_calls(), 3);
        assert_eq!(
            close
                .invalidated_objects()
                .iter()
                .map(|record| record.id())
                .collect::<Vec<_>>(),
            vec![ObjectId::new(2), ObjectId::new(1), ObjectId::new(3)]
        );
        for outcome in outcomes {
            let Poll::Ready(Ok(result)) = outcome else {
                panic!("transport-closed call should terminate");
            };
            assert_eq!(
                result
                    .outcome()
                    .as_ref()
                    .expect_err("transport-closed command should fail")
                    .kind(),
                FailureKind::TransportClosed
            );
        }
        assert!(
            cancellation_calls
                .lock()
                .expect("cancellation records should be available")
                .is_empty(),
            "a dead transport must not receive backend abort commands"
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
        assert_eq!(dispatcher.live_object_count(), Ok(0));
        assert_eq!(
            dispatcher.register_object(ObjectId::new(9), None, ObjectKindTag::new(3)),
            Err(RuntimeStateError::TransportClosed)
        );
        let mut after_close = Box::pin(dispatcher.call(command(8, 2, 44, CallMetadata::default())));
        let Poll::Ready(Ok(result)) = poll_once(after_close.as_mut()) else {
            panic!("new command should fail immediately after close");
        };
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("closed runtime should reject commands")
                .kind(),
            FailureKind::TransportClosed
        );
        let repeated = dispatcher
            .close_transport()
            .expect("repeated close should be idempotent");
        assert!(repeated.already_closed());
        assert_eq!(repeated.pending_calls(), 0);
        assert!(repeated.invalidated_objects().is_empty());
    }

    #[test]
    fn transport_close_preempts_a_simultaneously_ready_operation_without_cleanup() {
        // Arrange
        let service_ready = Arc::new(AtomicBool::new(false));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = ControlledDispatcher::new(
            ManuallyCompletedService {
                ready: Arc::clone(&service_ready),
                calls: Arc::new(AtomicUsize::new(0)),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::new(AtomicUsize::new(0)),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("page should register");
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        assert!(poll_once(future.as_mut()).is_pending());
        service_ready.store(true, Ordering::SeqCst);

        // Act
        dispatcher
            .close_transport()
            .expect("transport close should be accepted");
        let outcome = poll_once(future.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = outcome else {
            panic!("transport-closed call should terminate");
        };
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("transport close should preempt completion")
                .kind(),
            FailureKind::TransportClosed
        );
        assert!(
            cancellation_calls
                .lock()
                .expect("cancellation records should be available")
                .is_empty()
        );
    }

    #[test]
    fn dropping_an_active_controlled_dispatch_submits_cleanup_after_operation_drop() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = ControlledDispatcher::new(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::clone(&operation_drops),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("page should register");
        let mut future = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        assert!(poll_once(future.as_mut()).is_pending());

        // Act
        drop(future);
        let mut cleanup = dispatcher
            .cleanup_executor()
            .try_take()
            .expect("cleanup queue should load")
            .expect("dropped active call should submit cleanup");
        let cleanup_outcome = poll_once(cleanup.as_mut());

        // Assert
        assert_eq!(operation_drops.load(Ordering::SeqCst), 1);
        assert!(matches!(cleanup_outcome, Poll::Ready(Ok(()))));
        let calls = cancellation_calls
            .lock()
            .expect("cancellation records should be available");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0.call_id(), CallId::new(5));
        assert_eq!(calls[0].1, CancellationReason::Caller);
        assert_eq!(calls[0].2, 1, "operation must drop before cleanup runs");
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
        assert_eq!(
            dispatcher.abandoned_cleanup_metrics(),
            AbandonedCleanupMetrics {
                attempted: 1,
                submitted: 1,
                rejected: 0,
            }
        );
        assert_eq!(
            dispatcher.cleanup_executor().metrics(),
            Ok(CleanupQueueMetrics {
                capacity: 1,
                reserved: 0,
                queued: 0,
                peak_queued: 1,
                accepted: 1,
                rejected: 0,
                completed: 1,
                failed: 0,
            })
        );
    }

    #[test]
    fn dropping_a_queued_controlled_dispatch_does_not_submit_backend_cleanup() {
        // Arrange
        let service_calls = Arc::new(AtomicUsize::new(0));
        let dispatcher = ControlledDispatcher::new(
            CountingPendingService {
                calls: Arc::clone(&service_calls),
            },
            RecordingCancellation {
                calls: Arc::new(Mutex::new(Vec::new())),
                operation_drops: Arc::new(AtomicUsize::new(0)),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 2),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("page should register");
        let mut active = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        let mut queued = Box::pin(dispatcher.call(command(6, 2, 42, CallMetadata::default())));
        assert!(poll_once(active.as_mut()).is_pending());
        assert!(poll_once(queued.as_mut()).is_pending());

        // Act
        drop(queued);

        // Assert
        assert_eq!(service_calls.load(Ordering::SeqCst), 1);
        assert_eq!(dispatcher.pending_call_count(), Ok(1));
        assert_eq!(
            dispatcher.abandoned_cleanup_metrics(),
            AbandonedCleanupMetrics::default()
        );
        assert_eq!(
            dispatcher
                .cleanup_executor()
                .metrics()
                .expect("cleanup metrics should load")
                .queued,
            0
        );
        drop(active);
    }

    #[test]
    fn saturated_cleanup_executor_rejects_without_unbounded_growth() {
        // Arrange
        let operation_drops = Arc::new(AtomicUsize::new(0));
        let queue = CleanupQueue::new(1);
        let dispatcher = ControlledDispatcher::with_cleanup_executor(
            DropAwarePendingService {
                drops: Arc::clone(&operation_drops),
            },
            RecordingCancellation {
                calls: Arc::new(Mutex::new(Vec::new())),
                operation_drops: Arc::clone(&operation_drops),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(2, 2),
            queue.clone(),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("first page should register");
        dispatcher
            .register_object(ObjectId::new(3), None, ObjectKindTag::new(3))
            .expect("second page should register");
        let mut first = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        let mut second = Box::pin(dispatcher.call(command(6, 3, 42, CallMetadata::default())));
        assert!(poll_once(first.as_mut()).is_pending());
        assert!(poll_once(second.as_mut()).is_pending());

        // Act
        drop(first);
        drop(second);

        // Assert
        assert_eq!(operation_drops.load(Ordering::SeqCst), 2);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
        assert_eq!(
            dispatcher.abandoned_cleanup_metrics(),
            AbandonedCleanupMetrics {
                attempted: 2,
                submitted: 1,
                rejected: 1,
            }
        );
        assert_eq!(
            queue.metrics(),
            Ok(CleanupQueueMetrics {
                capacity: 1,
                reserved: 0,
                queued: 1,
                peak_queued: 1,
                accepted: 1,
                rejected: 1,
                completed: 0,
                failed: 0,
            })
        );
    }

    #[test]
    fn direct_dispatcher_bounds_commands_across_independent_targets() {
        // Arrange
        let service_calls = Arc::new(AtomicUsize::new(0));
        let dispatcher = DirectDispatcher::new(
            CountingPendingService {
                calls: Arc::clone(&service_calls),
            },
            RuntimeLimits::new(2, 2).with_active_commands(1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("first page should register");
        dispatcher
            .register_object(ObjectId::new(3), None, ObjectKindTag::new(3))
            .expect("second page should register");
        let mut first = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        let mut second = Box::pin(dispatcher.call(command(6, 3, 42, CallMetadata::default())));
        assert!(poll_once(first.as_mut()).is_pending());
        assert!(poll_once(second.as_mut()).is_pending());

        // Act
        drop(first);
        let second_outcome = poll_once(second.as_mut());

        // Assert
        assert!(second_outcome.is_pending());
        assert_eq!(service_calls.load(Ordering::SeqCst), 2);
        let metrics = dispatcher
            .connection_metrics()
            .expect("connection metrics should load");
        assert_eq!(metrics.active_commands, 1);
        assert_eq!(metrics.waiting_calls, 0);
        assert_eq!(metrics.peak_active_commands, 1);
        assert_eq!(metrics.peak_waiting_calls, 1);
        drop(second);
    }

    #[test]
    fn shared_budget_counts_waiters_as_pending_but_only_grants_as_active() {
        // Arrange
        let service_calls = Arc::new(AtomicUsize::new(0));
        let ledger = runtime_ledger_with_active(2, 2, 1);
        let dispatcher = DirectDispatcher::new(
            CountingPendingService {
                calls: Arc::clone(&service_calls),
            },
            RuntimeLimits::new(2, 2).with_active_commands(1),
        )
        .with_resource_ledger(ledger.clone())
        .expect("ledger should install before runtime state");
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("first page should register");
        dispatcher
            .register_object(ObjectId::new(3), None, ObjectKindTag::new(3))
            .expect("second page should register");
        let mut first = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        let mut second = Box::pin(dispatcher.call(command(6, 3, 42, CallMetadata::default())));
        assert!(poll_once(first.as_mut()).is_pending());
        assert!(poll_once(second.as_mut()).is_pending());
        let while_waiting = ledger.metrics().expect("resource metrics should load");
        let first_scope = ledger
            .scope_metrics(ResourceScope::Call(CallId::new(5)))
            .expect("first call metrics should load")
            .expect("first call should own a scope");
        let second_waiting_scope = ledger
            .scope_metrics(ResourceScope::Call(CallId::new(6)))
            .expect("second call metrics should load")
            .expect("second call should own a scope");

        // Act
        drop(first);
        let after_promotion = ledger.metrics().expect("resource metrics should load");
        let old_scope = ledger
            .scope_metrics(ResourceScope::Call(CallId::new(5)))
            .expect("old call metrics should load");
        let second_active_scope = ledger
            .scope_metrics(ResourceScope::Call(CallId::new(6)))
            .expect("second call metrics should load")
            .expect("promoted call should retain its scope");
        let promoted_outcome = poll_once(second.as_mut());
        drop(second);
        let released = ledger.metrics().expect("resource metrics should load");

        // Assert
        assert!(promoted_outcome.is_pending());
        assert_eq!(service_calls.load(Ordering::SeqCst), 2);
        assert_eq!(while_waiting.class(ResourceClass::PendingCall).current, 2);
        assert_eq!(while_waiting.class(ResourceClass::ActiveCommand).current, 1);
        assert_eq!(first_scope.units(ResourceClass::PendingCall), 1);
        assert_eq!(first_scope.units(ResourceClass::ActiveCommand), 1);
        assert_eq!(second_waiting_scope.units(ResourceClass::PendingCall), 1);
        assert_eq!(second_waiting_scope.units(ResourceClass::ActiveCommand), 0);
        assert!(old_scope.is_none());
        assert_eq!(after_promotion.class(ResourceClass::PendingCall).current, 1);
        assert_eq!(
            after_promotion.class(ResourceClass::ActiveCommand).current,
            1
        );
        assert_eq!(after_promotion.class(ResourceClass::ActiveCommand).peak, 1);
        assert_eq!(second_active_scope.units(ResourceClass::PendingCall), 1);
        assert_eq!(second_active_scope.units(ResourceClass::ActiveCommand), 1);
        assert_eq!(released.class(ResourceClass::PendingCall).current, 0);
        assert_eq!(released.class(ResourceClass::ActiveCommand).current, 0);
        assert_eq!(released.class(ResourceClass::LiveObject).current, 2);
        assert_eq!(released.active_leases, 2);
    }

    #[test]
    fn controlled_call_can_cancel_while_waiting_for_connection_capacity() {
        // Arrange
        let service_calls = Arc::new(AtomicUsize::new(0));
        let cancellation_calls = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = ControlledDispatcher::new(
            CountingPendingService {
                calls: Arc::clone(&service_calls),
            },
            RecordingCancellation {
                calls: Arc::clone(&cancellation_calls),
                operation_drops: Arc::new(AtomicUsize::new(0)),
                fail: false,
            },
            NoDeadlineScheduler,
            RuntimeLimits::new(2, 2).with_active_commands(1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("first page should register");
        dispatcher
            .register_object(ObjectId::new(3), None, ObjectKindTag::new(3))
            .expect("second page should register");
        let mut active = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        let cancellation_id = CancellationId::new(8);
        let mut waiting = Box::pin(dispatcher.call(command(
            6,
            3,
            42,
            CallMetadata::new(None, Some(cancellation_id)),
        )));
        assert!(poll_once(active.as_mut()).is_pending());
        assert!(poll_once(waiting.as_mut()).is_pending());
        assert_eq!(service_calls.load(Ordering::SeqCst), 1);

        // Act
        dispatcher
            .cancel_call(CallId::new(6), cancellation_id, CancellationReason::Caller)
            .expect("waiting call cancellation should be accepted");
        let waiting_outcome = poll_once(waiting.as_mut());

        // Assert
        let Poll::Ready(Ok(result)) = waiting_outcome else {
            panic!("cancelled connection waiter should terminate");
        };
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("waiting command should be cancelled")
                .kind(),
            FailureKind::Cancelled
        );
        assert!(
            cancellation_calls
                .lock()
                .expect("cancellation records should be available")
                .is_empty(),
            "backend cleanup must not run before backend admission"
        );
        assert_eq!(service_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            dispatcher
                .connection_metrics()
                .expect("connection metrics should load")
                .waiting_calls,
            0
        );
        drop(active);
    }

    #[test]
    fn target_followers_do_not_reserve_connection_capacity() {
        // Arrange
        let service_calls = Arc::new(AtomicUsize::new(0));
        let dispatcher = DirectDispatcher::new(
            CountingPendingService {
                calls: Arc::clone(&service_calls),
            },
            RuntimeLimits::new(2, 3).with_active_commands(1),
        );
        dispatcher
            .register_object(ObjectId::new(2), None, ObjectKindTag::new(3))
            .expect("first page should register");
        dispatcher
            .register_object(ObjectId::new(3), None, ObjectKindTag::new(3))
            .expect("second page should register");
        let mut first = Box::pin(dispatcher.call(command(5, 2, 41, CallMetadata::default())));
        let mut same_target = Box::pin(dispatcher.call(command(6, 2, 42, CallMetadata::default())));
        let mut other_target =
            Box::pin(dispatcher.call(command(7, 3, 43, CallMetadata::default())));
        assert!(poll_once(first.as_mut()).is_pending());
        assert!(poll_once(same_target.as_mut()).is_pending());
        assert!(poll_once(other_target.as_mut()).is_pending());
        assert_eq!(service_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            dispatcher
                .scheduler_metrics()
                .expect("target metrics should load")
                .waiting_calls,
            1
        );
        assert_eq!(
            dispatcher
                .connection_metrics()
                .expect("connection metrics should load")
                .waiting_calls,
            1
        );

        // Act
        drop(first);
        let other_outcome = poll_once(other_target.as_mut());
        let same_outcome = poll_once(same_target.as_mut());

        // Assert
        assert!(other_outcome.is_pending());
        assert!(same_outcome.is_pending());
        assert_eq!(
            service_calls.load(Ordering::SeqCst),
            2,
            "the other target head should receive global capacity first"
        );
        assert_eq!(
            dispatcher
                .connection_metrics()
                .expect("connection metrics should load")
                .waiting_calls,
            1
        );
        drop(other_target);
        assert!(poll_once(same_target.as_mut()).is_pending());
        assert_eq!(service_calls.load(Ordering::SeqCst), 3);
        drop(same_target);
    }
}
