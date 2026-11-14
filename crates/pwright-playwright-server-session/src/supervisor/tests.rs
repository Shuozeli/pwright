use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pwright_playwright_server_client_core::{ClientError, CloseReason, Ingress, ReplyOutcome};
use pwright_transport_ws::{
    MAX_MESSAGE_BYTES, WsCloseFrame, WsConnector, WsEndpoint, WsError, WsIncoming, WsLimits,
    WsOutgoing, WsPeer, WsRequest,
};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio::time::timeout;

#[cfg(feature = "close-notices")]
use super::CloseNotice;
#[cfg(feature = "page-events")]
use super::PageNotice;
use super::{
    EventPolicy, MAX_SUPERVISOR_COMMANDS, MAX_SUPERVISOR_EVENT_BYTES, MAX_SUPERVISOR_EVENTS,
    MAX_SUPERVISOR_OBJECT_BYTES, MAX_SUPERVISOR_OBJECTS, ObjectProjection, ProjectionError,
    RemoteBrowserSupervisor, SupervisorError, SupervisorState,
};
use crate::{RemoteBrowserLease, SdkLanguage, SessionError};
#[cfg(any(feature = "page-events", feature = "close-notices"))]
use pwright_playwright_server_page_events::{
    MAX_QUEUED_NOTICES, ProjectionError as PageProjectionError,
};

struct FakeConnector {
    incoming: Mutex<Option<mpsc::UnboundedReceiver<WsIncoming>>>,
    sent: mpsc::UnboundedSender<WsOutgoing>,
    close_count: Arc<AtomicUsize>,
    received: Option<mpsc::UnboundedSender<()>>,
}

struct FakePeer {
    incoming: mpsc::UnboundedReceiver<WsIncoming>,
    sent: mpsc::UnboundedSender<WsOutgoing>,
    close_count: Arc<AtomicUsize>,
    received: Option<mpsc::UnboundedSender<()>>,
}

impl WsConnector for FakeConnector {
    type Peer = FakePeer;
    type Future<'a> = Ready<Result<Self::Peer, WsError>>;

    fn connect(&self, _request: &WsRequest) -> Self::Future<'_> {
        ready(Ok(FakePeer {
            incoming: self.incoming.lock().unwrap().take().unwrap(),
            sent: self.sent.clone(),
            close_count: Arc::clone(&self.close_count),
            received: self.received.clone(),
        }))
    }
}

impl WsPeer for FakePeer {
    type SendFuture<'a> = Ready<Result<(), WsError>>;
    type ReceiveFuture<'a> = Pin<Box<dyn Future<Output = Result<WsIncoming, WsError>> + Send + 'a>>;
    type CloseFuture<'a> = Ready<Result<(), WsError>>;

    fn send(&mut self, message: WsOutgoing) -> Self::SendFuture<'_> {
        ready(self.sent.send(message).map_err(|_| WsError::IoFailed))
    }

    fn receive(&mut self) -> Self::ReceiveFuture<'_> {
        Box::pin(async move {
            let incoming = self
                .incoming
                .recv()
                .await
                .unwrap_or(WsIncoming::EndOfStream);
            if let Some(received) = &self.received {
                let _ = received.send(());
            }
            Ok(incoming)
        })
    }

    fn close(&mut self) -> Self::CloseFuture<'_> {
        self.close_count.fetch_add(1, Ordering::SeqCst);
        ready(Ok(()))
    }
}

fn text(value: Value) -> WsIncoming {
    WsIncoming::Text(value.to_string())
}

fn success_frames() -> Vec<WsIncoming> {
    vec![
        text(
            json!({"guid":"","method":"__create__","params":{"type":"BrowserType","guid":"chromium@1","initializer":{"name":"chromium","executablePath":"/browser"}}}),
        ),
        text(
            json!({"guid":"","method":"__create__","params":{"type":"BrowserType","guid":"firefox@1","initializer":{"name":"firefox","executablePath":"/browser"}}}),
        ),
        text(
            json!({"guid":"","method":"__create__","params":{"type":"BrowserType","guid":"webkit@1","initializer":{"name":"webkit","executablePath":"/browser"}}}),
        ),
        text(
            json!({"guid":"","method":"__create__","params":{"type":"Android","guid":"android@1","initializer":{}}}),
        ),
        text(
            json!({"guid":"","method":"__create__","params":{"type":"Electron","guid":"electron@1","initializer":{}}}),
        ),
        text(
            json!({"guid":"chromium@1","method":"__create__","params":{"type":"Browser","guid":"browser@1","initializer":{"name":"chromium","version":"155.0"}}}),
        ),
        text(
            json!({"guid":"","method":"__create__","params":{"type":"Playwright","guid":"playwright@1","initializer":{
                "chromium":{"guid":"chromium@1"},
                "firefox":{"guid":"firefox@1"},
                "webkit":{"guid":"webkit@1"},
                "android":{"guid":"android@1"},
                "electron":{"guid":"electron@1"},
                "preLaunchedBrowser":{"guid":"browser@1"}
            }}}),
        ),
        text(json!({"id":1,"result":{"playwright":{"guid":"playwright@1"}}})),
    ]
}

struct Fixture {
    browser: RemoteBrowserSupervisor,
    incoming: mpsc::UnboundedSender<WsIncoming>,
    sent: mpsc::UnboundedReceiver<WsOutgoing>,
    close_count: Arc<AtomicUsize>,
}

async fn fixture_with_frames(frames: Vec<WsIncoming>) -> Fixture {
    fixture_with_frames_and_inbound_limit(frames, 256 * 1024).await
}

async fn fixture_with_frames_and_inbound_limit(
    frames: Vec<WsIncoming>,
    inbound_bytes: usize,
) -> Fixture {
    fixture_with_policy(frames, inbound_bytes, EventPolicy::QueueBounded).await
}

async fn fixture_with_policy(
    frames: Vec<WsIncoming>,
    inbound_bytes: usize,
    event_policy: EventPolicy,
) -> Fixture {
    fixture_with_policy_and_receipts(frames, inbound_bytes, event_policy, None).await
}

async fn fixture_with_policy_and_receipts(
    frames: Vec<WsIncoming>,
    inbound_bytes: usize,
    event_policy: EventPolicy,
    received: Option<mpsc::UnboundedSender<()>>,
) -> Fixture {
    let (incoming, receiver) = mpsc::unbounded_channel();
    let (sent, mut sent_receiver) = mpsc::unbounded_channel();
    let close_count = Arc::new(AtomicUsize::new(0));
    let connector = FakeConnector {
        incoming: Mutex::new(Some(receiver)),
        sent,
        close_count: Arc::clone(&close_count),
        received,
    };
    for frame in frames {
        incoming.send(frame).unwrap();
    }
    let request = WsRequest::new(
        WsEndpoint::parse("ws://example.test/ws").unwrap(),
        WsLimits::new(inbound_bytes, 64 * 1024, 1000).unwrap(),
    );
    let lease = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Javascript, 123)
        .await
        .unwrap();
    let browser = RemoteBrowserSupervisor::start_with_event_policy(lease, event_policy)
        .await
        .unwrap();
    let WsOutgoing::Text(initialize) = sent_receiver.recv().await.unwrap() else {
        panic!("expected Root.initialize");
    };
    assert_eq!(serde_json::from_str::<Value>(&initialize).unwrap()["id"], 1);
    Fixture {
        browser,
        incoming,
        sent: sent_receiver,
        close_count,
    }
}

async fn fixture() -> Fixture {
    let fixture = fixture_with_frames(success_frames()).await;
    for _ in 0..7 {
        assert!(matches!(
            fixture.browser.next_event().await.unwrap(),
            Ingress::Created { .. }
        ));
    }
    fixture
}

async fn next_command(sent: &mut mpsc::UnboundedReceiver<WsOutgoing>) -> Value {
    let message = timeout(Duration::from_secs(1), sent.recv())
        .await
        .unwrap()
        .unwrap();
    let WsOutgoing::Text(message) = message else {
        panic!("expected text command");
    };
    serde_json::from_str(&message).unwrap()
}

#[cfg(any(feature = "page-events", feature = "close-notices"))]
fn created_frame(parent: &str, object_type: &str, guid: &str, initializer: Value) -> WsIncoming {
    text(json!({
        "guid": parent,
        "method": "__create__",
        "params": {"type": object_type, "guid": guid, "initializer": initializer}
    }))
}

#[cfg(any(feature = "page-events", feature = "close-notices"))]
fn event_frame(guid: &str, method: &str, params: Value) -> WsIncoming {
    text(json!({"guid": guid, "method": method, "params": params}))
}

#[cfg(any(feature = "page-events", feature = "close-notices"))]
fn page_graph_frames() -> Vec<WsIncoming> {
    vec![
        created_frame("browser@1", "BrowserContext", "context@1", json!({})),
        created_frame("context@1", "Page", "page@1", json!({})),
    ]
}

#[cfg(feature = "page-events")]
fn request_frame(guid: &str) -> WsIncoming {
    created_frame(
        "page@1",
        "Request",
        guid,
        json!({
            "url": "https://example.test/report",
            "method": "POST",
            "resourceType": "fetch",
            "headers": [{"name": "Content-Type", "value": "application/json"}],
            "isNavigationRequest": false,
            "postData": "not retained"
        }),
    )
}

