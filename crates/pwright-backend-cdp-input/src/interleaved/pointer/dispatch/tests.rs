use super::*;
use crate::{CdpInputCoordinator, CdpInputPolicy, MouseClickOptions};
use pwright_cdp::{MouseButton, connection::CdpError};
use pwright_fake::FakeCdpClient;
use pwright_input_model::{MouseClickCount, MouseMoveSteps};
use std::{
    collections::VecDeque,
    future::{poll_fn, ready},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Wake, Waker},
};

/// Explicitly released fake setup/cleanup; no clocks or background tasks.
#[derive(Default)]
struct Gate(Mutex<(bool, Option<Waker>)>);
impl Gate {
    fn open() -> Arc<Self> {
        Arc::new(Self(Mutex::new((true, None))))
    }
    fn poll(&self, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self.0.lock().unwrap();
        if state.0 {
            Poll::Ready(())
        } else {
            state.1 = Some(cx.waker().clone());
            Poll::Pending
        }
    }
    fn release(&self) {
        let wake = {
            let mut state = self.0.lock().unwrap();
            state.0 = true;
            state.1.take()
        };
        if let Some(wake) = wake {
            wake.wake();
        }
    }
}
#[derive(Default)]
struct WakeCount(AtomicUsize);
impl Wake for WakeCount {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

struct Plan {
    setup: Arc<Gate>,
    cleanup: Arc<Gate>,
    result_error: Option<&'static str>,
    cleanup_error: Option<&'static str>,
}
impl Default for Plan {
    fn default() -> Self {
        Self {
            setup: Gate::open(),
            cleanup: Gate::open(),
            result_error: None,
            cleanup_error: None,
        }
    }
}
#[derive(Default)]
struct Seen {
    kinds: Mutex<Vec<MouseDispatchKind>>,
    snapshots: Mutex<Vec<(DispatchMouseEventParams, MouseLiveState)>>,
    started: AtomicUsize,
    cleaned: AtomicUsize,
    dropped: AtomicUsize,
}
struct Dispatcher {
    plans: Mutex<VecDeque<Plan>>,
    seen: Arc<Seen>,
}
impl Dispatcher {
    fn new(plans: impl IntoIterator<Item = Plan>) -> Arc<Self> {
        Arc::new(Self {
            plans: Mutex::new(plans.into_iter().collect()),
            seen: Arc::new(Seen::default()),
        })
    }
}
impl CdpMouseDispatcher for Dispatcher {
    fn start(
        &self,
        client: Arc<dyn CdpClient>,
        request: MouseDispatchRequest,
    ) -> Box<dyn MouseDispatchOperation> {
        self.seen.kinds.lock().unwrap().push(request.kind);
        Box::new(Operation {
            plan: self.plans.lock().unwrap().pop_front().unwrap_or_default(),
            seen: self.seen.clone(),
            request: Some(request),
            client,
            native: None,
            started: false,
            observed: false,
            retiring: false,
            retired: false,
        })
    }
}
struct Operation {
    plan: Plan,
    seen: Arc<Seen>,
    request: Option<MouseDispatchRequest>,
    client: Arc<dyn CdpClient>,
    native: Option<DirectMouseDispatch>,
    started: bool,
    observed: bool,
    retiring: bool,
    retired: bool,
}
impl MouseDispatchOperation for Operation {
    fn poll_result(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        assert!(!self.observed && !self.retiring && !self.retired);
        if !self.started {
            self.started = true;
            self.seen.started.fetch_add(1, Ordering::SeqCst);
        }
        if self.plan.setup.poll(cx).is_pending() {
            return Poll::Pending;
        }
        if let Some(message) = self.plan.result_error {
            self.observed = true;
            return Poll::Ready(Err(CdpError::Other(message.into())));
        }
        if self.native.is_none() {
            let request = self.request.take().unwrap();
            self.seen
                .snapshots
                .lock()
                .unwrap()
                .push((request.event.clone(), request.live_state.snapshot()));
            self.native = Some(DirectMouseDispatch::new(self.client.clone(), request.event));
        }
        let result = self.native.as_mut().unwrap().poll_result(cx);
        self.observed = result.is_ready();
        result
    }
    fn poll_retire(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.retiring = true;
        if self.retired {
            return Poll::Ready(Ok(()));
        }
        if self.started {
            // Drain setup without progressing to the pointer action.
            if self.plan.setup.poll(cx).is_pending() {
                return Poll::Pending;
            }
            if let Some(native) = &mut self.native {
                match native.poll_retire(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(result) => result.expect("this fake native reply succeeds"),
                }
            }
            if self.plan.cleanup.poll(cx).is_pending() {
                return Poll::Pending;
            }
            self.seen.cleaned.fetch_add(1, Ordering::SeqCst);
        }
        self.retired = true;
        Poll::Ready(
            self.plan
                .cleanup_error
                .map_or(Ok(()), |message| Err(CdpError::Other(message.into()))),
        )
    }
    fn is_retired(&self) -> bool {
        self.retired
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        self.seen.dropped.fetch_add(1, Ordering::SeqCst);
    }
}

fn setup() -> (Arc<FakeCdpClient>, CdpInputCoordinator) {
    (
        Arc::new(FakeCdpClient::from_html("")),
        CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved),
    )
}

