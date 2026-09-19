use anyhow::{Context, Result, ensure};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cdp::{CdpClient, CdpEvent, CdpSession, connection::ConnectionConfig};
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionError, ResolutionProfile,
    ResolutionRoute, ResolutionRouteSource, ResolutionWaitError, ResolutionWaitProfile,
    ResolutionWaitStop, StrictResolutionWaiter,
};
use pwright_core::{
    AtomicObjectIdSource, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope,
};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use pwright_runtime::{CleanupQueue, DeadlineScheduler};
use pwright_selector_model::Selector;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::Notify;

// The test drives authoritative world replacement from the actual CDP events.
// This single-Frame fixture is not the production topology adapter.
struct Routes {
    current: Mutex<Option<ResolutionRoute>>,
    changed: Notify,
    closed: AtomicBool,
}
impl ResolutionRouteSource for Routes {
    type Current<'a> =
        Pin<Box<dyn Future<Output = Result<ResolutionRoute, ResolutionWaitError>> + Send + 'a>>;
    fn current(&self) -> Self::Current<'_> {
        Box::pin(async {
            loop {
                let changed = self.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                if self.closed.load(Ordering::Acquire) {
                    return Err(ResolutionWaitStop::TargetClosed.into());
                }
                if let Some(route) = self.current.lock().unwrap().clone() {
                    return Ok(route);
                }
                changed.await;
            }
        })
    }
    fn is_current(&self, route: &ResolutionRoute) -> Result<bool, ResolutionWaitError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(ResolutionWaitStop::TargetClosed.into());
        }
        Ok(self
            .current
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|current| {
                Arc::ptr_eq(&current.client, &route.client)
                    && current.utility_world == route.utility_world
            }))
    }
}
#[derive(Default)]
struct Pacing {
    sleeping: Notify,
}
impl DeadlineScheduler for Pacing {
    type Sleep<'a> = tokio::time::Sleep;
    fn sleep(&self, budget: Duration) -> Option<Self::Sleep<'_>> {
        self.sleeping.notify_one();
        Some(tokio::time::sleep(budget))
    }
}
async fn deadline(ms: u64) -> ResolutionWaitStop {
    tokio::time::sleep(Duration::from_millis(ms)).await;
    ResolutionWaitStop::Timeout
}
async fn world(
    events: &mut tokio::sync::broadcast::Receiver<CdpEvent>,
    numeric: Option<i64>,
) -> Result<Arc<str>> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let event = events.recv().await?;
            if event.method != "Runtime.executionContextCreated" {
                continue;
            }
            let ctx = &event.params["context"];
            if numeric.map_or_else(
                || ctx["auxData"]["isDefault"].as_bool() == Some(true),
                |id| ctx["id"].as_i64() == Some(id),
            ) {
                return Ok(ctx["uniqueId"]
                    .as_str()
                    .context("missing unique context")?
                    .into());
            }
        }
    })
    .await
    .context("world readiness timeout")?
}

