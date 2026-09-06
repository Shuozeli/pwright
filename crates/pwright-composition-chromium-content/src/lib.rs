//! Minimal Chromium CDP composition for optional Page screenshots.
//!
//! This leaf binds one pre-existing Page object to an injected target-local
//! CDP client, then exposes the generated `pwright.content` server handler. It
//! does not install DOM access, public evaluation, navigation, browser
//! discovery, browser acquisition, a runtime, a transport, or an executor.

use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub use pwright_backend_cdp_content::{
    CdpScreenshotProductionProfile, CdpScreenshotService, CdpScreenshotServiceFactory,
};
use pwright_cap_content::{ContentErrorDetails, PageScreenshot};
use pwright_cdp::CdpClient;
use pwright_codec_content::ContentCodec;
pub use pwright_codec_content::ContentCodecLimits;
use pwright_core::{
    CancellationReason, Command, CommandCancellation, CommandResult, FailureKind, OperationContext,
    OperationError, ResourceLedger, Service,
};
pub use pwright_core::{
    ObjectId, ProductionResourceError, ProductionResourceLedger, ProtocolVersion, ResourceClass,
    ResourceLimits, ResourceRequirements,
};
use pwright_server_content::{
    ContentServerHandlers, ContentServerServices, content_server_handlers,
};
pub use pwright_transport::FrameLimits;
use pwright_transport::WireProtocol;

/// Wire, decoded-result, and backend-capture bounds for one installed
/// screenshot-only Page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumContentProfile {
    backend: CdpScreenshotProductionProfile,
    codec: ContentCodecLimits,
    version: ProtocolVersion,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
}

impl ChromiumContentProfile {
    pub const fn new(
        backend: CdpScreenshotProductionProfile,
        codec: ContentCodecLimits,
        version: ProtocolVersion,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
    ) -> Self {
        Self {
            backend,
            codec,
            version,
            send_limits,
            receive_limits,
        }
    }

    pub const fn backend(self) -> CdpScreenshotProductionProfile {
        self.backend
    }

    pub const fn codec(self) -> ContentCodecLimits {
        self.codec
    }

    pub const fn version(self) -> ProtocolVersion {
        self.version
    }

    pub const fn send_limits(self) -> FrameLimits {
        self.send_limits
    }

    pub const fn receive_limits(self) -> FrameLimits {
        self.receive_limits
    }

    /// Capacity for one screenshot decoded through the public protocol. The
    /// server normally encodes backend-owned bytes, but its codec never owns an
    /// unaccounted private result ledger.
    pub fn codec_result_requirements(self) -> ResourceRequirements {
        let bytes = u64::try_from(self.codec.screenshot_bytes).unwrap_or(u64::MAX);
        if bytes == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::ScreenshotBytes, bytes, bytes)
                .with_metadata(1, 1)
        }
    }

    /// Complete concurrent peak for the backend result, a possible decoded
    /// protocol result, and one encoded wire frame.
    pub fn requirements(self) -> ResourceRequirements {
        self.backend
            .requirements()
            .combine_concurrent(self.codec_result_requirements())
            .combine_concurrent(self.sizing_protocol().production_resource_requirements())
    }

    fn sizing_protocol(self) -> WireProtocol<ContentCodec> {
        self.protocol_with_result_ledger(ResourceLedger::new(ResourceLimits::deny_all(0, 0)))
    }

    fn protocol_with_result_ledger(
        self,
        result_ledger: ResourceLedger,
    ) -> WireProtocol<ContentCodec> {
        WireProtocol::new_exact_directional(
            ContentCodec::new(self.codec, result_ledger),
            self.version,
            self.send_limits,
            self.receive_limits,
        )
    }
}

/// Why a screenshot-only Page could not be installed.
#[derive(Debug)]
pub enum ChromiumContentInstallError {
    ZeroCaptureCapacity {
        max_screenshot_bytes: usize,
        max_concurrent_captures: usize,
    },
    CodecLimitMismatch {
        backend_bytes: usize,
        codec_bytes: usize,
    },
    OutboundAttachmentTooSmall {
        attachment_slots: usize,
        attachment_bytes: usize,
        required_bytes: usize,
    },
    Resources(ProductionResourceError),
}

impl Display for ChromiumContentInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCaptureCapacity {
                max_screenshot_bytes,
                max_concurrent_captures,
            } => write!(
                formatter,
                "Page screenshot installation requires nonzero result and concurrency capacity, got {max_screenshot_bytes} bytes and {max_concurrent_captures} captures"
            ),
            Self::CodecLimitMismatch {
                backend_bytes,
                codec_bytes,
            } => write!(
                formatter,
                "Page screenshot backend limit is {backend_bytes} bytes but codec limit is {codec_bytes} bytes"
            ),
            Self::OutboundAttachmentTooSmall {
                attachment_slots,
                attachment_bytes,
                required_bytes,
            } => write!(
                formatter,
                "Page screenshot requires one outbound attachment of {required_bytes} bytes, profile has {attachment_slots} slot(s) of {attachment_bytes} bytes"
            ),
            Self::Resources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumContentInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resources(error) => Some(error),
            Self::ZeroCaptureCapacity { .. }
            | Self::CodecLimitMismatch { .. }
            | Self::OutboundAttachmentTooSmall { .. } => None,
        }
    }
}

