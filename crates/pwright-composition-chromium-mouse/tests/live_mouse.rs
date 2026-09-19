//! Binary-wire (default) or controlled-runtime (opt-in) Page Mouse oracle.
use pwright_backend_cdp_input::{CdpInputCoordinator, CdpInputPolicy};
use pwright_backend_cdp_mouse::*;
use pwright_cap_mouse::*;
use pwright_cdp::{CdpConnection, CdpSession, connection::ConnectionConfig};
#[cfg(not(feature = "runtime"))]
use pwright_codec_mouse::{MouseCodec, MouseCodecError};
use pwright_composition_chromium_mouse::*;
use pwright_core::{CallMetadata, CancellationId, Command, CommandSpec};
#[cfg(not(feature = "runtime"))]
use pwright_core::{CommandResult, OperationContext};
use pwright_core::{ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope};
use pwright_input_model::InvalidMouseMoveSteps;
#[cfg(not(feature = "runtime"))]
use pwright_server_wire::{ConcurrentServerFrameHandler, ServerFrameDisposition};
#[cfg(not(feature = "runtime"))]
use pwright_transport::{FrameTransport, WireCodec, WireFrame};
use serde::Deserialize;
use serde_json::Value;
#[cfg(not(feature = "runtime"))]
use std::{
    convert::Infallible,
    future::{Ready, ready},
    sync::Mutex,
};
use std::{error::Error, io::Read, sync::Arc, time::Duration};
type TestResult<T> = Result<T, Box<dyn Error>>;
struct Delay;
struct Probe {
    route: ChromiumMouseInstallation,
    #[cfg(feature = "runtime")]
    runtime: pwright_runtime::ControlledDispatcher<(), (), Delay>,
}
#[cfg(feature = "runtime")]
impl pwright_runtime::DeadlineScheduler for Delay {
    type Sleep<'a> = tokio::time::Sleep;
    fn sleep(&self, duration: Duration) -> Option<Self::Sleep<'_>> {
        Some(tokio::time::sleep(duration))
    }
}
impl CdpMouseDelay for Delay {
    fn wait(self: Arc<Self>, duration: Duration) -> MouseNativeFuture {
        Box::pin(async move {
            tokio::time::sleep(duration).await;
            Ok(())
        })
    }
}
fn installation(page: Arc<CdpSession>, input: &CdpInputCoordinator) -> TestResult<Probe> {
    let capacity = std::num::NonZeroUsize::new(1).unwrap();
    let bytes = CdpMouseServices::required_operation_bytes(page.clone(), input)?;
    let backend =
        CdpMouseProductionProfile::new(ResourceScope::extension(14, 32), bytes, capacity)?;
    let profile = ChromiumMouseProfile::new(
        backend,
        capacity,
        MouseCodecLimits::new(4096),
        ProtocolVersion::new(1, 0),
        FrameLimits::new(16384, 0, 0, 16384),
        FrameLimits::new(16384, 0, 0, 16384),
    )?;
    let requirements = profile.requirements();
    #[cfg(feature = "runtime")]
    let runtime_limits = pwright_runtime::RuntimeLimits::new(1, 4).with_active_commands(1);
    #[cfg(feature = "runtime")]
    let requirements = requirements.combine_concurrent(runtime_limits.resource_requirements());
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bytes = requirements.class(class);
        limits = limits.with_class(class, bytes.global, bytes.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, requirements)?;
    let route = ChromiumMouseInstallation::install(
        ObjectId::new(7),
        page,
        input.clone(),
        profile,
        Arc::new(Delay),
        &resources,
    )?;
    #[cfg(feature = "runtime")]
    let runtime = {
        use pwright_browser_model::{
            BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, FRAME_OBJECT_KIND, PAGE_OBJECT_KIND,
        };
        let runtime =
            pwright_runtime::ControlledDispatcher::with_cleanup_executor_and_object_kinds(
                (),
                (),
                Delay,
                runtime_limits,
                pwright_runtime::CleanupQueue::new(4),
                pwright_core::ObjectKindCatalog::new(&[
                    BROWSER_OBJECT_KIND,
                    BROWSER_CONTEXT_OBJECT_KIND,
                    PAGE_OBJECT_KIND,
                    FRAME_OBJECT_KIND,
                ]),
            )?
            .with_production_resources(&resources)?;
        runtime.register_object(ObjectId::new(7), None, PAGE_OBJECT_KIND.tag)?;
        runtime
    };
    Ok(Probe {
        route,
        #[cfg(feature = "runtime")]
        runtime,
    })
}
#[cfg(not(feature = "runtime"))]
#[derive(Default)]
struct Transport(Mutex<Option<WireFrame>>);
#[cfg(not(feature = "runtime"))]
impl FrameTransport for Transport {
    type Error = Infallible;
    type Send<'a> = Ready<Result<(), Infallible>>;
    type Receive<'a> = Ready<Result<Option<WireFrame>, Infallible>>;
    fn send(&self, frame: WireFrame) -> Self::Send<'_> {
        assert!(
            self.0.lock().unwrap().replace(frame).is_none(),
            "one retained reply"
        );
        ready(Ok(()))
    }
    fn receive(&self) -> Self::Receive<'_> {
        ready(Ok(None))
    }
}
#[cfg(not(feature = "runtime"))]
async fn dispatch<C>(probe: &mut Probe, id: u64, params: C::Params) -> TestResult<()>
where
    C: CommandSpec<Result = (), ErrorDetails = MouseErrorDetails>,
    MouseCodec: WireCodec<Command<C>, Error = MouseCodecError>
        + WireCodec<CommandResult<C>, Error = MouseCodecError>,
{
    let installed = &mut probe.route;
    let protocol = installed.protocol();
    let metadata = CallMetadata::new(None, Some(CancellationId::new(id)));
    let context = OperationContext::new(
        CallId::new(id),
        ObjectId::new(7),
        C::METHOD,
        metadata.cancellation,
    );
    let command = Command::<C>::new(CallId::new(id), ObjectId::new(7), metadata, params);
    let frame = protocol.encode(&command)?;
    let handlers = installed.handlers();
    let transport = Arc::new(Transport::default());
    // Dropping the timed-out wire future requests retirement but retains the
    // native owner in the Page queue. Keep the installation until it drains.
    let handled = tokio::time::timeout(
        Duration::from_secs(5),
        handlers.handle_concurrently(frame, transport.clone()),
    )
    .await;
    while installed.page().queued_operations() != 0 {
        let receipt = installed.retirement().next().await;
        if receipt.context() != context {
            return Err("retirement identity differs".into());
        }
        receipt.into_outcome()?;
    }
    if installed.page().retained_records() != 0 {
        return Err("Mouse route record remains".into());
    }
    if !matches!(handled??, ServerFrameDisposition::Handled(_)) {
        return Err("Mouse wire route not handled".into());
    }
    let frame = transport
        .0
        .lock()
        .unwrap()
        .take()
        .ok_or("missing wire reply")?;
    let result = protocol.decode::<CommandResult<C>>(frame)?;
    if result.call_id() != CallId::new(id) {
        return Err("reply identity differs".into());
    }
    result.into_outcome()?;
    Ok(())
}

