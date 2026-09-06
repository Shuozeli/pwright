use std::sync::Arc;

use pwright_backend_cdp::{CdpLocatorService, CdpNavigationService};
use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cap_locator::{
    AriaRole, ClickOptions, CssSelector, FillOptions, HoverOptions, KeyboardModifier,
    LocatorErrorDetails, Position, Press, PressKey, PressOptions, PressParams, RegexMatcher,
    RoleOptions, ScrollMode, SelectOptionOptions, SelectOptionValue, SelectOptionValues,
    SetCheckedOptions, TestIdAttributes, WaitForOptions, WaitForState,
};
use pwright_cap_navigation::{BeginNavigation, BeginNavigationParams, GotoParams};
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpClient, CdpConnection, CdpSession};
use pwright_client::{
    AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, PageKind, RemoteInitializer,
    RemoteKind,
};
use pwright_client_locator::{
    LocatorAllExt, LocatorAllInnerTextsExt, LocatorAllTextContentsExt, LocatorBlurExt,
    LocatorCheckExt, LocatorCheckedExt, LocatorClearExt, LocatorClickExt, LocatorCompositionExt,
    LocatorCountExt, LocatorEditableExt, LocatorEnabledExt, LocatorFactoryExt, LocatorFillExt,
    LocatorFocusExt, LocatorGetAttributeExt, LocatorHoverExt, LocatorInnerHtmlExt,
    LocatorInnerTextExt, LocatorInputValueExt, LocatorPressExt, LocatorScrollIntoViewIfNeededExt,
    LocatorSelectOptionExt, LocatorTextContentExt, LocatorUncheckExt, LocatorVisibilityExt,
    LocatorWaitExt,
};
use pwright_client_navigation::PageGotoExt;
use pwright_core::{
    CallId, CallMetadata, CancellationId, CancellationReason, Command, FailureKind, ObjectId,
    ObjectKindTag, Service,
};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use pwright_runtime::{ControlledDispatcher, DirectDispatcher, NoDeadlineScheduler, RuntimeLimits};

