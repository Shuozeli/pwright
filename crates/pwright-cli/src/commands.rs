/// Command handlers — each maps a CLI subcommand to pwright-bridge calls.
use std::sync::Arc;

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use pwright_bridge::content::ScreenshotFormat;
use pwright_bridge::navigate::{NavigateOptions, WaitStrategy};
use pwright_bridge::snapshot::{self, SnapshotFilter};
use pwright_bridge::{Browser, BrowserConfig};
use pwright_cdp::{CdpClient, MouseButton, MouseEventType};

use crate::output;
use crate::state::CliState;

/// Resolve a ref to a (session, node_id) pair. Used by ref-based action commands.
async fn resolve_ref_to_session(
    state: &mut CliState,
    ref_str: &str,
) -> Result<(Arc<dyn pwright_bridge::CdpClient>, i64)> {
    let browser = connect(state).await?;
    let node_id = browser
        .resolve_ref(&state.active_tab, ref_str)
        .await
        .context(ref_not_found(ref_str))?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;
    Ok((tab.session.clone(), node_id))
}

fn ref_not_found(ref_str: &str) -> String {
    format!(
        "ref '{}' not found -- run `pwright snapshot` first",
        ref_str
    )
}

/// Connect to Chrome and re-attach to the active tab from state.
pub async fn connect(state: &mut CliState) -> Result<Arc<Browser>> {
    if state.ws_url.is_empty() {
        anyhow::bail!("No connection. Run `pwright open` first.");
    }
    let config = BrowserConfig {
        cdp_url: state.ws_url.clone(),
        ..Default::default()
    };
    let browser = Browser::connect(config)
        .await
        .context("failed to connect to Chrome")?;

    // Re-attach to the active tab if we have one
    if !state.active_tab.is_empty()
        && !state.target_id.is_empty()
        && let Err(e) = browser
            .reattach_tab(&state.target_id, &state.active_tab)
            .await
    {
        tracing::warn!("failed to reattach tab: {}, try `pwright open` again", e);
    }

    Ok(browser)
}

// ─── Core Commands ───

/// `pwright open [url]`
pub async fn open(state: &mut CliState, cdp_url: &str, url: Option<&str>) -> Result<()> {
    let ws_url = CliState::fetch_ws_url(cdp_url).await?;
    state.cdp_url = cdp_url.to_string();
    state.ws_url = ws_url;

    let browser = connect(state).await?;
    let nav_url = url.unwrap_or("about:blank");
    let tab = browser
        .create_tab(nav_url)
        .await
        .context("failed to create tab")?;

    state.active_tab = tab.tab_id.clone();
    state.target_id = tab.target_id.clone();
    state.save()?;

    output::info(&format!("Connected to Chrome ({})", cdp_url));
    output::info(&format!("Tab: {}", tab.tab_id));
    if let Ok(u) =
        pwright_bridge::evaluate::evaluate(tab.session.as_ref(), "window.location.href").await
    {
        let url_str = u.get("value").and_then(|v| v.as_str()).unwrap_or("unknown");
        output::info(&format!("URL: {}", url_str));
    }
    Ok(())
}

/// `pwright goto <url>`
pub async fn goto(state: &mut CliState, url: &str) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    let opts = NavigateOptions {
        wait_for: WaitStrategy::Dom,
        ..Default::default()
    };
    pwright_bridge::navigate::navigate(tab.session.as_ref(), &state.active_tab, url, &opts)
        .await
        .context("navigation failed")?;

    output::ok(&format!("Navigated to {}", url));
    Ok(())
}

/// `pwright close`
pub async fn close(state: &mut CliState) -> Result<()> {
    let browser = connect(state).await?;
    browser
        .close_tab(&state.active_tab)
        .await
        .context("failed to close tab")?;
    output::ok(&format!("Closed tab {}", state.active_tab));
    state.active_tab.clear();
    state.target_id.clear();
    state.save()?;
    Ok(())
}

// ─── Content Commands ───

