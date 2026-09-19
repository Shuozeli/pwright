//! One-shot DOM prevalidation in a caller-owned native group. No payload read,
//! encoding, mutation, selector query or independent native release.

use super::{MAX_NATIVE_ID, PayloadTarget};
use pwright_cdp::connection::CdpError;
use pwright_core::{
    ProductionResourceError, ProductionResourceLedger, ResourceClass, ResourceError, ResourceLease,
    ResourceLedger, ResourceRequest, ResourceRequirements, ResourceScope,
};
use pwright_file_input_model::FileInputError;
use serde_json::Value;
use std::{fmt, future::Future};

const SCRIPT: &str = include_str!("retained_preflight.js");
const EXTRA_NATIVE: u64 = 2;
const DIAGNOSTIC_BYTES: usize = 1024;
const CONTROL_BYTES: usize = 8192;
// Typed CDP params and their JSON value can overlap: 2 copies each of world,
// group and receiver IDs, plus the separately retained result ID. The script
// can likewise appear in both typed/JSON params. Diagnostic String -> Box<str>
// conversion may briefly retain two bounded buffers. CONTROL_BYTES covers the
// fixed-shape argument/params container backing and the checked inline future.
// Serialized wire frames and decoded reply envelopes remain transport-bound.
const ADMISSION_BYTES: u64 =
    (7 * MAX_NATIVE_ID + 2 * SCRIPT.len() + 2 * DIAGNOSTIC_BYTES + CONTROL_BYTES) as u64;

/// Already-selected native truth values, NOT projection of a raw JS carrier.
/// The language layer must preserve its own read/coercion/serialization order.
#[derive(Clone, Copy, Debug)]
pub struct FileInputPreflightFlags {
    pub multiple: bool,
    pub directory_upload: bool,
}

