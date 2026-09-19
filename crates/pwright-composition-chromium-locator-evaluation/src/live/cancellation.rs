//! Protocol counterexample for NODE-027: a release acknowledgement retires
//! current wrappers, not an outstanding awaitPromise or its future exceptions.
use super::*;
use serde_json::json;
use std::task::Poll;

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_CDP_WS_URL"]
async fn releasing_a_group_does_not_retire_pending_by_value_evaluation() -> Result<()> {
    // Arrange: this test owns one Context; it never closes the browser or an
    // existing Page. A page-owned resolver supplies a causal gate, not a sleep.
    let endpoint = CdpWebSocketEndpoint::parse(std::env::var("PWRIGHT_CDP_WS_URL")?)?;
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await?;
    let connection = attachment.connection().clone();
    let browser = CdpSession::browser(connection.clone());
    let context = browser.target_create_browser_context(None, None).await?;
    let outcome = tokio::time::timeout(Duration::from_secs(10), async {
        let target = browser.target_create_in_context("about:blank", &context).await?;
        let session = browser.target_attach(&target).await?;
        let page = CdpSession::new(connection.clone(), session, target);
        let original = page.send("Runtime.evaluate", json!({
            "expression": "document.body", "objectGroup": "node027-proof"
        })).await?;
        let original_id = original["result"]["objectId"].as_str().context("original wrapper")?;
        let request = json!({
            "objectId": original_id,
            "functionDeclaration": "async function() { globalThis.__node027Calls = 1; await new Promise(resolve => { globalThis.__node027Resume = resolve; }); throw new TypeError('late native exception'); }",
            "returnByValue": true,
            "awaitPromise": true,
            "objectGroup": "node027-proof"
        });
        let mut pending = Box::pin(page.send("Runtime.callFunctionOn", request));
        std::future::poll_fn(|cx| {
            let state = pending.as_mut().poll(cx);
            Poll::Ready(if state.is_pending() { Ok(()) } else {
                Err(anyhow::anyhow!("evaluation did not stop at the causal gate"))
            })
        }).await?;
        let started = page.runtime_evaluate("globalThis.__node027Calls").await?;
        ensure!(started["result"]["value"] == 1, "native evaluation did not start");

        // Act: retire the group while the already-dispatched evaluation waits.
        page.send("Runtime.releaseObjectGroup", json!({"objectGroup": "node027-proof"})).await?;

        // Assert: the current wrapper is gone but the evaluation is not done.
        ensure!(page.runtime_call_function_on(original_id, "function(){return this.tagName}", vec![]).await.is_err());
        let still_pending = std::future::poll_fn(|cx| Poll::Ready(pending.as_mut().poll(cx).is_pending())).await;
        ensure!(still_pending, "group release unexpectedly settled evaluation");
        page.runtime_evaluate("globalThis.__node027Resume(); delete globalThis.__node027Resume").await?;
        let late = pending.await?;
        let exception_id = late["exceptionDetails"]["exception"]["objectId"].as_str().context("late exception RemoteObject despite returnByValue")?;
        let message = page.runtime_call_function_on(exception_id, "function(){return this.message}", vec![]).await?;
        ensure!(message["result"]["value"] == "late native exception");
        let calls = page.runtime_evaluate("globalThis.__node027Calls").await?;
        ensure!(calls["result"]["value"] == 1, "user code was replayed");

        // The late wrapper needs a second acknowledgement. This test deliberately
        // knows the result; a cancelled receiver cannot rely on that knowledge.
        page.send("Runtime.releaseObjectGroup", json!({"objectGroup": "node027-proof"})).await?;
        ensure!(page.runtime_call_function_on(exception_id, "function(){return this.message}", vec![]).await.is_err());
        Ok::<_, anyhow::Error>(())
    }).await.context("native cancellation counterexample deadline");
    // Dispose only this test's Context even when the counterexample fails.
    let disposed = browser.target_dispose_browser_context(&context).await;
    disposed?;
    outcome??;
    connection
        .send_command("Browser.getVersion", json!({}))
        .await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_CDP_WS_URL"]
async fn explicit_owned_session_detach_preserves_the_page_and_user_promise() -> Result<()> {
    // Arrange: the fixture owns a Context and Page, but the evaluation session
    // only borrows that Page. A second session independently observes survival.
    let endpoint = CdpWebSocketEndpoint::parse(std::env::var("PWRIGHT_CDP_WS_URL")?)?;
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await?;
    let connection = attachment.connection().clone();
    let browser = CdpSession::browser(connection.clone());
    let context = browser.target_create_browser_context(None, None).await?;
    let outcome = tokio::time::timeout(Duration::from_secs(10), async {
        let target = browser.target_create_in_context("about:blank", &context).await?;
        let owned_session = browser.target_attach(&target).await?;
        let observer_session = browser.target_attach(&target).await?;
        let page = CdpSession::new(connection.clone(), owned_session.clone(), target.clone());
        let observer = CdpSession::new(connection.clone(), observer_session, target);
        let mut pending = Box::pin(page.send("Runtime.evaluate", json!({
            "expression": "(async () => { globalThis.__node027Calls = 1; await new Promise(resolve => { globalThis.__node027Resume = resolve; }); globalThis.__node027Calls++; return 42; })()",
            "awaitPromise": true, "returnByValue": true, "objectGroup": "node027-detach"
        })));
        std::future::poll_fn(|cx| Poll::Ready(if pending.as_mut().poll(cx).is_pending() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("native evaluation did not wait"))
        })).await?;
        // Same-session FIFO establishes dispatch before observing via another
        // session; no elapsed-time assertion or sleep is used as readiness.
        let started = page.runtime_evaluate("globalThis.__node027Calls").await?;
        ensure!(started["result"]["value"] == 1);

        // Act: acknowledge detachment of the exact session we created, not a
        // generic detach event, Page close, Context close or transport failure.
        browser.target_detach(&owned_session).await?;

        // Assert: the borrowed target and its JavaScript remain usable. Detach
        // is not a promise abort. The future's drop below is not a production
        // cleanup receipt and makes no assertion about physical V8 heap usage.
        ensure!(page.runtime_evaluate("42").await.is_err());
        let alive = observer.runtime_evaluate("globalThis.__node027Calls").await?;
        ensure!(alive["result"]["value"] == 1);
        observer.runtime_evaluate("globalThis.__node027Resume(); delete globalThis.__node027Resume").await?;
        let resumed = observer.runtime_evaluate("globalThis.__node027Calls").await?;
        ensure!(resumed["result"]["value"] == 2, "user Promise did not survive owned session detach");
        drop(pending);
        Ok::<_, anyhow::Error>(())
    }).await.context("owned session detachment boundary deadline");
    let disposed = browser.target_dispose_browser_context(&context).await;
    disposed?;
    outcome??;
    connection
        .send_command("Browser.getVersion", json!({}))
        .await?;
    Ok(())
}
