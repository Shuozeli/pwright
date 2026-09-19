#![cfg(feature = "handles")]

use std::{sync::Arc, time::Duration};

use pwright_node_host::{
    HostSession, SessionMetrics,
    evaluation::{EvaluateParams, EvaluationValue},
    handles::EvaluateHandleParams,
};

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn close_retires_pending_creation_and_ready_handle_without_closing_borrowed_page() {
    // Arrange: an independent observer owns the Context. The subject may only
    // detach its own inspector sessions, not close the underlying Page/Context.
    let endpoint = std::env::var("PWRIGHT_TEST_CDP").unwrap();
    let observer = Arc::new(
        HostSession::connect_over_cdp(&endpoint, 5000)
            .await
            .unwrap(),
    );
    let context = observer.new_context().await.unwrap();
    let page = observer.new_page(&context.id).await.unwrap();
    observer
        .page_goto(
            &page.id,
            "data:text/html,<title>NODE165-close-owned-sessions</title>".into(),
            "load",
            None,
            5000,
        )
        .await
        .unwrap();
    let host = Arc::new(
        HostSession::connect_over_cdp(&endpoint, 5000)
            .await
            .unwrap(),
    );
    let subject = host.clone();
    let observing = observer.clone();
    let mut exercise = tokio::spawn(async move {
        let frame = subject
            .contexts()
            .unwrap()
            .into_iter()
            .flat_map(|context| subject.pages(&context.id).unwrap())
            .find(|candidate| {
                candidate
                    .main_frame_metadata
                    .url()
                    .unwrap()
                    .contains("NODE165-close-owned-sessions")
            })
            .expect("subject discovers the borrowed Page")
            .main_frame_id;
        let ready = subject
            .frame_prepare_handle(
                &frame,
                EvaluateHandleParams::expression("({retained: true})"),
            )
            .await
            .unwrap();
        let creating_host = subject.clone();
        let creating = tokio::spawn(async move {
            creating_host
                .frame_prepare_handle(
                    &frame,
                    EvaluateHandleParams::expression(
                        "(globalThis.__node165NeverStarted = true, new Promise(() => {}))",
                    ),
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let started = observing
                    .frame_evaluate(
                        &page.main_frame_id,
                        EvaluateParams::expression("globalThis.__node165NeverStarted === true"),
                    )
                    .await
                    .unwrap();
                if started == EvaluationValue::Bool(true) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the unresolved acquisition reached the borrowed Page");

        // Act: never resolve or cancel the browser Promise from the observer.
        let closed = tokio::time::timeout(Duration::from_secs(10), subject.close()).await;

        // Assert: cleanup must finish cleanly, not merely time out its admitted
        // continuation. A Ready descriptor may remain in user memory throughout.
        closed
            .expect("close must not wait for the unresolved Promise")
            .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(2), creating)
            .await
            .unwrap()
            .unwrap();
        assert!(
            result.is_err(),
            "closed subject published an unfinished acquisition"
        );
        assert_eq!(
            observing.frame_title(&page.main_frame_id).await.unwrap(),
            "NODE165-close-owned-sessions"
        );
        assert_eq!(
            subject.metrics().unwrap(),
            SessionMetrics {
                active_leases: 2,
                live_scopes: 1,
                ..SessionMetrics::default()
            }
        );
        drop(ready);
        assert_eq!(subject.metrics().unwrap(), SessionMetrics::default());
    });
    let outcome = tokio::time::timeout(Duration::from_secs(30), &mut exercise).await;
    if outcome.is_err() {
        exercise.abort();
        let _ = exercise.await;
    }
    // Preserve cleanup ownership outside assertions and deadline cancellation.
    let closed = tokio::time::timeout(Duration::from_secs(10), host.close()).await;
    let context_closed =
        tokio::time::timeout(Duration::from_secs(10), observer.close_context(&context.id)).await;
    let observer_closed = tokio::time::timeout(Duration::from_secs(10), observer.close()).await;
    assert!(
        matches!(closed, Ok(Ok(()))),
        "subject cleanup failed: {closed:?}; exercise: {outcome:?}"
    );
    assert!(
        matches!(context_closed, Ok(Ok(()))),
        "owned Context cleanup failed: {context_closed:?}"
    );
    assert!(
        matches!(observer_closed, Ok(Ok(()))),
        "observer cleanup failed: {observer_closed:?}"
    );
    assert!(
        matches!(outcome, Ok(Ok(()))),
        "shutdown ownership exercise failed: {outcome:?}"
    );
}
