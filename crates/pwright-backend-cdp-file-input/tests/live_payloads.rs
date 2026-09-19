use anyhow::{Context, Result, ensure};
use pwright_backend_cdp_file_input::{
    CdpFilePayloadService, PayloadError, PayloadOutcome, PayloadProfile, PayloadTarget,
};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cdp::{CdpClient, CdpEvent, CdpSession, connection::ConnectionConfig};
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionProfile, ResolutionRequest,
};
use pwright_core::{
    AtomicObjectIdSource, ProductionResourceLedger, ResourceClass, ResourceLedger, ResourceLimits,
    ResourceRequirements, ResourceScope, Service,
};
use pwright_file_input_fs::{PinnedMime, PreparedInput, SystemFileSystem};
use pwright_file_input_model::{FileInputError, FileInputLimits};
use pwright_file_input_source::{Content, PayloadSource, PreparedPayloads};
use pwright_file_input_tokio::{FileInputExecutor, WorkProfile};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use pwright_runtime::CleanupQueue;
use pwright_selector_model::Selector;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

fn production(requirements: ResourceRequirements) -> Result<ProductionResourceLedger> {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    Ok(ProductionResourceLedger::new(limits, requirements)?)
}
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
            if numeric.map_or_else(
                || context["auxData"]["isDefault"].as_bool() == Some(true),
                |id| context["id"].as_i64() == Some(id),
            ) {
                return Ok(context["uniqueId"]
                    .as_str()
                    .context("missing world identity")?
                    .into());
            }
        }
    })
    .await
    .context("world discovery timeout")?
}

