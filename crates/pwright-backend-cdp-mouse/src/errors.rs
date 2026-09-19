use crate::MOUSE_ERROR_MAX_BYTES;
use pwright_backend_cdp_common::classify_cdp_failure;
use pwright_cap_mouse::MouseErrorDetails;
use pwright_cdp::connection::CdpError;
use pwright_core::{FailureKind, OperationError};
use std::fmt::{self, Write};

pub(crate) fn resource_error(message: &str) -> OperationError<MouseErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(MouseErrorDetails::Resource(message.to_owned())),
    )
}

struct BoundedMessage(String);
impl Write for BoundedMessage {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if value.len() > MOUSE_ERROR_MAX_BYTES - self.0.len() {
            return Err(fmt::Error);
        }
        self.0.push_str(value);
        Ok(())
    }
}

pub(crate) fn native_error(error: CdpError) -> OperationError<MouseErrorDetails> {
    let mut message = BoundedMessage(String::with_capacity(MOUSE_ERROR_MAX_BYTES));
    if write!(&mut message, "{error}").is_err() {
        return resource_error("Native Mouse error exceeds the 4096-byte diagnostic limit");
    }
    // Classifying protocol messages can allocate a lower-case copy. Enforce the
    // diagnostic bound first so even that temporary stays within the allowance.
    let kind = classify_cdp_failure(&error);
    OperationError::new(
        kind,
        message.0.clone(),
        Some(MouseErrorDetails::Backend(message.0)),
    )
}
