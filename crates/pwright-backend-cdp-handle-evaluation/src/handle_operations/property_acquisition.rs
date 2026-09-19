//! Raw property-response ownership before the shared pure parser runs.

use super::{CdpHandleEvaluationLimits, CdpHandlePropertyLimits};
use crate::handle_evaluation::acquisition::{
    Diagnostic, NativeIdentity, bounded_protocol_error, javascript_error,
};
use pwright_cap_handle_evaluation::HandleEvaluationErrorDetails;
use pwright_cdp::CdpClient;
use pwright_core::{FailureKind, OperationError};
use serde_json::Value;
use std::fmt::Display;

type Error = OperationError<HandleEvaluationErrorDetails>;

// The backend temporarily owns a carrier plus N+1 overflow-detection values.
// This is distinct from N persistent route/host result slots. Fixed wrappers,
// staging vectors, Arc owners and publication names fit this conservative bound.
const WRAPPER_BYTES: u64 = 512;
// At most the raw response, one descriptor clone and one reconstructed native
// primitive coexist. Strings additionally cover two escaped JSON buffers (at
// most six bytes per input byte), preview/name copies and Arc staging.
const STRUCTURAL_COPIES: u64 = 3;
const STRING_STAGING_FACTOR: u64 = 16;
// serde_json uses BTreeMap in this workspace. Charge a whole sparse node per
// entry (rather than assuming densely packed nodes or serialized-byte ratios).
// 1 KiB covers its 11 String/Value slots, header and internal edges on 64-bit.
const MAP_ENTRY_BYTES: u64 = 1024;
const MAX_RESPONSE_DEPTH: usize = 128;

fn wrapper_bytes(properties: CdpHandlePropertyLimits) -> u64 {
    u64::try_from(properties.max_properties())
        .unwrap_or(u64::MAX)
        .saturating_add(2)
        .saturating_mul(WRAPPER_BYTES)
}

pub(super) fn admit(
    limits: CdpHandleEvaluationLimits,
    properties: CdpHandlePropertyLimits,
) -> Result<(), Error> {
    if wrapper_bytes(properties) >= limits.evaluation().maximum_retained_bytes_per_operation() {
        return Err(capacity_error(limits.evaluation().error_bytes));
    }
    Ok(())
}

pub(super) fn validate(
    response: &Value,
    limits: CdpHandleEvaluationLimits,
    properties: CdpHandlePropertyLimits,
) -> Result<(), Error> {
    if let Some(details) = response.get("exceptionDetails") {
        return Err(javascript_error(details, limits.evaluation().error_bytes));
    }
    // CDP bounds the incoming decoded response. Before the pure parser copies
    // names/values, additionally bound simultaneous pipeline staging against
    // the SAME evaluation allowance. This bounded-depth walk accounts actual
    // String/Vec capacity and sparse map nodes without allocating another tree.
    let available = limits
        .evaluation()
        .maximum_retained_bytes_per_operation()
        .saturating_sub(wrapper_bytes(properties));
    let mut budget = StagingBudget {
        remaining: available,
    };
    budget
        .value(response, 0)
        .map_err(|_| capacity_error(limits.evaluation().error_bytes))?;
    for (index, remote) in remotes(response).enumerate() {
        match NativeIdentity::read(Some(remote), limits.native_id_bytes()) {
            NativeIdentity::Unresolved => {
                return Err(bounded_protocol_error(
                    "property response has unresolved native ownership",
                    limits.evaluation().error_bytes,
                ));
            }
            NativeIdentity::Addressable(id) => {
                if remotes(response)
                    .take(index)
                    .any(|earlier| earlier.get("objectId").and_then(Value::as_str) == Some(id))
                {
                    return Err(bounded_protocol_error(
                        "property response repeats a native objectId",
                        limits.evaluation().error_bytes,
                    ));
                }
            }
            NativeIdentity::Absent => {}
        }
    }
    // A null-prototype data-only carrier must not create unreturned owners.
    // Reject native accessor/symbol/internal/private owners, then compensate
    // them on the common raw-response failure path instead of leaking them.
    if extra_remotes(response).any(|remote| remote.get("objectId").is_some()) {
        return Err(bounded_protocol_error(
            "property response contains unexpected retained owners",
            limits.evaluation().error_bytes,
        ));
    }
    if response
        .get("result")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|descriptor| {
            descriptor.get("enumerable").and_then(Value::as_bool) != Some(true)
                && descriptor
                    .get("value")
                    .is_some_and(|value| value.get("objectId").is_some())
        })
    {
        return Err(bounded_protocol_error(
            "property carrier returned a non-enumerable native owner",
            limits.evaluation().error_bytes,
        ));
    }
    Ok(())
}

struct StagingBudget {
    remaining: u64,
}

impl StagingBudget {
    fn charge(&mut self, count: u64) -> Result<(), ()> {
        if count > self.remaining {
            return Err(());
        }
        self.remaining -= count;
        Ok(())
    }

