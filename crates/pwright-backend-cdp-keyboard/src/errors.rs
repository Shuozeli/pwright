use crate::KEYBOARD_ERROR_MAX_BYTES;
use pwright_backend_cdp_common::classify_cdp_failure;
use pwright_backend_cdp_input::KeyInputError;
use pwright_cap_keyboard::KeyboardErrorDetails;
use pwright_cdp::connection::CdpError;
use pwright_core::{FailureKind, OperationError};
use std::fmt::{self, Write};

pub(crate) fn resource_error(message: &str) -> OperationError<KeyboardErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(KeyboardErrorDetails::Resource(message.to_owned())),
    )
}

struct BoundedMessage(String);
impl Write for BoundedMessage {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if value.len() > KEYBOARD_ERROR_MAX_BYTES - self.0.len() {
            return Err(fmt::Error);
        }
        self.0.push_str(value);
        Ok(())
    }
}

pub(crate) fn native_error(error: CdpError) -> OperationError<KeyboardErrorDetails> {
    let mut message = BoundedMessage(String::with_capacity(KEYBOARD_ERROR_MAX_BYTES));
    if write!(&mut message, "{error}").is_err() {
        return resource_error("Native Keyboard error exceeds the 4096-byte diagnostic limit");
    }
    // Classifying protocol messages can allocate a lower-case copy. Enforce the
    // diagnostic bound first so even that temporary stays within the allowance.
    let kind = classify_cdp_failure(&error);
    OperationError::new(
        kind,
        message.0.clone(),
        Some(KeyboardErrorDetails::Backend(message.0)),
    )
}
pub(crate) fn key_error(error: KeyInputError<'_>) -> OperationError<KeyboardErrorDetails> {
    match error {
        KeyInputError::UnknownKey(key) => {
            // The input layer borrows the actual failing token. Only this
            // bounded service materializes an owned diagnostic.
            let mut message = BoundedMessage(String::with_capacity(KEYBOARD_ERROR_MAX_BYTES));
            if write!(&mut message, "Unknown key: \"{key}\"").is_err() {
                return resource_error("Keyboard key error exceeds the diagnostic limit");
            }
            OperationError::new(
                FailureKind::Backend,
                message.0.clone(),
                Some(KeyboardErrorDetails::InvalidKey(message.0)),
            )
        }
        KeyInputError::Native(error) => native_error(error),
    }
}
