use anyhow::{Context, Result, ensure};
use pwright_backend_cdp_file_input::{CdpFilePayloadService, PayloadOutcome, PayloadProfile};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cdp::{CdpClient, CdpSession, connection::ConnectionConfig};
use pwright_composition_chromium_file_input::{FilePayloadOperationService, UploadProfile};
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionProfile, ResolutionRequest,
};
use pwright_core::{
    AtomicObjectIdSource, ProductionResourceLedger, ResourceClass, ResourceLedger, ResourceLimits,
    ResourceRequirements, ResourceScope, Service,
};
use pwright_file_input_model::FileInputLimits;
use pwright_file_input_source::{Content, PayloadSource, PreparedPayloads};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use pwright_runtime::CleanupQueue;
use pwright_selector_model::Selector;
use serde_json::json;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context as TaskContext, Waker},
    time::Duration,
};

fn production(requirements: ResourceRequirements) -> Result<ProductionResourceLedger> {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    Ok(ProductionResourceLedger::new(limits, requirements)?)
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_CDP_WS_URL"]
async fn native_upload_is_single_use_across_waiter_abandonment() -> Result<()> {
    // Arrange: one owned Context, admitted groups/sources/backend/continuations.
    let config = ConnectionConfig {
        timeout: Duration::from_secs(5),
        write_channel_capacity: 8,
        event_channel_capacity: 32,
        pending_command_capacity: 8,
        max_command_bytes: 2 << 20,
        max_response_bytes: 1 << 20,
        max_event_bytes: 64 << 10,
        max_websocket_bytes: 2 << 20,
        max_protocol_error_bytes: 2048,
    };
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(
            CdpWebSocketEndpoint::parse(std::env::var("PWRIGHT_CDP_WS_URL")?)?,
            config,
        ))
        .await?;
    let connection = attachment.connection().clone();
    let browser = CdpSession::browser(connection.clone());
    let context = browser.target_create_browser_context(None, None).await?;
    let queue = CleanupQueue::new(2);
    let profile = ResolutionProfile::new(ResourceScope::extension(146, 1), 1)?;
    let groups = production(profile.requirements())?;
    let resolver = ContinuableResolutionService::new(
        Arc::new(CdpLocatorResolutionService::new(
            profile,
            &groups,
            Arc::new(AtomicObjectIdSource::new(1)?),
        )?),
        queue.clone(),
    );
    let profile = PayloadProfile::new(ResourceScope::extension(146, 2), 1, 1, 128, 128)?;
    let backend = production(profile.requirements()?)?;
    let inner = Arc::new(CdpFilePayloadService::new(profile, &backend)?);
    let profile = UploadProfile::new(ResourceScope::extension(146, 3), 1, 4096)?;
    let owners = production(profile.requirements())?;
    let service = FilePayloadOperationService::new(profile, &owners, inner, queue.clone())?;
    let source = ResourceLedger::new(
        ResourceLimits::deny_all(1, 1)
            .with_class(ResourceClass::LiveObject, 1, 1)
            .with_class(ResourceClass::ArtifactBytes, 64, 64)
            .with_class(ResourceClass::BackendStateBytes, 4096, 4096),
    );
    let outcome = tokio::time::timeout(Duration::from_secs(15), async {
        let target = browser.target_create_in_context("about:blank", &context).await?;
        let session = browser.target_attach(&target).await?;
        let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(connection.clone(), session, target));
        let frame = page.page_get_frame_tree().await?.frame.id;
        let mut events = page.subscribe_events();
        page.runtime_enable().await?;
        let numeric = page.page_create_isolated_world(&frame, "__pwright_upload_continuation__", false).await?;
        let world: Arc<str> = loop {
            let event = events.recv().await?;
            if event.method == "Runtime.executionContextCreated" && event.params["context"]["id"] == numeric {
                break event.params["context"]["uniqueId"].as_str().context("world identity missing")?.into();
            }
        };
        for mode in ["complete", "drop-started", "drop-unpolled"] {
            page.runtime_evaluate("document.body.innerHTML='<input type=file>'; window.fileEvents=[]; for (const name of ['input','change']) document.querySelector('input').addEventListener(name,e=>window.fileEvents.push(e.type)); void 0;").await?;
            let element = resolver.call(ResolutionRequest { client: page.clone(), utility_world: world.clone(),
                selector: Selector::css("input")?, destruction: None }).await?.context("input missing")?;
            let native_id = element.element().object_id().to_owned();
            let payloads = PreparedPayloads::materialize(&source, ResourceScope::Connection,
                FileInputLimits::new(1, 128, 64, 64, 128)?, &mut [PayloadSource {
                    name: "owned.bin", mime_type: "application/octet-stream", last_modified_ms: Some(1234.0),
                    content: Content::Bytes(&[0, 255, 1]), }])?;
            let mut future = service.prepare(element, payloads)?;
            // Act: the single-threaded runtime cannot deliver CDP before this
            // first synchronous poll returns. No timing sleep or wire retry.
            match mode {
                "complete" => ensure!(future.await? == PayloadOutcome::Applied),
                "drop-started" => {
                    ensure!(Pin::new(&mut future).poll(&mut TaskContext::from_waker(Waker::noop())).is_pending());
                    drop(future);
                    ensure!(service.active_operations() == 1);
                    ensure!(source.metrics()?.class(ResourceClass::ArtifactBytes).current == 3);
                    ensure!(backend.metrics()?.class(ResourceClass::ActiveCommand).current == 1);
                }
                "drop-unpolled" => drop(future),
                _ => unreachable!(),
            }
            while let Some(job) = queue.try_take()? { job.await?; }
            // Assert: real File bytes, exactly one input/change pair or none,
            // root invalid after group release, and no remaining call owners.
            let actual = page.runtime_evaluate("(async()=>({files:await Promise.all(Array.from(document.querySelector('input').files).map(async f=>({name:f.name,type:f.type,mtime:f.lastModified,bytes:Array.from(new Uint8Array(await f.arrayBuffer()))}))),events:window.fileEvents}))()").await?;
            let expected = if mode == "drop-unpolled" { json!({"files":[],"events":[]}) }
                else { json!({"files":[{"name":"owned.bin","type":"application/octet-stream","mtime":1234,"bytes":[0,255,1]}],"events":["input","change"]}) };
            ensure!(actual["result"]["value"] == expected, "{}: {}", mode, actual);
            ensure!(page.runtime_call_function_on(&native_id,"function(){return this.id}",vec![]).await.is_err());
            ensure!(source.metrics()?.active_leases == 0 && groups.metrics()?.active_leases == 0 && backend.metrics()?.active_leases == 0);
            ensure!(service.active_operations() == 0 && service.is_cleanup_healthy());
            ensure!(queue.metrics()?.reserved == 0 && queue.metrics()?.queued == 0);
            println!("file-input continuation live case {} passed", mode);
        }
        Ok::<(), anyhow::Error>(())
    }).await;
    // Bound diagnostic failure cleanup and dispose only the owned Context.
    let disposed = browser.target_dispose_browser_context(&context).await;
    let mut cleanup = Ok(());
    while let Some(job) = queue.try_take()? {
        if let Err(error) = tokio::time::timeout(Duration::from_secs(5), job)
            .await
            .map_err(anyhow::Error::from)
            .and_then(|result| result.map_err(anyhow::Error::from))
        {
            cleanup = Err(error);
        }
    }
    disposed?;
    cleanup?;
    outcome??;
    drop(service);
    ensure!(owners.metrics()?.active_leases == 0);
    Ok(())
}
