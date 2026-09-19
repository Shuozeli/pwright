use super::*;
use pwright_backend_cdp_navigation::{
    CdpNavigationService,
    frame_lifecycle::navigation::{FrameNavigationWait, NavigationPoll},
};
use pwright_cap_navigation::{BeginNavigationParams, WaitUntil};
use pwright_composition_chromium::frame_lifecycle::NavigationWaitHandle;
use std::sync::Arc;

async fn outcome(
    ticket: &mut NavigationWaitHandle,
    page: &PageLifecycleHandle,
) -> Result<FrameNavigationWait> {
    loop {
        if let Some(result) = ticket.try_take()? {
            return Ok(result);
        }
        let next = ticket
            .next_deadline()?
            .ok_or("test navigation requires a deadline")?;
        let delay = next.saturating_sub(page.now()?);
        tokio::select! {
            changed = ticket.changed() => changed?,
            () = tokio::time::sleep(delay) => {},
        }
    }
}

pub(super) async fn exercise(
    host: &HostSession,
    connection: Arc<CdpConnection>,
    target: &str,
    page: &PageDescriptor,
    fixture: &str,
) -> Result<()> {
    let browser = CdpSession::browser(connection.clone());
    let session = browser.target_attach(target).await?;
    let client = Arc::new(CdpSession::new(
        connection,
        session.clone(),
        target.to_owned(),
    ));
    let run: Result<()> = async {
        let tree = client.page_get_frame_tree().await?;
        let service = CdpNavigationService::new(client.clone());
        let state = host.frame_lifecycle(&page.id).await?;
        let frame = ObjectId::new(page.main_frame_id.parse()?);
        let url = format!("{fixture}empty.html?node072=registered");
        // Arrange: actual host registration precedes independent native dispatch.
        let mut ticket = host.register_frame_navigation(&page.id, &page.main_frame_id, WaitUntil::Load, Some(Duration::from_secs(10))).await?;
        // Act: no test-side event normalization or response matching.
        let receipt = service.begin_frame_navigation(&tree.frame.id, BeginNavigationParams::new(url.clone())).await?;
        ticket.receipt(receipt.loader_id.as_deref())?;
        let result = outcome(&mut ticket, &state).await?;
        // Assert: response comes from the production host's ordered CDP cursor.
        let NavigationPoll::Complete { frame: actual, response_json: Some(bytes) } = result.outcome().ok_or("unsettled navigation")?? else { return Err("missing native HTTP response".into()); };
        let response: serde_json::Value = serde_json::from_slice(bytes)?;
        assert_eq!(actual, frame);
        assert_eq!(response["status"], 200);
        assert_eq!(response["url"], url);
        drop(result);
        drop(ticket);

        // A hash receipt waits for persistent subtree idle and returns null.
        let document = state.snapshot(frame)?.ok_or("missing document")?.document;
        let mut hash = host.register_frame_navigation(&page.id, &page.main_frame_id, WaitUntil::NetworkIdle, Some(Duration::from_secs(10))).await?;
        let receipt = service.begin_frame_navigation(&tree.frame.id, BeginNavigationParams::new(format!("{url}#same"))).await?;
        assert!(receipt.same_document);
        hash.receipt(None)?;
        let result = outcome(&mut hash, &state).await?;
        assert_eq!(result.outcome().ok_or("unsettled hash")??, NavigationPoll::Complete { frame, response_json: None });
        assert_eq!(state.snapshot(frame)?.unwrap().document, document);
        assert!(state.snapshot(frame)?.unwrap().network_idle);
        println!("PASS registered actual host: native HTTP response, hash idle, original Frame, no test event matcher");
        Ok(())
    }.await;
    let cleanup = browser.target_detach(&session).await;
    run?;
    cleanup?;
    // No auxiliary dispatch or test event parser on the host-owned path.
    let url = format!("{fixture}empty.html?node073=host-owned");
    let result = host
        .navigate_frame_native(
            &page.id,
            &page.main_frame_id,
            BeginNavigationParams::new(url.clone()),
            WaitUntil::Load,
            10000,
        )
        .await?;
    assert_response(&result, &page.main_frame_id, &url, 200)?;
    drop(result);
    let result = host
        .navigate_frame_native(
            &page.id,
            &page.main_frame_id,
            BeginNavigationParams::new(format!("{url}#host-hash")),
            WaitUntil::NetworkIdle,
            10000,
        )
        .await?;
    assert_eq!(
        result.outcome().ok_or("unsettled host hash")??,
        NavigationPoll::Complete {
            frame: ObjectId::new(page.main_frame_id.parse()?),
            response_json: None
        }
    );
    drop(result);
    let url = format!("{fixture}node073-intentionally-missing.html");
    let result = host
        .navigate_frame_native(
            &page.id,
            &page.main_frame_id,
            BeginNavigationParams::new(url.clone()),
            WaitUntil::Load,
            10000,
        )
        .await?;
    assert_response(&result, &page.main_frame_id, &url, 404)?;
    assert_eq!(host.metrics()?.active_operations, 0);
    println!("PASS host-owned navigation: HTTP 200, hash idle/null, HTTP 404, released operation");
    Ok(())
}

