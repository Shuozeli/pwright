//! Bounded caller-driven event fanout with prompt payload reclamation.
//!
//! Unlike a replay ring, a slot is removed as soon as every subscriber that
//! existed when it was published has advanced beyond it. The fanout starts no
//! task and never hides a slow subscriber: bounded overwrite is reported as an
//! explicit lag error on that subscriber's next receive.

use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::sync::Notify;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReclaimingFanoutMetrics {
    pub subscribers: usize,
    pub peak_subscribers: usize,
    pub queued_events: usize,
    pub peak_queued_events: usize,
    pub dropped_without_subscribers: u64,
    pub lagged_deliveries: u64,
}

#[derive(Debug)]
pub struct ReclaimingEventFanout<Event> {
    shared: Arc<FanoutShared<Event>>,
}

impl<Event> ReclaimingEventFanout<Event> {
    /// Maximum logical bytes retained by the preallocated slot and subscriber
    /// bookkeeping. Event payload bytes remain owned and metered separately.
    pub fn maximum_retained_bookkeeping_bytes(
        event_capacity: usize,
        subscriber_capacity: usize,
    ) -> u64 {
        let fixed = std::mem::size_of::<FanoutShared<Event>>()
            .saturating_add(std::mem::size_of::<FanoutState<Event>>())
            .saturating_add(2 * std::mem::size_of::<usize>());
        let events = std::mem::size_of::<EventSlot<Event>>().saturating_mul(event_capacity);
        let subscribers =
            std::mem::size_of::<Option<SubscriberState>>().saturating_mul(subscriber_capacity);
        u64::try_from(fixed.saturating_add(events).saturating_add(subscribers)).unwrap_or(u64::MAX)
    }

    pub fn new(
        event_capacity: usize,
        subscriber_capacity: usize,
    ) -> Result<Self, ReclaimingFanoutConfigError> {
        if event_capacity == 0 {
            return Err(ReclaimingFanoutConfigError::InvalidLimit("events"));
        }
        if subscriber_capacity == 0 {
            return Err(ReclaimingFanoutConfigError::InvalidLimit("subscribers"));
        }
        Ok(Self {
            shared: Arc::new(FanoutShared {
                event_capacity,
                subscriber_capacity,
                state: Mutex::new(FanoutState {
                    events: VecDeque::with_capacity(event_capacity),
                    subscribers: Vec::with_capacity(subscriber_capacity),
                    subscriber_count: 0,
                    peak_subscribers: 0,
                    next_sequence: 0,
                    closed: false,
                    peak_queued_events: 0,
                    dropped_without_subscribers: 0,
                    lagged_deliveries: 0,
                }),
                changed: Notify::new(),
            }),
        })
    }

    pub fn subscribe(
        &self,
    ) -> Result<ReclaimingEventReceiver<Event>, ReclaimingFanoutSubscribeError> {
        let mut state = self.shared.state();
        if state.closed {
            return Err(ReclaimingFanoutSubscribeError::Closed);
        }
        if state.subscriber_count == self.shared.subscriber_capacity {
            return Err(ReclaimingFanoutSubscribeError::CapacityExceeded {
                limit: self.shared.subscriber_capacity,
            });
        }
        let subscriber = state
            .subscribers
            .iter()
            .position(Option::is_none)
            .unwrap_or(state.subscribers.len());
        let next_sequence = state.next_sequence;
        let subscriber_state = Some(SubscriberState {
            next_sequence,
            lagged: 0,
        });
        if subscriber == state.subscribers.len() {
            state.subscribers.push(subscriber_state);
        } else {
            state.subscribers[subscriber] = subscriber_state;
        }
        state.subscriber_count += 1;
        state.peak_subscribers = state.peak_subscribers.max(state.subscriber_count);
        drop(state);
        Ok(ReclaimingEventReceiver {
            shared: Arc::clone(&self.shared),
            subscriber,
        })
    }

