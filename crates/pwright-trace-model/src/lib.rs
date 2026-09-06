//! Core-only bounded tracing state and retained payload ownership.
//!
//! This crate records ordered API metadata, groups, and chunks. It installs no
//! browser backend, DOM/ARIA snapshotter, screenshot or screencast producer,
//! network/HAR recorder, source collector, filesystem, archive writer,
//! transport, executor, or background task. Those are independent optional
//! contributors and sinks.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use pwright_core::{
    BudgetedBytes, BudgetedUtf8, CallId, ProductionResourceError, ProductionResourceLedger,
    ResourceClass, ResourceError, ResourceLedger, ResourceRequirements, ResourceScope,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceLimits {
    pub max_label_bytes: usize,
    pub max_entries_per_chunk: usize,
    pub max_entry_bytes: usize,
    pub max_chunk_bytes: usize,
    pub max_group_depth: usize,
}

impl TraceLimits {
    pub const fn new(
        max_label_bytes: usize,
        max_entries_per_chunk: usize,
        max_entry_bytes: usize,
        max_chunk_bytes: usize,
        max_group_depth: usize,
    ) -> Self {
        Self {
            max_label_bytes,
            max_entries_per_chunk,
            max_entry_bytes,
            max_chunk_bytes,
            max_group_depth,
        }
    }

