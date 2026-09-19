//! Optional native-only Handle value programs, without wire schema changes.

use super::*;

impl ChromiumHandleEvaluationCommandService {
    pub async fn evaluate_program_for_handle(
        &self,
        target: ObjectId,
        metadata: CallMetadata,
        params: CdpHandleEvaluationProgram,
    ) -> Result<HandleValueResult, OperationError<HandleEvaluationErrorDetails>> {
        validate_metadata(metadata, "Handle.evaluate")?;
        validate_live(&self.runtime, target, "Handle")?;
        let ids = params
            .referenced_handles(self.value_limits(target)?)
            .map_err(invalid_argument)?;
        let resolved = self.resolve_handle(target, &ids).await?;
        let ResolvedArguments { arguments, adopted } = resolved.arguments;
        let outcome = CdpHandleValueEvaluationService::from_evaluation(
            resolved.evaluation,
            self.native_id_bytes,
            self.preview_bytes,
        )
        .call(CdpHandleEvaluationRequest::new_program(
            params,
            Some((target, resolved.target)),
            arguments,
        ))
        .await;
        finish_adopted_operation(outcome, adopted)
            .await
            .map(HandleValueResult::new)
    }
}
