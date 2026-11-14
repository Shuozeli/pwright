use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pwright_playwright_server_client_core::{Ingress, RemoteError};
use pwright_playwright_server_session::{
    EventPolicy, RemoteBrowserLease, SdkLanguage, SupervisorState,
};
use pwright_transport_ws::{
    WsConnector, WsEndpoint, WsError, WsIncoming, WsLimits, WsOutgoing, WsPeer, WsRequest,
};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio::time::timeout;

use super::*;

const DEADLINE: Duration = Duration::from_secs(1);

struct FakeConnector {
    incoming: Mutex<Option<mpsc::UnboundedReceiver<WsIncoming>>>,
    sent: mpsc::UnboundedSender<WsOutgoing>,
    closes: Arc<AtomicUsize>,
}

struct FakePeer {
    incoming: mpsc::UnboundedReceiver<WsIncoming>,
    sent: mpsc::UnboundedSender<WsOutgoing>,
    closes: Arc<AtomicUsize>,
}

impl WsConnector for FakeConnector {
    type Peer = FakePeer;
    type Future<'a> = Ready<Result<Self::Peer, WsError>>;

    fn connect(&self, _request: &WsRequest) -> Self::Future<'_> {
        ready(Ok(FakePeer {
            incoming: self.incoming.lock().unwrap().take().unwrap(),
            sent: self.sent.clone(),
            closes: Arc::clone(&self.closes),
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
            Ok(self
                .incoming
                .recv()
                .await
                .unwrap_or(WsIncoming::EndOfStream))
        })
    }

    fn close(&mut self) -> Self::CloseFuture<'_> {
        self.closes.fetch_add(1, Ordering::SeqCst);
        ready(Ok(()))
    }
}

fn text(value: Value) -> WsIncoming {
    WsIncoming::Text(value.to_string())
}

fn bootstrap() -> Vec<WsIncoming> {
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
    browser: RemoteBrowser,
    supervisor: Arc<RemoteBrowserSupervisor>,
    incoming: mpsc::UnboundedSender<WsIncoming>,
    sent: mpsc::UnboundedReceiver<WsOutgoing>,
    closes: Arc<AtomicUsize>,
}

impl Fixture {
    async fn new() -> Self {
        Self::with_policy(EventPolicy::QueueBounded).await
    }

    async fn with_policy(event_policy: EventPolicy) -> Self {
        let (incoming, receiver) = mpsc::unbounded_channel();
        let (sent, mut sent_receiver) = mpsc::unbounded_channel();
        let closes = Arc::new(AtomicUsize::new(0));
        let connector = FakeConnector {
            incoming: Mutex::new(Some(receiver)),
            sent,
            closes: Arc::clone(&closes),
        };
        for frame in bootstrap() {
            incoming.send(frame).unwrap();
        }
        let request = WsRequest::new(
            WsEndpoint::parse("ws://server.test/playwright").unwrap(),
            WsLimits::new(256 * 1024, 64 * 1024, 1000).unwrap(),
        );
        let lease = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 123)
            .await
            .unwrap();
        let supervisor = Arc::new(
            RemoteBrowserSupervisor::start_with_event_policy(lease, event_policy)
                .await
                .unwrap(),
        );
        let browser = RemoteBrowser::new(Arc::clone(&supervisor)).unwrap();
        let WsOutgoing::Text(initialize) = sent_receiver.recv().await.unwrap() else {
            panic!("expected Root.initialize text frame");
        };
        assert_eq!(serde_json::from_str::<Value>(&initialize).unwrap()["id"], 1);
        Self {
            browser,
            supervisor,
            incoming,
            sent: sent_receiver,
            closes,
        }
    }

    fn send(&self, value: Value) {
        self.incoming.send(text(value)).unwrap();
    }

    async fn command(&mut self) -> Value {
        let WsOutgoing::Text(command) = timeout(DEADLINE, self.sent.recv()).await.unwrap().unwrap()
        else {
            panic!("expected command text frame");
        };
        serde_json::from_str(&command).unwrap()
    }

    async fn close(&self) {
        self.supervisor.close().await.unwrap();
        assert_eq!(self.closes.load(Ordering::SeqCst), 1);
    }
}

fn metadata(api_name: &str) -> OperationMetadata {
    OperationMetadata::new(api_name, 456).unwrap()
}

async fn create_context(fixture: &mut Fixture, guid: &str) -> RemoteContext {
    let browser = fixture.browser.clone();
    let operation = tokio::spawn(async move {
        browser
            .new_context(EmptyContextOptions, &metadata("browser.new_context"))
            .await
    });
    assert_eq!(
        fixture.command().await,
        json!({"id":2,"guid":"browser@1","method":"newContext","params":{},"metadata":{"wallTime":456,"apiName":"browser.new_context","internal":false}})
    );
    fixture.send(json!({"guid":"browser@1","method":"__create__","params":{"type":"BrowserContext","guid":guid,"initializer":{}}}));
    fixture.send(json!({"id":2,"result":{"context":{"guid":guid}}}));
    timeout(DEADLINE, operation)
        .await
        .unwrap()
        .unwrap()
        .unwrap()
}

