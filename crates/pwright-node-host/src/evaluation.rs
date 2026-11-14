//! Shared Node-profile bounds and typed value evaluation. No Node-API types.

pub use pwright_cap_evaluation::{
    EvaluateParams, EvaluationNumber, EvaluationProperty, EvaluationValue, EvaluationValueLimits,
    TypedArrayKind,
};
use pwright_cap_evaluation::{EvaluateValue, EvaluationErrorDetails};
use pwright_core::OperationError;

use crate::{HostError, HostSession};

pub const SOURCE_BYTES: usize = 32 * 1024;
pub const GENERATED_SCRIPT_BYTES: usize = 2 * 1024 * 1024;
#[cfg(feature = "large-evaluation-inputs")]
pub const INPUT_GENERATED_SCRIPT_BYTES: usize = 4 * 1024 * 1024;
#[cfg(not(feature = "large-evaluation-inputs"))]
pub const INPUT_GENERATED_SCRIPT_BYTES: usize = GENERATED_SCRIPT_BYTES;
pub const ERROR_BYTES: usize = 4096;
pub const MAX_DEPTH: usize = 64;
pub const MAX_NODES: usize = 4096;
pub const MAX_ENTRIES: usize = 4096;
pub const STRING_BYTES: usize = 256 * 1024;
#[cfg(feature = "large-evaluation-inputs")]
pub const INPUT_STRING_BYTES: usize = 3 * 1024 * 1024;
// The pre-existing large-result artifact could send roughly a megabyte-sized
// argument before its 2 MiB script/wire boundary. Keep that effective path.
#[cfg(all(
    not(feature = "large-evaluation-inputs"),
    feature = "large-evaluation-results"
))]
pub const INPUT_STRING_BYTES: usize = GENERATED_SCRIPT_BYTES;
#[cfg(all(
    not(feature = "large-evaluation-inputs"),
    not(feature = "large-evaluation-results")
))]
pub const INPUT_STRING_BYTES: usize = STRING_BYTES;
#[cfg(feature = "large-evaluation-results")]
pub const RESULT_STRING_BYTES: usize = 128 * 1024 * 1024;
#[cfg(not(feature = "large-evaluation-results"))]
pub const RESULT_STRING_BYTES: usize = STRING_BYTES;
#[cfg(feature = "large-evaluation-results")]
pub const CHUNK_UTF16_UNITS: usize = 256 * 1024;
#[cfg(feature = "large-evaluation-results")]
pub const LARGE_RESULT_FFI_ALLOWANCE: u64 = 400 * 1024 * 1024;
#[cfg(feature = "large-evaluation-results")]
pub const RESULT_WIRE_BYTES: usize = RESULT_STRING_BYTES + 2 * 1024 * 1024;
pub const BINARY_BYTES: usize = 256 * 1024;
pub const BINARY_BACKING_BYTES: usize = 1024 * 1024;
// Conservative simultaneous input/output trees, bounded references/keys,
// source staging, native future and error retention through JS conversion.
#[cfg(feature = "large-evaluation-inputs")]
pub const FFI_ALLOWANCE: u64 = 12 * 1024 * 1024;
#[cfg(not(feature = "large-evaluation-inputs"))]
pub const FFI_ALLOWANCE: u64 = 8 * 1024 * 1024;

pub fn value_limits() -> EvaluationValueLimits {
    EvaluationValueLimits::new(
        MAX_DEPTH,
        MAX_NODES,
        MAX_ENTRIES,
        RESULT_STRING_BYTES,
        BINARY_BYTES,
    )
    .expect("constant Node evaluation limits are valid")
}

pub fn input_value_limits() -> EvaluationValueLimits {
    EvaluationValueLimits::new(
        MAX_DEPTH,
        MAX_NODES,
        MAX_ENTRIES,
        INPUT_STRING_BYTES,
        BINARY_BYTES,
    )
    .expect("constant Node evaluation input limits are valid")
}

pub(crate) fn evaluation_error(
    error: OperationError<EvaluationErrorDetails>,
    timeout: u32,
) -> HostError {
    if let Some(EvaluationErrorDetails::Javascript {
        name,
        message,
        stack,
    }) = error.details()
    {
        HostError::Javascript {
            name: name.clone(),
            message: message.clone(),
            stack: stack.clone(),
        }
    } else {
        HostError::operation(error, timeout)
    }
}

