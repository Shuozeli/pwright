//! Real Chrome ownership checks at the same raw-response helper used by the
//! production service. Service wiring and transport settlement have separate
//! deterministic tests; these assertions prove the actual IDs become unusable.

use std::{future::Future, sync::Arc, time::Duration};

use pwright_cap_handle_evaluation::HandleEvaluationErrorDetails;
use pwright_cdp::{
    CdpClient, CdpSession,
    connection::{CdpConnection, CdpError},
};
use pwright_core::FailureKind;

use super::{CdpHandleEvaluationLimits, acquisition};

async fn with_owned_page<F, Work>(exercise: F)
where
    F: FnOnce(Arc<CdpSession>) -> Work + Send + 'static,
    Work: Future<Output = ()> + Send + 'static,
{
    let endpoint = std::env::var("PWRIGHT_CDP_WS_URL")
        .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile");
    let connection = CdpConnection::connect(&endpoint).await.unwrap();
    let browser = CdpSession::browser(Arc::clone(&connection));
    let context_id = browser
        .target_create_browser_context(None, None)
        .await
        .unwrap();
    let exercise_connection = Arc::clone(&connection);
    let exercise_context = context_id.clone();
    let mut work = tokio::spawn(async move {
        let browser = CdpSession::browser(Arc::clone(&exercise_connection));
        let target = browser
            .target_create_in_context("about:blank", &exercise_context)
            .await
            .unwrap();
        let session = browser.target_attach(&target).await.unwrap();
        exercise(Arc::new(CdpSession::new(
            exercise_connection,
            session,
            target,
        )))
        .await;
    });
    let outcome = tokio::time::timeout(Duration::from_secs(30), &mut work).await;
    if outcome.is_err() {
        work.abort();
        let _ = work.await;
    }
    // Keep Context ownership outside the spawned assertions so panic and
    // timeout still reach cleanup. The externally owned Chrome is never closed.
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
    let io_finished = connection.io_finished();
    connection.request_disconnect();
    let disconnected = tokio::time::timeout(Duration::from_secs(10), io_finished).await;
    assert!(
        matches!(cleanup, Ok(Ok(()))),
        "owned Context cleanup failed: {cleanup:?}; exercise: {outcome:?}"
    );
    assert!(
        matches!(browser_alive, Ok(Ok(_))),
        "external Chrome stopped responding: {browser_alive:?}"
    );
    assert!(disconnected.is_ok(), "test-owned CDP I/O did not stop");
    assert!(
        matches!(outcome, Ok(Ok(()))),
        "native ownership exercise failed or exceeded 30 seconds: {outcome:?}"
    );
}

async fn assert_usable(page: &CdpSession, id: &str) {
    let value = page
        .runtime_call_function_on(id, "function(){return true}", vec![])
        .await
        .unwrap();
    assert_eq!(value["result"]["value"], true);
}

async fn assert_released(page: &CdpSession, id: &str) {
    let result = page.runtime_get_properties(id).await;
    assert!(
        matches!(&result, Err(CdpError::Protocol { code: -32000, message })
            if message.contains("Could not find object with given id")),
        "the same formerly usable native ID must now be invalid, not a transport/session failure: {result:?}",
    );
}

#[tokio::test]
#[ignore = "requires dedicated Chrome CDP and PWRIGHT_CDP_WS_URL"]
async fn malformed_actual_remote_result_releases_its_native_id() {
    with_owned_page(|page| async move {
        // Arrange: acquire a real retained object, then corrupt only its local
        // response type. No fake ID, release method, or parser is substituted.
        let mut response = page
            .runtime_evaluate_as_object_to_settlement("({marker: 'malformed-live'})")
            .await
            .unwrap();
        let id = response["result"]["objectId"].as_str().unwrap().to_owned();
        assert_usable(&page, &id).await;
        response["result"].as_object_mut().unwrap().remove("type");
        let client: Arc<dyn CdpClient> = page.clone();

        // Act
        let error = acquisition::finish(client, response, CdpHandleEvaluationLimits::default())
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_released(&page, &id).await;
    })
    .await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome CDP and PWRIGHT_CDP_WS_URL"]
async fn actual_javascript_exception_releases_every_returned_native_id() {
    with_owned_page(|page| async move {
        // Arrange: keep both actual result/exception IDs, deduplicating only if
        // Chrome returned the same identity in both locations.
        let response = page
            .runtime_evaluate_as_object_to_settlement(
                "(() => { throw new Error('acquisition-live-marker'); })()",
            )
            .await
            .unwrap();
        let mut ids = vec![
            response["result"]["objectId"].as_str().unwrap().to_owned(),
            response["exceptionDetails"]["exception"]["objectId"]
                .as_str()
                .unwrap()
                .to_owned(),
        ];
        ids.sort();
        ids.dedup();
        for id in &ids {
            assert_usable(&page, id).await;
        }
        let client: Arc<dyn CdpClient> = page.clone();

        // Act
        let error = acquisition::finish(client, response, CdpHandleEvaluationLimits::default())
            .await
            .unwrap_err();

        // Assert
        assert!(
            matches!(error.details(), Some(HandleEvaluationErrorDetails::Javascript { message, .. })
            if message.contains("acquisition-live-marker"))
        );
        for id in &ids {
            assert_released(&page, id).await;
        }
    })
    .await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome CDP and PWRIGHT_CDP_WS_URL"]
async fn valid_actual_remote_result_remains_owned_until_explicit_release() {
    with_owned_page(|page| async move {
        // Arrange
        let response = page
            .runtime_evaluate_as_object_to_settlement("({marker: 'retained-live'})")
            .await
            .unwrap();
        let id = response["result"]["objectId"].as_str().unwrap().to_owned();
        let client: Arc<dyn CdpClient> = page.clone();

        // Act
        let handle = acquisition::finish(client, response, CdpHandleEvaluationLimits::default())
            .await
            .unwrap();

        // Assert: successful parsing must not perform failure compensation.
        assert_eq!(handle.object_id(), Some(id.as_str()));
        assert_usable(&page, &id).await;
        handle.release().await.unwrap();
        assert_released(&page, &id).await;
    })
    .await;
}
