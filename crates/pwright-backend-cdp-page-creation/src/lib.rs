//! Isolated Chromium CDP Page creation adapter for Pwright.
//!
//! It creates and attaches one about:blank target, validates its target,
//! session, and main-frame identities, and delegates exact-target compensation
//! to the independently installable Page lifecycle adapter.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use pwright_cap_page_creation::PageCreationErrorDetails;
use pwright_cdp::connection::CdpError;
use pwright_cdp::{CdpClient, SessionFactory};
use pwright_core::{FailureKind, OperationError, Service};

use pwright_backend_cdp_page_lifecycle::{
    CdpPageClosePoll, CdpPageCloseService, ImmediatePageClosePoll,
};

const INITIAL_PAGE_URL: &str = "about:blank";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpPageCreationLimits {
    pub native_id_bytes: usize,
    pub cleanup_confirmation_attempts: usize,
}

impl CdpPageCreationLimits {
    pub const fn new(native_id_bytes: usize) -> Self {
        Self {
            native_id_bytes,
            cleanup_confirmation_attempts: 20,
        }
    }

    pub const fn with_cleanup_confirmation_attempts(mut self, attempts: usize) -> Self {
        self.cleanup_confirmation_attempts = attempts;
        self
    }

    pub fn validate(self) -> Result<Self, CdpPageCreationSetupError> {
        if self.native_id_bytes == 0 {
            return Err(CdpPageCreationSetupError::ZeroNativeIdLimit);
        }
        if self.cleanup_confirmation_attempts == 0 {
            return Err(CdpPageCreationSetupError::ZeroCleanupConfirmationAttempts);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdpPageCreationSetupError {
    ZeroNativeIdLimit,
    ZeroCleanupConfirmationAttempts,
}

impl Display for CdpPageCreationSetupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroNativeIdLimit => {
                formatter.write_str("CDP page creation native-ID limit must be above zero")
            }
            Self::ZeroCleanupConfirmationAttempts => formatter
                .write_str("CDP page creation cleanup confirmation attempts must be above zero"),
        }
    }
}

impl Error for CdpPageCreationSetupError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdpPageCreationRequest {
    browser_context_id: Arc<str>,
}

impl CdpPageCreationRequest {
    pub fn new(browser_context_id: impl Into<Arc<str>>) -> Self {
        Self {
            browser_context_id: browser_context_id.into(),
        }
    }

    pub fn browser_context_id(&self) -> &str {
        &self.browser_context_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdpCreatedPage {
    target_id: Arc<str>,
    session_id: Arc<str>,
    main_frame_id: Arc<str>,
}

impl CdpCreatedPage {
    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn main_frame_id(&self) -> &str {
        &self.main_frame_id
    }
}

#[derive(Clone)]
pub struct CdpPageCreationService {
    browser_client: Arc<dyn CdpClient>,
    session_factory: Arc<dyn SessionFactory>,
    limits: CdpPageCreationLimits,
    cleanup_poll: Arc<dyn CdpPageClosePoll>,
}

impl CdpPageCreationService {
    pub fn new(
        browser_client: Arc<dyn CdpClient>,
        session_factory: Arc<dyn SessionFactory>,
        limits: CdpPageCreationLimits,
    ) -> Result<Self, CdpPageCreationSetupError> {
        Self::with_cleanup_poll(
            browser_client,
            session_factory,
            limits,
            Arc::new(ImmediatePageClosePoll),
        )
    }

    pub fn with_cleanup_poll(
        browser_client: Arc<dyn CdpClient>,
        session_factory: Arc<dyn SessionFactory>,
        limits: CdpPageCreationLimits,
        cleanup_poll: Arc<dyn CdpPageClosePoll>,
    ) -> Result<Self, CdpPageCreationSetupError> {
        let limits = limits.validate()?;
        Ok(Self {
            browser_client,
            session_factory,
            limits,
            cleanup_poll,
        })
    }

    pub async fn compensate_created_target(
        &self,
        target_id: &str,
        primary: OperationError<PageCreationErrorDetails>,
    ) -> OperationError<PageCreationErrorDetails> {
        match CdpPageCloseService::close_route(
            self.browser_client.as_ref(),
            target_id,
            self.limits.cleanup_confirmation_attempts,
            self.cleanup_poll.as_ref(),
        )
        .await
        {
            Ok(()) => primary,
            Err(cleanup) => OperationError::new(
                FailureKind::Cleanup,
                "BrowserContext.newPage failed and closing its exact CDP target also failed",
                Some(PageCreationErrorDetails::Cleanup {
                    primary: primary.to_string(),
                    cleanup: cleanup.to_string(),
                }),
            ),
        }
    }
}

impl Service<CdpPageCreationRequest> for CdpPageCreationService {
    type Response = CdpCreatedPage;
    type Error = OperationError<PageCreationErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, request: CdpPageCreationRequest) -> Self::Future<'_> {
        Box::pin(async move {
            let target_id = self
                .browser_client
                .target_create_in_context(INITIAL_PAGE_URL, request.browser_context_id())
                .await
                .map_err(|error| map_backend_error("Target.createTarget", error))?;
            if let Err(primary) = validate_native_id("CDP targetId", &target_id, self.limits) {
                return Err(self.compensate_created_target(&target_id, primary).await);
            }

            let session_id = match self.browser_client.target_attach(&target_id).await {
                Ok(session_id) => session_id,
                Err(error) => {
                    let primary = map_backend_error("Target.attachToTarget", error);
                    return Err(self.compensate_created_target(&target_id, primary).await);
                }
            };
            if let Err(primary) = validate_native_id("CDP sessionId", &session_id, self.limits) {
                return Err(self.compensate_created_target(&target_id, primary).await);
            }

            let page_client = self
                .session_factory
                .create_session(session_id.clone(), target_id.clone());
            let main_frame_id = match page_client.page_get_main_frame_id().await {
                Ok(frame_id) => frame_id,
                Err(error) => {
                    let primary = map_backend_error("Page.getFrameTree", error);
                    return Err(self.compensate_created_target(&target_id, primary).await);
                }
            };
            if let Err(primary) =
                validate_native_id("CDP main frameId", &main_frame_id, self.limits)
            {
                return Err(self.compensate_created_target(&target_id, primary).await);
            }

            Ok(CdpCreatedPage {
                target_id: Arc::from(target_id.into_boxed_str()),
                session_id: Arc::from(session_id.into_boxed_str()),
                main_frame_id: Arc::from(main_frame_id.into_boxed_str()),
            })
        })
    }
}

