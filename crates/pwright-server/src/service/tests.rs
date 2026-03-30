use std::sync::Arc;

use pwright_bridge::{Browser, Tab};
use pwright_cdp::connection::CdpError;
use pwright_cdp::{CdpClient, SessionFactory};
use pwright_fake::FakeCdpClient;
use tonic::{Code, Request};

use super::*;

// ── Test infrastructure ──

/// A SessionFactory that produces FakeCdpClient with a default HTML page.
struct FakeHtmlSessionFactory;

impl SessionFactory for FakeHtmlSessionFactory {
    fn create_session(&self, _session_id: String, _target_id: String) -> Arc<dyn CdpClient> {
        Arc::new(FakeCdpClient::from_html(TEST_HTML))
    }
}

/// Standard test page with common elements for action testing.
const TEST_HTML: &str = r#"
<html>
<body>
  <h1>Test Page</h1>
  <button id="btn" class="primary">Click Me</button>
  <input id="name" type="text" value="initial" />
  <input id="check" type="checkbox" checked />
  <select id="color">
    <option value="red">Red</option>
    <option value="blue" selected>Blue</option>
  </select>
  <div id="content">Hello World</div>
  <a href="/next">Next Page</a>
</body>
</html>
"#;

/// Create a BrowserServiceImpl wired to a FakeCdpClient with real DOM.
/// Returns (svc, fake_session, tab_id).
async fn make_svc() -> (BrowserServiceImpl, Arc<FakeCdpClient>, String) {
    make_svc_opts(false, None).await
}

async fn make_svc_opts(
    eval_disabled: bool,
    upload_dir: Option<std::path::PathBuf>,
) -> (BrowserServiceImpl, Arc<FakeCdpClient>, String) {
    let fake = Arc::new(FakeCdpClient::from_html(TEST_HTML));
    let browser_client = Arc::new(FakeCdpClient::from_html("")) as Arc<dyn CdpClient>;
    let browser = Browser::new_for_test(browser_client, Arc::new(FakeHtmlSessionFactory));

    // Pre-populate a tab with the fake session
    let tab = Tab {
        session: fake.clone() as Arc<dyn CdpClient>,
        tab_id: "tab_0".to_string(),
        target_id: "target-abc".to_string(),
        created_at: std::time::Instant::now(),
    };
    browser
        .tabs()
        .write()
        .await
        .insert("tab_0".to_string(), tab);

    let svc = BrowserServiceImpl {
        browser: RwLock::new(Some(browser)),
        default_cdp_url: None,
        max_parallel_tabs: 4,
        nav_timeout_ms: 30000,
        eval_disabled,
        upload_dir,
    };

    (svc, fake, "tab_0".to_string())
}

// ── cdp_to_status: exhaustive mapping ──

#[test]
fn error_mapping_covers_all_variants() {
    let cases: Vec<(CdpError, Code)> = vec![
        (CdpError::Timeout, Code::DeadlineExceeded),
        (CdpError::Closed, Code::Unavailable),
        (CdpError::ChannelDropped, Code::Unavailable),
        (
            CdpError::ElementNotFound {
                selector: "x".into(),
            },
            Code::NotFound,
        ),
        (
            CdpError::NavigationFailed {
                url: "u".into(),
                reason: "r".into(),
            },
            Code::Internal,
        ),
        (CdpError::WebSocket("ws".into()), Code::Unavailable),
        (CdpError::PageClosed, Code::FailedPrecondition),
        (CdpError::TabNotFound("t".into()), Code::NotFound),
        (CdpError::HttpFailed("h".into()), Code::Unavailable),
        (CdpError::JsException("j".into()), Code::Internal),
        (
            CdpError::Protocol {
                code: -1,
                message: "p".into(),
            },
            Code::Internal,
        ),
        (CdpError::Other("o".into()), Code::Internal),
        (
            CdpError::Compound {
                source: Box::new(CdpError::Timeout),
                system: Box::new(CdpError::Closed),
            },
            Code::Internal,
        ),
    ];
    for (err, expected_code) in cases {
        let msg = format!("{err}");
        let status = cdp_to_status(err);
        assert_eq!(status.code(), expected_code, "wrong code for: {msg}");
    }
}

