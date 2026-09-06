//! Task-free bounded in-memory Artifact and Stream registry services.
//!
//! Producers register explicit Artifact objects. The registry owns fixed-size
//! Artifact/Stream tables and an injected resource ledger; it neither records
//! traces nor selects a runtime, transport, filesystem, archive, or browser.
//! A remote composition must wrap object creation/disposal with its lifecycle
//! control channel before advertising the generated server handlers.

use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::{Ready, ready};
use std::sync::{Arc, Mutex, MutexGuard};

use pwright_artifact_model::{
    Artifact, ArtifactFailureResult, ArtifactStream, ArtifactStreamError, ArtifactStreamLimits,
    PreparedArtifactRead, StreamInitializer,
};
use pwright_browser_model::STREAM_OBJECT_KIND;
use pwright_cap_artifact::{
    ArtifactDelete, ArtifactErrorDetails, ArtifactFailure, ArtifactOpenStream, StreamClose,
    StreamRead,
};
use pwright_core::{
    BudgetedBytes, BudgetedUtf8, CallId, Command, CommandResult, FailureKind, ObjectId,
    ObjectIdSource, ProductionResourceError, ProductionResourceLedger, ResourceClass,
    ResourceError, ResourceLedger, ResourceRequirements, ResourceScope, Service,
};
use pwright_server_artifact::ArtifactServerServices;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactRegistryLimits {
    pub max_artifacts: usize,
    pub max_streams: usize,
    pub max_artifact_bytes: usize,
    pub max_read_bytes: usize,
    pub max_failure_bytes: usize,
    pub max_retained_result_bytes: usize,
}

impl ArtifactRegistryLimits {
    pub const fn new(
        max_artifacts: usize,
        max_streams: usize,
        max_artifact_bytes: usize,
        max_read_bytes: usize,
        max_failure_bytes: usize,
        max_retained_result_bytes: usize,
    ) -> Self {
        Self {
            max_artifacts,
            max_streams,
            max_artifact_bytes,
            max_read_bytes,
            max_failure_bytes,
            max_retained_result_bytes,
        }
    }

    pub fn validate(self) -> Result<(), ArtifactRegistryError> {
        for (name, value) in [
            ("Artifact count", self.max_artifacts),
            ("Stream count", self.max_streams),
            ("read bytes", self.max_read_bytes),
        ] {
            if value == 0 {
                return Err(ArtifactRegistryError::ZeroLimit(name));
            }
        }
        Ok(())
    }

    const fn stream_limits(self) -> ArtifactStreamLimits {
        ArtifactStreamLimits::new(self.max_artifact_bytes, self.max_read_bytes)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactRegistryProductionProfile {
    limits: ArtifactRegistryLimits,
    required_bytes: u64,
    required_per_scope_bytes: u64,
    required_scopes: usize,
    required_leases: usize,
}

impl ArtifactRegistryProductionProfile {
    pub fn new(limits: ArtifactRegistryLimits) -> Result<Self, ArtifactRegistryError> {
        limits.validate()?;
        let source_per_artifact = limits.max_artifact_bytes.max(limits.max_failure_bytes);
        let source_bytes = limits
            .max_artifacts
            .checked_mul(source_per_artifact)
            .ok_or(ArtifactRegistryError::LimitOverflow)?;
        let total_bytes = source_bytes
            .checked_add(limits.max_retained_result_bytes)
            .ok_or(ArtifactRegistryError::LimitOverflow)?;
        let result_scopes = limits.max_retained_result_bytes;
        let required_scopes = limits
            .max_artifacts
            .checked_add(result_scopes)
            .ok_or(ArtifactRegistryError::LimitOverflow)?;
        let required_leases = required_scopes;
        Ok(Self {
            limits,
            required_bytes: u64::try_from(total_bytes)
                .map_err(|_| ArtifactRegistryError::LimitOverflow)?,
            required_per_scope_bytes: u64::try_from(
                source_per_artifact
                    .max(limits.max_read_bytes)
                    .max(limits.max_failure_bytes),
            )
            .map_err(|_| ArtifactRegistryError::LimitOverflow)?,
            required_scopes,
            required_leases,
        })
    }

    pub const fn limits(self) -> ArtifactRegistryLimits {
        self.limits
    }

    pub const fn requirements(self) -> ResourceRequirements {
        ResourceRequirements::none()
            .with_class(
                ResourceClass::ArtifactBytes,
                self.required_bytes,
                self.required_per_scope_bytes,
            )
            .with_metadata(self.required_scopes, self.required_leases)
    }

    pub fn create(
        self,
        resources: &ProductionResourceLedger,
    ) -> Result<ArtifactRegistry, ArtifactRegistryError> {
        let ledger = resources.claim(self.requirements())?;
        ArtifactRegistry::new(self.limits, ledger)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactRegistryMetrics {
    pub artifacts: usize,
    pub artifact_peak: usize,
    pub streams: usize,
    pub stream_peak: usize,
    pub max_artifacts: usize,
    pub max_streams: usize,
}

#[derive(Debug)]
pub struct ArtifactDisposal {
    object_ids_child_first: Vec<ObjectId>,
}

impl ArtifactDisposal {
    pub fn object_ids_child_first(&self) -> &[ObjectId] {
        &self.object_ids_child_first
    }

    pub fn into_object_ids_child_first(self) -> Vec<ObjectId> {
        self.object_ids_child_first
    }
}

enum ArtifactPayload {
    Bytes(Artifact),
    Failure(BudgetedUtf8),
}

struct ArtifactEntry {
    id: ObjectId,
    parent: ObjectId,
    payload: ArtifactPayload,
}

struct StreamEntry {
    id: ObjectId,
    artifact_id: ObjectId,
    stream: ArtifactStream,
}

struct ArtifactRegistryState {
    artifacts: Vec<ArtifactEntry>,
    artifact_peak: usize,
    streams: Vec<StreamEntry>,
    stream_peak: usize,
}

/// A registry insertion whose table mutation is deferred until the matching
/// runtime lifecycle transaction has committed. The registry lock prevents a
/// command from observing a half-published Artifact.
pub struct PreparedArtifactRegistration<'a> {
    state: MutexGuard<'a, ArtifactRegistryState>,
    entry: Option<ArtifactEntry>,
}

impl PreparedArtifactRegistration<'_> {
    pub fn commit(mut self) {
        let entry = self
            .entry
            .take()
            .expect("an active Artifact registration owns its entry");
        self.state.artifacts.push(entry);
        self.state.artifact_peak = self.state.artifact_peak.max(self.state.artifacts.len());
    }
}

/// A Stream insertion held outside the registry table until its runtime
/// `Create` has committed.
pub struct PreparedStreamRegistration<'a> {
    state: MutexGuard<'a, ArtifactRegistryState>,
    entry: Option<StreamEntry>,
    initializer: StreamInitializer,
}

impl PreparedStreamRegistration<'_> {
    pub const fn initializer(&self) -> StreamInitializer {
        self.initializer
    }

