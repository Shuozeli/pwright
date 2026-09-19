use super::*;
use crate::{CdpInputCoordinator, CdpInputPolicy, CdpModifiers};
use pwright_fake::FakeCdpClient;
use std::{
    cell::Cell,
    future::{pending, ready},
    sync::Arc,
    task::{Context, Waker},
};

#[test]
fn raw_empty_tail_is_preserved_without_changing_strict_parser_or_token_limit() {
    // Arrange
    let oversized = std::iter::repeat_n("Shift", 17)
        .collect::<Vec<_>>()
        .join("+");
    // Act
    let empty = KeyTokens::parse_raw_keyboard("").unwrap();
    let trailing = KeyTokens::parse_raw_keyboard("Shift+").unwrap();
    // Assert
    assert_eq!(empty.as_slice(), [""]);
    assert_eq!(trailing.as_slice(), ["Shift", ""]);
    assert!(KeyTokens::parse("Shift+").is_err());
    assert!(matches!(
        KeyTokens::parse_raw_keyboard(&oversized),
        Err(crate::KeyChordParseError::TooManyTokens {
            tokens: 17,
            maximum: 16
        })
    ));
}

#[tokio::test]
async fn raw_empty_tail_rejects_after_modifying_the_page_prefix_state() {
    // Arrange
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let keys = KeyTokens::parse_raw_keyboard("Shift+").unwrap();
    // Act
    let result = press_keys(&mut action, &keys, false, || ready(Ok(()))).await;
    // Assert
    assert!(matches!(result, Err(KeyInputError::UnknownKey(""))));
    assert_eq!(client.calls().len(), 1);
    assert_eq!(action.modifiers(), CdpModifiers::SHIFT);
    action.key_up_named("Shift", false).await.unwrap();
    action.finish().await.unwrap();
}

