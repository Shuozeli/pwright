use super::*;
use crate::{CdpInputCoordinator, CdpInputPolicy};
use pwright_fake::FakeCdpClient;
use std::{
    future::{pending, ready},
    sync::Mutex,
    task::Waker,
};

fn setup() -> (Arc<FakeCdpClient>, CdpInputCoordinator) {
    (
        Arc::new(FakeCdpClient::from_html("")),
        CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved),
    )
}

#[tokio::test]
async fn retiring_unpolled_click_has_no_pointer_state_or_native_effects() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let mut click = action
        .raw_mouse_click(MouseClickOptions::new(80.0, 90.0).unwrap(), |_| {
            ready(Ok(()))
        })
        .unwrap();
    // Act
    click.retire().await.unwrap();
    // Assert
    assert!(client.calls().is_empty());
    action.raw_mouse_down(MouseButton::Left, 1).await.unwrap();
    assert_eq!(client.calls()[0].1[0]["x"], 0.0);
    assert_eq!(client.calls()[0].1[0]["y"], 0.0);
}

#[tokio::test]
async fn delay_failure_preserves_pressed_prefix_without_later_pairs() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let options = MouseClickOptions::new(80.0, 90.0)
        .unwrap()
        .with_count(MouseClickCount::TWO)
        .with_delay(Duration::from_millis(1));
    let mut click = action
        .raw_mouse_click(options, |_| {
            ready(Err(CdpError::Other("delay failed".into())))
        })
        .unwrap();
    // Act
    let result = (&mut click).await.unwrap_err();
    click.retire().await.unwrap();
    // Assert
    assert!(result.to_string().contains("delay failed"));
    assert_eq!(client.calls().len(), 2);
    action
        .raw_mouse_move(100.0, 110.0, MouseMoveSteps::ONE)
        .await
        .unwrap();
    assert_eq!(client.calls()[2].1[0]["buttons"], 1);
}

#[tokio::test]
async fn double_click_submits_move_and_two_numbered_pairs_without_delay() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let options = MouseClickOptions::new(20.0, 30.0)
        .unwrap()
        .with_count(MouseClickCount::TWO);
    let mut click = action
        .raw_mouse_click(options, |_| -> std::future::Ready<Result<()>> {
            panic!("zero-delay click requested a timer")
        })
        .unwrap();
    // Act
    (&mut click).await.unwrap();
    click.retire().await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 5);
    assert_eq!(calls[0].1[0]["type"], "mouseMoved");
    for (index, count, buttons) in [(1, 1, 1), (2, 1, 0), (3, 2, 1), (4, 2, 0)] {
        assert_eq!(calls[index].1[0]["clickCount"], count);
        assert_eq!(calls[index].1[0]["buttons"], buttons);
    }
    assert_eq!(click.pending_native_calls(), 0);
}

#[tokio::test]
async fn delayed_triple_click_waits_after_down_and_between_pairs_only() {
    // Arrange
    let (client, input) = setup();
    let waits = Arc::new(Mutex::new(Vec::new()));
    let seen = waits.clone();
    let recorder = client.clone();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let options = MouseClickOptions::new(20.0, 30.0)
        .unwrap()
        .with_count(MouseClickCount::try_from(3.0).unwrap())
        .with_delay(Duration::from_micros(500));
    let mut click = action
        .raw_mouse_click(options, move |duration| {
            seen.lock()
                .unwrap()
                .push((duration, recorder.calls().len()));
            ready(Ok(()))
        })
        .unwrap();
    // Act
    (&mut click).await.unwrap();
    click.retire().await.unwrap();
    // Assert
    assert_eq!(client.calls().len(), 7);
    assert_eq!(
        *waits.lock().unwrap(),
        [2, 3, 4, 5, 6].map(|count| (Duration::from_micros(500), count))
    );
}

#[tokio::test]
async fn no_delay_submits_the_whole_batch_before_move_reply() {
    // Arrange
    let (client, input) = setup();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let mut click = action
        .raw_mouse_click(MouseClickOptions::new(10.0, 20.0).unwrap(), |_| {
            ready(Ok(()))
        })
        .unwrap();
    // Act
    let first = Pin::new(&mut click).poll(&mut Context::from_waker(Waker::noop()));
    // Assert
    assert!(first.is_pending());
    assert_eq!(client.calls().len(), 3);
    assert_eq!(click.pending_native_calls(), 1);
    gate.notify_one();
    (&mut click).await.unwrap();
    click.retire().await.unwrap();
}