#[cfg(feature = "frame-title")]
#[derive(Clone, Copy)]
enum MainFrameOrder {
    BeforeCommand,
    BeforePage,
    AfterPage,
}

#[cfg(feature = "frame-title")]
async fn create_page_with_main_frame_layout(
    fixture: &mut Fixture,
    adopt: bool,
    order: MainFrameOrder,
    frame_type: &str,
    main_frame: Value,
) -> RemotePage {
    let context = create_context(fixture, "context@title").await;
    if matches!(order, MainFrameOrder::BeforeCommand) {
        fixture.send(json!({"guid":"context@title","method":"__create__","params":{"type":frame_type,"guid":"frame@main","initializer":{"url":"about:blank","name":"","loadStates":[]}}}));
        wait_until(|| fixture.supervisor.object("frame@main").is_some()).await;
    }
    let operation =
        tokio::spawn(async move { context.new_page(&metadata("browserContext.newPage")).await });
    let command = fixture.command().await;
    assert_eq!(command["guid"], "context@title");
    assert_eq!(command["method"], "newPage");
    if matches!(order, MainFrameOrder::BeforePage) {
        fixture.send(json!({"guid":"context@title","method":"__create__","params":{"type":frame_type,"guid":"frame@main","initializer":{"url":"about:blank","name":"","loadStates":[]}}}));
    }
    fixture.send(json!({"guid":"context@title","method":"__create__","params":{"type":"Page","guid":"page@title","initializer":{"mainFrame":main_frame,"isClosed":false}}}));
    if matches!(order, MainFrameOrder::AfterPage) {
        fixture.send(json!({"guid":"context@title","method":"__create__","params":{"type":frame_type,"guid":"frame@main","initializer":{"url":"about:blank","name":"","loadStates":[]}}}));
    }
    if adopt {
        fixture
            .send(json!({"guid":"page@title","method":"__adopt__","params":{"guid":"frame@main"}}));
    }
    fixture.send(json!({"id":command["id"],"result":{"page":{"guid":"page@title"}}}));
    timeout(DEADLINE, operation)
        .await
        .unwrap()
        .unwrap()
        .unwrap()
}

#[cfg(feature = "frame-title")]
async fn create_page_with_main_frame_order(
    fixture: &mut Fixture,
    adopt: bool,
    order: MainFrameOrder,
) -> RemotePage {
    create_page_with_main_frame_layout(fixture, adopt, order, "Frame", json!({"guid":"frame@main"}))
        .await
}

#[cfg(feature = "frame-title")]
async fn create_page_with_main_frame(fixture: &mut Fixture, adopt: bool) -> RemotePage {
    create_page_with_main_frame_order(fixture, adopt, MainFrameOrder::BeforePage).await
}

