#![cfg(feature = "frame-lifecycle")]
//! Real selected Rust host, not a Node facade or public Frame.goto oracle.
use pwright_cdp::{CdpConnection, CdpSession, connection::ConnectionConfig};
use pwright_composition_chromium::frame_lifecycle::PageLifecycleHandle;
use pwright_core::ObjectId;
use pwright_node_host::{HostSession, PageDescriptor};
use std::{error::Error, time::Duration};
type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[cfg(feature = "frame-navigation")]
#[path = "live_frame_lifecycle/cancellation.rs"]
mod cancellation;
#[cfg(feature = "frame-navigation")]
#[path = "live_frame_lifecycle/navigation.rs"]
mod navigation;

async fn find_page(host: &HostSession, url: &str) -> Result<PageDescriptor> {
    loop {
        for context in host.contexts()? {
            for page in host.pages(&context.id)? {
                if page.main_frame_metadata.url()? == url {
                    return Ok(page);
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
async fn ready(handle: &PageLifecycleHandle, frame: ObjectId) -> Result<()> {
    loop {
        if handle
            .snapshot(frame)?
            .is_some_and(|s| s.load && s.network_idle)
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight and verified Tailscale fixture base"]
async fn selected_host_observes_multiple_pages_and_seals_escaped_owners() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_test_writer()
        .try_init()?;
    // Arrange: an exact externally-created test Context is always recoverable,
    // even if host attachment or an assertion fails. No unrelated target closes.
    let endpoint = std::env::var("PWRIGHT_CDP_WS_URL")?;
    let fixture = std::env::var("PWRIGHT_FRAME_DOCUMENT_FIXTURE")?;
    assert!(endpoint.starts_with("ws://127.0.0.1:9222/devtools/browser/"));
    assert!(
        fixture.starts_with("http://macbookpro-yuacx.tail8f3b66.ts.net:") && fixture.ends_with('/')
    );
    let connection =
        CdpConnection::connect_with_config(&endpoint, ConnectionConfig::default()).await?;
    let browser = CdpSession::browser(connection.clone());
    let context = browser.target_create_browser_context(None, None).await?;
    println!("owned host-lifecycle Context {context}");
    let run_context = context.clone();
    let mut run = tokio::spawn(async move {
        let browser = CdpSession::browser(connection.clone());
        let first_url = format!("{fixture}empty.html?node070=first");
        let second_url = format!("{fixture}empty.html?node070=second");
        browser
            .target_create_in_context(&first_url, &run_context)
            .await?;
        let second_target = browser
            .target_create_in_context(&second_url, &run_context)
            .await?;
        #[cfg(not(feature = "frame-navigation"))]
        drop(second_target);
        let host = HostSession::connect_over_cdp(&endpoint, 15000).await?;
        let outcome = async {
            let first = find_page(&host, &first_url).await?;
            let second = find_page(&host, &second_url).await?;
            let first_state = host.frame_lifecycle(&first.id).await?;
            let second_state = host.frame_lifecycle(&second.id).await?;
            let first_frame = ObjectId::new(first.main_frame_id.parse()?);
            let second_frame = ObjectId::new(second.main_frame_id.parse()?);
            ready(&first_state, first_frame).await?;
            ready(&second_state, second_frame).await?;
            let previous = first_state.snapshot(first_frame)?.ok_or("missing first document")?.document;
            let second_before = second_state.snapshot(second_frame)?;
            // Act: existing Rust Page.goto drives actual paused OOPIF discovery;
            // the selected observer consumes the same production host cursor.
            host.page_goto(&first.id, format!("{fixture}frame-navigation.html?mode=sandbox"), "load", None, 10000).await?;
            ready(&first_state, first_frame).await?;
            // Assert
            assert!(browser.target_get_targets().await?.iter().any(|target|
                target.target_type == "iframe" && target.url == format!("{fixture}frame-lookup-parent.html")));
            assert_ne!(first_state.snapshot(first_frame)?.unwrap().document, previous);
            assert_eq!(second_state.snapshot(second_frame)?, second_before);
            assert!(first_state.snapshot(second_frame)?.is_none());
            assert_eq!(host.metrics()?.topology_drivers, 1);
            #[cfg(feature = "frame-navigation")]
            navigation::exercise(&host, connection.clone(), &second_target, &second, &fixture).await?;
            #[cfg(feature = "frame-navigation")]
            let retained_navigation = navigation::typed(&host, &second, &fixture).await?;
            #[cfg(feature = "frame-navigation")]
            navigation::child(&host, &first, &fixture).await?;
            host.close_page(&first.id).await?;
            assert!(first_state.is_sealed());
            assert!(first_state.snapshot(first_frame).is_err());
            assert!(!second_state.is_sealed());
            let third_url = format!("{fixture}empty.html?node070=third");
            browser.target_create_in_context(&third_url, &run_context).await?;
            let third = find_page(&host, &third_url).await?;
            let third_state = host.frame_lifecycle(&third.id).await?;
            ready(&third_state, ObjectId::new(third.main_frame_id.parse()?)).await?;
            #[cfg(feature = "frame-navigation")]
            let mut pending = host.register_frame_navigation(&third.id, &third.main_frame_id, pwright_cap_navigation::WaitUntil::Load, None).await?;
            #[cfg(feature = "frame-navigation")]
            cancellation::exercise(&host, &third).await?;
            #[cfg(not(feature = "frame-navigation"))]
            host.close().await?;
            #[cfg(feature = "frame-navigation")]
            {
            assert!(tokio::time::timeout(Duration::from_secs(1), pending.changed()).await?.is_err());
            assert!(pending.try_take().is_err());
            drop(pending);
            }
            assert!(second_state.is_sealed());
            assert!(third_state.is_sealed());
            assert!(second_state.next_deadline().is_err());
            assert_eq!(host.metrics()?.topology_drivers, 0);
            assert_eq!(host.metrics()?.live_objects, 0);
            assert!(host.metrics()?.active_leases > 0);
            drop(first_state);
            drop(second_state);
            drop(third_state);
            #[cfg(feature = "frame-navigation")]
            {
                assert_eq!(retained_navigation.outcome().as_ref().map_err(|e| e.to_string())?.response.as_ref().unwrap().status, 404);
                assert!(host.metrics()?.active_leases > 0);
                drop(retained_navigation);
            }
            assert_eq!(host.metrics()?.active_leases, 0);
            println!("PASS selected host: multiple Pages, paused OOPIF lifecycle, Page retirement, dynamic admission, escaped shutdown");
            Ok::<(), Box<dyn Error + Send + Sync>>(())
        }.await;
        let cleanup = host.close().await;
        outcome?;
        cleanup?;
        Ok::<(), Box<dyn Error + Send + Sync>>(())
    });
    let outcome = tokio::time::timeout(Duration::from_secs(35), &mut run).await;
    if outcome.is_err() {
        run.abort();
        assert!(run.await.is_err());
    }
    tokio::time::timeout(
        Duration::from_secs(10),
        browser.target_dispose_browser_context(&context),
    )
    .await??;
    assert!(
        !browser
            .target_get_browser_contexts()
            .await?
            .contains(&context)
    );
    println!("disposed host-lifecycle Context {context}");
    outcome???;
    Ok(())
}
