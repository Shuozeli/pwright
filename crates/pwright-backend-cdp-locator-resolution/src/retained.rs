//! Opt-in exact-node adoption into an independently owned utility-world group.

mod diagnostic;

use super::*;
use diagnostic::DIAGNOSTIC_BYTES;
pub(super) use diagnostic::bounded_error;

/// Borrowed source identity, not permission to release the source object.
/// The caller budgets these retained references and keeps the source/route
/// owners alive until this attempt and its cleanup finish. Both target-world
/// IDs must come from the same authoritative snapshot; they are not a lock
/// against subsequent browser navigation.
pub struct RetainedElementRequest {
    pub source_client: Arc<dyn CdpClient>,
    pub source_object_id: Arc<str>,
    pub client: Arc<dyn CdpClient>,
    pub utility_world: Arc<str>,
    pub utility_execution_context_id: i64,
    /// This proof is accepted only after native validation in the exact unique
    /// world. Before that, numeric context reuse could put the group elsewhere.
    pub destruction: Option<Arc<dyn ObjectGroupDestructionProof>>,
}

const VALIDATE_WORLD: &str = "function(node) { return true; }";
// One adopted wrapper and one possible exception from the unique-world probe.
const NATIVE_UNITS: u64 = 2;
// Source/probe IDs and simultaneous typed-parameter/JSON copies, two bounded
// failure diagnostics, plus checked inline future/argument/group overhead.
// The decoded raw envelope and transport's own buffers remain transport-bound.
const CONTROL_BYTES: usize = 4096;
pub(super) const ATTEMPT_BYTES: u64 =
    (8 * NATIVE_ID_BYTES + 2 * DIAGNOSTIC_BYTES + CONTROL_BYTES) as u64;

impl CdpLocatorResolutionService {
    fn prepare_retained(
        &self,
        request: &RetainedElementRequest,
    ) -> Result<(String, ResourceLease, ResourceLease), ResolutionError> {
        if !self.profile.retained_reference {
            return Err(ResolutionError::Setup(
                "retained-reference storage was not selected in the resource profile",
            ));
        }
        if !self.is_cleanup_healthy() {
            return Err(ResolutionError::Admission(
                "a prior native group cleanup failed",
            ));
        }
        if !Arc::ptr_eq(&request.source_client, &request.client) {
            return Err(ResolutionError::Setup(
                "retained adoption cannot cross a CDP target session",
            ));
        }
        if request.utility_execution_context_id <= 0
            || request.utility_world.is_empty()
            || request.utility_world.len() > NATIVE_ID_BYTES
            || request.source_object_id.is_empty()
            || request.source_object_id.len() > NATIVE_ID_BYTES
        {
            return Err(ResolutionError::Setup(
                "retained adoption requires bounded native IDs and a positive utility context",
            ));
        }
        let owner = self.ledger.reserve_many(
            self.profile.scope,
            &[
                ResourceRequest::new(ResourceClass::RemoteHandle, NATIVE_UNITS),
                ResourceRequest::new(ResourceClass::BackendStateBytes, NATIVE_UNITS * OWNER_BYTES),
            ],
        )?;
        let scratch = self.ledger.reserve(
            self.profile.scope,
            ResourceClass::BackendStateBytes,
            ATTEMPT_BYTES,
        )?;
        let id = self
            .ids
            .next_object_id()
            .map_err(|_| ResolutionError::Setup("object-group IDs exhausted"))?;
        Ok((format!("pwright.retained.{}", id.get()), owner, scratch))
    }
}

impl Service<RetainedElementRequest> for CdpLocatorResolutionService {
    type Response = ResolvedElement;
    type Error = ResolutionError;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<ResolvedElement, ResolutionError>> + Send>>;

