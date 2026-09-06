//! Minimal Chromium CDP composition for optional value-only evaluation.
//!
//! This leaf binds one pre-existing Frame object to an injected target-local
//! CDP client and exposes the generated `pwright.evaluation` server handler.
//! It does not discover, download, launch, or own Chromium; create Handles;
//! install navigation; or select a transport, runtime, or executor.

use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub use pwright_backend_cdp_evaluation::{
    CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpEvaluationService,
    CdpEvaluationServiceFactory,
};
pub use pwright_cap_evaluation::EvaluationValueLimits;
use pwright_cap_evaluation::{EvaluateValue, EvaluationErrorDetails};
use pwright_cdp::CdpClient;
use pwright_codec_evaluation::EvaluationCodec;
pub use pwright_codec_evaluation::EvaluationCodecLimits;
use pwright_core::{
    CancellationReason, Command, CommandCancellation, CommandResult, FailureKind, OperationContext,
    OperationError, ResourceError, ResourceLease, Service,
};
pub use pwright_core::{
    ObjectId, ProductionResourceError, ProductionResourceLedger, ProtocolVersion, ResourceLimits,
    ResourceRequirements, ResourceScope,
};
use pwright_server_evaluation::{
    EvaluationServerHandlers, EvaluationServerServices, evaluation_server_handlers,
};
pub use pwright_transport::FrameLimits;
use pwright_transport::WireProtocol;

/// Wire and backend bounds for one installed evaluation-only Frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumEvaluationProfile {
    backend: CdpEvaluationProductionProfile,
    codec: EvaluationCodecLimits,
    version: ProtocolVersion,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
}

impl ChromiumEvaluationProfile {
    pub const fn new(
        backend: CdpEvaluationProductionProfile,
        codec: EvaluationCodecLimits,
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

    pub const fn backend(self) -> CdpEvaluationProductionProfile {
        self.backend
    }

    pub const fn codec(self) -> EvaluationCodecLimits {
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

    /// Minimum shared-ledger declaration for the backend operation, an
    /// optional retained execution-context route, and one simultaneously
    /// encoded evaluation frame.
    pub fn requirements(self) -> ResourceRequirements {
        self.backend.requirements().combine_concurrent(
            self.unbudgeted_protocol()
                .production_resource_requirements(),
        )
    }

    fn unbudgeted_protocol(self) -> WireProtocol<EvaluationCodec> {
        WireProtocol::new_exact_directional(
            EvaluationCodec::new(self.codec),
            self.version,
            self.send_limits,
            self.receive_limits,
        )
    }
}

/// Why an evaluation-only Frame could not be installed.
#[derive(Debug)]
pub enum ChromiumEvaluationInstallError {
    MainFrameProfileHasContextRoutes {
        max_context_routes: usize,
        max_native_id_bytes: usize,
    },
    UniqueContextRouteNotBudgeted,
    EmptyUniqueContextId,
    UniqueContextIdTooLarge {
        actual: usize,
        limit: usize,
    },
    Resources(ProductionResourceError),
    RouteResources(ResourceError),
}

impl Display for ChromiumEvaluationInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MainFrameProfileHasContextRoutes {
                max_context_routes,
                max_native_id_bytes,
            } => write!(
                formatter,
                "main-Frame evaluation does not retain an execution-context route, but the profile declares {max_context_routes} routes with {max_native_id_bytes} native-ID bytes"
            ),
            Self::UniqueContextRouteNotBudgeted => formatter.write_str(
                "unique-context evaluation requires a nonzero context-route and native-ID budget",
            ),
            Self::EmptyUniqueContextId => {
                formatter.write_str("the Chromium unique execution-context ID must not be empty")
            }
            Self::UniqueContextIdTooLarge { actual, limit } => write!(
                formatter,
                "the Chromium unique execution-context ID has {actual} bytes, limit is {limit}"
            ),
            Self::Resources(error) => Display::fmt(error, formatter),
            Self::RouteResources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumEvaluationInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resources(error) => Some(error),
            Self::RouteResources(error) => Some(error),
            Self::MainFrameProfileHasContextRoutes { .. }
            | Self::UniqueContextRouteNotBudgeted
            | Self::EmptyUniqueContextId
            | Self::UniqueContextIdTooLarge { .. } => None,
        }
    }
}

