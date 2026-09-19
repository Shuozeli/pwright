//! Real native input and utility-world listener lifecycle, not full Mouse parity.
use pwright_backend_cdp_drag::{
    DragCapture, DragCaptureFactory, DragDataLimits, DragDataProfile, DragDataStore,
    DragInterceptionClient, DragListener, DragListenerFactory, DragMove, DragPage,
    DragPageOperation, DragPosition, DragWorld,
};
use pwright_cdp::{
    CdpConnection, CdpEventReceiver, CdpSession,
    connection::ConnectionConfig,
    generated::input::{DispatchMouseEventParams, MouseButton},
};
use pwright_core::{ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope};
use serde_json::json;
use std::{error::Error, num::NonZeroUsize, sync::Arc, time::Duration};
type TestResult<T> = Result<T, Box<dyn Error>>;

fn check<T: PartialEq + std::fmt::Debug>(actual: T, expected: T, label: &str) -> TestResult<()> {
    if actual != expected {
        return Err(format!("{label}: expected {expected:?}, actual {actual:?}").into());
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum Case {
    NoDrag,
    NativeDrag,
    NativeCancel,
    Prevented,
    RetireBeforeMove,
    TwoOwners,
    Collision,
}
async fn utility(page: &CdpSession, events: &mut CdpEventReceiver) -> TestResult<String> {
    let frame = page.page_get_frame_tree().await?.frame.id;
    let id = page
        .page_create_isolated_world(&frame, "__pwright_drag_observer_test", false)
        .await?;
    let unique = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = events
                .recv()
                .await
                .map_err(|e| format!("world event: {e:?}"))?;
            if event.session_id.as_deref() == page.session_id()
                && event.method == "Runtime.executionContextCreated"
                && event.params["context"]["id"] == id
            {
                return event.params["context"]["uniqueId"]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or("unique world ID absent".to_string());
            }
        }
    })
    .await??;
    Ok(unique)
}
async fn pointer(page: &CdpSession, kind: &str, x: f64, held: bool) -> TestResult<()> {
    page.input_dispatch_pointer_event(DispatchMouseEventParams {
        r#type: kind.into(),
        x,
        y: 40.0,
        button: Some(if held || kind != "mouseMoved" {
            MouseButton::Left
        } else {
            MouseButton::None
        }),
        buttons: Some(i64::from(held)),
        click_count: (kind != "mouseMoved").then_some(1),
        force: Some(if held { 0.5 } else { 0.0 }),
        ..Default::default()
    })
    .await?;
    Ok(())
}
async fn registry_size(page: &CdpSession, unique: &str) -> TestResult<u64> {
    let result = page
        .runtime_evaluate_in_unique_context(
            "globalThis.__pwrightDragObserversV1?.size ?? 0",
            unique,
        )
        .await?;
    result["result"]["value"]
        .as_u64()
        .ok_or_else(|| "registry size absent".into())
}
async fn listeners(page: &CdpSession, unique: &str) -> TestResult<(usize, usize)> {
    let object = page
        .runtime_evaluate_as_object_in_unique_context("window", unique)
        .await?;
    let object = object["result"]["objectId"]
        .as_str()
        .ok_or("window object absent")?;
    let reply = page
        .send("DOMDebugger.getEventListeners", json!({"objectId":object}))
        .await;
    page.runtime_release_object(object).await?;
    let reply = reply?;
    let values = reply["listeners"]
        .as_array()
        .ok_or("native listeners absent")?;
    if values.len() > 64 {
        return Err("unexpected fixture listener count".into());
    }
    let mut counts = (0, 0);
    for listener in values {
        match listener["type"].as_str() {
            Some("mousemove") => counts.0 += 1,
            Some("dragstart") => counts.1 += 1,
            _ => {}
        }
        for key in ["handler", "originalHandler"] {
            if let Some(id) = listener[key]["objectId"].as_str() {
                page.runtime_release_object(id).await?;
            }
        }
    }
    Ok(counts)
}
fn resources() -> TestResult<ProductionResourceLedger> {
    let req = DragListenerFactory::requirements(2)?
        .combine_concurrent(data_profile()?.requirements())
        .combine_concurrent(DragCaptureFactory::requirements())
        .combine_concurrent(DragMove::requirements(1)?)
        .combine_concurrent(DragPage::requirements(2)?);
    let bytes = req.class(ResourceClass::BackendStateBytes);
    Ok(ProductionResourceLedger::new(
        ResourceLimits::deny_all(req.scopes, req.leases).with_class(
            ResourceClass::BackendStateBytes,
            bytes.global,
            bytes.per_scope,
        ),
        req,
    )?)
}

