//! Diagnostic ABBA experiment, not the production attachment path or a parity gate.
//! Only enables session-local Page/Runtime domains and reads frame metadata.
//! Never registers scripts, creates worlds, resumes, navigates or closes targets.
use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};

use pwright_cdp::{
    connection::{CdpConnection, ConnectionConfig},
    domains::target::TargetInfo,
    session::CdpSession,
};
use serde::Serialize;

type ProbeResult<T> = Result<T, Box<dyn Error>>;
const DEADLINE: Duration = Duration::from_secs(5);
const MAX_TARGETS: usize = 8;

fn target_ids(targets: Vec<TargetInfo>) -> ProbeResult<Vec<String>> {
    let mut ids = Vec::new();
    for target in targets
        .into_iter()
        .filter(|target| target.target_type == "page")
    {
        if ids.len() == MAX_TARGETS || target.target_id.is_empty() || target.target_id.len() > 128 {
            return Err("invalid or excessive diagnostic Page identities".into());
        }
        ids.push(target.target_id);
    }
    ids.sort();
    if ids.len() < 2 || ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("probe requires two to eight distinct Pages".into());
    }
    Ok(ids)
}

fn websocket(value: &str) -> ProbeResult<url::Url> {
    if value.len() > 256 {
        return Err("diagnostic endpoint too long".into());
    }
    let url = url::Url::parse(value)?;
    if url.scheme() != "ws"
        || url.host_str() != Some("127.0.0.1")
        || url.port() != Some(9222)
        || !url.path().starts_with("/devtools/browser/")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("explicit dedicated loopback browser WebSocket required".into());
    }
    Ok(url)
}