/// `pwright snapshot`
pub async fn snapshot(state: &mut CliState) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    let (nodes, refs) = snapshot::get_snapshot(tab.session.as_ref(), &SnapshotFilter::All, -1)
        .await
        .context("failed to get snapshot")?;

    let ref_cache = pwright_bridge::RefCache {
        refs,
        nodes: nodes.clone(),
    };
    browser.set_ref_cache(&state.active_tab, ref_cache).await;
    output::print_snapshot(&nodes);
    Ok(())
}

/// `pwright screenshot [--filename]`
pub async fn screenshot(state: &mut CliState, filename: Option<&str>) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    let b64 = pwright_bridge::content::take_screenshot(
        tab.session.as_ref(),
        &ScreenshotFormat::Png,
        false,
    )
    .await
    .context("screenshot failed")?;

    let data = BASE64.decode(&b64).context("invalid base64")?;

    let name = filename.map(|s| s.to_string()).unwrap_or_else(|| {
        let ts = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S");
        format!("screenshot-{}.png", ts)
    });

    std::fs::write(&name, &data).context("failed to write screenshot")?;
    output::ok(&format!(
        "Screenshot saved: {} ({} bytes)",
        name,
        data.len()
    ));
    Ok(())
}

/// `pwright eval <expr>`
pub async fn eval(state: &mut CliState, expression: &str) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    let result = pwright_bridge::evaluate::evaluate(tab.session.as_ref(), expression)
        .await
        .context("evaluation failed")?;

    // Unwrap CDP result envelope: {"type":"string","value":"..."} → just the value
    if let Some(v) = result.get("value") {
        println!("{}", v);
    } else if result.get("type").and_then(|t| t.as_str()) == Some("undefined") {
        // CDP returns {"type":"undefined"} with no "value" key
        println!("undefined");
    } else {
        println!("{}", result);
    }
    Ok(())
}

// ─── Action Commands ───

/// `pwright click <ref>`
pub async fn click(state: &mut CliState, ref_str: &str) -> Result<()> {
    let (session, nid) = resolve_ref_to_session(state, ref_str).await?;
    pwright_bridge::actions::click_by_node_id(&*session, nid)
        .await
        .context("click failed")?;
    output::ok(&format!("Clicked [{}]", ref_str));
    Ok(())
}

/// `pwright click-at <x> <y> [--button] [--click-count]`
pub async fn click_at(
    state: &mut CliState,
    x: f64,
    y: f64,
    button: &str,
    click_count: i32,
) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    let session = tab.session.as_ref();
    let mouse_button = match button {
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        _ => MouseButton::Left,
    };
    session
        .input_dispatch_mouse_event(MouseEventType::Moved, x, y, None, None, None)
        .await
        .context("mouseMoved failed")?;
    session
        .input_dispatch_mouse_event(
            MouseEventType::Pressed,
            x,
            y,
            Some(mouse_button),
            Some(click_count),
            Some(1),
        )
        .await
        .context("mousePressed failed")?;
    session
        .input_dispatch_mouse_event(
            MouseEventType::Released,
            x,
            y,
            Some(mouse_button),
            Some(click_count),
            Some(0),
        )
        .await
        .context("mouseReleased failed")?;

    output::ok(&format!("Clicked at ({}, {})", x, y));
    Ok(())
}

/// `pwright dblclick <ref>`
pub async fn dblclick(state: &mut CliState, ref_str: &str) -> Result<()> {
    let (session, nid) = resolve_ref_to_session(state, ref_str).await?;
    pwright_bridge::actions::dblclick_by_node_id(&*session, nid)
        .await
        .context("dblclick failed")?;
    output::ok(&format!("Double-clicked [{}]", ref_str));
    Ok(())
}

/// `pwright hover-at <x> <y>`
pub async fn hover_at(state: &mut CliState, x: f64, y: f64) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    tab.session
        .as_ref()
        .input_dispatch_mouse_event(MouseEventType::Moved, x, y, None, None, None)
        .await
        .context("hover-at failed")?;

    output::ok(&format!("Hovered at ({}, {})", x, y));
    Ok(())
}

