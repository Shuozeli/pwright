use super::*;
use std::sync::Mutex;

#[derive(Default)]
struct WakeCount(AtomicUsize);
impl std::task::Wake for WakeCount {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Default)]
struct ReceiptState {
    ready: bool,
    waiter: Option<Waker>,
    polls: usize,
}
struct Receipt {
    state: Arc<Mutex<ReceiptState>>,
    client: Arc<dyn CdpClient>,
    world: &'static str,
}
impl NativeEvaluationRetirement for Receipt {
    fn poll_retired(
        &mut self,
        client: &Arc<dyn CdpClient>,
        world: &str,
        cx: &mut Context<'_>,
    ) -> Poll<()> {
        let mut state = self.state.lock().unwrap();
        state.polls += 1;
        if !Arc::ptr_eq(client, &self.client) || world != self.world {
            return Poll::Pending;
        }
        if state.ready {
            Poll::Ready(())
        } else {
            state.waiter = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
fn receipt(f: &Fixture) -> (Box<Receipt>, Arc<Mutex<ReceiptState>>) {
    let state = Arc::new(Mutex::new(ReceiptState::default()));
    (
        Box::new(Receipt {
            state: state.clone(),
            client: f.client.clone(),
            world: "main-unique",
        }),
        state,
    )
}
fn acknowledge(state: &Mutex<ReceiptState>) {
    let waiter = {
        let mut state = state.lock().unwrap();
        state.ready = true;
        state.waiter.take()
    };
    if let Some(waiter) = waiter {
        waiter.wake();
    }
}

#[tokio::test]
async fn acknowledged_retirement_before_first_poll_never_dispatches_user_code() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    let (proof, state) = receipt(&f);
    acknowledge(&state);
    // Act
    let result = prepared.evaluate_with_retirement(&params(), proof).await;
    // Assert
    assert!(matches!(result, Err(ElementEvaluationError::NativeRetired)));
    assert_eq!(f.client.calls().len(), 4);
    assert_eq!(
        f.client.method_names().last().unwrap(),
        "Runtime.releaseObjectGroup"
    );
    assert!(f.service.is_cleanup_healthy());
    f.idle();
}

#[tokio::test]
async fn retirement_wakes_cancelled_cleanup_without_releasing_the_user_promise_gate() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    f.value();
    f.client.set_evaluate_gate(Some(Arc::new(Notify::new())));
    let (proof, state) = receipt(&f);
    let mut evaluation = Box::pin(prepared.evaluate_with_retirement(&params(), proof));
    assert!(poll(evaluation.as_mut()).is_pending());
    let leases = f.resources.metrics().unwrap().active_leases;
    drop(evaluation);
    let mut cleanup = Box::pin(f.queue.try_take().unwrap().unwrap());
    let wakes = Arc::new(WakeCount::default());
    let waker = Waker::from(wakes.clone());
    assert!(
        cleanup
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    assert_eq!(f.resources.metrics().unwrap().active_leases, leases);
    // Act
    acknowledge(&state);
    assert_eq!(
        wakes.0.load(Ordering::SeqCst),
        1,
        "retirement must wake the actual cleanup task"
    );
    cleanup.await.unwrap();
    // Assert
    assert!(state.lock().unwrap().polls >= 3);
    assert_eq!(f.client.calls().len(), 5);
    assert!(f.service.is_cleanup_healthy());
    f.idle();
}

#[tokio::test]
async fn native_retirement_does_not_discharge_an_unproven_group_release_failure() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    let (proof, state) = receipt(&f);
    acknowledge(&state);
    f.client
        .set_release_object_error(Some("group still unproven".into()));
    // Act
    let result = prepared.evaluate_with_retirement(&params(), proof).await;
    // Assert
    assert!(
        matches!(result, Err(ElementEvaluationError::Cleanup { primary: Some(primary), .. }) if matches!(*primary, ElementEvaluationError::NativeRetired))
    );
    assert!(!f.service.is_cleanup_healthy());
    f.idle();
}

#[tokio::test]
async fn unacknowledged_receipt_preserves_the_normal_value_result() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    f.value();
    let (proof, state) = receipt(&f);
    // Act
    let result = prepared
        .evaluate_with_retirement(&params(), proof)
        .await
        .unwrap();
    // Assert
    assert_eq!(result, EvaluationValue::String("done".into()));
    assert!(!state.lock().unwrap().ready);
    assert_eq!(
        Arc::strong_count(&state),
        1,
        "receipt released after cleanup"
    );
    f.idle();
}