impl HostSession {
    pub async fn frame_evaluate(
        &self,
        frame_id: &str,
        params: EvaluateParams,
    ) -> Result<EvaluationValue, HostError> {
        self.frame_evaluate_with_timeout(frame_id, params, 0).await
    }

    pub(crate) async fn frame_evaluate_with_timeout(
        &self,
        frame_id: &str,
        params: EvaluateParams,
        timeout_ms: u32,
    ) -> Result<EvaluationValue, HostError> {
        if params.source().len() > SOURCE_BYTES {
            return Err(HostError::InvalidArgument(format!(
                "evaluation source exceeds {SOURCE_BYTES} UTF-8 bytes"
            )));
        }
        if let Some(argument) = params.argument() {
            argument
                .validate(input_value_limits())
                .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
        }
        let result = self
            .invoke_with_error::<EvaluateValue>(frame_id, params, timeout_ms, evaluation_error)
            .await;
        crate::completion::publish_command_result(
            result,
            self.is_connected(),
            self.flush_topology(),
        )
        .await
        .map(|result| result.into_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::FailureKind;

    #[test]
    fn javascript_failure_preserves_structured_details_in_the_safe_host() {
        // Arrange
        let failure = OperationError::new(
            FailureKind::Backend,
            "browser error",
            Some(EvaluationErrorDetails::Javascript {
                name: "TypeError".into(),
                message: "bad value".into(),
                stack: Some("browser stack".into()),
            }),
        );
        // Act
        let error = evaluation_error(failure, 0);
        // Assert
        assert_eq!(error.code(), "PWRIGHT_JAVASCRIPT");
        assert!(
            matches!(error, HostError::Javascript { stack: Some(stack), .. } if stack == "browser stack")
        );
    }

    #[test]
    fn typed_value_limits_bound_the_node_boundary_below_its_reservation() {
        // Arrange
        let limits = value_limits();
        let input = input_value_limits();
        // Act
        let tree_bytes = MAX_NODES * std::mem::size_of::<EvaluationValue>()
            + MAX_ENTRIES * std::mem::size_of::<EvaluationProperty>()
            + INPUT_STRING_BYTES
            + BINARY_BYTES;
        #[cfg(feature = "handles")]
        let handle_bytes = MAX_NODES
            * (std::mem::size_of::<crate::handles::HandleArgumentSlot>()
                + std::mem::size_of::<std::sync::Arc<crate::handles::PendingHandle>>())
            + 128
            + crate::handles::CREATION_FUTURE_BYTES;
        #[cfg(not(feature = "handles"))]
        let handle_bytes = 0;
        // Assert
        assert!(
            u64::try_from(
                2 * tree_bytes
                    + SOURCE_BYTES
                    + MAX_ENTRIES * 128
                    + BINARY_BACKING_BYTES
                    + handle_bytes
            )
            .unwrap()
                < FFI_ALLOWANCE
        );
        assert_eq!(limits.max_depth(), MAX_DEPTH);
        assert_eq!(input.max_string_bytes(), INPUT_STRING_BYTES);
    }

    #[test]
    fn input_profile_accepts_its_selected_maximum_and_rejects_one_more_byte() {
        // Arrange
        let at_limit = EvaluationValue::String("0".repeat(INPUT_STRING_BYTES));
        let over_limit = EvaluationValue::String("0".repeat(INPUT_STRING_BYTES + 1));

        // Act
        let accepted = at_limit.validate(input_value_limits());
        let rejected = over_limit.validate(input_value_limits());

        // Assert
        assert!(accepted.is_ok());
        assert!(matches!(
            rejected,
            Err(pwright_cap_evaluation::EvaluationValueError::StringBytesExceeded {
                actual,
                limit
            }) if actual == INPUT_STRING_BYTES + 1 && limit == INPUT_STRING_BYTES
        ));
        assert_eq!(value_limits().max_string_bytes(), RESULT_STRING_BYTES);
    }

    #[cfg(all(
        not(feature = "large-evaluation-inputs"),
        feature = "large-evaluation-results"
    ))]
    #[test]
    fn result_only_profile_preserves_megabyte_argument_admission() {
        // Arrange
        let argument = EvaluationValue::String("0".repeat(1024 * 1024));

        // Act
        let admitted = argument.validate(input_value_limits());

        // Assert
        assert!(admitted.is_ok());
        assert_eq!(INPUT_STRING_BYTES, GENERATED_SCRIPT_BYTES);
        assert_eq!(RESULT_STRING_BYTES, 128 * 1024 * 1024);
    }
}