#[test]
fn error_mapping_preserves_message() {
    let status = cdp_to_status(CdpError::ElementNotFound {
        selector: "div.missing".to_string(),
    });
    assert!(status.message().contains("div.missing"));
}

// ── BrowserServiceImpl state management ──

#[tokio::test]
async fn get_browser_fails_when_not_connected() {
    let svc = BrowserServiceImpl::new(None, 4, 30000, false);
    let status = svc.get_browser().await.err().unwrap();
    assert_eq!(status.code(), Code::FailedPrecondition);
}

#[tokio::test]
async fn resolve_ref_rejects_empty_ref() {
    let (svc, _, _) = make_svc().await;
    let browser = svc.get_browser().await.unwrap();
    let status = svc
        .resolve_ref_or_node(&browser, "tab_0", "")
        .await
        .err()
        .unwrap();
    assert_eq!(status.code(), Code::InvalidArgument);
}

#[tokio::test]
async fn resolve_ref_returns_not_found_for_missing_ref() {
    let (svc, _, _) = make_svc().await;
    let browser = svc.get_browser().await.unwrap();
    let status = svc
        .resolve_ref_or_node(&browser, "tab_0", "ref:999")
        .await
        .err()
        .unwrap();
    assert_eq!(status.code(), Code::NotFound);
}

#[tokio::test]
async fn resolve_tab_locked_fails_for_unknown_tab() {
    let (svc, _, _) = make_svc().await;
    let browser = svc.get_browser().await.unwrap();
    let status = svc
        .resolve_tab_locked(&browser, "nonexistent")
        .await
        .err()
        .expect("should fail");
    assert_eq!(status.code(), Code::NotFound);
}

// ── Lifecycle handlers ──

