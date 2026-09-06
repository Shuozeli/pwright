//! Minimal Chromium CDP composition for optional Locator screenshots.
//!
//! This leaf owns capture, codec, routing, and wire bounds. Selector-to-clip
//! resolution is an injected plugin: installing this crate does not install
//! the public Locator capability or its command backend.

use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub use pwright_backend_cdp_content::{
    CdpScreenshotProductionProfile, CdpScreenshotServiceFactory,
};
pub use pwright_backend_cdp_locator_screenshot::{
    CdpFrameClipTranslator, CdpLocatorScreenshotService, LocatorScreenshotGeometry,
    LocatorScreenshotGeometryResolver,
};
use pwright_cap_locator_screenshot::{LocatorScreenshot, LocatorScreenshotErrorDetails};
use pwright_cdp::CdpClient;
use pwright_codec_locator_screenshot::LocatorScreenshotCodec;
pub use pwright_codec_locator_screenshot::LocatorScreenshotCodecLimits;
use pwright_core::{
    CancellationReason, Command, CommandCancellation, CommandResult, FailureKind, OperationContext,
    OperationError, ResourceLedger, Service,
};
pub use pwright_core::{
    ObjectId, ProductionResourceError, ProductionResourceLedger, ProtocolVersion, ResourceClass,
    ResourceLimits, ResourceRequirements, ResourceScope,
};
use pwright_server_locator_screenshot::{
    LocatorScreenshotServerHandlers, LocatorScreenshotServerServices,
    locator_screenshot_server_handlers,
};
pub use pwright_transport::FrameLimits;
use pwright_transport::WireProtocol;

/// Capture, codec, and wire bounds for one Locator-screenshot target.
///
/// A selector plugin declares and claims its own resources separately. The
/// application combines those requirements concurrently with this profile
/// before constructing the shared production ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumLocatorScreenshotProfile {
    capture: CdpScreenshotProductionProfile,
    codec: LocatorScreenshotCodecLimits,
    version: ProtocolVersion,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
}

impl ChromiumLocatorScreenshotProfile {
    pub const fn new(
        capture: CdpScreenshotProductionProfile,
        codec: LocatorScreenshotCodecLimits,
        version: ProtocolVersion,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
    ) -> Self {
        Self {
            capture,
            codec,
            version,
            send_limits,
            receive_limits,
        }
    }

    pub const fn capture(self) -> CdpScreenshotProductionProfile {
        self.capture
    }

    pub const fn codec(self) -> LocatorScreenshotCodecLimits {
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

    pub fn requirements(self) -> ResourceRequirements {
        self.capture
            .requirements()
            .combine_concurrent(self.codec_result_requirements())
            .combine_concurrent(self.sizing_protocol().production_resource_requirements())
    }

    fn sizing_protocol(self) -> WireProtocol<LocatorScreenshotCodec> {
        self.protocol_with_result_ledger(ResourceLedger::new(ResourceLimits::deny_all(0, 0)))
    }

    fn protocol_with_result_ledger(
        self,
        result_ledger: ResourceLedger,
    ) -> WireProtocol<LocatorScreenshotCodec> {
        WireProtocol::new_exact_directional(
            LocatorScreenshotCodec::new(self.codec, result_ledger),
            self.version,
            self.send_limits,
            self.receive_limits,
        )
    }
}

#[derive(Debug)]
pub enum ChromiumLocatorScreenshotInstallError {
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

impl Display for ChromiumLocatorScreenshotInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCaptureCapacity {
                max_screenshot_bytes,
                max_concurrent_captures,
            } => write!(
                formatter,
                "Locator screenshot installation requires nonzero result and concurrency capacity, got {max_screenshot_bytes} bytes and {max_concurrent_captures} captures"
            ),
            Self::CodecLimitMismatch {
                backend_bytes,
                codec_bytes,
            } => write!(
                formatter,
                "Locator screenshot backend limit is {backend_bytes} bytes but codec limit is {codec_bytes} bytes"
            ),
            Self::OutboundAttachmentTooSmall {
                attachment_slots,
                attachment_bytes,
                required_bytes,
            } => write!(
                formatter,
                "Locator screenshot requires one outbound attachment of {required_bytes} bytes, profile has {attachment_slots} slot(s) of {attachment_bytes} bytes"
            ),
            Self::Resources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumLocatorScreenshotInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resources(error) => Some(error),
            Self::ZeroCaptureCapacity { .. }
            | Self::CodecLimitMismatch { .. }
            | Self::OutboundAttachmentTooSmall { .. } => None,
        }
    }
}

impl From<ProductionResourceError> for ChromiumLocatorScreenshotInstallError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

pub struct ChromiumLocatorScreenshotTarget {
    object_id: ObjectId,
    backend: Arc<CdpLocatorScreenshotService>,
}

impl ChromiumLocatorScreenshotTarget {
    pub const fn object_id(&self) -> ObjectId {
        self.object_id
    }

