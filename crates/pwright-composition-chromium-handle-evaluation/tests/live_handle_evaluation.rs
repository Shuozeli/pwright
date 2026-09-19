use std::sync::Arc;
use std::time::Duration;

use pwright_browser_model::{
    BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG,
    FRAME_OBJECT_KIND, PAGE_OBJECT_KIND,
};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cap_handle_evaluation::{
    EvaluateHandle, EvaluateHandleParams, EvaluationValue, EvaluationValueLimits,
    GetPropertyParams, HandleEvaluate, HandleEvaluateParams, HandleEvaluationValue,
    HandleGetProperty,
};
use pwright_cap_handle_lifecycle::HandleDispose;
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpClient, CdpSession};
use pwright_codec_control::ControlCodec;
use pwright_composition_chromium_handle_evaluation::{
    CdpElementHandleAdoptionBatchRequest, CdpElementHandleAdoptionBatchService,
    CdpElementHandleAdoptionProductionProfile, CdpElementHandleAdoptionRequest,
    CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpEvaluationService,
    CdpHandleEvaluationLimits, CdpHandleEvaluationRequest, CdpHandleEvaluationService,
    CdpHandlePropertyLimits, ChromiumHandleEvaluationBackendProfile,
    ChromiumHandleEvaluationBindings, ChromiumHandleEvaluationInstallation,
    ChromiumHandleEvaluationProfile, ChromiumHandleLifecycleProfile, ChromiumHandleRouteProfile,
    ContinuableElementHandleAdoptionService, FrameLimits, HandleEvaluationCodecLimits,
    HandleLifecycleCodecLimits,
};
use pwright_core::{
    AtomicObjectIdSource, CallId, CallMetadata, Command, CommandResult, ControlMessage, ObjectId,
    ProductionResourceLedger, ProtocolVersion, ResourceClass, ResourceLimits, ResourceRequirements,
    ResourceScope, Service,
};
use pwright_plugin_ordered_handle_delivery::{
    OrderedHandleDisposeService, OrderedHandleEvaluationHandler,
};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use pwright_runtime::{
    CleanupQueue, ControlledDispatcher, NoDeadlineScheduler, ObjectTopologyMode, RuntimeLimits,
};
use pwright_runtime_control::{OrderedRuntimeControl, RuntimeControlDriver};
use pwright_server_wire::{ConcurrentServerFrameHandler, ServerFrameDisposition};
use pwright_transport::{FrameRoute, FrameTransport, VersionPolicy, WireProtocol};
use pwright_transport_memory::MemoryFrameTransport;

fn exact_limits(requirements: ResourceRequirements) -> ResourceLimits {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let required = requirements.class(class);
        limits = limits.with_class(class, required.global, required.per_scope);
    }
    limits
}