#[cfg(any(feature = "frame-title", feature = "close-state"))]
async fn wait_until(mut condition: impl FnMut() -> bool) {
    timeout(DEADLINE, async move {
        while !condition() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn title_uses_original_adopted_main_frame_and_exact_string_reply() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame(&mut fixture, true).await;
    assert_eq!(
        fixture.supervisor.object("frame@main").unwrap().parent,
        Some("page@title".to_owned())
    );
    let operation = tokio::spawn(async move { page.title(&metadata("page.title")).await });

    // Act
    let command = fixture.command().await;
    assert_eq!(
        command,
        json!({"id":4,"guid":"frame@main","method":"title","params":{},"metadata":{"wallTime":456,"apiName":"page.title","internal":false}})
    );
    fixture.send(json!({"id":4,"result":{"value":"Example"}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert_eq!(result.unwrap(), "Example");
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn title_rejects_main_frame_that_was_not_adopted_before_page_reply() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame(&mut fixture, false).await;

    // Act
    let result = page.title(&metadata("page.title")).await;

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "page has no adopted main Frame"
        ))
    ));
    assert!(fixture.sent.try_recv().is_err());
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn title_rejects_main_frame_created_before_new_page_command() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page =
        create_page_with_main_frame_order(&mut fixture, true, MainFrameOrder::BeforeCommand).await;

    // Act
    let result = page.title(&metadata("page.title")).await;

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "page has no adopted main Frame"
        ))
    ));
    assert!(fixture.sent.try_recv().is_err());
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn title_rejects_main_frame_created_after_page() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page =
        create_page_with_main_frame_order(&mut fixture, true, MainFrameOrder::AfterPage).await;

    // Act
    let result = page.title(&metadata("page.title")).await;

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "page has no adopted main Frame"
        ))
    ));
    assert!(fixture.sent.try_recv().is_err());
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn title_rejects_wrong_main_frame_object_type() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame_layout(
        &mut fixture,
        true,
        MainFrameOrder::BeforePage,
        "Worker",
        json!({"guid":"frame@main"}),
    )
    .await;

    // Act
    let result = page.title(&metadata("page.title")).await;

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "page has no adopted main Frame"
        ))
    ));
    assert!(fixture.sent.try_recv().is_err());
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn title_rejects_main_frame_channel_with_extra_fields() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame_layout(
        &mut fixture,
        true,
        MainFrameOrder::BeforePage,
        "Frame",
        json!({"guid":"frame@main","unexpected":true}),
    )
    .await;

    // Act
    let result = page.title(&metadata("page.title")).await;

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "page has no adopted main Frame"
        ))
    ));
    assert!(fixture.sent.try_recv().is_err());
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn title_rejects_recycled_main_frame_guid_before_sending() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame(&mut fixture, true).await;
    let original_sequence = fixture
        .supervisor
        .object("frame@main")
        .unwrap()
        .created_sequence;
    fixture.send(json!({"guid":"frame@main","method":"__dispose__","params":{}}));
    fixture.send(json!({"guid":"page@title","method":"__create__","params":{"type":"Frame","guid":"frame@main","initializer":{"url":"about:blank","name":"","loadStates":[]}}}));
    wait_until(|| {
        fixture
            .supervisor
            .object("frame@main")
            .is_some_and(|object| object.created_sequence != original_sequence)
    })
    .await;

    // Act
    let result = page.title(&metadata("page.title")).await;

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "main Frame GUID was recycled"
        ))
    ));
    assert!(fixture.sent.try_recv().is_err());
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn title_rejects_main_frame_adopted_away_before_sending() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame(&mut fixture, true).await;
    fixture
        .send(json!({"guid":"context@title","method":"__adopt__","params":{"guid":"frame@main"}}));
    wait_until(|| {
        fixture
            .supervisor
            .object("frame@main")
            .unwrap()
            .parent
            .as_deref()
            == Some("context@title")
    })
    .await;

    // Act
    let result = page.title(&metadata("page.title")).await;

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "target GUID has wrong parent"
        ))
    ));
    assert!(fixture.sent.try_recv().is_err());
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn title_rejects_main_frame_adopted_away_before_successful_reply() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame(&mut fixture, true).await;
    let operation = tokio::spawn(async move { page.title(&metadata("page.title")).await });
    let command = fixture.command().await;

    // Act
    fixture
        .send(json!({"guid":"context@title","method":"__adopt__","params":{"guid":"frame@main"}}));
    fixture.send(json!({"id":command["id"],"result":{"value":"Stale"}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::Supervisor(
            SupervisorError::TargetGenerationChanged
        ))
    ));
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn title_rejects_adoption_away_and_back_before_reply() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame(&mut fixture, true).await;
    let original_epoch = fixture
        .supervisor
        .object_with_frame_title_state("frame@main")
        .unwrap()
        .1;
    let operation = tokio::spawn(async move { page.title(&metadata("page.title")).await });
    let command = fixture.command().await;

    // Act
    fixture
        .send(json!({"guid":"context@title","method":"__adopt__","params":{"guid":"frame@main"}}));
    fixture.send(json!({"guid":"page@title","method":"__adopt__","params":{"guid":"frame@main"}}));
    wait_until(|| {
        fixture
            .supervisor
            .object_with_frame_title_state("frame@main")
            .unwrap()
            .1
            > original_epoch + 1
    })
    .await;
    fixture.send(json!({"id":command["id"],"result":{"value":"Stale"}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::Supervisor(
            SupervisorError::TargetGenerationChanged
        ))
    ));
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn title_rejects_context_reparenting_before_reply() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame(&mut fixture, true).await;
    let operation = tokio::spawn(async move { page.title(&metadata("page.title")).await });
    let command = fixture.command().await;

    // Act
    fixture.send(json!({"guid":"firefox@1","method":"__create__","params":{"type":"Browser","guid":"browser@other","initializer":{"name":"firefox","version":"155.0"}}}));
    fixture.send(
        json!({"guid":"browser@other","method":"__adopt__","params":{"guid":"context@title"}}),
    );
    wait_until(|| {
        fixture
            .supervisor
            .object("context@title")
            .is_some_and(|context| context.parent.as_deref() == Some("browser@other"))
    })
    .await;
    fixture.send(json!({"id":command["id"],"result":{"value":"Stale"}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::Supervisor(
            SupervisorError::TargetGenerationChanged
        ))
    ));
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn page_close_tombstone_rejects_title_before_send_and_retains_graph() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame(&mut fixture, true).await;
    fixture.send(json!({"guid":"page@title","method":"close","params":{}}));
    wait_until(|| {
        fixture
            .supervisor
            .object_with_frame_title_state("page@title")
            .is_some_and(|(_, _, closed)| closed)
    })
    .await;

    // Act
    let result = page.title(&metadata("page.title")).await;

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "target GUID is closed"
        ))
    ));
    assert!(!page.is_open());
    assert!(fixture.supervisor.object("page@title").is_some());
    assert!(fixture.sent.try_recv().is_err());
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn page_close_before_reply_rejects_title_at_actor_reply() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame(&mut fixture, true).await;
    let operation = tokio::spawn(async move { page.title(&metadata("page.title")).await });
    let command = fixture.command().await;

    // Act
    fixture.send(json!({"guid":"page@title","method":"close","params":{}}));
    wait_until(|| {
        fixture
            .supervisor
            .object_with_frame_title_state("page@title")
            .is_some_and(|(_, _, closed)| closed)
    })
    .await;
    fixture.send(json!({"id":command["id"],"result":{"value":"Stale"}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::Supervisor(
            SupervisorError::TargetGenerationChanged
        ))
    ));
    assert!(fixture.supervisor.object("page@title").is_some());
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn valid_title_reply_stays_valid_when_later_page_close_is_projected() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame(&mut fixture, true).await;
    let operation = tokio::spawn(async move { page.title(&metadata("page.title")).await });
    let command = fixture.command().await;

    // Act
    fixture.send(json!({"id":command["id"],"result":{"value":"Before close"}}));
    fixture.send(json!({"guid":"page@title","method":"close","params":{}}));
    wait_until(|| {
        fixture
            .supervisor
            .object_with_frame_title_state("page@title")
            .is_some_and(|(_, _, closed)| closed)
    })
    .await;
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert_eq!(result.unwrap(), "Before close");
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn valid_title_reply_stays_valid_when_later_disposal_removes_page() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame(&mut fixture, true).await;
    let operation = tokio::spawn(async move { page.title(&metadata("page.title")).await });
    let command = fixture.command().await;

    // Act
    fixture.send(json!({"id":command["id"],"result":{"value":"Before dispose"}}));
    fixture.send(json!({"guid":"page@title","method":"__dispose__","params":{}}));
    wait_until(|| fixture.supervisor.object("page@title").is_none()).await;
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert_eq!(result.unwrap(), "Before dispose");
    fixture.close().await;
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn valid_title_reply_stays_valid_when_later_ingress_ends_connection() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame(&mut fixture, true).await;
    let operation = tokio::spawn(async move { page.title(&metadata("page.title")).await });
    let command = fixture.command().await;

    // Act
    fixture.send(json!({"id":command["id"],"result":{"value":"Before terminal"}}));
    fixture.incoming.send(WsIncoming::EndOfStream).unwrap();
    timeout(DEADLINE, fixture.supervisor.wait_terminal())
        .await
        .unwrap();
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert_eq!(result.unwrap(), "Before terminal");
    assert!(matches!(
        fixture.supervisor.state(),
        SupervisorState::Closed { .. }
    ));
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn title_rejects_non_string_or_extra_reply_fields() {
    for value in [json!({"value":null}), json!({"value":"ok","other":true})] {
        // Arrange
        let mut fixture = Fixture::new().await;
        let page = create_page_with_main_frame(&mut fixture, true).await;
        let operation = tokio::spawn(async move { page.title(&metadata("page.title")).await });
        let command = fixture.command().await;

        // Act
        fixture.send(json!({"id":command["id"],"result":value}));
        let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

        // Assert
        assert!(matches!(
            result,
            Err(BrowserOperationError::InvalidResult(
                "Frame.title did not return exactly {value: string}"
            ))
        ));
        fixture.close().await;
    }
}

