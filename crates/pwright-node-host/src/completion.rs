//! Publish completed command side effects without re-admitting retired targets.

use std::future::Future;

use crate::HostError;

pub(crate) async fn publish_command_result<T>(
    result: Result<T, HostError>,
    connected: bool,
    publication: impl Future<Output = Result<(), HostError>>,
) -> Result<T, HostError> {
    // TargetClosed is authoritative runtime retirement, not a transient
    // missing world. Its owner already controls final topology publication.
    // Re-admitting a fence here can overwrite the closed result with queue
    // exhaustion when many calls are cancelled by one Page/Context close.
    if connected && !matches!(result, Err(HostError::TargetClosed)) {
        publication.await?;
    }
    result
}

/// A Page-scoped action is not finished while its publication fence is pending.
/// Keep the caller's exact lifetime witness through that await. A route swap or
/// close intent alone is not a closed lifetime and must not relabel an error.
#[cfg(feature = "frame-navigation")]
pub(crate) async fn publish_lifetime_result<T>(
    result: Result<T, HostError>,
    connected: bool,
    publication: impl Future<Output = Result<(), HostError>>,
    closed: impl Future<Output = ()>,
    is_closed: impl Fn() -> bool,
) -> Result<T, HostError> {
    if is_closed() {
        return Err(HostError::TargetClosed);
    }
    let result = tokio::select! {
        biased;
        () = closed => return Err(HostError::TargetClosed),
        result = publish_command_result(result, connected, publication) => result,
    };
    if is_closed() {
        Err(HostError::TargetClosed)
    } else {
        result
    }
}

#[cfg(all(test, feature = "frame-navigation"))]
mod lifetime_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn retired_commands_do_not_reenter_a_full_publication_queue() {
        // Arrange
        let (sender, mut receiver) = mpsc::channel(8);
        for index in 0..8 {
            sender.try_send(index).unwrap();
        }
        // Act
        let mut results = Vec::new();
        for _ in 0..10 {
            results.push(
                publish_command_result::<()>(Err(HostError::TargetClosed), true, async {
                    sender
                        .try_send(99)
                        .expect("retired command must not publish");
                    Ok(())
                })
                .await,
            );
        }
        // Assert
        assert!(
            results
                .iter()
                .all(|result| matches!(result, Err(HostError::TargetClosed)))
        );
        for index in 0..8 {
            assert_eq!(receiver.try_recv().unwrap(), index);
        }
        assert!(receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn connected_success_waits_for_publication_before_returning() {
        // Arrange
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let completed = publish_command_result(Ok(42), true, async {
            receiver.await.unwrap();
            Ok(())
        });
        tokio::pin!(completed);
        // Act
        let pending = completed
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()));
        sender.send(()).unwrap();
        let result = completed.await;
        // Assert
        assert!(pending.is_pending());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn nonterminal_errors_still_publish_possible_side_effects() {
        // Arrange
        let mut publications = 0;
        let errors = [
            HostError::Timeout(250),
            HostError::Backend("browser exception".into()),
            HostError::ResourceExhausted("command full".into()),
            HostError::UnsupportedOperation("world not ready".into()),
        ];
        // Act
        let mut codes = Vec::new();
        for error in errors {
            let result = publish_command_result::<()>(Err(error), true, async {
                publications += 1;
                Ok(())
            })
            .await;
            codes.push(result.unwrap_err().code());
        }
        // Assert
        assert_eq!(publications, 4);
        assert_eq!(
            codes,
            [
                "PWRIGHT_TIMEOUT",
                "PWRIGHT_BACKEND",
                "PWRIGHT_RESOURCE_EXHAUSTED",
                "PWRIGHT_UNSUPPORTED"
            ]
        );
    }

    #[tokio::test]
    async fn publication_capacity_failure_is_not_suppressed_for_live_commands() {
        // Arrange
        let publication = async { Err(HostError::ResourceExhausted("fence full".into())) };
        // Act
        let result = publish_command_result(Ok(42), true, publication).await;
        // Assert
        assert!(
            matches!(result, Err(HostError::ResourceExhausted(message)) if message == "fence full")
        );
    }

    #[tokio::test]
    async fn disconnected_commands_do_not_poll_publication() {
        // Arrange
        let publication = async { panic!("disconnected host cannot publish") };
        // Act
        let result = publish_command_result(Ok(42), false, publication).await;
        // Assert
        assert_eq!(result.unwrap(), 42);
    }
}
