use super::*;
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cdp::{CdpSession, connection::ConnectionConfig};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use std::time::Duration;

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_CDP_WS_URL"]
async fn real_script_runs_once_while_local_publication_is_deferred() {
    // Arrange: use the real Chrome Frame route with the production-ledger
    // fixture. The unrelated local Page gate is an explicit deterministic seam,
    // not a claim of racing real Chrome Page-close acknowledgements.
    let endpoint =
        CdpWebSocketEndpoint::parse(std::env::var("PWRIGHT_CDP_WS_URL").unwrap()).unwrap();
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .unwrap();
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let context = browser
        .target_create_browser_context(None, None)
        .await
        .unwrap();
    let outcome: Result<(), Box<dyn std::error::Error>> = async {
        let target = browser.target_create_in_context("about:blank", &context).await?;
        let session = browser.target_attach(&target).await?;
        let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(Arc::clone(&connection), session, target));
        let mut events = page.subscribe_events();
        page.runtime_enable().await?;
        let (numeric, unique) = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let event = events.recv().await.unwrap();
                if event.method == "Runtime.executionContextCreated"
                    && event.params["context"]["auxData"]["isDefault"].as_bool() == Some(true)
                    && let Some(numeric) = event.params["context"]["id"].as_i64()
                    && let Some(unique) = event.params["context"]["uniqueId"].as_str()
                { break (numeric, unique.to_owned()); }
            }
        }).await?;
        let fixture = fixture(1);
        let routes = fixture.installation.routes();
        routes.unbind(ObjectId::new(4))?;
        routes.bind(ObjectId::new(4), ObjectId::new(3), page.clone(), numeric, unique)?;
        let unrelated = fixture.runtime.prepare_object_disposal(ObjectId::new(5))?;
        let service = fixture.installation.raw_service();
        let pending = service.stage_evaluate_handle(ObjectId::new(4), CallMetadata::default(),
            EvaluateHandleParams::expression("globalThis.node158Runs = (globalThis.node158Runs || 0) + 1; ({ answer: 42 })")
        ).await?;
        ensure(fixture.installation.lifecycle().routes().metrics()?.current == 0, "native result was published before commit")?;

        // Act: two deferred local attempts must not repeat the native script.
        let NativeHandleCreationCommit::Deferred(pending) = pending.commit_or_defer() else { return Err("expected busy publication".into()) };
        let NativeHandleCreationCommit::Deferred(pending) = pending.commit_or_defer() else { return Err("expected retained busy publication".into()) };
        drop(unrelated);
        let NativeHandleCreationCommit::Published(handle) = pending.commit_or_defer() else { return Err("expected publication after gate release".into()) };

        // Assert through the public retained-Handle command and real browser.
        let value = service.call(Command::<HandleEvaluate>::new(CallId::new(51), handle.object_id,
            CallMetadata::default(), HandleEvaluateParams::function_without_argument("target => target.answer === 42")
        )).await.unwrap().into_outcome()?;
        ensure(value.value == EvaluationValue::Bool(true), "published Handle returned the wrong value")?;
        let counter = page.runtime_evaluate("globalThis.node158Runs").await?;
        ensure(counter["result"]["value"] == 1, "native script did not execute exactly once")?;
        fixture.installation.lifecycle().raw_dispose_service().call(Command::<HandleDispose>::new(
            CallId::new(52), handle.object_id, CallMetadata::default(), ()
        )).await.unwrap().into_outcome()?;
        ensure(fixture.installation.lifecycle().routes().metrics()?.current == 0, "explicit disposal did not return route capacity")?;
        eprintln!("NODE158 real Chrome: script count=1, two deferred local commits, published value=true, explicit disposal returned route capacity");
        Ok(())
    }.await;
    let cleanup = browser.target_dispose_browser_context(&context).await;
    outcome.unwrap();
    cleanup.unwrap();
}

fn ensure(condition: bool, message: &'static str) -> Result<(), Box<dyn std::error::Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
