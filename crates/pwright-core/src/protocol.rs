use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;
use std::time::Duration;

use crate::capability::{Capability, CommandId, EventId};

macro_rules! numeric_id {
    ($name:ident, $value:ty) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name($value);

        impl $name {
            pub const fn new(value: $value) -> Self {
                Self(value)
            }

            pub const fn get(self) -> $value {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                Display::fmt(&self.0, formatter)
            }
        }
    };
}

numeric_id!(CallId, u64);
numeric_id!(ObjectId, u64);
numeric_id!(CancellationId, u64);
numeric_id!(MethodTag, u32);
numeric_id!(EventTag, u32);
numeric_id!(EventSequence, u64);
numeric_id!(ObjectKindTag, u32);

impl EventSequence {
    pub const FIRST: Self = Self::new(1);

    pub const fn checked_next(self) -> Option<Self> {
        match self.get().checked_add(1) {
            Some(value) => Some(Self::new(value)),
            None => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectKindId(&'static str);

impl ObjectKindId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl Display for ObjectKindId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// Stable semantic identity and compact wire tag for one remote object kind.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectKindDescriptor {
    pub id: ObjectKindId,
    pub tag: ObjectKindTag,
    pub name: &'static str,
    pub allows_root: bool,
    pub parent_kinds: &'static [ObjectKindId],
}

impl ObjectKindDescriptor {
    pub const fn new(id: ObjectKindId, tag: ObjectKindTag, name: &'static str) -> Self {
        Self {
            id,
            tag,
            name,
            allows_root: true,
            parent_kinds: &[],
        }
    }

    /// Declares the canonical parent kinds and makes a parent mandatory in a
    /// complete object graph.
    pub const fn with_parent_kinds(mut self, parent_kinds: &'static [ObjectKindId]) -> Self {
        self.allows_root = false;
        self.parent_kinds = parent_kinds;
        self
    }

    /// Also permits this kind to be a root in a complete object graph.
    pub const fn allowing_root(mut self) -> Self {
        self.allows_root = true;
        self
    }

    pub fn allows_parent(self, parent: ObjectKindId) -> bool {
        self.parent_kinds.contains(&parent)
    }
}

/// Associates a zero-sized Rust marker with one protocol object kind.
pub trait ObjectKindSpec: Send + Sync + 'static {
    const DESCRIPTOR: ObjectKindDescriptor;
}

/// Allocation-free catalog used to resolve semantic object IDs from wire tags.
#[derive(Clone, Copy, Debug)]
pub struct ObjectKindCatalog {
    kinds: &'static [ObjectKindDescriptor],
}

impl ObjectKindCatalog {
    pub const fn new(kinds: &'static [ObjectKindDescriptor]) -> Self {
        Self { kinds }
    }

    pub const fn kinds(self) -> &'static [ObjectKindDescriptor] {
        self.kinds
    }

    pub fn by_tag(self, tag: ObjectKindTag) -> Option<ObjectKindDescriptor> {
        self.kinds
            .iter()
            .find(|descriptor| descriptor.tag == tag)
            .copied()
    }

    pub fn by_id(self, id: ObjectKindId) -> Option<ObjectKindDescriptor> {
        self.kinds
            .iter()
            .find(|descriptor| descriptor.id == id)
            .copied()
    }

    pub fn validate(self) -> Result<(), ObjectKindCatalogError> {
        for (index, descriptor) in self.kinds.iter().enumerate() {
            if !valid_object_kind_id(descriptor.id.as_str()) {
                return Err(ObjectKindCatalogError::InvalidId { id: descriptor.id });
            }
            if descriptor.tag.get() == 0 {
                return Err(ObjectKindCatalogError::ZeroTag { id: descriptor.id });
            }
            if descriptor.name.is_empty() {
                return Err(ObjectKindCatalogError::EmptyName { id: descriptor.id });
            }
            if !descriptor.allows_root && descriptor.parent_kinds.is_empty() {
                return Err(ObjectKindCatalogError::MissingParentKinds { id: descriptor.id });
            }
            for (parent_index, parent) in descriptor.parent_kinds.iter().enumerate() {
                if !valid_object_kind_id(parent.as_str()) {
                    return Err(ObjectKindCatalogError::InvalidParentId {
                        id: descriptor.id,
                        parent: *parent,
                    });
                }
                if let Some(previous) = parent_index
                    .checked_sub(1)
                    .map(|previous| descriptor.parent_kinds[previous])
                {
                    if previous == *parent {
                        return Err(ObjectKindCatalogError::DuplicateParentKind {
                            id: descriptor.id,
                            parent: *parent,
                        });
                    }
                    if previous > *parent {
                        return Err(ObjectKindCatalogError::NonCanonicalParentKindOrder {
                            id: descriptor.id,
                        });
                    }
                }
                if self.by_id(*parent).is_none() {
                    return Err(ObjectKindCatalogError::UnknownParentKind {
                        id: descriptor.id,
                        parent: *parent,
                    });
                }
            }
            for previous in &self.kinds[..index] {
                if previous.id == descriptor.id {
                    return Err(ObjectKindCatalogError::DuplicateId { id: descriptor.id });
                }
                if previous.tag == descriptor.tag {
                    return Err(ObjectKindCatalogError::DuplicateTag {
                        tag: descriptor.tag,
                    });
                }
            }
        }
        Ok(())
    }
}

pub const EMPTY_OBJECT_KIND_CATALOG: ObjectKindCatalog = ObjectKindCatalog::new(&[]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectKindCatalogError {
    InvalidId {
        id: ObjectKindId,
    },
    ZeroTag {
        id: ObjectKindId,
    },
    EmptyName {
        id: ObjectKindId,
    },
    MissingParentKinds {
        id: ObjectKindId,
    },
    InvalidParentId {
        id: ObjectKindId,
        parent: ObjectKindId,
    },
    UnknownParentKind {
        id: ObjectKindId,
        parent: ObjectKindId,
    },
    DuplicateParentKind {
        id: ObjectKindId,
        parent: ObjectKindId,
    },
    NonCanonicalParentKindOrder {
        id: ObjectKindId,
    },
    DuplicateId {
        id: ObjectKindId,
    },
    DuplicateTag {
        tag: ObjectKindTag,
    },
}

impl Display for ObjectKindCatalogError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId { id } => write!(formatter, "object kind ID `{id}` is not canonical"),
            Self::ZeroTag { id } => write!(formatter, "object kind `{id}` uses reserved tag zero"),
            Self::EmptyName { id } => write!(formatter, "object kind `{id}` has an empty name"),
            Self::MissingParentKinds { id } => write!(
                formatter,
                "object kind `{id}` requires a parent but declares no parent kinds"
            ),
            Self::InvalidParentId { id, parent } => write!(
                formatter,
                "object kind `{id}` has noncanonical parent kind ID `{parent}`"
            ),
            Self::UnknownParentKind { id, parent } => write!(
                formatter,
                "object kind `{id}` references unknown parent kind `{parent}`"
            ),
            Self::DuplicateParentKind { id, parent } => write!(
                formatter,
                "object kind `{id}` repeats parent kind `{parent}`"
            ),
            Self::NonCanonicalParentKindOrder { id } => write!(
                formatter,
                "object kind `{id}` parent kinds are not canonically ordered"
            ),
            Self::DuplicateId { id } => write!(formatter, "object kind ID `{id}` is duplicated"),
            Self::DuplicateTag { tag } => {
                write!(formatter, "object kind tag `{tag}` is duplicated")
            }
        }
    }
}

