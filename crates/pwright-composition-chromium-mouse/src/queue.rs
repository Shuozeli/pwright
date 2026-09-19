//! Bounded retained owners. Native polling and waking happen outside locks.
use crate::{ChromiumMouseInstallError, MouseResult};
use pwright_backend_cdp_mouse::{CdpMouseOperation, CdpMouseServices};
use pwright_core::{
    CancellationReason, FailureKind, ObjectId, OperationContext, OperationError, ResourceClass,
    ResourceLease, ResourceLedger, ResourceScope,
};
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};

pub(crate) fn protocol_error(
    message: &str,
) -> pwright_core::OperationError<pwright_cap_mouse::MouseErrorDetails> {
    OperationError::new(FailureKind::ProtocolViolation, message, None)
}
pub(crate) fn resource_error(
    message: &str,
) -> pwright_core::OperationError<pwright_cap_mouse::MouseErrorDetails> {
    OperationError::new(FailureKind::ResourceExhausted, message, None)
}
pub(crate) fn cancelled(
    reason: CancellationReason,
) -> pwright_core::OperationError<pwright_cap_mouse::MouseErrorDetails> {
    let (kind, message) = match reason {
        CancellationReason::Caller => (FailureKind::Cancelled, "Mouse call cancelled"),
        CancellationReason::DeadlineExceeded => {
            (FailureKind::Timeout, "Mouse call deadline exceeded")
        }
        CancellationReason::ParentClosed => {
            (FailureKind::TargetClosed, "Mouse Page admission closed")
        }
        CancellationReason::TransportClosed => {
            (FailureKind::TransportClosed, "Mouse transport closed")
        }
    };
    OperationError::new(kind, message, None)
}
pub(crate) struct Shared {
    pub page_id: ObjectId,
    pub revoked: AtomicBool,
    pub backend: CdpMouseServices,
    slots: Mutex<Slots>,
    ledger: ResourceLedger,
    active: Arc<AtomicUsize>,
    _route: ResourceLease,
}
struct Slots {
    entries: Box<[Option<Arc<Record>>]>,
    driver: Option<Waker>,
}
struct Record {
    context: OperationContext,
    state: Mutex<RecordState>,
    _permit: RecordPermit,
}
struct RecordState {
    operation: Option<CdpMouseOperation>,
    busy: bool,
    polled: bool,
    observed: bool,
    retiring: bool,
    cancellation: Option<CancellationReason>,
    response: Option<Waker>,
    #[cfg(feature = "runtime")]
    completion: Completion,
}
#[cfg(feature = "runtime")]
#[derive(Default)]
struct Completion {
    interested: bool,
    outcome: Option<MouseResult>,
    observed: bool,
    waiting: bool,
    wake: Option<Waker>,
}
struct RecordPermit {
    resource: Option<ResourceLease>,
    active: Arc<AtomicUsize>,
}
impl Drop for RecordPermit {
    fn drop(&mut self) {
        drop(self.resource.take());
        let previous = self.active.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}
impl Shared {
    pub fn new(
        page_id: ObjectId,
        backend: CdpMouseServices,
        capacity: usize,
        ledger: ResourceLedger,
        route: ResourceLease,
    ) -> Self {
        Self {
            page_id,
            revoked: AtomicBool::new(false),
            backend,
            slots: Mutex::new(Slots {
                entries: std::iter::repeat_with(|| None).take(capacity).collect(),
                driver: None,
            }),
            ledger,
            active: Arc::new(AtomicUsize::new(0)),
            _route: route,
        }
    }
    pub fn allocation_bytes(capacity: usize) -> Result<usize, ChromiumMouseInstallError> {
        let entries = capacity
            .checked_mul(std::mem::size_of::<Option<Arc<Record>>>())
            .ok_or(ChromiumMouseInstallError::CapacityOverflow)?;
        // Shared Arc, record counter Arc and backend binding. Caller-owned
        // response futures, transport and executor waker allocations are separate.
        entries
            .checked_add(
                std::mem::size_of::<Self>()
                    + 4 * std::mem::size_of::<usize>()
                    + std::mem::size_of::<AtomicUsize>()
                    + CdpMouseServices::shared_binding_bytes(),
            )
            .ok_or(ChromiumMouseInstallError::CapacityOverflow)
    }
    pub const fn record_bytes() -> usize {
        // A controlled receipt retains its own bounded diagnostic copy after
        // the native owner is released. No provider diagnostics are unbounded.
        let diagnostics = if cfg!(feature = "runtime") {
            2 * pwright_backend_cdp_mouse::MOUSE_ERROR_MAX_BYTES
        } else {
            0
        };
        std::mem::size_of::<Record>() + 2 * std::mem::size_of::<usize>() + diagnostics
    }
    pub fn retained_records(&self) -> usize {
        self.active.load(Ordering::Acquire)
    }
    pub fn queued_operations(&self) -> usize {
        self.slots
            .lock()
            .expect("Mouse queue poisoned")
            .entries
            .iter()
            .flatten()
            .count()
    }
    fn wake_driver(&self) {
        let wake = self
            .slots
            .lock()
            .expect("Mouse queue poisoned")
            .driver
            .take();
        if let Some(wake) = wake {
            wake.wake();
        }
    }
    pub fn validate_identity(&self, actual: ObjectId) -> MouseResult {
        if actual != self.page_id {
            return Err(protocol_error("Mouse command targeted an unbound Page"));
        }
        Ok(())
    }
    pub fn admit(
        self: &Arc<Self>,
        context: OperationContext,
        start: impl FnOnce() -> Result<
            CdpMouseOperation,
            OperationError<pwright_cap_mouse::MouseErrorDetails>,
        >,
    ) -> Response {
        let admitted = (|| {
            self.validate_identity(context.object_id())?;
            let mut slots = self.slots.lock().expect("Mouse queue poisoned");
            if self.revoked.load(Ordering::Acquire) {
                return Err(cancelled(CancellationReason::ParentClosed));
            }
            if slots
                .entries
                .iter()
                .flatten()
                .any(|r| r.context.call_id() == context.call_id())
            {
                return Err(protocol_error("Mouse call ID already retained"));
            }
            let index = slots
                .entries
                .iter()
                .position(Option::is_none)
                .ok_or_else(|| resource_error("Mouse retirement slots exhausted"))?;
            let limit = slots.entries.len();
            if self
                .active
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                    (n < limit).then(|| n + 1)
                })
                .is_err()
            {
                return Err(resource_error(
                    "Mouse records retained by replies or retirement receipts",
                ));
            }
            let resource = match self.ledger.reserve(
                ResourceScope::Object(self.page_id),
                ResourceClass::BackendStateBytes,
                u64::try_from(Self::record_bytes()).expect("record size fits u64"),
            ) {
                Ok(resource) => resource,
                Err(_) => {
                    self.active.fetch_sub(1, Ordering::AcqRel);
                    return Err(resource_error("Mouse record lease could not be reserved"));
                }
            };
            let permit = RecordPermit {
                resource: Some(resource),
                active: self.active.clone(),
            };
            // All six closed native starters are effect-free; they do not poll a
            // provider or invoke user callbacks while this queue lock is held.
            let operation = start()?;
            let record = Arc::new(Record {
                context,
                state: Mutex::new(RecordState {
                    operation: Some(operation),
                    busy: false,
                    polled: false,
                    observed: false,
                    retiring: false,
                    cancellation: None,
                    response: None,
                    #[cfg(feature = "runtime")]
                    completion: Completion::default(),
                }),
                _permit: permit,
            });
            slots.entries[index] = Some(record.clone());
            Ok(record)
        })();
        match admitted {
            Ok(record) => Response {
                shared: self.clone(),
                record: Some(record),
                rejected: None,
                done: false,
            },
            Err(error) => Response {
                shared: self.clone(),
                record: None,
                rejected: Some(error),
                done: false,
            },
        }
    }
    fn cancel_record(&self, record: &Record, reason: CancellationReason) {
        let wake = {
            let mut state = record.state.lock().expect("Mouse record poisoned");
            if !state.observed {
                state.cancellation.get_or_insert(reason);
            }
            state.retiring = true;
            state.response.take()
        };
        if let Some(wake) = wake {
            wake.wake();
        }
        self.wake_driver();
    }
    /// Request acknowledgement only. Native completion is a separate receipt.
    pub fn request_cancel(
        &self,
        context: OperationContext,
        reason: CancellationReason,
    ) -> MouseResult {
        self.validate_identity(context.object_id())?;
        let record = self
            .slots
            .lock()
            .expect("Mouse queue poisoned")
            .entries
            .iter()
            .flatten()
            .find(|record| record.context.call_id() == context.call_id())
            .cloned()
            .ok_or_else(|| protocol_error("Mouse cancellation has no retained call"))?;
        if record.context != context {
            return Err(protocol_error(
                "Mouse cancellation identity differs from retained call",
            ));
        }
        self.cancel_record(&record, reason);
        Ok(())
    }
    pub fn request_cancel_all(&self, reason: CancellationReason) {
        let capacity = self
            .slots
            .lock()
            .expect("Mouse queue poisoned")
            .entries
            .len();
        for index in 0..capacity {
            let record = self.slots.lock().expect("Mouse queue poisoned").entries[index].clone();
            if let Some(record) = record {
                self.cancel_record(&record, reason);
            }
        }
    }
    fn poll_result(&self, record: &Arc<Record>, cx: &mut Context<'_>) -> Poll<MouseResult> {
        let mut operation = {
            let mut state = record.state.lock().expect("Mouse record poisoned");
            if !state.polled && self.revoked.load(Ordering::Acquire) {
                state
                    .cancellation
                    .get_or_insert(CancellationReason::ParentClosed);
                state.retiring = true;
            }
            if let Some(reason) = state.cancellation {
                state.observed = true;
                state.response = None;
                drop(state);
                self.wake_driver();
                return Poll::Ready(Err(cancelled(reason)));
            }
            if state.observed {
                return Poll::Ready(Err(protocol_error("Mouse result already observed")));
            }
            state.response = Some(cx.waker().clone());
            if state.busy {
                return Poll::Pending;
            }
            state.polled = true;
            state.busy = true;
            state
                .operation
                .take()
                .expect("active Mouse operation is retained")
        };
        // No mutex spans native polling. Cancellation may mark this record
        // retiring while the operation is temporarily borrowed by this poll.
        let outcome = std::pin::pin!(operation.result()).as_mut().poll(cx);
        let (outcome, retiring) = {
            let mut state = record.state.lock().expect("Mouse record poisoned");
            state.operation = Some(operation);
            state.busy = false;
            let outcome = match state.cancellation {
                Some(reason) => Poll::Ready(Err(cancelled(reason))),
                None => outcome,
            };
            if outcome.is_ready() {
                state.observed = true;
                state.retiring = true;
                state.response = None;
            }
            (outcome, state.retiring)
        };
        if retiring {
            self.wake_driver();
        }
        outcome
    }
}
pub(crate) struct Response {
    shared: Arc<Shared>,
    record: Option<Arc<Record>>,
    rejected: Option<OperationError<pwright_cap_mouse::MouseErrorDetails>>,
    done: bool,
}
#[cfg(feature = "runtime")]
impl Response {
    pub(crate) fn retirement_ticket(&self) -> Option<RetirementTicket> {
        self.record.as_ref().map(|record| {
            record
                .state
                .lock()
                .expect("Mouse record poisoned")
                .completion
                .interested = true;
            RetirementTicket {
                shared: self.shared.clone(),
                record: record.clone(),
            }
        })
    }
}
impl Future for Response {
    type Output = MouseResult;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        assert!(!self.done, "completed Mouse response polled again");
        if let Some(error) = self.rejected.take() {
            self.done = true;
            return Poll::Ready(Err(error));
        }
        let outcome = self
            .shared
            .poll_result(self.record.as_ref().expect("admitted Mouse response"), cx);
        if outcome.is_ready() {
            self.done = true;
            self.record = None;
        }
        outcome
    }
}
impl Drop for Response {
    fn drop(&mut self) {
        if !self.done
            && let Some(record) = self.record.take()
        {
            self.shared
                .cancel_record(&record, CancellationReason::Caller);
        }
    }
}
/// Retains metadata and native admission until the caller consumes/drops it.
/// A successful cancellation request is not interchangeable with this receipt.
pub struct MouseRetirementReceipt {
    context: OperationContext,
    outcome: MouseResult,
    _operation: CdpMouseOperation,
    _record: Arc<Record>,
}
impl MouseRetirementReceipt {
    pub const fn context(&self) -> OperationContext {
        self.context
    }
    pub const fn outcome(&self) -> &MouseResult {
        &self.outcome
    }
    /// Consume the receipt and release native admission. With `runtime`, this
    /// also delivers a bounded copy to its interested invocation's cleanup
    /// waiter. Dropping a receipt without consuming it does not acknowledge
    /// runtime cleanup. The driver consumer must handle this returned outcome
    /// even when the operation result has already been published.
    pub fn into_outcome(self) -> MouseResult {
        let Self {
            outcome,
            _operation: operation,
            _record: record,
            ..
        } = self;
        // Release native admission before waking a runtime cleanup waiter.
        drop(operation);
        #[cfg(feature = "runtime")]
        {
            let wake = {
                let mut state = record.state.lock().expect("Mouse record poisoned");
                if state.completion.interested {
                    state.completion.outcome = Some(outcome.clone());
                }
                state.completion.wake.take()
            };
            if let Some(wake) = wake {
                wake.wake();
            }
        }
        drop(record);
        outcome
    }
}
/// A route-record reference retained by one controlled invocation.
#[cfg(feature = "runtime")]
#[derive(Clone)]
pub(crate) struct RetirementTicket {
    shared: Arc<Shared>,
    record: Arc<Record>,
}
#[cfg(feature = "runtime")]
impl RetirementTicket {
    pub(crate) fn cancel(self, reason: CancellationReason) -> RetirementWait {
        self.shared.cancel_record(&self.record, reason);
        RetirementWait {
            ticket: self,
            claimed: false,
            done: false,
        }
    }
}
#[cfg(feature = "runtime")]
pub(crate) struct RetirementWait {
    ticket: RetirementTicket,
    claimed: bool,
    done: bool,
}
#[cfg(feature = "runtime")]
impl Future for RetirementWait {
    type Output = MouseResult;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<MouseResult> {
        let this = self.get_mut();
        assert!(!this.done, "completed Mouse retirement wait polled again");
        let mut state = this
            .ticket
            .record
            .state
            .lock()
            .expect("Mouse record poisoned");
        let completion = &mut state.completion;
        if completion.observed || (!this.claimed && completion.waiting) {
            this.done = true;
            return Poll::Ready(Err(protocol_error(
                "Mouse retirement has another or completed waiter",
            )));
        }
        this.claimed = true;
        completion.waiting = true;
        if let Some(outcome) = completion.outcome.take() {
            completion.observed = true;
            completion.waiting = false;
            completion.wake = None;
            this.done = true;
            return Poll::Ready(outcome);
        }
        completion.wake = Some(cx.waker().clone());
        Poll::Pending
    }
}
#[cfg(feature = "runtime")]
impl Drop for RetirementWait {
    fn drop(&mut self) {
        if self.claimed && !self.done {
            let wake = {
                let mut state = self
                    .ticket
                    .record
                    .state
                    .lock()
                    .expect("Mouse record poisoned");
                state.completion.waiting = false;
                state.completion.wake.take()
            };
            drop(wake);
        }
    }
}

