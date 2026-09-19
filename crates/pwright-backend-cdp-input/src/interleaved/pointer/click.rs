//! Caller-polled click scheduling with separate result and retirement boundaries.
use super::dispatch::{
    CdpMouseDispatcher, DirectMouseDispatch, DispatchCall, MouseDispatchKind, MouseDispatchRequest,
    MouseStateReader,
};
use super::{begin_move, down_event, finite_pair, move_event, up_event};
use crate::{
    interleaved::{Action, PageState},
    production::InputAdmission,
};
use pwright_cdp::{
    CdpClient, MouseButton,
    connection::{CdpError, Result},
    generated::input::DispatchMouseEventParams,
};
use pwright_input_model::{INPUT_MOUSE_MAX_CLICKS, MouseClickCount, MouseMoveSteps};
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

const SLOTS: usize = INPUT_MOUSE_MAX_CLICKS as usize * 2 + 1;

#[derive(Clone, Copy, Debug)]
pub struct MouseClickOptions {
    x: f64,
    y: f64,
    button: MouseButton,
    count: MouseClickCount,
    steps: MouseMoveSteps,
    delay: Duration,
    for_click: bool,
}
impl MouseClickOptions {
    pub fn new(x: f64, y: f64) -> Result<Self> {
        finite_pair(x, y)?;
        Ok(Self {
            x,
            y,
            button: MouseButton::Left,
            count: MouseClickCount::ONE,
            steps: MouseMoveSteps::ONE,
            delay: Duration::ZERO,
            for_click: true,
        })
    }
    pub const fn with_button(mut self, value: MouseButton) -> Self {
        self.button = value;
        self
    }
    pub const fn with_count(mut self, value: MouseClickCount) -> Self {
        self.count = value;
        self
    }
    pub const fn with_steps(mut self, value: MouseMoveSteps) -> Self {
        self.steps = value;
        self
    }
    pub const fn with_delay(mut self, value: Duration) -> Self {
        self.delay = value;
        self
    }
    /// Ordinary move uses false; even a zero-count click keeps the click bypass.
    pub const fn with_for_click(mut self, value: bool) -> Self {
        self.for_click = value;
        self
    }
}

#[derive(Clone, Copy)]
enum Stage {
    Initial,
    Moving { from: (f64, f64), next: u32 },
    Batch,
    Down(u8),
    WaitAfterDown(u8),
    Up(u8),
    WaitBetween(u8),
    Success,
    Observed,
}

/// One bounded execution owner. Await `&mut execution` for the operation result,
/// then call `retire` even after failure/cancellation. No task is spawned and
/// no synthetic release is issued. Dropping this owner is not retirement proof.
#[must_use = "click executions retain native calls; observe the result and retire the owner"]
pub struct MouseClickExecution<D, W> {
    page: Arc<PageState>,
    client: Arc<dyn CdpClient>,
    options: MouseClickOptions,
    delay: Box<D>,
    waiting: Option<Pin<Box<W>>>,
    calls: [Option<DispatchCall>; SLOTS],
    dispatcher: Option<Arc<dyn CdpMouseDispatcher>>,
    stage: Stage,
    retirement_error: Option<CdpError>,
    _admission: Option<Arc<InputAdmission>>,
}