impl Error for ObjectKindCatalogError {}

fn valid_object_kind_id(value: &str) -> bool {
    !value.is_empty()
        && value.split('.').all(|segment| {
            let mut bytes = segment.bytes();
            bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
                && bytes
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        })
}

/// Metadata that applies to one logical operation regardless of transport.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CallMetadata {
    pub timeout: Option<Duration>,
    pub cancellation: Option<CancellationId>,
}

impl CallMetadata {
    pub const fn new(timeout: Option<Duration>, cancellation: Option<CancellationId>) -> Self {
        Self {
            timeout,
            cancellation,
        }
    }
}

/// Associates one method tag with its statically typed protocol values.
pub trait CommandSpec: Send + Sync + 'static {
    const METHOD: MethodTag;
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = &[];
    type Params;
    type Result;
    type ErrorDetails;
}

/// Allocation-free routing metadata for one remotely advertised command.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CapabilityCommandDescriptor {
    command: CommandId,
    method: MethodTag,
    targets: &'static [ObjectKindId],
}

impl CapabilityCommandDescriptor {
    pub const fn new(
        command: CommandId,
        method: MethodTag,
        targets: &'static [ObjectKindId],
    ) -> Self {
        Self {
            command,
            method,
            targets,
        }
    }

    pub const fn command(self) -> CommandId {
        self.command
    }

