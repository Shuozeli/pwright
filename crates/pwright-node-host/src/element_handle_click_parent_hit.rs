//! Bounded parent utility-world iframe-owner hit proof for one-link retained clicks.

use pwright_cap_locator::LocatorErrorDetails;
use pwright_cdp::CdpClient;
use pwright_core::{FailureKind, OperationError};
use pwright_runtime::{CleanupFailure, CleanupReservation, ReservedCleanupExecutor};
use pwright_runtime_tokio::{TokioCleanupExecutor, TokioCleanupReservation};
use serde_json::{Value, json};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};
use tokio::sync::Semaphore;

type HitResult = Result<bool, OperationError<LocatorErrorDetails>>;
type HitOperation = Pin<Box<dyn Future<Output = HitResult> + Send>>;

pub(super) struct ParentHitRoute<'a> {
    pub(super) child_frame_id: &'a str,
    pub(super) execution_context_id: i64,
    pub(super) unique_context_id: &'a str,
}

// Document hit testing retargets an iframe inside an open shadow root to its
// host. Walk only the composed hit path, never search for another iframe.
const PARENT_HIT_SCRIPT: &str = r#"function(owner, x, y) {
  if (!owner || owner.nodeType !== 1 || !owner.isConnected ||
      owner.ownerDocument !== document) return false;
  let root = document;
  for (let depth = 0; depth < 8; ++depth) {
    const hit = root.elementFromPoint(x, y);
    if (hit === owner) return true;
    if (!hit || !hit.shadowRoot) return false;
    root = hit.shadowRoot;
  }
  return false;
}"#;

fn invalid(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(FailureKind::Backend, message, None)
}

fn resource(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(FailureKind::ResourceExhausted, message, None)
}

fn cleanup_error(error: impl std::fmt::Display) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::Cleanup,
        format!("retained click parent-owner group release failed: {error}"),
        None,
    )
}

fn resolved_owner_object_id(response: &Value) -> Result<&str, OperationError<LocatorErrorDetails>> {
    let object = response
        .get("object")
        .ok_or_else(|| invalid("retained click parent owner resolution omitted its object"))?;
    if response.get("exceptionDetails").is_some()
        || object.get("type").and_then(Value::as_str) != Some("object")
        || object.get("subtype").and_then(Value::as_str) != Some("node")
    {
        return Err(invalid("retained click parent owner is not a DOM node"));
    }
    object
        .get("objectId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 1024)
        .ok_or_else(|| invalid("retained click parent owner has an invalid object ID"))
}

// Once DOM.resolveNode can have been written, dropping the public waiter must
// settle that request and release its group, without starting new page JS.
struct ParentHitCheck {
    operation: Option<HitOperation>,
    reservation: Option<TokioCleanupReservation>,
    cancelled: Arc<AtomicBool>,
    started: bool,
}

impl Unpin for ParentHitCheck {}

impl Future for ParentHitCheck {
    type Output = HitResult;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let result = self
            .operation
            .as_mut()
            .expect("parent hit proof polled after completion")
            .as_mut()
            .poll(context);
        if result.is_ready() {
            self.operation = None;
            self.reservation = None;
        }
        result
    }
}

impl Drop for ParentHitCheck {
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        if let (Some(operation), Some(reservation)) =
            (self.operation.take(), self.reservation.take())
        {
            self.cancelled.store(true, Ordering::Release);
            reservation.submit(Box::pin(async move {
                operation.await.map(|_| ()).map_err(|_| {
                    CleanupFailure::new("retained click parent-owner proof or release failed")
                })
            }));
        }
    }
}

