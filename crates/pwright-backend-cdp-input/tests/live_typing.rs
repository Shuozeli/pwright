//! Compares the input primitive with a separately recorded pinned oracle.
//! Does not establish Locator focus/retry, wire, Node FFI or UTF-16 parity.
use std::{error::Error, io::Read, sync::Arc, time::Duration};

use pwright_backend_cdp_input::{CdpInputCoordinator, CdpModifiers, type_text};
use pwright_cdp::{CdpConnection, CdpSession, connection::ConnectionConfig};
use pwright_input_model::InputText;
use serde::{Deserialize, Serialize};
use serde_json::Value;

type TestResult<T> = Result<T, Box<dyn Error>>;

#[derive(Serialize)]
struct EmptyParams {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Case {
    id: String,
    selector: String,
    text: String,
    modifier: Option<String>,
    delay_ms: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Oracle {
    version: String,
    browser: String,
    is_mac: bool,
    observations: Vec<Observation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    id: String,
    snapshot: Value,
}

fn modifier(value: Option<&str>) -> TestResult<Option<CdpModifiers>> {
    Ok(match value {
        None => None,
        Some("Shift") => Some(CdpModifiers::SHIFT),
        Some("Control") => Some(CdpModifiers::CONTROL),
        Some("Alt") => Some(CdpModifiers::ALT),
        Some("Meta") => Some(CdpModifiers::META),
        Some(_) => return Err("unsupported test modifier".into()),
    })
}

fn endpoint() -> TestResult<String> {
    let value = std::env::var("PWRIGHT_CDP_WS_URL")?;
    let url = url::Url::parse(&value)?;
    if value.len() > 256
        || url.scheme() != "ws"
        || url.host_str() != Some("127.0.0.1")
        || url.port() != Some(9222)
        || !url.path().starts_with("/devtools/browser/")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("dedicated loopback browser WebSocket required".into());
    }
    Ok(value)
}

async fn compare(page: &Arc<CdpSession>, fixture: &str, oracle: &Oracle) -> TestResult<()> {
    page.page_enable().await?;
    page.runtime_enable().await?;
    page.page_navigate(fixture).await?;
    // Fixture readiness, not a production action wait. One outer deadline owns retries.
    while page
        .runtime_evaluate("typeof window.resetTyping === 'function'")
        .await?["result"]["value"]
        != true
    {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let is_mac = page
        .runtime_evaluate("navigator.platform.includes('Mac')")
        .await?["result"]["value"]
        == true;
    if is_mac != oracle.is_mac {
        return Err("oracle and candidate platforms differ".into());
    }
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../tests/data/sequential-typing-cases.json"
    ))?;
    if cases.len() != 13 || oracle.observations.len() != cases.len() {
        return Err("incomplete sequential typing corpus".into());
    }
    let input = CdpInputCoordinator::new();
    let mut failures = Vec::new();
    for (sample, expected) in cases.iter().zip(&oracle.observations) {
        if sample.id != expected.id || expected.snapshot["overflow"] != false {
            return Err("oracle order or event bound invalid".into());
        }
        page.runtime_evaluate(&format!(
            "window.resetTyping({})",
            serde_json::to_string(&sample.selector)?
        ))
        .await?;
        let mut transaction = input
            .begin_transaction(page.clone(), modifier(sample.modifier.as_deref())?)
            .await?;
        let action = type_text(
            &mut transaction,
            InputText::try_from(sample.text.as_str())?,
            is_mac,
            || async {
                if sample.delay_ms != 0 {
                    tokio::time::sleep(Duration::from_millis(sample.delay_ms)).await;
                }
                Ok(())
            },
        )
        .await;
        // Cleanup even when typing fails; never retry the text after partial input.
        transaction.finish().await?;
        action?;
        let result = page.runtime_evaluate("window.typingSnapshot()").await?;
        let actual = &result["result"]["value"];
        if actual != &expected.snapshot {
            let actual_events = actual["events"]
                .as_array()
                .ok_or("candidate events missing")?;
            let expected_events = expected.snapshot["events"]
                .as_array()
                .ok_or("oracle events missing")?;
            let difference = actual_events
                .iter()
                .zip(expected_events)
                .position(|(a, b)| a != b)
                .unwrap_or(actual_events.len().min(expected_events.len()));
            failures.push(format!(
                "{}: value {:?} vs {:?}; event counts {} vs {}; first difference {}: {:?} vs {:?}",
                sample.id,
                actual["value"],
                expected.snapshot["value"],
                actual_events.len(),
                expected_events.len(),
                difference,
                actual_events.get(difference),
                expected_events.get(difference)
            ));
        } else {
            println!(
                "typing case {}: exact DOM events and value match",
                sample.id
            );
        }
    }
    if !failures.is_empty() {
        return Err(failures.join("\n").into());
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight, served fixture and fresh pinned oracle file"]
async fn sequential_typing_matches_pinned_playwright_dom_events() -> TestResult<()> {
    // Arrange: bounded local evidence and an owned context, never existing Pages.
    let path = std::env::var("PWRIGHT_INPUT_ORACLE")?;
    let mut encoded = Vec::new();
    std::fs::File::open(path)?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut encoded)?;
    if encoded.len() > 1024 * 1024 {
        return Err("oracle file exceeds test bound".into());
    }
    let oracle: Oracle = serde_json::from_slice(&encoded)?;
    if oracle.version != "1.58.2" || oracle.browser.is_empty() {
        return Err("pinned oracle metadata missing".into());
    }
    let fixture = std::env::var("PWRIGHT_INPUT_FIXTURE")?;
    let fixture_url = url::Url::parse(&fixture)?;
    if fixture_url.scheme() != "http"
        || !fixture_url
            .host_str()
            .is_some_and(|host| host.ends_with(".ts.net"))
    {
        return Err("verified Tailscale fixture required".into());
    }
    let connection = CdpConnection::connect_with_config(
        &endpoint()?,
        ConnectionConfig {
            timeout: Duration::from_secs(5),
            ..ConnectionConfig::default()
        },
    )
    .await?;
    let browser = CdpSession::browser(connection.clone());
    // navigator.userAgent reduces the patch version; use native product metadata.
    let version: pwright_cdp::generated::browser::GetVersionReturns = serde_json::from_value(
        browser
            .send("Browser.getVersion", serde_json::to_value(EmptyParams {})?)
            .await?,
    )?;
    if version.product != format!("Chrome/{}", oracle.browser) {
        return Err("oracle and candidate Chrome versions differ".into());
    }
    let context = browser.target_create_browser_context(None, None).await?;
    let mut owned_target = None;
    // Act: deadline is test-owned; context disposal below also runs on timeout/failure.
    let outcome = tokio::time::timeout(Duration::from_secs(60), async {
        let target = browser
            .target_create_in_context("about:blank", &context)
            .await?;
        owned_target = Some(target.clone());
        let session = browser.target_attach(&target).await?;
        let page = Arc::new(CdpSession::new(connection.clone(), session, target));
        compare(&page, &fixture, &oracle).await
    })
    .await;
    browser.target_dispose_browser_context(&context).await?;
    // Assert only after exact owned-context cleanup, leaving the external browser live.
    let targets = browser.target_get_targets().await?;
    assert!(
        !targets
            .iter()
            .any(|target| Some(&target.target_id) == owned_target.as_ref())
    );
    assert!(
        !browser
            .target_get_browser_contexts()
            .await?
            .contains(&context)
    );
    outcome??;
    Ok(())
}
