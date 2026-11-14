//! Attach-only provider for explicit Chromium CDP WebSocket endpoints.
//!
//! This crate opens a transport connection to an existing endpoint. It does
//! not discover executables, spawn processes, download browsers, or possess a
//! browser-process termination capability.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use pwright_browser_provider::{AttachBrowserProvider, AttachedBrowser};
#[cfg(feature = "cdp-handshake-headers")]
use pwright_cdp::CdpHandshakeHeaders;
use pwright_cdp::connection::{CdpError, ConnectionConfig};
use pwright_cdp::{CdpConnection, CdpConnectionProductionProfile, CdpProductionConnectError};
use pwright_core::{
    ProductionResourceError, ProductionResourceLedger, ResourceLedger, ResourceRequirements,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdpWebSocketEndpoint(String);

impl CdpWebSocketEndpoint {
    pub fn parse(endpoint: impl Into<String>) -> Result<Self, EndpointError> {
        let endpoint = endpoint.into();
        let parsed = url::Url::parse(&endpoint).map_err(|error| EndpointError::InvalidUrl {
            message: error.to_string(),
        })?;

        if !matches!(parsed.scheme(), "ws" | "wss") {
            return Err(EndpointError::UnsupportedScheme {
                scheme: parsed.scheme().to_string(),
            });
        }
        if parsed.host_str().is_none() {
            return Err(EndpointError::MissingHost);
        }
        if parsed.fragment().is_some() {
            return Err(EndpointError::FragmentNotAllowed);
        }

        Ok(Self(endpoint))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for CdpWebSocketEndpoint {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EndpointError {
    InvalidUrl { message: String },
    UnsupportedScheme { scheme: String },
    MissingHost,
    FragmentNotAllowed,
}

impl Display for EndpointError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl { message } => {
                write!(formatter, "invalid CDP endpoint URL: {message}")
            }
            Self::UnsupportedScheme { scheme } => {
                write!(
                    formatter,
                    "CDP endpoint scheme must be ws or wss, got `{scheme}`"
                )
            }
            Self::MissingHost => formatter.write_str("CDP endpoint must include a host"),
            Self::FragmentNotAllowed => {
                formatter.write_str("CDP endpoint must not include a URL fragment")
            }
        }
    }
}

impl Error for EndpointError {}

#[derive(Clone, Debug)]
pub struct CdpAttachRequest {
    pub endpoint: CdpWebSocketEndpoint,
    pub connection_config: ConnectionConfig,
}

impl CdpAttachRequest {
    pub fn new(endpoint: CdpWebSocketEndpoint, connection_config: ConnectionConfig) -> Self {
        Self {
            endpoint,
            connection_config,
        }
    }
}

pub trait CdpConnector: Send + Sync {
    type Connection;
    type Error;
    type Future<'a>: Future<Output = Result<Self::Connection, Self::Error>> + Send
    where
        Self: 'a;

    fn connect(&self, endpoint: CdpWebSocketEndpoint, config: ConnectionConfig)
    -> Self::Future<'_>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LiveCdpConnector;

impl CdpConnector for LiveCdpConnector {
    type Connection = Arc<CdpConnection>;
    type Error = CdpError;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Connection, Self::Error>> + Send + 'a>>;

    fn connect(
        &self,
        endpoint: CdpWebSocketEndpoint,
        config: ConnectionConfig,
    ) -> Self::Future<'_> {
        Box::pin(async move { CdpConnection::connect_with_config(endpoint.as_str(), config).await })
    }
}

/// A CDP connector that refuses undeclared resource plans and configuration
/// drift before opening the browser transport.
#[derive(Clone, Debug)]
pub struct ProductionCdpConnector {
    profile: CdpConnectionProductionProfile,
    resources: ProductionResourceLedger,
    ledger: ResourceLedger,
}

impl ProductionCdpConnector {
    pub fn new(
        profile: CdpConnectionProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        let ledger = resources.claim(profile.requirements())?;
        Ok(Self {
            profile,
            resources: resources.clone(),
            ledger,
        })
    }

    pub const fn profile(&self) -> &CdpConnectionProductionProfile {
        &self.profile
    }

    #[cfg(feature = "cdp-handshake-headers")]
    pub async fn connect_with_headers(
        &self,
        endpoint: CdpWebSocketEndpoint,
        config: ConnectionConfig,
        headers: &CdpHandshakeHeaders,
    ) -> Result<ProductionCdpConnection, CdpProductionConnectError> {
        if config != *self.profile.config() {
            return Err(CdpError::InvalidConfiguration(
                "attach request does not match the production CDP profile",
            )
            .into());
        }
        let connection = CdpConnection::connect_with_production_resources_and_headers(
            endpoint.as_str(),
            self.profile.clone(),
            &self.resources,
            headers,
        )
        .await?;
        Ok(ProductionCdpConnection {
            connection,
            ledger: self.ledger.clone(),
            requirements: self.profile.requirements(),
        })
    }
}

/// A CDP connection carrying proof that its retained production state belongs
/// to one declared resource plan and ledger.
///
/// The inner connection is intentionally not exposed until this proof is
/// consumed by a composition root. This prevents a launched browser from
/// silently entering a different connection's production budget.
pub struct ProductionCdpConnection {
    connection: Arc<CdpConnection>,
    ledger: ResourceLedger,
    requirements: ResourceRequirements,
}

impl ProductionCdpConnection {
    pub const fn requirements(&self) -> ResourceRequirements {
        self.requirements
    }

    pub fn shares_budget_with(&self, ledger: &ResourceLedger) -> bool {
        self.ledger.shares_budget_with(ledger)
    }

    pub fn into_connection(self) -> Arc<CdpConnection> {
        self.connection
    }
}

impl CdpConnector for ProductionCdpConnector {
    type Connection = ProductionCdpConnection;
    type Error = CdpProductionConnectError;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Connection, Self::Error>> + Send + 'a>>;

    fn connect(
        &self,
        endpoint: CdpWebSocketEndpoint,
        config: ConnectionConfig,
    ) -> Self::Future<'_> {
        Box::pin(async move {
            if config != *self.profile.config() {
                return Err(CdpError::InvalidConfiguration(
                    "attach request does not match the production CDP profile",
                )
                .into());
            }
            let connection = CdpConnection::connect_with_production_resources(
                endpoint.as_str(),
                self.profile.clone(),
                &self.resources,
            )
            .await?;
            Ok(ProductionCdpConnection {
                connection,
                ledger: self.ledger.clone(),
                requirements: self.profile.requirements(),
            })
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CdpAttachProvider<Connector = LiveCdpConnector> {
    connector: Connector,
}

impl CdpAttachProvider<LiveCdpConnector> {
    pub const fn new() -> Self {
        Self {
            connector: LiveCdpConnector,
        }
    }
}

#[cfg(feature = "cdp-handshake-headers")]
impl CdpAttachProvider<ProductionCdpConnector> {
    pub async fn attach_with_headers(
        &self,
        request: CdpAttachRequest,
        headers: &CdpHandshakeHeaders,
    ) -> Result<AttachedBrowser<ProductionCdpConnection>, CdpProductionConnectError> {
        self.connector
            .connect_with_headers(request.endpoint, request.connection_config, headers)
            .await
            .map(AttachedBrowser::new)
    }
}

impl<Connector> CdpAttachProvider<Connector> {
    pub const fn with_connector(connector: Connector) -> Self {
        Self { connector }
    }
}

impl<Connector> AttachBrowserProvider<CdpAttachRequest> for CdpAttachProvider<Connector>
where
    Connector: CdpConnector,
    Connector::Connection: Send,
    Connector::Error: Send,
{
    type Connection = Connector::Connection;
    type Error = Connector::Error;
    type Future<'a>
        = Pin<
        Box<
            dyn Future<Output = Result<AttachedBrowser<Self::Connection>, Self::Error>> + Send + 'a,
        >,
    >
    where
        Self: 'a;

    fn attach(&self, request: CdpAttachRequest) -> Self::Future<'_> {
        Box::pin(async move {
            self.connector
                .connect(request.endpoint, request.connection_config)
                .await
                .map(AttachedBrowser::new)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};
    use std::sync::Mutex;
    use std::time::Duration;

    use pwright_browser_provider::AttachBrowserProvider;
    use pwright_cdp::CdpConnectionProductionProfile;
    use pwright_cdp::connection::ConnectionConfig;
    use pwright_core::{
        ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceRequirements,
        ResourceScope,
    };

    use super::{
        CdpAttachProvider, CdpAttachRequest, CdpConnector, CdpWebSocketEndpoint, EndpointError,
        ProductionCdpConnector,
    };

    fn exact_resources(
        profile: &CdpConnectionProductionProfile,
        plan: ResourceRequirements,
    ) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let wire = requirements.class(ResourceClass::WireBytes);
        let event_items = requirements.class(ResourceClass::EventItem);
        let event_bytes = requirements.class(ResourceClass::EventBytes);
        let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases)
            .with_class(ResourceClass::WireBytes, wire.global, wire.per_scope)
            .with_class(
                ResourceClass::EventItem,
                event_items.global,
                event_items.per_scope,
            )
            .with_class(
                ResourceClass::EventBytes,
                event_bytes.global,
                event_bytes.per_scope,
            );
        ProductionResourceLedger::new(limits, plan).expect("resource plan should be valid")
    }

    struct RecordingConnector {
        requests: Mutex<Vec<(String, ConnectionConfig)>>,
    }

    impl CdpConnector for RecordingConnector {
        type Connection = String;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Connection, Self::Error>>;

        fn connect(
            &self,
            endpoint: CdpWebSocketEndpoint,
            config: ConnectionConfig,
        ) -> Self::Future<'_> {
            self.requests
                .lock()
                .expect("recording connector lock should be available")
                .push((endpoint.as_str().to_string(), config));
            ready(Ok("external-cdp-connection".to_string()))
        }
    }

    #[test]
    fn endpoint_accepts_explicit_websocket_schemes() {
        // Arrange
        let endpoints = [
            "ws://127.0.0.1:9222/devtools/browser/one",
            "wss://browser.example/devtools/browser/two?token=secret",
        ];

        // Act
        let parsed = endpoints.map(CdpWebSocketEndpoint::parse);

        // Assert
        assert!(parsed.iter().all(Result::is_ok));
    }

    #[test]
    fn endpoint_rejects_http_discovery_urls() {
        // Arrange
        let endpoint = "http://127.0.0.1:9222";

        // Act
        let error = CdpWebSocketEndpoint::parse(endpoint)
            .expect_err("HTTP discovery must use a separate provider");

        // Assert
        assert_eq!(
            error,
            EndpointError::UnsupportedScheme {
                scheme: "http".to_string()
            }
        );
    }

    #[test]
    fn endpoint_rejects_fragments() {
        // Arrange
        let endpoint = "ws://127.0.0.1:9222/devtools/browser/one#fragment";

        // Act
        let error = CdpWebSocketEndpoint::parse(endpoint)
            .expect_err("fragments are not sent to the WebSocket server");

        // Assert
        assert_eq!(error, EndpointError::FragmentNotAllowed);
    }

    #[tokio::test]
    async fn provider_returns_an_external_attachment_with_exact_config() {
        // Arrange
        let connector = RecordingConnector {
            requests: Mutex::new(Vec::new()),
        };
        let provider = CdpAttachProvider::with_connector(connector);
        let endpoint = CdpWebSocketEndpoint::parse("ws://127.0.0.1:9222/devtools/browser/explicit")
            .expect("endpoint should be valid");
        let config = ConnectionConfig {
            timeout: Duration::from_secs(7),
            write_channel_capacity: 11,
            event_channel_capacity: 13,
            ..ConnectionConfig::default()
        };
        let request = CdpAttachRequest::new(endpoint, config.clone());

        // Act
        let attached = provider
            .attach(request)
            .await
            .expect("recording connector should attach");

        // Assert
        assert_eq!(attached.connection(), "external-cdp-connection");
        let requests = provider
            .connector
            .requests
            .lock()
            .expect("recorded requests lock should be available");
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].0,
            "ws://127.0.0.1:9222/devtools/browser/explicit"
        );
        assert_eq!(requests[0].1.timeout, Duration::from_secs(7));
        assert_eq!(requests[0].1.write_channel_capacity, 11);
        assert_eq!(requests[0].1.event_channel_capacity, 13);
    }

    #[test]
    fn production_connector_rejects_an_omitted_profile_at_construction() {
        // Arrange
        let profile = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig {
                pending_command_capacity: 1,
                event_channel_capacity: 1,
                max_command_bytes: 64,
                max_response_bytes: 64,
                max_event_bytes: 64,
                max_websocket_bytes: 64,
                ..ConnectionConfig::default()
            },
        );
        let resources = exact_resources(&profile, ResourceRequirements::none());

        // Act
        let result = ProductionCdpConnector::new(profile, &resources);

        // Assert
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn production_connector_rejects_config_drift_before_network_access() {
        // Arrange
        let profile = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig {
                pending_command_capacity: 1,
                event_channel_capacity: 1,
                max_command_bytes: 64,
                max_response_bytes: 64,
                max_event_bytes: 64,
                max_websocket_bytes: 64,
                ..ConnectionConfig::default()
            },
        );
        let resources = exact_resources(&profile, profile.requirements());
        let connector = ProductionCdpConnector::new(profile.clone(), &resources)
            .expect("declared profile should construct");
        let endpoint = CdpWebSocketEndpoint::parse("ws://127.0.0.1:1/should-not-connect")
            .expect("endpoint should parse");
        let mut drifted = profile.config().clone();
        drifted.pending_command_capacity = 2;

        // Act
        let error = match connector.connect(endpoint, drifted).await {
            Ok(_) => panic!("configuration drift must fail"),
            Err(error) => error,
        };

        // Assert
        assert!(
            error
                .to_string()
                .contains("does not match the production CDP profile")
        );
    }
}
