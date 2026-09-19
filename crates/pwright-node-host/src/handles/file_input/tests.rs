use super::RetainedUploads;
use crate::HostError;
use pwright_backend_cdp_file_input::{CdpFilePayloadService, PayloadProfile};
use pwright_composition_chromium_file_input::{FilePayloadOperationService, UploadProfile};
use pwright_composition_chromium_locator_resolution::RetainedElementRequest;
use pwright_core::{
    FailureKind, OperationError, ProductionResourceLedger, ResourceClass, ResourceLedger,
    ResourceLimits, ResourceRequirements, ResourceScope,
};
use pwright_fake::FakeCdpClient;
use pwright_file_input_model::FileInputLimits;
use pwright_file_input_source::{Content, PayloadSource, PreparedPayloads};
use pwright_runtime_tokio::TokioCleanupExecutor;
use serde_json::json;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
    time::Duration,
};
use tokio::sync::Notify;

const GROUP: &str = "pwright.retained.1";
const SOURCE_SCOPE: ResourceScope = ResourceScope::extension(169, 23);

struct Fixture {
    retained: RetainedUploads,
    upload: FilePayloadOperationService<TokioCleanupExecutor>,
    upload_cleanup: TokioCleanupExecutor,
    resources: ProductionResourceLedger,
    source: ResourceLedger,
    client: Arc<FakeCdpClient>,
    state_bytes: u64,
}