#[cfg(feature = "frame-title")]
#[tokio::test]
async fn title_preserves_remote_error_without_accepting_a_value() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page_with_main_frame(&mut fixture, true).await;
    let operation = tokio::spawn(async move { page.title(&metadata("page.title")).await });
    let command = fixture.command().await;

    // Act
    fixture.send(
        json!({"id":command["id"],"error":{"error":{"name":"Error","message":"frame detached"}}}),
    );
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(
        matches!(result, Err(BrowserOperationError::Remote(RemoteError::Exception { message, .. })) if message == "frame detached")
    );
    fixture.close().await;
}

#[cfg(any(feature = "page-events", feature = "close-state"))]
async fn create_page(fixture: &mut Fixture) -> RemotePage {
    let context = create_context(fixture, "context@1").await;
    let operation =
        tokio::spawn(async move { context.new_page(&metadata("browserContext.newPage")).await });
    let command = fixture.command().await;
    assert_eq!(command["method"], "newPage");
    fixture.send(json!({"guid":"context@1","method":"__create__","params":{"type":"Page","guid":"page@1","initializer":{}}}));
    fixture.send(json!({"id":command["id"],"result":{"page":{"guid":"page@1"}}}));
    timeout(DEADLINE, operation)
        .await
        .unwrap()
        .unwrap()
        .unwrap()
}