    pub fn send(&self, event: Event) -> Result<usize, ReclaimingFanoutSendError<Event>> {
        let mut state = self.shared.state();
        if state.closed {
            return Err(ReclaimingFanoutSendError::Closed(event));
        }
        if state.subscriber_count == 0 {
            state.dropped_without_subscribers = state.dropped_without_subscribers.saturating_add(1);
            return Ok(0);
        }
        let sequence = state.next_sequence;
        let Some(next_sequence) = sequence.checked_add(1) else {
            state.closed = true;
            drop(state);
            self.shared.changed.notify_waiters();
            return Err(ReclaimingFanoutSendError::SequenceExhausted(event));
        };
        state.next_sequence = next_sequence;
        if state.events.len() == self.shared.event_capacity {
            let evicted = state
                .events
                .pop_front()
                .expect("a full fanout contains an event");
            let mut lagged_deliveries = 0_u64;
            for subscriber in state.subscribers.iter_mut().flatten() {
                if subscriber.next_sequence <= evicted.sequence {
                    let skipped = evicted
                        .sequence
                        .saturating_add(1)
                        .saturating_sub(subscriber.next_sequence);
                    subscriber.next_sequence = evicted.sequence.saturating_add(1);
                    subscriber.lagged = subscriber.lagged.saturating_add(skipped);
                    lagged_deliveries = lagged_deliveries.saturating_add(skipped);
                }
            }
            state.lagged_deliveries = state.lagged_deliveries.saturating_add(lagged_deliveries);
        }
        state.events.push_back(EventSlot { sequence, event });
        state.peak_queued_events = state.peak_queued_events.max(state.events.len());
        let subscribers = state.subscriber_count;
        drop(state);
        self.shared.changed.notify_waiters();
        Ok(subscribers)
    }

    pub fn close(&self) {
        let mut state = self.shared.state();
        state.closed = true;
        drop(state);
        self.shared.changed.notify_waiters();
    }

    pub fn metrics(&self) -> ReclaimingFanoutMetrics {
        let state = self.shared.state();
        ReclaimingFanoutMetrics {
            subscribers: state.subscriber_count,
            peak_subscribers: state.peak_subscribers,
            queued_events: state.events.len(),
            peak_queued_events: state.peak_queued_events,
            dropped_without_subscribers: state.dropped_without_subscribers,
            lagged_deliveries: state.lagged_deliveries,
        }
    }
}

impl<Event> Clone for ReclaimingEventFanout<Event> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

#[derive(Debug)]
pub struct ReclaimingEventReceiver<Event> {
    shared: Arc<FanoutShared<Event>>,
    subscriber: usize,
}

impl<Event: Clone> ReclaimingEventReceiver<Event> {
    /// Advances this cursor to the current end without allocating another
    /// subscriber slot. Unread payloads are reclaimed if no other cursor needs
    /// them.
    pub fn resubscribe(&mut self) {
        let mut state = self.shared.state();
        let next_sequence = state.next_sequence;
        if let Some(subscriber) = state
            .subscribers
            .get_mut(self.subscriber)
            .and_then(Option::as_mut)
        {
            subscriber.next_sequence = next_sequence;
            subscriber.lagged = 0;
            reclaim_consumed(&mut state);
        }
    }

    pub async fn recv(&mut self) -> Result<Event, ReclaimingFanoutRecvError> {
        loop {
            let shared = Arc::clone(&self.shared);
            let changed = shared.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            match self.try_recv() {
                Ok(event) => return Ok(event),
                Err(ReclaimingFanoutTryRecvError::Empty) => changed.await,
                Err(ReclaimingFanoutTryRecvError::Closed) => {
                    return Err(ReclaimingFanoutRecvError::Closed);
                }
                Err(ReclaimingFanoutTryRecvError::Lagged(skipped)) => {
                    return Err(ReclaimingFanoutRecvError::Lagged(skipped));
                }
            }
        }
    }

    pub fn try_recv(&mut self) -> Result<Event, ReclaimingFanoutTryRecvError> {
        let mut state = self.shared.state();
        let Some(subscriber) = state
            .subscribers
            .get_mut(self.subscriber)
            .and_then(Option::as_mut)
        else {
            return Err(ReclaimingFanoutTryRecvError::Closed);
        };
        if subscriber.lagged != 0 {
            let skipped = subscriber.lagged;
            subscriber.lagged = 0;
            return Err(ReclaimingFanoutTryRecvError::Lagged(skipped));
        }
        let next_sequence = subscriber.next_sequence;
        let event = state.events.front().and_then(|front| {
            let offset = next_sequence.checked_sub(front.sequence)?;
            usize::try_from(offset)
                .ok()
                .and_then(|offset| state.events.get(offset))
                .map(|slot| slot.event.clone())
        });
        if let Some(event) = event {
            state
                .subscribers
                .get_mut(self.subscriber)
                .and_then(Option::as_mut)
                .expect("receiver remains registered while locked")
                .next_sequence = next_sequence.saturating_add(1);
            reclaim_consumed(&mut state);
            return Ok(event);
        }
        if state.closed {
            Err(ReclaimingFanoutTryRecvError::Closed)
        } else {
            Err(ReclaimingFanoutTryRecvError::Empty)
        }
    }
}