    fn call(&self, request: RetainedElementRequest) -> Self::Future<'_> {
        let prepared = self.prepare_retained(&request);
        let healthy = self.healthy.clone();
        let future = async move {
            let (group, owner, _scratch) = prepared?;
            if !healthy.load(Ordering::Acquire) {
                return Err(ResolutionError::Admission(
                    "a prior native group cleanup failed",
                ));
            }
            // This read creates no RemoteObject. Cancellation before resolve
            // needs no group compensation and does not poison the service.
            let backend_node = request
                .client
                .dom_describe_node_by_object_id(&request.source_object_id)
                .await
                .map_err(bounded_error)?;
            if backend_node <= 0 {
                return Err(ResolutionError::Protocol("non-positive backend node ID"));
            }
            let mut cleanup = GroupCleanupGuard {
                healthy,
                armed: true,
                // A numeric-only resolve can land in a reused context. Never
                // discharge its cleanup using the expected OLD world's proof.
                destruction: None,
            };
            let acquired = async {
                let response = request
                    .client
                    .dom_resolve_node_in_group_to_settlement(
                        backend_node,
                        request.utility_execution_context_id,
                        &group,
                    )
                    .await
                    .map_err(bounded_error)?;
                let object_id = parse_retained(&response, &request.source_object_id)?;
                drop(response);
                // CDP validates object arguments against this unique context
                // before invoking the constant probe; no DOM mutation or user
                // getter is involved. A mismatched/reused numeric context fails
                // here while the unverified group still has no world proof.
                let checked = request
                    .client
                    .runtime_call_function_by_value_in_group_to_settlement(
                        &request.utility_world,
                        VALIDATE_WORLD,
                        vec![serde_json::json!({ "objectId": object_id.as_ref() })],
                        &group,
                    )
                    .await
                    .map_err(bounded_error)?;
                if checked.get("exceptionDetails").is_some()
                    || checked.pointer("/result/type").and_then(Value::as_str) != Some("boolean")
                    || checked.pointer("/result/value").and_then(Value::as_bool) != Some(true)
                    || checked.pointer("/result/objectId").is_some()
                {
                    return Err(ResolutionError::Protocol(
                        "retained adoption unique-world validation failed",
                    ));
                }
                Ok(object_id)
            }
            .await;
            match acquired {
                Ok(object_id) => {
                    cleanup.destruction = request.destruction;
                    Ok(ResolvedElement {
                        client: request.client,
                        utility_world: request.utility_world,
                        object_id,
                        group,
                        _owner: owner,
                        cleanup,
                        release_mode: GroupReleaseMode::Settlement,
                    })
                }
                Err(primary) => {
                    match release_group(
                        request.client.as_ref(),
                        &group,
                        None,
                        GroupReleaseMode::Settlement,
                    )
                    .await
                    {
                        Ok(()) => {
                            cleanup.armed = false;
                            Err(primary)
                        }
                        Err(source) => Err(ResolutionError::Cleanup {
                            primary: Some(Box::new(primary)),
                            source,
                        }),
                    }
                }
            }
        };
        check_future_storage(&future);
        Box::pin(future)
    }
}

fn check_future_storage<F>(_: &F) {
    const {
        assert!(std::mem::size_of::<F>() + 1024 < CONTROL_BYTES);
    }
}

fn parse_retained(response: &Value, source: &str) -> Result<Box<str>, ResolutionError> {
    let object = response
        .get("object")
        .ok_or(ResolutionError::Protocol("missing adopted RemoteObject"))?;
    if object.get("type").and_then(Value::as_str) != Some("object")
        || object.get("subtype").and_then(Value::as_str) != Some("node")
        || response.get("exceptionDetails").is_some()
    {
        return Err(ResolutionError::Protocol("expected an adopted DOM node"));
    }
    let id = object
        .get("objectId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= NATIVE_ID_BYTES && *id != source)
        .ok_or(ResolutionError::Protocol(
            "adopted native ID is missing, oversized or aliases the borrowed source",
        ))?;
    Ok(id.into())
}

#[cfg(test)]
mod tests;
