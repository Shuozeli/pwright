//! Minimal Chromium CDP composition for optional read-only DOM content.
//!
//! This leaf binds one pre-existing Frame object to an injected target-local
//! CDP client and a caller-provisioned utility execution context, then exposes
//! the generated `pwright.dom_content` server handler. It does not install
//! public evaluation, screenshots, navigation, browser discovery, browser
//! acquisition, a runtime, a transport, or an executor.

use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub use pwright_backend_cdp_dom_content::{
    CdpDomContentProductionProfile, CdpDomContentService, CdpDomContentServiceFactory,
};
use pwright_cap_dom_content::{DomContentErrorDetails, FrameContent, FrameTitle};
use pwright_cdp::CdpClient;
use pwright_codec_dom_content::DomContentCodec;
pub use pwright_codec_dom_content::DomContentCodecLimits;
use pwright_core::{
    CancellationReason, Command, CommandCancellation, CommandResult, FailureKind, OperationContext,
    OperationError, ResourceError, ResourceLease, ResourceLedger, Service,
};
pub use pwright_core::{
    ObjectId, ProductionResourceError, ProductionResourceLedger, ProtocolVersion, ResourceClass,
    ResourceLimits, ResourceRequirements, ResourceScope,
};
use pwright_server_dom_content::{
    DomContentServerHandlers, DomContentServerServices, dom_content_server_handlers,
};
pub use pwright_transport::FrameLimits;
use pwright_transport::WireProtocol;

/// Wire, decoded-result, backend-operation, and utility-route bounds for one
/// installed DOM-content-only Frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumDomContentProfile {
    backend: CdpDomContentProductionProfile,
    codec: DomContentCodecLimits,
    version: ProtocolVersion,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
}

impl ChromiumDomContentProfile {
    pub const fn new(
        backend: CdpDomContentProductionProfile,
        codec: DomContentCodecLimits,
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

    pub const fn backend(self) -> CdpDomContentProductionProfile {
        self.backend
    }

    pub const fn codec(self) -> DomContentCodecLimits {
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

    /// Capacity for one result decoded through the publicly exposed protocol.
    /// The server path normally encodes backend-owned content instead, but the
    /// protocol never receives an unaccounted private result ledger.
    pub fn codec_result_requirements(self) -> ResourceRequirements {
        let bytes = u64::try_from(self.codec.content_bytes).unwrap_or(u64::MAX);
        if bytes == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::DomContentBytes, bytes, bytes)
                .with_metadata(1, 1)
        }
    }

    /// Complete concurrent peak for backend result ownership, the retained
    /// utility-context route, a possible codec-decoded result, and one encoded
    /// wire frame.
    pub fn requirements(self) -> ResourceRequirements {
        self.backend
            .requirements()
            .combine_concurrent(self.codec_result_requirements())
            .combine_concurrent(self.sizing_protocol().production_resource_requirements())
    }

    fn sizing_protocol(self) -> WireProtocol<DomContentCodec> {
        self.protocol_with_result_ledger(ResourceLedger::new(ResourceLimits::deny_all(0, 0)))
    }

    fn protocol_with_result_ledger(
        self,
        result_ledger: ResourceLedger,
    ) -> WireProtocol<DomContentCodec> {
        WireProtocol::new_exact_directional(
            DomContentCodec::new(self.codec, result_ledger),
            self.version,
            self.send_limits,
            self.receive_limits,
        )
    }
}

/// Why a DOM-content-only Frame could not be installed.
#[derive(Debug)]
pub enum ChromiumDomContentInstallError {
    ZeroContentCapacity {
        max_content_bytes: usize,
        max_concurrent_reads: usize,
    },
    UtilityContextRouteNotBudgeted,
    EmptyUniqueContextId,
    UniqueContextIdTooLarge {
        actual: usize,
        limit: usize,
    },
    Resources(ProductionResourceError),
    RouteResources(ResourceError),
}

impl Display for ChromiumDomContentInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroContentCapacity {
                max_content_bytes,
                max_concurrent_reads,
            } => write!(
                formatter,
                "DOM-content installation requires nonzero result and concurrency capacity, got {max_content_bytes} bytes and {max_concurrent_reads} reads"
            ),
            Self::UtilityContextRouteNotBudgeted => formatter.write_str(
                "DOM-content installation requires a nonzero utility-context route and native-ID budget",
            ),
            Self::EmptyUniqueContextId => {
                formatter.write_str("the Chromium utility-world unique context ID must not be empty")
            }
            Self::UniqueContextIdTooLarge { actual, limit } => write!(
                formatter,
                "the Chromium utility-world unique context ID has {actual} bytes, limit is {limit}"
            ),
            Self::Resources(error) => Display::fmt(error, formatter),
            Self::RouteResources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumDomContentInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resources(error) => Some(error),
            Self::RouteResources(error) => Some(error),
            Self::ZeroContentCapacity { .. }
            | Self::UtilityContextRouteNotBudgeted
            | Self::EmptyUniqueContextId
            | Self::UniqueContextIdTooLarge { .. } => None,
        }
    }
}

