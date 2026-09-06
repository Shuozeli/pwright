//! Optional launch composition for a Chromium process owned by Pwright.
//!
//! The attach-only composition remains independent from this crate. Selecting
//! this leaf requires an explicit, already-installed browser executable and an
//! explicit dedicated profile directory. It never discovers, downloads, or
//! bundles a browser as a side effect of composition.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;

use pwright_browser_provider::{BrowserProcessLease, LaunchBrowserProvider, LaunchedBrowser};
use pwright_cap_browser_lifecycle::{BROWSER_LIFECYCLE_CAPABILITY_ID, BrowserLifecycleCapability};
use pwright_cdp::CdpConnection;
use pwright_codec_browser_lifecycle::{BrowserLifecycleCodec, BrowserLifecycleCodecLimits};
use pwright_codec_handshake::HandshakeCodec;
use pwright_composition_chromium::{
    AttachedChromiumComposition, CapabilityWireProfile, ChromiumAttachError, ChromiumAttachFactory,
    ChromiumAttachFactoryError,
};
use pwright_core::{Capability, ProductionResourceError};
use pwright_provider_cdp::ProductionCdpConnector;
use pwright_provider_chromium::{
    ChromiumBrowserShutdown, ChromiumLaunchError, ChromiumLaunchProvider, ChromiumLaunchRequest,
    ChromiumProcessLease, ChromiumShutdownError, ChromiumShutdownPolicy, ChromiumShutdownReport,
    DevToolsActivePortWaiter, TokioChromiumChild, TokioChromiumProcessSpawner,
};
use pwright_server_wire::AcceptedServerHandshake;
use pwright_transport::{
    CapabilityManifest, CapabilitySetError, CapabilitySupport, ServerHandshakeConfig,
    WireCapabilityId, WireProtocol,
};

mod browser_close;

pub use browser_close::*;

/// Reusable optional recipe for launching one dedicated owned Chromium and
/// binding it to a fresh production composition.
#[derive(Clone, Debug)]
pub struct ChromiumLaunchCompositionFactory {
    attachment: ChromiumAttachFactory,
    browser_lifecycle: Option<OwnedBrowserLifecycleInstallationProfile>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OwnedBrowserLifecycleInstallationProfile {
    codec: BrowserLifecycleCodecLimits,
    wire: CapabilityWireProfile,
}

impl OwnedBrowserLifecycleInstallationProfile {
    pub const fn new(codec: BrowserLifecycleCodecLimits, wire: CapabilityWireProfile) -> Self {
        Self { codec, wire }
    }

    pub const fn codec(self) -> BrowserLifecycleCodecLimits {
        self.codec
    }

    pub const fn wire(self) -> CapabilityWireProfile {
        self.wire
    }

    pub fn requirements(self) -> pwright_core::ResourceRequirements {
        self.unbudgeted_protocol()
            .production_resource_requirements()
    }

    fn unbudgeted_protocol(self) -> WireProtocol<BrowserLifecycleCodec> {
        WireProtocol::new_exact_directional(
            BrowserLifecycleCodec::new(self.codec),
            self.wire.version(),
            self.wire.send_limits(),
            self.wire.receive_limits(),
        )
    }
}

/// An externally attached composition has no process lease and therefore
/// cannot enter the owned shutdown path.
///
/// ```compile_fail
/// use pwright_composition_chromium::AttachedChromiumComposition;
/// use pwright_provider_chromium::ChromiumShutdownPolicy;
///
/// async fn cannot_terminate_external(
///     attached: AttachedChromiumComposition,
///     policy: ChromiumShutdownPolicy,
/// ) {
///     attached.shutdown(policy).await.unwrap();
/// }
/// ```
///
/// ```compile_fail
/// use pwright_composition_chromium::AttachedChromiumComposition;
/// use pwright_server_wire::AcceptedServerHandshake;
///
/// fn cannot_authorize_remote_close(
///     attached: AttachedChromiumComposition,
///     accepted: &AcceptedServerHandshake<()>,
/// ) {
///     let _grant = attached.into_browser_close_grant(accepted).unwrap();
/// }
/// ```
const _ATTACHMENT_HAS_NO_PROCESS_AUTHORITY: () = ();

impl ChromiumLaunchCompositionFactory {
    pub const fn new(attachment: ChromiumAttachFactory) -> Self {
        Self {
            attachment,
            browser_lifecycle: None,
        }
    }

    pub const fn with_remote_browser_lifecycle(
        mut self,
        profile: OwnedBrowserLifecycleInstallationProfile,
    ) -> Self {
        self.browser_lifecycle = Some(profile);
        self
    }