#[tokio::test]
async fn retirement_receipt_observes_unique_world_identity() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    f.value();
    let (mut proof, state) = receipt(&f);
    proof.world = "an-old-world";
    acknowledge(&state);
    // Act
    let result = prepared
        .evaluate_with_retirement(&params(), proof)
        .await
        .unwrap();
    // Assert
    assert_eq!(result, EvaluationValue::String("done".into()));
    assert_eq!(f.client.calls().len(), 5);
    f.idle();
}

#[tokio::test]
async fn retirement_receipt_cannot_redirect_to_another_client_with_the_same_world_id() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    f.value();
    let (mut proof, state) = receipt(&f);
    proof.client = Arc::new(FakeCdpClient::from_html(""));
    acknowledge(&state);
    // Act
    let result = prepared
        .evaluate_with_retirement(&params(), proof)
        .await
        .unwrap();
    // Assert
    assert_eq!(result, EvaluationValue::String("done".into()));
    assert_eq!(f.client.calls().len(), 5);
    f.idle();
}

#[tokio::test]
async fn optional_receipt_future_fits_the_existing_inline_owner_allowance() {
    // Arrange
    let f = Fixture::new(4);
    let mut prepared = f.prepared().await;
    f.value();
    let (proof, _) = receipt(&f);
    let data = prepared.data.take().unwrap();
    let params = data.params(&params());
    // Act
    let future = data.evaluate(params, Some(proof));
    // Assert: this is the inline composition future, not the caller's dynamic
    // receipt allocation or browser/V8 heap; those are independently budgeted.
    assert!(std::mem::size_of_val(&future) < 8192);
    future.await.unwrap();
    drop(prepared);
    f.idle();
}

#[tokio::test]
async fn unpolled_evaluation_drop_releases_without_polling_the_retirement_receipt() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    let (proof, state) = receipt(&f);
    let evaluation = prepared.evaluate_with_retirement(&params(), proof);
    // Act
    drop(evaluation);
    f.drain().await;
    // Assert
    assert_eq!(state.lock().unwrap().polls, 0);
    assert_eq!(Arc::strong_count(&state), 1);
    assert_eq!(f.client.calls().len(), 4);
    f.idle();
}

#[tokio::test]
async fn retirement_during_pending_group_release_continues_that_release_only_once() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    f.value();
    let (proof, state) = receipt(&f);
    let gate = Arc::new(Notify::new());
    f.client.set_release_object_group_gate(Some(gate.clone()));
    let mut evaluation = Box::pin(prepared.evaluate_with_retirement(&params(), proof));
    assert!(poll(evaluation.as_mut()).is_pending());
    let before = f.resources.metrics().unwrap().active_leases;
    // Act
    acknowledge(&state);
    drop(evaluation);
    let mut cleanup = Box::pin(f.queue.try_take().unwrap().unwrap());
    assert!(poll(cleanup.as_mut()).is_pending());
    // Assert
    assert_eq!(f.resources.metrics().unwrap().active_leases, before);
    assert_eq!(
        f.client
            .method_names()
            .iter()
            .filter(|method| *method == "Runtime.releaseObjectGroup")
            .count(),
        1
    );
    gate.notify_waiters();
    cleanup.await.unwrap();
    f.idle();
}