/// Unique caller-polled driver; the embedding must consume each receipt.
pub struct MouseRetirementDriver {
    shared: Arc<Shared>,
    cursor: usize,
}
impl Drop for MouseRetirementDriver {
    fn drop(&mut self) {
        // Do not retain a stale executor waker after the unique driver is gone.
        let wake = self
            .shared
            .slots
            .lock()
            .expect("Mouse queue poisoned")
            .driver
            .take();
        drop(wake);
    }
}
impl MouseRetirementDriver {
    pub(crate) fn new(shared: Arc<Shared>) -> Self {
        Self { shared, cursor: 0 }
    }
    pub fn queued_operations(&self) -> usize {
        self.shared.queued_operations()
    }
    pub async fn next(&mut self) -> MouseRetirementReceipt {
        poll_fn(|cx| self.poll_next(cx)).await
    }
    pub fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<MouseRetirementReceipt> {
        let capacity = {
            let mut slots = self.shared.slots.lock().expect("Mouse queue poisoned");
            slots.driver = Some(cx.waker().clone());
            slots.entries.len()
        };
        for _ in 0..capacity {
            let index = self.cursor;
            self.cursor = (self.cursor + 1) % capacity;
            let Some(record) = self
                .shared
                .slots
                .lock()
                .expect("Mouse queue poisoned")
                .entries[index]
                .clone()
            else {
                continue;
            };
            let mut operation = {
                let mut state = record.state.lock().expect("Mouse record poisoned");
                if !state.retiring || state.busy {
                    continue;
                }
                let Some(operation) = state.operation.take() else {
                    continue;
                };
                state.busy = true;
                operation
            };
            let outcome = std::pin::pin!(operation.retire()).as_mut().poll(cx);
            match outcome {
                Poll::Pending => {
                    let mut state = record.state.lock().expect("Mouse record poisoned");
                    state.operation = Some(operation);
                    state.busy = false;
                }
                Poll::Ready(outcome) => {
                    {
                        let mut state = record.state.lock().expect("Mouse record poisoned");
                        state.busy = false;
                    }
                    let mut slots = self.shared.slots.lock().expect("Mouse queue poisoned");
                    let current = slots.entries[index]
                        .as_ref()
                        .expect("retiring record retains slot");
                    assert!(
                        Arc::ptr_eq(current, &record),
                        "retirement must remove the original record"
                    );
                    slots.entries[index] = None;
                    return Poll::Ready(MouseRetirementReceipt {
                        context: record.context,
                        outcome,
                        _operation: operation,
                        _record: record,
                    });
                }
            }
        }
        Poll::Pending
    }
}
