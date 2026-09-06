//! Caller-driven, bounded production of statically typed server events.
//!
//! A channel owns exactly one [`EventSpec`]. Producers retain decoded payloads
//! in a bounded queue, while the outbound adapter owns encoding and transport
//! I/O. No task, thread, timer, runtime, browser, or dynamic event registry is
//! created. Multiple adapters can be composed with `ConcurrentServerOutbound`.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Poll, Waker};

use pwright_core::{
    BoundedQueue, Event, EventSequence, EventSpec, EventTag, ObjectId, OverflowPolicy,
    ProductionResourceError, ProductionResourceLedger, PushOutcome, QueueMetrics, ResourceClass,
    ResourceError, ResourceLease, ResourceLedger, ResourceRequest, ResourceRequirements,
    ResourceScope, SequencedEvent,
};
use pwright_server_wire::ConcurrentServerOutbound;
use pwright_transport::{
    FrameRoute, FrameTransport, ProductionFrameTransport, WireCodec, WireProtocol,
    WireProtocolError,
};

/// Production capacity for one typed event queue and one overlapping encode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServerEventProductionProfile {
    capacity: usize,
    max_retained_bytes_per_event: usize,
    maximum_encoding_wire_bytes: usize,
    protocol_requirements: ResourceRequirements,
    requirements: ResourceRequirements,
}

impl ServerEventProductionProfile {
    pub fn new<Codec>(
        protocol: &WireProtocol<Codec>,
        capacity: usize,
        max_retained_bytes_per_event: usize,
    ) -> Self {
        let items = u64::try_from(capacity).unwrap_or(u64::MAX);
        let bytes =
            items.saturating_mul(u64::try_from(max_retained_bytes_per_event).unwrap_or(u64::MAX));
        let retained = if capacity == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::EventItem, items, items)
                .with_class(ResourceClass::EventBytes, bytes, bytes)
                .with_metadata(capacity, capacity)
        };
        let protocol_requirements = protocol.production_resource_requirements();
        Self {
            capacity,
            max_retained_bytes_per_event,
            maximum_encoding_wire_bytes: protocol.maximum_encoding_wire_bytes(),
            protocol_requirements,
            requirements: retained.combine_concurrent(protocol_requirements),
        }
    }

    pub const fn capacity(self) -> usize {
        self.capacity
    }

    pub const fn max_retained_bytes_per_event(self) -> usize {
        self.max_retained_bytes_per_event
    }

    pub const fn protocol_requirements(self) -> ResourceRequirements {
        self.protocol_requirements
    }

    pub const fn maximum_encoding_wire_bytes(self) -> usize {
        self.maximum_encoding_wire_bytes
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ServerEventEmitError {
    StateUnavailable,
    SequenceExhausted,
    RetainedSizeOutOfRange { bytes: usize },
    RetainedSizeExceeded { actual: usize, limit: usize },
    Resource(ResourceError),
}

impl Display for ServerEventEmitError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::StateUnavailable => formatter.write_str("typed event queue state is unavailable"),
            Self::SequenceExhausted => {
                formatter.write_str("typed event channel sequence space is exhausted")
            }
            Self::RetainedSizeOutOfRange { bytes } => write!(
                formatter,
                "retained event size {bytes} cannot be represented by the resource ledger"
            ),
            Self::RetainedSizeExceeded { actual, limit } => write!(
                formatter,
                "retained event size {actual} exceeds this producer's production limit {limit}"
            ),
            Self::Resource(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ServerEventEmitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resource(error) => Some(error),
            Self::StateUnavailable
            | Self::SequenceExhausted
            | Self::RetainedSizeOutOfRange { .. }
            | Self::RetainedSizeExceeded { .. } => None,
        }
    }
}

#[derive(Debug)]
pub enum ServerEventProductionError {
    Resources(ProductionResourceError),
    ProtocolProfileMismatch {
        declared_maximum_encoding_wire_bytes: usize,
        actual_maximum_encoding_wire_bytes: usize,
    },
    TransportLedgerMismatch,
}

impl Display for ServerEventProductionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resources(error) => Display::fmt(error, formatter),
            Self::ProtocolProfileMismatch {
                declared_maximum_encoding_wire_bytes,
                actual_maximum_encoding_wire_bytes,
            } => write!(
                formatter,
                "typed event protocol encode bound differs from the accepted protocol: declared {declared_maximum_encoding_wire_bytes} bytes, actual {actual_maximum_encoding_wire_bytes} bytes"
            ),
            Self::TransportLedgerMismatch => formatter.write_str(
                "typed event protocol and production transport do not share one connection ledger",
            ),
        }
    }
}

impl Error for ServerEventProductionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resources(error) => Some(error),
            Self::ProtocolProfileMismatch { .. } | Self::TransportLedgerMismatch => None,
        }
    }
}

impl From<ProductionResourceError> for ServerEventProductionError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

#[derive(Debug)]
pub enum ServerEventOutboundError<Codec, Transport> {
    StateUnavailable,
    SendAlreadyActive,
    Protocol(WireProtocolError<Codec>),
    EncodedRouteMismatch {
        expected: FrameRoute,
        actual: FrameRoute,
    },
    Transport(Transport),
}

