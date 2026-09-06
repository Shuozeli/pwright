//! Isolated Chromium CDP BrowserContext lifecycle adapter for Pwright.
//!
//! The adapter closes only an explicitly owned native context. It does not
//! own topology publication, transport, browser processes, or other commands.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use pwright_cap_context_lifecycle::ContextLifecycleErrorDetails;
use pwright_cdp::CdpClient;
use pwright_cdp::connection::CdpError;
use pwright_core::{FailureKind, OperationError, Service};

use pwright_backend_cdp_topology::CdpContextOwnership;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpContextCloseLimits {
    pub native_id_bytes: usize,
}

impl CdpContextCloseLimits {
    pub const fn new(native_id_bytes: usize) -> Self {
        Self { native_id_bytes }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdpContextCloseSetupError {
    EmptyNativeId,
    NativeIdTooLong { actual: usize, limit: usize },
}

impl Display for CdpContextCloseSetupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyNativeId => formatter.write_str("CDP browser-context ID must not be empty"),
            Self::NativeIdTooLong { actual, limit } => write!(
                formatter,
                "CDP browser-context ID has {actual} bytes, exceeding limit {limit}"
            ),
        }
    }
}

impl Error for CdpContextCloseSetupError {}

/// One native browser-context close operation bound to an exact topology route.
///
/// Construction owns and compacts the bounded native ID. Calling the service
/// performs no hidden retry and never substitutes Browser.close.
#[derive(Clone)]
pub struct CdpBrowserContextCloseService {
    client: Arc<dyn CdpClient>,
    browser_context_id: Arc<str>,
    ownership: CdpContextOwnership,
}

impl CdpBrowserContextCloseService {
    pub fn new(
        client: Arc<dyn CdpClient>,
        browser_context_id: impl Into<String>,
        ownership: CdpContextOwnership,
        limits: CdpContextCloseLimits,
    ) -> Result<Self, CdpContextCloseSetupError> {
        let browser_context_id = browser_context_id.into();
        if browser_context_id.is_empty() {
            return Err(CdpContextCloseSetupError::EmptyNativeId);
        }
        if browser_context_id.len() > limits.native_id_bytes {
            return Err(CdpContextCloseSetupError::NativeIdTooLong {
                actual: browser_context_id.len(),
                limit: limits.native_id_bytes,
            });
        }
        Ok(Self {
            client,
            browser_context_id: Arc::from(browser_context_id.into_boxed_str()),
            ownership,
        })
    }

    pub fn browser_context_id(&self) -> &str {
        &self.browser_context_id
    }

    pub const fn ownership(&self) -> CdpContextOwnership {
        self.ownership
    }

    pub async fn close_route(
        client: &dyn CdpClient,
        browser_context_id: &str,
        ownership: CdpContextOwnership,
    ) -> Result<(), OperationError<ContextLifecycleErrorDetails>> {
        if ownership != CdpContextOwnership::Owned {
            let ownership = match ownership {
                CdpContextOwnership::Default => "default",
                CdpContextOwnership::External => "external",
                CdpContextOwnership::Owned => unreachable!("owned context passed rejection"),
            };
            return Err(OperationError::new(
                FailureKind::UnsupportedCapability,
                format!("cannot close {ownership} CDP browser context"),
                Some(ContextLifecycleErrorDetails::NotOwned(format!(
                    "{ownership} context is not owned by this Pwright connection"
                ))),
            ));
        }
        match client
            .target_dispose_browser_context(browser_context_id)
            .await
        {
            Ok(()) => Ok(()),
            Err(error) => {
                let error = map_close_error(error);
                if error.kind() == FailureKind::TargetClosed {
                    Ok(())
                } else {
                    Err(error)
                }
            }
        }
    }
}

fn map_close_error(error: CdpError) -> OperationError<ContextLifecycleErrorDetails> {
    let kind = pwright_backend_cdp_common::classify_cdp_failure(&error);
    let message = error.to_string();
    OperationError::new(
        kind,
        format!("CDP Target.disposeBrowserContext failed: {message}"),
        Some(ContextLifecycleErrorDetails::Backend(message)),
    )
}