/// `pwright fill <ref> <text>`
pub async fn fill(state: &mut CliState, ref_str: &str, text: &str) -> Result<()> {
    let (session, nid) = resolve_ref_to_session(state, ref_str).await?;
    pwright_bridge::actions::fill_by_node_id(&*session, nid, text)
        .await
        .context("fill failed")?;
    output::ok(&format!("Filled [{}] with \"{}\"", ref_str, text));
    Ok(())
}

/// `pwright type <text>`
pub async fn type_text(state: &mut CliState, text: &str) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    // Use insertText (1 CDP call per char) instead of press_key (3 CDP calls per char)
    for ch in text.chars() {
        tab.session
            .input_insert_text(&ch.to_string())
            .await
            .context("type failed")?;
    }

    output::ok(&format!("Typed \"{}\"", text));
    Ok(())
}

/// `pwright press <key>`
pub async fn press(state: &mut CliState, key: &str) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    pwright_bridge::actions::press_key(tab.session.as_ref(), key)
        .await
        .context("press failed")?;

    output::ok(&format!("Pressed {}", key));
    Ok(())
}

/// `pwright hover <ref>`
pub async fn hover(state: &mut CliState, ref_str: &str) -> Result<()> {
    let (session, nid) = resolve_ref_to_session(state, ref_str).await?;
    pwright_bridge::actions::hover_by_node_id(&*session, nid)
        .await
        .context("hover failed")?;
    output::ok(&format!("Hovered [{}]", ref_str));
    Ok(())
}

/// `pwright select <ref> <val>`
pub async fn select(state: &mut CliState, ref_str: &str, value: &str) -> Result<()> {
    let (session, nid) = resolve_ref_to_session(state, ref_str).await?;
    pwright_bridge::actions::select_by_node_id(&*session, nid, value)
        .await
        .context("select failed")?;
    output::ok(&format!("Selected [{}] value=\"{}\"", ref_str, value));
    Ok(())
}

/// `pwright download <ref> [--dest <path>]`
pub async fn download(state: &mut CliState, ref_str: &str, dest: Option<&str>) -> Result<()> {
    let (session, node_id) = resolve_ref_to_session(state, ref_str).await?;
    let page = pwright_bridge::playwright::Page::new(session.clone());
    let session: &dyn CdpClient = &*session;

    let dl_path = page
        .expect_download(|| async {
            pwright_bridge::actions::click_by_node_id(session, node_id).await
        })
        .await
        .context("download failed")?;

    if let Some(dest_path) = dest {
        std::fs::copy(&dl_path, dest_path).context("failed to copy downloaded file")?;
        output::ok(&format!("Downloaded file saved to {}", dest_path));
    } else {
        output::ok(&format!("Downloaded file to {}", dl_path));
    }

    Ok(())
}

// ─── Navigation Commands ───

/// `pwright reload`
pub async fn reload(state: &mut CliState) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    tab.session.page_reload().await.context("reload failed")?;

    // Wait for DOM ready using the shared poll function
    pwright_bridge::navigate::poll_ready_state(
        tab.session.as_ref(),
        std::time::Duration::from_secs(10),
    )
    .await
    .context("reload wait timed out")?;

    output::ok("Reloaded");
    Ok(())
}

/// `pwright go-back`
pub async fn go_back(state: &mut CliState) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    pwright_bridge::evaluate::evaluate(tab.session.as_ref(), "history.back()")
        .await
        .context("go-back failed")?;

    output::ok("Went back");
    Ok(())
}

/// `pwright go-forward`
pub async fn go_forward(state: &mut CliState) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    pwright_bridge::evaluate::evaluate(tab.session.as_ref(), "history.forward()")
        .await
        .context("go-forward failed")?;

    output::ok("Went forward");
    Ok(())
}

// ─── Tab Commands ───

/// `pwright tab-list`
pub async fn tab_list(state: &mut CliState) -> Result<()> {
    let browser = connect(state).await?;
    let tabs = browser.list_tabs().await.context("failed to list tabs")?;

    let items: Vec<(String, String, String)> = tabs
        .iter()
        .map(|t| (t.target_id.clone(), t.title.clone(), t.url.clone()))
        .collect();

    output::print_tab_list(&items, &state.target_id);
    Ok(())
}

