use std::sync::Arc;
use std::time::Duration;

use pwright_backend_cdp::{
    CdpElementHandleAdoptionRequest, CdpElementHandleAdoptionService, CdpEvaluationService,
    CdpHandleEvaluationLimits, CdpHandleEvaluationRequest, CdpHandleEvaluationService,
    CdpHandleKind,
};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cap_handle_evaluation::{EvaluateHandleParams, HandleEvaluationValue};
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpClient, CdpSession};
use pwright_core::Service;
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn retained_element_and_primitive_handles_reach_real_chrome_and_release() {
    // Arrange one temporary target owned by this test.
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
    let browser_context_id = browser
        .target_create_browser_context(None, None)
        .await
        .unwrap();
    let target_id = browser
        .target_create_in_context(
            "data:text/html,<main id='root'>handle</main>",
            &browser_context_id,
        )
        .await
        .unwrap();
    let session_id = browser.target_attach(&target_id).await.unwrap();
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    let mut events = page.subscribe_events();
    page.runtime_enable().await.unwrap();
    let unique_context_id = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = events
                .recv()
                .await
                .expect("CDP event stream should remain open");
            if event.method == "Runtime.executionContextCreated"
                && event.params["context"]["auxData"]["isDefault"].as_bool() == Some(true)
                && let Some(unique_id) = event.params["context"]["uniqueId"].as_str()
            {
                break unique_id.to_owned();
            }
        }
    })
    .await
    .expect("main execution context should be announced");
    let evaluation = CdpEvaluationService::with_limits_in_unique_context(
        Arc::clone(&page),
        unique_context_id,
        Default::default(),
    );
    let service = CdpHandleEvaluationService::from_evaluation(evaluation, 4096, 4096);

    // Act.
    let element = service
        .call(EvaluateHandleParams::expression(
            "document.createElement('section')",
        ))
        .await
        .unwrap();
    let primitive = service
        .call(EvaluateHandleParams::function(
            "value => value + 1",
            pwright_cap_evaluation::EvaluationValue::number(41.0).unwrap(),
        ))
        .await
        .unwrap();
    let from_handle = service
        .call(CdpHandleEvaluationRequest::new(
            EvaluateHandleParams::function(
                "node => node.tagName",
                HandleEvaluationValue::Handle(pwright_core::ObjectId::new(41)),
            ),
            vec![(pwright_core::ObjectId::new(41), element.clone())],
        ))
        .await
        .unwrap();
    element.release().await.unwrap();
    primitive.release().await.unwrap();
    from_handle.release().await.unwrap();

    // Assert the DOM result owns a native object and primitive handles do not.
    assert_eq!(element.kind(), CdpHandleKind::Element);
    assert!(element.object_id().is_some());
    assert_eq!(primitive.kind(), CdpHandleKind::Js);
    assert_eq!(primitive.object_id(), None);
    assert_eq!(from_handle.kind(), CdpHandleKind::Js);
    assert_eq!(from_handle.object_id(), None);
    assert_eq!(from_handle.preview(), "SECTION");

    browser
        .target_dispose_browser_context(&browser_context_id)
        .await
        .unwrap();
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome must remain alive");
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn element_handle_adoption_reaches_a_second_real_execution_world_and_releases() {
    // Arrange one temporary target and discover its main execution world.
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
    let browser_context_id = browser
        .target_create_browser_context(None, None)
        .await
        .unwrap();
    let target_id = browser
        .target_create_in_context(
            "data:text/html,<main id='root'>adoption</main>",
            &browser_context_id,
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
    let main_unique_context_id = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = events
                .recv()
                .await
                .expect("CDP event stream should remain open");
            if event.method == "Runtime.executionContextCreated"
                && event.params["context"]["auxData"]["isDefault"].as_bool() == Some(true)
                && let Some(unique_id) = event.params["context"]["uniqueId"].as_str()
            {
                break unique_id.to_owned();
            }
        }
    })
    .await
    .expect("main execution context should be announced");
    let source_service = CdpHandleEvaluationService::from_evaluation(
        CdpEvaluationService::with_limits_in_unique_context(
            Arc::clone(&page),
            main_unique_context_id,
            Default::default(),
        ),
        4096,
        4096,
    );
    let source = source_service
        .call(EvaluateHandleParams::expression(
            "document.querySelector('main')",
        ))
        .await
        .unwrap();
    let frame_id = page.page_get_main_frame_id().await.unwrap();
    let utility_execution_context_id = page
        .page_create_isolated_world(&frame_id, "__pwright_adoption_test__", true)
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
    let adoption = CdpElementHandleAdoptionService::new(CdpHandleEvaluationLimits::default());
    let utility_service = CdpHandleEvaluationService::from_evaluation(
        CdpEvaluationService::with_limits_in_unique_context(
            Arc::clone(&page),
            utility_unique_context_id,
            Default::default(),
        ),
        4096,
        4096,
    );

    // Act through the temporary target-world wrapper, then release both owners.
    let adopted = adoption
        .call(CdpElementHandleAdoptionRequest::new(
            source.clone(),
            Arc::clone(&page),
            utility_execution_context_id,
        ))
        .await
        .unwrap();
    let result = utility_service
        .call(CdpHandleEvaluationRequest::new(
            EvaluateHandleParams::function(
                "node => node.tagName",
                HandleEvaluationValue::Handle(pwright_core::ObjectId::new(51)),
            ),
            vec![(pwright_core::ObjectId::new(51), adopted.handle().clone())],
        ))
        .await
        .unwrap();
    adopted.release().await.unwrap();
    source.release().await.unwrap();
    result.release().await.unwrap();

    // Assert the adopted node reached the other world and Chrome stays external.
    assert_eq!(result.kind(), CdpHandleKind::Js);
    assert_eq!(result.object_id(), None);
    assert_eq!(result.preview(), "MAIN");
    browser
        .target_dispose_browser_context(&browser_context_id)
        .await
        .unwrap();
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome must remain alive");
}