#[cfg(feature = "page-events")]
fn request_event(guid: &str, page_guid: &str) -> WsIncoming {
    event_frame(
        "context@1",
        "request",
        json!({"request": {"guid": guid}, "page": {"guid": page_guid}}),
    )
}

#[cfg(any(feature = "page-events", feature = "close-notices"))]
async fn created_sequence(browser: &RemoteBrowserSupervisor, guid: &str) -> u64 {
    timeout(Duration::from_secs(1), async {
        loop {
            if let Some(object) = browser.object(guid) {
                break object.created_sequence;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
}

#[cfg(feature = "frame-title")]
async fn frame_title_fixture(received: Option<mpsc::UnboundedSender<()>>) -> Fixture {
    let fixture = fixture_with_policy_and_receipts(
        success_frames(),
        256 * 1024,
        EventPolicy::DropUnsupportedEvents,
        received,
    )
    .await;
    for frame in [
        json!({"guid":"browser@1","method":"__create__","params":{"type":"BrowserContext","guid":"context@title","initializer":{}}}),
        json!({"guid":"context@title","method":"__create__","params":{"type":"Frame","guid":"frame@title","initializer":{}}}),
        json!({"guid":"context@title","method":"__create__","params":{"type":"Page","guid":"page@title","initializer":{"isClosed":false}}}),
        json!({"guid":"page@title","method":"__adopt__","params":{"guid":"frame@title"}}),
    ] {
        fixture.incoming.send(text(frame)).unwrap();
    }
    timeout(Duration::from_secs(1), async {
        while fixture
            .browser
            .object_with_frame_title_state("frame@title")
            .is_none_or(|(frame, epoch, _)| {
                frame.parent.as_deref() != Some("page@title") || epoch != 1
            })
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    fixture
}

#[cfg(feature = "frame-title")]
fn frame_title_generations(browser: &RemoteBrowserSupervisor) -> [u64; 4] {
    ["frame@title", "page@title", "context@title", "browser@1"]
        .map(|guid| browser.object(guid).unwrap().created_sequence)
}

#[cfg(feature = "frame-title")]
async fn queued_frame_title_after_ingress(ingress: Vec<Value>) {
    // Arrange
    let (received_sender, mut received) = mpsc::unbounded_channel();
    let fixture = frame_title_fixture(Some(received_sender)).await;
    while received.try_recv().is_ok() {}
    let browser = Arc::new(fixture.browser);
    let generations = frame_title_generations(&browser);
    let epoch = browser
        .object_with_frame_title_state("frame@title")
        .unwrap()
        .1;
    let (locked_sender, locked_receiver) = tokio::sync::oneshot::channel();
    let (release_sender, release_receiver) = std::sync::mpsc::channel();
    let objects = Arc::clone(&browser.objects);
    let projection_reader = std::thread::spawn(move || {
        let _projection_read = objects.read().unwrap();
        locked_sender.send(()).unwrap();
        release_receiver.recv().unwrap();
    });
    timeout(Duration::from_secs(1), locked_receiver)
        .await
        .unwrap()
        .unwrap();
    for frame in &ingress {
        fixture.incoming.send(text(frame.clone())).unwrap();
    }
    timeout(Duration::from_secs(1), received.recv())
        .await
        .unwrap()
        .unwrap();
    let title = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call_for_frame_title(
                    [
                        ("frame@title", generations[0]),
                        ("page@title", generations[1]),
                        ("context@title", generations[2]),
                        ("browser@1", generations[3]),
                    ],
                    epoch,
                    json!({}),
                )
                .await
        }
    });
    timeout(Duration::from_secs(1), async {
        while browser.commands.capacity() == MAX_SUPERVISOR_COMMANDS {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    // Act
    release_sender.send(()).unwrap();
    projection_reader.join().unwrap();
    let result = timeout(Duration::from_secs(1), title)
        .await
        .unwrap()
        .unwrap();

    // Assert
    assert_eq!(result, Err(SupervisorError::TargetGenerationChanged));
    assert!(fixture.sent.is_empty());
    browser.close().await.unwrap();
}

#[cfg(feature = "frame-title")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_frame_title_does_not_send_after_observed_adoption_away() {
    queued_frame_title_after_ingress(vec![
        json!({"guid":"context@title","method":"__adopt__","params":{"guid":"frame@title"}}),
    ])
    .await;
}

#[cfg(feature = "frame-title")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_frame_title_does_not_send_after_adoption_away_and_back() {
    queued_frame_title_after_ingress(vec![
        json!({"guid":"context@title","method":"__adopt__","params":{"guid":"frame@title"}}),
        json!({"guid":"page@title","method":"__adopt__","params":{"guid":"frame@title"}}),
    ])
    .await;
}

#[cfg(feature = "frame-title")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_frame_title_does_not_send_after_page_close_tombstone() {
    queued_frame_title_after_ingress(vec![
        json!({"guid":"page@title","method":"close","params":{}}),
    ])
    .await;
}

#[cfg(feature = "frame-title")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_queued_frame_title_sends_no_command() {
    // Arrange
    let (received_sender, mut received) = mpsc::unbounded_channel();
    let mut fixture = frame_title_fixture(Some(received_sender)).await;
    while received.try_recv().is_ok() {}
    let browser = Arc::new(fixture.browser);
    let generations = frame_title_generations(&browser);
    let epoch = browser
        .object_with_frame_title_state("frame@title")
        .unwrap()
        .1;
    let (locked_sender, locked_receiver) = tokio::sync::oneshot::channel();
    let (release_sender, release_receiver) = std::sync::mpsc::channel();
    let objects = Arc::clone(&browser.objects);
    let projection_reader = std::thread::spawn(move || {
        let _projection_read = objects.read().unwrap();
        locked_sender.send(()).unwrap();
        release_receiver.recv().unwrap();
    });
    timeout(Duration::from_secs(1), locked_receiver)
        .await
        .unwrap()
        .unwrap();
    fixture
        .incoming
        .send(text(
            json!({"guid":"page@title","method":"noop","params":{}}),
        ))
        .unwrap();
    timeout(Duration::from_secs(1), received.recv())
        .await
        .unwrap()
        .unwrap();
    let title = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call_for_frame_title(
                    [
                        ("frame@title", generations[0]),
                        ("page@title", generations[1]),
                        ("context@title", generations[2]),
                        ("browser@1", generations[3]),
                    ],
                    epoch,
                    json!({}),
                )
                .await
        }
    });
    timeout(Duration::from_secs(1), async {
        while browser.commands.capacity() == MAX_SUPERVISOR_COMMANDS {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    // Act
    title.abort();
    let _ = title.await;
    release_sender.send(()).unwrap();
    projection_reader.join().unwrap();
    timeout(Duration::from_secs(1), async {
        while browser.commands.capacity() != MAX_SUPERVISOR_COMMANDS {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    // Assert
    assert!(fixture.sent.try_recv().is_err());
    browser.close().await.unwrap();
}

#[cfg(feature = "page-events")]
async fn next_page_notice(browser: &RemoteBrowserSupervisor) -> PageNotice {
    timeout(Duration::from_secs(1), browser.next_page_notice())
        .await
        .unwrap()
        .unwrap()
}

#[cfg(feature = "close-notices")]
async fn next_close_notice(browser: &RemoteBrowserSupervisor) -> CloseNotice {
    timeout(Duration::from_secs(1), browser.next_close_notice())
        .await
        .unwrap()
        .unwrap()
}

#[cfg(feature = "close-notices")]
#[tokio::test]
async fn close_only_page_notice_is_visible_before_disposal() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectCloseNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    let page_sequence = created_sequence(&fixture.browser, "page@1").await;

    // Act
    fixture
        .incoming
        .send(event_frame("page@1", "close", json!({})))
        .unwrap();
    let notice = next_close_notice(&fixture.browser).await;

    // Assert
    assert_eq!(
        notice,
        CloseNotice::PageClosed(super::ObjectId {
            guid: "page@1".into(),
            generation: page_sequence,
        })
    );
    assert!(fixture.browser.object("page@1").is_some());
    assert!(fixture.browser.object_with_close_state("page@1").unwrap().1);
    assert!(
        fixture
            .browser
            .snapshot()
            .objects
            .iter()
            .any(|object| object.guid == "page@1" && object.closed)
    );
    assert_eq!(
        fixture.browser.next_event().await,
        Err(SupervisorError::EventStreamDisabled)
    );
    fixture.browser.close().await.unwrap();
}

#[cfg(feature = "close-notices")]
#[tokio::test]
async fn close_only_unsupported_burst_ignores_payloads_and_retains_wire_close() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectCloseNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }

    // Act
    fixture
        .incoming
        .send(created_frame(
            "page@1",
            "Request",
            "request@1",
            json!({"url": "not a complete Request"}),
        ))
        .unwrap();
    fixture
        .incoming
        .send(created_frame(
            "request@1",
            "Response",
            "response@1",
            json!({"status": "invalid"}),
        ))
        .unwrap();
    for index in 0..(MAX_QUEUED_NOTICES + 1) {
        let (guid, method) = match index % 4 {
            0 => ("context@1", "request"),
            1 => ("context@1", "response"),
            2 => ("page@1", "download"),
            _ => ("page@1", "crash"),
        };
        fixture
            .incoming
            .send(event_frame(guid, method, json!({"unsupported": index})))
            .unwrap();
    }
    fixture
        .incoming
        .send(event_frame("page@1", "close", json!({})))
        .unwrap();
    let notice = next_close_notice(&fixture.browser).await;

    // Assert
    assert!(matches!(notice, CloseNotice::PageClosed(id) if id.guid == "page@1"));
    assert!(fixture.browser.is_connected());
    fixture.browser.close().await.unwrap();
}

#[cfg(feature = "close-notices")]
#[tokio::test]
async fn close_only_unrelated_object_burst_exceeds_page_projection_cap_without_failing() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectCloseNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    let page_sequence = created_sequence(&fixture.browser, "page@1").await;

    // Act
    for index in 0..=pwright_playwright_server_page_events::MAX_PROJECTED_OBJECTS {
        fixture
            .incoming
            .send(created_frame(
                "page@1",
                "Request",
                &format!("request@{index}"),
                json!({"invalidRequestInitializer": true}),
            ))
            .unwrap();
    }
    fixture
        .incoming
        .send(event_frame("request@0", "close", json!({})))
        .unwrap();
    fixture
        .incoming
        .send(event_frame("request@0", "__dispose__", json!({})))
        .unwrap();
    fixture
        .incoming
        .send(event_frame("page@1", "close", json!({})))
        .unwrap();
    let notice = timeout(Duration::from_secs(5), fixture.browser.next_close_notice())
        .await
        .unwrap()
        .unwrap();

    // Assert
    assert_eq!(
        notice,
        CloseNotice::PageClosed(super::ObjectId {
            guid: "page@1".into(),
            generation: page_sequence,
        })
    );
    assert!(fixture.browser.is_connected());
    assert!(fixture.browser.object_with_close_state("page@1").unwrap().1);
    fixture.browser.close().await.unwrap();
}

#[cfg(feature = "close-notices")]
#[tokio::test]
async fn close_only_subtree_and_terminal_preserve_order_without_disposal_duplicates() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectCloseNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    created_sequence(&fixture.browser, "page@1").await;

    // Act
    for frame in [
        event_frame("context@1", "close", json!({})),
        event_frame("page@1", "__dispose__", json!({})),
        event_frame("context@1", "__dispose__", json!({})),
        event_frame("browser@1", "close", json!({})),
    ] {
        fixture.incoming.send(frame).unwrap();
    }
    let page = next_close_notice(&fixture.browser).await;
    let context = next_close_notice(&fixture.browser).await;
    let browser = next_close_notice(&fixture.browser).await;
    let disconnected = next_close_notice(&fixture.browser).await;

    // Assert
    assert!(matches!(page, CloseNotice::PageClosed(id) if id.guid == "page@1"));
    assert!(matches!(context, CloseNotice::ContextClosed(id) if id.guid == "context@1"));
    assert!(matches!(browser, CloseNotice::BrowserClosed(id) if id.guid == "browser@1"));
    assert_eq!(disconnected, CloseNotice::Disconnected);
    assert!(matches!(fixture.browser.next_close_notice().await,
        Err(SupervisorError::Terminal(failure)) if failure.error == SessionError::BrowserClosed));
}

