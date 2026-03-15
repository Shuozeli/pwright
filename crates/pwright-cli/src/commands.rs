/// Command handlers — each maps a CLI subcommand to pwright-bridge calls.
use std::sync::Arc;

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use pwright_bridge::content::ScreenshotFormat;
use pwright_bridge::navigate::{NavigateOptions, WaitStrategy};
use pwright_bridge::snapshot::{self, SnapshotFilter};
use pwright_bridge::{Browser, BrowserConfig};

use crate::output;
use crate::state::CliState;

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
    output::info(&format!("📸 {} ({} bytes)", name, data.len()));
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
    let browser = connect(state).await?;
    let node_id = browser
        .resolve_ref(&state.active_tab, ref_str)
        .await
        .context(format!(
            "ref '{}' not found — run `pwright snapshot` first",
            ref_str
        ))?;

    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    pwright_bridge::actions::click_by_node_id(tab.session.as_ref(), node_id)
        .await
        .context("click failed")?;

    output::ok(&format!("Clicked [{}]", ref_str));
    Ok(())
}

/// `pwright fill <ref> <text>`
pub async fn fill(state: &mut CliState, ref_str: &str, text: &str) -> Result<()> {
    let browser = connect(state).await?;
    let node_id = browser
        .resolve_ref(&state.active_tab, ref_str)
        .await
        .context(format!("ref '{}' not found", ref_str))?;

    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    pwright_bridge::actions::fill_by_node_id(tab.session.as_ref(), node_id, text)
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

    for ch in text.chars() {
        pwright_bridge::actions::press_key(tab.session.as_ref(), &ch.to_string())
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
    let browser = connect(state).await?;
    let node_id = browser
        .resolve_ref(&state.active_tab, ref_str)
        .await
        .context(format!("ref '{}' not found", ref_str))?;

    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    pwright_bridge::actions::hover_by_node_id(tab.session.as_ref(), node_id)
        .await
        .context("hover failed")?;

    output::ok(&format!("Hovered [{}]", ref_str));
    Ok(())
}

/// `pwright select <ref> <val>`
pub async fn select(state: &mut CliState, ref_str: &str, value: &str) -> Result<()> {
    let browser = connect(state).await?;
    let node_id = browser
        .resolve_ref(&state.active_tab, ref_str)
        .await
        .context(format!("ref '{}' not found", ref_str))?;

    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    pwright_bridge::actions::select_by_node_id(tab.session.as_ref(), node_id, value)
        .await
        .context("select failed")?;

    output::ok(&format!("Selected [{}] value=\"{}\"", ref_str, value));
    Ok(())
}

/// `pwright download <ref> [--dest <path>]`
pub async fn download(state: &mut CliState, ref_str: &str, dest: Option<&str>) -> Result<()> {
    let browser = connect(state).await?;
    let node_id = browser
        .resolve_ref(&state.active_tab, ref_str)
        .await
        .context(format!("ref '{}' not found", ref_str))?;

    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    let page = pwright_bridge::playwright::Page::new(tab.session.clone());
    let session: &dyn pwright_bridge::CdpClient = tab.session.as_ref();

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

    // Use CDP Page.reload for a proper reload
    tab.session.page_reload().await.context("reload failed")?;

    // Wait for DOM ready
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if std::time::Instant::now() > deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        if let Ok(result) =
            pwright_bridge::evaluate::evaluate(tab.session.as_ref(), "document.readyState").await
        {
            let state_str = result.get("value").and_then(|v| v.as_str()).unwrap_or("");
            if state_str == "interactive" || state_str == "complete" {
                break;
            }
        }
    }

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

    let cookie = serde_json::json!({
        "name": name,
        "value": value,
        "domain": domain,
        "path": path,
    });

    pwright_bridge::cookies::set_cookies(tab.session.as_ref(), vec![cookie])
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
    let browser = connect(state).await?;
    let node_id = browser
        .resolve_ref(&state.active_tab, ref_str)
        .await
        .context(format!("ref '{}' not found", ref_str))?;

    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    tab.session
        .dom_focus(node_id)
        .await
        .context("focus failed")?;

    output::ok(&format!("Focused [{}]", ref_str));
    Ok(())
}

/// `pwright drag <ref> --dx N --dy N`
pub async fn drag(state: &mut CliState, ref_str: &str, dx: i32, dy: i32) -> Result<()> {
    let browser = connect(state).await?;
    let node_id = browser
        .resolve_ref(&state.active_tab, ref_str)
        .await
        .context(format!("ref '{}' not found", ref_str))?;

    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    pwright_bridge::actions::drag_by_node_id(tab.session.as_ref(), node_id, dx, dy)
        .await
        .context("drag failed")?;

    output::ok(&format!("Dragged [{}] by ({}, {})", ref_str, dx, dy));
    Ok(())
}

/// `pwright upload <ref> <files...>`
pub async fn upload(state: &mut CliState, ref_str: &str, files: &[String]) -> Result<()> {
    let browser = connect(state).await?;
    let node_id = browser
        .resolve_ref(&state.active_tab, ref_str)
        .await
        .context(format!("ref '{}' not found", ref_str))?;

    let tab = browser
        .resolve_tab(&state.active_tab)
        .await
        .context("no active tab")?;

    tab.session
        .dom_set_file_input_files(node_id, files)
        .await
        .context("upload failed")?;

    output::ok(&format!(
        "Uploaded {} file(s) to [{}]",
        files.len(),
        ref_str
    ));
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
    output::info(&format!("📄 {} ({} bytes)", name, data.len()));
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
    sink.write_summary(&script.name, &result);

    handle.close().await?;

    if result.status != "ok" {
        return Err(pwright_cdp::connection::CdpError::Other(
            result.error.unwrap_or_else(|| "script failed".into()),
        )
        .into());
    }

    Ok(())
}