    pub fn commit(mut self) -> StreamInitializer {
        let entry = self
            .entry
            .take()
            .expect("an active Stream registration owns its entry");
        self.state.streams.push(entry);
        self.state.stream_peak = self.state.stream_peak.max(self.state.streams.len());
        self.initializer
    }
}

/// A stable child-first Artifact deletion plan. Holding this value prevents
/// registry operations from changing the plan between runtime prepare and
/// commit.
pub struct PreparedArtifactDisposal<'a> {
    state: MutexGuard<'a, ArtifactRegistryState>,
    artifact_id: ObjectId,
    object_ids_child_first: Vec<ObjectId>,
}

impl PreparedArtifactDisposal<'_> {
    pub fn object_ids_child_first(&self) -> &[ObjectId] {
        &self.object_ids_child_first
    }

    pub fn commit(mut self) -> Result<ArtifactDisposal, ArtifactRegistryError> {
        let artifact_index = self
            .state
            .artifacts
            .iter()
            .position(|entry| entry.id == self.artifact_id)
            .expect("a locked Artifact deletion plan keeps its target live");
        let mut index = 0;
        while index < self.state.streams.len() {
            if self.state.streams[index].artifact_id == self.artifact_id {
                self.state.streams[index].stream.abort()?;
                self.state.streams.swap_remove(index);
            } else {
                index += 1;
            }
        }
        if let ArtifactPayload::Bytes(artifact) = &self.state.artifacts[artifact_index].payload {
            artifact.delete()?;
        }
        self.state.artifacts.swap_remove(artifact_index);
        Ok(ArtifactDisposal {
            object_ids_child_first: self.object_ids_child_first,
        })
    }
}

/// A stable Stream-close target held until runtime disposal commits.
pub struct PreparedStreamDisposal<'a> {
    state: MutexGuard<'a, ArtifactRegistryState>,
    stream_id: ObjectId,
}

impl PreparedStreamDisposal<'_> {
    pub fn commit(mut self) -> Result<(), ArtifactRegistryError> {
        let index = self
            .state
            .streams
            .iter()
            .position(|entry| entry.id == self.stream_id)
            .expect("a locked Stream disposal keeps its target live");
        self.state.streams[index].stream.close()?;
        self.state.streams.swap_remove(index);
        Ok(())
    }
}

pub struct ArtifactRegistry {
    limits: ArtifactRegistryLimits,
    ledger: ResourceLedger,
    state: Mutex<ArtifactRegistryState>,
}

impl ArtifactRegistry {
    pub fn new(
        limits: ArtifactRegistryLimits,
        ledger: ResourceLedger,
    ) -> Result<Self, ArtifactRegistryError> {
        limits.validate()?;
        let mut artifacts = Vec::new();
        artifacts
            .try_reserve_exact(limits.max_artifacts)
            .map_err(|_| ArtifactRegistryError::AllocationFailed {
                collection: "Artifact table",
                limit: limits.max_artifacts,
            })?;
        let mut streams = Vec::new();
        streams.try_reserve_exact(limits.max_streams).map_err(|_| {
            ArtifactRegistryError::AllocationFailed {
                collection: "Stream table",
                limit: limits.max_streams,
            }
        })?;
        Ok(Self {
            limits,
            ledger,
            state: Mutex::new(ArtifactRegistryState {
                artifacts,
                artifact_peak: 0,
                streams,
                stream_peak: 0,
            }),
        })
    }

    pub const fn limits(&self) -> ArtifactRegistryLimits {
        self.limits
    }

    pub const fn ledger(&self) -> &ResourceLedger {
        &self.ledger
    }

    pub fn register_bytes(
        &self,
        artifact_id: ObjectId,
        parent: ObjectId,
        source: &[u8],
    ) -> Result<(), ArtifactRegistryError> {
        self.prepare_register_bytes(artifact_id, parent, source)?
            .commit();
        Ok(())
    }