    pub fn validate(self) -> Result<(), TraceError> {
        for (name, value) in [
            ("label bytes", self.max_label_bytes),
            ("entries per chunk", self.max_entries_per_chunk),
            ("entry bytes", self.max_entry_bytes),
            ("chunk bytes", self.max_chunk_bytes),
        ] {
            if value == 0 {
                return Err(TraceError::ZeroLimit(name));
            }
        }
        if self.max_entry_bytes > self.max_chunk_bytes {
            return Err(TraceError::EntryLimitExceedsChunk {
                entry: self.max_entry_bytes,
                chunk: self.max_chunk_bytes,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceRecorderProductionProfile {
    limits: TraceLimits,
    scope: ResourceScope,
    required_trace_bytes: u64,
    required_leases: usize,
}

impl TraceRecorderProductionProfile {
    pub fn new(limits: TraceLimits, scope: ResourceScope) -> Result<Self, TraceError> {
        limits.validate()?;
        let labels = limits
            .max_label_bytes
            .checked_mul(3)
            .ok_or(TraceError::LimitOverflow)?;
        let retained = labels
            .checked_add(limits.max_chunk_bytes)
            .ok_or(TraceError::LimitOverflow)?;
        let required_trace_bytes =
            u64::try_from(retained).map_err(|_| TraceError::LimitOverflow)?;
        let required_leases = limits
            .max_entries_per_chunk
            .checked_add(3)
            .ok_or(TraceError::LimitOverflow)?;
        Ok(Self {
            limits,
            scope,
            required_trace_bytes,
            required_leases,
        })
    }

    pub const fn limits(self) -> TraceLimits {
        self.limits
    }

    pub const fn scope(self) -> ResourceScope {
        self.scope
    }

    pub const fn requirements(self) -> ResourceRequirements {
        ResourceRequirements::none()
            .with_class(
                ResourceClass::TraceBytes,
                self.required_trace_bytes,
                self.required_trace_bytes,
            )
            .with_metadata(1, self.required_leases)
    }

    pub fn create(
        self,
        resources: &ProductionResourceLedger,
    ) -> Result<TraceRecorder, TraceProductionError> {
        let ledger = resources.claim(self.requirements())?;
        TraceRecorder::new(self.limits, ledger, self.scope).map_err(Into::into)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceChunkId(u64);

impl TraceChunkId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceEntryKind {
    ApiCallBefore,
    ApiCallAfter,
    ApiCallError,
    Log,
    GroupStart,
    GroupEnd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceSourceLocation<'a> {
    file: &'a str,
    line: Option<u32>,
    column: Option<u32>,
}

impl<'a> TraceSourceLocation<'a> {
    pub const fn new(file: &'a str, line: Option<u32>, column: Option<u32>) -> Self {
        Self { file, line, column }
    }

    pub const fn file(self) -> &'a str {
        self.file
    }

    pub const fn line(self) -> Option<u32> {
        self.line
    }

    pub const fn column(self) -> Option<u32> {
        self.column
    }
}

#[derive(Debug)]
pub struct TraceEntry {
    sequence: u64,
    monotonic_micros: u64,
    kind: TraceEntryKind,
    payload: BudgetedBytes,
}

impl TraceEntry {
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn monotonic_micros(&self) -> u64 {
        self.monotonic_micros
    }

    pub const fn kind(&self) -> TraceEntryKind {
        self.kind
    }

    pub fn payload(&self) -> &[u8] {
        self.payload.as_bytes()
    }
}

#[derive(Debug)]
pub struct TraceChunk {
    id: TraceChunkId,
    name: Option<BudgetedUtf8>,
    title: Option<BudgetedUtf8>,
    entries: Vec<TraceEntry>,
    payload_bytes: usize,
    detached: Arc<AtomicBool>,
}

impl Drop for TraceChunk {
    fn drop(&mut self) {
        drop(self.name.take());
        drop(self.title.take());
        drop(std::mem::take(&mut self.entries));
        self.detached.store(false, Ordering::Release);
    }
}

impl TraceChunk {
    pub const fn id(&self) -> TraceChunkId {
        self.id
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(BudgetedUtf8::as_str)
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().map(BudgetedUtf8::as_str)
    }

    pub fn entries(&self) -> &[TraceEntry] {
        &self.entries
    }

    pub const fn payload_bytes(&self) -> usize {
        self.payload_bytes
    }
}

#[derive(Debug)]
struct RecordingChunk {
    id: TraceChunkId,
    name: Option<BudgetedUtf8>,
    title: Option<BudgetedUtf8>,
    entries: Vec<TraceEntry>,
    payload_bytes: usize,
    group_depth: usize,
}

#[derive(Debug)]
enum TraceState {
    Idle,
    Started {
        session_name: Option<BudgetedUtf8>,
        next_chunk_id: u64,
    },
    Recording {
        session_name: Option<BudgetedUtf8>,
        next_chunk_id: u64,
        chunk: Box<RecordingChunk>,
    },
}

#[derive(Debug)]
pub struct TraceRecorder {
    limits: TraceLimits,
    ledger: ResourceLedger,
    scope: ResourceScope,
    state: TraceState,
    next_sequence: u64,
    detached: Arc<AtomicBool>,
}

impl TraceRecorder {
    pub fn new(
        limits: TraceLimits,
        ledger: ResourceLedger,
        scope: ResourceScope,
    ) -> Result<Self, TraceError> {
        limits.validate()?;
        Ok(Self {
            limits,
            ledger,
            scope,
            state: TraceState::Idle,
            next_sequence: 1,
            detached: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn is_started(&self) -> bool {
        !matches!(&self.state, TraceState::Idle)
    }

    pub fn is_recording(&self) -> bool {
        matches!(&self.state, TraceState::Recording { .. })
    }

    pub const fn limits(&self) -> TraceLimits {
        self.limits
    }

    pub fn start(&mut self, session_name: Option<&str>) -> Result<(), TraceError> {
        if self.detached.load(Ordering::Acquire) {
            return Err(TraceError::DetachedChunkPending);
        }
        if !matches!(self.state, TraceState::Idle) {
            return Err(TraceError::AlreadyStarted);
        }
        let session_name = self.copy_label("session name", session_name)?;
        self.state = TraceState::Started {
            session_name,
            next_chunk_id: 1,
        };
        Ok(())
    }

    pub fn start_chunk(
        &mut self,
        name: Option<&str>,
        title: Option<&str>,
    ) -> Result<TraceChunkId, TraceError> {
        if self.detached.load(Ordering::Acquire) {
            return Err(TraceError::DetachedChunkPending);
        }
        let next_chunk_id = match &self.state {
            TraceState::Idle => return Err(TraceError::NotStarted),
            TraceState::Recording { .. } => return Err(TraceError::ChunkAlreadyStarted),
            TraceState::Started { next_chunk_id, .. } => *next_chunk_id,
        };
        let name = self.copy_label("chunk name", name)?;
        let title = self.copy_label("chunk title", title)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(self.limits.max_entries_per_chunk)
            .map_err(|_| TraceError::EntryAllocationFailed {
                requested: self.limits.max_entries_per_chunk,
            })?;
        let following = next_chunk_id
            .checked_add(1)
            .ok_or(TraceError::ChunkIdExhausted)?;
        let id = TraceChunkId(next_chunk_id);
        let state = std::mem::replace(&mut self.state, TraceState::Idle);
        let TraceState::Started { session_name, .. } = state else {
            unreachable!("started state was validated before allocation")
        };
        self.state = TraceState::Recording {
            session_name,
            next_chunk_id: following,
            chunk: Box::new(RecordingChunk {
                id,
                name,
                title,
                entries,
                payload_bytes: 0,
                group_depth: 0,
            }),
        };
        Ok(id)
    }

    pub fn append(
        &mut self,
        kind: TraceEntryKind,
        monotonic_micros: u64,
        payload: &[u8],
    ) -> Result<u64, TraceError> {
        let TraceState::Recording { chunk, .. } = &self.state else {
            return Err(TraceError::NoActiveChunk);
        };
        if payload.len() > self.limits.max_entry_bytes {
            return Err(TraceError::EntryTooLarge {
                actual: payload.len(),
                limit: self.limits.max_entry_bytes,
            });
        }
        if chunk.entries.len() == self.limits.max_entries_per_chunk {
            return Err(TraceError::EntryCapacityExceeded {
                limit: self.limits.max_entries_per_chunk,
            });
        }
        let next_payload_bytes = chunk
            .payload_bytes
            .checked_add(payload.len())
            .ok_or(TraceError::LimitOverflow)?;
        if next_payload_bytes > self.limits.max_chunk_bytes {
            return Err(TraceError::ChunkBytesExceeded {
                actual: next_payload_bytes,
                limit: self.limits.max_chunk_bytes,
            });
        }
        let sequence = self.next_sequence;
        let following = sequence
            .checked_add(1)
            .ok_or(TraceError::SequenceExhausted)?;
        let payload = BudgetedBytes::copy_from_slice(
            &self.ledger,
            self.scope,
            ResourceClass::TraceBytes,
            payload,
        )?;
        let TraceState::Recording { chunk, .. } = &mut self.state else {
            unreachable!("recording state was validated before allocation")
        };
        chunk.entries.push(TraceEntry {
            sequence,
            monotonic_micros,
            kind,
            payload,
        });
        chunk.payload_bytes = next_payload_bytes;
        self.next_sequence = following;
        Ok(sequence)
    }

    /// Records one correlated API-call boundary without constructing an
    /// intermediate formatted string. The payload is version, call ID, method
    /// byte length, then UTF-8 method bytes.
    pub fn append_api_call(
        &mut self,
        kind: TraceEntryKind,
        monotonic_micros: u64,
        call_id: CallId,
        method: &str,
    ) -> Result<u64, TraceError> {
        if !matches!(
            kind,
            TraceEntryKind::ApiCallBefore
                | TraceEntryKind::ApiCallAfter
                | TraceEntryKind::ApiCallError
        ) {
            return Err(TraceError::InvalidApiCallKind(kind));
        }
        if method.len() > self.limits.max_label_bytes {
            return Err(TraceError::LabelTooLarge {
                field: "API method",
                actual: method.len(),
                limit: self.limits.max_label_bytes,
            });
        }
        let version = [1_u8];
        let call_id = call_id.get().to_be_bytes();
        let method_length = u32::try_from(method.len())
            .map_err(|_| TraceError::LabelEncodingOverflow {
                field: "API method",
            })?
            .to_be_bytes();
        self.append_parts(
            kind,
            monotonic_micros,
            &[&version, &call_id, &method_length, method.as_bytes()],
        )
    }

    pub fn begin_group(&mut self, monotonic_micros: u64, name: &str) -> Result<u64, TraceError> {
        self.begin_group_at(monotonic_micros, name, None)
    }

    pub fn begin_group_at(
        &mut self,
        monotonic_micros: u64,
        name: &str,
        location: Option<TraceSourceLocation<'_>>,
    ) -> Result<u64, TraceError> {
        let depth = match &self.state {
            TraceState::Recording { chunk, .. } => chunk.group_depth,
            _ => return Err(TraceError::NoActiveChunk),
        };
        if depth == self.limits.max_group_depth {
            return Err(TraceError::GroupDepthExceeded {
                limit: self.limits.max_group_depth,
            });
        }
        if name.len() > self.limits.max_label_bytes {
            return Err(TraceError::LabelTooLarge {
                field: "group name",
                actual: name.len(),
                limit: self.limits.max_label_bytes,
            });
        }
        if let Some(location) = location
            && location.file().len() > self.limits.max_label_bytes
        {
            return Err(TraceError::LabelTooLarge {
                field: "group location file",
                actual: location.file().len(),
                limit: self.limits.max_label_bytes,
            });
        }
        let name_length = u32::try_from(name.len())
            .map_err(|_| TraceError::LabelEncodingOverflow {
                field: "group name",
            })?
            .to_be_bytes();
        let version = [1_u8];
        let location_present = [u8::from(location.is_some())];
        let sequence = if let Some(location) = location {
            let file_length = u32::try_from(location.file().len())
                .map_err(|_| TraceError::LabelEncodingOverflow {
                    field: "group location file",
                })?
                .to_be_bytes();
            let line_present = [u8::from(location.line().is_some())];
            let line = location.line().unwrap_or_default().to_be_bytes();
            let column_present = [u8::from(location.column().is_some())];
            let column = location.column().unwrap_or_default().to_be_bytes();
            self.append_parts(
                TraceEntryKind::GroupStart,
                monotonic_micros,
                &[
                    &version,
                    &name_length,
                    name.as_bytes(),
                    &location_present,
                    &file_length,
                    location.file().as_bytes(),
                    &line_present,
                    &line,
                    &column_present,
                    &column,
                ],
            )?
        } else {
            self.append_parts(
                TraceEntryKind::GroupStart,
                monotonic_micros,
                &[&version, &name_length, name.as_bytes(), &location_present],
            )?
        };
        let TraceState::Recording { chunk, .. } = &mut self.state else {
            unreachable!("append preserves the recording state")
        };
        chunk.group_depth += 1;
        Ok(sequence)
    }

    pub fn end_group(&mut self, monotonic_micros: u64) -> Result<u64, TraceError> {
        let depth = match &self.state {
            TraceState::Recording { chunk, .. } => chunk.group_depth,
            _ => return Err(TraceError::NoActiveChunk),
        };
        if depth == 0 {
            return Err(TraceError::NoOpenGroup);
        }
        let sequence = self.append(TraceEntryKind::GroupEnd, monotonic_micros, &[])?;
        let TraceState::Recording { chunk, .. } = &mut self.state else {
            unreachable!("append preserves the recording state")
        };
        chunk.group_depth -= 1;
        Ok(sequence)
    }

    pub fn stop_chunk(&mut self) -> Result<TraceChunk, TraceError> {
        let TraceState::Recording { chunk, .. } = &self.state else {
            return Err(TraceError::NoActiveChunk);
        };
        if chunk.group_depth != 0 {
            return Err(TraceError::UnclosedGroups {
                count: chunk.group_depth,
            });
        }
        let state = std::mem::replace(&mut self.state, TraceState::Idle);
        let TraceState::Recording {
            session_name,
            next_chunk_id,
            chunk,
        } = state
        else {
            unreachable!("recording state was validated")
        };
        let chunk = *chunk;
        self.state = TraceState::Started {
            session_name,
            next_chunk_id,
        };
        self.detached.store(true, Ordering::Release);
        Ok(TraceChunk {
            id: chunk.id,
            name: chunk.name,
            title: chunk.title,
            entries: chunk.entries,
            payload_bytes: chunk.payload_bytes,
            detached: Arc::clone(&self.detached),
        })
    }

    pub fn discard_chunk(&mut self) -> Result<TraceChunkId, TraceError> {
        let state = std::mem::replace(&mut self.state, TraceState::Idle);
        match state {
            TraceState::Recording {
                session_name,
                next_chunk_id,
                chunk,
            } => {
                let id = chunk.id;
                drop(chunk);
                self.state = TraceState::Started {
                    session_name,
                    next_chunk_id,
                };
                Ok(id)
            }
            other => {
                self.state = other;
                Err(TraceError::NoActiveChunk)
            }
        }
    }

    pub fn stop(&mut self) -> Result<(), TraceError> {
        match &self.state {
            TraceState::Idle => Err(TraceError::NotStarted),
            TraceState::Recording { .. } => Err(TraceError::ActiveChunk),
            TraceState::Started { .. } => {
                self.state = TraceState::Idle;
                Ok(())
            }
        }
    }

    pub fn abort(&mut self) -> bool {
        let was_started = self.is_started();
        self.state = TraceState::Idle;
        was_started
    }

    fn copy_label(
        &self,
        field: &'static str,
        value: Option<&str>,
    ) -> Result<Option<BudgetedUtf8>, TraceError> {
        let Some(value) = value else {
            return Ok(None);
        };
        if value.len() > self.limits.max_label_bytes {
            return Err(TraceError::LabelTooLarge {
                field,
                actual: value.len(),
                limit: self.limits.max_label_bytes,
            });
        }
        BudgetedUtf8::copy_from_str(&self.ledger, self.scope, ResourceClass::TraceBytes, value)
            .map(Some)
            .map_err(Into::into)
    }

    fn append_parts(
        &mut self,
        kind: TraceEntryKind,
        monotonic_micros: u64,
        parts: &[&[u8]],
    ) -> Result<u64, TraceError> {
        let payload_bytes = parts.iter().try_fold(0_usize, |total, part| {
            total
                .checked_add(part.len())
                .ok_or(TraceError::LimitOverflow)
        })?;
        let TraceState::Recording { chunk, .. } = &self.state else {
            return Err(TraceError::NoActiveChunk);
        };
        if payload_bytes > self.limits.max_entry_bytes {
            return Err(TraceError::EntryTooLarge {
                actual: payload_bytes,
                limit: self.limits.max_entry_bytes,
            });
        }
        if chunk.entries.len() == self.limits.max_entries_per_chunk {
            return Err(TraceError::EntryCapacityExceeded {
                limit: self.limits.max_entries_per_chunk,
            });
        }
        let next_payload_bytes = chunk
            .payload_bytes
            .checked_add(payload_bytes)
            .ok_or(TraceError::LimitOverflow)?;
        if next_payload_bytes > self.limits.max_chunk_bytes {
            return Err(TraceError::ChunkBytesExceeded {
                actual: next_payload_bytes,
                limit: self.limits.max_chunk_bytes,
            });
        }
        let sequence = self.next_sequence;
        let following = sequence
            .checked_add(1)
            .ok_or(TraceError::SequenceExhausted)?;
        let lease = self.ledger.reserve(
            self.scope,
            ResourceClass::TraceBytes,
            u64::try_from(payload_bytes).map_err(|_| TraceError::LimitOverflow)?,
        )?;
        let mut payload = Vec::new();
        payload.try_reserve_exact(payload_bytes).map_err(|_| {
            TraceError::PayloadAllocationFailed {
                requested: payload_bytes,
            }
        })?;
        for part in parts {
            payload.extend_from_slice(part);
        }
        let payload = BudgetedBytes::from_reserved(lease, ResourceClass::TraceBytes, payload)?;
        let TraceState::Recording { chunk, .. } = &mut self.state else {
            unreachable!("recording state was validated before allocation")
        };
        chunk.entries.push(TraceEntry {
            sequence,
            monotonic_micros,
            kind,
            payload,
        });
        chunk.payload_bytes = next_payload_bytes;
        self.next_sequence = following;
        Ok(sequence)
    }
}

#[derive(Debug)]
pub enum TraceProductionError {
    Resources(ProductionResourceError),
    Trace(TraceError),
}

impl Display for TraceProductionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resources(error) => Display::fmt(error, formatter),
            Self::Trace(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for TraceProductionError {}

impl From<ProductionResourceError> for TraceProductionError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

impl From<TraceError> for TraceProductionError {
    fn from(error: TraceError) -> Self {
        Self::Trace(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceError {
    ZeroLimit(&'static str),
    EntryLimitExceedsChunk {
        entry: usize,
        chunk: usize,
    },
    LimitOverflow,
    AlreadyStarted,
    NotStarted,
    ChunkAlreadyStarted,
    DetachedChunkPending,
    NoActiveChunk,
    ActiveChunk,
    InvalidApiCallKind(TraceEntryKind),
    LabelTooLarge {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    LabelEncodingOverflow {
        field: &'static str,
    },
    EntryTooLarge {
        actual: usize,
        limit: usize,
    },
    EntryCapacityExceeded {
        limit: usize,
    },
    ChunkBytesExceeded {
        actual: usize,
        limit: usize,
    },
    GroupDepthExceeded {
        limit: usize,
    },
    NoOpenGroup,
    UnclosedGroups {
        count: usize,
    },
    EntryAllocationFailed {
        requested: usize,
    },
    PayloadAllocationFailed {
        requested: usize,
    },
    ChunkIdExhausted,
    SequenceExhausted,
    Resources(ResourceError),
}

impl Display for TraceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit(name) => write!(formatter, "trace {name} limit must be non-zero"),
            Self::EntryLimitExceedsChunk { entry, chunk } => write!(
                formatter,
                "trace entry byte limit {entry} exceeds chunk byte limit {chunk}"
            ),
            Self::LimitOverflow => formatter.write_str("trace resource limit overflowed"),
            Self::AlreadyStarted => formatter.write_str("tracing is already started"),
            Self::NotStarted => formatter.write_str("tracing is not started"),
            Self::ChunkAlreadyStarted => formatter.write_str("a trace chunk is already active"),
            Self::DetachedChunkPending => {
                formatter.write_str("the detached trace chunk must be dropped first")
            }
            Self::NoActiveChunk => formatter.write_str("no trace chunk is active"),
            Self::ActiveChunk => {
                formatter.write_str("the active trace chunk must be stopped first")
            }
            Self::InvalidApiCallKind(kind) => {
                write!(
                    formatter,
                    "trace entry kind {kind:?} is not an API-call boundary"
                )
            }
            Self::LabelTooLarge {
                field,
                actual,
                limit,
            } => write!(
                formatter,
                "trace {field} has {actual} bytes, exceeding limit {limit}"
            ),
            Self::LabelEncodingOverflow { field } => {
                write!(formatter, "trace {field} cannot be represented on the wire")
            }
            Self::EntryTooLarge { actual, limit } => write!(
                formatter,
                "trace entry has {actual} bytes, exceeding limit {limit}"
            ),
            Self::EntryCapacityExceeded { limit } => {
                write!(formatter, "trace chunk reached its {limit}-entry limit")
            }
            Self::ChunkBytesExceeded { actual, limit } => write!(
                formatter,
                "trace chunk would retain {actual} bytes, exceeding limit {limit}"
            ),
            Self::GroupDepthExceeded { limit } => {
                write!(formatter, "trace group depth reached its limit {limit}")
            }
            Self::NoOpenGroup => formatter.write_str("no trace group is open"),
            Self::UnclosedGroups { count } => {
                write!(formatter, "trace chunk has {count} unclosed groups")
            }
            Self::EntryAllocationFailed { requested } => write!(
                formatter,
                "trace recorder could not reserve {requested} entry slots"
            ),
            Self::PayloadAllocationFailed { requested } => write!(
                formatter,
                "trace recorder could not allocate a {requested}-byte payload"
            ),
            Self::ChunkIdExhausted => formatter.write_str("trace chunk ID space is exhausted"),
            Self::SequenceExhausted => formatter.write_str("trace sequence space is exhausted"),
            Self::Resources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for TraceError {}

impl From<ResourceError> for TraceError {
    fn from(error: ResourceError) -> Self {
        Self::Resources(error)
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{ProductionResourceLedger, ResourceLimits};

    use super::*;

    fn limits() -> TraceLimits {
        TraceLimits::new(16, 4, 32, 48, 2)
    }

    fn ledger(trace_bytes: u64, leases: usize) -> ResourceLedger {
        ResourceLedger::new(ResourceLimits::deny_all(1, leases).with_class(
            ResourceClass::TraceBytes,
            trace_bytes,
            trace_bytes,
        ))
    }

    #[test]
    fn session_chunk_groups_and_entries_move_into_one_owned_chunk() {
        // Arrange
        let ledger = ledger(64, 16);
        let mut recorder = TraceRecorder::new(limits(), ledger.clone(), ResourceScope::Connection)
            .expect("trace recorder should construct");
        recorder
            .start(Some("session"))
            .expect("session should start");
        let id = recorder
            .start_chunk(Some("chunk"), Some("title"))
            .expect("chunk should start");

        // Act
        let first = recorder
            .begin_group(10, "group")
            .expect("group should start");
        let second = recorder
            .append(TraceEntryKind::ApiCallBefore, 11, b"call")
            .expect("entry should append");
        recorder.end_group(12).expect("group should end");
        let chunk = recorder.stop_chunk().expect("balanced chunk should stop");
        recorder.stop().expect("session should stop");

        // Assert
        assert_eq!(id.get(), 1);
        assert_eq!((first, second), (1, 2));
        assert_eq!(chunk.name(), Some("chunk"));
        assert_eq!(chunk.title(), Some("title"));
        assert_eq!(chunk.entries().len(), 3);
        assert_eq!(chunk.payload_bytes(), 15);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            25,
            "detached chunk retains name, title, group, and call payloads"
        );
        drop(chunk);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[test]
    fn entry_count_and_bytes_are_independent_fail_closed_limits() {
        // Arrange
        let ledger = ledger(64, 16);
        let mut recorder = TraceRecorder::new(
            TraceLimits::new(16, 4, 8, 12, 2),
            ledger.clone(),
            ResourceScope::Connection,
        )
        .unwrap();
        recorder.start(None).unwrap();
        recorder.start_chunk(None, None).unwrap();
        recorder
            .append(TraceEntryKind::Log, 1, b"12345678")
            .unwrap();
        recorder.append(TraceEntryKind::Log, 2, b"1234").unwrap();

        // Act
        let bytes = recorder
            .append(TraceEntryKind::Log, 3, b"x")
            .expect_err("thirteen retained bytes should fail");
        recorder.append(TraceEntryKind::Log, 3, &[]).unwrap();
        recorder.append(TraceEntryKind::Log, 4, &[]).unwrap();
        let count = recorder
            .append(TraceEntryKind::Log, 5, &[])
            .expect_err("a fifth zero-byte entry should still fail");

        // Assert
        assert_eq!(
            bytes,
            TraceError::ChunkBytesExceeded {
                actual: 13,
                limit: 12
            }
        );
        assert_eq!(count, TraceError::EntryCapacityExceeded { limit: 4 });
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            12
        );
        recorder.discard_chunk().unwrap();
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[test]
    fn unbalanced_stop_rejects_while_abort_releases_every_lease() {
        // Arrange
        let ledger = ledger(64, 16);
        let mut recorder =
            TraceRecorder::new(limits(), ledger.clone(), ResourceScope::Connection).unwrap();
        recorder.start(Some("session")).unwrap();
        recorder.start_chunk(Some("chunk"), None).unwrap();
        recorder.begin_group(1, "group").unwrap();

        // Act
        let error = recorder
            .stop_chunk()
            .expect_err("open group should reject stop");
        let aborted = recorder.abort();

        // Assert
        assert_eq!(error, TraceError::UnclosedGroups { count: 1 });
        assert!(aborted);
        assert!(!recorder.is_started());
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[test]
    fn group_location_is_encoded_after_one_exact_reservation() {
        // Arrange
        let ledger = ledger(96, 16);
        let mut recorder =
            TraceRecorder::new(limits(), ledger.clone(), ResourceScope::Connection).unwrap();
        recorder.start(None).unwrap();
        recorder.start_chunk(None, None).unwrap();

        // Act
        recorder
            .begin_group_at(
                9,
                "group",
                Some(TraceSourceLocation::new("flow.rs", Some(7), Some(3))),
            )
            .unwrap();
        recorder.end_group(10).unwrap();
        let chunk = recorder.stop_chunk().unwrap();

        // Assert
        assert_eq!(chunk.payload_bytes(), 32);
        assert_eq!(
            chunk.entries()[0].payload(),
            &[
                1, 0, 0, 0, 5, b'g', b'r', b'o', b'u', b'p', 1, 0, 0, 0, 7, b'f', b'l', b'o', b'w',
                b'.', b'r', b's', 1, 0, 0, 0, 7, 1, 0, 0, 0, 3,
            ]
        );
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            32
        );
    }

    #[test]
    fn api_call_boundary_encodes_one_correlated_payload() {
        // Arrange
        let ledger = ledger(64, 16);
        let mut recorder =
            TraceRecorder::new(limits(), ledger.clone(), ResourceScope::Connection).unwrap();
        recorder.start(None).unwrap();
        recorder.start_chunk(None, None).unwrap();

        // Act
        recorder
            .append_api_call(
                TraceEntryKind::ApiCallBefore,
                17,
                CallId::new(42),
                "navigation.goto",
            )
            .unwrap();
        let chunk = recorder.stop_chunk().unwrap();

        // Assert
        assert_eq!(chunk.payload_bytes(), 28);
        assert_eq!(
            chunk.entries()[0].payload(),
            &[
                1, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 15, b'n', b'a', b'v', b'i', b'g', b'a', b't',
                b'i', b'o', b'n', b'.', b'g', b'o', b't', b'o',
            ]
        );
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            28
        );
    }

    #[test]
    fn detached_chunk_blocks_another_declared_peak_until_it_is_dropped() {
        // Arrange
        let ledger = ledger(64, 16);
        let mut recorder = TraceRecorder::new(limits(), ledger, ResourceScope::Connection)
            .expect("trace recorder should construct");
        recorder.start(None).expect("session should start");
        recorder
            .start_chunk(None, None)
            .expect("chunk should start");
        let first = recorder.stop_chunk().expect("chunk should detach");

        // Act
        let same_session = recorder
            .start_chunk(None, None)
            .expect_err("a detached chunk should reserve the declared peak");
        drop(first);
        let second_id = recorder
            .start_chunk(None, None)
            .expect("dropping the chunk should release the declared peak");
        let second = recorder.stop_chunk().expect("second chunk should detach");
        recorder.stop().expect("session should stop");
        let next_session = recorder
            .start(None)
            .expect_err("a detached chunk should also block a new session");
        drop(second);
        recorder.start(None).expect("new session should now start");
        let reset_id = recorder
            .start_chunk(None, None)
            .expect("new session chunk should start");

        // Assert
        assert_eq!(same_session, TraceError::DetachedChunkPending);
        assert_eq!(second_id.get(), 2);
        assert_eq!(next_session, TraceError::DetachedChunkPending);
        assert_eq!(reset_id.get(), 1);
    }

    #[test]
    fn production_profile_claims_the_complete_retained_peak() {
        // Arrange
        let profile =
            TraceRecorderProductionProfile::new(limits(), ResourceScope::extension(91, 1))
                .expect("profile should validate");
        let requirements = profile.requirements();
        let available = ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::TraceBytes,
                requirements.class(ResourceClass::TraceBytes).global,
                requirements.class(ResourceClass::TraceBytes).per_scope,
            ),
            requirements,
        )
        .expect("exact production resources should validate");

        // Act
        let recorder = profile
            .create(&available)
            .expect("declared trace profile should construct");

        // Assert
        assert!(!recorder.is_started());
        assert_eq!(requirements.class(ResourceClass::TraceBytes).global, 96);
        assert_eq!(requirements.leases, 7);
    }
}