    pub const fn attachment(&self) -> &ChromiumAttachFactory {
        &self.attachment
    }

    pub const fn browser_lifecycle_profile(
        &self,
    ) -> Option<OwnedBrowserLifecycleInstallationProfile> {
        self.browser_lifecycle
    }

    fn launch_attachment(&self) -> Result<ChromiumAttachFactory, ChromiumAttachFactoryError> {
        let mut profile = self.attachment.profile().clone();
        if let Some(browser_lifecycle) = self.browser_lifecycle {
            profile = profile.with_extension_requirements(
                browser_lifecycle
                    .unbudgeted_protocol()
                    .production_resource_requirements(),
            );
        }
        ChromiumAttachFactory::new(
            profile,
            self.attachment.limits(),
            self.attachment.browser_object_id(),
        )
    }

    /// Returns the attach manifest unchanged unless the launch-only lifecycle
    /// leaf was explicitly selected, in which case exactly one owned Browser
    /// capability is added to this server hello.
    pub fn remote_handshake_parts(
        &self,
    ) -> Result<Option<(HandshakeCodec, ServerHandshakeConfig)>, ChromiumLaunchCompositionError>
    {
        let launch_attachment = self
            .launch_attachment()
            .map_err(ChromiumLaunchCompositionError::Preparation)?;
        let prepared = launch_attachment
            .prepare()
            .map_err(ChromiumLaunchCompositionError::Preparation)?;
        let Some((codec, config)) = prepared.remote_handshake_parts() else {
            if self.browser_lifecycle.is_some() {
                return Err(ChromiumLaunchCompositionError::BrowserLifecycleRequiresHandshake);
            }
            return Ok(None);
        };
        let Some(browser_lifecycle) = self.browser_lifecycle else {
            return Ok(Some((codec, config)));
        };
        let handshake = self
            .attachment
            .profile()
            .remote_handshake()
            .expect("prepared remote handshake has its source profile");
        if browser_lifecycle.wire().send_limits().max_frame_bytes()
            != handshake.production().maximum_peer_receive_frame_bytes()
        {
            return Err(ChromiumLaunchCompositionError::BrowserLifecyclePeerFrameLimitMismatch);
        }
        let mut support = config
            .capabilities()
            .supported_ranges()
            .map(|(id, versions)| CapabilitySupport::new(id.clone(), versions))
            .collect::<Vec<_>>();
        let id = WireCapabilityId::new(
            BROWSER_LIFECYCLE_CAPABILITY_ID.as_str(),
            BROWSER_LIFECYCLE_CAPABILITY_ID.as_str().len(),
        )
        .expect("the static Browser lifecycle capability ID is valid");
        support.push(CapabilitySupport::new(
            id,
            pwright_transport::VersionPolicy::compatible_through(
                BrowserLifecycleCapability::DESCRIPTOR.plugin.version,
            ),
        ));
        let capabilities =
            CapabilityManifest::from_support(support, handshake.negotiation_limits())
                .map_err(ChromiumLaunchCompositionError::Capabilities)?;
        Ok(Some((
            codec,
            ServerHandshakeConfig::new(config.versions(), config.receive_limits(), capabilities),
        )))
    }

    /// Launches only the caller-selected executable and consumes the
    /// production-connection proof while binding it to this factory's fresh
    /// resource ledger and object namespace.
    pub async fn launch(
        &self,
        request: ChromiumLaunchRequest,
    ) -> Result<OwnedChromiumComposition<TokioChromiumChild>, ChromiumLaunchCompositionError> {
        let expected_config = self.attachment.profile().connection().config();
        if request.connection_config() != expected_config {
            return Err(ChromiumLaunchCompositionError::ConnectionConfigurationMismatch);
        }
        let remote_handshake = self.remote_handshake_parts()?;

        let launch_attachment = self
            .launch_attachment()
            .map_err(ChromiumLaunchCompositionError::Preparation)?;
        let prepared = launch_attachment
            .prepare()
            .map_err(ChromiumLaunchCompositionError::Preparation)?;
        let connector = ProductionCdpConnector::new(
            self.attachment.profile().connection().clone(),
            prepared.resources(),
        )
        .map_err(ChromiumLaunchCompositionError::Resources)?;
        let provider = ChromiumLaunchProvider::with_parts(
            TokioChromiumProcessSpawner,
            DevToolsActivePortWaiter::default(),
            connector,
        );
        let launched = provider
            .launch(request)
            .await
            .map_err(|error| ChromiumLaunchCompositionError::Launch(Box::new(error)))?;
        let (connection, lease) = launched.into_connection_and_lease();

        match prepared.bind_connection(
            connection,
            self.attachment.browser_object_id(),
            self.attachment.initial_epoch(),
        ) {
            Ok(composition) => Ok(OwnedChromiumComposition {
                composition,
                lease: Some(lease),
                browser_lifecycle: self.browser_lifecycle,
                remote_handshake,
            }),
            Err(binding) => match lease.terminate().await {
                Ok(_) => Err(ChromiumLaunchCompositionError::Binding(Box::new(binding))),
                Err(cleanup) => Err(ChromiumLaunchCompositionError::BindingCleanup {
                    binding: Box::new(binding),
                    cleanup,
                }),
            },
        }
    }
}

/// One attached Chromium composition plus the exclusive authority to
/// terminate exactly the process that produced its CDP connection.
pub struct OwnedChromiumComposition<Child> {
    composition: AttachedChromiumComposition,
    lease: Option<ChromiumProcessLease<Child>>,
    browser_lifecycle: Option<OwnedBrowserLifecycleInstallationProfile>,
    remote_handshake: Option<(HandshakeCodec, ServerHandshakeConfig)>,
}

impl<Child> OwnedChromiumComposition<Child> {
    pub const fn composition(&self) -> &AttachedChromiumComposition {
        &self.composition
    }

