use super::*;
use anyhow::{Context, Result, ensure};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cap_evaluation::{EvaluationProperty, TypedArrayKind};
use pwright_cdp::{CdpEvent, CdpSession, connection::ConnectionConfig};
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionProfile, ResolutionRequest,
};
use pwright_core::{AtomicObjectIdSource, ResourceLimits};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use pwright_runtime::CleanupQueue;
use pwright_selector_model::Selector;
use std::{future::Future, pin::Pin, time::Duration};

mod cancellation;
mod retirement;

async fn world(
    events: &mut tokio::sync::broadcast::Receiver<CdpEvent>,
    numeric: Option<i64>,
) -> Result<(i64, Arc<str>)> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let event = events.recv().await?;
            if event.method != "Runtime.executionContextCreated" {
                continue;
            }
            let context = &event.params["context"];
            if numeric.map_or_else(
                || context["auxData"]["isDefault"].as_bool() == Some(true),
                |id| context["id"].as_i64() == Some(id),
            ) {
                return Ok((
                    context["id"].as_i64().context("numeric context missing")?,
                    context["uniqueId"]
                        .as_str()
                        .context("unique context missing")?
                        .into(),
                ));
            }
        }
    })
    .await
    .context("execution context readiness deadline")?
}
fn property<'a>(value: &'a EvaluationValue, key: &str) -> Result<&'a EvaluationValue> {
    let EvaluationValue::Object { properties, .. } = value else {
        anyhow::bail!("expected value object");
    };
    properties
        .iter()
        .find(|property| property.key == key)
        .map(|property| &property.value)
        .context("missing result property")
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_CDP_WS_URL"]
async fn selected_element_evaluates_once_in_main_world_with_detachment_and_group_cleanup()
-> Result<()> {
    // Arrange: attach only; own one Context, two temporary native wrappers,
    // admitted value evaluation and two caller-driven cleanup reservations.
    let endpoint = CdpWebSocketEndpoint::parse(std::env::var("PWRIGHT_CDP_WS_URL")?)?;
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await?;
    let connection = attachment.connection().clone();
    let browser = CdpSession::browser(connection.clone());
    let context = browser.target_create_browser_context(None, None).await?;
    let resolution = ResolutionProfile::new(ResourceScope::extension(92, 1), 1)?;
    let evaluation =
        ElementEvaluationProfile::new(ResourceScope::extension(92, 2), 1, tests::limits())?;
    let required = resolution
        .requirements()
        .combine_concurrent(evaluation.requirements());
    let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
    for class in ResourceClass::ALL {
        let bound = required.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, required)?;
    let queue = CleanupQueue::new(2);
    let resolver = ContinuableResolutionService::new(
        Arc::new(CdpLocatorResolutionService::new(
            resolution,
            &resources,
            Arc::new(AtomicObjectIdSource::new(1)?),
        )?),
        queue.clone(),
    );
    let service = LocatorElementEvaluationService::new(evaluation, &resources, queue.clone())?;
    let outcome = tokio::time::timeout(Duration::from_secs(15), async {
        let target = browser.target_create_in_context("about:blank", &context).await?;
        let session = browser.target_attach(&target).await?;
        let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(connection.clone(), session, target));
        let mut events = page.subscribe_events(); page.runtime_enable().await?;
        let (main_numeric, main) = world(&mut events, None).await?;
        let frame = page.page_get_frame_tree().await?.frame.id;
        let utility_numeric = page.page_create_isolated_world(&frame, "__pwright_element_eval_live__", false).await?;
        let (_, utility) = world(&mut events, Some(utility_numeric)).await?;
        page.runtime_evaluate_in_unique_context("document.body.innerHTML='<label for=email>Email</label><input id=email value=original>'; globalThis.__pwMain='main'; globalThis.__pwCalls=0; Document.prototype.querySelectorAll=()=>{throw new Error('main query overridden')}", &main).await?;
        let select = || resolver.call(ResolutionRequest { client: page.clone(), utility_world: utility.clone(), selector: Selector::label("Email", true), destruction: None });
        let route = || MainWorldRoute { client: page.clone(), execution_context_id: main_numeric, unique_context_id: main.clone() };

        // Act: acquire/adopt first, then detach and replace the DOM node before
        // source/argument conversion. Evaluation must receive the original node.
        let source = select().await?.context("label did not resolve")?;
        let utility_id = source.element().object_id().to_owned();
        let element = service.adopt(source, route()).await?;
        let main_id = element.data.as_ref().unwrap().adopted.as_ref().unwrap().handle().object_id().unwrap().to_owned();
        page.runtime_evaluate_in_unique_context("document.getElementById('email').remove(); const replacement=document.createElement('input'); replacement.id='email'; replacement.value='replacement'; document.body.appendChild(replacement)", &main).await?;
        let argument = EvaluationValue::Object { id: None, properties: vec![
            EvaluationProperty::new("large", EvaluationValue::Bigint("1234567890123456789012345".into())),
            EvaluationProperty::new("bytes", EvaluationValue::TypedArray { kind: TypedArrayKind::Uint8, bytes: vec![7, 8, 9] }),
        ]};
        let value = element.evaluate(&EvaluateParams::function(
            "async (element, argument) => { globalThis.__pwCalls++; await new Promise(resolve=>setTimeout(resolve,60)); return { id:element.id, value:element.value, connected:element.isConnected, main:globalThis.__pwMain, calls:globalThis.__pwCalls, large:argument.large, bytes:argument.bytes }; }", argument)).await?;
        let utility_released = page.runtime_call_function_on(&utility_id, "function(){return this.id}", Vec::new()).await.is_err();
        let main_released = page.runtime_call_function_on(&main_id, "function(){return this.id}", Vec::new()).await.is_err();

        let throws = service.adopt(select().await?.context("replacement label")?, route()).await?;
        let error = throws.evaluate(&EvaluateParams::function_without_argument("element => { globalThis.__pwCalls++; throw new TypeError('locator main error'); }")).await;
        let error_matches = matches!(error, Err(ElementEvaluationError::Backend(error)) if matches!(error.details(), Some(HandleEvaluationErrorDetails::Javascript { name, message, .. }) if name == "TypeError" && message == "locator main error"));

        let unused = service.adopt(select().await?.context("unused label")?, route()).await?;
        let unpolled = unused.evaluate(&EvaluateParams::function_without_argument("element => { globalThis.__pwCalls += 100; }"));
        drop(unpolled);
        while let Some(job) = queue.try_take()? { job.await?; }

        let cancelled = service.adopt(select().await?.context("cancelled label")?, route()).await?;
        let cancelled_id = cancelled.data.as_ref().unwrap().adopted.as_ref().unwrap().handle().object_id().unwrap().to_owned();
        let mut evaluation = Box::pin(cancelled.evaluate(&EvaluateParams::function_without_argument("async element => { globalThis.__pwCalls++; await new Promise(resolve=>setTimeout(resolve,100)); return element.value; }")));
        // One poll dispatches Runtime.callFunctionOn; cancellation hands that
        // very future to the queue, which the test itself then drives.
        std::future::poll_fn(|cx| {
            if Pin::new(&mut evaluation).poll(cx).is_ready() {
                return std::task::Poll::Ready(Err(anyhow::anyhow!("expected pending native evaluation")));
            }
            std::task::Poll::Ready(Ok::<_, anyhow::Error>(()))
        }).await?;
        let before_cancel = page.runtime_evaluate_in_unique_context("globalThis.__pwCalls", &main).await?;
        ensure!(before_cancel["result"]["value"] == 3, "user function was not dispatched before cancellation");
        drop(evaluation);
        while let Some(job) = queue.try_take()? { job.await?; }
        let cancelled_released = page.runtime_call_function_on(&cancelled_id, "function(){return this.id}", Vec::new()).await.is_err();
        let count = page.runtime_evaluate_in_unique_context("globalThis.__pwCalls", &main).await?;
        Ok::<_, anyhow::Error>((value, utility_released, main_released, error_matches, cancelled_released, count))
    }).await.context("element evaluation overall deadline");
    let cleanup = tokio::time::timeout(Duration::from_secs(3), async {
        while let Some(job) = queue.try_take()? {
            job.await?;
        }
        Ok::<_, anyhow::Error>(())
    })
    .await
    .context("bounded native cleanup deadline");
    let disposed = browser.target_dispose_browser_context(&context).await;
    let healthy = service.is_cleanup_healthy();
    drop(service);
    cleanup??;
    disposed?;
    let (value, utility_released, main_released, error_matches, cancelled_released, count) =
        outcome??;
    // Assert after owned native targets and temporary leases are gone.
    ensure!(property(&value, "id")? == &EvaluationValue::String("email".into()));
    ensure!(property(&value, "value")? == &EvaluationValue::String("original".into()));
    ensure!(property(&value, "connected")? == &EvaluationValue::Bool(false));
    ensure!(property(&value, "main")? == &EvaluationValue::String("main".into()));
    ensure!(property(&value, "calls")? == &EvaluationValue::number(1.0)?);
    ensure!(
        property(&value, "large")? == &EvaluationValue::Bigint("1234567890123456789012345".into())
    );
    ensure!(
        property(&value, "bytes")?
            == &EvaluationValue::TypedArray {
                kind: TypedArrayKind::Uint8,
                bytes: vec![7, 8, 9]
            }
    );
    ensure!(utility_released && main_released && error_matches && cancelled_released && healthy);
    ensure!(count["result"]["value"] == 3);
    ensure!(resources.metrics()?.active_leases == 0);
    ensure!(queue.metrics()?.reserved == 0 && queue.metrics()?.queued == 0);
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await?;
    Ok(())
}
