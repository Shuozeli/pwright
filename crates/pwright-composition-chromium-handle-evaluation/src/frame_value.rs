//! Optional Frame by-value evaluation with the existing owned native arguments.

use super::*;

impl ChromiumHandleEvaluationCommandService {
    /// Reuses Frame/world resolution, Handle argument validation/adoption and
    /// the value-returning backend. No temporary result Handle is allocated.
    pub async fn evaluate_value_for_frame(
        &self,
        frame: ObjectId,
        metadata: CallMetadata,
        params: EvaluateHandleParams,
    ) -> Result<HandleValueResult, OperationError<HandleEvaluationErrorDetails>> {
        self.evaluate_program_for_frame(frame, metadata, params.into())
            .await
    }

    /// Native-only source/mode with arguments retained even for expressions.
    pub async fn evaluate_program_for_frame(
        &self,
        frame: ObjectId,
        metadata: CallMetadata,
        params: CdpHandleEvaluationProgram,
    ) -> Result<HandleValueResult, OperationError<HandleEvaluationErrorDetails>> {
        let resolved = self.resolve_frame_creation(frame, metadata, params).await?;
        let outcome = CdpHandleValueEvaluationService::from_evaluation(
            resolved.route.evaluation,
            self.native_id_bytes,
            self.preview_bytes,
        )
        .call(resolved.request)
        .await;
        finish_adopted_operation(outcome, resolved.adopted)
            .await
            .map(HandleValueResult::new)
    }
}