    pub const fn method(self) -> MethodTag {
        self.method
    }

    pub const fn targets(self) -> &'static [ObjectKindId] {
        self.targets
    }
}

/// Binds a typed command to the capability descriptor that owns it.
///
/// In-process-only test commands may implement `CommandSpec` alone. Commands
/// advertised across a negotiated server boundary implement this stronger
/// contract so the wire manifest can be derived from installed handlers.
pub trait CapabilityCommandSpec: CommandSpec {
    type Capability: Capability;
    const COMMAND: CommandId;
}

/// A typed command targeting one remote object.
pub struct Command<S>
where
    S: CommandSpec,
{
    call_id: CallId,
    object_id: ObjectId,
    metadata: CallMetadata,
    params: S::Params,
    marker: PhantomData<fn() -> S>,
}

impl<S> Command<S>
where
    S: CommandSpec,
{
    pub const fn new(
        call_id: CallId,
        object_id: ObjectId,
        metadata: CallMetadata,
        params: S::Params,
    ) -> Self {
        Self {
            call_id,
            object_id,
            metadata,
            params,
            marker: PhantomData,
        }
    }

    pub const fn call_id(&self) -> CallId {
        self.call_id
    }

    pub const fn object_id(&self) -> ObjectId {
        self.object_id
    }

    pub const fn method(&self) -> MethodTag {
        S::METHOD
    }

    pub const fn metadata(&self) -> CallMetadata {
        self.metadata
    }

    pub const fn params(&self) -> &S::Params {
        &self.params
    }

    pub fn into_params(self) -> S::Params {
        self.params
    }

    pub fn into_parts(self) -> (CallId, ObjectId, CallMetadata, S::Params) {
        (self.call_id, self.object_id, self.metadata, self.params)
    }
}

/// A response correlated to one typed command.
pub struct CommandResult<S>
where
    S: CommandSpec,
{
    call_id: CallId,
    outcome: Result<S::Result, OperationError<S::ErrorDetails>>,
}

impl<S> CommandResult<S>
where
    S: CommandSpec,
{
    pub const fn new(
        call_id: CallId,
        outcome: Result<S::Result, OperationError<S::ErrorDetails>>,
    ) -> Self {
        Self { call_id, outcome }
    }

    pub const fn call_id(&self) -> CallId {
        self.call_id
    }

    pub const fn outcome(&self) -> &Result<S::Result, OperationError<S::ErrorDetails>> {
        &self.outcome
    }

    pub fn into_outcome(self) -> Result<S::Result, OperationError<S::ErrorDetails>> {
        self.outcome
    }
}

/// Associates one event tag with its statically typed payload.
pub trait EventSpec: Send + Sync + 'static {
    const EVENT: EventTag;
    type Payload;
}

/// Stable metadata for one event surface owned by a capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityEventDescriptor {
    event: EventId,
    tag: EventTag,
    target_object_kinds: &'static [ObjectKindId],
}

impl CapabilityEventDescriptor {
    pub const fn new(
        event: EventId,
        tag: EventTag,
        target_object_kinds: &'static [ObjectKindId],
    ) -> Self {
        Self {
            event,
            tag,
            target_object_kinds,
        }
    }