#[cfg(feature = "close-state")]
#[tokio::test]
async fn page_close_tombstone_changes_is_open_before_disposal_without_frame_title() {
    // Arrange
    let mut fixture = Fixture::with_policy(EventPolicy::ProjectCloseNotices).await;
    let page = create_page(&mut fixture).await;
    assert!(page.is_open());

    // Act
    fixture.send(json!({"guid":"page@1","method":"close","params":{}}));
    wait_until(|| {
        fixture
            .supervisor
            .object_with_close_state("page@1")
            .is_some_and(|(_, closed)| closed)
    })
    .await;

    // Assert
    assert!(fixture.supervisor.object("page@1").is_some());
    assert!(!page.is_open());
    assert!(
        matches!(fixture.supervisor.next_close_notice().await.unwrap(),
        pwright_playwright_server_session::CloseNotice::PageClosed(id)
        if id.guid == "page@1" && id.generation == page.created_sequence())
    );
    fixture.close().await;
}

#[cfg(feature = "page-events")]
#[tokio::test]
async fn page_network_events_are_armed_only_after_both_correlated_empty_replies() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page(&mut fixture).await;
    let operation = tokio::spawn(async move {
        page.enable_network_events(&metadata("browserContext.newPage"))
            .await
    });

    // Act
    let request = fixture.command().await;
    assert_eq!(request["guid"], "page@1");
    assert_eq!(request["method"], "updateSubscription");
    assert_eq!(request["params"], json!({"event":"request","enabled":true}));
    fixture.send(json!({"id":request["id"],"result":{}}));
    let response = fixture.command().await;
    assert_eq!(response["guid"], "page@1");
    assert_eq!(response["method"], "updateSubscription");
    assert_eq!(
        response["params"],
        json!({"event":"response","enabled":true})
    );
    assert!(!operation.is_finished());
    fixture.send(json!({"id":response["id"],"result":{}}));

    // Assert
    timeout(DEADLINE, operation)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    fixture.close().await;
}

#[cfg(feature = "page-events")]
#[tokio::test]
async fn rejected_page_subscription_is_not_published_as_armed() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page(&mut fixture).await;
    let operation = tokio::spawn(async move {
        page.enable_network_events(&metadata("browserContext.newPage"))
            .await
    });
    let request = fixture.command().await;

    // Act
    fixture.send(json!({"id":request["id"],"error":{"error":{"name":"Error","message":"subscription rejected"}}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(
        matches!(result, Err(BrowserOperationError::Remote(RemoteError::Exception { message, .. })) if message == "subscription rejected")
    );
    assert!(
        timeout(Duration::from_millis(20), fixture.sent.recv())
            .await
            .is_err()
    );
    fixture.close().await;
}

#[cfg(feature = "page-events")]
#[tokio::test]
async fn recycled_page_guid_cannot_install_subscriptions() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let page = create_page(&mut fixture).await;
    fixture.send(json!({"guid":"page@1","method":"__dispose__","params":{}}));
    fixture.send(json!({"guid":"context@1","method":"__create__","params":{"type":"Page","guid":"page@1","initializer":{}}}));
    timeout(DEADLINE, async {
        while fixture
            .supervisor
            .object("page@1")
            .is_none_or(|object| object.created_sequence == page.created_sequence())
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    // Act
    let result = page
        .enable_network_events(&metadata("browserContext.newPage"))
        .await;

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "page GUID was recycled"
        ))
    ));
    assert!(
        timeout(Duration::from_millis(20), fixture.sent.recv())
            .await
            .is_err()
    );
    fixture.close().await;
}

#[tokio::test]
async fn correlated_new_context_and_new_page_return_typed_live_guids() {
    // Arrange
    let mut fixture = Fixture::new().await;

    // Act
    let context = create_context(&mut fixture, "context@1").await;
    let operation = tokio::spawn({
        let context = context.clone();
        async move {
            context
                .new_page(&metadata("browser_context.new_page"))
                .await
        }
    });
    let command = fixture.command().await;
    fixture.send(json!({"guid":"context@1","method":"__create__","params":{"type":"Page","guid":"page@1","initializer":{}}}));
    fixture.send(json!({"id":3,"result":{"page":{"guid":"page@1"}}}));
    let page = timeout(DEADLINE, operation)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    // Assert
    assert_eq!(context.guid(), "context@1");
    assert!(context.is_open());
    assert_eq!(
        context.created_sequence(),
        context
            .supervisor()
            .object(context.guid())
            .unwrap()
            .created_sequence
    );
    assert_eq!(
        command,
        json!({"id":3,"guid":"context@1","method":"newPage","params":{},"metadata":{"wallTime":456,"apiName":"browser_context.new_page","internal":false}})
    );
    assert_eq!(page.guid(), "page@1");
    assert!(page.is_open());
    assert_eq!(
        page.created_sequence(),
        context
            .supervisor()
            .object(page.guid())
            .unwrap()
            .created_sequence
    );
    assert_eq!(
        fixture
            .supervisor
            .object("context@1")
            .unwrap()
            .parent
            .as_deref(),
        Some("browser@1")
    );
    assert_eq!(
        fixture
            .supervisor
            .object("page@1")
            .unwrap()
            .parent
            .as_deref(),
        Some("context@1")
    );
    fixture.close().await;
    assert!(!context.is_open());
    assert!(!page.is_open());
}