#[cfg(feature = "close-notices")]
#[tokio::test]
async fn close_only_guid_reuse_gets_a_new_generation() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectCloseNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    created_sequence(&fixture.browser, "page@1").await;

    // Act
    fixture
        .incoming
        .send(event_frame("page@1", "__dispose__", json!({})))
        .unwrap();
    let first = next_close_notice(&fixture.browser).await;
    fixture
        .incoming
        .send(created_frame("context@1", "Page", "page@1", json!({})))
        .unwrap();
    fixture
        .incoming
        .send(event_frame("page@1", "close", json!({})))
        .unwrap();
    let second = next_close_notice(&fixture.browser).await;

    // Assert
    let CloseNotice::PageClosed(first) = first else {
        panic!("expected first Page close")
    };
    let CloseNotice::PageClosed(second) = second else {
        panic!("expected second Page close")
    };
    assert_eq!(first.guid, second.guid);
    assert!(first.generation < second.generation);
    fixture.browser.close().await.unwrap();
}

#[cfg(feature = "close-notices")]
#[tokio::test]
async fn close_only_queue_overflow_fails_closed_and_drains_prefix() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectCloseNotices,
    )
    .await;
    fixture
        .incoming
        .send(created_frame(
            "browser@1",
            "BrowserContext",
            "context@1",
            json!({}),
        ))
        .unwrap();
    created_sequence(&fixture.browser, "context@1").await;

    // Act
    for index in 0..=MAX_QUEUED_NOTICES {
        let guid = format!("page@{index}");
        fixture
            .incoming
            .send(created_frame("context@1", "Page", &guid, json!({})))
            .unwrap();
        fixture
            .incoming
            .send(event_frame(&guid, "close", json!({})))
            .unwrap();
    }
    let terminal = timeout(Duration::from_secs(1), fixture.browser.wait_terminal())
        .await
        .unwrap();

    // Assert
    assert!(matches!(terminal, SupervisorState::Closed { failure, .. }
        if failure.error == SessionError::PageEventProjection(PageProjectionError::NoticeLimit)
        && failure.report.reason == CloseReason::ResourceLimit));
    for _ in 0..MAX_QUEUED_NOTICES {
        assert!(matches!(
            next_close_notice(&fixture.browser).await,
            CloseNotice::PageClosed(_)
        ));
    }
    assert!(matches!(fixture.browser.next_close_notice().await,
        Err(SupervisorError::Terminal(failure))
        if failure.error == SessionError::PageEventProjection(PageProjectionError::NoticeLimit)));
}

#[cfg(feature = "close-notices")]
#[tokio::test]
async fn cancelled_close_notice_waiter_does_not_consume_next_notice() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectCloseNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    created_sequence(&fixture.browser, "page@1").await;
    let browser = Arc::new(fixture.browser);
    let waiter = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move { browser.next_close_notice().await }
    });
    tokio::task::yield_now().await;

    // Act
    waiter.abort();
    let _ = waiter.await;
    fixture
        .incoming
        .send(event_frame("page@1", "close", json!({})))
        .unwrap();
    let notice = next_close_notice(&browser).await;

    // Assert
    assert!(matches!(notice, CloseNotice::PageClosed(id) if id.guid == "page@1"));
    browser.close().await.unwrap();
}

