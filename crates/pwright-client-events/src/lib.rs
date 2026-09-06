//! Opt-in typed event routing, bounded queues, and subscription edge tracking.
//!
//! One handler owns one event type and leaves every other frame untouched for
//! composition. Queue overflow is always returned to the caller. Subscription
//! tracking produces control messages only on the first-listener and
//! last-listener edges and performs no transport I/O itself.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;

use pwright_client_wire::{FrameDisposition, FrameHandler};
use pwright_core::{
    BoundedQueue, ControlMessage, Event, EventSequence, EventSpec, EventTag, ObjectId,
    OverflowPolicy, ProductionResourceError, ProductionResourceLedger, PushOutcome, QueueMetrics,
    ResourceClass, ResourceError, ResourceLease, ResourceLedger, ResourceRequest,
    ResourceRequirements, ResourceScope, SequencedEvent,
};
use pwright_transport::{FrameRoute, WireCodec, WireFrame, WireProtocol, WireProtocolError};

#[derive(Debug, Eq, PartialEq)]
pub enum ClientEventError<Codec> {
    Protocol(WireProtocolError<Codec>),
    SourceMismatch {
        route: ObjectId,
        decoded: ObjectId,
    },
    RetainedSizeOutOfRange {
        bytes: usize,
    },
    RetainedSizeExceeded {
        actual: usize,
        limit: usize,
    },
    InvalidSequence {
        actual: EventSequence,
    },
    DuplicateSequence {
        sequence: EventSequence,
    },
    SequenceRegression {
        previous: EventSequence,
        received: EventSequence,
    },
    Resource(ResourceError),
}

