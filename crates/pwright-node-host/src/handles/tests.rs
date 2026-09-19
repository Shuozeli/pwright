use super::*;
use pwright_core::ResourceClass;
use std::{sync::Arc, time::Duration};

mod pending_live;
mod release_live;

fn check(condition: bool, message: &str) -> Result<(), HostError> {
    if condition {
        Ok(())
    } else {
        Err(HostError::backend(message))
    }
}

#[test]
fn selected_profile_declares_shared_handle_routes_and_independent_evaluation() {
    // Arrange
    let factory = crate::profile::attach_factory(Duration::from_secs(5)).unwrap();
    // Act
    let lifecycle = factory.profile().handle_lifecycle().unwrap();
    let evaluation = factory.profile().handle_evaluation().unwrap();
    let topology = factory.profile().topology().limits();
    // Assert
    assert_eq!(lifecycle.routes().max_handles(), MAX_HANDLES);
    assert_eq!(
        lifecycle.routes().max_native_id_bytes(),
        MAX_NATIVE_ID_BYTES
    );
    assert_eq!(lifecycle.routes().max_preview_bytes(), MAX_PREVIEW_BYTES);
    assert_ne!(
        lifecycle.routes().resource_scope(),
        evaluation.backend().resource_scope()
    );
    assert_eq!(evaluation.backend().max_concurrent_operations(), MAX_CALLS);
    assert!(factory.profile().handle_element_adoption().is_none());
    assert_eq!(
        lifecycle
            .routes()
            .requirements()
            .class(ResourceClass::RemoteHandle)
            .global,
        MAX_HANDLES as u64
    );
    assert_eq!(
        lifecycle.routes().maximum_retained_bytes(),
        MAX_HANDLES as u64
            * ((MAX_NATIVE_ID_BYTES + MAX_PREVIEW_BYTES) as u64
                + pwright_composition_chromium_handle_lifecycle::native_release::ROUTE_OWNER_BYTES)
    );
    let maximum_objects = 1 + topology.contexts + topology.pages + topology.frames + MAX_HANDLES;
    assert_eq!(maximum_objects, 161);
    assert!(maximum_objects <= factory.profile().runtime().live_objects);
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn selected_host_connects_and_retires_its_empty_handle_installation() {
    // Arrange
    let endpoint = std::env::var("PWRIGHT_TEST_CDP").unwrap();
    let host = HostSession::connect_over_cdp(&endpoint, 5000)
        .await
        .unwrap();
    assert_eq!(host.handle_metrics().unwrap(), HandleMetrics::default());
    // Act
    let result = host.close().await;
    // Assert
    result.unwrap();
    assert_eq!(host.handle_metrics().unwrap(), HandleMetrics::default());
    assert_eq!(host.metrics().unwrap(), crate::SessionMetrics::default());
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_CDP_WS_URL"]
async fn host_profile_handle_leaf_creates_disposes_and_retires_with_page() {
    use pwright_backend_cdp_page_lifecycle::ImmediatePageClosePoll;
    use pwright_cap_context_creation::BrowserNewContext;
    use pwright_cap_context_lifecycle::BrowserContextClose;
    use pwright_cap_handle_evaluation::{
        EvaluateHandle, EvaluateHandleParams, HandleJsonValue, JsonValueOptions,
    };
    use pwright_cap_handle_lifecycle::HandleDispose;
    use pwright_cap_page_creation::BrowserContextNewPage;
    use pwright_cap_page_lifecycle::PageClose;
    use pwright_core::{CallId, CallMetadata, Command, Service};
    use pwright_runtime_tokio::{TokioCleanupExecutor, TokioDeadlineScheduler};
    // Arrange: the exact selected host profile, existing runtime and leaf APIs.
    // This deliberately tests native assembly, not a not-yet-published Node API.
    let endpoint = pwright_provider_cdp::CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL").unwrap(),
    )
    .unwrap();
    let factory = crate::profile::attach_factory(Duration::from_secs(5)).unwrap();
    let mut attached = factory.attach(endpoint).await.unwrap();
    let cleanup = TokioCleanupExecutor::new(8);
    let runtime = attached
        .new_controlled_page_runtime_with_handle_leaf(TokioDeadlineScheduler, cleanup.clone())
        .unwrap();
    let mut topology = attached
        .topology_event_driver()
        .unwrap()
        .unwrap()
        .with_playwright_context_fallback();
    topology.initialize_snapshot(&runtime, 4096).await.unwrap();
    let context_create = attached
        .browser_new_context_service(runtime.lifecycle_handle(), cleanup.clone())
        .unwrap()
        .unwrap();
    let page_create = attached
        .browser_context_new_page_service(&runtime, cleanup.clone())
        .unwrap()
        .unwrap();
    let context_close = attached
        .browser_context_close_service_for_page_runtime(&runtime, cleanup.clone())
        .unwrap();
    let (page_close, _) = attached
        .page_lifecycle_services(&runtime, cleanup.clone(), Arc::new(ImmediatePageClosePoll))
        .unwrap();
    let (create, dispose) = attached
        .handle_leaf_services(&runtime, cleanup.clone())
        .unwrap()
        .unwrap();
    let context = context_create
        .call(Command::<BrowserNewContext>::new(
            CallId::new(1),
            attached.browser_object_id(),
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let outcome: Result<(), HostError> = async {
        let page = page_create
            .call(Command::<BrowserContextNewPage>::new(
                CallId::new(2),
                context.object_id,
                CallMetadata::default(),
                (),
            ))
            .await
            .unwrap()
            .into_outcome()
            .map_err(|error| HostError::operation(error, 0))?;
        // Match the host publication fence: consume native execution-context
        // announcements into the shared topology before resolving a Handle world.
        topology
            .initialize_snapshot(&runtime, 4096)
            .await
            .map_err(HostError::backend)?;
        let routes = runtime
            .service()
            .handle_leaf_routes()
            .ok_or_else(|| HostError::backend("missing leaf routes"))?;
        let handle = create
            .call(Command::<EvaluateHandle>::new(
                CallId::new(3),
                page.main_frame_id,
                CallMetadata::default(),
                EvaluateHandleParams::expression("({answer:42})"),
            ))
            .await
            .unwrap()
            .into_outcome()
            .map_err(|error| HostError::operation(error, 0))?;
        let value = create
            .call(Command::<HandleJsonValue>::new(
                CallId::new(4),
                handle.object_id,
                CallMetadata::default(),
                JsonValueOptions,
            ))
            .await
            .unwrap()
            .into_outcome()
            .map_err(|error| HostError::operation(error, 0))?;
        let expected = pwright_cap_evaluation::EvaluationProperty::new(
            "answer",
            pwright_cap_evaluation::EvaluationValue::Number(
                pwright_cap_evaluation::EvaluationNumber::new(42.0).unwrap(),
            ),
        );
        check(
            matches!(value.value, pwright_cap_evaluation::EvaluationValue::Object { properties, .. }
            if properties == [expected]),
            "Handle.jsonValue returned unexpected fields",
        )?;
        check(
            routes.metrics().unwrap().current == 1,
            "created handle route missing",
        )?;
        // Act: explicit disposal, then parent retirement for a separate handle.
        dispose
            .call(Command::<HandleDispose>::new(
                CallId::new(5),
                handle.object_id,
                CallMetadata::default(),
                (),
            ))
            .await
            .unwrap()
            .into_outcome()
            .map_err(|error| HostError::operation(error, 0))?;
        check(
            routes.metrics().unwrap().current == 0,
            "disposed handle route remains",
        )?;
        check(
            routes.metrics().unwrap().explicitly_disposed == 1,
            "explicit disposal unrecorded",
        )?;
        let source = "()=>{globalThis.__pwrightHandleCount=(globalThis.__pwrightHandleCount||0)+1;return document.createElement('input')}";
        for index in 0..MAX_HANDLES {
            let element = create.call(Command::<EvaluateHandle>::new(
                CallId::new(6 + index as u64), page.main_frame_id, CallMetadata::default(),
                EvaluateHandleParams::function_without_argument(source),
            )).await.unwrap().into_outcome().map_err(|error| HostError::operation(error, 0))?;
            check(element.object_kind == pwright_browser_model::ELEMENT_HANDLE_OBJECT_KIND.tag,
                "element classified as another Handle kind")?;
        }
        let overflow = create.call(Command::<EvaluateHandle>::new(CallId::new(100),
            page.main_frame_id, CallMetadata::default(), EvaluateHandleParams::function_without_argument(source)))
            .await.unwrap().into_outcome();
        check(overflow.is_err_and(|error| error.kind() == pwright_core::FailureKind::ResourceExhausted),
            "handle limit did not reject overflow")?;
        let counter = runtime.call(Command::<pwright_cap_evaluation::EvaluateValue>::new(CallId::new(101),
            page.main_frame_id, CallMetadata::default(), pwright_cap_evaluation::EvaluateParams::expression("globalThis.__pwrightHandleCount")))
            .await.unwrap().into_outcome().map_err(|error| HostError::operation(error, 0))?;
        check(counter.into_value() == pwright_cap_evaluation::EvaluationValue::Number(
            pwright_cap_evaluation::EvaluationNumber::new(MAX_HANDLES as f64).unwrap()),
            "overflow executed browser code before capacity rejection")?;
        page_close
            .call(Command::<PageClose>::new(
                CallId::new(102),
                page.object_id,
                CallMetadata::default(),
                (),
            ))
            .await
            .unwrap()
            .into_outcome()
            .map_err(|error| HostError::operation(error, 0))?;
        // Assert: one shared route owner, parent retirement and native budget return.
        check(
            routes.metrics().unwrap().current == 0,
            "Page close retained a Handle route",
        )?;
        check(routes.metrics().unwrap().parent_retired == MAX_HANDLES as u64,
            "Page handle retirement count differs from admitted capacity")?;
        check(
            attached
                .resources()
                .metrics()
                .unwrap()
                .class(ResourceClass::RemoteHandle)
                .current
                == 0,
            "remote handle budget not returned",
        )?;
        println!("selected Handle profile: created/read/disposed; {MAX_HANDLES} retained elements; overflow has no browser effect; Page close returned all native route leases");
        Ok(())
    }
    .await;
    let closed = context_close
        .call(Command::<BrowserContextClose>::new(
            CallId::new(103),
            context.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap()
        .into_outcome();
    let report = cleanup.shutdown().await;
    closed.unwrap();
    outcome.unwrap();
    assert_eq!(report.metrics.failed, 0);
    assert_eq!(report.metrics.reserved, 0);
}
