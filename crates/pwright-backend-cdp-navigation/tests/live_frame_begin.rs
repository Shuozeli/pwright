//! Native initiation only. Explicit diagnostic settlement is outside the API.
use pwright_backend_cdp_navigation::CdpNavigationService;
use pwright_cap_navigation::{BeginNavigationParams, GotoParams};
use pwright_cdp::{CdpClient, CdpConnection, CdpSession, generated::page::FrameTree};
use pwright_core::Service;
use serde_json::{Value, json};
use std::{error::Error, io::Read, sync::Arc, time::Duration};

type TestResult<T> = Result<T, Box<dyn Error>>;

fn child(tree: &FrameTree, depth: u64) -> TestResult<&FrameTree> {
    let mut selected = tree;
    for _ in 0..depth {
        selected = selected
            .child_frames
            .as_ref()
            .and_then(|frames| frames.first())
            .ok_or("missing selected child")?;
    }
    Ok(selected)
}

async fn unaffected(client: &dyn CdpClient) -> TestResult<Value> {
    let result = client.runtime_evaluate(
        "({root: document.URL, sibling: document.querySelectorAll('iframe')[1].contentDocument.URL})"
    ).await?;
    Ok(result["result"]["value"].clone())
}

async fn snapshot(client: &dyn CdpClient, depth: u64, url: &str) -> TestResult<Value> {
    let expression = if depth == 0 {
        "({url: document.URL, state: document.readyState, referrer: document.referrer})"
    } else if depth == 1 {
        "(() => { const document = window.document.querySelector('iframe').contentDocument; return document && {url: document.URL, state: document.readyState, referrer: document.referrer}; })()"
    } else {
        "(() => { const document = window.document.querySelector('iframe').contentDocument?.querySelector('iframe')?.contentDocument; return document && {url: document.URL, state: document.readyState, referrer: document.referrer}; })()"
    };
    let settled = async {
        loop {
            let value = client.runtime_evaluate(expression).await?["result"]["value"].clone();
            if value["url"] == url && value["state"] == "complete" {
                return Ok(value);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    tokio::time::timeout(Duration::from_secs(5), settled).await?
}

async fn remote_client(
    browser: &CdpSession,
    owner: &str,
    fixture: &str,
) -> TestResult<Arc<dyn CdpClient>> {
    use pwright_cdp::generated::target::{GetTargetsParams, GetTargetsReturns};
    let raw = browser
        .send(
            "Target.getTargets",
            serde_json::to_value(GetTargetsParams::default())?,
        )
        .await?;
    let targets: GetTargetsReturns = serde_json::from_value(raw)?;
    let expected_url = format!("{fixture}frame-lookup-parent.html");
    let mut owned = targets.target_infos.into_iter().filter(|info| {
        info.browser_context_id.as_deref() == Some(owner)
            && info.r#type == "iframe"
            && info.url == expected_url
    });
    let target = owned
        .next()
        .ok_or("sandbox fixture did not create an owned OOPIF")?;
    if owned.next().is_some() {
        return Err("ambiguous owned OOPIF".into());
    }
    let session = browser.target_attach(&target.target_id).await?;
    println!(
        "verified owned OOPIF {} session {session}",
        target.target_id
    );
    Ok(Arc::new(CdpSession::new(
        browser.connection().clone(),
        session,
        target.target_id,
    )))
}

async fn compare(
    client: Arc<dyn CdpClient>,
    browser: &CdpSession,
    owner: &str,
    oracle: &Value,
) -> TestResult<()> {
    let service = CdpNavigationService::new(client.clone());
    let fixture = oracle["fixture"].as_str().ok_or("missing fixture")?;
    if !fixture.starts_with("http://macbookpro-yuacx.tail8f3b66.ts.net:") {
        return Err("unexpected fixture host".into());
    }
    let cases = oracle["observations"]
        .as_array()
        .ok_or("missing observations")?;
    if cases.len() != 6 {
        return Err("expected six Frame observations".into());
    }
    let mut previous = ("", 0);
    let mut selected_client = client.clone();
    for spec in cases {
        let depth = spec["depth"].as_u64().ok_or("missing depth")?;
        let mode = spec["mode"].as_str().ok_or("missing mode")?;
        if !matches!((mode, depth), ("local", 1 | 2) | ("sandbox", 1)) {
            return Err("unexpected Frame mode/depth".into());
        }
        if (mode, depth) != previous {
            let initial_url = format!("{fixture}frame-navigation.html?mode={mode}");
            service.call(GotoParams::new(&initial_url)).await?;
            selected_client = if mode == "sandbox" {
                remote_client(browser, owner, fixture).await?
            } else {
                client.clone()
            };
        }
        previous = (mode, depth);
        let native_depth = if mode == "sandbox" { 0 } else { depth };
        let selected_service = CdpNavigationService::new(selected_client.clone());
        // Arrange
        let before = selected_client.page_get_frame_tree().await?;
        let unaffected_before = unaffected(client.as_ref()).await?;
        let selected_id = child(&before, native_depth)?.frame.id.clone();
        let url = spec["url"].as_str().ok_or("missing URL")?;
        let referer = spec["referer"].as_str().ok_or("missing referer")?;
        // Act: only the native initiation receipt is measured by this API.
        let result = selected_service
            .begin_frame_navigation(
                &selected_id,
                BeginNavigationParams::new(url).with_referrer(referer),
            )
            .await?;
        // Assert after a separate diagnostic barrier; no load-wait parity claim.
        let document = snapshot(selected_client.as_ref(), native_depth, url).await?;
        let after = selected_client.page_get_frame_tree().await?;
        let unaffected_after = unaffected(client.as_ref()).await?;
        let actual = json!({
            "rootUnchanged": unaffected_before["root"] == unaffected_after["root"],
            "siblingUnchanged": unaffected_before["sibling"] == unaffected_after["sibling"],
            "parentUnchanged": if depth == 1 { None } else { Some(child(&before, 1)?.frame.url == child(&after, 1)?.frame.url) },
            "selectedUrl": document["url"], "referrer": document["referrer"],
            "sameDocument": result.same_document,
            "sameIdentity": result.frame_id == selected_id && child(&after, native_depth)?.frame.id == selected_id,
        });
        if actual != spec["result"] {
            return Err(format!(
                "Frame native initiation discrepancy: {depth} {}; actual={actual}; expected={}",
                spec["phase"], spec["result"]
            )
            .into());
        }
        println!(
            "MATCH Frame mode={mode} depth={depth} phase={}",
            spec["phase"]
        );
    }
    // Invalid native identities must never silently target the Page.
    let root = client.page_get_frame_tree().await?.frame.url;
    let result = service
        .begin_frame_navigation(
            "pwright-missing-frame",
            BeginNavigationParams::new("about:blank"),
        )
        .await;
    if result.is_ok() || client.page_get_frame_tree().await?.frame.url != root {
        return Err("invalid Frame navigation succeeded or changed the Page".into());
    }
    println!("PASS invalid Frame target did not navigate the Page");
    Ok(())
}

#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight and fresh pinned Frame oracle"]
async fn native_frame_initiation_matches_pinned_final_state() -> TestResult<()> {
    let endpoint = std::env::var("PWRIGHT_CDP_WS_URL")?;
    assert!(endpoint.starts_with("ws://127.0.0.1:9222/devtools/browser/"));
    let file = std::fs::File::open(std::env::var("PWRIGHT_FRAME_NAVIGATION_ORACLE")?)?;
    let mut bytes = Vec::new();
    file.take(64 * 1024 + 1).read_to_end(&mut bytes)?;
    assert!(bytes.len() <= 64 * 1024);
    let oracle: Value = serde_json::from_slice(&bytes)?;
    assert_eq!(oracle["version"], "1.58.2");
    let connection = CdpConnection::connect(&endpoint).await?;
    let browser = CdpSession::browser(connection.clone());
    let owner = browser.target_create_browser_context(None, None).await?;
    println!("owned Frame initiation Context {owner}");
    let exercise = async {
        let target = browser
            .target_create_in_context("about:blank", &owner)
            .await?;
        let session = browser.target_attach(&target).await?;
        let client: Arc<dyn CdpClient> = Arc::new(CdpSession::new(connection, session, target));
        compare(client, &browser, &owner, &oracle).await
    };
    let outcome = tokio::time::timeout(Duration::from_secs(30), exercise).await;
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
    println!("disposed Frame initiation Context {owner}");
    outcome??;
    Ok(())
}