impl<Codec> Display for ClientEventError<Codec>
where
    Codec: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => Display::fmt(error, formatter),
            Self::SourceMismatch { route, decoded } => write!(
                formatter,
                "decoded event source `{decoded}` does not match frame route `{route}`"
            ),
            Self::RetainedSizeOutOfRange { bytes } => write!(
                formatter,
                "retained event size {bytes} cannot be represented by the resource ledger"
            ),
            Self::RetainedSizeExceeded { actual, limit } => write!(
                formatter,
                "retained event size {actual} exceeds this handler's production limit {limit}"
            ),
            Self::InvalidSequence { actual } => {
                write!(formatter, "event sequence {actual} is reserved and invalid")
            }
            Self::DuplicateSequence { sequence } => {
                write!(
                    formatter,
                    "event sequence {sequence} was received more than once"
                )
            }
            Self::SequenceRegression { previous, received } => write!(
                formatter,
                "event sequence regressed from {previous} to {received}"
            ),
            Self::Resource(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Codec> Error for ClientEventError<Codec>
where
    Codec: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Protocol(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::SourceMismatch { .. }
            | Self::RetainedSizeOutOfRange { .. }
            | Self::RetainedSizeExceeded { .. }
            | Self::InvalidSequence { .. }
            | Self::DuplicateSequence { .. }
            | Self::SequenceRegression { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventContinuity {
    Baseline,
    Consecutive,
    Gap {
        expected: EventSequence,
        received: EventSequence,
        missing: u64,
    },
}

pub struct ReceivedEvent<S>
where
    S: EventSpec,
{
    event: SequencedEvent<S>,
    continuity: EventContinuity,
}

impl<S> ReceivedEvent<S>
where
    S: EventSpec,
{
    const fn new(event: SequencedEvent<S>, continuity: EventContinuity) -> Self {
        Self { event, continuity }
    }

    pub const fn sequence(&self) -> EventSequence {
        self.event.sequence()
    }

    pub const fn continuity(&self) -> EventContinuity {
        self.continuity
    }

    pub const fn event(&self) -> &Event<S> {
        self.event.event()
    }

    pub const fn object_id(&self) -> ObjectId {
        self.event.object_id()
    }

    pub const fn payload(&self) -> &S::Payload {
        self.event.payload()
    }

    pub fn into_event(self) -> Event<S> {
        self.event.into_event()
    }

    pub fn into_payload(self) -> S::Payload {
        self.event.into_payload()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventSequenceMetrics {
    connection_epochs: u64,
    last_received: Option<EventSequence>,
    gaps: u64,
    missing: u64,
    duplicates: u64,
    regressions: u64,
}

impl EventSequenceMetrics {
    pub const fn connection_epochs(self) -> u64 {
        self.connection_epochs
    }

    pub const fn last_received(self) -> Option<EventSequence> {
        self.last_received
    }

    pub const fn gaps(self) -> u64 {
        self.gaps
    }

    pub const fn missing(self) -> u64 {
        self.missing
    }

    pub const fn duplicates(self) -> u64 {
        self.duplicates
    }

    pub const fn regressions(self) -> u64 {
        self.regressions
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypedEventProductionProfile {
    capacity: usize,
    max_retained_bytes_per_event: usize,
    requirements: ResourceRequirements,
}

impl TypedEventProductionProfile {
    pub fn new(capacity: usize, max_retained_bytes_per_event: usize) -> Self {
        let items = u64::try_from(capacity).unwrap_or(u64::MAX);
        let bytes =
            items.saturating_mul(u64::try_from(max_retained_bytes_per_event).unwrap_or(u64::MAX));
        let requirements = ResourceRequirements::none()
            .with_class(ResourceClass::EventItem, items, items)
            .with_class(ResourceClass::EventBytes, bytes, bytes)
            .with_metadata(capacity, capacity);
        Self {
            capacity,
            max_retained_bytes_per_event,
            requirements,
        }
    }

    pub const fn capacity(self) -> usize {
        self.capacity
    }

    pub const fn max_retained_bytes_per_event(self) -> usize {
        self.max_retained_bytes_per_event
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

struct QueuedEvent<S>
where
    S: EventSpec,
{
    event: ReceivedEvent<S>,
    resources: Option<ResourceLease>,
}

impl<S> QueuedEvent<S>
where
    S: EventSpec,
{
    fn unbudgeted(event: ReceivedEvent<S>) -> Self {
        Self {
            event,
            resources: None,
        }
    }

    fn budgeted(event: ReceivedEvent<S>, resources: ResourceLease) -> Self {
        Self {
            event,
            resources: Some(resources),
        }
    }

    fn into_event(self) -> ReceivedEvent<S> {
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
    max_retained_bytes: Option<usize>,
}

/// A bounded handler for exactly one statically typed event specification.
pub struct TypedEventHandler<S, Codec>
where
    S: EventSpec,
{
    protocol: WireProtocol<Codec>,
    queue: BoundedQueue<QueuedEvent<S>>,
    resources: Option<EventResourceBudget<S>>,
    sequence: EventSequenceMetrics,
    accepts_event: fn(EventTag) -> bool,
    marker: PhantomData<fn() -> S>,
}

impl<S, Codec> TypedEventHandler<S, Codec>
where
    S: EventSpec,
{
    pub fn new(protocol: WireProtocol<Codec>, capacity: usize, overflow: OverflowPolicy) -> Self {
        Self {
            protocol,
            queue: BoundedQueue::new(capacity, overflow),
            resources: None,
            sequence: EventSequenceMetrics {
                connection_epochs: 1,
                last_received: None,
                gaps: 0,
                missing: 0,
                duplicates: 0,
                regressions: 0,
            },
            accepts_event: |event| event == S::EVENT,
            marker: PhantomData,
        }
    }

    /// Builds one globally sequenced handler for a reviewed family of event
    /// tags encoded by a single aggregate payload codec.
    pub fn new_routed(
        protocol: WireProtocol<Codec>,
        capacity: usize,
        overflow: OverflowPolicy,
        accepts_event: fn(EventTag) -> bool,
    ) -> Self {
        Self {
            protocol,
            queue: BoundedQueue::new(capacity, overflow),
            resources: None,
            sequence: EventSequenceMetrics {
                connection_epochs: 1,
                last_received: None,
                gaps: 0,
                missing: 0,
                duplicates: 0,
                regressions: 0,
            },
            accepts_event,
            marker: PhantomData,
        }
    }

    /// Builds a queue whose retained items and payload bytes share one
    /// connection ledger. The sizing function measures the decoded event's
    /// variable-size retained data; `EventItem` accounts for fixed per-item
    /// ownership separately.
    pub fn new_with_resource_ledger(
        protocol: WireProtocol<Codec>,
        capacity: usize,
        overflow: OverflowPolicy,
        ledger: ResourceLedger,
        retained_bytes: fn(&Event<S>) -> usize,
    ) -> Self {
        Self {
            protocol,
            queue: BoundedQueue::new(capacity, overflow),
            resources: Some(EventResourceBudget {
                ledger,
                retained_bytes,
                max_retained_bytes: None,
            }),
            sequence: EventSequenceMetrics {
                connection_epochs: 1,
                last_received: None,
                gaps: 0,
                missing: 0,
                duplicates: 0,
                regressions: 0,
            },
            accepts_event: |event| event == S::EVENT,
            marker: PhantomData,
        }
    }

    /// Production-only queue construction. The profile covers every retained
    /// item at its declared maximum decoded payload size.
    pub fn new_with_production_resources(
        protocol: WireProtocol<Codec>,
        profile: TypedEventProductionProfile,
        overflow: OverflowPolicy,
        resources: &ProductionResourceLedger,
        retained_bytes: fn(&Event<S>) -> usize,
    ) -> Result<Self, ProductionResourceError> {
        Self::new_routed_with_production_resources(
            protocol,
            profile,
            overflow,
            resources,
            retained_bytes,
            |event| event == S::EVENT,
        )
    }

    pub fn new_routed_with_production_resources(
        protocol: WireProtocol<Codec>,
        profile: TypedEventProductionProfile,
        overflow: OverflowPolicy,
        resources: &ProductionResourceLedger,
        retained_bytes: fn(&Event<S>) -> usize,
        accepts_event: fn(EventTag) -> bool,
    ) -> Result<Self, ProductionResourceError> {
        let ledger = resources.claim(profile.requirements())?;
        Ok(Self {
            protocol,
            queue: BoundedQueue::new(profile.capacity(), overflow),
            resources: Some(EventResourceBudget {
                ledger,
                retained_bytes,
                max_retained_bytes: Some(profile.max_retained_bytes_per_event()),
            }),
            sequence: EventSequenceMetrics {
                connection_epochs: 1,
                last_received: None,
                gaps: 0,
                missing: 0,
                duplicates: 0,
                regressions: 0,
            },
            accepts_event,
            marker: PhantomData,
        })
    }

    pub fn pop(&mut self) -> Option<ReceivedEvent<S>> {
        self.queue.pop().map(QueuedEvent::into_event)
    }

    /// Discards every queued event and releases its resource leases.
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    pub fn metrics(&self) -> QueueMetrics {
        self.queue.metrics()
    }

    pub const fn sequence_metrics(&self) -> EventSequenceMetrics {
        self.sequence
    }

    /// Starts a fresh connection epoch. Queued events from the previous
    /// connection are released so sequence baselines cannot cross epochs.
    pub fn begin_new_connection(&mut self) {
        self.queue.clear();
        self.sequence.connection_epochs = self.sequence.connection_epochs.saturating_add(1);
        self.sequence.last_received = None;
    }

    pub const fn overflow_policy(&self) -> OverflowPolicy {
        self.queue.policy()
    }

    pub fn into_protocol(self) -> WireProtocol<Codec> {
        self.protocol
    }

    fn push_event<EventCodecError>(
        &mut self,
        event: ReceivedEvent<S>,
    ) -> Result<PushOutcome<ReceivedEvent<S>>, ClientEventError<EventCodecError>> {
        let Some(resources) = &self.resources else {
            return Ok(map_push_outcome(
                self.queue.push(QueuedEvent::unbudgeted(event)),
            ));
        };
        let is_full = self.queue.len() >= self.queue.capacity();
        if is_full
            && (self.queue.policy() == OverflowPolicy::RejectNewest || self.queue.capacity() == 0)
        {
            return Ok(map_push_outcome(
                self.queue.push(QueuedEvent::unbudgeted(event)),
            ));
        }

        let retained_bytes = (resources.retained_bytes)(event.event());
        if let Some(limit) = resources.max_retained_bytes
            && retained_bytes > limit
        {
            return Err(ClientEventError::RetainedSizeExceeded {
                actual: retained_bytes,
                limit,
            });
        }
        let retained_bytes = u64::try_from(retained_bytes).map_err(|_| {
            ClientEventError::RetainedSizeOutOfRange {
                bytes: retained_bytes,
            }
        })?;
        let requests = event_requests(retained_bytes);
        let request_count = if retained_bytes == 0 { 1 } else { 2 };
        let requests = &requests[..request_count];
        let scope = ResourceScope::Object(event.object_id());

        if is_full {
            let oldest = self
                .queue
                .front_mut()
                .expect("a full non-zero event queue has an oldest item");
            let lease = oldest
                .resources
                .as_mut()
                .expect("a budgeted event queue retains one lease per item");
            lease
                .try_replace(scope, requests)
                .map_err(ClientEventError::Resource)?;
            let lease = oldest
                .resources
                .take()
                .expect("the replacement lease remains active");
            return Ok(map_push_outcome(
                self.queue.push(QueuedEvent::budgeted(event, lease)),
            ));
        }

        let lease = resources
            .ledger
            .reserve_many(scope, requests)
            .map_err(ClientEventError::Resource)?;
        Ok(map_push_outcome(
            self.queue.push(QueuedEvent::budgeted(event, lease)),
        ))
    }

    fn observe_sequence<EventCodecError>(
        &mut self,
        sequence: EventSequence,
    ) -> Result<EventContinuity, ClientEventError<EventCodecError>> {
        if sequence.get() == 0 {
            return Err(ClientEventError::InvalidSequence { actual: sequence });
        }
        let Some(previous) = self.sequence.last_received else {
            self.sequence.last_received = Some(sequence);
            return Ok(EventContinuity::Baseline);
        };
        if sequence == previous {
            self.sequence.duplicates = self.sequence.duplicates.saturating_add(1);
            return Err(ClientEventError::DuplicateSequence { sequence });
        }
        let Some(expected) = previous.checked_next() else {
            self.sequence.regressions = self.sequence.regressions.saturating_add(1);
            return Err(ClientEventError::SequenceRegression {
                previous,
                received: sequence,
            });
        };
        if sequence < expected {
            self.sequence.regressions = self.sequence.regressions.saturating_add(1);
            return Err(ClientEventError::SequenceRegression {
                previous,
                received: sequence,
            });
        }
        self.sequence.last_received = Some(sequence);
        if sequence == expected {
            return Ok(EventContinuity::Consecutive);
        }
        let missing = sequence.get() - expected.get();
        self.sequence.gaps = self.sequence.gaps.saturating_add(1);
        self.sequence.missing = self.sequence.missing.saturating_add(missing);
        Ok(EventContinuity::Gap {
            expected,
            received: sequence,
            missing,
        })
    }
}

const fn event_requests(retained_bytes: u64) -> [ResourceRequest; 2] {
    [
        ResourceRequest::new(ResourceClass::EventItem, 1),
        ResourceRequest::new(ResourceClass::EventBytes, retained_bytes),
    ]
}

fn map_push_outcome<S>(outcome: PushOutcome<QueuedEvent<S>>) -> PushOutcome<ReceivedEvent<S>>
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

impl<S, Codec> FrameHandler for TypedEventHandler<S, Codec>
where
    S: EventSpec,
    Codec: WireCodec<SequencedEvent<S>>,
{
    type Event = PushOutcome<ReceivedEvent<S>>;
    type Error = ClientEventError<<Codec as WireCodec<SequencedEvent<S>>>::Error>;

    fn handle(&mut self, frame: WireFrame) -> Result<FrameDisposition<Self::Event>, Self::Error> {
        let FrameRoute::Event { source, event } = frame.route() else {
            return Ok(FrameDisposition::Unhandled(frame));
        };
        if !(self.accepts_event)(event) {
            return Ok(FrameDisposition::Unhandled(frame));
        }
        let decoded = self
            .protocol
            .decode::<SequencedEvent<S>>(frame)
            .map_err(ClientEventError::Protocol)?;
        if decoded.object_id() != source {
            return Err(ClientEventError::SourceMismatch {
                route: source,
                decoded: decoded.object_id(),
            });
        }
        let continuity = self.observe_sequence::<<Codec as WireCodec<SequencedEvent<S>>>::Error>(
            decoded.sequence(),
        )?;
        let received = ReceivedEvent::new(decoded, continuity);
        Ok(FrameDisposition::Handled(
            self.push_event::<<Codec as WireCodec<SequencedEvent<S>>>::Error>(received)?,
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubscriptionTransition {
    Unchanged,
    Send(ControlMessage),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SubscriptionMetrics {
    routes: usize,
    listeners: usize,
    peak_routes: usize,
    peak_listeners: usize,
    route_limit: usize,
}

impl SubscriptionMetrics {
    pub const fn routes(self) -> usize {
        self.routes
    }

    pub const fn listeners(self) -> usize {
        self.listeners
    }

    pub const fn peak_routes(self) -> usize {
        self.peak_routes
    }

    pub const fn peak_listeners(self) -> usize {
        self.peak_listeners
    }

    pub const fn route_limit(self) -> usize {
        self.route_limit
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubscriptionError {
    CapacityExceeded {
        limit: usize,
    },
    Missing {
        object_id: ObjectId,
        event: EventTag,
    },
    ListenerCountOverflow {
        object_id: ObjectId,
        event: EventTag,
    },
}

impl Display for SubscriptionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded { limit } => {
                write!(formatter, "subscription route limit {limit} was reached")
            }
            Self::Missing { object_id, event } => write!(
                formatter,
                "subscription `{event}` for object `{object_id}` does not exist"
            ),
            Self::ListenerCountOverflow { object_id, event } => write!(
                formatter,
                "subscription `{event}` for object `{object_id}` has too many listeners"
            ),
        }
    }
}

impl Error for SubscriptionError {}

/// Bounded reference counts for client listeners on `(object, event)` routes.
pub struct SubscriptionTracker {
    route_limit: usize,
    routes: BTreeMap<(ObjectId, EventTag), usize>,
    listeners: usize,
    peak_routes: usize,
    peak_listeners: usize,
}

impl SubscriptionTracker {
    pub const fn new(route_limit: usize) -> Self {
        Self {
            route_limit,
            routes: BTreeMap::new(),
            listeners: 0,
            peak_routes: 0,
            peak_listeners: 0,
        }
    }

    pub fn subscribe(
        &mut self,
        object_id: ObjectId,
        event: EventTag,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        let key = (object_id, event);
        if let Some(count) = self.routes.get_mut(&key) {
            let next_count = count
                .checked_add(1)
                .ok_or(SubscriptionError::ListenerCountOverflow { object_id, event })?;
            let next_listeners = self
                .listeners
                .checked_add(1)
                .ok_or(SubscriptionError::ListenerCountOverflow { object_id, event })?;
            *count = next_count;
            self.listeners = next_listeners;
            self.peak_listeners = self.peak_listeners.max(self.listeners);
            return Ok(SubscriptionTransition::Unchanged);
        }
        if self.routes.len() >= self.route_limit {
            return Err(SubscriptionError::CapacityExceeded {
                limit: self.route_limit,
            });
        }
        let next_listeners = self
            .listeners
            .checked_add(1)
            .ok_or(SubscriptionError::ListenerCountOverflow { object_id, event })?;
        self.routes.insert(key, 1);
        self.listeners = next_listeners;
        self.peak_routes = self.peak_routes.max(self.routes.len());
        self.peak_listeners = self.peak_listeners.max(self.listeners);
        Ok(SubscriptionTransition::Send(
            ControlMessage::SetSubscription {
                object_id,
                event,
                enabled: true,
            },
        ))
    }

    pub fn unsubscribe(
        &mut self,
        object_id: ObjectId,
        event: EventTag,
    ) -> Result<SubscriptionTransition, SubscriptionError> {
        let key = (object_id, event);
        let Some(count) = self.routes.get_mut(&key) else {
            return Err(SubscriptionError::Missing { object_id, event });
        };
        self.listeners -= 1;
        if *count > 1 {
            *count -= 1;
            return Ok(SubscriptionTransition::Unchanged);
        }
        self.routes.remove(&key);
        Ok(SubscriptionTransition::Send(
            ControlMessage::SetSubscription {
                object_id,
                event,
                enabled: false,
            },
        ))
    }

    /// Removes local listener state for an invalidated object without sending
    /// unsubscribe traffic to an object the runtime has already disposed.
    pub fn forget_object(&mut self, object_id: ObjectId) -> usize {
        let mut forgotten = 0usize;
        self.routes.retain(|(candidate, _), count| {
            if *candidate == object_id {
                forgotten = forgotten.saturating_add(*count);
                false
            } else {
                true
            }
        });
        self.listeners = self.listeners.saturating_sub(forgotten);
        forgotten
    }

    pub fn metrics(&self) -> SubscriptionMetrics {
        SubscriptionMetrics {
            routes: self.routes.len(),
            listeners: self.listeners,
            peak_routes: self.peak_routes,
            peak_listeners: self.peak_listeners,
            route_limit: self.route_limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fmt::{self, Display, Formatter};

    use pwright_client_wire::{FrameDisposition, FrameHandler};
    use pwright_core::{
        ControlMessage, Event, EventSequence, EventSpec, EventTag, ObjectId, OverflowPolicy,
        ProductionResourceLedger, ProtocolVersion, PushOutcome, ResourceClass, ResourceError,
        ResourceLedger, ResourceLimits, ResourceRequirements, ResourceScope, SequencedEvent,
    };
    use pwright_transport::{
        EncodedParts, FrameLimits, FrameRoute, VersionPolicy, WireCodec, WireFrame, WireProtocol,
    };

    use super::{
        ClientEventError, EventContinuity, SubscriptionError, SubscriptionTracker,
        SubscriptionTransition, TypedEventHandler, TypedEventProductionProfile,
    };

    struct ConsoleMessage;

    impl EventSpec for ConsoleMessage {
        const EVENT: EventTag = EventTag::new(9);
        type Payload = String;
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct CodecError;

    impl Display for CodecError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("event codec failed")
        }
    }

    impl Error for CodecError {}

    #[derive(Clone, Copy)]
    struct TestCodec {
        decoded_source: Option<ObjectId>,
    }

    impl WireCodec<SequencedEvent<ConsoleMessage>> for TestCodec {
        type Error = CodecError;

        fn encode_parts(
            &self,
            message: &SequencedEvent<ConsoleMessage>,
        ) -> Result<EncodedParts, Self::Error> {
            Ok(EncodedParts::new(
                FrameRoute::Event {
                    source: message.object_id(),
                    event: message.event_tag(),
                },
                [
                    message.sequence().get().to_be_bytes().as_slice(),
                    message.payload().as_bytes(),
                ]
                .concat(),
                Vec::new(),
            ))
        }

        fn decode_frame(
            &self,
            frame: WireFrame,
        ) -> Result<SequencedEvent<ConsoleMessage>, Self::Error> {
            let FrameRoute::Event { source, .. } = frame.route() else {
                return Err(CodecError);
            };
            let sequence_bytes = frame.payload().get(..8).ok_or(CodecError)?;
            let sequence = EventSequence::new(u64::from_be_bytes(
                sequence_bytes.try_into().map_err(|_| CodecError)?,
            ));
            let payload =
                String::from_utf8(frame.payload()[8..].to_vec()).map_err(|_| CodecError)?;
            Ok(SequencedEvent::new(
                sequence,
                Event::new(self.decoded_source.unwrap_or(source), payload),
            ))
        }
    }

    fn protocol(decoded_source: u64) -> WireProtocol<TestCodec> {
        WireProtocol::new(
            TestCodec {
                decoded_source: Some(ObjectId::new(decoded_source)),
            },
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("version policy should build"),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(128, 0, 0, 128),
        )
        .expect("wire protocol should build")
    }

    fn route_protocol() -> WireProtocol<TestCodec> {
        WireProtocol::new(
            TestCodec {
                decoded_source: None,
            },
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("version policy should build"),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(128, 0, 0, 128),
        )
        .expect("wire protocol should build")
    }

    fn event_ledger(items: u64, bytes: u64, per_scope_bytes: u64) -> ResourceLedger {
        ResourceLedger::new(
            ResourceLimits::deny_all(8, 8)
                .with_class(ResourceClass::EventItem, items, items)
                .with_class(ResourceClass::EventBytes, bytes, per_scope_bytes),
        )
    }

    fn retained_console_bytes(event: &Event<ConsoleMessage>) -> usize {
        event.payload().len()
    }

    fn zero_retained_bytes(_event: &Event<ConsoleMessage>) -> usize {
        0
    }

    fn production_resources(profile: TypedEventProductionProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let items = requirements.class(ResourceClass::EventItem);
        let bytes = requirements.class(ResourceClass::EventBytes);
        ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases)
                .with_class(ResourceClass::EventItem, items.global, items.per_scope)
                .with_class(ResourceClass::EventBytes, bytes.global, bytes.per_scope),
            requirements,
        )
        .expect("exact event profile should fit")
    }

    fn frame(source: u64, event: EventTag, payload: &str) -> WireFrame {
        frame_with_sequence(source, EventSequence::FIRST, event, payload)
    }

    fn frame_with_sequence(
        source: u64,
        sequence: EventSequence,
        event: EventTag,
        payload: &str,
    ) -> WireFrame {
        WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Event {
                source: ObjectId::new(source),
                event,
            },
            [sequence.get().to_be_bytes().as_slice(), payload.as_bytes()].concat(),
            Vec::new(),
            FrameLimits::new(128, 0, 0, 128),
        )
        .expect("event frame should build")
    }

    #[test]
    fn matching_event_is_decoded_and_enqueued() {
        // Arrange
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new(
            protocol(7),
            1,
            OverflowPolicy::RejectNewest,
        );

        // Act
        let outcome = handler
            .handle(frame(7, ConsoleMessage::EVENT, "hello"))
            .expect("event should decode");

        // Assert
        assert!(matches!(
            outcome,
            FrameDisposition::Handled(PushOutcome::Enqueued)
        ));
        let event = handler.pop().expect("queued event should exist");
        assert_eq!(event.object_id(), ObjectId::new(7));
        assert_eq!(event.payload(), "hello");
        assert_eq!(event.sequence(), EventSequence::FIRST);
        assert_eq!(event.continuity(), EventContinuity::Baseline);
    }

    #[test]
    fn client_reports_baseline_consecutive_and_gap_delivery() {
        // Arrange
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new(
            route_protocol(),
            3,
            OverflowPolicy::RejectNewest,
        );

        // Act
        for (sequence, payload) in [(40, "baseline"), (41, "next"), (44, "after-gap")] {
            handler
                .handle(frame_with_sequence(
                    7,
                    EventSequence::new(sequence),
                    ConsoleMessage::EVENT,
                    payload,
                ))
                .expect("monotonic event should be accepted");
        }
        let baseline = handler.pop().expect("baseline should be queued");
        let consecutive = handler.pop().expect("consecutive event should be queued");
        let gap = handler.pop().expect("gap event should be queued");
        let metrics = handler.sequence_metrics();

        // Assert
        assert_eq!(baseline.continuity(), EventContinuity::Baseline);
        assert_eq!(consecutive.continuity(), EventContinuity::Consecutive);
        assert_eq!(
            gap.continuity(),
            EventContinuity::Gap {
                expected: EventSequence::new(42),
                received: EventSequence::new(44),
                missing: 2,
            }
        );
        assert_eq!(metrics.last_received(), Some(EventSequence::new(44)));
        assert_eq!(metrics.gaps(), 1);
        assert_eq!(metrics.missing(), 2);
    }

    #[test]
    fn duplicate_regression_and_zero_sequences_are_protocol_errors() {
        // Arrange
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new(
            route_protocol(),
            2,
            OverflowPolicy::RejectNewest,
        );
        handler
            .handle(frame_with_sequence(
                7,
                EventSequence::new(9),
                ConsoleMessage::EVENT,
                "baseline",
            ))
            .expect("baseline should be accepted");

        // Act
        let duplicate = handler.handle(frame_with_sequence(
            7,
            EventSequence::new(9),
            ConsoleMessage::EVENT,
            "duplicate",
        ));
        let regression = handler.handle(frame_with_sequence(
            7,
            EventSequence::new(8),
            ConsoleMessage::EVENT,
            "regression",
        ));
        let mut zero_handler = TypedEventHandler::<ConsoleMessage, _>::new(
            route_protocol(),
            1,
            OverflowPolicy::RejectNewest,
        );
        let zero = zero_handler.handle(frame_with_sequence(
            7,
            EventSequence::new(0),
            ConsoleMessage::EVENT,
            "zero",
        ));
        let metrics = handler.sequence_metrics();

        // Assert
        assert!(matches!(
            duplicate,
            Err(ClientEventError::DuplicateSequence { sequence })
                if sequence == EventSequence::new(9)
        ));
        assert!(matches!(
            regression,
            Err(ClientEventError::SequenceRegression { previous, received })
                if previous == EventSequence::new(9)
                    && received == EventSequence::new(8)
        ));
        assert!(matches!(
            zero,
            Err(ClientEventError::InvalidSequence { actual })
                if actual == EventSequence::new(0)
        ));
        assert_eq!(metrics.duplicates(), 1);
        assert_eq!(metrics.regressions(), 1);
        assert_eq!(handler.metrics().current_len, 1);
        assert_eq!(zero_handler.metrics().current_len, 0);
    }

    #[test]
    fn new_connection_epoch_clears_old_events_and_accepts_a_new_baseline() {
        // Arrange
        let ledger = event_ledger(2, 32, 32);
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new_with_resource_ledger(
            route_protocol(),
            2,
            OverflowPolicy::RejectNewest,
            ledger.clone(),
            retained_console_bytes,
        );
        handler
            .handle(frame_with_sequence(
                7,
                EventSequence::new(99),
                ConsoleMessage::EVENT,
                "old",
            ))
            .expect("old connection baseline should queue");

        // Act
        handler.begin_new_connection();
        let after_reset = ledger.metrics().expect("resource metrics should load");
        handler
            .handle(frame_with_sequence(
                7,
                EventSequence::FIRST,
                ConsoleMessage::EVENT,
                "new",
            ))
            .expect("new connection baseline should queue");
        let received = handler.pop().expect("new baseline should be queued");
        let metrics = handler.sequence_metrics();

        // Assert
        assert_eq!(after_reset.class(ResourceClass::EventItem).current, 0);
        assert_eq!(after_reset.class(ResourceClass::EventBytes).current, 0);
        assert_eq!(received.payload(), "new");
        assert_eq!(received.continuity(), EventContinuity::Baseline);
        assert_eq!(metrics.connection_epochs(), 2);
        assert_eq!(metrics.last_received(), Some(EventSequence::FIRST));
    }

    #[test]
    fn budgeted_event_owns_item_and_bytes_until_dequeue() {
        // Arrange
        let ledger = event_ledger(2, 16, 16);
        let scope = ResourceScope::Object(ObjectId::new(7));
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new_with_resource_ledger(
            route_protocol(),
            2,
            OverflowPolicy::RejectNewest,
            ledger.clone(),
            retained_console_bytes,
        );

        // Act
        let outcome = handler
            .handle(frame(7, ConsoleMessage::EVENT, "hello"))
            .expect("event should fit the shared budget");
        let retained = ledger
            .scope_metrics(scope)
            .expect("scope metrics should be available")
            .expect("source scope should be retained");
        let event = handler.pop().expect("queued event should exist");
        let released = ledger.metrics().expect("metrics should be available");

        // Assert
        assert!(matches!(
            outcome,
            FrameDisposition::Handled(PushOutcome::Enqueued)
        ));
        assert_eq!(retained.units(ResourceClass::EventItem), 1);
        assert_eq!(retained.units(ResourceClass::EventBytes), 5);
        assert_eq!(event.payload(), "hello");
        assert_eq!(released.class(ResourceClass::EventItem).current, 0);
        assert_eq!(released.class(ResourceClass::EventBytes).current, 0);
    }

    #[test]
    fn production_profile_covers_every_queue_slot_at_its_payload_limit() {
        // Arrange
        let profile = TypedEventProductionProfile::new(3, 5);

        // Act
        let requirements = profile.requirements();

        // Assert
        assert_eq!(
            requirements.class(ResourceClass::EventItem),
            pwright_core::ResourceLimit::new(3, 3)
        );
        assert_eq!(
            requirements.class(ResourceClass::EventBytes),
            pwright_core::ResourceLimit::new(15, 15)
        );
        assert_eq!(requirements.scopes, 3);
        assert_eq!(requirements.leases, 3);
    }

    #[test]
    fn production_event_queue_claims_and_releases_its_complete_profile() {
        // Arrange
        let profile = TypedEventProductionProfile::new(2, 5);
        let resources = production_resources(profile);
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new_with_production_resources(
            route_protocol(),
            profile,
            OverflowPolicy::RejectNewest,
            &resources,
            retained_console_bytes,
        )
        .expect("declared production event queue should build");

        // Act
        handler
            .handle(frame(7, ConsoleMessage::EVENT, "hello"))
            .expect("first maximum event should fit");
        handler
            .handle(frame_with_sequence(
                8,
                EventSequence::new(2),
                ConsoleMessage::EVENT,
                "world",
            ))
            .expect("second maximum event should fit");
        let full = resources.metrics().expect("metrics should load");
        let first = handler.pop().expect("first event should exist");
        let after_pop = resources.metrics().expect("metrics should load");
        drop(handler);
        let after_drop = resources.metrics().expect("metrics should load");

        // Assert
        assert_eq!(first.payload(), "hello");
        assert_eq!(full.class(ResourceClass::EventItem).current, 2);
        assert_eq!(full.class(ResourceClass::EventBytes).current, 10);
        assert_eq!(after_pop.class(ResourceClass::EventItem).current, 1);
        assert_eq!(after_pop.class(ResourceClass::EventBytes).current, 5);
        assert_eq!(after_drop.active_leases, 0);
    }

    #[test]
    fn production_event_queue_rejects_payload_above_its_local_declaration() {
        // Arrange
        let profile = TypedEventProductionProfile::new(2, 4);
        let resources = production_resources(profile);
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new_with_production_resources(
            route_protocol(),
            profile,
            OverflowPolicy::RejectNewest,
            &resources,
            retained_console_bytes,
        )
        .expect("declared production event queue should build");

        // Act
        let error = match handler.handle(frame(7, ConsoleMessage::EVENT, "hello")) {
            Ok(_) => panic!("five decoded bytes exceed the four-byte local contract"),
            Err(error) => error,
        };
        let metrics = resources.metrics().expect("metrics should load");

        // Assert
        assert_eq!(
            error,
            ClientEventError::RetainedSizeExceeded {
                actual: 5,
                limit: 4,
            }
        );
        assert_eq!(handler.metrics().current_len, 0);
        assert_eq!(metrics.active_leases, 0);
        assert_eq!(metrics.rejected_reservations, 0);
    }

    #[test]
    fn production_zero_byte_events_still_own_item_capacity() {
        // Arrange
        let profile = TypedEventProductionProfile::new(1, 0);
        let resources = production_resources(profile);
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new_with_production_resources(
            route_protocol(),
            profile,
            OverflowPolicy::RejectNewest,
            &resources,
            zero_retained_bytes,
        )
        .expect("zero-byte event profile should build");

        // Act
        handler
            .handle(frame(7, ConsoleMessage::EVENT, "ignored-size"))
            .expect("item-only event should fit");
        let metrics = resources.metrics().expect("metrics should load");

        // Assert
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 1);
        assert_eq!(metrics.class(ResourceClass::EventBytes).current, 0);
        assert_eq!(metrics.active_leases, 1);
    }

    #[test]
    fn production_event_queue_rejects_an_omitted_event_plan() {
        // Arrange
        let profile = TypedEventProductionProfile::new(1, 5);
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(1, 1)
                .with_class(ResourceClass::EventItem, 1, 1)
                .with_class(ResourceClass::EventBytes, 5, 5),
            ResourceRequirements::none(),
        )
        .expect("empty plan may have stricter configured limits");

        // Act
        let error = match TypedEventHandler::<ConsoleMessage, _>::new_with_production_resources(
            route_protocol(),
            profile,
            OverflowPolicy::RejectNewest,
            &resources,
            retained_console_bytes,
        ) {
            Ok(_) => panic!("omitted production event queue must not build"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(
            error,
            pwright_core::ProductionResourceError::InsufficientGlobal {
                boundary: pwright_core::ResourceCapacityBoundary::DeclaredPlan,
                class: ResourceClass::EventItem,
                required: 1,
                available: 0,
            }
        ));
    }

    #[test]
    fn shared_event_budget_combines_independent_handler_queues() {
        // Arrange
        let ledger = event_ledger(2, 8, 8);
        let mut first = TypedEventHandler::<ConsoleMessage, _>::new_with_resource_ledger(
            route_protocol(),
            1,
            OverflowPolicy::RejectNewest,
            ledger.clone(),
            retained_console_bytes,
        );
        let mut second = TypedEventHandler::<ConsoleMessage, _>::new_with_resource_ledger(
            route_protocol(),
            1,
            OverflowPolicy::RejectNewest,
            ledger.clone(),
            retained_console_bytes,
        );
        first
            .handle(frame(7, ConsoleMessage::EVENT, "first"))
            .expect("first queue should consume five bytes");

        // Act
        let error = match second.handle(frame(8, ConsoleMessage::EVENT, "four")) {
            Err(error) => error,
            Ok(_) => panic!("aggregate event bytes should exceed the connection limit"),
        };
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(
            error,
            ClientEventError::Resource(ResourceError::GlobalLimitExceeded {
                class: ResourceClass::EventBytes,
                current: 5,
                requested: 4,
                limit: 8,
            })
        );
        assert_eq!(first.metrics().current_len, 1);
        assert_eq!(second.metrics().current_len, 0);
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 1);
        assert_eq!(metrics.class(ResourceClass::EventBytes).current, 5);
    }

    #[test]
    fn reliable_overflow_rejects_without_consuming_additional_budget() {
        // Arrange
        let ledger = event_ledger(1, 8, 8);
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new_with_resource_ledger(
            route_protocol(),
            1,
            OverflowPolicy::RejectNewest,
            ledger.clone(),
            retained_console_bytes,
        );
        handler
            .handle(frame(7, ConsoleMessage::EVENT, "first"))
            .expect("first event should fit");

        // Act
        let outcome = handler
            .handle(frame_with_sequence(
                7,
                EventSequence::new(2),
                ConsoleMessage::EVENT,
                "second",
            ))
            .expect("queue policy should return the rejected event");
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        let FrameDisposition::Handled(PushOutcome::Rejected { rejected }) = outcome else {
            panic!("full reliable queue should reject its newest event");
        };
        assert_eq!(rejected.payload(), "second");
        assert_eq!(rejected.sequence(), EventSequence::new(2));
        assert_eq!(rejected.continuity(), EventContinuity::Consecutive);
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 1);
        assert_eq!(metrics.class(ResourceClass::EventBytes).current, 5);
        assert_eq!(metrics.rejected_reservations, 0);
    }

    #[test]
    fn lossy_overflow_atomically_moves_the_oldest_lease_to_the_new_event() {
        // Arrange
        let ledger = event_ledger(1, 6, 6);
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new_with_resource_ledger(
            route_protocol(),
            1,
            OverflowPolicy::DropOldest,
            ledger.clone(),
            retained_console_bytes,
        );
        handler
            .handle(frame(7, ConsoleMessage::EVENT, "first"))
            .expect("first event should fit");

        // Act
        let outcome = handler
            .handle(frame_with_sequence(
                8,
                EventSequence::new(2),
                ConsoleMessage::EVENT,
                "second",
            ))
            .expect("replacement should reuse the admitted lease");
        let old_scope = ledger
            .scope_metrics(ResourceScope::Object(ObjectId::new(7)))
            .expect("old scope metrics should be available");
        let new_scope = ledger
            .scope_metrics(ResourceScope::Object(ObjectId::new(8)))
            .expect("new scope metrics should be available")
            .expect("new source scope should be retained");

        // Assert
        let FrameDisposition::Handled(PushOutcome::DroppedOldest { dropped }) = outcome else {
            panic!("lossy queue should return the dropped event");
        };
        assert_eq!(dropped.payload(), "first");
        assert!(old_scope.is_none());
        assert_eq!(new_scope.units(ResourceClass::EventItem), 1);
        assert_eq!(new_scope.units(ResourceClass::EventBytes), 6);
        assert_eq!(new_scope.active_leases, 1);
        assert_eq!(
            handler.pop().expect("new event should remain").payload(),
            "second"
        );
    }

    #[test]
    fn failed_lossy_replacement_preserves_the_queued_oldest_event() {
        // Arrange
        let ledger = event_ledger(1, 6, 6);
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new_with_resource_ledger(
            route_protocol(),
            1,
            OverflowPolicy::DropOldest,
            ledger.clone(),
            retained_console_bytes,
        );
        handler
            .handle(frame(7, ConsoleMessage::EVENT, "first"))
            .expect("first event should fit");

        // Act
        let error = match handler.handle(frame_with_sequence(
            8,
            EventSequence::new(2),
            ConsoleMessage::EVENT,
            "toolong",
        )) {
            Err(error) => error,
            Ok(_) => panic!("seven-byte replacement should exceed the shared budget"),
        };
        let retained = ledger.metrics().expect("metrics should be available");
        let oldest = handler.pop().expect("oldest event must remain queued");

        // Assert
        assert_eq!(
            error,
            ClientEventError::Resource(ResourceError::GlobalLimitExceeded {
                class: ResourceClass::EventBytes,
                current: 0,
                requested: 7,
                limit: 6,
            })
        );
        assert_eq!(oldest.payload(), "first");
        assert_eq!(retained.class(ResourceClass::EventItem).current, 1);
        assert_eq!(retained.class(ResourceClass::EventBytes).current, 5);
    }

    #[test]
    fn clearing_or_dropping_a_handler_releases_every_queued_event() {
        // Arrange
        let ledger = event_ledger(4, 32, 32);
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new_with_resource_ledger(
            route_protocol(),
            2,
            OverflowPolicy::RejectNewest,
            ledger.clone(),
            retained_console_bytes,
        );
        handler
            .handle(frame(7, ConsoleMessage::EVENT, "one"))
            .expect("first event should fit");
        handler
            .handle(frame_with_sequence(
                7,
                EventSequence::new(2),
                ConsoleMessage::EVENT,
                "two",
            ))
            .expect("second event should fit");

        // Act
        handler.clear();
        let after_clear = ledger.metrics().expect("metrics should be available");
        handler
            .handle(frame_with_sequence(
                8,
                EventSequence::new(3),
                ConsoleMessage::EVENT,
                "three",
            ))
            .expect("event after clear should fit");
        drop(handler);
        let after_drop = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(after_clear.class(ResourceClass::EventItem).current, 0);
        assert_eq!(after_clear.class(ResourceClass::EventBytes).current, 0);
        assert_eq!(after_drop.class(ResourceClass::EventItem).current, 0);
        assert_eq!(after_drop.class(ResourceClass::EventBytes).current, 0);
        assert_eq!(after_drop.active_leases, 0);
    }

    #[test]
    fn unrelated_event_is_preserved_for_another_handler() {
        // Arrange
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new(
            protocol(7),
            1,
            OverflowPolicy::RejectNewest,
        );
        let unrelated = frame(7, EventTag::new(10), "other");
        let expected_route = unrelated.route();

        // Act
        let outcome = handler
            .handle(unrelated)
            .expect("unrelated event should not decode");

        // Assert
        let FrameDisposition::Unhandled(unhandled) = outcome else {
            panic!("unrelated event should remain unhandled");
        };
        assert_eq!(unhandled.route(), expected_route);
        assert_eq!(handler.metrics().current_len, 0);
    }

    #[test]
    fn decoded_source_must_match_the_outer_route() {
        // Arrange
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new(
            protocol(8),
            1,
            OverflowPolicy::RejectNewest,
        );

        // Act
        let error = match handler.handle(frame(7, ConsoleMessage::EVENT, "hello")) {
            Err(error) => error,
            Ok(_) => panic!("mismatched source should fail"),
        };

        // Assert
        assert_eq!(
            error,
            ClientEventError::SourceMismatch {
                route: ObjectId::new(7),
                decoded: ObjectId::new(8),
            }
        );
        assert_eq!(handler.metrics().current_len, 0);
    }

    #[test]
    fn reliable_queue_rejection_returns_the_unqueued_event() {
        // Arrange
        let mut handler = TypedEventHandler::<ConsoleMessage, _>::new(
            protocol(7),
            1,
            OverflowPolicy::RejectNewest,
        );
        handler
            .handle(frame(7, ConsoleMessage::EVENT, "first"))
            .expect("first event should decode");

        // Act
        let outcome = handler
            .handle(frame_with_sequence(
                7,
                EventSequence::new(2),
                ConsoleMessage::EVENT,
                "second",
            ))
            .expect("second event should decode");

        // Assert
        let FrameDisposition::Handled(PushOutcome::Rejected { rejected }) = outcome else {
            panic!("full reliable queue should return its rejected event");
        };
        assert_eq!(rejected.payload(), "second");
        assert_eq!(
            handler
                .pop()
                .expect("first event should remain queued")
                .payload(),
            "first"
        );
        assert_eq!(handler.metrics().rejected_newest, 1);
        assert_eq!(
            handler.sequence_metrics().last_received(),
            Some(EventSequence::new(2))
        );
    }

    #[test]
    fn telemetry_queue_reports_the_dropped_oldest_event() {
        // Arrange
        let mut handler =
            TypedEventHandler::<ConsoleMessage, _>::new(protocol(7), 1, OverflowPolicy::DropOldest);
        handler
            .handle(frame(7, ConsoleMessage::EVENT, "first"))
            .expect("first event should decode");

        // Act
        let outcome = handler
            .handle(frame_with_sequence(
                7,
                EventSequence::new(2),
                ConsoleMessage::EVENT,
                "second",
            ))
            .expect("second event should decode");

        // Assert
        let FrameDisposition::Handled(PushOutcome::DroppedOldest { dropped }) = outcome else {
            panic!("full telemetry queue should report its dropped event");
        };
        assert_eq!(dropped.payload(), "first");
        assert_eq!(
            handler
                .pop()
                .expect("second event should now be queued")
                .payload(),
            "second"
        );
        assert_eq!(handler.metrics().dropped_oldest, 1);
    }

    #[test]
    fn subscription_controls_are_emitted_only_on_reference_count_edges() {
        // Arrange
        let mut subscriptions = SubscriptionTracker::new(1);
        let object_id = ObjectId::new(7);
        let event = ConsoleMessage::EVENT;

        // Act
        let first = subscriptions.subscribe(object_id, event);
        let second = subscriptions.subscribe(object_id, event);
        let remove_second = subscriptions.unsubscribe(object_id, event);
        let remove_last = subscriptions.unsubscribe(object_id, event);

        // Assert
        assert_eq!(
            first,
            Ok(SubscriptionTransition::Send(
                ControlMessage::SetSubscription {
                    object_id,
                    event,
                    enabled: true,
                }
            ))
        );
        assert_eq!(second, Ok(SubscriptionTransition::Unchanged));
        assert_eq!(remove_second, Ok(SubscriptionTransition::Unchanged));
        assert_eq!(
            remove_last,
            Ok(SubscriptionTransition::Send(
                ControlMessage::SetSubscription {
                    object_id,
                    event,
                    enabled: false,
                }
            ))
        );
        assert_eq!(subscriptions.metrics().routes(), 0);
        assert_eq!(subscriptions.metrics().listeners(), 0);
        assert_eq!(subscriptions.metrics().peak_routes(), 1);
        assert_eq!(subscriptions.metrics().peak_listeners(), 2);
    }

    #[test]
    fn subscription_route_capacity_is_a_hard_limit() {
        // Arrange
        let mut subscriptions = SubscriptionTracker::new(1);
        subscriptions
            .subscribe(ObjectId::new(7), ConsoleMessage::EVENT)
            .expect("first route should subscribe");

        // Act
        let outcome = subscriptions.subscribe(ObjectId::new(8), ConsoleMessage::EVENT);

        // Assert
        assert_eq!(
            outcome,
            Err(SubscriptionError::CapacityExceeded { limit: 1 })
        );
        assert_eq!(subscriptions.metrics().routes(), 1);
        assert_eq!(subscriptions.metrics().listeners(), 1);
    }

    #[test]
    fn forgetting_an_invalidated_object_sends_no_unsubscribe() {
        // Arrange
        let mut subscriptions = SubscriptionTracker::new(2);
        subscriptions
            .subscribe(ObjectId::new(7), ConsoleMessage::EVENT)
            .expect("first listener should subscribe");
        subscriptions
            .subscribe(ObjectId::new(7), ConsoleMessage::EVENT)
            .expect("second listener should subscribe");
        subscriptions
            .subscribe(ObjectId::new(8), ConsoleMessage::EVENT)
            .expect("other object should subscribe");

        // Act
        let forgotten = subscriptions.forget_object(ObjectId::new(7));

        // Assert
        assert_eq!(forgotten, 2);
        assert_eq!(subscriptions.metrics().routes(), 1);
        assert_eq!(subscriptions.metrics().listeners(), 1);
    }
}
