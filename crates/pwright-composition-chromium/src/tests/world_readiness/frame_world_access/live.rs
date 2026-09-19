use super::*;
use crate::ChromiumTopologyEventDriver;

type LiveResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn drive<T>(
    future: impl Future<Output = Result<T, FrameWorldAccessError>>,
    driver: &mut ChromiumTopologyEventDriver,
    runtime: &DirectDispatcher<CdpPageServices>,
) -> LiveResult<T> {
    tokio::pin!(future);
    loop {
        tokio::select! {
            result = &mut future => return Ok(result?),
            event = driver.receive_once(runtime) => { event?; }
        }
    }
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_CDP_WS_URL"]
async fn production_world_pairs_follow_real_navigation_child_detach_and_owned_context_close()
-> LiveResult<()> {
    // Arrange: the real production topology driver owns all world observations.
    // One externally supplied browser and one test-owned Context; no restart.
    // The shared dedicated browser can contain unrelated task Pages. Declare
    // both selected worlds for up to eight Frames, and use the same world-event
    // selection as the Node host. Do not close other tasks to fit a tiny fixture.
    let mut base = base_profile_with_runtime_and_topology(
        RuntimeLimits::new(16, 4).with_active_commands(2),
        CdpTopologyLimits::new(16, 4, 4, 8, 16, 128),
    )
    .with_dom_content(dom_content_profile())
    .with_locator(locator_profile())
    .with_evaluation(evaluation_profile());
    base.connection = base
        .connection
        .with_event_admission_policy(crate::chromium_frame_world_event_admission())?;
    let access_profile = ChromiumFrameWorldAccessProfile::new(
        ResourceScope::extension(98, 3),
        NonZeroUsize::new(4).unwrap(),
    )?;
    let events = ChromiumTopologyEventDriverProductionProfile::from_profiles(
        ResourceScope::extension(98, 1),
        base.connection(),
        base.topology(),
    )?;
    let readiness = ChromiumExecutionWorldReadinessProfile::from_topology(
        ResourceScope::extension(98, 2),
        base.topology(),
        NonZeroUsize::new(3).unwrap(),
    );
    let discovery =
        ChromiumPageDiscoveryProductionProfile::from_profiles(base.connection(), base.topology());
    let profile = base
        .with_topology_events(events)
        .with_page_discovery(discovery)
        .with_utility_world(ChromiumUtilityWorldProfile::playwright_compatible())
        .with_execution_world_readiness(readiness)
        .with_extension_requirements(access_profile.requirements());
    let factory = ChromiumAttachFactory::new(
        profile.clone(),
        exact_limits(profile.requirements()),
        ObjectId::new(1),
    )?;
    let endpoint = CdpWebSocketEndpoint::parse(std::env::var("PWRIGHT_CDP_WS_URL")?)?;
    let mut composition = factory.attach(endpoint).await?;
    let runtime = composition.new_page_runtime()?;
    let mut driver = composition
        .topology_event_driver()?
        .ok_or("topology driver not selected")?;
    let access =
        ChromiumFrameWorldAccess::new(runtime.service(), access_profile, composition.resources())?;
    let ledger = composition
        .resources()
        .claim(access_profile.requirements())?;
    let owner_bytes = ledger
        .scope_metrics(ResourceScope::extension(98, 3))?
        .ok_or("missing access scope")?
        .units(ResourceClass::BackendStateBytes);
    let browser = CdpSession::browser(composition.connection().clone());
    let context = browser.target_create_browser_context(None, None).await?;

    let outcome = tokio::time::timeout(Duration::from_secs(20), async {
        // Foreign nondefault Contexts are intentionally not auto-claimed by
        // discovery. Explicitly register only the Context created by this test,
        // using the same identity source as the production discovery driver.
        let context_object = composition.object_ids.next_object_id()?;
        {
            let mut topology = composition.topology()?;
            let epoch = topology.topology().epoch();
            crate::transact_topology_update(&mut topology, &runtime, |topology| {
                topology.register_context(epoch, &context, context_object, CdpContextOwnership::Owned)
            })?;
        }
        let target = browser.target_create_in_context("about:blank", &context).await?;
        driver.initialize_snapshot(&runtime, 1024).await?;
        let (frame, page) = loop {
            let found = {
                let topology = composition.topology()?;
                topology.topology().main_frame_object_id(&target)
                    .zip(topology.topology().page_object_id(&target))
            };
            if let Some(found) = found { break found; }
            driver.receive_once(&runtime).await?;
        };
        let mut first = drive(access.snapshot(frame), &mut driver, &runtime).await?;
        first.client().runtime_evaluate_in_unique_context("globalThis.__pwMainWorld=42", first.main_unique_id()).await?;
        let isolated = first.client().runtime_evaluate_in_unique_context("globalThis.__pwMainWorld === undefined", first.utility_unique_id()).await?;
        if isolated["result"]["value"] != true { return Err("utility route unexpectedly targets main world".into()); }

        // Act: replace the document while retaining its exact old paired route.
        first.client().page_navigate("data:text/html,<title>pwright-world-pair</title><iframe id=child srcdoc='<p>child</p>'></iframe>").await?;
        drive(first.wait_invalidated(), &mut driver, &runtime).await?;
        let second = drive(access.snapshot(frame), &mut driver, &runtime).await?;
        let replaced = first.main_unique_id() != second.main_unique_id()
            && first.utility_unique_id() != second.utility_unique_id()
            && !first.is_current()? && second.is_current()?;
        let stale_main_rejected = first.client().runtime_evaluate_in_unique_context("1", first.main_unique_id()).await.is_err();
        let stale_utility_rejected = first.client().runtime_evaluate_in_unique_context("1", first.utility_unique_id()).await.is_err();
        drop(first);

        let child_frame = loop {
            let child = composition.topology()?.topology().frame_page_routes()
                .find_map(|(candidate, owner)| (owner == page && candidate != frame).then_some(candidate));
            if let Some(child) = child { break child; }
            driver.receive_once(&runtime).await?;
        };
        let mut child = drive(access.snapshot(child_frame), &mut driver, &runtime).await?;
        let child_identity = child.page() == page && child.frame() != frame
            && child.main_unique_id() != second.main_unique_id();
        let existing = access.existing_page_utility_worlds(page, 2)?;
        if existing.worlds().len() != 2 { return Err("existing utility snapshot missed a live Frame".into()); }
        for world in existing.worlds() {
            let expected = if world.frame() == frame { &second } else { &child };
            if world.utility_unique_id() != expected.utility_unique_id() || !world.is_current()? {
                return Err("existing utility snapshot selected a different Frame/world".into());
            }
            let value = world.client().runtime_evaluate_in_unique_context(
                "globalThis.__pwUtilitySnapshot = 42; globalThis === window", world.utility_unique_id()).await?;
            if value["result"]["value"] != true { return Err("existing utility world evaluation failed".into()); }
            let main = expected.client().runtime_evaluate_in_unique_context(
                "globalThis.__pwUtilitySnapshot === undefined", expected.main_unique_id()).await?;
            if main["result"]["value"] != true { return Err("utility snapshot leaked into main world".into()); }
        }
        second.client().runtime_evaluate_in_unique_context("document.getElementById('child').remove()", second.main_unique_id()).await?;
        let child_closed = matches!(drive(child.wait_invalidated(), &mut driver, &runtime).await,
            Err(error) if error.downcast_ref::<FrameWorldAccessError>().is_some_and(|error|
                matches!(error, FrameWorldAccessError::Topology(CdpPageRouteError::WorldTargetClosed { .. }))));
        let old_child = existing.worlds().iter().find(|world| world.frame() == child_frame)
            .ok_or("missing retained child utility snapshot")?;
        if old_child.is_current()? { return Err("detached child utility snapshot remained current".into()); }
        drop((existing, child, second));
        let returned_to_baseline = ledger.scope_metrics(ResourceScope::extension(98, 3))?
            .ok_or("access owner disappeared")?.units(ResourceClass::BackendStateBytes) == owner_bytes;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((replaced, stale_main_rejected, stale_utility_rejected, child_identity, child_closed, returned_to_baseline, frame))
    }).await;

    // Always attempt owned native cleanup before asserting the observations.
    let cleanup = tokio::time::timeout(
        Duration::from_secs(3),
        browser.target_dispose_browser_context(&context),
    )
    .await;
    let browser_alive = composition
        .connection()
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .is_ok();
    cleanup??;
    let (replaced, stale_main, stale_utility, child_identity, child_closed, returned, frame) =
        outcome??;
    let closed = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            driver.receive_once(&runtime).await?;
            if runtime
                .service()
                .routes
                .lock()
                .map_err(|_| "route state poisoned")?
                .readiness_frames
                .binary_search_by_key(&frame, |(frame, _)| *frame)
                .is_err()
            {
                break Ok::<_, Box<dyn std::error::Error + Send + Sync>>(());
            }
        }
    })
    .await;
    closed??;
    let no_frame = access.snapshot(frame).await;
    drop(access);
    let access_released = ledger
        .scope_metrics(ResourceScope::extension(98, 3))?
        .is_none();

    // Assert: real driver publication, not a synthetic single-Frame route map.
    assert!(replaced && stale_main && stale_utility && child_identity && child_closed);
    assert!(returned && access_released && browser_alive);
    assert!(matches!(
        no_frame,
        Err(FrameWorldAccessError::Topology(
            CdpPageRouteError::WorldTargetClosed { .. }
        ))
    ));
    Ok(())
}