fn configuration() -> ConnectionConfig {
    ConnectionConfig {
        timeout: DEADLINE,
        write_channel_capacity: 32,
        event_channel_capacity: 1024,
        pending_command_capacity: 32,
        max_command_bytes: 4 * 1024 * 1024,
        max_response_bytes: 4 * 1024 * 1024,
        max_event_bytes: 256 * 1024,
        max_websocket_bytes: 4 * 1024 * 1024,
        max_protocol_error_bytes: 4096,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Sample {
    target_id: String,
    started_ms: u128,
    page_enabled_ms: u128,
    finished_ms: u128,
}

async fn prerequisites(session: &CdpSession, started: Instant) -> ProbeResult<Sample> {
    let started_ms = started.elapsed().as_millis();
    session.page_enable().await?;
    let page_enabled_ms = started.elapsed().as_millis();
    let (tree, ()) = tokio::try_join!(session.page_get_frame_tree(), session.runtime_enable())?;
    if tree.frame.id.is_empty() || tree.frame.parent_id.is_some() {
        return Err("invalid Page main-frame metadata".into());
    }
    // Discard frame URLs and tree payloads; retain only bounded target/timing metadata.
    Ok(Sample {
        target_id: session.target_id().to_owned(),
        started_ms,
        page_enabled_ms,
        finished_ms: started.elapsed().as_millis(),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Leg {
    width: usize,
    targets: Vec<String>,
    prerequisite_ms: u128,
    samples: Vec<Sample>,
    error: Option<String>,
    cleanup_errors: Vec<String>,
}

fn error_text(error: impl std::fmt::Display) -> String {
    error.to_string().chars().take(512).collect()
}

async fn leg(ws: &str, width: usize, expected: Option<&[String]>) -> ProbeResult<Leg> {
    let connection = tokio::time::timeout(
        DEADLINE,
        CdpConnection::connect_with_config(ws, configuration()),
    )
    .await??;
    let browser = CdpSession::browser(Arc::clone(&connection));
    let mut sessions = Vec::new();
    let mut report = Leg {
        width,
        targets: Vec::new(),
        prerequisite_ms: 0,
        samples: Vec::new(),
        error: None,
        cleanup_errors: Vec::new(),
    };
    let setup = tokio::time::timeout(DEADLINE, async {
        report.targets = target_ids(browser.target_get_targets().await?)?;
        if expected.is_some_and(|expected| expected != report.targets) {
            return Err::<(), Box<dyn Error>>(
                "target inventory changed during ABBA experiment".into(),
            );
        }
        for target in &report.targets {
            let id = browser.target_attach(target).await?;
            // Save each acknowledged session before the next fallible operation.
            sessions.push(CdpSession::new(Arc::clone(&connection), id, target.clone()));
        }
        Ok(())
    })
    .await;
    match setup {
        Ok(Ok(())) => {
            let started = Instant::now();
            let result = tokio::time::timeout(DEADLINE, async {
                for pair in sessions.chunks(width) {
                    let first = prerequisites(&pair[0], started);
                    if pair.len() == 2 {
                        // Exactly two caller-polled sessions, no spawned initialization tasks.
                        let (first, second) =
                            tokio::try_join!(first, prerequisites(&pair[1], started))?;
                        report.samples.extend([first, second]);
                    } else {
                        report.samples.push(first.await?);
                    }
                }
                Ok::<(), Box<dyn Error>>(())
            })
            .await;
            report.prerequisite_ms = started.elapsed().as_millis();
            report.error = match result {
                Ok(Ok(())) => None,
                Ok(Err(error)) => Some(error_text(error)),
                Err(error) => Some(error_text(error)),
            };
        }
        Ok(Err(error)) => report.error = Some(error_text(error)),
        Err(error) => report.error = Some(error_text(error)),
    }
    // Detach only this connection's acknowledged sessions. Transport Drop also
    // retires any attachment whose response was lost to the setup deadline.
    let cleanup = tokio::time::timeout(DEADLINE, async {
        for session in &sessions {
            if let Err(error) = browser
                .target_detach(session.session_id().expect("target session"))
                .await
            {
                report.cleanup_errors.push(error_text(error));
            }
        }
    })
    .await;
    if let Err(error) = cleanup {
        report.cleanup_errors.push(error_text(error));
    }
    Ok(report)
}

#[tokio::test]
#[ignore = "diagnostic only; requires explicit dedicated Chrome/Pwright preflight"]
async fn compare_serial_and_paired_session_prerequisites() -> ProbeResult<()> {
    // Arrange: the launcher holds a pinned oracle connection for all four legs.
    assert_eq!(std::env::var("PWRIGHT_TEST_CDP")?, "http://127.0.0.1:9222");
    let ws = websocket(&std::env::var("PWRIGHT_TEST_WS")?)?;
    let mut reports: Vec<Leg> = Vec::new();
    // Act: fixed ABBA order, never adaptive retries. No native addon involved.
    for width in [1, 2, 2, 1] {
        let expected = reports.first().map(|report| report.targets.as_slice());
        let report = leg(ws.as_str(), width, expected).await?;
        println!("{}", serde_json::to_string(&report)?);
        reports.push(report);
    }
    // Assert: failure stays failure; timing alone makes no compatibility claim.
    assert!(
        reports
            .iter()
            .all(|report| report.error.is_none() && report.cleanup_errors.is_empty())
    );
    Ok(())
}

fn fixture(id: &str) -> TargetInfo {
    TargetInfo {
        target_id: id.into(),
        target_type: "page".into(),
        title: String::new(),
        url: String::new(),
        attached: false,
    }
}

#[test]
fn diagnostic_inventory_is_bounded_and_sorted() {
    // Arrange
    let input = vec![fixture("b"), fixture("a")];
    // Act
    let actual = target_ids(input).unwrap();
    // Assert
    assert_eq!(actual, ["a", "b"]);
}

#[test]
fn diagnostic_inventory_rejects_duplicate_or_excessive_targets() {
    // Arrange
    let inputs = [
        vec![fixture("a"), fixture("a")],
        (0..9).map(|index| fixture(&index.to_string())).collect(),
        vec![fixture("a"), fixture(&"x".repeat(129))],
        vec![fixture("a"), fixture("")],
    ];
    // Act
    let rejected = inputs.into_iter().all(|input| target_ids(input).is_err());
    // Assert
    assert!(rejected);
}

#[test]
fn diagnostic_websocket_cannot_leave_dedicated_loopback() {
    // Arrange
    let inputs = [
        "ws://100.1.2.3:9222/devtools/browser/a",
        "ws://127.0.0.1:9333/devtools/browser/a",
        "ws://user:secret@127.0.0.1:9222/devtools/browser/a",
        "ws://127.0.0.1:9222/devtools/page/a",
    ];
    // Act
    let rejected = inputs.into_iter().all(|input| websocket(input).is_err());
    // Assert
    assert!(rejected);
}