#[tokio::test]
async fn retiring_unpolled_sequence_never_constructs_delegate_or_changes_position() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let dispatcher = Dispatcher::new([]);
    let mut click = action
        .raw_mouse_click_with_dispatcher(
            MouseClickOptions::new(80.0, 90.0).unwrap(),
            |_| ready(Ok(())),
            dispatcher.clone(),
        )
        .unwrap();
    // Act
    click.retire().await.unwrap();
    // Assert
    assert!(dispatcher.seen.kinds.lock().unwrap().is_empty());
    assert!(client.calls().is_empty());
    action.raw_mouse_down(MouseButton::Left, 1).await.unwrap();
    assert_eq!(client.calls()[0].1[0]["x"], 0.0);
    assert_eq!(client.calls()[0].1[0]["y"], 0.0);
}

#[tokio::test]
async fn cancelling_pending_setup_drains_then_cleans_without_late_pointer_actions() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let setup_gate = Arc::new(Gate::default());
    let cleanup_gate = Arc::new(Gate::default());
    let dispatcher = Dispatcher::new([Plan {
        setup: setup_gate.clone(),
        cleanup: cleanup_gate.clone(),
        ..Plan::default()
    }]);
    let mut click = action
        .raw_mouse_click_with_dispatcher(
            MouseClickOptions::new(80.0, 90.0)
                .unwrap()
                .with_steps(MouseMoveSteps::try_from(2.0).unwrap()),
            |_| ready(Ok(())),
            dispatcher.clone(),
        )
        .unwrap();
    assert!(
        Pin::new(&mut click)
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    let mut retirement = Box::pin(click.retire());
    assert!(
        retirement
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    setup_gate.release();
    assert!(
        retirement
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert_eq!(dispatcher.seen.dropped.load(Ordering::SeqCst), 0);
    // Abandon only the borrowed retirement future, not the execution owner.
    drop(retirement);
    cleanup_gate.release();
    click.retire().await.unwrap();
    // Assert
    assert!(client.calls().is_empty());
    assert_eq!(dispatcher.seen.kinds.lock().unwrap().len(), 1);
    assert_eq!(dispatcher.seen.cleaned.load(Ordering::SeqCst), 1);
    assert_eq!(dispatcher.seen.dropped.load(Ordering::SeqCst), 1);
    assert_eq!(click.pending_native_calls(), 0);
}

#[tokio::test]
async fn early_result_error_keeps_cleanup_receipt_and_first_polls_whole_batch() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let gate = Arc::new(Gate::default());
    let dispatcher = Dispatcher::new([Plan {
        cleanup: gate.clone(),
        result_error: Some("move failed"),
        cleanup_error: Some("cleanup failed"),
        ..Plan::default()
    }]);
    let mut click = action
        .raw_mouse_click_with_dispatcher(
            MouseClickOptions::new(80.0, 90.0).unwrap(),
            |_| ready(Ok(())),
            dispatcher.clone(),
        )
        .unwrap();
    // Act
    let result = (&mut click).await.unwrap_err();
    // Assert
    assert!(result.to_string().contains("move failed"));
    assert_eq!(dispatcher.seen.started.load(Ordering::SeqCst), 3);
    assert_eq!(dispatcher.seen.dropped.load(Ordering::SeqCst), 2);
    assert_eq!(client.calls().len(), 2);
    assert_eq!(click.pending_native_calls(), 1);
    let mut retirement = Box::pin(click.retire());
    assert!(
        retirement
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    gate.release();
    let cleanup = retirement.await.unwrap_err();
    assert!(cleanup.to_string().contains("cleanup failed"));
    assert_eq!(dispatcher.seen.dropped.load(Ordering::SeqCst), 3);
    assert_eq!(click.pending_native_calls(), 0);
    click.retire().await.unwrap();
}

#[tokio::test]
async fn successful_child_cleanup_registers_waker_and_gates_interpolation_slot_reuse() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let gate = Arc::new(Gate::default());
    let dispatcher = Dispatcher::new([Plan {
        cleanup: gate.clone(),
        ..Plan::default()
    }]);
    let wakes = Arc::new(WakeCount::default());
    let waker = Waker::from(wakes.clone());
    let mut click = action
        .raw_mouse_click_with_dispatcher(
            MouseClickOptions::new(80.0, 90.0)
                .unwrap()
                .with_steps(MouseMoveSteps::try_from(70.0).unwrap())
                .with_count(MouseClickCount::try_from(0.0).unwrap()),
            |_| ready(Ok(())),
            dispatcher.clone(),
        )
        .unwrap();
    // Act
    let result = Pin::new(&mut click).poll(&mut Context::from_waker(&waker));
    // Assert
    assert!(result.is_pending());
    assert_eq!(client.calls().len(), 1);
    assert_eq!(dispatcher.seen.kinds.lock().unwrap().len(), 1);
    assert_eq!(click.pending_native_calls(), 1);
    gate.release();
    assert_eq!(wakes.0.load(Ordering::SeqCst), 1);
    (&mut click).await.unwrap();
    click.retire().await.unwrap();
    assert_eq!(client.calls().len(), 70);
    assert_eq!(dispatcher.seen.cleaned.load(Ordering::SeqCst), 70);
    assert_eq!(click.pending_native_calls(), 0);
}

#[tokio::test]
async fn cleanup_failure_stops_later_interpolation_and_preserves_original_receipt() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let dispatcher = Dispatcher::new([Plan {
        cleanup_error: Some("disable failed"),
        ..Plan::default()
    }]);
    let mut click = action
        .raw_mouse_click_with_dispatcher(
            MouseClickOptions::new(80.0, 90.0)
                .unwrap()
                .with_steps(MouseMoveSteps::try_from(2.0).unwrap()),
            |_| ready(Ok(())),
            dispatcher.clone(),
        )
        .unwrap();
    // Act
    let result = (&mut click).await.unwrap_err();
    // Assert
    assert!(result.to_string().contains("mouse dispatch cleanup failed"));
    assert!(
        click
            .retire()
            .await
            .unwrap_err()
            .to_string()
            .contains("disable failed")
    );
    assert_eq!(client.calls().len(), 1);
    assert_eq!(dispatcher.seen.cleaned.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn deferred_reader_observes_current_buttons_and_modifiers_with_captured_position_and_button()
{
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let mut other = input.begin_transaction(client.clone(), None).await.unwrap();
    action.raw_mouse_down(MouseButton::Left, 1).await.unwrap();
    let gate = Arc::new(Gate::default());
    let dispatcher = Dispatcher::new([Plan {
        setup: gate.clone(),
        ..Plan::default()
    }]);
    let mut movement = action
        .raw_mouse_click_with_dispatcher(
            MouseClickOptions::new(80.0, 90.0)
                .unwrap()
                .with_count(MouseClickCount::try_from(0.0).unwrap())
                .with_for_click(false),
            |_| ready(Ok(())),
            dispatcher.clone(),
        )
        .unwrap();
    assert!(
        Pin::new(&mut movement)
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    other.raw_mouse_up(MouseButton::Left, 1).await.unwrap();
    other.key_down_named("Shift", false).await.unwrap();
    gate.release();
    (&mut movement).await.unwrap();
    movement.retire().await.unwrap();
    // Assert
    let samples = dispatcher.seen.snapshots.lock().unwrap();
    let (event, live) = &samples[0];
    assert_eq!((event.x, event.y), (80.0, 90.0));
    assert_eq!(
        event.button,
        Some(pwright_cdp::generated::input::MouseButton::Left)
    );
    assert_eq!(event.buttons, Some(1));
    assert_eq!(event.modifiers, Some(0));
    assert_eq!(
        *live,
        MouseLiveState {
            buttons: 0,
            modifiers: 8
        }
    );
    assert_eq!(
        *dispatcher.seen.kinds.lock().unwrap(),
        [MouseDispatchKind::Move { for_click: false }]
    );
}

#[tokio::test]
async fn zero_count_click_keeps_click_bypass_and_numbered_batch_has_down_up_kinds() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let dispatcher = Dispatcher::new([]);
    let mut zero = action
        .raw_mouse_click_with_dispatcher(
            MouseClickOptions::new(80.0, 90.0)
                .unwrap()
                .with_count(MouseClickCount::try_from(0.0).unwrap()),
            |_| ready(Ok(())),
            dispatcher.clone(),
        )
        .unwrap();
    let mut two = action
        .raw_mouse_click_with_dispatcher(
            MouseClickOptions::new(80.0, 90.0)
                .unwrap()
                .with_count(MouseClickCount::TWO),
            |_| ready(Ok(())),
            dispatcher.clone(),
        )
        .unwrap();
    // Act
    (&mut zero).await.unwrap();
    zero.retire().await.unwrap();
    (&mut two).await.unwrap();
    two.retire().await.unwrap();
    // Assert
    assert_eq!(
        *dispatcher.seen.kinds.lock().unwrap(),
        [
            MouseDispatchKind::Move { for_click: true },
            MouseDispatchKind::Move { for_click: true },
            MouseDispatchKind::Down,
            MouseDispatchKind::Up,
            MouseDispatchKind::Down,
            MouseDispatchKind::Up,
        ]
    );
    assert_eq!(client.calls().len(), 6);
}

#[tokio::test]
async fn direct_retirement_of_unstarted_rpc_has_no_native_effect() {
    // Arrange
    let (client, _) = setup();
    let mut dispatch =
        DirectMouseDispatch::new(client.clone(), DispatchMouseEventParams::default());
    // Act
    poll_fn(|cx| dispatch.poll_retire(cx)).await.unwrap();
    // Assert
    assert!(dispatch.is_retired());
    assert!(client.calls().is_empty());
}

#[tokio::test]
async fn direct_retirement_drains_started_rpc_without_resubmitting() {
    // Arrange
    let (client, _) = setup();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let mut dispatch =
        DirectMouseDispatch::new(client.clone(), DispatchMouseEventParams::default());
    assert!(
        dispatch
            .poll_result(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    let pending = dispatch.poll_retire(&mut Context::from_waker(Waker::noop()));
    // Assert
    assert!(pending.is_pending());
    assert!(!dispatch.is_retired());
    assert_eq!(client.calls().len(), 1);
    gate.notify_one();
    poll_fn(|cx| dispatch.poll_retire(cx)).await.unwrap();
    assert!(dispatch.is_retired());
    assert_eq!(client.calls().len(), 1);
}

#[tokio::test]
async fn unpolled_standalone_down_and_up_neither_construct_provider_nor_mutate_state() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let dispatcher = Dispatcher::new([]);
    let mut down = action
        .raw_mouse_down_with_dispatcher(MouseButton::Left, 1, dispatcher.clone())
        .unwrap();
    let mut up = action
        .raw_mouse_up_with_dispatcher(MouseButton::Left, 1, dispatcher.clone())
        .unwrap();
    // Act
    down.retire().await.unwrap();
    action.raw_mouse_down(MouseButton::Left, 1).await.unwrap();
    up.retire().await.unwrap();
    action
        .raw_mouse_move(20.0, 30.0, MouseMoveSteps::ONE)
        .await
        .unwrap();
    // Assert
    assert!(dispatcher.seen.kinds.lock().unwrap().is_empty());
    assert_eq!(client.calls().len(), 2);
    assert_eq!(client.calls()[1].1[0]["buttons"], 1);
}

#[tokio::test]
async fn standalone_button_result_keeps_pending_cleanup_until_resumable_retirement() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let gate = Arc::new(Gate::default());
    let dispatcher = Dispatcher::new([Plan {
        cleanup: gate.clone(),
        ..Plan::default()
    }]);
    let mut down = action
        .raw_mouse_down_with_dispatcher(MouseButton::Right, 3, dispatcher.clone())
        .unwrap();
    // Act
    (&mut down).await.unwrap();
    // Assert
    assert_eq!(down.pending_native_calls(), 1);
    assert_eq!(dispatcher.seen.dropped.load(Ordering::SeqCst), 0);
    let mut retire = Box::pin(down.retire());
    assert!(
        retire
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(retire);
    gate.release();
    down.retire().await.unwrap();
    assert_eq!(down.pending_native_calls(), 0);
    assert_eq!(dispatcher.seen.dropped.load(Ordering::SeqCst), 1);
    let calls = client.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].1[0]["buttons"], 2);
    assert_eq!(calls[0].1[0]["clickCount"], 3);
}

#[tokio::test]
async fn cancelling_standalone_up_during_setup_never_sends_late_release() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    action.raw_mouse_down(MouseButton::Left, 1).await.unwrap();
    let gate = Arc::new(Gate::default());
    let dispatcher = Dispatcher::new([Plan {
        setup: gate.clone(),
        ..Plan::default()
    }]);
    let mut up = action
        .raw_mouse_up_with_dispatcher(MouseButton::Left, 1, dispatcher.clone())
        .unwrap();
    assert!(
        Pin::new(&mut up)
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    let mut retire = Box::pin(up.retire());
    assert!(
        retire
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    gate.release();
    retire.await.unwrap();
    // Assert
    assert_eq!(client.calls().len(), 1);
    assert_eq!(
        *dispatcher.seen.kinds.lock().unwrap(),
        [MouseDispatchKind::Up]
    );
    assert_eq!(dispatcher.seen.cleaned.load(Ordering::SeqCst), 1);
    assert_eq!(up.pending_native_calls(), 0);
    // Logical up was already applied at first poll, despite cancelled native work.
    action
        .raw_mouse_move(20.0, 30.0, MouseMoveSteps::ONE)
        .await
        .unwrap();
    assert_eq!(client.calls()[1].1[0]["buttons"], 0);
}

#[tokio::test]
async fn standalone_button_preserves_result_and_cleanup_errors_separately() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let dispatcher = Dispatcher::new([Plan {
        result_error: Some("press rejected"),
        cleanup_error: Some("cleanup rejected"),
        ..Plan::default()
    }]);
    let mut down = action
        .raw_mouse_down_with_dispatcher(MouseButton::Left, 1, dispatcher.clone())
        .unwrap();
    // Act
    let error = (&mut down).await.unwrap_err();
    // Assert
    assert!(error.to_string().contains("press rejected"));
    assert_eq!(down.pending_native_calls(), 1);
    assert!(
        down.retire()
            .await
            .unwrap_err()
            .to_string()
            .contains("cleanup rejected")
    );
    assert_eq!(down.pending_native_calls(), 0);
    assert!(client.calls().is_empty());
    down.retire().await.unwrap();
}
