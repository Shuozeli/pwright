//! Explicit selected host budgets. No ordinary Node or CDP limits are raised.
use crate::HostError;
use pwright_backend_cdp_file_input::PayloadProfile;
use pwright_cdp::connection::large_commands::LargeCommandProfile;
use pwright_composition_chromium_file_input::UploadProfile;
use pwright_core::{ResourceClass, ResourceRequirements, ResourceScope};
use pwright_file_input_model::{FileInputLimits, UPLOAD_SIZE_LIMIT};
use pwright_file_input_tokio::WorkProfile;

pub const ITEMS: usize = 64;
pub const PATH_BYTES: usize = 4096;
pub const NAME_BYTES: usize = 4096;
pub const MIME_BYTES: usize = 1024;
pub const METADATA_BYTES: usize = 64 * 1024;
pub(super) const STATE_SCOPE: ResourceScope = ResourceScope::extension(148, 1);
pub(super) const SOURCE_SCOPE: ResourceScope = ResourceScope::extension(148, 2);
pub(super) const STATE_BYTES: u64 = 32 * 1024;
pub(super) const CALL_BYTES: u64 = 32 * 1024;
pub(super) const CALLS: u64 = 8;

pub fn limits() -> FileInputLimits {
    FileInputLimits::new(ITEMS, PATH_BYTES, NAME_BYTES, MIME_BYTES, METADATA_BYTES)
        .expect("positive fixed file-input limits")
}
pub(crate) fn large_commands() -> Result<LargeCommandProfile, HostError> {
    LargeCommandProfile::new(
        ResourceScope::extension(148, 3),
        1,
        72 * 1024 * 1024,
        64 * 1024,
    )
    .map_err(HostError::backend)
}
pub(super) fn payload() -> Result<PayloadProfile, HostError> {
    PayloadProfile::new(
        ResourceScope::extension(148, 4),
        1,
        ITEMS,
        70 * 1024 * 1024,
        METADATA_BYTES,
    )
    .map(PayloadProfile::with_large_commands)
    .map_err(HostError::backend)
}
pub(super) fn upload() -> Result<UploadProfile, HostError> {
    UploadProfile::new(ResourceScope::extension(148, 5), 1, 16 * 1024).map_err(HostError::backend)
}
pub(super) fn worker() -> Result<WorkProfile, HostError> {
    WorkProfile::new(1, limits(), ResourceScope::extension(148, 6), 16 * 1024)
        .map_err(HostError::backend)
}
pub(super) fn source_requirements() -> ResourceRequirements {
    // One total raw-byte ceiling, shared by preparing and already-started
    // batches. Path records/descriptors and source metadata can overlap.
    ResourceRequirements::none()
        .with_class(
            ResourceClass::ArtifactBytes,
            UPLOAD_SIZE_LIMIT,
            UPLOAD_SIZE_LIMIT,
        )
        .with_class(
            ResourceClass::LiveObject,
            ITEMS as u64 + CALLS + 2,
            ITEMS as u64 + CALLS + 2,
        )
        .with_class(
            ResourceClass::BackendStateBytes,
            4 * 1024 * 1024,
            4 * 1024 * 1024,
        )
        .with_metadata(2, ITEMS + CALLS as usize + 8)
}
pub(super) fn state_requirements() -> ResourceRequirements {
    let bytes = STATE_BYTES + CALLS * CALL_BYTES;
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_class(ResourceClass::ActiveCommand, CALLS, CALLS)
        .with_metadata(1, CALLS as usize + 1)
}
pub(crate) fn requirements() -> Result<ResourceRequirements, HostError> {
    let required = state_requirements()
        .combine_concurrent(source_requirements())
        .combine_concurrent(payload()?.requirements().map_err(HostError::backend)?)
        .combine_concurrent(upload()?.requirements())
        .combine_concurrent(worker()?.requirements().map_err(HostError::backend)?);
    #[cfg(feature = "retained-file-input")]
    let required =
        required.combine_concurrent(crate::handles::file_input::RetainedUploads::requirements()?);
    #[cfg(feature = "retained-serialized-payload-import")]
    let required =
        required.combine_concurrent(crate::handles::file_input::serialized_payload::requirements());
    Ok(required)
}
