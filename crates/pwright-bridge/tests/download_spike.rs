use anyhow::Result;
use pwright_cdp::connection::CdpConnection;
use pwright_cdp::session::CdpSession;
use serde_json::Value;
use serde_json::json;
use tokio::time::Duration;

#[tokio::test]
#[ignore] // requires a running Chrome on localhost:9222
async fn test_download_spike() -> Result<()> {
    let output = std::process::Command::new("curl")
        .arg("-s")
        .arg("http://localhost:9222/json/version")
        .output()?;
    let res: Value = serde_json::from_slice(&output.stdout)?;
    let ws_url = res["webSocketDebuggerUrl"].as_str().unwrap().to_string();
    let conn = CdpConnection::connect(&ws_url).await?;
    let browser_session = CdpSession::browser(conn.clone());

    let targets = browser_session.target_get_targets().await?;
    let target = targets
        .into_iter()
        .find(|t| t.target_type == "page")
        .unwrap();
    let target_id = target.target_id;

    let session_id = browser_session.target_attach(&target_id).await?;
    let session = CdpSession::new(conn, session_id, target_id);

    let mut rx = session.subscribe_events();

    println!("Setting up download behavior...");
    session
        .send(
            "Browser.setDownloadBehavior",
            json!({
                "behavior": "allowAndName",
                "downloadPath": "/tmp",
                "eventsEnabled": true
            }),
        )
        .await?;

    println!("Navigating to a dummy download link...");
    let html = r#"<a id="dl" href="data:text/plain;base64,SGVsbG8sIFdvcmxkIQ==" download="hello.txt">Download</a>"#;

    session.send("Page.enable", json!({})).await?;
    session
        .send(
            "Page.navigate",
            json!({
                "url": format!("data:text/html,{}", html)
            }),
        )
        .await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    println!("Clicking download link...");
    session
        .send(
            "Runtime.evaluate",
            json!({
                "expression": "document.getElementById('dl').click()"
            }),
        )
        .await?;

    loop {
        if let Ok(event) = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
            let event = event.unwrap();
            if event.method.starts_with("Browser.download") {
                println!("Got event: {} {:?}", event.method, event.params);
                if event.method == "Browser.downloadProgress"
                    && event.params["state"] == "completed"
                {
                    println!("Download completed!");
                    break;
                }
            }
        } else {
            println!("Timeout waiting for download event");
            break;
        }
    }

    Ok(())
}
