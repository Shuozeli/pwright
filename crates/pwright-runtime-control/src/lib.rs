//! Opt-in, caller-driven bidirectional runtime control frames.
//!
//! The driver sends at most one already-bounded frame per `send_once` call.
//! It starts no task, thread, or async runtime. The runtime message remains in
//! its bounded outbox when encoding or transport sending fails and is removed
//! only after a successful send. The receive side applies validated client
//! cancellation and tracks bounded subscription requests.

use std::collections::{BTreeMap, VecDeque};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use pwright_core::{
    CallId, CancellationId, CancellationReason, CancellationRequestOutcome, ControlMessage,
    EventTag, ObjectId, ProductionResourceError, ProductionResourceLedger, ResourceClass,
    ResourceError, ResourceLease, ResourceLedger, ResourceRequirements, ResourceScope,
};
use pwright_runtime::{
    ControlMessageClaim, ControlledDispatcher, DirectDispatcher, RuntimeStateError,
    TransportCloseOutcome,
};
use pwright_server_wire::{
    ConcurrentServerOutbound, ServerControlFrameHandler, ServerFrameDisposition, ServerFrameHandler,
};
use pwright_transport::{FrameRoute, FrameTransport, WireCodec, WireProtocol, WireProtocolError};

/// Runtime operations accepted from the client-to-runtime control direction.
pub trait CancellationTarget {
    fn cancel_call(
        &self,
        call_id: CallId,
        cancellation: CancellationId,
        reason: CancellationReason,
    ) -> Result<CancellationRequestOutcome, RuntimeStateError>;

    fn is_object_live(&self, object_id: ObjectId) -> Result<bool, RuntimeStateError>;
}

/// Owns the runtime transition caused by losing the client-to-server receive
/// side. Controlled runtimes wake every pending call with `TransportClosed`.
pub trait TransportCloseTarget {
    fn close_transport(&self) -> Result<TransportCloseOutcome, RuntimeStateError>;
}

impl<S, Cancellation, Deadlines, Cleanup> CancellationTarget
    for ControlledDispatcher<S, Cancellation, Deadlines, Cleanup>
{
    fn cancel_call(
        &self,
        call_id: CallId,
        cancellation: CancellationId,
        reason: CancellationReason,
    ) -> Result<CancellationRequestOutcome, RuntimeStateError> {
        ControlledDispatcher::cancel_call(self, call_id, cancellation, reason)
    }

    fn is_object_live(&self, object_id: ObjectId) -> Result<bool, RuntimeStateError> {
        ControlledDispatcher::is_object_live(self, object_id)
    }
}

impl<S, Cancellation, Deadlines, Cleanup> TransportCloseTarget
    for ControlledDispatcher<S, Cancellation, Deadlines, Cleanup>
{
    fn close_transport(&self) -> Result<TransportCloseOutcome, RuntimeStateError> {
        ControlledDispatcher::close_transport(self)
    }
}

impl<T> CancellationTarget for &T
where
    T: CancellationTarget + ?Sized,
{
    fn cancel_call(
        &self,
        call_id: CallId,
        cancellation: CancellationId,
        reason: CancellationReason,
    ) -> Result<CancellationRequestOutcome, RuntimeStateError> {
        T::cancel_call(*self, call_id, cancellation, reason)
    }

    fn is_object_live(&self, object_id: ObjectId) -> Result<bool, RuntimeStateError> {
        T::is_object_live(*self, object_id)
    }
}

impl<T> TransportCloseTarget for &T
where
    T: TransportCloseTarget + ?Sized,
{
    fn close_transport(&self) -> Result<TransportCloseOutcome, RuntimeStateError> {
        T::close_transport(*self)
    }
}

impl<T> CancellationTarget for std::sync::Arc<T>
where
    T: CancellationTarget + ?Sized,
{
    fn cancel_call(
        &self,
        call_id: CallId,
        cancellation: CancellationId,
        reason: CancellationReason,
    ) -> Result<CancellationRequestOutcome, RuntimeStateError> {
        T::cancel_call(self, call_id, cancellation, reason)
    }

    fn is_object_live(&self, object_id: ObjectId) -> Result<bool, RuntimeStateError> {
        T::is_object_live(self, object_id)
    }
}