#[tokio::test]
async fn result_error_does_not_discard_a_later_pending_native_reply() {
    // Arrange
    let (client, input) = setup();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_input_errors([Some("move rejected".into()), None, None]);
    client.set_pointer_gates([None, Some(gate.clone()), None]);
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let mut click = action
        .raw_mouse_click(MouseClickOptions::new(10.0, 20.0).unwrap(), |_| {
            ready(Ok(()))
        })
        .unwrap();
    // Act
    let result = (&mut click).await.unwrap_err();
    // Assert
    assert!(result.to_string().contains("move rejected"));
    assert_eq!(client.calls().len(), 3);
    assert_eq!(click.pending_native_calls(), 1);
    let mut retirement = Box::pin(click.retire());
    assert!(
        retirement
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    gate.notify_one();
    retirement.await.unwrap();
    assert_eq!(click.pending_native_calls(), 0);
    assert_eq!(client.calls().len(), 3);
}

#[tokio::test]
async fn retirement_during_interpolation_drains_current_call_without_next_move_or_click() {
    // Arrange
    let (client, input) = setup();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let options = MouseClickOptions::new(100.0, 80.0)
        .unwrap()
        .with_steps(MouseMoveSteps::try_from(4.0).unwrap());
    let mut click = action.raw_mouse_click(options, |_| ready(Ok(()))).unwrap();
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
    gate.notify_one();
    retirement.await.unwrap();
    // Assert
    assert_eq!(client.calls().len(), 1);
    assert_eq!(click.pending_native_calls(), 0);
    action.raw_mouse_down(MouseButton::Left, 1).await.unwrap();
    assert_eq!(client.calls()[1].1[0]["x"], 100.0);
}

#[tokio::test]
async fn retiring_a_delayed_press_does_not_synthesize_release() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let options = MouseClickOptions::new(10.0, 20.0)
        .unwrap()
        .with_delay(Duration::from_secs(1));
    let mut click = action
        .raw_mouse_click(options, |_| pending::<Result<()>>())
        .unwrap();
    assert!(
        Pin::new(&mut click)
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    click.retire().await.unwrap();
    action
        .raw_mouse_move(20.0, 30.0, MouseMoveSteps::ONE)
        .await
        .unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[2].1[0]["buttons"], 1);
    assert_eq!(calls[2].1[0]["button"], "left");
}

#[tokio::test]
async fn multi_step_click_awaits_movement_before_preparing_button_batch() {
    // Arrange
    let (client, input) = setup();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let options = MouseClickOptions::new(90.0, 60.0)
        .unwrap()
        .with_steps(MouseMoveSteps::try_from(3.0).unwrap());
    let mut click = action.raw_mouse_click(options, |_| ready(Ok(()))).unwrap();
    // Act
    let first = Pin::new(&mut click).poll(&mut Context::from_waker(Waker::noop()));
    // Assert
    assert!(first.is_pending());
    assert_eq!(client.calls().len(), 1);
    gate.notify_one();
    (&mut click).await.unwrap();
    click.retire().await.unwrap();
    let calls = client.calls();
    assert_eq!(calls.len(), 5);
    assert_eq!(calls[2].1[0]["type"], "mouseMoved");
    assert_eq!(calls[3].1[0]["type"], "mousePressed");
}

#[tokio::test]
async fn maximum_click_batch_stays_within_fixed_slot_bound() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let options = MouseClickOptions::new(10.0, 20.0)
        .unwrap()
        .with_count(MouseClickCount::try_from(f64::from(INPUT_MOUSE_MAX_CLICKS)).unwrap());
    let mut click = action.raw_mouse_click(options, |_| ready(Ok(()))).unwrap();
    // Act
    (&mut click).await.unwrap();
    click.retire().await.unwrap();
    // Assert
    assert_eq!(client.calls().len(), SLOTS);
    assert_eq!(click.pending_native_calls(), 0);
}

#[tokio::test]
async fn zero_count_with_delay_does_not_wait_or_press() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let options = MouseClickOptions::new(10.0, 20.0)
        .unwrap()
        .with_count(MouseClickCount::try_from(-1.0).unwrap())
        .with_delay(Duration::from_secs(1));
    let mut click = action
        .raw_mouse_click(options, |_| pending::<Result<()>>())
        .unwrap();
    // Act
    let result = Pin::new(&mut click).poll(&mut Context::from_waker(Waker::noop()));
    // Assert
    assert!(matches!(result, Poll::Ready(Ok(()))));
    assert_eq!(client.calls().len(), 1);
    click.retire().await.unwrap();
}

#[tokio::test]
async fn raw_mouse_call_captures_state_without_waiting_for_previous_reply() {
    // Arrange
    let (client, input) = setup();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let mut first = input.begin_transaction(client.clone(), None).await.unwrap();
    let mut second = input.begin_transaction(client.clone(), None).await.unwrap();
    let mut down = Box::pin(first.raw_mouse_down(MouseButton::Left, 1));
    assert!(
        down.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    second
        .raw_mouse_move(20.0, 30.0, MouseMoveSteps::ONE)
        .await
        .unwrap();
    // Assert
    assert_eq!(client.calls().len(), 2);
    assert_eq!(client.calls()[1].1[0]["buttons"], 1);
    gate.notify_one();
    down.await.unwrap();
}
