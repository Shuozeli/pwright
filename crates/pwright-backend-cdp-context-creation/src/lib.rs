//! Isolated Chromium CDP BrowserContext creation adapter for Pwright.
//!
//! It creates one native context, validates its bounded identity, and performs
//! exact-target compensation without owning topology or browser processes.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use pwright_cap_context_creation::ContextCreationErrorDetails;
use pwright_cdp::CdpClient;
use pwright_cdp::connection::CdpError;
use pwright_core::{FailureKind, OperationError, Service};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpContextCreationLimits {
    pub native_id_bytes: usize,
}

impl CdpContextCreationLimits {
    pub const fn new(native_id_bytes: usize) -> Self {
        Self { native_id_bytes }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdpContextCreationSetupError {
    ZeroNativeIdLimit,
}

impl Display for CdpContextCreationSetupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroNativeIdLimit => {
                formatter.write_str("CDP browser-context ID limit must be above zero")
            }
        }
    }
}

impl Error for CdpContextCreationSetupError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdpCreatedBrowserContext {
    browser_context_id: Arc<str>,
}

impl CdpCreatedBrowserContext {
    pub fn browser_context_id(&self) -> &str {
        &self.browser_context_id
    }

    pub fn into_browser_context_id(self) -> String {
        self.browser_context_id.to_string()
    }
}

/// Exact no-options Chromium context creation using the browser CDP session.
///
/// `CdpSession::target_create_browser_context` supplies
/// `disposeOnDetach: true`; this adapter validates and compacts the returned
/// native identity before it can enter a topology table.
#[derive(Clone)]
pub struct CdpBrowserContextCreationService {
    client: Arc<dyn CdpClient>,
    limits: CdpContextCreationLimits,
}

impl CdpBrowserContextCreationService {
    pub fn new(
        client: Arc<dyn CdpClient>,
        limits: CdpContextCreationLimits,
    ) -> Result<Self, CdpContextCreationSetupError> {
        if limits.native_id_bytes == 0 {
            return Err(CdpContextCreationSetupError::ZeroNativeIdLimit);
        }
        Ok(Self { client, limits })
    }
}

impl Service<()> for CdpBrowserContextCreationService {
    type Response = CdpCreatedBrowserContext;
    type Error = OperationError<ContextCreationErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, (): ()) -> Self::Future<'_> {
        Box::pin(async move {
            let browser_context_id = self
                .client
                .target_create_browser_context(None, None)
                .await
                .map_err(map_create_error)?;
            if browser_context_id.is_empty() {
                return Err(reject_invalid_created_context(
                    self.client.as_ref(),
                    &browser_context_id,
                    FailureKind::ProtocolViolation,
                    "CDP Target.createBrowserContext returned an empty browserContextId",
                )
                .await);
            }
            if browser_context_id.len() > self.limits.native_id_bytes {
                let primary = format!(
                    "CDP browserContextId has {} bytes, exceeding limit {}",
                    browser_context_id.len(),
                    self.limits.native_id_bytes
                );
                return Err(reject_invalid_created_context(
                    self.client.as_ref(),
                    &browser_context_id,
                    FailureKind::ResourceExhausted,
                    &primary,
                )
                .await);
            }
            Ok(CdpCreatedBrowserContext {
                browser_context_id: Arc::from(browser_context_id.into_boxed_str()),
            })
        })
    }
}

async fn reject_invalid_created_context(
    client: &dyn CdpClient,
    browser_context_id: &str,
    primary_kind: FailureKind,
    primary: &str,
) -> OperationError<ContextCreationErrorDetails> {
    match client
        .target_dispose_browser_context(browser_context_id)
        .await
    {
        Ok(()) => OperationError::new(
            primary_kind,
            primary,
            Some(ContextCreationErrorDetails::InvalidBackendResponse(
                primary.to_string(),
            )),
        ),
        Err(cleanup) => OperationError::new(
            FailureKind::Cleanup,
            "CDP returned an invalid context identity and compensation failed",
            Some(ContextCreationErrorDetails::Cleanup {
                primary: primary.to_string(),
                cleanup: cleanup.to_string(),
            }),
        ),
    }
}

fn map_create_error(error: CdpError) -> OperationError<ContextCreationErrorDetails> {
    let kind = pwright_backend_cdp_common::classify_cdp_failure(&error);
    let message = error.to_string();
    OperationError::new(
        kind,
        format!("CDP Target.createBrowserContext failed: {message}"),
        Some(ContextCreationErrorDetails::Backend(message)),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_core::{FailureKind, Service};
    use pwright_fake::FakeCdpClient;

    use super::{
        CdpBrowserContextCreationService, CdpContextCreationLimits, CdpContextCreationSetupError,
    };

    #[tokio::test]
    async fn creation_uses_no_proxy_options_and_returns_the_exact_native_id() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_create_browser_context_response("context-7");
        let service = CdpBrowserContextCreationService::new(
            client.clone(),
            CdpContextCreationLimits::new(64),
        )
        .expect("nonzero limit should build");

        // Act
        let created = service.call(()).await.expect("creation should succeed");

        // Assert
        assert_eq!(created.browser_context_id(), "context-7");
        assert_eq!(client.method_names(), ["Target.createBrowserContext"]);
        assert_eq!(
            client.calls()[0].1,
            [serde_json::json!({
                "disposeOnDetach": true,
                "proxyServer": null,
                "proxyBypassList": null,
            })]
        );
    }

    #[tokio::test]
    async fn invalid_native_identity_is_rejected_at_the_backend_boundary() {
        // Arrange
        let empty_client = Arc::new(FakeCdpClient::from_html(""));
        empty_client.set_create_browser_context_response("");
        let long_client = Arc::new(FakeCdpClient::from_html(""));
        long_client.set_create_browser_context_response("12345");
        let empty_service =
            CdpBrowserContextCreationService::new(empty_client, CdpContextCreationLimits::new(4))
                .unwrap();
        let long_service =
            CdpBrowserContextCreationService::new(long_client, CdpContextCreationLimits::new(4))
                .unwrap();

        // Act
        let empty = empty_service
            .call(())
            .await
            .expect_err("empty ID must fail");
        let oversized = long_service
            .call(())
            .await
            .expect_err("oversized ID must fail");

        // Assert
        assert_eq!(empty.kind(), FailureKind::ProtocolViolation);
        assert_eq!(oversized.kind(), FailureKind::ResourceExhausted);
    }

    #[tokio::test]
    async fn cdp_failure_preserves_transport_classification() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_create_browser_context_error(Some("creation rejected".to_string()));
        let service =
            CdpBrowserContextCreationService::new(client, CdpContextCreationLimits::new(64))
                .unwrap();

        // Act
        let error = service
            .call(())
            .await
            .expect_err("CDP failure must surface");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(error.message().contains("Target.createBrowserContext"));
    }

    #[test]
    fn setup_rejects_a_zero_native_identity_limit() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));

        // Act
        let result =
            CdpBrowserContextCreationService::new(client, CdpContextCreationLimits::new(0));

        // Assert
        assert!(matches!(
            result,
            Err(CdpContextCreationSetupError::ZeroNativeIdLimit)
        ));
    }
}
