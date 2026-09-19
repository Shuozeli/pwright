use super::*;
use pwright_composition_chromium_locator_resolution::ObjectGroupDestructionProof;
use serde_json::json;
use std::{
    sync::Mutex,
    task::{Context as TaskContext, Poll, Waker},
};

#[derive(Clone)]
struct AcknowledgedSession {
    state: Arc<Mutex<(bool, Option<Waker>)>>,
    client: Arc<dyn CdpClient>,
    world: Arc<str>,
}
impl AcknowledgedSession {
    fn acknowledge(&self) {
        let waiter = {
            let mut state = self.state.lock().unwrap();
            state.0 = true;
            state.1.take()
        };
        if let Some(waiter) = waiter {
            waiter.wake();
        }
    }
}
impl ObjectGroupDestructionProof for AcknowledgedSession {
    fn all_group_worlds_destroyed(&self) -> bool {
        false
    }
    fn all_group_objects_retired(&self) -> bool {
        self.state.lock().unwrap().0
    }
}
impl NativeEvaluationRetirement for AcknowledgedSession {
    fn poll_retired(
        &mut self,
        client: &Arc<dyn CdpClient>,
        world: &str,
        cx: &mut TaskContext<'_>,
    ) -> Poll<()> {
        if !Arc::ptr_eq(client, &self.client) || world != self.world.as_ref() {
            return Poll::Pending;
        }
        let mut state = self.state.lock().unwrap();
        if state.0 {
            Poll::Ready(())
        } else {
            state.1 = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_CDP_WS_URL"]
async fn acknowledged_session_retirement_drains_locator_cleanup_without_closing_borrowed_page()
-> Result<()> {
    // Arrange: the fixture owns a Context, but the evaluation session borrows
    // its Page. No producer shares this session and only this task can dispatch
    // into it, making the explicit dispatch fence below structural.
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
        let observer_id = browser.target_attach(&target).await?;
        let observer = CdpSession::new(connection.clone(), observer_id, target.clone());
        let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(connection.clone(), session.clone(), target));
        let mut events = page.subscribe_events();
        page.runtime_enable().await?;
        let (numeric, main) = world(&mut events, None).await?;
        let frame = page.page_get_frame_tree().await?.frame.id;
        let utility_numeric = page.page_create_isolated_world(&frame, "__node027_retirement__", false).await?;
        let (_, utility) = world(&mut events, Some(utility_numeric)).await?;
        let proof = AcknowledgedSession { state: Arc::new(Mutex::new((false, None))), client: page.clone(), world: main.clone() };
        let resolution = ResolutionProfile::new(ResourceScope::extension(92, 3), 1)?;
        let evaluation = ElementEvaluationProfile::new(ResourceScope::extension(92, 4), 1, tests::limits())?;
        let resources = tests::resources(resolution.requirements().combine_concurrent(evaluation.requirements()));
        let queue = CleanupQueue::new(2);
        let resolver = ContinuableResolutionService::new(Arc::new(CdpLocatorResolutionService::new(resolution, &resources, Arc::new(AtomicObjectIdSource::new(1)?))?), queue.clone());
        let service = LocatorElementEvaluationService::new(evaluation, &resources, queue.clone())?;
        let source = resolver.call(ResolutionRequest { client: page.clone(), utility_world: utility, selector: Selector::css("body")?, destruction: Some(Arc::new(proof.clone())) }).await?.context("body not found")?;
        let element = service.adopt(source, MainWorldRoute { client: page.clone(), execution_context_id: numeric, unique_context_id: main.clone() }).await?;
        let mut pending = Box::pin(element.evaluate_with_retirement(&EvaluateParams::function_without_argument("async element => { globalThis.__node027Calls=1; await new Promise(resolve => { globalThis.__node027Resume=resolve; }); globalThis.__node027Calls++; return element.tagName; }"), Box::new(proof.clone())));
        std::future::poll_fn(|cx| Poll::Ready(if pending.as_mut().poll(cx).is_pending() { Ok(()) } else { Err(anyhow::anyhow!("native evaluation did not wait")) })).await?;
        let started = page.runtime_evaluate_in_unique_context("globalThis.__node027Calls", &main).await?;
        ensure!(started["result"]["value"] == 1);

        // Act: cancellation transfers the same work; then this exclusive owner
        // fences all further dispatch and awaits the exact session detach ack.
        drop(pending);
        browser.target_detach(&session).await?;
        proof.acknowledge();
        while let Some(job) = queue.try_take()? { job.await?; }

        // Assert before fixture teardown: the user Promise was not resolved to
        // make cleanup pass, and the borrowed Page/Context still works.
        ensure!(service.is_cleanup_healthy());
        ensure!(!proof.all_group_worlds_destroyed());
        ensure!(resources.metrics()?.active_leases == 1);
        ensure!(queue.metrics()?.reserved == 0 && queue.metrics()?.queued == 0);
        let waiting = observer.runtime_evaluate("globalThis.__node027Calls").await?;
        ensure!(waiting["result"]["value"] == 1);
        observer.runtime_evaluate("globalThis.__node027Resume(); delete globalThis.__node027Resume").await?;
        let resumed = observer.runtime_evaluate("globalThis.__node027Calls").await?;
        ensure!(resumed["result"]["value"] == 2);
        drop(service);
        ensure!(resources.metrics()?.active_leases == 0);
        Ok::<_, anyhow::Error>(())
    }).await.context("receipt-driven Locator cleanup deadline");
    let disposed = browser.target_dispose_browser_context(&context).await;
    disposed?;
    outcome??;
    connection
        .send_command("Browser.getVersion", json!({}))
        .await?;
    Ok(())
}