impl<Codec, Transport> Display for ServerEventOutboundError<Codec, Transport>
where
    Codec: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::StateUnavailable => formatter.write_str("typed event queue state is unavailable"),
            Self::SendAlreadyActive => {
                formatter.write_str("a typed event send is already active for this channel")
            }
            Self::Protocol(error) => Display::fmt(error, formatter),
            Self::EncodedRouteMismatch { expected, actual } => write!(
                formatter,
                "typed event codec emitted route {actual:?}, expected {expected:?}"
            ),
            Self::Transport(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Codec, Transport> Error for ServerEventOutboundError<Codec, Transport>
where
    Codec: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Protocol(error) => Some(error),
            Self::Transport(error) => Some(error),
            Self::StateUnavailable
            | Self::SendAlreadyActive
            | Self::EncodedRouteMismatch { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServerEventSent {
    source: ObjectId,
    event: EventTag,
    sequence: EventSequence,
}

impl ServerEventSent {
    pub const fn source(self) -> ObjectId {
        self.source
    }

    pub const fn event(self) -> EventTag {
        self.event
    }

    pub const fn sequence(self) -> EventSequence {
        self.sequence
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ServerEventMetrics {
    queue: QueueMetrics,
    active_send: bool,
    sent: u64,
}

impl ServerEventMetrics {
    pub const fn queue(self) -> QueueMetrics {
        self.queue
    }

    pub const fn active_send(self) -> bool {
        self.active_send
    }

    pub const fn sent(self) -> u64 {
        self.sent
    }
}

struct QueuedServerEvent<S>
where
    S: EventSpec,
{
    event: SequencedEvent<S>,
    resources: Option<ResourceLease>,
}

impl<S> QueuedServerEvent<S>
where
    S: EventSpec,
{
    fn unbudgeted(event: SequencedEvent<S>) -> Self {
        Self {
            event,
            resources: None,
        }
    }

    fn budgeted(event: SequencedEvent<S>, resources: ResourceLease) -> Self {
        Self {
            event,
            resources: Some(resources),
        }
    }

    fn into_event(self) -> SequencedEvent<S> {
        let Self { event, resources } = self;
        drop(resources);
        event
    }
}

struct EventResourceBudget<S>
where
    S: EventSpec,
{
    ledger: ResourceLedger,
    retained_bytes: fn(&Event<S>) -> usize,
    max_retained_bytes: usize,
}

struct EventState<S>
where
    S: EventSpec,
{
    queue: BoundedQueue<QueuedServerEvent<S>>,
    resources: Option<EventResourceBudget<S>>,
    active_send: bool,
    waiter: Option<Waker>,
    sent: u64,
    next_sequence: Option<EventSequence>,
}

impl<S> EventState<S>
where
    S: EventSpec,
{
    fn metrics(&self) -> ServerEventMetrics {
        let mut queue = self.queue.metrics();
        queue.current_len = queue
            .current_len
            .saturating_add(usize::from(self.active_send));
        ServerEventMetrics {
            queue,
            active_send: self.active_send,
            sent: self.sent,
        }
    }
}

struct SharedEventState<S>
where
    S: EventSpec,
{
    state: Mutex<EventState<S>>,
}

impl<S> SharedEventState<S>
where
    S: EventSpec,
{
    fn lock(&self) -> Result<MutexGuard<'_, EventState<S>>, ()> {
        self.state.lock().map_err(|_| ())
    }

    fn claim(self: &Arc<Self>) -> Result<Option<EventClaim<S>>, ServerEventClaimError> {
        let mut state = self
            .lock()
            .map_err(|_| ServerEventClaimError::StateUnavailable)?;
        if state.active_send {
            return Err(ServerEventClaimError::SendAlreadyActive);
        }
        let Some(item) = state.queue.pop() else {
            return Ok(None);
        };
        state.active_send = true;
        drop(state);
        Ok(Some(EventClaim {
            shared: Arc::clone(self),
            item: Some(item),
        }))
    }
}

enum ServerEventClaimError {
    StateUnavailable,
    SendAlreadyActive,
}

struct EventClaim<S>
where
    S: EventSpec,
{
    shared: Arc<SharedEventState<S>>,
    item: Option<QueuedServerEvent<S>>,
}

impl<S> EventClaim<S>
where
    S: EventSpec,
{
    fn event(&self) -> &SequencedEvent<S> {
        &self
            .item
            .as_ref()
            .expect("an active claim owns an item")
            .event
    }

    fn commit(mut self) {
        let item = self.item.take().expect("an active claim owns an item");
        drop(item);
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.active_send = false;
        state.sent = state.sent.saturating_add(1);
        let waiter = if state.queue.is_empty() {
            None
        } else {
            state.waiter.take()
        };
        drop(state);
        if let Some(waiter) = waiter {
            waiter.wake();
        }
    }
}

impl<S> Drop for EventClaim<S>
where
    S: EventSpec,
{
    fn drop(&mut self) {
        let Some(item) = self.item.take() else {
            return;
        };
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let restored = state.queue.restore_front(item);
        debug_assert!(
            restored.is_ok(),
            "a claimed queue slot must remain available"
        );
        state.active_send = false;
        let waiter = state.waiter.take();
        drop(state);
        if let Some(waiter) = waiter {
            waiter.wake();
        }
    }
}

/// Cloneable producer handle for one statically selected event type.
pub struct TypedEventSender<S>
where
    S: EventSpec,
{
    shared: Arc<SharedEventState<S>>,
}

impl<S> Clone for TypedEventSender<S>
where
    S: EventSpec,
{
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<S> TypedEventSender<S>
where
    S: EventSpec,
    S::Payload: Send,
{
    pub fn emit(
        &self,
        event: Event<S>,
    ) -> Result<PushOutcome<SequencedEvent<S>>, ServerEventEmitError> {
        let mut state = self
            .shared
            .lock()
            .map_err(|_| ServerEventEmitError::StateUnavailable)?;
        let sequence = state
            .next_sequence
            .ok_or(ServerEventEmitError::SequenceExhausted)?;
        state.next_sequence = sequence.checked_next();
        let event = SequencedEvent::new(sequence, event);
        let reserved = usize::from(state.active_send);
        let logically_full = state.queue.len().saturating_add(reserved) >= state.queue.capacity();

        if logically_full
            && (state.queue.policy() == OverflowPolicy::RejectNewest
                || state.queue.capacity() == 0
                || state.queue.is_empty())
        {
            let outcome = state
                .queue
                .push_with_reserved_slots(QueuedServerEvent::unbudgeted(event), reserved);
            return Ok(finish_emit(state, outcome));
        }

        let Some(resources) = state.resources.as_ref() else {
            let outcome = state
                .queue
                .push_with_reserved_slots(QueuedServerEvent::unbudgeted(event), reserved);
            return Ok(finish_emit(state, outcome));
        };
        let retained_bytes = (resources.retained_bytes)(event.event());
        if retained_bytes > resources.max_retained_bytes {
            return Err(ServerEventEmitError::RetainedSizeExceeded {
                actual: retained_bytes,
                limit: resources.max_retained_bytes,
            });
        }
        let retained_bytes = u64::try_from(retained_bytes).map_err(|_| {
            ServerEventEmitError::RetainedSizeOutOfRange {
                bytes: retained_bytes,
            }
        })?;
        let requests = event_requests(retained_bytes);
        let request_count = if retained_bytes == 0 { 1 } else { 2 };
        let requests = &requests[..request_count];
        let scope = ResourceScope::Object(event.object_id());

        if logically_full {
            let oldest = state
                .queue
                .front_mut()
                .expect("a full queue with no active-only claim has an oldest item");
            let lease = oldest
                .resources
                .as_mut()
                .expect("a production queue retains one lease per item");
            lease
                .try_replace(scope, requests)
                .map_err(ServerEventEmitError::Resource)?;
            let lease = oldest
                .resources
                .take()
                .expect("the replacement lease remains active");
            let outcome = state
                .queue
                .push_with_reserved_slots(QueuedServerEvent::budgeted(event, lease), reserved);
            return Ok(finish_emit(state, outcome));
        }

        let lease = resources
            .ledger
            .reserve_many(scope, requests)
            .map_err(ServerEventEmitError::Resource)?;
        let outcome = state
            .queue
            .push_with_reserved_slots(QueuedServerEvent::budgeted(event, lease), reserved);
        Ok(finish_emit(state, outcome))
    }

    pub fn metrics(&self) -> Result<ServerEventMetrics, ServerEventEmitError> {
        self.shared
            .lock()
            .map(|state| state.metrics())
            .map_err(|_| ServerEventEmitError::StateUnavailable)
    }

    /// Discards queued events. An event in an active send remains owned by it.
    pub fn clear(&self) -> Result<(), ServerEventEmitError> {
        self.shared
            .lock()
            .map(|mut state| state.queue.clear())
            .map_err(|_| ServerEventEmitError::StateUnavailable)
    }
}

/// The single transport-facing consumer for a typed event channel.
///
/// Once `send_once` has been polled, its owner must retain the future through
/// completion or close that transport. Dropping it restores queue ownership
/// and resource accounting, but a generic `FrameTransport` may already own a
/// partial write, so cancellation does not authorize retry on the same stream.
pub struct TypedEventOutbound<S, Codec, Transport>
where
    S: EventSpec,
{
    shared: Arc<SharedEventState<S>>,
    protocol: Arc<WireProtocol<Codec>>,
    transport: Transport,
    event_tag: fn(&Event<S>) -> EventTag,
    marker: PhantomData<fn() -> S>,
}

impl<S, Codec, Transport> TypedEventOutbound<S, Codec, Transport>
where
    S: EventSpec,
    S::Payload: Send,
{
    pub fn new(
        protocol: WireProtocol<Codec>,
        transport: Transport,
        capacity: usize,
        overflow: OverflowPolicy,
    ) -> (TypedEventSender<S>, Self) {
        Self::from_state(
            protocol,
            transport,
            |_| S::EVENT,
            EventState {
                queue: BoundedQueue::new(capacity, overflow),
                resources: None,
                active_send: false,
                waiter: None,
                sent: 0,
                next_sequence: Some(EventSequence::FIRST),
            },
        )
    }

    /// Builds one globally sequenced queue whose payload chooses among a
    /// statically reviewed family of event tags. This is used when causal
    /// ordering spans several public event types, such as HTTP lifecycle.
    pub fn new_routed(
        protocol: WireProtocol<Codec>,
        transport: Transport,
        capacity: usize,
        overflow: OverflowPolicy,
        event_tag: fn(&Event<S>) -> EventTag,
    ) -> (TypedEventSender<S>, Self) {
        Self::from_state(
            protocol,
            transport,
            event_tag,
            EventState {
                queue: BoundedQueue::new(capacity, overflow),
                resources: None,
                active_send: false,
                waiter: None,
                sent: 0,
                next_sequence: Some(EventSequence::FIRST),
            },
        )
    }

    pub fn new_with_production_resources(
        protocol: WireProtocol<Codec>,
        transport: Transport,
        profile: ServerEventProductionProfile,
        overflow: OverflowPolicy,
        resources: &ProductionResourceLedger,
        retained_bytes: fn(&Event<S>) -> usize,
    ) -> Result<(TypedEventSender<S>, Self), ServerEventProductionError>
    where
        Transport: ProductionFrameTransport,
    {
        Self::new_routed_with_production_resources(
            protocol,
            transport,
            profile,
            overflow,
            resources,
            retained_bytes,
            |_| S::EVENT,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_routed_with_production_resources(
        protocol: WireProtocol<Codec>,
        transport: Transport,
        profile: ServerEventProductionProfile,
        overflow: OverflowPolicy,
        resources: &ProductionResourceLedger,
        retained_bytes: fn(&Event<S>) -> usize,
        event_tag: fn(&Event<S>) -> EventTag,
    ) -> Result<(TypedEventSender<S>, Self), ServerEventProductionError>
    where
        Transport: ProductionFrameTransport,
    {
        let actual = protocol.production_resource_requirements();
        if profile.protocol_requirements() != actual {
            return Err(ServerEventProductionError::ProtocolProfileMismatch {
                declared_maximum_encoding_wire_bytes: profile.maximum_encoding_wire_bytes(),
                actual_maximum_encoding_wire_bytes: protocol.maximum_encoding_wire_bytes(),
            });
        }
        let ledger = resources.claim(profile.requirements())?;
        if !ledger.shares_budget_with(transport.production_resources().ledger()) {
            return Err(ServerEventProductionError::TransportLedgerMismatch);
        }
        let protocol = protocol.with_production_resources(resources)?;
        Ok(Self::from_state(
            protocol,
            transport,
            event_tag,
            EventState {
                queue: BoundedQueue::new(profile.capacity(), overflow),
                resources: Some(EventResourceBudget {
                    ledger,
                    retained_bytes,
                    max_retained_bytes: profile.max_retained_bytes_per_event(),
                }),
                active_send: false,
                waiter: None,
                sent: 0,
                next_sequence: Some(EventSequence::FIRST),
            },
        ))
    }

    fn from_state(
        protocol: WireProtocol<Codec>,
        transport: Transport,
        event_tag: fn(&Event<S>) -> EventTag,
        state: EventState<S>,
    ) -> (TypedEventSender<S>, Self) {
        let shared = Arc::new(SharedEventState {
            state: Mutex::new(state),
        });
        (
            TypedEventSender {
                shared: Arc::clone(&shared),
            },
            Self {
                shared,
                protocol: Arc::new(protocol),
                transport,
                event_tag,
                marker: PhantomData,
            },
        )
    }

    pub fn sender(&self) -> TypedEventSender<S> {
        TypedEventSender {
            shared: Arc::clone(&self.shared),
        }
    }

    pub fn protocol(&self) -> &WireProtocol<Codec> {
        self.protocol.as_ref()
    }

    pub const fn transport(&self) -> &Transport {
        &self.transport
    }
}

impl<S, Codec, Transport> ConcurrentServerOutbound for TypedEventOutbound<S, Codec, Transport>
where
    S: EventSpec,
    S::Payload: Send,
    Codec: WireCodec<SequencedEvent<S>>,
    Transport: FrameTransport,
{
    type Event = ServerEventSent;
    type Error = ServerEventOutboundError<
        <Codec as WireCodec<SequencedEvent<S>>>::Error,
        <Transport as FrameTransport>::Error,
    >;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Option<Self::Event>, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn send_once(&self) -> Self::Future<'_> {
        Box::pin(async move {
            let claim = match self.shared.claim() {
                Ok(Some(claim)) => claim,
                Ok(None) => return Ok(None),
                Err(ServerEventClaimError::StateUnavailable) => {
                    return Err(ServerEventOutboundError::StateUnavailable);
                }
                Err(ServerEventClaimError::SendAlreadyActive) => {
                    return Err(ServerEventOutboundError::SendAlreadyActive);
                }
            };
            let source = claim.event().object_id();
            let event = (self.event_tag)(claim.event().event());
            let sequence = claim.event().sequence();
            let expected = FrameRoute::Event { source, event };
            let frame = self
                .protocol
                .encode(claim.event())
                .map_err(ServerEventOutboundError::Protocol)?;
            if frame.route() != expected {
                return Err(ServerEventOutboundError::EncodedRouteMismatch {
                    expected,
                    actual: frame.route(),
                });
            }
            self.transport
                .send(frame)
                .await
                .map_err(ServerEventOutboundError::Transport)?;
            claim.commit();
            Ok(Some(ServerEventSent {
                source,
                event,
                sequence,
            }))
        })
    }

    fn wait_until_ready(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let shared = Arc::clone(&self.shared);
        Box::pin(std::future::poll_fn(move |context| {
            let Ok(mut state) = shared.lock() else {
                return Poll::Ready(());
            };
            if !state.queue.is_empty() {
                state.waiter = None;
                return Poll::Ready(());
            }
            if state
                .waiter
                .as_ref()
                .is_none_or(|waiter| !waiter.will_wake(context.waker()))
            {
                state.waiter = Some(context.waker().clone());
            }
            Poll::Pending
        }))
    }
}

const fn event_requests(retained_bytes: u64) -> [ResourceRequest; 2] {
    [
        ResourceRequest::new(ResourceClass::EventItem, 1),
        ResourceRequest::new(ResourceClass::EventBytes, retained_bytes),
    ]
}

fn map_push_outcome<S>(outcome: PushOutcome<QueuedServerEvent<S>>) -> PushOutcome<SequencedEvent<S>>
where
    S: EventSpec,
{
    match outcome {
        PushOutcome::Enqueued => PushOutcome::Enqueued,
        PushOutcome::DroppedOldest { dropped } => PushOutcome::DroppedOldest {
            dropped: dropped.into_event(),
        },
        PushOutcome::Rejected { rejected } => PushOutcome::Rejected {
            rejected: rejected.into_event(),
        },
    }
}

fn finish_emit<S>(
    mut state: MutexGuard<'_, EventState<S>>,
    outcome: PushOutcome<QueuedServerEvent<S>>,
) -> PushOutcome<SequencedEvent<S>>
where
    S: EventSpec,
{
    let waiter = outcome
        .was_enqueued()
        .then(|| state.waiter.take())
        .flatten();
    drop(state);
    if let Some(waiter) = waiter {
        waiter.wake();
    }
    map_push_outcome(outcome)
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::error::Error;
    use std::fmt::{self, Display, Formatter};
    use std::future::{Future, Ready, pending, ready};
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    use pwright_core::{
        Event, EventSequence, EventSpec, EventTag, ObjectId, OverflowPolicy,
        ProductionResourceLedger, ProtocolVersion, PushOutcome, ResourceClass, ResourceLedger,
        ResourceLimits, ResourceRequirements, SequencedEvent,
    };
    use pwright_server_wire::{ConcurrentServerOutbound, ConcurrentServerOutboundExt};
    use pwright_transport::{
        EncodedParts, FrameLimits, FrameRoute, FrameTransport, ProductionFrameTransport,
        ProductionTransportResources, WireCodec, WireFrame, WireProtocol,
    };

    use super::{ServerEventEmitError, ServerEventProductionProfile, TypedEventOutbound};

    struct ConsoleMessage;

    impl EventSpec for ConsoleMessage {
        const EVENT: EventTag = EventTag::new(17);
        type Payload = String;
    }

    #[derive(Clone, Copy)]
    struct ConsoleCodec {
        wrong_route: bool,
    }

    impl WireCodec<SequencedEvent<ConsoleMessage>> for ConsoleCodec {
        type Error = Infallible;

        fn encode_parts(
            &self,
            event: &SequencedEvent<ConsoleMessage>,
        ) -> Result<EncodedParts, Self::Error> {
            let route = if self.wrong_route {
                FrameRoute::Control
            } else {
                FrameRoute::Event {
                    source: event.object_id(),
                    event: ConsoleMessage::EVENT,
                }
            };
            Ok(EncodedParts::new(
                route,
                [
                    event.sequence().get().to_be_bytes().as_slice(),
                    event.payload().as_bytes(),
                ]
                .concat(),
                Vec::new(),
            ))
        }

        fn decode_frame(
            &self,
            _frame: WireFrame,
        ) -> Result<SequencedEvent<ConsoleMessage>, Self::Error> {
            unreachable!("server event tests only encode")
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestTransportError;

    impl Display for TestTransportError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("test transport rejected the frame")
        }
    }

    impl Error for TestTransportError {}

    struct TestTransport {
        sent: Arc<Mutex<Vec<WireFrame>>>,
        fail: Arc<AtomicBool>,
        block: Arc<AtomicBool>,
        production: Option<(ResourceLedger, ResourceRequirements)>,
    }

    struct TestTransportHandles {
        sent: Arc<Mutex<Vec<WireFrame>>>,
        fail: Arc<AtomicBool>,
        block: Arc<AtomicBool>,
    }

    impl TestTransport {
        fn unbudgeted() -> (Self, TestTransportHandles) {
            let sent = Arc::new(Mutex::new(Vec::new()));
            let fail = Arc::new(AtomicBool::new(false));
            let block = Arc::new(AtomicBool::new(false));
            (
                Self {
                    sent: Arc::clone(&sent),
                    fail: Arc::clone(&fail),
                    block: Arc::clone(&block),
                    production: None,
                },
                TestTransportHandles { sent, fail, block },
            )
        }

        fn production(
            resources: &ProductionResourceLedger,
            requirements: ResourceRequirements,
        ) -> (Self, Arc<Mutex<Vec<WireFrame>>>) {
            let ledger = resources
                .claim(requirements)
                .expect("test transport resources were declared");
            let (mut transport, handles) = Self::unbudgeted();
            transport.production = Some((ledger, requirements));
            (transport, handles.sent)
        }
    }

    impl FrameTransport for TestTransport {
        type Error = TestTransportError;
        type Send<'a>
            = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>
        where
            Self: 'a;
        type Receive<'a>
            = Ready<Result<Option<WireFrame>, Self::Error>>
        where
            Self: 'a;

        fn send(&self, frame: WireFrame) -> Self::Send<'_> {
            Box::pin(async move {
                if self.block.load(Ordering::SeqCst) {
                    pending::<()>().await;
                }
                if self.fail.load(Ordering::SeqCst) {
                    return Err(TestTransportError);
                }
                self.sent
                    .lock()
                    .expect("test sent-frame state should be available")
                    .push(frame);
                Ok(())
            })
        }

        fn receive(&self) -> Self::Receive<'_> {
            ready(Ok(None))
        }
    }

    impl ProductionFrameTransport for TestTransport {
        fn production_resources(&self) -> ProductionTransportResources<'_> {
            let (ledger, requirements) = self
                .production
                .as_ref()
                .expect("only production test transports expose evidence");
            ProductionTransportResources::new(ledger, *requirements)
        }
    }

    fn protocol(codec: ConsoleCodec) -> WireProtocol<ConsoleCodec> {
        WireProtocol::new_exact_directional(
            codec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(16, 0, 0, 16),
            FrameLimits::new(16, 0, 0, 16),
        )
    }

    fn retained_bytes(event: &Event<ConsoleMessage>) -> usize {
        event.payload().len()
    }

    fn event(source: u64, payload: &str) -> Event<ConsoleMessage> {
        Event::new(ObjectId::new(source), payload.to_owned())
    }

    fn block_on<F>(future: F) -> F::Output
    where
        F: Future,
    {
        let mut future = Box::pin(future);
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    fn exact_production_resources(requirements: ResourceRequirements) -> ProductionResourceLedger {
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let limit = requirements.class(class);
            limits = limits.with_class(class, limit.global, limit.per_scope);
        }
        ProductionResourceLedger::new(limits, requirements)
            .expect("the exact aggregate production plan should fit")
    }

    #[test]
    fn production_profile_covers_queue_payloads_and_overlapping_encode() {
        // Arrange
        let protocol = protocol(ConsoleCodec { wrong_route: false });

        // Act
        let profile = ServerEventProductionProfile::new(&protocol, 2, 5);

        // Assert
        assert_eq!(
            profile
                .requirements()
                .class(ResourceClass::EventItem)
                .global,
            2
        );
        assert_eq!(
            profile
                .requirements()
                .class(ResourceClass::EventBytes)
                .global,
            10
        );
        assert_eq!(
            profile
                .requirements()
                .class(ResourceClass::WireBytes)
                .global,
            32
        );
        assert_eq!(profile.requirements().scopes, 3);
        assert_eq!(profile.requirements().leases, 3);
    }

    #[test]
    fn typed_events_are_sent_in_fifo_order_with_canonical_routes() {
        // Arrange
        let (transport, handles) = TestTransport::unbudgeted();
        let sent = handles.sent;
        let (sender, outbound) = TypedEventOutbound::<ConsoleMessage, _, _>::new(
            protocol(ConsoleCodec { wrong_route: false }),
            transport,
            2,
            OverflowPolicy::RejectNewest,
        );
        assert!(matches!(
            sender.emit(event(7, "first")),
            Ok(PushOutcome::Enqueued)
        ));
        assert!(matches!(
            sender.emit(event(8, "second")),
            Ok(PushOutcome::Enqueued)
        ));

        // Act
        let first = block_on(outbound.send_once())
            .expect("first send should succeed")
            .expect("first event should exist");
        let second = block_on(outbound.send_once())
            .expect("second send should succeed")
            .expect("second event should exist");
        let frames = sent.lock().expect("sent-frame state should load");

        // Assert
        assert_eq!(first.source(), ObjectId::new(7));
        assert_eq!(second.source(), ObjectId::new(8));
        assert_eq!(&frames[0].payload()[8..], b"first");
        assert_eq!(&frames[1].payload()[8..], b"second");
        assert_eq!(first.sequence(), EventSequence::new(1));
        assert_eq!(second.sequence(), EventSequence::new(2));
        assert_eq!(
            frames[0].route(),
            FrameRoute::Event {
                source: ObjectId::new(7),
                event: ConsoleMessage::EVENT,
            }
        );
    }

    #[test]
    fn rejected_events_consume_sequence_space_and_make_wire_loss_detectable() {
        // Arrange
        let (transport, handles) = TestTransport::unbudgeted();
        let sent = handles.sent;
        let (sender, outbound) = TypedEventOutbound::<ConsoleMessage, _, _>::new(
            protocol(ConsoleCodec { wrong_route: false }),
            transport,
            1,
            OverflowPolicy::RejectNewest,
        );
        let first = sender
            .emit(event(7, "first"))
            .expect("first event should enqueue");
        let rejected = sender
            .emit(event(7, "lost"))
            .expect("overflow should be explicit");

        // Act
        let first_sent = block_on(outbound.send_once())
            .expect("first send should succeed")
            .expect("first event should exist");
        let third = sender
            .emit(event(7, "third"))
            .expect("space should be available after send");
        let third_sent = block_on(outbound.send_once())
            .expect("third send should succeed")
            .expect("third event should exist");
        let frames = sent.lock().expect("sent-frame state should load");

        // Assert
        assert!(matches!(first, PushOutcome::Enqueued));
        let PushOutcome::Rejected { rejected } = rejected else {
            panic!("second event should be rejected");
        };
        assert_eq!(rejected.sequence(), EventSequence::new(2));
        assert_eq!(rejected.into_payload(), "lost");
        assert!(matches!(third, PushOutcome::Enqueued));
        assert_eq!(first_sent.sequence(), EventSequence::new(1));
        assert_eq!(third_sent.sequence(), EventSequence::new(3));
        assert_eq!(
            u64::from_be_bytes(
                frames[0].payload()[..8]
                    .try_into()
                    .expect("sequence prefix")
            ),
            1
        );
        assert_eq!(
            u64::from_be_bytes(
                frames[1].payload()[..8]
                    .try_into()
                    .expect("sequence prefix")
            ),
            3
        );
    }

    #[test]
    fn event_sequence_exhaustion_never_wraps() {
        // Arrange
        let (transport, _) = TestTransport::unbudgeted();
        let (sender, _outbound) = TypedEventOutbound::<ConsoleMessage, _, _>::new(
            protocol(ConsoleCodec { wrong_route: false }),
            transport,
            2,
            OverflowPolicy::RejectNewest,
        );
        sender
            .shared
            .lock()
            .expect("event state should load")
            .next_sequence = Some(EventSequence::new(u64::MAX));

        // Act
        let last = sender.emit(event(7, "last"));
        let exhausted = sender.emit(event(7, "wrapped"));

        // Assert
        assert!(matches!(last, Ok(PushOutcome::Enqueued)));
        assert!(matches!(
            exhausted,
            Err(ServerEventEmitError::SequenceExhausted)
        ));
        assert_eq!(
            sender
                .metrics()
                .expect("event metrics should load")
                .queue()
                .current_len,
            1
        );
    }

    #[test]
    fn an_idle_composed_lane_is_woken_by_a_later_event_without_a_runtime_task() {
        // Arrange
        let (first_transport, _) = TestTransport::unbudgeted();
        let (second_transport, _) = TestTransport::unbudgeted();
        let (_first_sender, first) = TypedEventOutbound::<ConsoleMessage, _, _>::new(
            protocol(ConsoleCodec { wrong_route: false }),
            first_transport,
            1,
            OverflowPolicy::RejectNewest,
        );
        let (second_sender, second) = TypedEventOutbound::<ConsoleMessage, _, _>::new(
            protocol(ConsoleCodec { wrong_route: false }),
            second_transport,
            1,
            OverflowPolicy::RejectNewest,
        );
        let outbound = first.chain(second);
        assert!(matches!(block_on(outbound.send_once()), Ok(None)));
        let mut ready = outbound.wait_until_ready();
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        assert!(ready.as_mut().poll(&mut context).is_pending());

        // Act
        second_sender
            .emit(event(8, "later"))
            .expect("event should wake the idle outbound chain");

        // Assert
        assert!(ready.as_mut().poll(&mut context).is_ready());
    }

    #[test]
    fn pre_send_transport_rejection_restores_the_event_at_the_front() {
        // Arrange
        let (transport, handles) = TestTransport::unbudgeted();
        let sent = handles.sent;
        let fail = handles.fail;
        fail.store(true, Ordering::SeqCst);
        let (sender, outbound) = TypedEventOutbound::<ConsoleMessage, _, _>::new(
            protocol(ConsoleCodec { wrong_route: false }),
            transport,
            2,
            OverflowPolicy::RejectNewest,
        );
        sender
            .emit(event(7, "first"))
            .expect("first event should enqueue");
        sender
            .emit(event(8, "second"))
            .expect("second event should enqueue");

        // Act
        assert!(block_on(outbound.send_once()).is_err());
        let restored = sender.metrics().expect("queue metrics should load");
        fail.store(false, Ordering::SeqCst);
        block_on(outbound.send_once()).expect("retry should send the restored event");
        let frames = sent.lock().expect("sent-frame state should load");

        // Assert
        assert!(!restored.active_send());
        assert_eq!(restored.queue().current_len, 2);
        assert_eq!(frames.len(), 1);
        assert_eq!(&frames[0].payload()[8..], b"first");
    }

    #[test]
    fn cancelled_send_restores_capacity_and_preserves_the_claimed_oldest_event() {
        // Arrange
        let (transport, handles) = TestTransport::unbudgeted();
        let block = handles.block;
        block.store(true, Ordering::SeqCst);
        let (sender, outbound) = TypedEventOutbound::<ConsoleMessage, _, _>::new(
            protocol(ConsoleCodec { wrong_route: false }),
            transport,
            1,
            OverflowPolicy::DropOldest,
        );
        sender
            .emit(event(7, "first"))
            .expect("first event should enqueue");
        let mut send = Box::pin(outbound.send_once());
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        assert!(matches!(send.as_mut().poll(&mut context), Poll::Pending));

        // Act
        let rejected = sender
            .emit(event(8, "second"))
            .expect("overflow is an explicit outcome");
        drop(send);
        let restored = sender.metrics().expect("queue metrics should load");

        // Assert
        let PushOutcome::Rejected { rejected } = rejected else {
            panic!("an active oldest claim must not be displaced");
        };
        assert_eq!(rejected.into_payload(), "second");
        assert!(!restored.active_send());
        assert_eq!(restored.queue().current_len, 1);
        block.store(false, Ordering::SeqCst);
        block_on(outbound.send_once()).expect("restored oldest event should send");
    }

    #[test]
    fn production_send_releases_event_ownership_but_retains_the_budgeted_frame() {
        // Arrange
        let raw_protocol = protocol(ConsoleCodec { wrong_route: false });
        let profile = ServerEventProductionProfile::new(&raw_protocol, 1, 5);
        let transport_requirements = ResourceRequirements::none()
            .with_class(ResourceClass::WireBytes, 16, 16)
            .with_metadata(1, 1);
        let aggregate = profile
            .requirements()
            .combine_concurrent(transport_requirements);
        let resources = exact_production_resources(aggregate);
        let (transport, sent) = TestTransport::production(&resources, transport_requirements);
        let (sender, outbound) =
            TypedEventOutbound::<ConsoleMessage, _, _>::new_with_production_resources(
                raw_protocol,
                transport,
                profile,
                OverflowPolicy::RejectNewest,
                &resources,
                retained_bytes,
            )
            .expect("declared event channel should compose with the transport");
        sender
            .emit(event(7, "hello"))
            .expect("event should fit its profile");

        // Act
        block_on(outbound.send_once()).expect("production event should send");
        let metrics = resources.metrics().expect("resource metrics should load");

        // Assert
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 0);
        assert_eq!(metrics.class(ResourceClass::EventBytes).current, 0);
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 13);
        assert_eq!(sent.lock().expect("sent-frame state should load").len(), 1);
    }

    #[test]
    fn production_event_channel_rejects_a_transport_from_another_ledger() {
        // Arrange
        let raw_protocol = protocol(ConsoleCodec { wrong_route: false });
        let profile = ServerEventProductionProfile::new(&raw_protocol, 1, 5);
        let transport_requirements = ResourceRequirements::none()
            .with_class(ResourceClass::WireBytes, 16, 16)
            .with_metadata(1, 1);
        let channel_resources = exact_production_resources(
            profile
                .requirements()
                .combine_concurrent(transport_requirements),
        );
        let other_resources = exact_production_resources(transport_requirements);
        let (transport, _) = TestTransport::production(&other_resources, transport_requirements);

        // Act
        let result = TypedEventOutbound::<ConsoleMessage, _, _>::new_with_production_resources(
            raw_protocol,
            transport,
            profile,
            OverflowPolicy::RejectNewest,
            &channel_resources,
            retained_bytes,
        );

        // Assert
        assert!(matches!(
            result,
            Err(super::ServerEventProductionError::TransportLedgerMismatch)
        ));
    }

    #[test]
    fn codec_cannot_redirect_a_typed_event_to_another_route() {
        // Arrange
        let (transport, handles) = TestTransport::unbudgeted();
        let sent = handles.sent;
        let (sender, outbound) = TypedEventOutbound::<ConsoleMessage, _, _>::new(
            protocol(ConsoleCodec { wrong_route: true }),
            transport,
            1,
            OverflowPolicy::RejectNewest,
        );
        sender
            .emit(event(7, "hello"))
            .expect("event should enqueue");

        // Act
        let result = block_on(outbound.send_once());
        let metrics = sender.metrics().expect("queue metrics should load");

        // Assert
        assert!(matches!(
            result,
            Err(super::ServerEventOutboundError::EncodedRouteMismatch {
                expected: FrameRoute::Event { .. },
                actual: FrameRoute::Control,
            })
        ));
        assert_eq!(metrics.queue().current_len, 1);
        assert!(
            sent.lock()
                .expect("sent-frame state should load")
                .is_empty()
        );
    }
}
