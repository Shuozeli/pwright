//! Current DOM owner lookup without publishing a JavaScript-visible Handle.

use std::sync::Arc;

use pwright_backend_cdp_evaluation::CdpEvaluationService;
use pwright_cap_handle_evaluation::{
    EvaluateHandleParams, HandleEvaluationErrorDetails, HandleEvaluationValue,
};
use pwright_cdp::{CdpClient, connection::CdpError};
use pwright_core::{FailureKind, OperationError};
use serde_json::Value;

use crate::handle_evaluation::acquisition::{Diagnostic, bounded_protocol_error};
use crate::{
    CdpEvaluatedHandle, CdpHandleEvaluationRequest, CdpHandleEvaluationService, CdpHandleKind,
};

// Match the pinned Chromium delegate: a Document is its own owner; other nodes
// use their CURRENT ownerDocument. In particular, isConnected is not a test for
// owning a Frame, and the Handle's original Frame is not the answer.
const DOCUMENT_ELEMENT_SOURCE: &str = r#"node => {
  const doc = node;
  if (doc.documentElement && doc.documentElement.ownerDocument === doc)
    return doc.documentElement;
  return node.ownerDocument ? node.ownerDocument.documentElement : null;
}"#;

type Error = OperationError<HandleEvaluationErrorDetails>;

/// Reads an ElementHandle's current DOM owner using its exact retained receiver.
///
/// The composing owner must retain the source Handle, this entire future and
/// admission for two transient native owners through settlement (one successful
/// document-element result, or a result plus exception on acquisition failure).
/// The evaluation lease covers backend state, not RemoteHandle-class admission.
/// A retained Rust descriptor is not a native-release pin: concurrent explicit
/// source disposal can fail acquisition. Only a successfully acquired distinct
/// document-element wrapper has independent native ownership.
/// Dropping a started future is not cleanup; callers must provide a continuation.
#[derive(Clone)]
pub struct CdpHandleOwnerFrameService {
    evaluation: CdpHandleEvaluationService,
    client: Arc<dyn CdpClient>,
}

impl CdpHandleOwnerFrameService {
    pub fn new(
        evaluation: CdpEvaluationService,
        max_native_id_bytes: usize,
        max_preview_bytes: usize,
    ) -> Self {
        let client = evaluation.shared_client();
        Self {
            evaluation: CdpHandleEvaluationService::from_evaluation(
                evaluation,
                max_native_id_bytes,
                max_preview_bytes,
            ),
            client,
        }
    }

    pub async fn owner_frame(&self, target: &CdpEvaluatedHandle) -> Result<Option<String>, Error> {
        let limits = self.evaluation.limits();
        let error_bytes = limits.evaluation().error_bytes;
        if target.kind() != CdpHandleKind::Element
            || !target.belongs_to_client(&self.client)
            || self.evaluation.unique_context_id().is_none()
            || target
                .object_id()
                .is_none_or(|id| id.is_empty() || id.len() > limits.native_id_bytes())
        {
            return Err(bounded_protocol_error(
                "ownerFrame requires an ElementHandle in the exact bound CDP world",
                error_bytes,
            ));
        }
        // Bound the extra native-ID/preview copies independently of raw CDP
        // response storage. The raw response remains under CDP's own allowance;
        // it is borrowed below, never cloned or recursively materialized here.
        let retained = u64::try_from(limits.native_id_bytes())
            .unwrap_or(u64::MAX)
            // Includes the acquisition-only protected receiver ID copy.
            .saturating_mul(5)
            .saturating_add(
                u64::try_from(limits.preview_bytes())
                    .unwrap_or(u64::MAX)
                    .saturating_mul(2),
            )
            .saturating_add(4096);
        if retained > limits.evaluation().maximum_retained_bytes_per_operation() {
            let message =
                Diagnostic::display("ownerFrame staging allowance is exhausted", error_bytes);
            return Err(OperationError::new(
                FailureKind::ResourceExhausted,
                message,
                None,
            ));
        }

        let request = CdpHandleEvaluationRequest::new_on_native_handle(
            EvaluateHandleParams::function(
                DOCUMENT_ELEMENT_SOURCE,
                HandleEvaluationValue::Undefined,
            ),
            target.clone(),
        );
        // One admission, held through the describe read AND terminal native
        // release. Acquisition failures already compensate both raw native IDs.
        let (document_element, _lease) = self
            .evaluation
            .call_retaining_lease_protecting_receiver(request)
            .await?;
        let Some(object_id) = document_element.object_id() else {
            return Ok(None);
        };
        let result = match self
            .client
            .dom_describe_node_by_object_id_raw(object_id)
            .await
        {
            Ok(response) => parse_frame_id(&response, limits.native_id_bytes(), error_bytes),
            Err(error) => Err(backend_error(error, error_bytes)),
        };
        // Never treat TargetClosed as evidence that this release succeeded.
        // Session retirement can still conservatively produce Cleanup.
        match self
            .client
            .runtime_release_object_to_settlement(object_id)
            .await
        {
            Ok(()) => result,
            Err(error) => {
                let primary = match result {
                    Ok(_) => Diagnostic::display("ownerFrame lookup completed", error_bytes),
                    Err(error) => Diagnostic::display(error.message(), error_bytes),
                };
                Err(cleanup_error(&primary, error, error_bytes))
            }
        }
    }
}

fn parse_frame_id(
    response: &Value,
    native_id_bytes: usize,
    error_bytes: usize,
) -> Result<Option<String>, Error> {
    let Some(node) = response.get("node").filter(|node| node.is_object()) else {
        return Err(bounded_protocol_error(
            "DOM.describeNode has no node object",
            error_bytes,
        ));
    };
    let Some(frame_id) = node.get("frameId").and_then(Value::as_str) else {
        return Ok(None);
    };
    if frame_id.len() > native_id_bytes {
        return Err(bounded_protocol_error(
            "DOM owner frameId exceeds the native identity limit",
            error_bytes,
        ));
    }
    Ok((!frame_id.is_empty()).then(|| frame_id.to_owned()))
}

fn backend_error(error: CdpError, error_bytes: usize) -> Error {
    let kind = pwright_backend_cdp_common::classify_cdp_failure(&error);
    let message = Diagnostic::display(error, error_bytes);
    OperationError::new(
        kind,
        message.clone(),
        Some(HandleEvaluationErrorDetails::Backend(message)),
    )
}

fn cleanup_error(primary: &str, cleanup: impl std::fmt::Display, error_bytes: usize) -> Error {
    let primary = Diagnostic::display(primary, error_bytes);
    let cleanup = Diagnostic::display(cleanup, error_bytes);
    let message = Diagnostic::display(
        format_args!("{primary}; cleanup failed: {cleanup}"),
        error_bytes,
    );
    OperationError::new(
        FailureKind::Cleanup,
        message,
        Some(HandleEvaluationErrorDetails::Cleanup { primary, cleanup }),
    )
}

#[cfg(test)]
mod tests;