impl<T> TransportCloseTarget for std::sync::Arc<T>
where
    T: TransportCloseTarget + ?Sized,
{
    fn close_transport(&self) -> Result<TransportCloseOutcome, RuntimeStateError> {
        T::close_transport(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeControlEvent {
    Cancellation {
        call_id: CallId,
        outcome: CancellationRequestOutcome,
    },
    Subscription {
        object_id: ObjectId,
        event: EventTag,
        enabled: bool,
        changed: bool,
    },
    TransportClosed {
        already_closed: bool,
        pending_calls: usize,
        invalidated_objects: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeControlDisposition {
    Handled(RuntimeControlEvent),
    Unhandled(pwright_transport::WireFrame),
}

/// Performs the backend side of one subscription edge.
///
/// The runtime calls this only for zero-to-one or one-to-zero transitions and
/// commits its logical subscription state only after the returned future
/// succeeds.
pub trait SubscriptionActivator {
    type Error;
    type Future<'a>: Future<Output = Result<(), Self::Error>>
    where
        Self: 'a;

    fn set_subscription<'a>(
        &'a self,
        object_id: ObjectId,
        event: EventTag,
        enabled: bool,
    ) -> Self::Future<'a>;
}

#[derive(Debug, Eq, PartialEq)]
pub struct SubscriptionActivationFailure<Activation> {
    pub object_id: ObjectId,
    pub event: EventTag,
    pub enabled: bool,
    pub source: Activation,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ActivatedRuntimeControlError<Control, Activation> {
    Control(Control),
    Activation {
        object_id: ObjectId,
        event: EventTag,
        enabled: bool,
        source: Activation,
    },
    TransportCloseCleanup {
        control: Option<Control>,
        first_activation: Option<SubscriptionActivationFailure<Activation>>,
        additional_activation_failures: usize,
    },
}

impl<Control, Activation> Display for ActivatedRuntimeControlError<Control, Activation>
where
    Control: Display,
    Activation: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Control(error) => Display::fmt(error, formatter),
            Self::Activation {
                object_id,
                event,
                enabled,
                source,
            } => write!(
                formatter,
                "backend subscription transition for object `{object_id}`, event `{event}`, enabled={enabled} failed: {source}"
            ),
            Self::TransportCloseCleanup {
                control,
                first_activation,
                additional_activation_failures,
            } => {
                formatter.write_str("transport-close cleanup failed")?;
                if let Some(error) = control {
                    write!(formatter, "; runtime close: {error}")?;
                }
                if let Some(error) = first_activation {
                    write!(
                        formatter,
                        "; backend deactivation for object `{}`, event `{}`: {}",
                        error.object_id, error.event, error.source
                    )?;
                    if *additional_activation_failures != 0 {
                        write!(
                            formatter,
                            " ({additional_activation_failures} additional failure(s))"
                        )?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl<Control, Activation> Error for ActivatedRuntimeControlError<Control, Activation>
where
    Control: Error + 'static,
    Activation: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Control(error) => Some(error),
            Self::Activation { source, .. } => Some(source),
            Self::TransportCloseCleanup {
                control,
                first_activation,
                additional_activation_failures: _,
            } => control
                .as_ref()
                .map(|error| error as &(dyn Error + 'static))
                .or_else(|| {
                    first_activation
                        .as_ref()
                        .map(|failure| &failure.source as &(dyn Error + 'static))
                }),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuntimeSubscriptionMetrics {
    active: usize,
    peak: usize,
    limit: usize,
    in_flight: bool,
}

impl RuntimeSubscriptionMetrics {
    pub const fn active(self) -> usize {
        self.active
    }

    pub const fn peak(self) -> usize {
        self.peak
    }

    pub const fn limit(self) -> usize {
        self.limit
    }

    pub const fn in_flight(self) -> bool {
        self.in_flight
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum RuntimeControlReceiveError<Codec> {
    Protocol(WireProtocolError<Codec>),
    Runtime(RuntimeStateError),
    InvalidDirection(ControlMessage),
    InvalidCancellationReason(CancellationReason),
    StaleObject(ObjectId),
    SubscriptionCapacityExceeded {
        limit: usize,
    },
    SubscriptionTransitionInProgress {
        object_id: ObjectId,
        event: EventTag,
        enabled: bool,
    },
    Resource(ResourceError),
}

impl<Codec> Display for RuntimeControlReceiveError<Codec>
where
    Codec: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => Display::fmt(error, formatter),
            Self::Runtime(error) => Display::fmt(error, formatter),
            Self::InvalidDirection(message) => write!(
                formatter,
                "control message {message:?} is not valid from client to runtime"
            ),
            Self::InvalidCancellationReason(reason) => write!(
                formatter,
                "cancellation reason {reason:?} is not valid from client to runtime"
            ),
            Self::StaleObject(object_id) => {
                write!(formatter, "subscription target `{object_id}` is not live")
            }
            Self::SubscriptionCapacityExceeded { limit } => {
                write!(formatter, "runtime subscription limit {limit} was reached")
            }
            Self::SubscriptionTransitionInProgress {
                object_id,
                event,
                enabled,
            } => write!(
                formatter,
                "backend subscription transition for object `{object_id}`, event `{event}`, enabled={enabled} is still in progress"
            ),
            Self::Resource(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Codec> Error for RuntimeControlReceiveError<Codec>
where
    Codec: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Protocol(error) => Some(error),
            Self::Runtime(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::InvalidDirection(_)
            | Self::InvalidCancellationReason(_)
            | Self::StaleObject(_)
            | Self::SubscriptionCapacityExceeded { .. }
            | Self::SubscriptionTransitionInProgress { .. } => None,
        }
    }
}

/// Production declaration for one control-protocol encode and every bounded
/// subscription route retained by its receiver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeControlProductionProfile {
    subscription_limit: usize,
    requirements: ResourceRequirements,
}

impl RuntimeControlProductionProfile {
    pub fn new<Codec>(protocol: &WireProtocol<Codec>, subscription_limit: usize) -> Self {
        let subscriptions = u64::try_from(subscription_limit).unwrap_or(u64::MAX);
        let retained = if subscriptions == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::EventItem, subscriptions, subscriptions)
                .with_metadata(subscription_limit, subscription_limit)
        };
        Self {
            subscription_limit,
            requirements: protocol
                .production_resource_requirements()
                .combine_concurrent(retained),
        }
    }

    pub const fn subscription_limit(self) -> usize {
        self.subscription_limit
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

/// Decodes client-to-runtime control frames and applies their runtime state
/// transitions. The optional activated wrapper couples subscription edges to
/// an asynchronous backend and retains one compensatable in-flight edge.
struct PendingSubscriptionTransition {
    object_id: ObjectId,
    event: EventTag,
    enabled: bool,
    resource: Option<ResourceLease>,
}

pub struct RuntimeControlReceiver<Runtime, Codec> {
    runtime: Runtime,
    protocol: WireProtocol<Codec>,
    subscriptions: BTreeMap<(ObjectId, EventTag), Option<ResourceLease>>,
    pending_subscription: Option<PendingSubscriptionTransition>,
    subscription_limit: usize,
    peak_subscriptions: usize,
    resource_ledger: Option<ResourceLedger>,
}

impl<Runtime, Codec> RuntimeControlReceiver<Runtime, Codec> {
    pub const fn new(
        runtime: Runtime,
        protocol: WireProtocol<Codec>,
        subscription_limit: usize,
    ) -> Self {
        Self {
            runtime,
            protocol,
            subscriptions: BTreeMap::new(),
            pending_subscription: None,
            subscription_limit,
            peak_subscriptions: 0,
            resource_ledger: None,
        }
    }

    pub fn new_with_production_resources(
        runtime: Runtime,
        protocol: WireProtocol<Codec>,
        subscription_limit: usize,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        let profile = RuntimeControlProductionProfile::new(&protocol, subscription_limit);
        let ledger = resources.claim(profile.requirements())?;
        Ok(Self {
            runtime,
            protocol: protocol.with_resource_ledger(ledger.clone()),
            subscriptions: BTreeMap::new(),
            pending_subscription: None,
            subscription_limit,
            peak_subscriptions: 0,
            resource_ledger: Some(ledger),
        })
    }

    pub fn subscription_metrics(&self) -> RuntimeSubscriptionMetrics {
        RuntimeSubscriptionMetrics {
            active: self.subscriptions.len(),
            peak: self.peak_subscriptions,
            limit: self.subscription_limit,
            in_flight: self.pending_subscription.is_some(),
        }
    }

    /// Forgets all routes owned by a disposed object. Returned routes let the
    /// caller deactivate any backend domains without retaining hidden work.
    pub fn forget_object(&mut self, object_id: ObjectId) -> Vec<EventTag> {
        let mut events = self
            .subscriptions
            .keys()
            .filter(|(candidate, _)| *candidate == object_id)
            .map(|(_, event)| *event)
            .collect::<Vec<_>>();
        if self
            .pending_subscription
            .as_ref()
            .is_some_and(|pending| pending.object_id == object_id)
        {
            let pending = self
                .pending_subscription
                .take()
                .expect("matching pending subscription should remain available");
            if !events.contains(&pending.event) {
                events.push(pending.event);
                events.sort_unstable();
            }
        }
        for event in &events {
            self.subscriptions.remove(&(object_id, *event));
        }
        events
    }
}

impl<Runtime, Codec> RuntimeControlReceiver<Runtime, Codec>
where
    Runtime: CancellationTarget,
    Codec: WireCodec<ControlMessage>,
{
    pub fn handle(
        &mut self,
        frame: pwright_transport::WireFrame,
    ) -> Result<RuntimeControlDisposition, RuntimeControlReceiveError<Codec::Error>> {
        if frame.route() != FrameRoute::Control {
            return Ok(RuntimeControlDisposition::Unhandled(frame));
        }
        if let Some(pending) = &self.pending_subscription {
            return Err(
                RuntimeControlReceiveError::SubscriptionTransitionInProgress {
                    object_id: pending.object_id,
                    event: pending.event,
                    enabled: pending.enabled,
                },
            );
        }
        let message = self
            .protocol
            .decode::<ControlMessage>(frame)
            .map_err(RuntimeControlReceiveError::Protocol)?;
        self.handle_message(message)
    }

    /// Handles one frame while making backend subscription activation part of
    /// the ordered control transaction. Logical state changes only after the
    /// backend future succeeds.
    pub async fn handle_with_activation<Activator>(
        &mut self,
        frame: pwright_transport::WireFrame,
        activator: &Activator,
    ) -> Result<
        RuntimeControlDisposition,
        ActivatedRuntimeControlError<RuntimeControlReceiveError<Codec::Error>, Activator::Error>,
    >
    where
        Activator: SubscriptionActivator,
    {
        if frame.route() != FrameRoute::Control {
            return Ok(RuntimeControlDisposition::Unhandled(frame));
        }
        if let Some(pending) = &self.pending_subscription {
            return Err(ActivatedRuntimeControlError::Control(
                RuntimeControlReceiveError::SubscriptionTransitionInProgress {
                    object_id: pending.object_id,
                    event: pending.event,
                    enabled: pending.enabled,
                },
            ));
        }
        let message = self
            .protocol
            .decode::<ControlMessage>(frame)
            .map_err(RuntimeControlReceiveError::Protocol)
            .map_err(ActivatedRuntimeControlError::Control)?;
        let ControlMessage::SetSubscription {
            object_id,
            event,
            enabled,
        } = message
        else {
            return self
                .handle_message(message)
                .map_err(ActivatedRuntimeControlError::Control);
        };

        let is_live = self
            .runtime
            .is_object_live(object_id)
            .map_err(RuntimeControlReceiveError::Runtime)
            .map_err(ActivatedRuntimeControlError::Control)?;
        if !is_live && enabled {
            return Err(ActivatedRuntimeControlError::Control(
                RuntimeControlReceiveError::StaleObject(object_id),
            ));
        }
        if !is_live {
            return Ok(RuntimeControlDisposition::Handled(
                RuntimeControlEvent::Subscription {
                    object_id,
                    event,
                    enabled,
                    changed: false,
                },
            ));
        }

        let key = (object_id, event);
        let changed = if enabled {
            if self.subscriptions.contains_key(&key) {
                false
            } else {
                if self.subscriptions.len() >= self.subscription_limit {
                    return Err(ActivatedRuntimeControlError::Control(
                        RuntimeControlReceiveError::SubscriptionCapacityExceeded {
                            limit: self.subscription_limit,
                        },
                    ));
                }
                let resource = self
                    .resource_ledger
                    .as_ref()
                    .map(|ledger| {
                        ledger.reserve(
                            ResourceScope::Object(object_id),
                            ResourceClass::EventItem,
                            1,
                        )
                    })
                    .transpose()
                    .map_err(RuntimeControlReceiveError::Resource)
                    .map_err(ActivatedRuntimeControlError::Control)?;
                self.pending_subscription = Some(PendingSubscriptionTransition {
                    object_id,
                    event,
                    enabled: true,
                    resource,
                });
                if let Err(source) = activator.set_subscription(object_id, event, true).await {
                    self.pending_subscription.take();
                    return Err(ActivatedRuntimeControlError::Activation {
                        object_id,
                        event,
                        enabled: true,
                        source,
                    });
                }
                let pending = self
                    .pending_subscription
                    .take()
                    .expect("successful activation should retain its pending transition");
                self.subscriptions.insert(key, pending.resource);
                self.peak_subscriptions = self.peak_subscriptions.max(self.subscriptions.len());
                true
            }
        } else if self.subscriptions.contains_key(&key) {
            self.pending_subscription = Some(PendingSubscriptionTransition {
                object_id,
                event,
                enabled: false,
                resource: None,
            });
            if let Err(source) = activator.set_subscription(object_id, event, false).await {
                self.pending_subscription.take();
                return Err(ActivatedRuntimeControlError::Activation {
                    object_id,
                    event,
                    enabled: false,
                    source,
                });
            }
            self.pending_subscription.take();
            self.subscriptions.remove(&key);
            true
        } else {
            false
        };
        Ok(RuntimeControlDisposition::Handled(
            RuntimeControlEvent::Subscription {
                object_id,
                event,
                enabled,
                changed,
            },
        ))
    }

    fn handle_message(
        &mut self,
        message: ControlMessage,
    ) -> Result<RuntimeControlDisposition, RuntimeControlReceiveError<Codec::Error>> {
        match message {
            ControlMessage::Create { .. }
            | ControlMessage::Adopt { .. }
            | ControlMessage::Dispose { .. }
            | ControlMessage::Close
            | ControlMessage::CloseAck => {
                Err(RuntimeControlReceiveError::InvalidDirection(message))
            }
            ControlMessage::Cancel {
                call_id,
                cancellation,
                reason,
            } => {
                if !matches!(
                    reason,
                    CancellationReason::Caller | CancellationReason::DeadlineExceeded
                ) {
                    return Err(RuntimeControlReceiveError::InvalidCancellationReason(
                        reason,
                    ));
                }
                let outcome = self
                    .runtime
                    .cancel_call(call_id, cancellation, reason)
                    .map_err(RuntimeControlReceiveError::Runtime)?;
                Ok(RuntimeControlDisposition::Handled(
                    RuntimeControlEvent::Cancellation { call_id, outcome },
                ))
            }
            ControlMessage::SetSubscription {
                object_id,
                event,
                enabled,
            } => {
                let is_live = self
                    .runtime
                    .is_object_live(object_id)
                    .map_err(RuntimeControlReceiveError::Runtime)?;
                if !is_live && enabled {
                    return Err(RuntimeControlReceiveError::StaleObject(object_id));
                }
                if !is_live {
                    return Ok(RuntimeControlDisposition::Handled(
                        RuntimeControlEvent::Subscription {
                            object_id,
                            event,
                            enabled,
                            changed: false,
                        },
                    ));
                }
                let key = (object_id, event);
                let changed = if enabled {
                    if self.subscriptions.contains_key(&key) {
                        false
                    } else {
                        if self.subscriptions.len() >= self.subscription_limit {
                            return Err(RuntimeControlReceiveError::SubscriptionCapacityExceeded {
                                limit: self.subscription_limit,
                            });
                        }
                        let resource = self
                            .resource_ledger
                            .as_ref()
                            .map(|ledger| {
                                ledger.reserve(
                                    ResourceScope::Object(object_id),
                                    ResourceClass::EventItem,
                                    1,
                                )
                            })
                            .transpose()
                            .map_err(RuntimeControlReceiveError::Resource)?;
                        self.subscriptions.insert(key, resource);
                        self.peak_subscriptions =
                            self.peak_subscriptions.max(self.subscriptions.len());
                        true
                    }
                } else {
                    self.subscriptions.remove(&key).is_some()
                };
                Ok(RuntimeControlDisposition::Handled(
                    RuntimeControlEvent::Subscription {
                        object_id,
                        event,
                        enabled,
                        changed,
                    },
                ))
            }
        }
    }
}

impl<Runtime, Codec, Transport> ServerFrameHandler<Transport>
    for RuntimeControlReceiver<Runtime, Codec>
where
    Runtime: CancellationTarget,
    Codec: WireCodec<ControlMessage>,
{
    type Event = RuntimeControlEvent;
    type Error = RuntimeControlReceiveError<<Codec as WireCodec<ControlMessage>>::Error>;
    type Future<'a>
        = std::future::Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>
    where
        Self: 'a,
        Transport: 'a;

    fn handle<'a>(
        &'a mut self,
        frame: pwright_transport::WireFrame,
        _transport: &'a Transport,
    ) -> Self::Future<'a> {
        std::future::ready(
            RuntimeControlReceiver::handle(self, frame).map(|disposition| match disposition {
                RuntimeControlDisposition::Handled(event) => ServerFrameDisposition::Handled(event),
                RuntimeControlDisposition::Unhandled(frame) => {
                    ServerFrameDisposition::Unhandled(frame)
                }
            }),
        )
    }
}

impl<Runtime, Codec> ServerControlFrameHandler for RuntimeControlReceiver<Runtime, Codec>
where
    Runtime: CancellationTarget + TransportCloseTarget,
    Codec: WireCodec<ControlMessage>,
{
    type Event = RuntimeControlEvent;
    type Error = RuntimeControlReceiveError<<Codec as WireCodec<ControlMessage>>::Error>;
    type Future<'a>
        = std::future::Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>
    where
        Self: 'a;
    type CloseFuture<'a>
        = std::future::Ready<Result<Option<Self::Event>, Self::Error>>
    where
        Self: 'a;

    fn handle_control<'a>(&'a mut self, frame: pwright_transport::WireFrame) -> Self::Future<'a> {
        std::future::ready(
            RuntimeControlReceiver::handle(self, frame).map(|disposition| match disposition {
                RuntimeControlDisposition::Handled(event) => ServerFrameDisposition::Handled(event),
                RuntimeControlDisposition::Unhandled(frame) => {
                    ServerFrameDisposition::Unhandled(frame)
                }
            }),
        )
    }

    fn releases_frame_before_pending(&self) -> bool {
        true
    }

    fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
        let outcome = self
            .runtime
            .close_transport()
            .map_err(RuntimeControlReceiveError::Runtime)
            .map(|outcome| RuntimeControlEvent::TransportClosed {
                already_closed: outcome.already_closed(),
                pending_calls: outcome.pending_calls(),
                invalidated_objects: outcome.invalidated_objects().len(),
            });
        if outcome.is_ok() {
            self.pending_subscription.take();
            self.subscriptions.clear();
        }
        std::future::ready(outcome.map(Some))
    }
}

/// Ordered control handler that transactionally couples runtime subscription
/// state with an async backend activator.
pub struct ActivatedRuntimeControlReceiver<Runtime, Codec, Activator> {
    receiver: RuntimeControlReceiver<Runtime, Codec>,
    activator: Activator,
}

impl<Runtime, Codec, Activator> ActivatedRuntimeControlReceiver<Runtime, Codec, Activator> {
    pub const fn new(
        receiver: RuntimeControlReceiver<Runtime, Codec>,
        activator: Activator,
    ) -> Self {
        Self {
            receiver,
            activator,
        }
    }

    pub const fn receiver(&self) -> &RuntimeControlReceiver<Runtime, Codec> {
        &self.receiver
    }

    pub const fn receiver_mut(&mut self) -> &mut RuntimeControlReceiver<Runtime, Codec> {
        &mut self.receiver
    }

    pub const fn activator(&self) -> &Activator {
        &self.activator
    }

    pub fn into_parts(self) -> (RuntimeControlReceiver<Runtime, Codec>, Activator) {
        (self.receiver, self.activator)
    }
}

impl<Runtime, Codec, Activator> ActivatedRuntimeControlReceiver<Runtime, Codec, Activator>
where
    Codec: WireCodec<ControlMessage>,
    Activator: SubscriptionActivator,
{
    /// Deactivates and forgets every subscription owned by one disposed
    /// object. Each successful edge is committed independently. A failed edge
    /// and every later edge remain retained so the caller can retry cleanup.
    pub async fn deactivate_and_forget_object(
        &mut self,
        object_id: ObjectId,
    ) -> Result<
        usize,
        ActivatedRuntimeControlError<RuntimeControlReceiveError<Codec::Error>, Activator::Error>,
    > {
        let mut deactivated = 0usize;
        if let Some((event, enabled)) = self
            .receiver
            .pending_subscription
            .as_ref()
            .filter(|pending| pending.object_id == object_id)
            .map(|pending| (pending.event, pending.enabled))
        {
            self.activator
                .set_subscription(object_id, event, false)
                .await
                .map_err(|source| ActivatedRuntimeControlError::Activation {
                    object_id,
                    event,
                    enabled: false,
                    source,
                })?;
            self.receiver.pending_subscription.take();
            if !enabled {
                self.receiver.subscriptions.remove(&(object_id, event));
            }
            deactivated = deactivated.saturating_add(1);
        }
        while let Some((_, event)) = self
            .receiver
            .subscriptions
            .keys()
            .find(|(candidate, _)| *candidate == object_id)
            .copied()
        {
            self.activator
                .set_subscription(object_id, event, false)
                .await
                .map_err(|source| ActivatedRuntimeControlError::Activation {
                    object_id,
                    event,
                    enabled: false,
                    source,
                })?;
            self.receiver.subscriptions.remove(&(object_id, event));
            deactivated = deactivated.saturating_add(1);
        }
        Ok(deactivated)
    }
}

impl<Runtime, Codec, Activator> ActivatedRuntimeControlReceiver<Runtime, Codec, Activator>
where
    Runtime: TransportCloseTarget,
    Codec: WireCodec<ControlMessage>,
    Activator: SubscriptionActivator,
{
    async fn close_transport_with_activation(
        &mut self,
    ) -> Result<
        RuntimeControlEvent,
        ActivatedRuntimeControlError<RuntimeControlReceiveError<Codec::Error>, Activator::Error>,
    > {
        let runtime_close = self.receiver.runtime.close_transport();
        let mut first_activation = None;
        let mut additional_activation_failures = 0usize;
        if let Some(pending) = self.receiver.pending_subscription.as_ref() {
            let object_id = pending.object_id;
            let event = pending.event;
            let result = self
                .activator
                .set_subscription(object_id, event, false)
                .await;
            self.receiver.pending_subscription.take();
            self.receiver.subscriptions.remove(&(object_id, event));
            if let Err(source) = result {
                first_activation = Some(SubscriptionActivationFailure {
                    object_id,
                    event,
                    enabled: false,
                    source,
                });
            }
        }
        while let Some((object_id, event)) = self.receiver.subscriptions.keys().next().copied() {
            let result = self
                .activator
                .set_subscription(object_id, event, false)
                .await;
            self.receiver.subscriptions.remove(&(object_id, event));
            if let Err(source) = result {
                let failure = SubscriptionActivationFailure {
                    object_id,
                    event,
                    enabled: false,
                    source,
                };
                if first_activation.is_none() {
                    first_activation = Some(failure);
                } else {
                    additional_activation_failures =
                        additional_activation_failures.saturating_add(1);
                }
            }
        }

        let (outcome, control) = match runtime_close {
            Ok(outcome) => (Some(outcome), None),
            Err(error) => (None, Some(RuntimeControlReceiveError::Runtime(error))),
        };
        if control.is_some() || first_activation.is_some() {
            return Err(ActivatedRuntimeControlError::TransportCloseCleanup {
                control,
                first_activation,
                additional_activation_failures,
            });
        }
        let outcome = outcome.expect("successful runtime close should retain its outcome");
        Ok(RuntimeControlEvent::TransportClosed {
            already_closed: outcome.already_closed(),
            pending_calls: outcome.pending_calls(),
            invalidated_objects: outcome.invalidated_objects().len(),
        })
    }
}

impl<Runtime, Codec, Activator> ServerControlFrameHandler
    for ActivatedRuntimeControlReceiver<Runtime, Codec, Activator>
where
    Runtime: CancellationTarget + TransportCloseTarget,
    Codec: WireCodec<ControlMessage>,
    Activator: SubscriptionActivator,
{
    type Event = RuntimeControlEvent;
    type Error = ActivatedRuntimeControlError<
        RuntimeControlReceiveError<<Codec as WireCodec<ControlMessage>>::Error>,
        Activator::Error,
    >;
    type Future<'a>
        =
        Pin<Box<dyn Future<Output = Result<ServerFrameDisposition<Self::Event>, Self::Error>> + 'a>>
    where
        Self: 'a;
    type CloseFuture<'a>
        = Pin<Box<dyn Future<Output = Result<Option<Self::Event>, Self::Error>> + 'a>>
    where
        Self: 'a;

    fn handle_control<'a>(&'a mut self, frame: pwright_transport::WireFrame) -> Self::Future<'a> {
        Box::pin(async move {
            self.receiver
                .handle_with_activation(frame, &self.activator)
                .await
                .map(|disposition| match disposition {
                    RuntimeControlDisposition::Handled(event) => {
                        ServerFrameDisposition::Handled(event)
                    }
                    RuntimeControlDisposition::Unhandled(frame) => {
                        ServerFrameDisposition::Unhandled(frame)
                    }
                })
        })
    }

    fn releases_frame_before_pending(&self) -> bool {
        true
    }

    fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
        Box::pin(async move { self.close_transport_with_activation().await.map(Some) })
    }
}

/// Supplies exclusive access to a runtime's oldest lifecycle message.
pub trait ControlMessageSource {
    fn claim_control_message(&self) -> Result<Option<ControlMessageClaim>, RuntimeStateError>;
}

impl<S> ControlMessageSource for DirectDispatcher<S> {
    fn claim_control_message(&self) -> Result<Option<ControlMessageClaim>, RuntimeStateError> {
        DirectDispatcher::claim_control_message(self)
    }
}

impl<S, Cancellation, Deadlines, Cleanup> ControlMessageSource
    for ControlledDispatcher<S, Cancellation, Deadlines, Cleanup>
{
    fn claim_control_message(&self) -> Result<Option<ControlMessageClaim>, RuntimeStateError> {
        ControlledDispatcher::claim_control_message(self)
    }
}

impl<Source> ControlMessageSource for std::sync::Arc<Source>
where
    Source: ControlMessageSource + ?Sized,
{
    fn claim_control_message(&self) -> Result<Option<ControlMessageClaim>, RuntimeStateError> {
        Source::claim_control_message(self)
    }
}

impl<Source> ControlMessageSource for &Source
where
    Source: ControlMessageSource + ?Sized,
{
    fn claim_control_message(&self) -> Result<Option<ControlMessageClaim>, RuntimeStateError> {
        Source::claim_control_message(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlSendOutcome {
    Idle,
    Sent(ControlMessage),
}

#[derive(Debug, Eq, PartialEq)]
pub enum RuntimeControlError<Codec, Transport> {
    Runtime(RuntimeStateError),
    Protocol(WireProtocolError<Codec>),
    InvalidRoute(FrameRoute),
    Transport(Transport),
}

impl<Codec, Transport> Display for RuntimeControlError<Codec, Transport>
where
    Codec: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(error) => Display::fmt(error, formatter),
            Self::Protocol(error) => Display::fmt(error, formatter),
            Self::InvalidRoute(route) => {
                write!(
                    formatter,
                    "lifecycle codec produced invalid route {route:?}"
                )
            }
            Self::Transport(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Codec, Transport> Error for RuntimeControlError<Codec, Transport>
where
    Codec: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Runtime(error) => Some(error),
            Self::Protocol(error) => Some(error),
            Self::InvalidRoute(_) => None,
            Self::Transport(error) => Some(error),
        }
    }
}

/// A single-consumer control-plane driver composed from a codec and transport.
pub struct RuntimeControlDriver<Codec, Transport> {
    protocol: WireProtocol<Codec>,
    transport: Transport,
}

/// Adapts a lifecycle source and its control driver into the supervisor's
/// optional fair outbound lane.
pub struct RuntimeControlOutbound<Source, Codec, Transport> {
    source: Source,
    driver: RuntimeControlDriver<Codec, Transport>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OrderedControlPriority {
    Barrier,
    Outbound,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderedControlGateError {
    ZeroWaiterLimit,
    CapacityExceeded { limit: usize },
    TicketExhausted,
    StateUnavailable,
}

impl Display for OrderedControlGateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroWaiterLimit => {
                formatter.write_str("ordered runtime control requires at least one writer slot")
            }
            Self::CapacityExceeded { limit } => write!(
                formatter,
                "ordered runtime control writer limit of {limit} was reached"
            ),
            Self::TicketExhausted => {
                formatter.write_str("ordered runtime control writer tickets were exhausted")
            }
            Self::StateUnavailable => {
                formatter.write_str("ordered runtime control writer state is unavailable")
            }
        }
    }
}

impl Error for OrderedControlGateError {}

struct OrderedControlWaiter {
    ticket: u64,
    waker: Option<Waker>,
}

struct OrderedControlGateState {
    active: Option<u64>,
    barriers: VecDeque<OrderedControlWaiter>,
    outbound: VecDeque<OrderedControlWaiter>,
    next_ticket: u64,
}

impl OrderedControlGateState {
    fn retained_writers(&self) -> usize {
        usize::from(self.active.is_some())
            .saturating_add(self.barriers.len())
            .saturating_add(self.outbound.len())
    }

    fn promote(&mut self) -> Option<Waker> {
        if self.active.is_some() {
            return None;
        }
        let waiter = self
            .barriers
            .pop_front()
            .or_else(|| self.outbound.pop_front())?;
        self.active = Some(waiter.ticket);
        waiter.waker
    }

    fn waiter_mut(&mut self, ticket: u64) -> Option<&mut OrderedControlWaiter> {
        self.barriers
            .iter_mut()
            .chain(self.outbound.iter_mut())
            .find(|waiter| waiter.ticket == ticket)
    }

    fn remove_waiter(&mut self, ticket: u64) -> bool {
        for waiters in [&mut self.barriers, &mut self.outbound] {
            if let Some(index) = waiters.iter().position(|waiter| waiter.ticket == ticket) {
                waiters.remove(index);
                return true;
            }
        }
        false
    }
}

struct OrderedControlGate {
    state: Mutex<OrderedControlGateState>,
    waiter_limit: usize,
}

impl OrderedControlGate {
    fn new(waiter_limit: usize) -> Result<Self, OrderedControlGateError> {
        if waiter_limit == 0 {
            return Err(OrderedControlGateError::ZeroWaiterLimit);
        }
        Ok(Self {
            state: Mutex::new(OrderedControlGateState {
                active: None,
                barriers: VecDeque::with_capacity(waiter_limit),
                outbound: VecDeque::with_capacity(waiter_limit),
                next_ticket: 0,
            }),
            waiter_limit,
        })
    }

    fn acquire(
        self: &Arc<Self>,
        priority: OrderedControlPriority,
    ) -> Result<OrderedControlAcquire, OrderedControlGateError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| OrderedControlGateError::StateUnavailable)?;
        if state.retained_writers() >= self.waiter_limit {
            return Err(OrderedControlGateError::CapacityExceeded {
                limit: self.waiter_limit,
            });
        }
        let ticket = state.next_ticket;
        state.next_ticket = state
            .next_ticket
            .checked_add(1)
            .ok_or(OrderedControlGateError::TicketExhausted)?;
        let waiter = OrderedControlWaiter {
            ticket,
            waker: None,
        };
        match priority {
            OrderedControlPriority::Barrier => state.barriers.push_back(waiter),
            OrderedControlPriority::Outbound => state.outbound.push_back(waiter),
        }
        let wake = state.promote();
        drop(state);
        if let Some(waker) = wake {
            waker.wake();
        }
        Ok(OrderedControlAcquire {
            gate: Arc::clone(self),
            ticket,
            completed: false,
        })
    }

    fn release(&self, ticket: u64) {
        let wake = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if state.active == Some(ticket) {
                state.active = None;
            } else if !state.remove_waiter(ticket) {
                return;
            }
            state.promote()
        };
        if let Some(waker) = wake {
            waker.wake();
        }
    }
}

struct OrderedControlAcquire {
    gate: Arc<OrderedControlGate>,
    ticket: u64,
    completed: bool,
}

impl Future for OrderedControlAcquire {
    type Output = Result<OrderedControlPermit, OrderedControlGateError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = match self.gate.state.lock() {
            Ok(state) => state,
            Err(_) => return Poll::Ready(Err(OrderedControlGateError::StateUnavailable)),
        };
        if state.active == Some(self.ticket) {
            drop(state);
            self.completed = true;
            return Poll::Ready(Ok(OrderedControlPermit {
                gate: Arc::clone(&self.gate),
                ticket: self.ticket,
            }));
        }
        let Some(waiter) = state.waiter_mut(self.ticket) else {
            return Poll::Ready(Err(OrderedControlGateError::StateUnavailable));
        };
        if waiter
            .waker
            .as_ref()
            .is_none_or(|registered| !registered.will_wake(context.waker()))
        {
            waiter.waker = Some(context.waker().clone());
        }
        Poll::Pending
    }
}