    pub fn composition_mut(&mut self) -> &mut AttachedChromiumComposition {
        &mut self.composition
    }

    pub const fn process_lease(&self) -> Option<&ChromiumProcessLease<Child>> {
        self.lease.as_ref()
    }

    /// Returns the owned launch leaf's exact handshake. Unlike the nested
    /// attach composition, this includes the explicitly selected
    /// `pwright.browser_lifecycle` capability.
    pub fn remote_handshake_parts(&self) -> Option<(HandshakeCodec, ServerHandshakeConfig)> {
        self.remote_handshake.clone()
    }

    pub fn browser_lifecycle_protocol<Authorization>(
        &self,
        accepted: &AcceptedServerHandshake<Authorization>,
    ) -> Result<Option<WireProtocol<BrowserLifecycleCodec>>, ProductionResourceError> {
        let Some(profile) = self.browser_lifecycle else {
            return Ok(None);
        };
        let id = WireCapabilityId::new(
            BROWSER_LIFECYCLE_CAPABILITY_ID.as_str(),
            BROWSER_LIFECYCLE_CAPABILITY_ID.as_str().len(),
        )
        .expect("the static Browser lifecycle capability ID is valid");
        if accepted.hello().enabled_capability_version(&id).is_none() {
            return Ok(None);
        }
        accepted
            .application_protocol(BrowserLifecycleCodec::new(profile.codec()))
            .with_production_resources(self.composition.resources())
            .map(Some)
    }

    pub fn into_parts(
        self,
    ) -> (
        AttachedChromiumComposition,
        Option<ChromiumProcessLease<Child>>,
    ) {
        (self.composition, self.lease)
    }
}

impl OwnedChromiumComposition<TokioChromiumChild> {
    /// Consumes the unique process lease, attempts bounded `Browser.close`,
    /// observes process exit, and escalates to the provider's bounded force
    /// cleanup when necessary.
    ///
    /// An unpolled future is lazy. Once first polled, cancellation activates
    /// the provider's kill-on-drop guard so started shutdown cannot orphan the
    /// owned process.
    pub async fn shutdown(
        &mut self,
        policy: ChromiumShutdownPolicy,
    ) -> Result<ChromiumShutdownReport, OwnedChromiumShutdownError> {
        let lease = self
            .lease
            .take()
            .ok_or(OwnedChromiumShutdownError::ProcessAuthorityConsumed)?;
        let connection = std::sync::Arc::clone(self.composition.connection());
        let browser: LaunchedBrowser<
            std::sync::Arc<CdpConnection>,
            ChromiumProcessLease<TokioChromiumChild>,
        > = LaunchedBrowser::new(connection, lease);
        ChromiumBrowserShutdown::new()
            .shutdown(browser, policy)
            .await
            .map_err(OwnedChromiumShutdownError::Provider)
    }
}

#[derive(Debug)]
pub enum OwnedChromiumShutdownError {
    ProcessAuthorityConsumed,
    Provider(ChromiumShutdownError),
}

impl Display for OwnedChromiumShutdownError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProcessAuthorityConsumed => {
                formatter.write_str("owned Chromium process authority was already consumed")
            }
            Self::Provider(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for OwnedChromiumShutdownError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Provider(error) => Some(error),
            Self::ProcessAuthorityConsumed => None,
        }
    }
}

