//! Native invocation semantics, separate from the unchanged portable wire model.

use pwright_cap_handle_evaluation::{
    EvaluateHandleParams, EvaluateParams, EvaluationMode, EvaluationValueLimits,
    HandleEvaluationValue, HandleEvaluationValueError,
};
use pwright_core::ObjectId;

/// An exact native source/mode plus a bounded argument graph. Expression values
/// are ignored by the browser program, but their native references must still
/// be resolved and validated. This type has no wire codec.
#[derive(Clone, Debug)]
pub struct CdpHandleEvaluationProgram {
    projection: EvaluateHandleParams,
    mode: EvaluationMode,
}

impl CdpHandleEvaluationProgram {
    pub const fn from_portable(params: EvaluateHandleParams) -> Self {
        Self {
            mode: params.mode(),
            projection: params,
        }
    }

    pub fn expression(source: String, argument: HandleEvaluationValue) -> Self {
        Self {
            // The projection is used for graph/source validation and admission
            // only, never to choose execution mode or to wrap/rewrite the source.
            projection: EvaluateHandleParams::function(source, argument),
            mode: EvaluationMode::Expression,
        }
    }

    pub fn source(&self) -> &str {
        self.projection.source()
    }
    pub const fn mode(&self) -> EvaluationMode {
        self.mode
    }
    pub const fn argument(&self) -> Option<&HandleEvaluationValue> {
        self.projection.argument()
    }

    pub fn referenced_handles(
        &self,
        limits: EvaluationValueLimits,
    ) -> Result<Vec<ObjectId>, HandleEvaluationValueError> {
        self.projection.referenced_handles(limits)
    }

    pub(crate) const fn portable_projection(&self) -> &EvaluateHandleParams {
        &self.projection
    }

    pub(crate) fn value_projection(
        &self,
        limits: EvaluationValueLimits,
    ) -> Result<EvaluateParams, HandleEvaluationValueError> {
        self.projection.value_projection(limits)
    }
}

impl From<EvaluateHandleParams> for CdpHandleEvaluationProgram {
    fn from(params: EvaluateHandleParams) -> Self {
        Self::from_portable(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> EvaluationValueLimits {
        EvaluationValueLimits::new(16, 4096, 4096, 1024 * 1024, 1024 * 1024).unwrap()
    }

    #[test]
    fn native_expression_preserves_source_and_validates_ignored_handle_arguments() {
        // Arrange
        let source = "var untouched = 42; untouched".to_owned();
        let limits = limits();
        let handle = ObjectId::new(41);

        // Act
        let program = CdpHandleEvaluationProgram::expression(
            source.clone(),
            HandleEvaluationValue::Handle(handle),
        );

        // Assert
        assert_eq!(program.mode(), EvaluationMode::Expression);
        assert_eq!(program.source(), source);
        assert_eq!(program.referenced_handles(limits).unwrap(), vec![handle]);
        let projection = program.value_projection(limits).unwrap();
        assert_eq!(projection.source(), source);
        assert!(
            projection.argument().is_some(),
            "ignored arguments must still consume validation/admission"
        );
    }

    #[test]
    fn portable_conversion_preserves_both_execution_modes_and_argument_shapes() {
        for params in [
            EvaluateHandleParams::expression("42"),
            EvaluateHandleParams::function("value => value", HandleEvaluationValue::Bool(true)),
            EvaluateHandleParams::function_without_argument("() => 42"),
        ] {
            // Arrange
            let expected = params.clone();
            // Act
            let program = CdpHandleEvaluationProgram::from(params);
            // Assert
            assert_eq!(program.mode(), expected.mode());
            assert_eq!(program.portable_projection(), &expected);
        }
    }

    #[test]
    fn expression_argument_graph_validation_cannot_be_skipped() {
        // Arrange
        let program = CdpHandleEvaluationProgram::expression(
            "42".into(),
            HandleEvaluationValue::Ref(std::num::NonZeroU32::new(99).unwrap()),
        );
        // Act
        let result = program.referenced_handles(limits());
        // Assert
        assert!(result.is_err());
    }
}
