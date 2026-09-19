//! Ignored assembled-host proof. The Act uses no raw CDP or fabricated Locator;
//! cleanup owns only this test's Context and attachment, never shared Chrome.

use crate::{
    HostError, HostSession,
    evaluation::EvaluationValue,
    file_input::MemoryFile,
    handles::{EvaluateHandleParams, HandleEvaluateParams},
};
use pwright_core::ResourceClass;
use std::{
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};

const STEP: Duration = Duration::from_secs(10);
type TestResult<T> = Result<T, HostError>;

fn check(condition: bool, message: &'static str) -> TestResult<()> {
    if condition {
        Ok(())
    } else {
        Err(HostError::backend(message))
    }
}

async fn bounded<T>(
    name: &'static str,
    work: impl Future<Output = TestResult<T>>,
) -> TestResult<T> {
    tokio::time::timeout(STEP, work)
        .await
        .map_err(|_| HostError::backend(format_args!("{name} exceeded {STEP:?}")))?
}

async fn exercise(
    host: Arc<HostSession>,
    owned_context: Arc<Mutex<Option<String>>>,
) -> TestResult<()> {
    // Arrange an original input, then retain it through the assembled host.
    let context = bounded("owned Context creation", host.new_context()).await?;
    *owned_context.lock().map_err(HostError::backend)? = Some(context.id.clone());
    let page = bounded("owned Page creation", host.new_page(&context.id)).await?;
    let original = Arc::new(
        bounded(
            "exact ElementHandle creation",
            host.frame_prepare_handle(
                &page.main_frame_id,
                EvaluateHandleParams::expression(
                    "(() => {
                        const input = document.createElement('input');
                        input.type = 'file'; input.id = 'node169-upload-target';
                        globalThis.__node169Events = [];
                        for (const type of ['input', 'change'])
                            input.addEventListener(type, event => globalThis.__node169Events.push(event.type));
                        document.body.append(input);
                        return input;
                    })()",
                ),
            ),
        )
        .await?,
    );
    check(
        original.is_element(),
        "source did not produce an ElementHandle",
    )?;
    let moved = bounded(
        "original rename and move",
        host.handle_evaluate(
            &original,
            HandleEvaluateParams::function_without_argument(
                "input => {
                    input.id = 'node169-original';
                    const destination = document.createElement('section');
                    destination.id = 'node169-destination';
                    document.body.append(destination); destination.append(input);
                    const replacement = document.createElement('input');
                    replacement.type = 'file'; replacement.id = 'node169-upload-target';
                    document.body.prepend(replacement);
                    return input.isConnected && input !== document.querySelector('#node169-upload-target');
                }",
            ),
        ),
    )
    .await?;
    check(
        moved == EvaluationValue::Bool(true),
        "replacement fixture did not separate the nodes",
    )?;
    let payload = host.prepare_file_payloads(&[MemoryFile {
        name: "exact.bin",
        mime_type: Some("application/octet-stream"),
        buffer: &[0, 128, 255, 10],
    }])?;

    // Act: only the exact persistent Handle enters the production upload path.
    bounded(
        "retained Handle upload",
        host.handle_set_input_files(original.clone(), payload, 5000),
    )
    .await?;

    // Assert via the same source after temporary-group cleanup. The matching
    // replacement remains empty, proving that no selector re-query was used.
    let observed = bounded(
        "retained source verification",
        host.handle_evaluate(
            &original,
            HandleEvaluateParams::function_without_argument(
                "async input => {
                    const file = input.files[0];
                    return JSON.stringify([
                        input.id, input.parentElement.id, file.name, file.type,
                        Array.from(new Uint8Array(await file.arrayBuffer())),
                        globalThis.__node169Events,
                        document.getElementById('node169-upload-target').files.length
                    ]);
                }",
            ),
        ),
    )
    .await?;
    check(
        observed == EvaluationValue::String(
            "[\"node169-original\",\"node169-destination\",\"exact.bin\",\"application/octet-stream\",[0,128,255,10],[\"input\",\"change\"],0]".into(),
        ),
        "upload changed the replacement, lost bytes/events, or invalidated the persistent source",
    )?;
    check(
        host.handle_metrics()?.current == 1,
        "upload changed persistent Handle ownership",
    )?;
    bounded("explicit source disposal", original.dispose()).await?;
    check(
        host.handle_metrics()?.current == 0,
        "source disposal did not retire its route",
    )?;
    drop(original);
    Ok(())
}

#[tokio::test]
#[ignore = "requires approved dedicated Chrome preflight and PWRIGHT_TEST_CDP=http://127.0.0.1:9222"]
async fn assembled_retained_upload_preserves_exact_node_bytes_events_and_source_lifetime()
-> Result<(), String> {
    // Arrange: only the explicit, dedicated loopback endpoint is permitted.
    let endpoint = std::env::var("PWRIGHT_TEST_CDP").map_err(|error| error.to_string())?;
    if endpoint != "http://127.0.0.1:9222" {
        return Err("PWRIGHT_TEST_CDP must be exactly http://127.0.0.1:9222".into());
    }
    let host = Arc::new(
        bounded(
            "host attachment",
            HostSession::connect_over_cdp(&endpoint, 5000),
        )
        .await
        .map_err(|error| error.to_string())?,
    );
    let owned_context = Arc::new(Mutex::new(None));
    let mut body = tokio::spawn(exercise(host.clone(), owned_context.clone()));

    // Act with a total bound; capture panics as well as ordinary failures.
    let outcome = tokio::time::timeout(Duration::from_secs(30), &mut body).await;
    let mut failures = Vec::new();
    match outcome {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(error))) => failures.push(format!("exercise: {error}")),
        Ok(Err(error)) => failures.push(format!("exercise task: {error}")),
        Err(_) => {
            failures.push("exercise exceeded 30s".into());
            body.abort();
            if tokio::time::timeout(STEP, &mut body).await.is_err() {
                failures.push("aborted exercise did not finish within cleanup bound".into());
            }
        }
    }

    // Always attempt both owned Context and attachment cleanup independently,
    // even after setup/verification failure, panic, or a timed-out native phase.
    let context = owned_context
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    if let Some(context) = context
        && let Err(error) = bounded("owned Context cleanup", host.close_context(&context)).await
    {
        failures.push(error.to_string());
    }
    match tokio::time::timeout(STEP, host.close()).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => failures.push(format!("host cleanup: {error}")),
        Err(_) => {
            failures.push("host cleanup exceeded 10s".into());
            // A test timeout is not native cleanup evidence. As a final bounded
            // fallback, stop ONLY this host's transport and still report failure.
            let connection = host
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_ref()
                .map(|live| live.composition.connection().clone());
            if let Some(connection) = connection {
                connection.request_disconnect();
            }
            if let Err(error) = bounded("host cleanup after disconnect", host.close()).await {
                failures.push(error.to_string());
            }
        }
    }

    // Assert only after every cleanup attempt. Local metrics are not browser
    // RSS and are additional evidence, not a replacement for native ACKs.
    match host.metrics() {
        Ok(metrics) if metrics == crate::SessionMetrics::default() => {}
        result => failures.push(format!("terminal host metrics: {result:?}")),
    }
    match host.resources.metrics() {
        Ok(metrics) => {
            for class in ResourceClass::ALL {
                if metrics.class(class).current != 0 {
                    failures.push(format!(
                        "retained {class:?}: {}",
                        metrics.class(class).current
                    ));
                }
            }
        }
        Err(error) => failures.push(format!("terminal resource metrics: {error}")),
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}
