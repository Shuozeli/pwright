//! Allocation-free validation before retaining an independent drag-data copy.
use pwright_cdp::{
    CdpEvent,
    generated::input::{DragData, DragDataItem},
};
use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger, ResourceRequirements,
    ResourceScope,
};
use serde_json::Value;
use std::{
    fmt,
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

const MAX_SESSION_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DragDataLimits {
    pub items: usize,
    pub files: usize,
    /// Sum of UTF-8 bytes across MIME types, contents, titles, base URLs and paths.
    pub text_bytes: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DragDataAdmissionError {
    ProfileOverflow,
    ResourceClaim,
    Storage,
    Capacity,
    Session,
    Object,
    UnknownField,
    Items,
    Files,
    ItemCount,
    FileCount,
    String,
    Mask,
    TextBytes,
}
impl fmt::Display for DragDataAdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ProfileOverflow => "drag data profile size overflow",
            Self::ResourceClaim => "drag data resource claim failed",
            Self::Storage => "drag data storage admission failed",
            Self::Capacity => "drag data retained-record capacity exceeded",
            Self::Session => "drag data requires an exact session ID of 1..=128 bytes",
            Self::Object => "drag event/data/item must be an object",
            Self::UnknownField => "drag data contains an unsupported field",
            Self::Items => "drag data requires an items array",
            Self::Files => "drag files must be a string array or null",
            Self::ItemCount => "drag item count exceeds profile",
            Self::FileCount => "drag file count exceeds profile",
            Self::String => "drag item field must be a string (optional fields may be null)",
            Self::Mask => "drag operations mask must be a signed 64-bit integer",
            Self::TextBytes => "drag text payload exceeds UTF-8 byte profile",
        })
    }
}
impl std::error::Error for DragDataAdmissionError {}
type Result<T> = std::result::Result<T, DragDataAdmissionError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DragDataProfile {
    scope: ResourceScope,
    limits: DragDataLimits,
    records: NonZeroUsize,
    record_bytes: u64,
    requirements: ResourceRequirements,
}
impl DragDataProfile {
    pub fn new(
        scope: ResourceScope,
        limits: DragDataLimits,
        records: NonZeroUsize,
    ) -> Result<Self> {
        let bytes = limits
            .items
            .checked_mul(std::mem::size_of::<DragDataItem>())
            .and_then(|bytes| {
                bytes.checked_add(limits.files.checked_mul(std::mem::size_of::<String>())?)
            })
            .and_then(|bytes| bytes.checked_add(limits.text_bytes))
            .and_then(|bytes| bytes.checked_add(MAX_SESSION_BYTES))
            .and_then(|bytes| {
                bytes.checked_add(std::mem::size_of::<Record>() + 2 * std::mem::size_of::<usize>())
            })
            .ok_or(DragDataAdmissionError::ProfileOverflow)?;
        let total = bytes
            .checked_mul(records.get())
            .and_then(|bytes| bytes.checked_add(state_bytes()))
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(DragDataAdmissionError::ProfileOverflow)?;
        let leases = records
            .get()
            .checked_add(1)
            .ok_or(DragDataAdmissionError::ProfileOverflow)?;
        Ok(Self {
            scope,
            limits,
            records,
            record_bytes: u64::try_from(bytes)
                .map_err(|_| DragDataAdmissionError::ProfileOverflow)?,
            requirements: ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, total, total)
                .with_metadata(1, leases),
        })
    }
    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
    pub const fn limits(self) -> DragDataLimits {
        self.limits
    }
    pub const fn record_bytes(self) -> u64 {
        self.record_bytes
    }
}