fn control_protocol() -> WireProtocol<ControlCodec> {
    WireProtocol::new(
        ControlCodec,
        VersionPolicy::exact(ProtocolVersion::new(1, 0)),
        ProtocolVersion::new(1, 0),
        FrameLimits::new(128, 0, 0, 128),
    )
    .expect("control protocol should build")
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn portable_property_preserves_promise_and_thenable_identity_after_target_disposal() {
    // Arrange one test-owned about:blank Context in the externally owned Chrome.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .unwrap();
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .unwrap();
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let context_id = browser
        .target_create_browser_context(None, None)
        .await
        .unwrap();
    let exercise_connection = Arc::clone(&connection);
    let exercise_context_id = context_id.clone();
    let mut exercise = tokio::spawn(async move {
        let connection = exercise_connection;
        let context_id = exercise_context_id;
        let browser = CdpSession::browser(Arc::clone(&connection));
        let target_id = browser
            .target_create_in_context("about:blank", &context_id)
            .await
            .unwrap();
        let session_id = browser.target_attach(&target_id).await.unwrap();
        let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
            Arc::clone(&connection),
            session_id,
            target_id,
        ));
        let mut events = page.subscribe_events();
        page.runtime_enable().await.unwrap();
        let (execution_context_id, unique_context_id) =
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    let event = events
                        .recv()
                        .await
                        .expect("CDP event stream should remain open");
                    if event.method == "Runtime.executionContextCreated"
                        && event.params["context"]["auxData"]["isDefault"].as_bool() == Some(true)
                        && let Some(execution_context_id) = event.params["context"]["id"].as_i64()
                        && let Some(unique_id) = event.params["context"]["uniqueId"].as_str()
                    {
                        break (execution_context_id, unique_id.to_owned());
                    }
                }
            })
            .await
            .expect("main execution context should be announced");
        let values = EvaluationValueLimits::new(8, 64, 64, 4096, 4096).unwrap();
        let frames = FrameLimits::new(32 * 1024, 0, 0, 32 * 1024);
        let lifecycle = ChromiumHandleLifecycleProfile::new(
            ChromiumHandleRouteProfile::new(ResourceScope::extension(102, 2), 3, 4096, 4096),
            HandleLifecycleCodecLimits::new(1024),
            frames,
            frames,
        );
        let profile = ChromiumHandleEvaluationProfile::new(
            ChromiumHandleEvaluationBackendProfile::new(
                CdpEvaluationProductionProfile::new(
                    ResourceScope::extension(102, 1),
                    CdpEvaluationLimits::new(4096, 64 * 1024, 4096, values),
                    1,
                )
                .with_context_routes(1, 4096),
                4096,
                4096,
                CdpHandlePropertyLimits::new(1, 4096).unwrap(),
            ),
            HandleEvaluationCodecLimits::new(4096, 4096, values).unwrap(),
            frames,
            frames,
            lifecycle,
        );
        let runtime_limits = RuntimeLimits::new(8, 2)
            .with_active_commands(1)
            .with_control_messages(8);
        let requirements = runtime_limits
            .resource_requirements()
            .combine_concurrent(profile.requirements());
        let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
            .expect("exact property production plan should build");
        let runtime = Arc::new(
            ControlledDispatcher::with_object_kinds_and_topology(
                (),
                (),
                NoDeadlineScheduler,
                runtime_limits,
                BROWSER_OBJECT_KIND_CATALOG,
                ObjectTopologyMode::Complete,
            )
            .unwrap()
            .with_production_resources(&resources)
            .unwrap(),
        );
        for (object_id, parent, kind) in [
            (ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag),
            (
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                BROWSER_CONTEXT_OBJECT_KIND.tag,
            ),
            (
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                PAGE_OBJECT_KIND.tag,
            ),
            (
                ObjectId::new(4),
                Some(ObjectId::new(3)),
                FRAME_OBJECT_KIND.tag,
            ),
        ] {
            runtime.register_object(object_id, parent, kind).unwrap();
        }
        while runtime.pop_control_message().unwrap().is_some() {}
        let creation_continuation = CleanupQueue::new(1);
        let lifecycle_continuation = CleanupQueue::new(1);
        let installation = ChromiumHandleEvaluationInstallation::install(
            ChromiumHandleEvaluationBindings {
                runtime: runtime.lifecycle_handle(),
                object_ids: Arc::new(AtomicObjectIdSource::new(5).unwrap()),
                creation_continuation: creation_continuation.clone(),
                lifecycle_continuation: lifecycle_continuation.clone(),
            },
            profile,
            &resources,
        )
        .unwrap();
        installation
            .routes()
            .bind(
                ObjectId::new(4),
                ObjectId::new(3),
                Arc::clone(&page),
                execution_context_id,
                unique_context_id,
            )
            .unwrap();
        let baseline = resources.metrics().unwrap();
        let creation = installation.creation_service();
        let evaluation = installation.raw_service();
        let dispose = installation.lifecycle().raw_dispose_service();
        let target = creation
            .call(Command::<EvaluateHandle>::new(
                CallId::new(1),
                ObjectId::new(4),
                CallMetadata::default(),
                EvaluateHandleParams::function_without_argument(
                    r#"() => {
                    let reads = 0;
                    return {
                        promise: Promise.resolve(42),
                        thenable: {
                            marker: 'retained',
                            get then() {
                                reads++;
                                return resolve => resolve(7);
                            }
                        },
                        get reads() { return reads; }
                    };
                }"#,
                ),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();

        // Act through portable 1.2 commands, not the optional staged native API.
        let mut properties = Vec::with_capacity(2);
        for (call_id, name) in [(2, "promise"), (3, "thenable")] {
            let command = Command::<HandleGetProperty>::new(
                CallId::new(call_id),
                target.object_id,
                CallMetadata::default(),
                GetPropertyParams::new(name),
            );
            let command = installation
                .protocol()
                .decode::<Command<HandleGetProperty>>(
                    installation.protocol().encode(&command).unwrap(),
                )
                .unwrap();
            properties.push(
                creation
                    .call(command)
                    .await
                    .unwrap()
                    .into_outcome()
                    .unwrap(),
            );
        }
        let mut original_identities = Vec::with_capacity(2);
        for (call_id, property, source) in [
            (
                4,
                &properties[0],
                "(value, target) => value === target.promise && value instanceof Promise",
            ),
            (
                5,
                &properties[1],
                "(value, target) => value === target.thenable && target.reads === 0",
            ),
        ] {
            original_identities.push(
                evaluation
                    .call(Command::<HandleEvaluate>::new(
                        CallId::new(call_id),
                        property.object_id,
                        CallMetadata::default(),
                        HandleEvaluateParams::function(
                            source,
                            HandleEvaluationValue::Handle(target.object_id),
                        ),
                    ))
                    .await
                    .unwrap()
                    .into_outcome()
                    .unwrap()
                    .value,
            );
        }
        dispose
            .call(Command::<HandleDispose>::new(
                CallId::new(6),
                target.object_id,
                CallMetadata::default(),
                (),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
        let retained_after_target_disposal = installation.lifecycle().routes().metrics().unwrap();
        let mut retained_values = Vec::with_capacity(2);
        for (call_id, property, source) in [
            (7, &properties[0], "value => value instanceof Promise"),
            (8, &properties[1], "value => value.marker === 'retained'"),
        ] {
            retained_values.push(
                evaluation
                    .call(Command::<HandleEvaluate>::new(
                        CallId::new(call_id),
                        property.object_id,
                        CallMetadata::default(),
                        HandleEvaluateParams::function_without_argument(source),
                    ))
                    .await
                    .unwrap()
                    .into_outcome()
                    .unwrap()
                    .value,
            );
        }
        for (call_id, property) in [(9, &properties[0]), (10, &properties[1])] {
            dispose
                .call(Command::<HandleDispose>::new(
                    CallId::new(call_id),
                    property.object_id,
                    CallMetadata::default(),
                    (),
                ))
                .await
                .unwrap()
                .into_outcome()
                .unwrap();
        }
        while runtime.pop_control_message().unwrap().is_some() {}
        let released = resources.metrics().unwrap();
        let routes = installation.lifecycle().routes().metrics().unwrap();

        // Assert identity was retained without Promise assimilation or reading the
        // then getter, and all published ownership returned to its starting budget.
        assert_eq!(
            original_identities,
            [EvaluationValue::Bool(true), EvaluationValue::Bool(true)]
        );
        assert_eq!(
            retained_values,
            [EvaluationValue::Bool(true), EvaluationValue::Bool(true)]
        );
        assert_eq!(retained_after_target_disposal.current, 2);
        assert!(!runtime.is_object_live(target.object_id).unwrap());
        for property in &properties {
            assert_eq!(property.parent_id, ObjectId::new(4));
            assert!(!runtime.is_object_live(property.object_id).unwrap());
        }
        assert_eq!(routes.current, 0);
        assert_eq!(routes.reserved, 0);
        assert_eq!(routes.explicitly_disposed, 3);
        assert_eq!(released.active_leases, baseline.active_leases);
        for class in [
            ResourceClass::RemoteHandle,
            ResourceClass::BackendStateBytes,
        ] {
            assert_eq!(released.class(class).current, baseline.class(class).current);
        }
        for continuation in [creation_continuation, lifecycle_continuation] {
            let metrics = continuation.metrics().unwrap();
            assert_eq!(metrics.reserved, 0);
            assert_eq!(metrics.queued, 0);
            assert_eq!(metrics.accepted, 0);
        }
    });
    let outcome = tokio::time::timeout(Duration::from_secs(30), &mut exercise).await;
    if outcome.is_err() {
        exercise.abort();
        let _ = exercise.await;
    }

    // Keep Context ownership outside the exercise so panic, operation failure,
    // and timeout all reach bounded cleanup before reporting the test outcome.
    let cleanup = tokio::time::timeout(
        Duration::from_secs(10),
        browser.target_dispose_browser_context(&context_id),
    )
    .await;
    let browser_alive = tokio::time::timeout(
        Duration::from_secs(10),
        connection.send_command("Browser.getVersion", serde_json::json!({})),
    )
    .await;
    assert!(
        matches!(cleanup, Ok(Ok(_))),
        "test-owned Context cleanup failed: {cleanup:?}; exercise: {outcome:?}"
    );
    assert!(
        matches!(browser_alive, Ok(Ok(_))),
        "externally owned Chrome must remain alive: {browser_alive:?}; exercise: {outcome:?}"
    );
    assert!(
        matches!(outcome, Ok(Ok(()))),
        "portable property exercise failed or exceeded 30 seconds: {outcome:?}"
    );
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn independently_installed_handle_evaluation_reaches_real_chrome_and_disposes() {
    // Arrange one externally owned browser and one test-owned target.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .unwrap();
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .unwrap();
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let context_id = browser
        .target_create_browser_context(None, None)
        .await
        .unwrap();
    let target_id = browser
        .target_create_in_context(
            "data:text/html,<main id='root'>composition</main>",
            &context_id,
        )
        .await
        .unwrap();
    let session_id = browser.target_attach(&target_id).await.unwrap();
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id,
    ));
    let mut events = page.subscribe_events();
    page.runtime_enable().await.unwrap();
    let (execution_context_id, unique_context_id) =
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let event = events
                    .recv()
                    .await
                    .expect("CDP event stream should remain open");
                if event.method == "Runtime.executionContextCreated"
                    && event.params["context"]["auxData"]["isDefault"].as_bool() == Some(true)
                    && let Some(execution_context_id) = event.params["context"]["id"].as_i64()
                    && let Some(unique_id) = event.params["context"]["uniqueId"].as_str()
                {
                    break (execution_context_id, unique_id.to_owned());
                }
            }
        })
        .await
        .expect("main execution context should be announced");

    let values = EvaluationValueLimits::new(8, 64, 64, 4096, 4096).unwrap();
    let frames = FrameLimits::new(32 * 1024, 0, 0, 32 * 1024);
    let lifecycle = ChromiumHandleLifecycleProfile::new(
        ChromiumHandleRouteProfile::new(ResourceScope::extension(99, 2), 2, 4096, 4096),
        HandleLifecycleCodecLimits::new(1024),
        frames,
        frames,
    );
    let profile = ChromiumHandleEvaluationProfile::new(
        ChromiumHandleEvaluationBackendProfile::new(
            CdpEvaluationProductionProfile::new(
                ResourceScope::extension(99, 1),
                CdpEvaluationLimits::new(4096, 64 * 1024, 4096, values),
                1,
            )
            .with_context_routes(1, 4096),
            4096,
            4096,
            CdpHandlePropertyLimits::new(8, 4096).unwrap(),
        ),
        HandleEvaluationCodecLimits::new(4096, 4096, values).unwrap(),
        frames,
        frames,
        lifecycle,
    );
    let runtime_limits = RuntimeLimits::new(8, 2)
        .with_active_commands(1)
        .with_control_messages(8);
    let requirements = runtime_limits
        .resource_requirements()
        .combine_concurrent(profile.requirements());
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("exact production plan should build");
    let runtime = Arc::new(
        ControlledDispatcher::with_object_kinds_and_topology(
            (),
            (),
            NoDeadlineScheduler,
            runtime_limits,
            BROWSER_OBJECT_KIND_CATALOG,
            ObjectTopologyMode::Complete,
        )
        .unwrap()
        .with_production_resources(&resources)
        .unwrap(),
    );
    for (object_id, parent, kind) in [
        (ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag),
        (
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            BROWSER_CONTEXT_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(3),
            Some(ObjectId::new(2)),
            PAGE_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(4),
            Some(ObjectId::new(3)),
            FRAME_OBJECT_KIND.tag,
        ),
    ] {
        runtime.register_object(object_id, parent, kind).unwrap();
    }
    while runtime.pop_control_message().unwrap().is_some() {}
    let installation = ChromiumHandleEvaluationInstallation::install(
        ChromiumHandleEvaluationBindings {
            runtime: runtime.lifecycle_handle(),
            object_ids: Arc::new(AtomicObjectIdSource::new(5).unwrap()),
            creation_continuation: CleanupQueue::new(1),
            lifecycle_continuation: CleanupQueue::new(1),
        },
        profile,
        &resources,
    )
    .unwrap();
    installation
        .routes()
        .bind(
            ObjectId::new(4),
            ObjectId::new(3),
            Arc::clone(&page),
            execution_context_id,
            unique_context_id,
        )
        .unwrap();

    let (server_transport, client_transport) = MemoryFrameTransport::pair(8);
    let server_transport = Arc::new(server_transport);
    let control = OrderedRuntimeControl::new(
        Arc::clone(&runtime),
        RuntimeControlDriver::new(control_protocol(), Arc::clone(&server_transport)),
        1,
    )
    .expect("ordered control should build");
    let handler = OrderedHandleEvaluationHandler::new(
        installation.protocol(),
        installation.raw_service(),
        installation.raw_service(),
        installation.lifecycle().raw_dispose_service(),
        CleanupQueue::new(1),
        control.clone(),
    );

    // Act through the independent 1.2 leaf plus the opt-in remote fence.
    let call_id = CallId::new(1);
    let command = Command::<EvaluateHandle>::new(
        call_id,
        ObjectId::new(4),
        CallMetadata::default(),
        EvaluateHandleParams::expression("({ answer: 42 })"),
    );
    let disposition = handler
        .handle_concurrently(
            installation.protocol().encode(&command).unwrap(),
            Arc::clone(&server_transport),
        )
        .await
        .unwrap();
    let create_frame = client_transport.receive().await.unwrap().unwrap();
    let result_frame = client_transport.receive().await.unwrap().unwrap();
    let handle = installation
        .protocol()
        .decode::<CommandResult<EvaluateHandle>>(result_frame.clone())
        .unwrap()
        .into_outcome()
        .unwrap();
    let value = installation
        .raw_service()
        .call(Command::<HandleEvaluate>::new(
            CallId::new(2),
            handle.object_id,
            CallMetadata::default(),
            HandleEvaluateParams::function_without_argument("target => target.answer"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    OrderedHandleDisposeService::new(installation.lifecycle().raw_dispose_service(), control)
        .call(Command::<HandleDispose>::new(
            CallId::new(3),
            handle.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let dispose_frame = client_transport.receive().await.unwrap().unwrap();

    // Assert Create preceded Result, Dispose preceded completion, portable
    // ownership retired, and external Chrome stayed alive.
    assert!(matches!(disposition, ServerFrameDisposition::Handled(_)));
    assert_eq!(create_frame.route(), FrameRoute::Control);
    assert!(matches!(
        control_protocol()
            .decode::<ControlMessage>(create_frame)
            .unwrap(),
        ControlMessage::Create {
            object_id,
            parent: Some(parent),
            ..
        } if object_id == handle.object_id && parent == ObjectId::new(4)
    ));
    assert_eq!(result_frame.route(), FrameRoute::Result { call_id });
    assert_eq!(dispose_frame.route(), FrameRoute::Control);
    assert_eq!(
        control_protocol()
            .decode::<ControlMessage>(dispose_frame)
            .unwrap(),
        ControlMessage::Dispose {
            object_id: handle.object_id,
        }
    );
    assert_eq!(value.value, EvaluationValue::number(42.0).unwrap());
    assert!(!runtime.is_object_live(handle.object_id).unwrap());
    assert!(
        !installation
            .lifecycle()
            .routes()
            .contains(handle.object_id)
            .unwrap()
    );
    browser
        .target_dispose_browser_context(&context_id)
        .await
        .unwrap();
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome must remain alive");
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn production_adoption_batch_reaches_real_chrome_and_releases_every_resource() {
    // Arrange one main-world ElementHandle, one utility world in the same CDP
    // target session, and an exact production budget for one temporary object.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .unwrap();
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .unwrap();
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let context_id = browser
        .target_create_browser_context(None, None)
        .await
        .unwrap();
    let target_id = browser
        .target_create_in_context("data:text/html,<main id='root'>MAIN</main>", &context_id)
        .await
        .unwrap();
    let session_id = browser.target_attach(&target_id).await.unwrap();
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id,
    ));
    let mut events = page.subscribe_events();
    page.runtime_enable().await.unwrap();
    let (main_execution_context_id, main_unique_context_id) =
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let event = events
                    .recv()
                    .await
                    .expect("CDP event stream should remain open");
                if event.method == "Runtime.executionContextCreated"
                    && event.params["context"]["auxData"]["isDefault"].as_bool() == Some(true)
                    && let Some(execution_context_id) = event.params["context"]["id"].as_i64()
                    && let Some(unique_id) = event.params["context"]["uniqueId"].as_str()
                {
                    break (execution_context_id, unique_id.to_owned());
                }
            }
        })
        .await
        .expect("main execution context should be announced");
    let handle_limits = CdpHandleEvaluationLimits::default();
    let main_service = CdpHandleEvaluationService::from_evaluation(
        CdpEvaluationService::with_limits_in_execution_context(
            Arc::clone(&page),
            main_execution_context_id,
            main_unique_context_id,
            handle_limits.evaluation(),
        ),
        handle_limits.native_id_bytes(),
        handle_limits.preview_bytes(),
    );
    let source = main_service
        .call(EvaluateHandleParams::expression(
            "document.querySelector('main')",
        ))
        .await
        .unwrap();
    let frame_id = page.page_get_main_frame_id().await.unwrap();
    let utility_execution_context_id = page
        .page_create_isolated_world(&frame_id, "__pwright_adoption_live__", false)
        .await
        .unwrap();
    let utility_unique_context_id = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = events
                .recv()
                .await
                .expect("CDP event stream should remain open");
            if event.method == "Runtime.executionContextCreated"
                && event.params["context"]["id"].as_i64() == Some(utility_execution_context_id)
                && let Some(unique_id) = event.params["context"]["uniqueId"].as_str()
            {
                break unique_id.to_owned();
            }
        }
    })
    .await
    .expect("utility execution context should be announced");
    let adoption_profile = CdpElementHandleAdoptionProductionProfile::new(
        ResourceScope::extension(99, 3),
        handle_limits,
        1,
        1,
    );
    let resources = ProductionResourceLedger::new(
        exact_limits(adoption_profile.requirements()),
        adoption_profile.requirements(),
    )
    .expect("exact adoption production plan should build");
    let raw_adoption = Arc::new(
        CdpElementHandleAdoptionBatchService::new_with_production_resources(
            adoption_profile,
            &resources,
        )
        .unwrap(),
    );
    let continuation = CleanupQueue::new(1);
    let adoption = ContinuableElementHandleAdoptionService::new(raw_adoption, continuation.clone());

    // Act: materialize the source node in the utility world, consume that
    // temporary native argument there, then explicitly release both results.
    let batch = adoption
        .call(CdpElementHandleAdoptionBatchRequest::new(vec![
            CdpElementHandleAdoptionRequest::new(
                source.clone(),
                Arc::clone(&page),
                utility_execution_context_id,
            ),
        ]))
        .await
        .unwrap();
    let while_owned = resources.metrics().unwrap();
    let utility_service = CdpHandleEvaluationService::from_evaluation(
        CdpEvaluationService::with_limits_in_execution_context(
            Arc::clone(&page),
            utility_execution_context_id,
            utility_unique_context_id,
            handle_limits.evaluation(),
        ),
        handle_limits.native_id_bytes(),
        handle_limits.preview_bytes(),
    );
    let output = utility_service
        .call(CdpHandleEvaluationRequest::new(
            EvaluateHandleParams::function(
                "value => value.textContent",
                HandleEvaluationValue::Handle(ObjectId::new(1)),
            ),
            vec![(ObjectId::new(1), batch.handles()[0].handle().clone())],
        ))
        .await
        .unwrap();
    batch.release().await.unwrap();
    source.release().await.unwrap();
    output.release().await.unwrap();
    let released = resources.metrics().unwrap();

    // Assert the utility-world call observed the adopted node, the temporary
    // object held its declared lease only while owned, every resource returned
    // to zero, and the externally owned Chrome process stayed alive.
    assert_eq!(output.preview(), "MAIN");
    assert_eq!(while_owned.class(ResourceClass::RemoteHandle).current, 1);
    assert!(while_owned.class(ResourceClass::BackendStateBytes).current > 0);
    assert_eq!(while_owned.active_leases, 1);
    assert_eq!(released.class(ResourceClass::RemoteHandle).current, 0);
    assert_eq!(released.class(ResourceClass::BackendStateBytes).current, 0);
    assert_eq!(released.active_leases, 0);
    let cleanup_metrics = continuation.metrics().unwrap();
    assert_eq!(cleanup_metrics.reserved, 0);
    assert_eq!(cleanup_metrics.queued, 0);
    assert_eq!(cleanup_metrics.accepted, 0);
    browser
        .target_dispose_browser_context(&context_id)
        .await
        .unwrap();
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome must remain alive");
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn opted_in_public_command_adopts_a_real_same_process_child_frame_element() {
    // Arrange one test-owned Page with a same-process child Frame and capture
    // both exact renderer-world identities from the dedicated Chrome process.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .unwrap();
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .unwrap();
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let context_id = browser
        .target_create_browser_context(None, None)
        .await
        .unwrap();
    let target_id = browser
        .target_create_in_context("about:blank", &context_id)
        .await
        .unwrap();
    let session_id = browser.target_attach(&target_id).await.unwrap();
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id,
    ));
    let main_frame_id = page.page_get_main_frame_id().await.unwrap();
    let mut events = page.subscribe_events();
    page.page_enable().await.unwrap();
    page.runtime_enable().await.unwrap();
    page.page_set_document_content(
        &main_frame_id,
        r#"<!doctype html><iframe srcdoc="<main id='child'>CHILD</main>"></iframe>"#,
    )
    .await
    .unwrap();
    let (main_world, child_frame_id, child_world) =
        tokio::time::timeout(Duration::from_secs(10), async {
            let mut main = None;
            let mut child = None;
            loop {
                let event = events
                    .recv()
                    .await
                    .expect("CDP event stream should remain open");
                if event.method != "Runtime.executionContextCreated"
                    || event.params["context"]["auxData"]["isDefault"].as_bool() != Some(true)
                {
                    continue;
                }
                let Some(frame_id) = event.params["context"]["auxData"]["frameId"].as_str() else {
                    continue;
                };
                let Some(numeric_id) = event.params["context"]["id"].as_i64() else {
                    continue;
                };
                let Some(unique_id) = event.params["context"]["uniqueId"].as_str() else {
                    continue;
                };
                if frame_id == main_frame_id {
                    main = Some((numeric_id, unique_id.to_owned()));
                } else {
                    child = Some((frame_id.to_owned(), (numeric_id, unique_id.to_owned())));
                }
                if let (Some(main), Some((child_frame_id, child))) = (&main, &child) {
                    break (main.clone(), child_frame_id.clone(), child.clone());
                }
            }
        })
        .await
        .expect("main and child execution contexts should be announced");
    let values = EvaluationValueLimits::new(8, 64, 64, 4096, 4096).unwrap();
    let frames = FrameLimits::new(32 * 1024, 0, 0, 32 * 1024);
    let lifecycle = ChromiumHandleLifecycleProfile::new(
        ChromiumHandleRouteProfile::new(ResourceScope::extension(100, 2), 3, 4096, 4096),
        HandleLifecycleCodecLimits::new(1024),
        frames,
        frames,
    );
    let profile = ChromiumHandleEvaluationProfile::new(
        ChromiumHandleEvaluationBackendProfile::new(
            CdpEvaluationProductionProfile::new(
                ResourceScope::extension(100, 1),
                CdpEvaluationLimits::new(4096, 64 * 1024, 4096, values),
                1,
            )
            .with_context_routes(2, 4096),
            4096,
            4096,
            CdpHandlePropertyLimits::new(8, 4096).unwrap(),
        ),
        HandleEvaluationCodecLimits::new(4096, 4096, values).unwrap(),
        frames,
        frames,
        lifecycle,
    );
    let adoption_profile = CdpElementHandleAdoptionProductionProfile::new(
        ResourceScope::extension(100, 3),
        CdpHandleEvaluationLimits::new(
            profile.backend().evaluation().limits(),
            profile.backend().native_id_bytes(),
            profile.backend().preview_bytes(),
        ),
        1,
        1,
    );
    let runtime_limits = RuntimeLimits::new(8, 2)
        .with_active_commands(1)
        .with_control_messages(8);
    let requirements = runtime_limits
        .resource_requirements()
        .combine_concurrent(profile.requirements_with_element_adoption(adoption_profile));
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("exact public-adoption plan should build");
    let runtime = Arc::new(
        ControlledDispatcher::with_object_kinds_and_topology(
            (),
            (),
            NoDeadlineScheduler,
            runtime_limits,
            BROWSER_OBJECT_KIND_CATALOG,
            ObjectTopologyMode::Complete,
        )
        .unwrap()
        .with_production_resources(&resources)
        .unwrap(),
    );
    for (object_id, parent, kind) in [
        (ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag),
        (
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            BROWSER_CONTEXT_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(3),
            Some(ObjectId::new(2)),
            PAGE_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(4),
            Some(ObjectId::new(3)),
            FRAME_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(5),
            Some(ObjectId::new(3)),
            FRAME_OBJECT_KIND.tag,
        ),
    ] {
        runtime.register_object(object_id, parent, kind).unwrap();
    }
    while runtime.pop_control_message().unwrap().is_some() {}
    let adoption_continuation = CleanupQueue::new(1);
    let installation = ChromiumHandleEvaluationInstallation::install_with_element_adoption(
        ChromiumHandleEvaluationBindings {
            runtime: runtime.lifecycle_handle(),
            object_ids: Arc::new(AtomicObjectIdSource::new(6).unwrap()),
            creation_continuation: CleanupQueue::new(1),
            lifecycle_continuation: CleanupQueue::new(1),
        },
        profile,
        adoption_profile,
        adoption_continuation.clone(),
        &resources,
    )
    .unwrap();
    installation
        .routes()
        .bind(
            ObjectId::new(4),
            ObjectId::new(3),
            Arc::clone(&page),
            main_world.0,
            main_world.1,
        )
        .unwrap();
    installation
        .routes()
        .bind(
            ObjectId::new(5),
            ObjectId::new(3),
            Arc::clone(&page),
            child_world.0,
            child_world.1,
        )
        .unwrap();
    let baseline = resources.metrics().unwrap();
    let service = installation.creation_service();

    // Act through the public command services: create the source in the child,
    // pass its portable ID to the parent, then dispose both published Handles.
    let source = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(5),
            CallMetadata::default(),
            EvaluateHandleParams::expression("document.querySelector('main')"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let output = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(2),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::function(
                "node => node.textContent",
                HandleEvaluationValue::Handle(source.object_id),
            ),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let dispose = installation.lifecycle().raw_dispose_service();
    for (call_id, object_id) in [
        (CallId::new(3), output.object_id),
        (CallId::new(4), source.object_id),
    ] {
        dispose
            .call(Command::<HandleDispose>::new(
                call_id,
                object_id,
                CallMetadata::default(),
                (),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
    }
    while runtime.pop_control_message().unwrap().is_some() {}
    let released = resources.metrics().unwrap();

    // Assert the actual same-process child node crossed worlds, all temporary
    // and published ownership returned to baseline, and external Chrome lives.
    assert_eq!(output.preview, "CHILD");
    assert_eq!(released.active_leases, baseline.active_leases);
    assert_eq!(
        released.class(ResourceClass::RemoteHandle).current,
        baseline.class(ResourceClass::RemoteHandle).current
    );
    assert_eq!(adoption_continuation.metrics().unwrap().reserved, 0);
    assert_eq!(adoption_continuation.metrics().unwrap().queued, 0);
    assert!(!child_frame_id.is_empty());
    browser
        .target_dispose_browser_context(&context_id)
        .await
        .unwrap();
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome must remain alive");
}

#[tokio::test]
#[ignore = "requires dedicated Chrome, internet, PWRIGHT_CDP_WS_URL, and PWRIGHT_OOPIF_FIXTURE_URL"]
async fn opted_in_public_command_rejects_a_real_auto_attached_oopif_element() {
    // Arrange an externally owned Chrome Page that recursively auto-attaches a
    // cross-site iframe into its own CDP target session.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .unwrap();
    let fixture_url = std::env::var("PWRIGHT_OOPIF_FIXTURE_URL")
        .expect("PWRIGHT_OOPIF_FIXTURE_URL must serve a cross-site iframe fixture");
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .unwrap();
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let context_id = browser
        .target_create_browser_context(None, None)
        .await
        .unwrap();
    let target_id = browser
        .target_create_in_context("about:blank", &context_id)
        .await
        .unwrap();
    let page_session_id = browser.target_attach(&target_id).await.unwrap();
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        page_session_id,
        target_id,
    ));
    let main_frame_id = page.page_get_main_frame_id().await.unwrap();
    let mut page_events = page.subscribe_events();
    page.page_enable().await.unwrap();
    page.runtime_enable().await.unwrap();
    let main_world = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = page_events
                .recv()
                .await
                .expect("Page event stream should remain open");
            if event.method == "Runtime.executionContextCreated"
                && event.params["context"]["auxData"]["frameId"].as_str()
                    == Some(main_frame_id.as_str())
                && event.params["context"]["auxData"]["isDefault"].as_bool() == Some(true)
                && let Some(numeric_id) = event.params["context"]["id"].as_i64()
                && let Some(unique_id) = event.params["context"]["uniqueId"].as_str()
            {
                break (numeric_id, unique_id.to_owned());
            }
        }
    })
    .await
    .expect("main execution context should be announced");
    page.target_set_auto_attach(true, false, true)
        .await
        .unwrap();
    page.page_navigate(&fixture_url).await.unwrap();
    let (oopif_session_id, oopif_target_id) =
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                let event = page_events
                    .recv()
                    .await
                    .expect("Page event stream should remain open");
                if event.method == "Target.attachedToTarget"
                    && event.params["targetInfo"]["type"].as_str() == Some("iframe")
                    && let Some(session_id) = event.params["sessionId"].as_str()
                    && let Some(target_id) = event.params["targetInfo"]["targetId"].as_str()
                {
                    break (session_id.to_owned(), target_id.to_owned());
                }
            }
        })
        .await
        .expect("cross-site iframe should auto-attach as an OOPIF");
    let oopif: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        oopif_session_id,
        oopif_target_id,
    ));
    let oopif_tree = oopif.page_get_frame_tree().await.unwrap();
    let oopif_frame_id = oopif_tree.frame.id;
    assert!(
        oopif_tree.frame.parent_id.is_some(),
        "Chrome OOPIF root should retain its embedding parent"
    );
    let mut oopif_events = oopif.subscribe_events();
    oopif.page_enable().await.unwrap();
    oopif.runtime_enable().await.unwrap();
    let oopif_world = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = oopif_events
                .recv()
                .await
                .expect("OOPIF event stream should remain open");
            if event.method == "Runtime.executionContextCreated"
                && event.params["context"]["auxData"]["frameId"].as_str()
                    == Some(oopif_frame_id.as_str())
                && event.params["context"]["auxData"]["isDefault"].as_bool() == Some(true)
                && let Some(numeric_id) = event.params["context"]["id"].as_i64()
                && let Some(unique_id) = event.params["context"]["uniqueId"].as_str()
            {
                break (numeric_id, unique_id.to_owned());
            }
        }
    })
    .await
    .expect("OOPIF execution context should be announced");

    let values = EvaluationValueLimits::new(8, 64, 64, 4096, 4096).unwrap();
    let frames = FrameLimits::new(32 * 1024, 0, 0, 32 * 1024);
    let lifecycle = ChromiumHandleLifecycleProfile::new(
        ChromiumHandleRouteProfile::new(ResourceScope::extension(101, 2), 2, 4096, 4096),
        HandleLifecycleCodecLimits::new(1024),
        frames,
        frames,
    );
    let profile = ChromiumHandleEvaluationProfile::new(
        ChromiumHandleEvaluationBackendProfile::new(
            CdpEvaluationProductionProfile::new(
                ResourceScope::extension(101, 1),
                CdpEvaluationLimits::new(4096, 64 * 1024, 4096, values),
                1,
            )
            .with_context_routes(2, 4096),
            4096,
            4096,
            CdpHandlePropertyLimits::new(8, 4096).unwrap(),
        ),
        HandleEvaluationCodecLimits::new(4096, 4096, values).unwrap(),
        frames,
        frames,
        lifecycle,
    );
    let adoption_profile = CdpElementHandleAdoptionProductionProfile::new(
        ResourceScope::extension(101, 3),
        CdpHandleEvaluationLimits::new(
            profile.backend().evaluation().limits(),
            profile.backend().native_id_bytes(),
            profile.backend().preview_bytes(),
        ),
        1,
        1,
    );
    let runtime_limits = RuntimeLimits::new(8, 2)
        .with_active_commands(1)
        .with_control_messages(8);
    let requirements = runtime_limits
        .resource_requirements()
        .combine_concurrent(profile.requirements_with_element_adoption(adoption_profile));
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("exact OOPIF-rejection plan should build");
    let runtime = Arc::new(
        ControlledDispatcher::with_object_kinds_and_topology(
            (),
            (),
            NoDeadlineScheduler,
            runtime_limits,
            BROWSER_OBJECT_KIND_CATALOG,
            ObjectTopologyMode::Complete,
        )
        .unwrap()
        .with_production_resources(&resources)
        .unwrap(),
    );
    for (object_id, parent, kind) in [
        (ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag),
        (
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            BROWSER_CONTEXT_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(3),
            Some(ObjectId::new(2)),
            PAGE_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(4),
            Some(ObjectId::new(3)),
            FRAME_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(5),
            Some(ObjectId::new(3)),
            FRAME_OBJECT_KIND.tag,
        ),
    ] {
        runtime.register_object(object_id, parent, kind).unwrap();
    }
    while runtime.pop_control_message().unwrap().is_some() {}
    let adoption_continuation = CleanupQueue::new(1);
    let installation = ChromiumHandleEvaluationInstallation::install_with_element_adoption(
        ChromiumHandleEvaluationBindings {
            runtime: runtime.lifecycle_handle(),
            object_ids: Arc::new(AtomicObjectIdSource::new(6).unwrap()),
            creation_continuation: CleanupQueue::new(1),
            lifecycle_continuation: CleanupQueue::new(1),
        },
        profile,
        adoption_profile,
        adoption_continuation.clone(),
        &resources,
    )
    .unwrap();
    installation
        .routes()
        .bind(
            ObjectId::new(4),
            ObjectId::new(3),
            Arc::clone(&page),
            main_world.0,
            main_world.1,
        )
        .unwrap();
    installation
        .routes()
        .bind(
            ObjectId::new(5),
            ObjectId::new(3),
            Arc::clone(&oopif),
            oopif_world.0,
            oopif_world.1,
        )
        .unwrap();
    let service = installation.creation_service();
    let source = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(5),
            CallMetadata::default(),
            EvaluateHandleParams::expression("document.body"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    while runtime.pop_control_message().unwrap().is_some() {}
    let before = resources.metrics().unwrap();

    // Act: pass the real OOPIF ElementHandle into the parent Frame.
    let error = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(2),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::function(
                "node => node.tagName",
                HandleEvaluationValue::Handle(source.object_id),
            ),
        ))
        .await
        .unwrap()
        .into_outcome()
        .expect_err("cross-session OOPIF ElementHandle must not be adopted");
    let after = resources.metrics().unwrap();

    // Assert rejection happened without temporary resource admission, then
    // clean only test-owned state and preserve the external browser process.
    assert_eq!(error.kind(), pwright_core::FailureKind::ProtocolViolation);
    assert_eq!(after.active_leases, before.active_leases);
    assert_eq!(
        after.class(ResourceClass::RemoteHandle).current,
        before.class(ResourceClass::RemoteHandle).current
    );
    assert_eq!(adoption_continuation.metrics().unwrap().reserved, 0);
    assert_eq!(adoption_continuation.metrics().unwrap().queued, 0);
    installation
        .lifecycle()
        .raw_dispose_service()
        .call(Command::<HandleDispose>::new(
            CallId::new(3),
            source.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    browser
        .target_dispose_browser_context(&context_id)
        .await
        .unwrap();
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome must remain alive");
}