#[tokio::test]
async fn raw_chord_presses_forward_and_releases_reverse_with_one_delay() {
    // Arrange
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let mut transaction = input.begin_transaction(client.clone(), None).await.unwrap();
    let keys = KeyTokens::parse("Control+Shift+KeyA").unwrap();
    let delays = Cell::new(0);
    // Act
    press_keys(&mut transaction, &keys, false, || {
        delays.set(delays.get() + 1);
        ready(Ok(()))
    })
    .await
    .unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(delays.get(), 1);
    assert_eq!(calls.len(), 6);
    let names: Vec<_> = calls
        .iter()
        .map(|call| call.1[0]["key"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Control", "Shift", "A", "A", "Shift", "Control"]);
    assert_eq!(calls[2].1[0]["modifiers"], 10);
    assert_eq!(calls[5].1[0]["modifiers"], 0);
    assert_eq!(transaction.modifiers(), CdpModifiers::NONE);
    transaction.finish().await.unwrap();
}

#[tokio::test]
async fn repeated_physical_aliases_use_page_state_across_transactions() {
    // Arrange
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let mut first = input.begin_transaction(client.clone(), None).await.unwrap();
    first.key_down_named("a", false).await.unwrap();
    first.finish().await.unwrap();
    let mut second = input.begin_transaction(client.clone(), None).await.unwrap();
    // Act
    second.key_down_named("KeyA", false).await.unwrap();
    // Assert
    assert_eq!(client.calls()[1].1[0]["autoRepeat"], true);
    second.key_up_named("KeyA", false).await.unwrap();
    second.finish().await.unwrap();
}

#[tokio::test]
async fn delay_does_not_lock_other_input_and_keyup_uses_current_shift() {
    // Arrange
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let mut first = input.begin_transaction(client.clone(), None).await.unwrap();
    let mut second = input.begin_transaction(client.clone(), None).await.unwrap();
    let keys = KeyTokens::parse("KeyA").unwrap();
    let (release, delay) = tokio::sync::oneshot::channel();
    let mut press = Box::pin(press_keys(&mut first, &keys, false, || async {
        delay.await.map_err(|_| CdpError::Closed)
    }));
    assert!(
        press
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    second.key_down_named("Shift", false).await.unwrap();
    release.send(()).unwrap();
    press.await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].1[0]["key"], "a");
    assert_eq!(calls[2].1[0]["type"], "keyUp");
    assert_eq!(calls[2].1[0]["key"], "A");
    assert_eq!(calls[2].1[0]["modifiers"], 8);
    second.key_up_named("Shift", false).await.unwrap();
    first.finish().await.unwrap();
    second.finish().await.unwrap();
}

#[tokio::test]
async fn unknown_later_token_preserves_the_acknowledged_prefix_without_delay() {
    // Arrange
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let keys = KeyTokens::parse("Shift+NotAKey").unwrap();
    let waited = Cell::new(false);
    // Act
    let result = press_keys(&mut action, &keys, false, || {
        waited.set(true);
        ready(Ok(()))
    })
    .await;
    // Assert
    assert!(matches!(result, Err(KeyInputError::UnknownKey("NotAKey"))));
    assert!(!waited.get());
    assert_eq!(client.calls().len(), 1);
    assert_eq!(action.modifiers(), CdpModifiers::SHIFT);
    action.key_up_named("Shift", false).await.unwrap();
    action.finish().await.unwrap();
}

#[tokio::test]
async fn cancelled_raw_press_preserves_interleaved_keys_until_explicit_release() {
    // Arrange
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let keys = KeyTokens::parse("Shift+KeyA").unwrap();
    let mut press = Box::pin(press_keys(
        &mut action,
        &keys,
        false,
        pending::<Result<(), CdpError>>,
    ));
    // Act
    assert!(
        press
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(press);
    drop(action);
    input.cleanup_pending(client.as_ref()).await.unwrap();
    // Assert
    assert_eq!(client.calls().len(), 2);
    let mut cleanup = input.begin_transaction(client.clone(), None).await.unwrap();
    assert_eq!(cleanup.modifiers(), CdpModifiers::SHIFT);
    cleanup.key_up_named("KeyA", false).await.unwrap();
    cleanup.key_up_named("Shift", false).await.unwrap();
    cleanup.finish().await.unwrap();
}

#[tokio::test]
async fn default_exclusive_policy_still_cleans_a_cancelled_raw_press() {
    // Arrange
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = CdpInputCoordinator::new();
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let keys = KeyTokens::parse("Shift+KeyA").unwrap();
    let mut press = Box::pin(press_keys(
        &mut action,
        &keys,
        false,
        pending::<Result<(), CdpError>>,
    ));
    assert!(
        press
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(press);
    drop(action);
    // Act
    input.cleanup_pending(client.as_ref()).await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 4);
    assert_eq!(calls[2].1[0]["key"], "A");
    assert_eq!(calls[3].1[0]["key"], "Shift");
    assert_eq!(calls[3].1[0]["type"], "keyUp");
}

#[tokio::test]
async fn native_failure_does_not_retry_or_dispatch_later_keys() {
    // Arrange
    let client = Arc::new(FakeCdpClient::from_html(""));
    client.set_input_errors([Some("native failure".into())]);
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let keys = KeyTokens::parse("Shift+KeyA").unwrap();
    // Act
    let result = press_keys(&mut action, &keys, false, || ready(Ok(()))).await;
    // Assert
    assert!(matches!(result, Err(KeyInputError::Native(_))));
    assert_eq!(client.calls().len(), 1);
    assert_eq!(action.modifiers(), CdpModifiers::SHIFT);
    action.key_up_named("Shift", false).await.unwrap();
    action.finish().await.unwrap();
}

#[tokio::test]
async fn literal_plus_and_smart_modifier_resolve_without_selector_work() {
    // Arrange
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
    let keys = KeyTokens::parse("ControlOrMeta++").unwrap();
    // Act
    press_keys(&mut action, &keys, true, || ready(Ok(())))
        .await
        .unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 4);
    assert!(calls.iter().all(|call| call.0 == "Input.dispatchKeyEvent"));
    assert_eq!(calls[0].1[0]["key"], "Meta");
    assert_eq!(calls[1].1[0]["key"], "+");
    action.finish().await.unwrap();
}