#[cfg(feature = "close-notices")]
#[tokio::test]
async fn aborted_actor_preserves_queued_close_before_terminal_children() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectCloseNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    created_sequence(&fixture.browser, "page@1").await;
    fixture
        .incoming
        .send(event_frame("page@1", "close", json!({})))
        .unwrap();
    timeout(Duration::from_secs(1), async {
        while !fixture
            .browser
            .object_with_close_state("page@1")
            .is_some_and(|(_, closed)| closed)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    // Act
    fixture.browser.actor_abort.abort();
    let terminal = timeout(Duration::from_secs(1), fixture.browser.wait_terminal())
        .await
        .unwrap();
    let page = next_close_notice(&fixture.browser).await;
    let context = next_close_notice(&fixture.browser).await;
    let browser = next_close_notice(&fixture.browser).await;
    let disconnected = next_close_notice(&fixture.browser).await;

    // Assert
    assert_eq!(terminal, SupervisorState::Stopped);
    assert!(matches!(page, CloseNotice::PageClosed(id) if id.guid == "page@1"));
    assert!(matches!(context, CloseNotice::ContextClosed(id) if id.guid == "context@1"));
    assert!(matches!(browser, CloseNotice::BrowserClosed(id) if id.guid == "browser@1"));
    assert_eq!(disconnected, CloseNotice::Disconnected);
    assert_eq!(
        fixture.browser.next_close_notice().await,
        Err(SupervisorError::ActorStopped)
    );
}

#[tokio::test]
async fn pre_reply_existing_context_is_observable_before_handle_is_returned() {
    // Arrange
    let mut frames = success_frames();
    frames.insert(
        7,
        text(json!({"guid":"browser@1","method":"__create__","params":{
            "type":"BrowserContext","guid":"context@1","initializer":{}
        }})),
    );
    let fixture = fixture_with_frames(frames).await;
    assert_eq!(fixture.browser.bootstrap_event_count(), 8);
    let context = fixture.browser.object("context@1").unwrap();
    assert_eq!(context.parent.as_deref(), Some("browser@1"));
    assert_eq!(context.object_type, "BrowserContext");
    assert_eq!(context.created_sequence, 8);
    assert_eq!(
        fixture.browser.object("browser@1").unwrap().children,
        ["context@1"]
    );

    // Act
    let mut replayed = Vec::new();
    for _ in 0..8 {
        replayed.push(fixture.browser.next_event().await.unwrap());
    }

    // Assert
    assert!(
        matches!(&replayed[7], Ingress::Created { parent, guid, object_type, .. }
        if parent == "browser@1" && guid == "context@1" && object_type == "BrowserContext")
    );
    assert!(fixture.browser.is_connected());
    fixture.browser.close().await.unwrap();
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn created_child_is_projected_before_its_command_reply_is_published() {
    // Arrange
    let mut fixture = fixture().await;
    let browser = Arc::new(fixture.browser);
    let waiter = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call("browser@1", "newContext", json!({}), json!({}))
                .await
        }
    });
    let command = next_command(&mut fixture.sent).await;
    let command_id = command["id"].as_u64().unwrap();

    // Act
    fixture
        .incoming
        .send(text(
            json!({"guid":"browser@1","method":"__create__","params":{
                "type":"BrowserContext","guid":"context@reply","initializer":{"isChromium":true}
            }}),
        ))
        .unwrap();
    fixture
        .incoming
        .send(text(
            json!({"id":command_id,"result":{"context":{"guid":"context@reply"}}}),
        ))
        .unwrap();
    let reply = waiter.await.unwrap().unwrap();
    let context = browser.object("context@reply").unwrap();
    let snapshot = browser.snapshot();

    // Assert
    assert_eq!(
        reply,
        ReplyOutcome::Result(json!({"context":{"guid":"context@reply"}}))
    );
    assert_eq!(context.parent.as_deref(), Some("browser@1"));
    assert_eq!(context.object_type, "BrowserContext");
    assert_eq!(context.initializer["isChromium"], true);
    assert_eq!(snapshot.objects.last().unwrap().guid, "context@reply");
    assert_eq!(snapshot.last_created_sequence, context.created_sequence);
    assert_eq!(browser.last_created_sequence(), context.created_sequence);
    assert!(snapshot.retained_bytes <= MAX_SUPERVISOR_OBJECT_BYTES);
    assert!(
        matches!(browser.next_event().await.unwrap(), Ingress::Created { guid, .. } if guid == "context@reply")
    );
    browser.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_prechecked_close_cannot_reach_an_observed_successor_generation() {
    // Arrange
    let (received_sender, mut received) = mpsc::unbounded_channel();
    let mut fixture = fixture_with_policy_and_receipts(
        success_frames(),
        256 * 1024,
        EventPolicy::QueueBounded,
        Some(received_sender),
    )
    .await;
    for _ in 0..8 {
        received.recv().await.unwrap();
    }
    for _ in 0..7 {
        fixture.browser.next_event().await.unwrap();
    }
    let browser = Arc::new(fixture.browser);
    fixture
        .incoming
        .send(text(
            json!({"guid":"browser@1","method":"__create__","params":{
                "type":"BrowserContext","guid":"context@reused","initializer":{}
            }}),
        ))
        .unwrap();
    browser.next_event().await.unwrap();
    received.recv().await.unwrap();
    let prechecked = browser.object("context@reused").unwrap();
    assert_eq!(prechecked.object_type, "BrowserContext");
    assert_eq!(prechecked.parent.as_deref(), Some("browser@1"));
    fixture
        .incoming
        .send(text(
            json!({"guid":"context@reused","method":"__dispose__","params":{}}),
        ))
        .unwrap();
    browser.next_event().await.unwrap();
    received.recv().await.unwrap();

    // Hold projection while the actor receives the successor create. Queue
    // the close before releasing it, so the actor observes reuse first.
    let (locked_sender, locked_receiver) = tokio::sync::oneshot::channel();
    let (release_sender, release_receiver) = std::sync::mpsc::channel();
    let objects = Arc::clone(&browser.objects);
    let projection_reader = std::thread::spawn(move || {
        let _projection_read = objects.read().unwrap();
        locked_sender.send(()).unwrap();
        release_receiver.recv().unwrap();
    });
    timeout(Duration::from_secs(1), locked_receiver)
        .await
        .unwrap()
        .unwrap();
    fixture
        .incoming
        .send(text(
            json!({"guid":"browser@1","method":"__create__","params":{
                "type":"BrowserContext","guid":"context@reused","initializer":{}
            }}),
        ))
        .unwrap();
    timeout(Duration::from_secs(1), received.recv())
        .await
        .unwrap()
        .unwrap();
    let close = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call_for_generation(
                    "context@reused",
                    prechecked.created_sequence,
                    "close",
                    json!({}),
                    json!({}),
                )
                .await
        }
    });
    timeout(Duration::from_secs(1), async {
        while browser.commands.capacity() == MAX_SUPERVISOR_COMMANDS {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    // Act
    release_sender.send(()).unwrap();
    projection_reader.join().unwrap();
    let result = timeout(Duration::from_secs(1), close)
        .await
        .unwrap()
        .unwrap();

    // Assert
    assert_eq!(result, Err(SupervisorError::TargetGenerationChanged));
    assert!(
        browser.object("context@reused").unwrap().created_sequence > prechecked.created_sequence
    );
    assert!(
        timeout(Duration::from_millis(20), fixture.sent.recv())
            .await
            .is_err()
    );
    let unguarded = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call("browser@1", "newContext", json!({}), json!({}))
                .await
        }
    });
    let next = next_command(&mut fixture.sent).await;
    assert_eq!(next["id"], 2);
    fixture
        .incoming
        .send(text(json!({"id":2,"result":{}})))
        .unwrap();
    assert_eq!(
        unguarded.await.unwrap(),
        Ok(ReplyOutcome::Result(json!({})))
    );
    browser.close().await.unwrap();
}

#[tokio::test]
async fn already_stale_guarded_target_sends_no_frame() {
    // Arrange
    let mut fixture = fixture().await;
    let browser_generation = fixture
        .browser
        .object("browser@1")
        .unwrap()
        .created_sequence;

    // Act
    let result = fixture
        .browser
        .call_for_generation(
            "browser@1",
            browser_generation + 1,
            "newContext",
            json!({}),
            json!({}),
        )
        .await;

    // Assert
    assert_eq!(result, Err(SupervisorError::TargetGenerationChanged));
    assert!(
        timeout(Duration::from_millis(20), fixture.sent.recv())
            .await
            .is_err()
    );
    fixture.browser.close().await.unwrap();
}

#[tokio::test]
async fn disposal_removes_descendants_and_adoption_updates_parent_snapshots() {
    // Arrange
    let fixture = fixture().await;
    for (parent, object_type, guid) in [
        ("browser@1", "BrowserContext", "context@one"),
        ("browser@1", "BrowserContext", "context@two"),
        ("context@one", "Page", "page@one"),
        ("page@one", "Frame", "frame@one"),
    ] {
        fixture
            .incoming
            .send(text(json!({"guid":parent,"method":"__create__","params":{
                "type":object_type,"guid":guid,"initializer":{}
            }})))
            .unwrap();
        assert!(matches!(
            fixture.browser.next_event().await.unwrap(),
            Ingress::Created { .. }
        ));
    }
    assert_eq!(
        fixture
            .browser
            .object("frame@one")
            .unwrap()
            .parent
            .as_deref(),
        Some("page@one")
    );

    // Act
    fixture
        .incoming
        .send(text(
            json!({"guid":"context@two","method":"__adopt__","params":{"guid":"page@one"}}),
        ))
        .unwrap();
    assert!(matches!(
        fixture.browser.next_event().await.unwrap(),
        Ingress::Adopted { .. }
    ));
    let adopted = fixture.browser.object("page@one").unwrap();
    fixture
        .incoming
        .send(text(
            json!({"guid":"context@two","method":"__dispose__","params":{}}),
        ))
        .unwrap();
    let disposed = fixture.browser.next_event().await.unwrap();

    // Assert
    assert_eq!(adopted.parent.as_deref(), Some("context@two"));
    assert_eq!(
        fixture.browser.object("context@one").unwrap().children,
        Vec::<String>::new()
    );
    assert!(matches!(disposed, Ingress::Disposed { guids, .. }
        if guids == ["context@two", "page@one", "frame@one"]));
    assert!(fixture.browser.object("context@two").is_none());
    assert!(fixture.browser.object("page@one").is_none());
    assert!(fixture.browser.object("frame@one").is_none());
    assert!(fixture.browser.object("context@one").is_some());
    assert_eq!(fixture.browser.last_created_sequence(), 11);
    fixture.browser.close().await.unwrap();
}

