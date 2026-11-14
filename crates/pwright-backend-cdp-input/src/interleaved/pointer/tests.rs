use crate::{CdpInputCoordinator, CdpInputPolicy};
use pwright_cdp::MouseButton;
use pwright_fake::FakeCdpClient;
use pwright_input_model::MouseMoveSteps;
use std::sync::Arc;

fn setup() -> (Arc<FakeCdpClient>, CdpInputCoordinator) {
    (
        Arc::new(FakeCdpClient::from_html("")),
        CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved),
    )
}

#[tokio::test]
async fn button_position_and_pressure_survive_separate_calls() {
    // Arrange
    let (client, input) = setup();
    let mut first = input.begin_transaction(client.clone(), None).await.unwrap();
    first
        .raw_mouse_move(12.5, 19.0, MouseMoveSteps::ONE)
        .await
        .unwrap();
    first.raw_mouse_down(MouseButton::Right, 2).await.unwrap();
    first.finish().await.unwrap();
    // Act
    let mut second = input.begin_transaction(client.clone(), None).await.unwrap();
    second
        .raw_mouse_move(20.0, 30.0, MouseMoveSteps::ONE)
        .await
        .unwrap();
    second.raw_mouse_up(MouseButton::Right, 3).await.unwrap();
    second.finish().await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 4);
    assert_eq!(calls[1].1[0]["x"], 12.5);
    assert_eq!(calls[2].1[0]["buttons"], 2);
    assert_eq!(calls[2].1[0]["button"], "right");
    assert_eq!(calls[2].1[0]["force"], 0.5);
    assert_eq!(calls[3].1[0]["clickCount"], 3);
    assert_eq!(calls[3].1[0]["buttons"], 0);
    assert!(calls[3].1[0].get("force").is_none());
}

#[tokio::test]
async fn multiple_buttons_and_last_button_are_not_a_single_owned_press() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    action.raw_mouse_down(MouseButton::Left, 1).await.unwrap();
    action.raw_mouse_down(MouseButton::Right, 1).await.unwrap();
    // Act
    action.raw_mouse_up(MouseButton::Right, 1).await.unwrap();
    action
        .raw_mouse_move(20.0, 30.0, MouseMoveSteps::ONE)
        .await
        .unwrap();
    action.finish().await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 4);
    assert_eq!(calls[1].1[0]["buttons"], 3);
    assert_eq!(calls[2].1[0]["buttons"], 1);
    assert_eq!(calls[3].1[0]["buttons"], 1);
    assert_eq!(calls[3].1[0]["button"], "none");
    assert_eq!(calls[3].1[0]["force"], 0.5);
}

#[tokio::test]
async fn up_dispatches_even_if_that_button_was_not_pressed() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    // Act
    action.raw_mouse_up(MouseButton::Middle, 7).await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].1[0]["button"], "middle");
    assert_eq!(calls[0].1[0]["clickCount"], 7);
    assert_eq!(calls[0].1[0]["buttons"], 0);
}

#[tokio::test]
async fn zero_steps_update_logical_destination_without_native_move() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    // Act
    action
        .raw_mouse_move(40.0, 60.0, MouseMoveSteps::try_from(0.0).unwrap())
        .await
        .unwrap();
    action.raw_mouse_down(MouseButton::Left, 1).await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].1[0]["x"], 40.0);
    assert_eq!(calls[0].1[0]["y"], 60.0);
}

#[tokio::test]
async fn integer_steps_interpolate_and_retain_final_logical_destination() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    // Act
    action
        .raw_mouse_move(100.0, 50.0, MouseMoveSteps::try_from(2.0).unwrap())
        .await
        .unwrap();
    action.raw_mouse_down(MouseButton::Left, 1).await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].1[0]["x"], 50.0);
    assert_eq!(calls[1].1[0]["x"], 100.0);
    assert_eq!(calls[2].1[0]["x"], 100.0);
}

#[tokio::test]
async fn failed_move_keeps_destination_for_next_action() {
    // Arrange
    let (client, input) = setup();
    client.set_input_errors([Some("move failed".into()), None]);
    let mut first = input.begin_transaction(client.clone(), None).await.unwrap();
    // Act
    let error = first
        .raw_mouse_move(100.0, 50.0, MouseMoveSteps::try_from(4.0).unwrap())
        .await
        .unwrap_err();
    drop(first);
    let mut second = input.begin_transaction(client.clone(), None).await.unwrap();
    second.raw_mouse_down(MouseButton::Left, 1).await.unwrap();
    // Assert
    assert!(error.to_string().contains("move failed"));
    let calls = client.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1[0]["x"], 25.0);
    assert_eq!(calls[1].1[0]["x"], 100.0);
}