fn validate_native_id(
    field: &'static str,
    value: &str,
    limits: CdpPageCreationLimits,
) -> Result<(), OperationError<PageCreationErrorDetails>> {
    let message = if value.is_empty() {
        Some(format!("{field} is empty"))
    } else if value.len() > limits.native_id_bytes {
        Some(format!(
            "{field} has {} bytes, exceeding limit {}",
            value.len(),
            limits.native_id_bytes
        ))
    } else {
        None
    };
    match message {
        Some(message) => Err(OperationError::new(
            FailureKind::ProtocolViolation,
            message.clone(),
            Some(PageCreationErrorDetails::InvalidBackendResponse(message)),
        )),
        None => Ok(()),
    }
}

fn map_backend_error(
    method: &'static str,
    error: CdpError,
) -> OperationError<PageCreationErrorDetails> {
    let kind = pwright_backend_cdp_common::classify_cdp_failure(&error);
    let details = error.to_string();
    OperationError::new(
        kind,
        format!("CDP {method} failed: {details}"),
        Some(PageCreationErrorDetails::Backend(details)),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cdp::{CdpClient, SessionFactory};
    use pwright_core::{FailureKind, Service};
    use pwright_fake::FakeCdpClient;

    use super::{CdpPageCreationLimits, CdpPageCreationRequest, CdpPageCreationService};

    struct FakeFactory {
        client: Arc<FakeCdpClient>,
    }

    impl SessionFactory for FakeFactory {
        fn create_session(&self, _session_id: String, _target_id: String) -> Arc<dyn CdpClient> {
            self.client.clone()
        }
    }

    #[tokio::test]
    async fn creation_uses_about_blank_context_attach_and_frame_tree_in_order() {
        // Arrange
        let browser = Arc::new(FakeCdpClient::from_html(""));
        let page = Arc::new(FakeCdpClient::from_html(""));
        page.set_main_frame_id("frame-7");
        let service = CdpPageCreationService::new(
            browser.clone(),
            Arc::new(FakeFactory {
                client: page.clone(),
            }),
            CdpPageCreationLimits::new(64),
        )
        .expect("valid limits should build");

        // Act
        let created = service
            .call(CdpPageCreationRequest::new("context-2"))
            .await
            .expect("backend creation should succeed");

        // Assert
        assert_eq!(created.target_id(), "fake-target-1");
        assert_eq!(created.session_id(), "fake-session-1");
        assert_eq!(created.main_frame_id(), "frame-7");
        assert_eq!(
            browser.method_names(),
            ["Target.createTarget", "Target.attachToTarget"]
        );
        assert_eq!(page.method_names(), ["Page.getFrameTree"]);
        assert_eq!(
            browser.calls()[0].1,
            [serde_json::json!({
                "url": "about:blank",
                "browserContextId": "context-2",
            })]
        );
    }

    #[tokio::test]
    async fn invalid_main_frame_compensates_the_exact_created_target() {
        // Arrange
        let browser = Arc::new(FakeCdpClient::from_html(""));
        let page = Arc::new(FakeCdpClient::from_html(""));
        page.set_main_frame_id("");
        let service = CdpPageCreationService::new(
            browser.clone(),
            Arc::new(FakeFactory { client: page }),
            CdpPageCreationLimits::new(64),
        )
        .expect("valid limits should build");

        // Act
        let error = service
            .call(CdpPageCreationRequest::new("context-2"))
            .await
            .expect_err("empty frame identity must fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(
            browser.method_names(),
            [
                "Target.createTarget",
                "Target.attachToTarget",
                "Target.closeTarget",
                "Target.getTargets"
            ]
        );
    }
}