    pub fn prepare_register_bytes(
        &self,
        artifact_id: ObjectId,
        parent: ObjectId,
        source: &[u8],
    ) -> Result<PreparedArtifactRegistration<'_>, ArtifactRegistryError> {
        let state = self.lock_state()?;
        self.preflight_artifact_insert(&state, artifact_id)?;
        let artifact = Artifact::new(
            self.limits.stream_limits(),
            self.ledger.clone(),
            ResourceScope::Object(artifact_id),
            source,
        )?;
        Ok(PreparedArtifactRegistration {
            state,
            entry: Some(ArtifactEntry {
                id: artifact_id,
                parent,
                payload: ArtifactPayload::Bytes(artifact),
            }),
        })
    }

    /// Registers an already budgeted source without allocating or copying a
    /// second Artifact buffer.
    pub fn register_budgeted_bytes(
        &self,
        artifact_id: ObjectId,
        parent: ObjectId,
        source: BudgetedBytes,
    ) -> Result<(), ArtifactRegistryError> {
        self.prepare_register_budgeted_bytes(artifact_id, parent, source)?
            .commit();
        Ok(())
    }

    pub fn prepare_register_budgeted_bytes(
        &self,
        artifact_id: ObjectId,
        parent: ObjectId,
        source: BudgetedBytes,
    ) -> Result<PreparedArtifactRegistration<'_>, ArtifactRegistryError> {
        let expected = ResourceScope::Object(artifact_id);
        if source.scope() != expected {
            return Err(ArtifactRegistryError::ArtifactSourceScopeMismatch {
                actual: source.scope(),
                expected,
            });
        }
        let state = self.lock_state()?;
        self.preflight_artifact_insert(&state, artifact_id)?;
        let artifact = Artifact::from_budgeted_source(
            self.limits.stream_limits(),
            self.ledger.clone(),
            source,
        )?;
        Ok(PreparedArtifactRegistration {
            state,
            entry: Some(ArtifactEntry {
                id: artifact_id,
                parent,
                payload: ArtifactPayload::Bytes(artifact),
            }),
        })
    }

    pub fn register_failure(
        &self,
        artifact_id: ObjectId,
        parent: ObjectId,
        error: &str,
    ) -> Result<(), ArtifactRegistryError> {
        self.prepare_register_failure(artifact_id, parent, error)?
            .commit();
        Ok(())
    }

    pub fn prepare_register_failure(
        &self,
        artifact_id: ObjectId,
        parent: ObjectId,
        error: &str,
    ) -> Result<PreparedArtifactRegistration<'_>, ArtifactRegistryError> {
        if error.is_empty() {
            return Err(ArtifactRegistryError::EmptyFailure);
        }
        if error.len() > self.limits.max_failure_bytes {
            return Err(ArtifactRegistryError::FailureTooLarge {
                actual: error.len(),
                limit: self.limits.max_failure_bytes,
            });
        }
        let state = self.lock_state()?;
        self.preflight_artifact_insert(&state, artifact_id)?;
        let error = BudgetedUtf8::copy_from_str(
            &self.ledger,
            ResourceScope::Object(artifact_id),
            ResourceClass::ArtifactBytes,
            error,
        )?;
        Ok(PreparedArtifactRegistration {
            state,
            entry: Some(ArtifactEntry {
                id: artifact_id,
                parent,
                payload: ArtifactPayload::Failure(error),
            }),
        })
    }

    pub fn open_stream(
        &self,
        artifact_id: ObjectId,
        stream_id: ObjectId,
    ) -> Result<StreamInitializer, ArtifactRegistryError> {
        Ok(self.prepare_open_stream(artifact_id, stream_id)?.commit())
    }

    pub fn prepare_open_stream(
        &self,
        artifact_id: ObjectId,
        stream_id: ObjectId,
    ) -> Result<PreparedStreamRegistration<'_>, ArtifactRegistryError> {
        let state = self.lock_state()?;
        if self.contains_id(&state, stream_id) {
            return Err(ArtifactRegistryError::DuplicateObject(stream_id));
        }
        if state.streams.len() == self.limits.max_streams {
            return Err(ArtifactRegistryError::StreamCapacityExceeded {
                limit: self.limits.max_streams,
            });
        }
        let artifact = state
            .artifacts
            .iter()
            .find(|entry| entry.id == artifact_id)
            .ok_or(ArtifactRegistryError::MissingArtifact(artifact_id))?;
        let ArtifactPayload::Bytes(artifact) = &artifact.payload else {
            return Err(ArtifactRegistryError::FailedArtifact(artifact_id));
        };
        let stream = artifact.open_stream()?;
        let initializer = StreamInitializer::new(stream_id, artifact_id, STREAM_OBJECT_KIND.tag);
        Ok(PreparedStreamRegistration {
            state,
            entry: Some(StreamEntry {
                id: stream_id,
                artifact_id,
                stream,
            }),
            initializer,
        })
    }

    pub fn failure(
        &self,
        artifact_id: ObjectId,
        call_id: CallId,
    ) -> Result<ArtifactFailureResult, ArtifactRegistryError> {
        let state = self.lock_state()?;
        let artifact = state
            .artifacts
            .iter()
            .find(|entry| entry.id == artifact_id)
            .ok_or(ArtifactRegistryError::MissingArtifact(artifact_id))?;
        let error = match &artifact.payload {
            ArtifactPayload::Bytes(_) => None,
            ArtifactPayload::Failure(error) => Some(error.as_str()),
        };
        ArtifactFailureResult::copy_from_str(&self.ledger, ResourceScope::Call(call_id), error)
            .map_err(ArtifactRegistryError::Resource)
    }

    pub fn prepare_read(
        &self,
        stream_id: ObjectId,
        requested_bytes: usize,
        call_id: CallId,
    ) -> Result<PreparedArtifactRead, ArtifactRegistryError> {
        let state = self.lock_state()?;
        let stream = state
            .streams
            .iter()
            .find(|entry| entry.id == stream_id)
            .ok_or(ArtifactRegistryError::MissingStream(stream_id))?;
        stream
            .stream
            .prepare_read(requested_bytes, ResourceScope::Call(call_id))
            .map_err(ArtifactRegistryError::Stream)
    }

    pub fn close_stream(&self, stream_id: ObjectId) -> Result<bool, ArtifactRegistryError> {
        let state = self.lock_state()?;
        if !state.streams.iter().any(|entry| entry.id == stream_id) {
            return Ok(false);
        }
        PreparedStreamDisposal { state, stream_id }.commit()?;
        Ok(true)
    }

    pub fn prepare_stream_disposal(
        &self,
        stream_id: ObjectId,
    ) -> Result<PreparedStreamDisposal<'_>, ArtifactRegistryError> {
        let state = self.lock_state()?;
        if !state.streams.iter().any(|entry| entry.id == stream_id) {
            return Err(ArtifactRegistryError::MissingStream(stream_id));
        }
        Ok(PreparedStreamDisposal { state, stream_id })
    }

    pub fn delete_artifact(
        &self,
        artifact_id: ObjectId,
    ) -> Result<ArtifactDisposal, ArtifactRegistryError> {
        self.prepare_artifact_disposal(artifact_id)?.commit()
    }

    pub fn prepare_artifact_disposal(
        &self,
        artifact_id: ObjectId,
    ) -> Result<PreparedArtifactDisposal<'_>, ArtifactRegistryError> {
        let state = self.lock_state()?;
        state
            .artifacts
            .iter()
            .position(|entry| entry.id == artifact_id)
            .ok_or(ArtifactRegistryError::MissingArtifact(artifact_id))?;
        let child_count = state
            .streams
            .iter()
            .filter(|entry| entry.artifact_id == artifact_id)
            .count();
        let mut object_ids = Vec::new();
        object_ids
            .try_reserve_exact(child_count.saturating_add(1))
            .map_err(|_| ArtifactRegistryError::AllocationFailed {
                collection: "Artifact disposal plan",
                limit: child_count.saturating_add(1),
            })?;
        object_ids.extend(
            state
                .streams
                .iter()
                .filter_map(|entry| (entry.artifact_id == artifact_id).then_some(entry.id)),
        );
        object_ids.push(artifact_id);
        Ok(PreparedArtifactDisposal {
            state,
            artifact_id,
            object_ids_child_first: object_ids,
        })
    }

    pub fn dispose_parent(
        &self,
        parent: ObjectId,
    ) -> Result<ArtifactDisposal, ArtifactRegistryError> {
        let mut state = self.lock_state()?;
        let artifact_count = state
            .artifacts
            .iter()
            .filter(|entry| entry.parent == parent)
            .count();
        let mut artifact_ids = Vec::new();
        artifact_ids
            .try_reserve_exact(artifact_count)
            .map_err(|_| ArtifactRegistryError::AllocationFailed {
                collection: "parent Artifact ID plan",
                limit: artifact_count,
            })?;
        artifact_ids.extend(
            state
                .artifacts
                .iter()
                .filter_map(|entry| (entry.parent == parent).then_some(entry.id)),
        );
        let stream_count = state
            .streams
            .iter()
            .filter(|entry| artifact_ids.contains(&entry.artifact_id))
            .count();
        let count = artifact_count
            .checked_add(stream_count)
            .ok_or(ArtifactRegistryError::LimitOverflow)?;
        let mut object_ids = Vec::new();
        object_ids.try_reserve_exact(count).map_err(|_| {
            ArtifactRegistryError::AllocationFailed {
                collection: "parent disposal plan",
                limit: count,
            }
        })?;
        let mut stream_index = 0;
        while stream_index < state.streams.len() {
            if artifact_ids.contains(&state.streams[stream_index].artifact_id) {
                state.streams[stream_index].stream.abort()?;
                object_ids.push(state.streams[stream_index].id);
                state.streams.swap_remove(stream_index);
            } else {
                stream_index += 1;
            }
        }
        let mut artifact_index = 0;
        while artifact_index < state.artifacts.len() {
            if state.artifacts[artifact_index].parent == parent {
                if let ArtifactPayload::Bytes(artifact) = &state.artifacts[artifact_index].payload {
                    artifact.delete()?;
                }
                object_ids.push(state.artifacts[artifact_index].id);
                state.artifacts.swap_remove(artifact_index);
            } else {
                artifact_index += 1;
            }
        }
        Ok(ArtifactDisposal {
            object_ids_child_first: object_ids,
        })
    }

    pub fn abort_all(&self) -> Result<ArtifactDisposal, ArtifactRegistryError> {
        let mut state = self.lock_state()?;
        let count = state
            .artifacts
            .len()
            .checked_add(state.streams.len())
            .ok_or(ArtifactRegistryError::LimitOverflow)?;
        let mut object_ids = Vec::new();
        object_ids.try_reserve_exact(count).map_err(|_| {
            ArtifactRegistryError::AllocationFailed {
                collection: "connection disposal plan",
                limit: count,
            }
        })?;
        for entry in &state.streams {
            entry.stream.abort()?;
            object_ids.push(entry.id);
        }
        state.streams.clear();
        for entry in &state.artifacts {
            if let ArtifactPayload::Bytes(artifact) = &entry.payload {
                artifact.delete()?;
            }
            object_ids.push(entry.id);
        }
        state.artifacts.clear();
        Ok(ArtifactDisposal {
            object_ids_child_first: object_ids,
        })
    }

    pub fn metrics(&self) -> Result<ArtifactRegistryMetrics, ArtifactRegistryError> {
        let state = self.lock_state()?;
        Ok(ArtifactRegistryMetrics {
            artifacts: state.artifacts.len(),
            artifact_peak: state.artifact_peak,
            streams: state.streams.len(),
            stream_peak: state.stream_peak,
            max_artifacts: self.limits.max_artifacts,
            max_streams: self.limits.max_streams,
        })
    }

    fn preflight_artifact_insert(
        &self,
        state: &ArtifactRegistryState,
        artifact_id: ObjectId,
    ) -> Result<(), ArtifactRegistryError> {
        if self.contains_id(state, artifact_id) {
            return Err(ArtifactRegistryError::DuplicateObject(artifact_id));
        }
        if state.artifacts.len() == self.limits.max_artifacts {
            return Err(ArtifactRegistryError::ArtifactCapacityExceeded {
                limit: self.limits.max_artifacts,
            });
        }
        Ok(())
    }

    fn contains_id(&self, state: &ArtifactRegistryState, id: ObjectId) -> bool {
        state.artifacts.iter().any(|entry| entry.id == id)
            || state.streams.iter().any(|entry| entry.id == id)
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, ArtifactRegistryState>, ArtifactRegistryError> {
        self.state
            .lock()
            .map_err(|_| ArtifactRegistryError::StatePoisoned)
    }
}

