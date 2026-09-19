use super::*;
use pwright_backend_cdp_drag::{
    DragDataLimits, DragDataProfile, DragDataStore, DragPage, DragPosition,
};
use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_cdp::{CdpSession, MouseButton, MouseEventType};
use pwright_composition_chromium::{
    CdpPageServices, ChromiumAttachFactory, ChromiumFrameWorldAccessProfile,
    ChromiumTopologyEventDriver, transact_topology_update,
};
use pwright_core::ResourceLimits;
use pwright_runtime::DirectDispatcher;
use std::{future::Future, num::NonZeroUsize, time::Duration};
type TestResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn limits(required: ResourceRequirements) -> ResourceLimits {
    let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
    for class in ResourceClass::ALL {
        let bound = required.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    limits
}
fn profile() -> PageDragSourceProfile {
    PageDragSourceProfile::new(
        ResourceScope::extension(121, 1),
        ResourceScope::extension(121, 2),
        2,
        1,
    )
    .unwrap()
}
fn scope_bytes(resources: &ProductionResourceLedger, scope: ResourceScope) -> u64 {
    resources
        .claim(ResourceRequirements::none())
        .unwrap()
        .scope_metrics(scope)
        .unwrap()
        .map_or(0, |metrics| metrics.units(ResourceClass::BackendStateBytes))
}
async fn drive<T, E: std::error::Error + Send + Sync + 'static>(
    future: impl Future<Output = std::result::Result<T, E>>,
    driver: &mut ChromiumTopologyEventDriver,
    runtime: &DirectDispatcher<CdpPageServices>,
) -> TestResult<T> {
    tokio::pin!(future);
    loop {
        tokio::select! {
            result = &mut future => return Ok(result?),
            event = driver.receive_once(runtime) => { event?; }
        }
    }
}
async fn pointer(client: &dyn CdpClient, kind: MouseEventType, x: f64, buttons: i32) -> Result<()> {
    client
        .input_dispatch_mouse_event(
            kind,
            x,
            40.0,
            Some(MouseButton::Left),
            Some(1),
            Some(buttons),
            Some(0),
        )
        .await
}

#[test]
fn source_profile_rejects_unbounded_counts_and_scope_aliasing() {
    // Arrange
    let scope = ResourceScope::extension(121, 1);
    let other = ResourceScope::extension(121, 2);
    // Act
    let invalid = [(0, 1), (33, 1), (1, 0), (1, 33)]
        .map(|(worlds, moves)| PageDragSourceProfile::new(scope, other, worlds, moves));
    // Assert
    assert!(invalid.into_iter().all(|result| result.is_err()));
    assert!(PageDragSourceProfile::new(scope, scope, 1, 1).is_err());
    assert!(profile().requirements().is_ok());
}

