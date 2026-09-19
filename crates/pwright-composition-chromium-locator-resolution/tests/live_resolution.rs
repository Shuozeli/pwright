use anyhow::{Context, Result, ensure};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cdp::{CdpClient, CdpEvent, CdpSession, connection::ConnectionConfig};
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionError, ResolutionProfile,
    ResolutionRequest,
};
use pwright_core::{
    AtomicObjectIdSource, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope,
    Service,
};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use pwright_runtime::CleanupQueue;
use pwright_selector_model::{AriaRole, RoleOptions, Selector};
use std::{sync::Arc, time::Duration};

async fn world(
    events: &mut tokio::sync::broadcast::Receiver<CdpEvent>,
    numeric: Option<i64>,
) -> Result<Arc<str>> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = events.recv().await?;
            if event.method != "Runtime.executionContextCreated" {
                continue;
            }
            let context = &event.params["context"];
            let matches = numeric.map_or_else(
                || context["auxData"]["isDefault"].as_bool() == Some(true),
                |id| context["id"].as_i64() == Some(id),
            );
            if matches {
                return Ok(context["uniqueId"]
                    .as_str()
                    .context("missing world identity")?
                    .into());
            }
        }
    })
    .await
    .context("execution world deadline")?
}

#[tokio::test]
#[ignore = "requires dedicated Chrome CDP preflight and PWRIGHT_CDP_WS_URL"]
async fn group_owned_semantic_resolution_keeps_exact_elements_and_physically_releases_them()
-> Result<()> {
    // Arrange: independent Context, connection, production ledger and caller-owned cleanup queue.
    let endpoint = CdpWebSocketEndpoint::parse(std::env::var("PWRIGHT_CDP_WS_URL")?)?;
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await?;
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(connection.clone());
    let context_id = browser.target_create_browser_context(None, None).await?;
    let queue = CleanupQueue::new(1);
    let profile = ResolutionProfile::new(ResourceScope::extension(88, 1), 1)?;
    let required = profile.requirements();
    let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
    for class in ResourceClass::ALL {
        let bound = required.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, required)?;
    let raw = CdpLocatorResolutionService::new(
        profile,
        &resources,
        Arc::new(AtomicObjectIdSource::new(1)?),
    )?;
    let service = ContinuableResolutionService::new(Arc::new(raw), queue.clone());
    let outcome: Result<_> = async {
        let target = browser.target_create_in_context("about:blank", &context_id).await?;
        let session = browser.target_attach(&target).await?;
        let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(connection.clone(), session, target));
        let mut events = page.subscribe_events();
        page.runtime_enable().await?;
        let main = world(&mut events, None).await?;
        page.runtime_evaluate_in_unique_context(r#"(() => {
          document.body.innerHTML = '<label for="email">Email</label><input id="email"><button>Save</button><button>Save</button><div id="shadow"></div>';
          document.getElementById('shadow').attachShadow({mode:'open'}).innerHTML = '<button id="shadow-save">Shadow save</button>';
          Document.prototype.querySelectorAll = () => { throw new Error('main-world prototype override'); };
          globalThis.__pwrightMainMarker = 'main';
        })()"#, &main).await?;
        let frame = page.page_get_frame_tree().await?.frame.id;
        let utility_numeric = page.page_create_isolated_world(&frame, "__pwright_resolution_live__", false).await?;
        let utility = world(&mut events, Some(utility_numeric)).await?;
        let request = |selector| ResolutionRequest { client: page.clone(), utility_world: utility.clone(), selector, destruction: None };

        // Act: query in the utility world, retain the exact node across detachment,
        // release it, then exercise semantic Shadow DOM, missing and strict cases.
        let element = service.call(request(Selector::label("Email", true))).await?.context("label should resolve")?;
        let native = element.element().object_id().to_owned();
        let before = page.runtime_call_function_on(&native,
            "function(){return {id:this.id,connected:this.isConnected,main:typeof globalThis.__pwrightMainMarker}}", Vec::new()).await?;
        page.runtime_evaluate_in_unique_context("document.getElementById('email').remove()", &main).await?;
        let after = page.runtime_call_function_on(&native,
            "function(){return {id:this.id,connected:this.isConnected}}", Vec::new()).await?;
        element.release().await?;
        let released = page.runtime_call_function_on(&native, "function(){return this.id}", Vec::new()).await.is_err();

        let shadow = service.call(request(Selector::role(AriaRole::Button, RoleOptions::new().with_name("Shadow save", true))?)).await?.context("Shadow DOM role should resolve")?;
        let shadow_native = shadow.element().object_id().to_owned();
        let shadow_id = page.runtime_call_function_on(&shadow_native, "function(){return this.id}", Vec::new()).await?;
        drop(shadow);
        queue.try_take()?.context("dropped element must enqueue cleanup")?.await?;
        let shadow_released = page.runtime_call_function_on(&shadow_native, "function(){return this.id}", Vec::new()).await.is_err();
        let missing = service.call(request(Selector::css("#absent")?)).await?.is_none();
        let strict = matches!(service.call(request(Selector::role(AriaRole::Button, RoleOptions::new().with_name("Save", true))?)).await,
            Err(ResolutionError::StrictViolation { matches: 2 }));
        Ok((before, after, released, shadow_id, shadow_released, missing, strict))
    }.await;

    // Cleanup on operational errors as well as success, without touching preexisting targets.
    let cleanup: Result<()> = async {
        while let Some(job) = queue.try_take()? {
            job.await?;
        }
        Ok(())
    }
    .await;
    let disposed = browser.target_dispose_browser_context(&context_id).await;
    cleanup?;
    disposed?;
    let (before, after, released, shadow, shadow_released, missing, strict) = outcome?;
    // Assert after owned native targets are gone.
    ensure!(
        before["result"]["value"]
            == serde_json::json!({"id":"email","connected":true,"main":"undefined"})
    );
    ensure!(after["result"]["value"] == serde_json::json!({"id":"email","connected":false}));
    ensure!(shadow["result"]["value"] == "shadow-save");
    ensure!(released && shadow_released && missing && strict);
    ensure!(resources.metrics()?.active_leases == 0);
    ensure!(queue.metrics()?.reserved == 0 && queue.metrics()?.queued == 0);
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await?;
    Ok(())
}