#[tokio::test]
async fn context_close_accepts_disposal_before_its_correlated_empty_reply() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let context = create_context(&mut fixture, "context@1").await;
    let operation = tokio::spawn({
        let context = context.clone();
        async move { context.close(&metadata("browser_context.close")).await }
    });

    // Act
    let command = fixture.command().await;
    fixture.send(json!({"guid":"context@1","method":"__dispose__","params":{}}));
    fixture.send(json!({"id":3,"result":{}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert_eq!(
        command,
        json!({"id":3,"guid":"context@1","method":"close","params":{},"metadata":{"wallTime":456,"apiName":"browser_context.close","internal":false}})
    );
    assert_eq!(result, Ok(()));
    assert!(!context.is_open());
    assert!(fixture.supervisor.object("context@1").is_none());
    fixture.close().await;
}

#[tokio::test]
async fn context_close_refusal_does_not_claim_success() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let context = create_context(&mut fixture, "context@1").await;
    let operation = tokio::spawn({
        let context = context.clone();
        async move { context.close(&metadata("browser_context.close")).await }
    });
    assert_eq!(fixture.command().await["method"], "close");

    // Act
    fixture.send(json!({"id":3,"error":{"error":{"name":"Error","message":"close refused"}}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::Remote(RemoteError::Exception { message, .. }))
            if message == "close refused"
    ));
    assert!(context.is_open());
    fixture.close().await;
}

#[tokio::test]
async fn context_close_rejects_nonempty_reply() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let context = create_context(&mut fixture, "context@1").await;
    let operation = tokio::spawn({
        let context = context.clone();
        async move { context.close(&metadata("browser_context.close")).await }
    });
    assert_eq!(fixture.command().await["method"], "close");

    // Act
    fixture.send(json!({"id":3,"result":{"unexpected":true}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "context close returned a nonempty result"
        ))
    ));
    assert!(context.is_open());
    fixture.close().await;
}

#[tokio::test]
async fn stale_context_generation_cannot_close_a_recycled_guid() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let old = create_context(&mut fixture, "context@reused").await;
    fixture.send(json!({"guid":"context@reused","method":"__dispose__","params":{}}));
    fixture.send(json!({"guid":"browser@1","method":"__create__","params":{"type":"BrowserContext","guid":"context@reused","initializer":{}}}));
    for _ in 0..10 {
        timeout(DEADLINE, fixture.supervisor.next_event())
            .await
            .unwrap()
            .unwrap();
    }

    // Act
    let result = old.close(&metadata("browser_context.close")).await;

    // Assert
    assert!(!old.is_open());
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "context GUID was recycled"
        ))
    ));
    assert!(
        timeout(Duration::from_millis(20), fixture.sent.recv())
            .await
            .is_err()
    );
    fixture.close().await;
}

#[tokio::test]
async fn remote_new_context_refusal_is_preserved_without_a_handle() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let browser = fixture.browser.clone();
    let operation = tokio::spawn(async move {
        browser
            .new_context(EmptyContextOptions, &metadata("browser.new_context"))
            .await
    });
    fixture.command().await;

    // Act
    fixture.send(json!({"id":2,"error":{"error":{"name":"Error","message":"SECRET refusal"}}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(
        matches!(result, Err(BrowserOperationError::Remote(RemoteError::Exception { message, .. })) if message == "SECRET refusal")
    );
    assert!(fixture.supervisor.object("context@1").is_none());
    assert!(
        !format!(
            "{:?}",
            BrowserOperationError::Remote(RemoteError::Exception {
                name: "Error".into(),
                message: "SECRET refusal".into(),
                stack: None,
                log: None,
            })
        )
        .contains("SECRET")
    );
    fixture.close().await;
}

#[tokio::test]
async fn malformed_channel_ref_never_returns_context_handle() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let browser = fixture.browser.clone();
    let operation = tokio::spawn(async move {
        browser
            .new_context(EmptyContextOptions, &metadata("browser.new_context"))
            .await
    });
    fixture.command().await;

    // Act
    fixture.send(json!({"id":2,"result":{"context":{"notGuid":"context@1"}}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "required channel GUID missing or invalid"
        ))
    ));
    fixture.close().await;
}