impl Service<()> for CdpBrowserContextCloseService {
    type Response = ();
    type Error = OperationError<ContextLifecycleErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, (): ()) -> Self::Future<'_> {
        Box::pin(Self::close_route(
            self.client.as_ref(),
            &self.browser_context_id,
            self.ownership,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cdp::connection::CdpError;
    use pwright_core::{FailureKind, Service};
    use pwright_fake::FakeCdpClient;

    use super::{CdpBrowserContextCloseService, CdpContextCloseLimits, CdpContextCloseSetupError};
    use pwright_backend_cdp_topology::CdpContextOwnership;

    fn limits() -> CdpContextCloseLimits {
        CdpContextCloseLimits::new(64)
    }

    #[tokio::test]
    async fn owned_context_uses_only_dispose_browser_context() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let service = CdpBrowserContextCloseService::new(
            client.clone(),
            "context-7",
            CdpContextOwnership::Owned,
            limits(),
        )
        .expect("owned context route should build");

        // Act
        service.call(()).await.expect("owned close should succeed");

        // Assert
        assert_eq!(client.method_names(), ["Target.disposeBrowserContext"]);
        assert_eq!(
            client.calls()[0].1,
            [serde_json::Value::String("context-7".to_string())]
        );
    }

    #[tokio::test]
    async fn default_and_external_contexts_fail_before_cdp() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let default_service = CdpBrowserContextCloseService::new(
            client.clone(),
            "default-context",
            CdpContextOwnership::Default,
            limits(),
        )
        .expect("default route should remain representable");
        let external_service = CdpBrowserContextCloseService::new(
            client.clone(),
            "external-context",
            CdpContextOwnership::External,
            limits(),
        )
        .expect("external route should remain representable");

        // Act
        let default_error = default_service
            .call(())
            .await
            .expect_err("default context must not close");
        let external_error = external_service
            .call(())
            .await
            .expect_err("external context must not close");

        // Assert
        assert_eq!(default_error.kind(), FailureKind::UnsupportedCapability);
        assert_eq!(external_error.kind(), FailureKind::UnsupportedCapability);
        assert!(client.method_names().is_empty());
    }

    #[tokio::test]
    async fn racing_missing_owned_context_is_idempotent_after_native_absence() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_dispose_browser_context_error(Some(
            "Failed to find context with id context-7".to_string(),
        ));
        let service = CdpBrowserContextCloseService::new(
            client.clone(),
            "context-7",
            CdpContextOwnership::Owned,
            limits(),
        )
        .unwrap();

        // Act
        let outcome = service.call(()).await;

        // Assert
        assert_eq!(outcome, Ok(()));
        assert_eq!(client.method_names(), ["Target.disposeBrowserContext"]);
    }

    #[test]
    fn setup_rejects_empty_and_oversized_native_ids() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));

        // Act
        let empty = CdpBrowserContextCloseService::new(
            client.clone(),
            "",
            CdpContextOwnership::Owned,
            limits(),
        );
        let oversized = CdpBrowserContextCloseService::new(
            client,
            "12345",
            CdpContextOwnership::Owned,
            CdpContextCloseLimits::new(4),
        );

        // Assert
        assert!(matches!(
            empty,
            Err(CdpContextCloseSetupError::EmptyNativeId)
        ));
        assert!(matches!(
            oversized,
            Err(CdpContextCloseSetupError::NativeIdTooLong {
                actual: 5,
                limit: 4
            })
        ));
    }

    #[test]
    fn missing_native_context_is_a_target_closed_race() {
        // Arrange
        let error = CdpError::Protocol {
            code: -32000,
            message: "Failed to find context with id context-7".to_string(),
        };

        // Act
        let error = super::map_close_error(error);

        // Assert
        assert_eq!(error.kind(), FailureKind::TargetClosed);
    }
}
