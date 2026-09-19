//! Paired native primitive gate; no portable command or Node FFI is implied.
use std::{error::Error, io::Read, sync::Arc, time::Duration};

use pwright_backend_cdp_navigation::CdpNavigationService;
use pwright_cap_navigation::{GotoParams, WaitUntil};
use pwright_cdp::{CdpClient, CdpConnection, CdpSession};
use pwright_core::Service;
use serde_json::{Value, json};

type TestResult<T> = Result<T, Box<dyn Error>>;

async fn compare(page: Arc<dyn CdpClient>, observations: &[Value]) -> TestResult<()> {
    page.runtime_enable().await?;
    let service = CdpNavigationService::new(page.clone());
    for sample in observations {
        let url = sample["url"].as_str().ok_or("missing oracle URL")?;
        let wait = match sample["waitUntil"].as_str() {
            Some("commit") => WaitUntil::Commit,
            Some("domcontentloaded") => WaitUntil::DomContentLoaded,
            Some("load") => WaitUntil::Load,
            Some("networkidle") => WaitUntil::NetworkIdle,
            _ => return Err("unknown oracle lifecycle".into()),
        };
        // Arrange
        service.call(GotoParams::new(url)).await?;
        let old = page.page_get_frame_tree().await?.frame.loader_id;
        let before = page.runtime_evaluate("history.length").await?["result"]["value"].clone();
        if !before.is_number() {
            return Err("history length did not return a number".into());
        }
        // Act
        let result = tokio::time::timeout(Duration::from_secs(5), service.reload(wait)).await??;
        // Assert
        if result.loader_id.as_deref() == Some(old.as_str()) || result.same_document {
            return Err("reload did not replace the document".into());
        }
        let after = page.runtime_evaluate("history.length").await?["result"]["value"].clone();
        let actual = json!({
            "url": result.url, "historyUnchanged": before == after,
            "response": result.response.map(|response| json!({
                "url": response.url, "status": response.status, "statusText": response.status_text,
            })),
        });
        if actual != sample["result"] {
            return Err(format!(
                "reload sample {url} {wait:?}: expected {}, got {actual}",
                sample["result"]
            )
            .into());
        }
        println!("reload matched {url} {wait:?}");
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight and a fresh pinned reload oracle"]
async fn native_reload_matches_pinned_oracle() -> TestResult<()> {
    let endpoint = std::env::var("PWRIGHT_CDP_WS_URL")?;
    assert!(endpoint.starts_with("ws://127.0.0.1:9222/devtools/browser/"));
    let file = std::fs::File::open(std::env::var("PWRIGHT_RELOAD_ORACLE")?)?;
    let mut bytes = Vec::new();
    file.take(64 * 1024 + 1).read_to_end(&mut bytes)?;
    assert!(bytes.len() <= 64 * 1024);
    let oracle: Value = serde_json::from_slice(&bytes)?;
    assert_eq!(oracle["version"], "1.58.2");
    let observations = oracle["observations"]
        .as_array()
        .ok_or("missing observations")?;
    assert_eq!(observations.len(), 12);
    let connection = CdpConnection::connect(&endpoint).await?;
    let browser = CdpSession::browser(connection.clone());
    let context = browser.target_create_browser_context(None, None).await?;
    println!("owned reload Context {context}");
    let exercise = async {
        let target = browser
            .target_create_in_context("about:blank", &context)
            .await?;
        let session = browser.target_attach(&target).await?;
        let page: Arc<dyn CdpClient> =
            Arc::new(CdpSession::new(connection.clone(), session, target));
        compare(page, observations).await
    };
    let outcome = tokio::time::timeout(Duration::from_secs(90), exercise).await;
    tokio::time::timeout(
        Duration::from_secs(10),
        browser.target_dispose_browser_context(&context),
    )
    .await??;
    println!("disposed reload Context {context}");
    outcome??;
    Ok(())
}