#[derive(Debug)]
pub enum ArtifactRegistryError {
    ZeroLimit(&'static str),
    LimitOverflow,
    AllocationFailed {
        collection: &'static str,
        limit: usize,
    },
    DuplicateObject(ObjectId),
    ArtifactCapacityExceeded {
        limit: usize,
    },
    StreamCapacityExceeded {
        limit: usize,
    },
    MissingArtifact(ObjectId),
    MissingStream(ObjectId),
    FailedArtifact(ObjectId),
    EmptyFailure,
    FailureTooLarge {
        actual: usize,
        limit: usize,
    },
    ArtifactSourceScopeMismatch {
        actual: ResourceScope,
        expected: ResourceScope,
    },
    StatePoisoned,
    Resource(ResourceError),
    Production(ProductionResourceError),
    Stream(ArtifactStreamError),
}

impl Display for ArtifactRegistryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit(name) => write!(formatter, "artifact registry {name} must be non-zero"),
            Self::LimitOverflow => formatter.write_str("artifact registry limit overflow"),
            Self::AllocationFailed { collection, limit } => {
                write!(
                    formatter,
                    "could not preallocate {collection} with limit {limit}"
                )
            }
            Self::DuplicateObject(id) => {
                write!(formatter, "Artifact object ID {id} already exists")
            }
            Self::ArtifactCapacityExceeded { limit } => {
                write!(
                    formatter,
                    "artifact registry reached its {limit}-Artifact limit"
                )
            }
            Self::StreamCapacityExceeded { limit } => {
                write!(
                    formatter,
                    "artifact registry reached its {limit}-Stream limit"
                )
            }
            Self::MissingArtifact(id) => write!(formatter, "Artifact {id} is not registered"),
            Self::MissingStream(id) => write!(formatter, "Stream {id} is not registered"),
            Self::FailedArtifact(id) => write!(formatter, "Artifact {id} has no readable bytes"),
            Self::EmptyFailure => formatter.write_str("failed Artifact error must be non-empty"),
            Self::FailureTooLarge { actual, limit } => write!(
                formatter,
                "Artifact failure has {actual} bytes, exceeding limit {limit}"
            ),
            Self::ArtifactSourceScopeMismatch { actual, expected } => write!(
                formatter,
                "Artifact source scope {actual:?} does not match {expected:?}"
            ),
            Self::StatePoisoned => formatter.write_str("artifact registry state is poisoned"),
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::Production(error) => Display::fmt(error, formatter),
            Self::Stream(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ArtifactRegistryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resource(error) => Some(error),
            Self::Production(error) => Some(error),
            Self::Stream(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ResourceError> for ArtifactRegistryError {
    fn from(error: ResourceError) -> Self {
        Self::Resource(error)
    }
}

impl From<ProductionResourceError> for ArtifactRegistryError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Production(error)
    }
}

impl From<ArtifactStreamError> for ArtifactRegistryError {
    fn from(error: ArtifactStreamError) -> Self {
        Self::Stream(error)
    }
}

pub struct ArtifactService<Ids> {
    registry: Arc<ArtifactRegistry>,
    object_ids: Ids,
}

impl<Ids> ArtifactService<Ids> {
    pub const fn new(registry: Arc<ArtifactRegistry>, object_ids: Ids) -> Self {
        Self {
            registry,
            object_ids,
        }
    }

