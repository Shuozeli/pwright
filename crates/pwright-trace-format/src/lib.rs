//! Shared PWRTRACE v1 encoding without selecting a sink, runtime, or executor.
//!
//! File sinks may stream directly to a writer. In-memory Artifact producers use
//! [`encode_trace_artifact`] so the final owned allocation is reserved before
//! allocation and can be transferred without a second byte copy.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};

use pwright_core::{
    BudgetedBytes, ObjectId, ResourceClass, ResourceError, ResourceLedger, ResourceScope,
};
use pwright_trace_model::{TraceChunk, TraceEntryKind};

pub const PWRTRACE_MAGIC: &[u8; 8] = b"PWRTRACE";
pub const PWRTRACE_FORMAT_VERSION: u16 = 1;
const FIXED_HEADER_BYTES: usize = 8 + 2 + 8 + 8 + 1 + 4 + 8;
const FIXED_ENTRY_BYTES: usize = 8 + 8 + 1 + 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceArtifactEncodingLimits {
    pub max_encoded_bytes: usize,
}

impl TraceArtifactEncodingLimits {
    pub const fn new(max_encoded_bytes: usize) -> Self {
        Self { max_encoded_bytes }
    }

    pub fn validate(self) -> Result<(), TraceFormatError> {
        if self.max_encoded_bytes == 0 {
            return Err(TraceFormatError::ZeroEncodedByteLimit);
        }
        Ok(())
    }
}

pub fn encoded_trace_chunk_bytes(chunk: &TraceChunk) -> Result<usize, TraceFormatError> {
    let labels = [chunk.name(), chunk.title()]
        .into_iter()
        .flatten()
        .try_fold(0_usize, |total, label| {
            total
                .checked_add(4)
                .and_then(|value| value.checked_add(label.len()))
        })
        .ok_or(TraceFormatError::ArithmeticOverflow)?;
    let entries = chunk.entries().iter().try_fold(0_usize, |total, entry| {
        total
            .checked_add(FIXED_ENTRY_BYTES)
            .and_then(|value| value.checked_add(entry.payload().len()))
            .ok_or(TraceFormatError::ArithmeticOverflow)
    })?;
    FIXED_HEADER_BYTES
        .checked_add(labels)
        .and_then(|value| value.checked_add(entries))
        .ok_or(TraceFormatError::ArithmeticOverflow)
}

pub fn write_trace_chunk(
    writer: &mut impl Write,
    context_id: ObjectId,
    chunk: &TraceChunk,
) -> io::Result<()> {
    writer.write_all(PWRTRACE_MAGIC)?;
    writer.write_all(&PWRTRACE_FORMAT_VERSION.to_be_bytes())?;
    writer.write_all(&context_id.get().to_be_bytes())?;
    writer.write_all(&chunk.id().get().to_be_bytes())?;
    let flags = u8::from(chunk.name().is_some()) | (u8::from(chunk.title().is_some()) << 1);
    writer.write_all(&[flags])?;
    if let Some(name) = chunk.name() {
        write_bytes(writer, name.as_bytes())?;
    }
    if let Some(title) = chunk.title() {
        write_bytes(writer, title.as_bytes())?;
    }
    let entries = u32::try_from(chunk.entries().len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "too many trace entries"))?;
    writer.write_all(&entries.to_be_bytes())?;
    writer.write_all(
        &u64::try_from(chunk.payload_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "trace payload too large"))?
            .to_be_bytes(),
    )?;
    for entry in chunk.entries() {
        writer.write_all(&entry.sequence().to_be_bytes())?;
        writer.write_all(&entry.monotonic_micros().to_be_bytes())?;
        writer.write_all(&[entry_kind_tag(entry.kind())])?;
        write_bytes(writer, entry.payload())?;
    }
    Ok(())
}

/// Encodes directly into the final Artifact-owned allocation.
///
/// Exact size and ledger admission are checked before `Vec` allocation. The
/// returned value owns that same vector and its `ArtifactBytes` lease, so a
/// registry can adopt it without a staging copy.
pub fn encode_trace_artifact(
    limits: TraceArtifactEncodingLimits,
    ledger: &ResourceLedger,
    artifact_id: ObjectId,
    context_id: ObjectId,
    chunk: &TraceChunk,
) -> Result<BudgetedBytes, TraceFormatError> {
    limits.validate()?;
    let encoded_bytes = encoded_trace_chunk_bytes(chunk)?;
    if encoded_bytes > limits.max_encoded_bytes {
        return Err(TraceFormatError::EncodedBytesExceeded {
            actual: encoded_bytes,
            limit: limits.max_encoded_bytes,
        });
    }
    let units = u64::try_from(encoded_bytes).map_err(|_| TraceFormatError::ArithmeticOverflow)?;
    let lease = ledger.reserve(
        ResourceScope::Object(artifact_id),
        ResourceClass::ArtifactBytes,
        units,
    )?;
    let mut encoded = Vec::new();
    encoded
        .try_reserve_exact(encoded_bytes)
        .map_err(|_| TraceFormatError::AllocationFailed {
            requested: encoded_bytes,
        })?;
    write_trace_chunk(&mut encoded, context_id, chunk).map_err(TraceFormatError::Write)?;
    if encoded.len() != encoded_bytes {
        return Err(TraceFormatError::EncodedLengthMismatch {
            expected: encoded_bytes,
            actual: encoded.len(),
        });
    }
    BudgetedBytes::from_reserved(lease, ResourceClass::ArtifactBytes, encoded)
        .map_err(TraceFormatError::Resource)
}

