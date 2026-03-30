//! Quick experiment: connect pwright to Lightpanda browser via CDP.

use pwright_bridge::browser::{Browser, BrowserConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Lightpanda + pwright experiment ===");

    // Connect via WebSocket directly (Lightpanda has no HTTP debug endpoints
    // except /json/version, so use ws:// for direct connection).
    println!("[1] Connecting to Lightpanda at ws://127.0.0.1:9333/ ...");
    let config = BrowserConfig {
        cdp_url: "ws://127.0.0.1:9333/".to_string(),
        ..BrowserConfig::default()
    };
    let browser = Browser::connect(config).await?;
    println!("[1] Connected!");

    // Step 2: Open a new tab
    println!("[2] Opening new tab...");
    match browser.new_tab("https://example.com").await {
        Ok(tab) => {
            println!("[2] Tab opened: target_id={}", tab.target_id());

            // Step 3: Get page and try basic operations
            let page = tab.page();

            println!("[3] Getting page title...");
            match page.title().await {
                Ok(title) => println!("[3] Title: {title:?}"),
                Err(e) => eprintln!("[3] title() failed: {e}"),
            }

            println!("[4] Getting page content...");
            match page.content().await {
                Ok(html) => {
                    let preview = if html.len() > 500 {
                        format!("{}...", &html[..500])
                    } else {
                        html.clone()
                    };
                    println!("[4] Content ({} bytes): {preview}", html.len());
                }
                Err(e) => eprintln!("[4] content() failed: {e}"),
            }

            // Step 5: Close tab
            println!("[5] Closing tab...");
            match tab.close().await {
                Ok(_) => println!("[5] Tab closed."),
                Err(e) => eprintln!("[5] close() failed: {e}"),
            }
        }
        Err(e) => {
            eprintln!("[2] new_tab failed: {e}");
        }
    }

    println!("=== Done ===");
    Ok(())
}