impl Drop for OrderedControlAcquire {
    fn drop(&mut self) {
        if !self.completed {
            self.gate.release(self.ticket);
        }
    }
}

struct OrderedControlPermit {
    gate: Arc<OrderedControlGate>,
    ticket: u64,
}

impl Drop for OrderedControlPermit {
    fn drop(&mut self) {
        self.gate.release(self.ticket);
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum OrderedRuntimeControlError<Codec, Transport> {
    Coordination(OrderedControlGateError),
    Control(RuntimeControlError<Codec, Transport>),
    CreationFenceMissing { object_id: ObjectId },
    DisposalFenceMissing { object_id: ObjectId },
}

impl<Codec, Transport> Display for OrderedRuntimeControlError<Codec, Transport>
where
    Codec: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coordination(error) => Display::fmt(error, formatter),
            Self::Control(error) => Display::fmt(error, formatter),
            Self::CreationFenceMissing { object_id } => write!(
                formatter,
                "runtime control outbox became idle before creation of `{object_id}`"
            ),
            Self::DisposalFenceMissing { object_id } => write!(
                formatter,
                "runtime control outbox became idle before disposal of `{object_id}`"
            ),
        }
    }
}

impl<Codec, Transport> Error for OrderedRuntimeControlError<Codec, Transport>
where
    Codec: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Coordination(error) => Some(error),
            Self::Control(error) => Some(error),
            Self::CreationFenceMissing { .. } | Self::DisposalFenceMissing { .. } => None,
        }
    }
}

