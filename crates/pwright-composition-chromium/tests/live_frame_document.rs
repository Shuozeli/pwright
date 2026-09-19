//! Production event-driver document retirement, without a Node or browser owner.
use pwright_backend_cdp::{
    CdpContextOwnership, CdpNavigationLimits, CdpNavigationProductionProfile, CdpPageObjectIds,
    CdpPageTarget, CdpTopologyLimits, CdpTopologyProductionProfile, ConnectionEpoch,
};
use pwright_browser_model::{BROWSER_CONTEXT_OBJECT_KIND, FRAME_OBJECT_KIND, PAGE_OBJECT_KIND};
use pwright_cap_navigation::BeginNavigationParams;
use pwright_cdp::{CdpConnectionProductionProfile, CdpSession, connection::ConnectionConfig};
use pwright_codec_navigation::NavigationCodecLimits;
use pwright_composition_chromium::{
    CapabilityWireProfile, CdpPageRouteProductionProfile, ChromiumAttachProductionProfile,
    ChromiumFrameNavigationAccess, ChromiumFrameNavigationProfile,
    ChromiumTopologyEventDriverProductionProfile, NavigationInstallationProfile,
    PreparedChromiumAttach,
};
use pwright_core::{ObjectId, ProtocolVersion, ResourceClass, ResourceLimits, ResourceScope};
use pwright_provider_cdp::CdpWebSocketEndpoint;
use pwright_runtime::RuntimeLimits;
use pwright_server_wire::ServerSupervisorLimits;
use pwright_transport::FrameLimits;
use std::{error::Error, num::NonZeroUsize, sync::Arc, time::Duration};

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[cfg(feature = "frame-lifecycle")]
#[path = "live_frame_document/lifecycle.rs"]
mod lifecycle;

#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight, WebSocket URL and served fixture base"]
async fn live_document_navigation_preserves_parent_and_sibling_runtime_objects() -> TestResult<()> {
    run_document_navigation(false).await
}

#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight, WebSocket URL and served fixture base"]
async fn cross_origin_document_preserves_frame_but_retires_renderer_snapshot() -> TestResult<()> {
    run_document_navigation(true).await
}