    pub const fn event(self) -> EventId {
        self.event
    }

    pub const fn tag(self) -> EventTag {
        self.tag
    }

    pub const fn target_object_kinds(self) -> &'static [ObjectKindId] {
        self.target_object_kinds
    }
}

/// Associates an event payload with its owning optional capability.
pub trait CapabilityEventSpec: EventSpec {
    type Capability: Capability;
    const EVENT_ID: EventId;
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId];
}

/// A typed event emitted by one remote object.
pub struct Event<S>
where
    S: EventSpec,
{
    object_id: ObjectId,
    payload: S::Payload,
    marker: PhantomData<fn() -> S>,
}

/// A typed event plus its monotonic position in one installed event channel.
///
/// Producers create [`Event`] values without transport metadata. The selected
/// server event channel assigns this sequence before queue admission, and the
/// matching client channel validates it before application delivery.
pub struct SequencedEvent<S>
where
    S: EventSpec,
{
    sequence: EventSequence,
    event: Event<S>,
}

impl<S> SequencedEvent<S>
where
    S: EventSpec,
{
    pub const fn new(sequence: EventSequence, event: Event<S>) -> Self {
        Self { sequence, event }
    }

    pub const fn sequence(&self) -> EventSequence {
        self.sequence
    }

    pub const fn event(&self) -> &Event<S> {
        &self.event
    }

    pub const fn object_id(&self) -> ObjectId {
        self.event.object_id()
    }

    pub const fn event_tag(&self) -> EventTag {
        self.event.event()
    }

    pub const fn payload(&self) -> &S::Payload {
        self.event.payload()
    }

    pub fn into_event(self) -> Event<S> {
        self.event
    }

    pub fn into_payload(self) -> S::Payload {
        self.event.into_payload()
    }
}

impl<S> Event<S>
where
    S: EventSpec,
{
    pub const fn new(object_id: ObjectId, payload: S::Payload) -> Self {
        Self {
            object_id,
            payload,
            marker: PhantomData,
        }
    }

    pub const fn object_id(&self) -> ObjectId {
        self.object_id
    }

    pub const fn event(&self) -> EventTag {
        S::EVENT
    }

    pub const fn payload(&self) -> &S::Payload {
        &self.payload
    }

    pub fn into_payload(self) -> S::Payload {
        self.payload
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancellationReason {
    Caller,
    DeadlineExceeded,
    ParentClosed,
    TransportClosed,
}

/// Stable identity passed to operation-specific cancellation handlers.
///
/// The context intentionally contains no backend value and does not retain
/// command parameters. A handler that needs additional state must keep it in a
/// bounded table keyed by `call_id`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationContext {
    call_id: CallId,
    object_id: ObjectId,
    method: MethodTag,
    cancellation: Option<CancellationId>,
}

impl OperationContext {
    pub const fn new(
        call_id: CallId,
        object_id: ObjectId,
        method: MethodTag,
        cancellation: Option<CancellationId>,
    ) -> Self {
        Self {
            call_id,
            object_id,
            method,
            cancellation,
        }
    }

    pub const fn call_id(self) -> CallId {
        self.call_id
    }

    pub const fn object_id(self) -> ObjectId {
        self.object_id
    }

    pub const fn method(self) -> MethodTag {
        self.method
    }

    pub const fn cancellation(self) -> Option<CancellationId> {
        self.cancellation
    }
}

/// Lifecycle and subscription operations that are independent of a codec.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlMessage {
    Create {
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: ObjectKindTag,
    },
    Adopt {
        object_id: ObjectId,
        new_parent: ObjectId,
    },
    Dispose {
        object_id: ObjectId,
    },
    Cancel {
        call_id: CallId,
        cancellation: CancellationId,
        reason: CancellationReason,
    },
    SetSubscription {
        object_id: ObjectId,
        event: EventTag,
        enabled: bool,
    },
    /// Requests an ordered, graceful connection shutdown.
    Close,
    /// Confirms that a graceful connection shutdown reached its peer barrier.
    CloseAck,
}