#[tokio::test]
async fn wrong_type_or_parent_is_not_promoted_from_a_reply() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let browser = fixture.browser.clone();
    let operation = tokio::spawn(async move {
        browser
            .new_context(EmptyContextOptions, &metadata("browser.new_context"))
            .await
    });
    fixture.command().await;

    // Act
    fixture.send(json!({"guid":"browser@1","method":"__create__","params":{"type":"Page","guid":"wrong@1","initializer":{}}}));
    fixture.send(json!({"id":2,"result":{"context":{"guid":"wrong@1"}}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "target GUID has wrong object type"
        ))
    ));
    fixture.close().await;
}

#[tokio::test]
async fn context_with_wrong_parent_is_not_promoted_from_a_reply() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let browser = fixture.browser.clone();
    let operation = tokio::spawn(async move {
        browser
            .new_context(EmptyContextOptions, &metadata("browser.new_context"))
            .await
    });
    fixture.command().await;

    // Act
    fixture.send(json!({"guid":"chromium@1","method":"__create__","params":{"type":"BrowserContext","guid":"wrong-parent@1","initializer":{}}}));
    fixture.send(json!({"id":2,"result":{"context":{"guid":"wrong-parent@1"}}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "target GUID has wrong parent"
        ))
    ));
    fixture.close().await;
}

#[tokio::test]
async fn malformed_new_page_reply_never_returns_a_page_handle() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let context = create_context(&mut fixture, "context@1").await;
    let operation = tokio::spawn(async move {
        context
            .new_page(&metadata("browser_context.new_page"))
            .await
    });
    fixture.command().await;

    // Act
    fixture.send(json!({"id":3,"result":{"page":{"notGuid":"page@1"}}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "required channel GUID missing or invalid"
        ))
    ));
    fixture.close().await;
}

#[tokio::test]
async fn remote_new_page_refusal_is_preserved_without_a_handle() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let context = create_context(&mut fixture, "context@1").await;
    let operation = tokio::spawn(async move {
        context
            .new_page(&metadata("browser_context.new_page"))
            .await
    });
    fixture.command().await;

    // Act
    fixture.send(json!({"id":3,"error":{"error":{"name":"Error","message":"page refused"}}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::Remote(RemoteError::Exception { message, .. }))
            if message == "page refused"
    ));
    fixture.close().await;
}

#[tokio::test]
async fn preexisting_context_guid_cannot_be_reused_as_a_new_result() {
    // Arrange
    let mut fixture = Fixture::new().await;
    fixture.send(json!({"guid":"browser@1","method":"__create__","params":{"type":"BrowserContext","guid":"old@1","initializer":{}}}));
    for _ in 0..8 {
        assert!(matches!(
            timeout(DEADLINE, fixture.supervisor.next_event())
                .await
                .unwrap()
                .unwrap(),
            Ingress::Created { .. }
        ));
    }
    let browser = fixture.browser.clone();
    let operation = tokio::spawn(async move {
        browser
            .new_context(EmptyContextOptions, &metadata("browser.new_context"))
            .await
    });
    fixture.command().await;

    // Act
    fixture.send(json!({"id":2,"result":{"context":{"guid":"old@1"}}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "result GUID predates command"
        ))
    ));
    fixture.close().await;
}

#[tokio::test]
async fn remote_close_rejects_pending_context_creation() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let browser = fixture.browser.clone();
    let operation = tokio::spawn(async move {
        browser
            .new_context(EmptyContextOptions, &metadata("browser.new_context"))
            .await
    });
    fixture.command().await;

    // Act
    fixture.incoming.send(WsIncoming::EndOfStream).unwrap();
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(result, Err(BrowserOperationError::Supervisor(_))));
    assert!(!fixture.browser.is_connected());
    assert!(matches!(
        fixture.supervisor.state(),
        SupervisorState::Closed { .. }
    ));
    assert_eq!(fixture.closes.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn disposed_result_context_is_not_returned_even_after_reply() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let browser = fixture.browser.clone();
    let operation = tokio::spawn(async move {
        browser
            .new_context(EmptyContextOptions, &metadata("browser.new_context"))
            .await
    });
    fixture.command().await;

    // Act
    fixture.send(json!({"guid":"browser@1","method":"__create__","params":{"type":"BrowserContext","guid":"gone@1","initializer":{}}}));
    fixture.send(json!({"guid":"gone@1","method":"__dispose__","params":{}}));
    fixture.send(json!({"id":2,"result":{"context":{"guid":"gone@1"}}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "target GUID is not live"
        ))
    ));
    fixture.close().await;
}