async fn run_document_navigation(cross_origin: bool) -> TestResult<()> {
    // Arrange: a production composition with one explicitly owned test Context.
    let ws = std::env::var("PWRIGHT_CDP_WS_URL")?;
    let fixture = std::env::var("PWRIGHT_FRAME_DOCUMENT_FIXTURE")?;
    assert!(
        fixture.starts_with("http://macbookpro-yuacx.tail8f3b66.ts.net:") && fixture.ends_with('/')
    );
    assert!(ws.starts_with("ws://127.0.0.1:9222/devtools/browser/"));
    let connection =
        CdpConnectionProductionProfile::new(ResourceScope::Connection, ConnectionConfig::default());
    let topology = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(64, 1, 1, 8, 32, 128),
        ResourceScope::extension(93, 1),
    )?;
    let events = ChromiumTopologyEventDriverProductionProfile::from_profiles(
        ResourceScope::extension(93, 3),
        &connection,
        topology,
    )?;
    let navigation_routes = ChromiumFrameNavigationProfile::new(
        ResourceScope::extension(93, 5),
        NonZeroUsize::new(8).unwrap(),
        NonZeroUsize::new(4).unwrap(),
        NonZeroUsize::new(128).unwrap(),
    )?;
    let profile = ChromiumAttachProductionProfile::new(
        connection,
        topology,
        CdpPageRouteProductionProfile::from_topology(ResourceScope::extension(93, 2), topology),
        RuntimeLimits::new(64, 2).with_control_messages(256),
        ServerSupervisorLimits::new(1)?,
    )
    .with_topology_events(events)
    .with_navigation(NavigationInstallationProfile::new(
        CdpNavigationProductionProfile::new(
            ResourceScope::extension(93, 4),
            CdpNavigationLimits::new(4096, 128, 16, 16, 4096, 128),
            1,
        ),
        NavigationCodecLimits::new(8192, 16, 4096),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(16384, 0, 0, 16384),
            FrameLimits::new(16384, 0, 0, 16384),
        ),
    ))
    .with_extension_requirements(navigation_routes.requirements());
    #[cfg(feature = "frame-lifecycle")]
    let profile = profile.with_extension_requirements(lifecycle::profile()?.requirements());
    let requirements = profile.requirements();
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let capacity = requirements.class(class);
        limits = limits.with_class(class, capacity.global, capacity.per_scope);
    }
    let mut composition = PreparedChromiumAttach::new(profile, limits)?
        .attach(
            CdpWebSocketEndpoint::parse(ws)?,
            ObjectId::new(100),
            ConnectionEpoch::new(1),
        )
        .await?;
    let connection = composition.connection().clone();
    let browser = CdpSession::browser(connection.clone());
    let owner = browser.target_create_browser_context(None, None).await?;
    println!("owned document-topology Context {owner}");
    let run_owner = owner.clone();
    let mut run = tokio::spawn(async move {
        let browser = CdpSession::browser(connection.clone());
        let target = browser
            .target_create_in_context("about:blank", &run_owner)
            .await?;
        let session = browser.target_attach(&target).await?;
        let page = Arc::new(CdpSession::new(connection, session.clone(), target.clone()));
        let epoch = ConnectionEpoch::new(1);
        composition.with_topology_mut(|t| {
            t.register_context(
                epoch,
                run_owner.clone(),
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
        })??;
        composition.with_topology_mut(|t| {
            t.attach_page(
                epoch,
                CdpPageTarget::new(target.clone(), session, run_owner),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
        })??;
        let runtime = composition.new_page_runtime()?;
        let navigation = ChromiumFrameNavigationAccess::new(
            runtime.service(),
            navigation_routes,
            composition.resources(),
        )?;
        for (id, parent, kind) in [
            (2, 100, BROWSER_CONTEXT_OBJECT_KIND.tag),
            (3, 2, PAGE_OBJECT_KIND.tag),
            (4, 3, FRAME_OBJECT_KIND.tag),
        ] {
            runtime.register_object(ObjectId::new(id), Some(ObjectId::new(parent)), kind)?;
        }
        let mut driver = composition
            .topology_event_driver()?
            .ok_or("missing driver")?;
        #[cfg(feature = "frame-lifecycle")]
        let observation = lifecycle::Probe::new(composition.resources())?;
        #[cfg(feature = "frame-lifecycle")]
        {
            driver = driver.with_frame_event_observer(observation.observer())?;
        }
        driver.initialize_snapshot(&runtime, 128).await?;
        page.page_navigate(&format!("{fixture}frame-navigation.html?mode=local"))
            .await?;
        loop {
            driver.receive_once(&runtime).await?;
            if composition
                .topology()?
                .topology()
                .frame_page_routes()
                .count()
                == 4
            {
                break;
            }
        }
        let tree = page.page_get_frame_tree().await?;
        driver.initialize_snapshot(&runtime, 128).await?;
        let children = tree.child_frames.ok_or("missing children")?;
        assert_eq!(children.len(), 2);
        let child = &children[0].frame.id;
        let nested = &children[0]
            .child_frames
            .as_ref()
            .ok_or("missing nested Frame")?[0]
            .frame
            .id;
        let sibling = &children[1].frame.id;
        let ids = {
            let t = composition.topology()?;
            [child, nested, sibling].map(|id| {
                t.topology()
                    .frame_object_id(id)
                    .expect("native Frame must be published")
            })
        };
        assert!(ids.iter().all(|id| runtime.is_object_live(*id) == Ok(true)));
        #[cfg(feature = "frame-lifecycle")]
        let previous_document = {
            observation
                .wait_loaded(&mut driver, &runtime, ObjectId::new(4), None)
                .await?;
            assert!(observation.responses() > 0);
            observation
                .owner
                .snapshot(ids[0])?
                .ok_or("missing child lifecycle")?
                .document
        };

        let selected_route = navigation.snapshot(ids[0])?;
        let nested_route = navigation.snapshot(ids[1])?;
        let sibling_route = navigation.snapshot(ids[2])?;

        // Act: actual document commit on the child, with the normal event driver.
        let destination = if cross_origin {
            "data:text/html,new-child".to_owned()
        } else {
            format!("{fixture}empty.html?node068=child")
        };
        selected_route
            .begin_navigation(BeginNavigationParams::new(destination))
            .await?;
        loop {
            driver.receive_once(&runtime).await?;
            let renderer_ready = !cross_origin
                || navigation
                    .snapshot(ids[0])
                    .is_ok_and(|route| route.session_id() != selected_route.session_id());
            if !runtime.is_object_live(ids[1])? && renderer_ready {
                break;
            }
        }
        page.page_get_frame_tree().await?;
        driver.initialize_snapshot(&runtime, 128).await?;
        #[cfg(feature = "frame-lifecycle")]
        {
            observation
                .wait_loaded(&mut driver, &runtime, ids[0], previous_document)
                .await?;
            let selected = observation
                .owner
                .snapshot(ids[0])?
                .ok_or("missing navigated lifecycle")?;
            assert_ne!(selected.document, previous_document);
            assert!(observation.owner.snapshot(ids[1])?.is_none());
            println!(
                "PASS production ordered lifecycle: new document, subtree load/idle, retired child; responses={}",
                observation.responses()
            );
        }
        // Assert: no Page fallback or sibling retirement.
        assert_eq!(runtime.is_object_live(ObjectId::new(4)), Ok(true));
        assert_eq!(runtime.is_object_live(ids[0]), Ok(true));
        assert_eq!(runtime.is_object_live(ids[1]), Ok(false));
        assert_eq!(runtime.is_object_live(ids[2]), Ok(true));
        let current_selected = navigation.snapshot(ids[0])?;
        println!(
            "selected route session before={} after={} retained={}",
            selected_route.session_id(),
            current_selected.session_id(),
            selected_route.is_current()
        );
        if cross_origin {
            assert_ne!(selected_route.session_id(), current_selected.session_id());
            assert!(!selected_route.is_current());
        } else {
            assert_eq!(selected_route.session_id(), current_selected.session_id());
            assert!(selected_route.is_current());
        }
        assert!(!nested_route.is_current());
        assert!(sibling_route.is_current());
        assert!(
            composition
                .topology()?
                .topology()
                .frame_object_id(nested)
                .is_none()
        );
        assert_eq!(
            composition.topology()?.topology().frame_object_id(child),
            Some(ids[0])
        );
        println!("PASS native child document retirement and parent/sibling runtime isolation");
        Ok::<(), Box<dyn Error + Send + Sync>>(())
    });
    let outcome = tokio::time::timeout(Duration::from_secs(20), &mut run).await;
    if outcome.is_err() {
        run.abort();
        assert!(run.await.is_err());
    }
    tokio::time::timeout(
        Duration::from_secs(10),
        browser.target_dispose_browser_context(&owner),
    )
    .await??;
    assert!(
        !browser
            .target_get_browser_contexts()
            .await?
            .contains(&owner)
    );
    println!("disposed document-topology Context {owner}");
    outcome???;
    Ok(())
}
