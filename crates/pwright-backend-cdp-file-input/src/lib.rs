//! Optional Chromium payload mutation. No path IO, directory traversal or Node.
//! The exact element/group owner and prepared payload owner must remain
//! alive through this call. Caller compositions own operation cancellation and
//! native group cleanup; this primitive never polls selectors or retries mutation.

#[cfg(feature = "retained-preflight")]
mod retained_preflight;
#[cfg(feature = "retained-preflight")]
pub use retained_preflight::{
    CdpFileInputPreflightService, FileInputPreflightFlags, PreflightAdmission,
    RetainedPreflightError, RetainedPreflightProfile,
};

#[cfg(feature = "retained-target-marking")]
mod target_marking;
#[cfg(feature = "retained-target-marking")]
pub use target_marking::{
    CdpTargetMarkingService, MarkerAdmission, MarkerState, MarkerWorld, TargetMarkingError,
    TargetMarkingProfile,
};

#[cfg(feature = "retained-serialized-payload-mutation")]
mod serialized_payload_mutation;
#[cfg(feature = "retained-serialized-payload-mutation")]
pub use serialized_payload_mutation::{
    CdpSerializedPayloadMutationService, SerializedPayloadMutationError,
    SerializedPayloadMutationProfile,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use pwright_backend_cdp_locator_resolution::ResolvedElement;
use pwright_cdp::{CdpClient, connection::CdpError};
use pwright_core::{
    ProductionResourceError, ProductionResourceLedger, ResourceClass, ResourceError,
    ResourceLedger, ResourceRequirements, ResourceScope,
};
use pwright_file_input_model::{FileInputError, InputTarget, UploadPlan};
use pwright_file_input_source::PreparedPayloads;
use serde::{Deserialize, Serialize};
use std::fmt;

const PAYLOAD_SCRIPT: &str = include_str!("payload.js");
const MAX_NATIVE_ID: usize = 1024;

#[derive(Debug)]
pub enum PayloadError {
    InvalidProfile,
    InvalidTarget,
    Capacity,
    InvalidModifiedTime,
    UnresolvedMime,
    Disconnected,
    Protocol,
    Policy(FileInputError),
    Budget(ResourceError),
    Plan(ProductionResourceError),
    Encoding(serde_json::Error),
    Allocation(std::collections::TryReserveError),
    Backend(CdpError),
}
impl fmt::Display for PayloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile => f.write_str("invalid file-input payload backend profile"),
            Self::InvalidTarget => {
                f.write_str("file-input native target identifiers must not be empty")
            }
            Self::Capacity => f.write_str("file-input payload exceeds selected backend capacity"),
            Self::InvalidModifiedTime => f.write_str("file-input modification time must be finite"),
            Self::UnresolvedMime => {
                f.write_str("file-input MIME must be resolved before browser dispatch")
            }
            Self::Disconnected => f.write_str("file-input target is not connected to an element"),
            Self::Protocol => f.write_str("invalid file-input mutation response"),
            Self::Policy(error) => error.fmt(f),
            Self::Budget(error) => error.fmt(f),
            Self::Plan(error) => error.fmt(f),
            Self::Encoding(error) => error.fmt(f),
            Self::Allocation(error) => error.fmt(f),
            Self::Backend(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for PayloadError {}

#[derive(Clone, Copy, Debug)]
pub struct PayloadProfile {
    scope: ResourceScope,
    calls: usize,
    items: usize,
    base64_bytes: u64,
    metadata_bytes: usize,
    operation_bytes: u64,
    large_commands: bool,
}
impl PayloadProfile {
    pub fn new(
        scope: ResourceScope,
        calls: usize,
        items: usize,
        base64_bytes: u64,
        metadata_bytes: usize,
    ) -> Result<Self, PayloadError> {
        if calls == 0 || items == 0 || base64_bytes == 0 || metadata_bytes == 0 {
            return Err(PayloadError::InvalidProfile);
        }
        let operation_bytes = base64_bytes
            .checked_mul(2)
            .and_then(|n| n.checked_add((metadata_bytes as u64).checked_mul(12)?))
            .and_then(|n| n.checked_add((items as u64).checked_mul(2048)?))
            .and_then(|n| {
                n.checked_add((4 * PAYLOAD_SCRIPT.len() + 12 * MAX_NATIVE_ID + 8192) as u64)
            })
            .ok_or(PayloadError::InvalidProfile)?;
        let result = Self {
            large_commands: false,
            scope,
            calls,
            items,
            base64_bytes,
            metadata_bytes,
            operation_bytes,
        };
        result.requirements()?;
        Ok(result)
    }
    /// Select the independently admitted large-command connection pool.
    #[cfg(feature = "large-commands")]
    pub fn with_large_commands(mut self) -> Self {
        self.large_commands = true;
        self
    }
    pub fn requirements(self) -> Result<ResourceRequirements, PayloadError> {
        let count = u64::try_from(self.calls).map_err(|_| PayloadError::InvalidProfile)?;
        let bytes = self
            .operation_bytes
            .checked_mul(count)
            .ok_or(PayloadError::InvalidProfile)?;
        Ok(ResourceRequirements::none()
            .with_class(ResourceClass::ActiveCommand, count, count)
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, self.calls))
    }
    fn validate(self, plan: UploadPlan) -> Result<(), PayloadError> {
        if plan.items() > self.items
            || plan.base64_bytes() > self.base64_bytes
            || plan.metadata_bytes() > self.metadata_bytes
        {
            return Err(PayloadError::Capacity);
        }
        Ok(())
    }
}

/// Borrowed exact native receiver, independent of selector resolution or public
/// Handle identity. This is not an ownership certificate: the caller must hold
/// the same-session object, execution-world route and separately admitted group
/// through native settlement and cleanup. Do not put a persistent Handle in a
/// temporary group that will release it when this upload completes.
///
/// Construction copies no IDs, reserves no resources and performs no native I/O.
/// It neither checks DOM attachment nor adopts objects between execution worlds.
pub struct PayloadTarget<'a> {
    client: &'a dyn CdpClient,
    world: &'a str,
    object_id: &'a str,
    group: &'a str,
}
impl<'a> PayloadTarget<'a> {
    pub fn new(
        client: &'a dyn CdpClient,
        world: &'a str,
        object_id: &'a str,
        group: &'a str,
    ) -> Result<Self, PayloadError> {
        if [world, object_id, group].iter().any(|id| id.is_empty()) {
            return Err(PayloadError::InvalidTarget);
        }
        if [world, object_id, group]
            .iter()
            .any(|id| id.len() > MAX_NATIVE_ID)
        {
            return Err(PayloadError::Capacity);
        }
        Ok(Self {
            client,
            world,
            object_id,
            group,
        })
    }
}

