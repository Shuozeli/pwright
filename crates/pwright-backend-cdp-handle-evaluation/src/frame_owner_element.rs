//! A Chromium Frame owner's exact DOM node in its parent's main world.
//!
//! The caller admits one persistent Handle route and a cancellation continuation
//! before polling this leaf. The leaf owns neither a Page nor a browser task.

use std::sync::Arc;

use pwright_cap_handle_evaluation::HandleEvaluationErrorDetails;
use pwright_cdp::CdpClient;
use pwright_core::{FailureKind, OperationError};
use serde_json::Value;

use crate::handle_evaluation::acquisition::{Diagnostic, bounded_protocol_error};
use crate::{CdpEvaluatedHandle, CdpHandleEvaluationLimits, CdpHandleKind};

type Error = OperationError<HandleEvaluationErrorDetails>;

/// Resolves a child Frame's owner through the exact parent CDP session. Unlike
/// CSS or page JavaScript, `DOM.getFrameOwner` sees closed shadow-root owners.
#[derive(Clone, Copy)]
pub struct CdpFrameOwnerElementService {
    limits: CdpHandleEvaluationLimits,
}

impl CdpFrameOwnerElementService {
    pub const fn new(limits: CdpHandleEvaluationLimits) -> Self {
        Self { limits }
    }

    /// On success the returned native object belongs to `group`; the caller
    /// must publish or explicitly release it. On every acquisition failure the
    /// whole group is released through settlement, including malformed replies.
    /// A dropped, started future is *not* cleanup: retain it in a pre-admitted
    /// continuation until both resolve and compensation settle.
    pub async fn resolve(
        &self,
        client: Arc<dyn CdpClient>,
        child_frame_id: &str,
        parent_main_execution_context_id: i64,
        group: &str,
    ) -> Result<CdpEvaluatedHandle, Error> {
        let error_bytes = self.limits.evaluation().error_bytes;
        if child_frame_id.is_empty()
            || child_frame_id.len() > self.limits.native_id_bytes()
            || parent_main_execution_context_id <= 0
            || group.is_empty()
            || group.len() > self.limits.native_id_bytes()
        {
            return Err(bounded_protocol_error(
                "frame owner route or object group is invalid",
                error_bytes,
            ));
        }
        let backend_node_id =
            client
                .dom_get_frame_owner(child_frame_id)
                .await
                .map_err(|error| {
                    if error
                        .to_string()
                        .contains("Frame with the given id was not found.")
                    {
                        detached(error_bytes)
                    } else {
                        backend_error(error, error_bytes)
                    }
                })?;
        if backend_node_id <= 0 {
            return Err(bounded_protocol_error(
                "DOM.getFrameOwner returned an invalid backend node ID",
                error_bytes,
            ));
        }
        // This request may have been written before the public waiter was
        // cancelled. The caller's continuation keeps it alive to settlement.
        let response = client
            .dom_resolve_node_in_group_to_settlement(
                backend_node_id,
                parent_main_execution_context_id,
                group,
            )
            .await;
        let result = match response {
            Ok(response) => parse_owner(client.clone(), &response, self.limits),
            Err(error) => Err(backend_error(error, error_bytes)),
        };
        match result {
            Ok(owner) => Ok(owner),
            Err(primary) => {
                match client
                    .runtime_release_object_group_to_settlement(group)
                    .await
                {
                    Ok(()) => Err(primary),
                    Err(cleanup) => Err(cleanup_error(primary.message(), cleanup, error_bytes)),
                }
            }
        }
    }
}

fn parse_owner(
    client: Arc<dyn CdpClient>,
    response: &Value,
    limits: CdpHandleEvaluationLimits,
) -> Result<CdpEvaluatedHandle, Error> {
    let error_bytes = limits.evaluation().error_bytes;
    let remote = response
        .get("object")
        .ok_or_else(|| bounded_protocol_error("DOM.resolveNode omitted its object", error_bytes))?;
    let owner = CdpEvaluatedHandle::from_remote_object(client, remote, limits)?;
    if owner.kind() != CdpHandleKind::Element || owner.object_id().is_none() {
        return Err(bounded_protocol_error(
            "DOM.getFrameOwner did not resolve to an Element",
            error_bytes,
        ));
    }
    Ok(owner)
}

fn detached(error_bytes: usize) -> Error {
    let message = Diagnostic::display("Frame has been detached.", error_bytes);
    OperationError::new(
        FailureKind::TargetClosed,
        message.clone(),
        Some(HandleEvaluationErrorDetails::Backend(message)),
    )
}

fn backend_error(error: pwright_cdp::connection::CdpError, error_bytes: usize) -> Error {
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
        format_args!("{primary}; frame owner cleanup failed: {cleanup}"),
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