#[tokio::test]
async fn retained_initializer_byte_budget_fails_closed_without_publishing_rejected_object() {
    // Arrange
    let fixture = fixture_with_frames_and_inbound_limit(success_frames(), MAX_MESSAGE_BYTES).await;
    let browser = fixture.browser;
    for _ in 0..7 {
        assert!(matches!(
            browser.next_event().await.unwrap(),
            Ingress::Created { .. }
        ));
    }
    let payload = "x".repeat(128 * 1024);
    let mut rejected_guid = None;

    // Act
    for index in 0..=MAX_SUPERVISOR_OBJECT_BYTES / payload.len() {
        let guid = format!("context@{index}");
        fixture
            .incoming
            .send(text(
                json!({"guid":"browser@1","method":"__create__","params":{
                    "type":"BrowserContext","guid":guid,"initializer":{"payload":payload}
                }}),
            ))
            .unwrap();
        match browser.next_event().await {
            Ok(Ingress::Created { guid: created, .. }) => assert_eq!(created, guid),
            Err(SupervisorError::Terminal(_)) => {
                rejected_guid = Some(guid);
                break;
            }
            other => panic!("unexpected projection outcome: {other:?}"),
        }
    }
    let terminal = timeout(Duration::from_secs(2), browser.wait_terminal())
        .await
        .unwrap();

    // Assert
    let rejected_guid = rejected_guid.expect("retained initializer limit must reject a GUID");
    assert!(matches!(terminal, SupervisorState::Closed { failure, .. }
        if failure.error == SessionError::ObjectGraphLimit
        && failure.report.reason == CloseReason::ResourceLimit));
    assert!(browser.object(&rejected_guid).is_none());
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[test]
fn projection_count_is_bounded_independently_of_initializer_bytes() {
    // Arrange
    let mut projection = ObjectProjection::new();
    for index in 1..MAX_SUPERVISOR_OBJECTS {
        projection
            .create("", &format!("object@{index}"), "Unit", &json!({}))
            .unwrap();
    }
    assert_eq!(projection.objects.len(), MAX_SUPERVISOR_OBJECTS);
    assert!(projection.retained_bytes < MAX_SUPERVISOR_OBJECT_BYTES);

    // Act
    let overflow = projection.create("", "one-more", "Unit", &json!({}));

    // Assert
    assert!(matches!(overflow, Err(ProjectionError::Limit)));
    assert!(projection.object("one-more").is_none());
}

#[tokio::test]
async fn bootstrap_event_overflow_fails_before_publishing_a_supervisor() {
    // Arrange
    let mut frames = success_frames();
    for index in 0..=MAX_SUPERVISOR_EVENTS {
        frames.insert(
            7 + index,
            text(json!({"guid":"browser@1","method":"burst","params":{"index":index}})),
        );
    }
    let (incoming, receiver) = mpsc::unbounded_channel();
    let (sent, _sent_receiver) = mpsc::unbounded_channel();
    let close_count = Arc::new(AtomicUsize::new(0));
    let connector = FakeConnector {
        incoming: Mutex::new(Some(receiver)),
        sent,
        close_count: Arc::clone(&close_count),
        received: None,
    };
    for frame in frames {
        incoming.send(frame).unwrap();
    }
    let request = WsRequest::new(
        WsEndpoint::parse("ws://example.test/ws").unwrap(),
        WsLimits::new(256 * 1024, 64 * 1024, 1000).unwrap(),
    );
    let lease = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Javascript, 123)
        .await
        .unwrap();

    // Act
    let result = RemoteBrowserSupervisor::start(lease).await;

    // Assert
    assert!(matches!(result, Err(failure)
        if failure.error == SessionError::EventQueueFull
        && failure.report.reason == CloseReason::ResourceLimit));
    assert_eq!(close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn command_admission_is_bounded_even_when_all_replies_are_pending() {
    // Arrange
    let mut fixture = fixture().await;
    let browser = Arc::new(fixture.browser);
    let mut waiters = Vec::new();
    for _ in 0..MAX_SUPERVISOR_COMMANDS {
        waiters.push(tokio::spawn({
            let browser = Arc::clone(&browser);
            async move {
                browser
                    .call("browser@1", "method", json!({}), json!({}))
                    .await
            }
        }));
        next_command(&mut fixture.sent).await;
    }

    // Act
    let over_limit = browser
        .call("browser@1", "method", json!({}), json!({}))
        .await;
    let outcome = browser.close().await.unwrap();

    // Assert
    assert_eq!(
        over_limit,
        Err(SupervisorError::Client(ClientError::PendingLimit))
    );
    assert_eq!(outcome.report.failed.len(), MAX_SUPERVISOR_COMMANDS);
    for waiter in waiters {
        assert!(matches!(
            waiter.await.unwrap(),
            Err(SupervisorError::Terminal(_))
        ));
    }
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn transport_send_failure_rejects_current_and_future_commands() {
    // Arrange
    let Fixture {
        browser,
        incoming: _incoming,
        sent,
        close_count,
    } = fixture().await;
    drop(sent);

    // Act
    let current = browser
        .call("browser@1", "method", json!({}), json!({}))
        .await;
    let future = browser
        .call("browser@1", "method", json!({}), json!({}))
        .await;
    let close = browser.close().await.unwrap();

    // Assert
    assert!(matches!(current, Err(SupervisorError::Terminal(failure))
        if failure.error == SessionError::WebSocket(WsError::IoFailed)));
    assert!(matches!(future, Err(SupervisorError::Terminal(failure))
        if failure.error == SessionError::WebSocket(WsError::IoFailed)));
    assert_eq!(close.report.reason, CloseReason::Transport);
    assert!(!browser.is_connected());
    assert_eq!(close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn send_failure_publishes_terminal_before_current_command_error() {
    // Arrange
    let Fixture {
        browser,
        incoming: _incoming,
        sent,
        close_count,
    } = fixture().await;
    drop(sent);

    // Act
    let error = browser
        .call("browser@1", "method", json!({}), json!({}))
        .await
        .unwrap_err();
    let state_at_error = browser.state();

    // Assert
    assert!(matches!(error, SupervisorError::Terminal(failure)
        if failure.error == SessionError::WebSocket(WsError::IoFailed)));
    assert!(
        matches!(state_at_error, SupervisorState::Closed { failure, .. }
        if failure.error == SessionError::WebSocket(WsError::IoFailed))
    );
    assert!(!browser.is_connected());
    assert_eq!(close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn aborted_actor_publishes_stopped_state_and_rejects_pending_calls() {
    // Arrange
    let mut fixture = fixture().await;
    let browser = Arc::new(fixture.browser);
    let waiter = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call("browser@1", "method", json!({}), json!({}))
                .await
        }
    });
    next_command(&mut fixture.sent).await;

    // Act
    browser.actor_abort.abort();
    let pending_result = timeout(Duration::from_secs(1), waiter)
        .await
        .unwrap()
        .unwrap();
    let event_result = browser.next_event().await;
    let close_result = browser.close().await;

    // Assert
    assert_eq!(pending_result, Err(SupervisorError::ActorStopped));
    assert_eq!(event_result, Err(SupervisorError::ActorStopped));
    assert_eq!(close_result, Err(SupervisorError::ActorStopped));
    assert_eq!(browser.state(), SupervisorState::Stopped);
    assert_eq!(browser.wait_terminal().await, SupervisorState::Stopped);
    assert!(!browser.is_connected());
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn concurrent_commands_correlate_out_of_order_replies_and_preserve_event_order() {
    // Arrange
    let mut fixture = fixture().await;
    let browser = Arc::new(fixture.browser);
    let first = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call("browser@1", "first", json!({}), json!({}))
                .await
        }
    });
    let first_command = next_command(&mut fixture.sent).await;
    let second = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call("browser@1", "second", json!({}), json!({}))
                .await
        }
    });
    let second_command = next_command(&mut fixture.sent).await;
    let first_id = first_command["id"].as_u64().unwrap();
    let second_id = second_command["id"].as_u64().unwrap();

    // Act
    fixture
        .incoming
        .send(text(
            json!({"guid":"browser@1","method":"one","params":{"order":1}}),
        ))
        .unwrap();
    fixture
        .incoming
        .send(text(
            json!({"guid":"browser@1","method":"two","params":{"order":2}}),
        ))
        .unwrap();
    fixture
        .incoming
        .send(text(json!({"id":second_id,"result":{"value":2}})))
        .unwrap();
    fixture
        .incoming
        .send(text(json!({"id":first_id,"result":{"value":1}})))
        .unwrap();
    let first_event = browser.next_event().await.unwrap();
    let second_event = browser.next_event().await.unwrap();
    let first_reply = first.await.unwrap().unwrap();
    let second_reply = second.await.unwrap().unwrap();

    // Assert
    assert_eq!((first_id, second_id), (2, 3));
    assert!(matches!(first_event, Ingress::Event { method, .. } if method == "one"));
    assert!(matches!(second_event, Ingress::Event { method, .. } if method == "two"));
    assert_eq!(first_reply, ReplyOutcome::Result(json!({"value":1})));
    assert_eq!(second_reply, ReplyOutcome::Result(json!({"value":2})));
    assert!(browser.is_connected());
    browser.close().await.unwrap();
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn local_close_during_pending_receive_rejects_command_and_is_idempotent() {
    // Arrange
    let mut fixture = fixture().await;
    let browser = Arc::new(fixture.browser);
    let waiter = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call("browser@1", "newContext", json!({}), json!({}))
                .await
        }
    });
    let command = next_command(&mut fixture.sent).await;

    // Act
    let first = timeout(Duration::from_secs(1), browser.close())
        .await
        .unwrap()
        .unwrap();
    let rejected = waiter.await.unwrap().unwrap_err();
    let second = browser.close().await.unwrap();

    // Assert
    assert_eq!(first, second);
    assert_eq!(first.report.failed.len(), 1);
    assert_eq!(
        first.report.failed[0].id.get(),
        command["id"].as_u64().unwrap()
    );
    assert_eq!(first.report.reason, CloseReason::Transport);
    assert!(matches!(rejected, SupervisorError::Terminal(failure)
        if failure.error == SessionError::BrowserClosed));
    assert!(!browser.is_connected());
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn remote_close_settles_pending_command_and_closes_event_stream() {
    // Arrange
    let mut fixture = fixture().await;
    let browser = Arc::new(fixture.browser);
    let waiter = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call("browser@1", "newContext", json!({}), json!({}))
                .await
        }
    });
    let command = next_command(&mut fixture.sent).await;
    let frame = WsCloseFrame::new(4123, "Oh my!").unwrap();

    // Act
    fixture
        .incoming
        .send(WsIncoming::RemoteClose(Some(frame.clone())))
        .unwrap();
    let rejected = waiter.await.unwrap().unwrap_err();
    let stream_end = browser.next_event().await.unwrap_err();
    let close = browser.close().await.unwrap();

    // Assert
    assert!(matches!(rejected, SupervisorError::Terminal(failure)
        if failure.error == SessionError::RemoteClose(Some(frame.clone()))
        && failure.report.failed[0].id.get() == command["id"].as_u64().unwrap()));
    assert!(matches!(stream_end, SupervisorError::Terminal(failure)
        if failure.error == SessionError::RemoteClose(Some(frame))));
    assert_eq!(close.report.failed.len(), 1);
    assert!(!browser.is_connected());
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn remote_close_publishes_terminal_before_pending_command_error() {
    // Arrange
    let mut fixture = fixture().await;
    let browser = Arc::new(fixture.browser);
    let pending = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            let error = browser
                .call("browser@1", "newContext", json!({}), json!({}))
                .await
                .unwrap_err();
            (error, browser.state())
        }
    });
    next_command(&mut fixture.sent).await;

    // Act
    fixture
        .incoming
        .send(WsIncoming::RemoteClose(Some(
            WsCloseFrame::new(4123, "Oh my!").unwrap(),
        )))
        .unwrap();
    let (error, state_at_error) = timeout(Duration::from_secs(1), pending)
        .await
        .unwrap()
        .unwrap();

    // Assert
    assert!(matches!(error, SupervisorError::Terminal(failure)
        if failure.error == SessionError::RemoteClose(Some(WsCloseFrame::new(4123, "Oh my!").unwrap()))));
    assert!(
        matches!(state_at_error, SupervisorState::Closed { failure, .. }
        if failure.error == SessionError::RemoteClose(Some(WsCloseFrame::new(4123, "Oh my!").unwrap())))
    );
    assert!(!browser.is_connected());
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn independent_terminal_waiters_observe_the_same_remote_close() {
    // Arrange
    let fixture = fixture().await;
    let browser = Arc::new(fixture.browser);
    assert_eq!(browser.bootstrap_event_count(), 7);
    let first = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move { browser.wait_terminal().await }
    });
    let second = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move { browser.wait_terminal().await }
    });

    // Act
    fixture
        .incoming
        .send(WsIncoming::RemoteClose(Some(
            WsCloseFrame::new(4123, "Oh my!").unwrap(),
        )))
        .unwrap();
    let first_state = timeout(Duration::from_secs(1), first)
        .await
        .unwrap()
        .unwrap();
    let second_state = timeout(Duration::from_secs(1), second)
        .await
        .unwrap()
        .unwrap();

    // Assert
    assert_eq!(first_state, second_state);
    assert!(
        matches!(first_state, SupervisorState::Closed { failure, .. }
        if failure.error == SessionError::RemoteClose(Some(WsCloseFrame::new(4123, "Oh my!").unwrap())))
    );
    assert_eq!(browser.wait_terminal().await, second_state);
    assert!(!browser.is_connected());
}

