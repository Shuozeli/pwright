//! Ownership at the raw retained-result boundary, before semantic parsing.
//! The response owns borrowed IDs until all failure releases have settled.

use super::{CdpEvaluatedHandle, CdpHandleEvaluationLimits, parse_borrowed_response};
use pwright_cap_handle_evaluation::HandleEvaluationErrorDetails;
use pwright_cdp::CdpClient;
use pwright_core::{FailureKind, OperationError};
use serde_json::Value;
use std::{
    fmt::{self, Write},
    sync::Arc,
};

type Error = OperationError<HandleEvaluationErrorDetails>;

#[derive(Clone, Copy)]
pub(crate) enum NativeIdentity<'a> {
    Absent,
    Addressable(&'a str),
    Unresolved,
}

impl<'a> NativeIdentity<'a> {
    pub(crate) fn read(remote: Option<&'a Value>, limit: usize) -> Self {
        let Some(remote) = remote else {
            return Self::Absent;
        };
        match remote.get("objectId") {
            Some(Value::String(id)) if !id.is_empty() && id.len() <= limit => Self::Addressable(id),
            Some(_) => Self::Unresolved,
            None if matches!(
                remote.get("type").and_then(Value::as_str),
                Some("object" | "function")
            ) && remote.get("subtype").and_then(Value::as_str) != Some("null") =>
            {
                Self::Unresolved
            }
            None => Self::Absent,
        }
    }
}

/// Existing evaluation admission stays outside this future through cleanup.
/// No ID copy, new queue, destructor I/O or detached task is introduced.
pub(super) async fn finish(
    client: Arc<dyn CdpClient>,
    response: Value,
    limits: CdpHandleEvaluationLimits,
) -> Result<CdpEvaluatedHandle, Error> {
    finish_with_protected_receiver(client, response, limits, None).await
}

/// The caller owns the bounded receiver ID under the same evaluation lease.
/// A reply cannot transfer that borrowed owner, even on a malformed/error path.
pub(super) async fn finish_protecting_receiver(
    client: Arc<dyn CdpClient>,
    response: Value,
    limits: CdpHandleEvaluationLimits,
    protected_receiver: &str,
) -> Result<CdpEvaluatedHandle, Error> {
    finish_with_protected_receiver(client, response, limits, Some(protected_receiver)).await
}

async fn finish_with_protected_receiver(
    client: Arc<dyn CdpClient>,
    response: Value,
    limits: CdpHandleEvaluationLimits,
    protected_receiver: Option<&str>,
) -> Result<CdpEvaluatedHandle, Error> {
    let result_id = NativeIdentity::read(response.get("result"), limits.native_id_bytes());
    let exception = response.get("exceptionDetails");
    let exception_id = NativeIdentity::read(
        exception.and_then(|details| details.get("exception")),
        limits.native_id_bytes(),
    );
    let primary = match exception {
        Some(details) => javascript_error(details, limits.evaluation().error_bytes),
        None => {
            if matches!(result_id, NativeIdentity::Addressable(id) if Some(id) == protected_receiver)
            {
                bounded_protocol_error(
                    "CDP retained result aliases the borrowed native receiver",
                    limits.evaluation().error_bytes,
                )
            } else if matches!(result_id, NativeIdentity::Unresolved) {
                bounded_protocol_error(
                    "CDP retained result has a missing, invalid or oversized objectId",
                    limits.evaluation().error_bytes,
                )
            } else {
                match parse_borrowed_response(Arc::clone(&client), &response, limits) {
                    Ok(handle) => return Ok(handle),
                    Err(error) => error,
                }
            }
        }
    };
    let mut cleanup: Option<Diagnostic> = None;
    let mut previous = None;
    for identity in [result_id, exception_id] {
        match identity {
            NativeIdentity::Absent => {}
            NativeIdentity::Unresolved => {
                let diagnostic =
                    cleanup.get_or_insert_with(|| Diagnostic::new(limits.evaluation().error_bytes));
                let _ = diagnostic.write_str(
                    "unresolved native ownership: missing, invalid or oversized objectId; ",
                );
            }
            NativeIdentity::Addressable(id) => {
                if previous == Some(id) {
                    continue;
                }
                previous = Some(id);
                if Some(id) == protected_receiver {
                    let diagnostic = cleanup
                        .get_or_insert_with(|| Diagnostic::new(limits.evaluation().error_bytes));
                    let _ = diagnostic
                        .write_str("temporary native ownership aliases the borrowed receiver; ");
                    continue;
                }
                if let Err(error) = client.runtime_release_object_to_settlement(id).await {
                    let diagnostic = cleanup
                        .get_or_insert_with(|| Diagnostic::new(limits.evaluation().error_bytes));
                    let _ = write!(diagnostic, "native release failed: {error}; ");
                }
            }
        }
    }
    let Some(cleanup) = cleanup else {
        return Err(primary);
    };
    // Drop detailed primary copies before allocating the final diagnostic fields.
    // Peak diagnostic storage fits the existing four-error-buffer allowance.
    let primary = primary.map_details(|_| ());
    let primary_message = Diagnostic::display(primary.message(), limits.evaluation().error_bytes);
    drop(primary);
    let cleanup = cleanup.text;
    let message = Diagnostic::display(
        format_args!("{primary_message}; cleanup failed: {cleanup}"),
        limits.evaluation().error_bytes,
    );
    Err(OperationError::new(
        FailureKind::Cleanup,
        message,
        Some(HandleEvaluationErrorDetails::Cleanup {
            primary: primary_message,
            cleanup,
        }),
    ))
}