fn write_bytes(writer: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    let length = u32::try_from(bytes.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "trace field too large"))?;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(bytes)
}

const fn entry_kind_tag(kind: TraceEntryKind) -> u8 {
    match kind {
        TraceEntryKind::ApiCallBefore => 1,
        TraceEntryKind::ApiCallAfter => 2,
        TraceEntryKind::ApiCallError => 3,
        TraceEntryKind::Log => 4,
        TraceEntryKind::GroupStart => 5,
        TraceEntryKind::GroupEnd => 6,
    }
}

#[derive(Debug)]
pub enum TraceFormatError {
    ZeroEncodedByteLimit,
    ArithmeticOverflow,
    EncodedBytesExceeded { actual: usize, limit: usize },
    AllocationFailed { requested: usize },
    EncodedLengthMismatch { expected: usize, actual: usize },
    Resource(ResourceError),
    Write(io::Error),
}

impl Display for TraceFormatError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroEncodedByteLimit => {
                formatter.write_str("trace Artifact encoded-byte limit must be non-zero")
            }
            Self::ArithmeticOverflow => formatter.write_str("trace encoding size overflowed"),
            Self::EncodedBytesExceeded { actual, limit } => write!(
                formatter,
                "encoded trace requires {actual} bytes, exceeding limit {limit}"
            ),
            Self::AllocationFailed { requested } => write!(
                formatter,
                "could not allocate the bounded {requested}-byte trace Artifact"
            ),
            Self::EncodedLengthMismatch { expected, actual } => write!(
                formatter,
                "trace encoder produced {actual} bytes instead of planned {expected}"
            ),
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::Write(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for TraceFormatError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resource(error) => Some(error),
            Self::Write(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ResourceError> for TraceFormatError {
    fn from(error: ResourceError) -> Self {
        Self::Resource(error)
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{ObjectId, ResourceClass, ResourceLedger, ResourceLimits, ResourceScope};
    use pwright_trace_model::{TraceEntryKind, TraceLimits, TraceRecorder};

    use super::{
        PWRTRACE_MAGIC, TraceArtifactEncodingLimits, TraceFormatError, encode_trace_artifact,
        encoded_trace_chunk_bytes,
    };

    fn ledgers() -> (ResourceLedger, ResourceLedger) {
        (
            ResourceLedger::new(ResourceLimits::deny_all(1, 8).with_class(
                ResourceClass::TraceBytes,
                256,
                256,
            )),
            ResourceLedger::new(ResourceLimits::deny_all(1, 2).with_class(
                ResourceClass::ArtifactBytes,
                256,
                256,
            )),
        )
    }

    #[test]
    fn exact_admission_encodes_into_one_budgeted_artifact_allocation() {
        // Arrange
        let (trace_ledger, artifact_ledger) = ledgers();
        let mut recorder = TraceRecorder::new(
            TraceLimits::new(16, 2, 32, 128, 1),
            trace_ledger,
            ResourceScope::Connection,
        )
        .unwrap();
        recorder.start(None).unwrap();
        recorder.start_chunk(Some("one"), Some("title")).unwrap();
        recorder.append(TraceEntryKind::Log, 7, b"payload").unwrap();
        let chunk = recorder.stop_chunk().unwrap();
        let planned = encoded_trace_chunk_bytes(&chunk).unwrap();

        // Act
        let encoded = encode_trace_artifact(
            TraceArtifactEncodingLimits::new(planned),
            &artifact_ledger,
            ObjectId::new(9),
            ObjectId::new(2),
            &chunk,
        )
        .unwrap();

        // Assert
        assert_eq!(encoded.len(), planned);
        assert_eq!(&encoded.as_bytes()[..PWRTRACE_MAGIC.len()], PWRTRACE_MAGIC);
        assert_eq!(encoded.scope(), ResourceScope::Object(ObjectId::new(9)));
        assert_eq!(encoded.class(), ResourceClass::ArtifactBytes);
        assert_eq!(
            artifact_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            u64::try_from(planned).unwrap()
        );
        drop(encoded);
        assert_eq!(
            artifact_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            0
        );
    }

    #[test]
    fn encoded_limit_rejects_before_artifact_budget_or_allocation() {
        // Arrange
        let (trace_ledger, artifact_ledger) = ledgers();
        let mut recorder = TraceRecorder::new(
            TraceLimits::new(16, 1, 32, 128, 1),
            trace_ledger,
            ResourceScope::Connection,
        )
        .unwrap();
        recorder.start(None).unwrap();
        recorder.start_chunk(None, None).unwrap();
        recorder.append(TraceEntryKind::Log, 1, b"x").unwrap();
        let chunk = recorder.stop_chunk().unwrap();
        let actual = encoded_trace_chunk_bytes(&chunk).unwrap();

        // Act
        let result = encode_trace_artifact(
            TraceArtifactEncodingLimits::new(actual - 1),
            &artifact_ledger,
            ObjectId::new(9),
            ObjectId::new(2),
            &chunk,
        );

        // Assert
        assert!(matches!(
            result,
            Err(TraceFormatError::EncodedBytesExceeded { actual: found, limit })
                if found == actual && limit == actual - 1
        ));
        assert_eq!(
            artifact_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            0
        );
    }
}
