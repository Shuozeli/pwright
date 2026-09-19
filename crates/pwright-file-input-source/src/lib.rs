//! Optional bounded content materialization. No Node, browser or executor.
//!
//! The caller supplies opened readers (including `std::fs::File`), resolved
//! MIME and observed modification time. This is not path preparation or an
//! upload action. It performs synchronous IO and must not run on an event loop.
//! No work is spawned: an embedding executor must admit work before scheduling,
//! retain this operation until it exits, and separately account source handles.

use pwright_core::{
    ResourceClass, ResourceError, ResourceLease, ResourceLedger, ResourceRequest, ResourceScope,
};
use pwright_file_input_model::{FileInputError, FileInputLimits, PayloadMetadata, UploadPlan};
use std::{
    collections::TryReserveError,
    fmt,
    io::{self, Read},
};

pub enum Content<'a> {
    /// Actual slice length is authoritative; no caller-supplied size is trusted.
    Bytes(&'a [u8]),
    /// Observed remaining length is a strict snapshot bound, not permission to
    /// grow. Readers must be finite and at the desired start position; the
    /// embedding filesystem layer must reject pipes/devices and open failures.
    Reader {
        reader: &'a mut dyn Read,
        byte_length: u64,
    },
}

pub struct PayloadSource<'a> {
    pub name: &'a str,
    /// Already resolved by the embedding MIME service.
    pub mime_type: &'a str,
    /// None preserves browser-time behavior for memory payloads.
    pub last_modified_ms: Option<f64>,
    pub content: Content<'a>,
}
impl PayloadSource<'_> {
    fn metadata(&self) -> PayloadMetadata<'_> {
        PayloadMetadata {
            name: self.name,
            mime_type: self.mime_type,
            byte_length: match &self.content {
                Content::Bytes(bytes) => bytes.len() as u64,
                Content::Reader { byte_length, .. } => *byte_length,
            },
        }
    }
}

#[derive(Debug)]
pub enum SourceError {
    Policy(FileInputError),
    Budget(ResourceError),
    Allocation(TryReserveError),
    Io(io::Error),
    SourceChanged,
    ItemIndex,
    SizeOverflow,
}
impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(error) => error.fmt(f),
            Self::Budget(error) => error.fmt(f),
            Self::Allocation(error) => error.fmt(f),
            Self::Io(error) => error.fmt(f),
            Self::SourceChanged => {
                f.write_str("file-input source length changed during preparation")
            }
            Self::SizeOverflow => f.write_str("file-input storage size overflow"),
            Self::ItemIndex => f.write_str("file-input item index is out of range"),
        }
    }
}
impl std::error::Error for SourceError {}

/// Fields cannot be moved out or cloned independently of the batch lease.
pub struct PreparedPayload {
    name: String,
    mime_type: String,
    last_modified_ms: Option<f64>,
    bytes: Vec<u8>,
}
impl PreparedPayload {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }
    pub const fn last_modified_ms(&self) -> Option<f64> {
        self.last_modified_ms
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Storage drops before the lease. Moving the batch transfers both; no unleased
/// `into_bytes` or implicit deep clone is available. Caller input is not owned.
pub struct PreparedPayloads {
    entries: Vec<PreparedPayload>,
    plan: UploadPlan,
    _lease: ResourceLease,
}
impl PreparedPayloads {
    /// Apply a post-read filesystem observation without resizing any storage.
    pub fn set_last_modified_ms(
        &mut self,
        index: usize,
        value: Option<f64>,
    ) -> Result<(), SourceError> {
        let entry = self.entries.get_mut(index).ok_or(SourceError::ItemIndex)?;
        entry.last_modified_ms = value;
        Ok(())
    }
    pub fn entries(&self) -> &[PreparedPayload] {
        &self.entries
    }
    pub const fn plan(&self) -> UploadPlan {
        self.plan
    }

    pub fn materialize(
        ledger: &ResourceLedger,
        scope: ResourceScope,
        limits: FileInputLimits,
        sources: &mut [PayloadSource<'_>],
    ) -> Result<Self, SourceError> {
        let plan = limits
            .payloads_iter(sources.iter().map(PayloadSource::metadata))
            .map_err(SourceError::Policy)?;
        // Fixed bookkeeping allowance covers owner and up to three lease request
        // records. Allocator rounding, ledger setup and caller readers are external.
        let metadata = plan
            .items()
            .checked_mul(std::mem::size_of::<PreparedPayload>())
            .and_then(|n| n.checked_add(plan.metadata_bytes()))
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>() + 512))
            .and_then(|n| u64::try_from(n).ok())
            .ok_or(SourceError::SizeOverflow)?;
        let requests = [
            ResourceRequest::new(ResourceClass::LiveObject, 1),
            ResourceRequest::new(ResourceClass::BackendStateBytes, metadata),
            ResourceRequest::new(ResourceClass::ArtifactBytes, plan.source_bytes()),
        ];
        let request_count = if plan.source_bytes() == 0 { 2 } else { 3 };
        let lease = ledger
            .reserve_many(scope, &requests[..request_count])
            .map_err(SourceError::Budget)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(plan.items())
            .map_err(SourceError::Allocation)?;
        for source in sources {
            let name = copy_string(source.name)?;
            let mime_type = copy_string(source.mime_type)?;
            let length = usize::try_from(source.metadata().byte_length)
                .map_err(|_| SourceError::SizeOverflow)?;
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(length)
                .map_err(SourceError::Allocation)?;
            match &mut source.content {
                Content::Bytes(input) => bytes.extend_from_slice(input),
                Content::Reader { reader, .. } => {
                    bytes.resize(length, 0);
                    match reader.read_exact(&mut bytes) {
                        Ok(()) => (),
                        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                            return Err(SourceError::SourceChanged);
                        }
                        Err(error) => return Err(SourceError::Io(error)),
                    }
                    // One stack byte detects growth, including an initially empty
                    // file. Never ask a reader to allocate or read to end.
                    let mut probe = [0];
                    loop {
                        match reader.read(&mut probe) {
                            Ok(0) => break,
                            Ok(_) => return Err(SourceError::SourceChanged),
                            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                            Err(error) => return Err(SourceError::Io(error)),
                        }
                    }
                }
            }
            entries.push(PreparedPayload {
                name,
                mime_type,
                last_modified_ms: source.last_modified_ms,
                bytes,
            });
        }
        Ok(Self {
            entries,
            plan,
            _lease: lease,
        })
    }
}

fn copy_string(value: &str) -> Result<String, SourceError> {
    let mut result = String::new();
    result
        .try_reserve_exact(value.len())
        .map_err(SourceError::Allocation)?;
    result.push_str(value);
    Ok(result)
}

#[cfg(test)]
mod tests;