impl From<ProductionResourceError> for ChromiumDomContentInstallError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

impl From<ResourceError> for ChromiumDomContentInstallError {
    fn from(error: ResourceError) -> Self {
        Self::RouteResources(error)
    }
}

/// One explicitly routed Frame backed only by the Chromium CDP DOM-content
/// adapter.
pub struct ChromiumDomContentFrame {
    frame_id: ObjectId,
    backend: Arc<CdpDomContentService>,
}

impl ChromiumDomContentFrame {
    pub const fn frame_id(&self) -> ObjectId {
        self.frame_id
    }

    pub fn backend(&self) -> &CdpDomContentService {
        self.backend.as_ref()
    }

    fn wrong_target(&self, actual: ObjectId) -> OperationError<DomContentErrorDetails> {
        OperationError::new(
            FailureKind::ProtocolViolation,
            format!(
                "DOM content targeted Frame `{actual}`, expected `{}`",
                self.frame_id
            ),
            None,
        )
    }
}

macro_rules! impl_dom_read_frame {
    ($command:ty) => {
        impl Service<Command<$command>> for ChromiumDomContentFrame {
            type Response = CommandResult<$command>;
            type Error = Infallible;
            type Future<'a> =
                Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

            fn call(&self, command: Command<$command>) -> Self::Future<'_> {
                let (call_id, object_id, _metadata, params) = command.into_parts();
                let route = (object_id == self.frame_id)
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

        impl CommandCancellation<$command> for ChromiumDomContentFrame {
            type Future<'a> = Pin<
                Box<
                    dyn Future<Output = Result<(), OperationError<DomContentErrorDetails>>>
                        + Send
                        + 'a,
                >,
            >;

            fn cancel(
                &self,
                context: OperationContext,
                _reason: CancellationReason,
            ) -> Self::Future<'_> {
                let route = (context.object_id() == self.frame_id)
                    .then_some(())
                    .ok_or_else(|| self.wrong_target(context.object_id()));
                // CDP has no operation-local cancellation primitive for
                // Runtime.evaluate. Dropping the read releases admission and discards
                // the late serialization without terminating unrelated JavaScript.
                Box::pin(async move { route })
            }
        }
    };
}

impl_dom_read_frame!(FrameContent);
impl_dom_read_frame!(FrameTitle);

pub type ChromiumDomContentHandlers =
    DomContentServerHandlers<ChromiumDomContentFrame, ChromiumDomContentFrame>;

/// A fully bound DOM-content-only server leaf. It is inert until the caller
/// drives the returned handler with its chosen transport and executor.
pub struct ChromiumDomContentInstallation {
    frame: Arc<ChromiumDomContentFrame>,
    protocol: Arc<WireProtocol<DomContentCodec>>,
    _route_resources: ResourceLease,
}

