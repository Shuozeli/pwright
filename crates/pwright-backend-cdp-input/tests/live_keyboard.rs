//! Raw Keyboard event comparison; typed commands and Node FFI are separate gates.
use std::{error::Error, future::ready, io::Read, sync::Arc, time::Duration};

use pwright_backend_cdp_input::{
    CdpInputCoordinator, CdpInputPolicy, KeyInputError, KeyTokens, press_keys,
};
use pwright_cdp::{CdpConnection, CdpSession, connection::ConnectionConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;

type TestResult<T> = Result<T, Box<dyn Error>>;

#[derive(Serialize)]
struct EmptyParams {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    actions: Vec<Action>,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "lowercase",
    deny_unknown_fields
)]
enum Action {
    Press(String),
    Down(String),
    Up(String),
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
    outcomes: Vec<String>,
    snapshot: Value,
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
        return Err("verified loopback browser WebSocket required".into());
    }
    Ok(value)
}

async fn compare(
    page: &Arc<CdpSession>,
    fixture: &str,
    sample: &Case,
    expected: &Observation,
    is_mac: bool,
) -> TestResult<()> {
    page.page_enable().await?;
    page.runtime_enable().await?;
    page.page_navigate(fixture).await?;
    while page
        .runtime_evaluate("typeof window.resetTyping === 'function'")
        .await?["result"]["value"]
        != true
    {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    page.runtime_evaluate("window.resetTyping('#text')").await?;
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let mut outcomes = Vec::new();
    for action in &sample.actions {
        let mut transaction = input.begin_transaction(page.clone(), None).await?;
        let result = match action {
            Action::Press(value) => {
                press_keys(
                    &mut transaction,
                    &KeyTokens::parse_raw_keyboard(value)?,
                    is_mac,
                    || ready(Ok(())),
                )
                .await
            }
            Action::Down(value) => transaction.key_down_named(value, is_mac).await,
            Action::Up(value) => transaction.key_up_named(value, is_mac).await,
        };
        transaction.finish().await?;
        outcomes.push(match result {
            Ok(()) => "ok",
            Err(KeyInputError::UnknownKey(_)) => "unknown-key",
            Err(KeyInputError::Native(error)) => return Err(error.into()),
        });
    }
    let actual = page.runtime_evaluate("window.typingSnapshot()").await?;
    if expected.snapshot["overflow"] != false
        || actual["result"]["value"] != expected.snapshot
        || outcomes != expected.outcomes
    {
        return Err(format!(
            "{} differs: outcomes {outcomes:?} vs {:?}; snapshot {} vs {}",
            sample.id, expected.outcomes, actual["result"]["value"], expected.snapshot
        )
        .into());
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight, served fixture and fresh oracle"]
async fn raw_keyboard_matches_pinned_playwright_events_and_error_state() -> TestResult<()> {
    // Arrange: bounded local evidence, never use another task's Pages.
    let mut encoded = Vec::new();
    std::fs::File::open(std::env::var("PWRIGHT_INPUT_ORACLE")?)?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut encoded)?;
    if encoded.len() > 1024 * 1024 {
        return Err("oracle exceeds bound".into());
    }
    let oracle: Oracle = serde_json::from_slice(&encoded)?;
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../../../tests/data/raw-keyboard-cases.json"))?;
    if oracle.version != "1.58.2" || cases.len() != 14 || oracle.observations.len() != cases.len() {
        return Err("incomplete pinned oracle/case catalog".into());
    }
    let fixture = std::env::var("PWRIGHT_INPUT_FIXTURE")?;
    let url = url::Url::parse(&fixture)?;
    if url.scheme() != "http" || !url.host_str().is_some_and(|host| host.ends_with(".ts.net")) {
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
    let version: pwright_cdp::generated::browser::GetVersionReturns = serde_json::from_value(
        browser
            .send("Browser.getVersion", serde_json::to_value(EmptyParams {})?)
            .await?,
    )?;
    if version.product != format!("Chrome/{}", oracle.browser) {
        return Err("browser version differs".into());
    }
    let mut failures = Vec::new();
    for (sample, expected) in cases.iter().zip(&oracle.observations) {
        if sample.id != expected.id || sample.actions.len() > 8 {
            return Err("oracle order/capacity invalid".into());
        }
        let context = browser.target_create_browser_context(None, None).await?;
        let mut owned_target = None;
        // Act: each bounded test attempt always returns through exact Context cleanup.
        let result = tokio::time::timeout(Duration::from_secs(10), async {
            let target = browser
                .target_create_in_context("about:blank", &context)
                .await?;
            owned_target = Some(target.clone());
            let session = browser.target_attach(&target).await?;
            let page = Arc::new(CdpSession::new(connection.clone(), session, target));
            compare(&page, &fixture, sample, expected, oracle.is_mac).await
        })
        .await;
        browser.target_dispose_browser_context(&context).await?;
        // Assert: native absence, not just acknowledgement of a close command.
        if browser
            .target_get_targets()
            .await?
            .iter()
            .any(|target| Some(&target.target_id) == owned_target.as_ref())
            || browser
                .target_get_browser_contexts()
                .await?
                .contains(&context)
        {
            return Err("owned target/Context remained after cleanup".into());
        }
        match result {
            Ok(Ok(())) => println!(
                "raw keyboard {}: exact DOM events/value and error state match",
                sample.id
            ),
            Ok(Err(error)) => failures.push(error.to_string()),
            Err(error) => failures.push(format!("{}: {error}", sample.id)),
        }
    }
    if !failures.is_empty() {
        return Err(failures.join("\n").into());
    }
    Ok(())
}
