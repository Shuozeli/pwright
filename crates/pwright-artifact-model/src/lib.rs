//! Core-only bounded Artifact byte ownership and cancellation-safe Stream reads.
//!
//! This crate defines no command, protocol, transport, filesystem, executor,
//! task, browser backend, trace recorder, or browser downloader. Optional
//! capabilities may expose this state through Playwright-shaped Artifact and
//! Stream objects.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::{Arc, Mutex, MutexGuard};

use pwright_core::{
    BudgetedBytes, BudgetedUtf8, ObjectId, ObjectKindTag, ProductionResourceError,
    ProductionResourceLedger, ResourceClass, ResourceError, ResourceLedger, ResourceRequirements,
    ResourceScope,
};

pub type ArtifactOpenStreamOptions = ();
pub type ArtifactDeleteOptions = ();
pub type StreamCloseOptions = ();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamInitializer {
    pub object_id: ObjectId,
    pub parent_id: ObjectId,
    pub object_kind: ObjectKindTag,
}

impl StreamInitializer {
    pub const fn new(object_id: ObjectId, parent_id: ObjectId, object_kind: ObjectKindTag) -> Self {
        Self {
            object_id,
            parent_id,
            object_kind,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamReadOptions {
    max_bytes: u32,
}

impl StreamReadOptions {
    pub const fn new(max_bytes: u32) -> Self {
        Self { max_bytes }
    }

    pub const fn max_bytes(self) -> u32 {
        self.max_bytes
    }
}

pub type ArtifactFailureOptions = ();

#[derive(Debug, Default)]
pub struct ArtifactFailureResult {
    error: Option<BudgetedUtf8>,
}

impl ArtifactFailureResult {
    pub const fn new(error: Option<BudgetedUtf8>) -> Self {
        Self { error }
    }

    pub fn copy_from_str(
        ledger: &ResourceLedger,
        scope: ResourceScope,
        error: Option<&str>,
    ) -> Result<Self, ResourceError> {
        Ok(Self {
            error: error
                .map(|error| {
                    BudgetedUtf8::copy_from_str(ledger, scope, ResourceClass::ArtifactBytes, error)
                })
                .transpose()?,
        })
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_ref().map(BudgetedUtf8::as_str)
    }

    pub fn into_error(self) -> Option<BudgetedUtf8> {
        self.error
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactStreamLimits {
    pub max_artifact_bytes: usize,
    pub max_read_bytes: usize,
}

impl ArtifactStreamLimits {
    pub const fn new(max_artifact_bytes: usize, max_read_bytes: usize) -> Self {
        Self {
            max_artifact_bytes,
            max_read_bytes,
        }
    }

    pub fn validate(self) -> Result<(), ArtifactStreamError> {
        if self.max_read_bytes == 0 {
            return Err(ArtifactStreamError::ZeroLimit("read bytes"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactStreamProductionProfile {
    limits: ArtifactStreamLimits,
    source_scope: ResourceScope,
    required_bytes: u64,
    required_per_scope_bytes: u64,
}

impl ArtifactStreamProductionProfile {
    pub fn one_stream(
        limits: ArtifactStreamLimits,
        source_scope: ResourceScope,
    ) -> Result<Self, ArtifactStreamError> {
        limits.validate()?;
        let largest_read = limits.max_read_bytes.min(limits.max_artifact_bytes);
        let peak = limits
            .max_artifact_bytes
            .checked_add(largest_read)
            .ok_or(ArtifactStreamError::LimitOverflow)?;
        let required_bytes = u64::try_from(peak).map_err(|_| ArtifactStreamError::LimitOverflow)?;
        let required_per_scope_bytes = u64::try_from(limits.max_artifact_bytes)
            .map_err(|_| ArtifactStreamError::LimitOverflow)?;
        Ok(Self {
            limits,
            source_scope,
            required_bytes,
            required_per_scope_bytes,
        })
    }

    pub const fn limits(self) -> ArtifactStreamLimits {
        self.limits
    }

    pub const fn source_scope(self) -> ResourceScope {
        self.source_scope
    }

    pub const fn requirements(self) -> ResourceRequirements {
        ResourceRequirements::none()
            .with_class(
                ResourceClass::ArtifactBytes,
                self.required_bytes,
                self.required_per_scope_bytes,
            )
            .with_metadata(2, 2)
    }

    pub fn create(
        self,
        source: &[u8],
        resources: &ProductionResourceLedger,
    ) -> Result<ArtifactStream, ArtifactStreamError> {
        self.create_artifact(source, resources)?.open_stream()
    }

    pub fn create_artifact(
        self,
        source: &[u8],
        resources: &ProductionResourceLedger,
    ) -> Result<Artifact, ArtifactStreamError> {
        let ledger = resources.claim(self.requirements())?;
        Artifact::new(self.limits, ledger, self.source_scope, source)
    }
}

#[derive(Debug)]
pub struct ArtifactReadResult {
    bytes: BudgetedBytes,
    eof: bool,
}

impl ArtifactReadResult {
    pub const fn new(bytes: BudgetedBytes, eof: bool) -> Self {
        Self { bytes, eof }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_bytes()
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub const fn eof(&self) -> bool {
        self.eof
    }

    pub const fn budgeted_bytes(&self) -> &BudgetedBytes {
        &self.bytes
    }
}

struct ArtifactState {
    source: Option<Arc<BudgetedBytes>>,
}

struct ArtifactInner {
    limits: ArtifactStreamLimits,
    ledger: ResourceLedger,
    state: Mutex<ArtifactState>,
}

/// One immutable, independently budgeted Artifact source.
///
/// Opening a Stream shares the immutable source allocation and creates only a
/// new cursor. Deleting the Artifact releases its source owner immediately;
/// existing Streams retain the same allocation until each Stream closes.
#[derive(Clone)]
pub struct Artifact {
    inner: Arc<ArtifactInner>,
}

impl Artifact {
    pub fn new(
        limits: ArtifactStreamLimits,
        ledger: ResourceLedger,
        source_scope: ResourceScope,
        source: &[u8],
    ) -> Result<Self, ArtifactStreamError> {
        limits.validate()?;
        if source.len() > limits.max_artifact_bytes {
            return Err(ArtifactStreamError::ArtifactTooLarge {
                actual: source.len(),
                limit: limits.max_artifact_bytes,
            });
        }
        let source = Arc::new(BudgetedBytes::copy_from_slice(
            &ledger,
            source_scope,
            ResourceClass::ArtifactBytes,
            source,
        )?);
        Ok(Self {
            inner: Arc::new(ArtifactInner {
                limits,
                ledger,
                state: Mutex::new(ArtifactState {
                    source: Some(source),
                }),
            }),
        })
    }

    /// Adopts an already budgeted Artifact source without copying its bytes.
    /// The source must use the Artifact byte class; its existing scope and
    /// lease remain authoritative for the full Artifact/Stream lifetime.
    pub fn from_budgeted_source(
        limits: ArtifactStreamLimits,
        ledger: ResourceLedger,
        source: BudgetedBytes,
    ) -> Result<Self, ArtifactStreamError> {
        limits.validate()?;
        if source.class() != ResourceClass::ArtifactBytes {
            return Err(ArtifactStreamError::InvalidSourceClass {
                actual: source.class(),
            });
        }
        if source.len() > limits.max_artifact_bytes {
            return Err(ArtifactStreamError::ArtifactTooLarge {
                actual: source.len(),
                limit: limits.max_artifact_bytes,
            });
        }
        Ok(Self {
            inner: Arc::new(ArtifactInner {
                limits,
                ledger,
                state: Mutex::new(ArtifactState {
                    source: Some(Arc::new(source)),
                }),
            }),
        })
    }

    pub fn open_stream(&self) -> Result<ArtifactStream, ArtifactStreamError> {
        let state = self
            .inner
            .state
            .lock()
            .map_err(|_| ArtifactStreamError::StatePoisoned)?;
        let source = state.source.as_ref().ok_or(ArtifactStreamError::Deleted)?;
        Ok(ArtifactStream::from_shared_source(
            self.inner.limits,
            self.inner.ledger.clone(),
            Arc::clone(source),
        ))
    }

    pub fn delete(&self) -> Result<bool, ArtifactStreamError> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| ArtifactStreamError::StatePoisoned)?;
        Ok(state.source.take().is_some())
    }

    pub fn metrics(&self) -> Result<ArtifactMetrics, ArtifactStreamError> {
        let state = self
            .inner
            .state
            .lock()
            .map_err(|_| ArtifactStreamError::StatePoisoned)?;
        Ok(ArtifactMetrics {
            source_bytes: state.source.as_ref().map_or(0, |source| source.len()),
            deleted: state.source.is_none(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactMetrics {
    pub source_bytes: usize,
    pub deleted: bool,
}

struct ArtifactStreamState {
    source: Option<Arc<BudgetedBytes>>,
    offset: usize,
    next_read_id: u64,
    prepared_read: Option<u64>,
    closed: bool,
}

struct ArtifactStreamInner {
    limits: ArtifactStreamLimits,
    ledger: ResourceLedger,
    state: Mutex<ArtifactStreamState>,
}

#[derive(Clone)]
pub struct ArtifactStream {
    inner: Arc<ArtifactStreamInner>,
}

impl ArtifactStream {
    pub fn new(
        limits: ArtifactStreamLimits,
        ledger: ResourceLedger,
        source_scope: ResourceScope,
        source: &[u8],
    ) -> Result<Self, ArtifactStreamError> {
        Artifact::new(limits, ledger, source_scope, source)?.open_stream()
    }

    fn from_shared_source(
        limits: ArtifactStreamLimits,
        ledger: ResourceLedger,
        source: Arc<BudgetedBytes>,
    ) -> Self {
        Self {
            inner: Arc::new(ArtifactStreamInner {
                limits,
                ledger,
                state: Mutex::new(ArtifactStreamState {
                    source: Some(source),
                    offset: 0,
                    next_read_id: 1,
                    prepared_read: None,
                    closed: false,
                }),
            }),
        }
    }

    pub fn limits(&self) -> ArtifactStreamLimits {
        self.inner.limits
    }

    pub fn prepare_read(
        &self,
        requested_bytes: usize,
        result_scope: ResourceScope,
    ) -> Result<PreparedArtifactRead, ArtifactStreamError> {
        if requested_bytes == 0 {
            return Err(ArtifactStreamError::ZeroReadSize);
        }
        if requested_bytes > self.inner.limits.max_read_bytes {
            return Err(ArtifactStreamError::ReadTooLarge {
                actual: requested_bytes,
                limit: self.inner.limits.max_read_bytes,
            });
        }
        let mut state = self.lock_state()?;
        if state.closed {
            return Err(ArtifactStreamError::Closed);
        }
        if state.prepared_read.is_some() {
            return Err(ArtifactStreamError::ReadAlreadyPrepared);
        }
        let source = state.source.as_ref().ok_or(ArtifactStreamError::Closed)?;
        let source_len = source.len();
        let following = state
            .offset
            .checked_add(requested_bytes)
            .ok_or(ArtifactStreamError::LimitOverflow)?
            .min(source_len);
        let bytes = BudgetedBytes::copy_from_slice(
            &self.inner.ledger,
            result_scope,
            ResourceClass::ArtifactBytes,
            &source.as_bytes()[state.offset..following],
        )?;
        let read_id = state.next_read_id;
        state.next_read_id = read_id
            .checked_add(1)
            .ok_or(ArtifactStreamError::ReadIdExhausted)?;
        state.prepared_read = Some(read_id);
        let result = ArtifactReadResult {
            bytes,
            eof: following == source_len,
        };
        Ok(PreparedArtifactRead {
            inner: Arc::clone(&self.inner),
            read_id,
            following,
            result: Some(result),
            active: true,
        })
    }

    pub fn close(&self) -> Result<bool, ArtifactStreamError> {
        let mut state = self.lock_state()?;
        if state.closed {
            return Ok(false);
        }
        if state.prepared_read.is_some() {
            return Err(ArtifactStreamError::ReadAlreadyPrepared);
        }
        state.closed = true;
        state.source = None;
        state.offset = 0;
        Ok(true)
    }

    /// Invalidates an in-flight prepared read and releases this Stream's
    /// shared source owner. Parent/connection teardown uses this stronger
    /// operation; ordinary explicit close keeps its prepare/commit boundary.
    pub fn abort(&self) -> Result<bool, ArtifactStreamError> {
        let mut state = self.lock_state()?;
        if state.closed {
            return Ok(false);
        }
        state.closed = true;
        state.prepared_read = None;
        state.source = None;
        state.offset = 0;
        Ok(true)
    }

    pub fn metrics(&self) -> Result<ArtifactStreamMetrics, ArtifactStreamError> {
        let state = self.lock_state()?;
        Ok(ArtifactStreamMetrics {
            source_bytes: state.source.as_ref().map_or(0, |source| source.len()),
            offset: state.offset,
            read_prepared: state.prepared_read.is_some(),
            closed: state.closed,
        })
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, ArtifactStreamState>, ArtifactStreamError> {
        self.inner
            .state
            .lock()
            .map_err(|_| ArtifactStreamError::StatePoisoned)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactStreamMetrics {
    pub source_bytes: usize,
    pub offset: usize,
    pub read_prepared: bool,
    pub closed: bool,
}

pub struct PreparedArtifactRead {
    inner: Arc<ArtifactStreamInner>,
    read_id: u64,
    following: usize,
    result: Option<ArtifactReadResult>,
    active: bool,
}

impl PreparedArtifactRead {
    pub fn result(&self) -> &ArtifactReadResult {
        self.result
            .as_ref()
            .expect("active prepared read always owns its result")
    }

    pub fn commit(mut self) -> Result<ArtifactReadResult, ArtifactStreamError> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| ArtifactStreamError::StatePoisoned)?;
        if state.prepared_read != Some(self.read_id) {
            return Err(ArtifactStreamError::PreparedReadInvalidated);
        }
        state.offset = self.following;
        state.prepared_read = None;
        self.active = false;
        drop(state);
        self.result
            .take()
            .ok_or(ArtifactStreamError::PreparedReadInvalidated)
    }
}

impl Drop for PreparedArtifactRead {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        if let Ok(mut state) = self.inner.state.lock()
            && state.prepared_read == Some(self.read_id)
        {
            state.prepared_read = None;
        }
    }
}

#[derive(Debug)]
pub enum ArtifactStreamError {
    ZeroLimit(&'static str),
    ArtifactTooLarge { actual: usize, limit: usize },
    ZeroReadSize,
    ReadTooLarge { actual: usize, limit: usize },
    ReadAlreadyPrepared,
    ReadIdExhausted,
    PreparedReadInvalidated,
    Deleted,
    Closed,
    LimitOverflow,
    StatePoisoned,
    InvalidSourceClass { actual: ResourceClass },
    Resource(ResourceError),
    Production(ProductionResourceError),
}

impl Display for ArtifactStreamError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit(name) => {
                write!(formatter, "artifact stream {name} limit must be non-zero")
            }
            Self::ArtifactTooLarge { actual, limit } => write!(
                formatter,
                "artifact retains {actual} bytes, exceeding limit {limit}"
            ),
            Self::ZeroReadSize => formatter.write_str("artifact stream read size must be non-zero"),
            Self::ReadTooLarge { actual, limit } => write!(
                formatter,
                "artifact stream read requests {actual} bytes, exceeding limit {limit}"
            ),
            Self::ReadAlreadyPrepared => {
                formatter.write_str("artifact stream already has a prepared read")
            }
            Self::ReadIdExhausted => formatter.write_str("artifact stream read ID exhausted"),
            Self::PreparedReadInvalidated => {
                formatter.write_str("prepared artifact read is no longer active")
            }
            Self::Deleted => formatter.write_str("artifact is deleted"),
            Self::Closed => formatter.write_str("artifact stream is closed"),
            Self::LimitOverflow => {
                formatter.write_str("artifact stream limit arithmetic overflowed")
            }
            Self::StatePoisoned => formatter.write_str("artifact stream state is poisoned"),
            Self::InvalidSourceClass { actual } => write!(
                formatter,
                "artifact source uses {actual:?} instead of ArtifactBytes"
            ),
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::Production(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ArtifactStreamError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resource(error) => Some(error),
            Self::Production(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ResourceError> for ArtifactStreamError {
    fn from(error: ResourceError) -> Self {
        Self::Resource(error)
    }
}

impl From<ProductionResourceError> for ArtifactStreamError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Production(error)
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{
        BudgetedBytes, CallId, ObjectId, ProductionResourceLedger, ResourceClass, ResourceLimits,
        ResourceScope,
    };

    use super::{
        Artifact, ArtifactStream, ArtifactStreamError, ArtifactStreamLimits,
        ArtifactStreamProductionProfile,
    };

    fn ledger(bytes: u64, leases: usize) -> pwright_core::ResourceLedger {
        pwright_core::ResourceLedger::new(ResourceLimits::deny_all(4, leases).with_class(
            ResourceClass::ArtifactBytes,
            bytes,
            bytes,
        ))
    }

    fn current_bytes(ledger: &pwright_core::ResourceLedger) -> u64 {
        ledger
            .metrics()
            .expect("metrics should be available")
            .class(ResourceClass::ArtifactBytes)
            .current
    }

    #[test]
    fn committed_reads_advance_in_bounded_chunks_and_release_result_copies() {
        // Arrange
        let limits = ArtifactStreamLimits::new(8, 3);
        let profile = ArtifactStreamProductionProfile::one_stream(
            limits,
            ResourceScope::Object(ObjectId::new(1)),
        )
        .expect("profile should validate");
        let requirements = profile.requirements();
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::ArtifactBytes,
                requirements.class(ResourceClass::ArtifactBytes).global,
                requirements.class(ResourceClass::ArtifactBytes).per_scope,
            ),
            requirements,
        )
        .expect("production resources should fit");
        let stream = profile
            .create(b"abcdef", &resources)
            .expect("source should fit");

        // Act
        let first = stream
            .prepare_read(3, ResourceScope::Call(CallId::new(7)))
            .expect("first read should prepare")
            .commit()
            .expect("first read should commit");
        let during_first = resources.metrics().expect("metrics should be available");
        let first_bytes = first.as_bytes().to_vec();
        let first_eof = first.eof();
        drop(first);
        let after_first = resources.metrics().expect("metrics should be available");
        let second = stream
            .prepare_read(3, ResourceScope::Call(CallId::new(8)))
            .expect("second read should prepare")
            .commit()
            .expect("second read should commit");

        // Assert
        assert_eq!(first_bytes, b"abc");
        assert!(!first_eof);
        assert_eq!(during_first.class(ResourceClass::ArtifactBytes).current, 9);
        assert_eq!(after_first.class(ResourceClass::ArtifactBytes).current, 6);
        assert_eq!(second.as_bytes(), b"def");
        assert!(second.eof());
        assert_eq!(stream.metrics().expect("metrics should work").offset, 6);
        drop(second);
        stream.close().expect("stream should close");
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should be available")
                .class(ResourceClass::ArtifactBytes)
                .current,
            0
        );
    }

    #[test]
    fn dropped_prepared_read_rolls_back_without_advancing_the_cursor() {
        // Arrange
        let ledger = ledger(16, 4);
        let stream = ArtifactStream::new(
            ArtifactStreamLimits::new(8, 2),
            ledger.clone(),
            ResourceScope::Object(ObjectId::new(1)),
            b"abcd",
        )
        .expect("stream should build");
        let prepared = stream
            .prepare_read(2, ResourceScope::Call(CallId::new(1)))
            .expect("read should prepare");
        assert_eq!(prepared.result().as_bytes(), b"ab");

        // Act
        drop(prepared);
        let retried = stream
            .prepare_read(2, ResourceScope::Call(CallId::new(2)))
            .expect("same read should prepare again")
            .commit()
            .expect("retry should commit");

        // Assert
        assert_eq!(retried.as_bytes(), b"ab");
        assert_eq!(stream.metrics().expect("metrics should work").offset, 2);
    }

    #[test]
    fn only_one_read_can_be_prepared_per_stream() {
        // Arrange
        let stream = ArtifactStream::new(
            ArtifactStreamLimits::new(8, 2),
            ledger(16, 4),
            ResourceScope::Object(ObjectId::new(1)),
            b"abcd",
        )
        .expect("stream should build");
        let first = stream
            .prepare_read(2, ResourceScope::Call(CallId::new(1)))
            .expect("first read should prepare");

        // Act
        let second = stream.prepare_read(2, ResourceScope::Call(CallId::new(2)));

        // Assert
        assert!(matches!(
            second,
            Err(ArtifactStreamError::ReadAlreadyPrepared)
        ));
        drop(first);
        assert!(!stream.metrics().expect("metrics should work").read_prepared);
    }

    #[test]
    fn read_limit_and_resource_overlap_fail_before_cursor_mutation() {
        // Arrange
        let ledger = ledger(4, 2);
        let stream = ArtifactStream::new(
            ArtifactStreamLimits::new(4, 2),
            ledger.clone(),
            ResourceScope::Object(ObjectId::new(1)),
            b"abcd",
        )
        .expect("source should exactly fit");

        // Act
        let oversized = stream.prepare_read(3, ResourceScope::Call(CallId::new(1)));
        let no_overlap_budget = stream.prepare_read(2, ResourceScope::Call(CallId::new(2)));

        // Assert
        assert!(matches!(
            oversized,
            Err(ArtifactStreamError::ReadTooLarge {
                actual: 3,
                limit: 2
            })
        ));
        assert!(matches!(
            no_overlap_budget,
            Err(ArtifactStreamError::Resource(_))
        ));
        assert_eq!(stream.metrics().expect("metrics should work").offset, 0);
        assert_eq!(current_bytes(&ledger), 4);
    }

    #[test]
    fn close_waits_for_prepared_read_then_releases_the_source_once() {
        // Arrange
        let ledger = ledger(16, 4);
        let stream = ArtifactStream::new(
            ArtifactStreamLimits::new(8, 2),
            ledger.clone(),
            ResourceScope::Object(ObjectId::new(1)),
            b"abcd",
        )
        .expect("stream should build");
        let prepared = stream
            .prepare_read(2, ResourceScope::Call(CallId::new(1)))
            .expect("read should prepare");

        // Act
        let while_prepared = stream.close();
        drop(prepared);
        let first_close = stream.close();
        let second_close = stream.close();
        let after_close = stream.prepare_read(1, ResourceScope::Call(CallId::new(2)));

        // Assert
        assert!(matches!(
            while_prepared,
            Err(ArtifactStreamError::ReadAlreadyPrepared)
        ));
        assert!(matches!(first_close, Ok(true)));
        assert!(matches!(second_close, Ok(false)));
        assert!(matches!(after_close, Err(ArtifactStreamError::Closed)));
        assert_eq!(current_bytes(&ledger), 0);
    }

    #[test]
    fn empty_artifact_returns_explicit_eof_without_a_byte_lease() {
        // Arrange
        let ledger = ledger(1, 2);
        let stream = ArtifactStream::new(
            ArtifactStreamLimits::new(0, 8),
            ledger.clone(),
            ResourceScope::Object(ObjectId::new(1)),
            b"",
        )
        .expect("empty stream should build");

        // Act
        let result = stream
            .prepare_read(8, ResourceScope::Call(CallId::new(1)))
            .expect("EOF read should prepare")
            .commit()
            .expect("EOF read should commit");

        // Assert
        assert!(result.is_empty());
        assert!(result.eof());
        assert_eq!(current_bytes(&ledger), 0);
    }

    #[test]
    fn multiple_streams_share_one_source_lease_and_keep_independent_cursors() {
        // Arrange
        let ledger = ledger(16, 4);
        let artifact = Artifact::new(
            ArtifactStreamLimits::new(8, 3),
            ledger.clone(),
            ResourceScope::Object(ObjectId::new(1)),
            b"abcdef",
        )
        .expect("artifact should build");
        let first = artifact.open_stream().expect("first Stream should open");
        let second = artifact.open_stream().expect("second Stream should open");

        // Act
        let first_result = first
            .prepare_read(3, ResourceScope::Call(CallId::new(1)))
            .expect("first read should prepare")
            .commit()
            .expect("first read should commit");
        drop(first_result);
        let deleted = artifact.delete();
        let reopening = artifact.open_stream();
        let second_result = second
            .prepare_read(2, ResourceScope::Call(CallId::new(2)))
            .expect("second cursor should still read from the start")
            .commit()
            .expect("second read should commit");

        // Assert
        assert!(matches!(deleted, Ok(true)));
        assert!(matches!(reopening, Err(ArtifactStreamError::Deleted)));
        assert_eq!(second_result.as_bytes(), b"ab");
        assert_eq!(first.metrics().unwrap().offset, 3);
        assert_eq!(second.metrics().unwrap().offset, 2);
        assert_eq!(current_bytes(&ledger), 8);
        drop(second_result);
        assert_eq!(current_bytes(&ledger), 6);
        first.close().unwrap();
        assert_eq!(current_bytes(&ledger), 6);
        second.close().unwrap();
        assert_eq!(current_bytes(&ledger), 0);
    }

    #[test]
    fn prebudgeted_source_is_adopted_without_a_second_byte_lease() {
        // Arrange
        let ledger = ledger(8, 2);
        let source = BudgetedBytes::copy_from_slice(
            &ledger,
            ResourceScope::Object(ObjectId::new(1)),
            ResourceClass::ArtifactBytes,
            b"abcdef",
        )
        .unwrap();
        assert_eq!(current_bytes(&ledger), 6);

        // Act
        let artifact =
            Artifact::from_budgeted_source(ArtifactStreamLimits::new(8, 2), ledger.clone(), source)
                .unwrap();

        // Assert
        assert_eq!(current_bytes(&ledger), 6);
        assert_eq!(artifact.metrics().unwrap().source_bytes, 6);
        artifact.delete().unwrap();
        assert_eq!(current_bytes(&ledger), 0);
    }
}