/// `pwright tab-new [url]`
pub async fn tab_new(state: &mut CliState, url: Option<&str>) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .create_tab(url.unwrap_or("about:blank"))
        .await
        .context("failed to create tab")?;

    output::info(&format!("New tab: {}", tab.tab_id));
    state.active_tab = tab.tab_id;
    state.target_id = tab.target_id;
    state.save()?;
    Ok(())
}

/// `pwright tab-close [id]`
pub async fn tab_close(state: &mut CliState, tab_id: Option<&str>) -> Result<()> {
    let id = tab_id.unwrap_or(&state.active_tab).to_string();
    let browser = connect(state).await?;
    browser
        .close_tab(&id)
        .await
        .context("failed to close tab")?;

    output::ok(&format!("Closed tab {}", id));
    if id == state.active_tab {
        state.active_tab.clear();
        state.target_id.clear();
        state.save()?;
    }
    Ok(())
}

/// `pwright tab-select <id>`
pub async fn tab_select(state: &mut CliState, tab_id: &str) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(tab_id)
        .await
        .context(format!("tab '{}' not found", tab_id))?;

    state.active_tab = tab.tab_id.clone();
    state.target_id = tab.target_id.clone();
    state.save()?;

    output::ok(&format!("Switched to tab {}", tab_id));
    Ok(())
}

// ─── Network Commands ───