#[derive(Clone)]
pub struct DragDataStore(Arc<Store>);
struct Store {
    profile: DragDataProfile,
    ledger: ResourceLedger,
    active: AtomicUsize,
    _storage: ResourceLease,
}
fn state_bytes() -> usize {
    std::mem::size_of::<Store>() + 2 * std::mem::size_of::<usize>()
}
impl DragDataStore {
    pub fn new(profile: DragDataProfile, resources: &ProductionResourceLedger) -> Result<Self> {
        let ledger = resources
            .claim(profile.requirements())
            .map_err(|_| DragDataAdmissionError::ResourceClaim)?;
        let storage = ledger
            .reserve(
                profile.scope,
                ResourceClass::BackendStateBytes,
                state_bytes() as u64,
            )
            .map_err(|_| DragDataAdmissionError::Storage)?;
        Ok(Self(Arc::new(Store {
            profile,
            ledger,
            active: AtomicUsize::new(0),
            _storage: storage,
        })))
    }
    pub fn active_records(&self) -> usize {
        self.0.active.load(Ordering::Acquire)
    }
    /// Unrelated events are ignored without validating or retaining their trees.
    /// The caller must subscribe before interception and separately enforce the
    /// capture epoch/first-event policy; session identity alone is not freshness.
    pub fn admit(
        &self,
        event: &CdpEvent,
        expected_session: &str,
    ) -> Result<Option<AdmittedDragData>> {
        if expected_session.is_empty() || expected_session.len() > MAX_SESSION_BYTES {
            return Err(DragDataAdmissionError::Session);
        }
        if event.method != "Input.dragIntercepted"
            || event.session_id.as_deref() != Some(expected_session)
        {
            return Ok(None);
        }
        // No owned strings, vectors, serde decoding or event clone before both
        // structural validation and the independent store admission below.
        let params = object(&event.params, &["data"])?;
        let data = params.get("data").ok_or(DragDataAdmissionError::Object)?;
        validate(data, self.0.profile.limits)?;
        self.0
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < self.0.profile.records.get()).then(|| n + 1)
            })
            .map_err(|_| DragDataAdmissionError::Capacity)?;
        let storage = match self.0.ledger.reserve(
            self.0.profile.scope,
            ResourceClass::BackendStateBytes,
            self.0.profile.record_bytes,
        ) {
            Ok(storage) => storage,
            Err(_) => {
                self.0.active.fetch_sub(1, Ordering::AcqRel);
                return Err(DragDataAdmissionError::Storage);
            }
        };
        let admission = Admission {
            store: self.clone(),
            storage: Some(storage),
        };
        let data = copy_validated(data);
        Ok(Some(AdmittedDragData(Arc::new(Record {
            data,
            session: expected_session.to_owned(),
            _admission: admission,
        }))))
    }
}
struct Admission {
    store: DragDataStore,
    storage: Option<ResourceLease>,
}
impl Drop for Admission {
    fn drop(&mut self) {
        drop(self.storage.take());
        self.store.0.active.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Record {
    data: DragData,
    session: String,
    _admission: Admission,
}
/// Clones share one independently admitted typed payload, not the source event's
/// potentially much larger JSON tree. No deep copy is made by Clone.
#[derive(Clone)]
pub struct AdmittedDragData(Arc<Record>);
impl AdmittedDragData {
    /// Origin identity retained with the data; a dispatcher must match its Page.
    pub fn session_id(&self) -> &str {
        &self.0.session
    }
    /// Borrow only. Caller-created clones/serialization/native request buffers
    /// need their own command admission and are not covered by this record lease.
    pub fn data(&self) -> &DragData {
        &self.0.data
    }
    pub fn shares_payload_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

fn object<'a>(value: &'a Value, keys: &[&str]) -> Result<&'a serde_json::Map<String, Value>> {
    let object = value.as_object().ok_or(DragDataAdmissionError::Object)?;
    if object.len() > keys.len() || object.keys().any(|key| !keys.contains(&key.as_str())) {
        return Err(DragDataAdmissionError::UnknownField);
    }
    Ok(object)
}
fn text<'a>(
    value: Option<&'a Value>,
    optional: bool,
    bytes: &mut usize,
    limit: usize,
) -> Result<Option<&'a str>> {
    if optional && value.is_none_or(Value::is_null) {
        return Ok(None);
    }
    let value = value
        .and_then(Value::as_str)
        .ok_or(DragDataAdmissionError::String)?;
    *bytes = bytes
        .checked_add(value.len())
        .filter(|bytes| *bytes <= limit)
        .ok_or(DragDataAdmissionError::TextBytes)?;
    Ok(Some(value))
}
fn validate(value: &Value, limits: DragDataLimits) -> Result<()> {
    let data = object(value, &["items", "files", "dragOperationsMask"])?;
    let items = data
        .get("items")
        .and_then(Value::as_array)
        .ok_or(DragDataAdmissionError::Items)?;
    if items.len() > limits.items {
        return Err(DragDataAdmissionError::ItemCount);
    }
    if data
        .get("dragOperationsMask")
        .and_then(Value::as_i64)
        .is_none()
    {
        return Err(DragDataAdmissionError::Mask);
    }
    let mut bytes = 0;
    for item in items {
        let item = object(item, &["mimeType", "data", "title", "baseURL"])?;
        for field in ["mimeType", "data"] {
            text(item.get(field), false, &mut bytes, limits.text_bytes)?;
        }
        for field in ["title", "baseURL"] {
            text(item.get(field), true, &mut bytes, limits.text_bytes)?;
        }
    }
    if let Some(files) = data.get("files").filter(|value| !value.is_null()) {
        let files = files.as_array().ok_or(DragDataAdmissionError::Files)?;
        if files.len() > limits.files {
            return Err(DragDataAdmissionError::FileCount);
        }
        for path in files {
            text(Some(path), false, &mut bytes, limits.text_bytes)?;
        }
    }
    Ok(())
}
fn copy_validated(value: &Value) -> DragData {
    let items = value["items"].as_array().expect("validated items");
    let mut copied = Vec::with_capacity(items.len());
    for item in items {
        copied.push(DragDataItem {
            mime_type: item["mimeType"]
                .as_str()
                .expect("validated MIME")
                .to_owned(),
            data: item["data"].as_str().expect("validated data").to_owned(),
            title: item.get("title").and_then(Value::as_str).map(str::to_owned),
            base_url: item
                .get("baseURL")
                .and_then(Value::as_str)
                .map(str::to_owned),
        });
    }
    let files = value.get("files").and_then(Value::as_array).map(|files| {
        let mut copied = Vec::with_capacity(files.len());
        for file in files {
            copied.push(file.as_str().expect("validated path").to_owned());
        }
        copied
    });
    DragData {
        items: copied,
        files,
        drag_operations_mask: value["dragOperationsMask"]
            .as_i64()
            .expect("validated mask"),
    }
}

#[cfg(test)]
mod tests;