    pub fn backend(&self) -> &CdpLocatorScreenshotService {
        self.backend.as_ref()
    }

    fn wrong_target(&self, actual: ObjectId) -> OperationError<LocatorScreenshotErrorDetails> {
        OperationError::new(
            FailureKind::ProtocolViolation,
            format!(
                "Locator screenshot targeted object `{actual}`, expected `{}`",
                self.object_id
            ),
            None,
        )
    }
}

impl Service<Command<LocatorScreenshot>> for ChromiumLocatorScreenshotTarget {
    type Response = CommandResult<LocatorScreenshot>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<LocatorScreenshot>) -> Self::Future<'_> {
        let (call_id, object_id, _metadata, params) = command.into_parts();
        let route = (object_id == self.object_id)
            .then_some(())
            .ok_or_else(|| self.wrong_target(object_id));
        Box::pin(async move {
            let outcome = match route {
                Ok(()) => self.backend.call(params).await,
                Err(error) => Err(error),
            };
            Ok(CommandResult::new(call_id, outcome))
        })
    }
}

impl CommandCancellation<LocatorScreenshot> for ChromiumLocatorScreenshotTarget {
    type Future<'a> = Pin<
        Box<
            dyn Future<Output = Result<(), OperationError<LocatorScreenshotErrorDetails>>>
                + Send
                + 'a,
        >,
    >;

    fn cancel(&self, context: OperationContext, reason: CancellationReason) -> Self::Future<'_> {
        let route = (context.object_id() == self.object_id)
            .then_some(())
            .ok_or_else(|| self.wrong_target(context.object_id()));
        Box::pin(async move {
            route?;
            self.backend.cancel(context, reason).await
        })
    }
}

pub type ChromiumLocatorScreenshotHandlers =
    LocatorScreenshotServerHandlers<ChromiumLocatorScreenshotTarget>;

pub struct ChromiumLocatorScreenshotInstallation {
    target: Arc<ChromiumLocatorScreenshotTarget>,
    protocol: Arc<WireProtocol<LocatorScreenshotCodec>>,
}

