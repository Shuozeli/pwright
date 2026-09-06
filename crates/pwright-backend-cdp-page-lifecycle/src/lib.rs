//! Isolated Chromium CDP Page lifecycle adapter for Pwright.
//!
//! Ordinary close confirms native target disappearance; before-unload close
//! addresses only the target-local Page session. The adapter owns no topology,
//! route publication, transport, or browser process.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::{Future, ready};
use std::pin::Pin;
use std::sync::Arc;

use pwright_cap_page_lifecycle::PageLifecycleErrorDetails;
use pwright_cdp::CdpClient;
use pwright_cdp::connection::CdpError;
use pwright_core::{FailureKind, OperationError, Service};

pub trait CdpPageClosePoll: Send + Sync + 'static {
    fn wait(&self, completed_checks: usize) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ImmediatePageClosePoll;

impl CdpPageClosePoll for ImmediatePageClosePoll {
    fn wait(&self, _completed_checks: usize) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(ready(()))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpPageCloseLimits {
    pub native_id_bytes: usize,
    pub confirmation_attempts: usize,
}

impl CdpPageCloseLimits {
    pub const fn new(native_id_bytes: usize, confirmation_attempts: usize) -> Self {
        Self {
            native_id_bytes,
            confirmation_attempts,
        }
    }

    pub fn validate(self) -> Result<Self, CdpPageCloseSetupError> {
        if self.native_id_bytes == 0 {
            return Err(CdpPageCloseSetupError::ZeroNativeIdLimit);
        }
        if self.confirmation_attempts == 0 {
            return Err(CdpPageCloseSetupError::ZeroConfirmationAttempts);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdpPageCloseSetupError {
    ZeroNativeIdLimit,
    EmptyTargetId,
    TargetIdTooLong { actual: usize, limit: usize },
    ZeroConfirmationAttempts,
}

impl Display for CdpPageCloseSetupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroNativeIdLimit => {
                formatter.write_str("Page close native-ID limit must be above zero")
            }
            Self::EmptyTargetId => formatter.write_str("CDP target ID must not be empty"),
            Self::TargetIdTooLong { actual, limit } => write!(
                formatter,
                "CDP target ID has {actual} bytes, exceeding limit {limit}"
            ),
            Self::ZeroConfirmationAttempts => {
                formatter.write_str("Page close requires at least one confirmation attempt")
            }
        }
    }
}

impl Error for CdpPageCloseSetupError {}

#[derive(Clone)]
pub struct CdpPageCloseService {
    browser_client: Arc<dyn CdpClient>,
    target_id: Arc<str>,
    confirmation_attempts: usize,
    poll: Arc<dyn CdpPageClosePoll>,
}

impl CdpPageCloseService {
    pub fn new(
        browser_client: Arc<dyn CdpClient>,
        target_id: impl Into<String>,
        limits: CdpPageCloseLimits,
        poll: Arc<dyn CdpPageClosePoll>,
    ) -> Result<Self, CdpPageCloseSetupError> {
        let target_id = target_id.into();
        let limits = limits.validate()?;
        if target_id.is_empty() {
            return Err(CdpPageCloseSetupError::EmptyTargetId);
        }
        if target_id.len() > limits.native_id_bytes {
            return Err(CdpPageCloseSetupError::TargetIdTooLong {
                actual: target_id.len(),
                limit: limits.native_id_bytes,
            });
        }
        Ok(Self {
            browser_client,
            target_id: Arc::from(target_id.into_boxed_str()),
            confirmation_attempts: limits.confirmation_attempts,
            poll,
        })
    }

    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub async fn close_route(
        browser_client: &dyn CdpClient,
        target_id: &str,
        confirmation_attempts: usize,
        poll: &dyn CdpPageClosePoll,
    ) -> Result<(), OperationError<PageLifecycleErrorDetails>> {
        if let Err(error) = browser_client.target_close(target_id).await {
            let error = backend_error("Target.closeTarget", error);
            if error.kind() != FailureKind::TargetClosed {
                return Err(error);
            }
        }

        for completed_checks in 0..confirmation_attempts {
            let targets = browser_client
                .target_get_targets()
                .await
                .map_err(|error| backend_error("Target.getTargets", error))?;
            if targets.iter().all(|target| target.target_id != target_id) {
                return Ok(());
            }
            if completed_checks + 1 < confirmation_attempts {
                poll.wait(completed_checks + 1).await;
            }
        }

        Err(OperationError::new(
            FailureKind::Backend,
            format!("CDP target {target_id} remained open after Target.closeTarget"),
            Some(PageLifecycleErrorDetails::TargetStillOpen(
                target_id.to_string(),
            )),
        ))
    }
}

impl Service<()> for CdpPageCloseService {
    type Response = ();
    type Error = OperationError<PageLifecycleErrorDetails>;
    type Future<'a> = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>;

    fn call(&self, (): ()) -> Self::Future<'_> {
        Box::pin(Self::close_route(
            self.browser_client.as_ref(),
            &self.target_id,
            self.confirmation_attempts,
            self.poll.as_ref(),
        ))
    }
}