#[derive(Debug)]
pub enum RetainedPreflightError {
    InvalidProfile,
    AlreadyUsed,
    Disconnected,
    Protocol(&'static str),
    Policy(FileInputError),
    /// Typed transport/protocol failures with bounded owned diagnostics.
    /// Opaque JSON and recursive compound errors normalize to `CdpError::Other`.
    Backend(CdpError),
    JavaScript(Box<str>),
    Budget(ResourceError),
    Plan(ProductionResourceError),
}
impl fmt::Display for RetainedPreflightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile => f.write_str("invalid retained file-input preflight profile"),
            Self::AlreadyUsed => {
                f.write_str("retained file-input preflight admission was already used")
            }
            Self::Disconnected => f.write_str("file-input target is not connected to an element"),
            Self::Protocol(message) => f.write_str(message),
            Self::Policy(error) => error.fmt(f),
            Self::Backend(error) => error.fmt(f),
            Self::JavaScript(message) => f.write_str(message),
            Self::Budget(error) => error.fmt(f),
            Self::Plan(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for RetainedPreflightError {}

/// Separate opt-in capacity for derived result AND exception wrappers. The
/// existing adopted group's native units are owned and budgeted elsewhere.
#[derive(Clone, Copy, Debug)]
pub struct RetainedPreflightProfile {
    scope: ResourceScope,
    capacity: usize,
}
impl RetainedPreflightProfile {
    pub fn new(scope: ResourceScope, capacity: usize) -> Result<Self, RetainedPreflightError> {
        let count = u64::try_from(capacity).map_err(|_| RetainedPreflightError::InvalidProfile)?;
        if count == 0 || count.checked_mul(ADMISSION_BYTES).is_none() {
            return Err(RetainedPreflightError::InvalidProfile);
        }
        Ok(Self { scope, capacity })
    }

    pub fn requirements(self) -> ResourceRequirements {
        let count = self.capacity as u64;
        ResourceRequirements::none()
            .with_class(
                ResourceClass::RemoteHandle,
                count * EXTRA_NATIVE,
                count * EXTRA_NATIVE,
            )
            .with_class(
                ResourceClass::BackendStateBytes,
                count * ADMISSION_BYTES,
                count * ADMISSION_BYTES,
            )
            .with_metadata(1, self.capacity)
    }
}

pub struct CdpFileInputPreflightService {
    profile: RetainedPreflightProfile,
    ledger: ResourceLedger,
}
impl CdpFileInputPreflightService {
    pub fn new(
        profile: RetainedPreflightProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, RetainedPreflightError> {
        Ok(Self {
            profile,
            ledger: resources
                .claim(profile.requirements())
                .map_err(RetainedPreflightError::Plan)?,
        })
    }

    /// Reserve before any native prevalidation call. This allowance must stay
    /// retained until the caller's ENTIRE group has acknowledged release (or
    /// its owner reports authoritative destruction), on every outcome.
    pub fn admit(&self) -> Result<PreflightAdmission, RetainedPreflightError> {
        let lease = self
            .ledger
            .reserve_many(
                self.profile.scope,
                &[
                    ResourceRequest::new(ResourceClass::RemoteHandle, EXTRA_NATIVE),
                    ResourceRequest::new(ResourceClass::BackendStateBytes, ADMISSION_BYTES),
                ],
            )
            .map_err(RetainedPreflightError::Budget)?;
        Ok(PreflightAdmission {
            used: false,
            object_id: None,
            _lease: lease,
        })
    }
}

/// Caller-owned group backing, not an independently releasable native object.
/// A raw caller must retain started validation and this admission through group
/// cleanup; prefer the cancellation-safe file-input composition owner.
pub struct PreflightAdmission {
    used: bool,
    object_id: Option<Box<str>>,
    _lease: ResourceLease,
}
impl PreflightAdmission {
    pub fn object_id(&self) -> Option<&str> {
        self.object_id.as_deref()
    }

    /// One call only, even when the returned future is never polled, fails, or
    /// is abandoned. No second result can escape the two-wrapper allowance.
    pub fn validate<'a>(
        &'a mut self,
        target: PayloadTarget<'a>,
        flags: FileInputPreflightFlags,
    ) -> impl Future<Output = Result<(), RetainedPreflightError>> + Send + 'a {
        let used = std::mem::replace(&mut self.used, true);
        let future = async move {
            if used {
                return Err(RetainedPreflightError::AlreadyUsed);
            }
            let response = target.client.runtime_call_function_as_object_in_group_to_settlement(
                target.world,
                SCRIPT,
                vec![serde_json::json!({ "objectId": target.object_id }), serde_json::json!({
                    "value": { "multiple": flags.multiple, "directoryUpload": flags.directory_upload }
                })],
                target.group,
                true,
            ).await.map_err(|error| RetainedPreflightError::Backend(bounded_error(error)))?;
            self.object_id = Some(parse(&response)?);
            Ok(())
        };
        check_validation_storage(&future);
        future
    }
}

fn check_validation_storage<F>(_: &F) {
    // Reserve half the control allowance for fixed-shape JSON/container
    // backing, independently of native strings/script/diagnostic allocations.
    const {
        assert!(
            std::mem::size_of::<F>()
                + std::mem::size_of::<PreflightAdmission>()
                + std::mem::size_of::<RetainedPreflightError>()
                + 4096
                < CONTROL_BYTES
        );
    }
}

fn parse(response: &Value) -> Result<Box<str>, RetainedPreflightError> {
    if let Some(exception) = response.get("exceptionDetails") {
        let message = exception
            .pointer("/exception/description")
            .and_then(Value::as_str)
            .or_else(|| exception.get("text").and_then(Value::as_str))
            .unwrap_or("retained file-input DOM prevalidation threw");
        return Err(RetainedPreflightError::JavaScript(bounded_text(message)));
    }
    let result = response
        .get("result")
        .ok_or(RetainedPreflightError::Protocol(
            "missing prevalidation result",
        ))?;
    if result.get("type").and_then(Value::as_str) == Some("number")
        && result.get("objectId").is_none()
    {
        return Err(match result.get("value").and_then(Value::as_u64) {
            Some(0) => RetainedPreflightError::Disconnected,
            Some(1) => RetainedPreflightError::Policy(FileInputError::NotInput),
            Some(2) => RetainedPreflightError::Policy(FileInputError::MultipleFiles),
            Some(3) => RetainedPreflightError::Policy(FileInputError::DirectoryNotSupported),
            Some(4) => RetainedPreflightError::Policy(FileInputError::DirectoryRequired),
            _ => RetainedPreflightError::Protocol("invalid prevalidation status"),
        });
    }
    if result.get("type").and_then(Value::as_str) != Some("object")
        || result.get("subtype").and_then(Value::as_str) != Some("node")
    {
        return Err(RetainedPreflightError::Protocol(
            "expected retargeted input node",
        ));
    }
    let id = result
        .get("objectId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= MAX_NATIVE_ID)
        .ok_or(RetainedPreflightError::Protocol(
            "missing or oversized prevalidation native ID",
        ))?;
    Ok(id.into())
}

fn bounded_text(text: &str) -> Box<str> {
    let mut end = text.len().min(DIAGNOSTIC_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].into()
}

/// Preserve exact terminal/resource kinds and protocol codes, never classify
/// arbitrary diagnostic text or infer successful native cleanup from an error.
/// Fresh bounded strings discard oversized backing capacity. Opaque JSON and
/// recursive compound storage cannot remain in the admitted error: JSON uses a
/// bounded formatter, while Compound deliberately avoids recursive formatting.
fn bounded_error(error: CdpError) -> CdpError {
    match error {
        CdpError::WebSocket(message) => CdpError::WebSocket(compact(&message, DIAGNOSTIC_BYTES)),
        CdpError::Protocol { code, message } => CdpError::Protocol {
            code,
            message: compact(&message, DIAGNOSTIC_BYTES),
        },
        CdpError::ElementNotFound { selector } => CdpError::ElementNotFound {
            selector: compact(&selector, DIAGNOSTIC_BYTES),
        },
        CdpError::NavigationFailed { url, reason } => CdpError::NavigationFailed {
            // Both retained strings share the single diagnostic allowance.
            url: compact(&url, DIAGNOSTIC_BYTES / 2),
            reason: compact(&reason, DIAGNOSTIC_BYTES - DIAGNOSTIC_BYTES / 2),
        },
        CdpError::TabNotFound(message) => {
            CdpError::TabNotFound(compact(&message, DIAGNOSTIC_BYTES))
        }
        CdpError::HttpFailed(message) => CdpError::HttpFailed(compact(&message, DIAGNOSTIC_BYTES)),
        CdpError::JsException(message) => {
            CdpError::JsException(compact(&message, DIAGNOSTIC_BYTES))
        }
        CdpError::Other(message) => CdpError::Other(compact(&message, DIAGNOSTIC_BYTES)),
        error @ CdpError::Json(_) => CdpError::Other(diagnostic(&error)),
        CdpError::Compound { .. } => {
            CdpError::Other("compound CDP error during retained file-input prevalidation".into())
        }
        error @ (CdpError::Closed
        | CdpError::SessionDetached
        | CdpError::ChannelDropped
        | CdpError::Timeout
        | CdpError::LimitExceeded { .. }
        | CdpError::InvalidConfiguration(_)
        | CdpError::IdentifierExhausted
        | CdpError::Resource(_)
        | CdpError::PageClosed) => error,
    }
}

fn compact(text: &str, maximum: usize) -> String {
    let mut end = text.len().min(maximum);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    // Truncating the input in place would retain its potentially huge capacity.
    String::from(&text[..end])
}

fn diagnostic(error: &CdpError) -> String {
    use fmt::Write;
    struct Bounded {
        value: String,
    }
    impl fmt::Write for Bounded {
        fn write_str(&mut self, value: &str) -> fmt::Result {
            let available = DIAGNOSTIC_BYTES - self.value.len();
            let mut end = available.min(value.len());
            while !value.is_char_boundary(end) {
                end -= 1;
            }
            self.value.push_str(&value[..end]);
            if end < value.len() {
                Err(fmt::Error)
            } else {
                Ok(())
            }
        }
    }
    let mut output = Bounded {
        value: String::with_capacity(DIAGNOSTIC_BYTES),
    };
    let _ = write!(&mut output, "{error}");
    output.value
}

#[cfg(test)]
mod tests;