pub(super) async fn verify(
    client: Arc<dyn CdpClient>,
    route: ParentHitRoute<'_>,
    x: f64,
    y: f64,
    group: String,
    cleanup: &TokioCleanupExecutor,
    slots: Arc<Semaphore>,
) -> HitResult {
    if route.child_frame_id.is_empty()
        || route.child_frame_id.len() > 1024
        || route.execution_context_id <= 0
        || route.unique_context_id.is_empty()
        || route.unique_context_id.len() > 1024
        || !x.is_finite()
        || !y.is_finite()
        || group.is_empty()
        || group.len() > 96
    {
        return Err(invalid("retained click parent hit route is invalid"));
    }
    let backend_node_id = client
        .dom_get_frame_owner(route.child_frame_id)
        .await
        .map_err(super::route_error)?;
    if backend_node_id <= 0 {
        return Err(invalid(
            "retained click iframe owner has an invalid backend node ID",
        ));
    }
    let slot = slots
        .try_acquire_owned()
        .map_err(|_| resource("retained click parent-owner proof capacity was exhausted"))?;
    let reservation = cleanup.try_reserve().map_err(|error| {
        resource(&format!(
            "retained click native cleanup unavailable: {error}"
        ))
    })?;
    let unique = route.unique_context_id.to_owned();
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancellation = cancelled.clone();
    let operation = async move {
        let _slot = slot;
        let primary = async {
            let resolved = client
                .dom_resolve_node_in_group_to_settlement(
                    backend_node_id,
                    route.execution_context_id,
                    &group,
                )
                .await
                .map_err(super::route_error)?;
            if cancellation.load(Ordering::Acquire) {
                return Ok(false);
            }
            let object_id = resolved_owner_object_id(&resolved)?;
            let response = client
                .runtime_call_function_by_value_in_group_to_settlement(
                    &unique,
                    PARENT_HIT_SCRIPT,
                    vec![
                        json!({"objectId": object_id}),
                        json!({"value": x}),
                        json!({"value": y}),
                    ],
                    &group,
                )
                .await
                .map_err(super::route_error)?;
            response
                .get("result")
                .and_then(|result| result.get("value"))
                .and_then(Value::as_bool)
                .ok_or_else(|| invalid("retained click parent-owner hit proof is not boolean"))
        }
        .await;
        let released = client
            .runtime_release_object_group_to_settlement(&group)
            .await
            .map_err(cleanup_error);
        match (primary, released) {
            (Ok(hit), Ok(())) => Ok(hit),
            (Err(error), Ok(())) => Err(error),
            (_, Err(error)) => Err(error),
        }
    };
    ParentHitCheck {
        operation: Some(Box::pin(operation)),
        reservation: Some(reservation),
        cancelled,
        started: false,
    }
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_fake::FakeCdpClient;
    use std::task::Waker;
    use std::time::Duration;
    use tokio::sync::Notify;

    fn fixture() -> (Arc<FakeCdpClient>, TokioCleanupExecutor, Arc<Semaphore>) {
        let fake = Arc::new(FakeCdpClient::from_html("<iframe></iframe>"));
        fake.set_frame_owner("child-frame", 91);
        fake.set_adopted_node_response(
            91,
            42,
            json!({"object": {"type": "object", "subtype": "node", "objectId": "parent-owner"}}),
        );
        (
            fake,
            TokioCleanupExecutor::new(1),
            Arc::new(Semaphore::new(1)),
        )
    }

    #[tokio::test]
    async fn cross_origin_child_uses_exact_parent_utility_world_owner_identity() {
        // Arrange
        let (fake, cleanup, slots) = fixture();
        fake.set_evaluate_response(json!({"result": {"value": true}}));

        // Act
        let hit = verify(
            fake.clone(),
            ParentHitRoute {
                child_frame_id: "child-frame",
                execution_context_id: 42,
                unique_context_id: "parent-utility-world",
            },
            128.0,
            136.0,
            "pwright.click.1.1".into(),
            &cleanup,
            slots,
        )
        .await
        .unwrap();

        // Assert
        assert!(hit);
        let calls = fake.calls();
        assert_eq!(
            calls.iter().map(|call| call.0.as_str()).collect::<Vec<_>>(),
            [
                "DOM.getFrameOwner",
                "DOM.resolveNode",
                "Runtime.callFunctionOn",
                "Runtime.releaseObjectGroup"
            ]
        );
        assert_eq!(calls[1].1[0]["executionContextId"], 42);
        assert_eq!(calls[1].1[0]["objectGroup"], "pwright.click.1.1");
        assert_eq!(calls[2].1[0]["uniqueContextId"], "parent-utility-world");
        assert_eq!(calls[2].1[0]["arguments"][0]["objectId"], "parent-owner");
        assert_eq!(calls[3].1[0], "pwright.click.1.1");
        assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
    }

    #[tokio::test]
    async fn open_shadow_iframe_walks_bounded_composed_hit_roots() {
        // Arrange
        let (fake, cleanup, slots) = fixture();
        fake.set_evaluate_response(json!({"result": {"value": true}}));

        // Act
        let hit = verify(
            fake.clone(),
            ParentHitRoute {
                child_frame_id: "child-frame",
                execution_context_id: 42,
                unique_context_id: "parent-utility-world",
            },
            128.0,
            136.0,
            "pwright.click.2.1".into(),
            &cleanup,
            slots,
        )
        .await
        .unwrap();

        // Assert
        assert!(hit);
        let call = &fake.calls()[2].1[0];
        let script = call["functionDeclaration"].as_str().unwrap();
        assert!(script.contains("root = hit.shadowRoot"));
        assert!(script.contains("depth < 8"));
        assert!(script.contains("hit === owner"));
        assert_eq!(call["arguments"][0]["objectId"], "parent-owner");
        assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
    }

    #[tokio::test]
    async fn malformed_parent_owner_still_releases_unique_group() {
        // Arrange
        let (fake, cleanup, slots) = fixture();
        fake.set_adopted_node_response(91, 42, json!({"object": {"type": "number", "value": 1}}));

        // Act
        let result = verify(
            fake.clone(),
            ParentHitRoute {
                child_frame_id: "child-frame",
                execution_context_id: 42,
                unique_context_id: "parent-utility-world",
            },
            128.0,
            136.0,
            "pwright.click.3.1".into(),
            &cleanup,
            slots,
        )
        .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(
            fake.method_names(),
            [
                "DOM.getFrameOwner",
                "DOM.resolveNode",
                "Runtime.releaseObjectGroup"
            ]
        );
        assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
    }

    #[tokio::test]
    async fn cancelled_resolve_skips_new_js_and_releases_group() {
        // Arrange
        let (fake, cleanup, slots) = fixture();
        fake.set_evaluate_response(json!({"result": {"value": true}}));
        let gate = Arc::new(Notify::new());
        fake.set_adoption_gate(Some(gate.clone()));
        let mut check = Box::pin(verify(
            fake.clone(),
            ParentHitRoute {
                child_frame_id: "child-frame",
                execution_context_id: 42,
                unique_context_id: "parent-utility-world",
            },
            128.0,
            136.0,
            "pwright.click.4.1".into(),
            &cleanup,
            slots,
        ));

        // Act
        assert!(
            check
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        drop(check);
        gate.notify_one();
        let report = cleanup.shutdown().await;

        // Assert
        assert_eq!(report.metrics.failed, 0);
        assert_eq!(
            fake.method_names(),
            [
                "DOM.getFrameOwner",
                "DOM.resolveNode",
                "Runtime.releaseObjectGroup"
            ]
        );
    }

    #[tokio::test]
    async fn cancelled_started_hit_evaluation_settles_before_group_release() {
        // Arrange
        let (fake, cleanup, slots) = fixture();
        fake.set_evaluate_response(json!({"result": {"value": true}}));
        let gate = Arc::new(Notify::new());
        fake.set_evaluate_gate(Some(gate.clone()));
        let mut check = Box::pin(verify(
            fake.clone(),
            ParentHitRoute {
                child_frame_id: "child-frame",
                execution_context_id: 42,
                unique_context_id: "parent-utility-world",
            },
            128.0,
            136.0,
            "pwright.click.4.2".into(),
            &cleanup,
            slots,
        ));

        // Act
        assert!(
            check
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        drop(check);
        gate.notify_one();
        let report = cleanup.shutdown().await;

        // Assert
        assert_eq!(report.metrics.failed, 0);
        assert_eq!(
            fake.method_names(),
            [
                "DOM.getFrameOwner",
                "DOM.resolveNode",
                "Runtime.callFunctionOn",
                "Runtime.releaseObjectGroup"
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn stalled_transport_cleanup_timeout_is_reported_without_false_release_claim() {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html("<iframe></iframe>"));
        fake.set_frame_owner("child-frame", 91);
        let gate = Arc::new(Notify::new());
        fake.set_adoption_gate(Some(gate));
        let cleanup =
            TokioCleanupExecutor::new_with_job_timeout(1, Duration::from_secs(5)).unwrap();
        let mut check = Box::pin(verify(
            fake.clone(),
            ParentHitRoute {
                child_frame_id: "child-frame",
                execution_context_id: 42,
                unique_context_id: "parent-utility-world",
            },
            128.0,
            136.0,
            "pwright.click.5.1".into(),
            &cleanup,
            Arc::new(Semaphore::new(1)),
        ));
        assert!(
            check
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );

        // Act
        drop(check);
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(5)).await;
        for _ in 0..10 {
            if cleanup.metrics().in_flight == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
        let report = cleanup.shutdown().await;

        // Assert
        assert_eq!(report.metrics.timed_out, 1);
        assert_eq!(report.metrics.in_flight, 0);
        assert_eq!(
            fake.method_names(),
            ["DOM.getFrameOwner", "DOM.resolveNode"]
        );
    }
}