#[derive(Clone)]
pub struct CdpPageRunBeforeUnloadService {
    page_client: Arc<dyn CdpClient>,
}

impl CdpPageRunBeforeUnloadService {
    pub const fn new(page_client: Arc<dyn CdpClient>) -> Self {
        Self { page_client }
    }
}

impl Service<()> for CdpPageRunBeforeUnloadService {
    type Response = ();
    type Error = OperationError<PageLifecycleErrorDetails>;
    type Future<'a> = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>;

    fn call(&self, (): ()) -> Self::Future<'_> {
        Box::pin(async move {
            self.page_client
                .page_close()
                .await
                .map_err(|error| backend_error("Page.close", error))
        })
    }
}

fn backend_error(
    method: &'static str,
    error: CdpError,
) -> OperationError<PageLifecycleErrorDetails> {
    let kind = pwright_backend_cdp_common::classify_cdp_failure(&error);
    let message = error.to_string();
    OperationError::new(
        kind,
        format!("CDP {method} failed: {message}"),
        Some(PageLifecycleErrorDetails::Backend(message)),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cdp::connection::CdpError;
    use pwright_cdp::domains::target::TargetInfo;
    use pwright_core::{FailureKind, Service};
    use pwright_fake::FakeCdpClient;

    use super::{
        CdpPageCloseLimits, CdpPageCloseService, CdpPageRunBeforeUnloadService,
        ImmediatePageClosePoll,
    };

    fn target(target_id: &str) -> TargetInfo {
        TargetInfo {
            target_id: target_id.to_string(),
            target_type: "page".to_string(),
            title: String::new(),
            url: "about:blank".to_string(),
            attached: true,
        }
    }

    fn close_service(client: Arc<FakeCdpClient>, attempts: usize) -> CdpPageCloseService {
        CdpPageCloseService::new(
            client,
            "target-a",
            CdpPageCloseLimits::new(64, attempts),
            Arc::new(ImmediatePageClosePoll),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn ordinary_close_confirms_the_exact_target_is_absent() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_target_responses([vec![target("other")]]);
        let service = close_service(client.clone(), 1);

        // Act
        service.call(()).await.unwrap();

        // Assert
        assert_eq!(
            client.method_names(),
            ["Target.closeTarget", "Target.getTargets"]
        );
    }

    #[tokio::test]
    async fn ordinary_close_retries_but_never_commits_on_acknowledgement_alone() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_target_responses([vec![target("target-a")], vec![target("target-a")]]);
        let service = close_service(client.clone(), 2);

        // Act
        let error = service.call(()).await.unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(
            client.method_names(),
            [
                "Target.closeTarget",
                "Target.getTargets",
                "Target.getTargets"
            ]
        );
    }

    #[tokio::test]
    async fn racing_target_closed_error_still_requires_and_accepts_absence_proof() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_target_close_error(Some("No target with given id found".to_string()));
        let service = close_service(client.clone(), 1);

        // Act
        service.call(()).await.unwrap();

        // Assert
        assert_eq!(
            client.method_names(),
            ["Target.closeTarget", "Target.getTargets"]
        );
    }

    #[test]
    fn closed_target_errors_retain_their_portable_failure_kind() {
        // Arrange
        let page_closed = CdpError::PageClosed;
        let target_missing = CdpError::Protocol {
            code: -32602,
            message: "No target with given id found".to_string(),
        };

        // Act
        let page_closed = super::backend_error("Target.closeTarget", page_closed);
        let target_missing = super::backend_error("Target.closeTarget", target_missing);

        // Assert
        assert_eq!(page_closed.kind(), FailureKind::TargetClosed);
        assert_eq!(target_missing.kind(), FailureKind::TargetClosed);
    }

    #[tokio::test]
    async fn before_unload_uses_only_the_page_session_and_does_not_confirm_closure() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let service = CdpPageRunBeforeUnloadService::new(client.clone());

        // Act
        service.call(()).await.unwrap();

        // Assert
        assert_eq!(client.method_names(), ["Page.close"]);
    }

    #[test]
    fn setup_rejects_unbounded_or_malformed_confirmation() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));

        // Act
        let empty = CdpPageCloseService::new(
            client.clone(),
            "",
            CdpPageCloseLimits::new(64, 1),
            Arc::new(ImmediatePageClosePoll),
        );
        let zero = CdpPageCloseService::new(
            client.clone(),
            "target-a",
            CdpPageCloseLimits::new(64, 0),
            Arc::new(ImmediatePageClosePoll),
        );
        let unbounded_native_id = CdpPageCloseService::new(
            client,
            "target-a",
            CdpPageCloseLimits::new(0, 1),
            Arc::new(ImmediatePageClosePoll),
        );

        // Assert
        assert!(empty.is_err());
        assert!(zero.is_err());
        assert!(unbounded_native_id.is_err());
    }
}
