//! Shared helpers for integration tests.
//!
//! Uses a dedicated tokio runtime for the CDP connection so that its
//! reader/writer tasks survive across individual `#[tokio::test]` runtimes.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::OnceLock;

use axum::Router;
use axum::response::{Html, Json};
use axum::routing::{get, post};
use pwright_bridge::playwright::Page;
use pwright_cdp::{CdpConnection, CdpSession};

/// The Chrome HTTP debug endpoint URL, resolved for Docker/local.
pub fn chrome_http_url() -> String {
    let host = std::env::var("CHROME_HOST").unwrap_or_else(|_| "localhost".into());
    let port = std::env::var("CHROME_PORT").unwrap_or_else(|_| "9222".into());
    // Chrome rejects Host headers that aren't localhost or an IP address.
    // Resolve hostname to IP when running in Docker.
    let resolved = if host == "localhost" || host.parse::<std::net::IpAddr>().is_ok() {
        host
    } else {
        use std::net::ToSocketAddrs;
        format!("{host}:{port}")
            .to_socket_addrs()
            .ok()
            .and_then(|mut addrs| addrs.next())
            .map(|addr| addr.ip().to_string())
            .unwrap_or(host)
    };
    format!("http://{resolved}:{port}")
}

// ── Shared CDP connection with its own runtime ──

struct SharedCdp {
    conn: Arc<CdpConnection>,
    browser: CdpSession,
    _runtime: tokio::runtime::Runtime,
}

// CdpSession/CdpConnection are Send+Sync. Runtime is Send+Sync.
// The unsafe is needed because Runtime doesn't impl Sync by default,
// but we only access it through OnceLock (which requires Sync).
unsafe impl Sync for SharedCdp {}

static SHARED_CDP: OnceLock<SharedCdp> = OnceLock::new();

fn ensure_cdp() -> &'static SharedCdp {
    SHARED_CDP.get_or_init(|| {
        // Build a dedicated runtime on a background thread to avoid
        // "cannot start a runtime from within a runtime" when called
        // from #[tokio::test].
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();

            let (conn, browser) = rt.block_on(async {
                let chrome_url = chrome_http_url();
                let version_url = format!("{chrome_url}/json/version");
                let resp: serde_json::Value = reqwest::get(&version_url)
                    .await
                    .expect("Chrome not running")
                    .json()
                    .await
                    .unwrap();
                let raw_ws = resp["webSocketDebuggerUrl"].as_str().unwrap();
                // Rewrite WS URL to use same host/port as HTTP URL
                // (Chrome returns ws://127.0.0.1:9223 but we connect via chrome:9222)
                let ws_url = pwright_bridge::browser::rewrite_ws_url(&chrome_url, raw_ws)
                    .unwrap_or_else(|_| raw_ws.to_string());
                let conn = CdpConnection::connect(&ws_url)
                    .await
                    .expect("CDP connect failed");
                let browser = CdpSession::browser(conn.clone());
                (conn, browser)
            });

            SharedCdp {
                conn,
                browser,
                _runtime: rt,
            }
        })
        .join()
        .unwrap()
    })
}

// ── Shared test server (on its own thread/runtime) ──

static SERVER_ADDR: OnceLock<SocketAddr> = OnceLock::new();

/// Returns the base URL for the test server.
///
/// When running in Docker (CHROME_HOST is set), binds to 0.0.0.0 and
/// uses the test-runner container hostname so Chrome can reach it.
/// When running locally, binds to 127.0.0.1.
fn ensure_server_blocking() -> String {
    if let Some(addr) = SERVER_ADDR.get() {
        return server_url(addr.port());
    }

    let in_docker = std::env::var("CHROME_HOST").is_ok();
    let bind_addr = if in_docker {
        "0.0.0.0:0"
    } else {
        "127.0.0.1:0"
    };

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let app = build_app();
            let listener = tokio::net::TcpListener::bind(bind_addr).await.unwrap();
            let addr = listener.local_addr().unwrap();
            tx.send(addr).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });

    let addr = rx.recv().unwrap();
    let _ = SERVER_ADDR.set(addr);

    for _ in 0..100 {
        if std::net::TcpStream::connect(addr).is_ok() {
            return server_url(addr.port());
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("test server failed to start")
}

fn server_url(port: u16) -> String {
    // In Docker: Chrome reaches test-runner via container name
    // Locally: Chrome runs on host network
    let host = if std::env::var("CHROME_HOST").is_ok() {
        "test-runner".to_string()
    } else {
        "127.0.0.1".to_string()
    };
    format!("http://{host}:{port}")
}

fn build_app() -> Router {
    let fixtures_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures");

    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/search", post(api_search))
        .route("/api/items", get(api_items))
        .route("/login", get(login_page))
        .route("/todo", get(todo_page))
        .route("/api-demo", get(api_demo_page))
        .fallback_service(tower_http::services::ServeDir::new(fixtures_dir))
}

async fn api_search(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let query = body
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Json(serde_json::json!({
        "query": query,
        "results": [
            format!("Result 1 for {query}"),
            format!("Result 2 for {query}"),
            format!("Result 3 for {query}"),
        ]
    }))
}

async fn api_items(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let count: u64 = params
        .get("count")
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let items: Vec<serde_json::Value> = (1..=count)
        .map(|i| {
            serde_json::json!({
                "id": i,
                "name": format!("Item {i}"),
                "done": i % 2 == 0,
            })
        })
        .collect();
    Json(serde_json::json!(items))
}

async fn login_page() -> Html<&'static str> {
    Html(include_str!("../pages/login.html"))
}

async fn todo_page() -> Html<&'static str> {
    Html(include_str!("../pages/todo.html"))
}

async fn api_demo_page() -> Html<String> {
    Html(include_str!("../pages/api-demo.html").to_string())
}

/// Get the test server port (for constructing URLs in tests).
pub fn server_port() -> u16 {
    ensure_server_blocking();
    SERVER_ADDR.get().unwrap().port()
}

/// Get the full test server base URL (works in both Docker and local).
pub fn server_base_url() -> String {
    ensure_server_blocking()
}

// ── Test helper ──

/// Connect to Chrome and create a page for testing.
///
/// Shares one CDP WebSocket connection (on a dedicated runtime).
/// Each test gets its own tab via Target.createTarget.
pub async fn connect_and_navigate(path: &str) -> Page {
    let server_url = ensure_server_blocking();
    let cdp = ensure_cdp();

    let target_id = cdp
        .browser
        .target_create("about:blank")
        .await
        .expect("failed to create tab");
    let session_id = cdp
        .browser
        .target_attach(&target_id)
        .await
        .expect("failed to attach");
    let session = Arc::new(CdpSession::new(
        cdp.conn.clone(),
        session_id,
        target_id.clone(),
    ));
    let page = Page::with_tab(session, target_id);

    let url = format!("{server_url}{path}");
    let opts = pwright_bridge::playwright::GotoOptions {
        wait_until: Some("domcontentloaded".to_string()),
        ..Default::default()
    };
    page.goto(&url, Some(opts))
        .await
        .expect("failed to navigate");

    page
}