#[tokio::test]
async fn recycled_guid_does_not_reanimate_an_old_context_handle() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let old = create_context(&mut fixture, "context@reused").await;
    fixture.send(json!({"guid":"context@reused","method":"__dispose__","params":{}}));
    for _ in 0..9 {
        timeout(DEADLINE, fixture.supervisor.next_event())
            .await
            .unwrap()
            .unwrap();
    }

    // Act
    fixture.send(json!({"guid":"browser@1","method":"__create__","params":{"type":"BrowserContext","guid":"context@reused","initializer":{}}}));
    timeout(DEADLINE, fixture.supervisor.next_event())
        .await
        .unwrap()
        .unwrap();
    let result = old.new_page(&metadata("browser_context.new_page")).await;

    // Assert
    assert!(!old.is_open());
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "context GUID was recycled"
        ))
    ));
    fixture.close().await;
}

#[tokio::test]
async fn recycled_context_during_new_page_reply_cannot_publish_a_page_handle() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let old_context = create_context(&mut fixture, "context@reused").await;
    let operation = tokio::spawn({
        let old_context = old_context.clone();
        async move {
            old_context
                .new_page(&metadata("browser_context.new_page"))
                .await
        }
    });
    assert_eq!(fixture.command().await["method"], "newPage");

    // Act
    fixture.send(json!({"guid":"context@reused","method":"__dispose__","params":{}}));
    fixture.send(json!({"guid":"browser@1","method":"__create__","params":{"type":"BrowserContext","guid":"context@reused","initializer":{}}}));
    fixture.send(json!({"guid":"context@reused","method":"__create__","params":{"type":"Page","guid":"page@new","initializer":{}}}));
    fixture.send(json!({"id":3,"result":{"page":{"guid":"page@new"}}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(
        result,
        Err(BrowserOperationError::InvalidResult(
            "context GUID was recycled"
        ))
    ));
    assert!(!old_context.is_open());
    assert!(fixture.supervisor.object("page@new").is_some());
    fixture.close().await;
}

#[tokio::test]
async fn adopted_context_invalidates_old_browser_and_page_lineage() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let context = create_context(&mut fixture, "context@1").await;
    let operation = tokio::spawn({
        let context = context.clone();
        async move {
            context
                .new_page(&metadata("browser_context.new_page"))
                .await
        }
    });
    assert_eq!(fixture.command().await["method"], "newPage");
    fixture.send(json!({"guid":"context@1","method":"__create__","params":{"type":"Page","guid":"page@1","initializer":{}}}));
    fixture.send(json!({"id":3,"result":{"page":{"guid":"page@1"}}}));
    let page = timeout(DEADLINE, operation)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(context.is_open());
    assert!(page.is_open());

    // Act
    fixture.send(json!({"guid":"firefox@1","method":"__create__","params":{"type":"Browser","guid":"browser@other","initializer":{"name":"firefox","version":"155.0"}}}));
    fixture
        .send(json!({"guid":"browser@other","method":"__adopt__","params":{"guid":"context@1"}}));
    for _ in 0..11 {
        timeout(DEADLINE, fixture.supervisor.next_event())
            .await
            .unwrap()
            .unwrap();
    }

    // Assert
    assert_eq!(
        fixture
            .supervisor
            .object("page@1")
            .unwrap()
            .created_sequence,
        page.created_sequence()
    );
    assert_eq!(
        fixture
            .supervisor
            .object("context@1")
            .unwrap()
            .parent
            .as_deref(),
        Some("browser@other")
    );
    assert!(!context.is_open());
    assert!(!page.is_open());
    fixture.close().await;
}

#[tokio::test]
async fn prelaunched_browser_disposal_terminates_pending_context_creation() {
    // Arrange
    let mut fixture = Fixture::new().await;
    let browser = fixture.browser.clone();
    let operation = tokio::spawn(async move {
        browser
            .new_context(EmptyContextOptions, &metadata("browser.new_context"))
            .await
    });
    assert_eq!(fixture.command().await["method"], "newContext");

    // Act
    fixture.send(json!({"guid":"browser@1","method":"__dispose__","params":{}}));
    let result = timeout(DEADLINE, operation).await.unwrap().unwrap();

    // Assert
    assert!(matches!(result, Err(BrowserOperationError::Supervisor(_))));
    assert!(!fixture.browser.is_connected());
    assert!(fixture.supervisor.object("context@new").is_none());
}

#[test]
fn metadata_rejects_controls_and_redacts_user_supplied_name() {
    // Arrange
    let valid = OperationMetadata::new("browser.new_context.SECRET", 1).unwrap();

    // Act
    let control = OperationMetadata::new("browser.\nnew_context", 1);
    let oversized = OperationMetadata::new(&"x".repeat(MAX_API_NAME_BYTES + 1), 1);
    let debug = format!("{valid:?}");

    // Assert
    assert!(matches!(
        control,
        Err(BrowserOperationError::InvalidMetadata)
    ));
    assert!(matches!(
        oversized,
        Err(BrowserOperationError::InvalidMetadata)
    ));
    assert!(!debug.contains("SECRET"));
}
