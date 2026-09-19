//! Current DOM ownership is distinct from the Handle's creation-world route.

use super::*;

impl ChromiumHandleEvaluationCommandService {
    /// Queries the retained receiver's current owner document. The returned ID
    /// is native and bounded; the embedding host must map it through its current
    /// topology within the source BrowserContext. It is not the creation Frame.
    ///
    /// The caller must retain and poll this entire future through completion,
    /// including on cancellation: it temporarily owns a document-element Handle.
    pub async fn owner_frame_native_id(
        &self,
        target: ObjectId,
    ) -> Result<Option<String>, OperationError<HandleEvaluationErrorDetails>> {
        validate_live(&self.runtime, target, "ElementHandle")?;
        let resolved = self.resolve_handle(target, &[]).await?;
        if resolved.target.kind() != CdpHandleKind::Element {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "ownerFrame requires an ElementHandle",
                None,
            ));
        }
        pwright_backend_cdp_handle_evaluation::CdpHandleOwnerFrameService::new(
            resolved.evaluation,
            self.native_id_bytes,
            self.preview_bytes,
        )
        .owner_frame(&resolved.target)
        .await
    }
}
