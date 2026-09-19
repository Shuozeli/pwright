//! Typed native Mouse starter oracle; wire/Node API remain separate gates.
use pwright_backend_cdp_input::{
    CdpInputCoordinator, CdpInputPolicy, CdpMouseDispatcher, DirectMouseDispatch,
    MouseDispatchKind, MouseDispatchOperation, MouseDispatchRequest,
};
use pwright_backend_cdp_mouse::*;
use pwright_cap_mouse::*;
use pwright_cdp::{CdpClient, CdpConnection, CdpSession, connection::ConnectionConfig};
use pwright_core::{ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope};
use pwright_input_model::InvalidMouseMoveSteps;
use serde::Deserialize;
use serde_json::Value;
use std::{
    error::Error,
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
type TestResult<T> = Result<T, Box<dyn Error>>;
/// Test-only direct provider: exercises selection, not Chromium drag semantics.
#[derive(Default)]
struct Passthrough([AtomicUsize; 4]);
impl CdpMouseDispatcher for Passthrough {
    fn start(
        &self,
        client: Arc<dyn CdpClient>,
        request: MouseDispatchRequest,
    ) -> Box<dyn MouseDispatchOperation> {
        let index = match request.kind {
            MouseDispatchKind::Move { for_click: false } => 0,
            MouseDispatchKind::Move { for_click: true } => 1,
            MouseDispatchKind::Down => 2,
            MouseDispatchKind::Up => 3,
        };
        self.0[index].fetch_add(1, Ordering::Relaxed);
        Box::new(DirectMouseDispatch::new(client, request.event))
    }
}
struct Delay;
impl CdpMouseDelay for Delay {
    fn wait(self: Arc<Self>, duration: Duration) -> MouseNativeFuture {
        Box::pin(async move {
            tokio::time::sleep(duration).await;
            Ok(())
        })
    }
}
fn native_services(
    page: Arc<CdpSession>,
    input: &CdpInputCoordinator,
    dispatcher: Option<Arc<Passthrough>>,
) -> TestResult<(CdpMouseServiceFactory, CdpMouseServices)> {
    let bytes = CdpMouseServices::required_operation_bytes(page.clone(), input)?;
    let profile = CdpMouseProductionProfile::new(
        ResourceScope::extension(14, 32),
        bytes,
        std::num::NonZeroUsize::new(1).unwrap(),
    )?;
    let requirements = profile.requirements();
    let class = requirements.class(ResourceClass::BackendStateBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
            ResourceClass::BackendStateBytes,
            class.global,
            class.per_scope,
        ),
        requirements,
    )?;
    let factory = CdpMouseServiceFactory::new_with_production_resources(
        profile,
        &resources,
        Arc::new(Delay),
    )?;
    let services = match dispatcher {
        Some(dispatcher) => factory.bind_with_dispatcher(page, input.clone(), dispatcher)?,
        None => factory.bind(page, input.clone())?,
    };
    Ok((factory, services))
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
    page: &Arc<CdpSession>,
    fixture: &str,
    sample: &Case,
    expected: &Observation,
    is_mac: bool,
    dispatcher: Option<Arc<Passthrough>>,
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
    let (factory, services) = native_services(page.clone(), &input, dispatcher)?;
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
        let operation = match action {
            Action::Click(click) => Some(
                services.mouse_click.start(ClickParams {
                    x: MouseNumber::try_from(click.x)?,
                    y: MouseNumber::try_from(click.y)?,
                    button: click.button.unwrap_or(Button::Left).native(),
                    click_count: MouseClickCount::try_from(click.count.unwrap_or(1.0))?,
                    delay: click
                        .delay
                        .map(|value| Duration::try_from_secs_f64(value / 1000.0))
                        .transpose()?,
                })?,
            ),
            Action::DoubleClick(click) => Some(
                services.mouse_dblclick.start(DblclickParams {
                    x: MouseNumber::try_from(click.x)?,
                    y: MouseNumber::try_from(click.y)?,
                    button: click.button.unwrap_or(Button::Left).native(),
                    delay: click
                        .delay
                        .map(|value| Duration::try_from_secs_f64(value / 1000.0))
                        .transpose()?,
                })?,
            ),
            Action::Move { x, y, steps, .. } => Some(services.mouse_move.start(MoveParams {
                x: MouseNumber::try_from(*x)?,
                y: MouseNumber::try_from(*y)?,
                steps: MouseMoveSteps::try_from(steps.unwrap_or(1.0))?,
            })?),
            Action::Down { button, count } => Some(services.mouse_down.start(DownParams {
                button: button.unwrap_or(Button::Left).native(),
                click_count: count.unwrap_or(1),
            })?),
            Action::Up { button, count } => Some(services.mouse_up.start(UpParams {
                button: button.unwrap_or(Button::Left).native(),
                click_count: count.unwrap_or(1),
            })?),
            Action::Wheel { x, y } => {
                wheels += 1;
                Some(services.mouse_wheel.start(WheelParams {
                    delta_x: MouseNumber::try_from(*x)?,
                    delta_y: MouseNumber::try_from(*y)?,
                })?)
            }
            Action::KeyDown { key } => {
                let mut transaction = input.begin_transaction(page.clone(), None).await?;
                transaction
                    .key_down_named(key, is_mac)
                    .await
                    .map_err(|error| format!("{error:?}"))?;
                transaction.finish().await?;
                None
            }
            Action::KeyUp { key } => {
                let mut transaction = input.begin_transaction(page.clone(), None).await?;
                transaction
                    .key_up_named(key, is_mac)
                    .await
                    .map_err(|error| format!("{error:?}"))?;
                transaction.finish().await?;
                None
            }
        };
        if let Some(mut operation) = operation {
            let result = tokio::time::timeout(Duration::from_secs(5), operation.result()).await;
            let retirement = operation.retire().await;
            retirement?;
            result??;
            if operation.pending_native_calls() != 0 {
                return Err("native Mouse calls remain after retirement".into());
            }
        }
        if factory.active_operations() != 0 {
            return Err("native Mouse admission remains after operation drop".into());
        }
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
async fn typed_native_mouse_matches_pinned_playwright_events() -> TestResult<()> {
    compare_native(None).await
}

#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight, serve-lib fixture and fresh pinned Mouse oracle"]
async fn selected_passthrough_mouse_matches_pinned_playwright_events() -> TestResult<()> {
    // Arrange
    let dispatcher = Arc::new(Passthrough::default());
    // Act
    compare_native(Some(dispatcher.clone())).await?;
    // Assert: actually used all four request kinds; not silently the default path.
    for count in &dispatcher.0 {
        assert!(count.load(Ordering::Relaxed) > 0);
    }
    Ok(())
}

async fn compare_native(dispatcher: Option<Arc<Passthrough>>) -> TestResult<()> {
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
            compare(
                &page,
                &fixture,
                sample,
                expected,
                oracle.is_mac,
                dispatcher.clone(),
            )
            .await
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
