use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_core::ProtocolVersion;

use crate::{FrameLimits, VersionPolicy};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NegotiationLimits {
    pub capabilities: usize,
    pub capability_id_bytes: usize,
}

impl NegotiationLimits {
    pub const fn new(capabilities: usize, capability_id_bytes: usize) -> Self {
        Self {
            capabilities,
            capability_id_bytes,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WireCapabilityId(String);

impl WireCapabilityId {
    pub fn new(value: impl Into<String>, max_bytes: usize) -> Result<Self, CapabilityIdError> {
        let value = value.into();
        Self::validate_value(&value, max_bytes)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(&self, max_bytes: usize) -> Result<(), CapabilityIdError> {
        Self::validate_value(&self.0, max_bytes)
    }

    fn validate_value(value: &str, max_bytes: usize) -> Result<(), CapabilityIdError> {
        if value.is_empty() {
            return Err(CapabilityIdError::Empty);
        }
        if value.len() > max_bytes {
            return Err(CapabilityIdError::TooLong {
                actual: value.len(),
                limit: max_bytes,
            });
        }
        if let Some((index, byte)) = value
            .bytes()
            .enumerate()
            .find(|(_, byte)| !byte.is_ascii_graphic())
        {
            return Err(CapabilityIdError::InvalidByte { index, byte });
        }
        Ok(())
    }
}

impl Display for WireCapabilityId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityIdError {
    Empty,
    TooLong { actual: usize, limit: usize },
    InvalidByte { index: usize, byte: u8 },
}

impl Display for CapabilityIdError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("wire capability ID is empty"),
            Self::TooLong { actual, limit } => write!(
                formatter,
                "wire capability ID has {actual} bytes, exceeding limit {limit}"
            ),
            Self::InvalidByte { index, byte } => write!(
                formatter,
                "wire capability ID contains invalid byte 0x{byte:02x} at index {index}"
            ),
        }
    }
}

impl Error for CapabilityIdError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireCapability {
    id: WireCapabilityId,
    version: ProtocolVersion,
}

impl WireCapability {
    pub const fn new(id: WireCapabilityId, version: ProtocolVersion) -> Self {
        Self { id, version }
    }

    pub const fn id(&self) -> &WireCapabilityId {
        &self.id
    }

    pub const fn version(&self) -> ProtocolVersion {
        self.version
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilitySetError {
    TooManyCapabilities { actual: usize, limit: usize },
    InvalidId(CapabilityIdError),
    DuplicateCapability { id: WireCapabilityId },
}

impl Display for CapabilitySetError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyCapabilities { actual, limit } => write!(
                formatter,
                "capability set has {actual} entries, exceeding limit {limit}"
            ),
            Self::InvalidId(error) => Display::fmt(error, formatter),
            Self::DuplicateCapability { id } => {
                write!(formatter, "wire capability `{id}` appears more than once")
            }
        }
    }
}

