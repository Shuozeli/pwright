#![cfg(feature = "drag-runtime")]
//! Actual HostSession Mouse dispatch and shutdown; CDP only observes/owns fixtures.
use pwright_cdp::{CdpConnection, CdpSession, connection::ConnectionConfig};
use pwright_node_host::{
    HostSession,
    locator::action::{LocatorAction, PressKey, PressOptions},
    locator_query::{QueryBudget, QueryLimits},
    mouse::{DownParams, MouseButton, MouseMoveSteps, MouseNumber, MoveParams, UpParams},
};
use serde::Deserialize;
use serde_json::Value;
use std::io::Read;
use std::{error::Error, sync::Arc, time::Duration};
type TestResult<T> = Result<T, Box<dyn Error>>;

async fn move_to(host: &HostSession, page: &str, x: f64) -> TestResult<()> {
    host.mouse_move(
        page,
        MoveParams {
            x: MouseNumber::try_from(x)?,
            y: MouseNumber::try_from(40.0)?,
            steps: MouseMoveSteps::try_from(1.0)?,
        },
    )
    .await?;
    Ok(())
}
async fn case(
    endpoint: &str,
    fixture: &url::Url,
    close_drag: bool,
    keyboard: Option<(&KeyboardCase, &Value, &str)>,
) -> TestResult<()> {
    // Arrange: unique test-owned Context, left alive across host disconnect.
    let connection = CdpConnection::connect_with_config(
        endpoint,
        ConnectionConfig {
            timeout: Duration::from_secs(5),
            ..ConnectionConfig::default()
        },
    )
    .await?;
    let browser = CdpSession::browser(connection.clone());
    let context = browser.target_create_browser_context(None, None).await?;
    let mut host = None;
    let mut owned_target = None;
    let outcome = tokio::time::timeout(Duration::from_secs(20), async {
        let mut url = fixture.clone();
        url.query_pairs_mut()
            .append_pair("host-drag-case", &context);
        let target = browser
            .target_create_in_context(url.as_str(), &context)
            .await?;
        owned_target = Some(target.clone());
        let session = browser.target_attach(&target).await?;
        let observer = CdpSession::new(connection.clone(), session, target.clone());
        observer.runtime_enable().await?;
        while observer
            .runtime_evaluate(
                "document.readyState !== 'loading' && typeof dragFixture === 'object'",
            )
            .await?["result"]["value"]
            != true
        {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        host = Some(Arc::new(
            HostSession::connect_over_cdp("http://127.0.0.1:9222", 5000).await?,
        ));
        let host = host.as_ref().ok_or("host missing")?;
        if let Some((_, _, version)) = keyboard
            && host.version() != version
        {
            return Err("host drag/Keyboard browser differs from oracle".into());
        }
        let mut selected = Vec::new();
        for context in host.contexts()? {
            for page in host.pages(&context.id)? {
                if page.main_frame_metadata.url()? == url.as_str() {
                    selected.push(page);
                }
            }
        }
        if selected.len() != 1 {
            return Err("exact borrowed Page route not found".into());
        }
        let page = selected.pop().ok_or("missing Page")?;
        move_to(host, &page.id, 40.0).await?;
        if keyboard.is_none_or(|(sample, _, _)| sample.drag) {
            host.mouse_down(
                &page.id,
                DownParams {
                    button: MouseButton::Left,
                    click_count: 1,
                },
            )
            .await?;
            // Act: ordinary move must use the selected source/dispatcher/controller.
            move_to(host, &page.id, 70.0).await?;
        }
        if let Some((sample, expected, _)) = keyboard {
            if sample.drag
                && observer
                    .runtime_evaluate("dragFixture.events.includes('dragstart')")
                    .await?["result"]["value"]
                    != true
            {
                return Err("drag/Keyboard setup did not start a native drag".into());
            }
            for key in &sample.keys {
                match key {
                    KeyAction::Down { key } => host.keyboard_down(&page.id, key).await?,
                    KeyAction::Up { key } => host.keyboard_up(&page.id, key).await?,
                    KeyAction::Press { key } => {
                        host.keyboard_press(&page.id, key, Duration::ZERO).await?
                    }
                    KeyAction::Locatorpress { key, timeout } => {
                        let budget = QueryBudget::new(QueryLimits {
                            recipes: 2,
                            bytes: 64 * 1024,
                        })?;
                        let query = budget.prepare_leaf(9)?.css("#keyboard".into())?;
                        let locator = host.locator(&page.main_frame_id, query)?;
                        let result = locator
                            .act(
                                LocatorAction::Press {
                                    key: PressKey::new(key)?,
                                    options: PressOptions::new()
                                        .with_no_wait_after(true)
                                        .with_delay(if *timeout {
                                            Duration::from_secs(60)
                                        } else {
                                            Duration::ZERO
                                        }),
                                },
                                if *timeout { 2000 } else { 5000 },
                            )
                            .await;
                        if *timeout {
                            if !matches!(result, Err(pwright_node_host::HostError::Timeout(2000))) {
                                return Err(
                                    "Locator press did not return the requested timeout".into()
                                );
                            }
                        } else {
                            result?;
                        }
                    }
                    KeyAction::Locatorsequential { text } => {
                        let budget = QueryBudget::new(QueryLimits {
                            recipes: 2,
                            bytes: 64 * 1024,
                        })?;
                        let query = budget.prepare_leaf(9)?.css("#keyboard".into())?;
                        let locator = host.locator(&page.main_frame_id, query)?;
                        locator
                            .act(
                                LocatorAction::PressSequentially {
                                    text: text.clone(),
                                    delay: None,
                                },
                                5000,
                            )
                            .await?;
                    }
                }
            }
            move_to(host, &page.id, 220.0).await?;
            host.mouse_up(
                &page.id,
                UpParams {
                    button: MouseButton::Left,
                    click_count: 1,
                },
            )
            .await?;
            // Compare before closing: close-cancel must never mask a missing
            // Escape cancellation or consume a keyup that should preserve drag.
            let actual = observer.runtime_evaluate("dragFixture").await?["result"]["value"].clone();
            if &actual != expected {
                return Err(format!(
                    "{} drag/Keyboard mismatch: expected {expected}, actual {actual}",
                    sample.id
                )
                .into());
            }
            host.close().await?;
        } else if close_drag {
            host.close().await?;
        } else {
            move_to(host, &page.id, 220.0).await?;
            host.mouse_up(
                &page.id,
                UpParams {
                    button: MouseButton::Left,
                    click_count: 1,
                },
            )
            .await?;
            host.close().await?;
        }
        // Assert: native drag state settled before host close returned, while
        // the externally owned target remains available to this observer.
        let value = observer.runtime_evaluate("dragFixture").await?;
        let state = &value["result"]["value"];
        if state["overflow"] != false
            || (keyboard.is_none_or(|(sample, _, _)| sample.drag)
                && !state["events"]
                    .as_array()
                    .is_some_and(|events| events.iter().any(|event| event == "dragend")))
        {
            return Err("host drag did not finish natively".into());
        }
        if keyboard.is_none() && close_drag {
            if !state["dropped"].is_null() {
                return Err("host close unexpectedly delivered a drop".into());
            }
        } else if keyboard.is_none() && state["dropped"] != "pwright-drag-observer" {
            return Err("host Mouse drop did not deliver exact drag data".into());
        }
        if !browser
            .target_get_targets()
            .await?
            .iter()
            .any(|candidate| candidate.target_id == target)
        {
            return Err("host close destroyed the borrowed Page".into());
        }
        let metrics = host.metrics()?;
        if metrics.connected || metrics.active_leases != 0 || metrics.active_operations != 0 {
            return Err(format!("host drag shutdown retained resources: {metrics:?}").into());
        }
        Ok::<_, Box<dyn Error>>(())
    })
    .await;
    // Always finish host teardown and independently dispose only the test Context.
    let closed = match host.as_ref() {
        Some(host) => host.close().await,
        None => Ok(()),
    };
    browser.target_dispose_browser_context(&context).await?;
    if browser
        .target_get_browser_contexts()
        .await?
        .iter()
        .any(|id| id == &context)
        || browser
            .target_get_targets()
            .await?
            .iter()
            .any(|target| Some(&target.target_id) == owned_target.as_ref())
    {
        return Err("host drag fixture target/Context survived cleanup".into());
    }
    closed?;
    outcome??;
    eprintln!(
        "Host drag {}: native outcome, borrowed Page survival and zero host leases verified",
        keyboard.map_or(
            if close_drag { "close-cancel" } else { "drop" },
            |(sample, _, _)| sample.id.as_str()
        )
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and verified PWRIGHT_DRAG_FIXTURE/PWRIGHT_CDP_WS_URL"]
async fn host_mouse_drag_drop_and_close_cancel_preserve_borrowed_page() -> TestResult<()> {
    let (endpoint, fixture) = parameters()?;
    case(&endpoint, &fixture, false, None).await?;
    case(&endpoint, &fixture, true, None).await
}

fn parameters() -> TestResult<(String, url::Url)> {
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
    Ok((endpoint, fixture))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyboardCase {
    id: String,
    drag: bool,
    keys: Vec<KeyAction>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum KeyAction {
    Down {
        key: String,
    },
    Up {
        key: String,
    },
    Press {
        key: String,
    },
    Locatorpress {
        key: String,
        #[serde(default)]
        timeout: bool,
    },
    Locatorsequential {
        text: String,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Oracle {
    version: String,
    browser: String,
    observations: Vec<Observation>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    id: String,
    snapshot: Value,
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight, drag fixture and fresh pinned drag/Keyboard oracle"]
async fn host_drag_keyboard_matches_pinned_playwright_before_close() -> TestResult<()> {
    // Arrange
    let (endpoint, fixture) = parameters()?;
    let mut bytes = Vec::new();
    std::fs::File::open(std::env::var("PWRIGHT_DRAG_KEYBOARD_ORACLE")?)?
        .take(128 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 128 * 1024 {
        return Err("drag/Keyboard oracle exceeds bound".into());
    }
    let oracle: Oracle = serde_json::from_slice(&bytes)?;
    let cases: Vec<KeyboardCase> = serde_json::from_str(include_str!(
        "../../../tests/data/raw-drag-keyboard-cases.json"
    ))?;
    if oracle.version != "1.58.2" || cases.len() != 10 || oracle.observations.len() != cases.len() {
        return Err("incomplete pinned drag/Keyboard oracle".into());
    }
    for (sample, expected) in cases.iter().zip(&oracle.observations) {
        if sample.id != expected.id || sample.keys.len() > 4 {
            return Err("drag/Keyboard oracle order or operation bound differs".into());
        }
        // Act / Assert: exact native events are observed before host close;
        // borrowed Page survival and independent fixture cleanup follow.
        case(
            &endpoint,
            &fixture,
            false,
            Some((sample, &expected.snapshot, &oracle.browser)),
        )
        .await?;
    }
    Ok(())
}