struct Case {
    name: &'static str,
    html: &'static str,
    count: usize,
    shadow: bool,
    expected: Result<PayloadOutcome, FileInputError>,
}
fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "binary",
            html: "<input id=target type=file>",
            count: 1,
            shadow: false,
            expected: Ok(PayloadOutcome::Applied),
        },
        Case {
            name: "multiple",
            html: "<input id=target type=file multiple>",
            count: 2,
            shadow: false,
            expected: Ok(PayloadOutcome::Applied),
        },
        Case {
            name: "clear",
            html: "<input id=target type=file>",
            count: 0,
            shadow: false,
            expected: Ok(PayloadOutcome::Applied),
        },
        Case {
            name: "label",
            html: "<label for=file><span id=target>Upload</span></label><input id=file type=file>",
            count: 1,
            shadow: false,
            expected: Ok(PayloadOutcome::Applied),
        },
        Case {
            name: "hidden-disabled",
            html: "<input id=target type=file disabled style='display:none'>",
            count: 1,
            shadow: false,
            expected: Ok(PayloadOutcome::Applied),
        },
        Case {
            name: "shadow",
            html: "<div id=host></div>",
            count: 1,
            shadow: true,
            expected: Ok(PayloadOutcome::Applied),
        },
        Case {
            name: "text-noop",
            html: "<input id=target type=text>",
            count: 1,
            shadow: false,
            expected: Ok(PayloadOutcome::Noop),
        },
        Case {
            name: "not-input",
            html: "<div id=target>Not input</div>",
            count: 1,
            shadow: false,
            expected: Err(FileInputError::NotInput),
        },
        Case {
            name: "button-not-label",
            html: "<label for=file><button id=target>Button</button></label><input id=file type=file>",
            count: 1,
            shadow: false,
            expected: Err(FileInputError::NotInput),
        },
        Case {
            name: "multiple-rejected",
            html: "<input id=target type=file>",
            count: 2,
            shadow: false,
            expected: Err(FileInputError::MultipleFiles),
        },
        Case {
            name: "directory-rejected",
            html: "<input id=target type=file webkitdirectory>",
            count: 1,
            shadow: false,
            expected: Err(FileInputError::DirectoryRequired),
        },
        Case {
            name: "directory-clear-rejected",
            html: "<input id=target type=file webkitdirectory>",
            count: 0,
            shadow: false,
            expected: Err(FileInputError::DirectoryRequired),
        },
    ]
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_CDP_WS_URL"]
async fn payload_mutation_preserves_browser_files_events_and_group_cleanup() -> Result<()> {
    // Arrange: explicit bounded connection, own browser Context and separate ledgers.
    let endpoint = CdpWebSocketEndpoint::parse(std::env::var("PWRIGHT_CDP_WS_URL")?)?;
    let config = ConnectionConfig {
        timeout: Duration::from_secs(10),
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
        .attach(CdpAttachRequest::new(endpoint, config))
        .await?;
    let connection = attachment.connection().clone();
    let browser = CdpSession::browser(connection.clone());
    let context_id = browser.target_create_browser_context(None, None).await?;
    let queue = CleanupQueue::new(1);
    let resolution_profile = ResolutionProfile::new(ResourceScope::extension(145, 1), 1)?;
    let resolution_resources = production(resolution_profile.requirements())?;
    let resolver = ContinuableResolutionService::new(
        Arc::new(CdpLocatorResolutionService::new(
            resolution_profile,
            &resolution_resources,
            Arc::new(AtomicObjectIdSource::new(1)?),
        )?),
        queue.clone(),
    );
    let profile = PayloadProfile::new(ResourceScope::extension(145, 2), 1, 4, 4096, 1024)?;
    let backend_resources = production(profile.requirements()?)?;
    let backend = CdpFilePayloadService::new(profile, &backend_resources)?;
    let source = ResourceLedger::new(
        ResourceLimits::deny_all(1, 4)
            .with_class(ResourceClass::LiveObject, 4, 4)
            .with_class(ResourceClass::ArtifactBytes, 4096, 4096)
            .with_class(ResourceClass::BackendStateBytes, 8192, 8192),
    );
    let outcome: Result<()> = async {
        let target = browser.target_create_in_context("about:blank", &context_id).await?;
        let session = browser.target_attach(&target).await?;
        let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(connection.clone(), session, target));
        let mut events = page.subscribe_events(); page.runtime_enable().await?; let main = world(&mut events, None).await?;
        let frame = page.page_get_frame_tree().await?.frame.id;
        let numeric = page.page_create_isolated_world(&frame, "__pwright_file_input_live__", false).await?;
        let utility = world(&mut events, Some(numeric)).await?;
        page.runtime_evaluate_in_unique_context("window.fileEvents=[]; for(const name of ['input','change']) document.addEventListener(name,e=>window.fileEvents.push({type:e.type,bubbles:e.bubbles,composed:e.composed,trusted:e.isTrusted}));", &main).await?;
        for case in cases() {
            let setup = format!("document.body.innerHTML={}; window.fileEvents=[]; if({}) document.getElementById('host').attachShadow({{mode:'open'}}).innerHTML='<input id=target type=file>'; if({}) {{ const data=new DataTransfer(); data.items.add(new File(['old'],'old.txt')); document.querySelector('input').files=data.files; }}", serde_json::to_string(case.html)?, case.shadow, case.name == "clear");
            page.runtime_evaluate_in_unique_context(&setup, &main).await?;
            let resolved = resolver.call(ResolutionRequest { client: page.clone(), utility_world: utility.clone(), selector: Selector::css("#target")?, destruction: None }).await?.context("target should resolve")?;
            let native_id = resolved.element().object_id().to_owned();
            let mut entries: Vec<_> = (0..case.count).map(|index| PayloadSource { name: if index == 0 { "first.txt" } else { "second.bin" }, mime_type: if index == 0 { "text/plain" } else { "application/octet-stream" }, last_modified_ms: Some(1234.5), content: Content::Bytes(&[0,255,1]) }).collect();
            let payloads = PreparedPayloads::materialize(&source, ResourceScope::Connection, FileInputLimits::new(4, 512, 128, 128, 512)?, &mut entries)?;
            // Act: one exact-element payload mutation, no Locator retry/fallback.
            let result = backend.apply(resolved.element(), &payloads).await;
            resolved.release().await?; drop(payloads);
            let state = page.runtime_evaluate_in_unique_context("(async()=>{ const input=document.querySelector('input')||document.getElementById('host')?.shadowRoot.querySelector('input'); return {files:await Promise.all(Array.from(input?.files||[]).map(async f=>({name:f.name,type:f.type,mtime:f.lastModified,bytes:Array.from(new Uint8Array(await f.arrayBuffer()))}))),events:window.fileEvents}; })()", &main).await?;
            // Assert: actual bytes/metadata/events, native disposal, exact policy errors.
            match case.expected {
                Ok(expected) => ensure!(result.as_ref().is_ok_and(|value| *value == expected), "{}: {:?}", case.name, result),
                Err(expected) => ensure!(matches!(result, Err(PayloadError::Policy(actual)) if actual == expected), "{}: {:?}", case.name, result),
            }
            let expected_files: Vec<Value> = if case.expected == Ok(PayloadOutcome::Applied) {
                (0..case.count).map(|index| json!({"name":if index==0 {"first.txt"} else {"second.bin"},"type":if index==0 {"text/plain"} else {"application/octet-stream"},"mtime":1234,"bytes":[0,255,1]})).collect()
            } else { Vec::new() };
            ensure!(state["result"]["value"]["files"] == json!(expected_files), "{} file data: {}", case.name, state);
            let expected_events = if case.expected == Ok(PayloadOutcome::Applied) {
                if case.shadow { json!([{"type":"input","bubbles":true,"composed":true,"trusted":false}]) }
                else { json!([{"type":"input","bubbles":true,"composed":true,"trusted":false},{"type":"change","bubbles":true,"composed":false,"trusted":false}]) }
            } else { json!([]) };
            ensure!(state["result"]["value"]["events"] == expected_events, "{} events: {}", case.name, state);
            ensure!(page.runtime_call_function_on(&native_id, "function(){return this.id}", Vec::new()).await.is_err(), "native group not released");
            ensure!(source.metrics()?.active_leases == 0 && backend_resources.metrics()?.active_leases == 0 && resolution_resources.metrics()?.active_leases == 0);
            println!("file-input live case {} passed", case.name);
        }
        // Arrange: make the native files setter throw in the utility world.
        page.runtime_evaluate_in_unique_context("document.body.innerHTML='<input id=target type=file>'; window.fileEvents=[];", &main).await?;
        page.runtime_evaluate_in_unique_context("window.originalFilesDescriptor=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'files'); Object.defineProperty(HTMLInputElement.prototype,'files',{...window.originalFilesDescriptor,set(){throw new Error('file-input setter failed');}}); void 0;", &utility).await.context("install throwing files setter")?;
        let resolved = resolver.call(ResolutionRequest { client: page.clone(), utility_world: utility.clone(), selector: Selector::css("#target")?, destruction: None }).await?.context("throwing target missing")?;
        let native_id = resolved.element().object_id().to_owned();
        let payloads = PreparedPayloads::materialize(&source, ResourceScope::Connection, FileInputLimits::new(4, 512, 128, 128, 512)?, &mut [PayloadSource { name: "failure.txt", mime_type: "text/plain", last_modified_ms: None, content: Content::Bytes(b"failure") }])?;
        // Act: a real browser exception follows the same grouped dispatch.
        let result = backend.apply(resolved.element(), &payloads).await;
        resolved.release().await?; drop(payloads);
        page.runtime_evaluate_in_unique_context("Object.defineProperty(HTMLInputElement.prototype,'files',window.originalFilesDescriptor); delete window.originalFilesDescriptor;", &utility).await?;
        // Assert: preserve the exception and release the root and Rust owners.
        ensure!(matches!(result, Err(PayloadError::Backend(pwright_cdp::connection::CdpError::JsException(ref message))) if message.contains("file-input setter failed")), "unexpected setter error: {:?}", result);
        ensure!(page.runtime_call_function_on(&native_id, "function(){return this.id}", Vec::new()).await.is_err(), "exception path retained root");
        ensure!(source.metrics()?.active_leases == 0 && backend_resources.metrics()?.active_leases == 0 && resolution_resources.metrics()?.active_leases == 0);
        let actual = page.runtime_evaluate_in_unique_context("({count:document.querySelector('input').files.length,events:window.fileEvents})", &main).await?;
        ensure!(actual["result"]["value"] == json!({"count":0,"events":[]}));
        println!("file-input live case setter-exception-cleanup passed");
        for removed in [false, true] {
            // Arrange: retain the original native object before a same-selector
            // replacement appears. The owner keeps its admitted group/world.
            page.runtime_evaluate_in_unique_context("document.body.innerHTML='<input id=target type=file>'; window.original=document.querySelector('input'); window.fileEvents=[];", &main).await?;
            let resolved = resolver.call(ResolutionRequest { client: page.clone(), utility_world: utility.clone(), selector: Selector::css("#target")?, destruction: None }).await?.context("retained target missing")?;
            let native_id = resolved.element().object_id().to_owned();
            let script = format!("(() => {{ window.original.id='original'; const replacement=window.original.cloneNode(); replacement.id='target'; window.original.after(replacement); if({removed}) window.original.remove(); }})()");
            page.runtime_evaluate_in_unique_context(&script, &main).await?;
            let payloads = PreparedPayloads::materialize(&source, ResourceScope::Connection, FileInputLimits::new(4, 512, 128, 128, 512)?, &mut [PayloadSource { name: "retained.bin", mime_type: "application/octet-stream", last_modified_ms: None, content: Content::Bytes(&[0,255,1]) }])?;
            let client = resolved.element().client();
            let target = PayloadTarget::new(client.as_ref(), resolved.element().utility_world(), resolved.element().object_id(), resolved.element().object_group())?;
            // Act: exact borrowed-node entry point, no selector resolution.
            let result = backend.apply_on_target(target, &payloads).await;
            // Assert: original bytes change even when removed, replacement does
            // not; the primitive has not released the native receiver itself.
            ensure!(result? == PayloadOutcome::Applied);
            let actual = page.runtime_evaluate_in_unique_context("(async()=>({connected:window.original.isConnected,bytes:[...new Uint8Array(await window.original.files[0].arrayBuffer())],replacement:document.querySelector('#target').files.length}))()", &main).await?;
            ensure!(actual["result"]["value"] == json!({"connected":!removed,"bytes":[0,255,1],"replacement":0}), "retained target state: {actual}");
            ensure!(page.runtime_call_function_on(&native_id, "function(){return this.id}", Vec::new()).await.is_ok(), "borrowed mutation disposed receiver");
            resolved.release().await?;
            drop(payloads);
            ensure!(page.runtime_call_function_on(&native_id, "function(){return this.id}", Vec::new()).await.is_err(), "explicit group release retained receiver");
            ensure!(source.metrics()?.active_leases == 0 && backend_resources.metrics()?.active_leases == 0 && resolution_resources.metrics()?.active_leases == 0);
            page.runtime_evaluate_in_unique_context("delete window.original", &main).await?;
            println!("file-input live retained-node removed={removed} passed");
        }
        // Arrange: actual path preparation precedes target resolution, as in
        // the public action contract. No Node fs/base64 implementation is used.
        page.runtime_evaluate_in_unique_context("document.body.innerHTML='<input id=target type=file>'; window.fileEvents=[];", &main).await?;
        let limits = FileInputLimits::new(4, 512, 128, 128, 512)?;
        let work_profile = WorkProfile::new(1, limits, ResourceScope::Connection, 4096)?;
        let queue_resources = production(work_profile.requirements()?)?;
        let executor = FileInputExecutor::new(work_profile, &queue_resources, source.clone(), tokio::runtime::Handle::current(), Arc::new(SystemFileSystem), Arc::new(PinnedMime))?;
        let PreparedInput::Payloads(payloads) = executor.submit_paths(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), &["../pwright-file-input-source/tests/fixture.txt"], pwright_file_input_model::BrowserLocation::Separate)?.await? else { anyhow::bail!("separate file path must become payload"); };
        let expected_mtime = payloads.entries()[0].last_modified_ms().context("path mtime missing")?.trunc() as i64;
        let resolved = resolver.call(ResolutionRequest { client: page.clone(), utility_world: utility.clone(), selector: Selector::css("#target")?, destruction: None }).await?.context("path target missing")?;
        // Act: prepared real-file content reaches the browser through Rust CDP.
        let result = backend.apply(resolved.element(), &payloads).await;
        resolved.release().await?; drop(payloads); executor.close_and_drain().await; drop(executor);
        // Assert: real file contents, pinned MIME and filesystem mtime survived.
        ensure!(result? == PayloadOutcome::Applied);
        let actual = page.runtime_evaluate_in_unique_context("(async()=>{const f=document.querySelector('input').files[0];return {name:f.name,type:f.type,mtime:f.lastModified,bytes:Array.from(new Uint8Array(await f.arrayBuffer()))};})()", &main).await?;
        ensure!(actual["result"]["value"] == json!({"name":"fixture.txt","type":"text/plain","mtime":expected_mtime,"bytes":include_bytes!("../../pwright-file-input-source/tests/fixture.txt").to_vec()}), "path upload data: {}", actual);
        ensure!(queue_resources.metrics()?.active_leases == 0 && source.metrics()?.active_leases == 0 && backend_resources.metrics()?.active_leases == 0 && resolution_resources.metrics()?.active_leases == 0);
        println!("file-input live case real-path-pipeline passed");
        Ok(())
    }.await;
    // Always remove only this diagnostic's Context; retain cleanup errors.
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
    outcome?;
    ensure!(queue.metrics()?.reserved == 0 && queue.metrics()?.queued == 0);
    Ok(())
}