impl From<ProductionResourceError> for ChromiumContentInstallError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

/// One explicitly routed Page backed only by the Chromium CDP screenshot
/// adapter.
pub struct ChromiumContentPage {
    page_id: ObjectId,
    backend: Arc<CdpScreenshotService>,
}

impl ChromiumContentPage {
    pub const fn page_id(&self) -> ObjectId {
        self.page_id
    }

    pub fn backend(&self) -> &CdpScreenshotService {
        self.backend.as_ref()
    }

    fn wrong_target(&self, actual: ObjectId) -> OperationError<ContentErrorDetails> {
        OperationError::new(
            FailureKind::ProtocolViolation,
            format!(
                "Page screenshot targeted Page `{actual}`, expected `{}`",
                self.page_id
            ),
            None,
        )
    }
}

impl Service<Command<PageScreenshot>> for ChromiumContentPage {
    type Response = CommandResult<PageScreenshot>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<PageScreenshot>) -> Self::Future<'_> {
        let (call_id, object_id, _metadata, options) = command.into_parts();
        let route = (object_id == self.page_id)
            .then_some(())
            .ok_or_else(|| self.wrong_target(object_id));
        Box::pin(async move {
            let outcome = match route {
                Ok(()) => self.backend.call(options).await,
                Err(error) => Err(error),
            };
            Ok(CommandResult::new(call_id, outcome))
        })
    }
}

impl CommandCancellation<PageScreenshot> for ChromiumContentPage {
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<(), OperationError<ContentErrorDetails>>> + Send + 'a>>;

    fn cancel(&self, context: OperationContext, reason: CancellationReason) -> Self::Future<'_> {
        let route = (context.object_id() == self.page_id)
            .then_some(())
            .ok_or_else(|| self.wrong_target(context.object_id()));
        Box::pin(async move {
            route?;
            self.backend.cancel(context, reason).await
        })
    }
}

pub type ChromiumContentHandlers = ContentServerHandlers<ChromiumContentPage>;

/// A fully bound screenshot-only server leaf. It remains inert until the
/// caller drives its handler with a chosen transport and executor.
pub struct ChromiumContentInstallation {
    page: Arc<ChromiumContentPage>,
    protocol: Arc<WireProtocol<ContentCodec>>,
}

