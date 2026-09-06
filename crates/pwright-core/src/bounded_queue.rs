use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverflowPolicy {
    RejectNewest,
    DropOldest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PushOutcome<T> {
    Enqueued,
    DroppedOldest { dropped: T },
    Rejected { rejected: T },
}

impl<T> PushOutcome<T> {
    pub const fn was_enqueued(&self) -> bool {
        matches!(self, Self::Enqueued | Self::DroppedOldest { .. })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QueueMetrics {
    pub capacity: usize,
    pub current_len: usize,
    pub peak_len: usize,
    pub enqueued: u64,
    pub dropped_oldest: u64,
    pub rejected_newest: u64,
}

/// A queue whose memory growth and overflow behavior are explicit.
#[derive(Clone, Debug)]
pub struct BoundedQueue<T> {
    capacity: usize,
    policy: OverflowPolicy,
    items: VecDeque<T>,
    peak_len: usize,
    enqueued: u64,
    dropped_oldest: u64,
    rejected_newest: u64,
}

impl<T> BoundedQueue<T> {
    pub fn new(capacity: usize, policy: OverflowPolicy) -> Self {
        Self {
            capacity,
            policy,
            items: VecDeque::new(),
            peak_len: 0,
            enqueued: 0,
            dropped_oldest: 0,
            rejected_newest: 0,
        }
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    pub const fn policy(&self) -> OverflowPolicy {
        self.policy
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn front(&self) -> Option<&T> {
        self.items.front()
    }

    /// Borrows the oldest item for an in-place, caller-validated replacement.
    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.items.front_mut()
    }

    pub fn push(&mut self, item: T) -> PushOutcome<T> {
        self.push_with_reserved_slots(item, 0)
    }

    /// Pushes while treating externally claimed items as occupied slots.
    ///
    /// This supports cancellation-safe consumers that temporarily move the
    /// front item into an owned future. A claimed item remains part of the
    /// queue's logical capacity even though it is not in `items`.
    pub fn push_with_reserved_slots(&mut self, item: T, reserved_slots: usize) -> PushOutcome<T> {
        let occupied = self.items.len().saturating_add(reserved_slots);
        if occupied < self.capacity {
            self.items.push_back(item);
            self.record_enqueue(reserved_slots);
            return PushOutcome::Enqueued;
        }

        match self.policy {
            OverflowPolicy::RejectNewest => {
                self.rejected_newest = self.rejected_newest.saturating_add(1);
                PushOutcome::Rejected { rejected: item }
            }
            OverflowPolicy::DropOldest if !self.items.is_empty() => {
                let dropped = self
                    .items
                    .pop_front()
                    .expect("queue was checked as non-empty");
                self.items.push_back(item);
                self.dropped_oldest = self.dropped_oldest.saturating_add(1);
                self.record_enqueue(reserved_slots);
                PushOutcome::DroppedOldest { dropped }
            }
            OverflowPolicy::DropOldest => {
                self.rejected_newest = self.rejected_newest.saturating_add(1);
                PushOutcome::Rejected { rejected: item }
            }
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        self.items.pop_front()
    }

    /// Restores a previously popped claim without counting a new enqueue.
    pub fn restore_front(&mut self, item: T) -> Result<(), T> {
        if self.items.len() >= self.capacity {
            return Err(item);
        }
        self.items.push_front(item);
        Ok(())
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn metrics(&self) -> QueueMetrics {
        QueueMetrics {
            capacity: self.capacity,
            current_len: self.items.len(),
            peak_len: self.peak_len,
            enqueued: self.enqueued,
            dropped_oldest: self.dropped_oldest,
            rejected_newest: self.rejected_newest,
        }
    }

    fn record_enqueue(&mut self, reserved_slots: usize) {
        self.enqueued = self.enqueued.saturating_add(1);
        self.peak_len = self
            .peak_len
            .max(self.items.len().saturating_add(reserved_slots));
    }
}

#[cfg(test)]
mod tests {
    use super::{BoundedQueue, OverflowPolicy, PushOutcome, QueueMetrics};

    #[test]
    fn reject_newest_preserves_queued_events() {
        // Arrange
        let mut queue = BoundedQueue::new(2, OverflowPolicy::RejectNewest);
        assert_eq!(queue.push("first"), PushOutcome::Enqueued);
        assert_eq!(queue.push("second"), PushOutcome::Enqueued);

        // Act
        let outcome = queue.push("third");

        // Assert
        assert_eq!(outcome, PushOutcome::Rejected { rejected: "third" });
        assert_eq!(queue.pop(), Some("first"));
        assert_eq!(queue.pop(), Some("second"));
    }

    #[test]
    fn drop_oldest_preserves_the_latest_bounded_window() {
        // Arrange
        let mut queue = BoundedQueue::new(2, OverflowPolicy::DropOldest);
        assert_eq!(queue.push(1), PushOutcome::Enqueued);
        assert_eq!(queue.push(2), PushOutcome::Enqueued);

        // Act
        let outcome = queue.push(3);

        // Assert
        assert_eq!(outcome, PushOutcome::DroppedOldest { dropped: 1 });
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(3));
    }

    #[test]
    fn zero_capacity_explicitly_rejects_every_event() {
        // Arrange
        let mut queue = BoundedQueue::new(0, OverflowPolicy::DropOldest);

        // Act
        let outcome = queue.push("event");

        // Assert
        assert_eq!(outcome, PushOutcome::Rejected { rejected: "event" });
        assert!(queue.is_empty());
        assert_eq!(queue.metrics().rejected_newest, 1);
    }

    #[test]
    fn metrics_attribute_accepted_and_dropped_events() {
        // Arrange
        let mut queue = BoundedQueue::new(1, OverflowPolicy::DropOldest);
        assert_eq!(queue.push(1), PushOutcome::Enqueued);
        assert_eq!(queue.push(2), PushOutcome::DroppedOldest { dropped: 1 });

        // Act
        let metrics = queue.metrics();

        // Assert
        assert_eq!(
            metrics,
            QueueMetrics {
                capacity: 1,
                current_len: 1,
                peak_len: 1,
                enqueued: 2,
                dropped_oldest: 1,
                rejected_newest: 0,
            }
        );
    }

    #[test]
    fn reserved_claim_rejects_newest_when_it_alone_fills_capacity() {
        // Arrange
        let mut queue = BoundedQueue::new(1, OverflowPolicy::DropOldest);
        assert_eq!(queue.push("claimed"), PushOutcome::Enqueued);
        let claimed = queue.pop().expect("the oldest item is claimable");

        // Act
        let outcome = queue.push_with_reserved_slots("newest", 1);
        queue
            .restore_front(claimed)
            .expect("the claimed capacity remains restorable");

        // Assert
        assert_eq!(outcome, PushOutcome::Rejected { rejected: "newest" });
        assert_eq!(queue.pop(), Some("claimed"));
        assert_eq!(queue.metrics().rejected_newest, 1);
    }

    #[test]
    fn reserved_claim_keeps_drop_oldest_order_for_the_remaining_queue() {
        // Arrange
        let mut queue = BoundedQueue::new(2, OverflowPolicy::DropOldest);
        assert_eq!(queue.push(1), PushOutcome::Enqueued);
        assert_eq!(queue.push(2), PushOutcome::Enqueued);
        let claimed = queue.pop().expect("the oldest item is claimable");

        // Act
        let outcome = queue.push_with_reserved_slots(3, 1);
        queue
            .restore_front(claimed)
            .expect("the claimed capacity remains restorable");

        // Assert
        assert_eq!(outcome, PushOutcome::DroppedOldest { dropped: 2 });
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(3));
    }

    #[test]
    fn restoring_a_claim_does_not_count_as_a_second_enqueue() {
        // Arrange
        let mut queue = BoundedQueue::new(2, OverflowPolicy::RejectNewest);
        assert_eq!(queue.push("event"), PushOutcome::Enqueued);
        let claimed = queue.pop().expect("the oldest item is claimable");

        // Act
        queue
            .restore_front(claimed)
            .expect("an unchanged claim fits its original queue");

        // Assert
        assert_eq!(queue.metrics().enqueued, 1);
        assert_eq!(queue.metrics().peak_len, 1);
        assert_eq!(queue.pop(), Some("event"));
    }
}