struct OrderedRuntimeControlShared<Source, Codec, Transport> {
    source: Source,
    driver: RuntimeControlDriver<Codec, Transport>,
    gate: Arc<OrderedControlGate>,
}

/// One bounded writer coordinator shared by the ordinary lifecycle lane and
/// command-completion barriers.
///
/// A barrier has priority over queued ordinary sends and holds exclusive
/// ownership across a complete control sequence. This lets a command prove
/// that its terminal lifecycle message was accepted by the transport before
/// its typed result is written, without selecting an executor or a runtime.
pub struct OrderedRuntimeControl<Source, Codec, Transport> {
    shared: Arc<OrderedRuntimeControlShared<Source, Codec, Transport>>,
}

impl<Source, Codec, Transport> Clone for OrderedRuntimeControl<Source, Codec, Transport> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<Source, Codec, Transport> OrderedRuntimeControl<Source, Codec, Transport> {
    pub fn new(
        source: Source,
        driver: RuntimeControlDriver<Codec, Transport>,
        waiter_limit: usize,
    ) -> Result<Self, OrderedControlGateError> {
        Ok(Self {
            shared: Arc::new(OrderedRuntimeControlShared {
                source,
                driver,
                gate: Arc::new(OrderedControlGate::new(waiter_limit)?),
            }),
        })
    }

    pub fn waiter_limit(&self) -> usize {
        self.shared.gate.waiter_limit
    }

    pub async fn begin_barrier(
        &self,
    ) -> Result<OrderedRuntimeControlBarrier<'_, Source, Codec, Transport>, OrderedControlGateError>
    {
        let permit = self
            .shared
            .gate
            .acquire(OrderedControlPriority::Barrier)?
            .await?;
        Ok(OrderedRuntimeControlBarrier {
            control: self,
            _permit: permit,
        })
    }
}

/// Exclusive access to an ordered runtime-control sequence.
pub struct OrderedRuntimeControlBarrier<'a, Source, Codec, Transport> {
    control: &'a OrderedRuntimeControl<Source, Codec, Transport>,
    _permit: OrderedControlPermit,
}

impl<Source, Codec, Transport> OrderedRuntimeControlBarrier<'_, Source, Codec, Transport>
where
    Source: ControlMessageSource,
    Codec: WireCodec<ControlMessage>,
    Transport: FrameTransport,
{
    /// Sends control messages through the exact object creation. The complete
    /// initializer is matched so a stale or malformed `Create` cannot release
    /// a typed command result for a different identity.
    pub async fn send_through_creation(
        &self,
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: pwright_core::ObjectKindTag,
    ) -> Result<usize, OrderedRuntimeControlError<Codec::Error, Transport::Error>> {
        let expected = ControlMessage::Create {
            object_id,
            parent,
            kind,
        };
        let mut sent = 0usize;
        loop {
            match self
                .control
                .shared
                .driver
                .send_once(&self.control.shared.source)
                .await
                .map_err(OrderedRuntimeControlError::Control)?
            {
                ControlSendOutcome::Idle => {
                    return Err(OrderedRuntimeControlError::CreationFenceMissing { object_id });
                }
                ControlSendOutcome::Sent(message) => {
                    sent = sent.saturating_add(1);
                    if message == expected {
                        return Ok(sent);
                    }
                }
            }
        }
    }

    /// Sends every message that precedes the exact target disposal while
    /// leaving that disposal at the front of the runtime outbox.
    ///
    /// Terminal object events can use this split fence to establish
    /// descendant-disposal -> terminal-event -> target-disposal ordering on
    /// one transport. The barrier remains held across all three phases.
    pub async fn send_before_disposal(
        &self,
        object_id: ObjectId,
    ) -> Result<usize, OrderedRuntimeControlError<Codec::Error, Transport::Error>> {
        let expected = ControlMessage::Dispose { object_id };
        let mut sent = 0usize;
        loop {
            let Some(claim) = self
                .control
                .shared
                .source
                .claim_control_message()
                .map_err(RuntimeControlError::Runtime)
                .map_err(OrderedRuntimeControlError::Control)?
            else {
                return Err(OrderedRuntimeControlError::DisposalFenceMissing { object_id });
            };
            let message = claim.message();
            drop(claim);
            if message == expected {
                return Ok(sent);
            }
            match self
                .control
                .shared
                .driver
                .send_once(&self.control.shared.source)
                .await
                .map_err(OrderedRuntimeControlError::Control)?
            {
                ControlSendOutcome::Idle => {
                    return Err(OrderedRuntimeControlError::DisposalFenceMissing { object_id });
                }
                ControlSendOutcome::Sent(_) => sent = sent.saturating_add(1),
            }
        }
    }

    /// Sends control messages through the exact target disposal. Becoming
    /// idle first is an invariant failure: the command must not publish its
    /// successful result without its lifecycle fence.
    pub async fn send_through_disposal(
        &self,
        object_id: ObjectId,
    ) -> Result<usize, OrderedRuntimeControlError<Codec::Error, Transport::Error>> {
        let mut sent = 0usize;
        loop {
            match self
                .control
                .shared
                .driver
                .send_once(&self.control.shared.source)
                .await
                .map_err(OrderedRuntimeControlError::Control)?
            {
                ControlSendOutcome::Idle => {
                    return Err(OrderedRuntimeControlError::DisposalFenceMissing { object_id });
                }
                ControlSendOutcome::Sent(message) => {
                    sent = sent.saturating_add(1);
                    if message == (ControlMessage::Dispose { object_id }) {
                        return Ok(sent);
                    }
                }
            }
        }
    }
}

impl<Source, Codec, Transport> ConcurrentServerOutbound
    for OrderedRuntimeControl<Source, Codec, Transport>
where
    Source: ControlMessageSource + Send + Sync,
    Codec: WireCodec<ControlMessage>,
    Transport: FrameTransport,
{
    type Event = ControlMessage;
    type Error = OrderedRuntimeControlError<Codec::Error, Transport::Error>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Option<Self::Event>, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn send_once(&self) -> Self::Future<'_> {
        Box::pin(async move {
            let acquire = match self.shared.gate.acquire(OrderedControlPriority::Outbound) {
                Ok(acquire) => acquire,
                Err(error) => {
                    return Err(OrderedRuntimeControlError::Coordination(error));
                }
            };
            let _permit = acquire
                .await
                .map_err(OrderedRuntimeControlError::Coordination)?;
            match self
                .shared
                .driver
                .send_once(&self.shared.source)
                .await
                .map_err(OrderedRuntimeControlError::Control)?
            {
                ControlSendOutcome::Idle => Ok(None),
                ControlSendOutcome::Sent(message) => Ok(Some(message)),
            }
        })
    }
}

impl<Source, Codec, Transport> RuntimeControlOutbound<Source, Codec, Transport> {
    pub const fn new(source: Source, driver: RuntimeControlDriver<Codec, Transport>) -> Self {
        Self { source, driver }
    }

    pub const fn source(&self) -> &Source {
        &self.source
    }

    pub const fn driver(&self) -> &RuntimeControlDriver<Codec, Transport> {
        &self.driver
    }

    pub fn into_parts(self) -> (Source, RuntimeControlDriver<Codec, Transport>) {
        (self.source, self.driver)
    }
}

impl<Source, Codec, Transport> ConcurrentServerOutbound
    for RuntimeControlOutbound<Source, Codec, Transport>
where
    Source: ControlMessageSource + Sync,
    Codec: WireCodec<ControlMessage>,
    Transport: FrameTransport,
{
    type Event = ControlMessage;
    type Error = RuntimeControlError<Codec::Error, Transport::Error>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Option<Self::Event>, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn send_once(&self) -> Self::Future<'_> {
        Box::pin(async move {
            match self.driver.send_once(&self.source).await? {
                ControlSendOutcome::Idle => Ok(None),
                ControlSendOutcome::Sent(message) => Ok(Some(message)),
            }
        })
    }
}

impl<Codec, Transport> RuntimeControlDriver<Codec, Transport> {
    pub const fn new(protocol: WireProtocol<Codec>, transport: Transport) -> Self {
        Self {
            protocol,
            transport,
        }
    }

    pub const fn protocol(&self) -> &WireProtocol<Codec> {
        &self.protocol
    }

    pub const fn transport(&self) -> &Transport {
        &self.transport
    }

    pub fn into_parts(self) -> (WireProtocol<Codec>, Transport) {
        (self.protocol, self.transport)
    }
}