impl Fixture {
    fn new() -> Self {
        // Small, explicit test-local profiles exercise the actual host runner
        // without widening the visibility of production file-input profiles.
        let payload = PayloadProfile::new(ResourceScope::extension(169, 21), 1, 4, 4096, 1024)
            .unwrap()
            .with_large_commands();
        let upload = UploadProfile::new(ResourceScope::extension(169, 22), 1, 16 * 1024).unwrap();
        let source_requirements = ResourceRequirements::none()
            .with_class(ResourceClass::LiveObject, 1, 1)
            .with_class(ResourceClass::ArtifactBytes, 64, 64)
            .with_class(ResourceClass::BackendStateBytes, 8192, 8192)
            .with_metadata(1, 1);
        let requirements = RetainedUploads::requirements()
            .unwrap()
            .combine_concurrent(payload.requirements().unwrap())
            .combine_concurrent(upload.requirements())
            .combine_concurrent(source_requirements);
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            limits = limits.with_class(class, required.global, required.per_scope);
        }
        let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
        let retained = RetainedUploads::new(&resources).unwrap();
        let backend = Arc::new(CdpFilePayloadService::new(payload, &resources).unwrap());
        let upload_cleanup = TokioCleanupExecutor::new(1);
        let upload =
            FilePayloadOperationService::new(upload, &resources, backend, upload_cleanup.clone())
                .unwrap();
        let source = resources.claim(source_requirements).unwrap();
        let state_bytes = resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current;
        let client = Arc::new(FakeCdpClient::from_html("<input type=file>"));
        client.set_object_backend_node("source-element", 73);
        client.set_adopted_node_response(
            73,
            19,
            json!({"object":{"type":"object","subtype":"node","objectId":"adopted-element"}}),
        );
        client.set_evaluate_responses([
            json!({"result":{"type":"boolean","value":true}}),
            json!({"result":{"value":{"status":"applied","target":{
                "input":true,"multiple":false,"directory":false
            }}}}),
        ]);
        Self {
            retained,
            upload,
            upload_cleanup,
            resources,
            source,
            client,
            state_bytes,
        }
    }

    fn request(&self) -> RetainedElementRequest {
        RetainedElementRequest {
            source_client: self.client.clone(),
            source_object_id: "source-element".into(),
            client: self.client.clone(),
            utility_world: "exact-utility-world".into(),
            utility_execution_context_id: 19,
            destruction: None,
        }
    }

    fn payloads(&self) -> PreparedPayloads {
        PreparedPayloads::materialize(
            &self.source,
            SOURCE_SCOPE,
            FileInputLimits::new(4, 512, 128, 128, 512).unwrap(),
            &mut [PayloadSource {
                name: "test.txt",
                mime_type: "text/plain",
                last_modified_ms: Some(123.0),
                content: Content::Bytes(&[0, 255, 1]),
            }],
        )
        .unwrap()
    }

    fn mutations(&self) -> usize {
        self.client
            .calls()
            .iter()
            .filter(|(method, args)| {
                method == "Runtime.callFunctionOn"
                    && args[0]["arguments"]
                        .as_array()
                        .is_some_and(|args| args.len() == 2)
            })
            .count()
    }

    fn current(&self, class: ResourceClass) -> u64 {
        self.resources.metrics().unwrap().class(class).current
    }

    async fn shutdown_clean(&self) {
        let report = self.upload_cleanup.shutdown().await;
        assert_eq!(report.metrics.failed, 0);
        assert_eq!(report.metrics.timed_out, 0);
        assert!(self.upload.is_cleanup_healthy());
        self.retained.shutdown().await.unwrap();
    }

    fn assert_group_only_cleanup(&self) {
        let releases: Vec<_> = self
            .client
            .calls()
            .into_iter()
            .filter(|(method, _)| method.starts_with("Runtime.release"))
            .collect();
        assert_eq!(
            releases,
            [("Runtime.releaseObjectGroup".into(), vec![json!(GROUP)])]
        );
    }

    fn assert_drained(self) {
        assert_eq!(self.upload.active_operations(), 0);
        for executor in [&self.upload_cleanup, &self.retained.cleanup] {
            assert_eq!(executor.metrics().reserved, 0);
            assert_eq!(executor.metrics().in_flight, 0);
        }
        // Installed upload and the separately budgeted fixed invocation
        // registry retain one state lease each until installation Drop.
        // The opt-in marker installation additionally owns fixed cache,
        // backend, persistent-state pool and call-pool leases. Selected target
        // prevalidation adds its independently budgeted retained owner pool.
        let marker_leases = if cfg!(feature = "retained-target-marking") {
            4
        } else {
            0
        };
        let prevalidation_leases = if cfg!(feature = "retained-target-prevalidation") {
            1
        } else {
            0
        };
        let mutation_leases = if cfg!(feature = "retained-serialized-payload-mutation") {
            1
        } else {
            0
        };
        assert_eq!(
            self.resources.metrics().unwrap().active_leases,
            2 + marker_leases + prevalidation_leases + mutation_leases
        );
        for class in ResourceClass::ALL {
            assert_eq!(
                self.current(class),
                if class == ResourceClass::BackendStateBytes {
                    self.state_bytes
                } else {
                    0
                },
                "unexpected retained {class:?}"
            );
        }
        assert_eq!(
            self.resources
                .metrics()
                .unwrap()
                .class(ResourceClass::SelectorBytes)
                .peak,
            0
        );
        let resources = self.resources.clone();
        drop(self);
        let metrics = resources.metrics().unwrap();
        assert_eq!(metrics.active_leases, 0);
        for class in ResourceClass::ALL {
            assert_eq!(metrics.class(class).current, 0, "unreturned {class:?}");
        }
    }
}

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

