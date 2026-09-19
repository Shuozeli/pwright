//! Raw Mouse primitive oracle gate; portable command/Node API are separate gates.
use pwright_backend_cdp_input::{CdpInputCoordinator, CdpInputPolicy, MouseClickOptions};
use pwright_cdp::{CdpConnection, CdpSession, MouseButton, connection::ConnectionConfig};
use pwright_input_model::{InvalidMouseMoveSteps, MouseClickCount, MouseMoveSteps};
use serde::Deserialize;
use serde_json::Value;
use std::{error::Error, io::Read, sync::Arc, time::Duration};
type TestResult<T> = Result<T, Box<dyn Error>>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    actions: Vec<Action>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum Action {
    #[serde(rename = "click")]
    Click(Click),
    #[serde(rename = "dblclick")]
    DoubleClick(Click),
    #[serde(rename = "move")]
    Move {
        x: f64,
        y: f64,
        steps: Option<f64>,
        error: Option<String>,
    },
    #[serde(rename = "down")]
    Down {
        button: Option<Button>,
        #[serde(rename = "clickCount")]
        count: Option<i32>,
    },
    #[serde(rename = "up")]
    Up {
        button: Option<Button>,
        #[serde(rename = "clickCount")]
        count: Option<i32>,
    },
    #[serde(rename = "wheel")]
    Wheel {
        #[serde(rename = "deltaX")]
        x: f64,
        #[serde(rename = "deltaY")]
        y: f64,
    },
    #[serde(rename = "keyDown")]
    KeyDown { key: String },
    #[serde(rename = "keyUp")]
    KeyUp { key: String },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Click {
    x: f64,
    y: f64,
    button: Option<Button>,
    #[serde(rename = "clickCount")]
    count: Option<f64>,
    delay: Option<f64>,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Button {
    Left,
    Right,
    Middle,
}
impl Button {
    fn native(self) -> MouseButton {
        match self {
            Self::Left => MouseButton::Left,
            Self::Right => MouseButton::Right,
            Self::Middle => MouseButton::Middle,
        }
    }
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
    tokio::time::timeout(Duration::from_secs(5), async {
        while page
            .runtime_evaluate("typeof window.resetMouse === 'function'")
            .await?["result"]["value"]
            != true
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok::<_, Box<dyn Error>>(())
    })
    .await??;
    page.runtime_evaluate("window.resetMouse()").await?;
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let mut wheels = 0;
    let mut outcomes = Vec::new();
    for action in &sample.actions {
        if let Action::Move { steps, error, .. } = action {
            match MouseMoveSteps::try_from(steps.unwrap_or(1.0)) {
                Err(InvalidMouseMoveSteps::Fractional)
                    if error.as_deref() == Some("invalid-steps") =>
                {
                    outcomes.push("invalid-steps".to_owned());
                    continue;
                }
                Err(error) => return Err(error.into()),
                Ok(_) if error.is_some() => {
                    return Err("expected invalid steps were accepted".into());
                }
                Ok(_) => {}
            }
        }
        let mut transaction = input.begin_transaction(page.clone(), None).await?;
        match action {
            Action::Click(click) | Action::DoubleClick(click) => {
                let count = if matches!(action, Action::DoubleClick(_)) {
                    MouseClickCount::TWO
                } else {
                    MouseClickCount::try_from(click.count.unwrap_or(1.0))?
                };
                let options = MouseClickOptions::new(click.x, click.y)?
                    .with_button(click.button.unwrap_or(Button::Left).native())
                    .with_count(count)
                    .with_delay(Duration::try_from_secs_f64(
                        click.delay.unwrap_or(0.0) / 1000.0,
                    )?);
                let mut execution =
                    transaction.raw_mouse_click(options, |duration| async move {
                        tokio::time::sleep(duration).await;
                        Ok(())
                    })?;
                // Observe cancellation without dropping the native owner. Each
                // retained CDP call has its own five-second response deadline.
                let result = tokio::time::timeout(Duration::from_secs(5), &mut execution).await;
                let retirement = execution.retire().await;
                retirement?;
                result??;
            }
            Action::Move { x, y, steps, .. } => {
                transaction
                    .raw_mouse_move(*x, *y, MouseMoveSteps::try_from(steps.unwrap_or(1.0))?)
                    .await?
            }
            Action::Down { button, count } => {
                transaction
                    .raw_mouse_down(button.unwrap_or(Button::Left).native(), count.unwrap_or(1))
                    .await?
            }
            Action::Up { button, count } => {
                transaction
                    .raw_mouse_up(button.unwrap_or(Button::Left).native(), count.unwrap_or(1))
                    .await?
            }
            Action::Wheel { x, y } => {
                transaction.raw_mouse_wheel(*x, *y).await?;
                wheels += 1;
            }
            Action::KeyDown { key } => transaction
                .key_down_named(key, is_mac)
                .await
                .map_err(|error| format!("{error:?}"))?,
            Action::KeyUp { key } => transaction
                .key_up_named(key, is_mac)
                .await
                .map_err(|error| format!("{error:?}"))?,
        }
        transaction.finish().await?;
        outcomes.push("ok".to_owned());
        if matches!(action, Action::Wheel { .. }) {
            tokio::time::timeout(Duration::from_secs(5), async {
                while page
                    .runtime_evaluate("window.mouseSnapshot().wheels")
                    .await?["result"]["value"]
                    != wheels
                {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Ok::<_, Box<dyn Error>>(())
            })
            .await??;
        }
    }
    let actual = page.runtime_evaluate("window.mouseSnapshot()").await?["result"]["value"].clone();
    if expected.snapshot["overflow"] != false
        || actual != expected.snapshot
        || outcomes != expected.outcomes
    {
        return Err(format!(
            "{} differs: expected {}, actual {actual}",
            sample.id, expected.snapshot
        )
        .into());
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight, serve-lib fixture and fresh pinned Mouse oracle"]
async fn raw_mouse_matches_pinned_playwright_events() -> TestResult<()> {
    // Arrange
    let mut encoded = Vec::new();
    std::fs::File::open(std::env::var("PWRIGHT_MOUSE_ORACLE")?)?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut encoded)?;
    if encoded.len() > 1024 * 1024 {
        return Err("mouse oracle exceeds bound".into());
    }
    let oracle: Oracle = serde_json::from_slice(&encoded)?;
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../../../tests/data/raw-mouse-cases.json"))?;
    if oracle.version != "1.58.2" || cases.len() != 25 || oracle.observations.len() != cases.len() {
        return Err("incomplete pinned Mouse oracle".into());
    }
    let fixture = std::env::var("PWRIGHT_MOUSE_FIXTURE")?;
    let url = url::Url::parse(&fixture)?;
    if url.scheme() != "http" || !url.host_str().is_some_and(|host| host.ends_with(".ts.net")) {
        return Err("verified Tailscale fixture required".into());
    }
    let endpoint = std::env::var("PWRIGHT_CDP_WS_URL")?;
    if !endpoint.starts_with("ws://127.0.0.1:9222/devtools/browser/") || endpoint.len() > 256 {
        return Err("verified loopback browser WebSocket required".into());
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
    let version = browser
        .send(
            "Browser.getVersion",
            serde_json::Value::Object(Default::default()),
        )
        .await?;
    if version["product"] != format!("Chrome/{}", oracle.browser) {
        return Err("oracle browser version differs".into());
    }
    let mut failures = Vec::new();
    for (sample, expected) in cases.iter().zip(&oracle.observations) {
        if sample.id != expected.id || sample.actions.len() > 12 {
            return Err("oracle order/action bound differs".into());
        }
        let context = browser.target_create_browser_context(None, None).await?;
        let mut owned_target = None;
        // Act: each observation owns its Page input state and exact Context.
        // Bound native commands and fixture waits individually: cancelling an
        // outer case future would discard the click's retained native owner.
        let result = async {
            let target = browser
                .target_create_in_context("about:blank", &context)
                .await?;
            owned_target = Some(target.clone());
            let session = browser.target_attach(&target).await?;
            let page = Arc::new(CdpSession::new(connection.clone(), session, target));
            compare(&page, &fixture, sample, expected, oracle.is_mac).await
        }
        .await;
        browser.target_dispose_browser_context(&context).await?;
        // Assert: command acknowledgement alone does not prove native absence.
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
            return Err("owned Mouse target/Context remains".into());
        }
        match result {
            Ok(()) => println!("raw mouse {}: exact DOM event sequence matches", sample.id),
            Err(error) => failures.push(format!("{}: {error}", sample.id)),
        }
    }
    if !failures.is_empty() {
        return Err(failures.join("\n").into());
    }
    Ok(())
}
