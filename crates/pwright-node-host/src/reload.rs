use crate::{HostError, HostSession, NavigationResult, ResponseMetadata};
use pwright_cap_navigation::{Reload, ReloadParams};

#[cfg(feature = "page-bindings")]
async fn await_reload_binding_publications(
    publication: impl std::future::Future<Output = Result<(), HostError>>,
    stopped: &tokio_util::sync::CancellationToken,
    deadline: Option<tokio::time::Instant>,
    timeout_ms: u32,
) -> Result<(), HostError> {
    tokio::pin!(publication);
    if let Some(deadline) = deadline {
        tokio::select! {
            biased;
            () = stopped.cancelled() => Err(HostError::TargetClosed),
            () = tokio::time::sleep_until(deadline) => Err(HostError::Timeout(timeout_ms)),
            result = &mut publication => result,
        }
    } else {
        tokio::select! {
            biased;
            () = stopped.cancelled() => Err(HostError::TargetClosed),
            result = &mut publication => result,
        }
    }
}

#[cfg(test)]
mod boundary_probe;

#[cfg(all(test, feature = "page-bindings"))]
mod tests {
    use super::*;
    use std::future::Future;

    #[tokio::test]
    async fn reload_waits_for_its_frame_publication_acknowledgement() {
        // Arrange
        let stopped = tokio_util::sync::CancellationToken::new();
        let (acknowledge, acknowledged) = tokio::sync::oneshot::channel();
        let publication = await_reload_binding_publications(
            async { acknowledged.await.map_err(HostError::backend) },
            &stopped,
            None,
            0,
        );
        tokio::pin!(publication);

        // Act
        let pending = publication
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()));
        acknowledge.send(()).unwrap();
        let result = publication.await;

        // Assert
        assert!(pending.is_pending());
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn reload_publication_cannot_outlive_its_original_deadline() {
        // Arrange
        let stopped = tokio_util::sync::CancellationToken::new();
        let publication = std::future::pending::<Result<(), HostError>>();
        let deadline = tokio::time::Instant::now();

        // Act
        let result =
            await_reload_binding_publications(publication, &stopped, Some(deadline), 37).await;

        // Assert
        assert!(matches!(result, Err(HostError::Timeout(37))));
    }

    #[tokio::test]
    async fn shutdown_retires_pending_reload_publication() {
        // Arrange
        let stopped = tokio_util::sync::CancellationToken::new();
        stopped.cancel();
        let publication = std::future::pending::<Result<(), HostError>>();

        // Act
        let result = await_reload_binding_publications(publication, &stopped, None, 0).await;

        // Assert
        assert!(matches!(result, Err(HostError::TargetClosed)));
    }
}

impl HostSession {
    pub async fn page_reload(
        &self,
        page_id: &str,
        wait_until: &str,
        timeout_ms: u32,
    ) -> Result<NavigationResult, HostError> {
        let wait_until = crate::commands::parse_wait_until(wait_until)?;
        #[cfg(feature = "page-bindings")]
        let deadline = (timeout_ms != 0).then(|| {
            tokio::time::Instant::now() + std::time::Duration::from_millis(u64::from(timeout_ms))
        });
        let result = self
            .invoke::<Reload>(page_id, ReloadParams { wait_until }, timeout_ms)
            .await;
        let result = crate::completion::publish_command_result(
            result,
            self.is_connected(),
            self.flush_topology(),
        )
        .await?;
        #[cfg(feature = "page-bindings")]
        if let Some(frame) = self.page_binding_frame_for_reload(page_id)? {
            await_reload_binding_publications(
                self.drain_frame_binding_publications(&frame),
                &self.stop,
                deadline,
                timeout_ms,
            )
            .await?;
        }
        let request_id = result.loader_id;
        #[cfg(feature = "response-request")]
        let request = if result.response.is_some() {
            match request_id.as_deref() {
                Some(request_id) => Some(self.request_by_native_id(page_id, request_id).await?),
                None => None,
            }
        } else {
            None
        };
        Ok(NavigationResult {
            url: result.url,
            response: result.response.map(|response| ResponseMetadata {
                request_id,
                #[cfg(feature = "response-request")]
                request,
                #[cfg(feature = "navigation-response-finished")]
                finished_owner: None,
                #[cfg(feature = "response-security-details")]
                security_details: None,
                url: response.url,
                status: response.status,
                status_text: response.status_text,
                ok: (200..=299).contains(&response.status),
                headers: response
                    .headers
                    .into_iter()
                    .map(|header| (header.name.to_ascii_lowercase(), header.value))
                    .collect(),
            }),
        })
    }
}