impl<Event> Drop for ReclaimingEventReceiver<Event> {
    fn drop(&mut self) {
        let mut state = self.shared.state();
        if let Some(slot) = state.subscribers.get_mut(self.subscriber)
            && slot.take().is_some()
        {
            state.subscriber_count -= 1;
        }
        reclaim_consumed(&mut state);
    }
}

#[derive(Debug)]
struct FanoutShared<Event> {
    event_capacity: usize,
    subscriber_capacity: usize,
    state: Mutex<FanoutState<Event>>,
    changed: Notify,
}

impl<Event> FanoutShared<Event> {
    fn state(&self) -> MutexGuard<'_, FanoutState<Event>> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[derive(Debug)]
struct FanoutState<Event> {
    events: VecDeque<EventSlot<Event>>,
    subscribers: Vec<Option<SubscriberState>>,
    subscriber_count: usize,
    peak_subscribers: usize,
    next_sequence: u64,
    closed: bool,
    peak_queued_events: usize,
    dropped_without_subscribers: u64,
    lagged_deliveries: u64,
}

#[derive(Debug)]
struct EventSlot<Event> {
    sequence: u64,
    event: Event,
}

#[derive(Clone, Copy, Debug)]
struct SubscriberState {
    next_sequence: u64,
    lagged: u64,
}

fn reclaim_consumed<Event>(state: &mut FanoutState<Event>) {
    let Some(minimum_next) = state
        .subscribers
        .iter()
        .flatten()
        .map(|subscriber| subscriber.next_sequence)
        .min()
    else {
        state.events.clear();
        return;
    };
    while state
        .events
        .front()
        .is_some_and(|event| event.sequence < minimum_next)
    {
        state.events.pop_front();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReclaimingFanoutConfigError {
    InvalidLimit(&'static str),
}

impl Display for ReclaimingFanoutConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit(name) => write!(formatter, "{name} limit must be non-zero"),
        }
    }
}

impl std::error::Error for ReclaimingFanoutConfigError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReclaimingFanoutSubscribeError {
    Closed,
    CapacityExceeded { limit: usize },
}

impl Display for ReclaimingFanoutSubscribeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("event fanout is closed"),
            Self::CapacityExceeded { limit } => {
                write!(formatter, "event subscriber capacity {limit} is exhausted")
            }
        }
    }
}

impl std::error::Error for ReclaimingFanoutSubscribeError {}

#[derive(Debug, Eq, PartialEq)]
pub enum ReclaimingFanoutSendError<Event> {
    Closed(Event),
    SequenceExhausted(Event),
}

impl<Event> Display for ReclaimingFanoutSendError<Event> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed(_) => formatter.write_str("event fanout is closed"),
            Self::SequenceExhausted(_) => formatter.write_str("event fanout sequence is exhausted"),
        }
    }
}

impl<Event: fmt::Debug> std::error::Error for ReclaimingFanoutSendError<Event> {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReclaimingFanoutRecvError {
    Closed,
    Lagged(u64),
}

impl Display for ReclaimingFanoutRecvError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("event fanout is closed"),
            Self::Lagged(skipped) => write!(formatter, "event receiver lagged by {skipped}"),
        }
    }
}

impl std::error::Error for ReclaimingFanoutRecvError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReclaimingFanoutTryRecvError {
    Empty,
    Closed,
    Lagged(u64),
}

impl Display for ReclaimingFanoutTryRecvError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("event fanout is empty"),
            Self::Closed => formatter.write_str("event fanout is closed"),
            Self::Lagged(skipped) => write!(formatter, "event receiver lagged by {skipped}"),
        }
    }
}

impl std::error::Error for ReclaimingFanoutTryRecvError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn event_is_reclaimed_after_the_last_existing_subscriber_reads_it() {
        // Arrange
        let fanout = ReclaimingEventFanout::new(2, 2).expect("fanout should validate");
        let mut first = fanout.subscribe().expect("first subscriber should fit");
        let mut second = fanout.subscribe().expect("second subscriber should fit");
        let payload = Arc::new("event");
        assert_eq!(fanout.send(Arc::clone(&payload)), Ok(2));

        // Act
        let first_event = first.recv().await.expect("first should receive");
        let after_first = fanout.metrics();
        drop(first_event);
        let second_event = second.recv().await.expect("second should receive");
        let after_second = fanout.metrics();