#[tokio::test]
async fn terminal_watcher_does_not_keep_the_socket_owner_alive() {
    // Arrange
    let fixture = fixture().await;
    let mut watcher = fixture.browser.terminal_watcher();
    assert_eq!(watcher.state(), SupervisorState::Connected);

    // Act
    drop(fixture.browser);
    let terminal = timeout(Duration::from_secs(1), watcher.wait_terminal())
        .await
        .unwrap();

    // Assert
    assert!(matches!(terminal, SupervisorState::Closed { .. }));
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn drop_unsupported_events_preserves_live_graph_after_an_unconsumed_burst() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::DropUnsupportedEvents,
    )
    .await;
    let browser = &fixture.browser;
    assert_eq!(browser.bootstrap_event_count(), 0);
    assert_eq!(
        browser.next_event().await,
        Err(SupervisorError::EventStreamDisabled)
    );

    // Act
    for index in 0..=MAX_SUPERVISOR_EVENTS {
        fixture
            .incoming
            .send(text(json!({
                "guid":"browser@1","method":"burst","params":{"index":index}
            })))
            .unwrap();
    }
    fixture
        .incoming
        .send(text(json!({
            "guid":"browser@1","method":"__create__","params":{
                "type":"BrowserContext","guid":"context@kept","initializer":{}
            }
        })))
        .unwrap();
    fixture
        .incoming
        .send(text(json!({
            "guid":"context@kept","method":"__create__","params":{
                "type":"Page","guid":"page@disposed","initializer":{}
            }
        })))
        .unwrap();
    fixture
        .incoming
        .send(text(json!({
            "guid":"page@disposed","method":"__dispose__","params":{}
        })))
        .unwrap();
    fixture
        .incoming
        .send(text(json!({
            "guid":"browser@1","method":"__create__","params":{
                "type":"BrowserContext","guid":"context@marker","initializer":{}
            }
        })))
        .unwrap();
    timeout(Duration::from_secs(1), async {
        while browser.object("context@marker").is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    // Assert
    assert!(browser.is_connected());
    assert_eq!(
        browser.object("context@kept").unwrap().object_type,
        "BrowserContext"
    );
    assert!(browser.object("page@disposed").is_none());
    assert_eq!(browser.last_created_sequence(), 10);
    assert_eq!(
        browser.next_event().await,
        Err(SupervisorError::EventStreamDisabled)
    );
    browser.close().await.unwrap();
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn drop_unsupported_events_still_fails_on_graph_byte_limit() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        MAX_MESSAGE_BYTES,
        EventPolicy::DropUnsupportedEvents,
    )
    .await;
    let browser = &fixture.browser;
    let payload = "x".repeat(128 * 1024);

    // Act
    for index in 0..=MAX_SUPERVISOR_OBJECT_BYTES / payload.len() {
        fixture
            .incoming
            .send(text(json!({
                "guid":"browser@1","method":"__create__","params":{
                    "type":"BrowserContext","guid":format!("context@{index}"),
                    "initializer":{"payload":payload}
                }
            })))
            .unwrap();
    }
    let terminal = timeout(Duration::from_secs(2), browser.wait_terminal())
        .await
        .unwrap();

    // Assert
    assert!(matches!(terminal, SupervisorState::Closed { failure, .. }
        if failure.error == SessionError::ObjectGraphLimit
        && failure.report.reason == CloseReason::ResourceLimit));
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn full_event_queue_fails_closed_without_blocking_pending_command_or_close() {
    // Arrange
    let mut fixture = fixture().await;
    let browser = Arc::new(fixture.browser);
    let waiter = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call("browser@1", "newContext", json!({}), json!({}))
                .await
        }
    });
    next_command(&mut fixture.sent).await;

    // Act
    for index in 0..=MAX_SUPERVISOR_EVENTS {
        fixture
            .incoming
            .send(text(
                json!({"guid":"browser@1","method":"burst","params":{"index":index}}),
            ))
            .unwrap();
    }
    let rejected = timeout(Duration::from_secs(1), waiter)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    let close = timeout(Duration::from_secs(1), browser.close())
        .await
        .unwrap()
        .unwrap();

    // Assert
    assert!(matches!(rejected, SupervisorError::Terminal(failure)
        if failure.error == SessionError::EventQueueFull
        && failure.report.reason == CloseReason::ResourceLimit));
    assert_eq!(close.report.reason, CloseReason::ResourceLimit);
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
    for expected in 0..MAX_SUPERVISOR_EVENTS {
        assert!(
            matches!(browser.next_event().await.unwrap(), Ingress::Event { params, .. }
            if params["index"] == expected)
        );
    }
    assert!(
        matches!(browser.next_event().await, Err(SupervisorError::Terminal(failure))
        if failure.error == SessionError::EventQueueFull)
    );
}