impl From<ProductionResourceError> for ChromiumEvaluationInstallError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

impl From<ResourceError> for ChromiumEvaluationInstallError {
    fn from(error: ResourceError) -> Self {
        Self::RouteResources(error)
    }
}

/// One explicitly routed Frame backed only by the Chromium CDP value
/// evaluation adapter.
pub struct ChromiumEvaluationFrame {
    frame_id: ObjectId,
    backend: Arc<CdpEvaluationService>,
}

impl ChromiumEvaluationFrame {
    pub const fn frame_id(&self) -> ObjectId {
        self.frame_id
    }

    pub fn backend(&self) -> &CdpEvaluationService {
        self.backend.as_ref()
    }

    fn wrong_target(&self, actual: ObjectId) -> OperationError<EvaluationErrorDetails> {
        OperationError::new(
            FailureKind::ProtocolViolation,
            format!(
                "evaluation targeted Frame `{actual}`, expected `{}`",
                self.frame_id
            ),
            None,
        )
    }
}

impl Service<Command<EvaluateValue>> for ChromiumEvaluationFrame {
    type Response = CommandResult<EvaluateValue>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<EvaluateValue>) -> Self::Future<'_> {
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

impl CommandCancellation<EvaluateValue> for ChromiumEvaluationFrame {
    type Future<'a> = Pin<
        Box<dyn Future<Output = Result<(), OperationError<EvaluationErrorDetails>>> + Send + 'a>,
    >;

    fn cancel(&self, context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        let route = (context.object_id() == self.frame_id)
            .then_some(())
            .ok_or_else(|| self.wrong_target(context.object_id()));
        // CDP has no operation-local cancellation primitive for
        // Runtime.evaluate. Dropping the call rejects its portable result
        // without terminating unrelated JavaScript in the renderer.
        Box::pin(async move { route })
    }
}

pub type ChromiumEvaluationHandlers = EvaluationServerHandlers<ChromiumEvaluationFrame>;

/// A fully bound evaluation-only server leaf. It is inert until the caller
/// drives the returned handler with its chosen transport and executor.
pub struct ChromiumEvaluationInstallation {
    frame: Arc<ChromiumEvaluationFrame>,
    protocol: Arc<WireProtocol<EvaluationCodec>>,
    _route_resources: Option<ResourceLease>,
}