/// `pwright network-listen [--duration N] [--filter X] [--type T]`
///
/// Attaches a SECOND CDP session to the active tab, enables Network domain,
/// and streams request/response events as JSONL to stdout until Ctrl+C or timeout.
pub async fn network_listen(
    state: &mut CliState,
    duration: Option<u64>,
    filter: Option<&str>,
    resource_type: Option<&str>,
) -> Result<()> {
    if state.ws_url.is_empty() || state.target_id.is_empty() {
        anyhow::bail!("No active tab. Run `pwright open` first.");
    }

    // Connect and create a SECOND session on the same target
    let config = BrowserConfig {
        cdp_url: state.ws_url.clone(),
        ..Default::default()
    };
    let browser = Browser::connect(config)
        .await
        .context("failed to connect to Chrome")?;

    let session_id = browser
        .browser_session()
        .target_attach(&state.target_id)
        .await
        .context("failed to attach listener session")?;

    let listener_session = Arc::new(pwright_cdp::CdpSession::new(
        browser.connection().clone(),
        session_id.clone(),
        state.target_id.clone(),
    ));

    // Enable Network domain on the listener session
    listener_session
        .network_enable()
        .await
        .context("failed to enable Network domain")?;

    let mut event_rx = listener_session.subscribe_events();

    output::info("Listening for network traffic (Ctrl+C to stop)...");

    let deadline =
        duration.map(|d| tokio::time::Instant::now() + std::time::Duration::from_secs(d));

    loop {
        let event = if let Some(dl) = deadline {
            match tokio::time::timeout_at(dl, event_rx.recv()).await {
                Ok(result) => result,
                Err(_) => break, // timeout
            }
        } else {
            event_rx.recv().await
        };

        match event {
            Ok(cdp_event) => {
                let method = cdp_event.method.as_str();
                let params = &cdp_event.params;

                if method == "Network.requestWillBeSent"
                    && let Some(req) =
                        pwright_bridge::playwright::network::parse_network_request(params)
                    && matches_filter(&req.url, filter)
                    && matches_type(&req.resource_type, resource_type)
                {
                    let json = serde_json::json!({
                        "event": "request",
                        "reqid": req.request_id,
                        "method": req.method,
                        "url": req.url,
                        "type": req.resource_type,
                        "post_data": req.post_data,
                    });
                    println!("{json}");
                } else if method == "Network.responseReceived"
                    && let Some(resp) =
                        pwright_bridge::playwright::network::parse_network_response(params)
                    && matches_filter(&resp.url, filter)
                {
                    let json = serde_json::json!({
                        "event": "response",
                        "reqid": resp.request_id,
                        "status": resp.status,
                        "mime": resp.mime_type,
                        "url": resp.url,
                    });
                    println!("{json}");
                } else if method == "Network.loadingFailed"
                    && let Some(reqid) = params.get("requestId").and_then(|v| v.as_str())
                {
                    let error_text = params
                        .get("errorText")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let json = serde_json::json!({
                        "event": "failed",
                        "reqid": reqid,
                        "error": error_text,
                    });
                    println!("{json}");
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                eprintln!("warning: listener lagged, missed {n} events");
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }

    // Cleanup: detach the listener session
    if let Err(e) = browser.browser_session().target_detach(&session_id).await {
        tracing::debug!("listener session detach failed: {e}");
    }

    output::info("Listener stopped.");
    Ok(())
}

fn matches_filter(url: &str, filter: Option<&str>) -> bool {
    match filter {
        Some(f) => url.contains(f),
        None => true,
    }
}

fn matches_type(actual: &str, filter: Option<&str>) -> bool {
    match filter {
        Some(f) => f.split(',').any(|t| t.eq_ignore_ascii_case(actual)),
        None => true,
    }
}

/// `pwright network-list [--filter X]`
///
/// Quick retroactive query using JS Performance API. No listener needed.
pub async fn network_list(state: &mut CliState, filter: Option<&str>) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    let js = r##"JSON.stringify(performance.getEntriesByType('resource').map(e => ({
        name: e.name,
        type: e.initiatorType,
        duration: Math.round(e.duration),
        size: e.transferSize || 0,
        status: e.responseStatus || 0
    })))"##;

    let result = pwright_bridge::evaluate::evaluate(tab.session.as_ref(), js)
        .await
        .context("failed to query performance entries")?;

    let json_str = result.get("value").and_then(|v| v.as_str()).unwrap_or("[]");

    let entries: Vec<serde_json::Value> = serde_json::from_str(json_str).unwrap_or_default();

    if entries.is_empty() {
        println!("  (no resources)");
        return Ok(());
    }

    let header = "    #  Type       Status     Size  Duration  URL";
    println!("{header}");
    for (i, e) in entries.iter().enumerate() {
        let url = e["name"].as_str().unwrap_or("");
        if let Some(f) = filter
            && !url.contains(f)
        {
            continue;
        }
        let entry_type = e["type"].as_str().unwrap_or("");
        let status = e["status"].as_i64().unwrap_or(0);
        let size = e["size"].as_i64().unwrap_or(0);
        let duration = e["duration"].as_i64().unwrap_or(0);
        let size_str = if size > 1024 {
            format!("{:.1}KB", size as f64 / 1024.0)
        } else {
            format!("{}B", size)
        };
        println!(
            "  {:>3}  {:<10} {:>6} {:>8} {:>6}ms  {}",
            i + 1,
            entry_type,
            status,
            size_str,
            duration,
            url
        );
    }
    Ok(())
}

/// `pwright network-get <reqid> [--output file]`
///
/// Fetch response body from Chrome by request ID. Chrome keeps bodies in
/// memory for the current page load.
pub async fn network_get(
    state: &mut CliState,
    reqid: &str,
    output_file: Option<&str>,
) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    // Network domain must be enabled to fetch response bodies
    tab.session
        .network_enable()
        .await
        .context("failed to enable Network domain")?;

    let body = tab
        .session
        .network_get_response_body(reqid)
        .await
        .context(format!("failed to get response body for {reqid}"))?;

    let content = if body.base64_encoded {
        let decoded = BASE64.decode(&body.body).context("invalid base64 body")?;
        if let Some(path) = output_file {
            std::fs::write(path, &decoded).context("failed to write file")?;
            output::ok(&format!("Saved {} bytes to {}", decoded.len(), path));
            return Ok(());
        }
        String::from_utf8_lossy(&decoded).to_string()
    } else {
        body.body
    };

    if let Some(path) = output_file {
        std::fs::write(path, &content).context("failed to write file")?;
        output::ok(&format!("Saved {} bytes to {}", content.len(), path));
    } else {
        println!("{}", content);
    }
    Ok(())
}

// ─── Cookie / PDF / Health ───

