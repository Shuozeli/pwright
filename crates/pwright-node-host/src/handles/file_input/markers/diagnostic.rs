//! Cache-owned host diagnostics. The incoming factory error remains caller-
//! owned during handoff; only these fresh bounded strings survive in Flight.
//! This preserves semantic variants/codes and never formats an unbounded error.

use crate::HostError;

pub(super) const BYTES: usize = 1024;

fn copy(value: &str, maximum: usize) -> String {
    let mut end = value.len().min(maximum);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    // In-place truncation would keep arbitrary incoming capacity alive.
    String::from(&value[..end])
}

pub(super) fn bounded(error: HostError) -> HostError {
    match error {
        HostError::InvalidArgument(value) => HostError::InvalidArgument(copy(&value, BYTES)),
        HostError::ResourceExhausted(value) => HostError::ResourceExhausted(copy(&value, BYTES)),
        HostError::Backend(value) => HostError::Backend(copy(&value, BYTES)),
        HostError::UnsupportedOperation(value) => {
            HostError::UnsupportedOperation(copy(&value, BYTES))
        }
        HostError::Javascript {
            name,
            message,
            stack,
        } => HostError::Javascript {
            name: copy(&name, BYTES / 8),
            message: copy(&message, BYTES / 2),
            stack: stack.map(|value| copy(&value, BYTES - BYTES / 8 - BYTES / 2)),
        },
        HostError::ActionTimeout {
            api,
            timeout_ms,
            locator,
            progress,
        } => HostError::ActionTimeout {
            api,
            timeout_ms,
            locator: copy(&locator, BYTES),
            progress,
        },
        error @ (HostError::Timeout(_)
        | HostError::ConnectTimeout { .. }
        | HostError::TargetClosed
        | HostError::FrameDetached) => error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_diagnostics_discard_large_input_capacity_without_changing_variant_or_code() {
        for variant in 0..4 {
            // Arrange
            let mut input = String::with_capacity(BYTES * 64);
            input.push_str("small sentinel");
            let error = match variant {
                0 => HostError::InvalidArgument(input),
                1 => HostError::ResourceExhausted(input),
                2 => HostError::Backend(input),
                _ => HostError::UnsupportedOperation(input),
            };
            let kind = std::mem::discriminant(&error);
            let code = error.code();

            // Act
            let result = bounded(error);

            // Assert
            assert_eq!(std::mem::discriminant(&result), kind);
            assert_eq!(result.code(), code);
            let value = match result {
                HostError::InvalidArgument(value)
                | HostError::ResourceExhausted(value)
                | HostError::Backend(value)
                | HostError::UnsupportedOperation(value) => value,
                _ => panic!("diagnostic kind changed"),
            };
            assert_eq!(value, "small sentinel");
            assert_eq!(value.capacity(), value.len());
            assert!(value.capacity() <= BYTES);
        }
    }

    #[test]
    fn javascript_strings_share_one_fresh_utf8_budget_and_keep_their_source_prefixes() {
        // Arrange
        let source = "界é漢".repeat(BYTES);
        let error = HostError::Javascript {
            name: source.clone(),
            message: source.clone(),
            stack: Some(source.clone()),
        };
        let kind = std::mem::discriminant(&error);
        let code = error.code();

        // Act
        let result = bounded(error);

        // Assert
        assert_eq!(std::mem::discriminant(&result), kind);
        assert_eq!(result.code(), code);
        let HostError::Javascript {
            name,
            message,
            stack: Some(stack),
        } = result
        else {
            panic!("JavaScript fields changed")
        };
        assert!(name.capacity() + message.capacity() + stack.capacity() <= BYTES);
        for value in [name, message, stack] {
            assert!(!value.is_empty());
            assert!(source.starts_with(&value));
            assert!(source.is_char_boundary(value.len()));
            assert_eq!(value.capacity(), value.len());
        }
    }

    #[test]
    fn static_diagnostics_preserve_their_exact_variant_and_code() {
        // Arrange
        let errors = [
            HostError::ActionTimeout {
                api: "Locator.click",
                timeout_ms: 17,
                locator: "button".into(),
                progress: "waiting for element",
            },
            HostError::FrameDetached,
        ];

        for error in errors {
            let kind = std::mem::discriminant(&error);
            let code = error.code();

            // Act
            let result = bounded(error);

            // Assert
            assert_eq!(std::mem::discriminant(&result), kind);
            assert_eq!(result.code(), code);
        }
    }
}
