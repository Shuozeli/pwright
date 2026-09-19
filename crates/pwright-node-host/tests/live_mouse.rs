//! Safe HostSession Mouse comparison; CDP is only the fixture observer/cleanup auditor.
use pwright_cdp::{CdpConnection, CdpSession, connection::ConnectionConfig};
use pwright_input_model::InvalidMouseMoveSteps;
use pwright_node_host::{HostSession, PageDescriptor, mouse::*};
use serde::Deserialize;
use serde_json::Value;
use std::{error::Error, io::Read, sync::Arc, time::Duration};
type TestResult<T> = Result<T, Box<dyn Error>>;

#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight and verified serve-lib Mouse fixture"]
async fn host_close_during_click_delay_joins_retirement_without_synthetic_mouseup() -> TestResult<()>
{
    // Arrange
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
    // The observer owns this non-dispose-on-detach Context. HostSession only
    // borrows it, leaving the DOM available for the post-close release audit.
    let native_context = browser.target_create_browser_context(None, None).await?;
    let mut host_owner = None;
    let mut owned_target = None;
    let result = async {
        let mut probe_url = url.clone();
        probe_url.query_pairs_mut().append_pair("mouse-close-probe", &native_context);
        let target = browser.target_create_in_context(probe_url.as_str(), &native_context).await?;
        owned_target = Some(target.clone());
        let session = browser.target_attach(&target).await?;
        let page = CdpSession::new(connection.clone(), session, target);
        page.runtime_enable().await?;
        tokio::time::timeout(Duration::from_secs(5), async {
            while page.runtime_evaluate("typeof window.resetMouse === 'function'").await?["result"]["value"] != true {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            Ok::<_, Box<dyn Error>>(())
        }).await??;
        host_owner = Some(HostSession::connect_over_cdp("http://127.0.0.1:9222", 5000).await?);
        let host = host_owner.as_ref().ok_or("host did not attach")?;
        let mut descriptors = Vec::new();
        for context in host.contexts()? {
            for candidate in host.pages(&context.id)? {
                // Cached topology metadata only; never act on unrelated Pages.
                if candidate.main_frame_metadata.url()? == probe_url.as_str() { descriptors.push(candidate); }
            }
        }
        if descriptors.len() != 1 { return Err("cannot identify exact borrowed Page route".into()); }
        let descriptor = descriptors.pop().ok_or("borrowed Page missing")?;
        page.runtime_evaluate("window.resetMouse()").await?;
        let click = host.mouse_click(&descriptor.id, ClickParams {
            x: MouseNumber::try_from(20.0)?, y: MouseNumber::try_from(30.0)?,
            button: MouseButton::Left, click_count: MouseClickCount::ONE, delay: Some(Duration::from_secs(60)),
        });
        // Act: wait for actual native press before closing, not a fixed sleep.
        let close = async {
            let pressed = tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if page.runtime_evaluate("window.mouseSnapshot().records.some(event => event.type === 'mousedown')").await?["result"]["value"] == true { break; }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                Ok::<_, Box<dyn Error>>(())
            }).await;
            host.request_shutdown();
            let closed = host.close().await;
            pressed??;
            closed?;
            Ok::<_, Box<dyn Error>>(())
        };
        let (clicked, closed) = tokio::join!(click, close);
        // Assert
        closed?;
        if !matches!(clicked, Err(pwright_node_host::HostError::TargetClosed)) { return Err("click did not receive host-close failure".into()); }
        let snapshot = page.runtime_evaluate("window.mouseSnapshot()").await?["result"]["value"].clone();
        if snapshot["overflow"] != false || snapshot["records"].as_array().ok_or("missing event records")?.iter().any(|event| event["type"] == "mouseup" || event["type"] == "click") {
            return Err("host close synthesized a release/click".into());
        }
        let metrics = host.metrics()?;
        if metrics.connected || metrics.active_leases != 0 || metrics.active_operations != 0 || metrics.cleanup_in_flight != 0 { return Err(format!("host close retained owners: {metrics:?}").into()); }
        println!("host close during 60-second click delay: TargetClosed, no synthetic release, all host leases/tasks drained");
        Ok::<_, Box<dyn Error>>(())
    }.await;
    // Host close is attachment teardown; dispose only this test-owned Context.
    let closed = match &host_owner {
        Some(host) => host.close().await,
        None => Ok(()),
    };
    if browser
        .target_get_browser_contexts()
        .await?
        .contains(&native_context)
    {
        browser
            .target_dispose_browser_context(&native_context)
            .await?;
    }
    if browser
        .target_get_browser_contexts()
        .await?
        .iter()
        .any(|id| id == &native_context)
        || browser
            .target_get_targets()
            .await?
            .iter()
            .any(|target| Some(&target.target_id) == owned_target.as_ref())
    {
        return Err("close-probe target/Context remains".into());
    }
    closed?;
    result
}
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
    host: &HostSession,
    descriptor: &PageDescriptor,
    page: &Arc<CdpSession>,
    fixture: &str,
    sample: &Case,
    expected: &Observation,
    _is_mac: bool,
) -> TestResult<()> {
    page.page_enable().await?;
    page.runtime_enable().await?;
    host.page_goto(
        &descriptor.id,
        fixture.to_owned(),
        "domcontentloaded",
        None,
        5000,
    )
    .await?;
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
        let _ = match action {
            Action::Click(click) => Some(
                host.mouse_click(
                    &descriptor.id,
                    ClickParams {
                        x: MouseNumber::try_from(click.x)?,
                        y: MouseNumber::try_from(click.y)?,
                        button: click.button.unwrap_or(Button::Left).native(),
                        click_count: MouseClickCount::try_from(click.count.unwrap_or(1.0))?,
                        delay: click
                            .delay
                            .map(|value| Duration::try_from_secs_f64(value / 1000.0))
                            .transpose()?,
                    },
                )
                .await?,
            ),
            Action::DoubleClick(click) => Some(
                host.mouse_dblclick(
                    &descriptor.id,
                    DblclickParams {
                        x: MouseNumber::try_from(click.x)?,
                        y: MouseNumber::try_from(click.y)?,
                        button: click.button.unwrap_or(Button::Left).native(),
                        delay: click
                            .delay
                            .map(|value| Duration::try_from_secs_f64(value / 1000.0))
                            .transpose()?,
                    },
                )
                .await?,
            ),
            Action::Move { x, y, steps, .. } => Some(
                host.mouse_move(
                    &descriptor.id,
                    MoveParams {
                        x: MouseNumber::try_from(*x)?,
                        y: MouseNumber::try_from(*y)?,
                        steps: MouseMoveSteps::try_from(steps.unwrap_or(1.0))?,
                    },
                )
                .await?,
            ),
            Action::Down { button, count } => Some(
                host.mouse_down(
                    &descriptor.id,
                    DownParams {
                        button: button.unwrap_or(Button::Left).native(),
                        click_count: count.unwrap_or(1),
                    },
                )
                .await?,
            ),
            Action::Up { button, count } => Some(
                host.mouse_up(
                    &descriptor.id,
                    UpParams {
                        button: button.unwrap_or(Button::Left).native(),
                        click_count: count.unwrap_or(1),
                    },
                )
                .await?,
            ),
            Action::Wheel { x, y } => {
                wheels += 1;
                Some(
                    host.mouse_wheel(
                        &descriptor.id,
                        WheelParams {
                            delta_x: MouseNumber::try_from(*x)?,
                            delta_y: MouseNumber::try_from(*y)?,
                        },
                    )
                    .await?,
                )
            }
            Action::KeyDown { key } => {
                host.keyboard_down(&descriptor.id, key).await?;
                None
            }
            Action::KeyUp { key } => {
                host.keyboard_up(&descriptor.id, key).await?;
                None
            }
        };
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
async fn safe_host_mouse_matches_pinned_playwright_events() -> TestResult<()> {
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
    let host = HostSession::connect_over_cdp("http://127.0.0.1:9222", 5000).await?;
    let result = async {
        if host.version() != oracle.browser {
            return Err("host browser version differs".into());
        }
        let mut failures = Vec::new();
        for (sample, expected) in cases.iter().zip(&oracle.observations) {
            if sample.id != expected.id || sample.actions.len() > 12 {
                return Err("oracle order/action bound differs".into());
            }
            let before = browser.target_get_browser_contexts().await?;
            let context = host.new_context().await?;
            let mut native_context = None;
            let mut owned_target = None;
            // Act: route/input ownership comes from HostSession, not a locally
            // manufactured coordinator or standalone Mouse recipe.
            let observed = async {
                let created: Vec<_> = browser
                    .target_get_browser_contexts()
                    .await?
                    .into_iter()
                    .filter(|id| !before.contains(id))
                    .collect();
                if created.len() != 1 {
                    return Err("cannot identify exact host-owned Context".into());
                }
                native_context = Some(created[0].clone());
                let descriptor = host.new_page(&context.id).await?;
                let inventory: pwright_cdp::generated::target::GetTargetsReturns =
                    serde_json::from_value(
                        browser
                            .send("Target.getTargets", serde_json::json!({}))
                            .await?,
                    )?;
                let targets: Vec<_> = inventory
                    .target_infos
                    .into_iter()
                    .filter(|target| {
                        target.browser_context_id.as_ref() == native_context.as_ref()
                            && target.r#type == "page"
                    })
                    .collect();
                if targets.len() != 1 {
                    return Err("cannot identify exact host-owned Page".into());
                }
                let target = targets[0].target_id.clone();
                owned_target = Some(target.clone());
                let session = browser.target_attach(&target).await?;
                let page = Arc::new(CdpSession::new(connection.clone(), session, target));
                let comparison = compare(
                    &host,
                    &descriptor,
                    &page,
                    &fixture,
                    sample,
                    expected,
                    oracle.is_mac,
                )
                .await;
                // Native command deadlines remain owned by the host workers.
                // This only bounds observation of their already-requested drain.
                let drained = tokio::time::timeout(Duration::from_secs(5), async {
                    loop {
                        let metrics = host.mouse_metrics()?;
                        if metrics.retained_invocations == 0 && metrics.retirement_tasks == 0 {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                    Ok::<_, Box<dyn Error>>(())
                })
                .await?;
                drained?;
                if host.mouse_metrics()?.cleanup_failures != 0 {
                    return Err("host Mouse cleanup failed".into());
                }
                comparison
            }
            .await;
            let closed = host.close_context(&context.id).await;
            closed?;
            // Assert native absence, not just portable close acknowledgement.
            if browser
                .target_get_targets()
                .await?
                .iter()
                .any(|target| Some(&target.target_id) == owned_target.as_ref())
                || browser
                    .target_get_browser_contexts()
                    .await?
                    .iter()
                    .any(|id| Some(id) == native_context.as_ref())
            {
                return Err("host-owned Mouse target or Context remains".into());
            }
            match observed {
                Ok(()) => println!(
                    "host mouse {}: exact DOM events and independent retirement match",
                    sample.id
                ),
                Err(error) => failures.push(format!("{}: {error}", sample.id)),
            }
        }
        if failures.is_empty() {
            Ok::<_, Box<dyn Error>>(())
        } else {
            Err(failures.join("\n").into())
        }
    }
    .await;
    // Always join the host even when an observation or assertion fails.
    let closed = host.close().await;
    let metrics = host.metrics()?;
    if metrics.active_leases != 0
        || metrics.active_operations != 0
        || metrics.cleanup_in_flight != 0
        || metrics.connected
    {
        return Err(format!("host failed to drain: {metrics:?}").into());
    }
    closed?;
    result
}