#[tokio::test]
async fn failed_down_and_dropped_action_do_not_implicitly_release_page_button() {
    // Arrange
    let (client, input) = setup();
    client.set_input_errors([Some("down failed".into()), None]);
    let mut first = input.begin_transaction(client.clone(), None).await.unwrap();
    // Act
    let error = first
        .raw_mouse_down(MouseButton::Left, 1)
        .await
        .unwrap_err();
    drop(first);
    input.cleanup_pending(client.as_ref()).await.unwrap();
    let mut second = input.begin_transaction(client.clone(), None).await.unwrap();
    second
        .raw_mouse_move(10.0, 20.0, MouseMoveSteps::ONE)
        .await
        .unwrap();
    // Assert
    assert!(error.to_string().contains("down failed"));
    let calls = client.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].1[0]["buttons"], 1);
}

#[tokio::test]
async fn owned_locator_press_is_released_after_its_transaction_is_dropped() {
    // Arrange
    let (client, input) = setup();
    let owner = input.new_owned_mouse_press().unwrap();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    action.set_owned_mouse_press(&owner).unwrap();
    action.mouse_down(MouseButton::Right, 2).await.unwrap();
    drop(action);

    // Act
    input
        .release_owned_mouse_press(client.as_ref(), &owner)
        .await
        .unwrap();

    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1[0]["type"], "mousePressed");
    assert_eq!(calls[1].1[0]["type"], "mouseReleased");
    assert_eq!(calls[1].1[0]["button"], "right");
    assert_eq!(calls[1].1[0]["clickCount"], 2);
    assert_eq!(calls[1].1[0]["buttons"], 0);
}

#[tokio::test]
async fn owned_locator_press_cleanup_does_nothing_after_normal_up() {
    // Arrange
    let (client, input) = setup();
    let owner = input.new_owned_mouse_press().unwrap();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    action.set_owned_mouse_press(&owner).unwrap();
    action.mouse_down(MouseButton::Left, 1).await.unwrap();
    action.mouse_up().await.unwrap();
    drop(action);

    // Act
    input
        .release_owned_mouse_press(client.as_ref(), &owner)
        .await
        .unwrap();

    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].1[0]["type"], "mouseReleased");
}

#[tokio::test]
async fn old_owned_locator_press_cannot_release_a_newer_same_button_press() {
    // Arrange
    let (client, input) = setup();
    let old_owner = input.new_owned_mouse_press().unwrap();
    let mut old = input.begin_transaction(client.clone(), None).await.unwrap();
    old.set_owned_mouse_press(&old_owner).unwrap();
    old.mouse_down(MouseButton::Left, 1).await.unwrap();
    drop(old);
    let mut newer = input.begin_transaction(client.clone(), None).await.unwrap();
    newer.mouse_down(MouseButton::Left, 3).await.unwrap();
    drop(newer);

    // Act
    input
        .release_owned_mouse_press(client.as_ref(), &old_owner)
        .await
        .unwrap();

    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].1[0]["type"], "mousePressed");
    assert_eq!(calls[1].1[0]["buttons"], 1);
}

#[tokio::test]
async fn owned_locator_press_cleanup_preserves_another_button() {
    // Arrange
    let (client, input) = setup();
    let owner = input.new_owned_mouse_press().unwrap();
    let mut click = input.begin_transaction(client.clone(), None).await.unwrap();
    click.set_owned_mouse_press(&owner).unwrap();
    click.mouse_down(MouseButton::Left, 1).await.unwrap();
    drop(click);
    let mut other = input.begin_transaction(client.clone(), None).await.unwrap();
    other.mouse_down(MouseButton::Right, 4).await.unwrap();
    drop(other);

    // Act
    input
        .release_owned_mouse_press(client.as_ref(), &owner)
        .await
        .unwrap();

    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[1].1[0]["buttons"], 3);
    assert_eq!(calls[2].1[0]["type"], "mouseReleased");
    assert_eq!(calls[2].1[0]["button"], "left");
    assert_eq!(calls[2].1[0]["buttons"], 2);
}

#[tokio::test]
async fn raw_mouse_down_invalidates_an_older_owned_locator_press() {
    // Arrange
    let (client, input) = setup();
    let owner = input.new_owned_mouse_press().unwrap();
    let mut click = input.begin_transaction(client.clone(), None).await.unwrap();
    click.set_owned_mouse_press(&owner).unwrap();
    click.mouse_down(MouseButton::Left, 1).await.unwrap();
    drop(click);
    let mut raw = input.begin_transaction(client.clone(), None).await.unwrap();
    raw.raw_mouse_down(MouseButton::Left, 2).await.unwrap();
    drop(raw);

    // Act
    input
        .release_owned_mouse_press(client.as_ref(), &owner)
        .await
        .unwrap();

    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].1[0]["type"], "mousePressed");
    assert_eq!(calls[1].1[0]["clickCount"], 2);
}