async fn wait_for_release(client: &FakeCdpClient) {
    // Bounded scheduler turns, not a wall-clock race or a browser delay.
    for _ in 0..64 {
        if client
            .method_names()
            .iter()
            .any(|method| method == "Runtime.releaseObjectGroup")
        {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("admitted continuation did not reach group release");
}

fn retired_source() -> OperationError<HostError> {
    OperationError::new(FailureKind::Backend, "source retired before upload", None)
}

#[tokio::test]
async fn adopted_upload_uses_exact_native_target_and_releases_only_its_group() {
    // Arrange
    let fixture = Fixture::new();
    let route = Arc::new(());
    let observed_route = Arc::downgrade(&route);
    let input = Arc::new(());
    let observed_input = Arc::downgrade(&input);

    // Act
    let result = fixture
        .retained
        .run(
            &fixture.upload,
            fixture.request(),
            (fixture.payloads(), input),
            route,
            || Ok(()),
        )
        .await;

    // Assert
    result.unwrap();
    assert!(observed_route.upgrade().is_none());
    assert!(observed_input.upgrade().is_none());
    let calls = fixture.client.calls();
    assert_eq!(
        fixture.client.method_names(),
        [
            "DOM.describeNode",
            "DOM.resolveNode",
            "Runtime.callFunctionOn",
            "Runtime.callFunctionOn",
            "Runtime.releaseObjectGroup"
        ]
    );
    assert_eq!(calls[0].1, [json!({"objectId":"source-element"})]);
    assert_eq!(
        calls[1].1,
        [json!({
            "backendNodeId":73,"executionContextId":19,"objectGroup":GROUP
        })]
    );
    assert_eq!(
        calls[2].1,
        [json!({
            "uniqueContextId":"exact-utility-world",
            "functionDeclaration":"function(node) { return true; }",
            "arguments":[{"objectId":"adopted-element"}],
            "objectGroup":GROUP,"returnByValue":true,"awaitPromise":true
        })]
    );
    let mutation = &calls[3].1[0];
    assert_eq!(mutation["uniqueContextId"], "exact-utility-world");
    assert_eq!(mutation["objectGroup"], GROUP);
    assert_eq!(mutation["returnByValue"], true);
    assert_eq!(mutation["awaitPromise"], true);
    assert_eq!(
        mutation["arguments"],
        json!([
            {"objectId":"adopted-element"},
            {"value":[{"name":"test.txt","mimeType":"text/plain","buffer":"AP8B","lastModifiedMs":123.0}]}
        ])
    );
    assert!(
        mutation["functionDeclaration"]
            .as_str()
            .unwrap()
            .contains("DataTransfer")
    );
    assert_eq!(mutation.as_object().unwrap().len(), 6);
    assert_eq!(fixture.mutations(), 1);
    fixture.assert_group_only_cleanup();
    fixture.shutdown_clean().await;
    fixture.assert_drained();
}

#[tokio::test]
async fn dropping_an_unpolled_run_releases_local_owners_without_native_io() {
    // Arrange
    let fixture = Fixture::new();
    let route = Arc::new(());
    let observed = Arc::downgrade(&route);
    let future = fixture.retained.run(
        &fixture.upload,
        fixture.request(),
        (fixture.payloads(), ()),
        route,
        || Ok(()),
    );

    // Act
    drop(future);

    // Assert
    assert!(observed.upgrade().is_none());
    assert!(fixture.client.calls().is_empty());
    fixture.shutdown_clean().await;
    fixture.assert_drained();
}

#[tokio::test]
async fn cancellation_during_adoption_cleans_up_without_ever_submitting_upload() {
    // Arrange
    let fixture = Fixture::new();
    let adoption = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    fixture.client.set_adoption_gate(Some(adoption.clone()));
    fixture
        .client
        .set_release_object_group_gate(Some(release.clone()));
    let route = Arc::new(());
    let observed_route = Arc::downgrade(&route);
    let input = Arc::new(());
    let observed_input = Arc::downgrade(&input);
    let revalidated = AtomicBool::new(false);
    let mut future = Box::pin(fixture.retained.run(
        &fixture.upload,
        fixture.request(),
        (fixture.payloads(), input),
        route,
        || {
            revalidated.store(true, Ordering::Release);
            Ok(())
        },
    ));
    assert!(poll_now(future.as_mut()).is_pending());

    // Act
    drop(future);
    adoption.notify_one();
    wait_for_release(&fixture.client).await;

    // Assert: source bytes need not survive an upload that was never started,
    // but the native route remains owned until terminal group-release ACK.
    assert!(!revalidated.load(Ordering::Acquire));
    assert!(observed_input.upgrade().is_none());
    assert!(observed_route.upgrade().is_some());
    assert_eq!(fixture.current(ResourceClass::RemoteHandle), 2);
    assert_eq!(fixture.current(ResourceClass::ArtifactBytes), 0);
    assert_eq!(fixture.mutations(), 0);
    assert_eq!(fixture.upload.active_operations(), 0);
    release.notify_one();
    fixture.shutdown_clean().await;
    assert!(observed_route.upgrade().is_none());
    fixture.assert_group_only_cleanup();
    fixture.assert_drained();
}

#[tokio::test]
async fn cancellation_after_mutation_keeps_input_and_route_until_group_release() {
    // Arrange
    let fixture = Fixture::new();
    let mutation = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    fixture
        .client
        .set_release_object_group_gate(Some(release.clone()));
    let route = Arc::new(());
    let observed_route = Arc::downgrade(&route);
    let input = Arc::new(());
    let observed_input = Arc::downgrade(&input);
    let mut future = Box::pin(fixture.retained.run(
        &fixture.upload,
        fixture.request(),
        (fixture.payloads(), input),
        route,
        || {
            // Install this gate only after the unique-world probe has passed.
            fixture.client.set_evaluate_gate(Some(mutation.clone()));
            Ok(())
        },
    ));
    assert!(poll_now(future.as_mut()).is_pending());
    assert_eq!(fixture.mutations(), 1);

    // Act
    drop(future);

    // Assert
    assert!(observed_route.upgrade().is_some());
    assert!(observed_input.upgrade().is_some());
    assert_eq!(fixture.current(ResourceClass::RemoteHandle), 2);
    assert_eq!(fixture.current(ResourceClass::ActiveCommand), 1);
    assert_eq!(fixture.current(ResourceClass::ArtifactBytes), 3);
    assert_eq!(fixture.upload.active_operations(), 1);
    mutation.notify_one();
    wait_for_release(&fixture.client).await;
    assert!(observed_route.upgrade().is_some());
    assert!(observed_input.upgrade().is_some());
    assert_eq!(fixture.current(ResourceClass::ArtifactBytes), 3);
    release.notify_one();
    fixture.shutdown_clean().await;
    assert!(observed_route.upgrade().is_none());
    assert!(observed_input.upgrade().is_none());
    assert_eq!(fixture.mutations(), 1);
    fixture.assert_group_only_cleanup();
    fixture.assert_drained();
}

#[tokio::test]
async fn failed_revalidation_releases_adopted_group_without_mutation() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let error = fixture
        .retained
        .run(
            &fixture.upload,
            fixture.request(),
            (fixture.payloads(), ()),
            Arc::new(()),
            || Err(retired_source()),
        )
        .await
        .unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert_eq!(error.message(), "source retired before upload");
    assert_eq!(fixture.mutations(), 0);
    fixture.assert_group_only_cleanup();
    fixture.shutdown_clean().await;
    fixture.assert_drained();
}

#[tokio::test]
async fn revalidation_and_release_failure_preserves_cleanup_error_and_unhealthy_shutdown() {
    // Arrange
    let fixture = Fixture::new();
    fixture
        .client
        .set_release_object_error(Some("group release failed".into()));

    // Act
    let error = fixture
        .retained
        .run(
            &fixture.upload,
            fixture.request(),
            (fixture.payloads(), ()),
            Arc::new(()),
            || Err(retired_source()),
        )
        .await
        .unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    assert_eq!(
        error.message(),
        "retained upload revalidation and group cleanup failed"
    );
    let details = error.details().unwrap().to_string();
    assert!(details.contains("source retired before upload"));
    assert!(details.contains("group release failed"));
    assert_eq!(fixture.mutations(), 0);
    fixture.assert_group_only_cleanup();
    assert!(fixture.retained.shutdown().await.is_err());
    assert_eq!(fixture.upload_cleanup.shutdown().await.metrics.failed, 0);
    fixture.assert_drained();
}

#[tokio::test]
async fn malformed_adoption_is_group_compensated_and_cleanup_failure_is_not_hidden() {
    for fail_cleanup in [false, true] {
        // Arrange
        let fixture = Fixture::new();
        fixture.client.set_adopted_node_response(
            73,
            19,
            json!({"object":{
                "type":"object","subtype":"node","objectId":null
            }}),
        );
        if fail_cleanup {
            fixture
                .client
                .set_release_object_error(Some("group release failed".into()));
        }

        // Act
        let error = fixture
            .retained
            .run(
                &fixture.upload,
                fixture.request(),
                (fixture.payloads(), ()),
                Arc::new(()),
                || Ok(()),
            )
            .await
            .unwrap_err();

        // Assert
        assert_eq!(
            error.kind(),
            if fail_cleanup {
                FailureKind::Cleanup
            } else {
                FailureKind::Backend
            }
        );
        assert_eq!(
            fixture.client.method_names(),
            [
                "DOM.describeNode",
                "DOM.resolveNode",
                "Runtime.releaseObjectGroup"
            ]
        );
        assert_eq!(fixture.mutations(), 0);
        fixture.assert_group_only_cleanup();
        assert_eq!(fixture.retained.shutdown().await.is_err(), fail_cleanup);
        assert_eq!(fixture.upload_cleanup.shutdown().await.metrics.failed, 0);
        fixture.assert_drained();
    }
}

#[tokio::test(start_paused = true)]
async fn abandoned_adoption_and_terminal_release_stay_owned_beyond_five_seconds() {
    // Arrange
    let fixture = Fixture::new();
    let adoption = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    fixture.client.set_adoption_gate(Some(adoption.clone()));
    fixture
        .client
        .set_release_object_group_gate(Some(release.clone()));
    let route = Arc::new(());
    let observed = Arc::downgrade(&route);
    let mut future = Box::pin(fixture.retained.run(
        &fixture.upload,
        fixture.request(),
        (fixture.payloads(), ()),
        route,
        || Ok(()),
    ));
    assert!(poll_now(future.as_mut()).is_pending());

    // Act
    drop(future);
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(6)).await;

    // Assert: the host's ordinary five-second cleanup policy must not truncate
    // either retained acquisition or its later terminal group release.
    assert_eq!(fixture.retained.cleanup.job_timeout(), None);
    assert_eq!(fixture.retained.cleanup.metrics().in_flight, 1);
    assert_eq!(fixture.retained.cleanup.metrics().timed_out, 0);
    assert!(observed.upgrade().is_some());
    assert_eq!(fixture.current(ResourceClass::RemoteHandle), 2);
    adoption.notify_one();
    wait_for_release(&fixture.client).await;
    let mut shutdown = Box::pin(fixture.retained.shutdown());
    assert!(poll_now(shutdown.as_mut()).is_pending());
    tokio::time::advance(Duration::from_secs(6)).await;
    assert!(poll_now(shutdown.as_mut()).is_pending());
    assert!(observed.upgrade().is_some());
    assert_eq!(fixture.current(ResourceClass::RemoteHandle), 2);
    assert_eq!(fixture.mutations(), 0);
    release.notify_one();
    shutdown.await.unwrap();
    assert!(observed.upgrade().is_none());
    assert_eq!(fixture.retained.cleanup.metrics().completed, 1);
    assert_eq!(fixture.retained.cleanup.metrics().timed_out, 0);
    assert_eq!(fixture.upload_cleanup.shutdown().await.metrics.failed, 0);
    fixture.assert_group_only_cleanup();
    fixture.assert_drained();
}