impl ChromiumDomContentInstallation {
    pub fn install(
        frame_id: ObjectId,
        client: Arc<dyn CdpClient>,
        unique_context_id: impl Into<String>,
        profile: ChromiumDomContentProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumDomContentInstallError> {
        let backend_profile = profile.backend();
        if backend_profile.max_content_bytes() == 0 || backend_profile.max_concurrent_reads() == 0 {
            return Err(ChromiumDomContentInstallError::ZeroContentCapacity {
                max_content_bytes: backend_profile.max_content_bytes(),
                max_concurrent_reads: backend_profile.max_concurrent_reads(),
            });
        }
        if backend_profile.max_context_routes() == 0 || backend_profile.max_native_id_bytes() == 0 {
            return Err(ChromiumDomContentInstallError::UtilityContextRouteNotBudgeted);
        }
        let unique_context_id = unique_context_id.into();
        if unique_context_id.is_empty() {
            return Err(ChromiumDomContentInstallError::EmptyUniqueContextId);
        }
        if unique_context_id.len() > backend_profile.max_native_id_bytes() {
            return Err(ChromiumDomContentInstallError::UniqueContextIdTooLarge {
                actual: unique_context_id.len(),
                limit: backend_profile.max_native_id_bytes(),
            });
        }

        let factory =
            CdpDomContentServiceFactory::new_with_production_resources(backend_profile, resources)?;
        let route_resources = factory
            .reserve_context_routes()?
            .ok_or(ChromiumDomContentInstallError::UtilityContextRouteNotBudgeted)?;
        let result_ledger = resources.claim(profile.codec_result_requirements())?;
        let protocol = profile
            .protocol_with_result_ledger(result_ledger)
            .with_production_resources(resources)?;
        let backend = factory
            .bind_shared_unique_context(client, Arc::from(unique_context_id.into_boxed_str()));

        Ok(Self {
            frame: Arc::new(ChromiumDomContentFrame {
                frame_id,
                backend: Arc::new(backend),
            }),
            protocol: Arc::new(protocol),
            _route_resources: route_resources,
        })
    }

    pub fn frame(&self) -> Arc<ChromiumDomContentFrame> {
        Arc::clone(&self.frame)
    }

    pub fn protocol(&self) -> Arc<WireProtocol<DomContentCodec>> {
        Arc::clone(&self.protocol)
    }

    pub fn handlers(&self) -> ChromiumDomContentHandlers {
        dom_content_server_handlers(
            Arc::clone(&self.protocol),
            DomContentServerServices {
                frame_content: Arc::clone(&self.frame),
                frame_title: Arc::clone(&self.frame),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cap_dom_content::{DomContentParams, FrameContent};
    use pwright_core::{
        CallId, CallMetadata, Command, FailureKind, ObjectId, ProductionResourceLedger,
        ProtocolVersion, ResourceClass, ResourceLimits, ResourceScope, Service,
    };
    use pwright_fake::FakeCdpClient;
    use pwright_transport::FrameLimits;
    use serde_json::json;

    use super::{
        CdpDomContentProductionProfile, ChromiumDomContentInstallError,
        ChromiumDomContentInstallation, ChromiumDomContentProfile, DomContentCodecLimits,
    };

    fn profile(with_context_route: bool) -> ChromiumDomContentProfile {
        let mut backend =
            CdpDomContentProductionProfile::new(ResourceScope::Object(ObjectId::new(7)), 1_024, 1);
        if with_context_route {
            backend = backend.with_context_routes(1, 64);
        }
        ChromiumDomContentProfile::new(
            backend,
            DomContentCodecLimits::new(1_024, 256),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(4_096, 1, 1_024, 5_120),
            FrameLimits::new(4_096, 1, 1_024, 5_120),
        )
    }

    fn resources(profile: ChromiumDomContentProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            limits = limits.with_class(class, required.global, required.per_scope);
        }
        ProductionResourceLedger::new(limits, requirements)
            .expect("exact DOM-content resources should build")
    }

    #[tokio::test]
    async fn installed_frame_serializes_dom_in_the_bound_utility_context() {
        // Arrange
        let profile = profile(true);
        let resources = resources(profile);
        let fake = Arc::new(FakeCdpClient::from_html(""));
        fake.set_evaluate_response(json!({"result":{"value":"<html></html>"}}));
        let client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
        let installation = ChromiumDomContentInstallation::install(
            ObjectId::new(7),
            client,
            "utility-7",
            profile,
            &resources,
        )
        .expect("DOM-content-only composition should install");
        assert_eq!(
            resources
                .metrics()
                .expect("route metrics should be readable")
                .class(ResourceClass::BackendStateBytes)
                .current,
            640
        );
        let command = Command::<FrameContent>::new(
            CallId::new(11),
            ObjectId::new(7),
            CallMetadata::default(),
            DomContentParams,
        );

        // Act
        let result = installation
            .frame()
            .call(command)
            .await
            .expect("command service is infallible")
            .into_outcome()
            .expect("CDP DOM serialization should succeed");

        // Assert
        assert_eq!(result.as_str(), "<html></html>");
        assert_eq!(fake.method_names(), ["Runtime.evaluate"]);
        assert_eq!(fake.calls()[0].1[0]["uniqueContextId"], "utility-7");
        assert!(
            fake.calls()[0].1[0]["expression"]
                .as_str()
                .expect("serializer should be a string")
                .contains("XMLSerializer")
        );
        assert_eq!(
            resources
                .metrics()
                .expect("result metrics should be readable")
                .class(ResourceClass::DomContentBytes)
                .current,
            13
        );
        let _handlers = installation.handlers();
        drop(result);
        assert_eq!(
            resources
                .metrics()
                .expect("released result metrics should be readable")
                .class(ResourceClass::DomContentBytes)
                .current,
            0
        );
        drop(installation);
        assert_eq!(
            resources
                .metrics()
                .expect("released route metrics should be readable")
                .class(ResourceClass::BackendStateBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn installed_frame_rejects_an_unbound_object_before_cdp() {
        // Arrange
        let profile = profile(true);
        let resources = resources(profile);
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
        let installation = ChromiumDomContentInstallation::install(
            ObjectId::new(7),
            client,
            "utility-7",
            profile,
            &resources,
        )
        .expect("DOM-content-only composition should install");
        let command = Command::<FrameContent>::new(
            CallId::new(12),
            ObjectId::new(8),
            CallMetadata::default(),
            DomContentParams,
        );

        // Act
        let error = installation
            .frame()
            .call(command)
            .await
            .expect("command service is infallible")
            .into_outcome()
            .expect_err("an unbound Frame must be rejected");

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert!(fake.calls().is_empty());
    }

    #[test]
    fn installation_rejects_an_unbudgeted_utility_route() {
        // Arrange
        let profile = profile(false);
        let resources = resources(profile);
        let client: Arc<dyn pwright_cdp::CdpClient> = Arc::new(FakeCdpClient::from_html(""));

        // Act
        let error = match ChromiumDomContentInstallation::install(
            ObjectId::new(7),
            client,
            "utility-7",
            profile,
            &resources,
        ) {
            Ok(_) => panic!("DOM-content installation must require a route budget"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(
            error,
            ChromiumDomContentInstallError::UtilityContextRouteNotBudgeted
        ));
    }
}
