//! Native Frame verification through HostLocator. Internal enumeration is not
//! a public Node Page.frames() implementation or an alternate input transport.
use crate::locator::action::{LocatorAction, PressKey, PressOptions};
use crate::locator_query::{QueryBudget, QueryLimits};
use crate::mouse::{DownParams, MouseButton, MouseMoveSteps, MouseNumber, MoveParams, UpParams};
use crate::{HostError, HostSession};
use pwright_cdp::{CdpConnection, CdpSession, connection::ConnectionConfig};
use pwright_core::ObjectId;
use serde::Deserialize;
use serde_json::Value;
use std::{error::Error, io::Read, sync::Arc, time::Duration};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

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
    mode: String,
    operation: String,
    oopif: bool,
    drag: Value,
    frame: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    mode: String,
    operation: String,
    oopif: bool,
}

async fn move_to(host: &HostSession, page: &str, x: f64) -> Result<()> {
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

fn child_frame(
    host: &HostSession,
    page: ObjectId,
    main: ObjectId,
    url: &str,
    oopif: bool,
) -> Result<Option<ObjectId>> {
    let state = host.state.lock().map_err(HostError::backend)?;
    let live = state.as_ref().ok_or(HostError::TargetClosed)?;
    let guard = live.composition.topology()?;
    let topology = guard.topology();
    let mut selected = None;
    for (frame, owner) in topology.frame_page_routes() {
        if owner != page || frame == main || host.frame_metadata.get(frame)?.url()? != url {
            continue;
        }
        if selected.is_some() {
            return Err("duplicate child Frame route".into());
        }
        if topology.parent_frame_object_id(frame) != Some(main) {
            return Err("incorrect portable parent Frame".into());
        }
        let child_session = topology
            .frame_session_id(frame)
            .ok_or("child native session missing")?;
        let main_session = topology
            .frame_session_id(main)
            .ok_or("main native session missing")?;
        if (child_session != main_session) != oopif {
            return Err("Host topology does not prove expected native process boundary".into());
        }
        selected = Some(frame);
    }
    Ok(selected)
}

async fn frame_snapshot(observer: &CdpSession, child: Option<&CdpSession>) -> Result<Value> {
    Ok(match child {
        Some(child) => child.runtime_evaluate("frameSnapshot()").await?,
        None => {
            observer
                .runtime_evaluate("document.querySelector('iframe').contentWindow.frameSnapshot()")
                .await?
        }
    }["result"]["value"]
        .clone())
}

async fn compare(
    endpoint: &str,
    fixture: &url::Url,
    sample: &Observation,
    version: &str,
) -> Result<()> {
    // Arrange: observer owns the Context; HostSession only borrows its Page.
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
    let mut host_owner = None;
    let mut owned_target = None;
    let mut owned_child = None;
    let stage = std::cell::Cell::new("create fixture");
    let outcome = tokio::time::timeout(Duration::from_secs(25), async {
        let mut url = fixture.clone();
        url.query_pairs_mut().append_pair("frame-mode", &sample.mode).append_pair("frame-case", &context);
        let mut child_url = url.join("raw-drag-frame.html")?;
        child_url.set_query(url.query());
        let target = browser.target_create_in_context(url.as_str(), &context).await?;
        owned_target = Some(target.clone());
        let observer = CdpSession::new(connection.clone(), browser.target_attach(&target).await?, target.clone());
        observer.runtime_enable().await?;
        stage.set("observe native child readiness");
        let child = if sample.oopif {
            let child_target = loop {
                let targets = browser.target_get_targets().await?;
                let mut found = targets.iter().filter(|candidate| candidate.target_type == "iframe" && candidate.url == child_url.as_str());
                if let Some(candidate) = found.next() {
                    if found.next().is_some() { return Err("duplicate native OOPIF targets".into()); }
                    break candidate.target_id.clone();
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            };
            owned_child = Some(child_target.clone());
            let child = CdpSession::new(connection.clone(), browser.target_attach(&child_target).await?, child_target);
            child.runtime_enable().await?;
            while child.runtime_evaluate("typeof frameSnapshot === 'function'").await?["result"]["value"] != true { tokio::time::sleep(Duration::from_millis(5)).await; }
            Some(child)
        } else {
            while observer.runtime_evaluate("typeof document.querySelector('iframe')?.contentWindow?.frameSnapshot === 'function'").await?["result"]["value"] != true { tokio::time::sleep(Duration::from_millis(5)).await; }
            if browser.target_get_targets().await?.iter().any(|target| target.target_type == "iframe" && target.url == child_url.as_str()) { return Err("local Frame unexpectedly has a remote target".into()); }
            None
        };
        stage.set("attach HostSession");
        host_owner = Some(Arc::new(HostSession::connect_over_cdp("http://127.0.0.1:9222", 5000).await?));
        let host = host_owner.as_ref().ok_or("host missing")?;
        if host.version() != version { return Err("oracle/host browser versions differ".into()); }
        let mut pages = Vec::new();
        for context in host.contexts()? {
            for page in host.pages(&context.id)? {
                if page.main_frame_metadata.url()? == url.as_str() { pages.push(page); }
            }
        }
        if pages.len() != 1 { return Err("exact borrowed Page missing".into()); }
        let page = pages.pop().ok_or("Page missing")?;
        let page_id = ObjectId::new(page.id.parse()?);
        let main_id = ObjectId::new(page.main_frame_id.parse()?);
        stage.set("select portable child route");
        let frame = loop {
            host.flush_topology().await?;
            if let Some(frame) = child_frame(host, page_id, main_id, child_url.as_str(), sample.oopif)? { break frame; }
            tokio::time::sleep(Duration::from_millis(5)).await;
        };
        let budget = QueryBudget::new(QueryLimits { recipes: 2, bytes: 64 * 1024 })?;
        let query = budget.prepare_leaf(9)?.css(if sample.operation == "before-focus" { "#absent" } else { "#keyboard" }.into())?;
        let locator = host.locator(&frame.get().to_string(), query)?;
        stage.set("start native drag");
        move_to(host, &page.id, 40.0).await?;
        if !["sequential", "before-focus"].contains(&sample.operation.as_str()) {
            host.mouse_down(&page.id, DownParams { button: MouseButton::Left, click_count: 1 }).await?;
            move_to(host, &page.id, 70.0).await?;
            if observer.runtime_evaluate("dragFixture.events.includes('dragstart')").await?["result"]["value"] != true { return Err("native drag did not start".into()); }
        }
        let mut detached_snapshot = None;
        let mut terminal_drag = None;
        // Act: Frame selection/focus, Page keyboard and drag cancellation all
        // execute through real HostLocator/HostSession, never the observer.
        let removes_frame = ["detach", "detach-finish", "before-focus"].contains(&sample.operation.as_str());
        let destroys_page = ["page-close", "context-close"].contains(&sample.operation.as_str());
        if sample.operation == "before-focus" {
            stage.set("admit unresolved Locator before Frame removal");
            let mut pending = Box::pin(locator.act(LocatorAction::Press {
                key: PressKey::new("Escape")?, options: PressOptions::new().with_no_wait_after(true),
            }, 5000));
            std::future::poll_fn(|cx| match std::future::Future::poll(pending.as_mut(), cx) {
                std::task::Poll::Pending => std::task::Poll::Ready(Ok(())),
                std::task::Poll::Ready(result) => std::task::Poll::Ready(Err(format!("unresolved Locator completed before detach: {}", result.err().map_or("resolved".into(), |error| error.to_string())))),
            }).await?;
            if host.metrics()?.active_operations != 1 { return Err("pre-focus call was not admitted".into()); }
            let frame = frame_snapshot(&observer, child.as_ref()).await?;
            if frame["focused"] != false || frame["keys"] != serde_json::json!([]) { return Err("input preceded pre-focus detach".into()); }
            observer.runtime_evaluate("document.querySelector('iframe').remove()").await?;
            stage.set("await pre-focus Frame cancellation");
            let result = pending.await;
            if !matches!(result, Err(HostError::TargetClosed)) { return Err(format!("pre-focus detach must close target: {}", result.err().map_or("resolved".into(), |error| error.to_string())).into()); }
            detached_snapshot = Some(frame);
            host.flush_topology().await?;
            if child_frame(host, page_id, main_id, child_url.as_str(), sample.oopif)?.is_some() { return Err("pre-focus detached route survived".into()); }
            host.keyboard_down(&page.id, "Escape").await?;
            host.keyboard_up(&page.id, "Escape").await?;
        } else if destroys_page {
            stage.set("press and destroy owning Page or Context");
            let pending = async {
                let result = locator.act(LocatorAction::Press {
                    key: PressKey::new("Escape")?,
                    options: PressOptions::new().with_no_wait_after(true).with_delay(Duration::from_secs(60)),
                }, 5000).await;
                if !matches!(result, Err(HostError::TargetClosed)) {
                    return Err(format!("closed Page input must be TargetClosed, received {}", result.err().map_or("resolved".into(), |error| error.to_string())).into());
                }
                Ok::<_, Box<dyn Error>>(())
            };
            let destroy = async {
                while observer.runtime_evaluate("dragFixture.events.includes('dragend')").await?["result"]["value"] != true {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                let frame = frame_snapshot(&observer, child.as_ref()).await?;
                let drag = observer.runtime_evaluate("dragFixture").await?["result"]["value"].clone();
                if frame["keys"] != serde_json::json!([]) { return Err("keyup preceded Page destruction".into()); }
                stage.set("await Page input cancellation after native destruction");
                if sample.operation == "page-close" { browser.target_close(&target).await?; }
                else { browser.target_dispose_browser_context(&context).await?; }
                Ok::<_, Box<dyn Error>>((drag, frame))
            };
            let ((), (drag, frame)) = tokio::try_join!(pending, destroy)?;
            terminal_drag = Some(drag);
            detached_snapshot = Some(frame);
            host.flush_topology().await?;
            {
                let state = host.state.lock().map_err(HostError::backend)?;
                if state.as_ref().ok_or(HostError::TargetClosed)?.runtime.is_object_live(page_id)? { return Err("closed portable Page remains live".into()); }
            }
            if browser.target_get_browser_contexts().await?.contains(&context) != (sample.operation == "page-close") { return Err("incorrect native Context destruction scope".into()); }
        } else if removes_frame {
            stage.set("press and await native dragend");
            let finish = sample.operation == "detach-finish";
            let pending = async {
                let result = locator.act(LocatorAction::Press {
                key: PressKey::new("Escape")?,
                options: PressOptions::new().with_no_wait_after(!finish).with_delay(if finish { Duration::from_millis(1500) } else { Duration::from_secs(60) }),
                }, 5000).await;
                if finish { result?; }
                else if !matches!(result, Err(HostError::Timeout(5000))) { return Err(format!("detached Locator did not preserve the oracle timeout: {}", result.err().map_or("resolved".into(), |error| error.to_string())).into()); }
                Ok::<_, Box<dyn Error>>(())
            };
            let detach = async {
                while observer.runtime_evaluate("dragFixture.events.includes('dragend')").await?["result"]["value"] != true {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                let snapshot = frame_snapshot(&observer, child.as_ref()).await?;
                if snapshot["keys"] != serde_json::json!([]) { return Err("keyup preceded Frame removal".into()); }
                stage.set("remove child during key delay");
                observer.runtime_evaluate("document.querySelector('iframe').remove()").await?;
                stage.set("await Page keyboard completion");
                Ok::<_, Box<dyn Error>>(snapshot)
            };
            let ((), snapshot) = tokio::try_join!(pending, detach)?;
            detached_snapshot = Some(snapshot);
            stage.set("flush detached Frame topology");
            host.flush_topology().await?;
            if child_frame(host, page_id, main_id, child_url.as_str(), sample.oopif)?.is_some() { return Err("detached portable Frame route survived".into()); }
            stage.set("probe surviving Page keyboard");
            host.keyboard_down(&page.id, "Escape").await?;
            host.keyboard_up(&page.id, "Escape").await?;
        } else if sample.operation == "sequential" {
            locator.act(LocatorAction::PressSequentially { text: "a中".into(), delay: None }, 5000).await?;
        } else {
            let timeout = sample.operation == "timeout";
            let result = locator.act(LocatorAction::Press {
                key: PressKey::new(if sample.operation == "chord" { "Control+Escape" } else { "Escape" })?,
                options: PressOptions::new().with_no_wait_after(true).with_delay(if timeout { Duration::from_secs(60) } else { Duration::ZERO }),
            }, if timeout { 2000 } else { 5000 }).await;
            if timeout {
                if !matches!(result, Err(HostError::Timeout(2000))) { return Err("expected Locator timeout".into()); }
                host.keyboard_down(&page.id, "Escape").await?;
                host.keyboard_up(&page.id, "Escape").await?;
            } else { result?; }
        }
        stage.set("finish native mouse");
        if !destroys_page {
            move_to(host, &page.id, 220.0).await?;
            host.mouse_up(&page.id, UpParams { button: MouseButton::Left, click_count: 1 }).await?;
        }
        // Assert before host close, so shutdown cannot hide missing Escape.
        if host.metrics()?.active_operations != 0 { return Err("settled input retained active operations".into()); }
        let drag = match terminal_drag { Some(drag) => drag, None => observer.runtime_evaluate("dragFixture").await?["result"]["value"].clone() };
        let frame = match detached_snapshot { Some(snapshot) => snapshot, None => frame_snapshot(&observer, child.as_ref()).await? };
        if drag != sample.drag || frame != sample.frame { return Err(format!("{} differs: drag={drag}; frame={frame}; expected drag={}; frame={}", sample.id, sample.drag, sample.frame).into()); }
        stage.set("close borrowed HostSession");
        host.close().await?;
        let metrics = host.metrics()?;
        if metrics.connected || metrics.active_leases != 0 || metrics.active_operations != 0 { return Err(format!("retained host owners: {metrics:?}").into()); }
        if !removes_frame && !destroys_page && frame_snapshot(&observer, child.as_ref()).await? != frame { return Err("host shutdown changed borrowed Frame input".into()); }
        if browser.target_get_targets().await?.iter().any(|item| item.target_id == target) == destroys_page { return Err("incorrect borrowed Page lifetime".into()); }
        Ok::<_, Box<dyn Error>>(())
    }).await;
    let closed = match host_owner.as_ref() {
        Some(host) => host.close().await,
        None => Ok(()),
    };
    let audit = match host_owner.as_ref() {
        Some(host) => host.metrics().and_then(|metrics| {
            if metrics.connected || metrics.active_leases != 0 || metrics.active_operations != 0 {
                Err(HostError::Backend(format!(
                    "failed-case cleanup retained owners: {metrics:?}"
                )))
            } else {
                Ok(())
            }
        }),
        None => Ok(()),
    };
    if browser
        .target_get_browser_contexts()
        .await?
        .contains(&context)
    {
        browser.target_dispose_browser_context(&context).await?;
    }
    if browser
        .target_get_browser_contexts()
        .await?
        .contains(&context)
        || browser.target_get_targets().await?.iter().any(|target| {
            Some(&target.target_id) == owned_target.as_ref()
                || Some(&target.target_id) == owned_child.as_ref()
        })
    {
        return Err("owned Frame fixture survived disposal".into());
    }
    closed?;
    audit?;
    outcome
        .map_err(|error| format!("{}: {error}", stage.get()))?
        .map_err(|error| format!("{}: {error}", stage.get()))?;
    eprintln!(
        "Host Frame {}: native process proof, exact input/drag parity, expected Page lifetime and zero host owners",
        sample.id
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight, serve-lib fixture and pinned Frame oracle"]
async fn host_locator_keyboard_matches_native_local_and_oopif_frames() -> Result<()> {
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
    let mut bytes = Vec::new();
    std::fs::File::open(std::env::var("PWRIGHT_DRAG_FRAME_ORACLE")?)?
        .take(128 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 128 * 1024 {
        return Err("Frame oracle exceeds bound".into());
    }
    let oracle: Oracle = serde_json::from_slice(&bytes)?;
    if oracle.version != "1.58.2" || oracle.observations.len() != 18 {
        return Err("incomplete pinned Frame oracle".into());
    }
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../../tests/data/raw-drag-frame-cases.json"
    ))?;
    if cases.len() != oracle.observations.len() {
        return Err("shared Frame catalog size differs".into());
    }
    let mut failures = Vec::new();
    for (case, sample) in cases.iter().zip(&oracle.observations) {
        if sample.id != case.id
            || sample.mode != case.mode
            || sample.operation != case.operation
            || sample.oopif != case.oopif
            || !["local", "sandbox"].contains(&case.mode.as_str())
            || ![
                "escape",
                "chord",
                "sequential",
                "timeout",
                "detach",
                "detach-finish",
                "page-close",
                "context-close",
                "before-focus",
            ]
            .contains(&case.operation.as_str())
            || case.oopif != (case.mode == "sandbox")
        {
            return Err("Frame catalog order/process proof mismatch".into());
        }
        if let Err(error) = compare(&endpoint, &fixture, sample, &oracle.browser).await {
            eprintln!(
                "Host Frame {} FAILED after owned cleanup: {error}",
                sample.id
            );
            failures.push(sample.id.as_str());
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("Frame discrepancy gate failed: {}", failures.join(", ")).into())
    }
}