#[tokio::test]
async fn event_byte_budget_fails_closed_before_entry_cap_and_settles_command() {
    // Arrange
    let mut fixture = fixture().await;
    let browser = Arc::new(fixture.browser);
    let waiter = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call("browser@1", "newContext", json!({}), json!({}))
                .await
        }
    });
    next_command(&mut fixture.sent).await;
    let payload = "x".repeat(64 * 1024);
    let burst_count = MAX_SUPERVISOR_EVENT_BYTES / payload.len() + 1;
    assert!(burst_count < MAX_SUPERVISOR_EVENTS);

    // Act
    for index in 0..burst_count {
        fixture
            .incoming
            .send(text(json!({
                "guid":"browser@1","method":"burst",
                "params":{"index":index,"payload":payload}
            })))
            .unwrap();
    }
    let rejected = timeout(Duration::from_secs(1), waiter)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    let close = browser.close().await.unwrap();
    let mut retained_events = 0;
    while browser.next_event().await.is_ok() {
        retained_events += 1;
    }

    // Assert
    assert!(matches!(rejected, SupervisorError::Terminal(failure)
        if failure.error == SessionError::EventQueueFull
        && failure.report.reason == CloseReason::ResourceLimit));
    assert_eq!(close.report.reason, CloseReason::ResourceLimit);
    assert!(retained_events > 0 && retained_events < MAX_SUPERVISOR_EVENTS);
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn taking_events_releases_byte_budget_for_later_events() {
    // Arrange
    let fixture = fixture().await;
    let payload = "x".repeat(64 * 1024);
    let cumulative_count = MAX_SUPERVISOR_EVENT_BYTES / payload.len() + 2;

    // Act
    let mut observed = Vec::new();
    for index in 0..cumulative_count {
        fixture
            .incoming
            .send(text(json!({
                "guid":"browser@1","method":"burst",
                "params":{"index":index,"payload":payload}
            })))
            .unwrap();
        let Ingress::Event { params, .. } = fixture.browser.next_event().await.unwrap() else {
            panic!("expected event");
        };
        observed.push(params["index"].as_u64());
    }

    // Assert
    assert_eq!(
        observed,
        (0..cumulative_count)
            .map(|index| Some(index as u64))
            .collect::<Vec<_>>()
    );
    assert!(fixture.browser.is_connected());
    fixture.browser.close().await.unwrap();
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn cancelled_caller_keeps_command_slot_until_reply_is_observed() {
    // Arrange
    let mut fixture = fixture().await;
    let browser = Arc::new(fixture.browser);
    let cancelled = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call("browser@1", "first", json!({}), json!({}))
                .await
        }
    });
    let first = next_command(&mut fixture.sent).await;

    // Act
    cancelled.abort();
    let _ = cancelled.await;
    fixture
        .incoming
        .send(text(json!({"id":first["id"],"result":{}})))
        .unwrap();
    let second = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call("browser@1", "second", json!({}), json!({}))
                .await
        }
    });
    let second_command = next_command(&mut fixture.sent).await;
    fixture
        .incoming
        .send(text(
            json!({"id":second_command["id"],"result":{"ok":true}}),
        ))
        .unwrap();
    let result = second.await.unwrap().unwrap();

    // Assert
    assert_eq!(second_command["id"], first["id"].as_u64().unwrap() + 1);
    assert_eq!(result, ReplyOutcome::Result(json!({"ok":true})));
    assert!(browser.is_connected());
    browser.close().await.unwrap();
}

#[tokio::test]
async fn invalid_or_oversized_command_is_rejected_before_reaching_the_actor() {
    // Arrange
    let mut fixture = fixture().await;
    let oversized = "x".repeat(65 * 1024);

    // Act
    let invalid = fixture
        .browser
        .call("browser@1", "newContext", Value::Null, json!({}))
        .await;
    let large = fixture
        .browser
        .call(
            "browser@1",
            "newContext",
            json!({"payload":oversized}),
            json!({}),
        )
        .await;

    // Assert
    assert_eq!(
        invalid,
        Err(SupervisorError::Client(ClientError::InvalidParams))
    );
    assert_eq!(
        large,
        Err(SupervisorError::Client(ClientError::OutboundTooLarge))
    );
    assert!(fixture.sent.try_recv().is_err());
    assert_eq!(fixture.browser.state(), SupervisorState::Connected);
    fixture.browser.close().await.unwrap();
}

#[cfg(feature = "page-events")]
#[tokio::test]
async fn projected_request_response_and_download_survive_wire_disposal() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectPageNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    let page_sequence = created_sequence(&fixture.browser, "page@1").await;
    for frame in [
        request_frame("request@1"),
        created_frame(
            "request@1",
            "Response",
            "response@1",
            json!({
                "request": {"guid": "request@1"},
                "url": "https://example.test/report",
                "status": 201,
                "statusText": "Created",
                "headers": [{"name": "Content-Type", "value": "application/json"}],
                "fromServiceWorker": false
            }),
        ),
        created_frame(
            "context@1",
            "Artifact",
            "artifact@1",
            json!({"absolutePath": "/private/server/report.csv"}),
        ),
        request_event("request@1", "page@1"),
        event_frame(
            "context@1",
            "response",
            json!({"response": {"guid": "response@1"}, "page": {"guid": "page@1"}}),
        ),
        event_frame(
            "page@1",
            "download",
            json!({
                "url": "https://example.test/report.csv",
                "suggestedFilename": "report.csv",
                "artifact": {"guid": "artifact@1"}
            }),
        ),
        event_frame("request@1", "__dispose__", json!({})),
        event_frame("page@1", "__dispose__", json!({})),
        created_frame("browser@1", "BrowserContext", "context@marker", json!({})),
    ] {
        fixture.incoming.send(frame).unwrap();
    }
    created_sequence(&fixture.browser, "context@marker").await;
    assert!(fixture.browser.object("request@1").is_none());
    assert!(fixture.browser.object("response@1").is_none());
    assert!(fixture.browser.object("page@1").is_none());

    // Act
    let request = next_page_notice(&fixture.browser).await;
    let response = next_page_notice(&fixture.browser).await;
    let download = next_page_notice(&fixture.browser).await;
    let close = next_page_notice(&fixture.browser).await;

    // Assert
    let PageNotice::Request {
        page,
        request,
        snapshot,
    } = request
    else {
        panic!("expected Request notice");
    };
    assert_eq!(page.guid, "page@1");
    assert_eq!(page.generation, page_sequence);
    assert_eq!(request.guid, "request@1");
    assert_eq!(snapshot.url, "https://example.test/report");
    assert_eq!(snapshot.method, "POST");
    assert_eq!(snapshot.resource_type, "fetch");
    assert_eq!(snapshot.headers[0].name, "Content-Type");
    let PageNotice::Response {
        page,
        response,
        snapshot,
        request_snapshot,
    } = response
    else {
        panic!("expected Response notice");
    };
    assert_eq!(page.generation, page_sequence);
    assert_eq!(response.guid, "response@1");
    assert_eq!(snapshot.status, 201);
    assert_eq!(snapshot.request, request);
    assert_eq!(request_snapshot.url, "https://example.test/report");
    let PageNotice::Download {
        page,
        artifact,
        url,
        suggested_filename,
    } = download
    else {
        panic!("expected Download notice");
    };
    assert_eq!(page.generation, page_sequence);
    assert_eq!(artifact.guid, "artifact@1");
    assert_eq!(url, "https://example.test/report.csv");
    assert_eq!(suggested_filename, "report.csv");
    assert!(matches!(close, PageNotice::PageClosed(page)
        if page.guid == "page@1" && page.generation == page_sequence));
    fixture.browser.close().await.unwrap();
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "page-events")]
#[tokio::test]
async fn projected_request_guid_reuse_has_a_new_generation() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectPageNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    for frame in [
        request_frame("request@reused"),
        request_event("request@reused", "page@1"),
        event_frame("request@reused", "__dispose__", json!({})),
        request_frame("request@reused"),
        request_event("request@reused", "page@1"),
        created_frame("browser@1", "BrowserContext", "context@marker", json!({})),
    ] {
        fixture.incoming.send(frame).unwrap();
    }
    created_sequence(&fixture.browser, "context@marker").await;
    let live_generation = fixture
        .browser
        .object("request@reused")
        .unwrap()
        .created_sequence;

    // Act
    let first = next_page_notice(&fixture.browser).await;
    let second = next_page_notice(&fixture.browser).await;

    // Assert
    let PageNotice::Request { request: first, .. } = first else {
        panic!("expected first Request notice");
    };
    let PageNotice::Request {
        request: second, ..
    } = second
    else {
        panic!("expected second Request notice");
    };
    assert_eq!(first.guid, second.guid);
    assert!(first.generation < second.generation);
    assert_eq!(second.generation, live_generation);
    fixture.browser.close().await.unwrap();
}

#[cfg(feature = "page-events")]
#[tokio::test]
async fn projected_bootstrap_replays_download_after_unsupported_event_burst() {
    // Arrange
    let mut frames = success_frames();
    let mut early = page_graph_frames();
    early.push(created_frame(
        "context@1",
        "Artifact",
        "artifact@early",
        json!({}),
    ));
    for index in 0..160 {
        early.push(event_frame(
            "browser@1",
            "unsupportedNoise",
            json!({"index": index}),
        ));
    }
    early.push(event_frame(
        "page@1",
        "download",
        json!({
            "url": "https://example.test/early.csv",
            "suggestedFilename": "early.csv",
            "artifact": {"guid": "artifact@early"}
        }),
    ));
    frames.splice(7..7, early);

    // Act
    let fixture = fixture_with_policy(frames, 256 * 1024, EventPolicy::ProjectPageNotices).await;
    let notice = next_page_notice(&fixture.browser).await;

    // Assert
    assert_eq!(fixture.browser.bootstrap_event_count(), 0);
    assert_eq!(
        fixture.browser.next_event().await,
        Err(SupervisorError::EventStreamDisabled)
    );
    assert!(
        matches!(notice, PageNotice::Download { page, artifact, suggested_filename, .. }
        if page.guid == "page@1" && artifact.guid == "artifact@early"
        && suggested_filename == "early.csv")
    );
    assert!(fixture.browser.is_connected());
    fixture.browser.close().await.unwrap();
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "page-events")]
#[tokio::test]
async fn projected_live_unsupported_burst_does_not_fill_raw_event_queue() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectPageNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    fixture
        .incoming
        .send(created_frame(
            "context@1",
            "Artifact",
            "artifact@live",
            json!({}),
        ))
        .unwrap();

    // Act
    for index in 0..160 {
        fixture
            .incoming
            .send(event_frame(
                "browser@1",
                "unsupportedNoise",
                json!({"index": index}),
            ))
            .unwrap();
    }
    fixture
        .incoming
        .send(event_frame(
            "page@1",
            "download",
            json!({
                "url": "https://example.test/live.csv",
                "suggestedFilename": "live.csv",
                "artifact": {"guid": "artifact@live"}
            }),
        ))
        .unwrap();
    let notice = next_page_notice(&fixture.browser).await;

    // Assert
    assert!(
        matches!(notice, PageNotice::Download { artifact, suggested_filename, .. }
        if artifact.guid == "artifact@live" && suggested_filename == "live.csv")
    );
    assert!(fixture.browser.is_connected());
    assert_eq!(
        fixture.browser.next_event().await,
        Err(SupervisorError::EventStreamDisabled)
    );
    fixture.browser.close().await.unwrap();
}

