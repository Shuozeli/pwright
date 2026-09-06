use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CapabilityId(&'static str);

impl CapabilityId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl Display for CapabilityId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CommandId(&'static str);

impl CommandId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl Display for CommandId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventId(&'static str);

impl EventId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl Display for EventId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    pub const fn satisfies(self, minimum: Self) -> bool {
        self.major == minimum.major && self.minor >= minimum.minor
    }
}

impl Display for ProtocolVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PluginInfo {
    pub id: CapabilityId,
    pub name: &'static str,
    pub version: ProtocolVersion,
}

impl PluginInfo {
    pub const fn new(id: CapabilityId, name: &'static str, version: ProtocolVersion) -> Self {
        Self { id, name, version }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityDependency {
    pub id: CapabilityId,
    pub minimum_version: ProtocolVersion,
}

impl CapabilityDependency {
    pub const fn new(id: CapabilityId, minimum_version: ProtocolVersion) -> Self {
        Self {
            id,
            minimum_version,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityDescriptor {
    pub plugin: PluginInfo,
    pub dependencies: &'static [CapabilityDependency],
    pub commands: &'static [CommandId],
    pub events: &'static [EventId],
}

impl CapabilityDescriptor {
    pub const fn new(
        plugin: PluginInfo,
        dependencies: &'static [CapabilityDependency],
        commands: &'static [CommandId],
    ) -> Self {
        Self {
            plugin,
            dependencies,
            commands,
            events: &[],
        }
    }

    pub const fn with_events(mut self, events: &'static [EventId]) -> Self {
        self.events = events;
        self
    }
}

pub trait Capability: Send + Sync + 'static {
    const DESCRIPTOR: CapabilityDescriptor;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    DuplicateCapability {
        id: CapabilityId,
    },
    DuplicateCommand {
        command: CommandId,
        existing_owner: CapabilityId,
        requested_owner: CapabilityId,
    },
    DuplicateEvent {
        event: EventId,
        existing_owner: CapabilityId,
        requested_owner: CapabilityId,
    },
    MissingDependency {
        capability: CapabilityId,
        dependency: CapabilityId,
    },
    IncompatibleDependency {
        capability: CapabilityId,
        dependency: CapabilityId,
        minimum: ProtocolVersion,
        installed: ProtocolVersion,
    },
    DependencyCycle {
        capabilities: Vec<CapabilityId>,
    },
}

impl Display for RegistryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCapability { id } => {
                write!(formatter, "capability `{id}` is already registered")
            }
            Self::DuplicateCommand {
                command,
                existing_owner,
                requested_owner,
            } => write!(
                formatter,
                "command `{command}` is owned by both `{existing_owner}` and `{requested_owner}`"
            ),
            Self::DuplicateEvent {
                event,
                existing_owner,
                requested_owner,
            } => write!(
                formatter,
                "event `{event}` is owned by both `{existing_owner}` and `{requested_owner}`"
            ),
            Self::MissingDependency {
                capability,
                dependency,
            } => write!(
                formatter,
                "capability `{capability}` requires missing capability `{dependency}`"
            ),
            Self::IncompatibleDependency {
                capability,
                dependency,
                minimum,
                installed,
            } => write!(
                formatter,
                "capability `{capability}` requires `{dependency}` >= {minimum} with the same major version, but {installed} is installed"
            ),
            Self::DependencyCycle { capabilities } => {
                formatter.write_str("capability dependency cycle: ")?;
                for (index, capability) in capabilities.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(" -> ")?;
                    }
                    Display::fmt(capability, formatter)?;
                }
                Ok(())
            }
        }
    }
}

impl Error for RegistryError {}

#[derive(Default)]
pub struct RegistryBuilder {
    capabilities: BTreeMap<CapabilityId, CapabilityDescriptor>,
    command_owners: BTreeMap<CommandId, CapabilityId>,
    event_owners: BTreeMap<EventId, CapabilityId>,
}

impl RegistryBuilder {
    pub const fn new() -> Self {
        Self {
            capabilities: BTreeMap::new(),
            command_owners: BTreeMap::new(),
            event_owners: BTreeMap::new(),
        }
    }

    pub fn register<C>(&mut self) -> Result<(), RegistryError>
    where
        C: Capability,
    {
        self.register_descriptor(C::DESCRIPTOR)
    }

    pub fn register_descriptor(
        &mut self,
        descriptor: CapabilityDescriptor,
    ) -> Result<(), RegistryError> {
        let requested_owner = descriptor.plugin.id;
        if self.capabilities.contains_key(&requested_owner) {
            return Err(RegistryError::DuplicateCapability {
                id: requested_owner,
            });
        }

        let mut commands_in_descriptor = BTreeSet::new();
        for command in descriptor.commands {
            if !commands_in_descriptor.insert(*command) {
                return Err(RegistryError::DuplicateCommand {
                    command: *command,
                    existing_owner: requested_owner,
                    requested_owner,
                });
            }
            if let Some(existing_owner) = self.command_owners.get(command) {
                return Err(RegistryError::DuplicateCommand {
                    command: *command,
                    existing_owner: *existing_owner,
                    requested_owner,
                });
            }
        }

        let mut events_in_descriptor = BTreeSet::new();
        for event in descriptor.events {
            if !events_in_descriptor.insert(*event) {
                return Err(RegistryError::DuplicateEvent {
                    event: *event,
                    existing_owner: requested_owner,
                    requested_owner,
                });
            }
            if let Some(existing_owner) = self.event_owners.get(event) {
                return Err(RegistryError::DuplicateEvent {
                    event: *event,
                    existing_owner: *existing_owner,
                    requested_owner,
                });
            }
        }

        self.capabilities.insert(requested_owner, descriptor);
        for command in descriptor.commands {
            self.command_owners.insert(*command, requested_owner);
        }
        for event in descriptor.events {
            self.event_owners.insert(*event, requested_owner);
        }
        Ok(())
    }

    pub fn build(self) -> Result<Registry, RegistryError> {
        self.validate_dependencies()?;
        self.validate_acyclic()?;
        Ok(Registry {
            capabilities: self.capabilities,
            command_owners: self.command_owners,
            event_owners: self.event_owners,
        })
    }

    fn validate_dependencies(&self) -> Result<(), RegistryError> {
        for descriptor in self.capabilities.values() {
            for dependency in descriptor.dependencies {
                let Some(installed) = self.capabilities.get(&dependency.id) else {
                    return Err(RegistryError::MissingDependency {
                        capability: descriptor.plugin.id,
                        dependency: dependency.id,
                    });
                };
                if !installed
                    .plugin
                    .version
                    .satisfies(dependency.minimum_version)
                {
                    return Err(RegistryError::IncompatibleDependency {
                        capability: descriptor.plugin.id,
                        dependency: dependency.id,
                        minimum: dependency.minimum_version,
                        installed: installed.plugin.version,
                    });
                }
            }
        }
        Ok(())
    }

    fn validate_acyclic(&self) -> Result<(), RegistryError> {
        let mut states = BTreeMap::new();
        let mut path = Vec::new();
        for capability in self.capabilities.keys().copied() {
            self.visit(capability, &mut states, &mut path)?;
        }
        Ok(())
    }

    fn visit(
        &self,
        capability: CapabilityId,
        states: &mut BTreeMap<CapabilityId, VisitState>,
        path: &mut Vec<CapabilityId>,
    ) -> Result<(), RegistryError> {
        match states.get(&capability).copied() {
            Some(VisitState::Visited) => return Ok(()),
            Some(VisitState::Visiting) => {
                let cycle_start = path
                    .iter()
                    .position(|candidate| *candidate == capability)
                    .unwrap_or(0);
                let mut capabilities = path[cycle_start..].to_vec();
                capabilities.push(capability);
                return Err(RegistryError::DependencyCycle { capabilities });
            }
            None => {}
        }

        states.insert(capability, VisitState::Visiting);
        path.push(capability);
        let descriptor = &self.capabilities[&capability];
        for dependency in descriptor.dependencies {
            self.visit(dependency.id, states, path)?;
        }
        path.pop();
        states.insert(capability, VisitState::Visited);
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum VisitState {
    Visiting,
    Visited,
}

#[derive(Debug)]
pub struct Registry {
    capabilities: BTreeMap<CapabilityId, CapabilityDescriptor>,
    command_owners: BTreeMap<CommandId, CapabilityId>,
    event_owners: BTreeMap<EventId, CapabilityId>,
}

impl Registry {
    pub fn contains(&self, capability: CapabilityId) -> bool {
        self.capabilities.contains_key(&capability)
    }

    pub fn descriptor(&self, capability: CapabilityId) -> Option<&CapabilityDescriptor> {
        self.capabilities.get(&capability)
    }

    pub fn owner_of(&self, command: CommandId) -> Option<CapabilityId> {
        self.command_owners.get(&command).copied()
    }

    pub fn owner_of_event(&self, event: EventId) -> Option<CapabilityId> {
        self.event_owners.get(&event).copied()
    }

    pub fn capabilities(
        &self,
    ) -> impl ExactSizeIterator<Item = &CapabilityDescriptor> + DoubleEndedIterator {
        self.capabilities.values()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Capability, CapabilityDependency, CapabilityDescriptor, CapabilityId, CommandId, EventId,
        PluginInfo, ProtocolVersion, RegistryBuilder, RegistryError,
    };

    const DOM_ID: CapabilityId = CapabilityId::new("pwright.dom");
    const NAVIGATION_ID: CapabilityId = CapabilityId::new("pwright.navigation");
    const DOM_QUERY: CommandId = CommandId::new("dom.query");
    const NAVIGATE: CommandId = CommandId::new("navigation.goto");
    const PAGE_LOAD: EventId = EventId::new("page.load");

    struct Dom;

    impl Capability for Dom {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(DOM_ID, "DOM", ProtocolVersion::new(1, 2)),
            &[],
            &[DOM_QUERY],
        );
    }

    struct Navigation;

    impl Capability for Navigation {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(NAVIGATION_ID, "Navigation", ProtocolVersion::new(1, 0)),
            &[CapabilityDependency::new(
                DOM_ID,
                ProtocolVersion::new(1, 1),
            )],
            &[NAVIGATE],
        );
    }

    struct ConflictingNavigation;

    impl Capability for ConflictingNavigation {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(
                CapabilityId::new("pwright.other-navigation"),
                "Other navigation",
                ProtocolVersion::new(1, 0),
            ),
            &[],
            &[NAVIGATE],
        );
    }

    struct PageEvents;

    impl Capability for PageEvents {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(
                CapabilityId::new("pwright.page-events"),
                "Page events",
                ProtocolVersion::new(1, 0),
            ),
            &[],
            &[],
        )
        .with_events(&[PAGE_LOAD]);
    }

    struct ConflictingPageEvents;

    impl Capability for ConflictingPageEvents {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(
                CapabilityId::new("pwright.other-page-events"),
                "Other page events",
                ProtocolVersion::new(1, 0),
            ),
            &[],
            &[],
        )
        .with_events(&[PAGE_LOAD]);
    }

    struct FutureNavigation;

    impl Capability for FutureNavigation {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(
                CapabilityId::new("pwright.future-navigation"),
                "Future navigation",
                ProtocolVersion::new(1, 0),
            ),
            &[CapabilityDependency::new(
                DOM_ID,
                ProtocolVersion::new(2, 0),
            )],
            &[],
        );
    }

    struct CycleA;

    impl Capability for CycleA {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(
                CapabilityId::new("cycle.a"),
                "Cycle A",
                ProtocolVersion::new(1, 0),
            ),
            &[CapabilityDependency::new(
                CapabilityId::new("cycle.b"),
                ProtocolVersion::new(1, 0),
            )],
            &[],
        );
    }

    struct CycleB;

    impl Capability for CycleB {
        const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
            PluginInfo::new(
                CapabilityId::new("cycle.b"),
                "Cycle B",
                ProtocolVersion::new(1, 0),
            ),
            &[CapabilityDependency::new(
                CapabilityId::new("cycle.a"),
                ProtocolVersion::new(1, 0),
            )],
            &[],
        );
    }

    #[test]
    fn registry_exposes_registered_capabilities_and_command_owners() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder.register::<Dom>().expect("DOM should register");
        builder
            .register::<Navigation>()
            .expect("navigation should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert!(registry.contains(DOM_ID));
        assert_eq!(registry.owner_of(NAVIGATE), Some(NAVIGATION_ID));
        assert_eq!(registry.capabilities().len(), 2);
    }

    #[test]
    fn register_rejects_a_duplicate_capability() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<Dom>()
            .expect("first registration should work");

        // Act
        let error = builder
            .register::<Dom>()
            .expect_err("duplicate capability should fail");

        // Assert
        assert_eq!(error, RegistryError::DuplicateCapability { id: DOM_ID });
    }

    #[test]
    fn register_rejects_duplicate_command_ownership() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<Navigation>()
            .expect("first command owner should register");

        // Act
        let error = builder
            .register::<ConflictingNavigation>()
            .expect_err("duplicate command owner should fail");

        // Assert
        assert_eq!(
            error,
            RegistryError::DuplicateCommand {
                command: NAVIGATE,
                existing_owner: NAVIGATION_ID,
                requested_owner: CapabilityId::new("pwright.other-navigation"),
            }
        );
    }

    #[test]
    fn registry_exposes_event_ownership() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<PageEvents>()
            .expect("page events should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert_eq!(
            registry.owner_of_event(PAGE_LOAD),
            Some(CapabilityId::new("pwright.page-events"))
        );
    }

    #[test]
    fn register_rejects_duplicate_event_ownership() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<PageEvents>()
            .expect("first event owner should register");

        // Act
        let error = builder
            .register::<ConflictingPageEvents>()
            .expect_err("duplicate event owner should fail");

        // Assert
        assert_eq!(
            error,
            RegistryError::DuplicateEvent {
                event: PAGE_LOAD,
                existing_owner: CapabilityId::new("pwright.page-events"),
                requested_owner: CapabilityId::new("pwright.other-page-events"),
            }
        );
    }

    #[test]
    fn build_rejects_a_missing_dependency() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<Navigation>()
            .expect("capability metadata should register");

        // Act
        let error = builder
            .build()
            .expect_err("missing dependency should fail validation");

        // Assert
        assert_eq!(
            error,
            RegistryError::MissingDependency {
                capability: NAVIGATION_ID,
                dependency: DOM_ID,
            }
        );
    }

    #[test]
    fn build_accepts_a_compatible_dependency_version() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder.register::<Dom>().expect("DOM should register");
        builder
            .register::<Navigation>()
            .expect("navigation should register");

        // Act
        let result = builder.build();

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn build_rejects_an_incompatible_dependency_version() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder.register::<Dom>().expect("DOM should register");
        builder
            .register::<FutureNavigation>()
            .expect("future navigation should register");

        // Act
        let error = builder
            .build()
            .expect_err("incompatible dependency should fail validation");

        // Assert
        assert_eq!(
            error,
            RegistryError::IncompatibleDependency {
                capability: CapabilityId::new("pwright.future-navigation"),
                dependency: DOM_ID,
                minimum: ProtocolVersion::new(2, 0),
                installed: ProtocolVersion::new(1, 2),
            }
        );
    }

    #[test]
    fn build_rejects_a_dependency_cycle() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<CycleA>()
            .expect("cycle A should register");
        builder
            .register::<CycleB>()
            .expect("cycle B should register");

        // Act
        let error = builder
            .build()
            .expect_err("dependency cycle should fail validation");

        // Assert
        assert_eq!(
            error,
            RegistryError::DependencyCycle {
                capabilities: vec![
                    CapabilityId::new("cycle.a"),
                    CapabilityId::new("cycle.b"),
                    CapabilityId::new("cycle.a"),
                ],
            }
        );
    }
}
