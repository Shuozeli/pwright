//! Language conversion retains owners, never merely portable Handle IDs.

use super::HandleEvaluationProgram;
use super::pending::PendingHandle;
use crate::{
    HostError, HostSession,
    evaluation::{self, EvaluationValue},
};
use pwright_cap_handle_evaluation::HandleEvaluationValue;
use std::{iter::Peekable, sync::Arc, vec::IntoIter};

pub struct HandleArgumentSlot {
    index: usize,
    handle: Arc<PendingHandle>,
}
impl HandleArgumentSlot {
    /// Zero-based preorder position in the bounded ordinary value tree. The
    /// referenced position must be an Undefined placeholder from conversion.
    pub fn new(index: usize, handle: Arc<PendingHandle>) -> Self {
        Self { index, handle }
    }
}

pub struct OwnedHandleArgument {
    value: HandleEvaluationValue,
    handles: Vec<Arc<PendingHandle>>,
}
impl OwnedHandleArgument {
    pub fn from_slots(
        value: EvaluationValue,
        slots: Vec<HandleArgumentSlot>,
    ) -> Result<Self, HostError> {
        value
            .validate(evaluation::value_limits())
            .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
        if slots.len() > evaluation::MAX_NODES
            || slots.windows(2).any(|pair| pair[0].index >= pair[1].index)
        {
            return Err(invalid_slots());
        }
        let mut value = HandleEvaluationValue::from(value);
        let mut handles = Vec::with_capacity(slots.len());
        let mut slots = slots.into_iter().peekable();
        bind(&mut value, &mut 0, &mut slots, &mut handles)?;
        if slots.next().is_some() {
            return Err(invalid_slots());
        }
        Ok(Self { value, handles })
    }

    pub fn has_handles(&self) -> bool {
        !self.handles.is_empty()
    }

    pub fn into_value(self) -> Result<EvaluationValue, HostError> {
        EvaluationValue::try_from(self.value)
            .map_err(|error| HostError::InvalidArgument(error.to_string()))
    }

    pub(super) fn into_parts(self) -> (HandleEvaluationValue, Vec<Arc<PendingHandle>>) {
        (self.value, self.handles)
    }
}

fn invalid_slots() -> HostError {
    HostError::InvalidArgument(
        "Handle argument slots must reference ordered unique Undefined placeholders".into(),
    )
}

fn bind(
    value: &mut HandleEvaluationValue,
    next: &mut usize,
    slots: &mut Peekable<IntoIter<HandleArgumentSlot>>,
    handles: &mut Vec<Arc<PendingHandle>>,
) -> Result<(), HostError> {
    let index = *next;
    *next += 1; // bounded by the already validated input tree
    if slots.peek().is_some_and(|slot| slot.index == index) {
        if !matches!(value, HandleEvaluationValue::Undefined) {
            return Err(invalid_slots());
        }
        let slot = slots.next().expect("peeked slot exists");
        *value = HandleEvaluationValue::Handle(slot.handle.initializer().object_id);
        handles.push(slot.handle);
        return Ok(());
    }
    match value {
        HandleEvaluationValue::Array { items, .. } => {
            for item in items {
                bind(item, next, slots, handles)?;
            }
        }
        HandleEvaluationValue::Object { properties, .. } => {
            for property in properties {
                bind(&mut property.value, next, slots, handles)?;
            }
        }
        HandleEvaluationValue::Undefined
        | HandleEvaluationValue::Null
        | HandleEvaluationValue::Bool(_)
        | HandleEvaluationValue::Number(_)
        | HandleEvaluationValue::Nan
        | HandleEvaluationValue::Infinity
        | HandleEvaluationValue::NegativeInfinity
        | HandleEvaluationValue::NegativeZero
        | HandleEvaluationValue::String(_)
        | HandleEvaluationValue::Bigint(_)
        | HandleEvaluationValue::Date(_)
        | HandleEvaluationValue::Url(_)
        | HandleEvaluationValue::Regexp { .. }
        | HandleEvaluationValue::Error { .. }
        | HandleEvaluationValue::TypedArray { .. }
        | HandleEvaluationValue::Ref(_)
        | HandleEvaluationValue::Handle(_) => {}
    }
    Ok(())
}

impl HostSession {
    fn validate_handle_arguments(&self, handles: &[Arc<PendingHandle>]) -> Result<(), HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .handles
                .clone()
        };
        installation
            .owners
            .check_arguments(handles.iter().map(Arc::as_ref))
    }

    pub async fn frame_evaluate_owned(
        &self,
        frame: &str,
        source: String,
        is_function: bool,
        argument: OwnedHandleArgument,
    ) -> Result<EvaluationValue, HostError> {
        let (argument, owners) = argument.into_parts();
        self.validate_handle_arguments(&owners)?;
        self.evaluate_frame_argument(frame, program(source, is_function, argument))
            .await
    }

    pub async fn frame_prepare_handle_owned(
        &self,
        frame: &str,
        source: String,
        is_function: bool,
        argument: OwnedHandleArgument,
    ) -> Result<PendingHandle, HostError> {
        let (argument, owners) = argument.into_parts();
        self.validate_handle_arguments(&owners)?;
        self.frame_prepare_program(frame, program(source, is_function, argument))
            .await
    }

    pub async fn handle_evaluate_owned(
        &self,
        target: &PendingHandle,
        source: String,
        argument: OwnedHandleArgument,
    ) -> Result<EvaluationValue, HostError> {
        self.handle_evaluate_program_owned(target, source, true, argument)
            .await
    }

    pub async fn handle_evaluate_program_owned(
        &self,
        target: &PendingHandle,
        source: String,
        is_function: bool,
        argument: OwnedHandleArgument,
    ) -> Result<EvaluationValue, HostError> {
        let (argument, owners) = argument.into_parts();
        self.validate_handle_arguments(&owners)?;
        self.handle_evaluate_program(target, program(source, is_function, argument))
            .await
    }

    pub async fn handle_prepare_owned(
        &self,
        target: &PendingHandle,
        source: String,
        is_function: bool,
        argument: OwnedHandleArgument,
    ) -> Result<PendingHandle, HostError> {
        let (argument, owners) = argument.into_parts();
        self.validate_handle_arguments(&owners)?;
        self.handle_prepare_program(target, program(source, is_function, argument))
            .await
    }
}

fn program(
    source: String,
    is_function: bool,
    argument: HandleEvaluationValue,
) -> HandleEvaluationProgram {
    if is_function {
        super::EvaluateHandleParams::function(source, argument).into()
    } else {
        HandleEvaluationProgram::expression(source, argument)
    }
}
