use crate::HostError;
use pwright_cap_handle_evaluation::HandleEvaluationErrorDetails;
use pwright_composition_chromium_locator_evaluation::{
    ElementEvaluationError, preparation::PreparationError,
};
use pwright_composition_chromium_locator_resolution::{
    ResolutionError, ResolutionWaitError, ResolutionWaitStop,
};

pub(super) fn preparation(error: PreparationError) -> HostError {
    match error {
        PreparationError::Waiting(error) => waiting(error),
        PreparationError::Resolution(error) => resolution(error),
        PreparationError::Adoption(error) => evaluation(error),
    }
}
pub(super) fn waiting(error: ResolutionWaitError) -> HostError {
    match error {
        ResolutionWaitError::Resolution(error) => resolution(error),
        ResolutionWaitError::Stopped(ResolutionWaitStop::TargetClosed) => HostError::TargetClosed,
        // The outer phase supplies timeout and transport cancellation; this
        // inner waiter has no independent/restarted public deadline.
        other => HostError::backend(other),
    }
}
fn resolution(error: ResolutionError) -> HostError {
    match error {
        ResolutionError::Admission(message) => HostError::ResourceExhausted(message.into()),
        ResolutionError::Selector(error) => HostError::operation(error, 0),
        other => HostError::backend(other),
    }
}
pub(super) fn evaluation(error: ElementEvaluationError) -> HostError {
    match error {
        ElementEvaluationError::Backend(error) => {
            if let Some(HandleEvaluationErrorDetails::Javascript {
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
                HostError::operation(error, 0)
            }
        }
        ElementEvaluationError::Admission(message) => HostError::ResourceExhausted(message.into()),
        ElementEvaluationError::NativeRetired => HostError::TargetClosed,
        other => HostError::backend(other),
    }
}