pub struct CdpFilePayloadService {
    profile: PayloadProfile,
    ledger: ResourceLedger,
}
impl CdpFilePayloadService {
    pub fn new(
        profile: PayloadProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, PayloadError> {
        Ok(Self {
            profile,
            ledger: resources
                .claim(profile.requirements()?)
                .map_err(PayloadError::Plan)?,
        })
    }
    /// A single mutation, not a complete public Locator action. The caller must
    /// retain its started operation until native completion and release the
    /// resolution group via the existing cancellation-safe composition.
    pub async fn apply(
        &self,
        element: &ResolvedElement,
        payloads: &PreparedPayloads,
    ) -> Result<PayloadOutcome, PayloadError> {
        let client = element.client();
        let target = PayloadTarget::new(
            client.as_ref(),
            element.utility_world(),
            element.object_id(),
            element.object_group(),
        )?;
        self.apply_on_target(target, payloads).await
    }

    /// A single mutation on a caller-retained native receiver. All encoding,
    /// policy and scratch admission is shared with the Locator entry point.
    /// The caller owns adoption, liveness, cancellation-safe continuation and
    /// group release; this method never retries a selector or disposes a Handle.
    pub async fn apply_on_target(
        &self,
        target: PayloadTarget<'_>,
        payloads: &PreparedPayloads,
    ) -> Result<PayloadOutcome, PayloadError> {
        let plan = payloads.plan();
        self.profile.validate(plan)?;
        if payloads
            .entries()
            .iter()
            .any(|entry| entry.mime_type().is_empty())
        {
            return Err(PayloadError::UnresolvedMime);
        }
        if payloads.entries().iter().any(|entry| {
            entry
                .last_modified_ms()
                .is_some_and(|time| !time.is_finite())
        }) {
            return Err(PayloadError::InvalidModifiedTime);
        }
        let _lease = self
            .ledger
            .reserve_many(
                self.profile.scope,
                &[
                    pwright_core::ResourceRequest::new(ResourceClass::ActiveCommand, 1),
                    pwright_core::ResourceRequest::new(
                        ResourceClass::BackendStateBytes,
                        self.profile.operation_bytes,
                    ),
                ],
            )
            .map_err(PayloadError::Budget)?;
        let arguments = arguments(target.object_id, payloads)?;
        let client = target.client;
        let response = if self.profile.large_commands {
            client
                .runtime_call_function_by_value_in_large_group_to_settlement(
                    target.world,
                    PAYLOAD_SCRIPT,
                    arguments,
                    target.group,
                )
                .await
        } else {
            client
                .runtime_call_function_by_value_in_group_to_settlement(
                    target.world,
                    PAYLOAD_SCRIPT,
                    arguments,
                    target.group,
                )
                .await
        }
        .map_err(PayloadError::Backend)?;
        parse_response(response, plan)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NativePayload<'a> {
    name: &'a str,
    mime_type: &'a str,
    buffer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_modified_ms: Option<f64>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ObjectArgument<'a> {
    object_id: &'a str,
}
#[derive(Serialize)]
struct ValueArgument<T> {
    value: T,
}
fn arguments(
    object_id: &str,
    source: &PreparedPayloads,
) -> Result<Vec<serde_json::Value>, PayloadError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(source.entries().len())
        .map_err(PayloadError::Allocation)?;
    for entry in source.entries() {
        let length = entry.bytes().len().div_ceil(3) * 4;
        let mut buffer = String::new();
        buffer
            .try_reserve_exact(length)
            .map_err(PayloadError::Allocation)?;
        STANDARD.encode_string(entry.bytes(), &mut buffer);
        values.push(NativePayload {
            name: entry.name(),
            mime_type: entry.mime_type(),
            buffer,
            last_modified_ms: entry.last_modified_ms(),
        });
    }
    Ok(vec![
        serde_json::to_value(ObjectArgument { object_id }).map_err(PayloadError::Encoding)?,
        serde_json::to_value(ValueArgument { value: values }).map_err(PayloadError::Encoding)?,
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadOutcome {
    Applied,
    Noop,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Applied,
    Noop,
    Rejected,
    Disconnected,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    input: bool,
    multiple: bool,
    directory: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    status: Status,
    target: Option<Target>,
}
fn parse_response(
    mut response: serde_json::Value,
    plan: UploadPlan,
) -> Result<PayloadOutcome, PayloadError> {
    let value = response
        .get_mut("result")
        .and_then(|result| result.get_mut("value"))
        .ok_or(PayloadError::Protocol)?
        .take();
    // Reject malformed/oversized status values before serde formats an error
    // containing browser-controlled strings. The transport owns envelope bytes.
    let object = value
        .as_object()
        .filter(|object| object.len() == 2)
        .ok_or(PayloadError::Protocol)?;
    if object
        .get("status")
        .and_then(serde_json::Value::as_str)
        .is_none_or(|status| status.len() > 16)
    {
        return Err(PayloadError::Protocol);
    }
    let target = object.get("target").ok_or(PayloadError::Protocol)?;
    if !target.is_null()
        && !target.as_object().is_some_and(|target| {
            target.len() == 3
                && ["input", "multiple", "directory"]
                    .iter()
                    .all(|key| target.get(*key).is_some_and(serde_json::Value::is_boolean))
        })
    {
        return Err(PayloadError::Protocol);
    }
    let result: Response = serde_json::from_value(value).map_err(|_| PayloadError::Protocol)?;
    let Some(target) = result.target else {
        return if matches!(result.status, Status::Disconnected) {
            Err(PayloadError::Disconnected)
        } else {
            Err(PayloadError::Protocol)
        };
    };
    let validation = plan.validate_target(InputTarget {
        is_input: target.input,
        multiple: target.multiple,
        directory: target.directory,
    });
    match (result.status, validation) {
        (Status::Applied, Ok(())) => Ok(PayloadOutcome::Applied),
        (Status::Noop, Ok(())) => Ok(PayloadOutcome::Noop),
        (Status::Rejected, Err(error)) => Err(PayloadError::Policy(error)),
        (Status::Rejected, Ok(()))
        | (Status::Applied | Status::Noop, Err(_))
        | (Status::Disconnected, _) => Err(PayloadError::Protocol),
    }
}

#[cfg(test)]
mod tests;
