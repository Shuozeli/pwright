#![cfg(feature = "mouse-dispatch")]
//! Real input facade using the selected drag dispatcher, not Node/FFI parity.
use pwright_backend_cdp_drag::{
    DragCaptureFactory, DragDataLimits, DragDataProfile, DragDataStore, DragListenerFactory,
    DragMove, DragPage, DragToggleFuture, DragWorld,
    mouse::{DragMoveSource, MouseDragDispatcher},
};
use pwright_backend_cdp_input::{
    CdpInputCoordinator, CdpInputPolicy, MouseButtonExecution, MouseClickOptions,
};
use pwright_cdp::{
    CdpConnection, CdpSession, MouseButton,
    connection::{ConnectionConfig, Result as CdpResult},
};
use pwright_core::{ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope};
use serde_json::json;
use std::{
    error::Error,
    future::ready,
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
type TestResult<T> = Result<T, Box<dyn Error>>;
fn check<T: PartialEq + std::fmt::Debug>(actual: T, expected: T, label: &str) -> TestResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{label}: expected {expected:?}, got {actual:?}").into())
    }
}
struct Source {
    resources: Arc<ProductionResourceLedger>,
    listeners: DragListenerFactory,
    captures: DragCaptureFactory,
    client: Arc<CdpSession>,
    world: String,
    starts: AtomicUsize,
}
impl DragMoveSource for Source {
    fn captures(&self) -> &DragCaptureFactory {
        &self.captures
    }
    fn start(&self, movement: DragToggleFuture) -> CdpResult<DragMove> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        DragMove::start(
            ResourceScope::extension(14, 63),
            &self.resources,
            &self.listeners,
            &self.captures,
            &[DragWorld {
                client: self.client.clone(),
                unique_context: &self.world,
            }],
            movement,
        )
    }
}
fn resources() -> TestResult<(Arc<ProductionResourceLedger>, DragDataProfile)> {
    let profile = DragDataProfile::new(
        ResourceScope::extension(14, 61),
        DragDataLimits {
            items: 8,
            files: 8,
            text_bytes: 65536,
        },
        NonZeroUsize::new(1).unwrap(),
    )?;
    let req = profile
        .requirements()
        .combine_concurrent(DragCaptureFactory::requirements())
        .combine_concurrent(DragListenerFactory::requirements(1)?)
        .combine_concurrent(DragMove::requirements(1)?)
        .combine_concurrent(DragPage::requirements(4)?)
        .combine_concurrent(MouseDragDispatcher::requirements(8)?);
    let bytes = req.class(ResourceClass::BackendStateBytes);
    Ok((
        Arc::new(ProductionResourceLedger::new(
            ResourceLimits::deny_all(req.scopes, req.leases).with_class(
                ResourceClass::BackendStateBytes,
                bytes.global,
                bytes.per_scope,
            ),
            req,
        )?),
        profile,
    ))
}
async fn move_to(
    input: &CdpInputCoordinator,
    client: Arc<CdpSession>,
    dispatcher: Arc<MouseDragDispatcher>,
    x: f64,
    for_click: bool,
) -> TestResult<()> {
    let mut action = input.begin_transaction(client, None).await?;
    let mut movement = action.raw_mouse_click_with_dispatcher(
        MouseClickOptions::new(x, 40.0)?
            .with_count(0.0.try_into()?)
            .with_for_click(for_click),
        |_| ready(Ok(())),
        dispatcher,
    )?;
    let result = tokio::time::timeout(Duration::from_secs(5), &mut movement).await;
    let cleanup = movement.retire().await;
    result??;
    cleanup?;
    Ok(())
}
async fn button(mut command: MouseButtonExecution) -> TestResult<()> {
    let result = tokio::time::timeout(Duration::from_secs(5), &mut command).await;
    let cleanup = command.retire().await;
    result??;
    cleanup?;
    Ok(())
}
#[derive(Clone, Copy, Debug)]
enum Case {
    Direct,
    Drop,
    Cancel,
    ClickBypass,
}
async fn run_case(
    browser: &CdpSession,
    connection: &Arc<CdpConnection>,
    fixture: &str,
    case: Case,
) -> TestResult<()> {
    let context = browser.target_create_browser_context(None, None).await?;
    let (resources, profile) = resources()?;
    let mut target = None;
    let mut native_page = None;
    let mut dispatcher_owner = None;
    let mut driver_owner = None;
    let mut drag_page_owner = None;
    let result = async {
        // Arrange: own native Context and an existing main-frame utility world.
        let id = browser.target_create_in_context("about:blank", &context).await?;
        target = Some(id.clone());
        let session = browser.target_attach(&id).await?;
        let page = Arc::new(CdpSession::new(connection.clone(), session, id));
        native_page = Some(page.clone());
        let mut events = page.subscribe_composable_events()?;
        page.page_enable().await?;
        page.runtime_enable().await?;
        page.page_navigate(fixture).await?;
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if page.runtime_evaluate("typeof window.dragFixture === 'object'").await?["result"]["value"] == true { break; }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            Ok::<_, Box<dyn Error>>(())
        }).await??;
        page.runtime_evaluate("window.nativeDownCount = 0; addEventListener('mousedown', () => ++window.nativeDownCount)").await?;
        let frame = page.page_get_frame_tree().await?.frame.id;
        let world_id = page.page_create_isolated_world(&frame, "__pwright_mouse_drag", false).await?;
        let world = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let event = events.recv().await.map_err(|error| format!("world event: {error:?}"))?;
                if event.session_id.as_deref() == page.session_id() && event.method == "Runtime.executionContextCreated"
                    && event.params["context"]["id"] == world_id {
                    return event.params["context"]["uniqueId"].as_str().map(str::to_owned).ok_or("unique ID absent".to_string());
                }
            }
        }).await??;
        drop(events);
        let store = DragDataStore::new(profile, &resources)?;
        let captures = DragCaptureFactory::new(page.clone(), store.clone(), ResourceScope::extension(14, 62), &resources)?;
        let listeners = DragListenerFactory::new("mouse_native", ResourceScope::extension(14, 60), 1, &resources)?;
        let source = Arc::new(Source {resources: resources.clone(), captures: captures.clone(), listeners,
            client: page.clone(), world, starts: AtomicUsize::new(0)});
        let drag_page = DragPage::new(page.clone(), captures, ResourceScope::extension(14, 64), 4, &resources)?;
        drag_page_owner = Some(drag_page.clone());
        let (dispatcher, driver) = MouseDragDispatcher::new(page.clone(), drag_page.clone(), source.clone(), ResourceScope::extension(14, 65), 8, &resources)?;
        let dispatcher = Arc::new(dispatcher);
        dispatcher_owner = Some(dispatcher.clone());
        driver_owner = Some(driver);
        let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let mut action = input.begin_transaction(page.clone(), None).await?;
        // Act: all pointer actions below select the actual Mouse dispatcher.
        let initial = if matches!(case, Case::ClickBypass) { 220.0 } else { 40.0 };
        move_to(&input, page.clone(), dispatcher.clone(), initial, false).await?;
        if matches!(case, Case::Direct) {
            move_to(&input, page.clone(), dispatcher.clone(), 80.0, false).await?;
        } else {
            button(action.raw_mouse_down_with_dispatcher(MouseButton::Left, 1, dispatcher.clone())?).await?;
            if matches!(case, Case::ClickBypass) {
                move_to(&input, page.clone(), dispatcher.clone(), 225.0, true).await?;
                button(action.raw_mouse_up_with_dispatcher(MouseButton::Left, 1, dispatcher.clone())?).await?;
            } else {
                move_to(&input, page.clone(), dispatcher.clone(), 70.0, false).await?;
                check(drag_page.is_dragging(), true, "selected move entered drag")?;
                let before = page.runtime_evaluate("window.nativeDownCount").await?["result"]["value"].clone();
                check(before.clone(), json!(1), "initial native down observed")?;
                button(action.raw_mouse_down_with_dispatcher(MouseButton::Right, 1, dispatcher.clone())?).await?;
                check(page.runtime_evaluate("window.nativeDownCount").await?["result"]["value"].clone(), before, "physical down suppressed")?;
                move_to(&input, page.clone(), dispatcher.clone(), 220.0, false).await?;
                if matches!(case, Case::Drop) {
                    button(action.raw_mouse_up_with_dispatcher(MouseButton::Left, 1, dispatcher.clone())?).await?;
                } else {
                    let mut cancel = drag_page.cancel()?;
                    let outcome = (&mut cancel).await;
                    cancel.retire().await?;
                    check(outcome?, true, "explicit Page cancel")?;
                    button(action.raw_mouse_up_with_dispatcher(MouseButton::Left, 1, dispatcher.clone())?).await?;
                }
                button(action.raw_mouse_up_with_dispatcher(MouseButton::Right, 1, dispatcher.clone())?).await?;
                check(page.runtime_evaluate("window.dragFixture.events.includes('dragend')").await?["result"]["value"].clone(), json!(true), "native dragend")?;
            }
        }
        // Assert: actual DOM outcome, source selection, Page and native ownership.
        check(drag_page.is_dragging(), false, "drag Page idle")?;
        check(source.starts.load(Ordering::SeqCst), usize::from(matches!(case, Case::Drop | Case::Cancel)), "capture branch count")?;
        check(page.runtime_evaluate("window.dragFixture.dropped").await?["result"]["value"].clone(),
            if matches!(case, Case::Drop) { json!("pwright-drag-observer") } else { json!(null) }, "native delivery")?;
        check(page.runtime_evaluate_in_unique_context("globalThis.__pwrightDragObserversV1?.size ?? 0", &source.world).await?["result"]["value"].clone(), json!(0), "listener registry empty")?;
        check(dispatcher.is_drained(), true, "dispatch slots drained")?;
        check(store.active_records(), 0, "data records released")?;
        Ok::<_, Box<dyn Error>>(())
    }.await;
    let mut failures = Vec::new();
    if let Err(error) = result {
        failures.push(format!("{case:?}: {error}"));
    }
    if let Some(dispatcher) = &dispatcher_owner {
        dispatcher.stop();
    }
    if let Some(driver) = &mut driver_owner
        && let Err(error) = driver.retire_quarantined().await
    {
        failures.push(format!("quarantine: {error}"));
    }
    if let Some(drag_page) = &drag_page_owner {
        match drag_page.cancel() {
            Ok(mut cancel) => {
                let outcome = (&mut cancel).await;
                if let Err(error) = cancel.retire().await {
                    failures.push(format!("cancel retirement: {error}"));
                }
                if let Err(error) = outcome {
                    failures.push(format!("cancel result: {error}"));
                }
            }
            Err(error) => failures.push(format!("cancel admission: {error}")),
        }
    }
    if let Some(page) = &native_page
        && let Err(error) = page.send("Input.dispatchDragEvent", json!({"type":"dragCancel","x":220,"y":40,"data":{"items":[],"dragOperationsMask":65535}})).await
    { failures.push(format!("native final cancel: {error}")); }
    browser.target_dispose_browser_context(&context).await?;
    check(
        browser
            .target_get_browser_contexts()
            .await?
            .contains(&context),
        false,
        "Context absent",
    )?;
    check(
        browser
            .target_get_targets()
            .await?
            .iter()
            .any(|item| Some(&item.target_id) == target.as_ref()),
        false,
        "target absent",
    )?;
    if let Some(dispatcher) = &dispatcher_owner {
        check(
            dispatcher.is_drained(),
            true,
            "final dispatch slots drained",
        )?;
    }
    drop(driver_owner);
    drop(dispatcher_owner);
    drop(drag_page_owner);
    check(
        resources.metrics()?.active_leases,
        0,
        "all resource leases released",
    )?;
    if !failures.is_empty() {
        return Err(failures.join("\n").into());
    }
    println!(
        "selected Mouse {case:?}: native outcome, Page/source state, Context absence and zero leases verified"
    );
    Ok(())
}
#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight and verified serve-lib fixture"]
async fn selected_mouse_dispatch_drags_drops_and_cancels() -> TestResult<()> {
    let endpoint = std::env::var("PWRIGHT_CDP_WS_URL")?;
    if !endpoint.starts_with("ws://127.0.0.1:9222/devtools/browser/") {
        return Err("dedicated loopback Chrome required".into());
    }
    let fixture = std::env::var("PWRIGHT_DRAG_FIXTURE")?;
    let url = url::Url::parse(&fixture)?;
    if url.scheme() != "http" || !url.host_str().is_some_and(|host| host.ends_with(".ts.net")) {
        return Err("Tailscale fixture required".into());
    }
    let connection = CdpConnection::connect_with_config(
        &endpoint,
        ConnectionConfig {
            timeout: Duration::from_secs(5),
            ..Default::default()
        },
    )
    .await?;
    let browser = CdpSession::browser(connection.clone());
    for case in [Case::Direct, Case::Drop, Case::Cancel, Case::ClickBypass] {
        run_case(&browser, &connection, &fixture, case).await?;
    }
    Ok(())
}