fn assert_response(
    result: &FrameNavigationWait,
    frame: &str,
    url: &str,
    status: i64,
) -> Result<()> {
    let NavigationPoll::Complete {
        frame: actual,
        response_json: Some(bytes),
    } = result.outcome().ok_or("unsettled host navigation")??
    else {
        return Err("missing host HTTP response".into());
    };
    assert_eq!(actual, ObjectId::new(frame.parse()?));
    let response: serde_json::Value = serde_json::from_slice(bytes)?;
    assert_eq!(response["url"], url);
    assert_eq!(response["status"], status);
    Ok(())
}

pub(super) async fn typed(
    host: &HostSession,
    page: &PageDescriptor,
    fixture: &str,
) -> Result<pwright_node_host::FrameGotoOutput> {
    use pwright_cap_navigation::v1_2::{GotoParams, WaitUntil};
    let url = format!("{fixture}empty.html?node075=typed");
    let result = host
        .frame_goto(
            &page.id,
            &page.main_frame_id,
            GotoParams::new(url.clone()),
            10000,
        )
        .await?;
    let response = result.outcome().as_ref().map_err(|e| e.to_string())?;
    assert_eq!(response.frame_id, page.main_frame_id.parse::<u64>()?);
    assert_eq!(
        response
            .response
            .as_ref()
            .ok_or("missing typed response")?
            .status,
        200
    );
    assert_eq!(response.response.as_ref().unwrap().url, url);
    assert!(!response.response.as_ref().unwrap().headers.is_empty());
    drop(result);
    let result = host
        .frame_goto(
            &page.id,
            &page.main_frame_id,
            GotoParams::new(format!("{url}#same")).with_wait_until(WaitUntil::NetworkIdle),
            10000,
        )
        .await?;
    assert!(
        result
            .outcome()
            .as_ref()
            .map_err(|e| e.to_string())?
            .response
            .is_none()
    );
    drop(result);
    let url = format!("{fixture}node075-intentionally-missing.html");
    let result = host
        .frame_goto(
            &page.id,
            &page.main_frame_id,
            GotoParams::new(url.clone()),
            10000,
        )
        .await?;
    let response = result
        .outcome()
        .as_ref()
        .map_err(|e| e.to_string())?
        .response
        .as_ref()
        .ok_or("missing typed 404")?;
    assert_eq!(response.status, 404);
    assert_eq!(response.url, url);
    assert_eq!(host.metrics()?.active_operations, 0);
    println!(
        "PASS typed Rust host: HTTP 200/headers, hash idle/null, HTTP 404, result retained independently"
    );
    Ok(result)
}

pub(super) async fn child(host: &HostSession, page: &PageDescriptor, fixture: &str) -> Result<()> {
    host.frame_publications().fence()?.publish()?;
    let frames = page.frame_tree.frames()?;
    let mut child = None;
    let mut sibling = None;
    for frame in &frames {
        if frame.with_name(|name| name == "navigation-target")? {
            child = Some(frame);
        }
        if frame.with_name(|name| name == "unaffected-sibling")? {
            sibling = Some(frame);
        }
    }
    let child = child.ok_or("missing OOPIF child")?;
    let sibling = sibling.ok_or("missing unaffected sibling")?;
    let sibling_url = sibling.url()?;
    let parent_url = page.main_frame_metadata.url()?;
    let frame = child.id().get().to_string();
    let url = format!("{fixture}empty.html?node073=child");
    // Arrange/Act: selected OOPIF navigation, not a Page.goto fallback.
    let result = host
        .frame_goto(
            &page.id,
            &frame,
            pwright_cap_navigation::v1_2::GotoParams::new(url.clone()),
            10000,
        )
        .await?;
    // Assert
    let response = result.outcome().as_ref().map_err(|e| e.to_string())?;
    assert_eq!(response.frame_id, frame.parse::<u64>()?);
    assert_eq!(
        response
            .response
            .as_ref()
            .ok_or("missing typed child response")?
            .status,
        200
    );
    assert_eq!(response.response.as_ref().unwrap().url, url);
    host.frame_publications().fence()?.publish()?;
    assert_eq!(child.url()?, url);
    assert!(!child.is_detached());
    assert_eq!(sibling.url()?, sibling_url);
    assert_eq!(page.main_frame_metadata.url()?, parent_url);
    println!(
        "PASS typed host OOPIF child: selected Frame response, published URL, unchanged parent/sibling"
    );
    Ok(())
}
