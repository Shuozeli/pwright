//! Candidate lifetime observations through the real Rust host and metadata
//! projection. No DOM-based reconstruction of Frame ownership is used.
use crate::HostSession;
use crate::evaluation::EvaluateParams;
use crate::frame_tree::{FrameNode, FrameTree};
use pwright_cdp::generated::target::{GetTargetInfoParams, GetTargetInfoReturns};
use pwright_cdp::{CdpConnection, CdpSession, connection::ConnectionConfig};
use serde::{Deserialize, Serialize};
use std::{error::Error, time::Duration};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Deserialize)]
struct Oracle {
    version: String,
    observations: Vec<Observation>,
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct Observation {
    mode: String,
    before: Before,
    removed: Removed,
    closed: Closed,
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Before {
    count: usize,
    parent: bool,
    owner: bool,
    same: bool,
    detached: bool,
    main_detached: bool,
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Removed {
    count: usize,
    parent: bool,
    children: usize,
    detached: bool,
    #[serde(rename = "retainedURL")]
    retained_url: bool,
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Closed {
    count: usize,
    children: usize,
    main_detached: bool,
    child_detached: bool,
    parent: bool,
    owner: bool,
}

async fn settled(host: &HostSession, tree: &FrameTree, count: usize) -> Result<Vec<FrameNode>> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            host.flush_topology().await?;
            host.frame_publications().fence()?.publish()?;
            let frames = tree.frames()?;
            if frames.len() == count {
                return Ok(frames);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| "Frame publication count deadline exceeded")?
}

async fn compare(
    host: &HostSession,
    browser: &CdpSession,
    fixture: &url::Url,
    oracle: &Observation,
) -> Result<()> {
    let context = host.new_context().await?;
    let mut native_owner = None;
    let mut native_child = None;
    let stage = std::cell::Cell::new("create Page");
    let outcome = tokio::time::timeout(Duration::from_secs(25), async {
        // Arrange
        let page = host.new_page(&context.id).await?;
        let mut url = fixture.clone();
        url.query_pairs_mut().append_pair("frame-mode", &oracle.mode)
            .append_pair("frame-case", &format!("node052-{}-{}", std::process::id(), oracle.mode));
        stage.set("navigate Page");
        host.page_goto(&page.id, url.to_string(), "load", None, 5000).await?;
        let target = browser.target_get_targets().await?.into_iter()
            .find(|target| target.target_type == "page" && target.url == url.as_str())
            .ok_or("owned native Page was not observed")?;
        let details: GetTargetInfoReturns = serde_json::from_value(browser.send("Target.getTargetInfo",
            serde_json::to_value(GetTargetInfoParams { target_id: Some(target.target_id.clone()) })?).await?)?;
        let native_context = details.target_info.browser_context_id.ok_or("owned Page has no native Context")?;
        native_owner = Some((target.target_id.clone(), native_context));
        stage.set("publish initial Frame tree");
        let frames = settled(host, &page.frame_tree, 2).await?;
        let main = frames[0].clone();
        let child = frames[1].clone();
        stage.set("read child Frame title");
        if host.frame_title(&child.id().get().to_string()).await? != "Pwright child Frame keyboard fixture" {
            return Err("retained child did not route to its native Frame".into());
        }
        let mut child_url = url.join("raw-drag-frame.html")?;
        child_url.set_query(url.query());
        native_child = browser.target_get_targets().await?.into_iter()
            .find(|target| target.target_type == "iframe" && target.url == child_url.as_str())
            .map(|target| target.target_id);
        let oopif = native_child.is_some();
        if oopif != (oracle.mode == "sandbox") { return Err("fixture renderer boundary differs from expected mode".into()); }
        let before = Before { count: frames.len(), parent: child.parent_id() == Some(main.id()),
            owner: host.pages(&context.id)?.iter().any(|candidate| candidate.id == page.id
                && candidate.frame_tree.frames().is_ok_and(|nodes| nodes.iter().any(|node| node.same_identity(&child)))),
            same: page.frame_tree.frames()?.iter().any(|node| node.same_identity(&child)),
            detached: child.is_detached(), main_detached: main.is_detached() };

        // Act: browser automation goes through the actual Rust command services.
        stage.set("remove iframe and publish detach");
        host.frame_evaluate(&page.main_frame_id, EvaluateParams::expression("document.querySelector('iframe').remove()")).await?;
        let removed_frames = settled(host, &page.frame_tree, 1).await?;
        let removed = Removed { count: removed_frames.len(), parent: child.parent_id() == Some(main.id()),
            children: removed_frames.iter().filter(|frame| frame.parent_id() == Some(main.id())).count(),
            detached: child.is_detached(), retained_url: child.url()?.contains("raw-drag-frame.html") };
        stage.set("insert replacement iframe");
        host.frame_evaluate(&page.main_frame_id, EvaluateParams::expression("(() => { const f = document.createElement('iframe'); f.srcdoc = '<title>new child</title>'; document.body.append(f); })()")).await?;
        let replaced = settled(host, &page.frame_tree, 2).await?;
        let replacement = replaced[1].clone();
        if replacement.same_identity(&child) { return Err("replacement reused detached Frame identity".into()); }
        stage.set("close Page");
        host.close_page(&page.id).await?;
        host.frame_publications().fence()?.publish()?;
        let closed_frames = page.frame_tree.frames()?;
        let closed = Closed { count: closed_frames.len(),
            children: closed_frames.iter().filter(|frame| frame.parent_id() == Some(main.id())).count(),
            main_detached: main.is_detached(), child_detached: replacement.is_detached(),
            parent: replacement.parent_id() == Some(main.id()),
            // Rust's Page descriptor owns this exact immutable collection;
            // JS Frame.page() wrapper identity is a later, separate FFI gate.
            owner: closed_frames.iter().any(|frame| frame.same_identity(&replacement)) };

        // Assert: compare stable publication points, not post-evaluate timing.
        let actual = Observation { mode: oracle.mode.clone(), before, removed, closed };
        if actual != *oracle { return Err(format!("Frame metadata differs: actual={actual:?}, oracle={oracle:?}").into()); }
        eprintln!("Rust Frame metadata {}", serde_json::to_string(&actual)?);
        Ok::<_, Box<dyn Error>>(())
    }).await;
    let cleanup = host.close_context(&context.id).await;
    if let Some((target, context)) = native_owner
        && (browser
            .target_get_browser_contexts()
            .await?
            .contains(&context)
            || browser.target_get_targets().await?.iter().any(|candidate| {
                candidate.target_id == target || native_child.as_ref() == Some(&candidate.target_id)
            }))
    {
        return Err("owned native Context/targets survived cleanup".into());
    }
    if let Ok(Err(error)) = &outcome {
        eprintln!("{} at {}: {error}", oracle.mode, stage.get());
    }
    cleanup.map_err(|error| {
        format!(
            "{} Context cleanup after {}: {error}",
            oracle.mode,
            stage.get()
        )
    })?;
    outcome
        .map_err(|_| format!("{} at {}: case timed out", oracle.mode, stage.get()))?
        .map_err(|error| format!("{} at {}: {error}", oracle.mode, stage.get()))?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight, served Frame fixture and pinned identity oracle"]
async fn real_host_frame_metadata_matches_oracle_lifetime() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(false)
        .try_init()
        .map_err(|error| format!("tracing initialization: {error}"))?;
    let endpoint = std::env::var("PWRIGHT_CDP_WS_URL")?;
    if !endpoint.starts_with("ws://127.0.0.1:9222/devtools/browser/") {
        return Err("dedicated loopback CDP required".into());
    }
    let fixture = url::Url::parse(&std::env::var("PWRIGHT_DRAG_FIXTURE")?)?;
    if fixture.scheme() != "http"
        || !fixture
            .host_str()
            .is_some_and(|host| host.ends_with(".ts.net"))
    {
        return Err("verified Tailscale fixture required".into());
    }
    let oracle: Oracle = serde_json::from_slice(&std::fs::read(std::env::var(
        "PWRIGHT_FRAME_IDENTITY_REPORT",
    )?)?)?;
    if oracle.version != "1.58.2"
        || oracle
            .observations
            .iter()
            .map(|case| case.mode.as_str())
            .collect::<Vec<_>>()
            != ["local", "sandbox"]
    {
        return Err("pinned local and sandbox oracle required".into());
    }
    let connection = CdpConnection::connect_with_config(
        &endpoint,
        ConnectionConfig {
            timeout: Duration::from_secs(5),
            ..ConnectionConfig::default()
        },
    )
    .await?;
    let browser = CdpSession::browser(connection.clone());
    let host = HostSession::connect_over_cdp(&endpoint, 15000).await?;
    let outcome = async {
        for case in &oracle.observations {
            compare(&host, &browser, &fixture, case).await?;
        }
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    let cleanup = host.close().await;
    host.frame_publications().fence()?.publish()?;
    let metrics = host.metrics()?;
    let trees = host.frame_tree_metrics()?;
    let urls = host.frame_metadata_metrics()?;
    let io_finished = connection.io_finished();
    drop(browser);
    drop(connection);
    io_finished.await;
    if let Err(error) = &outcome {
        eprintln!("Frame scenario error: {error}");
    }
    cleanup.map_err(|error| format!("Host shutdown: {error}"))?;
    outcome?;
    if metrics.active_leases != 0
        || metrics.active_operations != 0
        || trees.states != 1
        || trees.revisions != 0
        || urls.entries != 0
        || urls.pending_publications != 0
    {
        return Err(
            format!("retained owners: host={metrics:?}, trees={trees:?}, urls={urls:?}").into(),
        );
    }
    eprintln!(
        "Native Frame gate: both modes match; owned Context/targets absent; only fixed empty index remains: {trees:?}"
    );
    Ok(())
}