    pub const fn registry(&self) -> &Arc<ArtifactRegistry> {
        &self.registry
    }
}

pub type SharedArtifactServerServices<Ids> = ArtifactServerServices<
    ArtifactService<Ids>,
    ArtifactService<Ids>,
    ArtifactService<Ids>,
    ArtifactService<Ids>,
    ArtifactService<Ids>,
>;

pub fn artifact_server_services<Ids>(
    service: Arc<ArtifactService<Ids>>,
) -> SharedArtifactServerServices<Ids> {
    ArtifactServerServices {
        artifact_delete: Arc::clone(&service),
        artifact_failure: Arc::clone(&service),
        artifact_open_stream: Arc::clone(&service),
        stream_close: Arc::clone(&service),
        stream_read: service,
    }
}

impl<Ids> Service<Command<ArtifactOpenStream>> for ArtifactService<Ids>
where
    Ids: ObjectIdSource,
{
    type Response = CommandResult<ArtifactOpenStream>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<ArtifactOpenStream>) -> Self::Future<'_> {
        let result = self
            .object_ids
            .next_object_id()
            .map_err(|error| {
                pwright_core::OperationError::new(
                    FailureKind::ResourceExhausted,
                    error.to_string(),
                    Some(ArtifactErrorDetails::Resource(error.to_string())),
                )
            })
            .and_then(|stream_id| {
                self.registry
                    .open_stream(command.object_id(), stream_id)
                    .map_err(artifact_registry_operation_error)
            });
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Ids> Service<Command<ArtifactFailure>> for ArtifactService<Ids>
where
    Ids: ObjectIdSource,
{
    type Response = CommandResult<ArtifactFailure>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<ArtifactFailure>) -> Self::Future<'_> {
        let result = self
            .registry
            .failure(command.object_id(), command.call_id())
            .map_err(artifact_registry_operation_error);
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Ids> Service<Command<ArtifactDelete>> for ArtifactService<Ids>
where
    Ids: ObjectIdSource,
{
    type Response = CommandResult<ArtifactDelete>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<ArtifactDelete>) -> Self::Future<'_> {
        let result = self
            .registry
            .delete_artifact(command.object_id())
            .map(drop)
            .map_err(artifact_registry_operation_error);
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Ids> Service<Command<StreamRead>> for ArtifactService<Ids>
where
    Ids: ObjectIdSource,
{
    type Response = CommandResult<StreamRead>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<StreamRead>) -> Self::Future<'_> {
        let result = usize::try_from(command.params().max_bytes())
            .map_err(|_| ArtifactRegistryError::LimitOverflow)
            .and_then(|requested| {
                self.registry
                    .prepare_read(command.object_id(), requested, command.call_id())
            })
            .and_then(|prepared| prepared.commit().map_err(ArtifactRegistryError::Stream))
            .map_err(artifact_registry_operation_error);
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

impl<Ids> Service<Command<StreamClose>> for ArtifactService<Ids>
where
    Ids: ObjectIdSource,
{
    type Response = CommandResult<StreamClose>;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, command: Command<StreamClose>) -> Self::Future<'_> {
        let result = self
            .registry
            .close_stream(command.object_id())
            .and_then(|closed| {
                if closed {
                    Ok(())
                } else {
                    Err(ArtifactRegistryError::MissingStream(command.object_id()))
                }
            })
            .map_err(artifact_registry_operation_error);
        ready(Ok(CommandResult::new(command.call_id(), result)))
    }
}