fn data_profile() -> TestResult<DragDataProfile> {
    Ok(DragDataProfile::new(
        ResourceScope::extension(14, 61),
        DragDataLimits {
            items: 8,
            files: 8,
            text_bytes: 65536,
        },
        NonZeroUsize::new(1).unwrap(),
    )?)
}

async fn run_case(
    browser: &CdpSession,
    connection: &Arc<CdpConnection>,
    fixture: &str,
    case: Case,
) -> TestResult<()> {
    let context = browser.target_create_browser_context(None, None).await?;
    let mut target = None;
    let mut page_owner = None;
    let mut owners: Vec<DragListener> = Vec::with_capacity(2);
    let mut capture_owner: Option<DragCapture> = None;
    let resources = resources()?;
    let mut duplicate_resources = None;
    let result = async {
        let id = browser
            .target_create_in_context("about:blank", &context)
            .await?;
        target = Some(id.clone());
        let session = browser.target_attach(&id).await?;
        let page = Arc::new(CdpSession::new(connection.clone(), session, id));
        page_owner = Some(page.clone());
        let mut events = page.subscribe_composable_events()?;
        page.page_enable().await?;
        page.runtime_enable().await?;
        page.page_navigate(fixture).await?;
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if page
                    .runtime_evaluate("typeof window.dragFixture === 'object'")
                    .await?["result"]["value"]
                    == true
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            Ok::<_, Box<dyn Error>>(())
        })
        .await??;
        let unique = utility(&page, &mut events).await?;
        let baseline = listeners(&page, &unique).await?;
        let factory = DragListenerFactory::new(
            "native_test",
            ResourceScope::extension(14, 60),
            2,
            &resources,
        )?;
        let dragging = matches!(case, Case::NativeDrag | Case::Prevented);
        if matches!(case, Case::Prevented) {
            page.runtime_evaluate("window.dragFixture.prevented = true")
                .await?;
        }
        pointer(&page, "mouseMoved", 40.0, false).await?;
        if dragging {
            pointer(&page, "mousePressed", 40.0, true).await?;
        }
        owners.push(factory.start(page.clone(), &unique)?);
        owners[0].prepare().await?;
        if matches!(case, Case::TwoOwners) {
            owners.push(factory.start(page.clone(), &unique)?);
            owners[1].prepare().await?;
        }
        if matches!(case, Case::Collision) {
            duplicate_resources = Some(self::resources()?);
            let duplicate = DragListenerFactory::new(
                "native_test",
                ResourceScope::extension(14, 60),
                2,
                duplicate_resources
                    .as_ref()
                    .ok_or("duplicate resources absent")?,
            )?;
            owners.push(duplicate.start(page.clone(), &unique)?);
            check(
                owners[1].prepare().await.is_err(),
                true,
                "duplicate rejected",
            )?;
            check(
                owners[1].is_retired(),
                true,
                "duplicate confirmed not installed",
            )?;
            owners[1].retire().await?;
            check(
                registry_size(&page, &unique).await?,
                1,
                "original token preserved",
            )?;
        }
        let installed = listeners(&page, &unique).await?;
        check(
            installed.0,
            baseline.0 + owners.iter().filter(|owner| !owner.is_retired()).count(),
            "installed mouse listeners",
        )?;
        if matches!(case, Case::RetireBeforeMove) {
            owners[0].retire().await?;
        }
        let store = DragDataStore::new(data_profile()?, &resources)?;
        let capture_factory = DragCaptureFactory::new(
            page.clone(),
            store.clone(),
            ResourceScope::extension(14, 62),
            &resources,
        )?;
        capture_owner = Some(capture_factory.start()?);
        capture_owner
            .as_mut()
            .ok_or("capture absent")?
            .enable()
            .await?;
        // Act: actual trusted native movement, not synthetic dragstart events.
        pointer(&page, "mouseMoved", 70.0, dragging).await?;
        let observed = if matches!(case, Case::RetireBeforeMove) {
            false
        } else {
            owners[0].observe().await?
        };
        // Assert
        check(
            observed,
            matches!(case, Case::NativeDrag),
            "drag observation",
        )?;
        if matches!(case, Case::TwoOwners) {
            check(
                registry_size(&page, &unique).await?,
                1,
                "other owner retained",
            )?;
            check(owners[1].observe().await?, false, "other owner observation")?;
        }
        check(registry_size(&page, &unique).await?, 0, "empty registry")?;
        check(
            listeners(&page, &unique).await?,
            baseline,
            "native listeners restored",
        )?;
        check(
            owners.iter().all(DragListener::is_retired),
            true,
            "owners retired",
        )?;
        if observed {
            let data = tokio::time::timeout(
                Duration::from_secs(5),
                capture_owner.as_mut().ok_or("capture absent")?.receive(),
            )
            .await??;
            check(store.active_records(), 1, "admitted drag record")?;
            check(
                data.session_id(),
                page.session_id().ok_or("Page session absent")?,
                "drag origin session",
            )?;
            check(
                data.data().items.iter().any(|item| {
                    item.mime_type == "text/plain" && item.data == "pwright-drag-observer"
                }),
                true,
                "intercepted drag data",
            )?;
            capture_owner
                .as_mut()
                .ok_or("capture absent")?
                .retire()
                .await?;
            check(
                capture_owner.as_ref().ok_or("capture absent")?.is_retired(),
                true,
                "interception disable receipt",
            )?;
            for (kind, x) in [("dragEnter", 70), ("dragOver", 220), ("drop", 220)] {
                page.send(
                    "Input.dispatchDragEvent",
                    json!({"type":kind,"x":x,"y":40,"data":data.data()}),
                )
                .await?;
            }
            check(
                &page.runtime_evaluate("window.dragFixture.dropped").await?["result"]["value"],
                &json!("pwright-drag-observer"),
                "dropped payload",
            )?;
            drop(data);
            drop(capture_owner.take());
            check(
                store.active_records(),
                0,
                "drag record released after native dispatch",
            )?;
        } else {
            capture_owner
                .as_mut()
                .ok_or("capture absent")?
                .retire()
                .await?;
            check(
                capture_owner.as_ref().ok_or("capture absent")?.is_retired(),
                true,
                "non-drag interception disable receipt",
            )?;
        }
        check(
            &page.runtime_evaluate("window.dragFixture.overflow").await?["result"]["value"],
            &json!(false),
            "fixture overflow",
        )?;
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    // Native cleanup is independent of result assertions. Keep listener owners
    // until exact Context/target destruction is independently confirmed.
    let mut failures = Vec::new();
    if let Err(error) = result {
        failures.push(format!("{case:?}: {error}"));
    }
    for owner in &mut owners {
        if let Err(error) = owner.retire().await {
            failures.push(format!("listener cleanup: {error}"));
        }
    }
    if let Some(capture) = &mut capture_owner
        && let Err(error) = capture.retire().await
    {
        failures.push(format!("capture cleanup: {error}"));
    }
    if let Some(page) = &page_owner
        && let Err(error) = page
            .send(
                "Input.dispatchDragEvent",
                json!({"type":"dragCancel", "x":70, "y":40,
            "data":{"items":[],"dragOperationsMask":65535}}),
            )
            .await
    {
        failures.push(format!("drag cancel: {error}"));
    }
    browser.target_dispose_browser_context(&context).await?;
    if browser
        .target_get_browser_contexts()
        .await?
        .contains(&context)
        || browser
            .target_get_targets()
            .await?
            .iter()
            .any(|info| Some(&info.target_id) == target.as_ref())
    {
        return Err("owned drag Context/target remains".into());
    }
    for owner in &mut owners {
        if !owner.is_retired() {
            let unique = owner.unique_context().to_owned();
            owner.acknowledge_destroyed_world(&unique)?;
        }
    }
    if let Some(capture) = &mut capture_owner
        && !capture.is_retired()
    {
        let exact: Arc<dyn DragInterceptionClient> =
            page_owner.as_ref().ok_or("capture Page absent")?.clone();
        capture.acknowledge_destroyed_session(&exact)?;
    }
    drop(capture_owner);
    drop(owners);
    check(
        resources.metrics()?.active_leases,
        0,
        "drained resource leases",
    )?;
    if let Some(resources) = duplicate_resources {
        check(
            resources.metrics()?.active_leases,
            0,
            "duplicate resource leases",
        )?;
    }
    if !failures.is_empty() {
        return Err(failures.join("\n").into());
    }
    println!(
        "drag listener {case:?}: native behavior, listener removal and Context absence verified"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight and verified serve-lib raw-drag fixture"]
async fn native_drag_observation_and_exact_listener_cleanup() -> TestResult<()> {
    // Arrange
    let endpoint = std::env::var("PWRIGHT_CDP_WS_URL")?;
    if !endpoint.starts_with("ws://127.0.0.1:9222/devtools/browser/") {
        return Err("loopback Chrome required".into());
    }
    let fixture = std::env::var("PWRIGHT_DRAG_FIXTURE")?;
    let url = url::Url::parse(&fixture)?;
    if url.scheme() != "http" || !url.host_str().is_some_and(|host| host.ends_with(".ts.net")) {
        return Err("verified Tailscale fixture required".into());
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
    // Act / Assert: isolated owned Context per case, no whole-operation timeout
    // that could discard a retained listener/native reply owner.
    for case in [
        Case::NoDrag,
        Case::NativeDrag,
        Case::Prevented,
        Case::RetireBeforeMove,
        Case::TwoOwners,
        Case::Collision,
    ] {
        run_case(&browser, &connection, &fixture, case).await?;
    }
    for case in [
        Case::NoDrag,
        Case::NativeDrag,
        Case::Prevented,
        Case::NativeCancel,
    ] {
        run_composed_case(&browser, &connection, &fixture, case).await?;
    }
    Ok(())
}

async fn run_composed_case(
    browser: &CdpSession,
    connection: &Arc<CdpConnection>,
    fixture: &str,
    case: Case,
) -> TestResult<()> {
    let context = browser.target_create_browser_context(None, None).await?;
    let resources = resources()?;
    let mut target = None;
    let mut page_owner = None;
    let mut unique_owner = None;
    let mut movement: Option<DragMove> = None;
    let mut drag_page_owner = None;
    let mut command_owner = None;
    let result = async {
        // Arrange: the caller selects one already existing utility world.
        let id = browser
            .target_create_in_context("about:blank", &context)
            .await?;
        target = Some(id.clone());
        let session = browser.target_attach(&id).await?;
        let page = Arc::new(CdpSession::new(connection.clone(), session, id));
        page_owner = Some(page.clone());
        let mut events = page.subscribe_composable_events()?;
        page.page_enable().await?;
        page.runtime_enable().await?;
        page.page_navigate(fixture).await?;
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if page
                    .runtime_evaluate("typeof window.dragFixture === 'object'")
                    .await?["result"]["value"]
                    == true
                {
                    return Ok::<_, Box<dyn Error>>(());
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await??;
        let unique = utility(&page, &mut events).await?;
        unique_owner = Some(unique.clone());
        drop(events);
        let baseline = listeners(&page, &unique).await?;
        let held = matches!(
            case,
            Case::NativeDrag | Case::Prevented | Case::NativeCancel
        );
        if matches!(case, Case::Prevented) {
            page.runtime_evaluate("window.dragFixture.prevented = true")
                .await?;
        }
        pointer(&page, "mouseMoved", 40.0, false).await?;
        if held {
            pointer(&page, "mousePressed", 40.0, true).await?;
        }
        let listeners_factory =
            DragListenerFactory::new("composed", ResourceScope::extension(14, 60), 2, &resources)?;
        let store = DragDataStore::new(data_profile()?, &resources)?;
        let captures = DragCaptureFactory::new(
            page.clone(),
            store.clone(),
            ResourceScope::extension(14, 62),
            &resources,
        )?;
        let drag_page = DragPage::new(
            page.clone(),
            captures.clone(),
            ResourceScope::extension(14, 64),
            2,
            &resources,
        )?;
        drag_page_owner = Some(drag_page.clone());
        drag_page.record_position(DragPosition { x: 70.0, y: 40.0 })?;
        let native = page.clone();
        movement = Some(DragMove::start(
            ResourceScope::extension(14, 63),
            &resources,
            &listeners_factory,
            &captures,
            &[DragWorld {
                client: page.clone(),
                unique_context: &unique,
            }],
            Box::pin(async move {
                native
                    .input_dispatch_pointer_event(DispatchMouseEventParams {
                        r#type: "mouseMoved".into(),
                        x: 70.0,
                        y: 40.0,
                        button: Some(if held {
                            MouseButton::Left
                        } else {
                            MouseButton::None
                        }),
                        buttons: Some(i64::from(held)),
                        force: Some(if held { 0.5 } else { 0.0 }),
                        ..Default::default()
                    })
                    .await
            }),
        )?);
        // Act: the Rust composition owns setup, interception, native movement,
        // browser observation, data collection and interception disable.
        let data = tokio::time::timeout(
            Duration::from_secs(5),
            movement.as_mut().ok_or("move absent")?.result(),
        )
        .await??;
        // Assert: callback outcomes plus actual listener/native lifecycle.
        check(
            data.is_some(),
            matches!(case, Case::NativeDrag | Case::NativeCancel),
            "composed drag result",
        )?;
        check(
            movement.as_ref().ok_or("move absent")?.is_retired(),
            true,
            "composed retirement",
        )?;
        check(
            listeners(&page, &unique).await?,
            baseline,
            "composed native listeners restored",
        )?;
        check(
            registry_size(&page, &unique).await?,
            0,
            "composed registry removed",
        )?;
        if let Some(data) = data {
            let enter = drag_page.enter(
                movement.as_ref().ok_or("move absent")?,
                data,
                DragPosition { x: 70.0, y: 40.0 },
                0,
            )?;
            run_page_command(&mut command_owner, enter).await?;
            check(drag_page.is_dragging(), true, "Page stores active drag")?;
            run_page_command(
                &mut command_owner,
                drag_page.over(DragPosition { x: 220.0, y: 40.0 }, 0)?,
            )
            .await?;
            if matches!(case, Case::NativeCancel) {
                run_page_command(&mut command_owner, drag_page.cancel()?).await?;
            } else {
                run_page_command(
                    &mut command_owner,
                    drag_page.drop_at(DragPosition { x: 220.0, y: 40.0 }, 0)?,
                )
                .await?;
            }
            check(
                drag_page.is_dragging(),
                false,
                "Page drag cleared after native receipt",
            )?;
            check(
                &page.runtime_evaluate("window.dragFixture.dropped").await?["result"]["value"],
                &if matches!(case, Case::NativeCancel) {
                    json!(null)
                } else {
                    json!("pwright-drag-observer")
                },
                "composed delivered data",
            )?;
            check(
                &page
                    .runtime_evaluate("window.dragFixture.events.includes('dragend')")
                    .await?["result"]["value"],
                &json!(true),
                "native drag ended",
            )?;
        } else {
            check(
                run_page_command(&mut command_owner, drag_page.cancel()?).await?,
                false,
                "no-drag cancel is no-op",
            )?;
        }
        check(
            &page.runtime_evaluate("window.dragFixture.overflow").await?["result"]["value"],
            &json!(false),
            "composed fixture overflow",
        )?;
        drop(movement.take());
        check(store.active_records(), 0, "composed data released")?;
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    // The timeout above drops only a borrowed result future, never this owner.
    let mut failures = Vec::new();
    if let Err(error) = result {
        failures.push(format!("composed {case:?}: {error}"));
    }
    if let Some(movement) = &mut movement
        && let Err(error) = movement.retire().await
    {
        failures.push(format!("composed cleanup: {error}"));
    }
    if let Some(command) = &mut command_owner
        && let Err(error) = command.retire().await
    {
        failures.push(format!("Page command retirement: {error}"));
    }
    drop(command_owner);
    if let Some(page) = &page_owner
        && let Err(error) = page
            .send(
                "Input.dispatchDragEvent",
                json!({"type":"dragCancel","x":70,"y":40,
            "data":{"items":[],"dragOperationsMask":65535}}),
            )
            .await
    {
        failures.push(format!("composed drag cancel: {error}"));
    }
    browser.target_dispose_browser_context(&context).await?;
    if browser
        .target_get_browser_contexts()
        .await?
        .contains(&context)
        || browser
            .target_get_targets()
            .await?
            .iter()
            .any(|info| Some(&info.target_id) == target.as_ref())
    {
        return Err("composed Context/target remains".into());
    }
    if let Some(movement) = &mut movement
        && !movement.is_retired()
    {
        let exact_world: Arc<dyn pwright_cdp::CdpClient> =
            page_owner.as_ref().ok_or("Page absent")?.clone();
        movement.acknowledge_destroyed_world(
            &exact_world,
            unique_owner.as_deref().ok_or("world absent")?,
        )?;
        let exact: Arc<dyn DragInterceptionClient> =
            page_owner.as_ref().ok_or("Page absent")?.clone();
        movement.acknowledge_destroyed_session(&exact)?;
        check(
            movement.is_retired(),
            true,
            "composed destroyed children retired",
        )?;
    }
    drop(movement);
    drop(drag_page_owner);
    check(
        resources.metrics()?.active_leases,
        0,
        "composed leases released",
    )?;
    if !failures.is_empty() {
        return Err(failures.join("\n").into());
    }
    println!(
        "composed drag {case:?}: native movement/data, listener restoration, Context absence and zero leases verified"
    );
    Ok(())
}

async fn run_page_command(
    owner: &mut Option<DragPageOperation>,
    command: DragPageOperation,
) -> TestResult<bool> {
    *owner = Some(command);
    let command = owner.as_mut().ok_or("Page command absent")?;
    let result = tokio::time::timeout(Duration::from_secs(5), &mut *command).await??;
    check(
        command.is_retired(),
        true,
        "Page command native receipt drained",
    )?;
    drop(owner.take());
    Ok(result)
}