impl<Codec, Transport> RuntimeControlDriver<Codec, Transport>
where
    Codec: WireCodec<ControlMessage>,
    Transport: FrameTransport,
{
    /// Sends at most one lifecycle frame and commits its runtime claim only
    /// after the transport reports success.
    pub async fn send_once<Source>(
        &self,
        source: &Source,
    ) -> Result<ControlSendOutcome, RuntimeControlError<Codec::Error, Transport::Error>>
    where
        Source: ControlMessageSource,
    {
        let Some(claim) = source
            .claim_control_message()
            .map_err(RuntimeControlError::Runtime)?
        else {
            return Ok(ControlSendOutcome::Idle);
        };
        let message = claim.message();
        let frame = self
            .protocol
            .encode(&message)
            .map_err(RuntimeControlError::Protocol)?;
        if frame.route() != FrameRoute::Control {
            return Err(RuntimeControlError::InvalidRoute(frame.route()));
        }
        self.transport
            .send(frame)
            .await
            .map_err(RuntimeControlError::Transport)?;
        claim.commit().map_err(RuntimeControlError::Runtime)?;
        Ok(ControlSendOutcome::Sent(message))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::error::Error;
    use std::fmt::{self, Display, Formatter};
    use std::future::{Future, Ready, ready};
    use std::sync::{Arc, Mutex};
    use std::task::Poll;

    use pwright_core::{ControlMessage, ObjectId, ObjectKindTag, ProtocolVersion};
    use pwright_runtime::{DirectDispatcher, RuntimeLimits};
    use pwright_server_wire::ConcurrentServerOutbound;
    use pwright_transport::{
        EncodedParts, FrameLimits, FrameRoute, FrameTransport, VersionPolicy, WireCodec, WireFrame,
        WireProtocol, WireProtocolError,
    };

    use super::{
        ControlSendOutcome, OrderedControlGateError, OrderedRuntimeControl,
        OrderedRuntimeControlError, RuntimeControlDriver, RuntimeControlError,
    };

    struct UnusedService;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum CodecError {
        Rejected,
    }

    impl Display for CodecError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("codec rejected lifecycle message")
        }
    }

    impl Error for CodecError {}

    #[derive(Clone, Copy)]
    struct TestCodec {
        fail: bool,
        route: FrameRoute,
    }

    impl WireCodec<ControlMessage> for TestCodec {
        type Error = CodecError;

        fn encode_parts(&self, message: &ControlMessage) -> Result<EncodedParts, Self::Error> {
            if self.fail {
                return Err(CodecError::Rejected);
            }
            let object_id = match message {
                ControlMessage::Create { object_id, .. }
                | ControlMessage::Adopt { object_id, .. }
                | ControlMessage::Dispose { object_id }
                | ControlMessage::SetSubscription { object_id, .. } => object_id.get(),
                ControlMessage::Cancel { call_id, .. } => call_id.get(),
                ControlMessage::Close | ControlMessage::CloseAck => 0,
            };
            Ok(EncodedParts::new(
                self.route,
                object_id.to_le_bytes().to_vec(),
                Vec::new(),
            ))
        }

        fn decode_frame(&self, _frame: WireFrame) -> Result<ControlMessage, Self::Error> {
            Err(CodecError::Rejected)
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum TransportError {
        Rejected,
    }

    impl Display for TransportError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("transport rejected frame")
        }
    }

    impl Error for TransportError {}

    struct RecordingTransport {
        sent: Mutex<VecDeque<WireFrame>>,
        fail: bool,
    }

    impl RecordingTransport {
        fn new(fail: bool) -> Self {
            Self {
                sent: Mutex::new(VecDeque::new()),
                fail,
            }
        }
    }

    impl FrameTransport for RecordingTransport {
        type Error = TransportError;
        type Send<'a> = Ready<Result<(), Self::Error>>;
        type Receive<'a> = Ready<Result<Option<WireFrame>, Self::Error>>;

        fn send(&self, frame: WireFrame) -> Self::Send<'_> {
            if self.fail {
                return ready(Err(TransportError::Rejected));
            }
            self.sent
                .lock()
                .expect("sent frames should be available")
                .push_back(frame);
            ready(Ok(()))
        }

        fn receive(&self) -> Self::Receive<'_> {
            ready(Ok(None))
        }
    }

    fn protocol(codec: TestCodec) -> WireProtocol<TestCodec> {
        WireProtocol::new(
            codec,
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("version policy should build"),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
        )
        .expect("wire protocol should build")
    }

    fn runtime() -> DirectDispatcher<UnusedService> {
        DirectDispatcher::new(
            UnusedService,
            RuntimeLimits::new(1, 1).with_control_messages(1),
        )
    }

    #[test]
    fn successful_send_commits_the_runtime_message() {
        // Arrange
        let runtime = Arc::new(runtime());
        runtime
            .register_object(ObjectId::new(7), None, ObjectKindTag::new(3))
            .expect("page should register");
        let driver = RuntimeControlDriver::new(
            protocol(TestCodec {
                fail: false,
                route: FrameRoute::Control,
            }),
            RecordingTransport::new(false),
        );

        // Act
        let outcome = futures_lite_block_on(driver.send_once(&runtime));

        // Assert
        assert_eq!(
            outcome,
            Ok(ControlSendOutcome::Sent(ControlMessage::Create {
                object_id: ObjectId::new(7),
                parent: None,
                kind: ObjectKindTag::new(3),
            }))
        );
        assert_eq!(runtime.pop_control_message(), Ok(None));
        let sent = driver
            .transport()
            .sent
            .lock()
            .expect("sent frames should be available");
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].route(), FrameRoute::Control);
    }

    #[test]
    fn encoding_failure_rolls_the_runtime_claim_back() {
        // Arrange
        let runtime = runtime();
        runtime
            .register_object(ObjectId::new(7), None, ObjectKindTag::new(3))
            .expect("page should register");
        let driver = RuntimeControlDriver::new(
            protocol(TestCodec {
                fail: true,
                route: FrameRoute::Control,
            }),
            RecordingTransport::new(false),
        );

        // Act
        let outcome = futures_lite_block_on(driver.send_once(&runtime));

        // Assert
        assert_eq!(
            outcome,
            Err(RuntimeControlError::Protocol(WireProtocolError::Codec(
                CodecError::Rejected
            )))
        );
        assert!(
            runtime
                .pop_control_message()
                .expect("control queue should load")
                .is_some()
        );
    }

    #[test]
    fn invalid_route_rolls_the_runtime_claim_back() {
        // Arrange
        let runtime = runtime();
        runtime
            .register_object(ObjectId::new(7), None, ObjectKindTag::new(3))
            .expect("page should register");
        let route = FrameRoute::Handshake;
        let driver = RuntimeControlDriver::new(
            protocol(TestCodec { fail: false, route }),
            RecordingTransport::new(false),
        );

        // Act
        let outcome = futures_lite_block_on(driver.send_once(&runtime));

        // Assert
        assert_eq!(outcome, Err(RuntimeControlError::InvalidRoute(route)));
        assert!(
            runtime
                .pop_control_message()
                .expect("control queue should load")
                .is_some()
        );
    }

    #[test]
    fn transport_failure_rolls_the_runtime_claim_back() {
        // Arrange
        let runtime = runtime();
        runtime
            .register_object(ObjectId::new(7), None, ObjectKindTag::new(3))
            .expect("page should register");
        let driver = RuntimeControlDriver::new(
            protocol(TestCodec {
                fail: false,
                route: FrameRoute::Control,
            }),
            RecordingTransport::new(true),
        );

        // Act
        let outcome = futures_lite_block_on(driver.send_once(&runtime));

        // Assert
        assert_eq!(
            outcome,
            Err(RuntimeControlError::Transport(TransportError::Rejected))
        );
        assert!(
            runtime
                .pop_control_message()
                .expect("control queue should load")
                .is_some()
        );
    }

    #[test]
    fn empty_runtime_is_idle_without_touching_transport() {
        // Arrange
        let runtime = runtime();
        let driver = RuntimeControlDriver::new(
            protocol(TestCodec {
                fail: false,
                route: FrameRoute::Control,
            }),
            RecordingTransport::new(false),
        );

        // Act
        let outcome = futures_lite_block_on(driver.send_once(&runtime));

        // Assert
        assert_eq!(outcome, Ok(ControlSendOutcome::Idle));
        assert!(
            driver
                .transport()
                .sent
                .lock()
                .expect("sent frames should be available")
                .is_empty()
        );
    }

    #[test]
    fn ordered_barrier_sends_preceding_messages_through_target_disposal() {
        // Arrange
        let runtime = Arc::new(DirectDispatcher::new(
            UnusedService,
            RuntimeLimits::new(2, 1).with_control_messages(3),
        ));
        runtime
            .register_object(ObjectId::new(7), None, ObjectKindTag::new(3))
            .expect("root should register");
        runtime
            .register_object(
                ObjectId::new(8),
                Some(ObjectId::new(7)),
                ObjectKindTag::new(4),
            )
            .expect("child should register");
        runtime
            .dispose_object(ObjectId::new(8))
            .expect("child should dispose");
        let transport = Arc::new(RecordingTransport::new(false));
        let control = OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(
                protocol(TestCodec {
                    fail: false,
                    route: FrameRoute::Control,
                }),
                Arc::clone(&transport),
            ),
            2,
        )
        .expect("ordered writer should build");

        // Act
        let barrier = futures_lite_block_on(control.begin_barrier())
            .expect("barrier should acquire its writer");
        let sent = futures_lite_block_on(barrier.send_through_disposal(ObjectId::new(8)))
            .expect("barrier should reach the target disposal");

        // Assert
        assert_eq!(sent, 3);
        assert_eq!(runtime.pop_control_message(), Ok(None));
        let payloads = transport
            .sent
            .lock()
            .expect("sent frames should be available")
            .iter()
            .map(|frame| u64::from_le_bytes(frame.payload().try_into().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(payloads, [7, 8, 8]);
    }

    #[test]
    fn split_disposal_fence_leaves_the_target_after_sending_its_descendants() {
        // Arrange
        let runtime = Arc::new(DirectDispatcher::new(
            UnusedService,
            RuntimeLimits::new(2, 1).with_control_messages(4),
        ));
        runtime
            .register_object(ObjectId::new(7), None, ObjectKindTag::new(3))
            .expect("root should register");
        runtime
            .register_object(
                ObjectId::new(8),
                Some(ObjectId::new(7)),
                ObjectKindTag::new(4),
            )
            .expect("child should register");
        runtime
            .dispose_object(ObjectId::new(7))
            .expect("subtree should dispose child first");
        let transport = Arc::new(RecordingTransport::new(false));
        let control = OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(
                protocol(TestCodec {
                    fail: false,
                    route: FrameRoute::Control,
                }),
                Arc::clone(&transport),
            ),
            2,
        )
        .expect("ordered writer should build");

        // Act
        let barrier = futures_lite_block_on(control.begin_barrier())
            .expect("barrier should acquire its writer");
        let sent = futures_lite_block_on(barrier.send_before_disposal(ObjectId::new(7)))
            .expect("barrier should stop before the root disposal");

        // Assert
        assert_eq!(sent, 3);
        assert_eq!(
            runtime.pop_control_message(),
            Ok(Some(ControlMessage::Dispose {
                object_id: ObjectId::new(7),
            }))
        );
        let payloads = transport
            .sent
            .lock()
            .expect("sent frames should be available")
            .iter()
            .map(|frame| u64::from_le_bytes(frame.payload().try_into().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(payloads, [7, 8, 8]);
    }

    #[test]
    fn ordered_barrier_sends_preceding_messages_through_exact_creation() {
        // Arrange
        let runtime = Arc::new(DirectDispatcher::new(
            UnusedService,
            RuntimeLimits::new(2, 1).with_control_messages(2),
        ));
        runtime
            .register_object(ObjectId::new(7), None, ObjectKindTag::new(3))
            .expect("root should register");
        runtime
            .register_object(
                ObjectId::new(8),
                Some(ObjectId::new(7)),
                ObjectKindTag::new(4),
            )
            .expect("child should register");
        let transport = Arc::new(RecordingTransport::new(false));
        let control = OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(
                protocol(TestCodec {
                    fail: false,
                    route: FrameRoute::Control,
                }),
                Arc::clone(&transport),
            ),
            2,
        )
        .expect("ordered writer should build");

        // Act
        let barrier = futures_lite_block_on(control.begin_barrier())
            .expect("barrier should acquire its writer");
        let sent = futures_lite_block_on(barrier.send_through_creation(
            ObjectId::new(8),
            Some(ObjectId::new(7)),
            ObjectKindTag::new(4),
        ))
        .expect("barrier should reach the exact target creation");

        // Assert
        assert_eq!(sent, 2);
        assert_eq!(runtime.pop_control_message(), Ok(None));
        let payloads = transport
            .sent
            .lock()
            .expect("sent frames should be available")
            .iter()
            .map(|frame| u64::from_le_bytes(frame.payload().try_into().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(payloads, [7, 8]);
    }

    #[test]
    fn ordered_creation_barrier_fails_if_the_exact_initializer_is_absent() {
        // Arrange
        let runtime = Arc::new(runtime());
        runtime
            .register_object(ObjectId::new(7), None, ObjectKindTag::new(3))
            .expect("different creation should register");
        let control = OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(
                protocol(TestCodec {
                    fail: false,
                    route: FrameRoute::Control,
                }),
                RecordingTransport::new(false),
            ),
            2,
        )
        .expect("ordered writer should build");

        // Act
        let barrier = futures_lite_block_on(control.begin_barrier())
            .expect("barrier should acquire its writer");
        let result = futures_lite_block_on(barrier.send_through_creation(
            ObjectId::new(8),
            Some(ObjectId::new(7)),
            ObjectKindTag::new(4),
        ));

        // Assert
        assert_eq!(
            result,
            Err(OrderedRuntimeControlError::CreationFenceMissing {
                object_id: ObjectId::new(8),
            })
        );
        assert_eq!(runtime.pop_control_message(), Ok(None));
    }

    #[test]
    fn ordered_barrier_excludes_the_ordinary_outbound_writer_until_drop() {
        // Arrange
        let runtime = Arc::new(runtime());
        runtime
            .register_object(ObjectId::new(7), None, ObjectKindTag::new(3))
            .expect("page should register");
        let control = OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(
                protocol(TestCodec {
                    fail: false,
                    route: FrameRoute::Control,
                }),
                RecordingTransport::new(false),
            ),
            2,
        )
        .expect("ordered writer should build");
        let barrier = futures_lite_block_on(control.begin_barrier())
            .expect("barrier should acquire its writer");
        let mut outbound = std::pin::pin!(control.send_once());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());

        // Act
        let while_held = outbound.as_mut().poll(&mut context);
        drop(barrier);
        let after_drop = outbound.as_mut().poll(&mut context);

        // Assert
        assert!(while_held.is_pending());
        assert!(matches!(
            after_drop,
            std::task::Poll::Ready(Ok(Some(ControlMessage::Create {
                object_id,
                parent: None,
                kind,
            }))) if object_id == ObjectId::new(7) && kind == ObjectKindTag::new(3)
        ));
        assert_eq!(runtime.pop_control_message(), Ok(None));
    }

    #[test]
    fn queued_barrier_precedes_an_ordinary_writer_after_the_active_barrier() {
        // Arrange
        let runtime = Arc::new(runtime());
        runtime
            .register_object(ObjectId::new(7), None, ObjectKindTag::new(3))
            .expect("page should register");
        let control = OrderedRuntimeControl::new(
            runtime,
            RuntimeControlDriver::new(
                protocol(TestCodec {
                    fail: false,
                    route: FrameRoute::Control,
                }),
                RecordingTransport::new(false),
            ),
            3,
        )
        .expect("ordered writer should build");
        let active = futures_lite_block_on(control.begin_barrier())
            .expect("first barrier should acquire its writer");
        let mut outbound = std::pin::pin!(control.send_once());
        let mut next_barrier = std::pin::pin!(control.begin_barrier());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(outbound.as_mut().poll(&mut context).is_pending());
        assert!(next_barrier.as_mut().poll(&mut context).is_pending());

        // Act
        drop(active);
        let preferred = next_barrier.as_mut().poll(&mut context);
        let ordinary = outbound.as_mut().poll(&mut context);

        // Assert
        let Poll::Ready(Ok(preferred)) = preferred else {
            panic!("queued completion barrier should acquire before ordinary outbound");
        };
        assert!(ordinary.is_pending());
        drop(preferred);
        assert!(outbound.as_mut().poll(&mut context).is_ready());
    }

    #[test]
    fn ordered_writer_rejects_waiters_beyond_its_hard_limit() {
        // Arrange
        let runtime = Arc::new(runtime());
        runtime
            .register_object(ObjectId::new(7), None, ObjectKindTag::new(3))
            .expect("page should register");
        let control = OrderedRuntimeControl::new(
            runtime,
            RuntimeControlDriver::new(
                protocol(TestCodec {
                    fail: false,
                    route: FrameRoute::Control,
                }),
                RecordingTransport::new(false),
            ),
            1,
        )
        .expect("ordered writer should build");
        let _active = futures_lite_block_on(control.begin_barrier())
            .expect("barrier should own the only writer slot");
        let mut outbound = std::pin::pin!(control.send_once());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());

        // Act
        let rejected = outbound.as_mut().poll(&mut context);

        // Assert
        assert!(matches!(
            rejected,
            Poll::Ready(Err(OrderedRuntimeControlError::Coordination(
                OrderedControlGateError::CapacityExceeded { limit: 1 }
            )))
        ));
    }

    fn futures_lite_block_on<Future>(future: Future) -> Future::Output
    where
        Future: std::future::Future,
    {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => output,
            std::task::Poll::Pending => panic!("test transport future should be ready"),
        }
    }
}

