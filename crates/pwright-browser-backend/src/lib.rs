//! Backend-neutral browser adapter identity and support manifests.
//!
//! Portable capability crates define Playwright-shaped commands. Concrete
//! adapter crates declare which of those commands they implement through this
//! manifest, while protocol sessions and native message types remain in the
//! adapter crate. This crate selects no browser protocol, transport, runtime,
//! provider, or browser process.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;
use std::sync::Arc;

use pwright_core::{
    Capability, CapabilityCommandSpec, CapabilityId, CommandId, EventId, ProtocolVersion, Registry,
    Service,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BackendId(&'static str);

impl BackendId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl Display for BackendId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BackendProtocolId(&'static str);

impl BackendProtocolId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl Display for BackendProtocolId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BackendFeatureId(&'static str);

impl BackendFeatureId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl Display for BackendFeatureId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BrowserEngine {
    Chromium,
    Firefox,
    WebKit,
}

/// Whether an adapter maps a command directly or implements it by composition.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BackendImplementation {
    Native,
    Emulated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendFeatureDescriptor {
    pub id: BackendFeatureId,
    pub capability: CapabilityId,
    pub minimum_version: ProtocolVersion,
    pub command: CommandId,
    pub implementation: BackendImplementation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendEventFeatureDescriptor {
    pub id: BackendFeatureId,
    pub capability: CapabilityId,
    pub minimum_version: ProtocolVersion,
    pub event: EventId,
    pub implementation: BackendImplementation,
}

impl BackendEventFeatureDescriptor {
    pub const fn new(
        id: BackendFeatureId,
        capability: CapabilityId,
        minimum_version: ProtocolVersion,
        event: EventId,
        implementation: BackendImplementation,
    ) -> Self {
        Self {
            id,
            capability,
            minimum_version,
            event,
            implementation,
        }
    }
}

impl BackendFeatureDescriptor {
    pub const fn new(
        id: BackendFeatureId,
        capability: CapabilityId,
        minimum_version: ProtocolVersion,
        command: CommandId,
        implementation: BackendImplementation,
    ) -> Self {
        Self {
            id,
            capability,
            minimum_version,
            command,
            implementation,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendDescriptor {
    pub id: BackendId,
    pub name: &'static str,
    pub engine: BrowserEngine,
    pub protocol: BackendProtocolId,
    pub features: &'static [BackendFeatureDescriptor],
    pub event_features: &'static [BackendEventFeatureDescriptor],
}

impl BackendDescriptor {
    pub const fn new(
        id: BackendId,
        name: &'static str,
        engine: BrowserEngine,
        protocol: BackendProtocolId,
        features: &'static [BackendFeatureDescriptor],
    ) -> Self {
        Self {
            id,
            name,
            engine,
            protocol,
            features,
            event_features: &[],
        }
    }

    pub const fn with_event_features(
        mut self,
        event_features: &'static [BackendEventFeatureDescriptor],
    ) -> Self {
        self.event_features = event_features;
        self
    }
}

/// Marker implemented by one concrete browser adapter family.
pub trait BrowserBackend: Send + Sync + 'static {
    const DESCRIPTOR: BackendDescriptor;
}

/// Explicit native-protocol escape hatch for an adapter-owned value.
///
/// Portable objects do not implement this trait. Access requires importing the
/// concrete backend marker and its adapter crate, making protocol coupling
/// visible in the consumer's dependency graph.
pub trait NativeBackendAccess<Backend>
where
    Backend: BrowserBackend,
{
    type Native: ?Sized;

    fn native_backend(&self) -> &Self::Native;
}

/// Validated support declaration for one concrete backend and capability set.
///
/// This value contains descriptors only. It cannot expose a native protocol
/// session or issue browser commands.
///
/// ```compile_fail
/// use pwright_browser_backend::BackendManifest;
///
/// fn manifest_has_no_native_session(manifest: BackendManifest) {
///     let _ = manifest.native_session();
/// }
/// ```
#[derive(Clone, Debug)]
pub struct BackendManifest {
    descriptor: BackendDescriptor,
    features: BTreeMap<BackendFeatureId, BackendFeatureDescriptor>,
    event_features: BTreeMap<BackendFeatureId, BackendEventFeatureDescriptor>,
}

/// Service wrapper that rejects an unsupported backend feature before calling
/// the concrete adapter.
#[derive(Clone, Debug)]
pub struct BackendCheckedService<Command, Inner> {
    inner: Inner,
    manifest: Arc<BackendManifest>,
    feature: BackendFeatureId,
    marker: PhantomData<fn() -> Command>,
}

impl<Command, Inner> BackendCheckedService<Command, Inner>
where
    Command: CapabilityCommandSpec,
{
    pub fn try_new(
        inner: Inner,
        manifest: Arc<BackendManifest>,
        feature: BackendFeatureId,
    ) -> Result<Self, BackendServiceBindingError> {
        let declared = manifest
            .require(feature)
            .map_err(BackendServiceBindingError::Unsupported)?;
        if declared.command != Command::COMMAND {
            return Err(BackendServiceBindingError::CommandMismatch {
                feature,
                expected: Command::COMMAND,
                declared: declared.command,
            });
        }
        let expected_capability = <Command::Capability as Capability>::DESCRIPTOR.plugin.id;
        if declared.capability != expected_capability {
            return Err(BackendServiceBindingError::CapabilityMismatch {
                feature,
                expected: expected_capability,
                declared: declared.capability,
            });
        }
        Ok(Self {
            inner,
            manifest,
            feature,
            marker: PhantomData,
        })
    }

    pub const fn feature(&self) -> BackendFeatureId {
        self.feature
    }

    pub fn manifest(&self) -> &Arc<BackendManifest> {
        &self.manifest
    }

    pub const fn inner(&self) -> &Inner {
        &self.inner
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendServiceBindingError {
    Unsupported(UnsupportedBackendFeature),
    CommandMismatch {
        feature: BackendFeatureId,
        expected: CommandId,
        declared: CommandId,
    },
    CapabilityMismatch {
        feature: BackendFeatureId,
        expected: CapabilityId,
        declared: CapabilityId,
    },
}

impl Display for BackendServiceBindingError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(error) => Display::fmt(error, formatter),
            Self::CommandMismatch {
                feature,
                expected,
                declared,
            } => write!(
                formatter,
                "backend feature `{feature}` declares command `{declared}` but typed service expects `{expected}`"
            ),
            Self::CapabilityMismatch {
                feature,
                expected,
                declared,
            } => write!(
                formatter,
                "backend feature `{feature}` declares capability `{declared}` but typed service expects `{expected}`"
            ),
        }
    }
}

impl Error for BackendServiceBindingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Unsupported(error) => Some(error),
            Self::CommandMismatch { .. } | Self::CapabilityMismatch { .. } => None,
        }
    }
}

impl<Command, Inner> Service<Command::Params> for BackendCheckedService<Command, Inner>
where
    Command: CapabilityCommandSpec,
    Inner: Service<Command::Params>,
{
    type Response = Inner::Response;
    type Error = Inner::Error;
    type Future<'a>
        = Inner::Future<'a>
    where
        Self: 'a;

    fn call(&self, request: Command::Params) -> Self::Future<'_> {
        self.inner.call(request)
    }
}

impl BackendManifest {
    pub fn build(
        descriptor: BackendDescriptor,
        capabilities: &Registry,
    ) -> Result<Self, BackendManifestError> {
        Self::build_with_missing_policy(descriptor, capabilities, MissingCapabilityPolicy::Reject)
    }

    fn build_with_missing_policy(
        descriptor: BackendDescriptor,
        capabilities: &Registry,
        missing_policy: MissingCapabilityPolicy,
    ) -> Result<Self, BackendManifestError> {
        validate_identifier("backend", descriptor.id.as_str())?;
        validate_identifier("backend protocol", descriptor.protocol.as_str())?;
        if descriptor.name.trim().is_empty() {
            return Err(BackendManifestError::EmptyBackendName {
                backend: descriptor.id,
            });
        }
        let mut features = BTreeMap::new();
        let mut event_features = BTreeMap::new();
        let mut declared_features = BTreeSet::new();
        for feature in descriptor.features {
            validate_feature_identifier(feature.id)?;
            if !declared_features.insert(feature.id) {
                return Err(BackendManifestError::DuplicateFeature {
                    backend: descriptor.id,
                    feature: feature.id,
                });
            }
            let Some(capability) = capabilities.descriptor(feature.capability) else {
                match missing_policy {
                    MissingCapabilityPolicy::Reject => {
                        return Err(BackendManifestError::MissingCapability {
                            backend: descriptor.id,
                            feature: feature.id,
                            capability: feature.capability,
                        });
                    }
                    MissingCapabilityPolicy::Omit => continue,
                }
            };
            if !capability.plugin.version.satisfies(feature.minimum_version) {
                return Err(BackendManifestError::IncompatibleCapability {
                    backend: descriptor.id,
                    feature: feature.id,
                    capability: feature.capability,
                    minimum: feature.minimum_version,
                    installed: capability.plugin.version,
                });
            }
            let actual_owner = capabilities.owner_of(feature.command);
            if actual_owner != Some(feature.capability) {
                return Err(BackendManifestError::CommandOwnerMismatch {
                    backend: descriptor.id,
                    feature: feature.id,
                    command: feature.command,
                    expected: feature.capability,
                    actual: actual_owner,
                });
            }
            features.insert(feature.id, *feature);
        }
        for feature in descriptor.event_features {
            validate_feature_identifier(feature.id)?;
            if !declared_features.insert(feature.id) {
                return Err(BackendManifestError::DuplicateFeature {
                    backend: descriptor.id,
                    feature: feature.id,
                });
            }
            let Some(capability) = capabilities.descriptor(feature.capability) else {
                match missing_policy {
                    MissingCapabilityPolicy::Reject => {
                        return Err(BackendManifestError::MissingCapability {
                            backend: descriptor.id,
                            feature: feature.id,
                            capability: feature.capability,
                        });
                    }
                    MissingCapabilityPolicy::Omit => continue,
                }
            };
            if !capability.plugin.version.satisfies(feature.minimum_version) {
                return Err(BackendManifestError::IncompatibleCapability {
                    backend: descriptor.id,
                    feature: feature.id,
                    capability: feature.capability,
                    minimum: feature.minimum_version,
                    installed: capability.plugin.version,
                });
            }
            let actual_owner = capabilities.owner_of_event(feature.event);
            if actual_owner != Some(feature.capability) {
                return Err(BackendManifestError::EventOwnerMismatch {
                    backend: descriptor.id,
                    feature: feature.id,
                    event: feature.event,
                    expected: feature.capability,
                    actual: actual_owner,
                });
            }
            event_features.insert(feature.id, *feature);
        }
        Ok(Self {
            descriptor,
            features,
            event_features,
        })
    }

    pub fn for_backend<Backend>(capabilities: &Registry) -> Result<Self, BackendManifestError>
    where
        Backend: BrowserBackend,
    {
        Self::build(Backend::DESCRIPTOR, capabilities)
    }

    /// Builds the backend view for exactly the capabilities selected by one
    /// composition root.
    ///
    /// The backend descriptor is the adapter's complete potential support;
    /// features owned by capabilities absent from `capabilities` are omitted.
    /// Installed capabilities remain fully version- and ownership-checked.
    pub fn for_installed_capabilities<Backend>(
        capabilities: &Registry,
    ) -> Result<Self, BackendManifestError>
    where
        Backend: BrowserBackend,
    {
        Self::build_with_missing_policy(
            Backend::DESCRIPTOR,
            capabilities,
            MissingCapabilityPolicy::Omit,
        )
    }

    pub const fn descriptor(&self) -> BackendDescriptor {
        self.descriptor
    }

    pub fn supports(&self, feature: BackendFeatureId) -> bool {
        self.features.contains_key(&feature) || self.event_features.contains_key(&feature)
    }

    pub fn require(
        &self,
        feature: BackendFeatureId,
    ) -> Result<&BackendFeatureDescriptor, UnsupportedBackendFeature> {
        self.features
            .get(&feature)
            .ok_or(UnsupportedBackendFeature {
                backend: self.descriptor.id,
                feature,
            })
    }

    pub fn features(
        &self,
    ) -> impl ExactSizeIterator<Item = &BackendFeatureDescriptor> + DoubleEndedIterator {
        self.features.values()
    }

    pub fn require_event(
        &self,
        feature: BackendFeatureId,
    ) -> Result<&BackendEventFeatureDescriptor, UnsupportedBackendFeature> {
        self.event_features
            .get(&feature)
            .ok_or(UnsupportedBackendFeature {
                backend: self.descriptor.id,
                feature,
            })
    }

    pub fn event_features(
        &self,
    ) -> impl ExactSizeIterator<Item = &BackendEventFeatureDescriptor> + DoubleEndedIterator {
        self.event_features.values()
    }
}

#[derive(Clone, Copy)]
enum MissingCapabilityPolicy {
    Reject,
    Omit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnsupportedBackendFeature {
    pub backend: BackendId,
    pub feature: BackendFeatureId,
}

impl Display for UnsupportedBackendFeature {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "backend `{}` does not support feature `{}`",
            self.backend, self.feature
        )
    }
}

impl Error for UnsupportedBackendFeature {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendManifestError {
    InvalidIdentifier {
        kind: &'static str,
        value: &'static str,
    },
    EmptyBackendName {
        backend: BackendId,
    },
    DuplicateFeature {
        backend: BackendId,
        feature: BackendFeatureId,
    },
    MissingCapability {
        backend: BackendId,
        feature: BackendFeatureId,
        capability: CapabilityId,
    },
    IncompatibleCapability {
        backend: BackendId,
        feature: BackendFeatureId,
        capability: CapabilityId,
        minimum: ProtocolVersion,
        installed: ProtocolVersion,
    },
    CommandOwnerMismatch {
        backend: BackendId,
        feature: BackendFeatureId,
        command: CommandId,
        expected: CapabilityId,
        actual: Option<CapabilityId>,
    },
    EventOwnerMismatch {
        backend: BackendId,
        feature: BackendFeatureId,
        event: EventId,
        expected: CapabilityId,
        actual: Option<CapabilityId>,
    },
}

impl Display for BackendManifestError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { kind, value } => write!(
                formatter,
                "{kind} identifier `{value}` must contain lowercase dotted ASCII segments"
            ),
            Self::EmptyBackendName { backend } => {
                write!(formatter, "backend `{backend}` has an empty display name")
            }
            Self::DuplicateFeature { backend, feature } => write!(
                formatter,
                "backend `{backend}` declares feature `{feature}` more than once"
            ),
            Self::MissingCapability {
                backend,
                feature,
                capability,
            } => write!(
                formatter,
                "backend `{backend}` feature `{feature}` requires missing capability `{capability}`"
            ),
            Self::IncompatibleCapability {
                backend,
                feature,
                capability,
                minimum,
                installed,
            } => write!(
                formatter,
                "backend `{backend}` feature `{feature}` requires capability `{capability}` >= {minimum} with the same major version, but {installed} is installed"
            ),
            Self::CommandOwnerMismatch {
                backend,
                feature,
                command,
                expected,
                actual,
            } => write!(
                formatter,
                "backend `{backend}` feature `{feature}` maps command `{command}` to capability `{expected}`, but registry owner is {actual:?}"
            ),
            Self::EventOwnerMismatch {
                backend,
                feature,
                event,
                expected,
                actual,
            } => write!(
                formatter,
                "backend `{backend}` feature `{feature}` expects event `{event}` to belong to `{expected}`, got {actual:?}"
            ),
        }
    }
}