/// `pwright cookie-list`
pub async fn cookie_list(state: &mut CliState) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    let cookies = pwright_bridge::cookies::get_cookies(tab.session.as_ref())
        .await
        .context("failed to get cookies")?;

    if cookies.is_empty() {
        println!("  (no cookies)");
    } else {
        for c in &cookies {
            println!("  {} = {} ({})", c.name, c.value, c.domain);
        }
    }
    Ok(())
}

/// `pwright cookie-set --name N --value V --domain D [--path P]`
pub async fn cookie_set(
    state: &mut CliState,
    name: &str,
    value: &str,
    domain: &str,
    path: &str,
) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    let cookie = pwright_cdp::domains::network::Cookie {
        name: name.to_string(),
        value: value.to_string(),
        domain: domain.to_string(),
        path: path.to_string(),
        expires: 0.0,
        http_only: false,
        secure: false,
        same_site: String::new(),
    };

    pwright_bridge::cookies::set_cookies(tab.session.as_ref(), &[cookie])
        .await
        .context("failed to set cookie")?;

    output::ok(&format!(
        "Set cookie {} = {} ({}{})",
        name, value, domain, path
    ));
    Ok(())
}

/// `pwright focus <ref>`
pub async fn focus(state: &mut CliState, ref_str: &str) -> Result<()> {
    let (session, nid) = resolve_ref_to_session(state, ref_str).await?;
    session.dom_focus(nid).await.context("focus failed")?;
    output::ok(&format!("Focused [{}]", ref_str));
    Ok(())
}

/// `pwright drag <ref> --dx N --dy N`
pub async fn drag(state: &mut CliState, ref_str: &str, dx: i32, dy: i32) -> Result<()> {
    let (session, nid) = resolve_ref_to_session(state, ref_str).await?;
    pwright_bridge::actions::drag_by_node_id(&*session, nid, dx, dy)
        .await
        .context("drag failed")?;
    output::ok(&format!("Dragged [{}] by ({}, {})", ref_str, dx, dy));
    Ok(())
}

/// `pwright upload <ref> <files...>`
pub async fn upload(state: &mut CliState, ref_str: &str, files: &[String]) -> Result<()> {
    let (session, nid) = resolve_ref_to_session(state, ref_str).await?;
    session
        .dom_set_file_input_files(nid, files)
        .await
        .context("upload failed")?;
    output::ok(&format!(
        "Uploaded {} file(s) to [{}]",
        files.len(),
        ref_str
    ));
    Ok(())
}

/// Check if a checkbox is checked by node_id.
async fn is_checked(session: &dyn pwright_bridge::CdpClient, node_id: i64) -> Result<bool> {
    let resolved = session.dom_resolve_node(node_id).await?;
    let obj_id = resolved["object"]["objectId"]
        .as_str()
        .context("could not resolve node")?;
    let result = session
        .runtime_call_function_on(obj_id, pwright_js::element::IS_CHECKED, vec![])
        .await?;
    Ok(result
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}

/// `pwright check <ref>`
pub async fn check(state: &mut CliState, ref_str: &str) -> Result<()> {
    let (session, nid) = resolve_ref_to_session(state, ref_str).await?;
    if !is_checked(&*session, nid).await? {
        pwright_bridge::actions::click_by_node_id(&*session, nid)
            .await
            .context("click to check failed")?;
    }

    output::ok(&format!("Checked [{}]", ref_str));
    Ok(())
}

/// `pwright uncheck <ref>`
pub async fn uncheck(state: &mut CliState, ref_str: &str) -> Result<()> {
    let (session, nid) = resolve_ref_to_session(state, ref_str).await?;
    if is_checked(&*session, nid).await? {
        pwright_bridge::actions::click_by_node_id(&*session, nid)
            .await
            .context("click to uncheck failed")?;
    }

    output::ok(&format!("Unchecked [{}]", ref_str));
    Ok(())
}

/// `pwright scroll <ref>`
pub async fn scroll(state: &mut CliState, ref_str: &str) -> Result<()> {
    let (session, nid) = resolve_ref_to_session(state, ref_str).await?;
    pwright_bridge::actions::scroll_by_node_id(&*session, nid)
        .await
        .context("scroll failed")?;

    output::ok(&format!("Scrolled [{}] into view", ref_str));
    Ok(())
}

/// `pwright text`
pub async fn text(state: &mut CliState) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    let result = pwright_bridge::evaluate::evaluate(
        tab.session.as_ref(),
        "(document.body?.innerText || '')",
    )
    .await
    .context("text extraction failed")?;

    let text = result.get("value").and_then(|v| v.as_str()).unwrap_or("");
    println!("{text}");
    Ok(())
}

