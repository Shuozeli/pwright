//! Dependency-free composition and capability contracts for Pwright.
//!
//! This crate deliberately contains no async runtime, wire codec, browser
//! protocol, browser launcher, or delivery surface. Those choices belong in
//! components layered above this kernel.

mod bounded_queue;
mod capability;
mod object_graph;
mod object_id;
mod pending_calls;
mod protocol;
mod resource;
mod service;

pub use bounded_queue::{BoundedQueue, OverflowPolicy, PushOutcome, QueueMetrics};
pub use capability::{
    Capability, CapabilityDependency, CapabilityDescriptor, CapabilityId, CommandId, EventId,
    PluginInfo, ProtocolVersion, Registry, RegistryBuilder, RegistryError,
};
pub use object_graph::{ObjectGraph, ObjectGraphError, ObjectRecord};
pub use object_id::{AtomicObjectIdSource, ObjectIdSource, ObjectIdSourceError};
pub use pending_calls::{
    CancellationRequestOutcome, PendingCall, PendingCallError, PendingCallState, PendingCallTable,
};
pub use protocol::{
    CallId, CallMetadata, CancellationId, CancellationReason, CapabilityCommandDescriptor,
    CapabilityCommandSpec, CapabilityEventDescriptor, CapabilityEventSpec, Command, CommandResult,
    CommandSpec, ControlMessage, EMPTY_OBJECT_KIND_CATALOG, Event, EventSequence, EventSpec,
    EventTag, FailureKind, MethodTag, ObjectId, ObjectKindCatalog, ObjectKindCatalogError,
    ObjectKindDescriptor, ObjectKindId, ObjectKindSpec, ObjectKindTag, OperationContext,
    OperationError, SequencedEvent,
};
pub use resource::{
    BudgetedBytes, BudgetedUtf8, ProductionResourceError, ProductionResourceLedger,
    ResourceCapacityBoundary, ResourceClass, ResourceClassMetrics, ResourceError, ResourceLease,
    ResourceLedger, ResourceLimit, ResourceLimits, ResourceMetrics, ResourceRequest,
    ResourceRequirements, ResourceScope, ResourceScopeMetrics,
};
pub use service::{CommandCancellation, Layer, Service, Stack};
