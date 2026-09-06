use std::num::NonZeroU32;
use std::sync::Arc;

use pwright_backend_cdp::{CdpEvaluationLimits, CdpEvaluationService};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cap_evaluation::{
    EvaluateParams, EvaluationErrorDetails, EvaluationProperty, EvaluationValue,
    EvaluationValueLimits, TypedArrayKind,
};
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpClient, CdpSession};
use pwright_core::{FailureKind, Service};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};

async fn dispose_context_and_verify_target(
    browser: &CdpSession,
    browser_context_id: &str,
    target_id: &str,
) {
    browser
        .target_dispose_browser_context(browser_context_id)
        .await
        .expect("owned browser context should dispose");
    for _ in 0..20 {
        let targets = browser
            .target_get_targets()
            .await
            .expect("target list should remain available");
        if !targets.iter().any(|target| target.target_id == target_id) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("temporary target {target_id} remained live after context disposal");
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn value_evaluation_reaches_real_chrome_without_creating_handles() {
    // Arrange one externally owned Chrome target and its exact target session.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .expect("provider should attach without taking process ownership");
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let browser_context_id = browser
        .target_create_browser_context(None, None)
        .await
        .expect("temporary browser context should open");
    let target_id = browser
        .target_create_in_context("about:blank", &browser_context_id)
        .await
        .expect("temporary target should open");
    let session_id = browser
        .target_attach(&target_id)
        .await
        .expect("temporary target should attach");
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    let service = CdpEvaluationService::new(Arc::clone(&page));

    // Act across expression, function-argument, Promise, and thrown-error paths.
    let cyclic = service
        .call(EvaluateParams::expression(
            "(() => { const x = { n: -0, bytes: new Uint16Array([258, 772]) }; x.self = x; return x; })()",
        ))
        .await
        .expect("cyclic special value should serialize");
    let promised = service
        .call(EvaluateParams::function(
            "async value => ({ answer: value + 1n, url: new URL('https://example.test/next') })",
            EvaluationValue::Bigint("9007199254740993".to_owned()),
        ))
        .await
        .expect("function argument and Promise should round trip");
    let omitted_argument = service
        .call(EvaluateParams::function_without_argument(
            "function(value) { return { isUndefined: value === undefined, count: arguments.length }; }",
        ))
        .await
        .expect("omitted argument should become one explicit undefined argument");
    let thrown = service
        .call(EvaluateParams::expression(
            "Promise.reject(new TypeError('expected boom'))",
        ))
        .await
        .expect_err("thrown JavaScript must stay distinct");
    let small_values = EvaluationValueLimits::new(4, 16, 16, 8, 8)
        .expect("small evaluation limits should validate");
    let bounded = CdpEvaluationService::with_limits(
        Arc::clone(&page),
        CdpEvaluationLimits::new(256, 32 * 1024, 256, small_values),
    )
    .call(EvaluateParams::expression("'x'.repeat(4096)"))
    .await
    .expect_err("browser-side serializer should reject an oversized value");
    page.page_navigate("data:text/html,<title>after-navigation</title>")
        .await
        .expect("real target should navigate");
    let after_navigation = service
        .call(EvaluateParams::expression("document.title"))
        .await
        .expect("same target session should evaluate after navigation");

    // Assert exact portable values, with no CDP objectId escaping the adapter.
    assert_eq!(
        cyclic.value,
        EvaluationValue::Object {
            id: NonZeroU32::new(1),
            properties: vec![
                EvaluationProperty::new("n", EvaluationValue::NegativeZero),
                EvaluationProperty::new(
                    "bytes",
                    EvaluationValue::TypedArray {
                        kind: TypedArrayKind::Uint16,
                        bytes: vec![2, 1, 4, 3],
                    },
                ),
                EvaluationProperty::new("self", EvaluationValue::Ref(NonZeroU32::new(1).unwrap()),),
            ],
        }
    );
    assert!(matches!(
        promised.value,
        EvaluationValue::Object { properties, .. }
            if properties[0].value == EvaluationValue::Bigint("9007199254740994".to_owned())
                && matches!(&properties[1].value, EvaluationValue::Url(url) if url.ends_with("/next"))
    ));
    assert!(matches!(
        omitted_argument.value,
        EvaluationValue::Object { properties, .. }
            if properties[0].value == EvaluationValue::Bool(true)
                && properties[1].value == EvaluationValue::number(1.0).unwrap()
    ));
    assert_eq!(thrown.kind(), FailureKind::Backend);
    assert!(matches!(
        thrown.details(),
        Some(EvaluationErrorDetails::Javascript { name, message, .. })
            if name == "TypeError" && message == "expected boom"
    ));
    assert_eq!(bounded.kind(), FailureKind::ResourceExhausted);
    assert!(matches!(
        bounded.details(),
        Some(EvaluationErrorDetails::InvalidValue(message))
            if message.contains("string-byte limit")
    ));
    assert_eq!(
        after_navigation.value,
        EvaluationValue::String("after-navigation".to_owned())
    );

    dispose_context_and_verify_target(&browser, &browser_context_id, &target_id).await;
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("external browser should remain alive");
}