impl<D, W> MouseClickExecution<D, W>
where
    D: FnMut(Duration) -> W,
    W: Future<Output = Result<()>>,
{
    pub(crate) fn new(
        action: &Action,
        admission: Option<Arc<InputAdmission>>,
        options: MouseClickOptions,
        delay: D,
        dispatcher: Option<Arc<dyn CdpMouseDispatcher>>,
    ) -> Self {
        Self {
            page: action.page.clone(),
            client: action.client.clone(),
            options,
            delay: Box::new(delay),
            waiting: None,
            calls: std::array::from_fn(|_| None),
            dispatcher,
            stage: Stage::Initial,
            retirement_error: None,
            _admission: admission,
        }
    }

    /// Number of retained dispatch owners, including those not yet first-polled.
    /// A delegated owner may contain multiple provider-budgeted native calls.
    pub fn pending_native_calls(&self) -> usize {
        self.calls.iter().flatten().count()
    }

    /// Conservative payload bound for this owner, its delay boxes, outer native
    /// futures and native event type strings. Does not include allocator headers,
    /// shared Page/client state, or allocations inside injected client/delay
    /// futures and their errors; those require the provider's own admission.
    /// Delegated operation boxes/subcalls also require provider admission; this
    /// bound includes fixed slot storage, not a delegate's retained allocation tree.
    pub fn allocation_payload_bytes(client: Arc<dyn CdpClient>) -> usize {
        std::mem::size_of::<Self>()
            + std::mem::size_of::<D>()
            + std::mem::size_of::<W>()
            + SLOTS * (DirectMouseDispatch::future_payload_bytes(client) + "mouseReleased".len())
    }

    fn enqueue(&mut self, event: DispatchMouseEventParams, kind: MouseDispatchKind) {
        let slot = self
            .calls
            .iter_mut()
            .find(|slot| slot.is_none())
            .expect("validated click count bounds pending slots");
        let client = self.client.clone();
        *slot = Some(match &self.dispatcher {
            None => DispatchCall::Direct(DirectMouseDispatch::new(client, event)),
            Some(dispatcher) => DispatchCall::Delegated {
                operation: dispatcher.start(
                    client,
                    MouseDispatchRequest {
                        kind,
                        event,
                        live_state: MouseStateReader::new(
                            self.page.clone(),
                            self._admission.clone(),
                        ),
                    },
                ),
                observed: false,
            },
        });
    }

    fn poll_calls(&mut self, cx: &mut Context<'_>, retiring: bool) -> (bool, Option<CdpError>) {
        let mut pending = false;
        let mut error = None;
        // Poll every submitted call even if an earlier one fails synchronously.
        // Upstream has initiated all no-delay calls before observing Promise.all.
        for slot in &mut self.calls {
            let Some(call) = slot.as_mut() else {
                continue;
            };
            if !retiring && !call.observed() {
                match call.poll_result(cx) {
                    Poll::Pending => {
                        pending = true;
                        continue;
                    }
                    Poll::Ready(result) => {
                        let failed = result.is_err();
                        if let Err(failure) = result
                            && error.is_none()
                        {
                            error = Some(failure);
                        }
                        if call.is_retired() {
                            *slot = None;
                            continue;
                        }
                        // Preserve early result errors independently of retained
                        // provider cleanup. The caller's retire drives that work.
                        if failed {
                            pending = true;
                            continue;
                        }
                    }
                }
            }
            // Successful sub-operations retire before a later interpolation
            // step can reuse their slot. Poll now to register the cleanup waker;
            // no unbounded collection of successful-but-unretired moves builds.
            match call.poll_retire(cx) {
                Poll::Pending => pending = true,
                Poll::Ready(result) => {
                    *slot = None;
                    if let Err(failure) = result
                        && self.retirement_error.is_none()
                    {
                        self.retirement_error = Some(failure);
                    }
                }
            }
        }
        (pending, error)
    }

    fn buttons_batch(&mut self) {
        for count in 1..=self.options.count.get() {
            self.enqueue(
                down_event(&self.page, self.options.button, i32::from(count)),
                MouseDispatchKind::Down,
            );
            self.enqueue(
                up_event(&self.page, self.options.button, i32::from(count)),
                MouseDispatchKind::Up,
            );
        }
        self.stage = Stage::Batch;
    }

    /// Stop planning future actions, drop only an injected wait, and retire all
    /// retained dispatch owners (native replies plus delegated cleanup). This
    /// never plans an extra up or retries a pointer action.
    pub async fn retire(&mut self) -> Result<()> {
        self.stage = Stage::Observed;
        self.waiting = None;
        poll_fn(|cx| {
            let (pending, _) = self.poll_calls(cx, true);
            if pending {
                Poll::Pending
            } else {
                Poll::Ready(self.retirement_error.take().map_or(Ok(()), Err))
            }
        })
        .await
    }
}