#[tokio::test]
async fn owned_mouse_press_rejects_a_different_page_before_dispatch() {
    // Arrange
    let (client, first) = setup();
    let (_, second) = setup();
    let owner = first.new_owned_mouse_press().unwrap();
    let mut action = second
        .begin_transaction(client.clone(), None)
        .await
        .unwrap();

    // Act
    let selection = action.set_owned_mouse_press(&owner);
    let cleanup = second
        .release_owned_mouse_press(client.as_ref(), &owner)
        .await;

    // Assert
    assert!(selection.unwrap_err().to_string().contains("another Page"));
    assert!(cleanup.unwrap_err().to_string().contains("another Page"));
    assert!(client.calls().is_empty());
}

#[tokio::test]
async fn wheel_is_native_and_uses_shared_pointer_and_keyboard_state() {
    // Arrange
    let (client, input) = setup();
    let mut first = input.begin_transaction(client.clone(), None).await.unwrap();
    first
        .raw_mouse_move(10.0, 20.0, MouseMoveSteps::ONE)
        .await
        .unwrap();
    first.key_down_named("Shift", false).await.unwrap();
    first.finish().await.unwrap();
    // Act
    let mut second = input.begin_transaction(client.clone(), None).await.unwrap();
    second.raw_mouse_wheel(1.5, -90.25).await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[2].0, "Input.dispatchMouseEvent");
    let wheel = &calls[2].1[0];
    assert_eq!(wheel["type"], "mouseWheel");
    assert_eq!(wheel["x"], 10.0);
    assert_eq!(wheel["modifiers"], 8);
    assert_eq!(wheel["deltaX"], 1.5);
    assert_eq!(wheel["deltaY"], -90.25);
    assert!(wheel.get("buttons").is_none());
}

#[tokio::test]
async fn exclusive_policy_rejects_raw_pointer_without_dispatch() {
    // Arrange
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = CdpInputCoordinator::new();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    // Act
    let error = action
        .raw_mouse_down(MouseButton::Left, 1)
        .await
        .unwrap_err();
    // Assert
    assert!(error.to_string().contains("interleaved input policy"));
    assert!(client.calls().is_empty());
}

#[tokio::test]
async fn nonfinite_coordinates_reject_before_mutating_destination() {
    // Arrange
    let (client, input) = setup();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    // Act
    let error = action
        .raw_mouse_move(f64::NAN, 9.0, MouseMoveSteps::ONE)
        .await
        .unwrap_err();
    action.raw_mouse_down(MouseButton::Left, 1).await.unwrap();
    // Assert
    assert!(error.to_string().contains("finite"));
    let calls = client.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].1[0]["x"], 0.0);
    assert_eq!(calls[0].1[0]["y"], 0.0);
}

#[tokio::test]
async fn locator_pointer_transaction_and_raw_mouse_share_position_and_button_state() {
    // Arrange: existing Locator primitives keep their compatibility API.
    let (client, input) = setup();
    let mut locator = input.begin_transaction(client.clone(), None).await.unwrap();
    locator.move_mouse(13.0, 14.0).await.unwrap();
    locator.mouse_down(MouseButton::Right, 2).await.unwrap();
    // Act
    let mut raw = input.begin_transaction(client.clone(), None).await.unwrap();
    raw.raw_mouse_move(23.0, 24.0, MouseMoveSteps::ONE)
        .await
        .unwrap();
    raw.finish().await.unwrap();
    locator.mouse_up().await.unwrap();
    locator.finish().await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 4);
    assert_eq!(calls[2].1[0]["button"], "right");
    assert_eq!(calls[2].1[0]["buttons"], 2);
    assert_eq!(calls[3].1[0]["x"], 23.0);
    assert_eq!(calls[3].1[0]["y"], 24.0);
    assert_eq!(calls[3].1[0]["buttons"], 0);
}

#[tokio::test]
async fn failed_locator_interpolation_preserves_destination_for_raw_mouse() {
    // Arrange
    let (client, input) = setup();
    client.set_input_errors([Some("move failed".into()), None]);
    let mut locator = input.begin_transaction(client.clone(), None).await.unwrap();
    // Act
    let error = locator
        .move_mouse_in_steps(100.0, 80.0, std::num::NonZeroU32::new(4).unwrap())
        .await
        .unwrap_err();
    drop(locator);
    let mut raw = input.begin_transaction(client.clone(), None).await.unwrap();
    raw.raw_mouse_down(MouseButton::Left, 1).await.unwrap();
    // Assert
    assert!(error.to_string().contains("move failed"));
    let calls = client.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1[0]["x"], 25.0);
    assert_eq!(calls[1].1[0]["x"], 100.0);
    assert_eq!(calls[1].1[0]["y"], 80.0);
}