#[derive(Debug)]
pub enum ChromiumLaunchCompositionError {
    ConnectionConfigurationMismatch,
    Preparation(ChromiumAttachFactoryError),
    Resources(ProductionResourceError),
    BrowserLifecycleRequiresHandshake,
    BrowserLifecyclePeerFrameLimitMismatch,
    Capabilities(CapabilitySetError),
    Launch(Box<ChromiumLaunchError>),
    Binding(Box<ChromiumAttachError>),
    BindingCleanup {
        binding: Box<ChromiumAttachError>,
        cleanup: io::Error,
    },
}

impl Display for ChromiumLaunchCompositionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionConfigurationMismatch => formatter.write_str(
                "Chromium launch request CDP configuration does not match the composition profile",
            ),
            Self::Preparation(error) => Display::fmt(error, formatter),
            Self::Resources(error) => Display::fmt(error, formatter),
            Self::BrowserLifecycleRequiresHandshake => formatter.write_str(
                "remote owned Browser lifecycle requires a remote handshake profile",
            ),
            Self::BrowserLifecyclePeerFrameLimitMismatch => formatter.write_str(
                "owned Browser lifecycle outbound frame bound does not match the handshake peer bound",
            ),
            Self::Capabilities(error) => Display::fmt(error, formatter),
            Self::Launch(error) => Display::fmt(error, formatter),
            Self::Binding(error) => Display::fmt(error, formatter),
            Self::BindingCleanup { binding, cleanup } => write!(
                formatter,
                "launched Chromium connection could not bind ({binding}) and owned-process cleanup failed: {cleanup}"
            ),
        }
    }
}

impl Error for ChromiumLaunchCompositionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Preparation(error) => Some(error),
            Self::Resources(error) => Some(error),
            Self::Capabilities(error) => Some(error),
            Self::Launch(error) => Some(error.as_ref()),
            Self::Binding(error) => Some(error.as_ref()),
            Self::BindingCleanup { binding, .. } => Some(binding.as_ref()),
            Self::ConnectionConfigurationMismatch
            | Self::BrowserLifecycleRequiresHandshake
            | Self::BrowserLifecyclePeerFrameLimitMismatch => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use pwright_backend_cdp::{CdpTopologyLimits, CdpTopologyProductionProfile};
    use pwright_cdp::CdpConnectionProductionProfile;
    use pwright_cdp::connection::ConnectionConfig;
    use pwright_composition_chromium::{
        CapabilityWireProfile, CdpPageRouteProductionProfile, ChromiumAttachProductionProfile,
        ChromiumRemoteHandshakeProfile,
    };
    use pwright_core::{
        ObjectId, ProtocolVersion, ResourceClass, ResourceLimits, ResourceRequirements,
        ResourceScope,
    };
    use pwright_provider_chromium::{ChromiumChannel, ChromiumExecutable, ChromiumLaunchRequest};
    use pwright_runtime::RuntimeLimits;
    use pwright_server_wire::ServerSupervisorLimits;
    use pwright_transport::{FrameLimits, NegotiationLimits, VersionPolicy, WireCapabilityId};

    use super::{
        ChromiumLaunchCompositionError, ChromiumLaunchCompositionFactory,
        OwnedBrowserLifecycleInstallationProfile,
    };

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("test clock should be after the Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "pwright-launch-composition-{label}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("test directory should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn profile(config: ConnectionConfig) -> ChromiumAttachProductionProfile {
        let connection = CdpConnectionProductionProfile::new(ResourceScope::Connection, config);
        let topology = CdpTopologyProductionProfile::new(
            CdpTopologyLimits::new(8, 2, 2, 4, 4, 128),
            ResourceScope::extension(21, 1),
        )
        .expect("topology profile should validate");
        let routes =
            CdpPageRouteProductionProfile::from_topology(ResourceScope::extension(21, 2), topology);
        let supervisor =
            ServerSupervisorLimits::new(1).expect("supervisor profile should validate");
        ChromiumAttachProductionProfile::new(
            connection,
            topology,
            routes,
            RuntimeLimits::new(8, 2).with_active_commands(1),
            supervisor,
        )
        .with_page_events()
    }

    fn exact_limits(requirements: ResourceRequirements) -> ResourceLimits {
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            limits = limits.with_class(class, required.global, required.per_scope);
        }
        limits
    }

    fn browser_lifecycle_profile() -> OwnedBrowserLifecycleInstallationProfile {
        let frames = FrameLimits::new(1024, 0, 0, 1024);
        OwnedBrowserLifecycleInstallationProfile::new(
            pwright_codec_browser_lifecycle::BrowserLifecycleCodecLimits::new(256),
            CapabilityWireProfile::new(ProtocolVersion::new(1, 0), frames, frames),
        )
    }

    fn with_remote_handshake(
        profile: ChromiumAttachProductionProfile,
    ) -> ChromiumAttachProductionProfile {
        let frames = FrameLimits::new(1024, 0, 0, 1024);
        profile.with_remote_handshake(ChromiumRemoteHandshakeProfile::new(
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0)).unwrap(),
            frames,
            NegotiationLimits::new(4, 128),
            1024,
        ))
    }

    fn fake_executable(directory: &TestDirectory) -> ChromiumExecutable {
        let path = directory.path().join("fake-chromium");
        std::fs::write(&path, b"#!/bin/sh\nexit 1\n").expect("fake executable should be written");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&path)
                .expect("fake executable metadata should exist")
                .permissions();
            permissions.set_mode(0o700);
            std::fs::set_permissions(&path, permissions)
                .expect("fake executable should be executable");
        }
        ChromiumExecutable::from_path(path, ChromiumChannel::Custom)
            .expect("fake executable should pass path validation")
    }

    #[tokio::test]
    async fn configuration_drift_fails_before_the_browser_can_be_spawned() {
        // Arrange
        let executable_dir = TestDirectory::new("executable");
        let profile_dir = TestDirectory::new("profile");
        let config = ConnectionConfig::default();
        let profile = profile(config.clone());
        let requirements = profile.requirements();
        let attachment = pwright_composition_chromium::ChromiumAttachFactory::new(
            profile,
            exact_limits(requirements),
            ObjectId::new(1),
        )
        .expect("attachment factory should validate");
        let factory = ChromiumLaunchCompositionFactory::new(attachment);
        let mut drifted = config;
        drifted.pending_command_capacity = drifted.pending_command_capacity.saturating_add(1);
        let request = ChromiumLaunchRequest::new(
            fake_executable(&executable_dir),
            profile_dir.path(),
            drifted,
        )
        .with_startup_timeout(Duration::from_millis(1));

        // Act
        let result = factory.launch(request).await;

        // Assert
        assert!(matches!(
            result,
            Err(ChromiumLaunchCompositionError::ConnectionConfigurationMismatch)
        ));
    }

    #[tokio::test]
    async fn remote_browser_lifecycle_without_a_handshake_fails_before_spawn() {
        // Arrange
        let executable_dir = TestDirectory::new("missing-handshake-executable");
        let profile_dir = TestDirectory::new("missing-handshake-profile");
        let config = ConnectionConfig::default();
        let profile = profile(config.clone());
        let lifecycle = browser_lifecycle_profile();
        let limits = exact_limits(
            profile
                .requirements()
                .combine_concurrent(lifecycle.requirements()),
        );
        let attachment = pwright_composition_chromium::ChromiumAttachFactory::new(
            profile,
            limits,
            ObjectId::new(1),
        )
        .expect("attachment factory should validate");
        let factory = ChromiumLaunchCompositionFactory::new(attachment)
            .with_remote_browser_lifecycle(lifecycle);
        let request = ChromiumLaunchRequest::new(
            fake_executable(&executable_dir),
            profile_dir.path(),
            config,
        )
        .with_startup_timeout(Duration::from_millis(1));

        // Act
        let result = factory.launch(request).await;

        // Assert
        assert!(matches!(
            result,
            Err(ChromiumLaunchCompositionError::BrowserLifecycleRequiresHandshake)
        ));
    }

    #[test]
    fn launch_only_browser_lifecycle_is_added_without_changing_attach_manifest() {
        // Arrange
        let profile = with_remote_handshake(profile(ConnectionConfig::default()));
        let lifecycle = browser_lifecycle_profile();
        let limits = exact_limits(
            profile
                .requirements()
                .combine_concurrent(lifecycle.requirements()),
        );
        let attachment = pwright_composition_chromium::ChromiumAttachFactory::new(
            profile,
            limits,
            ObjectId::new(1),
        )
        .unwrap();
        let attach_manifest = attachment.prepare().unwrap();
        let factory = ChromiumLaunchCompositionFactory::new(attachment)
            .with_remote_browser_lifecycle(lifecycle);
        let capability = WireCapabilityId::new("pwright.browser_lifecycle", 128).unwrap();

        // Act
        let (_, owned_handshake) = factory.remote_handshake_parts().unwrap().unwrap();

        // Assert
        assert!(
            attach_manifest
                .capability_manifest()
                .support(&capability)
                .is_none()
        );
        assert!(
            owned_handshake
                .capabilities()
                .support(&capability)
                .is_some()
        );
    }
}
