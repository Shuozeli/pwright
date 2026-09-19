//! Real safe-host assembly, before exposing an N-API method. No native handle
//! or transport shortcut is used for the Act phase.
use super::*;
use crate::{
    evaluation::{EvaluateParams, EvaluationValue},
    locator::HostLocator,
    locator_query::{QueryBudget, QueryLimits},
};
use std::{future::Future, time::Duration};

fn query(host: &Arc<HostSession>, frame: &str, selector: &str) -> Result<HostLocator, HostError> {
    let budget = QueryBudget::new(QueryLimits {
        recipes: 16,
        bytes: 1024 * 1024,
    })?;
    host.locator(
        frame,
        budget
            .prepare_leaf(selector.len())?
            .css(selector.to_owned())?,
    )
}
async fn evaluate(
    host: &HostSession,
    frame: &str,
    source: &str,
) -> Result<EvaluationValue, HostError> {
    host.frame_evaluate(frame, EvaluateParams::expression(source))
        .await
}
async fn fixture() -> (Arc<HostSession>, String, crate::PageDescriptor) {
    let endpoint = std::env::var("PWRIGHT_TEST_CDP").expect("explicit dedicated Chrome endpoint");
    let host = Arc::new(
        HostSession::connect_over_cdp(&endpoint, 5000)
            .await
            .unwrap(),
    );
    let context = host.new_context().await.unwrap();
    let page = host.new_page(&context.id).await.unwrap();
    (host, context.id, page)
}
async fn cleanup(host: Arc<HostSession>, context: String, outcome: Result<(), HostError>) {
    let context_close = host.close_context(&context).await;
    let session_close = host.close().await;
    outcome.expect("host upload case");
    context_close.expect("owned Context cleanup");
    session_close.expect("host shutdown");
    assert_eq!(host.metrics().unwrap(), crate::SessionMetrics::default());
    for class in ResourceClass::ALL {
        assert_eq!(
            host.resources.metrics().unwrap().class(class).current,
            0,
            "{class:?}"
        );
    }
}
fn check(condition: bool, message: &str) -> Result<(), HostError> {
    if condition {
        Ok(())
    } else {
        Err(HostError::backend(message))
    }
}
async fn bounded(work: impl Future<Output = Result<(), HostError>>) -> Result<(), HostError> {
    tokio::time::timeout(Duration::from_secs(20), work)
        .await
        .map_err(HostError::backend)?
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn host_buffer_upload_uses_rust_mime_and_preserves_bytes_events_and_cleanup() {
    // Arrange
    let (host, context, page) = fixture().await;
    let outcome = bounded(async {
        evaluate(&host, &page.main_frame_id, "document.body.innerHTML='<label for=f id=l>file</label><input id=f type=file>';globalThis.events=[];for(const t of ['input','change'])document.querySelector('input').addEventListener(t,e=>events.push(e.type));void 0").await?;
        let locator = query(&host, &page.main_frame_id, "#l")?;
        let input = host.prepare_file_payloads(&[MemoryFile { name: "note.txt", mime_type: None, buffer: &[0, 128, 255, 10] }])?;
        // Act
        locator.set_input_files(input, 5000).await?;
        // Assert
        let summary = evaluate(&host, &page.main_frame_id, "(async()=>{const f=document.querySelector('input').files[0];return JSON.stringify([f.name,f.type,[...new Uint8Array(await f.arrayBuffer())],events]);})()").await?;
        check(summary == EvaluationValue::String("[\"note.txt\",\"text/plain\",[0,128,255,10],[\"input\",\"change\"]]".into()), "buffered File summary differs")?;
        check(host.resources.metrics().map_err(HostError::backend)?.class(ResourceClass::ArtifactBytes).current == 0, "source bytes remain admitted")?;
        Ok(())
    }).await;
    cleanup(host, context, outcome).await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn host_path_upload_uses_admitted_rust_filesystem_preparation() {
    // Arrange
    let (host, context, page) = fixture().await;
    let outcome = bounded(async {
        evaluate(
            &host,
            &page.main_frame_id,
            "document.body.innerHTML='<input type=file>';void 0",
        )
        .await?;
        let locator = query(&host, &page.main_frame_id, "input")?;
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../pwright-file-input-source/tests");
        // Act
        locator
            .set_input_files_paths(&base, &["fixture.txt"], 5000)
            .await?;
        // Assert
        let text = evaluate(
            &host,
            &page.main_frame_id,
            "document.querySelector('input').files[0].text()",
        )
        .await?;
        check(
            text == EvaluationValue::String("bounded file-input bytes\n".into()),
            "path bytes differ",
        )?;
        Ok(())
    })
    .await;
    cleanup(host, context, outcome).await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn host_upload_timeout_covers_waiting_and_releases_prepared_source() {
    // Arrange
    let (host, context, page) = fixture().await;
    let outcome = bounded(async {
        let locator = query(&host, &page.main_frame_id, "#missing")?;
        let input = host.prepare_file_payloads(&[MemoryFile {
            name: "a",
            mime_type: None,
            buffer: b"x",
        }])?;
        // Act
        let result = locator.set_input_files(input, 50).await;
        // Assert
        check(
            matches!(result, Err(HostError::Timeout(50))),
            "missing target did not use original timeout",
        )?;
        check(
            host.resources
                .metrics()
                .map_err(HostError::backend)?
                .class(ResourceClass::ArtifactBytes)
                .current
                == 0,
            "timed-out source remains admitted",
        )?;
        Ok(())
    })
    .await;
    cleanup(host, context, outcome).await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn host_bounds_even_empty_prepared_sources_and_reuses_a_dropped_slot() {
    // Arrange
    let (host, context, _) = fixture().await;
    let first = host.prepare_file_payloads(&[]).unwrap();
    let second = host.prepare_file_payloads(&[]).unwrap();
    // Act
    let rejected = host.prepare_file_payloads(&[]);
    // Assert
    let outcome = check(
        matches!(rejected, Err(HostError::ResourceExhausted(_))),
        "third empty source was admitted",
    );
    drop(first);
    let replacement = host.prepare_file_payloads(&[]);
    let recovered = check(
        replacement.is_ok(),
        "dropped source did not return capacity",
    );
    drop(replacement);
    drop(second);
    cleanup(host, context, outcome.and(recovered)).await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn host_selected_upload_crosses_ordinary_command_limit_without_widening_it() {
    // Arrange
    let (host, context, page) = fixture().await;
    let outcome = bounded(async {
        evaluate(&host, &page.main_frame_id, "document.body.innerHTML='<input type=file>';void 0").await?;
        let locator = query(&host, &page.main_frame_id, "input")?;
        let fixture = vec![0xa5; 8 * 1024 * 1024];
        let input = host.prepare_file_payloads(&[MemoryFile { name: "large.bin", mime_type: Some("application/octet-stream"), buffer: &fixture }])?;
        drop(fixture);
        // Act
        locator.set_input_files(input, 10000).await?;
        // Assert
        let verified = evaluate(&host, &page.main_frame_id, "(async()=>{const f=document.querySelector('input').files[0];const bytes=new Uint8Array(await f.arrayBuffer());return f.size===8388608&&bytes.every(x=>x===165);})()").await?;
        check(verified == EvaluationValue::Bool(true), "large host upload bytes differ")?;
        Ok(())
    }).await;
    cleanup(host, context, outcome).await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn upload_cleanup_preserves_a_concurrent_locator_evaluation_group() {
    // Arrange: a separately retained native element in the SAME session/world.
    let (host, context, page) = fixture().await;
    let outcome = bounded(async {
        evaluate(
            &host,
            &page.main_frame_id,
            "document.body.innerHTML='<input type=file>';void 0",
        )
        .await?;
        let locator = query(&host, &page.main_frame_id, "input")?;
        let retained = locator.copy()?;
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (resume_tx, resume_rx) = tokio::sync::oneshot::channel();
        let mut calls = tokio::task::JoinSet::new();
        calls.spawn(async move {
            retained
                .evaluate_with(5000, move || async {
                    ready_tx.send(()).map_err(|_| HostError::TargetClosed)?;
                    resume_rx.await.map_err(HostError::backend)?;
                    Ok(EvaluateParams::function_without_argument(
                        "element=>element.files[0].name",
                    ))
                })
                .await
        });
        ready_rx.await.map_err(HostError::backend)?;
        let input = host.prepare_file_payloads(&[MemoryFile {
            name: "shared.txt",
            mime_type: None,
            buffer: b"x",
        }])?;
        // Act
        locator.set_input_files(input, 5000).await?;
        // Assert: upload cleanup must not invalidate the other group's object.
        resume_tx.send(()).map_err(|_| HostError::TargetClosed)?;
        let observed = calls
            .join_next()
            .await
            .ok_or(HostError::TargetClosed)?
            .map_err(HostError::backend)??;
        check(
            observed == EvaluationValue::String("shared.txt".into()),
            "upload retired a concurrent evaluation group",
        )?;
        Ok(())
    })
    .await;
    cleanup(host, context, outcome).await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn host_upload_strictness_failure_does_not_modify_either_input() {
    // Arrange
    let (host, context, page) = fixture().await;
    let outcome = bounded(async {
        evaluate(
            &host,
            &page.main_frame_id,
            "document.body.innerHTML='<input type=file><input type=file>';void 0",
        )
        .await?;
        let locator = query(&host, &page.main_frame_id, "input")?;
        let input = host.prepare_file_payloads(&[MemoryFile {
            name: "a",
            mime_type: None,
            buffer: b"x",
        }])?;
        // Act
        let result = locator.set_input_files(input, 1000).await;
        // Assert
        check(
            result.is_err_and(|error| error.to_string().contains("strict mode violation")),
            "strictness error missing",
        )?;
        let unchanged = evaluate(
            &host,
            &page.main_frame_id,
            "[...document.querySelectorAll('input')].every(input=>input.files.length===0)",
        )
        .await?;
        check(
            unchanged == EvaluationValue::Bool(true),
            "strictness failure mutated an input",
        )?;
        Ok(())
    })
    .await;
    cleanup(host, context, outcome).await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn context_close_cancels_pending_file_selection_and_host_shutdown_drains_owners() {
    // Arrange
    let (host, context, page) = fixture().await;
    let outcome = bounded(async {
        let locator = query(&host, &page.main_frame_id, "#missing")?;
        let input = host.prepare_file_payloads(&[MemoryFile {
            name: "a",
            mime_type: None,
            buffer: b"x",
        }])?;
        let pending = locator.set_input_files(input, 0);
        tokio::pin!(pending);
        let first =
            std::future::poll_fn(|cx| std::task::Poll::Ready(pending.as_mut().poll(cx))).await;
        check(first.is_pending(), "selection should be pending")?;
        // Act
        let (closed, result) = tokio::join!(host.close_context(&context), pending);
        // Assert
        closed?;
        check(
            matches!(result, Err(HostError::TargetClosed)),
            "Context close did not stop upload",
        )?;
        Ok(())
    })
    .await;
    // The Act owns Context closure; do not close it twice.
    let session_close = host.close().await;
    outcome.expect("pending upload close");
    session_close.expect("host close");
    assert_eq!(host.metrics().unwrap(), crate::SessionMetrics::default());
}
