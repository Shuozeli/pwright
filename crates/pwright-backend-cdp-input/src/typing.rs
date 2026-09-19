//! Caller-owned sequential typing; no selector, clock, task or text allocation.
use std::future::Future;

use pwright_cdp::connection::Result;
use pwright_input_model::InputText;

use crate::{CdpInputTransaction, resolve_key};

/// Types literal Unicode scalar values using Playwright's key/insertText split.
///
/// The caller supplies each delay future, owns the one overall deadline and
/// retains the Page transaction until completion. Errors are not retried. On
/// cancellation the caller must finish or drain the coordinator's pending input
/// cleanup before abandoning the native session. No self-owned task is spawned.
pub async fn type_text<Wait: Future<Output = Result<()>>>(
    transaction: &mut CdpInputTransaction,
    text: InputText<'_>,
    is_mac: bool,
    mut delay: impl FnMut() -> Wait,
) -> Result<()> {
    for character in text.as_str().chars() {
        let mut bytes = [0; 4];
        let literal = character.encode_utf8(&mut bytes);
        // Character aliases in upstream's layout already encode literal case;
        // held Shift does not turn a literal `a` into the physical `KeyA` alias.
        match resolve_key(literal, false, is_mac) {
            Ok(key) => {
                transaction.key_down(key, is_mac).await?;
                delay().await?;
                transaction.key_up(key).await?;
            }
            Err(_) => {
                delay().await?;
                transaction
                    .insert_text(
                        InputText::try_from(&*literal)
                            .expect("one scalar fits the bounded input text model"),
                    )
                    .await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CdpInputCoordinator, CdpModifiers};
    use pwright_cdp::connection::CdpError;
    use pwright_fake::FakeCdpClient;
    use std::{
        cell::Cell,
        future::{Future, ready},
        sync::Arc,
        task::{Context, Waker},
    };

    #[tokio::test]
    async fn typing_mixes_literal_keys_and_unicode_without_shortcut_parsing() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::new();
        let mut transaction = input.begin_transaction(client.clone(), None).await.unwrap();
        let delays = Cell::new(0);
        // Act
        type_text(
            &mut transaction,
            InputText::try_from("aA+中😀\n\t").unwrap(),
            false,
            || {
                delays.set(delays.get() + 1);
                ready(Ok(()))
            },
        )
        .await
        .unwrap();
        // Assert
        let calls = client.calls();
        assert_eq!(delays.get(), 7);
        assert_eq!(calls.len(), 11);
        assert_eq!(calls[0].1[0]["key"], "a");
        assert_eq!(calls[2].1[0]["key"], "A");
        assert_eq!(calls[4].1[0]["key"], "+");
        assert_eq!(calls[6].0, "Input.insertText");
        assert_eq!(calls[6].1[0], "中");
        assert_eq!(calls[7].1[0], "😀");
        assert_eq!(calls[8].1[0]["key"], "Enter");
        assert_eq!(calls[10].1[0], "\t");
        transaction.finish().await.unwrap();
    }

    #[tokio::test]
    async fn literal_case_is_preserved_with_held_shift() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::new();
        let mut transaction = input
            .begin_transaction(client.clone(), Some(CdpModifiers::SHIFT))
            .await
            .unwrap();
        // Act
        type_text(
            &mut transaction,
            InputText::try_from("a").unwrap(),
            false,
            || ready(Ok(())),
        )
        .await
        .unwrap();
        // Assert
        let calls = client.calls();
        assert_eq!(calls[1].1[0]["key"], "a");
        assert_eq!(calls[1].1[0]["text"], "a");
        assert_eq!(calls[1].1[0]["modifiers"], 8);
        transaction.finish().await.unwrap();
    }

    #[tokio::test]
    async fn cancelled_delay_leaves_acknowledged_key_for_existing_cleanup() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::new();
        let mut transaction = input.begin_transaction(client.clone(), None).await.unwrap();
        let mut operation = Box::pin(type_text(
            &mut transaction,
            InputText::try_from("ab").unwrap(),
            false,
            std::future::pending::<Result<()>>,
        ));
        // Act
        let outcome = operation
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()));
        drop(operation);
        drop(transaction);
        input.cleanup_pending(client.as_ref()).await.unwrap();
        // Assert
        assert!(outcome.is_pending());
        let calls = client.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1[0]["key"], "a");
        assert_eq!(calls[1].1[0]["type"], "keyUp");
    }

    #[tokio::test]
    async fn unicode_delay_error_prevents_insert_and_later_characters() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::new();
        let mut transaction = input.begin_transaction(client.clone(), None).await.unwrap();
        // Act
        let outcome = type_text(
            &mut transaction,
            InputText::try_from("中a").unwrap(),
            false,
            || ready(Err(CdpError::Closed)),
        )
        .await;
        // Assert
        assert!(matches!(outcome, Err(CdpError::Closed)));
        assert!(client.calls().is_empty());
        transaction.finish().await.unwrap();
    }

    #[tokio::test]
    async fn native_insert_failure_stops_without_retry_or_later_keydown() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_input_errors([Some("insert failed".into())]);
        let input = CdpInputCoordinator::new();
        let mut transaction = input.begin_transaction(client.clone(), None).await.unwrap();
        // Act
        let result = type_text(
            &mut transaction,
            InputText::try_from("中a").unwrap(),
            false,
            || ready(Ok(())),
        )
        .await;
        transaction.finish().await.unwrap();
        // Assert
        assert!(result.is_err());
        let calls = client.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "Input.insertText");
        assert_eq!(calls[0].1[0], "中");
    }

    #[tokio::test]
    async fn native_keydown_failure_does_not_wait_or_type_later_characters() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_input_errors([Some("key down failed".into())]);
        let input = CdpInputCoordinator::new();
        let mut transaction = input.begin_transaction(client.clone(), None).await.unwrap();
        let waits = Cell::new(0);
        // Act
        let result = type_text(
            &mut transaction,
            InputText::try_from("ab").unwrap(),
            false,
            || {
                waits.set(waits.get() + 1);
                ready(Ok(()))
            },
        )
        .await;
        // Assert
        assert!(result.is_err());
        assert_eq!(waits.get(), 0);
        assert_eq!(client.calls().len(), 1);
        transaction.finish().await.unwrap();
    }

    #[tokio::test]
    async fn keyup_failure_stops_text_and_remains_recoverable_by_transaction_owner() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_input_errors([None, Some("key up failed".into()), None]);
        let input = CdpInputCoordinator::new();
        let mut transaction = input.begin_transaction(client.clone(), None).await.unwrap();
        // Act
        let result = type_text(
            &mut transaction,
            InputText::try_from("ab").unwrap(),
            false,
            || ready(Ok(())),
        )
        .await;
        // Assert: typing did not retry or send `b`; existing explicit cleanup owns recovery.
        assert!(result.is_err());
        assert_eq!(client.calls().len(), 2);
        transaction.finish().await.unwrap();
        let calls = client.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[2].1[0]["type"], "keyUp");
        assert_eq!(calls[2].1[0]["key"], "a");
    }

    #[tokio::test]
    async fn cancelled_unicode_delay_never_inserts_or_leaves_a_pressed_key() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::new();
        let mut transaction = input.begin_transaction(client.clone(), None).await.unwrap();
        let mut operation = Box::pin(type_text(
            &mut transaction,
            InputText::try_from("😀a").unwrap(),
            false,
            std::future::pending::<Result<()>>,
        ));
        // Act
        let outcome = operation
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()));
        drop(operation);
        drop(transaction);
        input.cleanup_pending(client.as_ref()).await.unwrap();
        // Assert
        assert!(outcome.is_pending());
        assert!(client.calls().is_empty());
    }

    #[tokio::test]
    async fn delay_occurs_between_physical_events_but_before_unicode_insertion() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::new();
        let mut transaction = input.begin_transaction(client.clone(), None).await.unwrap();
        let phase = Cell::new(0);
        // Act
        type_text(
            &mut transaction,
            InputText::try_from("a中").unwrap(),
            false,
            || {
                let calls = client.calls();
                if phase.get() == 0 {
                    assert_eq!(calls.len(), 1);
                    assert_eq!(calls[0].1[0]["type"], "keyDown");
                } else {
                    assert_eq!(calls.len(), 2);
                    assert_eq!(calls[1].1[0]["type"], "keyUp");
                }
                phase.set(phase.get() + 1);
                ready(Ok(()))
            },
        )
        .await
        .unwrap();
        // Assert
        assert_eq!(phase.get(), 2);
        assert_eq!(client.calls()[2].0, "Input.insertText");
        transaction.finish().await.unwrap();
    }

    #[tokio::test]
    async fn empty_text_performs_no_io_or_delay() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::new();
        let mut transaction = input.begin_transaction(client.clone(), None).await.unwrap();
        // Act
        type_text(
            &mut transaction,
            InputText::try_from("").unwrap(),
            false,
            || ready(Err(CdpError::Closed)),
        )
        .await
        .unwrap();
        // Assert
        assert!(client.calls().is_empty());
        transaction.finish().await.unwrap();
    }
}