#[tokio::test]
#[ignore = "requires dedicated Chrome CDP preflight and PWRIGHT_CDP_WS_URL"]
async fn strict_wait_handles_delayed_hidden_elements_replacement_and_one_deadline() -> Result<()> {
    // Arrange: the caller owns Chrome attachment, one disposable Context, exact
    // production budgets, a bounded cleanup queue and the clock implementation.
    let endpoint = CdpWebSocketEndpoint::parse(std::env::var("PWRIGHT_CDP_WS_URL")?)?;
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await?;
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(connection.clone());
    let context_id = browser.target_create_browser_context(None, None).await?;
    let resolution = ResolutionProfile::new(ResourceScope::extension(90, 1), 1)?;
    let waiting = ResolutionWaitProfile::new(
        ResourceScope::extension(90, 2),
        1,
        [0, 20, 50, 100, 100, 500].map(Duration::from_millis),
    )?;
    let required = resolution
        .requirements()
        .combine_concurrent(waiting.requirements());
    let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
    for class in ResourceClass::ALL {
        let bound = required.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, required)?;
    let raw = Arc::new(CdpLocatorResolutionService::new(
        resolution,
        &resources,
        Arc::new(AtomicObjectIdSource::new(1)?),
    )?);
    let queue = CleanupQueue::new(1);
    let attempts = ContinuableResolutionService::new(raw.clone(), queue.clone());
    let waiter = StrictResolutionWaiter::new(waiting, &resources)?;
    let outcome = tokio::time::timeout(Duration::from_secs(15), async {
        let target = browser.target_create_in_context("about:blank", &context_id).await?;
        let session = browser.target_attach(&target).await?;
        let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(connection.clone(), session, target.clone()));
        let mut events = page.subscribe_events();
        page.runtime_enable().await?;
        let main = world(&mut events, None).await?;
        let frame = page.page_get_frame_tree().await?.frame.id;
        let numeric = page.page_create_isolated_world(&frame, "__pwright_wait_live__", false).await?;
        let utility = world(&mut events, Some(numeric)).await?;
        let routes = Routes { current: Mutex::new(Some(ResolutionRoute { client: page.clone(), utility_world: utility, destruction: None })), changed: Notify::new(), closed: AtomicBool::new(false) };
        let pacing = Pacing::default();
        page.runtime_evaluate_in_unique_context(
            "Document.prototype.querySelectorAll = () => { throw new Error('main override'); }; setTimeout(() => { document.body.innerHTML = '<button hidden disabled id=late>Late</button>'; }, 120)", &main).await?;

        // Act: attached means neither visible nor enabled. Holding the result
        // through detachment must preserve identity without another query.
        let element = waiter.wait(Selector::css("#late")?, &attempts, &routes, &pacing, deadline(2000)).await?;
        let native = element.element().object_id().to_owned();
        let before = page.runtime_call_function_on(&native, "function(){return [this.id,this.hidden,this.disabled,this.isConnected]}", Vec::new()).await?;
        page.runtime_evaluate_in_unique_context("document.getElementById('late').remove()", &main).await?;
        let detached = page.runtime_call_function_on(&native, "function(){return [this.id,this.isConnected]}", Vec::new()).await?;
        element.release().await?;
        let released = page.runtime_call_function_on(&native, "function(){return this.id}", Vec::new()).await.is_err();

        // A replacement is introduced after an actual missing attempt. Both
        // futures are caller-polled; no background task/browser is installed.
        let navigation_pacing = Pacing::default();
        let selecting = waiter.wait(Selector::css("#replacement")?, &attempts, &routes, &navigation_pacing, deadline(3000));
        let navigate = async {
            navigation_pacing.sleeping.notified().await;
            routes.current.lock().unwrap().take();
            page.page_navigate("data:text/html,<button id=replacement>New document</button>").await?;
            let next_main = world(&mut events, None).await?;
            let frame = page.page_get_frame_tree().await?.frame.id;
            let numeric = page.page_create_isolated_world(&frame, "__pwright_wait_live__", false).await?;
            let utility = world(&mut events, Some(numeric)).await?;
            *routes.current.lock().unwrap() = Some(ResolutionRoute { client: page.clone(), utility_world: utility, destruction: None });
            routes.changed.notify_waiters();
            Ok::<_, anyhow::Error>(next_main)
        };
        let (selected, navigated) = tokio::join!(selecting, navigate);
        let next_main = navigated?;
        let replacement = selected?;
        let new_id = page.runtime_call_function_on(replacement.element().object_id(), "function(){return this.id}", Vec::new()).await?;
        replacement.release().await?;

        let timeout = waiter.wait(Selector::css("#never")?, &attempts, &routes, &pacing, deadline(100)).await;
        while let Some(job) = queue.try_take()? { job.await?; }
        page.runtime_evaluate_in_unique_context("document.body.innerHTML = '<button>One</button><button>Two</button>'", &next_main).await?;
        let strict = waiter.wait(Selector::css("button")?, &attempts, &routes, &pacing, deadline(1000)).await;
        let close_pacing = Pacing::default();
        let closing = waiter.wait(Selector::css("#absent")?, &attempts, &routes, &close_pacing, deadline(1000));
        let close = async {
            close_pacing.sleeping.notified().await;
            routes.closed.store(true, Ordering::Release);
            routes.changed.notify_waiters();
            browser.target_close(&target).await
        };
        let (closed, physically_closed) = tokio::join!(closing, close);
        physically_closed?;
        Ok::<_, anyhow::Error>((before, detached, released, new_id,
            matches!(timeout, Err(ResolutionWaitError::Stopped(ResolutionWaitStop::Timeout))),
            matches!(strict, Err(ResolutionWaitError::Resolution(ResolutionError::StrictViolation { matches: 2 }))),
            matches!(closed, Err(ResolutionWaitError::Stopped(ResolutionWaitStop::TargetClosed)))))
    }).await.context("live strict wait overall deadline");
    // Always attempt owned Context cleanup before asserting or propagating an
    // operational failure, even if continuation cleanup itself reports failure.
    let cleanup: Result<()> = async {
        while let Some(job) = queue.try_take()? {
            job.await?;
        }
        Ok(())
    }
    .await;
    let disposed = browser.target_dispose_browser_context(&context_id).await;
    drop(waiter);
    cleanup?;
    disposed?;
    let (before, detached, released, new_id, timeout, strict, closed) = outcome??;
    // Assert after all test-owned pages and temporary groups are gone.
    ensure!(before["result"]["value"] == serde_json::json!(["late", true, true, true]));
    ensure!(detached["result"]["value"] == serde_json::json!(["late", false]));
    ensure!(new_id["result"]["value"] == "replacement");
    ensure!(released && timeout && strict && closed);
    ensure!(resources.metrics()?.active_leases == 0);
    ensure!(queue.metrics()?.reserved == 0 && queue.metrics()?.queued == 0);
    ensure!(raw.is_cleanup_healthy());
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await?;
    Ok(())
}