#[cfg(feature = "runtime")]
async fn dispatch<C>(probe: &mut Probe, id: u64, params: C::Params) -> TestResult<()>
where
    C: CommandSpec<Result = (), ErrorDetails = MouseErrorDetails> + 'static,
    C::Params: Send + 'static,
    ChromiumMouseInvocation<C>: pwright_runtime::TargetedService<
            C::Params,
            Response = (),
            Error = pwright_core::OperationError<MouseErrorDetails>,
        >,
{
    let command = Command::<C>::new(
        CallId::new(id),
        ObjectId::new(7),
        CallMetadata::new(Some(Duration::from_secs(5)), Some(CancellationId::new(id))),
        params,
    );
    let invocation = Arc::new(ChromiumMouseInvocation::for_command(
        probe.route.page(),
        &command,
    )?);
    let context = invocation.context();
    let mut execution =
        probe
            .runtime
            .dispatch_with(command, invocation.as_ref(), invocation.clone());
    let mut cleanup = Ok(());
    // Poll receipts concurrently with controlled cancellation: awaiting only
    // execution would deadlock a deadline waiting on its native receipt.
    let result = loop {
        tokio::select! {
            biased;
            result = &mut execution => break result,
            receipt = probe.route.retirement().next() => {
                if receipt.context() != context { return Err("controlled receipt identity differs".into()); }
                let retired = receipt.into_outcome();
                if cleanup.is_ok() { cleanup = retired; }
            }
        }
    };
    drop(execution);
    while probe.route.page().queued_operations() != 0 {
        let receipt = probe.route.retirement().next().await;
        if receipt.context() != context {
            return Err("controlled receipt identity differs".into());
        }
        let retired = receipt.into_outcome();
        if cleanup.is_ok() {
            cleanup = retired;
        }
    }
    drop(invocation);
    if probe.route.page().retained_records() != 0
        || probe.runtime.pending_call_count()? != 0
        || probe.runtime.connection_metrics()?.active_commands != 0
        || probe.runtime.cleanup_executor().metrics()?.queued != 0
    {
        return Err("controlled Mouse owners remain".into());
    }
    cleanup?;
    let result = result?;
    if result.call_id() != CallId::new(id) {
        return Err("controlled reply identity differs".into());
    }
    result.into_outcome()?;
    Ok(())
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
    let mut installed = installation(page.clone(), &input)?;
    let mut wheels = 0;
    let mut outcomes = Vec::new();
    for (index, action) in sample.actions.iter().enumerate() {
        let id = u64::try_from(index)? + 1;
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
                dispatch::<MouseClick>(
                    &mut installed,
                    id,
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
                dispatch::<MouseDblclick>(
                    &mut installed,
                    id,
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
                dispatch::<MouseMove>(
                    &mut installed,
                    id,
                    MoveParams {
                        x: MouseNumber::try_from(*x)?,
                        y: MouseNumber::try_from(*y)?,
                        steps: MouseMoveSteps::try_from(steps.unwrap_or(1.0))?,
                    },
                )
                .await?,
            ),
            Action::Down { button, count } => Some(
                dispatch::<MouseDown>(
                    &mut installed,
                    id,
                    DownParams {
                        button: button.unwrap_or(Button::Left).native(),
                        click_count: count.unwrap_or(1),
                    },
                )
                .await?,
            ),
            Action::Up { button, count } => Some(
                dispatch::<MouseUp>(
                    &mut installed,
                    id,
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
                    dispatch::<MouseWheel>(
                        &mut installed,
                        id,
                        WheelParams {
                            delta_x: MouseNumber::try_from(*x)?,
                            delta_y: MouseNumber::try_from(*y)?,
                        },
                    )
                    .await?,
                )
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
async fn page_mouse_matches_pinned_playwright_events() -> TestResult<()> {
    // Arrange
    println!(
        "Mouse oracle mode: {}",
        if cfg!(feature = "runtime") {
            "controlled runtime and explicit receipt pump"
        } else {
            "generated binary wire"
        }
    );
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
        // outer case future would discard the Page route's retained native owner.
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