pub fn artifact_registry_operation_error(
    error: ArtifactRegistryError,
) -> pwright_core::OperationError<ArtifactErrorDetails> {
    let message = error.to_string();
    match error {
        ArtifactRegistryError::MissingArtifact(_) | ArtifactRegistryError::MissingStream(_) => {
            pwright_core::OperationError::new(
                FailureKind::TargetClosed,
                "Artifact or Stream is closed",
                Some(ArtifactErrorDetails::Backend(message)),
            )
        }
        ArtifactRegistryError::ZeroLimit(_)
        | ArtifactRegistryError::LimitOverflow
        | ArtifactRegistryError::AllocationFailed { .. }
        | ArtifactRegistryError::ArtifactCapacityExceeded { .. }
        | ArtifactRegistryError::StreamCapacityExceeded { .. }
        | ArtifactRegistryError::FailureTooLarge { .. }
        | ArtifactRegistryError::Resource(_)
        | ArtifactRegistryError::Production(_)
        | ArtifactRegistryError::Stream(ArtifactStreamError::ArtifactTooLarge { .. })
        | ArtifactRegistryError::Stream(ArtifactStreamError::ReadTooLarge { .. })
        | ArtifactRegistryError::Stream(ArtifactStreamError::LimitOverflow)
        | ArtifactRegistryError::Stream(ArtifactStreamError::Resource(_))
        | ArtifactRegistryError::Stream(ArtifactStreamError::Production(_)) => {
            pwright_core::OperationError::new(
                FailureKind::ResourceExhausted,
                "Artifact resource limit reached",
                Some(ArtifactErrorDetails::Resource(message)),
            )
        }
        ArtifactRegistryError::EmptyFailure
        | ArtifactRegistryError::DuplicateObject(_)
        | ArtifactRegistryError::ArtifactSourceScopeMismatch { .. }
        | ArtifactRegistryError::Stream(ArtifactStreamError::ZeroLimit(_))
        | ArtifactRegistryError::Stream(ArtifactStreamError::InvalidSourceClass { .. })
        | ArtifactRegistryError::Stream(ArtifactStreamError::ZeroReadSize) => {
            pwright_core::OperationError::new(
                FailureKind::ProtocolViolation,
                "invalid Artifact operation",
                Some(ArtifactErrorDetails::InvalidOptions(message)),
            )
        }
        ArtifactRegistryError::FailedArtifact(_)
        | ArtifactRegistryError::StatePoisoned
        | ArtifactRegistryError::Stream(ArtifactStreamError::ReadAlreadyPrepared)
        | ArtifactRegistryError::Stream(ArtifactStreamError::ReadIdExhausted)
        | ArtifactRegistryError::Stream(ArtifactStreamError::PreparedReadInvalidated)
        | ArtifactRegistryError::Stream(ArtifactStreamError::Deleted)
        | ArtifactRegistryError::Stream(ArtifactStreamError::Closed)
        | ArtifactRegistryError::Stream(ArtifactStreamError::StatePoisoned) => {
            pwright_core::OperationError::new(
                FailureKind::Backend,
                "Artifact operation failed",
                Some(ArtifactErrorDetails::Backend(message)),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cap_artifact::{
        ArtifactDelete, ArtifactFailure, ArtifactOpenStream, StreamClose, StreamRead,
        StreamReadOptions,
    };
    use pwright_client::{AtomicCallIdSource, BrowserKind, Client, RemoteInitializer, RemoteKind};
    use pwright_client_artifact::{ArtifactExt, StreamExt};
    use pwright_client_wire::{DriverEvent, EncodedClientChannel};
    use pwright_codec_artifact::{ArtifactCodec, ArtifactCodecLimits};
    use pwright_core::{
        AtomicObjectIdSource, BudgetedBytes, CallId, CallMetadata, Command, ObjectId,
        ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope, Service,
    };
    use pwright_server_artifact::artifact_server_handlers;
    use pwright_server_wire::{ConcurrentServerFrameHandler, ServerFrameDisposition};
    use pwright_transport::{FrameLimits, FrameTransport, WireProtocol};
    use pwright_transport_memory::MemoryFrameTransport;

    use super::{
        ArtifactRegistry, ArtifactRegistryError, ArtifactRegistryLimits,
        ArtifactRegistryProductionProfile, ArtifactService, artifact_server_services,
    };

    fn limits(max_artifacts: usize, max_streams: usize) -> ArtifactRegistryLimits {
        ArtifactRegistryLimits::new(max_artifacts, max_streams, 8, 3, 8, 16)
    }

    fn registry_with_limits(
        limits: ArtifactRegistryLimits,
    ) -> (Arc<ArtifactRegistry>, ProductionResourceLedger) {
        let profile = ArtifactRegistryProductionProfile::new(limits).unwrap();
        let requirements = profile.requirements();
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::ArtifactBytes,
                requirements.class(ResourceClass::ArtifactBytes).global,
                requirements.class(ResourceClass::ArtifactBytes).per_scope,
            ),
            requirements,
        )
        .unwrap();
        let registry = Arc::new(profile.create(&resources).unwrap());
        (registry, resources)
    }

    fn command<Spec: pwright_core::CommandSpec>(
        call_id: u64,
        object_id: u64,
        params: Spec::Params,
    ) -> Command<Spec> {
        Command::new(
            CallId::new(call_id),
            ObjectId::new(object_id),
            CallMetadata::default(),
            params,
        )
    }

    fn wire_protocol(ledger: &pwright_core::ResourceLedger) -> WireProtocol<ArtifactCodec> {
        let limits = FrameLimits::new(4_096, 1, 8, 4_104);
        WireProtocol::new_exact_directional(
            ArtifactCodec::new(ArtifactCodecLimits::new(8, 64), ledger.clone()),
            pwright_core::ProtocolVersion::new(1, 0),
            limits,
            limits,
        )
    }

    #[test]
    fn production_profile_accounts_for_sources_and_retained_results() {
        // Arrange
        let limits = ArtifactRegistryLimits::new(2, 3, 8, 3, 5, 7);

        // Act
        let profile = ArtifactRegistryProductionProfile::new(limits).unwrap();
        let requirements = profile.requirements();

        // Assert
        assert_eq!(profile.limits(), limits);
        assert_eq!(requirements.class(ResourceClass::ArtifactBytes).global, 23);
        assert_eq!(
            requirements.class(ResourceClass::ArtifactBytes).per_scope,
            8
        );
        assert_eq!(requirements.scopes, 9);
        assert_eq!(requirements.leases, 9);
    }

    #[test]
    fn multiple_streams_share_source_bytes_and_delete_child_first() {
        // Arrange
        let (registry, resources) = registry_with_limits(limits(1, 2));
        registry
            .register_bytes(ObjectId::new(10), ObjectId::new(2), b"abcdef")
            .unwrap();
        registry
            .open_stream(ObjectId::new(10), ObjectId::new(11))
            .unwrap();
        registry
            .open_stream(ObjectId::new(10), ObjectId::new(12))
            .unwrap();

        // Act
        let first = registry
            .prepare_read(ObjectId::new(11), 3, CallId::new(1))
            .unwrap()
            .commit()
            .unwrap();
        let second = registry
            .prepare_read(ObjectId::new(12), 2, CallId::new(2))
            .unwrap()
            .commit()
            .unwrap();
        let during_reads = resources.metrics().unwrap();
        let disposal = registry.delete_artifact(ObjectId::new(10)).unwrap();
        let after_delete = resources.metrics().unwrap();

        // Assert
        assert_eq!(first.as_bytes(), b"abc");
        assert_eq!(second.as_bytes(), b"ab");
        assert_eq!(during_reads.class(ResourceClass::ArtifactBytes).current, 11);
        assert_eq!(
            disposal.object_ids_child_first(),
            &[ObjectId::new(11), ObjectId::new(12), ObjectId::new(10)]
        );
        assert_eq!(registry.metrics().unwrap().artifacts, 0);
        assert_eq!(registry.metrics().unwrap().streams, 0);
        assert_eq!(after_delete.class(ResourceClass::ArtifactBytes).current, 5);
        drop((first, second));
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            0
        );
    }

    #[test]
    fn count_and_payload_limits_reject_before_registry_mutation() {
        // Arrange
        let (registry, _) = registry_with_limits(limits(1, 1));
        let (payload_registry, _) = registry_with_limits(limits(1, 1));
        registry
            .register_bytes(ObjectId::new(10), ObjectId::new(2), b"12345678")
            .unwrap();
        registry
            .open_stream(ObjectId::new(10), ObjectId::new(11))
            .unwrap();

        // Act
        let artifact_full = registry.register_bytes(ObjectId::new(12), ObjectId::new(2), b"");
        let stream_full = registry.open_stream(ObjectId::new(10), ObjectId::new(13));
        let duplicate = registry.register_bytes(ObjectId::new(11), ObjectId::new(2), b"");
        let oversized =
            payload_registry.register_bytes(ObjectId::new(14), ObjectId::new(2), b"123456789");

        // Assert
        assert!(matches!(
            artifact_full,
            Err(ArtifactRegistryError::ArtifactCapacityExceeded { limit: 1 })
        ));
        assert!(matches!(
            stream_full,
            Err(ArtifactRegistryError::StreamCapacityExceeded { limit: 1 })
        ));
        assert!(matches!(
            duplicate,
            Err(ArtifactRegistryError::DuplicateObject(id)) if id == ObjectId::new(11)
        ));
        assert!(matches!(
            oversized,
            Err(ArtifactRegistryError::Stream(
                pwright_artifact_model::ArtifactStreamError::ArtifactTooLarge {
                    actual: 9,
                    limit: 8
                }
            ))
        ));
        let metrics = registry.metrics().unwrap();
        assert_eq!(metrics.artifacts, 1);
        assert_eq!(metrics.streams, 1);
        assert_eq!(payload_registry.metrics().unwrap().artifacts, 0);
    }

    #[test]
    fn failed_artifact_is_queryable_but_cannot_open_a_stream() {
        // Arrange
        let (registry, resources) = registry_with_limits(limits(1, 1));
        registry
            .register_failure(ObjectId::new(10), ObjectId::new(2), "failed")
            .unwrap();

        // Act
        let failure = registry.failure(ObjectId::new(10), CallId::new(1)).unwrap();
        let opening = registry.open_stream(ObjectId::new(10), ObjectId::new(11));
        let during_result = resources.metrics().unwrap();
        drop(failure);
        let after_result = resources.metrics().unwrap();
        registry.delete_artifact(ObjectId::new(10)).unwrap();

        // Assert
        assert!(matches!(
            opening,
            Err(ArtifactRegistryError::FailedArtifact(id)) if id == ObjectId::new(10)
        ));
        assert_eq!(
            during_result.class(ResourceClass::ArtifactBytes).current,
            12
        );
        assert_eq!(after_result.class(ResourceClass::ArtifactBytes).current, 6);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            0
        );
    }

    #[test]
    fn prebudgeted_source_enters_registry_without_a_second_copy_or_lease() {
        // Arrange
        let (registry, resources) = registry_with_limits(limits(1, 1));
        let source = BudgetedBytes::copy_from_slice(
            registry.ledger(),
            ResourceScope::Object(ObjectId::new(10)),
            ResourceClass::ArtifactBytes,
            b"abcdef",
        )
        .unwrap();
        let before = resources
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current;

        // Act
        registry
            .register_budgeted_bytes(ObjectId::new(10), ObjectId::new(2), source)
            .unwrap();
        let after = resources
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current;

        // Assert
        assert_eq!(before, 6);
        assert_eq!(after, before);
        assert_eq!(registry.metrics().unwrap().artifacts, 1);
    }

    #[test]
    fn prebudgeted_source_scope_mismatch_fails_before_registry_mutation() {
        // Arrange
        let (registry, resources) = registry_with_limits(limits(1, 1));
        let source = BudgetedBytes::copy_from_slice(
            registry.ledger(),
            ResourceScope::Object(ObjectId::new(99)),
            ResourceClass::ArtifactBytes,
            b"abc",
        )
        .unwrap();

        // Act
        let result = registry.register_budgeted_bytes(ObjectId::new(10), ObjectId::new(2), source);

        // Assert
        assert!(matches!(
            result,
            Err(ArtifactRegistryError::ArtifactSourceScopeMismatch {
                actual: ResourceScope::Object(actual),
                expected: ResourceScope::Object(expected),
            }) if actual == ObjectId::new(99) && expected == ObjectId::new(10)
        ));
        assert_eq!(registry.metrics().unwrap().artifacts, 0);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            0
        );
    }

    #[test]
    fn parent_disposal_invalidates_a_prepared_read_and_releases_every_owner() {
        // Arrange
        let (registry, resources) = registry_with_limits(limits(2, 2));
        registry
            .register_bytes(ObjectId::new(10), ObjectId::new(2), b"abcdef")
            .unwrap();
        registry
            .register_failure(ObjectId::new(20), ObjectId::new(3), "other")
            .unwrap();
        registry
            .open_stream(ObjectId::new(10), ObjectId::new(11))
            .unwrap();
        let prepared = registry
            .prepare_read(ObjectId::new(11), 3, CallId::new(1))
            .unwrap();

        // Act
        let disposal = registry.dispose_parent(ObjectId::new(2)).unwrap();
        let committed = prepared.commit();

        // Assert
        assert_eq!(
            disposal.object_ids_child_first(),
            &[ObjectId::new(11), ObjectId::new(10)]
        );
        assert!(matches!(
            committed,
            Err(pwright_artifact_model::ArtifactStreamError::PreparedReadInvalidated)
        ));
        assert_eq!(registry.metrics().unwrap().artifacts, 1);
        assert_eq!(registry.metrics().unwrap().streams, 0);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            5
        );
        registry.abort_all().unwrap();
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn five_services_share_registry_state_and_burn_monotonic_stream_ids() {
        // Arrange
        let (registry, _) = registry_with_limits(limits(1, 1));
        registry
            .register_bytes(ObjectId::new(10), ObjectId::new(2), b"abcdef")
            .unwrap();
        let service = Arc::new(ArtifactService::new(
            Arc::clone(&registry),
            AtomicObjectIdSource::new(11).unwrap(),
        ));

        // Act
        let stream = service
            .call(command::<ArtifactOpenStream>(1, 10, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        let failure = service
            .call(command::<ArtifactFailure>(2, 10, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        let read = service
            .call(command::<StreamRead>(
                3,
                stream.object_id.get(),
                StreamReadOptions::new(3),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        service
            .call(command::<StreamClose>(4, stream.object_id.get(), ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        service
            .call(command::<ArtifactDelete>(5, 10, ()))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();

        // Assert
        assert_eq!(stream.object_id, ObjectId::new(11));
        assert_eq!(failure.error(), None);
        assert_eq!(read.as_bytes(), b"abc");
        assert!(!read.eof());
        assert_eq!(registry.metrics().unwrap().artifacts, 0);
        assert_eq!(registry.metrics().unwrap().streams, 0);
    }

    #[test]
    fn generated_service_bundle_reuses_one_exact_registry_service() {
        // Arrange
        let (registry, _) = registry_with_limits(limits(1, 1));
        let service = Arc::new(ArtifactService::new(
            registry,
            AtomicObjectIdSource::new(11).unwrap(),
        ));

        // Act
        let services = artifact_server_services(Arc::clone(&service));

        // Assert
        assert!(Arc::ptr_eq(&services.artifact_delete, &service));
        assert!(Arc::ptr_eq(&services.artifact_failure, &service));
        assert!(Arc::ptr_eq(&services.artifact_open_stream, &service));
        assert!(Arc::ptr_eq(&services.stream_close, &service));
        assert!(Arc::ptr_eq(&services.stream_read, &service));
    }

    #[tokio::test]
    async fn typed_client_pulls_bytes_through_generated_server_and_binary_attachment() {
        macro_rules! round_trip {
            ($call:expr, $handler:expr, $server:expr, $driver:expr) => {{
                let call = $call;
                let server_turn = async {
                    let frame = $server
                        .receive()
                        .await
                        .expect("server receive should succeed")
                        .expect("server should receive command");
                    $handler
                        .handle_concurrently(frame, Arc::clone(&$server))
                        .await
                        .expect("server should handle command")
                };
                let (result, handled, delivered) =
                    tokio::join!(call, server_turn, $driver.receive_once());
                assert!(matches!(handled, ServerFrameDisposition::Handled(_)));
                assert!(matches!(delivered, Ok(DriverEvent::Delivered(_))));
                result
            }};
        }

        // Arrange
        let (registry, _) = registry_with_limits(limits(1, 1));
        registry
            .register_bytes(ObjectId::new(10), ObjectId::new(1), b"abcdef")
            .unwrap();
        let service = Arc::new(ArtifactService::new(
            Arc::clone(&registry),
            AtomicObjectIdSource::new(11).unwrap(),
        ));
        let client_result_ledger = pwright_core::ResourceLedger::new(
            ResourceLimits::deny_all(8, 8).with_class(ResourceClass::ArtifactBytes, 32, 8),
        );
        let server_result_ledger = pwright_core::ResourceLedger::new(
            ResourceLimits::deny_all(8, 8).with_class(ResourceClass::ArtifactBytes, 32, 8),
        );
        let (client_transport, server_transport) = MemoryFrameTransport::pair(2);
        let server_transport = Arc::new(server_transport);
        let (channel, mut client_driver) =
            EncodedClientChannel::new(wire_protocol(&client_result_ledger), client_transport, 1);
        let client = Client::new(channel, AtomicCallIdSource::new(20));
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BrowserKind::OBJECT_KIND,
            ))
            .unwrap();
        let artifact = browser
            .artifact_from_result(RemoteInitializer::new(
                ObjectId::new(10),
                Some(ObjectId::new(1)),
                pwright_browser_model::ARTIFACT_OBJECT_KIND.tag,
            ))
            .unwrap();
        let handler = artifact_server_handlers(
            Arc::new(wire_protocol(&server_result_ledger)),
            artifact_server_services(service),
        );

        // Act
        let stream = round_trip!(
            artifact.create_read_stream(),
            handler,
            server_transport,
            client_driver
        )
        .unwrap();
        let first = round_trip!(stream.read(3), handler, server_transport, client_driver).unwrap();
        let second = round_trip!(stream.read(3), handler, server_transport, client_driver).unwrap();
        round_trip!(stream.close(), handler, server_transport, client_driver).unwrap();
        round_trip!(artifact.delete(), handler, server_transport, client_driver).unwrap();

        // Assert
        assert_eq!(first.as_bytes(), b"abc");
        assert!(!first.eof());
        assert_eq!(second.as_bytes(), b"def");
        assert!(second.eof());
        assert_eq!(registry.metrics().unwrap().artifacts, 0);
        assert_eq!(registry.metrics().unwrap().streams, 0);
        assert_eq!(client.channel().metrics().unwrap().awaiting_results(), 0);
        assert_eq!(server_transport.metrics().unwrap().incoming_depth(), 0);
        assert_eq!(server_transport.metrics().unwrap().outgoing_depth(), 0);
    }
}
