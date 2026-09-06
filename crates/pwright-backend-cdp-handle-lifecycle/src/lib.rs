//! Isolated Chromium CDP JavaScript Handle lifecycle adapter for Pwright.
//!
//! The adapter releases one bounded native RemoteObject identity and owns no
//! evaluation, topology, transport, runtime, or browser process.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use pwright_cap_handle_lifecycle::HandleLifecycleErrorDetails;
use pwright_cdp::CdpClient;
use pwright_cdp::connection::CdpError;
use pwright_core::{OperationError, Service};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpHandleDisposeLimits {
    pub native_id_bytes: usize,
}

impl CdpHandleDisposeLimits {
    pub const fn new(native_id_bytes: usize) -> Self {
        Self { native_id_bytes }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdpHandleDisposeSetupError {
    EmptyNativeId,
    NativeIdTooLong { actual: usize, limit: usize },
}

impl Display for CdpHandleDisposeSetupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyNativeId => formatter.write_str("CDP remote-object ID must not be empty"),
            Self::NativeIdTooLong { actual, limit } => write!(
                formatter,
                "CDP remote-object ID has {actual} bytes, exceeding limit {limit}"
            ),
        }
    }
}

impl Error for CdpHandleDisposeSetupError {}

/// One explicit handle-dispose operation bound to its exact CDP session.
///
/// Construction owns the bounded native object ID. The service issues exactly
/// one `Runtime.releaseObject` call and performs no hidden retry.
#[derive(Clone)]
pub struct CdpHandleDisposeService {
    client: Arc<dyn CdpClient>,
    object_id: Arc<str>,
}

impl CdpHandleDisposeService {
    pub fn new(
        client: Arc<dyn CdpClient>,
        object_id: impl Into<String>,
        limits: CdpHandleDisposeLimits,
    ) -> Result<Self, CdpHandleDisposeSetupError> {
        let object_id = object_id.into();
        if object_id.is_empty() {
            return Err(CdpHandleDisposeSetupError::EmptyNativeId);
        }
        if object_id.len() > limits.native_id_bytes {
            return Err(CdpHandleDisposeSetupError::NativeIdTooLong {
                actual: object_id.len(),
                limit: limits.native_id_bytes,
            });
        }
        Ok(Self {
            client,
            object_id: Arc::from(object_id.into_boxed_str()),
        })
    }

    pub fn object_id(&self) -> &str {
        &self.object_id
    }
}

impl Service<()> for CdpHandleDisposeService {
    type Response = ();
    type Error = OperationError<HandleLifecycleErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, (): ()) -> Self::Future<'_> {
        Box::pin(async move {
            self.client
                .runtime_release_object(&self.object_id)
                .await
                .map_err(map_dispose_error)
        })
    }
}

fn map_dispose_error(error: CdpError) -> OperationError<HandleLifecycleErrorDetails> {
    let kind = pwright_backend_cdp_common::classify_cdp_failure(&error);
    let message = error.to_string();
    OperationError::new(
        kind,
        format!("CDP Runtime.releaseObject failed: {message}"),
        Some(HandleLifecycleErrorDetails::Backend(message)),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_core::{FailureKind, Service};
    use pwright_fake::FakeCdpClient;

    use super::{CdpHandleDisposeLimits, CdpHandleDisposeService, CdpHandleDisposeSetupError};

    fn limits() -> CdpHandleDisposeLimits {
        CdpHandleDisposeLimits::new(64)
    }

    #[tokio::test]
    async fn dispose_uses_only_release_object() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let service = CdpHandleDisposeService::new(client.clone(), "object-7", limits())
            .expect("handle route should build");

        // Act
        service.call(()).await.expect("dispose should succeed");

        // Assert
        assert_eq!(client.method_names(), ["Runtime.releaseObject"]);
        assert_eq!(
            client.calls()[0].1,
            [serde_json::Value::String("object-7".to_string())]
        );
    }

    #[tokio::test]
    async fn backend_failure_remains_visible() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_release_object_error(Some("Cannot find object".to_string()));
        let service = CdpHandleDisposeService::new(client, "object-7", limits()).unwrap();

        // Act
        let error = service
            .call(())
            .await
            .expect_err("native release failure must remain visible");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(error.message().contains("Runtime.releaseObject"));
    }

    #[test]
    fn setup_rejects_empty_and_oversized_native_ids() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));

        // Act
        let empty = CdpHandleDisposeService::new(client.clone(), "", limits());
        let oversized =
            CdpHandleDisposeService::new(client, "12345", CdpHandleDisposeLimits::new(4));

        // Assert
        assert!(matches!(
            empty,
            Err(CdpHandleDisposeSetupError::EmptyNativeId)
        ));
        assert!(matches!(
            oversized,
            Err(CdpHandleDisposeSetupError::NativeIdTooLong {
                actual: 5,
                limit: 4
            })
        ));
    }
}