impl<D, W> Future for MouseClickExecution<D, W>
where
    D: FnMut(Duration) -> W,
    W: Future<Output = Result<()>>,
{
    type Output = Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if matches!(this.stage, Stage::Observed) {
            panic!("click result already observed; use retire for retained calls");
        }
        loop {
            let (pending, error) = this.poll_calls(cx, false);
            if let Some(error) = error {
                this.stage = Stage::Observed;
                return Poll::Ready(Err(error));
            }
            if this.retirement_error.is_some() {
                // Failed cleanup must not unlock a later pointer action. Keep
                // the original cleanup error for the retirement receipt.
                this.stage = Stage::Observed;
                return Poll::Ready(Err(CdpError::Other(
                    "mouse dispatch cleanup failed; retire the execution for its receipt".into(),
                )));
            }
            if pending {
                return Poll::Pending;
            }
            match this.stage {
                Stage::Initial => {
                    let from = begin_move(&this.page, this.options.x, this.options.y);
                    if this.options.delay.is_zero() && this.options.steps.get() <= 1.0 {
                        if this.options.steps.get() == 1.0 {
                            this.enqueue(
                                move_event(&this.page, this.options.x, this.options.y),
                                MouseDispatchKind::Move {
                                    for_click: this.options.for_click,
                                },
                            );
                        }
                        this.buttons_batch();
                    } else {
                        this.stage = Stage::Moving { from, next: 1 };
                    }
                }
                Stage::Moving { from, next } => {
                    if f64::from(next) <= this.options.steps.get() {
                        let p = f64::from(next) / this.options.steps.get();
                        this.enqueue(
                            move_event(
                                &this.page,
                                from.0 + (this.options.x - from.0) * p,
                                from.1 + (this.options.y - from.1) * p,
                            ),
                            MouseDispatchKind::Move {
                                for_click: this.options.for_click,
                            },
                        );
                        this.stage = Stage::Moving {
                            from,
                            next: next + 1,
                        };
                    } else if this.options.delay.is_zero() {
                        this.buttons_batch();
                    } else {
                        this.stage = Stage::Down(1);
                    }
                }
                Stage::Down(count) => {
                    if count > this.options.count.get() {
                        this.stage = Stage::Success;
                    } else {
                        this.enqueue(
                            down_event(&this.page, this.options.button, i32::from(count)),
                            MouseDispatchKind::Down,
                        );
                        this.stage = Stage::WaitAfterDown(count);
                    }
                }
                Stage::Up(count) => {
                    this.enqueue(
                        up_event(&this.page, this.options.button, i32::from(count)),
                        MouseDispatchKind::Up,
                    );
                    this.stage = if count < this.options.count.get() {
                        Stage::WaitBetween(count + 1)
                    } else {
                        Stage::Success
                    };
                }
                Stage::WaitAfterDown(count) | Stage::WaitBetween(count) => {
                    let wait = this
                        .waiting
                        .get_or_insert_with(|| Box::pin((this.delay)(this.options.delay)));
                    match wait.as_mut().poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(error)) => {
                            this.waiting = None;
                            this.stage = Stage::Observed;
                            return Poll::Ready(Err(error));
                        }
                        Poll::Ready(Ok(())) => {
                            this.waiting = None;
                            this.stage = match this.stage {
                                Stage::WaitAfterDown(_) => Stage::Up(count),
                                Stage::WaitBetween(_) => Stage::Down(count),
                                _ => unreachable!(),
                            };
                        }
                    }
                }
                Stage::Batch | Stage::Success => {
                    this.stage = Stage::Observed;
                    return Poll::Ready(Ok(()));
                }
                Stage::Observed => unreachable!(),
            }
        }
    }
}

#[cfg(test)]
mod tests;