#[tokio::test]
async fn abandoned_adoption_cleanup_failure_reaches_host_shutdown() {
    // Arrange
    let fixture = Fixture::new();
    let adoption = Arc::new(Notify::new());
    fixture.client.set_adoption_gate(Some(adoption.clone()));
    fixture
        .client
        .set_release_object_error(Some("late group release failed".into()));
    let route = Arc::new(());
    let observed = Arc::downgrade(&route);
    let mut future = Box::pin(fixture.retained.run(
        &fixture.upload,
        fixture.request(),
        (fixture.payloads(), ()),
        route,
        || Ok(()),
    ));
    assert!(poll_now(future.as_mut()).is_pending());

    // Act
    drop(future);
    adoption.notify_one();
    let shutdown = fixture.retained.shutdown().await;

    // Assert
    assert!(shutdown.is_err());
    assert_eq!(fixture.retained.cleanup.metrics().failed, 1);
    assert_eq!(fixture.retained.cleanup.metrics().timed_out, 0);
    assert!(!fixture.retained.raw.is_cleanup_healthy());
    assert!(observed.upgrade().is_none());
    assert_eq!(fixture.mutations(), 0);
    fixture.assert_group_only_cleanup();
    assert_eq!(fixture.upload_cleanup.shutdown().await.metrics.failed, 0);
    fixture.assert_drained();
}