/// Stable high-level failure categories exposed above browser backends.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureKind {
    Timeout,
    Cancelled,
    TargetClosed,
    BrowserCrashed,
    UnsupportedCapability,
    ResourceExhausted,
    TransportClosed,
    ProtocolViolation,
    Backend,
    Cleanup,
}

/// A classified operation failure with optional command-specific details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationError<D> {
    kind: FailureKind,
    message: String,
    details: Option<D>,
}

impl<D> OperationError<D> {
    pub fn new(kind: FailureKind, message: impl Into<String>, details: Option<D>) -> Self {
        Self {
            kind,
            message: message.into(),
            details,
        }
    }

    pub const fn kind(&self) -> FailureKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub const fn details(&self) -> Option<&D> {
        self.details.as_ref()
    }

    pub fn map_details<T>(self, map: impl FnOnce(D) -> T) -> OperationError<T> {
        OperationError {
            kind: self.kind,
            message: self.message,
            details: self.details.map(map),
        }
    }

    pub fn reclassify(mut self, kind: FailureKind) -> Self {
        self.kind = kind;
        self
    }
}

impl<D> Display for OperationError<D> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl<D> Error for OperationError<D> where D: fmt::Debug + Send + Sync + 'static {}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec, Event,
        EventSequence, EventSpec, EventTag, FailureKind, MethodTag, ObjectId, ObjectKindCatalog,
        ObjectKindCatalogError, ObjectKindDescriptor, ObjectKindId, ObjectKindTag,
        OperationContext, OperationError, SequencedEvent,
    };

    struct Navigate;

    impl CommandSpec for Navigate {
        const METHOD: MethodTag = MethodTag::new(7);
        type Params = &'static str;
        type Result = u16;
        type ErrorDetails = &'static str;
    }

    struct PageClosed;

    impl EventSpec for PageClosed {
        const EVENT: EventTag = EventTag::new(3);
        type Payload = &'static str;
    }

    #[test]
    fn command_preserves_typed_parameters_and_call_metadata() {
        // Arrange
        let metadata =
            CallMetadata::new(Some(Duration::from_secs(5)), Some(CancellationId::new(13)));

        // Act
        let command = Command::<Navigate>::new(
            CallId::new(11),
            ObjectId::new(17),
            metadata,
            "https://example.com",
        );

        // Assert
        assert_eq!(command.call_id(), CallId::new(11));
        assert_eq!(command.object_id(), ObjectId::new(17));
        assert_eq!(command.method(), MethodTag::new(7));
        assert_eq!(command.metadata(), metadata);
        assert_eq!(*command.params(), "https://example.com");
    }

    #[test]
    fn command_can_be_decomposed_without_erasing_its_parameter_type() {
        // Arrange
        let command = Command::<Navigate>::new(
            CallId::new(29),
            ObjectId::new(31),
            CallMetadata::default(),
            "https://example.org",
        );

        // Act
        let parts = command.into_parts();

        // Assert
        assert_eq!(parts.0, CallId::new(29));
        assert_eq!(parts.1, ObjectId::new(31));
        assert_eq!(parts.2, CallMetadata::default());
        assert_eq!(parts.3, "https://example.org");
    }

    #[test]
    fn command_result_keeps_failure_classification_and_typed_details() {
        // Arrange
        let failure = OperationError::new(
            FailureKind::TargetClosed,
            "page closed during navigation",
            Some("frame detached"),
        );

        // Act
        let result = CommandResult::<Navigate>::new(CallId::new(19), Err(failure));

        // Assert
        assert_eq!(result.call_id(), CallId::new(19));
        let error = result.outcome().as_ref().expect_err("result should fail");
        assert_eq!(error.kind(), FailureKind::TargetClosed);
        assert_eq!(error.details(), Some(&"frame detached"));
    }

    #[test]
    fn operation_context_keeps_protocol_identities_in_separate_types() {
        // Arrange
        let context = OperationContext::new(
            CallId::new(11),
            ObjectId::new(17),
            MethodTag::new(23),
            Some(CancellationId::new(29)),
        );

        // Act
        let identities = (
            context.call_id(),
            context.object_id(),
            context.method(),
            context.cancellation(),
        );

        // Assert
        assert_eq!(identities.0, CallId::new(11));
        assert_eq!(identities.1, ObjectId::new(17));
        assert_eq!(identities.2, MethodTag::new(23));
        assert_eq!(identities.3, Some(CancellationId::new(29)));
    }

    #[test]
    fn event_preserves_its_typed_payload_and_source_object() {
        // Arrange
        let page_id = ObjectId::new(23);

        // Act
        let event = Event::<PageClosed>::new(page_id, "browser process exited");

        // Assert
        assert_eq!(event.object_id(), page_id);
        assert_eq!(event.event(), EventTag::new(3));
        assert_eq!(*event.payload(), "browser process exited");
    }

    #[test]
    fn sequenced_event_keeps_delivery_metadata_out_of_the_backend_event() {
        // Arrange
        let event = Event::<PageClosed>::new(ObjectId::new(23), "browser process exited");

        // Act
        let sequenced = SequencedEvent::new(EventSequence::FIRST, event);

        // Assert
        assert_eq!(sequenced.sequence(), EventSequence::new(1));
        assert_eq!(sequenced.object_id(), ObjectId::new(23));
        assert_eq!(sequenced.event_tag(), EventTag::new(3));
        assert_eq!(*sequenced.payload(), "browser process exited");
        assert_eq!(
            sequenced.sequence().checked_next(),
            Some(EventSequence::new(2))
        );
        assert_eq!(EventSequence::new(u64::MAX).checked_next(), None);
    }

    #[test]
    fn object_kind_catalog_rejects_duplicate_wire_tags() {
        // Arrange
        const KINDS: &[ObjectKindDescriptor] = &[
            ObjectKindDescriptor::new(
                ObjectKindId::new("test.page"),
                ObjectKindTag::new(3),
                "Page",
            ),
            ObjectKindDescriptor::new(
                ObjectKindId::new("test.frame"),
                ObjectKindTag::new(3),
                "Frame",
            ),
        ];
        let catalog = ObjectKindCatalog::new(KINDS);

        // Act
        let error = catalog.validate().expect_err("duplicate tag must fail");

        // Assert
        assert_eq!(
            error,
            ObjectKindCatalogError::DuplicateTag {
                tag: ObjectKindTag::new(3),
            }
        );
    }

    #[test]
    fn object_kind_catalog_rejects_an_unknown_parent_kind() {
        // Arrange
        const KINDS: &[ObjectKindDescriptor] = &[ObjectKindDescriptor::new(
            ObjectKindId::new("test.page"),
            ObjectKindTag::new(3),
            "Page",
        )
        .with_parent_kinds(&[ObjectKindId::new("test.context")])];
        let catalog = ObjectKindCatalog::new(KINDS);

        // Act
        let error = catalog
            .validate()
            .expect_err("unknown parent kind must fail");

        // Assert
        assert_eq!(
            error,
            ObjectKindCatalogError::UnknownParentKind {
                id: ObjectKindId::new("test.page"),
                parent: ObjectKindId::new("test.context"),
            }
        );
    }

    #[test]
    fn object_kind_catalog_requires_canonical_parent_order() {
        // Arrange
        const FRAME: ObjectKindId = ObjectKindId::new("test.frame");
        const PAGE: ObjectKindId = ObjectKindId::new("test.page");
        const KINDS: &[ObjectKindDescriptor] = &[
            ObjectKindDescriptor::new(FRAME, ObjectKindTag::new(4), "Frame")
                .with_parent_kinds(&[PAGE, FRAME]),
            ObjectKindDescriptor::new(PAGE, ObjectKindTag::new(3), "Page"),
        ];
        let catalog = ObjectKindCatalog::new(KINDS);

        // Act
        let error = catalog
            .validate()
            .expect_err("noncanonical parent order must fail");

        // Assert
        assert_eq!(
            error,
            ObjectKindCatalogError::NonCanonicalParentKindOrder { id: FRAME }
        );
    }
}