        // Assert
        assert_eq!(*second_event, "event");
        assert_eq!(after_first.queued_events, 1);
        assert_eq!(after_second.queued_events, 0);
        assert_eq!(after_second.peak_queued_events, 1);
        assert_eq!(
            Arc::strong_count(&payload),
            2,
            "only the caller and delivered value may retain the payload"
        );
    }

    #[test]
    fn slow_subscriber_receives_explicit_lag_before_the_retained_event() {
        // Arrange
        let fanout = ReclaimingEventFanout::new(1, 1).expect("fanout should validate");
        let mut receiver = fanout.subscribe().expect("subscriber should fit");
        fanout.send(1).expect("first event should send");

        // Act
        fanout
            .send(2)
            .expect("second event should replace the first");
        let lag = receiver.try_recv();
        let retained = receiver.try_recv();

        // Assert
        assert_eq!(lag, Err(ReclaimingFanoutTryRecvError::Lagged(1)));
        assert_eq!(retained, Ok(2));
        assert_eq!(fanout.metrics().queued_events, 0);
        assert_eq!(fanout.metrics().lagged_deliveries, 1);
    }

    #[test]
    fn dropping_the_last_subscriber_reclaims_unread_events() {
        // Arrange
        let fanout = ReclaimingEventFanout::new(2, 1).expect("fanout should validate");
        let receiver = fanout.subscribe().expect("subscriber should fit");
        fanout.send("unread").expect("event should send");

        // Act
        drop(receiver);

        // Assert
        assert_eq!(fanout.metrics().subscribers, 0);
        assert_eq!(fanout.metrics().queued_events, 0);
    }

    #[test]
    fn subscriber_count_has_a_hard_limit() {
        // Arrange
        let fanout = ReclaimingEventFanout::<()>::new(1, 2).expect("fanout should validate");
        let first = fanout.subscribe().expect("first subscriber should fit");
        let second = fanout.subscribe().expect("second subscriber should fit");

        // Act
        let error = fanout
            .subscribe()
            .expect_err("third subscriber should exceed capacity");
        drop(first);
        drop(second);

        // Assert
        assert_eq!(
            error,
            ReclaimingFanoutSubscribeError::CapacityExceeded { limit: 2 }
        );
        assert_eq!(fanout.metrics().subscribers, 0);
        assert_eq!(fanout.metrics().peak_subscribers, 2);
    }

    #[tokio::test]
    async fn new_subscriber_sees_only_future_events_and_close_drains_the_queue() {
        // Arrange
        let fanout = ReclaimingEventFanout::new(2, 2).expect("fanout should validate");
        let mut first = fanout.subscribe().expect("first subscriber should fit");
        fanout.send(1).expect("historical event should send");
        let mut second = fanout.subscribe().expect("second subscriber should fit");
        fanout.send(2).expect("shared event should send");
        fanout.close();

        // Act
        let first_values = [first.recv().await, first.recv().await];
        let first_closed = first.recv().await;
        let second_value = second.recv().await;
        let second_closed = second.recv().await;

        // Assert
        assert_eq!(first_values, [Ok(1), Ok(2)]);
        assert_eq!(first_closed, Err(ReclaimingFanoutRecvError::Closed));
        assert_eq!(second_value, Ok(2));
        assert_eq!(second_closed, Err(ReclaimingFanoutRecvError::Closed));
        assert_eq!(fanout.metrics().queued_events, 0);
    }

    #[test]
    fn event_without_subscribers_is_not_retained() {
        // Arrange
        let fanout = ReclaimingEventFanout::new(1, 1).expect("fanout should validate");

        // Act
        let receivers = fanout.send("unused").expect("open fanout should accept");

        // Assert
        assert_eq!(receivers, 0);
        assert_eq!(fanout.metrics().queued_events, 0);
        assert_eq!(fanout.metrics().dropped_without_subscribers, 1);
    }

    #[test]
    fn resubscribe_discards_only_that_cursor_unread_history() {
        // Arrange
        let fanout = ReclaimingEventFanout::new(2, 1).expect("fanout should validate");
        let mut receiver = fanout.subscribe().expect("subscriber should fit");
        fanout.send("old").expect("old event should send");

        // Act
        receiver.resubscribe();
        let after = receiver.try_recv();

        // Assert
        assert_eq!(after, Err(ReclaimingFanoutTryRecvError::Empty));
        assert_eq!(fanout.metrics().queued_events, 0);
    }
}