pub(crate) fn javascript_error(details: &Value, limit: usize) -> Error {
    if !details.is_object() {
        return bounded_protocol_error("CDP exceptionDetails is not an object", limit);
    }
    let mut diagnostic = Diagnostic::new(limit);
    let text = details
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("JavaScript exception");
    let _ = diagnostic.write_str(text);
    let exception = details.get("exception");
    if let Some(description) = exception
        .and_then(|value| value.get("description"))
        .and_then(Value::as_str)
    {
        let _ = write!(diagnostic, ": {}", description.trim_end());
    } else if let Some(value) = exception.and_then(|value| value.get("value")) {
        // Value's Display streams JSON through the bounded writer; no to_string
        // allocation of an arbitrarily large thrown value is made.
        let _ = write!(diagnostic, ": {value}");
    }
    if let Some(line) = details.get("lineNumber").and_then(Value::as_i64) {
        let _ = write!(diagnostic, " at line {line}");
    }
    if let Some(column) = details.get("columnNumber").and_then(Value::as_i64) {
        let _ = write!(diagnostic, ":{column}");
    }
    if diagnostic.truncated {
        return bounded_protocol_error(
            "CDP JavaScript exception exceeds the configured byte limit",
            limit,
        );
    }
    let message = Diagnostic::display(
        format_args!("JavaScript exception: {}", diagnostic.text),
        limit,
    );
    OperationError::new(
        FailureKind::Backend,
        message,
        Some(HandleEvaluationErrorDetails::Javascript {
            name: Diagnostic::display("Error", limit),
            message: diagnostic.text,
            stack: None,
        }),
    )
}

pub(crate) fn bounded_protocol_error(message: &str, limit: usize) -> Error {
    let message = Diagnostic::display(message, limit);
    OperationError::new(
        FailureKind::ProtocolViolation,
        message.clone(),
        Some(HandleEvaluationErrorDetails::InvalidBackendResponse(
            message,
        )),
    )
}

pub(crate) struct Diagnostic {
    text: String,
    limit: usize,
    truncated: bool,
}

impl Diagnostic {
    fn new(limit: usize) -> Self {
        Self {
            text: String::with_capacity(limit),
            limit,
            truncated: false,
        }
    }

    pub(crate) fn display(value: impl fmt::Display, limit: usize) -> String {
        let mut diagnostic = Self::new(limit);
        let _ = write!(diagnostic, "{value}");
        diagnostic.text
    }
}

impl Write for Diagnostic {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let mut end = value.len().min(self.limit - self.text.len());
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        self.text.push_str(&value[..end]);
        if end < value.len() {
            self.truncated = true;
            return Err(fmt::Error);
        }
        Ok(())
    }
}