/// `pwright pdf [--filename]`
pub async fn pdf(state: &mut CliState, filename: Option<&str>) -> Result<()> {
    let browser = connect(state).await?;
    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    let b64 = pwright_bridge::content::get_pdf(tab.session.as_ref())
        .await
        .context("PDF generation failed")?;

    let data = BASE64.decode(&b64).context("invalid base64")?;

    let name = filename.map(|s| s.to_string()).unwrap_or_else(|| {
        let ts = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S");
        format!("page-{}.pdf", ts)
    });

    std::fs::write(&name, &data).context("failed to write PDF")?;
    output::ok(&format!("PDF saved: {} ({} bytes)", name, data.len()));
    Ok(())
}

/// `pwright health`
pub async fn health(state: &CliState) -> Result<()> {
    if state.cdp_url.is_empty() {
        output::info("Not connected. Run `pwright open` first.");
        return Ok(());
    }

    match CliState::fetch_ws_url(&state.cdp_url).await {
        Ok(ws) => {
            output::ok(&format!("Chrome reachable at {}", state.cdp_url));
            output::info(&format!("  WS: {}", ws));
        }
        Err(e) => {
            output::error(&format!("Chrome unreachable: {}", e));
        }
    }
    Ok(())
}

pub async fn script(cdp: &str, action: crate::ScriptAction) -> Result<()> {
    use pwright_script::{executor, output::JsonlSink, parser, validator};
    use std::collections::HashMap;

    let (script_path, raw_params, param_file, validate_only) = match &action {
        crate::ScriptAction::Run {
            script,
            params,
            param_file,
        } => (script, params, param_file, false),
        crate::ScriptAction::Validate {
            script,
            params,
            param_file,
        } => (script, params, param_file, true),
    };

    // Parse script
    let script =
        parser::parse_yaml_file(script_path).map_err(|e| anyhow::anyhow!("parse error: {e}"))?;

    // Merge params: param-file first, then --param flags (flags take precedence)
    let mut params: HashMap<String, String> = HashMap::new();
    if let Some(pf) = param_file {
        let file_params =
            parser::load_param_file(pf).map_err(|e| anyhow::anyhow!("param file error: {e}"))?;
        params.extend(file_params);
    }
    for (k, v) in raw_params {
        params.insert(k.clone(), v.clone());
    }

    // Validate
    if let Err(errors) = validator::validate(&script, &params) {
        for e in &errors {
            crate::output::error(e);
        }
        anyhow::bail!("script validation failed ({} error(s))", errors.len());
    }

    if validate_only {
        println!(
            "OK: {} steps, {} params",
            script.steps.len(),
            script.params.len()
        );
        return Ok(());
    }

    // Execute
    let browser = pwright_bridge::Browser::connect_http(cdp).await?;
    let handle = browser.new_tab("about:blank").await?;
    let page = handle.page();

    let mut stdout = std::io::stdout();
    let mut sink = JsonlSink::new(&mut stdout);
    let result = executor::execute(&script, &page, &params, &mut sink)
        .await
        .map_err(|e| pwright_cdp::connection::CdpError::Other(e.to_string()))?;
    sink.write_summary(&script.name, &result)
        .map_err(|e| pwright_cdp::connection::CdpError::Other(e.to_string()))?;

    handle.close().await?;

    if result.status != pwright_script::executor::ExecutionStatus::Ok {
        return Err(pwright_cdp::connection::CdpError::Other(
            result.error.unwrap_or_else(|| "script failed".into()),
        )
        .into());
    }

    Ok(())
}