impl Error for BackendManifestError {}

fn validate_feature_identifier(feature: BackendFeatureId) -> Result<(), BackendManifestError> {
    validate_identifier("backend feature", feature.as_str())
}

fn validate_identifier(
    kind: &'static str,
    value: &'static str,
) -> Result<(), BackendManifestError> {
    let valid = !value.is_empty()
        && value.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        });
    if valid {
        Ok(())
    } else {
        Err(BackendManifestError::InvalidIdentifier { kind, value })
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Future, Ready, ready};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use pwright_core::{
        Capability, CapabilityCommandSpec, CapabilityDescriptor, CapabilityId, CommandId,
        CommandSpec, MethodTag, PluginInfo, ProtocolVersion, RegistryBuilder, Service,
    };

    use super::{
        BackendCheckedService, BackendDescriptor, BackendFeatureDescriptor, BackendFeatureId,
        BackendId, BackendImplementation, BackendManifest, BackendManifestError, BackendProtocolId,
        BackendServiceBindingError, BrowserBackend, BrowserEngine,
    };

    const NAVIGATION_ID: CapabilityId = CapabilityId::new("pwright.navigation");
    const LOCATOR_ID: CapabilityId = CapabilityId::new("pwright.locator");
    const GOTO: CommandId = CommandId::new("navigation.goto");
    const CLICK: CommandId = CommandId::new("locator.click");
    const GOTO_FEATURE: BackendFeatureId = BackendFeatureId::new("navigation.goto");
    const CLICK_FEATURE: BackendFeatureId = BackendFeatureId::new("locator.click");
    const FEATURES: &[BackendFeatureDescriptor] = &[BackendFeatureDescriptor::new(
        GOTO_FEATURE,
        NAVIGATION_ID,
        ProtocolVersion::new(1, 0),
        GOTO,
        BackendImplementation::Native,
    )];

    struct Navigation;

    impl Capability for Navigation {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(NAVIGATION_ID, "Navigation", ProtocolVersion::new(1, 0)),
            &[],
            &[GOTO],
        );
    }

    struct TestBackend;

    impl BrowserBackend for TestBackend {
        const DESCRIPTOR: BackendDescriptor = BackendDescriptor::new(
            BackendId::new("test.browser"),
            "Test browser",
            BrowserEngine::Chromium,
            BackendProtocolId::new("test.protocol"),
            FEATURES,
        );
    }

    const SELECTIVE_FEATURES: &[BackendFeatureDescriptor] = &[
        FEATURES[0],
        BackendFeatureDescriptor::new(
            CLICK_FEATURE,
            LOCATOR_ID,
            ProtocolVersion::new(1, 0),
            CLICK,
            BackendImplementation::Emulated,
        ),
    ];

    struct SelectiveBackend;

    impl BrowserBackend for SelectiveBackend {
        const DESCRIPTOR: BackendDescriptor = BackendDescriptor::new(
            BackendId::new("test.selective-browser"),
            "Selective test browser",
            BrowserEngine::Chromium,
            BackendProtocolId::new("test.protocol"),
            SELECTIVE_FEATURES,
        );
    }

    struct TestGoto;

    impl CommandSpec for TestGoto {
        const METHOD: MethodTag = MethodTag::new(1);
        type Params = &'static str;
        type Result = &'static str;
        type ErrorDetails = Infallible;
    }

    impl CapabilityCommandSpec for TestGoto {
        type Capability = Navigation;
        const COMMAND: CommandId = GOTO;
    }

    struct WrongCommand;

    impl CommandSpec for WrongCommand {
        const METHOD: MethodTag = MethodTag::new(2);
        type Params = &'static str;
        type Result = &'static str;
        type ErrorDetails = Infallible;
    }

    impl CapabilityCommandSpec for WrongCommand {
        type Capability = Navigation;
        const COMMAND: CommandId = CommandId::new("navigation.wrong");
    }

    struct WrongCapability;

    impl Capability for WrongCapability {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(
                CapabilityId::new("pwright.wrong"),
                "Wrong capability",
                ProtocolVersion::new(1, 0),
            ),
            &[],
            &[],
        );
    }

    #[test]
    fn manifest_accepts_features_owned_by_compatible_capabilities() {
        // Arrange
        let mut capabilities = RegistryBuilder::new();
        capabilities
            .register::<Navigation>()
            .expect("navigation should register");
        let capabilities = capabilities.build().expect("registry should validate");

        // Act
        let manifest = BackendManifest::for_backend::<TestBackend>(&capabilities)
            .expect("backend manifest should validate");
        let feature = manifest
            .require(GOTO_FEATURE)
            .expect("goto should be supported");

        // Assert
        assert_eq!(manifest.descriptor().id, BackendId::new("test.browser"));
        assert_eq!(manifest.features().len(), 1);
        assert_eq!(feature.command, GOTO);
        assert_eq!(feature.implementation, BackendImplementation::Native);
        assert!(!manifest.supports(BackendFeatureId::new("locator.click")));
    }

    #[test]
    fn manifest_rejects_a_feature_before_its_capability_is_installed() {
        // Arrange
        let capabilities = RegistryBuilder::new()
            .build()
            .expect("empty registry should validate");

        // Act
        let result = BackendManifest::for_backend::<TestBackend>(&capabilities);

        // Assert
        assert!(matches!(
            result,
            Err(BackendManifestError::MissingCapability {
                feature: GOTO_FEATURE,
                capability: NAVIGATION_ID,
                ..
            })
        ));
    }

    #[test]
    fn installed_capability_manifest_omits_unselected_backend_features() {
        // Arrange
        let mut capabilities = RegistryBuilder::new();
        capabilities
            .register::<Navigation>()
            .expect("navigation should register");
        let capabilities = capabilities.build().expect("registry should validate");

        // Act
        let manifest =
            BackendManifest::for_installed_capabilities::<SelectiveBackend>(&capabilities)
                .expect("selected capability subset should validate");

        // Assert
        assert_eq!(manifest.features().len(), 1);
        assert!(manifest.supports(GOTO_FEATURE));
        assert!(!manifest.supports(CLICK_FEATURE));
    }

    #[test]
    fn manifest_rejects_duplicate_features_and_command_owner_mismatches() {
        // Arrange
        const DUPLICATES: &[BackendFeatureDescriptor] = &[FEATURES[0], FEATURES[0]];
        const WRONG_OWNER: CapabilityId = CapabilityId::new("pwright.wrong");
        const MISMATCH: &[BackendFeatureDescriptor] = &[BackendFeatureDescriptor::new(
            GOTO_FEATURE,
            WRONG_OWNER,
            ProtocolVersion::new(1, 0),
            GOTO,
            BackendImplementation::Emulated,
        )];
        let mut capabilities = RegistryBuilder::new();
        capabilities
            .register::<Navigation>()
            .expect("navigation should register");
        capabilities
            .register::<WrongCapability>()
            .expect("wrong capability should register");
        let capabilities = capabilities.build().expect("registry should validate");
        let duplicate = BackendDescriptor::new(
            BackendId::new("test.duplicate"),
            "Duplicate",
            BrowserEngine::Firefox,
            BackendProtocolId::new("test.protocol"),
            DUPLICATES,
        );
        let mismatch = BackendDescriptor::new(
            BackendId::new("test.mismatch"),
            "Mismatch",
            BrowserEngine::WebKit,
            BackendProtocolId::new("test.protocol"),
            MISMATCH,
        );

        // Act
        let duplicate_result = BackendManifest::build(duplicate, &capabilities);
        let mismatch_result = BackendManifest::build(mismatch, &capabilities);

        // Assert
        assert!(matches!(
            duplicate_result,
            Err(BackendManifestError::DuplicateFeature {
                feature: GOTO_FEATURE,
                ..
            })
        ));
        assert!(matches!(
            mismatch_result,
            Err(BackendManifestError::CommandOwnerMismatch {
                expected: WRONG_OWNER,
                actual: Some(NAVIGATION_ID),
                ..
            })
        ));
    }

    #[test]
    fn missing_feature_is_a_typed_pre_dispatch_failure() {
        // Arrange
        let mut capabilities = RegistryBuilder::new();
        capabilities
            .register::<Navigation>()
            .expect("navigation should register");
        let capabilities = capabilities.build().expect("registry should validate");
        let manifest = BackendManifest::for_backend::<TestBackend>(&capabilities)
            .expect("backend manifest should validate");
        let missing = BackendFeatureId::new("locator.fill");

        // Act
        let result = manifest.require(missing);

        // Assert
        let error = result.expect_err("missing feature should fail");
        assert_eq!(error.backend, BackendId::new("test.browser"));
        assert_eq!(error.feature, missing);
    }

    #[derive(Clone, Debug)]
    struct CountingService {
        calls: Arc<AtomicUsize>,
    }

    impl Service<&'static str> for CountingService {
        type Response = &'static str;
        type Error = Infallible;
        type Future<'a>
            = Ready<Result<Self::Response, Self::Error>>
        where
            Self: 'a;

        fn call(&self, request: &'static str) -> Self::Future<'_> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ready(Ok(request))
        }
    }

    fn test_manifest() -> BackendManifest {
        let mut capabilities = RegistryBuilder::new();
        capabilities
            .register::<Navigation>()
            .expect("navigation should register");
        let capabilities = capabilities.build().expect("registry should validate");
        BackendManifest::for_backend::<TestBackend>(&capabilities)
            .expect("backend manifest should validate")
    }

    #[test]
    fn checked_service_delegates_a_supported_feature() {
        // Arrange
        let calls = Arc::new(AtomicUsize::new(0));
        let service = BackendCheckedService::<TestGoto, _>::try_new(
            CountingService {
                calls: Arc::clone(&calls),
            },
            Arc::new(test_manifest()),
            GOTO_FEATURE,
        )
        .expect("typed goto binding should validate");

        // Act
        let result = poll_ready(service.call("request"));

        // Assert
        assert_eq!(result, Ok("request"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn checked_service_rejects_an_unsupported_feature_before_inner_call() {
        // Arrange
        let calls = Arc::new(AtomicUsize::new(0));
        let missing = BackendFeatureId::new("locator.click");
        let binding = BackendCheckedService::<TestGoto, _>::try_new(
            CountingService {
                calls: Arc::clone(&calls),
            },
            Arc::new(test_manifest()),
            missing,
        );

        // Act
        let result = binding;

        // Assert
        assert!(matches!(
            result,
            Err(BackendServiceBindingError::Unsupported(error)) if error.feature == missing
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn checked_service_rejects_a_typed_command_feature_mismatch() {
        // Arrange
        let calls = Arc::new(AtomicUsize::new(0));
        let inner = CountingService {
            calls: Arc::clone(&calls),
        };

        // Act
        let result = BackendCheckedService::<WrongCommand, _>::try_new(
            inner,
            Arc::new(test_manifest()),
            GOTO_FEATURE,
        );

        // Assert
        assert!(matches!(
            result,
            Err(BackendServiceBindingError::CommandMismatch {
                feature: GOTO_FEATURE,
                expected,
                declared: GOTO,
            }) if expected == CommandId::new("navigation.wrong")
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    fn poll_ready<F: Future>(future: F) -> F::Output {
        use std::task::{Context, Poll, Waker};

        let mut context = Context::from_waker(Waker::noop());
        let mut future = std::pin::pin!(future);
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("test future should complete immediately"),
        }
    }
}