impl ChromiumEvaluationInstallation {
    /// Installs the main Frame, which uses the target-local CDP session's
    /// default execution context and therefore must not declare a retained
    /// context route.
    pub fn install_main_frame(
        frame_id: ObjectId,
        client: Arc<dyn CdpClient>,
        profile: ChromiumEvaluationProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumEvaluationInstallError> {
        let backend_profile = profile.backend();
        if backend_profile.max_context_routes() != 0 || backend_profile.max_native_id_bytes() != 0 {
            return Err(
                ChromiumEvaluationInstallError::MainFrameProfileHasContextRoutes {
                    max_context_routes: backend_profile.max_context_routes(),
                    max_native_id_bytes: backend_profile.max_native_id_bytes(),
                },
            );
        }
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(backend_profile, resources)?;
        Self::finish(frame_id, factory.bind(client), profile, resources, None)
    }

    /// Installs a Frame whose execution world is selected by CDP's stable
    /// `uniqueContextId`. The profile must explicitly budget retained route
    /// state and the complete declared route reservation is held for this
    /// installation's lifetime.
    pub fn install_unique_context(
        frame_id: ObjectId,
        client: Arc<dyn CdpClient>,
        unique_context_id: impl Into<String>,
        profile: ChromiumEvaluationProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumEvaluationInstallError> {
        let backend_profile = profile.backend();
        if backend_profile.max_context_routes() == 0 || backend_profile.max_native_id_bytes() == 0 {
            return Err(ChromiumEvaluationInstallError::UniqueContextRouteNotBudgeted);
        }
        let unique_context_id = unique_context_id.into();
        if unique_context_id.is_empty() {
            return Err(ChromiumEvaluationInstallError::EmptyUniqueContextId);
        }
        if unique_context_id.len() > backend_profile.max_native_id_bytes() {
            return Err(ChromiumEvaluationInstallError::UniqueContextIdTooLarge {
                actual: unique_context_id.len(),
                limit: backend_profile.max_native_id_bytes(),
            });
        }
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(backend_profile, resources)?;
        let route_resources = factory
            .reserve_context_routes()?
            .ok_or(ChromiumEvaluationInstallError::UniqueContextRouteNotBudgeted)?;
        Self::finish(
            frame_id,
            factory.bind_unique_context(client, unique_context_id),
            profile,
            resources,
            Some(route_resources),
        )
    }

    fn finish(
        frame_id: ObjectId,
        backend: CdpEvaluationService,
        profile: ChromiumEvaluationProfile,
        resources: &ProductionResourceLedger,
        route_resources: Option<ResourceLease>,
    ) -> Result<Self, ChromiumEvaluationInstallError> {
        let protocol = profile
            .unbudgeted_protocol()
            .with_production_resources(resources)?;
        Ok(Self {
            frame: Arc::new(ChromiumEvaluationFrame {
                frame_id,
                backend: Arc::new(backend),
            }),
            protocol: Arc::new(protocol),
            _route_resources: route_resources,
        })
    }

    pub fn frame(&self) -> Arc<ChromiumEvaluationFrame> {
        Arc::clone(&self.frame)
    }

    pub fn protocol(&self) -> Arc<WireProtocol<EvaluationCodec>> {
        Arc::clone(&self.protocol)
    }

    pub fn handlers(&self) -> ChromiumEvaluationHandlers {
        evaluation_server_handlers(
            Arc::clone(&self.protocol),
            EvaluationServerServices {
                evaluate_value: Arc::clone(&self.frame),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cap_evaluation::{
        EvaluateParams, EvaluateValue, EvaluationValue, EvaluationValueLimits,
    };
    use pwright_core::{
        CallId, CallMetadata, Command, FailureKind, ObjectId, ProductionResourceLedger,
        ProtocolVersion, ResourceClass, ResourceLimits, ResourceScope, Service,
    };
    use pwright_fake::FakeCdpClient;
    use pwright_transport::FrameLimits;
    use serde_json::json;

    use super::{
        CdpEvaluationLimits, CdpEvaluationProductionProfile, ChromiumEvaluationInstallError,
        ChromiumEvaluationInstallation, ChromiumEvaluationProfile, EvaluationCodecLimits,
    };

    fn profile(with_context_route: bool) -> ChromiumEvaluationProfile {
        let values = EvaluationValueLimits::new(4, 16, 16, 256, 256)
            .expect("small evaluation values should be valid");
        let mut backend = CdpEvaluationProductionProfile::new(
            ResourceScope::Object(ObjectId::new(7)),
            CdpEvaluationLimits::new(256, 32 * 1024, 128, values),
            1,
        );
        if with_context_route {
            backend = backend.with_context_routes(1, 64);
        }
        ChromiumEvaluationProfile::new(
            backend,
            EvaluationCodecLimits::new(256, 128, values)
                .expect("evaluation codec limits should be valid"),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(4_096, 0, 0, 4_096),
            FrameLimits::new(4_096, 0, 0, 4_096),
        )
    }

    fn resources(profile: ChromiumEvaluationProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let backend = requirements.class(ResourceClass::BackendStateBytes);
        let wire = requirements.class(ResourceClass::WireBytes);
        let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases)
            .with_class(
                ResourceClass::BackendStateBytes,
                backend.global,
                backend.per_scope,
            )
            .with_class(ResourceClass::WireBytes, wire.global, wire.per_scope);
        ProductionResourceLedger::new(limits, requirements)
            .expect("exact evaluation resources should build")
    }

    #[tokio::test]
    async fn installed_main_frame_evaluates_through_the_cdp_adapter() {
        // Arrange
        let profile = profile(false);
        let resources = resources(profile);
        let fake = Arc::new(FakeCdpClient::from_html(""));
        fake.set_evaluate_response(
            json!({"result":{"value":{"ok":true,"value":{"t":"str","v":"ready"}}}}),
        );
        let client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
        let installation = ChromiumEvaluationInstallation::install_main_frame(
            ObjectId::new(7),
            client,
            profile,
            &resources,
        )
        .expect("evaluation-only composition should install");
        let command = Command::<EvaluateValue>::new(
            CallId::new(11),
            ObjectId::new(7),
            CallMetadata::default(),
            EvaluateParams::expression("document.readyState"),
        );

        // Act
        let result = installation
            .frame()
            .call(command)
            .await
            .expect("command service is infallible")
            .into_outcome()
            .expect("CDP evaluation should succeed");

        // Assert
        assert_eq!(result.value, EvaluationValue::String("ready".to_owned()));
        assert_eq!(fake.method_names(), ["Runtime.evaluate"]);
        let _handlers = installation.handlers();
    }

    #[tokio::test]
    async fn installed_unique_context_uses_the_budgeted_frame_route() {
        // Arrange
        let profile = profile(true);
        let resources = resources(profile);
        let fake = Arc::new(FakeCdpClient::from_html(""));
        fake.set_evaluate_response(
            json!({"result":{"value":{"ok":true,"value":{"t":"bool","v":true}}}}),
        );
        let client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
        let installation = ChromiumEvaluationInstallation::install_unique_context(
            ObjectId::new(7),
            client,
            "child-world",
            profile,
            &resources,
        )
        .expect("context-routed evaluation should install");
        assert_eq!(
            resources
                .metrics()
                .expect("route metrics should be readable")
                .active_leases,
            1
        );
        let command = Command::<EvaluateValue>::new(
            CallId::new(12),
            ObjectId::new(7),
            CallMetadata::default(),
            EvaluateParams::expression("true"),
        );

        // Act
        let result = installation
            .frame()
            .call(command)
            .await
            .expect("command service is infallible")
            .into_outcome()
            .expect("context-routed CDP evaluation should succeed");

        // Assert
        assert_eq!(result.value, EvaluationValue::Bool(true));
        assert_eq!(fake.calls()[0].1[0]["uniqueContextId"], "child-world");
        drop(installation);
        assert_eq!(
            resources
                .metrics()
                .expect("released route metrics should be readable")
                .active_leases,
            0
        );
    }

    #[tokio::test]
    async fn installed_frame_rejects_an_unbound_object_before_cdp() {
        // Arrange
        let profile = profile(false);
        let resources = resources(profile);
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
        let installation = ChromiumEvaluationInstallation::install_main_frame(
            ObjectId::new(7),
            client,
            profile,
            &resources,
        )
        .expect("evaluation-only composition should install");
        let command = Command::<EvaluateValue>::new(
            CallId::new(13),
            ObjectId::new(8),
            CallMetadata::default(),
            EvaluateParams::expression("42"),
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
    fn main_frame_rejects_a_context_routing_profile() {
        // Arrange
        let profile = profile(true);
        let resources = resources(profile);
        let client: Arc<dyn pwright_cdp::CdpClient> = Arc::new(FakeCdpClient::from_html(""));

        // Act
        let error = match ChromiumEvaluationInstallation::install_main_frame(
            ObjectId::new(7),
            client,
            profile,
            &resources,
        ) {
            Ok(_) => panic!("main-Frame installation must reject retained routes"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(
            error,
            ChromiumEvaluationInstallError::MainFrameProfileHasContextRoutes { .. }
        ));
    }
}