    fn string(&mut self, value: &String) -> Result<(), ()> {
        self.charge(
            u64::try_from(value.capacity())
                .unwrap_or(u64::MAX)
                .saturating_mul(STRING_STAGING_FACTOR),
        )
    }

    fn value(&mut self, value: &Value, depth: usize) -> Result<(), ()> {
        if depth > MAX_RESPONSE_DEPTH {
            return Err(());
        }
        let value_bytes = u64::try_from(std::mem::size_of::<Value>()).unwrap_or(u64::MAX);
        self.charge(value_bytes.saturating_mul(STRUCTURAL_COPIES))?;
        match value {
            Value::String(value) => self.string(value)?,
            Value::Array(values) => {
                self.charge(
                    u64::try_from(values.capacity())
                        .unwrap_or(u64::MAX)
                        .saturating_mul(value_bytes)
                        .saturating_mul(STRUCTURAL_COPIES),
                )?;
                for value in values {
                    self.value(value, depth + 1)?;
                }
            }
            Value::Object(values) => {
                self.charge(
                    u64::try_from(values.len())
                        .unwrap_or(u64::MAX)
                        .saturating_mul(MAP_ENTRY_BYTES)
                        .saturating_mul(STRUCTURAL_COPIES),
                )?;
                for (key, value) in values {
                    self.string(key)?;
                    self.value(value, depth + 1)?;
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
        Ok(())
    }
}

fn extra_remotes(response: &Value) -> impl Iterator<Item = &Value> + Clone {
    response
        .get("result")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|descriptor| {
            ["get", "set", "symbol"]
                .into_iter()
                .filter_map(move |key| descriptor.get(key))
        })
        .chain(
            ["internalProperties", "privateProperties"]
                .into_iter()
                .flat_map(move |key| {
                    response
                        .get(key)
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                })
                .flat_map(|descriptor| {
                    ["value", "get", "set"]
                        .into_iter()
                        .filter_map(move |key| descriptor.get(key))
                }),
        )
}

fn remotes(response: &Value) -> impl Iterator<Item = &Value> + Clone {
    response
        .get("result")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|descriptor| descriptor.get("value"))
        // Malformed CDP responses can put one RemoteObject where the result
        // array belongs. Its addressable ID still needs compensation.
        .chain(response.get("result").filter(|value| !value.is_array()))
        .chain(extra_remotes(response))
        .chain(
            response
                .get("exceptionDetails")
                .and_then(|details| details.get("exception")),
        )
}

pub(super) fn contains_id(response: &Value, id: &str) -> bool {
    remotes(response).any(|remote| remote.get("objectId").and_then(Value::as_str) == Some(id))
}

/// Every addressable raw owner is attempted, even after another release fails.
/// IDs are borrowed and deduplicated without an unbounded copied-ID collection.
pub(super) async fn release_response(
    client: &dyn CdpClient,
    response: &Value,
    already_attempted: Option<&str>,
    limits: CdpHandleEvaluationLimits,
) -> Result<(), String> {
    let mut first_failure = None;
    for (index, remote) in remotes(response).enumerate() {
        match NativeIdentity::read(Some(remote), limits.native_id_bytes()) {
            NativeIdentity::Absent => {}
            NativeIdentity::Unresolved => {
                first_failure.get_or_insert_with(|| {
                    Diagnostic::display(
                        "unresolved native ownership: missing, invalid or oversized objectId",
                        limits.evaluation().error_bytes,
                    )
                });
            }
            NativeIdentity::Addressable(id) => {
                if already_attempted == Some(id)
                    || remotes(response)
                        .take(index)
                        .any(|earlier| earlier.get("objectId").and_then(Value::as_str) == Some(id))
                {
                    continue;
                }
                if let Err(error) = client.runtime_release_object_to_settlement(id).await
                    && first_failure.is_none()
                {
                    first_failure =
                        Some(Diagnostic::display(error, limits.evaluation().error_bytes));
                }
            }
        }
    }
    first_failure.map_or(Ok(()), Err)
}

pub(super) fn with_cleanup<E: Display>(primary: Error, cleanup: Option<E>, limit: usize) -> Error {
    let Some(cleanup) = cleanup else {
        return primary;
    };
    let primary = primary.map_details(|_| ());
    let primary_message = Diagnostic::display(primary.message(), limit);
    drop(primary);
    let cleanup = Diagnostic::display(cleanup, limit);
    let message = Diagnostic::display(
        format_args!("{primary_message}; cleanup failed: {cleanup}"),
        limit,
    );
    OperationError::new(
        FailureKind::Cleanup,
        message,
        Some(HandleEvaluationErrorDetails::Cleanup {
            primary: primary_message,
            cleanup,
        }),
    )
}

fn capacity_error(limit: usize) -> Error {
    OperationError::new(
        FailureKind::ResourceExhausted,
        Diagnostic::display(
            "Handle property staging exceeds the admitted evaluation allowance",
            limit,
        ),
        None,
    )
}