#[cfg(test)]
mod receive_tests {
    use std::collections::BTreeSet;
    use std::future::{Pending, Ready, pending, ready};
    use std::sync::Mutex;

    use pwright_client::{AtomicCallIdSource, Client, PageKind, RemoteKind};
    use pwright_client_control::{ClientLifecycleEvent, ClientLifecycleHandler};
    use pwright_client_wire::{FrameDisposition, FrameHandler};
    use pwright_codec_control::ControlCodec;
    use pwright_core::{
        CallId, CancellationId, CancellationReason, CancellationRequestOutcome, ControlMessage,
        EventTag, ObjectId, ObjectKindTag, ProductionResourceLedger, ProtocolVersion,
        ResourceClass, ResourceLimits,
    };
    use pwright_runtime::{DirectDispatcher, RuntimeLimits, RuntimeStateError};
    use pwright_server_wire::ServerControlFrameHandler;
    use pwright_transport::{
        FrameLimits, FrameRoute, FrameTransport, VersionPolicy, WireFrame, WireProtocol,
    };
    use pwright_transport_stream::{StreamFrameCodec, StreamFrameLimits};
    use pwright_transport_tokio::TokioStreamTransport;
    use tokio::io::duplex;

    use super::{
        ActivatedRuntimeControlError, ActivatedRuntimeControlReceiver, CancellationTarget,
        ControlSendOutcome, RuntimeControlDisposition, RuntimeControlDriver, RuntimeControlEvent,
        RuntimeControlProductionProfile, RuntimeControlReceiveError, RuntimeControlReceiver,
        SubscriptionActivator, TransportCloseTarget,
    };

    struct FakeRuntime {
        live: BTreeSet<ObjectId>,
        cancellations: Mutex<Vec<(CallId, CancellationId, CancellationReason)>>,
    }

    impl FakeRuntime {
        fn new(live: impl IntoIterator<Item = ObjectId>) -> Self {
            Self {
                live: live.into_iter().collect(),
                cancellations: Mutex::new(Vec::new()),
            }
        }
    }

    impl CancellationTarget for FakeRuntime {
        fn cancel_call(
            &self,
            call_id: CallId,
            cancellation: CancellationId,
            reason: CancellationReason,
        ) -> Result<CancellationRequestOutcome, RuntimeStateError> {
            self.cancellations
                .lock()
                .expect("cancellation records should be available")
                .push((call_id, cancellation, reason));
            Ok(CancellationRequestOutcome::Requested)
        }

        fn is_object_live(&self, object_id: ObjectId) -> Result<bool, RuntimeStateError> {
            Ok(self.live.contains(&object_id))
        }
    }

    struct ClosingRuntime {
        runtime: DirectDispatcher<UnusedService>,
    }

    impl ClosingRuntime {
        fn new() -> Self {
            Self {
                runtime: DirectDispatcher::new(UnusedService, RuntimeLimits::new(1, 1)),
            }
        }
    }

    impl CancellationTarget for ClosingRuntime {
        fn cancel_call(
            &self,
            _call_id: CallId,
            _cancellation: CancellationId,
            _reason: CancellationReason,
        ) -> Result<CancellationRequestOutcome, RuntimeStateError> {
            Ok(CancellationRequestOutcome::Requested)
        }

        fn is_object_live(&self, object_id: ObjectId) -> Result<bool, RuntimeStateError> {
            self.runtime.is_object_live(object_id)
        }
    }

    impl TransportCloseTarget for ClosingRuntime {
        fn close_transport(
            &self,
        ) -> Result<pwright_runtime::TransportCloseOutcome, RuntimeStateError> {
            self.runtime.close_transport()
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ActivationError;

    impl std::fmt::Display for ActivationError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("activation rejected")
        }
    }

    impl std::error::Error for ActivationError {}

    struct RecordingActivator {
        calls: Mutex<Vec<(ObjectId, EventTag, bool)>>,
        fail: bool,
    }