impl ChromiumContentInstallation {
    pub fn install(
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        profile: ChromiumContentProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumContentInstallError> {
        let backend = profile.backend();
        if backend.max_screenshot_bytes() == 0 || backend.max_concurrent_captures() == 0 {
            return Err(ChromiumContentInstallError::ZeroCaptureCapacity {
                max_screenshot_bytes: backend.max_screenshot_bytes(),
                max_concurrent_captures: backend.max_concurrent_captures(),
            });
        }
        if profile.codec().screenshot_bytes != backend.max_screenshot_bytes() {
            return Err(ChromiumContentInstallError::CodecLimitMismatch {
                backend_bytes: backend.max_screenshot_bytes(),
                codec_bytes: profile.codec().screenshot_bytes,
            });
        }
        let send = profile.send_limits();
        if send.attachments < 1 || send.attachment_bytes < backend.max_screenshot_bytes() {
            return Err(ChromiumContentInstallError::OutboundAttachmentTooSmall {
                attachment_slots: send.attachments,
                attachment_bytes: send.attachment_bytes,
                required_bytes: backend.max_screenshot_bytes(),
            });
        }

        let factory =
            CdpScreenshotServiceFactory::new_with_production_resources(backend, resources)?;
        let result_ledger = resources.claim(profile.codec_result_requirements())?;
        let protocol = profile
            .protocol_with_result_ledger(result_ledger)
            .with_production_resources(resources)?;
        let backend = factory.bind(client, page_id);

        Ok(Self {
            page: Arc::new(ChromiumContentPage {
                page_id,
                backend: Arc::new(backend),
            }),
            protocol: Arc::new(protocol),
        })
    }

    pub fn page(&self) -> Arc<ChromiumContentPage> {
        Arc::clone(&self.page)
    }

    pub fn protocol(&self) -> Arc<WireProtocol<ContentCodec>> {
        Arc::clone(&self.protocol)
    }

    pub fn handlers(&self) -> ChromiumContentHandlers {
        content_server_handlers(
            Arc::clone(&self.protocol),
            ContentServerServices {
                page_screenshot: Arc::clone(&self.page),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cap_content::{PageScreenshot, ScreenshotFormat, ScreenshotOptions};
    use pwright_core::{
        CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId,
        ProductionResourceLedger, ProtocolVersion, ResourceClass, ResourceLimits, Service,
    };
    use pwright_fake::FakeCdpClient;
    use pwright_transport::FrameLimits;

    use super::{
        CdpScreenshotProductionProfile, ChromiumContentInstallError, ChromiumContentInstallation,
        ChromiumContentProfile, ContentCodecLimits,
    };

    fn profile() -> ChromiumContentProfile {
        ChromiumContentProfile::new(
            CdpScreenshotProductionProfile::new(1_024, 1),
            ContentCodecLimits::new(1_024, 256),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(4_096, 1, 1_024, 5_120),
            FrameLimits::new(4_096, 1, 1_024, 5_120),
        )
    }

    fn resources(profile: ChromiumContentProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            limits = limits.with_class(class, required.global, required.per_scope);
        }
        ProductionResourceLedger::new(limits, requirements)
            .expect("exact Page screenshot resources should build")
    }

    #[tokio::test]
    async fn installed_page_captures_and_retains_exact_screenshot_bytes() {
        // Arrange
        let profile = profile();
        let resources = resources(profile);
        let fake = Arc::new(FakeCdpClient::from_html("<main>hello</main>"));
        let client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
        let installation =
            ChromiumContentInstallation::install(ObjectId::new(7), client, profile, &resources)
                .expect("screenshot-only composition should install");
        let command = Command::<PageScreenshot>::new(
            CallId::new(11),
            ObjectId::new(7),
            CallMetadata::default(),
            ScreenshotOptions {
                format: Some(ScreenshotFormat::Png),
                ..ScreenshotOptions::default()
            },
        );

        // Act
        let result = installation
            .page()
            .call(command)
            .await
            .expect("command service is infallible");
        let frame = installation
            .protocol()
            .encode(&result)
            .expect("screenshot result should encode");
        let decoded: CommandResult<PageScreenshot> = installation
            .protocol()
            .decode(frame.clone())
            .expect("screenshot result should decode");

        // Assert
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect("backend result should succeed")
                .as_bytes(),
            b"mock"
        );
        let decoded_screenshot = decoded
            .outcome()
            .as_ref()
            .expect("decoded result should succeed");
        assert_eq!(decoded_screenshot.as_bytes(), b"mock");
        assert_eq!(decoded_screenshot.format(), ScreenshotFormat::Png);
        assert_eq!(fake.method_names(), ["Page.captureScreenshot"]);
        assert_eq!(
            resources
                .metrics()
                .expect("screenshot metrics should be readable")
                .class(ResourceClass::ScreenshotBytes)
                .current,
            8
        );
        assert!(
            resources
                .metrics()
                .expect("wire metrics should be readable")
                .class(ResourceClass::WireBytes)
                .current
                > 0
        );
        let _handlers = installation.handlers();
        drop(decoded);
        assert_eq!(
            resources
                .metrics()
                .expect("screenshot metrics should be readable")
                .class(ResourceClass::ScreenshotBytes)
                .current,
            4
        );
        drop(frame);
        assert_eq!(
            resources
                .metrics()
                .expect("wire metrics should be readable")
                .class(ResourceClass::WireBytes)
                .current,
            0
        );
        drop(result);
        assert_eq!(
            resources
                .metrics()
                .expect("screenshot metrics should be readable")
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn installed_page_rejects_an_unbound_object_before_cdp() {
        // Arrange
        let profile = profile();
        let resources = resources(profile);
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
        let installation =
            ChromiumContentInstallation::install(ObjectId::new(7), client, profile, &resources)
                .expect("screenshot-only composition should install");
        let command = Command::<PageScreenshot>::new(
            CallId::new(12),
            ObjectId::new(8),
            CallMetadata::default(),
            ScreenshotOptions::default(),
        );

        // Act
        let error = installation
            .page()
            .call(command)
            .await
            .expect("command service is infallible")
            .into_outcome()
            .expect_err("wrong Page must fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert!(fake.method_names().is_empty());
    }

    #[test]
    fn installation_rejects_backend_and_codec_byte_drift() {
        // Arrange
        let profile = ChromiumContentProfile::new(
            CdpScreenshotProductionProfile::new(1_024, 1),
            ContentCodecLimits::new(512, 256),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(4_096, 1, 1_024, 5_120),
            FrameLimits::new(4_096, 1, 1_024, 5_120),
        );
        let resources = resources(profile);
        let client: Arc<dyn pwright_cdp::CdpClient> = Arc::new(FakeCdpClient::from_html(""));

        // Act
        let error =
            ChromiumContentInstallation::install(ObjectId::new(7), client, profile, &resources)
                .err()
                .expect("mismatched limits must fail");

        // Assert
        assert!(matches!(
            error,
            ChromiumContentInstallError::CodecLimitMismatch {
                backend_bytes: 1_024,
                codec_bytes: 512
            }
        ));
    }
}