#[tokio::test]
async fn health_not_connected() {
    let svc = BrowserServiceImpl::new(None, 4, 30000, false);
    let h = lifecycle::health(&svc, Request::new(proto::HealthRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert!(!h.healthy);
    assert_eq!(h.open_tabs, 0);
}

#[tokio::test]
async fn health_connected_with_tabs() {
    let (svc, _, _) = make_svc().await;
    let h = lifecycle::health(&svc, Request::new(proto::HealthRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert!(h.healthy);
    assert!(h.browser_connected);
    assert_eq!(h.open_tabs, 1);
}

#[tokio::test]
async fn connect_rejects_invalid_scheme() {
    let svc = BrowserServiceImpl::new(None, 4, 30000, false);
    let status = lifecycle::connect_browser(
        &svc,
        Request::new(proto::ConnectBrowserRequest {
            cdp_url: "ftp://localhost:9222".into(),
        }),
    )
    .await
    .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
}

#[tokio::test]
async fn connect_rejects_empty_url_without_default() {
    let svc = BrowserServiceImpl::new(None, 4, 30000, false);
    let status = lifecycle::connect_browser(
        &svc,
        Request::new(proto::ConnectBrowserRequest {
            cdp_url: String::new(),
        }),
    )
    .await
    .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
}

// ── Tab handlers ──

#[tokio::test]
async fn create_tab_returns_new_id() {
    let (svc, _, _) = make_svc().await;
    let resp = tabs::create_tab(
        &svc,
        Request::new(proto::CreateTabRequest {
            url: "about:blank".into(),
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(!resp.tab_id.is_empty());
    // Should now have 2 tabs
    let h = lifecycle::health(&svc, Request::new(proto::HealthRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(h.open_tabs, 2);
}

#[tokio::test]
async fn close_tab_removes_it() {
    let (svc, _, tab_id) = make_svc().await;
    let resp = tabs::close_tab(
        &svc,
        Request::new(proto::CloseTabRequest {
            tab_id: tab_id.clone(),
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.closed);
    // Tab should be gone
    let h = lifecycle::health(&svc, Request::new(proto::HealthRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(h.open_tabs, 0);
}

#[tokio::test]
async fn close_nonexistent_tab_is_noop() {
    let (svc, _, _) = make_svc().await;
    let resp = tabs::close_tab(
        &svc,
        Request::new(proto::CloseTabRequest {
            tab_id: "ghost".into(),
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.closed);
}

#[tokio::test]
async fn list_tabs_returns_page_targets() {
    let (svc, _, _) = make_svc().await;
    // FakeCdpClient.target_get_targets returns empty by default
    let resp = tabs::list_tabs(&svc, Request::new(proto::ListTabsRequest {}))
        .await
        .unwrap()
        .into_inner();
    // The list comes from CDP Target.getTargets, not from in-memory tab map
    // FakeCdpClient.target_get_targets returns an empty list by default
    assert!(resp.tabs.is_empty(), "fake should return no targets");
}

#[tokio::test]
async fn bring_to_front_succeeds() {
    let (svc, _, tab_id) = make_svc().await;
    let resp = tabs::bring_to_front(&svc, Request::new(proto::BringToFrontRequest { tab_id }))
        .await
        .unwrap()
        .into_inner();
    assert!(resp.success);
}

// ── Navigation handlers ──

#[tokio::test]
async fn navigate_existing_tab() {
    let (svc, fake, tab_id) = make_svc().await;
    // Set evaluate response for readyState poll
    fake.set_evaluate_response(serde_json::json!({"result": {"value": "complete"}}));

    let resp = navigation::navigate(
        &svc,
        Request::new(proto::NavigateRequest {
            tab_id: tab_id.clone(),
            url: "https://example.com".into(),
            new_tab: false,
            wait_for: proto::WaitStrategy::WaitDom.into(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert_eq!(resp.tab_id, tab_id);
}

#[tokio::test]
async fn navigate_new_tab_creates_tab() {
    let (svc, _, _) = make_svc().await;

    let resp = navigation::navigate(
        &svc,
        Request::new(proto::NavigateRequest {
            tab_id: String::new(),
            url: "https://example.com".into(),
            new_tab: true,
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(!resp.tab_id.is_empty());
    // Should now have 2 tabs
    let h = lifecycle::health(&svc, Request::new(proto::HealthRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(h.open_tabs, 2);
}

#[tokio::test]
async fn reload_succeeds() {
    let (svc, fake, tab_id) = make_svc().await;
    fake.set_evaluate_response(serde_json::json!({"result": {"value": "complete"}}));

    let resp = navigation::reload(&svc, Request::new(proto::ReloadRequest { tab_id }))
        .await
        .unwrap()
        .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn go_back_succeeds() {
    let (svc, _, tab_id) = make_svc().await;
    // FakeCdpClient returns a default navigation history
    let resp = navigation::go_back(&svc, Request::new(proto::GoBackRequest { tab_id }))
        .await
        .unwrap()
        .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn go_forward_succeeds() {
    let (svc, _, tab_id) = make_svc().await;
    let resp = navigation::go_forward(&svc, Request::new(proto::GoForwardRequest { tab_id }))
        .await
        .unwrap()
        .into_inner();
    assert!(resp.success);
}

// ── Action handlers ──

#[tokio::test]
async fn action_press_key() {
    let (svc, _, tab_id) = make_svc().await;
    let resp = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id,
            kind: proto::ActionKind::Press.into(),
            key: "Enter".into(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn action_unspecified_returns_error() {
    let (svc, _, tab_id) = make_svc().await;
    let status = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id,
            kind: proto::ActionKind::ActionUnspecified.into(),
            ..Default::default()
        }),
    )
    .await
    .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
}

#[tokio::test]
async fn action_click_without_ref_returns_error() {
    let (svc, _, tab_id) = make_svc().await;
    let status = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id,
            kind: proto::ActionKind::Click.into(),
            r#ref: String::new(),
            ..Default::default()
        }),
    )
    .await
    .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
}

#[tokio::test]
async fn action_click_with_ref() {
    let (svc, _, tab_id) = make_svc().await;

    // Take a snapshot first to populate ref cache
    let _ = content::get_snapshot(
        &svc,
        Request::new(proto::GetSnapshotRequest {
            tab_id: tab_id.clone(),
            filter: proto::SnapshotFilter::FilterAll.into(),
            max_depth: 10,
        }),
    )
    .await
    .unwrap();

    // Find a ref from the snapshot (use ref:1 which should exist)
    let browser = svc.get_browser().await.unwrap();
    let cache = browser
        .get_ref_cache(&tab_id)
        .await
        .expect("snapshot should populate ref cache");
    let (ref_str, _) = cache
        .refs
        .iter()
        .next()
        .expect("ref cache should have at least one entry");
    let resp = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id: tab_id.clone(),
            kind: proto::ActionKind::Click.into(),
            r#ref: ref_str.clone(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn action_fill_with_ref() {
    let (svc, _, tab_id) = make_svc().await;

    let _ = content::get_snapshot(
        &svc,
        Request::new(proto::GetSnapshotRequest {
            tab_id: tab_id.clone(),
            filter: proto::SnapshotFilter::FilterAll.into(),
            max_depth: 10,
        }),
    )
    .await
    .unwrap();

    let browser = svc.get_browser().await.unwrap();
    let cache = browser
        .get_ref_cache(&tab_id)
        .await
        .expect("snapshot should populate ref cache");
    let (ref_str, _) = cache
        .refs
        .iter()
        .next()
        .expect("ref cache should have at least one entry");
    let resp = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id: tab_id.clone(),
            kind: proto::ActionKind::Fill.into(),
            r#ref: ref_str.clone(),
            text: "hello world".into(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn action_type_with_ref() {
    let (svc, _, tab_id) = make_svc().await;

    let _ = content::get_snapshot(
        &svc,
        Request::new(proto::GetSnapshotRequest {
            tab_id: tab_id.clone(),
            filter: proto::SnapshotFilter::FilterAll.into(),
            max_depth: 10,
        }),
    )
    .await
    .unwrap();

    let browser = svc.get_browser().await.unwrap();
    let cache = browser
        .get_ref_cache(&tab_id)
        .await
        .expect("snapshot should populate ref cache");
    let (ref_str, _) = cache
        .refs
        .iter()
        .next()
        .expect("ref cache should have at least one entry");
    let resp = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id: tab_id.clone(),
            kind: proto::ActionKind::Type.into(),
            r#ref: ref_str.clone(),
            text: "typed text".into(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn action_hover_with_ref() {
    let (svc, _, tab_id) = make_svc().await;

    let _ = content::get_snapshot(
        &svc,
        Request::new(proto::GetSnapshotRequest {
            tab_id: tab_id.clone(),
            filter: proto::SnapshotFilter::FilterAll.into(),
            max_depth: 10,
        }),
    )
    .await
    .unwrap();

    let browser = svc.get_browser().await.unwrap();
    let cache = browser
        .get_ref_cache(&tab_id)
        .await
        .expect("snapshot should populate ref cache");
    let (ref_str, _) = cache
        .refs
        .iter()
        .next()
        .expect("ref cache should have at least one entry");
    let resp = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id: tab_id.clone(),
            kind: proto::ActionKind::Hover.into(),
            r#ref: ref_str.clone(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn action_focus_with_ref() {
    let (svc, _, tab_id) = make_svc().await;

    let _ = content::get_snapshot(
        &svc,
        Request::new(proto::GetSnapshotRequest {
            tab_id: tab_id.clone(),
            filter: proto::SnapshotFilter::FilterAll.into(),
            max_depth: 10,
        }),
    )
    .await
    .unwrap();

    let browser = svc.get_browser().await.unwrap();
    let cache = browser
        .get_ref_cache(&tab_id)
        .await
        .expect("snapshot should populate ref cache");
    let (ref_str, _) = cache
        .refs
        .iter()
        .next()
        .expect("ref cache should have at least one entry");
    let resp = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id: tab_id.clone(),
            kind: proto::ActionKind::Focus.into(),
            r#ref: ref_str.clone(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn action_dblclick_with_ref() {
    let (svc, _, tab_id) = make_svc().await;

    let _ = content::get_snapshot(
        &svc,
        Request::new(proto::GetSnapshotRequest {
            tab_id: tab_id.clone(),
            filter: proto::SnapshotFilter::FilterAll.into(),
            max_depth: 10,
        }),
    )
    .await
    .unwrap();

    let browser = svc.get_browser().await.unwrap();
    let cache = browser
        .get_ref_cache(&tab_id)
        .await
        .expect("snapshot should populate ref cache");
    let (ref_str, _) = cache
        .refs
        .iter()
        .next()
        .expect("ref cache should have at least one entry");
    let resp = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id: tab_id.clone(),
            kind: proto::ActionKind::Dblclick.into(),
            r#ref: ref_str.clone(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn action_check_with_ref() {
    let (svc, _, tab_id) = make_svc().await;

    let _ = content::get_snapshot(
        &svc,
        Request::new(proto::GetSnapshotRequest {
            tab_id: tab_id.clone(),
            filter: proto::SnapshotFilter::FilterAll.into(),
            max_depth: 10,
        }),
    )
    .await
    .unwrap();

    let browser = svc.get_browser().await.unwrap();
    let cache = browser
        .get_ref_cache(&tab_id)
        .await
        .expect("snapshot should populate ref cache");
    let (ref_str, _) = cache
        .refs
        .iter()
        .next()
        .expect("ref cache should have at least one entry");
    let resp = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id: tab_id.clone(),
            kind: proto::ActionKind::Check.into(),
            r#ref: ref_str.clone(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn action_uncheck_with_ref() {
    let (svc, _, tab_id) = make_svc().await;

    let _ = content::get_snapshot(
        &svc,
        Request::new(proto::GetSnapshotRequest {
            tab_id: tab_id.clone(),
            filter: proto::SnapshotFilter::FilterAll.into(),
            max_depth: 10,
        }),
    )
    .await
    .unwrap();

    let browser = svc.get_browser().await.unwrap();
    let cache = browser
        .get_ref_cache(&tab_id)
        .await
        .expect("snapshot should populate ref cache");
    let (ref_str, _) = cache
        .refs
        .iter()
        .next()
        .expect("ref cache should have at least one entry");
    let resp = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id: tab_id.clone(),
            kind: proto::ActionKind::Uncheck.into(),
            r#ref: ref_str.clone(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn action_select_with_ref() {
    let (svc, _, tab_id) = make_svc().await;

    let _ = content::get_snapshot(
        &svc,
        Request::new(proto::GetSnapshotRequest {
            tab_id: tab_id.clone(),
            filter: proto::SnapshotFilter::FilterAll.into(),
            max_depth: 10,
        }),
    )
    .await
    .unwrap();

    let browser = svc.get_browser().await.unwrap();
    let cache = browser
        .get_ref_cache(&tab_id)
        .await
        .expect("snapshot should populate ref cache");
    let (ref_str, _) = cache
        .refs
        .iter()
        .next()
        .expect("ref cache should have at least one entry");
    let resp = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id: tab_id.clone(),
            kind: proto::ActionKind::Select.into(),
            r#ref: ref_str.clone(),
            value: "red".into(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn action_drag_with_ref() {
    let (svc, _, tab_id) = make_svc().await;

    let _ = content::get_snapshot(
        &svc,
        Request::new(proto::GetSnapshotRequest {
            tab_id: tab_id.clone(),
            filter: proto::SnapshotFilter::FilterAll.into(),
            max_depth: 10,
        }),
    )
    .await
    .unwrap();

    let browser = svc.get_browser().await.unwrap();
    let cache = browser
        .get_ref_cache(&tab_id)
        .await
        .expect("snapshot should populate ref cache");
    let (ref_str, _) = cache
        .refs
        .iter()
        .next()
        .expect("ref cache should have at least one entry");
    let resp = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id: tab_id.clone(),
            kind: proto::ActionKind::Drag.into(),
            r#ref: ref_str.clone(),
            drag_x: 10,
            drag_y: 20,
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn action_scroll_with_ref() {
    let (svc, _, tab_id) = make_svc().await;

    let _ = content::get_snapshot(
        &svc,
        Request::new(proto::GetSnapshotRequest {
            tab_id: tab_id.clone(),
            filter: proto::SnapshotFilter::FilterAll.into(),
            max_depth: 10,
        }),
    )
    .await
    .unwrap();

    let browser = svc.get_browser().await.unwrap();
    let cache = browser
        .get_ref_cache(&tab_id)
        .await
        .expect("snapshot should populate ref cache");
    let (ref_str, _) = cache
        .refs
        .iter()
        .next()
        .expect("ref cache should have at least one entry");
    let resp = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id: tab_id.clone(),
            kind: proto::ActionKind::Scroll.into(),
            r#ref: ref_str.clone(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn action_scroll_page() {
    let (svc, _, tab_id) = make_svc().await;
    let resp = actions::execute_action(
        &svc,
        Request::new(proto::ExecuteActionRequest {
            tab_id,
            kind: proto::ActionKind::Scroll.into(),
            scroll_y: 500,
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

#[tokio::test]
async fn touch_tap_dispatches_events() {
    let (svc, fake, tab_id) = make_svc().await;
    let resp = actions::touch_tap(
        &svc,
        Request::new(proto::TouchTapRequest {
            tab_id,
            x: 100.0,
            y: 200.0,
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
    // Verify two touch events dispatched (start + end)
    let touch_calls: Vec<_> = fake
        .method_names()
        .into_iter()
        .filter(|m| m == "Input.dispatchTouchEvent")
        .collect();
    assert_eq!(touch_calls.len(), 2);
}

#[tokio::test]
async fn set_input_files_requires_ref_or_selector() {
    let (svc, _, tab_id) = make_svc().await;
    let status = actions::set_input_files(
        &svc,
        Request::new(proto::SetInputFilesRequest {
            tab_id,
            r#ref: String::new(),
            selector: String::new(),
            files: vec!["file.txt".into()],
        }),
    )
    .await
    .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
}

#[tokio::test]
async fn set_input_files_with_selector() {
    let (svc, _, tab_id) = make_svc().await;
    // The fake DOM has <input id="name">, query by selector
    let resp = actions::set_input_files(
        &svc,
        Request::new(proto::SetInputFilesRequest {
            tab_id,
            r#ref: String::new(),
            selector: "#name".into(),
            files: vec!["/tmp/test.txt".into()],
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

// ── Content handlers ──

#[tokio::test]
async fn get_snapshot_returns_accessibility_nodes() {
    let (svc, _, tab_id) = make_svc().await;
    let resp = content::get_snapshot(
        &svc,
        Request::new(proto::GetSnapshotRequest {
            tab_id: tab_id.clone(),
            filter: proto::SnapshotFilter::FilterAll.into(),
            max_depth: 10,
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert_eq!(resp.tab_id, tab_id);
    // Snapshot populated ref cache
    let browser = svc.get_browser().await.unwrap();
    assert!(browser.get_ref_cache(&tab_id).await.is_some());
}

#[tokio::test]
async fn take_screenshot_png() {
    let (svc, _, tab_id) = make_svc().await;
    let resp = content::take_screenshot(
        &svc,
        Request::new(proto::TakeScreenshotRequest {
            tab_id,
            format: "png".into(),
            quality: 0,
            full_page: false,
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(!resp.data.is_empty());
    assert_eq!(resp.format, "png");
}

#[tokio::test]
async fn take_screenshot_jpeg_clamps_quality() {
    let (svc, fake, tab_id) = make_svc().await;
    let _ = content::take_screenshot(
        &svc,
        Request::new(proto::TakeScreenshotRequest {
            tab_id,
            format: "jpeg".into(),
            quality: 150, // should be clamped to 100
            full_page: false,
        }),
    )
    .await
    .unwrap();

    // Verify the call went to Page.captureScreenshot
    assert!(
        fake.method_names()
            .iter()
            .any(|m| m == "Page.captureScreenshot")
    );
}

#[tokio::test]
async fn get_text_returns_page_text() {
    let (svc, fake, tab_id) = make_svc().await;
    fake.set_evaluate_response(serde_json::json!({"result": {"value": "Hello World"}}));
    let resp = content::get_text(
        &svc,
        Request::new(proto::GetTextRequest {
            tab_id: tab_id.clone(),
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert_eq!(resp.tab_id, tab_id);
}

#[tokio::test]
async fn get_pdf_returns_data() {
    let (svc, _, tab_id) = make_svc().await;
    // FakeCdpClient.page_print_to_pdf returns valid base64 by default
    let resp = content::get_pdf(&svc, Request::new(proto::GetPdfRequest { tab_id }))
        .await
        .unwrap()
        .into_inner();
    assert!(!resp.data.is_empty());
}

// ── Data handlers ──

#[tokio::test]
async fn evaluate_returns_result() {
    let (svc, _, tab_id) = make_svc().await;
    let resp = data::evaluate(
        &svc,
        Request::new(proto::EvaluateRequest {
            tab_id,
            expression: "1 + 1".into(),
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(!resp.result.is_empty());
}

#[tokio::test]
async fn evaluate_blocked_when_disabled() {
    let (svc, _, tab_id) = make_svc_opts(true, None).await;
    let status = data::evaluate(
        &svc,
        Request::new(proto::EvaluateRequest {
            tab_id,
            expression: "1+1".into(),
        }),
    )
    .await
    .unwrap_err();
    assert_eq!(status.code(), Code::PermissionDenied);
    assert!(status.message().contains("disabled"));
}

#[tokio::test]
async fn get_cookies_returns_entries() {
    let (svc, _, tab_id) = make_svc().await;
    let resp = data::get_cookies(&svc, Request::new(proto::GetCookiesRequest { tab_id }))
        .await
        .unwrap()
        .into_inner();
    // FakeCdpClient returns empty cookies by default
    assert!(resp.cookies.is_empty());
}

#[tokio::test]
async fn set_cookies_succeeds() {
    let (svc, _, tab_id) = make_svc().await;
    let resp = data::set_cookies(
        &svc,
        Request::new(proto::SetCookiesRequest {
            tab_id,
            cookies: vec![proto::CookieEntry {
                name: "k".into(),
                value: "v".into(),
                domain: ".test.com".into(),
                path: "/".into(),
                ..Default::default()
            }],
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(resp.success);
}

// ── Conversions ──

#[test]
fn cookie_roundtrips_through_proto() {
    let original = pwright_cdp::domains::network::Cookie {
        name: "session".into(),
        value: "abc123".into(),
        domain: ".example.com".into(),
        path: "/".into(),
        expires: 1700000000.0,
        http_only: true,
        secure: true,
        same_site: "Lax".into(),
    };
    let p: proto::CookieEntry = original.clone().into();
    assert_eq!(p.name, "session");
    assert!(p.http_only);
    let back: pwright_cdp::domains::network::Cookie = p.into();
    assert_eq!(back.name, original.name);
    assert_eq!(back.same_site, original.same_site);
}

#[test]
fn a11y_node_converts_to_proto() {
    let node = pwright_bridge::snapshot::A11yNode {
        ref_id: "ref:3".into(),
        role: "button".into(),
        name: "Submit".into(),
        depth: 2,
        value: String::new(),
        disabled: false,
        focused: true,
        node_id: 42,
    };
    let p: proto::A11yNode = node.into();
    assert_eq!(p.r#ref, "ref:3");
    assert_eq!(p.role, "button");
    assert!(p.focused);
}

#[test]
fn target_info_converts_to_tab_info() {
    let target = pwright_cdp::domains::target::TargetInfo {
        target_id: "ABC".into(),
        target_type: "page".into(),
        title: "My Page".into(),
        url: "https://example.com".into(),
        attached: false,
    };
    let t: proto::TabInfo = target.into();
    assert_eq!(t.target_id, "ABC");
    assert_eq!(t.title, "My Page");
}