#[cfg(feature = "page-events")]
#[tokio::test]
async fn projected_concurrent_waiters_each_receive_one_idle_wakeup() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectPageNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    fixture.incoming.send(request_frame("request@1")).unwrap();
    fixture.incoming.send(request_frame("request@2")).unwrap();
    created_sequence(&fixture.browser, "request@2").await;
    let browser = Arc::new(fixture.browser);
    let mut first = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move { browser.next_page_notice().await }
    });
    let mut second = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move { browser.next_page_notice().await }
    });
    tokio::task::yield_now().await;

    // Act
    fixture
        .incoming
        .send(request_event("request@1", "page@1"))
        .unwrap();
    let (first_notice, other_is_second) = timeout(Duration::from_secs(1), async {
        tokio::select! {
            result = &mut first => (result.unwrap().unwrap(), true),
            result = &mut second => (result.unwrap().unwrap(), false),
        }
    })
    .await
    .unwrap();
    let remaining = if other_is_second {
        &mut second
    } else {
        &mut first
    };
    assert!(!remaining.is_finished());
    fixture
        .incoming
        .send(request_event("request@2", "page@1"))
        .unwrap();
    let second_notice = timeout(Duration::from_secs(1), remaining)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    // Assert
    assert!(matches!(first_notice, PageNotice::Request { request, .. }
        if request.guid == "request@1"));
    assert!(matches!(second_notice, PageNotice::Request { request, .. }
        if request.guid == "request@2"));
    assert!(browser.is_connected());
    browser.close().await.unwrap();
}

#[cfg(feature = "page-events")]
#[tokio::test]
async fn projected_terminal_closes_page_then_context_then_browser_once() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectPageNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    let page_sequence = created_sequence(&fixture.browser, "page@1").await;

    // Act
    for frame in [
        event_frame("page@1", "close", json!({})),
        event_frame("context@1", "close", json!({})),
        event_frame("context@1", "__dispose__", json!({})),
        // The lease consumes Browser.close and reports a terminal instead of Ingress.
        event_frame("browser@1", "close", json!({})),
    ] {
        fixture.incoming.send(frame).unwrap();
    }
    let page = next_page_notice(&fixture.browser).await;
    let context = next_page_notice(&fixture.browser).await;
    let browser = next_page_notice(&fixture.browser).await;
    let disconnected = next_page_notice(&fixture.browser).await;
    let terminal = timeout(Duration::from_secs(1), fixture.browser.wait_terminal())
        .await
        .unwrap();

    // Assert
    assert!(matches!(page, PageNotice::PageClosed(page)
        if page.guid == "page@1" && page.generation == page_sequence));
    assert!(matches!(context, PageNotice::ContextClosed(context)
        if context.guid == "context@1"));
    assert!(matches!(browser, PageNotice::BrowserClosed(browser)
        if browser.guid == "browser@1"));
    assert!(matches!(disconnected, PageNotice::Disconnected));
    assert!(matches!(terminal, SupervisorState::Closed { failure, .. }
        if failure.error == SessionError::BrowserClosed));
    assert!(matches!(fixture.browser.next_page_notice().await,
        Err(SupervisorError::Terminal(failure))
        if failure.error == SessionError::BrowserClosed));
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "page-events")]
#[tokio::test]
async fn pending_error_observes_terminal_after_ordered_child_notices_are_finalized() {
    // Arrange
    let mut fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectPageNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    let page_sequence = created_sequence(&fixture.browser, "page@1").await;
    let browser = Arc::new(fixture.browser);
    let pending = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            let error = browser
                .call("browser@1", "newContext", json!({}), json!({}))
                .await
                .unwrap_err();
            (error, browser.state())
        }
    });
    next_command(&mut fixture.sent).await;

    // Act
    for frame in [
        event_frame("page@1", "close", json!({})),
        event_frame("context@1", "close", json!({})),
        event_frame("context@1", "__dispose__", json!({})),
        event_frame("browser@1", "close", json!({})),
    ] {
        fixture.incoming.send(frame).unwrap();
    }
    let (error, state_at_error) = timeout(Duration::from_secs(1), pending)
        .await
        .unwrap()
        .unwrap();
    let page = next_page_notice(&browser).await;
    let context = next_page_notice(&browser).await;
    let browser_notice = next_page_notice(&browser).await;
    let disconnected = next_page_notice(&browser).await;

    // Assert
    assert!(matches!(error, SupervisorError::Terminal(failure)
        if failure.error == SessionError::BrowserClosed));
    assert!(
        matches!(state_at_error, SupervisorState::Closed { failure, .. }
        if failure.error == SessionError::BrowserClosed)
    );
    assert!(matches!(page, PageNotice::PageClosed(page)
        if page.guid == "page@1" && page.generation == page_sequence));
    assert!(matches!(context, PageNotice::ContextClosed(context)
        if context.guid == "context@1"));
    assert!(matches!(browser_notice, PageNotice::BrowserClosed(browser)
        if browser.guid == "browser@1"));
    assert!(matches!(disconnected, PageNotice::Disconnected));
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "page-events")]
#[tokio::test]
async fn malformed_projected_page_lineage_fails_closed() {
    // Arrange
    let fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectPageNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    fixture
        .incoming
        .send(created_frame("context@1", "Page", "page@2", json!({})))
        .unwrap();
    fixture.incoming.send(request_frame("request@1")).unwrap();

    // Act
    fixture
        .incoming
        .send(request_event("request@1", "page@2"))
        .unwrap();
    let terminal = timeout(Duration::from_secs(1), fixture.browser.wait_terminal())
        .await
        .unwrap();

    // Assert
    assert!(matches!(terminal, SupervisorState::Closed { failure, .. }
        if failure.error == SessionError::PageEventProjection(
            PageProjectionError::Invalid("network Request Page"))
        && failure.report.reason == CloseReason::Protocol(
            pwright_playwright_server_client_core::ProtocolViolation::InvalidParams)));
    assert!(matches!(fixture.browser.next_page_notice().await,
        Err(SupervisorError::Terminal(failure))
        if failure.error == SessionError::PageEventProjection(
            PageProjectionError::Invalid("network Request Page"))));
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "page-events")]
#[tokio::test]
async fn projected_notice_overflow_rejects_pending_call_and_fails_closed() {
    // Arrange
    let mut fixture = fixture_with_policy(
        success_frames(),
        256 * 1024,
        EventPolicy::ProjectPageNotices,
    )
    .await;
    for frame in page_graph_frames() {
        fixture.incoming.send(frame).unwrap();
    }
    fixture.incoming.send(request_frame("request@1")).unwrap();
    let browser = Arc::new(fixture.browser);
    let pending = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call("browser@1", "newContext", json!({}), json!({}))
                .await
        }
    });
    next_command(&mut fixture.sent).await;

    // Act
    for _ in 0..=MAX_QUEUED_NOTICES {
        fixture
            .incoming
            .send(request_event("request@1", "page@1"))
            .unwrap();
    }
    let terminal = timeout(Duration::from_secs(1), browser.wait_terminal())
        .await
        .unwrap();
    let rejected = timeout(Duration::from_secs(1), pending)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();

    // Assert
    assert!(matches!(terminal, SupervisorState::Closed { failure, .. }
        if failure.error == SessionError::PageEventProjection(PageProjectionError::NoticeLimit)
        && failure.report.reason == CloseReason::ResourceLimit));
    assert!(matches!(rejected, SupervisorError::Terminal(failure)
        if failure.error == SessionError::PageEventProjection(PageProjectionError::NoticeLimit)));
    for _ in 0..MAX_QUEUED_NOTICES {
        assert!(matches!(next_page_notice(&browser).await,
            PageNotice::Request { page, request, .. }
            if page.guid == "page@1" && request.guid == "request@1"));
    }
    assert!(matches!(browser.next_page_notice().await,
        Err(SupervisorError::Terminal(failure))
        if failure.error == SessionError::PageEventProjection(PageProjectionError::NoticeLimit)));
    assert_eq!(fixture.close_count.load(Ordering::SeqCst), 1);
}