#[test]
fn fake_clients_do_not_fabricate_a_native_session_view() {
    // Arrange
    let client: Arc<dyn CdpClient> = Arc::new(pwright_fake::FakeCdpClient::from_html(""));
    // Act
    let native = client.retained_native_session();
    // Assert
    assert!(native.is_none());
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and verified PWRIGHT_DRAG_FIXTURE/PWRIGHT_CDP_WS_URL"]
async fn native_source_retains_current_worlds_and_delivers_drag_data() -> TestResult<()> {
    // Arrange: production attachment and route driver; only an owned Context.
    let fixture = std::env::var("PWRIGHT_DRAG_FIXTURE")?;
    let url = url::Url::parse(&fixture)?;
    if url.scheme() != "http" || !url.host_str().is_some_and(|host| host.ends_with(".ts.net")) {
        return Err("verified Tailscale fixture required".into());
    }
    let ws = std::env::var("PWRIGHT_CDP_WS_URL")?;
    if !ws.starts_with("ws://127.0.0.1:9222/devtools/browser/") {
        return Err("dedicated loopback CDP required".into());
    }
    let access_scope = ResourceScope::extension(121, 3);
    let access_profile =
        ChromiumFrameWorldAccessProfile::new(access_scope, NonZeroUsize::new(4).unwrap())?
            .with_native_retirement()?;
    let source_profile = profile();
    let data_profile = DragDataProfile::new(
        ResourceScope::extension(121, 4),
        DragDataLimits {
            items: 8,
            files: 8,
            text_bytes: 65536,
        },
        NonZeroUsize::new(1).unwrap(),
    )?;
    let extra = access_profile
        .requirements()
        .combine_concurrent(source_profile.requirements()?)
        .combine_concurrent(data_profile.requirements())
        .combine_concurrent(DragListenerFactory::requirements(2)?)
        .combine_concurrent(DragCaptureFactory::requirements())
        .combine_concurrent(DragPage::requirements(4)?);
    let base = crate::profile::attach_factory(Duration::from_secs(5))?;
    let selected = base.profile().clone().with_extension_requirements(extra);
    let factory = ChromiumAttachFactory::new(
        selected.clone(),
        limits(selected.requirements()),
        ObjectId::new(1),
    )?;
    let mut composition = factory
        .attach(pwright_provider_cdp::CdpWebSocketEndpoint::parse(ws)?)
        .await?;
    let runtime = composition.new_page_runtime()?;
    let mut driver = composition
        .topology_event_driver()?
        .ok_or("missing topology driver")?;
    let resources = composition.resources().clone();
    let access = ChromiumFrameWorldAccess::new(runtime.service(), access_profile, &resources)?;
    let browser = CdpSession::browser(composition.connection().clone());
    let context = browser.target_create_browser_context(None, None).await?;
    let outcome = tokio::time::timeout(Duration::from_secs(20), async {
        {
            let mut topology = composition.topology()?;
            let epoch = topology.topology().epoch();
            // Explicit test ID outside the bounded attachment driver's small
            // sequential namespace; no random/global identity source.
            transact_topology_update(&mut topology, &runtime, |topology| {
                topology.register_context(
                    epoch,
                    &context,
                    ObjectId::new(1_000_000),
                    CdpContextOwnership::Owned,
                )
            })?;
        }
        let target = browser.target_create_in_context(&fixture, &context).await?;
        driver.initialize_snapshot(&runtime, 4096).await?;
        let (frame, page_id) = loop {
            let found = {
                let topology = composition.topology()?;
                topology
                    .topology()
                    .main_frame_object_id(&target)
                    .zip(topology.topology().page_object_id(&target))
            };
            if let Some(found) = found {
                break found;
            }
            driver.receive_once(&runtime).await?;
        };
        // Target creation acknowledges before the document's parser/scripts.
        // Observe the actual fixture precondition before starting any input.
        let pair = loop {
            let candidate = drive(access.snapshot(frame), &mut driver, &runtime).await?;
            let ready = drive(candidate.client().runtime_evaluate_in_unique_context(
                "document.readyState !== 'loading' && typeof dragFixture === 'object'",
                candidate.main_unique_id(),
            ), &mut driver, &runtime).await?;
            if ready["result"]["value"] == true { break candidate; }
            drop(candidate);
            tokio::select! {
                event = driver.receive_once(&runtime) => { event?; }
                () = tokio::time::sleep(Duration::from_millis(5)) => {}
            }
        };
        let (client, _) = access
            .page_input_binding(page_id)?
            .ok_or("missing Page input")?
            .into_parts();
        let native = client
            .clone()
            .retained_native_session()
            .ok_or("missing native session view")?;
        let same: Arc<dyn CdpClient> = native.clone();
        if !Arc::ptr_eq(&same, &client) {
            return Err("native session view changed allocation".into());
        }
        drop(pair);
        let baseline = scope_bytes(&resources, access_scope);
        let store = DragDataStore::new(data_profile, &resources)?;
        let captures = DragCaptureFactory::new(
            native.clone(),
            store.clone(),
            ResourceScope::extension(121, 5),
            &resources,
        )?;
        let listeners =
            DragListenerFactory::new("node043", ResourceScope::extension(121, 6), 2, &resources)?;
        let source = PageDragMoveSource::new(
            access.clone(),
            page_id,
            listeners,
            captures.clone(),
            source_profile,
            &resources,
        )?;
        let page = DragPage::new(
            native.clone(),
            captures,
            ResourceScope::extension(121, 7),
            4,
            &resources,
        )?;

        // Unpolled cancellation and rejected overlap retain/release exact owners.
        let mut unpolled = source.start(Box::pin(async { panic!("unpolled movement ran") }))?;
        if source.start(Box::pin(async { Ok(()) })).is_ok() {
            return Err("overlap exceeded source capacity".into());
        }
        unpolled.retire().await?;
        if source.retained_moves() != 1 || scope_bytes(&resources, access_scope) != baseline + 8192
        {
            return Err("result/retirement released retained world budget early".into());
        }
        drop(unpolled);
        if source.retained_moves() != 0 || scope_bytes(&resources, access_scope) != baseline {
            return Err("unpolled source budget leaked".into());
        }

        pointer(client.as_ref(), MouseEventType::Moved, 40.0, 0).await?;
        pointer(client.as_ref(), MouseEventType::Pressed, 40.0, 1).await?;
        let moved = client.clone();
        let mut movement = source.start(Box::pin(async move {
            pointer(moved.as_ref(), MouseEventType::Moved, 70.0, 1).await
        }))?;
        // Act: the actual source captures native HTML drag data.
        let data = drive(movement.result(), &mut driver, &runtime)
            .await?
            .ok_or("no intercepted drag data")?;
        page.enter(&movement, data, DragPosition { x: 70.0, y: 40.0 }, 0)?
            .await?;
        drop(movement);
        page.over(DragPosition { x: 220.0, y: 40.0 }, 0)?.await?;
        page.drop_at(DragPosition { x: 220.0, y: 40.0 }, 0)?.await?;
        pointer(client.as_ref(), MouseEventType::Released, 220.0, 0).await?;
        let result = client.runtime_evaluate("dragFixture").await?;
        // Assert
        if result["result"]["value"]["dropped"] != "pwright-drag-observer" || page.is_dragging() {
            return Err("native source drag/drop outcome differs".into());
        }
        if source.retained_moves() != 0 || scope_bytes(&resources, access_scope) != baseline {
            return Err("completed move retained source/snapshot admission".into());
        }
        // A later move must acquire newly observed children, not reuse its first
        // batch. Keep native listeners on that child through its real removal.
        client.runtime_evaluate("{ const f = document.createElement('iframe'); f.id = 'source-child'; f.srcdoc = '<p>child</p>'; document.body.append(f); }").await?;
        let child = loop {
            let found = {
                let topology = composition.topology()?;
                topology.topology().frame_page_routes().find_map(|(candidate, owner)|
                    (owner == page_id && candidate != frame).then_some(candidate))
            };
            if let Some(found) = found { break found; }
            driver.receive_once(&runtime).await?;
        };
        drop(drive(access.snapshot(child), &mut driver, &runtime).await?);
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let signal = started.clone();
        let gate = release.clone();
        let mut parked = source.start(Box::pin(async move {
            signal.notify_one();
            gate.notified().await;
            Ok(())
        }))?;
        if scope_bytes(&resources, access_scope) != baseline + 2 * 8192 {
            return Err("new move did not retain the fresh two-world snapshot".into());
        }
        drive(async {
            tokio::select! {
                result = parked.result() => { result?; Err(error("parked movement completed early")) },
                () = started.notified() => Ok(()),
            }
        }, &mut driver, &runtime).await?;
        let selected = access.existing_page_utility_worlds(page_id, 2)?;
        let destroyed = selected.worlds().iter().find(|world| world.frame() == child)
            .and_then(|world| world.native_lifetime()).ok_or("child destruction proof missing")?;
        drop(selected);
        client.runtime_evaluate("document.getElementById('source-child').remove()").await?;
        while !destroyed.world_destroyed() { driver.receive_once(&runtime).await?; }
        // Real destruction alone may remove listeners, but cannot release the
        // retained native movement future or its Frame snapshot admissions.
        let state = parked.poll_retire(&mut std::task::Context::from_waker(std::task::Waker::noop()));
        if !state.is_pending() || source.retained_moves() != 1 {
            return Err("world destruction discarded the pending movement owner".into());
        }
        release.notify_one();
        drive(parked.retire(), &mut driver, &runtime).await?;
        if !parked.is_retired() { return Err("destruction-backed source cleanup remained dirty".into()); }
        drop((parked, destroyed));
        if source.retained_moves() != 0 || scope_bytes(&resources, access_scope) != baseline {
            return Err("destruction-backed retirement leaked source/snapshot admission".into());
        }
        drop((page, source, store));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(target)
    })
    .await;
    // Always dispose only this test's Context; never drop a failed assertion
    // before making the independent native cleanup attempt.
    let cleanup = browser.target_dispose_browser_context(&context).await;
    cleanup?;
    let target = outcome??;
    if browser
        .target_get_targets()
        .await?
        .iter()
        .any(|candidate| candidate.target_id == target)
    {
        return Err("owned target survived Context cleanup".into());
    }
    Ok(())
}