impl Error for CapabilitySetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidId(error) => Some(error),
            Self::TooManyCapabilities { .. } | Self::DuplicateCapability { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilitySupport {
    id: WireCapabilityId,
    versions: VersionPolicy,
}

impl CapabilitySupport {
    pub const fn new(id: WireCapabilityId, versions: VersionPolicy) -> Self {
        Self { id, versions }
    }

    pub const fn id(&self) -> &WireCapabilityId {
        &self.id
    }

    pub const fn versions(&self) -> VersionPolicy {
        self.versions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityManifest {
    capabilities: BTreeMap<WireCapabilityId, BTreeMap<u16, VersionPolicy>>,
}

impl CapabilityManifest {
    pub fn new(
        capabilities: Vec<WireCapability>,
        limits: NegotiationLimits,
    ) -> Result<Self, CapabilitySetError> {
        let support = capabilities
            .into_iter()
            .map(|capability| {
                CapabilitySupport::new(
                    capability.id,
                    VersionPolicy::compatible_through(capability.version),
                )
            })
            .collect();
        Self::from_support(support, limits)
    }

    pub fn from_support(
        support: Vec<CapabilitySupport>,
        limits: NegotiationLimits,
    ) -> Result<Self, CapabilitySetError> {
        if support.len() > limits.capabilities {
            return Err(CapabilitySetError::TooManyCapabilities {
                actual: support.len(),
                limit: limits.capabilities,
            });
        }
        let mut indexed: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
        for capability in support {
            capability
                .id
                .validate(limits.capability_id_bytes)
                .map_err(CapabilitySetError::InvalidId)?;
            let major = capability.versions.minimum().major;
            if indexed
                .entry(capability.id.clone())
                .or_default()
                .insert(major, capability.versions)
                .is_some()
            {
                return Err(CapabilitySetError::DuplicateCapability { id: capability.id });
            }
        }
        Ok(Self {
            capabilities: indexed,
        })
    }

    pub fn get(&self, id: &WireCapabilityId) -> Option<ProtocolVersion> {
        self.support(id).map(|versions| versions.maximum())
    }

    /// Returns the newest installed major line for a capability.
    ///
    /// Use [`Self::support_major`] during negotiation, where the requester has
    /// already selected exactly one major line.
    pub fn support(&self, id: &WireCapabilityId) -> Option<VersionPolicy> {
        self.capabilities
            .get(id)
            .and_then(|tracks| tracks.last_key_value())
            .map(|(_, versions)| *versions)
    }

    /// Returns one installed major-version track without falling across a
    /// wire-incompatible major boundary.
    pub fn support_major(&self, id: &WireCapabilityId, major: u16) -> Option<VersionPolicy> {
        self.capabilities
            .get(id)
            .and_then(|tracks| tracks.get(&major))
            .copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&WireCapabilityId, ProtocolVersion)> {
        self.capabilities.iter().flat_map(|(id, tracks)| {
            tracks
                .values()
                .map(move |versions| (id, versions.maximum()))
        })
    }

    pub fn supported_ranges(&self) -> impl Iterator<Item = (&WireCapabilityId, VersionPolicy)> {
        self.capabilities
            .iter()
            .flat_map(|(id, tracks)| tracks.values().map(move |versions| (id, *versions)))
    }

    pub fn len(&self) -> usize {
        self.capabilities.values().map(BTreeMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityRequirement {
    id: WireCapabilityId,
    versions: VersionPolicy,
}

impl CapabilityRequirement {
    pub const fn new(id: WireCapabilityId, minimum_version: ProtocolVersion) -> Self {
        Self {
            id,
            versions: VersionPolicy::compatible_from(minimum_version),
        }
    }

    pub const fn within(id: WireCapabilityId, versions: VersionPolicy) -> Self {
        Self { id, versions }
    }

    pub const fn id(&self) -> &WireCapabilityId {
        &self.id
    }

    pub const fn minimum_version(&self) -> ProtocolVersion {
        self.versions.minimum()
    }

    pub const fn maximum_version(&self) -> ProtocolVersion {
        self.versions.maximum()
    }

    pub const fn versions(&self) -> VersionPolicy {
        self.versions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityRequest {
    required: BTreeMap<WireCapabilityId, VersionPolicy>,
    optional: BTreeMap<WireCapabilityId, VersionPolicy>,
}

impl CapabilityRequest {
    pub fn new(
        required: Vec<CapabilityRequirement>,
        optional: Vec<CapabilityRequirement>,
        limits: NegotiationLimits,
    ) -> Result<Self, CapabilitySetError> {
        let actual = required.len().saturating_add(optional.len());
        if actual > limits.capabilities {
            return Err(CapabilitySetError::TooManyCapabilities {
                actual,
                limit: limits.capabilities,
            });
        }

        let mut required_index = BTreeMap::new();
        let mut optional_index = BTreeMap::new();
        for requirement in required {
            requirement
                .id
                .validate(limits.capability_id_bytes)
                .map_err(CapabilitySetError::InvalidId)?;
            if required_index
                .insert(requirement.id.clone(), requirement.versions)
                .is_some()
            {
                return Err(CapabilitySetError::DuplicateCapability { id: requirement.id });
            }
        }
        for requirement in optional {
            requirement
                .id
                .validate(limits.capability_id_bytes)
                .map_err(CapabilitySetError::InvalidId)?;
            if required_index.contains_key(&requirement.id)
                || optional_index
                    .insert(requirement.id.clone(), requirement.versions)
                    .is_some()
            {
                return Err(CapabilitySetError::DuplicateCapability { id: requirement.id });
            }
        }
        Ok(Self {
            required: required_index,
            optional: optional_index,
        })
    }

    pub fn required(&self) -> impl Iterator<Item = (&WireCapabilityId, ProtocolVersion)> {
        self.required
            .iter()
            .map(|(id, versions)| (id, versions.minimum()))
    }

    pub fn optional(&self) -> impl Iterator<Item = (&WireCapabilityId, ProtocolVersion)> {
        self.optional
            .iter()
            .map(|(id, versions)| (id, versions.minimum()))
    }

    pub fn required_ranges(&self) -> impl Iterator<Item = (&WireCapabilityId, VersionPolicy)> {
        self.required.iter().map(|(id, versions)| (id, *versions))
    }

    pub fn optional_ranges(&self) -> impl Iterator<Item = (&WireCapabilityId, VersionPolicy)> {
        self.optional.iter().map(|(id, versions)| (id, *versions))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientHello {
    versions: VersionPolicy,
    receive_limits: FrameLimits,
    capabilities: CapabilityRequest,
}

impl ClientHello {
    pub const fn new(
        versions: VersionPolicy,
        receive_limits: FrameLimits,
        capabilities: CapabilityRequest,
    ) -> Self {
        Self {
            versions,
            receive_limits,
            capabilities,
        }
    }

    pub const fn versions(&self) -> VersionPolicy {
        self.versions
    }

    pub const fn receive_limits(&self) -> FrameLimits {
        self.receive_limits
    }

    pub const fn capabilities(&self) -> &CapabilityRequest {
        &self.capabilities
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerHandshakeConfig {
    versions: VersionPolicy,
    receive_limits: FrameLimits,
    capabilities: CapabilityManifest,
}

impl ServerHandshakeConfig {
    pub const fn new(
        versions: VersionPolicy,
        receive_limits: FrameLimits,
        capabilities: CapabilityManifest,
    ) -> Self {
        Self {
            versions,
            receive_limits,
            capabilities,
        }
    }

    pub const fn versions(&self) -> VersionPolicy {
        self.versions
    }

    pub const fn receive_limits(&self) -> FrameLimits {
        self.receive_limits
    }

    pub const fn capabilities(&self) -> &CapabilityManifest {
        &self.capabilities
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerHello {
    selected_version: ProtocolVersion,
    client_receive_limits: FrameLimits,
    server_receive_limits: FrameLimits,
    enabled_capabilities: Box<[WireCapability]>,
}

impl ServerHello {
    pub fn new(
        selected_version: ProtocolVersion,
        client_receive_limits: FrameLimits,
        server_receive_limits: FrameLimits,
        enabled_capabilities: Vec<WireCapability>,
        limits: NegotiationLimits,
    ) -> Result<Self, CapabilitySetError> {
        if enabled_capabilities.len() > limits.capabilities {
            return Err(CapabilitySetError::TooManyCapabilities {
                actual: enabled_capabilities.len(),
                limit: limits.capabilities,
            });
        }
        let mut indexed = BTreeMap::new();
        for capability in enabled_capabilities {
            capability
                .id
                .validate(limits.capability_id_bytes)
                .map_err(CapabilitySetError::InvalidId)?;
            if indexed
                .insert(capability.id.clone(), capability.version)
                .is_some()
            {
                return Err(CapabilitySetError::DuplicateCapability { id: capability.id });
            }
        }
        let enabled_capabilities = indexed
            .into_iter()
            .map(|(id, version)| WireCapability::new(id, version))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Ok(Self {
            selected_version,
            client_receive_limits,
            server_receive_limits,
            enabled_capabilities,
        })
    }

    pub const fn selected_version(&self) -> ProtocolVersion {
        self.selected_version
    }

    pub const fn client_receive_limits(&self) -> FrameLimits {
        self.client_receive_limits
    }

    pub const fn server_receive_limits(&self) -> FrameLimits {
        self.server_receive_limits
    }

    pub fn enabled_capabilities(&self) -> &[WireCapability] {
        &self.enabled_capabilities
    }

    /// Looks up the exact capability version selected for this connection.
    ///
    /// The list is canonical by ID and contains at most one major track for
    /// each capability, so connection setup can choose its codec and handler
    /// table before decoding any application frame.
    pub fn enabled_capability_version(&self, id: &WireCapabilityId) -> Option<ProtocolVersion> {
        self.enabled_capabilities
            .binary_search_by(|capability| capability.id.cmp(id))
            .ok()
            .map(|index| self.enabled_capabilities[index].version)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NegotiationError {
    NoCommonVersion {
        client_minimum: ProtocolVersion,
        client_maximum: ProtocolVersion,
        server_minimum: ProtocolVersion,
        server_maximum: ProtocolVersion,
    },
    MissingRequiredCapability {
        id: WireCapabilityId,
    },
    IncompatibleRequiredCapability {
        id: WireCapabilityId,
        client: VersionPolicy,
        server: VersionPolicy,
    },
}

impl Display for NegotiationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCommonVersion {
                client_minimum,
                client_maximum,
                server_minimum,
                server_maximum,
            } => write!(
                formatter,
                "no common protocol version between client {client_minimum}..={client_maximum} and server {server_minimum}..={server_maximum}"
            ),
            Self::MissingRequiredCapability { id } => {
                write!(formatter, "required capability `{id}` is not installed")
            }
            Self::IncompatibleRequiredCapability { id, client, server } => write!(
                formatter,
                "required capability `{id}` has no common version between client {}..={} and server {}..={}",
                client.minimum(),
                client.maximum(),
                server.minimum(),
                server.maximum(),
            ),
        }
    }
}

impl Error for NegotiationError {}

/// Rejects a `ServerHello` that could not have resulted from the client's
/// request, even when its binary representation is otherwise well formed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClientHelloValidationError {
    UnsupportedSelectedVersion {
        selected: ProtocolVersion,
        minimum: ProtocolVersion,
        maximum: ProtocolVersion,
    },
    ClientReceiveLimitsMismatch {
        advertised: FrameLimits,
        returned: FrameLimits,
    },
    MissingRequiredCapability {
        id: WireCapabilityId,
    },
    UnrequestedCapability {
        id: WireCapabilityId,
    },
    IncompatibleEnabledCapability {
        id: WireCapabilityId,
        minimum: ProtocolVersion,
        maximum: ProtocolVersion,
        enabled: ProtocolVersion,
    },
}

impl Display for ClientHelloValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSelectedVersion {
                selected,
                minimum,
                maximum,
            } => write!(
                formatter,
                "server selected protocol version {selected} outside requested range {minimum}..={maximum}"
            ),
            Self::ClientReceiveLimitsMismatch {
                advertised,
                returned,
            } => write!(
                formatter,
                "server returned client receive limits {returned:?}, but client advertised {advertised:?}"
            ),
            Self::MissingRequiredCapability { id } => {
                write!(formatter, "server omitted required capability `{id}`")
            }
            Self::UnrequestedCapability { id } => {
                write!(formatter, "server enabled unrequested capability `{id}`")
            }
            Self::IncompatibleEnabledCapability {
                id,
                minimum,
                maximum,
                enabled,
            } => write!(
                formatter,
                "server enabled capability `{id}` at {enabled}, outside requested range {minimum}..={maximum}"
            ),
        }
    }
}

impl Error for ClientHelloValidationError {}

pub fn validate_server_hello(
    client: &ClientHello,
    server: &ServerHello,
) -> Result<(), ClientHelloValidationError> {
    if client.versions.validate(server.selected_version).is_err() {
        return Err(ClientHelloValidationError::UnsupportedSelectedVersion {
            selected: server.selected_version,
            minimum: client.versions.minimum(),
            maximum: client.versions.maximum(),
        });
    }
    if server.client_receive_limits != client.receive_limits {
        return Err(ClientHelloValidationError::ClientReceiveLimitsMismatch {
            advertised: client.receive_limits,
            returned: server.client_receive_limits,
        });
    }

    let mut enabled = BTreeMap::new();
    for capability in &server.enabled_capabilities {
        let versions = client
            .capabilities
            .required
            .get(&capability.id)
            .or_else(|| client.capabilities.optional.get(&capability.id))
            .copied()
            .ok_or_else(|| ClientHelloValidationError::UnrequestedCapability {
                id: capability.id.clone(),
            })?;
        if versions.validate(capability.version).is_err() {
            return Err(ClientHelloValidationError::IncompatibleEnabledCapability {
                id: capability.id.clone(),
                minimum: versions.minimum(),
                maximum: versions.maximum(),
                enabled: capability.version,
            });
        }
        enabled.insert(capability.id.clone(), capability.version);
    }
    for id in client.capabilities.required.keys() {
        if !enabled.contains_key(id) {
            return Err(ClientHelloValidationError::MissingRequiredCapability { id: id.clone() });
        }
    }
    Ok(())
}

pub fn negotiate(
    client: &ClientHello,
    server: &ServerHandshakeConfig,
) -> Result<ServerHello, NegotiationError> {
    let selected_version = negotiate_version(client.versions, server.versions)?;
    let mut enabled = Vec::new();

    for (id, client_versions) in client.capabilities.required_ranges() {
        let Some(server_versions) = server
            .capabilities
            .support_major(id, client_versions.minimum().major)
        else {
            let Some(server_versions) = server.capabilities.support(id) else {
                return Err(NegotiationError::MissingRequiredCapability { id: id.clone() });
            };
            return Err(NegotiationError::IncompatibleRequiredCapability {
                id: id.clone(),
                client: client_versions,
                server: server_versions,
            });
        };
        let Some(selected) = highest_common_version(client_versions, server_versions) else {
            return Err(NegotiationError::IncompatibleRequiredCapability {
                id: id.clone(),
                client: client_versions,
                server: server_versions,
            });
        };
        enabled.push(WireCapability::new(id.clone(), selected));
    }
    for (id, client_versions) in client.capabilities.optional_ranges() {
        if let Some(server_versions) = server
            .capabilities
            .support_major(id, client_versions.minimum().major)
            && let Some(selected) = highest_common_version(client_versions, server_versions)
        {
            enabled.push(WireCapability::new(id.clone(), selected));
        }
    }
    enabled.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(ServerHello {
        selected_version,
        client_receive_limits: client.receive_limits,
        server_receive_limits: server.receive_limits,
        enabled_capabilities: enabled.into_boxed_slice(),
    })
}

fn negotiate_version(
    client: VersionPolicy,
    server: VersionPolicy,
) -> Result<ProtocolVersion, NegotiationError> {
    let client_minimum = client.minimum();
    let client_maximum = client.maximum();
    let server_minimum = server.minimum();
    let server_maximum = server.maximum();
    if let Some(selected) = highest_common_version(client, server) {
        return Ok(selected);
    }
    Err(NegotiationError::NoCommonVersion {
        client_minimum,
        client_maximum,
        server_minimum,
        server_maximum,
    })
}

fn highest_common_version(left: VersionPolicy, right: VersionPolicy) -> Option<ProtocolVersion> {
    if left.minimum().major != right.minimum().major {
        return None;
    }
    let minimum_minor = left.minimum().minor.max(right.minimum().minor);
    let maximum_minor = left.maximum().minor.min(right.maximum().minor);
    (minimum_minor <= maximum_minor)
        .then(|| ProtocolVersion::new(left.minimum().major, maximum_minor))
}

#[cfg(test)]
mod tests {
    use pwright_core::ProtocolVersion;

    use crate::{FrameLimits, VersionPolicy};

    use super::{
        CapabilityIdError, CapabilityManifest, CapabilityRequest, CapabilityRequirement,
        CapabilitySetError, CapabilitySupport, ClientHello, ClientHelloValidationError,
        NegotiationError, NegotiationLimits, ServerHandshakeConfig, ServerHello, WireCapability,
        WireCapabilityId, negotiate, validate_server_hello,
    };

    fn id(value: &str) -> WireCapabilityId {
        WireCapabilityId::new(value, 64).expect("test capability ID should be valid")
    }

    fn versions(minimum: u16, maximum: u16) -> VersionPolicy {
        VersionPolicy::new(
            ProtocolVersion::new(1, minimum),
            ProtocolVersion::new(1, maximum),
        )
        .expect("test version range should be valid")
    }

    fn limits() -> NegotiationLimits {
        NegotiationLimits::new(8, 64)
    }

    fn frames(value: usize) -> FrameLimits {
        FrameLimits::new(value, value, value, value)
    }

    fn manifest(capabilities: &[(&str, ProtocolVersion)]) -> CapabilityManifest {
        CapabilityManifest::new(
            capabilities
                .iter()
                .map(|(name, version)| WireCapability::new(id(name), *version))
                .collect(),
            limits(),
        )
        .expect("test manifest should build")
    }

    fn request(
        required: &[(&str, ProtocolVersion)],
        optional: &[(&str, ProtocolVersion)],
    ) -> CapabilityRequest {
        CapabilityRequest::new(
            required
                .iter()
                .map(|(name, version)| CapabilityRequirement::new(id(name), *version))
                .collect(),
            optional
                .iter()
                .map(|(name, version)| CapabilityRequirement::new(id(name), *version))
                .collect(),
            limits(),
        )
        .expect("test request should build")
    }

    #[test]
    fn wire_capability_id_rejects_empty_long_and_non_graphic_values() {
        // Arrange
        let long = "navigation";

        // Act
        let empty = WireCapabilityId::new("", 8);
        let too_long = WireCapabilityId::new(long, 3);
        let whitespace = WireCapabilityId::new("nav igation", 16);

        // Assert
        assert_eq!(empty, Err(CapabilityIdError::Empty));
        assert_eq!(
            too_long,
            Err(CapabilityIdError::TooLong {
                actual: long.len(),
                limit: 3,
            })
        );
        assert_eq!(
            whitespace,
            Err(CapabilityIdError::InvalidByte {
                index: 3,
                byte: b' ',
            })
        );
    }

    #[test]
    fn manifest_revalidates_count_ids_and_uniqueness() {
        // Arrange
        let navigation = WireCapability::new(id("navigation"), ProtocolVersion::new(1, 0));

        // Act
        let too_many =
            CapabilityManifest::new(vec![navigation.clone()], NegotiationLimits::new(0, 64));
        let too_long =
            CapabilityManifest::new(vec![navigation.clone()], NegotiationLimits::new(1, 3));
        let duplicate = CapabilityManifest::new(
            vec![navigation.clone(), navigation],
            NegotiationLimits::new(2, 64),
        );

        // Assert
        assert_eq!(
            too_many,
            Err(CapabilitySetError::TooManyCapabilities {
                actual: 1,
                limit: 0,
            })
        );
        assert!(matches!(
            too_long,
            Err(CapabilitySetError::InvalidId(
                CapabilityIdError::TooLong { .. }
            ))
        ));
        assert_eq!(
            duplicate,
            Err(CapabilitySetError::DuplicateCapability {
                id: id("navigation")
            })
        );
    }

    #[test]
    fn manifest_keeps_distinct_major_tracks_for_one_capability() {
        // Arrange
        let locator_v1 = CapabilitySupport::new(
            id("pwright.locator"),
            VersionPolicy::new(ProtocolVersion::new(1, 3), ProtocolVersion::new(1, 17))
                .expect("v1 range should build"),
        );
        let locator_v2 = CapabilitySupport::new(
            id("pwright.locator"),
            VersionPolicy::new(ProtocolVersion::new(2, 0), ProtocolVersion::new(2, 4))
                .expect("v2 range should build"),
        );

        // Act
        let manifest = CapabilityManifest::from_support(
            vec![locator_v2.clone(), locator_v1.clone()],
            limits(),
        )
        .expect("distinct major tracks should coexist");

        // Assert
        assert_eq!(manifest.len(), 2);
        assert_eq!(
            manifest.support_major(locator_v1.id(), 1),
            Some(locator_v1.versions())
        );
        assert_eq!(
            manifest.support_major(locator_v2.id(), 2),
            Some(locator_v2.versions())
        );
        assert_eq!(
            manifest.support(locator_v1.id()),
            Some(locator_v2.versions())
        );
        assert_eq!(
            manifest
                .supported_ranges()
                .map(|(_, versions)| versions)
                .collect::<Vec<_>>(),
            vec![locator_v1.versions(), locator_v2.versions()]
        );
    }

    #[test]
    fn server_hello_rejects_two_selected_majors_for_one_capability() {
        // Arrange
        let enabled = vec![
            WireCapability::new(id("pwright.locator"), ProtocolVersion::new(1, 17)),
            WireCapability::new(id("pwright.locator"), ProtocolVersion::new(2, 0)),
        ];

        // Act
        let result = ServerHello::new(
            ProtocolVersion::new(1, 0),
            frames(10),
            frames(20),
            enabled,
            limits(),
        );

        // Assert
        assert_eq!(
            result,
            Err(CapabilitySetError::DuplicateCapability {
                id: id("pwright.locator")
            })
        );
    }

    #[test]
    fn server_hello_exposes_the_exact_selected_track_for_connection_setup() {
        // Arrange
        let locator = id("pwright.locator");
        let hello = ServerHello::new(
            ProtocolVersion::new(1, 0),
            frames(10),
            frames(20),
            vec![
                WireCapability::new(id("pwright.navigation"), ProtocolVersion::new(1, 8)),
                WireCapability::new(locator.clone(), ProtocolVersion::new(2, 3)),
            ],
            limits(),
        )
        .expect("server hello should build");

        // Act
        let selected = hello.enabled_capability_version(&locator);
        let absent = hello.enabled_capability_version(&id("pwright.tracing"));

        // Assert
        assert_eq!(selected, Some(ProtocolVersion::new(2, 3)));
        assert_eq!(absent, None);
    }

    #[test]
    fn request_rejects_duplicate_across_required_and_optional_sets() {
        // Arrange
        let required = CapabilityRequirement::new(id("navigation"), ProtocolVersion::new(1, 0));
        let optional = CapabilityRequirement::new(id("navigation"), ProtocolVersion::new(1, 1));

        // Act
        let error = CapabilityRequest::new(vec![required], vec![optional], limits())
            .expect_err("cross-set duplicate should fail");

        // Assert
        assert_eq!(
            error,
            CapabilitySetError::DuplicateCapability {
                id: id("navigation")
            }
        );
    }

    #[test]
    fn negotiation_selects_highest_common_minor_and_preserves_directional_limits() {
        // Arrange
        let client_receive = frames(10);
        let server_receive = frames(20);
        let client = ClientHello::new(
            versions(1, 4),
            client_receive,
            request(&[("navigation", ProtocolVersion::new(1, 0))], &[]),
        );
        let server = ServerHandshakeConfig::new(
            versions(2, 3),
            server_receive,
            manifest(&[("navigation", ProtocolVersion::new(1, 2))]),
        );

        // Act
        let hello = negotiate(&client, &server).expect("peers should negotiate");

        // Assert
        assert_eq!(hello.selected_version(), ProtocolVersion::new(1, 3));
        assert_eq!(hello.client_receive_limits(), client_receive);
        assert_eq!(hello.server_receive_limits(), server_receive);
    }

    #[test]
    fn capability_negotiation_selects_the_highest_version_inside_both_ranges() {
        // Arrange
        let client = ClientHello::new(
            versions(0, 0),
            frames(10),
            CapabilityRequest::new(
                vec![CapabilityRequirement::within(
                    id("navigation"),
                    versions(1, 2),
                )],
                Vec::new(),
                limits(),
            )
            .expect("ranged request should build"),
        );
        let server = ServerHandshakeConfig::new(
            versions(0, 0),
            frames(20),
            CapabilityManifest::from_support(
                vec![CapabilitySupport::new(id("navigation"), versions(0, 4))],
                limits(),
            )
            .expect("ranged manifest should build"),
        );

        // Act
        let hello = negotiate(&client, &server).expect("ranges should overlap");

        // Assert
        assert_eq!(
            hello.enabled_capabilities(),
            &[WireCapability::new(
                id("navigation"),
                ProtocolVersion::new(1, 2)
            )]
        );
    }

    #[test]
    fn capability_negotiation_selects_only_the_requested_major_track() {
        // Arrange
        let client = ClientHello::new(
            versions(0, 0),
            frames(10),
            CapabilityRequest::new(
                vec![CapabilityRequirement::within(
                    id("pwright.locator"),
                    VersionPolicy::new(ProtocolVersion::new(1, 10), ProtocolVersion::new(1, 20))
                        .expect("client range should build"),
                )],
                Vec::new(),
                limits(),
            )
            .expect("request should build"),
        );
        let server = ServerHandshakeConfig::new(
            versions(0, 0),
            frames(20),
            CapabilityManifest::from_support(
                vec![
                    CapabilitySupport::new(
                        id("pwright.locator"),
                        VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 17))
                            .expect("server v1 range should build"),
                    ),
                    CapabilitySupport::new(
                        id("pwright.locator"),
                        VersionPolicy::new(ProtocolVersion::new(2, 0), ProtocolVersion::new(2, 4))
                            .expect("server v2 range should build"),
                    ),
                ],
                limits(),
            )
            .expect("multi-track manifest should build"),
        );

        // Act
        let hello = negotiate(&client, &server).expect("v1 track should negotiate");

        // Assert
        assert_eq!(
            hello.enabled_capabilities(),
            &[WireCapability::new(
                id("pwright.locator"),
                ProtocolVersion::new(1, 17)
            )]
        );
    }

    #[test]
    fn required_capability_with_disjoint_ranges_rejects_negotiation() {
        // Arrange
        let client_range = versions(0, 1);
        let server_range = versions(2, 4);
        let client = ClientHello::new(
            versions(0, 0),
            frames(10),
            CapabilityRequest::new(
                vec![CapabilityRequirement::within(
                    id("navigation"),
                    client_range,
                )],
                Vec::new(),
                limits(),
            )
            .expect("ranged request should build"),
        );
        let server = ServerHandshakeConfig::new(
            versions(0, 0),
            frames(20),
            CapabilityManifest::from_support(
                vec![CapabilitySupport::new(id("navigation"), server_range)],
                limits(),
            )
            .expect("ranged manifest should build"),
        );

        // Act
        let error = negotiate(&client, &server).expect_err("disjoint requirement must fail");

        // Assert
        assert_eq!(
            error,
            NegotiationError::IncompatibleRequiredCapability {
                id: id("navigation"),
                client: client_range,
                server: server_range,
            }
        );
    }

    #[test]
    fn optional_capability_with_disjoint_ranges_is_omitted() {
        // Arrange
        let client = ClientHello::new(
            versions(0, 0),
            frames(10),
            CapabilityRequest::new(
                Vec::new(),
                vec![CapabilityRequirement::within(id("tracing"), versions(0, 1))],
                limits(),
            )
            .expect("optional ranged request should build"),
        );
        let server = ServerHandshakeConfig::new(
            versions(0, 0),
            frames(20),
            CapabilityManifest::from_support(
                vec![CapabilitySupport::new(id("tracing"), versions(2, 3))],
                limits(),
            )
            .expect("ranged manifest should build"),
        );

        // Act
        let hello = negotiate(&client, &server).expect("optional mismatch should not fail");

        // Assert
        assert!(hello.enabled_capabilities().is_empty());
    }

    #[test]
    fn client_rejects_enabled_capability_above_its_maximum() {
        // Arrange
        let client = ClientHello::new(
            versions(1, 2),
            frames(10),
            CapabilityRequest::new(
                vec![CapabilityRequirement::within(
                    id("navigation"),
                    versions(0, 1),
                )],
                Vec::new(),
                limits(),
            )
            .expect("bounded request should build"),
        );
        let hello = ServerHello::new(
            ProtocolVersion::new(1, 2),
            frames(10),
            frames(20),
            vec![WireCapability::new(
                id("navigation"),
                ProtocolVersion::new(1, 2),
            )],
            limits(),
        )
        .expect("synthetic server hello should build");

        // Act
        let error = validate_server_hello(&client, &hello)
            .expect_err("server selection above client maximum must fail");

        // Assert
        assert!(matches!(
            error,
            ClientHelloValidationError::IncompatibleEnabledCapability {
                minimum,
                maximum,
                enabled,
                ..
            } if minimum == ProtocolVersion::new(1, 0)
                && maximum == ProtocolVersion::new(1, 1)
                && enabled == ProtocolVersion::new(1, 2)
        ));
    }

    #[test]
    fn client_accepts_a_server_hello_derived_from_its_request() {
        // Arrange
        let client = ClientHello::new(
            versions(1, 4),
            frames(10),
            request(
                &[("navigation", ProtocolVersion::new(1, 0))],
                &[("tracing", ProtocolVersion::new(1, 0))],
            ),
        );
        let server = ServerHandshakeConfig::new(
            versions(2, 3),
            frames(20),
            manifest(&[
                ("navigation", ProtocolVersion::new(1, 2)),
                ("tracing", ProtocolVersion::new(1, 1)),
            ]),
        );
        let hello = negotiate(&client, &server).expect("server hello should negotiate");

        // Act
        let validation = validate_server_hello(&client, &hello);

        // Assert
        assert_eq!(validation, Ok(()));
    }

    #[test]
    fn client_rejects_a_server_selected_version_outside_its_request() {
        // Arrange
        let client = ClientHello::new(versions(1, 2), frames(10), request(&[], &[]));
        let hello = ServerHello::new(
            ProtocolVersion::new(1, 3),
            frames(10),
            frames(20),
            Vec::new(),
            limits(),
        )
        .expect("synthetic server hello should build");

        // Act
        let error = validate_server_hello(&client, &hello)
            .expect_err("unrequested selected version must fail");

        // Assert
        assert!(matches!(
            error,
            ClientHelloValidationError::UnsupportedSelectedVersion { .. }
        ));
    }

    #[test]
    fn client_rejects_server_hello_that_changes_its_receive_limits() {
        // Arrange
        let client = ClientHello::new(versions(1, 2), frames(10), request(&[], &[]));
        let hello = ServerHello::new(
            ProtocolVersion::new(1, 2),
            frames(11),
            frames(20),
            Vec::new(),
            limits(),
        )
        .expect("synthetic server hello should build");

        // Act
        let error = validate_server_hello(&client, &hello)
            .expect_err("changed client receive limits must fail");

        // Assert
        assert!(matches!(
            error,
            ClientHelloValidationError::ClientReceiveLimitsMismatch { .. }
        ));
    }

    #[test]
    fn client_rejects_a_missing_required_capability() {
        // Arrange
        let client = ClientHello::new(
            versions(1, 2),
            frames(10),
            request(&[("navigation", ProtocolVersion::new(1, 0))], &[]),
        );
        let missing = ServerHello::new(
            ProtocolVersion::new(1, 2),
            frames(10),
            frames(20),
            Vec::new(),
            limits(),
        )
        .expect("synthetic missing-capability hello should build");
        // Act
        let error = validate_server_hello(&client, &missing)
            .expect_err("missing required capability must fail");

        // Assert
        assert!(matches!(
            error,
            ClientHelloValidationError::MissingRequiredCapability { .. }
        ));
    }

    #[test]
    fn client_rejects_an_unrequested_capability() {
        // Arrange
        let client = ClientHello::new(versions(1, 2), frames(10), request(&[], &[]));
        let hello = ServerHello::new(
            ProtocolVersion::new(1, 2),
            frames(10),
            frames(20),
            vec![WireCapability::new(
                id("tracing"),
                ProtocolVersion::new(1, 0),
            )],
            limits(),
        )
        .expect("synthetic unrequested-capability hello should build");

        // Act
        let error =
            validate_server_hello(&client, &hello).expect_err("unrequested capability must fail");

        // Assert
        assert!(matches!(
            error,
            ClientHelloValidationError::UnrequestedCapability { .. }
        ));
    }

    #[test]
    fn client_rejects_enabled_capability_below_its_requested_minimum() {
        // Arrange
        let client = ClientHello::new(
            versions(1, 2),
            frames(10),
            request(&[("navigation", ProtocolVersion::new(2, 0))], &[]),
        );
        let hello = ServerHello::new(
            ProtocolVersion::new(1, 2),
            frames(10),
            frames(20),
            vec![WireCapability::new(
                id("navigation"),
                ProtocolVersion::new(1, 9),
            )],
            limits(),
        )
        .expect("synthetic incompatible-capability hello should build");

        // Act
        let error = validate_server_hello(&client, &hello)
            .expect_err("incompatible capability version must fail");

        // Assert
        assert!(matches!(
            error,
            ClientHelloValidationError::IncompatibleEnabledCapability { .. }
        ));
    }

    #[test]
    fn negotiation_rejects_missing_required_capability() {
        // Arrange
        let client = ClientHello::new(
            versions(0, 0),
            frames(10),
            request(&[("navigation", ProtocolVersion::new(1, 0))], &[]),
        );
        let server = ServerHandshakeConfig::new(versions(0, 0), frames(10), manifest(&[]));

        // Act
        let error = negotiate(&client, &server).expect_err("missing requirement should fail");

        // Assert
        assert_eq!(
            error,
            NegotiationError::MissingRequiredCapability {
                id: id("navigation")
            }
        );
    }

    #[test]
    fn negotiation_rejects_incompatible_required_capability() {
        // Arrange
        let client = ClientHello::new(
            versions(0, 0),
            frames(10),
            request(&[("navigation", ProtocolVersion::new(2, 0))], &[]),
        );
        let server = ServerHandshakeConfig::new(
            versions(0, 0),
            frames(10),
            manifest(&[("navigation", ProtocolVersion::new(1, 9))]),
        );

        // Act
        let error = negotiate(&client, &server).expect_err("incompatible requirement should fail");

        // Assert
        assert_eq!(
            error,
            NegotiationError::IncompatibleRequiredCapability {
                id: id("navigation"),
                client: VersionPolicy::compatible_from(ProtocolVersion::new(2, 0)),
                server: VersionPolicy::compatible_through(ProtocolVersion::new(1, 9)),
            }
        );
    }

    #[test]
    fn negotiation_enables_compatible_requests_in_deterministic_id_order() {
        // Arrange
        let client = ClientHello::new(
            versions(0, 0),
            frames(10),
            request(
                &[("navigation", ProtocolVersion::new(1, 0))],
                &[
                    ("tracing", ProtocolVersion::new(1, 0)),
                    ("dom", ProtocolVersion::new(2, 0)),
                    ("missing", ProtocolVersion::new(1, 0)),
                ],
            ),
        );
        let server = ServerHandshakeConfig::new(
            versions(0, 0),
            frames(10),
            manifest(&[
                ("tracing", ProtocolVersion::new(1, 1)),
                ("navigation", ProtocolVersion::new(1, 2)),
                ("dom", ProtocolVersion::new(1, 9)),
                ("unrequested", ProtocolVersion::new(1, 0)),
            ]),
        );

        // Act
        let hello = negotiate(&client, &server).expect("required capability should negotiate");

        // Assert
        assert_eq!(
            hello
                .enabled_capabilities()
                .iter()
                .map(|capability| capability.id().as_str())
                .collect::<Vec<_>>(),
            vec!["navigation", "tracing"]
        );
    }

    #[test]
    fn negotiation_rejects_disjoint_version_ranges() {
        // Arrange
        let client = ClientHello::new(versions(0, 1), frames(10), request(&[], &[]));
        let server = ServerHandshakeConfig::new(versions(2, 3), frames(10), manifest(&[]));

        // Act
        let error = negotiate(&client, &server).expect_err("disjoint ranges should fail");

        // Assert
        assert!(matches!(error, NegotiationError::NoCommonVersion { .. }));
    }

    #[test]
    fn negotiation_rejects_different_major_versions() {
        // Arrange
        let client = ClientHello::new(versions(0, 1), frames(10), request(&[], &[]));
        let server_versions =
            VersionPolicy::new(ProtocolVersion::new(2, 0), ProtocolVersion::new(2, 1))
                .expect("server version range should build");
        let server = ServerHandshakeConfig::new(server_versions, frames(10), manifest(&[]));

        // Act
        let error = negotiate(&client, &server).expect_err("major mismatch should fail");

        // Assert
        assert!(matches!(error, NegotiationError::NoCommonVersion { .. }));
    }
}
