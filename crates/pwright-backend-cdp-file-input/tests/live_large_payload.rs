#![cfg(feature = "large-commands")]

use anyhow::{Context, Result, ensure};
use pwright_backend_cdp_file_input::{CdpFilePayloadService, PayloadOutcome, PayloadProfile};
use pwright_backend_cdp_locator_resolution::{
    CdpLocatorResolutionService, ResolutionProfile, ResolutionRequest,
};
use pwright_cdp::{
    CdpClient, CdpConnection, CdpConnectionProductionProfile, CdpSession,
    connection::{ConnectionConfig, large_commands::LargeCommandProfile},
};
use pwright_core::{
    AtomicObjectIdSource, ProductionResourceLedger, ResourceClass, ResourceLedger, ResourceLimits,
    ResourceRequirements, ResourceScope, Service,
};
use pwright_file_input_model::{FileInputLimits, UPLOAD_SIZE_LIMIT};
use pwright_file_input_source::{Content, PayloadSource, PreparedPayloads};
use pwright_selector_model::Selector;
use serde_json::json;
use std::{sync::Arc, time::Duration};

fn production(requirements: ResourceRequirements) -> Result<ProductionResourceLedger> {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    Ok(ProductionResourceLedger::new(limits, requirements)?)
}

#[tokio::test]
#[ignore = "memory-heavy; dedicated Chrome preflight and PWRIGHT_CDP_WS_URL required"]
async fn near_fifty_mib_payload_uses_bounded_outbound_fragments_and_small_responses() -> Result<()>
{
    // Arrange: ordinary 4 MiB commands/receive ceiling remains unchanged.
    let large = LargeCommandProfile::new(ResourceScope::extension(147, 1), 1, 72 << 20, 64 << 10)?;
    let profile = CdpConnectionProductionProfile::new(
        ResourceScope::Connection,
        ConnectionConfig {
            timeout: Duration::from_secs(10),
            write_channel_capacity: 8,
            pending_command_capacity: 8,
            max_command_bytes: 4 << 20,
            max_response_bytes: 64 << 10,
            max_event_bytes: 64 << 10,
            max_websocket_bytes: 4 << 20,
            event_channel_capacity: 32,
            max_protocol_error_bytes: 2048,
        },
    )
    .with_large_commands(large)?
    .with_pending_session_retirement(256)?;
    ensure!(profile.config().max_command_bytes == 4 << 20);
    ensure!(profile.config().max_websocket_bytes == 4 << 20);
    let transport = production(profile.requirements())?;
    let connection = CdpConnection::connect_with_production_resources(
        &std::env::var("PWRIGHT_CDP_WS_URL")?,
        profile,
        &transport,
    )
    .await?;
    let io_finished = connection.io_finished();
    let browser = CdpSession::browser(connection.clone());
    let context = browser.target_create_browser_context(None, None).await?;
    let profile = ResolutionProfile::new(ResourceScope::extension(147, 2), 1)?;
    let groups = production(profile.requirements())?;
    let resolver = CdpLocatorResolutionService::new(
        profile,
        &groups,
        Arc::new(AtomicObjectIdSource::new(1)?),
    )?;
    let profile = PayloadProfile::new(ResourceScope::extension(147, 3), 1, 1, 70 << 20, 512)?
        .with_large_commands();
    let backend_resources = production(profile.requirements()?)?;
    let backend = CdpFilePayloadService::new(profile, &backend_resources)?;
    let source = ResourceLedger::new(
        ResourceLimits::deny_all(1, 1)
            .with_class(
                ResourceClass::ArtifactBytes,
                UPLOAD_SIZE_LIMIT,
                UPLOAD_SIZE_LIMIT,
            )
            .with_class(ResourceClass::LiveObject, 1, 1)
            .with_class(ResourceClass::BackendStateBytes, 8192, 8192),
    );
    let outcome = tokio::time::timeout(Duration::from_secs(30), async {
        let target = browser.target_create_in_context("about:blank", &context).await?;
        let session = browser.target_attach(&target).await?;
        let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(connection.clone(), session, target));
        let frame = page.page_get_frame_tree().await?.frame.id;
        let mut events = page.subscribe_events();
        page.runtime_enable().await?;
        let numeric = page.page_create_isolated_world(&frame, "__pwright_large_upload__", false).await?;
        let world: Arc<str> = loop {
            let event = events.recv().await?;
            if event.method == "Runtime.executionContextCreated" && event.params["context"]["id"] == numeric {
                break event.params["context"]["uniqueId"].as_str().context("world missing")?.into();
            }
        };
        page.runtime_evaluate("document.body.innerHTML='<input type=file>'; window.fileEvents=[]; for (const name of ['input','change']) document.querySelector('input').addEventListener(name,e=>window.fileEvents.push(e.type)); void 0;").await?;
        let count = usize::try_from(UPLOAD_SIZE_LIMIT - 1)?;
        // This temporary fixture allocation is external to the admitted source;
        // release it before encoding. No Node Buffer or filesystem participates.
        let bytes = vec![0xa5; count];
        let payloads = PreparedPayloads::materialize(&source, ResourceScope::Connection,
            FileInputLimits::new(1, 512, 128, 128, 512)?, &mut [PayloadSource {
                name: "near-limit.bin", mime_type: "application/octet-stream", last_modified_ms: Some(1234.0),
                content: Content::Bytes(&bytes), }])?;
        drop(bytes);
        let base64_bytes = payloads.plan().base64_bytes();
        let element = resolver.call(ResolutionRequest { client: page.clone(), utility_world: world.clone(),
            selector: Selector::css("input")?, destruction: None }).await?.context("input missing")?;
        let native_id = element.object_id().to_owned();
        // Act: a real nearly-50-MiB File is created through the selected backend.
        let result = backend.apply(&element, &payloads).await;
        element.release().await?;
        drop(payloads);
        // Assert: inspect every browser byte but return only a small summary.
        ensure!(result? == PayloadOutcome::Applied);
        let actual = page.runtime_evaluate("(async()=>{const f=document.querySelector('input').files[0];const b=new Uint8Array(await f.arrayBuffer());return {name:f.name,type:f.type,size:f.size,mtime:f.lastModified,allBytes:b.every(x=>x===165),events:window.fileEvents};})()").await?;
        ensure!(actual["result"]["value"] == json!({"name":"near-limit.bin","type":"application/octet-stream","size":count,"mtime":1234,"allBytes":true,"events":["input","change"]}), "large File summary: {}", actual);
        ensure!(page.runtime_call_function_on(&native_id,"function(){return this.id}",vec![]).await.is_err());
        let metrics = connection.large_command_metrics().context("large pool absent")?;
        ensure!(metrics.active == 0 && metrics.largest_fragment_payload == large.fragment_bytes());
        ensure!(metrics.fragments_written >= base64_bytes.div_ceil(large.fragment_bytes() as u64));
        ensure!(source.metrics()?.active_leases == 0 && groups.metrics()?.active_leases == 0 && backend_resources.metrics()?.active_leases == 0);
        println!("large upload passed: source_bytes={}, base64_bytes={}, fragments={}, largest_fragment={}", count, base64_bytes, metrics.fragments_written, metrics.largest_fragment_payload);
        // Separate framing scenario: multi-byte code points cross byte boundaries.
        let group = "__pwright_large_utf8__";
        let unicode = "界𠮷".repeat(50_000);
        let result = page.runtime_call_function_by_value_in_large_group_to_settlement(&world,
            "function(value){return value.length}", vec![json!({"value":unicode})], group).await;
        page.runtime_release_object_group(group).await?;
        ensure!(result?["result"]["value"] == 150_000);
        ensure!(connection.large_command_metrics().unwrap().active == 0);
        println!("large upload UTF-8 fragment reconstruction passed");
        Ok::<(), anyhow::Error>(())
    }).await;
    let disposed = browser.target_dispose_browser_context(&context).await;
    drop(browser);
    drop(connection);
    tokio::time::timeout(Duration::from_secs(5), io_finished).await?;
    disposed?;
    outcome??;
    ensure!(transport.metrics()?.active_leases == 0);
    Ok(())
}