impl ChromiumLocatorScreenshotInstallation {
    pub fn install(
        object_id: ObjectId,
        capture_client: Arc<dyn CdpClient>,
        resolver: Arc<dyn LocatorScreenshotGeometryResolver>,
        clip_translator: CdpFrameClipTranslator,
        profile: ChromiumLocatorScreenshotProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumLocatorScreenshotInstallError> {
        let capture = profile.capture();
        if capture.max_screenshot_bytes() == 0 || capture.max_concurrent_captures() == 0 {
            return Err(ChromiumLocatorScreenshotInstallError::ZeroCaptureCapacity {
                max_screenshot_bytes: capture.max_screenshot_bytes(),
                max_concurrent_captures: capture.max_concurrent_captures(),
            });
        }
        if profile.codec().screenshot_bytes != capture.max_screenshot_bytes() {
            return Err(ChromiumLocatorScreenshotInstallError::CodecLimitMismatch {
                backend_bytes: capture.max_screenshot_bytes(),
                codec_bytes: profile.codec().screenshot_bytes,
            });
        }
        let send = profile.send_limits();
        if send.attachments < 1 || send.attachment_bytes < capture.max_screenshot_bytes() {
            return Err(
                ChromiumLocatorScreenshotInstallError::OutboundAttachmentTooSmall {
                    attachment_slots: send.attachments,
                    attachment_bytes: send.attachment_bytes,
                    required_bytes: capture.max_screenshot_bytes(),
                },
            );
        }

        let factory =
            CdpScreenshotServiceFactory::new_with_production_resources(capture, resources)?;
        let result_ledger = resources.claim(profile.codec_result_requirements())?;
        let protocol = profile
            .protocol_with_result_ledger(result_ledger)
            .with_production_resources(resources)?;
        let screenshot = factory.bind(capture_client, object_id);
        let backend = CdpLocatorScreenshotService::new(resolver, screenshot)
            .with_clip_translator(clip_translator);

        Ok(Self {
            target: Arc::new(ChromiumLocatorScreenshotTarget {
                object_id,
                backend: Arc::new(backend),
            }),
            protocol: Arc::new(protocol),
        })
    }

    pub fn target(&self) -> Arc<ChromiumLocatorScreenshotTarget> {
        Arc::clone(&self.target)
    }

    pub fn protocol(&self) -> Arc<WireProtocol<LocatorScreenshotCodec>> {
        Arc::clone(&self.protocol)
    }

    pub fn handlers(&self) -> ChromiumLocatorScreenshotHandlers {
        locator_screenshot_server_handlers(
            Arc::clone(&self.protocol),
            LocatorScreenshotServerServices {
                locator_screenshot: Arc::clone(&self.target),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    use pwright_cap_locator_screenshot::{
        LocatorScreenshot, LocatorScreenshotErrorDetails, LocatorScreenshotParams,
        ScreenshotOptions, Selector,
    };
    use pwright_core::{
        CallId, CallMetadata, Command, CommandResult, FailureKind, OperationError, Service,
    };
    use pwright_fake::FakeCdpClient;

    use super::{
        CdpFrameClipTranslator, CdpScreenshotProductionProfile,
        ChromiumLocatorScreenshotInstallation, ChromiumLocatorScreenshotProfile, FrameLimits,
        LocatorScreenshotCodecLimits, LocatorScreenshotGeometry, LocatorScreenshotGeometryResolver,
        ObjectId, ProductionResourceLedger, ProtocolVersion, ResourceClass, ResourceLimits,
    };

    struct FixedResolver;

    impl LocatorScreenshotGeometryResolver for FixedResolver {
        fn resolve(
            &self,
            _selector: Selector,
        ) -> Pin<
            Box<
                dyn Future<
                        Output = Result<
                            LocatorScreenshotGeometry,
                            OperationError<LocatorScreenshotErrorDetails>,
                        >,
                    > + Send
                    + '_,
            >,
        > {
            Box::pin(async {
                Ok(LocatorScreenshotGeometry::new(10.0, 20.0, 30.0, 40.0)
                    .expect("fixed geometry should validate"))
            })
        }
    }

    fn profile() -> ChromiumLocatorScreenshotProfile {
        ChromiumLocatorScreenshotProfile::new(
            CdpScreenshotProductionProfile::new(1_024, 1),
            LocatorScreenshotCodecLimits::new(1_024, 256, 32, 64),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(4_096, 1, 1_024, 5_120),
            FrameLimits::new(4_096, 1, 1_024, 5_120),
        )
    }

    fn resources(profile: ChromiumLocatorScreenshotProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            limits = limits.with_class(class, required.global, required.per_scope);
        }
        ProductionResourceLedger::new(limits, requirements)
            .expect("exact Locator screenshot resources should build")
    }

    #[tokio::test]
    async fn injected_resolver_captures_and_crosses_only_locator_screenshot_wire() {
        // Arrange
        let profile = profile();
        let resources = resources(profile);
        let fake = Arc::new(FakeCdpClient::from_html("<button>Save</button>"));
        let capture_client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
        let installation = ChromiumLocatorScreenshotInstallation::install(
            ObjectId::new(7),
            capture_client,
            Arc::new(FixedResolver),
            CdpFrameClipTranslator::default(),
            profile,
            &resources,
        )
        .expect("pluggable Locator screenshot composition should install");
        let command = Command::<LocatorScreenshot>::new(
            CallId::new(11),
            ObjectId::new(7),
            CallMetadata::default(),
            LocatorScreenshotParams::new(
                Selector::css("button").expect("selector should validate"),
                ScreenshotOptions::default(),
            ),
        );

        // Act
        let result = installation
            .target()
            .call(command)
            .await
            .expect("command service is infallible");
        let frame = installation
            .protocol()
            .encode(&result)
            .expect("Locator screenshot result should encode");
        let decoded: CommandResult<LocatorScreenshot> = installation
            .protocol()
            .decode(frame.clone())
            .expect("Locator screenshot result should decode");

        // Assert
        assert_eq!(
            decoded
                .outcome()
                .as_ref()
                .expect("capture should succeed")
                .as_bytes(),
            b"mock"
        );
        let capture = fake
            .calls()
            .into_iter()
            .find(|(method, _)| method == "Page.captureScreenshot")
            .expect("capture should be dispatched");
        assert_eq!(capture.1[0]["clip"]["x"], 10.0);
        assert_eq!(capture.1[0]["clip"]["height"], 40.0);
        let _handlers = installation.handlers();
        drop(decoded);
        drop(frame);
        drop(result);
        let metrics = resources.metrics().expect("metrics should be readable");
        assert_eq!(metrics.class(ResourceClass::ScreenshotBytes).current, 0);
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 0);
    }

    #[tokio::test]
    async fn target_rejects_an_unbound_object_before_resolver_or_cdp() {
        // Arrange
        let profile = profile();
        let resources = resources(profile);
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let capture_client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
        let installation = ChromiumLocatorScreenshotInstallation::install(
            ObjectId::new(7),
            capture_client,
            Arc::new(FixedResolver),
            CdpFrameClipTranslator::default(),
            profile,
            &resources,
        )
        .expect("composition should install");
        let command = Command::<LocatorScreenshot>::new(
            CallId::new(12),
            ObjectId::new(8),
            CallMetadata::default(),
            LocatorScreenshotParams::new(
                Selector::css("button").expect("selector should validate"),
                ScreenshotOptions::default(),
            ),
        );

        // Act
        let error = installation
            .target()
            .call(command)
            .await
            .expect("command service is infallible")
            .into_outcome()
            .expect_err("wrong object must fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert!(fake.method_names().is_empty());
    }
}