    impl RecordingActivator {
        fn new(fail: bool) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                fail,
            }
        }
    }

    impl SubscriptionActivator for RecordingActivator {
        type Error = ActivationError;
        type Future<'a> = Ready<Result<(), Self::Error>>;

        fn set_subscription<'a>(
            &'a self,
            object_id: ObjectId,
            event: EventTag,
            enabled: bool,
        ) -> Self::Future<'a> {
            self.calls
                .lock()
                .expect("activation calls should be available")
                .push((object_id, event, enabled));
            ready(if self.fail {
                Err(ActivationError)
            } else {
                Ok(())
            })
        }
    }

    struct PendingActivator;

    impl SubscriptionActivator for PendingActivator {
        type Error = ActivationError;
        type Future<'a> = Pending<Result<(), Self::Error>>;

        fn set_subscription<'a>(
            &'a self,
            _object_id: ObjectId,
            _event: EventTag,
            _enabled: bool,
        ) -> Self::Future<'a> {
            pending()
        }
    }

    fn frame_limits() -> FrameLimits {
        FrameLimits::new(32, 0, 0, 32)
    }

    fn protocol() -> WireProtocol<ControlCodec> {
        WireProtocol::new(
            ControlCodec,
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("version policy should build"),
            ProtocolVersion::new(1, 0),
            frame_limits(),
        )
        .expect("wire protocol should build")
    }

    fn control_frame(message: ControlMessage) -> WireFrame {
        protocol()
            .encode(&message)
            .expect("control message should encode")
    }

    fn block_on_ready<Output>(future: impl std::future::Future<Output = Output>) -> Output {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => output,
            std::task::Poll::Pending => panic!("test activation should be ready"),
        }
    }

    fn production_resources(profile: RuntimeControlProductionProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let events = requirements.class(ResourceClass::EventItem);
        let wire = requirements.class(ResourceClass::WireBytes);
        ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases)
                .with_class(ResourceClass::EventItem, events.global, events.per_scope)
                .with_class(ResourceClass::WireBytes, wire.global, wire.per_scope),
            requirements,
        )
        .expect("exact control production profile should fit")
    }

    #[test]
    fn caller_cancellation_is_applied_to_the_runtime() {
        // Arrange
        let runtime = FakeRuntime::new([]);
        let mut receiver = RuntimeControlReceiver::new(&runtime, protocol(), 1);
        let message = ControlMessage::Cancel {
            call_id: CallId::new(7),
            cancellation: CancellationId::new(8),
            reason: CancellationReason::Caller,
        };

        // Act
        let outcome = receiver
            .handle(control_frame(message))
            .expect("cancellation should be handled");

        // Assert
        assert_eq!(
            outcome,
            RuntimeControlDisposition::Handled(RuntimeControlEvent::Cancellation {
                call_id: CallId::new(7),
                outcome: CancellationRequestOutcome::Requested,
            })
        );
        assert_eq!(
            runtime
                .cancellations
                .lock()
                .expect("cancellation records should be available")
                .as_slice(),
            &[(
                CallId::new(7),
                CancellationId::new(8),
                CancellationReason::Caller,
            )]
        );
    }

    #[test]
    fn runtime_owned_controls_and_termination_reasons_are_rejected() {
        // Arrange
        let runtime = FakeRuntime::new([]);
        let mut receiver = RuntimeControlReceiver::new(&runtime, protocol(), 1);
        let create = ControlMessage::Create {
            object_id: ObjectId::new(1),
            parent: None,
            kind: ObjectKindTag::new(1),
        };
        let parent_closed = ControlMessage::Cancel {
            call_id: CallId::new(7),
            cancellation: CancellationId::new(8),
            reason: CancellationReason::ParentClosed,
        };

        // Act
        let create_error = receiver.handle(control_frame(create));
        let cancellation_error = receiver.handle(control_frame(parent_closed));

        // Assert
        assert_eq!(
            create_error,
            Err(RuntimeControlReceiveError::InvalidDirection(create))
        );
        assert_eq!(
            cancellation_error,
            Err(RuntimeControlReceiveError::InvalidCancellationReason(
                CancellationReason::ParentClosed
            ))
        );
        assert!(
            runtime
                .cancellations
                .lock()
                .expect("cancellation records should be available")
                .is_empty()
        );
    }

    #[test]
    fn subscription_state_is_bounded_and_edge_triggered() {
        // Arrange
        let runtime = FakeRuntime::new([ObjectId::new(1), ObjectId::new(2)]);
        let mut receiver = RuntimeControlReceiver::new(&runtime, protocol(), 1);
        let first = ControlMessage::SetSubscription {
            object_id: ObjectId::new(1),
            event: EventTag::new(9),
            enabled: true,
        };
        let second = ControlMessage::SetSubscription {
            object_id: ObjectId::new(2),
            event: EventTag::new(9),
            enabled: true,
        };

        // Act
        let enabled = receiver.handle(control_frame(first));
        let duplicate = receiver.handle(control_frame(first));
        let full = receiver.handle(control_frame(second));
        let disabled = receiver.handle(control_frame(ControlMessage::SetSubscription {
            object_id: ObjectId::new(1),
            event: EventTag::new(9),
            enabled: false,
        }));

        // Assert
        assert_eq!(
            enabled,
            Ok(RuntimeControlDisposition::Handled(
                RuntimeControlEvent::Subscription {
                    object_id: ObjectId::new(1),
                    event: EventTag::new(9),
                    enabled: true,
                    changed: true,
                }
            ))
        );
        assert_eq!(
            duplicate,
            Ok(RuntimeControlDisposition::Handled(
                RuntimeControlEvent::Subscription {
                    object_id: ObjectId::new(1),
                    event: EventTag::new(9),
                    enabled: true,
                    changed: false,
                }
            ))
        );
        assert_eq!(
            full,
            Err(RuntimeControlReceiveError::SubscriptionCapacityExceeded { limit: 1 })
        );
        assert_eq!(
            disabled,
            Ok(RuntimeControlDisposition::Handled(
                RuntimeControlEvent::Subscription {
                    object_id: ObjectId::new(1),
                    event: EventTag::new(9),
                    enabled: false,
                    changed: true,
                }
            ))
        );
        let metrics = receiver.subscription_metrics();
        assert_eq!(metrics.active(), 0);
        assert_eq!(metrics.peak(), 1);
        assert_eq!(metrics.limit(), 1);
    }

    #[test]
    fn failed_backend_enable_does_not_commit_logical_subscription_state() {
        // Arrange
        let runtime = FakeRuntime::new([ObjectId::new(1)]);
        let mut receiver = RuntimeControlReceiver::new(&runtime, protocol(), 1);
        let activator = RecordingActivator::new(true);
        let message = ControlMessage::SetSubscription {
            object_id: ObjectId::new(1),
            event: EventTag::new(9),
            enabled: true,
        };

        // Act
        let error =
            block_on_ready(receiver.handle_with_activation(control_frame(message), &activator))
                .expect_err("failed backend activation should fail the control barrier");

        // Assert
        assert!(matches!(
            error,
            ActivatedRuntimeControlError::Activation {
                object_id,
                event,
                enabled: true,
                source: ActivationError,
            } if object_id == ObjectId::new(1) && event == EventTag::new(9)
        ));
        assert_eq!(receiver.subscription_metrics().active(), 0);
        assert!(!receiver.subscription_metrics().in_flight());
    }

    #[test]
    fn successful_backend_enable_commits_one_logical_subscription() {
        // Arrange
        let runtime = FakeRuntime::new([ObjectId::new(1)]);
        let mut receiver = RuntimeControlReceiver::new(&runtime, protocol(), 1);
        let activator = RecordingActivator::new(false);
        let message = ControlMessage::SetSubscription {
            object_id: ObjectId::new(1),
            event: EventTag::new(9),
            enabled: true,
        };

        // Act
        let outcome =
            block_on_ready(receiver.handle_with_activation(control_frame(message), &activator));

        // Assert
        assert_eq!(
            outcome,
            Ok(RuntimeControlDisposition::Handled(
                RuntimeControlEvent::Subscription {
                    object_id: ObjectId::new(1),
                    event: EventTag::new(9),
                    enabled: true,
                    changed: true,
                }
            ))
        );
        assert_eq!(receiver.subscription_metrics().active(), 1);
        assert_eq!(
            activator
                .calls
                .lock()
                .expect("activation calls should be available")
                .as_slice(),
            &[(ObjectId::new(1), EventTag::new(9), true)]
        );
    }

    #[test]
    fn repeated_backend_enable_is_an_idempotent_edge_without_activation() {
        // Arrange
        let runtime = FakeRuntime::new([ObjectId::new(1)]);
        let mut receiver = RuntimeControlReceiver::new(&runtime, protocol(), 1);
        receiver
            .handle(control_frame(ControlMessage::SetSubscription {
                object_id: ObjectId::new(1),
                event: EventTag::new(9),
                enabled: true,
            }))
            .expect("arranged subscription should be active");
        let activator = RecordingActivator::new(false);
        let message = ControlMessage::SetSubscription {
            object_id: ObjectId::new(1),
            event: EventTag::new(9),
            enabled: true,
        };

        // Act
        let outcome =
            block_on_ready(receiver.handle_with_activation(control_frame(message), &activator));

        // Assert
        assert_eq!(
            outcome,
            Ok(RuntimeControlDisposition::Handled(
                RuntimeControlEvent::Subscription {
                    object_id: ObjectId::new(1),
                    event: EventTag::new(9),
                    enabled: true,
                    changed: false,
                }
            ))
        );
        assert_eq!(receiver.subscription_metrics().active(), 1);
        assert!(
            activator
                .calls
                .lock()
                .expect("activation calls should be available")
                .is_empty()
        );
    }

    #[test]
    fn failed_backend_disable_preserves_the_existing_logical_subscription() {
        // Arrange
        let runtime = FakeRuntime::new([ObjectId::new(1)]);
        let mut receiver = RuntimeControlReceiver::new(&runtime, protocol(), 1);
        receiver
            .handle(control_frame(ControlMessage::SetSubscription {
                object_id: ObjectId::new(1),
                event: EventTag::new(9),
                enabled: true,
            }))
            .expect("arranged subscription should be active");
        let activator = RecordingActivator::new(true);
        let message = ControlMessage::SetSubscription {
            object_id: ObjectId::new(1),
            event: EventTag::new(9),
            enabled: false,
        };

        // Act
        let outcome =
            block_on_ready(receiver.handle_with_activation(control_frame(message), &activator));

        // Assert
        assert!(matches!(
            outcome,
            Err(ActivatedRuntimeControlError::Activation {
                enabled: false,
                source: ActivationError,
                ..
            })
        ));
        assert_eq!(receiver.subscription_metrics().active(), 1);
        assert!(!receiver.subscription_metrics().in_flight());
    }

    #[test]
    fn disposed_object_deactivation_commits_each_successful_edge() {
        // Arrange
        let runtime = FakeRuntime::new([ObjectId::new(1)]);
        let mut receiver = RuntimeControlReceiver::new(&runtime, protocol(), 2);
        for event in [EventTag::new(9), EventTag::new(10)] {
            receiver
                .handle(control_frame(ControlMessage::SetSubscription {
                    object_id: ObjectId::new(1),
                    event,
                    enabled: true,
                }))
                .expect("subscription should be active");
        }
        let activator = RecordingActivator::new(false);
        let mut receiver = ActivatedRuntimeControlReceiver::new(receiver, activator);

        // Act
        let events = block_on_ready(receiver.deactivate_and_forget_object(ObjectId::new(1)));

        // Assert
        assert_eq!(events, Ok(2));
        assert_eq!(receiver.receiver().subscription_metrics().active(), 0);
        assert_eq!(
            receiver
                .activator()
                .calls
                .lock()
                .expect("activation calls should be available")
                .as_slice(),
            &[
                (ObjectId::new(1), EventTag::new(9), false),
                (ObjectId::new(1), EventTag::new(10), false),
            ]
        );
    }

    #[test]
    fn failed_disposed_object_deactivation_retains_the_route_for_retry() {
        // Arrange
        let runtime = FakeRuntime::new([ObjectId::new(1)]);
        let protocol = protocol();
        let profile = RuntimeControlProductionProfile::new(&protocol, 2);
        let resources = production_resources(profile);
        let mut receiver = RuntimeControlReceiver::new_with_production_resources(
            &runtime, protocol, 2, &resources,
        )
        .expect("declared control resources should install");
        for event in [EventTag::new(9), EventTag::new(10)] {
            receiver
                .handle(control_frame(ControlMessage::SetSubscription {
                    object_id: ObjectId::new(1),
                    event,
                    enabled: true,
                }))
                .expect("subscription should be active");
        }
        let activator = RecordingActivator::new(true);
        let mut receiver = ActivatedRuntimeControlReceiver::new(receiver, activator);

        // Act
        let error = block_on_ready(receiver.deactivate_and_forget_object(ObjectId::new(1)))
            .expect_err("failed deactivation should remain retryable");

        // Assert
        assert!(matches!(
            error,
            ActivatedRuntimeControlError::Activation {
                object_id,
                event,
                enabled: false,
                source: ActivationError,
            } if object_id == ObjectId::new(1) && event == EventTag::new(9)
        ));
        assert_eq!(receiver.receiver().subscription_metrics().active(), 2);
        assert_eq!(
            resources
                .metrics()
                .expect("resource metrics should load")
                .class(ResourceClass::EventItem)
                .current,
            2
        );
        assert_eq!(
            receiver
                .activator()
                .calls
                .lock()
                .expect("activation calls should be available")
                .as_slice(),
            &[(ObjectId::new(1), EventTag::new(9), false)]
        );
    }

    #[test]
    fn cancelled_pending_enable_is_compensated_during_transport_close() {
        // Arrange
        let runtime = ClosingRuntime::new();
        runtime
            .runtime
            .register_object(ObjectId::new(1), None, PageKind::OBJECT_KIND)
            .expect("page should register");
        let protocol = protocol();
        let profile = RuntimeControlProductionProfile::new(&protocol, 1);
        let resources = production_resources(profile);
        let mut receiver = RuntimeControlReceiver::new_with_production_resources(
            &runtime, protocol, 1, &resources,
        )
        .expect("declared control resources should install");
        let frame = control_frame(ControlMessage::SetSubscription {
            object_id: ObjectId::new(1),
            event: EventTag::new(9),
            enabled: true,
        });
        let pending = PendingActivator;
        let mut transition = Box::pin(receiver.handle_with_activation(frame, &pending));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());

        // Act
        let poll = std::future::Future::poll(transition.as_mut(), &mut context);
        drop(transition);
        let retained = resources.metrics().expect("resource metrics should load");
        let activator = RecordingActivator::new(false);
        let mut receiver = ActivatedRuntimeControlReceiver::new(receiver, activator);
        let closed = block_on_ready(receiver.handle_transport_closed());

        // Assert
        assert!(poll.is_pending());
        assert_eq!(retained.class(ResourceClass::EventItem).current, 1);
        assert!(closed.is_ok());
        assert_eq!(receiver.receiver().subscription_metrics().active(), 0);
        assert!(!receiver.receiver().subscription_metrics().in_flight());
        assert_eq!(
            receiver
                .activator()
                .calls
                .lock()
                .expect("activation calls should be available")
                .as_slice(),
            &[(ObjectId::new(1), EventTag::new(9), false)]
        );
        let released = resources.metrics().expect("resource metrics should load");
        assert_eq!(released.class(ResourceClass::EventItem).current, 0);
        assert_eq!(released.active_leases, 0);
    }

    #[test]
    fn cancelled_pending_disable_keeps_active_state_until_close_compensation() {
        // Arrange
        let runtime = ClosingRuntime::new();
        runtime
            .runtime
            .register_object(ObjectId::new(1), None, PageKind::OBJECT_KIND)
            .expect("page should register");
        let protocol = protocol();
        let profile = RuntimeControlProductionProfile::new(&protocol, 1);
        let resources = production_resources(profile);
        let mut receiver = RuntimeControlReceiver::new_with_production_resources(
            &runtime, protocol, 1, &resources,
        )
        .expect("declared control resources should install");
        receiver
            .handle(control_frame(ControlMessage::SetSubscription {
                object_id: ObjectId::new(1),
                event: EventTag::new(9),
                enabled: true,
            }))
            .expect("subscription should be active");
        let frame = control_frame(ControlMessage::SetSubscription {
            object_id: ObjectId::new(1),
            event: EventTag::new(9),
            enabled: false,
        });
        let pending = PendingActivator;
        let mut transition = Box::pin(receiver.handle_with_activation(frame, &pending));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());

        // Act
        let poll = std::future::Future::poll(transition.as_mut(), &mut context);
        drop(transition);
        let retained = receiver.subscription_metrics();
        let activator = RecordingActivator::new(false);
        let mut receiver = ActivatedRuntimeControlReceiver::new(receiver, activator);
        let closed = block_on_ready(receiver.handle_transport_closed());

        // Assert
        assert!(poll.is_pending());
        assert_eq!(retained.active(), 1);
        assert!(retained.in_flight());
        assert!(closed.is_ok());
        assert_eq!(receiver.receiver().subscription_metrics().active(), 0);
        assert!(!receiver.receiver().subscription_metrics().in_flight());
        assert_eq!(
            receiver
                .activator()
                .calls
                .lock()
                .expect("activation calls should be available")
                .as_slice(),
            &[(ObjectId::new(1), EventTag::new(9), false)]
        );
        let released = resources.metrics().expect("resource metrics should load");
        assert_eq!(released.class(ResourceClass::EventItem).current, 0);
        assert_eq!(released.active_leases, 0);
    }

    #[test]
    fn cancelled_close_cleanup_retains_the_current_route_for_retry() {
        // Arrange
        let runtime = ClosingRuntime::new();
        runtime
            .runtime
            .register_object(ObjectId::new(1), None, PageKind::OBJECT_KIND)
            .expect("page should register");
        let protocol = protocol();
        let profile = RuntimeControlProductionProfile::new(&protocol, 1);
        let resources = production_resources(profile);
        let mut receiver = RuntimeControlReceiver::new_with_production_resources(
            &runtime, protocol, 1, &resources,
        )
        .expect("declared control resources should install");
        receiver
            .handle(control_frame(ControlMessage::SetSubscription {
                object_id: ObjectId::new(1),
                event: EventTag::new(9),
                enabled: true,
            }))
            .expect("subscription should be active");
        let mut receiver = ActivatedRuntimeControlReceiver::new(receiver, PendingActivator);
        let mut close = Box::pin(receiver.handle_transport_closed());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());

        // Act
        let poll = std::future::Future::poll(close.as_mut(), &mut context);
        drop(close);
        let retained = receiver.receiver().subscription_metrics();
        let retained_resources = resources.metrics().expect("resource metrics should load");
        let (receiver, _) = receiver.into_parts();
        let activator = RecordingActivator::new(false);
        let mut receiver = ActivatedRuntimeControlReceiver::new(receiver, activator);
        let retried = block_on_ready(receiver.handle_transport_closed());

        // Assert
        assert!(poll.is_pending());
        assert_eq!(retained.active(), 1);
        assert_eq!(
            retained_resources.class(ResourceClass::EventItem).current,
            1
        );
        assert!(retried.is_ok());
        assert_eq!(receiver.receiver().subscription_metrics().active(), 0);
        assert_eq!(
            receiver
                .activator()
                .calls
                .lock()
                .expect("activation calls should be available")
                .as_slice(),
            &[(ObjectId::new(1), EventTag::new(9), false)]
        );
        let released = resources.metrics().expect("resource metrics should load");
        assert_eq!(released.class(ResourceClass::EventItem).current, 0);
        assert_eq!(released.active_leases, 0);
    }

    #[test]
    fn transport_close_attempts_every_deactivation_and_releases_all_routes() {
        // Arrange
        let runtime = ClosingRuntime::new();
        runtime
            .runtime
            .register_object(ObjectId::new(1), None, PageKind::OBJECT_KIND)
            .expect("page should register");
        let protocol = protocol();
        let profile = RuntimeControlProductionProfile::new(&protocol, 2);
        let resources = production_resources(profile);
        let mut receiver = RuntimeControlReceiver::new_with_production_resources(
            &runtime, protocol, 2, &resources,
        )
        .expect("declared control resources should install");
        for event in [EventTag::new(9), EventTag::new(10)] {
            receiver
                .handle(control_frame(ControlMessage::SetSubscription {
                    object_id: ObjectId::new(1),
                    event,
                    enabled: true,
                }))
                .expect("subscription should be active");
        }
        let activator = RecordingActivator::new(true);
        let mut receiver = ActivatedRuntimeControlReceiver::new(receiver, activator);

        // Act
        let error = block_on_ready(receiver.handle_transport_closed())
            .expect_err("backend cleanup failures should be explicit");

        // Assert
        assert!(matches!(
            error,
            ActivatedRuntimeControlError::TransportCloseCleanup {
                control: None,
                first_activation: Some(_),
                additional_activation_failures: 1,
            }
        ));
        assert_eq!(receiver.receiver().subscription_metrics().active(), 0);
        assert_eq!(
            receiver
                .activator()
                .calls
                .lock()
                .expect("activation calls should be available")
                .as_slice(),
            &[
                (ObjectId::new(1), EventTag::new(9), false),
                (ObjectId::new(1), EventTag::new(10), false),
            ]
        );
        assert!(
            !runtime
                .is_object_live(ObjectId::new(1))
                .expect("runtime state should remain available")
        );
        let metrics = resources.metrics().expect("resource metrics should load");
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 0);
        assert_eq!(metrics.active_leases, 0);
    }

    #[test]
    fn production_control_profile_accounts_protocol_and_subscription_routes() {
        // Arrange
        let protocol = protocol();

        // Act
        let profile = RuntimeControlProductionProfile::new(&protocol, 2);
        let requirements = profile.requirements();

        // Assert
        assert_eq!(profile.subscription_limit(), 2);
        assert_eq!(requirements.class(ResourceClass::EventItem).global, 2);
        assert_eq!(requirements.class(ResourceClass::EventItem).per_scope, 2);
        assert_eq!(requirements.class(ResourceClass::WireBytes).global, 64);
        assert_eq!(requirements.class(ResourceClass::WireBytes).per_scope, 64);
        assert_eq!(requirements.scopes, 3);
        assert_eq!(requirements.leases, 3);
    }

    #[test]
    fn production_subscription_leases_follow_enable_disable_and_disposal() {
        // Arrange
        let runtime = FakeRuntime::new([ObjectId::new(1)]);
        let protocol = protocol();
        let profile = RuntimeControlProductionProfile::new(&protocol, 2);
        let resources = production_resources(profile);
        let mut receiver = RuntimeControlReceiver::new_with_production_resources(
            &runtime, protocol, 2, &resources,
        )
        .expect("declared control resources should install");
        let enable = |event| ControlMessage::SetSubscription {
            object_id: ObjectId::new(1),
            event,
            enabled: true,
        };

        // Act
        receiver
            .handle(control_frame(enable(EventTag::new(9))))
            .expect("first subscription should be retained");
        receiver
            .handle(control_frame(enable(EventTag::new(10))))
            .expect("second subscription should be retained");
        receiver
            .handle(control_frame(enable(EventTag::new(9))))
            .expect("duplicate subscription should be unchanged");
        let retained = resources.metrics().expect("resource metrics should load");
        receiver
            .handle(control_frame(ControlMessage::SetSubscription {
                object_id: ObjectId::new(1),
                event: EventTag::new(9),
                enabled: false,
            }))
            .expect("disabled subscription should release its lease");
        let after_disable = resources.metrics().expect("resource metrics should load");
        let forgotten = receiver.forget_object(ObjectId::new(1));
        let after_disposal = resources.metrics().expect("resource metrics should load");

        // Assert
        let retained_events = retained.class(ResourceClass::EventItem);
        assert_eq!(retained_events.current, 2);
        assert_eq!(retained_events.peak, 2);
        assert_eq!(after_disable.class(ResourceClass::EventItem).current, 1);
        assert_eq!(forgotten, vec![EventTag::new(10)]);
        assert_eq!(after_disposal.class(ResourceClass::EventItem).current, 0);
        assert_eq!(after_disposal.active_leases, 0);
    }

    #[test]
    fn stale_subscription_and_non_control_frame_do_not_mutate_state() {
        // Arrange
        let runtime = FakeRuntime::new([]);
        let mut receiver = RuntimeControlReceiver::new(&runtime, protocol(), 1);
        let stale = ControlMessage::SetSubscription {
            object_id: ObjectId::new(99),
            event: EventTag::new(9),
            enabled: true,
        };
        let stale_disable = ControlMessage::SetSubscription {
            object_id: ObjectId::new(99),
            event: EventTag::new(9),
            enabled: false,
        };
        let handshake = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Handshake,
            Vec::new(),
            Vec::new(),
            frame_limits(),
        )
        .expect("handshake should build");

        // Act
        let stale_error = receiver.handle(control_frame(stale));
        let stale_disable = receiver.handle(control_frame(stale_disable));
        let unrelated = receiver
            .handle(handshake)
            .expect("non-control frame should be preserved");

        // Assert
        assert_eq!(
            stale_error,
            Err(RuntimeControlReceiveError::StaleObject(ObjectId::new(99)))
        );
        assert_eq!(
            stale_disable,
            Ok(RuntimeControlDisposition::Handled(
                RuntimeControlEvent::Subscription {
                    object_id: ObjectId::new(99),
                    event: EventTag::new(9),
                    enabled: false,
                    changed: false,
                }
            ))
        );
        let RuntimeControlDisposition::Unhandled(frame) = unrelated else {
            panic!("handshake should remain unhandled");
        };
        assert_eq!(frame.route(), FrameRoute::Handshake);
        assert_eq!(receiver.subscription_metrics().active(), 0);
    }

    #[test]
    fn forgetting_disposed_object_returns_backend_deactivation_plan() {
        // Arrange
        let runtime = FakeRuntime::new([ObjectId::new(1)]);
        let mut receiver = RuntimeControlReceiver::new(&runtime, protocol(), 2);
        for event in [EventTag::new(9), EventTag::new(10)] {
            receiver
                .handle(control_frame(ControlMessage::SetSubscription {
                    object_id: ObjectId::new(1),
                    event,
                    enabled: true,
                }))
                .expect("subscription should be accepted");
        }

        // Act
        let events = receiver.forget_object(ObjectId::new(1));

        // Assert
        assert_eq!(events, vec![EventTag::new(9), EventTag::new(10)]);
        assert_eq!(receiver.subscription_metrics().active(), 0);
    }

    struct UnusedService;

    #[tokio::test]
    async fn lifecycle_crosses_runtime_codec_stream_and_client_handler() {
        // Arrange
        let (runtime_io, client_io) = duplex(512);
        let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 128));
        let runtime_transport = TokioStreamTransport::from_io(runtime_io, stream_codec);
        let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
        let runtime = DirectDispatcher::new(
            UnusedService,
            RuntimeLimits::new(1, 1).with_control_messages(1),
        );
        runtime
            .register_object(ObjectId::new(7), None, PageKind::OBJECT_KIND)
            .expect("page should register");
        let sender = RuntimeControlDriver::new(protocol(), runtime_transport);
        let client = Client::new((), AtomicCallIdSource::new(1));
        let mut handler = ClientLifecycleHandler::new(client, protocol());

        // Act
        let sent = sender.send_once(&runtime).await;
        let frame = client_transport
            .receive()
            .await
            .expect("control frame should receive")
            .expect("stream should remain open");
        let handled = handler.handle(frame).expect("create should decode");

        // Assert
        let sent = sent.expect("lifecycle control should send");
        assert_eq!(
            sent,
            ControlSendOutcome::Sent(ControlMessage::Create {
                object_id: ObjectId::new(7),
                parent: None,
                kind: PageKind::OBJECT_KIND,
            })
        );
        assert_eq!(
            handled,
            FrameDisposition::Handled(ClientLifecycleEvent::Created(
                pwright_client::RemoteInitializer::new(
                    ObjectId::new(7),
                    None,
                    PageKind::OBJECT_KIND,
                )
            ))
        );
        assert_eq!(runtime.pop_control_message(), Ok(None));
    }

    #[tokio::test]
    async fn subscription_crosses_client_codec_stream_and_runtime_receiver() {
        // Arrange
        let (client_io, runtime_io) = duplex(512);
        let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 128));
        let client_transport = TokioStreamTransport::from_io(client_io, stream_codec);
        let runtime_transport = TokioStreamTransport::from_io(runtime_io, stream_codec);
        let runtime = FakeRuntime::new([ObjectId::new(7)]);
        let mut receiver = RuntimeControlReceiver::new(&runtime, protocol(), 1);
        let message = ControlMessage::SetSubscription {
            object_id: ObjectId::new(7),
            event: EventTag::new(9),
            enabled: true,
        };

        // Act
        client_transport
            .send(control_frame(message))
            .await
            .expect("subscription frame should send");
        let frame = runtime_transport
            .receive()
            .await
            .expect("subscription frame should receive")
            .expect("stream should remain open");
        let handled = receiver
            .handle(frame)
            .expect("subscription should be handled");

        // Assert
        assert_eq!(
            handled,
            RuntimeControlDisposition::Handled(RuntimeControlEvent::Subscription {
                object_id: ObjectId::new(7),
                event: EventTag::new(9),
                enabled: true,
                changed: true,
            })
        );
        assert_eq!(receiver.subscription_metrics().active(), 1);
    }
}