async fn dispose_context_and_verify_target(
    browser: &CdpSession,
    browser_context_id: &str,
    target_id: &str,
) {
    browser
        .target_dispose_browser_context(browser_context_id)
        .await
        .expect("owned browser context should dispose");
    for _ in 0..20 {
        let targets = browser
            .target_get_targets()
            .await
            .expect("target list should remain available");
        if !targets.iter().any(|target| target.target_id == target_id) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("temporary target {target_id} remained live after context disposal");
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn typed_navigation_reaches_a_real_chromium_target() {
    // Arrange
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .expect("provider should attach without taking process ownership");
    let connection = Arc::clone(attachment.connection());
    let browser_session = CdpSession::browser(Arc::clone(&connection));
    let browser_context_id = browser_session
        .target_create_browser_context(None, None)
        .await
        .expect("temporary browser context should open");
    let target_id = browser_session
        .target_create_in_context("about:blank", &browser_context_id)
        .await
        .expect("temporary target should open");
    let session_id = browser_session
        .target_attach(&target_id)
        .await
        .expect("temporary target should attach");
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    let dispatcher = DirectDispatcher::with_object_kinds(
        CdpNavigationService::new(page),
        RuntimeLimits::new(3, 1),
        BROWSER_OBJECT_KIND_CATALOG,
    )
    .expect("browser object catalog should be valid");
    let browser_id = ObjectId::new(1);
    let context_id = ObjectId::new(2);
    let page_id = ObjectId::new(3);
    dispatcher
        .register_object(browser_id, None, BrowserKind::OBJECT_KIND)
        .expect("browser object should register");
    dispatcher
        .register_object(
            context_id,
            Some(browser_id),
            BrowserContextKind::OBJECT_KIND,
        )
        .expect("context object should register");
    dispatcher
        .register_object(page_id, Some(context_id), PageKind::OBJECT_KIND)
        .expect("page object should register");
    let client = Client::new(dispatcher, AtomicCallIdSource::new(1));
    let browser = client
        .browser(RemoteInitializer::new(
            browser_id,
            None,
            BrowserKind::OBJECT_KIND,
        ))
        .expect("typed browser should initialize");
    let context = browser
        .context(RemoteInitializer::new(
            context_id,
            Some(browser_id),
            BrowserContextKind::OBJECT_KIND,
        ))
        .expect("typed context should initialize");
    let typed_page = context
        .page(RemoteInitializer::new(
            page_id,
            Some(context_id),
            PageKind::OBJECT_KIND,
        ))
        .expect("typed page should initialize");

    // Act
    let navigation = typed_page
        .goto(
            GotoParams::new("data:text/html,<title>Pwright%20v2</title>"),
            CallMetadata::default(),
        )
        .await
        .expect("typed client goto should wait for real CDP commit and load");
    let pending_call_count = client.channel().pending_call_count();
    drop(typed_page);
    drop(context);
    drop(browser);
    drop(client);
    dispose_context_and_verify_target(&browser_session, &browser_context_id, &target_id).await;

    // Assert
    assert!(!navigation.frame_id.is_empty());
    assert!(!navigation.same_document);
    assert!(navigation.response.is_none());
    assert!(navigation.url.starts_with("data:text/html,"));
    assert_eq!(pending_call_count, Ok(0));
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("external browser should remain alive after typed client drop");
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process plus PWRIGHT_CDP_WS_URL, TAILSCALE_IP, and TAILSCALE_HOST"]
async fn explicit_cancellation_stops_a_real_pending_navigation() {
    // Arrange
    let websocket_url = std::env::var("PWRIGHT_CDP_WS_URL")
        .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile");
    let tailscale_ip = std::env::var("TAILSCALE_IP").expect("TAILSCALE_IP must be set");
    let tailscale_host = std::env::var("TAILSCALE_HOST").expect("TAILSCALE_HOST must be set");
    let listener = tokio::net::TcpListener::bind(format!("{tailscale_ip}:0"))
        .await
        .expect("hanging test server should bind to the Tailscale interface");
    let port = listener
        .local_addr()
        .expect("test server should have a local address")
        .port();
    let hanging_server = tokio::spawn(async move {
        let (_socket, _peer) = listener
            .accept()
            .await
            .expect("Chrome should connect to the hanging test server");
        std::future::pending::<()>().await;
    });

    let connection = CdpConnection::connect(&websocket_url)
        .await
        .expect("CDP connection should open");
    let browser = CdpSession::browser(Arc::clone(&connection));
    let browser_context_id = browser
        .target_create_browser_context(None, None)
        .await
        .expect("temporary browser context should open");
    let target_id = browser
        .target_create_in_context("about:blank", &browser_context_id)
        .await
        .expect("temporary target should open");
    let session_id = browser
        .target_attach(&target_id)
        .await
        .expect("temporary target should attach");
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    let dispatcher = ControlledDispatcher::with_object_kinds(
        CdpNavigationService::new(Arc::clone(&page)),
        CdpNavigationService::new(page),
        NoDeadlineScheduler,
        RuntimeLimits::new(1, 1),
        BROWSER_OBJECT_KIND_CATALOG,
    )
    .expect("browser object catalog should be valid");
    let page_id = ObjectId::new(1);
    dispatcher
        .register_object(page_id, None, ObjectKindTag::new(3))
        .expect("page object should register");
    let cancellation_id = CancellationId::new(7);
    let command = Command::<BeginNavigation>::new(
        CallId::new(1),
        page_id,
        CallMetadata::new(None, Some(cancellation_id)),
        BeginNavigationParams::new(format!("http://{tailscale_host}:{port}/hang")),
    );
    let mut dispatch = Box::pin(dispatcher.call(command));
    tokio::select! {
        result = &mut dispatch => panic!("navigation completed before cancellation: {}", result.is_ok()),
        () = tokio::time::sleep(std::time::Duration::from_millis(150)) => {}
    }

    // Act
    dispatcher
        .cancel_call(CallId::new(1), cancellation_id, CancellationReason::Caller)
        .expect("matching cancellation should be accepted");
    let dispatch_result = tokio::time::timeout(std::time::Duration::from_secs(5), dispatch)
        .await
        .expect("cancelled dispatch should terminate promptly")
        .expect("in-process dispatch should be infallible");

    // Assert
    assert_eq!(
        dispatch_result
            .outcome()
            .as_ref()
            .expect_err("navigation should be cancelled")
            .kind(),
        FailureKind::Cancelled
    );
    assert_eq!(dispatcher.pending_call_count(), Ok(0));
    dispose_context_and_verify_target(&browser, &browser_context_id, &target_id).await;
    hanging_server.abort();
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn explicit_cancellation_releases_real_pressed_keys_in_reverse() {
    // Arrange
    let websocket_url = std::env::var("PWRIGHT_CDP_WS_URL")
        .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile");
    let connection = CdpConnection::connect(&websocket_url)
        .await
        .expect("CDP connection should open");
    let browser = CdpSession::browser(Arc::clone(&connection));
    let browser_context_id = browser
        .target_create_browser_context(None, None)
        .await
        .expect("temporary browser context should open");
    let target_id = browser
        .target_create_in_context("about:blank", &browser_context_id)
        .await
        .expect("temporary target should open");
    let session_id = browser
        .target_attach(&target_id)
        .await
        .expect("temporary target should attach");
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    page.runtime_evaluate(
        r#"(() => {
          document.body.innerHTML = '<input id="target" value="abc">';
          window.__press_events = [];
          for (const type of ['keydown', 'keyup']) {
            document.querySelector('#target').addEventListener(type, event => {
              window.__press_events.push({
                type,
                key: event.key,
                code: event.code,
                shift: event.shiftKey,
              });
            });
          }
          return true;
        })()"#,
    )
    .await
    .expect("press fixture should install");
    let service = CdpLocatorService::new(Arc::clone(&page));
    let dispatcher = Arc::new(
        ControlledDispatcher::with_object_kinds(
            service.clone(),
            service,
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid"),
    );
    let page_id = ObjectId::new(1);
    dispatcher
        .register_object(page_id, None, PageKind::OBJECT_KIND)
        .expect("page object should register");
    let call_id = CallId::new(1);
    let cancellation_id = CancellationId::new(11);
    let command = Command::<Press>::new(
        call_id,
        page_id,
        CallMetadata::new(None, Some(cancellation_id)),
        PressParams::new(
            CssSelector::new("#target").expect("valid target CSS"),
            PressKey::new("Shift+KeyA").expect("valid shifted chord"),
            PressOptions::new()
                .with_delay(std::time::Duration::from_secs(30))
                .with_no_wait_after(true),
        ),
    );
    let dispatch = tokio::spawn({
        let dispatcher = Arc::clone(&dispatcher);
        async move { dispatcher.call(command).await }
    });
    let mut saw_main_key_down = false;
    for _ in 0..100 {
        let state = page
            .runtime_evaluate("window.__press_events")
            .await
            .expect("press events should remain readable");
        saw_main_key_down = state["result"]["value"].as_array().is_some_and(|events| {
            events
                .iter()
                .any(|event| event["type"] == "keydown" && event["code"] == "KeyA")
        });
        if saw_main_key_down {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        saw_main_key_down,
        "main key should be down before cancellation"
    );

    // Act
    dispatcher
        .cancel_call(call_id, cancellation_id, CancellationReason::Caller)
        .expect("matching cancellation should be accepted");
    let dispatch_result = tokio::time::timeout(std::time::Duration::from_secs(5), dispatch)
        .await
        .expect("cancelled press should terminate promptly")
        .expect("dispatch task should join")
        .expect("in-process dispatch should be infallible");
    let released = page
        .runtime_evaluate("window.__press_events")
        .await
        .expect("released key events should be readable");

    // Assert
    assert_eq!(
        dispatch_result
            .outcome()
            .as_ref()
            .expect_err("press should be cancelled")
            .kind(),
        FailureKind::Cancelled
    );
    assert_eq!(
        released["result"]["value"],
        serde_json::json!([
            {"type": "keydown", "key": "Shift", "code": "ShiftLeft", "shift": true},
            {"type": "keydown", "key": "A", "code": "KeyA", "shift": true},
            {"type": "keyup", "key": "A", "code": "KeyA", "shift": true},
            {"type": "keyup", "key": "Shift", "code": "ShiftLeft", "shift": false},
        ])
    );
    assert_eq!(dispatcher.pending_call_count(), Ok(0));
    dispose_context_and_verify_target(&browser, &browser_context_id, &target_id).await;
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn typed_scroll_into_view_preserves_visible_geometry_and_reveals_below_fold_target() {
    // Arrange
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .expect("provider should attach without taking process ownership");
    let connection = Arc::clone(attachment.connection());
    let browser_session = CdpSession::browser(Arc::clone(&connection));
    let browser_context_id = browser_session
        .target_create_browser_context(None, None)
        .await
        .expect("temporary browser context should open");
    let target_id = browser_session
        .target_create_in_context("about:blank", &browser_context_id)
        .await
        .expect("temporary target should open");
    let session_id = browser_session
        .target_attach(&target_id)
        .await
        .expect("temporary target should attach");
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    page.runtime_evaluate(
        r#"document.body.innerHTML = `
          <div id="visible" style="position:fixed;left:8px;top:8px;width:20px;height:20px">Visible</div>
          <div style="height:250vh"></div>
          <div id="below" style="height:40px">Below fold</div>`; true"#,
    )
    .await
    .expect("scroll fixture should install");
    let dispatcher = DirectDispatcher::with_object_kinds(
        CdpLocatorService::new(Arc::clone(&page)),
        RuntimeLimits::new(3, 1),
        BROWSER_OBJECT_KIND_CATALOG,
    )
    .expect("browser object catalog should be valid");
    let browser_id = ObjectId::new(1);
    let context_id = ObjectId::new(2);
    let page_id = ObjectId::new(3);
    dispatcher
        .register_object(browser_id, None, BrowserKind::OBJECT_KIND)
        .expect("browser object should register");
    dispatcher
        .register_object(
            context_id,
            Some(browser_id),
            BrowserContextKind::OBJECT_KIND,
        )
        .expect("context object should register");
    dispatcher
        .register_object(page_id, Some(context_id), PageKind::OBJECT_KIND)
        .expect("page object should register");
    let client = Client::new(dispatcher, AtomicCallIdSource::new(1));
    let browser = client
        .browser(RemoteInitializer::new(
            browser_id,
            None,
            BrowserKind::OBJECT_KIND,
        ))
        .expect("typed browser should initialize");
    let context = browser
        .context(RemoteInitializer::new(
            context_id,
            Some(browser_id),
            BrowserContextKind::OBJECT_KIND,
        ))
        .expect("typed context should initialize");
    let typed_page = context
        .page(RemoteInitializer::new(
            page_id,
            Some(context_id),
            PageKind::OBJECT_KIND,
        ))
        .expect("typed page should initialize");
    let visible = typed_page.locator(CssSelector::new("#visible").expect("valid visible CSS"));
    let below = typed_page.locator(CssSelector::new("#below").expect("valid below-fold CSS"));
    let before = page
        .runtime_evaluate(
            "({ y: scrollY, viewport: innerHeight, visibleTop: document.querySelector('#visible').getBoundingClientRect().top, belowTop: document.querySelector('#below').getBoundingClientRect().top })",
        )
        .await
        .expect("pre-scroll geometry should be readable");

    // Act
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        visible.scroll_into_view_if_needed(CallMetadata::default()),
    )
    .await
    .expect("visible-target scroll should terminate")
    .expect("visible target should already satisfy the command");
    let after_visible = page
        .runtime_evaluate(
            "({ y: scrollY, visibleTop: document.querySelector('#visible').getBoundingClientRect().top })",
        )
        .await
        .expect("visible-target geometry should remain readable");
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        below.scroll_into_view_if_needed(CallMetadata::default()),
    )
    .await
    .expect("below-fold scroll should terminate")
    .expect("below-fold target should scroll into view");
    let after_below = page
        .runtime_evaluate(
            "({ y: scrollY, viewport: innerHeight, belowTop: document.querySelector('#below').getBoundingClientRect().top, belowBottom: document.querySelector('#below').getBoundingClientRect().bottom })",
        )
        .await
        .expect("post-scroll geometry should be readable");

    // Assert
    let before_value = &before["result"]["value"];
    let visible_value = &after_visible["result"]["value"];
    let below_value = &after_below["result"]["value"];
    assert_eq!(visible_value["y"], before_value["y"]);
    assert_eq!(visible_value["visibleTop"], before_value["visibleTop"]);
    assert!(
        before_value["belowTop"]
            .as_f64()
            .expect("below top is numeric")
            > before_value["viewport"]
                .as_f64()
                .expect("viewport height is numeric")
    );
    assert!(below_value["y"].as_f64().expect("scroll y is numeric") > 0.0);
    assert!(
        below_value["belowTop"]
            .as_f64()
            .expect("below top is numeric")
            >= 0.0
    );
    assert!(
        below_value["belowBottom"]
            .as_f64()
            .expect("below bottom is numeric")
            <= below_value["viewport"]
                .as_f64()
                .expect("viewport height is numeric")
    );
    drop(below);
    drop(visible);
    drop(typed_page);
    drop(context);
    drop(browser);
    drop(client);
    dispose_context_and_verify_target(&browser_session, &browser_context_id, &target_id).await;
}

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn typed_locator_actions_reach_a_real_chromium_target() {
    // Arrange
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .expect("provider should attach without taking process ownership");
    let connection = Arc::clone(attachment.connection());
    let browser_session = CdpSession::browser(Arc::clone(&connection));
    let browser_context_id = browser_session
        .target_create_browser_context(None, None)
        .await
        .expect("temporary browser context should open");
    let target_id = browser_session
        .target_create_in_context("about:blank", &browser_context_id)
        .await
        .expect("temporary target should open");
    let session_id = browser_session
        .target_attach(&target_id)
        .await
        .expect("temporary target should attach");
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    CdpNavigationService::new(Arc::clone(&page))
        .call(GotoParams::new(
            "data:text/html,%3Cbutton%20aria-label%3D%22Save%20changes%22%20onclick%3D%22window.__outside%3Dtrue%22%3EOutside%3C%2Fbutton%3E%3Csection%20role%3D%22dialog%22%20aria-label%3D%22Settings%22%3E%3Cbutton%20disabled%20aria-label%3D%22Save%20changes%22%3EDisabled%3C%2Fbutton%3E%3Cbutton%20onclick%3D%22window.__role_clicked%3Dtrue%22%20aria-label%3D%22Save%20changes%22%3E%3Cspan%3ESave%3C%2Fspan%3E%3C%2Fbutton%3E%3C%2Fsection%3E",
        ))
        .await
        .expect("test document should load");
    page.runtime_evaluate(
        r#"document.body.insertAdjacentHTML('beforeend', `
          <article class="card" style="display:none"><span class="ready">Ready</span><button onclick="window.__wrong_hidden=true">Choose</button></article>
          <article class="card"><span class="ready">Ready</span><i data-excluded></i><button onclick="window.__wrong_excluded=true">Choose</button></article>
          <article class="card"><span class="ready">Ready</span><button onclick="window.__filtered_clicked=true">Choose</button></article>
          <article class="card">Blocked<button onclick="window.__wrong_blocked=true">Choose</button></article>
          <button id="disabled-state" disabled>Disabled state</button>
          <input id="editable-state">
          <label>Agree <input id="checked-state" type="checkbox" checked></label>
          <textarea id="input-value-textarea">line one
line two</textarea>
          <select id="input-value-select"><option value="first">First</option><option value="chosen" selected>Chosen</option></select>
          <div id="input-value-invalid">Not a form control</div>
          <input class="input-value-duplicate" value="one"><input class="input-value-duplicate" value="two">
          <div class="all-text-row">first<span> nested</span></div>
          <div class="all-text-row"></div>
          <div class="all-text-row" style="display:none">hidden</div>
          <svg class="all-text-row"><text>svg</text></svg>
          <div class="all-inner-row">Visible<span style="display:none">hidden</span> text</div>
          <div class="all-inner-row"></div>
          <div class="all-inner-row">third</div>
          <svg id="all-inner-invalid"><text>svg</text></svg>
        `); true"#,
    )
    .await
    .expect("filter fixture should be installed");
    let dispatcher = DirectDispatcher::with_object_kinds(
        CdpLocatorService::new(Arc::clone(&page)),
        RuntimeLimits::new(3, 1),
        BROWSER_OBJECT_KIND_CATALOG,
    )
    .expect("browser object catalog should be valid");
    let browser_id = ObjectId::new(1);
    let context_id = ObjectId::new(2);
    let page_id = ObjectId::new(3);
    dispatcher
        .register_object(browser_id, None, BrowserKind::OBJECT_KIND)
        .expect("browser object should register");
    dispatcher
        .register_object(
            context_id,
            Some(browser_id),
            BrowserContextKind::OBJECT_KIND,
        )
        .expect("context object should register");
    dispatcher
        .register_object(page_id, Some(context_id), PageKind::OBJECT_KIND)
        .expect("page object should register");
    let client = Client::new(dispatcher, AtomicCallIdSource::new(1));
    let browser = client
        .browser(RemoteInitializer::new(
            browser_id,
            None,
            BrowserKind::OBJECT_KIND,
        ))
        .expect("typed browser should initialize");
    let context = browser
        .context(RemoteInitializer::new(
            context_id,
            Some(browser_id),
            BrowserContextKind::OBJECT_KIND,
        ))
        .expect("typed context should initialize");
    let typed_page = context
        .page(RemoteInitializer::new(
            page_id,
            Some(context_id),
            PageKind::OBJECT_KIND,
        ))
        .expect("typed page should initialize");
    let locator = typed_page
        .get_by_role(
            AriaRole::Dialog,
            RoleOptions::new()
                .with_name_regex(RegexMatcher::new("^Sett.ngs$", "").expect("valid dialog regex")),
        )
        .expect("dialog role selector should be valid")
        .get_by_role(
            AriaRole::Button,
            RoleOptions::new()
                .with_name_regex(
                    RegexMatcher::new("^Save\\s+changes$", "").expect("valid button-name regex"),
                )
                .with_disabled(false),
        )
        .expect("button role selector should be valid");
    let label_locator = typed_page.get_by_label_regex(
        RegexMatcher::new("^primary\\s+action$", "i").expect("valid label regex"),
    );
    let text_locator = typed_page
        .get_by_text_regex(RegexMatcher::new("^Text\\s+target$", "").expect("valid text regex"));
    let card_count_locator = typed_page.locator(CssSelector::new(".card").expect("valid card CSS"));
    let all_text_contents_locator =
        typed_page.locator(CssSelector::new(".all-text-row").expect("valid all-text-contents CSS"));
    let all_inner_texts_locator =
        typed_page.locator(CssSelector::new(".all-inner-row").expect("valid all-inner-texts CSS"));
    let invalid_all_inner_texts_locator = typed_page.locator(
        CssSelector::new("#all-inner-invalid").expect("valid invalid all-inner-texts CSS"),
    );
    let hidden_card_locator = typed_page
        .locator(CssSelector::new(".card").expect("valid card CSS"))
        .nth(0);
    let visible_card_locator = typed_page
        .locator(CssSelector::new(".card").expect("valid card CSS"))
        .nth(2);
    let missing_visibility_locator =
        typed_page.locator(CssSelector::new(".missing").expect("valid missing CSS"));
    let strict_visibility_locator =
        typed_page.locator(CssSelector::new(".card").expect("valid card CSS"));
    let disabled_state_locator =
        typed_page.locator(CssSelector::new("#disabled-state").expect("valid disabled CSS"));
    let editable_state_locator =
        typed_page.locator(CssSelector::new("#editable-state").expect("valid editable CSS"));
    let checked_state_locator = typed_page.get_by_text("Agree", true);
    let invalid_checked_locator = typed_page
        .locator(CssSelector::new(".card").expect("valid card CSS"))
        .nth(0);
    let late_enabled_locator =
        typed_page.locator(CssSelector::new("#late-enabled").expect("valid late CSS"));
    let wait_cycle_locator =
        typed_page.locator(CssSelector::new("#wait-cycle").expect("valid wait CSS"));
    let text_content_locator =
        typed_page.locator(CssSelector::new("#late-text-content").expect("valid text CSS"));
    let inner_html_locator =
        typed_page.locator(CssSelector::new("#late-inner-html").expect("valid inner-HTML CSS"));
    let svg_inner_html_locator =
        typed_page.locator(CssSelector::new("#svg-inner-html").expect("valid SVG CSS"));
    let inner_text_locator =
        typed_page.locator(CssSelector::new("#late-inner-text").expect("valid inner-text CSS"));
    let svg_inner_text_locator =
        typed_page.locator(CssSelector::new("#svg-inner-text").expect("valid SVG CSS"));
    let delayed_label_value_locator = typed_page
        .locator(CssSelector::new("#late-input-value-label").expect("valid input-value label CSS"));
    let textarea_value_locator = typed_page.locator(
        CssSelector::new("#input-value-textarea").expect("valid textarea input-value CSS"),
    );
    let select_value_locator = typed_page
        .locator(CssSelector::new("#input-value-select").expect("valid select input-value CSS"));
    let invalid_value_locator = typed_page
        .locator(CssSelector::new("#input-value-invalid").expect("valid invalid input-value CSS"));
    let strict_value_locator = typed_page
        .locator(CssSelector::new(".input-value-duplicate").expect("valid strict input-value CSS"));
    let filtered_locator = typed_page
        .locator(CssSelector::new(".card").expect("valid card CSS"))
        .filter_has(typed_page.locator(CssSelector::new(".ready").expect("valid marker CSS")))
        .expect("same-Page has filter should compose")
        .filter_has_not(
            typed_page.locator(CssSelector::new("[data-excluded]").expect("valid excluded CSS")),
        )
        .expect("same-Page has-not filter should compose")
        .filter_text_regex(RegexMatcher::new("^Ready", "").expect("valid positive regex"))
        .filter_not_text_regex(RegexMatcher::new("Blocked", "i").expect("valid negative regex"))
        .filter_visible(true)
        .get_by_role(
            AriaRole::Button,
            RoleOptions::new().with_name_regex(
                RegexMatcher::new("^Choose$", "").expect("valid filtered-role regex"),
            ),
        )
        .expect("filtered descendant role should be valid");
    let invalid_regex_locator = typed_page.get_by_text_regex(
        RegexMatcher::new("[", "").expect("flags are valid even when browser syntax is not"),
    );
    let placeholder_locator = typed_page.get_by_placeholder("User  Name", true);
    let alt_locator = typed_page.get_by_alt_text_regex(
        RegexMatcher::new("^Product Hero \\d+$", "").expect("valid alt regex"),
    );
    let title_locator = typed_page.get_by_title("release notes", false);
    let standard_test_id_locator = typed_page.get_by_test_id("standard-action");
    let custom_test_id_locator = typed_page.get_by_test_id_regex_with_attributes(
        TestIdAttributes::new("data-pw,data-testid").expect("valid test-id attributes"),
        RegexMatcher::new("^custom-\\d+$", "").expect("valid test-id regex"),
    );
    let hover_target_locator =
        typed_page.locator(CssSelector::new("#hover-target").expect("valid hover target CSS"));
    let hover_trial_locator =
        typed_page.locator(CssSelector::new("#hover-trial").expect("valid hover trial CSS"));
    let focus_target_locator =
        typed_page.locator(CssSelector::new("#focus-target").expect("valid focus target CSS"));
    let non_focusable_locator =
        typed_page.locator(CssSelector::new("#non-focusable").expect("valid non-focusable CSS"));
    let strict_focus_locator =
        typed_page.locator(CssSelector::new(".focus-duplicate").expect("valid strict focus CSS"));
    let press_target_locator =
        typed_page.locator(CssSelector::new("#press-target").expect("valid press target CSS"));
    let set_checked_locator = typed_page.get_by_text("Remember me", true);
    let set_checked_trial_locator = typed_page
        .locator(CssSelector::new("#set-checked-trial").expect("valid SetChecked trial CSS"));
    let set_checked_radio_locator = typed_page
        .locator(CssSelector::new("#set-checked-radio").expect("valid SetChecked radio CSS"));
    let select_label_locator = typed_page.get_by_text("Flavor", true);
    let select_multiple_locator = typed_page
        .locator(CssSelector::new("#select-multiple").expect("valid multiple select CSS"));
    let press_submit_locator =
        typed_page.locator(CssSelector::new("#press-submit").expect("valid press submit CSS"));

    // Act
    let initial_card_count = card_count_locator
        .count(CallMetadata::default())
        .await
        .expect("count should snapshot every matching card without strictness");
    let initial_all_text_contents = all_text_contents_locator
        .all_text_contents(CallMetadata::default())
        .await
        .expect("all text contents should snapshot every match without strictness");
    let all_locator_list = all_text_contents_locator
        .clone()
        .all(CallMetadata::default())
        .await
        .expect("all should compose one count snapshot with lazy nth locators");
    let all_locator_list_len = all_locator_list.len();
    let initial_all_inner_texts = all_inner_texts_locator
        .all_inner_texts(CallMetadata::default())
        .await
        .expect("all inner texts should snapshot every HTML match without strictness");
    let invalid_all_inner_texts_error = invalid_all_inner_texts_locator
        .all_inner_texts(CallMetadata::default())
        .await
        .expect_err("all inner texts should reject a non-HTMLElement match");
    let hidden_card = hidden_card_locator
        .is_hidden(CallMetadata::default())
        .await
        .expect("hidden should be composed from the visibility snapshot");
    let visible_card = visible_card_locator
        .is_visible(CallMetadata::default())
        .await
        .expect("visible card should be reported immediately");
    let missing_visibility = missing_visibility_locator
        .is_visible(CallMetadata::default())
        .await
        .expect("missing visibility target should return false immediately");
    let strict_visibility_error = strict_visibility_locator
        .is_visible(CallMetadata::default())
        .await
        .expect_err("visibility should remain strict for multiple matches");
    let disabled_state = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        disabled_state_locator.is_disabled(CallMetadata::default()),
    )
    .await
    .expect("disabled state query should terminate")
    .expect("disabled should compose over the enabled state command");
    let editable_state = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        editable_state_locator.is_editable(CallMetadata::default()),
    )
    .await
    .expect("editable state query should terminate")
    .expect("plain input should be editable");
    let checked_state = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        checked_state_locator.is_checked(CallMetadata::default()),
    )
    .await
    .expect("checked state query should terminate")
    .expect("checked checkbox should report its current state");
    let invalid_checked_error = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        invalid_checked_locator.is_checked(CallMetadata::default()),
    )
    .await
    .expect("invalid checked-state query should terminate")
    .expect_err("non-checkable target should remain a typed failure");
    page.runtime_evaluate(
        "setTimeout(() => document.body.insertAdjacentHTML('beforeend', '<button id=late-enabled>Late</button>'), 75); true",
    )
    .await
    .expect("late state target should be scheduled");
    let late_enabled = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        late_enabled_locator.is_enabled(CallMetadata::default()),
    )
    .await
    .expect("late enabled-state query should terminate")
    .expect("state getter should wait for its strict target to appear");
    page.runtime_evaluate(
        r#"setTimeout(() => {
          const element = document.createElement('div');
          element.id = 'late-text-content';
          element.textContent = '  Ready\nnow  ';
          document.body.append(element);
        }, 75); true"#,
    )
    .await
    .expect("late text-content target should be scheduled");
    let exact_text_content = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        text_content_locator.text_content(CallMetadata::default()),
    )
    .await
    .expect("text-content query should terminate")
    .expect("text-content query should wait for the strict target");
    let strict_text_content_error = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        strict_visibility_locator.text_content(CallMetadata::default()),
    )
    .await
    .expect("strict text-content query should terminate")
    .expect_err("text-content should remain strict for multiple matches");
    page.runtime_evaluate(
        r#"setTimeout(() => document.body.insertAdjacentHTML(
          'beforeend',
          '<div id=late-inner-html data-state=ready><span data-state=ready>Text &amp; more</span><!--note--></div><svg id=svg-inner-html data-kind=vector><text>SVG &amp; text</text></svg>'
        ), 75); true"#,
    )
    .await
    .expect("serialized inner-HTML targets should be scheduled");
    let serialized_inner_html = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        inner_html_locator.inner_html(CallMetadata::default()),
    )
    .await
    .expect("inner-HTML query should terminate")
    .expect("inner-HTML query should wait for the strict target");
    let svg_inner_html = svg_inner_html_locator
        .inner_html(CallMetadata::default())
        .await
        .expect("SVG elements should expose serialized inner HTML");
    let strict_inner_html_error = strict_visibility_locator
        .inner_html(CallMetadata::default())
        .await
        .expect_err("inner HTML should remain strict for multiple matches");
    let attribute = inner_html_locator
        .get_attribute("data-state", CallMetadata::default())
        .await
        .expect("HTML element should expose its exact attribute");
    let missing_attribute = inner_html_locator
        .get_attribute("missing", CallMetadata::default())
        .await
        .expect("missing attribute should be a successful nullable query");
    let svg_attribute = svg_inner_html_locator
        .get_attribute("data-kind", CallMetadata::default())
        .await
        .expect("SVG element should expose its exact attribute");
    let strict_attribute_error = strict_visibility_locator
        .get_attribute("class", CallMetadata::default())
        .await
        .expect_err("get attribute should remain strict for multiple matches");
    page.runtime_evaluate(
        r#"setTimeout(() => document.body.insertAdjacentHTML(
          'beforeend',
          '<div id=late-inner-text>Visible<span style=display:none>hidden</span> text</div><svg id=svg-inner-text><text>SVG text</text></svg>'
        ), 75); true"#,
    )
    .await
    .expect("rendered inner-text targets should be scheduled");
    let rendered_inner_text = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        inner_text_locator.inner_text(CallMetadata::default()),
    )
    .await
    .expect("inner-text query should terminate")
    .expect("inner-text query should wait for the strict HTML target");
    let raw_inner_text_content = inner_text_locator
        .text_content(CallMetadata::default())
        .await
        .expect("text-content should expose the raw DOM text for comparison");
    let svg_inner_text_error = svg_inner_text_locator
        .inner_text(CallMetadata::default())
        .await
        .expect_err("inner-text should reject non-HTMLElement targets");
    page.runtime_evaluate(
        r#"setTimeout(() => document.body.insertAdjacentHTML(
          'beforeend',
          '<label id=late-input-value-label for=late-input-value>Account</label><input id=late-input-value value="alice &amp; exact">'
        ), 75); true"#,
    )
    .await
    .expect("label-retargeted input-value target should be scheduled");
    let label_input_value = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        delayed_label_value_locator.input_value(CallMetadata::default()),
    )
    .await
    .expect("label input-value query should terminate")
    .expect("input-value should wait and follow the associated label");
    let textarea_input_value = textarea_value_locator
        .input_value(CallMetadata::default())
        .await
        .expect("textarea should expose its current value");
    let select_input_value = select_value_locator
        .input_value(CallMetadata::default())
        .await
        .expect("select should expose its selected value");
    let invalid_input_value_error = invalid_value_locator
        .input_value(CallMetadata::default())
        .await
        .expect_err("non-control target should remain a typed failure");
    let strict_input_value_error = strict_value_locator
        .input_value(CallMetadata::default())
        .await
        .expect_err("input-value should remain strict for multiple controls");
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        wait_cycle_locator.wait_for(
            WaitForOptions::new().with_state(WaitForState::Hidden),
            CallMetadata::default(),
        ),
    )
    .await
    .expect("missing-target hidden wait should terminate")
    .expect("hidden should succeed when the strict target is absent");
    page.runtime_evaluate(
        "setTimeout(() => document.body.insertAdjacentHTML('beforeend', '<div id=wait-cycle style=display:none>Wait</div>'), 75); true",
    )
    .await
    .expect("hidden wait target should be scheduled");
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        wait_cycle_locator.wait_for(
            WaitForOptions::new().with_state(WaitForState::Attached),
            CallMetadata::default(),
        ),
    )
    .await
    .expect("attached wait should terminate")
    .expect("attached should not require visibility");
    page.runtime_evaluate(
        "setTimeout(() => document.querySelector('#wait-cycle').style.display = 'block', 75); true",
    )
    .await
    .expect("visible transition should be scheduled");
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        wait_cycle_locator.wait_for(WaitForOptions::default(), CallMetadata::default()),
    )
    .await
    .expect("visible wait should terminate")
    .expect("default wait state should be visible");
    page.runtime_evaluate(
        "setTimeout(() => document.querySelector('#wait-cycle').remove(), 75); true",
    )
    .await
    .expect("detached transition should be scheduled");
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        wait_cycle_locator.wait_for(
            WaitForOptions::new().with_state(WaitForState::Detached),
            CallMetadata::default(),
        ),
    )
    .await
    .expect("detached wait should terminate")
    .expect("detached should succeed after removal");
    let strict_wait_error = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        strict_visibility_locator.wait_for(WaitForOptions::default(), CallMetadata::default()),
    )
    .await
    .expect("strict wait should terminate")
    .expect_err("wait should remain strict for multiple matches");
    locator
        .click(ClickOptions::new(), CallMetadata::default())
        .await
        .expect("typed locator click should pass actionability and dispatch input");
    filtered_locator
        .click(ClickOptions::new(), CallMetadata::default())
        .await
        .expect("Playwright-style filters should retain only the intended card");
    let invalid_regex_error = invalid_regex_locator
        .click(ClickOptions::new(), CallMetadata::default())
        .await
        .expect_err("browser-invalid ECMAScript source should fail as an invalid selector");
    let first_clicked = page
        .runtime_evaluate(
            "({ role: window.__role_clicked === true, outside: window.__outside === true, filtered: window.__filtered_clicked === true, wrongHidden: window.__wrong_hidden === true, wrongExcluded: window.__wrong_excluded === true, wrongBlocked: window.__wrong_blocked === true })",
        )
        .await
        .expect("first-document click state should be readable");
    CdpNavigationService::new(Arc::clone(&page))
        .call(GotoParams::new(
            "data:text/html,%3Cinput%20type%3D%22button%22%20aria-label%3D%22Primary%20action%22%20onclick%3D%22window.__label_clicked%3Dtrue%22%3E%3Cbutton%20onclick%3D%22window.__text_clicked%3Dtrue%22%3EText%20%20target%3C%2Fbutton%3E",
        ))
        .await
        .expect("second document should replace the selector execution context");
    page.runtime_evaluate(
        r#"document.body.insertAdjacentHTML('beforeend', `
          <input placeholder="User  Name" onclick="window.__placeholder_clicked=true">
          <input placeholder="User Name" onclick="window.__wrong_normalized_placeholder=true">
          <img alt="Product Hero 42" style="display:block;width:40px;height:20px"
            onclick="window.__alt_clicked=true">
          <button title="Release Notes 2026" onclick="window.__title_clicked=true">Notes</button>
          <button data-testid="standard-action"
            onclick="window.__standard_test_id_clicked=true">Standard test id</button>
          <button data-pw="custom-42"
            onclick="window.__custom_test_id_clicked=true">Custom test id</button>
          <button id="hover-target" disabled
            style="position:fixed;left:40px;top:80px;border:4px solid;padding:20px;width:120px;height:80px">
            Disabled hover target
          </button>
          <button id="hover-trial"
            style="position:fixed;left:240px;top:80px;width:120px;height:80px">
            Trial hover target
          </button>
          <button id="focus-target">Focus target</button>
          <div id="non-focusable">Non-focusable target</div>
          <input class="focus-duplicate"><input class="focus-duplicate">
          <input id="press-target" value="abc">
          <label id="set-checked-label" for="set-checked-target">Remember me</label>
          <input id="set-checked-target" type="checkbox">
          <input id="set-checked-trial" type="checkbox">
          <input id="set-checked-radio" type="radio" checked>
          <label id="select-label" for="select-target">Flavor</label>
          <select id="select-target">
            <option value="vanilla">Vanilla</option>
            <option value="chocolate">Dark   Chocolate</option>
          </select>
          <select id="select-multiple" multiple>
            <option value="a">Alpha</option>
            <option value="b">Beta</option>
            <option value="c">Gamma</option>
          </select>
          <form action="about:blank" method="get">
            <input id="press-submit" name="q" value="ok">
          </form>
          <p class="all-text-row">replacement</p>
          <p class="all-inner-row">rendered replacement</p>
        `);
        window.__hover_count = 0;
        window.__hover_trial_count = 0;
        window.__hover_key_events = [];
        window.__focus_calls = 0;
        window.__focus_events = [];
        const focusTarget = document.querySelector('#focus-target');
        focusTarget.focus = function() {
          window.__focus_calls += 1;
          HTMLElement.prototype.focus.call(this);
        };
        focusTarget.addEventListener('focus', () => window.__focus_events.push('focus'));
        focusTarget.addEventListener('blur', () => window.__focus_events.push('blur'));
        window.__press_events = [];
        for (const type of ['keydown', 'keyup']) {
          document.querySelector('#press-target').addEventListener(type, event => {
            window.__press_events.push({
              type,
              key: event.key,
              code: event.code,
              shift: event.shiftKey,
              meta: event.metaKey,
              control: event.ctrlKey,
              repeat: event.repeat,
            });
          });
        }
        window.__set_checked_events = [];
        window.__set_checked_trial_events = [];
        window.__set_checked_radio_events = [];
        window.__select_events = [];
        window.__select_multiple_events = [];
        for (const type of ['click', 'input', 'change']) {
          document.querySelector('#set-checked-target').addEventListener(
            type,
            () => window.__set_checked_events.push(type),
          );
          document.querySelector('#set-checked-trial').addEventListener(
            type,
            () => window.__set_checked_trial_events.push(type),
          );
          document.querySelector('#set-checked-radio').addEventListener(
            type,
            () => window.__set_checked_radio_events.push(type),
          );
        }
        for (const type of ['input', 'change']) {
          document.querySelector('#select-target').addEventListener(
            type,
            event => window.__select_events.push({ type, composed: event.composed }),
          );
          document.querySelector('#select-multiple').addEventListener(
            type,
            event => window.__select_multiple_events.push({ type, composed: event.composed }),
          );
        }
        window.__click_input_events = [];
        for (const type of ['mousemove', 'mousedown', 'mouseup', 'click']) {
          document.querySelector('[data-testid="standard-action"]').addEventListener(
            type,
            () => window.__click_input_events.push(type),
          );
        }
        for (const type of ['keydown', 'keyup']) {
          document.addEventListener(type, event => {
            window.__hover_key_events.push({
              type,
              key: event.key,
              shift: event.shiftKey,
              meta: event.metaKey,
              control: event.ctrlKey,
            });
          });
        }
        document.querySelector('#hover-target').addEventListener('mouseover', event => {
          window.__hover_count += 1;
          const target = event.currentTarget;
          const rect = target.getBoundingClientRect();
          const style = getComputedStyle(target);
          window.__hover = {
            shift: event.shiftKey,
            meta: event.metaKey,
            control: event.ctrlKey,
            xDelta: Math.abs(
              event.clientX - (rect.left + Number.parseFloat(style.borderLeftWidth) + 6.5),
            ),
            yDelta: Math.abs(
              event.clientY - (rect.top + Number.parseFloat(style.borderTopWidth) + 7.25),
            ),
            isMac: /Mac|iPhone|iPad|iPod/.test(navigator.platform),
          };
        });
        document.querySelector('#hover-trial').addEventListener(
          'mouseover',
          () => window.__hover_trial_count += 1,
        );
        true"#,
    )
    .await
    .expect("attribute locator fixture should be installed");
    let card_count_after_navigation = card_count_locator
        .count(CallMetadata::default())
        .await
        .expect("the same lazy locator should snapshot the replacement document");
    let all_text_contents_after_navigation = all_text_contents_locator
        .all_text_contents(CallMetadata::default())
        .await
        .expect("the same lazy collection locator should snapshot the replacement document");
    let all_inner_texts_after_navigation = all_inner_texts_locator
        .all_inner_texts(CallMetadata::default())
        .await
        .expect(
            "the same lazy rendered collection locator should snapshot the replacement document",
        );
    let first_from_prior_all_snapshot = all_locator_list
        .get(0)
        .expect("the earlier four-item snapshot should contain index zero")
        .text_content(CallMetadata::default())
        .await
        .expect("an nth locator returned before navigation should resolve lazily afterward");
    let visible_card_after_navigation = visible_card_locator
        .is_visible(CallMetadata::default())
        .await
        .expect("the same visibility locator should resolve in the replacement document");
    label_locator
        .click(ClickOptions::new(), CallMetadata::default())
        .await
        .expect("label locator should resolve its labelled control");
    text_locator
        .click(ClickOptions::new(), CallMetadata::default())
        .await
        .expect("text locator should resolve its smallest exact match");
    placeholder_locator
        .click(ClickOptions::new(), CallMetadata::default())
        .await
        .expect("placeholder exact match should preserve raw whitespace");
    alt_locator
        .click(ClickOptions::new(), CallMetadata::default())
        .await
        .expect("alt regex should match the raw attribute value");
    title_locator
        .click(ClickOptions::new(), CallMetadata::default())
        .await
        .expect("title substring should match case-insensitively");
    standard_test_id_locator
        .click(ClickOptions::new(), CallMetadata::default())
        .await
        .expect("standard data-testid selector should resolve");
    custom_test_id_locator
        .click(ClickOptions::new(), CallMetadata::default())
        .await
        .expect("explicit multi-name test-id regex should resolve");
    hover_target_locator
        .hover(
            HoverOptions::new()
                .with_scroll(ScrollMode::None)
                .with_modifier(KeyboardModifier::ControlOrMeta)
                .with_modifier(KeyboardModifier::Shift)
                .with_position(
                    Position::new(6.5, 7.25).expect("finite hover position should build"),
                ),
            CallMetadata::default(),
        )
        .await
        .expect("disabled target should receive a positioned hover with modifiers");
    page.runtime_evaluate(
        "window.__hover_keys = window.__hover_key_events; window.__hover_key_events = []; true",
    )
    .await
    .expect("ordinary hover key events should be snapshotted");
    hover_trial_locator
        .hover(
            HoverOptions::new()
                .with_modifier(KeyboardModifier::Shift)
                .with_trial(true),
            CallMetadata::default(),
        )
        .await
        .expect("trial hover should run actionability without moving the pointer");
    focus_target_locator
        .focus(CallMetadata::default())
        .await
        .expect("focus should call the target's native focus path twice");
    let focused = page
        .runtime_evaluate(
            "({ active: document.activeElement.id, calls: window.__focus_calls, events: [...window.__focus_events] })",
        )
        .await
        .expect("focused state should be readable");
    focus_target_locator
        .blur(CallMetadata::default())
        .await
        .expect("focused target should blur");
    focus_target_locator
        .blur(CallMetadata::default())
        .await
        .expect("already blurred target should be a successful no-op");
    non_focusable_locator
        .focus(CallMetadata::default())
        .await
        .expect("non-focusable element should preserve native no-op success");
    let blurred = page
        .runtime_evaluate(
            "({ active: document.activeElement.id, events: [...window.__focus_events] })",
        )
        .await
        .expect("blurred state should be readable");
    let strict_focus_error = strict_focus_locator
        .focus(CallMetadata::default())
        .await
        .expect_err("focus should remain strict for multiple matches");
    page.runtime_evaluate(
        "window.__hover_trial_keys = window.__hover_key_events; window.__hover_key_events = []; true",
    )
    .await
    .expect("trial hover key events should be snapshotted before Press");
    let press_options = PressOptions::new().with_no_wait_after(true);
    press_target_locator
        .press(
            PressKey::new("ArrowRight").expect("ArrowRight should validate"),
            press_options,
            CallMetadata::default(),
        )
        .await
        .expect("first press should reset selection before moving right");
    let selection_after_first_press = page
        .runtime_evaluate("document.querySelector('#press-target').selectionStart")
        .await
        .expect("selection after first press should be readable");
    press_target_locator
        .press(
            PressKey::new("x").expect("x should validate"),
            press_options,
            CallMetadata::default(),
        )
        .await
        .expect("printable key should insert at the current selection");
    press_target_locator
        .press(
            PressKey::new("Shift+KeyA").expect("shifted chord should validate"),
            press_options,
            CallMetadata::default(),
        )
        .await
        .expect("shifted chord should insert uppercase text");
    press_target_locator
        .press(
            PressKey::new("ControlOrMeta+KeyA").expect("smart shortcut should validate"),
            press_options,
            CallMetadata::default(),
        )
        .await
        .expect("platform shortcut should select the input contents");
    press_target_locator
        .press(
            PressKey::new("b").expect("b should validate"),
            press_options,
            CallMetadata::default(),
        )
        .await
        .expect("typing after select-all should replace the input value");
    let pressed = page
        .runtime_evaluate(
            "({ value: document.querySelector('#press-target').value, active: document.activeElement.id, events: window.__press_events })",
        )
        .await
        .expect("press state should be readable");
    let hovered = page
        .runtime_evaluate(
            "({ hover: window.__hover, count: window.__hover_count, trialCount: window.__hover_trial_count, keys: window.__hover_keys, trialKeys: window.__hover_trial_keys })",
        )
        .await
        .expect("hover state should be readable");
    let second_clicked = page
        .runtime_evaluate(
            "({ label: window.__label_clicked === true, text: window.__text_clicked === true, placeholder: window.__placeholder_clicked === true, wrongNormalizedPlaceholder: window.__wrong_normalized_placeholder === true, alt: window.__alt_clicked === true, title: window.__title_clicked === true, standardTestId: window.__standard_test_id_clicked === true, customTestId: window.__custom_test_id_clicked === true, clickInputEvents: window.__click_input_events })",
        )
        .await
        .expect("click state should be readable");
    set_checked_locator
        .check(SetCheckedOptions::new(), CallMetadata::default())
        .await
        .expect("Check should click an associated label to select its checkbox");
    set_checked_locator
        .check(SetCheckedOptions::new(), CallMetadata::default())
        .await
        .expect("Check should be idempotent when the checkbox already matches");
    set_checked_locator
        .uncheck(SetCheckedOptions::new(), CallMetadata::default())
        .await
        .expect("Uncheck should click the associated label once");
    set_checked_trial_locator
        .check(
            SetCheckedOptions::new().with_trial(true),
            CallMetadata::default(),
        )
        .await
        .expect("trial Check should complete actionability without input");
    let radio_uncheck_error = set_checked_radio_locator
        .uncheck(SetCheckedOptions::new(), CallMetadata::default())
        .await
        .expect_err("a selected radio must not be unchecked directly");
    let set_checked = page
        .runtime_evaluate(
            "({ checked: document.querySelector('#set-checked-target').checked, events: window.__set_checked_events, trialChecked: document.querySelector('#set-checked-trial').checked, trialEvents: window.__set_checked_trial_events, radioChecked: document.querySelector('#set-checked-radio').checked, radioEvents: window.__set_checked_radio_events })",
        )
        .await
        .expect("SetChecked state and native events should be readable");
    let selected_by_normalized_label = select_label_locator
        .select_option(
            "Dark Chocolate",
            SelectOptionOptions::new(),
            CallMetadata::default(),
        )
        .await
        .expect("string selection should follow a label and normalize option text");
    let selected_in_dom_order = select_multiple_locator
        .select_option(
            [
                SelectOptionValue::new().with_value("c"),
                SelectOptionValue::new().with_index(0),
            ],
            SelectOptionOptions::new(),
            CallMetadata::default(),
        )
        .await
        .expect("descriptor selection should return matching values in DOM order");
    let selected_state = page
        .runtime_evaluate(
            "({ single: document.querySelector('#select-target').value, multiple: [...document.querySelector('#select-multiple').selectedOptions].map(option => option.value), singleEvents: window.__select_events, multipleEvents: window.__select_multiple_events })",
        )
        .await
        .expect("selected option state and events should be readable");
    let deselected = select_multiple_locator
        .select_option(
            SelectOptionValues::new(),
            SelectOptionOptions::new(),
            CallMetadata::default(),
        )
        .await
        .expect("an empty value list should deselect all options");
    let deselected_state = page
        .runtime_evaluate(
            "[...document.querySelector('#select-multiple').selectedOptions].map(option => option.value)",
        )
        .await
        .expect("deselected option state should be readable");
    page.runtime_evaluate(
        r#"(() => {
          document.body.insertAdjacentHTML('beforeend', `
            <label>Email <input id="email" readonly
              oninput="window.__email_value=this.value"></label>
            <div id="editor" contenteditable
              oninput="window.__editor_value=this.textContent">old</div>
            <input id="date" type="date"
              oninput="window.__date_input=(window.__date_input||0)+1"
              onchange="window.__date_change=(window.__date_change||0)+1">
            <input id="clear-target" value="seed"
              oninput="window.__clear_value=this.value">
          `);
          setTimeout(() => document.querySelector('#email').readOnly = false, 75);
          return true;
        })()"#,
    )
    .await
    .expect("fillable controls should be installed");
    typed_page
        .get_by_text("Email", true)
        .fill(
            "user@example.com",
            FillOptions::new(),
            CallMetadata::default(),
        )
        .await
        .expect("text locator should retarget its label and wait until editable");
    typed_page
        .locator(CssSelector::new("#editor").expect("valid editor CSS"))
        .fill("new content", FillOptions::new(), CallMetadata::default())
        .await
        .expect("contenteditable should receive inserted text");
    typed_page
        .locator(CssSelector::new("#date").expect("valid date CSS"))
        .fill("2026-09-02", FillOptions::new(), CallMetadata::default())
        .await
        .expect("date input should use direct constrained-value fill");
    typed_page
        .locator(CssSelector::new("#clear-target").expect("valid clear CSS"))
        .clear(FillOptions::new(), CallMetadata::default())
        .await
        .expect("clear should reuse fill with an empty value");
    let filled = page
        .runtime_evaluate(
            "({ email: document.querySelector('#email').value, emailEvent: window.__email_value, editor: document.querySelector('#editor').textContent, editorEvent: window.__editor_value, date: document.querySelector('#date').value, dateInput: window.__date_input, dateChange: window.__date_change, clear: document.querySelector('#clear-target').value, clearEvent: window.__clear_value })",
        )
        .await
        .expect("fill state should be readable");
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        press_submit_locator.press(
            PressKey::new("Enter").expect("Enter should validate"),
            PressOptions::new(),
            CallMetadata::default(),
        ),
    )
    .await
    .expect("Enter submission should settle its navigation barrier")
    .expect("Enter should submit the form");
    let submitted_url = page
        .runtime_evaluate("location.href")
        .await
        .expect("submitted URL should be readable");
    let pending_call_count = client.channel().pending_call_count();
    drop(text_locator);
    drop(label_locator);
    drop(card_count_locator);
    drop(all_locator_list);
    drop(all_inner_texts_locator);
    drop(invalid_all_inner_texts_locator);
    drop(all_text_contents_locator);
    drop(hidden_card_locator);
    drop(visible_card_locator);
    drop(missing_visibility_locator);
    drop(strict_visibility_locator);
    drop(late_enabled_locator);
    drop(wait_cycle_locator);
    drop(text_content_locator);
    drop(inner_html_locator);
    drop(svg_inner_html_locator);
    drop(inner_text_locator);
    drop(svg_inner_text_locator);
    drop(delayed_label_value_locator);
    drop(textarea_value_locator);
    drop(select_value_locator);
    drop(invalid_value_locator);
    drop(strict_value_locator);
    drop(invalid_checked_locator);
    drop(checked_state_locator);
    drop(editable_state_locator);
    drop(disabled_state_locator);
    drop(filtered_locator);
    drop(invalid_regex_locator);
    drop(custom_test_id_locator);
    drop(standard_test_id_locator);
    drop(title_locator);
    drop(alt_locator);
    drop(placeholder_locator);
    drop(hover_trial_locator);
    drop(hover_target_locator);
    drop(strict_focus_locator);
    drop(press_submit_locator);
    drop(press_target_locator);
    drop(set_checked_radio_locator);
    drop(set_checked_trial_locator);
    drop(set_checked_locator);
    drop(select_multiple_locator);
    drop(select_label_locator);
    drop(non_focusable_locator);
    drop(focus_target_locator);
    drop(locator);
    drop(typed_page);
    drop(context);
    drop(browser);
    drop(client);
    dispose_context_and_verify_target(&browser_session, &browser_context_id, &target_id).await;

    // Assert
    assert_eq!(initial_card_count, 4);
    assert_eq!(
        initial_all_text_contents,
        ["first nested", "", "hidden", "svg"]
    );
    assert_eq!(initial_all_inner_texts, ["Visible text", "", "third"]);
    assert_eq!(all_locator_list_len, 4);
    assert_eq!(
        invalid_all_inner_texts_error.details(),
        Some(&LocatorErrorDetails::InvalidTarget(
            "Node is not an HTMLElement".to_string()
        ))
    );
    assert_eq!(card_count_after_navigation, 0);
    assert_eq!(all_text_contents_after_navigation, ["replacement"]);
    assert_eq!(all_inner_texts_after_navigation, ["rendered replacement"]);
    assert_eq!(
        first_from_prior_all_snapshot.as_deref(),
        Some("replacement")
    );
    assert!(hidden_card);
    assert!(visible_card);
    assert!(!missing_visibility);
    assert!(!visible_card_after_navigation);
    assert!(disabled_state);
    assert!(editable_state);
    assert!(checked_state);
    assert!(late_enabled);
    assert_eq!(exact_text_content.as_deref(), Some("  Ready\nnow  "));
    assert_eq!(
        serialized_inner_html,
        "<span data-state=\"ready\">Text &amp; more</span><!--note-->"
    );
    assert_eq!(svg_inner_html, "<text>SVG &amp; text</text>");
    assert_eq!(attribute.as_deref(), Some("ready"));
    assert_eq!(missing_attribute, None);
    assert_eq!(svg_attribute.as_deref(), Some("vector"));
    assert_eq!(
        strict_attribute_error.details(),
        Some(&LocatorErrorDetails::StrictViolation {
            selector: ".card".to_string(),
            matches: 4,
        })
    );
    assert_eq!(
        strict_inner_html_error.details(),
        Some(&LocatorErrorDetails::StrictViolation {
            selector: ".card".to_string(),
            matches: 4,
        })
    );
    assert_eq!(rendered_inner_text, "Visible text");
    assert_eq!(label_input_value, "alice & exact");
    assert_eq!(textarea_input_value, "line one\nline two");
    assert_eq!(select_input_value, "chosen");
    assert_eq!(
        invalid_input_value_error.details(),
        Some(&LocatorErrorDetails::InvalidTarget(
            "Node is not an <input>, <textarea> or <select> element".to_string()
        ))
    );
    assert_eq!(
        strict_input_value_error.details(),
        Some(&LocatorErrorDetails::StrictViolation {
            selector: ".input-value-duplicate".to_string(),
            matches: 2,
        })
    );
    assert_eq!(
        raw_inner_text_content.as_deref(),
        Some("Visiblehidden text")
    );
    assert_eq!(
        svg_inner_text_error.details(),
        Some(&LocatorErrorDetails::InvalidTarget(
            "Node is not an HTMLElement".to_string()
        ))
    );
    assert_eq!(
        strict_text_content_error.details(),
        Some(&LocatorErrorDetails::StrictViolation {
            selector: ".card".to_string(),
            matches: 4,
        })
    );
    assert_eq!(
        strict_wait_error.details(),
        Some(&LocatorErrorDetails::StrictViolation {
            selector: ".card".to_string(),
            matches: 4,
        })
    );
    assert_eq!(
        invalid_checked_error.details(),
        Some(&LocatorErrorDetails::InvalidTarget(
            "Not a checkbox or radio button".to_string()
        ))
    );
    assert_eq!(
        strict_visibility_error.details(),
        Some(&LocatorErrorDetails::StrictViolation {
            selector: ".card".to_string(),
            matches: 4,
        })
    );
    assert_eq!(
        first_clicked["result"]["value"],
        serde_json::json!({
            "role": true,
            "outside": false,
            "filtered": true,
            "wrongHidden": false,
            "wrongExcluded": false,
            "wrongBlocked": false,
        })
    );
    assert!(matches!(
        invalid_regex_error.details(),
        Some(LocatorErrorDetails::InvalidSelector(_))
    ));
    assert_eq!(
        second_clicked["result"]["value"],
        serde_json::json!({
            "label": true,
            "text": true,
            "placeholder": true,
            "wrongNormalizedPlaceholder": false,
            "alt": true,
            "title": true,
            "standardTestId": true,
            "customTestId": true,
            "clickInputEvents": ["mousemove", "mousedown", "mouseup", "click"],
        })
    );
    assert_eq!(
        set_checked["result"]["value"],
        serde_json::json!({
            "checked": false,
            "events": ["click", "input", "change", "click", "input", "change"],
            "trialChecked": false,
            "trialEvents": [],
            "radioChecked": true,
            "radioEvents": [],
        })
    );
    assert_eq!(radio_uncheck_error.kind(), FailureKind::Backend);
    assert!(
        radio_uncheck_error
            .message()
            .contains("Cannot uncheck radio button")
    );
    assert_eq!(selected_by_normalized_label, ["chocolate"]);
    assert_eq!(selected_in_dom_order, ["a", "c"]);
    assert!(deselected.is_empty());
    assert_eq!(deselected_state["result"]["value"], serde_json::json!([]));
    assert_eq!(
        selected_state["result"]["value"],
        serde_json::json!({
            "single": "chocolate",
            "multiple": ["a", "c"],
            "singleEvents": [
                {"type": "input", "composed": true},
                {"type": "change", "composed": false},
            ],
            "multipleEvents": [
                {"type": "input", "composed": true},
                {"type": "change", "composed": false},
            ],
        })
    );
    let hovered = &hovered["result"]["value"];
    assert_eq!(hovered["count"], 1);
    assert_eq!(hovered["trialCount"], 0);
    assert_eq!(
        focused["result"]["value"],
        serde_json::json!({
            "active": "focus-target",
            "calls": 2,
            "events": ["focus"],
        })
    );
    assert_eq!(
        blurred["result"]["value"],
        serde_json::json!({
            "active": "",
            "events": ["focus", "blur"],
        })
    );
    assert_eq!(
        strict_focus_error.details(),
        Some(&LocatorErrorDetails::StrictViolation {
            selector: ".focus-duplicate".to_string(),
            matches: 2,
        })
    );
    assert_eq!(selection_after_first_press["result"]["value"], 1);
    let pressed = &pressed["result"]["value"];
    assert_eq!(pressed["value"], "b");
    assert_eq!(pressed["active"], "press-target");
    let press_events = pressed["events"]
        .as_array()
        .expect("press events should be recorded");
    assert_eq!(
        press_events.first().and_then(|event| event["key"].as_str()),
        Some("ArrowRight")
    );
    assert!(
        press_events
            .iter()
            .any(|event| event["key"] == "A" && event["shift"] == true)
    );
    assert!(press_events.iter().all(|event| event["repeat"] == false));
    assert_eq!(
        press_events.last().and_then(|event| event["key"].as_str()),
        Some("b")
    );
    assert_eq!(hovered["hover"]["shift"], true);
    let keys = hovered["keys"]
        .as_array()
        .expect("ordinary hover should record key events");
    assert_eq!(keys.len(), 4);
    assert_eq!(keys[0]["type"], "keydown");
    assert_eq!(keys[1]["type"], "keydown");
    assert_eq!(keys[1]["key"], "Shift");
    assert_eq!(keys[2]["type"], "keyup");
    assert_eq!(keys[3]["type"], "keyup");
    assert_eq!(keys[3]["key"], "Shift");
    assert_eq!(keys[3]["shift"], false);
    assert_eq!(keys[3]["meta"], false);
    assert_eq!(keys[3]["control"], false);
    let trial_keys = hovered["trialKeys"]
        .as_array()
        .expect("trial should record modifier transitions");
    assert_eq!(trial_keys.len(), 2);
    assert_eq!(trial_keys[0]["type"], "keydown");
    assert_eq!(trial_keys[0]["key"], "Shift");
    assert_eq!(trial_keys[1]["type"], "keyup");
    assert_eq!(trial_keys[1]["key"], "Shift");
    assert_eq!(trial_keys[1]["shift"], false);
    assert!(hovered["hover"]["xDelta"].as_f64().unwrap_or(f64::MAX) <= 0.5);
    assert!(hovered["hover"]["yDelta"].as_f64().unwrap_or(f64::MAX) <= 0.5);
    if hovered["hover"]["isMac"] == true {
        assert_eq!(keys[0]["key"], "Meta");
        assert_eq!(keys[2]["key"], "Meta");
        assert_eq!(hovered["hover"]["meta"], true);
        assert_eq!(hovered["hover"]["control"], false);
    } else {
        assert_eq!(keys[0]["key"], "Control");
        assert_eq!(keys[2]["key"], "Control");
        assert_eq!(hovered["hover"]["meta"], false);
        assert_eq!(hovered["hover"]["control"], true);
    }
    assert_eq!(
        filled["result"]["value"],
        serde_json::json!({
            "email": "user@example.com",
            "emailEvent": "user@example.com",
            "editor": "new content",
            "editorEvent": "new content",
            "date": "2026-09-02",
            "dateInput": 1,
            "dateChange": 1,
            "clear": "",
            "clearEvent": "",
        })
    );
    let submitted_url = submitted_url["result"]["value"]
        .as_str()
        .expect("submitted location should be a string");
    assert!(submitted_url.starts_with("about:blank"));
    assert!(submitted_url.contains("q=ok"));
    assert_eq!(pending_call_count, Ok(0));
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("external browser should remain alive after typed client drop");
}
